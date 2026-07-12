//! Tests for the composition loop engine.

use std::cell::RefCell;
use std::path::Path;

use darkmatter::markdown::{Frontmatter, Markdown};
use serde_json::json;
use tempfile::TempDir;

use super::*;
use super::super::seed::build_loop_seed_with_lifecycle;
use crate::composition::prepare::prepare_direct;
use crate::composition::types::{LoopAction, LoopCondition};

/// The loop-engine wiring captures non-empty `timing`/`current` globals so
/// loop lifecycle events (`initialize`, `loop`) expose `timing.document_ms`
/// and a populated `current.env`, rather than the pre-fix `None`/`None`.
#[test]
fn capture_loop_lifecycle_globals_populates_timing_and_env() {
    let loop_start = std::time::Instant::now();
    let (timing, current) =
        capture_loop_lifecycle_globals(Some(Path::new(".")), None, loop_start);

    assert!(
        timing.document_ms.is_some(),
        "document_ms is populated from the run-level instant"
    );
    assert!(
        timing.total_ms.is_some(),
        "total_ms is populated because a run_start instant is supplied"
    );
    assert!(
        current.env.is_object() && !current.env.as_object().unwrap().is_empty(),
        "current.env is a non-empty process-environment snapshot"
    );
    // base_dir = "." → ctx is captured (at minimum ctx.today).
    assert!(
        current.ctx.get("today").is_some(),
        "current.ctx snapshot carries today"
    );
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn counter_loop(max: usize) -> LoopConfig {
    LoopConfig {
        condition: LoopCondition::While(format!("counter < {max}")),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    }
}

fn make_source(frontmatter: &[(&str, serde_json::Value)]) -> ResolvedCompositionSource {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("loop.md");
    let mut fm = darkmatter::markdown::Frontmatter::new();
    for (key, value) in frontmatter {
        fm.insert(key, value.clone()).unwrap();
    }
    let md = darkmatter::markdown::Markdown::with_frontmatter(fm, "Body");
    std::fs::write(&file, md.as_string()).unwrap();
    let original_text = std::fs::read_to_string(&file).unwrap();
    let markdown: darkmatter::markdown::Markdown = original_text.clone().into();
    ResolvedCompositionSource {
        original_ref: file.to_string_lossy().to_string(),
        resolved_path: file,
        original_text,
        markdown,
    }
}

fn make_source_with_body(
    frontmatter: &[(&str, serde_json::Value)],
    body: &str,
) -> ResolvedCompositionSource {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("loop.md");
    let mut fm = Frontmatter::new();
    for (key, value) in frontmatter {
        fm.insert(key, value.clone()).unwrap();
    }
    let md = Markdown::with_frontmatter(fm, body);
    std::fs::write(&file, md.as_string()).unwrap();
    let original_text = std::fs::read_to_string(&file).unwrap();
    let markdown: Markdown = original_text.clone().into();
    ResolvedCompositionSource {
        original_ref: file.to_string_lossy().to_string(),
        resolved_path: file,
        original_text,
        markdown,
    }
}

#[test]
fn build_loop_seed_resolves_control_variables_and_omits_derived() {
    let source = make_source(&[
        ("phase", json!("{{ initial_phase || 1 }}")),
        ("total_phases", json!("{{ 6 }}")),
        (
            "pass_icon",
            json!("{{ _loop_is_last ? '✅' : '🧑‍💻' }}"),
        ),
        (
            "loop",
            json!({"until": "phase > total_phases", "action": "increment(phase)"}),
        ),
    ]);
    let config = resolve_loop_config(&source).unwrap().unwrap();
    let options = PrepareOptions {
        set_overrides: Some(json!({"initial_phase": 1})),
        ..PrepareOptions::default()
    };

    let seed =
        build_loop_seed(&source, &config, options, CompositionMode::ChainedDocument).unwrap();

    assert_eq!(seed.get("phase"), Some(&json!(1)));
    assert_eq!(seed.get("total_phases"), Some(&json!(6)));
    assert!(!seed.contains_key("pass_icon"), "derived keys must not be lifted into the seed");
    assert_eq!(seed.get("initial_phase"), Some(&json!(1)));
}

/// The control-variable-only seed drops every lifecycle event block, so
/// parsing lifecycle from it yields an empty config — the root cause of the
/// loop-path lifecycle bug. `build_loop_seed_with_lifecycle` instead parses
/// lifecycle from the full composed frontmatter, so the event blocks and
/// the `loop:` gate concerns survive even though they are absent from the
/// seed.
#[test]
fn build_loop_seed_with_lifecycle_carries_event_blocks_dropped_from_seed() {
    let source = make_source(&[
        ("phase", json!(1)),
        (
            "loop",
            json!({
                "until": "phase > 2",
                "action": "increment(phase)",
                "stack": [{"action": {"append_line": ["events.log", "gate"]}}],
            }),
        ),
        ("initialize", json!({"stack": [{"action": {"append_line": ["events.log", "initialize"]}}]})),
        ("start", json!({"stack": [{"action": {"append_line": ["events.log", "start"]}}]})),
        ("finalize", json!({"stack": [{"action": {"append_line": ["events.log", "finalize"]}}]})),
    ]);
    let config = resolve_loop_config(&source).unwrap().unwrap();

    let result = build_loop_seed_with_lifecycle(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::ChainedDocument,
    )
    .unwrap();

    // The seed itself carries only control variables, never event blocks.
    assert!(!result.seed.contains_key("initialize"));
    assert!(!result.seed.contains_key("finalize"));

    // The parsed lifecycle, however, carries every event block plus the
    // `loop:` gate concerns — exactly what the loop runner needs.
    assert!(!result.lifecycle.is_empty(), "lifecycle must not be empty");
    assert!(result.lifecycle.stacks.initialize.is_some());
    assert!(result.lifecycle.stacks.start.is_some());
    assert!(result.lifecycle.stacks.finalize.is_some());
    assert!(
        result.lifecycle.stacks.loop_gate.is_some(),
        "the loop gate's lifecycle concerns must survive into the parsed config"
    );
}

#[test]
fn build_loop_seed_inline_mode_resolves_prompt_frontmatter_with_empty_body() {
    let source = make_source_with_body(
        &[
            ("prompt", json!("Build phase {{phase}}")),
            ("phase", json!("{{ start_phase || 1 }}")),
            (
                "loop",
                json!({"while": "phase < 2", "action": "increment(phase)"}),
            ),
        ],
        "",
    );
    let config = resolve_loop_config(&source).unwrap().unwrap();

    // Inline mode composes the `prompt:` frontmatter value as the body,
    // so an empty document body still resolves and the control variable
    // `phase` lifts into the seed.
    let seed = build_loop_seed(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::InlineFrontmatterPrompt,
    )
    .expect("inline seed should resolve from prompt frontmatter with empty body");
    assert_eq!(seed.get("phase"), Some(&json!(1)));

    // Direct mode composes the document body itself, which is empty, so
    // seeding fails before iteration 1 with `ComposedBodyEmpty`. This
    // locks in the mode distinction that motivates parameterizing
    // `build_loop_seed` by `CompositionMode`.
    let direct = build_loop_seed(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::ChainedDocument,
    );
    assert!(
        matches!(direct, Err(CompositionError::ComposedBodyEmpty { .. })),
        "direct mode with empty body should fail seed resolution; got {direct:?}"
    );
}

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

fn throttled(message: Option<&str>, reset_in_secs: Option<i64>) -> RateLimitInfo {
    RateLimitInfo {
        is_throttled: Some(true),
        retry_after_ms: None,
        message: message.map(str::to_string),
        reset_at: reset_in_secs.map(|s| chrono::Utc::now() + chrono::Duration::seconds(s)),
    }
}

#[test]
fn rate_limit_continue_policy_proceeds_without_pausing() {
    // While-condition exits after 2 successful iterations. Even though
    // iteration 1 carries a rate-limit trailer, the `Continue` policy
    // means we don't pause and we don't abort.
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 2".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: Some(OnRateLimit::Continue),
    };

    let observed = RefCell::new(Vec::new());
    let start = std::time::Instant::now();
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |ctx| {
            observed.borrow_mut().push(ctx.iteration);
            Ok(LoopIterationOutput::success("ok")
                .with_rate_limit(Some(throttled(Some("hit cap"), Some(60)))))
        },
    )
    .unwrap();
    let elapsed = start.elapsed();

    assert!(result.error.is_none(), "got: {result:?}");
    assert_eq!(result.iteration_count, 2);
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "Continue policy should not sleep; elapsed = {elapsed:?}"
    );
}

