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
    const head = `<header class="view-head"><h1>Crews</h1></header>`;
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
      root.innerHTML = `<header class="section-head"><div class="view-head"><h1>Crews</h1>
          <p class="lede">Your regular groups.</p></div>
          ${crews.length ? `<button type="button" class="btn btn--soft btn--sm" data-new-crew>New crew</button>` : ""}</header>
        ${
          crews.length
            ? `<ul class="group">${crews
                .map((c) => {
                  const d = details.get(c.id);
                  const members = d ? d.members.map((m) => m.user) : [];
                  return `<li><button type="button" class="row" data-open="${esc(c.id)}">
                    <span class="row__main"><strong>${esc(c.name)}</strong><span class="muted">${esc(c.description)}</span></span>
                    ${avatarStack(members, Math.max(0, members.length - 3))}
                    <svg class="icon menu__chev" aria-hidden="true"><use href="#i-chevron" /></svg></button></li>`;
                })
                .join("")}</ul>`
            : `<div class="empty"><span class="empty__emoji" aria-hidden="true">🫶</span><h2>No crews yet</h2>
               <p>A crew is your regular group — see when everyone's free and plan hangouts together.</p>
               <button type="button" class="btn btn--primary" data-new-crew>Start a crew</button></div>`
        }`;
      root.querySelectorAll("[data-open]").forEach((b) => b.addEventListener("click", () => showCrew(b.dataset.open)));
      root.querySelectorAll("[data-new-crew]").forEach((b) => b.addEventListener("click", newCrewSheet));
    };

    /** "Start a crew" bottom sheet. */
    const newCrewSheet = () => {
      openSheet(
        "Start a crew",
        `<form id="crew-new" class="stack">
          <label class="field"><span>Name</span><input name="name" required maxlength="60" placeholder="Smash Saturdays" /></label>
          <label class="field"><span>Description</span><input name="description" placeholder="What's it about?" /></label>
          <button class="btn btn--primary btn--block" type="submit">Create crew</button></form>`,
        (body, close) => {
          body.querySelector("[name=name]").focus();
          body.querySelector("#crew-new").addEventListener("submit", async (e) => {
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
              close();
              toast("Crew created 🎉");
              paint();
            } catch (err) {
              toast(err.message || "Could not create the crew");
            }
          });
        },
      );
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
        ${owner ? `<button type="button" class="btn btn--soft btn--block" data-add-member>Add a member</button>` : ""}`,
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
          body.querySelector("[data-add-member]")?.addEventListener("click", () => addMemberSheet(id));
        },
      );
    };

    /** "Add a member" bottom sheet (owners only). */
    const addMemberSheet = (id) => {
      openSheet(
        "Add a member",
        `<label class="field"><span class="visually-hidden">Search by name</span><input type="search" id="m-search" placeholder="Search students by name" autocomplete="off" /></label>
         <ul class="people" id="m-results"><li class="muted">Type a name to find people.</li></ul>`,
        (body, close) => {
          body.querySelector("#m-search").focus();
          bindSearch(
            body.querySelector("#m-search"),
            body.querySelector("#m-results"),
            (u) =>
              details.get(id).members.some((m) => m.user.id === u.id)
                ? `<span class="chip">In crew</span>`
                : `<button type="button" class="btn btn--soft btn--sm" data-pick="${esc(u.id)}">Add</button>`,
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
        },
      );
    };

    paint();
  },
};
