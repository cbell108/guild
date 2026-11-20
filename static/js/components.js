/**
 * Event card building blocks shared by the feed, Browse and the event page.
 * Pure HTML-string builders; every dynamic value goes through `esc`.
 */
import { esc, fmtClock } from "./util.js";
import { cover, dateBadgeHtml, avatarStack } from "./ui.js";

/** "Priya S." → "Priya". */
function firstName(user) {
  return String(user && user.display_name ? user.display_name : "").split(/\s+/)[0];
}

/**
 * Friend presence line from a `FriendPresence`, e.g. "Jordan + 2 going".
 * Returns "" when no friends are going or interested.
 */
export function friendLine(friends) {
  if (!friends) return "";
  const lead = friends.sample && friends.sample[0] ? firstName(friends.sample[0]) : "";
  if (friends.going > 0) {
    if (friends.going === 1) return lead ? `${lead} is going` : "1 friend going";
    return lead ? `${lead} + ${friends.going - 1} going` : `${friends.going} friends going`;
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
  if (my.level === "going_with") return my.with ? `Going with ${firstName(my.with)}` : "Going";
  return my.level === "going" ? "Going" : "Interested";
}

/** Kind chip ("Official" / "Hangout"). */
export function kindChip(kind) {
  return `<span class="kind-chip kind-chip--${esc(kind)}">${kind === "official" ? "Official" : "Hangout"}</span>`;
}

/** Venue as one line ("Student Center · Game Room 274"). */
export function placeLine(location) {
  if (!location) return "";
  return [location.name, location.room].filter(Boolean).map(esc).join(" · ");
}

/** Capacity text ("2 of 8 spots left" / "Full · waitlist open"), or "". */
export function spotsText(event) {
  if (event.capacity === null || event.capacity === undefined) return "";
  if (event.spots_left === 0) return "Full · waitlist open";
  return `${event.spots_left} of ${event.capacity} spots left`;
}

/** Pill shown only when spots are low (≤ 3) or the event is full. */
function spotsPill(ev) {
  if (ev.capacity === null || ev.capacity === undefined) return "";
  if (ev.spots_left === 0) return `<span class="pill pill--warn">Full · waitlist</span>`;
  if (ev.spots_left <= 3) {
    return `<span class="pill pill--warn">${ev.spots_left} spot${ev.spots_left === 1 ? "" : "s"} left</span>`;
  }
  return "";
}

/** "7 PM · Student Center" — clock time and venue name on one line. */
export function whenPlace(ev) {
  return `${esc(fmtClock(ev.starts_at))} · ${esc(ev.location ? ev.location.name : "")}`;
}

/** Friend avatars plus the "Jordan + 2 going" text (or ""). */
export function socialLine(ev) {
  const text = friendLine(ev.friends);
  if (!text) return "";
  const f = ev.friends;
  const shown = Math.min(3, (f.sample || []).length);
  const total = f.going + (f.going ? 0 : f.interested);
  return `<p class="social">${avatarStack(f.sample || [], Math.max(0, total - shown))}<span>${esc(text)}</span></p>`;
}

/**
 * One carousel/grid card. `item` is a `FeedItem` ({event, reasons, breakdown}) or a
 * bare `EventCard`. `variant`: `"hero"` (large, title over the cover) or `"card"`.
 * A feed item's reason shows as one ✨ line; its "Why?" info button carries the
 * data-why attribute (the feed view opens a bottom sheet for it).
 */
export function eventCard(item, variant = "card") {
  const ev = item.event || item;
  const reasons = item.reasons || [];
  const mine = intentLabel(ev.my_intent);
  const title = `<a class="ecard__title-link" href="#/event/${esc(ev.id)}">${esc(ev.title)}</a>`;
  const badges = `${dateBadgeHtml(ev.starts_at)}<span class="cover__kind">${kindChip(ev.kind)}</span>`;
  const hero = variant === "hero";
  const art = cover(
    ev,
    `${badges}${mine ? `<span class="cover__mine">${esc(mine)}</span>` : ""}${
      hero ? `<h3 class="ecard__title ecard__title--over">${title}</h3>` : ""
    }`,
  );
  const reason = reasons[0]
    ? `<p class="reason"><span class="reason__text"><span aria-hidden="true">✨</span> ${esc(reasons[0].text)}</span>
        <button type="button" class="reason__info" data-why="${esc(ev.id)}" aria-label="Why am I seeing this?"><svg class="icon" aria-hidden="true"><use href="#i-info" /></svg></button></p>`
    : "";
  return `<article class="ecard ecard--${variant}" data-id="${esc(ev.id)}">
    ${art}
    <div class="ecard__body">
      ${hero ? "" : `<h3 class="ecard__title">${title}</h3>`}
      <p class="ecard__meta">${whenPlace(ev)}</p>
      ${socialLine(ev)}
      ${reason}
      ${spotsPill(ev)}
    </div>
  </article>`;
}

/** Compact Browse row: 72 px cover thumbnail on the left, text on the right. */
export function eventRow(ev) {
  const mine = intentLabel(ev.my_intent);
  const friends = friendLine(ev.friends);
  return `<a class="erow" href="#/event/${esc(ev.id)}">
    ${cover(ev)}
    <span class="erow__text">
      <span class="erow__title">${esc(ev.title)}</span>
      <span class="erow__meta">${whenPlace(ev)}</span>
      <span class="erow__tags">${kindChip(ev.kind)}${friends ? `<span class="erow__friends">${esc(friends)}</span>` : ""}${
        mine ? `<span class="pill">${esc(mine)}</span>` : ""
      }${spotsPill(ev)}</span>
    </span>
  </a>`;
}

/** Bottom-sheet body explaining a feed item's reasons + score breakdown. */
export function whyHtml(item) {
  const bd = item.breakdown;
  const bars = bd
    ? `<div class="why__bars">${["content", "collaborative", "social", "popular"]
        .filter((k) => bd[k] > 0)
        .map(
          (k) =>
            `<div class="why__bar"><span>${k}</span><span class="why__track"><span class="why__fill" style="width:${Math.min(100, Math.round(bd[k] * 250))}%"></span></span></div>`,
        )
        .join("")}</div>`
    : "";
  return `<ul class="why__list">${(item.reasons || [])
    .map((r) => `<li><span class="why__kind">${esc(r.kind)}</span> ${esc(r.text)}</li>`)
    .join("")}</ul>${bars}`;
}
