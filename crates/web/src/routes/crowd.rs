//! `GET /api/events/{id}/crowd`: who is going, with the times they can attend.
//!
//! Privacy (docs/API.md): only the viewer and their accepted friends who share attendance
//! with friends appear. There is no public attendee list and no total.

use api::{CrowdPerson, EventCrowd, UserSummary};
use axum::Json;
use axum::extract::{Path, State};
use hangouts_core::time::{format_date, local_date};
use hangouts_core::{IntentKind, IntentStatus, UserId, Visibility};

use super::event_id;
use crate::auth::Auth;
use crate::error::{WebError, WebResult};
use crate::state::AppState;

/// `GET /api/events/{id}/crowd`.
pub async fn get(
    State(state): State<AppState>,
    Auth(viewer): Auth,
    Path(id): Path<String>,
) -> WebResult<Json<EventCrowd>> {
    let event = state
        .store
        .get_event(event_id(&id)?)
        .await?
        .ok_or_else(|| WebError::not_found("Event"))?;
    let snap = state
        .store
        .load_event_view(std::slice::from_ref(&event), Some(viewer))
        .await?;
    let friends = snap.friends_of(viewer);

    let mut people: Vec<(UserId, UserSummary, IntentKind)> = Vec::new();
    for intent in snap.intents_for_event(event.id) {
        let is_me = intent.user_id == viewer;
        if !is_me && !friends.contains(&intent.user_id) {
            continue;
        }
        let Some(user) = snap.user(intent.user_id) else {
            continue;
        };
        if !is_me && user.privacy.attendance != Visibility::Friends {
            continue;
        }
        // A waitlisted friend holds no spot yet: show them as interested.
        let level = if intent.status == IntentStatus::Waitlisted {
            IntentKind::Interested
        } else {
            intent.level.kind()
        };
        people.push((user.id, UserSummary::from(user), level));
    }
    people.sort_by_key(|(uid, user, level)| {
        (
            *uid != viewer,
            *level == IntentKind::Interested,
            user.display_name.clone(),
        )
    });

    let date = local_date(event.starts_at);
    let ids: Vec<UserId> = people.iter().map(|(u, _, _)| *u).collect();
    let windows = state.store.list_availability_on(date, Some(&ids)).await?;

    let people = people
        .into_iter()
        .map(|(uid, user, level)| {
            let window = windows.iter().find(|a| a.user_id == uid).map(|a| a.window);
            CrowdPerson {
                user,
                level,
                is_me: uid == viewer,
                free_from: window.map(|w| w.start()),
                free_until: window.map(|w| w.end()),
            }
        })
        .collect();
    Ok(Json(EventCrowd {
        date: format_date(date),
        people,
    }))
}
