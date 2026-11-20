//! Shared helpers for API tests: an in-memory app with a fixed clock, driven via `oneshot`.
#![allow(dead_code, clippy::unwrap_used)]

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use chrono::{DateTime, TimeDelta, Utc};
use hangouts_core::{Event, EventId, EventKind, Location, PrivacySettings, User, UserId};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use serde_json::Value;
use store::Store;
use tower::ServiceExt;
use web::AppState;

/// The fixed "now" of every test: Fri 2026-11-20 15:00 UTC (10:00 in New York).
pub fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-20T15:00:00Z")
        .unwrap()
        .into()
}

/// A test application plus direct store access for arranging data.
pub struct TestApp {
    pub router: Router,
    pub state: AppState,
}

impl TestApp {
    pub async fn new() -> Self {
        let store = Store::in_memory().await.unwrap();
        store.migrate().await.unwrap();
        let llm = llm::LlmClient::new(llm::LlmConfig {
            // Port 9 refuses fast, so the LLM is reliably "down" in tests.
            base_url: "http://127.0.0.1:9/v1".into(),
            ..llm::LlmConfig::default()
        })
        .unwrap();
        let mut state = AppState::new(store, Arc::new(llm));
        state.clock = Arc::new(now);
        state.static_dir = std::env::temp_dir().join("campus-hangouts-no-static");
        Self {
            router: web::app(state.clone()),
            state,
        }
    }

    pub fn store(&self) -> &Store {
        &self.state.store
    }

    /// Sends a request; returns status, headers' `set-cookie` and the raw body.
    pub async fn send(&self, req: Request<Body>) -> Resp {
        let resp = self.router.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let headers = resp.headers().clone();
        let body = resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();
        Resp {
            status,
            headers,
            body,
        }
    }

    pub async fn get(&self, uri: &str, as_user: Option<UserId>) -> Resp {
        self.send(request("GET", uri, as_user, None)).await
    }

    pub async fn post(&self, uri: &str, as_user: Option<UserId>, body: Value) -> Resp {
        self.send(request("POST", uri, as_user, Some(body))).await
    }

    pub async fn put(&self, uri: &str, as_user: Option<UserId>, body: Value) -> Resp {
        self.send(request("PUT", uri, as_user, Some(body))).await
    }

    pub async fn delete(&self, uri: &str, as_user: Option<UserId>) -> Resp {
        self.send(request("DELETE", uri, as_user, None)).await
    }

    /// Inserts a demo user with the default privacy settings.
    pub async fn user(&self, n: u128, name: &str) -> UserId {
        let id = UserId::from_u128(n);
        self.store()
            .insert_user(&User {
                id,
                display_name: name.into(),
                bio: None,
                interests: vec!["gaming".into()],
                discord_id: None,
                is_demo: true,
                privacy: PrivacySettings::default(),
                created_at: now(),
            })
            .await
            .unwrap();
        id
    }

    /// Makes two users accepted friends.
    pub async fn befriend(&self, a: UserId, b: UserId) {
        self.store().request_friend(a, b, now()).await.unwrap();
        self.store().accept_friend(b, a, now()).await.unwrap();
    }

    /// Inserts a hangout starting `hours` from now.
    pub async fn event(&self, n: u128, title: &str, hours: i64, capacity: Option<u32>) -> EventId {
        let id = EventId::from_u128(n);
        self.store()
            .insert_event(&Event {
                id,
                kind: EventKind::Hangout,
                title: title.into(),
                description: format!("{title} description"),
                tags: vec!["gaming".into()],
                location: Location {
                    name: "Student Center".into(),
                    ..Location::default()
                },
                starts_at: now() + TimeDelta::hours(hours),
                ends_at: now() + TimeDelta::hours(hours + 2),
                capacity,
                host_id: None,
                crew_id: None,
                external_id: None,
                url: None,
                image_url: None,
                organizer: None,
                created_at: now(),
            })
            .await
            .unwrap();
        id
    }
}

/// Builds a request, optionally with the session cookie and a JSON body.
pub fn request(
    method: &str,
    uri: &str,
    as_user: Option<UserId>,
    body: Option<Value>,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(user) = as_user {
        builder = builder.header(header::COOKIE, format!("uid={user}"));
    }
    match body {
        Some(json) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

/// A response with helpers to decode the body.
pub struct Resp {
    pub status: StatusCode,
    pub headers: axum::http::HeaderMap,
    pub body: Vec<u8>,
}

impl Resp {
    pub fn json<T: DeserializeOwned>(&self) -> T {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|e| panic!("bad body {:?}: {e}", String::from_utf8_lossy(&self.body)))
    }

    pub fn value(&self) -> Value {
        self.json()
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// The error code of an `ApiError` body.
    pub fn error_code(&self) -> String {
        self.value()["error"]["code"].as_str().unwrap().to_owned()
    }
}
