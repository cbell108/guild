//! Recommendation output: a score plus plain-English reasons.

use serde::{Deserialize, Serialize};

use crate::ids::EventId;

/// Which part of the hybrid recommender produced a reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonKind {
    /// Matches the user's interests or past events.
    Content,
    /// People who go to what you go to also go to this.
    Collaborative,
    /// Friends are going / you're likely to actually attend.
    Social,
    /// Popular on campus.
    Popular,
}

/// One explanation shown on a card ("Because you like board games").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reason {
    /// Source component.
    pub kind: ReasonKind,
    /// User-facing sentence.
    pub text: String,
}

/// Per-component contributions (already weighted) that sum to the score.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Content-based part.
    pub content: f64,
    /// Collaborative part.
    pub collaborative: f64,
    /// Social part.
    pub social: f64,
    /// Popularity part.
    pub popular: f64,
}

impl ScoreBreakdown {
    /// Sum of all parts.
    pub fn total(&self) -> f64 {
        self.content + self.collaborative + self.social + self.popular
    }
}

/// One ranked feed item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    /// The event.
    pub event_id: EventId,
    /// Final score (higher is better).
    pub score: f64,
    /// Reasons, most important first; at least one.
    pub reasons: Vec<Reason>,
    /// Component contributions for "Why am I seeing this?".
    pub breakdown: ScoreBreakdown,
}
