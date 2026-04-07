//! Top-level configuration type for the Claudine tool.
//!
//! This module defines [`ClaudineConfig`], the canonical flat configuration
//! schema written to `~/.claudine/config.json`. It replaces the old
//! per-provider `HookerConfig` with a single cross-provider event map.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::actions::HookAction;
use crate::error::{ClaudineError, Result};
use crate::events::{AgenticEvent, Provider};
use crate::services::protect::config::ProtectConfig;

// ============================================================================
// Default env var name helpers
// ============================================================================

fn default_discord_bot_token() -> String {
    "DISCORD_BOT_TOKEN".to_string()
}

fn default_slack_bot_token() -> String {
    "SLACK_BOT_TOKEN".to_string()
}

fn default_signal_rpc_url() -> String {
    "SIGNAL_RPC_URL".to_string()
}

fn default_signal_account() -> String {
    "SIGNAL_ACCOUNT".to_string()
}

fn default_whatsapp_access_token() -> String {
    "WHATSAPP_ACCESS_TOKEN".to_string()
}

fn default_whatsapp_phone_number_id() -> String {
    "WHATSAPP_PHONE_NUMBER_ID".to_string()
}

// ============================================================================
// TTS types
// ============================================================================

/// Gender preference for TTS voice selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
}

fn default_gender() -> Gender {
    Gender::Female
}

/// Selects a TTS voice either as a single fixed voice ID or as gendered aliases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VoiceSelection {
    /// A single fixed voice ID string.
    Single(String),
    /// Separate voice IDs for male and female output.
    Gendered { male: String, female: String },
}

/// Full TTS configuration when more than a boolean toggle is needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsConfigSettings {
    /// Which TTS provider to use (e.g., "say", "espeak", "elevenlabs").
    ///
    /// Resolved to a `TtsProvider` variant at runtime in the dispatch runner.
    pub provider: String,

    /// Optional voice selection (single ID or gendered aliases).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<VoiceSelection>,

    /// Preferred gender when the voice is auto-selected.
    #[serde(default = "default_gender")]
    pub gender: Gender,
}

/// TTS configuration: either a simple boolean toggle or full settings.
///
/// `"tts": true` enables TTS with provider auto-detection.
/// `"tts": false` disables TTS entirely.
/// `"tts": { "provider": ..., "voice": ..., "gender": ... }` sets explicit options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TtsValue {
    /// Boolean shorthand: enable or disable TTS.
    Boolean(bool),
    /// Full TTS settings.
    Config(TtsConfigSettings),
}

impl Default for TtsValue {
    fn default() -> Self {
        TtsValue::Boolean(false)
    }
}

// ============================================================================
// Default sounds
// ============================================================================

/// Default sound effects to play for common outcome categories.
///
/// Each field is an optional sound effect name as understood by
/// `playa::SoundEffect::from_name`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultSounds {
    /// Sound to play on successful completion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<String>,

    /// Sound to play when user attention is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention: Option<String>,

    /// Sound to play when an error occurs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// Messenger config
// ============================================================================

/// Provider-specific configuration for a single messaging route.
///
/// Serializes/deserializes with a `"provider"` tag and snake_case variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case", deny_unknown_fields)]
pub enum MessengerProviderConfig {
    /// Discord bot configuration.
    Discord {
        /// Discord channel ID to send messages to.
        channel_id: String,
        /// Environment variable holding the bot token.
        #[serde(default = "default_discord_bot_token")]
        bot_token_env: String,
    },
    /// Slack bot configuration.
    Slack {
        /// Slack channel ID to send messages to.
        channel_id: String,
        /// Environment variable holding the bot token.
        #[serde(default = "default_slack_bot_token")]
        bot_token_env: String,
    },
    /// Signal messenger configuration.
    Signal {
        /// Recipient phone number or group ID.
        recipient: String,
        /// Environment variable holding the RPC URL.
        #[serde(default = "default_signal_rpc_url")]
        rpc_url_env: String,
        /// Environment variable holding the account.
        #[serde(default = "default_signal_account")]
        account_env: String,
    },
    /// WhatsApp Business API configuration.
    Whatsapp {
        /// Recipient phone number.
        recipient: String,
        /// Environment variable holding the access token.
        #[serde(default = "default_whatsapp_access_token")]
        access_token_env: String,
        /// Environment variable holding the phone number ID.
        #[serde(default = "default_whatsapp_phone_number_id")]
        phone_number_id_env: String,
    },
}

