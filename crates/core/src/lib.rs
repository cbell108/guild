//! Domain types and pure logic for campus-hangouts.
//!
//! This crate has no I/O and no async. Everything else in the workspace builds on these
//! types, and they are **frozen after Phase 0**: lanes that need a change append it to
//! `NEEDS.md` instead of editing this crate.
//!
//! Time rules (ported from the DiscordSchedulerBot prototype) live in [`time`]: strict
//! date/time parsing, zero-padded dates, and DST-safe overnight rollover.

pub mod availability;
pub mod checkin;
pub mod draft;
pub mod error;
pub mod event;
pub mod gear;
pub mod ids;
pub mod intent;
pub mod presence;
pub mod rec;
pub mod snapshot;
pub mod social;
pub mod time;
pub mod user;

pub use availability::{Availability, Slot, best_slot, slots};
pub use checkin::CheckIn;
pub use draft::HangoutDraft;
pub use error::{DomainError, TimeError};
pub use event::{
    Event, EventKind, HangoutSpec, Location, build_hangout, normalize_tag, normalize_tags,
};
pub use gear::{Gear, Setup};
pub use ids::{CrewId, EventId, UserId};
pub use intent::{Intent, IntentKind, IntentLevel, IntentStatus, admission, next_to_promote};
pub use presence::{Presence, PresenceReport};
pub use rec::{Reason, ReasonKind, Recommendation, ScoreBreakdown};
pub use snapshot::Snapshot;
pub use social::{BuddyRequest, Crew, CrewMember, CrewRole, Friendship, FriendshipStatus};
pub use time::Window;
pub use user::{PrivacySettings, User, Visibility};
