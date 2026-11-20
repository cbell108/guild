//! Live accuracy check for natural-language hosting: runs `phrases.json` against the local
//! LLM server and prints a per-field score. Ignored by default because it needs the server:
//!
//! `cargo test -p llm --test live_phrases -- --ignored --nocapture`
//!
//! `now` is fixed (Thu 2026-11-19 12:00 America/New_York) so expectations are stable.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{DateTime, Utc};
use hangouts_core::HangoutDraft;
use llm::{LlmClient, LlmConfig, parse_hangout};
use serde_json::Value;

const PHRASES: &str = include_str!("phrases.json");
/// Minimum overall field accuracy the prompt must keep reaching.
const MIN_ACCURACY: f64 = 0.85;

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-19T17:00:00Z")
        .unwrap()
        .to_utc()
}

/// `(correct, scored)` for one phrase, listing misses in `misses`.
fn score(draft: &HangoutDraft, case: &Value, misses: &mut Vec<String>) -> (u32, u32) {
    let (mut ok, mut total) = (0, 0);
    let mut check = |field: &str, good: bool, got: String| {
        total += 1;
        if good {
            ok += 1;
        } else {
            misses.push(format!("{field}={got}"));
        }
    };
    let opt_str = |expected: &Value, got: &Option<String>| {
        expected
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v.as_str() == got.as_deref()))
    };
    if case.get("date").is_some() {
        check(
            "date",
            opt_str(&case["date"], &draft.date),
            format!("{:?}", draft.date),
        );
    }
    if case.get("start").is_some() {
        check(
            "start",
            opt_str(&case["start"], &draft.start),
            format!("{:?}", draft.start),
        );
    }
    if case.get("end").is_some() {
        check(
            "end",
            opt_str(&case["end"], &draft.end),
            format!("{:?}", draft.end),
        );
    }
    if let Some(words) = case.get("location").and_then(Value::as_array) {
        let loc = draft.location.clone().unwrap_or_default().to_lowercase();
        let good = words
            .iter()
            .filter_map(Value::as_str)
            .any(|w| loc.contains(w));
        check("location", good, format!("{loc:?}"));
    }
    if let Some(words) = case.get("tags").and_then(Value::as_array) {
        let good = draft.tags.iter().any(|t| {
            words
                .iter()
                .filter_map(Value::as_str)
                .any(|w| t.contains(w))
        });
        check("tags", good, format!("{:?}", draft.tags));
    }
    if let Some(caps) = case.get("capacity").and_then(Value::as_array) {
        let good = caps
            .iter()
            .any(|v| v.as_u64().map(|n| n as u32) == draft.capacity);
        check("capacity", good, format!("{:?}", draft.capacity));
    }
    (ok, total)
}

#[tokio::test]
#[ignore = "needs a running LLM server"]
async fn phrase_set_accuracy() {
    let client = LlmClient::new(LlmConfig::from_env()).unwrap();
    let cases: Vec<Value> = serde_json::from_str(PHRASES).unwrap();
    let (mut ok, mut total) = (0, 0);
    for case in &cases {
        let text = case["text"].as_str().unwrap();
        let draft = parse_hangout(&client, text, now()).await.unwrap();
        let mut misses = Vec::new();
        let (o, t) = score(&draft, case, &mut misses);
        ok += o;
        total += t;
        println!("{o}/{t}  {text}");
        for m in misses {
            println!("      MISS {m}");
        }
    }
    let accuracy = f64::from(ok) / f64::from(total);
    println!("\nOVERALL {ok}/{total} = {:.1}%", accuracy * 100.0);
    assert!(
        accuracy >= MIN_ACCURACY,
        "accuracy {accuracy:.3} below {MIN_ACCURACY}"
    );
}

/// Phrases whose raw server replies are kept in `tests/fixtures/phrases/` for the offline
/// tests in `recorded.rs`.
const RECORDED: &[(&str, &str)] = &[
    ("tmrw_soccer", "pickup soccer tmrw at 4 on the campus green"),
    ("in_two_hours", "midnight snack run in 2 hours"),
    ("no_day", "chill hangout at the rec center"),
    ("late_night", "late night ramen run 11pm"),
    (
        "past_midnight",
        "friday night 9 to 1 dance party in the rec center, max 40",
    ),
    (
        "next_tuesday",
        "coffee and homework next tuesday 3:30pm at the starbucks in the library",
    ),
    (
        "typos",
        "smsh bros tonite 8 to 11 in the studnet center gameroom",
    ),
    (
        "bare_evening_hour",
        "saturday 7 mario kart tournament, 8 spots",
    ),
];

/// Re-records the fixtures from the live server:
/// `cargo test -p llm --test live_phrases -- --ignored record_fixtures`.
#[tokio::test]
#[ignore = "needs a running LLM server"]
async fn record_fixtures() {
    let config = LlmConfig::from_env();
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/phrases");
    std::fs::create_dir_all(&dir).unwrap();
    for (name, text) in RECORDED {
        let body = llm::hangout::request(&config.model, text, now());
        let reply = reqwest::Client::new()
            .post(&url)
            .json(&body)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let fixture = serde_json::json!({
            "input": text,
            "now": "2026-11-19T17:00:00Z",
            "response": serde_json::from_str::<Value>(&reply).unwrap(),
        });
        let path = dir.join(format!("{name}.json"));
        std::fs::write(path, serde_json::to_string_pretty(&fixture).unwrap() + "\n").unwrap();
    }
}
