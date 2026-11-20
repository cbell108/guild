//! Features ported from the `DiscordSchedulerBot` prototype: availability,
//! gear and presence.

use chrono::{DateTime, NaiveDate, Utc};
use hangouts_core::{Availability, Gear, Presence, PresenceReport, UserId};

use crate::{Result, Store, StoreError};

impl Store {
    /// Replace any availability window the user has on the same campus-local
    /// start date (`hangouts_core::time::local_date(window.start())`), like
    /// the prototype.
    pub async fn set_availability(&self, availability: &Availability) -> Result<()> {
        let _ = (self, availability);
        Err(StoreError::NotImplemented("set_availability"))
    }

    /// Availability windows overlapping the campus-local `date`
    /// (`hangouts_core::Window::overlaps_local_date`), optionally only for
    /// `users`.
    pub async fn list_availability_on(
        &self,
        date: NaiveDate,
        users: Option<&[UserId]>,
    ) -> Result<Vec<Availability>> {
        let _ = (self, date, users);
        Err(StoreError::NotImplemented("list_availability_on"))
    }

    /// Record a gear declaration. Idempotent per (user, setup, game).
    pub async fn add_gear(&self, gear: &Gear) -> Result<()> {
        let _ = (self, gear);
        Err(StoreError::NotImplemented("add_gear"))
    }

    /// All gear declarations.
    pub async fn list_gear(&self) -> Result<Vec<Gear>> {
        let _ = self;
        Err(StoreError::NotImplemented("list_gear"))
    }

    /// Record the user's latest self-reported presence at `at`.
    pub async fn set_presence(
        &self,
        user: UserId,
        presence: Presence,
        at: DateTime<Utc>,
    ) -> Result<()> {
        let _ = (self, user, presence, at);
        Err(StoreError::NotImplemented("set_presence"))
    }

    /// The latest self-reported presence of every user, excluding `Out`,
    /// newest first.
    pub async fn list_presence(&self) -> Result<Vec<PresenceReport>> {
        let _ = self;
        Err(StoreError::NotImplemented("list_presence"))
    }
}
