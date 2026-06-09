use strum_macros::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Hardware and device icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Hardware {
    ServerNode,
    FileServer,
    ServerNetwork,
    ServerTower,
    Laptop,
    Monitor,
    Chip,
    Camera,
    Microphone,
    Speaker,
    Hammer,
    Wrench,
    Printer,
}

impl DomainIcon for Hardware {
    fn iconify_id(self) -> &'static str {
        match self {
            Hardware::ServerNode => "mdi:server",
            Hardware::FileServer => "uil:file-network",
            Hardware::ServerNetwork => "mdi:server-network",
            Hardware::ServerTower => "bi:hdd-rack-fill",
            Hardware::Laptop => "mdi:laptop",
            Hardware::Monitor => "mdi:monitor",
            Hardware::Chip => "mdi:chip",
            Hardware::Camera => "mdi:camera",
            Hardware::Microphone => "mdi:microphone",
            Hardware::Speaker => "mdi:speaker",
            Hardware::Hammer => "mdi:hammer",
            Hardware::Wrench => "mdi:wrench",
            Hardware::Printer => "mdi:printer",
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
        for variant in Hardware::iter() {
            let s = variant.to_string();
            assert_eq!(Hardware::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Hardware::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
