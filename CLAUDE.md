# campus-hangouts — agent guide

Web app for KSU campus events and informal hangouts, with a recommender.
The Discord bot is one integration, not the core. Product spec: `docs/SPEC.md`.
API contract: `docs/API.md`. Decisions: `docs/DECISIONS.md`. Work queue: `TASKS.md`.

Hackathon: KSU AI Club Hackathon 2026. Submission due **Fri Nov 20, 12:00 PM ET**.
Judged on the AI Club rubric and the Nudox track (most organized codebase), so
structure, naming, docs and tests count as much as features.

## Layout
| Path | Owns | May depend on |
|---|---|---|
| `crates/core` | Domain types + pure logic (time parsing, availability overlap). No I/O, no async. **FROZEN after foundation.** | serde, chrono, chrono-tz, thiserror, uuid |
| `crates/api` | Wire DTOs for docs/API.md; `static/mock/*.json` round-trip through them. **FROZEN after foundation.** | core, serde |
| `crates/store` | sqlx + SQLite repository functions | core |
| `crates/recommend` | Embeddings, collaborative, social, hybrid scorer, reasons, user similarity. Takes data in; never touches the DB. | core |
| `crates/llm` | Client for the local OpenAI-compatible server; natural-language → hangout draft | core |
| `crates/ingest` | KSU calendar API (`calendar.kennesaw.edu/api/2/events`) → official events | core, store |
| `crates/seed` | Deterministic demo data (users, crews, friendships, intent history); pure `generate()` + `write()` | core, store |
| `crates/eval` | Offline recommender evaluation binary → `docs/EVAL.md` | core, seed, recommend |
| `crates/web` | axum JSON API (see docs/API.md) + serves `static/` | core, store, recommend, llm |
| `crates/bot` | poise Discord bot | core, store, recommend |
| `static/` | Plain HTML/CSS/JS, no build step; `?mock=1` reads `static/mock/*.json` | — |
| `migrations/` | sqlx migrations | — |

The core crate's package is `hangouts-core` (lib `hangouts_core`), so it doesn't shadow Rust's built-in `core`.
No dependency cycles. `recommend` must stay DB-free so it is unit-testable.

## Commands
- Format: `cargo fmt --all`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Test: `cargo test --workspace`
- Run web: `cargo run -p web` → http://127.0.0.1:3000
- Run bot: `cargo run -p bot` (needs `DISCORD_TOKEN` in `.env`)
- Seed: `cargo run -p seed`
- Ingest: `cargo run -p ingest`
- Prefer `cargo check --message-format short` while iterating.

## Conventions
- Times: store `DateTime<Utc>`; parse/display in `America/New_York` via chrono-tz.
  Keep the prototype's rules: strict date/time parsing, zero-padded dates, DST-safe overnight rollover.
- sqlx: runtime `sqlx::query()` only, never `query!` macros (builds must not need a database).
- Store writes take an explicit `at: DateTime<Utc>`; recommend takes a `core::Snapshot`.
- Errors: `thiserror` in library crates, `anyhow` in binaries. No `unwrap`/`expect` outside tests.
- Every `pub` item gets a doc comment. Unit tests live next to the code; API tests in `crates/web/tests`.
- Local LLM at `LLM_BASE_URL` (default `http://127.0.0.1:8080/v1`). Features degrade gracefully when it is down;
  tests never require it.
- Privacy defaults: attendance private; friends see friends only; no public attendee lists or follower counts.
- Commits: Conventional Commits, one task per commit, e.g. `feat(web): add intent endpoint (A4)`.
- Never commit `.env`, tokens, or `*.db`.

## Rules for parallel lanes
- Edit only your lane's crates/paths (see TASKS.md). Never edit `crates/core`, `crates/api`, `docs/API.md`, mock shapes, frozen signatures, or another lane's files.
- Need a core or API change? Append it to `NEEDS.md` and work around it.
- A task that fails after 3 attempts → write what you tried in `BLOCKED.md`, skip it.
- Done means: fmt + clippy + tests pass, committed, box ticked in TASKS.md.
