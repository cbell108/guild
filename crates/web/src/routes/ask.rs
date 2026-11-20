//! "Ask to browse": natural-language search → Browse filters via the local LLM.

use api::{AskRequest, AskResponse, SearchFilters, SearchWhen};
use axum::Json;
use axum::extract::State;
use llm::search::{SearchIntent, SearchWhen as LlmWhen};

use crate::auth::Auth;
use crate::error::{WebError, WebResult};
use crate::state::AppState;

/// Longest request the LLM is asked to interpret.
const MAX_TEXT_CHARS: usize = 300;
/// How many of the most common upcoming tags the model may choose from.
const VOCAB_TAGS: usize = 40;

/// Maps the LLM crate's intent to the wire type.
fn to_response(intent: SearchIntent) -> AskResponse {
    AskResponse {
        filters: SearchFilters {
            when: intent.when.map(|w| match w {
                LlmWhen::Today => SearchWhen::Today,
                LlmWhen::Tomorrow => SearchWhen::Tomorrow,
                LlmWhen::Weekend => SearchWhen::Weekend,
                LlmWhen::Week => SearchWhen::Week,
            }),
            kind: intent.kind,
            tags: intent.tags,
            q: intent.q,
        },
        summary: intent.summary,
    }
}

/// `POST /api/ask`. When the LLM is down, answers 503 `llm_unavailable`; the Browse page then
/// falls back to plain text search.
pub async fn ask(
    State(state): State<AppState>,
    Auth(_uid): Auth,
    Json(req): Json<AskRequest>,
) -> WebResult<Json<AskResponse>> {
    let text = req.text.trim();
    if text.is_empty() || text.chars().count() > MAX_TEXT_CHARS {
        return Err(WebError::bad_request(format!(
            "Ask in 1 to {MAX_TEXT_CHARS} characters."
        )));
    }
    let vocabulary: Vec<String> = state
        .store
        .list_tags()
        .await?
        .into_iter()
        .take(VOCAB_TAGS)
        .map(|(tag, _)| tag)
        .collect();
    match llm::interpret_search(&state.llm, text, state.now(), &vocabulary).await {
        Ok(intent) => Ok(Json(to_response(intent))),
        Err(err) => {
            tracing::warn!("interpret_search failed: {err}");
            Err(WebError::new(
                api::ErrorCode::LlmUnavailable,
                "The assistant isn't available right now. Search by keyword instead.",
            ))
        }
    }
}
