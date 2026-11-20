//! Edge cases for [`ingest::map_page`], pinned against inline Localist-shaped JSON. No network.
//!
//! Covers the cases the saved fixture does not exercise: virtual events without a venue,
//! HTML entities in titles, duplicate instance ids, ends at or before the start, and tag
//! normalization/capping.

// Test code is allowed to panic on failure.
#![allow(clippy::unwrap_used)]

use chrono::{DateTime, TimeDelta, Utc};
use hangouts_core::event::{MAX_TAG_CHARS, MAX_TAGS};
use ingest::{DEFAULT_EVENT_HOURS, map_page};

/// A fixed, parseable start in `America/New_York` (EST = UTC-5).
const START: &str = "2026-11-20T09:00:00-05:00";
/// An end two hours after [`START`].
const END: &str = "2026-11-20T11:00:00-05:00";

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-19T12:00:00Z")
        .unwrap()
        .to_utc()
}

/// Wraps comma-separated event wrappers (`{"event":{...}}` objects) into a full page body.
fn page(events: &str) -> String {
    format!(r#"{{"events":[{events}],"page":{{"current":1,"next_page":null}}}}"#)
}

/// One event wrapper. `rest` is the remainder of the event object and must contain
/// `event_instances`.
fn event(title: &str, rest: &str) -> String {
    format!(r#"{{"event":{{"id":1,"title":"{title}","status":"live",{rest}}}}}"#)
}

/// One event-instance wrapper; `end` of `None` means the feed omitted it.
fn instance(id: u64, start: &str, end: Option<&str>, all_day: bool) -> String {
    let end = end.map_or_else(|| "null".to_owned(), |e| format!("\"{e}\""));
    format!(
        r#"{{"event_instance":{{"id":{id},"start":"{start}","end":{end},"all_day":{all_day}}}}}"#
    )
}

#[test]
fn virtual_event_without_location_is_online() {
    let virtual_e = event(
        "Zoom Lecture",
        &format!(
            r#""experience":"virtual","stream_url":"meet.google.com/abc","event_instances":[{}]"#,
            instance(10, START, Some(END), false)
        ),
    );
    let hybrid_e = event(
        "Hybrid Meetup",
        &format!(
            r#""experience":"hybrid","event_instances":[{}]"#,
            instance(11, START, Some(END), false)
        ),
    );
    let in_person = event(
        "In Person Meetup",
        &format!(
            r#""event_instances":[{}]"#,
            instance(12, START, Some(END), false)
        ),
    );
    let events = map_page(&page(&[virtual_e, hybrid_e, in_person].join(",")), now()).unwrap();
    assert_eq!(events.len(), 3);

    // Virtual event with no venue is placed "Online" and gets the "virtual" tag.
    assert_eq!(events[0].location.name, "Online");
    assert_eq!(events[0].location.room, None);
    assert_eq!(
        events[0].location.online_url.as_deref(),
        Some("https://meet.google.com/abc"),
        "a scheme-less stream url gets an https prefix"
    );
    assert!(events[0].tags.contains(&"virtual".to_owned()));

    // Hybrid counts as online too.
    assert_eq!(events[1].location.name, "Online");
    assert!(events[1].tags.contains(&"virtual".to_owned()));

    // Only virtual/hybrid events become "Online"; the rest fall back to "TBA".
    assert_eq!(events[2].location.name, "TBA");
    assert!(!events[2].tags.contains(&"virtual".to_owned()));
    assert_eq!(events[2].location.online_url, None);
}

#[test]
fn html_entities_in_titles_are_kept_readable() {
    let entity_title = "KSU &amp; the City: Fall &ldquo;Fling&rdquo; (5 &lt; 6)";
    let entity_e = event(
        entity_title,
        &format!(
            r#""event_instances":[{}]"#,
            instance(10, START, Some(END), false)
        ),
    );
    let padded_e = event(
        "   Padded Title   ",
        &format!(
            r#""event_instances":[{}]"#,
            instance(11, START, Some(END), false)
        ),
    );
    let events = map_page(&page(&[entity_e, padded_e].join(",")), now()).unwrap();
    assert_eq!(events.len(), 2);

    // The mapper does not decode HTML entities; the title must at least survive intact
    // (apart from trimming) so the card stays readable.
    assert_eq!(events[0].title, entity_title);
    assert_eq!(events[1].title, "Padded Title");
    assert_eq!(events[0].created_at, now());
}

#[test]
fn duplicate_instance_ids_are_deduplicated() {
    let first = event(
        "First Event",
        &format!(
            r#""event_instances":[{},{}]"#,
            instance(10, START, Some(END), false),
            instance(10, START, Some(END), false)
        ),
    );
    let second = event(
        "Second Event",
        &format!(
            r#""event_instances":[{}]"#,
            instance(10, START, Some(END), false)
        ),
    );
    let third = event(
        "Third Event",
        &format!(
            r#""event_instances":[{}]"#,
            instance(11, START, Some(END), false)
        ),
    );
    let events = map_page(&page(&[first, second, third].join(",")), now()).unwrap();

    // Instance 10 appears three times (twice in one event, once in another): kept once.
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].title, "First Event", "the first occurrence wins");
    assert_eq!(events[0].external_id.as_deref(), Some("ksu:10"));
    assert_eq!(events[1].title, "Third Event");
    assert_eq!(events[1].external_id.as_deref(), Some("ksu:11"));
}

