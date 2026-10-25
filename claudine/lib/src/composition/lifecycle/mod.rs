//! Lifecycle stacks for composition frontmatter.
//!
//! This module provides support for the eight composition lifecycle events
//! (`initialize`, `start`, `success`, `blocked`, `failure`, `finalize`,
//! `loop`, `cancelled`). Each event is a stack: an ordered action list, or one dictionary
//! of verbs that expands into that list in canonical order (see
//! [`actions::LifecycleNode`]). The `loop:` block additionally carries
//! iteration controls parsed by [`super::looping::resolve_loop_config`]; its
//! lifecycle concerns are loose verb keys or a `gate:` list.
//!
//! The action model and its parser live in [`darkmatter::lifecycle`]; this
//! module re-exports them, supplies Claudine's parse profile (the Playa effect
//! catalog, unknown verbs rejected), maps [`darkmatter::lifecycle::error::LifecycleError`]
//! onto [`CompositionError`], and executes the parsed stacks.
//!
//! Communication reaches a [`LifecycleEmitter`] through
//! [`effects::ClaudineLifecycleEffects`], Claudine's implementation of
//! Darkmatter's [`LifecycleEffects`] service.
//!
//! The run guard is Darkmatter's
//! ([`darkmatter::lifecycle::run_guard::LifecycleRunGuard`]); this module names
//! it over [`ClaudineRunHost`] and keeps its Claudine constructor
//! ([`LifecycleRunGuardExt::new`]) and the [`LifecycleRuntimeContext`] it is
//! built from.

// rustfmt doesn't support let-chains yet, so nested ifs are required
#![allow(clippy::collapsible_if)]

use std::path::Path;

use biscuit_speaks::{SpeedLevel, TtsConfig, TtsFailoverStrategy};
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::terminal::Terminal;
use darkmatter::lifecycle::LifecycleEffects;
use tracing::warn;

use super::error::CompositionError;
use self::actions::{
    LifecycleAction, LifecycleNode,
};
// Only the lifecycle tests use these, through `super::*`.
#[cfg(test)]
use self::actions::{CommunicationChannel, LifecycleActionKind};
#[cfg(test)]
use darkmatter::markdown::compose::expression::{Expr, ExpressionFinder};
use crate::events::{GlobalSettings, TtsSettings};
use crate::messaging::RuntimeMessagingSettings;

pub use darkmatter::lifecycle::{actions, policy, shell_template};
mod audio;
pub mod bindings;
mod binding_checks;
pub mod cancellation;
pub mod context;
pub mod control;
pub mod effects;
pub mod executor;
pub mod prep;
pub mod registry;
pub mod runtime;
mod structured_shell;
pub mod prewarm;

pub(crate) use audio::*;
pub use binding_checks::validate_no_err_in_no_error_events;
pub(crate) use darkmatter::lifecycle::{LOOP_ITERATION_KEYS, action_value_to_expr, closest_spelling};
pub use registry::claudine_registry;
pub use darkmatter::lifecycle::{
    LifecycleConfig, LifecycleParseOptions, LifecycleSignal, LifecycleStacks,
    UnknownVerbPolicy, collect_lifecycle_shell_commands, collect_lifecycle_shell_commands_for,
    scan_removed_validation_keys,
};

/// Rejects a `{{ … }}` span authored inside a quoted string literal on a
/// single-pass lifecycle surface; see
/// [`darkmatter::lifecycle::validate_no_nested_spans_in_literals`].
///
/// ## Errors
///
/// - [`CompositionError::LifecycleNestedSpanInLiteral`] for the first defect.
/// - [`CompositionError::LifecycleInvalid`] when a parsed stack surface has no
///   authored source record.
pub fn validate_no_nested_spans_in_literals(
    frontmatter: &serde_json::Value,
    lifecycle: &LifecycleConfig,
    source_path: &Path,
) -> Result<(), CompositionError> {
    darkmatter::lifecycle::validate_no_nested_spans_in_literals(frontmatter, lifecycle, source_path)
        .map_err(CompositionError::from)
}

