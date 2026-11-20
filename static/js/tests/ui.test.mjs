/**
 * Unit tests for the pure (import-safe) helpers in `static/js/ui.js`:
 * hash, cover, dateBadgeHtml, avatar, avatarStack, skeleton, stateHtml,
 * errorHtml. The DOM-bound exports (toast, openSheet, initSheet, transition,
 * reducedMotion) are not tested here.
 */
import test from "node:test";
import assert from "node:assert/strict";

import {
  hash,
  cover,
  dateBadgeHtml,
  avatar,
  avatarStack,
  skeleton,
  stateHtml,
  errorHtml,
} from "../ui.js";

test("hash: deterministic, unsigned 32-bit", () => {
  const h = hash("campus-hangouts");
  assert.equal(h, hash("campus-hangouts"));
  assert.notEqual(h, hash("campus-hangout"));
  assert.notEqual(h, hash(""));
  for (const s of ["a", "b", "campus-hangouts", "🎮", ""]) {
    const v = hash(s);
    assert.ok(Number.isInteger(v), `${s} → integer`);
    assert.ok(v >= 0 && v <= 0xffffffff, `${s} → unsigned 32-bit`);
  }
});

test("cover: deterministic category gradient + emoji fallback", () => {
  const ev = { id: "e1", title: "Smash Bros Night", tags: ["gaming"] };
  const once = cover(ev);
  assert.ok(once.startsWith('<div class="cover cover--'));
  assert.match(once, /cover--(dots|lines|rings)"/);
  assert.ok(once.includes("🎮"), "gaming category emoji");
  assert.ok(once.includes('--cv-a:hsl(') && once.includes("--cv-angle:"), "style vars present");
  assert.equal(cover({ ...ev }), once, "same event renders identically");
});

test("cover: image_url is inserted and escaped", () => {
  const html = cover({ id: "e2", title: "T", image_url: "pics/a.png?x=1&y=2" });
  assert.ok(html.includes('src="pics/a.png?x=1&amp;y=2"'), "image url escaped");
  assert.ok(html.includes('class="cover__img"'));
});

test("dateBadgeHtml: SAT 21 badge markup", () => {
  assert.equal(
    dateBadgeHtml("2026-11-21T12:00:00Z"),
    '<span class="date-badge"><span class="date-badge__dow">SAT</span>' +
      '<span class="date-badge__day">21</span></span>',
  );
});

test("avatar: initials with deterministic hue", () => {
  const user = { id: "u1", display_name: "Priya S." };
  const html = avatar(user);
  assert.match(html, /^<span class="avatar " style="--av-hue:\d{1,3}" aria-hidden="true">PS<\/span>$/);
  assert.equal(avatar({ ...user }), html);
  assert.ok(avatar(user, "", true).includes(">P</span>"), "single: first initial only");
  assert.ok(avatar(user, "avatar--sm").includes('class="avatar avatar--sm"'));
});

test("avatarStack: at most 3 avatars plus a +N bubble", () => {
  const users = [
    { id: "u1", display_name: "Ada One" },
    { id: "u2", display_name: "Ben Two" },
    { id: "u3", display_name: "Cara Three" },
    { id: "u4", display_name: "Dee Four" },
  ];
  const html = avatarStack(users, 2);
  assert.ok(html.startsWith('<span class="avatar-stack">'));
  for (const letter of ["A", "B", "C"]) assert.ok(html.includes(`>${letter}</span>`), `initial ${letter}`);
  assert.ok(!html.includes(">D</span>"), "fourth user is not rendered");
  assert.ok(html.includes('class="avatar avatar--more"'), "+N bubble present");
  assert.ok(html.includes("+2</span>"));
  const noExtra = avatarStack(users.slice(0, 1), 0);
  assert.ok(!noExtra.includes("avatar--more"), "no bubble without extra");
});

test("skeleton: rows and rails variants", () => {
  const rows = skeleton("rows");
  assert.match(rows, /^<div class="skel-list" aria-hidden="true">/);
  assert.equal(rows.split("skel-row").length - 1, 5, "five row placeholders");
  const rails = skeleton();
  assert.equal((rails.match(/"skel skel-card"/g) || []).length, 3, "three big cards");
  assert.equal(rails.split("skel-card--sm").length - 1, 3, "three small cards");
});

test("stateHtml: escapes title/text/action, omits empty pieces", () => {
  const html = stateHtml({ emoji: "📭", title: "No <hangouts>", text: "Try <filters>", action: "Browse" });
  assert.ok(html.includes('<h2>No &lt;hangouts&gt;</h2>'));
  assert.ok(html.includes("<p>Try &lt;filters&gt;</p>"));
  assert.ok(html.includes('<button type="button" class="btn btn--primary" data-action>Browse</button>'));
  const minimal = stateHtml({ emoji: "📭", title: "Empty" });
  assert.ok(!minimal.includes("<p>"), "no <p> without text");
  assert.ok(!minimal.includes("data-action"), "no button without action");
});

test("errorHtml: offline, 404 and generic variants", () => {
  const offline = errorHtml({ status: 0, message: "No connection <x>" });
  assert.ok(offline.includes("📡") && /You(&#39;|&#x27;|')re offline/.test(offline));
  assert.ok(offline.includes("No connection &lt;x&gt;"), "message escaped");

  const missing = errorHtml({ status: 404, message: "Gone" });
  assert.ok(missing.includes("🧭") && missing.includes("Not found"));

  const generic = errorHtml({ status: 500 });
  assert.ok(generic.includes("😕") && generic.includes("Something went wrong"));
  assert.ok(generic.includes("Please try again."), "falls back when no message");

  for (const html of [offline, missing, generic]) {
    assert.ok(html.includes("Try again"), "default action button");
  }
  assert.ok(errorHtml({ status: 0 }, "Reset").includes(">Reset</button>"));
});
