//! A8: static files, SPA fallback and the `.ics` export.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

mod common;

use axum::http::{StatusCode, header};
use common::TestApp;
use hangouts_core::{Event, EventId, EventKind, Location};

#[tokio::test]
async fn calendar_export() {
    let app = TestApp::new().await;
    let id = EventId::from_u128(7);
    app.store()
        .insert_event(&Event {
            id,
            kind: EventKind::Hangout,
            title: "Smash, night; 1".into(),
            description: "Line one\nLine two".into(),
            tags: vec![],
            location: Location {
                name: "Student Center".into(),
                room: Some("Game Room".into()),
                ..Location::default()
            },
            starts_at: "2026-11-22T00:00:00Z".parse().unwrap(),
            ends_at: "2026-11-22T03:00:00Z".parse().unwrap(),
            capacity: None,
            host_id: None,
            crew_id: None,
            external_id: None,
            url: None,
            image_url: None,
            organizer: None,
            created_at: common::now(),
        })
        .await
        .unwrap();
    let resp = app
        .get(&format!("/api/events/{id}/calendar.ics"), None)
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/calendar")
    );
    assert_eq!(
        resp.headers[header::CONTENT_DISPOSITION].to_str().unwrap(),
        "attachment; filename=\"smash-night-1.ics\""
    );
    let text = resp.text();
    assert!(text.starts_with("BEGIN:VCALENDAR\r\n") && text.ends_with("END:VCALENDAR\r\n"));
    assert_eq!(text.matches("BEGIN:VEVENT").count(), 1);
    for line in [
        format!("UID:{id}@campus-hangouts"),
        "DTSTART:20261122T000000Z".to_owned(),
        "DTEND:20261122T030000Z".to_owned(),
        "DTSTAMP:20261120T150000Z".to_owned(),
        "SUMMARY:Smash\\, night\\; 1".to_owned(),
        "LOCATION:Student Center\\, Game Room".to_owned(),
        "DESCRIPTION:Line one\\nLine two".to_owned(),
    ] {
        assert!(
            text.contains(&format!("{line}\r\n")),
            "missing {line:?} in {text}"
        );
    }

    let missing = app
        .get(
            &format!("/api/events/{}/calendar.ics", EventId::from_u128(8)),
            None,
        )
        .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert_eq!(
        app.get("/api/events/nope/calendar.ics", None).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn serves_static_files_with_index_fallback() {
    let dir = std::env::temp_dir().join(format!("campus-hangouts-static-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<h1>home</h1>").unwrap();
    std::fs::write(dir.join("app.js"), "console.log(1)").unwrap();

    let mut app = TestApp::new().await;
    app.state.static_dir = dir.clone();
    app.router = web::app(app.state.clone());

    let resp = app.get("/", None).await;
    assert_eq!(
        (resp.status, resp.text().as_str()),
        (StatusCode::OK, "<h1>home</h1>")
    );
    assert_eq!(app.get("/app.js", None).await.text(), "console.log(1)");
    // unknown non-API paths fall back to index.html (client-side routing)
    assert_eq!(app.get("/event/123", None).await.text(), "<h1>home</h1>");
    // unknown API paths are JSON 404s, never the frontend
    let resp = app.get("/api/nope", None).await;
    assert_eq!(
        (resp.status, resp.error_code().as_str()),
        (StatusCode::NOT_FOUND, "not_found")
    );
    // real API routes still win
    assert_eq!(app.get("/api/health", None).await.status, StatusCode::OK);
    std::fs::remove_dir_all(dir).ok();
}
