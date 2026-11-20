//! A9: friends, crews and availability.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

mod common;

use api::{AvailabilityView, CrewAvailability, CrewDetail, CrewSummary, FriendsResponse};
use axum::http::StatusCode;
use common::TestApp;
use hangouts_core::CrewRole;
use serde_json::json;

fn names(list: &[api::UserSummary]) -> Vec<&str> {
    list.iter().map(|u| u.display_name.as_str()).collect()
}

#[tokio::test]
async fn friend_request_lifecycle() {
    let app = TestApp::new().await;
    let a = app.user(1, "Alice").await;
    let b = app.user(2, "Bob").await;
    let c = app.user(3, "Cara").await;

    assert_eq!(
        app.get("/api/friends", None).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.post("/api/friends", Some(a), json!({"user_id": a}))
            .await
            .status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        app.post(
            "/api/friends",
            Some(a),
            json!({"user_id": common_missing()})
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );

    let r: FriendsResponse = app
        .post("/api/friends", Some(a), json!({"user_id": b}))
        .await
        .json();
    assert_eq!(names(&r.outgoing), ["Bob"]);
    assert!(r.friends.is_empty() && r.incoming.is_empty());
    let r: FriendsResponse = app.get("/api/friends", Some(b)).await.json();
    assert_eq!(names(&r.incoming), ["Alice"]);

    // accept needs a pending request from that user
    assert_eq!(
        app.post(&format!("/api/friends/{a}/accept"), Some(c), json!({}))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.post(&format!("/api/friends/{b}/accept"), Some(a), json!({}))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let r: FriendsResponse = app
        .post(&format!("/api/friends/{a}/accept"), Some(b), json!({}))
        .await
        .json();
    assert_eq!(names(&r.friends), ["Alice"]);
    assert!(r.incoming.is_empty());

    // a reciprocal request accepts automatically
    app.post("/api/friends", Some(c), json!({"user_id": a}))
        .await;
    let r: FriendsResponse = app
        .post("/api/friends", Some(a), json!({"user_id": c}))
        .await
        .json();
    assert_eq!(names(&r.friends), ["Bob", "Cara"]);

    let r: FriendsResponse = app
        .delete(&format!("/api/friends/{b}"), Some(a))
        .await
        .json();
    assert_eq!(names(&r.friends), ["Cara"]);
    // removing is idempotent
    assert_eq!(
        app.delete(&format!("/api/friends/{b}"), Some(a))
            .await
            .status,
        StatusCode::OK
    );
}

fn common_missing() -> hangouts_core::UserId {
    hangouts_core::UserId::from_u128(404)
}

#[tokio::test]
async fn crew_permissions() {
    let app = TestApp::new().await;
    let owner = app.user(1, "Owner").await;
    let member = app.user(2, "Member").await;
    let outsider = app.user(3, "Outsider").await;

    assert_eq!(
        app.post("/api/crews", None, json!({"name": "X"}))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.post("/api/crews", Some(owner), json!({"name": "  "}))
            .await
            .status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        app.post("/api/crews", Some(owner), json!({"name": "x".repeat(61)}))
            .await
            .status,
        StatusCode::BAD_REQUEST
    );

    let resp = app
        .post(
            "/api/crews",
            Some(owner),
            json!({"name": " Smash Crew ", "description": "Fridays"}),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED);
    let crew: CrewDetail = resp.json();
    assert_eq!(crew.crew.name, "Smash Crew");
    assert_eq!(
        (crew.members.len(), crew.members[0].role),
        (1, CrewRole::Owner)
    );
    let id = crew.crew.id;

    // members only
    assert_eq!(
        app.get(&format!("/api/crews/{id}"), Some(outsider))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.get(
            &format!("/api/crews/{}", hangouts_core::CrewId::from_u128(9)),
            Some(owner)
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert!(
        app.get("/api/crews", Some(outsider))
            .await
            .json::<Vec<CrewSummary>>()
            .is_empty()
    );

    // only owners add
    let add = |who| json!({"user_id": who});
    assert_eq!(
        app.post(
            &format!("/api/crews/{id}/members"),
            Some(outsider),
            add(member)
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.post(
            &format!("/api/crews/{id}/members"),
            Some(owner),
            add(common_missing())
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    let crew: CrewDetail = app
        .post(
            &format!("/api/crews/{id}/members"),
            Some(owner),
            add(member),
        )
        .await
        .json();
    assert_eq!(crew.members.len(), 2);
    // idempotent, never demotes the owner
    let crew: CrewDetail = app
        .post(&format!("/api/crews/{id}/members"), Some(owner), add(owner))
        .await
        .json();
    assert_eq!(crew.members.len(), 2);
    assert_eq!(crew.members[0].role, CrewRole::Owner);
    assert_eq!(
        app.post(
            &format!("/api/crews/{id}/members"),
            Some(member),
            add(outsider)
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    let mine: Vec<CrewSummary> = app.get("/api/crews", Some(member)).await.json();
    assert_eq!(mine.len(), 1);

    // members can't remove others, but can leave
    assert_eq!(
        app.delete(&format!("/api/crews/{id}/members/{owner}"), Some(member))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.delete(&format!("/api/crews/{id}/members/{member}"), Some(member))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        app.get(&format!("/api/crews/{id}"), Some(member))
            .await
            .status,
        StatusCode::FORBIDDEN
    );

    // owner removes others; the last owner leaving hands over ownership
    app.post(
        &format!("/api/crews/{id}/members"),
        Some(owner),
        add(member),
    )
    .await;
    app.post(
        &format!("/api/crews/{id}/members"),
        Some(owner),
        add(outsider),
    )
    .await;
    let crew: CrewDetail = app
        .delete(&format!("/api/crews/{id}/members/{outsider}"), Some(owner))
        .await
        .json();
    assert_eq!(crew.members.len(), 2);
    let crew: CrewDetail = app
        .delete(&format!("/api/crews/{id}/members/{owner}"), Some(owner))
        .await
        .json();
    assert_eq!(crew.members.len(), 1);
    assert_eq!(crew.members[0].user.id, member);
    assert_eq!(crew.members[0].role, CrewRole::Owner);
}

#[tokio::test]
async fn availability_grid_and_best_slot() {
    let app = TestApp::new().await;
    let a = app.user(1, "Alice").await;
    let b = app.user(2, "Bob").await;
    let outsider = app.user(3, "Out").await;
    let crew: CrewDetail = app
        .post("/api/crews", Some(a), json!({"name": "C"}))
        .await
        .json();
    let id = crew.crew.id;
    app.post(
        &format!("/api/crews/{id}/members"),
        Some(a),
        json!({"user_id": b}),
    )
    .await;

    let put = |who, start: &str, end: &str| {
        app.put(
            "/api/me/availability",
            Some(who),
            json!({"date": "2026-11-21", "start": start, "end": end}),
        )
    };
    let v: AvailabilityView = put(a, "10:00", "13:00").await.json();
    assert_eq!(v.date, "2026-11-21");
    assert!(!v.overnight);
    let v: AvailabilityView = put(b, "12:00", "15:00").await.json();
    assert_eq!(v.starts_at.to_rfc3339(), "2026-11-21T17:00:00+00:00");
    // an outsider's availability never shows up in the crew grid
    put(outsider, "00:00", "23:00").await;

    let grid: CrewAvailability = app
        .get(
            &format!("/api/crews/{id}/availability?date=2026-11-21"),
            Some(b),
        )
        .await
        .json();
    assert_eq!((grid.date.as_str(), grid.slot_minutes), ("2026-11-21", 60));
    assert_eq!(grid.members.len(), 2);
    assert_eq!(grid.slots.len(), 24);
    let best = grid.best.unwrap();
    assert_eq!(best.available.len(), 2);
    assert_eq!(best.starts_at.to_rfc3339(), "2026-11-21T17:00:00+00:00");
    assert!(grid.slots.iter().all(|s| !s.available.contains(&outsider)));
    let both = grid.slots.iter().filter(|s| s.available.len() == 2).count();
    assert_eq!(both, 1, "12:00-13:00 is the only hour both are free");

    let grid: CrewAvailability = app
        .get(
            &format!("/api/crews/{id}/availability?date=2026-11-21&slot_minutes=5"),
            Some(a),
        )
        .await
        .json();
    assert_eq!(grid.slot_minutes, 15);
    assert_eq!(
        app.get(
            &format!("/api/crews/{id}/availability?date=2026-11-21"),
            Some(outsider)
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.get(
            &format!("/api/crews/{id}/availability?date=2026-02-30"),
            Some(a)
        )
        .await
        .status,
        StatusCode::BAD_REQUEST
    );
    assert!(
        app.get(
            &format!("/api/crews/{id}/availability?date=2026-11-25"),
            Some(a)
        )
        .await
        .json::<CrewAvailability>()
        .best
        .is_none()
    );
}

#[tokio::test]
async fn availability_rules_match_the_bot() {
    let app = TestApp::new().await;
    let a = app.user(1, "Alice").await;
    // defaults 00:00-23:59, zero-padded date
    let v: AvailabilityView = app
        .put("/api/me/availability", Some(a), json!({"date": "2026-1-5"}))
        .await
        .json();
    assert_eq!(v.date, "2026-01-05");
    assert_eq!((v.ends_at - v.starts_at).num_minutes(), 23 * 60 + 59);
    // end not after start rolls to the next day
    let v: AvailabilityView = app
        .put(
            "/api/me/availability",
            Some(a),
            json!({"date": "2026-11-21", "start": "22:00", "end": "02:00"}),
        )
        .await
        .json();
    assert!(v.overnight);
    assert_eq!((v.ends_at - v.starts_at).num_hours(), 4);
    // strict parsing names the bad field
    let resp = app
        .put(
            "/api/me/availability",
            Some(a),
            json!({"date": "2026-11-21", "start": "7pm"}),
        )
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert!(
        resp.value()["error"]["message"]
            .as_str()
            .unwrap()
            .contains("start")
    );
    let resp = app
        .put(
            "/api/me/availability",
            Some(a),
            json!({"date": "2026-02-30"}),
        )
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        app.put("/api/me/availability", None, json!({"date": "2026-11-21"}))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}