#[test]
fn rate_limit_abort_policy_halts_with_structured_error() {
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: Some(OnRateLimit::Abort),
    };

    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| {
            Ok(LoopIterationOutput::success("ok")
                .with_rate_limit(Some(throttled(Some("usage cap"), Some(60))))
                .with_attribution(Some("k2p6".into()), Some("kimi-for-coding".into())))
        },
    )
    .unwrap();

    assert_eq!(result.iteration_count, 1);
    match result.error {
        Some(CompositionError::LoopRateLimited {
            iteration,
            provider,
            model,
            reset_at,
            message,
            ..
        }) => {
            assert_eq!(iteration, 1);
            assert_eq!(provider.as_deref(), Some("k2p6"));
            assert_eq!(model.as_deref(), Some("kimi-for-coding"));
            assert!(reset_at.is_some());
            assert_eq!(message.as_deref(), Some("usage cap"));
        }
        other => panic!("expected LoopRateLimited, got {other:?}"),
    }
}

#[test]
fn rate_limit_pause_with_no_reset_falls_back_to_abort() {
    // No `reset_at` → Pause cannot wait an unbounded amount, so we
    // abort cleanly with the same structured error.
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: Some(OnRateLimit::Pause),
    };

    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| {
            Ok(LoopIterationOutput::success("ok")
                .with_rate_limit(Some(throttled(Some("no reset clock"), None))))
        },
    )
    .unwrap();

    assert_eq!(result.iteration_count, 1);
    assert!(
        matches!(
            result.error,
            Some(CompositionError::LoopRateLimited { reset_at: None, .. })
        ),
        "got: {:?}",
        result.error
    );
}

#[test]
fn rate_limit_pause_skipped_on_final_iteration() {
    // When the loop is already going to exit (is_last == true), the
    // engine must not pause — it would block for nothing.
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 1".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: Some(OnRateLimit::Pause),
    };

    let start = std::time::Instant::now();
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| {
            // Iteration 1 IS the last (counter goes 0 → 1, condition fails next round).
            Ok(LoopIterationOutput::success("ok")
                .with_rate_limit(Some(throttled(Some("trailer on last"), Some(300)))))
        },
    )
    .unwrap();
    let elapsed = start.elapsed();

    assert!(result.error.is_none(), "got: {result:?}");
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "should skip pause on last iteration; elapsed = {elapsed:?}"
    );
}

