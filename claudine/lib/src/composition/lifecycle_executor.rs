//! Lifecycle stack execution engine.
//!
//! Given a parsed [`LifecycleConfig`] and a [`LifecycleSignal`], this module
//! runs one lifecycle event: it emits the top-level communication properties
//! first, then processes the typed `stack:` top to bottom. Each stack item's
//! `when:` clause is evaluated against the lifecycle execution context; when
//! it matches, the item's actions run in order until a lifecycle control
//! action terminates the event.
//!
//! ## Scope
//!
//! This is the **engine**. It dispatches communication, side-effect, shell,
//! and expression-function actions through their existing routes and reports
//! which lifecycle control action fired (if any) plus whether an
//! unintentional action error must route the run to `failure`. It does **not**
//! perform the runtime control flow those outcomes imply (`Skip` opt-out,
//! `Proxy` hand-off, `Retry`/`Resume`/`Defer` re-entry) — that wiring lives
//! in the composition runtime.
//!
//! ## Error propagation
//!
//! An action that errors (and is not marked `no_error: true`) stops the stack
//! and is logged. At a setup-phase event (`initialize`/`start`/`blocked`) the
//! error routes the run to `failure`; at a terminal-phase event
//! (`success`/`failure`/`finalize`/`loop`) the composition outcome is
//! unchanged. The explicit `Error` lifecycle action is distinct — it is a
//! deliberate author choice surfaced as [`StackControl::Error`] for the
//! runtime to act on.

// rustfmt doesn't support let-chains yet, so nested ifs are required
#![allow(clippy::collapsible_if)]

use std::path::Path;

use biscuit_terminal::terminal::Terminal;
use darkmatter::effects::EffectEngine;
use darkmatter::markdown::compose::expression::{
    Expr, ExpressionFinder, evaluate, is_truthy, parse, scalar_string,
};
use serde_json::{Map, Value};
use tracing::warn;

use super::lifecycle::{
    LifecycleConfig, LifecycleEmitter, LifecycleNotification, LifecycleSignal, audio_phases,
    tts_config_from_settings,
};
use super::lifecycle_actions::{
    CommunicationChannel, LifecycleAction, LifecycleActionKind, LifecycleControlAction,
    RetryBackoff, is_known_side_effect,
};
use super::lifecycle_context::{
    LifecycleCurrent, LifecycleErrorInfo, LifecycleLookup, LifecycleTiming,
};
use crate::events::GlobalSettings;
use crate::messaging::RuntimeMessagingSettings;

/// A resolved lifecycle control action — the runtime-flow effect a stack item
/// requested, with every expression argument already evaluated.
///
/// The parse-time [`LifecycleControlAction`] carries unevaluated [`Expr`]
/// arguments; this is its post-evaluation form, suitable for the composition
/// runtime to act on directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackControl {
    /// End this event's stack cleanly; outcome unchanged.
    Stop,

    /// Whole-document opt-out (valid only at `initialize`).
    Skip,

    /// Mark this event as failed, with an optional reason.
    Error {
        /// Evaluated human-readable reason, if one was authored.
        reason: Option<String>,
    },

    /// Hand off to another prompt document.
    Proxy {
        /// Evaluated target prompt reference (e.g. `@prompts/foo.md`).
        target: String,
    },

    /// Try the current prompt again.
    Retry {
        /// Additional attempts beyond the original (default `1`).
        max_attempts: u32,
        /// Backoff strategy (default [`RetryBackoff::Fixed`]).
        backoff: RetryBackoff,
        /// Evaluated delay duration (default `"0s"`).
        delay: String,
    },

    /// Resume the agent session with a follow-up message.
    Resume {
        /// Evaluated follow-up prompt.
        message: String,
        /// Additional attempts beyond the original (default `1`).
        max_attempts: u32,
    },

    /// Push this prompt onto the deferred-execution queue.
    Defer {
        /// Evaluated delay duration.
        delay: String,
        /// Evaluated optional reason.
        reason: Option<String>,
    },
}

/// The result of running one lifecycle event's communication + stack.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LifecycleEventOutcome {
    /// The lifecycle control action that terminated stack processing, if any.
    /// `None` means the stack ran to completion (or there was no stack).
    pub control: Option<StackControl>,

    /// Set when an unintentional (non-`no_error`) action error stopped the
    /// stack. Carries the error snapshot so a routed-to `failure` event can
    /// expose `err.kind`/`err.variant`/`err.msg`.
    pub action_error: Option<LifecycleErrorInfo>,
}

