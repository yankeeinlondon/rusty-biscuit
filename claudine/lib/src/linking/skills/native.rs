use std::fs;
use std::path::{Path, PathBuf};

use super::SkillFixSummary;
use crate::error::Result;
use crate::linking::capabilities::{ALL_PROVIDERS, LinkableResource, capabilities_for};
use crate::linking::compatibility::fix_frontmatter_indentation_tabs;
use crate::linking::paths::{ProviderSkillPaths, ResourceScope};
use crate::linking::symlink::{LinkResult, create_skill_link};
use crate::provider::Provider;

/// Fix missing skill links for non-Claude providers.
///
/// For each provider that supports skills (and doesn't read from Claude's directory),
/// creates missing skill directories and symlinks canonical Claude skills into them.
/// Both user and repo scopes are handled.
pub fn fix_missing_skills(paths: &ProviderSkillPaths) -> Result<SkillFixSummary> {
    let mut summary = SkillFixSummary::default();

    let user_dir = paths.target_dir(
        Provider::Claude,
        LinkableResource::Skill,
        ResourceScope::User,
    );
    let repo_dir = paths.target_dir(
        Provider::Claude,
        LinkableResource::Skill,
        ResourceScope::Repo,
    );

    let all_user_skills = super::portable::scan_skill_dir(user_dir.as_ref());
    let all_repo_skills = super::portable::scan_skill_dir(repo_dir.as_ref());

    // Filter out skills with Claude-specific frontmatter — these are not shareable
    let (user_skills, user_skipped) = super::partial::filter_unshareable_skills(all_user_skills);
    let (repo_skills, repo_skipped) = super::partial::filter_unshareable_skills(all_repo_skills);
    summary.not_shareable = user_skipped + repo_skipped;

    // Fix missing `name` property in canonical Claude skills
    for (name, path) in user_skills.iter().chain(repo_skills.iter()) {
        let skill_md = path.join("SKILL.md");
        if fix_frontmatter_indentation_tabs(&skill_md)? {
            summary.yaml_tabs_fixed += 1;
        }
        if super::partial::fix_missing_name(name, &skill_md)? {
            summary.names_inserted += 1;
        }
    }

    for provider in ALL_PROVIDERS {
        if provider == Provider::Claude {
            continue;
        }

        let caps = capabilities_for(provider);
        if !caps.skills.level.is_supported() {
            continue;
        }

        for scope in [ResourceScope::User, ResourceScope::Repo] {
            // Skip providers that also read from Claude's directory for this scope
            let reads_claude = paths
                .also_reads_from(provider, LinkableResource::Skill, scope)
                .iter()
                .any(|p| p.to_string_lossy().contains(".claude/skills"));
            if reads_claude {
                continue;
            }
            let source_skills = match scope {
                ResourceScope::User => &user_skills,
                ResourceScope::Repo => &repo_skills,
            };

            if source_skills.is_empty() {
                continue;
            }

            let Some(provider_dir) = paths.target_dir(provider, LinkableResource::Skill, scope)
            else {
                continue;
            };

            // Create the provider's skills directory if it doesn't exist
            if !provider_dir.exists() {
                fs::create_dir_all(&provider_dir)?;
                summary.directories_created += 1;
            }

            fix_scope_skills(source_skills, &provider_dir, scope, &mut summary)?;
        }
    }

    Ok(summary)
}

