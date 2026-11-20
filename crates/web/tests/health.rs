//! Pattern for API tests: build the router on an in-memory store and call it with `oneshot`.

#![allow(clippy::unwrap_used, clippy::expect_used)] // test helpers may unwrap

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn test_app() -> axum::Router {
    let store = store::Store::in_memory().await.unwrap();
    store.migrate().await.unwrap();
    // Port 9 (discard) refuses fast, so the LLM is reliably "down" in tests.
    let llm = llm::LlmClient::new(llm::LlmConfig {
        base_url: "http://127.0.0.1:9/v1".into(),
        ..llm::LlmConfig::default()
    })
    .unwrap();
    web::app(web::AppState {
        store,
        llm: Arc::new(llm),
    })
}

#[tokio::test]
async fn health_reports_ok_and_llm_down() {
    let response = test_app()
        .await
        .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: api::HealthResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        body,
        api::HealthResponse {
            ok: true,
            llm: false
        }
    );
}
