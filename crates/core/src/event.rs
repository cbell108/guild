//! Events: official KSU calendar events and student-hosted hangouts.

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::{CrewId, EventId, UserId};
use crate::time;

/// Maximum title length in characters.
pub const MAX_TITLE_CHARS: usize = 100;
/// Maximum description length in characters.
pub const MAX_DESCRIPTION_CHARS: usize = 2000;
/// Maximum number of tags per event.
pub const MAX_TAGS: usize = 10;
/// Maximum length of one normalized tag.
pub const MAX_TAG_CHARS: usize = 32;
/// Hangout length when the host gives no end time.
pub const DEFAULT_HANGOUT_HOURS: i64 = 2;
/// How far in the past a new hangout may start (grace for "starting now").
pub const PAST_START_GRACE_MINUTES: i64 = 60;

/// Where an event comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Imported from the KSU public calendar.
    Official,
    /// Hosted by a student (web or Discord `/hangout`).
    Hangout,
}

impl EventKind {
    /// Lowercase name used in storage and query strings.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Hangout => "hangout",
        }
    }

    /// Inverse of [`EventKind::as_str`].
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "official" => Some(Self::Official),
            "hangout" => Some(Self::Hangout),
            _ => None,
        }
    }
}

/// Where an event happens.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Location {
    /// Building / venue name, e.g. "Student Center" or "Online".
    pub name: String,
    /// Room, e.g. "Game Room 274".
    pub room: Option<String>,
    /// Latitude, when known.
    pub lat: Option<f64>,
    /// Longitude, when known.
    pub lon: Option<f64>,
    /// Stream / meeting link for virtual events.
    pub online_url: Option<String>,
}

/// An official event or a hangout. One row per occurrence (KSU event instance).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Stable id.
    pub id: EventId,
    /// Official or hangout.
    pub kind: EventKind,
    /// Short title.
    pub title: String,
    /// Plain-text description (no HTML).
    pub description: String,
    /// Normalized tags (see [`normalize_tag`]).
    pub tags: Vec<String>,
    /// Venue.
    pub location: Location,
    /// Start instant.
    pub starts_at: DateTime<Utc>,
    /// End instant (after `starts_at`).
    pub ends_at: DateTime<Utc>,
    /// Max confirmed Going attendees; `None` = unlimited (no waitlist).
    pub capacity: Option<u32>,
    /// Hosting student (hangouts only).
    pub host_id: Option<UserId>,
    /// Crew the hangout belongs to, if any.
    pub crew_id: Option<CrewId>,
    /// Upsert key for imported events, e.g. `"ksu:53744714532551"` (the KSU event instance id).
    pub external_id: Option<String>,
    /// Canonical link (KSU calendar page for official events).
    pub url: Option<String>,
    /// Image URL.
    pub image_url: Option<String>,
    /// Organizing department or club (official events).
    pub organizer: Option<String>,
    /// When the row was created.
    pub created_at: DateTime<Utc>,
}

impl Event {
    /// The event's time span as a window.
    pub fn window(&self) -> Result<time::Window, crate::TimeError> {
        time::Window::new(self.starts_at, self.ends_at)
    }
}

/// Raw hangout input as typed by a host (web form or Discord), before validation.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HangoutSpec {
    /// Title.
    pub title: String,
    /// Description (may be empty).
    pub description: String,
    /// Campus-local date, `YYYY-MM-DD`.
    pub date: String,
    /// Campus-local start, `HH:MM`.
    pub start: String,
    /// Campus-local end, `HH:MM`; earlier than `start` means it ends the next day.
    /// `None` = [`DEFAULT_HANGOUT_HOURS`] after the start.
    pub end: Option<String>,
    /// Venue.
    pub location: Location,
    /// Free-form tags; normalized during validation.
    pub tags: Vec<String>,
    /// Optional capacity (≥ 1).
    pub capacity: Option<u32>,
    /// Optional crew.
    pub crew_id: Option<CrewId>,
}

