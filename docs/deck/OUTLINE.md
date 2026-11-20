# Pitch deck outline — campus-hangouts

Target: ≤ 4 minutes spoken, 13 slides, plus the 3-minute live demo cued on slide 6 (`docs/DEMO.md`).
Build: `NODE_PATH=<dir with pptxgenjs> node docs/deck/build.js` → `docs/deck/campus-hangouts.pptx` (same text is in each slide's speaker notes; strip zip folder entries after building).
All numbers come from `docs/EVAL.md`, `docs/SPEC.md`, `docs/API.md`. Demo data is synthetic.

## 1. Title — campus-hangouts
*Never go alone.* KSU AI Club Hackathon 2026. `[TEAM NAMES]`
Notes: "We built campus-hangouts: a web app that finds you the right campus event, shows who's going, and finds someone to go with." (10 s)

## 2. The problem
- 64% of ~48,000 students felt very lonely in the past year (Chapman University news)
- ~25.8% struggle with social anxiety (Talkspace)
- Free events regularly see over 50% no-shows vs 10–20% for paid (AddEvent)
- Discovery is a date-ordered firehose; informal hangouts never appear
Notes: Every platform assumes you show up alone, so RSVPs mean little and the barrier is going alone. (25 s)

## 3. Who it's for
KSU students (especially newcomers and commuters), student hosts, and organizations; KSU's calendar is mid-migration from OwlLife to Owl Life, so the official calendar API plus student hangouts is the gap we fill.
Notes: (15 s)

## 4. The solution
Six feature tiles: For you feed with a reason on every card · Interested → Going → Going with ___ · buddy matching · friend presence · anyone can host (natural language via a local LLM) · crews.
Notes: Finite feed ("You're caught up"), waitlist auto-promote, one-tap back-out. (30 s)

## 5. The product (real screenshot)
`feed-for-you-desktop.jpg`: official KSU events and student hangouts in one feed, reason lines, friends going, spots left.
Notes: (15 s)

## 5b. Works on a phone
Three real mobile screenshots (Going with + buddy, describe-it hosting, privacy settings). Notes: 10 s.

## 6. Live demo
Cue: follow `docs/DEMO.md` (feed → event + Going with + buddy → Host "describe it" → privacy). Fallback: `/?mock=1`.
Notes: Hand over to the demo here. (3 min, not counted in the 4)

## 7. How the recommender works
Hybrid score = content + collaborative + social, each with a reason shown to the student. Content: embeddings (solves cold start). Collaborative: item-item over check-in > going > interested. Social: friend boost + learned probability of actually attending.
Notes: (25 s)

## 8. Results
Offline eval, last week held out, answer key = real check-ins, 110 student-weeks, 5 seeds.
Hybrid vs popularity baseline: precision@5 0.189 vs 0.059 (×3.18); recall@10 0.883 vs 0.558 (×1.58); NDCG@10 0.618 vs 0.255. Attendance model log loss 0.554 vs 0.685 hand-set prior.
Honest caveat: seeded synthetic demo data.
Notes: (25 s)

## 9. Privacy by design
No attendee lists or counts · friends see friends only, and only if they opted in · attendance private by default · only id, name, bio shown to others · buddy matching opt-in · personalization can be turned off · crews visible to members only · delete my history.
Notes: Students feel tracked, not helped, so we made these rules normative and tested. (25 s)

## 10. Engineering and organization (Nudox track)
10 crates with a strict dependency graph (core, api, store, recommend, llm, ingest, seed, eval, web, bot); frozen core and API contracts with mock JSON that round-trips; 206 tests, `cargo fmt` + clippy `-D warnings` + tests enforced by a pre-commit hook and a GitHub Actions CI workflow; docs: SPEC, API, DECISIONS, EVAL, DEMO.
Notes: (25 s)

## 11. What's next
Real check-in (offline QR), after-event attendee-only threads, club streaks, pilot with KSU orgs, real-data evaluation, Owl Life integration.
Notes: (10 s)

## 12. Thank you
`[TEAM NAMES]` · repo + demo. Questions?
Notes: (5 s)
