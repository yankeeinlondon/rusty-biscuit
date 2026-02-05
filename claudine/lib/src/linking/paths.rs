use std::path::PathBuf;

/// Scope for skill linking operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkScope {
    /// User-level (home directory) skills — absolute symlinks.
    User,
    /// Repository-level skills — relative symlinks.
    Repo,
}

/// Skill and command paths for a single provider.
#[derive(Debug, Clone)]
pub struct ProviderPaths {
    /// User-level skill directory.
    pub user_skills: PathBuf,
    /// Repo-level skill directory.
    pub repo_skills: PathBuf,
    /// User-level command directory (if supported).
    pub user_commands: Option<PathBuf>,
    /// Repo-level command directory (if supported).
    pub repo_commands: Option<PathBuf>,
    /// Directories this provider also reads from (e.g., OpenCode reads Claude's).
    pub also_reads_from: Vec<PathBuf>,
}

/// Skill and command paths for all providers.
#[derive(Debug, Clone)]
pub struct ProviderSkillPaths {
    pub claude: ProviderPaths,
    pub gemini: ProviderPaths,
    pub codex: ProviderPaths,
    pub opencode: ProviderPaths,
}

impl ProviderSkillPaths {
    /// Construct provider paths using the user's home directory.
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("home directory must exist");

        let claude_user_skills = home.join(".claude/skills");
        let claude_repo_skills = PathBuf::from(".claude/skills");

        Self {
            claude: ProviderPaths {
                user_skills: claude_user_skills.clone(),
                repo_skills: claude_repo_skills.clone(),
                user_commands: Some(home.join(".claude/commands")),
                repo_commands: Some(PathBuf::from(".claude/commands")),
                also_reads_from: vec![],
            },
            gemini: ProviderPaths {
                user_skills: home.join(".gemini/skills"),
                repo_skills: PathBuf::from(".gemini/skills"),
                user_commands: None, // Gemini uses TOML format — incompatible
                repo_commands: None,
                also_reads_from: vec![],
            },
            codex: ProviderPaths {
                user_skills: home.join(".codex/skills"),
                repo_skills: PathBuf::from(".codex/skills"),
                user_commands: None, // Codex has no command system
                repo_commands: None,
                also_reads_from: vec![],
            },
            opencode: ProviderPaths {
                user_skills: home.join(".config/opencode/skills"),
                repo_skills: PathBuf::from(".opencode/skills"),
                user_commands: Some(home.join(".config/opencode/commands")),
                repo_commands: Some(PathBuf::from(".opencode/commands")),
                also_reads_from: vec![claude_user_skills, claude_repo_skills],
            },
        }
    }

    /// Return provider names and their skill paths for the given scope.
    pub fn for_scope(&self, scope: LinkScope) -> Vec<(&str, &PathBuf)> {
        let providers = [
            ("claude", &self.claude),
            ("gemini", &self.gemini),
            ("codex", &self.codex),
            ("opencode", &self.opencode),
        ];

        providers
            .iter()
            .map(|(name, paths)| {
                let dir = match scope {
                    LinkScope::User => &paths.user_skills,
                    LinkScope::Repo => &paths.repo_skills,
                };
                (*name, dir)
            })
            .collect()
    }

    /// Return provider names and their command paths for the given scope.
    ///
    /// Only returns providers that support Markdown commands
    /// (Claude, OpenCode). Gemini (TOML) and Codex (none)
    /// are excluded.
    pub fn commands_for_scope(&self, scope: LinkScope) -> Vec<(&str, &PathBuf)> {
        let providers: Vec<(&str, &ProviderPaths)> = vec![
            ("claude", &self.claude),
            ("opencode", &self.opencode),
        ];

        providers
            .into_iter()
            .filter_map(|(name, paths)| {
                let dir = match scope {
                    LinkScope::User => paths.user_commands.as_ref()?,
                    LinkScope::Repo => paths.repo_commands.as_ref()?,
                };
                Some((name, dir))
            })
            .collect()
    }

    /// Return the also_reads_from paths for a given provider name.
    pub fn also_reads_from(&self, provider: &str) -> &[PathBuf] {
        match provider {
            "claude" => &self.claude.also_reads_from,
            "gemini" => &self.gemini.also_reads_from,
            "codex" => &self.codex.also_reads_from,
            "opencode" => &self.opencode.also_reads_from,
            _ => &[],
        }
    }
}

impl Default for ProviderSkillPaths {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_populates_all_providers() {
        let paths = ProviderSkillPaths::new();
        assert!(paths.claude.user_skills.ends_with(".claude/skills"));
        assert!(paths.gemini.user_skills.ends_with(".gemini/skills"));
        assert!(paths.codex.user_skills.ends_with(".codex/skills"));
        assert!(paths
            .opencode
            .user_skills
            .ends_with(".config/opencode/skills"));
    }

    #[test]
    fn for_scope_returns_all_providers() {
        let paths = ProviderSkillPaths::new();
        let user_scope = paths.for_scope(LinkScope::User);
        assert_eq!(user_scope.len(), 4);

        let names: Vec<&str> = user_scope.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"claude"));
        assert!(names.contains(&"gemini"));
        assert!(names.contains(&"codex"));
        assert!(names.contains(&"opencode"));
    }

    #[test]
    fn commands_for_scope_excludes_gemini_and_codex() {
        let paths = ProviderSkillPaths::new();
        let cmds = paths.commands_for_scope(LinkScope::User);
        let names: Vec<&str> = cmds.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"claude"));
        assert!(names.contains(&"opencode"));
        assert!(!names.contains(&"gemini"));
        assert!(!names.contains(&"codex"));
    }

    #[test]
    fn opencode_reads_from_claude() {
        let paths = ProviderSkillPaths::new();
        assert!(!paths.opencode.also_reads_from.is_empty());
        assert!(paths.claude.also_reads_from.is_empty());
    }

    #[test]
    fn gemini_has_no_commands() {
        let paths = ProviderSkillPaths::new();
        assert!(paths.gemini.user_commands.is_none());
        assert!(paths.gemini.repo_commands.is_none());
    }

    #[test]
    fn codex_has_no_commands() {
        let paths = ProviderSkillPaths::new();
        assert!(paths.codex.user_commands.is_none());
        assert!(paths.codex.repo_commands.is_none());
    }
}
