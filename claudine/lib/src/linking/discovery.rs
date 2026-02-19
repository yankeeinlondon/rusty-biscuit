use std::fs;
use std::path::PathBuf;

use crate::error::Result;
use crate::events::Provider;

use super::paths::{ProviderPaths, ProviderSkillPaths, ResourceScope};

/// A resource discovered during the scan phase.
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    /// Resource name (skill directory name or command filename stem).
    pub name: String,
    /// Path to the resource (directory for skills, file for commands).
    pub path: PathBuf,
    /// Which provider owns this resource location.
    pub provider: Provider,
    /// Whether this entry is already a symlink.
    pub is_symlink: bool,
    /// Content hash (populated during hashing phase).
    pub hash: Option<u64>,
}

/// Discover all Markdown skills across providers for the given scope.
pub fn discover_skills(
    paths: &ProviderSkillPaths,
    scope: ResourceScope,
) -> Result<Vec<DiscoveredSkill>> {
    let mut skills = Vec::new();

    for (provider, dir) in paths.for_scope(scope) {
        if !dir.exists() {
            continue;
        }
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = match path.file_name().and_then(|name| name.to_str()) {
                Some(name) if name.starts_with('.') => continue,
                Some(name) => name.to_string(),
                None => continue,
            };

            if !path.join("SKILL.md").exists() {
                continue;
            }

            let is_symlink = fs::symlink_metadata(&path)
                .map(|meta| meta.file_type().is_symlink())
                .unwrap_or(false);

            skills.push(DiscoveredSkill {
                name,
                path,
                provider,
                is_symlink,
                hash: None,
            });
        }
    }

    skills.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.provider.as_slug().cmp(right.provider.as_slug()))
    });
    Ok(skills)
}

/// Discover Markdown slash commands across providers for the given scope.
pub fn discover_commands(
    paths: &ProviderSkillPaths,
    scope: ResourceScope,
) -> Result<Vec<DiscoveredSkill>> {
    let mut commands = Vec::new();

    for (provider, dir) in paths.commands_for_scope(scope) {
        if !dir.exists() {
            continue;
        }
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let file_name = match path.file_name().and_then(|name| name.to_str()) {
                Some(name) if name.ends_with(".md") && !name.starts_with('.') => name.to_string(),
                _ => continue,
            };

            let is_symlink = fs::symlink_metadata(&path)
                .map(|meta| meta.file_type().is_symlink())
                .unwrap_or(false);

            let name = file_name
                .strip_suffix(".md")
                .unwrap_or(&file_name)
                .to_string();

            commands.push(DiscoveredSkill {
                name,
                path,
                provider,
                is_symlink,
                hash: None,
            });
        }
    }

    commands.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.provider.as_slug().cmp(right.provider.as_slug()))
    });
    Ok(commands)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;

    use tempfile::TempDir;

    use super::*;

    fn empty_provider(provider: Provider) -> ProviderPaths {
        ProviderPaths {
            provider,
            user_skills: None,
            repo_skills: None,
            user_commands: None,
            repo_commands: None,
            skill_also_reads_from: vec![],
            command_also_reads_from: vec![],
        }
    }

    fn test_paths(claude_user_skills: PathBuf) -> ProviderSkillPaths {
        let mut providers = HashMap::new();
        for provider in super::super::capabilities::ALL_PROVIDERS {
            providers.insert(provider, empty_provider(provider));
        }

        providers.insert(
            Provider::Claude,
            ProviderPaths {
                provider: Provider::Claude,
                user_skills: Some(claude_user_skills),
                repo_skills: None,
                user_commands: None,
                repo_commands: None,
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
            },
        );

        ProviderSkillPaths::from_providers_for_test(providers, PathBuf::from("/tmp"))
    }

    fn setup_skill(base: &Path, provider_dir: &str, skill_name: &str) -> PathBuf {
        let skill_dir = base.join(provider_dir).join(skill_name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: test-skill\ndescription: A test skill\n---\n\n# Test\n",
        )
        .unwrap();
        skill_dir
    }

    #[test]
    fn discovers_skills_in_temp_dir() {
        let temp_dir = TempDir::new().unwrap();
        setup_skill(temp_dir.path(), "claude_skills", "my-skill");

        let paths = test_paths(temp_dir.path().join("claude_skills"));
        let skills = discover_skills(&paths, ResourceScope::User).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "my-skill");
        assert_eq!(skills[0].provider, Provider::Claude);
        assert!(!skills[0].is_symlink);
    }

    #[test]
    fn skips_hidden_directories() {
        let temp_dir = TempDir::new().unwrap();
        setup_skill(temp_dir.path(), "claude_skills", ".system");
        setup_skill(temp_dir.path(), "claude_skills", "visible-skill");

        let paths = test_paths(temp_dir.path().join("claude_skills"));
        let skills = discover_skills(&paths, ResourceScope::User).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "visible-skill");
    }

    #[test]
    fn skips_directories_without_skill_md() {
        let temp_dir = TempDir::new().unwrap();
        let no_skill = temp_dir.path().join("claude_skills/no-skill-md");
        fs::create_dir_all(&no_skill).unwrap();
        fs::write(no_skill.join("README.md"), "# Not a skill").unwrap();

        let paths = test_paths(temp_dir.path().join("claude_skills"));
        let skills = discover_skills(&paths, ResourceScope::User).unwrap();

        assert!(skills.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn detects_symlinked_skill() {
        let temp_dir = TempDir::new().unwrap();
        let claude_skills = temp_dir.path().join("claude_skills");
        let source = setup_skill(temp_dir.path(), "real_skills", "my-skill");
        fs::create_dir_all(&claude_skills).unwrap();
        std::os::unix::fs::symlink(&source, claude_skills.join("my-skill")).unwrap();

        let paths = test_paths(claude_skills);
        let skills = discover_skills(&paths, ResourceScope::User).unwrap();

        assert_eq!(skills.len(), 1);
        assert!(skills[0].is_symlink);
    }
}
