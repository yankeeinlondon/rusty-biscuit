//! Loop execution orchestration.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::error::CompositionError;
use super::loop_actions::ActionStaging;
use super::loop_config::resolve_loop_config;
use super::loop_expression::{LoopAmbient, LoopExpressionLookup, evaluate_condition};
use super::types::{LoopConfig, ResolvedCompositionSource};

/// Default safety cap for prompt loops.
pub const DEFAULT_MAX_ITERATIONS: usize = 100;

/// Runtime options that can override per-document loop configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoopExecutionOptions {
    /// Runtime iteration cap override.
    pub max_iterations: Option<usize>,
    /// Runtime fail-fast override.
    pub fail_fast: Option<bool>,
}

/// Context passed to a single loop iteration executor.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopIterationContext {
    /// 1-based iteration index.
    pub iteration: usize,
    /// Frontmatter state for this iteration.
    pub frontmatter: Map<String, Value>,
    /// Ambient variables for this iteration.
    pub ambient: LoopAmbient,
}

impl LoopIterationContext {
    /// Build `set_overrides` for prompt preparation.
    ///
    /// The returned object contains the current frontmatter plus read-only
    /// ambient loop variables. Ambient variables intentionally shadow
    /// frontmatter keys for the duration of an iteration.
    pub fn as_set_overrides(&self) -> Value {
        let mut overrides = self.frontmatter.clone();
        insert_ambient_overrides(&mut overrides, &self.ambient);
        Value::Object(overrides)
    }
}

/// Result from executing one prompt iteration.
#[derive(Debug)]
pub struct LoopIterationOutput {
    /// Captured stdout or composed output for this iteration.
    pub output: String,
    /// Process-style exit code for this iteration.
    pub exit_code: i32,
    /// Optional execution error associated with the exit code.
    pub error: Option<CompositionError>,
}

impl LoopIterationOutput {
    /// Construct a successful iteration output.
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            exit_code: 0,
            error: None,
        }
    }

    /// Construct a failed iteration output with a process-style exit code.
    pub fn failure(output: impl Into<String>, exit_code: i32, error: CompositionError) -> Self {
        Self {
            output: output.into(),
            exit_code,
            error: Some(error),
        }
    }
}

/// Final result from a loop run.
#[derive(Debug)]
pub struct LoopExecutionResult {
    /// Exit code from the last executed iteration, or `0` if no iteration ran.
    pub final_exit_code: i32,
    /// Final committed frontmatter state.
    pub final_frontmatter: Map<String, Value>,
    /// Number of prompt iterations that actually ran.
    pub iteration_count: usize,
    /// Last captured iteration output.
    pub last_output: String,
    /// Optional loop, action, or iteration execution error.
    pub error: Option<CompositionError>,
}

impl LoopExecutionResult {
    fn success(
        final_frontmatter: Map<String, Value>,
        iteration_count: usize,
        last_output: String,
        final_exit_code: i32,
    ) -> Self {
        Self {
            final_exit_code,
            final_frontmatter,
            iteration_count,
            last_output,
            error: None,
        }
    }

    fn failure(
        final_frontmatter: Map<String, Value>,
        iteration_count: usize,
        last_output: String,
        final_exit_code: i32,
        error: CompositionError,
    ) -> Self {
        Self {
            final_exit_code,
            final_frontmatter,
            iteration_count,
            last_output,
            error: Some(error),
        }
    }
}

/// Execute a loop defined on a resolved composition source.
///
/// Returns `Ok(None)` when the source has no `loop` frontmatter.
///
/// ## Errors
///
/// Returns parse/evaluation errors that prevent the engine from determining
/// loop control flow. Per-iteration prompt/action failures are represented in
/// [`LoopExecutionResult::error`] according to fail-fast semantics.
pub fn execute_loop(
    source: &ResolvedCompositionSource,
    options: LoopExecutionOptions,
    executor: impl FnMut(LoopIterationContext) -> Result<LoopIterationOutput, CompositionError>,
) -> Result<Option<LoopExecutionResult>, CompositionError> {
    let Some(config) = resolve_loop_config(source)? else {
        return Ok(None);
    };
    let initial_frontmatter = source
        .markdown
        .frontmatter()
        .as_map()
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    execute_loop_with_config(
        &source.resolved_path,
        &config,
        initial_frontmatter,
        options,
        executor,
    )
    .map(Some)
}

