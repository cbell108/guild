//! A hangout draft produced from natural language (e.g. by the local LLM).

use serde::{Deserialize, Serialize};

use crate::event::normalize_tags;
use crate::time::{format_date, format_time, parse_date, parse_time};

/// Prefill values for the Host form. Every field is optional; the host edits and submits.
///
/// Date/time fields use the same text formats as [`crate::HangoutSpec`] so the frontend can
/// drop them straight into the form.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HangoutDraft {
    /// Suggested title.
    pub title: Option<String>,
    /// Suggested description.
    pub description: Option<String>,
    /// Campus-local date, zero-padded `YYYY-MM-DD`.
    pub date: Option<String>,
    /// Campus-local start, `HH:MM`.
    pub start: Option<String>,
    /// Campus-local end, `HH:MM`.
    pub end: Option<String>,
    /// Venue name (and room, free text).
    pub location: Option<String>,
    /// Normalized tags.
    pub tags: Vec<String>,
    /// Suggested capacity.
    pub capacity: Option<u32>,
    /// Gear the host mentioned ("2 Switches").
    pub gear_notes: Option<String>,
}

impl HangoutDraft {
    /// Applies the strict rules to model output: invalid dates/times are dropped (left for
    /// the host to fill), valid ones are zero-padded, blank strings become `None`, tags are
    /// normalized, and a zero capacity is dropped.
    #[must_use]
    pub fn normalized(self) -> Self {
        let clean = |s: Option<String>| s.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
        Self {
            title: clean(self.title),
            description: clean(self.description),
            date: self.date.and_then(|d| parse_date(&d).ok()).map(format_date),
            start: self
                .start
                .and_then(|t| parse_time(&t).ok())
                .map(format_time),
            end: self.end.and_then(|t| parse_time(&t).ok()).map(format_time),
            location: clean(self.location),
            tags: normalize_tags(&self.tags),
            capacity: self.capacity.filter(|c| *c > 0),
            gear_notes: clean(self.gear_notes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_model_output() {
        let raw = HangoutDraft {
            title: Some(" Smash night ".into()),
            description: Some("  ".into()),
            date: Some("2026-11-2".into()),
            start: Some("7:00".into()),
            end: Some("10pm".into()),
            location: Some("Game Room".into()),
            tags: vec!["Smash".into(), "smash".into()],
            capacity: Some(0),
            gear_notes: Some("2 switches".into()),
        };
        let d = raw.normalized();
        assert_eq!(d.title.as_deref(), Some("Smash night"));
        assert_eq!(d.description, None);
        assert_eq!(d.date.as_deref(), Some("2026-11-02"));
        assert_eq!(d.start.as_deref(), Some("07:00"));
        assert_eq!(d.end, None);
        assert_eq!(d.tags, vec!["smash"]);
        assert_eq!(d.capacity, None);
    }

    #[test]
    fn missing_fields_deserialize_as_defaults() {
        let d: HangoutDraft = serde_json::from_str(r#"{"title":"x"}"#).unwrap();
        assert_eq!(d.title.as_deref(), Some("x"));
        assert_eq!(d.tags, Vec::<String>::new());
    }
}
