/**
 * "Browse" view: a sticky pill search bar and one row of filter chips (date |
 * kind | top tags); results are compact rows grouped by day with sticky day
 * headers and a "Show more" button. Filters go to `/api/events` where the API
 * supports them (kind, tags, q); date chips filter client-side.
 */
import { api } from "../api.js";
import { esc, localDate, dayOffset, weekendDays, dayHeading } from "../util.js";
import { eventRow } from "../components.js";
import { skeleton } from "../ui.js";

const PAGE = 20;

const DATE_CHIPS = [
  ["today", "Today"],
  ["tomorrow", "Tomorrow"],
  ["weekend", "This weekend"],
  ["week", "Next 7 days"],
];

const KIND_CHIPS = [
  ["", "All"],
  ["official", "Official"],
  ["hangout", "Hangouts"],
];

/** Does an event fall inside the chosen date chip? */
function inRange(ev, range) {
  if (!range) return true;
  const day = localDate(ev.starts_at);
  if (range === "today") return day === dayOffset(0);
  if (range === "tomorrow") return day === dayOffset(1);
  if (range === "weekend") return weekendDays().includes(day);
  return day >= dayOffset(0) && day <= dayOffset(6);
}

/** Renders the "Browse" view. */
export default {
  name: "browse",
  async render(root) {
    const f = { q: "", range: "", kind: "", tags: [] };
    let shown = PAGE;
    let tagList = [];
    try {
      tagList = (await api.get("/api/tags")).slice(0, 12);
    } catch {
      /* tags are optional */
    }

    const chip = (attr, value, label, on) =>
      `<button type="button" class="fchip${on ? " is-on" : ""}" data-${attr}="${esc(value)}" aria-pressed="${on}">${esc(label)}</button>`;

    root.innerHTML = `
      <header class="view-head"><h1>Browse</h1></header>
      <div class="browse-bar">
        <label class="search"><svg class="icon" aria-hidden="true"><use href="#i-search" /></svg>
          <span class="visually-hidden">Search events</span>
          <input type="search" id="f-q" placeholder="Search events and hangouts" autocomplete="off" /></label>
        <div class="fchips" role="group" aria-label="Filters" id="chips"></div>
      </div>
      <p id="b-count" class="muted" role="status" aria-live="polite"></p>
      <div id="b-list" class="b-list"></div>`;

    const chips = root.querySelector("#chips");
    const list = root.querySelector("#b-list");
    const count = root.querySelector("#b-count");

    const paintChips = () => {
      chips.innerHTML =
        DATE_CHIPS.map(([v, l]) => chip("range", v, l, f.range === v)).join("") +
        `<span class="fchips__sep" aria-hidden="true"></span>` +
        KIND_CHIPS.map(([v, l]) => chip("kind", v, l, f.kind === v)).join("") +
        `<span class="fchips__sep" aria-hidden="true"></span>` +
        tagList.map((t) => chip("tag", t.tag, `#${t.tag}`, f.tags.includes(t.tag))).join("");
    };

    let all = [];
    let seq = 0;
    const paintList = () => {
      const rows = all.filter((e) => inRange(e, f.range));
      count.textContent = rows.length ? `${rows.length} event${rows.length === 1 ? "" : "s"}` : "";
      if (!rows.length) {
        list.innerHTML = `<div class="empty"><span class="empty__emoji" aria-hidden="true">🔍</span><h2>No matches</h2><p>Try a different day or clear a filter.</p></div>`;
        return;
      }
      const visible = rows.slice(0, shown);
      let html = "";
      let day = "";
      for (const e of visible) {
        const d = localDate(e.starts_at);
        if (d !== day) {
          if (day) html += "</section>";
          day = d;
          html += `<section class="day"><h2 class="day__head">${esc(dayHeading(e.starts_at))}</h2>`;
        }
        html += eventRow(e);
      }
      html += "</section>";
      if (rows.length > shown) {
        html += `<button type="button" class="btn btn--ghost btn--block" id="more">Show more (${rows.length - shown})</button>`;
      }
      list.innerHTML = html;
      list.querySelector("#more")?.addEventListener("click", () => {
        shown += PAGE;
        paintList();
      });
    };

    const load = async () => {
      const mine = ++seq;
      const qs = new URLSearchParams({ limit: "200" });
      if (f.kind) qs.set("kind", f.kind);
      if (f.tags.length) qs.set("tags", f.tags.join(","));
      if (f.q.trim()) qs.set("q", f.q.trim());
      list.innerHTML = skeleton("rows");
      try {
        const events = await api.get(`/api/events?${qs}`);
        if (mine !== seq) return;
        // Defensive client-side re-check (mock mode ignores query params).
        all = events.filter(
          (e) =>
            (!f.kind || e.kind === f.kind) &&
            (!f.tags.length || f.tags.some((t) => (e.tags || []).includes(t))) &&
            (!f.q.trim() || `${e.title} ${(e.tags || []).join(" ")}`.toLowerCase().includes(f.q.trim().toLowerCase())),
        );
        shown = PAGE;
        paintList();
      } catch (err) {
        if (mine === seq) list.innerHTML = `<p class="form-error" role="alert">${esc(err.message || "Could not load events.")}</p>`;
      }
    };

    chips.addEventListener("click", (e) => {
      const b = e.target.closest("button");
      if (!b) return;
      if (b.dataset.range !== undefined) {
        f.range = f.range === b.dataset.range ? "" : b.dataset.range;
        paintChips();
        shown = PAGE;
        paintList();
        return;
      }
      if (b.dataset.kind !== undefined) f.kind = b.dataset.kind;
      if (b.dataset.tag !== undefined) {
        f.tags = f.tags.includes(b.dataset.tag) ? f.tags.filter((t) => t !== b.dataset.tag) : [...f.tags, b.dataset.tag];
      }
      paintChips();
      load();
    });

    let timer;
    root.querySelector("#f-q").addEventListener("input", (e) => {
      f.q = e.target.value;
      clearTimeout(timer);
      timer = setTimeout(load, 250);
    });

    paintChips();
    await load();
  },
};
