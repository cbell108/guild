//! Strongly typed identifiers. All are UUIDs serialized as hyphenated strings.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// A new random (v4) id.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// A fixed id, for deterministic seed data and tests.
            pub const fn from_u128(value: u128) -> Self {
                Self(Uuid::from_u128(value))
            }

            /// The underlying UUID.
            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.hyphenated().fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }
    };
}

id_type!(
    /// Identifies a [`crate::User`].
    UserId
);
id_type!(
    /// Identifies an [`crate::Event`] (official or hangout).
    EventId
);
id_type!(
    /// Identifies a [`crate::Crew`].
    CrewId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_and_parse_round_trip() {
        let id = UserId::from_u128(0x101);
        let text = id.to_string();
        assert_eq!(text, "00000000-0000-0000-0000-000000000101");
        assert_eq!(text.parse::<UserId>().unwrap(), id);
    }

    #[test]
    fn serializes_as_plain_string() {
        let id = EventId::from_u128(7);
        assert_eq!(
            serde_json::to_string(&id).unwrap(),
            "\"00000000-0000-0000-0000-000000000007\""
        );
    }
}
