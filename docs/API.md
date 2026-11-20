# API contract — FROZEN after Phase 0

Every request/response body is a Rust type in `crates/api/src/lib.rs` (re-exported core types
where noted). This document describes what the endpoints *do*; the types are the source of truth
for field names. `static/mock/*.json` contains one example per response type and
`cargo test -p api` proves the mocks round-trip through those types unchanged.

## Conventions
- JSON everywhere except `.ics`. Field names `snake_case`. Enums are `snake_case` strings.
- Times are RFC 3339 **UTC** (`2026-11-21T00:00:00Z`). The frontend formats them in
  `America/New_York` (`Intl.DateTimeFormat(…, {timeZone: "America/New_York"})`).
  Request fields typed as text (`date`, `start`, `end`) are campus-local `YYYY-MM-DD` / `HH:MM`
  and follow `hangouts_core::time` rules: strict parsing, zero-padded output, and an end not after
  the start means the next day (DST-safe).
- `Option` fields are always present in responses (`null` when empty) and may be omitted in requests.
- **Session:** demo login sets an HTTP-only cookie `uid=<user id>` (`SameSite=Lax`, `Path=/`).
  Endpoints marked 🔒 return 401 `unauthorized` without it. Endpoints not marked work logged out
  (personal fields such as `my_intent` are then `null` and `friends` is all zeros).
- **Errors:** non-2xx responses have body `ApiError` → `{"error":{"code":"…","message":"…"}}`.
  `message` is safe to show to the user. Codes → status: `bad_request` 400, `invalid_token` 400,
  `unauthorized` 401, `forbidden` 403, `not_found` 404, `conflict` 409, `llm_unavailable` 503,
  `internal` 500. Validation errors from core (`DomainError`, `TimeError`) map to `bad_request` with
  their Display text as the message.
- IDs are UUID strings. Unknown or malformed ids → 404 `not_found`.

