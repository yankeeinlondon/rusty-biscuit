use std::path::Path;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::repo::{Package, RepoInfo};

use super::RepoFilter;
use super::packages::{dirty_package_names, staged_package_names, unstaged_package_names};
use super::resolve_dir;
use crate::args::PackagesFormat;

/// Collect unique package area names, honoring the optional scope and filters.
fn select_repo_package_areas<'a>(
    packages: &'a [Package],
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> Vec<&'a str> {
    select_repo_package_areas_with_roots(packages, repo_filter, package, package_area)
        .into_iter()
        .map(|(area, _)| area)
        .collect()
}

/// Compute the repo-relative area root directory for a given package.
///
/// For a package whose `package_area` is `"root"` (a top-level package living
/// directly at the repo root, such as `model_id` in this workspace), returns
/// `"."` — the repo root itself.
///
/// Otherwise, when `pkg.relative` starts with `pkg.package_area` (the common
/// case — including multi-segment areas such as `apps/browser`), the area root
/// is the `package_area` value verbatim. This preserves correctness for nested
/// monorepo layouts where an area name can legitimately contain `/`.
///
/// If neither of those holds, falls back to the first path component of
/// `pkg.relative`, which matches `Package::package_area` for the overwhelming
/// majority of this workspace's layouts.
///
/// Returns a borrowed `&str` to avoid allocation in the hot render loop.
pub(super) fn package_area_root(pkg: &Package) -> &str {
    if pkg.package_area == "root" {
        return ".";
    }

    let relative = pkg.relative.trim_start_matches("./");

    // Prefer `pkg.package_area` when it prefixes `relative` at a path boundary.
    // This handles multi-segment areas like `apps/browser/my_package` where the
    // area is `apps/browser` — naive `split('/').next()` would incorrectly
    // return `apps`.
    if relative == pkg.package_area {
        return &pkg.package_area;
    }
    if let Some(rest) = relative.strip_prefix(pkg.package_area.as_str())
        && rest.starts_with('/')
    {
        return &pkg.package_area;
    }

    relative.split('/').next().unwrap_or(relative)
}

/// Same selection logic as [`select_repo_package_areas`] but also returns the
/// repo-relative area root directory derived from each area's first package.
///
/// `package` matches by exact case-insensitive `Package.name` and narrows the
/// area set to the area containing that package. `package_area` matches by
/// case-insensitive **prefix** on `Package.package_area`, so
/// `--package-area homelab` includes both `homelab` and `homelab/server`.
fn select_repo_package_areas_with_roots<'a>(
    packages: &'a [Package],
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> Vec<(&'a str, &'a str)> {
    // Resolve `--package` to its declared area so we can apply the exact-name
    // filter on the area level (an area is shown only if it contains the
    // matched package).
    let package_areas: Option<std::collections::HashSet<String>> = package.map(|name| {
        let needle = name.to_lowercase();
        packages
            .iter()
            .filter(|p| p.name.to_lowercase() == needle)
            .map(|p| p.package_area.to_lowercase())
            .collect()
    });

    // Capture the first package encountered for each area (deterministic via
    // BTreeMap ordering) so we can derive the area root once.
    let mut seen: std::collections::BTreeMap<&str, &Package> = std::collections::BTreeMap::new();
    for pkg in packages {
        seen.entry(pkg.package_area.as_str()).or_insert(pkg);
    }

    let scope = package_area.map(str::to_lowercase);
    let filters: Vec<RepoFilter> = if repo_filter.is_empty() {
        Vec::new()
    } else {
        repo_filter.iter().map(|f| RepoFilter::parse(f)).collect()
    };

    seen.into_iter()
        .filter(|(area, _)| {
            let lower = area.to_lowercase();
            if let Some(needle) = scope.as_deref()
                && !lower.starts_with(needle)
            {
                return false;
            }
            if let Some(ref allowed) = package_areas
                && !allowed.contains(&lower)
            {
                return false;
            }
            if filters.is_empty() {
                return true;
            }
            filters.iter().any(|f| {
                let hit = lower.contains(&f.query.to_lowercase());
                if f.negate { !hit } else { hit }
            })
        })
        .map(|(area, pkg)| (area, package_area_root(pkg)))
        .collect()
}

/// Collect unique package area names matching the given filters and scope.
///
/// Returns an empty vec when the repo is not a monorepo.
pub fn collect_repo_package_area_names<'a>(
    repo: &'a RepoInfo,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> Vec<&'a str> {
    if !repo.is_monorepo {
        return Vec::new();
    }
    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };
    select_repo_package_areas(packages, repo_filter, package, package_area)
}

