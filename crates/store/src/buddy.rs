//! Buddy requests: users who want to find a buddy for an event.

use chrono::{DateTime, Utc};
use hangouts_core::{EventId, UserId};
use sqlx::Row;

use crate::codec::{parse_id, ts};
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
        if looking {
            let res = sqlx::query(
                "INSERT INTO buddy_requests (user_id, event_id, created_at) VALUES (?, ?, ?) \
                 ON CONFLICT(user_id, event_id) DO NOTHING",
            )
            .bind(user.to_string())
            .bind(event.to_string())
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
        } else {
            sqlx::query("DELETE FROM buddy_requests WHERE user_id = ? AND event_id = ?")
                .bind(user.to_string())
                .bind(event.to_string())
                .execute(&self.pool)
                .await?;
            Ok(())
        }
    }

    /// Whether the user currently has a buddy request on the event.
    pub async fn is_buddy_looking(&self, user: UserId, event: EventId) -> Result<bool> {
        let row =
            sqlx::query("SELECT 1 AS x FROM buddy_requests WHERE user_id = ? AND event_id = ?")
                .bind(user.to_string())
                .bind(event.to_string())
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.is_some())
    }

    /// Users who are looking for a buddy on the event, oldest request first.
    pub async fn list_buddy_seekers(&self, event: EventId) -> Result<Vec<UserId>> {
        let rows = sqlx::query(
            "SELECT user_id FROM buddy_requests WHERE event_id = ? ORDER BY created_at, user_id",
        )
        .bind(event.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| parse_id(&r.try_get::<String, _>("user_id")?))
            .collect()
    }
}
