//! iCalendar (`.ics`) export of one event.

use axum::extract::{Path, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use hangouts_core::Event;

use super::event_id;
use crate::error::{WebError, WebResult};
use crate::state::AppState;

/// Escapes text values per RFC 5545 (backslash, semicolon, comma, newline).
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\r' => {}
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out
}

/// Folds a content line to at most 75 octets per physical line (continuations start with a space).
fn fold(line: &str) -> String {
    let mut out = String::new();
    let mut octets = 0;
    for c in line.chars() {
        let width = c.len_utf8();
        if octets + width > 74 {
            out.push_str("\r\n ");
            octets = 1;
        }
        out.push(c);
        octets += width;
    }
    out
}

fn utc(t: DateTime<Utc>) -> String {
    t.format("%Y%m%dT%H%M%SZ").to_string()
}

/// Lowercase ASCII slug for the download filename.
fn slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let slug = out.trim_matches('-');
    if slug.is_empty() {
        "event".to_owned()
    } else {
        slug.to_owned()
    }
}

/// Renders the event as an iCalendar document with CRLF line endings.
pub fn render(event: &Event, stamp: DateTime<Utc>) -> String {
    let mut location = event.location.name.clone();
    if let Some(room) = &event.location.room {
        location = format!("{location}, {room}");
    }
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_owned(),
        "VERSION:2.0".to_owned(),
        "PRODID:-//campus-hangouts//EN".to_owned(),
        "CALSCALE:GREGORIAN".to_owned(),
        "BEGIN:VEVENT".to_owned(),
        format!("UID:{}@campus-hangouts", event.id),
        format!("DTSTAMP:{}", utc(stamp)),
        format!("DTSTART:{}", utc(event.starts_at)),
        format!("DTEND:{}", utc(event.ends_at)),
        format!("SUMMARY:{}", escape(&event.title)),
        format!("LOCATION:{}", escape(&location)),
        format!("DESCRIPTION:{}", escape(&event.description)),
    ];
    if let Some(url) = &event.url {
        lines.push(format!("URL:{}", escape(url)));
    }
    lines.push("END:VEVENT".to_owned());
    lines.push("END:VCALENDAR".to_owned());
    let mut out: String = lines.iter().map(|l| fold(l) + "\r\n").collect();
    out.shrink_to_fit();
    out
}

/// `GET /api/events/{id}/calendar.ics`.
pub async fn ics(State(state): State<AppState>, Path(id): Path<String>) -> WebResult<Response> {
    let id = event_id(&id)?;
    let event = state
        .store
        .get_event(id)
        .await?
        .ok_or_else(|| WebError::not_found("Event"))?;
    let body = render(&event, state.now());
    let disposition = format!("attachment; filename=\"{}.ics\"", slug(&event.title));
    let mut response = body.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/calendar; charset=utf-8"),
    );
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_and_folds() {
        assert_eq!(escape("a,b;c\\d\ne"), "a\\,b\\;c\\\\d\\ne");
        let folded = fold(&"x".repeat(160));
        assert!(folded.split("\r\n").all(|l| l.len() <= 75));
        assert_eq!(folded.replace("\r\n ", "").len(), 160);
    }

    #[test]
    fn slugs() {
        assert_eq!(slug("Smash Night!! 2"), "smash-night-2");
        assert_eq!(slug("!!!"), "event");
    }
}
