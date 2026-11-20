//! Wire types (DTOs) for every endpoint in `docs/API.md`. **FROZEN after Phase 0.**
//!
//! The web server serializes exactly these types, and `static/mock/*.json` must round-trip
//! through them unchanged (see `tests/mocks.rs`), so the frontend built against the mocks
//! works against the live API.
//!
//! Conventions: field names are `snake_case`; times are RFC 3339 UTC; `Option` fields are
//! always present in responses (as `null` when empty) and may be omitted in requests.

use chrono::{DateTime, Utc};
use hangouts_core::{
    CrewId, CrewRole, DomainError, EventId, EventKind, HangoutSpec, IntentKind, IntentLevel,
    IntentStatus, Location, PrivacySettings, Reason, ScoreBreakdown, User, UserId,
};
use serde::{Deserialize, Serialize};

pub use hangouts_core::HangoutDraft;

// --- System -------------------------------------------------------------------------------

/// `GET /api/health`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Always `true` when the server answers.
    pub ok: bool,
    /// Whether the local LLM answered its health check (natural-language hosting available).
    pub llm: bool,
}

/// Machine-readable error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// 400: malformed or invalid input.
    BadRequest,
    /// 401: no valid `uid` cookie.
    Unauthorized,
    /// 403: logged in but not allowed (e.g. not a crew owner).
    Forbidden,
    /// 404: no such resource (or not visible to you).
    NotFound,
    /// 409: conflicts with current state.
    Conflict,
    /// 503: the local LLM is down; the form still works manually.
    LlmUnavailable,
    /// 400: QR token expired or forged.
    InvalidToken,
    /// 500: anything else.
    Internal,
}

/// The `error` object inside [`ApiError`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    /// Machine-readable code.
    pub code: ErrorCode,
    /// User-facing message (safe to show verbatim).
    pub message: String,
}

/// Body of every non-2xx JSON response: `{"error":{"code":"…","message":"…"}}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    /// The error.
    pub error: ErrorBody,
}

impl ApiError {
    /// Builds an error body.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            error: ErrorBody {
                code,
                message: message.into(),
            },
        }
    }
}

// --- Users & session ----------------------------------------------------------------------

/// The public face of a user: never includes attendance, interests, or contact details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSummary {
    /// Id.
    pub id: UserId,
    /// Display name.
    pub display_name: String,
    /// Short bio.
    pub bio: Option<String>,
}

impl From<&User> for UserSummary {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            display_name: user.display_name.clone(),
            bio: user.bio.clone(),
        }
    }
}

/// `POST /api/login` body (demo login: pick a seeded user).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginRequest {
    /// A user from `GET /api/demo-users`.
    pub user_id: UserId,
}

/// `GET /api/me` (also the `POST /api/login` response).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Me {
    /// The logged-in user.
    pub user: UserSummary,
    /// Their interest tags (only ever shown to themself).
    pub interests: Vec<String>,
    /// Their privacy settings.
    pub privacy: PrivacySettings,
}

/// `GET /api/users?q=` query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSearchQuery {
    /// Case-insensitive name substring.
    pub q: String,
}

// --- Events -------------------------------------------------------------------------------

/// Friends' attendance on an event, as visible to the viewer.
///
/// Only accepted friends whose attendance visibility is `friends` are counted. There is no
/// total attendee count and no attendee list.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FriendPresence {
    /// Friends Going (incl. Going with).
    pub going: u32,
    /// Friends Interested.
    pub interested: u32,
    /// Up to 2 of those friends to name on the card, Going first ("Jordan + 2 friends going").
    pub sample: Vec<UserSummary>,
}

/// The viewer's own intent on an event (also the `PUT /api/events/{id}/intent` response).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MyIntent {
    /// Level.
    pub level: IntentKind,
    /// Companion for `going_with`.
    pub with: Option<UserSummary>,
    /// Confirmed or waitlisted.
    pub status: IntentStatus,
    /// 1-based waitlist position when waitlisted.
    pub waitlist_position: Option<u32>,
}

/// `PUT /api/events/{id}/intent` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetIntentRequest {
    /// Level.
    pub level: IntentKind,
    /// Required for `going_with`, ignored otherwise.
    pub with: Option<UserId>,
}

impl SetIntentRequest {
    /// Converts to the domain level (fails if `going_with` has no companion).
    pub fn to_level(&self) -> Result<IntentLevel, DomainError> {
        IntentLevel::from_parts(self.level, self.with)
    }
}

