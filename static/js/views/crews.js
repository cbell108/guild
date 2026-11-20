/**
 * "Crews" view: your crews as soft cards with member avatars; tap one to see
 * and manage its members in a bottom-sheet-free inline panel, or start a new crew.
 */
import { api } from "../api.js";
import { esc } from "../util.js";
import { avatarStack, toast, openSheet } from "../ui.js";
import { personRow, bindSearch } from "../people.js";

/** Renders the "Crews" view. */
export default {
  name: "crews",
  async render(root, { state }) {
    const head = `<header class="view-head"><h1>Crews</h1>
      <p class="lede">Your regular groups. Crew details are visible to members only.</p></header>`;
    if (!state.me) {
      root.innerHTML = `${head}<section class="empty"><span class="empty__emoji" aria-hidden="true">👥</span>
        <h2>Sign in to see your crews</h2><button type="button" class="btn btn--primary" data-signin>Sign in</button></section>`;
      root.querySelector("[data-signin]").addEventListener("click", () => state.openLogin());
      return;
    }
    const myId = state.me.user.id;
    let crews = await api.get("/api/crews");
    // Member avatars need the detail call; fetch them all (crews are few).
    const details = new Map();
    const loadDetails = async () => {
      await Promise.all(
        crews.map(async (c) => {
          try {
            details.set(c.id, await api.get(`/api/crews/${c.id}`));
          } catch {
            /* members-only: skip avatars */
          }
        }),
      );
    };
    await loadDetails();

    const paint = () => {
      root.innerHTML = `${head}
        <ul class="crew-list">${
          crews.length
            ? crews
                .map((c) => {
                  const d = details.get(c.id);
                  const members = d ? d.members.map((m) => m.user) : [];
                  return `<li><button type="button" class="crew-card" data-open="${esc(c.id)}">
                    <span class="crew-card__text"><strong>${esc(c.name)}</strong><span class="muted">${esc(c.description)}</span></span>
                    ${avatarStack(members, Math.max(0, members.length - 3))}
                    <svg class="icon menu__chev" aria-hidden="true"><use href="#i-chevron" /></svg></button></li>`;
                })
                .join("")
            : `<li class="empty"><span class="empty__emoji" aria-hidden="true">🫶</span><h2>No crews yet</h2><p>Start one for your regular group.</p></li>`
        }</ul>
        <form id="crew-new" class="panel panel--stretch">
          <h2>Start a crew</h2>
          <label class="field"><span>Name</span><input name="name" required maxlength="60" /></label>
          <label class="field"><span>Description</span><input name="description" /></label>
          <button class="btn btn--primary" type="submit">Create crew</button>
        </form>`;
      root.querySelectorAll("[data-open]").forEach((b) => b.addEventListener("click", () => showCrew(b.dataset.open)));
      root.querySelector("#crew-new").addEventListener("submit", async (e) => {
        e.preventDefault();
        const form = e.target;
        try {
          const created = await api.post("/api/crews", {
            name: form.elements.name.value.trim(),
            description: form.elements.description.value.trim(),
          });
          crews = await api.get("/api/crews");
          details.set(created.id, created);
          await loadDetails();
          toast("Crew created");
          paint();
        } catch (err) {
          toast(err.message || "Could not create the crew");
        }
      });
    };

    /** Members sheet for one crew (add/remove/leave). */
    const showCrew = (id) => {
      const d = details.get(id);
      if (!d) return;
      const owner = d.members.some((m) => m.user.id === myId && m.role === "owner");
      openSheet(
        d.name,
        `<ul class="people">${d.members
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
            ? `<label class="field"><span>Add a member</span><input type="search" id="m-search" placeholder="Search by name" /></label>
               <ul class="people" id="m-results"></ul>`
            : ""
        }`,
        (body, close) => {
          body.querySelectorAll("[data-remove]").forEach((b) =>
            b.addEventListener("click", async () => {
              try {
                const uid = b.dataset.remove;
                const next = await api.del(`/api/crews/${id}/members/${uid}`);
                if (uid === myId) {
                  crews = await api.get("/api/crews");
                  details.delete(id);
                } else {
                  details.set(id, next);
                }
                close();
                toast(uid === myId ? "You left the crew" : "Member removed");
                paint();
              } catch (err) {
                toast(err.message || "Something went wrong");
              }
            }),
          );
          const search = body.querySelector("#m-search");
          if (search) {
            bindSearch(
              search,
              body.querySelector("#m-results"),
              (u) => `<button type="button" class="btn btn--ghost btn--sm" data-pick="${esc(u.id)}">Add</button>`,
              async (uid) => {
                try {
                  details.set(id, await api.post(`/api/crews/${id}/members`, { user_id: uid }));
                  close();
                  toast("Member added");
                  paint();
                } catch (err) {
                  toast(err.message || "Something went wrong");
                }
              },
            );
          }
        },
      );
    };

    paint();
  },
};
