//! AI CLI detection — type alias with backward-compatible accessors.

use std::path::PathBuf;

use crate::programs::enums::AiCli;
use crate::programs::contract::ExecutableSource;
use crate::programs::types::CategoryDetector;

/// AI-powered CLI coding tools found on the system.
pub type InstalledAiClients = CategoryDetector<AiCli>;

/// Backward-compatible convenience methods for InstalledAiClients.
impl InstalledAiClients {
    /// Returns true if Claude Code is installed.
    pub fn claude(&self) -> bool {
        self.is_installed(AiCli::Claude)
    }

    /// Returns true if OpenCode is installed.
    pub fn opencode(&self) -> bool {
        self.is_installed(AiCli::Opencode)
    }

    /// Returns true if Roo Code is installed.
    pub fn roo(&self) -> bool {
        self.is_installed(AiCli::Roo)
    }

    /// Returns true if Gemini CLI is installed.
    pub fn gemini_cli(&self) -> bool {
        self.is_installed(AiCli::GeminiCli)
    }

    /// Returns true if Aider is installed.
    pub fn aider(&self) -> bool {
        self.is_installed(AiCli::Aider)
    }

    /// Returns true if Codex CLI is installed.
    pub fn codex(&self) -> bool {
        self.is_installed(AiCli::Codex)
    }

    /// Returns true if Goose is installed.
    pub fn goose(&self) -> bool {
        self.is_installed(AiCli::Goose)
    }

    /// Returns true if Kimi Code CLI is installed.
    pub fn kimi_cli(&self) -> bool {
        self.is_installed(AiCli::KimiCli)
    }

    /// Returns true if Qwen Code CLI is installed.
    pub fn qwen_cli(&self) -> bool {
        self.is_installed(AiCli::QwenCli)
    }

    /// Mark a client as installed (for testing purposes).
    pub fn with_client(self, client: AiCli) -> Self {
        use crate::programs::schema::ProgramMetadata;
        let info = client.info();
        let fake_path = PathBuf::from(format!("/usr/bin/{}", info.binary_name));
        self.with_program(client, fake_path, ExecutableSource::Path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_client_marks_as_installed() {
        let clients = InstalledAiClients::default().with_client(AiCli::Claude);
        assert!(clients.is_installed(AiCli::Claude));
        assert!(!clients.is_installed(AiCli::Aider));
    }

    #[test]
    fn test_boolean_accessors() {
        let clients = InstalledAiClients::default().with_client(AiCli::Claude);
        assert!(clients.claude());
        assert!(!clients.opencode());
        assert!(!clients.roo());
        assert!(!clients.gemini_cli());
        assert!(!clients.aider());
        assert!(!clients.codex());
        assert!(!clients.goose());
        assert!(!clients.kimi_cli());
        assert!(!clients.qwen_cli());
    }
}
