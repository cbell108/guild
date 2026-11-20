//! Prompt, JSON schema and response parsing for [`crate::parse_hangout`]. Pure, so the
//! prompt and the parser are unit-tested against a recorded server response.

use chrono::{DateTime, Datelike, TimeDelta, Utc};
use hangouts_core::HangoutDraft;
use hangouts_core::time::{format_date, to_local};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::LlmError;

/// Upper bound on generated tokens (a draft is small).
pub const MAX_TOKENS: u32 = 400;

/// JSON schema the server constrains its output to (llama.cpp `response_format`).
pub fn draft_schema() -> Value {
    let text = json!({"type": ["string", "null"]});
    json!({
        "type": "object",
        "properties": {
            "title": text,
            "description": text,
            "date": {"type": ["string", "null"], "description": "YYYY-MM-DD"},
            "start": {"type": ["string", "null"], "description": "HH:MM, 24-hour"},
            "end": {"type": ["string", "null"], "description": "HH:MM, 24-hour"},
            "location": text,
            "tags": {"type": "array", "items": {"type": "string"}, "maxItems": 5},
            "capacity": {"type": ["integer", "null"], "minimum": 1},
            "gear_notes": text
        },
        "required": [
            "title", "description", "date", "start", "end",
            "location", "tags", "capacity", "gear_notes"
        ],
        "additionalProperties": false
    })
}

/// The system prompt, with today's campus-local date and the next week's weekdays spelled
/// out so the model can resolve "tonight" or "friday" without date arithmetic.
pub fn system_prompt(now: DateTime<Utc>) -> String {
    let local = to_local(now);
    let today = local.date_naive();
    let week: Vec<String> = (0..7)
        .map(|d| {
            let day = today + TimeDelta::days(d);
            format!("{} {}", day.weekday(), format_date(day))
        })
        .collect();
    format!(
        "You turn a student's short description of a campus hangout at Kennesaw State \
         University into a JSON form prefill.\n\
         Now: {} {} (America/New_York). Next 7 days: {}.\n\
         Rules:\n\
         - date: YYYY-MM-DD. \"tonight\"/\"today\" = today; a weekday name = its next \
         occurrence from the list (today if it matches).\n\
         - start/end: 24-hour HH:MM. \"7-10\" in the evening means 19:00-22:00.\n\
         - title: short and friendly, max 60 characters.\n\
         - description: one sentence for other students, or null.\n\
         - location: building and room as written, or null.\n\
         - tags: 1-5 lowercase topic words (e.g. smash-bros, board-games, study).\n\
         - capacity: only if a number of people or spots is stated, else null.\n\
         - gear_notes: equipment the host mentions bringing, else null.\n\
         - Use null for anything not stated. Never invent details.",
        local.weekday(),
        local.format("%Y-%m-%d %H:%M"),
        week.join(", ")
    )
}

/// The full `/chat/completions` request body.
pub fn request(model: &str, text: &str, now: DateTime<Utc>) -> Value {
    json!({
        "model": model,
        "temperature": 0,
        "max_tokens": MAX_TOKENS,
        "messages": [
            {"role": "system", "content": system_prompt(now)},
            {"role": "user", "content": text}
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {"name": "hangout_draft", "strict": true, "schema": draft_schema()}
        },
        // llama.cpp / Qwen: answer directly, no thinking block.
        "chat_template_kwargs": {"enable_thinking": false}
    })
}

#[derive(Deserialize)]
struct Completion {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}

/// Extracts and normalizes the draft from a `/chat/completions` response body. Tolerates a
/// Markdown code fence or text around the JSON object.
pub fn parse_response(body: &str) -> Result<HangoutDraft, LlmError> {
    let completion: Completion =
        serde_json::from_str(body).map_err(|e| LlmError::BadResponse(e.to_string()))?;
    let content = completion
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .ok_or_else(|| LlmError::BadResponse("no message content".to_owned()))?;
    parse_content(&content)
}

