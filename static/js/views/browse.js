/**
 * "Browse" view: a sticky pill search bar and one row of filter chips (date |
 * kind | top tags); results are compact rows grouped by day with sticky day
 * headers and a "Show more" button. Filters go to `/api/events` where the API
 * supports them (kind, tags, q); date chips filter client-side.
 */
import { api } from "../api.js";
import { esc, localDate, dayOffset, weekendDays, dayHeading } from "../util.js";
import { eventRow } from "../components.js";
import { skeleton, stateHtml, errorHtml, toast } from "../ui.js";

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
  async render(root, { state }) {
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
      ${
        state.me
          ? `<form class="ask" id="ask" role="search"><label class="visually-hidden" for="ask-q">Ask for something</label>
              <input id="ask-q" type="text" maxlength="300" placeholder="Ask for something… “chill tonight with food”" autocomplete="off" />
              <button type="submit" class="ask__go" aria-label="Find it"><span aria-hidden="true">✨</span></button></form>
             <p id="ask-pill" class="ask-pill" role="status" aria-live="polite" hidden></p>`
          : ""
      }
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
        const filtered = f.q || f.range || f.kind || f.tags.length;
        list.innerHTML = stateHtml({
          emoji: "🔍",
          title: filtered ? "No matches" : "No events yet",
          text: filtered ? "Try a different day or clear a filter." : "Nothing is scheduled — be the first to host something.",
          action: filtered ? "Clear filters" : "Host a hangout",
        });
        list.querySelector("[data-action]").addEventListener("click", () => {
          if (!filtered) {
            window.location.hash = "#/host";
            return;
          }
          Object.assign(f, { q: "", range: "", kind: "", tags: [] });
          root.querySelector("#f-q").value = "";
          paintChips();
          load();
        });
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
        if (mine === seq) {
          list.innerHTML = errorHtml(err);
          list.querySelector("[data-action]").addEventListener("click", load);
        }
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

    // --- Ask: natural language → filters (falls back to plain text search) -------------------
    const pill = root.querySelector("#ask-pill");
    const clearFilters = () => {
      Object.assign(f, { q: "", range: "", kind: "", tags: [] });
      root.querySelector("#f-q").value = "";
      pill.hidden = true;
      paintChips();
      load();
    };
    root.querySelector("#ask")?.addEventListener("submit", async (e) => {
      e.preventDefault();
      const input = root.querySelector("#ask-q");
      const text = input.value.trim();
      if (!text) return;
      const go = root.querySelector(".ask__go");
      go.disabled = true;
      go.classList.add("is-busy");
      try {
        const res = await api.post("/api/ask", { text });
        const a = res.filters;
        Object.assign(f, { range: a.when || "", kind: a.kind || "", tags: a.tags || [], q: a.q || "" });
        for (const t of f.tags) if (!tagList.some((x) => x.tag === t)) tagList.unshift({ tag: t, count: 0 });
        root.querySelector("#f-q").value = f.q;
        pill.innerHTML = `<span>✨ ${esc(res.summary)}</span><button type="button" aria-label="Clear this search" data-clear>×</button>`;
        pill.hidden = false;
        pill.querySelector("[data-clear]").addEventListener("click", () => {
          input.value = "";
          clearFilters();
        });
      } catch {
        // LLM down (503) or any failure: quietly search the words instead.
        Object.assign(f, { range: "", kind: "", tags: [], q: text });
        root.querySelector("#f-q").value = text;
        pill.hidden = true;
      } finally {
        go.disabled = false;
        go.classList.remove("is-busy");
      }
      paintChips();
      shown = PAGE;
      await load();
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
