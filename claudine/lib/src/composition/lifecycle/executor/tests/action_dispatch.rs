//! action dispatch executor tests.

use super::*;

#[test]
fn top_level_communication_fires_before_stack() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stderr": "top-level first",
                "stack": [{"action": {"info": "then the stack"}}]
            }
        }),
        Path::new("test.md"),
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
        Path::new("test.md"),
    );

    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Stderr("top-level first".to_string()),
            Emitted::Info("then the stack".to_string()),
        ]
    );
}

#[test]
fn success_channel_top_level_and_stack_route_to_emit_success() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "success": "top-level success",
                "stack": [{"action": {"success": "stack success"}}]
            }
        }),
        Path::new("test.md"),
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
        Path::new("test.md"),
    );

    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Success("top-level success".to_string()),
            Emitted::Success("stack success".to_string()),
        ]
    );
}

#[test]
fn top_level_message_fallback_resolves_to_default_for_unknown_optional() {
    // A top-level communication field whose value uses the documented
    // `{{ missing || 'default' }}` migration path must resolve to the
    // fallback at event-time through the real executor (resolve_emit ->
    // resolve_string_value), never erroring on the unknown optional root.
    let config = parse_lifecycle_config(
        &json!({
            "failure": {
                "message": "{{ missing_optional || 'default' }}"
            }
        }),
        Path::new("test.md"),
    )
    .unwrap();

    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("test.md"),
    );

    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("default".to_string())]
    );
}

#[test]
fn stdout_channel_top_level_and_stack_route_to_emit_stdout() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stdout": "top-level stdout",
                "stack": [{"action": {"stdout": "stack stdout"}}]
            }
        }),
        Path::new("test.md"),
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
        Path::new("test.md"),
    );

    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Stdout("top-level stdout".to_string()),
            Emitted::Stdout("stack stdout".to_string()),
        ]
    );
}

#[test]
fn shell_action_runs_command() {
    let config = parse_lifecycle_config(
        &json!({"start": {"stack": [{"action": {"shell": "git status --short"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
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
    assert_eq!(shell.commands(), vec!["git status --short".to_string()]);
}

#[test]
fn shell_nonzero_at_setup_routes_to_failure() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stack": [{
                    "action": {"action": "shell", "command": "false", "on_error": "build failed"}
                }]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(1);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(outcome.action_error.is_some());
    assert!(outcome.routes_to_failure(LifecycleSignal::Start));
    // on_error was surfaced as a warning.
    assert_eq!(recorder.events(), vec![Emitted::Warn("build failed".to_string())]);
}

#[test]
fn shell_nonzero_at_terminal_does_not_route_to_failure() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"shell": "false"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(2);
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
    assert!(outcome.action_error.is_some());
    assert!(!outcome.routes_to_failure(LifecycleSignal::Success));
}

#[test]
fn no_error_suppresses_propagation_and_continues() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stack": [
                    {"action": {"action": "shell", "command": "false", "no_error": true}},
                    {"action": {"info": "reached"}}
                ]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(1);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
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
    assert_eq!(recorder.events(), vec![Emitted::Info("reached".to_string())]);
}

/// `no_error` is scoped to side-effect dispatch failures: an action whose
/// message interpolation *raises* (an unknown root) must still surface as an
/// evaluation error and halt the stack even when `no_error: true` is set.
#[test]
fn no_error_does_not_suppress_evaluation_raise() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stack": [
                    {"action": {"action": "message", "message": "{{spec_fil}}", "no_error": true}},
                    {"action": {"info": "unreached"}}
                ]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"spec_file": "x"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
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
        "an evaluation raise halts despite no_error"
    );
    assert!(outcome.action_error.is_none());
    assert!(recorder.events().is_empty(), "the stack stopped at the raise");
}

#[test]
fn errored_action_stops_remaining_actions() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stack": [{
                    "action": [{"shell": "false"}, {"info": "unreached"}]
                }]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(1);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(outcome.action_error.is_some());
    assert!(recorder.events().is_empty());
}

#[test]
fn explicit_error_control_surfaces_with_reason() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"error": "manual failure"}}]}}),
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
    assert_eq!(
        outcome.control,
        Some(StackControl::Error {
            reason: Some("manual failure".to_string())
        })
    );
    assert!(outcome.action_error.is_none());
}

