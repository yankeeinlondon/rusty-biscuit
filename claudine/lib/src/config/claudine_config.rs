//! Top-level configuration type for the Claudine tool.
//!
//! This module defines [`ClaudineConfig`], the canonical flat configuration
//! schema written to `~/.claudine/config.json`. It replaces the old
//! per-provider config format with a single cross-provider event map.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::actions::HookAction;
use crate::error::{ClaudineError, Result};
use crate::events::AgenticEvent;
use crate::protect::config::ProtectConfig;
use crate::provider::Provider;

// Re-exports for backward compatibility
pub use crate::config::messaging_block::{ClaudineMessengerConfig, MessengerProviderConfig};
pub use crate::config::tts::{Gender, TtsConfigSettings, TtsValue, VoiceSelection};

// ============================================================================
// DefaultSounds
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
// ClaudineConfig
// ============================================================================

/// User-supplied per-provider model override entries.
///
/// Two shapes are supported on disk:
///
/// 1. A bare string list (additive): `models.codex: ["gpt-x", "gpt-y"]`
/// 2. An explicit object: `models.codex: { mode: "replace", values: [...] }`
///
/// The bare-list shorthand always means [`ModelOverrideMode::Add`]. Use the
/// object form to fully replace the dynamically fetched catalog for a
/// provider via [`ModelOverrideMode::Replace`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProviderModelOverride {
    /// Additive shorthand: a bare list of model identifiers added to the
    /// fetched catalog.
    AddList(Vec<String>),
    /// Explicit object form supporting `mode` and `values`.
    Detailed(DetailedModelOverride),
}

/// Explicit object form of a model override entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetailedModelOverride {
    /// Override mode: additive (default) or replace.
    #[serde(default)]
    pub mode: ModelOverrideMode,

    /// Model identifiers to add or replace with.
    #[serde(default)]
    pub values: Vec<String>,
}

/// Whether a [`ProviderModelOverride`] adds to or replaces the fetched
/// catalog for a provider.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelOverrideMode {
    /// Add user-supplied entries to the fetched list.
    #[default]
    Add,
    /// Replace the fetched list entirely with user-supplied entries.
    Replace,
}

impl ProviderModelOverride {
    /// Return the configured override mode.
    pub fn mode(&self) -> ModelOverrideMode {
        match self {
            ProviderModelOverride::AddList(_) => ModelOverrideMode::Add,
            ProviderModelOverride::Detailed(detailed) => detailed.mode,
        }
    }

    /// Return the user-supplied model identifiers.
    pub fn values(&self) -> &[String] {
        match self {
            ProviderModelOverride::AddList(values) => values,
            ProviderModelOverride::Detailed(detailed) => &detailed.values,
        }
    }
}

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
    #[serde(default = "default_logging")]
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

    /// Optional per-event matcher strings.
    ///
    /// Each value is compiled at load time into a [`RuntimeMatcher`]. The
    /// loader tries to parse the string as a Darkmatter condition first
    /// (`tool_name == 'Bash' && git.branch == 'main'`); on parse failure
    /// it falls back to compiling a regex (`Bash|Edit`). Strings that are
    /// neither produce a warning and the binding fires unconditionally.
    ///
    /// [`RuntimeMatcher`]: crate::dispatch::matcher::RuntimeMatcher
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub matchers: HashMap<AgenticEvent, String>,

    /// Favorite agent provider for lazy composition operations.
    ///
    /// Stored on disk as `preferred_agent` (with `favorite_agent` accepted
    /// as a serde alias on read). When absent, composition resolution
    /// simply has one fewer signal to consider — it is never an error.
    #[serde(
        default,
        alias = "favorite_agent",
        skip_serializing_if = "Option::is_none"
    )]
    pub preferred_agent: Option<Provider>,

    /// The canonical provider for this scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_provider: Option<Provider>,

    /// Per-provider model catalog overrides.
    ///
    /// Augments or replaces the dynamically fetched model catalog used to
    /// validate frontmatter `model` hints during composition resolution.
    /// User-scope only; repo configs may not declare this field.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub models: HashMap<Provider, ProviderModelOverride>,

    /// Default sound effects for outcome categories.
    #[serde(default)]
    pub default_sounds: DefaultSounds,

    /// Whether to prompt for missing required schema properties.
    ///
    /// When `true` (the default), composition operations enter Interactive
    /// Mode to collect missing required values when stdin and stderr are
    /// TTYs. When `false`, missing required values fail with a
    /// `MissingProperties` error instead.
    ///
    /// User-scope only; repo configs may not declare this field.
    #[serde(
        default = "default_prompt_for_missing",
        skip_serializing_if = "is_true"
    )]
    pub prompt_for_missing: bool,
}

