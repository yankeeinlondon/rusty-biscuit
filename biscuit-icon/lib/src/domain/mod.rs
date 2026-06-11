//! Curated domain icon sets, accessed enum-first.

mod actors;
mod arrow;
mod brand;
mod button;
mod control;
mod data;
mod devops;
mod emoji;
mod file;
mod hardware;
mod nav;
mod network;
mod os;
mod social;
mod sport;
mod timing;

pub use actors::Actors;
pub use arrow::Arrow;
pub use brand::Brand;
pub use button::Button;
pub use control::Control;
pub use data::Data;
pub use devops::DevOps;
pub use emoji::Emoji;
pub use file::File;
pub use hardware::Hardware;
pub use nav::Nav;
pub use network::Network;
pub use os::Os;
pub use social::Social;
pub use sport::Sport;
pub use timing::Timing;

pub(crate) mod generated;

use std::collections::BTreeSet;

use crate::body::IconBody;
use crate::glyph::Glyph;
use crate::icon::Icon;

/// A curated domain-set variant exposed by the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainVariant {
    /// The snake_case variant name (e.g. `"happy"`).
    pub name: String,
    /// The optional glyph mapping for this variant.
    pub glyph: Option<Glyph>,
    /// The upstream Iconify identifier (e.g. `"fluent-emoji-flat:grinning-face"`).
    pub iconify_id: &'static str,
}

/// Registry entry that pairs a domain-set enum with its canonical set name.
struct DomainSet {
    name: &'static str,
    count: fn() -> usize,
    variants: fn() -> Vec<DomainVariant>,
    prefixes: fn() -> Vec<&'static str>,
    icon_for_name: fn(&str) -> Option<Icon>,
}

macro_rules! domain_set {
    ($enum:ty, $name:literal) => {
        DomainSet {
            name: $name,
            count: || {
                use strum::IntoEnumIterator;
                <$enum>::iter().count()
            },
            variants: || {
                use strum::IntoEnumIterator;
                <$enum>::iter()
                    .map(|v| DomainVariant {
                        name: format!("{v}"),
                        glyph: crate::domain::DomainIcon::glyph(v),
                        iconify_id: crate::domain::DomainIcon::iconify_id(v),
                    })
                    .collect()
            },
            prefixes: || {
                use strum::IntoEnumIterator;
                let mut prefixes = std::collections::HashSet::new();
                for v in <$enum>::iter() {
                    if let Some((p, _)) = crate::domain::DomainIcon::iconify_id(v).split_once(':') {
                        prefixes.insert(p);
                    }
                }
                prefixes.into_iter().collect()
            },
            icon_for_name: |name| {
                use std::str::FromStr;
                <$enum>::from_str(name)
                    .ok()
                    .map(|v: $enum| crate::domain::DomainIcon::icon(v))
            },
        }
    };
}

fn all_domain_sets() -> &'static [DomainSet] {
    static SETS: std::sync::OnceLock<Vec<DomainSet>> = std::sync::OnceLock::new();
    SETS.get_or_init(|| {
        vec![
            domain_set!(Os, "os"),
            domain_set!(Emoji, "emoji"),
            domain_set!(Arrow, "arrow"),
            domain_set!(Data, "data"),
            domain_set!(File, "file"),
            domain_set!(Hardware, "hardware"),
            domain_set!(Timing, "timing"),
            domain_set!(Button, "button"),
            domain_set!(Control, "control"),
            domain_set!(Network, "network"),
            domain_set!(DevOps, "dev_ops"),
            domain_set!(Actors, "actors"),
            domain_set!(Nav, "nav"),
            domain_set!(Sport, "sport"),
            domain_set!(Brand, "brand"),
            domain_set!(Social, "social"),
        ]
    })
}

/// Returns the 16 curated domain set names together with their variant counts.
#[must_use]
pub fn domain_sets() -> Vec<(&'static str, usize)> {
    all_domain_sets()
        .iter()
        .map(|s| (s.name, (s.count)()))
        .collect()
}

/// Returns `true` when `set` is one of the 16 curated domain set names.
#[must_use]
pub fn is_domain_set(set: &str) -> bool {
    all_domain_sets().iter().any(|s| s.name == set)
}

/// Returns every variant of the named domain set, or `None` if `set` is unknown.
#[must_use]
pub fn domain_variants(set: &str) -> Option<Vec<DomainVariant>> {
    all_domain_sets()
        .iter()
        .find(|s| s.name == set)
        .map(|s| (s.variants)())
}

/// Resolves a `set:variant` string to an [`Icon`] when both parts name a
/// curated domain entry. Returns `None` for unknown sets or variants.
#[must_use]
pub fn domain_icon(set_and_variant: &str) -> Option<Icon> {
    let (set, variant) = set_and_variant.split_once(':')?;
    all_domain_sets()
        .iter()
        .find(|s| s.name == set)
        .and_then(|s| (s.icon_for_name)(variant))
}

