//! Natural-language hosting: turn free text into a hangout draft with the local LLM.

use api::ParseRequest;
use axum::Json;
use axum::extract::State;
use hangouts_core::HangoutDraft;

use crate::auth::Auth;
use crate::error::{WebError, WebResult};
use crate::state::AppState;

/// Longest description the LLM is asked to parse.
const MAX_TEXT_CHARS: usize = 1000;

/// `POST /api/parse`. When the LLM is down or not built, answers 503 `llm_unavailable` with a
/// message the Host page can show next to its manual form.
pub async fn parse(
    State(state): State<AppState>,
    Auth(_uid): Auth,
    Json(req): Json<ParseRequest>,
) -> WebResult<Json<HangoutDraft>> {
    let text = req.text.trim();
    if text.is_empty() || text.chars().count() > MAX_TEXT_CHARS {
        return Err(WebError::bad_request(format!(
            "Describe your hangout in 1 to {MAX_TEXT_CHARS} characters."
        )));
    }
    match llm::parse_hangout(&state.llm, text, state.now()).await {
        Ok(draft) => Ok(Json(draft)),
        Err(err) => {
            // Unavailable and BadResponse both degrade the same way.
            tracing::warn!("parse_hangout failed: {err}");
            Err(WebError::new(
                api::ErrorCode::LlmUnavailable,
                "The assistant isn't available right now. Fill in the form by hand instead.",
            ))
        }
    }
}
