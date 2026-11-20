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

/// The `.env.example` placeholder, treated like an unset `QR_SECRET`.
const QR_SECRET_PLACEHOLDER: &str = "change-me-to-a-random-string";

fn qr_secret_from_env() -> String {
    match std::env::var("QR_SECRET") {
        Ok(s) if !s.trim().is_empty() && s != QR_SECRET_PLACEHOLDER => s,
        _ => {
            tracing::warn!("QR_SECRET is not set; using a random key for this process");
            format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            )
        }
    }
}

impl AppState {
    /// State with production defaults: default embedder, prior attendance model, system clock,
    /// `static/` directory and the `QR_SECRET` env var. Without `QR_SECRET` (or with the
    /// `.env.example` placeholder) a random per-process key is used, so check-in tokens can't
    /// be forged from a value published in the repo; they then stop working on restart.
    pub fn new(store: Store, llm: Arc<LlmClient>) -> Self {
        let secret = qr_secret_from_env();
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

    /// Retrains the attendance model on the current database contents.
    pub async fn retrain_model(&self) -> Result<(), store::StoreError> {
        let snapshot = self.store.load_snapshot().await?;
        let trained = AttendanceModel::train(&snapshot, self.now());
        *self
            .model
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = trained;
        Ok(())
    }

    /// The current time.
    pub fn now(&self) -> DateTime<Utc> {
        (self.clock)()
    }
}
