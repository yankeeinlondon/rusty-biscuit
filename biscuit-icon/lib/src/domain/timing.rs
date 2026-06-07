use strum_macros::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Timing and flow-control icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Timing {
    StartFlag,
    StopSign,
    StopSquare,
    Timer,
}

impl DomainIcon for Timing {
    fn iconify_id(self) -> &'static str {
        match self {
            Timing::StartFlag => "mdi:flag-checkered",
            Timing::StopSign => "mdi:stop-circle",
            Timing::StopSquare => "mdi:stop",
            Timing::Timer => "mdi:timer",
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
        for variant in Timing::iter() {
            let s = variant.to_string();
            assert_eq!(Timing::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Timing::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
