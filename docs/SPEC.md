# Product spec — campus-hangouts (working title)

**Pitch line:** 64% of students feel very lonely, and every campus platform still assumes you'll show up alone.

## Problem (with sources for the deck)
1. Discovery is a date-ordered firehose; informal hangouts never appear (they aren't official org events).
   KSU is mid-migration from the old OwlLife (Anthology Engage) to Owl Life powered by Suitable.
2. RSVPs mean little: free events regularly exceed 50% no-shows vs 10–20% for paid
   (https://www.addevent.com/blog/why-people-register-for-your-events-but-never-show-up).
3. Going alone is the real barrier: 64% of ~48,000 students felt very lonely in the past year
   (https://news.chapman.edu/?p=97184); ~25.8% struggle with social anxiety
   (https://www.talkspace.com/blog/social-anxiety-in-college/).
4. Check-in is fragile (QR/Wi-Fi failures; UCL, Anthology docs).
5. Students feel tracked, not helped (Oberlin students pushed back on Presence's data tracking).

## Features (MUST for the demo in bold)
- **"For you" feed** with a one-line reason per card; finite ("You're caught up").
- **Three intent levels:** Interested → Going → Going with ___; same-day "still going?" one-tap back-out; waitlist auto-promotes.
- **Go with someone:** "looking for a buddy" on any event → match a friend first, else a similar student.
- **Friend presence** on cards (friends only; never full attendee lists).
- **Anyone can host a hangout** (web or `/hangout` in Discord), shown alongside official KSU events from the public calendar API.
- **Natural-language hosting:** "smash tonight 7–10 game room, I have 2 switches" → prefilled form (local LLM).
- Crews (small groups, no follower counts), crew availability grid / best time.
- Privacy controlled by students: attendance private by default, "why am I seeing this?", deletable history.
- Stretch: rotating offline-capable QR check-in; after-event attendee-only thread; club (not individual) streaks.

## Recommender
Hybrid score = w1·content + w2·collaborative + w3·social, each part explainable.
1. Content: embeddings of event text vs. user interests + attended events (solves cold start).
2. Collaborative: item-item over implicit feedback (check-in > going > interested).
3. Social: friend boost + predicted probability of actually attending (learned intent→check-in gap).
Evaluate offline vs. a popularity baseline (`docs/EVAL.md`) — the numbers go on a slide.

## Discord integration (ported from DiscordSchedulerBot)
Presence self-report via buttons (no privileged intents), per-date availability windows, gear & games declarations,
plus `/hangout` that creates an app event and a Discord Scheduled Event. Buttons/menus over free text; ephemeral replies.

## Data sources
- KSU public calendar: `https://calendar.kennesaw.edu/api/2/events` (JSON; title, times, location, geo, tags, departments, filters).
- Seeded demo users/history (`crates/seed`), clearly labeled as demo data in the pitch.
