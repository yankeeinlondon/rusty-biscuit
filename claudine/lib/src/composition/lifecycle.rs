//! Lifecycle notification types and parsing for composition frontmatter.
//!
//! This module provides support for `start`, `success`, `blocked`, and `failure`
//! lifecycle notifications in composition frontmatter. Each notification can
//! specify optional fields like `say`, `effect`, `message`, etc.

// rustfmt doesn't support let-chains yet, so nested ifs are required
#![allow(clippy::collapsible_if)]

use std::path::Path;

use biscuit_speaks::{SpeedLevel, TtsConfig, TtsFailoverStrategy};
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::compose::expression::{Expr, ExpressionFinder, parse};
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::error::CompositionError;
use crate::events::{GlobalSettings, TtsSettings};
use crate::messaging::RuntimeMessagingSettings;

/// A single lifecycle notification configuration.
///
/// ## Examples
///
/// ```yaml
/// start:
///   say: "Starting composition workflow"
///   effect: "confirmation"
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LifecycleNotification {
    /// Text to say using TTS (mutually exclusive with `say_first`).
    pub say: Option<String>,

    /// Text to say before other actions (mutually exclusive with `say`).
    pub say_first: Option<String>,

    /// Sound effect to play (kebab-case name like "confirmation").
    pub effect: Option<String>,

    /// Message to display in the terminal.
    pub message: Option<String>,

    /// Message to write to stderr.
    pub stderr: Option<String>,

    /// Title for a local desktop notification.
    pub notify: Option<String>,
}

/// Complete lifecycle configuration for a composition.
///
/// Parsed from frontmatter properties: `start`, `success`, `blocked`, `failure`.
///
/// ## Examples
///
/// ```yaml
/// start:
///   message: "Starting..."
/// success:
///   say: "Composition complete"
///   effect: "crowd-applause"
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LifecycleConfig {
    /// Notification emitted when composition begins.
    pub start: Option<LifecycleNotification>,

    /// Notification emitted when composition succeeds.
    pub success: Option<LifecycleNotification>,

    /// Notification emitted when composition is blocked.
    pub blocked: Option<LifecycleNotification>,

    /// Notification emitted when composition fails.
    pub failure: Option<LifecycleNotification>,
}

/// Lifecycle event signal types.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LifecycleSignal {
    /// Composition is starting.
    Start,

    /// Composition completed successfully.
    Success,

    /// Composition is blocked (waiting for user input, etc.).
    Blocked,

    /// Composition failed with an error.
    Failure,
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

/// Trait for emitting lifecycle notification side effects.
///
/// Injectable to allow test doubles that capture emissions without hitting
/// real stderr, messaging, TTS, or sound playback.
pub trait LifecycleEmitter {
    /// Write a styled status line to stderr.
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
}

/// Production emitter that performs real side effects.
pub struct DefaultLifecycleEmitter;

impl LifecycleEmitter for DefaultLifecycleEmitter {
    fn emit_stderr(&self, signal: LifecycleSignal, text: &str, term: &Terminal) {
        let rendered = Status::from_prose(text)
            .state(signal.status_state())
            .theme(StatusTheme::Circular)
            .render(term);
        eprintln!("{rendered}");
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
        say_blocking(text, tts_config);
    }

    fn emit_effect(&self, name: &str) {
        play_effect_blocking(name);
    }

    fn emit_notification(&self, title: &str) {
        crate::messaging::execute_notification(title, None);
    }
}

// ---------------------------------------------------------------------------
// LifecycleRunGuard
// ---------------------------------------------------------------------------

/// RAII guard that centralizes lifecycle state transitions and guarantees a
/// terminal signal (`blocked` or `failure`) is emitted when a run exits
/// after `start` without an explicit terminal signal.
///
/// ## Drop behaviour
///
/// | `start_emitted` | `provider_launched` | Drop emits |
/// |-----------------|---------------------|------------|
/// | `false`         | —                   | nothing    |
/// | `true`          | `false`             | `Blocked`  |
/// | `true`          | `true`              | `Failure`  |
///
/// Explicit calls to [`emit_terminal`](Self::emit_terminal),
/// [`emit_blocked_or_failure`](Self::emit_blocked_or_failure), or
/// [`defuse`](Self::defuse) suppress the Drop emission.
pub struct LifecycleRunGuard<'a> {
    config: &'a LifecycleConfig,
    ctx: &'a LifecycleRuntimeContext<'a>,
    emitter: &'a dyn LifecycleEmitter,
    start_emitted: bool,
    provider_launched: bool,
    terminal_emitted: bool,
}

impl<'a> LifecycleRunGuard<'a> {
    /// Create a new guard.
    pub fn new(
        config: &'a LifecycleConfig,
        ctx: &'a LifecycleRuntimeContext<'a>,
        emitter: &'a dyn LifecycleEmitter,
    ) -> Self {
        Self {
            config,
            ctx,
            emitter,
            start_emitted: false,
            provider_launched: false,
            terminal_emitted: false,
        }
    }

    /// Emit the `Start` signal (idempotent — only emits on the first call).
    pub fn emit_start_once(&mut self) {
        if !self.start_emitted {
            self.emit_signal(LifecycleSignal::Start);
            self.start_emitted = true;
        }
    }

    /// Record that the provider child process has actually been spawned.
    ///
    /// Call this **after** `execute_harness_attempt` returns `Ok`, not before.
    pub fn mark_provider_launched(&mut self) {
        self.provider_launched = true;
    }

    /// Emit a specific terminal signal and suppress the Drop safety-net.
    pub fn emit_terminal(&mut self, signal: LifecycleSignal) {
        self.emit_signal(signal);
        self.terminal_emitted = true;
    }

    /// Emit `Blocked` (pre-launch) or `Failure` (post-launch) and return
    /// the original error unchanged. Convenient for wrapping `?` returns.
    pub fn emit_blocked_or_err<E>(&mut self, err: E) -> E {
        self.emit_blocked_or_failure();
        err
    }

    /// Emit `Blocked` (pre-launch) or `Failure` (post-launch) and suppress
    /// the Drop safety-net.
    pub fn emit_blocked_or_failure(&mut self) {
        let signal = if self.provider_launched {
            LifecycleSignal::Failure
        } else {
            LifecycleSignal::Blocked
        };
        self.emit_signal(signal);
        self.terminal_emitted = true;
    }

    /// Suppress the Drop emission without emitting any signal.
    ///
    /// Use when transferring lifecycle responsibility elsewhere.
    pub fn defuse(&mut self) {
        self.terminal_emitted = true;
    }

    pub fn start_emitted(&self) -> bool {
        self.start_emitted
    }

    pub fn provider_launched(&self) -> bool {
        self.provider_launched
    }

    /// Emit a single lifecycle signal through the injected emitter.
    ///
    /// Short-circuits on `crate::interrupt::interrupted()` so a Ctrl+C
    /// during execution skips all blocking post-execute side effects
    /// (messenger sends, desktop notifications, TTS playback, sound
    /// effects). The cheap stderr line still emits so the user sees the
    /// terminal status before the process exits.
    fn emit_signal(&self, signal: LifecycleSignal) {
        let Some(notification) = self.config.get(signal) else {
            return;
        };

        let interrupted = crate::interrupt::interrupted();

        // --- Non-audio fan-out (immediate) ---
        if let Some(stderr_text) = &notification.stderr {
            self.emitter.emit_stderr(signal, stderr_text, self.ctx.term);
        }
        if interrupted {
            return;
        }
        if let Some(message_text) = &notification.message {
            self.emitter.emit_message(
                message_text,
                self.ctx.source_path,
                self.ctx.repo_root,
                self.ctx.messaging,
            );
        }
        if let Some(notify_title) = &notification.notify {
            self.emitter.emit_notification(notify_title);
        }

        // --- Audio phases (sequential, blocking, lazy TTS config) ---
        let phases = audio_phases(notification);
        let mut tts_config: Option<TtsConfig> = None;

        for phase in phases {
            if crate::interrupt::interrupted() {
                return;
            }
            match phase {
                AudioPhase::Speak(text) => {
                    let config = tts_config
                        .get_or_insert_with(|| {
                            tts_config_from_settings(self.ctx.settings.tts.as_ref())
                        })
                        .clone();
                    self.emitter.emit_speech(&text, config);
                }
                AudioPhase::Effect(name) => self.emitter.emit_effect(&name),
            }
        }
    }
}

