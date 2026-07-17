//! The active-document coordinator: the one owner of "which document is the
//! run running".
//!
//! Sits above both the document-loop engine and the provider-attempt harness.
//! Every proxy producer — the library loop engine's `initialize` route, the
//! CLI pipeline's `initialize` route, and any terminal event's control stack —
//! returns a [`DocumentTransition::Proxy`] carrying an
//! [`EvaluatedProxyRequest`]. None of them resolves a target, decides a hop, or
//! touches [`HarnessPromptState`]; they hand the request here and this module
//! commits it.
//!
//! The pure state decisions (resolve → approve → commit) live in
//! `claudine::composition::commit_proxy`. What stays here is the CLI-side
//! effect of a committed handoff: repointing the loop's prompt state and
//! resetting the lifecycle guard.

use std::path::Path;

use claudine::composition::{
    ActiveDocumentState, DocumentEntryReason, EvaluatedProxyRequest, LifecycleConfig,
    LifecycleRunGuard, ProxyCommitError, ProxyProvenance, RunLedger, SharedApprovalCache,
    commit_proxy,
};

use super::super::HarnessPromptState;

/// Owns active-document identity for one `run_harness_loop` call.
///
/// The invariant this type exists to hold: **nothing else assigns
/// [`HarnessPromptState::source_path`]**. Before the coordinator, the
/// provider-attempt harness swapped its own source in place from inside a
/// terminal-control dispatch, which is why the loop route and the single
/// route could disagree about what the active document was.
pub(super) struct ActiveDocumentCoordinator {
    ledger: RunLedger,
    bootstrap_pending: bool,
    active_provenance: Option<ProxyProvenance>,
    /// The active target's rebuilt launch-identity env overlays
    /// (`AGENT`/`MODEL`/`YOLO`), recomputed by the R6 target launch rebuild and
    /// retained so every subsequent attempt's child environment carries the
    /// target's own configuration, not the router's. Empty until a proxy is
    /// committed and its target's launch state is rebuilt; discarded on the next
    /// hand-off (a fresh target rebuilds its own).
    target_env_overrides: Vec<(String, String)>,
}

impl ActiveDocumentCoordinator {
    /// Open a coordinator for a run originating at `origin`.
    ///
    /// `approval_cache` is the run's real exact-command approval cache, not a
    /// fresh one: it is invocation-wide state that must survive every handoff
    /// so an already-approved command is not re-prompted at the target.
    pub(super) fn new(origin: std::path::PathBuf, approval_cache: SharedApprovalCache) -> Self {
        Self {
            ledger: RunLedger::new(origin, approval_cache),
            bootstrap_pending: false,
            active_provenance: None,
            target_env_overrides: Vec::new(),
        }
    }

    /// The active target's rebuilt launch-identity env overlays, applied to every
    /// attempt's child environment. Empty for a directly-invoked document.
    pub(super) fn target_env_overrides(&self) -> &[(String, String)] {
        &self.target_env_overrides
    }

    /// Install the rebuilt target launch identity produced by the R6 rebuild.
    ///
    /// Called during the target's staged boot, once the target's provider/model
    /// have been recomputed from its own frontmatter. Replaces (never merges) any
    /// prior target's overlays: launch identity is document-scoped and a fresh
    /// hand-off owns its own.
    pub(super) fn set_target_env_overrides(&mut self, overrides: Vec<(String, String)>) {
        self.target_env_overrides = overrides;
    }

    /// How the active document was reached, or `None` when the caller named it
    /// directly.
    ///
    /// Retained across the target's staged boot so a bootstrap failure can name
    /// the `proxy` that requested the target rather than only the target that
    /// failed. The user authored the source; they may never have heard of the
    /// target.
    pub(super) fn active_provenance(&self) -> Option<&ProxyProvenance> {
        self.active_provenance.as_ref()
    }

    /// The run ledger, for the chain and hop accounting a transition decision
    /// needs to read.
    pub(super) fn ledger(&self) -> &RunLedger {
        &self.ledger
    }

