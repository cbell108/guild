//! User accounts, interest tags and privacy settings.

use chrono::{DateTime, Utc};
use hangouts_core::{PrivacySettings, User, UserId};

use crate::{Result, Store, StoreError};

impl Store {
    /// Insert a new user. Returns [`StoreError::Conflict`] if the id exists.
    pub async fn insert_user(&self, user: &User) -> Result<()> {
        let _ = (self, user);
        Err(StoreError::NotImplemented("insert_user"))
    }

    /// One user by id.
    pub async fn get_user(&self, id: UserId) -> Result<Option<User>> {
        let _ = (self, id);
        Err(StoreError::NotImplemented("get_user"))
    }

    /// All users, ordered by `display_name`.
    pub async fn list_users(&self) -> Result<Vec<User>> {
        let _ = self;
        Err(StoreError::NotImplemented("list_users"))
    }

    /// Users with `is_demo`, ordered by `display_name`.
    pub async fn list_demo_users(&self) -> Result<Vec<User>> {
        let _ = self;
        Err(StoreError::NotImplemented("list_demo_users"))
    }

    /// Case-insensitive substring match on `display_name`, at most `limit` hits.
    pub async fn search_users(&self, query: &str, limit: u32) -> Result<Vec<User>> {
        let _ = (self, query, limit);
        Err(StoreError::NotImplemented("search_users"))
    }

    /// Find a user by `discord_id`, or create a non-demo user with default
    /// privacy at `at` if none exists; updates `display_name`.
    pub async fn upsert_discord_user(
        &self,
        discord_id: &str,
        display_name: &str,
        at: DateTime<Utc>,
    ) -> Result<User> {
        let _ = (self, discord_id, display_name, at);
        Err(StoreError::NotImplemented("upsert_discord_user"))
    }

    /// Replace the user's interest tags (the caller normalizes them).
    /// Returns [`StoreError::NotFound`] if there is no such user.
    pub async fn set_interests(&self, user: UserId, interests: &[String]) -> Result<()> {
        let _ = (self, user, interests);
        Err(StoreError::NotImplemented("set_interests"))
    }

    /// Replace the user's privacy settings.
    /// Returns [`StoreError::NotFound`] if there is no such user.
    pub async fn set_privacy(&self, user: UserId, privacy: &PrivacySettings) -> Result<()> {
        let _ = (self, user, privacy);
        Err(StoreError::NotImplemented("set_privacy"))
    }

    /// Delete the user's intents, check-ins, buddy requests and availability
    /// windows. Does not delete the account, friendships or crew membership.
    pub async fn delete_history(&self, user: UserId) -> Result<()> {
        let _ = (self, user);
        Err(StoreError::NotImplemented("delete_history"))
    }
}
