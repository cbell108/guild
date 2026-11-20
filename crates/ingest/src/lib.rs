//! Imports official events from the KSU public calendar API
//! (`calendar.kennesaw.edu/api/2/events`, Localist). One [`hangouts_core::Event`] per event
//! instance, upserted by `external_id = "ksu:<instance id>"`.

use chrono::{DateTime, Utc};
use hangouts_core::Event;
use store::{Store, StoreError};
use thiserror::Error;

/// Default API endpoint.
pub const KSU_EVENTS_URL: &str = "https://calendar.kennesaw.edu/api/2/events";

/// Ingest errors.
#[derive(Debug, Error)]
pub enum IngestError {
    /// Network / HTTP failure.
    #[error("fetch failed: {0}")]
    Http(String),
    /// Unexpected JSON.
    #[error("parse failed: {0}")]
    Parse(String),
    /// Database failure.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// Not built yet.
    #[error("not implemented yet")]
    NotImplemented,
}

/// Ingest settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestConfig {
    /// API endpoint.
    pub url: String,
    /// Days ahead to import.
    pub days: u32,
    /// Page size.
    pub per_page: u32,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            url: KSU_EVENTS_URL.to_owned(),
            days: 30,
            per_page: 100,
        }
    }
}

/// What a run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IngestReport {
    /// Event instances fetched.
    pub fetched: usize,
    /// Rows inserted or updated.
    pub upserted: usize,
}

/// Maps one API response page (raw JSON body) to events. Pure; tested against a fixture.
pub fn map_page(body: &str, now: DateTime<Utc>) -> Result<Vec<Event>, IngestError> {
    // Lane C (C1).
    let _ = (body, now);
    Err(IngestError::NotImplemented)
}

/// Fetches every page and upserts the events.
#[allow(clippy::unused_async)] // stub until implemented; remove this line then
pub async fn run(store: &Store, config: &IngestConfig) -> Result<IngestReport, IngestError> {
    // Lane C (C1).
    let _ = (store, config);
    Err(IngestError::NotImplemented)
}
