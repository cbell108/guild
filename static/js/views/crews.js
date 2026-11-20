/**
 * "Crews" view (task B1 shell; the crew UI lands in B6).
 */
import { esc } from "../util.js";

const ICON = `
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
    <circle cx="9" cy="7" r="4"/>
    <path d="M23 21v-2a4 4 0 0 0-3-3.87"/>
    <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
  </svg>`;

/** Renders the "Crews" view. */
export default {
  name: "crews",
  async render(root, { state }) {
    const hint = state.me
      ? "Crews are small groups that plan hangouts together — shared availability, shared events. Yours will appear here."
      : "Sign in to see your crews. Crews are small groups that plan hangouts together — shared availability, shared events.";
    root.innerHTML = `
      <header class="view-head">
        <h1>Crews</h1>
        <p class="lede">Plan around your people: see who's free, then host a hangout the whole crew can join.</p>
      </header>
      <section class="empty-state card" aria-label="Crews placeholder">
        <span class="empty-state__icon">${ICON}</span>
        <h2>Your crews live here</h2>
        <p>${esc(hint)}</p>
      </section>`;
  },
};
