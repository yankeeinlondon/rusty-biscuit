use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::serde_yaml_ng;

use super::capabilities::{ALL_PROVIDERS, LinkableResource, ResourceFormat, capabilities_for};
use super::compatibility::{
    fix_frontmatter_indentation_tabs, frontmatter_has_indentation_tabs,
    has_claude_specific_properties, parse_markdown_document,
};
use super::filter::ResourceFilter;
use super::paths::{ProviderSkillPaths, ResourceScope};
use super::symlink::{LinkResult, create_skill_link};
use crate::error::Result;
use crate::provider::Provider;

/// Scope classification for a discovered agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgentScope {
    /// Only in user-level agents directory (~/.claude/agents/).
    User,
    /// Present in both user and repo — repo wins (masks user).
    RepoMasked,
    /// Only in repo-level agents directory (.claude/agents/).
    Repo,
}

/// Information about a single discovered agent definition.
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// Agent name (file stem).
    pub name: String,
    /// Description from frontmatter.
    pub description: Option<String>,
    /// Scope classification.
    pub scope: AgentScope,
    /// Path to the agent file.
    pub agent_file_path: PathBuf,
    /// Resource format (Markdown, YAML, etc.).
    pub format: ResourceFormat,
    /// Provider where this agent was discovered.
    pub provider: Provider,
    /// Whether the agent file contains a `model` property.
    pub has_model_property: bool,
}

/// Type of exception found during agent validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgentExceptionType {
    /// Agent file missing for a provider.
    Missing,
    /// Agent file missing required frontmatter properties.
    Invalid,
    /// Agent frontmatter uses tab indentation that strict YAML parsers reject.
    YamlTabs,
    /// No links in agent body.
    NoLinks,
    /// Agent specifies a `model` property that limits cross-provider shareability.
    ModelPropertyNotShareable,
    /// Format incompatible with target provider.
    FormatIncompatible,
}

impl std::fmt::Display for AgentExceptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentExceptionType::Missing => write!(f, "missing"),
            AgentExceptionType::Invalid => write!(f, "invalid"),
            AgentExceptionType::YamlTabs => write!(f, "yaml-tabs"),
            AgentExceptionType::NoLinks => write!(f, "no-links"),
            AgentExceptionType::ModelPropertyNotShareable => write!(f, "model-not-shareable"),
            AgentExceptionType::FormatIncompatible => write!(f, "format-incompatible"),
        }
    }
}

/// A single exception found during agent validation.
#[derive(Debug, Clone)]
pub struct AgentException {
    /// Provider where the exception was found.
    pub provider: Provider,
    /// Type of exception.
    pub exception_type: AgentExceptionType,
    /// Agent name.
    pub name: String,
    /// Path to the agent file (or expected location).
    pub agent_file_path: PathBuf,
    /// For `Invalid`: names of missing required frontmatter properties.
    pub missing_properties: Vec<String>,
    /// For `FormatIncompatible`: source format.
    pub source_format: Option<ResourceFormat>,
    /// For `FormatIncompatible`: target format.
    pub target_format: Option<ResourceFormat>,
}

/// Directory-level diagnostic explaining why all agents for a scope are missing.
#[derive(Debug, Clone)]
pub struct AgentDirectoryDiagnostic {
    /// Provider where the issue was found.
    pub provider: Provider,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Complete report from agent discovery.
#[derive(Debug, Clone)]
pub struct AgentsReport {
    /// All discovered agents with scope classification.
    pub agents: Vec<AgentInfo>,
    /// Exceptions found during validation.
    pub exceptions: Vec<AgentException>,
    /// Directory-level diagnostics (missing directories).
    pub diagnostics: Vec<AgentDirectoryDiagnostic>,
}

/// Summary of fix operations applied by [`fix_missing_agents`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AgentFixSummary {
    /// Directories created for providers that had no agents directory.
    pub directories_created: usize,
    /// Symlinks created for individual missing agents.
    pub links_created: usize,
    /// Agents that were already correctly linked.
    pub already_linked: usize,
    /// Operations skipped.
    pub skipped: usize,
    /// Agents where format is incompatible with target provider.
    pub format_incompatible: usize,
    /// Agent files whose YAML indentation tabs were normalized to spaces.
    pub yaml_tabs_fixed: usize,
    /// Agents skipped because they have Claude-specific frontmatter (not shareable).
    pub not_shareable: usize,
}

