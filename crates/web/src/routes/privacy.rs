//! Privacy settings and history deletion.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use hangouts_core::PrivacySettings;

use crate::auth::Auth;
use crate::error::{WebError, WebResult};
use crate::state::AppState;

/// `GET /api/me/privacy`.
pub async fn get(
    State(state): State<AppState>,
    Auth(uid): Auth,
) -> WebResult<Json<PrivacySettings>> {
    let user = state
        .store
        .get_user(uid)
        .await?
        .ok_or_else(WebError::unauthorized)?;
    Ok(Json(user.privacy))
}

/// `PUT /api/me/privacy`: replaces the settings and echoes them back.
pub async fn put(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Json(privacy): Json<PrivacySettings>,
) -> WebResult<Json<PrivacySettings>> {
    state.store.set_privacy(uid, &privacy).await?;
    Ok(Json(privacy))
}

/// `DELETE /api/me/history`: removes intents, check-ins, buddy requests and availability.
pub async fn delete_history(
    State(state): State<AppState>,
    Auth(uid): Auth,
) -> WebResult<StatusCode> {
    state.store.delete_history(uid).await?;
    Ok(StatusCode::NO_CONTENT)
}
