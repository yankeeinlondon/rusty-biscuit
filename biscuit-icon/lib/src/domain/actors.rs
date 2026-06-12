use strum::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// People / account icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Actors {
    ProfileCircular,
    ProfileSquare,
    ProfilePin,
    Group,
}

impl DomainIcon for Actors {
    fn iconify_id(self) -> &'static str {
        match self {
            Actors::ProfileCircular => "material-symbols:account-circle",
            Actors::ProfileSquare => "material-symbols:account-box",
            Actors::ProfilePin => "material-symbols:person-pin",
            Actors::Group => "material-symbols:group-rounded",
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
        for variant in Actors::iter() {
            let s = variant.to_string();
            assert_eq!(Actors::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Actors::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
