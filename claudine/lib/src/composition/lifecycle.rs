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
use biscuit_terminal::prelude::Renderable;
use biscuit_terminal::terminal::Terminal;
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
        crate::messaging::execute_notification(title);
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

    /// Whether `Start` has been emitted.
    pub fn start_emitted(&self) -> bool {
        self.start_emitted
    }

    /// Whether the provider child has actually been spawned.
    pub fn provider_launched(&self) -> bool {
        self.provider_launched
    }

    /// Emit a single lifecycle signal through the injected emitter.
    fn emit_signal(&self, signal: LifecycleSignal) {
        let Some(notification) = self.config.get(signal) else {
            return;
        };

        // --- Non-audio fan-out (immediate) ---
        if let Some(stderr_text) = &notification.stderr {
            self.emitter.emit_stderr(signal, stderr_text, self.ctx.term);
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
fn parse_serde_unknown_field(
    err: &serde_json::Error,
) -> (Option<String>, Vec<String>) {
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

/// Play a sound effect synchronously (blocking).
fn play_effect_blocking(name: &str) {
    let Some(effect) = playa::SoundEffect::from_name(name) else {
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
}

/// Speak text synchronously using the Tokio runtime.
///
/// Uses `block_in_place` to avoid panicking when called from within a
/// Tokio async context (e.g. `#[tokio::main]`).
fn say_blocking(text: &str, config: TtsConfig) {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let text = text.to_string();
        tokio::task::block_in_place(|| {
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
    } else {
        warn!("No Tokio runtime available for lifecycle TTS");
    }
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
        crate::messaging::execute_notification(notify_title);
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
        parse_lifecycle_config(&json!({
            "start":   { "stderr": "starting" },
            "success": { "stderr": "done" },
            "blocked": { "stderr": "blocked" },
            "failure": { "stderr": "failed" },
        }), dummy_path())
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
    fn guard_non_audio_before_audio() {
        let config = parse_lifecycle_config(&json!({
            "start": {
                "stderr": "starting",
                "message": "msg",
                "notify": "notify-msg",
                "say": "hello",
                "effect": "confirmation",
            }
        }), dummy_path())
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
        let config = parse_lifecycle_config(&json!({
            "start": {
                "say_first": "hello",
                "effect": "confirmation",
            }
        }), dummy_path())
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
        let config = parse_lifecycle_config(&json!({
            "start": { "notify": "Hello desktop" }
        }), dummy_path())
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
        let config = parse_lifecycle_config(&json!({
            "start": {
                "notify": "Desktop first",
                "say": "hello",
                "effect": "confirmation",
            }
        }), dummy_path())
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
        let config = parse_lifecycle_config(&json!({
            "success": { "notify": "Only notify" }
        }), dummy_path())
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
        // Fire-and-forget: should return immediately without panic
        emitter.emit_notification("test notification title");
        // Give the spawned task a moment to start
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

        let err = parse_lifecycle_config(&frontmatter, Path::new("prompts/sentrux.md")).unwrap_err();
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
        assert!(rendered.contains("success.speak"), "dotted property should appear: {rendered}");
        assert!(rendered.contains("sentrux.md"), "file name should appear: {rendered}");
        assert!(rendered.contains("say"), "expected fields should list 'say': {rendered}");
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
