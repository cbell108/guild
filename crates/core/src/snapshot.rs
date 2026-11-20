//! An in-memory copy of everything the recommender and evaluation need.

use std::collections::{BTreeMap, BTreeSet};

use crate::checkin::CheckIn;
use crate::event::Event;
use crate::ids::{EventId, UserId};
use crate::intent::Intent;
use crate::social::{BuddyRequest, Crew, CrewMember, Friendship, FriendshipStatus};
use crate::user::User;

/// All app data at one point in time.
///
/// Produced by `store::Store::load_snapshot` (live) and `seed::generate` (demo/eval), and
/// consumed by `recommend`. Hackathon scale (hundreds of users, thousands of events) fits in
/// memory comfortably.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    /// All users.
    pub users: Vec<User>,
    /// All events (past and future).
    pub events: Vec<Event>,
    /// All intents.
    pub intents: Vec<Intent>,
    /// All check-ins.
    pub checkins: Vec<CheckIn>,
    /// All friendships (pending and accepted).
    pub friendships: Vec<Friendship>,
    /// All crews.
    pub crews: Vec<Crew>,
    /// All crew memberships.
    pub crew_members: Vec<CrewMember>,
    /// All open "looking for a buddy" requests.
    pub buddy_requests: Vec<BuddyRequest>,
    /// Stored event embeddings (may be missing for some events).
    pub event_embeddings: BTreeMap<EventId, Vec<f32>>,
}

impl Snapshot {
    /// Looks up a user.
    pub fn user(&self, id: UserId) -> Option<&User> {
        self.users.iter().find(|u| u.id == id)
    }

    /// Looks up an event.
    pub fn event(&self, id: EventId) -> Option<&Event> {
        self.events.iter().find(|e| e.id == id)
    }

    /// Accepted friends of `user`.
    pub fn friends_of(&self, user: UserId) -> BTreeSet<UserId> {
        self.friendships
            .iter()
            .filter(|f| f.status == FriendshipStatus::Accepted)
            .filter_map(|f| f.other(user))
            .collect()
    }

    /// Intents for one event.
    pub fn intents_for_event(&self, event: EventId) -> impl Iterator<Item = &Intent> {
        self.intents.iter().filter(move |i| i.event_id == event)
    }

    /// Intents by one user.
    pub fn intents_for_user(&self, user: UserId) -> impl Iterator<Item = &Intent> {
        self.intents.iter().filter(move |i| i.user_id == user)
    }

    /// Check-ins by one user.
    pub fn checkins_for_user(&self, user: UserId) -> impl Iterator<Item = &CheckIn> {
        self.checkins.iter().filter(move |c| c.user_id == user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    #[test]
    fn friends_are_accepted_and_symmetric() {
        let (a, b, c) = (
            UserId::from_u128(1),
            UserId::from_u128(2),
            UserId::from_u128(3),
        );
        let at = DateTime::<Utc>::UNIX_EPOCH;
        let snap = Snapshot {
            friendships: vec![
                Friendship {
                    requester: a,
                    addressee: b,
                    status: FriendshipStatus::Accepted,
                    created_at: at,
                },
                Friendship {
                    requester: c,
                    addressee: a,
                    status: FriendshipStatus::Pending,
                    created_at: at,
                },
            ],
            ..Snapshot::default()
        };
        assert_eq!(snap.friends_of(a), BTreeSet::from([b]));
        assert_eq!(snap.friends_of(b), BTreeSet::from([a]));
        assert!(snap.friends_of(c).is_empty());
    }
}
