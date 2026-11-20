//! Serde shapes for the subset of the Localist `/api/2/events` response we use.
//! Unknown fields are ignored; every field we read is optional or defaulted, so a sparse
//! event never fails the whole page.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RawPage {
    #[serde(default)]
    pub events: Vec<RawEventWrapper>,
    pub page: Option<RawPageInfo>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawPageInfo {
    pub next_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawEventWrapper {
    pub event: RawEvent,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct RawEvent {
    pub title: String,
    pub url: Option<String>,
    pub localist_url: Option<String>,
    pub photo_url: Option<String>,
    pub location: Option<String>,
    pub location_name: Option<String>,
    pub room_number: Option<String>,
    pub status: Option<String>,
    pub experience: Option<String>,
    pub stream_url: Option<String>,
    pub private: bool,
    pub rejected: bool,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,
    pub description_text: Option<String>,
    pub geo: Option<RawGeo>,
    pub departments: Vec<RawNamed>,
    pub filters: RawFilters,
    pub event_instances: Vec<RawInstanceWrapper>,
}

impl RawEvent {
    /// Live, public, not rejected.
    pub fn is_listable(&self) -> bool {
        !self.private && !self.rejected && self.status.as_deref().is_none_or(|s| s == "live")
    }

    /// Virtual or hybrid.
    pub fn is_online(&self) -> bool {
        matches!(self.experience.as_deref(), Some("virtual" | "hybrid"))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct RawGeo {
    pub latitude: Option<String>,
    pub longitude: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct RawNamed {
    pub name: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct RawFilters {
    pub event_types: Vec<RawNamed>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawInstanceWrapper {
    pub event_instance: RawInstance,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawInstance {
    pub id: u64,
    pub start: String,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub all_day: bool,
}
