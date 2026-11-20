/**
 * Unit tests for the pure HTML-string builders in `static/js/components.js`:
 * friendLine, intentLabel, kindChip, placeLine, spotsText, whenPlace,
 * socialLine, eventCard, eventRow, whyHtml.
 */
import test from "node:test";
import assert from "node:assert/strict";

import {
  friendLine,
  intentLabel,
  kindChip,
  placeLine,
  spotsText,
  whenPlace,
  socialLine,
  eventCard,
  eventRow,
  whyHtml,
} from "../components.js";

const user = (id, name) => ({ id, display_name: name });

test("friendLine: going variants", () => {
  assert.equal(friendLine(null), "");
  assert.equal(friendLine({}), "");
  assert.equal(friendLine({ going: 1, interested: 0, sample: [] }), "1 friend going");
  assert.equal(
    friendLine({ going: 1, interested: 0, sample: [user("u1", "Jordan H.")] }),
    "Jordan is going",
  );
  assert.equal(
    friendLine({ going: 3, interested: 0, sample: [user("u1", "Jordan H.")] }),
    "Jordan + 2 going",
  );
  assert.equal(
    friendLine({ going: 3, interested: 0, sample: [] }),
    "3 friends going",
  );
});

test("friendLine: interested variants and empty", () => {
  assert.equal(friendLine({ going: 0, interested: 0 }), "");
  assert.equal(friendLine({ going: 0, interested: 1, sample: [] }), "1 friend interested");
  assert.equal(
    friendLine({ going: 0, interested: 1, sample: [user("u1", "Maya R.")] }),
    "Maya is interested",
  );
  assert.equal(friendLine({ going: 0, interested: 2, sample: [] }), "2 friends interested");
  // going beats interested
  assert.equal(friendLine({ going: 2, interested: 5, sample: [] }), "2 friends going");
});

test("intentLabel", () => {
  assert.equal(intentLabel(null), "");
  assert.equal(intentLabel({ status: "waitlisted" }), "Waitlisted");
  assert.equal(intentLabel({ status: "waitlisted", waitlist_position: 3 }), "Waitlisted · #3");
  assert.equal(intentLabel({ level: "going" }), "Going");
  assert.equal(intentLabel({ level: "going_with" }), "Going");
  assert.equal(
    intentLabel({ level: "going_with", with: user("u2", "Maya R.") }),
    "Going with Maya",
  );
  assert.equal(intentLabel({ level: "interested" }), "Interested");
});

test("kindChip: official and hangout, kind is escaped in the class", () => {
  assert.equal(
    kindChip("official"),
    '<span class="kind-chip kind-chip--official">Official</span>',
  );
  assert.equal(
    kindChip("hangout"),
    '<span class="kind-chip kind-chip--hangout">Hangout</span>',
  );
  assert.equal(kindChip("<x>"), '<span class="kind-chip kind-chip--&lt;x&gt;">Hangout</span>');
});

test("placeLine: full, partial and escaping", () => {
  assert.equal(placeLine(null), "");
  assert.equal(placeLine({ name: "Student Center" }), "Student Center");
  assert.equal(placeLine({ room: "Game Room 274" }), "Game Room 274");
  assert.equal(placeLine({ name: "Student Center", room: "Game Room 274" }), "Student Center · Game Room 274");
  assert.equal(placeLine({ name: "<a>", room: "b&c" }), "&lt;a&gt; · b&amp;c");
});

test("spotsText: open, full and limited", () => {
  assert.equal(spotsText({ capacity: null }), "");
  assert.equal(spotsText({}), "");
  assert.equal(spotsText({ capacity: 8, spots_left: 0 }), "Full · waitlist open");
  assert.equal(spotsText({ capacity: 8, spots_left: 2 }), "2 of 8 spots left");
  assert.equal(spotsText({ capacity: 1, spots_left: 1 }), "1 of 1 spots left");
});

test("whenPlace: clock plus venue name", () => {
  assert.equal(
    whenPlace({ starts_at: "2026-11-21T23:00:00Z", location: { name: "Student Center" } }),
    "6 PM · Student Center",
  );
  assert.equal(whenPlace({ starts_at: "2026-11-21T23:00:00Z", location: null }), "6 PM · ");
});

test("socialLine: empty without friends, avatars + text with friends", () => {
  assert.equal(socialLine({ friends: null }), "");
  assert.equal(socialLine({ friends: { going: 0, interested: 0, sample: [] } }), "");

  const ev = {
    friends: {
      going: 3,
      interested: 0,
      sample: [user("u1", "Priya S."), user("u2", "Ben T."), user("u3", "Cara T.")],
    },
  };
  const html = socialLine(ev);
  assert.ok(html.startsWith('<p class="social">'), "wraps in <p class=\"social\">");
  assert.ok(html.includes("Priya + 2 going"), "friend text");
  assert.ok(html.includes("avatar-stack"), "avatar stack");

  // 5 going, 5 samples: 3 avatars shown plus a "+2" bubble.
  const many = {
    friends: {
      going: 5,
      interested: 0,
      sample: [1, 2, 3, 4, 5].map((n) => user(`u${n}`, `Name ${n}`)),
    },
  };
  const lots = socialLine(many);
  assert.ok(lots.includes("avatar--more"), "extra bubble present");
  assert.ok(lots.includes("+2</span>"));
});

