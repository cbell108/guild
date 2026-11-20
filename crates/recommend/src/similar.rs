//! Similar students, for buddy matching.
//!
//! Similarity = [`INTEREST_SHARE`] × Jaccard(interest tags) + the rest × cosine of the two
//! students' implicit-feedback vectors (what they went to). Students who turned off
//! personalization are compared on interests only.

use std::collections::BTreeSet;

use hangouts_core::{Reason, ReasonKind, UserId, Visibility};

use crate::RecContext;
use crate::content::tag_words;
use crate::feedback::{GOING_WEIGHT, implicit_feedback, sparse_cosine};

/// Share of the similarity that comes from interests (the rest is co-attendance).
pub const INTEREST_SHARE: f64 = 0.5;

/// A candidate buddy with a similarity score.
#[derive(Debug, Clone, PartialEq)]
pub struct UserMatch {
    /// Candidate.
    pub user_id: UserId,
    /// Similarity (higher is better).
    pub score: f64,
    /// Why they match, most important first; at least one.
    pub reasons: Vec<Reason>,
}

/// Ranks `candidates` (excluding `ctx.user`) by interest and co-attendance similarity to
/// `ctx.user`, best first, at most `k`. Candidates who turned off buddy matching in their
/// privacy settings must already be excluded by the caller or are skipped here.
/// Candidates whose attendance is private are compared on interests only: their history
/// never affects the score or appears in a reason ("been to the same events").
///
/// Duplicates in `candidates` are ignored; ties break by user id.
pub fn similar_users(ctx: &RecContext<'_>, candidates: &[UserId], k: usize) -> Vec<UserMatch> {
    let snapshot = ctx.snapshot;
    let feedback = implicit_feedback(snapshot, ctx.now);
    let empty = std::collections::BTreeMap::default();
    let mine = feedback.get(&ctx.user).unwrap_or(&empty);
    let my_interests: BTreeSet<&String> = snapshot
        .user(ctx.user)
        .map(|u| u.interests.iter().collect())
        .unwrap_or_default();

    let mut seen = BTreeSet::new();
    let mut out: Vec<UserMatch> = candidates
        .iter()
        .copied()
        .filter(|u| *u != ctx.user && seen.insert(*u))
        .filter(|u| snapshot.user(*u).is_none_or(|x| x.privacy.buddy_matching))
        .map(|user_id| {
            let private = snapshot
                .user(user_id)
                .is_some_and(|u| u.privacy.attendance == Visibility::Private);
            let theirs = if private {
                &empty
            } else {
                feedback.get(&user_id).unwrap_or(&empty)
            };
            let their_interests: BTreeSet<&String> = snapshot
                .user(user_id)
                .map(|u| u.interests.iter().collect())
                .unwrap_or_default();
            let shared: Vec<&String> = my_interests
                .intersection(&their_interests)
                .copied()
                .collect();
            let union = my_interests.union(&their_interests).count();
            let jaccard = if union == 0 {
                0.0
            } else {
                shared.len() as f64 / union as f64
            };
            let co_attendance = sparse_cosine(mine, theirs);
            let together = mine
                .iter()
                .filter(|(e, w)| {
                    **w >= GOING_WEIGHT && theirs.get(*e).is_some_and(|t| *t >= GOING_WEIGHT)
                })
                .count();
            UserMatch {
                user_id,
                score: INTEREST_SHARE * jaccard + (1.0 - INTEREST_SHARE) * co_attendance,
                reasons: match_reasons(&shared, together),
            }
        })
        .collect();
    out.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.user_id.cmp(&b.user_id)));
    out.truncate(k);
    out
}

