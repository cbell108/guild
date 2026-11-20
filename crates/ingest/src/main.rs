//! `cargo run -p ingest`: imports official KSU events into `DATABASE_URL`.

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://campus.db".to_owned());
    let store = store::Store::connect(&url).await.context("open database")?;
    store.migrate().await.context("migrate")?;
    let report = ingest::run(&store, &ingest::IngestConfig::default())
        .await
        .context("ingest")?;
    tracing::info!(
        fetched = report.fetched,
        upserted = report.upserted,
        "ingested"
    );
    Ok(())
}
