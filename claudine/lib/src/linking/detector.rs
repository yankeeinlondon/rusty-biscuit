use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::events::Provider;

use super::capabilities::{ALL_PROVIDERS, LinkableResource, ResourceFormat, capabilities_for};
use super::hashing;
use super::paths::{ProviderSkillPaths, ResourceScope};

/// A resource discovered by detector-based scanning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredResource {
    /// Resource name (directory/file stem).
    pub name: String,
    /// Full filesystem path to the resource.
    pub path: PathBuf,
    /// Provider that owns this resource location.
    pub provider: Provider,
    /// Whether this resource entry is a symlink.
    pub is_symlink: bool,
    /// Optional content hash.
    pub hash: Option<u64>,
}

#[derive(Debug, Clone)]
struct DetectorState {
    resource: LinkableResource,
    providers: Vec<Provider>,
    user_base: Option<Provider>,
    repo_base: Option<Provider>,
    user_canonical_resources: HashMap<String, DiscoveredResource>,
    user_resources: Vec<DiscoveredResource>,
    repo_canonical_resources: Option<HashMap<String, DiscoveredResource>>,
    repo_resources: Option<Vec<DiscoveredResource>>,
    repo_base_path: Option<PathBuf>,
}

/// Contract for per-resource detectors with scope-separated state.
pub trait LinkDetector {
    /// Run detection and cache results.
    fn detect(
        paths: &ProviderSkillPaths,
        user_base: Option<Provider>,
        repo_base: Option<Provider>,
        include_repo: bool,
    ) -> Result<Self>
    where
        Self: Sized;

    /// Resource kind this detector is responsible for.
    fn resource(&self) -> LinkableResource;

    /// Providers supporting this resource type.
    fn supported_providers(&self) -> &[Provider];

    /// Canonical provider for user-scoped resources.
    fn user_base(&self) -> Option<Provider>;

    /// Canonical provider for repo-scoped resources.
    fn repo_base(&self) -> Option<Provider>;

    /// Canonical resources discovered in user scope.
    fn user_canonical_resources(&self) -> &HashMap<String, DiscoveredResource>;

    /// All resources discovered in user scope.
    fn user_resources(&self) -> &[DiscoveredResource];

    /// Canonical resources discovered in repo scope.
    fn repo_canonical_resources(&self) -> Option<&HashMap<String, DiscoveredResource>>;

    /// All resources discovered in repo scope.
    fn repo_resources(&self) -> Option<&[DiscoveredResource]>;

    /// Resolved repo root used during repo-scoped detection.
    fn repo_base_path(&self) -> Option<&Path>;
}

/// Skill detector state.
#[derive(Debug, Clone)]
pub struct SkillsDetector {
    state: DetectorState,
}

/// Slash command detector state.
#[derive(Debug, Clone)]
pub struct SlashCommandsDetector {
    state: DetectorState,
}

/// Agent/subagent definition detector state.
#[derive(Debug, Clone)]
pub struct AgentDefinitionsDetector {
    state: DetectorState,
}

/// Shared scripts directory detector state.
#[derive(Debug, Clone)]
pub struct SharedScriptsDetector {
    state: DetectorState,
}

impl SkillsDetector {
    pub fn new(
        paths: &ProviderSkillPaths,
        user_base: Option<Provider>,
        repo_base: Option<Provider>,
        include_repo: bool,
    ) -> Result<Self> {
        Self::detect(paths, user_base, repo_base, include_repo)
    }
}

impl SlashCommandsDetector {
    pub fn new(
        paths: &ProviderSkillPaths,
        user_base: Option<Provider>,
        repo_base: Option<Provider>,
        include_repo: bool,
    ) -> Result<Self> {
        Self::detect(paths, user_base, repo_base, include_repo)
    }
}

impl AgentDefinitionsDetector {
    pub fn new(
        paths: &ProviderSkillPaths,
        user_base: Option<Provider>,
        repo_base: Option<Provider>,
        include_repo: bool,
    ) -> Result<Self> {
        Self::detect(paths, user_base, repo_base, include_repo)
    }
}

impl SharedScriptsDetector {
    pub fn new(
        paths: &ProviderSkillPaths,
        user_base: Option<Provider>,
        repo_base: Option<Provider>,
        include_repo: bool,
    ) -> Result<Self> {
        Self::detect(paths, user_base, repo_base, include_repo)
    }
}

