//! The two-stage proxy handoff: an evaluated request, then a resolved,
//! hop-approved commitment.

use std::fmt;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;

use crate::composition::lifecycle::LifecycleSignal;

/// Render an overlay as its property names against a redaction placeholder.
///
/// An overlay is evaluated from the source document's live state, so a value
/// can be a token, a key, or anything else `{{ ... }}` reached. Status may say
/// a handoff carries an overlay and tracing may record which properties it
/// names; neither may print what is in it. A derived `Debug` would print every
/// value verbatim, and `Debug` is one `tracing::debug!(?handoff)` away from
/// being live — so the overlay-carrying types implement it by hand through
/// this helper rather than leaving the leak one keystroke away.
fn redacted_overlay(overlay: &IndexMap<String, serde_json::Value>) -> impl fmt::Debug + '_ {
    struct Redacted<'a>(&'a IndexMap<String, serde_json::Value>);

    impl fmt::Debug for Redacted<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_map()
                .entries(self.0.keys().map(|key| (key, REDACTED_VALUE)))
                .finish()
        }
    }

    Redacted(overlay)
}

/// What an overlay value renders as anywhere it would otherwise be printed.
pub(crate) const REDACTED_VALUE: &str = "<redacted>";

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
#[derive(Clone, PartialEq)]
pub struct EvaluatedProxyRequest {
    target: String,
    overlay: IndexMap<String, serde_json::Value>,
    provenance: ProxyProvenance,
}

impl fmt::Debug for EvaluatedProxyRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvaluatedProxyRequest")
            .field("target", &self.target)
            .field("overlay", &redacted_overlay(&self.overlay))
            .field("provenance", &self.provenance)
            .finish()
    }
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
#[derive(Clone, PartialEq)]
pub struct ProxyHandoff {
    authored_target: String,
    resolved_target: PathBuf,
    overlay: IndexMap<String, serde_json::Value>,
    provenance: ProxyProvenance,
}

impl fmt::Debug for ProxyHandoff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyHandoff")
            .field("authored_target", &self.authored_target)
            .field("resolved_target", &self.resolved_target)
            .field("overlay", &redacted_overlay(&self.overlay))
            .field("provenance", &self.provenance)
            .finish()
    }
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

/// A proxy handoff surfaced to the command-owned active-document loop.
///
/// Every proxy producer — `initialize` stacks, per-iteration terminal stacks,
/// and the document-loop gate — returns this one shape to its caller; none of
/// them swaps its own active document. The two variants distinguish *who*
/// commits the request against the invocation-wide [`RunLedger`], because a
/// commit failure routes differently per route (spec "Atomic handoff"):
///
/// [`RunLedger`]: super::invocation::RunLedger
#[derive(Debug, Clone, PartialEq)]
pub enum SurfacedHandoff {
    /// An evaluated request awaiting commit by the command-level ledger.
    ///
    /// Produced by the `initialize` and document-loop-gate routes, whose
    /// commit failures follow the setup-error route — the same typed
    /// diagnostic a direct invocation's `initialize` failure produces, with
    /// no catch events owed yet.
    Request(EvaluatedProxyRequest),
    /// A handoff the provider harness already committed against the shared
    /// invocation ledger. Terminal routes (`success`/`failure`/`finalize`/
    /// `start` stacks) commit while the source document's stacks are still
    /// live so a refused hop routes through the source's `failure`/`finalize`
    /// with the concrete `err`; a committed handoff then ends the source
    /// cleanly. The consumer re-prepares the resolved target directly —
    /// nothing resolves the target again.
    Committed(Box<ProxyHandoff>),
}

impl SurfacedHandoff {
    /// The evaluated overlay this handoff carries, whichever variant it is.
    pub fn overlay(&self) -> &IndexMap<String, serde_json::Value> {
        match self {
            Self::Request(request) => request.overlay(),
            Self::Committed(handoff) => handoff.overlay(),
        }
    }

    /// How the document that requested this handoff reached it, whichever
    /// variant it is.
    pub fn provenance(&self) -> &ProxyProvenance {
        match self {
            Self::Request(request) => request.provenance(),
            Self::Committed(handoff) => handoff.provenance(),
        }
    }
}
