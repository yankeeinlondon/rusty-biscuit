//! event time interpolation executor tests.

use super::*;

#[test]
fn message_interpolates_frontmatter_in_literal() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"action": "info", "message": "done {{ name }}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"name": "alpha"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(recorder.events(), vec![Emitted::Info("done alpha".to_string())]);
}

fn io_err(msg: &str) -> LifecycleErrorInfo {
    LifecycleErrorInfo {
        kind: "ClaudineError",
        variant: "Io".to_string(),
        msg: msg.to_string(),
        facets: None,
    }
}

/// Top-level `failure.message: "{{err.msg}}"` is a deferred (raw) key that
/// must interpolate the real error at event-time — the original bug.
#[test]
fn top_level_message_interpolates_err_at_event_time() {
    let config = parse_lifecycle_config(
        &json!({"failure": {"message": "❌️ {{err.msg}}"}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let err = io_err("disk full");
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("❌️ disk full".to_string())]
    );
}

/// A `failure` stack `message(❌️ {{err.msg}})` renders the real error
/// end-to-end through composition (parse → executor → DM2).
#[test]
fn stack_message_interpolates_err_at_event_time() {
    let config = parse_lifecycle_config(
        &json!({"failure": {"stack": [{"action": {"message": "❌️ {{err.msg}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let err = io_err("disk full");
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("❌️ disk full".to_string())]
    );
}

/// A mixed body resolves both an early-binding frontmatter span (`phase`)
/// and a late-binding global span (`err.msg`) at event-time.
#[test]
fn mixed_body_resolves_both_spans_at_event_time() {
    let config = parse_lifecycle_config(
        &json!({"failure": {"stack": [
            {"action": {"message": "phase {{phase}} failed: {{err.msg}}"}}
        ]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"phase": 6}));
    let err = io_err("disk full");
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("phase 6 failed: disk full".to_string())]
    );
}

/// Currentness: the same lifecycle config re-resolves `{{phase}}` against
/// each event's live frontmatter, so a loop message reflects the current
/// iteration's value (the raw deferred subtree stays the stored definition).
#[test]
fn message_reflects_current_frontmatter_per_event() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "iter {{phase}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let harness = Harness::default();
    for (phase, expected) in [(1u64, "iter 1"), (2u64, "iter 2")] {
        let fm = map(json!({ "phase": phase }));
        let recorder = Recorder::default();
        let context = ctx(
            LifecycleSignal::Success,
            &fm,
            None,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        context.execute_event(&config);
        assert_eq!(recorder.events(), vec![Emitted::Message(expected.to_string())]);
    }
}

/// Event-time rendering stays identical to Darkmatter subtree composition;
/// Claudine does not introduce a second interpolation engine.
#[test]
fn event_time_rendering_matches_compose() {
    use darkmatter::markdown::compose::EffectiveStateBuilder;
    use darkmatter::markdown::compose::subtree::{SubtreeStrictness, compose_subtree};

    let template = "phase {{phase}}: {{err.msg}}";
    let err = io_err("disk full");

    // Executor path.
    let config = parse_lifecycle_config(
        &json!({"failure": {"stack": [{"action": {"message": template}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"phase": 6}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    let Emitted::Message(executor_text) = &recorder.events()[0] else {
        panic!("expected a Message emission");
    };

    // Direct DM2 subtree compose for the same string + data.
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(
            [("phase".to_string(), json!(6))].into_iter().collect(),
        )
        .with_context(
            darkmatter::markdown::compose::ComposeContext::capture_for_content(
                Path::new("."),
                "",
            ),
        )
        .build()
        .unwrap();
    let compose_value = compose_subtree(
        &json!(template),
        &state,
        lifecycle_injected_globals(Some(&err), None, None),
        SubtreeStrictness::Lenient,
    )
    .unwrap();
    assert_eq!(executor_text, compose_value.as_str().unwrap());
    assert_eq!(executor_text, "phase 6: disk full");
}

/// Phase 7 reproduction fixture (acceptance criterion 1): a top-level
/// `failure` block shaped like `prompts/implement-plan.md` — both a `say`
/// and a `message` field mixing an early-binding frontmatter span
/// (`{{phase}}`) with the late-binding `err` global — renders the real
/// values when the failure event fires. This is the original bug: before
/// late binding, `{{err.msg}}` collapsed to empty at compose time.
#[test]
fn reproduction_failure_block_renders_real_error_at_event_time() {
    let config = parse_lifecycle_config(
        &json!({"failure": {
            "say": "Phase {{phase}} ran into problems!",
            "message": "❌️ phase {{phase}} failed: {{err.msg}}",
        }}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"phase": 6}));
    let err = io_err("disk full");
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Message("❌️ phase 6 failed: disk full".to_string()),
            Emitted::Speech("Phase 6 ran into problems!".to_string()),
        ]
    );
}

// ── Phase 5 (C4): fail-closed event-time resolution ─────────────────

/// A reference whose root is a *known* frontmatter key that resolves to
/// `null`/empty renders empty and does **not** error (5.6).
#[test]
fn known_but_empty_reference_renders_empty() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "spec={{spec_file}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"spec_file": null}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(recorder.events(), vec![Emitted::Message("spec=".to_string())]);
}

/// A typo (an unknown root) fails closed: the action errors and nothing is
/// dispatched (5.6).
#[test]
fn unknown_root_typo_fails_closed() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "{{spec_fil}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"spec_file": "x"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(
        outcome.evaluation_error.is_some(),
        "typo must fail closed through the evaluation channel"
    );
    assert!(outcome.action_error.is_none());
    assert!(recorder.events().is_empty(), "nothing dispatched");
}

/// A top-level field with an unknown root fails the event closed before any
/// side effect is dispatched (5.5).
#[test]
fn top_level_unknown_root_fails_event_closed() {
    let config = parse_lifecycle_config(
        &json!({"success": {"message": "{{spec_fil}}"}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(
        outcome.evaluation_error.is_some(),
        "a top-level interpolation raise is an evaluation error"
    );
    assert!(outcome.action_error.is_none());
    assert!(recorder.events().is_empty());
}

/// Post-DM2 leak guard (5.4): a known reference whose resolved value is
/// itself raw template text leaves a surviving `{{ … }}` span, which fails
/// before dispatch.
#[test]
fn post_dm2_surviving_span_fails_before_dispatch() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "{{tmpl}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    // The frontmatter value is literal template text — resolving `{{tmpl}}`
    // yields `{{x}}`, a surviving recognized span.
    let fm = map(json!({"tmpl": "{{x}}"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(
        outcome.evaluation_error.is_some(),
        "surviving span is an evaluation-layer failure"
    );
    assert!(outcome.action_error.is_none());
    assert!(recorder.events().is_empty(), "no side effect dispatched");
}

