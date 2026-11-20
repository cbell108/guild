# eval

Offline evaluation of the recommender against baselines.
`cargo run -p eval` writes the results table to [../../docs/EVAL.md](../../docs/EVAL.md).

## Public API

- `run(now, seeds)` — the full protocol: seed a world, hold out the last week, rank, score, average.
- `to_markdown(&Report)` — render the report as the markdown table in `docs/EVAL.md`.
- `Report`, `Metrics`, `Method`, `NamedMethod`, `methods()` — the report shape and the evaluated methods (hybrid feed + baselines).
- `as_of`, `cases` — hide the holdout week and build per-student ranking cases.
- `rank`, `evaluate`, `score_ranking` — per-case ranking and precision@5 / recall@10 / NDCG@10.
- `model_report` — attendance-model accuracy on the holdout week.
- `HOLDOUT` — the 7-day holdout window.

The `feed_coverage` binary (`src/bin/feed_coverage.rs`) reports how much of the hidden week the feed covers.

## Depends on

- `hangouts-core` — operates on `Snapshot`s.
- `seed` — generates the world offline (no database).
- `recommend` — the methods under test.

## Tests

Unit tests live inline in `src/`. Run with `cargo test -p eval`.

## Notes

- No database, no network: the world comes from `seed::generate`, so the table is reproducible.
- Events already RSVP'd to before the cutoff are removed from the ranking and the answer key — methods are scored on *discovery*, not echoing known RSVPs.
- The binary fixes `EVAL_NOW` and the seed list so the committed `docs/EVAL.md` is reproducible.
- The protocol is explained for the pitch in [../../docs/EVAL.md](../../docs/EVAL.md).
