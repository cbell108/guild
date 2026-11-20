# ingest

Imports official events from the KSU public calendar API
(`calendar.kennesaw.edu/api/2/events`, Localist) into the store as official `Event`s.

## Public API

- `IngestConfig` — endpoint, days ahead, page size (defaults: KSU URL, 30 days, 100/page).
- `IngestReport` — fetched and upserted counts.
- `run`, `run_with` — fetch the pages and upsert the events.
- `map_page` — pure: one Localist JSON page → `Vec<Event>` (validated against core limits).
- `next_page` — pure: extract the next page number from a response body.
- `external_id`, `event_id_for` — `ksu:<instance id>` keys and the deterministic `EventId` (UUID v5).
- `page_url` — build the URL for a page of the config.
- `interest_tags`, `mapped_interests` — map Localist category names to app interest tags.
- `IngestError` — error type.

## Depends on

- `hangouts-core` — produces `Event`/`Location` domain types.
- `store` — upserts through the repository.

## Tests

Unit tests inline in `src/` plus `tests/` (`fixture.rs`, `store.rs`, `vocab.rs`);
`map_page` is tested against a saved response in `tests/fixtures/`.
Run with `cargo test -p ingest`.

## Notes

- Each event instance is upserted by `external_id = "ksu:<instance id>"`, so re-runs are idempotent.
- `EventId`s are derived from the external id (UUID v5), so the same instance always maps to the same row.
- `map_page` is pure and network-free; only `run` does HTTP, so the mapping logic is testable offline.
- Timed events without an end get `DEFAULT_EVENT_HOURS` (1h); paging is capped at `MAX_PAGES` (50).