impl LifecycleEventOutcome {
    /// Whether the unintentional action error recorded by this outcome must
    /// route the run to `failure`.
    ///
    /// True only when an action error occurred **and** `signal` is a
    /// setup-phase event ([`LifecycleSignal::routes_action_error_to_failure`]).
    pub fn routes_to_failure(&self, signal: LifecycleSignal) -> bool {
        self.action_error.is_some() && signal.routes_action_error_to_failure()
    }
}

/// Abstraction over running an approved shell command.
///
/// Lifecycle shell actions are audited against Claudine's command whitelist
/// during pre-flight; this trait runs an already-approved command. Injectable
/// so tests can assert command dispatch without spawning real processes.
pub trait ShellRunner {
    /// Run `command`. Returns the process exit code (or an error string when
    /// the process could not be spawned at all).
    fn run(&self, command: &str) -> Result<i32, String>;
}

/// Production [`ShellRunner`] that runs commands through the system shell.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemShellRunner;

impl ShellRunner for SystemShellRunner {
    fn run(&self, command: &str) -> Result<i32, String> {
        let mut cmd = system_shell_command(command);
        let status = cmd.status().map_err(|e| e.to_string())?;
        Ok(status.code().unwrap_or(-1))
    }
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(windows)]
fn system_shell_command(command: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new("cmd");
    cmd.arg("/C").arg(command);
    cmd
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(not(windows))]
fn system_shell_command(command: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}

/// Everything the executor needs to run one lifecycle event.
///
/// Construct one per event with the active [`LifecycleSignal`], the composed
/// frontmatter, the lifecycle globals (`err`/`timing`/`current`), and the
/// side-effect / shell / emitter routes.
pub struct StackExecutionContext<'a> {
    /// The event being processed.
    pub signal: LifecycleSignal,
    /// Composed frontmatter — the base namespace for expression evaluation.
    pub frontmatter: &'a Map<String, Value>,
    /// The `err` global snapshot (only meaningful for error-carrying events).
    pub err: Option<&'a LifecycleErrorInfo>,
    /// The `timing` global snapshot.
    pub timing: Option<&'a LifecycleTiming>,
    /// The `current` global snapshot (lazy `ctx`/`env` capture).
    pub current: Option<&'a LifecycleCurrent>,
    /// Base directory for read-side expression functions and file references.
    pub base_dir: Option<&'a Path>,
    /// Darkmatter side-effect engine.
    pub effect_engine: &'a EffectEngine,
    /// Approved-shell runner.
    pub shell_runner: &'a dyn ShellRunner,
    /// Communication emitter (stderr/info/warn/message/notify/say/effect).
    pub emitter: &'a dyn LifecycleEmitter,
    /// Terminal for rendering status lines.
    pub term: &'a Terminal,
    /// Composition source path (for messenger provenance).
    pub source_path: &'a Path,
    /// Repository root, if any.
    pub repo_root: Option<&'a Path>,
    /// Messaging routes.
    pub messaging: &'a RuntimeMessagingSettings,
    /// Global settings (TTS configuration).
    pub settings: &'a GlobalSettings,
}

/// What a single action did.
enum ActionStep {
    /// Continue to the next action / item.
    Continue,
    /// A lifecycle control action fired; terminate the event's stack.
    Control(StackControl),
    /// An action errored (not suppressed by `no_error`); stop the stack.
    Errored(LifecycleErrorInfo),
}

