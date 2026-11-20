//! Friendships and pending friend requests between users.

use chrono::{DateTime, Utc};
use hangouts_core::{Friendship, FriendshipStatus, UserId};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::codec::{id_col, ts, ts_col};
use crate::{Result, Store, StoreError};

pub(crate) fn friendship_from_row(row: &SqliteRow) -> Result<Friendship> {
    let status = match row.try_get::<String, _>("status")?.as_str() {
        "accepted" => FriendshipStatus::Accepted,
        _ => FriendshipStatus::Pending,
    };
    Ok(Friendship {
        requester: id_col(row, "requester")?,
        addressee: id_col(row, "addressee")?,
        status,
        created_at: ts_col(row, "created_at")?,
    })
}

impl Store {
    /// Fetch the row between two users in either direction.
    async fn friendship_between(&self, a: UserId, b: UserId) -> Result<Option<Friendship>> {
        let row = sqlx::query(
            "SELECT * FROM friendships WHERE (requester = ?1 AND addressee = ?2) \
             OR (requester = ?2 AND addressee = ?1)",
        )
        .bind(a.to_string())
        .bind(b.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(friendship_from_row).transpose()
    }

    /// Create a pending request from `from` to `to`. If `to` has already
    /// requested `from`, accept it instead.
    /// Returns [`StoreError::Invalid`] if `from == to`. Returns the current
    /// row.
    pub async fn request_friend(
        &self,
        from: UserId,
        to: UserId,
        at: DateTime<Utc>,
    ) -> Result<Friendship> {
        if from == to {
            return Err(StoreError::Invalid("cannot befriend yourself".into()));
        }
        if let Some(existing) = self.friendship_between(from, to).await? {
            if existing.status == FriendshipStatus::Pending && existing.requester == to {
                return self.accept_friend(from, to, at).await;
            }
            return Ok(existing);
        }
        let res = sqlx::query(
            "INSERT INTO friendships (requester, addressee, status, created_at) \
             VALUES (?, ?, 'pending', ?)",
        )
        .bind(from.to_string())
        .bind(to.to_string())
        .bind(ts(at))
        .execute(&self.pool)
        .await;
        match res {
            Ok(_) => {}
            Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
                return Err(StoreError::NotFound);
            }
            Err(e) => return Err(e.into()),
        }
        Ok(Friendship {
            requester: from,
            addressee: to,
            status: FriendshipStatus::Pending,
            created_at: at,
        })
    }

    /// Accept the pending request that `requester` sent to `user`.
    /// Returns [`StoreError::NotFound`] if there is none.
    pub async fn accept_friend(
        &self,
        user: UserId,
        requester: UserId,
        _at: DateTime<Utc>,
    ) -> Result<Friendship> {
        let res = sqlx::query(
            "UPDATE friendships SET status = 'accepted' \
             WHERE requester = ? AND addressee = ? AND status = 'pending'",
        )
        .bind(requester.to_string())
        .bind(user.to_string())
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        self.friendship_between(user, requester)
            .await?
            .ok_or(StoreError::NotFound)
    }

    /// Delete the friendship or pending request between `a` and `b` in either
    /// direction. Returns `true` if one existed.
    pub async fn remove_friend(&self, a: UserId, b: UserId) -> Result<bool> {
        let res = sqlx::query(
            "DELETE FROM friendships WHERE (requester = ?1 AND addressee = ?2) \
             OR (requester = ?2 AND addressee = ?1)",
        )
        .bind(a.to_string())
        .bind(b.to_string())
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    /// All friendship rows involving the user (pending and accepted).
    pub async fn list_friendships(&self, user: UserId) -> Result<Vec<Friendship>> {
        let rows = sqlx::query(
            "SELECT * FROM friendships WHERE requester = ?1 OR addressee = ?1 \
             ORDER BY created_at, requester, addressee",
        )
        .bind(user.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(friendship_from_row).collect()
    }

    /// The user's accepted friends.
    pub async fn friend_ids(&self, user: UserId) -> Result<Vec<UserId>> {
        Ok(self
            .list_friendships(user)
            .await?
            .into_iter()
            .filter(|f| f.status == FriendshipStatus::Accepted)
            .filter_map(|f| f.other(user))
            .collect())
    }
}