fn default_logging() -> bool {
    true
}

fn default_prompt_for_missing() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

fn default_protect() -> ProtectConfig {
    ProtectConfig::default()
}

// ============================================================================
// RepoOverrideConfig
// ============================================================================

/// Repo-scoped configuration override.
///
/// A repo config file (`{repo}/.claudine/config.json`) only contains the
/// fields that are allowed to differ per-repo. Unlike [`ClaudineConfig`],
/// all fields are optional, so a repo file that contains only
/// `{ "canonical_provider": "gemini" }` will deserialize successfully.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoOverrideConfig {
    /// Override the canonical provider for this repo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_provider: Option<Provider>,

    /// Override or extend actions for this repo (per-event replacement).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub actions: HashMap<AgenticEvent, Vec<HookAction>>,

    /// Override or extend per-event matchers for this repo.
    ///
    /// Per-event replacement: a repo entry fully replaces the user's entry
    /// for the same event. See [`ClaudineConfig::matchers`] for compilation
    /// semantics.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub matchers: HashMap<AgenticEvent, String>,

    /// Override the active messenger configuration key for this repo.
    ///
    /// - `None` (absent): no override, inherit from user config.
    /// - `Some(None)` (JSON `null`): disable messenger for this repo.
    /// - `Some(Some(key))`: use the named configuration for this repo.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_messenger_override"
    )]
    pub active_messenger: Option<Option<String>>,
}

impl RepoOverrideConfig {
    pub fn is_empty(&self) -> bool {
        self.canonical_provider.is_none()
            && self.actions.is_empty()
            && self.matchers.is_empty()
            && self.active_messenger.is_none()
    }
}

fn deserialize_optional_messenger_override<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<String> = serde::Deserialize::deserialize(deserializer)?;
    Ok(Some(value))
}

impl Default for ClaudineConfig {
    fn default() -> Self {
        Self {
            tts: TtsValue::default(),
            messenger: None,
            logging: true,
            protect: ProtectConfig::default(),
            actions: HashMap::new(),
            matchers: HashMap::new(),
            preferred_agent: None,
            canonical_provider: None,
            models: HashMap::new(),
            default_sounds: DefaultSounds::default(),
            prompt_for_missing: true,
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
        self.protect.validate()?;

        if let Some(messenger) = &self.messenger {
            messenger.validate()?;
        }

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
                    { "type": "sound_effect", "effect": "doorbell" }
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
                voice: None,
                gender: None,
                when: None,
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
        assert_eq!(config.preferred_agent, Some(Provider::Claude));
    }

    #[test]
    fn favorite_agent_alias_deserializes() {
        let json = serde_json::json!({ "favorite_agent": "codex" });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.preferred_agent, Some(Provider::Codex));
    }

    #[test]
    fn preferred_agent_absent_deserializes_to_none() {
        let json = serde_json::json!({});
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert!(config.preferred_agent.is_none());
    }

