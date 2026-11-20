//! The "For you" feed: a weighted sum of four explainable components.
//!
//! 1. **content** — cosine between the viewer's profile and the event ([`crate::content`]);
//! 2. **collaborative** — item-item similarity to what the viewer attended ([`crate::collab`]);
//! 3. **social** — friends going + P(viewer shows up) ([`crate::social`]);
//! 4. **popular** — campus-wide engagement ([`crate::popular`]).
//!
//! Content and collaborative scores are rescaled so the best candidate gets 1, which keeps the
//! weights comparable. Reasons come from the components that contributed most.

use chrono::TimeDelta;
use hangouts_core::{Event, Reason, ReasonKind, Recommendation, ScoreBreakdown};

use crate::RecContext;
use crate::collab::CollabScorer;
use crate::content::{Component, ContentScorer};
use crate::popular::popularity;
use crate::social::SocialScorer;

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

impl Weights {
    /// Only content (the "content-only" baseline in `docs/EVAL.md`).
    pub const CONTENT_ONLY: Self = Self {
        content: 1.0,
        collaborative: 0.0,
        social: 0.0,
        popular: 0.0,
    };
    /// Only popularity (the "popularity" baseline).
    pub const POPULAR_ONLY: Self = Self {
        content: 0.0,
        collaborative: 0.0,
        social: 0.0,
        popular: 1.0,
    };
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

/// At most this many reasons per card.
pub const MAX_REASONS: usize = 2;
/// A second reason is shown only if its component added at least this much to the score.
pub const MIN_SECOND_REASON: f64 = 0.05;

/// Ranks upcoming events (starting in `[now, now + horizon)`) for `ctx.user`, best first,
/// at most `cfg.limit`, each with at least one plain-English reason.
///
/// Events the viewer hosts are left out. Ties break by start time, then id.
pub fn feed(ctx: &RecContext<'_>, cfg: &FeedConfig) -> Vec<Recommendation> {
    let end = ctx.now + cfg.horizon;
    let candidates: Vec<&Event> = ctx
        .snapshot
        .events
        .iter()
        .filter(|e| e.starts_at >= ctx.now && e.starts_at < end)
        .filter(|e| e.host_id != Some(ctx.user))
        .collect();
    if candidates.is_empty() || cfg.limit == 0 {
        return Vec::new();
    }
    let w = cfg.weights;
    let content = ContentScorer::new(ctx, &candidates);
    let collab = CollabScorer::new(ctx);
    let social = SocialScorer::new(ctx);
    let popular = popularity(ctx, &candidates);

    let content_scores: Vec<Component> = candidates.iter().map(|e| content.score(e)).collect();
    let collab_scores: Vec<Component> = candidates.iter().map(|e| collab.score(e)).collect();
    let content_max = max_value(&content_scores);
    let collab_max = max_value(&collab_scores);

    let mut ranked: Vec<(&Event, Recommendation)> = candidates
        .iter()
        .enumerate()
        .map(|(k, event)| {
            let c = scaled(&content_scores[k], content_max);
            let cf = scaled(&collab_scores[k], collab_max);
            let s = social.score(event);
            let p = popular.get(&event.id).cloned().unwrap_or(Component::ZERO);
            let breakdown = ScoreBreakdown {
                content: w.content * c.value,
                collaborative: w.collaborative * cf.value,
                social: w.social * s.value,
                popular: w.popular * p.value,
            };
            let reasons = reasons(&[
                (breakdown.content, c.reason),
                (breakdown.collaborative, cf.reason),
                (breakdown.social, s.reason),
                (breakdown.popular, p.reason),
            ]);
            (
                *event,
                Recommendation {
                    event_id: event.id,
                    score: breakdown.total(),
                    reasons,
                    breakdown,
                },
            )
        })
        .collect();
    ranked.sort_by(|(ea, a), (eb, b)| {
        b.score
            .total_cmp(&a.score)
            .then(ea.starts_at.cmp(&eb.starts_at))
            .then(ea.id.cmp(&eb.id))
    });
    ranked.into_iter().take(cfg.limit).map(|(_, r)| r).collect()
}

fn max_value(scores: &[Component]) -> f64 {
    scores.iter().map(|c| c.value).fold(0.0_f64, f64::max)
}

fn scaled(c: &Component, max: f64) -> Component {
    Component {
        value: if max > 0.0 { c.value / max } else { 0.0 },
        reason: c.reason.clone(),
    }
}

/// Reasons of the top contributing components (those that have one), best first, with the
/// generic "popular" reason ranked after specific ones; falls back to "Coming up soon" so
/// every card has at least one.
fn reasons(parts: &[(f64, Option<Reason>)]) -> Vec<Reason> {
    let mut with_reason: Vec<(f64, &Reason)> = parts
        .iter()
        .filter(|(v, _)| *v > 0.0)
        .filter_map(|(v, r)| r.as_ref().map(|r| (*v, r)))
        .collect();
    // Specific reasons (interests, people) beat the generic "popular" one; then by size.
    with_reason.sort_by(|a, b| {
        (a.1.kind == ReasonKind::Popular)
            .cmp(&(b.1.kind == ReasonKind::Popular))
            .then(b.0.total_cmp(&a.0))
    });
    let out: Vec<Reason> = with_reason
        .into_iter()
        .enumerate()
        .take_while(|(k, (v, _))| *k == 0 || *v >= MIN_SECOND_REASON)
        .take(MAX_REASONS)
        .map(|(_, (_, r))| r.clone())
        .collect();
    if out.is_empty() {
        vec![Reason {
            kind: ReasonKind::Popular,
            text: "Coming up soon".to_owned(),
        }]
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::{AttendanceModel, HashEmbedder};
    use hangouts_core::{EventId, UserId};

    fn run(snap: &hangouts_core::Snapshot, user: UserId, cfg: &FeedConfig) -> Vec<Recommendation> {
        let model = AttendanceModel::prior();
        let embedder = HashEmbedder::default();
        feed(&ctx(snap, user, &model, &embedder), cfg)
    }

    fn texts(r: &Recommendation) -> Vec<&str> {
        r.reasons.iter().map(|r| r.text.as_str()).collect()
    }

    #[test]
    fn interests_and_friends_put_game_night_first_with_reasons() {
        let snap = fixture();
        let recs = run(&snap, ALICE, &FeedConfig::default());
        let ids: Vec<EventId> = recs.iter().map(|r| r.event_id).collect();
        assert_eq!(ids[0], GAME_NIGHT);
        assert!(!ids.contains(&FAR_FUTURE), "outside the 7-day horizon");
        assert!(
            !ids.contains(&HOOPS_PAST),
            "past events are not recommended"
        );
        assert_eq!(
            texts(&recs[0]),
            vec!["Because you like board games and tabletop", "Carl is going"]
        );
        assert_eq!(recs[0].reasons[0].kind, ReasonKind::Content);
        assert_eq!(recs[0].reasons[1].kind, ReasonKind::Social);
    }

    #[test]
    fn history_drives_bob_to_basketball() {
        let snap = fixture();
        let recs = run(&snap, BOB, &FeedConfig::default());
        assert_eq!(recs[0].event_id, HOOPS);
        assert!(
            texts(&recs[0]).contains(&"You've been to Pickup Basketball before"),
            "{:?}",
            texts(&recs[0])
        );
    }

    #[test]
    fn every_card_has_a_reason_and_breakdown_sums_to_score() {
        let snap = fixture();
        for user in [ALICE, BOB, CARL, DANA, ERIN, UserId::from_u128(999)] {
            let recs = run(&snap, user, &FeedConfig::default());
            assert_ne!(recs.len(), 0);
            for r in &recs {
                assert!((1..=MAX_REASONS).contains(&r.reasons.len()));
                assert!((r.breakdown.total() - r.score).abs() < 1e-12);
            }
            assert!(recs.windows(2).all(|w| w[0].score >= w[1].score));
        }
    }

    #[test]
    fn weights_are_configurable() {
        let snap = fixture();
        let cfg = FeedConfig {
            weights: Weights::POPULAR_ONLY,
            ..FeedConfig::default()
        };
        let recs = run(&snap, ERIN, &cfg);
        assert!(recs.iter().all(|r| r.breakdown.content == 0.0));
        // Popularity alone: the events with Going intents lead.
        assert!(matches!(recs[0].event_id, id if id == GAME_NIGHT || id == HOOPS));

        let content_only = FeedConfig {
            weights: Weights::CONTENT_ONLY,
            ..FeedConfig::default()
        };
        let recs = run(&snap, ERIN, &content_only);
        assert_eq!(recs[0].event_id, MOVIE);
        assert_eq!(texts(&recs[0]), vec!["Because you like film and art"]);
    }

    #[test]
    fn limit_and_hosted_events() {
        let mut snap = fixture();
        let cfg = FeedConfig {
            limit: 1,
            ..FeedConfig::default()
        };
        assert_eq!(run(&snap, ALICE, &cfg).len(), 1);
        for e in &mut snap.events {
            if e.id == GAME_NIGHT {
                e.host_id = Some(ALICE);
            }
        }
        let recs = run(&snap, ALICE, &FeedConfig::default());
        assert!(recs.iter().all(|r| r.event_id != GAME_NIGHT));
    }

    #[test]
    fn reason_selection() {
        let r = |t: &str| Reason {
            kind: ReasonKind::Content,
            text: t.to_owned(),
        };
        assert_eq!(
            reasons(&[
                (0.4, Some(r("a"))),
                (0.1, Some(r("b"))),
                (0.3, Some(r("c")))
            ]),
            vec![r("a"), r("c")]
        );
        assert_eq!(
            reasons(&[(0.4, Some(r("a"))), (0.01, Some(r("b"))), (0.2, None)]),
            vec![r("a")]
        );
        assert_eq!(reasons(&[(0.0, Some(r("a")))])[0].text, "Coming up soon");
    }

    #[test]
    fn demo_persona_feed_on_seed_data() {
        let now = now();
        let snap = seed::generate(&seed::SeedConfig::demo(now));
        let model = AttendanceModel::train(&snap, now);
        let embedder = HashEmbedder::default();
        let ctx = RecContext {
            snapshot: &snap,
            user: seed::DEMO_USER,
            now,
            model: &model,
            embedder: &embedder,
        };
        let recs = feed(&ctx, &FeedConfig::default());
        assert_eq!(recs.len(), 10);
        let interests = &snap.user(seed::DEMO_USER).unwrap().interests;
        let top = snap.event(recs[0].event_id).unwrap();
        assert!(
            top.tags.iter().any(|t| interests.contains(t)),
            "top pick {} matches an interest",
            top.title
        );
        assert!(
            recs.iter()
                .all(|r| r.reasons.iter().all(|x| !x.text.is_empty()))
        );
        assert!(
            recs.iter()
                .flat_map(|r| &r.reasons)
                .any(|x| x.kind == ReasonKind::Social),
            "friends show up somewhere in the feed"
        );
    }
}
