# Contributing

This guide covers local setup, the quality gate, commits, and the crate rules
that keep the workspace tidy. `CLAUDE.md` is the project guide for humans and
agents alike; where the two disagree, keep them in sync.

## Setup

You need a stable Rust toolchain (≥ 1.94, see `rust-version` in the root
`Cargo.toml`) and `cargo`. The frontend is plain HTML/CSS/JS with no build
step; `node` is only needed for the frontend unit tests.

1. Clone the repo and enable the pre-commit hook (format + clippy on every
   commit):

   ```sh
   git clone <repo> campus-hangouts
   cd campus-hangouts
   git config core.hooksPath .githooks
   ```

2. Create your environment file:

   ```sh
   cp .env.example .env
   ```

   Only fill in what you're running:

   - `DISCORD_TOKEN` — only the bot (`cargo run -p bot`) needs it.
   - `DATABASE_URL` — default `sqlite://campus.db`. The DB file is
     gitignored; never commit it.
   - `LLM_BASE_URL` / `LLM_MODEL` — the local OpenAI-compatible server. Every
     LLM feature degrades gracefully when it is down; tests never require it.
   - `BIND_ADDR`, `QR_SECRET`, `EMBEDDER`, `RUST_LOG` — see the defaults in
     `.env.example`.

3. Run what you need:

   ```sh
   cargo run -p seed      # deterministic demo data (safe to re-run)
   cargo run -p ingest    # pull official events from the KSU calendar API
   cargo run -p web       # → http://127.0.0.1:3000
   cargo run -p bot       # Discord bot; needs DISCORD_TOKEN
   ```

   Open the web app with `?mock=1` appended to the URL to serve
   `static/mock/*.json` instead of talking to the API.

## The gate

Three commands define "done"; CI (`.github/workflows/ci.yml`) runs the same
thing on every push and PR:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Formatting follows `rustfmt.toml` (edition 2024, 100-column width); the lint
policy is the `[workspace.lints]` table in the root `Cargo.toml` plus
`clippy.toml` (a pedantic subset, `missing_docs` on, `unsafe` forbidden).
`cargo check --message-format short` is a fine fast loop while iterating.

Frontend unit tests live in `static/js/tests/` and run with
`node --test static/js/tests/`.

The pre-commit hook in `.githooks/pre-commit` runs
`cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --
-D warnings` on every commit and fails the commit if either reports a problem.
Bypass it with `SKIP_HOOKS=1 git commit ...` only when you know why, and run
the full gate before pushing.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/), **one task
per commit**, with the task id from `TASKS.md` at the end of the subject:

```
feat(web): add intent endpoint (A4)
fix(store): make LIKE search escape % and _ (A2)
test(recommend): edge cases for empty snapshots (N4)
docs: crate graph and request flows (N1)
chore: foundation
```

Scope is the crate or area touched (`web`, `store`, `recommend`, `static`,
`docs`, …). Never commit `.env`, tokens, or `*.db` — they are gitignored, but
check `git status` before committing anyway.

## Crate ownership and dependency rules

The workspace layout, from `CLAUDE.md`:

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

Rules that keep the graph clean:

- **No dependency cycles.** Each crate may depend only on the crates in its
  "May depend on" column.
- `crates/core`'s package is `hangouts-core` (lib `hangouts_core`), so it
  doesn't shadow Rust's built-in `core`.
- `crates/recommend` must stay DB-free — it scores from a `core::Snapshot` —
  so it is unit-testable without SQLite.
- New external dependencies go in `[workspace.dependencies]` in the root
  `Cargo.toml`, not directly in a crate's `Cargo.toml`.

Conventions enforced across crates:

- **Times:** store `DateTime<Utc>`; parse/display in `America/New_York` via
  chrono-tz. Strict date/time parsing, zero-padded output, and an end not
  after the start means the next day (DST-safe overnight rollover).
- **sqlx:** runtime `sqlx::query()` only — never the `query!` macros — so
  builds don't need a database.
- **Store writes** take an explicit `at: DateTime<Utc>`.
- **Errors:** `thiserror` in library crates, `anyhow` in binaries. No
  `unwrap`/`expect` outside tests (see `clippy.toml`).
- **Doc comments** on every `pub` item (the `missing_docs` lint is on).
- **Tests:** unit tests live next to the code; API tests live in
  `crates/web/tests`, one file per area (see `feed.rs`, `intents.rs`).

## Frozen surfaces

Frozen after Phase 0 — no lane changes these unilaterally:

- `crates/core` — domain types and pure logic.
- `crates/api` — wire DTOs; `static/mock/*.json` must round-trip through
  them unchanged (`cargo test -p api` enforces it).
- `docs/API.md` — the API contract.
- The JSON shapes in `static/mock/`.
- The public signatures in `crates/store/src/lib.rs`,
  `crates/recommend/src/lib.rs` and `crates/llm/src/lib.rs` — implement their
  bodies in your own lane; never change a signature.

Need one of these changed? Append the request to `NEEDS.md` under your lane
and work around it meanwhile.

## Adding an endpoint

Endpoints flow through five steps, in this order:

1. **DTO in `crates/api`.** Request and response types in
   `crates/api/src/lib.rs`: `snake_case` fields, RFC 3339 UTC times, `Option`
   fields always present in responses (as `null` when empty).
2. **Route in `crates/web`.** A handler in `crates/web/src/routes/` (one
   module per area, e.g. `feed.rs`), wired into the router in
   `crates/web/src/lib.rs`.
3. **Test in `crates/web/tests`.** An integration test driving the router
   with `tower::ServiceExt::oneshot`, one file per area.
4. **`docs/API.md`.** Document the endpoint in the table (method, path, auth,
   request, response, mock file) plus a behaviour note where needed. The DTO
   types stay the source of truth for field names.
5. **Mock in `static/mock`.** One example JSON per response type.
   `cargo test -p api` proves the mock round-trips through the DTOs
   unchanged, so the `?mock=1` frontend sees the same shape as the live API.

Afterwards the frontend in `static/` can be built against the mock before the
route lands — that is the point of the DTO-first order.
