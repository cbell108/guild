//! Every `static/mock/*.json` file must round-trip through its DTO unchanged, so the
//! frontend (built against mocks) and the server (serializing these DTOs) agree exactly.
//!
//! The mapping below is the same table as "Mock files" in `docs/API.md`.

#![allow(clippy::unwrap_used, clippy::expect_used)] // test helpers may unwrap

use std::path::PathBuf;

use api::{
    ApiError, AskResponse, AvailabilityView, BuddyStatus, CheckInResponse, CrewAvailability,
    CrewDetail, CrewSummary, EventCard, EventDetail, FeedResponse, FriendsResponse, HangoutDraft,
    HealthResponse, Me, MyIntent, QrToken, TagCount, UserSummary,
};
use hangouts_core::PrivacySettings;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

fn mock_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../static/mock")
}

fn round_trip<T: DeserializeOwned + Serialize>(file: &str) {
    let path = mock_dir().join(file);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{file}: {e}"));
    let original: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{file}: {e}"));
    let typed: T = serde_json::from_value(original.clone())
        .unwrap_or_else(|e| panic!("{file} does not match its DTO: {e}"));
    let back = serde_json::to_value(&typed).unwrap();
    assert_eq!(
        back, original,
        "{file}: re-serialized JSON differs (missing/extra field or null?)"
    );
}

macro_rules! mocks {
    ($($test:ident: $file:literal => $ty:ty,)*) => {
        $(#[test] fn $test() { round_trip::<$ty>($file); })*

        #[test]
        fn no_unlisted_mock_files() {
            let listed = [$($file),*];
            for entry in std::fs::read_dir(mock_dir()).unwrap() {
                let name = entry.unwrap().file_name().into_string().unwrap();
                assert!(listed.contains(&name.as_str()), "{name} has no DTO mapping");
            }
        }
    };
}

mocks! {
    health: "health.json" => HealthResponse,
    ask: "ask.json" => AskResponse,
    demo_users: "demo-users.json" => Vec<UserSummary>,
    me: "me.json" => Me,
    events: "events.json" => Vec<EventCard>,
    event: "event.json" => EventDetail,
    feed: "feed.json" => FeedResponse,
    intent: "intent.json" => MyIntent,
    buddy: "buddy.json" => BuddyStatus,
    tags: "tags.json" => Vec<TagCount>,
    users: "users.json" => Vec<UserSummary>,
    friends: "friends.json" => FriendsResponse,
    crews: "crews.json" => Vec<CrewSummary>,
    crew: "crew.json" => CrewDetail,
    crew_availability: "crew-availability.json" => CrewAvailability,
    availability: "availability.json" => AvailabilityView,
    parse: "parse.json" => HangoutDraft,
    qr: "qr.json" => QrToken,
    checkin: "checkin.json" => CheckInResponse,
    privacy: "privacy.json" => PrivacySettings,
    error_llm: "error-llm.json" => ApiError,
}
