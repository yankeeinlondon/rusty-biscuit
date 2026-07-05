//! Typed system-prompt delivery metadata.
//!
//! Phase 5 of the centralized providers refactor replaces the
//! free-form `supplement_sources` / `replacement_mechanisms` strings on
//! the retired legacy `SystemPromptCapabilities` tree
//! with structured [`SystemPromptDelivery`] enums describing how each
//! provider accepts system-prompt overrides in interactive vs
//! non-interactive mode.

use serde::Serialize;

use super::path_template::PathTemplate;

/// One mechanism by which a provider accepts a system prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemPromptDelivery {
    /// An inline CLI flag accepting the prompt text directly,
    /// e.g. `--append-system-prompt "..."`.
    InlineFlag {
        /// The CLI flag name including leading dashes.
        flag: &'static str,
    },
    /// A CLI flag accepting a path to a file containing the prompt,
    /// e.g. `--append-system-prompt-file <path>`.
    FileFlag {
        /// The CLI flag name including leading dashes.
        flag: &'static str,
    },
    /// An environment variable pointing to a file containing the prompt.
    EnvVarFile {
        /// The environment variable name.
        env_var: &'static str,
    },
    /// Place a file under a shadow `HOME` overlay (e.g. Gemini's
    /// `~/.gemini/AGENTS.md`).
    ShadowHomeFile {
        /// Path within the shadow home, relative to its root.
        relative_path: &'static str,
    },
    /// Provider config-override flag plus key, with the prompt content
    /// inlined as the value. E.g. Codex `-c developer_instructions="..."`.
    ConfigKeyInline {
        /// The CLI flag name including leading dashes.
        flag: &'static str,
        /// The config key name.
        key: &'static str,
    },
    /// Provider config-override flag plus key, with the value being a
    /// path to a file containing the prompt. E.g. Codex
    /// `-c model_instructions_file=<path>`.
    ConfigKeyFile {
        /// The CLI flag name including leading dashes.
        flag: &'static str,
        /// The config key name.
        key: &'static str,
    },
    /// Provider-specific behavior captured by a typed tag rather than
    /// a flag.
    Custom(SystemPromptCustomTag),
    /// The provider does not support this style of system-prompt
    /// override in this mode.
    Unsupported,
}

/// Tags for system-prompt delivery patterns that don't fit the standard
/// flag/env-var shape. New variants are added as new providers are
/// onboarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemPromptCustomTag {
    /// Kimi Code: prompts are delivered via a per-session AGENT YAML file.
    KimiAgentFile,
    /// Goose: receipt management is delegated to the configurator stack.
    GooseRecipe,
    /// Codex: `model_instructions_file` config setting in `config.toml`.
    #[deprecated(
        since = "0.6.0",
        note = "Replaced by ConfigKeyFile/ConfigKeyInline in SystemPromptDelivery; kept for one deprecation cycle"
    )]
    CodexInstructionsFile,
}

/// System-prompt delivery split by entrypoint mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SystemPromptDeliveryByMode {
    /// Mechanism used in interactive mode.
    pub interactive: SystemPromptDelivery,
    /// Mechanism used in non-interactive (`--print` / `exec`) mode.
    pub non_interactive: SystemPromptDelivery,
}

/// Composite system-prompt capability descriptor for a provider.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SystemPromptSpec {
    /// How the provider accepts an *appended* system prompt (added on
    /// top of the built-in prompt).
    pub append: SystemPromptDeliveryByMode,
    /// How the provider accepts a *replacement* system prompt (full
    /// override of the built-in prompt).
    pub replace: SystemPromptDeliveryByMode,
    /// Memory / instruction files that contribute to the system prompt
    /// hierarchy, e.g. `CLAUDE.md`, `AGENTS.md`.
    pub memory_files: &'static [PathTemplate],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_key_inline_serializes_round_trip() {
        let delivery = SystemPromptDelivery::ConfigKeyInline {
            flag: "-c",
            key: "developer_instructions",
        };
        let json = serde_json::to_value(delivery).expect("ConfigKeyInline serializes");
        assert_eq!(
            json,
            serde_json::json!({
                "config_key_inline": {
                    "flag": "-c",
                    "key": "developer_instructions"
                }
            })
        );
    }

    #[test]
    fn config_key_file_serializes_round_trip() {
        let delivery = SystemPromptDelivery::ConfigKeyFile {
            flag: "-c",
            key: "model_instructions_file",
        };
        let json = serde_json::to_value(delivery).expect("ConfigKeyFile serializes");
        assert_eq!(
            json,
            serde_json::json!({
                "config_key_file": {
                    "flag": "-c",
                    "key": "model_instructions_file"
                }
            })
        );
    }
}
