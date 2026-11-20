//! Deterministic demo data: users with personas, crews, friendships, and weeks of intent and
//! check-in history with realistic no-shows. Same config ⇒ same data, byte for byte.
//!
//! [`generate`] is pure (used by `eval` without a database); [`write`] persists a snapshot.
//!
//! How behaviour is simulated (so the recommender has real signal to find):
//! - each student has a primary and a secondary [persona](catalog::PERSONAS); interest tags
//!   come from both, and the chance of RSVPing to an event grows with tag overlap;
//! - crew members are much more likely to RSVP to their crew's hangouts, and students are
//!   more likely to join events their friends are already going to;
//! - each student has a personal reliability; check-ins depend on it, on the intent level
//!   (Interested rarely shows), and on friends going — which gives no-shows to learn from;
//! - RSVPs are made 0.5–7 days ahead, so upcoming events already have some intents.

mod catalog;

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, TimeDelta, Utc};
use hangouts_core::time::{local_date, resolve_local};
use hangouts_core::{
    BuddyRequest, CheckIn, Crew, CrewId, CrewMember, CrewRole, Event, EventId, EventKind,
    Friendship, FriendshipStatus, Intent, IntentLevel, IntentStatus, Location, PrivacySettings,
    Snapshot, User, UserId, admission, normalize_tags,
};
use rand::seq::{IndexedRandom, SliceRandom};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use store::{Store, StoreError};

use catalog::{CREWS, FIRST_NAMES, LAST_INITIALS, ONE_OFF, PERSONAS, Template, WEEKLY};

/// The presenter's demo persona (first user, first on the login screen).
pub const DEMO_USER: UserId = UserId::from_u128(USER_BASE);

/// How many users are flagged `is_demo` (selectable on the demo login screen).
pub const DEMO_LOGIN_USERS: usize = 8;

/// Weeks of upcoming events generated after `now`.
pub const FUTURE_WEEKS: u32 = 2;

/// One-off events added per week on top of the weekly ones.
pub const ONE_OFFS_PER_WEEK: usize = 8;

const USER_BASE: u128 = 0x5eed_0000_0000_0000_0000_0001_0000_0000;
const EVENT_BASE: u128 = 0x5eed_0000_0000_0000_0000_0002_0000_0000;
const CREW_BASE: u128 = 0x5eed_0000_0000_0000_0000_0003_0000_0000;

/// Generation settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedConfig {
    /// RNG seed (fixed for reproducibility).
    pub rng_seed: u64,
    /// Number of demo users.
    pub users: usize,
    /// Number of crews.
    pub crews: usize,
    /// Weeks of past history before `now`.
    pub weeks_of_history: u32,
    /// The instant history is generated relative to.
    pub now: DateTime<Utc>,
}

impl SeedConfig {
    /// The demo defaults (60 users, 10 crews, 4 weeks) relative to `now`.
    pub fn demo(now: DateTime<Utc>) -> Self {
        Self {
            rng_seed: 2026,
            users: 60,
            crews: 10,
            weeks_of_history: 4,
            now,
        }
    }
}

/// Seeded user `n` (0-based); `user_id(0)` is [`DEMO_USER`].
pub const fn user_id(n: usize) -> UserId {
    UserId::from_u128(USER_BASE + n as u128)
}

/// Seeded event `n` (0-based, in generation order).
pub const fn event_id(n: usize) -> EventId {
    EventId::from_u128(EVENT_BASE + n as u128)
}

/// Seeded crew `n` (0-based).
pub const fn crew_id(n: usize) -> CrewId {
    CrewId::from_u128(CREW_BASE + n as u128)
}

/// Per-user simulation state that is not part of the domain model.
struct Sim {
    primary: usize,
    secondary: usize,
    /// Probability-ish multiplier for showing up (0.45..0.97).
    reliability: f64,
    /// How often this student RSVPs at all (0.6..1.4).
    activity: f64,
}

/// Generates the demo dataset.
pub fn generate(config: &SeedConfig) -> Snapshot {
    let mut rng = ChaCha8Rng::seed_from_u64(config.rng_seed);
    let now = config.now;
    let history_start = now - TimeDelta::weeks(i64::from(config.weeks_of_history));
    let account_created = history_start - TimeDelta::weeks(1);

    let (users, sims) = gen_users(&mut rng, config.users, account_created);
    let (crews, crew_members) = gen_crews(&mut rng, config.crews, &sims, account_created);
    let friendships = gen_friendships(&mut rng, &users, &crew_members, account_created);
    let events = gen_events(&mut rng, config, &users, &crew_members, history_start);

    let mut snap = Snapshot {
        users,
        events,
        friendships,
        crews,
        crew_members,
        ..Snapshot::default()
    };
    gen_activity(&mut rng, &mut snap, &sims, now);
    snap
}

