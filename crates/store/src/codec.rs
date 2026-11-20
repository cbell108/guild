//! Conversions between domain types and SQLite column values.

use chrono::{DateTime, Utc};
use hangouts_core::{
    Event, EventId, EventKind, Intent, IntentKind, IntentLevel, IntentStatus, Location,
    PrivacySettings, User, UserId, Visibility,
};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::{Result, StoreError};

/// Timestamp → microseconds since the epoch.
pub(crate) fn ts(t: DateTime<Utc>) -> i64 {
    t.timestamp_micros()
}

/// Microseconds since the epoch → timestamp.
pub(crate) fn from_ts(micros: i64) -> Result<DateTime<Utc>> {
    DateTime::from_timestamp_micros(micros)
        .ok_or_else(|| StoreError::Invalid(format!("timestamp out of range: {micros}")))
}

/// Parse a stored UUID string.
pub(crate) fn parse_id<T: std::str::FromStr>(text: &str) -> Result<T> {
    text.parse()
        .map_err(|_| StoreError::Invalid(format!("bad id in database: {text:?}")))
}

/// Read an id column.
pub(crate) fn id_col<T: std::str::FromStr>(row: &SqliteRow, col: &str) -> Result<T> {
    parse_id(&row.try_get::<String, _>(col)?)
}

/// Read a timestamp column.
pub(crate) fn ts_col(row: &SqliteRow, col: &str) -> Result<DateTime<Utc>> {
    from_ts(row.try_get::<i64, _>(col)?)
}

/// Read a JSON string-array column.
pub(crate) fn json_list(row: &SqliteRow, col: &str) -> Result<Vec<String>> {
    serde_json::from_str(&row.try_get::<String, _>(col)?)
        .map_err(|e| StoreError::Invalid(format!("bad JSON in {col}: {e}")))
}

/// Encode a string list as JSON.
pub(crate) fn to_json(list: &[String]) -> String {
    serde_json::to_string(list).unwrap_or_else(|_| "[]".to_owned())
}

/// Build a [`User`] from a `users` row.
pub(crate) fn user_from_row(row: &SqliteRow) -> Result<User> {
    let attendance = match row.try_get::<String, _>("privacy_attendance")?.as_str() {
        "private" => Visibility::Private,
        _ => Visibility::Friends,
    };
    Ok(User {
        id: id_col(row, "id")?,
        display_name: row.try_get("display_name")?,
        bio: row.try_get("bio")?,
        interests: json_list(row, "interests")?,
        discord_id: row.try_get("discord_id")?,
        is_demo: row.try_get("is_demo")?,
        privacy: PrivacySettings {
            attendance,
            buddy_matching: row.try_get("privacy_buddy_matching")?,
            personalize: row.try_get("privacy_personalize")?,
        },
        created_at: ts_col(row, "created_at")?,
    })
}

/// Column value for a [`Visibility`].
pub(crate) const fn visibility_str(v: Visibility) -> &'static str {
    match v {
        Visibility::Private => "private",
        Visibility::Friends => "friends",
    }
}

/// Build an [`Event`] from an `events` row.
pub(crate) fn event_from_row(row: &SqliteRow) -> Result<Event> {
    let kind = EventKind::parse(&row.try_get::<String, _>("kind")?)
        .ok_or_else(|| StoreError::Invalid("bad event kind".into()))?;
    let opt_id = |col: &str| -> Result<Option<String>> { Ok(row.try_get(col)?) };
    Ok(Event {
        id: id_col(row, "id")?,
        kind,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        tags: json_list(row, "tags")?,
        location: Location {
            name: row.try_get("location_name")?,
            room: row.try_get("location_room")?,
            lat: row.try_get("location_lat")?,
            lon: row.try_get("location_lon")?,
            online_url: row.try_get("online_url")?,
        },
        starts_at: ts_col(row, "starts_at")?,
        ends_at: ts_col(row, "ends_at")?,
        capacity: row.try_get::<Option<i64>, _>("capacity")?.map(|c| c as u32),
        host_id: opt_id("host_id")?.map(|s| parse_id(&s)).transpose()?,
        crew_id: opt_id("crew_id")?.map(|s| parse_id(&s)).transpose()?,
        external_id: row.try_get("external_id")?,
        url: row.try_get("url")?,
        image_url: row.try_get("image_url")?,
        organizer: row.try_get("organizer")?,
        created_at: ts_col(row, "created_at")?,
    })
}

/// Build an [`Intent`] from an `intents` row.
pub(crate) fn intent_from_row(row: &SqliteRow) -> Result<Intent> {
    let kind = IntentKind::parse(&row.try_get::<String, _>("level")?)
        .ok_or_else(|| StoreError::Invalid("bad intent level".into()))?;
    let with: Option<UserId> = row
        .try_get::<Option<String>, _>("with_user")?
        .map(|s| parse_id(&s))
        .transpose()?;
    let level =
        IntentLevel::from_parts(kind, with).map_err(|e| StoreError::Invalid(e.to_string()))?;
    let status = IntentStatus::parse(&row.try_get::<String, _>("status")?)
        .ok_or_else(|| StoreError::Invalid("bad intent status".into()))?;
    Ok(Intent {
        user_id: id_col(row, "user_id")?,
        event_id: id_col::<EventId>(row, "event_id")?,
        level,
        status,
        created_at: ts_col(row, "created_at")?,
        updated_at: ts_col(row, "updated_at")?,
    })
}
