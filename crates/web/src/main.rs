//! `cargo run -p web`: serves the API and `static/` on `BIND_ADDR` (default 127.0.0.1:3000).

use std::sync::Arc;

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://campus.db".to_owned());
    let bind = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());

    let store = store::Store::connect(&db_url)
        .await
        .context("open database")?;
    store.migrate().await.context("migrate")?;
    let llm = llm::LlmClient::new(llm::LlmConfig::from_env()).context("LLM client")?;
    let state = web::AppState::new(store, Arc::new(llm));
    let embedded = web::embedding::backfill(&state).await;
    tracing::info!("embedded {embedded} events");
    state
        .retrain_model()
        .await
        .context("train attendance model")?;
    let app = web::app(state);

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("bind {bind}"))?;
    tracing::info!("listening on http://{bind}");
    axum::serve(listener, app).await.context("serve")
}
