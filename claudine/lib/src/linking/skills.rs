use std::collections::BTreeMap;
use std::path::PathBuf;
use std::{collections::BTreeSet, fs};

use regex::Regex;

use crate::error::Result;
use crate::events::Provider;

use super::capabilities::{ALL_PROVIDERS, LinkableResource, capabilities_for};
use super::compatibility::parse_markdown_document;
use super::paths::{ProviderSkillPaths, ResourceScope};

/// Scope classification for a discovered skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillScope {
    /// Only in user-level skills directory (~/.claude/skills/).
    User,
    /// Present in both user and repo — repo wins (masks user).
    RepoMasked,
    /// Only in repo-level skills directory (.claude/skills/).
    Repo,
}

/// Information about a single discovered skill.
#[derive(Debug, Clone)]
pub struct SkillInfo {
    /// Skill directory name.
    pub name: String,
    /// Scope classification.
    pub scope: SkillScope,
    /// Description from SKILL.md frontmatter.
    pub description: Option<String>,
    /// Path to the SKILL.md file.
    pub skill_md_path: PathBuf,
}

/// Type of exception found during skill validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExceptionType {
    /// Skill directory missing for a provider.
    Missing,
    /// SKILL.md missing required `description` frontmatter.
    Invalid,
    /// SKILL.md contains a broken relative link.
    BrokenLink,
    /// SKILL.md body is long but contains no markdown links.
    NoLinks,
}

impl std::fmt::Display for ExceptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExceptionType::Missing => write!(f, "missing"),
            ExceptionType::Invalid => write!(f, "invalid"),
            ExceptionType::BrokenLink => write!(f, "broken-link"),
            ExceptionType::NoLinks => write!(f, "no-links"),
        }
    }
}

/// A single exception found during skill validation.
#[derive(Debug, Clone)]
pub struct SkillException {
    /// Provider where the exception was found.
    pub provider: Provider,
    /// Type of exception.
    pub exception_type: ExceptionType,
    /// Skill topic name.
    pub topic: String,
    /// Path to the SKILL.md file (or expected location).
    pub skill_md_path: PathBuf,
}

/// Directory-level diagnostic explaining why all skills for a scope are missing.
#[derive(Debug, Clone)]
pub struct SkillDirectoryDiagnostic {
    /// Provider where the issue was found.
    pub provider: Provider,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Complete report from skill discovery.
#[derive(Debug, Clone)]
pub struct SkillsReport {
    /// All discovered skills with scope classification.
    pub skills: Vec<SkillInfo>,
    /// Exceptions found during validation.
    pub exceptions: Vec<SkillException>,
    /// Directory-level diagnostics (missing directories).
    pub diagnostics: Vec<SkillDirectoryDiagnostic>,
}

/// Discover all skills from Claude's user and repo skill directories,
/// classify their scope, parse descriptions, and gather exceptions.
pub fn list_skills(
    paths: &ProviderSkillPaths,
    filters: &[String],
) -> Result<SkillsReport> {
    let user_dir = paths.target_dir(Provider::Claude, LinkableResource::Skill, ResourceScope::User);
    let repo_dir = paths.target_dir(Provider::Claude, LinkableResource::Skill, ResourceScope::Repo);

    let user_skills = scan_skill_dir(user_dir.as_ref());
    let repo_skills = scan_skill_dir(repo_dir.as_ref());

    let user_names: BTreeSet<&str> = user_skills.iter().map(|(name, _)| name.as_str()).collect();
    let repo_names: BTreeSet<&str> = repo_skills.iter().map(|(name, _)| name.as_str()).collect();

    let mut skills = Vec::new();

    // Repo-only skills
    for (name, path) in &repo_skills {
        if !user_names.contains(name.as_str()) {
            let skill_md = path.join("SKILL.md");
            let description = read_description(&skill_md);
            skills.push(SkillInfo {
                name: name.clone(),
                scope: SkillScope::Repo,
                description,
                skill_md_path: skill_md,
            });
        }
    }

    // User-only skills
    for (name, path) in &user_skills {
        if !repo_names.contains(name.as_str()) {
            let skill_md = path.join("SKILL.md");
            let description = read_description(&skill_md);
            skills.push(SkillInfo {
                name: name.clone(),
                scope: SkillScope::User,
                description,
                skill_md_path: skill_md,
            });
        }
    }

    // Skills in both (repo masks user)
    for (name, _user_path) in &user_skills {
        if repo_names.contains(name.as_str()) {
            let repo_path = repo_skills
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, p)| p)
                .unwrap();
            let skill_md = repo_path.join("SKILL.md");
            let description = read_description(&skill_md);
            skills.push(SkillInfo {
                name: name.clone(),
                scope: SkillScope::RepoMasked,
                description,
                skill_md_path: skill_md,
            });
        }
    }

    // Apply fuzzy filters
    if !filters.is_empty() {
        let lowered: Vec<String> = filters.iter().map(|f| f.to_lowercase()).collect();
        skills.retain(|skill| {
            let name_lower = skill.name.to_lowercase();
            lowered.iter().any(|filter| name_lower.contains(filter))
        });
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));

    let (exceptions, diagnostics) = gather_exceptions(paths, &user_skills, &repo_skills)?;

    Ok(SkillsReport {
        skills,
        exceptions,
        diagnostics,
    })
}

