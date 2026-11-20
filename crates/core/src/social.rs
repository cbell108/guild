//! Friendships, crews, and buddy requests.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{CrewId, EventId, UserId};

/// State of a friendship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FriendshipStatus {
    /// Requested by `requester`, not yet accepted.
    Pending,
    /// Mutual.
    Accepted,
}

/// A friendship between two users (at most one per unordered pair).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Friendship {
    /// Who asked.
    pub requester: UserId,
    /// Who was asked.
    pub addressee: UserId,
    /// Pending or accepted.
    pub status: FriendshipStatus,
    /// When the request was made.
    pub created_at: DateTime<Utc>,
}

impl Friendship {
    /// Whether `user` is one of the two sides.
    pub fn involves(&self, user: UserId) -> bool {
        self.requester == user || self.addressee == user
    }

    /// The other side, if `user` is one side.
    pub fn other(&self, user: UserId) -> Option<UserId> {
        if self.requester == user {
            Some(self.addressee)
        } else if self.addressee == user {
            Some(self.requester)
        } else {
            None
        }
    }
}

/// A small group that hangs out together. No follower counts, no public membership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crew {
    /// Stable id.
    pub id: CrewId,
    /// Name, e.g. "Smash Saturdays".
    pub name: String,
    /// Short description.
    pub description: String,
    /// Founding member.
    pub created_by: UserId,
    /// When the crew was created.
    pub created_at: DateTime<Utc>,
}

/// A member's role in a crew.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrewRole {
    /// Can add and remove members.
    Owner,
    /// Regular member.
    Member,
}

/// Membership of one user in one crew.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrewMember {
    /// Crew.
    pub crew_id: CrewId,
    /// Member.
    pub user_id: UserId,
    /// Role.
    pub role: CrewRole,
    /// When they joined.
    pub joined_at: DateTime<Utc>,
}

/// "Looking for a buddy" on an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuddyRequest {
    /// Who is looking.
    pub user_id: UserId,
    /// For which event.
    pub event_id: EventId,
    /// Since when.
    pub created_at: DateTime<Utc>,
}