#[test]
fn rate_limit_default_policy_is_pause() {
    // Neither options nor config set on_rate_limit. With no reset_at,
    // the default Pause falls back to Abort.
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
        |_ctx| {
            Ok(LoopIterationOutput::success("ok").with_rate_limit(Some(throttled(None, None))))
        },
    )
    .unwrap();

    assert!(
        matches!(result.error, Some(CompositionError::LoopRateLimited { .. })),
        "default should be Pause→Abort fallback; got: {:?}",
        result.error
    );
}

#[test]
fn rate_limit_pause_sleeps_until_reset_then_continues() {
    // With Pause policy the engine must sleep until `reset_at` (plus the
    // safety margin) before running the next iteration. We inject a zero
    // margin and a 1s reset so the test verifies the wait-then-continue
    // behaviour without burning the production 5s margin.
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 2".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: Some(OnRateLimit::Pause),
    };

    let start = std::time::Instant::now();
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions {
            pause_reset_margin: Some(std::time::Duration::ZERO),
            ..LoopExecutionOptions::default()
        },
        |ctx| {
            let rl = if ctx.iteration == 1 {
                // 1s reset + 0 margin → the engine pauses ~1s before
                // proceeding to iteration 2.
                Some(throttled(Some("brief cap"), Some(1)))
            } else {
                None
            };
            Ok(LoopIterationOutput::success("ok").with_rate_limit(rl))
        },
    )
    .unwrap();
    let elapsed = start.elapsed();

    assert!(result.error.is_none(), "got: {result:?}");
    assert_eq!(result.iteration_count, 2);
    // 1s reset + 0 margin → it must have waited, but not unbounded.
    assert!(
        elapsed >= std::time::Duration::from_millis(500),
        "expected ~1s pause; elapsed = {elapsed:?}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "pause should not be unbounded; elapsed = {elapsed:?}"
    );
}

#[test]
fn rate_limit_pause_is_interrupt_aware() {
    // When the interrupt_check callback returns true, the pause exits
    // immediately and the engine returns Proceed (caller will see the
    // interrupt on the next iteration via its wrapped executor).
    use std::sync::atomic::{AtomicBool, Ordering};

    // Static flag because LoopExecutionOptions.interrupt_check is a
    // bare `fn() -> bool` (Copy).
    static FIRED: AtomicBool = AtomicBool::new(false);
    FIRED.store(true, Ordering::SeqCst);
    fn always_interrupted() -> bool {
        FIRED.load(Ordering::SeqCst)
    }

    let config = LoopConfig {
        condition: LoopCondition::While("counter < 2".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: Some(OnRateLimit::Pause),
    };

    let start = std::time::Instant::now();
    let _result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions {
            interrupt_check: Some(always_interrupted),
            ..LoopExecutionOptions::default()
        },
        |ctx| {
            let rl = if ctx.iteration == 1 {
                // Long reset to prove the interrupt cut it short.
                Some(throttled(Some("long cap"), Some(60)))
            } else {
                None
            };
            Ok(LoopIterationOutput::success("ok").with_rate_limit(rl))
        },
    )
    .unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "interrupt should cut pause short; elapsed = {elapsed:?}"
    );
    FIRED.store(false, Ordering::SeqCst);
}

// ── Seeded-loop integration tests ────────────────────────────────────

#[test]
fn seeded_loop_repro_runs_to_completion_with_live_derived_variable() {
    let source = make_source_with_body(
        &[
            ("phase", json!("{{ start_phase || 1 }}")),
            ("total_phases", json!(6)),
            (
                "pass_icon",
                json!("{{ _loop_is_last ? '✅' : '🧑‍💻' }}"),
            ),
            (
                "loop",
                json!({"until": "phase > total_phases", "action": "increment(phase)"}),
            ),
        ],
        "Implement Phase {{ phase }} of {{ total_phases }}",
    );
    let config = resolve_loop_config(&source).unwrap().unwrap();
    let seed = build_loop_seed(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::ChainedDocument,
    )
    .unwrap();

    let captured = RefCell::new(Vec::new());
    let result = execute_loop_with_config(
        &source.resolved_path,
        &config,
        seed,
        LoopExecutionOptions::default(),
        |ctx| {
            let prepared = prepare_direct(
                &source,
                PrepareOptions {
                    set_overrides: Some(ctx.as_set_overrides()),
                    ..PrepareOptions::default()
                },
            )?;
            let pass_icon = prepared
                .effective_frontmatter
                .as_object()
                .and_then(|fm| fm.get("pass_icon"))
                .cloned();
            let body = prepared.prompt.clone();
            captured.borrow_mut().push((
                ctx.iteration,
                ctx.frontmatter.get("phase").cloned(),
                body,
                pass_icon,
            ));
            Ok(LoopIterationOutput::success(prepared.prompt))
        },
    )
    .unwrap();

    assert!(result.error.is_none(), "expected clean run, got {result:?}");
    assert_eq!(result.iteration_count, 6);
    assert_eq!(result.final_frontmatter.get("phase"), Some(&json!(7)));

    let seen = captured.into_inner();
    assert_eq!(seen.len(), 6);
    for (index, (iteration, phase, body, pass_icon)) in seen.iter().enumerate() {
        let n = index + 1;
        assert_eq!(*iteration, n);
        assert_eq!(*phase, Some(json!(n)));
        assert_eq!(body.trim(), format!("Implement Phase {n} of 6"));
        let expected_icon = if n == 6 { "✅" } else { "🧑‍💻" };
        assert_eq!(
            pass_icon.as_ref().and_then(|v| v.as_str()),
            Some(expected_icon),
            "pass_icon on iteration {n} should be {expected_icon}"
        );
    }
}

