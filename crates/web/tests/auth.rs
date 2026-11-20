//! A5: demo login and session cookie.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

mod common;

use api::{Me, UserSummary};
use axum::http::{StatusCode, header};
use common::TestApp;
use hangouts_core::UserId;
use serde_json::json;

#[tokio::test]
async fn demo_users_lists_only_demo_accounts() {
    let app = TestApp::new().await;
    app.user(1, "Zed").await;
    app.user(2, "Amy").await;
    app.store()
        .upsert_discord_user("d", "Real Person", common::now())
        .await
        .unwrap();
    let users: Vec<UserSummary> = app.get("/api/demo-users", None).await.json();
    assert_eq!(
        users
            .iter()
            .map(|u| u.display_name.as_str())
            .collect::<Vec<_>>(),
        ["Amy", "Zed"]
    );
}

#[tokio::test]
async fn login_sets_cookie_and_me_works() {
    let app = TestApp::new().await;
    let id = app.user(1, "Amy").await;

    assert_eq!(
        app.get("/api/me", None).await.status,
        StatusCode::UNAUTHORIZED
    );

    let resp = app.post("/api/login", None, json!({"user_id": id})).await;
    assert_eq!(resp.status, StatusCode::OK);
    let cookie = resp
        .headers
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(cookie.starts_with(&format!("uid={id}")), "{cookie}");
    assert!(
        cookie.contains("HttpOnly") && cookie.contains("SameSite=Lax") && cookie.contains("Path=/")
    );
    let me: Me = resp.json();
    assert_eq!(me.user.display_name, "Amy");
    assert_eq!(me.interests, ["gaming"]);

    let me2: Me = app.get("/api/me", Some(id)).await.json();
    assert_eq!(me, me2);
}

#[tokio::test]
async fn login_unknown_user_is_404_and_bad_cookie_is_401() {
    let app = TestApp::new().await;
    let resp = app
        .post("/api/login", None, json!({"user_id": UserId::from_u128(9)}))
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
    // a well-formed cookie for a user that does not exist
    assert_eq!(
        app.get("/api/me", Some(UserId::from_u128(9))).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn logout_clears_cookie() {
    let app = TestApp::new().await;
    let resp = app.post("/api/logout", None, json!({})).await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);
    let cookie = resp
        .headers
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cookie.starts_with("uid="));
    assert!(cookie.contains("Max-Age=0"), "{cookie}");
}

#[tokio::test]
async fn user_search_excludes_self_and_exposes_summaries_only() {
    let app = TestApp::new().await;
    let me = app.user(1, "Sam").await;
    app.user(2, "Samantha").await;
    app.user(3, "Bob").await;
    let resp = app.get("/api/users?q=sam", Some(me)).await;
    let value = resp.value();
    assert_eq!(value.as_array().unwrap().len(), 1);
    assert_eq!(value[0]["display_name"], "Samantha");
    assert!(value[0].get("interests").is_none() && value[0].get("privacy").is_none());
    assert_eq!(
        app.get("/api/users?q=sam", None).await.status,
        StatusCode::UNAUTHORIZED
    );
}
