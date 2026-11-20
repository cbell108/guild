//! Campus-local time handling, ported from the DiscordSchedulerBot prototype.
//!
//! Instants are stored as `DateTime<Utc>`; people type and read wall-clock times in
//! [`TZ`] (America/New_York). The prototype shipped three fixes that are kept as rules here,
//! each with tests below:
//!
//! 1. **Strict parsing** (`a8443b3`): `std::get_time` accepted trailing junk (`2026-10-31x`),
//!    impossible dates (`2026-02-30`, `2025-02-29`), and out-of-range times (`25:99`, `23:60`).
//!    [`parse_date`] and [`parse_time`] reject all of these.
//! 2. **Zero-padded dates** (`ad75048`): `2026-1-1` is accepted but always normalized to
//!    `2026-01-01` by [`format_date`], so lookups by date string match stored rows.
//! 3. **DST-safe overnight rollover** (`1a1fda4`): when a window's end is not after its start,
//!    the end moves to the *next calendar day* and is resolved in the local zone again. Adding a
//!    fixed 86 400 s is wrong on transition days (those days are 23 h or 25 h long). Local
//!    midnight is also resolved by the zone, never assumed to be at a fixed UTC offset.

use chrono::{
    DateTime, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeDelta, TimeZone, Utc,
};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::error::TimeError;

/// The campus time zone. All user-facing dates and times are in this zone.
pub const TZ: Tz = chrono_tz::America::New_York;

/// Earliest accepted year in [`parse_date`].
pub const MIN_YEAR: i32 = 2000;
/// Latest accepted year in [`parse_date`].
pub const MAX_YEAR: i32 = 2100;

