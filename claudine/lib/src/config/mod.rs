pub mod atomic;
pub(crate) mod backup;
pub(crate) mod claude;
pub mod claudine_config;
pub(crate) mod codex;
pub(crate) mod gemini;
pub(crate) mod goose;
pub(crate) mod kimicode;
pub mod migration;
pub(crate) mod opencode;
pub(crate) mod qwen;
pub(crate) mod roo;
mod trait_def;

pub use trait_def::{AgentConfigurator, ProviderHookPlan, RegistrationResult, SkipReason};

pub(crate) use claude::ClaudeConfigurator;
pub(crate) use codex::CodexConfigurator;
pub(crate) use gemini::GeminiConfigurator;
pub(crate) use goose::GooseConfigurator;
pub(crate) use kimicode::KimiCodeConfigurator;
pub(crate) use opencode::OpenCodeConfigurator;
pub(crate) use qwen::QwenConfigurator;
pub(crate) use roo::RooConfigurator;

use std::path::PathBuf;

use sniff::programs::{AiCli, InstalledAiClients, ProgramMetadata, find_program::find_program};

/// Resolve the full path to the claudine executable.
///
/// Returns the absolute path to `claudine` if found on PATH, otherwise
/// falls back to just "claudine" (relying on PATH at runtime).
pub(crate) fn claudine_command() -> String {
    find_program("claudine")
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "claudine".to_string())
}

/// Return a provider-bound builder for `claudine handle` shell commands.
///
/// The first call fixes the provider, and the returned closure receives
/// the normalized event name to dispatch.
pub(crate) fn claudine_handle_command(provider: Provider) -> impl Fn(&str) -> String {
    let claudine_bin = claudine_command();
    let provider = provider.as_slug().to_string();
    move |event| format!("{claudine_bin} handle {event} --provider {provider}")
}

use crate::events::Provider;

/// Rich information about a detected agent.
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// The provider type.
    pub provider: Provider,
    /// Whether the config directory/file exists.
    pub config_exists: bool,
    /// Whether the binary is on PATH.
    pub on_path: bool,
    /// Human-readable display name.
    pub display_name: &'static str,
    /// Binary name to invoke.
    pub binary_name: &'static str,
    /// Path to the config file (if it exists).
    pub config_path: Option<PathBuf>,
}

impl AgentInfo {
    /// Returns true if the agent is available (either config exists or binary on PATH).
    pub fn is_available(&self) -> bool {
        self.config_exists || self.on_path
    }
}

/// Discover all supported agents with rich availability information.
///
/// Returns information about all supported providers, including whether
/// their config exists and whether their binary is on PATH.
pub fn discover_agents_full() -> Vec<AgentInfo> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    let ai_clients = InstalledAiClients::new();

    // Provider configs: (Provider, config_path, AiCli)
    let providers = [
        (
            Provider::Claude,
            home.join(".claude").join("settings.json"),
            AiCli::Claude,
        ),
        (
            Provider::Codex,
            home.join(".codex").join("config.toml"),
            AiCli::Codex,
        ),
        (
            Provider::Gemini,
            home.join(".gemini").join("settings.json"),
            AiCli::GeminiCli,
        ),
        (
            Provider::Goose,
            home.join(".config").join("goose").join("config.yaml"),
            AiCli::Goose,
        ),
        (
            Provider::KimiCode,
            home.join(".kimi").join("config.json"),
            AiCli::KimiCli,
        ),
        (
            Provider::OpenCode,
            home.join(".config").join("opencode").join("opencode.json"),
            AiCli::Opencode,
        ),
        (
            Provider::QwenCode,
            home.join(".qwen").join("settings.json"),
            AiCli::QwenCli,
        ),
        (
            Provider::RooCode,
            home.join(".roo").join("settings.json"),
            AiCli::Roo,
        ),
    ];

    providers
        .into_iter()
        .map(|(provider, config_path, ai_cli)| {
            let config_exists = config_path.exists();
            let on_path = ai_clients.is_installed(ai_cli);

            AgentInfo {
                provider,
                config_exists,
                on_path,
                display_name: ai_cli.display_name(),
                binary_name: ai_cli.binary_name(),
                config_path: if config_exists {
                    Some(config_path)
                } else {
                    None
                },
            }
        })
        .collect()
}

/// Get the configurator for a specific provider.
///
/// Returns the appropriate configurator regardless of whether the provider's
/// config file exists. Use this when you want to register hooks for a provider
/// that may not have been set up yet.
pub fn get_configurator(provider: Provider) -> Box<dyn AgentConfigurator> {
    crate::provider::provider_info(provider)
        .configurator
        .agent_configurator()
}

/// Detect available agents by checking for their config files.
///
/// Returns a list of `(Provider, Configurator)` pairs for every provider
/// whose config directory exists on the current system.
pub fn detect_agents() -> Vec<(Provider, Box<dyn AgentConfigurator>)> {
    discover_agents_full()
        .into_iter()
        .filter(|info| info.config_exists)
        .map(|info| (info.provider, get_configurator(info.provider)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_agents_returns_vec() {
        let agents = detect_agents();
        // Returns empty vec if no agents detected (valid behavior)
        let _ = agents;
    }

    #[test]
    fn discover_agents_full_returns_all_eight() {
        let agents = discover_agents_full();
        assert_eq!(agents.len(), 8);

        // Check all providers are present
        let providers: Vec<_> = agents.iter().map(|a| a.provider).collect();
        assert!(providers.contains(&Provider::Claude));
        assert!(providers.contains(&Provider::Codex));
        assert!(providers.contains(&Provider::Gemini));
        assert!(providers.contains(&Provider::Goose));
        assert!(providers.contains(&Provider::KimiCode));
        assert!(providers.contains(&Provider::OpenCode));
        assert!(providers.contains(&Provider::QwenCode));
        assert!(providers.contains(&Provider::RooCode));
    }

    #[test]
    fn agent_info_display_names() {
        let agents = discover_agents_full();
        let claude = agents
            .iter()
            .find(|a| a.provider == Provider::Claude)
            .unwrap();
        assert_eq!(claude.display_name, "Claude Code");
        assert_eq!(claude.binary_name, "claude");
    }

    #[test]
    fn agent_info_is_available() {
        // Test with neither config nor binary
        let info = AgentInfo {
            provider: Provider::Claude,
            config_exists: false,
            on_path: false,
            display_name: "Test",
            binary_name: "test",
            config_path: None,
        };
        assert!(!info.is_available());

        // Test with config only
        let info = AgentInfo {
            provider: Provider::Claude,
            config_exists: true,
            on_path: false,
            display_name: "Test",
            binary_name: "test",
            config_path: Some(PathBuf::from("/test")),
        };
        assert!(info.is_available());

        // Test with binary only
        let info = AgentInfo {
            provider: Provider::Claude,
            config_exists: false,
            on_path: true,
            display_name: "Test",
            binary_name: "test",
            config_path: None,
        };
        assert!(info.is_available());
    }
}
