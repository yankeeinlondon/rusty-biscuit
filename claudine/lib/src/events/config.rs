use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::agentic_event::AgenticEvent;
use super::provider::Provider;
use crate::actions::{HookAction, LogTarget};
use crate::error::{ClaudineError, Result};
use crate::messaging::ScopedMessagingSettings;
use crate::services::protect::ProtectConfig;

/// Root configuration loaded from `~/.claudine/config.json`.
///
/// The config is organized per-provider, with each provider having its own
/// set of event bindings. This allows different providers to have different
/// events configured with different actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    /// Event bindings for this provider.
    #[serde(default)]
    pub events: HashMap<AgenticEvent, EventBinding>,
}

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

    /// Validate semantic config invariants that are not expressible in serde types.
    pub fn validate(&self) -> Result<()> {
        if let Some(protect) = self.settings.protect.as_ref() {
            protect.validate().map_err(|error| {
                ClaudineError::ConfigValidation(format!("invalid settings.protect: {error}"))
            })?;
        }

        if let Some(messaging) = self.settings.messaging.as_ref() {
            messaging.validate("config").map_err(|error| {
                ClaudineError::ConfigValidation(format!("invalid settings.messaging: {error}"))
            })?;
        }

        Ok(())
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
                    "type": "file",
                    "path": "~/.claudine/events.jsonl",
                    "rotate_daily": false
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
                                        "template": "[TOOL] {{tool_name}}: executing"
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
                actions: vec![HookAction::Speak {
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
        assert!(settings.linking.is_none());
        assert!(settings.protect.is_none());
    }

    #[test]
    fn linking_settings_round_trip() {
        let mut config = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            providers: HashMap::new(),
        };
        config.settings.linking = Some(LinkingSettings {
            preference: vec![Provider::Codex, Provider::Claude],
            canonical_provider: CanonicalProviderSettings {
                user_skill: Some(Provider::Codex),
                repo_skill: Some(Provider::Claude),
                ..CanonicalProviderSettings::default()
            },
        });

        let serialized = serde_json::to_string(&config).unwrap();
        let restored: HookerConfig = serde_json::from_str(&serialized).unwrap();
        let linking = restored.settings.linking.unwrap();

        assert_eq!(linking.preference, vec![Provider::Codex, Provider::Claude]);
        assert_eq!(linking.canonical_provider.user_skill, Some(Provider::Codex));
        assert_eq!(
            linking.canonical_provider.repo_skill,
            Some(Provider::Claude)
        );
    }

    #[test]
    fn empty_providers_default() {
        let json = serde_json::json!({
            "version": "1.0"
        });
        let config: HookerConfig = serde_json::from_value(json).unwrap();
        assert!(config.providers.is_empty());
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

    #[test]
    fn validate_rejects_invalid_protect_settings() {
        let config = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                protect: Some(ProtectConfig {
                    completion: crate::services::protect::CompletionPolicy {
                        enabled: true,
                        max_retries: 0,
                        ..crate::services::protect::CompletionPolicy::default()
                    },
                    ..ProtectConfig::default()
                }),
                ..GlobalSettings::default()
            },
            providers: HashMap::new(),
        };

        let error = config.validate().unwrap_err().to_string();
        assert!(error.contains("invalid settings.protect"));
    }

    #[test]
    fn full_config_example_with_messaging_parses() {
        let json = serde_json::json!({
            "version": "1.0",
            "settings": {
                "messaging": {
                    "active": "work-slack",
                    "configs": {
                        "work-slack": {
                            "provider": "slack",
                            "channel_id": "C012345ABC",
                            "bot_token_env": "SLACK_BOT_TOKEN"
                        },
                        "personal-discord": {
                            "provider": "discord",
                            "channel_id": "123456789012345678",
                            "bot_token_env": "DISCORD_BOT_TOKEN"
                        }
                    }
                }
            },
            "providers": {}
        });

        let config: HookerConfig = serde_json::from_value(json).unwrap();
        let messaging = config.settings.messaging.as_ref().unwrap();
        assert_eq!(messaging.active.as_deref(), Some("work-slack"));
        assert_eq!(messaging.configs.len(), 2);
        config.validate().unwrap();
    }
}
