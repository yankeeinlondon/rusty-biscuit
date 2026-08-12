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
use crate::runaway::{validate_exit_expressions, ExitExpressionsValue, GuardSettings};

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
    pub values: Vec<ModelOverrideValue>,
}

/// One entry in a [`DetailedModelOverride`]'s `values` list.
///
/// Two on-disk shapes are supported:
///
/// 1. A bare string: just the model identifier (the common shorthand).
/// 2. An object: `{ "id": "...", "catalog_id": "..." }` where `catalog_id`
///    is optional.
///
/// A `catalog_id` is an unchained-ai models-catalog identity key (e.g.
/// `anthropic/claude-opus@4.8`). Claudine treats it as an opaque join key —
/// its grammar is not validated here — so a user-added model can still join
/// catalog identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelOverrideValue {
    /// Bare-string shorthand carrying only the model identifier.
    Plain(String),
    /// Object form with an optional catalog identity join key.
    Entry(ModelOverrideEntry),
}

/// Object form of a [`ModelOverrideValue`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelOverrideEntry {
    /// Model identifier as passed to the provider.
    pub id: String,

    /// Optional models-catalog identity key (opaque join key).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_id: Option<String>,
}

impl ModelOverrideValue {
    /// Return the model identifier.
    pub fn id(&self) -> &str {
        match self {
            ModelOverrideValue::Plain(id) => id,
            ModelOverrideValue::Entry(entry) => &entry.id,
        }
    }

    /// Return the models-catalog identity key, if configured.
    pub fn catalog_id(&self) -> Option<&str> {
        match self {
            ModelOverrideValue::Plain(_) => None,
            ModelOverrideValue::Entry(entry) => entry.catalog_id.as_deref(),
        }
    }
}

impl From<String> for ModelOverrideValue {
    fn from(id: String) -> Self {
        ModelOverrideValue::Plain(id)
    }
}

impl From<&str> for ModelOverrideValue {
    fn from(id: &str) -> Self {
        ModelOverrideValue::Plain(id.to_string())
    }
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
    pub fn values(&self) -> Vec<&str> {
        match self {
            ProviderModelOverride::AddList(values) => {
                values.iter().map(String::as_str).collect()
            }
            ProviderModelOverride::Detailed(detailed) => {
                detailed.values.iter().map(ModelOverrideValue::id).collect()
            }
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
    #[serde(default = "default_prompt_for_missing", skip_serializing_if = "is_true")]
    pub prompt_for_missing: bool,

    /// Opt-in harvest of unmatched error/warning-class signal payloads.
    ///
    /// When `true`, wrapped runs append scrubbed payloads that fired no
    /// detection record to `~/.claudine/harvest/<provider>/<date>.jsonl`
    /// as candidate detection-record evidence (see
    /// `claudine::signals::harvest`). The `CLAUDINE_HARVEST` env var
    /// (`1`/`true`/`0`/`false`) overrides this value. Config-file + env
    /// only for v1 — deliberately not surfaced in the config TUI.
    ///
    /// User-scope only; repo configs may not declare this field.
    #[serde(default, skip_serializing_if = "is_false")]
    pub harvest_unmatched: bool,

    /// Exit-expression rules for the runaway-output guard (Cluster E).
    ///
    /// Accepts either a bare array (uses the user layer's semantics — it
    /// is the base set) or an object `{ mode, rules }` (explicit mode).
    /// Defaults to `None` (empty set — exit-expressions ship purely
    /// user-authored, never pre-populated).
    ///
    /// Repo configs may declare their own `exit_expressions`; the repo
    /// layer combines with this one per its own combine mode. The
    /// frontmatter layer is parsed from the composition document and
    /// participates in resolution at run time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_expressions: Option<ExitExpressionsValue>,

    /// Scalar knobs for the repetition + volume guards (Cluster F).
    ///
    /// Follows last-writer precedence (frontmatter > repo > user >
    /// built-in) like `timeout` / `step_timeout` — there is no combine
    /// mode here, only the list-typed `exit_expressions` carries one.
    #[serde(default, skip_serializing_if = "is_default_guard_settings")]
    pub guard_settings: GuardSettings,
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

fn is_false(value: &bool) -> bool {
    !*value
}

fn default_protect() -> ProtectConfig {
    ProtectConfig::default()
}

/// Skip-serializing helper for [`GuardSettings`]: omit the field when it
/// matches the built-in defaults so a clean user config stays minimal.
fn is_default_guard_settings(value: &GuardSettings) -> bool {
    *value == GuardSettings::default()
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

    /// Repo-scoped exit-expression rules (Cluster E1).
    ///
    /// The repo layer combines with the user layer per its own combine
    /// mode (default [`crate::runaway::LayerMode::Override`] so every
    /// contributor gets identical guard behavior; may opt into
    /// [`crate::runaway::LayerMode::Merge`] for additive rules).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_expressions: Option<ExitExpressionsValue>,

    /// Repo-scoped scalar guard settings (Cluster F).
    ///
    /// If present, fully replaces the user-scope [`GuardSettings`] (the
    /// sub-fields are not individually merged).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard_settings: Option<GuardSettings>,
}

impl RepoOverrideConfig {
    pub fn is_empty(&self) -> bool {
        self.canonical_provider.is_none()
            && self.actions.is_empty()
            && self.matchers.is_empty()
            && self.active_messenger.is_none()
            && self.exit_expressions.is_none()
            && self.guard_settings.is_none()
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
            harvest_unmatched: false,
            exit_expressions: None,
            guard_settings: GuardSettings::default(),
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
    /// - `exit_expressions` regexes compile and `scope` strings reference
    ///   known `Provider` variants (Cluster E2 / E3a — fail at
    ///   config-load, never mid-stream)
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

        if let Some(exit_expressions) = &self.exit_expressions {
            validate_exit_expressions(exit_expressions.rules())?;
        }

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
mod tests;