impl StackExecutionContext<'_> {
    /// Run the event: top-level communication first, then the stack.
    pub fn execute_event(&self, config: &LifecycleConfig) -> LifecycleEventOutcome {
        if let Some(notification) = config.get(self.signal) {
            self.emit_top_level(notification);
        }
        self.execute_stack_for_signal(config)
    }

    /// Run only the stack for this event's signal, without emitting the
    /// top-level communication properties.
    ///
    /// Used by [`LifecycleRunGuard`] when it owns top-level emission and
    /// only needs the stack actions evaluated.
    pub fn execute_stack_for_signal(&self, config: &LifecycleConfig) -> LifecycleEventOutcome {
        match config.stack(self.signal) {
            Some(items) if !items.is_empty() => self.execute_stack(items),
            _ => LifecycleEventOutcome::default(),
        }
    }

    /// Emit only the top-level communication properties for this event's
    /// signal, without running the stack.
    ///
    /// The counterpart to [`Self::execute_stack_for_signal`]: a caller that has
    /// already run a stack once (e.g. to inspect its [`StackControl`] before
    /// committing to a terminal signal) uses this to fire the communication
    /// surface without re-running the stack's side effects.
    pub fn emit_top_level_for_signal(&self, config: &LifecycleConfig) {
        if let Some(notification) = config.get(self.signal) {
            self.emit_top_level(notification);
        }
    }

    /// Return a copy of this context targeting a different lifecycle signal.
    ///
    /// Used by [`LifecycleRunGuard::execute_event`](super::lifecycle::LifecycleRunGuard::execute_event)
    /// so one constructed context can service every signal in a run.
    pub fn with_signal(&self, signal: LifecycleSignal) -> StackExecutionContext<'_> {
        StackExecutionContext {
            signal,
            frontmatter: self.frontmatter,
            err: self.err,
            timing: self.timing,
            current: self.current,
            base_dir: self.base_dir,
            effect_engine: self.effect_engine,
            shell_runner: self.shell_runner,
            emitter: self.emitter,
            term: self.term,
            source_path: self.source_path,
            repo_root: self.repo_root,
            messaging: self.messaging,
            settings: self.settings,
        }
    }

    /// Return a copy of this context with the `err` global attached.
    ///
    /// Used when routing an unintentional action error into the `failure`
    /// event so `err.kind`/`err.variant`/`err.msg` are available to the
    /// failure stack.
    pub fn with_error<'a>(
        &'a self,
        err: &'a LifecycleErrorInfo,
    ) -> StackExecutionContext<'a> {
        StackExecutionContext {
            signal: self.signal,
            frontmatter: self.frontmatter,
            err: Some(err),
            timing: self.timing,
            current: self.current,
            base_dir: self.base_dir,
            effect_engine: self.effect_engine,
            shell_runner: self.shell_runner,
            emitter: self.emitter,
            term: self.term,
            source_path: self.source_path,
            repo_root: self.repo_root,
            messaging: self.messaging,
            settings: self.settings,
        }
    }

    /// Build the expression lookup for the current event.
    fn lookup(&self) -> LifecycleLookup<'_> {
        let mut lookup = LifecycleLookup::new(self.frontmatter).with_base_dir(self.base_dir);
        if let Some(err) = self.err {
            lookup = lookup.with_err(err);
        }
        if let Some(timing) = self.timing {
            lookup = lookup.with_timing(timing);
        }
        if let Some(current) = self.current {
            lookup = lookup.with_current(current);
        }
        lookup
    }

    /// Emit the top-level communication properties for one notification.
    ///
    /// Order: stdout, stderr, info, warn, success, message, notify, then the
    /// audio phases (`say`/`say_first` and `effect`, in their deterministic
    /// order). These strings were resolved during composition, so they are
    /// emitted verbatim (no further expression evaluation).
    fn emit_top_level(&self, n: &LifecycleNotification) {
        if let Some(text) = &n.stdout {
            self.emitter.emit_stdout(text, self.term);
        }
        if let Some(text) = &n.stderr {
            self.emitter.emit_stderr(self.signal, text, self.term);
        }
        if let Some(text) = &n.info {
            self.emitter.emit_info(text, self.term);
        }
        if let Some(text) = &n.warn {
            self.emitter.emit_warn(text, self.term);
        }
        if let Some(text) = &n.success {
            self.emitter.emit_success(text, self.term);
        }
        if let Some(text) = &n.message {
            self.emitter
                .emit_message(text, self.source_path, self.repo_root, self.messaging);
        }
        if let Some(title) = &n.notify {
            self.emitter.emit_notification(title);
        }
        for phase in audio_phases(n) {
            match phase {
                super::lifecycle::AudioPhase::Speak(text) => {
                    let config = tts_config_from_settings(self.settings.tts.as_ref());
                    self.emitter.emit_speech(&text, config);
                }
                super::lifecycle::AudioPhase::Effect(name) => self.emitter.emit_effect(&name),
            }
        }
    }

    /// Process the typed stack top to bottom.
    fn execute_stack(&self, items: &[super::lifecycle_actions::LifecycleStackItem]) -> LifecycleEventOutcome {
        for item in items {
            if !self.when_matches(item.when.as_ref()) {
                continue;
            }
            for action in &item.actions {
                match self.run_action(action) {
                    ActionStep::Continue => {}
                    ActionStep::Control(control) => {
                        return LifecycleEventOutcome {
                            control: Some(control),
                            action_error: None,
                        };
                    }
                    ActionStep::Errored(info) => {
                        return LifecycleEventOutcome {
                            control: None,
                            action_error: Some(info),
                        };
                    }
                }
            }
        }
        LifecycleEventOutcome::default()
    }

    /// Evaluate an optional `when:` clause. Omitted clauses always match. An
    /// evaluation failure logs a warning and treats the item as non-matching
    /// so a malformed guard cannot fire an unintended action.
    fn when_matches(&self, when: Option<&Expr>) -> bool {
        let Some(expr) = when else {
            return true;
        };
        match evaluate(expr, &self.lookup()) {
            Ok(value) => is_truthy(&value),
            Err(e) => {
                warn!(error = %e, "lifecycle `when` evaluation failed; treating as false");
                false
            }
        }
    }

    /// Run one action, applying the `no_error` escape hatch.
    fn run_action(&self, action: &LifecycleAction) -> ActionStep {
        match self.execute_action_inner(action) {
            Ok(None) => ActionStep::Continue,
            Ok(Some(control)) => ActionStep::Control(control),
            Err(info) => {
                if action.no_error {
                    warn!(
                        kind = info.kind,
                        variant = %info.variant,
                        message = %info.msg,
                        "lifecycle action errored (no_error: suppressed)"
                    );
                    ActionStep::Continue
                } else {
                    warn!(
                        kind = info.kind,
                        variant = %info.variant,
                        message = %info.msg,
                        "lifecycle action errored"
                    );
                    ActionStep::Errored(info)
                }
            }
        }
    }

    /// Execute one action's body.
    ///
    /// Returns `Ok(None)` to continue, `Ok(Some(control))` when a lifecycle
    /// control action fired, or `Err(info)` on an action error.
    fn execute_action_inner(
        &self,
        action: &LifecycleAction,
    ) -> Result<Option<StackControl>, LifecycleErrorInfo> {
        match &action.kind {
            LifecycleActionKind::LifecycleControl(control) => self
                .resolve_control(control)
                .map(Some)
                .map_err(|msg| LifecycleErrorInfo::from_action_failure(control.verb(), msg)),
            LifecycleActionKind::Communication(comm) => {
                let message = self
                    .render_message(&comm.message)
                    .map_err(|msg| LifecycleErrorInfo::from_action_failure(comm.channel.verb(), msg))?;
                self.emit_communication(comm.channel, &message);
                Ok(None)
            }
            LifecycleActionKind::Shell(shell) => {
                self.run_shell_action(shell).map(|()| None)
            }
            LifecycleActionKind::SideEffect(effect) => self
                .dispatch_side_effect(&effect.verb, &effect.args)
                .map(|_| None)
                .map_err(|msg| LifecycleErrorInfo::from_action_failure(effect.verb.clone(), msg)),
            LifecycleActionKind::ExpressionFunction(func) => {
                // A short-form `verb(args)` whose verb is actually a known
                // side effect parses as an expression-function action; route
                // it to the side-effect engine here.
                if is_known_side_effect(&func.function) {
                    return self
                        .dispatch_side_effect(&func.function, &func.args)
                        .map(|_| None)
                        .map_err(|msg| {
                            LifecycleErrorInfo::from_action_failure(func.function.clone(), msg)
                        });
                }
                self.invoke_expression_function(&func.function, &func.args)
                    .map(|_| None)
                    .map_err(|msg| {
                        LifecycleErrorInfo::from_action_failure(func.function.clone(), msg)
                    })
            }
        }
    }

    /// Evaluate a message expression to its display string, resolving any
    /// `{{ … }}` interpolation surviving inside a literal value.
    fn render_message(&self, expr: &Expr) -> Result<String, String> {
        let lookup = self.lookup();
        let value = evaluate(expr, &lookup)?;
        let rendered = scalar_string(&value);
        if rendered.contains("{{") {
            Ok(interpolate(&rendered, &lookup))
        } else {
            Ok(rendered)
        }
    }

    /// Emit a resolved communication message on the given channel.
    fn emit_communication(&self, channel: CommunicationChannel, message: &str) {
        match channel {
            CommunicationChannel::Say | CommunicationChannel::Speak => {
                let config = tts_config_from_settings(self.settings.tts.as_ref());
                self.emitter.emit_speech(message, config);
            }
            CommunicationChannel::Effect => self.emitter.emit_effect(message),
            CommunicationChannel::Message => {
                self.emitter
                    .emit_message(message, self.source_path, self.repo_root, self.messaging);
            }
            CommunicationChannel::Notify => self.emitter.emit_notification(message),
            CommunicationChannel::Stderr => {
                self.emitter.emit_stderr(self.signal, message, self.term);
            }
            CommunicationChannel::Info => self.emitter.emit_info(message, self.term),
            CommunicationChannel::Warn => self.emitter.emit_warn(message, self.term),
            CommunicationChannel::Success => self.emitter.emit_success(message, self.term),
            CommunicationChannel::Stdout => self.emitter.emit_stdout(message, self.term),
        }
    }

    /// Run a shell action. A non-zero exit is an action error (unless
    /// `no_error` suppresses it upstream); `on_error` is emitted as a warning
    /// status line before the error propagates.
    fn run_shell_action(
        &self,
        shell: &super::lifecycle_actions::ShellAction,
    ) -> Result<(), LifecycleErrorInfo> {
        let command = self
            .render_message(&shell.command)
            .map_err(|msg| LifecycleErrorInfo::from_action_failure("shell", msg))?;
        match self.shell_runner.run(&command) {
            Ok(0) => Ok(()),
            Ok(code) => {
                if let Some(on_error) = &shell.on_error {
                    if let Ok(text) = self.render_message(on_error) {
                        self.emitter.emit_warn(&text, self.term);
                    }
                }
                Err(LifecycleErrorInfo::from_action_failure(
                    "shell",
                    format!("command `{command}` exited with code {code}"),
                ))
            }
            Err(spawn_err) => Err(LifecycleErrorInfo::from_action_failure(
                "shell",
                format!("command `{command}` failed to run: {spawn_err}"),
            )),
        }
    }

    /// Dispatch a Darkmatter side effect by verb with positional, evaluated
    /// arguments.
    fn dispatch_side_effect(&self, verb: &str, args: &[Expr]) -> Result<Value, String> {
        let lookup = self.lookup();
        let values = args
            .iter()
            .map(|expr| evaluate(expr, &lookup))
            .collect::<Result<Vec<_>, _>>()?;
        let engine = self.effect_engine;
        let s = |idx: usize| -> Result<String, String> {
            values
                .get(idx)
                .map(scalar_string)
                .ok_or_else(|| format!("`{verb}` is missing a required argument"))
        };
        let v = |idx: usize| -> Result<Value, String> {
            values
                .get(idx)
                .cloned()
                .ok_or_else(|| format!("`{verb}` is missing a required argument"))
        };

        let result = match verb {
            "set_frontmatter" => engine.set_frontmatter(&s(0)?, &s(1)?, v(2)?),
            "merge_frontmatter" => engine.merge_frontmatter(&s(0)?, v(1)?),
            "delete_frontmatter" => engine.delete_frontmatter(&s(0)?, &s(1)?),
            "increment_frontmatter" => engine.increment_frontmatter(&s(0)?, &s(1)?),
            "decrement_frontmatter" => engine.decrement_frontmatter(&s(0)?, &s(1)?),
            "append_frontmatter" => engine.append_frontmatter(&s(0)?, &s(1)?, v(2)?),
            "prepend_frontmatter" => engine.prepend_frontmatter(&s(0)?, &s(1)?, v(2)?),
            "ensure_file" => {
                if values.len() >= 2 {
                    engine.ensure_file_with_content(&s(0)?, &s(1)?).map(Value::String)
                } else {
                    engine.ensure_file(&s(0)?).map(Value::String)
                }
            }
            "ensure_dir" => engine.ensure_dir(&s(0)?).map(Value::String),
            "append_line" => engine.append_line(&s(0)?, &s(1)?).map(Value::String),
            "append_jsonl" => engine.append_jsonl(&s(0)?, v(1)?).map(Value::String),
            "http_post" => engine.http_post(&s(0)?, s(1)?.into_bytes()),
            other => return Err(format!("unknown side effect `{other}`")),
        };
        result.map_err(|e| e.to_string())
    }

    /// Invoke a read-only expression function for its result, logging it in
    /// the lifecycle/status style.
    fn invoke_expression_function(&self, function: &str, args: &[Expr]) -> Result<Value, String> {
        let call = Expr::FunctionCall {
            name: function.to_string(),
            args: args.to_vec(),
        };
        let value = evaluate(&call, &self.lookup())?;
        warn!(
            function,
            result = %scalar_string(&value),
            "lifecycle expression-function action evaluated"
        );
        Ok(value)
    }

    /// Resolve a parse-time [`LifecycleControlAction`] into a runtime
    /// [`StackControl`] by evaluating its expression arguments.
    fn resolve_control(&self, control: &LifecycleControlAction) -> Result<StackControl, String> {
        use LifecycleControlAction as C;
        Ok(match control {
            C::Stop => StackControl::Stop,
            C::Skip => StackControl::Skip,
            C::Error { reason } => StackControl::Error {
                reason: self.eval_opt_string(reason.as_ref())?,
            },
            C::Proxy { target } => StackControl::Proxy {
                target: self.render_message(target)?,
            },
            C::Retry {
                max_attempts,
                backoff,
                delay,
            } => StackControl::Retry {
                max_attempts: self.eval_opt_u32(max_attempts.as_ref())?.unwrap_or(1),
                backoff: backoff.unwrap_or(RetryBackoff::Fixed),
                delay: self
                    .eval_opt_string(delay.as_ref())?
                    .unwrap_or_else(|| "0s".to_string()),
            },
            C::Resume {
                message,
                max_attempts,
            } => StackControl::Resume {
                message: self.render_message(message)?,
                max_attempts: self.eval_opt_u32(max_attempts.as_ref())?.unwrap_or(1),
            },
            C::Defer { delay, reason } => StackControl::Defer {
                delay: self.render_message(delay)?,
                reason: self.eval_opt_string(reason.as_ref())?,
            },
        })
    }

    /// Evaluate an optional expression to a display string.
    fn eval_opt_string(&self, expr: Option<&Expr>) -> Result<Option<String>, String> {
        match expr {
            Some(expr) => Ok(Some(self.render_message(expr)?)),
            None => Ok(None),
        }
    }

    /// Evaluate an optional expression to a non-negative integer count.
    fn eval_opt_u32(&self, expr: Option<&Expr>) -> Result<Option<u32>, String> {
        let Some(expr) = expr else {
            return Ok(None);
        };
        let value = evaluate(expr, &self.lookup())?;
        let n = value
            .as_f64()
            .ok_or_else(|| format!("expected a number, got {value}"))?;
        if n < 0.0 || n.fract() != 0.0 {
            return Err(format!("expected a non-negative whole number, got {n}"));
        }
        Ok(Some(n as u32))
    }
}

