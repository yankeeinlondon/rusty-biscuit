//! Typed provider-overlay descriptors for [`ProviderInfo`].
//!
//! The vocabulary enums live in the `claudine-catalog-types` leaf crate so
//! the catalog generator can introspect them without linking this library
//! (provider-metadata design F1); the two records below are the shapes the
//! generated `data.rs` constructs.
//!
//! An overlay redirects a provider's configuration through a documented,
//! provider-owned selector instead of replacing the child's `HOME`.
//! Whether that is possible is a per-(provider, reason) verdict recorded in
//! [`OverlayCapabilities`]; how to reach the provider's root is recorded in
//! [`OverlaySelectorSpec`]. Both are ruled data — the audit at
//! `fixes/2026-09-12-shadow-home/audit.md` is the evidence behind every
//! value, and the invariants binding the two together are tested in
//! `provider::tests`.
//!
//! [`ProviderInfo`]: super::ProviderInfo

use serde::Serialize;

pub use claudine_catalog_types::{
    OverlayCapability, OverlayReason, OverlayResourceClass, OverlaySelectorShape,
};

use super::path_template::PathTemplate;

/// The provider-owned environment variable Claudine uses to point a
/// provider at an overlay, plus what that variable's value means.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct OverlaySelectorSpec {
    /// The environment variable name, e.g. `CODEX_HOME`.
    pub env_var: &'static str,

    /// What the variable's value names relative to the directory the
    /// provider reads its configuration from.
    pub shape: OverlaySelectorShape,

    /// Resource classes the selector moves with it. A class absent here
    /// stays at its pre-overlay location even when the selector is set.
    pub relocates: &'static [OverlayResourceClass],

    /// Whether the selector is *additive* — loaded in addition to the
    /// user's own discovery paths rather than replacing them. An additive
    /// selector can override settings but cannot mask user resources, so it
    /// can never satisfy [`OverlayReason::RepoResources`].
    pub additive: bool,

    /// Where the provider reads configuration from with no overlay in
    /// play — the directory an overlay is built *from*.
    ///
    /// `None` means Claudine cannot compute a single pre-overlay root for
    /// this provider: either no overlay is ever materialized for it, or its
    /// configuration is spread across several roots that vary by OS. A
    /// `None` here forbids a [`OverlayCapability::NativeRoot`] verdict for
    /// [`OverlayReason::RepoResources`].
    pub source_root: Option<PathTemplate>,
}

impl OverlaySelectorSpec {
    /// Returns the child segment the provider appends to the selector's
    /// value, or `None` when the value is the provider directory itself.
    pub fn child_segment(&self) -> Option<&'static str> {
        match self.shape {
            OverlaySelectorShape::ParentOfProviderDir { child } => Some(child),
            OverlaySelectorShape::ProviderDir | OverlaySelectorShape::Inline => None,
        }
    }

    /// Returns whether the selector relocates the given resource class.
    pub fn relocates(&self, class: OverlayResourceClass) -> bool {
        self.relocates.contains(&class)
    }
}

/// The per-reason capability verdicts for one provider.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct OverlayCapabilities {
    /// Verdict for `--repo` resource isolation.
    pub repo_resources: OverlayCapability,

    /// Verdict for repository prompt discovery.
    pub repo_prompt: OverlayCapability,

    /// Verdict for runtime MCP configuration delivery.
    pub mcp: OverlayCapability,
}

impl OverlayCapabilities {
    /// Returns the verdict for one activation reason.
    pub fn for_reason(&self, reason: OverlayReason) -> OverlayCapability {
        match reason {
            OverlayReason::RepoResources => self.repo_resources,
            OverlayReason::RepoPrompt => self.repo_prompt,
            OverlayReason::Mcp => self.mcp,
        }
    }
}
