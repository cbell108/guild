# 3-minute demo script

Setup (before judges arrive): `cargo run -p seed && cargo run -p ingest && cargo run -p web`,
start the local LLM server, open <http://127.0.0.1:3000>. Click **Sign in** (top right) and pick **Casey (demo)**, the seeded demo student with friends, two crews and history.

## 0:00 — The problem (20 s)
Say: "64% of students feel very lonely, yet every campus platform assumes you show up alone.
Ours finds the right event, shows who's going, and finds someone to go with."

## 0:20 — For you feed (40 s)
1. Click **For you** in the nav. Real KSU calendar events and student hangouts sit in one feed.
2. Point at the **reason line** on a card (e.g. a friend or interest match), then click
   **Why am I seeing this?** to open the full reasons.
3. Point at friend presence ("Jasmine + 1 friend going"). Friends only, never public lists.
4. Scroll to the end: **You're caught up**. The feed is finite on purpose.

## 1:00 — Event page: Going with + buddy match (45 s)
1. Click a card with friends going.
2. Under **Your plans** click **Going with…**, pick a friend chip.
3. In **Find a buddy**, tick **Looking for a buddy**. The match appears: a friend first (Jasmine L.
   on Smash Saturdays), otherwise a similar student you don't know yet. Other seeded students are
   already looking on several events, so matches show up on most upcoming hangouts.
4. Click **Add to calendar** to show the `.ics` download.

## 1:45 — Host: describe it (40 s)
1. Click **Host**.
2. In **Describe it** type "Smash tonight 7-10 in the game room, I have 2 Switches" and click
   **Prefill the form**. The local LLM fills title, time, place, tags.
   The prefill takes about 6 s on the local model; keep talking over it. "tonight" resolves to today.
3. Click **Create hangout**. It appears in the feed. If the LLM is down, the form still works.
4. Optional (10 s): scroll to **When is your crew free?**, pick Owl Hacks and **tomorrow** in Day.
   The grid shades the overlap and says "Best time: 5 PM — 6 of 6 free". Today is empty by design.

## 2:25 — Privacy (15 s)
Click your name chip (top right) to open **Privacy**. Show "Who can see which events I'm going to"
(My friends / Only me), buddy matching, personalization, and **Delete my history**.

## 2:40 — Eval slide (20 s)
Show `docs/EVAL.md`: hybrid precision@5 0.189 vs popularity 0.059 (×3.18), recall@10 0.883 vs
0.558, NDCG@10 0.618 vs 0.255. Last week held out, answer key is real check-ins. Mention the
data is seeded and synthetic.

## Screenshots (`docs/screenshots/`)
Taken with the seeded demo user at 390 px wide (phone) unless noted.

| File | Shows |
|---|---|
| `feed-for-you-mobile.jpg` | For you feed with reason lines and friend presence |
| `feed-for-you-desktop.jpg` | Same feed at 1280 px, three-column grid |
| `why-this-panel-mobile.jpg` | "Why am I seeing this?" with content / collaborative / social / popular breakdown |
| `event-going-with-buddy-mobile.jpg` | Event page: Going with a friend, buddy match (full page) |
| `host-describe-prefill-mobile.jpg` | Host page after Describe it prefilled the form |
| `crew-availability-grid-mobile.jpg` | Crew best-time grid for tomorrow |
| `privacy-settings-mobile.jpg` | Privacy settings and Delete my history |
| `browse-mobile.jpg` | Browse with search, type, day and tag filters |
| `feed.jpg` | Original feed capture |

## If something breaks
- LLM down: skip the Describe-it step and fill the form by hand.
- No backend: open `/?mock=1`.
