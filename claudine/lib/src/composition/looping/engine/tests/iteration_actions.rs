//! iteration actions loop-engine tests.

use super::*;

#[test]
fn loop_iterations_share_one_exact_document_epoch() {
    let source = make_source_with_body(
        &[
            (
                "loop",
                json!({
                    "while": "counter < 1",
                    "actions": ["increment(counter)"]
                }),
            ),
            ("counter", json!(0)),
            ("prepared", json!("{{ ctx.os }}")),
            ("initialize", json!({"stderr": "{{ ctx.os }}"})),
        ],
        "body={{ ctx.os }}",
    );
    let invocation = crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), source.resolved_path.parent().unwrap()).unwrap();
    let requirements =
        darkmatter::markdown::compose::ContextRequirements::for_document(&source.markdown);
    let document_epoch = invocation.begin_document_epoch();
    let prepared_context = document_epoch.capture_launch_context(&requirements);
    let prepare_options = PrepareOptions {
        invocation_context: Some(invocation.clone()),
        document_epoch: Some(document_epoch.clone()),
        prepared_context: Some(prepared_context.clone()),
        ..PrepareOptions::new(crate::test_support::context())
    };
    crate::composition::preflight_document_shell(
        &source,
        &prepare_options,
        &crate::harness::ShellApprovalOptions::default(),
    )
    .unwrap();

    let config = resolve_loop_config(&source).unwrap().unwrap();
    let seed = crate::composition::build_loop_seed_with_lifecycle(
        &source,
        &config,
        prepare_options.clone(),
        CompositionMode::ChainedDocument,
    )
    .unwrap();
    let settings = crate::events::GlobalSettings::default();
    let messaging = crate::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = biscuit_terminal::terminal::Terminal::default();
    let lifecycle_ctx = crate::composition::LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: &source.resolved_path,
        repo_root: source.resolved_path.parent(),
        launch_area: source.resolved_path.parent(),
        context: Some(&prepared_context),
    };
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(source.resolved_path.parent().unwrap())
        .auto_rehash(false)
        .build();

    let result = execute_loop_with_lifecycle(
        &source.resolved_path,
        &config,
        seed.seed,
        &seed.initialize_frontmatter,
        None,
        LoopExecutionOptions::default(),
        &seed.lifecycle,
        &lifecycle_ctx,
        &effect_engine,
        &crate::composition::lifecycle_executor::SystemShellRunner,
        &crate::composition::DefaultLifecycleEmitter,
        crate::test_support::process_context(),
        Some(&document_epoch),
        |_ctx, _guard| {
            let prepared = crate::composition::prepare_direct(&source, prepare_options.clone())?;
            Ok(LoopIterationOutput::success(prepared.prompt))
        },
    )
    .unwrap();

    assert_eq!(result.iteration_count, 2);
    assert_eq!(
        document_epoch.work_snapshot(),
        crate::invocation_context::DocumentEpochWork {
            volatile_observations: Default::default(),
            launch_context_constructions: 1,
            launch_context_extensions: 0,
            ambient_fallbacks: 0,
            prepared_context_consumers: std::collections::BTreeMap::from([
                ("body".to_string(), 3),
                ("effective-frontmatter".to_string(), 3),
                ("lifecycle".to_string(), 1),
                ("loop-condition".to_string(), 1),
                ("preflight".to_string(), 1),
            ]),
        },
        "the loop seed and every iteration must reuse one epoch snapshot"
    );
}

#[test]
fn runs_until_condition_stops_and_commits_actions() {
    let config = counter_loop(3);
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("ok")),
    )
    .unwrap();

    assert!(result.error.is_none());
    // Post-checked: iterations run with counter 0, 1, 2, 3; the gate after
    // the counter-3 iteration reads `3 < 3` and stops without incrementing.
    assert_eq!(result.iteration_count, 4);
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(3)));
    assert_eq!(result.final_exit_code, 0);
    assert_eq!(result.last_output, "ok");
}

