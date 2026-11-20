//! Check-ins: proof of attendance at hangouts.

use hangouts_core::{CheckIn, EventId, UserId};

use crate::{Result, Store, StoreError};

impl Store {
    /// Record a check-in. Returns `true` if it was newly recorded, `false` if
    /// the user had already checked in.
    pub async fn record_checkin(&self, checkin: &CheckIn) -> Result<bool> {
        let _ = (self, checkin);
        Err(StoreError::NotImplemented("record_checkin"))
    }

    /// Whether the user has checked in to the event.
    pub async fn has_checked_in(&self, user: UserId, event: EventId) -> Result<bool> {
        let _ = (self, user, event);
        Err(StoreError::NotImplemented("has_checked_in"))
    }
}
