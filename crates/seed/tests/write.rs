//! `seed::write` against a real (in-memory, migrated) store: what goes in comes back out.

// Test code is allowed to panic on failure.
#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use hangouts_core::{IntentStatus, Snapshot};
use seed::{SeedConfig, generate, write};
use store::Store;

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-19T17:00:00Z")
        .unwrap()
        .to_utc()
}

async fn written(snapshot: &Snapshot) -> Snapshot {
    let store = Store::in_memory().await.unwrap();
    store.migrate().await.unwrap();
    write(&store, snapshot).await.unwrap();
    store.load_snapshot().await.unwrap()
}

fn status_counts(s: &Snapshot) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for i in &s.intents {
        let key = match i.status {
            IntentStatus::Confirmed => "confirmed",
            IntentStatus::Waitlisted => "waitlisted",
        };
        *counts.entry(key).or_default() += 1;
    }
    counts
}

#[tokio::test]
async fn write_round_trips_every_table_count() {
    let generated = generate(&SeedConfig::demo(now()));
    let loaded = written(&generated).await;

    assert_eq!(loaded.users.len(), generated.users.len());
    assert_eq!(loaded.events.len(), generated.events.len());
    assert_eq!(loaded.crews.len(), generated.crews.len());
    assert_eq!(loaded.crew_members.len(), generated.crew_members.len());
    assert_eq!(loaded.friendships.len(), generated.friendships.len());
    assert_eq!(loaded.intents.len(), generated.intents.len());
    assert_eq!(loaded.checkins.len(), generated.checkins.len());
    assert_eq!(loaded.buddy_requests.len(), generated.buddy_requests.len());
    assert_eq!(
        loaded.event_embeddings.len(),
        generated.event_embeddings.len()
    );
    assert!(!loaded.users.is_empty() && !loaded.intents.is_empty());
}

#[tokio::test]
async fn write_preserves_waitlist_decisions() {
    let generated = generate(&SeedConfig::demo(now()));
    let loaded = written(&generated).await;

    assert_eq!(status_counts(&loaded), status_counts(&generated));
    assert!(
        status_counts(&loaded).contains_key("waitlisted"),
        "demo data should include waitlisted intents"
    );
    let by_pair = |s: &Snapshot| {
        s.intents
            .iter()
            .map(|i| ((i.user_id, i.event_id), (i.level, i.status)))
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(by_pair(&loaded), by_pair(&generated));
}

#[tokio::test]
async fn write_preserves_entities_and_embeddings() {
    let generated = generate(&SeedConfig::demo(now()));
    let loaded = written(&generated).await;

    let users = |s: &Snapshot| {
        s.users
            .iter()
            .map(|u| (u.id, u.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    let events = |s: &Snapshot| {
        s.events
            .iter()
            .map(|e| (e.id, e.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(users(&loaded), users(&generated));
    assert_eq!(events(&loaded), events(&generated));
    assert_eq!(loaded.event_embeddings, generated.event_embeddings);
}

#[tokio::test]
async fn writing_is_deterministic() {
    let cfg = SeedConfig::demo(now());
    let a = written(&generate(&cfg)).await;
    let b = written(&generate(&cfg)).await;
    assert_eq!(a, b);
}
