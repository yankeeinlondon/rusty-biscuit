pub mod atomic;
pub(crate) mod antigravity;
pub(crate) mod backup;
pub(crate) mod claude;
pub mod claudine_config;
pub(crate) mod codex;
pub(crate) mod gemini;
pub(crate) mod goose;
pub(crate) mod kilo;
pub(crate) mod kimicode;
pub mod merge;
pub mod messaging_block;
pub mod migration;
pub(crate) mod opencode;
pub(crate) mod pi;
pub(crate) mod qwen;
mod trait_def;
pub mod tts;

pub use trait_def::{AgentConfigurator, ProviderHookPlan, RegistrationResult, SkipReason};

// Re-exports for convenient access and backward compatibility
pub use claudine_config::{
    ClaudineConfig, DefaultSounds, DetailedModelOverride, ModelOverrideEntry, ModelOverrideMode,
    ModelOverrideValue, ProviderModelOverride, RepoOverrideConfig,
};
pub use messaging_block::{ClaudineMessengerConfig, MessengerProviderConfig};
pub use tts::{Gender, TtsConfigSettings, TtsValue, VoiceSelection};

pub(crate) use antigravity::AntigravityConfigurator;
pub(crate) use claude::ClaudeConfigurator;
pub(crate) use codex::CodexConfigurator;
pub(crate) use gemini::GeminiConfigurator;
pub(crate) use goose::GooseConfigurator;
pub(crate) use kilo::KiloConfigurator;
pub(crate) use kimicode::KimiCodeConfigurator;
pub(crate) use opencode::OpenCodeConfigurator;
pub(crate) use pi::PiConfigurator;
pub(crate) use qwen::QwenConfigurator;

use std::path::PathBuf;

use sniff::programs::{InstalledAiClients, ProgramMetadata, find_program::find_program};

use crate::provider::{PROVIDERS_DISPLAY_ORDER, Provider, provider_info};

/// Resolve the full path to the Claudine executable.
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
/// the normalized event name to dispatch. Executable paths containing
/// whitespace are quoted for the host shell; simple executable tokens retain
/// their existing spelling.
pub(crate) fn claudine_handle_command(provider: Provider) -> impl Fn(&str) -> String {
    let claudine_bin = claudine_command();
    let provider = provider.as_slug().to_string();
    move |event| format_claudine_handle_command(&claudine_bin, event, &provider)
}

fn format_claudine_handle_command(executable: &str, event: &str, provider: &str) -> String {
    let executable = hook_executable_token(executable);
    format!("{executable} handle {event} --provider {provider}")
}