/// Each iteration gets the current frontmatter as overrides and its `loop`
/// values as [`LoopIterationContext::ambient`], never as frontmatter.
#[test]
fn injects_ambient_values_and_current_frontmatter() {
    let config = counter_loop(2);
    let seen = RefCell::new(Vec::new());
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0, "iteration": 99})),
        LoopExecutionOptions::default(),
        |ctx| {
            seen.borrow_mut().push((
                ctx.as_layered_overrides(&crate::composition::LayeredOverrides::new())
                    .to_value(),
                ctx.ambient.to_value(),
            ));
            Ok(LoopIterationOutput::success(format!(
                "run {}",
                ctx.iteration
            )))
        },
    )
    .unwrap();

    assert!(result.error.is_none());
    let seen = seen.borrow();
    // `counter < 2` from 0 runs with counter 0, 1, 2.
    assert_eq!(seen.len(), 3);
    let (overrides, ambient) = &seen[0];
    assert_eq!(overrides["counter"], json!(0));
    // User frontmatter property `iteration` is preserved verbatim, and no
    // loop value is written into the frontmatter overrides.
    assert_eq!(overrides["iteration"], json!(99));
    assert_eq!(
        overrides.as_object().unwrap().keys().collect::<std::collections::BTreeSet<_>>(),
        ["counter", "iteration"].iter().map(ToString::to_string).collect::<std::collections::BTreeSet<_>>().iter().collect()
    );
    assert_eq!(ambient["count"], json!(1));
    assert_eq!(ambient["is_first"], json!(true));
    assert_eq!(ambient["last_output"], json!(""));
    let (overrides, ambient) = &seen[1];
    assert_eq!(overrides["counter"], json!(1));
    assert_eq!(overrides["iteration"], json!(99));
    assert_eq!(ambient["count"], json!(2));
    assert_eq!(ambient["is_first"], json!(false));
    assert_eq!(ambient["last_output"], json!("run 1"));
}

/// `loop.is_last` names only the final pass the cap permits; it never reads
/// the condition ahead of the pass. `counter < 3` from 0 runs four passes
/// under the default cap, and none of them is the last permitted one.
#[test]
fn is_last_never_predicts_the_condition() {
    let config = counter_loop(3);
    let seen = RefCell::new(Vec::new());
    run_loop(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |ctx| {
            seen.borrow_mut().push(ctx.ambient.is_last);
            Ok(LoopIterationOutput::success("ok"))
        },
    )
    .unwrap();

    assert_eq!(&*seen.borrow(), &[false, false, false, false]);
}

#[test]
fn computes_is_last_when_max_iterations_is_stopping_condition() {
    let config = LoopConfig {
        condition: LoopCondition::While("true".into()),
        actions: vec![],
        max_iterations: None,
        max_template: None,
        on_rate_limit: None,
    };
    let seen = RefCell::new(Vec::new());
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        Map::new(),
        LoopExecutionOptions {
            max_iterations: Some(2),
            on_rate_limit: None,
            interrupt_check: None,
            pause_reset_margin: None,
        },
        |ctx| {
            seen.borrow_mut().push(ctx.ambient.is_last);
            Ok(LoopIterationOutput::success("ok"))
        },
    )
    .unwrap();

    assert_eq!(&*seen.borrow(), &[false, true]);
    assert!(matches!(
        result.error,
        Some(CompositionError::LoopLimitExceeded {
            cap: 2,
            iteration: 2,
            ..
        })
    ));
}

#[test]
fn a_failed_pass_ends_the_loop_before_its_gate_and_action() {
    let config = counter_loop(3);
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| {
            Ok(LoopIterationOutput::failure(
                "failed",
                9,
                CompositionError::LoopInvalid {
                        source_path: "loop.md".into(),
                        property: "loop".into(),
                        message: "iteration failed".into(),
                    },
            ))
        },
    )
    .unwrap();

    assert_eq!(result.iteration_count, 1);
    assert_eq!(result.final_exit_code, 9);
    assert!(matches!(
        result.error,
        Some(CompositionError::LoopInvalid { .. })
    ));
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(0)));
}

#[test]
fn a_failed_action_stage_commits_nothing_and_fails_the_loop() {
    let config = LoopConfig {
        condition: LoopCondition::While("loop.count < 3".into()),
        actions: vec![
            LoopAction::Increment("counter".into()),
            LoopAction::Increment("bad".into()),
        ],
        max_iterations: None,
        max_template: None,
        on_rate_limit: None,
    };
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0, "bad": "abc"})),
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("ok")),
    )
    .unwrap();

    // The first continuing gate's action list fails on `bad`: the whole stage
    // is discarded, so `counter` never moves even though its own increment
    // succeeded, and the loop fails at that pass.
    assert_eq!(result.iteration_count, 1);
    assert!(
        matches!(&result.error, Some(CompositionError::LoopEvaluationFailed { property, iteration: 1, .. }) if property == "loop.action"),
        "{:?}",
        result.error
    );
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(0)));
    assert_eq!(result.final_frontmatter.get("bad"), Some(&json!("abc")));
}

