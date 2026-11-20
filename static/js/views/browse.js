/**
 * "Browse" view (B5): all upcoming events with kind / tag / date filters and
 * text search. Filters are sent to `/api/events`; the date filter and a
 * defensive re-check of the others run client-side too (so mock mode filters).
 */
import { api } from "../api.js";
import { esc, localDate } from "../util.js";
import { eventCard } from "../components.js";

/** Keeps events matching the active filters. */
function applyFilters(events, { kind, tags, q, date }) {
  const needle = q.trim().toLowerCase();
  return events.filter((e) => {
    if (kind && e.kind !== kind) return false;
    if (tags.length && !tags.some((t) => (e.tags || []).includes(t))) return false;
    if (needle && !`${e.title} ${(e.tags || []).join(" ")}`.toLowerCase().includes(needle)) {
      return false;
    }
    if (date && localDate(e.starts_at) !== date) return false;
    return true;
  });
}

/** Renders the "Browse" view. */
export default {
  name: "browse",
  async render(root) {
    const filters = { kind: "", tags: [], q: "", date: "" };
    let tagList = [];
    try {
      tagList = await api.get("/api/tags");
    } catch {
      /* tags are optional */
    }

    root.innerHTML = `
      <header class="view-head">
        <h1>Browse</h1>
        <p class="lede">Everything coming up on campus — official events and student hangouts.</p>
      </header>
      <section class="card panel filters" aria-label="Filters">
        <div class="field-row">
          <label class="field"><span>Search</span><input type="search" id="f-q" placeholder="Title or keyword" /></label>
          <label class="field"><span>Type</span>
            <select id="f-kind"><option value="">All</option><option value="official">Official</option><option value="hangout">Hangouts</option></select></label>
          <label class="field"><span>Day</span><input type="date" id="f-date" /></label>
        </div>
        <div class="chips" role="group" aria-label="Tags">
          ${tagList
            .map((t) => `<button type="button" class="chip chip--btn" data-tag="${esc(t.tag)}" aria-pressed="false">#${esc(t.tag)}</button>`)
            .join("")}
        </div>
      </section>
      <p id="b-count" class="muted" role="status" aria-live="polite"></p>
      <div id="b-list" class="card-grid"></div>`;

    const list = root.querySelector("#b-list");
    const count = root.querySelector("#b-count");
    let seq = 0;
    const load = async () => {
      const mine = ++seq;
      const qs = new URLSearchParams();
      if (filters.kind) qs.set("kind", filters.kind);
      if (filters.tags.length) qs.set("tags", filters.tags.join(","));
      if (filters.q.trim()) qs.set("q", filters.q.trim());
      qs.set("limit", "100");
      let events;
      try {
        events = await api.get(`/api/events?${qs}`);
      } catch (err) {
        if (mine === seq) count.textContent = err.message || "Could not load events.";
        return;
      }
      if (mine !== seq) return;
      const shown = applyFilters(events, filters);
      count.textContent = shown.length ? `${shown.length} event${shown.length === 1 ? "" : "s"}` : "";
      list.innerHTML = shown.length
        ? shown.map(eventCard).join("")
        : `<section class="empty-state card"><h2>No matches</h2><p>Try fewer filters or a different day.</p></section>`;
    };

    let timer;
    root.querySelector("#f-q").addEventListener("input", (e) => {
      filters.q = e.target.value;
      clearTimeout(timer);
      timer = setTimeout(load, 250);
    });
    root.querySelector("#f-kind").addEventListener("change", (e) => {
      filters.kind = e.target.value;
      load();
    });
    root.querySelector("#f-date").addEventListener("change", (e) => {
      filters.date = e.target.value;
      load();
    });
    root.querySelectorAll("[data-tag]").forEach((b) =>
      b.addEventListener("click", () => {
        const t = b.dataset.tag;
        const on = !filters.tags.includes(t);
        filters.tags = on ? [...filters.tags, t] : filters.tags.filter((x) => x !== t);
        b.setAttribute("aria-pressed", String(on));
        b.classList.toggle("is-selected", on);
        load();
      }),
    );
    await load();
  },
};
