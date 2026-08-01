pub mod agents;
mod canonical;
pub(crate) mod capabilities;
pub mod commands;
mod compatibility;
mod conflict;
mod detector;
mod discovery;
mod filter;
mod hashing;
pub mod model;
mod paths;
mod report;
pub mod skills;
mod symlink;

pub use agents::{
    AgentDirectoryDiagnostic, AgentException, AgentExceptionType, AgentFixSummary, AgentInfo,
    AgentScope, AgentsReport, fix_missing_agents, list_agents,
};
pub use canonical::{
    CanonicalSelection, canonical_provider, preference_prompt_count, ranked_provider_preferences,
    select_canonical_provider, set_canonical_provider,
};
pub use capabilities::{
    ALL_PROVIDERS, LinkableResource, ProviderCapabilities, ResourceFormat, ResourcePropertySchema,
    ResourceSupport, SkillFrontmatter, SupportLevel, capabilities_for,
};
pub use commands::{
    CommandDirectoryDiagnostic, CommandException, CommandExceptionType, CommandFixSummary,
    CommandInfo, CommandScope, CommandsReport, fix_missing_commands, list_commands,
};
pub use compatibility::{classify_canonical_candidate, classify_target_reference};
pub use conflict::SkillSyncStatus;
pub use detector::{
    AgentDefinitionsDetector, DiscoveredResource, LinkDetector, SharedScriptsDetector,
    SkillsDetector, SlashCommandsDetector,
};
pub use discovery::DiscoveredSkill;
pub use filter::ResourceFilter;
pub use paths::{ProviderPaths, ProviderSkillPaths, ResourceScope, resolve_repo_root};
pub use report::{ConflictEntry, InSyncEntry, LinkReport, LinkedEntry, SkippedEntry};
pub use skills::{
    ExceptionType, SkillDirectoryDiagnostic, SkillException, SkillFilter, SkillFixSummary,
    SkillInfo, SkillScope, SkillsReport, fix_missing_skills, list_skills,
};
pub use symlink::{LinkResult, category_link_target, relative_path};

use crate::error::Result;
use tracing::{debug, info_span};

use conflict::build_also_reads_from;

/// Orchestrate skill and command linking across providers.
pub fn link_skills(
    scope: ResourceScope,
    filter: Option<&str>,
    dry_run: bool,
) -> Result<LinkReport> {
    let _span = info_span!(
        "link_skills",
        ?scope,
        dry_run,
        filter = filter.unwrap_or("")
    )
    .entered();
    link_skills_inner(&ProviderSkillPaths::new(), scope, filter, dry_run)
}

/// Core linking logic, injectable for tests.
fn link_skills_inner(
    provider_paths: &ProviderSkillPaths,
    scope: ResourceScope,
    filter: Option<&str>,
    dry_run: bool,
) -> Result<LinkReport> {
    let mut report = LinkReport::default();
    report_category_level_symlinks(provider_paths, scope, LinkableResource::Skill, &mut report)?;

    let mut skills = discovery::discover_skills(provider_paths, scope)?;
    if let Some(name_filter) = filter {
        skills.retain(|skill| skill.name == name_filter);
    }

    debug!(skill_count = skills.len(), "discovered skills for linking");

    for skill in &mut skills {
        skill.hash = hashing::hash_skill_dir(&skill.path).ok();
    }

    let providers: Vec<crate::provider::Provider> = provider_paths
        .for_scope(scope)
        .iter()
        .map(|(provider, _)| *provider)
        .collect();
    let also_reads =
        build_also_reads_from(provider_paths, &providers, LinkableResource::Skill, scope);
    let statuses = conflict::analyze_skills(skills, &providers, &also_reads);

    apply_statuses(
        statuses,
        provider_paths,
        scope,
        dry_run,
        &mut report,
        LinkableResource::Skill,
    )?;

    link_commands(provider_paths, scope, filter, dry_run, &mut report)?;

    Ok(report)
}