/// Validates a [`HangoutSpec`] and builds the hangout [`Event`].
///
/// Rules: title 1–100 chars after trimming; description ≤ 2000; location name required;
/// capacity ≥ 1; at most [`MAX_TAGS`] tags after normalization; times parsed with the strict
/// rules in [`crate::time`] (overnight end rolls to the next day); the start may be at most
/// [`PAST_START_GRACE_MINUTES`] in the past relative to `now`.
pub fn build_hangout(
    spec: &HangoutSpec,
    host: UserId,
    id: EventId,
    now: DateTime<Utc>,
) -> Result<Event, DomainError> {
    let title = spec.title.trim();
    if title.is_empty() {
        return Err(DomainError::invalid("title", "is required"));
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(DomainError::invalid(
            "title",
            format!("must be at most {MAX_TITLE_CHARS} characters"),
        ));
    }
    let description = spec.description.trim();
    if description.chars().count() > MAX_DESCRIPTION_CHARS {
        return Err(DomainError::invalid(
            "description",
            format!("must be at most {MAX_DESCRIPTION_CHARS} characters"),
        ));
    }
    let location_name = spec.location.name.trim();
    if location_name.is_empty() {
        return Err(DomainError::invalid("location", "is required"));
    }
    if spec.capacity == Some(0) {
        return Err(DomainError::invalid("capacity", "must be at least 1"));
    }
    let tags = normalize_tags(&spec.tags);
    if tags.len() > MAX_TAGS {
        return Err(DomainError::invalid(
            "tags",
            format!("at most {MAX_TAGS} tags"),
        ));
    }

    let window = if let Some(end) = &spec.end {
        time::parse_local_window(&spec.date, Some(&spec.start), Some(end))?
    } else {
        let start = time::parse_local_window(&spec.date, Some(&spec.start), None)?.start();
        time::Window::new(start, start + TimeDelta::hours(DEFAULT_HANGOUT_HOURS))?
    };
    if window.start() < now - TimeDelta::minutes(PAST_START_GRACE_MINUTES) {
        return Err(DomainError::invalid("start", "is in the past"));
    }

    let room = spec
        .location
        .room
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .map(str::to_owned);
    Ok(Event {
        id,
        kind: EventKind::Hangout,
        title: title.to_owned(),
        description: description.to_owned(),
        tags,
        location: Location {
            name: location_name.to_owned(),
            room,
            ..spec.location.clone()
        },
        starts_at: window.start(),
        ends_at: window.end(),
        capacity: spec.capacity,
        host_id: Some(host),
        crew_id: spec.crew_id,
        external_id: None,
        url: None,
        image_url: None,
        organizer: None,
        created_at: now,
    })
}

/// Normalizes one tag: trims, lowercases, drops a leading `#`, turns whitespace and `_`
/// into `-`, keeps only ASCII letters/digits/`-`, collapses repeated `-`, and caps the length
/// at [`MAX_TAG_CHARS`]. Returns `None` if nothing is left.
///
/// `"Super Smash_Bros!"` → `"super-smash-bros"`.
pub fn normalize_tag(raw: &str) -> Option<String> {
    let mut out = String::new();
    for c in raw.trim().trim_start_matches('#').chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if (c.is_whitespace() || c == '-' || c == '_') && !out.ends_with('-') {
            out.push('-');
        }
    }
    let mut tag: String = out.trim_matches('-').chars().take(MAX_TAG_CHARS).collect();
    while tag.ends_with('-') {
        tag.pop();
    }
    (!tag.is_empty()).then_some(tag)
}

