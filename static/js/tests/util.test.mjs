/**
 * Unit tests for the pure helpers in `static/js/util.js`.
 *
 * Run with: `node --test static/js/tests/`
 *
 * Time formatting always runs in America/New_York regardless of the host
 * time zone, so the assertions below hold in any CI environment.
 */
import test from "node:test";
import assert from "node:assert/strict";

import * as u from "../util.js";

const DATE_RE = /^\d{4}-\d{2}-\d{2}$/;
const LABEL_RE = /^(Today|Tomorrow|[A-Z][a-z]{2}, [A-Z][a-z]{2} \d{1,2})$/;

test("esc escapes all five HTML special characters", () => {
  assert.equal(u.esc(`<script>"&'`), "&lt;script&gt;&quot;&amp;&#39;");
  assert.equal(u.esc("plain"), "plain");
});

test("initials", () => {
  assert.equal(u.initials("Casey (demo)"), "C");
  assert.equal(u.initials("Jordan H."), "JH");
  assert.equal(u.initials("Maya R."), "MR");
  assert.equal(u.initials(""), "?");
  assert.equal(u.initials("  priya  s. "), "PS");
});

test("TZ is the campus zone", () => {
  assert.equal(u.TZ, "America/New_York");
});

test("localDate converts UTC to the campus calendar date", () => {
  // 2026-11-01 is the US DST fall-back day: the offset flips from EDT (−4)
  // to EST (−5) at 06:00 UTC. Both instants below are 1:30 AM local.
  assert.equal(u.localDate("2026-11-01T05:30:00Z"), "2026-11-01"); // 1:30 AM EDT
  assert.equal(u.localDate("2026-11-01T06:30:00Z"), "2026-11-01"); // 1:30 AM EST
  // Before local midnight (04:00Z = 00:00 EDT) it is still the prior day.
  assert.equal(u.localDate("2026-11-01T03:30:00Z"), "2026-10-31"); // 11:30 PM EDT
  // Overnight event: 1:30 AM Saturday.
  assert.equal(u.localDate("2026-11-02T05:30:00Z"), "2026-11-02");
});

test("localHM converts UTC to campus HH:MM (24 h)", () => {
  // Same wall clock (1:30) across the DST transition, even though the UTC
  // instants are an hour apart.
  assert.equal(u.localHM("2026-11-01T05:30:00Z"), "01:30"); // EDT
  assert.equal(u.localHM("2026-11-01T06:30:00Z"), "01:30"); // EST
  assert.equal(u.localHM("2026-11-21T23:00:00Z"), "18:00");
  assert.equal(u.localHM("2026-11-02T05:30:00Z"), "00:30"); // EST after the switch
});

test("fmtClock formats the campus clock, dropping :00 minutes", () => {
  assert.equal(u.fmtClock("2026-11-01T05:30:00Z"), "1:30 AM"); // 1:30 AM EDT
  assert.equal(u.fmtClock("2026-11-01T06:30:00Z"), "1:30 AM"); // 1:30 AM EST
  assert.equal(u.fmtClock("2026-11-21T23:00:00Z"), "6 PM"); // :00 drops the minutes
  assert.equal(u.fmtClock("2026-11-21T23:30:00Z"), "6:30 PM");
  assert.equal(u.fmtClock("2026-11-21T05:30:00Z"), "12:30 AM"); // midnight hour
});

test("fmtEventTime: window with start, end and no end", () => {
  // The day label is relative to the real clock (Today/Tomorrow/"Dow, Mon D"),
  // so assert the label shape and the exact time portion.
  const start = "2026-11-21T16:00:00Z"; // Sat 11:00 AM
  const end = "2026-11-21T21:00:00Z"; // Sat 4:00 PM
  const out = u.fmtEventTime(start, end);
  assert.match(out, /^.+ · 11 AM – 4 PM$/);
  assert.ok(!out.includes("next day"), "same-day window must not say next day");

  // No end time: start clock only.
  assert.match(u.fmtEventTime(start, null), /^.+ · 11 AM$/);

  // Same instant: a single clock, no en dash range.
  assert.match(u.fmtEventTime(start, start), /^.+ · 11 AM$/);
});

test("fmtEventTime: overnight event ending after midnight", () => {
  // Fri 6:30 PM EST → Sat 12:30 AM EST: crosses the campus midnight.
  const start = "2026-11-20T23:30:00Z";
  const end = "2026-11-21T05:30:00Z";
  const out = u.fmtEventTime(start, end);
  assert.match(out, /^.+ · 6:30 PM – 12:30 AM · next day$/);
});

test("dayLabel: Today/Tomorrow/short date shape", () => {
  // dayLabel reads the real clock; assert the documented shape, not a value.
  for (const iso of ["2026-11-21T12:00:00Z", "2026-01-01T12:00:00Z"]) {
    assert.match(u.dayLabel(iso), LABEL_RE);
  }
});

test("todayLocal: campus-local YYYY-MM-DD of now", () => {
  assert.match(u.todayLocal(), DATE_RE);
});

test("dayOffset: YYYY-MM-DD n days from today", () => {
  assert.match(u.dayOffset(0), DATE_RE);
  assert.equal(u.dayOffset(0), u.todayLocal(), "dayOffset(0) is today");
  assert.match(u.dayOffset(1), DATE_RE);
  assert.notEqual(u.dayOffset(1), u.dayOffset(0));
});

test("weekendDays: one or two YYYY-MM-DD dates, Saturday before Sunday", () => {
  const days = u.weekendDays();
  assert.ok(Array.isArray(days) && (days.length === 1 || days.length === 2));
  for (const d of days) assert.match(d, DATE_RE);
  if (days.length === 2) assert.ok(days[0] < days[1]);
});

test("localWeekday: 0 = Sunday … 6 = Saturday", () => {
  assert.equal(u.localWeekday("2026-11-01T12:00:00Z"), 0); // Sunday
  assert.equal(u.localWeekday("2026-11-20T12:00:00Z"), 5); // Friday
  assert.equal(u.localWeekday("2026-11-21T12:00:00Z"), 6); // Saturday
});

test("dateBadge: short weekday (upper) and numeric day in campus time", () => {
  assert.deepEqual(u.dateBadge("2026-11-21T12:00:00Z"), { dow: "SAT", day: "21" });
  assert.deepEqual(u.dateBadge("2026-11-01T12:00:00Z"), { dow: "SUN", day: "1" });
});

test("dayHeading: long date with optional Today/Tomorrow prefix", () => {
  // dayHeading embeds dayLabel (real clock); assert the documented shapes:
  // "Saturday, November 21" or "Today · …" / "Tomorrow · …"
  const out = u.dayHeading("2026-11-21T12:00:00Z");
  assert.match(
    out,
    /^(Today|Tomorrow) · [A-Z][a-z]+, [A-Z][a-z]+ \d{1,2}$|^[A-Z][a-z]+, [A-Z][a-z]+ \d{1,2}$/,
  );
});