/// [`darkmatter::lifecycle::reject_incompatible_calls`] under Claudine's
/// event registry: a lifecycle call no value of its argument's type can
/// satisfy rejects the document before any event runs, as it would in the
/// body.
///
/// ## Returns
///
/// The lifecycle surfaces' call warnings (category 3, always-false calls),
/// located in `authored` (the loaded document text) when given; the caller
/// reports them with composition's own warnings.
///
/// ## Errors
///
/// [`CompositionError::LifecycleCallRejected`] for the first rejected call.
pub fn reject_incompatible_calls(
    frontmatter: &serde_json::Value,
    lifecycle: &LifecycleConfig,
    source_path: &Path,
    authored: Option<&str>,
    schema: Option<&darkmatter::markdown::schemas::EffectiveSchema>,
) -> Result<Vec<darkmatter::markdown::compose::ComposeWarning>, CompositionError> {
    darkmatter::lifecycle::reject_incompatible_calls(
        frontmatter,
        lifecycle,
        source_path,
        authored,
        registry::claudine_registry(),
        schema,
    )
    .map_err(CompositionError::from)
}

/// Check the shape every `with:` shares and return its mapping; see
/// [`darkmatter::lifecycle::with_mapping_shape`].
///
/// ## Errors
///
/// The `LifecycleWith*` shape family of [`CompositionError`].
pub(crate) fn with_mapping_shape<'a>(
    value: &'a serde_json::Value,
    source_file: &Path,
    property: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, CompositionError> {
    darkmatter::lifecycle::with_mapping_shape(value, source_file, property).map_err(CompositionError::from)
}

/// The parse options of Claudine's lifecycle profile: Claudine's event
/// registry, the embedded Playa effect catalog, and unknown verbs rejected
/// because every verb must run.
fn claudine_parse_options<'a>(
    authored: Option<&'a darkmatter::markdown::Frontmatter>,
) -> LifecycleParseOptions<'a> {
    LifecycleParseOptions {
        is_known_effect: &is_known_effect,
        authored,
        unknown_verbs: UnknownVerbPolicy::Reject,
        registry: claudine_registry(),
    }
}

/// Parses lifecycle configuration from composition frontmatter.
///
/// Every event (`initialize`, `start`, `success`, `blocked`, `failure`,
/// `finalize`, `cancelled`) and the `loop:` block's lifecycle concerns parse into an
/// ordered [`LifecycleNode`] stack on [`LifecycleConfig::stacks`]. Literal
/// `effect` names are checked against the embedded Playa catalog here, so an
/// unknown name fails before anything runs; a `{{ … }}` name is checked when
/// the event resolves it.
///
/// ## Errors
///
/// The lifecycle authoring family of [`CompositionError`]: Darkmatter's
/// spelling of an event Claudine renamed (`composed`, `failed`;
/// [`CompositionError::LifecycleRenamedEvent`]), shape violations
/// ([`CompositionError::LifecycleStackInvalidShape`]), removed spellings
/// ([`CompositionError::RemovedGrammar`]), unknown verbs or parameters,
/// directive placement and order, and
/// [`CompositionError::LifecycleUnknownEffect`].
///
/// ## Examples
///
/// ```
/// # use serde_json::json;
/// # use claudine::composition::{parse_lifecycle_config, LifecycleSignal};
/// let frontmatter = json!({
///     "title": "My Composition",
///     "start": [{ "message": "Starting..." }]
/// });
/// let config = parse_lifecycle_config(&frontmatter, std::path::Path::new("test.md")).unwrap();
/// assert_eq!(config.stack(LifecycleSignal::Start).len(), 1);
/// ```
pub fn parse_lifecycle_config(
    frontmatter: &serde_json::Value,
    source_file: &Path,
) -> Result<LifecycleConfig, CompositionError> {
    parse_lifecycle_config_with_orders(frontmatter, source_file, None)
}

/// [`parse_lifecycle_config`] with the authored frontmatter, whose retained
/// mapping order gives a `set:` mapping its authored key order.
pub(crate) fn parse_lifecycle_config_with_orders(
    frontmatter: &serde_json::Value,
    source_file: &Path,
    authored: Option<&darkmatter::markdown::Frontmatter>,
) -> Result<LifecycleConfig, CompositionError> {
    darkmatter::lifecycle::parse_lifecycle_config_with(
        frontmatter,
        source_file,
        &claudine_parse_options(authored),
    )
    .map_err(CompositionError::from)
}

