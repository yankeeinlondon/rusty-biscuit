use strum_macros::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Navigation icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Nav {
    Home,
    Settings,
    Profile,
    Location,
    Cart,
    Bag,
}

impl DomainIcon for Nav {
    fn iconify_id(self) -> &'static str {
        match self {
            Nav::Home => "material-symbols:home",
            Nav::Settings => "material-symbols:settings",
            Nav::Profile => "material-symbols:account-circle",
            Nav::Location => "material-symbols-light:my-location",
            Nav::Cart => "material-symbols-light:shopping-cart-outline",
            Nav::Bag => "material-symbols-light:shopping-bag-outline",
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
        for variant in Nav::iter() {
            let s = variant.to_string();
            assert_eq!(Nav::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Nav::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
