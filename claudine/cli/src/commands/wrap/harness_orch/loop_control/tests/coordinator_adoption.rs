//! The coordinator is the only thing that may change the active document.
//!
//! Every proxy producer routes through [`ActiveDocumentCoordinator::adopt`], so
//! these assertions cover the `initialize` route, the terminal-recovery route,
//! and a chained target proxy alike — there is no per-route resolution or
//! cycle semantics left to test separately.

use super::*;

use claudine::composition::{
    ActiveDocumentState, DocumentEntryReason, EvaluatedProxyRequest, ProxyProvenance,
};

/// A request for `target`, authored by `source`'s `failure` stack, carrying
/// the chain the ledger would have stamped on it.
fn request_for(source: &Path, target: &Path, chain: Vec<PathBuf>) -> EvaluatedProxyRequest {
    EvaluatedProxyRequest::new(
        target.display().to_string(),
        indexmap::IndexMap::new(),
        ProxyProvenance::new(
            source.to_path_buf(),
            claudine::composition::ActionLocation::new(LifecycleSignal::Failure, 0, 0),
            chain,
        ),
    )
}

struct AdoptFixture {
    fx: Fixture,
    target: PathBuf,
}

fn adopt_fixture() -> AdoptFixture {
    let fx = fixture(serde_json::json!({}));
    // Both documents exist on disk: the coordinator resolves through the shared
    // resolver, which checks existence, so a cycle test needs a real cycle
    // rather than a resolution failure standing in for one.
    std::fs::write(&fx.source_path, "---\n---\nbody\n").unwrap();
    let target = fx._dir.path().join("target.md");
    std::fs::write(&target, "---\n---\nbody\n").unwrap();
    AdoptFixture { fx, target }
}

