/**
 * Event detail view (B3): details, Interested / Going / Going-with buttons,
 * the "looking for a buddy" toggle with its match, and an add-to-calendar link.
 */
import { api } from "../api.js";
import { esc, fmtEventTime } from "../util.js";
import { kindBadge, placeLine, friendLine, intentLabel, spotsText } from "../components.js";

const LEVELS = [
  ["interested", "Interested"],
  ["going", "Going"],
  ["going_with", "Going with…"],
];

/** Renders the buddy block from a `BuddyStatus`. */
function buddyHtml(status) {
  const match = status.matched;
  let result = "";
  if (status.looking && match) {
    result = `<p class="buddy-match"><strong>${esc(match.user.display_name)}</strong>
      ${match.is_friend ? '<span class="chip">Friend</span>' : ""}<br>
      <span class="muted">${esc(match.reason)}</span></p>`;
  } else if (status.looking) {
    result = `<p class="muted">You're on the list — we'll show a match as soon as someone fits.</p>`;
  }
  return `
    <label class="toggle">
      <input type="checkbox" id="buddy-toggle" ${status.looking ? "checked" : ""} />
      <span>Looking for a buddy</span>
    </label>
    <div id="buddy-result" aria-live="polite">${result}</div>`;
}

/** Renders the event detail view. */
export default {
  name: "event",
  async render(root, { state, params }) {
    root.innerHTML = `<p class="muted" role="status">Loading event…</p>`;
    const ev = await api.get(`/api/events/${encodeURIComponent(params.id)}`);
    const me = state.me;
    let my = ev.my_intent;
    let buddy = { looking: ev.buddy_looking, matched: null };
    if (me && ev.buddy_looking) {
      try {
        buddy = await api.get(`/api/events/${encodeURIComponent(ev.id)}/buddy`);
      } catch {
        /* buddy match is optional */
      }
    }

    const spots = spotsText(ev);
    const friends = friendLine(ev.friends);
    const meta = [
      ["When", esc(fmtEventTime(ev.starts_at, ev.ends_at))],
      ["Where", placeLine(ev.location)],
      ev.host ? ["Host", esc(ev.host.display_name)] : null,
      ev.organizer ? ["Organizer", esc(ev.organizer)] : null,
      ev.crew ? ["Crew", esc(ev.crew.name)] : null,
      friends ? ["Friends", `<strong>${esc(friends)}</strong>`] : null,
      spots ? ["Spots", esc(spots)] : null,
    ]
      .filter(Boolean)
      .map(([k, v]) => `<div><dt>${k}</dt><dd>${v}</dd></div>`)
      .join("");

    root.innerHTML = `
      <p><a class="back-link" href="#/">← Back</a></p>
      <article class="card event-detail">
        ${ev.image_url ? `<img class="event-detail__img" src="${esc(ev.image_url)}" alt="" loading="lazy" onerror="this.remove()" />` : ""}
        <div class="event-detail__body">
          ${kindBadge(ev.kind)}
          <h1>${esc(ev.title)}</h1>
          <dl class="detail-meta">${meta}</dl>
          <p class="event-detail__desc">${esc(ev.description || "")}</p>
          <div class="chips">${(ev.tags || []).map((t) => `<span class="chip">#${esc(t)}</span>`).join("")}</div>
          ${ev.url ? `<p><a href="${esc(ev.url)}" target="_blank" rel="noopener noreferrer">View on the KSU calendar ↗</a></p>` : ""}
        </div>
      </article>

      <section class="card panel" aria-labelledby="attend-h">
        <h2 id="attend-h">Your plans</h2>
        ${
          me
            ? `<p id="intent-status" class="status-line" aria-live="polite"></p>
               <div class="seg" role="group" aria-label="Your attendance">
                 ${LEVELS.map(([v, l]) => `<button type="button" class="btn btn--ghost" data-level="${v}">${l}</button>`).join("")}
               </div>
               <div id="with-picker" class="with-picker" hidden></div>
               <button type="button" class="btn btn--ghost btn--sm" id="cant-make-it" hidden>Can't make it</button>
               <p id="intent-error" class="form-error" role="alert" hidden></p>`
            : `<p class="muted">Sign in (top right) to say you're interested or going.</p>`
        }
        <a class="btn btn--ghost btn--sm" href="${esc(ev.calendar_url)}" download>Add to calendar</a>
        ${me && ev.host && ev.host.id === me.user.id ? `<a class="btn btn--ghost btn--sm" href="#/checkin/${esc(ev.id)}">Open check-in</a>` : ""}
      </section>

      ${
        me
          ? `<section class="card panel" aria-labelledby="buddy-h">
               <h2 id="buddy-h">Find a buddy</h2>
               <p class="muted">Don't want to go alone? Say so and we'll suggest someone who'd get along.</p>
               <div id="buddy">${buddyHtml(buddy)}</div>
             </section>`
          : ""
      }`;

    if (!me) return;

    const statusEl = root.querySelector("#intent-status");
    const errEl = root.querySelector("#intent-error");
    const cant = root.querySelector("#cant-make-it");
    const picker = root.querySelector("#with-picker");
    const buttons = [...root.querySelectorAll("[data-level]")];

    const paint = () => {
      const label = intentLabel(my);
      statusEl.textContent = label ? `You: ${label}` : "You haven't responded yet.";
      buttons.forEach((b) => {
        const on = my && my.level === b.dataset.level;
        b.classList.toggle("is-selected", Boolean(on));
        b.setAttribute("aria-pressed", String(Boolean(on)));
      });
      cant.hidden = !my;
    };
    const fail = (err) => {
      errEl.textContent = err && err.message ? err.message : "Something went wrong";
      errEl.hidden = false;
    };
    const setIntent = async (level, withId) => {
      errEl.hidden = true;
      try {
        my = await api.put(`/api/events/${encodeURIComponent(ev.id)}/intent`, {
          level,
          with: withId || null,
        });
        picker.hidden = true;
        paint();
      } catch (err) {
        fail(err);
      }
    };

    buttons.forEach((b) =>
      b.addEventListener("click", async () => {
        if (b.dataset.level !== "going_with") return setIntent(b.dataset.level);
        errEl.hidden = true;
        picker.hidden = false;
        picker.innerHTML = `<span class="muted">Loading friends…</span>`;
        try {
          const { friends: list } = await api.get("/api/friends");
          picker.innerHTML = list.length
            ? `<span class="muted">Going with:</span> ${list
                .map((f) => `<button type="button" class="chip chip--btn" data-friend="${esc(f.id)}">${esc(f.display_name)}</button>`)
                .join("")}`
            : `<span class="muted">Add a friend first to go together.</span>`;
          picker.querySelectorAll("[data-friend]").forEach((fb) =>
            fb.addEventListener("click", () => setIntent("going_with", fb.dataset.friend)),
          );
        } catch (err) {
          picker.hidden = true;
          fail(err);
        }
      }),
    );
    cant.addEventListener("click", async () => {
      errEl.hidden = true;
      try {
        await api.del(`/api/events/${encodeURIComponent(ev.id)}/intent`);
        my = null;
        paint();
      } catch (err) {
        fail(err);
      }
    });
    paint();

    const buddyRoot = root.querySelector("#buddy");
    const bindBuddy = () => {
      const box = buddyRoot.querySelector("#buddy-toggle");
      box.addEventListener("change", async () => {
        box.disabled = true;
        try {
          buddy = await api.post(`/api/events/${encodeURIComponent(ev.id)}/buddy`, { looking: box.checked });
        } catch (err) {
          box.checked = !box.checked;
          fail(err);
        }
        buddyRoot.innerHTML = buddyHtml(buddy);
        bindBuddy();
      });
    };
    bindBuddy();
  },
};
