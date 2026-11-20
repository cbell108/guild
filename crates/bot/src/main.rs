//! `cargo run -p bot`: the poise Discord bot (`/game`, `/availability`, `/gear`, `/hangout`).
//! Reads `DISCORD_TOKEN` from the environment or `.env`.

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let token = std::env::var("DISCORD_TOKEN").unwrap_or_default();
    if token.trim().is_empty() {
        eprintln!("DISCORD_TOKEN is not set. Add it to .env to run the bot.");
        std::process::exit(1);
    }
    anyhow::bail!("the bot is not implemented yet (TASKS.md C10)")
}
