//! Intents to attend: each user's level per event and the resulting status.

use chrono::{DateTime, Utc};
use hangouts_core::{
    EventId, Intent, IntentLevel, IntentStatus, UserId, admission, next_to_promote,
};
use sqlx::{Row, SqliteConnection};

use crate::codec::{intent_from_row, ts};
use crate::{Result, Store, StoreError};

/// Outcome of [`Store::remove_intent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RemoveOutcome {
    /// `true` if an intent existed and was removed.
    pub removed: bool,
    /// The waitlisted user who received the freed spot, if any.
    pub promoted: Option<UserId>,
}

/// Promote the earliest waitlisted intent on the event, if any.
async fn promote_next(
    conn: &mut SqliteConnection,
    event: EventId,
    at: DateTime<Utc>,
) -> Result<Option<UserId>> {
    let rows = sqlx::query("SELECT * FROM intents WHERE event_id = ? AND status = 'waitlisted'")
        .bind(event.to_string())
        .fetch_all(&mut *conn)
        .await?;
    let intents = rows
        .iter()
        .map(intent_from_row)
        .collect::<Result<Vec<_>>>()?;
    let Some(next) = next_to_promote(&intents) else {
        return Ok(None);
    };
    sqlx::query(
        "UPDATE intents SET status = 'confirmed', updated_at = ? WHERE user_id = ? AND event_id = ?",
    )
    .bind(ts(at))
    .bind(next.to_string())
    .bind(event.to_string())
    .execute(&mut *conn)
    .await?;
    Ok(Some(next))
}

impl Store {
    /// In one transaction: decide the intent's status with
    /// [`hangouts_core::admission`] (excluding the user's own current spot),
    /// upsert it keeping its `created_at`, and return the stored intent.
    /// If the change frees a confirmed spot (e.g. Going → Interested) the
    /// next waitlisted user is promoted.
    /// Returns [`StoreError::NotFound`] if the event does not exist.
    pub async fn set_intent(
        &self,
        user: UserId,
        event: EventId,
        level: IntentLevel,
        at: DateTime<Utc>,
    ) -> Result<Intent> {
        let mut tx = self.pool.begin().await?;
        let capacity: Option<Option<i64>> = sqlx::query("SELECT capacity FROM events WHERE id = ?")
            .bind(event.to_string())
            .fetch_optional(&mut *tx)
            .await?
            .map(|r| r.try_get("capacity"))
            .transpose()?;
        let capacity = capacity.ok_or(StoreError::NotFound)?.map(|c| c as u32);

        let previous = sqlx::query("SELECT * FROM intents WHERE user_id = ? AND event_id = ?")
            .bind(user.to_string())
            .bind(event.to_string())
            .fetch_optional(&mut *tx)
            .await?
            .as_ref()
            .map(intent_from_row)
            .transpose()?;

        let taken: i64 = sqlx::query(
            "SELECT COUNT(*) AS n FROM intents WHERE event_id = ? AND user_id <> ? \
             AND status = 'confirmed' AND level <> 'interested'",
        )
        .bind(event.to_string())
        .bind(user.to_string())
        .fetch_one(&mut *tx)
        .await?
        .try_get("n")?;
        let status = admission(capacity, taken as u32, &level);
        let created_at = previous.as_ref().map_or(at, |p| p.created_at);

        sqlx::query(
            "INSERT INTO intents (user_id, event_id, level, with_user, status, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(user_id, event_id) DO UPDATE SET level = excluded.level, \
             with_user = excluded.with_user, status = excluded.status, updated_at = excluded.updated_at",
        )
        .bind(user.to_string())
        .bind(event.to_string())
        .bind(level.kind().as_str())
        .bind(level.companion().map(|u| u.to_string()))
        .bind(status.as_str())
        .bind(ts(created_at))
        .bind(ts(at))
        .execute(&mut *tx)
        .await?;

        let held_spot = previous
            .as_ref()
            .is_some_and(|p| p.status == IntentStatus::Confirmed && p.level.uses_spot());
        if held_spot && !(status == IntentStatus::Confirmed && level.uses_spot()) {
            promote_next(&mut tx, event, at).await?;
        }

        let row = sqlx::query("SELECT * FROM intents WHERE user_id = ? AND event_id = ?")
            .bind(user.to_string())
            .bind(event.to_string())
            .fetch_one(&mut *tx)
            .await?;
        let intent = intent_from_row(&row)?;
        tx.commit().await?;
        Ok(intent)
    }

    /// One user's intent on an event, if any.
    pub async fn get_intent(&self, user: UserId, event: EventId) -> Result<Option<Intent>> {
        let row = sqlx::query("SELECT * FROM intents WHERE user_id = ? AND event_id = ?")
            .bind(user.to_string())
            .bind(event.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(intent_from_row).transpose()
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
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT * FROM intents WHERE user_id = ? AND event_id = ?")
            .bind(user.to_string())
            .bind(event.to_string())
            .fetch_optional(&mut *tx)
            .await?;
        let Some(row) = row else {
            return Ok(RemoveOutcome::default());
        };
        let intent = intent_from_row(&row)?;
        sqlx::query("DELETE FROM intents WHERE user_id = ? AND event_id = ?")
            .bind(user.to_string())
            .bind(event.to_string())
            .execute(&mut *tx)
            .await?;
        let promoted = if intent.status == IntentStatus::Confirmed && intent.level.uses_spot() {
            promote_next(&mut tx, event, at).await?
        } else {
            None
        };
        tx.commit().await?;
        Ok(RemoveOutcome {
            removed: true,
            promoted,
        })
    }

    /// All intents on an event, ordered by `created_at`.
    pub async fn list_intents_for_event(&self, event: EventId) -> Result<Vec<Intent>> {
        let rows =
            sqlx::query("SELECT * FROM intents WHERE event_id = ? ORDER BY created_at, user_id")
                .bind(event.to_string())
                .fetch_all(&self.pool)
                .await?;
        rows.iter().map(intent_from_row).collect()
    }

    /// All intents of a user, ordered by `created_at`.
    pub async fn list_intents_for_user(&self, user: UserId) -> Result<Vec<Intent>> {
        let rows =
            sqlx::query("SELECT * FROM intents WHERE user_id = ? ORDER BY created_at, event_id")
                .bind(user.to_string())
                .fetch_all(&self.pool)
                .await?;
        rows.iter().map(intent_from_row).collect()
    }

    /// The user's 1-based position among waitlisted intents on the event,
    /// or `None` if the user is not waitlisted.
    pub async fn waitlist_position(&self, user: UserId, event: EventId) -> Result<Option<u32>> {
        let rows = sqlx::query(
            "SELECT user_id FROM intents WHERE event_id = ? AND status = 'waitlisted' \
             ORDER BY created_at, user_id",
        )
        .bind(event.to_string())
        .fetch_all(&self.pool)
        .await?;
        let me = user.to_string();
        for (i, row) in rows.iter().enumerate() {
            if row.try_get::<String, _>("user_id")? == me {
                return Ok(Some(i as u32 + 1));
            }
        }
        Ok(None)
    }
}
