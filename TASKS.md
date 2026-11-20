# TASKS

Priority order inside each lane: **MUST** first, then STRETCH. Tick `[x]` when done (fmt + clippy + tests green, committed).

## Phase 0 — Foundation (interactive, Opus, before overnight launch)
- [x] F1 Cargo workspace with all crates from CLAUDE.md stubbed; `cargo build --workspace` passes.
- [x] F2 `core` types frozen: User, Event {kind: Official|Hangout, tags, capacity?, location, starts/ends UTC}, IntentLevel {Interested, Going, GoingWith(UserId)}, Intent, Crew, Friendship, Availability, Gear, CheckIn, Recommendation {event_id, score, reasons: Vec<Reason>}, Reason {kind: Content|Collaborative|Social|Popular, text}. Port prototype time parsing + DST tests from ../DiscordSchedulerBot.
- [x] F3 `store` public async fn signatures frozen (stubs return a `NotImplemented` error, not panics).
- [x] F4 `recommend` + `llm` public fn signatures frozen (stubs), incl. `recommend::feed(...)`, `recommend::similar_users(...)`, `llm::parse_hangout(text) -> HangoutDraft`.
- [x] F5 `docs/API.md` (every endpoint in Lane A with request/response JSON) + matching `static/mock/*.json`.
- [x] F6 Tooling: `[workspace.lints]` (clippy pedantic subset), `rustfmt.toml`, `.githooks/pre-commit` (fmt+clippy), `.env.example`, README skeleton, `.gitignore` incl. `.claude/worktrees/`.
- [x] F7 All green; commit `chore: foundation`.

## Lane A — web + store (Sonnet) · owns `crates/store`, `crates/web`, `migrations/`
Your branch has Lane C's crates as placeholders: `recommend::feed` returns upcoming events unscored, `similar_users` returns candidates in order, `llm::parse_hangout` returns `NotImplemented` (→ 503). Build and test against those.
- [x] A1 MUST Migrations: users (+ interests, privacy columns, discord_id unique), events (+ `embedding BLOB`, external_id unique), intents (level, with_user, status, created_at — the waitlist is `status = waitlisted`), crews, crew_members, friendships, buddy_requests, availability, gear, presence, checkins. Shapes follow the frozen `Store` signatures and core types. Accept: `cargo test -p store` migrates an in-memory DB.
- [x] A2 MUST Implement every frozen store fn with tests.
- [x] A3 MUST `GET /api/events`, `GET /api/events/:id`, `POST /api/events` (hangout). Accept: tests in `crates/web/tests` using `tower::ServiceExt::oneshot`.
- [x] A4 MUST `PUT/DELETE /api/events/:id/intent` incl. waitlist auto-promote on back-out when capacity is set.
- [x] A5 MUST Demo login: `GET /api/demo-users`, `POST /api/login {user_id}` sets `uid` cookie.
- [x] A6 MUST `GET /api/feed` → `recommend::feed` over `store.load_snapshot()`, top 10 for the next 7 days + `caught_up: true` at the end; friend presence counts (friends only). Hold `recommend::default_embedder()` and `AttendanceModel::train(..)` in `AppState`; backfill `events_missing_embedding()` at startup and embed new hangouts on create.
- [x] A7 MUST Buddy match: `POST /api/events/:id/buddy` (toggle looking), `GET /api/events/:id/buddy` → friend first, else `recommend::similar_users` among others looking.
- [x] A8 MUST Serve `static/` at `/`; `GET /api/events/{id}/calendar.ics`.
- [x] A9 Friends + crews endpoints.
- [x] A10 `POST /api/parse` → `llm::parse_hangout`; 503 with a friendly message if the LLM is down.
- [x] A11 STRETCH Check-in: rotating HMAC QR token (60 s window) `GET /api/events/:id/qr`, `POST /api/checkin`.
- [x] A12 STRETCH Privacy settings endpoints (visibility toggles, delete history).

## Lane B — frontend (Sonnet) · owns `static/` (not `static/mock/` shapes — those follow docs/API.md)
Accept for every task: works with `?mock=1` and against the live API; no console errors; mobile width 390 px looks right.
- [x] B1 MUST Shell + design tokens; nav: For you · Browse · Crews · Host.
- [x] B2 MUST For you feed: cards with title, time, place, friend presence ("Jordan + 2 friends going"), one reason line, "You're caught up" end state.
- [x] B3 MUST Event page: details, Interested / Going / Going with ___ buttons, "looking for a buddy" toggle + match display, add-to-calendar link.
- [x] B4 MUST Host page: hangout form + "describe it" box that calls `/api/parse` to prefill; crew availability grid.
- [x] B5 Browse: filter official/hangout, tags, date; text search.
- [x] B6 Crews page.
- [x] B7 "Why am I seeing this?" panel on cards (shows reasons) + privacy settings page.
- [x] B8 STRETCH Check-in page (host shows rotating QR; attendee scans).
- [x] B9 Polish + accessibility pass; save screenshots to `docs/screenshots/` for the deck.

