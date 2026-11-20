//! `cargo run -p eval`: offline evaluation (C8). Holds out the last week of seeded history and
//! compares precision@5 / recall@10 for the hybrid feed vs. popularity and content-only
//! baselines, writing the table to `docs/EVAL.md`.

use anyhow::Context;
use chrono::{DateTime, Utc};

/// Fixed so the committed `docs/EVAL.md` is reproducible.
const EVAL_NOW: &str = "2026-11-20T17:00:00Z";
/// Seeds averaged in the table.
const SEEDS: &[u64] = &[2026, 1, 2, 3, 4];

fn main() -> anyhow::Result<()> {
    let now: DateTime<Utc> = DateTime::parse_from_rfc3339(EVAL_NOW)
        .context("parse EVAL_NOW")?
        .to_utc();
    let report = eval::run(now, SEEDS);
    let md = eval::to_markdown(&report);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/EVAL.md");
    std::fs::write(&path, &md).with_context(|| format!("write {}", path.display()))?;
    println!("{md}");
    Ok(())
}
