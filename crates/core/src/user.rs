//! Students and their privacy settings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::UserId;

/// A student account (demo or real).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// Stable id.
    pub id: UserId,
    /// Name shown to friends and on cards.
    pub display_name: String,
    /// Short self-description (seed personas use this).
    pub bio: Option<String>,
    /// Normalized interest tags (see [`crate::normalize_tag`]); drives cold-start recommendations.
    pub interests: Vec<String>,
    /// Discord snowflake, when the account is linked to the bot.
    pub discord_id: Option<String>,
    /// Seeded demo account (selectable on the demo login screen).
    pub is_demo: bool,
    /// What this user lets others see and what the recommender may use.
    pub privacy: PrivacySettings,
    /// When the account was created.
    pub created_at: DateTime<Utc>,
}

/// Who can see a user's attendance (intents and check-ins).
///
/// There is deliberately no `Public` option: nobody ever sees full attendee lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Only the user themself.
    Private,
    /// Accepted friends (the default, so friend presence works).
    #[default]
    Friends,
}

/// Privacy controls, all owned by the student.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// Who sees this user's attendance.
    pub attendance: Visibility,
    /// Whether this user can be suggested as a buddy to non-friends.
    pub buddy_matching: bool,
    /// Whether the recommender may use this user's history (off = interests only).
    pub personalize: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            attendance: Visibility::Friends,
            buddy_matching: true,
            personalize: true,
        }
    }
}
