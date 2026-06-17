use strum::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Directional arrow icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Arrow {
    CircularLeft,
    CircularRight,
    CircularUp,
    CircularDown,
}

impl DomainIcon for Arrow {
    fn iconify_id(self) -> &'static str {
        match self {
            Arrow::CircularLeft => "mdi:arrow-left-circle",
            Arrow::CircularRight => "mdi:arrow-right-circle",
            Arrow::CircularUp => "mdi:arrow-up-circle",
            Arrow::CircularDown => "mdi:arrow-down-circle",
        }
    }

    fn body(self) -> IconBody {
        body_for(self.iconify_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    #[test]
    fn display_string_round_trips_every_variant() {
        for variant in Arrow::iter() {
            let s = variant.to_string();
            assert_eq!(Arrow::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Arrow::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