#[test]
fn set_template_renders_against_post_executor_iteration_state() {
    // After iteration N runs, `set(stamp, {{loop.count}})` should land
    // a typed JSON number reflecting the iteration that just ran (N),
    // and `set(echo, {{loop.last_output}})` should reflect the output
    // the executor produced moments ago.
    let config = LoopConfig {
        condition: LoopCondition::While("loop.count < 3".into()),
        actions: vec![
            LoopAction::Set {
                prop: "stamp".into(),
                value: Value::String("{{loop.count}}".into()),
            },
            LoopAction::Set {
                prop: "echo".into(),
                value: Value::String("{{loop.last_output}}".into()),
            },
        ],
        max_iterations: Some(2),
        max_template: None,
        on_rate_limit: None,
    };
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        Map::new(),
        LoopExecutionOptions::default(),
        |ctx| {
            Ok(LoopIterationOutput::success(format!(
                "ran-{}",
                ctx.iteration
            )))
        },
    )
    .unwrap();

    // Pass 1's actions render against the pass that just ran: a typed JSON
    // number and its captured output. Pass 2 is the last permitted and still
    // asks for another, so the loop is exhausted and no action follows it.
    assert!(
        matches!(result.error, Some(CompositionError::LoopLimitExceeded { cap: 2, iteration: 2, .. })),
        "{:?}",
        result.error
    );
    assert_eq!(result.outcome, Some(LoopOutcome::Exhausted));
    assert_eq!(result.final_frontmatter.get("stamp"), Some(&json!(1)));
    assert_eq!(result.final_frontmatter.get("echo"), Some(&json!("ran-1")));
}

#[test]
fn counter_below_five_runs_six_iterations() {
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        max_template: None,
        on_rate_limit: None,
    };
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("tick")),
    )
    .unwrap();

    assert!(result.error.is_none());
    // Iterations run with counter 0 through 5; the counter-5 gate stops.
    assert_eq!(result.iteration_count, 6);
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(5)));
    assert_eq!(result.last_output, "tick");
}

#[test]
fn until_loop_runs_until_condition_met() {
    // `until: "counter >= 2"` continues while the iteration that just ran
    // had counter < 2.
    let config = LoopConfig {
        condition: LoopCondition::Until("counter >= 2".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        max_template: None,
        on_rate_limit: None,
    };
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("ok")),
    )
    .unwrap();

    assert!(result.error.is_none());
    // Iteration 1 runs with counter=0; gate: 0 >= 2 false -> counter=1
    // Iteration 2 runs with counter=1; gate: 1 >= 2 false -> counter=2
    // Iteration 3 runs with counter=2; gate: 2 >= 2 true -> stop
    assert_eq!(result.iteration_count, 3);
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(2)));
}

#[test]
fn until_loop_with_counter_reaches_target() {
    // Continue until counter >= 3; actions increment each iteration
    let config = LoopConfig {
        condition: LoopCondition::Until("counter >= 3".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        max_template: None,
        on_rate_limit: None,
    };
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("ok")),
    )
    .unwrap();

    assert!(result.error.is_none());
    // Iterations run with counter=0, 1, 2, 3; the gate after the counter=3
    // iteration reads `3 >= 3` and stops.
    assert_eq!(result.iteration_count, 4);
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(3)));
}

#[test]
fn append_accumulates_log_across_iterations() {
    let config = LoopConfig {
        condition: LoopCondition::While("loop.count < 4".into()),
        actions: vec![LoopAction::Append {
            prop: "log".into(),
            value: json!({"event": "tick"}),
        }],
        max_iterations: None,
        max_template: None,
        on_rate_limit: None,
    };
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        object(json!({"log": ""})),
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("ok")),
    )
    .unwrap();

    assert!(result.error.is_none());
    // Gates 1-3 read `loop.count < 4` as true and append; gate 4 stops.
    assert_eq!(result.iteration_count, 4);
    let log = result
        .final_frontmatter
        .get("log")
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(log.matches("tick").count(), 3);
}

