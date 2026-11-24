//! Focused `repo git-status` preparation.
//!
//! [`crate::filesystem::detect_repo_git_status`] runs one filesystem
//! detection, then applies the report's selectors (package or package-area
//! path history, a different branch, a linked worktree) and, when asked,
//! links the commits the report will display. Everything reuses the request's
//! one repository discovery and ref snapshot, so rendering the result reads
//! no repository at all.

use std::collections::HashMap;

use super::commit_links::{COMMIT_VISIT_BUDGET, CommitLink, link_commits_from_snapshot};
use super::discovery::{
    PathHistoryOptions, get_commits_for_branch_from_snapshot, get_commits_for_path_fallible,
    get_recent_commits_with_decorations,
};
use super::remote_refresh::RefSnapshot;
use super::types::{CommitInfo, FileStatus, GitInfo, GitRepo};
use crate::filesystem::FilesystemObservation;
use crate::filesystem::repo::{Package, resolve_package_and_area};
use crate::performance::StageTimer;
use crate::request::{FilesystemRequest, GitRequest, RepoRequest};
use crate::{Result, SniffError};

/// What a focused status report shows.
///
/// Selectors apply in a fixed order: `package` / `package_area` replace the
/// history with commits touching that path and narrow the working-tree
/// changes to it; a `branch` other than the current one then replaces the
/// history with that branch's and clears the working-tree changes; a
/// `worktree` finally replaces history and working-tree changes with the
/// selected linked worktree's. An empty `branch` keeps the current
/// selection.
#[derive(Debug, Clone, Copy, Default)]
pub struct GitStatusOptions<'a> {
    /// Commits to display. Zero displays none and links none, though a path
    /// selector still reports every matching commit, as before.
    pub history_count: usize,
    /// Fetch remote-tracking refs first and record each displayed commit's
    /// containing remotes (the deep Git tier).
    pub refresh_remotes: bool,
    /// Link the displayed commits to their containing remote's browser page.
    pub include_commit_links: bool,
    pub package: Option<&'a str>,
    pub package_area: Option<&'a str>,
    pub branch: Option<&'a str>,
    /// A linked worktree's branch name or directory basename.
    pub worktree: Option<&'a str>,
}

/// A prepared status report.
#[derive(Debug, Clone, Default)]
pub struct PreparedGitStatus {
    /// Git facts after every selector, or `None` outside a repository.
    pub git: Option<GitInfo>,
    /// Links for the displayed commits, keyed by full commit ID. Empty when
    /// linking was not requested, nothing is displayed, or the refs needed to
    /// decide containment could not be read; a displayed commit missing from
    /// a nonempty map had an invalid ID.
    pub commit_links: HashMap<String, CommitLink>,
}

/// The filesystem request behind a focused status report.
pub(crate) fn status_request(options: &GitStatusOptions<'_>) -> FilesystemRequest {
    let git = if options.refresh_remotes {
        GitRequest::deep()
    } else {
        GitRequest::full()
    }
    .commit_count(options.history_count);
    let request = FilesystemRequest::new()
        .git(git)
        .without_docs()
        .without_formatting()
        .without_file_inventory();
    // A package catalog is needed only to resolve a path selector, and the
    // structure tier is enough for that.
    if options.package.is_some() || options.package_area.is_some() {
        request.repo(RepoRequest::structure())
    } else {
        request.without_repo()
    }
}

/// Whether the initial ref snapshot must include every remote-tracking tip.
pub(crate) fn wants_link_tips(options: &GitStatusOptions<'_>) -> bool {
    options.include_commit_links && options.history_count > 0
}

/// Which handle and refs link the final history.
enum LinkSource {
    Initial(Option<RefSnapshot>),
    Worktree(Box<GitRepo>, Option<RefSnapshot>),
    /// The selected worktree disappeared after it was listed.
    Unavailable,
}

/// Apply the selectors to the initial detection and link the final history.
///
/// Selector validation runs even outside a repository. Each selector's reader
/// runs, and its errors propagate, even when a later selector replaces its
/// result.
pub(crate) fn finish_status(
    observation: &FilesystemObservation,
    initial: Option<(GitInfo, Option<RefSnapshot>)>,
    packages: Option<&[Package]>,
    options: &GitStatusOptions<'_>,
) -> Result<(Option<GitInfo>, HashMap<String, CommitLink>)> {
    let scope = resolve_package_and_area(packages, options.package, options.package_area)?;
    let Some((mut git, refs)) = initial else {
        return Ok((None, HashMap::new()));
    };

    let links = observation.with_repository(|repo| {
        let selection = StageTimer::start("filesystem.git.selection");
        let source = select(repo, &mut git, refs, scope.as_deref(), options)?;
        selection.finish();
        if !options.include_commit_links {
            return Ok(HashMap::new());
        }
        let shown = &git.recent[..git.recent.len().min(options.history_count)];
        Ok(match &source {
            LinkSource::Initial(Some(refs)) => {
                repo.with_cached_gix(|gix| link_displayed(gix, refs, shown))
            }
            LinkSource::Worktree(selected, Some(refs)) => {
                selected.with_cached_gix(|gix| link_displayed(gix, refs, shown))
            }
            LinkSource::Initial(None) | LinkSource::Worktree(_, None) | LinkSource::Unavailable => {
                HashMap::new()
            }
        })
    })?;
    Ok((Some(git), links.unwrap_or_default()))
}