    /// Whether a newly adopted document still owes its bootstrap (lifecycle
    /// re-parse plus its own `initialize`).
    ///
    /// This is not the deleted out-of-band `pending` flag: adoption is no
    /// longer optional (a `Proxy` transition must be consumed to compile), so
    /// this only records *how far along* an already-committed handoff is.
    /// Phase 7 owns the staged bootstrap this gates.
    pub(super) fn bootstrap_pending(&self) -> bool {
        self.bootstrap_pending
    }

    /// Consume the bootstrap-pending flag, returning whether it was set.
    pub(super) fn take_bootstrap_pending(&mut self) -> bool {
        std::mem::take(&mut self.bootstrap_pending)
    }

    /// Commit `request` and repoint the run at its target.
    ///
    /// The only mutation of active-document identity in the CLI. Resolution
    /// and the hop/cycle decision are delegated to the library coordinator, so
    /// a proxy from `initialize` and a proxy from terminal recovery fail
    /// identically on a missing target and on a cycle.
    ///
    /// Clean-handoff semantics: the source document's execution state is
    /// discarded, not carried. The target — not the source — owns the run's
    /// closure and output from here.
    ///
    /// `active` is taken by `&mut` rather than reset by the caller so that a
    /// hand-off cannot be committed without discarding the source's whole
    /// active-document execution state (attempt slice, retry/resume ceilings,
    /// live session, resume follow-up): the discard and the identity change are
    /// one operation.
    ///
    /// ## Errors
    ///
    /// Returns the typed commit failure when the target cannot be resolved or
    /// the ledger refuses the hop. `prompt_state`, `lifecycle_guard`, and
    /// `active` are untouched in that case: a refused hand-off never
    /// half-activates a target, and the source it is still running keeps the
    /// execution state it earned.
    pub(super) fn adopt(
        &mut self,
        request: EvaluatedProxyRequest,
        repo_root: Option<&Path>,
        prompt_state: &mut HarnessPromptState,
        lifecycle_guard: &mut LifecycleRunGuard<'_>,
        active: &mut ActiveDocumentState,
    ) -> Result<(), ProxyCommitError> {
        let handoff = commit_proxy(&mut self.ledger, request, repo_root)?;

        // Assignment, not merge: the overlay is document-scoped, so the source's
        // is discarded with the rest of its state and the target gets only what
        // its own `with:` evaluated to. Forwarding is explicit — an omitted
        // `with:` commits an empty map and installs it, which is what makes a
        // three-hop chain's middle document unable to leak into the third.
        prompt_state.overlay = handoff.overlay().clone();
        self.active_provenance = Some(handoff.provenance().clone());
        prompt_state.source_path = handoff.resolved_target().to_path_buf();
        prompt_state.original_ref = handoff.authored_target().to_string();
        prompt_state.entry = DocumentEntryReason::ProxyTarget;
        // The document's prompt tail belonged to the source and is discarded
        // with the rest of its state.
        prompt_state.prompt_tail.clear();
        // A proxy discards the whole active-document execution layer and builds
        // the target's fresh: iteration 1, attempt 1, unestablished retry/resume
        // ceilings, no live session, no resume follow-up. The retry/resume
        // ceilings, the attempt number, and the resume follow-up/session all
        // belonged to the document being replaced. Invocation-wide hop/cycle
        // accounting is *not* here — it lives in the ledger and survives.
        *active = ActiveDocumentState::initial();
        lifecycle_guard.reset_for_proxy();
        // Clean-handoff semantics, applied to the guard's *config* and not just
        // its emission ledger: the source's stacks belong to the document being
        // replaced. Leaving them installed would let a failure while booting the
        // target fire the source's `blocked`/`finalize` — a synthetic source
        // closure after the source has already ended. The target's own config is
        // installed by its staged boot, once it exists.
        lifecycle_guard.set_config(LifecycleConfig::default());
        // The replaced document's rebuilt launch identity is discarded with the
        // rest of its state; the newly adopted target rebuilds its own during its
        // staged boot.
        self.target_env_overrides.clear();
        self.bootstrap_pending = true;
        Ok(())
    }
}