/// Normalizes a list of tags, dropping empties and duplicates (first occurrence wins).
pub fn normalize_tags<S: AsRef<str>>(raw: &[S]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tag in raw.iter().filter_map(|t| normalize_tag(t.as_ref())) {
        if !out.contains(&tag) {
            out.push(tag);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-20T15:00:00Z")
            .unwrap()
            .into()
    }

    fn spec() -> HangoutSpec {
        HangoutSpec {
            title: "  Smash night ".into(),
            description: "Bring a controller".into(),
            date: "2026-11-20".into(),
            start: "19:00".into(),
            end: Some("22:00".into()),
            location: Location {
                name: "Student Center".into(),
                room: Some(" Game Room ".into()),
                ..Location::default()
            },
            tags: vec!["Smash Bros".into(), "#gaming".into(), "gaming".into()],
            capacity: Some(8),
            crew_id: None,
        }
    }

    #[test]
    fn builds_valid_hangout() {
        let host = UserId::from_u128(1);
        let e = build_hangout(&spec(), host, EventId::from_u128(2), now()).unwrap();
        assert_eq!(e.title, "Smash night");
        assert_eq!(e.kind, EventKind::Hangout);
        assert_eq!(e.tags, vec!["smash-bros", "gaming"]);
        assert_eq!(e.location.room.as_deref(), Some("Game Room"));
        assert_eq!(e.starts_at.to_rfc3339(), "2026-11-21T00:00:00+00:00");
        assert_eq!(e.ends_at - e.starts_at, TimeDelta::hours(3));
        assert_eq!(e.host_id, Some(host));
    }

    #[test]
    fn overnight_hangout_and_default_length() {
        let mut s = spec();
        s.start = "22:00".into();
        s.end = Some("01:00".into());
        let e = build_hangout(&s, UserId::from_u128(1), EventId::from_u128(2), now()).unwrap();
        assert_eq!(e.ends_at - e.starts_at, TimeDelta::hours(3));

        s.end = None;
        let e = build_hangout(&s, UserId::from_u128(1), EventId::from_u128(2), now()).unwrap();
        assert_eq!(
            e.ends_at - e.starts_at,
            TimeDelta::hours(DEFAULT_HANGOUT_HOURS)
        );
    }

    #[test]
    fn rejects_invalid_fields() {
        type Case = (fn(&mut HangoutSpec), &'static str);
        let cases: Vec<Case> = vec![
            (|s| s.title = "   ".into(), "title"),
            (|s| s.title = "x".repeat(101), "title"),
            (|s| s.description = "x".repeat(2001), "description"),
            (|s| s.location.name = String::new(), "location"),
            (|s| s.capacity = Some(0), "capacity"),
            (
                |s| s.tags = (0..11).map(|i| format!("t{i}")).collect(),
                "tags",
            ),
            (|s| s.date = "2026-11-19".into(), "start"),
        ];
        for (mutate, field) in cases {
            let mut s = spec();
            mutate(&mut s);
            match build_hangout(&s, UserId::from_u128(1), EventId::from_u128(2), now()) {
                Err(DomainError::Invalid { field: f, .. }) => assert_eq!(f, field),
                other => panic!("expected invalid {field}, got {other:?}"),
            }
        }
    }

    #[test]
    fn rejects_bad_times_with_specific_errors() {
        let mut s = spec();
        s.date = "2026-02-30".into();
        let err = build_hangout(&s, UserId::from_u128(1), EventId::from_u128(2), now());
        assert_eq!(err, Err(DomainError::Time(crate::TimeError::InvalidDate)));
        let mut s = spec();
        s.start = "7pm".into();
        let err = build_hangout(&s, UserId::from_u128(1), EventId::from_u128(2), now());
        assert_eq!(err, Err(DomainError::Time(crate::TimeError::InvalidStart)));
    }

    #[test]
    fn tag_normalization() {
        assert_eq!(
            normalize_tag("Super Smash_Bros!").as_deref(),
            Some("super-smash-bros")
        );
        assert_eq!(
            normalize_tag("  #Board  Games ").as_deref(),
            Some("board-games")
        );
        assert_eq!(normalize_tag("--a--b--").as_deref(), Some("a-b"));
        assert_eq!(normalize_tag("!!!"), None);
        assert_eq!(
            normalize_tag(&"x".repeat(40)).map(|t| t.len()),
            Some(MAX_TAG_CHARS)
        );
        assert_eq!(
            normalize_tags(&["Food", "food", " FOOD ", ""]),
            vec!["food"]
        );
    }

    #[test]
    fn kind_round_trips() {
        for k in [EventKind::Official, EventKind::Hangout] {
            assert_eq!(EventKind::parse(k.as_str()), Some(k));
        }
    }
}
