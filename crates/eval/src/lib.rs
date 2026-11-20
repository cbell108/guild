//! Offline evaluation of the recommender on seeded data (no database, no network).
//!
//! Protocol (`docs/EVAL.md` explains it for the pitch):
//! 1. Generate the demo world with [`seed::generate`] relative to a fixed `now`.
//! 2. Hide the last week: the **cutoff** is `now − 7 days`; intents made and check-ins
//!    recorded after it are removed, events stay (they are announced in advance).
//! 3. For each student, ask each method for a ranking of the events starting in the hidden
//!    week as of the cutoff. Events the student had already RSVP'd to before the cutoff are
//!    removed from both the ranking and the answer key, so every method is scored on
//!    *discovery*, not on echoing known RSVPs.
//! 4. The answer key is the events the student actually checked in to.
//! 5. Report precision@5, recall@10 and NDCG@10, averaged over students with at least one
//!    hidden check-in, and averaged again over several seeds.

use std::collections::BTreeSet;

use chrono::{DateTime, TimeDelta, Utc};
use hangouts_core::{EventId, Snapshot, UserId};
use recommend::{AttendanceModel, FeedConfig, HashEmbedder, RecContext, Weights, feed};

/// Length of the held-out window.
pub const HOLDOUT: TimeDelta = TimeDelta::days(7);

/// A ranking method under evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    /// Soonest events first (the pre-recommender placeholder).
    Chronological,
    /// The feed with custom weights (hybrid, a single component, or an ablation).
    Feed(Weights),
}

/// A named method, as shown in the table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NamedMethod {
    /// Row label.
    pub name: &'static str,
    /// What it runs.
    pub method: Method,
}

/// Weights picked by a grid search on validation seeds 100–105 (never on the reported seeds).
pub const VALIDATION_TUNED: Weights = Weights {
    content: 0.6,
    collaborative: 0.0,
    social: 0.4,
    popular: 0.0,
};

/// The rows of the results table, baselines first, the default hybrid last.
pub fn methods() -> Vec<NamedMethod> {
    let only = |content, collaborative, social, popular| {
        Method::Feed(Weights {
            content,
            collaborative,
            social,
            popular,
        })
    };
    vec![
        NamedMethod {
            name: "Chronological (soonest first)",
            method: Method::Chronological,
        },
        NamedMethod {
            name: "Popularity baseline",
            method: Method::Feed(Weights::POPULAR_ONLY),
        },
        NamedMethod {
            name: "Content-only",
            method: Method::Feed(Weights::CONTENT_ONLY),
        },
        NamedMethod {
            name: "Collaborative-only",
            method: only(0.0, 1.0, 0.0, 0.0),
        },
        NamedMethod {
            name: "Social-only",
            method: only(0.0, 0.0, 1.0, 0.0),
        },
        NamedMethod {
            name: "Hybrid (weights tuned on validation seeds)",
            method: Method::Feed(VALIDATION_TUNED),
        },
        NamedMethod {
            name: "**Hybrid (default weights)**",
            method: Method::Feed(Weights::default()),
        },
    ]
}

/// Averaged ranking metrics.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Metrics {
    /// Mean precision@5.
    pub precision_at_5: f64,
    /// Mean recall@10.
    pub recall_at_10: f64,
    /// Mean NDCG@10 (binary relevance).
    pub ndcg_at_10: f64,
    /// Students evaluated.
    pub users: usize,
}

impl Metrics {
    /// Element-wise mean of several runs (`users` is summed).
    pub fn mean(runs: &[Metrics]) -> Metrics {
        let n = runs.len().max(1) as f64;
        Metrics {
            precision_at_5: runs.iter().map(|m| m.precision_at_5).sum::<f64>() / n,
            recall_at_10: runs.iter().map(|m| m.recall_at_10).sum::<f64>() / n,
            ndcg_at_10: runs.iter().map(|m| m.ndcg_at_10).sum::<f64>() / n,
            users: runs.iter().map(|m| m.users).sum(),
        }
    }
}

/// The world as of `cutoff`: later intents, check-ins and buddy requests removed.
pub fn as_of(snapshot: &Snapshot, cutoff: DateTime<Utc>) -> Snapshot {
    let mut s = snapshot.clone();
    s.intents.retain(|i| i.created_at < cutoff);
    s.checkins.retain(|c| c.at < cutoff);
    s.buddy_requests.retain(|b| b.created_at < cutoff);
    s
}

/// One student's held-out task.
#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    /// The student.
    pub user: UserId,
    /// Hidden-week events they checked in to and had not RSVP'd to before the cutoff.
    pub truth: BTreeSet<EventId>,
    /// Events removed from rankings (RSVP'd before the cutoff, or hosted).
    pub known: BTreeSet<EventId>,
}

