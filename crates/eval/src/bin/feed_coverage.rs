//! `cargo run -p eval --bin feed_coverage`: how many upcoming *official* events in each demo
//! user's "For you" feed get a specific reason (interests, history, friends, collaborative)
//! rather than only the generic popularity / "Coming up soon" one.
//!
//! Reads `DATABASE_URL` (seed it with `cargo run -p seed`, then `cargo run -p ingest`) and
//! prints a table. Not part of `docs/EVAL.md`: it needs real calendar data.

use anyhow::Context;
use chrono::{TimeDelta, Utc};
use hangouts_core::{ReasonKind, UserId};
use recommend::{AttendanceModel, FeedConfig, RecContext, default_embedder, feed};

/// Demo users sampled.
const USERS: usize = 5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://campus.db".to_owned());
    let store = store::Store::connect(&url).await.context("open database")?;
    let snapshot = store.load_snapshot().await.context("load snapshot")?;
    let users: Vec<UserId> = store
        .list_demo_users()
        .await?
        .into_iter()
        .take(USERS)
        .map(|u| u.id)
        .collect();
    let now = Utc::now();
    let model = AttendanceModel::prior();
    let embedder = default_embedder();
    let cfg = FeedConfig {
        horizon: TimeDelta::days(14),
        limit: usize::MAX,
        ..FeedConfig::default()
    };

    println!(
        "| user | official events | content | interest-tag | social | collab | generic only |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|");
    let mut total = [0_usize; 6];
    for user in users {
        let ctx = RecContext {
            snapshot: &snapshot,
            user,
            now,
            model: &model,
            embedder: embedder.as_ref(),
        };
        let mut row = [0_usize; 6];
        for rec in feed(&ctx, &cfg) {
            let is_official = snapshot
                .event(rec.event_id)
                .is_some_and(|e| e.external_id.is_some());
            if !is_official {
                continue;
            }
            let has = |k: ReasonKind| rec.reasons.iter().any(|r| r.kind == k);
            row[0] += 1;
            row[1] += usize::from(has(ReasonKind::Content));
            row[2] += usize::from(
                rec.reasons
                    .iter()
                    .any(|r| r.text.starts_with("Because you like")),
            );
            row[3] += usize::from(has(ReasonKind::Social));
            row[4] += usize::from(has(ReasonKind::Collaborative));
            row[5] += usize::from(rec.reasons.iter().all(|r| r.kind == ReasonKind::Popular));
        }
        let name = snapshot.user(user).map_or("?", |u| u.display_name.as_str());
        println!(
            "| {name} | {} | {} | {} | {} | {} | {} |",
            row[0], row[1], row[2], row[3], row[4], row[5]
        );
        for (t, r) in total.iter_mut().zip(row) {
            *t += r;
        }
    }
    println!(
        "| **total** | {} | {} | {} | {} | {} | {} ({:.0}%) |",
        total[0],
        total[1],
        total[2],
        total[3],
        total[4],
        total[5],
        100.0 * total[5] as f64 / total[0].max(1) as f64
    );
    Ok(())
}
