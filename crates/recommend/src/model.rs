//! Attendance probability: will this person actually show up? (intent → check-in gap)

use chrono::{DateTime, Utc};
use hangouts_core::{IntentKind, Snapshot};

/// Inputs to [`AttendanceModel::predict`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttendanceFeatures {
    /// Current intent level.
    pub level: IntentKind,
    /// Days from now until the event starts (may be fractional, ≥ 0).
    pub days_ahead: f64,
    /// Accepted friends Going.
    pub friends_going: u32,
    /// The user's historical check-ins ÷ Going intents on past events (0..=1; use 0.5 if none).
    pub past_show_rate: f64,
}

/// Logistic model of P(check-in | features). Internals are Lane C's to change.
#[derive(Debug, Clone, PartialEq)]
pub struct AttendanceModel {
    bias: f64,
    weights: [f64; 4],
}

impl AttendanceModel {
    /// Hand-set prior used before (or instead of) training.
    pub fn prior() -> Self {
        Self {
            bias: -1.5,
            weights: [1.5, -0.05, 0.3, 2.0],
        }
    }

    /// Fits the model on past intents vs. check-ins in `snapshot` (events before `now`).
    /// Falls back to [`AttendanceModel::prior`] when there is too little history.
    pub fn train(snapshot: &Snapshot, now: DateTime<Utc>) -> Self {
        // Lane C (C4): logistic regression on seed history.
        let _ = (snapshot, now);
        Self::prior()
    }

    /// Probability in `0..=1`.
    pub fn predict(&self, f: &AttendanceFeatures) -> f64 {
        let level = match f.level {
            IntentKind::Interested => 0.0,
            IntentKind::Going => 1.0,
            IntentKind::GoingWith => 1.3,
        };
        let x = [
            level,
            f.days_ahead,
            f64::from(f.friends_going),
            f.past_show_rate,
        ];
        let z = self.bias + self.weights.iter().zip(x).map(|(w, x)| w * x).sum::<f64>();
        1.0 / (1.0 + (-z).exp())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prior_orders_commitment_levels() {
        let m = AttendanceModel::prior();
        let p = |level| {
            m.predict(&AttendanceFeatures {
                level,
                days_ahead: 1.0,
                friends_going: 0,
                past_show_rate: 0.5,
            })
        };
        assert!(p(IntentKind::Interested) < p(IntentKind::Going));
        assert!(p(IntentKind::Going) < p(IntentKind::GoingWith));
        assert!((0.0..=1.0).contains(&p(IntentKind::Going)));
    }
}