/// Render the unique package area list for `sniff repo package-areas` in the
/// requested format.
///
/// Honors `--md` (Markdown unordered list), `--list` (one entry per line), and
/// the default csv form. With `verbose > 0`, each entry is annotated with the
/// dimmed repo-relative area directory.
pub fn render_repo_package_areas_formatted(
    repo: &RepoInfo,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
    format: PackagesFormat,
    verbose: u8,
) -> String {
    if !repo.is_monorepo {
        return String::from(
            "- the \"package-areas\" subcommand is only intended to be used in a monorepo",
        );
    }

    let Some(packages) = repo.packages.as_ref() else {
        return String::new();
    };

    let areas = select_repo_package_areas_with_roots(packages, repo_filter, package, package_area);
    if areas.is_empty() {
        return String::new();
    }

    let term = Terminal::default();
    let entries: Vec<String> = areas
        .iter()
        .map(|(area, root)| {
            let markup = if verbose > 0 {
                // Special-case the "root" area so the annotation reads
                // "root (./)" rather than "root (./root)" (a non-existent
                // directory). Every other area renders as "./{root}".
                let dir_label = if *root == "." {
                    String::from("./")
                } else {
                    format!("./{root}")
                };
                // Note the SPACE before the open paren — spec requires
                // "{package-area} (<dim><i>{dir}</i></dim>)".
                format!("{area} (<dim><i>{dir_label}</i></dim>)")
            } else {
                (*area).to_string()
            };
            Prose::new(markup).render(&term)
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
/// Pure selector returning dirty package area names, honoring the repo `filter`.
///
/// Returns an empty vector when the repo is not a monorepo or when no
/// packages are detected. The result is sorted and de-duplicated.
pub(crate) fn select_dirty_package_area_names(
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
    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };

    let dirty_names = dirty_package_names(result);
    let dirty_set: std::collections::HashSet<&str> =
        dirty_names.iter().map(|n| n.as_str()).collect();

    let filtered =
        super::packages::select_repo_packages(packages, repo_filter, package, package_area);
    let mut areas: Vec<String> = filtered
        .iter()
        .filter(|p| dirty_set.contains(p.name.as_str()))
        .map(|p| p.package_area.clone())
        .collect();
    areas.sort();
    areas.dedup();
    areas
}
/// Render package area names with uncommitted changes as a comma-separated list.
///
/// Returns an error message if the repo is not a monorepo.
pub fn render_dirty_package_areas(
    result: &sniff::SniffResult,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> String {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            select_dirty_package_area_names(result, repo_filter, package, package_area).join(", ")
        }
        _ => String::from(
            "- the \"--dirty-package-areas\" switch is only intended to be used in a monorepo",
        ),
    }
}
/// Pure selector returning staged package area names, honoring the repo `filter`.
pub(crate) fn select_staged_package_area_names(
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
    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };

    let staged_names = staged_package_names(result);
    let staged_set: std::collections::HashSet<&str> =
        staged_names.iter().map(|n| n.as_str()).collect();

    let filtered =
        super::packages::select_repo_packages(packages, repo_filter, package, package_area);
    let mut areas: Vec<String> = filtered
        .iter()
        .filter(|p| staged_set.contains(p.name.as_str()))
        .map(|p| p.package_area.clone())
        .collect();
    areas.sort();
    areas.dedup();
    areas
}
/// Render package area names with staged files as a comma-separated list.
///
/// Returns an error message if the repo is not a monorepo.
pub fn render_staged_package_areas(
    result: &sniff::SniffResult,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> String {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            select_staged_package_area_names(result, repo_filter, package, package_area).join(", ")
        }
        _ => String::from(
            "- the \"staged-package-areas\" subcommand is only intended to be used in a monorepo",
        ),
    }
}
/// Pure selector returning unstaged package area names, honoring the repo `filter`.
pub(crate) fn select_unstaged_package_area_names(
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
    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };

    let unstaged_names = unstaged_package_names(result);
    let unstaged_set: std::collections::HashSet<&str> =
        unstaged_names.iter().map(|n| n.as_str()).collect();

    let filtered =
        super::packages::select_repo_packages(packages, repo_filter, package, package_area);
    let mut areas: Vec<String> = filtered
        .iter()
        .filter(|p| unstaged_set.contains(p.name.as_str()))
        .map(|p| p.package_area.clone())
        .collect();
    areas.sort();
    areas.dedup();
    areas
}
/// Render package area names with unstaged changes as a comma-separated list.
///
/// Returns an error message if the repo is not a monorepo.
pub fn render_unstaged_package_areas(
    result: &sniff::SniffResult,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> String {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            select_unstaged_package_area_names(result, repo_filter, package, package_area)
                .join(", ")
        }
        _ => String::from(
            "- the \"unstaged-package-areas\" subcommand is only intended to be used in a monorepo",
        ),
    }
}
/// Render the package area for the given directory.
///
/// Returns empty string if not in a package area. In a monorepo, a directory
/// whose crates are not yet workspace members resolves via the directory
/// structure (e.g. a freshly scaffolded area).
pub fn render_repo_package_area(result: &sniff::SniffResult, base_dir: Option<&Path>) -> String {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    repo.and_then(|r| r.package_area_label_for_dir(&dir))
        .unwrap_or_default()
        .to_string()
}

/// Render the area name for the given directory.
///
/// Returns the package name if `dir` is inside a package, otherwise the
/// surrounding package-area string (with `"root"` as the final fallback for
/// top-level locations). Returns an empty string only when the underlying
/// repository is not a monorepo (callers handle that path separately).
pub fn render_repo_area(result: &sniff::SniffResult, base_dir: Option<&Path>) -> String {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    let Some(repo) = repo else {
        return String::new();
    };
    if !repo.is_monorepo {
        return String::new();
    }
    repo.area_for_dir(&dir).to_string()
}

/// Render the root directory of the package area containing the given directory.
///
/// Returns empty string if not in a package area. Root-level packages (area
/// `"root"`) are not considered to be inside a package area directory, so this
/// also returns empty for them.
pub fn render_repo_package_area_root(
    result: &sniff::SniffResult,
    base_dir: Option<&Path>,
) -> String {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    if let Some(area) = repo.and_then(|r| r.package_area_for_dir(&dir)) {
        if area == "root" {
            // Root-level packages have no real package area directory
            String::new()
        } else {
            repo.unwrap()
                .root
                .join(area)
                .display()
                .to_string()
                .replace(std::path::MAIN_SEPARATOR, "/")
        }
    } else {
        String::new()
    }
}
