# Decisions

Short records of choices that shape the codebase. "Prototype" = the DiscordSchedulerBot C++/D++ bot
this project grew out of.

## Carried over from the prototype
- **Presence is self-reported** through buttons ("I'm here" / "I'm interested" / "I'm out").
  No Discord privileged intents (`GUILD_PRESENCES`), no presence tracking. → `core::Presence`.
- **Buttons and menus over free text**; bot replies to individuals are **ephemeral**.
- **All user data lives in our SQLite DB**, never in Discord state. The DB file and tokens are never committed.
- **Gear & games:** setup is `console | laptop | other` (trimmed, case-insensitive); a blank game
  title is rejected; declarations are idempotent. → `core::Gear`, `core::Setup`.
- **Availability:** one window per user per local date; a new one replaces it; defaults
  00:00–23:59. → `core::time::local_window`, `store::Store::set_availability`.
- **Strict date/time parsing** (prototype fix `a8443b3`): reject trailing junk, impossible dates,
  out-of-range years and times, and reply with which input was wrong instead of silently using
  defaults. → `core::time::{parse_date, parse_time, parse_local_window}`.
- **Zero-padded dates** (fix `ad75048`): `2026-1-1` is accepted and normalized to `2026-01-01`
  everywhere downstream. → `core::time::format_date`.
- **DST-safe overnight rollover** (fix `1a1fda4`): an end not after the start moves to the next
  *calendar day* and is resolved in the zone again, never `+86400 s`; local midnight is resolved by
  the zone. Nonexistent local times shift forward like `mktime`; ambiguous ones take the earlier
  instant. Tests in `crates/core/src/time.rs` pin the 2026 transitions (Mar 8, Nov 1).

## Made in Phase 0 (foundation)
- **Frozen contracts for parallel lanes.** `crates/core`, `crates/api`, `docs/API.md` and the
  listed public signatures of `store`, `recommend` and `llm` were fixed before three agents built
  against them overnight. Change requests go to `NEEDS.md`.
- **Wire types are Rust (`crates/api`), not just prose.** `static/mock/*.json` must round-trip
  through them (`cargo test -p api`), so the mock-driven frontend and the server can't drift.
- **Snapshot in, results out.** `recommend` takes a `core::Snapshot` (all data, in memory at
  hackathon scale) instead of querying the DB. It stays pure and unit-testable, and `eval` runs on
  `seed::generate()` with no database.
- **Explicit `at` on every store write.** Seed can backdate four weeks of history, and tests are
  deterministic.
- **Runtime `sqlx::query()` only**, no `query!` macros, so building never needs a live
  `DATABASE_URL` (matters in fresh worktrees).
- **Offline-first embeddings.** `recommend::HashEmbedder` (feature hashing, deterministic) is the
  default and the test embedder; fastembed is an optional cargo feature. Tests never download a model.
- **Intent levels:** `Interested` never takes a spot; `Going` / `Going with` take one each; full ⇒
  waitlist in `created_at` order with auto-promote on back-out. → `core::admission`, `core::next_to_promote`.
- **Privacy defaults:** attendance visible to friends only (there is no "public" option), buddy
  matching opt-out, personalization opt-out. No attendee lists or follower/member counts anywhere.
- **Times:** stored as `DateTime<Utc>`, shown in `America/New_York`; API text inputs are
  campus-local `YYYY-MM-DD` / `HH:MM` so the Host form, the bot and the LLM draft share one parser.
- **Calendar export path** is `/api/events/{id}/calendar.ics`, because axum can't route `{id}.ics`
  next to `{id}`.
