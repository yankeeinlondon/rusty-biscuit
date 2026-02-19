use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::events::Provider;

use super::capabilities::{
    ALL_PROVIDERS, LinkableResource, ProviderCapabilities, ResourceFormat, ResourceSupport,
    capabilities_for,
};

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
    /// Additional skill roots this provider reads from.
    pub skill_also_reads_from: Vec<PathBuf>,
    /// Additional command roots this provider reads from.
    pub command_also_reads_from: Vec<PathBuf>,
}

/// Provider paths for all supported providers, derived from capabilities metadata.
#[derive(Debug, Clone)]
pub struct ProviderSkillPaths {
    providers: HashMap<Provider, ProviderPaths>,
    home_dir: PathBuf,
}

impl ProviderSkillPaths {
    /// Construct provider paths from capability metadata.
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let providers = ALL_PROVIDERS
            .iter()
            .map(|provider| {
                let provider_paths = Self::build_provider_paths(*provider, &home_dir);
                (*provider, provider_paths)
            })
            .collect();

        Self {
            providers,
            home_dir,
        }
    }

    fn build_provider_paths(provider: Provider, home_dir: &Path) -> ProviderPaths {
        let caps = capabilities_for(provider);
        let (user_skills, repo_skills) =
            Self::markdown_link_paths(caps.support_for(LinkableResource::Skill), home_dir);
        let (user_commands, repo_commands) =
            Self::markdown_link_paths(caps.support_for(LinkableResource::Command), home_dir);

        ProviderPaths {
            provider,
            user_skills,
            repo_skills,
            user_commands,
            repo_commands,
            skill_also_reads_from: caps.skills.also_reads_from.clone(),
            command_also_reads_from: caps.commands.also_reads_from.clone(),
        }
    }

    fn markdown_link_paths(
        support: &ResourceSupport,
        home_dir: &Path,
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
            .cloned();

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

    fn resolve_also_reads(&self, paths: &[PathBuf], scope: ResourceScope) -> Vec<PathBuf> {
        paths
            .iter()
            .filter(|path| !path.as_os_str().is_empty())
            .map(|path| match scope {
                ResourceScope::User => Self::expand_user_path(path, &self.home_dir),
                ResourceScope::Repo => path.clone(),
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
            home_dir,
        }
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
            _ => None,
        }
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
            LinkableResource::Agent | LinkableResource::Script => return vec![],
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
        assert!(providers.contains(&Provider::RooCode));

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
        assert!(reads.contains(&PathBuf::from(".claude/skills")));
    }

    #[test]
    fn target_dir_returns_none_for_unsupported_resource() {
        let paths = ProviderSkillPaths::new();
        let target = paths.target_dir(
            Provider::Claude,
            LinkableResource::Agent,
            ResourceScope::User,
        );
        assert!(target.is_none());
    }
}
