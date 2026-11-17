//! An action that fails in the `loop:` gate is reported once and leaves the
//! loop's outcome unchanged; this one runner serves every looping owner.

use super::*;
use super::gate_ambient::TextRecorder;
use crate::composition::lifecycle::{LifecycleRuntimeContext, parse_lifecycle_config};

/// Run one pass of a loop whose gate is `item`, with the mutation root at
/// `root`; returns the result and the lines the emitter received.
fn run_gate(item: Value, root: &Path) -> (LoopExecutionResult, Vec<String>) {
    let config = LoopConfig {
        condition: LoopCondition::Until("loop.count >= 1".into()),
        actions: Vec::new(),
        max_iterations: Some(2),
        max_template: None,
        on_rate_limit: None,
    };
    let source = root.join("loop.md");
    let lifecycle = parse_lifecycle_config(&json!({ "loop": { "gate": [item] } }), &source).unwrap();
    let emitter = TextRecorder::default();
    let settings = crate::events::GlobalSettings::default();
    let messaging = crate::messaging::RuntimeMessagingSettings { user: None, repo: None };
    let term = biscuit_terminal::terminal::Terminal::default();
    let lifecycle_ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: &source,
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let effect_engine = darkmatter::effects::EffectEngine::builder().mutation_root(root).auto_rehash(false).build();
    let initial = object(json!({}));
    let result = execute_loop_with_lifecycle(
        &source,
        &config,
        initial.clone(),
        &initial,
        None,
        LoopExecutionOptions::default(),
        &lifecycle,
        &lifecycle_ctx,
        &effect_engine,
        &crate::composition::lifecycle_executor::SystemShellRunner,
        &emitter,
        crate::test_support::process_context(),
        None,
        |_, guard| {
            guard.emit_start_once();
            Ok(LoopIterationOutput::success("done"))
        },
    )
    .unwrap();
    (result, emitter.lines.into_inner().unwrap())
}

/// A workspace whose `root` is the mutation root and whose `outside`
/// directory it does not contain.
fn workspace() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let workspace = TempDir::new().unwrap();
    let root = workspace.path().join("root");
    let outside = workspace.path().join("outside");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    (workspace, root, outside)
}

#[test]
fn a_refused_gate_write_warns_once_naming_target_and_root_and_the_loop_completes() {
    let (_workspace, root, outside) = workspace();
    let target = outside.join("refused.md");

    let (result, lines) = run_gate(json!({ "ensure_file": target.to_string_lossy() }), &root);

    assert!(result.error.is_none(), "a gate action error leaves the outcome: {result:?}");
    assert_eq!(result.iteration_count, 1);
    assert_eq!(lines.len(), 1, "one report: {lines:?}");
    let line = &lines[0];
    assert!(line.starts_with("warn: `loop` action failed: refusing to write outside the mutation root"), "{line}");
    assert!(line.contains(&*target.to_string_lossy()), "names the target: {line}");
    assert!(line.contains("(root: "), "names the root: {line}");
    assert!(!target.exists());
}

#[test]
fn a_refused_gate_write_under_no_error_is_not_reported() {
    let (_workspace, root, outside) = workspace();
    let target = outside.join("refused.md");

    let (result, lines) = run_gate(json!({ "ensure_file": target.to_string_lossy(), "no_error": true }), &root);

    assert!(result.error.is_none(), "{result:?}");
    assert!(lines.is_empty(), "{lines:?}");
    assert!(!target.exists());
}

#[test]
fn an_in_root_gate_write_is_made_and_not_reported() {
    let (_workspace, root, _outside) = workspace();
    let target = root.join("gate.md");

    let (result, lines) = run_gate(json!({ "ensure_file": target.to_string_lossy() }), &root);

    assert!(result.error.is_none(), "{result:?}");
    assert!(lines.is_empty(), "{lines:?}");
    assert!(target.exists());
}

#[test]
fn a_gate_evaluation_error_fails_the_loop_and_is_not_an_action_failure() {
    let (_workspace, root, _outside) = workspace();

    let (result, lines) = run_gate(json!({ "ensure_file": "{{ 1 / 0 }}" }), &root);

    assert!(
        matches!(result.error, Some(CompositionError::LifecycleEvaluationError { .. })),
        "{:?}",
        result.error
    );
    assert!(lines.iter().all(|line| !line.contains("action failed")), "{lines:?}");
}
