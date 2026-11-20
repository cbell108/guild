//! Intents to attend: each user's level per event and the resulting status.

use chrono::{DateTime, Utc};
use hangouts_core::{EventId, Intent, IntentLevel, UserId};

use crate::{Result, Store, StoreError};

/// Outcome of [`Store::remove_intent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RemoveOutcome {
    /// `true` if an intent existed and was removed.
    pub removed: bool,
    /// The waitlisted user who received the freed spot, if any.
    pub promoted: Option<UserId>,
}

impl Store {
    /// In one transaction: decide the intent's status with
    /// [`hangouts_core::admission`] (excluding the user's own current spot),
    /// upsert it keeping its `created_at`, and return the stored intent.
    /// Returns [`StoreError::NotFound`] if the event does not exist.
    pub async fn set_intent(
        &self,
        user: UserId,
        event: EventId,
        level: IntentLevel,
        at: DateTime<Utc>,
    ) -> Result<Intent> {
        let _ = (self, user, event, level, at);
        Err(StoreError::NotImplemented("set_intent"))
    }

    /// One user's intent on an event, if any.
    pub async fn get_intent(&self, user: UserId, event: EventId) -> Result<Option<Intent>> {
        let _ = (self, user, event);
        Err(StoreError::NotImplemented("get_intent"))
    }

    /// In one transaction: delete the intent; if it held a confirmed spot,
    /// promote [`hangouts_core::next_to_promote`] to confirmed with
    /// `updated_at = at`.
    pub async fn remove_intent(
        &self,
        user: UserId,
        event: EventId,
        at: DateTime<Utc>,
    ) -> Result<RemoveOutcome> {
        let _ = (self, user, event, at);
        Err(StoreError::NotImplemented("remove_intent"))
    }

    /// All intents on an event, ordered by `created_at`.
    pub async fn list_intents_for_event(&self, event: EventId) -> Result<Vec<Intent>> {
        let _ = (self, event);
        Err(StoreError::NotImplemented("list_intents_for_event"))
    }

    /// All intents of a user, ordered by `created_at`.
    pub async fn list_intents_for_user(&self, user: UserId) -> Result<Vec<Intent>> {
        let _ = (self, user);
        Err(StoreError::NotImplemented("list_intents_for_user"))
    }

    /// The user's 1-based position among waitlisted intents on the event,
    /// or `None` if the user is not waitlisted.
    pub async fn waitlist_position(&self, user: UserId, event: EventId) -> Result<Option<u32>> {
        let _ = (self, user, event);
        Err(StoreError::NotImplemented("waitlist_position"))
    }
}
