// `CompositionError` is intentionally large (it carries frontmatter excerpts),
// and a rejected hop returns one. The whole composition/wrap execution path
// opts out of `result_large_err` the same way (see `wrap/composition/target.rs`,
// `commands/compose/`, `commands/sequence.rs`).
#![allow(clippy::result_large_err)]

//! The one place a proxy request becomes a committed handoff.
//!
//! [`commit_proxy`] is the sole caller of
//! [`resolve_proxy_target`][crate::composition::resolve_proxy_target] and the
//! sole route to a [`HopApproval`], so resolution and hop/cycle validation
//! happen exactly once per hop, in one order, for every proxy producer. A
//! downstream layer receives a [`ProxyHandoff`] whose target is already
//! resolved and already approved, and has no entry point to resolve a target
//! string itself.

use std::path::{Path, PathBuf};

use crate::composition::error::CompositionError;
use crate::composition::resolve_proxy_target;
use crate::harness::HarnessError;

use super::handoff::{EvaluatedProxyRequest, ProxyHandoff, ResolvedProxyTarget};
use super::invocation::RunLedger;

/// Why a proxy request could not become a handoff.
///
/// Both variants keep their concrete cause: a resolution failure is a typed
/// [`HarnessError`] (whose `Diagnostic` facets a `failure`/`finalize` stack
/// reads as `err.code` / `err.detail.*`), and a rejected hop is the typed
/// [`CompositionError::LifecycleProxyCycle`] the run ledger produced.
#[derive(Debug)]
pub enum ProxyCommitError {
    /// The authored target does not resolve to an existing document.
    Resolution {
        /// The reference as the source document authored it.
        target: String,
        /// The document whose lifecycle requested the hop.
        source_path: PathBuf,
        /// The resolver's typed failure.
        error: HarnessError,
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
    let resolved = resolve_proxy_target(request.target(), &source_path, repo_root).map_err(
        |error| ProxyCommitError::Resolution {
            target: request.target().to_string(),
            source_path: source_path.clone(),
            error,
        },
    )?;
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