#[test]
fn end_before_start_gets_default_length() {
    let backwards = event(
        "Backwards Event",
        &format!(
            r#""event_instances":[{}]"#,
            instance(10, START, Some("2026-11-20T08:00:00-05:00"), false)
        ),
    );
    let zero_length = event(
        "Zero Length Event",
        &format!(
            r#""event_instances":[{}]"#,
            instance(11, START, Some(START), false)
        ),
    );
    let sane = event(
        "Sane Event",
        &format!(
            r#""event_instances":[{}]"#,
            instance(12, START, Some(END), false)
        ),
    );
    let events = map_page(&page(&[backwards, zero_length, sane].join(",")), now()).unwrap();
    assert_eq!(events.len(), 3);

    assert_eq!(
        events[0].starts_at.to_rfc3339(),
        "2026-11-20T14:00:00+00:00"
    );
    // An end at or before the start is dropped and the default length applies.
    for e in &events[0..2] {
        assert_eq!(
            e.ends_at - e.starts_at,
            TimeDelta::hours(DEFAULT_EVENT_HOURS),
            "{} should get the default length",
            e.title
        );
    }
    // A real end is respected.
    assert_eq!(events[2].ends_at - events[2].starts_at, TimeDelta::hours(2));
}

#[test]
fn tags_are_normalized_and_capped() {
    // Fourteen raw tags: a 40-char tag (per-tag cap), messy casing/punctuation,
    // duplicates, and enough distinct tags to hit the per-event cap.
    let long_tag = "x".repeat(40);
    let raw: &[&str] = &[
        &long_tag,    //   → 32 chars (per-tag cap)
        "Alpha One",  // → alpha-one
        "alpha-one",  //   duplicate, dropped
        "#Beta",      //   → beta
        "BETA",       //   duplicate, dropped
        "Gamma_Beta", //  → gamma-beta
        "delta  two", //  → delta-two
        "Epsilon",    //   → epsilon
        "zeta-omega", // → zeta-omega
        "Eta 3",      // → eta-3
        "Theta",      //   → theta
        "Iota",       //   → iota
        "kappa",      // → kappa (11th distinct: cut by the cap)
        "lambda",     // → lambda (12th distinct: cut by the cap)
    ];
    let tags = raw
        .iter()
        .map(|t| format!("\"{t}\""))
        .collect::<Vec<_>>()
        .join(",");
    let body = page(&event(
        "Quiet Hours",
        &format!(
            r#""tags":[{tags}],"event_instances":[{}]"#,
            instance(10, START, Some(END), false)
        ),
    ));
    let events = map_page(&body, now()).unwrap();
    assert_eq!(events.len(), 1);

    let mut expected = vec!["x".repeat(MAX_TAG_CHARS)];
    expected.extend(
        [
            "alpha-one",
            "beta",
            "gamma-beta",
            "delta-two",
            "epsilon",
            "zeta-omega",
            "eta-3",
            "theta",
            "iota",
        ]
        .map(String::from),
    );
    assert_eq!(events[0].tags, expected);
    assert_eq!(events[0].tags.len(), MAX_TAGS);
}

#[test]
fn keywords_and_event_types_join_the_tag_pool() {
    let body = page(&event(
        "Quiet Hours",
        &format!(
            r#""keywords":["Piano"],"filters":{{"event_types":[{{"name":"Poetry"}}]}},
               "event_instances":[{}]"#,
            instance(10, START, Some(END), false)
        ),
    ));
    let events = map_page(&body, now()).unwrap();
    assert_eq!(events[0].tags, vec!["piano", "poetry"]);
}
