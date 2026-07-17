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
    DocumentEntryReason, EvaluatedProxyRequest, LifecycleConfig, LifecycleRunGuard,
    ProxyCommitError, ProxyProvenance, RunLedger, SharedApprovalCache, commit_proxy,
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
        }
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
    /// `budgets` is taken by `&mut` rather than reset by the caller so that a
    /// hand-off cannot be committed without retiring the source's retry/resume
    /// ceilings: the reset and the identity change are one operation.
    ///
    /// ## Errors
    ///
    /// Returns the typed commit failure when the target cannot be resolved or
    /// the ledger refuses the hop. `prompt_state`, `lifecycle_guard`, and
    /// `budgets` are untouched in that case: a refused hand-off never
    /// half-activates a target, and the source it is still running keeps the
    /// ceilings it earned.
    pub(super) fn adopt(
        &mut self,
        request: EvaluatedProxyRequest,
        repo_root: Option<&Path>,
        prompt_state: &mut HarnessPromptState,
        lifecycle_guard: &mut LifecycleRunGuard<'_>,
        budgets: &mut super::ControlBudgets,
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
        // Active-document execution state is discarded by a proxy: the tail,
        // the pending resume follow-up, and the live session all belonged to
        // the document being replaced.
        prompt_state.prompt_tail.clear();
        prompt_state.next_prompt_override = None;
        prompt_state.next_resume_session_id = None;
        // Same category as the state above, but it lives on the loop rather
        // than on the prompt state: the retry/resume ceilings were earned by
        // the document being replaced.
        budgets.reset_for_document();
        lifecycle_guard.reset_for_proxy();
        // Clean-handoff semantics, applied to the guard's *config* and not just
        // its emission ledger: the source's stacks belong to the document being
        // replaced. Leaving them installed would let a failure while booting the
        // target fire the source's `blocked`/`finalize` — a synthetic source
        // closure after the source has already ended. The target's own config is
        // installed by its staged boot, once it exists.
        lifecycle_guard.set_config(LifecycleConfig::default());
        self.bootstrap_pending = true;
        Ok(())
    }
}
