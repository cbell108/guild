//! Edge cases for the store, complementing `store.rs`.
//!
//! Covers the behaviours the happy-path suite does not pin down: LIKE
//! escaping in `search_users`, any-of tag filtering in `list_events`,
//! `remove_friend` in both directions, `delete_history` keeping
//! friendships and crews, and `set_availability` replacing the same
//! campus-local start date only.
#![allow(
    clippy::unwrap_used,
    clippy::many_single_char_names,
    clippy::assert_is_empty
)]

use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use hangouts_core::{
    Availability, CheckIn, Crew, CrewId, CrewRole, Event, EventId, EventKind, FriendshipStatus,
    IntentLevel, Location, PrivacySettings, User, UserId, Window,
};
use store::{EventFilter, Store};

fn t0() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-20T12:00:00Z")
        .unwrap()
        .into()
}

fn utc(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().into()
}

async fn db() -> Store {
    let s = Store::in_memory().await.unwrap();
    s.migrate().await.unwrap();
    s
}

fn user(n: u128, name: &str) -> User {
    User {
        id: UserId::from_u128(n),
        display_name: name.into(),
        bio: None,
        interests: vec!["gaming".into()],
        discord_id: None,
        is_demo: false,
        privacy: PrivacySettings::default(),
        created_at: t0(),
    }
}

async fn with_users(s: &Store, n: u128) {
    for i in 1..=n {
        s.insert_user(&user(i, &format!("User {i}"))).await.unwrap();
    }
}

fn event(n: u128, title: &str, hours: i64, tags: Vec<String>) -> Event {
    Event {
        id: EventId::from_u128(n),
        kind: EventKind::Hangout,
        title: title.into(),
        description: "desc".into(),
        tags,
        location: Location {
            name: "Student Center".into(),
            ..Location::default()
        },
        starts_at: t0() + TimeDelta::hours(hours),
        ends_at: t0() + TimeDelta::hours(hours + 2),
        capacity: None,
        host_id: None,
        crew_id: None,
        external_id: None,
        url: None,
        image_url: None,
        organizer: None,
        created_at: t0(),
    }
}

async fn titles(s: &Store, f: EventFilter) -> Vec<String> {
    let mut v = s
        .list_events(&f)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.title.clone())
        .collect::<Vec<_>>();
    v.sort();
    v
}

#[tokio::test]
async fn search_users_treats_percent_and_underscore_literally() {
    let s = db().await;
    s.insert_user(&user(1, "100%")).await.unwrap();
    s.insert_user(&user(2, "a_b")).await.unwrap();
    // Three characters, no wildcard chars: a live "_" wildcard would also match this.
    s.insert_user(&user(3, "xyz")).await.unwrap();

    // A lone "%" must be a literal percent, not a match-everything wildcard.
    let hit = s.search_users("%", 10).await.unwrap();
    assert_eq!(
        hit.iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![UserId::from_u128(1)],
        "a lone % is a literal percent, not a wildcard"
    );
    // A lone "_" must be a literal underscore, not a single-char wildcard.
    let hit = s.search_users("_", 10).await.unwrap();
    assert_eq!(
        hit.iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![UserId::from_u128(2)],
        "a lone _ is a literal underscore, not a wildcard"
    );
    // Escaping must not break ordinary case-insensitive substring search.
    let hit = s.search_users("A_B", 10).await.unwrap();
    assert_eq!(
        hit.iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![UserId::from_u128(2)]
    );
}

