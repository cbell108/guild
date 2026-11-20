//! A tiny hand-built world shared by the recommender's unit tests.

use chrono::{DateTime, TimeDelta, Utc};
use hangouts_core::{
    CheckIn, Event, EventId, EventKind, Friendship, FriendshipStatus, Intent, IntentLevel,
    IntentStatus, Location, PrivacySettings, Snapshot, User, UserId, Visibility,
};

use crate::{AttendanceModel, Embedder, RecContext};

pub const ALICE: UserId = UserId::from_u128(1);
pub const BOB: UserId = UserId::from_u128(2);
pub const CARL: UserId = UserId::from_u128(3);
pub const DANA: UserId = UserId::from_u128(4);
pub const ERIN: UserId = UserId::from_u128(5);

pub const GAME_NIGHT: EventId = EventId::from_u128(10);
pub const HOOPS: EventId = EventId::from_u128(11);
pub const MOVIE: EventId = EventId::from_u128(12);
pub const HOOPS_PAST: EventId = EventId::from_u128(20);
pub const GAMES_PAST: EventId = EventId::from_u128(21);
pub const FAR_FUTURE: EventId = EventId::from_u128(30);

pub fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-19T17:00:00Z")
        .unwrap()
        .to_utc()
}

pub fn user(id: UserId, name: &str, interests: &[&str]) -> User {
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

pub fn event(id: EventId, title: &str, tags: &[&str], days_from_now: i64) -> Event {
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

pub fn intent(user: UserId, event: EventId, level: IntentLevel, days_ago: i64) -> Intent {
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

pub fn friends(a: UserId, b: UserId) -> Friendship {
    Friendship {
        requester: a,
        addressee: b,
        status: FriendshipStatus::Accepted,
        created_at: now() - TimeDelta::days(50),
    }
}

/// Alice likes board games; Bob went to basketball; Carl (Alice's friend) is going to game
/// night; Dana (Alice's friend, private attendance) is going to hoops; Erin likes film.
pub fn fixture() -> Snapshot {
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

pub fn ctx<'a>(
    snapshot: &'a Snapshot,
    user: UserId,
    model: &'a AttendanceModel,
    embedder: &'a dyn Embedder,
) -> RecContext<'a> {
    RecContext {
        snapshot,
        user,
        now: now(),
        model,
        embedder,
    }
}