macro_rules! impl_detector {
    ($detector:ident, $resource:expr) => {
        impl LinkDetector for $detector {
            fn detect(
                paths: &ProviderSkillPaths,
                user_base: Option<Provider>,
                repo_base: Option<Provider>,
                include_repo: bool,
            ) -> Result<Self> {
                Ok(Self {
                    state: build_state($resource, paths, user_base, repo_base, include_repo)?,
                })
            }

            fn resource(&self) -> LinkableResource {
                self.state.resource
            }

            fn supported_providers(&self) -> &[Provider] {
                &self.state.providers
            }

            fn user_base(&self) -> Option<Provider> {
                self.state.user_base
            }

            fn repo_base(&self) -> Option<Provider> {
                self.state.repo_base
            }

            fn user_canonical_resources(&self) -> &HashMap<String, DiscoveredResource> {
                &self.state.user_canonical_resources
            }

            fn user_resources(&self) -> &[DiscoveredResource] {
                &self.state.user_resources
            }

            fn repo_canonical_resources(&self) -> Option<&HashMap<String, DiscoveredResource>> {
                self.state.repo_canonical_resources.as_ref()
            }

            fn repo_resources(&self) -> Option<&[DiscoveredResource]> {
                self.state.repo_resources.as_deref()
            }

            fn repo_base_path(&self) -> Option<&Path> {
                self.state.repo_base_path.as_deref()
            }
        }
    };
}

impl_detector!(SkillsDetector, LinkableResource::Skill);
impl_detector!(SlashCommandsDetector, LinkableResource::Command);
impl_detector!(AgentDefinitionsDetector, LinkableResource::Agent);
impl_detector!(SharedScriptsDetector, LinkableResource::Script);

fn build_state(
    resource: LinkableResource,
    paths: &ProviderSkillPaths,
    user_base: Option<Provider>,
    repo_base: Option<Provider>,
    include_repo: bool,
) -> Result<DetectorState> {
    let providers = providers_for_resource(resource);
    let user_resources = detect_scope_resources(paths, resource, ResourceScope::User)?;
    let user_canonical_resources = canonical_map(&user_resources, user_base);

    let (repo_resources, repo_canonical_resources, repo_base_path) = if include_repo {
        let repo_resources = detect_scope_resources(paths, resource, ResourceScope::Repo)?;
        let repo_canonical = canonical_map(&repo_resources, repo_base);
        (
            Some(repo_resources),
            Some(repo_canonical),
            Some(paths.repo_root().to_path_buf()),
        )
    } else {
        (None, None, None)
    };

    Ok(DetectorState {
        resource,
        providers,
        user_base,
        repo_base,
        user_canonical_resources,
        user_resources,
        repo_canonical_resources,
        repo_resources,
        repo_base_path,
    })
}

fn providers_for_resource(resource: LinkableResource) -> Vec<Provider> {
    ALL_PROVIDERS
        .iter()
        .copied()
        .filter(|provider| {
            capabilities_for(*provider)
                .support_for(resource)
                .level
                .is_supported()
        })
        .collect()
}

fn detect_scope_resources(
    paths: &ProviderSkillPaths,
    resource: LinkableResource,
    scope: ResourceScope,
) -> Result<Vec<DiscoveredResource>> {
    let mut resources = Vec::new();

    for provider in providers_for_resource(resource) {
        let caps = capabilities_for(provider);
        let support = caps.support_for(resource);
        let Some(root) = resource_root(paths, provider, resource, scope) else {
            continue;
        };

        let mut discovered =
            discover_provider_resources(provider, resource, support.format, &root)?;
        resources.append(&mut discovered);
    }

    resources.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.provider.as_slug().cmp(right.provider.as_slug()))
            .then(left.path.cmp(&right.path))
    });

    Ok(resources)
}

fn resource_root(
    paths: &ProviderSkillPaths,
    provider: Provider,
    resource: LinkableResource,
    scope: ResourceScope,
) -> Option<PathBuf> {
    let caps = capabilities_for(provider);
    let support = caps.support_for(resource);

    let base = match scope {
        ResourceScope::User => support.user_path.as_ref()?,
        ResourceScope::Repo => support.repo_path.as_ref()?,
    };

    if base.as_os_str().is_empty() {
        return None;
    }

    Some(match scope {
        ResourceScope::User => {
            if base.is_absolute() {
                base.clone()
            } else {
                paths.home_dir().join(base)
            }
        }
        ResourceScope::Repo => {
            if base.is_absolute() {
                base.clone()
            } else {
                paths.repo_root().join(base)
            }
        }
    })
}

