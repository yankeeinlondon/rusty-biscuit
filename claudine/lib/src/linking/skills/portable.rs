use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use biscuit_file::serde_yaml_ng;
use regex::Regex;

use super::{
    ExceptionType, SkillDirectoryDiagnostic, SkillException, SkillFilter, SkillInfo, SkillScope,
    SkillsReport,
};
use crate::error::Result;
use crate::linking::capabilities::{ALL_PROVIDERS, LinkableResource, capabilities_for};
use crate::linking::compatibility::{frontmatter_has_indentation_tabs, parse_markdown_document};
use crate::linking::paths::{ProviderSkillPaths, ResourceScope};
use crate::provider::Provider;

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
pub(super) fn scan_skill_dir(dir: Option<&PathBuf>) -> Vec<(String, PathBuf)> {
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
mod tests;