#[tokio::test]
async fn list_events_tags_match_any_of() {
    let s = db().await;
    s.insert_event(&event(
        1,
        "Games",
        1,
        vec!["gaming".into(), "board-games".into()],
    ))
    .await
    .unwrap();
    s.insert_event(&event(2, "Open Mic", 2, vec!["music".into()]))
        .await
        .unwrap();
    s.insert_event(&event(
        3,
        "Game Night w/ DJ",
        3,
        vec!["gaming".into(), "music".into(), "food".into()],
    ))
    .await
    .unwrap();

    // Any of the requested tags hits, not all of them: "gaming" OR "music"
    // picks every event, while all-of would only pick the DJ event.
    assert_eq!(
        titles(
            &s,
            EventFilter {
                tags: vec!["gaming".into(), "music".into()],
                ..Default::default()
            }
        )
        .await,
        vec!["Game Night w/ DJ", "Games", "Open Mic"]
    );
    // A single tag still works.
    assert_eq!(
        titles(
            &s,
            EventFilter {
                tags: vec!["gaming".into()],
                ..Default::default()
            }
        )
        .await,
        vec!["Game Night w/ DJ", "Games"]
    );
    // An unknown tag matches nothing, and does not error.
    assert_eq!(
        titles(
            &s,
            EventFilter {
                tags: vec!["nope".into()],
                ..Default::default()
            }
        )
        .await,
        Vec::<String>::new()
    );
    // An event missing every requested tag is not returned.
    assert_eq!(
        titles(
            &s,
            EventFilter {
                tags: vec!["board-games".into()],
                ..Default::default()
            }
        )
        .await,
        vec!["Games"]
    );
}

#[tokio::test]
async fn remove_friend_works_in_both_directions() {
    let s = db().await;
    with_users(&s, 3).await;
    let u = |n| UserId::from_u128(n);

    // Pending request stored 1 -> 2; the reverse direction removes it too.
    let f = s.request_friend(u(1), u(2), t0()).await.unwrap();
    assert_eq!(f.status, FriendshipStatus::Pending);
    assert!(
        s.remove_friend(u(2), u(1)).await.unwrap(),
        "reverse of the stored row"
    );
    assert!(
        !s.remove_friend(u(1), u(2)).await.unwrap(),
        "already removed"
    );

    // Accepted friendship stored 2 -> 3, removable from either side.
    s.request_friend(u(2), u(3), t0()).await.unwrap();
    s.request_friend(u(3), u(2), t0()).await.unwrap(); // reciprocal -> accepted
    assert_eq!(s.friend_ids(u(2)).await.unwrap(), vec![u(3)]);
    assert!(s.remove_friend(u(3), u(2)).await.unwrap());
    assert!(s.friend_ids(u(2)).await.unwrap().is_empty());
    assert!(s.friend_ids(u(3)).await.unwrap().is_empty());
    assert!(s.list_friendships(u(3)).await.unwrap().is_empty());
}

#[tokio::test]
async fn delete_history_keeps_friendships_and_crews() {
    let s = db().await;
    with_users(&s, 3).await;
    let (a, b, c, e) = (
        UserId::from_u128(1),
        UserId::from_u128(2),
        UserId::from_u128(3),
        EventId::from_u128(1),
    );
    s.insert_event(&event(1, "E", 1, vec![])).await.unwrap();

    // a's history: an intent, a buddy request, a check-in, an availability
    // window. b keeps an intent of their own.
    s.set_intent(a, e, IntentLevel::Going, t0()).await.unwrap();
    s.set_intent(b, e, IntentLevel::Interested, t0())
        .await
        .unwrap();
    s.set_buddy_looking(a, e, true, t0()).await.unwrap();
    s.record_checkin(&CheckIn {
        user_id: a,
        event_id: e,
        at: t0(),
    })
    .await
    .unwrap();
    let window = Window::new(t0(), t0() + TimeDelta::hours(2)).unwrap();
    s.set_availability(&Availability { user_id: a, window })
        .await
        .unwrap();

    // a owns a crew; b is a member.
    let crew = Crew {
        id: CrewId::from_u128(1),
        name: "C".into(),
        description: "d".into(),
        created_by: a,
        created_at: t0(),
    };
    s.insert_crew(&crew).await.unwrap();
    s.add_crew_member(crew.id, b, CrewRole::Member, t0())
        .await
        .unwrap();

    // a<->b accepted, a->c still pending.
    s.request_friend(a, b, t0()).await.unwrap();
    s.request_friend(b, a, t0()).await.unwrap();
    s.request_friend(a, c, t0()).await.unwrap();

    s.delete_history(a).await.unwrap();

    let snap = s.load_snapshot().await.unwrap();
    assert!(
        snap.intents.iter().all(|i| i.user_id != a)
            && snap.checkins.iter().all(|c| c.user_id != a)
            && snap.buddy_requests.iter().all(|r| r.user_id != a),
        "a's history is gone"
    );
    // The account survives.
    assert_eq!(snap.users.len(), 3);
    assert!(snap.user(a).is_some());
    // Friendships survive, pending and accepted.
    assert_eq!(snap.friendships.len(), 2);
    assert!(
        snap.friendships
            .iter()
            .any(|f| f.status == FriendshipStatus::Accepted),
        "accepted friendship kept"
    );
    assert!(
        snap.friendships
            .iter()
            .any(|f| f.status == FriendshipStatus::Pending),
        "pending request kept"
    );
    // Crews and memberships survive.
    assert_eq!(snap.crews.len(), 1);
    assert_eq!(snap.crew_members.len(), 2);
    assert!(
        snap.crew_members
            .iter()
            .any(|m| m.user_id == a && m.role == CrewRole::Owner)
    );
    assert!(
        snap.crew_members
            .iter()
            .any(|m| m.user_id == b && m.role == CrewRole::Member)
    );
    assert_eq!(s.list_crews_for_user(a).await.unwrap().len(), 1);
    // Availability was one of the things delete_history clears.
    let day = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
    assert!(
        s.list_availability_on(day, Some(&[a]))
            .await
            .unwrap()
            .is_empty()
    );
    // Nobody else's history is touched.
    assert_eq!(s.get_intent(a, e).await.unwrap(), None);
    assert_eq!(
        s.get_intent(b, e).await.unwrap().unwrap().level,
        IntentLevel::Interested
    );
}

