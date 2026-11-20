//! Friendships and pending friend requests between users.

use chrono::{DateTime, Utc};
use hangouts_core::{Friendship, UserId};

use crate::{Result, Store, StoreError};

impl Store {
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
        let _ = (self, from, to, at);
        Err(StoreError::NotImplemented("request_friend"))
    }

    /// Accept the pending request that `requester` sent to `user`.
    /// Returns [`StoreError::NotFound`] if there is none.
    pub async fn accept_friend(
        &self,
        user: UserId,
        requester: UserId,
        at: DateTime<Utc>,
    ) -> Result<Friendship> {
        let _ = (self, user, requester, at);
        Err(StoreError::NotImplemented("accept_friend"))
    }

    /// Delete the friendship or pending request between `a` and `b` in either
    /// direction. Returns `true` if one existed.
    pub async fn remove_friend(&self, a: UserId, b: UserId) -> Result<bool> {
        let _ = (self, a, b);
        Err(StoreError::NotImplemented("remove_friend"))
    }

    /// All friendship rows involving the user (pending and accepted).
    pub async fn list_friendships(&self, user: UserId) -> Result<Vec<Friendship>> {
        let _ = (self, user);
        Err(StoreError::NotImplemented("list_friendships"))
    }

    /// The user's accepted friends.
    pub async fn friend_ids(&self, user: UserId) -> Result<Vec<UserId>> {
        let _ = (self, user);
        Err(StoreError::NotImplemented("friend_ids"))
    }
}
