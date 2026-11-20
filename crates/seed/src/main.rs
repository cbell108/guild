//! `cargo run -p seed`: writes deterministic demo data to `DATABASE_URL`.

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://campus.db".to_owned());
    let store = store::Store::connect(&url).await.context("open database")?;
    store.migrate().await.context("migrate")?;
    let snapshot = seed::generate(&seed::SeedConfig::demo(chrono::Utc::now()));
    seed::write(&store, &snapshot)
        .await
        .context("write seed data")?;
    tracing::info!(
        users = snapshot.users.len(),
        events = snapshot.events.len(),
        "seeded"
    );
    Ok(())
}
