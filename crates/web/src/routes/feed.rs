//! The "For you" feed.

use api::{FeedItem, FeedResponse};
use axum::Json;
use axum::extract::State;
use recommend::{FeedConfig, RecContext};

use crate::auth::Auth;
use crate::error::WebResult;
use crate::state::AppState;
use crate::view;

/// `GET /api/feed`: the top picks for the next 7 days; always ends with `caught_up`.
pub async fn feed(State(state): State<AppState>, Auth(uid): Auth) -> WebResult<Json<FeedResponse>> {
    let snap = state.store.load_snapshot().await?;
    let now = state.now();
    let recs = {
        let model = state
            .model
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let ctx = RecContext {
            snapshot: &snap,
            user: uid,
            now,
            model: &model,
            embedder: state.embedder.as_ref(),
        };
        recommend::feed(&ctx, &FeedConfig::default())
    };
    let items = recs
        .into_iter()
        .filter_map(|rec| {
            let event = snap.event(rec.event_id)?;
            Some(FeedItem {
                event: view::card(&snap, event, Some(uid)),
                score: rec.score,
                reasons: rec.reasons,
                breakdown: rec.breakdown,
            })
        })
        .collect();
    Ok(Json(FeedResponse {
        items,
        caught_up: true,
    }))
}
