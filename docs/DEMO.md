# 3-minute demo script

Setup (before judges arrive): `cargo run -p seed && cargo run -p ingest && cargo run -p web`,
start the local LLM server, open <http://127.0.0.1:3000>. Click **Sign in** (top right) and pick **Casey (demo)**, the seeded demo student with friends, two crews and history.

## 0:00 — The problem (20 s)
Say: "64% of students feel very lonely, yet every campus platform assumes you show up alone.
Ours finds the right event, shows who's going, and finds someone to go with."

## 0:20 — For you feed (35 s)
1. **For you** (bottom nav) opens on **Top picks for you**: real KSU calendar events and student
   hangouts in one feed, then rails for Today, This weekend, Your friends are going and
   Because you like ….
2. Point at the **✨ reason line** on a card, then tap the ⓘ (**Why am I seeing this?**) to open the
   content / collaborative / social / popular breakdown.
3. Point at friend presence ("Jasmine + 1 going"). Friends only, never public lists.
4. Scroll to the end: **You're caught up**. The feed is finite on purpose.

## 0:55 — Ask for something (15 s)
Tap **Browse**, type "something chill tonight with food" in **Ask for something** and press ✨.
The local LLM turns it into filters (Today · Hangouts · #food) and a pill saying what it chose.
It takes a few seconds; if the LLM is down it falls back to plain search.

## 1:10 — Event page: Going with + buddy match (35 s)
1. Open **Smash Saturdays** (a card with friends going).
2. In the bottom action bar tap **With…** and pick a friend chip.
3. Under **🤝 Don't want to go alone?** turn on **Looking for a buddy**. The match appears: a friend
   first (Jasmine L. on Smash Saturdays), otherwise a similar student you don't know yet.
4. The calendar icon at the top of the photo downloads an `.ics`.

## 1:45 — Host: describe it, then check people in (40 s)
1. Tap **Host** (the + in the bottom nav).
2. In **Describe your hangout** type "Smash tonight 7-10 in the game room, I have 2 Switches" and
   tap **✨ Fill it in**. The local LLM fills title, time, place and tags (about 6 s; keep talking).
3. Tap **Create hangout**, then **Open check-in** on the new event. A rotating QR code appears
   (it refreshes every 20 s, so screenshots don't work); scanning it on a phone checks the
   attendee in. For a live scan, run web with `BIND_ADDR=0.0.0.0:3000` and open the site by the
   laptop's LAN address so the phone can reach it.
4. Optional (10 s): back on Host, **When is your crew free?**: pick Owl Hacks and tomorrow. The grid
   shades the overlap and names the best time.

## 2:25 — Privacy (15 s)
**Me** → **Privacy**. Show "Who can see which events I'm going to" (My friends / Only me), buddy
matching, personalization, and **Delete my history**.

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
| `ask-browse-mobile.jpg` | Ask for something: natural language turned into Browse filters |
| `onboarding-mobile.jpg` | First sign-in interest picker |
| `checkin-qr-mobile.jpg` | Host check-in page with the rotating QR code |
| `*-dark.jpg` | Dark-mode versions of the feed, event, me and privacy pages |

## If something breaks
- LLM down: skip **Fill it in** and fill the form by hand; Ask falls back to plain search.
- No backend: open `/?mock=1`.
