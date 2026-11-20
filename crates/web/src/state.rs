//! Shared application state.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use llm::LlmClient;
use recommend::{AttendanceModel, Embedder};
use store::Store;

/// Source of "now", replaceable in tests.
pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// Shared state for handlers.
#[derive(Clone)]
pub struct AppState {
    /// Database.
    pub store: Store,
    /// Local LLM client.
    pub llm: Arc<LlmClient>,
    /// Embeds event text for the content recommender.
    pub embedder: Arc<dyn Embedder>,
    /// Attendance model trained at startup.
    pub model: Arc<RwLock<AttendanceModel>>,
    /// Secret for check-in QR tokens.
    pub qr_secret: Arc<Vec<u8>>,
    /// Directory served at `/`.
    pub static_dir: PathBuf,
    /// Clock.
    pub clock: Clock,
}

impl AppState {
    /// State with production defaults: default embedder, prior attendance model, system clock,
    /// `static/` directory and the `QR_SECRET` env var (a fixed dev fallback if unset).
    pub fn new(store: Store, llm: Arc<LlmClient>) -> Self {
        let secret = std::env::var("QR_SECRET").unwrap_or_else(|_| "dev-only-qr-secret".to_owned());
        Self {
            store,
            llm,
            embedder: Arc::from(recommend::default_embedder()),
            model: Arc::new(RwLock::new(AttendanceModel::prior())),
            qr_secret: Arc::new(secret.into_bytes()),
            static_dir: PathBuf::from("static"),
            clock: Arc::new(Utc::now),
        }
    }

    /// The current time.
    pub fn now(&self) -> DateTime<Utc> {
        (self.clock)()
    }
}