/// Fix missing agent links for non-Claude providers.
pub fn fix_missing_agents(paths: &ProviderSkillPaths) -> Result<AgentFixSummary> {
    let mut summary = AgentFixSummary::default();

    let user_dir = paths.target_dir(
        Provider::Claude,
        LinkableResource::Agent,
        ResourceScope::User,
    );
    let repo_dir = paths.target_dir(
        Provider::Claude,
        LinkableResource::Agent,
        ResourceScope::Repo,
    );

    let all_user_agents = scan_agent_dir(user_dir.as_ref());
    let all_repo_agents = scan_agent_dir(repo_dir.as_ref());

    // Filter out agents with Claude-specific frontmatter — these are not shareable
    let (user_agents, user_skipped) = filter_unshareable(all_user_agents);
    let (repo_agents, repo_skipped) = filter_unshareable(all_repo_agents);
    summary.not_shareable = user_skipped + repo_skipped;

    for (_name, path) in user_agents.iter().chain(repo_agents.iter()) {
        if fix_frontmatter_indentation_tabs(path)? {
            summary.yaml_tabs_fixed += 1;
        }
    }

    for provider in ALL_PROVIDERS {
        if provider == Provider::Claude {
            continue;
        }

        let caps = capabilities_for(provider);
        let agent_support = caps.support_for(LinkableResource::Agent);
        if !agent_support.level.is_supported() {
            continue;
        }

        // Only link to Markdown-compatible providers
        if agent_support.format != Some(ResourceFormat::Markdown) {
            let count = user_agents.len() + repo_agents.len();
            summary.format_incompatible += count;
            continue;
        }

        for scope in [ResourceScope::User, ResourceScope::Repo] {
            let reads_claude = paths
                .also_reads_from(provider, LinkableResource::Agent, scope)
                .iter()
                .any(|p| p.to_string_lossy().contains(".claude/agents"));
            if reads_claude {
                continue;
            }

            let source_agents = match scope {
                ResourceScope::User => &user_agents,
                ResourceScope::Repo => &repo_agents,
            };

            if source_agents.is_empty() {
                continue;
            }

            let Some(provider_dir) = paths.target_dir(provider, LinkableResource::Agent, scope)
            else {
                continue;
            };

            if !provider_dir.exists() {
                fs::create_dir_all(&provider_dir)?;
                summary.directories_created += 1;
            }

            fix_scope_agents(source_agents, &provider_dir, scope, &mut summary)?;
        }
    }

    Ok(summary)
}

/// Create symlinks for missing agents in a single (provider, scope) pair.
fn fix_scope_agents(
    source_agents: &[(String, PathBuf)],
    provider_dir: &Path,
    scope: ResourceScope,
    summary: &mut AgentFixSummary,
) -> Result<()> {
    for (_name, source_path) in source_agents {
        match create_skill_link(source_path, provider_dir, scope)? {
            LinkResult::Linked { .. } => summary.links_created += 1,
            LinkResult::AlreadyLinked => summary.already_linked += 1,
            LinkResult::Skipped { .. } => summary.skipped += 1,
        }
    }
    Ok(())
}

