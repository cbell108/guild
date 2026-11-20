//! Availability windows and the crew "best time" grid.

use chrono::TimeDelta;
use serde::{Deserialize, Serialize};

use crate::ids::UserId;
use crate::time::Window;

/// A window in which a user said they are free (one per user per campus-local date,
/// matching the prototype's `/availability`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Availability {
    /// Who.
    pub user_id: UserId,
    /// When.
    pub window: Window,
}

/// One cell of an availability grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    /// The cell's time span.
    pub window: Window,
    /// Users whose availability fully covers the cell, sorted, deduplicated.
    pub available: Vec<UserId>,
}

/// Splits `range` into cells of `step` (the last cell may be shorter) and lists who is free
/// for the whole of each cell. Returns nothing if `step` is not positive.
pub fn slots(entries: &[Availability], range: Window, step: TimeDelta) -> Vec<Slot> {
    if step <= TimeDelta::zero() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut cursor = range.start();
    while cursor < range.end() {
        let end = (cursor + step).min(range.end());
        let Ok(cell) = Window::new(cursor, end) else {
            break;
        };
        let mut available: Vec<UserId> = entries
            .iter()
            .filter(|a| a.window.covers(&cell))
            .map(|a| a.user_id)
            .collect();
        available.sort_unstable();
        available.dedup();
        out.push(Slot {
            window: cell,
            available,
        });
        cursor = end;
    }
    out
}

/// The cell with the most people free (earliest wins ties); `None` if nobody is free.
pub fn best_slot(slots: &[Slot]) -> Option<&Slot> {
    slots
        .iter()
        .filter(|s| !s.available.is_empty())
        .min_by_key(|s| (std::cmp::Reverse(s.available.len()), s.window.start()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::parse_local_window;

    fn avail(n: u128, start: &str, end: &str) -> Availability {
        Availability {
            user_id: UserId::from_u128(n),
            window: parse_local_window("2026-11-20", Some(start), Some(end)).unwrap(),
        }
    }

    #[test]
    fn grid_and_best_slot() {
        let entries = vec![
            avail(1, "18:00", "22:00"),
            avail(2, "19:00", "23:00"),
            avail(3, "20:00", "21:00"),
        ];
        let range = parse_local_window("2026-11-20", Some("18:00"), Some("23:00")).unwrap();
        let grid = slots(&entries, range, TimeDelta::hours(1));
        assert_eq!(grid.len(), 5);
        let counts: Vec<usize> = grid.iter().map(|s| s.available.len()).collect();
        assert_eq!(counts, vec![1, 2, 3, 2, 1]);
        let best = best_slot(&grid).unwrap();
        assert_eq!(best.window, grid[2].window);
    }

    #[test]
    fn partial_last_cell_and_empty_cases() {
        let range = parse_local_window("2026-11-20", Some("18:00"), Some("19:30")).unwrap();
        let grid = slots(&[], range, TimeDelta::hours(1));
        assert_eq!(grid.len(), 2);
        assert_eq!(grid[1].window.duration(), TimeDelta::minutes(30));
        assert!(best_slot(&grid).is_none());
        assert_eq!(slots(&[], range, TimeDelta::zero()), Vec::new());
    }
}