/// Messenger settings: named configurations and an active selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudineMessengerConfig {
    /// Key of the currently active messenger configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_config: Option<String>,

    /// Named messenger route configurations.
    #[serde(default)]
    pub configurations: HashMap<String, MessengerProviderConfig>,
}

impl ClaudineMessengerConfig {
    /// Validate that `active_config` references an existing key in `configurations`.
    fn validate(&self) -> Result<()> {
        if let Some(active) = &self.active_config {
            if active.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(
                    "messenger.active_config cannot be blank".to_string(),
                ));
            }
            if !self.configurations.contains_key(active) {
                return Err(ClaudineError::ConfigValidation(format!(
                    "messenger.active_config '{active}' not found in messenger.configurations"
                )));
            }
        }
        Ok(())
    }
}

// ============================================================================
// ClaudineConfig
// ============================================================================

/// Top-level configuration for the Claudine tool.
///
/// Written to `~/.claudine/config.json` (JSON5 is also accepted).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudineConfig {
    /// Text-to-speech configuration.
    ///
    /// `true` enables TTS with auto-detection; `false` disables it.
    /// An object enables TTS with explicit provider/voice settings.
    #[serde(default)]
    pub tts: TtsValue,

    /// Messaging platform configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messenger: Option<ClaudineMessengerConfig>,

    /// Whether the logging service is enabled.
    #[serde(default)]
    pub logging: bool,

    /// Protect service configuration.
    ///
    /// `true` enables protect with defaults; `false` disables it.
    /// An object allows fine-grained rule control.
    #[serde(default = "default_protect")]
    pub protect: ProtectConfig,

    /// Actions bound to canonical Claudine events (cross-provider).
    #[serde(default)]
    pub actions: HashMap<AgenticEvent, Vec<HookAction>>,

    /// Preferred agent provider for lazy composition operations.
    pub preferred_agent: Provider,

    /// The canonical provider for this scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_provider: Option<Provider>,

    /// Default sound effects for outcome categories.
    #[serde(default)]
    pub default_sounds: DefaultSounds,
}

fn default_protect() -> ProtectConfig {
    ProtectConfig::default()
}

impl Default for ClaudineConfig {
    fn default() -> Self {
        Self {
            tts: TtsValue::default(),
            messenger: None,
            logging: false,
            protect: ProtectConfig::default(),
            actions: HashMap::new(),
            preferred_agent: Provider::Claude,
            canonical_provider: None,
            default_sounds: DefaultSounds::default(),
        }
    }
}

impl ClaudineConfig {
    /// Validate the configuration for semantic correctness.
    ///
    /// Checks:
    /// - `protect` rules are valid
    /// - `messenger.active_config` references an existing key
    /// - `default_sounds` names are valid `playa::SoundEffect` names
    ///
    /// ## Errors
    ///
    /// Returns [`ClaudineError::ConfigValidation`] if any check fails.
    pub fn validate(&self) -> Result<()> {
        // Validate protect config
        self.protect.validate()?;

        // Validate messenger active_config reference
        if let Some(messenger) = &self.messenger {
            messenger.validate()?;
        }

        // Validate default sound effect names
        validate_sound_name("default_sounds.success", &self.default_sounds.success)?;
        validate_sound_name("default_sounds.attention", &self.default_sounds.attention)?;
        validate_sound_name("default_sounds.error", &self.default_sounds.error)?;

        Ok(())
    }
}

