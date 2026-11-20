# recommend

The hybrid event recommender: content (embedding) + collaborative + social + popularity,
each part explainable, plus attendance modelling and user similarity.
Takes data in, never touches the database.

## Public API

- `RecContext` — snapshot + viewer + now + attendance model + embedder; one context per call.
- `feed(ctx, config)` — rank events for the viewer into explainable `Recommendation`s.
- `FeedConfig`, `Weights` — feed settings and per-signal weighting.
- `similar_users`, `UserMatch` — find users with overlapping interests/attendance.
- `AttendanceModel`, `AttendanceFeatures`, `training_samples`, `past_show_rate`, `friends_going` — show-up probability model (train once, or `AttendanceModel::prior`).
- `Embedder`, `HashEmbedder`, `default_embedder`, `event_text`, `cosine`, `mean_normalized` — text embedding; `FastEmbedder` behind the `fastembed` feature.
- `RecommendError` — error type.

## Depends on

- `hangouts-core` — operates on `Snapshot` and core domain types.
- `seed` (dev-dependency) — builds deterministic test worlds.

## Tests

Unit tests live inline in `src/` (`#[cfg(test)]` modules, with `test_support.rs` building small snapshots).
Run with `cargo test -p recommend`.

## Notes

- **DB-free by rule**: the crate takes a `core::Snapshot` in and returns results, so it stays unit-testable without a database.
- Stable entry points (`RecContext`, `FeedConfig`, `Weights`, `feed`, `similar_users`, `UserMatch`, `AttendanceModel`, `AttendanceFeatures`, `Embedder`, `HashEmbedder`, `default_embedder`, `event_text`, `RecommendError`) keep stable signatures; internals may change freely.
- `now` is injected into `RecContext`, never read from the wall clock.
- The `eval` crate measures this recommender offline; see [../../docs/EVAL.md](../../docs/EVAL.md).
