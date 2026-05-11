//! Deprecated re-exports from the pre-migration provider module.
//!
//! The canonical home for these items is [`crate::provider`]. This module
//! survives one release cycle so external consumers can update without
//! breakage. See `claudine/features/2026-04-26-centralized-providers/design.md`
//! §6.1 ("Deprecation Mechanics"). Removal is scheduled for the post-Phase-8
//! cleanup release.

#[deprecated(
    since = "0.9.0",
    note = "moved to `claudine::provider::Provider`; this re-export will be removed in the post-Phase-8 cleanup release"
)]
pub use crate::provider::Provider;

#[deprecated(
    since = "0.9.0",
    note = "moved to `claudine::provider::PROVIDERS_DISPLAY_ORDER`; this re-export will be removed in the post-Phase-8 cleanup release"
)]
pub use crate::provider::PROVIDERS_DISPLAY_ORDER;

#[deprecated(
    since = "0.9.0",
    note = "moved to `claudine::provider::EventSupportLevel`; this re-export will be removed in the post-Phase-8 cleanup release"
)]
pub use crate::provider::EventSupportLevel;
