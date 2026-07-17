//! The two-stage proxy handoff: an evaluated request, then a resolved,
//! hop-approved commitment.

use std::fmt;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;

use crate::composition::lifecycle::LifecycleSignal;

/// Where in a document's lifecycle frontmatter an action was authored.
///
/// Renders as the dotted `"{event}.stack[{i}].action[{j}]"` form that the
/// shell-preflight resolver already uses for its diagnostics
/// (`composition::preflight`), so a proxy diagnostic and a shell diagnostic
/// name the same property the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionLocation {
    signal: LifecycleSignal,
    stack_index: usize,
    action_index: usize,
}

impl ActionLocation {
    /// Locate the `action_index`-th action of the `stack_index`-th stack item
    /// under `signal`. Both indices are 0-based, matching the authored YAML
    /// sequence order.
    pub fn new(signal: LifecycleSignal, stack_index: usize, action_index: usize) -> Self {
        Self {
            signal,
            stack_index,
            action_index,
        }
    }

    #[allow(missing_docs)]
    pub fn signal(&self) -> LifecycleSignal {
        self.signal
    }

    #[allow(missing_docs)]
    pub fn stack_index(&self) -> usize {
        self.stack_index
    }

    #[allow(missing_docs)]
    pub fn action_index(&self) -> usize {
        self.action_index
    }
}

impl fmt::Display for ActionLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.stack[{}].action[{}]",
            self.signal.property_name(),
            self.stack_index,
            self.action_index
        )
    }
}

/// Everything a diagnostic needs to attribute a proxy back to the exact
/// property that requested it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyProvenance {
    source_path: PathBuf,
    location: ActionLocation,
    chain: Vec<PathBuf>,
}

impl ProxyProvenance {
    /// `chain` is the invocation-wide proxy chain as it stood when the action
    /// fired, ordered oldest-first and including the originating document.
    pub fn new(source_path: PathBuf, location: ActionLocation, chain: Vec<PathBuf>) -> Self {
        Self {
            source_path,
            location,
            chain,
        }
    }

    /// The document whose lifecycle authored the proxy.
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    #[allow(missing_docs)]
    pub fn location(&self) -> &ActionLocation {
        &self.location
    }

    /// The lifecycle signal that fired the proxy.
    pub fn signal(&self) -> LifecycleSignal {
        self.location.signal()
    }

    #[allow(missing_docs)]
    pub fn chain(&self) -> &[PathBuf] {
        &self.chain
    }
}

/// A proxy request whose target reference and `with:` overlay are fully
/// evaluated, but whose target has not been resolved to a file.
///
/// This is the most a lifecycle evaluation may produce: evaluation is
/// provider-neutral and does not consult the filesystem, so it cannot know
/// whether `target` names a real document. Turning one into a committable
/// [`ProxyHandoff`] requires a [`ResolvedProxyTarget`] from the shared
/// file-resolution authority plus a hop/cycle approval from the coordinator's
/// run ledger.
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedProxyRequest {
    target: String,
    overlay: IndexMap<String, serde_json::Value>,
    provenance: ProxyProvenance,
}

impl EvaluatedProxyRequest {
    /// `target` is the authored reference exactly as it was evaluated (e.g.
    /// `@prompts/foo.md`); `overlay` is the evaluated `with:` mapping, empty
    /// when `with:` was omitted.
    pub fn new(
        target: String,
        overlay: IndexMap<String, serde_json::Value>,
        provenance: ProxyProvenance,
    ) -> Self {
        Self {
            target,
            overlay,
            provenance,
        }
    }

    /// The authored, unresolved target reference.
    pub fn target(&self) -> &str {
        &self.target
    }

    #[allow(missing_docs)]
    pub fn overlay(&self) -> &IndexMap<String, serde_json::Value> {
        &self.overlay
    }

    #[allow(missing_docs)]
    pub fn provenance(&self) -> &ProxyProvenance {
        &self.provenance
    }
}

/// A proxy target that the shared file-resolution authority has resolved to an
/// existing document.
///
/// Only this crate can mint one, and it does so only from the resolver. That
/// is what makes [`ProxyHandoff`] unconstructable without a resolution step:
/// no caller outside `claudine` — including `claudine-cli`, which drives the
/// coordinator — can fabricate a resolved target from a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProxyTarget {
    path: PathBuf,
}

impl ResolvedProxyTarget {
    /// Mint a resolved target. Reserved for the shared file-resolution
    /// authority — today [`resolve_proxy_target`][crate::composition::resolve_proxy_target],
    /// which the file-resolution feature will replace in place. Its one caller
    /// is [`commit_proxy`][super::commit_proxy].
    pub(crate) fn from_resolver(path: PathBuf) -> Self {
        Self { path }
    }

    #[allow(missing_docs)]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Proof that the run ledger accepted a hop to the carried target.
///
/// Minted only by [`LedgerMut::approve_hop`][super::LedgerMut::approve_hop],
/// which owns the invocation-wide chain and is therefore the only thing that
/// can answer the cycle and hop-limit questions. It carries the approved
/// target rather than merely attesting to one, so a handoff cannot be
/// committed for a document other than the one that was approved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HopApproval {
    target: ResolvedProxyTarget,
}

impl HopApproval {
    pub(super) fn new(target: ResolvedProxyTarget) -> Self {
        Self { target }
    }

    #[allow(missing_docs)]
    pub fn target(&self) -> &ResolvedProxyTarget {
        &self.target
    }
}

/// A committable handoff: a resolved target, its overlay, and the provenance
/// that produced it.
///
/// Downstream preparation accepts this by value and has no string-target
/// resolver entry point, so a target is resolved exactly once per hop.
///
/// A handoff cannot be assembled from a bare string — [`commit`](Self::commit)
/// consumes a [`HopApproval`], which in turn consumes a
/// [`ResolvedProxyTarget`] that only the resolver can mint:
///
/// ```compile_fail
/// use claudine::composition::ProxyHandoff;
/// // No such constructor exists: a handoff needs a resolved, approved target.
/// let _ = ProxyHandoff::new("@prompts/foo.md");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ProxyHandoff {
    authored_target: String,
    resolved_target: PathBuf,
    overlay: IndexMap<String, serde_json::Value>,
    provenance: ProxyProvenance,
}

impl ProxyHandoff {
    /// Commit `request` against the target `approval` accepted.
    ///
    /// The request's own `target` is retained as `authored_target` for
    /// diagnostics; the resolved path comes from the approval, never from the
    /// request.
    pub fn commit(request: EvaluatedProxyRequest, approval: HopApproval) -> Self {
        let EvaluatedProxyRequest {
            target,
            overlay,
            provenance,
        } = request;
        Self {
            authored_target: target,
            resolved_target: approval.target.path.clone(),
            overlay,
            provenance,
        }
    }

    /// The reference as the source document authored it.
    pub fn authored_target(&self) -> &str {
        &self.authored_target
    }

    #[allow(missing_docs)]
    pub fn resolved_target(&self) -> &Path {
        &self.resolved_target
    }

    #[allow(missing_docs)]
    pub fn overlay(&self) -> &IndexMap<String, serde_json::Value> {
        &self.overlay
    }

    #[allow(missing_docs)]
    pub fn provenance(&self) -> &ProxyProvenance {
        &self.provenance
    }

    /// Take the overlay, leaving the handoff's other fields intact for
    /// diagnostics.
    pub(crate) fn into_overlay(self) -> IndexMap<String, serde_json::Value> {
        self.overlay
    }
}