impl Drop for LifecycleRunGuard<'_> {
    fn drop(&mut self) {
        if self.start_emitted && !self.terminal_emitted {
            let signal = if self.provider_launched {
                LifecycleSignal::Failure
            } else {
                LifecycleSignal::Blocked
            };
            self.emit_signal(signal);
        }
    }
}

/// Runtime context required for emitting lifecycle notifications.
///
/// Holds references to settings, messaging configuration, terminal, and paths
/// needed to resolve and emit lifecycle notifications.
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
}

/// A single audio playback phase.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
enum AudioPhase {
    Speak(String),
    Effect(String),
}

/// Compute the ordered audio phases for a notification.
///
/// When both speech and effect are present:
/// - `say` + `effect` → effect first, then speech
/// - `say_first` + `effect` → speech first, then effect
///
/// When only one audio output is present, it is the sole phase.
fn audio_phases(n: &LifecycleNotification) -> Vec<AudioPhase> {
    let speech_text = n
        .say
        .as_deref()
        .or(n.say_first.as_deref())
        .filter(|s| !s.is_empty());
    let effect_name = n.effect.as_deref().filter(|s| !s.is_empty());
    let speech_first = n.say_first.is_some();

    match (speech_text, effect_name) {
        (Some(text), Some(effect)) if speech_first => {
            vec![
                AudioPhase::Speak(text.to_string()),
                AudioPhase::Effect(effect.to_string()),
            ]
        }
        (Some(text), Some(effect)) => {
            vec![
                AudioPhase::Effect(effect.to_string()),
                AudioPhase::Speak(text.to_string()),
            ]
        }
        (Some(text), None) => vec![AudioPhase::Speak(text.to_string())],
        (None, Some(effect)) => vec![AudioPhase::Effect(effect.to_string())],
        (None, None) => vec![],
    }
}

impl LifecycleSignal {
    /// Returns the frontmatter property name for this signal.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::LifecycleSignal;
    /// assert_eq!(LifecycleSignal::Start.property_name(), "start");
    /// assert_eq!(LifecycleSignal::Success.property_name(), "success");
    /// ```
    pub fn property_name(&self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Success => "success",
            Self::Blocked => "blocked",
            Self::Failure => "failure",
        }
    }

    /// Returns the status state for this signal.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::LifecycleSignal;
    /// # use biscuit_terminal::components::status::StatusState;
    /// assert_eq!(LifecycleSignal::Start.status_state(), StatusState::Info);
    /// assert_eq!(LifecycleSignal::Success.status_state(), StatusState::Success);
    /// assert_eq!(LifecycleSignal::Blocked.status_state(), StatusState::Error);
    /// assert_eq!(LifecycleSignal::Failure.status_state(), StatusState::Error);
    /// ```
    pub fn status_state(&self) -> StatusState {
        match self {
            Self::Start => StatusState::Info,
            Self::Success => StatusState::Success,
            Self::Blocked | Self::Failure => StatusState::Error,
        }
    }
}

impl LifecycleConfig {
    /// Returns the notification for a given signal, if configured.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::{LifecycleConfig, LifecycleSignal};
    /// let config = LifecycleConfig::default();
    /// assert!(config.get(LifecycleSignal::Start).is_none());
    /// ```
    pub fn get(&self, signal: LifecycleSignal) -> Option<&LifecycleNotification> {
        match signal {
            LifecycleSignal::Start => self.start.as_ref(),
            LifecycleSignal::Success => self.success.as_ref(),
            LifecycleSignal::Blocked => self.blocked.as_ref(),
            LifecycleSignal::Failure => self.failure.as_ref(),
        }
    }

    /// Returns `true` if no lifecycle notifications are configured.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::LifecycleConfig;
    /// let config = LifecycleConfig::default();
    /// assert!(config.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.start.is_none()
            && self.success.is_none()
            && self.blocked.is_none()
            && self.failure.is_none()
    }
}

/// Parses lifecycle configuration from composition frontmatter.
///
/// Extracts only the lifecycle properties (`start`, `success`, `blocked`, `failure`)
/// and ignores all other frontmatter keys. Validates mutual exclusivity of `say`
/// and `say_first`, and validates sound effect names.
///
/// ## Returns
///
/// Returns `Ok(LifecycleConfig)` on success, or a `CompositionError` if validation fails.
///
/// ## Errors
///
/// - `LifecycleSayConflict`: Both `say` and `say_first` are present
/// - `LifecycleUnknownEffect`: An unknown sound effect name is referenced
///
/// ## Examples
///
/// ```
/// # use serde_json::json;
/// # use claudine::composition::parse_lifecycle_config;
/// let frontmatter = json!({
///     "title": "My Composition",
///     "start": {
///         "message": "Starting..."
///     }
/// });
/// let config = parse_lifecycle_config(&frontmatter, std::path::Path::new("test.md")).unwrap();
/// assert!(config.start.is_some());
/// ```
pub fn parse_lifecycle_config(
    frontmatter: &serde_json::Value,
    source_file: &Path,
) -> Result<LifecycleConfig, CompositionError> {
    // Non-object frontmatter returns default
    let Some(fm_obj) = frontmatter.as_object() else {
        return Ok(LifecycleConfig::default());
    };

    let mut config = LifecycleConfig::default();

    // Process each lifecycle property
    for (property_name, field_ref) in [
        ("start", &mut config.start),
        ("success", &mut config.success),
        ("blocked", &mut config.blocked),
        ("failure", &mut config.failure),
    ] {
        let Some(value) = fm_obj.get(property_name) else {
            continue;
        };

        // Skip null values
        if value.is_null() {
            continue;
        }

        // Deserialize the notification
        let mut notification: LifecycleNotification = serde_json::from_value(value.clone())
            .map_err(|e| {
                let (unknown_field, expected_fields) = parse_serde_unknown_field(&e);
                CompositionError::LifecycleInvalid {
                    property: property_name.to_string(),
                    message: e.to_string(),
                    source_file: source_file.to_path_buf(),
                    unknown_field,
                    expected_fields,
                }
            })?;

        // Normalize empty strings to None
        normalize_empty_string(&mut notification.say);
        normalize_empty_string(&mut notification.say_first);
        normalize_empty_string(&mut notification.effect);
        normalize_empty_string(&mut notification.message);
        normalize_empty_string(&mut notification.stderr);
        normalize_empty_string(&mut notification.notify);

        // Validate mutual exclusivity of say and say_first
        if notification.say.is_some() && notification.say_first.is_some() {
            return Err(CompositionError::LifecycleSayConflict(
                property_name.to_string(),
            ));
        }

        // Validate effect name if present
        if let Some(effect_name) = &notification.effect {
            if playa::SoundEffect::from_name(effect_name).is_none() {
                return Err(CompositionError::LifecycleUnknownEffect(
                    property_name.to_string(),
                    effect_name.clone(),
                ));
            }
        }

        *field_ref = Some(notification);
    }

    Ok(config)
}

