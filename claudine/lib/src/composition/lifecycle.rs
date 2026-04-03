//! Lifecycle notification types and parsing for composition frontmatter.
//!
//! This module provides support for `start`, `success`, `blocked`, and `failure`
//! lifecycle notifications in composition frontmatter. Each notification can
//! specify optional fields like `speak`, `effect`, `message`, etc.

use std::path::Path;

use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::terminal::Terminal;
use serde::{Deserialize, Serialize};

use super::error::CompositionError;
use crate::events::GlobalSettings;
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
            .map_err(|e| CompositionError::ComposeFailed(format!("invalid {}: {}", property_name, e)))?;

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
}
