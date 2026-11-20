//! Slash commands and the button handler. Thin glue: validation and text live in
//! [`crate::helpers`], data in [`store`], ranking in [`recommend`].

use anyhow::Context as _;
use chrono::Utc;
use hangouts_core::{IntentLevel, UserId};
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use recommend::{AttendanceModel, Embedder, FeedConfig, RecContext};
use store::Store;

use crate::helpers::{
    self, BUTTON_HERE, BUTTON_INTERESTED, BUTTON_OUT, ButtonAction, HangoutInput,
};
use crate::service::{self, HangoutOutcome, Reply};

/// Shared state for every command.
pub struct Data {
    /// Database.
    pub store: Store,
    /// Base URL of the web app, for links in hangout posts.
    pub web_base: String,
    /// Attendance model trained at startup (prior if there was no history).
    pub model: AttendanceModel,
    /// Text embedder for `/foryou`.
    pub embedder: Box<dyn Embedder>,
}

/// Command error type.
pub type Error = anyhow::Error;
/// Command context.
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// Our user for the invoking Discord account (created on first use).
async fn caller(store: &Store, user: &serenity::User) -> Result<UserId, Error> {
    let name = helpers::display_name(user.global_name.as_deref(), &user.name);
    let u = store
        .upsert_discord_user(&user.id.to_string(), &name, Utc::now())
        .await
        .context("link discord user")?;
    Ok(u.id)
}

async fn send_reply(ctx: Context<'_>, reply: Reply) -> Result<(), Error> {
    let mut out = CreateReply::default()
        .content(reply.content)
        .ephemeral(true);
    if let Some((title, body)) = reply.embed {
        out = out.embed(serenity::CreateEmbed::new().title(title).description(body));
    }
    ctx.send(out).await?;
    Ok(())
}

async fn say_private(ctx: Context<'_>, text: impl Into<String>) -> Result<(), Error> {
    ctx.send(CreateReply::default().content(text).ephemeral(true))
        .await?;
    Ok(())
}

/// Open the live gameroom menu
#[poise::command(slash_command)]
pub async fn game(ctx: Context<'_>) -> Result<(), Error> {
    let row = serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new(BUTTON_HERE)
            .label("I'm in the gameroom")
            .style(serenity::ButtonStyle::Success),
        serenity::CreateButton::new(BUTTON_INTERESTED)
            .label("I'm interested")
            .style(serenity::ButtonStyle::Secondary),
        serenity::CreateButton::new(BUTTON_OUT)
            .label("I'm out")
            .style(serenity::ButtonStyle::Danger),
    ]);
    ctx.send(
        CreateReply::default()
            .content("Where are you at? Tap one:")
            .components(vec![row]),
    )
    .await?;
    Ok(())
}

/// Set and view the group's availability for a date
#[poise::command(slash_command)]
pub async fn availability(
    ctx: Context<'_>,
    #[description = "Date as YYYY-MM-DD"] date: String,
    #[description = "Start time as HH:MM (default 00:00)"] start: Option<String>,
    #[description = "End time as HH:MM (default 23:59)"] end: Option<String>,
) -> Result<(), Error> {
    let store = &ctx.data().store;
    let me = caller(store, ctx.author()).await?;
    let reply =
        service::set_availability(store, me, &date, start.as_deref(), end.as_deref()).await?;
    send_reply(ctx, reply).await
}

/// Declare the setup and game you bring to meetups
#[poise::command(slash_command)]
pub async fn gear(
    ctx: Context<'_>,
    #[description = "Your setup: console, laptop, or other"] setup: String,
    #[description = "A game you own with that setup"] game: String,
) -> Result<(), Error> {
    let store = &ctx.data().store;
    let me = caller(store, ctx.author()).await?;
    let reply = service::declare_gear(store, me, &setup, &game).await?;
    send_reply(ctx, reply).await
}

