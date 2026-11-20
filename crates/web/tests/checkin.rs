//! A11: rotating QR token and check-in.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use api::{CheckInResponse, QrToken};
use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

/// A host user and an event they host.
async fn hosted(app: &TestApp) -> (hangouts_core::UserId, hangouts_core::EventId) {
    let host = app.user(1, "Host").await;
    let resp = app
        .post(
            "/api/events",
            Some(host),
            json!({"title": "Game night", "date": "2026-11-21", "start": "19:00", "location": {"name": "SC"}}),
        )
        .await;
    let detail: api::EventDetail = resp.json();
    (host, detail.card.id)
}

#[tokio::test]
async fn only_the_host_gets_a_qr_token() {
    let app = TestApp::new().await;
    let (host, event) = hosted(&app).await;
    let guest = app.user(2, "Guest").await;
    let uri = format!("/api/events/{event}/qr");

    assert_eq!(app.get(&uri, None).await.status, StatusCode::UNAUTHORIZED);
    let resp = app.get(&uri, Some(guest)).await;
    assert_eq!(
        (resp.status, resp.error_code().as_str()),
        (StatusCode::FORBIDDEN, "forbidden")
    );
    let qr: QrToken = app.get(&uri, Some(host)).await.json();
    assert_eq!(qr.expires_at.to_rfc3339(), "2026-11-20T15:02:00+00:00");
    assert_eq!(
        qr.checkin_url,
        format!("/checkin?event={event}&token={}", qr.token)
    );
    let missing = format!("/api/events/{}/qr", hangouts_core::EventId::from_u128(9));
    assert_eq!(
        app.get(&missing, Some(host)).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn checkin_is_idempotent_and_records_attendance() {
    let app = TestApp::new().await;
    let (host, event) = hosted(&app).await;
    let guest = app.user(2, "Guest").await;
    let qr: QrToken = app
        .get(&format!("/api/events/{event}/qr"), Some(host))
        .await
        .json();
    let body = json!({"event_id": event, "token": qr.token});

    assert_eq!(
        app.post("/api/checkin", None, body.clone()).await.status,
        StatusCode::UNAUTHORIZED
    );
    let resp = app.post("/api/checkin", Some(guest), body.clone()).await;
    assert_eq!(resp.status, StatusCode::OK);
    let done: CheckInResponse = resp.json();
    assert!(done.checked_in);
    assert_eq!(done.event_id, event);
    assert!(app.store().has_checked_in(guest, event).await.unwrap());
    // repeat check-ins are fine
    assert_eq!(
        app.post("/api/checkin", Some(guest), body).await.status,
        StatusCode::OK
    );
    let snap = app.store().load_snapshot().await.unwrap();
    assert_eq!(snap.checkins.len(), 1);
}

#[tokio::test]
async fn forged_or_wrong_event_tokens_are_rejected() {
    let app = TestApp::new().await;
    let (host, event) = hosted(&app).await;
    let guest = app.user(2, "Guest").await;
    let other = app.event(50, "Other", 3, None).await;
    let qr: QrToken = app
        .get(&format!("/api/events/{event}/qr"), Some(host))
        .await
        .json();

    for body in [
        json!({"event_id": event, "token": "forged"}),
        json!({"event_id": event, "token": ""}),
        json!({"event_id": other, "token": qr.token}),
    ] {
        let resp = app.post("/api/checkin", Some(guest), body).await;
        assert_eq!(
            (resp.status, resp.error_code().as_str()),
            (StatusCode::BAD_REQUEST, "invalid_token")
        );
    }
    assert!(!app.store().has_checked_in(guest, event).await.unwrap());
}

#[tokio::test]
async fn tokens_expire_after_two_windows() {
    let mut app = TestApp::new().await;
    let secs = Arc::new(AtomicI64::new(0));
    let clock_secs = Arc::clone(&secs);
    app.state.clock = Arc::new(move || {
        common::now() + chrono::TimeDelta::seconds(clock_secs.load(Ordering::SeqCst))
    });
    app.router = web::app(app.state.clone());
    let (host, event) = hosted(&app).await;
    let guest = app.user(2, "Guest").await;
    let qr: QrToken = app
        .get(&format!("/api/events/{event}/qr"), Some(host))
        .await
        .json();
    let body = json!({"event_id": event, "token": qr.token});

    // 15:00:00 is a window start; +119 s is still inside the next window
    secs.store(119, Ordering::SeqCst);
    assert_eq!(
        app.post("/api/checkin", Some(guest), body.clone())
            .await
            .status,
        StatusCode::OK
    );
    secs.store(120, Ordering::SeqCst);
    let resp = app.post("/api/checkin", Some(guest), body).await;
    assert_eq!(resp.error_code(), "invalid_token");
    // the host's next token works
    let fresh: QrToken = app
        .get(&format!("/api/events/{event}/qr"), Some(host))
        .await
        .json();
    assert_ne!(fresh.token, qr.token);
}
