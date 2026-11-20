//! Item-item collaborative scorer over implicit feedback ([`crate::feedback`]).
//!
//! Each event is a vector of user weights (check-in 3, Going 2, Interested 1). A candidate's
//! score is the viewer's weighted mean similarity between it and the events in their
//! history. The viewer is left out of every item vector, so their own RSVP to a candidate
//! never makes it look similar to their history ("leave-viewer-out").

use std::collections::BTreeMap;

use hangouts_core::{Event, EventId, Reason, ReasonKind, UserId};

use crate::RecContext;
use crate::content::Component;
use crate::feedback::{implicit_feedback, sparse_cosine};

/// Similarity below this does not earn the "people who go to X" reason.
pub const REASON_MIN_SIMILARITY: f64 = 0.15;

/// Per-viewer item-item scorer, built once per feed call.
pub struct CollabScorer {
    /// event → (user → weight), viewer excluded.
    items: BTreeMap<EventId, BTreeMap<UserId, f64>>,
    /// The viewer's history: (event, weight, title).
    history: Vec<(EventId, f64, String)>,
}

impl CollabScorer {
    /// Builds the scorer for `ctx.user`. Viewers who turned personalization off have no
    /// history here, so every score is zero.
    pub fn new(ctx: &RecContext<'_>) -> Self {
        let feedback = implicit_feedback(ctx.snapshot, ctx.now);
        let mut items: BTreeMap<EventId, BTreeMap<UserId, f64>> = BTreeMap::new();
        for (user, events) in &feedback {
            if *user == ctx.user {
                continue;
            }
            for (event, w) in events {
                items.entry(*event).or_default().insert(*user, *w);
            }
        }
        let history = feedback
            .get(&ctx.user)
            .map(|events| {
                events
                    .iter()
                    .filter_map(|(id, w)| {
                        ctx.snapshot.event(*id).map(|e| (*id, *w, e.title.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self { items, history }
    }

    /// Item-item similarity (viewer excluded); 0 if either event has no other engagement.
    pub fn similarity(&self, a: EventId, b: EventId) -> f64 {
        match (self.items.get(&a), self.items.get(&b)) {
            (Some(x), Some(y)) => sparse_cosine(x, y),
            _ => 0.0,
        }
    }

    /// Scores one candidate in `0..=1`.
    pub fn score(&self, event: &Event) -> Component {
        let mut total_weight = 0.0;
        let mut sum = 0.0;
        let mut best: Option<(f64, &str)> = None;
        for (id, w, title) in &self.history {
            if *id == event.id {
                continue;
            }
            total_weight += w;
            let sim = self.similarity(event.id, *id);
            sum += w * sim;
            if sim > best.map_or(0.0, |(s, _)| s) {
                best = Some((sim, title.as_str()));
            }
        }
        if total_weight == 0.0 || sum <= 0.0 {
            return Component::ZERO;
        }
        let reason = best
            .filter(|(sim, _)| *sim >= REASON_MIN_SIMILARITY)
            .map(|(_, title)| Reason {
                kind: ReasonKind::Collaborative,
                text: if title == event.title {
                    format!("Regulars at {title} are going")
                } else {
                    format!("People who go to {title} also go to this")
                },
            });
        Component {
            value: (sum / total_weight).clamp(0.0, 1.0),
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::{AttendanceModel, HashEmbedder};
    use hangouts_core::IntentLevel;

    #[test]
    fn co_engaged_events_score_and_explain() {
        // Carl went to the past game night with Alice and is going to the next one.
        let mut snap = fixture();
        snap.events.push(event(
            EventId::from_u128(40),
            "Movie Marathon",
            &["film"],
            -10,
        ));
        snap.intents
            .push(intent(BOB, EventId::from_u128(40), IntentLevel::Going, 12));
        let model = AttendanceModel::prior();
        let embedder = HashEmbedder::default();
        let ctx = ctx(&snap, ALICE, &model, &embedder);
        let cf = CollabScorer::new(&ctx);
        let games = cf.score(snap.event(GAME_NIGHT).unwrap());
        assert!((games.value - 1.0).abs() < 1e-9, "{games:?}");
        assert_eq!(
            games.reason.unwrap().text,
            "Regulars at Tabletop Tuesday are going"
        );
        // Nobody who shares Alice's history goes to hoops or the movie.
        assert_eq!(cf.score(snap.event(HOOPS).unwrap()), Component::ZERO);
        assert_eq!(cf.score(snap.event(MOVIE).unwrap()), Component::ZERO);
    }

    #[test]
    fn viewer_is_left_out_of_item_vectors() {
        let mut snap = fixture();
        // Alice RSVPs to hoops herself; that alone must not make hoops look like her history.
        snap.intents
            .push(intent(ALICE, HOOPS, IntentLevel::Going, 1));
        let model = AttendanceModel::prior();
        let embedder = HashEmbedder::default();
        let ctx = ctx(&snap, ALICE, &model, &embedder);
        let cf = CollabScorer::new(&ctx);
        assert!(cf.similarity(HOOPS, GAMES_PAST).abs() < 1e-12);
        assert_eq!(cf.score(snap.event(HOOPS).unwrap()), Component::ZERO);
    }

    #[test]
    fn no_history_means_zero() {
        let snap = fixture();
        let model = AttendanceModel::prior();
        let embedder = HashEmbedder::default();
        let ctx = ctx(&snap, UserId::from_u128(999), &model, &embedder);
        let cf = CollabScorer::new(&ctx);
        assert_eq!(cf.score(snap.event(GAME_NIGHT).unwrap()), Component::ZERO);
    }
}
