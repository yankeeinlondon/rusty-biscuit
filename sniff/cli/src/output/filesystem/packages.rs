use std::path::{Path, PathBuf};

use biscuit_terminal::components::prose::{InlineProse, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::blast_radius::ChangeScope;
use sniff::filesystem::git::{FileStatus, GitInfo};
use sniff::filesystem::repo::{Package, RepoInfo, attribute_paths, observed_scope_paths};

use super::filter_packages;
use super::resolve_dir;
use crate::args::PackagesFormat;

/// Render package names as a comma-separated plain text list.
///
/// Returns an error message if the repo is not a monorepo.
/// Filter a package list by name/area filters and optional `--package` /
/// `--package-area` scopes.
///
/// `package` matches by exact case-insensitive `Package.name`. `package_area`
/// matches by case-insensitive **prefix** on `Package.package_area`, so
/// `--package-area homelab` includes `homelab` and `homelab/server`.
pub(crate) fn select_repo_packages<'a>(
    packages: &'a [Package],
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> Vec<&'a Package> {
    let mut filtered = filter_packages(packages, repo_filter);
    if let Some(name) = package {
        let needle = name.to_lowercase();
        filtered.retain(|p| p.name.to_lowercase() == needle);
    }
    if let Some(area) = package_area {
        let needle = area.to_lowercase();
        filtered.retain(|p| p.package_area.to_lowercase().starts_with(&needle));
    }
    filtered
}

/// Collect package names matching the given filters and scope.
///
/// Includes a single standalone root package: the canonical catalog
/// ([`RepoInfo::packages`]) holds one entry for a recognized non-monorepo
/// project, so `packages` / `package-count` describe the same package universe
/// as the repo-wide `package_manager` and `dependencies` facts. Returns an
/// empty vec only when no package catalog is present.
pub fn collect_repo_package_names<'a>(
    repo: &'a RepoInfo,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> Vec<&'a str> {
    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };
    select_repo_packages(packages, repo_filter, package, package_area)
        .into_iter()
        .map(|p| p.name.as_str())
        .collect()
}

/// Render a styled package entry; with `verbose > 0` appends the dimmed/italic
/// repo-relative root directory (e.g. `name(<dim><i>./relative</i></dim>)`).
fn package_entry_markup(pkg: &Package, verbose: u8) -> String {
    if verbose > 0 {
        format!(
            "{}(<dim><i>./{}</i></dim>)",
            pkg.name,
            pkg.relative.trim_start_matches("./")
        )
    } else {
        pkg.name.clone()
    }
}

/// Render the package list for `sniff repo packages` in the requested format.
///
/// Honors `--md` (Markdown unordered list), `--list` (one entry per line), and
/// the default csv form. With `verbose > 0`, each entry is annotated with the
/// dimmed package root directory.
pub fn render_repo_packages_formatted(
    repo: &RepoInfo,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
    format: PackagesFormat,
    verbose: u8,
) -> String {
    let Some(packages) = repo.packages.as_ref() else {
        return String::new();
    };

    let filtered = select_repo_packages(packages, repo_filter, package, package_area);
    if filtered.is_empty() {
        return String::new();
    }

    let term = Terminal::default();
    let entries: Vec<String> = filtered
        .iter()
        .map(|pkg| {
            let markup = package_entry_markup(pkg, verbose);
            InlineProse::new(markup).render(&term)
        })
        .collect();

    match format {
        PackagesFormat::Csv => entries.join(", "),
        PackagesFormat::Markdown => entries
            .iter()
            .map(|e| format!("- {e}"))
            .collect::<Vec<_>>()
            .join("\n"),
        PackagesFormat::List => entries.join("\n"),
    }
}
/// Collect package names that own an uncommitted change: staged, unstaged,
/// untracked, or conflicted.
///
/// Empty when the result carries no package catalog or its changed paths were
/// not observed ([`sniff::filesystem::repo::observed_scope_paths`]).
pub(crate) fn dirty_package_names(result: &sniff::SniffResult) -> Vec<String> {
    changed_package_names(result, |git| {
        observed_scope_paths(git, ChangeScope::Dirty).unwrap_or_default()
    })
}

/// Names of the packages that own any of the changed paths `select` picks,
/// sorted and de-duplicated, using the library's package ownership rules.
fn changed_package_names(
    result: &sniff::SniffResult,
    select: impl FnOnce(&GitInfo) -> Vec<PathBuf>,
) -> Vec<String> {
    let Some(fs) = result.filesystem.as_ref() else {
        return vec![];
    };
    let Some(packages) = fs.repo.as_ref().and_then(|r| r.packages.as_ref()) else {
        return vec![];
    };
    let Some(git) = fs.git.as_ref() else {
        return vec![];
    };
    let paths = select(git);
    if paths.is_empty() {
        return vec![];
    }
    attribute_paths(packages, &paths).packages
}

