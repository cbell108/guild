/**
 * "Who's going" + time matrix for the event page.
 *
 * Shows you and the friends who share attendance (never a public list), a grid of who
 * can attend each hour, and a form to set the range of times *you* can attend.
 * Game-night events (smash, fighting games, …) get a dynamic "Match times" heading and tips.
 */
import { api } from "./api.js";
import { esc } from "./util.js";
import { avatar, toast } from "./ui.js";

const TZ = "America/New_York";
const HOUR = 3600_000;

/** Tags that make an event a "game night" (gets the match-times treatment). */
const GAME_TAGS = new Set([
  "gaming", "smash", "smash-bros", "fighting-games", "fighting-game", "fgc", "tekken",
  "street-fighter", "mario-kart", "esports", "nintendo", "board-games",
]);

/**
 * Dynamic detail profile for an event, chosen from its tags/title.
 * @returns {{kind: string, heading: string, lede: string, tip: string}}
 */
export function profileFor(ev) {
  const tags = new Set(ev.tags || []);
  const text = `${ev.title} ${ev.description || ""}`.toLowerCase();
  const game = [...tags].some((t) => GAME_TAGS.has(t)) || /smash|tekken|street fighter|fighting game|mario kart/.test(text);
  if (game) {
    return {
      kind: "game",
      heading: "Match times",
      lede: "See when everyone can play and share the hours you're free.",
      tip: "🎮 Bring a controller if you have one — setups are shared.",
    };
  }
  return { kind: "default", heading: "Who's going", lede: "You and your friends who share their plans.", tip: "" };
}

/** `HH:MM` (24 h) in campus time. */
function hhmm(date) {
  return new Intl.DateTimeFormat("en-GB", { timeZone: TZ, hour: "2-digit", minute: "2-digit", hour12: false }).format(date);
}

/** Short hour label like "7p". */
function hourLabel(ms) {
  const parts = new Intl.DateTimeFormat("en-US", { timeZone: TZ, hour: "numeric", hour12: true }).formatToParts(new Date(ms));
  const h = parts.find((p) => p.type === "hour").value;
  const ap = parts.find((p) => p.type === "dayPeriod").value.toLowerCase()[0];
  return `${h}${ap}`;
}

/** Hour columns covering the event plus an hour either side (max 14). */
function columns(ev, people) {
  const starts = [new Date(ev.starts_at).getTime(), ...people.filter((p) => p.free_from).map((p) => new Date(p.free_from).getTime())];
  const ends = [new Date(ev.ends_at).getTime(), ...people.filter((p) => p.free_until).map((p) => new Date(p.free_until).getTime())];
  const first = Math.floor((Math.min(...starts) - HOUR) / HOUR) * HOUR;
  const last = Math.ceil((Math.max(...ends) + HOUR) / HOUR) * HOUR;
  const cols = [];
  for (let t = first; t < last && cols.length < 14; t += HOUR) cols.push(t);
  return cols;
}

/** Whether a person's shared range overlaps the hour starting at `t`. */
const free = (p, t) =>
  Boolean(p.free_from && new Date(p.free_from).getTime() < t + HOUR && new Date(p.free_until).getTime() > t);

/** The crowd card's inner HTML. */
export function crowdHtml(ev, crowd) {
  const profile = profileFor(ev);
  const people = crowd.people;
  if (!people.length) {
    return `<p class="muted">Nobody you know has said they're going yet. Tap Going to be first.</p>`;
  }
  const cols = columns(ev, people);
  const evStart = new Date(ev.starts_at).getTime();
  const evEnd = new Date(ev.ends_at).getTime();
  const counts = cols.map((t) => people.filter((p) => free(p, t)).length);
  const best = Math.max(0, ...counts);
  const head = cols
    .map((t) => `<th scope="col" class="${t + HOUR > evStart && t < evEnd ? "is-event" : ""}">${esc(hourLabel(t))}</th>`)
    .join("");
  const rows = people
    .map((p) => {
      const cells = cols
        .map((t) => `<td class="${free(p, t) ? "is-free" : ""}"><span class="visually-hidden">${free(p, t) ? "free" : "not shared"}</span></td>`)
        .join("");
      const label = p.level === "interested" ? "Interested" : p.level === "going_with" ? "Going with" : "Going";
      return `<tr><th scope="row" class="matrix__who">${avatar(p.user, "avatar--md")}<span>${esc(p.is_me ? "You" : p.user.display_name)}
        <small>${esc(label)}${p.free_from ? "" : " · no times shared"}</small></span></th>${cells}</tr>`;
    })
    .join("");
  const total = counts
    .map((c) => `<td class="${c && c === best ? "is-best" : ""}">${c || ""}</td>`)
    .join("");
  const bestCols = cols.filter((_, i) => counts[i] === best && best > 1);
  const bestText = bestCols.length
    ? `Best time: ${hourLabel(bestCols[0])}–${hourLabel(bestCols[bestCols.length - 1] + HOUR)} (${best} free)`
    : "Share your hours below to find a time that works.";
  return `<div class="matrix__scroll"><table class="matrix">
      <caption class="visually-hidden">${esc(profile.heading)}: who can attend each hour</caption>
      <thead><tr><th><span class="visually-hidden">Person</span></th>${head}</tr></thead>
      <tbody>${rows}<tr class="matrix__total"><th scope="row">Free</th>${total}</tr></tbody></table></div>
    <p class="matrix__best" aria-live="polite">${esc(bestText)}</p>`;
}

/** Mounts the card into `box` (an element) and wires the availability form. */
export async function mountCrowd(box, ev, me) {
  const profile = profileFor(ev);
  const id = encodeURIComponent(ev.id);
  const defaults = { start: hhmm(new Date(ev.starts_at)), end: hhmm(new Date(ev.ends_at)) };
  const load = async () => {
    let crowd;
    try {
      crowd = await api.get(`/api/events/${id}/crowd`);
    } catch {
      box.hidden = true;
      return;
    }
    const mine = crowd.people.find((p) => p.is_me);
    const from = mine && mine.free_from ? hhmm(new Date(mine.free_from)) : defaults.start;
    const until = mine && mine.free_until ? hhmm(new Date(mine.free_until)) : defaults.end;
    box.hidden = false;
    box.innerHTML = `<h2><span aria-hidden="true">${profile.kind === "game" ? "🕹️" : "👥"}</span> ${esc(profile.heading)}</h2>
      <p class="muted">${esc(profile.lede)}</p>
      ${crowdHtml(ev, crowd)}
      <form class="range" id="range">
        <p class="range__title"><strong>When can you make it?</strong></p>
        <div class="range__row">
          <label class="field"><span>From</span><input type="time" name="start" value="${esc(from)}" required /></label>
          <label class="field"><span>Until</span><input type="time" name="end" value="${esc(until)}" required /></label>
          <button class="btn btn--soft" type="submit">Save my times</button>
        </div>
      </form>
      ${profile.tip ? `<p class="muted range__tip">${esc(profile.tip)}</p>` : ""}`;
    box.querySelector("#range").addEventListener("submit", async (e) => {
      e.preventDefault();
      const f = e.target.elements;
      try {
        await api.put("/api/me/availability", { date: crowd.date, start: f.start.value, end: f.end.value });
        toast("Times saved");
        await load();
      } catch (err) {
        toast(err.message || "Couldn't save your times");
      }
    });
  };
  await load();
}
