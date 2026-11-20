//! Attendance probability: will this person actually show up? (intent → check-in gap)

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use hangouts_core::{EventId, IntentKind, IntentStatus, Snapshot, UserId, Visibility};

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
    ///
    /// Full-batch gradient descent on the log loss with a small L2 penalty, starting from
    /// the prior, for a fixed number of steps: deterministic for a given snapshot.
    pub fn train(snapshot: &Snapshot, now: DateTime<Utc>) -> Self {
        let samples = training_samples(snapshot, now);
        let positives = samples.iter().filter(|(_, y)| *y).count();
        if samples.len() < MIN_TRAINING_SAMPLES || positives == 0 || positives == samples.len() {
            return Self::prior();
        }
        let xs: Vec<[f64; 4]> = samples.iter().map(|(f, _)| encode(f)).collect();
        let ys: Vec<f64> = samples
            .iter()
            .map(|(_, y)| if *y { 1.0 } else { 0.0 })
            .collect();
        let n = xs.len() as f64;
        let mut m = Self::prior();
        for _ in 0..TRAIN_STEPS {
            let mut grad_b = 0.0;
            let mut grad_w = [0.0; 4];
            for (x, y) in xs.iter().zip(&ys) {
                let err = m.prob(x) - y;
                grad_b += err;
                for (g, xi) in grad_w.iter_mut().zip(x) {
                    *g += err * xi;
                }
            }
            m.bias -= LEARNING_RATE * grad_b / n;
            for (w, g) in m.weights.iter_mut().zip(grad_w) {
                *w -= LEARNING_RATE * (g / n + L2 * *w);
            }
        }
        m
    }

    /// Bias and weights for (level, days ahead, friends going, past show rate), for reports.
    pub fn coefficients(&self) -> (f64, [f64; 4]) {
        (self.bias, self.weights)
    }

    /// Mean log loss on labelled samples (lower is better); 0 for no samples.
    pub fn log_loss(&self, samples: &[(AttendanceFeatures, bool)]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        let total: f64 = samples
            .iter()
            .map(|(f, y)| {
                let p = self.predict(f).clamp(1e-9, 1.0 - 1e-9);
                if *y { -p.ln() } else { -(1.0 - p).ln() }
            })
            .sum();
        total / samples.len() as f64
    }

    fn prob(&self, x: &[f64; 4]) -> f64 {
        let z = self.bias + self.weights.iter().zip(x).map(|(w, x)| w * x).sum::<f64>();
        1.0 / (1.0 + (-z).exp())
    }

    /// Probability in `0..=1`.
    pub fn predict(&self, f: &AttendanceFeatures) -> f64 {
        self.prob(&encode(f))
    }
}

/// Fewer labelled intents than this ⇒ [`AttendanceModel::train`] keeps the prior.
pub const MIN_TRAINING_SAMPLES: usize = 30;
const TRAIN_STEPS: usize = 1500;
const LEARNING_RATE: f64 = 0.3;
const L2: f64 = 1e-3;
/// Friends going is capped here before entering the model (diminishing returns).
const MAX_FRIENDS_FEATURE: u32 = 5;

fn encode(f: &AttendanceFeatures) -> [f64; 4] {
    let level = match f.level {
        IntentKind::Interested => 0.0,
        IntentKind::Going => 1.0,
        IntentKind::GoingWith => 1.3,
    };
    [
        level,
        f.days_ahead.max(0.0),
        f64::from(f.friends_going.min(MAX_FRIENDS_FEATURE)),
        f.past_show_rate.clamp(0.0, 1.0),
    ]
}

/// `user`'s check-ins ÷ Going intents over events that started before `before`
/// (`exclude` left out); 0.5 when there are none.
pub fn past_show_rate(
    snapshot: &Snapshot,
    user: UserId,
    before: DateTime<Utc>,
    exclude: Option<EventId>,
) -> f64 {
    let checked: BTreeSet<EventId> = snapshot
        .checkins_for_user(user)
        .map(|c| c.event_id)
        .collect();
    let mut going = 0_u32;
    let mut shows = 0_u32;
    for i in snapshot.intents_for_user(user) {
        if !i.level.uses_spot() || Some(i.event_id) == exclude {
            continue;
        }
        let Some(e) = snapshot.event(i.event_id) else {
            continue;
        };
        if e.starts_at >= before {
            continue;
        }
        going += 1;
        if checked.contains(&i.event_id) {
            shows += 1;
        }
    }
    if going == 0 {
        0.5
    } else {
        f64::from(shows) / f64::from(going)
    }
}

