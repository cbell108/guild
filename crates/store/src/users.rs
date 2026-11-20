//! User accounts, interest tags and privacy settings.

use chrono::{DateTime, Utc};
use hangouts_core::{PrivacySettings, User, UserId};

use crate::codec::{to_json, ts, user_from_row, visibility_str};
use crate::{Result, Store, StoreError};

/// Escape `%`, `_` and `\` for a LIKE pattern using `ESCAPE '\'`.
pub(crate) fn like_pattern(text: &str) -> String {
    let mut out = String::from("%");
    for c in text.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

impl Store {
    /// Insert a new user. Returns [`StoreError::Conflict`] if the id exists.
    pub async fn insert_user(&self, user: &User) -> Result<()> {
        let res = sqlx::query(
            "INSERT INTO users (id, display_name, bio, interests, discord_id, is_demo, \
             privacy_attendance, privacy_buddy_matching, privacy_personalize, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user.id.to_string())
        .bind(&user.display_name)
        .bind(&user.bio)
        .bind(to_json(&user.interests))
        .bind(&user.discord_id)
        .bind(user.is_demo)
        .bind(visibility_str(user.privacy.attendance))
        .bind(user.privacy.buddy_matching)
        .bind(user.privacy.personalize)
        .bind(ts(user.created_at))
        .execute(&self.pool)
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                Err(StoreError::Conflict("user already exists".into()))
            }
            Err(e) => Err(e.into()),
        }
    }

    /// One user by id.
    pub async fn get_user(&self, id: UserId) -> Result<Option<User>> {
        let row = sqlx::query("SELECT * FROM users WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(user_from_row).transpose()
    }

    /// All users, ordered by `display_name`.
    pub async fn list_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query("SELECT * FROM users ORDER BY display_name, id")
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(user_from_row).collect()
    }

    /// Users with `is_demo`, ordered by `display_name`.
    pub async fn list_demo_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query("SELECT * FROM users WHERE is_demo = 1 ORDER BY display_name, id")
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(user_from_row).collect()
    }

    /// Case-insensitive substring match on `display_name`, at most `limit` hits.
    pub async fn search_users(&self, query: &str, limit: u32) -> Result<Vec<User>> {
        let rows = sqlx::query(
            "SELECT * FROM users WHERE display_name LIKE ? ESCAPE '\\' \
             ORDER BY display_name, id LIMIT ?",
        )
        .bind(like_pattern(query))
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(user_from_row).collect()
    }

    /// Find a user by `discord_id`, or create a non-demo user with default
    /// privacy at `at` if none exists; updates `display_name`.
    pub async fn upsert_discord_user(
        &self,
        discord_id: &str,
        display_name: &str,
        at: DateTime<Utc>,
    ) -> Result<User> {
        let defaults = PrivacySettings::default();
        sqlx::query(
            "INSERT INTO users (id, display_name, interests, discord_id, is_demo, \
             privacy_attendance, privacy_buddy_matching, privacy_personalize, created_at) \
             VALUES (?, ?, '[]', ?, 0, ?, ?, ?, ?) \
             ON CONFLICT(discord_id) DO UPDATE SET display_name = excluded.display_name",
        )
        .bind(UserId::new().to_string())
        .bind(display_name)
        .bind(discord_id)
        .bind(visibility_str(defaults.attendance))
        .bind(defaults.buddy_matching)
        .bind(defaults.personalize)
        .bind(ts(at))
        .execute(&self.pool)
        .await?;
        let row = sqlx::query("SELECT * FROM users WHERE discord_id = ?")
            .bind(discord_id)
            .fetch_one(&self.pool)
            .await?;
        user_from_row(&row)
    }

    /// Replace the user's interest tags (the caller normalizes them).
    /// Returns [`StoreError::NotFound`] if there is no such user.
    pub async fn set_interests(&self, user: UserId, interests: &[String]) -> Result<()> {
        let res = sqlx::query("UPDATE users SET interests = ? WHERE id = ?")
            .bind(to_json(interests))
            .bind(user.to_string())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    /// Replace the user's privacy settings.
    /// Returns [`StoreError::NotFound`] if there is no such user.
    pub async fn set_privacy(&self, user: UserId, privacy: &PrivacySettings) -> Result<()> {
        let res = sqlx::query(
            "UPDATE users SET privacy_attendance = ?, privacy_buddy_matching = ?, \
             privacy_personalize = ? WHERE id = ?",
        )
        .bind(visibility_str(privacy.attendance))
        .bind(privacy.buddy_matching)
        .bind(privacy.personalize)
        .bind(user.to_string())
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    /// Delete the user's intents, check-ins, buddy requests and availability
    /// windows. Does not delete the account, friendships or crew membership.
    pub async fn delete_history(&self, user: UserId) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for table in ["intents", "checkins", "buddy_requests", "availability"] {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "DELETE FROM {table} WHERE user_id = ?"
            )))
            .bind(user.to_string())
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
