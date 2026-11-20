# core

Domain types and pure logic: entity types, time parsing, and availability-overlap rules.
No I/O, no async, no database. **Frozen after foundation** — changes go through `NEEDS.md`.

## Public API

- `User`, `PrivacySettings`, `Visibility` — a person and their privacy defaults.
- `Event`, `EventKind`, `Location` — official events and user hangouts (see `event.rs` limits).
- `build_hangout`, `HangoutSpec`, `HangoutDraft` — construct a validated hangout from a draft.
- `normalize_tag`, `normalize_tags` — clean interest-tag input.
- `Intent`, `IntentKind`, `IntentLevel`, `IntentStatus`, `admission`, `next_to_promote` — RSVP model and promotion rules.
- `CheckIn` — an attendance record.
- `CrewId`, `EventId`, `UserId` — deterministic id types (see `ids.rs`).
- `Snapshot` — the read model passed to recommenders (see `snapshot.rs`).
- `Availability`, `Slot`, `Window`, `best_slot`, `slots` — recurring availability and overlap logic.
- `Reason`, `ReasonKind`, `Recommendation`, `ScoreBreakdown` — explainable recommendation outputs.
- `Gear`, `Setup`, `Presence`, `PresenceReport` — supporting domain types.
- `TimeError`, `DomainError` — error types.

## Depends on

- serde, chrono, chrono-tz, thiserror, uuid only. No other workspace crate.

## Tests

Unit tests live inline in `src/` (`#[cfg(test)]` modules). Run with
`cargo test -p hangouts-core`.

## Notes

- The package is named `hangouts-core` (lib `hangouts_core`) so it does not shadow Rust's built-in `core`.
- Times are stored as `DateTime<Utc>`; display/parsing is in `America/New_York` via chrono-tz.
- Time parsing is strict: zero-padded dates, explicit DST-safe overnight rollover (see `time.rs`).
- See [../../docs/SPEC.md](../../docs/SPEC.md) for the domain rules this crate implements.
