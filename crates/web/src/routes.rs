//! Route handlers, grouped by area.

pub mod auth;
pub mod events;
pub mod intents;

use hangouts_core::EventId;

use crate::error::WebError;

/// Parses a path id; malformed ids are 404 like unknown ones.
pub fn parse_id<T: std::str::FromStr>(text: &str, what: &str) -> Result<T, WebError> {
    text.parse().map_err(|_| WebError::not_found(what))
}

/// Parses an event id path segment.
pub fn event_id(text: &str) -> Result<EventId, WebError> {
    parse_id(text, "Event")
}
