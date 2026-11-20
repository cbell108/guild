/**
 * Event page: full-bleed hero cover (back / calendar / share over it), icon
 * rows, friends, description, buddy card and a sticky action bar with
 * optimistic Going / Interested / Going with… (rolled back on error).
 */
import { api } from "../api.js";
import { esc, fmtEventTime } from "../util.js";
import { cover, dateBadgeHtml, avatar, avatarStack, openSheet, toast, reducedMotion, errorHtml, stateHtml } from "../ui.js";
import { mountCrowd } from "../crowd.js";
import { kindChip, placeLine, friendLine, intentLabel, spotsText } from "../components.js";

/** Icon + text row. */
const row = (icon, html) =>
  `<li class="irow"><span class="irow__icon"><svg class="icon" aria-hidden="true"><use href="#i-${icon}" /></svg></span><span class="irow__text">${html}</span></li>`;

/** Buddy card body from a `BuddyStatus`: switch row plus the match (or a waiting note). */
function buddyHtml(status) {
  const match = status.matched;
  let result = "";
  if (status.looking && match) {
    result = `<div class="buddy-match">${avatar(match.user, "avatar--md")}
      <div><strong>${esc(match.user.display_name)}</strong>${match.is_friend ? ' <span class="chip">Friend</span>' : ""}
      <p>${esc(match.reason)}</p></div></div>`;
  } else if (status.looking) {
    result = `<p class="buddy-wait">⏳ You're on the list — we'll show a match as soon as someone fits.</p>`;
  }
  return `<div class="row buddy-row"><label class="row__main" for="buddy-toggle"><strong>Looking for a buddy</strong>
      <span class="muted">Share the plan with someone new</span></label>
      <span class="switch"><input type="checkbox" id="buddy-toggle" ${status.looking ? "checked" : ""} /><span class="switch__track"></span></span></div>
    <div id="buddy-result" aria-live="polite">${result}</div>`;
}

/** Small celebratory burst + pop on the element (skipped for reduced motion). */
function celebrate(el) {
  if (!el || reducedMotion()) return;
  el.classList.remove("pop");
  void el.offsetWidth;
  el.classList.add("pop");
  const burst = document.createElement("span");
  burst.className = "burst";
  burst.setAttribute("aria-hidden", "true");
  burst.innerHTML = ["🎉", "✨", "💜", "✨", "🎉", "💜"]
    .map((e, i) => `<span style="--a:${i * 60 - 150}deg">${e}</span>`)
    .join("");
  el.appendChild(burst);
  setTimeout(() => burst.remove(), 900);
}

