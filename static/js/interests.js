/**
 * Interest picker shared by first-run onboarding and "Edit interests".
 * The groups mirror the interest vocabulary the seed data and KSU ingest use
 * (`seed::interest_vocabulary`); tags outside it that a user already has are kept
 * under "More" so editing never drops them.
 */
import { api } from "./api.js";
import { esc } from "./util.js";
import { openSheet, toast } from "./ui.js";

/** Minimum interests a user must pick. */
export const MIN_INTERESTS = 3;

/** [emoji, label, tags] */
export const GROUPS = [
  ["🎮", "Gaming", ["gaming", "smash-bros", "esports", "nintendo", "mario-kart"]],
  ["🎲", "Tabletop", ["board-games", "tabletop", "dnd", "card-games", "puzzles"]],
  ["💻", "Tech", ["coding", "ai", "hackathons", "robotics", "cybersecurity"]],
  ["🏀", "Sports & fitness", ["sports", "basketball", "soccer", "fitness", "intramurals", "volleyball"]],
  ["🎨", "Arts", ["art", "drawing", "photography", "film", "design", "theater"]],
  ["🎵", "Music", ["music", "concerts", "jazz", "open-mic", "guitar"]],
  ["🍕", "Food", ["food", "free-food", "cooking", "coffee", "baking"]],
  ["🌲", "Outdoors", ["outdoors", "hiking", "climbing", "running", "nature"]],
  ["💼", "Career", ["career", "networking", "resume", "internships", "leadership"]],
  ["🎌", "Anime & pop", ["anime", "manga", "cosplay", "k-pop", "japanese"]],
  ["🎓", "Learning", ["research", "talks", "workshops"]],
];

/** "smash-bros" → "Smash bros". */
const SPECIAL = { ai: "AI", dnd: "D&D", "k-pop": "K-pop", esports: "Esports" };
const label = (tag) => {
  if (SPECIAL[tag]) return SPECIAL[tag];
  const t = tag.replace(/-/g, " ");
  return t.charAt(0).toUpperCase() + t.slice(1);
};

/** Chips grouped by category; `selected` is a Set of tags. */
export function pickerHtml(selected) {
  const known = new Set(GROUPS.flatMap((g) => g[2]));
  const extra = [...selected].filter((t) => !known.has(t));
  const groups = extra.length ? [...GROUPS, ["✨", "More", extra]] : GROUPS;
  return groups
    .map(
      ([emoji, name, tags]) => `<section class="igroup" aria-label="${esc(name)}">
        <h3><span aria-hidden="true">${emoji}</span> ${esc(name)}</h3>
        <div class="ichips">${tags
          .map(
            (t) =>
              `<button type="button" class="ichip" data-tag="${esc(t)}" aria-pressed="${selected.has(t)}">${esc(label(t))}</button>`,
          )
          .join("")}</div></section>`,
    )
    .join("");
}

/** Wires chip toggling inside `root`; calls `onChange(selected)` after each toggle. */
export function bindPicker(root, selected, onChange) {
  root.addEventListener("click", (e) => {
    const b = e.target.closest(".ichip");
    if (!b) return;
    const tag = b.dataset.tag;
    if (selected.has(tag)) selected.delete(tag);
    else selected.add(tag);
    b.setAttribute("aria-pressed", String(selected.has(tag)));
    onChange(selected);
  });
}

/** Saves interests; resolves to the updated `Me`. */
export const saveInterests = (interests) => api.put("/api/me/interests", { interests });

/** "Edit interests" bottom sheet. `onSaved(me)` runs after a successful save. */
export function openInterestSheet(me, onSaved) {
  const selected = new Set(me.interests || []);
  openSheet(
    "Your interests",
    `<p class="muted">Pick at least ${MIN_INTERESTS} — your feed adapts to them.</p>
     <div id="ipick">${pickerHtml(selected)}</div>
     <button type="button" class="btn btn--primary btn--block" id="isave"></button>`,
    (body, close) => {
      const save = body.querySelector("#isave");
      const sync = () => {
        save.disabled = selected.size < MIN_INTERESTS;
        save.textContent = selected.size < MIN_INTERESTS ? `Pick ${MIN_INTERESTS - selected.size} more` : `Save (${selected.size})`;
      };
      bindPicker(body.querySelector("#ipick"), selected, sync);
      sync();
      save.addEventListener("click", async () => {
        save.disabled = true;
        try {
          const updated = await saveInterests([...selected]);
          close();
          toast("Interests saved ✨");
          onSaved(updated);
        } catch (err) {
          toast(err.message || "Couldn't save that");
          sync();
        }
      });
    },
  );
}

/**
 * Full-screen "What are you into?" step. Resolves with the updated `Me` once saved,
 * or `null` if the user skipped.
 */
export function showOnboarding(me) {
  return new Promise((resolve) => {
    const selected = new Set(me.interests || []);
    const el = document.createElement("div");
    el.className = "onboarding";
    el.setAttribute("role", "dialog");
    el.setAttribute("aria-modal", "true");
    el.setAttribute("aria-labelledby", "ob-h");
    el.innerHTML = `<div class="onboarding__scroll">
        <header><p class="onboarding__eyebrow">Welcome, ${esc(me.user.display_name.split(" ")[0])} 👋</p>
          <h1 id="ob-h">What are you into?</h1>
          <p class="muted">Pick at least ${MIN_INTERESTS}. We'll use them to find events and people for you.</p></header>
        <div id="ob-pick">${pickerHtml(selected)}</div></div>
      <footer class="onboarding__bar"><button type="button" class="btn btn--primary btn--block" id="ob-go"></button>
        <button type="button" class="link-btn link-btn--brand" id="ob-skip">Skip for now</button></footer>`;
    document.body.appendChild(el);
    document.body.classList.add("is-onboarding");
    const go = el.querySelector("#ob-go");
    const sync = () => {
      go.disabled = selected.size < MIN_INTERESTS;
      go.textContent = selected.size < MIN_INTERESTS ? `Pick ${MIN_INTERESTS - selected.size} more` : `Continue (${selected.size})`;
    };
    const finish = (result) => {
      el.remove();
      document.body.classList.remove("is-onboarding");
      resolve(result);
    };
    bindPicker(el.querySelector("#ob-pick"), selected, sync);
    sync();
    go.addEventListener("click", async () => {
      go.disabled = true;
      go.textContent = "Saving…";
      try {
        finish(await saveInterests([...selected]));
      } catch (err) {
        toast(err.message || "Couldn't save that");
        sync();
      }
    });
    el.querySelector("#ob-skip").addEventListener("click", () => finish(null));
    el.querySelector(".ichip")?.focus();
  });
}
