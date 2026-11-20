/**
 * "For you" view (B2): the recommender feed as event cards with friend
 * presence and one reason line each, ending in a "You're caught up" state.
 *
 * View contract: default export `{ name, render(root, { state, params }) }`.
 */
import { api } from "../api.js";
import { esc } from "../util.js";
import { eventCard } from "../components.js";

/** Renders the "For you" view. */
export default {
  name: "feed",
  async render(root, { state }) {
    const head = `
      <header class="view-head">
        <h1>For you</h1>
        <p class="lede">Your picks for the next 7 days — matched to your interests, with friends' plans called out.</p>
      </header>`;
    if (!state.me) {
      root.innerHTML = `${head}
        <section class="empty-state card">
          <h2>Sign in to see your picks</h2>
          <p>Use the Sign in button (top right) to pick a demo student. Meanwhile, <a href="#/browse">browse all events</a>.</p>
        </section>`;
      return;
    }
    root.innerHTML = `${head}<p class="muted" role="status">Loading your picks…</p>`;
    const feed = await api.get("/api/feed");
    const items = feed.items || [];
    const cards = items.map(eventCard).join("");
    const end = feed.caught_up
      ? `<section class="caught-up" aria-label="End of feed">
           <h2>You're caught up</h2>
           <p>That's everything for the next 7 days. <a href="#/browse">Browse more</a> or <a href="#/host">host your own</a>.</p>
         </section>`
      : "";
    root.innerHTML = `${head}
      ${items.length ? `<div class="card-grid">${cards}</div>` : `<p class="muted">${esc("Nothing matched yet.")}</p>`}
      ${end}`;
  },
};
