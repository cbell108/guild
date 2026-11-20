//! Pure helpers behind the bot commands: button ids, reply text and input validation.
//! Everything here is unit-tested; nothing talks to Discord or the database.

use chrono::{DateTime, Utc};
use hangouts_core::time::{format_date, format_time, local_date, parse_local_window, to_local};
use hangouts_core::{
    Availability, DomainError, Event, EventId, Gear, HangoutSpec, IntentLevel, Location, Presence,
    Recommendation, Setup, TimeError, UserId,
};

/// Button id for "I'm in the gameroom" (same ids as the prototype).
pub const BUTTON_HERE: &str = "live_in";
/// Button id for "I'm interested".
pub const BUTTON_INTERESTED: &str = "live_interest";
/// Button id for "I'm out".
pub const BUTTON_OUT: &str = "live_out";
/// Prefix of intent buttons on hangout posts: `intent:<level>:<event id>`.
pub const INTENT_PREFIX: &str = "intent:";

/// What a clicked button means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    /// A `/game` menu button.
    Presence(Presence),
    /// An Interested / Going button on a hangout post.
    Intent {
        /// The hangout.
        event: EventId,
        /// The level to record.
        level: IntentLevel,
    },
}

/// Parses a component custom id; `None` for buttons that are not ours.
pub fn parse_button(custom_id: &str) -> Option<ButtonAction> {
    match custom_id {
        BUTTON_HERE => return Some(ButtonAction::Presence(Presence::Here)),
        BUTTON_INTERESTED => return Some(ButtonAction::Presence(Presence::Interested)),
        BUTTON_OUT => return Some(ButtonAction::Presence(Presence::Out)),
        _ => {}
    }
    let rest = custom_id.strip_prefix(INTENT_PREFIX)?;
    let (level, id) = rest.split_once(':')?;
    let level = match level {
        "interested" => IntentLevel::Interested,
        "going" => IntentLevel::Going,
        _ => return None,
    };
    Some(ButtonAction::Intent {
        event: id.parse().ok()?,
        level,
    })
}

/// The custom id for an intent button (Going with ___ needs a person, so it lives on the web).
pub fn intent_button_id(event: EventId, level: IntentLevel) -> String {
    let level = match level {
        IntentLevel::Interested => "interested",
        IntentLevel::Going | IntentLevel::GoingWith(_) => "going",
    };
    format!("{INTENT_PREFIX}{level}:{event}")
}

/// Past-tense wording for a presence, as in the prototype ("in the gameroom").
pub fn presence_label(p: Presence) -> &'static str {
    match p {
        Presence::Here => "in the gameroom",
        Presence::Interested => "interested",
        Presence::Out => "out",
    }
}

/// The roster shown after a presence click: who is here and who is interested.
pub fn format_roster(entries: &[(String, Presence)]) -> String {
    let names = |want: Presence| {
        let list: Vec<&str> = entries
            .iter()
            .filter(|(_, p)| *p == want)
            .map(|(n, _)| n.as_str())
            .collect();
        if list.is_empty() {
            "(nobody yet)".to_owned()
        } else {
            list.join(", ")
        }
    };
    format!(
        "**In the gameroom:** {}\n**Interested:** {}",
        names(Presence::Here),
        names(Presence::Interested)
    )
}

/// The reply after a presence click.
pub fn presence_reply(p: Presence, roster: &[(String, Presence)]) -> String {
    format!(
        "You're now marked **{}**.\n{}",
        presence_label(p),
        format_roster(roster)
    )
}