#[test]
fn retry_count_shorthand_resolves_max_attempts() {
    let config = parse_lifecycle_config(
        &json!({"failure": {"stack": [{"action": {"retry": 3}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(
        outcome.control,
        Some(StackControl::Retry {
            max_attempts: 3,
            backoff: RetryBackoff::Fixed,
            delay: "0s".to_string(),
        })
    );
}

#[test]
fn side_effect_short_form_quoted_args_dispatch_to_engine() {
    // Positional form with an array of quoted string args is the
    // unambiguous path: each arg parses as a string literal, so
    // `prop`/`value` reach the engine verbatim.
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stack": [{"action": {"set_frontmatter": ["state.md", "status", "in-progress"]}}]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (dir, engine) = temp_engine();
    std::fs::write(dir.path().join("state.md"), "---\n---\nbody\n").unwrap();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
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
    let written = std::fs::read_to_string(dir.path().join("state.md")).unwrap();
    assert!(written.contains("status"), "frontmatter updated: {written}");
    assert!(written.contains("in-progress"));
}

#[test]
fn side_effect_long_form_reorders_named_params_positionally() {
    // The parser reorders long-form named params into the verb's
    // positional signature (`http_post` → `[url, body]`), not alphabetical
    // order (which would yield `[body, url]`). The executor then dispatches
    // positionally.
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stack": [{
                    "action": {"action": "http_post", "url": "https://example.com/hook", "body": "hello"}
                }]
            }
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let actions = &config.stack(LifecycleSignal::Start).unwrap()[0].actions;
    match &actions[0].kind {
        LifecycleActionKind::SideEffect(effect) => {
            assert_eq!(effect.verb, "http_post");
            assert_eq!(effect.args.len(), 2);
            assert_eq!(
                effect.args[0],
                Expr::StringLiteral("https://example.com/hook".to_string()),
                "url must be the first positional arg"
            );
        }
        other => panic!("expected SideEffect, got {other:?}"),
    }
}

#[test]
fn side_effect_short_form_routes_through_expression_path() {
    let config = parse_lifecycle_config(
        &json!({"start": {"stack": [{"action": {"ensure_file": "out/log.md"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
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
    assert!(dir.path().join("out/log.md").exists());
}

#[test]
fn emit_top_level_for_signal_fires_comm_without_running_stack() {
    // The stack carries a side-effect action; `emit_top_level_for_signal`
    // must emit only the top-level communication and never touch it.
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stderr": "top-level only",
                "stack": [{"action": {"info": "must not run"}}]
            }
        }),
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

    context.emit_top_level_for_signal(&config);

    assert_eq!(
        recorder.events(),
        vec![Emitted::Stderr("top-level only".to_string())],
        "only the top-level stderr fires; the stack's info action does not"
    );
}

#[test]
fn empty_event_yields_default_outcome() {
    let config = LifecycleConfig::default();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
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
    assert!(recorder.events().is_empty());
}

/// Legacy top-level-only lifecycle prompts (no `stack:`, no `initialize`/
/// `finalize`/`loop`) must emit the same channels and order as before the
/// seven-event model was introduced.
#[test]
fn legacy_top_level_only_prompts_emit_same_channels() {
    let config = parse_lifecycle_config(
        &json!({
            "start": {
                "stderr": "starting",
                "say": "go",
                "effect": "confirmation"
            },
            "success": {
                "stderr": "done",
                "say": "finished",
                "effect": "crowd-applause"
            },
            "blocked": { "stderr": "blocked" },
            "failure": { "stderr": "failed" }
        }),
        Path::new("t.md"),
    )
    .unwrap();

    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();

    let cases: &[(LifecycleSignal, &[Emitted])] = &[
        (
            LifecycleSignal::Start,
            &[
                Emitted::Stderr("starting".to_string()),
                Emitted::Effect("confirmation".to_string()),
                Emitted::Speech("go".to_string()),
            ],
        ),
        (
            LifecycleSignal::Success,
            &[
                Emitted::Stderr("done".to_string()),
                Emitted::Effect("crowd-applause".to_string()),
                Emitted::Speech("finished".to_string()),
            ],
        ),
        (
            LifecycleSignal::Blocked,
            &[Emitted::Stderr("blocked".to_string())],
        ),
        (
            LifecycleSignal::Failure,
            &[Emitted::Stderr("failed".to_string())],
        ),
    ];

    for (signal, expected) in cases {
        let context = ctx(
            *signal,
            &fm,
            None,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        let outcome = context.execute_event(&config);
        assert_eq!(outcome, LifecycleEventOutcome::default(), "{signal:?}");
        assert_eq!(recorder.events(), *expected, "{signal:?}");
        recorder.events.lock().unwrap().clear();
    }
}

/// A resolved effect absent from the catalog reports
/// `LifecycleUnknownEffect` without dispatching a side effect.
#[test]
fn deferred_effect_invalid_resolved_name_reports_unknown_effect() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"effect": "{{effect_name}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"effect_name": "nonexistent-effect-xyz"}));
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
    let info = outcome.action_error.expect("invalid effect fails closed");
    assert_eq!(info.variant, "LifecycleUnknownEffect");
    assert!(recorder.events().is_empty(), "no effect dispatched");
}

/// A deferred effect whose resolved name *is* in the catalog dispatches
/// normally.
#[test]
fn deferred_effect_valid_resolved_name_dispatches() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"effect": "{{effect_name}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"effect_name": "confirmation"}));
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
    assert_eq!(
        recorder.events(),
        vec![Emitted::Effect("confirmation".to_string())]
    );
}

// ── typed transport at the executor's snapshot boundary (spec §D1/§D9) ──────

/// A shell command that never starts projects the runner's typed error into the
/// snapshot instead of the executor rebuilding the prose around it.
///
/// The `err.msg` assertion is the §D10 contract, not a restatement of the
/// format string: this text feeds TTS and webhooks verbatim, and typing the
/// transport must not reword it.
#[test]
fn shell_spawn_failure_projects_the_typed_runner_error() {
    let config = parse_lifecycle_config(
        &json!({"start": {"stack": [{"action": {"shell": "definitely-not-a-binary"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = SpawnFailShell;
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Start,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    let info = outcome.action_error.expect("a spawn failure is a dispatch error");

    // The facet-less action-failure aliases are unmoved: `ShellRunError` is not
    // a registered diagnostic, so selection finds nothing and `err.kind` /
    // `err.variant` keep their action-failure spellings. Authored rules matching
    // on them still match.
    assert_eq!(info.kind, "LifecycleAction");
    assert_eq!(info.variant, "shell");
    assert!(info.snapshot.is_none(), "`shell` names no catalog code");
    assert_eq!(
        info.msg,
        format!(
            "command `definitely-not-a-binary` failed to run: {}",
            SpawnFailShell::io_error()
        )
    );
}

/// `ShellRunError` keeps the `io::Error` recoverable through `Error::source()`
/// (spec §L1) rather than surviving only inside its own `Display`.
#[test]
fn shell_run_error_exposes_its_io_source() {
    let error = ShellRunError::Spawn {
        command: "git push".to_string(),
        source: SpawnFailShell::io_error(),
    };
    let source = std::error::Error::source(&error).expect("the io cause is on the chain");
    let io = source
        .downcast_ref::<std::io::Error>()
        .expect("the concrete io::Error is recoverable, not boxed away");
    assert_eq!(io.kind(), std::io::ErrorKind::NotFound);
}

/// A side-effect dispatch failure reaches the snapshot as the effect engine's
/// typed error. The engine's error is Darkmatter's, so selection finds no
/// *Claudine* diagnostic and the `err.*` aliases stay on their action-failure
/// spellings — the §D10 property that makes typing this site safe.
#[test]
fn side_effect_dispatch_failure_keeps_the_action_failure_aliases() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stack": [{"action": {"append_line": ["/nonexistent-dir/x/y.md", "line"]}}]
            }
        }),
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
    let info = outcome
        .action_error
        .expect("an effect-engine error is a dispatch failure");
    assert_eq!(info.kind, "LifecycleAction");
    assert_eq!(info.variant, "append_line");
    assert!(info.snapshot.is_none());
    assert!(!info.msg.is_empty());
}
