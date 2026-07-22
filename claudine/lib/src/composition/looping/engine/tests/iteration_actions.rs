//! iteration actions loop-engine tests.

use super::*;

#[test]
fn runs_until_condition_stops_and_commits_actions() {
    let config = counter_loop(3);
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("ok")),
    )
    .unwrap();

    assert!(result.error.is_none());
    assert_eq!(result.iteration_count, 3);
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(3)));
    assert_eq!(result.final_exit_code, 0);
    assert_eq!(result.last_output, "ok");
}

#[test]
fn injects_ambient_values_and_current_frontmatter() {
    let config = counter_loop(2);
    let seen = RefCell::new(Vec::new());
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0, "iteration": 99})),
        LoopExecutionOptions::default(),
        |ctx| {
            seen.borrow_mut().push(ctx.as_set_overrides());
            Ok(LoopIterationOutput::success(format!(
                "run {}",
                ctx.iteration
            )))
        },
    )
    .unwrap();

    assert!(result.error.is_none());
    let seen = seen.borrow();
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

#[test]
fn computes_is_last_from_post_action_condition() {
    let config = counter_loop(3);
    let seen = RefCell::new(Vec::new());
    execute_loop_with_config(
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

    assert_eq!(&*seen.borrow(), &[false, false, true]);
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
    let result = execute_loop_with_config(
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
    let result = execute_loop_with_config(
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
    assert_eq!(&*seen_exit_codes.borrow(), &[0, 42, 0]);
    assert_eq!(result.iteration_count, 3);
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(2)));
}

#[test]
fn fail_fast_true_stops_after_iteration_failure() {
    let config = counter_loop(3);
    let result = execute_loop_with_config(
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
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0, "bad": "abc"})),
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("ok")),
    )
    .unwrap();

    assert!(result.error.is_none());
    assert_eq!(result.iteration_count, 2);
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
    let result = execute_loop_with_config(
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
fn five_iteration_counter_loop() {
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("tick")),
    )
    .unwrap();

    assert!(result.error.is_none());
    assert_eq!(result.iteration_count, 5);
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(5)));
    assert_eq!(result.last_output, "tick");
}

#[test]
fn until_loop_runs_until_condition_met() {
    // until: "counter >= 2" means "continue while counter < 2"
    // actions increment counter each iteration, so 2 iterations run
    let config = LoopConfig {
        condition: LoopCondition::Until("counter >= 2".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("ok")),
    )
    .unwrap();

    assert!(result.error.is_none());
    // Iteration 1: counter=0 < 2 -> continue -> counter=1
    // Iteration 2: counter=1 < 2 -> continue -> counter=2
    // Iteration 3: counter=2 >= 2 -> stop
    assert_eq!(result.iteration_count, 2);
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
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("ok")),
    )
    .unwrap();

    assert!(result.error.is_none());
    // Iteration 1: counter=0 < 3 -> continue -> counter=1
    // Iteration 2: counter=1 < 3 -> continue -> counter=2
    // Iteration 3: counter=2 < 3 -> continue -> counter=3
    // Iteration 4: counter=3 >= 3 -> stop
    assert_eq!(result.iteration_count, 3);
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
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"log": ""})),
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("ok")),
    )
    .unwrap();

    assert!(result.error.is_none());
    assert_eq!(result.iteration_count, 3);
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
    let result = execute_loop_with_config(
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
    assert_eq!(result.iteration_count, 3);
    assert_eq!(result.last_output, "run-3");

    let seen = outputs.borrow();
    assert_eq!(seen[0], (1, String::new(), 0));
    assert_eq!(seen[1], (2, "run-1".into(), 0));
    assert_eq!(seen[2], (3, "run-2".into(), 0));
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
    let result = execute_loop_with_config(
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
    assert_eq!(result.iteration_count, 3);
    assert_eq!(result.final_exit_code, 0);

    let seen = exit_codes.borrow();
    assert_eq!(&*seen, &[0, 0, 7]);
}

#[test]
fn until_file_exists_resolves_against_prompt_parent() {
    // `until="file_exists('artifact')"` continues while the artifact is
    // absent and stops once the executor creates it under the prompt's
    // parent directory — proving the loop condition's read-side function
    // resolves against the prompt document root, re-probed each iteration.
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

    let result = execute_loop_with_config(
        &prompt_path,
        &config,
        Map::new(),
        LoopExecutionOptions::default(),
        |ctx| {
            // Create the artifact on the third iteration; earlier passes
            // see it absent and keep looping.
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


