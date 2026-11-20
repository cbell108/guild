/**
 * "Crews" view (B6): your crews (create, open, add/remove members) and your
 * friends (requests, accept, search to add). Both are 🔒 endpoints.
 */
import { api } from "../api.js";
import { esc, initials } from "../util.js";

/** Small person row with an optional action button. */
function personRow(user, actionHtml = "") {
  return `<li class="person">
    <span class="avatar" aria-hidden="true">${esc(initials(user.display_name))}</span>
    <span class="person__meta"><strong>${esc(user.display_name)}</strong>
      <span class="muted">${esc(user.bio || "")}</span></span>${actionHtml}</li>`;
}

/** Debounced user search that renders results into `out` with an action per user. */
function bindSearch(input, out, actionFor, onPick) {
  let timer;
  input.addEventListener("input", () => {
    clearTimeout(timer);
    timer = setTimeout(async () => {
      const q = input.value.trim();
      if (!q) {
        out.innerHTML = "";
        return;
      }
      try {
        const users = await api.get(`/api/users?q=${encodeURIComponent(q)}`);
        out.innerHTML = users.length
          ? users.map((u) => personRow(u, actionFor(u))).join("")
          : `<li class="muted">No students found.</li>`;
        out.querySelectorAll("[data-pick]").forEach((b) =>
          b.addEventListener("click", () => onPick(b.dataset.pick)),
        );
      } catch (err) {
        out.innerHTML = `<li class="form-error">${esc(err.message)}</li>`;
      }
    }, 250);
  });
}

/** Renders the "Crews" view. */
export default {
  name: "crews",
  async render(root, { state }) {
    const head = `
      <header class="view-head">
        <h1>Crews</h1>
        <p class="lede">Your regular groups and friends. Crew details are visible to members only.</p>
      </header>`;
    if (!state.me) {
      root.innerHTML = `${head}<section class="empty-state card"><h2>Sign in to see your crews</h2>
        <p>Use the Sign in button (top right) to pick a demo student.</p></section>`;
      return;
    }
    const [crews, social] = await Promise.all([api.get("/api/crews"), api.get("/api/friends")]);
    let open = null; // CrewDetail currently expanded
    const myId = state.me.user.id;

    const paint = () => {
      root.innerHTML = `${head}
        <p id="c-error" class="form-error" role="alert" hidden></p>
        <section class="card panel" aria-labelledby="crews-h">
          <h2 id="crews-h">Your crews</h2>
          ${
            crews.length
              ? `<ul class="people">${crews
                  .map(
                    (c) => `<li class="crew-row"><div><strong>${esc(c.name)}</strong>
                      <span class="muted">${esc(c.description)}</span></div>
                      <button type="button" class="btn btn--ghost btn--sm" data-open="${esc(c.id)}">Members</button></li>`,
                  )
                  .join("")}</ul>`
              : `<p class="muted">You're not in a crew yet — start one below.</p>`
          }
          <div id="crew-detail" class="crew-detail" aria-live="polite"></div>
          <form id="crew-new" class="field-row">
            <label class="field"><span>New crew name</span><input name="name" required maxlength="60" /></label>
            <label class="field"><span>Description</span><input name="description" /></label>
            <button class="btn btn--primary btn--sm" type="submit">Create crew</button>
          </form>
        </section>

        <section class="card panel" aria-labelledby="friends-h">
          <h2 id="friends-h">Friends</h2>
          ${
            social.incoming.length
              ? `<h3>Requests</h3><ul class="people">${social.incoming
                  .map((u) => personRow(u, `<button type="button" class="btn btn--primary btn--sm" data-accept="${esc(u.id)}">Accept</button>`))
                  .join("")}</ul>`
              : ""
          }
          <ul class="people">${
            social.friends.length
              ? social.friends.map((u) => personRow(u)).join("")
              : `<li class="muted">No friends yet.</li>`
          }</ul>
          ${
            social.outgoing.length
              ? `<p class="muted">Pending: ${social.outgoing.map((u) => esc(u.display_name)).join(", ")}</p>`
              : ""
          }
          <label class="field"><span>Find students to add</span><input type="search" id="f-search" placeholder="Name" /></label>
          <ul class="people" id="f-results"></ul>
        </section>`;
      bind();
      if (open) paintDetail();
    };

    const fail = (err) => {
      const el = root.querySelector("#c-error");
      el.textContent = err && err.message ? err.message : "Something went wrong";
      el.hidden = false;
    };

    const paintDetail = () => {
      const box = root.querySelector("#crew-detail");
      const owner = open.members.some((m) => m.user.id === myId && m.role === "owner");
      box.innerHTML = `<h3>${esc(open.name)}</h3>
        <ul class="people">${open.members
          .map((m) =>
            personRow(
              m.user,
              `<span class="chip">${esc(m.role)}</span>${
                owner || m.user.id === myId
                  ? `<button type="button" class="btn btn--ghost btn--sm" data-remove="${esc(m.user.id)}">${m.user.id === myId ? "Leave" : "Remove"}</button>`
                  : ""
              }`,
            ),
          )
          .join("")}</ul>
        ${
          owner
            ? `<label class="field"><span>Add a member</span><input type="search" id="m-search" placeholder="Name" /></label>
               <ul class="people" id="m-results"></ul>`
            : ""
        }`;
      box.querySelectorAll("[data-remove]").forEach((b) =>
        b.addEventListener("click", async () => {
          try {
            const id = b.dataset.remove;
            open = await api.del(`/api/crews/${open.id}/members/${id}`);
            if (id === myId) {
              open = null;
              const fresh = await api.get("/api/crews");
              crews.splice(0, crews.length, ...fresh);
            }
            paint();
          } catch (err) {
            fail(err);
          }
        }),
      );
      const search = box.querySelector("#m-search");
      if (search) {
        bindSearch(
          search,
          box.querySelector("#m-results"),
          (u) => `<button type="button" class="btn btn--ghost btn--sm" data-pick="${esc(u.id)}">Add</button>`,
          async (id) => {
            try {
              open = await api.post(`/api/crews/${open.id}/members`, { user_id: id });
              paint();
            } catch (err) {
              fail(err);
            }
          },
        );
      }
    };

    const bind = () => {
      root.querySelectorAll("[data-open]").forEach((b) =>
        b.addEventListener("click", async () => {
          try {
            open = await api.get(`/api/crews/${b.dataset.open}`);
            paintDetail();
          } catch (err) {
            fail(err);
          }
        }),
      );
      root.querySelector("#crew-new").addEventListener("submit", async (e) => {
        e.preventDefault();
        const form = e.target;
        try {
          open = await api.post("/api/crews", {
            name: form.elements.name.value.trim(),
            description: form.elements.description.value.trim(),
          });
          const fresh = await api.get("/api/crews");
          crews.splice(0, crews.length, ...fresh);
          paint();
        } catch (err) {
          fail(err);
        }
      });
      root.querySelectorAll("[data-accept]").forEach((b) =>
        b.addEventListener("click", async () => {
          try {
            Object.assign(social, await api.post(`/api/friends/${b.dataset.accept}/accept`));
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
            : `<button type="button" class="btn btn--ghost btn--sm" data-pick="${esc(u.id)}">Add friend</button>`,
        async (id) => {
          try {
            Object.assign(social, await api.post("/api/friends", { user_id: id }));
            paint();
          } catch (err) {
            fail(err);
          }
        },
      );
    };

    paint();
  },
};