/// Discover all agents from Claude's user and repo agent directories.
pub fn list_agents(paths: &ProviderSkillPaths, filters: &[String]) -> Result<AgentsReport> {
    let user_dir = paths.target_dir(
        Provider::Claude,
        LinkableResource::Agent,
        ResourceScope::User,
    );
    let repo_dir = paths.target_dir(
        Provider::Claude,
        LinkableResource::Agent,
        ResourceScope::Repo,
    );

    let user_agents = scan_agent_dir(user_dir.as_ref());
    let repo_agents = scan_agent_dir(repo_dir.as_ref());

    let user_names: BTreeSet<&str> = user_agents.iter().map(|(name, _)| name.as_str()).collect();
    let repo_names: BTreeSet<&str> = repo_agents.iter().map(|(name, _)| name.as_str()).collect();

    let mut agents = Vec::new();

    // Repo-only agents
    for (name, path) in &repo_agents {
        if !user_names.contains(name.as_str()) {
            let (description, has_model) = read_agent_metadata(path);
            agents.push(AgentInfo {
                name: name.clone(),
                scope: AgentScope::Repo,
                description,
                agent_file_path: path.clone(),
                format: ResourceFormat::Markdown,
                provider: Provider::Claude,
                has_model_property: has_model,
            });
        }
    }

    // User-only agents
    for (name, path) in &user_agents {
        if !repo_names.contains(name.as_str()) {
            let (description, has_model) = read_agent_metadata(path);
            agents.push(AgentInfo {
                name: name.clone(),
                scope: AgentScope::User,
                description,
                agent_file_path: path.clone(),
                format: ResourceFormat::Markdown,
                provider: Provider::Claude,
                has_model_property: has_model,
            });
        }
    }

    // Agents in both (repo masks user)
    for (name, _user_path) in &user_agents {
        if repo_names.contains(name.as_str()) {
            let repo_path = repo_agents
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, p)| p)
                .unwrap();
            let (description, has_model) = read_agent_metadata(repo_path);
            agents.push(AgentInfo {
                name: name.clone(),
                scope: AgentScope::RepoMasked,
                description,
                agent_file_path: repo_path.clone(),
                format: ResourceFormat::Markdown,
                provider: Provider::Claude,
                has_model_property: has_model,
            });
        }
    }

    // Apply filters
    let parsed_filters = ResourceFilter::parse_all(filters);
    if !parsed_filters.is_empty() {
        agents.retain(|agent| ResourceFilter::retain(&parsed_filters, &agent.name));
    }

    agents.sort_by(|a, b| a.scope.cmp(&b.scope).then(a.name.cmp(&b.name)));

    let (exceptions, diagnostics) = gather_exceptions(paths, &user_agents, &repo_agents)?;

    Ok(AgentsReport {
        agents,
        exceptions,
        diagnostics,
    })
}

/// Scan an agent directory for `*.md` files, returning `(name, path)` pairs.
fn scan_agent_dir(dir: Option<&PathBuf>) -> Vec<(String, PathBuf)> {
    let Some(dir) = dir else {
        return Vec::new();
    };
    if !dir.exists() {
        return Vec::new();
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_stem().and_then(|n| n.to_str()) {
            Some(name) if name.starts_with('.') => continue,
            Some(name) => name.to_string(),
            None => continue,
        };
        // Only include .md files
        match path.extension().and_then(|e| e.to_str()) {
            Some("md") => {}
            _ => continue,
        }
        results.push((name, path));
    }
    results
}

/// Read description and model property from an agent Markdown file.
fn read_agent_metadata(agent_file: &Path) -> (Option<String>, bool) {
    let content = match fs::read_to_string(agent_file) {
        Ok(c) => c,
        Err(_) => return (None, false),
    };
    let parsed = match parse_markdown_document(&content) {
        Ok(p) => p,
        Err(_) => return (None, false),
    };

    let description = parsed
        .frontmatter
        .get(serde_yaml_ng::Value::String("description".to_string()))
        .and_then(|v| match v {
            serde_yaml_ng::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
            _ => None,
        });

    let has_model = parsed
        .frontmatter
        .contains_key(serde_yaml_ng::Value::String("model".to_string()));

    (description, has_model)
}

