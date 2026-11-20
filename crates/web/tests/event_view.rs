//! The targeted event-view snapshot must render exactly what the full snapshot does, and
//! writes must show up in the very next read (nothing is cached).
#![allow(clippy::unwrap_used)]

mod common;

use api::EventCard;
use chrono::{DateTime, Utc};
use common::TestApp;
use hangouts_core::UserId;
use serde_json::json;
use web::view;

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-19T17:00:00Z")
        .unwrap()
        .to_utc()
}

#[tokio::test]
async fn targeted_view_equals_full_snapshot_for_every_viewer_and_event() {
    let store = store::Store::in_memory().await.unwrap();
    store.migrate().await.unwrap();
    let generated = seed::generate(&seed::SeedConfig::demo(now()));
    seed::write(&store, &generated).await.unwrap();

    let full = store.load_snapshot().await.unwrap();
    let mut viewers: Vec<Option<UserId>> = full.users.iter().take(12).map(|u| Some(u.id)).collect();
    viewers.push(None);
    for viewer in viewers {
        // One batch of all events, like the list route.
        let batch = store.load_event_view(&full.events, viewer).await.unwrap();
        for event in &full.events {
            let want = view::detail(&full, event, viewer);
            // The batch view and a single-event view both agree with the full snapshot.
            assert_eq!(
                view::detail(&batch, event, viewer),
                want,
                "batch {viewer:?}"
            );
            let one = store
                .load_event_view(std::slice::from_ref(event), viewer)
                .await
                .unwrap();
            assert_eq!(view::detail(&one, event, viewer), want, "single {viewer:?}");
        }
    }
}

async fn list(app: &TestApp, me: UserId, event: hangouts_core::EventId) -> EventCard {
    let cards: Vec<EventCard> = app.get("/api/events", Some(me)).await.json();
    cards.into_iter().find(|c| c.id == event).unwrap()
}

#[tokio::test]
async fn an_rsvp_is_visible_in_the_next_list_and_detail_read() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let event = app.event(1, "E", 1, Some(1)).await;

    assert!(list(&app, me, event).await.my_intent.is_none());
    assert_eq!(list(&app, me, event).await.spots_left, Some(1));

    app.put(
        &format!("/api/events/{event}/intent"),
        Some(me),
        json!({"level": "going"}),
    )
    .await;
    let card = list(&app, me, event).await;
    assert!(card.my_intent.is_some());
    assert_eq!(card.spots_left, Some(0));

    app.delete(&format!("/api/events/{event}/intent"), Some(me))
        .await;
    let card = list(&app, me, event).await;
    assert!(card.my_intent.is_none());
    assert_eq!(card.spots_left, Some(1));
}

#[tokio::test]
async fn buddy_view_matches_full_snapshot_for_similar_users() {
    let store = store::Store::in_memory().await.unwrap();
    store.migrate().await.unwrap();
    let generated = seed::generate(&seed::SeedConfig::demo(now()));
    seed::write(&store, &generated).await.unwrap();
    let full = store.load_snapshot().await.unwrap();
    let model = recommend::AttendanceModel::prior();
    let embedder = recommend::HashEmbedder::default();

    let mut compared = 0;
    for event in full.events.iter().map(|e| e.id) {
        let seekers: Vec<UserId> = full
            .buddy_requests
            .iter()
            .filter(|b| b.event_id == event)
            .map(|b| b.user_id)
            .collect();
        for &viewer in &seekers {
            let others: Vec<UserId> = seekers.iter().copied().filter(|u| *u != viewer).collect();
            let small = store.load_buddy_view(event, viewer).await.unwrap();
            let rank = |snapshot| {
                let ctx = recommend::RecContext {
                    snapshot,
                    user: viewer,
                    now: now(),
                    model: &model,
                    embedder: &embedder,
                };
                recommend::similar_users(&ctx, &others, 3)
            };
            assert_eq!(
                rank(&small),
                rank(&full),
                "viewer {viewer:?} event {event:?}"
            );
            assert_eq!(small.friends_of(viewer), full.friends_of(viewer));
            assert_eq!(small.buddy_requests.len(), seekers.len());
            assert!(small.event(event).is_some());
            compared += 1;
        }
    }
    assert!(
        compared >= 2,
        "seed data should have buddy seekers ({compared})"
    );
}
