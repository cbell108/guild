//! Social scorer: friends who are going, plus how likely the viewer is to actually show up
//! (the [`AttendanceModel`](crate::AttendanceModel)).
//!
//! Privacy: only accepted friends count, and friends whose attendance is private are never
//! counted or named.

use std::collections::{BTreeMap, BTreeSet};

use hangouts_core::{
    Event, EventId, IntentKind, IntentStatus, Reason, ReasonKind, UserId, Visibility,
};

use crate::content::Component;
use crate::{AttendanceFeatures, RecContext, model};

/// Share of the social score that comes from friends (the rest is attendance probability).
pub const FRIEND_SHARE: f64 = 0.6;
/// Each friend Going closes this fraction of the remaining gap to 1 (Interested counts half).
const FRIEND_STEP: f64 = 0.4;

/// Per-viewer social signals, built once per feed call.
pub struct SocialScorer<'a> {
    ctx: &'a RecContext<'a>,
    /// Visible accepted friends.
    friends: BTreeSet<UserId>,
    own: BTreeMap<EventId, IntentKind>,
    show_rate: f64,
}

/// Friends of the viewer who are going to / interested in one event, in name order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FriendPresence {
    /// Display names of friends Going (confirmed).
    pub going: Vec<String>,
    /// Display names of friends Interested.
    pub interested: Vec<String>,
}

impl<'a> SocialScorer<'a> {
    /// Collects the viewer's visible friends, own intents and show rate.
    pub fn new(ctx: &'a RecContext<'a>) -> Self {
        let snapshot = ctx.snapshot;
        let personalize = snapshot
            .user(ctx.user)
            .is_none_or(|u| u.privacy.personalize);
        let friends = snapshot
            .friends_of(ctx.user)
            .into_iter()
            .filter(|f| {
                snapshot
                    .user(*f)
                    .is_none_or(|u| u.privacy.attendance != Visibility::Private)
            })
            .collect();
        let own = snapshot
            .intents_for_user(ctx.user)
            .filter(|i| i.created_at < ctx.now)
            .map(|i| (i.event_id, i.level.kind()))
            .collect();
        let show_rate = if personalize {
            model::past_show_rate(snapshot, ctx.user, ctx.now, None)
        } else {
            0.5
        };
        Self {
            ctx,
            friends,
            own,
            show_rate,
        }
    }

    /// Which visible friends are going to or interested in `event`.
    pub fn presence(&self, event: EventId) -> FriendPresence {
        let mut going: Vec<String> = Vec::new();
        let mut interested: Vec<String> = Vec::new();
        for i in self.ctx.snapshot.intents_for_event(event) {
            if !self.friends.contains(&i.user_id) || i.created_at >= self.ctx.now {
                continue;
            }
            let Some(user) = self.ctx.snapshot.user(i.user_id) else {
                continue;
            };
            if i.level.uses_spot() {
                if i.status == IntentStatus::Confirmed {
                    going.push(user.display_name.clone());
                }
            } else {
                interested.push(user.display_name.clone());
            }
        }
        going.sort();
        interested.sort();
        FriendPresence { going, interested }
    }

    /// P(viewer checks in) if they RSVP at their current level (Interested if none yet).
    pub fn attendance_probability(&self, event: &Event, friends_going: usize) -> f64 {
        let days_ahead =
            (event.starts_at - self.ctx.now).num_minutes().max(0) as f64 / (24.0 * 60.0);
        self.ctx.model.predict(&AttendanceFeatures {
            level: self
                .own
                .get(&event.id)
                .copied()
                .unwrap_or(IntentKind::Interested),
            days_ahead,
            friends_going: friends_going as u32,
            past_show_rate: self.show_rate,
        })
    }

    /// Scores one candidate.
    pub fn score(&self, event: &Event) -> Component {
        let presence = self.presence(event.id);
        let weight = presence.going.len() as f64 + 0.5 * presence.interested.len() as f64;
        let friend_part = 1.0 - (1.0 - FRIEND_STEP).powf(weight);
        let p = self.attendance_probability(event, presence.going.len());
        Component {
            value: FRIEND_SHARE * friend_part + (1.0 - FRIEND_SHARE) * p,
            reason: friend_reason(&presence).map(|text| Reason {
                kind: ReasonKind::Social,
                text,
            }),
        }
    }
}

/// "Jordan is going", "Jordan + 2 friends are going", "Maya is interested", or `None`.
pub fn friend_reason(p: &FriendPresence) -> Option<String> {
    match (p.going.as_slice(), p.interested.as_slice()) {
        ([], []) => None,
        ([one], _) => Some(format!("{one} is going")),
        ([first, rest @ ..], _) => Some(format!(
            "{first} + {} friend{} are going",
            rest.len(),
            if rest.len() == 1 { "" } else { "s" }
        )),
        ([], [one]) => Some(format!("{one} is interested")),
        ([], [first, rest @ ..]) => Some(format!(
            "{first} + {} friend{} are interested",
            rest.len(),
            if rest.len() == 1 { "" } else { "s" }
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::{AttendanceModel, HashEmbedder};

    #[test]
    fn friend_going_boosts_and_explains() {
        let snap = fixture();
        let model = AttendanceModel::prior();
        let embedder = HashEmbedder::default();
        let ctx = ctx(&snap, ALICE, &model, &embedder);
        let s = SocialScorer::new(&ctx);
        let games = s.score(snap.event(GAME_NIGHT).unwrap());
        let movie = s.score(snap.event(MOVIE).unwrap());
        assert!(games.value > movie.value + 0.2);
        assert_eq!(games.reason.unwrap().text, "Carl is going");
        assert_eq!(movie.reason, None);
    }

    #[test]
    fn private_friends_are_never_named_or_counted() {
        let snap = fixture();
        let model = AttendanceModel::prior();
        let embedder = HashEmbedder::default();
        let ctx = ctx(&snap, ALICE, &model, &embedder);
        let s = SocialScorer::new(&ctx);
        assert_eq!(s.presence(HOOPS), FriendPresence::default());
        assert_eq!(s.score(snap.event(HOOPS).unwrap()).reason, None);
    }

    #[test]
    fn reason_wording() {
        let p = |g: &[&str], i: &[&str]| FriendPresence {
            going: g.iter().map(|s| (*s).to_owned()).collect(),
            interested: i.iter().map(|s| (*s).to_owned()).collect(),
        };
        assert_eq!(friend_reason(&p(&[], &[])), None);
        assert_eq!(
            friend_reason(&p(&["Jordan", "Maya", "Sam"], &["Eli"])).as_deref(),
            Some("Jordan + 2 friends are going")
        );
        assert_eq!(
            friend_reason(&p(&["Jordan", "Maya"], &[])).as_deref(),
            Some("Jordan + 1 friend are going")
        );
        assert_eq!(
            friend_reason(&p(&[], &["Eli"])).as_deref(),
            Some("Eli is interested")
        );
    }

    #[test]
    fn own_commitment_raises_attendance_probability() {
        let mut snap = fixture();
        let model = AttendanceModel::prior();
        let embedder = HashEmbedder::default();
        let movie = snap.event(MOVIE).unwrap().clone();
        let before = {
            let ctx = ctx(&snap, ALICE, &model, &embedder);
            SocialScorer::new(&ctx).attendance_probability(&movie, 0)
        };
        snap.intents
            .push(intent(ALICE, MOVIE, hangouts_core::IntentLevel::Going, 1));
        let ctx = ctx(&snap, ALICE, &model, &embedder);
        let after = SocialScorer::new(&ctx).attendance_probability(&movie, 0);
        assert!(after > before);
    }
}