/// Resolve every `{{ … }}` span in `input` against `lookup`, substituting the
/// evaluated value (or empty string when the span fails to parse/evaluate).
fn interpolate(input: &str, lookup: &LifecycleLookup<'_>) -> String {
    let spans = ExpressionFinder::find_all_plain(input);
    let mut out = input.to_string();
    // Replace right-to-left so earlier byte offsets stay valid.
    for span in spans.iter().rev() {
        let replacement = parse(&span.expression)
            .ok()
            .and_then(|expr| evaluate(&expr, lookup).ok())
            .map(|value| scalar_string(&value))
            .unwrap_or_default();
        out.replace_range(span.start..span.end, &replacement);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use serde_json::json;

    use super::super::lifecycle::parse_lifecycle_config;

    /// Recording emitter + shell runner test double.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Emitted {
        Stderr(String),
        Info(String),
        Warn(String),
        Success(String),
        Stdout(String),
        Message(String),
        Notify(String),
        Speech(String),
        Effect(String),
    }

    #[derive(Default)]
    struct Recorder {
        events: Mutex<Vec<Emitted>>,
    }

    impl Recorder {
        fn events(&self) -> Vec<Emitted> {
            self.events.lock().unwrap().clone()
        }
        fn push(&self, e: Emitted) {
            self.events.lock().unwrap().push(e);
        }
    }

    impl LifecycleEmitter for Recorder {
        fn emit_stderr(&self, _signal: LifecycleSignal, text: &str, _term: &Terminal) {
            self.push(Emitted::Stderr(text.to_string()));
        }
        fn emit_message(
            &self,
            text: &str,
            _source_path: &Path,
            _repo_root: Option<&Path>,
            _messaging: &RuntimeMessagingSettings,
        ) {
            self.push(Emitted::Message(text.to_string()));
        }
        fn emit_speech(&self, text: &str, _config: biscuit_speaks::TtsConfig) {
            self.push(Emitted::Speech(text.to_string()));
        }
        fn emit_effect(&self, name: &str) {
            self.push(Emitted::Effect(name.to_string()));
        }
        fn emit_notification(&self, title: &str) {
            self.push(Emitted::Notify(title.to_string()));
        }
        fn emit_info(&self, text: &str, _term: &Terminal) {
            self.push(Emitted::Info(text.to_string()));
        }
        fn emit_warn(&self, text: &str, _term: &Terminal) {
            self.push(Emitted::Warn(text.to_string()));
        }
        fn emit_success(&self, text: &str, _term: &Terminal) {
            self.push(Emitted::Success(text.to_string()));
        }
        fn emit_stdout(&self, text: &str, _term: &Terminal) {
            self.push(Emitted::Stdout(text.to_string()));
        }
    }

    /// Shell runner that records commands and returns a programmed exit code.
    struct MockShell {
        code: i32,
        commands: Mutex<Vec<String>>,
    }

    impl MockShell {
        fn new(code: i32) -> Self {
            Self {
                code,
                commands: Mutex::new(Vec::new()),
            }
        }
        fn commands(&self) -> Vec<String> {
            self.commands.lock().unwrap().clone()
        }
    }

    impl ShellRunner for MockShell {
        fn run(&self, command: &str) -> Result<i32, String> {
            self.commands.lock().unwrap().push(command.to_string());
            Ok(self.code)
        }
    }

    fn temp_engine() -> (tempfile::TempDir, EffectEngine) {
        let dir = tempfile::tempdir().unwrap();
        let engine = EffectEngine::builder()
            .mutation_root(dir.path())
            .auto_rehash(false)
            .build();
        (dir, engine)
    }

    struct Harness {
        settings: GlobalSettings,
        messaging: RuntimeMessagingSettings,
        term: Terminal,
    }

    impl Default for Harness {
        fn default() -> Self {
            Self {
                settings: GlobalSettings::default(),
                messaging: RuntimeMessagingSettings {
                    user: None,
                    repo: None,
                },
                term: Terminal::default(),
            }
        }
    }

    /// Build a context for `signal` over `frontmatter`, wired to the given
    /// recorder, shell, and effect engine.
    #[allow(clippy::too_many_arguments)]
    fn ctx<'a>(
        signal: LifecycleSignal,
        frontmatter: &'a Map<String, Value>,
        err: Option<&'a LifecycleErrorInfo>,
        engine: &'a EffectEngine,
        shell: &'a dyn ShellRunner,
        recorder: &'a Recorder,
        harness: &'a Harness,
        source_path: &'a Path,
    ) -> StackExecutionContext<'a> {
        StackExecutionContext {
            signal,
            frontmatter,
            err,
            timing: None,
            current: None,
            base_dir: None,
            effect_engine: engine,
            shell_runner: shell,
            emitter: recorder,
            term: &harness.term,
            source_path,
            repo_root: None,
            messaging: &harness.messaging,
            settings: &harness.settings,
        }
    }

    fn map(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn top_level_communication_fires_before_stack() {
        let config = parse_lifecycle_config(
            &json!({
                "success": {
                    "stderr": "top-level first",
                    "stack": [{"action": "info('then the stack')"}]
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
                    "stack": [{"action": "success('stack success')"}]
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
    fn stdout_channel_top_level_and_stack_route_to_emit_stdout() {
        let config = parse_lifecycle_config(
            &json!({
                "success": {
                    "stdout": "top-level stdout",
                    "stack": [{"action": "stdout('stack stdout')"}]
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
    fn when_false_skips_item_when_true_runs() {
        let config = parse_lifecycle_config(
            &json!({
                "success": {
                    "stack": [
                        {"when": "flag == 'yes'", "action": "say('matched')"},
                        {"when": "flag == 'no'", "action": "say('never')"}
                    ]
                }
            }),
            Path::new("t.md"),
        )
        .unwrap();

        let fm = map(json!({"flag": "yes"}));
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
        assert_eq!(recorder.events(), vec![Emitted::Speech("matched".to_string())]);
    }

    #[test]
    fn omitted_when_always_runs() {
        let config = parse_lifecycle_config(
            &json!({"success": {"stack": [{"action": "warn('always')"}]}}),
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
        context.execute_event(&config);
        assert_eq!(recorder.events(), vec![Emitted::Warn("always".to_string())]);
    }

    #[test]
    fn array_actions_run_in_order_then_stop_at_control() {
        let config = parse_lifecycle_config(
            &json!({
                "failure": {
                    "stack": [{
                        "action": ["say('one')", "message('two')", "stop"]
                    }]
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
        assert_eq!(outcome.control, Some(StackControl::Stop));
        assert_eq!(
            recorder.events(),
            vec![
                Emitted::Speech("one".to_string()),
                Emitted::Message("two".to_string()),
            ]
        );
    }

    #[test]
    fn control_action_terminates_remaining_items() {
        let config = parse_lifecycle_config(
            &json!({
                "failure": {
                    "stack": [
                        {"action": "stop"},
                        {"action": "say('unreached')"}
                    ]
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
        assert_eq!(outcome.control, Some(StackControl::Stop));
        assert!(recorder.events().is_empty());
    }

    #[test]
    fn shell_action_runs_command() {
        let config = parse_lifecycle_config(
            &json!({"start": {"stack": [{"action": "shell('git status --short')"}]}}),
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
                        "action": "shell",
                        "command": "false",
                        "on_error": "build failed"
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
            &json!({"success": {"stack": [{"action": "shell('false')"}]}}),
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
                        {"action": "shell", "command": "false", "no_error": true},
                        {"action": "info('reached')"}
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

    #[test]
    fn errored_action_stops_remaining_actions() {
        let config = parse_lifecycle_config(
            &json!({
                "start": {
                    "stack": [{
                        "action": ["shell('false')", "info('unreached')"]
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
            &json!({"success": {"stack": [{"action": "error('manual failure')"}]}}),
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
            &json!({"failure": {"stack": [{"action": "retry(3)"}]}}),
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
        // Short form with quoted string args is the unambiguous path: each
        // arg parses as a string literal, so `prop`/`value` reach the engine
        // verbatim.
        let config = parse_lifecycle_config(
            &json!({
                "start": {
                    "stack": [{"action": "set_frontmatter('state.md', 'status', 'in-progress')"}]
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
                        "action": "http_post",
                        "url": "https://example.com/hook",
                        "body": "hello"
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
            &json!({"start": {"stack": [{"action": "ensure_file('out/log.md')"}]}}),
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
    fn err_global_visible_in_failure_stack_when() {
        let config = parse_lifecycle_config(
            &json!({
                "failure": {
                    "stack": [{
                        "when": "err.variant == 'Io'",
                        "action": "stderr('saw io error')"
                    }]
                }
            }),
            Path::new("t.md"),
        )
        .unwrap();
        let fm = map(json!({}));
        let err = LifecycleErrorInfo {
            kind: "ClaudineError",
            variant: "Io".to_string(),
            msg: "disk full".to_string(),
        };
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
            vec![Emitted::Stderr("saw io error".to_string())]
        );
    }

    #[test]
    fn message_interpolates_frontmatter_in_literal() {
        let config = parse_lifecycle_config(
            &json!({"success": {"stack": [{"action": "info", "message": "done {{ name }}"}]}}),
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

    #[test]
    fn emit_top_level_for_signal_fires_comm_without_running_stack() {
        // The stack carries a side-effect action; `emit_top_level_for_signal`
        // must emit only the top-level communication and never touch it.
        let config = parse_lifecycle_config(
            &json!({
                "success": {
                    "stderr": "top-level only",
                    "stack": [{"action": "info('must not run')"}]
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
            "only the top-level stderr fires; the stack's info() does not"
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
}
