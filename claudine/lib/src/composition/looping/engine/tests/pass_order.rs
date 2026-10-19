//! The pass order every loop shares: `initialize` inside the loop, the
//! condition before the cap, and the action only before another pass.

use super::*;
use crate::composition::lifecycle::{LifecycleEmitter, parse_lifecycle_config};
use std::sync::Mutex;

#[derive(Default)]
struct InfoRecorder {
    lines: Mutex<Vec<String>>,
}

impl LifecycleEmitter for InfoRecorder {
    fn emit_stderr(&self, _signal: LifecycleSignal, _text: &str, _term: &biscuit_terminal::terminal::Terminal) {}
    fn emit_message(
        &self,
        _text: &str,
        _source_path: &Path,
        _repo_root: Option<&Path>,
        _messaging: &crate::messaging::RuntimeMessagingSettings,
    ) {
    }
    fn emit_info(&self, text: &str, _term: &biscuit_terminal::terminal::Terminal) {
        self.lines.lock().unwrap().push(text.to_string());
    }
    fn emit_warn(&self, _text: &str, _term: &biscuit_terminal::terminal::Terminal) {}
    fn emit_speech(&self, _text: &str, _tts_config: biscuit_speaks::TtsConfig) {}
    fn emit_effect(&self, _name: &str) {}
    fn emit_notification(&self, _title: &str) {}
}

const VALUES: &str = "count={{loop.count}} first={{loop.is_first}} last={{loop.is_last}} \
                      out=[{{loop.last_output}}] code={{loop.last_exit_code}}";

/// Run `config` with `lifecycle`, recording every `info` line, with passes
/// that each succeed with `pass-N`.
fn run_recorded(config: &LoopConfig, lifecycle: &LifecycleConfig, max: Option<usize>) -> (LoopExecutionResult, Vec<String>) {
    let emitter = InfoRecorder::default();
    let settings = crate::events::GlobalSettings::default();
    let messaging = crate::messaging::RuntimeMessagingSettings { user: None, repo: None };
    let term = biscuit_terminal::terminal::Terminal::default();
    let lifecycle_ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: Path::new("loop.md"),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(Path::new("."))
        .auto_rehash(false)
        .build();
    let initial = object(json!({"counter": 0}));
    let result = execute_loop_with_lifecycle(
        Path::new("loop.md"),
        config,
        initial.clone(),
        &initial,
        None,
        LoopExecutionOptions { max_iterations: max, ..LoopExecutionOptions::default() },
        lifecycle,
        &lifecycle_ctx,
        &effect_engine,
        &crate::composition::lifecycle_executor::SystemShellRunner,
        &emitter,
        crate::test_support::process_context(),
        None,
        |ctx, guard| {
            guard.emit_start_once();
            Ok(LoopIterationOutput::success(format!("pass-{}", ctx.iteration)))
        },
    )
    .unwrap();
    (result, emitter.lines.into_inner().unwrap())
}

#[test]
fn initialize_reads_the_first_pass_values_before_any_result_exists() {
    let lifecycle = parse_lifecycle_config(
        &json!({ "initialize": [{ "info": format!("init {VALUES}") }] }),
        Path::new("loop.md"),
    )
    .unwrap();
    for (max, last) in [(1, true), (3, false)] {
        let config = LoopConfig {
            condition: LoopCondition::Until("true".into()),
            actions: vec![],
            max_iterations: Some(max),
            max_template: None,
            on_rate_limit: None,
        };
        let (result, lines) = run_recorded(&config, &lifecycle, None);
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(
            lines,
            [format!("init count=1 first=true last={last} out=[] code=0")],
            "max {max}"
        );
    }
}

#[test]
fn a_reached_cap_is_exhaustion_with_no_action_after_the_last_pass() {
    let lifecycle = parse_lifecycle_config(&json!({ "loop": { "info": VALUES } }), Path::new("loop.md")).unwrap();
    let (result, lines) = run_recorded(&counter_loop(10), &lifecycle, Some(3));
    assert!(
        matches!(result.error, Some(CompositionError::LoopLimitExceeded { cap: 3, iteration: 3, .. })),
        "{:?}",
        result.error
    );
    assert_eq!(result.outcome, Some(LoopOutcome::Exhausted));
    assert_eq!(result.iteration_count, 3);
    // Passes 1 and 2 continued, so each staged one increment; pass 3 did not.
    assert_eq!(result.final_frontmatter["counter"], json!(2));
    // The gate runs after every pass, the last one included, with the values
    // of the pass that just finished.
    assert_eq!(
        lines,
        [
            "count=1 first=true last=false out=[pass-1] code=0",
            "count=2 first=false last=false out=[pass-2] code=0",
            "count=3 first=false last=true out=[pass-3] code=0",
        ]
    );
}

#[test]
fn a_condition_met_on_the_final_permitted_pass_succeeds() {
    let config = LoopConfig {
        condition: LoopCondition::Until("counter >= 2".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: Some(3),
        max_template: None,
        on_rate_limit: None,
    };
    let (result, _) = run_recorded(&config, &LifecycleConfig::default(), None);
    assert!(result.error.is_none(), "{result:?}");
    assert_eq!(result.outcome, Some(LoopOutcome::ConditionMet));
    assert_eq!(result.iteration_count, 3);
    assert_eq!(result.final_frontmatter["counter"], json!(2));
    assert_eq!(result.last_output, "pass-3");
}

#[test]
fn a_runtime_cap_override_outranks_the_authored_cap() {
    let config = LoopConfig {
        condition: LoopCondition::While("true".into()),
        actions: vec![],
        max_iterations: Some(5),
        max_template: None,
        on_rate_limit: None,
    };
    let (result, _) = run_recorded(&config, &LifecycleConfig::default(), Some(2));
    assert_eq!(result.iteration_count, 2);
    assert!(matches!(result.error, Some(CompositionError::LoopLimitExceeded { cap: 2, .. })));
}