/// Execute a loop with an already parsed configuration and initial state.
///
/// This is the core engine used by tests and by higher-level CLI integration.
///
/// ## Errors
///
/// Returns condition evaluation errors. Runtime prompt/action failures are
/// carried by the returned [`LoopExecutionResult`] so callers can report the
/// final state together with the error.
pub fn execute_loop_with_config(
    prompt_path: &Path,
    config: &LoopConfig,
    initial_frontmatter: Map<String, Value>,
    options: LoopExecutionOptions,
    mut executor: impl FnMut(LoopIterationContext) -> Result<LoopIterationOutput, CompositionError>,
) -> Result<LoopExecutionResult, CompositionError> {
    let max_iterations = options
        .max_iterations
        .or(config.max_iterations)
        .unwrap_or(DEFAULT_MAX_ITERATIONS);
    let fail_fast = options.fail_fast.or(config.fail_fast).unwrap_or(true);

    let mut frontmatter = initial_frontmatter;
    let mut iteration_count = 0usize;
    let mut last_output = String::new();
    let mut last_exit_code = 0i32;

    for iteration in 1..=max_iterations {
        let is_last = compute_is_last(
            prompt_path,
            config,
            &frontmatter,
            iteration,
            max_iterations,
            &last_output,
            last_exit_code,
        )?;
        let ambient = LoopAmbient::new(
            iteration,
            iteration == 1,
            is_last,
            last_output.clone(),
            last_exit_code,
        );
        let lookup = LoopExpressionLookup::new(&frontmatter, &ambient);
        if !evaluate_condition(&config.condition, &lookup)? {
            return Ok(LoopExecutionResult::success(
                frontmatter,
                iteration_count,
                last_output,
                last_exit_code,
            ));
        }

        let context = LoopIterationContext {
            iteration,
            frontmatter: frontmatter.clone(),
            ambient,
        };
        let output = match executor(context) {
            Ok(output) => output,
            Err(error) => {
                last_output.clear();
                last_exit_code = 1;
                iteration_count += 1;
                if fail_fast {
                    return Ok(LoopExecutionResult::failure(
                        frontmatter,
                        iteration_count,
                        last_output,
                        last_exit_code,
                        error,
                    ));
                }
                continue;
            }
        };

        last_output = output.output;
        last_exit_code = output.exit_code;
        iteration_count += 1;

        if let Some(error) = output.error {
            if fail_fast {
                return Ok(LoopExecutionResult::failure(
                    frontmatter,
                    iteration_count,
                    last_output,
                    last_exit_code,
                    error,
                ));
            }
            continue;
        }

        match apply_actions(config, &frontmatter, iteration) {
            Ok(next_frontmatter) => frontmatter = next_frontmatter,
            Err(error) => {
                if fail_fast {
                    return Ok(LoopExecutionResult::failure(
                        frontmatter,
                        iteration_count,
                        last_output,
                        last_exit_code,
                        error,
                    ));
                }
            }
        }

        if iteration == max_iterations
            && should_continue_after_cap(
                config,
                &frontmatter,
                iteration + 1,
                &last_output,
                last_exit_code,
            )?
        {
            return Ok(LoopExecutionResult::failure(
                frontmatter,
                iteration_count,
                last_output,
                last_exit_code,
                CompositionError::LoopLimitExceeded {
                    cap: max_iterations,
                    prompt_path: PathBuf::from(prompt_path),
                    iteration,
                },
            ));
        }
    }

    Ok(LoopExecutionResult::success(
        frontmatter,
        iteration_count,
        last_output,
        last_exit_code,
    ))
}

fn apply_actions(
    config: &LoopConfig,
    frontmatter: &Map<String, Value>,
    iteration: usize,
) -> Result<Map<String, Value>, CompositionError> {
    let mut stage = ActionStaging::new(frontmatter, iteration, config.actions.len());
    for (index, action) in config.actions.iter().enumerate() {
        stage.apply_action(action, index + 1)?;
    }
    Ok(stage.commit_map())
}

