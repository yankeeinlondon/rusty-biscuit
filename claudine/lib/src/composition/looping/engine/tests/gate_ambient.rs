//! The `loop:` gate's lifecycle concerns read the loop's ambient values.

use super::*;
use crate::composition::lifecycle::{LifecycleEmitter, LifecycleRuntimeContext, parse_lifecycle_config};
use std::sync::Mutex;

/// Records every line the gate's notification fields and stack produce,
/// tagged with the channel that carried it.
#[derive(Default)]
struct TextRecorder {
    lines: Mutex<Vec<String>>,
}

impl TextRecorder {
    fn push(&self, channel: &str, text: &str) {
        self.lines.lock().unwrap().push(format!("{channel}: {text}"));
    }
}

impl LifecycleEmitter for TextRecorder {
    fn emit_stderr(
        &self,
        signal: LifecycleSignal,
        text: &str,
        _term: &biscuit_terminal::terminal::Terminal,
    ) {
        self.push(&format!("stderr[{}]", signal.property_name()), text);
    }
    fn emit_message(
        &self,
        text: &str,
        _source_path: &Path,
        _repo_root: Option<&Path>,
        _messaging: &crate::messaging::RuntimeMessagingSettings,
    ) {
        self.push("message", text);
    }
    fn emit_info(&self, text: &str, _term: &biscuit_terminal::terminal::Terminal) {
        self.push("info", text);
    }
    fn emit_warn(&self, text: &str, _term: &biscuit_terminal::terminal::Terminal) {
        self.push("warn", text);
    }
    fn emit_speech(&self, _text: &str, _tts_config: biscuit_speaks::TtsConfig) {}
    fn emit_effect(&self, _name: &str) {}
    fn emit_notification(&self, _title: &str) {}
}

const AMBIENT: &str = "count={{_loop_count}} first={{_loop_is_first}} last={{_loop_is_last}} \
                       out={{_loop_last_output}} code={{_loop_last_exit_code}}";

/// Every `_loop_*` value resolves in the gate's `info`, `warn`, `message`,
/// `stderr`, and stack (both a `when:` guard and an action), on every pass
/// including the one that ends the loop, with the values the condition reads:
/// those of the iteration that just finished.
#[test]
fn loop_gate_concerns_read_just_finished_iteration_ambient_on_every_pass() {
    let config = LoopConfig {
        condition: LoopCondition::Until("_loop_count >= 2".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: Some(false),
        on_rate_limit: None,
    };
    let lifecycle = parse_lifecycle_config(
        &json!({
            "loop": {
                "info": format!("info {AMBIENT}"),
                "warn": format!("warn {AMBIENT}"),
                "message": format!("message {AMBIENT}"),
                "stderr": format!("stderr {AMBIENT}"),
                "stack": [
                    { "when": "_loop_count >= 1", "action": { "info": format!("stack {AMBIENT}") } },
                ],
            },
        }),
        Path::new("loop.md"),
    )
    .unwrap();
    let emitter = TextRecorder::default();
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
        None,
        None,
        |ctx, guard| {
            guard.emit_start_once();
            // Iteration 1 exits 3 so the gate's `_loop_last_exit_code` is
            // distinguishable from the seed value; `fail_fast: false` keeps
            // the loop going to the gate.
            let output = format!("out-{}", ctx.iteration);
            Ok(if ctx.iteration == 1 {
                LoopIterationOutput::failure(
                    output,
                    3,
                    CompositionError::LoopInvalid("iteration 1 failed".into()),
                )
            } else {
                LoopIterationOutput::success(output)
            })
        },
    )
    .unwrap();

    assert!(result.error.is_none(), "got: {result:?}");
    assert_eq!(result.iteration_count, 2);
    let pass = |count, first, last, code| {
        format!("count={count} first={first} last={last} out=out-{count} code={code}")
    };
    let pass_one = pass(1, true, false, 3);
    let pass_two = pass(2, false, true, 0);
    let mut lines = emitter.lines.into_inner().unwrap();
    lines.sort();
    let mut expected: Vec<String> = [&pass_one, &pass_two]
        .iter()
        .flat_map(|values| {
            [
                format!("info: info {values}"),
                format!("warn: warn {values}"),
                format!("message: message {values}"),
                format!("stderr[loop]: stderr {values}"),
                format!("info: stack {values}"),
            ]
        })
        .collect();
    expected.sort();
    assert_eq!(lines, expected);
}