/// Parses a `YYYY-MM-DD` date strictly.
///
/// Surrounding whitespace is ignored. Month and day may be one or two digits (`2026-1-1`);
/// the year must be four digits in `2000..=2100`. Anything else, including trailing
/// characters and impossible calendar dates, is rejected.
pub fn parse_date(text: &str) -> Result<NaiveDate, TimeError> {
    let mut parts = text.trim().split('-');
    let (Some(y), Some(m), Some(d), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(TimeError::InvalidDate);
    };
    let digits = |s: &str, min: usize, max: usize| {
        (min..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
    };
    if !digits(y, 4, 4) || !digits(m, 1, 2) || !digits(d, 1, 2) {
        return Err(TimeError::InvalidDate);
    }
    let year: i32 = y.parse().map_err(|_| TimeError::InvalidDate)?;
    let month: u32 = m.parse().map_err(|_| TimeError::InvalidDate)?;
    let day: u32 = d.parse().map_err(|_| TimeError::InvalidDate)?;
    if !(MIN_YEAR..=MAX_YEAR).contains(&year) {
        return Err(TimeError::InvalidDate);
    }
    NaiveDate::from_ymd_opt(year, month, day).ok_or(TimeError::InvalidDate)
}

/// Parses a 24-hour `HH:MM` time strictly.
///
/// Surrounding whitespace is ignored. The hour may be one or two digits (`7:30`); minutes
/// must be exactly two. Hours `0..=23`, minutes `0..=59`; suffixes like `pm` are rejected.
pub fn parse_time(text: &str) -> Result<NaiveTime, TimeError> {
    let mut parts = text.trim().split(':');
    let (Some(h), Some(m), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(TimeError::InvalidTime);
    };
    let ok =
        |s: &str, min: usize| (min..=2).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    if !ok(h, 1) || !ok(m, 2) {
        return Err(TimeError::InvalidTime);
    }
    let hour: u32 = h.parse().map_err(|_| TimeError::InvalidTime)?;
    let minute: u32 = m.parse().map_err(|_| TimeError::InvalidTime)?;
    NaiveTime::from_hms_opt(hour, minute, 0).ok_or(TimeError::InvalidTime)
}

/// Formats a date as zero-padded `YYYY-MM-DD`.
pub fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Formats a time as zero-padded 24-hour `HH:MM`.
pub fn format_time(time: NaiveTime) -> String {
    time.format("%H:%M").to_string()
}

/// Converts an instant to campus-local time.
pub fn to_local(instant: DateTime<Utc>) -> DateTime<Tz> {
    instant.with_timezone(&TZ)
}

/// The campus-local calendar date of an instant.
pub fn local_date(instant: DateTime<Utc>) -> NaiveDate {
    to_local(instant).date_naive()
}

/// Resolves a campus-local wall-clock time to an instant.
///
/// - Ambiguous times (the repeated hour when DST ends) take the **earlier** instant.
/// - Nonexistent times (the skipped hour when DST starts) are shifted forward by the gap,
///   like C `mktime`: `02:30` on the spring-forward date becomes `03:30` EDT.
pub fn resolve_local(local: NaiveDateTime) -> DateTime<Utc> {
    match TZ.from_local_datetime(&local) {
        LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => dt.with_timezone(&Utc),
        LocalResult::None => {
            // Use the offset in effect just before the gap (US gaps are 1 h, months apart).
            let before = local - TimeDelta::hours(3);
            let offset_secs = TZ
                .offset_from_local_datetime(&before)
                .earliest()
                .map_or(-5 * 3600, |o| o.fix().local_minus_utc());
            (local - TimeDelta::seconds(i64::from(offset_secs))).and_utc()
        }
    }
}

/// The instant at which a campus-local calendar day starts (DST-aware).
pub fn local_midnight(date: NaiveDate) -> DateTime<Utc> {
    resolve_local(date.and_time(NaiveTime::MIN))
}

/// The whole campus-local day `[midnight, next midnight)` as a window.
///
/// Falls back to a 24 h window for the final representable date.
pub fn local_day(date: NaiveDate) -> Window {
    let start = local_midnight(date);
    let end = date
        .succ_opt()
        .map_or(start + TimeDelta::days(1), local_midnight);
    Window { start, end }
}

/// Builds a window on a campus-local date (the prototype's `/availability` rule).
///
/// `start` defaults to 00:00 and `end` to 23:59. If the end is not after the start, the end
/// moves to the next calendar day (overnight window, e.g. 22:00–02:00). `end == start` gives
/// a 24-hour-ish window ending at the same wall-clock time tomorrow.
pub fn local_window(
    date: NaiveDate,
    start: Option<NaiveTime>,
    end: Option<NaiveTime>,
) -> Result<Window, TimeError> {
    let start_time = start.unwrap_or(NaiveTime::MIN);
    let end_time = end.unwrap_or_else(default_end_time);
    let start_at = resolve_local(date.and_time(start_time));
    let mut end_at = resolve_local(date.and_time(end_time));
    if end_at <= start_at {
        // Bump the calendar day and resolve again; never add a fixed 86 400 s.
        let next = date.succ_opt().ok_or(TimeError::InvalidDate)?;
        end_at = resolve_local(next.and_time(end_time));
    }
    Window::new(start_at, end_at)
}

/// Parses user text into a window: `date` is required, `start`/`end` optional.
///
/// Errors say which input was bad ([`TimeError::InvalidDate`], [`TimeError::InvalidStart`],
/// [`TimeError::InvalidEnd`]) so callers can reply precisely instead of silently using the
/// defaults (another prototype fix).
pub fn parse_local_window(
    date: &str,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<Window, TimeError> {
    let date = parse_date(date)?;
    let start = start
        .map(|s| parse_time(s).map_err(|_| TimeError::InvalidStart))
        .transpose()?;
    let end = end
        .map(|s| parse_time(s).map_err(|_| TimeError::InvalidEnd))
        .transpose()?;
    local_window(date, start, end)
}

fn default_end_time() -> NaiveTime {
    NaiveTime::from_hms_opt(23, 59, 0).unwrap_or(NaiveTime::MIN)
}

/// A non-empty half-open time range `[start, end)` in UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "RawWindow", into = "RawWindow")]
pub struct Window {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
struct RawWindow {
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
}

impl TryFrom<RawWindow> for Window {
    type Error = TimeError;

    fn try_from(raw: RawWindow) -> Result<Self, Self::Error> {
        Window::new(raw.starts_at, raw.ends_at)
    }
}

impl From<Window> for RawWindow {
    fn from(w: Window) -> Self {
        RawWindow {
            starts_at: w.start,
            ends_at: w.end,
        }
    }
}

impl Window {
    /// A window from `start` to `end`; fails unless `end > start`.
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Self, TimeError> {
        if end > start {
            Ok(Self { start, end })
        } else {
            Err(TimeError::EmptyWindow)
        }
    }

    /// Inclusive start.
    pub const fn start(&self) -> DateTime<Utc> {
        self.start
    }

    /// Exclusive end.
    pub const fn end(&self) -> DateTime<Utc> {
        self.end
    }

    /// Length of the window.
    pub fn duration(&self) -> TimeDelta {
        self.end - self.start
    }

    /// Whether `instant` falls inside the window.
    pub fn contains(&self, instant: DateTime<Utc>) -> bool {
        self.start <= instant && instant < self.end
    }

    /// Whether this window fully covers `other`.
    pub fn covers(&self, other: &Window) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    /// Whether the two windows share any time.
    pub fn overlaps(&self, other: &Window) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// The shared part of two windows, if any.
    pub fn intersect(&self, other: &Window) -> Option<Window> {
        Window::new(self.start.max(other.start), self.end.min(other.end)).ok()
    }

    /// Whether the window overlaps the given campus-local calendar day.
    pub fn overlaps_local_date(&self, date: NaiveDate) -> bool {
        self.overlaps(&local_day(date))
    }

    /// Whether start and end fall on different campus-local dates (shown as "overnight").
    pub fn crosses_midnight(&self) -> bool {
        local_date(self.start) != local_date(self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        parse_date(s).unwrap()
    }
    fn t(s: &str) -> NaiveTime {
        parse_time(s).unwrap()
    }
    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    // --- Fix a8443b3: strict date parsing -------------------------------------------------

    #[test]
    fn date_accepts_valid_and_leap_day() {
        assert_eq!(
            d("2026-10-31"),
            NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()
        );
        assert_eq!(
            d("2024-02-29"),
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );
        assert_eq!(
            d("  2026-11-20 "),
            NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()
        );
    }

    #[test]
    fn date_rejects_trailing_junk_and_bad_shapes() {
        for bad in [
            "2026-10-31x",
            "2026-10-31 junk",
            "2026/01/01",
            "26-01-01",
            "2026-001-01",
            "2026-01",
            "2026-01-01-01",
            "+2026-01-01",
            "2026-1a-01",
            "",
        ] {
            assert_eq!(parse_date(bad), Err(TimeError::InvalidDate), "{bad:?}");
        }
    }

    #[test]
    fn date_rejects_impossible_calendar_dates() {
        for bad in [
            "2026-02-30",
            "2025-02-29",
            "2100-02-29",
            "2026-04-31",
            "2026-13-01",
            "2026-00-10",
            "2026-01-00",
        ] {
            assert_eq!(parse_date(bad), Err(TimeError::InvalidDate), "{bad:?}");
        }
    }

    #[test]
    fn date_rejects_out_of_range_years() {
        assert_eq!(parse_date("1999-12-31"), Err(TimeError::InvalidDate));
        assert_eq!(parse_date("2101-01-01"), Err(TimeError::InvalidDate));
        assert!(parse_date("2000-01-01").is_ok());
        assert!(parse_date("2100-12-31").is_ok());
    }

    // --- Fix a8443b3: strict time parsing -------------------------------------------------

    #[test]
    fn time_accepts_valid() {
        assert_eq!(t("00:00"), NaiveTime::MIN);
        assert_eq!(t("23:59"), NaiveTime::from_hms_opt(23, 59, 0).unwrap());
        assert_eq!(t("7:30"), NaiveTime::from_hms_opt(7, 30, 0).unwrap());
        assert_eq!(t("07:30"), NaiveTime::from_hms_opt(7, 30, 0).unwrap());
    }

    #[test]
    fn time_rejects_out_of_range_and_junk() {
        for bad in [
            "25:99", "23:60", "24:00", "12:30pm", "12:30x", "12", "12:3", "123:00", ":30",
            "12:30:00", "-1:00", "",
        ] {
            assert_eq!(parse_time(bad), Err(TimeError::InvalidTime), "{bad:?}");
        }
    }

    #[test]
    fn window_errors_name_the_bad_input() {
        assert_eq!(
            parse_local_window("2026-02-30", None, None),
            Err(TimeError::InvalidDate)
        );
        assert_eq!(
            parse_local_window("2026-11-20", Some("25:00"), None),
            Err(TimeError::InvalidStart)
        );
        assert_eq!(
            parse_local_window("2026-11-20", Some("19:00"), Some("7pm")),
            Err(TimeError::InvalidEnd)
        );
    }

    // --- Fix ad75048: zero-padded dates ---------------------------------------------------

    #[test]
    fn one_digit_dates_normalize_to_zero_padded() {
        assert_eq!(format_date(d("2026-1-1")), "2026-01-01");
        assert_eq!(format_date(d("2026-3-08")), "2026-03-08");
        assert_eq!(format_time(t("7:05")), "07:05");
    }

    // --- Fix 1a1fda4: DST-safe overnight rollover -----------------------------------------

    #[test]
    fn overnight_across_spring_forward_is_three_hours() {
        // DST starts 2026-03-08 02:00 local. 23:00 EST -> 03:00 EDT is 3 real hours.
        let w = local_window(d("2026-03-07"), Some(t("23:00")), Some(t("03:00"))).unwrap();
        assert_eq!(w.start(), utc("2026-03-08T04:00:00Z"));
        assert_eq!(w.end(), utc("2026-03-08T07:00:00Z"));
        assert_eq!(w.duration(), TimeDelta::hours(3));
        assert!(w.crosses_midnight());
    }

    #[test]
    fn overnight_across_fall_back_is_five_hours() {
        // DST ends 2026-11-01 02:00 local. 23:00 EDT -> 03:00 EST is 5 real hours.
        let w = local_window(d("2026-10-31"), Some(t("23:00")), Some(t("03:00"))).unwrap();
        assert_eq!(w.start(), utc("2026-11-01T03:00:00Z"));
        assert_eq!(w.end(), utc("2026-11-01T08:00:00Z"));
        assert_eq!(w.duration(), TimeDelta::hours(5));
    }

    #[test]
    fn whole_day_on_transition_dates_uses_local_midnight() {
        // The prototype's "midnight bug": midnight must resolve with the right offset.
        let spring = local_window(d("2026-03-08"), None, None).unwrap();
        assert_eq!(spring.start(), utc("2026-03-08T05:00:00Z"));
        assert_eq!(spring.end(), utc("2026-03-09T03:59:00Z"));
        assert_eq!(
            spring.duration(),
            TimeDelta::hours(22) + TimeDelta::minutes(59)
        );

        let fall = local_window(d("2026-11-01"), None, None).unwrap();
        assert_eq!(fall.start(), utc("2026-11-01T04:00:00Z"));
        assert_eq!(fall.end(), utc("2026-11-02T04:59:00Z"));
        assert_eq!(
            fall.duration(),
            TimeDelta::hours(24) + TimeDelta::minutes(59)
        );
    }

    #[test]
    fn rollover_carries_month_and_year() {
        let nye = local_window(d("2026-12-31"), Some(t("22:00")), Some(t("01:00"))).unwrap();
        assert_eq!(local_date(nye.end()), d("2027-01-01"));
        assert_eq!(nye.duration(), TimeDelta::hours(3));

        let feb = local_window(d("2026-02-28"), Some(t("21:00")), Some(t("00:30"))).unwrap();
        assert_eq!(local_date(feb.end()), d("2026-03-01"));
    }

    #[test]
    fn equal_start_and_end_rolls_to_next_day() {
        let w = local_window(d("2026-11-20"), Some(t("18:00")), Some(t("18:00"))).unwrap();
        assert_eq!(w.duration(), TimeDelta::hours(24));
    }

    #[test]
    fn same_day_window_does_not_roll() {
        let w = parse_local_window("2026-11-20", Some("19:00"), Some("22:00")).unwrap();
        assert_eq!(w.start(), utc("2026-11-21T00:00:00Z"));
        assert_eq!(w.duration(), TimeDelta::hours(3));
        assert!(!w.crosses_midnight());
    }

    #[test]
    fn nonexistent_local_time_shifts_forward_like_mktime() {
        let gap = d("2026-03-08").and_time(t("02:30"));
        assert_eq!(resolve_local(gap), utc("2026-03-08T07:30:00Z")); // 03:30 EDT
    }

    #[test]
    fn ambiguous_local_time_takes_earlier_instant() {
        let repeated = d("2026-11-01").and_time(t("01:30"));
        assert_eq!(resolve_local(repeated), utc("2026-11-01T05:30:00Z")); // 01:30 EDT
    }

    // --- Window helpers -------------------------------------------------------------------

    #[test]
    fn window_rejects_empty_and_round_trips_json() {
        let a = utc("2026-11-20T10:00:00Z");
        assert_eq!(Window::new(a, a), Err(TimeError::EmptyWindow));
        let w = Window::new(a, a + TimeDelta::hours(1)).unwrap();
        let json = serde_json::to_string(&w).unwrap();
        assert_eq!(
            json,
            r#"{"starts_at":"2026-11-20T10:00:00Z","ends_at":"2026-11-20T11:00:00Z"}"#
        );
        assert_eq!(serde_json::from_str::<Window>(&json).unwrap(), w);
        let backwards = r#"{"starts_at":"2026-11-20T11:00:00Z","ends_at":"2026-11-20T10:00:00Z"}"#;
        assert!(serde_json::from_str::<Window>(backwards).is_err());
    }

    #[test]
    fn overlap_intersect_and_local_date() {
        let evening = parse_local_window("2026-11-20", Some("18:00"), Some("22:00")).unwrap();
        let late = parse_local_window("2026-11-20", Some("21:00"), Some("01:00")).unwrap();
        assert!(evening.overlaps(&late));
        assert_eq!(
            evening.intersect(&late).unwrap().duration(),
            TimeDelta::hours(1)
        );
        assert!(late.overlaps_local_date(d("2026-11-21")));
        assert!(!evening.overlaps_local_date(d("2026-11-21")));
        assert!(
            evening
                .covers(&parse_local_window("2026-11-20", Some("19:00"), Some("20:00")).unwrap())
        );
    }
}