/// Validates that no rendered lifecycle string contains a surviving
/// `{{ … }}` interpolation span.
///
/// Iterates every configured signal in deterministic order
/// (`Start`, `Success`, `Blocked`, `Failure`) and every string field on the
/// notification in deterministic order (`say`, `say_first`, `message`,
/// `stderr`, `notify`). The first field with a non-empty span list aborts
/// with [`CompositionError::LifecycleInterpolationLeak`].
///
/// This runs **after** composition, when expressions should have resolved.
/// It intentionally does *not* run inside [`parse_lifecycle_config`], which
/// is also used for raw frontmatter inspection where unresolved templates
/// are legitimate.
///
/// ## Arguments
///
/// * `config` — parsed lifecycle configuration.
/// * `source_path` — composed prompt file, used for the diagnostic.
/// * `warnings` — compose report warnings, used best-effort to enrich the
///   leak reason.
pub fn validate_no_interpolation_leaks(
    config: &LifecycleConfig,
    source_path: &Path,
    warnings: &[darkmatter::markdown::compose::ComposeWarning],
) -> Result<(), CompositionError> {
    let signals = [
        LifecycleSignal::Start,
        LifecycleSignal::Success,
        LifecycleSignal::Blocked,
        LifecycleSignal::Failure,
    ];

    for signal in signals {
        let Some(notification) = config.get(signal) else {
            continue;
        };

        let fields: [(&str, Option<&String>); 5] = [
            ("say", notification.say.as_ref()),
            ("say_first", notification.say_first.as_ref()),
            ("message", notification.message.as_ref()),
            ("stderr", notification.stderr.as_ref()),
            ("notify", notification.notify.as_ref()),
        ];

        for (field_name, value) in fields {
            let Some(text) = value else {
                continue;
            };
            if text.is_empty() {
                continue;
            }

            let spans = ExpressionFinder::find_all_plain(text);
            if let Some(first) = spans.first() {
                let property = format!("{}.{}", signal.property_name(), field_name);
                let expression = first.expression.clone();
                let reason = find_matching_warning_reason(&expression, warnings);
                return Err(CompositionError::LifecycleInterpolationLeak {
                    source_path: source_path.to_path_buf(),
                    property,
                    expression,
                    reason,
                });
            }
        }
    }

    Ok(())
}

/// Best-effort extraction of a warning reason mentioning the leaked expression.
fn find_matching_warning_reason(
    expression: &str,
    warnings: &[darkmatter::markdown::compose::ComposeWarning],
) -> String {
    let inner = expression
        .trim_start_matches("{{")
        .trim_end_matches("}}")
        .trim();

    for warning in warnings {
        if warning.message.contains(expression) || warning.message.contains(inner) {
            return warning.message.clone();
        }
    }

    String::new()
}

/// Validates that no raw lifecycle string references a bare variable that is
/// undefined after composition.
///
/// Darkmatter resolves an unknown bare variable to an empty string with no
/// warning and no error — even in fail-fast mode (see
/// `frontmatter_interpolation::missing_variable_resolves_to_empty`). So the
/// post-compose [`validate_no_interpolation_leaks`] guard, which only scans
/// the *rendered* string for surviving spans, never sees the collapsed
/// reference. This guard closes that gap by inspecting the **raw**
/// (pre-composition) lifecycle strings, where the `{{ … }}` span is still
/// present, and resolving each bare variable against the composed frontmatter.
///
/// Every bare variable reachable in the parsed expression tree is checked, not
/// just spans that are exactly `{{ variable }}`: a missing operand buried in a
/// function argument (`{{ parent_dir(missing) }}`), comparison, or arithmetic
/// node is rejected the same way a top-level `{{ missing }}` is. A ternary
/// condition (`{{ missing ? 'a' : 'b' }}`) is descended because it is evaluated,
/// but the ternary branch operands and fallback (`{{ x || 'y' }}`) subtrees
/// intentionally tolerate undefined operands, so they are skipped. `ctx.*` /
/// `env.*` / `doc` references resolve from outside the frontmatter and are
/// skipped — a bare name resolves only against top-level frontmatter keys.
///
/// Iterates signals (`Start`, `Success`, `Blocked`, `Failure`) and fields
/// (`say`, `say_first`, `message`, `stderr`, `notify`) in the same
/// deterministic order as [`validate_no_interpolation_leaks`]; the first
/// undefined variable aborts with
/// [`CompositionError::LifecycleUndefinedVariable`].
///
/// ## Arguments
///
/// * `raw_frontmatter` — the pre-composition frontmatter holding the original
///   lifecycle strings (`{{ … }}` spans intact).
/// * `effective_frontmatter` — the composed frontmatter object; a bare
///   variable is "defined" when its root segment is one of these keys.
/// * `source_path` — prompt file, used for the diagnostic.
pub fn validate_no_undefined_lifecycle_variables(
    raw_frontmatter: &darkmatter::markdown::Frontmatter,
    effective_frontmatter: &serde_json::Value,
    source_path: &Path,
) -> Result<(), CompositionError> {
    let signals = [
        LifecycleSignal::Start,
        LifecycleSignal::Success,
        LifecycleSignal::Blocked,
        LifecycleSignal::Failure,
    ];
    let fields = ["say", "say_first", "message", "stderr", "notify"];

    let raw_map = raw_frontmatter.as_map();
    let defined = effective_frontmatter.as_object();

    for signal in signals {
        let Some(serde_json::Value::Object(notification)) = raw_map.get(signal.property_name())
        else {
            continue;
        };

        for field in fields {
            let Some(serde_json::Value::String(text)) = notification.get(field) else {
                continue;
            };

            for span in ExpressionFinder::find_all_plain(text) {
                let Ok(expr) = parse(&span.expression) else {
                    continue;
                };
                if let Some(variable) = find_undefined_variable(&expr, defined) {
                    return Err(CompositionError::LifecycleUndefinedVariable {
                        source_path: source_path.to_path_buf(),
                        property: format!("{}.{}", signal.property_name(), field),
                        variable: variable.to_string(),
                    });
                }
            }
        }
    }

    Ok(())
}

/// Recursively walks `expr`, returning the first frontmatter-scoped bare
/// variable whose root key is undefined in the composed frontmatter.
///
/// A ternary condition is descended because it is evaluated during composition,
/// but the ternary branch operands and fallback (`||`) subtrees are not: those
/// forms exist precisely to tolerate an undefined operand, so a miss inside them
/// is intentional, not a leak. Every other node — function-call arguments,
/// comparisons, arithmetic, indexing, member access, unary, parens — is
/// descended so an undefined variable buried in `parent_dir(missing)` is caught
/// like a top-level `{{ missing }}`. The returned reference borrows from `expr`.
fn find_undefined_variable<'a>(
    expr: &'a Expr,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    match expr {
        Expr::Variable(path) => undefined_bare_variable(path, defined),
        // Ternary conditions are evaluated, but the branches intentionally
        // tolerate undefined operands by design.
        Expr::Ternary { condition, .. } => find_undefined_variable(condition, defined),
        Expr::Fallback { .. } => None,
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => None,
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            find_undefined_variable(inner, defined)
        }
        Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } => {
            find_undefined_variable(left, defined)
                .or_else(|| find_undefined_variable(right, defined))
        }
        Expr::Index { base, index } => find_undefined_variable(base, defined)
            .or_else(|| find_undefined_variable(index, defined)),
        Expr::MemberAccess { base, .. } => find_undefined_variable(base, defined),
        Expr::FunctionCall { args, .. } => args
            .iter()
            .find_map(|arg| find_undefined_variable(arg, defined)),
    }
}