/// Builds every student's case for the window `[cutoff, cutoff + HOLDOUT)`.
pub fn cases(full: &Snapshot, train: &Snapshot, cutoff: DateTime<Utc>) -> Vec<Case> {
    let end = cutoff + HOLDOUT;
    let hidden: BTreeSet<EventId> = full
        .events
        .iter()
        .filter(|e| e.starts_at >= cutoff && e.starts_at < end)
        .map(|e| e.id)
        .collect();
    full.users
        .iter()
        .filter_map(|u| {
            let mut known: BTreeSet<EventId> =
                train.intents_for_user(u.id).map(|i| i.event_id).collect();
            known.extend(
                full.events
                    .iter()
                    .filter(|e| e.host_id == Some(u.id))
                    .map(|e| e.id),
            );
            let truth: BTreeSet<EventId> = full
                .checkins_for_user(u.id)
                .map(|c| c.event_id)
                .filter(|e| hidden.contains(e) && !known.contains(e))
                .collect();
            (!truth.is_empty()).then_some(Case {
                user: u.id,
                truth,
                known,
            })
        })
        .collect()
}

/// A method's ranking of hidden-week events for `case.user`, known events removed.
pub fn rank(
    method: Method,
    train: &Snapshot,
    case: &Case,
    cutoff: DateTime<Utc>,
    model: &AttendanceModel,
    embedder: &HashEmbedder,
) -> Vec<EventId> {
    let ranked: Vec<EventId> = match method {
        Method::Chronological => {
            let mut events: Vec<_> = train
                .events
                .iter()
                .filter(|e| e.starts_at >= cutoff && e.starts_at < cutoff + HOLDOUT)
                .collect();
            events.sort_by_key(|e| (e.starts_at, e.id));
            events.into_iter().map(|e| e.id).collect()
        }
        Method::Feed(weights) => {
            let ctx = RecContext {
                snapshot: train,
                user: case.user,
                now: cutoff,
                model,
                embedder,
            };
            let cfg = FeedConfig {
                horizon: HOLDOUT,
                limit: usize::MAX,
                weights,
            };
            feed(&ctx, &cfg).into_iter().map(|r| r.event_id).collect()
        }
    };
    ranked
        .into_iter()
        .filter(|e| !case.known.contains(e))
        .collect()
}

/// precision@k, recall@k and NDCG@k of one ranking.
pub fn score_ranking(ranked: &[EventId], truth: &BTreeSet<EventId>, k: usize) -> (f64, f64, f64) {
    if k == 0 || truth.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let top = &ranked[..ranked.len().min(k)];
    let hits = top.iter().filter(|e| truth.contains(e)).count() as f64;
    let dcg: f64 = top
        .iter()
        .enumerate()
        .filter(|(_, e)| truth.contains(e))
        .map(|(i, _)| 1.0 / ((i + 2) as f64).log2())
        .sum();
    let ideal: f64 = (0..truth.len().min(k))
        .map(|i| 1.0 / ((i + 2) as f64).log2())
        .sum();
    (hits / k as f64, hits / truth.len() as f64, dcg / ideal)
}

/// Metrics for one method on one seeded world.
pub fn evaluate(
    method: Method,
    train: &Snapshot,
    cases: &[Case],
    cutoff: DateTime<Utc>,
    model: &AttendanceModel,
) -> Metrics {
    let embedder = HashEmbedder::default();
    let mut sums = (0.0, 0.0, 0.0);
    for case in cases {
        let ranked = rank(method, train, case, cutoff, model, &embedder);
        let (p5, _, _) = score_ranking(&ranked, &case.truth, 5);
        let (_, r10, n10) = score_ranking(&ranked, &case.truth, 10);
        sums.0 += p5;
        sums.1 += r10;
        sums.2 += n10;
    }
    let n = cases.len().max(1) as f64;
    Metrics {
        precision_at_5: sums.0 / n,
        recall_at_10: sums.1 / n,
        ndcg_at_10: sums.2 / n,
        users: cases.len(),
    }
}

/// Attendance-model quality on the hidden week: log loss of the prior vs. the model trained
/// on data before the cutoff, scored on intents for hidden-week events.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelReport {
    /// Model trained at the cutoff.
    pub trained: AttendanceModel,
    /// Mean log loss of the hand-set prior.
    pub prior_log_loss: f64,
    /// Mean log loss of the trained model.
    pub trained_log_loss: f64,
    /// Labelled hidden-week intents scored.
    pub samples: usize,
}

