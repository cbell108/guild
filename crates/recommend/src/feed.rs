//! The "For you" feed.

use chrono::TimeDelta;
use hangouts_core::{Reason, ReasonKind, Recommendation, ScoreBreakdown};

use crate::RecContext;

/// Component weights for the hybrid score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weights {
    /// Content-based (interests, attended events).
    pub content: f64,
    /// Item-item collaborative.
    pub collaborative: f64,
    /// Friends + attendance probability.
    pub social: f64,
    /// Campus-wide popularity.
    pub popular: f64,
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            content: 0.4,
            collaborative: 0.25,
            social: 0.25,
            popular: 0.1,
        }
    }
}

/// Feed options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeedConfig {
    /// Only events starting within this long after `now` (default 7 days).
    pub horizon: TimeDelta,
    /// Max items (default 10).
    pub limit: usize,
    /// Hybrid weights.
    pub weights: Weights,
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            horizon: TimeDelta::days(7),
            limit: 10,
            weights: Weights::default(),
        }
    }
}

/// Ranks upcoming events (starting in `[now, now + horizon)`) for `ctx.user`, best first,
/// at most `cfg.limit`, each with at least one plain-English reason.
pub fn feed(ctx: &RecContext<'_>, cfg: &FeedConfig) -> Vec<Recommendation> {
    // Lane C (C5) replaces this placeholder: soonest upcoming events, unscored.
    let end = ctx.now + cfg.horizon;
    let mut upcoming: Vec<_> = ctx
        .snapshot
        .events
        .iter()
        .filter(|e| e.starts_at >= ctx.now && e.starts_at < end)
        .collect();
    upcoming.sort_by_key(|e| (e.starts_at, e.id));
    upcoming
        .into_iter()
        .take(cfg.limit)
        .map(|e| Recommendation {
            event_id: e.id,
            score: 0.0,
            reasons: vec![Reason {
                kind: ReasonKind::Popular,
                text: "Coming up soon".to_owned(),
            }],
            breakdown: ScoreBreakdown::default(),
        })
        .collect()
}
