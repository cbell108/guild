//! `POST /api/ask` (natural-language Browse): validation and graceful degradation. The
//! LLM prompt and parsing are covered by `crates/llm` tests.
#![allow(clippy::unwrap_used)]

mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn ask_requires_login() {
    let app = TestApp::new().await;
    let resp = app
        .post("/api/ask", None, json!({"text": "free food tonight"}))
        .await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn ask_rejects_empty_and_oversized_text() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    for text in [String::new(), "  ".to_owned(), "x".repeat(301)] {
        let resp = app.post("/api/ask", Some(me), json!({"text": text})).await;
        assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn ask_is_503_with_friendly_message_when_llm_is_down() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let resp = app
        .post(
            "/api/ask",
            Some(me),
            json!({"text": "something chill tonight"}),
        )
        .await;
    assert_eq!(resp.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(resp.error_code(), "llm_unavailable");
    assert_eq!(app.get("/api/health", None).await.status, StatusCode::OK);
}
