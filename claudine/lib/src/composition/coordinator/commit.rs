// `CompositionError` is intentionally large (it carries frontmatter excerpts),
// and a rejected hop returns one. The whole composition/wrap execution path
// opts out of `result_large_err` the same way (see `wrap/composition/target.rs`,
// `commands/compose/`, `commands/sequence.rs`).
#![allow(clippy::result_large_err)]

//! The one place a proxy request becomes a committed handoff.
//!
//! [`commit_proxy`] and [`commit_proxy_in_context`] are the sole *committing*
//! callers of the proxy resolvers and the
//! sole route to a [`HopApproval`], so resolution and hop/cycle validation
//! happen exactly once per hop, in one order, for every proxy producer. A
//! downstream layer receives a [`ProxyHandoff`] whose target is already
//! resolved and already approved, and has no entry point to resolve a target
//! into a committed handoff itself. (The CLI pipeline also resolves a target
//! read-only, without approving a hop, to peek whether it declares a `loop:`
//! and decide loop ownership before committing — that pre-check produces no
//! handoff and mutates no ledger.)

use std::path::{Path, PathBuf};

use crate::composition::error::CompositionError;
use crate::composition::{resolve_proxy_target, resolve_proxy_target_in_context};
use super::handoff::{EvaluatedProxyRequest, ProxyHandoff, ResolvedProxyTarget};
use super::invocation::RunLedger;

const PROXY_TARGET_HINT: &str =
    "A `proxy` target must name an existing Markdown document: an absolute path, \
     an explicit `./` or `../` path relative to the document that declares it, a \
     bare path (tried next to the document first, then at the repository root), \
     `@`-prefixed for a magic-root search, or `~`-prefixed for a home path.";

/// Why a proxy request could not become a handoff.
///
/// Both variants keep their concrete cause: a resolution failure is a typed
/// [`CompositionError::InvalidFileReference`] with the resolver's typed cause,
/// and a rejected hop is the typed [`CompositionError::LifecycleProxyCycle`]
/// the run ledger produced.
#[derive(Debug)]
pub enum ProxyCommitError {
    /// The authored target does not resolve to an existing document.
    Resolution {
        /// The reference as the source document authored it.
        target: String,
        /// The document whose lifecycle requested the hop.
        source_path: PathBuf,
        /// The authoring-context wrapper around the resolver's typed failure.
        error: CompositionError,
    },
    /// The run ledger refused the hop as a cycle or a budget overrun.
    Rejected(CompositionError),
}

impl std::fmt::Display for ProxyCommitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolution { error, .. } => write!(f, "lifecycle proxy: {error}"),
            Self::Rejected(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ProxyCommitError {
    /// Expose the concrete cause to the renderer.
    ///
    /// Every consumer selects its styled block by walking `source()` and
    /// downcasting (`crate::output::error_walker` in `claudine-cli`), so the
    /// default `None` would render a proxy cycle — which carries a perfectly
    /// good [`CompositionError::LifecycleProxyCycle`] — as a bare `Display`
    /// string. Keeping the concrete error reachable is what makes the same
    /// failure render identically whichever route produced it.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Resolution { error, .. } => Some(error),
            Self::Rejected(error) => Some(error),
        }
    }
}

/// Resolve, approve, and commit `request` against `ledger`.
///
/// On success the ledger has recorded the hop and the transition, and the
/// returned handoff carries the resolved target the ledger approved — not
/// whatever the request's own string would resolve to on a second attempt.
///
/// ## Errors
///
/// Returns [`ProxyCommitError::Resolution`] when the authored target does not
/// name an existing document, and [`ProxyCommitError::Rejected`] when the hop
/// would revisit a document already in the chain or exceed
/// [`MAX_PROXY_HOPS`][crate::composition::MAX_PROXY_HOPS]. In both cases the
/// ledger is left unchanged: a refused hand-off never half-activates a target.
pub fn commit_proxy(
    ledger: &mut RunLedger,
    request: EvaluatedProxyRequest,
    repo_root: Option<&Path>,
) -> Result<ProxyHandoff, ProxyCommitError> {
    let source_path = request.provenance().source_path().to_path_buf();
    let event = request.provenance().signal().property_name();
    let target = request.target().to_string();
    let resolved = resolve_proxy_target(&target, &source_path, repo_root).map_err(|source| {
        proxy_resolution_error(&target, &source_path, event, source)
    })?;
    commit_resolved_proxy(ledger, request, source_path, resolved)
}

/// Resolve and commit a proxy using the invocation's immutable file context.
///
/// Canonical Claudine orchestration uses this entry point so proxy resolution
/// cannot re-read process CWD, HOME, environment, or repository topology.
/// [`commit_proxy`] remains the ambient compatibility API.
pub fn commit_proxy_in_context(
    ledger: &mut RunLedger,
    request: EvaluatedProxyRequest,
    request_context: &biscuit_file::FileResolutionContext,
) -> Result<ProxyHandoff, ProxyCommitError> {
    let source_path = request.provenance().source_path().to_path_buf();
    let event = request.provenance().signal().property_name();
    let target = request.target().to_string();
    let resolved = resolve_proxy_target_in_context(
        &target,
        &source_path,
        request_context,
    )
    .map_err(|source| proxy_resolution_error(&target, &source_path, event, source))?;
    commit_resolved_proxy(ledger, request, source_path, resolved)
}

fn proxy_resolution_error(
    target: &str,
    source_path: &Path,
    event: &str,
    source: crate::harness::HarnessError,
) -> ProxyCommitError {
    ProxyCommitError::Resolution {
        target: target.to_string(),
        source_path: source_path.to_path_buf(),
        error: CompositionError::InvalidFileReference {
            context: Box::new(crate::composition::FileReferenceContext {
                source_path: source_path.to_path_buf(),
                event: Some(event.to_string()),
                property: format!("{event}.stack[*].proxy"),
                reference: target.to_string(),
                hint: PROXY_TARGET_HINT.to_string(),
            }),
            source,
        },
    }
}

fn commit_resolved_proxy(
    ledger: &mut RunLedger,
    request: EvaluatedProxyRequest,
    source_path: PathBuf,
    resolved: PathBuf,
) -> Result<ProxyHandoff, ProxyCommitError> {
    // Bind before matching so the `LedgerMut` temporary is dropped here: the
    // rejection arm below needs to read the chain back off the ledger.
    let decision = ledger
        .access()
        .approve_hop(ResolvedProxyTarget::from_resolver(resolved));
    let approval = match decision {
        Ok(approval) => approval,
        Err(rejection) => {
            return Err(ProxyCommitError::Rejected(rejection.into_error(
                source_path,
                request.target().to_string(),
                ledger.chain(),
            )));
        }
    };
    let handoff = ProxyHandoff::commit(request, approval);
    ledger.access().record_proxy(&handoff);
    Ok(handoff)
}