/// Host a hangout: posts it here, adds a Discord event, and shows it on the web app
#[poise::command(slash_command, guild_only)]
#[allow(clippy::too_many_arguments)] // one parameter per slash-command option
pub async fn hangout(
    ctx: Context<'_>,
    #[description = "Title"] title: String,
    #[description = "Date as YYYY-MM-DD"] date: String,
    #[description = "Start time as HH:MM"] start: String,
    #[description = "Where (building / room)"] location: String,
    #[description = "End time as HH:MM (default: 2 hours)"] end: Option<String>,
    #[description = "What's the plan?"] description: Option<String>,
    #[description = "Comma-separated tags"] tags: Option<String>,
    #[description = "Max people"]
    #[min = 1]
    capacity: Option<u32>,
) -> Result<(), Error> {
    let data = ctx.data();
    let me = caller(&data.store, ctx.author()).await?;
    let input = HangoutInput {
        title,
        date,
        start,
        end,
        location,
        description,
        tags,
        capacity,
    };
    let event = match service::create_hangout(&data.store, me, &input, Utc::now()).await? {
        HangoutOutcome::Created(e) => *e,
        HangoutOutcome::Rejected(text) => return say_private(ctx, text).await,
    };

    // The Discord scheduled event is a nice-to-have: missing permissions must not fail /hangout.
    if let Some(guild) = ctx.guild_id() {
        let start_ts = serenity::Timestamp::from_unix_timestamp(event.starts_at.timestamp())?;
        let end_ts = serenity::Timestamp::from_unix_timestamp(event.ends_at.timestamp())?;
        let builder = serenity::CreateScheduledEvent::new(
            serenity::ScheduledEventType::External,
            &event.title,
            start_ts,
        )
        .end_time(end_ts)
        .location(helpers::location_text(&event))
        .description(&event.description);
        if let Err(e) = guild.create_scheduled_event(ctx, builder).await {
            tracing::warn!(error = %e, "could not create Discord scheduled event");
        }
    }

    let buttons = serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new(helpers::intent_button_id(event.id, IntentLevel::Interested))
            .label("Interested")
            .style(serenity::ButtonStyle::Secondary),
        serenity::CreateButton::new(helpers::intent_button_id(event.id, IntentLevel::Going))
            .label("Going")
            .style(serenity::ButtonStyle::Success),
    ]);
    ctx.send(
        CreateReply::default()
            .content(helpers::hangout_post(&event, &data.web_base))
            .components(vec![buttons]),
    )
    .await?;
    Ok(())
}

/// Your top 3 picks for this week, with reasons (only you can see this)
#[poise::command(slash_command)]
pub async fn foryou(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();
    let me = caller(&data.store, ctx.author()).await?;
    let snapshot = data.store.load_snapshot().await?;
    let rec_ctx = RecContext {
        snapshot: &snapshot,
        user: me,
        now: Utc::now(),
        model: &data.model,
        embedder: data.embedder.as_ref(),
    };
    let recs = recommend::feed(
        &rec_ctx,
        &FeedConfig {
            limit: 3,
            ..FeedConfig::default()
        },
    );
    let text = helpers::foryou_lines(&recs, |id| {
        snapshot.event(id).map(|e| (e.title.clone(), e.starts_at))
    });
    say_private(ctx, text).await
}

/// Handles our buttons (`/game` presence menu, hangout intent buttons).
pub async fn on_component(
    ctx: &serenity::Context,
    data: &Data,
    mci: &serenity::ComponentInteraction,
) -> Result<(), Error> {
    let Some(action) = helpers::parse_button(&mci.data.custom_id) else {
        mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
            .await?;
        return Ok(());
    };
    let me = caller(&data.store, &mci.user).await?;
    let now = Utc::now();
    let text = match action {
        ButtonAction::Presence(p) => service::set_presence(&data.store, me, p, now).await?,
        ButtonAction::Intent { event, level } => {
            service::record_intent(&data.store, me, event, level, now).await?
        }
    };
    mci.create_response(
        ctx,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .content(text)
                .ephemeral(true),
        ),
    )
    .await?;
    Ok(())
}
