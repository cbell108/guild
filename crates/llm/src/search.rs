//! Prompt, JSON schema and response parsing for [`crate::interpret_search`]: turns a student's
//! natural-language request ("something chill tonight with food") into Browse filters.
//! Pure, so the prompt and the normalization are unit-tested against recorded responses.

use chrono::{DateTime, Datelike, Utc};
use hangouts_core::time::{format_date, to_local};
use hangouts_core::{EventKind, normalize_tags};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::LlmError;

/// Upper bound on generated tokens (the answer is a handful of fields).
pub const MAX_TOKENS: u32 = 300;
/// Longest free-text keyword query kept.
const MAX_Q_CHARS: usize = 60;
/// Longest summary pill kept.
const MAX_SUMMARY_CHARS: usize = 80;
/// Most tags kept.
const MAX_TAGS: usize = 4;

/// The date window a request refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchWhen {
    /// Today (campus-local).
    Today,
    /// Tomorrow.
    Tomorrow,
    /// The coming Saturday and Sunday.
    Weekend,
    /// The next 7 days.
    Week,
}

/// Browse filters derived from a request, plus a one-line description of them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchIntent {
    /// Date window, if the request names one.
    pub when: Option<SearchWhen>,
    /// Official events or hangouts only.
    pub kind: Option<EventKind>,
    /// Interest/topic tags (restricted to the known vocabulary when one is given).
    pub tags: Vec<String>,
    /// Leftover keywords for free-text search.
    pub q: Option<String>,
    /// Short pill text, e.g. "Showing food events tonight".
    pub summary: String,
}

/// JSON schema the server constrains its output to (llama.cpp `response_format`).
pub fn intent_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "when": {"enum": ["today", "tomorrow", "weekend", "week", null]},
            "kind": {"enum": ["official", "hangout", null]},
            "tags": {"type": "array", "items": {"type": "string"}, "maxItems": MAX_TAGS},
            "q": {"type": ["string", "null"]},
            "summary": {"type": "string"}
        },
        "required": ["when", "kind", "tags", "q", "summary"],
        "additionalProperties": false
    })
}

/// The system prompt: today's date, the available tags, and the field rules.
pub fn system_prompt(now: DateTime<Utc>, known_tags: &[String]) -> String {
    let local = to_local(now);
    let tags = if known_tags.is_empty() {
        "(any lowercase topic words)".to_owned()
    } else {
        known_tags.join(", ")
    };
    format!(
        "You turn a Kennesaw State student's request for campus events into search filters.\n\
         Now: {} {} (America/New_York); today is {}.\n\
         Rules:\n\
         - when: \"today\" (also tonight), \"tomorrow\", \"weekend\" (this weekend), \"week\" (this week \
         or next 7 days), or null if no time is mentioned.\n\
         - kind: \"official\" for university/department events, \"hangout\" for student-hosted \
         hangouts, null if unspecified.\n\
         - tags: up to {MAX_TAGS} topic tags chosen ONLY from this list: {tags}. \"free stuff\" or \
         \"free food\" means free-food/food; \"chill\" or \"relaxed\" is a mood, not a tag. \
         Use [] if none fit.\n\
         - q: leftover keywords worth text-searching (a lowercase word or two), else null. \
         Never repeat words already captured by when, kind or tags.\n\
         - summary: one short pill like \"Showing food events tonight\" or \"Showing hangouts this weekend\"; \
         describe only what the other fields apply.\n\
         - Never invent filters the request doesn't imply.",
        local.weekday(),
        local.format("%Y-%m-%d %H:%M"),
        format_date(local.date_naive()),
    )
}

