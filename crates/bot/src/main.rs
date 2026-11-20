//! `cargo run -p bot`: the poise Discord bot (`/game`, `/availability`, `/gear`, `/hangout`,
//! `/foryou`). Reads `DISCORD_TOKEN`, `DATABASE_URL` and `WEB_BASE_URL` from the environment
//! or `.env`. Uses only non-privileged gateway intents (presence is self-reported).

mod commands;
mod helpers;

use anyhow::Context as _;
use poise::serenity_prelude as serenity;

use commands::{Data, Error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN").unwrap_or_default();
    if token.trim().is_empty() {
        eprintln!("DISCORD_TOKEN is not set. Add it to .env to run the bot.");
        std::process::exit(1);
    }
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://campus.db".to_owned());
    let web_base =
        std::env::var("WEB_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_owned());

    let store = store::Store::connect(&db_url)
        .await
        .context("open database")?;
    store.migrate().await.context("migrate")?;
    let now = chrono::Utc::now();
    let model = match store.load_snapshot().await {
        Ok(s) => recommend::AttendanceModel::train(&s, now),
        Err(e) => {
            tracing::warn!(error = %e, "no snapshot; using the prior attendance model");
            recommend::AttendanceModel::prior()
        }
    };
    let data = Data {
        store,
        web_base,
        model,
        embedder: recommend::default_embedder(),
    };

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::game(),
                commands::availability(),
                commands::gear(),
                commands::hangout(),
                commands::foryou(),
            ],
            event_handler: |framework, event| Box::pin(on_event(framework, event)),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(data)
            })
        })
        .build();

    let mut client =
        serenity::ClientBuilder::new(token, serenity::GatewayIntents::non_privileged())
            .framework(framework)
            .await
            .context("build Discord client")?;
    client.start().await.context("run Discord client")?;
    Ok(())
}

async fn on_event(
    framework: poise::FrameworkContext<'_, Data, Error>,
    event: &serenity::FullEvent,
) -> Result<(), Error> {
    let (ctx, data) = (framework.serenity_context, framework.user_data);
    if let serenity::FullEvent::InteractionCreate {
        interaction: serenity::Interaction::Component(mci),
    } = event
        && let Err(e) = commands::on_component(ctx, data, mci).await
    {
        tracing::error!(error = %e, custom_id = %mci.data.custom_id, "button failed");
    }
    Ok(())
}
