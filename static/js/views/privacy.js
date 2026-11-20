/**
 * "Privacy" view (B7): attendance visibility, buddy matching, personalization,
 * and deleting activity history. Reached from the user chip in the header.
 */
import { api } from "../api.js";

/** Renders the privacy settings view. */
export default {
  name: "privacy",
  async render(root, { state }) {
    const head = `
      <a class="back-link" href="#/me"><svg class="icon" aria-hidden="true"><use href="#i-back" /></svg> Me</a>
      <header class="view-head">
        <h1>Privacy</h1>
        <p class="lede">You decide what friends can see. There are no public attendee lists or follower counts.</p>
      </header>`;
    if (!state.me) {
      root.innerHTML = `${head}<section class="empty"><h2>Sign in first</h2>
        <p>Use the Sign in button (top right) to pick a demo student.</p></section>`;
      return;
    }
    let p = await api.get("/api/me/privacy");
    root.innerHTML = `${head}
      <form class="panel panel--stretch" id="privacy-form">
        <label class="field"><span>Who can see which events I'm going to</span>
          <select name="attendance">
            <option value="friends">My friends</option>
            <option value="private">Only me</option>
          </select></label>
        <label class="toggle"><input type="checkbox" name="buddy_matching" /> <span>Let others find me as a buddy match</span></label>
        <label class="toggle"><input type="checkbox" name="personalize" /> <span>Personalize my feed from my activity</span></label>
        <p id="p-msg" class="muted" role="status" aria-live="polite"></p>
      </form>
      <section class="panel panel--stretch">
        <h2>Delete my activity</h2>
        <p class="muted">Removes your intents, check-ins, buddy requests and availability. Your account, friends and crews stay.</p>
        <button type="button" class="btn btn--ghost btn--sm" id="del-history">Delete my history</button>
        <p id="d-msg" class="muted" role="status" aria-live="polite"></p>
      </section>`;

    const form = root.querySelector("#privacy-form");
    const msg = root.querySelector("#p-msg");
    const paint = () => {
      form.elements.attendance.value = p.attendance;
      form.elements.buddy_matching.checked = p.buddy_matching;
      form.elements.personalize.checked = p.personalize;
    };
    paint();
    form.addEventListener("change", async () => {
      const next = {
        attendance: form.elements.attendance.value,
        buddy_matching: form.elements.buddy_matching.checked,
        personalize: form.elements.personalize.checked,
      };
      msg.textContent = "Saving…";
      try {
        p = await api.put("/api/me/privacy", next);
        msg.textContent = "Saved.";
      } catch (err) {
        msg.textContent = err.message || "Could not save.";
      }
      paint();
    });

    const del = root.querySelector("#del-history");
    const dmsg = root.querySelector("#d-msg");
    del.addEventListener("click", async () => {
      if (!window.confirm("Delete your activity history? This cannot be undone.")) return;
      try {
        await api.del("/api/me/history");
        dmsg.textContent = "Your history was deleted.";
      } catch (err) {
        dmsg.textContent = err.message || "Could not delete.";
      }
    });
  },
};