/// Create symlinks for missing skills in a single (provider, scope) pair.
fn fix_scope_skills(
    source_skills: &[(String, PathBuf)],
    provider_dir: &Path,
    scope: ResourceScope,
    summary: &mut SkillFixSummary,
) -> Result<()> {
    for (_name, source_path) in source_skills {
        match create_skill_link(source_path, provider_dir, scope)? {
            LinkResult::Linked { .. } => summary.links_created += 1,
            LinkResult::AlreadyLinked => summary.already_linked += 1,
            LinkResult::Skipped { .. } => summary.skipped += 1,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::collections::HashMap;

    use tempfile::TempDir;

    #[cfg(unix)]
    use super::super::ExceptionType;
    use super::fix_missing_skills;
    #[cfg(unix)]
    use crate::linking::capabilities::ALL_PROVIDERS;
    #[cfg(unix)]
    use crate::linking::paths::ProviderPaths;
    use crate::linking::skills::test_helpers::test_paths_with_gemini;
    #[cfg(unix)]
    use crate::linking::skills::test_helpers::{empty_provider, setup_skill};
    #[cfg(unix)]
    use crate::provider::Provider;

    #[cfg(unix)]
    #[test]
    fn fix_creates_missing_directory_and_symlinks() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths_with_gemini(tmp.path());

        // Create Claude user skills
        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "alpha", "Alpha", "# A\n");
        setup_skill(&user_dir, "beta", "Beta", "# B\n");

        // Gemini user dir does not exist yet
        let gemini_dir = tmp.path().join("gemini/skills");
        assert!(!gemini_dir.exists());

        let summary = fix_missing_skills(&paths).unwrap();

        // Directory should have been created
        assert!(gemini_dir.exists());
        assert!(summary.directories_created > 0);

        // Symlinks should have been created
        assert_eq!(summary.links_created, 2);
        assert!(gemini_dir.join("alpha").exists());
        assert!(gemini_dir.join("beta").exists());

        // Verify they are symlinks
        assert!(
            gemini_dir
                .join("alpha")
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(
            gemini_dir
                .join("beta")
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[cfg(unix)]
    #[test]
    fn fix_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths_with_gemini(tmp.path());

        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "alpha", "Alpha", "# A\n");

        // First fix
        let first = fix_missing_skills(&paths).unwrap();
        assert_eq!(first.links_created, 1);

        // Second fix — should report already_linked
        let second = fix_missing_skills(&paths).unwrap();
        assert_eq!(second.links_created, 0);
        assert_eq!(second.already_linked, 1);
        assert_eq!(second.directories_created, 0);
    }

    #[cfg(unix)]
    #[test]
    fn fix_handles_repo_scope() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths_with_gemini(tmp.path());

        // Create Claude repo skills
        let repo_dir = tmp.path().join("repo/skills");
        setup_skill(&repo_dir, "repo-tool", "Repo tool", "# Body\n");

        let summary = fix_missing_skills(&paths).unwrap();

        // Should have created symlink in Gemini repo scope
        let gemini_repo_dir = tmp.path().join("repo/.gemini/skills");
        assert!(gemini_repo_dir.join("repo-tool").exists());
        assert!(summary.links_created >= 1);
    }

    #[cfg(unix)]
    #[test]
    fn fix_skips_providers_that_read_from_claude() {
        let tmp = TempDir::new().unwrap();

        // Create paths where Gemini reads from Claude
        let mut providers = HashMap::new();
        for provider in ALL_PROVIDERS {
            providers.insert(provider, empty_provider(provider));
        }
        providers.insert(
            Provider::Claude,
            ProviderPaths {
                provider: Provider::Claude,
                user_skills: Some(tmp.path().join("user/skills")),
                repo_skills: Some(tmp.path().join("repo/skills")),
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
                user_skills: Some(tmp.path().join("gemini/skills")),
                repo_skills: None,
                user_commands: None,
                repo_commands: None,
                user_agents: None,
                repo_agents: None,
                skill_also_reads_from: vec![tmp.path().join("user/.claude/skills")],
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
            },
        );
        let paths = crate::linking::paths::ProviderSkillPaths::from_providers_for_test(
            providers,
            tmp.path().to_path_buf(),
        );

        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "my-skill", "Skill", "# Body\n");

        let summary = fix_missing_skills(&paths).unwrap();

        // Gemini reads from Claude, so no links should be created for it
        let gemini_dir = tmp.path().join("gemini/skills");
        assert!(!gemini_dir.exists());
        assert_eq!(summary.links_created, 0);
    }

    #[test]
    fn fix_with_no_canonical_skills_is_noop() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths_with_gemini(tmp.path());

        // No Claude skills exist
        let summary = fix_missing_skills(&paths).unwrap();
        assert_eq!(summary.directories_created, 0);
        assert_eq!(summary.links_created, 0);
        assert_eq!(summary.already_linked, 0);
        assert_eq!(summary.skipped, 0);
    }

    #[cfg(unix)]
    #[test]
    fn fix_clears_diagnostics_and_missing_exceptions() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths_with_gemini(tmp.path());

        // Create skills in both scopes so both Gemini scopes are fixable
        let user_dir = tmp.path().join("user/skills");
        let repo_dir = tmp.path().join("repo/skills");
        setup_skill(&user_dir, "my-skill", "Skill", "# Body\n");
        setup_skill(&repo_dir, "my-skill", "Skill", "# Body\n");

        // Before fix: should have diagnostics
        let before = crate::linking::skills::list_skills(&paths, &[]).unwrap();
        assert!(!before.diagnostics.is_empty());

        // Apply fix
        fix_missing_skills(&paths).unwrap();

        // After fix: diagnostics and missing exceptions should be resolved
        let after = crate::linking::skills::list_skills(&paths, &[]).unwrap();
        let gemini_diags: Vec<_> = after
            .diagnostics
            .iter()
            .filter(|d| d.provider == Provider::Gemini)
            .collect();
        assert!(gemini_diags.is_empty());

        let gemini_missing: Vec<_> = after
            .exceptions
            .iter()
            .filter(|e| {
                e.provider == Provider::Gemini && e.exception_type == ExceptionType::Missing
            })
            .collect();
        assert!(gemini_missing.is_empty());
    }
}