/// Validates `/availability` input with the strict prototype rules; errors are user-facing
/// ("Invalid start time, expected HH:MM.").
pub fn availability_from_input(
    user: UserId,
    date: &str,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<Availability, TimeError> {
    Ok(Availability {
        user_id: user,
        window: parse_local_window(date, start, end)?,
    })
}

/// Discord timestamp markup (`<t:…:t>`), rendered in each reader's own time zone.
pub fn discord_time(at: DateTime<Utc>) -> String {
    format!("<t:{}:t>", at.timestamp())
}

/// "You're set for **2026-11-20** (18:00 - 22:00)." — in campus-local time, zero-padded.
pub fn availability_confirmation(a: &Availability) -> String {
    let start = to_local(a.window.start());
    let end = to_local(a.window.end());
    format!(
        "You're set for **{}** ({} - {}).",
        format_date(local_date(a.window.start())),
        format_time(start.time()),
        format_time(end.time())
    )
}

/// One line per window: "- **Name**: <t:…> - <t:…> (overnight)".
pub fn availability_lines(entries: &[(String, Availability)]) -> String {
    if entries.is_empty() {
        return "Nobody else has availability set for this date yet.".to_owned();
    }
    entries
        .iter()
        .map(|(name, a)| {
            let overnight = if a.window.crosses_midnight() {
                " (overnight)"
            } else {
                ""
            };
            format!(
                "- **{name}**: {} - {}{overnight}",
                discord_time(a.window.start()),
                discord_time(a.window.end())
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Validates `/gear` input: setup trimmed and case-insensitive, game non-blank.
pub fn gear_from_input(user: UserId, setup: &str, game: &str) -> Result<Gear, DomainError> {
    Gear::new(user, Setup::parse(setup)?, game)
}

/// The prototype's user-facing message for a bad `/gear` input.
pub fn gear_error_text(e: &DomainError) -> &'static str {
    match e {
        DomainError::Invalid { field: "game", .. } => {
            "Invalid game name, expected a non-empty title."
        }
        _ => "Invalid setup, expected console, laptop, or other.",
    }
}

/// One line per declaration: "- **Name**: console — Smash".
pub fn gear_lines(entries: &[(String, Gear)]) -> String {
    if entries.is_empty() {
        return "Nobody has declared gear yet.".to_owned();
    }
    entries
        .iter()
        .map(|(name, g)| format!("- **{name}**: {} — {}", g.setup.as_str(), g.game))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `/hangout` options as typed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HangoutInput {
    /// Title.
    pub title: String,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// `HH:MM`.
    pub start: String,
    /// `HH:MM`, optional.
    pub end: Option<String>,
    /// Venue.
    pub location: String,
    /// Optional description.
    pub description: Option<String>,
    /// Comma-separated tags.
    pub tags: Option<String>,
    /// Optional capacity.
    pub capacity: Option<u32>,
}

impl HangoutInput {
    /// The core spec (tags split on commas; validation happens in `build_hangout`).
    pub fn to_spec(&self) -> HangoutSpec {
        HangoutSpec {
            title: self.title.clone(),
            description: self.description.clone().unwrap_or_default(),
            date: self.date.clone(),
            start: self.start.clone(),
            end: self.end.clone(),
            location: Location {
                name: self.location.clone(),
                ..Location::default()
            },
            tags: self
                .tags
                .as_deref()
                .unwrap_or("")
                .split(',')
                .map(str::to_owned)
                .collect(),
            capacity: self.capacity,
            crew_id: None,
        }
    }
}

/// The location string for a Discord scheduled event ("Student Center, Game Room").
pub fn location_text(event: &Event) -> String {
    match &event.location.room {
        Some(room) => format!("{}, {room}", event.location.name),
        None => event.location.name.clone(),
    }
}

/// The public post for a new hangout. No attendee counts (privacy defaults).
pub fn hangout_post(event: &Event, web_base: &str) -> String {
    use std::fmt::Write as _;
    let mut text = format!(
        "**{}**\n<t:{}:F> – {} · {}",
        event.title,
        event.starts_at.timestamp(),
        discord_time(event.ends_at),
        location_text(event)
    );
    if let Some(cap) = event.capacity {
        let _ = write!(text, " · {cap} spots");
    }
    if !event.description.is_empty() {
        text.push('\n');
        text.push_str(&event.description);
    }
    if !event.tags.is_empty() {
        let tags: Vec<String> = event.tags.iter().map(|t| format!("`{t}`")).collect();
        text.push('\n');
        text.push_str(&tags.join(" "));
    }
    let _ = write!(
        text,
        "\n{}/#/event/{}",
        web_base.trim_end_matches('/'),
        event.id
    );
    text
}

/// Reply after an intent button.
pub fn intent_reply(event_title: &str, level: IntentLevel, waitlisted: bool) -> String {
    match (level, waitlisted) {
        (IntentLevel::Interested, _) => format!("Marked **interested** in {event_title}."),
        (_, true) => format!(
            "{event_title} is full, so you're on the **waitlist**. You'll get the next spot."
        ),
        _ => format!("You're **going** to {event_title}. See you there!"),
    }
}

/// Reasons shown per `/foryou` pick.
pub const FORYOU_REASONS: usize = 2;

/// `/foryou`: the top picks as numbered lines with their top reasons (ephemeral reply).
pub fn foryou_lines(
    recs: &[Recommendation],
    title_of: impl Fn(EventId) -> Option<(String, DateTime<Utc>)>,
) -> String {
    let lines: Vec<String> = recs
        .iter()
        .filter_map(|r| {
            let (title, at) = title_of(r.event_id)?;
            let reasons: Vec<&str> = r
                .reasons
                .iter()
                .take(FORYOU_REASONS)
                .map(|x| x.text.as_str())
                .collect();
            let reason = if reasons.is_empty() {
                "Coming up soon".to_owned()
            } else {
                reasons.join(" · ")
            };
            Some(format!(
                "**{title}** · <t:{}:R>\n  _{reason}_",
                at.timestamp()
            ))
        })
        .collect();
    if lines.is_empty() {
        "You're caught up. Nothing new this week.".to_owned()
    } else {
        lines
            .iter()
            .enumerate()
            .map(|(i, l)| format!("{}. {l}", i + 1))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// The best name we have for a Discord user: global display name, else username.
pub fn display_name(global_name: Option<&str>, username: &str) -> String {
    global_name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(username)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hangouts_core::{Reason, ReasonKind, ScoreBreakdown, build_hangout};

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-19T17:00:00Z")
            .unwrap()
            .to_utc()
    }

    #[test]
    fn button_ids_round_trip() {
        assert_eq!(
            parse_button("live_in"),
            Some(ButtonAction::Presence(Presence::Here))
        );
        assert_eq!(
            parse_button("live_out"),
            Some(ButtonAction::Presence(Presence::Out))
        );
        let e = EventId::from_u128(42);
        for level in [IntentLevel::Interested, IntentLevel::Going] {
            assert_eq!(
                parse_button(&intent_button_id(e, level)),
                Some(ButtonAction::Intent { event: e, level })
            );
        }
        assert_eq!(parse_button("intent:maybe:x"), None);
        assert_eq!(parse_button("intent:going:not-a-uuid"), None);
        assert_eq!(parse_button("something_else"), None);
    }

    #[test]
    fn roster_matches_prototype_wording() {
        assert_eq!(
            format_roster(&[]),
            "**In the gameroom:** (nobody yet)\n**Interested:** (nobody yet)"
        );
        let roster = vec![
            ("Ana".to_owned(), Presence::Here),
            ("Ben".to_owned(), Presence::Interested),
            ("Cy".to_owned(), Presence::Here),
        ];
        assert_eq!(
            presence_reply(Presence::Here, &roster),
            "You're now marked **in the gameroom**.\n**In the gameroom:** Ana, Cy\n**Interested:** Ben"
        );
    }

    #[test]
    fn availability_validation_and_text() {
        let u = UserId::from_u128(1);
        let a = availability_from_input(u, "2026-1-5", Some("22:00"), Some("02:00")).unwrap();
        assert_eq!(
            availability_confirmation(&a),
            "You're set for **2026-01-05** (22:00 - 02:00)."
        );
        assert!(availability_lines(&[("Ana".to_owned(), a)]).ends_with("(overnight)"));
        let whole = availability_from_input(u, "2026-11-20", None, None).unwrap();
        assert_eq!(
            availability_confirmation(&whole),
            "You're set for **2026-11-20** (00:00 - 23:59)."
        );
        assert_eq!(
            availability_from_input(u, "2026-02-30", None, None),
            Err(TimeError::InvalidDate)
        );
        assert_eq!(
            availability_from_input(u, "2026-11-20", Some("7pm"), None),
            Err(TimeError::InvalidStart)
        );
        assert_eq!(
            availability_from_input(u, "2026-11-20", None, Some("25:00")),
            Err(TimeError::InvalidEnd)
        );
        assert_eq!(
            TimeError::InvalidStart.to_string(),
            "Invalid start time, expected HH:MM."
        );
    }

    #[test]
    fn gear_validation_and_text() {
        let u = UserId::from_u128(1);
        let g = gear_from_input(u, " Console ", " Smash ").unwrap();
        assert_eq!(g.setup, Setup::Console);
        assert_eq!(
            gear_lines(&[("Ana".to_owned(), g)]),
            "- **Ana**: console — Smash"
        );
        let bad_setup = gear_from_input(u, "phone", "Smash").unwrap_err();
        assert_eq!(
            gear_error_text(&bad_setup),
            "Invalid setup, expected console, laptop, or other."
        );
        let blank = gear_from_input(u, "laptop", "  ").unwrap_err();
        assert_eq!(
            gear_error_text(&blank),
            "Invalid game name, expected a non-empty title."
        );
    }

    #[test]
    fn hangout_input_builds_a_valid_event_and_post() {
        let input = HangoutInput {
            title: "Smash night".into(),
            date: "2026-11-20".into(),
            start: "19:00".into(),
            end: Some("22:00".into()),
            location: "Student Center".into(),
            description: Some("Bring a controller".into()),
            tags: Some("Smash Bros, gaming".into()),
            capacity: Some(8),
        };
        let e = build_hangout(
            &input.to_spec(),
            UserId::from_u128(1),
            EventId::from_u128(2),
            now(),
        )
        .unwrap();
        assert_eq!(e.tags, vec!["smash-bros", "gaming"]);
        let post = hangout_post(&e, "http://127.0.0.1:3000/");
        assert!(post.starts_with("**Smash night**\n<t:"));
        assert!(post.contains("Student Center · 8 spots"));
        assert!(post.contains("`smash-bros` `gaming`"));
        assert!(post.ends_with(&format!("http://127.0.0.1:3000/#/event/{}", e.id)));
        assert_eq!(location_text(&e), "Student Center");
    }

    #[test]
    fn intent_and_foryou_text() {
        assert!(intent_reply("Smash", IntentLevel::Going, true).contains("waitlist"));
        assert!(intent_reply("Smash", IntentLevel::Going, false).contains("going"));
        let rec = |n| Recommendation {
            event_id: EventId::from_u128(n),
            score: 1.0,
            reasons: vec![
                Reason {
                    kind: ReasonKind::Content,
                    text: "Because you like board games".into(),
                },
                Reason {
                    kind: ReasonKind::Social,
                    text: "Carl is going".into(),
                },
                Reason {
                    kind: ReasonKind::Popular,
                    text: "Popular on campus this week".into(),
                },
            ],
            breakdown: ScoreBreakdown::default(),
        };
        let text = foryou_lines(&[rec(1), rec(2)], |id| {
            (id == EventId::from_u128(1)).then(|| ("Tabletop Tuesday".to_owned(), now()))
        });
        assert_eq!(
            text,
            format!(
                "1. **Tabletop Tuesday** · <t:{}:R>\n  _Because you like board games · Carl is going_",
                now().timestamp()
            )
        );
        assert!(foryou_lines(&[], |_| None).contains("caught up"));
    }

    #[test]
    fn names() {
        assert_eq!(display_name(Some(" Ana "), "ana_42"), "Ana");
        assert_eq!(display_name(Some(""), "ana_42"), "ana_42");
        assert_eq!(display_name(None, "ana_42"), "ana_42");
    }
}
