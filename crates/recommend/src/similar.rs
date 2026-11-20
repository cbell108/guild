//! Similar students, for buddy matching.

use hangouts_core::{Reason, ReasonKind, UserId};

use crate::RecContext;

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
pub fn similar_users(ctx: &RecContext<'_>, candidates: &[UserId], k: usize) -> Vec<UserMatch> {
    // Lane C (C6) replaces this placeholder: candidates in the given order, unscored.
    candidates
        .iter()
        .copied()
        .filter(|u| *u != ctx.user)
        .filter(|u| {
            ctx.snapshot
                .user(*u)
                .is_none_or(|x| x.privacy.buddy_matching)
        })
        .take(k)
        .map(|user_id| UserMatch {
            user_id,
            score: 0.0,
            reasons: vec![Reason {
                kind: ReasonKind::Social,
                text: "Also looking for someone to go with".to_owned(),
            }],
        })
        .collect()
}