#[test]
fn seeded_loop_reports_honest_error_for_non_numeric_control_variable() {
    let source = make_source_with_body(
        &[
            ("area", json!("claudine")),
            (
                "loop",
                json!({"while": "true", "action": "increment(area)"}),
            ),
        ],
        "work",
    );
    let config = resolve_loop_config(&source).unwrap().unwrap();
    let seed = build_loop_seed(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::ChainedDocument,
    )
    .unwrap();

    let result = execute_loop_with_config(
        &source.resolved_path,
        &config,
        seed,
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("ok")),
    )
    .unwrap();

    assert_eq!(result.iteration_count, 1);
    match result.error {
        Some(CompositionError::InvalidIncrementType {
            property,
            found,
            value_excerpt,
            ..
        }) => {
            assert_eq!(property, "area");
            assert_eq!(found, "string");
            assert!(
                value_excerpt.contains("claudine"),
                "excerpt should quote the value: {value_excerpt}"
            );
        }
        other => panic!("expected InvalidIncrementType, got {other:?}"),
    }
}

#[test]
fn seeded_loop_doc_namespace_condition_retains_readonly_control_value() {
    let source = make_source_with_body(
        &[
            ("counter", json!(0)),
            ("total", json!(2)),
            (
                "loop",
                json!({"while": "doc.counter < doc.total", "action": "increment(counter)"}),
            ),
        ],
        "Step {{ counter }} of {{ total }}",
    );
    let config = resolve_loop_config(&source).unwrap().unwrap();
    let seed = build_loop_seed(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::ChainedDocument,
    )
    .unwrap();

    assert_eq!(seed.get("counter"), Some(&json!(0)));
    assert_eq!(seed.get("total"), Some(&json!(2)));

    let captured = RefCell::new(Vec::new());
    let result = execute_loop_with_config(
        &source.resolved_path,
        &config,
        seed,
        LoopExecutionOptions::default(),
        |ctx| {
            let prepared = prepare_direct(
                &source,
                PrepareOptions {
                    set_overrides: Some(ctx.as_set_overrides()),
                    ..PrepareOptions::default()
                },
            )?;
            let body = prepared.prompt.clone();
            captured.borrow_mut().push((ctx.iteration, body));
            Ok(LoopIterationOutput::success(prepared.prompt))
        },
    )
    .unwrap();

    assert!(result.error.is_none(), "expected clean run, got {result:?}");
    assert_eq!(result.iteration_count, 2);
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(2)));

    let seen = captured.into_inner();
    assert_eq!(seen.len(), 2);
    for (index, (iteration, body)) in seen.iter().enumerate() {
        let n = index + 1;
        assert_eq!(*iteration, n);
        // Iteration N uses the counter value BEFORE the increment fires
        // at the end of the iteration (counter 0→1→2), so the rendered
        // body shows the starting counter for that pass.
        assert_eq!(body.trim(), format!("Step {} of 2", n - 1));
    }
}

// ── Loop-path `initialize` lifecycle-control tests ───────────────────
//
// These exercise `execute_loop_with_lifecycle` directly (the
// `compose --loop` driver) to prove that the `initialize` event's
// returned `LifecycleEventOutcome` is honored for all four controls
// — the gap Finding 2 reported. The non-loop path
// (`wrap/composition/mod.rs`) handles the same controls; these lock the
// loop path to identical behavior so there is no mode divergence.

use std::sync::Mutex;

use crate::composition::lifecycle::{
    LifecycleConfig, LifecycleEmitter, LifecycleRuntimeContext, parse_lifecycle_config,
};

/// Test emitter that records every emitted lifecycle signal so a test can
/// assert which events fired (and, crucially, which did *not*).
#[derive(Default)]
struct SignalRecorder {
    signals: Mutex<Vec<LifecycleSignal>>,
}

impl SignalRecorder {
    fn signals(&self) -> Vec<LifecycleSignal> {
        self.signals.lock().unwrap().clone()
    }
}

impl LifecycleEmitter for SignalRecorder {
    fn emit_stderr(
        &self,
        signal: LifecycleSignal,
        _text: &str,
        _term: &biscuit_terminal::terminal::Terminal,
    ) {
        self.signals.lock().unwrap().push(signal);
    }
    fn emit_message(
        &self,
        _text: &str,
        _source_path: &Path,
        _repo_root: Option<&Path>,
        _messaging: &crate::messaging::RuntimeMessagingSettings,
    ) {
    }
    fn emit_speech(&self, _text: &str, _tts_config: biscuit_speaks::TtsConfig) {}
    fn emit_effect(&self, _name: &str) {}
    fn emit_notification(&self, _title: &str) {}
}

fn lifecycle_from(json: serde_json::Value) -> LifecycleConfig {
    parse_lifecycle_config(&json, Path::new("loop.md")).unwrap()
}