/// Validate a sound effect name using `playa::SoundEffect::from_name`.
fn validate_sound_name(field: &str, name: &Option<String>) -> Result<()> {
    let Some(name) = name else {
        return Ok(());
    };
    if playa::SoundEffect::from_name(name).is_none() {
        return Err(ClaudineError::ConfigValidation(format!(
            "{field}: unknown sound effect name '{name}'"
        )));
    }
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // TtsValue
    // -------------------------------------------------------------------------

    #[test]
    fn tts_boolean_true_deserializes() {
        let config: ClaudineConfig =
            serde_json::from_value(serde_json::json!({ "preferred_agent": "claude", "tts": true }))
                .unwrap();
        assert!(matches!(config.tts, TtsValue::Boolean(true)));
    }

    #[test]
    fn tts_boolean_false_deserializes() {
        let config: ClaudineConfig =
            serde_json::from_value(serde_json::json!({ "preferred_agent": "claude", "tts": false }))
                .unwrap();
        assert!(matches!(config.tts, TtsValue::Boolean(false)));
    }

    #[test]
    fn tts_config_settings_deserializes() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "tts": {
                "provider": "say",
                "gender": "male"
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        match &config.tts {
            TtsValue::Config(settings) => {
                assert_eq!(settings.provider, "say");
                assert_eq!(settings.gender, Gender::Male);
                assert!(settings.voice.is_none());
            }
            other => panic!("expected Config variant, got {other:?}"),
        }
    }

    #[test]
    fn tts_config_with_single_voice() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "tts": {
                "provider": "espeak",
                "voice": "Samantha"
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        match &config.tts {
            TtsValue::Config(settings) => match &settings.voice {
                Some(VoiceSelection::Single(v)) => assert_eq!(v, "Samantha"),
                other => panic!("expected Single voice, got {other:?}"),
            },
            other => panic!("expected Config variant, got {other:?}"),
        }
    }

    #[test]
    fn tts_config_with_gendered_voice() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "tts": {
                "provider": "elevenlabs",
                "voice": { "male": "Alex", "female": "Samantha" }
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        match &config.tts {
            TtsValue::Config(settings) => match &settings.voice {
                Some(VoiceSelection::Gendered { male, female }) => {
                    assert_eq!(male, "Alex");
                    assert_eq!(female, "Samantha");
                }
                other => panic!("expected Gendered voice, got {other:?}"),
            },
            other => panic!("expected Config variant, got {other:?}"),
        }
    }

    // -------------------------------------------------------------------------
    // Protect (boolean shorthand via ProtectConfig custom Deserialize)
    // -------------------------------------------------------------------------

    #[test]
    fn protect_boolean_true_deserializes() {
        let config: ClaudineConfig = serde_json::from_value(
            serde_json::json!({ "preferred_agent": "claude", "protect": true }),
        )
        .unwrap();
        assert!(config.protect.enabled);
    }

    #[test]
    fn protect_boolean_false_deserializes() {
        let config: ClaudineConfig = serde_json::from_value(
            serde_json::json!({ "preferred_agent": "claude", "protect": false }),
        )
        .unwrap();
        assert!(!config.protect.enabled);
    }

    #[test]
    fn protect_expanded_form_deserializes() {
        let config: ClaudineConfig = serde_json::from_value(serde_json::json!({
            "preferred_agent": "claude",
            "protect": {
                "enabled": true,
                "rules": { "git_destructive": false }
            }
        }))
        .unwrap();
        assert!(config.protect.enabled);
    }

    // -------------------------------------------------------------------------
    // Actions map
    // -------------------------------------------------------------------------

    #[test]
    fn actions_map_snake_case_keys() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "actions": {
                "session_start": [
                    { "type": "sound_effect", "name": "doorbell" }
                ],
                "turn_complete": [
                    { "type": "speak", "message": "Done!" }
                ]
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert!(config.actions.contains_key(&AgenticEvent::SessionStart));
        assert!(config.actions.contains_key(&AgenticEvent::TurnComplete));
        assert_eq!(config.actions[&AgenticEvent::SessionStart].len(), 1);
    }

    #[test]
    fn actions_map_round_trip() {
        let mut actions = HashMap::new();
        actions.insert(
            AgenticEvent::BeforeTool,
            vec![HookAction::Speak {
                message: "tool starting".to_string(),
            }],
        );
        let config = ClaudineConfig {
            actions,
            ..Default::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        let back: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert!(back.actions.contains_key(&AgenticEvent::BeforeTool));
    }

    // -------------------------------------------------------------------------
    // Messenger config
    // -------------------------------------------------------------------------

    #[test]
    fn messenger_discord_deserializes_with_env_default() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "messenger": {
                "active_config": "work",
                "configurations": {
                    "work": {
                        "provider": "discord",
                        "channel_id": "123456789"
                    }
                }
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        let messenger = config.messenger.unwrap();
        assert_eq!(messenger.active_config.as_deref(), Some("work"));
        match messenger.configurations.get("work").unwrap() {
            MessengerProviderConfig::Discord {
                channel_id,
                bot_token_env,
                ..
            } => {
                assert_eq!(channel_id, "123456789");
                assert_eq!(bot_token_env, "DISCORD_BOT_TOKEN");
            }
            other => panic!("expected Discord, got {other:?}"),
        }
    }

    #[test]
    fn messenger_slack_deserializes() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "messenger": {
                "configurations": {
                    "alerts": {
                        "provider": "slack",
                        "channel_id": "C0ABC"
                    }
                }
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        let messenger = config.messenger.unwrap();
        assert!(messenger.active_config.is_none());
        match messenger.configurations.get("alerts").unwrap() {
            MessengerProviderConfig::Slack {
                channel_id,
                bot_token_env,
                ..
            } => {
                assert_eq!(channel_id, "C0ABC");
                assert_eq!(bot_token_env, "SLACK_BOT_TOKEN");
            }
            other => panic!("expected Slack, got {other:?}"),
        }
    }

    #[test]
    fn messenger_signal_deserializes_with_defaults() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "messenger": {
                "configurations": {
                    "personal": {
                        "provider": "signal",
                        "recipient": "+15551234567"
                    }
                }
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        let messenger = config.messenger.unwrap();
        match messenger.configurations.get("personal").unwrap() {
            MessengerProviderConfig::Signal {
                recipient,
                rpc_url_env,
                account_env,
                ..
            } => {
                assert_eq!(recipient, "+15551234567");
                assert_eq!(rpc_url_env, "SIGNAL_RPC_URL");
                assert_eq!(account_env, "SIGNAL_ACCOUNT");
            }
            other => panic!("expected Signal, got {other:?}"),
        }
    }

    #[test]
    fn messenger_whatsapp_deserializes_with_defaults() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "messenger": {
                "configurations": {
                    "biz": {
                        "provider": "whatsapp",
                        "recipient": "+15559876543"
                    }
                }
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        let messenger = config.messenger.unwrap();
        match messenger.configurations.get("biz").unwrap() {
            MessengerProviderConfig::Whatsapp {
                recipient,
                access_token_env,
                phone_number_id_env,
                ..
            } => {
                assert_eq!(recipient, "+15559876543");
                assert_eq!(access_token_env, "WHATSAPP_ACCESS_TOKEN");
                assert_eq!(phone_number_id_env, "WHATSAPP_PHONE_NUMBER_ID");
            }
            other => panic!("expected Whatsapp, got {other:?}"),
        }
    }

    #[test]
    fn messenger_round_trip() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "messenger": {
                "active_config": "main",
                "configurations": {
                    "main": {
                        "provider": "slack",
                        "channel_id": "C999"
                    }
                }
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json.clone()).unwrap();
        let serialized = serde_json::to_value(&config).unwrap();
        let back: ClaudineConfig = serde_json::from_value(serialized).unwrap();
        let messenger = back.messenger.unwrap();
        assert_eq!(messenger.active_config.as_deref(), Some("main"));
    }

    // -------------------------------------------------------------------------
    // DefaultSounds
    // -------------------------------------------------------------------------

    #[test]
    fn default_sounds_deserializes() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "default_sounds": {
                "success": "doorbell",
                "error": "space-alarm"
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.default_sounds.success.as_deref(), Some("doorbell"));
        assert_eq!(config.default_sounds.error.as_deref(), Some("space-alarm"));
        assert!(config.default_sounds.attention.is_none());
    }

    #[test]
    fn default_sounds_all_fields() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "default_sounds": {
                "success": "doorbell",
                "attention": "bong",
                "error": "space-alarm"
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.default_sounds.success.as_deref(), Some("doorbell"));
        assert_eq!(config.default_sounds.attention.as_deref(), Some("bong"));
        assert_eq!(config.default_sounds.error.as_deref(), Some("space-alarm"));
    }

    // -------------------------------------------------------------------------
    // Preferred agent / canonical provider
    // -------------------------------------------------------------------------

    #[test]
    fn preferred_agent_deserializes() {
        let json = serde_json::json!({ "preferred_agent": "claude" });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.preferred_agent, Provider::Claude);
    }

    #[test]
    fn canonical_provider_deserializes() {
        let json = serde_json::json!({ "preferred_agent": "claude", "canonical_provider": "goose" });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.canonical_provider, Some(Provider::Goose));
    }

    // -------------------------------------------------------------------------
    // Minimal / defaults
    // -------------------------------------------------------------------------

    #[test]
    fn minimal_config_deserializes_with_defaults() {
        let config: ClaudineConfig =
            serde_json::from_value(serde_json::json!({ "preferred_agent": "claude" })).unwrap();
        assert!(matches!(config.tts, TtsValue::Boolean(false)));
        assert!(!config.logging);
        assert!(config.protect.enabled);
        assert!(config.actions.is_empty());
        assert!(config.messenger.is_none());
        assert_eq!(config.preferred_agent, Provider::Claude);
        assert!(config.canonical_provider.is_none());
    }

    #[test]
    fn default_impl_matches_minimal_deserialization() {
        let from_default = ClaudineConfig::default();
        let from_json: ClaudineConfig =
            serde_json::from_value(serde_json::json!({ "preferred_agent": "claude" })).unwrap();
        // Both should have TtsValue::Boolean(false)
        assert!(matches!(from_default.tts, TtsValue::Boolean(false)));
        assert!(matches!(from_json.tts, TtsValue::Boolean(false)));
        assert_eq!(from_default.logging, from_json.logging);
        assert_eq!(from_default.protect.enabled, from_json.protect.enabled);
        assert_eq!(from_default.preferred_agent, Provider::Claude);
        assert_eq!(from_json.preferred_agent, Provider::Claude);
    }

    // -------------------------------------------------------------------------
    // Reject unknown fields
    // -------------------------------------------------------------------------

    #[test]
    fn rejects_unknown_top_level_field() {
        let result = serde_json::from_value::<ClaudineConfig>(serde_json::json!({
            "unknown_field": true
        }));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("unknown_field"), "error: {msg}");
    }

    #[test]
    fn rejects_unknown_messenger_field() {
        let result = serde_json::from_value::<ClaudineConfig>(serde_json::json!({
            "messenger": {
                "active_config": "x",
                "typo_field": true
            }
        }));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_default_sounds_field() {
        let result = serde_json::from_value::<ClaudineConfig>(serde_json::json!({
            "default_sounds": {
                "success": "doorbell",
                "warning": "bong"
            }
        }));
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // Validate method
    // -------------------------------------------------------------------------

    #[test]
    fn validate_accepts_minimal_config() {
        let config = ClaudineConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_missing_active_messenger_config() {
        let config: ClaudineConfig = serde_json::from_value(serde_json::json!({
            "preferred_agent": "claude",
            "messenger": {
                "active_config": "nonexistent",
                "configurations": {}
            }
        }))
        .unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("nonexistent"), "error: {msg}");
    }

    #[test]
    fn validate_accepts_valid_messenger_active_config() {
        let config: ClaudineConfig = serde_json::from_value(serde_json::json!({
            "preferred_agent": "claude",
            "messenger": {
                "active_config": "main",
                "configurations": {
                    "main": {
                        "provider": "discord",
                        "channel_id": "999"
                    }
                }
            }
        }))
        .unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_accepts_messenger_with_no_active_config() {
        let config: ClaudineConfig = serde_json::from_value(serde_json::json!({
            "preferred_agent": "claude",
            "messenger": {
                "configurations": {
                    "route1": {
                        "provider": "slack",
                        "channel_id": "C123"
                    }
                }
            }
        }))
        .unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_invalid_protect_config() {
        let config: ClaudineConfig = serde_json::from_value(serde_json::json!({
            "preferred_agent": "claude",
            "protect": {
                "rules": {
                    "git_destructive": {
                        "enabled": true,
                        "allow_paths": ["something"]
                    }
                }
            }
        }))
        .unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_unknown_success_sound() {
        let config: ClaudineConfig = serde_json::from_value(serde_json::json!({
            "preferred_agent": "claude",
            "default_sounds": {
                "success": "this-sound-does-not-exist-xyz-abc"
            }
        }))
        .unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("success") && msg.contains("this-sound-does-not-exist-xyz-abc"),
            "error: {msg}"
        );
    }

    #[test]
    fn validate_accepts_known_sound_names() {
        let config: ClaudineConfig = serde_json::from_value(serde_json::json!({
            "preferred_agent": "claude",
            "default_sounds": {
                "success": "doorbell",
                "attention": "bong",
                "error": "space-alarm"
            }
        }))
        .unwrap();
        assert!(config.validate().is_ok());
    }

    // -------------------------------------------------------------------------
    // Full round-trip
    // -------------------------------------------------------------------------

    #[test]
    fn full_config_round_trip() {
        let json = serde_json::json!({
            "tts": false,
            "logging": true,
            "protect": true,
            "preferred_agent": "claude",
            "canonical_provider": "gemini",
            "actions": {
                "session_start": [
                    { "type": "sound_effect", "name": "doorbell" }
                ]
            },
            "messenger": {
                "active_config": "work",
                "configurations": {
                    "work": {
                        "provider": "slack",
                        "channel_id": "C0ABC"
                    }
                }
            },
            "default_sounds": {
                "success": "doorbell"
            }
        });

        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert!(config.validate().is_ok());

        let serialized = serde_json::to_value(&config).unwrap();
        let back: ClaudineConfig = serde_json::from_value(serialized).unwrap();
        assert!(back.validate().is_ok());
        assert!(back.logging);
        assert!(back.protect.enabled);
        assert_eq!(back.preferred_agent, Provider::Claude);
        assert_eq!(back.canonical_provider, Some(Provider::Gemini));
    }
}
