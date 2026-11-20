//! Friends and friend requests.

use api::{FriendRequest, FriendsResponse, UserSummary};
use axum::Json;
use axum::extract::{Path, State};
use hangouts_core::{FriendshipStatus, UserId};

use super::parse_id;
use crate::auth::Auth;
use crate::error::{WebError, WebResult};
use crate::state::AppState;

/// Builds the viewer's friends / incoming / outgoing lists, each sorted by name.
async fn overview(state: &AppState, uid: UserId) -> WebResult<FriendsResponse> {
    let friendships = state.store.list_friendships(uid).await?;
    let mut out = FriendsResponse {
        friends: Vec::new(),
        incoming: Vec::new(),
        outgoing: Vec::new(),
    };
    for f in friendships {
        let Some(other) = f.other(uid) else { continue };
        let Some(user) = state.store.get_user(other).await? else {
            continue;
        };
        let summary = UserSummary::from(&user);
        match (f.status, f.requester == uid) {
            (FriendshipStatus::Accepted, _) => out.friends.push(summary),
            (FriendshipStatus::Pending, true) => out.outgoing.push(summary),
            (FriendshipStatus::Pending, false) => out.incoming.push(summary),
        }
    }
    for list in [&mut out.friends, &mut out.incoming, &mut out.outgoing] {
        list.sort_by(|a, b| a.display_name.cmp(&b.display_name).then(a.id.cmp(&b.id)));
    }
    Ok(out)
}

/// `GET /api/friends`.
pub async fn list(
    State(state): State<AppState>,
    Auth(uid): Auth,
) -> WebResult<Json<FriendsResponse>> {
    Ok(Json(overview(&state, uid).await?))
}

/// `POST /api/friends`: send a request, or accept one the other user already sent.
pub async fn request(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Json(req): Json<FriendRequest>,
) -> WebResult<Json<FriendsResponse>> {
    if req.user_id == uid {
        return Err(WebError::bad_request("You can't befriend yourself."));
    }
    state
        .store
        .request_friend(uid, req.user_id, state.now())
        .await?;
    Ok(Json(overview(&state, uid).await?))
}

/// `POST /api/friends/{user_id}/accept`.
pub async fn accept(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Path(other): Path<String>,
) -> WebResult<Json<FriendsResponse>> {
    let requester: UserId = parse_id(&other, "Friend request")?;
    state
        .store
        .accept_friend(uid, requester, state.now())
        .await
        .map_err(|e| match e {
            store::StoreError::NotFound => WebError::not_found("Friend request"),
            other => other.into(),
        })?;
    Ok(Json(overview(&state, uid).await?))
}

/// `DELETE /api/friends/{user_id}`: unfriend, cancel or decline.
pub async fn remove(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Path(other): Path<String>,
) -> WebResult<Json<FriendsResponse>> {
    let other: UserId = parse_id(&other, "User")?;
    state.store.remove_friend(uid, other).await?;
    Ok(Json(overview(&state, uid).await?))
}
