use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::serde_yaml_ng;

use crate::error::Result;
use crate::events::Provider;

use super::capabilities::{ALL_PROVIDERS, LinkableResource, ResourceFormat, capabilities_for};
use super::compatibility::parse_markdown_document;
use super::filter::ResourceFilter;
use super::paths::{ProviderSkillPaths, ResourceScope};
use super::symlink::{LinkResult, create_resource_link};

#[derive(Debug, Clone)]
struct ScannedCommand {
    name: String,
    path: PathBuf,
    relative_path: PathBuf,
}

/// Scope classification for a discovered slash command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandScope {
    /// Only in user-level commands directory (~/.claude/commands/).
    User,
    /// Present in both user and repo — repo wins (masks user).
    RepoMasked,
    /// Only in repo-level commands directory (.claude/commands/).
    Repo,
}

/// Information about a single discovered slash command.
#[derive(Debug, Clone)]
pub struct CommandInfo {
    /// Command name (file stem).
    pub name: String,
    /// Description from frontmatter.
    pub description: Option<String>,
    /// Scope classification.
    pub scope: CommandScope,
    /// Path to the command file.
    pub command_file_path: PathBuf,
    /// Provider where this command was discovered.
    pub provider: Provider,
    /// Full frontmatter as key-value pairs (for detail view).
    pub frontmatter: BTreeMap<String, String>,
    /// Whether the command file contains a `model` property.
    pub has_model_property: bool,
}

/// Type of exception found during command validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandExceptionType {
    /// Command file missing for a provider.
    Missing,
    /// Command file missing required frontmatter properties.
    Invalid,
    /// No links in command body.
    NoLinks,
    /// Command specifies a `model` property that limits cross-provider shareability.
    ModelPropertyNotShareable,
    /// Format incompatible with target provider.
    FormatIncompatible,
}

impl std::fmt::Display for CommandExceptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandExceptionType::Missing => write!(f, "missing"),
            CommandExceptionType::Invalid => write!(f, "invalid"),
            CommandExceptionType::NoLinks => write!(f, "no-links"),
            CommandExceptionType::ModelPropertyNotShareable => write!(f, "model-not-shareable"),
            CommandExceptionType::FormatIncompatible => write!(f, "format-incompatible"),
        }
    }
}

/// A single exception found during command validation.
#[derive(Debug, Clone)]
pub struct CommandException {
    /// Provider where the exception was found.
    pub provider: Provider,
    /// Type of exception.
    pub exception_type: CommandExceptionType,
    /// Command name.
    pub name: String,
    /// Path to the command file (or expected location).
    pub command_file_path: PathBuf,
    /// For `Invalid`: names of missing required frontmatter properties.
    pub missing_properties: Vec<String>,
    /// For `FormatIncompatible`: source format.
    pub source_format: Option<ResourceFormat>,
    /// For `FormatIncompatible`: target format.
    pub target_format: Option<ResourceFormat>,
    /// For `ModelPropertyNotShareable`: the model value.
    pub model_value: Option<String>,
}

/// Directory-level diagnostic explaining why all commands for a scope are missing.
#[derive(Debug, Clone)]
pub struct CommandDirectoryDiagnostic {
    /// Provider where the issue was found.
    pub provider: Provider,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Complete report from command discovery.
#[derive(Debug, Clone)]
pub struct CommandsReport {
    /// All discovered commands with scope classification.
    pub commands: Vec<CommandInfo>,
    /// Exceptions found during validation.
    pub exceptions: Vec<CommandException>,
    /// Directory-level diagnostics (missing directories).
    pub diagnostics: Vec<CommandDirectoryDiagnostic>,
}

/// Summary of fix operations applied by [`fix_missing_commands`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CommandFixSummary {
    /// Directories created for providers that had no commands directory.
    pub directories_created: usize,
    /// Symlinks created for individual missing commands.
    pub links_created: usize,
    /// Commands that were already correctly linked.
    pub already_linked: usize,
    /// Operations skipped.
    pub skipped: usize,
    /// Commands where format is incompatible with target provider.
    pub format_incompatible: usize,
}

