use strum::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Data and storage icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Data {
    Cloud,
    Database,
    Floppy,
    SdCard,
    UnorderedList,
    OrderedList,
    List,
}

impl DomainIcon for Data {
    fn iconify_id(self) -> &'static str {
        match self {
            Data::Cloud => "mdi:cloud",
            Data::Database => "mdi:database",
            Data::Floppy => "mdi:floppy",
            Data::SdCard => "mdi:sd",
            Data::UnorderedList => "mdi:format-list-bulleted",
            Data::OrderedList => "mdi:format-list-numbered",
            Data::List => "mdi:view-list",
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
        for variant in Data::iter() {
            let s = variant.to_string();
            assert_eq!(Data::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Data::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
