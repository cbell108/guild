//! Client for the local OpenAI-compatible LLM server (llama.cpp router) and the
//! natural-language → [`HangoutDraft`] feature.
//!
//! Public surface: [`LlmConfig`], [`LlmClient`], [`parse_hangout`], [`interpret_search`],
//! [`LlmError`].
//! Everything degrades gracefully: callers treat [`LlmError::Unavailable`] and
//! [`LlmError::BadResponse`] as "feature unavailable" (HTTP 503), and tests never need a
//! running server.

pub mod hangout;
pub mod search;

use std::time::Duration;

use chrono::{DateTime, Utc};
use hangouts_core::HangoutDraft;
use thiserror::Error;

/// Default server URL.
pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:8080/v1";
/// Default model id: the Q3 preset resident on the RX 9070 XT in the router's split layout.
/// Parsing a hangout is a small task (153/153 on `tests/phrases.json`), so it never needs to
/// evict the split layout for the cross-GPU Q6 model. Override with `LLM_MODEL`.
pub const DEFAULT_MODEL: &str = "qwen3.8-27B-UD-Q3_K_XL:9070XT";

/// LLM errors.
#[derive(Debug, Error)]
pub enum LlmError {
    /// Server down, timed out, or returned a non-2xx status.
    #[error("the local LLM is unavailable: {0}")]
    Unavailable(String),
    /// Server answered but the output could not be used.
    #[error("the LLM returned an unusable response: {0}")]
    BadResponse(String),
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
///
/// Blank input returns an empty draft without calling the server.
pub async fn parse_hangout(
    client: &LlmClient,
    text: &str,
    now: DateTime<Utc>,
) -> Result<HangoutDraft, LlmError> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(HangoutDraft::default());
    }
    let url = format!(
        "{}/chat/completions",
        client.config.base_url.trim_end_matches('/')
    );
    let request = hangout::request(&client.config.model, text, now);
    let response = client
        .http
        .post(url)
        .json(&request)
        .send()
        .await
        .map_err(|e| LlmError::Unavailable(e.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(LlmError::Unavailable(format!("server returned {status}")));
    }
    let body = response
        .text()
        .await
        .map_err(|e| LlmError::Unavailable(e.to_string()))?;
    hangout::parse_response(&body).map(|draft| hangout::refine(draft, text, now))
}

/// Turns a request ("something chill tonight with food") into Browse filters plus a one-line
/// summary. `known_tags` is the vocabulary the model may choose tags from (the result is
/// restricted to it). Blank input returns an empty intent without calling the server.
pub async fn interpret_search(
    client: &LlmClient,
    text: &str,
    now: DateTime<Utc>,
    known_tags: &[String],
) -> Result<search::SearchIntent, LlmError> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(search::SearchIntent::default());
    }
    let url = format!(
        "{}/chat/completions",
        client.config.base_url.trim_end_matches('/')
    );
    let request = search::request(&client.config.model, text, now, known_tags);
    let response = client
        .http
        .post(url)
        .json(&request)
        .send()
        .await
        .map_err(|e| LlmError::Unavailable(e.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(LlmError::Unavailable(format!("server returned {status}")));
    }
    let body = response
        .text()
        .await
        .map_err(|e| LlmError::Unavailable(e.to_string()))?;
    search::parse_response(&body, known_tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(base_url: &str) -> LlmClient {
        LlmClient::new(LlmConfig {
            base_url: base_url.to_owned(),
            model: "test".to_owned(),
            timeout: Duration::from_secs(2),
        })
        .unwrap()
    }

    #[tokio::test]
    async fn blank_input_needs_no_server() {
        let c = client("http://127.0.0.1:9/v1");
        let d = parse_hangout(&c, "   ", Utc::now()).await.unwrap();
        assert_eq!(d, HangoutDraft::default());
    }

    #[tokio::test]
    async fn server_down_is_unavailable() {
        // Port 9 (discard) is closed on any dev machine: the connection is refused fast.
        let c = client("http://127.0.0.1:9/v1");
        assert!(!c.health().await);
        let err = parse_hangout(&c, "smash tonight", Utc::now()).await;
        assert!(matches!(err, Err(LlmError::Unavailable(_))), "{err:?}");
    }

    #[tokio::test]
    async fn interpret_blank_needs_no_server() {
        let c = client("http://127.0.0.1:9/v1");
        let i = interpret_search(&c, " ", Utc::now(), &[]).await.unwrap();
        assert_eq!(i, search::SearchIntent::default());
    }

    #[tokio::test]
    async fn interpret_server_down_is_unavailable() {
        let c = client("http://127.0.0.1:9/v1");
        let err = interpret_search(&c, "free food tonight", Utc::now(), &[]).await;
        assert!(matches!(err, Err(LlmError::Unavailable(_))), "{err:?}");
    }

    /// Live check against the local router (`cargo test -p llm -- --ignored`).
    #[tokio::test]
    #[ignore = "needs the local LLM server"]
    async fn live_interpret_search() {
        let c = LlmClient::new(LlmConfig::from_env()).unwrap();
        let known: Vec<String> = ["food", "free-food", "music", "sports"]
            .map(String::from)
            .to_vec();
        let i = interpret_search(&c, "free food tonight", Utc::now(), &known)
            .await
            .unwrap();
        assert_eq!(i.when, Some(search::SearchWhen::Today), "{i:?}");
        assert!(i.tags.iter().any(|t| t.contains("food")), "{i:?}");
    }
}