/// Scan a skill directory, returning `(name, path)` pairs for valid skill dirs.
fn scan_skill_dir(dir: Option<&PathBuf>) -> Vec<(String, PathBuf)> {
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
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) if name.starts_with('.') => continue,
            Some(name) => name.to_string(),
            None => continue,
        };
        if !path.join("SKILL.md").exists() {
            continue;
        }
        results.push((name, path));
    }
    results
}

/// Read and parse the `description` field from SKILL.md frontmatter.
fn read_description(skill_md: &PathBuf) -> Option<String> {
    let content = fs::read_to_string(skill_md).ok()?;
    let parsed = parse_markdown_document(&content).ok()?;
    let desc = parsed
        .frontmatter
        .get(serde_yaml::Value::String("description".to_string()))?;
    match desc {
        serde_yaml::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Gather exceptions across all providers that support skills.
fn gather_exceptions(
    paths: &ProviderSkillPaths,
    user_skills: &[(String, PathBuf)],
    repo_skills: &[(String, PathBuf)],
) -> Result<(Vec<SkillException>, Vec<SkillDirectoryDiagnostic>)> {
    let mut exceptions = Vec::new();
    let mut diagnostics = Vec::new();

    // Canonical skill names from Claude (union of user + repo)
    let canonical: BTreeMap<&str, &PathBuf> = {
        let mut map = BTreeMap::new();
        for (name, path) in user_skills {
            map.insert(name.as_str(), path);
        }
        // Repo overrides user if both exist
        for (name, path) in repo_skills {
            map.insert(name.as_str(), path);
        }
        map
    };

    // Check canonical skills for invalid (missing description) and broken links
    for (name, path) in &canonical {
        let skill_md = path.join("SKILL.md");
        check_invalid(&mut exceptions, name, &skill_md);
        check_broken_links(&mut exceptions, name, &skill_md);
        check_no_links(&mut exceptions, name, &skill_md);
    }

    // Check other providers for missing skills
    for provider in ALL_PROVIDERS {
        if provider == Provider::Claude {
            continue;
        }

        let caps = capabilities_for(provider);
        if !caps.skills.level.is_supported() {
            continue;
        }

        // Skip providers that also_reads_from Claude
        let also_reads = &caps.skills.also_reads_from;
        let reads_claude = also_reads
            .iter()
            .any(|p| p.to_string_lossy().contains(".claude/skills"));
        if reads_claude {
            continue;
        }

        // Check both user and repo scopes
        for scope in [ResourceScope::User, ResourceScope::Repo] {
            check_scope_missing(
                paths,
                provider,
                scope,
                &canonical,
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
            .then(a.topic.cmp(&b.topic))
    });

    Ok((exceptions, diagnostics))
}

/// Check a single provider scope for missing skills or missing directories.
fn check_scope_missing(
    paths: &ProviderSkillPaths,
    provider: Provider,
    scope: ResourceScope,
    canonical: &BTreeMap<&str, &PathBuf>,
    exceptions: &mut Vec<SkillException>,
    diagnostics: &mut Vec<SkillDirectoryDiagnostic>,
) {
    let scope_label = match scope {
        ResourceScope::User => "user",
        ResourceScope::Repo => "repo",
    };

    let Some(provider_dir) = paths.target_dir(provider, LinkableResource::Skill, scope) else {
        return;
    };

    if !provider_dir.exists() {
        // Directory doesn't exist — emit a directory-level diagnostic instead
        // of listing every individual skill as missing.
        let message = if provider_dir
            .parent()
            .is_some_and(|parent| parent.exists())
        {
            format!(
                "All <b>{scope_label}</b> scoped skills are NOT currently linked because the skills directory for <b>{provider}</b> does not exist! Use the <red>--fix</red> flag to fix this.",
            )
        } else {
            format!(
                "All <b>{scope_label}</b> scoped skills are NOT currently linked because the base configuration directory for <b>{provider}</b> does not exist! Use the <red>--fix</red> flag to fix this.",
            )
        };

        diagnostics.push(SkillDirectoryDiagnostic { provider, message });
        return;
    }

    // Directory exists — check individual skills
    for name in canonical.keys() {
        let expected = provider_dir.join(name);
        if !expected.exists() || !expected.join("SKILL.md").exists() {
            exceptions.push(SkillException {
                provider,
                exception_type: ExceptionType::Missing,
                topic: name.to_string(),
                skill_md_path: expected.join("SKILL.md"),
            });
        }
    }
}

/// Check if a SKILL.md is missing a `description` in frontmatter.
fn check_invalid(exceptions: &mut Vec<SkillException>, name: &str, skill_md: &PathBuf) {
    let content = match fs::read_to_string(skill_md) {
        Ok(c) => c,
        Err(_) => return,
    };
    let parsed = match parse_markdown_document(&content) {
        Ok(p) => p,
        Err(_) => return,
    };

    let has_description = parsed
        .frontmatter
        .get(serde_yaml::Value::String("description".to_string()))
        .map(|v| match v {
            serde_yaml::Value::String(s) => !s.trim().is_empty(),
            _ => false,
        })
        .unwrap_or(false);

    if !has_description {
        exceptions.push(SkillException {
            provider: Provider::Claude,
            exception_type: ExceptionType::Invalid,
            topic: name.to_string(),
            skill_md_path: skill_md.clone(),
        });
    }
}

/// Check for broken relative links in SKILL.md body.
fn check_broken_links(exceptions: &mut Vec<SkillException>, name: &str, skill_md: &PathBuf) {
    let content = match fs::read_to_string(skill_md) {
        Ok(c) => c,
        Err(_) => return,
    };
    let parsed = match parse_markdown_document(&content) {
        Ok(p) => p,
        Err(_) => return,
    };

    let parent = match skill_md.parent() {
        Some(p) => p,
        None => return,
    };

    let link_re = Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").unwrap();
    for cap in link_re.captures_iter(&parsed.body) {
        let target = &cap[2];
        if target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("mailto:")
        {
            continue;
        }
        // Strip fragment
        let path_part = target.split('#').next().unwrap_or(target);
        if path_part.is_empty() {
            continue;
        }
        let resolved = parent.join(path_part);
        if !resolved.exists() {
            exceptions.push(SkillException {
                provider: Provider::Claude,
                exception_type: ExceptionType::BrokenLink,
                topic: name.to_string(),
                skill_md_path: skill_md.clone(),
            });
        }
    }
}

/// Check if SKILL.md body is long but has no markdown links.
fn check_no_links(exceptions: &mut Vec<SkillException>, name: &str, skill_md: &PathBuf) {
    let content = match fs::read_to_string(skill_md) {
        Ok(c) => c,
        Err(_) => return,
    };
    let parsed = match parse_markdown_document(&content) {
        Ok(p) => p,
        Err(_) => return,
    };

    if parsed.body.len() <= 250 {
        return;
    }

    let link_re = Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").unwrap();
    if !link_re.is_match(&parsed.body) {
        exceptions.push(SkillException {
            provider: Provider::Claude,
            exception_type: ExceptionType::NoLinks,
            topic: name.to_string(),
            skill_md_path: skill_md.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempfile::TempDir;

    use super::*;
    use crate::linking::paths::ProviderPaths;

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

    fn test_paths(base: &std::path::Path) -> ProviderSkillPaths {
        let mut providers = HashMap::new();
        for provider in ALL_PROVIDERS {
            providers.insert(provider, empty_provider(provider));
        }

        providers.insert(
            Provider::Claude,
            ProviderPaths {
                provider: Provider::Claude,
                user_skills: Some(base.join("user/skills")),
                repo_skills: Some(base.join("repo/skills")),
                user_commands: None,
                repo_commands: None,
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
            },
        );

        ProviderSkillPaths::from_providers_for_test(providers, base.to_path_buf())
    }

    fn setup_skill(dir: &std::path::Path, name: &str, description: &str, body: &str) {
        let skill_dir = dir.join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\ndescription: {description}\n---\n{body}"),
        )
        .unwrap();
    }

    #[test]
    fn discovers_user_only_skill() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "my-skill", "A test skill", "# Body\n");

        let report = list_skills(&paths, &[]).unwrap();
        assert_eq!(report.skills.len(), 1);
        assert_eq!(report.skills[0].name, "my-skill");
        assert_eq!(report.skills[0].scope, SkillScope::User);
        assert_eq!(
            report.skills[0].description.as_deref(),
            Some("A test skill")
        );
    }

    #[test]
    fn discovers_repo_only_skill() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let repo_dir = tmp.path().join("repo/skills");
        setup_skill(&repo_dir, "repo-skill", "Repo only", "# Body\n");

        let report = list_skills(&paths, &[]).unwrap();
        assert_eq!(report.skills.len(), 1);
        assert_eq!(report.skills[0].scope, SkillScope::Repo);
    }

    #[test]
    fn classifies_masked_skill() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        let repo_dir = tmp.path().join("repo/skills");
        setup_skill(&user_dir, "shared", "User version", "# User\n");
        setup_skill(&repo_dir, "shared", "Repo version", "# Repo\n");

        let report = list_skills(&paths, &[]).unwrap();
        assert_eq!(report.skills.len(), 1);
        assert_eq!(report.skills[0].scope, SkillScope::RepoMasked);
        // Description should come from repo (the winning version)
        assert_eq!(
            report.skills[0].description.as_deref(),
            Some("Repo version")
        );
    }

    #[test]
    fn filters_skills_by_name() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "alpha", "Alpha skill", "# A\n");
        setup_skill(&user_dir, "beta", "Beta skill", "# B\n");
        setup_skill(&user_dir, "gamma", "Gamma skill", "# G\n");

        let report = list_skills(&paths, &["bet".to_string()]).unwrap();
        assert_eq!(report.skills.len(), 1);
        assert_eq!(report.skills[0].name, "beta");
    }

    #[test]
    fn filter_is_case_insensitive() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "MySkill", "My skill", "# Body\n");

        let report = list_skills(&paths, &["myskill".to_string()]).unwrap();
        assert_eq!(report.skills.len(), 1);
    }

    #[test]
    fn detects_invalid_missing_description() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        let skill_dir = user_dir.join("no-desc");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: no-desc\n---\n# Body\n").unwrap();

        let report = list_skills(&paths, &[]).unwrap();
        let invalid: Vec<_> = report
            .exceptions
            .iter()
            .filter(|e| e.exception_type == ExceptionType::Invalid)
            .collect();
        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0].topic, "no-desc");
    }

    #[test]
    fn detects_broken_links() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        setup_skill(
            &user_dir,
            "broken",
            "Has broken link",
            "See [details](./nonexistent.md) for more.\n",
        );

        let report = list_skills(&paths, &[]).unwrap();
        let broken: Vec<_> = report
            .exceptions
            .iter()
            .filter(|e| e.exception_type == ExceptionType::BrokenLink)
            .collect();
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].topic, "broken");
    }

    #[test]
    fn detects_no_links_in_long_body() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        let long_body = "x".repeat(300);
        setup_skill(&user_dir, "verbose", "Long body", &long_body);

        let report = list_skills(&paths, &[]).unwrap();
        let no_links: Vec<_> = report
            .exceptions
            .iter()
            .filter(|e| e.exception_type == ExceptionType::NoLinks)
            .collect();
        assert_eq!(no_links.len(), 1);
        assert_eq!(no_links[0].topic, "verbose");
    }

    #[test]
    fn no_exception_for_short_body_without_links() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "short", "Short skill", "Brief content.\n");

        let report = list_skills(&paths, &[]).unwrap();
        let no_links: Vec<_> = report
            .exceptions
            .iter()
            .filter(|e| e.exception_type == ExceptionType::NoLinks)
            .collect();
        assert!(no_links.is_empty());
    }

    #[test]
    fn skips_hidden_directories() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, ".hidden", "Hidden", "# Body\n");
        setup_skill(&user_dir, "visible", "Visible", "# Body\n");

        let report = list_skills(&paths, &[]).unwrap();
        assert_eq!(report.skills.len(), 1);
        assert_eq!(report.skills[0].name, "visible");
    }

    #[test]
    fn empty_directories_produce_empty_report() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());

        let report = list_skills(&paths, &[]).unwrap();
        assert!(report.skills.is_empty());
    }

    /// Build paths with a non-Claude provider (Gemini) for testing missing dir diagnostics.
    fn test_paths_with_gemini(base: &std::path::Path) -> ProviderSkillPaths {
        let mut providers = HashMap::new();
        for provider in ALL_PROVIDERS {
            providers.insert(provider, empty_provider(provider));
        }

        providers.insert(
            Provider::Claude,
            ProviderPaths {
                provider: Provider::Claude,
                user_skills: Some(base.join("user/skills")),
                repo_skills: Some(base.join("repo/skills")),
                user_commands: None,
                repo_commands: None,
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
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
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
            },
        );

        ProviderSkillPaths::from_providers_for_test(providers, base.to_path_buf())
    }

    #[test]
    fn diagnostic_when_skills_dir_missing_but_parent_exists() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths_with_gemini(tmp.path());

        // Create Claude skills
        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "my-skill", "A skill", "# Body\n");

        // Create Gemini parent but NOT the skills dir
        fs::create_dir_all(tmp.path().join("gemini")).unwrap();

        let report = list_skills(&paths, &[]).unwrap();
        let gemini_diags: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.provider == Provider::Gemini)
            .collect();
        assert!(!gemini_diags.is_empty());
        assert!(gemini_diags
            .iter()
            .any(|d| d.message.contains("skills directory")));
    }

    #[test]
    fn diagnostic_when_base_config_dir_missing() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths_with_gemini(tmp.path());

        // Create Claude skills
        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "my-skill", "A skill", "# Body\n");

        // Don't create the Gemini dir at all (no gemini/ parent)

        let report = list_skills(&paths, &[]).unwrap();
        let gemini_diags: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.provider == Provider::Gemini)
            .collect();
        assert!(!gemini_diags.is_empty());
        assert!(gemini_diags
            .iter()
            .any(|d| d.message.contains("base configuration directory")));
    }

    #[test]
    fn no_diagnostic_when_skills_dir_exists() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths_with_gemini(tmp.path());

        // Create Claude skills
        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "my-skill", "A skill", "# Body\n");

        // Create Gemini skills directories for both scopes (exist but empty)
        fs::create_dir_all(tmp.path().join("gemini/skills")).unwrap();
        fs::create_dir_all(tmp.path().join("repo/.gemini/skills")).unwrap();

        let report = list_skills(&paths, &[]).unwrap();
        // Should have individual missing exceptions, NOT diagnostics
        let gemini_diags: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.provider == Provider::Gemini)
            .collect();
        assert!(gemini_diags.is_empty());

        let gemini_missing: Vec<_> = report
            .exceptions
            .iter()
            .filter(|e| {
                e.provider == Provider::Gemini && e.exception_type == ExceptionType::Missing
            })
            .collect();
        // 1 missing in user scope + 1 missing in repo scope
        assert_eq!(gemini_missing.len(), 2);
        assert!(gemini_missing.iter().all(|e| e.topic == "my-skill"));
    }

    #[test]
    fn repo_scope_diagnostic_when_repo_dir_missing() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths_with_gemini(tmp.path());

        // Create Claude skills (repo scope)
        let repo_dir = tmp.path().join("repo/skills");
        setup_skill(&repo_dir, "repo-skill", "Repo skill", "# Body\n");

        // Don't create the Gemini repo dir at all

        let report = list_skills(&paths, &[]).unwrap();
        let gemini_diags: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.provider == Provider::Gemini)
            .collect();
        // Should have diagnostics for both user and repo scopes
        // Message contains Prose markup: "<b>repo</b> scoped"
        assert!(gemini_diags
            .iter()
            .any(|d| d.message.contains("repo</b> scoped")));
    }
}