fn apply_statuses(
    statuses: Vec<SkillSyncStatus>,
    provider_paths: &ProviderSkillPaths,
    scope: ResourceScope,
    dry_run: bool,
    report: &mut LinkReport,
    resource: LinkableResource,
) -> Result<()> {
    let prefix = match resource {
        LinkableResource::Command => "cmd:",
        _ => "",
    };

    for status in statuses {
        match status {
            SkillSyncStatus::LinkCandidate {
                source,
                target_providers,
            } => {
                let mut linked: Vec<String> = Vec::new();
                for target_provider in target_providers {
                    let Some(dest_dir) =
                        provider_paths.target_dir(target_provider, resource, scope)
                    else {
                        continue;
                    };

                    if !matches_scope_style(&source.path, &dest_dir, scope) {
                        report.skipped.push(SkippedEntry {
                            name: format!("{prefix}{}", source.name),
                            reason: format!(
                                "{}: skipped due to scope isolation mismatch (source: {}, destination: {})",
                                target_provider,
                                biscuit_file::to_portable_string(&source.path),
                                biscuit_file::to_portable_string(&dest_dir)
                            ),
                        });
                        continue;
                    }

                    if let Some(category_target) = symlink::category_link_target(&dest_dir)? {
                        report.skipped.push(SkippedEntry {
                            name: format!("{prefix}{}", source.name),
                            reason: format!(
                                "{}: category-level symlink at {} -> {}. Keep as-is; switch to granular links manually.",
                                target_provider,
                                biscuit_file::to_portable_string(&dest_dir),
                                biscuit_file::to_portable_string(&category_target)
                            ),
                        });
                        continue;
                    }

                    if dry_run {
                        linked.push(target_provider.to_string());
                        continue;
                    }

                    let link_result = if let Some(source_root) =
                        provider_paths.resource_dir(source.provider, resource, scope)
                    {
                        if source.path.starts_with(&source_root) {
                            symlink::create_resource_link(
                                &source.path,
                                &source_root,
                                &dest_dir,
                                scope,
                            )?
                        } else {
                            symlink::create_skill_link(&source.path, &dest_dir, scope)?
                        }
                    } else {
                        symlink::create_skill_link(&source.path, &dest_dir, scope)?
                    };

                    match link_result {
                        LinkResult::Linked { .. } => linked.push(target_provider.to_string()),
                        LinkResult::AlreadyLinked => {}
                        LinkResult::Skipped { reason } => {
                            report.skipped.push(SkippedEntry {
                                name: format!("{prefix}{}", source.name),
                                reason: format!("{}: {reason}", target_provider),
                            });
                        }
                    }
                }

                if !linked.is_empty() {
                    report.linked.push(LinkedEntry {
                        name: format!("{prefix}{}", source.name),
                        source_provider: source.provider.to_string(),
                        target_providers: linked,
                    });
                }
            }
            SkillSyncStatus::InSync { name, providers } => {
                report.in_sync.push(InSyncEntry {
                    name: format!("{prefix}{name}"),
                    providers: providers
                        .iter()
                        .map(|(provider, _, _)| provider.to_string())
                        .collect(),
                });
            }
            SkillSyncStatus::Conflict { name, versions } => {
                report.conflicts.push(ConflictEntry {
                    name: format!("{prefix}{name}"),
                    versions: versions
                        .into_iter()
                        .map(|(provider, path, hash)| (provider.to_string(), path, hash))
                        .collect(),
                });
            }
            SkillSyncStatus::AlreadyLinked { name, .. } => {
                report.already_linked.push(format!("{prefix}{name}"));
            }
        }
    }

    Ok(())
}

/// Internal path style validation.
///
/// Both scopes operate on absolute filesystem paths during discovery/apply.
/// Relative path policy for repo scope is enforced by symlink target creation.
fn matches_scope_style(
    source: &std::path::Path,
    dest: &std::path::Path,
    _scope: ResourceScope,
) -> bool {
    source.is_absolute() && dest.is_absolute()
}

