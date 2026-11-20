/**
 * "Browse" view (task B1 shell; filters land in B5).
 */
import { esc } from "../util.js";

const ICON = `
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <circle cx="11" cy="11" r="7"/>
    <path d="M21 21l-4.35-4.35"/>
  </svg>`;

/** Renders the "Browse" view. */
export default {
  name: "browse",
  async render(root) {
    root.innerHTML = `
      <header class="view-head">
        <h1>Browse</h1>
        <p class="lede">Everything on the KSU calendar, plus every student hangout — filter by kind, tag or date, or search by name.</p>
      </header>
      <section class="empty-state card" aria-label="Browse placeholder">
        <span class="empty-state__icon">${ICON}</span>
        <h2>All events live here</h2>
        <p>${esc("Official campus events and student-hosted hangouts, newest first, with filters for kind, tags and dates.")}</p>
      </section>`;
  },
};
