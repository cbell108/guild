# api

Wire DTOs for the JSON API described in [../../docs/API.md](../../docs/API.md).
Every `static/mock/*.json` fixture round-trips through these types.
**Frozen after foundation** — changes go through `NEEDS.md`.

## Public API

- `HealthResponse`, `ErrorCode`, `ApiError` — health check and the uniform error body.
- `UserSummary`, `Me` — user profiles and the signed-in view.
- `EventCard`, `EventDetail` — list and detail event payloads.
- `CreateHangoutRequest` — hangout creation payload.
- `FeedItem`, `FeedResponse` — the ranked feed with reasons.
- `FriendPresence`, `MyIntent`, `SetIntentRequest` — presence and RSVP payloads.
- `FriendRequest` — friend request payloads.
- `CrewSummary`, `CrewDetail` — crew payloads.
- `SetAvailabilityRequest`, `CrewAvailability` — availability payloads.
- `ParseRequest` — natural-language hangout parsing payload.
- `QrToken`, `CheckInRequest`, `CheckInResponse` — QR check-in payloads.

## Depends on

- `hangouts-core` — the DTOs wrap core domain types.

## Tests

Unit tests inline in `src/` plus `tests/mocks.rs` (round-trips the mock JSON files).
Run with `cargo test -p api`.

## Notes

- DTOs are a boundary: web and bot both speak these shapes, so they are frozen with the API.
- The contract they implement is [../../docs/API.md](../../docs/API.md).
- Errors surface as `ApiError` with an `ErrorCode` the frontend can match on.
