/**
 * Event detail view (task B1 shell; details, intents and buddy matching
 * land in B3).
 */
import { esc } from "../util.js";

const ICON = `
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <rect x="3" y="4" width="18" height="18" rx="2"/>
    <path d="M16 2v4M8 2v4M3 10h18"/>
  </svg>`;

/** Renders the event detail stub. */
export default {
  name: "event",
  async render(root, { params }) {
    root.innerHTML = `
      <header class="view-head">
        <h1>Event</h1>
        <p class="lede">Details, attendance, buddy matching and the calendar link for ${esc(params.id || "this event")}</p>
      </header>
      <section class="empty-state card" aria-label="Event placeholder">
        <span class="empty-state__icon">${ICON}</span>
        <h2>Event details land here</h2>
        <p>${esc("Full details, the Interested / Going / Going-with buttons, the buddy toggle and an add-to-calendar link.")}</p>
      </section>`;
  },
};
