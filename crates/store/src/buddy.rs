//! Buddy requests: users who want to find a buddy for an event.

use chrono::{DateTime, Utc};
use hangouts_core::{EventId, UserId};

use crate::{Result, Store, StoreError};

impl Store {
    /// Create (or delete, when `looking` is `false`) the user's buddy request
    /// on the event, timestamped `at`.
    pub async fn set_buddy_looking(
        &self,
        user: UserId,
        event: EventId,
        looking: bool,
        at: DateTime<Utc>,
    ) -> Result<()> {
        let _ = (self, user, event, looking, at);
        Err(StoreError::NotImplemented("set_buddy_looking"))
    }

    /// Whether the user currently has a buddy request on the event.
    pub async fn is_buddy_looking(&self, user: UserId, event: EventId) -> Result<bool> {
        let _ = (self, user, event);
        Err(StoreError::NotImplemented("is_buddy_looking"))
    }

    /// Users who are looking for a buddy on the event, oldest request first.
    pub async fn list_buddy_seekers(&self, event: EventId) -> Result<Vec<UserId>> {
        let _ = (self, event);
        Err(StoreError::NotImplemented("list_buddy_seekers"))
    }
}
