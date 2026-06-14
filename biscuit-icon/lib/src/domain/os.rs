use strum::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Operating-system and platform icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Os {
    Finder,
    AppStore,
    Windows,
    Linux,
    MacOs,
    Apple,
}

impl DomainIcon for Os {
    fn iconify_id(self) -> &'static str {
        match self {
            Os::Finder => "hugeicons:apple-finder",
            Os::AppStore => "ri:app-store-fill",
            Os::Windows => "whh:windowseight",
            Os::Linux => "ant-design:linux-outlined",
            Os::MacOs => "f7:logo-macos",
            Os::Apple => "ic:baseline-apple",
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
    fn string_round_trip_uses_snake_case() {
        assert_eq!(Os::from_str("app_store").unwrap(), Os::AppStore);
        assert_eq!(Os::MacOs.to_string(), "mac_os");
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Os::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
