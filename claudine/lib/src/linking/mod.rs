mod conflict;
mod discovery;
mod hashing;
mod paths;
mod report;
mod symlink;

pub use conflict::SkillSyncStatus;
pub use discovery::DiscoveredSkill;
pub use paths::{LinkScope, ProviderPaths, ProviderSkillPaths};
pub use report::{ConflictEntry, InSyncEntry, LinkReport, LinkedEntry, SkippedEntry};
pub use symlink::{LinkResult, relative_path};

use conflict::build_also_reads_from;

use crate::error::Result;

/// Orchestrate skill linking across providers.
///
/// Runs the 4-phase linking algorithm:
/// 1. Discovery -- find skills across all providers
/// 2. Hashing -- content-hash each skill directory
/// 3. Analysis -- detect conflicts, candidates, and in-sync state
/// 4. Linking -- create symlinks for candidates
pub fn link_skills(
    scope: LinkScope,
    filter: Option<&str>,
    dry_run: bool,
) -> Result<LinkReport> {
    link_skills_inner(&ProviderSkillPaths::new(), scope, filter, dry_run)
}

/// Core linking logic, injectable for testing.
fn link_skills_inner(
    provider_paths: &ProviderSkillPaths,
    scope: LinkScope,
    filter: Option<&str>,
    dry_run: bool,
) -> Result<LinkReport> {
    // Phase 1: Discovery
    let mut skills = discovery::discover_skills(provider_paths, scope)?;
    if let Some(f) = filter {
        skills.retain(|s| s.name == f);
    }

    // Phase 2: Hashing
    for skill in &mut skills {
        skill.hash = hashing::hash_skill_dir(&skill.path).ok();
    }

    // Phase 3: Analysis
    let provider_names: Vec<&str> =
        provider_paths.for_scope(scope).iter().map(|(n, _)| *n).collect();
    let also_reads = build_also_reads_from(provider_paths, &provider_names);
    let statuses = conflict::analyze_skills(skills, &provider_names, &also_reads);

    // Phase 4: Linking
    let mut report = LinkReport::default();
    apply_statuses(statuses, provider_paths, scope, dry_run, &mut report, false)?;

    // Command linking
    link_commands(provider_paths, scope, filter, dry_run, &mut report)?;

    Ok(report)
}

/// Apply sync statuses to the report, creating symlinks as needed.
fn apply_statuses(
    statuses: Vec<SkillSyncStatus>,
    provider_paths: &ProviderSkillPaths,
    scope: LinkScope,
    dry_run: bool,
    report: &mut LinkReport,
    is_command: bool,
) -> Result<()> {
    let prefix = if is_command { "cmd:" } else { "" };

    for status in statuses {
        match status {
            SkillSyncStatus::LinkCandidate { source, target_providers } => {
                let mut linked = Vec::new();
                for target in &target_providers {
                    let dest_dir = if is_command {
                        match resolve_provider_command_dir(provider_paths, target, scope) {
                            Some(d) => d,
                            None => continue,
                        }
                    } else {
                        resolve_provider_skill_dir(provider_paths, target, scope)
                    };

                    if dry_run {
                        linked.push(target.clone());
                        continue;
                    }

                    match symlink::create_skill_link(&source.path, &dest_dir, scope)? {
                        LinkResult::Linked { .. } => linked.push(target.clone()),
                        LinkResult::AlreadyLinked => {}
                        LinkResult::Skipped { reason } => {
                            report.skipped.push(SkippedEntry {
                                name: format!("{prefix}{}", source.name),
                                reason: format!("{target}: {reason}"),
                            });
                        }
                    }
                }
                if !linked.is_empty() {
                    report.linked.push(LinkedEntry {
                        name: format!("{prefix}{}", source.name),
                        source_provider: source.provider.clone(),
                        target_providers: linked,
                    });
                }
            }
            SkillSyncStatus::InSync { name, providers } => {
                report.in_sync.push(InSyncEntry {
                    name: format!("{prefix}{name}"),
                    providers: providers.iter().map(|(p, _, _)| p.clone()).collect(),
                });
            }
            SkillSyncStatus::Conflict { name, versions } => {
                report.conflicts.push(ConflictEntry {
                    name: format!("{prefix}{name}"),
                    versions,
                });
            }
            SkillSyncStatus::AlreadyLinked { name, .. } => {
                report.already_linked.push(format!("{prefix}{name}"));
            }
        }
    }
    Ok(())
}

