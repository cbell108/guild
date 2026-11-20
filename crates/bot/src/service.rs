//! The store-backed logic behind each command, separated from Discord so it can be tested
//! against an in-memory store. Every function returns the text the command should send.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use hangouts_core::time::{format_date, local_date};
use hangouts_core::{Event, EventId, IntentLevel, IntentStatus, Presence, UserId, build_hangout};
use store::Store;

use crate::helpers::{self, HangoutInput};

/// A reply with optional embed (title, body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// Message text.
    pub content: String,
    /// Embed `(title, description)`, if any.
    pub embed: Option<(String, String)>,
}

impl Reply {
    fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            embed: None,
        }
    }
}

/// What `/hangout` did.
#[derive(Debug, Clone, PartialEq)]
pub enum HangoutOutcome {
    /// Input was invalid; the text is the user-facing error. Nothing was stored.
    Rejected(String),
    /// The hangout was stored.
    Created(Box<Event>),
}

async fn names(store: &Store) -> Result<BTreeMap<UserId, String>, store::StoreError> {
    Ok(store
        .list_users()
        .await?
        .into_iter()
        .map(|u| (u.id, u.display_name))
        .collect())
}

/// `/availability`: validates, saves the caller's window, lists everyone's for that date.
pub async fn set_availability(
    store: &Store,
    me: UserId,
    date: &str,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<Reply, store::StoreError> {
    let entry = match helpers::availability_from_input(me, date, start, end) {
        Ok(a) => a,
        Err(e) => return Ok(Reply::text(e.to_string())),
    };
    store.set_availability(&entry).await?;
    let day = local_date(entry.window.start());
    let names = names(store).await?;
    let others: Vec<(String, _)> = store
        .list_availability_on(day, None)
        .await?
        .into_iter()
        .map(|a| (names.get(&a.user_id).cloned().unwrap_or_default(), a))
        .collect();
    Ok(Reply {
        content: helpers::availability_confirmation(&entry),
        embed: Some((
            format!("Availability for {}", format_date(day)),
            helpers::availability_lines(&others),
        )),
    })
}

/// `/gear`: validates, saves the declaration, lists everyone's gear.
pub async fn declare_gear(
    store: &Store,
    me: UserId,
    setup: &str,
    game: &str,
) -> Result<Reply, store::StoreError> {
    let declared = match helpers::gear_from_input(me, setup, game) {
        Ok(g) => g,
        Err(e) => return Ok(Reply::text(helpers::gear_error_text(&e))),
    };
    store.add_gear(&declared).await?;
    let names = names(store).await?;
    let all: Vec<(String, _)> = store
        .list_gear()
        .await?
        .into_iter()
        .map(|g| (names.get(&g.user_id).cloned().unwrap_or_default(), g))
        .collect();
    Ok(Reply {
        content: format!(
            "You're now marked with **{}** and **{}**.",
            declared.setup.as_str(),
            declared.game
        ),
        embed: Some(("Group gear".to_owned(), helpers::gear_lines(&all))),
    })
}

/// A `/game` menu click: records the caller's presence and returns the roster reply.
pub async fn set_presence(
    store: &Store,
    me: UserId,
    presence: Presence,
    now: DateTime<Utc>,
) -> Result<String, store::StoreError> {
    store.set_presence(me, presence, now).await?;
    let names = names(store).await?;
    let roster: Vec<(String, _)> = store
        .list_presence()
        .await?
        .into_iter()
        .map(|r| {
            (
                names.get(&r.user_id).cloned().unwrap_or_default(),
                r.presence,
            )
        })
        .collect();
    Ok(helpers::presence_reply(presence, &roster))
}

/// `/hangout`: builds and stores the event, or returns the validation error text.
pub async fn create_hangout(
    store: &Store,
    me: UserId,
    input: &HangoutInput,
    now: DateTime<Utc>,
) -> Result<HangoutOutcome, store::StoreError> {
    match build_hangout(&input.to_spec(), me, EventId::new(), now) {
        Err(e) => Ok(HangoutOutcome::Rejected(e.to_string())),
        Ok(event) => {
            store.insert_event(&event).await?;
            Ok(HangoutOutcome::Created(Box::new(event)))
        }
    }
}

/// An Interested / Going button on a hangout post.
pub async fn record_intent(
    store: &Store,
    me: UserId,
    event: EventId,
    level: IntentLevel,
    now: DateTime<Utc>,
) -> Result<String, store::StoreError> {
    Ok(match store.get_event(event).await? {
        None => "That hangout no longer exists.".to_owned(),
        Some(e) => {
            let intent = store.set_intent(me, event, level, now).await?;
            helpers::intent_reply(&e.title, level, intent.status == IntentStatus::Waitlisted)
        }
    })
}

#[cfg(test)]
mod tests {
    // Test code is allowed to panic on failure.
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-19T17:00:00Z")
            .unwrap()
            .to_utc()
    }

    async fn store_with_users() -> (Store, UserId, UserId) {
        let store = Store::in_memory().await.unwrap();
        store.migrate().await.unwrap();
        let a = store.upsert_discord_user("1", "Ada", now()).await.unwrap();
        let b = store.upsert_discord_user("2", "Bo", now()).await.unwrap();
        (store, a.id, b.id)
    }

    fn input() -> HangoutInput {
        HangoutInput {
            title: "Smash night".into(),
            date: "2026-11-25".into(),
            start: "19:00".into(),
            end: None,
            location: "Student Center".into(),
            description: Some("Bring a controller".into()),
            tags: Some("gaming, Smash".into()),
            capacity: Some(1),
        }
    }

    #[tokio::test]
    async fn availability_saves_and_lists_everyone() {
        let (store, ada, bo) = store_with_users().await;
        set_availability(&store, ada, "2026-11-20", Some("18:00"), Some("22:00"))
            .await
            .unwrap();
        let reply = set_availability(&store, bo, "2026-11-20", None, None)
            .await
            .unwrap();
        assert_eq!(
            reply.content,
            "You're set for **2026-11-20** (00:00 - 23:59)."
        );
        let (title, body) = reply.embed.unwrap();
        assert_eq!(title, "Availability for 2026-11-20");
        assert!(
            body.contains("**Ada**") && body.contains("**Bo**"),
            "{body}"
        );
    }

    #[tokio::test]
    async fn availability_replaces_same_day_window() {
        let (store, ada, _) = store_with_users().await;
        set_availability(&store, ada, "2026-11-20", Some("09:00"), Some("10:00"))
            .await
            .unwrap();
        let reply = set_availability(&store, ada, "2026-11-20", Some("18:00"), Some("22:00"))
            .await
            .unwrap();
        let (_, body) = reply.embed.unwrap();
        assert_eq!(body.matches("**Ada**").count(), 1, "{body}");
    }

    #[tokio::test]
    async fn availability_overnight_rolls_into_next_day() {
        let (store, ada, _) = store_with_users().await;
        let reply = set_availability(&store, ada, "2026-11-20", Some("22:00"), Some("02:00"))
            .await
            .unwrap();
        assert_eq!(
            reply.content,
            "You're set for **2026-11-20** (22:00 - 02:00)."
        );
        let (_, body) = reply.embed.unwrap();
        assert!(body.contains("(overnight)"), "{body}");
        // The window also shows up when looking at the next local day.
        let next = hangouts_core::time::parse_date("2026-11-21").unwrap();
        assert_eq!(
            store.list_availability_on(next, None).await.unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn availability_overnight_across_dst_change_keeps_wall_clock() {
        // US clocks fall back on 2026-11-01: 22:00 -> 02:00 is five real hours, not four.
        let (store, ada, _) = store_with_users().await;
        let reply = set_availability(&store, ada, "2026-10-31", Some("22:00"), Some("02:00"))
            .await
            .unwrap();
        assert_eq!(
            reply.content,
            "You're set for **2026-10-31** (22:00 - 02:00)."
        );
    }

    #[tokio::test]
    async fn availability_normalizes_one_digit_dates_to_zero_padded() {
        let (store, ada, _) = store_with_users().await;
        let reply = set_availability(&store, ada, "2026-12-5", Some("9:00"), Some("10:30"))
            .await
            .unwrap();
        assert_eq!(
            reply.content,
            "You're set for **2026-12-05** (09:00 - 10:30)."
        );
    }

    #[tokio::test]
    async fn availability_rejects_bad_input_without_storing() {
        let (store, ada, _) = store_with_users().await;
        for (date, start, end) in [
            ("2026-02-30", None, None),
            ("2026-13-01", None, None),
            ("not a date", None, None),
            ("2026-11-20", Some("6pm"), None),
            ("2026-11-20", Some("18:00"), Some("25:00")),
        ] {
            let reply = set_availability(&store, ada, date, start, end)
                .await
                .unwrap();
            assert!(reply.embed.is_none(), "{date} {start:?} {end:?}");
            assert!(reply.content.starts_with("Invalid"), "{}", reply.content);
        }
        let day = hangouts_core::time::parse_date("2026-11-20").unwrap();
        assert_eq!(
            store.list_availability_on(day, None).await.unwrap().len(),
            0
        );
        let r = set_availability(&store, ada, "2026-11-20", Some("6pm"), None)
            .await
            .unwrap();
        assert_eq!(r.content, "Invalid start time, expected HH:MM.");
        let r = set_availability(&store, ada, "2026-11-20", None, Some("99:99"))
            .await
            .unwrap();
        assert_eq!(r.content, "Invalid end time, expected HH:MM.");
    }

    #[tokio::test]
    async fn gear_declares_and_lists() {
        let (store, ada, bo) = store_with_users().await;
        declare_gear(&store, ada, "  Console ", "Smash")
            .await
            .unwrap();
        // Declaring the same thing twice is idempotent.
        declare_gear(&store, ada, "console", "Smash").await.unwrap();
        let reply = declare_gear(&store, bo, "laptop", "Celeste").await.unwrap();
        assert_eq!(
            reply.content,
            "You're now marked with **laptop** and **Celeste**."
        );
        let (title, body) = reply.embed.unwrap();
        assert_eq!(title, "Group gear");
        assert_eq!(body.matches("**Ada**").count(), 1, "{body}");
        assert!(body.contains("- **Bo**: laptop — Celeste"), "{body}");
    }

    #[tokio::test]
    async fn gear_rejects_bad_input() {
        let (store, ada, _) = store_with_users().await;
        let r = declare_gear(&store, ada, "toaster", "Smash").await.unwrap();
        assert_eq!(
            r.content,
            "Invalid setup, expected console, laptop, or other."
        );
        let r = declare_gear(&store, ada, "console", "   ").await.unwrap();
        assert_eq!(r.content, "Invalid game name, expected a non-empty title.");
        assert_eq!(store.list_gear().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn presence_updates_roster() {
        let (store, ada, bo) = store_with_users().await;
        set_presence(&store, ada, Presence::Here, now())
            .await
            .unwrap();
        let text = set_presence(&store, bo, Presence::Interested, now())
            .await
            .unwrap();
        assert!(text.contains("You're now marked **interested**."), "{text}");
        assert!(text.contains("**In the gameroom:** Ada"), "{text}");
        assert!(text.contains("**Interested:** Bo"), "{text}");
        // Changing your mind moves you, it doesn't duplicate you.
        let text = set_presence(&store, ada, Presence::Out, now())
            .await
            .unwrap();
        assert!(text.contains("**In the gameroom:** (nobody yet)"), "{text}");
    }

    #[tokio::test]
    async fn hangout_is_created_and_stored() {
        let (store, ada, _) = store_with_users().await;
        let HangoutOutcome::Created(event) =
            create_hangout(&store, ada, &input(), now()).await.unwrap()
        else {
            panic!("expected a created hangout");
        };
        assert_eq!(event.host_id, Some(ada));
        assert_eq!(event.capacity, Some(1));
        // Default end is two hours after the start.
        assert_eq!(event.ends_at - event.starts_at, chrono::TimeDelta::hours(2));
        let stored = store.get_event(event.id).await.unwrap().unwrap();
        assert_eq!(stored, *event);
    }

    #[tokio::test]
    async fn hangout_overnight_end_rolls_to_next_day() {
        let (store, ada, _) = store_with_users().await;
        let mut i = input();
        i.start = "23:00".into();
        i.end = Some("01:00".into());
        let HangoutOutcome::Created(event) = create_hangout(&store, ada, &i, now()).await.unwrap()
        else {
            panic!("expected a created hangout");
        };
        assert_eq!(event.ends_at - event.starts_at, chrono::TimeDelta::hours(2));
    }

    #[tokio::test]
    async fn hangout_rejects_bad_input_without_storing() {
        let (store, ada, _) = store_with_users().await;
        for mutate in [
            (|i: &mut HangoutInput| i.date = "2026-02-30".into()) as fn(&mut HangoutInput),
            |i| i.start = "7pm".into(),
            |i| i.end = Some("26:00".into()),
            |i| i.title = "  ".into(),
            |i| i.location = String::new(),
        ] {
            let mut i = input();
            mutate(&mut i);
            let out = create_hangout(&store, ada, &i, now()).await.unwrap();
            let HangoutOutcome::Rejected(text) = out else {
                panic!("expected rejection for {i:?}");
            };
            assert_ne!(text, "");
        }
        assert_eq!(store.load_snapshot().await.unwrap().events.len(), 0);
    }

    #[tokio::test]
    async fn intent_buttons_confirm_then_waitlist() {
        let (store, ada, bo) = store_with_users().await;
        let HangoutOutcome::Created(event) =
            create_hangout(&store, ada, &input(), now()).await.unwrap()
        else {
            panic!("expected a created hangout");
        };
        let r = record_intent(&store, ada, event.id, IntentLevel::Going, now())
            .await
            .unwrap();
        assert_eq!(r, "You're **going** to Smash night. See you there!");
        // Capacity is 1, so the second person is waitlisted.
        let r = record_intent(&store, bo, event.id, IntentLevel::Going, now())
            .await
            .unwrap();
        assert!(r.contains("**waitlist**"), "{r}");
        let r = record_intent(&store, bo, event.id, IntentLevel::Interested, now())
            .await
            .unwrap();
        assert_eq!(r, "Marked **interested** in Smash night.");
        let r = record_intent(&store, bo, EventId::new(), IntentLevel::Going, now())
            .await
            .unwrap();
        assert_eq!(r, "That hangout no longer exists.");
    }
}
