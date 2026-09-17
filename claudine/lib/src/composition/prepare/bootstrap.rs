//! The initialize-bootstrap read of a document.
//!
//! A document's `initialize` may create the very files its body includes, so
//! the read that feeds `initialize` must not touch the body. The bootstrap read
//! composes the frontmatter surface only (Darkmatter's
//! `ComposeOptions::only_frontmatter_surface`), and its result is a separate
//! type rather than a [`PreparedComposition`] so nothing can launch a provider
//! from it: there is no prompt, no closure plan, and no schema verdict here.
//! The full preparation happens on the post-`initialize` stabilized reread.
//!
//! [`PreparedComposition`]: crate::composition::PreparedComposition

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use darkmatter::markdown::compose::ComposeContext;

use super::DocumentEntryReason;
use crate::composition::lifecycle::LifecycleConfig;
use crate::composition::types::{CallerInputLayers, CompositionMode, EffectiveSelectionHints};

/// The effective frontmatter and lifecycle surface of a document, read before
/// its `initialize` runs.
///
/// Holds nothing derived from the body. Initialization shell actions and
/// frontmatter shell expansion are forbidden. Other events' shell commands
/// remain subject to the audit after the stabilized reread.
#[derive(Debug, Clone)]
pub struct BootstrapPreparation {
    /// Which composer the stabilized reread will run.
    pub mode: CompositionMode,
    /// Why the document is being staged; only entries that emit `initialize`.
    pub entry: DocumentEntryReason,
    /// Resolved absolute path to the source file.
    pub resolved_path: PathBuf,
    /// Repository root of the source file, when known.
    pub source_repo_root: Option<PathBuf>,
    /// The caller's input layers, for the stabilized reread to reapply.
    pub input_layers: CallerInputLayers,
    /// Composed frontmatter with lifecycle subtrees still holding their `{{ }}`
    /// spans. The schema verdict was withheld.
    pub effective_frontmatter: serde_json::Value,
    /// Provider/model/interactive hints parsed from `effective_frontmatter`.
    pub selection_hints: EffectiveSelectionHints,
    /// Parsed lifecycle stacks with C3-resolved shell commands.
    pub lifecycle: LifecycleConfig,
    /// Lifecycle keys Darkmatter deferred, sorted.
    pub deferred_lifecycle_keys: Vec<String>,
    /// The early-binding snapshot this read composed against; the stabilized
    /// reread extends it rather than recapturing launch state.
    pub compose_context: ComposeContext,
    /// Document epoch the read was attributed to.
    pub document_epoch: Option<crate::invocation_context::DocumentEpoch>,
}
