/**
 * "Host" view (B4): a hangout form with a "describe it" box that prefills it
 * via `/api/parse`, plus a crew availability grid that suggests a time.
 */
import { api } from "../api.js";
import { esc, fmtClock, localDate, localHM, todayLocal } from "../util.js";

/** Splits "a, b" into trimmed non-empty tags. */
const parseTags = (s) =>
  s.split(",").map((t) => t.trim().replace(/^#/, "")).filter(Boolean);

/** Renders the crew availability grid into `box`. */
function gridHtml(av) {
  const total = av.members.length;
  const best = av.best ? av.best.starts_at : null;
  const cells = av.slots
    .map((s) => {
      const n = s.available.length;
      const level = total ? Math.ceil((n / total) * 4) : 0;
      const isBest = s.starts_at === best;
      return `<button type="button" class="slot slot--${level}${isBest ? " is-best" : ""}"
        data-start="${esc(s.starts_at)}" data-end="${esc(s.ends_at)}"
        aria-label="${esc(fmtClock(s.starts_at))}: ${n} of ${total} free${isBest ? " (best)" : ""}">
        <span class="slot__time">${esc(fmtClock(s.starts_at))}</span>
        <span class="slot__n">${n}/${total}</span></button>`;
    })
    .join("");
  const note = av.best
    ? `Best time: <strong>${esc(fmtClock(av.best.starts_at))}</strong> — ${av.best.available.length} of ${total} free. Tap a slot to use it.`
    : "Nobody has shared availability for this day yet.";
  return `<p>${note}</p><div class="slot-grid">${cells}</div>`;
}

/** Renders the "Host" view. */
export default {
  name: "host",
  async render(root, { state }) {
    const head = `
      <header class="view-head">
        <h1>Host a hangout</h1>
        <p class="lede">Pick a time, a place and who you're bringing — or just describe it and we'll fill the form.</p>
      </header>`;
    if (!state.me) {
      root.innerHTML = `${head}<section class="empty-state card"><h2>Sign in to host</h2>
        <p>Use the Sign in button (top right) to pick a demo student.</p></section>`;
      return;
    }
    let crews = [];
    try {
      crews = await api.get("/api/crews");
    } catch {
      /* crews are optional */
    }

    root.innerHTML = `${head}
      <section class="card panel" aria-labelledby="nl-h">
        <h2 id="nl-h">Describe it</h2>
        <label class="field field--full"><span class="visually-hidden">Describe your hangout</span>
          <textarea id="nl-text" rows="2" placeholder="Smash tonight 7–10 in the game room, I have 2 Switches"></textarea></label>
        <button type="button" class="btn btn--ghost btn--sm" id="nl-go">Prefill the form</button>
        <p id="nl-msg" class="muted" role="status" aria-live="polite"></p>
      </section>

      <form id="host-form" class="card panel" novalidate>
        <h2>Details</h2>
        <label class="field"><span>Title</span><input name="title" required maxlength="100" /></label>
        <label class="field"><span>Description</span><textarea name="description" rows="3"></textarea></label>
        <div class="field-row">
          <label class="field"><span>Date</span><input type="date" name="date" required value="${todayLocal()}" /></label>
          <label class="field"><span>Start</span><input type="time" name="start" required /></label>
          <label class="field"><span>End</span><input type="time" name="end" /></label>
        </div>
        <div class="field-row">
          <label class="field"><span>Where</span><input name="location" required placeholder="Student Center" /></label>
          <label class="field"><span>Room</span><input name="room" /></label>
        </div>
        <div class="field-row">
          <label class="field"><span>Tags (comma-separated)</span><input name="tags" placeholder="smash, gaming" /></label>
          <label class="field"><span>Capacity</span><input type="number" min="1" name="capacity" /></label>
        </div>
        <label class="field"><span>Crew (optional)</span>
          <select name="crew">
            <option value="">No crew</option>
            ${crews.map((c) => `<option value="${esc(c.id)}">${esc(c.name)}</option>`).join("")}
          </select></label>
        <p id="host-error" class="form-error" role="alert" hidden></p>
        <button class="btn btn--primary" type="submit">Create hangout</button>
      </form>

      ${
        crews.length
          ? `<section class="card panel" aria-labelledby="av-h">
               <h2 id="av-h">When is your crew free?</h2>
               <div class="field-row">
                 <label class="field"><span>Crew</span><select id="av-crew">${crews
                   .map((c) => `<option value="${esc(c.id)}">${esc(c.name)}</option>`)
                   .join("")}</select></label>
                 <label class="field"><span>Day</span><input type="date" id="av-date" value="${todayLocal()}" /></label>
               </div>
               <div id="av-box" class="av-box" aria-live="polite"></div>
             </section>`
          : ""
      }`;

    const form = root.querySelector("#host-form");
    const errEl = root.querySelector("#host-error");
    const f = (name) => form.elements[name];

    // --- Natural-language prefill ---------------------------------------------------
    const nlMsg = root.querySelector("#nl-msg");
    root.querySelector("#nl-go").addEventListener("click", async () => {
      const text = root.querySelector("#nl-text").value.trim();
      if (!text) {
        nlMsg.textContent = "Type a sentence first.";
        return;
      }
      nlMsg.textContent = "Reading it…";
      try {
        const d = await api.post("/api/parse", { text });
        if (d.title) f("title").value = d.title;
        if (d.description) f("description").value = d.description;
        if (d.date) f("date").value = d.date;
        if (d.start) f("start").value = d.start;
        if (d.end) f("end").value = d.end;
        if (d.location) f("location").value = d.location;
        if (d.tags && d.tags.length) f("tags").value = d.tags.join(", ");
        if (d.capacity) f("capacity").value = d.capacity;
        if (d.gear_notes && !f("description").value.includes(d.gear_notes)) {
          f("description").value = `${f("description").value}\nBring/provided: ${d.gear_notes}`.trim();
        }
        nlMsg.textContent = "Filled in what we could — check it over and create.";
      } catch (err) {
        nlMsg.textContent =
          err && err.status === 503
            ? "The describe-it helper is offline right now — fill the form below instead."
            : (err && err.message) || "Couldn't read that.";
      }
    });

    // --- Create ---------------------------------------------------------------------
    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      errEl.hidden = true;
      const body = {
        title: f("title").value.trim(),
        description: f("description").value.trim(),
        date: f("date").value,
        start: f("start").value,
        end: f("end").value || null,
        location: { name: f("location").value.trim(), room: f("room").value.trim() || null },
        tags: parseTags(f("tags").value),
        capacity: f("capacity").value ? Number(f("capacity").value) : null,
        crew_id: f("crew").value || null,
      };
      try {
        const ev = await api.post("/api/events", body);
        window.location.hash = `#/event/${ev.id}`;
      } catch (err) {
        errEl.textContent = err && err.message ? err.message : "Could not create the hangout.";
        errEl.hidden = false;
      }
    });

    // --- Crew availability ------------------------------------------------------------
    const box = root.querySelector("#av-box");
    if (box) {
      const load = async () => {
        const crewId = root.querySelector("#av-crew").value;
        const date = root.querySelector("#av-date").value;
        box.innerHTML = `<p class="muted">Loading…</p>`;
        try {
          const av = await api.get(
            `/api/crews/${encodeURIComponent(crewId)}/availability?date=${encodeURIComponent(date)}`,
          );
          box.innerHTML = gridHtml(av);
          box.querySelectorAll(".slot").forEach((b) =>
            b.addEventListener("click", () => {
              f("date").value = localDate(b.dataset.start);
              f("start").value = localHM(b.dataset.start);
              f("end").value = localHM(b.dataset.end);
              f("crew").value = crewId;
              form.scrollIntoView({ behavior: "smooth" });
            }),
          );
        } catch (err) {
          box.innerHTML = `<p class="form-error">${esc(err.message || "Could not load availability.")}</p>`;
        }
      };
      root.querySelector("#av-crew").addEventListener("change", load);
      root.querySelector("#av-date").addEventListener("change", load);
      await load();
    }
  },
};