#[tokio::test]
async fn set_availability_replaces_same_local_date_only() {
    let s = db().await;
    s.insert_user(&user(1, "A")).await.unwrap();
    s.insert_user(&user(2, "B")).await.unwrap();
    let a = UserId::from_u128(1);
    let b = UserId::from_u128(2);
    let day = |d: u32| NaiveDate::from_ymd_opt(2026, 11, d).unwrap();
    let w = |start: &str, end: &str| Window::new(utc(start), utc(end)).unwrap();

    // Overnight window: local Nov 21 22:00 to Nov 22 02:00 (EST).
    let a1 = w("2026-11-22T03:00:00Z", "2026-11-22T07:00:00Z");
    s.set_availability(&Availability {
        user_id: a,
        window: a1,
    })
    .await
    .unwrap();
    // A window on an earlier local date coexists, even though it is
    // adjacent.
    let a0 = w("2026-11-20T19:30:00Z", "2026-11-20T22:30:00Z");
    s.set_availability(&Availability {
        user_id: a,
        window: a0,
    })
    .await
    .unwrap();
    assert_eq!(
        s.list_availability_on(day(20), None).await.unwrap().len(),
        1
    );
    assert_eq!(
        s.list_availability_on(day(21), None).await.unwrap().len(),
        1
    );
    assert_eq!(
        s.list_availability_on(day(22), None).await.unwrap().len(),
        1,
        "the overnight window overlaps its end date"
    );

    // Same local start date as the overnight one, but a different
    // non-overlapping time: it must replace it, keyed by local date, not
    // time overlap.
    let a2 = w("2026-11-21T20:00:00Z", "2026-11-21T22:00:00Z");
    s.set_availability(&Availability {
        user_id: a,
        window: a2,
    })
    .await
    .unwrap();
    let on21 = s.list_availability_on(day(21), Some(&[a])).await.unwrap();
    assert_eq!(on21.len(), 1);
    assert_eq!(on21[0].user_id, a);
    assert_eq!(
        on21[0].window, a2,
        "same local start date replaces, even without time overlap"
    );
    assert!(
        s.list_availability_on(day(22), None)
            .await
            .unwrap()
            .is_empty(),
        "the replaced overnight window's tail is gone"
    );
    assert_eq!(
        s.list_availability_on(day(20), None).await.unwrap().len(),
        1,
        "other local dates are untouched"
    );

    // Another user's window on the same local date is untouched.
    let b1 = w("2026-11-21T15:00:00Z", "2026-11-21T17:00:00Z");
    s.set_availability(&Availability {
        user_id: b,
        window: b1,
    })
    .await
    .unwrap();
    let on21 = s.list_availability_on(day(21), None).await.unwrap();
    assert_eq!(on21.len(), 2);
    assert_eq!(
        on21.iter()
            .filter(|x| x.user_id == b)
            .map(|x| x.window)
            .collect::<Vec<_>>(),
        vec![b1]
    );
}
