/**
 * Check-in view (B8).
 *
 * Host mode (`#/checkin/<event id>`): shows the rotating check-in link
 * (refreshes every 20 s; tokens last ~60 s) with a copy button.
 * Attendee mode (`/checkin?event=…&token=…`, the link the host shares):
 * posts the token to `/api/checkin` and confirms.
 * NOTE: the link is shown as text/copy — no QR image is drawn, because the
 * frontend ships no dependencies and has no QR encoder yet.
 */
import { api } from "../api.js";
import { esc, fmtEventTime } from "../util.js";

/** Renders the check-in view. */
export default {
  name: "checkin",
  async render(root, { state, params }) {
    if (!state.me) {
      root.innerHTML = `<header class="view-head"><h1>Check in</h1></header>
        <section class="empty-state card"><h2>Sign in first</h2>
        <p>Use the Sign in button (top right), then open the check-in link again.</p></section>`;
      return;
    }

    // Attendee: arrived through the host's link.
    if (params.token) {
      root.innerHTML = `<header class="view-head"><h1>Check in</h1></header><p class="muted" role="status">Checking you in…</p>`;
      try {
        const res = await api.post("/api/checkin", { event_id: params.event, token: params.token });
        root.innerHTML = `<header class="view-head"><h1>You're checked in</h1></header>
          <section class="card panel"><p>Checked in at ${esc(fmtEventTime(res.at, null).split(" · ")[1])}. Have fun!</p>
          <a class="btn btn--primary btn--sm" href="/#/event/${esc(res.event_id)}">Back to the event</a></section>`;
      } catch (err) {
        root.innerHTML = `<header class="view-head"><h1>Check in</h1></header>
          <p class="form-error" role="alert">${esc(err.status === 400 ? "That check-in code expired — ask the host to refresh it and scan again." : err.message)}</p>`;
      }
      return;
    }

    // Host: rotating link.
    const id = params.id;
    const ev = await api.get(`/api/events/${encodeURIComponent(id)}`);
    root.innerHTML = `
      <p><a class="back-link" href="#/event/${esc(id)}">← Back</a></p>
      <header class="view-head"><h1>Check-in · ${esc(ev.title)}</h1>
        <p class="lede">Share this link with attendees as they arrive. It refreshes every 20 seconds.</p></header>
      <section class="card panel">
        <p id="ci-msg" class="muted" role="status" aria-live="polite">Loading…</p>
        <code id="ci-link" class="ci-link"></code>
        <button type="button" class="btn btn--ghost btn--sm" id="ci-copy">Copy link</button>
      </section>`;
    const msg = root.querySelector("#ci-msg");
    const link = root.querySelector("#ci-link");
    let url = "";
    const load = async () => {
      if (!root.isConnected || !root.querySelector("#ci-link")) return;
      try {
        const qr = await api.get(`/api/events/${encodeURIComponent(id)}/qr`);
        url = new URL(qr.checkin_url, window.location.origin).href;
        link.textContent = url;
        msg.textContent = "Live check-in link:";
      } catch (err) {
        msg.textContent = err.status === 403 ? "Only the host can open check-in." : err.message;
        return;
      }
      setTimeout(load, 20000);
    };
    root.querySelector("#ci-copy").addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(url);
        msg.textContent = "Copied.";
      } catch {
        msg.textContent = "Select the link above and copy it.";
      }
    });
    await load();
  },
};
