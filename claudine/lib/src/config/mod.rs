mod atomic;
mod backup;
mod claude;
mod codex;
mod gemini;
mod opencode;
mod trait_def;

pub use trait_def::{AgentConfigurator, RegistrationResult, SkipReason};

use std::path::PathBuf;

use sniff::programs::{AiCli, InstalledAiClients, ProgramMetadata};

use crate::events::Provider;

use claude::ClaudeConfigurator;
use codex::CodexConfigurator;
use gemini::GeminiConfigurator;
use opencode::OpenCodeConfigurator;

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
/// Returns information about all 6 supported providers, including whether
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

/// Detect available agents by checking for their config files.
///
/// Returns a list of `(Provider, Configurator)` pairs for every provider
/// whose config directory exists on the current system.
pub fn detect_agents() -> Vec<(Provider, Box<dyn AgentConfigurator>)> {
    let mut agents: Vec<(Provider, Box<dyn AgentConfigurator>)> = Vec::new();
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return agents,
    };

    // Claude: ~/.claude/settings.json
    if home.join(".claude").join("settings.json").exists() {
        agents.push((Provider::Claude, Box::new(ClaudeConfigurator)));
    }

    // Gemini: ~/.gemini/settings.json
    if home.join(".gemini").join("settings.json").exists() {
        agents.push((Provider::Gemini, Box::new(GeminiConfigurator)));
    }

    // Codex: ~/.codex/config.toml
    if home.join(".codex").join("config.toml").exists() {
        agents.push((Provider::Codex, Box::new(CodexConfigurator)));
    }

    // OpenCode: ~/.config/opencode/opencode.json
    if home
        .join(".config")
        .join("opencode")
        .join("opencode.json")
        .exists()
    {
        agents.push((Provider::OpenCode, Box::new(OpenCodeConfigurator)));
    }

    agents
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
    fn discover_agents_full_returns_all_six() {
        let agents = discover_agents_full();
        assert_eq!(agents.len(), 6);

        // Check all providers are present
        let providers: Vec<_> = agents.iter().map(|a| a.provider).collect();
        assert!(providers.contains(&Provider::Claude));
        assert!(providers.contains(&Provider::Codex));
        assert!(providers.contains(&Provider::Gemini));
        assert!(providers.contains(&Provider::Goose));
        assert!(providers.contains(&Provider::KimiCode));
        assert!(providers.contains(&Provider::OpenCode));
    }

    #[test]
    fn agent_info_display_names() {
        let agents = discover_agents_full();
        let claude = agents.iter().find(|a| a.provider == Provider::Claude).unwrap();
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
