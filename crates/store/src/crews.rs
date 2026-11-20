//! Crews: named groups of users with per-member roles.

use chrono::{DateTime, Utc};
use hangouts_core::{Crew, CrewId, CrewMember, CrewRole, UserId};

use crate::{Result, Store, StoreError};

impl Store {
    /// Insert a crew and add `crew.created_by` as its Owner, with
    /// `joined_at = crew.created_at`.
    pub async fn insert_crew(&self, crew: &Crew) -> Result<()> {
        let _ = (self, crew);
        Err(StoreError::NotImplemented("insert_crew"))
    }

    /// One crew by id.
    pub async fn get_crew(&self, id: CrewId) -> Result<Option<Crew>> {
        let _ = (self, id);
        Err(StoreError::NotImplemented("get_crew"))
    }

    /// Crews the user belongs to, ordered by name.
    pub async fn list_crews_for_user(&self, user: UserId) -> Result<Vec<Crew>> {
        let _ = (self, user);
        Err(StoreError::NotImplemented("list_crews_for_user"))
    }

    /// Add (or update) a member with `role`, joined at `at`. Idempotent.
    /// Returns [`StoreError::NotFound`] if the crew or user is missing.
    pub async fn add_crew_member(
        &self,
        crew: CrewId,
        user: UserId,
        role: CrewRole,
        at: DateTime<Utc>,
    ) -> Result<()> {
        let _ = (self, crew, user, role, at);
        Err(StoreError::NotImplemented("add_crew_member"))
    }

    /// Remove a member from the crew. Returns `true` if one existed.
    pub async fn remove_crew_member(&self, crew: CrewId, user: UserId) -> Result<bool> {
        let _ = (self, crew, user);
        Err(StoreError::NotImplemented("remove_crew_member"))
    }

    /// All members of the crew, ordered by `joined_at`.
    pub async fn list_crew_members(&self, crew: CrewId) -> Result<Vec<CrewMember>> {
        let _ = (self, crew);
        Err(StoreError::NotImplemented("list_crew_members"))
    }
}