/// Returns the bare variable name when `path` is a frontmatter-scoped reference
/// whose root segment is absent from the composed frontmatter, or `None` when
/// it resolves elsewhere (`ctx.*` / `env.*` / `doc`) or its root key exists.
///
/// Nested misses (`{{ a.b }}` where `a` exists but `b` does not) are treated as
/// defined: only the bare-root contract the spec describes is enforced.
fn undefined_bare_variable<'a>(
    path: &'a str,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    if path.starts_with("ctx.")
        || path.starts_with("env.")
        || path == "doc"
        || path.starts_with("doc.")
    {
        return None;
    }
    let root = path.split('.').next().unwrap_or(path);
    if root.is_empty() {
        return None;
    }
    match defined {
        Some(map) if map.contains_key(root) => None,
        _ => Some(root),
    }
}

/// Normalizes empty or whitespace-only strings to `None`.
fn normalize_empty_string(field: &mut Option<String>) {
    if let Some(s) = field {
        if s.trim().is_empty() {
            *field = None;
        }
    }
}

/// Parse a `serde_json` "unknown field" error to extract the field name
/// and the list of expected fields.
///
/// Serde's message format is:
/// `unknown field `X`, expected one of `A`, `B`, `C``
///
/// Returns `(Some("X"), vec!["A", "B", "C"])` on match, or
/// `(None, LIFECYCLE_NOTIFICATION_FIELDS)` as a fallback.
fn parse_serde_unknown_field(err: &serde_json::Error) -> (Option<String>, Vec<String>) {
    let msg = err.to_string();

    // Extract unknown field name between first pair of backticks.
    let unknown_field = extract_backtick_value(&msg, 0);

    // Extract expected fields from "expected one of `A`, `B`, `C`"
    // or "expected `A`" (single field).
    let expected = if let Some(idx) = msg.find("expected") {
        collect_backtick_values(&msg[idx..])
    } else {
        LIFECYCLE_NOTIFICATION_FIELDS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    };

    (unknown_field, expected)
}

/// The canonical field names for [`LifecycleNotification`].
pub(crate) const LIFECYCLE_NOTIFICATION_FIELDS: &[&str] =
    &["say", "say_first", "effect", "message", "stderr", "notify"];

/// Extract the text inside the `n`th pair of backticks in `s`.
fn extract_backtick_value(s: &str, nth: usize) -> Option<String> {
    let mut start = 0;
    for i in 0..=nth {
        let open = s[start..].find('`')?;
        let abs_open = start + open;
        let close = s[abs_open + 1..].find('`')?;
        if i == nth {
            return Some(s[abs_open + 1..abs_open + 1 + close].to_string());
        }
        start = abs_open + 1 + close + 1;
    }
    None
}

/// Collect all backtick-delimited values from `s`.
fn collect_backtick_values(s: &str) -> Vec<String> {
    let mut vals = Vec::new();
    let mut chars = s.char_indices().peekable();
    while let Some(&(i, ch)) = chars.peek() {
        if ch == '`' {
            chars.next();
            let start = i + 1;
            let mut end = start;
            for (j, c) in chars.by_ref() {
                if c == '`' {
                    end = j;
                    break;
                }
            }
            if end > start {
                vals.push(s[start..end].to_string());
            }
        } else {
            chars.next();
        }
    }
    if vals.is_empty() {
        LIFECYCLE_NOTIFICATION_FIELDS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    } else {
        vals
    }
}

/// Build a `TtsConfig` from global settings.
fn tts_config_from_settings(tts: Option<&TtsSettings>) -> TtsConfig {
    let mut config = TtsConfig::new();
    let Some(settings) = tts else {
        return config;
    };

    if let Some(voice) = settings.voice.as_deref() {
        config = config.with_voice(voice);
    }
    if let Some(rate) = settings.rate {
        config = config.with_speed(SpeedLevel::Explicit(rate));
    }
    if let Some(provider) = settings.provider.as_deref() {
        if let Some(provider) = biscuit_speaks::parse_provider_name(provider) {
            config = config.with_failover(TtsFailoverStrategy::SpecificProvider(provider));
        } else {
            warn!(
                provider,
                "Unknown TTS provider in settings; using automatic selection"
            );
        }
    }
    config
}

/// Maximum wall-clock time a single lifecycle TTS playback may block the
/// composition thread before it is abandoned.
///
/// A wedged TTS provider (a stalled network voice, a contended audio device,
/// a hung `say` subprocess) must never freeze a compose run — the danger is
/// acute *between* loop iterations, where no child wait loop is installed and
/// the Ctrl+C interrupt flag cannot reach a synchronous call. Generous enough
/// for a long sentence, short enough that a hang can't wedge the run.
const TTS_PLAYBACK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Maximum wall-clock time a single lifecycle sound effect may block.
const EFFECT_PLAYBACK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// Run a blocking side effect on a detached worker thread, bounding how long
/// the caller waits for it.
///
/// Returns once the work finishes or `timeout` elapses, whichever comes first.
/// On timeout the worker is detached (it may keep running harmlessly in the
/// background) and a warning is logged. This is the lifecycle analogue of the
/// wrapper's `join_with_timeout`: the only safe way to bound an arbitrary
/// blocking call (subprocess wait, audio device, network voice) from outside
/// is to stop waiting on it.
fn run_blocking_with_timeout<F>(label: &'static str, timeout: std::time::Duration, work: F)
where
    F: FnOnce() + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        work();
        // A closed receiver (timed-out caller already moved on) is expected;
        // the send is best-effort.
        let _ = tx.send(());
    });
    match rx.recv_timeout(timeout) {
        Ok(()) => {}
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            warn!(
                label,
                ?timeout,
                "lifecycle side effect exceeded its time budget; detaching and continuing"
            );
        }
        // The worker dropped its sender without signaling (e.g. it panicked).
        // There is nothing left to wait for, and no timeout was breached.
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {}
    }
}

/// Play a sound effect synchronously (blocking), bounded by
/// [`EFFECT_PLAYBACK_TIMEOUT`].
fn play_effect_blocking(name: &str) {
    let name = name.to_string();
    run_blocking_with_timeout("effect", EFFECT_PLAYBACK_TIMEOUT, move || {
        let Some(effect) = playa::SoundEffect::from_name(&name) else {
            warn!(%name, "Unknown sound effect in lifecycle notification");
            return;
        };
        match playa::Playa::from_bytes(effect.bytes().to_vec()) {
            Ok(player) => {
                if let Err(e) = player.play() {
                    warn!(%e, "Lifecycle sound effect playback failed");
                }
            }
            Err(e) => warn!(%e, "Failed to construct sound effect player"),
        }
    });
}

/// Speak text using the Tokio runtime, bounded by [`TTS_PLAYBACK_TIMEOUT`].
///
/// The playback runs on a detached worker thread that drives the async
/// `play()` future via the current runtime's `Handle` (cloned in before the
/// thread is spawned, since `Handle::block_on` works from any thread). This
/// both bounds the wait and avoids `block_in_place`, which would panic off a
/// runtime worker thread.
fn say_blocking(text: &str, config: TtsConfig) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        warn!("No Tokio runtime available for lifecycle TTS");
        return;
    };
    let text = text.to_string();
    run_blocking_with_timeout("tts", TTS_PLAYBACK_TIMEOUT, move || {
        handle.block_on(async move {
            if let Err(e) = biscuit_speaks::Speak::new(text)
                .with_config(config)
                .play()
                .await
            {
                warn!(%e, "Lifecycle TTS playback failed");
            }
        });
    });
}

