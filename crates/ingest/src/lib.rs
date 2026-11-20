//! Imports official events from the KSU public calendar API
//! (`calendar.kennesaw.edu/api/2/events`, Localist). One [`hangouts_core::Event`] per event
//! instance, upserted by `external_id = "ksu:<instance id>"`.
//!
//! [`map_page`] is pure and tested against a saved response in `tests/fixtures/`;
//! [`run`] does the network paging and the store writes.

mod localist;
mod vocab;

pub use vocab::{interest_tags, mapped_interests};

use std::collections::BTreeSet;

use chrono::{DateTime, TimeDelta, Utc};
use hangouts_core::event::{MAX_DESCRIPTION_CHARS, MAX_TAGS, MAX_TITLE_CHARS};
use hangouts_core::time::{local_date, local_day};
use hangouts_core::{Event, EventId, EventKind, Location, normalize_tags};
use store::{Store, StoreError};
use thiserror::Error;
use uuid::Uuid;

use localist::{RawEvent, RawInstance, RawPage};

/// Default API endpoint.
pub const KSU_EVENTS_URL: &str = "https://calendar.kennesaw.edu/api/2/events";

/// Length given to a timed event that has no end in the feed.
pub const DEFAULT_EVENT_HOURS: i64 = 1;

/// Hard cap on pages fetched in one run (guards against a looping `next_page`).
pub const MAX_PAGES: u32 = 50;

/// Namespace for deterministic event ids derived from external ids (UUID v5).
const EVENT_ID_NAMESPACE: Uuid = Uuid::from_u128(0x6b73_752d_6576_656e_7473_2d6e_616d_6573);

/// Ingest errors.
#[derive(Debug, Error)]
pub enum IngestError {
    /// Network / HTTP failure.
    #[error("fetch failed: {0}")]
    Http(String),
    /// Unexpected JSON.
    #[error("parse failed: {0}")]
    Parse(String),
    /// Database failure.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Ingest settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestConfig {
    /// API endpoint.
    pub url: String,
    /// Days ahead to import.
    pub days: u32,
    /// Page size.
    pub per_page: u32,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            url: KSU_EVENTS_URL.to_owned(),
            days: 30,
            per_page: 100,
        }
    }
}

/// What a run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IngestReport {
    /// Event instances fetched.
    pub fetched: usize,
    /// Rows inserted or updated.
    pub upserted: usize,
}

/// The external id for a KSU event instance, e.g. `"ksu:53744714532551"`.
pub fn external_id(instance_id: u64) -> String {
    format!("ksu:{instance_id}")
}

/// A stable event id derived from an external id, so re-imports into a fresh database give
/// the same ids (the store still keeps an existing row's id on upsert).
pub fn event_id_for(external_id: &str) -> EventId {
    EventId(Uuid::new_v5(&EVENT_ID_NAMESPACE, external_id.as_bytes()))
}

/// Maps one API response page (raw JSON body) to events. Pure; tested against a fixture.
///
/// Rules: one event per instance (duplicate instance ids keep the first); private, rejected,
/// or non-live events are skipped; HTML is never used (`description_text` only); tags come
/// from the event's tags, keywords and event types; virtual events without a venue are
/// placed "Online"; all-day instances span the campus-local day; a missing end means
/// [`DEFAULT_EVENT_HOURS`]. `now` becomes `created_at`.
pub fn map_page(body: &str, now: DateTime<Utc>) -> Result<Vec<Event>, IngestError> {
    let page: RawPage =
        serde_json::from_str(body).map_err(|e| IngestError::Parse(e.to_string()))?;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for wrapper in &page.events {
        let raw = &wrapper.event;
        if !raw.is_listable() {
            continue;
        }
        for inst in &raw.event_instances {
            let inst = &inst.event_instance;
            if !seen.insert(inst.id) {
                continue;
            }
            if let Some(event) = map_instance(raw, inst, now) {
                out.push(event);
            }
        }
    }
    Ok(out)
}

