/** Shared person-row and user-search helpers (Crews + Me). */
import { api } from "./api.js";
import { esc } from "./util.js";
import { avatar } from "./ui.js";

/** Person row with an optional trailing action (HTML). */
export function personRow(user, actionHtml = "") {
  return `<li class="person">${avatar(user, "avatar--md")}
    <span class="person__meta"><strong>${esc(user.display_name)}</strong>
    <span class="muted">${esc(user.bio || "")}</span></span>${actionHtml}</li>`;
}

/** Debounced user search rendering results into `out` with an action per user. */
export function bindSearch(input, out, actionFor, onPick) {
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
