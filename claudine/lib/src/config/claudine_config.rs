//! Top-level configuration type for the Claudine tool.
//!
//! This module defines [`ClaudineConfig`], the user file
//! `~/.claudine/config.yaml`, and [`RepoOverrideConfig`], the repository file
//! `{repo-root}/.claudine/config.yaml`. Their shape and the user file's
//! defaults are the shipped schema's ([`super::schema`]): a file is judged
//! and its defaults put in place ([`super::load`]) before serde reads it, so
//! the serde attributes below never decide a file's defaults.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::actions::HookAction;
use crate::error::{ClaudineError, Result};
use crate::events::AgenticEvent;
use crate::protect::config::ProtectConfig;
use crate::provider::Provider;
use crate::runaway::{validate_exit_expressions, ExitExpressionsValue, GuardSettings};
use crate::steering::automatic::SteeringConfig;

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

/// Top-level configuration for the Claudine tool: `~/.claudine/config.yaml`.
///
/// Every field without `Option` is required here: the schema's defaults are
/// in place before serde reads a file ([`Self::from_user_document`]), and
/// [`Default`] is the schema's projection of an empty file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudineConfig {
    /// Text-to-speech configuration; see [`TtsValue`] for what each shape
    /// means for hook speech, lifecycle playback, and pre-warm.
    pub tts: TtsValue,

    /// Messaging platform configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messenger: Option<ClaudineMessengerConfig>,

    /// Whether the logging service is enabled.
    pub logging: bool,

    /// Protect service configuration.
    ///
    /// `true` enables protect with defaults; `false` disables it.
    /// An object allows fine-grained rule control.
    pub protect: ProtectConfig,

    /// Actions bound to canonical Claudine events (cross-provider).
    ///
    /// Written in the file as keyed actions (`- bash: { command: … }`); an
    /// event whose list is `null` has no actions.
    #[serde(with = "super::wire::actions")]
    pub actions: HashMap<AgenticEvent, Vec<HookAction>>,

    /// Optional per-event matcher strings.
    ///
    /// Each value is compiled at load time into a [`RuntimeMatcher`]. The
    /// loader tries to parse the string as a Darkmatter condition first
    /// (`tool_name == 'Bash' && git.branch == 'main'`); on parse failure
    /// it falls back to compiling a regex (`Bash|Edit`). Strings that are
    /// neither produce a warning and the binding fires unconditionally.
    ///
    /// An event whose matcher is `null` has none.
    ///
    /// [`RuntimeMatcher`]: crate::dispatch::matcher::RuntimeMatcher
    #[serde(deserialize_with = "super::wire::null_entries_unset", skip_serializing_if = "HashMap::is_empty")]
    pub matchers: HashMap<AgenticEvent, String>,

    /// Favorite agent provider for lazy composition operations.
    ///
    /// Stored on disk as `preferred_agent` (with `favorite_agent` accepted
    /// as a serde alias on read). When absent, composition resolution
    /// simply has one fewer signal to consider — it is never an error.
    #[serde(alias = "favorite_agent", skip_serializing_if = "Option::is_none")]
    pub preferred_agent: Option<Provider>,

    /// The canonical provider for this scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_provider: Option<Provider>,

    /// Per-provider model catalog overrides.
    ///
    /// Augments or replaces the dynamically fetched model catalog used to
    /// validate frontmatter `model` hints during composition resolution.
    /// User-scope only; repo configs may not declare this field. A provider
    /// whose override is `null` has none.
    #[serde(deserialize_with = "super::wire::null_entries_unset", skip_serializing_if = "HashMap::is_empty")]
    pub models: HashMap<Provider, ProviderModelOverride>,

    /// Default sound effects for outcome categories.
    pub default_sounds: DefaultSounds,

    /// Whether to prompt for missing required schema properties.
    ///
    /// When `true` (the default), composition operations enter Interactive
    /// Mode to collect missing required values when stdin and stderr are
    /// TTYs. When `false`, missing required values fail with a
    /// `MissingProperties` error instead.
    ///
    /// User-scope only; repo configs may not declare this field.
    #[serde(skip_serializing_if = "is_true")]
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
    #[serde(skip_serializing_if = "is_false")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_expressions: Option<ExitExpressionsValue>,

    /// Scalar knobs for the repetition + volume guards (Cluster F).
    ///
    /// Follows last-writer precedence (frontmatter > repo > user >
    /// built-in) like `timeout` / `step_timeout` — there is no combine
    /// mode here, only the list-typed `exit_expressions` carries one.
    #[serde(skip_serializing_if = "is_default_guard_settings")]
    pub guard_settings: GuardSettings,

    /// Steering settings; `steering.automatic.enabled` turns automatic
    /// repetition help off or on.
    #[serde(skip_serializing_if = "SteeringConfig::is_unset")]
    pub steering: SteeringConfig,

    /// Darkmatter's `permissions` block (or a reference to the file holding
    /// it), exactly as written. Darkmatter's imported type validates it and
    /// Darkmatter interprets it; Claudine only keeps it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<serde_json::Value>,
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_false(value: &bool) -> bool {
    !*value
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
/// A repo config file (`{repo}/.claudine/config.yaml`) only contains the
/// fields that are allowed to differ per-repo. Unlike [`ClaudineConfig`],
/// every field is an override: absent or `null` inherits the user's value,
/// so the `#[serde(default)]`s below and the null-as-unset readers read
/// both as "no override".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoOverrideConfig {
    /// Override the canonical provider for this repo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_provider: Option<Provider>,

    /// Override or extend actions for this repo (per-event replacement).
    /// Keyed in the file as in [`ClaudineConfig::actions`]; a `null` event
    /// list is no override for that event.
    #[serde(default, with = "super::wire::actions", skip_serializing_if = "HashMap::is_empty")]
    pub actions: HashMap<AgenticEvent, Vec<HookAction>>,

    /// Override or extend per-event matchers for this repo.
    ///
    /// Per-event replacement: a repo entry fully replaces the user's entry
    /// for the same event. See [`ClaudineConfig::matchers`] for compilation
    /// semantics. A `null` matcher is no override for that event.
    #[serde(default, deserialize_with = "super::wire::null_entries_unset", skip_serializing_if = "HashMap::is_empty")]
    pub matchers: HashMap<AgenticEvent, String>,

    /// The user's messenger route this repo sends to instead of
    /// `messenger.active_config`; `None` (absent or `null`) inherits it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_messenger: Option<String>,

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

    /// Repo-scoped steering settings. A value set here overrides the user's;
    /// an absent or `null` one inherits it.
    #[serde(default, deserialize_with = "super::wire::null_as_default", skip_serializing_if = "SteeringConfig::is_unset")]
    pub steering: SteeringConfig,

    /// Darkmatter's repository `permissions` block, exactly as written; see
    /// [`ClaudineConfig::permissions`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<serde_json::Value>,
}

