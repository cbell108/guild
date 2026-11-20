//! Contract edge cases from docs/API.md: login coverage of every 🔒 endpoint,
//! malformed ids, bad request bodies, and intent/calendar behaviour.
#![allow(clippy::unwrap_used)]

mod common;

use api::MyIntent;
use axum::http::{StatusCode, header};
use common::TestApp;
use hangouts_core::{Crew, CrewId, EventId, IntentStatus};
use serde_json::{Value, json};

/// One 🔒 row from the Endpoints table of docs/API.md, ready to be called
/// logged out: the method, the fully expanded path, and a minimal body for
/// requests that take one.
struct Protected {
    method: &'static str,
    uri: String,
    body: Option<Value>,
}

async fn call(app: &TestApp, row: &Protected) -> common::Resp {
    let body = row.body.clone().unwrap_or_else(|| json!({}));
    match row.method {
        "GET" => app.get(&row.uri, None).await,
        "POST" => app.post(&row.uri, None, body).await,
        "PUT" => app.put(&row.uri, None, body).await,
        "DELETE" => app.delete(&row.uri, None).await,
        _ => panic!("unhandled method {}", row.method),
    }
}

#[tokio::test]
async fn protected_endpoints_require_login() {
    let app = TestApp::new().await;
    let user = app.user(1, "Amy").await;
    let event = app.event(1, "E", 3, None).await;
    let crew = CrewId::from_u128(5);
    app.store()
        .insert_crew(&Crew {
            id: crew,
            name: "Smash".into(),
            description: String::new(),
            created_by: user,
            created_at: common::now(),
        })
        .await
        .unwrap();

    // Every 🔒 row of the Endpoints table in docs/API.md.
    let rows = [
        Protected {
            method: "GET",
            uri: "/api/me".into(),
            body: None,
        },
        Protected {
            method: "GET",
            uri: "/api/users?q=".into(),
            body: None,
        },
        Protected {
            method: "POST",
            uri: "/api/events".into(),
            body: None,
        },
        Protected {
            method: "PUT",
            uri: format!("/api/events/{event}/intent"),
            body: None,
        },
        Protected {
            method: "DELETE",
            uri: format!("/api/events/{event}/intent"),
            body: None,
        },
        Protected {
            method: "GET",
            uri: "/api/feed".into(),
            body: None,
        },
        Protected {
            method: "GET",
            uri: format!("/api/events/{event}/buddy"),
            body: None,
        },
        Protected {
            method: "POST",
            uri: format!("/api/events/{event}/buddy"),
            body: None,
        },
        Protected {
            method: "GET",
            uri: "/api/friends".into(),
            body: None,
        },
        Protected {
            method: "POST",
            uri: "/api/friends".into(),
            body: None,
        },
        Protected {
            method: "POST",
            uri: format!("/api/friends/{user}/accept"),
            body: None,
        },
        Protected {
            method: "DELETE",
            uri: format!("/api/friends/{user}"),
            body: None,
        },
        Protected {
            method: "GET",
            uri: "/api/crews".into(),
            body: None,
        },
        Protected {
            method: "POST",
            uri: "/api/crews".into(),
            body: None,
        },
        Protected {
            method: "GET",
            uri: format!("/api/crews/{crew}"),
            body: None,
        },
        Protected {
            method: "POST",
            uri: format!("/api/crews/{crew}/members"),
            body: None,
        },
        Protected {
            method: "DELETE",
            uri: format!("/api/crews/{crew}/members/{user}"),
            body: None,
        },
        Protected {
            method: "GET",
            uri: format!("/api/crews/{crew}/availability?date=2026-11-20"),
            body: None,
        },
        Protected {
            method: "PUT",
            uri: "/api/me/availability".into(),
            body: None,
        },
        Protected {
            method: "POST",
            uri: "/api/parse".into(),
            body: None,
        },
        Protected {
            method: "GET",
            uri: format!("/api/events/{event}/qr"),
            body: None,
        },
        Protected {
            method: "POST",
            uri: "/api/checkin".into(),
            body: None,
        },
        Protected {
            method: "GET",
            uri: "/api/me/privacy".into(),
            body: None,
        },
        Protected {
            method: "PUT",
            uri: "/api/me/privacy".into(),
            body: None,
        },
        Protected {
            method: "DELETE",
            uri: "/api/me/history".into(),
            body: None,
        },
    ];
    assert_eq!(rows.len(), 25);
    for row in &rows {
        let resp = call(&app, row).await;
        assert_eq!(
            (resp.status, resp.error_code().as_str()),
            (StatusCode::UNAUTHORIZED, "unauthorized"),
            "{} {}",
            row.method,
            row.uri
        );
    }
}

#[tokio::test]
async fn malformed_ids_are_not_found() {
    let app = TestApp::new().await;
    let resp = app.get("/api/events/not-a-uuid", None).await;
    assert_eq!(
        (resp.status, resp.error_code().as_str()),
        (StatusCode::NOT_FOUND, "not_found")
    );
    let missing = EventId::from_u128(0x404);
    let resp = app.get(&format!("/api/events/{missing}"), None).await;
    assert_eq!(
        (resp.status, resp.error_code().as_str()),
        (StatusCode::NOT_FOUND, "not_found")
    );
}

#[tokio::test]
async fn bad_json_is_bad_request() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let event = app.event(1, "E", 3, None).await;
    let resp = app
        .put(
            &format!("/api/events/{event}/intent"),
            Some(me),
            json!({"level": "maybe"}),
        )
        .await;
    assert!(resp.status.is_client_error());
}

#[tokio::test]
async fn going_with_requires_companion() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let event = app.event(1, "E", 3, None).await;
    let resp = app
        .put(
            &format!("/api/events/{event}/intent"),
            Some(me),
            json!({"level": "going_with"}),
        )
        .await;
    assert_eq!(
        (resp.status, resp.error_code().as_str()),
        (StatusCode::BAD_REQUEST, "bad_request")
    );
}

#[tokio::test]
async fn delete_intent_is_idempotent() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let event = app.event(1, "E", 3, None).await;
    let uri = format!("/api/events/{event}/intent");
    assert_eq!(
        app.delete(&uri, Some(me)).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.delete(&uri, Some(me)).await.status,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn interested_never_takes_a_spot() {
    let app = TestApp::new().await;
    let a = app.user(1, "A").await;
    let b = app.user(2, "B").await;
    let event = app.event(1, "Small", 3, Some(1)).await;
    let uri = format!("/api/events/{event}/intent");
    let interested: MyIntent = app
        .put(&uri, Some(a), json!({"level": "interested"}))
        .await
        .json();
    assert_eq!(interested.status, IntentStatus::Confirmed);
    let going: MyIntent = app
        .put(&uri, Some(b), json!({"level": "going"}))
        .await
        .json();
    assert_eq!(going.status, IntentStatus::Confirmed);
}

#[tokio::test]
async fn calendar_ics_headers() {
    let app = TestApp::new().await;
    let event = app.event(1, "Game night", 3, None).await;
    let resp = app
        .get(&format!("/api/events/{event}/calendar.ics"), None)
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let content_type = resp
        .headers
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(content_type.starts_with("text/calendar"), "{content_type}");
    let body = resp.text();
    assert!(body.contains("BEGIN:VEVENT"), "{body}");
    assert!(body.contains("UID:"), "{body}");
}