const baseEvent = {
  id: "e1",
  title: "Smash Bros <Night>",
  kind: "hangout",
  starts_at: "2026-11-21T16:00:00Z", // Sat 11:00 AM EST
  location: { name: "Student Center" },
  tags: ["gaming"],
  friends: null,
  my_intent: null,
  capacity: null,
};

test("eventCard (card variant): structure, escaping, meta", () => {
  const html = eventCard({ ...baseEvent });
  assert.ok(html.startsWith('<article class="ecard ecard--card" data-id="e1">'), "article tag");
  assert.ok(html.includes("Smash Bros &lt;Night&gt;"), "title escaped");
  assert.ok(html.includes('<h3 class="ecard__title">'), "card variant: title in body");
  assert.ok(!html.includes("ecard__title--over"), "no hero title in card variant");
  assert.ok(html.includes("cover__emoji"), "generated cover present");
  assert.ok(html.includes("11 AM · Student Center"), "when/place meta");
  assert.ok(html.includes("kind-chip--hangout"), "kind chip");
  assert.ok(!html.includes("reason"), "no reason without reasons");
});

test("eventCard (hero variant): title over the cover", () => {
  const html = eventCard({ ...baseEvent }, "hero");
  assert.ok(html.startsWith('<article class="ecard ecard--hero"'), "hero article");
  assert.ok(html.includes("ecard__title--over"), "hero title inside cover");
  assert.ok(!html.includes('<h3 class="ecard__title">'), "no body title in hero");
});

test("eventCard with feed item: reason line and data-why button", () => {
  const item = { event: { ...baseEvent }, reasons: [{ kind: "content", text: "You like <gaming>" }] };
  const html = eventCard(item);
  assert.ok(html.includes('data-why="e1"'), "why button carries the event id");
  assert.ok(html.includes("You like &lt;gaming&gt;"), "reason text escaped");
  assert.ok(html.includes('class="reason"'), "reason line present");
});

test("eventCard spots pill: shown at low spots, absent otherwise", () => {
  const low = eventCard({ ...baseEvent, capacity: 5, spots_left: 2 });
  assert.ok(low.includes('<span class="pill pill--warn">2 spots left</span>'), "low-spots pill");
  const full = eventCard({ ...baseEvent, capacity: 5, spots_left: 0 });
  assert.ok(full.includes('<span class="pill pill--warn">Full · waitlist</span>'), "full pill");
  const comfy = eventCard({ ...baseEvent, capacity: 50, spots_left: 40 });
  assert.ok(!comfy.includes("pill--warn"), "no pill with plenty of spots");
});

test("eventRow: compact row with chip, friends and intent pill", () => {
  const ev = {
    ...baseEvent,
    my_intent: { level: "going" },
    friends: { going: 1, interested: 0, sample: [user("u2", "Maya R.")] },
    capacity: 4,
    spots_left: 0,
  };
  const html = eventRow(ev);
  assert.ok(html.startsWith('<a class="erow" href="#/event/e1">'), "row anchor");
  assert.ok(html.includes("Smash Bros &lt;Night&gt;"), "title escaped");
  assert.ok(html.includes("11 AM · Student Center"), "meta line");
  assert.ok(html.includes('class="kind-chip kind-chip--hangout"'), "kind chip");
  assert.ok(html.includes('<span class="erow__friends">Maya is going</span>'), "friends line");
  assert.ok(html.includes('<span class="pill">Going</span>'), "intent pill");
  assert.ok(html.includes('<span class="pill pill--warn">Full · waitlist</span>'), "full pill");

  const bare = eventRow({ ...baseEvent });
  assert.ok(!bare.includes("erow__friends"), "no friends span without friends");
  assert.ok(!bare.includes('class="pill"'), "no intent pill without intent");
});

test("whyHtml: reasons list plus score breakdown bars", () => {
  const html = whyHtml({
    reasons: [
      { kind: "content", text: "You like <gaming>" },
      { kind: "social", text: "Maya is going" },
    ],
    breakdown: { content: 0.6, collaborative: 0, social: 0.2, popular: 0 },
  });
  assert.ok(html.startsWith('<ul class="why__list">'), "reasons list");
  assert.ok(html.includes('<li><span class="why__kind">content</span> You like &lt;gaming&gt;</li>'));
  assert.ok(html.includes('<li><span class="why__kind">social</span> Maya is going</li>'));
  assert.ok(html.includes('<div class="why__bars">'), "breakdown bars");
  assert.ok(html.includes('why__fill" style="width:100%"'), "capped at 100%");
  assert.ok(html.includes('why__fill" style="width:50%"'), "social bar at 50%");
  assert.ok(!html.includes("collaborative</span>"), "zero bars omitted");
  assert.ok(!html.includes("popular</span>"), "zero bars omitted");
});

test("whyHtml: no breakdown, no reasons", () => {
  const html = whyHtml({ reasons: [] });
  assert.equal(html, '<ul class="why__list"></ul>');
  assert.ok(!html.includes("why__bars"), "no bars without breakdown");
});
