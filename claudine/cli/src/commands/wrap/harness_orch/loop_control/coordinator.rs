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
    LifecycleRunGuard, ProxyCommitError, ProxyHandoff, ProxyProvenance, RunLedger,
    SharedApprovalCache, commit_proxy, commit_proxy_in_context,
};

use super::super::HarnessPromptState;

/// Owns active-document identity for one `run_harness_loop` call.
///
/// The invariant this type exists to hold: **nothing else assigns
/// [`HarnessPromptState::source_path`]**. Before the coordinator, the
/// provider-attempt harness swapped its own source in place from inside a
/// terminal-control dispatch, which is why the loop route and the single
/// route could disagree about what the active document was.
/// How much of the staged canonical boot a document still owes.
///
/// A freshly adopted proxy target owes all of it. A directly-invoked document
/// whose `initialize` was already routed by the setup pipeline owes only the
/// tail: the stabilized reread that reaches its schema verdict, and the audit
/// over the document `initialize` may have just rewritten. Both tails are the
/// same code — which is the point, since the two routes must render the same
/// diagnostic for the same failure (AC28).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BootstrapStage {
    /// Narrow initialize-shell gate → `initialize` → stabilized reread → audit.
    Full,
    /// Stabilized reread → audit, for a document whose `initialize` already ran.
    StabilizeOnly,
}

pub(super) struct ActiveDocumentCoordinator {
    ledger: RunLedger,
    bootstrap_pending: Option<BootstrapStage>,
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
            bootstrap_pending: None,
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
        self.bootstrap_pending.is_some()
    }

    /// Consume the pending stage, returning which one was owed.
    pub(super) fn take_bootstrap_pending(&mut self) -> Option<BootstrapStage> {
        self.bootstrap_pending.take()
    }

    /// Arm the tail of the staged boot for a directly-invoked document.
    ///
    /// The setup pipeline routed this document's `initialize` already, so only
    /// the stabilized reread and the full audit are still owed. Arming it is
    /// what lets a direct document's `initialize` add or repair a schema
    /// property before the verdict is reached — the same order a proxy target
    /// gets from [`adopt_committed`](Self::adopt_committed).
    pub(super) fn arm_stabilization(&mut self) {
        if self.bootstrap_pending.is_none() {
            self.bootstrap_pending = Some(BootstrapStage::StabilizeOnly);
        }
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
        let handoff = match prompt_state.file_resolution_context() {
            Some(context) => commit_proxy_in_context(&mut self.ledger, request, context)?,
            None => {
                if let Some(invocation) = prompt_state.invocation_context.as_ref() {
                    invocation.record_ambient_fallback();
                }
                commit_proxy(&mut self.ledger, request, repo_root)?
            }
        };

        // Assignment, not merge: the overlay is document-scoped, so the source's
        // is discarded with the rest of its state and the target gets only what
        // its own `with:` evaluated to. Forwarding is explicit — an omitted
        // `with:` commits an empty map and installs it, which is what makes a
        // three-hop chain's middle document unable to leak into the third.
        self.active_provenance = Some(handoff.provenance().clone());
        Self::repoint_prompt_state(prompt_state, &handoff);
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
        self.bootstrap_pending = Some(BootstrapStage::Full);
        Ok(())
    }

    /// Adopt an **already-committed** handoff for its staged bootstrap.
    ///
    /// The command-owned coordinator has already resolved the target and
    /// approved the hop against the invocation-wide shared ledger (an
    /// `initialize`-route commit while the source's stacks were live, or a
    /// terminal-route commit the harness itself made). This performs the same
    /// CLI-side repointing as [`adopt`](Self::adopt) — discarding the source's
    /// execution state, resetting the guard, and arming the bootstrap so the
    /// staged boot runs the target's own narrow gate, `initialize`, stabilized
    /// reread, and full audit — but it does **not** re-resolve or re-approve the
    /// hop: doing so would re-count it and, because the target is already in the
    /// chain, reject it as a cycle. Hop/cycle accounting stays where it was made,
    /// on the shared ledger.
    pub(super) fn adopt_committed(
        &mut self,
        handoff: ProxyHandoff,
        prompt_state: &mut HarnessPromptState,
        lifecycle_guard: &mut LifecycleRunGuard<'_>,
        active: &mut ActiveDocumentState,
    ) {
        self.active_provenance = Some(handoff.provenance().clone());
        Self::repoint_prompt_state(prompt_state, &handoff);
        prompt_state.prompt_tail.clear();
        *active = ActiveDocumentState::initial();
        lifecycle_guard.reset_for_proxy();
        lifecycle_guard.set_config(LifecycleConfig::default());
        self.bootstrap_pending = Some(BootstrapStage::Full);
    }

    fn repoint_prompt_state(prompt_state: &mut HarnessPromptState, handoff: &ProxyHandoff) {
        let source_context = prompt_state.invocation_context.as_ref().map(|invocation| {
            invocation
                .derive_source(handoff.resolved_target())
                .expect("resolved proxy target always has an authoring directory")
        });

        prompt_state.overlay = handoff.overlay().clone();
        prompt_state.source_path = handoff.resolved_target().to_path_buf();
        prompt_state.original_ref = handoff.authored_target().to_string();
        prompt_state.entry = DocumentEntryReason::ProxyTarget;
        // A new document starts a new preparation epoch: the source's launch
        // snapshot must not leak into the target's reads. The target's first
        // canonical read constructs its own.
        prompt_state.epoch_context = None;
        if let Some(source_context) = source_context.as_ref() {
            prompt_state.input_layers.file_resolution_context =
                Some(source_context.file_resolution_context().clone());
        }
        prompt_state.source_context = source_context;
    }
}