/// Maps an Iconify prefix to the curated domain set name that uses it,
/// if any. The first matching set wins when multiple sets share a prefix.
#[must_use]
pub fn domain_set_name_for_prefix(prefix: &str) -> Option<&'static str> {
    all_domain_sets()
        .iter()
        .find(|s| (s.prefixes)().contains(&prefix))
        .map(|s| s.name)
}

/// Builds an [`Icon`] for a curated domain id, if one exists.
#[must_use]
pub fn icon_for_id(id: &str) -> Option<Icon> {
    use strum::IntoEnumIterator;

    macro_rules! try_set {
        ($enum:ty) => {
            for variant in <$enum>::iter() {
                if variant.iconify_id() == id {
                    return Some(variant.icon());
                }
            }
        };
    }

    try_set!(Os);
    try_set!(Emoji);
    try_set!(Arrow);
    try_set!(Data);
    try_set!(File);
    try_set!(Hardware);
    try_set!(Timing);
    try_set!(Button);
    try_set!(Control);
    try_set!(Network);
    try_set!(DevOps);
    try_set!(Actors);
    try_set!(Nav);
    try_set!(Sport);
    try_set!(Brand);
    try_set!(Social);

    None
}

/// Common behavior for every curated domain-icon enum.
pub trait DomainIcon: Copy {
    /// The upstream Iconify identifier, e.g. `"hugeicons:apple-finder"`.
    fn iconify_id(self) -> &'static str;

    /// The embedded icon body for this variant.
    fn body(self) -> IconBody;

    /// The character representation, if this icon defines one.
    fn glyph(self) -> Option<Glyph> {
        None
    }

    /// Builds an [`Icon`] for this domain variant.
    fn icon(self) -> Icon {
        Icon::from_domain(self.iconify_id(), self.body(), self.glyph())
    }
}

/// Every curated Iconify identifier across all domain sets.
///
/// The dev-only asset pipeline ([`crate::iconify`]-backed `populate_assets`
/// binary) uses this as its source of truth — adding a variant to any domain
/// enum automatically enrolls it for vendoring.
#[must_use]
pub fn all_iconify_ids() -> BTreeSet<&'static str> {
    use strum::IntoEnumIterator;

    let mut ids = BTreeSet::new();
    macro_rules! collect {
        ($enum:ty) => {
            for variant in <$enum>::iter() {
                ids.insert(variant.iconify_id());
            }
        };
    }
    collect!(Os);
    collect!(Emoji);
    collect!(Arrow);
    collect!(Data);
    collect!(File);
    collect!(Hardware);
    collect!(Timing);
    collect!(Button);
    collect!(Control);
    collect!(Network);
    collect!(DevOps);
    collect!(Actors);
    collect!(Nav);
    collect!(Sport);
    collect!(Brand);
    collect!(Social);
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_sets_returns_16_entries_with_counts() {
        let sets = domain_sets();
        assert_eq!(sets.len(), 16, "expected 16 domain sets; got: {sets:?}");

        let emoji_count = sets.iter().find(|(n, _)| *n == "emoji").map(|(_, c)| *c);
        assert_eq!(emoji_count, Some(5), "emoji set should have 5 variants");

        let os_count = sets.iter().find(|(n, _)| *n == "os").map(|(_, c)| *c);
        assert_eq!(os_count, Some(6), "os set should have 6 variants");
    }

    #[test]
    fn domain_variants_returns_variants_for_known_set() {
        let variants = domain_variants("emoji").expect("emoji should be a known domain set");
        assert!(!variants.is_empty());
        assert!(variants.iter().any(|v| v.name == "happy" && v.iconify_id == "fluent-emoji-flat:grinning-face"));
    }

    #[test]
    fn domain_variants_returns_none_for_unknown_set() {
        assert!(domain_variants("nope").is_none());
    }

    #[test]
    fn is_domain_set_recognizes_curated_names() {
        assert!(is_domain_set("os"));
        assert!(is_domain_set("sport"));
        assert!(!is_domain_set("mdi"));
    }

    #[test]
    fn domain_icon_resolves_set_variant_string() {
        let icon = domain_icon("emoji:happy").expect("emoji:happy should resolve");
        assert_eq!(icon.id(), "fluent-emoji-flat:grinning-face");
    }

    #[test]
    fn domain_icon_returns_none_for_unknown_set_or_variant() {
        assert!(domain_icon("mdi:home").is_none());
        assert!(domain_icon("emoji:frobnicate").is_none());
    }

    #[test]
    fn domain_set_name_for_prefix_maps_iconify_prefix_to_enum_name() {
        assert_eq!(domain_set_name_for_prefix("fluent-emoji-flat"), Some("emoji"));
        assert_eq!(domain_set_name_for_prefix("ic"), Some("os"));
        assert_eq!(domain_set_name_for_prefix("unknown"), None);
    }
}
