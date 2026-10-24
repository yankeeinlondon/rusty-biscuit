//! Lifecycle stack execution engine.
//!
//! Given a parsed [`LifecycleConfig`] and a [`LifecycleSignal`], this module
//! runs one lifecycle event: it walks the event's ordered nodes serially. A
//! `when` guard is evaluated once, when its item is reached, against the live
//! state, and only the selected body runs. The first flow-control directive
//! reached ends the whole event, however deeply it is nested.
//!
//! ## Scope
//!
//! This is the **engine**. It dispatches communication, side-effect, shell,
//! and expression-function actions through their routes and reports which
//! lifecycle control action fired (if any) plus whether an unintentional
//! action error must route the run to `failure`. It does **not** perform the
//! runtime control flow those outcomes imply (`Skip` opt-out, `Proxy`
//! hand-off, `Retry`/`Resume`/`Defer` re-entry) — that belongs to the host's
//! runtime.
//!
//! What depends on the host's model is asked through seams: [`LifecycleHost`]
//! (bindings, runtime-state writes, loops, `prep`, shell policy, cancellation,
//! failure classification), [`ShellRunner`], and the [`LifecycleServices`]
//! (communication and the diagnostic catalog).
//!
//! ## Error propagation
//!
//! Two distinct failure channels are reported, because they have different
//! halting policies:
//!
//! - A side-effect **dispatch** failure — a channel/TTS/shell/effect-engine
//!   error after the expression layer resolved cleanly — populates
//!   [`LifecycleEventOutcome::action_error`]. It honors `no_error: true` and
//!   the per-phase routing policy: at a setup-phase event
//!   (`initialize`/`start`/`blocked`) it routes the run to `failure`; at a
//!   terminal-phase event (`success`/`failure`/`finalize`/`loop`) the
//!   composition outcome is unchanged.
//! - A late-binding **evaluation** error — a `when:` guard, an interpolation,
//!   or any event-time expression that *raised* — populates
//!   [`LifecycleEventOutcome::evaluation_error`]. It is **not** suppressible by
//!   `no_error` and halts on every phase.
//!
//! The explicit `Error` lifecycle action is distinct from both — it is a
//! deliberate author choice surfaced as [`StackControl::Error`] for the
//! runtime to act on.

// rustfmt doesn't support let-chains yet, so nested ifs are required
#![allow(clippy::collapsible_if)]

use std::collections::HashMap;
use std::error::Error as StdError;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use biscuit_file::{FileReference, FileReferenceKind};
use indexmap::IndexMap;
use serde_json::{Map, Value};
use tracing::warn;

use crate::effects::{EffectEngine, effect_registration};
use crate::markdown::compose::expression::{
    BindingView, Expr, ExpressionError, ExpressionFinder, ResolutionContext, RuntimeBinding, evaluate, functions,
    is_truthy, scalar_string, unknown_function_error,
};
use crate::markdown::compose::shell_expansion::template::{
    HoleEvaluator, LookupEvaluator, fill_template,
};
use crate::markdown::compose::subtree::{SubtreeCompose, layered_session};
use crate::markdown::compose::{
    ComposeContext, ContextBuildError, CurrentAuthority, EffectiveState, EffectiveStateBuilder,
    ResolvedShellValue, ShellCommandOrigin, ShellExpansionError,
};

use super::actions::{
    CommunicationChannel, LifecycleAction, LifecycleActionKind, LifecycleControlAction, LifecycleNode,
    RetryBackoff, RuntimeSet, ShellAction, WithOverlay, WithValue, is_known_side_effect,
};
use super::cancellation;
use super::context::{LifecycleCause, LifecycleErrorInfo, LifecycleEvaluationReason};
use super::effects::{LifecycleEffects, dispatch_communication};
use super::envelope::DiagnosticCatalog;
use super::error::LifecycleExprError;
use super::services::{LifecycleServices, MissingServiceError};
use super::shell_template::{LifecycleShellMode, LifecycleShellTemplate};
use super::registry::EventRegistry;
use super::{LifecycleConfig, LifecycleSignal};

mod host;
mod outcome;
mod shell;

pub use host::{LifecycleHost, LifecycleRunFailure, PrepError, PrepRequest};
pub use outcome::{ActionLocation, LifecycleEventOutcome, StackControl};
pub use shell::{
    DisabledShellRunner, ShellRunError, ShellRunner, ShellValueRequest, compose_request,
    execute_set_shell_values,
};

/// A poisoned live-frontmatter cell means a task panicked mid-stack; there is
/// no partial document state worth recovering.
const LIVE_POISONED: &str = "live frontmatter mutex poisoned by a panicking task";

/// The per-event inputs of a [`StackExecutor`]: the event, the document state
/// it reads and writes, and the routes its actions dispatch through.
#[derive(Clone)]
pub struct StackInputs<'a> {
    /// The event being processed.
    pub signal: LifecycleSignal,
    /// Composed frontmatter — the base namespace for expression evaluation.
    pub frontmatter: &'a Map<String, Value>,
    /// Shared cross-event live document frontmatter for the current attempt.
    ///
    /// When `Some`, lifecycle frontmatter side effects (`set_frontmatter` and
    /// friends) that target the document persist into this cell so a *later*
    /// event in the same attempt reads the mutated value — the "current
    /// effective document state at the moment the event fires" contract
    /// across events. When `None`, `frontmatter` is the only base state and
    /// stack mutations are visible intra-stack only.
    pub live_frontmatter: Option<&'a Mutex<Map<String, Value>>>,
    /// The invocation's refresh authority for Darkmatter's lazy `current` root.
    ///
    /// `None` fails closed: every `current.<key>` renders `null` and records a
    /// `PartialRuntimeCapture` diagnostic rather than probing the host.
    pub current: Option<CurrentAuthority>,
    /// Base directory for read-side expression functions and file references.
    pub base_dir: Option<&'a Path>,
    /// Base directory for `ctx.*` capture only (the launch area); when `None`,
    /// falls back to `base_dir`.
    pub ctx_base_dir: Option<&'a Path>,
    /// The single early-binding context snapshot captured once at composition
    /// start. When `Some`, it serves `ctx.*`/`env.*` instead of a per-event
    /// capture (so body and lifecycle cannot diverge); when `None`, a
    /// demand-driven capture rooted at `ctx_base_dir`/`base_dir` serves them.
    pub prepared_context: Option<&'a ComposeContext>,
    /// The request's file-resolution context, used by document-authored file
    /// expressions.
    pub file_resolution_context: &'a biscuit_file::FileResolutionContext,
    /// Darkmatter side-effect engine.
    pub effect_engine: &'a EffectEngine,
    /// Approved-shell runner.
    pub shell_runner: &'a dyn ShellRunner,
    /// Composition source path.
    pub source_path: &'a Path,
    /// The host's policy for this event.
    pub host: &'a dyn LifecycleHost,
}

/// Runs one lifecycle event's stack, or one task stack, against
/// [`StackInputs`] with the host's services.
pub struct StackExecutor<'a> {
    inputs: StackInputs<'a>,
    effects: &'a dyn LifecycleEffects,
    catalog: &'a dyn DiagnosticCatalog,
}

/// What a single action did.
enum ActionStep {
    /// Continue to the next action / item.
    Continue,
    /// A lifecycle control action fired; terminate the event's stack.
    Control(StackControl),
    /// A side-effect dispatch failure (not suppressed by `no_error`); stop the
    /// stack and report it as an [`LifecycleEventOutcome::action_error`].
    Errored(LifecycleErrorInfo),
    /// An expression-layer evaluation raise; stop the stack and report it as an
    /// [`LifecycleEventOutcome::evaluation_error`]. Never suppressed by
    /// `no_error`.
    EvaluationErrored(LifecycleErrorInfo),
}

/// Which layer an action error originated in.
///
/// The stack loop routes the two to different outcome channels: an
/// [`ActionFailure::Evaluation`] (an expression/binding raise) halts on every
/// phase and ignores `no_error`; an [`ActionFailure::Dispatch`] (a side-effect
/// that failed after a clean evaluation) keeps the `no_error` / per-phase
/// policy.
enum ActionFailure {
    /// An expression/binding raise: control-argument evaluation, message/shell
    /// interpolation, or side-effect argument evaluation.
    Evaluation(LifecycleErrorInfo),
    /// A side-effect dispatch failure: shell non-zero, an effect-engine error,
    /// or an invalid resolved effect name.
    Dispatch(LifecycleErrorInfo),
}

impl<'a> StackExecutor<'a> {
    /// An executor over `inputs` whose communication and error classification
    /// come from `services`.
    ///
    /// ## Errors
    ///
    /// [`MissingServiceError`] when `services` lacks the communication or the
    /// diagnostic service; both are required, because any action can fail and
    /// any stack can communicate.
    pub fn new(inputs: StackInputs<'a>, services: LifecycleServices<'a>) -> Result<Self, MissingServiceError> {
        Ok(Self::with_services(inputs, services.effects()?, services.diagnostics()?))
    }

    /// An executor over `inputs` with services the caller already holds.
    pub fn with_services(
        inputs: StackInputs<'a>,
        effects: &'a dyn LifecycleEffects,
        catalog: &'a dyn DiagnosticCatalog,
    ) -> Self {
        Self { inputs, effects, catalog }
    }

