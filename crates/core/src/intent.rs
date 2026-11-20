//! Attendance intent: Interested → Going → Going with ___, plus the capacity/waitlist rule.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::{EventId, UserId};

/// How committed a student is to an event.
///
/// JSON: `{"level":"interested"}`, `{"level":"going"}`, `{"level":"going_with","with":"<user id>"}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "level", content = "with", rename_all = "snake_case")]
pub enum IntentLevel {
    /// Might go; does not take a spot.
    Interested,
    /// Going; takes a spot when the event has a capacity.
    Going,
    /// Going together with another student (who records their own intent).
    GoingWith(UserId),
}

/// The level without its companion, for storage columns and API fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentKind {
    /// See [`IntentLevel::Interested`].
    Interested,
    /// See [`IntentLevel::Going`].
    Going,
    /// See [`IntentLevel::GoingWith`].
    GoingWith,
}

impl IntentKind {
    /// Snake-case name used in storage.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Interested => "interested",
            Self::Going => "going",
            Self::GoingWith => "going_with",
        }
    }

    /// Inverse of [`IntentKind::as_str`].
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "interested" => Some(Self::Interested),
            "going" => Some(Self::Going),
            "going_with" => Some(Self::GoingWith),
            _ => None,
        }
    }
}

impl IntentLevel {
    /// The level's kind.
    pub const fn kind(&self) -> IntentKind {
        match self {
            Self::Interested => IntentKind::Interested,
            Self::Going => IntentKind::Going,
            Self::GoingWith(_) => IntentKind::GoingWith,
        }
    }

    /// The companion for [`IntentLevel::GoingWith`].
    pub const fn companion(&self) -> Option<UserId> {
        match self {
            Self::GoingWith(u) => Some(*u),
            _ => None,
        }
    }

    /// Rebuilds a level from its storage/API parts.
    ///
    /// `going_with` requires a companion; the other kinds ignore it.
    pub fn from_parts(kind: IntentKind, with: Option<UserId>) -> Result<Self, DomainError> {
        match (kind, with) {
            (IntentKind::Interested, _) => Ok(Self::Interested),
            (IntentKind::Going, _) => Ok(Self::Going),
            (IntentKind::GoingWith, Some(u)) => Ok(Self::GoingWith(u)),
            (IntentKind::GoingWith, None) => {
                Err(DomainError::invalid("with", "is required for going_with"))
            }
        }
    }

    /// Whether this level takes one spot of a capped event.
    pub const fn uses_spot(&self) -> bool {
        !matches!(self, Self::Interested)
    }
}

/// Whether an intent holds a spot or is waiting for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    /// Counts as attending (or is Interested, which never waits).
    Confirmed,
    /// Waiting for a spot; promoted in `created_at` order when someone backs out.
    Waitlisted,
}

impl IntentStatus {
    /// Snake-case name used in storage.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Waitlisted => "waitlisted",
        }
    }

    /// Inverse of [`IntentStatus::as_str`].
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "confirmed" => Some(Self::Confirmed),
            "waitlisted" => Some(Self::Waitlisted),
            _ => None,
        }
    }
}

/// One student's intent for one event (at most one per pair).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    /// Who.
    pub user_id: UserId,
    /// Which event.
    pub event_id: EventId,
    /// Commitment level.
    #[serde(flatten)]
    pub level: IntentLevel,
    /// Spot held or waitlisted.
    pub status: IntentStatus,
    /// First time this user expressed any intent for the event (waitlist order).
    pub created_at: DateTime<Utc>,
    /// Last change.
    pub updated_at: DateTime<Utc>,
}

/// Decides the status for a new or changed intent.
///
/// `spots_taken` is the number of *other* users currently `Confirmed` with a level that
/// [uses a spot](IntentLevel::uses_spot) — callers must exclude the user's own current intent,
/// so someone already holding a spot keeps it when switching Going ↔ Going with.
/// Interested never waits; with no capacity nobody waits.
pub fn admission(capacity: Option<u32>, spots_taken: u32, level: &IntentLevel) -> IntentStatus {
    match capacity {
        Some(cap) if level.uses_spot() && spots_taken >= cap => IntentStatus::Waitlisted,
        _ => IntentStatus::Confirmed,
    }
}

/// The waitlisted user to promote when a spot frees up: earliest `created_at`, ties broken
/// by user id. `intents` should be the event's intents.
pub fn next_to_promote(intents: &[Intent]) -> Option<UserId> {
    intents
        .iter()
        .filter(|i| i.status == IntentStatus::Waitlisted)
        .min_by_key(|i| (i.created_at, i.user_id))
        .map(|i| i.user_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    #[test]
    fn json_shape() {
        let with = UserId::from_u128(9);
        assert_eq!(
            serde_json::to_string(&IntentLevel::Going).unwrap(),
            r#"{"level":"going"}"#
        );
        assert_eq!(
            serde_json::to_string(&IntentLevel::GoingWith(with)).unwrap(),
            r#"{"level":"going_with","with":"00000000-0000-0000-0000-000000000009"}"#
        );
        let parsed: IntentLevel = serde_json::from_str(r#"{"level":"interested"}"#).unwrap();
        assert_eq!(parsed, IntentLevel::Interested);
    }

    #[test]
    fn parts_round_trip() {
        let u = UserId::from_u128(3);
        for level in [
            IntentLevel::Interested,
            IntentLevel::Going,
            IntentLevel::GoingWith(u),
        ] {
            assert_eq!(
                IntentLevel::from_parts(level.kind(), level.companion()),
                Ok(level)
            );
            assert_eq!(IntentKind::parse(level.kind().as_str()), Some(level.kind()));
        }
        assert!(IntentLevel::from_parts(IntentKind::GoingWith, None).is_err());
    }

    #[test]
    fn admission_rules() {
        use IntentStatus::{Confirmed, Waitlisted};
        let u = UserId::from_u128(3);
        assert_eq!(admission(None, 999, &IntentLevel::Going), Confirmed);
        assert_eq!(admission(Some(2), 1, &IntentLevel::Going), Confirmed);
        assert_eq!(admission(Some(2), 2, &IntentLevel::Going), Waitlisted);
        assert_eq!(
            admission(Some(2), 2, &IntentLevel::GoingWith(u)),
            Waitlisted
        );
        assert_eq!(admission(Some(2), 5, &IntentLevel::Interested), Confirmed);
    }

    #[test]
    fn promotes_earliest_waitlisted() {
        let t0 = DateTime::<Utc>::UNIX_EPOCH;
        let mk = |n: u128, status, mins| Intent {
            user_id: UserId::from_u128(n),
            event_id: EventId::from_u128(1),
            level: IntentLevel::Going,
            status,
            created_at: t0 + TimeDelta::minutes(mins),
            updated_at: t0,
        };
        let intents = vec![
            mk(1, IntentStatus::Confirmed, 0),
            mk(2, IntentStatus::Waitlisted, 20),
            mk(3, IntentStatus::Waitlisted, 10),
        ];
        assert_eq!(next_to_promote(&intents), Some(UserId::from_u128(3)));
        assert_eq!(next_to_promote(&intents[..1]), None);
    }
}
