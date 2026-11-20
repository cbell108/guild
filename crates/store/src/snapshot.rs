//! Loading the whole database into an in-memory domain snapshot.

use hangouts_core::{BuddyRequest, CheckIn, Snapshot};
use sqlx::Row;

use crate::codec::{event_from_row, id_col, intent_from_row, ts_col, user_from_row};
use crate::crews::{crew_from_row, crew_member_from_row};
use crate::events::blob_to_embedding;
use crate::friends::friendship_from_row;
use crate::{Result, Store};

impl Store {
    /// Load every table into a [`hangouts_core::Snapshot`], including event
    /// embeddings.
    pub async fn load_snapshot(&self) -> Result<Snapshot> {
        let mut snap = Snapshot::default();
        let rows = sqlx::query("SELECT * FROM users ORDER BY display_name, id")
            .fetch_all(&self.pool)
            .await?;
        snap.users = rows.iter().map(user_from_row).collect::<Result<_>>()?;

        let rows = sqlx::query("SELECT * FROM events ORDER BY starts_at, id")
            .fetch_all(&self.pool)
            .await?;
        for row in &rows {
            let event = event_from_row(row)?;
            if let Some(blob) = row.try_get::<Option<Vec<u8>>, _>("embedding")? {
                snap.event_embeddings
                    .insert(event.id, blob_to_embedding(&blob));
            }
            snap.events.push(event);
        }

        let rows = sqlx::query("SELECT * FROM intents ORDER BY created_at, user_id, event_id")
            .fetch_all(&self.pool)
            .await?;
        snap.intents = rows.iter().map(intent_from_row).collect::<Result<_>>()?;

        let rows = sqlx::query("SELECT * FROM checkins ORDER BY at, user_id")
            .fetch_all(&self.pool)
            .await?;
        snap.checkins = rows
            .iter()
            .map(|r| {
                Ok(CheckIn {
                    user_id: id_col(r, "user_id")?,
                    event_id: id_col(r, "event_id")?,
                    at: ts_col(r, "at")?,
                })
            })
            .collect::<Result<_>>()?;

        let rows =
            sqlx::query("SELECT * FROM friendships ORDER BY created_at, requester, addressee")
                .fetch_all(&self.pool)
                .await?;
        snap.friendships = rows
            .iter()
            .map(friendship_from_row)
            .collect::<Result<_>>()?;

        let rows = sqlx::query("SELECT * FROM crews ORDER BY name, id")
            .fetch_all(&self.pool)
            .await?;
        snap.crews = rows.iter().map(crew_from_row).collect::<Result<_>>()?;

        let rows = sqlx::query("SELECT * FROM crew_members ORDER BY joined_at, crew_id, user_id")
            .fetch_all(&self.pool)
            .await?;
        snap.crew_members = rows
            .iter()
            .map(crew_member_from_row)
            .collect::<Result<_>>()?;

        let rows = sqlx::query("SELECT * FROM buddy_requests ORDER BY created_at, user_id")
            .fetch_all(&self.pool)
            .await?;
        snap.buddy_requests = rows
            .iter()
            .map(|r| {
                Ok(BuddyRequest {
                    user_id: id_col(r, "user_id")?,
                    event_id: id_col(r, "event_id")?,
                    created_at: ts_col(r, "created_at")?,
                })
            })
            .collect::<Result<_>>()?;
        Ok(snap)
    }
}
