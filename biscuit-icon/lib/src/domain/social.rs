use strum_macros::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Social-network icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Social {
    WhatsApp,
    Twitter,
    FacebookCircular,
    FacebookSquare,
    InstagramCircular,
    Instagram,
    X,
    PinterestCircular,
    LinkedInCircular,
    BlueSky,
    YouTube,
    YouTubeAlt,
}

impl DomainIcon for Social {
    fn iconify_id(self) -> &'static str {
        match self {
            Social::WhatsApp => "tabler:brand-whatsapp-filled",
            Social::Twitter => "mdi:twitter",
            Social::FacebookCircular => "ic:baseline-facebook",
            Social::FacebookSquare => "ri:facebook-box-fill",
            Social::InstagramCircular => "typcn:social-instagram-circular",
            Social::Instagram => "typcn:social-instagram",
            Social::X => "mingcute:social-x-line",
            Social::PinterestCircular => "ion:social-pinterest-outline",
            Social::LinkedInCircular => "typcn:social-linkedin-circular",
            Social::BlueSky => "mingcute:bluesky-social-line",
            Social::YouTube => "famicons:logo-youtube",
            Social::YouTubeAlt => "zmdi:youtube",
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
        for variant in Social::iter() {
            let s = variant.to_string();
            assert_eq!(Social::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Social::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