/// The full `/chat/completions` request body.
pub fn request(model: &str, text: &str, now: DateTime<Utc>, known_tags: &[String]) -> Value {
    json!({
        "model": model,
        "temperature": 0,
        "max_tokens": MAX_TOKENS,
        "messages": [
            {"role": "system", "content": system_prompt(now, known_tags)},
            {"role": "user", "content": text}
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {"name": "search_intent", "strict": true, "schema": intent_schema()}
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

/// Parses a full `/chat/completions` body into a normalized intent.
pub fn parse_response(body: &str, known_tags: &[String]) -> Result<SearchIntent, LlmError> {
    let completion: Completion =
        serde_json::from_str(body).map_err(|e| LlmError::BadResponse(e.to_string()))?;
    let content = completion
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .ok_or_else(|| LlmError::BadResponse("no message content".to_owned()))?;
    parse_content(&content, known_tags)
}

/// Parses the model's message text (a JSON object, possibly fenced) into a normalized intent.
pub fn parse_content(content: &str, known_tags: &[String]) -> Result<SearchIntent, LlmError> {
    let json = match (content.find('{'), content.rfind('}')) {
        (Some(s), Some(e)) if e > s => &content[s..=e],
        _ => return Err(LlmError::BadResponse("no JSON object in reply".to_owned())),
    };
    let raw: SearchIntent =
        serde_json::from_str(json).map_err(|e| LlmError::BadResponse(e.to_string()))?;
    Ok(normalize(raw, known_tags))
}

/// Cleans a model answer: tags are normalized and restricted to `known_tags` (when given),
/// blank strings become `None`, lengths are capped, and a missing summary is generated.
pub fn normalize(mut intent: SearchIntent, known_tags: &[String]) -> SearchIntent {
    let mut tags = normalize_tags(&intent.tags);
    if !known_tags.is_empty() {
        tags.retain(|t| known_tags.contains(t));
    }
    tags.truncate(MAX_TAGS);
    intent.tags = tags;
    intent.q = intent
        .q
        .map(|q| q.trim().to_lowercase())
        .filter(|q| !q.is_empty())
        .map(|q| q.chars().take(MAX_Q_CHARS).collect());
    let summary: String = intent
        .summary
        .trim()
        .chars()
        .take(MAX_SUMMARY_CHARS)
        .collect();
    intent.summary = if summary.is_empty() {
        describe(&intent)
    } else {
        summary
    };
    intent
}

/// A plain description of the filters ("Showing food events tonight").
pub fn describe(intent: &SearchIntent) -> String {
    let what = match (&intent.kind, intent.tags.is_empty()) {
        (Some(EventKind::Hangout), true) => "hangouts".to_owned(),
        (Some(EventKind::Official), true) => "official events".to_owned(),
        (Some(EventKind::Hangout), false) => format!("{} hangouts", intent.tags.join(" & ")),
        (Some(EventKind::Official), false) => {
            format!("official {} events", intent.tags.join(" & "))
        }
        (None, false) => format!("{} events", intent.tags.join(" & ")),
        (None, true) => intent
            .q
            .as_ref()
            .map_or_else(|| "events".to_owned(), |q| format!("events matching “{q}”")),
    };
    let when = match intent.when {
        Some(SearchWhen::Today) => " tonight",
        Some(SearchWhen::Tomorrow) => " tomorrow",
        Some(SearchWhen::Weekend) => " this weekend",
        Some(SearchWhen::Week) => " this week",
        None => "",
    };
    format!("Showing {what}{when}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-20T23:30:00Z")
            .unwrap()
            .into()
    }

    fn known() -> Vec<String> {
        ["food", "free-food", "music", "sports"]
            .map(String::from)
            .to_vec()
    }

    /// A recorded llama.cpp reply for "something chill tonight with food".
    const RECORDED: &str = r#"{"choices":[{"message":{"role":"assistant","content":"{\"when\":\"today\",\"kind\":null,\"tags\":[\"Food\",\"free-food\",\"pizza\"],\"q\":\" \",\"summary\":\"Showing food events tonight\"}"}}]}"#;

    #[test]
    fn recorded_reply_is_parsed_and_normalized() {
        let intent = parse_response(RECORDED, &known()).unwrap();
        assert_eq!(intent.when, Some(SearchWhen::Today));
        assert_eq!(intent.kind, None);
        // "pizza" is not in the vocabulary; "Food" is normalized.
        assert_eq!(intent.tags, ["food", "free-food"]);
        assert_eq!(intent.q, None);
        assert_eq!(intent.summary, "Showing food events tonight");
    }

    #[test]
    fn fenced_output_and_missing_summary_are_handled() {
        let content = "```json\n{\"when\":\"weekend\",\"kind\":\"hangout\",\"tags\":[],\"q\":\"Smash\",\"summary\":\"\"}\n```";
        let intent = parse_content(content, &known()).unwrap();
        assert_eq!(intent.q.as_deref(), Some("smash"));
        assert_eq!(intent.summary, "Showing hangouts this weekend");
    }

    #[test]
    fn garbage_is_a_bad_response() {
        assert!(matches!(
            parse_content("no json here", &[]),
            Err(LlmError::BadResponse(_))
        ));
        assert!(matches!(
            parse_response("{}", &[]),
            Err(LlmError::BadResponse(_))
        ));
        // An unknown `when` value is rejected rather than guessed.
        let bad = r#"{"when":"yesterday","kind":null,"tags":[],"q":null,"summary":"x"}"#;
        assert!(matches!(
            parse_content(bad, &[]),
            Err(LlmError::BadResponse(_))
        ));
    }

    #[test]
    fn request_carries_schema_and_date() {
        let req = request("m", "free stuff this weekend", now(), &known());
        assert_eq!(
            req["response_format"]["json_schema"]["name"],
            "search_intent"
        );
        let system = req["messages"][0]["content"].as_str().unwrap();
        assert!(system.contains("2026-11-20"), "{system}");
        assert!(system.contains("free-food"), "{system}");
        assert_eq!(req["messages"][1]["content"], "free stuff this weekend");
    }

    #[test]
    fn describe_covers_kind_tags_and_when() {
        let i = SearchIntent {
            when: Some(SearchWhen::Week),
            kind: Some(EventKind::Official),
            tags: vec!["music".into()],
            ..SearchIntent::default()
        };
        assert_eq!(describe(&i), "Showing official music events this week");
    }
}