/// Drive `execute_loop_with_lifecycle` against a parsed lifecycle config,
/// counting executor (iteration) invocations. The executor always succeeds
/// so any iteration that runs is unambiguously the engine's decision, not a
/// terminal-signal artifact.
fn run_loop_lifecycle(
    prompt_path: &Path,
    config: &LoopConfig,
    initial_frontmatter: Map<String, Value>,
    lifecycle: &LifecycleConfig,
    emitter: &dyn LifecycleEmitter,
    invocations: &RefCell<usize>,
) -> LoopExecutionResult {
    let settings = crate::events::GlobalSettings::default();
    let messaging = crate::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = biscuit_terminal::terminal::Terminal::default();
    let lifecycle_ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: prompt_path,
        repo_root: prompt_path.parent(),
        launch_area: None,
        context: None,
    };
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(prompt_path.parent().unwrap_or(Path::new(".")))
        .auto_rehash(false)
        .build();
    execute_loop_with_lifecycle(
        prompt_path,
        config,
        initial_frontmatter,
        LoopExecutionOptions::default(),
        lifecycle,
        &lifecycle_ctx,
        &effect_engine,
        &crate::composition::lifecycle_executor::SystemShellRunner,
        emitter,
        |_ctx, _guard| {
            *invocations.borrow_mut() += 1;
            Ok(LoopIterationOutput::success("ran"))
        },
    )
    .unwrap()
}