/// Link commands across providers that support Markdown commands.
fn link_commands(
    provider_paths: &ProviderSkillPaths,
    scope: LinkScope,
    filter: Option<&str>,
    dry_run: bool,
    report: &mut LinkReport,
) -> Result<()> {
    let mut commands = discovery::discover_commands(provider_paths, scope)?;
    if let Some(f) = filter {
        commands.retain(|c| c.name == f);
    }

    for cmd in &mut commands {
        if let Ok(content) = std::fs::read(&cmd.path) {
            cmd.hash = Some(biscuit_hash::xx_hash_bytes(&content));
        }
    }

    let names: Vec<&str> =
        provider_paths.commands_for_scope(scope).iter().map(|(n, _)| *n).collect();
    let also_reads = build_also_reads_from(provider_paths, &names);
    let statuses = conflict::analyze_skills(commands, &names, &also_reads);

    apply_statuses(statuses, provider_paths, scope, dry_run, report, true)
}

/// Resolve the skill directory path for a provider and scope.
fn resolve_provider_skill_dir(
    paths: &ProviderSkillPaths,
    provider: &str,
    scope: LinkScope,
) -> std::path::PathBuf {
    let p = match provider {
        "claude" => &paths.claude,
        "gemini" => &paths.gemini,
        "codex" => &paths.codex,
        "opencode" => &paths.opencode,
        _ => &paths.claude,
    };
    match scope {
        LinkScope::User => p.user_skills.clone(),
        LinkScope::Repo => p.repo_skills.clone(),
    }
}

/// Resolve the command directory path for a provider and scope.
fn resolve_provider_command_dir(
    paths: &ProviderSkillPaths,
    provider: &str,
    scope: LinkScope,
) -> Option<std::path::PathBuf> {
    let p = match provider {
        "claude" => &paths.claude,
        "opencode" => &paths.opencode,
        _ => return None,
    };
    match scope {
        LinkScope::User => p.user_commands.clone(),
        LinkScope::Repo => p.repo_commands.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn test_paths(base: &std::path::Path) -> ProviderSkillPaths {
        ProviderSkillPaths {
            claude: ProviderPaths {
                user_skills: base.join("claude/skills"),
                repo_skills: base.join("repo/.claude/skills"),
                user_commands: Some(base.join("claude/commands")),
                repo_commands: Some(base.join("repo/.claude/commands")),
                also_reads_from: vec![],
            },
            gemini: ProviderPaths {
                user_skills: base.join("gemini/skills"),
                repo_skills: base.join("repo/.gemini/skills"),
                user_commands: None,
                repo_commands: None,
                also_reads_from: vec![],
            },
            codex: ProviderPaths {
                user_skills: base.join("codex/skills"),
                repo_skills: base.join("repo/.codex/skills"),
                user_commands: None,
                repo_commands: None,
                also_reads_from: vec![],
            },
            opencode: ProviderPaths {
                user_skills: base.join("opencode/skills"),
                repo_skills: base.join("repo/.opencode/skills"),
                user_commands: Some(base.join("opencode/commands")),
                repo_commands: Some(base.join("repo/.opencode/commands")),
                also_reads_from: vec![base.join("claude/skills")],
            },
        }
    }

    fn setup_skill(dir: &std::path::Path, name: &str, content: &str) {
        let skill = dir.join(name);
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: test\n---\n{content}"),
        )
        .unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn end_to_end_link_single_skill() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        setup_skill(&paths.claude.user_skills, "my-tool", "# My Tool\n");

        let report = link_skills_inner(&paths, LinkScope::User, None, false).unwrap();

        assert!(!report.linked.is_empty());
        let entry = &report.linked[0];
        assert_eq!(entry.name, "my-tool");
        assert_eq!(entry.source_provider, "claude");
        assert!(!entry.target_providers.contains(&"opencode".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn dry_run_does_not_create_symlinks() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        setup_skill(&paths.claude.user_skills, "dry-test", "# Content\n");

        let report = link_skills_inner(&paths, LinkScope::User, None, true).unwrap();

        assert!(!report.linked.is_empty());
        assert!(!paths.codex.user_skills.join("dry-test").exists());
        assert!(!paths.gemini.user_skills.join("dry-test").exists());
    }

    #[test]
    fn filter_restricts_to_named_skill() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        setup_skill(&paths.claude.user_skills, "alpha", "# A\n");
        setup_skill(&paths.claude.user_skills, "beta", "# B\n");

        let report =
            link_skills_inner(&paths, LinkScope::User, Some("alpha"), true).unwrap();

        assert_eq!(report.linked.len(), 1);
        assert_eq!(report.linked[0].name, "alpha");
    }
}
