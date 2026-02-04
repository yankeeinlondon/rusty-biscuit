use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::agentric_event::AgenticEvent;
use super::event_action::{EventAction, LogTarget};
use super::provider::Provider;

/// Root configuration loaded from `~/.hooker`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookerConfig {
    /// Schema version for forward compatibility.
    pub version: String,

    /// Global settings.
    #[serde(default)]
    pub settings: GlobalSettings,

    /// Event bindings: map from event name to its configuration.
    #[serde(default)]
    pub events: HashMap<AgenticEvent, EventBinding>,
}

/// Global settings that apply to all event bindings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalSettings {
    /// Default log target used when an event's `Log` action
    /// doesn't specify its own target.
    #[serde(default)]
    pub default_log_target: Option<LogTarget>,

    /// Default TTS voice/engine settings for `Speak` actions.
    /// Passed through to biscuit-speaks.
    #[serde(default)]
    pub tts: Option<TtsSettings>,
}

/// TTS configuration forwarded to biscuit-speaks.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Configuration for a single event binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBinding {
    /// Whether this binding is active. Defaults to `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Actions to execute when this event fires (all providers).
    #[serde(default)]
    pub actions: Vec<EventAction>,

    /// Optional filter: only fire for events matching this regex
    /// against the tool name, notification type, or session source.
    #[serde(default)]
    pub matcher: Option<String>,

    /// Per-provider overrides. When a provider key is present,
    /// its actions REPLACE the top-level actions for that provider.
    #[serde(default)]
    pub overrides: HashMap<Provider, ProviderOverride>,
}

/// Provider-specific override for an event binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderOverride {
    /// Whether this override is active. Defaults to `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Actions that REPLACE the parent binding's actions
    /// for this specific provider.
    #[serde(default)]
    pub actions: Vec<EventAction>,

    /// Provider-specific matcher override.
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
    fn deserialize_example_config() {
        let json = serde_json::json!({
            "version": "1.0",
            "settings": {
                "default_log_target": {
                    "type": "local_file",
                    "path": "~/.claudine/events.jsonl"
                },
                "tts": {
                    "provider": "say",
                    "voice": "Samantha",
                    "rate": 1.2
                }
            },
            "events": {
                "session_start": {
                    "enabled": true,
                    "actions": [
                        {
                            "type": "sound_effect",
                            "name": "power-up"
                        },
                        {
                            "type": "speak",
                            "message": "Session started"
                        },
                        {
                            "type": "log",
                            "target": {
                                "type": "local_file",
                                "path": "~/.claudine/events.jsonl"
                            }
                        }
                    ]
                },
                "turn_complete": {
                    "enabled": true,
                    "actions": [
                        {
                            "type": "sound_effect",
                            "name": "success",
                            "volume": 0.6
                        }
                    ],
                    "overrides": {
                        "claude": {
                            "enabled": true,
                            "actions": [
                                {
                                    "type": "speak",
                                    "message": "Claude has finished"
                                }
                            ]
                        },
                        "codex": {
                            "enabled": true,
                            "actions": [
                                {
                                    "type": "speak",
                                    "message": "Codex turn done"
                                }
                            ]
                        }
                    }
                },
                "before_tool": {
                    "enabled": true,
                    "matcher": "Bash|bash",
                    "actions": [
                        {
                            "type": "report",
                            "handler": {
                                "format": "compact",
                                "template": "[TOOL] {tool_name}: executing",
                                "include_metadata": false
                            }
                        }
                    ]
                },
                "tool_error": {
                    "enabled": true,
                    "actions": [
                        {
                            "type": "sound_effect",
                            "name": "sad-trombone",
                            "volume": 0.8
                        },
                        {
                            "type": "speak",
                            "message": "Tool {tool_name} failed: {error}"
                        },
                        {
                            "type": "log",
                            "target": {
                                "type": "server",
                                "url": "https://my-logging-server.example.com/events"
                            }
                        }
                    ]
                },
                "notification": {
                    "enabled": true,
                    "matcher": "permission_prompt|ToolPermission",
                    "actions": [
                        {
                            "type": "sound_effect",
                            "name": "notification"
                        },
                        {
                            "type": "speak",
                            "message": "Attention needed"
                        }
                    ]
                }
            }
        });

        let config: HookerConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.version, "1.0");

        // Settings
        assert!(config.settings.default_log_target.is_some());
        let tts = config.settings.tts.as_ref().unwrap();
        assert_eq!(tts.provider.as_deref(), Some("say"));
        assert_eq!(tts.voice.as_deref(), Some("Samantha"));
        assert_eq!(tts.rate, Some(1.2));

        // Events
        assert_eq!(config.events.len(), 5);

        // session_start
        let session_start = config.events.get(&AgenticEvent::SessionStart).unwrap();
        assert!(session_start.enabled);
        assert_eq!(session_start.actions.len(), 3);

        // turn_complete with overrides
        let turn_complete = config.events.get(&AgenticEvent::TurnComplete).unwrap();
        assert_eq!(turn_complete.overrides.len(), 2);
        assert!(turn_complete.overrides.contains_key(&Provider::Claude));
        assert!(turn_complete.overrides.contains_key(&Provider::Codex));

        // before_tool with matcher
        let before_tool = config.events.get(&AgenticEvent::BeforeTool).unwrap();
        assert_eq!(before_tool.matcher.as_deref(), Some("Bash|bash"));

        // tool_error
        let tool_error = config.events.get(&AgenticEvent::ToolError).unwrap();
        assert_eq!(tool_error.actions.len(), 3);
    }

    #[test]
    fn round_trip_config() {
        let config = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            events: HashMap::new(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: HookerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version, "1.0");
        assert!(back.events.is_empty());
    }

    #[test]
    fn event_binding_defaults() {
        let json = serde_json::json!({
            "actions": []
        });
        let binding: EventBinding = serde_json::from_value(json).unwrap();
        assert!(binding.enabled); // defaults to true
        assert!(binding.matcher.is_none());
        assert!(binding.overrides.is_empty());
    }

    #[test]
    fn provider_override_defaults() {
        let json = serde_json::json!({
            "actions": [
                { "type": "speak", "message": "hello" }
            ]
        });
        let override_: ProviderOverride = serde_json::from_value(json).unwrap();
        assert!(override_.enabled); // defaults to true
        assert_eq!(override_.actions.len(), 1);
        assert!(override_.matcher.is_none());
    }

    #[test]
    fn global_settings_default() {
        let settings = GlobalSettings::default();
        assert!(settings.default_log_target.is_none());
        assert!(settings.tts.is_none());
    }
}