#[test]
fn last_output_and_last_exit_code_propagate() {
    let config = LoopConfig {
        condition: LoopCondition::While("loop.count < 4".into()),
        actions: vec![],
        max_iterations: None,
        max_template: None,
        on_rate_limit: None,
    };
    let outputs = RefCell::new(Vec::new());
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        object(json!({})),
        LoopExecutionOptions::default(),
        |ctx| {
            let out = format!("run-{}", ctx.iteration);
            outputs.borrow_mut().push((
                ctx.iteration,
                ctx.ambient.last_output.clone(),
                ctx.ambient.last_exit_code,
            ));
            Ok(LoopIterationOutput::success(out))
        },
    )
    .unwrap();

    assert!(result.error.is_none());
    // `loop.count < 4` stops at the gate after iteration 4.
    assert_eq!(result.iteration_count, 4);
    assert_eq!(result.last_output, "run-4");

    let seen = outputs.borrow();
    assert_eq!(seen[0], (1, json!(""), 0));
    assert_eq!(seen[1], (2, json!("run-1"), 0));
    assert_eq!(seen[2], (3, json!("run-2"), 0));
    assert_eq!(seen[3], (4, json!("run-3"), 0));
}

/// A pass whose own lifecycle settled it as `success` keeps its non-zero
/// exit status in `loop.last_exit_code`; only a failed pass ends the loop.
#[test]
fn last_exit_code_reports_a_succeeded_pass_with_a_non_zero_status() {
    let config = LoopConfig {
        condition: LoopCondition::While("loop.count < 4".into()),
        actions: vec![],
        max_iterations: None,
        max_template: None,
        on_rate_limit: None,
    };
    let exit_codes = RefCell::new(Vec::new());
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        object(json!({})),
        LoopExecutionOptions::default(),
        |ctx| {
            exit_codes.borrow_mut().push(ctx.ambient.last_exit_code);
            if ctx.iteration == 2 {
                Ok(LoopIterationOutput { exit_code: 7, ..LoopIterationOutput::success("bad") })
            } else {
                Ok(LoopIterationOutput::success("ok"))
            }
        },
    )
    .unwrap();

    assert!(result.error.is_none());
    // `loop.count < 4` stops at the gate after iteration 4, which succeeded.
    assert_eq!(result.iteration_count, 4);
    assert_eq!(result.final_exit_code, 0);

    let seen = exit_codes.borrow();
    assert_eq!(&*seen, &[0, 0, 7, 0]);
}

#[test]
fn until_file_exists_resolves_against_prompt_parent() {
    // `until="file_exists('artifact')"` continues while the artifact is
    // absent and stops at the gate after the iteration that creates it under
    // the prompt's parent directory — proving the loop condition's read-side
    // function resolves against the prompt document root, re-probed at each
    // gate.
    let dir = tempfile::TempDir::new().unwrap();
    let prompt_path = dir.path().join("loop.md");
    let artifact = dir.path().join("artifact");

    let config = LoopConfig {
        condition: LoopCondition::Until("file_exists('artifact')".into()),
        actions: vec![],
        max_iterations: None,
        max_template: None,
        on_rate_limit: None,
    };

    let result = run_loop(
        &prompt_path,
        &config,
        Map::new(),
        LoopExecutionOptions::default(),
        |ctx| {
            // Create the artifact on the third iteration; the gates after
            // iterations 1 and 2 see it absent and keep looping.
            if ctx.iteration == 3 {
                std::fs::write(&artifact, "done").unwrap();
            }
            Ok(LoopIterationOutput::success("ok"))
        },
    )
    .unwrap();

    assert!(result.error.is_none(), "got: {result:?}");
    assert_eq!(result.iteration_count, 3);
}

// ── Rate-limit policy tests ──────────────────────────────────────────

#[test]
fn an_unresolved_cap_template_is_refused_before_any_iteration() {
    let config = LoopConfig {
        condition: LoopCondition::While("true".into()),
        actions: vec![],
        max_iterations: None,
        max_template: Some("{{ cap }}".into()),
        on_rate_limit: None,
    };
    let launches = RefCell::new(0);
    let err = run_loop(
        Path::new("loop.md"),
        &config,
        Map::new(),
        LoopExecutionOptions::default(),
        |_| {
            *launches.borrow_mut() += 1;
            Ok(LoopIterationOutput::success("ok"))
        },
    )
    .unwrap_err();

    assert_eq!(*launches.borrow(), 0, "the default cap is never used in its place");
    assert!(
        matches!(&err, CompositionError::LoopInvalid { property, .. } if property == "loop.max"),
        "got {err:?}"
    );
}
