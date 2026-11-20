//! Popularity: campus-wide engagement with an event so far (Going counts 2, Interested 1),
//! log-scaled and normalized so the most popular candidate scores 1. Only the reason text
//! ever reaches users; counts are never shown.

use std::collections::BTreeMap;

use hangouts_core::{Event, EventId, IntentStatus, Reason, ReasonKind};

use crate::RecContext;
use crate::content::Component;

/// Normalized popularity at or above this earns the "Popular" reason.
pub const POPULAR_REASON_THRESHOLD: f64 = 0.6;

/// Popularity of each candidate, keyed by event id.
pub fn popularity(ctx: &RecContext<'_>, candidates: &[&Event]) -> BTreeMap<EventId, Component> {
    let raw: Vec<(EventId, f64)> = candidates
        .iter()
        .map(|e| {
            let points: f64 = ctx
                .snapshot
                .intents_for_event(e.id)
                .filter(|i| i.created_at < ctx.now)
                .map(|i| match (i.level.uses_spot(), i.status) {
                    (true, IntentStatus::Confirmed) => 2.0,
                    _ => 1.0,
                })
                .sum();
            (e.id, points.ln_1p())
        })
        .collect();
    let max = raw.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max);
    raw.into_iter()
        .map(|(id, v)| {
            let value = if max > 0.0 { v / max } else { 0.0 };
            let reason = (value >= POPULAR_REASON_THRESHOLD).then(|| Reason {
                kind: ReasonKind::Popular,
                text: "Popular on campus this week".to_owned(),
            });
            (id, Component { value, reason })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::{AttendanceModel, HashEmbedder};

    #[test]
    fn most_engaged_event_scores_one() {
        let mut snap = fixture();
        snap.intents.push(intent(
            ALICE,
            GAME_NIGHT,
            hangouts_core::IntentLevel::Interested,
            1,
        ));
        let model = AttendanceModel::prior();
        let embedder = HashEmbedder::default();
        let ctx = ctx(&snap, BOB, &model, &embedder);
        let cands: Vec<&Event> = [GAME_NIGHT, HOOPS, MOVIE]
            .iter()
            .map(|id| snap.event(*id).unwrap())
            .collect();
        let p = popularity(&ctx, &cands);
        assert!((p[&GAME_NIGHT].value - 1.0).abs() < 1e-9);
        assert!(p[&GAME_NIGHT].reason.is_some());
        assert!(p[&HOOPS].value < 1.0 && p[&HOOPS].value > p[&MOVIE].value);
    }
}