fn discover_provider_resources(
    provider: Provider,
    resource: LinkableResource,
    format: Option<ResourceFormat>,
    root: &Path,
) -> Result<Vec<DiscoveredResource>> {
    let mut out = Vec::new();

    if !root.exists() {
        return Ok(out);
    }

    if root.is_file() {
        if let Some(resource) = discover_file_resource(provider, resource, format, root, true) {
            out.push(resource);
        }
        return Ok(out);
    }

    if !root.is_dir() {
        return Ok(out);
    }

    if resource == LinkableResource::Command {
        discover_nested_command_resources(provider, format, root, root, &mut out)?;
        return Ok(out);
    }

    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Ok(out),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if let Some(resource) = discover_path_resource(provider, resource, format, &path)? {
            out.push(resource);
        }
    }

    Ok(out)
}

fn discover_nested_command_resources(
    provider: Provider,
    format: Option<ResourceFormat>,
    root: &Path,
    current: &Path,
    out: &mut Vec<DiscoveredResource>,
) -> Result<()> {
    let entries = match std::fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if file_name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            discover_nested_command_resources(provider, format, root, &path, out)?;
            continue;
        }

        if !matches_resource_file(&path, LinkableResource::Command, format) {
            continue;
        }

        let Some(relative_path) = path.strip_prefix(root).ok() else {
            continue;
        };
        let Some(name) = namespaced_name_from_relative_path(relative_path) else {
            continue;
        };

        out.push(build_discovered_resource(
            provider,
            name,
            path,
            LinkableResource::Command,
        ));
    }

    Ok(())
}

fn discover_path_resource(
    provider: Provider,
    resource: LinkableResource,
    format: Option<ResourceFormat>,
    path: &Path,
) -> Result<Option<DiscoveredResource>> {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return Ok(None);
    };

    if file_name.starts_with('.') {
        return Ok(None);
    }

    if path.is_file() {
        return Ok(discover_file_resource(
            provider, resource, format, path, false,
        ));
    }

    if !path.is_dir() {
        return Ok(None);
    }

    if resource != LinkableResource::Skill {
        return Ok(None);
    }

    if !path.join("SKILL.md").is_file() {
        return Ok(None);
    }

    Ok(Some(build_discovered_resource(
        provider,
        file_name.to_string(),
        path.to_path_buf(),
        resource,
    )))
}

fn discover_file_resource(
    provider: Provider,
    resource: LinkableResource,
    format: Option<ResourceFormat>,
    path: &Path,
    allow_hidden_name: bool,
) -> Option<DiscoveredResource> {
    if !matches_resource_file(path, resource, format) {
        return None;
    }

    let name = if resource == LinkableResource::Script {
        path.file_name().and_then(|name| name.to_str())
    } else {
        path.file_stem()
            .and_then(|name| name.to_str())
            .or_else(|| path.file_name().and_then(|name| name.to_str()))
    }?;

    if !allow_hidden_name && name.starts_with('.') {
        return None;
    }

    Some(build_discovered_resource(
        provider,
        name.to_string(),
        path.to_path_buf(),
        resource,
    ))
}

fn matches_resource_file(
    path: &Path,
    resource: LinkableResource,
    format: Option<ResourceFormat>,
) -> bool {
    match resource {
        LinkableResource::Skill => false,
        LinkableResource::Script => path.is_file(),
        LinkableResource::Command | LinkableResource::Agent => match format {
            Some(ResourceFormat::Markdown) => has_extension(path, &["md"]),
            Some(ResourceFormat::Toml) => has_extension(path, &["toml"]),
            Some(ResourceFormat::Yaml) => has_extension(path, &["yaml", "yml"]),
            Some(ResourceFormat::Executable) => path.is_file(),
            _ => false,
        },
    }
}

fn has_extension(path: &Path, exts: &[&str]) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    exts.iter()
        .any(|expected| ext.eq_ignore_ascii_case(expected))
}

fn build_discovered_resource(
    provider: Provider,
    name: String,
    path: PathBuf,
    resource: LinkableResource,
) -> DiscoveredResource {
    let is_symlink = std::fs::symlink_metadata(&path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false);

    let hash = match resource {
        LinkableResource::Skill if path.is_dir() => hashing::hash_skill_dir(&path).ok(),
        _ if path.is_file() => std::fs::read(&path)
            .ok()
            .map(|content| biscuit_hash::xx_hash_bytes(&content)),
        _ => None,
    };

    DiscoveredResource {
        name,
        path,
        provider,
        is_symlink,
        hash,
    }
}

fn namespaced_name_from_relative_path(relative_path: &Path) -> Option<String> {
    let stemmed = relative_path.with_extension("");
    let mut parts = Vec::new();
    for component in stemmed.components() {
        let part = component.as_os_str().to_str()?;
        if part.is_empty() {
            return None;
        }
        parts.push(part);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(":"))
    }
}

