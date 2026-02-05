use std::fs;
use std::path::PathBuf;

use crate::error::Result;

use super::paths::{LinkScope, ProviderSkillPaths};

/// A skill discovered during the scan phase.
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    /// Skill name (directory name).
    pub name: String,
    /// Absolute path to the skill directory.
    pub path: PathBuf,
    /// Which provider owns this skill directory.
    pub provider: String,
    /// Whether this is already a symlink.
    pub is_symlink: bool,
    /// Content hash of the skill directory (populated during hashing phase).
    pub hash: Option<u64>,
}

/// Discover all skills across providers for the given scope.
///
/// Walks each provider's skill directory. Each subdirectory containing
/// a `SKILL.md` file is treated as a skill. Hidden directories (starting
/// with `.`) are skipped.
pub fn discover_skills(
    paths: &ProviderSkillPaths,
    scope: LinkScope,
) -> Result<Vec<DiscoveredSkill>> {
    let mut skills = Vec::new();

    for (provider, dir) in paths.for_scope(scope) {
        if !dir.exists() {
            continue;
        }
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let dir_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) if name.starts_with('.') => continue,
                Some(name) => name.to_string(),
                None => continue,
            };
            if !path.join("SKILL.md").exists() {
                continue;
            }
            let is_symlink = fs::symlink_metadata(&path)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);
            skills.push(DiscoveredSkill {
                name: dir_name,
                path,
                provider: provider.to_string(),
                is_symlink,
                hash: None,
            });
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name).then(a.provider.cmp(&b.provider)));
    Ok(skills)
}

/// Discover commands across providers for the given scope.
///
/// Returns a list of `DiscoveredSkill` where each represents a command
/// file (`.md`). Only providers with Markdown command support are scanned.
pub fn discover_commands(
    paths: &ProviderSkillPaths,
    scope: LinkScope,
) -> Result<Vec<DiscoveredSkill>> {
    let mut commands = Vec::new();

    for (provider, dir) in paths.commands_for_scope(scope) {
        if !dir.exists() {
            continue;
        }
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) if n.ends_with(".md") && !n.starts_with('.') => n.to_string(),
                _ => continue,
            };
            let is_symlink = fs::symlink_metadata(&path)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);
            let name = file_name.strip_suffix(".md").unwrap_or(&file_name).to_string();
            commands.push(DiscoveredSkill {
                name,
                path,
                provider: provider.to_string(),
                is_symlink,
                hash: None,
            });
        }
    }

    commands.sort_by(|a, b| a.name.cmp(&b.name).then(a.provider.cmp(&b.provider)));
    Ok(commands)
}

#[cfg(test)]
mod tests {
    use super::super::paths::ProviderPaths;
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn empty_paths() -> ProviderPaths {
        ProviderPaths {
            user_skills: PathBuf::from("/nonexistent"),
            repo_skills: PathBuf::from("/nonexistent"),
            user_commands: None,
            repo_commands: None,
            also_reads_from: vec![],
        }
    }

    /// Build paths where only Claude's user_skills points to `dir`.
    fn claude_only_paths(dir: PathBuf) -> ProviderSkillPaths {
        ProviderSkillPaths {
            claude: ProviderPaths { user_skills: dir, ..empty_paths() },
            gemini: empty_paths(),
            codex: empty_paths(),
            opencode: empty_paths(),
        }
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
        let tmp = TempDir::new().unwrap();
        setup_skill(tmp.path(), "claude_skills", "my-skill");

        let paths = claude_only_paths(tmp.path().join("claude_skills"));
        let skills = discover_skills(&paths, LinkScope::User).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "my-skill");
        assert_eq!(skills[0].provider, "claude");
        assert!(!skills[0].is_symlink);
    }

    #[test]
    fn skips_hidden_directories() {
        let tmp = TempDir::new().unwrap();
        setup_skill(tmp.path(), "claude_skills", ".system");
        setup_skill(tmp.path(), "claude_skills", "visible-skill");

        let paths = claude_only_paths(tmp.path().join("claude_skills"));
        let skills = discover_skills(&paths, LinkScope::User).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "visible-skill");
    }

    #[test]
    fn skips_directories_without_skill_md() {
        let tmp = TempDir::new().unwrap();
        let no_skill = tmp.path().join("claude_skills/no-skill-md");
        fs::create_dir_all(&no_skill).unwrap();
        fs::write(no_skill.join("README.md"), "# Not a skill").unwrap();

        let paths = claude_only_paths(tmp.path().join("claude_skills"));
        let skills = discover_skills(&paths, LinkScope::User).unwrap();

        assert!(skills.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn detects_symlinked_skill() {
        let tmp = TempDir::new().unwrap();
        let claude_skills = tmp.path().join("claude_skills");
        let source = setup_skill(tmp.path(), "real_skills", "my-skill");
        fs::create_dir_all(&claude_skills).unwrap();
        std::os::unix::fs::symlink(&source, claude_skills.join("my-skill")).unwrap();

        let paths = claude_only_paths(claude_skills);
        let skills = discover_skills(&paths, LinkScope::User).unwrap();

        assert_eq!(skills.len(), 1);
        assert!(skills[0].is_symlink);
    }

}
