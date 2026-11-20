/**
 * "For you" view (task B1 shell; the feed itself lands in B2).
 *
 * View contract: default export `{ name, render(root, { state, params }) }`;
 * `render` replaces `root`'s contents and may be async (it may await API
 * calls). `params` carries route parameters (e.g. `{ id }` on the event view).
 */
import { esc } from "../util.js";

const ICON = `
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M12 3l2.1 5.6L20 11l-5.9 2.4L12 19l-2.1-5.6L4 11l5.9-2.4z"/>
  </svg>`;

/** Renders the "For you" view. */
export default {
  name: "feed",
  async render(root, { state }) {
    const hint = state.me
      ? "The recommender fills this list from your interests, your friends and what's happening on campus — best picks first, with a reason for each."
      : "Sign in (top right) and the recommender will fill this list from your interests, your friends and what's happening on campus — best picks first, with a reason for each.";
    root.innerHTML = `
      <header class="view-head">
        <h1>For you</h1>
        <p class="lede">Your picks for the next 7 days — matched to your interests, with friends' plans called out.</p>
      </header>
      <section class="empty-state card" aria-label="Feed placeholder">
        <span class="empty-state__icon">${ICON}</span>
        <h2>Your picks land here</h2>
        <p>${esc(hint)}</p>
      </section>`;
  },
};