fn compute_is_last(
    prompt_path: &Path,
    config: &LoopConfig,
    frontmatter: &Map<String, Value>,
    iteration: usize,
    max_iterations: usize,
    last_output: &str,
    last_exit_code: i32,
) -> Result<bool, CompositionError> {
    if iteration == max_iterations {
        return Ok(true);
    }

    let Ok(next_frontmatter) = apply_actions(config, frontmatter, iteration) else {
        return Ok(false);
    };
    let next_ambient = LoopAmbient::new(
        iteration + 1,
        false,
        iteration + 1 == max_iterations,
        last_output,
        last_exit_code,
    );
    let lookup = LoopExpressionLookup::new(&next_frontmatter, &next_ambient);
    evaluate_condition(&config.condition, &lookup)
        .map(|will_continue| !will_continue)
        .map_err(|error| match error {
            CompositionError::LoopInvalid(message) => CompositionError::LoopInvalid(format!(
                "failed to compute loop is_last for {} at iteration {iteration}: {message}",
                prompt_path.display()
            )),
            other => other,
        })
}

fn should_continue_after_cap(
    config: &LoopConfig,
    frontmatter: &Map<String, Value>,
    next_iteration: usize,
    last_output: &str,
    last_exit_code: i32,
) -> Result<bool, CompositionError> {
    let ambient = LoopAmbient::new(next_iteration, false, true, last_output, last_exit_code);
    let lookup = LoopExpressionLookup::new(frontmatter, &ambient);
    evaluate_condition(&config.condition, &lookup)
}

fn insert_ambient_overrides(frontmatter: &mut Map<String, Value>, ambient: &LoopAmbient) {
    frontmatter.insert(
        "iteration".to_string(),
        Value::Number(ambient.iteration.into()),
    );
    frontmatter.insert("is_first".to_string(), Value::Bool(ambient.is_first));
    frontmatter.insert("is_last".to_string(), Value::Bool(ambient.is_last));
    frontmatter.insert(
        "last_output".to_string(),
        Value::String(ambient.last_output.clone()),
    );
    frontmatter.insert(
        "last_exit_code".to_string(),
        Value::Number(ambient.last_exit_code.into()),
    );
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::Path;

    use serde_json::json;

    use super::*;
    use crate::composition::types::{LoopAction, LoopCondition};

    fn object(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn counter_loop(max: usize) -> LoopConfig {
        LoopConfig {
            condition: LoopCondition::While(format!("counter < {max}")),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
        }
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
        assert_eq!(seen[0]["iteration"], json!(1));
        assert_eq!(seen[0]["is_first"], json!(true));
        assert_eq!(seen[0]["last_output"], json!(""));
        assert_eq!(seen[1]["counter"], json!(1));
        assert_eq!(seen[1]["iteration"], json!(2));
        assert_eq!(seen[1]["is_first"], json!(false));
        assert_eq!(seen[1]["last_output"], json!("run 1"));
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
        };
        let seen = RefCell::new(Vec::new());
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            Map::new(),
            LoopExecutionOptions {
                max_iterations: Some(2),
                fail_fast: None,
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
            condition: LoopCondition::While("iteration < 4".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: Some(false),
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
            condition: LoopCondition::While("iteration < 3".into()),
            actions: vec![
                LoopAction::Increment("counter".into()),
                LoopAction::Increment("bad".into()),
            ],
            max_iterations: None,
            fail_fast: Some(false),
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
    fn five_iteration_counter_loop() {
        let config = LoopConfig {
            condition: LoopCondition::While("counter < 5".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
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
            condition: LoopCondition::While("iteration < 4".into()),
            actions: vec![LoopAction::Append {
                prop: "log".into(),
                value: json!({"event": "tick"}),
            }],
            max_iterations: None,
            fail_fast: None,
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
        let log = result.final_frontmatter.get("log").unwrap().as_str().unwrap();
        assert_eq!(log.matches("tick").count(), 3);
    }

    #[test]
    fn last_output_and_last_exit_code_propagate() {
        let config = LoopConfig {
            condition: LoopCondition::While("iteration < 4".into()),
            actions: vec![],
            max_iterations: None,
            fail_fast: None,
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
            condition: LoopCondition::While("iteration < 4".into()),
            actions: vec![],
            max_iterations: None,
            fail_fast: Some(false),
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
                    Ok(LoopIterationOutput::failure("bad", 7, CompositionError::LoopInvalid("boom".into())))
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

}
