# store

sqlx + SQLite repository functions: the single `Store` gateway for all persistence.

## Public API

- `Store` — the repository handle.
  - `Store::connect(url)` — open a SQLite database from a URL.
  - `Store::in_memory()` — an ephemeral database for tests.
  - `Store::migrate()` — apply the migrations in `../../migrations/`.
  - `Store::load_snapshot()` — read the whole world as a `hangouts_core::Snapshot` for the recommender.
  - Other methods per area: `users`, `events`, `intents`, `checkins`, `friends`, `crews`, `buddy`, `prototype`.
- `StoreError`, `Result<T>` — error type and alias.
- `EventFilter` — list filtering options.
- `RemoveOutcome` — what a removal deleted (and cascaded).

## Depends on

- `hangouts-core` — the rows map to core domain types.

## Tests

Unit tests inline in `src/` plus the integration suite in `tests/store.rs` (runs against an in-memory database).
Run with `cargo test -p store`.

## Notes

- Runtime `sqlx::query()` only, never the `query!` macro — builds must not need a database.
- Every write method takes an explicit `at: DateTime<Utc>`; nothing reads the clock.
- Schema lives in `../../migrations/` and is applied by `Store::migrate`, not by the migrations tool.
- Times are stored as UTC `DateTime<Utc>`.
