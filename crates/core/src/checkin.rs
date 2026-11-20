//! Check-ins: proof that someone actually showed up (the signal no-show learning uses).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{EventId, UserId};

/// A user checked in at an event (at most one per pair).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckIn {
    /// Who.
    pub user_id: UserId,
    /// Where.
    pub event_id: EventId,
    /// When.
    pub at: DateTime<Utc>,
}