fn match_reasons(shared: &[&String], together: usize) -> Vec<Reason> {
    let mut reasons = Vec::new();
    if !shared.is_empty() {
        let words: Vec<String> = shared.iter().take(2).map(|t| tag_words(t)).collect();
        reasons.push(Reason {
            kind: ReasonKind::Content,
            text: format!("You both like {}", words.join(" and ")),
        });
    }
    if together > 0 {
        reasons.push(Reason {
            kind: ReasonKind::Collaborative,
            text: if together == 1 {
                "You've both been to the same event before".to_owned()
            } else {
                format!("You've both been to {together} of the same events")
            },
        });
    }
    if reasons.is_empty() {
        reasons.push(Reason {
            kind: ReasonKind::Social,
            text: "Also looking for someone to go with".to_owned(),
        });
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::{AttendanceModel, HashEmbedder};

    fn texts(m: &UserMatch) -> Vec<&str> {
        m.reasons.iter().map(|r| r.text.as_str()).collect()
    }

    #[test]
    fn ranks_by_interests_and_co_attendance() {
        let snap = fixture();
        let model = AttendanceModel::prior();
        let embedder = HashEmbedder::default();
        let ctx = ctx(&snap, ALICE, &model, &embedder);
        let m = similar_users(&ctx, &[ERIN, BOB, CARL, ALICE, CARL], 10);
        let ids: Vec<UserId> = m.iter().map(|x| x.user_id).collect();
        assert_eq!(
            ids[0], CARL,
            "shares board games and went to the same night"
        );
        assert!(!ids.contains(&ALICE), "never matches yourself");
        assert_eq!(ids.len(), 3, "duplicates dropped");
        assert_eq!(
            texts(&m[0]),
            vec![
                "You both like board games",
                "You've both been to the same event before"
            ]
        );
        assert_eq!(texts(&m[2]), vec!["Also looking for someone to go with"]);
        assert!(m.windows(2).all(|w| w[0].score >= w[1].score));
    }

    #[test]
    fn private_attendance_never_shows_in_score_or_reasons() {
        let mut snap = fixture();
        for u in &mut snap.users {
            if u.id == CARL {
                u.privacy.attendance = Visibility::Private;
            }
        }
        let model = AttendanceModel::prior();
        let embedder = HashEmbedder::default();
        let private_ctx = ctx(&snap, ALICE, &model, &embedder);
        let m = similar_users(&private_ctx, &[CARL], 1);
        assert_eq!(texts(&m[0]), vec!["You both like board games"]);
        let public_snap = fixture();
        let public_ctx = ctx(&public_snap, ALICE, &model, &embedder);
        let public = similar_users(&public_ctx, &[CARL], 1);
        assert!(m[0].score < public[0].score, "co-attendance must not count");
    }

    #[test]
    fn respects_buddy_opt_out_and_k() {
        let mut snap = fixture();
        for u in &mut snap.users {
            if u.id == CARL {
                u.privacy.buddy_matching = false;
            }
        }
        let model = AttendanceModel::prior();
        let embedder = HashEmbedder::default();
        let ctx = ctx(&snap, ALICE, &model, &embedder);
        let m = similar_users(&ctx, &[CARL, BOB, ERIN], 1);
        assert_eq!(m.len(), 1);
        assert_ne!(m[0].user_id, CARL);
    }

    #[test]
    fn seed_crewmates_are_most_similar() {
        let now = now();
        let snap = seed::generate(&seed::SeedConfig::demo(now));
        let model = AttendanceModel::prior();
        let embedder = HashEmbedder::default();
        let ctx = RecContext {
            snapshot: &snap,
            user: seed::DEMO_USER,
            now,
            model: &model,
            embedder: &embedder,
        };
        let everyone: Vec<UserId> = snap.users.iter().map(|u| u.id).collect();
        let top = similar_users(&ctx, &everyone, 5);
        let my_crews: BTreeSet<_> = snap
            .crew_members
            .iter()
            .filter(|m| m.user_id == seed::DEMO_USER)
            .map(|m| m.crew_id)
            .collect();
        let crewmates = top
            .iter()
            .filter(|m| {
                snap.crew_members
                    .iter()
                    .any(|c| c.user_id == m.user_id && my_crews.contains(&c.crew_id))
            })
            .count();
        assert!(crewmates >= 3, "{crewmates} of top 5 share a crew");
    }
}
