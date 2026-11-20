//! `PUT /api/me/interests` (interest onboarding).
#![allow(clippy::unwrap_used)]

mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn requires_login() {
    let app = TestApp::new().await;
    let resp = app
        .put("/api/me/interests", None, json!({"interests": ["music"]}))
        .await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn saves_normalized_deduplicated_interests_and_me_reflects_them() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let resp = app
        .put(
            "/api/me/interests",
            Some(me),
            json!({"interests": ["Music", "  board games ", "music", "Free-Food"]}),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.value();
    let saved: Vec<&str> = body["interests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(saved, ["music", "board-games", "free-food"]);

    let again = app.get("/api/me", Some(me)).await.value();
    assert_eq!(again["interests"], body["interests"]);
}

#[tokio::test]
async fn can_be_replaced_and_cleared() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    app.put(
        "/api/me/interests",
        Some(me),
        json!({"interests": ["a1", "b2"]}),
    )
    .await;
    let resp = app
        .put("/api/me/interests", Some(me), json!({"interests": []}))
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.value()["interests"], json!([]));
}

#[tokio::test]
async fn rejects_too_many_interests() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let many: Vec<String> = (0..41).map(|i| format!("tag{i}")).collect();
    let resp = app
        .put("/api/me/interests", Some(me), json!({"interests": many}))
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
}
