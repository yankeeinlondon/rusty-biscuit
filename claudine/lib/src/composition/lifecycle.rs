//! Lifecycle notification types and parsing for composition frontmatter.
//!
//! This module provides support for `start`, `success`, `blocked`, and `failure`
//! lifecycle notifications in composition frontmatter. Each notification can
//! specify optional fields like `speak`, `effect`, `message`, etc.

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
///   speak: "Starting composition workflow"
///   effect: "confirmation"
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LifecycleNotification {
    /// Text to speak using TTS (mutually exclusive with `speak_first`).
    pub speak: Option<String>,

    /// Text to speak before other actions (mutually exclusive with `speak`).
    pub speak_first: Option<String>,

    /// Sound effect to play (kebab-case name like "confirmation").
    pub effect: Option<String>,

    /// Message to display in the terminal.
    pub message: Option<String>,

    /// Message to write to stderr.
    pub stderr: Option<String>,
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
///   speak: "Composition complete"
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
/// Used to track which lifecycle signals have been emitted during execution.
#[derive(Debug, Clone, Default)]
pub struct LifecycleRuntimeState {
    /// Whether the `start` signal has been emitted.
    pub start_emitted: bool,

    /// Whether the provider launch has started (used for start signal timing).
    pub provider_launch_started: bool,
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
/// - `speak` + `effect` → effect first, then speech
/// - `speak_first` + `effect` → speech first, then effect
///
/// When only one audio output is present, it is the sole phase.
fn audio_phases(n: &LifecycleNotification) -> Vec<AudioPhase> {
    let speech_text = n
        .speak
        .as_deref()
        .or(n.speak_first.as_deref())
        .filter(|s| !s.is_empty());
    let effect_name = n.effect.as_deref().filter(|s| !s.is_empty());
    let speech_first = n.speak_first.is_some();

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
    /// assert_eq!(LifecycleSignal::Blocked.status_state(), StatusState::Failure);
    /// assert_eq!(LifecycleSignal::Failure.status_state(), StatusState::Failure);
    /// ```
    pub fn status_state(&self) -> StatusState {
        match self {
            Self::Start => StatusState::Info,
            Self::Success => StatusState::Success,
            Self::Blocked | Self::Failure => StatusState::Failure,
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
/// and ignores all other frontmatter keys. Validates mutual exclusivity of `speak`
/// and `speak_first`, and validates sound effect names.
///
/// ## Returns
///
/// Returns `Ok(LifecycleConfig)` on success, or a `CompositionError` if validation fails.
///
/// ## Errors
///
/// - `LifecycleSpeakConflict`: Both `speak` and `speak_first` are present
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
/// let config = parse_lifecycle_config(&frontmatter).unwrap();
/// assert!(config.start.is_some());
/// ```
pub fn parse_lifecycle_config(
    frontmatter: &serde_json::Value,
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
                CompositionError::ComposeFailed(format!("invalid {}: {}", property_name, e))
            })?;

        // Normalize empty strings to None
        normalize_empty_string(&mut notification.speak);
        normalize_empty_string(&mut notification.speak_first);
        normalize_empty_string(&mut notification.effect);
        normalize_empty_string(&mut notification.message);
        normalize_empty_string(&mut notification.stderr);

        // Validate mutual exclusivity of speak and speak_first
        if notification.speak.is_some() && notification.speak_first.is_some() {
            return Err(CompositionError::LifecycleSpeakConflict(
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
fn speak_blocking(text: &str, config: TtsConfig) {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let text = text.to_string();
        handle.block_on(async move {
            if let Err(e) = biscuit_speaks::Speak::new(text)
                .with_config(config)
                .play()
                .await
            {
                warn!(%e, "Lifecycle TTS playback failed");
            }
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

    // --- Audio phases (sequential, blocking) ---

    let phases = audio_phases(notification);
    let tts_config = tts_config_from_settings(ctx.settings.tts.as_ref());

    for phase in phases {
        match phase {
            AudioPhase::Speak(text) => speak_blocking(&text, tts_config.clone()),
            AudioPhase::Effect(name) => play_effect_blocking(&name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_valid_lifecycle_config() {
        let frontmatter = json!({
            "start": {
                "message": "Starting composition..."
            },
            "success": {
                "speak": "All done!",
                "effect": "confirmation"
            }
        });

        let config = parse_lifecycle_config(&frontmatter).unwrap();
        assert!(config.start.is_some());
        assert_eq!(
            config.start.as_ref().unwrap().message.as_deref(),
            Some("Starting composition...")
        );

        assert!(config.success.is_some());
        let success = config.success.as_ref().unwrap();
        assert_eq!(success.speak.as_deref(), Some("All done!"));
        assert_eq!(success.effect.as_deref(), Some("confirmation"));
    }

    #[test]
    fn rejects_both_speak_and_speak_first() {
        let frontmatter = json!({
            "start": {
                "speak": "Starting",
                "speak_first": "Also starting"
            }
        });

        let result = parse_lifecycle_config(&frontmatter);
        assert!(matches!(
            result,
            Err(CompositionError::LifecycleSpeakConflict(_))
        ));
    }

    #[test]
    fn trims_empty_strings_to_none() {
        let frontmatter = json!({
            "start": {
                "message": "   ",
                "speak": ""
            }
        });

        let config = parse_lifecycle_config(&frontmatter).unwrap();
        let start = config.start.as_ref().unwrap();
        assert!(start.message.is_none());
        assert!(start.speak.is_none());
    }

    #[test]
    fn rejects_unknown_keys() {
        let frontmatter = json!({
            "start": {
                "message": "Starting",
                "unknown_field": "value"
            }
        });

        let result = parse_lifecycle_config(&frontmatter);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_effect_name() {
        let frontmatter = json!({
            "start": {
                "effect": "nonexistent-effect"
            }
        });

        let result = parse_lifecycle_config(&frontmatter);
        assert!(matches!(
            result,
            Err(CompositionError::LifecycleUnknownEffect(_, _))
        ));
    }

    #[test]
    fn speak_plus_effect_is_valid() {
        let frontmatter = json!({
            "success": {
                "speak": "Done!",
                "effect": "confirmation"
            }
        });

        let config = parse_lifecycle_config(&frontmatter).unwrap();
        let success = config.success.as_ref().unwrap();
        assert_eq!(success.speak.as_deref(), Some("Done!"));
        assert_eq!(success.effect.as_deref(), Some("confirmation"));
    }

    #[test]
    fn speak_first_plus_effect_is_valid() {
        let frontmatter = json!({
            "success": {
                "speak_first": "Starting now",
                "effect": "confirmation"
            }
        });

        let config = parse_lifecycle_config(&frontmatter).unwrap();
        let success = config.success.as_ref().unwrap();
        assert_eq!(success.speak_first.as_deref(), Some("Starting now"));
        assert_eq!(success.effect.as_deref(), Some("confirmation"));
    }

    #[test]
    fn empty_frontmatter_returns_default() {
        let frontmatter = json!({});
        let config = parse_lifecycle_config(&frontmatter).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn non_object_frontmatter_returns_default() {
        let frontmatter = json!("not an object");
        let config = parse_lifecycle_config(&frontmatter).unwrap();
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

        let config = parse_lifecycle_config(&frontmatter).unwrap();
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

        let config = parse_lifecycle_config(&frontmatter).unwrap();
        assert!(config.start.is_some());
    }

    #[test]
    fn audio_order_speak_plus_effect() {
        let n = LifecycleNotification {
            speak: Some("Hello".into()),
            effect: Some("doorbell".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 2);
        assert!(matches!(phases[0], AudioPhase::Effect(_)));
        assert!(matches!(phases[1], AudioPhase::Speak(_)));
    }

    #[test]
    fn audio_order_speak_first_plus_effect() {
        let n = LifecycleNotification {
            speak_first: Some("Hello".into()),
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
            speak: Some("Hello".into()),
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
        assert_eq!(
            LifecycleSignal::Blocked.status_state(),
            StatusState::Failure
        );
        assert_eq!(
            LifecycleSignal::Failure.status_state(),
            StatusState::Failure
        );
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
        let config = parse_lifecycle_config(&fm).unwrap();
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
        let non_empty = parse_lifecycle_config(&fm).unwrap();
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn lifecycle_runtime_state_defaults() {
        let state = LifecycleRuntimeState::default();
        assert!(!state.start_emitted);
        assert!(!state.provider_launch_started);
    }
}