/// Fix missing command links for non-Claude providers.
pub fn fix_missing_commands(paths: &ProviderSkillPaths) -> Result<CommandFixSummary> {
    let mut summary = CommandFixSummary::default();

    let user_dir = paths.target_dir(
        Provider::Claude,
        LinkableResource::Command,
        ResourceScope::User,
    );
    let repo_dir = paths.target_dir(
        Provider::Claude,
        LinkableResource::Command,
        ResourceScope::Repo,
    );

    let user_commands = scan_command_dir(user_dir.as_ref());
    let repo_commands = scan_command_dir(repo_dir.as_ref());

    for provider in ALL_PROVIDERS {
        if provider == Provider::Claude {
            continue;
        }

        let caps = capabilities_for(provider);
        let cmd_support = caps.support_for(LinkableResource::Command);
        if !cmd_support.level.is_supported() {
            continue;
        }

        // Only link to Markdown-compatible providers
        if cmd_support.format != Some(ResourceFormat::Markdown) {
            let count = user_commands.len() + repo_commands.len();
            summary.format_incompatible += count;
            continue;
        }

        for scope in [ResourceScope::User, ResourceScope::Repo] {
            let source_root = match scope {
                ResourceScope::User => user_dir.as_deref(),
                ResourceScope::Repo => repo_dir.as_deref(),
            };
            let reads_claude = paths
                .also_reads_from(provider, LinkableResource::Command, scope)
                .iter()
                .any(|p| p.to_string_lossy().contains(".claude/commands"));
            if reads_claude {
                continue;
            }

            let source_commands = match scope {
                ResourceScope::User => &user_commands,
                ResourceScope::Repo => &repo_commands,
            };

            if source_commands.is_empty() {
                continue;
            }

            let Some(provider_dir) = paths.target_dir(provider, LinkableResource::Command, scope)
            else {
                continue;
            };

            if !provider_dir.exists() {
                fs::create_dir_all(&provider_dir)?;
                summary.directories_created += 1;
            }

            let Some(source_root) = source_root else {
                continue;
            };

            fix_scope_commands(
                source_commands,
                source_root,
                &provider_dir,
                scope,
                &mut summary,
            )?;
        }
    }

    Ok(summary)
}

/// Create symlinks for missing commands in a single (provider, scope) pair.
fn fix_scope_commands(
    source_commands: &[ScannedCommand],
    source_root: &Path,
    provider_dir: &Path,
    scope: ResourceScope,
    summary: &mut CommandFixSummary,
) -> Result<()> {
    for command in source_commands {
        match create_resource_link(&command.path, source_root, provider_dir, scope)? {
            LinkResult::Linked { .. } => summary.links_created += 1,
            LinkResult::AlreadyLinked => summary.already_linked += 1,
            LinkResult::Skipped { .. } => summary.skipped += 1,
        }
    }
    Ok(())
}

/// Discover all commands from Claude's user and repo command directories.
pub fn list_commands(paths: &ProviderSkillPaths, filters: &[String]) -> Result<CommandsReport> {
    let user_dir = paths.target_dir(
        Provider::Claude,
        LinkableResource::Command,
        ResourceScope::User,
    );
    let repo_dir = paths.target_dir(
        Provider::Claude,
        LinkableResource::Command,
        ResourceScope::Repo,
    );

    let user_commands = scan_command_dir(user_dir.as_ref());
    let repo_commands = scan_command_dir(repo_dir.as_ref());

    let user_names: BTreeSet<&str> = user_commands.iter().map(|cmd| cmd.name.as_str()).collect();
    let repo_names: BTreeSet<&str> = repo_commands.iter().map(|cmd| cmd.name.as_str()).collect();

    let mut commands = Vec::new();

    // Repo-only commands
    for command in &repo_commands {
        if !user_names.contains(command.name.as_str()) {
            let (description, frontmatter, has_model) = read_command_metadata(&command.path);
            commands.push(CommandInfo {
                name: command.name.clone(),
                scope: CommandScope::Repo,
                description,
                command_file_path: command.path.clone(),
                provider: Provider::Claude,
                frontmatter,
                has_model_property: has_model,
            });
        }
    }

    // User-only commands
    for command in &user_commands {
        if !repo_names.contains(command.name.as_str()) {
            let (description, frontmatter, has_model) = read_command_metadata(&command.path);
            commands.push(CommandInfo {
                name: command.name.clone(),
                scope: CommandScope::User,
                description,
                command_file_path: command.path.clone(),
                provider: Provider::Claude,
                frontmatter,
                has_model_property: has_model,
            });
        }
    }

    // Commands in both (repo masks user)
    for command in &user_commands {
        if repo_names.contains(command.name.as_str()) {
            let repo_path = repo_commands
                .iter()
                .find(|candidate| candidate.name == command.name)
                .map(|candidate| &candidate.path)
                .unwrap();
            let (description, frontmatter, has_model) = read_command_metadata(repo_path);
            commands.push(CommandInfo {
                name: command.name.clone(),
                scope: CommandScope::RepoMasked,
                description,
                command_file_path: repo_path.clone(),
                provider: Provider::Claude,
                frontmatter,
                has_model_property: has_model,
            });
        }
    }

    // Apply filters
    let parsed_filters = ResourceFilter::parse_all(filters);
    if !parsed_filters.is_empty() {
        commands.retain(|cmd| ResourceFilter::retain(&parsed_filters, &cmd.name));
    }

    commands.sort_by(|a, b| a.scope.cmp(&b.scope).then(a.name.cmp(&b.name)));

    let (exceptions, diagnostics) = gather_exceptions(paths, &user_commands, &repo_commands)?;

    Ok(CommandsReport {
        commands,
        exceptions,
        diagnostics,
    })
}

