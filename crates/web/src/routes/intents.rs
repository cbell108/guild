//! Intent to attend (Interested / Going / Going with) and the waitlist.

use api::{MyIntent, SetIntentRequest, UserSummary};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use hangouts_core::{IntentLevel, IntentStatus};

use super::event_id;
use crate::auth::Auth;
use crate::error::{WebError, WebResult};
use crate::state::AppState;

/// `PUT /api/events/{id}/intent`.
pub async fn set(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Path(id): Path<String>,
    Json(req): Json<SetIntentRequest>,
) -> WebResult<Json<MyIntent>> {
    let event = event_id(&id)?;
    let level = req.to_level()?;
    let mut with = None;
    if let IntentLevel::GoingWith(friend) = level {
        if !state.store.friend_ids(uid).await?.contains(&friend) {
            return Err(WebError::bad_request(
                "You can only go with someone who is your friend.",
            ));
        }
        with = state
            .store
            .get_user(friend)
            .await?
            .map(|u| UserSummary::from(&u));
    }
    let intent = state
        .store
        .set_intent(uid, event, level, state.now())
        .await?;
    let waitlist_position = if intent.status == IntentStatus::Waitlisted {
        state.store.waitlist_position(uid, event).await?
    } else {
        None
    };
    Ok(Json(MyIntent {
        level: intent.level.kind(),
        with,
        status: intent.status,
        waitlist_position,
    }))
}

/// `DELETE /api/events/{id}/intent`: the one-tap "can't make it". Idempotent.
pub async fn remove(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Path(id): Path<String>,
) -> WebResult<StatusCode> {
    state
        .store
        .remove_intent(uid, event_id(&id)?, state.now())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
