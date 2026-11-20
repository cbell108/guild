//! Demo login, session and user lookup.

use api::{LoginRequest, Me, UserSearchQuery, UserSummary};
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum_extra::extract::CookieJar;

use crate::auth::{Auth, clear_cookie, session_cookie};
use crate::error::{WebError, WebResult};
use crate::state::AppState;

fn me(user: &hangouts_core::User) -> Me {
    Me {
        user: UserSummary::from(user),
        interests: user.interests.clone(),
        privacy: user.privacy,
    }
}

/// `GET /api/demo-users`.
pub async fn demo_users(State(state): State<AppState>) -> WebResult<Json<Vec<UserSummary>>> {
    let users = state.store.list_demo_users().await?;
    Ok(Json(users.iter().map(UserSummary::from).collect()))
}

/// `POST /api/login`.
pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> WebResult<(CookieJar, Json<Me>)> {
    let user = state
        .store
        .get_user(req.user_id)
        .await?
        .ok_or_else(|| WebError::not_found("User"))?;
    Ok((jar.add(session_cookie(user.id)), Json(me(&user))))
}

/// `POST /api/logout`.
pub async fn logout(jar: CookieJar) -> (CookieJar, StatusCode) {
    (jar.add(clear_cookie()), StatusCode::NO_CONTENT)
}

/// `GET /api/me`.
pub async fn get_me(State(state): State<AppState>, Auth(uid): Auth) -> WebResult<Json<Me>> {
    let user = state
        .store
        .get_user(uid)
        .await?
        .ok_or_else(WebError::unauthorized)?;
    Ok(Json(me(&user)))
}

/// `GET /api/users?q=`.
pub async fn search_users(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Query(query): Query<UserSearchQuery>,
) -> WebResult<Json<Vec<UserSummary>>> {
    let users = state.store.search_users(&query.q, 21).await?;
    Ok(Json(
        users
            .iter()
            .filter(|u| u.id != uid)
            .take(20)
            .map(UserSummary::from)
            .collect(),
    ))
}
