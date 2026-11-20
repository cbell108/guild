/**
 * Shared UI primitives: generated covers, avatars, toasts, bottom sheets,
 * view transitions. Everything dynamic goes through `esc`.
 */
import { esc, initials, dateBadge } from "./util.js";

/** True when the user asked the OS for reduced motion. */
export const reducedMotion = () =>
  window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Small deterministic string hash (FNV-1a), unsigned. */
export function hash(str) {
  let h = 2166136261;
  for (let i = 0; i < str.length; i += 1) {
    h ^= str.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

/* --- Generated covers -------------------------------------------------------------- */

/** [pattern, emoji, hue A, hue B] — first match on tags + title wins. */
const CATEGORIES = [
  [/smash|gam(e|ing)|esport|nintendo|video|league|valorant/, "🎮", 262, 315],
  [/board|card|tabletop|trivia|puzzle|chess/, "🎲", 24, 346],
  [/soccer|football/, "⚽", 142, 188],
  [/basket|volley|tennis|sport|fitness|gym|run|ping|table-tennis|intramural|yoga|climb/, "🏀", 14, 42],
  [/music|concert|band|jam|dance|karaoke|dj|choir|open-mic/, "🎵", 318, 266],
  [/food|coffee|pizza|dinner|lunch|cook|bake|taco|snack|boba/, "🍕", 22, 48],
  [/art|design|craft|pottery|paint|photo|film|movie|theat|draw/, "🎨", 338, 18],
  [/ai|lecture|tech|cs|code|coding|hack|data|robot|research|science|engineering/, "💡", 212, 256],
  [/career|business|network|intern|job|resume|entrepreneur/, "💼", 196, 168],
  [/study|library|academic|writing|exam|tutor/, "📚", 172, 212],
  [/outdoor|hike|nature|camp|garden|walk|lake/, "🌿", 146, 176],
  [/volunteer|service|community|charity|wellness|health/, "🤝", 160, 124],
  [/greek|social|party|mixer|celebrat|festival|game-night/, "🎉", 340, 292],
];

/** Picks a category (emoji + two hues) for an event, deterministically. */
function category(ev) {
  const hay = `${(ev.tags || []).join(" ")} ${ev.title || ""}`.toLowerCase();
  for (const [re, emoji, a, b] of CATEGORIES) {
    if (re.test(hay)) return { emoji, a, b };
  }
  return ev.kind === "official"
    ? { emoji: "🎓", a: 236, b: 276 }
    : { emoji: "✨", a: 266, b: 216 };
}

/**
 * Cover art for an event: its `image_url` over a generated fallback.
 * The fallback (category gradient + big emoji + subtle pattern) is derived
 * from the tags/title and the event id, so one event always looks the same.
 * `inner` is HTML layered above the art (badges, chips).
 */
export function cover(ev, inner = "") {
  const c = category(ev);
  const h = hash(String(ev.id || ev.title));
  const shift = (h % 21) - 10;
  const angle = 120 + (h % 5) * 18;
  const pattern = ["dots", "lines", "rings"][(h >> 3) % 3];
  const style =
    `--cv-a:hsl(${(c.a + shift + 360) % 360} 72% 52%);` +
    `--cv-b:hsl(${(c.b - shift + 360) % 360} 78% 38%);--cv-angle:${angle}deg;` +
    `--cv-x:${(h >> 5) % 2 ? 62 : 6}%;--cv-y:${8 + ((h >> 7) % 30)}%;--cv-rot:${((h >> 9) % 41) - 20}deg`;
  const img = ev.image_url
    ? `<img class="cover__img" src="${esc(ev.image_url)}" alt="" loading="lazy" onerror="this.remove()" />`
    : "";
  return `<div class="cover cover--${pattern}" style="${style}">
    <span class="cover__emoji" aria-hidden="true">${c.emoji}</span>${img}
    <span class="cover__scrim" aria-hidden="true"></span>${inner}</div>`;
}

/** The "SAT 21" badge shown on covers. */
export function dateBadgeHtml(iso) {
  const b = dateBadge(iso);
  return `<span class="date-badge"><span class="date-badge__dow">${esc(b.dow)}</span><span class="date-badge__day">${esc(b.day)}</span></span>`;
}

/* --- Avatars ------------------------------------------------------------------------- */

/** Round initials avatar, colour derived from the user id. */
export function avatar(user, cls = "") {
  const hue = hash(String(user.id || user.display_name)) % 360;
  return `<span class="avatar ${cls}" style="--av-hue:${hue}" aria-hidden="true">${esc(initials(user.display_name))}</span>`;
}

/** Overlapping avatars (max 3) plus an optional "+N" bubble. */
export function avatarStack(users, extra = 0) {
  const shown = users.slice(0, 3).map((u) => avatar(u)).join("");
  const more = extra > 0 ? `<span class="avatar avatar--more" aria-hidden="true">+${extra}</span>` : "";
  return `<span class="avatar-stack">${shown}${more}</span>`;
}

/* --- Toast ----------------------------------------------------------------------------- */

/** Shows a short message at the bottom of the screen (announced to screen readers). */
export function toast(message) {
  const region = document.getElementById("toasts");
  if (!region) return;
  const el = document.createElement("div");
  el.className = "toast";
  el.textContent = message;
  region.appendChild(el);
  setTimeout(() => {
    el.classList.add("toast--out");
    setTimeout(() => el.remove(), 250);
  }, 2600);
}

/* --- Bottom sheet ------------------------------------------------------------------------- */

/**
 * Opens the shared bottom sheet. `content` is an HTML string; `onMount`
 * receives the sheet body to attach listeners. Returns `{ close }`.
 */
export function openSheet(title, content, onMount) {
  const dlg = document.getElementById("sheet");
  if (!dlg) return { close() {} };
  dlg.querySelector("[data-sheet-title]").textContent = title;
  const body = dlg.querySelector("[data-sheet-body]");
  body.innerHTML = content;
  const close = () => {
    if (dlg.open) dlg.close();
  };
  if (onMount) onMount(body, close);
  if (!dlg.open) dlg.showModal();
  return { close };
}

/** Wires the sheet's close button and backdrop click (call once at boot). */
export function initSheet() {
  const dlg = document.getElementById("sheet");
  if (!dlg) return;
  dlg.querySelector("[data-sheet-close]").addEventListener("click", () => dlg.close());
  dlg.addEventListener("click", (e) => {
    if (e.target === dlg) dlg.close();
  });
}

/* --- Page transitions ------------------------------------------------------------------------- */

/** Runs `update` inside a View Transition when supported (and motion is allowed). */
export async function transition(update) {
  if (document.startViewTransition && !reducedMotion() && document.visibilityState === "visible") {
    // Name the page layer only for the duration of the transition, so the bars stay put.
    const view = document.getElementById("view");
    view.style.viewTransitionName = "page";
    const vt = document.startViewTransition(update);
    const clear = () => {
      view.style.viewTransitionName = "";
    };
    vt.finished.then(clear, clear);
    // A hidden tab aborts the animation; the update itself still runs.
    vt.ready.catch(() => {});
    vt.finished.catch(() => {});
    await vt.updateCallbackDone;
  } else {
    await update();
  }
}

/** Skeleton placeholder markup: `rails` carousel skeletons or `rows` list skeletons. */
export function skeleton(kind = "rails") {
  if (kind === "rows") {
    return `<div class="skel-list" aria-hidden="true">${'<div class="skel skel-row"></div>'.repeat(5)}</div>`;
  }
  return `<div aria-hidden="true"><div class="skel skel-title"></div><div class="skel-rail">
    ${'<div class="skel skel-card"></div>'.repeat(3)}</div>
    <div class="skel skel-title"></div><div class="skel-rail">${'<div class="skel skel-card skel-card--sm"></div>'.repeat(3)}</div></div>`;
}
