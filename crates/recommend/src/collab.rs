//! Item-item collaborative scorer (C7).

use hangouts_core::Event;

use crate::RecContext;
use crate::content::Component;

/// Item-item collaborative filtering over implicit feedback.
pub struct CollabScorer;

impl CollabScorer {
    /// Builds the scorer for `ctx.user`.
    pub fn new(ctx: &RecContext<'_>) -> Self {
        let _ = ctx;
        Self
    }

    /// Scores one candidate.
    pub fn score(&self, event: &Event) -> Component {
        let _ = event;
        Component::ZERO
    }
}
