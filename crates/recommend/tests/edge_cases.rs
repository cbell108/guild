//! Edge-case integration tests for [`recommend::feed`], run against a self-contained,
//! deterministic fixture (a mirror of `recommend::test_support`, which this test target
//! cannot import). No database and no LLM: only pure in-memory scoring.
//!
//! Covered: the empty snapshot, a viewer with no interests or history, the 7-day
//! horizon, `limit`, score ordering, and the "every card has a reason" guarantee.
#![allow(clippy::unwrap_used)]

use chrono::{DateTime, TimeDelta, Utc};
use hangouts_core::{
    CheckIn, Event, EventId, EventKind, Friendship, FriendshipStatus, Intent, IntentLevel,
    IntentStatus, Location, PrivacySettings, Recommendation, Snapshot, User, UserId, Visibility,
};
use recommend::{AttendanceModel, FeedConfig, HashEmbedder, RecContext, feed};

/// The fixed "now" every fixture is built around.
fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-19T17:00:00Z")
        .unwrap()
        .to_utc()
}

const ALICE: UserId = UserId::from_u128(1);
const BOB: UserId = UserId::from_u128(2);
const CARL: UserId = UserId::from_u128(3);
const DANA: UserId = UserId::from_u128(4);
const ERIN: UserId = UserId::from_u128(5);
/// A user who is not in the fixture at all: no interests, no history, no friends.
const GUEST: UserId = UserId::from_u128(999);

const GAME_NIGHT: EventId = EventId::from_u128(10);
const HOOPS: EventId = EventId::from_u128(11);
const MOVIE: EventId = EventId::from_u128(12);
const HOOPS_PAST: EventId = EventId::from_u128(20);
const GAMES_PAST: EventId = EventId::from_u128(21);
const FAR_FUTURE: EventId = EventId::from_u128(30);

/// A demo user; `created_at` is pinned to `now() - 60 days`.
fn user(id: UserId, name: &str, interests: &[&str]) -> User {
    User {
        id,
        display_name: name.to_owned(),
        bio: None,
        interests: interests.iter().map(|s| (*s).to_owned()).collect(),
        discord_id: None,
        is_demo: true,
        privacy: PrivacySettings::default(),
        created_at: now() - TimeDelta::days(60),
    }
}

/// A hangout event starting `days_from_now` (plus 2 hours) from `now()`, lasting 2 hours.
fn event(id: EventId, title: &str, tags: &[&str], days_from_now: i64) -> Event {
    let starts_at = now() + TimeDelta::days(days_from_now) + TimeDelta::hours(2);
    Event {
        id,
        kind: EventKind::Hangout,
        title: title.to_owned(),
        description: String::new(),
        tags: tags.iter().map(|s| (*s).to_owned()).collect(),
        location: Location {
            name: "Student Center".to_owned(),
            ..Location::default()
        },
        starts_at,
        ends_at: starts_at + TimeDelta::hours(2),
        capacity: None,
        host_id: None,
        crew_id: None,
        external_id: None,
        url: None,
        image_url: None,
        organizer: None,
        created_at: now() - TimeDelta::days(30),
    }
}

/// A confirmed intent expressed `days_ago` days before `now()`.
fn intent(user: UserId, event: EventId, level: IntentLevel, days_ago: i64) -> Intent {
    let at = now() - TimeDelta::days(days_ago);
    Intent {
        user_id: user,
        event_id: event,
        level,
        status: IntentStatus::Confirmed,
        created_at: at,
        updated_at: at,
    }
}

/// An accepted friendship created 50 days before `now()`.
fn friends(a: UserId, b: UserId) -> Friendship {
    Friendship {
        requester: a,
        addressee: b,
        status: FriendshipStatus::Accepted,
        created_at: now() - TimeDelta::days(50),
    }
}

