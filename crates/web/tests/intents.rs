//! A4: intents, capacity, waitlist and auto-promote.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

mod common;

use api::{EventDetail, MyIntent};
use axum::http::StatusCode;
use common::TestApp;
use hangouts_core::{IntentKind, IntentStatus, UserId};
use serde_json::json;

fn uri(event: hangouts_core::EventId) -> String {
    format!("/api/events/{event}/intent")
}

#[tokio::test]
async fn set_and_remove_intent() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let event = app.event(1, "E", 3, None).await;

    assert_eq!(
        app.put(&uri(event), None, json!({"level": "going"}))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );

    let resp = app
        .put(&uri(event), Some(me), json!({"level": "going"}))
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let intent: MyIntent = resp.json();
    assert_eq!(
        (intent.level, intent.status, intent.waitlist_position),
        (IntentKind::Going, IntentStatus::Confirmed, None)
    );

    let detail: EventDetail = app
        .get(&format!("/api/events/{event}"), Some(me))
        .await
        .json();
    assert_eq!(detail.card.my_intent, Some(intent));

    assert_eq!(
        app.delete(&uri(event), Some(me)).await.status,
        StatusCode::NO_CONTENT
    );
    // idempotent
    assert_eq!(
        app.delete(&uri(event), Some(me)).await.status,
        StatusCode::NO_CONTENT
    );
    let detail: EventDetail = app
        .get(&format!("/api/events/{event}"), Some(me))
        .await
        .json();
    assert!(detail.card.my_intent.is_none());
}

#[tokio::test]
async fn going_with_needs_an_accepted_friend() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let pal = app.user(2, "Pal").await;
    let stranger = app.user(3, "Stranger").await;
    app.befriend(me, pal).await;
    let event = app.event(1, "E", 3, None).await;

    let resp = app
        .put(&uri(event), Some(me), json!({"level": "going_with"}))
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    let resp = app
        .put(
            &uri(event),
            Some(me),
            json!({"level": "going_with", "with": stranger}),
        )
        .await;
    assert_eq!(
        (resp.status, resp.error_code().as_str()),
        (StatusCode::BAD_REQUEST, "bad_request")
    );
    let resp = app
        .put(
            &uri(event),
            Some(me),
            json!({"level": "going_with", "with": pal}),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let intent: MyIntent = resp.json();
    assert_eq!(intent.with.unwrap().display_name, "Pal");
}

#[tokio::test]
async fn unknown_event_is_404() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let missing = hangouts_core::EventId::from_u128(404);
    assert_eq!(
        app.put(&uri(missing), Some(me), json!({"level": "going"}))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.put(
            "/api/events/xyz/intent",
            Some(me),
            json!({"level": "going"})
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn full_event_waitlists_and_back_out_promotes() {
    let app = TestApp::new().await;
    let a = app.user(1, "A").await;
    let b = app.user(2, "B").await;
    let c = app.user(3, "C").await;
    let d = app.user(4, "D").await;
    let event = app.event(1, "Small", 3, Some(1)).await;
    let going = json!({"level": "going"});

    let first: MyIntent = app.put(&uri(event), Some(a), going.clone()).await.json();
    assert_eq!(first.status, IntentStatus::Confirmed);
    // interested never takes a spot
    let int: MyIntent = app
        .put(&uri(event), Some(d), json!({"level": "interested"}))
        .await
        .json();
    assert_eq!(int.status, IntentStatus::Confirmed);

    let second: MyIntent = app.put(&uri(event), Some(b), going.clone()).await.json();
    assert_eq!(
        (second.status, second.waitlist_position),
        (IntentStatus::Waitlisted, Some(1))
    );
    let third: MyIntent = app.put(&uri(event), Some(c), going.clone()).await.json();
    assert_eq!(
        (third.status, third.waitlist_position),
        (IntentStatus::Waitlisted, Some(2))
    );

    let detail: EventDetail = app
        .get(&format!("/api/events/{event}"), Some(a))
        .await
        .json();
    assert_eq!(detail.card.spots_left, Some(0));

    // the confirmed user backs out: B (earliest waitlisted) gets the spot, C moves up
    assert_eq!(
        app.delete(&uri(event), Some(a)).await.status,
        StatusCode::NO_CONTENT
    );
    let b_detail: EventDetail = app
        .get(&format!("/api/events/{event}"), Some(b))
        .await
        .json();
    assert_eq!(
        b_detail.card.my_intent.unwrap().status,
        IntentStatus::Confirmed
    );
    let c_detail: EventDetail = app
        .get(&format!("/api/events/{event}"), Some(c))
        .await
        .json();
    let c_intent = c_detail.card.my_intent.unwrap();
    assert_eq!(
        (c_intent.status, c_intent.waitlist_position),
        (IntentStatus::Waitlisted, Some(1))
    );
    assert_eq!(c_detail.card.spots_left, Some(0));
}

#[tokio::test]
async fn friend_presence_respects_privacy() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let open = app.user(2, "Open").await;
    let shy = app.user(3, "Shy").await;
    let stranger = app.user(4, "Stranger").await;
    app.befriend(me, open).await;
    app.befriend(me, shy).await;
    app.store()
        .set_privacy(
            shy,
            &hangouts_core::PrivacySettings {
                attendance: hangouts_core::Visibility::Private,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let event = app.event(1, "E", 3, None).await;
    for user in [open, shy, stranger] {
        app.put(&uri(event), Some(user), json!({"level": "going"}))
            .await;
    }
    let detail: EventDetail = app
        .get(&format!("/api/events/{event}"), Some(me))
        .await
        .json();
    let friends = detail.card.friends;
    assert_eq!((friends.going, friends.interested), (1, 0));
    assert_eq!(friends.sample.len(), 1);
    assert_eq!(friends.sample[0].id, open);

    // logged-out viewers see no friend presence
    let detail: EventDetail = app.get(&format!("/api/events/{event}"), None).await.json();
    assert_eq!(detail.card.friends, api::FriendPresence::default());
    let _ = UserId::from_u128(0);
}
