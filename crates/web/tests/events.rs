//! A3: event listing, detail and hangout creation.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

mod common;

use api::{EventCard, EventDetail};
use axum::http::StatusCode;
use common::TestApp;
use hangouts_core::{EventKind, UserId};
use serde_json::json;

fn hangout_body() -> serde_json::Value {
    json!({
        "title": "Smash night",
        "description": "Bring a controller",
        "date": "2026-11-21",
        "start": "19:00",
        "end": "22:00",
        "location": {"name": "Student Center", "room": "Game Room"},
        "tags": ["Smash Bros", "#gaming"],
        "capacity": 8
    })
}

#[tokio::test]
async fn health_is_ok() {
    let app = TestApp::new().await;
    let resp = app.get("/api/health", None).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.value()["ok"], true);
}

#[tokio::test]
async fn list_filters_and_orders_events() {
    let app = TestApp::new().await;
    app.event(2, "Later", 30, None).await;
    app.event(1, "Sooner", 3, None).await;
    // already over: excluded by the default `from = now`
    app.event(3, "Past", -10, None).await;

    let cards: Vec<EventCard> = app.get("/api/events", None).await.json();
    let titles: Vec<_> = cards.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["Sooner", "Later"]);

    let cards: Vec<EventCard> = app.get("/api/events?q=later", None).await.json();
    assert_eq!(cards.len(), 1);
    let cards: Vec<EventCard> = app
        .get("/api/events?tags=gaming,nope&limit=1", None)
        .await
        .json();
    assert_eq!(cards.len(), 1);
    let cards: Vec<EventCard> = app.get("/api/events?kind=official", None).await.json();
    assert!(cards.is_empty());
    let cards: Vec<EventCard> = app
        .get(
            "/api/events?from=2026-11-01T00:00:00Z&to=2026-11-20T16:00:00Z",
            None,
        )
        .await
        .json();
    assert_eq!(
        cards.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(),
        ["Past"]
    );
    assert_eq!(
        app.get("/api/events?kind=bogus", None).await.status,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn detail_and_not_found() {
    let app = TestApp::new().await;
    let id = app.event(1, "Game night", 3, Some(4)).await;
    let resp = app.get(&format!("/api/events/{id}"), None).await;
    assert_eq!(resp.status, StatusCode::OK);
    let detail: EventDetail = resp.json();
    assert_eq!(detail.card.title, "Game night");
    assert_eq!(detail.card.spots_left, Some(4));
    assert_eq!(detail.description, "Game night description");
    assert_eq!(
        detail.calendar_url,
        format!("/api/events/{id}/calendar.ics")
    );
    assert!(detail.card.my_intent.is_none() && !detail.buddy_looking);

    let resp = app
        .get(&format!("/api/events/{}", UserId::from_u128(99)), None)
        .await;
    assert_eq!(
        (resp.status, resp.error_code().as_str()),
        (StatusCode::NOT_FOUND, "not_found")
    );
    let resp = app.get("/api/events/not-a-uuid", None).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_hangout_requires_login_and_validates() {
    let app = TestApp::new().await;
    let host = app.user(1, "Hosty").await;

    let resp = app.post("/api/events", None, hangout_body()).await;
    assert_eq!(
        (resp.status, resp.error_code().as_str()),
        (StatusCode::UNAUTHORIZED, "unauthorized")
    );

    let resp = app.post("/api/events", Some(host), hangout_body()).await;
    assert_eq!(resp.status, StatusCode::CREATED);
    let detail: EventDetail = resp.json();
    assert_eq!(detail.card.kind, EventKind::Hangout);
    assert_eq!(detail.card.host.as_ref().unwrap().id, host);
    assert_eq!(detail.card.tags, ["smash-bros", "gaming"]);
    assert_eq!(detail.card.capacity, Some(8));
    // 19:00 EST = 00:00 UTC the next day
    assert_eq!(
        detail.card.starts_at.to_rfc3339(),
        "2026-11-22T00:00:00+00:00"
    );
    let again: EventDetail = app
        .get(&format!("/api/events/{}", detail.card.id), None)
        .await
        .json();
    assert_eq!(again.card.title, "Smash night");

    // created events are embedded for the recommender
    assert!(
        app.store()
            .events_missing_embedding()
            .await
            .unwrap()
            .is_empty()
    );

    let mut bad = hangout_body();
    bad["start"] = json!("7pm");
    let resp = app.post("/api/events", Some(host), bad).await;
    assert_eq!(
        (resp.status, resp.error_code().as_str()),
        (StatusCode::BAD_REQUEST, "bad_request")
    );
    assert!(
        resp.value()["error"]["message"]
            .as_str()
            .unwrap()
            .contains("start")
    );

    let mut bad = hangout_body();
    bad["title"] = json!("  ");
    assert_eq!(
        app.post("/api/events", Some(host), bad).await.status,
        StatusCode::BAD_REQUEST
    );
    let mut bad = hangout_body();
    bad["date"] = json!("2026-11-19");
    assert_eq!(
        app.post("/api/events", Some(host), bad).await.status,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn create_for_crew_requires_membership() {
    let app = TestApp::new().await;
    let owner = app.user(1, "Owner").await;
    let outsider = app.user(2, "Outsider").await;
    let crew_id = hangouts_core::CrewId::from_u128(5);
    app.store()
        .insert_crew(&hangouts_core::Crew {
            id: crew_id,
            name: "Smash".into(),
            description: String::new(),
            created_by: owner,
            created_at: common::now(),
        })
        .await
        .unwrap();
    let mut body = hangout_body();
    body["crew_id"] = json!(crew_id);
    assert_eq!(
        app.post("/api/events", Some(outsider), body.clone())
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let resp = app.post("/api/events", Some(owner), body).await;
    assert_eq!(resp.status, StatusCode::CREATED);
    let detail: EventDetail = resp.json();
    assert_eq!(detail.crew.unwrap().name, "Smash");
}

#[tokio::test]
async fn tags_endpoint() {
    let app = TestApp::new().await;
    app.event(1, "A", 3, None).await;
    app.event(2, "B", 4, None).await;
    let tags: Vec<api::TagCount> = app.get("/api/tags", None).await.json();
    assert_eq!(
        tags,
        [api::TagCount {
            tag: "gaming".into(),
            count: 2
        }]
    );
}