fn gen_users(rng: &mut ChaCha8Rng, n: usize, at: DateTime<Utc>) -> (Vec<User>, Vec<Sim>) {
    let mut users = Vec::with_capacity(n);
    let mut sims = Vec::with_capacity(n);
    let initials: Vec<char> = LAST_INITIALS.chars().collect();
    for i in 0..n {
        let (name, primary, secondary, interests, bio) = if i == 0 {
            // The presenter: gamer + coder who also likes board games.
            (
                "Casey (demo)".to_owned(),
                0,
                2,
                normalize_tags(&["gaming", "smash-bros", "coding", "ai", "board-games"]),
                "CS student. Smash on weekends, hackathons when I can.".to_owned(),
            )
        } else {
            let primary = i % PERSONAS.len();
            let mut secondary = rng.random_range(0..PERSONAS.len());
            if secondary == primary {
                secondary =
                    (secondary + 1 + rng.random_range(0..PERSONAS.len() - 1)) % PERSONAS.len();
            }
            let mut tags: Vec<&str> = PERSONAS[primary].tags.to_vec();
            tags.shuffle(rng);
            tags.truncate(rng.random_range(3..=4));
            let mut extra: Vec<&str> = PERSONAS[secondary].tags.to_vec();
            extra.shuffle(rng);
            tags.extend(extra.into_iter().take(rng.random_range(1..=2)));
            let first = FIRST_NAMES[(i - 1) % FIRST_NAMES.len()];
            let last = initials[(i * 7) % initials.len()];
            let bio = PERSONAS[primary]
                .bios
                .choose(rng)
                .copied()
                .unwrap_or_default()
                .to_owned();
            (
                format!("{first} {last}."),
                primary,
                secondary,
                normalize_tags(&tags),
                bio,
            )
        };
        users.push(User {
            id: user_id(i),
            display_name: name,
            bio: Some(bio),
            interests,
            discord_id: None,
            is_demo: i < DEMO_LOGIN_USERS,
            privacy: PrivacySettings::default(),
            created_at: at,
        });
        sims.push(Sim {
            primary,
            secondary,
            reliability: if i == 0 {
                0.9
            } else {
                rng.random_range(0.45..0.97)
            },
            activity: if i == 0 {
                1.2
            } else {
                rng.random_range(0.6..1.4)
            },
        });
    }
    (users, sims)
}

fn gen_crews(
    rng: &mut ChaCha8Rng,
    n: usize,
    sims: &[Sim],
    at: DateTime<Utc>,
) -> (Vec<Crew>, Vec<CrewMember>) {
    let mut crews = Vec::new();
    let mut members = Vec::new();
    for (c, (name, description, persona)) in CREWS.iter().take(n).enumerate() {
        // Mostly people with this primary persona, plus a couple who have it as secondary.
        let mut primary: Vec<usize> = (0..sims.len())
            .filter(|u| sims[*u].primary == *persona)
            .collect();
        let mut secondary: Vec<usize> = (0..sims.len())
            .filter(|u| sims[*u].secondary == *persona && sims[*u].primary != *persona)
            .collect();
        primary.shuffle(rng);
        secondary.shuffle(rng);
        let mut chosen: Vec<usize> = primary.into_iter().take(5).collect();
        chosen.extend(secondary.into_iter().take(rng.random_range(1..=3)));
        // The demo user is in the gaming and hacking crews.
        if (*persona == 0 || *persona == 2) && !chosen.contains(&0) {
            chosen.insert(0, 0);
        }
        let Some(&owner) = chosen.first() else {
            continue;
        };
        let crew = crew_id(c);
        crews.push(Crew {
            id: crew,
            name: (*name).to_owned(),
            description: (*description).to_owned(),
            created_by: user_id(owner),
            created_at: at,
        });
        for (k, u) in chosen.iter().enumerate() {
            members.push(CrewMember {
                crew_id: crew,
                user_id: user_id(*u),
                role: if k == 0 {
                    CrewRole::Owner
                } else {
                    CrewRole::Member
                },
                joined_at: at + TimeDelta::hours(k as i64),
            });
        }
    }
    (crews, members)
}

