//! Features ported from the `DiscordSchedulerBot` prototype: availability,
//! gear and presence.

use chrono::{DateTime, NaiveDate, Utc};
use hangouts_core::time::{format_date, local_date};
use hangouts_core::{Availability, Gear, Presence, PresenceReport, Setup, UserId, Window};
use sqlx::Row;

use crate::codec::{id_col, ts, ts_col};
use crate::{Result, Store, StoreError};

impl Store {
    /// Replace any availability window the user has on the same campus-local
    /// start date (`hangouts_core::time::local_date(window.start())`), like
    /// the prototype.
    pub async fn set_availability(&self, availability: &Availability) -> Result<()> {
        let w = &availability.window;
        let res = sqlx::query(
            "INSERT INTO availability (user_id, local_date, starts_at, ends_at) VALUES (?, ?, ?, ?) \
             ON CONFLICT(user_id, local_date) DO UPDATE SET starts_at = excluded.starts_at, \
             ends_at = excluded.ends_at",
        )
        .bind(availability.user_id.to_string())
        .bind(format_date(local_date(w.start())))
        .bind(ts(w.start()))
        .bind(ts(w.end()))
        .execute(&self.pool)
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
                Err(StoreError::NotFound)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Availability windows overlapping the campus-local `date`
    /// (`hangouts_core::Window::overlaps_local_date`), optionally only for
    /// `users`.
    pub async fn list_availability_on(
        &self,
        date: NaiveDate,
        users: Option<&[UserId]>,
    ) -> Result<Vec<Availability>> {
        let rows = sqlx::query("SELECT * FROM availability ORDER BY starts_at, user_id")
            .fetch_all(&self.pool)
            .await?;
        let mut out = Vec::new();
        for row in &rows {
            let user_id: UserId = id_col(row, "user_id")?;
            if users.is_some_and(|us| !us.contains(&user_id)) {
                continue;
            }
            let window = Window::new(ts_col(row, "starts_at")?, ts_col(row, "ends_at")?)
                .map_err(|e| StoreError::Invalid(e.to_string()))?;
            if window.overlaps_local_date(date) {
                out.push(Availability { user_id, window });
            }
        }
        Ok(out)
    }

    /// Record a gear declaration. Idempotent per (user, setup, game).
    pub async fn add_gear(&self, gear: &Gear) -> Result<()> {
        let res = sqlx::query(
            "INSERT INTO gear (user_id, setup, game) VALUES (?, ?, ?) ON CONFLICT DO NOTHING",
        )
        .bind(gear.user_id.to_string())
        .bind(gear.setup.as_str())
        .bind(&gear.game)
        .execute(&self.pool)
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
                Err(StoreError::NotFound)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// All gear declarations.
    pub async fn list_gear(&self) -> Result<Vec<Gear>> {
        let rows = sqlx::query("SELECT * FROM gear ORDER BY user_id, setup, game")
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|r| {
                Ok(Gear {
                    user_id: id_col(r, "user_id")?,
                    setup: Setup::parse(&r.try_get::<String, _>("setup")?)
                        .map_err(|e| StoreError::Invalid(e.to_string()))?,
                    game: r.try_get("game")?,
                })
            })
            .collect()
    }

    /// Record the user's latest self-reported presence at `at`.
    pub async fn set_presence(
        &self,
        user: UserId,
        presence: Presence,
        at: DateTime<Utc>,
    ) -> Result<()> {
        let res = sqlx::query(
            "INSERT INTO presence (user_id, presence, at) VALUES (?, ?, ?) \
             ON CONFLICT(user_id) DO UPDATE SET presence = excluded.presence, at = excluded.at",
        )
        .bind(user.to_string())
        .bind(presence.as_str())
        .bind(ts(at))
        .execute(&self.pool)
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
                Err(StoreError::NotFound)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// The latest self-reported presence of every user, excluding `Out`,
    /// newest first.
    pub async fn list_presence(&self) -> Result<Vec<PresenceReport>> {
        let rows =
            sqlx::query("SELECT * FROM presence WHERE presence <> 'out' ORDER BY at DESC, user_id")
                .fetch_all(&self.pool)
                .await?;
        rows.iter()
            .map(|r| {
                Ok(PresenceReport {
                    user_id: id_col(r, "user_id")?,
                    presence: Presence::parse(&r.try_get::<String, _>("presence")?)
                        .ok_or_else(|| StoreError::Invalid("bad presence".into()))?,
                    at: ts_col(r, "at")?,
                })
            })
            .collect()
    }
}
