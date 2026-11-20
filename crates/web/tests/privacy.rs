//! A12: privacy settings and history deletion.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

mod common;

use api::{BuddyStatus, EventDetail, Me};
use axum::http::StatusCode;
use common::TestApp;
use hangouts_core::{PrivacySettings, Visibility};
use serde_json::json;

#[tokio::test]
async fn privacy_defaults_and_update() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    assert_eq!(
        app.get("/api/me/privacy", None).await.status,
        StatusCode::UNAUTHORIZED
    );

    let p: PrivacySettings = app.get("/api/me/privacy", Some(me)).await.json();
    assert_eq!(p, PrivacySettings::default());
    assert_eq!(p.attendance, Visibility::Friends);

    let body = json!({"attendance": "private", "buddy_matching": false, "personalize": false});
    let resp = app.put("/api/me/privacy", Some(me), body).await;
    assert_eq!(resp.status, StatusCode::OK);
    let updated: PrivacySettings = resp.json();
    assert_eq!(updated.attendance, Visibility::Private);
    assert!(!updated.buddy_matching && !updated.personalize);

    let p: PrivacySettings = app.get("/api/me/privacy", Some(me)).await.json();
    assert_eq!(p, updated);
    let who: Me = app.get("/api/me", Some(me)).await.json();
    assert_eq!(who.privacy, updated);

    // there is no "public" option
    let resp = app
        .put(
            "/api/me/privacy",
            Some(me),
            json!({"attendance": "public", "buddy_matching": true, "personalize": true}),
        )
        .await;
    assert!(resp.status.is_client_error());
}

#[tokio::test]
async fn private_attendance_hides_from_friends() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let pal = app.user(2, "Pal").await;
    app.befriend(me, pal).await;
    let event = app.event(1, "E", 3, None).await;
    app.put(
        &format!("/api/events/{event}/intent"),
        Some(pal),
        json!({"level": "going"}),
    )
    .await;
    assert_eq!(friends_going(&app, me, event).await, 1);
    app.put(
        "/api/me/privacy",
        Some(pal),
        json!({"attendance": "private", "buddy_matching": true, "personalize": true}),
    )
    .await;
    assert_eq!(friends_going(&app, me, event).await, 0);
}

#[tokio::test]
async fn delete_history_clears_activity_but_keeps_the_account() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let pal = app.user(2, "Pal").await;
    app.befriend(me, pal).await;
    let event = app.event(1, "E", 3, None).await;
    app.put(
        &format!("/api/events/{event}/intent"),
        Some(me),
        json!({"level": "going"}),
    )
    .await;
    app.post(
        &format!("/api/events/{event}/buddy"),
        Some(me),
        json!({"looking": true}),
    )
    .await;
    app.put(
        "/api/me/availability",
        Some(me),
        json!({"date": "2026-11-21"}),
    )
    .await;

    assert_eq!(
        app.delete("/api/me/history", None).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.delete("/api/me/history", Some(me)).await.status,
        StatusCode::NO_CONTENT
    );

    let detail: EventDetail = app
        .get(&format!("/api/events/{event}"), Some(me))
        .await
        .json();
    assert!(detail.card.my_intent.is_none() && !detail.buddy_looking);
    let status: BuddyStatus = app
        .get(&format!("/api/events/{event}/buddy"), Some(me))
        .await
        .json();
    assert!(!status.looking);
    let day = hangouts_core::time::parse_date("2026-11-21").unwrap();
    assert!(
        app.store()
            .list_availability_on(day, None)
            .await
            .unwrap()
            .is_empty()
    );
    // account and friendships stay
    assert_eq!(app.get("/api/me", Some(me)).await.status, StatusCode::OK);
    let friends: api::FriendsResponse = app.get("/api/friends", Some(me)).await.json();
    assert_eq!(friends.friends.len(), 1);
}

/// How many of the viewer's friends are shown as going.
async fn friends_going(
    app: &TestApp,
    viewer: hangouts_core::UserId,
    event: hangouts_core::EventId,
) -> u32 {
    let detail: EventDetail = app
        .get(&format!("/api/events/{event}"), Some(viewer))
        .await
        .json();
    detail.card.friends.going
}