/// An event as shown on a card (feed, browse).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventCard {
    /// Id.
    pub id: EventId,
    /// Official or hangout.
    pub kind: EventKind,
    /// Title.
    pub title: String,
    /// Start.
    pub starts_at: DateTime<Utc>,
    /// End.
    pub ends_at: DateTime<Utc>,
    /// Venue.
    pub location: Location,
    /// Tags.
    pub tags: Vec<String>,
    /// Capacity, if capped.
    pub capacity: Option<u32>,
    /// Remaining spots, if capped (never negative).
    pub spots_left: Option<u32>,
    /// Host (hangouts only).
    pub host: Option<UserSummary>,
    /// Image.
    pub image_url: Option<String>,
    /// Friends going / interested.
    pub friends: FriendPresence,
    /// The viewer's intent; `null` when none or logged out.
    pub my_intent: Option<MyIntent>,
}

/// `GET /api/events/{id}` (also the `POST /api/events` response): the card plus details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventDetail {
    /// All card fields, flattened into this object.
    #[serde(flatten)]
    pub card: EventCard,
    /// Plain-text description.
    pub description: String,
    /// External link (KSU calendar page).
    pub url: Option<String>,
    /// Organizing department / club.
    pub organizer: Option<String>,
    /// Crew, for crew hangouts.
    pub crew: Option<CrewSummary>,
    /// Whether the viewer is looking for a buddy for this event.
    pub buddy_looking: bool,
    /// Relative link to the `.ics` download.
    pub calendar_url: String,
}

/// `GET /api/events` query string.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EventListQuery {
    /// `official` or `hangout`.
    pub kind: Option<EventKind>,
    /// Comma-separated tags; matches events with any of them.
    pub tags: Option<String>,
    /// Only events ending after this instant (default: now).
    pub from: Option<DateTime<Utc>>,
    /// Only events starting before this instant.
    pub to: Option<DateTime<Utc>>,
    /// Case-insensitive text search in title/description.
    pub q: Option<String>,
    /// Max results (default 50, max 200).
    pub limit: Option<u32>,
}

/// Venue as typed in the Host form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationInput {
    /// Building / venue.
    pub name: String,
    /// Room.
    pub room: Option<String>,
}

/// `POST /api/events` body: create a hangout. Times are campus-local text, validated by
/// `hangouts_core::build_hangout` (strict parsing; an end before the start means next day).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateHangoutRequest {
    /// Title.
    pub title: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// `HH:MM`.
    pub start: String,
    /// `HH:MM`; `null` = 2 hours after the start.
    pub end: Option<String>,
    /// Venue.
    pub location: LocationInput,
    /// Tags (normalized server-side).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Capacity (≥ 1).
    pub capacity: Option<u32>,
    /// Crew (the host must be a member).
    pub crew_id: Option<CrewId>,
}

impl From<CreateHangoutRequest> for HangoutSpec {
    fn from(req: CreateHangoutRequest) -> Self {
        HangoutSpec {
            title: req.title,
            description: req.description,
            date: req.date,
            start: req.start,
            end: req.end,
            location: Location {
                name: req.location.name,
                room: req.location.room,
                ..Location::default()
            },
            tags: req.tags,
            capacity: req.capacity,
            crew_id: req.crew_id,
        }
    }
}

/// One entry of `GET /api/tags`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagCount {
    /// Tag.
    pub tag: String,
    /// Upcoming events with it.
    pub count: u32,
}

// --- Feed & buddies -----------------------------------------------------------------------

/// One feed card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedItem {
    /// The event.
    pub event: EventCard,
    /// Recommender score.
    pub score: f64,
    /// Reasons, most important first; the card shows the first.
    pub reasons: Vec<Reason>,
    /// Component contributions for "Why am I seeing this?".
    pub breakdown: ScoreBreakdown,
}

/// `GET /api/feed`: top picks for the next 7 days. Finite by design.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedResponse {
    /// Up to 10 items, best first.
    pub items: Vec<FeedItem>,
    /// Always `true`: render "You're caught up" after the last item.
    pub caught_up: bool,
}

/// `POST /api/events/{id}/buddy` body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuddyToggleRequest {
    /// Turn "looking for a buddy" on or off.
    pub looking: bool,
}

/// A suggested buddy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuddyMatch {
    /// Who.
    pub user: UserSummary,
    /// A friend (preferred) or a similar student.
    pub is_friend: bool,
    /// Why ("You both like board games and went to Game Night").
    pub reason: String,
}

/// `GET`/`POST /api/events/{id}/buddy` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuddyStatus {
    /// Whether the viewer is looking.
    pub looking: bool,
    /// Best match among others looking; `null` if not looking or nobody fits yet.
    pub matched: Option<BuddyMatch>,
}

// --- Friends & crews ----------------------------------------------------------------------

/// `POST /api/friends` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendRequest {
    /// Who to befriend.
    pub user_id: UserId,
}

