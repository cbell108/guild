//! A7: buddy matching.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

mod common;

use api::BuddyStatus;
use axum::http::StatusCode;
use common::TestApp;
use hangouts_core::PrivacySettings;
use serde_json::json;

fn uri(event: hangouts_core::EventId) -> String {
    format!("/api/events/{event}/buddy")
}

#[tokio::test]
async fn toggle_and_status() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let event = app.event(1, "E", 3, None).await;
    assert_eq!(
        app.get(&uri(event), None).await.status,
        StatusCode::UNAUTHORIZED
    );

    let status: BuddyStatus = app.get(&uri(event), Some(me)).await.json();
    assert_eq!(
        status,
        BuddyStatus {
            looking: false,
            matched: None
        }
    );

    let status: BuddyStatus = app
        .post(&uri(event), Some(me), json!({"looking": true}))
        .await
        .json();
    assert!(
        status.looking && status.matched.is_none(),
        "nobody else is looking yet"
    );
    let detail: api::EventDetail = app
        .get(&format!("/api/events/{event}"), Some(me))
        .await
        .json();
    assert!(detail.buddy_looking);

    let status: BuddyStatus = app
        .post(&uri(event), Some(me), json!({"looking": false}))
        .await
        .json();
    assert!(!status.looking);

    let missing = hangouts_core::EventId::from_u128(9);
    assert_eq!(
        app.post(&uri(missing), Some(me), json!({"looking": true}))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn friend_is_matched_before_strangers() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let stranger = app.user(2, "Stranger").await;
    let pal = app.user(3, "Jordan").await;
    app.befriend(me, pal).await;
    let event = app.event(1, "E", 3, None).await;
    // the stranger asked first, but the friend wins
    app.post(&uri(event), Some(stranger), json!({"looking": true}))
        .await;
    app.post(&uri(event), Some(pal), json!({"looking": true}))
        .await;
    let status: BuddyStatus = app
        .post(&uri(event), Some(me), json!({"looking": true}))
        .await
        .json();
    let matched = status.matched.unwrap();
    assert!(matched.is_friend);
    assert_eq!(matched.user.id, pal);
    assert_eq!(matched.reason, "Jordan is looking too");
}

#[tokio::test]
async fn falls_back_to_similar_user_and_respects_opt_out() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let other = app.user(2, "Other").await;
    let private = app.user(3, "Private").await;
    app.store()
        .set_privacy(
            private,
            &PrivacySettings {
                buddy_matching: false,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let event = app.event(1, "E", 3, None).await;
    app.post(&uri(event), Some(private), json!({"looking": true}))
        .await;
    let status: BuddyStatus = app
        .post(&uri(event), Some(me), json!({"looking": true}))
        .await
        .json();
    assert!(
        status.matched.is_none(),
        "users who opted out are never suggested"
    );

    app.post(&uri(event), Some(other), json!({"looking": true}))
        .await;
    let status: BuddyStatus = app.get(&uri(event), Some(me)).await.json();
    let matched = status.matched.unwrap();
    assert!(!matched.is_friend);
    assert_eq!(matched.user.id, other);
    assert!(!matched.reason.is_empty());
    // the match never leaks beyond a UserSummary
    assert!(
        app.get(&uri(event), Some(me)).await.value()["matched"]["user"]
            .get("interests")
            .is_none()
    );

    // the opted-out user gets no match at all
    let status: BuddyStatus = app.get(&uri(event), Some(private)).await.json();
    assert!(status.looking && status.matched.is_none());
}