/// Gather exceptions across all providers that support agents.
fn gather_exceptions(
    paths: &ProviderSkillPaths,
    user_agents: &[(String, PathBuf)],
    repo_agents: &[(String, PathBuf)],
) -> Result<(Vec<AgentException>, Vec<AgentDirectoryDiagnostic>)> {
    let mut exceptions = Vec::new();
    let mut diagnostics = Vec::new();

    // Canonical agent names from Claude (union of user + repo)
    let canonical: BTreeMap<&str, &PathBuf> = {
        let mut map = BTreeMap::new();
        for (name, path) in user_agents {
            map.insert(name.as_str(), path);
        }
        for (name, path) in repo_agents {
            map.insert(name.as_str(), path);
        }
        map
    };

    // Check canonical agents for invalid and model property
    for (name, path) in &canonical {
        check_yaml_tabs(&mut exceptions, name, path);
        check_invalid(&mut exceptions, name, path);
        check_model_property(&mut exceptions, name, path);
    }

    // Scope-specific agent sets for missing-link checks
    let user_canonical: BTreeMap<&str, &PathBuf> = user_agents
        .iter()
        .map(|(name, path)| (name.as_str(), path))
        .collect();
    let repo_canonical: BTreeMap<&str, &PathBuf> = repo_agents
        .iter()
        .map(|(name, path)| (name.as_str(), path))
        .collect();

    for provider in ALL_PROVIDERS {
        if provider == Provider::Claude {
            continue;
        }

        let caps = capabilities_for(provider);
        let agent_support = caps.support_for(LinkableResource::Agent);
        if !agent_support.level.is_supported() {
            continue;
        }

        // YAML/non-Markdown providers get FormatIncompatible exceptions
        if agent_support.format != Some(ResourceFormat::Markdown) {
            let target_format = agent_support.format;
            for (name, path) in &canonical {
                exceptions.push(AgentException {
                    provider,
                    exception_type: AgentExceptionType::FormatIncompatible,
                    name: name.to_string(),
                    agent_file_path: path.to_path_buf(),
                    missing_properties: Vec::new(),
                    source_format: Some(ResourceFormat::Markdown),
                    target_format,
                });
            }
            continue;
        }

        // Skip providers that also_reads_from Claude
        let also_reads = &agent_support.also_reads_from;
        let reads_claude = also_reads
            .iter()
            .any(|p| p.to_string_lossy().contains(".claude/agents"));
        if reads_claude {
            continue;
        }

        for scope in [ResourceScope::User, ResourceScope::Repo] {
            let scope_agents = match scope {
                ResourceScope::User => &user_canonical,
                ResourceScope::Repo => &repo_canonical,
            };
            check_scope_missing(
                paths,
                provider,
                scope,
                scope_agents,
                &mut exceptions,
                &mut diagnostics,
            );
        }
    }

    exceptions.sort_by(|a, b| {
        a.provider
            .as_slug()
            .cmp(b.provider.as_slug())
            .then(a.exception_type.cmp(&b.exception_type))
            .then(a.name.cmp(&b.name))
    });

    Ok((exceptions, diagnostics))
}

/// Check a single provider scope for missing agents or missing directories.
fn check_scope_missing(
    paths: &ProviderSkillPaths,
    provider: Provider,
    scope: ResourceScope,
    canonical: &BTreeMap<&str, &PathBuf>,
    exceptions: &mut Vec<AgentException>,
    diagnostics: &mut Vec<AgentDirectoryDiagnostic>,
) {
    let scope_label = match scope {
        ResourceScope::User => "user",
        ResourceScope::Repo => "repo",
    };

    let Some(provider_dir) = paths.target_dir(provider, LinkableResource::Agent, scope) else {
        return;
    };

    if !provider_dir.exists() {
        let count = canonical.len();
        let message = format!(
            "All <b><yellow>{count}</yellow> {scope_label}</b> scoped agents are missing for <b>{provider}</b> because the directory for agents doesn't exist!",
        );

        diagnostics.push(AgentDirectoryDiagnostic { provider, message });
        return;
    }

    for name in canonical.keys() {
        let expected = provider_dir.join(format!("{name}.md"));
        if !expected.exists() {
            exceptions.push(AgentException {
                provider,
                exception_type: AgentExceptionType::Missing,
                name: name.to_string(),
                agent_file_path: expected,
                missing_properties: Vec::new(),
                source_format: None,
                target_format: None,
            });
        }
    }
}

fn check_yaml_tabs(exceptions: &mut Vec<AgentException>, name: &str, agent_file: &Path) {
    let content = match fs::read_to_string(agent_file) {
        Ok(c) => c,
        Err(_) => return,
    };

    let has_tabs = match frontmatter_has_indentation_tabs(&content) {
        Ok(has_tabs) => has_tabs,
        Err(_) => return,
    };

    if has_tabs {
        exceptions.push(AgentException {
            provider: Provider::Claude,
            exception_type: AgentExceptionType::YamlTabs,
            name: name.to_string(),
            agent_file_path: agent_file.to_path_buf(),
            missing_properties: Vec::new(),
            source_format: None,
            target_format: None,
        });
    }
}