/// `skip` at `initialize` in a looping document ends the run immediately:
/// zero iterations, no executor invocation, no terminal/`finalize`/`loop`
/// events — the whole-document opt-out the spec requires (spec.md:338).
#[test]
fn loop_initialize_skip_ends_run_with_zero_iterations() {
    let config = counter_loop(3);
    let lifecycle = lifecycle_from(json!({
        "initialize": { "stack": [{ "action": "skip" }] },
        "finalize": { "stack": [{ "action": {"append_line": ["never.log", "finalize"]} }] },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    assert!(result.error.is_none(), "skip is a clean opt-out: {result:?}");
    assert_eq!(result.iteration_count, 0, "no iteration runs after skip");
    assert_eq!(*invocations.borrow(), 0, "the executor must never be invoked");
    assert!(
        result.init_proxy_target.is_none(),
        "skip is not a proxy hand-off"
    );
    // Only `initialize` may have emitted (the stack control fires before any
    // terminal handling); no terminal/finalize/loop signal escaped.
    let signals = emitter.signals();
    assert!(
        !signals.contains(&LifecycleSignal::Finalize),
        "skip must not run finalize; got {signals:?}"
    );
    assert!(
        !signals.contains(&LifecycleSignal::Success)
            && !signals.contains(&LifecycleSignal::Failure),
        "skip emits no terminal signal; got {signals:?}"
    );
}

/// `error(...)` at `initialize` routes the run to `failure` then `finalize`
/// and terminates the loop with a typed `LifecycleInitializeFailed` — no
/// iteration runs (initialize fires once, before iterations). Mirrors the
/// non-loop path (spec.md:607).
#[test]
fn loop_initialize_error_routes_to_failure_and_finalize() {
    let config = counter_loop(3);
    let lifecycle = lifecycle_from(json!({
        "initialize": { "stack": [{ "action": {"error": "preflight refused"} }] },
        "failure": { "stderr": "fail" },
        "finalize": { "stderr": "final" },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    assert_eq!(*invocations.borrow(), 0, "no iteration runs after an init error");
    assert_eq!(result.iteration_count, 0);
    match &result.error {
        Some(CompositionError::LifecycleInitializeFailed { reason, .. }) => {
            assert!(
                reason.contains("preflight refused"),
                "the authored reason must survive: {reason}"
            );
        }
        other => panic!("expected LifecycleInitializeFailed, got {other:?}"),
    }
    let signals = emitter.signals();
    assert!(
        signals.contains(&LifecycleSignal::Failure),
        "init error routes to failure; got {signals:?}"
    );
    assert!(
        signals.contains(&LifecycleSignal::Finalize),
        "init error then runs finalize; got {signals:?}"
    );
}

/// `stop` at `initialize` ends only the initialize stack; the run proceeds
/// into the iteration loop unchanged (spec.md:337). Proven by parity: the
/// iteration count with a `stop` init control equals the count from an
/// otherwise-identical document whose initialize stack is benign (an
/// `info(...)` that never re-routes), so `stop` is confirmed to leave the
/// loop untouched without hard-coding the engine's iteration arithmetic.
#[test]
fn loop_initialize_stop_proceeds_into_iterations() {
    let run = |action: serde_json::Value| {
        let config = counter_loop(3);
        let lifecycle = lifecycle_from(json!({
            "initialize": { "stack": [{ "action": action }] },
        }));
        let emitter = SignalRecorder::default();
        let invocations = RefCell::new(0usize);
        let result = run_loop_lifecycle(
            Path::new("loop.md"),
            &config,
            object(json!({ "counter": 0 })),
            &lifecycle,
            &emitter,
            &invocations,
        );
        (result, invocations.into_inner())
    };

    let (stop_result, stop_invocations) = run(json!("stop"));
    let (baseline_result, baseline_invocations) = run(json!({ "info": "init ran" }));

    assert!(stop_result.error.is_none(), "stop is benign: {stop_result:?}");
    assert!(stop_invocations > 0, "the loop must run after a benign stop");
    assert_eq!(
        stop_result.iteration_count, baseline_result.iteration_count,
        "stop must not change how many iterations run"
    );
    assert_eq!(
        stop_invocations, baseline_invocations,
        "stop must not change executor invocations vs. a benign init stack"
    );
    assert_eq!(stop_result.iteration_count, stop_invocations);
}

/// `proxy(...)` at `initialize` resolves the target and hands off without
/// running any iteration, terminal, `finalize`, or `loop` event — the
/// caller re-enters with the target's own `initialize` (spec.md:340,607).
#[test]
fn loop_initialize_proxy_hands_off_without_iterating() {
    let dir = TempDir::new().unwrap();
    let prompt = dir.path().join("loop.md");
    std::fs::write(&prompt, "---\n---\nbody").unwrap();
    let target = dir.path().join("target.md");
    std::fs::write(&target, "---\n---\nbody").unwrap();

    let config = counter_loop(3);
    let lifecycle = lifecycle_from(json!({
        "initialize": { "stack": [{ "action": {"proxy": "target.md"} }] },
        "finalize": { "stderr": "final" },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        &prompt,
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    assert!(result.error.is_none(), "clean proxy hand-off: {result:?}");
    assert_eq!(result.iteration_count, 0, "no iteration runs on a proxy hand-off");
    assert_eq!(*invocations.borrow(), 0);
    assert_eq!(
        result.init_proxy_target.as_deref(),
        Some(target.as_path()),
        "the resolved target is surfaced for the caller to re-enter"
    );
    let signals = emitter.signals();
    assert!(
        !signals.contains(&LifecycleSignal::Finalize)
            && !signals.contains(&LifecycleSignal::Failure)
            && !signals.contains(&LifecycleSignal::Success),
        "a clean hand-off fires no terminal/finalize/loop events; got {signals:?}"
    );
}

/// A `proxy(...)` target that cannot be resolved (missing file) is reported
/// as an initialize failure (routed through failure + finalize), matching
/// the non-loop path's behavior rather than silently iterating.
#[test]
fn loop_initialize_proxy_unresolvable_routes_to_failure() {
    let dir = TempDir::new().unwrap();
    let prompt = dir.path().join("loop.md");
    std::fs::write(&prompt, "---\n---\nbody").unwrap();

    let config = counter_loop(3);
    let lifecycle = lifecycle_from(json!({
        "initialize": { "stack": [{ "action": {"proxy": "does-not-exist.md"} }] },
        "finalize": { "stderr": "final" },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        &prompt,
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    assert_eq!(*invocations.borrow(), 0, "no iteration runs on a failed proxy");
    assert!(result.init_proxy_target.is_none());
    assert!(
        matches!(
            result.error,
            Some(CompositionError::LifecycleInitializeFailed { .. })
        ),
        "an unresolvable proxy target is an initialize failure; got {:?}",
        result.error
    );
}

// ── Loop-gate `error(...)` tests (Finding 3) ─────────────────────────
//
// The `loop:` gate is a terminal-phase event, so only an *explicit*
// `error(...)` lifecycle action converts the loop's final outcome to
// failure and exits the loop. An *unintentional* action error there must
// leave the outcome unchanged (`routes_to_failure(Loop)` is always false).

/// An explicit `error(...)` in the `loop:` gate stack converts the loop's
/// final outcome to failure and exits — even though the `until` condition
/// would otherwise continue iterating. The error takes precedence over the
/// condition, so the gate's mutation (`increment(counter)`) is NOT applied
/// and no further iteration runs (spec.md:334-341, "convert final outcome
/// to failure and exit the loop").
#[test]
fn loop_gate_explicit_error_fails_and_exits_without_mutation() {
    // `until: counter > 5` with `counter` starting at 0 would continue
    // looping, so an exit here can only come from the gate's `error(...)`.
    let config = LoopConfig {
        condition: LoopCondition::Until("counter > 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    let lifecycle = lifecycle_from(json!({
        "loop": { "stack": [{ "action": {"error": "gate rejected final state"} }] },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    match &result.error {
        Some(CompositionError::LifecycleLoopGateFailed { reason, .. }) => {
            assert!(
                reason.contains("gate rejected final state"),
                "the authored reason must survive: {reason}"
            );
        }
        other => panic!("expected LifecycleLoopGateFailed, got {other:?}"),
    }
    assert_eq!(
        *invocations.borrow(),
        1,
        "exactly one iteration ran before the gate failed the loop"
    );
    assert_eq!(result.iteration_count, 1);
    assert_eq!(
        result.final_frontmatter.get("counter"),
        Some(&json!(0)),
        "the gate mutation must NOT be applied when the gate raises an error"
    );
}

/// An *unintentional* action error in the `loop:` gate stack (a `shell`
/// command that exits non-zero) must NOT invert the outcome: `loop` is a
/// terminal-phase event, so the gate proceeds to the condition and the loop
/// finishes successfully once the condition stops it. Contrast with the
/// explicit-`error` test above.
#[test]
fn loop_gate_unintentional_error_does_not_invert_outcome() {
    // `until: counter > 1` with `counter` starting at 0 and an
    // `increment(counter)` gate mutation: the gate evaluates its condition
    // against the pre-mutation state, so the loop runs two iterations
    // (counter 0→1→2) before the condition stops it. The `shell('false')`
    // gate action errors on every pass but, being unintentional at a
    // terminal-phase event, never aborts the loop.
    let config = LoopConfig {
        condition: LoopCondition::Until("counter > 1".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    let lifecycle = lifecycle_from(json!({
        "loop": { "stack": [{ "action": {"shell": "false"} }] },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    assert!(
        result.error.is_none(),
        "an unintentional gate action error must not fail the loop: {result:?}"
    );
    assert_eq!(
        result.final_frontmatter.get("counter"),
        Some(&json!(2)),
        "the loop ran to completion: the gate mutation applied on each \
         continuing pass despite the unintentional action error"
    );
}

/// A late-binding **evaluation** error in the `loop:` gate (a crashed
/// `when:` guard) halts the loop *before* the condition is evaluated and
/// before any gate mutation is applied — unlike an unintentional dispatch
/// failure, which is tolerated (Decision #3). The run reports the typed
/// `LifecycleEvaluationError`.
#[test]
fn loop_gate_evaluation_error_fails_before_condition_and_mutation() {
    // `until: counter > 5` with `counter` starting at 0 would loop forever,
    // so an exit here can only come from the gate's evaluation error.
    let config = LoopConfig {
        condition: LoopCondition::Until("counter > 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    // The gate item's `when:` references an undefined root, so it *raises*
    // at event time rather than evaluating cleanly to false.
    let lifecycle = lifecycle_from(json!({
        "loop": { "stack": [{ "when": "missing_root == true", "action": {"stderr": "x"} }] },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    match &result.error {
        Some(CompositionError::LifecycleEvaluationError { event, .. }) => {
            assert_eq!(event, "loop", "the failure names the loop gate event");
        }
        other => panic!("expected LifecycleEvaluationError, got {other:?}"),
    }
    assert_eq!(
        *invocations.borrow(),
        1,
        "exactly one iteration ran before the gate evaluation error halted the loop"
    );
    assert_eq!(
        result.final_frontmatter.get("counter"),
        Some(&json!(0)),
        "the gate mutation must NOT be applied when the gate raises an evaluation error"
    );
}

/// An explicit `error(...)` at `initialize` whose catch `failure.when:`
/// guard raises (undefined root) surfaces the FAILURE evaluation error —
/// not the original `LifecycleInitializeFailed`. Proves the broken path
/// that previously discarded the failure outcome now threads it through
/// `catch_evaluation_error`.
#[test]
fn loop_initialize_error_with_failure_raise_surfaces_failure_evaluation_error() {
    let config = counter_loop(3);
    // The `initialize` stack raises an explicit `error(...)`, which routes
    // to `failure`. The `failure.when:` references an undefined root, so it
    // *raises* at event time rather than evaluating cleanly to false.
    let lifecycle = lifecycle_from(json!({
        "initialize": { "stack": [{ "action": {"error": "preflight refused"} }] },
        "failure": {
            "stderr": "fail",
            "stack": [{ "when": "missing_root == true", "action": {"stderr": "never"}}]
        },
        "finalize": { "stderr": "final" },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    assert_eq!(*invocations.borrow(), 0, "no iteration runs after init error");
    match &result.error {
        Some(CompositionError::LifecycleEvaluationError { event, .. }) => {
            assert_eq!(
                event, "failure",
                "the surfaced error must name the failure event (its `when:` raised)"
            );
        }
        other => panic!(
            "expected LifecycleEvaluationError for failure, got {other:?}"
        ),
    }
    let signals = emitter.signals();
    assert!(
        signals.contains(&LifecycleSignal::Failure),
        "init error routes to failure; got {signals:?}"
    );
    assert!(
        signals.contains(&LifecycleSignal::Finalize),
        "init error then runs finalize; got {signals:?}"
    );
}

/// Drive `execute_loop_with_lifecycle` with an executor that emits the
/// `Success` terminal signal through the guard before returning, so the
/// post-finalize loop gate's `finalize` can actually fire (it is gated on a
/// recorded terminal emission). The standard `run_loop_lifecycle` helper's
/// executor never emits a terminal signal, so `finalize` at the gate would
/// be a no-op there — this helper is needed to prove the loop-gate
/// evaluation-error → `finalize` catch path.
fn run_loop_lifecycle_emitting_terminal(
    prompt_path: &Path,
    config: &LoopConfig,
    initial_frontmatter: Map<String, Value>,
    lifecycle: &LifecycleConfig,
    emitter: &dyn LifecycleEmitter,
    invocations: &RefCell<usize>,
) -> LoopExecutionResult {
    let settings = crate::events::GlobalSettings::default();
    let messaging = crate::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = biscuit_terminal::terminal::Terminal::default();
    let lifecycle_ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: prompt_path,
        repo_root: prompt_path.parent(),
        launch_area: None,
        context: None,
    };
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(prompt_path.parent().unwrap_or(Path::new(".")))
        .auto_rehash(false)
        .build();
    let shell_runner = crate::composition::lifecycle_executor::SystemShellRunner;
    let loop_start = std::time::Instant::now();
    execute_loop_with_lifecycle(
        prompt_path,
        config,
        initial_frontmatter,
        LoopExecutionOptions::default(),
        lifecycle,
        &lifecycle_ctx,
        &effect_engine,
        &shell_runner,
        emitter,
        |ctx, guard| {
            *invocations.borrow_mut() += 1;
            // Emit the terminal `Success` signal so the loop gate's
            // `finalize` is enabled (it requires a recorded terminal
            // emission). The owned `timing`/`current` outlive the borrowed
            // context within this closure body.
            let (timing, current) = capture_loop_lifecycle_globals(
                prompt_path.parent(),
                lifecycle_ctx.launch_area,
                loop_start,
            );
            let success_ctx = build_loop_stack_context(
                LifecycleSignal::Success,
                &ctx.frontmatter,
                &lifecycle_ctx,
                &effect_engine,
                &shell_runner,
                emitter,
                prompt_path.parent(),
                Some(&timing),
                Some(&current),
            );
            guard.execute_event(LifecycleSignal::Success, &success_ctx);
            Ok(LoopIterationOutput::success("ran"))
        },
    )
    .unwrap()
}

/// A late-binding **evaluation** error in the `loop:` gate is a
/// terminal-phase raise (Decision #3): it must fire `finalize` exactly once
/// carrying the loop error as the `err` global, so a `finalize.stack` can
/// react. Proven by a `finalize` stack whose `append_line` is gated on
/// `err.variant` — the line only lands if `err` reached `finalize`.
#[test]
fn loop_gate_evaluation_error_fires_finalize_with_err() {
    let dir = TempDir::new().unwrap();
    let prompt = dir.path().join("loop.md");
    std::fs::write(&prompt, "---\n---\nbody").unwrap();

    // `until: counter > 5` with `counter` at 0 would loop forever, so the
    // only exit is the gate's evaluation error.
    let config = LoopConfig {
        condition: LoopCondition::Until("counter > 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    // The gate `when:` references an undefined root, so it *raises* at event
    // time. `finalize` (top-level `stderr` fires so the recorder logs the
    // signal) writes the threaded `err` fields to a log, gated on the
    // canonical `when: "err"` truthiness guard.
    let lifecycle = lifecycle_from(json!({
        "loop": { "stack": [{ "when": "missing_root == true", "action": {"stderr": "x"} }] },
        "finalize": {
            "stderr": "done",
            "stack": [{
                "when": "err",
                "action": {"append_line": ["err.log", "{{ err.variant + '|' + err.msg }}"]}
            }]
        },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle_emitting_terminal(
        &prompt,
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    match &result.error {
        Some(CompositionError::LifecycleEvaluationError { event, .. }) => {
            assert_eq!(event, "loop", "the surfaced error names the loop gate");
        }
        other => panic!("expected LifecycleEvaluationError for loop, got {other:?}"),
    }
    let signals = emitter.signals();
    assert!(
        signals.contains(&LifecycleSignal::Finalize),
        "a loop-gate evaluation error must fire finalize; got {signals:?}"
    );
    let log = dir.path().join("err.log");
    let contents = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(
        !contents.trim().is_empty(),
        "finalize must see `err` from the loop evaluation error (err.log empty)"
    );
    assert!(
        contents.contains('|'),
        "finalize received both err.variant and err.msg: {contents:?}"
    );
}

/// When the `loop:` gate raises AND the catch `finalize` itself raises, the
/// surfaced error must name `finalize` (the latest crash) — precedence
/// finalize > loop. A raise inside `finalize` must not re-enter `finalize`.
#[test]
fn loop_gate_evaluation_error_with_finalize_raise_surfaces_finalize() {
    let dir = TempDir::new().unwrap();
    let prompt = dir.path().join("loop.md");
    std::fs::write(&prompt, "---\n---\nbody").unwrap();

    let config = LoopConfig {
        condition: LoopCondition::Until("counter > 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    // `finalize` top-level `stderr` fires (recorder logs the signal) before
    // its stack `when:` raises on an undefined root.
    let lifecycle = lifecycle_from(json!({
        "loop": { "stack": [{ "when": "missing_root == true", "action": {"stderr": "x"} }] },
        "finalize": {
            "stderr": "done",
            "stack": [{ "when": "also_missing == true", "action": {"stderr": "never"} }]
        },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle_emitting_terminal(
        &prompt,
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    match &result.error {
        Some(CompositionError::LifecycleEvaluationError { event, .. }) => {
            assert_eq!(
                event, "finalize",
                "the surfaced error must name finalize (latest crash)"
            );
        }
        other => panic!("expected LifecycleEvaluationError for finalize, got {other:?}"),
    }
    let finalize_count = emitter
        .signals()
        .iter()
        .filter(|s| **s == LifecycleSignal::Finalize)
        .count();
    assert_eq!(
        finalize_count, 1,
        "a raise inside finalize must not re-enter finalize"
    );
}

/// When both `failure.when` and `finalize.when` raise after an explicit
/// `initialize.error(...)`, the surfaced error must name `finalize` (the
/// latest lifecycle crash) — not `failure` or `initialize`. This proves
/// the precedence rule (finalize > failure > original) holds for the
/// previously-broken explicit-error catch path.
#[test]
fn loop_initialize_error_with_failure_and_finalize_raise_surfaces_finalize() {
    let config = counter_loop(3);
    let lifecycle = lifecycle_from(json!({
        "initialize": { "stack": [{ "action": {"error": "preflight refused"} }] },
        "failure": {
            "stderr": "fail",
            "stack": [{ "when": "missing_root == true", "action": {"stderr": "never"}}]
        },
        "finalize": {
            "stderr": "final",
            "stack": [{ "when": "also_missing == true", "action": {"stderr": "never"}}]
        },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    match &result.error {
        Some(CompositionError::LifecycleEvaluationError { event, .. }) => {
            assert_eq!(
                event, "finalize",
                "the surfaced error must name the finalize event (latest crash)"
            );
        }
        other => panic!(
            "expected LifecycleEvaluationError for finalize, got {other:?}"
        ),
    }
}
