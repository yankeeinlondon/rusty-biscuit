//! Provider-neutral vocabulary for document transitions and the four state
//! ownership layers.
//!
//! Nothing here performs process, terminal, or filesystem work, and nothing
//! resolves a file reference — a resolved `PathBuf` arrives as a value from
//! the shared file-resolution authority. The CLI driver owns those effects.
//!
//! The four layers, and what each one survives:
//!
//! | Layer | Lifetime | Survives a proxy? |
//! |---|---|---|
//! | [`InvocationInputs`] | command | yes — immutable |
//! | [`RunLedger`] | command | yes — extended, never reset |
//! | [`PreparedDocument`] | one active document | no — the target is prepared afresh |
//! | [`ActiveDocumentState`] | one active document | no — discarded |
//!
//! Handoff state ([`EvaluatedProxyRequest`] → [`ProxyHandoff`]) is the seam
//! between the last two: it is the only thing that crosses from a source
//! document to its target.
//!
//! See [`claudine/features/2026-07-13-proxy-with/spec.md`](../../../../features/2026-07-13-proxy-with/spec.md)
//! → R1/R2 for the authoritative design.

mod active;
mod commit;
mod document;
mod handoff;
mod invocation;
mod transition;

#[cfg(test)]
mod tests;

pub use commit::{ProxyCommitError, commit_proxy, commit_proxy_in_context};

pub use active::{
    ActiveDocumentState, AttemptOutcome, ControlBudget, ControlBudgetKind, DocumentIteration,
    ProviderAttempt, SessionCompatibilityKey,
};
pub use document::{DocumentOverlay, PreparedDocument};
pub use handoff::{
    ActionLocation, EvaluatedProxyRequest, HopApproval, ProxyHandoff, ProxyProvenance,
    ResolvedProxyTarget, SurfacedHandoff,
};
pub use invocation::{
    CommandOutputPolicy, HopRejection, InvocationInputs, InvocationInputsDraft, LaunchDiscovery,
    LedgerMut, RunLedger, SharedRunLedger, TransitionRecord,
};
pub use transition::{DocumentTransition, TransitionAbort};
