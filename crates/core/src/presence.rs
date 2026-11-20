//! Self-reported live presence (the prototype's `/game` menu). No Discord presence intents.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::UserId;

/// What a user reported via the live menu buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    /// "I'm in the gameroom" / "I'm here".
    Here,
    /// "I'm interested".
    Interested,
    /// "I'm out" (hidden from the roster).
    Out,
}

impl Presence {
    /// Lowercase name used in storage.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Here => "here",
            Self::Interested => "interested",
            Self::Out => "out",
        }
    }

    /// Inverse of [`Presence::as_str`].
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "here" => Some(Self::Here),
            "interested" => Some(Self::Interested),
            "out" => Some(Self::Out),
            _ => None,
        }
    }
}

/// A user's latest presence report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceReport {
    /// Who.
    pub user_id: UserId,
    /// What they reported.
    pub presence: Presence,
    /// When.
    pub at: DateTime<Utc>,
}
