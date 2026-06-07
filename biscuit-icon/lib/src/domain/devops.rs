use strum_macros::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// DevOps and source-control icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum DevOps {
    Git,
    GitAlt,
    Github,
    GitMerge,
    GitLab,
    Gitea,
    CiCd,
    Deployment,
    Versions,
}

impl DomainIcon for DevOps {
    fn iconify_id(self) -> &'static str {
        match self {
            DevOps::Git => "ion:git-network",
            DevOps::GitAlt => "fe:git",
            DevOps::Github => "uil:github",
            DevOps::GitMerge => "bx:git-merge",
            DevOps::GitLab => "lucide:gitlab",
            DevOps::Gitea => "pajamas:gitea",
            DevOps::CiCd => "clarity:ci-cd-line",
            DevOps::Deployment => "material-symbols-light:deployed-code-sharp",
            DevOps::Versions => "system-uicons:versions",
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
        for variant in DevOps::iter() {
            let s = variant.to_string();
            assert_eq!(DevOps::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in DevOps::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
