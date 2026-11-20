/**
 * "For you" view: horizontally scrolling carousels derived client-side from
 * `/api/feed` (top picks) and `/api/events` (Tonight, This weekend, friends,
 * interests). Sections are de-duplicated and hidden when empty.
 */
import { api } from "../api.js";
import { esc, localDate, dayOffset, weekendDays } from "../util.js";
import { eventCard, whyHtml } from "../components.js";
import { openSheet, skeleton } from "../ui.js";

/** One carousel: a labelled, keyboard-scrollable region of cards. */
function rail(title, cards, { more } = {}) {
  const id = `rail-${title.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`;
  return `<section class="rail-section" aria-labelledby="${id}">
    <div class="rail-section__head"><h2 id="${id}">${esc(title)}</h2>${
      more ? `<a class="rail-section__more" href="${esc(more)}">See all</a>` : ""
    }</div>
    <div class="rail" role="region" aria-label="${esc(title)}" tabindex="0">${cards}</div>
  </section>`;
}

/** Time-of-day greeting. */
function greeting(name) {
  const h = new Date().getHours();
  const part = h < 5 ? "Late night" : h < 12 ? "Good morning" : h < 18 ? "Good afternoon" : "Good evening";
  return name ? `${part}, ${esc(name.split(" ")[0])}` : "Find your people";
}

/** Renders the "For you" view. */
export default {
  name: "feed",
  async render(root, { state }) {
    const me = state.me;
    root.innerHTML = `<header class="greeting view-head"><h1>${greeting(me && me.user.display_name)}</h1>
      <p>${me ? "Here's what's on for you." : "Campus events and hangouts, with the people to go with."}</p></header>
      ${skeleton("rails")}`;

    const [feed, events] = await Promise.all([
      me ? api.get("/api/feed").catch(() => ({ items: [] })) : Promise.resolve({ items: [] }),
      api.get("/api/events?limit=200"),
    ]);

    const items = new Map(feed.items.map((i) => [i.event.id, i]));
    const seen = new Set();
    const take = (list, max = 10) => {
      const out = list.filter((e) => !seen.has(e.id)).slice(0, max);
      out.forEach((e) => seen.add(e.id));
      return out;
    };
    const card = (e, variant = "card") => eventCard(items.get(e.id) || e, variant);

    const today = dayOffset(0);
    const weekend = weekendDays();
    const upcoming = events.filter((e) => new Date(e.ends_at) > new Date());
    const sections = [];

    // Top picks: the recommender's best, as hero cards (logged out: soonest events).
    const tops = me ? take(feed.items.map((i) => i.event), 5) : take(upcoming, 5);
    if (tops.length) {
      sections.push(rail(me ? "Top picks for you" : "Coming up", tops.map((e) => card(e, "hero")).join("")));
    }
    const tonight = take(upcoming.filter((e) => localDate(e.starts_at) === today));
    if (tonight.length) sections.push(rail("Tonight", tonight.map((e) => card(e)).join("")));
    const wk = take(upcoming.filter((e) => weekend.includes(localDate(e.starts_at))));
    if (wk.length) sections.push(rail("This weekend", wk.map((e) => card(e)).join("")));
    const fr = take(upcoming.filter((e) => e.friends && (e.friends.going > 0 || e.friends.interested > 0)));
    if (fr.length) sections.push(rail("Your friends are going", fr.map((e) => card(e)).join("")));
    if (me) {
      for (const interest of me.interests.slice(0, 3)) {
        const list = take(upcoming.filter((e) => (e.tags || []).includes(interest)));
        if (list.length) {
          sections.push(rail(`Because you like ${interest.replace(/-/g, " ")}`, list.map((e) => card(e)).join("")));
        }
      }
    }

    const end = `<section class="caught-up" aria-label="End of feed">
      <span class="caught-up__emoji" aria-hidden="true">🎉</span>
      <h2>You're caught up</h2>
      <p>That's everything for now.</p>
      <div class="caught-up__actions">
        <a class="btn btn--soft" href="#/browse">Browse everything</a>
        <a class="btn btn--primary" href="#/host">Host a hangout</a>
      </div></section>`;
    const nudge = me
      ? ""
      : `<section class="empty"><h2>Get picks made for you</h2><p>Sign in to see recommendations based on your interests and friends.</p>
         <button type="button" class="btn btn--primary" data-signin>Sign in</button></section>`;

    root.innerHTML = `<header class="greeting view-head"><h1>${greeting(me && me.user.display_name)}</h1>
      <p>${me ? "Here's what's on for you." : "Campus events and hangouts, with the people to go with."}</p></header>
      ${sections.join("") || `<section class="empty"><span class="empty__emoji" aria-hidden="true">🌙</span><h2>Quiet week</h2><p>Nothing coming up yet — host something!</p></section>`}
      ${nudge}${end}`;

    root.querySelector("[data-signin]")?.addEventListener("click", () => state.openLogin());
    root.querySelectorAll("[data-why]").forEach((b) =>
      b.addEventListener("click", (e) => {
        e.preventDefault();
        const item = items.get(b.dataset.why);
        if (item) openSheet("Why am I seeing this?", whyHtml(item));
      }),
    );
  },
};
