use serde::{Deserialize, Serialize};

use crate::provider::Provider;
use crate::actions::{HookAction, LogTarget};
use crate::messaging::ScopedMessagingSettings;
use crate::services::protect::ProtectConfig;

/// Global settings that apply to all event bindings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalSettings {
    /// Default log target used when an event's `Log` action
    /// doesn't specify its own target.
    #[serde(default)]
    pub default_log_target: Option<LogTarget>,

    /// Default TTS voice/engine settings for `Speak` actions.
    /// Passed through to biscuit-speaks.
    #[serde(default)]
    pub tts: Option<TtsSettings>,

    /// Link strategy settings used for canonical provider selection and ordering.
    #[serde(default)]
    pub linking: Option<LinkingSettings>,

    /// Protect service policy configuration.
    #[serde(default)]
    pub protect: Option<ProtectConfig>,

    /// Messaging destination settings for `Message` actions.
    #[serde(default)]
    pub messaging: Option<ScopedMessagingSettings>,
}

/// TTS configuration forwarded to biscuit-speaks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsSettings {
    /// Preferred TTS provider (e.g., "say", "espeak", "elevenlabs").
    #[serde(default)]
    pub provider: Option<String>,

    /// Voice name or identifier.
    #[serde(default)]
    pub voice: Option<String>,

    /// Speech rate multiplier (1.0 = normal).
    #[serde(default)]
    pub rate: Option<f32>,
}

/// Link strategy settings persisted in user/repo config.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LinkingSettings {
    /// Ordered provider preference used for canonical candidate ranking.
    #[serde(default)]
    pub preference: Vec<Provider>,

    /// Persisted canonical providers per `(scope, resource)` slot.
    #[serde(default)]
    pub canonical_provider: CanonicalProviderSettings,
}

/// Canonical provider slots for each scope/resource pair.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalProviderSettings {
    #[serde(default)]
    pub user_skill: Option<Provider>,
    #[serde(default)]
    pub user_command: Option<Provider>,
    #[serde(default)]
    pub user_agent: Option<Provider>,
    #[serde(default)]
    pub user_script: Option<Provider>,
    #[serde(default)]
    pub repo_skill: Option<Provider>,
    #[serde(default)]
    pub repo_command: Option<Provider>,
    #[serde(default)]
    pub repo_agent: Option<Provider>,
    #[serde(default)]
    pub repo_script: Option<Provider>,
}

/// Configuration for a single event binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventBinding {
    /// Whether this binding is active. Defaults to `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Actions to execute when this event fires.
    #[serde(default)]
    pub actions: Vec<HookAction>,

    /// Optional filter: only fire for events matching this regex
    /// against the tool name, notification type, or session source.
    #[serde(default)]
    pub matcher: Option<String>,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_binding_defaults() {
        let json = serde_json::json!({
            "actions": []
        });
        let binding: EventBinding = serde_json::from_value(json).unwrap();
        assert!(binding.enabled); // defaults to true
        assert!(binding.matcher.is_none());
    }

    #[test]
    fn global_settings_default() {
        let settings = GlobalSettings::default();
        assert!(settings.default_log_target.is_none());
        assert!(settings.tts.is_none());
        assert!(settings.linking.is_none());
        assert!(settings.protect.is_none());
    }

    #[test]
    fn global_settings_with_messaging() {
        let json = serde_json::json!({
            "messaging": {
                "active": "ops",
                "configs": {
                    "ops": {
                        "provider": "slack",
                        "channel_id": "C123"
                    }
                }
            }
        });

        let settings: GlobalSettings = serde_json::from_value(json).unwrap();
        let messaging = settings.messaging.unwrap();
        assert_eq!(messaging.active.as_deref(), Some("ops"));
    }

    #[test]
    fn global_settings_without_messaging() {
        let json = serde_json::json!({});
        let settings: GlobalSettings = serde_json::from_value(json).unwrap();
        assert!(settings.messaging.is_none());
    }
}