## Privacy rules (normative)
1. No endpoint returns an attendee list or a total attendee count. The only counts are
   `FriendPresence` (the viewer's accepted friends) and `spots_left` (capacity minus confirmed spots).
2. A friend is counted in `FriendPresence` only if the friendship is `accepted` **and** the friend's
   `privacy.attendance` is `friends`. `private` users are never counted or sampled.
3. `UserSummary` is the only user shape shown to others: id, display name, bio. Interests and
   privacy settings appear only in the viewer's own `Me`.
4. Buddy matching (`/buddy`) only suggests users who are themselves looking on that event and have
   `privacy.buddy_matching = true`. Friends are suggested first.
5. With `privacy.personalize = false` the feed uses only the user's interests and popularity (Lane C).
6. Crew details and crew availability are visible to members only (403 `forbidden` otherwise).
   No member counts in `CrewSummary`.

## Endpoints

| # | Method & path | Auth | Request | Response (2xx) | Mock file |
|---|---|---|---|---|---|
| 1 | `GET /api/health` | | — | `HealthResponse` | `health.json` |
| 2 | `GET /api/demo-users` | | — | `[UserSummary]` | `demo-users.json` |
| 3 | `POST /api/login` | | `LoginRequest` | `Me` + sets `uid` cookie | `me.json` |
| 4 | `POST /api/logout` | | — | 204, clears cookie | — |
| 5 | `GET /api/me` | 🔒 | — | `Me` | `me.json` |
| 6 | `GET /api/users?q=` | 🔒 | `UserSearchQuery` (query) | `[UserSummary]` (max 20, excludes self) | `users.json` |
| 7 | `GET /api/events` | | `EventListQuery` (query) | `[EventCard]` by `starts_at` | `events.json` |
| 8 | `GET /api/events/{id}` | | — | `EventDetail` | `event.json` |
| 9 | `POST /api/events` | 🔒 | `CreateHangoutRequest` | 201 `EventDetail` | `event.json` |
| 10 | `PUT /api/events/{id}/intent` | 🔒 | `SetIntentRequest` | `MyIntent` | `intent.json` |
| 11 | `DELETE /api/events/{id}/intent` | 🔒 | — | 204 | — |
| 12 | `GET /api/feed` | 🔒 | — | `FeedResponse` | `feed.json` |
| 13 | `GET /api/events/{id}/buddy` | 🔒 | — | `BuddyStatus` | `buddy.json` |
| 14 | `POST /api/events/{id}/buddy` | 🔒 | `BuddyToggleRequest` | `BuddyStatus` | `buddy.json` |
| 15 | `GET /api/events/{id}/calendar.ics` | | — | `text/calendar` (one VEVENT, UTC times) | — |
| 16 | `GET /api/tags` | | — | `[TagCount]` (upcoming events, most common first, max 30) | `tags.json` |
| 17 | `GET /api/friends` | 🔒 | — | `FriendsResponse` | `friends.json` |
| 18 | `POST /api/friends` | 🔒 | `FriendRequest` | `FriendsResponse` | `friends.json` |
| 19 | `POST /api/friends/{user_id}/accept` | 🔒 | — | `FriendsResponse` | `friends.json` |
| 20 | `DELETE /api/friends/{user_id}` | 🔒 | — | `FriendsResponse` | `friends.json` |
| 21 | `GET /api/crews` | 🔒 | — | `[CrewSummary]` (viewer's crews) | `crews.json` |
| 22 | `POST /api/crews` | 🔒 | `CreateCrewRequest` | 201 `CrewDetail` | `crew.json` |
| 23 | `GET /api/crews/{id}` | 🔒 | — | `CrewDetail` | `crew.json` |
| 24 | `POST /api/crews/{id}/members` | 🔒 | `AddCrewMemberRequest` | `CrewDetail` | `crew.json` |
| 25 | `DELETE /api/crews/{id}/members/{user_id}` | 🔒 | — | `CrewDetail` | `crew.json` |
| 26 | `GET /api/crews/{id}/availability?date=` | 🔒 | `CrewAvailabilityQuery` (query) | `CrewAvailability` | `crew-availability.json` |
| 27 | `PUT /api/me/availability` | 🔒 | `SetAvailabilityRequest` | `AvailabilityView` | `availability.json` |
| 28 | `POST /api/parse` | 🔒 | `ParseRequest` | `HangoutDraft` (core) | `parse.json` (503: `error-llm.json`) |
| 29 | `GET /api/events/{id}/qr` | 🔒 host | — | `QrToken` | `qr.json` |
| 30 | `POST /api/checkin` | 🔒 | `CheckInRequest` | `CheckInResponse` | `checkin.json` |
| 31 | `GET /api/me/privacy` | 🔒 | — | `PrivacySettings` (core) | `privacy.json` |
| 32 | `PUT /api/me/privacy` | 🔒 | `PrivacySettings` (core) | `PrivacySettings` | `privacy.json` |
| 33 | `DELETE /api/me/history` | 🔒 | — | 204 | — |
| — | `GET /` and any non-`/api` path | | — | files from `static/` (`index.html` fallback) | — |

## Behaviour notes

**3 Login.** `user_id` must be an existing user (demo users in practice) → else 404.

**7 Events list.** Defaults: `from` = now (events that haven't ended), no `to`, `limit` 50, max 200.
`tags` is comma-separated and matches any. `q` searches title and description, case-insensitive.

**9 Create hangout.** Body → `HangoutSpec` (`From` impl) → `hangouts_core::build_hangout` with the
viewer as host. Validation errors → 400 with the core message. A `crew_id` the host isn't a member
of → 403. Errors name the bad field (e.g. "Invalid start time, expected HH:MM.").

**10 Set intent.** `going_with` requires `with` (an accepted friend of the viewer, else 400).
Capacity: `Interested` never uses a spot; `Going`/`Going with` take one spot each; when full the
intent is stored `waitlisted` with a 1-based `waitlist_position`. Changing level keeps a held spot.
Hosts may set intents on their own hangouts like anyone else.

**11 Remove intent** (the one-tap "can't make it"). Frees the spot and auto-promotes the earliest
waitlisted user. Idempotent: 204 even if there was no intent.

**12 Feed.** `recommend::feed` over a fresh `store.load_snapshot()`: events starting in the next
7 days, max 10, best first. Always `caught_up: true` (the UI ends the list with "You're caught up").
Events the viewer already marked `going`/`going_with` stay in the feed. Each item carries the same
`EventCard` as the events list (with `friends` and `my_intent`).

**13/14 Buddy.** POST toggles "looking for a buddy" and returns the new status. `matched` is chosen
among *other users looking on the same event*: an accepted friend first (`is_friend: true`, reason
e.g. "Jordan is looking too"), otherwise the top result of `recommend::similar_users` with its
first reason. `null` when not looking or nobody matches yet.

**15 Calendar.** `BEGIN:VCALENDAR` / one `VEVENT` with `UID:<event id>@campus-hangouts`,
`DTSTART`/`DTEND` in UTC (`20261121T000000Z`), `SUMMARY`, `LOCATION`, `DESCRIPTION`.
`Content-Disposition: attachment; filename="<slug>.ics"`.

**18–20 Friends.** POST sends a request (or accepts it if the other user already asked you);
accept requires a pending request from `{user_id}`; DELETE removes a friend or cancels/declines a
request. Befriending yourself → 400.

**22–25 Crews.** The creator becomes `owner`. Only owners add or remove others; anyone may remove
themself. Crew names are 1–60 chars.

**26 Crew availability.** For the campus-local `date`: grid of `slot_minutes` cells (default 60,
min 15) over that whole local day, from members' availability windows that overlap it;
`hangouts_core::slots` + `best_slot`. Non-member → 403.

**27 Availability.** Same rule as the bot's `/availability`: one window per user per local date
(replaces the earlier one); defaults 00:00–23:59; end ≤ start ⇒ ends next day (`overnight: true`).

**28 Parse.** `llm::parse_hangout(text, now)`. If the LLM is down or not built
(`LlmError::Unavailable` / `NotImplemented`) → 503 `llm_unavailable`; `BadResponse` → 503 too.
The Host page must keep working without it.

**29/30 QR check-in.** Host-only (`forbidden` otherwise). Token = URL-safe base64 of
HMAC-SHA256(`QR_SECRET`, `"<event id>:<unix time / 60>"`), valid for the current and previous
60 s window. `POST /api/checkin` with an expired or forged token → 400 `invalid_token`.
Check-ins are idempotent.

**33 Delete history.** Removes the viewer's intents, check-ins, buddy requests and availability.
The account, friendships and crews stay.

## Mock mode (frontend)
With `?mock=1`, the frontend reads `static/mock/<file>` (table above) instead of calling the API.
Writes resolve to the same file as the matching read (e.g. `PUT …/intent` → `intent.json`);
204 endpoints resolve to nothing. Path parameters are ignored (every event id returns
`event.json`). The mocks share ids, so links between them work.
