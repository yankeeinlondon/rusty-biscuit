use strum_macros::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Brand / vendor icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Brand {
    Ubiquiti,
    UbiquitiAccessPoint,
    Anthropic,
    OpenAi,
}

impl DomainIcon for Brand {
    fn iconify_id(self) -> &'static str {
        match self {
            Brand::Ubiquiti => "cbi:ubiquiti",
            Brand::UbiquitiAccessPoint => "cbi:ubiquiti-ap",
            Brand::Anthropic => "ri:anthropic-fill",
            Brand::OpenAi => "ri:openai-fill",
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
        for variant in Brand::iter() {
            let s = variant.to_string();
            assert_eq!(Brand::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Brand::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
