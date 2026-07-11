use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::capabilities::{
    ALL_PROVIDERS, LinkableResource, ProviderCapabilities, ResourceFormat, ResourceSupport,
    capabilities_for,
};
use crate::provider::Provider;

/// Scope for linking operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceScope {
    /// User-level resources (home directory) use absolute symlink targets.
    User,
    /// Repository-level resources use relative symlink targets.
    Repo,
}

/// Paths and discovery metadata for one provider.
#[derive(Debug, Clone)]
pub struct ProviderPaths {
    /// Provider identity.
    pub provider: Provider,
    /// User-level skill directory, if Markdown skill linking is supported.
    pub user_skills: Option<PathBuf>,
    /// Repo-level skill directory, if Markdown skill linking is supported.
    pub repo_skills: Option<PathBuf>,
    /// User-level command directory, if Markdown command linking is supported.
    pub user_commands: Option<PathBuf>,
    /// Repo-level command directory, if Markdown command linking is supported.
    pub repo_commands: Option<PathBuf>,
    /// User-level agent directory, if Markdown agent linking is supported.
    pub user_agents: Option<PathBuf>,
    /// Repo-level agent directory, if Markdown agent linking is supported.
    pub repo_agents: Option<PathBuf>,
    /// Additional skill roots this provider reads from.
    pub skill_also_reads_from: Vec<PathBuf>,
    /// Additional command roots this provider reads from.
    pub command_also_reads_from: Vec<PathBuf>,
    /// Additional agent roots this provider reads from.
    pub agent_also_reads_from: Vec<PathBuf>,
}

/// Provider paths for all supported providers, derived from capabilities metadata.
#[derive(Debug, Clone)]
pub struct ProviderSkillPaths {
    providers: HashMap<Provider, ProviderPaths>,
    home_dir: PathBuf,
    repo_root: PathBuf,
}

