//! Hybrid event recommender: content + collaborative + social (+ popularity), each part
//! explainable. **DB-free**: callers pass a [`hangouts_core::Snapshot`] in and get results out.
//!
//! Frozen after Phase 0 (other lanes call these): [`RecContext`], [`FeedConfig`], [`Weights`],
//! [`feed`], [`similar_users`], [`UserMatch`], [`AttendanceModel`], [`AttendanceFeatures`],
//! [`Embedder`], [`HashEmbedder`], [`default_embedder`], [`event_text`], [`RecommendError`].
//! Lane C may add modules and pub items but must not change those signatures.

mod embed;
mod error;
mod feed;
mod model;
mod similar;

pub use embed::{Embedder, HashEmbedder, default_embedder, event_text};
pub use error::RecommendError;
pub use feed::{FeedConfig, Weights, feed};
pub use model::{AttendanceFeatures, AttendanceModel};
pub use similar::{UserMatch, similar_users};

use chrono::{DateTime, Utc};
use hangouts_core::{Snapshot, UserId};

/// Everything a recommendation call needs.
pub struct RecContext<'a> {
    /// All app data (see `store::Store::load_snapshot` / `seed::generate`).
    pub snapshot: &'a Snapshot,
    /// The viewer.
    pub user: UserId,
    /// "Now", injected for deterministic tests and offline evaluation.
    pub now: DateTime<Utc>,
    /// Attendance-probability model (train once at startup, or [`AttendanceModel::prior`]).
    pub model: &'a AttendanceModel,
    /// Text embedder for events missing from `snapshot.event_embeddings` and for interests.
    pub embedder: &'a dyn Embedder,
}