/// The page number after `body`'s page, if any (from `page.next_page`).
pub fn next_page(body: &str) -> Result<Option<u32>, IngestError> {
    let page: RawPage =
        serde_json::from_str(body).map_err(|e| IngestError::Parse(e.to_string()))?;
    Ok(page.page.and_then(|p| p.next_page))
}

fn map_instance(raw: &RawEvent, inst: &RawInstance, now: DateTime<Utc>) -> Option<Event> {
    let title = truncate(raw.title.trim(), MAX_TITLE_CHARS);
    if title.is_empty() {
        return None;
    }
    let start = DateTime::parse_from_rfc3339(&inst.start).ok()?.to_utc();
    let (starts_at, ends_at) = if inst.all_day {
        let day = local_day(local_date(start));
        (day.start(), day.end())
    } else {
        let end = inst
            .end
            .as_deref()
            .and_then(|e| DateTime::parse_from_rfc3339(e).ok())
            .map(|e| e.to_utc())
            .filter(|e| *e > start)
            .unwrap_or(start + TimeDelta::hours(DEFAULT_EVENT_HOURS));
        (start, end)
    };

    let mut raw_tags: Vec<&str> = raw.tags.iter().map(String::as_str).collect();
    raw_tags.extend(raw.keywords.iter().map(String::as_str));
    raw_tags.extend(raw.filters.event_types.iter().map(|t| t.name.as_str()));
    if raw.is_online() {
        raw_tags.push("virtual");
    }
    let ksu_tags = normalize_tags(&raw_tags);
    let department = raw.departments.first().map(|d| d.name.as_str());
    // Interest-vocabulary tags first, so they survive the cap and lead the card.
    let mut tags = normalize_tags(&interest_tags(&ksu_tags, department, &title));
    for tag in ksu_tags {
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    tags.truncate(MAX_TAGS);

    let external = external_id(inst.id);
    Some(Event {
        id: event_id_for(&external),
        kind: EventKind::Official,
        title,
        description: truncate(
            raw.description_text.as_deref().unwrap_or("").trim(),
            MAX_DESCRIPTION_CHARS,
        ),
        tags,
        location: location(raw),
        starts_at,
        ends_at,
        capacity: None,
        host_id: None,
        crew_id: None,
        external_id: Some(external),
        url: non_empty(raw.localist_url.as_deref()).or_else(|| non_empty(raw.url.as_deref())),
        image_url: non_empty(raw.photo_url.as_deref()),
        organizer: raw
            .departments
            .first()
            .and_then(|d| non_empty(Some(&d.name))),
        created_at: now,
    })
}

fn location(raw: &RawEvent) -> Location {
    let venue =
        non_empty(raw.location_name.as_deref()).or_else(|| non_empty(raw.location.as_deref()));
    let name = venue.unwrap_or_else(|| {
        if raw.is_online() {
            "Online".to_owned()
        } else {
            "TBA".to_owned()
        }
    });
    let coord = |s: Option<&String>| s.and_then(|v| v.trim().parse::<f64>().ok());
    let online_url = non_empty(raw.stream_url.as_deref()).map(|u| {
        if u.starts_with("http://") || u.starts_with("https://") {
            u
        } else {
            format!("https://{u}")
        }
    });
    Location {
        name,
        room: non_empty(raw.room_number.as_deref()),
        lat: raw.geo.as_ref().and_then(|g| coord(g.latitude.as_ref())),
        lon: raw.geo.as_ref().and_then(|g| coord(g.longitude.as_ref())),
        online_url,
    }
}

fn non_empty(s: Option<&str>) -> Option<String> {
    s.map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
}

fn truncate(s: &str, max_chars: usize) -> String {
    s.chars()
        .take(max_chars)
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// The request URL for one page, e.g. `…/events?days=30&pp=100&page=2`.
pub fn page_url(config: &IngestConfig, page: u32) -> String {
    let sep = if config.url.contains('?') { '&' } else { '?' };
    format!(
        "{}{sep}days={}&pp={}&page={page}",
        config.url, config.days, config.per_page
    )
}

/// Fetches every page and upserts the events.
pub async fn run(store: &Store, config: &IngestConfig) -> Result<IngestReport, IngestError> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("campus-hangouts-ingest/0.1")
        .build()
        .map_err(|e| IngestError::Http(e.to_string()))?;
    run_with(store, Utc::now(), |page| {
        let url = page_url(config, page);
        let http = http.clone();
        async move {
            http.get(&url)
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
                .map_err(|e| IngestError::Http(e.to_string()))?
                .text()
                .await
                .map_err(|e| IngestError::Http(e.to_string()))
        }
    })
    .await
}

/// The paging and upsert loop behind [`run`], with the page fetch injected (`fetch(page)`
/// returns the raw JSON body of that page). Lets tests feed saved responses without a network.
pub async fn run_with<F, Fut>(
    store: &Store,
    now: DateTime<Utc>,
    mut fetch: F,
) -> Result<IngestReport, IngestError>
where
    F: FnMut(u32) -> Fut,
    Fut: std::future::Future<Output = Result<String, IngestError>>,
{
    let mut report = IngestReport::default();
    let mut page = 1_u32;
    loop {
        let body = fetch(page).await?;
        let events = map_page(&body, now)?;
        report.fetched += events.len();
        for event in &events {
            store.upsert_official_event(event).await?;
            report.upserted += 1;
        }
        tracing::info!(page, events = events.len(), "ingested page");
        match next_page(&body)? {
            Some(next) if next > page && next <= MAX_PAGES => page = next,
            _ => break,
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-19T12:00:00Z")
            .unwrap()
            .to_utc()
    }

    fn minimal(instances: &str, extra: &str) -> String {
        format!(
            r#"{{"events":[{{"event":{{"id":1,"title":"Test","status":"live",{extra}
            "event_instances":[{instances}]}}}}],"page":{{"current":1,"next_page":null}}}}"#
        )
    }

    #[test]
    fn missing_end_defaults_and_all_day_spans_local_day() {
        let body = minimal(
            r#"{"event_instance":{"id":10,"start":"2026-11-20T09:00:00-05:00","end":null,"all_day":false}},
               {"event_instance":{"id":11,"start":"2026-11-21T00:00:00-05:00","end":null,"all_day":true}},
               {"event_instance":{"id":10,"start":"2026-11-20T09:00:00-05:00","end":null,"all_day":false}}"#,
            "",
        );
        let events = map_page(&body, now()).unwrap();
        assert_eq!(events.len(), 2, "duplicate instance dropped");
        assert_eq!(
            events[0].ends_at - events[0].starts_at,
            TimeDelta::hours(DEFAULT_EVENT_HOURS)
        );
        assert_eq!(
            events[1].starts_at.to_rfc3339(),
            "2026-11-21T05:00:00+00:00"
        );
        assert_eq!(
            events[1].ends_at - events[1].starts_at,
            TimeDelta::hours(24)
        );
        assert_eq!(events[0].location.name, "TBA");
        assert_eq!(events[0].external_id.as_deref(), Some("ksu:10"));
    }

    #[test]
    fn private_and_rejected_are_skipped() {
        let inst = r#"{"event_instance":{"id":10,"start":"2026-11-20T09:00:00-05:00"}}"#;
        assert_eq!(
            map_page(&minimal(inst, r#""private":true,"#), now())
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            map_page(&minimal(inst, r#""rejected":true,"#), now())
                .unwrap()
                .len(),
            0
        );
        assert_eq!(map_page(&minimal(inst, ""), now()).unwrap().len(), 1);
    }

    #[test]
    fn bad_json_is_a_parse_error() {
        assert!(matches!(
            map_page("not json", now()),
            Err(IngestError::Parse(_))
        ));
    }

    #[test]
    fn page_url_has_paging_params() {
        assert_eq!(
            page_url(&IngestConfig::default(), 2),
            "https://calendar.kennesaw.edu/api/2/events?days=30&pp=100&page=2"
        );
    }

    #[test]
    fn ids_are_stable() {
        assert_eq!(event_id_for("ksu:1"), event_id_for("ksu:1"));
        assert_ne!(event_id_for("ksu:1"), event_id_for("ksu:2"));
    }
}
