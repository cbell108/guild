# bot

The poise Discord bot: one integration into the app, not the core.
`cargo run -p bot` needs `DISCORD_TOKEN` in `.env`.

## Public API

Binary crate — no library API. Its surface is the slash commands:

- `/game` — open the live gameroom menu.
- `/availability` — set and view the group's availability for a date.
- `/gear` — declare the setup and game you bring to meetups.
- `/hangout` — host a hangout: posts it in Discord, adds a Discord event, and shows it on the web app.
- `/foryou` — your top 3 picks for the week, with reasons (only you can see this).

The internal modules are `commands.rs` (the command handlers), `service.rs` (snapshot + recommender glue), and `helpers.rs` (Discord display helpers).

## Depends on

- `hangouts-core` — domain types.
- `store` — persistence and `load_snapshot`.
- `recommend` — attendance model and feed for `/foryou`.

## Tests

Unit tests live inline in `src/` (`#[cfg(test)]` in `helpers.rs` and `service.rs`).
Run with `cargo test -p bot`.

## Notes

- Reads `DISCORD_TOKEN`, `DATABASE_URL` (default `sqlite://campus.db`) and `WEB_BASE_URL` from the environment or `.env`; never commits tokens.
- Uses only non-privileged gateway intents — presence is self-reported by the user, never read from Discord.
- Trains the `AttendanceModel` from the snapshot at startup and falls back to `AttendanceModel::prior()` when there is none.
- Shares the `recommend` and `store` crates with the web app, so the two surfaces stay consistent.
