# web

The axum JSON API (contract in [../../docs/API.md](../../docs/API.md), wire types in the `api` crate)
plus serving the plain HTML/CSS/JS frontend from `static/`.

## Public API

- `app(state)` — the full router: all `/api/*` routes, unknown `/api` paths as JSON 404s, and everything else falling through to `static/` (SPA fallback to `index.html`).
- `AppState`, `Clock` — what the app holds: store, LLM client, embedder, static dir, and an injectable clock.
- `auth`, `embedding`, `error`, `routes`, `state`, `view` — the modules: session handling, embedding cache, `WebError`, the route handlers, app state, and view helpers.

## Depends on

- `hangouts-core` — domain types.
- `api` — the wire DTOs every handler speaks.
- `store` — persistence.
- `recommend` — the `/api/feed` recommender.
- `llm` — the `/api/parse` natural-language feature.

## Tests

API tests live in `tests/` (`auth`, `events`, `feed`, `intents`, `parse`, `privacy`, `privacy_surfaces`, `social`, `buddy`, `checkin`, `health`, `static_and_calendar`), driving `app` with `tower::ServiceExt::oneshot` and no network.
Run with `cargo test -p web`.

## Notes

- The local LLM degrades gracefully: when it is down, `/api/parse` returns a 503 and tests never require a running server.
- Privacy defaults hold at the API boundary: attendance stays private, friends see friends only, no public attendee lists.
- The frontend is plain HTML/CSS/JS in `../../static/` with no build step; `?mock=1` reads `static/mock/*.json`.
- Wire shapes come from the frozen `api` crate, not from this crate.
