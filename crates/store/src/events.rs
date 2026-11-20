//! Events: official KSU calendar events and user-created hangouts.

use chrono::{DateTime, Utc};
use hangouts_core::{Event, EventId, EventKind};
use sqlx::Row;

use crate::codec::{event_from_row, to_json, ts};
use crate::users::like_pattern;
use crate::{Result, Store, StoreError};

/// Filter for [`Store::list_events`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventFilter {
    /// Only events of this kind; `None` matches any kind.
    pub kind: Option<EventKind>,
    /// The event must carry at least one of these tags (any-of match).
    pub tags: Vec<String>,
    /// Start of the window: events whose time range overlaps `[from, to)`.
    pub from: Option<DateTime<Utc>>,
    /// End of the window (exclusive).
    pub to: Option<DateTime<Utc>>,
    /// Case-insensitive substring match on the title or description.
    pub query: Option<String>,
    /// Maximum number of events to return (default 100).
    pub limit: Option<u32>,
}

pub(crate) const INSERT_COLUMNS: &str = "id, kind, title, description, tags, location_name, location_room, \
    location_lat, location_lon, online_url, starts_at, ends_at, capacity, host_id, crew_id, \
    external_id, url, image_url, organizer, created_at";

/// Bind every event column in [`INSERT_COLUMNS`] order.
fn bind_event<'q>(
    q: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments>,
    e: &'q Event,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments> {
    q.bind(e.id.to_string())
        .bind(e.kind.as_str())
        .bind(&e.title)
        .bind(&e.description)
        .bind(to_json(&e.tags))
        .bind(&e.location.name)
        .bind(&e.location.room)
        .bind(e.location.lat)
        .bind(e.location.lon)
        .bind(&e.location.online_url)
        .bind(ts(e.starts_at))
        .bind(ts(e.ends_at))
        .bind(e.capacity.map(i64::from))
        .bind(e.host_id.map(|h| h.to_string()))
        .bind(e.crew_id.map(|c| c.to_string()))
        .bind(&e.external_id)
        .bind(&e.url)
        .bind(&e.image_url)
        .bind(&e.organizer)
        .bind(ts(e.created_at))
}

