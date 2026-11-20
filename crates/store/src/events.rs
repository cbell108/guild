//! Events: official KSU calendar events and user-created hangouts.

use chrono::{DateTime, Utc};
use hangouts_core::{Event, EventId, EventKind};

use crate::{Result, Store, StoreError};

/// Filter for [`Store::list_events`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventFilter {
    /// Only events of this kind; `None` matches any kind.
    pub kind: Option<EventKind>,
    /// The event must carry at least one of these tags (any-of match).
    pub tags: Vec<String>,
    /// Start of the window: events whose time range overlaps `[from, to)`.
    pub from: Option<DateTime<Utc>>,
    /// End of the window (exclusive).
    pub to: Option<DateTime<Utc>>,
    /// Case-insensitive substring match on the title or description.
    pub query: Option<String>,
    /// Maximum number of events to return (default 100).
    pub limit: Option<u32>,
}

impl Store {
    /// Insert an event. Returns [`StoreError::Conflict`] if the id or the
    /// `external_id` already exists.
    pub async fn insert_event(&self, event: &Event) -> Result<()> {
        let _ = (self, event);
        Err(StoreError::NotImplemented("insert_event"))
    }

    /// Insert or update an official event keyed by `external_id`, keeping the
    /// id of the existing row (which is returned).
    /// Returns [`StoreError::Invalid`] if `external_id` is `None`.
    pub async fn upsert_official_event(&self, event: &Event) -> Result<EventId> {
        let _ = (self, event);
        Err(StoreError::NotImplemented("upsert_official_event"))
    }

    /// One event by id.
    pub async fn get_event(&self, id: EventId) -> Result<Option<Event>> {
        let _ = (self, id);
        Err(StoreError::NotImplemented("get_event"))
    }

    /// Events matching `filter`, ordered by `starts_at`.
    pub async fn list_events(&self, filter: &EventFilter) -> Result<Vec<Event>> {
        let _ = (self, filter);
        Err(StoreError::NotImplemented("list_events"))
    }

    /// Tags used by upcoming events with usage counts, most common first.
    pub async fn list_tags(&self) -> Result<Vec<(String, u32)>> {
        let _ = self;
        Err(StoreError::NotImplemented("list_tags"))
    }

    /// Store an event's embedding as a little-endian `f32` BLOB.
    pub async fn set_event_embedding(&self, event: EventId, embedding: &[f32]) -> Result<()> {
        let _ = (self, event, embedding);
        Err(StoreError::NotImplemented("set_event_embedding"))
    }

    /// Events that have no stored embedding yet.
    pub async fn events_missing_embedding(&self) -> Result<Vec<Event>> {
        let _ = self;
        Err(StoreError::NotImplemented("events_missing_embedding"))
    }
}
