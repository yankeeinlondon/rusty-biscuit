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
            seen.borrow_mut().push(
                ctx.as_layered_overrides(&crate::composition::LayeredOverrides::new())
                    .to_value(),
            );
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
    assert_eq!(seen[0]["counter"], json!(0));
    // User frontmatter property `iteration` is preserved verbatim
    // because loop ambients live under `_loop_*`.
    assert_eq!(seen[0]["iteration"], json!(99));
    assert_eq!(seen[0]["_loop_count"], json!(1));
    assert_eq!(seen[0]["_loop_is_first"], json!(true));
    assert_eq!(seen[0]["_loop_last_output"], json!(""));
    assert_eq!(seen[1]["counter"], json!(1));
    assert_eq!(seen[1]["iteration"], json!(99));
    assert_eq!(seen[1]["_loop_count"], json!(2));
    assert_eq!(seen[1]["_loop_is_first"], json!(false));
    assert_eq!(seen[1]["_loop_last_output"], json!("run 1"));
}

/// `_loop_is_last` is predicted by reading the condition against the state an
/// iteration is about to run with. `counter < 3` from 0 runs four times; only
/// the counter-3 iteration fails the condition, so only it is last.
#[test]
fn computes_is_last_from_condition_on_iteration_state() {
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

    assert_eq!(&*seen.borrow(), &[false, false, false, true]);
}

#[test]
fn computes_is_last_when_max_iterations_is_stopping_condition() {
    let config = LoopConfig {
        condition: LoopCondition::While("true".into()),
        actions: vec![],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    let seen = RefCell::new(Vec::new());
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        Map::new(),
        LoopExecutionOptions {
            max_iterations: Some(2),
            fail_fast: None,
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
fn fail_fast_false_continues_after_iteration_failure() {
    let config = LoopConfig {
        condition: LoopCondition::While("_loop_count < 4".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: Some(false),
        on_rate_limit: None,
    };
    let seen_exit_codes = RefCell::new(Vec::new());
    let result = run_loop(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |ctx| {
            seen_exit_codes
                .borrow_mut()
                .push(ctx.ambient.last_exit_code);
            if ctx.iteration == 1 {
                Ok(LoopIterationOutput::failure(
                    "failed",
                    42,
                    CompositionError::LoopInvalid("iteration failed".into()),
                ))
            } else {
                Ok(LoopIterationOutput::success("ok"))
            }
        },
    )
    .unwrap();

    assert!(result.error.is_none());
    // `_loop_count < 4` stops at the gate after iteration 4. Under
    // `fail_fast: false` the failed iteration 1 still reaches the gate, so its
    // action applies like any other continuing pass: gates 1-3 increment.
    assert_eq!(&*seen_exit_codes.borrow(), &[0, 42, 0, 0]);
    assert_eq!(result.iteration_count, 4);
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(3)));
}

#[test]
fn fail_fast_true_stops_after_iteration_failure() {
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
                CompositionError::LoopInvalid("iteration failed".into()),
            ))
        },
    )
    .unwrap();

    assert_eq!(result.iteration_count, 1);
    assert_eq!(result.final_exit_code, 9);
    assert!(matches!(
        result.error,
        Some(CompositionError::LoopInvalid(_))
    ));
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(0)));
}

#[test]
fn fail_fast_false_discards_failed_action_stage() {
    let config = LoopConfig {
        condition: LoopCondition::While("_loop_count < 3".into()),
        actions: vec![
            LoopAction::Increment("counter".into()),
            LoopAction::Increment("bad".into()),
        ],
        max_iterations: None,
        fail_fast: Some(false),
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

    assert!(result.error.is_none());
    // `_loop_count < 3` stops at the gate after iteration 3. Each continuing
    // gate's action list fails on `bad`, and the whole stage is discarded, so
    // `counter` never moves even though its own increment succeeded.
    assert_eq!(result.iteration_count, 3);
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(0)));
    assert_eq!(result.final_frontmatter.get("bad"), Some(&json!("abc")));
}

#[test]
fn set_template_renders_against_post_executor_iteration_state() {
    // After iteration N runs, `set(stamp, {{_loop_count}})` should land
    // a typed JSON number reflecting the iteration that just ran (N),
    // and `set(echo, {{_loop_last_output}})` should reflect the output
    // the executor produced moments ago.
    let config = LoopConfig {
        condition: LoopCondition::While("_loop_count < 3".into()),
        actions: vec![
            LoopAction::Set {
                prop: "stamp".into(),
                value: Value::String("{{_loop_count}}".into()),
            },
            LoopAction::Set {
                prop: "echo".into(),
                value: Value::String("{{_loop_last_output}}".into()),
            },
        ],
        max_iterations: Some(2),
        fail_fast: None,
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

    assert!(result.error.is_none());
    // After iteration 2 runs and its actions apply, `stamp` should be
    // the JSON number 2 (typed), and `echo` should be the string output
    // captured from iteration 2's executor.
    assert_eq!(result.final_frontmatter.get("stamp"), Some(&json!(2)));
    assert_eq!(result.final_frontmatter.get("echo"), Some(&json!("ran-2")));
}

#[test]
fn counter_below_five_runs_six_iterations() {
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
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
        fail_fast: None,
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
        fail_fast: None,
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
        condition: LoopCondition::While("_loop_count < 4".into()),
        actions: vec![LoopAction::Append {
            prop: "log".into(),
            value: json!({"event": "tick"}),
        }],
        max_iterations: None,
        fail_fast: None,
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
    // Gates 1-3 read `_loop_count < 4` as true and append; gate 4 stops.
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
        condition: LoopCondition::While("_loop_count < 4".into()),
        actions: vec![],
        max_iterations: None,
        fail_fast: None,
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
    // `_loop_count < 4` stops at the gate after iteration 4.
    assert_eq!(result.iteration_count, 4);
    assert_eq!(result.last_output, "run-4");

    let seen = outputs.borrow();
    assert_eq!(seen[0], (1, String::new(), 0));
    assert_eq!(seen[1], (2, "run-1".into(), 0));
    assert_eq!(seen[2], (3, "run-2".into(), 0));
    assert_eq!(seen[3], (4, "run-3".into(), 0));
}

#[test]
fn last_exit_code_reflects_failure_in_next_iteration() {
    let config = LoopConfig {
        condition: LoopCondition::While("_loop_count < 4".into()),
        actions: vec![],
        max_iterations: None,
        fail_fast: Some(false),
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
                Ok(LoopIterationOutput::failure(
                    "bad",
                    7,
                    CompositionError::LoopInvalid("boom".into()),
                ))
            } else {
                Ok(LoopIterationOutput::success("ok"))
            }
        },
    )
    .unwrap();

    assert!(result.error.is_none());
    // `_loop_count < 4` stops at the gate after iteration 4, which succeeded.
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
        fail_fast: None,
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