impl RepoOverrideConfig {
    pub fn is_empty(&self) -> bool {
        self.canonical_provider.is_none()
            && self.actions.is_empty()
            && self.matchers.is_empty()
            && self.active_messenger.is_none()
            && self.exit_expressions.is_none()
            && self.guard_settings.is_none()
            && self.steering.is_unset()
            && self.permissions.is_none()
    }

    /// Reads a whole repository document, judged by the repository schema.
    /// An absent or `null` override inherits.
    ///
    /// ## Errors
    ///
    /// [`ClaudineError::ConfigInvalid`] with each problem's JSON pointer.
    pub fn from_repository_document(document: serde_json::Value) -> Result<Self> {
        let effective = super::load::project_value(super::schema::ConfigScope::Repository, &document)?;
        Self::from_effective(effective, None)
    }

    /// Reads a judged document's effective value.
    pub(crate) fn from_effective(effective: serde_json::Value, file: Option<&std::path::Path>) -> Result<Self> {
        serde_json::from_value(effective).map_err(|error| typed_read_failure(file, &error))
    }
}

/// The schema's projection of an empty user file.
impl Default for ClaudineConfig {
    fn default() -> Self {
        Self::from_user_document(serde_json::Value::Object(serde_json::Map::new()))
            .expect("the shipped schema's defaults read as a ClaudineConfig")
    }
}

/// A judged document serde could not read: a defect of the schema or the
/// types, reported against the file rather than panicking.
fn typed_read_failure(file: Option<&std::path::Path>, error: &serde_json::Error) -> ClaudineError {
    ClaudineError::ConfigInvalid {
        problems: vec![super::load::ConfigProblem::whole(file, format!("the configuration cannot be read: {error}"))],
    }
}

impl ClaudineConfig {
    /// Reads a whole user document: judged by the user schema, with its
    /// defaults in place. Semantic checks are [`Self::validate`]'s.
    ///
    /// ## Errors
    ///
    /// [`ClaudineError::ConfigInvalid`] with each problem's JSON pointer.
    pub fn from_user_document(document: serde_json::Value) -> Result<Self> {
        let effective = super::load::project_value(super::schema::ConfigScope::User, &document)?;
        Self::from_effective(effective, None)
    }

    /// Reads a judged document's effective value.
    pub(crate) fn from_effective(effective: serde_json::Value, file: Option<&std::path::Path>) -> Result<Self> {
        serde_json::from_value(effective).map_err(|error| typed_read_failure(file, &error))
    }

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
