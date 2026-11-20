//! Behavioural tests for the store against an in-memory database.
#![allow(
    clippy::unwrap_used,
    clippy::many_single_char_names,
    clippy::assert_is_empty
)]

use chrono::{DateTime, TimeDelta, Utc};
use hangouts_core::{
    Availability, CheckIn, Crew, CrewId, CrewRole, Event, EventId, EventKind, Friendship,
    FriendshipStatus, Gear, IntentLevel, IntentStatus, Location, Presence, PrivacySettings, Setup,
    User, UserId, Visibility,
};
use store::{EventFilter, Store, StoreError};

fn t0() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-20T12:00:00Z")
        .unwrap()
        .into()
}

async fn count(s: &Store, f: EventFilter) -> usize {
    s.list_events(&f).await.unwrap().len()
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

fn event(n: u128, title: &str, hours: i64, capacity: Option<u32>) -> Event {
    Event {
        id: EventId::from_u128(n),
        kind: EventKind::Hangout,
        title: title.into(),
        description: "desc".into(),
        tags: vec!["gaming".into(), "board-games".into()],
        location: Location {
            name: "Student Center".into(),
            ..Location::default()
        },
        starts_at: t0() + TimeDelta::hours(hours),
        ends_at: t0() + TimeDelta::hours(hours + 2),
        capacity,
        host_id: None,
        crew_id: None,
        external_id: None,
        url: None,
        image_url: None,
        organizer: None,
        created_at: t0(),
    }
}

#[tokio::test]
async fn users_roundtrip_and_queries() {
    let s = db().await;
    let mut u = user(1, "Ada");
    u.is_demo = true;
    u.discord_id = Some("d1".into());
    u.bio = Some("hi".into());
    s.insert_user(&u).await.unwrap();
    s.insert_user(&user(2, "Bob_100%")).await.unwrap();
    assert_eq!(s.get_user(u.id).await.unwrap(), Some(u.clone()));
    assert!(s.get_user(UserId::from_u128(99)).await.unwrap().is_none());
    assert!(matches!(
        s.insert_user(&u).await,
        Err(StoreError::Conflict(_))
    ));
    assert_eq!(s.list_users().await.unwrap().len(), 2);
    assert_eq!(s.list_demo_users().await.unwrap(), vec![u.clone()]);
    assert_eq!(s.search_users("ADA", 5).await.unwrap().len(), 1);
    assert_eq!(
        s.search_users("_", 5).await.unwrap().len(),
        1,
        "LIKE wildcards are escaped"
    );
    assert_eq!(s.search_users("", 1).await.unwrap().len(), 1);

    s.set_interests(u.id, &["a".into(), "b".into()])
        .await
        .unwrap();
    let p = PrivacySettings {
        attendance: Visibility::Private,
        buddy_matching: false,
        personalize: false,
    };
    s.set_privacy(u.id, &p).await.unwrap();
    let got = s.get_user(u.id).await.unwrap().unwrap();
    assert_eq!(got.interests, vec!["a", "b"]);
    assert_eq!(got.privacy, p);
    let missing = UserId::from_u128(99);
    assert!(matches!(
        s.set_interests(missing, &[]).await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        s.set_privacy(missing, &p).await,
        Err(StoreError::NotFound)
    ));
}

#[tokio::test]
async fn discord_upsert_is_stable() {
    let s = db().await;
    let a = s.upsert_discord_user("42", "Old", t0()).await.unwrap();
    let b = s
        .upsert_discord_user("42", "New", t0() + TimeDelta::hours(1))
        .await
        .unwrap();
    assert_eq!(a.id, b.id);
    assert_eq!(b.display_name, "New");
    assert_eq!(b.created_at, t0());
    assert!(!b.is_demo);
    assert_eq!(s.list_users().await.unwrap().len(), 1);
}

#[tokio::test]
async fn events_crud_filter_and_tags() {
    let s = db().await;
    let mut official = event(2, "Career Fair", 30, None);
    official.kind = EventKind::Official;
    official.external_id = Some("ext-1".into());
    official.tags = vec!["career".into()];
    official.location.lat = Some(34.0);
    s.insert_event(&event(1, "Board Games", 1, Some(4)))
        .await
        .unwrap();
    s.insert_event(&official).await.unwrap();
    assert!(matches!(
        s.insert_event(&official).await,
        Err(StoreError::Conflict(_))
    ));
    assert_eq!(
        s.get_event(official.id).await.unwrap(),
        Some(official.clone())
    );

    let all = s.list_events(&EventFilter::default()).await.unwrap();
    assert_eq!(
        all.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(),
        ["Board Games", "Career Fair"]
    );

    assert_eq!(
        count(
            &s,
            EventFilter {
                kind: Some(EventKind::Official),
                ..Default::default()
            }
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &s,
            EventFilter {
                tags: vec!["career".into(), "nope".into()],
                ..Default::default()
            }
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &s,
            EventFilter {
                tags: vec!["gaming".into()],
                ..Default::default()
            }
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &s,
            EventFilter {
                query: Some("fair".into()),
                ..Default::default()
            }
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &s,
            EventFilter {
                query: Some("%".into()),
                ..Default::default()
            }
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &s,
            EventFilter {
                from: Some(t0() + TimeDelta::hours(10)),
                ..Default::default()
            }
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &s,
            EventFilter {
                to: Some(t0() + TimeDelta::hours(2)),
                ..Default::default()
            }
        )
        .await,
        1
    );
    // overlap: first event runs hours 1..3
    assert_eq!(
        count(
            &s,
            EventFilter {
                from: Some(t0() + TimeDelta::hours(2)),
                to: Some(t0() + TimeDelta::hours(5)),
                ..Default::default()
            }
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &s,
            EventFilter {
                limit: Some(1),
                ..Default::default()
            }
        )
        .await,
        1
    );

    let tags = s.list_tags().await.unwrap();
    assert_eq!(
        tags.iter()
            .map(|(t, _)| t.as_str())
            .collect::<Vec<_>>()
            .len(),
        3
    );
}

#[tokio::test]
async fn upsert_official_keeps_id() {
    let s = db().await;
    let mut e = event(1, "Talk", 5, None);
    e.kind = EventKind::Official;
    assert!(matches!(
        s.upsert_official_event(&e).await,
        Err(StoreError::Invalid(_))
    ));
    e.external_id = Some("x".into());
    assert_eq!(s.upsert_official_event(&e).await.unwrap(), e.id);
    s.set_event_embedding(e.id, &[1.0, 2.5]).await.unwrap();
    let mut changed = event(77, "Talk v2", 6, Some(10));
    changed.kind = EventKind::Official;
    changed.external_id = Some("x".into());
    assert_eq!(s.upsert_official_event(&changed).await.unwrap(), e.id);
    let got = s.get_event(e.id).await.unwrap().unwrap();
    assert_eq!(got.title, "Talk v2");
    assert_eq!(got.capacity, Some(10));
    assert_eq!(
        s.list_events(&EventFilter::default()).await.unwrap().len(),
        1
    );
    assert_eq!(
        s.events_missing_embedding().await.unwrap().len(),
        1,
        "changed content invalidates embedding"
    );
}

#[tokio::test]
async fn embeddings_roundtrip_through_snapshot() {
    let s = db().await;
    s.insert_event(&event(1, "A", 1, None)).await.unwrap();
    s.insert_event(&event(2, "B", 2, None)).await.unwrap();
    assert_eq!(s.events_missing_embedding().await.unwrap().len(), 2);
    s.set_event_embedding(EventId::from_u128(1), &[0.5, -1.0, 3.25])
        .await
        .unwrap();
    assert_eq!(s.events_missing_embedding().await.unwrap().len(), 1);
    assert!(matches!(
        s.set_event_embedding(EventId::from_u128(9), &[1.0]).await,
        Err(StoreError::NotFound)
    ));
    let snap = s.load_snapshot().await.unwrap();
    assert_eq!(snap.events.len(), 2);
    assert_eq!(
        snap.event_embeddings[&EventId::from_u128(1)],
        vec![0.5, -1.0, 3.25]
    );
    assert!(!snap.event_embeddings.contains_key(&EventId::from_u128(2)));
}

#[tokio::test]
async fn waitlist_and_auto_promote() {
    let s = db().await;
    with_users(&s, 4).await;
    let e = EventId::from_u128(1);
    s.insert_event(&event(1, "Small", 1, Some(2)))
        .await
        .unwrap();
    let u = |n| UserId::from_u128(n);
    let at = |m| t0() + TimeDelta::minutes(m);

    assert_eq!(
        s.set_intent(u(1), e, IntentLevel::Going, at(1))
            .await
            .unwrap()
            .status,
        IntentStatus::Confirmed
    );
    assert_eq!(
        s.set_intent(u(2), e, IntentLevel::GoingWith(u(4)), at(2))
            .await
            .unwrap()
            .status,
        IntentStatus::Confirmed
    );
    // interested never takes a spot
    assert_eq!(
        s.set_intent(u(4), e, IntentLevel::Interested, at(3))
            .await
            .unwrap()
            .status,
        IntentStatus::Confirmed
    );
    let w3 = s
        .set_intent(u(3), e, IntentLevel::Going, at(4))
        .await
        .unwrap();
    assert_eq!(w3.status, IntentStatus::Waitlisted);
    assert_eq!(s.waitlist_position(u(3), e).await.unwrap(), Some(1));
    assert_eq!(s.waitlist_position(u(1), e).await.unwrap(), None);

    // re-asserting keeps created_at and own spot
    let again = s
        .set_intent(u(1), e, IntentLevel::Going, at(10))
        .await
        .unwrap();
    assert_eq!(again.status, IntentStatus::Confirmed);
    assert_eq!(again.created_at, at(1));
    assert_eq!(again.updated_at, at(10));

    // interested user does not promote; back out of confirmed does
    let out = s.remove_intent(u(4), e, at(11)).await.unwrap();
    assert!(out.removed && out.promoted.is_none());
    let out = s.remove_intent(u(1), e, at(12)).await.unwrap();
    assert_eq!(out.promoted, Some(u(3)));
    let p = s.get_intent(u(3), e).await.unwrap().unwrap();
    assert_eq!(p.status, IntentStatus::Confirmed);
    assert_eq!(p.updated_at, at(12));
    assert_eq!(s.waitlist_position(u(3), e).await.unwrap(), None);
    assert!(!s.remove_intent(u(1), e, at(13)).await.unwrap().removed);
    assert!(matches!(
        s.set_intent(u(1), EventId::from_u128(9), IntentLevel::Going, at(1))
            .await,
        Err(StoreError::NotFound)
    ));
    assert_eq!(s.list_intents_for_event(e).await.unwrap().len(), 2);
    assert_eq!(s.list_intents_for_user(u(2)).await.unwrap().len(), 1);
}

#[tokio::test]
async fn downgrade_frees_spot_and_going_with_roundtrips() {
    let s = db().await;
    with_users(&s, 3).await;
    let e = EventId::from_u128(1);
    s.insert_event(&event(1, "One spot", 1, Some(1)))
        .await
        .unwrap();
    let u = |n| UserId::from_u128(n);
    s.set_intent(u(1), e, IntentLevel::GoingWith(u(3)), t0())
        .await
        .unwrap();
    let w = s
        .set_intent(u(2), e, IntentLevel::Going, t0() + TimeDelta::minutes(1))
        .await
        .unwrap();
    assert_eq!(w.status, IntentStatus::Waitlisted);
    s.set_intent(
        u(1),
        e,
        IntentLevel::Interested,
        t0() + TimeDelta::minutes(2),
    )
    .await
    .unwrap();
    assert_eq!(
        s.get_intent(u(2), e).await.unwrap().unwrap().status,
        IntentStatus::Confirmed
    );
    assert_eq!(
        s.get_intent(u(1), e).await.unwrap().unwrap().level,
        IntentLevel::Interested
    );
    // companion survives a round trip
    s.set_intent(u(3), e, IntentLevel::Interested, t0())
        .await
        .unwrap();
    let uncapped = EventId::from_u128(2);
    s.insert_event(&event(2, "Open", 1, None)).await.unwrap();
    let i = s
        .set_intent(u(1), uncapped, IntentLevel::GoingWith(u(2)), t0())
        .await
        .unwrap();
    assert_eq!(i.level, IntentLevel::GoingWith(u(2)));
    assert_eq!(s.get_intent(u(1), uncapped).await.unwrap().unwrap(), i);
}

#[tokio::test]
async fn friendships() {
    let s = db().await;
    with_users(&s, 3).await;
    let u = |n| UserId::from_u128(n);
    assert!(matches!(
        s.request_friend(u(1), u(1), t0()).await,
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        s.request_friend(u(1), u(9), t0()).await,
        Err(StoreError::NotFound)
    ));
    let f = s.request_friend(u(1), u(2), t0()).await.unwrap();
    assert_eq!(f.status, FriendshipStatus::Pending);
    assert!(s.friend_ids(u(1)).await.unwrap().is_empty());
    // repeating is idempotent
    assert_eq!(
        s.request_friend(u(1), u(2), t0()).await.unwrap().status,
        FriendshipStatus::Pending
    );
    // wrong direction accept fails
    assert!(matches!(
        s.accept_friend(u(1), u(2), t0()).await,
        Err(StoreError::NotFound)
    ));
    // reciprocal request accepts
    let f = s.request_friend(u(2), u(1), t0()).await.unwrap();
    assert_eq!(f.status, FriendshipStatus::Accepted);
    assert_eq!(s.friend_ids(u(1)).await.unwrap(), vec![u(2)]);
    assert_eq!(s.friend_ids(u(2)).await.unwrap(), vec![u(1)]);
    s.request_friend(u(3), u(1), t0()).await.unwrap();
    assert_eq!(s.list_friendships(u(1)).await.unwrap().len(), 2);
    let acc: Friendship = s.accept_friend(u(1), u(3), t0()).await.unwrap();
    assert_eq!(acc.status, FriendshipStatus::Accepted);
    assert!(s.remove_friend(u(2), u(1)).await.unwrap());
    assert!(!s.remove_friend(u(2), u(1)).await.unwrap());
    assert_eq!(s.friend_ids(u(1)).await.unwrap(), vec![u(3)]);
}

#[tokio::test]
async fn crews() {
    let s = db().await;
    with_users(&s, 2).await;
    let crew = Crew {
        id: CrewId::from_u128(1),
        name: "Smash".into(),
        description: "d".into(),
        created_by: UserId::from_u128(1),
        created_at: t0(),
    };
    s.insert_crew(&crew).await.unwrap();
    assert!(matches!(
        s.insert_crew(&crew).await,
        Err(StoreError::Conflict(_))
    ));
    assert_eq!(s.get_crew(crew.id).await.unwrap(), Some(crew.clone()));
    let members = s.list_crew_members(crew.id).await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].role, CrewRole::Owner);
    s.add_crew_member(
        crew.id,
        UserId::from_u128(2),
        CrewRole::Member,
        t0() + TimeDelta::hours(1),
    )
    .await
    .unwrap();
    s.add_crew_member(
        crew.id,
        UserId::from_u128(2),
        CrewRole::Member,
        t0() + TimeDelta::hours(2),
    )
    .await
    .unwrap();
    let members = s.list_crew_members(crew.id).await.unwrap();
    assert_eq!(members.len(), 2);
    assert_eq!(members[1].joined_at, t0() + TimeDelta::hours(1));
    assert_eq!(
        s.list_crews_for_user(UserId::from_u128(2))
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(matches!(
        s.add_crew_member(
            CrewId::from_u128(9),
            UserId::from_u128(2),
            CrewRole::Member,
            t0()
        )
        .await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        s.add_crew_member(crew.id, UserId::from_u128(9), CrewRole::Member, t0())
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(
        s.remove_crew_member(crew.id, UserId::from_u128(2))
            .await
            .unwrap()
    );
    assert!(
        !s.remove_crew_member(crew.id, UserId::from_u128(2))
            .await
            .unwrap()
    );
    assert!(
        s.list_crews_for_user(UserId::from_u128(2))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn buddy_and_checkins() {
    let s = db().await;
    with_users(&s, 2).await;
    s.insert_event(&event(1, "E", 1, None)).await.unwrap();
    let (a, b, e) = (
        UserId::from_u128(1),
        UserId::from_u128(2),
        EventId::from_u128(1),
    );
    assert!(!s.is_buddy_looking(a, e).await.unwrap());
    s.set_buddy_looking(b, e, true, t0() + TimeDelta::minutes(5))
        .await
        .unwrap();
    s.set_buddy_looking(a, e, true, t0()).await.unwrap();
    s.set_buddy_looking(a, e, true, t0() + TimeDelta::minutes(9))
        .await
        .unwrap();
    assert!(s.is_buddy_looking(a, e).await.unwrap());
    assert_eq!(s.list_buddy_seekers(e).await.unwrap(), vec![a, b]);
    s.set_buddy_looking(a, e, false, t0()).await.unwrap();
    assert_eq!(s.list_buddy_seekers(e).await.unwrap(), vec![b]);

    let c = CheckIn {
        user_id: a,
        event_id: e,
        at: t0(),
    };
    assert!(!s.has_checked_in(a, e).await.unwrap());
    assert!(s.record_checkin(&c).await.unwrap());
    assert!(!s.record_checkin(&c).await.unwrap());
    assert!(s.has_checked_in(a, e).await.unwrap());
}

#[tokio::test]
async fn availability_gear_presence() {
    let s = db().await;
    with_users(&s, 2).await;
    let (a, b) = (UserId::from_u128(1), UserId::from_u128(2));
    let day = hangouts_core::time::parse_date("2026-11-21").unwrap();
    let w = |from: &str, to: &str| {
        hangouts_core::time::parse_local_window("2026-11-21", Some(from), Some(to)).unwrap()
    };
    s.set_availability(&Availability {
        user_id: a,
        window: w("10:00", "12:00"),
    })
    .await
    .unwrap();
    s.set_availability(&Availability {
        user_id: a,
        window: w("14:00", "16:00"),
    })
    .await
    .unwrap();
    s.set_availability(&Availability {
        user_id: b,
        window: w("09:00", "11:00"),
    })
    .await
    .unwrap();
    let on = s.list_availability_on(day, None).await.unwrap();
    assert_eq!(on.len(), 2, "same-date window replaces the previous one");
    assert_eq!(
        on.iter().find(|x| x.user_id == a).unwrap().window,
        w("14:00", "16:00")
    );
    assert_eq!(
        s.list_availability_on(day, Some(&[b])).await.unwrap().len(),
        1
    );
    assert!(
        s.list_availability_on(day.succ_opt().unwrap().succ_opt().unwrap(), None)
            .await
            .unwrap()
            .is_empty()
    );

    let g = Gear::new(a, Setup::Console, "Smash").unwrap();
    s.add_gear(&g).await.unwrap();
    s.add_gear(&g).await.unwrap();
    assert_eq!(s.list_gear().await.unwrap(), vec![g]);

    s.set_presence(a, Presence::Here, t0()).await.unwrap();
    s.set_presence(b, Presence::Interested, t0() + TimeDelta::minutes(1))
        .await
        .unwrap();
    let rows = s.list_presence().await.unwrap();
    assert_eq!(
        rows.iter().map(|r| r.user_id).collect::<Vec<_>>(),
        vec![b, a]
    );
    s.set_presence(b, Presence::Out, t0() + TimeDelta::minutes(2))
        .await
        .unwrap();
    assert_eq!(s.list_presence().await.unwrap().len(), 1);
}

#[tokio::test]
async fn delete_history_keeps_account_and_social() {
    let s = db().await;
    with_users(&s, 2).await;
    let (a, b, e) = (
        UserId::from_u128(1),
        UserId::from_u128(2),
        EventId::from_u128(1),
    );
    s.insert_event(&event(1, "E", 1, None)).await.unwrap();
    s.set_intent(a, e, IntentLevel::Going, t0()).await.unwrap();
    s.set_buddy_looking(a, e, true, t0()).await.unwrap();
    s.record_checkin(&CheckIn {
        user_id: a,
        event_id: e,
        at: t0(),
    })
    .await
    .unwrap();
    s.request_friend(a, b, t0()).await.unwrap();
    s.delete_history(a).await.unwrap();
    let snap = s.load_snapshot().await.unwrap();
    assert!(snap.intents.is_empty() && snap.checkins.is_empty() && snap.buddy_requests.is_empty());
    assert_eq!(snap.users.len(), 2);
    assert_eq!(snap.friendships.len(), 1);
}
