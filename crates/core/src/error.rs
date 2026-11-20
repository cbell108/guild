//! Error types for domain validation.

use thiserror::Error;

/// A date, time, or window that failed the strict parsing rules in [`crate::time`].
///
/// Display strings are user-facing (the bot and web show them verbatim).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TimeError {
    /// Not a valid `YYYY-MM-DD` calendar date between 2000 and 2100.
    #[error("Invalid date, expected YYYY-MM-DD.")]
    InvalidDate,
    /// Not a valid `HH:MM` 24-hour time.
    #[error("Invalid time, expected HH:MM.")]
    InvalidTime,
    /// The start time of a window was not a valid `HH:MM` time.
    #[error("Invalid start time, expected HH:MM.")]
    InvalidStart,
    /// The end time of a window was not a valid `HH:MM` time.
    #[error("Invalid end time, expected HH:MM.")]
    InvalidEnd,
    /// A window whose end is not after its start.
    #[error("The end must be after the start.")]
    EmptyWindow,
}

/// A domain value that failed validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    /// A date/time problem.
    #[error(transparent)]
    Time(#[from] TimeError),
    /// A field failed validation; `reason` is user-facing.
    #[error("Invalid {field}: {reason}")]
    Invalid {
        /// The offending field, matching the API request field name.
        field: &'static str,
        /// Why it was rejected.
        reason: String,
    },
}

impl DomainError {
    /// Shorthand for [`DomainError::Invalid`].
    pub fn invalid(field: &'static str, reason: impl Into<String>) -> Self {
        Self::Invalid {
            field,
            reason: reason.into(),
        }
    }
}
