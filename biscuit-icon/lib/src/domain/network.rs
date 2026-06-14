use strum::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Networking and connectivity icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Network {
    WifiStrong,
    WifiWeak,
    Nodes,
    NodesStructured,
    Ethernet,
    #[strum(serialize = "3g")]
    ThreeG,
    #[strum(serialize = "4g")]
    FourG,
    #[strum(serialize = "5g")]
    FiveG,
    Lte,
}

impl DomainIcon for Network {
    fn iconify_id(self) -> &'static str {
        match self {
            Network::WifiStrong => "mdi:wifi-strength-4",
            Network::WifiWeak => "mdi:wifi-strength-1",
            Network::Nodes => "carbon:network-1",
            Network::NodesStructured => "carbon:network-2",
            Network::Ethernet => "mdi:ethernet",
            Network::ThreeG => "streamline-freehand:cellular-network-wifi-3g",
            Network::FourG => "streamline-freehand:cellular-network-wifi-4g",
            Network::FiveG => "streamline-freehand:cellular-network-wifi-5g",
            Network::Lte => "streamline-freehand:cellular-network-lte",
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
        for variant in Network::iter() {
            let s = variant.to_string();
            assert_eq!(
                Network::from_str(&s).unwrap(),
                variant,
                "round trip for {s}"
            );
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Network::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
