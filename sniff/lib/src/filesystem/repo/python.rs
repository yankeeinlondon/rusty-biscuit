//! Python pyproject.toml / requirements.txt parsing helpers.

use std::path::Path;

use biscuit_file::toml_crate;
use tracing::debug;

use crate::package::{DependencyEntry, DependencyKind};
use crate::performance;
use crate::performance::counters;

/// Extracts package name from a PEP 508 requirement string.
pub(super) fn parse_python_requirement_name(requirement: &str) -> Option<String> {
    let without_comment = requirement.split('#').next()?.trim();
    if without_comment.is_empty() {
        return None;
    }

    let before_marker = without_comment
        .split(';')
        .next()
        .unwrap_or(without_comment)
        .trim();
    if before_marker.is_empty() {
        return None;
    }

    let before_at = before_marker
        .split('@')
        .next()
        .unwrap_or(before_marker)
        .trim();
    if before_at.is_empty() {
        return None;
    }

    let end = before_at
        .find(|c: char| {
            c == '['
                || c == '<'
                || c == '>'
                || c == '='
                || c == '!'
                || c == '~'
                || c.is_whitespace()
        })
        .unwrap_or(before_at.len());
    let name = before_at[..end].trim();

    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Parses pyproject.toml dependencies from an already-parsed TOML value.
pub(super) fn pyproject_dependencies_from_value(
    parsed: &toml_crate::Value,
) -> Option<(Vec<DependencyEntry>, Vec<DependencyEntry>)> {
    let project = parsed.get("project")?;

    let dependencies = project
        .get("dependencies")
        .and_then(|v| v.as_array())
        .map(|deps| {
            deps.iter()
                .filter_map(|entry| entry.as_str())
                .filter_map(|req| parse_python_requirement_name(req).map(|name| (name, req)))
                .map(|(name, req)| DependencyEntry {
                    name,
                    kind: DependencyKind::Normal,
                    targeted_version: req.to_string(),
                    actual_version: None,
                    package_manager: Some("pip".to_string()),
                    latest_version: None,
                    target: None,
                    optional: false,
                    features: Vec::new(),
                    is_updatable: false,
                    has_major_update: false,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let optional_dependencies = project
        .get("optional-dependencies")
        .and_then(|v| v.as_table())
        .map(|groups| {
            groups
                .values()
                .filter_map(|value| value.as_array())
                .flat_map(|entries| entries.iter())
                .filter_map(|entry| entry.as_str())
                .filter_map(|req| parse_python_requirement_name(req).map(|name| (name, req)))
                .map(|(name, req)| DependencyEntry {
                    name,
                    kind: DependencyKind::Optional,
                    targeted_version: req.to_string(),
                    actual_version: None,
                    package_manager: Some("pip".to_string()),
                    latest_version: None,
                    target: None,
                    optional: true,
                    features: Vec::new(),
                    is_updatable: false,
                    has_major_update: false,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some((dependencies, optional_dependencies))
}

/// Parses requirements.txt dependency lines.
pub(super) fn parse_requirements_txt_dependencies(
    requirements_path: &Path,
) -> Option<Vec<DependencyEntry>> {
    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let content = std::fs::read_to_string(requirements_path)
        .map_err(|e| {
            debug!(path = %requirements_path.display(), error = %e, "could not read file");
            e
        })
        .ok()?;
    performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
    performance::increment_counter(counters::REPO_MANIFEST_PARSES, 1);
    let mut deps = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some(name) = parse_python_requirement_name(trimmed) else {
            continue;
        };

        deps.push(DependencyEntry {
            name,
            kind: DependencyKind::Normal,
            targeted_version: trimmed.to_string(),
            actual_version: None,
            package_manager: Some("pip".to_string()),
            latest_version: None,
            target: None,
            optional: false,
            features: Vec::new(),
            is_updatable: false,
            has_major_update: false,
        });
    }

    if deps.is_empty() { None } else { Some(deps) }
}

/// Extracts the package name from a parsed pyproject.toml `[project].name`.
pub(crate) fn pyproject_package_name(parsed: &toml_crate::Value) -> Option<String> {
    parsed
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(String::from)
}

/// Extracts the package version from a parsed pyproject.toml `[project].version`.
pub(crate) fn pyproject_package_version(parsed: &toml_crate::Value) -> Option<String> {
    parsed
        .get("project")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from)
}