/// Check if an agent file is missing required frontmatter properties.
fn check_invalid(exceptions: &mut Vec<AgentException>, name: &str, agent_file: &Path) {
    const REQUIRED_PROPERTIES: &[&str] = &["description"];

    let content = match fs::read_to_string(agent_file) {
        Ok(c) => c,
        Err(_) => return,
    };
    let parsed = match parse_markdown_document(&content) {
        Ok(p) => p,
        Err(_) => return,
    };

    let mut missing = Vec::new();
    for &prop in REQUIRED_PROPERTIES {
        let has_prop = parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String(prop.to_string()))
            .map(|v| match v {
                serde_yaml_ng::Value::String(s) => !s.trim().is_empty(),
                _ => false,
            })
            .unwrap_or(false);
        if !has_prop {
            missing.push(prop.to_string());
        }
    }

    if !missing.is_empty() {
        exceptions.push(AgentException {
            provider: Provider::Claude,
            exception_type: AgentExceptionType::Invalid,
            name: name.to_string(),
            agent_file_path: agent_file.to_path_buf(),
            missing_properties: missing,
            source_format: None,
            target_format: None,
        });
    }
}

/// Partition resources into shareable (no Claude-specific properties) and count of skipped.
fn filter_unshareable(items: Vec<(String, PathBuf)>) -> (Vec<(String, PathBuf)>, usize) {
    let mut shareable = Vec::new();
    let mut skipped = 0;
    for entry in items {
        if has_claude_specific_properties(&entry.1) {
            skipped += 1;
        } else {
            shareable.push(entry);
        }
    }
    (shareable, skipped)
}