/// Parse the lifecycle concerns of a `loop:` block authored at `property`
/// (`loop`, `tasks[2].loop`): its loose verb keys or its `gate:` stack, by the
/// grammar a document's `loop:` uses. Iteration controls are skipped.
///
/// ## Errors
///
/// The same [`CompositionError`] family [`parse_lifecycle_config`] raises.
pub(crate) fn parse_loop_gate(
    loop_obj: &serde_json::Map<String, serde_json::Value>,
    source_file: &Path,
    property: &str,
    authored: &darkmatter::lifecycle::AuthoredOrder,
) -> Result<Vec<LifecycleNode>, CompositionError> {
    darkmatter::lifecycle::parse_loop_concerns_with(
        loop_obj,
        source_file,
        property,
        authored,
        &claudine_parse_options(None),
    )
    .map_err(CompositionError::from)
}

/// Parse a task's `setup:` / `teardown:` value into a typed action stack.
///
/// The value is a stack like any event: a list, or one dictionary of verbs.
/// `property` is the source-rooted path of the value itself
/// (`tasks[0].setup`), and item `n` is `{property}[n]`.
///
/// ## Errors
///
/// The same [`CompositionError`] family [`parse_lifecycle_config`] raises.
pub fn parse_task_action_stack(
    signal: LifecycleSignal,
    raw: &serde_json::Value,
    source_file: &Path,
    property: &str,
) -> Result<Vec<LifecycleNode>, CompositionError> {
    parse_task_action_stack_with_order(
        signal,
        raw,
        source_file,
        property,
        &darkmatter::lifecycle::AuthoredOrder::default(),
    )
}

/// [`parse_task_action_stack`] against the stack's authored key order, which
/// is positioned at the `setup:`/`teardown:` value itself.
pub(crate) fn parse_task_action_stack_with_order(
    signal: LifecycleSignal,
    raw: &serde_json::Value,
    source_file: &Path,
    property: &str,
    authored: &darkmatter::lifecycle::AuthoredOrder,
) -> Result<Vec<LifecycleNode>, CompositionError> {
    darkmatter::lifecycle::parse_task_action_stack_with(
        signal,
        raw,
        source_file,
        property,
        authored,
        &claudine_parse_options(None),
    )
    .map_err(CompositionError::from)
}

/// Parse a task's single `side_effect:` action.
///
/// `property` is the source-rooted path of the action value itself
/// (`tasks[0].side_effect`); the action's path is `{property}.{verb}`.
///
/// ## Errors
///
/// [`CompositionError::LifecycleStackInvalidShape`] when the value is not a
/// one-verb dictionary, and the standard action rejections otherwise.
pub fn parse_single_action(
    signal: LifecycleSignal,
    raw: &serde_json::Value,
    source_file: &Path,
    property: &str,
) -> Result<LifecycleAction, CompositionError> {
    parse_single_action_with_order(signal, raw, source_file, property, None)
}

/// [`parse_single_action`] with a `set:` mapping's authored key order.
pub(crate) fn parse_single_action_with_order(
    signal: LifecycleSignal,
    raw: &serde_json::Value,
    source_file: &Path,
    property: &str,
    authored_set_order: Option<&[String]>,
) -> Result<LifecycleAction, CompositionError> {
    darkmatter::lifecycle::parse_single_action_with(
        signal,
        raw,
        source_file,
        property,
        authored_set_order,
        &claudine_parse_options(None),
    )
    .map_err(CompositionError::from)
}

/// Runtime state tracking for lifecycle events.
///
/// Superseded by [`LifecycleRunGuard`] which enforces transitions mechanically.
/// Retained for backward compatibility.
#[deprecated(
    since = "0.1.0",
    note = "use LifecycleRunGuard instead, which enforces transitions mechanically"
)]
#[derive(Debug, Clone, Default)]
pub struct LifecycleRuntimeState {
    /// Whether the `start` signal has been emitted.
    pub start_emitted: bool,

    /// Whether the provider launch has started (used for start signal timing).
    pub provider_launch_started: bool,
}

// ---------------------------------------------------------------------------
// Emitter trait + default implementation
// ---------------------------------------------------------------------------

/// Trait for emitting lifecycle communication side effects.
///
/// Injectable to allow test doubles that capture emissions without hitting
/// real stderr, messaging, TTS, or sound playback. The text channels keep the
/// rows of their text: a newline (authored, or inside an interpolated value
/// such as a captured output) starts a new line.
pub trait LifecycleEmitter: Sync {
    /// Write a plain prose line (no status glyph) to stderr.
    ///
    /// `stderr` is the statusless channel: rich text and links are honored, but
    /// no `Status` decoration is applied. Status decoration is reserved for
    /// [`emit_info`](Self::emit_info) and [`emit_warn`](Self::emit_warn).
    fn emit_stderr(&self, signal: LifecycleSignal, text: &str, term: &Terminal);

