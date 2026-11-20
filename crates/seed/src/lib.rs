//! Deterministic demo data: users with personas, crews, friendships, and weeks of intent and
//! check-in history with realistic no-shows. Same config ⇒ same data, byte for byte.
//!
//! [`generate`] is pure (used by `eval` without a database); [`write`] persists a snapshot.

use chrono::{DateTime, Utc};
use hangouts_core::Snapshot;
use store::{Store, StoreError};

/// Generation settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedConfig {
    /// RNG seed (fixed for reproducibility).
    pub rng_seed: u64,
    /// Number of demo users.
    pub users: usize,
    /// Number of crews.
    pub crews: usize,
    /// Weeks of past history before `now`.
    pub weeks_of_history: u32,
    /// The instant history is generated relative to.
    pub now: DateTime<Utc>,
}

impl SeedConfig {
    /// The demo defaults (60 users, 10 crews, 4 weeks) relative to `now`.
    pub fn demo(now: DateTime<Utc>) -> Self {
        Self {
            rng_seed: 2026,
            users: 60,
            crews: 10,
            weeks_of_history: 4,
            now,
        }
    }
}

/// Generates the demo dataset.
pub fn generate(config: &SeedConfig) -> Snapshot {
    // Lane C (C2).
    let _ = config;
    Snapshot::default()
}

/// Writes a generated snapshot through the store (users, events, friendships, crews,
/// intents, check-ins).
#[allow(clippy::unused_async)] // stub until implemented; remove this line then
pub async fn write(store: &Store, snapshot: &Snapshot) -> Result<(), StoreError> {
    // Lane C (C2).
    let _ = (store, snapshot);
    Err(StoreError::NotImplemented("seed::write"))
}
