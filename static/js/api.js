/**
 * API client for the frontend.
 *
 * Live mode: calls the JSON API in docs/API.md (relative `/api/...` paths,
 * same-origin `uid` cookie for 🔒 endpoints).
 *
 * Mock mode (`?mock=1`): instead of the API, reads `static/mock/<file>` —
 * one file per response type, path parameters ignored. Writes resolve to the
 * same file as the matching read (e.g. `PUT /api/events/{id}/intent` →
 * `intent.json`); 204 endpoints resolve to nothing. See the "Mock mode"
 * section of docs/API.md; the route table below mirrors it one-to-one.
 */

/** Error for a non-2xx API response (or a failed mock lookup). */
export class ApiHttpError extends Error {
  /**
   * @param {number} status HTTP status.
   * @param {string} code machine-readable error code (`ApiError.error.code`).
   * @param {string} message user-safe message.
   */
  constructor(status, code, message) {
    super(message);
    this.name = "ApiHttpError";
    this.status = status;
    this.code = code;
    this.message = message;
  }
}

/** True when the page was loaded with `?mock=1` (any value other than `0`/`false`). */
export function isMock() {
  const value = new URLSearchParams(window.location.search).get("mock");
  return value !== null && value !== "0" && value !== "false";
}

/** Minimal synthetic ICS so the add-to-calendar link works in mock mode. */
const MOCK_ICS = [
  "BEGIN:VCALENDAR",
  "VERSION:2.0",
  "PRODID:-//campus-hangouts//mock//EN",
  "BEGIN:VEVENT",
  "UID:mock-event@campus-hangouts",
  "DTSTART:20261121T000000Z",
  "DTEND:20261121T030000Z",
  "SUMMARY:Mock event",
  "LOCATION:Student Center",
  "END:VEVENT",
  "END:VCALENDAR",
].join("\r\n");

/**
 * Mock route table: [method, path template, mock file (null = 204/no body),
 * inline body (text responses that have no mock file)].
 *
 * Templates use `:param` segments; matching requires the same segment count,
 * so `/api/events/:id` never shadows `/api/events/:id/intent`.
 */
const MOCK_ROUTES = [
  ["GET", "/api/health", "health.json"],
  ["GET", "/api/demo-users", "demo-users.json"],
  ["POST", "/api/login", "me.json"],
  ["POST", "/api/logout", null],
  ["GET", "/api/me", "me.json"],
  ["GET", "/api/users", "users.json"],
  ["GET", "/api/events", "events.json"],
  ["POST", "/api/events", "event.json"],
  ["GET", "/api/events/:id/calendar.ics", null, MOCK_ICS],
  ["GET", "/api/events/:id", "event.json"],
  ["PUT", "/api/events/:id/intent", "intent.json"],
  ["DELETE", "/api/events/:id/intent", null],
  ["GET", "/api/feed", "feed.json"],
  ["GET", "/api/events/:id/buddy", "buddy.json"],
  ["POST", "/api/events/:id/buddy", "buddy.json"],
  ["GET", "/api/events/:id/qr", "qr.json"],
  ["POST", "/api/checkin", "checkin.json"],
  ["GET", "/api/tags", "tags.json"],
  ["GET", "/api/friends", "friends.json"],
  ["POST", "/api/friends", "friends.json"],
  ["POST", "/api/friends/:id/accept", "friends.json"],
  ["DELETE", "/api/friends/:id", "friends.json"],
  ["GET", "/api/crews", "crews.json"],
  ["POST", "/api/crews", "crew.json"],
  ["GET", "/api/crews/:id", "crew.json"],
  ["POST", "/api/crews/:id/members", "crew.json"],
  ["DELETE", "/api/crews/:id/members/:userId", "crew.json"],
  ["GET", "/api/crews/:id/availability", "crew-availability.json"],
  ["PUT", "/api/me/availability", "availability.json"],
  ["PUT", "/api/me/interests", "me.json"],
  ["POST", "/api/ask", "ask.json"],
  ["POST", "/api/parse", "parse.json"],
  ["GET", "/api/me/privacy", "privacy.json"],
  ["PUT", "/api/me/privacy", "privacy.json"],
  ["DELETE", "/api/me/history", null],
];

/** Matches a live path against the mock table; returns the route or null. */
function matchMock(method, path) {
  const segs = path.split("?")[0].split("/").filter(Boolean);
  for (const [m, template, file, inline] of MOCK_ROUTES) {
    if (m !== method) continue;
    const tsegs = template.split("/").filter(Boolean);
    if (tsegs.length !== segs.length) continue;
    if (tsegs.every((t, i) => t.startsWith(":") || t === segs[i])) {
      return { file, inline };
    }
  }
  return null;
}

/**
 * Performs one API request.
 *
 * @param {"GET"|"POST"|"PUT"|"DELETE"} method
 * @param {string} path e.g. `/api/events` (query string included).
 * @param {object} [opts]
 * @param {object} [opts.body] JSON request body (POST/PUT).
 * @param {"json"|"text"} [opts.as] response body type (default `json`).
 * @returns {Promise<any>} parsed body, or `null` for 204.
 * @throws {ApiHttpError} on non-2xx (message is safe to show) or a bad mock.
 */
export async function apiFetch(method, path, { body, as = "json" } = {}) {
  if (isMock()) {
    const route = matchMock(method, path);
    if (!route) {
      throw new ApiHttpError(404, "not_found", `No mock data for ${method} ${path}`);
    }
    if (route.inline) return route.inline;
    if (route.file === null) return null;
    const res = await fetch(new URL(`../mock/${route.file}`, import.meta.url));
    if (!res.ok) {
      throw new ApiHttpError(res.status, "internal", `Mock file ${route.file} is missing`);
    }
    return as === "text" ? await res.text() : await res.json();
  }

  const headers = {};
  let payload;
  if (body !== undefined) {
    headers["Content-Type"] = "application/json";
    payload = JSON.stringify(body);
  }
  let res;
  try {
    res = await fetch(path, {
      method,
      headers,
      body: payload,
      credentials: "same-origin",
    });
  } catch {
    throw new ApiHttpError(0, "network", "Can't reach the server. Check your connection and try again.");
  }
  if (res.status === 204) return null;
  const text = await res.text();
  let data = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    /* non-JSON body (shouldn't happen for /api); handled below */
  }
  if (!res.ok) {
    const err = data && data.error ? data.error : null;
    throw new ApiHttpError(
      res.status,
      err ? err.code : "internal",
      err ? err.message : `Request failed (HTTP ${res.status})`,
    );
  }
  if (as === "text") return text;
  if (data === null) {
    throw new ApiHttpError(res.status, "internal", "Empty response from API");
  }
  return data;
}

/** Small verb sugar over {@link apiFetch}. */
export const api = {
  /** GET a JSON endpoint. */
  get: (path, opts) => apiFetch("GET", path, opts),
  /** POST a JSON body. */
  post: (path, body, opts) => apiFetch("POST", path, { ...opts, body }),
  /** PUT a JSON body. */
  put: (path, body, opts) => apiFetch("PUT", path, { ...opts, body }),
  /** DELETE (no body). */
  del: (path, opts) => apiFetch("DELETE", path, opts),
};
