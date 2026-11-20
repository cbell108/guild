//! Recommender errors.

use thiserror::Error;

/// Errors from the recommender's fallible parts (embedding).
#[derive(Debug, Error)]
pub enum RecommendError {
    /// The embedding backend failed (model missing, ONNX error, ...).
    #[error("embedding failed: {0}")]
    Embedding(String),
    /// Not built yet.
    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),
}