/// `GET /api/friends` (also returned by every friends write).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendsResponse {
    /// Accepted friends.
    pub friends: Vec<UserSummary>,
    /// Requests waiting for the viewer.
    pub incoming: Vec<UserSummary>,
    /// Requests the viewer sent.
    pub outgoing: Vec<UserSummary>,
}

/// A crew without its members (no member counts by design).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrewSummary {
    /// Id.
    pub id: CrewId,
    /// Name.
    pub name: String,
    /// Description.
    pub description: String,
}

/// A crew member as shown to other members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrewMemberView {
    /// Who.
    pub user: UserSummary,
    /// Role.
    pub role: CrewRole,
}

/// `GET /api/crews/{id}` (members only; also returned by crew writes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrewDetail {
    /// Crew fields, flattened into this object.
    #[serde(flatten)]
    pub crew: CrewSummary,
    /// Members, owner first.
    pub members: Vec<CrewMemberView>,
}

/// `POST /api/crews` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateCrewRequest {
    /// Name (1–60 chars).
    pub name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
}

/// `POST /api/crews/{id}/members` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddCrewMemberRequest {
    /// Who to add (owner only).
    pub user_id: UserId,
}

// --- Availability -------------------------------------------------------------------------

/// `PUT /api/me/availability` body (same rules as the bot's `/availability`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetAvailabilityRequest {
    /// `YYYY-MM-DD`.
    pub date: String,
    /// `HH:MM`, default 00:00.
    pub start: Option<String>,
    /// `HH:MM`, default 23:59; not after `start` = next day.
    pub end: Option<String>,
}

/// `PUT /api/me/availability` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailabilityView {
    /// Normalized zero-padded date.
    pub date: String,
    /// Start.
    pub starts_at: DateTime<Utc>,
    /// End.
    pub ends_at: DateTime<Utc>,
    /// Ends on a later local date.
    pub overnight: bool,
}

/// `GET /api/crews/{id}/availability` query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrewAvailabilityQuery {
    /// `YYYY-MM-DD`.
    pub date: String,
    /// Grid cell size in minutes (default 60, min 15).
    pub slot_minutes: Option<u32>,
}

/// One grid cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotView {
    /// Start.
    pub starts_at: DateTime<Utc>,
    /// End.
    pub ends_at: DateTime<Utc>,
    /// Members free for the whole cell.
    pub available: Vec<UserId>,
}

/// `GET /api/crews/{id}/availability`: the crew grid for one local day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrewAvailability {
    /// Normalized date.
    pub date: String,
    /// Cell size.
    pub slot_minutes: u32,
    /// Crew members (to label `available` ids).
    pub members: Vec<UserSummary>,
    /// Cells covering the local day.
    pub slots: Vec<SlotView>,
    /// Cell with the most members free (earliest on ties); `null` if nobody is free.
    pub best: Option<SlotView>,
}

// --- Natural-language hosting -------------------------------------------------------------

/// `POST /api/parse` body. The response is a [`HangoutDraft`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseRequest {
    /// e.g. "smash tonight 7–10 game room, I have 2 switches".
    pub text: String,
}

// --- Check-in -----------------------------------------------------------------------------

/// `GET /api/events/{id}/qr` (host only): a rotating check-in token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QrToken {
    /// Opaque token (HMAC over event id + 60 s window).
    pub token: String,
    /// When it stops being accepted.
    pub expires_at: DateTime<Utc>,
    /// Relative URL to encode in the QR code.
    pub checkin_url: String,
}

/// `POST /api/checkin` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckInRequest {
    /// Event.
    pub event_id: EventId,
    /// Token from the QR code.
    pub token: String,
}

/// `POST /api/checkin` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckInResponse {
    /// Event.
    pub event_id: EventId,
    /// Always `true` on success (repeat check-ins are fine).
    pub checked_in: bool,
    /// When the check-in was recorded.
    pub at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_intent_requires_companion_for_going_with() {
        let req = SetIntentRequest {
            level: IntentKind::GoingWith,
            with: None,
        };
        assert!(req.to_level().is_err());
        let req: SetIntentRequest = serde_json::from_str(r#"{"level":"going"}"#).unwrap();
        assert_eq!(req.to_level(), Ok(IntentLevel::Going));
    }

    #[test]
    fn create_request_defaults_and_conversion() {
        let req: CreateHangoutRequest = serde_json::from_str(
            r#"{"title":"Smash","date":"2026-11-20","start":"19:00","location":{"name":"SC"}}"#,
        )
        .unwrap();
        assert!(req.tags.is_empty() && req.end.is_none());
        let spec = HangoutSpec::from(req);
        assert_eq!(spec.location.name, "SC");
    }
}