impl Store {
    /// Insert an event. Returns [`StoreError::Conflict`] if the id or the
    /// `external_id` already exists.
    pub async fn insert_event(&self, event: &Event) -> Result<()> {
        let sql = format!(
            "INSERT INTO events ({INSERT_COLUMNS}) VALUES ({})",
            vec!["?"; 20].join(", ")
        );
        match bind_event(sqlx::query(sqlx::AssertSqlSafe(sql)), event)
            .execute(&self.pool)
            .await
        {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                Err(StoreError::Conflict("event already exists".into()))
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Insert or update an official event keyed by `external_id`, keeping the
    /// id of the existing row (which is returned).
    /// Returns [`StoreError::Invalid`] if `external_id` is `None`.
    pub async fn upsert_official_event(&self, event: &Event) -> Result<EventId> {
        let Some(external) = event.external_id.as_deref() else {
            return Err(StoreError::Invalid("external_id is required".into()));
        };
        let mut tx = self.pool.begin().await?;
        let existing = sqlx::query("SELECT id FROM events WHERE external_id = ?")
            .bind(external)
            .fetch_optional(&mut *tx)
            .await?;
        let id = if let Some(row) = existing {
            let id: EventId = crate::codec::id_col(&row, "id")?;
            sqlx::query(
                "UPDATE events SET kind = ?, title = ?, description = ?, tags = ?, \
                 location_name = ?, location_room = ?, location_lat = ?, location_lon = ?, \
                 online_url = ?, starts_at = ?, ends_at = ?, capacity = ?, url = ?, \
                 image_url = ?, organizer = ? WHERE id = ?",
            )
            .bind(event.kind.as_str())
            .bind(&event.title)
            .bind(&event.description)
            .bind(to_json(&event.tags))
            .bind(&event.location.name)
            .bind(&event.location.room)
            .bind(event.location.lat)
            .bind(event.location.lon)
            .bind(&event.location.online_url)
            .bind(ts(event.starts_at))
            .bind(ts(event.ends_at))
            .bind(event.capacity.map(i64::from))
            .bind(&event.url)
            .bind(&event.image_url)
            .bind(&event.organizer)
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?;
            // Content may have changed, so the stored embedding is stale.
            sqlx::query("UPDATE events SET embedding = NULL WHERE id = ?")
                .bind(id.to_string())
                .execute(&mut *tx)
                .await?;
            id
        } else {
            let sql = format!(
                "INSERT INTO events ({INSERT_COLUMNS}) VALUES ({})",
                vec!["?"; 20].join(", ")
            );
            bind_event(sqlx::query(sqlx::AssertSqlSafe(sql)), event)
                .execute(&mut *tx)
                .await?;
            event.id
        };
        tx.commit().await?;
        Ok(id)
    }

    /// One event by id.
    pub async fn get_event(&self, id: EventId) -> Result<Option<Event>> {
        let sql = format!("SELECT {INSERT_COLUMNS} FROM events WHERE id = ?");
        let row = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(event_from_row).transpose()
    }

    /// Events matching `filter`, ordered by `starts_at`.
    pub async fn list_events(&self, filter: &EventFilter) -> Result<Vec<Event>> {
        // Explicit columns: the embedding BLOB is never needed for display.
        let mut sql = format!("SELECT {INSERT_COLUMNS} FROM events WHERE 1 = 1");
        if filter.kind.is_some() {
            sql.push_str(" AND kind = ?");
        }
        if !filter.tags.is_empty() {
            let marks = vec!["?"; filter.tags.len()].join(", ");
            sql.push_str(" AND EXISTS (SELECT 1 FROM json_each(events.tags) WHERE value IN (");
            sql.push_str(&marks);
            sql.push_str("))");
        }
        if filter.from.is_some() {
            sql.push_str(" AND ends_at > ?");
        }
        if filter.to.is_some() {
            sql.push_str(" AND starts_at < ?");
        }
        if filter.query.is_some() {
            sql.push_str(" AND (title LIKE ? ESCAPE '\\' OR description LIKE ? ESCAPE '\\')");
        }
        sql.push_str(" ORDER BY starts_at, id LIMIT ?");

        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        if let Some(kind) = filter.kind {
            q = q.bind(kind.as_str());
        }
        for tag in &filter.tags {
            q = q.bind(tag);
        }
        if let Some(from) = filter.from {
            q = q.bind(ts(from));
        }
        if let Some(to) = filter.to {
            q = q.bind(ts(to));
        }
        if let Some(text) = &filter.query {
            let pattern = like_pattern(text);
            q = q.bind(pattern.clone()).bind(pattern);
        }
        q = q.bind(i64::from(filter.limit.unwrap_or(100)));
        let rows = q.fetch_all(&self.pool).await?;
        rows.iter().map(event_from_row).collect()
    }

    /// Tags used by upcoming events with usage counts, most common first.
    pub async fn list_tags(&self) -> Result<Vec<(String, u32)>> {
        let rows = sqlx::query(
            "SELECT j.value AS tag, COUNT(*) AS n FROM events e, json_each(e.tags) j \
             WHERE e.ends_at > ? GROUP BY j.value ORDER BY n DESC, tag",
        )
        .bind(ts(Utc::now()))
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok((
                    r.try_get::<String, _>("tag")?,
                    r.try_get::<i64, _>("n")? as u32,
                ))
            })
            .collect()
    }

    /// Store an event's embedding as a little-endian `f32` BLOB.
    pub async fn set_event_embedding(&self, event: EventId, embedding: &[f32]) -> Result<()> {
        let blob = embedding_to_blob(embedding);
        let res = sqlx::query("UPDATE events SET embedding = ? WHERE id = ?")
            .bind(blob)
            .bind(event.to_string())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    /// Events that have no stored embedding yet.
    pub async fn events_missing_embedding(&self) -> Result<Vec<Event>> {
        let rows =
            sqlx::query("SELECT * FROM events WHERE embedding IS NULL ORDER BY starts_at, id")
                .fetch_all(&self.pool)
                .await?;
        rows.iter().map(event_from_row).collect()
    }
}

/// Encode an embedding as little-endian `f32` bytes.
pub(crate) fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Decode little-endian `f32` bytes.
pub(crate) fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect()
}