fn select(
    repo: &GitRepo,
    git: &mut GitInfo,
    refs: Option<RefSnapshot>,
    scope: Option<&str>,
    options: &GitStatusOptions<'_>,
) -> Result<LinkSource> {
    if let Some(prefix) = scope {
        // A zero-count detection observed no decorations; the path reader then
        // collects its own, as it always has.
        let decorations = refs
            .as_ref()
            .filter(|refs| refs.has_decorations())
            .map(RefSnapshot::decorations);
        let scoped = repo.with_cached_gix(|gix| {
            get_commits_for_path_fallible(
                gix,
                prefix,
                PathHistoryOptions::new(options.history_count),
                decorations,
            )
        })?;
        git.recent = scoped.commits;
        narrow_working_tree(git, prefix);
    }

    if let Some(branch) = options.branch.filter(|branch| !branch.is_empty())
        && git.current_branch.as_deref() != Some(branch)
    {
        git.recent = repo.with_cached_gix(|gix| {
            get_commits_for_branch_from_snapshot(gix, refs.as_ref(), branch, options.history_count)
        })?;
        // Working-tree state belongs to the checked-out branch, not the one
        // being inspected.
        clear_working_tree(git);
    }

    match options.worktree {
        Some(selector) => select_worktree(repo, git, refs, selector, options),
        None => Ok(LinkSource::Initial(refs)),
    }
}

fn select_worktree(
    repo: &GitRepo,
    git: &mut GitInfo,
    refs: Option<RefSnapshot>,
    selector: &str,
    options: &GitStatusOptions<'_>,
) -> Result<LinkSource> {
    let matched = git
        .worktrees
        .values()
        .find(|info| {
            info.branch == selector
                || info
                    .filepath
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|basename| basename == selector)
        })
        .cloned();
    let Some(info) = matched else {
        let names: Vec<String> = git
            .worktrees
            .values()
            .map(|info| info.branch.clone())
            .collect();
        let listing = if names.is_empty() {
            "no linked worktrees found".to_string()
        } else {
            format!("available: {}", names.join(", "))
        };
        return Err(SniffError::RepositorySelection(format!(
            "Worktree not found: {selector} ({listing})"
        )));
    };

    let source = match GitRepo::discover(&info.filepath)? {
        Some(selected) => {
            let refs = selected_worktree_refs(repo, &selected, refs, options);
            // Best effort, as `GitRepo::recent_commits` and `repo_status` are:
            // a worktree whose refs or status cannot be read still reports.
            git.recent = selected.with_cached_gix(|gix| {
                get_recent_commits_with_decorations(
                    gix,
                    options.history_count,
                    refs.as_ref().map(RefSnapshot::decorations),
                )
            });
            let (status, file_changes) = selected
                .with_cached_gix(|gix| super::status::get_repo_status_with_changes(gix, false))
                .unwrap_or_default();
            git.status = Some(status);
            git.file_changes = file_changes;
            LinkSource::Worktree(Box::new(selected), refs)
        }
        None => LinkSource::Unavailable,
    };
    // Absolute file paths in the report resolve against the worktree, and the
    // selected branch reads as the current one.
    git.repo_root = info.filepath.clone();
    git.current_branch = Some(info.branch.clone());
    Ok(source)
}

/// Refs for the selected worktree's history decorations and links.
///
/// The initial snapshot is reused, with `HEAD` re-marked for the selected
/// worktree, when both handles provably read the same refs. Otherwise one
/// snapshot is taken on the selected handle; if that fails, the worktree
/// reports undecorated history and no links.
fn selected_worktree_refs(
    initial: &GitRepo,
    selected: &GitRepo,
    refs: Option<RefSnapshot>,
    options: &GitStatusOptions<'_>,
) -> Option<RefSnapshot> {
    if options.history_count == 0 {
        return None;
    }
    if let Some(refs) = refs.as_ref()
        && refs.is_worktree_independent()
        && initial.with_gix(|a| selected.with_gix(|b| same_ref_view(a, b)))
    {
        let head = selected.with_gix(|gix| {
            gix.head_name()
                .ok()
                .flatten()
                .map(|name| name.shorten().to_string())
        });
        return Some(refs.with_head_branch(head.as_deref()));
    }
    selected
        .with_cached_gix(|gix| RefSnapshot::observe(gix, false, wants_link_tips(options), true))
        .ok()
}