    #[test]
    fn preferred_agent_round_trip_when_set() {
        let config = ClaudineConfig {
            preferred_agent: Some(Provider::Gemini),
            ..ClaudineConfig::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        let back: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert_eq!(back.preferred_agent, Some(Provider::Gemini));
    }

    #[test]
    fn preferred_agent_round_trip_when_none_skips_field() {
        let config = ClaudineConfig::default();
        let json = serde_json::to_value(&config).unwrap();
        assert!(
            json.get("preferred_agent").is_none(),
            "preferred_agent should be omitted when None"
        );
        let back: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert!(back.preferred_agent.is_none());
    }

    #[test]
    fn canonical_provider_deserializes() {
        let json =
            serde_json::json!({ "preferred_agent": "claude", "canonical_provider": "goose" });
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
        assert!(config.logging);
        assert!(config.protect.enabled);
        assert!(config.actions.is_empty());
        assert!(config.messenger.is_none());
        assert_eq!(config.preferred_agent, Some(Provider::Claude));
        assert!(config.canonical_provider.is_none());
        assert!(config.models.is_empty());
    }

    #[test]
    fn default_impl_matches_empty_deserialization() {
        let from_default = ClaudineConfig::default();
        let from_json: ClaudineConfig = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(matches!(from_default.tts, TtsValue::Boolean(false)));
        assert!(matches!(from_json.tts, TtsValue::Boolean(false)));
        assert_eq!(from_default.logging, from_json.logging);
        assert_eq!(from_default.protect.enabled, from_json.protect.enabled);
        assert!(from_default.preferred_agent.is_none());
        assert!(from_json.preferred_agent.is_none());
        assert!(from_default.models.is_empty());
        assert!(from_json.models.is_empty());
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
    // Phase 5.1: TUI write-path fix tests
    // -------------------------------------------------------------------------

    /// Verifies that creating a sound effect action for any event using
    /// `recommended_sound` produces a valid playa sound effect name, and
    /// that the resulting config passes `validate()`.
    #[test]
    fn sound_effect_action_with_recommended_sound_is_valid() {
        use crate::events::recommended_sound;

        let all_events = [
            AgenticEvent::SessionStart,
            AgenticEvent::SessionEnd,
            AgenticEvent::BeforePrompt,
            AgenticEvent::BeforeTool,
            AgenticEvent::AfterTool,
            AgenticEvent::ToolError,
            AgenticEvent::PermissionRequest,
            AgenticEvent::HumanInTheLoop,
            AgenticEvent::TurnComplete,
            AgenticEvent::TurnError,
            AgenticEvent::SubagentStart,
            AgenticEvent::SubagentStop,
            AgenticEvent::BeforeModel,
            AgenticEvent::AfterModel,
            AgenticEvent::BeforeCompact,
            AgenticEvent::Notification,
        ];

        for event in all_events {
            let effect = recommended_sound(&event).to_string();
            assert!(
                playa::SoundEffect::from_name(&effect).is_some(),
                "recommended_sound for {event:?} returned '{effect}' which is not a valid playa sound effect"
            );

            // Build a config with this action and verify it validates
            let mut actions = HashMap::new();
            actions.insert(
                event,
                vec![HookAction::SoundEffect {
                    effect,
                    volume: 1.0,
                    speed: 1.0,
                    when: None,
                }],
            );
            let config = ClaudineConfig {
                actions,
                ..Default::default()
            };
            assert!(
                config.validate().is_ok(),
                "config with recommended sound for {event:?} should validate"
            );
        }
    }

    // -------------------------------------------------------------------------
    // Phase 5.4: Validation round-trip tests
    // -------------------------------------------------------------------------

    /// Validates that each type of invalid data is independently rejected
    /// by `validate()`.
    #[test]
    fn validate_rejects_blank_messenger_active_config() {
        let config = ClaudineConfig {
            messenger: Some(ClaudineMessengerConfig {
                active_config: Some("   ".to_string()),
                configurations: HashMap::new(),
            }),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("blank"), "error: {msg}");
    }

    #[test]
    fn validate_rejects_unknown_attention_sound() {
        let config: ClaudineConfig = serde_json::from_value(serde_json::json!({
            "preferred_agent": "claude",
            "default_sounds": {
                "attention": "nonexistent-sound-xyz"
            }
        }))
        .unwrap();
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("attention") && msg.contains("nonexistent-sound-xyz"),
            "error: {msg}"
        );
    }

    #[test]
    fn validate_rejects_unknown_error_sound() {
        let config: ClaudineConfig = serde_json::from_value(serde_json::json!({
            "preferred_agent": "claude",
            "default_sounds": {
                "error": "bogus-sound-name"
            }
        }))
        .unwrap();
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("error") && msg.contains("bogus-sound-name"),
            "error: {msg}"
        );
    }

