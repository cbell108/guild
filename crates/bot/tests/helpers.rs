//! Integration tests for the bot's message formatting helpers.
//!
//! Complements the unit tests that live next to `crates/bot/src/helpers.rs` by
//! exercising the helpers as a black box, the way the commands use them: the
//! roster text, the availability chart lines (including the "(overnight)"
//! suffix) and the gear list. All timestamps are pinned against
//! `America/New_York` (EST, UTC-5) instants in November 2026.
//!
//! The helpers are a module of the bot *binary* (there is no library target),
//! so this test compiles the file into the test crate by path instead of
//! importing it.

// Test code is allowed to panic on failure.
#![allow(clippy::unwrap_used)]

#[path = "../src/helpers.rs"]
mod helpers;

use chrono::{DateTime, Utc};
use hangouts_core::{Availability, Presence, Setup, UserId};

use helpers::{
    availability_confirmation, availability_from_input, availability_lines, discord_time,
    format_roster, gear_from_input, gear_lines, presence_label, presence_reply,
};

/// A single user for constructing `Availability` / `Gear` values.
fn user() -> UserId {
    UserId::from_u128(1)
}

/// The campus-local 2026-11-20 22:00–02:00 window: 2026-11-21 03:00Z–07:00Z.
fn overnight() -> Availability {
    availability_from_input(user(), "2026-11-20", Some("22:00"), Some("02:00")).unwrap()
}

/// The campus-local 2026-11-20 18:00–22:00 window: 2026-11-20 23:00Z–2026-11-21 03:00Z.
fn evening() -> Availability {
    availability_from_input(user(), "2026-11-20", Some("18:00"), Some("22:00")).unwrap()
}

#[test]
fn presence_labels_pin_prototype_wording() {
    assert_eq!(presence_label(Presence::Here), "in the gameroom");
    assert_eq!(presence_label(Presence::Interested), "interested");
    assert_eq!(presence_label(Presence::Out), "out");
}

#[test]
fn roster_groups_by_presence_and_keeps_input_order() {
    let roster = vec![
        ("Ana".into(), Presence::Here),
        ("Ben".into(), Presence::Out),
        ("Cy".into(), Presence::Here),
        ("Dee".into(), Presence::Interested),
        ("Eli".into(), Presence::Here),
    ];
    assert_eq!(
        format_roster(&roster),
        "**In the gameroom:** Ana, Cy, Eli\n**Interested:** Dee"
    );
}

#[test]
fn roster_marks_each_empty_category_separately() {
    let only_here = vec![("Ana".into(), Presence::Here)];
    assert_eq!(
        format_roster(&only_here),
        "**In the gameroom:** Ana\n**Interested:** (nobody yet)"
    );
    let nobody = vec![("Ben".into(), Presence::Out)];
    assert_eq!(
        format_roster(&nobody),
        "**In the gameroom:** (nobody yet)\n**Interested:** (nobody yet)"
    );
}

#[test]
fn presence_reply_pins_wording_for_each_presence() {
    let roster = vec![("Ana".into(), Presence::Here)];
    let base = "**In the gameroom:** Ana\n**Interested:** (nobody yet)";
    assert_eq!(
        presence_reply(Presence::Here, &roster),
        format!("You're now marked **in the gameroom**.\n{base}")
    );
    assert_eq!(
        presence_reply(Presence::Interested, &roster),
        format!("You're now marked **interested**.\n{base}")
    );
    assert_eq!(
        presence_reply(Presence::Out, &roster),
        format!("You're now marked **out**.\n{base}")
    );
}

#[test]
fn availability_lines_pin_discord_timestamps_and_overnight_flag() {
    assert_eq!(
        availability_lines(&[("Ana".into(), overnight())]),
        "- **Ana**: <t:1795230000:t> - <t:1795244400:t> (overnight)"
    );
}

