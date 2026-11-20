//! Cookie-based demo session: the `uid` cookie holds the logged-in user's id.

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use hangouts_core::UserId;

use crate::error::WebError;
use crate::state::AppState;

/// Name of the session cookie.
pub const COOKIE: &str = "uid";

/// A logged-in viewer; rejects with 401 otherwise.
#[derive(Debug, Clone, Copy)]
pub struct Auth(pub UserId);

/// A viewer who may be logged out.
#[derive(Debug, Clone, Copy)]
pub struct MaybeAuth(pub Option<UserId>);

/// Reads the `uid` cookie and checks the user still exists.
async fn viewer(parts: &mut Parts, state: &AppState) -> Result<Option<UserId>, WebError> {
    let jar = CookieJar::from_headers(&parts.headers);
    let Some(id) = jar
        .get(COOKIE)
        .and_then(|c| c.value().parse::<UserId>().ok())
    else {
        return Ok(None);
    };
    Ok(state.store.get_user(id).await?.map(|u| u.id))
}

impl<S> FromRequestParts<S> for Auth
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);
        viewer(parts, &state)
            .await?
            .map(Self)
            .ok_or_else(WebError::unauthorized)
    }
}

impl<S> FromRequestParts<S> for MaybeAuth
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);
        Ok(Self(viewer(parts, &state).await?))
    }
}

/// The session cookie for `user`.
pub fn session_cookie(user: UserId) -> Cookie<'static> {
    Cookie::build((COOKIE, user.to_string()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .build()
}

/// An expired cookie (same path) that makes the browser drop the session.
pub fn clear_cookie() -> Cookie<'static> {
    let mut cookie = Cookie::build(COOKIE).path("/").build();
    cookie.make_removal();
    cookie
}
