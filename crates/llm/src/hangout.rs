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
         - date: YYYY-MM-DD. \"tonight\"/\"today\" = today; \"tomorrow\"/\"tmrw\" = the day after \
         today; a weekday name = its next occurrence from the list (today if it matches); \
         \"next <weekday>\" = that weekday in the coming week. If the text names no day at all, \
         use null.\n\
         - start/end: 24-hour HH:MM. Hours without am/pm are evening or afternoon for \
         social plans (\"7-10\" = 19:00-22:00, \"at 4\" = 16:00, \"saturday 7\" = 19:00), but \
         \"morning\", \"breakfast\" and \"6:30am\" are AM. \"noon\" = 12:00, \"midnight\" = 00:00 \
         (an end of 00:00 or 02:00 means after midnight). \"in 2 hours\" = Now plus 2 hours. \
         If no time is stated, use null.\n\
         - title: short and friendly, max 60 characters.\n\
         - description: one sentence for other students, or null.\n\
         - location: the place as the host wrote it, informal names included (\"the lawn\", \"my dorm\"), or null.\n\
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

/// Words that name a day; if the host's text has none the date must be left for them to pick.
const DAY_WORDS: &[&str] = &[
    "today",
    "tonight",
    "tonite",
    "tomorrow",
    "tmrw",
    "tmr",
    "tomorow",
    "weekend",
    "next",
    "monday",
    "mon",
    "tuesday",
    "tue",
    "tues",
    "wednesday",
    "wed",
    "thursday",
    "thu",
    "thur",
    "thurs",
    "friday",
    "fri",
    "saturday",
    "sat",
    "sunday",
    "sun",
    "january",
    "jan",
    "february",
    "feb",
    "march",
    "mar",
    "april",
    "apr",
    "may",
    "june",
    "jun",
    "july",
    "jul",
    "august",
    "aug",
    "september",
    "sep",
    "sept",
    "october",
    "oct",
    "november",
    "nov",
    "december",
    "dec",
];

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// "in 2 hours" / "in an hour" / "in 30 minutes" as an offset from now.
fn relative_offset(words: &[String]) -> Option<TimeDelta> {
    for (i, w) in words.iter().enumerate() {
        if w != "in" {
            continue;
        }
        let amount = match words.get(i + 1).map(String::as_str) {
            Some("a" | "an") => 1,
            Some(n) => n
                .parse::<i64>()
                .ok()
                .filter(|n| (1..=24 * 60).contains(n))?,
            None => continue,
        };
        match words.get(i + 2).map(String::as_str) {
            Some("hour" | "hours" | "hr" | "hrs") => return Some(TimeDelta::hours(amount)),
            Some("minute" | "minutes" | "min" | "mins") => return Some(TimeDelta::minutes(amount)),
            _ => {}
        }
    }
    None
}

/// Deterministic fixes for what the model gets wrong or invents, checked against the host's
/// own words: a date is kept only if the text names a day (or an "in 2 hours" offset), and an
/// "in N hours/minutes" start is computed from `now` instead of trusting model arithmetic.
#[must_use]
pub fn refine(mut draft: HangoutDraft, text: &str, now: DateTime<Utc>) -> HangoutDraft {
    let words = words(text);
    if let Some(offset) = relative_offset(&words) {
        let at = to_local(now + offset);
        draft.date = Some(format_date(at.date_naive()));
        draft.start = Some(at.format("%H:%M").to_string());
    } else if !words.iter().any(|w| DAY_WORDS.contains(&w.as_str())) && !text.contains('/') {
        draft.date = None;
    }
    draft.normalized()
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

    fn draft_with(date: Option<&str>, start: Option<&str>) -> HangoutDraft {
        HangoutDraft {
            title: Some("x".into()),
            date: date.map(str::to_owned),
            start: start.map(str::to_owned),
            ..HangoutDraft::default()
        }
    }

    #[test]
    fn refine_drops_a_date_the_host_never_gave() {
        let d = refine(
            draft_with(Some("2026-11-19"), Some("23:00")),
            "ramen run 11pm",
            recorded_now(),
        );
        assert_eq!(d.date, None, "an invented date is left for the host");
        assert_eq!(d.start.as_deref(), Some("23:00"), "the stated time stays");
    }

    #[test]
    fn refine_keeps_dates_when_a_day_is_named() {
        for text in [
            "soccer tmrw at 4",
            "Fri 6pm",
            "movie TONITE",
            "hike nov 28",
            "next tuesday",
            "party 11/28",
        ] {
            let d = refine(draft_with(Some("2026-11-20"), None), text, recorded_now());
            assert_eq!(d.date.as_deref(), Some("2026-11-20"), "{text}");
        }
        // "sunny" is not "sun".
        let d = refine(
            draft_with(Some("2026-11-22"), None),
            "sunny picnic",
            recorded_now(),
        );
        assert_eq!(d.date, None);
    }

    #[test]
    fn refine_computes_relative_starts_itself() {
        // Models botch "in 2 hours"; Now is Thu 12:00 local, so 14:00 today.
        let wrong = draft_with(Some("2026-11-20"), Some("01:00"));
        let d = refine(wrong, "snack run in 2 hours", recorded_now());
        assert_eq!(
            (d.date.as_deref(), d.start.as_deref()),
            (Some("2026-11-19"), Some("14:00"))
        );
        let d = refine(
            draft_with(None, None),
            "grab food in an hour",
            recorded_now(),
        );
        assert_eq!(d.start.as_deref(), Some("13:00"));
        let d = refine(draft_with(None, None), "meet in 45 minutes", recorded_now());
        assert_eq!(d.start.as_deref(), Some("12:45"));
        // Not an offset: "in the library".
        let d = refine(
            draft_with(Some("2026-11-19"), Some("18:00")),
            "dinner in the library today",
            recorded_now(),
        );
        assert_eq!(d.start.as_deref(), Some("18:00"));
    }

    #[test]
    fn prompt_covers_the_known_traps() {
        let p = system_prompt(recorded_now());
        for needle in [
            "tmrw",
            "evening or afternoon",
            "Now plus 2 hours",
            "next <weekday>",
            "midnight",
        ] {
            assert!(p.contains(needle), "prompt lost its {needle:?} rule");
        }
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
