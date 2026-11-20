//! "Looking for a buddy" toggle and match.

use api::{BuddyMatch, BuddyStatus, BuddyToggleRequest, UserSummary};
use axum::Json;
use axum::extract::{Path, State};
use hangouts_core::{EventId, UserId};
use recommend::RecContext;

use super::event_id;
use crate::auth::Auth;
use crate::error::{WebError, WebResult};
use crate::state::AppState;

/// Computes the viewer's buddy status on an event.
async fn status(state: &AppState, uid: UserId, event: EventId) -> WebResult<BuddyStatus> {
    let snap = state.store.load_snapshot().await?;
    snap.event(event)
        .ok_or_else(|| WebError::not_found("Event"))?;
    let looking = snap
        .buddy_requests
        .iter()
        .any(|b| b.event_id == event && b.user_id == uid);
    let me = snap.user(uid).ok_or_else(WebError::unauthorized)?;
    if !looking || !me.privacy.buddy_matching {
        return Ok(BuddyStatus {
            looking,
            matched: None,
        });
    }
    // Other seekers who allow buddy matching, oldest request first.
    let others: Vec<UserId> = snap
        .buddy_requests
        .iter()
        .filter(|b| b.event_id == event && b.user_id != uid)
        .filter(|b| {
            snap.user(b.user_id)
                .is_some_and(|u| u.privacy.buddy_matching)
        })
        .map(|b| b.user_id)
        .collect();

    let friends = snap.friends_of(uid);
    if let Some(friend) = others.iter().find(|u| friends.contains(u)) {
        let user = snap
            .user(*friend)
            .ok_or_else(|| WebError::not_found("User"))?;
        return Ok(BuddyStatus {
            looking,
            matched: Some(BuddyMatch {
                user: UserSummary::from(user),
                is_friend: true,
                reason: format!("{} is looking too", user.display_name),
            }),
        });
    }

    let best = {
        let model = state
            .model
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let ctx = RecContext {
            snapshot: &snap,
            user: uid,
            now: state.now(),
            model: &model,
            embedder: state.embedder.as_ref(),
        };
        recommend::similar_users(&ctx, &others, 1)
            .into_iter()
            .next()
    };
    let matched = best.and_then(|m| {
        let user = snap.user(m.user_id)?;
        Some(BuddyMatch {
            user: UserSummary::from(user),
            is_friend: false,
            reason: m.reasons.first().map_or_else(
                || "Also looking for someone to go with".to_owned(),
                |r| r.text.clone(),
            ),
        })
    });
    Ok(BuddyStatus { looking, matched })
}

/// `GET /api/events/{id}/buddy`.
pub async fn get(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Path(id): Path<String>,
) -> WebResult<Json<BuddyStatus>> {
    Ok(Json(status(&state, uid, event_id(&id)?).await?))
}

/// `POST /api/events/{id}/buddy`.
pub async fn toggle(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Path(id): Path<String>,
    Json(req): Json<BuddyToggleRequest>,
) -> WebResult<Json<BuddyStatus>> {
    let event = event_id(&id)?;
    state
        .store
        .set_buddy_looking(uid, event, req.looking, state.now())
        .await?;
    Ok(Json(status(&state, uid, event).await?))
}
