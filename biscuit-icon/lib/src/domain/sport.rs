use strum::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Sport and activity icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Sport {
    Baseball,
    Basketball,
    Football,
    Soccer,
    Tennis,
    Cricket,
    Cycling,
    Running,
    Swimming,
    Golf,
    MartialArts,
    Volleyball,
}

impl DomainIcon for Sport {
    fn iconify_id(self) -> &'static str {
        match self {
            Sport::Baseball => "material-symbols:sports-baseball",
            Sport::Basketball => "ic:sharp-sports-basketball",
            Sport::Football => "ic:round-sports-football",
            Sport::Soccer => "ic:baseline-sports-soccer",
            Sport::Tennis => "material-symbols-light:sports-tennis-rounded",
            Sport::Cricket => "mdi:cricket",
            Sport::Cycling => "solar:bicycling-outline",
            Sport::Running => "solar:running-2-bold",
            Sport::Swimming => "maki:swimming",
            Sport::Golf => "ic:baseline-sports-golf",
            Sport::MartialArts => "ic:twotone-sports-gymnastics",
            Sport::Volleyball => "material-symbols-light:sports-volleyball-outline",
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
        for variant in Sport::iter() {
            let s = variant.to_string();
            assert_eq!(Sport::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Sport::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
