//! `GET /api/events/{id}/crowd`: who's going (viewer + sharing friends) with time ranges.
#![allow(clippy::unwrap_used)]

mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn requires_login() {
    let app = TestApp::new().await;
    let ev = app.event(1, "Smash night", 8, None).await;
    let resp = app.get(&format!("/api/events/{ev}/crowd"), None).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn lists_me_and_friends_only_with_their_shared_times() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let pal = app.user(2, "Pal").await;
    let stranger = app.user(3, "Stranger").await;
    app.befriend(me, pal).await;
    let ev = app.event(10, "Smash night", 8, None).await;
    for (u, level) in [(me, "going"), (pal, "interested"), (stranger, "going")] {
        let r = app
            .put(
                &format!("/api/events/{ev}/intent"),
                Some(u),
                json!({"level": level}),
            )
            .await;
        assert_eq!(r.status, StatusCode::OK);
    }

    let body = app
        .get(&format!("/api/events/{ev}/crowd"), Some(me))
        .await
        .value();
    let people = body["people"].as_array().unwrap();
    let names: Vec<&str> = people
        .iter()
        .map(|p| p["user"]["display_name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Me", "Pal"], "strangers are never listed");
    assert_eq!(people[0]["is_me"], true);
    assert!(people[0]["free_from"].is_null());

    // Setting a range for the event's date shows up in the matrix.
    let date = body["date"].as_str().unwrap().to_owned();
    let r = app
        .put(
            "/api/me/availability",
            Some(me),
            json!({"date": date, "start": "18:00", "end": "22:00"}),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    let body = app
        .get(&format!("/api/events/{ev}/crowd"), Some(me))
        .await
        .value();
    assert!(!body["people"][0]["free_from"].is_null());
    assert!(body["people"][1]["free_from"].is_null());
}

#[tokio::test]
async fn private_friends_are_hidden() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let pal = app.user(2, "Pal").await;
    app.befriend(me, pal).await;
    let ev = app.event(10, "Smash night", 8, None).await;
    app.put(
        &format!("/api/events/{ev}/intent"),
        Some(pal),
        json!({"level": "going"}),
    )
    .await;
    app.put(
        "/api/me/privacy",
        Some(pal),
        json!({"attendance": "private", "buddy_matching": true, "personalize": true}),
    )
    .await;
    let body = app
        .get(&format!("/api/events/{ev}/crowd"), Some(me))
        .await
        .value();
    assert_eq!(body["people"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn unknown_event_is_404() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let resp = app
        .get(
            "/api/events/00000000-0000-0000-0000-000000000999/crowd",
            Some(me),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}
