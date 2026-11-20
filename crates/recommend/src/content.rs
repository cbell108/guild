//! Content-based scorer: how well an event's text matches the viewer's interests and the
//! events they actually went to.
//!
//! Profile = mean of the embeddings of each interest tag and of each attended event
//! (checked in, or Going); score = cosine(profile, event embedding), floored at 0.

use std::collections::{BTreeMap, BTreeSet};

use hangouts_core::{Event, EventId, Reason, ReasonKind, Snapshot, UserId};

use crate::RecContext;
use crate::embed::{Embedder, cosine, event_text, mean_normalized, try_embed};

/// Event embeddings for one recommendation call: stored ones when their length matches the
/// embedder, the rest embedded in a single batch.
pub struct EmbeddingCache {
    vectors: BTreeMap<EventId, Vec<f32>>,
}

impl EmbeddingCache {
    /// Embeds every event in `events` that has no usable stored embedding.
    pub fn new<'e>(
        snapshot: &Snapshot,
        embedder: &dyn Embedder,
        events: impl IntoIterator<Item = &'e Event>,
    ) -> Self {
        let dim = embedder.dim();
        let mut vectors = BTreeMap::new();
        let mut missing: Vec<&Event> = Vec::new();
        for event in events {
            if vectors.contains_key(&event.id) {
                continue;
            }
            match snapshot.event_embeddings.get(&event.id) {
                Some(v) if v.len() == dim => {
                    vectors.insert(event.id, v.clone());
                }
                _ => {
                    if !missing.iter().any(|e| e.id == event.id) {
                        missing.push(event);
                    }
                }
            }
        }
        let texts: Vec<String> = missing.iter().map(|e| event_text(e)).collect();
        if let Some(embedded) = try_embed(embedder, &texts) {
            for (event, v) in missing.into_iter().zip(embedded) {
                vectors.insert(event.id, v);
            }
        }
        Self { vectors }
    }

    /// The vector for `event`, if it could be embedded.
    pub fn get(&self, event: EventId) -> Option<&[f32]> {
        self.vectors.get(&event).map(Vec::as_slice)
    }
}

/// One component score in `0..=1` plus the sentence explaining it (if any).
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    /// Unweighted score, `0..=1`.
    pub value: f64,
    /// Plain-English explanation, when the component has something to say.
    pub reason: Option<Reason>,
}

impl Component {
    /// A zero score with no reason.
    pub const ZERO: Self = Self {
        value: 0.0,
        reason: None,
    };
}

/// Events `user` engaged with strongly: checked in, or Going / Going with (before `now`).
pub fn attended_events(ctx: &RecContext<'_>, user: UserId) -> BTreeSet<EventId> {
    let mut out: BTreeSet<EventId> = ctx
        .snapshot
        .checkins_for_user(user)
        .filter(|c| c.at < ctx.now)
        .map(|c| c.event_id)
        .collect();
    out.extend(
        ctx.snapshot
            .intents_for_user(user)
            .filter(|i| i.level.uses_spot() && i.created_at < ctx.now)
            .map(|i| i.event_id),
    );
    out
}

/// Interest tag as readable words: `"board-games"` → `"board games"`.
pub fn tag_words(tag: &str) -> String {
    tag.replace('-', " ")
}

/// The viewer's content profile, built once per feed call.
pub struct ContentScorer {
    profile: Option<Vec<f32>>,
    interests: Vec<String>,
    /// (title, vector) of attended events, for "similar to X" reasons.
    history: Vec<(String, Vec<f32>)>,
    cache: EmbeddingCache,
}

/// Cosine above which "Similar to X, which you went to" is used as the reason.
const SIMILAR_EVENT_THRESHOLD: f64 = 0.5;

impl ContentScorer {
    /// Builds the profile for `ctx.user` and embeds `candidates`. History is ignored when
    /// the viewer turned personalization off (interests only).
    pub fn new(ctx: &RecContext<'_>, candidates: &[&Event]) -> Self {
        let user = ctx.snapshot.user(ctx.user);
        let interests: Vec<String> = user.map(|u| u.interests.clone()).unwrap_or_default();
        let personalize = user.is_none_or(|u| u.privacy.personalize);
        let attended: Vec<&Event> = if personalize {
            attended_events(ctx, ctx.user)
                .into_iter()
                .filter_map(|id| ctx.snapshot.event(id))
                .filter(|e| e.starts_at < ctx.now)
                .collect()
        } else {
            Vec::new()
        };
        let cache = EmbeddingCache::new(
            ctx.snapshot,
            ctx.embedder,
            candidates.iter().copied().chain(attended.iter().copied()),
        );
        let interest_texts: Vec<String> = interests.iter().map(|t| tag_words(t)).collect();
        let interest_vecs = try_embed(ctx.embedder, &interest_texts).unwrap_or_default();
        let history: Vec<(String, Vec<f32>)> = attended
            .iter()
            .filter_map(|e| cache.get(e.id).map(|v| (e.title.clone(), v.to_vec())))
            .collect();
        let all: Vec<&[f32]> = interest_vecs
            .iter()
            .map(Vec::as_slice)
            .chain(history.iter().map(|(_, v)| v.as_slice()))
            .collect();
        Self {
            profile: mean_normalized(&all),
            interests,
            history,
            cache,
        }
    }

