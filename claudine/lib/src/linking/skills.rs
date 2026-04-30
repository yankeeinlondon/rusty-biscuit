use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{collections::BTreeSet, fs};

use biscuit_file::serde_yaml_ng;
use regex::Regex;

use super::capabilities::{ALL_PROVIDERS, LinkableResource, capabilities_for};
use super::compatibility::{
    fix_frontmatter_indentation_tabs, frontmatter_has_indentation_tabs,
    has_claude_specific_properties, parse_markdown_document,
};
use super::filter::ResourceFilter;
use super::paths::{ProviderSkillPaths, ResourceScope};
use super::symlink::{LinkResult, create_skill_link};
use crate::error::Result;
use crate::provider::Provider;

/// Backward-compatible alias for `ResourceFilter`.
pub type SkillFilter = ResourceFilter;

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
    /// SKILL.md frontmatter uses tab indentation that strict YAML parsers reject.
    YamlTabs,
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
            ExceptionType::YamlTabs => write!(f, "yaml-tabs"),
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
    /// For `Invalid`: names of missing required frontmatter properties.
    pub missing_properties: Vec<String>,
    /// For `BrokenLink`: the markdown link text (`[text]`).
    pub link_text: Option<String>,
    /// For `BrokenLink`: the link target path (`(target)`).
    pub link_target: Option<String>,
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

