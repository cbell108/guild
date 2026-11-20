//! axum JSON API (see `docs/API.md`, wire types in the `api` crate) and the static frontend.
//!
//! [`app`] builds the router from an [`AppState`], so integration tests in `tests/` drive it
//! with `tower::ServiceExt::oneshot` and no network.

use axum::routing::{get, post, put};
use axum::{Json, Router, extract::State};

pub mod auth;
pub mod embedding;
pub mod error;
pub mod routes;
pub mod state;
pub mod view;

pub use state::{AppState, Clock};

use routes::{auth as auth_routes, events, intents};

/// The full router.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/demo-users", get(auth_routes::demo_users))
        .route("/api/login", post(auth_routes::login))
        .route("/api/logout", post(auth_routes::logout))
        .route("/api/me", get(auth_routes::get_me))
        .route("/api/users", get(auth_routes::search_users))
        .route("/api/events", get(events::list).post(events::create))
        .route("/api/events/{id}", get(events::get))
        .route(
            "/api/events/{id}/intent",
            put(intents::set).delete(intents::remove),
        )
        .route("/api/tags", get(events::tags))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<api::HealthResponse> {
    Json(api::HealthResponse {
        ok: true,
        llm: state.llm.health().await,
    })
}
