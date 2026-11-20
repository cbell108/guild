//! Keeping event embeddings in the database for the content recommender.

use hangouts_core::Event;

use crate::state::AppState;

/// Embeds one event and stores the vector. Failures are logged, never fatal: the recommender
/// degrades gracefully without embeddings.
pub async fn embed_event(state: &AppState, event: &Event) {
    let text = recommend::event_text(event);
    match state.embedder.embed(&[text]) {
        Ok(mut vectors) => {
            if let Some(vector) = vectors.pop()
                && let Err(err) = state.store.set_event_embedding(event.id, &vector).await
            {
                tracing::warn!("could not store embedding for {}: {err}", event.id);
            }
        }
        Err(err) => tracing::warn!("could not embed {}: {err}", event.id),
    }
}

/// Embeds every event that has no stored embedding yet; returns how many were embedded.
pub async fn backfill(state: &AppState) -> usize {
    let events = match state.store.events_missing_embedding().await {
        Ok(events) => events,
        Err(err) => {
            tracing::warn!("could not list events missing embeddings: {err}");
            return 0;
        }
    };
    for event in &events {
        embed_event(state, event).await;
    }
    events.len()
}
