//! axum JSON API (see `docs/API.md`, wire types in the `api` crate) and the static frontend.
//!
//! [`app`] builds the router from an [`AppState`], so integration tests in `tests/` drive it
//! with `tower::ServiceExt::oneshot` and no network.

use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use llm::LlmClient;
use store::Store;

/// Shared state for handlers. Lane A may add fields.
#[derive(Clone)]
pub struct AppState {
    /// Database.
    pub store: Store,
    /// Local LLM client.
    pub llm: Arc<LlmClient>,
}

/// The full router.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<api::HealthResponse> {
    Json(api::HealthResponse {
        ok: true,
        llm: state.llm.health().await,
    })
}
