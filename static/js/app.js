/**
 * Campus Hangouts — app shell.
 *
 * Boots the session, the hash router (`#/`, `#/browse`, `#/host`, `#/crews`,
 * `#/me`, `#/privacy`, `#/event/<id>`, `#/checkin/<id>`), the nav, the demo
 * login sheet and view transitions. Views live in `js/views/` and are
 * swapped into `#view`.
 */
import { api, isMock } from "./api.js";
import { esc } from "./util.js";
import { avatar, initSheet, openSheet, transition } from "./ui.js";
import feedView from "./views/for-you.js";
import browseView from "./views/browse.js";
import crewsView from "./views/crews.js";
import hostView from "./views/host.js";
import eventView from "./views/event.js";
import meView from "./views/me.js";
import privacyView from "./views/privacy.js";
import checkinView from "./views/checkin.js";

/** View registry, keyed by route name. */
const VIEWS = {
  feed: feedView,
  browse: browseView,
  crews: crewsView,
  host: hostView,
  event: eventView,
  me: meView,
  privacy: privacyView,
  checkin: checkinView,
};

/** Routes that highlight a tab (others keep the previous highlight off). */
const TAB_FOR = { feed: "feed", browse: "browse", host: "host", crews: "crews", me: "me", privacy: "me" };

/** Routes that hide the tab bar (the event page has its own action bar). */
const NO_TABBAR = new Set(["event", "checkin"]);

/**
 * In-memory session state.
 * `me` is the `Me` object from `GET /api/me` (mock: `me.json`) or `null`
 * when logged out. `openLogin`/`logout` are exposed so views can trigger them.
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
  /** Opens the demo sign-in sheet. */
  openLogin: () => {},
  /** Signs out. */
  logout: () => {},
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

/**
 * Per-browser hint that a session cookie probably exists. The `uid` cookie is HTTP-only,
 * so without this every signed-out page load would ask `/api/me` and log a 401. The server
 * stays the authority: a stale hint just costs one 401, which clears it.
 */
const SESSION_HINT = "campus-hangouts.signed-in";

function setSessionHint(on) {
  try {
    if (on) localStorage.setItem(SESSION_HINT, "1");
    else localStorage.removeItem(SESSION_HINT);
  } catch {
    /* storage blocked (private mode): fall back to always asking */
  }
}

function hasSessionHint() {
  try {
    return localStorage.getItem(SESSION_HINT) === "1";
  } catch {
    return true;
  }
}

/** Loads the session; a 401 (or an API that isn't up yet) means logged out. */
async function loadSession() {
  if (!isMock() && !hasSessionHint()) {
    state.me = null;
    return;
  }
  try {
    state.me = await api.get("/api/me");
    setSessionHint(true);
  } catch {
    state.me = null;
    setSessionHint(false);
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
  setSessionHint(false);
  renderHeader();
  renderRoute();
}

/** Renders the top bar's session area: avatar (links to Me) or a Sign in button. */
function renderHeader() {
  const session = document.getElementById("session");
  if (!session) return;
  if (!state.me) {
    session.innerHTML = `<button type="button" class="btn btn--soft" id="sign-in-btn">Sign in</button>`;
    session.querySelector("#sign-in-btn").addEventListener("click", openLogin);
    return;
  }
  const me = state.me.user;
  session.innerHTML = `<a class="session__me" href="#/me" aria-label="${esc(me.display_name)} — your profile">${avatar(me, "avatar--md")}</a>`;
}

/** Highlights the tab for the current route and hides the bar where needed. */
function setNavActive() {
  const active = TAB_FOR[state.route];
  document.querySelectorAll("#tabbar .tab").forEach((link) => {
    const on = link.dataset.nav === active;
    link.classList.toggle("is-active", on);
    if (on) link.setAttribute("aria-current", "page");
    else link.removeAttribute("aria-current");
  });
  document.body.classList.toggle("no-tabbar", NO_TABBAR.has(state.route));
  document.body.dataset.route = state.route;
}

let firstRender = true;
let rendering = false;
let rerender = false;

/** Renders the current route; calls made mid-render are coalesced into one follow-up render. */
async function renderRoute() {
  if (rendering) {
    rerender = true;
    return;
  }
  rendering = true;
  try {
    do {
      rerender = false;
      await renderOnce();
    } while (rerender);
  } finally {
    rendering = false;
  }
}

/** Renders the current route's view into `#view`, with an error fallback. */
async function renderOnce() {
  const { name, params, known } = parseHash();
  state.route = name;
  state.params = params;

  // Unknown hash: send the user home; the hashchange re-renders the feed.
  if (!known) {
    window.location.replace("#/");
    return;
  }

  const root = document.getElementById("view");
  if (!root) return;
  const view = VIEWS[name];

  const run = firstRender ? (fn) => fn() : transition; // no animation on the first paint
  firstRender = false;
  await run(async () => {
    setNavActive();
    window.scrollTo(0, 0);
    try {
      await view.render(root, { state, params });
      // Accessibility: move focus to the new view's heading and update the title.
      const h1 = root.querySelector("h1");
      if (h1) {
        h1.setAttribute("tabindex", "-1");
        h1.focus({ preventScroll: true });
        document.title = `${h1.textContent} · Campus Hangouts`;
      }
    } catch (err) {
      root.innerHTML = `
        <div class="empty">
          <h2>Something went wrong</h2>
          <p>${esc(err && err.message ? err.message : "Unknown error")}</p>
          <button type="button" class="btn btn--soft" data-retry>Try again</button>
        </div>`;
      root.querySelector("[data-retry]").addEventListener("click", renderRoute);
    }
  });
}

/** Opens the demo sign-in sheet listing the selectable demo students. */
async function openLogin() {
  const { close } = openSheet("Sign in (demo)", `<p class="muted">Loading demo students…</p>`);
  let users;
  try {
    users = await api.get("/api/demo-users");
  } catch (err) {
    openSheet("Sign in (demo)", `<p class="form-error" role="alert">${esc(err.message || "Could not load demo students.")}</p>`);
    return;
  }
  openSheet(
    "Sign in (demo)",
    users.length
      ? `<p class="muted">Pick a demo student to explore as.</p><ul class="people">${users
          .map(
            (u) => `<li><button type="button" class="person person--btn" data-id="${esc(u.id)}">
              ${avatar(u, "avatar--md")}<span class="person__meta"><strong>${esc(u.display_name)}</strong>
              <span class="muted">${esc(u.bio || "")}</span></span></button></li>`,
          )
          .join("")}</ul>`
      : `<p class="muted">No demo students available.</p>`,
    (body) =>
      body.querySelectorAll("[data-id]").forEach((b) =>
        b.addEventListener("click", async () => {
          try {
            state.me = await api.post("/api/login", { user_id: b.dataset.id });
            setSessionHint(true);
            close();
            renderHeader();
            renderRoute();
          } catch (err) {
            body.insertAdjacentHTML("afterbegin", `<p class="form-error" role="alert">${esc(err.message || "Sign-in failed")}</p>`);
          }
        }),
      ),
  );
}

/** Boots the shell: mode pill, session, listeners, first render. */
async function boot() {
  state.openLogin = openLogin;
  state.logout = logout;
  initSheet();
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
