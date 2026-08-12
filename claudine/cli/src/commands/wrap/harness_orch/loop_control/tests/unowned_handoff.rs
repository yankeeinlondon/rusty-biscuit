//! What happens to a well-formed `proxy` hand-off raised by a run that owns no
//! active-document coordinator — the direct provider wrappers
//! (`claudine claude`, `claudine goose`, …).
//!
//! Review-7 finding 3. Before this, such a hand-off was adopted in place on the
//! harness-local coordinator, which repointed the harness's own source document
//! and ran the target under the *invocation's* profile, binary, argv entrypoint,
//! and MCP runtime injection — the R3 reduced launch path, and the reason AC10
//! was recorded partial. There is no coordinator on this path that can re-enter
//! the selection/MCP/argv pipeline, so the hand-off is now refused with the
//! typed diagnostic the spec's "Errors and Diagnostics" section names: *any
//! supported transition returned without an owning coordinator able to consume
//! it*.
//!
//! The counterpart rows proving a coordinator-owned hand-off is still consumed
//! normally live in [`super::coordinator_adoption`]; the end-to-end wrapper
//! behavior is `cli/tests/level2_lifecycle_control.rs`
//! (`level2_lifecycle_wrapper_*`).

use super::*;

/// A `failure`-stack proxy naming an existing target, as a wrapper's memory-file
/// lifecycle would raise it.
fn request_to(source: &Path, target: &Path) -> claudine::composition::EvaluatedProxyRequest {
    claudine::composition::EvaluatedProxyRequest::new(
        target.display().to_string(),
        indexmap::IndexMap::new(),
        claudine::composition::ProxyProvenance::new(
            source.to_path_buf(),
            claudine::composition::ActionLocation::new(LifecycleSignal::Failure, 0, 1),
            vec![source.to_path_buf()],
        ),
    )
}

/// A fixture whose every catch stack stamps its own name into `events.log`, so
/// the AC29 routing a refusal owes is observable as an ordered event log rather
/// than a bare count.
fn routing_fixture() -> Fixture {
    fixture(serde_json::json!({
        "failure": {"stack": [{"action": {"append_line": ["events.log", "failure"]}}]},
        "blocked": {"stack": [{"action": {"append_line": ["events.log", "blocked"]}}]},
        "finalize": {"stack": [{"action": {"append_line": ["events.log", "finalize"]}}]},
    }))
}

/// The lines a fixture's stacks stamped, in the order they ran.
fn events(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|s| s.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

/// The refusal is typed, names both the target and the command that cannot host
/// it, and leaves the source document active.
///
/// "Leaves the source active" is the load-bearing half: adoption used to mutate
/// `prompt_state.source_path`, which is what made the target run against a
/// borrowed launch bundle. A refusal that still repointed the document would be
/// the same bug with a better error message.
#[test]
fn an_unowned_handoff_is_refused_with_a_typed_diagnostic() {
    let fx = routing_fixture();
    let target = fx._dir.path().join("target.md");
    std::fs::write(&target, "---\nagent: codex\n---\nbody\n").unwrap();
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
    let eng = engine(fx._dir.path());

    let error = match surface_or_adopt_terminal_proxy(
        // No ledger: this is the direct provider wrapper.
        None,
        Provider::Claude,
        request_to(&fx.source_path, &target),
        Some(fx._dir.path()),
        &mut state,
        &mut guard,
        &fx.materialized,
        &fx.term,
        &eng,
        std::time::Instant::now(),
        None,
        None,
    ) {
        Err(error) => error,
        Ok(_) => panic!("a hand-off with no owning coordinator must be refused"),
    };

    let typed = error
        .downcast_ref::<claudine::composition::CompositionError>()
        .expect("the refusal keeps its typed identity rather than becoming an eyre string");
    let claudine::composition::CompositionError::LifecycleProxyWithoutOwningCoordinator {
        source_path,
        property,
        target: named_target,
        command,
    } = typed
    else {
        panic!("expected LifecycleProxyWithoutOwningCoordinator, got {typed:?}");
    };
    assert_eq!(source_path, &fx.source_path);
    assert_eq!(
        property, "failure.stack[0].action[1]",
        "the diagnostic points at the `proxy` action the user authored",
    );
    assert_eq!(named_target, &target.display().to_string());
    assert_eq!(
        command, "claudine claude",
        "the diagnostic names the invoked wrapper, which is what the user must change",
    );

    assert_eq!(
        state.source_path, fx.source_path,
        "a refused hand-off must not repoint the active document",
    );
}

/// The refusal owes the source's closure exactly as a refused *commit* does: the
/// source is still the active document, so its terminal event and `finalize`
/// each fire exactly once (AC29).
///
/// The provider has launched here — a terminal-recovery proxy is raised from a
/// `failure`/`success` stack — so the owed terminal is `failure`, not `blocked`.
#[test]
fn a_refused_unowned_handoff_routes_the_sources_terminal_then_finalize() {
    let fx = routing_fixture();
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
    guard.mark_provider_launched();
    let mut state = prompt_state(&fx.source_path);
    let eng = engine(fx._dir.path());

    let result = surface_or_adopt_terminal_proxy(
        None,
        Provider::Goose,
        request_to(&fx.source_path, &target),
        Some(fx._dir.path()),
        &mut state,
        &mut guard,
        &fx.materialized,
        &fx.term,
        &eng,
        std::time::Instant::now(),
        None,
        None,
    );
    assert!(result.is_err(), "an unowned hand-off must be refused");

    assert_eq!(
        events(&fx.log_path),
        ["failure", "finalize"],
        "the source owes its terminal event and finalize, once each and in order",
    );
}

/// The typed cause reaches `err.*`, so a wrapper memory file's `blocked`/
/// `finalize` stacks can branch on the refusal the same way they branch on a
/// cycle or a missing target.
#[test]
fn the_refusal_projects_its_typed_identity_into_err() {
    let target = Path::new("/tmp/does-not-matter.md");
    let source = Path::new("/tmp/CLAUDE.md");
    let error = handoff_without_owning_coordinator(&request_to(source, target), Provider::Codex);
    let info = claudine::composition::LifecycleErrorInfo::from_composition_error(&error);

    assert_eq!(info.kind, "CompositionError");
    assert_eq!(info.variant, "LifecycleProxyWithoutOwningCoordinator");
    assert!(
        info.msg.contains("no owning coordinator"),
        "err.msg carries the refusal, got {}",
        info.msg,
    );
}
