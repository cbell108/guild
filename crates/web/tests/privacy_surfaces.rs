//! Privacy rules from `docs/API.md` checked on every surface that shows attendance:
//! the events list, event detail and the feed must agree, and no response may leak an
//! attendee list, a total attendee count, or another user's interests or privacy settings.
#![allow(clippy::unwrap_used)]

mod common;

use api::{EventCard, EventDetail, FeedResponse};
use common::TestApp;
use hangouts_core::{EventId, UserId};
use serde_json::{Value, json};

/// Friend presence for `event` as `viewer` sees it on (list, detail, feed).
async fn presence_everywhere(app: &TestApp, viewer: UserId, event: EventId) -> [(u32, u32); 3] {
    let list: Vec<EventCard> = app.get("/api/events", Some(viewer)).await.json();
    let card = list.iter().find(|c| c.id == event).unwrap();
    let detail: EventDetail = app
        .get(&format!("/api/events/{event}"), Some(viewer))
        .await
        .json();
    let feed: FeedResponse = app.get("/api/feed", Some(viewer)).await.json();
    let item = feed.items.iter().find(|i| i.event.id == event).unwrap();
    [
        (card.friends.going, card.friends.interested),
        (detail.card.friends.going, detail.card.friends.interested),
        (item.event.friends.going, item.event.friends.interested),
    ]
}

async fn set_intent(app: &TestApp, user: UserId, event: EventId, level: &str) {
    let resp = app
        .put(
            &format!("/api/events/{event}/intent"),
            Some(user),
            json!({ "level": level }),
        )
        .await;
    assert!(resp.status.is_success(), "{}", resp.text());
}

#[tokio::test]
async fn only_visible_friends_count_on_every_surface() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let pal = app.user(2, "Pal").await;
    let shy = app.user(3, "Shy").await;
    let stranger = app.user(4, "Stranger").await;
    app.befriend(me, pal).await;
    app.befriend(me, shy).await;
    let event = app.event(10, "Board games", 3, None).await;

    set_intent(&app, pal, event, "going").await;
    set_intent(&app, shy, event, "interested").await;
    set_intent(&app, stranger, event, "going").await;
    app.put(
        "/api/me/privacy",
        Some(shy),
        json!({"attendance": "private", "buddy_matching": true, "personalize": true}),
    )
    .await;

    // Pal counts; the private friend and the non-friend never do, on any surface.
    assert_eq!(presence_everywhere(&app, me, event).await, [(1, 0); 3]);

    // The stranger sees no friends at all (none of the attendees are their friends).
    assert_eq!(
        presence_everywhere(&app, stranger, event).await,
        [(0, 0); 3]
    );
}

#[tokio::test]
async fn samples_name_only_visible_friends() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let pal = app.user(2, "Pal").await;
    let shy = app.user(3, "Shy").await;
    let stranger = app.user(4, "Stranger").await;
    app.befriend(me, pal).await;
    app.befriend(me, shy).await;
    let event = app.event(10, "Board games", 3, None).await;
    for u in [pal, shy, stranger] {
        set_intent(&app, u, event, "going").await;
    }
    app.put(
        "/api/me/privacy",
        Some(shy),
        json!({"attendance": "private", "buddy_matching": true, "personalize": true}),
    )
    .await;

    let detail: EventDetail = app
        .get(&format!("/api/events/{event}"), Some(me))
        .await
        .json();
    let sampled: Vec<UserId> = detail.card.friends.sample.iter().map(|u| u.id).collect();
    assert_eq!(sampled, vec![pal]);
}

/// Every object key anywhere in a JSON value.
fn keys(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                out.push(k.clone());
                keys(v, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|v| keys(v, out)),
        _ => {}
    }
}

#[tokio::test]
async fn responses_never_leak_attendees_or_other_users_settings() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let pal = app.user(2, "Pal").await;
    let stranger = app.user(3, "Stranger").await;
    app.befriend(me, pal).await;
    let event = app.event(10, "Board games", 3, Some(5)).await;
    for u in [pal, stranger] {
        set_intent(&app, u, event, "going").await;
    }

    let forbidden = [
        "attendees",
        "attendee_count",
        "attending",
        "going_count",
        "interests",
        "privacy",
        "followers",
        "follower_count",
        "member_count",
    ];
    for uri in [
        "/api/events".to_owned(),
        format!("/api/events/{event}"),
        "/api/feed".to_owned(),
        format!("/api/events/{event}/buddy"),
        "/api/friends".to_owned(),
        "/api/demo-users".to_owned(),
        "/api/users?q=a".to_owned(),
    ] {
        let resp = app.get(&uri, Some(me)).await;
        assert!(resp.status.is_success(), "{uri}: {}", resp.text());
        let mut found = Vec::new();
        keys(&resp.value(), &mut found);
        for key in forbidden {
            assert!(!found.iter().any(|k| k == key), "{uri} exposes `{key}`");
        }
        // The stranger's id may only appear where the viewer could legitimately see them;
        // they are not a friend, so no attendance surface may mention them.
        if uri.starts_with("/api/events") || uri == "/api/feed" {
            assert!(
                !resp.text().contains(&stranger.to_string()),
                "{uri} mentions a non-friend attendee"
            );
        }
    }

    // `/api/me` is the one place a user's own interests and privacy appear.
    let mut own = Vec::new();
    keys(&app.get("/api/me", Some(me)).await.value(), &mut own);
    assert!(own.iter().any(|k| k == "interests") && own.iter().any(|k| k == "privacy"));
}

#[tokio::test]
async fn spots_left_is_the_only_count_for_capped_events() {
    let app = TestApp::new().await;
    let me = app.user(1, "Me").await;
    let a = app.user(2, "A").await;
    let b = app.user(3, "B").await;
    let event = app.event(10, "Small table", 3, Some(3)).await;
    set_intent(&app, a, event, "going").await;
    set_intent(&app, b, event, "interested").await;

    let detail: EventDetail = app
        .get(&format!("/api/events/{event}"), Some(me))
        .await
        .json();
    // Interested never takes a spot; non-friends are not counted in presence.
    assert_eq!(detail.card.spots_left, Some(2));
    assert_eq!(
        (detail.card.friends.going, detail.card.friends.interested),
        (0, 0)
    );
}