/// Scan a command directory for `*.md` files, returning namespaced command metadata.
fn scan_command_dir(dir: Option<&PathBuf>) -> Vec<ScannedCommand> {
    let Some(dir) = dir else {
        return Vec::new();
    };
    if !dir.exists() {
        return Vec::new();
    }

    let mut results = Vec::new();
    scan_command_dir_recursive(dir, dir, &mut results);
    results.sort_by(|left, right| left.name.cmp(&right.name).then(left.path.cmp(&right.path)));
    results
}

fn scan_command_dir_recursive(root: &Path, current: &Path, results: &mut Vec<ScannedCommand>) {
    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if file_name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            scan_command_dir_recursive(root, &path, results);
            continue;
        }

        if !matches!(path.extension().and_then(|e| e.to_str()), Some("md")) {
            continue;
        }

        let Some(relative_path) = path.strip_prefix(root).ok().map(Path::to_path_buf) else {
            continue;
        };
        let Some(name) = command_name_from_relative_path(&relative_path) else {
            continue;
        };

        results.push(ScannedCommand {
            name,
            path,
            relative_path,
        });
    }
}

fn command_name_from_relative_path(relative_path: &Path) -> Option<String> {
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

/// Read description, frontmatter, and model property from a command file.
fn read_command_metadata(command_file: &Path) -> (Option<String>, BTreeMap<String, String>, bool) {
    let content = match fs::read_to_string(command_file) {
        Ok(c) => c,
        Err(_) => return (None, BTreeMap::new(), false),
    };
    let parsed = match parse_markdown_document(&content) {
        Ok(p) => p,
        Err(_) => return (None, BTreeMap::new(), false),
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

    let mut fm = BTreeMap::new();
    for (k, v) in &parsed.frontmatter {
        if let (serde_yaml_ng::Value::String(key), serde_yaml_ng::Value::String(val)) = (k, v) {
            fm.insert(key.clone(), val.clone());
        }
    }

    (description, fm, has_model)
}

/// Gather exceptions across all providers that support commands.
fn gather_exceptions(
    paths: &ProviderSkillPaths,
    user_commands: &[ScannedCommand],
    repo_commands: &[ScannedCommand],
) -> Result<(Vec<CommandException>, Vec<CommandDirectoryDiagnostic>)> {
    let mut exceptions = Vec::new();
    let mut diagnostics = Vec::new();

    let canonical: BTreeMap<&str, &ScannedCommand> = {
        let mut map = BTreeMap::new();
        for command in user_commands {
            map.insert(command.name.as_str(), command);
        }
        for command in repo_commands {
            map.insert(command.name.as_str(), command);
        }
        map
    };

    // Check canonical commands for model property
    for (name, command) in &canonical {
        check_model_property(&mut exceptions, name, &command.path);
    }

    let user_canonical: BTreeMap<&str, &ScannedCommand> = user_commands
        .iter()
        .map(|command| (command.name.as_str(), command))
        .collect();
    let repo_canonical: BTreeMap<&str, &ScannedCommand> = repo_commands
        .iter()
        .map(|command| (command.name.as_str(), command))
        .collect();

    for provider in ALL_PROVIDERS {
        if provider == Provider::Claude {
            continue;
        }

        let caps = capabilities_for(provider);
        let cmd_support = caps.support_for(LinkableResource::Command);
        if !cmd_support.level.is_supported() {
            continue;
        }

        // Non-Markdown providers get FormatIncompatible exceptions
        if cmd_support.format != Some(ResourceFormat::Markdown) {
            let target_format = cmd_support.format;
            for (name, command) in &canonical {
                exceptions.push(CommandException {
                    provider,
                    exception_type: CommandExceptionType::FormatIncompatible,
                    name: name.to_string(),
                    command_file_path: command.path.clone(),
                    missing_properties: Vec::new(),
                    source_format: Some(ResourceFormat::Markdown),
                    target_format,
                    model_value: None,
                });
            }
            continue;
        }

        // Skip providers that also_reads_from Claude
        let also_reads = &cmd_support.also_reads_from;
        let reads_claude = also_reads
            .iter()
            .any(|p| p.to_string_lossy().contains(".claude/commands"));
        if reads_claude {
            continue;
        }

        for scope in [ResourceScope::User, ResourceScope::Repo] {
            let scope_commands = match scope {
                ResourceScope::User => &user_canonical,
                ResourceScope::Repo => &repo_canonical,
            };
            check_scope_missing(
                paths,
                provider,
                scope,
                scope_commands,
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

/// Check a single provider scope for missing commands or missing directories.
fn check_scope_missing(
    paths: &ProviderSkillPaths,
    provider: Provider,
    scope: ResourceScope,
    canonical: &BTreeMap<&str, &ScannedCommand>,
    exceptions: &mut Vec<CommandException>,
    diagnostics: &mut Vec<CommandDirectoryDiagnostic>,
) {
    let scope_label = match scope {
        ResourceScope::User => "user",
        ResourceScope::Repo => "repo",
    };

    let Some(provider_dir) = paths.target_dir(provider, LinkableResource::Command, scope) else {
        return;
    };

    if !provider_dir.exists() {
        let count = canonical.len();
        let message = format!(
            "All <b><yellow>{count}</yellow> {scope_label}</b> scoped commands are missing for <b>{provider}</b> because the directory for commands doesn't exist!",
        );

        diagnostics.push(CommandDirectoryDiagnostic { provider, message });
        return;
    }

    for (name, command) in canonical {
        let expected = provider_dir.join(&command.relative_path);
        if !expected.exists() {
            exceptions.push(CommandException {
                provider,
                exception_type: CommandExceptionType::Missing,
                name: name.to_string(),
                command_file_path: expected,
                missing_properties: Vec::new(),
                source_format: None,
                target_format: None,
                model_value: None,
            });
        }
    }
}

/// Flag commands that specify a `model` property.
fn check_model_property(exceptions: &mut Vec<CommandException>, name: &str, command_file: &Path) {
    let content = match fs::read_to_string(command_file) {
        Ok(c) => c,
        Err(_) => return,
    };
    let parsed = match parse_markdown_document(&content) {
        Ok(p) => p,
        Err(_) => return,
    };

    let model_value = parsed
        .frontmatter
        .get(serde_yaml_ng::Value::String("model".to_string()))
        .and_then(|v| match v {
            serde_yaml_ng::Value::String(s) => Some(s.clone()),
            _ => None,
        });

    if let Some(val) = model_value {
        exceptions.push(CommandException {
            provider: Provider::Claude,
            exception_type: CommandExceptionType::ModelPropertyNotShareable,
            name: name.to_string(),
            command_file_path: command_file.to_path_buf(),
            missing_properties: Vec::new(),
            source_format: None,
            target_format: None,
            model_value: Some(val),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_command(dir: &Path, name: &str, frontmatter: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join(format!("{name}.md")),
            format!("---\n{frontmatter}\n---\nCommand prompt here.\n"),
        )
        .unwrap();
    }

    #[test]
    fn scan_command_dir_finds_md_files() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("commands");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("commit.md"), "# Commit").unwrap();
        fs::write(dir.join("readme.txt"), "nope").unwrap();

        let results = scan_command_dir(Some(&dir));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "commit");
    }

    #[test]
    fn scan_command_dir_preserves_namespace_from_nested_directories() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("commands");
        let nested = dir.join("prompts");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("create-deep-dive-document.md"), "# Prompt").unwrap();

        let results = scan_command_dir(Some(&dir));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "prompts:create-deep-dive-document");
        assert_eq!(
            results[0].relative_path,
            PathBuf::from("prompts/create-deep-dive-document.md")
        );
    }

    #[test]
    fn read_command_metadata_extracts_all() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.md");
        fs::write(
            &file,
            "---\ndescription: A test command\nmodel: opus\nmode: plan\n---\nContent\n",
        )
        .unwrap();

        let (desc, fm, has_model) = read_command_metadata(&file);
        assert_eq!(desc.unwrap(), "A test command");
        assert!(has_model);
        assert_eq!(fm.get("mode").unwrap(), "plan");
    }

    #[test]
    fn scope_classification_user_only() {
        let tmp = TempDir::new().unwrap();
        let paths = test_command_paths(tmp.path());
        let user_dir = paths
            .target_dir(
                Provider::Claude,
                LinkableResource::Command,
                ResourceScope::User,
            )
            .unwrap();
        setup_command(&user_dir, "commit", "description: Commit helper");

        let report = list_commands(&paths, &[]).unwrap();
        assert_eq!(report.commands.len(), 1);
        assert_eq!(report.commands[0].scope, CommandScope::User);
    }

    #[test]
    fn model_property_flagged_as_exception() {
        let tmp = TempDir::new().unwrap();
        let paths = test_command_paths(tmp.path());
        let user_dir = paths
            .target_dir(
                Provider::Claude,
                LinkableResource::Command,
                ResourceScope::User,
            )
            .unwrap();
        setup_command(
            &user_dir,
            "model-cmd",
            "description: Has model\nmodel: opus",
        );

        let report = list_commands(&paths, &[]).unwrap();
        let model_exceptions: Vec<_> = report
            .exceptions
            .iter()
            .filter(|e| e.exception_type == CommandExceptionType::ModelPropertyNotShareable)
            .collect();
        assert_eq!(model_exceptions.len(), 1);
        assert_eq!(model_exceptions[0].model_value.as_deref(), Some("opus"));
    }

    #[cfg(unix)]
    #[test]
    fn fix_missing_commands_preserves_nested_directories() {
        use super::super::paths::ProviderPaths;
        use std::collections::HashMap;

        let tmp = TempDir::new().unwrap();
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
                user_commands: Some(tmp.path().join("home/.claude/commands")),
                repo_commands: Some(tmp.path().join("repo/.claude/commands")),
                user_agents: None,
                repo_agents: None,
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
            },
        );
        providers.insert(
            Provider::Codex,
            ProviderPaths {
                provider: Provider::Codex,
                user_skills: None,
                repo_skills: None,
                user_commands: Some(tmp.path().join("home/.codex/prompts")),
                repo_commands: None,
                user_agents: None,
                repo_agents: None,
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
            },
        );

        let paths = ProviderSkillPaths::from_providers_for_test(providers, tmp.path().join("home"));
        let user_source = tmp
            .path()
            .join("home/.claude/commands/prompts/create-deep-dive-document.md");
        fs::create_dir_all(user_source.parent().unwrap()).unwrap();
        fs::write(
            &user_source,
            "---\ndescription: Prompt\n---\nCreate a deep dive.\n",
        )
        .unwrap();

        let summary = fix_missing_commands(&paths).unwrap();
        assert_eq!(summary.links_created, 1);

        let dest = tmp
            .path()
            .join("home/.codex/prompts/prompts/create-deep-dive-document.md");
        let metadata = fs::symlink_metadata(&dest).unwrap();
        assert!(metadata.file_type().is_symlink());
        assert_eq!(
            fs::read_link(&dest).unwrap(),
            tmp.path()
                .join("home/.claude/commands/prompts/create-deep-dive-document.md")
        );
    }

    fn test_command_paths(base: &Path) -> ProviderSkillPaths {
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
                user_commands: Some(base.join("claude/commands")),
                repo_commands: Some(base.join("repo/.claude/commands")),
                user_agents: None,
                repo_agents: None,
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
            },
        );

        ProviderSkillPaths::from_providers_for_test(providers, base.to_path_buf())
    }
}
