/**
 * Shared helpers (no dependencies).
 *
 * Per docs/API.md: the API speaks RFC 3339 UTC; the frontend formats in
 * `America/New_York` with `Intl.DateTimeFormat(…, {timeZone: "America/New_York"})`.
 */

/** Campus time zone, the only zone the UI ever displays. */
export const TZ = "America/New_York";

/** Escapes a value for safe interpolation into `innerHTML`. */
export function esc(value) {
  return String(value).replace(/[&<>"']/g, (c) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  })[c]);
}

/** "Priya S." → "PS"; "Casey (demo)" → "C" (words not starting with a letter/digit are skipped). */
export function initials(name) {
  const words = String(name || "")
    .trim()
    .split(/\s+/)
    .filter((w) => /^[\p{L}\p{N}]/u.test(w))
    .slice(0, 2);
  return words.map((w) => w[0]).join("").toUpperCase() || "?";
}

/* --- Date/time formatting ------------------------------------------------------ */

const DATE_KEYS = { timeZone: TZ, year: "numeric", month: "2-digit", day: "2-digit" };

/** Local (America/New_York) calendar date of an instant, `YYYY-MM-DD`. */
function dayKey(date) {
  const parts = new Intl.DateTimeFormat("en-CA", DATE_KEYS).formatToParts(date);
  const get = (type) => parts.find((p) => p.type === type)?.value || "";
  return `${get("year")}-${get("month")}-${get("day")}`;
}

/**
 * Calendar date (in TZ) of `baseIso` shifted by `delta` days.
 * Shifts the local date components and re-resolves at UTC noon, which is safe
 * for America/New_York on every DST transition.
 */
function shiftedDayKey(baseIso, delta) {
  const parts = new Intl.DateTimeFormat("en-US", DATE_KEYS).formatToParts(new Date(baseIso));
  const get = (type) => Number(parts.find((p) => p.type === type)?.value || 0);
  const utc = Date.UTC(get("year"), get("month") - 1, get("day") + delta, 12);
  return dayKey(new Date(utc));
}

/** "Today", "Tomorrow", or "Fri, Nov 21" (in TZ). */
export function dayLabel(iso) {
  const key = dayKey(new Date(iso));
  const now = new Date();
  if (key === dayKey(now)) return "Today";
  if (key === shiftedDayKey(isoOf(now), 1)) return "Tomorrow";
  return new Intl.DateTimeFormat("en-US", {
    timeZone: TZ,
    weekday: "short",
    month: "short",
    day: "numeric",
  }).format(new Date(iso));
}

/** "7:00 PM" → "7 PM"; "7:30 PM" stays "7:30 PM". */
export function fmtClock(iso) {
  const parts = new Intl.DateTimeFormat("en-US", {
    timeZone: TZ,
    hour: "numeric",
    minute: "2-digit",
  }).formatToParts(new Date(iso));
  const get = (type) => parts.find((p) => p.type === type)?.value || "";
  return get("minute") === "00" ? `${get("hour")} ${get("dayPeriod")}` : `${get("hour")}:${get("minute")} ${get("dayPeriod")}`;
}

/**
 * One human line for an event window: "Today · 7 – 10 PM",
 * "Fri, Nov 21 · 10 AM – 2 PM", "Sat, Nov 21 · 11 PM – 1 AM · next day".
 * `endsIso` may be null (renders the start clock only).
 */
export function fmtEventTime(startsIso, endsIso) {
  const label = dayLabel(startsIso);
  const t1 = fmtClock(startsIso);
  if (!endsIso) return `${label} · ${t1}`;
  const t2 = fmtClock(endsIso);
  const sameInstant = new Date(startsIso).getTime() === new Date(endsIso).getTime();
  let time = sameInstant ? t1 : `${t1} – ${t2}`;
  if (!sameInstant && dayKey(new Date(startsIso)) !== dayKey(new Date(endsIso))) {
    time += " · next day";
  }
  return `${label} · ${time}`;
}

function isoOf(date) {
  return date.toISOString();
}

/** Campus-local `YYYY-MM-DD` of an RFC 3339 instant (zero-padded). */
export function localDate(iso) {
  return dayKey(new Date(iso));
}

/** Campus-local `HH:MM` (24 h) of an RFC 3339 instant. */
export function localHM(iso) {
  const parts = new Intl.DateTimeFormat("en-GB", {
    timeZone: TZ,
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).formatToParts(new Date(iso));
  const get = (type) => parts.find((p) => p.type === type)?.value || "00";
  return `${get("hour")}:${get("minute")}`;
}

/** Today's campus-local date, `YYYY-MM-DD`. */
export function todayLocal() {
  return dayKey(new Date());
}
