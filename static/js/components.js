/**
 * Shared UI fragments (HTML-string builders) used by several views.
 * Every dynamic value goes through `esc`.
 */
import { esc } from "./util.js";
import { fmtEventTime } from "./util.js";

/** "Priya S." → "Priya". */
function firstName(user) {
  return String(user && user.display_name ? user.display_name : "").split(/\s+/)[0];
}

/**
 * Friend presence line from a `FriendPresence`, e.g. "Jordan + 2 friends going".
 * Returns "" when no friends are going or interested.
 */
export function friendLine(friends) {
  if (!friends) return "";
  const lead = friends.sample && friends.sample[0] ? firstName(friends.sample[0]) : "";
  const plural = (n) => (n === 1 ? "friend" : "friends");
  if (friends.going > 0) {
    if (friends.going === 1) return lead ? `${lead} is going` : "1 friend going";
    return lead
      ? `${lead} + ${friends.going - 1} ${plural(friends.going - 1)} going`
      : `${friends.going} friends going`;
  }
  if (friends.interested > 0) {
    if (friends.interested === 1) return lead ? `${lead} is interested` : "1 friend interested";
    return `${friends.interested} friends interested`;
  }
  return "";
}

/** Short label for the viewer's own intent, or "". */
export function intentLabel(my) {
  if (!my) return "";
  if (my.status === "waitlisted") {
    return my.waitlist_position ? `Waitlisted · #${my.waitlist_position}` : "Waitlisted";
  }
  if (my.level === "going_with") {
    return my.with ? `Going with ${firstName(my.with)}` : "Going";
  }
  return my.level === "going" ? "Going" : "Interested";
}

/** Kind badge ("Official" / "Hangout"). */
export function kindBadge(kind) {
  const label = kind === "official" ? "Official" : "Hangout";
  return `<span class="badge-kind badge-kind--${esc(kind)}">${label}</span>`;
}

/** Venue as one line ("Student Center · Game Room 274"). */
export function placeLine(location) {
  if (!location) return "";
  return [location.name, location.room].filter(Boolean).map(esc).join(" · ");
}

/** Pin icon + text helper for meta rows. */
function metaRow(icon, html) {
  return `<li class="meta-row"><span class="meta-row__icon" aria-hidden="true">${icon}</span><span>${html}</span></li>`;
}

const ICONS = {
  clock: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></svg>`,
  pin: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 21s7-6.2 7-11.5A7 7 0 0 0 5 9.5C5 14.8 12 21 12 21z"/><circle cx="12" cy="9.5" r="2.5"/></svg>`,
  people: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="9" cy="8" r="3.2"/><path d="M3 20c0-3.3 2.7-5.5 6-5.5s6 2.2 6 5.5M16 5.2a3 3 0 0 1 0 5.6M18 14.8c1.9.7 3 2.4 3 5.2"/></svg>`,
};

/** Capacity text ("2 spots left" / "Full — join the waitlist"), or "". */
export function spotsText(event) {
  if (event.capacity === null || event.capacity === undefined) return "";
  if (event.spots_left === 0) return "Full · waitlist open";
  return `${event.spots_left} of ${event.capacity} spots left`;
}

/**
 * One event card. `item` may be a `FeedItem` ({event, reasons}) or a bare
 * `EventCard`; at most one reason line is shown inline (the rest lives behind
 * the "Why this?" disclosure).
 */
export function eventCard(item) {
  const ev = item.event || item;
  const reasons = item.reasons || [];
  const friends = friendLine(ev.friends);
  const mine = intentLabel(ev.my_intent);
  const spots = spotsText(ev);
  const tags = (ev.tags || []).slice(0, 4).map((t) => `<span class="chip">#${esc(t)}</span>`).join("");
  const why = reasons.length > 1
    ? `<details class="why"><summary>Why am I seeing this?</summary><ul>${reasons
        .map((r) => `<li><span class="why__kind">${esc(r.kind)}</span> ${esc(r.text)}</li>`)
        .join("")}</ul></details>`
    : "";
  return `
    <article class="event-card card">
      <div class="event-card__top">
        ${kindBadge(ev.kind)}
        ${mine ? `<span class="status-pill">${esc(mine)}</span>` : ""}
      </div>
      <h2 class="event-card__title"><a href="#/event/${esc(ev.id)}">${esc(ev.title)}</a></h2>
      <ul class="meta-list">
        ${metaRow(ICONS.clock, esc(fmtEventTime(ev.starts_at, ev.ends_at)))}
        ${metaRow(ICONS.pin, placeLine(ev.location))}
        ${friends ? metaRow(ICONS.people, `<strong>${esc(friends)}</strong>`) : ""}
      </ul>
      ${reasons[0] ? `<p class="reason">${esc(reasons[0].text)}</p>` : ""}
      ${why}
      <div class="event-card__foot">
        <span class="chips">${tags}</span>
        ${spots ? `<span class="spots">${esc(spots)}</span>` : ""}
      </div>
    </article>`;
}
