//! Errors for the store crate.

/// Errors that can occur in store operations.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The requested row (user, event, crew, ...) does not exist.
    #[error("not found")]
    NotFound,

    /// The write would duplicate or otherwise conflict with existing data.
    #[error("conflict: {0}")]
    Conflict(String),

    /// The caller supplied data that violates an invariant.
    #[error("invalid data: {0}")]
    Invalid(String),

    /// A database-level failure.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// A migration failure.
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
}
