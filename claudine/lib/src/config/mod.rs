mod atomic;
mod backup;
mod claude;
mod codex;
mod gemini;
mod opencode;
mod roo;
mod trait_def;

pub use trait_def::{AgentConfigurator, RegistrationResult, SkipReason};

use crate::events::Provider;

use claude::ClaudeConfigurator;
use codex::CodexConfigurator;
use gemini::GeminiConfigurator;
use opencode::OpenCodeConfigurator;
use roo::RooConfigurator;

/// Detect available agents by checking for their config files.
///
/// Returns a list of `(Provider, Configurator)` pairs for every provider
/// whose config directory exists on the current system. Roo Code is always
/// included (it returns `SkipReason::WrapperOnly`).
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

    // Roo Code: always included (returns WrapperOnly)
    agents.push((Provider::RooCode, Box::new(RooConfigurator)));

    agents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_agents_always_includes_roo() {
        let agents = detect_agents();
        assert!(
            agents.iter().any(|(p, _)| *p == Provider::RooCode),
            "Roo Code should always be included"
        );
    }

    #[test]
    fn detect_agents_returns_vec() {
        let agents = detect_agents();
        // At minimum, Roo is always there
        assert!(!agents.is_empty());
    }
}