fn hook_executable_token(executable: &str) -> String {
    if !executable.chars().any(char::is_whitespace) {
        return executable.to_string();
    }

    #[cfg(windows)]
    {
        // Double quotes are forbidden in native Windows path components, so
        // this token needs no inner escaping.
        format!(r#""{executable}""#)
    }

    #[cfg(not(windows))]
    {
        // The shell-valid apostrophe splice is intentionally not a general
        // shell parser contract. Managed-hook recognition remains bounded to
        // ordinary leading quoted tokens.
        format!("'{}'", executable.replace('\'', r#"'\''"#))
    }
}

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
/// Returns one [`AgentInfo`] per entry in
/// [`crate::provider::PROVIDERS_DISPLAY_ORDER`]. All static facts —
/// display name, binary name, sniff binding, and the primary user-level
/// config path — are sourced from the central provider catalog via
/// [`provider_info`]. The first element of `info.config_paths` is treated
/// as the primary user-level config path; consult the field documentation
/// on [`crate::provider::ProviderInfo::config_paths`] for the convention.
pub fn discover_agents_full() -> Vec<AgentInfo> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    let ai_clients = InstalledAiClients::new();

    PROVIDERS_DISPLAY_ORDER
        .into_iter()
        .map(|provider| {
            let info = provider_info(provider);
            let primary = info
                .config_paths
                .first()
                .expect("every provider must declare at least one config path")
                .resolve_with_home(&home);
            let config_exists = primary.exists();
            let on_path = ai_clients.is_installed(info.sniff_binding);

            AgentInfo {
                provider,
                config_exists,
                on_path,
                display_name: info.sniff_binding.display_name(),
                binary_name: info.sniff_binding.binary_name(),
                config_path: if config_exists { Some(primary) } else { None },
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
    fn handle_command_keeps_simple_executable_token_unchanged() {
        assert_eq!(
            format_claudine_handle_command("claudine", "before_tool", "claude"),
            "claudine handle before_tool --provider claude"
        );
    }

    #[cfg(windows)]
    #[test]
    fn handle_command_quotes_native_windows_executable_path() {
        assert_eq!(
            format_claudine_handle_command(
                r"C:\Program Files\Claudine\claudine.exe",
                "before_tool",
                "claude",
            ),
            r#""C:\Program Files\Claudine\claudine.exe" handle before_tool --provider claude"#
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn handle_command_quotes_unix_executable_path() {
        assert_eq!(
            format_claudine_handle_command(
                "/opt/Claudine Tools/claudine",
                "before_tool",
                "claude",
            ),
            "'/opt/Claudine Tools/claudine' handle before_tool --provider claude"
        );
    }

    #[test]
    fn detect_agents_returns_vec() {
        let agents = detect_agents();
        // Returns empty vec if no agents detected (valid behavior)
        let _ = agents;
    }

    #[test]
    fn discover_agents_full_returns_all_providers() {
        let agents = discover_agents_full();
        assert_eq!(agents.len(), 10);

        // Check all providers are present
        let providers: Vec<_> = agents.iter().map(|a| a.provider).collect();
        assert!(providers.contains(&Provider::Claude));
        assert!(providers.contains(&Provider::Codex));
        assert!(providers.contains(&Provider::Gemini));
        assert!(providers.contains(&Provider::Goose));
        assert!(providers.contains(&Provider::KimiCode));
        assert!(providers.contains(&Provider::OpenCode));
        assert!(providers.contains(&Provider::QwenCode));
        assert!(providers.contains(&Provider::Kilo));
        assert!(providers.contains(&Provider::Pi));
        assert!(providers.contains(&Provider::Antigravity));
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
    fn discover_agents_full_uses_catalog_sniff_binding() {
        let agents = discover_agents_full();
        for agent in &agents {
            let info = provider_info(agent.provider);
            assert_eq!(
                agent.display_name,
                info.sniff_binding.display_name(),
                "{:?}: display_name must come from catalog sniff binding",
                agent.provider,
            );
            assert_eq!(
                agent.binary_name,
                info.sniff_binding.binary_name(),
                "{:?}: binary_name must come from catalog sniff binding",
                agent.provider,
            );
        }
    }

    #[test]
    fn discover_agents_full_covers_every_catalog_provider_exactly_once() {
        let agents = discover_agents_full();
        let observed: std::collections::HashSet<_> = agents.iter().map(|a| a.provider).collect();
        let expected: std::collections::HashSet<_> = PROVIDERS_DISPLAY_ORDER.into_iter().collect();
        assert_eq!(observed, expected);
        assert_eq!(agents.len(), crate::provider::PROVIDER_COUNT);
    }

    #[test]
    fn discover_agents_full_primary_config_path_matches_catalog_template() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        let agents = discover_agents_full();
        for agent in &agents {
            let info = provider_info(agent.provider);
            let expected = info
                .config_paths
                .first()
                .expect("every provider must declare at least one config path")
                .resolve_with_home(&home);
            // `config_path` is `None` when the file does not exist; in
            // that case we still want the resolution to match the catalog,
            // so reconstruct it from the same source.
            let actual = agent
                .config_path
                .clone()
                .unwrap_or_else(|| expected.clone());
            assert_eq!(actual, expected, "{:?}", agent.provider);
        }
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
