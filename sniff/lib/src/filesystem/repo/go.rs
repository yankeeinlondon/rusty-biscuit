//! Go module (`go.mod`) parsing and `go.work` workspace detection helpers.

use std::path::Path;

use crate::Result;
use crate::package::{DependencyEntry, DependencyKind};

use super::detection::{create_package, dedupe_packages, resolve_internal_deps};
use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};

pub(super) fn detect_go_workspace(root: &Path) -> Result<Option<RepoInfo>> {
    let go_work = root.join("go.work");
    if !go_work.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&go_work)?;
    let uses = parse_go_work_uses(&content);
    if uses.is_empty() {
        return Ok(None);
    }

    let lock_versions = None;
    let mut packages = Vec::new();
    for member in uses {
        let member_path = root.join(&member);
        if !member_path.exists() {
            continue;
        }
        packages.push(create_package(
            &member_path,
            root,
            MonorepoTool::Unknown,
            &lock_versions,
            PackageDiscoverySource::ManifestScan,
        ));
    }

    if packages.is_empty() {
        return Ok(None);
    }

    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: None,
        workspace_tools: Vec::new(),
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        monorepo_standards: Vec::new(),
        monorepo_layers: Vec::new(),
        packages: Some(packages),
    }))
}

/// Parse the relative module paths from a `go.work` file's `use` directives.
///
/// Handles both the single-line form (`use ./svc-a`) and the parenthesized
/// block form (`use (\n\t./svc-a\n\t./svc-b\n)`). Leading `./` and surrounding
/// quotes are stripped so each entry is a clean repo-relative path.
pub(super) fn parse_go_work_uses(content: &str) -> Vec<String> {
    let mut uses = Vec::new();
    let mut in_block = false;

    for line in content.lines() {
        let trimmed = line.split("//").next().unwrap_or("").trim();
        if trimmed.is_empty() {
            continue;
        }

        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
                continue;
            }
            if let Some(path) = clean_use_path(trimmed) {
                uses.push(path);
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("use") {
            let rest = rest.trim();
            if let Some(after_paren) = rest.strip_prefix('(') {
                in_block = true;
                // Tolerate `use ( ./a` on the opening line, though gofmt splits it.
                if let Some(path) = clean_use_path(after_paren.trim()) {
                    uses.push(path);
                }
                continue;
            }
            if let Some(path) = clean_use_path(rest) {
                uses.push(path);
            }
        }
    }

    uses
}

/// Normalize a single `use` entry to a repo-relative path, or `None` when blank.
fn clean_use_path(raw: &str) -> Option<String> {
    let unquoted = raw.trim().trim_matches('"');
    let relative = unquoted.trim_start_matches("./").trim_end_matches('/');
    if relative.is_empty() {
        None
    } else {
        Some(relative.to_string())
    }
}

/// Parses go.mod `require` entries from cached file contents.
pub(super) fn go_mod_dependencies_from_content(content: &str) -> Option<Vec<DependencyEntry>> {
    let mut deps = Vec::new();
    let mut in_require_block = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if in_require_block {
            if trimmed.starts_with(')') {
                in_require_block = false;
                continue;
            }

            let line_without_comment = trimmed.split("//").next().unwrap_or(trimmed).trim();
            let mut parts = line_without_comment.split_whitespace();
            let Some(name) = parts.next() else {
                continue;
            };
            let Some(version) = parts.next() else {
                continue;
            };

            deps.push(DependencyEntry {
                name: name.to_string(),
                kind: DependencyKind::Normal,
                targeted_version: version.to_string(),
                actual_version: None,
                package_manager: Some("go".to_string()),
                latest_version: None,
                target: None,
                optional: false,
                features: Vec::new(),
                is_updatable: false,
                has_major_update: false,
            });

            continue;
        }

        if trimmed.starts_with("require (") {
            in_require_block = true;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("require ") {
            let line_without_comment = rest.split("//").next().unwrap_or(rest).trim();
            let mut parts = line_without_comment.split_whitespace();
            let Some(name) = parts.next() else {
                continue;
            };
            let Some(version) = parts.next() else {
                continue;
            };

            deps.push(DependencyEntry {
                name: name.to_string(),
                kind: DependencyKind::Normal,
                targeted_version: version.to_string(),
                actual_version: None,
                package_manager: Some("go".to_string()),
                latest_version: None,
                target: None,
                optional: false,
                features: Vec::new(),
                is_updatable: false,
                has_major_update: false,
            });
        }
    }

    if deps.is_empty() { None } else { Some(deps) }
}

/// Extracts the module name from go.mod content (`module` directive).
pub(super) fn go_module_name_from_content(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("module ")
            .map(|value| value.trim().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uses_reads_block_form() {
        let content = "go 1.21\n\nuse (\n\t./svc-a\n\t./svc-b\n)\n";
        assert_eq!(parse_go_work_uses(content), vec!["svc-a", "svc-b"]);
    }

    #[test]
    fn parse_uses_reads_single_line_form() {
        let content = "go 1.21\n\nuse ./only\n";
        assert_eq!(parse_go_work_uses(content), vec!["only"]);
    }

    #[test]
    fn parse_uses_empty_block_yields_nothing() {
        let content = "go 1.21\n\nuse (\n)\n";
        assert!(parse_go_work_uses(content).is_empty());
    }

    #[test]
    fn parse_uses_strips_comments_and_nested_paths() {
        let content = "use (\n\t./svc-a // primary\n\t./group/svc-b\n)\n";
        assert_eq!(parse_go_work_uses(content), vec!["svc-a", "group/svc-b"]);
    }
}
