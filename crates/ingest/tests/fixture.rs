//! `map_page` against a real KSU calendar response saved on 2026-11-19
//! (`days=30&pp=20&page=1`). No network.

#![allow(clippy::unwrap_used)]

use chrono::{DateTime, Utc};
use hangouts_core::EventKind;

const PAGE: &str = include_str!("fixtures/ksu_page1.json");

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-19T12:00:00Z")
        .unwrap()
        .to_utc()
}

#[test]
fn maps_every_instance_of_the_fixture() {
    let events = ingest::map_page(PAGE, now()).unwrap();
    assert_eq!(events.len(), 20);
    for e in &events {
        assert_eq!(e.kind, EventKind::Official);
        assert!(e.ends_at > e.starts_at, "{}", e.title);
        assert_ne!(e.title, "");
        assert_ne!(e.location.name, "");
        assert!(!e.description.contains('<'), "no HTML in {}", e.title);
        assert!(e.external_id.as_deref().unwrap().starts_with("ksu:"));
        assert!(e.tags.len() <= hangouts_core::event::MAX_TAGS);
        assert_eq!(e.created_at, now());
    }
    let mut ids: Vec<_> = events.iter().map(|e| e.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), events.len());
}

#[test]
fn maps_fields_of_a_known_event() {
    let events = ingest::map_page(PAGE, now()).unwrap();
    let tour = events
        .iter()
        .find(|e| e.external_id.as_deref() == Some("ksu:54303721792461"))
        .unwrap();
    assert_eq!(
        tour.title,
        "10AM Kennesaw Campus Spotlight Tour: B-12 Teacher Education"
    );
    assert_eq!(tour.starts_at.to_rfc3339(), "2026-11-19T15:00:00+00:00");
    assert_eq!(tour.ends_at.to_rfc3339(), "2026-11-19T18:00:00+00:00");
    assert_eq!(tour.location.name, "Carmichael Student Center");
    assert_eq!(tour.location.room, None);
    assert!((tour.location.lat.unwrap() - 34.038_633).abs() < 1e-6);
    assert_eq!(tour.organizer.as_deref(), Some("Visit KSU"));
    assert!(tour.tags.contains(&"kennesaw-campus-tour".to_owned()));
    assert!(tour.tags.contains(&"info-sessions-tours".to_owned()));
    assert!(
        tour.url
            .as_deref()
            .unwrap()
            .starts_with("https://calendar.kennesaw.edu/event/")
    );
}

#[test]
fn virtual_event_is_online_with_a_link() {
    let events = ingest::map_page(PAGE, now()).unwrap();
    let e = events
        .iter()
        .find(|e| e.external_id.as_deref() == Some("ksu:53744714532551"))
        .unwrap();
    assert_eq!(e.location.name, "Online");
    assert!(e.tags.contains(&"virtual".to_owned()));
    assert!(
        e.location
            .online_url
            .as_deref()
            .unwrap()
            .starts_with("https://")
    );
}

#[test]
fn reports_next_page() {
    assert_eq!(ingest::next_page(PAGE).unwrap(), Some(2));
}