impl ProviderSkillPaths {
    /// Construct provider paths from capability metadata.
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let repo_root = resolve_repo_root(&cwd);
        Self::from_roots(home_dir, repo_root)
    }

    fn from_roots(home_dir: PathBuf, repo_root: PathBuf) -> Self {
        let providers = ALL_PROVIDERS
            .iter()
            .map(|provider| {
                let provider_paths = Self::build_provider_paths(*provider, &home_dir, &repo_root);
                (*provider, provider_paths)
            })
            .collect();

        Self {
            providers,
            home_dir,
            repo_root,
        }
    }

    fn build_provider_paths(
        provider: Provider,
        home_dir: &Path,
        repo_root: &Path,
    ) -> ProviderPaths {
        let caps = capabilities_for(provider);
        let (user_skills, repo_skills) = Self::markdown_link_paths(
            caps.support_for(LinkableResource::Skill),
            home_dir,
            repo_root,
        );
        let (user_commands, repo_commands) = Self::markdown_link_paths(
            caps.support_for(LinkableResource::Command),
            home_dir,
            repo_root,
        );
        let (user_agents, repo_agents) = Self::markdown_link_paths(
            caps.support_for(LinkableResource::Agent),
            home_dir,
            repo_root,
        );

        ProviderPaths {
            provider,
            user_skills,
            repo_skills,
            user_commands,
            repo_commands,
            user_agents,
            repo_agents,
            skill_also_reads_from: caps.skills.also_reads_from.clone(),
            command_also_reads_from: caps.commands.also_reads_from.clone(),
            agent_also_reads_from: caps.agents.also_reads_from.clone(),
        }
    }

    fn markdown_link_paths(
        support: &ResourceSupport,
        home_dir: &Path,
        repo_root: &Path,
    ) -> (Option<PathBuf>, Option<PathBuf>) {
        if !support.level.allows_custom() || support.format != Some(ResourceFormat::Markdown) {
            return (None, None);
        }

        let user = support
            .user_path
            .as_ref()
            .and_then(Self::non_empty_path)
            .map(|path| Self::expand_user_path(path, home_dir));

        let repo = support
            .repo_path
            .as_ref()
            .and_then(Self::non_empty_path)
            .map(|path| Self::expand_repo_path(path, repo_root));

        (user, repo)
    }

    fn non_empty_path(path: &PathBuf) -> Option<&PathBuf> {
        if path.as_os_str().is_empty() {
            None
        } else {
            Some(path)
        }
    }

    fn expand_user_path(path: &Path, home_dir: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            home_dir.join(path)
        }
    }

    fn expand_repo_path(path: &Path, repo_root: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            repo_root.join(path)
        }
    }

    fn resolve_also_reads(&self, paths: &[PathBuf], scope: ResourceScope) -> Vec<PathBuf> {
        paths
            .iter()
            .filter(|path| !path.as_os_str().is_empty())
            .map(|path| match scope {
                ResourceScope::User => Self::expand_user_path(path, &self.home_dir),
                ResourceScope::Repo => Self::expand_repo_path(path, &self.repo_root),
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn from_providers_for_test(
        providers: HashMap<Provider, ProviderPaths>,
        home_dir: PathBuf,
    ) -> Self {
        Self {
            providers,
            home_dir: home_dir.clone(),
            repo_root: home_dir,
        }
    }

    #[cfg(test)]
    pub(crate) fn from_roots_for_test(home_dir: PathBuf, repo_root: PathBuf) -> Self {
        Self::from_roots(home_dir, repo_root)
    }

    /// Return provider names and skill paths for the given scope.
    pub fn for_scope(&self, scope: ResourceScope) -> Vec<(Provider, &PathBuf)> {
        ALL_PROVIDERS
            .iter()
            .filter_map(|provider| {
                let paths = self.providers.get(provider)?;
                let dir = match scope {
                    ResourceScope::User => paths.user_skills.as_ref(),
                    ResourceScope::Repo => paths.repo_skills.as_ref(),
                }?;
                Some((*provider, dir))
            })
            .collect()
    }

    /// Return provider names and command paths for the given scope.
    pub fn commands_for_scope(&self, scope: ResourceScope) -> Vec<(Provider, &PathBuf)> {
        ALL_PROVIDERS
            .iter()
            .filter_map(|provider| {
                let paths = self.providers.get(provider)?;
                let dir = match scope {
                    ResourceScope::User => paths.user_commands.as_ref(),
                    ResourceScope::Repo => paths.repo_commands.as_ref(),
                }?;
                Some((*provider, dir))
            })
            .collect()
    }

    /// Return provider names and agent paths for the given scope.
    pub fn agents_for_scope(&self, scope: ResourceScope) -> Vec<(Provider, &PathBuf)> {
        ALL_PROVIDERS
            .iter()
            .filter_map(|provider| {
                let paths = self.providers.get(provider)?;
                let dir = match scope {
                    ResourceScope::User => paths.user_agents.as_ref(),
                    ResourceScope::Repo => paths.repo_agents.as_ref(),
                }?;
                Some((*provider, dir))
            })
            .collect()
    }

    /// Resolve the target directory for a provider/resource in a scope.
    pub fn target_dir(
        &self,
        provider: Provider,
        resource: LinkableResource,
        scope: ResourceScope,
    ) -> Option<PathBuf> {
        let paths = self.providers.get(&provider)?;
        match (resource, scope) {
            (LinkableResource::Skill, ResourceScope::User) => paths.user_skills.clone(),
            (LinkableResource::Skill, ResourceScope::Repo) => paths.repo_skills.clone(),
            (LinkableResource::Command, ResourceScope::User) => paths.user_commands.clone(),
            (LinkableResource::Command, ResourceScope::Repo) => paths.repo_commands.clone(),
            (LinkableResource::Agent, ResourceScope::User) => paths.user_agents.clone(),
            (LinkableResource::Agent, ResourceScope::Repo) => paths.repo_agents.clone(),
            _ => None,
        }
    }

    /// Resolve the configured resource directory for any custom-supporting provider/resource.
    ///
    /// Unlike `target_dir`, this method is format-agnostic and includes non-markdown
    /// resource formats (for derived artifact workflows).
    pub fn resource_dir(
        &self,
        provider: Provider,
        resource: LinkableResource,
        scope: ResourceScope,
    ) -> Option<PathBuf> {
        let caps = capabilities_for(provider);
        let support = caps.support_for(resource);
        if !support.level.allows_custom() {
            return None;
        }

        let base = match scope {
            ResourceScope::User => support.user_path.as_ref()?,
            ResourceScope::Repo => support.repo_path.as_ref()?,
        };

        if base.as_os_str().is_empty() {
            return None;
        }

        Some(match scope {
            ResourceScope::User => Self::expand_user_path(base, &self.home_dir),
            ResourceScope::Repo => Self::expand_repo_path(base, &self.repo_root),
        })
    }

    /// Return scope-aware "also reads from" directories for a provider/resource.
    pub fn also_reads_from(
        &self,
        provider: Provider,
        resource: LinkableResource,
        scope: ResourceScope,
    ) -> Vec<PathBuf> {
        let Some(paths) = self.providers.get(&provider) else {
            return vec![];
        };

        let raw = match resource {
            LinkableResource::Skill => &paths.skill_also_reads_from,
            LinkableResource::Command => &paths.command_also_reads_from,
            LinkableResource::Agent => &paths.agent_also_reads_from,
            LinkableResource::Script => return vec![],
        };

        self.resolve_also_reads(raw, scope)
    }

    /// Lookup all capabilities as an ordered vec (display order).
    pub fn capabilities(&self) -> Vec<ProviderCapabilities> {
        ALL_PROVIDERS
            .iter()
            .map(|provider| capabilities_for(*provider))
            .collect()
    }

    /// Resolved home directory used for user-scoped path expansion.
    pub fn home_dir(&self) -> &Path {
        &self.home_dir
    }

    /// Resolved repository root used for repo-scoped path expansion.
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }
}

impl Default for ProviderSkillPaths {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve the repository root for a working directory via Sniff.
///
/// If no repository root can be detected, falls back to `cwd`.
pub fn resolve_repo_root(cwd: &Path) -> PathBuf {
    if let Ok(Some(git)) = sniff::filesystem::detect_git(cwd, false, 1) {
        return git.repo_root;
    }

    if let Ok(Some(repo)) = sniff::filesystem::detect_repo(cwd) {
        return repo.root;
    }

    cwd.to_path_buf()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    use tempfile::TempDir;

    use super::*;

    fn init_git_repo(path: &Path) -> bool {
        Command::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(path)
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[test]
    fn new_populates_all_providers() {
        let paths = ProviderSkillPaths::new();
        let user_scope = paths.for_scope(ResourceScope::User);
        assert_eq!(user_scope.len(), ALL_PROVIDERS.len());
    }

    #[test]
    fn for_scope_contains_expected_skills_dirs() {
        let paths = ProviderSkillPaths::new();
        let user_scope = paths.for_scope(ResourceScope::User);

        let providers: Vec<Provider> = user_scope.iter().map(|(provider, _)| *provider).collect();
        assert!(providers.contains(&Provider::Claude));
        assert!(providers.contains(&Provider::Codex));
        assert!(providers.contains(&Provider::OpenCode));
    }

    #[test]
    fn commands_for_scope_uses_capability_metadata() {
        let paths = ProviderSkillPaths::new();
        let commands = paths.commands_for_scope(ResourceScope::User);
        let providers: Vec<Provider> = commands.iter().map(|(provider, _)| *provider).collect();

        // Markdown commands supported.
        assert!(providers.contains(&Provider::Claude));
        assert!(providers.contains(&Provider::Codex));
        assert!(providers.contains(&Provider::OpenCode));
        assert!(providers.contains(&Provider::QwenCode));

        // Gemini commands are TOML, not Markdown-linkable.
        assert!(!providers.contains(&Provider::Gemini));
    }

    #[test]
    fn opencode_also_reads_from_claude_for_skills() {
        let paths = ProviderSkillPaths::new();
        let reads = paths.also_reads_from(
            Provider::OpenCode,
            LinkableResource::Skill,
            ResourceScope::Repo,
        );
        assert!(reads.iter().any(|path| path.ends_with(".claude/skills")));
    }

    #[test]
    fn repo_scope_target_paths_are_absolute() {
        let paths = ProviderSkillPaths::new();
        let repo_target = paths.target_dir(
            Provider::Claude,
            LinkableResource::Skill,
            ResourceScope::Repo,
        );
        assert!(repo_target.is_some());
        assert!(repo_target.unwrap().is_absolute());
    }

    #[test]
    fn resolve_repo_root_returns_nested_git_root() {
        let tmp = TempDir::new().unwrap();
        if !init_git_repo(tmp.path()) {
            // Skip this assertion if git isn't available in the test environment.
            return;
        }

        let nested = tmp.path().join("nested/deeper/path");
        fs::create_dir_all(&nested).unwrap();

        let resolved = resolve_repo_root(&nested);
        assert_eq!(
            resolved.canonicalize().unwrap(),
            tmp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn resolve_repo_root_falls_back_to_cwd_when_not_in_repo() {
        let tmp = TempDir::new().unwrap();
        let resolved = resolve_repo_root(tmp.path());
        assert_eq!(
            resolved.canonicalize().unwrap(),
            tmp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn agents_for_scope_contains_markdown_providers() {
        let paths = ProviderSkillPaths::new();
        let agents = paths.agents_for_scope(ResourceScope::User);
        let providers: Vec<Provider> = agents.iter().map(|(p, _)| *p).collect();

        // Claude agents are Markdown.
        assert!(providers.contains(&Provider::Claude));
    }

    #[test]
    fn target_dir_resolves_agent_paths() {
        let paths = ProviderSkillPaths::new();
        let target = paths.target_dir(
            Provider::Claude,
            LinkableResource::Agent,
            ResourceScope::User,
        );
        assert!(target.is_some());
        assert!(target.unwrap().ends_with(".claude/agents"));
    }

    #[test]
    fn resource_dir_resolves_non_markdown_custom_paths() {
        let paths = ProviderSkillPaths::new();
        let gemini_cmds = paths.resource_dir(
            Provider::Gemini,
            LinkableResource::Command,
            ResourceScope::User,
        );
        assert!(gemini_cmds.is_some());
        assert!(gemini_cmds.unwrap().ends_with(".gemini/commands"));
    }
}