/// The shared demo world: three events inside the 7-day horizon, two in the past, and one
/// 20 days out; Alice (board games) has friends Carl (going to game night) and Dana (going
/// to hoops, private attendance). Mirrors `recommend::test_support::fixture`.
fn fixture() -> Snapshot {
    let mut dana = user(DANA, "Dana", &["basketball"]);
    dana.privacy.attendance = Visibility::Private;
    Snapshot {
        users: vec![
            user(ALICE, "Alice", &["board-games", "tabletop"]),
            user(BOB, "Bob", &[]),
            user(CARL, "Carl", &["board-games", "dnd"]),
            dana,
            user(ERIN, "Erin", &["film", "art"]),
        ],
        events: vec![
            event(
                GAME_NIGHT,
                "Tabletop Tuesday",
                &["board-games", "tabletop"],
                1,
            ),
            event(HOOPS, "Pickup Basketball", &["basketball", "fitness"], 2),
            event(MOVIE, "Student Film Screening", &["film", "art"], 3),
            event(
                HOOPS_PAST,
                "Pickup Basketball",
                &["basketball", "fitness"],
                -5,
            ),
            event(
                GAMES_PAST,
                "Tabletop Tuesday",
                &["board-games", "tabletop"],
                -6,
            ),
            event(FAR_FUTURE, "Board Game Marathon", &["board-games"], 20),
        ],
        intents: vec![
            intent(BOB, HOOPS_PAST, IntentLevel::Going, 7),
            intent(CARL, GAMES_PAST, IntentLevel::Going, 8),
            intent(ALICE, GAMES_PAST, IntentLevel::Going, 8),
            intent(CARL, GAME_NIGHT, IntentLevel::Going, 1),
            intent(DANA, HOOPS, IntentLevel::Going, 1),
            intent(ERIN, MOVIE, IntentLevel::Interested, 1),
        ],
        checkins: vec![
            CheckIn {
                user_id: BOB,
                event_id: HOOPS_PAST,
                at: now() - TimeDelta::days(5),
            },
            CheckIn {
                user_id: CARL,
                event_id: GAMES_PAST,
                at: now() - TimeDelta::days(6),
            },
        ],
        friendships: vec![friends(ALICE, CARL), friends(DANA, ALICE)],
        ..Snapshot::default()
    }
}

/// Runs `feed` on `snapshot` for `viewer` with the prior attendance model and the
/// deterministic hash embedder.
fn run(snapshot: &Snapshot, viewer: UserId, cfg: &FeedConfig) -> Vec<Recommendation> {
    let model = AttendanceModel::prior();
    let embedder = HashEmbedder::default();
    let ctx = RecContext {
        snapshot,
        user: viewer,
        now: now(),
        model: &model,
        embedder: &embedder,
    };
    feed(&ctx, cfg)
}

/// The event ids in recommendation order.
fn ids(recs: &[Recommendation]) -> Vec<EventId> {
    recs.iter().map(|r| r.event_id).collect()
}

#[test]
fn empty_snapshot_yields_an_empty_feed() {
    let recs = run(&Snapshot::default(), GUEST, &FeedConfig::default());
    assert_eq!(recs.len(), 0);
}

#[test]
fn viewer_without_interests_or_history_still_gets_cards_with_reasons() {
    let snap = fixture();
    let recs = run(&snap, GUEST, &FeedConfig::default());
    assert!(
        !recs.is_empty(),
        "a brand-new viewer should still get cards (popularity keeps the feed alive)"
    );
    for r in &recs {
        assert!(
            !r.reasons.is_empty(),
            "every card needs at least one reason"
        );
    }
}

#[test]
fn only_events_inside_the_seven_day_horizon_appear() {
    let snap = fixture();
    let recs = run(&snap, ALICE, &FeedConfig::default());
    let found = ids(&recs);
    assert_eq!(
        found.len(),
        3,
        "exactly the three upcoming events should be candidates, got {found:?}"
    );
    for id in [GAME_NIGHT, HOOPS, MOVIE] {
        assert!(
            ids(&recs).contains(&id),
            "expected {id:?} inside the horizon"
        );
    }
    for id in [HOOPS_PAST, GAMES_PAST, FAR_FUTURE] {
        assert!(!ids(&recs).contains(&id), "{id:?} is outside the horizon");
    }
}

#[test]
fn limit_zero_is_empty_and_limit_one_is_one_card() {
    let snap = fixture();
    let none = run(
        &snap,
        ALICE,
        &FeedConfig {
            limit: 0,
            ..FeedConfig::default()
        },
    );
    assert_eq!(none.len(), 0);
    let one = run(
        &snap,
        ALICE,
        &FeedConfig {
            limit: 1,
            ..FeedConfig::default()
        },
    );
    assert_eq!(one.len(), 1);
}

#[test]
fn scores_are_sorted_in_descending_order() {
    let snap = fixture();
    let recs = run(&snap, ALICE, &FeedConfig::default());
    assert!(recs.len() >= 2, "need at least two cards to check ordering");
    assert!(
        recs.windows(2).all(|pair| pair[0].score >= pair[1].score),
        "scores must be non-increasing, got {:?}",
        recs.iter().map(|r| r.score).collect::<Vec<_>>()
    );
}

#[test]
fn every_card_has_at_least_one_reason_for_every_viewer() {
    let snap = fixture();
    for viewer in [ALICE, BOB, GUEST] {
        let recs = run(&snap, viewer, &FeedConfig::default());
        assert!(!recs.is_empty(), "viewer {viewer:?} should still get cards");
        for r in &recs {
            assert!(
                !r.reasons.is_empty(),
                "card {:?} for viewer {viewer:?} has no reasons",
                r.event_id
            );
        }
    }
}