    /// Dispatch a message via the configured messaging route.
    fn emit_message(
        &self,
        text: &str,
        source_path: &Path,
        repo_root: Option<&Path>,
        messaging: &RuntimeMessagingSettings,
    );

    /// Speak text via TTS.
    fn emit_speech(&self, text: &str, tts_config: TtsConfig);

    /// Play a named sound effect.
    fn emit_effect(&self, name: &str);

    /// Fire a local desktop notification.
    fn emit_notification(&self, title: &str);

    /// Write a styled `Info` status line to stderr.
    ///
    /// Unlike [`emit_stderr`](Self::emit_stderr), the status state is fixed to
    /// [`StatusState::Info`] regardless of the owning lifecycle event. The
    /// default renders the same way [`DefaultLifecycleEmitter`] would.
    fn emit_info(&self, text: &str, term: &Terminal) {
        let rendered = darkmatter::lifecycle::render_status_line(StatusState::Info, text, term);
        crate::render::console::write_stderr_line(&rendered);
    }

    /// Write a styled `Warning` status line to stderr.
    ///
    /// The status state is fixed to [`StatusState::Warning`] regardless of the
    /// owning lifecycle event.
    fn emit_warn(&self, text: &str, term: &Terminal) {
        let rendered = darkmatter::lifecycle::render_status_line(StatusState::Warning, text, term);
        crate::render::console::write_stderr_line(&rendered);
    }

    /// Write a styled `Success` status line to stderr.
    ///
    /// The status state is fixed to [`StatusState::Success`] regardless of the
    /// owning lifecycle event.
    fn emit_success(&self, text: &str, term: &Terminal) {
        let rendered = darkmatter::lifecycle::render_status_line(StatusState::Success, text, term);
        crate::render::console::write_stderr_line(&rendered);
    }

    /// Write a plain prose line (no status glyph) to **stdout**.
    ///
    /// The only lifecycle channel that targets stdout. Inline styling and links
    /// are honored, but no status decoration is applied. Authors opt into this
    /// knowing stdout is otherwise reserved for pipeable command data.
    fn emit_stdout(&self, text: &str, term: &Terminal) {
        let rendered = darkmatter::lifecycle::render_prose_line(text, term);
        crate::render::console::write_stdout_line(&rendered);
    }
}

/// Production emitter that performs real side effects.
pub struct DefaultLifecycleEmitter;

impl LifecycleEmitter for DefaultLifecycleEmitter {
    fn emit_stderr(&self, _signal: LifecycleSignal, text: &str, term: &Terminal) {
        // `stderr` carries no status: it is plain prose (rich text/links honored)
        // routed to STDERR. Status glyphs belong to `info`/`warn` only.
        let rendered = darkmatter::lifecycle::render_prose_line(text, term);
        crate::render::console::write_stderr_line(&rendered);
    }

    fn emit_message(
        &self,
        text: &str,
        source_path: &Path,
        repo_root: Option<&Path>,
        messaging: &RuntimeMessagingSettings,
    ) {
        crate::messaging::execute_resolved_message(
            text,
            None,
            Some(source_path),
            repo_root,
            messaging,
        );
    }

    fn emit_speech(&self, text: &str, tts_config: TtsConfig) {
        enqueue_speech(text, tts_config);
    }

    fn emit_effect(&self, name: &str) {
        enqueue_effect(name);
    }

    fn emit_notification(&self, title: &str) {
        crate::messaging::execute_notification(title, None);
    }
}

/// The run guard Claudine's drivers hold: Darkmatter's
/// [`darkmatter::lifecycle::run_guard::LifecycleRunGuard`] over
/// [`ClaudineRunHost`].
///
/// Construct it with [`LifecycleRunGuardExt::new`]; the accounting (the one
/// terminal slot, `finalize` once, the shell lock until `start`, proxy and
/// iteration resets, the drop safety net) is Darkmatter's.
pub type LifecycleRunGuard<'a> = darkmatter::lifecycle::run_guard::LifecycleRunGuard<'a, ClaudineRunHost<'a>>;

/// Claudine's side of a lifecycle run: the runtime context and emitter a
/// [`LifecycleRunGuard`] was built with, and the communication service built
/// from them once for the run.
pub struct ClaudineRunHost<'a> {
    ctx: &'a LifecycleRuntimeContext<'a>,
    emitter: &'a dyn LifecycleEmitter,
    effects: effects::ClaudineLifecycleEffects<'a>,
}