/// Parses the model's message text (a JSON object, possibly fenced) into a normalized draft.
pub fn parse_content(content: &str) -> Result<HangoutDraft, LlmError> {
    let start = content.find('{');
    let end = content.rfind('}');
    let json = match (start, end) {
        (Some(s), Some(e)) if e > s => &content[s..=e],
        _ => return Err(LlmError::BadResponse("no JSON object in reply".to_owned())),
    };
    let draft: HangoutDraft =
        serde_json::from_str(json).map_err(|e| LlmError::BadResponse(e.to_string()))?;
    Ok(draft.normalized())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real llama.cpp response (Qwen3.8-27B, combined layout) to
    /// [`RECORDED_INPUT`] at [`recorded_now`], captured with the ignored `record` test.
    const RECORDED: &str = include_str!("../tests/fixtures/parse_hangout_response.json");
    const RECORDED_INPUT: &str =
        "smash tonight 7-10 in the student center game room, I have 2 switches, max 8 people";

    fn recorded_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-19T17:00:00Z")
            .unwrap()
            .to_utc()
    }

    #[test]
    fn prompt_spells_out_dates() {
        let p = system_prompt(recorded_now());
        assert!(p.contains("Now: Thu 2026-11-19 12:00"), "{p}");
        assert!(p.contains("Fri 2026-11-20"));
        assert!(p.contains("Wed 2026-11-25"));
    }

    #[test]
    fn request_is_schema_constrained() {
        let r = request("qwen", "x", recorded_now());
        assert_eq!(r["model"], "qwen");
        assert_eq!(r["response_format"]["type"], "json_schema");
        let schema = &r["response_format"]["json_schema"]["schema"];
        assert_eq!(schema["required"].as_array().unwrap().len(), 9);
        assert_eq!(r["messages"][1]["content"], "x");
    }

    #[test]
    fn parses_recorded_response() {
        let d = parse_response(RECORDED).unwrap();
        assert_eq!(d.date.as_deref(), Some("2026-11-19"));
        assert_eq!(d.start.as_deref(), Some("19:00"));
        assert_eq!(d.end.as_deref(), Some("22:00"));
        assert_eq!(d.capacity, Some(8));
        assert!(d.title.as_deref().unwrap().to_lowercase().contains("smash"));
        assert!(
            d.location
                .as_deref()
                .unwrap()
                .to_lowercase()
                .contains("game room")
        );
        assert!(d.gear_notes.as_deref().unwrap().contains('2'));
        assert!(d.tags.iter().all(|t| t == &t.to_lowercase()));
    }

    #[test]
    fn tolerates_fences_and_normalizes() {
        let d = parse_content(
            "```json\n{\"title\":\" Board games \",\"date\":\"2026-11-2\",\"start\":\"7pm\",\
             \"tags\":[\"Board Games\"],\"capacity\":0}\n```",
        )
        .unwrap();
        assert_eq!(d.title.as_deref(), Some("Board games"));
        assert_eq!(d.date.as_deref(), Some("2026-11-02"));
        assert_eq!(d.start, None, "invalid time dropped for the host to fill");
        assert_eq!(d.tags, vec!["board-games"]);
        assert_eq!(d.capacity, None);
    }

    #[test]
    fn bad_replies_are_bad_response() {
        assert!(matches!(
            parse_response("nope"),
            Err(LlmError::BadResponse(_))
        ));
        assert!(matches!(
            parse_response(r#"{"choices":[]}"#),
            Err(LlmError::BadResponse(_))
        ));
        assert!(matches!(
            parse_content("I can't help with that"),
            Err(LlmError::BadResponse(_))
        ));
    }

    /// Records [`RECORDED`] from a live server. Run manually:
    /// `LLM_MODEL=<id> cargo test -p llm -- --ignored record --nocapture`.
    #[test]
    #[ignore = "needs a running LLM server"]
    fn record() {
        let config = crate::LlmConfig::from_env();
        let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
        let body = request(&config.model, RECORDED_INPUT, recorded_now());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let text = rt.block_on(async {
            reqwest::Client::new()
                .post(url)
                .json(&body)
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        });
        println!("{text}");
    }
}