fn canonical_map(
    resources: &[DiscoveredResource],
    base_provider: Option<Provider>,
) -> HashMap<String, DiscoveredResource> {
    let Some(provider) = base_provider else {
        return HashMap::new();
    };

    let mut map = HashMap::new();
    for resource in resources
        .iter()
        .filter(|resource| resource.provider == provider)
    {
        map.entry(resource.name.clone())
            .or_insert_with(|| resource.clone());
    }

    map
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    fn write_skill(root: &Path, name: &str) {
        let skill_path = root.join(name).join("SKILL.md");
        write(
            &skill_path,
            "---\nname: test\ndescription: test\n---\n# Skill\n",
        );
    }

    #[test]
    fn skills_detector_tracks_user_and_repo_state() {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();
        let paths = ProviderSkillPaths::from_roots_for_test(
            home.path().to_path_buf(),
            repo.path().to_path_buf(),
        );

        write_skill(&home.path().join(".claude/skills"), "user-skill");
        write_skill(&repo.path().join(".claude/skills"), "repo-skill");

        let detector =
            SkillsDetector::new(&paths, Some(Provider::Claude), Some(Provider::Claude), true)
                .unwrap();

        assert_eq!(detector.resource(), LinkableResource::Skill);
        assert!(detector.supported_providers().contains(&Provider::Claude));
        assert!(
            detector
                .user_resources()
                .iter()
                .any(|resource| resource.name == "user-skill")
        );
        assert!(
            detector
                .user_canonical_resources()
                .contains_key("user-skill")
        );
        let repo_resources = detector.repo_resources().unwrap();
        assert!(
            repo_resources
                .iter()
                .any(|resource| resource.name == "repo-skill")
        );
        assert!(
            detector
                .repo_canonical_resources()
                .unwrap()
                .contains_key("repo-skill")
        );
        assert_eq!(detector.repo_base_path().unwrap(), repo.path());
    }

    #[test]
    fn command_agent_and_script_detectors_read_expected_formats() {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();
        let paths = ProviderSkillPaths::from_roots_for_test(
            home.path().to_path_buf(),
            repo.path().to_path_buf(),
        );

        write(
            &home.path().join(".claude/commands/user-cmd.md"),
            "---\ndescription: test\n---\n",
        );
        write(
            &home
                .path()
                .join(".claude/commands/prompts/create-deep-dive-document.md"),
            "---\ndescription: nested\n---\n",
        );
        write(
            &home.path().join(".gemini/commands/user-gemini.toml"),
            r#"prompt = "hello""#,
        );
        write(
            &repo.path().join(".claude/agents/repo-agent.md"),
            "---\nname: agent\ndescription: test\n---\n",
        );
        write(
            &home.path().join(".codex/scripts/tool.sh"),
            "#!/usr/bin/env bash\necho tool\n",
        );

        let commands = SlashCommandsDetector::new(
            &paths,
            Some(Provider::Claude),
            Some(Provider::Claude),
            true,
        )
        .unwrap();
        assert!(
            commands
                .user_resources()
                .iter()
                .any(|resource| resource.name == "user-cmd")
        );
        assert!(
            commands
                .user_resources()
                .iter()
                .any(|resource| resource.name == "prompts:create-deep-dive-document")
        );
        assert!(
            commands
                .user_resources()
                .iter()
                .any(|resource| resource.name == "user-gemini")
        );

        let agents = AgentDefinitionsDetector::new(
            &paths,
            Some(Provider::Claude),
            Some(Provider::Claude),
            true,
        )
        .unwrap();
        assert!(
            agents
                .repo_resources()
                .unwrap()
                .iter()
                .any(|resource| resource.name == "repo-agent")
        );

        let scripts =
            SharedScriptsDetector::new(&paths, Some(Provider::Codex), Some(Provider::Codex), true)
                .unwrap();
        assert!(
            scripts
                .user_resources()
                .iter()
                .any(|resource| resource.name == "tool.sh")
        );
    }

    #[test]
    fn include_repo_false_keeps_repo_state_empty() {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();
        let paths = ProviderSkillPaths::from_roots_for_test(
            home.path().to_path_buf(),
            repo.path().to_path_buf(),
        );

        write_skill(&home.path().join(".claude/skills"), "user-only");
        let detector = SkillsDetector::new(&paths, Some(Provider::Claude), None, false).unwrap();

        assert!(detector.repo_resources().is_none());
        assert!(detector.repo_canonical_resources().is_none());
        assert!(detector.repo_base_path().is_none());
    }
}