/// Summary of fix operations applied by [`fix_missing_skills`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SkillFixSummary {
    /// Directories created for providers that had no skills directory.
    pub directories_created: usize,
    /// Symlinks created for individual missing skills.
    pub links_created: usize,
    /// Skills that were already correctly linked.
    pub already_linked: usize,
    /// Operations skipped (real directory exists, symlink points elsewhere, etc.).
    pub skipped: usize,
    /// SKILL.md files that had a missing `name` property auto-inserted.
    pub names_inserted: usize,
    /// SKILL.md files whose YAML indentation tabs were normalized to spaces.
    pub yaml_tabs_fixed: usize,
    /// Skills skipped because they have Claude-specific frontmatter (not shareable).
    pub not_shareable: usize,
}

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

    let all_user_skills = scan_skill_dir(user_dir.as_ref());
    let all_repo_skills = scan_skill_dir(repo_dir.as_ref());

    // Filter out skills with Claude-specific frontmatter — these are not shareable
    let (user_skills, user_skipped) = filter_unshareable_skills(all_user_skills);
    let (repo_skills, repo_skipped) = filter_unshareable_skills(all_repo_skills);
    summary.not_shareable = user_skipped + repo_skipped;

    // Fix missing `name` property in canonical Claude skills
    for (name, path) in user_skills.iter().chain(repo_skills.iter()) {
        let skill_md = path.join("SKILL.md");
        if fix_frontmatter_indentation_tabs(&skill_md)? {
            summary.yaml_tabs_fixed += 1;
        }
        if fix_missing_name(name, &skill_md)? {
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

/// Partition skills into shareable (no Claude-specific properties) and count of skipped.
///
/// Skills are directories; the check targets `SKILL.md` inside each directory.
fn filter_unshareable_skills(skills: Vec<(String, PathBuf)>) -> (Vec<(String, PathBuf)>, usize) {
    let mut shareable = Vec::new();
    let mut skipped = 0;
    for entry in skills {
        let skill_md = entry.1.join("SKILL.md");
        if skill_md.exists() && has_claude_specific_properties(&skill_md) {
            skipped += 1;
        } else {
            shareable.push(entry);
        }
    }
    (shareable, skipped)
}

/// Discover all skills from Claude's user and repo skill directories,
/// classify their scope, parse descriptions, and gather exceptions.
pub fn list_skills(paths: &ProviderSkillPaths, filters: &[String]) -> Result<SkillsReport> {
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

    // Apply filters (fuzzy, exact, negation)
    let parsed_filters = SkillFilter::parse_all(filters);
    if !parsed_filters.is_empty() {
        skills.retain(|skill| SkillFilter::retain(&parsed_filters, &skill.name));
    }

    skills.sort_by(|a, b| a.scope.cmp(&b.scope).then(a.name.cmp(&b.name)));

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
        .get(serde_yaml_ng::Value::String("description".to_string()))?;
    match desc {
        serde_yaml_ng::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
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
        check_yaml_tabs(&mut exceptions, name, &skill_md);
        check_invalid(&mut exceptions, name, &skill_md);
        check_broken_links(&mut exceptions, name, &skill_md);
        check_no_links(&mut exceptions, name, &skill_md);
    }

    // Scope-specific skill sets for missing-link checks.
    // Each scope should only expect skills that exist in Claude's corresponding scope.
    let user_canonical: BTreeMap<&str, &PathBuf> = user_skills
        .iter()
        .map(|(name, path)| (name.as_str(), path))
        .collect();
    let repo_canonical: BTreeMap<&str, &PathBuf> = repo_skills
        .iter()
        .map(|(name, path)| (name.as_str(), path))
        .collect();

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

        // Check each scope against its own source skill set
        for scope in [ResourceScope::User, ResourceScope::Repo] {
            let scope_skills = match scope {
                ResourceScope::User => &user_canonical,
                ResourceScope::Repo => &repo_canonical,
            };
            check_scope_missing(
                paths,
                provider,
                scope,
                scope_skills,
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
        let message = if provider_dir.parent().is_some_and(|parent| parent.exists()) {
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
                missing_properties: Vec::new(),
                link_text: None,
                link_target: None,
            });
        }
    }
}

fn check_yaml_tabs(exceptions: &mut Vec<SkillException>, name: &str, skill_md: &PathBuf) {
    let content = match fs::read_to_string(skill_md) {
        Ok(c) => c,
        Err(_) => return,
    };

    let has_tabs = match frontmatter_has_indentation_tabs(&content) {
        Ok(has_tabs) => has_tabs,
        Err(_) => return,
    };

    if has_tabs {
        exceptions.push(SkillException {
            provider: Provider::Claude,
            exception_type: ExceptionType::YamlTabs,
            topic: name.to_string(),
            skill_md_path: skill_md.clone(),
            missing_properties: Vec::new(),
            link_text: None,
            link_target: None,
        });
    }
}

/// Check if a SKILL.md is missing required frontmatter properties.
fn check_invalid(exceptions: &mut Vec<SkillException>, name: &str, skill_md: &PathBuf) {
    const REQUIRED_PROPERTIES: &[&str] = &["name", "description"];

    let content = match fs::read_to_string(skill_md) {
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
        exceptions.push(SkillException {
            provider: Provider::Claude,
            exception_type: ExceptionType::Invalid,
            topic: name.to_string(),
            skill_md_path: skill_md.clone(),
            missing_properties: missing,
            link_text: None,
            link_target: None,
        });
    }
}

/// Fix a SKILL.md that is missing the `name` frontmatter property.
///
/// - If frontmatter exists but has no `name`, inserts it after the opening `---`.
/// - If no frontmatter exists, prepends a `---\nname: {topic}\n---\n` block.
///
/// Returns `true` if a fix was applied.
fn fix_missing_name(topic: &str, skill_md: &PathBuf) -> Result<bool> {
    let content = match fs::read_to_string(skill_md) {
        Ok(c) => c,
        Err(_) => return Ok(false),
    };
    let parsed = match parse_markdown_document(&content) {
        Ok(p) => p,
        Err(_) => return Ok(false),
    };

    let has_name = parsed
        .frontmatter
        .get(serde_yaml_ng::Value::String("name".to_string()))
        .map(|v| match v {
            serde_yaml_ng::Value::String(s) => !s.trim().is_empty(),
            _ => false,
        })
        .unwrap_or(false);

    if has_name {
        return Ok(false);
    }

    let new_content = if parsed.had_frontmatter {
        // Insert `name: {topic}` right after the opening `---`
        if let Some(rest) = content.strip_prefix("---\r\n") {
            format!("---\r\nname: {topic}\r\n{rest}")
        } else if let Some(rest) = content.strip_prefix("---\n") {
            format!("---\nname: {topic}\n{rest}")
        } else {
            return Ok(false);
        }
    } else {
        // No frontmatter — prepend a new block
        format!("---\nname: {topic}\n---\n\n{content}")
    };

    fs::write(skill_md, new_content)?;
    Ok(true)
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
        let link_text = cap[1].to_string();
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
                missing_properties: Vec::new(),
                link_text: Some(link_text),
                link_target: Some(target.to_string()),
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
            missing_properties: Vec::new(),
            link_text: None,
            link_target: None,
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
            user_agents: None,
            repo_agents: None,
            skill_also_reads_from: vec![],
            command_also_reads_from: vec![],
            agent_also_reads_from: vec![],
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
                user_agents: None,
                repo_agents: None,
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
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
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: no-desc\n---\n# Body\n",
        )
        .unwrap();

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
    fn detects_and_fixes_yaml_tabs() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        let skill_dir = user_dir.join("tabbed");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: tabbed\ndescription: Has tabbed yaml\nprompt: |-\n\tline one\n\t\tline two\n---\n# Body\n",
        )
        .unwrap();

        let report = list_skills(&paths, &[]).unwrap();
        let yaml_tabs: Vec<_> = report
            .exceptions
            .iter()
            .filter(|e| e.exception_type == ExceptionType::YamlTabs)
            .collect();
        assert_eq!(yaml_tabs.len(), 1);
        assert_eq!(yaml_tabs[0].topic, "tabbed");

        let summary = fix_missing_skills(&paths).unwrap();
        assert_eq!(summary.yaml_tabs_fixed, 1);

        let content = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        assert!(!frontmatter_has_indentation_tabs(&content).unwrap());
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
    fn sorts_by_scope_then_alpha() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        let repo_dir = tmp.path().join("repo/skills");
        setup_skill(&user_dir, "gamma", "Gamma skill", "# G\n");
        setup_skill(&user_dir, "alpha", "Alpha skill", "# A\n");
        setup_skill(&repo_dir, "beta", "Beta skill", "# B\n");

        let report = list_skills(&paths, &[]).unwrap();
        assert_eq!(report.skills.len(), 3);
        // User scope first (alpha, gamma), then Repo scope (beta)
        assert_eq!(report.skills[0].name, "alpha");
        assert_eq!(report.skills[0].scope, SkillScope::User);
        assert_eq!(report.skills[1].name, "gamma");
        assert_eq!(report.skills[1].scope, SkillScope::User);
        assert_eq!(report.skills[2].name, "beta");
        assert_eq!(report.skills[2].scope, SkillScope::Repo);
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
                user_skills: Some(base.join("gemini/skills")),
                repo_skills: Some(base.join("repo/.gemini/skills")),
                user_commands: None,
                repo_commands: None,
                user_agents: None,
                repo_agents: None,
                skill_also_reads_from: vec![],
                command_also_reads_from: vec![],
                agent_also_reads_from: vec![],
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
        assert!(
            gemini_diags
                .iter()
                .any(|d| d.message.contains("skills directory"))
        );
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
        assert!(
            gemini_diags
                .iter()
                .any(|d| d.message.contains("base configuration directory"))
        );
    }

    #[test]
    fn no_diagnostic_when_skills_dir_exists() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths_with_gemini(tmp.path());

        // Create Claude skills in user scope only
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
        // Only user scope: user-only skills are NOT expected in repo scope
        assert_eq!(gemini_missing.len(), 1);
        assert_eq!(gemini_missing[0].topic, "my-skill");
    }

    #[test]
    fn missing_exceptions_scope_aware_both_scopes() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths_with_gemini(tmp.path());

        // Create Claude skills in BOTH scopes
        let user_dir = tmp.path().join("user/skills");
        let repo_dir = tmp.path().join("repo/skills");
        setup_skill(&user_dir, "user-tool", "User tool", "# Body\n");
        setup_skill(&repo_dir, "repo-tool", "Repo tool", "# Body\n");

        // Create Gemini skills directories (exist but empty)
        fs::create_dir_all(tmp.path().join("gemini/skills")).unwrap();
        fs::create_dir_all(tmp.path().join("repo/.gemini/skills")).unwrap();

        let report = list_skills(&paths, &[]).unwrap();

        let gemini_missing: Vec<_> = report
            .exceptions
            .iter()
            .filter(|e| {
                e.provider == Provider::Gemini && e.exception_type == ExceptionType::Missing
            })
            .collect();
        // 1 from user scope (user-tool) + 1 from repo scope (repo-tool)
        assert_eq!(gemini_missing.len(), 2);
        let topics: BTreeSet<&str> = gemini_missing.iter().map(|e| e.topic.as_str()).collect();
        assert!(topics.contains("user-tool"));
        assert!(topics.contains("repo-tool"));
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
        assert!(
            gemini_diags
                .iter()
                .any(|d| d.message.contains("repo</b> scoped"))
        );
    }

    // ── fix_missing_skills tests ─────────────────────────────────────

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
        let paths =
            ProviderSkillPaths::from_providers_for_test(providers, tmp.path().to_path_buf());

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
        let before = list_skills(&paths, &[]).unwrap();
        assert!(!before.diagnostics.is_empty());

        // Apply fix
        fix_missing_skills(&paths).unwrap();

        // After fix: diagnostics and missing exceptions should be resolved
        let after = list_skills(&paths, &[]).unwrap();
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

    // ── SkillFilter parsing tests ────────────────────────────────────

    #[test]
    fn parse_simple_fuzzy() {
        let f = SkillFilter::parse("rust").unwrap();
        assert_eq!(f.pattern, "rust");
        assert!(!f.negated);
        assert!(!f.exact);
    }

    #[test]
    fn parse_exact_suffix() {
        let f = SkillFilter::parse("rust!").unwrap();
        assert_eq!(f.pattern, "rust");
        assert!(!f.negated);
        assert!(f.exact);
    }

    #[test]
    fn parse_negation_dash_prefix() {
        let f = SkillFilter::parse("-rust").unwrap();
        assert_eq!(f.pattern, "rust");
        assert!(f.negated);
        assert!(!f.exact);
    }

    #[test]
    fn parse_negation_bang_prefix() {
        let f = SkillFilter::parse("!rust").unwrap();
        assert_eq!(f.pattern, "rust");
        assert!(f.negated);
        assert!(!f.exact);
    }

    #[test]
    fn parse_negation_and_exact() {
        let f = SkillFilter::parse("-rust!").unwrap();
        assert_eq!(f.pattern, "rust");
        assert!(f.negated);
        assert!(f.exact);
    }

    #[test]
    fn parse_is_case_insensitive() {
        let f = SkillFilter::parse("Rust").unwrap();
        assert_eq!(f.pattern, "rust");
    }

    #[test]
    fn parse_empty_returns_none() {
        assert!(SkillFilter::parse("").is_none());
        assert!(SkillFilter::parse("-").is_none());
        assert!(SkillFilter::parse("!").is_none());
        assert!(SkillFilter::parse("-!").is_none());
    }

    #[test]
    fn parse_all_filters_empty() {
        let raw = vec!["rust".to_string(), "".to_string(), "-!".to_string()];
        let filters = SkillFilter::parse_all(&raw);
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].pattern, "rust");
    }

    // ── SkillFilter matching tests ───────────────────────────────────

    #[test]
    fn fuzzy_matches_substring() {
        let f = SkillFilter::parse("us").unwrap();
        assert!(f.matches("rust"));
        assert!(f.matches("RUST"));
        assert!(!f.matches("python"));
    }

    #[test]
    fn exact_matches_only_full_name() {
        let f = SkillFilter::parse("rust!").unwrap();
        assert!(f.matches("rust"));
        assert!(f.matches("Rust"));
        assert!(!f.matches("rusty"));
        assert!(!f.matches("my-rust"));
    }

    // ── SkillFilter::retain tests ────────────────────────────────────

    #[test]
    fn retain_positive_fuzzy_only() {
        let filters = SkillFilter::parse_all(&["us".to_string()]);
        assert!(SkillFilter::retain(&filters, "rust"));
        assert!(!SkillFilter::retain(&filters, "python"));
    }

    #[test]
    fn retain_negation_only() {
        let filters = SkillFilter::parse_all(&["-rust".to_string()]);
        assert!(!SkillFilter::retain(&filters, "rust"));
        assert!(!SkillFilter::retain(&filters, "rusty"));
        assert!(SkillFilter::retain(&filters, "python"));
    }

    #[test]
    fn retain_negation_wins_over_positive() {
        let filters = SkillFilter::parse_all(&["us".to_string(), "-rust!".to_string()]);
        // "rust" matches positive "us" but is excluded by exact negation "-rust!"
        assert!(!SkillFilter::retain(&filters, "rust"));
        // "rusty" matches positive "us" and is NOT excluded by exact negation "-rust!"
        assert!(SkillFilter::retain(&filters, "rusty"));
    }

    #[test]
    fn retain_combined_positive_and_negation() {
        let filters = SkillFilter::parse_all(&["a".to_string(), "-alpha".to_string()]);
        // "gamma" contains "a" → included, not negated → kept
        assert!(SkillFilter::retain(&filters, "gamma"));
        // "alpha" contains "a" → included, but negated by "-alpha" → excluded
        assert!(!SkillFilter::retain(&filters, "alpha"));
        // "beta" contains "a" → included, not negated → kept
        assert!(SkillFilter::retain(&filters, "beta"));
        // "xyz" does not contain "a" → not included → excluded
        assert!(!SkillFilter::retain(&filters, "xyz"));
    }

    #[test]
    fn retain_only_negations_keeps_non_matches() {
        let filters = SkillFilter::parse_all(&["-beta".to_string()]);
        assert!(SkillFilter::retain(&filters, "alpha"));
        assert!(!SkillFilter::retain(&filters, "beta"));
        assert!(SkillFilter::retain(&filters, "gamma"));
    }

    // ── list_skills with new filter modes ────────────────────────────

    #[test]
    fn list_skills_negation_excludes_match() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "alpha", "Alpha", "# A\n");
        setup_skill(&user_dir, "beta", "Beta", "# B\n");
        setup_skill(&user_dir, "gamma", "Gamma", "# G\n");

        let report = list_skills(&paths, &["-beta".to_string()]).unwrap();
        let names: Vec<&str> = report.skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(!names.contains(&"beta"));
        assert!(names.contains(&"gamma"));
    }

    #[test]
    fn list_skills_exact_matches_only_full_name() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "alpha", "Alpha", "# A\n");
        setup_skill(&user_dir, "alpha-extended", "Alpha Ext", "# AE\n");

        let report = list_skills(&paths, &["alpha!".to_string()]).unwrap();
        assert_eq!(report.skills.len(), 1);
        assert_eq!(report.skills[0].name, "alpha");
    }

    #[test]
    fn list_skills_negation_exact_combo() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(tmp.path());
        let user_dir = tmp.path().join("user/skills");
        setup_skill(&user_dir, "rust", "Rust", "# R\n");
        setup_skill(&user_dir, "rusty", "Rusty", "# Ry\n");
        setup_skill(&user_dir, "python", "Python", "# P\n");

        // Fuzzy "rust" but exclude exact "rust"
        let report = list_skills(&paths, &["rust".to_string(), "-rust!".to_string()]).unwrap();
        let names: Vec<&str> = report.skills.iter().map(|s| s.name.as_str()).collect();
        assert!(!names.contains(&"rust"));
        assert!(names.contains(&"rusty"));
        assert!(!names.contains(&"python"));
    }
}
