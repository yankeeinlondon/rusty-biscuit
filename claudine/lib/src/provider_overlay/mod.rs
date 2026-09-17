//! Provider overlays — redirecting a provider's configuration without
//! replacing the child's home identity.
//!
//! A *provider overlay* is Claudine-controlled storage holding the config and
//! resource view for one wrapped launch. The provider is pointed at it through
//! a documented, provider-owned selector whose effect is limited to that
//! provider. Every other process the launch starts — `git`, `gpg`, `gh`, a
//! package manager — keeps the user's ordinary home environment.
//!
//! Three types carry that:
//!
//! - [`OverlayReasons`] — why an overlay is needed, as a set, because
//!   `--repo` plus `--mcp` raises several reasons for one launch.
//! - [`OverlayPlan`] — the complete decision for one launch, produced by
//!   [`OverlayPlanner`] from generated provider metadata.
//! - [`OverlaySelector`] — the provider-owned variable and the value it
//!   carries, plus the one place the selector's path shape is applied.
//!
//! Each launch owns its overlay root under `~/.claudine/overlays/<provider>/`
//! (or [`OVERLAY_DIR_ENV`]`/<provider>/`) through an [`OverlayLease`]: nothing a previous or concurrent launch placed
//! there can appear in this launch's view, and the root is removed when the
//! launch ends, after a guarded [`WriteBack`] carries changed stable provider
//! files back to the source. A root holding a change the write-back could not
//! persist is kept for recovery instead ([`OverlayRelease::Retained`]). Compatible legacy storage (`~/.claudine/<agent_offset>`) is
//! left untouched.
//!
//! When a provider has no verified mechanism for a requested reason, planning
//! returns an [`OverlayRefusal`] rather than a weaker plan; the wrapper turns
//! that into a pre-spawn [`ClaudineError::ProviderOverlayUnsupported`] instead
//! of moving `HOME`.
//!
//! The per-(provider, reason) verdicts and the evidence behind them are in
//! `fixes/2026-09-12-shadow-home/audit.md`.
//!
//! [`ClaudineError::ProviderOverlayUnsupported`]:
//!     crate::error::ClaudineError::ProviderOverlayUnsupported

mod lease;
mod plan;
mod selector;
mod write_back;

pub use plan::{
    OVERLAY_REASONS, OverlayEntryKind, OverlayMaterialization, OverlayPlan, OverlayPlanner, OverlayReasons,
    OverlayRefusal, OverlayStage, repo_isolated_resources,
};
pub use lease::{OverlayLease, OverlayRelease, sweep_abandoned_overlays};
pub use write_back::{WriteBack, WriteBackFailure, WriteBackOutcome, record_claudine_write};
pub use selector::{OVERLAY_DIR_ENV, OVERLAY_LAUNCHES_DIR, OverlaySelector, OverlayStorage, provider_visible_root};

// Re-exported so a consumer needs one import path for the whole vocabulary.
// The enums themselves live in `claudine-catalog-types` because the catalog
// generator validates the facts records against their variant names without
// linking this library.
pub use crate::provider::{
    OverlayCapability, OverlayReason, OverlayResourceClass, OverlaySelectorShape,
};

#[cfg(test)]
mod tests;
