use strum::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Media-button icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Button {
    Play,
    Pause,
    FastForward,
    Rewind,
    Stop,
    Mute,
    Power,
}

impl DomainIcon for Button {
    fn iconify_id(self) -> &'static str {
        match self {
            Button::Play => "mdi:play",
            Button::Pause => "mdi:pause",
            Button::FastForward => "mdi:fast-forward",
            Button::Rewind => "mdi:rewind",
            Button::Stop => "mdi:stop",
            Button::Mute => "mdi:volume-mute",
            Button::Power => "mdi:power",
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
        for variant in Button::iter() {
            let s = variant.to_string();
            assert_eq!(Button::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Button::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
