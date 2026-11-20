//! A6: the feed endpoint, embeddings backfill and friend presence.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

mod common;

use api::FeedResponse;
use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn feed_requires_login() {
    let app = TestApp::new().await;
    assert_eq!(
        app.get("/api/feed", None).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn feed_lists_next_seven_days_and_is_finite() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    for n in 0..12 {
        app.event(
            100 + n,
            &format!("Event {n}"),
            2 + i64::from(n as u32),
            None,
        )
        .await;
    }
    app.event(500, "Far away", 24 * 30, None).await;
    app.event(501, "Already over", -10, None).await;

    let feed: FeedResponse = app.get("/api/feed", Some(me)).await.json();
    assert!(feed.caught_up);
    assert_eq!(feed.items.len(), 10);
    assert!(feed.items.iter().all(|i| !i.reasons.is_empty()));
    assert!(
        feed.items
            .iter()
            .all(|i| i.event.title != "Far away" && i.event.title != "Already over")
    );
}

#[tokio::test]
async fn feed_cards_carry_friend_presence_and_my_intent() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let pal = app.user(2, "Jordan").await;
    app.befriend(me, pal).await;
    let event = app.event(1, "Game night", 5, None).await;
    app.put(
        &format!("/api/events/{event}/intent"),
        Some(pal),
        json!({"level": "going"}),
    )
    .await;
    app.put(
        &format!("/api/events/{event}/intent"),
        Some(me),
        json!({"level": "interested"}),
    )
    .await;

    let feed: FeedResponse = app.get("/api/feed", Some(me)).await.json();
    let card = &feed.items[0].event;
    assert_eq!(card.friends.going, 1);
    assert_eq!(card.friends.sample[0].display_name, "Jordan");
    assert!(card.my_intent.is_some());
}

#[tokio::test]
async fn backfill_embeds_missing_events_once() {
    let app = TestApp::new().await;
    app.event(1, "A", 3, None).await;
    app.event(2, "B", 4, None).await;
    assert_eq!(
        app.store().events_missing_embedding().await.unwrap().len(),
        2
    );
    assert_eq!(web::embedding::backfill(&app.state).await, 2);
    assert_eq!(web::embedding::backfill(&app.state).await, 0);
    let snap = app.store().load_snapshot().await.unwrap();
    assert_eq!(snap.event_embeddings.len(), 2);
    assert_eq!(
        snap.event_embeddings.values().next().unwrap().len(),
        app.state.embedder.dim()
    );
}

#[tokio::test]
async fn retrain_model_succeeds_on_empty_and_seeded_db() {
    let app = TestApp::new().await;
    app.state.retrain_model().await.unwrap();
}
