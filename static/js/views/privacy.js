/**
 * "Privacy" view: attendance visibility (segmented), buddy matching and
 * personalization switches, and a confirm sheet for deleting activity history.
 */
import { api } from "../api.js";
import { esc } from "../util.js";
import { openSheet, toast } from "../ui.js";

/** One switch row. */
const switchRow = (name, icon, title, hint, checked) => `<li class="row">
  <span class="row__icon"><svg class="icon" aria-hidden="true"><use href="#i-${icon}" /></svg></span>
  <label class="row__main" for="sw-${name}"><strong>${esc(title)}</strong><span class="muted">${esc(hint)}</span></label>
  <span class="switch"><input type="checkbox" id="sw-${name}" name="${name}" ${checked ? "checked" : ""} /><span class="switch__track"></span></span></li>`;

/** Renders the privacy settings view. */
export default {
  name: "privacy",
  async render(root, { state }) {
    const head = `
      <a class="back-link" href="#/me"><svg class="icon" aria-hidden="true"><use href="#i-back" /></svg> Me</a>
      <header class="view-head"><h1>Privacy</h1>
        <p class="lede">You decide what friends can see. No public attendee lists or follower counts.</p></header>`;
    if (!state.me) {
      root.innerHTML = `${head}<section class="empty"><span class="empty__emoji" aria-hidden="true">🔒</span><h2>Sign in first</h2>
        <button type="button" class="btn btn--primary" data-signin>Sign in</button></section>`;
      root.querySelector("[data-signin]").addEventListener("click", () => state.openLogin());
      return;
    }
    let p = await api.get("/api/me/privacy");

    const paint = () => {
      root.innerHTML = `${head}
        <section class="stack" aria-labelledby="vis-h">
          <h2 id="vis-h" class="sub">Who can see which events you're going to</h2>
          <div class="segmented" role="group" aria-labelledby="vis-h">
            <button type="button" data-att="friends" aria-pressed="${p.attendance === "friends"}">My friends</button>
            <button type="button" data-att="private" aria-pressed="${p.attendance === "private"}">Only me</button>
          </div>
        </section>
        <ul class="group" aria-label="Preferences">
          ${switchRow("buddy_matching", "users", "Buddy matching", "Let others find you as a buddy for events", p.buddy_matching)}
          ${switchRow("personalize", "sparkles", "Personalized feed", "Use your activity to pick events for you", p.personalize)}
        </ul>
        <div class="group"><button type="button" class="row row--danger" id="del-history">
          <span class="row__icon"><svg class="icon" aria-hidden="true"><use href="#i-close" /></svg></span>
          <span class="row__main"><strong>Delete my activity</strong><span class="muted">Intents, check-ins, buddy requests, availability</span></span></button></div>`;
      bind();
    };

    const save = async (next) => {
      const before = p;
      p = next;
      paint(); // optimistic
      try {
        p = await api.put("/api/me/privacy", next);
        toast("Saved");
      } catch (err) {
        p = before;
        toast(err.message || "Couldn't save that");
      }
      paint();
    };

    const bind = () => {
      root.querySelectorAll("[data-att]").forEach((b) =>
        b.addEventListener("click", () => b.dataset.att !== p.attendance && save({ ...p, attendance: b.dataset.att })),
      );
      root.querySelectorAll(".switch input").forEach((i) =>
        i.addEventListener("change", () => save({ ...p, [i.name]: i.checked })),
      );
      root.querySelector("#del-history").addEventListener("click", () =>
        openSheet(
          "Delete your activity?",
          `<p class="muted">This removes your intents, check-ins, buddy requests and availability. Your account, friends and crews stay. It can't be undone.</p>
           <div class="sheet-actions"><button type="button" class="btn btn--ghost" data-cancel>Cancel</button>
           <button type="button" class="btn btn--danger" data-confirm>Delete</button></div>`,
          (body, close) => {
            body.querySelector("[data-cancel]").addEventListener("click", close);
            body.querySelector("[data-confirm]").addEventListener("click", async () => {
              try {
                await api.del("/api/me/history");
                close();
                toast("Your activity was deleted");
              } catch (err) {
                toast(err.message || "Couldn't delete that");
              }
            });
          },
        ),
      );
    };

    paint();
  },
};
