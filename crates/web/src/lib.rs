//! axum JSON API (see `docs/API.md`, wire types in the `api` crate) and the static frontend.
//!
//! [`app`] builds the router from an [`AppState`], so integration tests in `tests/` drive it
//! with `tower::ServiceExt::oneshot` and no network.

use axum::routing::{any, delete, get, post, put};
use axum::{Json, Router, extract::State};
use tower_http::services::{ServeDir, ServeFile};

pub mod auth;
pub mod embedding;
pub mod error;
pub mod routes;
pub mod state;
pub mod view;

pub use state::{AppState, Clock};

use routes::{
    ask, auth as auth_routes, buddy, calendar, checkin, crews, events, feed, friends, intents,
    interests, parse, privacy,
};

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
        .route("/api/events/{id}/calendar.ics", get(calendar::ics))
        .route(
            "/api/events/{id}/buddy",
            get(buddy::get).post(buddy::toggle),
        )
        .route("/api/events/{id}/qr", get(checkin::qr))
        .route("/api/checkin", post(checkin::checkin))
        .route("/api/me/privacy", get(privacy::get).put(privacy::put))
        .route("/api/me/history", delete(privacy::delete_history))
        .route("/api/me/interests", put(interests::put))
        .route("/api/ask", post(ask::ask))
        .route("/api/feed", get(feed::feed))
        .route("/api/parse", post(parse::parse))
        .route("/api/friends", get(friends::list).post(friends::request))
        .route("/api/friends/{user_id}/accept", post(friends::accept))
        .route("/api/friends/{user_id}", delete(friends::remove))
        .route("/api/crews", get(crews::list).post(crews::create))
        .route("/api/crews/{id}", get(crews::get))
        .route("/api/crews/{id}/members", post(crews::add_member))
        .route(
            "/api/crews/{id}/members/{user_id}",
            delete(crews::remove_member),
        )
        .route("/api/crews/{id}/availability", get(crews::availability))
        .route("/api/me/availability", put(crews::set_availability))
        .route("/api/tags", get(events::tags))
        .route("/api/{*rest}", any(api_not_found))
        .fallback_service(
            ServeDir::new(&state.static_dir)
                .fallback(ServeFile::new(state.static_dir.join("index.html"))),
        )
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<api::HealthResponse> {
    Json(api::HealthResponse {
        ok: true,
        llm: state.llm.health().await,
    })
}

/// Unknown `/api` paths get the JSON error body instead of the frontend's `index.html`.
async fn api_not_found() -> error::WebError {
    error::WebError::not_found("Endpoint")
}