## Lane C — recommend + llm + ingest + seed + eval + bot (Opus) · owns those crates
Your branch has Lane A's `store` as stubs (`StoreError::NotImplemented`). Test pure functions: `seed::generate`, `ingest::map_page` (fixture), recommender modules, bot helpers. Store-backed paths are checked after merge.
- [x] C1 MUST `ingest`: page through the KSU API (`days=30&pp=100`), map to official Events, upsert. Save one real response as a test fixture; tests use the fixture, not the network.
- [x] C2 MUST `seed`: deterministic (fixed RNG seed) 60 demo users with personas/interest tags, ~10 crews, friendships, 4 weeks of intent + check-in history with realistic no-shows; includes the user's own demo persona.
- [x] C3 MUST Content scorer: fastembed embeddings of title+description+tags; user profile = mean of interest tags + attended events; cosine.
- [x] C4 MUST Social scorer: friend boost + attendance probability (logistic regression on intent level, days ahead, friends going, past show rate; trained on seed history; prior fallback).
- [x] C5 MUST Hybrid `feed()`: weighted sum, configurable weights, reasons from top contributing component(s) in plain English. Tests assert reasons.
- [x] C6 MUST `similar_users()` for buddy matching (interest + co-attendance similarity).
- [x] C7 MUST Collaborative scorer: item-item over implicit feedback (check-in 3, going 2, interested 1).
- [x] C8 MUST Offline eval binary (`crates/eval`, on `seed::generate()`): hold out last week, precision@5 + recall@10 for hybrid vs popularity baseline vs content-only; write table to `docs/EVAL.md` (pitch slide).
- [x] C9 `llm::parse_hangout` via JSON-schema-constrained chat completion; unit-test the prompt/parse with a recorded response.
- [x] C10 `bot`: port `/game`, `/availability`, `/gear` from the prototype to poise; `/hangout` creates an Event + Discord Scheduled Event; buttons write intents. Test pure helpers; no token needed overnight.
- [x] C11 STRETCH `bot` `/foryou` → ephemeral top 3 with reasons.

## Lane N — night queue (local model, RTX 5080) · docs + tests only, never `static/`
- [x] N1 `docs/ARCHITECTURE.md`: crate dependency graph (mermaid `flowchart`), request flow for `GET /api/feed` and `PUT /api/events/{id}/intent` (mermaid `sequenceDiagram`), data model (mermaid `erDiagram` from `migrations/0001_init.sql`), and where each design rule from `docs/DECISIONS.md` is enforced (file paths). Link it from `README.md`.
- [ ] N2 `CHANGELOG.md` (Keep a Changelog format) for 0.1.0, grouped Added / Changed / Fixed, written from `git log --oneline` (summarize; no commit hashes needed).
- [ ] N3 `CONTRIBUTING.md`: setup, the fmt/clippy/test gate and pre-commit hook, Conventional Commits with task ids, crate ownership and the dependency rules from `CLAUDE.md`, how to add an endpoint (DTO in `crates/api` → route in `crates/web` → test in `crates/web/tests` → `docs/API.md` → mock in `static/mock`). Plus `.editorconfig` (utf-8, lf, 4-space Rust, 2-space JS/CSS/HTML/JSON/YAML/MD).
- [ ] N4 More `crates/recommend` unit tests in a new file `crates/recommend/tests/edge_cases.rs`: empty snapshot → empty feed; user with no interests and no history still gets a non-empty feed with reasons; events outside the 7-day horizon never appear; `limit` respected; scores sorted descending; every item has at least one reason.

## Lane M — night queue (local model, RX 9070 XT) · tests only, never `static/`
- [x] M1 More `crates/ingest` tests in `crates/ingest/tests/mapping_edge_cases.rs` using inline JSON: virtual event with no location → "Online"; HTML entities in titles decoded or kept readable; duplicate instance ids deduplicated; events with an end before the start get the default length; tags normalized and capped.
- [ ] M2 More `crates/store` tests in a new file `crates/store/tests/edge_cases.rs`: `search_users` treats `%` and `_` literally; `list_events` with `tags` matches any tag; `remove_friend` works in both directions; `delete_history` keeps friendships and crews; `set_availability` replaces the same local date only.
- [ ] M3 Bot helper tests in `crates/bot/tests/` for message formatting helpers (roster text, availability chart lines incl. "(overnight)", gear list) against the functions in `crates/bot/src/helpers.rs`.