fn gen_friendships(
    rng: &mut ChaCha8Rng,
    users: &[User],
    crew_members: &[CrewMember],
    at: DateTime<Utc>,
) -> Vec<Friendship> {
    let mut pairs: BTreeMap<(UserId, UserId), Friendship> = BTreeMap::new();
    let mut add = |a: UserId, b: UserId, accepted: bool, offset: i64| {
        if a == b {
            return;
        }
        let key = if a < b { (a, b) } else { (b, a) };
        pairs.entry(key).or_insert(Friendship {
            requester: a,
            addressee: b,
            status: if accepted {
                FriendshipStatus::Accepted
            } else {
                FriendshipStatus::Pending
            },
            created_at: at + TimeDelta::minutes(offset),
        });
    };
    let mut by_crew: BTreeMap<CrewId, Vec<UserId>> = BTreeMap::new();
    for m in crew_members {
        by_crew.entry(m.crew_id).or_default().push(m.user_id);
    }
    let mut offset = 0;
    for members in by_crew.values() {
        for (i, a) in members.iter().enumerate() {
            for b in &members[i + 1..] {
                offset += 1;
                if rng.random_bool(0.55) {
                    add(*a, *b, true, offset);
                }
            }
        }
    }
    for u in users {
        for _ in 0..2 {
            let Some(other) = users.choose(rng) else {
                continue;
            };
            offset += 1;
            let accepted = rng.random_bool(0.85);
            add(u.id, other.id, accepted, offset);
        }
    }
    pairs.into_values().collect()
}

fn gen_events(
    rng: &mut ChaCha8Rng,
    config: &SeedConfig,
    users: &[User],
    crew_members: &[CrewMember],
    history_start: DateTime<Utc>,
) -> Vec<Event> {
    let first_day = local_date(history_start);
    let monday = first_day - TimeDelta::days(i64::from(first_day.weekday().num_days_from_monday()));
    let end = config.now + TimeDelta::weeks(i64::from(FUTURE_WEEKS));
    let mut events = Vec::new();
    let weeks = config.weeks_of_history + FUTURE_WEEKS + 1;
    for w in 0..weeks {
        let week_monday = monday + TimeDelta::weeks(i64::from(w));
        let mut templates: Vec<&Template> = WEEKLY.iter().collect();
        let mut picks: Vec<&Template> = ONE_OFF.iter().collect();
        picks.shuffle(rng);
        templates.extend(picks.into_iter().take(ONE_OFFS_PER_WEEK));
        for tpl in templates {
            let day = week_monday + TimeDelta::days(i64::from(tpl.weekday));
            let starts_at = local_at(day, tpl.hour);
            if starts_at < history_start || starts_at >= end {
                continue;
            }
            let n = events.len();
            events.push(build_event(
                rng,
                tpl,
                event_id(n),
                n,
                starts_at,
                users,
                crew_members,
            ));
        }
    }
    events.sort_by_key(|e| (e.starts_at, e.id));
    events
}

fn local_at(day: NaiveDate, hour: u32) -> DateTime<Utc> {
    resolve_local(day.and_time(NaiveTime::from_hms_opt(hour, 0, 0).unwrap_or(NaiveTime::MIN)))
}

fn build_event(
    rng: &mut ChaCha8Rng,
    tpl: &Template,
    id: EventId,
    n: usize,
    starts_at: DateTime<Utc>,
    users: &[User],
    crew_members: &[CrewMember],
) -> Event {
    let crew = (!tpl.official && tpl.persona < CREWS.len()).then(|| crew_id(tpl.persona));
    let crew_users: Vec<UserId> = crew_members
        .iter()
        .filter(|m| Some(m.crew_id) == crew)
        .map(|m| m.user_id)
        .collect();
    let host = if tpl.official {
        None
    } else {
        crew_users
            .choose(rng)
            .copied()
            .or_else(|| users.choose(rng).map(|u| u.id))
    };
    Event {
        id,
        kind: if tpl.official {
            EventKind::Official
        } else {
            EventKind::Hangout
        },
        title: tpl.title.to_owned(),
        description: tpl.description.to_owned(),
        tags: normalize_tags(tpl.tags),
        location: Location {
            name: tpl.venue.to_owned(),
            room: tpl.room.map(str::to_owned),
            ..Location::default()
        },
        starts_at,
        ends_at: starts_at + TimeDelta::hours(tpl.hours),
        capacity: if tpl.official { None } else { tpl.capacity },
        host_id: host,
        crew_id: crew.filter(|_| host.is_some_and(|h| crew_users.contains(&h))),
        external_id: tpl.official.then(|| format!("seed:{n}")),
        url: None,
        image_url: None,
        organizer: tpl.official.then(|| "Kennesaw State University".to_owned()),
        created_at: starts_at - TimeDelta::days(10),
    }
}

