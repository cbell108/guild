# seed

Deterministic demo data: users with personas, crews, friendships, and weeks of intent and
check-in history with realistic no-shows. Pure `generate()` + `write()`.

## Public API

- `SeedConfig`, `SeedConfig::demo(now)` — generation settings (rng seed, users, crews, history weeks); demo defaults are 60 users / 10 crews / 4 weeks.
- `generate(&SeedConfig) -> Snapshot` — pure: build the whole world with no database.
- `write(store, snapshot)` — persist a generated snapshot.
- `write_availability` — persist generated availability.
- `DEMO_USER`, `DEMO_LOGIN_USERS` — the presenter persona (first on the demo login screen) and how many users are selectable.
- `user_id(n)`, `event_id(n)`, `crew_id(n)` — deterministic id lookups for seeded rows.
- `interest_vocabulary` — the tag vocabulary the catalog draws from.
- `FUTURE_WEEKS`, `ONE_OFFS_PER_WEEK` — how much upcoming data is generated.

## Depends on

- `hangouts-core` — produces a `Snapshot` of core domain types.
- `store` — persistence in `write`.

## Tests

Unit tests inline in `src/` plus `tests/write.rs` (round-trips through a store).
Run with `cargo test -p seed`.

## Notes

- Same config ⇒ same data, byte for byte (seeded ChaCha8 RNG), so `eval` results are reproducible.
- Behaviour is simulated to give the recommender real signal: personas drive interest tags,
  crew mates and friends drive RSVPs, and per-student reliability drives check-ins and no-shows.
- RSVPs are generated 0.5–7 days ahead, so upcoming events already have some intents at `now`.
- The generator is the offline source for `eval`; it never touches the network.
