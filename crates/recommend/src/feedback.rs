//! Implicit feedback: how strongly each user engaged with each event
//! (check-in 3, Going / Going with 2, Interested 1; the strongest signal wins).
//!
//! Users who turned personalization off contribute nothing: their history is not used to
//! recommend to anyone, including themselves.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use hangouts_core::{EventId, IntentKind, Snapshot, UserId};

/// Weight of a check-in.
pub const CHECKIN_WEIGHT: f64 = 3.0;
/// Weight of a Going / Going-with intent.
pub const GOING_WEIGHT: f64 = 2.0;
/// Weight of an Interested intent.
pub const INTERESTED_WEIGHT: f64 = 1.0;

/// user → (event → weight), from intents made and check-ins recorded before `now`.
pub type Feedback = BTreeMap<UserId, BTreeMap<EventId, f64>>;

/// Builds the user → event weight table.
pub fn implicit_feedback(snapshot: &Snapshot, now: DateTime<Utc>) -> Feedback {
    let opted_out: Vec<UserId> = snapshot
        .users
        .iter()
        .filter(|u| !u.privacy.personalize)
        .map(|u| u.id)
        .collect();
    let mut out: Feedback = BTreeMap::new();
    let mut put = |user: UserId, event: EventId, w: f64| {
        if opted_out.contains(&user) {
            return;
        }
        let slot = out.entry(user).or_default().entry(event).or_insert(0.0);
        *slot = slot.max(w);
    };
    for i in snapshot.intents.iter().filter(|i| i.created_at < now) {
        let w = match i.level.kind() {
            IntentKind::Interested => INTERESTED_WEIGHT,
            IntentKind::Going | IntentKind::GoingWith => GOING_WEIGHT,
        };
        put(i.user_id, i.event_id, w);
    }
    for c in snapshot.checkins.iter().filter(|c| c.at < now) {
        put(c.user_id, c.event_id, CHECKIN_WEIGHT);
    }
    out
}

/// Cosine similarity of two sparse vectors.
pub fn sparse_cosine<K: Ord>(a: &BTreeMap<K, f64>, b: &BTreeMap<K, f64>) -> f64 {
    let (small, large) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    let dot: f64 = small
        .iter()
        .filter_map(|(k, x)| large.get(k).map(|y| x * y))
        .sum();
    let na: f64 = a.values().map(|x| x * x).sum::<f64>().sqrt();
    let nb: f64 = b.values().map(|x| x * x).sum::<f64>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[test]
    fn strongest_signal_wins_and_opt_out_is_respected() {
        let mut snap = fixture();
        let fb = implicit_feedback(&snap, now());
        assert!((fb[&BOB][&HOOPS_PAST] - CHECKIN_WEIGHT).abs() < 1e-9);
        assert!((fb[&ERIN][&MOVIE] - INTERESTED_WEIGHT).abs() < 1e-9);
        assert!((fb[&ALICE][&GAMES_PAST] - GOING_WEIGHT).abs() < 1e-9);
        for u in &mut snap.users {
            u.privacy.personalize = u.id != BOB;
        }
        assert!(!implicit_feedback(&snap, now()).contains_key(&BOB));
    }

    #[test]
    fn cosine_of_sparse_maps() {
        let a = BTreeMap::from([(1, 1.0), (2, 1.0)]);
        let b = BTreeMap::from([(2, 1.0), (3, 1.0)]);
        assert!((sparse_cosine(&a, &b) - 0.5).abs() < 1e-9);
        assert!(sparse_cosine(&a, &BTreeMap::new()).abs() < 1e-9);
    }
}