impl darkmatter::lifecycle::run_guard::LifecycleRunHost for ClaudineRunHost<'_> {
    type TerminalScope = crate::interrupt::TerminalLifecycleScope;

    fn source_path(&self) -> &Path {
        self.ctx.source_path
    }

    fn prepared_context(&self) -> Option<&darkmatter::markdown::compose::ComposeContext> {
        self.ctx.context
    }

    fn effects(&self) -> &dyn LifecycleEffects {
        &self.effects
    }

    fn diagnostics(&self) -> &dyn darkmatter::lifecycle::DiagnosticCatalog {
        &crate::diagnostics::ClaudineDiagnostics
    }

    fn interrupted(&self) -> bool {
        crate::interrupt::interrupted()
    }

    /// A repeat Ctrl+C gives an in-flight terminal event a short grace window
    /// rather than force-exiting through its side effects.
    fn enter_terminal_event(&self) -> Self::TerminalScope {
        crate::interrupt::TerminalLifecycleScope::enter()
    }

    fn failure(
        &self,
        failure: darkmatter::lifecycle::executor::LifecycleRunFailure,
    ) -> context::LifecycleErrorInfo {
        executor::classify_run_failure(failure)
    }
}

/// Claudine's constructor and accessors for [`LifecycleRunGuard`].
pub trait LifecycleRunGuardExt<'a> {
    /// Create a guard for a run of `config` whose communication `emitter`
    /// delivers with `ctx`'s settings.
    fn new(
        config: &'a LifecycleConfig,
        ctx: &'a LifecycleRuntimeContext<'a>,
        emitter: &'a dyn LifecycleEmitter,
    ) -> Self;

    /// The runtime context this guard was constructed with.
    fn context(&self) -> &'a LifecycleRuntimeContext<'a>;

    /// The emitter this guard delegates communication to.
    fn emitter(&self) -> &'a dyn LifecycleEmitter;
}

impl<'a> LifecycleRunGuardExt<'a> for LifecycleRunGuard<'a> {
    fn new(
        config: &'a LifecycleConfig,
        ctx: &'a LifecycleRuntimeContext<'a>,
        emitter: &'a dyn LifecycleEmitter,
    ) -> Self {
        let effects = effects::ClaudineLifecycleEffects::new(
            emitter,
            ctx.settings,
            ctx.messaging,
            ctx.term,
            ctx.source_path,
            ctx.repo_root,
        );
        Self::with_host(config, ClaudineRunHost { ctx, emitter, effects })
    }

    fn context(&self) -> &'a LifecycleRuntimeContext<'a> {
        self.host().ctx
    }

    fn emitter(&self) -> &'a dyn LifecycleEmitter {
        self.host().emitter
    }
}

/// Runtime context required for emitting lifecycle communication.
///
/// Holds references to settings, messaging configuration, terminal, and paths
/// needed to resolve and emit lifecycle communication actions. A
/// [`LifecycleRunGuard`] built from it resolves that configuration into one
/// [`effects::ClaudineLifecycleEffects`] for the whole run.
#[derive(Debug)]
pub struct LifecycleRuntimeContext<'a> {
    /// Global settings (includes TTS configuration).
    pub settings: &'a GlobalSettings,

    /// Runtime messaging settings.
    pub messaging: &'a RuntimeMessagingSettings,

    /// Terminal for output rendering.
    pub term: &'a Terminal,

    /// Path to the composition source file.
    pub source_path: &'a Path,

    /// Repository root (if in a git repository).
    pub repo_root: Option<&'a Path>,

    /// Directory the caller launched from (the package area), used as the base
    /// for `ctx.*` capture in lifecycle events; falls back to the prompt/source
    /// directory when `None`.
    pub launch_area: Option<&'a Path>,

    /// The single early-binding context snapshot captured once at composition
    /// start (against the launch area) and shared with body compose and
    /// preflight. When `Some`, lifecycle events reuse this exact `ctx.*`/`env.*`
    /// snapshot so they can never diverge from the body; when `None`, the
    /// executor falls back to a demand-driven re-capture rooted at the launch
    /// area. `current.*` stays event-time regardless.
    pub context: Option<&'a darkmatter::markdown::compose::ComposeContext>,
}

/// Removed harness validation/handler DSL keys and their lifecycle
/// replacements.
///
/// These top-level frontmatter keys were retired when the lifecycle stack
/// model replaced the harness validation and handler execution layers.
#[cfg(test)]
mod tests;
