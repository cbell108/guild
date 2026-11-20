/**
 * "Me" view: profile, friends (requests, accept, search to add), privacy link
 * and sign out. Signed out, it offers the demo sign-in.
 */
import { api } from "../api.js";
import { esc } from "../util.js";
import { avatar, toast } from "../ui.js";
import { personRow, bindSearch } from "../people.js";

/** Renders the "Me" view. */
export default {
  name: "me",
  async render(root, { state }) {
    if (!state.me) {
      root.innerHTML = `<header class="view-head"><h1>Me</h1></header>
        <section class="empty"><span class="empty__emoji" aria-hidden="true">👋</span>
        <h2>Sign in to get started</h2>
        <p>Pick a demo student to see your picks, friends and crews.</p>
        <button type="button" class="btn btn--primary" data-signin>Sign in</button></section>`;
      root.querySelector("[data-signin]").addEventListener("click", () => state.openLogin());
      return;
    }
    const user = state.me.user;
    const social = await api.get("/api/friends");

    const paint = () => {
      root.innerHTML = `
        <header class="profile">
          ${avatar(user, "avatar--lg")}
          <div><h1>${esc(user.display_name)}</h1><p class="muted">${esc(user.bio || "")}</p>
          <p class="profile__tags">${(state.me.interests || []).map((t) => `<span class="chip">#${esc(t)}</span>`).join("")}</p></div>
        </header>
        <p id="me-error" class="form-error" role="alert" hidden></p>

        <section class="panel panel--stretch" aria-labelledby="friends-h">
          <h2 id="friends-h">Friends</h2>
          ${
            social.incoming.length
              ? `<h3 class="sub">Requests</h3><ul class="people">${social.incoming
                  .map((u) => personRow(u, `<button type="button" class="btn btn--primary btn--sm" data-accept="${esc(u.id)}">Accept</button>`))
                  .join("")}</ul>`
              : ""
          }
          <ul class="people">${
            social.friends.length ? social.friends.map((u) => personRow(u)).join("") : `<li class="muted">No friends yet.</li>`
          }</ul>
          ${social.outgoing.length ? `<p class="muted">Pending: ${social.outgoing.map((u) => esc(u.display_name)).join(", ")}</p>` : ""}
          <label class="field"><span>Find students to add</span><input type="search" id="f-search" placeholder="Search by name" /></label>
          <ul class="people" id="f-results"></ul>
        </section>

        <nav class="panel panel--stretch menu" aria-label="Account">
          <a class="menu__item" href="#/privacy"><svg class="icon" aria-hidden="true"><use href="#i-lock" /></svg><span>Privacy settings</span><svg class="icon menu__chev" aria-hidden="true"><use href="#i-chevron" /></svg></a>
          <button type="button" class="menu__item" id="sign-out"><svg class="icon" aria-hidden="true"><use href="#i-logout" /></svg><span>Sign out</span></button>
        </nav>`;
      bind();
    };

    const fail = (err) => {
      const el = root.querySelector("#me-error");
      el.textContent = err && err.message ? err.message : "Something went wrong";
      el.hidden = false;
    };

    const bind = () => {
      root.querySelectorAll("[data-accept]").forEach((b) =>
        b.addEventListener("click", async () => {
          try {
            Object.assign(social, await api.post(`/api/friends/${b.dataset.accept}/accept`));
            toast("Friend added");
            paint();
          } catch (err) {
            fail(err);
          }
        }),
      );
      bindSearch(
        root.querySelector("#f-search"),
        root.querySelector("#f-results"),
        (u) =>
          social.friends.some((f) => f.id === u.id)
            ? `<span class="chip">Friends</span>`
            : `<button type="button" class="btn btn--ghost btn--sm" data-pick="${esc(u.id)}">Add</button>`,
        async (id) => {
          try {
            Object.assign(social, await api.post("/api/friends", { user_id: id }));
            toast("Request sent");
            paint();
          } catch (err) {
            fail(err);
          }
        },
      );
      root.querySelector("#sign-out").addEventListener("click", () => state.logout());
    };

    paint();
  },
};
