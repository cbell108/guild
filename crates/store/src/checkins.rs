//! Check-ins: proof of attendance at hangouts.

use hangouts_core::{CheckIn, EventId, UserId};

use crate::codec::ts;
use crate::{Result, Store, StoreError};

impl Store {
    /// Record a check-in. Returns `true` if it was newly recorded, `false` if
    /// the user had already checked in.
    pub async fn record_checkin(&self, checkin: &CheckIn) -> Result<bool> {
        let res = sqlx::query(
            "INSERT INTO checkins (user_id, event_id, at) VALUES (?, ?, ?) \
             ON CONFLICT(user_id, event_id) DO NOTHING",
        )
        .bind(checkin.user_id.to_string())
        .bind(checkin.event_id.to_string())
        .bind(ts(checkin.at))
        .execute(&self.pool)
        .await;
        match res {
            Ok(r) => Ok(r.rows_affected() > 0),
            Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
                Err(StoreError::NotFound)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Whether the user has checked in to the event.
    pub async fn has_checked_in(&self, user: UserId, event: EventId) -> Result<bool> {
        let row = sqlx::query("SELECT 1 AS x FROM checkins WHERE user_id = ? AND event_id = ?")
            .bind(user.to_string())
            .bind(event.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }
}
