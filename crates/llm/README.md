# llm

Client for the local OpenAI-compatible server (llama.cpp router) and the natural-language
hosting feature behind `POST /api/parse`: `parse_hangout(text, now)` → `HangoutDraft`.

Everything degrades gracefully: `LlmError::Unavailable` / `BadResponse` become a 503 and the
Host page falls back to its manual form. Tests never need the server.

## How parsing works

1. `hangout::request` builds a schema-constrained chat request. The system prompt spells out
   today's campus-local date and the next seven weekdays, so the model picks dates from a list
   instead of doing calendar arithmetic, and states the traps explicitly (`tmrw`, bare evening
   hours, `midnight`, `next <weekday>`, "in 2 hours", "no day named means null").
2. `hangout::parse_response` extracts and `HangoutDraft::normalized` validates the JSON
   (invalid dates/times dropped, tags cleaned).
3. `hangout::refine` checks the draft against the host's own words, deterministically:
   a date survives only if the text names a day (today, tonight, tmrw, weekdays, months, `next`,
   `11/28`); and "in 2 hours / an hour / 45 minutes" is computed from `now` rather than from
   model arithmetic.

## Accuracy

Phrase set: `tests/phrases.json`, 30 phrasings (typos, ALL CAPS, 12-hour and bare hours,
"tmrw", "next tuesday", month dates, past-midnight ends, capacity phrasing, missing day/time,
relative times) against `qwen3.8-27B-UD-Q6_K_M`, with `now` fixed at Thu 2026-11-19 12:00 ET.
Each phrase scores the date, start, end, location, tags and capacity it expects (153 checks).

| Version | Checks correct |
|---|---|
| Before (original prompt, no `refine`) | 151 / 153 (98.7%) |
| After (this prompt + `refine`) | 153 / 153 (100%) |
| After, on `qwen3.8-27B-UD-Q3_K_XL:9070XT` (the default since 2026-11-20) | 153 / 153 (100%) |

The two baseline misses were the traps this work targets: a date invented for "chill hangout at
the rec center" (no day given) and "midnight snack run in 2 hours" parsed as 01:00 instead of
14:00. The first set of 16 easier phrasings was already 98.8% before any change, so the model
is solid on the common cases and the work here is mostly guard rails. Since the prompt also
changed, "in 2 hours" now comes out right from the model alone; `refine` is the safety net.
Note: after the first run the expectations for three phrases with a time but no day were
widened to accept today or null, because the Host form already defaults its date to today.

The score is one run at temperature 0 on one `now`; it is a regression check, not a benchmark.

## Tests

- Offline (default `cargo test -p llm`): unit tests for the prompt, parsing, normalization and
  `refine`, plus `tests/recorded.rs`, which replays 8 real server replies from
  `tests/fixtures/phrases/` through parse + refine.
- Live (`#[ignore]`, needs the server):

```bash
cargo test -p llm --test live_phrases -- --ignored --nocapture phrase_set_accuracy
```

  prints a per-phrase score and fails below 85%. Re-record the fixtures after a prompt change:

```bash
cargo test -p llm --test live_phrases -- --ignored record_fixtures
```