#[test]
fn adopt_commits_identity_and_discards_source_execution_state() {
    let AdoptFixture { fx, target } = adopt_fixture();
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    guard.mark_provider_launched();
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    let mut state = prompt_state(&fx.source_path);
    // Active-document execution state belonging to the document being replaced:
    // the prompt tail lives on the prompt state; the live session, the resume
    // follow-up, an advanced attempt number, and an earned retry ceiling all
    // live on the single `ActiveDocumentState` owner.
    state.prompt_tail.push("tail from the source".to_string());
    let mut active = ActiveDocumentState::initial();
    active
        .iteration_mut()
        .resume_attempt("sess-1".to_string(), Some("follow-up".to_string()));
    active.iteration_mut().retry_budget_mut().ceiling_for(1, 1);

    let mut coord = coordinator(&fx.source_path);
    coord
        .adopt(
            request_for(&fx.source_path, &target, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut active,
        )
        .expect("the hop is resolvable and uncontested");

    assert_eq!(state.source_path, target, "identity moved to the target");
    assert_eq!(state.original_ref, target.display().to_string());
    assert_eq!(
        state.entry,
        DocumentEntryReason::ProxyTarget,
        "the target is prepared as a proxy target, not as a direct document"
    );
    assert!(state.prompt_tail.is_empty(), "the source's tail is discarded");
    // The whole execution layer is rebuilt fresh for the target: iteration 1,
    // attempt 1, no live session, no resume follow-up, no earned ceilings.
    assert_eq!(active.iteration().number(), 1);
    assert_eq!(active.iteration().attempt().number(), 1);
    assert_eq!(
        active.iteration().attempt().session_id(),
        None,
        "a proxy clears the source's live session"
    );
    assert_eq!(active.iteration().attempt().resume_followup(), None);
    assert_eq!(active.iteration().retry_budget().ceiling(), None);
    assert!(
        !guard.initialize_emitted() && guard.terminal_signal().is_none(),
        "the guard is reset so the target emits its own initialize"
    );
    assert!(
        coord.bootstrap_pending(),
        "the adopted target still owes its lifecycle re-parse and initialize"
    );
    assert_eq!(
        coord.ledger().chain(),
        [fx.source_path.clone(), target.clone()],
        "the committed hop extends the invocation-wide chain"
    );
    assert_eq!(coord.ledger().hops(), 1, "the seeded origin is not a hop");
    assert_eq!(
        coord.ledger().transitions().len(),
        1,
        "the hop is recorded for diagnostics and final output"
    );
}

/// A refused hand-off must never half-activate the target. This is the
/// resolver-bypass regression, now structurally impossible: the coordinator is
/// the only caller of `resolve_proxy_target`, so `initialize` and terminal
/// recovery cannot disagree about a missing document.
#[test]
fn adopt_rejects_a_missing_target_without_activating_it() {
    let fx = fixture(serde_json::json!({}));
    let missing = fx._dir.path().join("missing.md");
    assert!(!missing.exists(), "the fixture target must not exist");
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    guard.mark_provider_launched();
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    let mut state = prompt_state(&fx.source_path);
    let mut coord = coordinator(&fx.source_path);

    let error = coord
        .adopt(
            request_for(&fx.source_path, &missing, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut ActiveDocumentState::initial(),
        )
        .expect_err("a proxy to a missing document must be refused");

    assert!(
        matches!(
            error,
            claudine::composition::ProxyCommitError::Resolution { .. }
        ),
        "the refusal keeps the resolver's typed cause, got {error:?}"
    );
    assert!(
        error.to_string().contains("proxy target does not exist"),
        "the diagnostic carries the shared resolver's existence check: {error}"
    );
    assert_eq!(
        state.source_path, fx.source_path,
        "the source document stays active when the hand-off is refused"
    );
    assert!(
        !coord.bootstrap_pending(),
        "a refused hand-off flags no pending bootstrap"
    );
    assert!(
        !coord.ledger().contains_document(&missing),
        "a missing target never enters the chain"
    );
    assert!(
        guard.initialize_emitted() || guard.terminal_signal().is_some(),
        "the guard is not reset for a hand-off that was refused"
    );
}

/// The ledger seeds its chain with the originating document, so a document
/// that proxies to itself is a cycle from the *first* hop. Before the
/// coordinator, four call sites each decided independently whether the current
/// document was in the chain yet.
#[test]
fn adopt_rejects_a_hop_back_to_a_document_already_in_the_chain() {
    let AdoptFixture { fx, target } = adopt_fixture();
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    let mut state = prompt_state(&fx.source_path);
    let mut coord = coordinator(&fx.source_path);

    // Hop A -> B.
    coord
        .adopt(
            request_for(&fx.source_path, &target, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut ActiveDocumentState::initial(),
        )
        .expect("the first hop is uncontested");
    let _ = coord.take_bootstrap_pending();

    // B -> A closes the cycle. Multi-hop history is exactly what the loop
    // engine's single-element chain slice could not see.
    let error = coord
        .adopt(
            request_for(
                &target,
                &fx.source_path,
                vec![fx.source_path.clone(), target.clone()],
            ),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut ActiveDocumentState::initial(),
        )
        .expect_err("an A->B->A cycle must be refused");

    let claudine::composition::ProxyCommitError::Rejected(composition) = error else {
        panic!("a cycle is a ledger rejection, not a resolution failure");
    };
    assert!(
        matches!(
            composition,
            claudine::composition::CompositionError::LifecycleProxyCycle { .. }
        ),
        "the refusal is the typed cycle error"
    );
    assert_eq!(
        state.source_path, target,
        "the refused hop leaves the current target active"
    );
    assert_eq!(
        coord.ledger().chain(),
        [fx.source_path.clone(), target.clone()],
        "a refused hop does not extend the chain"
    );
}

/// Clean-handoff semantics: once `proxy` is selected, the source document's
/// closure belongs to the target. No later source terminal or `finalize`
/// signal may be synthesized — including by the guard's `Drop` net, which
/// exists to emit `failure`/`blocked` for a run that started and never
/// reached a terminal.
///
/// The Drop net is safe here only because [`ActiveDocumentCoordinator::adopt`]
/// resets the guard, clearing `start_emitted`. That coupling is the reason
/// this is a test and not a comment.
#[test]
fn a_committed_handoff_synthesizes_no_source_finalize() {
    let AdoptFixture { fx, target } = adopt_fixture();
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut state = prompt_state(&fx.source_path);
    let mut coord = coordinator(&fx.source_path);
    {
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        // The source got as far as launching a provider: without a hand-off,
        // dropping the guard here would emit a synthetic `failure`.
        assert!(guard.record_event_emission(LifecycleSignal::Start));
        guard.mark_provider_launched();

        coord
            .adopt(
                request_for(&fx.source_path, &target, vec![fx.source_path.clone()]),
                Some(fx._dir.path()),
                &mut state,
                &mut guard,
                &mut ActiveDocumentState::initial(),
            )
            .expect("the hop is resolvable and uncontested");
        // Guard drops here, at the end of the scope.
    }

    let signals: Vec<LifecycleSignal> = emitter
        .events()
        .into_iter()
        .filter_map(|event| match event {
            Emitted::Stderr(signal, _) => Some(signal),
            _ => None,
        })
        .collect();
    assert!(
        !signals.contains(&LifecycleSignal::Finalize)
            && !signals.contains(&LifecycleSignal::Failure)
            && !signals.contains(&LifecycleSignal::Blocked),
        "a handed-off source closes nothing; the target owns closure. Got {signals:?}"
    );
}

/// Closure and output ownership follow the active document, so an
/// `inline-compose` run closes over the **final target** and never the router
/// it passed through.
///
/// This holds because `materialize_harness_prompt` composes from
/// `prompt_state.source_path` and derives the closure plan from that freshly
/// prepared document — and the coordinator is what repoints `source_path`
/// before the next attempt materializes. The plan's `original_document_text`
/// is the document it will write back to, so it is the honest witness for
/// which document owns closure.
#[test]
fn inline_closure_ownership_follows_the_adopted_target() {
    let fx = fixture(serde_json::json!({}));
    std::fs::write(
        &fx.source_path,
        "---\nprompt: ROUTER-PROMPT\n---\nROUTER-BODY\n",
    )
    .unwrap();
    let target = fx._dir.path().join("target.md");
    std::fs::write(&target, "---\nprompt: TARGET-PROMPT\n---\nTARGET-BODY\n").unwrap();

    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    let mut state = prompt_state(&fx.source_path);
    state.mode = HarnessPromptMode::Inline;

    // Before the hand-off, the router owns closure.
    let before = super::super::super::materialize_harness_prompt(
        &state,
        Some(fx._dir.path()),
        fx._dir.path(),
        None,
        claudine::composition::SchemaStage::Validate,
    )
    .expect("the router materializes");
    let router_plan = before
        .inline_closure_plan
        .expect("inline mode plans a closure");
    assert!(
        router_plan.original_document_text.contains("ROUTER-BODY"),
        "the router owns closure while it is the active document"
    );

    coordinator(&fx.source_path)
        .adopt(
            request_for(&fx.source_path, &target, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut ActiveDocumentState::initial(),
        )
        .expect("the hop is resolvable and uncontested");

    let after = super::super::super::materialize_harness_prompt(
        &state,
        Some(fx._dir.path()),
        fx._dir.path(),
        None,
        claudine::composition::SchemaStage::Validate,
    )
    .expect("the adopted target materializes");
    let target_plan = after
        .inline_closure_plan
        .expect("inline mode plans a closure for the target too");
    assert!(
        target_plan.original_document_text.contains("TARGET-BODY"),
        "closure ownership moved to the target with the hand-off"
    );
    assert!(
        !target_plan.original_document_text.contains("ROUTER-BODY"),
        "the router is not the closure owner once it has handed off"
    );
}

/// The handoff discards the source's lifecycle *config*, not only the guard's
/// emission ledger.
///
/// Everything between the commit and the target installing its own config is
/// the target's staged boot. If the source's stacks were still loaded there, a
/// failure while booting the target — a malformed target surface, a refused
/// `initialize` command — would fire the *source's* `blocked`/`finalize`: a
/// synthetic closure for a document that already ended. Those failures must
/// surface as their own typed diagnostic instead.
#[test]
fn adopt_discards_the_sources_lifecycle_config_so_the_boot_has_no_stale_catch() {
    let fx = fixture(serde_json::json!({
        "blocked": {"stack": [{"action": {"append_line": ["events.log", "source-blocked"]}}]},
        "finalize": {"stack": [{"action": {"append_line": ["events.log", "source-finalize"]}}]},
    }));
    std::fs::write(&fx.source_path, "---\n---\nbody\n").unwrap();
    let target = fx._dir.path().join("target.md");
    std::fs::write(&target, "---\n---\nbody\n").unwrap();

    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    assert!(
        guard.config().stack(LifecycleSignal::Blocked).is_some(),
        "precondition: the source declares a blocked stack"
    );

    let mut state = prompt_state(&fx.source_path);
    let mut coord = coordinator(&fx.source_path);
    coord
        .adopt(
            request_for(&fx.source_path, &target, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut ActiveDocumentState::initial(),
        )
        .expect("the hop is resolvable and uncontested");

    assert!(
        guard.config().stack(LifecycleSignal::Blocked).is_none(),
        "the source's blocked stack must not survive the hand-off"
    );
    assert!(
        guard.config().stack(LifecycleSignal::Finalize).is_none(),
        "the source's finalize stack must not survive the hand-off"
    );
}

/// A refused hop leaves the guard's config alone along with everything else —
/// the source is still the active document and still owns its own closure.
#[test]
fn a_refused_hop_leaves_the_active_documents_lifecycle_config_installed() {
    let fx = fixture(serde_json::json!({
        "finalize": {"stack": [{"action": {"append_line": ["events.log", "source-finalize"]}}]},
    }));
    std::fs::write(&fx.source_path, "---\n---\nbody\n").unwrap();
    let missing = fx._dir.path().join("does-not-exist.md");

    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    let mut state = prompt_state(&fx.source_path);
    let mut coord = coordinator(&fx.source_path);

    coord
        .adopt(
            request_for(&fx.source_path, &missing, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut ActiveDocumentState::initial(),
        )
        .expect_err("an unresolvable target must be refused");

    assert!(
        guard.config().stack(LifecycleSignal::Finalize).is_some(),
        "a refused hand-off never half-activates a target: the source keeps its \
         own closure"
    );
    assert!(
        !coord.bootstrap_pending(),
        "a refused hand-off owes no boot"
    );
}

// -- typed identity across the error chain ----------------------------------
//
// Phase 12 of `features/2026-07-13-proxy-with`. `ProxyCommitError`'s variants
// carry their concrete causes, but that only helps if the causes are
// *reachable*: every renderer selects its styled block by walking `source()`
// and downcasting (`crate::output::error_walker`), so a `source()` returning
// `None` renders a perfectly good typed error as a bare `Display` string. These
// pin the trait plumbing rather than the data — the data was already right when
// the plumbing was not, and only the plumbing is what production reads.

/// The runtime context these tests all build identically.
fn adopt_context(fx: &Fixture) -> LifecycleRuntimeContext<'_> {
    LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    }
}

/// A rejected hop's typed cause must survive the walk the renderer performs.
#[test]
fn a_rejected_hop_exposes_its_typed_cause_to_the_error_chain() {
    let AdoptFixture { fx, target } = adopt_fixture();
    let emitter = RecordingEmitter::default();
    let ctx = adopt_context(&fx);
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    let mut state = prompt_state(&fx.source_path);
    let mut coord = coordinator(&fx.source_path);

    coord
        .adopt(
            request_for(&fx.source_path, &target, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut ActiveDocumentState::initial(),
        )
        .expect("the first hop is uncontested");
    let _ = coord.take_bootstrap_pending();

    let error = coord
        .adopt(
            request_for(
                &target,
                &fx.source_path,
                vec![fx.source_path.clone(), target.clone()],
            ),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut ActiveDocumentState::initial(),
        )
        .expect_err("an A->B->A cycle must be refused");

    let source = std::error::Error::source(&error).expect(
        "a rejected hop must expose its cause: the renderer selects the styled block \
         by walking `source()`, so `None` renders the typed cycle error as a bare string",
    );
    assert!(
        source
            .downcast_ref::<claudine::composition::CompositionError>()
            .is_some_and(|composition| matches!(
                composition,
                claudine::composition::CompositionError::LifecycleProxyCycle { .. }
            )),
        "the walk must reach the typed `LifecycleProxyCycle` — that downcast is \
         literally what `error_walker` performs to pick the renderer"
    );
}

/// The same for the other variant: a resolution failure's typed `HarnessError`
/// must be reachable too, or a missing proxy target loses its diagnostic facets
/// (`err.code` / `err.detail.*`) at the render boundary.
#[test]
fn an_unresolvable_target_exposes_its_typed_cause_to_the_error_chain() {
    let AdoptFixture { fx, target: _ } = adopt_fixture();
    let emitter = RecordingEmitter::default();
    let ctx = adopt_context(&fx);
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    let mut state = prompt_state(&fx.source_path);
    let mut coord = coordinator(&fx.source_path);

    let missing = fx._dir.path().join("nope.md");
    let error = coord
        .adopt(
            request_for(&fx.source_path, &missing, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut ActiveDocumentState::initial(),
        )
        .expect_err("an unresolvable target must be refused");

    let source = std::error::Error::source(&error)
        .expect("a resolution failure must expose its typed `HarnessError`");
    assert!(
        source
            .downcast_ref::<claudine::harness::HarnessError>()
            .is_some(),
        "the walk must reach the resolver's typed failure, whose `Diagnostic` facets \
         a `failure`/`finalize` stack reads as `err.code` / `err.detail.*`"
    );
}

/// A refused hand-off's `err` payload is built from the concrete cause rather
/// than from a rendered message, so a `failure.stack` branching on `err.kind`
/// branches correctly per variant.
#[test]
fn a_refused_handoff_projects_typed_err_facets_per_variant() {
    use claudine::composition::lifecycle_context::LifecycleErrorInfo;

    let AdoptFixture { fx, target } = adopt_fixture();
    let emitter = RecordingEmitter::default();
    let ctx = adopt_context(&fx);
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    let mut state = prompt_state(&fx.source_path);
    let mut coord = coordinator(&fx.source_path);

    let missing = fx._dir.path().join("nope.md");
    let resolution = coord
        .adopt(
            request_for(&fx.source_path, &missing, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut ActiveDocumentState::initial(),
        )
        .expect_err("an unresolvable target must be refused");
    let resolution_info = LifecycleErrorInfo::from_proxy_commit_error(&resolution);
    assert_eq!(
        resolution_info.kind, "HarnessError",
        "a resolution failure must reach the stack as the resolver's own type, not \
         as a generic action failure"
    );

    coord
        .adopt(
            request_for(&fx.source_path, &target, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut ActiveDocumentState::initial(),
        )
        .expect("the first hop is uncontested");
    let _ = coord.take_bootstrap_pending();
    let rejection = coord
        .adopt(
            request_for(
                &target,
                &fx.source_path,
                vec![fx.source_path.clone(), target.clone()],
            ),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut ActiveDocumentState::initial(),
        )
        .expect_err("an A->B->A cycle must be refused");
    let rejection_info = LifecycleErrorInfo::from_proxy_commit_error(&rejection);

    assert_eq!(
        rejection_info.kind, "CompositionError",
        "a cycle is a composition-level refusal and must say so"
    );
    assert_ne!(
        resolution_info.kind, rejection_info.kind,
        "the two refusal causes must stay distinguishable to a stack that branches \
         on `err.kind` — collapsing both to one label is exactly the stringification \
         this phase removes"
    );
    assert!(!resolution_info.msg.is_empty() && !rejection_info.msg.is_empty());
}
