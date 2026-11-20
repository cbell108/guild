# Morning notes — Fri Nov 20 (submission due 12:00 PM ET)

## Merged into `main` overnight
- **UI redesign and polish:** a scrollable, simplified events list. Screenshots were checked at 390 px.
- **API speed:** the buddy endpoint went from 55 ms to 2 ms in a debug build.
- **Parse robustness:** the local LLM scores 153/153 on the phrase set.
- **AI features:**
  - Interest onboarding (`PUT /api/me/interests`).
  - Ask-to-browse (`POST /api/ask`, natural language → Browse filters, with a plain-search fallback).
- **Frontend unit tests:** 39, now in CI. They caught a real bug (signed hash in generated covers), which is fixed.
- **Night GPU queues** (local Qwen Q3, reviewed before merge):
  - N1 `docs/ARCHITECTURE.md`
  - N2 `CHANGELOG.md`
  - M1 ingest mapping edge tests
  - M2 store edge tests

**Gate on `main` at 10:25:**
- fmt and clippy (`-D warnings`) are clean.
- **281 Rust tests pass**, 0 fail.
- **44 JS tests pass** (`node --test static/js/tests/`).
- Lanes N and M are finished (N1–N4, M1–M3).

## Added this morning (09:40–)
- **QR check-in (B8 resolved):** the host's check-in page draws the rotating QR with a dependency-free encoder (`static/js/qr.js`), verified module-for-module against libqrencode for every length 1–213 bytes, with node tests. The attendee link was tested end to end.
- **For you:** the today rail says "Today" before 5 PM instead of "Tonight".
- **Docs:**
  - README's screenshot was a broken link; it now shows three real screenshots.
  - `docs/DEMO.md` matches the redesigned UI and adds Ask and QR check-in.
  - B9 resolved.
  - N3 `CONTRIBUTING.md` + `.editorconfig`, N4 recommender edge tests and M3 bot helper tests merged.
- **Date time bomb fixed:** `Store::list_tags` used the wall clock while tests are pinned to 2026-11-20. A store test failed from 07:00 ET today, and the web `/api/tags` test would have failed from about 3 PM ET (CI red during judging). `/api/tags` and `/api/ask` now use the app clock via a new `Store::list_tags_from`.
- **Queue script bug found and fixed:** `git add` named gitignored files, so it exited 1 and `set -e` killed the script after every passing task. That's why the overnight queues stopped after one task each.

**For a live QR scan:** the phone must reach the laptop. Run web with `BIND_ADDR=0.0.0.0:3000` and open the site by the laptop's LAN IP, not 127.0.0.1, since the QR encodes the page's origin.

## Run the demo
```bash
~/bin/gpu-mode split          # Q3 on each GPU; the app uses qwen3.8-27B-UD-Q3_K_XL:9070XT
cargo run -p seed             # deterministic demo users, crews, history
cargo run -p ingest           # real KSU calendar events
cargo run -p web              # http://127.0.0.1:3000
```
- `.env` needs `LLM_MODEL=qwen3.8-27B-UD-Q3_K_XL:9070XT` (see `.env.example`).
- Without the LLM, parse and ask return 503 and the UI falls back gracefully.
- Never point the app at the Q6 model: it evicts both Q3 workers.

## Your to-dos
1. **RX 9070 XT runtime PM.** The card can hang when it resumes from power-off. Pick one:
   - for this boot: `echo on | sudo tee /sys/bus/pci/devices/0000:18:00.0/power/control`
   - permanent: add `amdgpu.runpm=0` to the kernel command line

   The overnight keep-alive script stops at 06:00.
2. **`deck-backup`** (backup slides + SCRIPT.md) is still unmerged, as you asked.
3. **itcan.do track:** I couldn't find its requirements. Please check them.
4. **Bot:** put the real `DISCORD_TOKEN` in `.env` if the bot is part of the demo.
5. **Demo video.**
6. **Submit by 12:00 PM ET.**

## BLOCKED.md
B8 and B9 are resolved; no blocked tasks remain.