/// Trains at `cutoff` and scores both models on the hidden week's intents.
pub fn model_report(full: &Snapshot, train: &Snapshot, cutoff: DateTime<Utc>) -> ModelReport {
    let trained = AttendanceModel::train(train, cutoff);
    let hidden = hidden_samples(full, cutoff);
    ModelReport {
        prior_log_loss: AttendanceModel::prior().log_loss(&hidden),
        trained_log_loss: trained.log_loss(&hidden),
        samples: hidden.len(),
        trained,
    }
}

/// Labelled intents for events starting in the hidden week. Features (friends going, past
/// show rate) are computed by [`recommend::training_samples`] on the full world, which only
/// looks at history before each event's start.
fn hidden_samples(
    full: &Snapshot,
    cutoff: DateTime<Utc>,
) -> Vec<(recommend::AttendanceFeatures, bool)> {
    let window_end = cutoff + HOLDOUT;
    let rows = recommend::training_samples(full, window_end);
    // `training_samples` yields one row per intent, in order, for intents whose event
    // exists and both started and was RSVP'd before `window_end`; mirror that filter.
    let in_hidden_week: Vec<bool> = full
        .intents
        .iter()
        .filter_map(|i| {
            let e = full.event(i.event_id)?;
            (e.starts_at < window_end && i.created_at < window_end).then_some(e.starts_at >= cutoff)
        })
        .collect();
    debug_assert_eq!(rows.len(), in_hidden_week.len());
    rows.into_iter()
        .zip(in_hidden_week)
        .filter_map(|(row, keep)| keep.then_some(row))
        .collect()
}

/// Everything `docs/EVAL.md` reports.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// The evaluation's "now" (end of the hidden week).
    pub now: DateTime<Utc>,
    /// RNG seeds evaluated.
    pub seeds: Vec<u64>,
    /// (method name, metrics averaged over seeds).
    pub rows: Vec<(&'static str, Metrics)>,
    /// Attendance model on the first seed.
    pub model: ModelReport,
}

/// Runs every method on every seed.
pub fn run(now: DateTime<Utc>, seeds: &[u64]) -> Report {
    let cutoff = now - HOLDOUT;
    let methods = methods();
    let mut per_method: Vec<Vec<Metrics>> = vec![Vec::new(); methods.len()];
    let mut model = None;
    for &seed in seeds {
        let full = seed::generate(&seed::SeedConfig {
            rng_seed: seed,
            ..seed::SeedConfig::demo(now)
        });
        let train = as_of(&full, cutoff);
        let report = model_report(&full, &train, cutoff);
        let cases = cases(&full, &train, cutoff);
        for (k, m) in methods.iter().enumerate() {
            per_method[k].push(evaluate(m.method, &train, &cases, cutoff, &report.trained));
        }
        model.get_or_insert(report);
    }
    Report {
        now,
        seeds: seeds.to_vec(),
        rows: methods
            .iter()
            .zip(per_method)
            .map(|(m, runs)| (m.name, Metrics::mean(&runs)))
            .collect(),
        model: model.unwrap_or_else(|| ModelReport {
            trained: AttendanceModel::prior(),
            prior_log_loss: 0.0,
            trained_log_loss: 0.0,
            samples: 0,
        }),
    }
}