/// Link commands across providers that support Markdown command files.
fn link_commands(
    provider_paths: &ProviderSkillPaths,
    scope: ResourceScope,
    filter: Option<&str>,
    dry_run: bool,
    report: &mut LinkReport,
) -> Result<()> {
    report_category_level_symlinks(provider_paths, scope, LinkableResource::Command, report)?;

    let mut commands = discovery::discover_commands(provider_paths, scope)?;
    if let Some(name_filter) = filter {
        commands.retain(|command| command.name == name_filter);
    }

    for command in &mut commands {
        if let Ok(content) = std::fs::read(&command.path) {
            command.hash = Some(biscuit_hash::xx_hash_bytes(&content));
        }
    }

    let providers: Vec<crate::provider::Provider> = provider_paths
        .commands_for_scope(scope)
        .iter()
        .map(|(provider, _)| *provider)
        .collect();
    let also_reads =
        build_also_reads_from(provider_paths, &providers, LinkableResource::Command, scope);
    let statuses = conflict::analyze_skills(commands, &providers, &also_reads);

    apply_statuses(
        statuses,
        provider_paths,
        scope,
        dry_run,
        report,
        LinkableResource::Command,
    )
}

fn report_category_level_symlinks(
    provider_paths: &ProviderSkillPaths,
    scope: ResourceScope,
    resource: LinkableResource,
    report: &mut LinkReport,
) -> Result<()> {
    let roots: Vec<(crate::provider::Provider, &std::path::PathBuf)> = match resource {
        LinkableResource::Skill => provider_paths.for_scope(scope),
        LinkableResource::Command => provider_paths.commands_for_scope(scope),
        LinkableResource::Agent => provider_paths.agents_for_scope(scope),
        LinkableResource::Script => vec![],
    };

    let resource_label = match resource {
        LinkableResource::Skill => "skill-root",
        LinkableResource::Command => "command-root",
        LinkableResource::Agent => "agent-root",
        LinkableResource::Script => "script-root",
    };

    for (provider, root_dir) in roots {
        if let Some(target) = symlink::category_link_target(root_dir)? {
            report.skipped.push(SkippedEntry {
                name: format!("{resource_label}:{provider}"),
                reason: format!(
                    "{provider}: category-level symlink at {} -> {}. Keep as-is; switch to granular links manually.",
                    biscuit_file::to_portable_string(&root_dir),
                    biscuit_file::to_portable_string(&target)
                ),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempfile::TempDir;

    use super::*;
    use crate::provider::Provider;
    fn empty_provider(provider: Provider) -> ProviderPaths {
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

    fn test_paths(base: &std::path::Path, opencode_reads_from_claude: bool) -> ProviderSkillPaths {
        let mut providers = HashMap::new();
        for provider in ALL_PROVIDERS {
            providers.insert(provider, empty_provider(provider));
        }

        providers.insert(
            Provider::Claude,
            ProviderPaths {
                provider: Provider::Claude,
                user_skills: Some(base.join("claude/skills")),
                repo_skills: Some(base.join("repo/.claude/skills")),
                user_commands: Some(base.join("claude/commands")),
                repo_commands: Some(base.join("repo/.claude/commands")),
                user_agents: Some(base.join("claude/agents")),
                repo_agents: Some(base.join("repo/.claude/agents")),
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
            },
        );
        providers.insert(
            Provider::Codex,
            ProviderPaths {
                provider: Provider::Codex,
                user_skills: Some(base.join("codex/skills")),
                repo_skills: Some(base.join("repo/.codex/skills")),
                user_commands: Some(base.join("codex/commands")),
                repo_commands: Some(base.join("repo/.codex/commands")),
                user_agents: Some(base.join("codex/agents")),
                repo_agents: Some(base.join("repo/.codex/agents")),
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
                user_agents: Some(base.join("gemini/agents")),
                repo_agents: Some(base.join("repo/.gemini/agents")),
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
            },
        );
        providers.insert(
            Provider::OpenCode,
            ProviderPaths {
                provider: Provider::OpenCode,
                user_skills: Some(base.join("opencode/skills")),
                repo_skills: Some(base.join("repo/.opencode/skills")),
                user_commands: Some(base.join("opencode/commands")),
                repo_commands: Some(base.join("repo/.opencode/commands")),
                user_agents: Some(base.join("opencode/agents")),
                repo_agents: Some(base.join("repo/.opencode/agents")),
                skill_also_reads_from: if opencode_reads_from_claude {
                    vec![base.join("claude/skills")]
                } else {
                    vec![]
                },
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
            },
        );

        ProviderSkillPaths::from_providers_for_test(providers, base.to_path_buf())
    }

    fn setup_skill(dir: &std::path::Path, name: &str, content: &str) {
        let skill = dir.join(name);
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(
            skill.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: test\n---\n{content}"),
        )
        .unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn end_to_end_link_single_skill() {
        let temp_dir = TempDir::new().unwrap();
        let paths = test_paths(temp_dir.path(), true);
        setup_skill(
            &paths
                .target_dir(
                    Provider::Claude,
                    LinkableResource::Skill,
                    ResourceScope::User,
                )
                .unwrap(),
            "my-tool",
            "# My Tool\n",
        );

        let report = link_skills_inner(&paths, ResourceScope::User, None, false).unwrap();

        assert!(!report.linked.is_empty());
        let entry = &report.linked[0];
        assert_eq!(entry.name, "my-tool");
        assert_eq!(entry.source_provider, "Claude");
        assert!(!entry.target_providers.contains(&"OpenCode".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn dry_run_does_not_create_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let paths = test_paths(temp_dir.path(), true);
        setup_skill(
            &paths
                .target_dir(
                    Provider::Claude,
                    LinkableResource::Skill,
                    ResourceScope::User,
                )
                .unwrap(),
            "dry-test",
            "# Content\n",
        );

        let report = link_skills_inner(&paths, ResourceScope::User, None, true).unwrap();

        assert!(!report.linked.is_empty());
        let codex_target = paths
            .target_dir(
                Provider::Codex,
                LinkableResource::Skill,
                ResourceScope::User,
            )
            .unwrap();
        let gemini_target = paths
            .target_dir(
                Provider::Gemini,
                LinkableResource::Skill,
                ResourceScope::User,
            )
            .unwrap();
        assert!(!codex_target.join("dry-test").exists());
        assert!(!gemini_target.join("dry-test").exists());
    }

    #[test]
    fn filter_restricts_to_named_skill() {
        let temp_dir = TempDir::new().unwrap();
        let paths = test_paths(temp_dir.path(), true);
        let source_dir = paths
            .target_dir(
                Provider::Claude,
                LinkableResource::Skill,
                ResourceScope::User,
            )
            .unwrap();
        setup_skill(&source_dir, "alpha", "# A\n");
        setup_skill(&source_dir, "beta", "# B\n");

        let report = link_skills_inner(&paths, ResourceScope::User, Some("alpha"), true).unwrap();

        assert_eq!(report.linked.len(), 1);
        assert_eq!(report.linked[0].name, "alpha");
    }

    #[cfg(unix)]
    #[test]
    fn category_level_symlink_is_reported_and_not_modified() {
        let temp_dir = TempDir::new().unwrap();
        let paths = test_paths(temp_dir.path(), false);

        let source_dir = paths
            .target_dir(
                Provider::Claude,
                LinkableResource::Skill,
                ResourceScope::User,
            )
            .unwrap();
        let opencode_dir = paths
            .target_dir(
                Provider::OpenCode,
                LinkableResource::Skill,
                ResourceScope::User,
            )
            .unwrap();
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::create_dir_all(opencode_dir.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&source_dir, &opencode_dir).unwrap();
        setup_skill(&source_dir, "category-test", "# Category Test\n");

        let report =
            link_skills_inner(&paths, ResourceScope::User, Some("category-test"), false).unwrap();

        assert!(
            report
                .skipped
                .iter()
                .any(|entry| entry.reason.contains("category-level symlink"))
        );
    }
}
