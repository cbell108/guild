//! SQLite repository for campus-hangouts.
//!
//! The public API below is stable: the web server, bot, seed and eval code
//! against these signatures.
//!
//! Implementation notes:
//! - Only the runtime [`sqlx::query`] API is used. The compile-time `query!`
//!   macros are deliberately avoided so that building this crate never requires
//!   a `DATABASE_URL`.
//! - Every write takes an explicit `at: DateTime<Utc>` so the `seed` crate can
//!   backdate history deterministically.
//!
//! The schema lives in `migrations/` and is applied with [`Store::migrate`].

pub mod buddy;
pub mod checkins;
mod codec;
pub mod crews;
pub mod error;
pub mod events;
pub mod friends;
pub mod intents;
pub mod prototype;
pub mod snapshot;
pub mod users;

pub use error::StoreError;
pub use events::EventFilter;
pub use intents::RemoveOutcome;

/// Result type used by all store operations.
pub type Result<T> = std::result::Result<T, StoreError>;

/// Handle to the campus-hangouts SQLite database.
///
/// Cheap to clone (it wraps a connection pool). All operations are async.
#[derive(Debug, Clone)]
pub struct Store {
    pool: sqlx::SqlitePool,
}

impl Store {
    /// Open a pool to the SQLite database at `url`, creating the database
    /// file if it does not exist yet.
    ///
    /// Foreign keys are enforced; the pool holds up to 5 connections.
    pub async fn connect(url: &str) -> Result<Self> {
        let options = url
            .parse::<sqlx::sqlite::SqliteConnectOptions>()
            .map_err(|err| StoreError::Invalid(format!("invalid sqlite url {url:?}: {err}")))?
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = sqlx::pool::PoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    /// Open a pool to a single in-memory database (`sqlite::memory:`).
    ///
    /// The pool holds exactly one connection (min 1, max 1) with no idle
    /// timeout and no max lifetime, so the in-memory database stays alive for
    /// as long as the pool does.
    pub async fn in_memory() -> Result<Self> {
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .in_memory(true)
            .foreign_keys(true);
        let pool = sqlx::pool::PoolOptions::new()
            .max_connections(1)
            .min_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    /// Apply the SQL migrations in `migrations/`.
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("../../migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Borrow the underlying connection pool.
    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    // Test code is allowed to panic on failure.
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// An in-memory store migrates cleanly.
    #[tokio::test]
    async fn in_memory_store_migrates() {
        Store::in_memory().await.unwrap().migrate().await.unwrap();
    }
}
