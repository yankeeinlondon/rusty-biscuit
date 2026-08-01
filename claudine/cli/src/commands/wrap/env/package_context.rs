//! Monorepo package-context and launch-workspace resolution.
//!
//! Resolves the [`LaunchWorkspaceContext`] (repo root, child cwd, and the
//! monorepo [`PackageContext`]) the wrap pipeline stamps into the child env.
//! Two entry points exist: [`resolve_launch_workspace_context`] runs its own
//! sniff scans, while [`launch_workspace_context_from_repo_info`] reuses a
//! shared `SniffResult` to avoid extra filesystem walks.

use std::path::{Path, PathBuf};

use color_eyre::eyre::Result;
use sniff::filesystem::git::detect_git;
use sniff::filesystem::repo::{Package, RepoInfo, detect_repo};

use super::{LaunchWorkspaceContext, PackageContext};

struct RepoContext {
    package_context: Option<PackageContext>,
    warnings: Vec<String>,
}

fn detect_repo_root(cwd: &Path) -> Option<PathBuf> {
    detect_git(cwd, false, 1)
        .ok()
        .flatten()
        .map(|info| info.repo_root)
        .or_else(|| detect_repo(cwd).ok().flatten().map(|repo| repo.root))
}

pub(crate) fn resolve_launch_workspace_context(
    launch_cwd: &Path,
    repo_root_hint: Option<&Path>,
) -> LaunchWorkspaceContext {
    // `repo_root` is metadata — guardrails, MCP defaults, harness path
    // resolution. When composing a document, the caller passes the
    // document's enclosing git root as a hint so those subsystems key
    // off the document's repo (e.g. `@`-references, per-repo guardrails).
    let repo_root = repo_root_hint
        .map(Path::to_path_buf)
        .or_else(|| detect_repo_root(launch_cwd));
    // `child_cwd` is the directory the spawned provider process will
    // run in. It must ALWAYS follow the user's launch directory, never
    // the document hint — otherwise a sequence that composes a markdown
    // file from an unrelated nested clone would jump OpenCode/Claude/etc.
    // into that clone and flag the user's real worktree as external.
    let child_cwd = detect_repo_root(launch_cwd).unwrap_or_else(|| launch_cwd.to_path_buf());

    match resolve_monorepo_package_context(launch_cwd) {
        Ok(repo_ctx) => LaunchWorkspaceContext {
            launch_cwd: launch_cwd.to_path_buf(),
            repo_root,
            child_cwd,
            package_context: repo_ctx.package_context,
            warnings: repo_ctx.warnings,
        },
        Err(error) => LaunchWorkspaceContext {
            launch_cwd: launch_cwd.to_path_buf(),
            repo_root,
            child_cwd,
            package_context: None,
            warnings: vec![format!(
                "failed to resolve monorepo package metadata for '{}': {}",
                biscuit_file::to_portable_string(launch_cwd),
                error
            )],
        },
    }
}

/// Build a [`LaunchWorkspaceContext`] from data already produced by a
/// single `sniff::detect_with_plan` call, without performing any further
/// filesystem walks.
///
/// `git_root` should come from the filesystem git section of the shared
/// `SniffResult`; `repo` should come from its repo section. Callers that
/// have neither can pass `None` for both — the resulting context will
/// behave as if no repo was detected.
///
/// `source_repo_root_hint` preserves the legacy
/// [`resolve_launch_workspace_context`] split contract: when a composed
/// markdown source lives in a different repo than the launch CWD (sibling
/// clone, external prompt), the metadata-bearing `repo_root` should follow
/// the source's repo so guardrails, MCP defaults, and harness path
/// resolution key off the document. The `child_cwd` (where the spawned
/// provider process actually runs) must still follow the launch CWD's
/// repo root so the provider does not jump into an unrelated worktree.
pub(crate) fn launch_workspace_context_from_repo_info(
    launch_cwd: &Path,
    git_root: Option<&Path>,
    repo: Option<&RepoInfo>,
    source_repo_root_hint: Option<&Path>,
) -> LaunchWorkspaceContext {
    let launch_repo_root = git_root
        .map(Path::to_path_buf)
        .or_else(|| repo.map(|r| r.root.clone()));
    let repo_root = source_repo_root_hint
        .map(Path::to_path_buf)
        .or_else(|| launch_repo_root.clone());
    let child_cwd = launch_repo_root
        .clone()
        .unwrap_or_else(|| launch_cwd.to_path_buf());

    let (package_context, warnings) = match repo {
        Some(repo) if repo.is_monorepo => match repo.packages.as_deref() {
            Some(packages) => resolve_package_context_from_packages(launch_cwd, repo, packages),
            None => (
                None,
                vec![format!(
                    "monorepo detected at '{}' but no packages were reported",
                    biscuit_file::to_portable_string(&repo.root)
                )],
            ),
        },
        _ => (None, Vec::new()),
    };

    LaunchWorkspaceContext {
        launch_cwd: launch_cwd.to_path_buf(),
        repo_root,
        child_cwd,
        package_context,
        warnings,
    }
}

