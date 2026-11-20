//! Interest onboarding: the viewer's interest tags.

use api::{Me, SetInterestsRequest, UserSummary};
use axum::Json;
use axum::extract::State;
use hangouts_core::normalize_tags;

use crate::auth::Auth;
use crate::error::{WebError, WebResult};
use crate::state::AppState;

/// Most interests a user may keep.
const MAX_INTERESTS: usize = 40;

/// `PUT /api/me/interests`: replaces the viewer's interests (normalized, de-duplicated) and
/// answers with the updated [`Me`], so the client can refresh the feed right away.
pub async fn put(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Json(req): Json<SetInterestsRequest>,
) -> WebResult<Json<Me>> {
    let interests = normalize_tags(&req.interests);
    if interests.len() > MAX_INTERESTS {
        return Err(WebError::bad_request(format!(
            "Pick at most {MAX_INTERESTS} interests."
        )));
    }
    state.store.set_interests(uid, &interests).await?;
    let user = state
        .store
        .get_user(uid)
        .await?
        .ok_or_else(WebError::unauthorized)?;
    Ok(Json(Me {
        user: UserSummary::from(&user),
        interests: user.interests.clone(),
        privacy: user.privacy,
    }))
}