    // -------------------------------------------------------------------------
    // RepoOverrideConfig — deny_unknown_fields
    // -------------------------------------------------------------------------

    #[test]
    fn repo_override_rejects_preferred_agent_field() {
        let result = serde_json::from_value::<RepoOverrideConfig>(serde_json::json!({
            "preferred_agent": "claude"
        }));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("preferred_agent"),
            "error should mention the unknown field: {msg}"
        );
    }

    #[test]
    fn repo_override_rejects_logging_field() {
        let result = serde_json::from_value::<RepoOverrideConfig>(serde_json::json!({
            "logging": true
        }));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("logging"),
            "error should mention the unknown field: {msg}"
        );
    }

    #[test]
    fn repo_override_accepts_canonical_provider() {
        let config = serde_json::from_value::<RepoOverrideConfig>(serde_json::json!({
            "canonical_provider": "gemini"
        }))
        .unwrap();
        assert_eq!(config.canonical_provider, Some(Provider::Gemini));
        assert!(config.actions.is_empty());
    }

    #[test]
    fn repo_override_accepts_actions() {
        let config = serde_json::from_value::<RepoOverrideConfig>(serde_json::json!({
            "actions": {
                "session_start": [
                    { "type": "sound_effect", "effect": "doorbell" }
                ]
            }
        }))
        .unwrap();
        assert!(config.canonical_provider.is_none());
        assert!(config.actions.contains_key(&AgenticEvent::SessionStart));
        assert_eq!(config.actions[&AgenticEvent::SessionStart].len(), 1);
    }

    #[test]
    fn repo_override_with_active_messenger_round_trips() {
        let json = r#"{ "active_messenger": "work-slack" }"#;
        let repo: RepoOverrideConfig = serde_json::from_str(json).unwrap();
        assert_eq!(repo.active_messenger, Some(Some("work-slack".to_string())));
        let serialized = serde_json::to_string(&repo).unwrap();
        assert!(serialized.contains("work-slack"));
    }

    #[test]
    fn repo_override_with_null_active_messenger_disables() {
        let json = r#"{ "active_messenger": null }"#;
        let repo: RepoOverrideConfig = serde_json::from_str(json).unwrap();
        assert_eq!(repo.active_messenger, Some(None));
    }

    #[test]
    fn repo_override_without_active_messenger_is_no_override() {
        let json = r#"{}"#;
        let repo: RepoOverrideConfig = serde_json::from_str(json).unwrap();
        assert_eq!(repo.active_messenger, None);
    }

    /// Comprehensive round-trip: build a fully-populated valid config,
    /// validate it, serialize, deserialize, and validate again.
    #[test]
    fn validate_round_trip_fully_populated_config() {
        let mut actions = HashMap::new();
        actions.insert(
            AgenticEvent::SessionStart,
            vec![HookAction::SoundEffect {
                effect: "doorbell".to_string(),
                volume: 0.8,
                speed: 1.0,
                when: None,
            }],
        );
        actions.insert(
            AgenticEvent::TurnComplete,
            vec![
                HookAction::Speak {
                    message: "Done!".to_string(),
                    voice: Some("Samantha".to_string()),
                    gender: Some(Gender::Female),
                    when: None,
                },
                HookAction::Report {
                    handler: None,
                    when: None,
                },
            ],
        );

        let config = ClaudineConfig {
            tts: TtsValue::Config(TtsConfigSettings {
                provider: "say".to_string(),
                voice: Some(VoiceSelection::Gendered {
                    male: "Alex".to_string(),
                    female: "Samantha".to_string(),
                }),
                gender: Gender::Male,
            }),
            messenger: Some(ClaudineMessengerConfig {
                active_config: Some("work".to_string()),
                configurations: HashMap::from([
                    (
                        "work".to_string(),
                        MessengerProviderConfig::Slack {
                            channel_id: "C0ABC".to_string(),
                            bot_token_env: "SLACK_BOT_TOKEN".to_string(),
                        },
                    ),
                    (
                        "personal".to_string(),
                        MessengerProviderConfig::Signal {
                            recipient: "+15551234567".to_string(),
                            rpc_url_env: "SIGNAL_RPC_URL".to_string(),
                            account_env: "SIGNAL_ACCOUNT".to_string(),
                        },
                    ),
                ]),
            }),
            logging: true,
            protect: ProtectConfig::default(),
            actions,
            preferred_agent: Some(Provider::Codex),
            canonical_provider: Some(Provider::Gemini),
            models: HashMap::new(),
            matchers: HashMap::new(),
            default_sounds: DefaultSounds {
                success: Some("doorbell".to_string()),
                attention: Some("bong".to_string()),
                error: Some("space-alarm".to_string()),
            },
            prompt_for_missing: true,
        };

        // First validation pass
        config.validate().expect("initial validation should pass");

        // Serialize and deserialize
        let json = serde_json::to_value(&config).unwrap();
        let roundtripped: ClaudineConfig = serde_json::from_value(json).unwrap();

        // Second validation pass
        roundtripped
            .validate()
            .expect("round-tripped validation should pass");

        // Verify key fields survived the round-trip
        assert_eq!(roundtripped.preferred_agent, Some(Provider::Codex));
        assert_eq!(roundtripped.canonical_provider, Some(Provider::Gemini));
        assert!(roundtripped.logging);
        assert_eq!(
            roundtripped.actions.len(),
            2,
            "both event action entries should survive"
        );
        assert_eq!(
            roundtripped.actions[&AgenticEvent::TurnComplete].len(),
            2,
            "multi-action event should preserve all actions"
        );
        match &roundtripped.tts {
            TtsValue::Config(cfg) => {
                assert_eq!(cfg.provider, "say");
                assert_eq!(cfg.gender, Gender::Male);
                match &cfg.voice {
                    Some(VoiceSelection::Gendered { male, female }) => {
                        assert_eq!(male, "Alex");
                        assert_eq!(female, "Samantha");
                    }
                    other => panic!("expected Gendered voice, got {other:?}"),
                }
            }
            other => panic!("expected Config TTS, got {other:?}"),
        }
        let messenger = roundtripped.messenger.unwrap();
        assert_eq!(messenger.active_config.as_deref(), Some("work"));
        assert_eq!(messenger.configurations.len(), 2);
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
                    { "type": "sound_effect", "effect": "doorbell" }
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
        assert_eq!(back.preferred_agent, Some(Provider::Claude));
        assert_eq!(back.canonical_provider, Some(Provider::Gemini));
    }

    // -------------------------------------------------------------------------
    // ProviderModelOverride / ModelOverrideMode
    // -------------------------------------------------------------------------

    #[test]
    fn models_additive_shorthand_deserializes() {
        let json = serde_json::json!({
            "models": {
                "codex": ["gpt-5.4", "gpt-5.4-mini"]
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        let entry = config
            .models
            .get(&Provider::Codex)
            .expect("codex override should be present");
        assert_eq!(entry.mode(), ModelOverrideMode::Add);
        assert_eq!(entry.values(), &["gpt-5.4", "gpt-5.4-mini"]);
    }

    #[test]
    fn models_replace_object_deserializes() {
        let json = serde_json::json!({
            "models": {
                "open_code": {
                    "mode": "replace",
                    "values": ["openrouter/auto"]
                }
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        let entry = config
            .models
            .get(&Provider::OpenCode)
            .expect("opencode override should be present");
        assert_eq!(entry.mode(), ModelOverrideMode::Replace);
        assert_eq!(entry.values(), &["openrouter/auto"]);
    }

    #[test]
    fn models_object_defaults_to_add_mode() {
        let json = serde_json::json!({
            "models": {
                "claude": { "values": ["claude-x"] }
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        let entry = config
            .models
            .get(&Provider::Claude)
            .expect("claude override should be present");
        assert_eq!(entry.mode(), ModelOverrideMode::Add);
        assert_eq!(entry.values(), &["claude-x"]);
    }

    #[test]
    fn models_round_trip_preserves_modes() {
        let mut models = HashMap::new();
        models.insert(
            Provider::Codex,
            ProviderModelOverride::AddList(vec!["a".to_string(), "b".to_string()]),
        );
        models.insert(
            Provider::OpenCode,
            ProviderModelOverride::Detailed(DetailedModelOverride {
                mode: ModelOverrideMode::Replace,
                values: vec!["openrouter/auto".to_string()],
            }),
        );
        let config = ClaudineConfig {
            models,
            ..ClaudineConfig::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        let back: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert_eq!(
            back.models.get(&Provider::Codex).unwrap().mode(),
            ModelOverrideMode::Add
        );
        assert_eq!(
            back.models.get(&Provider::OpenCode).unwrap().mode(),
            ModelOverrideMode::Replace
        );
    }

    #[test]
    fn models_field_skipped_when_empty() {
        let config = ClaudineConfig::default();
        let json = serde_json::to_value(&config).unwrap();
        assert!(
            json.get("models").is_none(),
            "empty `models` should be omitted from serialized output"
        );
    }

    #[test]
    fn models_detailed_form_rejects_unknown_field() {
        let json = serde_json::json!({
            "models": {
                "codex": {
                    "mode": "add",
                    "values": ["x"],
                    "extra": true
                }
            }
        });
        let result = serde_json::from_value::<ClaudineConfig>(json);
        assert!(
            result.is_err(),
            "unknown field inside detailed override should be rejected"
        );
    }

    // -------------------------------------------------------------------------
    // RepoOverrideConfig — preferred_agent/favorite_agent/models rejection
    // -------------------------------------------------------------------------

    #[test]
    fn repo_override_rejects_favorite_agent_field() {
        let result = serde_json::from_value::<RepoOverrideConfig>(serde_json::json!({
            "favorite_agent": "claude"
        }));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("favorite_agent"),
            "error should mention the unknown field: {msg}"
        );
    }

    #[test]
    fn repo_override_rejects_models_field() {
        let result = serde_json::from_value::<RepoOverrideConfig>(serde_json::json!({
            "models": { "codex": ["foo"] }
        }));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("models"),
            "error should mention the unknown field: {msg}"
        );
    }

    // -------------------------------------------------------------------------
    // prompt_for_missing
    // -------------------------------------------------------------------------

    #[test]
    fn prompt_for_missing_defaults_to_true_when_absent() {
        let config: ClaudineConfig = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(config.prompt_for_missing);
    }

    #[test]
    fn prompt_for_missing_deserializes_false() {
        let config: ClaudineConfig =
            serde_json::from_value(serde_json::json!({ "prompt_for_missing": false })).unwrap();
        assert!(!config.prompt_for_missing);
    }

    #[test]
    fn prompt_for_missing_omitted_when_true() {
        let config = ClaudineConfig::default();
        let json = serde_json::to_value(&config).unwrap();
        assert!(
            json.get("prompt_for_missing").is_none(),
            "prompt_for_missing should be omitted when true (the default)"
        );
    }

    #[test]
    fn prompt_for_missing_serialized_when_false() {
        let config = ClaudineConfig {
            prompt_for_missing: false,
            ..ClaudineConfig::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(
            json.get("prompt_for_missing").and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    #[test]
    fn prompt_for_missing_round_trips() {
        let config = ClaudineConfig {
            prompt_for_missing: false,
            ..ClaudineConfig::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        let back: ClaudineConfig = serde_json::from_value(json).unwrap();
        assert!(!back.prompt_for_missing);
    }

    #[test]
    fn repo_override_rejects_prompt_for_missing_field() {
        let result = serde_json::from_value::<RepoOverrideConfig>(serde_json::json!({
            "prompt_for_missing": false
        }));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("prompt_for_missing"),
            "error should mention the unknown field: {msg}"
        );
    }
}
