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

use crate::body::IconBody;
use crate::glyph::Glyph;
use crate::icon::Icon;

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
pub fn all_iconify_ids() -> std::collections::BTreeSet<&'static str> {
    use strum::IntoEnumIterator;

    let mut ids = std::collections::BTreeSet::new();
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
