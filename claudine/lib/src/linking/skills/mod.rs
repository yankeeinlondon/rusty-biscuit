use std::path::PathBuf;

use crate::provider::Provider;
use super::filter::ResourceFilter;

pub mod native;
pub mod partial;
pub mod portable;

// Re-exports preserving the public API surface
pub use native::fix_missing_skills;
pub use portable::list_skills;

/// Backward-compatible alias for `ResourceFilter`.
pub type SkillFilter = ResourceFilter;

/// Scope classification for a discovered skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillScope {
    /// Only in user-level skills directory (~/.claude/skills/).
    User,
    /// Present in both user and repo — repo wins (masks user).
    RepoMasked,
    /// Only in repo-level skills directory (.claude/skills/).
    Repo,
}

/// Information about a single discovered skill.
#[derive(Debug, Clone)]
pub struct SkillInfo {
    /// Skill directory name.
    pub name: String,
    /// Scope classification.
    pub scope: SkillScope,
    /// Description from SKILL.md frontmatter.
    pub description: Option<String>,
    /// Path to the SKILL.md file.
    pub skill_md_path: PathBuf,
}

/// Type of exception found during skill validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExceptionType {
    /// Skill directory missing for a provider.
    Missing,
    /// SKILL.md missing required `description` frontmatter.
    Invalid,
    /// SKILL.md frontmatter uses tab indentation that strict YAML parsers reject.
    YamlTabs,
    /// SKILL.md contains a broken relative link.
    BrokenLink,
    /// SKILL.md body is long but contains no markdown links.
    NoLinks,
}

impl std::fmt::Display for ExceptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExceptionType::Missing => write!(f, "missing"),
            ExceptionType::Invalid => write!(f, "invalid"),
            ExceptionType::YamlTabs => write!(f, "yaml-tabs"),
            ExceptionType::BrokenLink => write!(f, "broken-link"),
            ExceptionType::NoLinks => write!(f, "no-links"),
        }
    }
}

/// A single exception found during skill validation.
#[derive(Debug, Clone)]
pub struct SkillException {
    /// Provider where the exception was found.
    pub provider: Provider,
    /// Type of exception.
    pub exception_type: ExceptionType,
    /// Skill topic name.
    pub topic: String,
    /// Path to the SKILL.md file (or expected location).
    pub skill_md_path: PathBuf,
    /// For `Invalid`: names of missing required frontmatter properties.
    pub missing_properties: Vec<String>,
    /// For `BrokenLink`: the markdown link text (`[text]`).
    pub link_text: Option<String>,
    /// For `BrokenLink`: the link target path (`(target)`).
    pub link_target: Option<String>,
}

/// Directory-level diagnostic explaining why all skills for a scope are missing.
#[derive(Debug, Clone)]
pub struct SkillDirectoryDiagnostic {
    /// Provider where the issue was found.
    pub provider: Provider,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Complete report from skill discovery.
#[derive(Debug, Clone)]
pub struct SkillsReport {
    /// All discovered skills with scope classification.
    pub skills: Vec<SkillInfo>,
    /// Exceptions found during validation.
    pub exceptions: Vec<SkillException>,
    /// Directory-level diagnostics (missing directories).
    pub diagnostics: Vec<SkillDirectoryDiagnostic>,
}

/// Summary of fix operations applied by [`fix_missing_skills`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SkillFixSummary {
    /// Directories created for providers that had no skills directory.
    pub directories_created: usize,
    /// Symlinks created for individual missing skills.
    pub links_created: usize,
    /// Skills that were already correctly linked.
    pub already_linked: usize,
    /// Operations skipped (real directory exists, symlink points elsewhere, etc.).
    pub skipped: usize,
    /// SKILL.md files that had a missing `name` property auto-inserted.
    pub names_inserted: usize,
    /// SKILL.md files whose YAML indentation tabs were normalized to spaces.
    pub yaml_tabs_fixed: usize,
    /// Skills skipped because they have Claude-specific frontmatter (not shareable).
    pub not_shareable: usize,
}

#[cfg(test)]
pub(super) mod test_helpers {
    use std::collections::HashMap;

    use crate::linking::paths::ProviderPaths;
    use crate::linking::capabilities::ALL_PROVIDERS;
    use crate::provider::Provider;

    pub fn empty_provider(provider: Provider) -> ProviderPaths {
        ProviderPaths {
            provider,
            user_skills: None,
            repo_skills: None,
            user_commands: None,
            repo_commands: None,
            user_agents: None,
            repo_agents: None,
            skill_also_reads_from: vec![],
            command_also_reads_from: vec![],
            agent_also_reads_from: vec![],
        }
    }

    pub fn test_paths(base: &std::path::Path) -> super::super::paths::ProviderSkillPaths {
        let mut providers = HashMap::new();
        for provider in ALL_PROVIDERS {
            providers.insert(provider, empty_provider(provider));
        }

        providers.insert(
            Provider::Claude,
            ProviderPaths {
                provider: Provider::Claude,
                user_skills: Some(base.join("user/skills")),
                repo_skills: Some(base.join("repo/skills")),
                user_commands: None,
                repo_commands: None,
                user_agents: None,
                repo_agents: None,
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
            },
        );

        super::super::paths::ProviderSkillPaths::from_providers_for_test(providers, base.to_path_buf())
    }

    pub fn setup_skill(dir: &std::path::Path, name: &str, description: &str, body: &str) {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\ndescription: {description}\n---\n{body}"),
        )
        .unwrap();
    }

    pub fn test_paths_with_gemini(base: &std::path::Path) -> super::super::paths::ProviderSkillPaths {
        let mut providers = HashMap::new();
        for provider in ALL_PROVIDERS {
            providers.insert(provider, empty_provider(provider));
        }

        providers.insert(
            Provider::Claude,
            ProviderPaths {
                provider: Provider::Claude,
                user_skills: Some(base.join("user/skills")),
                repo_skills: Some(base.join("repo/skills")),
                user_commands: None,
                repo_commands: None,
                user_agents: None,
                repo_agents: None,
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
            },
        );
        providers.insert(
            Provider::Gemini,
            ProviderPaths {
                provider: Provider::Gemini,
                user_skills: Some(base.join("gemini/skills")),
                repo_skills: Some(base.join("repo/.gemini/skills")),
                user_commands: None,
                repo_commands: None,
                user_agents: None,
                repo_agents: None,
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
            },
        );

        super::super::paths::ProviderSkillPaths::from_providers_for_test(providers, base.to_path_buf())
    }
}