/// Pure selector returning dirty package names, honoring the repo `filter`.
///
/// Returns an empty vector when the repo is not a monorepo, when no packages
/// are detected, or when no dirty paths exist. The result is sorted and
/// de-duplicated.
///
/// ## Notes
///
/// This is the JSON-friendly counterpart to [`render_dirty_packages`]: it
/// never produces prose error strings, so JSON consumers see an empty array
/// instead of the "only intended to be used in a monorepo" message.
pub(crate) fn select_dirty_package_names(
    result: &sniff::SniffResult,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> Vec<String> {
    let Some(repo) = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref()) else {
        return Vec::new();
    };
    if !repo.is_monorepo {
        return Vec::new();
    }

    let names = dirty_package_names(result);
    if repo_filter.is_empty() && package.is_none() && package_area.is_none() {
        return names;
    }

    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };
    let filtered = select_repo_packages(packages, repo_filter, package, package_area);
    let filtered_names: std::collections::HashSet<&str> =
        filtered.iter().map(|p| p.name.as_str()).collect();
    names
        .into_iter()
        .filter(|n| filtered_names.contains(n.as_str()))
        .collect()
}
/// Render package names with uncommitted changes as a comma-separated list.
///
/// Returns an error message if the repo is not a monorepo.
pub fn render_dirty_packages(
    result: &sniff::SniffResult,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> String {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            select_dirty_package_names(result, repo_filter, package, package_area).join(", ")
        }
        _ => String::from(
            "- the \"--dirty-packages\" switch is only intended to be used in a monorepo",
        ),
    }
}
/// Collect package names that have staged files (in index).
pub(crate) fn staged_package_names(result: &sniff::SniffResult) -> Vec<String> {
    changed_package_names(result, |git| {
        observed_scope_paths(git, ChangeScope::Staged).unwrap_or_default()
    })
}

/// Pure selector returning staged package names, honoring the repo `filter`.
///
/// Returns an empty vector when the repo is not a monorepo or when nothing
/// is staged. The result is sorted and de-duplicated.
pub(crate) fn select_staged_package_names(
    result: &sniff::SniffResult,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> Vec<String> {
    let Some(repo) = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref()) else {
        return Vec::new();
    };
    if !repo.is_monorepo {
        return Vec::new();
    }

    let names = staged_package_names(result);
    if repo_filter.is_empty() && package.is_none() && package_area.is_none() {
        return names;
    }

    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };
    let filtered = select_repo_packages(packages, repo_filter, package, package_area);
    let filtered_names: std::collections::HashSet<&str> =
        filtered.iter().map(|p| p.name.as_str()).collect();
    names
        .into_iter()
        .filter(|n| filtered_names.contains(n.as_str()))
        .collect()
}
/// Render package names with staged files as a comma-separated list.
///
/// Returns an error message if the repo is not a monorepo.
pub fn render_staged_packages(
    result: &sniff::SniffResult,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> String {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            select_staged_package_names(result, repo_filter, package, package_area).join(", ")
        }
        _ => String::from(
            "- the \"staged-packages\" subcommand is only intended to be used in a monorepo",
        ),
    }
}
/// Collect package names that have unstaged changes (modified in working tree).
///
/// Cross-references git file_changes with `Modified` or `Both` status against
/// package relative paths. Unlike `ChangeScope::Unstaged`, conflicted paths are
/// not counted.
pub(crate) fn unstaged_package_names(result: &sniff::SniffResult) -> Vec<String> {
    changed_package_names(result, |git| {
        git.file_changes
            .iter()
            .filter(|f| matches!(f.status, FileStatus::Modified | FileStatus::Both))
            .map(|f| f.path.clone())
            .collect()
    })
}

/// Pure selector returning unstaged package names, honoring the repo `filter`.
pub(crate) fn select_unstaged_package_names(
    result: &sniff::SniffResult,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> Vec<String> {
    let Some(repo) = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref()) else {
        return Vec::new();
    };
    if !repo.is_monorepo {
        return Vec::new();
    }

    let names = unstaged_package_names(result);
    if repo_filter.is_empty() && package.is_none() && package_area.is_none() {
        return names;
    }

    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };
    let filtered = select_repo_packages(packages, repo_filter, package, package_area);
    let filtered_names: std::collections::HashSet<&str> =
        filtered.iter().map(|p| p.name.as_str()).collect();
    names
        .into_iter()
        .filter(|n| filtered_names.contains(n.as_str()))
        .collect()
}
/// Render package names with unstaged changes as a comma-separated list.
///
/// Returns an error message if the repo is not a monorepo.
pub fn render_unstaged_packages(
    result: &sniff::SniffResult,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> String {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            select_unstaged_package_names(result, repo_filter, package, package_area).join(", ")
        }
        _ => String::from(
            "- the \"unstaged-packages\" subcommand is only intended to be used in a monorepo",
        ),
    }
}
/// Render the package name for the given directory.
///
/// With `verbose >= 1`, appends the package root directory on the same line.
/// Returns empty string if not in a package.
pub fn render_repo_package(
    result: &sniff::SniffResult,
    base_dir: Option<&Path>,
    verbose: u8,
) -> String {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    if let Some(pkg) = repo.and_then(|r| r.package_for_dir(&dir)) {
        if verbose > 0 {
            let terminal = Terminal::default();
            let line = format!(
                "{} (<i>located in</i> <blue>{}</blue>)",
                pkg.name, pkg.relative
            );
            Prose::new(&line).display(&terminal).to_string()
        } else {
            pkg.name.clone()
        }
    } else {
        String::new()
    }
}

/// Render the root directory of the package containing the given directory.
///
/// Returns empty string if not in a package.
pub fn render_repo_package_root(result: &sniff::SniffResult, base_dir: Option<&Path>) -> String {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    repo.and_then(|r| r.package_for_dir(&dir))
        .map(|pkg| {
            pkg.path
                .display()
                .to_string()
                .replace(std::path::MAIN_SEPARATOR, "/")
        })
        .unwrap_or_default()
}
