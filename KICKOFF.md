# Kickoff (Phase 0, interactive)

Start in the repo root: `claude --model opus --effort high`, then paste:

> Read CLAUDE.md, TASKS.md and docs/SPEC.md. We're doing Phase 0 (F1–F7) only, tonight, interactively.
> Port domain decisions and the bug-fix tests (strict date/time parsing, zero-padded dates, DST-safe overnight
> rollover) from ../DiscordSchedulerBot (AGENTS.md, src/). The core types, store/recommend/llm signatures and
> docs/API.md become frozen after this phase, because three parallel agents will build against them overnight
> without being able to change them — design them so Lanes A, B and C in TASKS.md never need to.
> Start in plan mode and show me the plan before writing code.

When F7 is committed: `./scripts/launch-overnight.sh`, check `claude agents` shows three sessions working, sleep.

## Morning
1. `claude agents` — see what finished. Read NEEDS.md and BLOCKED.md.
2. `git branch` to see the lane branches; merge each into main (lane C, then A, then B), fixing integration with Opus.
3. Connect the real bot token, `cargo run -p seed`, `cargo run -p ingest`, run web + bot, rehearse the demo.
4. 8 AM feature freeze → Nudox cleanup → demo video → PowerPoint deck → submit before 12:00 PM.
