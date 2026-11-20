//! QR check-in: the host's screen shows a rotating HMAC token; attendees scan it.
//!
//! Token = URL-safe base64 (no padding) of HMAC-SHA256(`QR_SECRET`, `"<event id>:<unix time / 60>"`),
//! accepted for the current and the previous 60-second window.

use api::{CheckInRequest, CheckInResponse, QrToken};
use axum::Json;
use axum::extract::{Path, State};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use hangouts_core::{CheckIn, EventId};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use super::event_id;
use crate::auth::Auth;
use crate::error::{WebError, WebResult};
use crate::state::AppState;

/// Length of one token window in seconds.
const WINDOW_SECS: i64 = 60;

fn mac_for(secret: &[u8], event: EventId, window: i64) -> Option<Hmac<Sha256>> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).ok()?;
    mac.update(format!("{event}:{window}").as_bytes());
    Some(mac)
}

/// The token for `event` in the window containing `at`.
pub fn token_at(secret: &[u8], event: EventId, at: DateTime<Utc>) -> Option<String> {
    let window = at.timestamp().div_euclid(WINDOW_SECS);
    let tag = mac_for(secret, event, window)?.finalize().into_bytes();
    Some(URL_SAFE_NO_PAD.encode(tag))
}

/// Whether `token` is valid for `event` at `at` (current or previous window).
pub fn verify(secret: &[u8], event: EventId, token: &str, at: DateTime<Utc>) -> bool {
    let Ok(tag) = URL_SAFE_NO_PAD.decode(token) else {
        return false;
    };
    let window = at.timestamp().div_euclid(WINDOW_SECS);
    [window, window - 1]
        .into_iter()
        .any(|w| mac_for(secret, event, w).is_some_and(|mac| mac.verify_slice(&tag).is_ok()))
}

/// `GET /api/events/{id}/qr`: host only.
pub async fn qr(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Path(id): Path<String>,
) -> WebResult<Json<QrToken>> {
    let id = event_id(&id)?;
    let event = state
        .store
        .get_event(id)
        .await?
        .ok_or_else(|| WebError::not_found("Event"))?;
    if event.host_id != Some(uid) {
        return Err(WebError::forbidden(
            "Only the host can show the check-in code.",
        ));
    }
    let now = state.now();
    let token = token_at(&state.qr_secret, id, now).ok_or_else(|| {
        WebError::new(
            api::ErrorCode::Internal,
            "Could not create a check-in code.",
        )
    })?;
    // Valid for the rest of this window and all of the next one.
    let window = now.timestamp().div_euclid(WINDOW_SECS);
    let expires_at = DateTime::from_timestamp((window + 2) * WINDOW_SECS, 0).unwrap_or(now);
    Ok(Json(QrToken {
        checkin_url: format!("/checkin?event={id}&token={token}"),
        token,
        expires_at,
    }))
}

/// `POST /api/checkin`: idempotent.
pub async fn checkin(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Json(req): Json<CheckInRequest>,
) -> WebResult<Json<CheckInResponse>> {
    let now = state.now();
    if !verify(&state.qr_secret, req.event_id, &req.token, now) {
        return Err(WebError::new(
            api::ErrorCode::InvalidToken,
            "That check-in code has expired. Ask the host to show the current one.",
        ));
    }
    if state.store.get_event(req.event_id).await?.is_none() {
        return Err(WebError::not_found("Event"));
    }
    let record = CheckIn {
        user_id: uid,
        event_id: req.event_id,
        at: now,
    };
    state.store.record_checkin(&record).await?;
    Ok(Json(CheckInResponse {
        event_id: req.event_id,
        checked_in: true,
        at: now,
    }))
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use super::*;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap_or_default()
    }

    #[test]
    fn token_window_rules() {
        let secret = b"secret";
        let event = EventId::from_u128(1);
        let t0 = at(6000);
        let token = token_at(secret, event, t0).unwrap();
        assert!(verify(secret, event, &token, t0));
        assert!(verify(secret, event, &token, t0 + TimeDelta::seconds(59)));
        // still valid in the next window, expired in the one after
        assert!(verify(secret, event, &token, t0 + TimeDelta::seconds(60)));
        assert!(verify(secret, event, &token, t0 + TimeDelta::seconds(119)));
        assert!(!verify(secret, event, &token, t0 + TimeDelta::seconds(120)));
        assert!(!verify(b"other", event, &token, t0));
        assert!(!verify(secret, EventId::from_u128(2), &token, t0));
        assert!(!verify(secret, event, "not base64!!", t0));
        assert!(!verify(secret, event, "", t0));
    }
}
