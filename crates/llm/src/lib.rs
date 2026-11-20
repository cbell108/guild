//! Client for the local OpenAI-compatible LLM server (llama.cpp router) and the
//! natural-language → [`HangoutDraft`] feature.
//!
//! Frozen after Phase 0: [`LlmConfig`], [`LlmClient`], [`parse_hangout`], [`LlmError`].
//! Everything degrades gracefully: callers treat [`LlmError::Unavailable`] and
//! [`LlmError::NotImplemented`] as "feature unavailable" (HTTP 503), and tests never need a
//! running server.

use std::time::Duration;

use chrono::{DateTime, Utc};
use hangouts_core::HangoutDraft;
use thiserror::Error;

/// Default server URL.
pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:8080/v1";
/// Default model id.
pub const DEFAULT_MODEL: &str = "qwen";

/// LLM errors.
#[derive(Debug, Error)]
pub enum LlmError {
    /// Server down, timed out, or returned a non-2xx status.
    #[error("the local LLM is unavailable: {0}")]
    Unavailable(String),
    /// Server answered but the output could not be used.
    #[error("the LLM returned an unusable response: {0}")]
    BadResponse(String),
    /// Not built yet.
    #[error("not implemented yet")]
    NotImplemented,
}

/// Connection settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmConfig {
    /// OpenAI-compatible base URL, ending in `/v1`.
    pub base_url: String,
    /// Model id sent in requests.
    pub model: String,
    /// Per-request timeout.
    pub timeout: Duration,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_owned(),
            model: DEFAULT_MODEL.to_owned(),
            timeout: Duration::from_secs(20),
        }
    }
}

impl LlmConfig {
    /// Reads `LLM_BASE_URL` and `LLM_MODEL`, falling back to the defaults.
    pub fn from_env() -> Self {
        let defaults = Self::default();
        Self {
            base_url: std::env::var("LLM_BASE_URL").unwrap_or(defaults.base_url),
            model: std::env::var("LLM_MODEL").unwrap_or(defaults.model),
            timeout: defaults.timeout,
        }
    }
}

/// A reusable HTTP client for the LLM server.
#[derive(Debug, Clone)]
pub struct LlmClient {
    http: reqwest::Client,
    config: LlmConfig,
}

impl LlmClient {
    /// Builds a client; does not contact the server.
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| LlmError::Unavailable(e.to_string()))?;
        Ok(Self { http, config })
    }

    /// The settings in use.
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// The underlying HTTP client.
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// Whether `GET {base_url}/models` answers with 2xx.
    pub async fn health(&self) -> bool {
        let url = format!("{}/models", self.config.base_url.trim_end_matches('/'));
        matches!(self.http.get(url).send().await, Ok(r) if r.status().is_success())
    }
}

/// Turns a host's description ("smash tonight 7–10 game room, I have 2 switches") into a
/// form prefill. `now` resolves relative dates ("tonight", "friday") in campus-local time.
/// The draft is returned [normalized](HangoutDraft::normalized).
#[allow(clippy::unused_async)] // stub until implemented; remove this line then
pub async fn parse_hangout(
    client: &LlmClient,
    text: &str,
    now: DateTime<Utc>,
) -> Result<HangoutDraft, LlmError> {
    // Lane C (C9): JSON-schema-constrained chat completion.
    let _ = (client, text, now);
    Err(LlmError::NotImplemented)
}