fn resolve_package_context_from_packages(
    cwd: &Path,
    repo: &RepoInfo,
    packages: &[Package],
) -> (Option<PackageContext>, Vec<String>) {
    if let Some(package_ctx) = select_package_for_cwd(cwd, packages) {
        return (Some(package_ctx), Vec::new());
    }

    if let Some(package_area) = select_package_area_for_cwd(cwd, &repo.root, packages) {
        let candidates = package_candidates_for_area(&package_area, packages);
        return (
            Some(PackageContext {
                package_area,
                package: None,
                candidates,
            }),
            Vec::new(),
        );
    }

    (
        None,
        vec![format!(
            "monorepo detected at '{}' but no package area matched cwd '{}'",
            biscuit_file::to_portable_string(&repo.root),
            biscuit_file::to_portable_string(cwd)
        )],
    )
}

fn resolve_monorepo_package_context(cwd: &Path) -> Result<RepoContext> {
    let git_root = detect_git(cwd, false, 1)?.map(|info| info.repo_root);
    let repo_probe_root = git_root.clone().unwrap_or_else(|| cwd.to_path_buf());
    let Some(repo) = detect_repo(&repo_probe_root)? else {
        return Ok(RepoContext {
            package_context: None,
            warnings: Vec::new(),
        });
    };

    if !repo.is_monorepo {
        return Ok(RepoContext {
            package_context: None,
            warnings: Vec::new(),
        });
    }

    let Some(packages) = repo.packages else {
        return Ok(RepoContext {
            package_context: None,
            warnings: vec![format!(
                "monorepo detected at '{}' but no packages were reported",
                biscuit_file::to_portable_string(&repo.root)
            )],
        });
    };

    if let Some(package_ctx) = select_package_for_cwd(cwd, &packages) {
        return Ok(RepoContext {
            package_context: Some(package_ctx),
            warnings: Vec::new(),
        });
    }

    if let Some(package_area) = select_package_area_for_cwd(cwd, &repo.root, &packages) {
        let candidates = package_candidates_for_area(&package_area, &packages);
        return Ok(RepoContext {
            package_context: Some(PackageContext {
                package_area,
                package: None,
                candidates,
            }),
            warnings: Vec::new(),
        });
    }

    Ok(RepoContext {
        package_context: None,
        warnings: vec![format!(
            "monorepo detected at '{}' but no package area matched cwd '{}'",
            biscuit_file::to_portable_string(&repo.root),
            biscuit_file::to_portable_string(cwd)
        )],
    })
}

pub(crate) fn select_package_for_cwd(cwd: &Path, packages: &[Package]) -> Option<PackageContext> {
    let cwd_normalized = canonical_or_self(cwd);

    packages
        .iter()
        .filter_map(|package| {
            let package_path = canonical_or_self(&package.path);
            if cwd_normalized.starts_with(&package_path) {
                Some((package_path.components().count(), package))
            } else {
                None
            }
        })
        .max_by_key(|(depth, _)| *depth)
        .map(|(_, package)| PackageContext {
            package_area: package.package_area.clone(),
            package: Some(package.name.clone()),
            candidates: vec![package.name.clone()],
        })
}

pub(crate) fn select_package_area_for_cwd(
    cwd: &Path,
    repo_root: &Path,
    packages: &[Package],
) -> Option<String> {
    let cwd_normalized = canonical_or_self(cwd);
    let repo_root_normalized = canonical_or_self(repo_root);

    packages
        .iter()
        .map(|package| {
            let area_root = if package.package_area == "root" {
                repo_root_normalized.clone()
            } else {
                repo_root_normalized.join(&package.package_area)
            };
            (area_root, package.package_area.clone())
        })
        .filter(|(area_root, _)| cwd_normalized.starts_with(area_root))
        .max_by_key(|(area_root, _)| area_root.components().count())
        .map(|(_, area)| area)
}

pub(crate) fn package_candidates_for_area(package_area: &str, packages: &[Package]) -> Vec<String> {
    let mut candidates: Vec<String> = packages
        .iter()
        .filter(|package| package.package_area == package_area)
        .map(|package| package.name.clone())
        .collect();
    candidates.sort();
    candidates.dedup();
    candidates
}

pub(crate) fn canonical_or_self(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
