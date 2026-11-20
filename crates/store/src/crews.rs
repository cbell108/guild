//! Crews: named groups of users with per-member roles.

use chrono::{DateTime, Utc};
use hangouts_core::{Crew, CrewId, CrewMember, CrewRole, UserId};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::codec::{id_col, ts, ts_col};
use crate::{Result, Store, StoreError};

pub(crate) fn crew_from_row(row: &SqliteRow) -> Result<Crew> {
    Ok(Crew {
        id: id_col(row, "id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        created_by: id_col(row, "created_by")?,
        created_at: ts_col(row, "created_at")?,
    })
}

const fn role_str(role: CrewRole) -> &'static str {
    match role {
        CrewRole::Owner => "owner",
        CrewRole::Member => "member",
    }
}

impl Store {
    /// Insert a crew and add `crew.created_by` as its Owner, with
    /// `joined_at = crew.created_at`.
    pub async fn insert_crew(&self, crew: &Crew) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let res = sqlx::query(
            "INSERT INTO crews (id, name, description, created_by, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(crew.id.to_string())
        .bind(&crew.name)
        .bind(&crew.description)
        .bind(crew.created_by.to_string())
        .bind(ts(crew.created_at))
        .execute(&mut *tx)
        .await;
        match res {
            Ok(_) => {}
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                return Err(StoreError::Conflict("crew already exists".into()));
            }
            Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
                return Err(StoreError::NotFound);
            }
            Err(e) => return Err(e.into()),
        }
        sqlx::query(
            "INSERT INTO crew_members (crew_id, user_id, role, joined_at) VALUES (?, ?, 'owner', ?)",
        )
        .bind(crew.id.to_string())
        .bind(crew.created_by.to_string())
        .bind(ts(crew.created_at))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// One crew by id.
    pub async fn get_crew(&self, id: CrewId) -> Result<Option<Crew>> {
        let row = sqlx::query("SELECT * FROM crews WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(crew_from_row).transpose()
    }

    /// Crews the user belongs to, ordered by name.
    pub async fn list_crews_for_user(&self, user: UserId) -> Result<Vec<Crew>> {
        let rows = sqlx::query(
            "SELECT c.* FROM crews c JOIN crew_members m ON m.crew_id = c.id \
             WHERE m.user_id = ? ORDER BY c.name, c.id",
        )
        .bind(user.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(crew_from_row).collect()
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
        let res = sqlx::query(
            "INSERT INTO crew_members (crew_id, user_id, role, joined_at) VALUES (?, ?, ?, ?) \
             ON CONFLICT(crew_id, user_id) DO UPDATE SET role = excluded.role",
        )
        .bind(crew.to_string())
        .bind(user.to_string())
        .bind(role_str(role))
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

    /// Remove a member from the crew. Returns `true` if one existed.
    pub async fn remove_crew_member(&self, crew: CrewId, user: UserId) -> Result<bool> {
        let res = sqlx::query("DELETE FROM crew_members WHERE crew_id = ? AND user_id = ?")
            .bind(crew.to_string())
            .bind(user.to_string())
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    /// All members of the crew, ordered by `joined_at`.
    pub async fn list_crew_members(&self, crew: CrewId) -> Result<Vec<CrewMember>> {
        let rows =
            sqlx::query("SELECT * FROM crew_members WHERE crew_id = ? ORDER BY joined_at, user_id")
                .bind(crew.to_string())
                .fetch_all(&self.pool)
                .await?;
        rows.iter().map(crew_member_from_row).collect()
    }
}

pub(crate) fn crew_member_from_row(row: &SqliteRow) -> Result<CrewMember> {
    let role = match row.try_get::<String, _>("role")?.as_str() {
        "owner" => CrewRole::Owner,
        _ => CrewRole::Member,
    };
    Ok(CrewMember {
        crew_id: id_col(row, "crew_id")?,
        user_id: id_col(row, "user_id")?,
        role,
        joined_at: ts_col(row, "joined_at")?,
    })
}
