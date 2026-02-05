use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::agentric_event::AgenticEvent;
use super::event_action::{EventAction, LogTarget};
use super::provider::Provider;

/// Root configuration loaded from `~/.hooker`.
///
/// The config is organized per-provider, with each provider having its own
/// set of event bindings. This allows different providers to have different
/// events configured with different actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookerConfig {
    /// Schema version for forward compatibility.
    pub version: String,

    /// Global settings.
    #[serde(default)]
    pub settings: GlobalSettings,

    /// Per-provider configuration.
    #[serde(default)]
    pub providers: HashMap<Provider, ProviderConfig>,
}

/// Configuration for a single provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Event bindings for this provider.
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

    /// Actions to execute when this event fires.
    #[serde(default)]
    pub actions: Vec<EventAction>,

    /// Optional filter: only fire for events matching this regex
    /// against the tool name, notification type, or session source.
    #[serde(default)]
    pub matcher: Option<String>,
}

fn default_true() -> bool {
    true
}

impl HookerConfig {
    /// Get the event binding for a specific provider and event.
    pub fn get_binding(&self, provider: Provider, event: &AgenticEvent) -> Option<&EventBinding> {
        self.providers
            .get(&provider)
            .and_then(|p| p.events.get(event))
    }

    /// Get all configured events for a provider.
    pub fn events_for_provider(&self, provider: Provider) -> Vec<&AgenticEvent> {
        self.providers
            .get(&provider)
            .map(|p| p.events.keys().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_per_provider_config() {
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
            "providers": {
                "claude": {
                    "events": {
                        "session_start": {
                            "enabled": true,
                            "actions": [
                                { "type": "sound_effect", "name": "power-up" },
                                { "type": "speak", "message": "Session started" }
                            ]
                        },
                        "turn_complete": {
                            "enabled": true,
                            "actions": [
                                { "type": "speak", "message": "Claude has finished" }
                            ]
                        },
                        "tool_error": {
                            "enabled": true,
                            "actions": [
                                { "type": "sound_effect", "name": "error" }
                            ]
                        }
                    }
                },
                "codex": {
                    "events": {
                        "turn_complete": {
                            "enabled": true,
                            "actions": [
                                { "type": "speak", "message": "Codex turn done" }
                            ]
                        }
                    }
                },
                "gemini": {
                    "events": {
                        "session_start": {
                            "enabled": true,
                            "actions": [
                                { "type": "sound_effect", "name": "power-up" }
                            ]
                        },
                        "before_tool": {
                            "enabled": true,
                            "matcher": "Bash|bash",
                            "actions": [
                                {
                                    "type": "report",
                                    "handler": {
                                        "format": "compact",
                                        "template": "[TOOL] {tool_name}: executing"
                                    }
                                }
                            ]
                        }
                    }
                }
            }
        });

        let config: HookerConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.version, "1.0");

        // Settings
        assert!(config.settings.default_log_target.is_some());
        let tts = config.settings.tts.as_ref().unwrap();
        assert_eq!(tts.provider.as_deref(), Some("say"));

        // Claude has 3 events
        let claude = config.providers.get(&Provider::Claude).unwrap();
        assert_eq!(claude.events.len(), 3);
        assert!(claude.events.contains_key(&AgenticEvent::SessionStart));
        assert!(claude.events.contains_key(&AgenticEvent::TurnComplete));
        assert!(claude.events.contains_key(&AgenticEvent::ToolError));

        // Codex has only 1 event
        let codex = config.providers.get(&Provider::Codex).unwrap();
        assert_eq!(codex.events.len(), 1);
        assert!(codex.events.contains_key(&AgenticEvent::TurnComplete));

        // Gemini has 2 events
        let gemini = config.providers.get(&Provider::Gemini).unwrap();
        assert_eq!(gemini.events.len(), 2);

        // Check matcher on gemini's before_tool
        let before_tool = gemini.events.get(&AgenticEvent::BeforeTool).unwrap();
        assert_eq!(before_tool.matcher.as_deref(), Some("Bash|bash"));
    }

    #[test]
    fn get_binding_returns_correct_event() {
        let mut config = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            providers: HashMap::new(),
        };

        let mut claude_config = ProviderConfig::default();
        claude_config.events.insert(
            AgenticEvent::TurnComplete,
            EventBinding {
                enabled: true,
                actions: vec![EventAction::Speak {
                    message: "done".to_string(),
                }],
                matcher: None,
            },
        );
        config.providers.insert(Provider::Claude, claude_config);

        // Should find Claude's turn_complete
        let binding = config.get_binding(Provider::Claude, &AgenticEvent::TurnComplete);
        assert!(binding.is_some());

        // Should not find Codex's turn_complete
        let binding = config.get_binding(Provider::Codex, &AgenticEvent::TurnComplete);
        assert!(binding.is_none());

        // Should not find Claude's session_start
        let binding = config.get_binding(Provider::Claude, &AgenticEvent::SessionStart);
        assert!(binding.is_none());
    }

    #[test]
    fn events_for_provider_returns_configured_events() {
        let mut config = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            providers: HashMap::new(),
        };

        let mut claude_config = ProviderConfig::default();
        claude_config.events.insert(
            AgenticEvent::TurnComplete,
            EventBinding {
                enabled: true,
                actions: vec![],
                matcher: None,
            },
        );
        claude_config.events.insert(
            AgenticEvent::SessionStart,
            EventBinding {
                enabled: true,
                actions: vec![],
                matcher: None,
            },
        );
        config.providers.insert(Provider::Claude, claude_config);

        let events = config.events_for_provider(Provider::Claude);
        assert_eq!(events.len(), 2);

        let events = config.events_for_provider(Provider::Codex);
        assert!(events.is_empty());
    }

    #[test]
    fn round_trip_config() {
        let mut config = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            providers: HashMap::new(),
        };

        let mut claude_config = ProviderConfig::default();
        claude_config.events.insert(
            AgenticEvent::TurnComplete,
            EventBinding {
                enabled: true,
                actions: vec![],
                matcher: None,
            },
        );
        config.providers.insert(Provider::Claude, claude_config);

        let json = serde_json::to_string(&config).unwrap();
        let back: HookerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version, "1.0");
        assert!(back.providers.contains_key(&Provider::Claude));
    }

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
    }

    #[test]
    fn empty_providers_default() {
        let json = serde_json::json!({
            "version": "1.0"
        });
        let config: HookerConfig = serde_json::from_value(json).unwrap();
        assert!(config.providers.is_empty());
    }
}