/// Flag agents that specify a `model` property.
fn check_model_property(exceptions: &mut Vec<AgentException>, name: &str, agent_file: &Path) {
    let content = match fs::read_to_string(agent_file) {
        Ok(c) => c,
        Err(_) => return,
    };
    let parsed = match parse_markdown_document(&content) {
        Ok(p) => p,
        Err(_) => return,
    };

    let has_model = parsed
        .frontmatter
        .contains_key(serde_yaml_ng::Value::String("model".to_string()));

    if has_model {
        exceptions.push(AgentException {
            provider: Provider::Claude,
            exception_type: AgentExceptionType::ModelPropertyNotShareable,
            name: name.to_string(),
            agent_file_path: agent_file.to_path_buf(),
            missing_properties: Vec::new(),
            source_format: None,
            target_format: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_agent(dir: &Path, name: &str, frontmatter: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join(format!("{name}.md")),
            format!("---\n{frontmatter}\n---\nAgent instructions here.\n"),
        )
        .unwrap();
    }

    #[test]
    fn scan_agent_dir_finds_md_files() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("agents");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("researcher.md"), "# Agent").unwrap();
        fs::write(dir.join("not-agent.txt"), "nope").unwrap();
        fs::write(dir.join(".hidden.md"), "hidden").unwrap();

        let results = scan_agent_dir(Some(&dir));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "researcher");
    }

    #[test]
    fn scan_agent_dir_returns_empty_for_missing() {
        let results = scan_agent_dir(None);
        assert!(results.is_empty());

        let results = scan_agent_dir(Some(&PathBuf::from("/nonexistent")));
        assert!(results.is_empty());
    }

    #[test]
    fn read_agent_metadata_extracts_description_and_model() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.md");
        fs::write(
            &file,
            "---\ndescription: A test agent\nmodel: claude-sonnet-4-5-20250514\n---\nContent\n",
        )
        .unwrap();

        let (desc, has_model) = read_agent_metadata(&file);
        assert_eq!(desc.unwrap(), "A test agent");
        assert!(has_model);
    }

    #[test]
    fn read_agent_metadata_no_model() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.md");
        fs::write(&file, "---\ndescription: Simple agent\n---\nContent\n").unwrap();

        let (desc, has_model) = read_agent_metadata(&file);
        assert_eq!(desc.unwrap(), "Simple agent");
        assert!(!has_model);
    }

    #[test]
    fn scope_classification_user_only() {
        let tmp = TempDir::new().unwrap();
        let paths = test_agent_paths(tmp.path());
        let user_dir = paths
            .target_dir(
                Provider::Claude,
                LinkableResource::Agent,
                ResourceScope::User,
            )
            .unwrap();
        setup_agent(&user_dir, "researcher", "description: Research agent");

        let report = list_agents(&paths, &[]).unwrap();
        assert_eq!(report.agents.len(), 1);
        assert_eq!(report.agents[0].scope, AgentScope::User);
    }

    #[test]
    fn scope_classification_repo_masks_user() {
        let tmp = TempDir::new().unwrap();
        let paths = test_agent_paths(tmp.path());
        let user_dir = paths
            .target_dir(
                Provider::Claude,
                LinkableResource::Agent,
                ResourceScope::User,
            )
            .unwrap();
        let repo_dir = paths
            .target_dir(
                Provider::Claude,
                LinkableResource::Agent,
                ResourceScope::Repo,
            )
            .unwrap();
        setup_agent(&user_dir, "researcher", "description: User version");
        setup_agent(&repo_dir, "researcher", "description: Repo version");

        let report = list_agents(&paths, &[]).unwrap();
        assert_eq!(report.agents.len(), 1);
        assert_eq!(report.agents[0].scope, AgentScope::RepoMasked);
        assert_eq!(
            report.agents[0].description.as_deref(),
            Some("Repo version")
        );
    }

    #[test]
    fn filter_applies_to_agents() {
        let tmp = TempDir::new().unwrap();
        let paths = test_agent_paths(tmp.path());
        let user_dir = paths
            .target_dir(
                Provider::Claude,
                LinkableResource::Agent,
                ResourceScope::User,
            )
            .unwrap();
        setup_agent(&user_dir, "researcher", "description: Research");
        setup_agent(&user_dir, "coder", "description: Coding");

        let report = list_agents(&paths, &["researcher".into()]).unwrap();
        assert_eq!(report.agents.len(), 1);
        assert_eq!(report.agents[0].name, "researcher");
    }

    #[test]
    fn model_property_flagged_as_exception() {
        let tmp = TempDir::new().unwrap();
        let paths = test_agent_paths(tmp.path());
        let user_dir = paths
            .target_dir(
                Provider::Claude,
                LinkableResource::Agent,
                ResourceScope::User,
            )
            .unwrap();
        setup_agent(
            &user_dir,
            "model-agent",
            "description: Has model\nmodel: claude-sonnet-4-5-20250514",
        );

        let report = list_agents(&paths, &[]).unwrap();
        let model_exceptions: Vec<_> = report
            .exceptions
            .iter()
            .filter(|e| e.exception_type == AgentExceptionType::ModelPropertyNotShareable)
            .collect();
        assert_eq!(model_exceptions.len(), 1);
        assert_eq!(model_exceptions[0].name, "model-agent");
    }

    #[test]
    fn detects_and_fixes_yaml_tabs() {
        let tmp = TempDir::new().unwrap();
        let paths = test_agent_paths(tmp.path());
        let user_dir = paths
            .target_dir(
                Provider::Claude,
                LinkableResource::Agent,
                ResourceScope::User,
            )
            .unwrap();
        fs::create_dir_all(&user_dir).unwrap();
        let agent_file = user_dir.join("tabbed.md");
        fs::write(
            &agent_file,
            "---\ndescription: Has tabbed yaml\nprompt: |-\n\tline one\n\t\tline two\n---\nAgent instructions here.\n",
        )
        .unwrap();

        let report = list_agents(&paths, &[]).unwrap();
        let yaml_tabs: Vec<_> = report
            .exceptions
            .iter()
            .filter(|e| e.exception_type == AgentExceptionType::YamlTabs)
            .collect();
        assert_eq!(yaml_tabs.len(), 1);
        assert_eq!(yaml_tabs[0].name, "tabbed");

        let summary = fix_missing_agents(&paths).unwrap();
        assert_eq!(summary.yaml_tabs_fixed, 1);

        let content = fs::read_to_string(&agent_file).unwrap();
        assert!(!frontmatter_has_indentation_tabs(&content).unwrap());
    }

    fn test_agent_paths(base: &Path) -> ProviderSkillPaths {
        use super::super::paths::ProviderPaths;
        use std::collections::HashMap;

        let mut providers = HashMap::new();
        for provider in ALL_PROVIDERS {
            providers.insert(
                provider,
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
                },
            );
        }

        providers.insert(
            Provider::Claude,
            ProviderPaths {
                provider: Provider::Claude,
                user_skills: None,
                repo_skills: None,
                user_commands: None,
                repo_commands: None,
                user_agents: Some(base.join("claude/agents")),
                repo_agents: Some(base.join("repo/.claude/agents")),
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
            },
        );

        ProviderSkillPaths::from_providers_for_test(providers, base.to_path_buf())
    }
}
