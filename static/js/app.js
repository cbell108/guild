/**
 * Campus Hangouts — app shell (Lane B, task B1).
 *
 * Boots the header/session state, the hash router
 * (`#/`, `#/browse`, `#/crews`, `#/host`, `#/event/<id>`), and the demo
 * login dialog. Views live in `js/views/` and are swapped into `#view`.
 */
import { api, isMock } from "./api.js";
import { esc, initials } from "./util.js";
import feedView from "./views/for-you.js";
import browseView from "./views/browse.js";
import crewsView from "./views/crews.js";
import hostView from "./views/host.js";
import eventView from "./views/event.js";
import privacyView from "./views/privacy.js";
import checkinView from "./views/checkin.js";

/** View registry, keyed by route name. */
const VIEWS = {
  feed: feedView,
  browse: browseView,
  crews: crewsView,
  host: hostView,
  event: eventView,
  privacy: privacyView,
  checkin: checkinView,
};

/**
 * In-memory session state.
 * `me` is the `Me` object from `GET /api/me` (mock: `me.json`) or `null`
 * when logged out.
 */
export const state = {
  /** `?mock=1` active. */
  mock: isMock(),
  /** Current user, or null. */
  me: null,
  /** Current route name. */
  route: "feed",
  /** Route parameters (e.g. `{ id }` for the event view). */
  params: {},
};

/**
 * Splits `location.hash` into a route name and params.
 * Unknown hashes report `known: false` and default to the feed.
 */
function parseHash() {
  // The host's shared link is a real path: /checkin?event=<id>&token=<t>.
  if (window.location.pathname === "/checkin") {
    const q = new URLSearchParams(window.location.search);
    return { name: "checkin", params: { event: q.get("event"), token: q.get("token") }, known: true };
  }
  const raw = window.location.hash.replace(/^#\/?/, "");
  const segs = raw.split("/").filter(Boolean);
  if (segs.length === 0) return { name: "feed", params: {}, known: true };
  if (segs.length === 2 && segs[0] === "event" && segs[1]) {
    return { name: "event", params: { id: segs[1] }, known: true };
  }
  if (segs.length === 2 && segs[0] === "checkin" && segs[1]) {
    return { name: "checkin", params: { id: segs[1] }, known: true };
  }
  if (segs.length === 1 && segs[0] !== "checkin" && VIEWS[segs[0]]) {
    return { name: segs[0], params: {}, known: true };
  }
  return { name: "feed", params: {}, known: false };
}

/** Loads the session; a 401 (or an API that isn't up yet) means logged out. */
async function loadSession() {
  try {
    state.me = await api.get("/api/me");
  } catch {
    state.me = null;
  }
}

/** Signs the viewer out (best-effort; clears local state even if it fails). */
async function logout() {
  try {
    await api.post("/api/logout");
  } catch {
    /* server may be down; local state still resets */
  }
  state.me = null;
  renderHeader();
  renderRoute();
}

/** Renders the header's session area (user chip or a Sign in button). */
function renderHeader() {
  const session = document.getElementById("session");
  if (!session) return;
  if (!state.me) {
    session.innerHTML = `
      <button type="button" class="btn btn--primary btn--sm" id="sign-in-btn">Sign in</button>`;
    session.querySelector("#sign-in-btn").addEventListener("click", openLogin);
    return;
  }
  const me = state.me.user;
  session.innerHTML = `
    <a class="user-chip" href="#/privacy" title="Privacy settings" aria-label="${esc(me.display_name)} — privacy settings">
      <span class="avatar" aria-hidden="true">${esc(initials(me.display_name))}</span>
      <span class="user-chip__name">${esc(me.display_name)}</span>
    </a>
    <button type="button" class="icon-button" id="sign-out-btn" aria-label="Sign out" title="Sign out">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/>
        <path d="M16 17l5-5-5-5M21 12H9"/>
      </svg>
    </button>`;
  session.querySelector("#sign-out-btn").addEventListener("click", logout);
}

/** Highlights the nav link matching the current route. */
function setNavActive() {
  document.querySelectorAll(".app-nav .nav-link").forEach((link) => {
    const active = link.dataset.nav === state.route;
    link.classList.toggle("is-active", active);
    if (active) link.setAttribute("aria-current", "page");
    else link.removeAttribute("aria-current");
  });
}

/** Renders the current route's view into `#view`, with an error fallback. */
async function renderRoute() {
  const { name, params, known } = parseHash();
  state.route = name;
  state.params = params;
  setNavActive();

  // Unknown hash: send the user home; the hashchange re-renders the feed.
  if (!known) {
    window.location.replace("#/");
    return;
  }

  const root = document.getElementById("view");
  if (!root) return;
  window.scrollTo(0, 0);

  const view = VIEWS[name];
  try {
    await view.render(root, { state, params });
  } catch (err) {
    root.innerHTML = `
      <div class="error-panel" role="alert">
        <h2>Something went wrong</h2>
        <p>${esc(err && err.message ? err.message : "Unknown error")}</p>
        <button type="button" class="btn btn--ghost btn--sm" data-retry>Try again</button>
      </div>`;
    root.querySelector("[data-retry]").addEventListener("click", renderRoute);
  }
}

/** Opens the demo login dialog and loads the selectable demo users. */
async function openLogin() {
  const dialog = document.getElementById("login-dialog");
  const list = document.getElementById("login-users");
  if (!dialog || !list) return;
  dialog.showModal();
  list.innerHTML = `<p class="dialog-sub">Loading demo students…</p>`;

  let users;
  try {
    users = await api.get("/api/demo-users");
  } catch (err) {
    list.innerHTML = `
      <div class="error-panel" role="alert">
        <h2>Could not load demo students</h2>
        <p>${esc(err && err.message ? err.message : "Unknown error")}</p>
      </div>`;
    return;
  }
  if (!users || users.length === 0) {
    list.innerHTML = `<p class="dialog-sub">No demo students available.</p>`;
    return;
  }
  list.innerHTML = users
    .map(
      (u) => `
      <button type="button" class="user-row" data-id="${esc(u.id)}">
        <span class="avatar" aria-hidden="true">${esc(initials(u.display_name))}</span>
        <span class="user-row__meta">
          <span class="user-row__name">${esc(u.display_name)}</span>
          <span class="user-row__bio">${esc(u.bio || "")}</span>
        </span>
      </button>`,
    )
    .join("");
  list.querySelectorAll(".user-row").forEach((btn) => {
    btn.addEventListener("click", () => loginAs(btn.dataset.id));
  });
}

/** Logs in as a demo student and refreshes the shell. */
async function loginAs(userId) {
  const list = document.getElementById("login-users");
  try {
    state.me = await api.post("/api/login", { user_id: userId });
  } catch (err) {
    if (list) {
      list.innerHTML = `
        <div class="error-panel" role="alert">
          <h2>Sign-in failed</h2>
          <p>${esc(err && err.message ? err.message : "Unknown error")}</p>
        </div>`;
    }
    return;
  }
  const dialog = document.getElementById("login-dialog");
  if (dialog) dialog.close();
  renderHeader();
  renderRoute();
}

/** Boots the shell: mode pill, session, listeners, first render. */
async function boot() {
  if (state.mock) {
    const pill = document.getElementById("mode-pill");
    if (pill) pill.hidden = false;
  }
  await loadSession();
  renderHeader();
  window.addEventListener("hashchange", renderRoute);
  await renderRoute();
}

boot();