#[test]
fn availability_lines_omit_overnight_for_same_day_windows() {
    assert_eq!(
        availability_lines(&[("Ana".into(), evening())]),
        "- **Ana**: <t:1795215600:t> - <t:1795230000:t>"
    );
}

#[test]
fn availability_lines_join_multiple_people_in_order() {
    assert_eq!(
        availability_lines(&[
            ("Ana".into(), overnight()),
            ("Ben".into(), evening()),
            ("Cy".into(), overnight()),
        ]),
        "- **Ana**: <t:1795230000:t> - <t:1795244400:t> (overnight)\n\
         - **Ben**: <t:1795215600:t> - <t:1795230000:t>\n\
         - **Cy**: <t:1795230000:t> - <t:1795244400:t> (overnight)"
    );
}

#[test]
fn availability_lines_empty_state() {
    assert_eq!(
        availability_lines(&[]),
        "Nobody else has availability set for this date yet."
    );
}

#[test]
fn window_ending_at_local_midnight_is_overnight() {
    // 23:00–00:00: the end rolls to local 2026-11-21 00:00 (2026-11-21 05:00Z).
    let a = availability_from_input(user(), "2026-11-20", Some("23:00"), Some("00:00")).unwrap();
    assert_eq!(
        availability_lines(&[("Ana".into(), a.clone())]),
        "- **Ana**: <t:1795233600:t> - <t:1795237200:t> (overnight)"
    );
    assert_eq!(
        availability_confirmation(&a),
        "You're set for **2026-11-20** (23:00 - 00:00)."
    );
}

#[test]
fn whole_day_window_is_not_overnight() {
    let a = availability_from_input(user(), "2026-11-20", None, None).unwrap();
    assert!(!a.window.crosses_midnight());
    assert_eq!(
        availability_lines(&[("Ana".into(), a)]),
        "- **Ana**: <t:1795150800:t> - <t:1795237140:t>"
    );
}

#[test]
fn availability_confirmation_pins_local_date_and_times() {
    assert_eq!(
        availability_confirmation(&overnight()),
        "You're set for **2026-11-20** (22:00 - 02:00)."
    );
    // Zero-padded dates are normalized on the way in (prototype fix ad75048).
    let a = availability_from_input(user(), "2026-1-5", Some("09:30"), Some("11:00")).unwrap();
    assert_eq!(
        availability_confirmation(&a),
        "You're set for **2026-01-05** (09:30 - 11:00)."
    );
}

#[test]
fn discord_time_uses_compact_timestamp_mark() {
    let at: DateTime<Utc> = DateTime::parse_from_rfc3339("2026-11-20T12:00:00Z")
        .unwrap()
        .to_utc();
    assert_eq!(discord_time(at), "<t:1795176000:t>");
}

#[test]
fn gear_lines_join_declarations_in_order() {
    let u = user();
    let console = gear_from_input(u, "CONSOLE", "Smash Ultimate").unwrap();
    let laptop = gear_from_input(u, " laptop ", "Hades II").unwrap();
    let other = gear_from_input(u, "Other", "Catan").unwrap();
    assert_eq!(
        gear_lines(&[
            ("Ana".into(), console),
            ("Ben".into(), laptop),
            ("Cy".into(), other),
        ]),
        "- **Ana**: console — Smash Ultimate\n- **Ben**: laptop — Hades II\n- **Cy**: other — Catan"
    );
}

#[test]
fn gear_lines_empty_state() {
    assert_eq!(gear_lines(&[]), "Nobody has declared gear yet.");
}

#[test]
fn gear_setup_normalization_is_case_and_space_insensitive() {
    for text in ["console", " Console ", "CONSOLE"] {
        let g = gear_from_input(user(), text, "Smash").unwrap();
        assert_eq!(g.setup, Setup::Console);
        assert_eq!(
            gear_lines(&[("Ana".into(), g)]),
            "- **Ana**: console — Smash"
        );
    }
}
