use strum_macros::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;
use crate::glyph::Glyph;

/// Emoji / facial-expression icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Emoji {
    Happy,
    Sad,
    Laughing,
    Angry,
    Surprised,
}

impl DomainIcon for Emoji {
    fn iconify_id(self) -> &'static str {
        match self {
            Emoji::Happy => "fluent-emoji-flat:grinning-face",
            Emoji::Sad => "fluent-emoji-flat:crying-face",
            Emoji::Laughing => "fluent-emoji-flat:grinning-squinting-face",
            Emoji::Angry => "fluent-emoji-flat:angry-face",
            Emoji::Surprised => "fluent-emoji-flat:astonished-face",
        }
    }

    fn body(self) -> IconBody {
        body_for(self.iconify_id())
    }

    fn glyph(self) -> Option<Glyph> {
        Some(match self {
            Emoji::Happy => Glyph::unicode('\u{1F600}'),
            Emoji::Sad => Glyph::unicode('\u{1F622}'),
            Emoji::Laughing => Glyph::unicode('\u{1F606}'),
            Emoji::Angry => Glyph::unicode('\u{1F620}'),
            Emoji::Surprised => Glyph::unicode('\u{1F632}'),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    #[test]
    fn display_string_round_trips_every_variant() {
        for variant in Emoji::iter() {
            let s = variant.to_string();
            assert_eq!(Emoji::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Emoji::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
