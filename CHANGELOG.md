# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-20

First release of campus-hangouts for the KSU AI Club Hackathon 2026: a "For you" feed that
merges KSU's official calendar events with student-hosted hangouts, with a plain-English
explanation on every card, friend-only presence, buddy matching and student-controlled
privacy. The Discord bot is one integration of the same backend, not the core.

### Added

- Cargo workspace with ten crates: a frozen `core` domain (users, events, intents with
  waitlists, crews, friendships, availability, gear, check-ins), wire DTOs in `crates/api`
  that round-trip the `static/mock/*.json` examples, and frozen public signatures for
  `store`, `recommend` and `llm`.
- Web API (axum): health, demo users and demo login (`uid` cookie), events list and detail,
  hangout creation, set/remove intent with capacity and waitlist auto-promote on back-out,
  hybrid "For you" feed with reasons, buddy matching, calendar `.ics` export, tags, friends,
  crews with member management and a day availability grid, per-user availability, privacy
  settings, and history deletion.
- Hybrid recommender: content scoring over title/description/tag embeddings, item-item
  collaborative scoring over implicit feedback, social scoring with a trained logistic
  attendance model, weighted into one score with plain-English reasons from the top
  contributing components.
- Deterministic `HashEmbedder` (feature hashing) as the default embedder; fastembed as an
  optional cargo feature, so tests never download a model.
- Buddy matching via `similar_users` (interest + co-attendance similarity): an accepted
  friend looking for a buddy on the same event is suggested first, otherwise the most
  similar student.
- Natural-language hosting: `POST /api/parse` → `llm::parse_hangout` against a local
  OpenAI-compatible server, schema-constrained; the host page still works by hand when the
  server is down (503 with a friendly message).
- QR check-in: host-only rotating HMAC token valid for a 60 s window, with an idempotent
  check-in endpoint and an attendee confirm flow.
- Ingest: paged fetch of the KSU calendar API mapped to official events, with KSU categories
  mapped onto the interest vocabulary, plus a `feed_coverage` reporting tool.
- Seed: a deterministic demo world — 60 students with personas and interest tags, ~10 crews,
  friendships, four weeks of intent and check-in history with realistic no-shows, buddy
  requests, and crew availability for the next two days.
- Offline evaluation binary: hold out the last week of the seeded world and report
  precision@5, recall@10 and NDCG@10 for hybrid vs popularity baseline vs content-only
  (table in `docs/EVAL.md`).
- Discord bot: `/game`, `/availability` and `/gear` ported from the prototype, plus `/hangout`
  (creates the event and a Discord Scheduled Event, with intent buttons) and `/foryou`
  (ephemeral top 3 picks with their reasons).
- Frontend (plain HTML/CSS/JS, no build step; `?mock=1` for mock data): mobile-first shell
  with dark mode; For you feed with generated covers, a reason line and the "You're caught
  up" end state; event page with intent buttons, buddy toggle and add-to-calendar; host page
  with describe-it prefill and a crew availability grid; browse with kind/tag/day filters and
  text search; crews and friends pages; "Why am I seeing this?" panel; privacy settings page;
  check-in page with rotating link; friendly empty and error states with retry actions; and
  an accessibility pass.
- Docs and project files: product spec, API contract, design decisions, architecture guide,
  evaluation write-up, pitch deck, demo script, a README per crate, and mock JSON for every
  response shape.
- CI: `cargo fmt`, `cargo clippy` and `cargo test` on push and pull request.

### Changed

- Content reasons now cite the specific words in the event's title or description that
  matched, instead of a generic "similar to your interests".
- Event list, event detail and buddy status use targeted store queries instead of loading
  more of the snapshot than they need.
- The LLM defaults to the per-GPU Q3 preset served by the local router.
- Dropped unused `NotImplemented` error variants and stale lane/frozen wording from the
  crates.

### Fixed

- RSVP writes are serialized with `BEGIN IMMEDIATE` so capacity and waitlist positions
  cannot race under concurrent updates.
- Waitlisted friends now count as "interested", not "going", in friend presence.
- Private attendance is kept out of buddy matching.
- The QR endpoint never falls back to a published secret.
- Hardened the hangout prompt and added a refine step so the LLM stops inventing dates or
  misreading relative times.
- Demo seeding: crews are seeded before events, the bot uses the router's model id, and
  buddy requests come from other students so the match demo has someone to match.
- Frontend: two-column browse on desktop, unclipped avatar initials, a link-free header
  avatar, clean initials when signed out, and no 401 noise when not signed in.