/// Interest overlap in `0..=1`: share of the event's tags the user likes, with tags from the
/// user's personas counting half when not in their explicit interests.
fn affinity(user: &User, sim: &Sim, event: &Event) -> f64 {
    if event.tags.is_empty() {
        return 0.0;
    }
    let persona_tags: BTreeSet<&str> = PERSONAS[sim.primary]
        .tags
        .iter()
        .chain(PERSONAS[sim.secondary].tags)
        .copied()
        .collect();
    let score: f64 = event
        .tags
        .iter()
        .map(|t| {
            if user.interests.contains(t) {
                1.0
            } else if persona_tags.contains(t.as_str()) {
                0.5
            } else {
                0.0
            }
        })
        .sum();
    score / event.tags.len() as f64
}

fn gen_activity(rng: &mut ChaCha8Rng, snap: &mut Snapshot, sims: &[Sim], now: DateTime<Utc>) {
    let friends: Vec<BTreeSet<UserId>> = snap.users.iter().map(|u| snap.friends_of(u.id)).collect();
    let index: BTreeMap<UserId, usize> = snap
        .users
        .iter()
        .enumerate()
        .map(|(i, u)| (u.id, i))
        .collect();
    let crew_of: BTreeMap<CrewId, BTreeSet<UserId>> =
        snap.crew_members
            .iter()
            .fold(BTreeMap::new(), |mut acc, m| {
                acc.entry(m.crew_id).or_default().insert(m.user_id);
                acc
            });

    let mut intents = Vec::new();
    let mut checkins = Vec::new();
    let mut buddy = Vec::new();
    for event in &snap.events {
        let crew = event.crew_id.and_then(|c| crew_of.get(&c));
        // (user index, level, created_at)
        let mut rsvps: Vec<(usize, IntentLevel, DateTime<Utc>)> = Vec::new();
        let mut going: BTreeSet<UserId> = BTreeSet::new();
        if let Some((host_id, host)) = event.host_id.and_then(|h| index.get(&h).map(|i| (h, *i))) {
            rsvps.push((host, IntentLevel::Going, event.created_at));
            going.insert(host_id);
        }
        // Pass 1: interest-driven RSVPs. Pass 2: friends pull friends in.
        for pass in 0..2 {
            let mut order: Vec<usize> = (0..snap.users.len()).collect();
            order.shuffle(rng);
            for u in order {
                if rsvps.iter().any(|(x, ..)| *x == u) {
                    continue;
                }
                let user = &snap.users[u];
                let sim = &sims[u];
                let in_crew = crew.is_some_and(|c| c.contains(&user.id));
                let friends_going = friends[u].intersection(&going).count();
                let p = if pass == 0 {
                    let aff = affinity(user, sim, event);
                    (0.01 + 0.30 * aff * aff + if in_crew { 0.30 } else { 0.0 }) * sim.activity
                } else {
                    (0.07 * friends_going.min(3) as f64) * sim.activity
                };
                if !rng.random_bool(p.clamp(0.0, 0.95)) {
                    continue;
                }
                let lead = TimeDelta::minutes(rng.random_range(30..7 * 24 * 60));
                let created = event.starts_at - lead;
                let companion = friends[u].intersection(&going).copied().collect::<Vec<_>>();
                let level = if !companion.is_empty() && rng.random_bool(0.3) {
                    IntentLevel::GoingWith(companion[rng.random_range(0..companion.len())])
                } else if rng.random_bool(if in_crew { 0.8 } else { 0.55 }) {
                    IntentLevel::Going
                } else {
                    IntentLevel::Interested
                };
                if level.uses_spot() {
                    going.insert(user.id);
                }
                rsvps.push((u, level, created));
            }
        }
        // Only RSVPs made before "now" exist.
        rsvps.retain(|(_, _, created)| *created < now);
        rsvps.sort_by_key(|(u, _, created)| (*created, *u));

        let mut taken = 0_u32;
        let mut confirmed_going: BTreeSet<UserId> = BTreeSet::new();
        for (u, level, created) in &rsvps {
            let status = admission(event.capacity, taken, level);
            if status == IntentStatus::Confirmed && level.uses_spot() {
                taken += 1;
                confirmed_going.insert(snap.users[*u].id);
            }
            intents.push(Intent {
                user_id: snap.users[*u].id,
                event_id: event.id,
                level: *level,
                status,
                created_at: *created,
                updated_at: *created,
            });
        }

        if event.starts_at < now {
            for (u, level, _) in &rsvps {
                let uid = snap.users[*u].id;
                let friends_there = friends[*u].intersection(&confirmed_going).count();
                let base = match level {
                    IntentLevel::Interested => 0.25,
                    IntentLevel::Going => 0.80,
                    IntentLevel::GoingWith(_) => 0.90,
                };
                let waitlisted = !confirmed_going.contains(&uid) && level.uses_spot();
                let p = if waitlisted {
                    0.05
                } else {
                    base * sims[*u].reliability + 0.05 * friends_there.min(3) as f64
                };
                if event.host_id == Some(uid) || rng.random_bool(p.clamp(0.0, 0.98)) {
                    let at = event.starts_at + TimeDelta::minutes(rng.random_range(0..30));
                    checkins.push(CheckIn {
                        user_id: uid,
                        event_id: event.id,
                        at,
                    });
                }
            }
        } else if event.starts_at < now + TimeDelta::weeks(1) {
            for (u, level, created) in &rsvps {
                let force = *u == 0 && buddy.is_empty();
                if *level == IntentLevel::Interested && (force || rng.random_bool(0.2)) {
                    buddy.push(BuddyRequest {
                        user_id: snap.users[*u].id,
                        event_id: event.id,
                        created_at: *created,
                    });
                }
            }
        }
    }
    snap.intents = intents;
    snap.checkins = checkins;
    snap.buddy_requests = buddy;
}

