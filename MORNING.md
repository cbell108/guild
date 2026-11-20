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

**Gate on `main` at 05:30:**
- fmt and clippy (`-D warnings`) are clean.
- **253 Rust tests pass**, 0 fail.
- **39 JS tests pass** (`node --test static/js/tests/`).

**Not done:** N3 (CONTRIBUTING + .editorconfig), N4 (recommend edge tests) and M3 (bot helper tests) are still unticked in TASKS.md.

Both queue runs stopped silently at the commit step, at about 02:20 and again after 03:43. Their work had already passed the gate, so I committed it by hand. Worth a look before reusing `scripts/local-takeover.sh`.

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

## Partial items from BLOCKED.md
- **B8** — host check-in shows the rotating link and a copy button, but no QR image yet (the frontend has no QR encoder). A small vendored encoder, such as nayuki's MIT `qrcodegen`, would close this.
- **B9** — the accessibility pass is done. Only a low-resolution `docs/screenshots/feed.jpg` is saved; re-capture feed, event, host, browse and crews at full size for the deck.
