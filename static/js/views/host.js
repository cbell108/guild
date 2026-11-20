/**
 * "Host" view: "Describe your hangout" is the hero — a big textarea and a
 * ✨ Fill it in button calling `/api/parse`; the filled fields animate in as a
 * draft summary. Manual fields live under a collapsible "Details" (opened
 * automatically if the helper is down or a field is missing). The crew
 * availability grid sits below.
 */
import { api } from "../api.js";
import { esc, fmtClock, localDate, localHM, todayLocal } from "../util.js";
import { toast } from "../ui.js";

/** Splits "a, b" into trimmed non-empty tags. */
const parseTags = (s) => s.split(",").map((t) => t.trim().replace(/^#/, "")).filter(Boolean);

/** Crew availability grid markup. */
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
        <span>${esc(fmtClock(s.starts_at))}</span><small>${n}/${total}</small></button>`;
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
    const head = `<header class="view-head"><h1>Host a hangout</h1></header>`;
    if (!state.me) {
      root.innerHTML = `${head}<section class="empty"><span class="empty__emoji" aria-hidden="true">🎈</span>
        <h2>Sign in to host</h2><p>Pick a demo student to host as.</p>
        <button type="button" class="btn btn--primary" data-signin>Sign in</button></section>`;
      root.querySelector("[data-signin]").addEventListener("click", () => state.openLogin());
      return;
    }
    let crews = [];
    try {
      crews = await api.get("/api/crews");
    } catch {
      /* crews are optional */
    }

    root.innerHTML = `${head}
      <section class="describe" aria-labelledby="nl-h">
        <h2 id="nl-h">Describe your hangout</h2>
        <p class="muted">One sentence is enough — we'll fill in the rest.</p>
        <label class="visually-hidden" for="nl-text">Describe your hangout</label>
        <textarea id="nl-text" rows="4" placeholder="Smash tonight 7–10 in the game room, I have 2 Switches"></textarea>
        <button type="button" class="btn btn--primary btn--block" id="nl-go"><span aria-hidden="true">✨</span> Fill it in</button>
        <p id="nl-msg" class="muted" role="status" aria-live="polite"></p>
        <div id="draft" class="draft" hidden></div>
      </section>

      <form id="host-form" class="host-form" novalidate>
        <details id="details" class="details">
          <summary>Details</summary>
          <div class="details__body">
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
              <select name="crew"><option value="">No crew</option>${crews
                .map((c) => `<option value="${esc(c.id)}">${esc(c.name)}</option>`)
                .join("")}</select></label>
          </div>
        </details>
        <p id="host-error" class="form-error" role="alert" hidden></p>
        <button class="btn btn--primary btn--block" type="submit">Create hangout</button>
      </form>

      ${
        crews.length
          ? `<details class="details" id="av-details"><summary>When is your crew free?</summary>
               <div class="details__body">
                 <div class="field-row">
                   <label class="field"><span>Crew</span><select id="av-crew">${crews
                     .map((c) => `<option value="${esc(c.id)}">${esc(c.name)}</option>`)
                     .join("")}</select></label>
                   <label class="field"><span>Day</span><input type="date" id="av-date" value="${todayLocal()}" /></label>
                 </div>
                 <div id="av-box" aria-live="polite"></div>
               </div></details>`
          : ""
      }`;

    const form = root.querySelector("#host-form");
    const details = root.querySelector("#details");
    const errEl = root.querySelector("#host-error");
    const f = (name) => form.elements[name];

    // --- Natural-language prefill ------------------------------------------------------
    const nlMsg = root.querySelector("#nl-msg");
    const draft = root.querySelector("#draft");
    const go = root.querySelector("#nl-go");
    go.addEventListener("click", async () => {
      const text = root.querySelector("#nl-text").value.trim();
      if (!text) {
        nlMsg.textContent = "Type a sentence first.";
        return;
      }
      nlMsg.textContent = "Reading it…";
      go.disabled = true;
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
        const bits = [
          d.title && ["🏷️", d.title],
          d.date && d.start && ["🕖", `${d.date} · ${d.start}${d.end ? `–${d.end}` : ""}`],
          d.location && ["📍", d.location],
          d.capacity && ["👥", `${d.capacity} spots`],
          d.tags && d.tags.length && ["#", d.tags.join(" · ")],
        ].filter(Boolean);
        draft.hidden = false;
        draft.innerHTML = bits
          .map(([i, t], k) => `<span class="draft__item" style="--i:${k}"><span aria-hidden="true">${i}</span> ${esc(t)}</span>`)
          .join("");
        nlMsg.textContent = "Here's your draft — check it over and create.";
        if (!d.title || !d.start || !d.location) {
          details.open = true;
          nlMsg.textContent = "Filled in what we could — complete the rest under Details.";
        }
      } catch (err) {
        details.open = true;
        const down = err && err.status === 503;
        nlMsg.innerHTML = `${
          down
            ? "The describe-it helper is offline right now — fill in the details below, or"
            : esc((err && err.message) || "Couldn't read that — use the details below, or")
        } <button type="button" class="link-btn link-btn--brand" data-retry>try again</button>.`;
        nlMsg.querySelector("[data-retry]").addEventListener("click", () => go.click());
      } finally {
        go.disabled = false;
      }
    });

    // --- Create ----------------------------------------------------------------------------
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
        toast("Hangout created 🎉");
        window.location.hash = `#/event/${ev.id}`;
      } catch (err) {
        details.open = true;
        errEl.textContent = err && err.message ? err.message : "Could not create the hangout.";
        errEl.hidden = false;
      }
    });

    // --- Crew availability --------------------------------------------------------------------
    const box = root.querySelector("#av-box");
    if (box) {
      const load = async () => {
        const crewId = root.querySelector("#av-crew").value;
        const date = root.querySelector("#av-date").value;
        box.innerHTML = `<div class="skel" style="height:6rem"></div>`;
        try {
          const av = await api.get(`/api/crews/${encodeURIComponent(crewId)}/availability?date=${encodeURIComponent(date)}`);
          box.innerHTML = gridHtml(av);
          box.querySelectorAll(".slot").forEach((b) =>
            b.addEventListener("click", () => {
              f("date").value = localDate(b.dataset.start);
              f("start").value = localHM(b.dataset.start);
              f("end").value = localHM(b.dataset.end);
              f("crew").value = crewId;
              details.open = true;
              toast("Time added to your hangout");
              details.scrollIntoView({ behavior: "smooth" });
            }),
          );
        } catch (err) {
          box.innerHTML = `<p class="form-error">${esc(err.message || "Could not load availability.")}</p>`;
        }
      };
      root.querySelector("#av-crew").addEventListener("change", load);
      root.querySelector("#av-date").addEventListener("change", load);
      root.querySelector("#av-details").addEventListener("toggle", (e) => {
        if (e.target.open && !box.innerHTML) load();
      });
    }
  },
};
