//! Loading the whole database into an in-memory domain snapshot.

use std::collections::BTreeSet;

use hangouts_core::{BuddyRequest, CheckIn, Event, EventId, Snapshot, UserId};
use sqlx::Row;

use crate::codec::{event_from_row, id_col, intent_from_row, ts_col, user_from_row};
use crate::crews::{crew_from_row, crew_member_from_row};
use crate::events::{INSERT_COLUMNS, blob_to_embedding};
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

        let sql = format!("SELECT {INSERT_COLUMNS}, embedding FROM events ORDER BY starts_at, id");
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
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

    /// A small snapshot with just what rendering `events` as cards or details for `viewer`
    /// needs, via targeted queries instead of reading every table:
    ///
    /// - `events`: exactly the ones passed in;
    /// - `intents`: every intent on those events (spots and waitlist positions);
    /// - `friendships`: all of the viewer's;
    /// - `users`: the viewer, hosts, "going with" companions, and the viewer's friends who
    ///   have an intent on these events;
    /// - `crews` / `crew_members`: the events' crews, and only the viewer's memberships;
    /// - `buddy_requests`: only the viewer's, on these events.
    ///
    /// No check-ins, embeddings or unrelated users. Views built from it equal views built from
    /// [`Store::load_snapshot`] (the `web` crate tests this).
    pub async fn load_event_view(
        &self,
        events: &[Event],
        viewer: Option<UserId>,
    ) -> Result<Snapshot> {
        let mut snap = Snapshot {
            events: events.to_vec(),
            ..Snapshot::default()
        };
        if events.is_empty() {
            return Ok(snap);
        }
        let event_ids: Vec<String> = events.iter().map(|e| e.id.to_string()).collect();
        snap.intents = self
            .fetch_in(
                "SELECT * FROM intents WHERE event_id IN ({}) ORDER BY created_at, user_id",
                &event_ids,
                None,
            )
            .await?
            .iter()
            .map(intent_from_row)
            .collect::<Result<_>>()?;

        let mut user_ids: BTreeSet<UserId> = events.iter().filter_map(|e| e.host_id).collect();
        user_ids.extend(snap.intents.iter().filter_map(|i| i.level.companion()));
        if let Some(viewer) = viewer {
            user_ids.insert(viewer);
            snap.friendships = self.list_friendships(viewer).await?;
            let friends = snap.friends_of(viewer);
            user_ids.extend(
                snap.intents
                    .iter()
                    .map(|i| i.user_id)
                    .filter(|u| friends.contains(u)),
            );
            let rows = sqlx::query(
                "SELECT * FROM buddy_requests WHERE user_id = ? ORDER BY created_at, user_id",
            )
            .bind(viewer.to_string())
            .fetch_all(&self.pool)
            .await?;
            for r in &rows {
                let event_id: EventId = id_col(r, "event_id")?;
                if events.iter().any(|e| e.id == event_id) {
                    snap.buddy_requests.push(BuddyRequest {
                        user_id: id_col(r, "user_id")?,
                        event_id,
                        created_at: ts_col(r, "created_at")?,
                    });
                }
            }
        }

        let user_ids: Vec<String> = user_ids.iter().map(ToString::to_string).collect();
        snap.users = self
            .fetch_in("SELECT * FROM users WHERE id IN ({})", &user_ids, None)
            .await?
            .iter()
            .map(user_from_row)
            .collect::<Result<_>>()?;

        let crew_ids: Vec<String> = events
            .iter()
            .filter_map(|e| e.crew_id)
            .collect::<BTreeSet<_>>()
            .iter()
            .map(ToString::to_string)
            .collect();
        if !crew_ids.is_empty() {
            snap.crews = self
                .fetch_in("SELECT * FROM crews WHERE id IN ({})", &crew_ids, None)
                .await?
                .iter()
                .map(crew_from_row)
                .collect::<Result<_>>()?;
            if let Some(viewer) = viewer {
                snap.crew_members = self
                    .fetch_in(
                        "SELECT * FROM crew_members WHERE crew_id IN ({}) AND user_id = ?",
                        &crew_ids,
                        Some(viewer.to_string()),
                    )
                    .await?
                    .iter()
                    .map(crew_member_from_row)
                    .collect::<Result<_>>()?;
            }
        }
        Ok(snap)
    }

    /// Runs `template` with its `{}` replaced by one `?` per value, then binds the values
    /// (and `extra`, bound last, for templates with a trailing `?`).
    async fn fetch_in(
        &self,
        template: &str,
        values: &[String],
        extra: Option<String>,
    ) -> Result<Vec<sqlx::sqlite::SqliteRow>> {
        if values.is_empty() {
            return Ok(Vec::new());
        }
        let marks = vec!["?"; values.len()].join(", ");
        let sql = template.replacen("{}", &marks, 1);
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        for v in values {
            q = q.bind(v);
        }
        if let Some(extra) = extra {
            q = q.bind(extra);
        }
        Ok(q.fetch_all(&self.pool).await?)
    }

    /// A small snapshot for buddy matching of `viewer` on `event`: the event, all buddy
    /// requests on it, the viewer's friendships, the users involved (viewer, seekers, the
    /// viewer's friends), and the intents and check-ins of the viewer and the seekers, which
    /// is all `recommend::similar_users` reads. The event is absent from `events` if it does
    /// not exist.
    pub async fn load_buddy_view(&self, event: EventId, viewer: UserId) -> Result<Snapshot> {
        let mut snap = Snapshot::default();
        if let Some(e) = self.get_event(event).await? {
            snap.events.push(e);
        }
        let rows = sqlx::query(
            "SELECT * FROM buddy_requests WHERE event_id = ? ORDER BY created_at, user_id",
        )
        .bind(event.to_string())
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
        snap.friendships = self.list_friendships(viewer).await?;

        let mut people: BTreeSet<UserId> = snap.buddy_requests.iter().map(|b| b.user_id).collect();
        people.insert(viewer);
        let people_ids: Vec<String> = people.iter().map(ToString::to_string).collect();
        let mut user_ids = people.clone();
        user_ids.extend(snap.friends_of(viewer));
        let user_ids: Vec<String> = user_ids.iter().map(ToString::to_string).collect();

        snap.users = self
            .fetch_in("SELECT * FROM users WHERE id IN ({})", &user_ids, None)
            .await?
            .iter()
            .map(user_from_row)
            .collect::<Result<_>>()?;
        snap.intents = self
            .fetch_in(
                "SELECT * FROM intents WHERE user_id IN ({}) ORDER BY created_at, user_id, event_id",
                &people_ids,
                None,
            )
            .await?
            .iter()
            .map(intent_from_row)
            .collect::<Result<_>>()?;
        snap.checkins = self
            .fetch_in(
                "SELECT * FROM checkins WHERE user_id IN ({}) ORDER BY at, user_id",
                &people_ids,
                None,
            )
            .await?
            .iter()
            .map(|r| {
                Ok(CheckIn {
                    user_id: id_col(r, "user_id")?,
                    event_id: id_col(r, "event_id")?,
                    at: ts_col(r, "at")?,
                })
            })
            .collect::<Result<_>>()?;
        Ok(snap)
    }
}
