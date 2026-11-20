//! A10: `/api/parse` degrades gracefully when the LLM is down.
#![allow(clippy::unwrap_used)]

mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn parse_requires_login() {
    let app = TestApp::new().await;
    let resp = app
        .post("/api/parse", None, json!({"text": "smash tonight"}))
        .await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn parse_rejects_empty_and_oversized_text() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    for text in [String::new(), "   ".to_owned(), "x".repeat(1001)] {
        let resp = app
            .post("/api/parse", Some(me), json!({"text": text}))
            .await;
        assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn parse_is_503_with_friendly_message_when_llm_is_down() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let resp = app
        .post(
            "/api/parse",
            Some(me),
            json!({"text": "smash tonight 7-10 game room"}),
        )
        .await;
    assert_eq!(resp.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(resp.error_code(), "llm_unavailable");
    let message = resp.value()["error"]["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(message.contains("form"), "{message}");
    // the rest of the app keeps working
    assert_eq!(app.get("/api/health", None).await.status, StatusCode::OK);
}
