//! Gear & games declarations (ported from the prototype's `/gear`).

use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::UserId;

/// What kind of setup someone can bring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Setup {
    /// A console (Switch, PlayStation, ...).
    Console,
    /// A laptop.
    Laptop,
    /// Anything else (board games, controllers, ...).
    Other,
}

impl Setup {
    /// Lowercase name used in storage and replies.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Console => "console",
            Self::Laptop => "laptop",
            Self::Other => "other",
        }
    }

    /// Parses user input, trimming and ignoring case (`" Console "` → `Console`).
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        match text.trim().to_ascii_lowercase().as_str() {
            "console" => Ok(Self::Console),
            "laptop" => Ok(Self::Laptop),
            "other" => Ok(Self::Other),
            _ => Err(DomainError::invalid(
                "setup",
                "expected console, laptop, or other",
            )),
        }
    }
}

/// One declaration: "I can bring this setup, and I own this game" (idempotent per triple).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Gear {
    /// Who.
    pub user_id: UserId,
    /// Setup they bring.
    pub setup: Setup,
    /// Game title, trimmed, non-empty.
    pub game: String,
}

impl Gear {
    /// Validates a declaration; a blank game is rejected (prototype fix `a8443b3`).
    pub fn new(user_id: UserId, setup: Setup, game: &str) -> Result<Self, DomainError> {
        let game = game.trim();
        if game.is_empty() {
            return Err(DomainError::invalid("game", "expected a non-empty title"));
        }
        Ok(Self {
            user_id,
            setup,
            game: game.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_parsing_normalizes() {
        assert_eq!(Setup::parse(" Console "), Ok(Setup::Console));
        assert_eq!(Setup::parse("LAPTOP"), Ok(Setup::Laptop));
        assert!(Setup::parse("phone").is_err());
    }

    #[test]
    fn blank_game_rejected() {
        let u = UserId::from_u128(1);
        assert!(Gear::new(u, Setup::Console, "   ").is_err());
        assert_eq!(
            Gear::new(u, Setup::Console, " Smash ").unwrap().game,
            "Smash"
        );
    }
}
