//! Building API view types from a [`Snapshot`]: event cards, intents, friend presence.
//!
//! Privacy (docs/API.md): no attendee lists or totals; friends are counted only when the
//! friendship is accepted and the friend shares attendance with friends.

use api::{EventCard, EventDetail, FriendPresence, MyIntent, UserSummary};
use hangouts_core::{Event, IntentLevel, IntentStatus, Snapshot, UserId, Visibility};

/// Spots taken by confirmed non-"interested" intents.
fn spots_taken(snap: &Snapshot, event: &Event) -> u32 {
    snap.intents_for_event(event.id)
        .filter(|i| i.status == IntentStatus::Confirmed && i.level.uses_spot())
        .count() as u32
}

fn summary(snap: &Snapshot, id: UserId) -> Option<UserSummary> {
    snap.user(id).map(UserSummary::from)
}

/// The viewer's intent on `event`, if any.
pub fn my_intent(snap: &Snapshot, event: &Event, viewer: Option<UserId>) -> Option<MyIntent> {
    let viewer = viewer?;
    let intent = snap
        .intents_for_event(event.id)
        .find(|i| i.user_id == viewer)?;
    let waitlist_position = (intent.status == IntentStatus::Waitlisted).then(|| {
        let mut waiting: Vec<_> = snap
            .intents_for_event(event.id)
            .filter(|i| i.status == IntentStatus::Waitlisted)
            .map(|i| (i.created_at, i.user_id))
            .collect();
        waiting.sort();
        waiting
            .iter()
            .position(|(_, u)| *u == viewer)
            .map_or(1, |p| p as u32 + 1)
    });
    Some(MyIntent {
        level: intent.level.kind(),
        with: intent.level.companion().and_then(|u| summary(snap, u)),
        status: intent.status,
        waitlist_position,
    })
}

/// Friends of the viewer who are going / interested, honouring their privacy.
pub fn friend_presence(snap: &Snapshot, event: &Event, viewer: Option<UserId>) -> FriendPresence {
    let Some(viewer) = viewer else {
        return FriendPresence::default();
    };
    let friends = snap.friends_of(viewer);
    let mut going = Vec::new();
    let mut interested = Vec::new();
    for intent in snap.intents_for_event(event.id) {
        if !friends.contains(&intent.user_id) {
            continue;
        }
        let Some(friend) = snap.user(intent.user_id) else {
            continue;
        };
        if friend.privacy.attendance != Visibility::Friends {
            continue;
        }
        // A waitlisted friend isn't going yet; count them as interested so cards never
        // say "going" for someone who holds no spot (and agree with `spots_left`).
        match (intent.level, intent.status) {
            (IntentLevel::Interested, _) | (_, IntentStatus::Waitlisted) => {
                interested.push(UserSummary::from(friend));
            }
            _ => going.push(UserSummary::from(friend)),
        }
    }
    let count = |v: &Vec<UserSummary>| v.len() as u32;
    FriendPresence {
        going: count(&going),
        interested: count(&interested),
        sample: going.into_iter().chain(interested).take(2).collect(),
    }
}

/// The card for `event` as seen by `viewer`.
pub fn card(snap: &Snapshot, event: &Event, viewer: Option<UserId>) -> EventCard {
    EventCard {
        id: event.id,
        kind: event.kind,
        title: event.title.clone(),
        starts_at: event.starts_at,
        ends_at: event.ends_at,
        location: event.location.clone(),
        tags: event.tags.clone(),
        capacity: event.capacity,
        spots_left: event
            .capacity
            .map(|c| c.saturating_sub(spots_taken(snap, event))),
        host: event.host_id.and_then(|h| summary(snap, h)),
        image_url: event.image_url.clone(),
        friends: friend_presence(snap, event, viewer),
        my_intent: my_intent(snap, event, viewer),
    }
}

/// The detail view for `event` as seen by `viewer`. The crew is shown to its members only.
pub fn detail(snap: &Snapshot, event: &Event, viewer: Option<UserId>) -> EventDetail {
    let crew = event.crew_id.and_then(|cid| {
        let viewer = viewer?;
        snap.crew_members
            .iter()
            .any(|m| m.crew_id == cid && m.user_id == viewer)
            .then(|| snap.crews.iter().find(|c| c.id == cid))
            .flatten()
            .map(|c| api::CrewSummary {
                id: c.id,
                name: c.name.clone(),
                description: c.description.clone(),
            })
    });
    let buddy_looking = viewer.is_some_and(|v| {
        snap.buddy_requests
            .iter()
            .any(|b| b.event_id == event.id && b.user_id == v)
    });
    EventDetail {
        card: card(snap, event, viewer),
        description: event.description.clone(),
        url: event.url.clone(),
        organizer: event.organizer.clone(),
        crew,
        buddy_looking,
        calendar_url: format!("/api/events/{}/calendar.ics", event.id),
    }
}
