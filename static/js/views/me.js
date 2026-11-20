/**
 * "Me" view: profile, friends (requests, list, "Add friends" sheet), account
 * rows (privacy, sign out). Signed out, it offers the demo sign-in.
 */
import { api } from "../api.js";
import { esc } from "../util.js";
import { avatar, openSheet, toast } from "../ui.js";
import { bindSearch } from "../people.js";

/** One friend/request row. */
function friendRow(u, action = "") {
  return `<li class="row">${avatar(u, "avatar--md")}<span class="row__main"><strong>${esc(u.display_name)}</strong>
    <span class="muted">${esc(u.bio || "")}</span></span>${action}</li>`;
}

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

        ${
          social.incoming.length
            ? `<section class="stack" aria-labelledby="req-h"><div class="section-head"><h2 id="req-h">Friend requests</h2></div>
                <ul class="group">${social.incoming
                  .map((u) => friendRow(u, `<button type="button" class="btn btn--primary btn--sm" data-accept="${esc(u.id)}">Accept</button>`))
                  .join("")}</ul></section>`
            : ""
        }

        <section class="stack" aria-labelledby="friends-h">
          <div class="section-head"><h2 id="friends-h">Friends</h2>
            ${social.friends.length ? `<button type="button" class="btn btn--soft btn--sm" data-add-friend>Add friends</button>` : ""}</div>
          ${
            social.friends.length
              ? `<ul class="group">${social.friends.map((u) => friendRow(u)).join("")}</ul>`
              : `<div class="empty"><span class="empty__emoji" aria-hidden="true">🧑‍🤝‍🧑</span><h2>No friends yet</h2>
                 <p>Add friends to see what they're going to.</p>
                 <button type="button" class="btn btn--primary" data-add-friend>Add friends</button></div>`
          }
          ${social.outgoing.length ? `<p class="muted">Pending: ${social.outgoing.map((u) => esc(u.display_name)).join(", ")}</p>` : ""}
        </section>

        <nav class="group" aria-label="Account">
          <a class="row" href="#/privacy"><span class="row__icon"><svg class="icon" aria-hidden="true"><use href="#i-lock" /></svg></span>
            <span class="row__main"><strong>Privacy</strong><span class="muted">Who sees your plans, your data</span></span>
            <svg class="icon menu__chev" aria-hidden="true"><use href="#i-chevron" /></svg></a>
          <button type="button" class="row row--danger" id="sign-out"><span class="row__icon"><svg class="icon" aria-hidden="true"><use href="#i-logout" /></svg></span>
            <span class="row__main"><strong>Sign out</strong></span></button>
        </nav>`;
      bind();
    };

    /** Add-friends bottom sheet: search, then Add. */
    const addFriendSheet = () => {
      openSheet(
        "Add friends",
        `<label class="field"><span class="visually-hidden">Search by name</span><input type="search" id="f-search" placeholder="Search students by name" autocomplete="off" /></label>
         <ul class="people" id="f-results"><li class="muted">Type a name to find people.</li></ul>`,
        (body, close) => {
          bindSearch(
            body.querySelector("#f-search"),
            body.querySelector("#f-results"),
            (u) =>
              social.friends.some((f) => f.id === u.id)
                ? `<span class="chip">Friends</span>`
                : social.outgoing.some((f) => f.id === u.id)
                  ? `<span class="chip">Requested</span>`
                  : `<button type="button" class="btn btn--soft btn--sm" data-pick="${esc(u.id)}">Add</button>`,
            async (id) => {
              try {
                Object.assign(social, await api.post("/api/friends", { user_id: id }));
                close();
                toast("Request sent");
                paint();
              } catch (err) {
                toast(err.message || "Couldn't send that request");
              }
            },
          );
          body.querySelector("#f-search").focus();
        },
      );
    };

    const bind = () => {
      root.querySelectorAll("[data-accept]").forEach((b) =>
        b.addEventListener("click", async () => {
          try {
            Object.assign(social, await api.post(`/api/friends/${b.dataset.accept}/accept`));
            toast("Friend added 🎉");
            paint();
          } catch (err) {
            toast(err.message || "Couldn't accept that");
          }
        }),
      );
      root.querySelectorAll("[data-add-friend]").forEach((b) => b.addEventListener("click", addFriendSheet));
      root.querySelector("#sign-out").addEventListener("click", () => state.logout());
    };

    paint();
  },
};
