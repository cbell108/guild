//! Offline tests: real server replies recorded in `tests/fixtures/phrases/` (see
//! `live_phrases.rs::record_fixtures`) run through the same parse and refine path as
//! production. Never needs the LLM.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{DateTime, Utc};
use llm::hangout::{parse_response, refine};
use serde_json::Value;

/// What a recorded phrase must come out as: date, start, end, capacity, location fragment.
struct Expect {
    name: &'static str,
    date: Option<&'static str>,
    start: Option<&'static str>,
    end: Option<&'static str>,
    capacity: Option<u32>,
    location: Option<&'static str>,
}

const CASES: &[Expect] = &[
    Expect {
        name: "tmrw_soccer",
        date: Some("2026-11-20"),
        start: Some("16:00"),
        end: None,
        capacity: None,
        location: Some("green"),
    },
    Expect {
        name: "in_two_hours",
        date: Some("2026-11-19"),
        start: Some("14:00"),
        end: None,
        capacity: None,
        location: None,
    },
    Expect {
        name: "no_day",
        date: None,
        start: None,
        end: None,
        capacity: None,
        location: Some("rec center"),
    },
    Expect {
        name: "late_night",
        date: None,
        start: Some("23:00"),
        end: None,
        capacity: None,
        location: None,
    },
    Expect {
        name: "past_midnight",
        date: Some("2026-11-20"),
        start: Some("21:00"),
        end: Some("01:00"),
        capacity: Some(40),
        location: Some("rec center"),
    },
    Expect {
        name: "next_tuesday",
        date: Some("2026-11-24"),
        start: Some("15:30"),
        end: None,
        capacity: None,
        location: Some("library"),
    },
    Expect {
        name: "typos",
        date: Some("2026-11-19"),
        start: Some("20:00"),
        end: Some("23:00"),
        capacity: None,
        location: Some("center"),
    },
    Expect {
        name: "bare_evening_hour",
        date: Some("2026-11-21"),
        start: Some("19:00"),
        end: None,
        capacity: Some(8),
        location: None,
    },
];

#[test]
fn recorded_replies_come_out_right() {
    for case in CASES {
        let path = format!(
            "{}/tests/fixtures/phrases/{}.json",
            env!("CARGO_MANIFEST_DIR"),
            case.name
        );
        let fixture: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let input = fixture["input"].as_str().unwrap();
        let now: DateTime<Utc> = fixture["now"].as_str().unwrap().parse().unwrap();
        let body = fixture["response"].to_string();
        let draft = refine(parse_response(&body).unwrap(), input, now);
        let ctx = format!("{} ({input:?}) -> {draft:?}", case.name);
        assert_eq!(draft.date.as_deref(), case.date, "date: {ctx}");
        assert_eq!(draft.start.as_deref(), case.start, "start: {ctx}");
        assert_eq!(draft.end.as_deref(), case.end, "end: {ctx}");
        assert_eq!(draft.capacity, case.capacity, "capacity: {ctx}");
        match case.location {
            Some(frag) => assert!(
                draft
                    .location
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(frag),
                "location: {ctx}"
            ),
            None => assert_eq!(draft.location, None, "location: {ctx}"),
        }
        assert!(!draft.tags.is_empty(), "tags: {ctx}");
    }
}