/// Renders the report as the Markdown in `docs/EVAL.md`.
pub fn to_markdown(r: &Report) -> String {
    use std::fmt::Write as _;
    let mut md = String::new();
    let seeds: Vec<String> = r.seeds.iter().map(u64::to_string).collect();
    let _ = writeln!(md, "# Recommender evaluation\n");
    let _ = writeln!(
        md,
        "Generated by `cargo run -p eval` — do not edit by hand. Offline, deterministic: \
         seeded demo world (60 students, 10 crews, 4 weeks of history), `HashEmbedder`, \
         no network.\n"
    );
    let _ = writeln!(md, "## Results\n");
    let _ = writeln!(
        md,
        "Last week held out; mean over seeds {} ({} student-weeks with at least one hidden check-in).\n",
        seeds.join(", "),
        r.rows.first().map_or(0, |(_, m)| m.users)
    );
    let _ = writeln!(md, "| Method | Precision@5 | Recall@10 | NDCG@10 |");
    let _ = writeln!(md, "|---|---:|---:|---:|");
    for (name, m) in &r.rows {
        let _ = writeln!(
            md,
            "| {name} | {:.3} | {:.3} | {:.3} |",
            m.precision_at_5, m.recall_at_10, m.ndcg_at_10
        );
    }
    let baseline = r
        .rows
        .iter()
        .find(|(n, _)| n.starts_with("Popularity"))
        .map(|(_, m)| *m);
    let hybrid = r.rows.last().map(|(_, m)| *m);
    if let (Some(b), Some(h)) = (baseline, hybrid)
        && b.precision_at_5 > 0.0
    {
        let _ = writeln!(
            md,
            "\nHybrid vs. popularity: precision@5 ×{:.2}, recall@10 ×{:.2}.",
            h.precision_at_5 / b.precision_at_5,
            h.recall_at_10 / b.recall_at_10.max(f64::EPSILON)
        );
    }
    let (bias, w) = r.model.trained.coefficients();
    let _ = writeln!(md, "\n## Attendance model (will they actually show up?)\n");
    let _ = writeln!(
        md,
        "Logistic regression trained on intents before the cutoff; scored on {} hidden-week intents.\n",
        r.model.samples
    );
    let _ = writeln!(md, "| Model | Log loss (lower is better) |");
    let _ = writeln!(md, "|---|---:|");
    let _ = writeln!(md, "| Hand-set prior | {:.3} |", r.model.prior_log_loss);
    let _ = writeln!(md, "| Trained | {:.3} |", r.model.trained_log_loss);
    let _ = writeln!(
        md,
        "\nCoefficients: bias {bias:.2}; intent level {:.2}; days ahead {:.2}; friends going {:.2}; past show rate {:.2}.",
        w[0], w[1], w[2], w[3]
    );
    let _ = writeln!(md, "\n## Method\n");
    let _ = writeln!(
        md,
        "1. Generate the demo world with `seed::generate` ending at {} (fixed, so this file is reproducible).\n\
         2. Cutoff = one week earlier. Intents and check-ins after the cutoff are hidden; events stay, since they are announced in advance.\n\
         3. For each student, every method ranks the events starting in the hidden week, as of the cutoff.\n\
         4. Events the student had already RSVP'd to (or hosts) before the cutoff are removed from both the ranking and the answer key, so methods are scored on **discovery**.\n\
         5. The answer key is the hidden-week events the student actually **checked in** to (not just RSVP'd).\n\
         6. Metrics are averaged over students with at least one such check-in, then over seeds.\n",
        r.now.format("%Y-%m-%d %H:%M UTC")
    );
    let _ = writeln!(
        md,
        "Baselines: *chronological* is the pre-recommender placeholder; *popularity* ranks by campus-wide RSVPs so far. \
         *Content-only*, *collaborative-only* and *social-only* are the hybrid's components on their own. \
         The *validation-tuned* row uses weights from a grid search on seeds 100–105, which are disjoint from the reported seeds; \
         it is a check that the default weights are not cherry-picked for this table. On this synthetic data the \
         collaborative part adds little once content and friends are known; it should matter more as real history accumulates."
    );
    md
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-20T17:00:00Z")
            .unwrap()
            .to_utc()
    }

    #[test]
    fn ranking_metrics() {
        let e = |n| EventId::from_u128(n);
        let truth = BTreeSet::from([e(1), e(3)]);
        let (p, r, n) = score_ranking(&[e(1), e(2), e(3)], &truth, 2);
        assert!((p - 0.5).abs() < 1e-9 && (r - 0.5).abs() < 1e-9);
        assert!(n > 0.0 && n < 1.0);
        let (_, _, perfect) = score_ranking(&[e(3), e(1)], &truth, 10);
        assert!((perfect - 1.0).abs() < 1e-9);
        assert_eq!(score_ranking(&[], &truth, 5), (0.0, 0.0, 0.0));
    }

    #[test]
    fn holdout_hides_the_future_and_known_rsvps() {
        let full = seed::generate(&seed::SeedConfig::demo(now()));
        let cutoff = now() - HOLDOUT;
        let train = as_of(&full, cutoff);
        assert!(train.intents.iter().all(|i| i.created_at < cutoff));
        assert!(train.checkins.iter().all(|c| c.at < cutoff));
        let cases = cases(&full, &train, cutoff);
        assert!(cases.len() >= 10, "{} cases", cases.len());
        for c in &cases {
            assert!(c.truth.is_disjoint(&c.known));
        }
    }

    #[test]
    fn hybrid_beats_popularity_on_one_seed() {
        let r = run(now(), &[2026]);
        let get = |prefix: &str| {
            r.rows
                .iter()
                .find(|(n, _)| n.contains(prefix))
                .map(|(_, m)| *m)
                .unwrap()
        };
        let hybrid = get("Hybrid");
        let popular = get("Popularity");
        assert!(hybrid.precision_at_5 > popular.precision_at_5, "{r:?}");
        assert!(hybrid.recall_at_10 > popular.recall_at_10, "{r:?}");
        assert!(r.model.trained_log_loss < r.model.prior_log_loss);
        let md = to_markdown(&r);
        assert!(md.contains("| **Hybrid (default weights)** |"));
    }
}
