/**
 * "Host" view (task B1 shell; the hangout form lands in B4).
 */
import { esc } from "../util.js";

const ICON = `
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M12 20h9"/>
    <path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4z"/>
  </svg>`;

/** Renders the "Host" view. */
export default {
  name: "host",
  async render(root, { state }) {
    const hint = state.me
      ? "Describe your hangout in a sentence — or fill the form — and it goes live for your friends and the feed."
      : "Sign in to host a hangout. Describe it in a sentence — or fill the form — and it goes live for your friends and the feed.";
    root.innerHTML = `
      <header class="view-head">
        <h1>Host a hangout</h1>
        <p class="lede">Pick a time, a place and who you're bringing. Natural-language prefill handles the typing.</p>
      </header>
      <section class="empty-state card" aria-label="Host placeholder">
        <span class="empty-state__icon">${ICON}</span>
        <h2>The hangout form lives here</h2>
        <p>${esc(hint)}</p>
      </section>`;
  },
};