    /// Whether the viewer has any content signal (interests or history).
    pub fn has_profile(&self) -> bool {
        self.profile.is_some()
    }

    /// Scores one candidate.
    pub fn score(&self, event: &Event) -> Component {
        let (Some(profile), Some(vector)) = (&self.profile, self.cache.get(event.id)) else {
            return Component::ZERO;
        };
        let value = cosine(profile, vector).max(0.0);
        if value <= 0.0 {
            return Component::ZERO;
        }
        Component {
            value,
            reason: Some(self.reason(event, vector)),
        }
    }

    fn reason(&self, event: &Event, vector: &[f32]) -> Reason {
        let matched: Vec<String> = event
            .tags
            .iter()
            .filter(|t| self.interests.contains(t))
            .take(2)
            .map(|t| tag_words(t))
            .collect();
        let text = if !matched.is_empty() {
            format!("Because you like {}", matched.join(" and "))
        } else if let Some(title) = self
            .history
            .iter()
            .filter(|(title, _)| *title != event.title)
            .map(|(title, v)| (title, cosine(v, vector)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .filter(|(_, sim)| *sim >= SIMILAR_EVENT_THRESHOLD)
            .map(|(title, _)| title)
        {
            format!("Similar to {title}, which you went to")
        } else if let Some((title, _)) =
            self.history.iter().find(|(title, _)| *title == event.title)
        {
            format!("You've been to {title} before")
        } else {
            "Matches your interests".to_owned()
        };
        Reason {
            kind: ReasonKind::Content,
            text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[test]
    fn interests_rank_matching_events_first() {
        let snap = fixture();
        let model = crate::AttendanceModel::prior();
        let embedder = crate::HashEmbedder::default();
        let ctx = ctx(&snap, ALICE, &model, &embedder);
        let games = snap.event(GAME_NIGHT).unwrap();
        let hoops = snap.event(HOOPS).unwrap();
        let scorer = ContentScorer::new(&ctx, &[games, hoops]);
        assert!(scorer.has_profile());
        let g = scorer.score(games);
        let h = scorer.score(hoops);
        assert!(g.value > h.value + 0.2, "{g:?} vs {h:?}");
        assert_eq!(
            g.reason.unwrap().text,
            "Because you like board games and tabletop"
        );
    }

    #[test]
    fn attended_history_shapes_profile_and_reason() {
        let snap = fixture();
        let model = crate::AttendanceModel::prior();
        let embedder = crate::HashEmbedder::default();
        // Bob has no interests but went to a past basketball event.
        let ctx = ctx(&snap, BOB, &model, &embedder);
        let hoops = snap.event(HOOPS).unwrap();
        let games = snap.event(GAME_NIGHT).unwrap();
        let scorer = ContentScorer::new(&ctx, &[hoops, games]);
        let h = scorer.score(hoops);
        assert!(h.value > scorer.score(games).value);
        assert_eq!(
            h.reason.unwrap().text,
            "You've been to Pickup Basketball before"
        );
    }

    #[test]
    fn personalization_off_ignores_history() {
        let mut snap = fixture();
        for u in &mut snap.users {
            if u.id == BOB {
                u.privacy.personalize = false;
            }
        }
        let model = crate::AttendanceModel::prior();
        let embedder = crate::HashEmbedder::default();
        let ctx = ctx(&snap, BOB, &model, &embedder);
        let hoops = snap.event(HOOPS).unwrap();
        let scorer = ContentScorer::new(&ctx, &[hoops]);
        assert!(!scorer.has_profile());
        assert_eq!(scorer.score(hoops), Component::ZERO);
    }

    #[test]
    fn stored_embeddings_are_used_when_dims_match() {
        let mut snap = fixture();
        let embedder = crate::HashEmbedder::new(4);
        snap.event_embeddings
            .insert(GAME_NIGHT, vec![1.0, 0.0, 0.0, 0.0]);
        snap.event_embeddings.insert(HOOPS, vec![1.0, 0.0]); // wrong dim: re-embedded
        let games = snap.event(GAME_NIGHT).unwrap();
        let hoops = snap.event(HOOPS).unwrap();
        let cache = EmbeddingCache::new(&snap, &embedder, [games, hoops]);
        assert_eq!(cache.get(GAME_NIGHT), Some(&[1.0, 0.0, 0.0, 0.0][..]));
        assert_eq!(cache.get(HOOPS).map(<[f32]>::len), Some(4));
    }
}