/// Whether two handles of one repository resolve branches, remote-tracking
/// refs, and tags identically. Anything unproven, including a failed path
/// probe, is `false`.
fn same_ref_view(a: &gix::Repository, b: &gix::Repository) -> bool {
    // Both sides are canonicalized the same way and compared only with each
    // other, so the Windows verbatim prefix cannot cause a mismatch.
    let (Ok(common_a), Ok(common_b)) = (
        crate::filesystem::canonicalize_observation_path(a.common_dir()),
        crate::filesystem::canonicalize_observation_path(b.common_dir()),
    ) else {
        return false;
    };
    common_a == common_b
        && a.namespace().map(|ns| ns.as_bstr()) == b.namespace().map(|ns| ns.as_bstr())
        && effective_replacements(a) == effective_replacements(b)
        && !may_shadow_shared_refs(a)
        && !may_shadow_shared_refs(b)
}

/// Object replacements (`refs/replace/`) this handle applies, in a stable
/// order; none when the handle ignores them.
fn effective_replacements(repo: &gix::Repository) -> Vec<(gix::ObjectId, gix::ObjectId)> {
    if repo.objects.ignore_replacements {
        return Vec::new();
    }
    let mut replacements: Vec<_> = repo.objects.store_ref().replacements().collect();
    replacements.sort();
    replacements
}

/// A linked worktree's private `refs` directory can overlay entries that
/// other worktrees read from the common directory.
fn may_shadow_shared_refs(repo: &gix::Repository) -> bool {
    if repo.git_dir() == repo.common_dir() {
        return false;
    }
    match std::fs::symlink_metadata(repo.git_dir().join("refs")) {
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
        Ok(_) => true,
    }
}

/// Links for `shown`, keyed by full commit ID. A snapshot without
/// remote-tracking tips cannot decide containment, so it links nothing.
fn link_displayed(
    repo: &gix::Repository,
    refs: &RefSnapshot,
    shown: &[CommitInfo],
) -> HashMap<String, CommitLink> {
    if shown.is_empty() || !refs.has_remote_branch_tips() {
        return HashMap::new();
    }
    let parsed: Vec<Option<gix::ObjectId>> = shown
        .iter()
        .map(|commit| gix::ObjectId::from_hex(commit.sha.as_bytes()).ok())
        .collect();
    let targets: Vec<gix::ObjectId> = parsed.iter().flatten().copied().collect();
    let timer = StageTimer::start("filesystem.git.commit_links");
    let mut links = link_commits_from_snapshot(repo, refs, &targets, COMMIT_VISIT_BUDGET).into_iter();
    timer.finish();
    shown
        .iter()
        .zip(parsed)
        .map(|(commit, id)| {
            let link = match id {
                Some(_) => links.next().unwrap_or_default(),
                None => CommitLink::default(),
            };
            (commit.sha.clone(), link)
        })
        .collect()
}

/// Keep only working-tree changes under `prefix` and recount them.
fn narrow_working_tree(git: &mut GitInfo, prefix: &str) {
    git.file_changes
        .retain(|change| change.path.to_string_lossy().starts_with(prefix));
    let Some(status) = git.status.as_mut() else {
        return;
    };
    status
        .dirty
        .retain(|file| file.filepath.to_string_lossy().starts_with(prefix));
    status
        .untracked
        .retain(|file| file.filepath.to_string_lossy().starts_with(prefix));
    let count = |wanted: &[FileStatus]| {
        git.file_changes
            .iter()
            .filter(|change| wanted.contains(&change.status))
            .count()
    };
    status.staged_count = count(&[FileStatus::Staged, FileStatus::Both]);
    status.unstaged_count = count(&[FileStatus::Modified, FileStatus::Both]);
    status.untracked_count = count(&[FileStatus::Untracked]);
    status.is_dirty = status.staged_count > 0
        || status.unstaged_count > 0
        || status.untracked_count > 0
        || git
            .file_changes
            .iter()
            .any(|change| change.status == FileStatus::Conflicted);
}

fn clear_working_tree(git: &mut GitInfo) {
    git.file_changes.clear();
    if let Some(status) = git.status.as_mut() {
        status.dirty.clear();
        status.untracked.clear();
        status.staged_count = 0;
        status.unstaged_count = 0;
        status.untracked_count = 0;
        status.is_dirty = false;
    }
}

#[cfg(test)]
#[path = "status_report_tests.rs"]
mod tests;