/// Emit a lifecycle signal with deterministic audio ordering.
///
/// Dispatches non-audio targets (stderr, message) immediately, then
/// plays audio phases in order. All errors are logged as warnings and
/// never propagated.
///
/// Prefer [`LifecycleRunGuard`] for new code — it uses this same
/// emission logic but adds mechanical state-transition enforcement.
#[deprecated(
    since = "0.1.0",
    note = "use LifecycleRunGuard instead, which enforces state transitions and emits signals via the LifecycleEmitter trait"
)]
pub fn emit_lifecycle_signal(
    config: &LifecycleConfig,
    signal: LifecycleSignal,
    ctx: &LifecycleRuntimeContext<'_>,
) {
    let Some(notification) = config.get(signal) else {
        return;
    };

    // --- Non-audio fan-out (immediate) ---

    // stderr
    if let Some(stderr_text) = &notification.stderr {
        let rendered = Status::from_prose(stderr_text)
            .state(signal.status_state())
            .theme(StatusTheme::Circular)
            .render(ctx.term);
        eprintln!("{rendered}");
    }

    // message
    if let Some(message_text) = &notification.message {
        crate::messaging::execute_resolved_message(
            message_text,
            None,
            Some(ctx.source_path),
            ctx.repo_root,
            ctx.messaging,
        );
    }
    // notify
    if let Some(notify_title) = &notification.notify {
        crate::messaging::execute_notification(notify_title, None);
    }

    // --- Audio phases (sequential, blocking, lazy TTS config) ---

    let phases = audio_phases(notification);
    let mut tts_config: Option<TtsConfig> = None;

    for phase in phases {
        match phase {
            AudioPhase::Speak(text) => {
                let config = tts_config
                    .get_or_insert_with(|| tts_config_from_settings(ctx.settings.tts.as_ref()))
                    .clone();
                say_blocking(&text, config);
            }
            AudioPhase::Effect(name) => play_effect_blocking(&name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dummy_path() -> &'static Path {
        Path::new("test.md")
    }

    /// A blocking lifecycle side effect that wedges (never returns) must not
    /// be able to freeze the composition thread: `run_blocking_with_timeout`
    /// has to return after roughly its budget, not after the work finishes.
    /// This is the core of fix #1 — a hung TTS / sound provider between loop
    /// iterations used to lock the run with no way for Ctrl+C to break in.
    #[test]
    fn run_blocking_with_timeout_returns_when_work_hangs() {
        use std::time::{Duration, Instant};

        let start = Instant::now();
        run_blocking_with_timeout("test-hang", Duration::from_millis(100), || {
            // Simulate a wedged audio device / network voice.
            std::thread::sleep(Duration::from_secs(30));
        });
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_secs(5),
            "must abandon the wedged side effect near the 100ms budget, \
             not wait out the 30s sleep; took {elapsed:?}"
        );
    }

    /// The happy path must still run the work to completion and return its
    /// result — bounding the wait must not turn into fire-and-forget for work
    /// that finishes within budget.
    #[test]
    fn run_blocking_with_timeout_runs_work_to_completion_within_budget() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::Duration;

        let done = Arc::new(AtomicBool::new(false));
        let done_clone = Arc::clone(&done);
        run_blocking_with_timeout("test-quick", Duration::from_secs(5), move || {
            std::thread::sleep(Duration::from_millis(20));
            done_clone.store(true, Ordering::SeqCst);
        });

        assert!(
            done.load(Ordering::SeqCst),
            "work that finishes within budget must complete before the call returns"
        );
    }

    #[test]
    fn parses_valid_lifecycle_config() {
        let frontmatter = json!({
            "start": {
                "message": "Starting composition..."
            },
            "success": {
                "say": "All done!",
                "effect": "confirmation"
            }
        });

        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        assert!(config.start.is_some());
        assert_eq!(
            config.start.as_ref().unwrap().message.as_deref(),
            Some("Starting composition...")
        );

        assert!(config.success.is_some());
        let success = config.success.as_ref().unwrap();
        assert_eq!(success.say.as_deref(), Some("All done!"));
        assert_eq!(success.effect.as_deref(), Some("confirmation"));
    }

    #[test]
    fn rejects_both_say_and_say_first() {
        let frontmatter = json!({
            "start": {
                "say": "Starting",
                "say_first": "Also starting"
            }
        });

        let result = parse_lifecycle_config(&frontmatter, dummy_path());
        assert!(matches!(
            result,
            Err(CompositionError::LifecycleSayConflict(_))
        ));
    }

    #[test]
    fn trims_empty_strings_to_none() {
        let frontmatter = json!({
            "start": {
                "message": "   ",
                "say": ""
            }
        });

        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        let start = config.start.as_ref().unwrap();
        assert!(start.message.is_none());
        assert!(start.say.is_none());
    }

    #[test]
    fn rejects_unknown_keys() {
        let frontmatter = json!({
            "start": {
                "message": "Starting",
                "unknown_field": "value"
            }
        });

        let result = parse_lifecycle_config(&frontmatter, dummy_path());
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_effect_name() {
        let frontmatter = json!({
            "start": {
                "effect": "nonexistent-effect"
            }
        });

        let result = parse_lifecycle_config(&frontmatter, dummy_path());
        assert!(matches!(
            result,
            Err(CompositionError::LifecycleUnknownEffect(_, _))
        ));
    }

    #[test]
    fn say_plus_effect_is_valid() {
        let frontmatter = json!({
            "success": {
                "say": "Done!",
                "effect": "confirmation"
            }
        });

        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        let success = config.success.as_ref().unwrap();
        assert_eq!(success.say.as_deref(), Some("Done!"));
        assert_eq!(success.effect.as_deref(), Some("confirmation"));
    }

    #[test]
    fn say_first_plus_effect_is_valid() {
        let frontmatter = json!({
            "success": {
                "say_first": "Starting now",
                "effect": "confirmation"
            }
        });

        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        let success = config.success.as_ref().unwrap();
        assert_eq!(success.say_first.as_deref(), Some("Starting now"));
        assert_eq!(success.effect.as_deref(), Some("confirmation"));
    }

    #[test]
    fn empty_frontmatter_returns_default() {
        let frontmatter = json!({});
        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn non_object_frontmatter_returns_default() {
        let frontmatter = json!("not an object");
        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn null_lifecycle_property_is_skipped() {
        let frontmatter = json!({
            "start": null,
            "success": {
                "message": "Done"
            }
        });

        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        assert!(config.start.is_none());
        assert!(config.success.is_some());
    }

    #[test]
    fn frontmatter_with_non_lifecycle_keys_is_fine() {
        let frontmatter = json!({
            "title": "My Composition",
            "agent": "claude",
            "start": {
                "message": "Starting"
            }
        });

        let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
        assert!(config.start.is_some());
    }

    #[test]
    fn audio_order_say_plus_effect() {
        let n = LifecycleNotification {
            say: Some("Hello".into()),
            effect: Some("doorbell".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 2);
        assert!(matches!(phases[0], AudioPhase::Effect(_)));
        assert!(matches!(phases[1], AudioPhase::Speak(_)));
    }

    #[test]
    fn audio_order_say_first_plus_effect() {
        let n = LifecycleNotification {
            say_first: Some("Hello".into()),
            effect: Some("doorbell".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 2);
        assert!(matches!(phases[0], AudioPhase::Speak(_)));
        assert!(matches!(phases[1], AudioPhase::Effect(_)));
    }

    #[test]
    fn audio_order_speech_only() {
        let n = LifecycleNotification {
            say: Some("Hello".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 1);
        assert!(matches!(phases[0], AudioPhase::Speak(_)));
    }

    #[test]
    fn audio_order_effect_only() {
        let n = LifecycleNotification {
            effect: Some("doorbell".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 1);
        assert!(matches!(phases[0], AudioPhase::Effect(_)));
    }

    #[test]
    fn audio_order_no_audio() {
        let n = LifecycleNotification {
            stderr: Some("Status only".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert!(phases.is_empty());
    }

    #[test]
    fn status_state_mapping() {
        assert_eq!(LifecycleSignal::Start.status_state(), StatusState::Info);
        assert_eq!(
            LifecycleSignal::Success.status_state(),
            StatusState::Success
        );
        assert_eq!(LifecycleSignal::Blocked.status_state(), StatusState::Error);
        assert_eq!(LifecycleSignal::Failure.status_state(), StatusState::Error);
    }

    #[test]
    fn property_names() {
        assert_eq!(LifecycleSignal::Start.property_name(), "start");
        assert_eq!(LifecycleSignal::Success.property_name(), "success");
        assert_eq!(LifecycleSignal::Blocked.property_name(), "blocked");
        assert_eq!(LifecycleSignal::Failure.property_name(), "failure");
    }

    #[test]
    fn lifecycle_config_get() {
        let fm = json!({
            "start": { "stderr": "Starting" },
            "failure": { "stderr": "Failed" }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(config.get(LifecycleSignal::Start).is_some());
        assert!(config.get(LifecycleSignal::Success).is_none());
        assert!(config.get(LifecycleSignal::Blocked).is_none());
        assert!(config.get(LifecycleSignal::Failure).is_some());
    }

    #[test]
    fn lifecycle_config_is_empty() {
        let empty = LifecycleConfig::default();
        assert!(empty.is_empty());

        let fm = json!({ "start": { "stderr": "Go" } });
        let non_empty = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(!non_empty.is_empty());
    }

    #[test]
    #[allow(deprecated)]
    fn lifecycle_runtime_state_defaults() {
        let state = LifecycleRuntimeState::default();
        assert!(!state.start_emitted);
        assert!(!state.provider_launch_started);
    }

    // -- RecordingEmitter + LifecycleRunGuard tests -------------------------

    use std::sync::Mutex;

    #[derive(Debug, Clone, PartialEq)]
    enum EmittedAction {
        Stderr {
            signal: LifecycleSignal,
            text: String,
        },
        Message {
            text: String,
        },
        Notification {
            title: String,
        },
        Speech {
            text: String,
        },
        Effect {
            name: String,
        },
    }

    struct RecordingEmitter {
        actions: Mutex<Vec<EmittedAction>>,
    }

    impl RecordingEmitter {
        fn new() -> Self {
            Self {
                actions: Mutex::new(Vec::new()),
            }
        }

        fn actions(&self) -> Vec<EmittedAction> {
            self.actions.lock().unwrap().clone()
        }

        fn signals(&self) -> Vec<LifecycleSignal> {
            self.actions
                .lock()
                .unwrap()
                .iter()
                .filter_map(|a| match a {
                    EmittedAction::Stderr { signal, .. } => Some(*signal),
                    _ => None,
                })
                .collect()
        }
    }

    impl LifecycleEmitter for RecordingEmitter {
        fn emit_stderr(&self, signal: LifecycleSignal, text: &str, _term: &Terminal) {
            self.actions.lock().unwrap().push(EmittedAction::Stderr {
                signal,
                text: text.to_string(),
            });
        }

        fn emit_message(
            &self,
            text: &str,
            _source_path: &Path,
            _repo_root: Option<&Path>,
            _messaging: &RuntimeMessagingSettings,
        ) {
            self.actions.lock().unwrap().push(EmittedAction::Message {
                text: text.to_string(),
            });
        }

        fn emit_speech(&self, text: &str, _tts_config: TtsConfig) {
            self.actions.lock().unwrap().push(EmittedAction::Speech {
                text: text.to_string(),
            });
        }

        fn emit_effect(&self, name: &str) {
            self.actions.lock().unwrap().push(EmittedAction::Effect {
                name: name.to_string(),
            });
        }

        fn emit_notification(&self, title: &str) {
            self.actions
                .lock()
                .unwrap()
                .push(EmittedAction::Notification {
                    title: title.to_string(),
                });
        }
    }

    fn test_config() -> LifecycleConfig {
        parse_lifecycle_config(
            &json!({
                "start":   { "stderr": "starting" },
                "success": { "stderr": "done" },
                "blocked": { "stderr": "blocked" },
                "failure": { "stderr": "failed" },
            }),
            dummy_path(),
        )
        .unwrap()
    }

    fn test_ctx() -> (GlobalSettings, RuntimeMessagingSettings, Terminal) {
        (
            GlobalSettings::default(),
            RuntimeMessagingSettings {
                user: None,
                repo: None,
            },
            Terminal::default(),
        )
    }

    fn make_guard<'a>(
        config: &'a LifecycleConfig,
        ctx: &'a LifecycleRuntimeContext<'a>,
        emitter: &'a RecordingEmitter,
    ) -> LifecycleRunGuard<'a> {
        LifecycleRunGuard::new(config, ctx, emitter)
    }

    #[test]
    fn guard_emits_start_once() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);

        guard.emit_start_once();
        guard.emit_start_once(); // second call is idempotent
        guard.defuse();

        assert_eq!(emitter.signals(), vec![LifecycleSignal::Start]);
    }

    #[test]
    fn guard_drop_emits_blocked_before_launch() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let mut guard = make_guard(&config, &ctx, &emitter);
            guard.emit_start_once();
            // drop without terminal signal, not launched
        }

        assert_eq!(
            emitter.signals(),
            vec![LifecycleSignal::Start, LifecycleSignal::Blocked]
        );
    }

    #[test]
    fn guard_drop_emits_failure_after_launch() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let mut guard = make_guard(&config, &ctx, &emitter);
            guard.emit_start_once();
            guard.mark_provider_launched();
            // drop without terminal signal, but launched
        }

        assert_eq!(
            emitter.signals(),
            vec![LifecycleSignal::Start, LifecycleSignal::Failure]
        );
    }

    #[test]
    fn guard_drop_silent_without_start() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let _guard = make_guard(&config, &ctx, &emitter);
            // drop without ever emitting start
        }

        assert!(emitter.signals().is_empty());
    }

    #[test]
    fn guard_emit_terminal_prevents_drop_emission() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let mut guard = make_guard(&config, &ctx, &emitter);
            guard.emit_start_once();
            guard.mark_provider_launched();
            guard.emit_terminal(LifecycleSignal::Success);
            // drop after explicit terminal — no double emission
        }

        assert_eq!(
            emitter.signals(),
            vec![LifecycleSignal::Start, LifecycleSignal::Success]
        );
    }

    #[test]
    fn guard_defuse_prevents_drop_emission() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let mut guard = make_guard(&config, &ctx, &emitter);
            guard.emit_start_once();
            guard.defuse();
        }

        // Only start, no terminal from Drop
        assert_eq!(emitter.signals(), vec![LifecycleSignal::Start]);
    }

    #[test]
    fn guard_emit_blocked_or_failure_pre_launch() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let mut guard = make_guard(&config, &ctx, &emitter);
            guard.emit_start_once();
            guard.emit_blocked_or_failure(); // pre-launch → Blocked
        }

        assert_eq!(
            emitter.signals(),
            vec![LifecycleSignal::Start, LifecycleSignal::Blocked]
        );
    }

    #[test]
    fn guard_emit_blocked_or_failure_post_launch() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();

        {
            let mut guard = make_guard(&config, &ctx, &emitter);
            guard.emit_start_once();
            guard.mark_provider_launched();
            guard.emit_blocked_or_failure(); // post-launch → Failure
        }

        assert_eq!(
            emitter.signals(),
            vec![LifecycleSignal::Start, LifecycleSignal::Failure]
        );
    }

    #[test]
    fn guard_state_accessors() {
        let config = test_config();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);

        assert!(!guard.start_emitted());
        assert!(!guard.provider_launched());

        guard.emit_start_once();
        assert!(guard.start_emitted());
        assert!(!guard.provider_launched());

        guard.mark_provider_launched();
        assert!(guard.provider_launched());

        guard.defuse();
    }

    #[test]
    fn validation_is_the_dispatch_gate_for_leaked_lifecycle() {
        // The `LifecycleRunGuard` does not re-validate; it dispatches whatever
        // string the config holds. The contract "no side effect dispatches a
        // leaked expression" is upheld by `validate_no_interpolation_leaks`
        // running in the prepare layer, *before* a guard is ever built. This
        // test proves both halves of that boundary against the fake emitter.
        let leaked = parse_lifecycle_config(
            &json!({ "start": { "message": "{{ broken( }}" } }),
            dummy_path(),
        )
        .unwrap();

        // 1. Validation rejects the leaked config — the production choke point.
        let err = validate_no_interpolation_leaks(&leaked, dummy_path(), &[]).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::LifecycleInterpolationLeak { .. }
        ));

        // 2. A guard built from that same config WOULD dispatch the raw span
        //    (the message reaches the emitter verbatim), confirming the guard
        //    itself is not the gate — only the prepare-layer validation is.
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();
        {
            let mut guard = make_guard(&leaked, &ctx, &emitter);
            guard.emit_start_once();
            guard.defuse();
        }
        assert!(
            emitter.actions().iter().any(|a| matches!(
                a,
                EmittedAction::Message { text } if text.contains("{{ broken(")
            )),
            "guard does not self-gate; validation must run before a guard exists"
        );
    }

    fn fm_from_json(value: serde_json::Value) -> darkmatter::markdown::Frontmatter {
        let mut fm = darkmatter::markdown::Frontmatter::new();
        if let serde_json::Value::Object(map) = value {
            for (key, val) in map {
                fm.insert(&key, val).unwrap();
            }
        }
        fm
    }

    #[test]
    fn undefined_bare_variable_flags_missing_root() {
        let effective = json!({ "area": "claudine" });
        let defined = effective.as_object();
        assert_eq!(undefined_bare_variable("missing", defined), Some("missing"));
        assert_eq!(undefined_bare_variable("area", defined), None);
        // Nested miss under a defined root is treated as defined.
        assert_eq!(undefined_bare_variable("area.sub", defined), None);
        // Runtime namespaces resolve outside the frontmatter.
        assert_eq!(undefined_bare_variable("ctx.area", defined), None);
        assert_eq!(undefined_bare_variable("env.HOME", defined), None);
        assert_eq!(undefined_bare_variable("doc", defined), None);
        assert_eq!(undefined_bare_variable("doc.area", defined), None);
    }

    #[test]
    fn undefined_lifecycle_variable_is_rejected() {
        let raw = fm_from_json(json!({
            "start": { "message": "before {{ missing_lifecycle_var }} after" }
        }));
        let effective = json!({ "start": { "message": "before  after" } });

        let err =
            validate_no_undefined_lifecycle_variables(&raw, &effective, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable {
                property, variable, ..
            } => {
                assert_eq!(property, "start.message");
                assert_eq!(variable, "missing_lifecycle_var");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn defined_and_namespaced_lifecycle_variables_pass() {
        let raw = fm_from_json(json!({
            "start": { "message": "{{ area }} on {{ ctx.today }}" },
            "success": { "say": "{{ missing || 'fallback' }}" },
        }));
        let effective = json!({ "area": "claudine" });

        assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, dummy_path()).is_ok());
    }

    #[test]
    fn undefined_variable_inside_function_call_is_rejected() {
        // The original broken prompt used `parent_dir(review)`: a bare undefined
        // variable as a function argument must fail preparation, not collapse to
        // an empty string the way the whole-span-only guard let it.
        let raw = fm_from_json(json!({
            "start": { "message": "before {{ parent_dir(missing_review) }} after" }
        }));
        let effective = json!({ "area": "claudine" });

        let err =
            validate_no_undefined_lifecycle_variables(&raw, &effective, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable {
                property, variable, ..
            } => {
                assert_eq!(property, "start.message");
                assert_eq!(variable, "missing_review");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn undefined_variable_inside_fallback_argument_passes() {
        // Fallback semantics tolerate the undefined operand even when it is
        // wrapped in a function call, so the whole subtree is skipped.
        let raw = fm_from_json(json!({
            "start": { "message": "{{ parent_dir(missing) || 'home' }}" }
        }));
        let effective = json!({ "area": "claudine" });

        assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, dummy_path()).is_ok());
    }

    #[test]
    fn undefined_variable_in_ternary_condition_is_rejected() {
        let raw = fm_from_json(json!({
            "start": { "message": "{{ missing == 'x' ? 'a' : 'b' }}" }
        }));
        let effective = json!({});

        let err =
            validate_no_undefined_lifecycle_variables(&raw, &effective, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable {
                property, variable, ..
            } => {
                assert_eq!(property, "start.message");
                assert_eq!(variable, "missing");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn undefined_variable_in_ternary_truthy_condition_is_rejected() {
        let raw = fm_from_json(json!({
            "start": { "message": "{{ missing ? 'a' : 'b' }}" }
        }));
        let effective = json!({});

        let err =
            validate_no_undefined_lifecycle_variables(&raw, &effective, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable {
                property, variable, ..
            } => {
                assert_eq!(property, "start.message");
                assert_eq!(variable, "missing");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn defined_condition_with_undefined_branch_operands_passes() {
        // Ternary branches intentionally tolerate undefined operands; only the
        // condition is checked.
        let raw = fm_from_json(json!({
            "start": { "message": "{{ defined ? missing : also_missing }}" }
        }));
        let effective = json!({ "defined": true });

        assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, dummy_path()).is_ok());
    }

    #[test]
    fn undefined_variable_in_index_is_rejected() {
        let raw = fm_from_json(json!({
            "start": { "message": "{{ missing[0] }}" }
        }));
        let effective = json!({});

        let err =
            validate_no_undefined_lifecycle_variables(&raw, &effective, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable { variable, .. } => {
                assert_eq!(variable, "missing");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn undefined_variable_in_member_access_is_rejected() {
        let raw = fm_from_json(json!({
            "start": { "message": "{{ missing.foo }}" }
        }));
        let effective = json!({});

        let err =
            validate_no_undefined_lifecycle_variables(&raw, &effective, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable { variable, .. } => {
                assert_eq!(variable, "missing");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn defined_variable_inside_function_call_passes() {
        let raw = fm_from_json(json!({
            "start": { "message": "{{ parent_dir(area) }}" }
        }));
        let effective = json!({ "area": "/repo/claudine" });

        assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, dummy_path()).is_ok());
    }

    #[test]
    fn guard_non_audio_before_audio() {
        let config = parse_lifecycle_config(
            &json!({
                "start": {
                    "stderr": "starting",
                    "message": "msg",
                    "notify": "notify-msg",
                    "say": "hello",
                    "effect": "confirmation",
                }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.defuse();

        let actions = emitter.actions();
        assert_eq!(actions.len(), 5);
        // Non-audio first
        assert!(matches!(actions[0], EmittedAction::Stderr { .. }));
        assert!(matches!(actions[1], EmittedAction::Message { .. }));
        assert!(matches!(actions[2], EmittedAction::Notification { .. }));
        // Audio: effect before say (default order)
        assert!(matches!(actions[3], EmittedAction::Effect { .. }));
        assert!(matches!(actions[4], EmittedAction::Speech { .. }));
    }

    #[test]
    fn guard_say_first_ordering() {
        let config = parse_lifecycle_config(
            &json!({
                "start": {
                    "say_first": "hello",
                    "effect": "confirmation",
                }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.defuse();

        let actions = emitter.actions();
        assert_eq!(actions.len(), 2);
        // say_first → speech before effect
        assert!(matches!(actions[0], EmittedAction::Speech { .. }));
        assert!(matches!(actions[1], EmittedAction::Effect { .. }));
    }

    #[test]
    #[serial_test::serial]
    fn emit_signal_skips_blocking_side_effects_when_interrupted() {
        // Bug fix (2026-05-09): a Ctrl+C during a long compose run must
        // skip messenger sends, desktop notifications, TTS, and sound
        // effects so the process exits promptly. Only the cheap stderr
        // line is allowed to render so the user sees the terminal status.
        let config = parse_lifecycle_config(
            &json!({
                "failure": {
                    "stderr": "failed",
                    "message": "Compose run failed",
                    "notify": "Compose failed",
                    "say": "compose failed",
                    "effect": "confirmation",
                }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);

        crate::interrupt::clear_for_tests();
        crate::interrupt::mark_interrupted();
        guard.emit_terminal(LifecycleSignal::Failure);
        crate::interrupt::clear_for_tests();

        let actions = emitter.actions();
        assert_eq!(
            actions.len(),
            1,
            "interrupt must drop messenger/notification/TTS/effect; got: {actions:?}"
        );
        assert!(
            matches!(actions[0], EmittedAction::Stderr { .. }),
            "stderr line must still render so the user sees the terminal status"
        );
    }

    #[test]
    #[serial_test::serial]
    fn emit_signal_runs_all_side_effects_when_not_interrupted() {
        // Companion to the interrupt test: when no interrupt is observed,
        // every configured side effect still fires.
        let config = parse_lifecycle_config(
            &json!({
                "failure": {
                    "stderr": "failed",
                    "message": "Compose run failed",
                    "notify": "Compose failed",
                }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);

        crate::interrupt::clear_for_tests();
        guard.emit_terminal(LifecycleSignal::Failure);

        let actions = emitter.actions();
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, EmittedAction::Stderr { .. }))
        );
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, EmittedAction::Message { .. }))
        );
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, EmittedAction::Notification { .. }))
        );
    }

    // =====================================================================
    // notify parsing and emission (Phase 3)
    // =====================================================================

    #[test]
    fn parses_notify_for_all_signals() {
        let fm = json!({
            "start": { "notify": "Starting" },
            "success": { "notify": "Done" },
            "blocked": { "notify": "Blocked" },
            "failure": { "notify": "Failed" }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();

        assert_eq!(
            config.start.as_ref().unwrap().notify.as_deref(),
            Some("Starting")
        );
        assert_eq!(
            config.success.as_ref().unwrap().notify.as_deref(),
            Some("Done")
        );
        assert_eq!(
            config.blocked.as_ref().unwrap().notify.as_deref(),
            Some("Blocked")
        );
        assert_eq!(
            config.failure.as_ref().unwrap().notify.as_deref(),
            Some("Failed")
        );
    }

    #[test]
    fn parses_message_and_notify_independently() {
        let fm = json!({
            "start": {
                "message": "Remote message",
                "notify": "Local notification"
            }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let start = config.start.as_ref().unwrap();
        assert_eq!(start.message.as_deref(), Some("Remote message"));
        assert_eq!(start.notify.as_deref(), Some("Local notification"));
    }

    #[test]
    fn blank_notify_is_normalized_to_none() {
        let fm = json!({
            "start": { "notify": "   " },
            "success": { "notify": "" }
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(config.start.as_ref().unwrap().notify.is_none());
        assert!(config.success.as_ref().unwrap().notify.is_none());
    }

    #[test]
    fn notify_emits_without_active_route() {
        let config = parse_lifecycle_config(
            &json!({
                "start": { "notify": "Hello desktop" }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.defuse();

        let actions = emitter.actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            EmittedAction::Notification {
                title: "Hello desktop".to_string()
            }
        );
    }

    #[test]
    fn notify_emits_before_audio_phases() {
        let config = parse_lifecycle_config(
            &json!({
                "start": {
                    "notify": "Desktop first",
                    "say": "hello",
                    "effect": "confirmation",
                }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_start_once();
        guard.defuse();

        let actions = emitter.actions();
        assert_eq!(actions.len(), 3);
        assert!(matches!(actions[0], EmittedAction::Notification { .. }));
        assert!(matches!(actions[1], EmittedAction::Effect { .. }));
        assert!(matches!(actions[2], EmittedAction::Speech { .. }));
    }

    #[test]
    fn notify_alone_no_other_outputs() {
        let config = parse_lifecycle_config(
            &json!({
                "success": { "notify": "Only notify" }
            }),
            dummy_path(),
        )
        .unwrap();
        let (settings, messaging, term) = test_ctx();
        let ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: Path::new("/tmp/test.md"),
            repo_root: None,
        };
        let emitter = RecordingEmitter::new();
        let mut guard = make_guard(&config, &ctx, &emitter);
        guard.emit_terminal(LifecycleSignal::Success);

        let actions = emitter.actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            EmittedAction::Notification {
                title: "Only notify".to_string()
            }
        );
    }

    #[tokio::test]
    async fn default_lifecycle_emitter_emit_notification_does_not_panic() {
        let emitter = DefaultLifecycleEmitter;
        // Fire-and-forget through the title-only trait method.
        emitter.emit_notification("unit testing");
        // And exercise the body-bearing path directly so the rendered
        // notification has a distinct title and message line.
        crate::messaging::execute_notification(
            "unit testing",
            Some("you can dismiss this notification"),
        );
        // Give the spawned tasks a moment to start
        tokio::task::yield_now().await;
    }

    #[test]
    fn lifecycle_invalid_error_renders_as_block_error() {
        use biscuit_terminal::errors::BlockError;
        use biscuit_terminal::utils::escape_codes::strip_escape_codes;

        let frontmatter = json!({
            "success": {
                "speak": "hello"
            }
        });

        let err =
            parse_lifecycle_config(&frontmatter, Path::new("prompts/sentrux.md")).unwrap_err();
        let CompositionError::LifecycleInvalid {
            property,
            unknown_field,
            expected_fields,
            source_file,
            ..
        } = &err
        else {
            panic!("expected LifecycleInvalid, got {err:?}");
        };

        assert_eq!(property, "success");
        assert_eq!(unknown_field.as_deref(), Some("speak"));
        assert_eq!(source_file, Path::new("prompts/sentrux.md"));
        assert!(expected_fields.contains(&"say".to_string()));
        assert!(expected_fields.contains(&"say_first".to_string()));
        assert!(expected_fields.contains(&"effect".to_string()));

        let rendered = strip_escape_codes(err.report_block_error_optimistic(Some(80)));
        assert!(
            rendered.contains("success.speak"),
            "dotted property should appear: {rendered}"
        );
        assert!(
            rendered.contains("sentrux.md"),
            "file name should appear: {rendered}"
        );
        assert!(
            rendered.contains("say"),
            "expected fields should list 'say': {rendered}"
        );
    }

    #[test]
    fn parse_serde_unknown_field_extracts_field_and_expected() {
        let frontmatter = json!({
            "failure": {
                "bogus_field": true
            }
        });

        let err = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap_err();
        let CompositionError::LifecycleInvalid {
            property,
            unknown_field,
            expected_fields,
            ..
        } = &err
        else {
            panic!("expected LifecycleInvalid, got {err:?}");
        };

        assert_eq!(property, "failure");
        assert_eq!(unknown_field.as_deref(), Some("bogus_field"));
        assert!(!expected_fields.is_empty());
        assert!(expected_fields.contains(&"say".to_string()));
    }
}
