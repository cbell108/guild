//! `cargo run -p eval`: offline evaluation (C8). Holds out the last week of seeded history and
//! compares precision@5 / recall@10 for the hybrid feed vs. popularity and content-only
//! baselines, writing the table to `docs/EVAL.md`.

fn main() -> anyhow::Result<()> {
    let now = chrono::Utc::now();
    let snapshot = seed::generate(&seed::SeedConfig::demo(now));
    let _ = (&snapshot, recommend::FeedConfig::default());
    anyhow::bail!("eval is not implemented yet (TASKS.md C8)")
}