/// Writes a generated snapshot through the store (users, events, friendships, crews,
/// intents, check-ins, buddy requests). Intents are written in `created_at` order so the
/// store's waitlist decisions match the generated ones.
pub async fn write(store: &Store, snapshot: &Snapshot) -> Result<(), StoreError> {
    for user in &snapshot.users {
        store.insert_user(user).await?;
    }
    // Crews before events: crew hangouts reference crews(id).
    for crew in &snapshot.crews {
        store.insert_crew(crew).await?;
    }
    for m in &snapshot.crew_members {
        store
            .add_crew_member(m.crew_id, m.user_id, m.role, m.joined_at)
            .await?;
    }
    for event in &snapshot.events {
        store.insert_event(event).await?;
    }
    for (id, embedding) in &snapshot.event_embeddings {
        store.set_event_embedding(*id, embedding).await?;
    }
    for f in &snapshot.friendships {
        store
            .request_friend(f.requester, f.addressee, f.created_at)
            .await?;
        if f.status == FriendshipStatus::Accepted {
            store
                .accept_friend(f.addressee, f.requester, f.created_at)
                .await?;
        }
    }
    let mut intents: Vec<&Intent> = snapshot.intents.iter().collect();
    intents.sort_by_key(|i| (i.created_at, i.user_id, i.event_id));
    for i in intents {
        store
            .set_intent(i.user_id, i.event_id, i.level, i.created_at)
            .await?;
    }
    for c in &snapshot.checkins {
        store.record_checkin(c).await?;
    }
    for b in &snapshot.buddy_requests {
        store
            .set_buddy_looking(b.user_id, b.event_id, true, b.created_at)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hangouts_core::IntentKind;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-19T17:00:00Z")
            .unwrap()
            .to_utc()
    }

    fn snap() -> Snapshot {
        generate(&SeedConfig::demo(now()))
    }

    #[test]
    fn is_deterministic() {
        assert_eq!(snap(), snap());
        let other = generate(&SeedConfig {
            rng_seed: 7,
            ..SeedConfig::demo(now())
        });
        assert_ne!(snap().intents, other.intents);
    }

    #[test]
    fn has_the_configured_shape() {
        let s = snap();
        assert_eq!(s.users.len(), 60);
        assert_eq!(s.crews.len(), 10);
        assert_eq!(s.users[0].id, DEMO_USER);
        assert_eq!(
            s.users.iter().filter(|u| u.is_demo).count(),
            DEMO_LOGIN_USERS
        );
        assert!(s.users.iter().all(|u| u.interests.len() >= 3));
        let mut names: Vec<_> = s.users.iter().map(|u| &u.display_name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 60, "display names are unique");
        // ~4 weeks of history + 2 upcoming weeks of events.
        let past = s.events.iter().filter(|e| e.starts_at < now()).count();
        let future = s.events.len() - past;
        assert!((60..=110).contains(&past), "past events: {past}");
        assert!(future >= 20, "future events: {future}");
        assert!(s.events.iter().any(|e| e.kind == EventKind::Official));
        assert!(s.events.iter().any(|e| e.kind == EventKind::Hangout));
    }

    #[test]
    fn everyone_has_friends_and_demo_user_is_social() {
        let s = snap();
        let lonely = s
            .users
            .iter()
            .filter(|u| s.friends_of(u.id).is_empty())
            .count();
        assert!(lonely <= 3, "{lonely} users without friends");
        assert!(s.friends_of(DEMO_USER).len() >= 4);
        assert!(
            s.crew_members
                .iter()
                .filter(|m| m.user_id == DEMO_USER)
                .count()
                >= 2
        );
        assert!(
            s.friendships
                .iter()
                .any(|f| f.status == FriendshipStatus::Pending)
        );
    }

    #[test]
    fn history_has_realistic_no_shows() {
        let s = snap();
        let checked: BTreeSet<(UserId, EventId)> =
            s.checkins.iter().map(|c| (c.user_id, c.event_id)).collect();
        let past: BTreeSet<EventId> = s
            .events
            .iter()
            .filter(|e| e.starts_at < now())
            .map(|e| e.id)
            .collect();
        let rate = |kind: IntentKind| {
            let rel: Vec<_> = s
                .intents
                .iter()
                .filter(|i| past.contains(&i.event_id) && i.level.kind() == kind)
                .collect();
            let shows = rel
                .iter()
                .filter(|i| checked.contains(&(i.user_id, i.event_id)))
                .count();
            (shows as f64 / rel.len().max(1) as f64, rel.len())
        };
        let (going, n_going) = rate(IntentKind::Going);
        let (interested, n_interested) = rate(IntentKind::Interested);
        assert!(
            n_going > 200 && n_interested > 100,
            "{n_going} {n_interested}"
        );
        assert!((0.5..0.9).contains(&going), "going show rate {going}");
        assert!(interested < 0.35, "interested show rate {interested}");
        // Every check-in is for a past event and backed by an intent.
        assert!(s.checkins.iter().all(|c| past.contains(&c.event_id)));
        assert!(s.checkins.iter().all(|c| {
            s.intents
                .iter()
                .any(|i| i.user_id == c.user_id && i.event_id == c.event_id)
        }));
    }

    #[test]
    fn intents_are_consistent() {
        let s = snap();
        let mut pairs = BTreeSet::new();
        for i in &s.intents {
            assert!(pairs.insert((i.user_id, i.event_id)), "one intent per pair");
            assert!(i.created_at < now());
            let e = s.event(i.event_id).unwrap();
            assert!(i.created_at <= e.starts_at);
            if let IntentLevel::GoingWith(w) = i.level {
                assert!(s.friends_of(i.user_id).contains(&w));
            }
        }
        for e in &s.events {
            if let Some(cap) = e.capacity {
                let confirmed = s
                    .intents_for_event(e.id)
                    .filter(|i| i.status == IntentStatus::Confirmed && i.level.uses_spot())
                    .count();
                assert!(confirmed <= cap as usize, "{} over capacity", e.title);
            }
        }
        // Upcoming events already have RSVPs, and the demo user is looking for a buddy.
        assert!(
            s.intents
                .iter()
                .any(|i| s.event(i.event_id).unwrap().starts_at > now())
        );
        assert!(s.buddy_requests.iter().any(|b| b.user_id == DEMO_USER));
    }

    #[test]
    fn interests_drive_attendance() {
        // Gamers RSVP to gaming events far more than athletes do.
        let s = snap();
        let gaming: BTreeSet<EventId> = s
            .events
            .iter()
            .filter(|e| e.tags.iter().any(|t| t == "gaming"))
            .map(|e| e.id)
            .collect();
        let share = |persona: usize| {
            let ids: BTreeSet<UserId> = (1..s.users.len())
                .filter(|i| i % PERSONAS.len() == persona)
                .map(user_id)
                .collect();
            let all = s
                .intents
                .iter()
                .filter(|i| ids.contains(&i.user_id))
                .count();
            let g = s
                .intents
                .iter()
                .filter(|i| ids.contains(&i.user_id) && gaming.contains(&i.event_id))
                .count();
            g as f64 / all.max(1) as f64
        };
        assert!(share(0) > 2.0 * share(3), "{} vs {}", share(0), share(3));
    }
}
