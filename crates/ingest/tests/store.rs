//! The ingest upsert path against an in-memory store, fed from the saved fixture. No network.

// Test code is allowed to panic on failure.
#![allow(clippy::unwrap_used)]

use chrono::{DateTime, Utc};
use hangouts_core::{Event, EventId};
use ingest::{IngestError, IngestReport};
use store::Store;

const PAGE: &str = include_str!("fixtures/ksu_page1.json");

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-19T12:00:00Z")
        .unwrap()
        .to_utc()
}

async fn fresh_store() -> Store {
    let store = Store::in_memory().await.unwrap();
    store.migrate().await.unwrap();
    store
}

/// Serves `body` as page 1 and an empty last page after it (the fixture links to a page 2).
async fn run_fixture(store: &Store, body: &str) -> IngestReport {
    let body = body.to_owned();
    ingest::run_with(store, now(), move |page| {
        let body = if page == 1 {
            body.clone()
        } else {
            r#"{"events":[],"page":{"current":2,"next_page":null}}"#.to_owned()
        };
        async move { Ok::<_, IngestError>(body) }
    })
    .await
    .unwrap()
}

async fn events(store: &Store) -> Vec<Event> {
    let mut events = store.load_snapshot().await.unwrap().events;
    events.sort_by_key(|e| e.id);
    events
}

#[tokio::test]
async fn upsert_twice_is_idempotent() {
    let store = fresh_store().await;
    let mapped = ingest::map_page(PAGE, now()).unwrap();

    let first = run_fixture(&store, PAGE).await;
    assert_eq!(first.fetched, mapped.len());
    let after_first = events(&store).await;
    assert_eq!(after_first.len(), mapped.len());

    let second = run_fixture(&store, PAGE).await;
    assert_eq!(second, first);
    let after_second = events(&store).await;
    assert_eq!(after_first, after_second, "no duplicates, nothing changed");

    // The stored ids are the deterministic ones derived from the external ids.
    let mut expected: Vec<EventId> = mapped.iter().map(|e| e.id).collect();
    expected.sort();
    let stored: Vec<EventId> = after_second.iter().map(|e| e.id).collect();
    assert_eq!(stored, expected);
}

#[tokio::test]
async fn upsert_overwrites_changed_fields_and_keeps_id() {
    let store = fresh_store().await;
    run_fixture(&store, PAGE).await;

    let mut changed = ingest::map_page(PAGE, now()).unwrap();
    let original = changed[0].clone();
    changed[0].title = "Renamed by the organizer".to_owned();
    changed[0].location.name = "Somewhere Else".to_owned();
    changed[0].tags = vec!["updated".to_owned()];
    let id = store.upsert_official_event(&changed[0]).await.unwrap();
    assert_eq!(id, original.id, "existing row keeps its id");

    let all = events(&store).await;
    assert_eq!(all.len(), changed.len());
    let row = all.iter().find(|e| e.id == original.id).unwrap();
    assert_eq!(row.title, "Renamed by the organizer");
    assert_eq!(row.location.name, "Somewhere Else");
    assert_eq!(row.tags, vec!["updated".to_owned()]);
    assert_eq!(row.external_id, original.external_id);
}

#[tokio::test]
async fn run_with_follows_next_page_and_stops() {
    let store = fresh_store().await;
    // Two pages: the fixture, then an empty page that has no next page.
    let empty = r#"{"events":[],"page":{"current":2,"next_page":null}}"#;
    let mut requested = Vec::new();
    let report = ingest::run_with(&store, now(), |page| {
        requested.push(page);
        let body = if page == 1 {
            with_next_page(PAGE)
        } else {
            empty.to_owned()
        };
        async move { Ok::<_, IngestError>(body) }
    })
    .await
    .unwrap();
    assert_eq!(requested, vec![1, 2]);
    assert_eq!(report.fetched, 20);
}

/// Rewrites the fixture's `next_page` to 2.
fn with_next_page(body: &str) -> String {
    let mut v: serde_json::Value = serde_json::from_str(body).unwrap();
    v["page"]["next_page"] = serde_json::json!(2);
    v.to_string()
}

#[tokio::test]
async fn fetch_errors_propagate() {
    let store = fresh_store().await;
    let err = ingest::run_with(&store, now(), |_| async {
        Err::<String, _>(IngestError::Http("boom".into()))
    })
    .await
    .unwrap_err();
    assert!(matches!(err, IngestError::Http(_)));
    assert_eq!(events(&store).await.len(), 0);
}