    /// The inputs this executor runs against.
    pub fn inputs(&self) -> &StackInputs<'a> {
        &self.inputs
    }

    /// The event's authored name under the host's registry.
    fn event_name(&self) -> &str {
        super::parse::event_name(self.inputs.host.registry(), self.inputs.signal)
    }

    /// Run the event's stack.
    pub fn execute_event(&self, config: &LifecycleConfig) -> LifecycleEventOutcome {
        self.execute_stack(config.stack(self.inputs.signal))
    }

    /// Run one already-parsed action stack — a task's `setup:` or `teardown:`.
    ///
    /// Mutations reach the live cell exactly as they do for an event stack.
    /// Diagnostics name each action's authored path, which the parser rooted
    /// at the task property (`tasks[0].setup[1].shell`), so a task-stack
    /// failure names the authored task property rather than the synthetic
    /// signal the stack was parsed under.
    pub fn execute_action_stack(&self, nodes: &[LifecycleNode]) -> LifecycleEventOutcome {
        self.execute_stack(nodes)
    }

    /// Evaluate a parsed guard against the inputs' frontmatter and bindings,
    /// with the lifecycle `when` truthiness rule.
    ///
    /// The guard is read-only: it runs no stack and dispatches nothing.
    ///
    /// ## Errors
    ///
    /// The evaluation failure. A guard that raises is never read as false.
    pub fn evaluate_guard(&self, expr: &Expr) -> Result<bool, LifecycleCause> {
        self.eval_expr(expr, self.inputs.frontmatter)
            .map(|value| is_truthy(&value))
            .map_err(LifecycleCause::new)
    }

    /// Dispatch one action as a task's `side_effect:` primary and return the
    /// value the effect produced.
    ///
    /// Only a side-effect action is executable work: the standard grammar also
    /// admits communication and flow-control verbs, and those are rejected here
    /// rather than silently succeeding with no effect.
    ///
    /// `no_error: true` keeps its dispatch-only meaning — a failed effect is
    /// suppressed and yields `Value::Null`, while an expression-layer raise
    /// still surfaces.
    ///
    /// `property` is the source-rooted semantic path of a runtime `set`
    /// mapping; nested value failures extend it with their key/index suffix.
    ///
    /// ## Errors
    ///
    /// Returns the error snapshot for an unsuppressed dispatch failure, an
    /// expression-layer raise, or a non-side-effect action.
    pub fn dispatch_task_side_effect(&self, action: &LifecycleAction, property: &str) -> Result<Value, LifecycleErrorInfo> {
        let mut working: Map<String, Value> = match self.inputs.live_frontmatter {
            Some(cell) => cell.lock().expect(LIVE_POISONED).clone(),
            None => self.inputs.frontmatter.clone(),
        };
        let result = self.dispatch_task_side_effect_inner(action, property, &mut working);
        if let Some(cell) = self.inputs.live_frontmatter {
            *cell.lock().expect(LIVE_POISONED) = working;
        }
        result
    }

    /// The dispatch half of [`Self::dispatch_task_side_effect`], against a
    /// caller-owned working map.
    fn dispatch_task_side_effect_inner(
        &self,
        action: &LifecycleAction,
        property: &str,
        working: &mut Map<String, Value>,
    ) -> Result<Value, LifecycleErrorInfo> {
        let dispatched = match &action.kind {
            LifecycleActionKind::RuntimeSet(set) => self
                .dispatch_runtime_set(set, property, working)
                .map(|prior| Value::Object(prior.into_iter().collect())),
            LifecycleActionKind::SideEffect(effect) => self.dispatch_side_effect(&effect.verb, &effect.args, working),
            LifecycleActionKind::ExpressionFunction(func) if is_known_side_effect(&func.function) => {
                self.dispatch_side_effect(&func.function, &func.args, working)
            }
            _ => {
                return Err(self.action_failure("side_effect", "value is not a side-effect action"));
            }
        };
        match dispatched {
            Ok(value) => Ok(value),
            Err(ActionFailure::Dispatch(info)) if action.no_error => {
                warn!(
                    kind = info.kind,
                    variant = %info.variant,
                    message = %info.msg,
                    "task side effect errored (no_error: suppressed)"
                );
                Ok(Value::Null)
            }
            Err(failure) => Err(match failure {
                ActionFailure::Evaluation(info) | ActionFailure::Dispatch(info) => info,
            }),
        }
    }

    /// Return a copy of this executor that reuses `prepared` as its single
    /// early-binding snapshot.
    ///
    /// Lets a caller that evaluates many leaves against one event (a whole
    /// `proxy.with` overlay) capture the snapshot-less fallback context once
    /// and reuse it, instead of re-capturing per leaf.
    fn with_prepared_context<'b>(&'b self, prepared: &'b ComposeContext) -> StackExecutor<'b> {
        StackExecutor {
            inputs: StackInputs {
                prepared_context: Some(prepared),
                ..self.inputs.clone()
            },
            effects: self.effects,
            catalog: self.catalog,
        }
    }

    /// [`LifecycleErrorInfo::from_expr_error_with`] in this executor's catalog.
    fn expr_error(&self, verb: impl Into<String>, error: LifecycleExprError) -> LifecycleErrorInfo {
        LifecycleErrorInfo::from_expr_error_with(self.catalog, verb, error)
    }

    /// [`LifecycleErrorInfo::from_error_or_action_with`] in this executor's
    /// catalog.
    fn error_or_action(&self, verb: impl Into<String>, error: &(dyn StdError + 'static)) -> LifecycleErrorInfo {
        // The locked-shell text names the event where shells unlock, which
        // only the host can spell.
        if matches!(error.downcast_ref::<ShellRunError>(), Some(ShellRunError::BeforePreflight))
            && self.catalog.select(error).is_none()
        {
            let unlock_event = self.catalog.event_name(LifecycleSignal::Start);
            return self.action_failure(verb, ShellRunError::before_preflight_message(unlock_event));
        }
        LifecycleErrorInfo::from_error_or_action_with(self.catalog, verb, error)
    }

    /// [`LifecycleErrorInfo::from_action_failure_with`] in this executor's
    /// catalog.
    fn action_failure(&self, verb: impl Into<String>, msg: impl Into<String>) -> LifecycleErrorInfo {
        LifecycleErrorInfo::from_action_failure_with(self.catalog, verb, msg)
    }

    /// Build resolution state for expressions authored by `source_path`.
    ///
    /// Implicit references resolve source-relative, then against the repository. The
    /// launch-area `ctx_base_dir` is retained as `file_ref_fallback_dir`
    /// diagnostic metadata; it is not a third resolution candidate.
    ///
    /// ## Errors
    ///
    /// The [`ContextBuildError`] when the file-resolution context fails
    /// validation.
    pub fn resolution_context(&self) -> Result<ResolutionContext, ContextBuildError> {
        self.inputs.host.resolution_context(
            self.inputs.source_path,
            self.inputs.prepared_context,
            self.inputs.file_resolution_context,
            self.inputs.ctx_base_dir,
        )
    }

    /// The early-binding `ctx.*`/`env.*` snapshot for this event.
    ///
    /// Reuses the single `prepared_context` captured once at composition start
    /// when present — so plain `ctx.*`/`env.*` in a lifecycle string is the
    /// exact snapshot the body composed against and cannot diverge. Falls back
    /// to a demand-driven capture (rooted at the launch area / prompt dir,
    /// scanned against `scan_hint`) only for callers that supply no snapshot.
    fn early_binding_context(&self, scan_hint: &str) -> ComposeContext {
        match self.inputs.prepared_context {
            Some(prepared) => prepared.clone(),
            None => {
                let base = self
                    .inputs
                    .ctx_base_dir
                    .or(self.inputs.base_dir)
                    .unwrap_or_else(|| Path::new("."));
                ComposeContext::capture_for_content(base, scan_hint)
            }
        }
    }

    /// Build the effective state over the current document `fm`.
    ///
    /// `ctx.*`/`env.*` come from [`Self::early_binding_context`]; the event's
    /// globals come separately from the host's bindings; `current.*` is the
    /// reserved root this state's refresh authority serves.
    fn build_state(&self, fm: &Map<String, Value>, scan_hint: &str) -> EffectiveState {
        let frontmatter: HashMap<String, Value> = fm.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let context = self.early_binding_context(scan_hint);
        EffectiveStateBuilder::new()
            .with_frontmatter(frontmatter)
            .with_name_coercion_keys(self.inputs.host.name_coercion_keys())
            .with_context(context)
            // Every event builds its own state, so every event starts fresh
            // evaluation scopes: a `current.<key>` read in a later event
            // observes the fact as it stands then, not as `ctx` froze it.
            .with_current_authority(self.inputs.current.clone().unwrap_or_default())
            // A deferred lifecycle subtree never defines `ctx`; downgrade any
            // pathological `ctx` shape to a warning rather than aborting.
            .with_allow_ctx_override(true)
            .build()
            .unwrap_or_else(|_| {
                EffectiveState::new(
                    &fm.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                    None,
                    self.early_binding_context(""),
                )
            })
    }

    /// Evaluate a parsed expression at event time against the live document
    /// state `fm` plus the host's globals, through Darkmatter's layered lookup.
    ///
    /// ## Errors
    ///
    /// The evaluation failure, or a resolution context that failed to build.
    pub fn eval_expr(&self, expr: &Expr, fm: &Map<String, Value>) -> Result<Value, LifecycleExprError> {
        let state = self.build_state(fm, &expr.to_string());
        let (view, globals) = self.inputs.host.bindings();
        evaluate_expr_in(expr, &state, view, globals, Some(self.resolution_context()?))
    }

    /// Interpolate a string's `{{ … }}` spans at event time through subtree
    /// compose, preserving whole-value typing.
    ///
    /// Subtree compose fails closed: a malformed span, an unknown function, or
    /// a read of a global this scope declares unavailable returns an error
    /// instead of degrading to empty. An absent document property renders as
    /// `null`/empty.
    ///
    /// `s` is authored text and is scanned exactly once. Whatever the spans
    /// insert is data: a frontmatter value or file content that itself holds
    /// `{{ … }}` or `$( … )` is delivered verbatim, never expanded again.
    ///
    /// ## Errors
    ///
    /// The compose failure, or a resolution context that failed to build.
    pub fn resolve_string_value(&self, s: &str, fm: &Map<String, Value>) -> Result<Value, LifecycleExprError> {
        let state = self.build_state(fm, s);
        let (view, globals) = self.inputs.host.bindings();
        interpolate_text_in(s, &state, view, globals, Some(self.resolution_context()?))
    }

    /// Evaluate one action operand at event time.
    ///
    /// An authored literal body with `{{ … }}` spans is a template: its spans
    /// are interpolated once. Every other expression — a whole-value span such
    /// as `{{ frontmatter('log.md', 'note') }}` — yields data, returned as is:
    /// text it produced is never scanned again, so an agent-written `{{ … }}`
    /// or `$( … )` arrives exactly as written.
    fn evaluate_operand(&self, expr: &Expr, fm: &Map<String, Value>) -> Result<Value, LifecycleExprError> {
        match expr {
            Expr::StringLiteral(text) if is_template_operand(expr) => self.resolve_string_value(text, fm),
            _ => self.eval_expr(expr, fm),
        }
    }

    /// Validate a resolved sound-effect name against the host's catalog
    /// immediately before dispatch.
    ///
    /// An `effect: "{{name}}"` action cannot be validated at prepare time
    /// because the name is interpolation-dependent; this checks the *resolved*
    /// name so an invalid name fails closed rather than playing nothing.
    fn validate_effect_name(&self, name: &str) -> Result<(), LifecycleErrorInfo> {
        if self.inputs.host.is_known_effect(name) {
            return Ok(());
        }
        Err(self.inputs.host.failure(LifecycleRunFailure::UnknownEffect {
            event: self.event_name().to_string(),
            name: name.to_string(),
        }))
    }

    /// Process the typed stack top to bottom.
    ///
    /// Holds an evolving in-memory `working` frontmatter, seeded from the shared
    /// cross-event live cell when present (so this event sees frontmatter
    /// mutations made by *earlier* events in the same attempt) and otherwise
    /// from the composed document state. A frontmatter-mutating side effect that
    /// targets the document mirrors its change onto `working`, so a later action
    /// in the same stack resolves `{{ … }}` against the mutated value
    /// (just-in-time resolution — not a single snapshot taken when the event
    /// fired). On every return path the final `working` is written back to the
    /// live cell so a *later* event observes this event's mutations — including
    /// mutations made before an early control/error return (the side effect
    /// already hit disk).
    fn execute_stack(&self, nodes: &[LifecycleNode]) -> LifecycleEventOutcome {
        if nodes.is_empty() {
            return LifecycleEventOutcome::default();
        }
        let mut working: Map<String, Value> = match self.inputs.live_frontmatter {
            Some(cell) => cell.lock().expect(LIVE_POISONED).clone(),
            None => self.inputs.frontmatter.clone(),
        };
        let outcome = self.execute_nodes(nodes, &mut working).unwrap_or_default();
        if let Some(cell) = self.inputs.live_frontmatter {
            *cell.lock().expect(LIVE_POISONED) = working;
        }
        outcome
    }

    /// Run one ordered node list against a caller-owned `working` map.
    ///
    /// Returns `Some` when the event must end — a directive fired, or an
    /// action or guard failed — so every enclosing list stops at once: a
    /// directive inside a nested `else` ends the whole event, not just its
    /// branch. `None` means the list ran to completion. Every return path
    /// leaves `working` holding the mutations made up to that point, which
    /// the caller persists.
    ///
    /// A guard is evaluated once, when its item is reached, against the live
    /// state; only the selected body runs. A guard that raises is an
    /// evaluation error and never selects `else`.
    fn execute_nodes(&self, nodes: &[LifecycleNode], working: &mut Map<String, Value>) -> Option<LifecycleEventOutcome> {
        for node in nodes {
            if self.inputs.host.stops_on_interrupt() && self.inputs.host.interrupted() {
                return Some(LifecycleEventOutcome::default());
            }
            let ended = match node {
                LifecycleNode::Conditional(conditional) => match self.when_matches(Some(&conditional.when), working) {
                    Ok(true) => self.execute_nodes(&conditional.then, working),
                    Ok(false) => self.execute_nodes(&conditional.otherwise, working),
                    Err(info) => Some(LifecycleEventOutcome {
                        evaluation_error: Some(info.at_property(conditional.path.field("when").to_string())),
                        ..Default::default()
                    }),
                },
                LifecycleNode::Action(action) => {
                    let location = ActionLocation::new(self.inputs.signal, action.path.clone());
                    let property = action.path.to_string();
                    match self.run_action(action, location, &property, working) {
                        ActionStep::Continue => None,
                        ActionStep::Control(control) => Some(LifecycleEventOutcome {
                            control: Some(control),
                            ..Default::default()
                        }),
                        ActionStep::Errored(info) => Some(LifecycleEventOutcome {
                            action_error: Some(info),
                            ..Default::default()
                        }),
                        ActionStep::EvaluationErrored(info) => Some(LifecycleEventOutcome {
                            evaluation_error: Some(info.at_property(property)),
                            ..Default::default()
                        }),
                    }
                }
            };
            if ended.is_some() {
                return ended;
            }
        }
        None
    }

    /// Evaluate an optional `when:` clause, failing closed on an unresolvable
    /// guard. Omitted clauses always match (`Ok(true)`).
    ///
    /// A `when:` guard reads live document state, evaluated just in time
    /// against the live `fm`. An absent property is `null`, so a guard over it
    /// is falsy and skips the item. A malformed expression, an unknown
    /// function, or a read of a global this scope declares unavailable returns
    /// `Err`, so the event fails closed before any side effect dispatches.
    fn when_matches(&self, when: Option<&Expr>, fm: &Map<String, Value>) -> Result<bool, LifecycleErrorInfo> {
        let Some(expr) = when else {
            return Ok(true);
        };
        self.eval_expr(expr, fm)
            .map(|value| is_truthy(&value))
            .map_err(|error| self.expr_error("when", error))
    }

    /// Run one action, applying the `no_error` escape hatch.
    ///
    /// `no_error` is scoped to side-effect **dispatch** failures only: an
    /// expression-layer **evaluation** raise always halts the stack and
    /// surfaces, because a thrown guard/interpolation is a defect the author
    /// cannot meaningfully "tolerate" the way a flaky channel send can be.
    fn run_action(
        &self,
        action: &LifecycleAction,
        location: ActionLocation,
        property: &str,
        working: &mut Map<String, Value>,
    ) -> ActionStep {
        match self.execute_action_inner(action, location, property, working) {
            Ok(None) => ActionStep::Continue,
            Ok(Some(control)) => ActionStep::Control(control),
            Err(ActionFailure::Evaluation(info)) => {
                warn!(
                    kind = info.kind,
                    variant = %info.variant,
                    message = %info.msg,
                    "lifecycle action evaluation error"
                );
                ActionStep::EvaluationErrored(info)
            }
            Err(ActionFailure::Dispatch(info)) => {
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

    /// Execute one action's body against the evolving `working` frontmatter.
    ///
    /// Returns `Ok(None)` to continue, `Ok(Some(control))` when a lifecycle
    /// control action fired, or `Err(ActionFailure)` on an action error — tagged
    /// [`ActionFailure::Evaluation`] for an expression-layer raise (which always
    /// halts) and [`ActionFailure::Dispatch`] for a side-effect failure (subject
    /// to `no_error` and the per-phase policy).
    ///
    /// `location` is the proxy-provenance identity; `property` is the authored
    /// path the same action reports in diagnostics. The two differ for a task
    /// `setup:`/`teardown:` stack, whose signal is synthetic.
    fn execute_action_inner(
        &self,
        action: &LifecycleAction,
        location: ActionLocation,
        property: &str,
        working: &mut Map<String, Value>,
    ) -> Result<Option<StackControl>, ActionFailure> {
        match &action.kind {
            // `prep` returns: the event continues with the next item.
            LifecycleActionKind::LifecycleControl(LifecycleControlAction::Prep { target, with }) => {
                self.run_prep(target, with, &location, working).map(|()| None)
            }
            LifecycleActionKind::LifecycleControl(control) => self
                .resolve_control(control, location, working)
                .map(Some)
                .map_err(ActionFailure::Evaluation),
            LifecycleActionKind::Communication(comm) => {
                let message = self
                    .render_message(&comm.text, working)
                    .map_err(|error| ActionFailure::Evaluation(self.expr_error(comm.channel.verb(), error)))?;
                // An `effect` positional action's name is only known after
                // interpolation, so validate the resolved name before
                // dispatch. The interpolation already succeeded, so an invalid
                // catalog name is a dispatch failure.
                if comm.channel == CommunicationChannel::Effect {
                    self.validate_effect_name(&message).map_err(ActionFailure::Dispatch)?;
                }
                self.emit_communication(comm.channel, &message);
                Ok(None)
            }
            LifecycleActionKind::Shell(shell) => self.run_shell_action(shell, property, working).map(|()| None),
            LifecycleActionKind::RuntimeSet(set) => {
                // `property` is the `set` verb's own authored path.
                self.dispatch_runtime_set(set, property, working).map(|_| None)
            }
            LifecycleActionKind::SideEffect(effect) => self
                .dispatch_side_effect(&effect.verb, &effect.args, working)
                .map(|_| None),
            LifecycleActionKind::ExpressionFunction(func) => {
                // A positional action whose verb is a known side effect was
                // parsed as an expression-function action; route it to the
                // side-effect engine here.
                if is_known_side_effect(&func.function) {
                    return self
                        .dispatch_side_effect(&func.function, &func.args, working)
                        .map(|_| None);
                }
                self.invoke_expression_function(&func.function, &func.args, working)
                    .map(|_| None)
                    .map_err(|error| ActionFailure::Evaluation(self.expr_error(func.function.clone(), error)))
            }
            // Only a profile that ignores verbs produces one. It runs nothing,
            // and each occurrence reached says so once.
            LifecycleActionKind::Ignored(ignored) => {
                self.effects.warn(&format!(
                    "`{}` does not run `{}`; skipped it in `{}`",
                    self.inputs.host.registry().host(),
                    ignored.verb,
                    self.event_name()
                ));
                Ok(None)
            }
        }
    }

    /// Evaluate a message expression to its display string at event time.
    ///
    /// A literal-default action body is an [`Expr::StringLiteral`] whose
    /// `{{ … }}` spans interpolate once against the current document `fm` plus
    /// the host's globals; a whole-value `{{ … }}` span resolves to a typed
    /// expression whose result is data ([`Self::evaluate_operand`]).
    ///
    /// An absent document property renders through ordinary `null` semantics.
    /// A genuine evaluation error fails before dispatch.
    fn render_message(&self, expr: &Expr, fm: &Map<String, Value>) -> Result<String, LifecycleExprError> {
        self.evaluate_operand(expr, fm).map(|value| scalar_string(&value))
    }

    /// Emit a resolved communication message on the given channel.
    fn emit_communication(&self, channel: CommunicationChannel, message: &str) {
        dispatch_communication(self.effects, channel, self.inputs.signal, message);
    }

    /// Run a shell action. A failed command-string interpolation is an
    /// evaluation error; a non-zero exit or spawn failure is a dispatch error
    /// (subject to `no_error` upstream). Initialization and preflight prohibitions
    /// are unsuppressible. `on_error` is emitted as a warning status line before
    /// a dispatch error propagates.
    ///
    /// A site approved by its shape runs its template
    /// ([`Self::run_shell_template`]); one approved in neither mode is refused.
    fn run_shell_action(&self, shell: &ShellAction, property: &str, fm: &Map<String, Value>) -> Result<(), ActionFailure> {
        if self.inputs.signal == LifecycleSignal::Initialize {
            return Err(ActionFailure::Evaluation(
                self.error_or_action("shell", &ShellRunError::BeforePreflight),
            ));
        }
        if let Some(site) = &shell.template {
            match site.mode() {
                Some(LifecycleShellMode::ResolvedBytes) => {}
                Some(LifecycleShellMode::Template { policy_root }) => {
                    return self.run_shell_template(shell, site, policy_root.as_deref(), property, fm);
                }
                None => {
                    self.ensure_shell_permitted("shell")?;
                    return Err(self.not_approved("shell", property, &site.template().authored));
                }
            }
        }
        let command = self
            .render_shell_text(shell, &shell.command, fm)
            .map_err(|error| ActionFailure::Evaluation(self.expr_error("shell", error)))?;
        match self.inputs.shell_runner.run(&command) {
            Err(ShellRunError::BeforePreflight) => Err(ActionFailure::Evaluation(
                self.error_or_action("shell", &ShellRunError::BeforePreflight),
            )),
            Ok(0) => Ok(()),
            Ok(code) => {
                if let Some(on_error) = &shell.on_error {
                    if let Ok(text) = self.render_shell_text(shell, on_error, fm) {
                        self.effects.warn(&text);
                    }
                }
                Err(ActionFailure::Dispatch(
                    self.action_failure("shell", format!("command `{command}` exited with code {code}")),
                ))
            }
            // `ShellRunError` owns the "command `…` failed to run: …" prose, so
            // the snapshot projects it rather than the executor rebuilding it
            // around an untyped runner error.
            Err(spawn_err) => Err(ActionFailure::Dispatch(self.error_or_action("shell", &spawn_err))),
        }
    }

    /// Run a shell action approved by its shape.
    ///
    /// Every placeholder is read from the state current now, checked against
    /// its declared type, and filled into exactly one argument; the whole
    /// filled command then meets the built-in and user blacklists, and only
    /// then does any segment start. A value that is missing or rejected, a
    /// blacklisted fill, and a program that needs an interpreter are
    /// evaluation errors: `no_error` cannot suppress them, and their message
    /// never shows a value. A non-zero exit is a dispatch error, as for bytes.
    fn run_shell_template(
        &self,
        shell: &ShellAction,
        site: &LifecycleShellTemplate,
        policy_root: Option<&Path>,
        property: &str,
        fm: &Map<String, Value>,
    ) -> Result<(), ActionFailure> {
        self.ensure_shell_permitted("shell")?;
        let template = site.template();
        let authored = template.authored.as_str();
        let chain = self
            .with_hole_evaluator(fm, authored, |holes| fill_template(template, holes, site.late_values()))
            .map_err(|error| ActionFailure::Evaluation(self.expr_error("shell", error)))?
            .map_err(|error| {
                let ctx = self.template_context(authored);
                self.template_refusal(property, authored, error.into_shell_error(&ctx, authored, &template_origin(property)))
            })?;
        let blacklist = self.inputs.host.user_blacklist(policy_root);
        if let Some(reason) = crate::markdown::compose::shell_expansion::check_filled_chain(&chain, &blacklist) {
            let error = ShellExpansionError::Blacklisted {
                ctx: Box::new(self.template_context(authored)),
                command: authored.to_string(),
                reason,
                origin: template_origin(property),
            };
            return Err(self.template_refusal(property, authored, error));
        }
        match self.inputs.shell_runner.run_structured(&chain) {
            Ok(0) => Ok(()),
            Ok(code) => {
                if let Some(on_error) = &shell.on_error {
                    if let Ok(text) = self.render_shell_text(shell, on_error, fm) {
                        self.effects.warn(&text);
                    }
                }
                // The authored text: the filled arguments may hold a secret.
                Err(ActionFailure::Dispatch(
                    self.action_failure("shell", format!("command `{authored}` exited with code {code}")),
                ))
            }
            Err(ShellRunError::BeforePreflight) => Err(ActionFailure::Evaluation(
                self.error_or_action("shell", &ShellRunError::BeforePreflight),
            )),
            Err(ShellRunError::Target { reason, .. }) => {
                let error = ShellExpansionError::TemplateTarget {
                    ctx: Box::new(self.template_context(authored)),
                    command: authored.to_string(),
                    reason,
                    origin: template_origin(property),
                };
                Err(self.template_refusal(property, authored, error))
            }
            Err(spawn_err) => Err(ActionFailure::Dispatch(self.error_or_action("shell", &spawn_err))),
        }
    }

    /// The runner's prohibition, if any, as the unsuppressible error a shell
    /// prohibition always is.
    fn ensure_shell_permitted(&self, verb: &str) -> Result<(), ActionFailure> {
        self.inputs
            .shell_runner
            .ensure_permitted()
            .map_err(|error| ActionFailure::Evaluation(self.error_or_action(verb.to_string(), &error)))
    }

    /// A site with a placeholder that pre-flight approved in neither mode.
    fn not_approved(&self, verb: &str, property: &str, authored: &str) -> ActionFailure {
        ActionFailure::Evaluation(
            self.action_failure(
                verb.to_string(),
                format!(
                    "`{property}` runs `{authored}`, which pre-flight never approved as its bytes or \
                     its shape, so it cannot run"
                ),
            )
            .at_property(property.to_string()),
        )
    }

    /// A template-mode failure that refused the command before it ran:
    /// unsuppressible, with Darkmatter's typed cause kept for `err.*`.
    fn template_refusal(&self, property: &str, authored: &str, error: ShellExpansionError) -> ActionFailure {
        let message = error.to_string();
        let info = self.inputs.host.failure(LifecycleRunFailure::ShellTemplateRefused {
            source_path: self.inputs.source_path.to_path_buf(),
            property: property.to_string(),
            command: authored.to_string(),
            message,
            error,
        });
        ActionFailure::Evaluation(info.at_property(property.to_string()))
    }

    /// The authored command as the excerpt a template diagnostic shows.
    fn template_context(&self, authored: &str) -> biscuit_terminal::errors::SourceContext {
        biscuit_terminal::errors::SourceContext::new(
            self.inputs.source_path.to_path_buf(),
            self.inputs.source_path.to_path_buf(),
            authored.to_string(),
        )
    }

    /// Run `f` with an evaluator that reads a placeholder against `fm` and
    /// this event's globals, exactly as every other action expression reads.
    fn with_hole_evaluator<R>(
        &self,
        fm: &Map<String, Value>,
        scan_hint: &str,
        f: impl FnOnce(&dyn HoleEvaluator) -> R,
    ) -> Result<R, LifecycleExprError> {
        let state = self.build_state(fm, scan_hint);
        let (view, globals) = self.inputs.host.bindings();
        let session = layered_session(&state, globals, Some(view), Some(self.resolution_context()?))
            .map_err(|error| LifecycleExprError::Evaluate(Box::new(ExpressionError::Binding(Box::new(error)))))?;
        Ok(f(&LookupEvaluator::new(&session)))
    }

    /// The text of a shell action's `command` or `on_error`.
    ///
    /// A pre-flight-resolved literal is the approved byte string and runs as it
    /// stands; everything else renders like a message body.
    fn render_shell_text(&self, shell: &ShellAction, expr: &Expr, fm: &Map<String, Value>) -> Result<String, LifecycleExprError> {
        match expr {
            Expr::StringLiteral(text) if shell.pre_resolved => Ok(text.clone()),
            _ => self.render_message(expr, fm),
        }
    }

    /// Dispatch a Darkmatter side effect by verb with positional, evaluated
    /// arguments.
    ///
    /// Arguments follow the communication-body rule ([`Self::evaluate_operand`]):
    /// an authored literal interpolates once, and a whole-value span keeps its
    /// typed result, which is data and is never interpolated again. A frontmatter-mutating verb
    /// that targets the document mirrors its change onto `working` so a later
    /// action in the same stack reads the mutated value. Lifecycle `set`
    /// mappings use [`Self::dispatch_runtime_set`] instead of this positional
    /// Darkmatter-effect path.
    ///
    /// The evaluated arguments bind to the verb's typed overloads
    /// ([`effect_registration`]) before the engine is called, by the same rule
    /// as an expression-function call. A call that does not bind is an
    /// evaluation failure carrying [`ExpressionError::Call`], exactly as a
    /// failed expression-function binding is, and nothing is written. The
    /// engine receives the bound values (a number given for a `string`
    /// parameter arrives as its text), and the bound overload, not the
    /// argument count, picks the `ensure_file` form.
    fn dispatch_side_effect(&self, verb: &str, args: &[Expr], working: &mut Map<String, Value>) -> Result<Value, ActionFailure> {
        // Argument evaluation/interpolation and binding are the expression
        // layer: a raise here is an evaluation error.
        let evaluation_err = |error: LifecycleExprError| ActionFailure::Evaluation(self.expr_error(verb, error));
        let evaluated = args
            .iter()
            .map(|expr| self.evaluate_operand(expr, working))
            .collect::<Result<Vec<_>, LifecycleExprError>>()
            .map_err(evaluation_err)?;
        let (values, takes_content) = match effect_registration(verb) {
            Some(registration) => {
                let call = registration.bind_call(&evaluated).map_err(|error| {
                    evaluation_err(LifecycleExprError::Evaluate(Box::new(ExpressionError::Call(Box::new(error)))))
                })?;
                let takes_content = registration.overloads()[call.overload()].parameter("content").is_some();
                let bound: Vec<Value> = evaluated
                    .iter()
                    .enumerate()
                    .map(|(index, value)| call.args().get(index).unwrap_or(value).clone())
                    .collect();
                (bound, takes_content)
            }
            None => {
                let takes_content = evaluated.len() >= 2;
                (evaluated, takes_content)
            }
        };
        // From here on, an error is a side-effect dispatch failure (an unknown
        // verb, or an effect-engine error). Binding already checked arity for
        // every typed verb; the missing-argument error stays as a guard should
        // the typed catalog and this dispatch table ever diverge.
        let dispatch_err = |msg: String| ActionFailure::Dispatch(self.action_failure(verb, msg));
        let engine = self.inputs.effect_engine;
        let s = |idx: usize| -> Result<String, ActionFailure> {
            values
                .get(idx)
                .map(scalar_string)
                .ok_or_else(|| dispatch_err(format!("`{verb}` is missing a required argument")))
        };
        let path = |idx: usize| -> Result<String, ActionFailure> {
            let raw = s(idx)?;
            self.resolve_effect_path(verb, &raw)
        };
        let v = |idx: usize| -> Result<Value, ActionFailure> {
            values
                .get(idx)
                .cloned()
                .ok_or_else(|| dispatch_err(format!("`{verb}` is missing a required argument")))
        };

        // Every written value is the product of one evaluation, so it is data:
        // the host stores it so the next preparation reads it back instead of
        // scanning it. `working` keeps it raw.
        let stored = |idx: usize| v(idx).map(|value| self.inputs.host.persisted_data(&value));
        let result = match verb {
            "set_frontmatter" => engine.set_frontmatter(&s(0)?, &s(1)?, stored(2)?),
            "merge_frontmatter" => engine.merge_frontmatter(&s(0)?, stored(1)?),
            "delete_frontmatter" => engine.delete_frontmatter(&s(0)?, &s(1)?),
            "increment_frontmatter" => engine.increment_frontmatter(&s(0)?, &s(1)?),
            "decrement_frontmatter" => engine.decrement_frontmatter(&s(0)?, &s(1)?),
            "append_frontmatter" => engine.append_frontmatter(&s(0)?, &s(1)?, stored(2)?),
            "prepend_frontmatter" => engine.prepend_frontmatter(&s(0)?, &s(1)?, stored(2)?),
            "ensure_file" => {
                if takes_content {
                    engine.ensure_file_with_content(&path(0)?, &s(1)?).map(Value::String)
                } else {
                    engine.ensure_file(&path(0)?).map(Value::String)
                }
            }
            "ensure_dir" => engine.ensure_dir(&path(0)?).map(Value::String),
            "append_line" => engine.append_line(&path(0)?, &s(1)?).map(Value::String),
            "append_jsonl" => engine.append_jsonl(&path(0)?, v(1)?).map(Value::String),
            "http_post" => engine.http_post(&s(0)?, s(1)?.into_bytes()),
            other => return Err(dispatch_err(format!("unknown side effect `{other}`"))),
        };
        // The effect engine's error is the snapshot boundary's input: hand it
        // over typed so the projection reads it, rather than pre-flattening it
        // here and handing the snapshot prose.
        let out = result.map_err(|error| ActionFailure::Dispatch(self.error_or_action(verb, &error)))?;
        self.mirror_frontmatter_mutation(verb, &values, working);
        Ok(out)
    }

    /// Resolve against the pre-write snapshot; an absent destination is a
    /// declared null, so optional values can be copied before being reset.
    ///
    /// Every value reads the same pre-write snapshot, and no destination is
    /// written unless every value succeeds. Expression values evaluate first;
    /// whole-value `$( … )` values run afterwards, together, through the
    /// shell runner, so a failed expression runs no command. A command's
    /// external effects are not undone when a later value fails.
    fn dispatch_runtime_set(
        &self,
        set: &RuntimeSet,
        property: &str,
        working: &mut Map<String, Value>,
    ) -> Result<IndexMap<String, Value>, ActionFailure> {
        let mut snapshot = working.clone();
        for (key, _) in set.iter() {
            snapshot.entry(key.clone()).or_insert(Value::Null);
        }
        let mut resolved_values = HashMap::with_capacity(set.len());
        for (key, value) in set.iter() {
            if matches!(value, WithValue::Shell(_)) {
                continue;
            }
            let resolved = self.resolve_with_value(value, &snapshot).map_err(|(suffix, error)| {
                let value_property = format!("{property}.{key}{suffix}");
                let reason = LifecycleEvaluationReason::Expression;
                let cause = LifecycleCause::new(error);
                let mut info = self
                    .inputs
                    .host
                    .failure(LifecycleRunFailure::EvaluationFailed {
                        source_path: self.inputs.source_path.to_path_buf(),
                        event: self.event_name().to_string(),
                        surface: "set".to_string(),
                        message: cause.to_string(),
                        property: Some(value_property.clone()),
                        reason: reason.clone(),
                        cause: Some(cause),
                    })
                    .at_property(value_property);
                info.variant = "set".to_string();
                info.reason = reason;
                ActionFailure::Evaluation(info)
            })?;
            resolved_values.insert(key.clone(), resolved);
        }
        if set.has_shell_values() {
            resolved_values.extend(self.run_set_shell_values(set, property, &snapshot)?);
        }
        let updates: IndexMap<String, Value> = set
            .iter()
            .map(|(key, _)| {
                let value = resolved_values
                    .remove(key)
                    .expect("every set destination resolved to a value");
                (key.clone(), value)
            })
            .collect();

        let prior = self
            .inputs
            .host
            .commit_set(self.inputs.effect_engine, &updates, &snapshot)
            .map_err(|error| {
                let error: &(dyn StdError + 'static) = &*error;
                ActionFailure::Dispatch(self.error_or_action("set", error))
            })?;
        for (key, value) in updates {
            working.insert(key, value);
        }
        Ok(prior)
    }

    /// Run a `set` action's whole-value `$( … )` values against the pre-write
    /// `snapshot`.
    ///
    /// A value preflight never resolved does not run, nor does one approved in
    /// neither mode. A value approved by its shape runs its template, its
    /// placeholders read from `snapshot` with the event's globals; a refusal of
    /// its value, blacklisted fill, or program is an evaluation error. A runner that prohibits
    /// shell execution, and a user interruption, are evaluation errors that
    /// `no_error` cannot suppress; a command that fails is a dispatch error.
    fn run_set_shell_values(
        &self,
        set: &RuntimeSet,
        property: &str,
        snapshot: &Map<String, Value>,
    ) -> Result<Vec<(String, Value)>, ActionFailure> {
        let mut values: Vec<std::borrow::Cow<'_, ResolvedShellValue>> = Vec::new();
        let mut authored = String::new();
        let mut policy_root = None;
        for (key, shell) in set.shell_values() {
            let Some(resolved) = shell.resolved.as_ref() else {
                return Err(ActionFailure::Evaluation(
                    self.action_failure(
                        "set",
                        format!(
                            "`{property}.{key}` runs a command that pre-flight never resolved or \
                             approved, so it cannot run"
                        ),
                    )
                    .at_property(format!("{property}.{key}")),
                ));
            };
            authored.push_str(&shell.authored);
            // A value approved by its shape runs its template; one approved in
            // neither mode never runs.
            match shell.template.as_ref().map(|site| (site, site.mode())) {
                None | Some((_, Some(LifecycleShellMode::ResolvedBytes))) => {
                    values.push(std::borrow::Cow::Borrowed(resolved));
                }
                Some((site, Some(LifecycleShellMode::Template { policy_root: root }))) => {
                    self.ensure_shell_permitted("set")?;
                    policy_root = root.or(policy_root);
                    let value = resolved.clone().with_template(site.template().clone());
                    values.push(std::borrow::Cow::Owned(match site.late_values() {
                        Some(late) => value.with_late_values(late.clone()),
                        None => value,
                    }));
                }
                Some((site, None)) => {
                    self.ensure_shell_permitted("set")?;
                    return Err(self.not_approved("set", &format!("{property}.{key}"), &site.template().authored));
                }
            }
        }
        let values: Vec<&ResolvedShellValue> = values.iter().map(std::convert::AsRef::as_ref).collect();
        let run = |holes: Option<&dyn HoleEvaluator>| {
            let request = ShellValueRequest {
                values: &values,
                state: snapshot,
                source_path: self.inputs.source_path,
                context: self.early_binding_context(&authored),
                file_resolution_context: self.inputs.file_resolution_context,
                holes,
                policy_root: policy_root.as_deref(),
            };
            self.inputs.shell_runner.run_values(&request)
        };
        // Placeholders read the pre-write snapshot, as every other value does.
        let templated = values.iter().any(|value| value.template().is_some());
        let outcome = if templated {
            self.with_hole_evaluator(snapshot, &authored, |holes| run(Some(holes)))
                .map_err(|error| ActionFailure::Evaluation(self.expr_error("set", error)))?
        } else {
            run(None)
        };
        let interrupted = || cancellation::requested_with(|| self.inputs.host.interrupted());
        match outcome {
            Ok(results) if !interrupted() => Ok(results),
            Ok(_) => Err(ActionFailure::Evaluation(
                self.action_failure("set", "the run was interrupted while a `set` command ran")
                    .at_property(property.to_string()),
            )),
            Err(error @ ShellRunError::BeforePreflight) => Err(ActionFailure::Evaluation(
                self.error_or_action("set", &error).at_property(property.to_string()),
            )),
            // A template value refused before it ran (missing or rejected
            // value, blacklisted fill, interpreter target) is an input error,
            // never a command failure `no_error` may absorb.
            Err(ShellRunError::Value { source }) if templated && template_input_refusal(&source) => {
                let command = source.command().unwrap_or_default().to_string();
                Err(self.template_refusal(property, &command, source))
            }
            Err(error) if interrupted() => Err(ActionFailure::Evaluation(
                self.error_or_action("set", &error).at_property(property.to_string()),
            )),
            Err(error) => Err(ActionFailure::Dispatch(
                self.error_or_action("set", &error).at_property(property.to_string()),
            )),
        }
    }

    /// Resolve a document-authored mutation target through the same captured
    /// file-reference context used by composition.
    ///
    /// Existing targets follow ordinary document resolution precedence. For a
    /// missing implicit-relative target, creation stays anchored to the
    /// mutation root, preserving the effect engine's relative mutation policy;
    /// other reference kinds use their first document-scoped candidate. A later
    /// transclusion therefore resolves the same identity.
    fn resolve_effect_path(&self, verb: &str, raw: &str) -> Result<String, ActionFailure> {
        let request_context = self.inputs.file_resolution_context;
        let reference = FileReference::new(raw).map_err(|error| ActionFailure::Dispatch(self.error_or_action(verb, &error)))?;
        let document_context = request_context.for_source(self.inputs.source_path);
        let mutation_context = request_context.for_cwd(self.inputs.effect_engine.mutation_root());
        let resolved = reference
            .resolve_in_context(&document_context)
            .map_err(|error| ActionFailure::Dispatch(self.error_or_action(verb, &error)))?;
        let path = match resolved {
            Some(path) => path,
            None => reference
                .candidate_plan(if reference.class().kind == FileReferenceKind::ImplicitRelative {
                    &mutation_context
                } else {
                    &document_context
                })
                .map_err(|error| ActionFailure::Dispatch(self.error_or_action(verb, &error)))?
                .into_iter()
                .next()
                .map(|candidate| candidate.path().to_path_buf())
                .ok_or_else(|| {
                    ActionFailure::Dispatch(
                        self.action_failure(verb, format!("file reference `{raw}` has no local mutation target")),
                    )
                })?,
        };
        Ok(path.to_string_lossy().into_owned())
    }

    /// Mirror a successful frontmatter-verb mutation onto the in-memory
    /// `working` document state when it targets the document being processed.
    ///
    /// The side-effect engine writes to disk; this keeps the executor's live
    /// view consistent so a later action in the same stack resolves `{{ … }}`
    /// against the mutated value (just-in-time resolution). Mutations targeting
    /// any other file are not mirrored — they do not change this document.
    fn mirror_frontmatter_mutation(&self, verb: &str, values: &[Value], working: &mut Map<String, Value>) {
        let Some(Value::String(target)) = values.first() else {
            return;
        };
        if !self.targets_document(target) {
            return;
        }
        let prop = || values.get(1).and_then(Value::as_str);
        match verb {
            "set_frontmatter" => {
                if let (Some(p), Some(val)) = (prop(), values.get(2)) {
                    working.insert(p.to_string(), val.clone());
                }
            }
            "merge_frontmatter" => {
                if let Some(Value::Object(obj)) = values.get(1) {
                    for (k, v) in obj {
                        working.insert(k.clone(), v.clone());
                    }
                }
            }
            "delete_frontmatter" => {
                if let Some(p) = prop() {
                    working.remove(p);
                }
            }
            "increment_frontmatter" | "decrement_frontmatter" => {
                if let Some(p) = prop() {
                    let delta = if verb == "increment_frontmatter" { 1 } else { -1 };
                    let next = working.get(p).and_then(Value::as_i64).unwrap_or(0) + delta;
                    working.insert(p.to_string(), Value::from(next));
                }
            }
            "append_frontmatter" | "prepend_frontmatter" => {
                if let (Some(p), Some(val)) = (prop(), values.get(2)) {
                    let entry = working.entry(p.to_string()).or_insert_with(|| Value::Array(Vec::new()));
                    if let Value::Array(arr) = entry {
                        if verb == "append_frontmatter" {
                            arr.push(val.clone());
                        } else {
                            arr.insert(0, val.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Whether a side-effect target path refers to the document being processed.
    ///
    /// Resolves both the target and the document source against the effect
    /// engine's mutation root (relative paths join it; absolute paths win) and
    /// compares them lexically.
    fn targets_document(&self, target: &str) -> bool {
        let root = self.inputs.effect_engine.mutation_root();
        let resolve = |raw: &Path| -> PathBuf {
            if raw.is_absolute() {
                lexical_normalize(raw)
            } else {
                lexical_normalize(&root.join(raw))
            }
        };
        let target_clean = target.trim().trim_matches('"').trim_matches('\'');
        resolve(Path::new(target_clean)) == resolve(self.inputs.source_path)
    }

    /// Invoke a read-only expression function for its result, logging it in
    /// the lifecycle/status style.
    ///
    /// Operands follow the action-operand rule ([`Self::evaluate_operand`]):
    /// a template operand (`"n={{ fm.count }}"`) is interpolated once and the
    /// function receives the text it renders, never the template itself. A
    /// call with no template operand evaluates as the expression call it
    /// spells, so a context function keeps its caller-file provenance. A lazy
    /// function (`and`, `or`) still evaluates its operands one at a time and
    /// stops at the short-circuit, templates included.
    fn invoke_expression_function(&self, function: &str, args: &[Expr], fm: &Map<String, Value>) -> Result<Value, LifecycleExprError> {
        let value = if args.iter().any(is_template_operand) {
            self.call_with_operand_values(function, args, fm)?
        } else {
            let call = Expr::FunctionCall {
                name: function.to_string(),
                args: args.to_vec(),
            };
            self.eval_expr(&call, fm)?
        };
        warn!(
            function,
            result = %scalar_string(&value),
            "lifecycle expression-function action evaluated"
        );
        Ok(value)
    }

    /// Call `function` with each operand evaluated by
    /// [`Self::evaluate_operand`], through the runtime registry and binder.
    fn call_with_operand_values(&self, function: &str, args: &[Expr], fm: &Map<String, Value>) -> Result<Value, LifecycleExprError> {
        let call_error = |error| LifecycleExprError::Evaluate(Box::new(ExpressionError::Call(Box::new(error))));
        if let Some((registration, short_circuit)) = functions::lazy_function(function) {
            registration.check_arity(args.len()).map_err(call_error)?;
            let mut call = registration.lazy_call().expect("a lazy registration starts a lazy call");
            let stop_on_truthy = short_circuit == functions::ShortCircuit::OnTruthy;
            for arg in args {
                let value = self.evaluate_operand(arg, fm)?;
                if is_truthy(call.bind_next(value).map_err(call_error)?) == stop_on_truthy {
                    return Ok(Value::Bool(stop_on_truthy));
                }
            }
            call.finish().map_err(call_error)?;
            return Ok(Value::Bool(!stop_on_truthy));
        }
        let values = args
            .iter()
            .map(|arg| self.evaluate_operand(arg, fm))
            .collect::<Result<Vec<_>, _>>()?;
        let context = self.resolution_context()?;
        functions::call_function(function, &values, Some(&context))
            .unwrap_or_else(|| Err(unknown_function_error(function)))
            .map_err(|error| LifecycleExprError::Evaluate(Box::new(error)))
    }

    /// Resolve a parse-time [`LifecycleControlAction`] into a runtime
    /// [`StackControl`] by evaluating its expression arguments against `fm`.
    ///
    /// `fm` is the live intra-stack frontmatter, so a preceding
    /// `set_frontmatter` in the same stack is visible to every argument —
    /// including a `proxy.with` value.
    fn resolve_control(
        &self,
        control: &LifecycleControlAction,
        location: ActionLocation,
        fm: &Map<String, Value>,
    ) -> Result<StackControl, LifecycleErrorInfo> {
        use LifecycleControlAction as C;
        let verb = control.verb();
        let classify = |error: LifecycleExprError| self.expr_error(verb, error);
        Ok(match control {
            C::Stop => StackControl::Stop,
            C::Skip => StackControl::Skip,
            C::Error { reason } => StackControl::Error {
                reason: self.eval_opt_string(reason.as_ref(), fm).map_err(classify)?,
            },
            C::Proxy { target, with } => {
                // Atomicity: the target and the *complete* overlay resolve
                // before this returns, and this returns before anything acts on
                // the handoff. A failure in either leaves the source active with
                // no partial overlay installed and the target untouched.
                let target = self.render_message(target, fm).map_err(classify)?;
                let overlay = self.resolve_with_overlay(with, &target, &location, fm)?;
                StackControl::Proxy {
                    target,
                    overlay,
                    location,
                }
            }
            // The overlay resolves here, before the directive is returned, so an
            // evaluation failure leaves no retry for the runtime to dispatch:
            // no transition is taken and no attempt budget is spent.
            C::Retry {
                max_attempts,
                backoff,
                delay,
                with,
            } => StackControl::Retry {
                max_attempts: self
                    .eval_opt_u32(max_attempts.as_ref(), fm)
                    .map_err(classify)?
                    .unwrap_or(1),
                backoff: backoff.unwrap_or(RetryBackoff::Fixed),
                delay: self
                    .eval_opt_string(delay.as_ref(), fm)
                    .map_err(classify)?
                    .unwrap_or_else(|| "0s".to_string()),
                // A retry's target is the fresh re-read of this same document.
                overlay: self.resolve_with_overlay(
                    with,
                    &biscuit_file::to_portable_string(self.inputs.source_path),
                    &location,
                    fm,
                )?,
            },
            C::Resume { message, max_attempts } => StackControl::Resume {
                message: self.render_message(message, fm).map_err(classify)?,
                max_attempts: self
                    .eval_opt_u32(max_attempts.as_ref(), fm)
                    .map_err(classify)?
                    .unwrap_or(1),
            },
            C::Defer { delay, reason } => StackControl::Defer {
                delay: self.render_message(delay, fm).map_err(classify)?,
                reason: self.eval_opt_string(reason.as_ref(), fm).map_err(classify)?,
            },
            // A `break` belongs to the innermost loop around this page. With
            // none, it is a run-time error of the event, which `no_error`
            // cannot suppress, so it is never read as `stop`.
            C::Break { reason, code } => {
                if !self.inputs.host.in_loop() {
                    let mut info = self.inputs.host.failure(LifecycleRunFailure::BreakWithoutLoop {
                        source_path: self.inputs.source_path.to_path_buf(),
                        property: location.path().to_string(),
                    });
                    info.reason = LifecycleEvaluationReason::BreakWithoutLoop;
                    return Err(info);
                }
                let reason = self.eval_opt_string(reason.as_ref(), fm).map_err(classify)?;
                self.inputs.host.record_break(reason.clone(), *code);
                StackControl::Break { reason, code: *code }
            }
            C::Prep { .. } => unreachable!("`prep` is a returning action, run by `run_prep`"),
        })
    }

    /// Run a `prep` call to completion through the host.
    ///
    /// The target and the whole overlay evaluate first, as `proxy`'s do, so
    /// an evaluation failure calls nothing. A target that did not return
    /// successfully is a dispatch failure of this event, with the target's
    /// failure as its cause: the event takes its ordinary error route, and
    /// `no_error` suppresses it as it does a failed shell action.
    fn run_prep(
        &self,
        target: &Expr,
        with: &WithOverlay,
        location: &ActionLocation,
        fm: &Map<String, Value>,
    ) -> Result<(), ActionFailure> {
        let target = self
            .render_message(target, fm)
            .map_err(|error| ActionFailure::Evaluation(self.expr_error("prep", error)))?;
        let overlay = self
            .resolve_with_overlay(with, &target, location, fm)
            .map_err(ActionFailure::Evaluation)?;
        let property = location.path().to_string();
        let call = PrepRequest {
            target: &target,
            overlay: &overlay,
            caller: self.inputs.source_path,
            file_resolution_context: self.inputs.file_resolution_context,
            property: &property,
        };
        match self.inputs.host.prep(&call) {
            Ok(()) => Ok(()),
            Err(PrepError::Unavailable) => Err(ActionFailure::Evaluation(self.inputs.host.failure(
                LifecycleRunFailure::PrepUnavailable {
                    source_path: self.inputs.source_path.to_path_buf(),
                    property,
                },
            ))),
            Err(PrepError::Failed { message, snapshot }) => Err(ActionFailure::Dispatch(self.inputs.host.failure(
                LifecycleRunFailure::PrepFailed {
                    source_path: self.inputs.source_path.to_path_buf(),
                    property: property.clone(),
                    target: target.clone(),
                    message,
                    snapshot,
                },
            ))),
        }
    }

    /// Evaluate a whole `proxy.with` or `retry.with` mapping into the typed
    /// overlay the target will receive.
    ///
    /// `target` names the document the overlay is built for in a diagnostic:
    /// the proxy target, or the current document for `retry`. `location` is the
    /// authored directive, whose last path segment names the verb.
    ///
    /// Every value goes through the same subtree composition the rest of the
    /// lifecycle surface uses, so `with:` adds no second interpolation
    /// grammar. The first failing value aborts the whole mapping.
    fn resolve_with_overlay(
        &self,
        with: &WithOverlay,
        target: &str,
        location: &ActionLocation,
        fm: &Map<String, Value>,
    ) -> Result<IndexMap<String, Value>, LifecycleErrorInfo> {
        // Capture the snapshot-less fallback context ONCE for the whole overlay,
        // then walk every leaf against it. Recursive `resolve_with_value`
        // evaluates each scalar leaf independently; without this a
        // `prepared_context: None` caller re-runs a demand-driven capture per
        // leaf. A prepared caller already reuses its single snapshot.
        match self.inputs.prepared_context {
            Some(_) => self.walk_with_overlay(self, with, target, location, fm),
            None => {
                let base = self
                    .inputs
                    .ctx_base_dir
                    .or(self.inputs.base_dir)
                    .unwrap_or_else(|| Path::new("."));
                let content = proxy_with_scan_content(with);
                let fallback = self.inputs.host.capture_overlay_context(base, &content);
                let walker = self.with_prepared_context(&fallback);
                self.walk_with_overlay(&walker, with, target, location, fm)
            }
        }
    }

    /// Walk `with`'s leaves through `walker`, rooting a diagnostic at the exact
    /// nested property on failure.
    fn walk_with_overlay(
        &self,
        walker: &StackExecutor<'_>,
        with: &WithOverlay,
        target: &str,
        location: &ActionLocation,
        fm: &Map<String, Value>,
    ) -> Result<IndexMap<String, Value>, LifecycleErrorInfo> {
        let mut overlay = IndexMap::with_capacity(with.len());
        for (key, value) in with.iter() {
            let resolved = walker
                .resolve_with_value(value, fm)
                .and_then(|resolved| reject_control_plane_template(self.inputs.host.registry(), key, resolved))
                .map_err(|(suffix, error)| {
                    let cause = LifecycleCause::new(error);
                    self.inputs.host.failure(LifecycleRunFailure::WithEvaluationFailed {
                        source_path: self.inputs.source_path.to_path_buf(),
                        property: location.to_string(),
                        path: format!("with.{key}{suffix}"),
                        target: target.to_string(),
                        message: cause.to_string(),
                        cause: Some(cause),
                    })
                })?;
            overlay.insert(key.clone(), resolved);
        }
        Ok(overlay)
    }

    /// Resolve one `with:` value tree.
    ///
    /// On failure returns the path suffix *below* the overlay key (e.g.
    /// `.metadata.area`, `[2]`) alongside the reason, so the caller can root a
    /// diagnostic at the exact nested property without the value ever being
    /// echoed.
    fn resolve_with_value(&self, value: &WithValue, fm: &Map<String, Value>) -> Result<Value, (String, LifecycleExprError)> {
        match value {
            WithValue::Null => Ok(Value::Null),
            // Only a `set` mapping holds one, and `set` runs it separately.
            WithValue::Shell(shell) => Err((
                String::new(),
                LifecycleExprError::prose(format!(
                    "`{}` is a shell value, which only a `set` action runs",
                    shell.authored
                )),
            )),
            WithValue::Scalar(expr) => self.resolve_typed_value(expr, fm).map_err(|msg| (String::new(), msg)),
            WithValue::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    self.resolve_with_value(item, fm)
                        .map_err(|(suffix, msg)| (format!("[{i}]{suffix}"), msg))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            WithValue::Object(map) => map
                .iter()
                .map(|(k, v)| {
                    self.resolve_with_value(v, fm)
                        .map(|resolved| (k.clone(), resolved))
                        .map_err(|(suffix, msg)| (format!(".{k}{suffix}"), msg))
                })
                .collect::<Result<Map<_, _>, _>>()
                .map(Value::Object),
        }
    }

    /// Evaluate one expression at event time, **preserving its type**.
    ///
    /// The type-preserving counterpart of [`Self::render_message`], which folds
    /// everything to a display string via `scalar_string` — the path that
    /// reduces `proxy.target` to text. A whole-value span keeps its resolved
    /// `bool`/number/array/object/null; a mixed string interpolates and stays
    /// a string.
    ///
    /// An absent document property resolves to `null`. The resolved value is
    /// data: a `{{ … }}` inside it — at any depth of a container — reaches a
    /// `set:` or a `proxy.with:` target as text, never as a template.
    fn resolve_typed_value(&self, expr: &Expr, fm: &Map<String, Value>) -> Result<Value, LifecycleExprError> {
        self.evaluate_operand(expr, fm)
    }

    /// Evaluate an optional expression to a display string.
    fn eval_opt_string(&self, expr: Option<&Expr>, fm: &Map<String, Value>) -> Result<Option<String>, LifecycleExprError> {
        match expr {
            Some(expr) => Ok(Some(self.render_message(expr, fm)?)),
            None => Ok(None),
        }
    }

    /// Evaluate an optional expression to a non-negative integer count.
    fn eval_opt_u32(&self, expr: Option<&Expr>, fm: &Map<String, Value>) -> Result<Option<u32>, LifecycleExprError> {
        let Some(expr) = expr else {
            return Ok(None);
        };
        let value = self.eval_expr(expr, fm)?;
        let n = value
            .as_f64()
            .ok_or_else(|| LifecycleExprError::prose(format!("expected a number, got {value}")))?;
        if n < 0.0 || n.fract() != 0.0 {
            return Err(LifecycleExprError::prose(format!(
                "expected a non-negative whole number, got {n}"
            )));
        }
        Ok(Some(n as u32))
    }
}

/// Combined `ctx.*` scan content across a whole `with:` overlay, so a single
/// fallback [`ComposeContext`] capture covers every leaf.
///
/// A mixed-interpolation leaf is an [`Expr::StringLiteral`] whose raw text still
/// holds its `{{ … }}` spans, so that text is scanned directly; a whole-value
/// span is a parsed [`Expr`], scanned as its rendering. The union feeds
/// [`ComposeContext::capture_for_content`], which stays datetime-only when no
/// leaf references `ctx.*`.
fn proxy_with_scan_content(with: &WithOverlay) -> String {
    let mut content = String::new();
    for (_, value) in with.iter() {
        push_proxy_with_scan_content(value, &mut content);
    }
    content
}

/// Recursively append one `with:` value's scan content to `content`.
fn push_proxy_with_scan_content(value: &WithValue, content: &mut String) {
    match value {
        WithValue::Null => {}
        WithValue::Shell(shell) => {
            content.push(' ');
            content.push_str(&shell.authored);
        }
        WithValue::Scalar(Expr::StringLiteral(s)) => {
            content.push(' ');
            content.push_str(s);
        }
        WithValue::Scalar(expr) => {
            content.push(' ');
            content.push_str(&expr.to_string());
        }
        WithValue::Array(items) => {
            for item in items {
                push_proxy_with_scan_content(item, content);
            }
        }
        WithValue::Object(map) => {
            for (_, value) in map {
                push_proxy_with_scan_content(value, content);
            }
        }
    }
}

/// Whether an action operand is a template, interpolated once, rather than an
/// expression evaluated to data ([`evaluate_operand_in`]): an authored literal
/// body that [`is_deferred_template`] recognizes (the one template rule passive
/// operand validation also uses), or one holding an interpolation literal
/// (`{{{ … }}}`) or a literal token (`{{!…}}`), which only the compose pass
/// unwraps or refuses. Text whose `{{` the scanner recognizes as none of
/// these (an unclosed or escaped opener) stays the literal it is written as.
///
/// [`is_deferred_template`]: crate::markdown::compose::expression::is_deferred_template
pub fn is_template_operand(expr: &Expr) -> bool {
    let Expr::StringLiteral(text) = expr else {
        return false;
    };
    if crate::markdown::compose::expression::is_deferred_template(text) {
        return true;
    }
    let scan = ExpressionFinder::scan_plain(text);
    !scan.literals.is_empty() || !scan.tokens.is_empty()
}

/// Evaluate one action operand against an established state, as lifecycle
/// playback does: a template is interpolated, every other expression yields
/// data that is never scanned again.
///
/// Shared by event-time playback and a host's speculative resolution (speech
/// pre-warm), so both produce the same text from the same inputs.
///
/// ## Errors
///
/// The evaluation or compose failure.
pub fn evaluate_operand_in(
    expr: &Expr,
    state: &EffectiveState,
    view: Arc<BindingView>,
    globals: HashMap<String, RuntimeBinding<'static>>,
    resolution_context: Option<ResolutionContext>,
) -> Result<Value, LifecycleExprError> {
    match expr {
        Expr::StringLiteral(text) if is_template_operand(expr) => {
            interpolate_text_in(text, state, view, globals, resolution_context)
        }
        _ => evaluate_expr_in(expr, state, view, globals, resolution_context),
    }
}

/// The origin a lifecycle template diagnostic names: the authored property.
fn template_origin(property: &str) -> ShellCommandOrigin {
    ShellCommandOrigin::Frontmatter {
        key: property.to_string(),
        line: None,
    }
}

/// Whether a `set` value's failure refused a template command before it ran.
fn template_input_refusal(error: &ShellExpansionError) -> bool {
    matches!(
        error,
        ShellExpansionError::MissingValue { .. }
            | ShellExpansionError::TemplateValueRejected { .. }
            | ShellExpansionError::TemplateGrammar { .. }
            | ShellExpansionError::TemplateTarget { .. }
            | ShellExpansionError::Blacklisted { .. }
    )
}

fn evaluate_expr_in(
    expr: &Expr,
    state: &EffectiveState,
    view: Arc<BindingView>,
    globals: HashMap<String, RuntimeBinding<'static>>,
    resolution_context: Option<ResolutionContext>,
) -> Result<Value, LifecycleExprError> {
    let lookup = layered_session(state, globals, Some(view), resolution_context)
        .map_err(|error| LifecycleExprError::Evaluate(Box::new(ExpressionError::Binding(Box::new(error)))))?;
    evaluate(expr, &lookup).map_err(|error| LifecycleExprError::Evaluate(Box::new(error)))
}

fn interpolate_text_in(
    text: &str,
    state: &EffectiveState,
    view: Arc<BindingView>,
    globals: HashMap<String, RuntimeBinding<'static>>,
    resolution_context: Option<ResolutionContext>,
) -> Result<Value, LifecycleExprError> {
    let value = Value::String(text.to_string());
    let compose = SubtreeCompose::new(&value, state)
        .with_globals(globals)
        .with_binding_view(view);
    let compose = match resolution_context {
        Some(context) => compose.with_resolution_context(context),
        None => compose,
    };
    compose
        .compose()
        .map_err(|error| LifecycleExprError::Compose(Box::new(error)))
}

/// Refuse template syntax in a `with:` value that becomes lifecycle
/// configuration in the target.
///
/// Every other overlay key reaches the target as a data layer, so a `{{ … }}`
/// in it is text. A key the host's registry names as a lifecycle stack
/// (`success:`, `loop:`, … under Claudine) is different: the
/// target reparses it as its own stack or loop, and those strings are
/// evaluated at event time, outside compose. Run-time data carrying template
/// syntax there would become an instruction, so it fails closed. On failure
/// returns the path suffix below the key, as
/// [`StackExecutor::resolve_with_value`] does.
fn reject_control_plane_template(
    registry: &EventRegistry,
    key: &str,
    resolved: Value,
) -> Result<Value, (String, LifecycleExprError)> {
    if !registry.authored_event_keys().contains(&key) {
        return Ok(resolved);
    }
    match first_template_span(&resolved, &mut String::new()) {
        None => Ok(resolved),
        Some((suffix, span)) => Err((
            suffix,
            LifecycleExprError::prose(format!(
                "the value holds `{span}`, but `{key}` becomes the target's lifecycle \
                 configuration, where template syntax would run as an instruction"
            )),
        )),
    }
}

/// The path suffix and text of the first `{{ … }}` span anywhere in `value`.
fn first_template_span(value: &Value, suffix: &mut String) -> Option<(String, String)> {
    match value {
        Value::String(text) => ExpressionFinder::find_all_plain(text)
            .first()
            .map(|span| (suffix.clone(), text[span.start..span.end].to_string())),
        Value::Array(items) => items.iter().enumerate().find_map(|(index, item)| {
            let mark = suffix.len();
            suffix.push_str(&format!("[{index}]"));
            let found = first_template_span(item, suffix);
            suffix.truncate(mark);
            found
        }),
        Value::Object(map) => map.iter().find_map(|(key, item)| {
            let mark = suffix.len();
            suffix.push_str(&format!(".{key}"));
            let found = first_template_span(item, suffix);
            suffix.truncate(mark);
            found
        }),
        _ => None,
    }
}

/// Lexically normalize a path (resolve `.`/`..` components) without touching the
/// filesystem, so two paths that name the same document compare equal.
fn lexical_normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests;
