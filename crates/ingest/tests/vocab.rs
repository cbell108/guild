//! KSU categories → interest vocabulary, checked against the seeded vocabulary and the saved
//! calendar response. No network.

// Test code is allowed to panic on failure.
#![allow(clippy::unwrap_used)]

use chrono::{DateTime, Utc};

const PAGE: &str = include_str!("fixtures/ksu_page1.json");

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-19T12:00:00Z")
        .unwrap()
        .to_utc()
}

fn tags(ksu: &[&str], dept: Option<&str>, title: &str) -> Vec<&'static str> {
    let ksu: Vec<String> = ksu.iter().map(|s| (*s).to_owned()).collect();
    ingest::interest_tags(&ksu, dept, title)
}

#[test]
fn every_mapped_interest_is_in_the_seeded_vocabulary() {
    let vocab = seed::interest_vocabulary();
    for tag in ingest::mapped_interests() {
        assert!(vocab.contains(tag), "{tag} is not a seeded interest");
    }
}

#[test]
fn ksu_categories_map_to_interests() {
    assert_eq!(tags(&["lectures-guest-speakers"], None, "Dr. X"), ["talks"]);
    assert_eq!(
        tags(
            &["athletics"],
            Some("Athletics"),
            "Men's Basketball vs UTEP"
        ),
        ["sports", "basketball"]
    );
    assert_eq!(
        tags(&["arts"], Some("Bailey School of Music"), "Holiday Concert"),
        ["music", "concerts"]
    );
    assert_eq!(
        tags(
            &[],
            Some("Department of Career Planning and Development"),
            "Drop-in"
        ),
        ["career"]
    );
    assert_eq!(
        tags(&["training-workshops"], None, "Revision Workshop"),
        ["workshops"]
    );
    assert_eq!(
        tags(&[], None, "Board meeting").len(),
        0,
        "board is not a board game"
    );
    assert_eq!(tags(&["virtual"], None, "Campus Tour").len(), 0);
}

#[test]
fn fixture_events_get_interest_tags_first_and_keep_ksu_tags() {
    let vocab = seed::interest_vocabulary();
    let events = ingest::map_page(PAGE, now()).unwrap();
    let with_interest = events
        .iter()
        .filter(|e| e.tags.iter().any(|t| vocab.contains(t.as_str())))
        .count();
    assert!(
        with_interest >= 5,
        "only {with_interest}/{} fixture events carry an interest tag",
        events.len()
    );
    for e in &events {
        assert!(e.tags.len() <= hangouts_core::event::MAX_TAGS);
    }
    let spotlight = events
        .iter()
        .find(|e| e.external_id.as_deref() == Some("ksu:54303721792461"))
        .unwrap();
    assert!(spotlight.tags.contains(&"info-sessions-tours".to_owned()));
}