/** Renders the event page. */
export default {
  name: "event",
  async render(root, { state, params }) {
    root.innerHTML = `<div class="skel skel-hero" aria-hidden="true"></div>
      <div class="skel skel-title" aria-hidden="true"></div>`;
    const id = encodeURIComponent(params.id);
    let ev;
    try {
      ev = await api.get(`/api/events/${id}`);
    } catch (err) {
      root.innerHTML = err.status === 404
        ? stateHtml({ emoji: "🧭", title: "Event not found", text: "It may have been removed.", action: "Browse events" })
        : errorHtml(err);
      root.querySelector("[data-action]").addEventListener("click", () => {
        if (err.status === 404) window.location.hash = "#/browse";
        else window.location.reload();
      });
      return;
    }
    const me = state.me;
    let my = ev.my_intent;
    let buddy = { looking: ev.buddy_looking, matched: null };
    if (me && ev.buddy_looking) {
      try {
        buddy = await api.get(`/api/events/${id}/buddy`);
      } catch {
        /* optional */
      }
    }

    const friends = friendLine(ev.friends);
    const spots = spotsText(ev);
    const isHost = Boolean(me && ev.host && ev.host.id === me.user.id);
    const f = ev.friends;
    const heroInner = `${dateBadgeHtml(ev.starts_at)}
      <a class="hero-btn hero-btn--back" href="#/" data-back aria-label="Back"><svg class="icon" aria-hidden="true"><use href="#i-back" /></svg></a>
      <span class="hero-actions">
        <a class="hero-btn" href="${esc(ev.calendar_url)}" download aria-label="Add to calendar"><svg class="icon" aria-hidden="true"><use href="#i-calendar" /></svg></a>
        <button type="button" class="hero-btn" id="share" aria-label="Share"><svg class="icon" aria-hidden="true"><use href="#i-share" /></svg></button>
      </span>`;

    root.innerHTML = `
      <article class="event">
        <div class="event__hero">${cover(ev, heroInner)}</div>
        <div class="event__main">
          <p class="event__kind">${kindChip(ev.kind)}${ev.crew ? "" : ""}</p>
          <h1>${esc(ev.title)}</h1>
          <ul class="irows">
            ${row("clock", esc(fmtEventTime(ev.starts_at, ev.ends_at)))}
            ${row("pin", placeLine(ev.location))}
            ${ev.host ? row("user", `Hosted by <strong>${esc(ev.host.display_name)}</strong>`) : ""}
            ${ev.organizer ? row("user", esc(ev.organizer)) : ""}
            ${ev.crew ? row("users", `Crew · ${esc(ev.crew.name)}`) : ""}
            ${spots ? row("star", esc(spots)) : ""}
          </ul>
          ${
            friends
              ? `<p class="social social--lg">${avatarStack(f.sample || [], Math.max(0, f.going + (f.going ? 0 : f.interested) - Math.min(3, (f.sample || []).length)))}<span>${esc(friends)}</span></p>`
              : ""
          }
          ${ev.description ? `<p class="event__desc">${esc(ev.description)}</p>` : ""}
          <p class="chips">${(ev.tags || []).map((t) => `<span class="chip">#${esc(t)}</span>`).join("")}</p>
          ${ev.url ? `<p><a href="${esc(ev.url)}" target="_blank" rel="noopener noreferrer">View on the KSU calendar ↗</a></p>` : ""}
          ${isHost ? `<p><a class="btn btn--ghost" href="#/checkin/${esc(ev.id)}">Open check-in</a></p>` : ""}
          ${me ? `<section class="crowd-card" id="crowd" aria-live="polite" hidden></section>` : ""}
          ${
            me
              ? `<section class="buddy-card" aria-labelledby="buddy-h">
                   <h2 id="buddy-h"><span aria-hidden="true">🤝</span> Don't want to go alone?</h2>
                   <p class="muted">Turn this on and we'll suggest someone who'd get along.</p>
                   <div id="buddy" class="buddy-card__box">${buddyHtml(buddy)}</div></section>`
              : ""
          }
        </div>
      </article>
      <div class="actionbar" role="group" aria-label="Your plans">
        ${
          me
            ? `<p class="actionbar__status" aria-live="polite"><span id="intent-status"></span>
                 <button type="button" class="link-btn" id="cant" hidden>Can't make it</button></p>
               <div class="actionbar__row">
                 <button type="button" class="btn btn--ghost" data-level="interested">Interested</button>
                 <button type="button" class="btn btn--primary actionbar__main" data-level="going">Going</button>
                 <button type="button" class="btn btn--ghost" data-level="going_with">With…</button>
               </div>`
            : `<button type="button" class="btn btn--primary btn--block" data-signin>Sign in to say you're going</button>`
        }
      </div>`;

    // Back: real history when we came from inside the app.
    root.querySelector("[data-back]").addEventListener("click", (e) => {
      if (window.history.length > 1) {
        e.preventDefault();
        window.history.back();
      }
    });
    root.querySelector("#share").addEventListener("click", async () => {
      const url = window.location.href;
      try {
        if (navigator.share) {
          await navigator.share({ title: ev.title, url });
          return;
        }
        await navigator.clipboard.writeText(url);
        toast("Link copied");
      } catch (err) {
        if (err && err.name !== "AbortError") toast("Couldn't share — copy the address bar link");
      }
    });
    root.querySelector("[data-signin]")?.addEventListener("click", () => state.openLogin());
    if (!me) return;

    mountCrowd(root.querySelector("#crowd"), ev, me);

    const status = root.querySelector("#intent-status");
    const cant = root.querySelector("#cant");
    const buttons = [...root.querySelectorAll("[data-level]")];
    const paint = () => {
      const label = intentLabel(my);
      status.textContent = label ? `You: ${label}` : "Are you in?";
      buttons.forEach((b) => {
        const on = Boolean(my && my.level === b.dataset.level);
        b.setAttribute("aria-pressed", String(on));
        b.classList.toggle("is-on", on);
      });
      cant.hidden = !my;
    };

    /** Optimistic update: show it now, roll back if the API says no. */
    const setIntent = async (level, withUser) => {
      const before = my;
      my = { level, with: withUser || null, status: "confirmed", waitlist_position: null };
      paint();
      try {
        my = await api.put(`/api/events/${id}/intent`, { level, with: withUser ? withUser.id : null });
        paint();
        toast(my.status === "waitlisted" ? "You're on the waitlist" : level === "interested" ? "Marked interested" : "You're going 🎉");
        mountCrowd(root.querySelector("#crowd"), ev, me);
        if (my.status !== "waitlisted") celebrate(root.querySelector(`[data-level="${level}"]`));
      } catch (err) {
        my = before;
        paint();
        toast(err.message || "Couldn't save that — try again");
      }
    };

    buttons.forEach((b) =>
      b.addEventListener("click", async () => {
        if (b.dataset.level !== "going_with") return setIntent(b.dataset.level);
        let list = [];
        try {
          list = (await api.get("/api/friends")).friends;
        } catch (err) {
          toast(err.message || "Couldn't load friends");
          return;
        }
        openSheet(
          "Going with…",
          list.length
            ? `<ul class="people">${list
                .map((u) => `<li><button type="button" class="person person--btn" data-friend="${esc(u.id)}">${avatar(u, "avatar--md")}
                  <span class="person__meta"><strong>${esc(u.display_name)}</strong><span class="muted">${esc(u.bio || "")}</span></span></button></li>`)
                .join("")}</ul>`
            : `<p class="muted">Add a friend first (Me → Friends) to go together.</p>`,
          (body, close) =>
            body.querySelectorAll("[data-friend]").forEach((fb) =>
              fb.addEventListener("click", () => {
                close();
                setIntent("going_with", list.find((u) => u.id === fb.dataset.friend));
              }),
            ),
        );
      }),
    );
    cant.addEventListener("click", async () => {
      const before = my;
      my = null;
      paint();
      try {
        await api.del(`/api/events/${id}/intent`);
        toast("Removed — see you next time");
      } catch (err) {
        my = before;
        paint();
        toast(err.message || "Couldn't update that");
      }
    });
    paint();

    const buddyRoot = root.querySelector("#buddy");
    const bindBuddy = () => {
      const box = buddyRoot.querySelector("#buddy-toggle");
      box.addEventListener("change", async () => {
        box.disabled = true;
        const want = box.checked;
        try {
          buddy = await api.post(`/api/events/${id}/buddy`, { looking: want });
          toast(want ? "We'll look for a buddy" : "Buddy search off");
        } catch (err) {
          toast(err.message || "Couldn't update that");
        }
        buddyRoot.innerHTML = buddyHtml(buddy);
        bindBuddy();
      });
    };
    bindBuddy();
  },
};