/// Accepted friends of `user` with a confirmed Going / Going-with intent on `event` made
/// before `at`, skipping friends whose attendance is private.
pub fn friends_going(snapshot: &Snapshot, user: UserId, event: EventId, at: DateTime<Utc>) -> u32 {
    let friends = snapshot.friends_of(user);
    let hidden: BTreeSet<UserId> = snapshot
        .users
        .iter()
        .filter(|u| u.privacy.attendance == Visibility::Private)
        .map(|u| u.id)
        .collect();
    snapshot
        .intents_for_event(event)
        .filter(|i| {
            friends.contains(&i.user_id)
                && !hidden.contains(&i.user_id)
                && i.level.uses_spot()
                && i.status == IntentStatus::Confirmed
                && i.created_at < at
        })
        .count() as u32
}

/// One labelled example per intent on an event that started before `now`: features as of
/// the moment the intent was made, label = the user checked in.
pub fn training_samples(
    snapshot: &Snapshot,
    now: DateTime<Utc>,
) -> Vec<(AttendanceFeatures, bool)> {
    let checked: BTreeSet<(UserId, EventId)> = snapshot
        .checkins
        .iter()
        .map(|c| (c.user_id, c.event_id))
        .collect();
    let starts: BTreeMap<EventId, DateTime<Utc>> = snapshot
        .events
        .iter()
        .map(|e| (e.id, e.starts_at))
        .collect();
    let mut out = Vec::new();
    for i in &snapshot.intents {
        let Some(&start) = starts.get(&i.event_id) else {
            continue;
        };
        if start >= now || i.created_at >= now {
            continue;
        }
        let features = AttendanceFeatures {
            level: i.level.kind(),
            days_ahead: (start - i.created_at).num_minutes().max(0) as f64 / (24.0 * 60.0),
            friends_going: friends_going(snapshot, i.user_id, i.event_id, start),
            past_show_rate: past_show_rate(snapshot, i.user_id, start, Some(i.event_id)),
        };
        out.push((features, checked.contains(&(i.user_id, i.event_id))));
    }
    out
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

    #[test]
    fn too_little_history_keeps_prior() {
        let snap = crate::test_support::fixture();
        assert_eq!(
            AttendanceModel::train(&snap, crate::test_support::now()),
            AttendanceModel::prior()
        );
    }

    #[test]
    fn show_rate_and_friends_going() {
        use crate::test_support::*;
        let snap = fixture();
        assert!((past_show_rate(&snap, BOB, now(), None) - 1.0).abs() < 1e-9);
        assert!((past_show_rate(&snap, ALICE, now(), None)).abs() < 1e-9);
        assert!((past_show_rate(&snap, ERIN, now(), None) - 0.5).abs() < 1e-9);
        assert_eq!(friends_going(&snap, ALICE, GAME_NIGHT, now()), 1);
        // Dana's attendance is private, so she is not counted.
        assert_eq!(friends_going(&snap, ALICE, HOOPS, now()), 0);
    }

    #[test]
    fn trains_on_seed_history_and_beats_prior() {
        let now = crate::test_support::now();
        let snap = seed::generate(&seed::SeedConfig::demo(now));
        let samples = training_samples(&snap, now);
        assert!(samples.len() > 300, "{} samples", samples.len());
        let trained = AttendanceModel::train(&snap, now);
        assert_eq!(trained, AttendanceModel::train(&snap, now), "deterministic");
        assert!(trained.log_loss(&samples) < AttendanceModel::prior().log_loss(&samples));
        let (_, w) = trained.coefficients();
        assert!(
            w[0] > 0.0,
            "commitment level raises show probability: {w:?}"
        );
        assert!(w[3] > 0.0, "reliable people keep showing up: {w:?}");
    }
}
