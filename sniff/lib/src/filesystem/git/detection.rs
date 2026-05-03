use chrono::DateTime;
use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, instrument, warn};

use crate::Result;
use crate::request::GitRequest;

use super::types::*;

pub fn detect_git(path: &Path, deep: bool, commit_count: usize) -> Result<Option<GitInfo>> {
    match GitRepo::discover(path)? {
        Some(handle) => handle.detect_full(deep, commit_count).map(Some),
        None => Ok(None),
    }
}

/// Detect git information for a path according to the given request.
#[instrument(skip_all, fields(path = %path.display()))]
pub fn detect_git_with_request(path: &Path, request: &GitRequest) -> Result<Option<GitInfo>> {
    match GitRepo::discover(path)? {
        Some(git) => Ok(Some(git.detect_with_request(request)?)),
        None => Ok(None),
    }
}

/// Collects all refs (branches, remote tracking, tags) pointing to each commit.
///
/// Returns a HashMap from commit OID to a vector of ref decorations.
pub(crate) fn collect_ref_decorations(repo: &Repository) -> HashMap<git2::Oid, Vec<RefDecoration>> {
    let mut decorations: HashMap<git2::Oid, Vec<RefDecoration>> = HashMap::new();

    // Get current HEAD target to mark the active branch
    let head_target = repo
        .head()
        .map_err(|e| {
            debug!(error = %e, "could not read HEAD for decorations");
            e
        })
        .ok()
        .and_then(|h| {
            if h.is_branch() {
                h.shorthand().map(String::from)
            } else {
                None
            }
        });

    // Iterate all references
    let Ok(refs) = repo.references() else {
        return decorations;
    };

    for reference in refs.flatten() {
        let Some(name) = reference.name() else {
            continue;
        };

        // Resolve the reference to its target commit
        let Ok(target) = reference.peel_to_commit() else {
            continue;
        };
        let oid = target.id();

        // Determine ref kind and display name
        let (kind, display_name) = if let Some(branch) = name.strip_prefix("refs/heads/") {
            (RefKind::LocalBranch, branch.to_string())
        } else if let Some(remote) = name.strip_prefix("refs/remotes/") {
            (RefKind::RemoteBranch, remote.to_string())
        } else if let Some(tag) = name.strip_prefix("refs/tags/") {
            (RefKind::Tag, tag.to_string())
        } else {
            continue; // Skip other refs (notes, stash, etc.)
        };

        // Check if this is the HEAD branch
        let is_head = kind == RefKind::LocalBranch
            && head_target.as_ref().is_some_and(|h| h == &display_name);

        let decoration = RefDecoration {
            name: display_name,
            kind,
            is_head,
        };

        decorations.entry(oid).or_default().push(decoration);
    }

    // Sort decorations: HEAD branch first, then local branches, remote branches, tags
    for refs in decorations.values_mut() {
        refs.sort_by(|a, b| {
            // HEAD branch comes first
            if a.is_head != b.is_head {
                return b.is_head.cmp(&a.is_head);
            }
            // Then by kind: LocalBranch < RemoteBranch < Tag
            match (a.kind, b.kind) {
                (RefKind::LocalBranch, RefKind::LocalBranch) => a.name.cmp(&b.name),
                (RefKind::LocalBranch, _) => std::cmp::Ordering::Less,
                (_, RefKind::LocalBranch) => std::cmp::Ordering::Greater,
                (RefKind::RemoteBranch, RefKind::RemoteBranch) => a.name.cmp(&b.name),
                (RefKind::RemoteBranch, _) => std::cmp::Ordering::Less,
                (_, RefKind::RemoteBranch) => std::cmp::Ordering::Greater,
                (RefKind::Tag, RefKind::Tag) => a.name.cmp(&b.name),
            }
        });
    }

    decorations
}

/// Gets the last N commits from HEAD using revwalk.
pub(crate) fn get_recent_commits(repo: &Repository, count: usize) -> Vec<CommitInfo> {
    get_recent_commits_with_decorations(repo, count, None)
}

/// Gets the last N commits from HEAD using revwalk, with optional pre-computed ref decorations.
pub(crate) fn get_recent_commits_with_decorations(
    repo: &Repository,
    count: usize,
    ref_decorations: Option<&HashMap<git2::Oid, Vec<RefDecoration>>>,
) -> Vec<CommitInfo> {
    let mut commits = Vec::new();

    let Ok(mut revwalk) = repo.revwalk() else {
        return commits;
    };

    if revwalk.push_head().is_err() {
        return commits;
    }

    // Collect ref decorations once for all commits (if not provided)
    let cached = ref_decorations.cloned();
    let decorations = cached.unwrap_or_else(|| collect_ref_decorations(repo));

    for oid_result in revwalk.take(count) {
        let Ok(oid) = oid_result else {
            continue;
        };
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };

        // Get refs pointing to this commit
        let refs = decorations.get(&oid).cloned().unwrap_or_default();

        let author = commit.author();
        commits.push(CommitInfo {
            sha: commit.id().to_string(),
            message: commit.message().unwrap_or("").trim().to_string(),
            author: author.name().unwrap_or("Unknown").to_string(),
            timestamp: DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default(),
            remotes: None,
            refs,
        });
    }

    commits
}

/// Gathers repository status including staged, unstaged, and untracked changes.
/// Also returns file changes with their status for rich output.
///
/// Builds the staged and unstaged diffs once for the whole repository and
/// walks each diff a single time to accumulate per-file line stats and (when
/// `include_diffs` is true) per-file unified patch text. This avoids the
/// previous O(dirty_files * diff_setup_cost) scaling of issuing one
/// pathspec-restricted diff per dirty file.
///
/// When `include_diffs` is true, `RepoStatus.dirty` and `RepoStatus.untracked` are
/// populated with full unified diff payloads. When false, those fields are
/// empty `Vec`s and only the cheaper per-file stats (paths, status, line counts)
/// are computed.
pub(crate) fn get_repo_status_with_changes(
    repo: &Repository,
    include_diffs: bool,
) -> Result<(RepoStatus, Vec<FileChange>)> {
    use std::collections::HashSet;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    // Recurse into untracked directories to get individual file paths
    opts.recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;

    // Resolve HEAD tree once upfront so the staged diff can be built without
    // repeating tree resolution per file.
    let head_tree = repo.head().and_then(|h| h.peel_to_tree()).ok();

    // Build staged (HEAD -> index) and unstaged (index -> workdir) diffs once
    // for the whole repository, then walk each diff a single time to fill the
    // per-path accumulators below.
    let staged_diff = head_tree
        .as_ref()
        .and_then(|tree| repo.diff_tree_to_index(Some(tree), None, None).ok());
    let unstaged_diff = repo.diff_index_to_workdir(None, None).ok();

    let mut diff_stats: HashMap<PathBuf, LineStats> = HashMap::new();
    let mut staged_patches: HashMap<PathBuf, String> = HashMap::new();
    let mut unstaged_patches: HashMap<PathBuf, String> = HashMap::new();

    if let Some(diff) = staged_diff.as_ref() {
        let patch_sink = include_diffs.then_some(&mut staged_patches);
        aggregate_diff(diff, &mut diff_stats, patch_sink)?;
    }
    if let Some(diff) = unstaged_diff.as_ref() {
        let patch_sink = include_diffs.then_some(&mut unstaged_patches);
        aggregate_diff(diff, &mut diff_stats, patch_sink)?;
    }

    let mut staged = 0;
    let mut unstaged = 0;
    let mut untracked_count = 0;

    // Use HashSet for O(1) deduplication instead of Vec::contains which is O(n)
    let mut dirty_set: HashSet<PathBuf> = HashSet::new();
    let mut untracked_paths: Vec<PathBuf> = Vec::new();
    let mut file_changes: Vec<FileChange> = Vec::new();

    for entry in statuses.iter() {
        let status = entry.status();
        let path = entry.path().map(PathBuf::from);

        let is_staged =
            status.is_index_new() || status.is_index_modified() || status.is_index_deleted();
        let is_unstaged = status.is_wt_modified() || status.is_wt_deleted();
        let is_untracked = status.is_wt_new();

        // Determine the specific action for staged and unstaged changes
        let staged_action = if status.is_index_new() {
            Some(FileAction::Created)
        } else if status.is_index_deleted() {
            Some(FileAction::Deleted)
        } else if status.is_index_modified() {
            Some(FileAction::Modified)
        } else {
            None
        };

        let unstaged_action = if status.is_wt_deleted() {
            Some(FileAction::Deleted)
        } else if status.is_wt_modified() {
            Some(FileAction::Modified)
        } else {
            None
        };

        if is_staged {
            staged += 1;
        }
        if is_unstaged {
            unstaged += 1;
        }
        if is_untracked {
            untracked_count += 1;
            if let Some(ref p) = path {
                untracked_paths.push(p.clone());
                file_changes.push(FileChange {
                    path: p.clone(),
                    status: FileStatus::Untracked,
                    action: FileAction::Created,
                    lines_added: 0,
                    lines_removed: 0,
                });
            }
        }

        // Add to dirty set if staged or unstaged (but not untracked)
        if let Some(ref p) = path
            && !is_untracked
        {
            let LineStats {
                added: lines_added,
                removed: lines_removed,
            } = diff_stats.get(p).copied().unwrap_or_default();

            let (file_status, action) = if is_staged && is_unstaged {
                (
                    FileStatus::Both,
                    staged_action.unwrap_or(FileAction::Modified),
                )
            } else if is_staged {
                (
                    FileStatus::Staged,
                    staged_action.unwrap_or(FileAction::Modified),
                )
            } else if is_unstaged {
                (
                    FileStatus::Modified,
                    unstaged_action.unwrap_or(FileAction::Modified),
                )
            } else {
                continue;
            };

            file_changes.push(FileChange {
                path: p.clone(),
                status: file_status,
                action,
                lines_added,
                lines_removed,
            });
            dirty_set.insert(p.clone());
        }
    }

    let conflicted_paths = detect_merge_conflicts(repo);
    if !conflicted_paths.is_empty() {
        let conflicted_set: HashSet<_> = conflicted_paths.iter().cloned().collect();
        file_changes.retain(|change| !conflicted_set.contains(&change.path));

        let conflicted_changes = conflicted_paths.into_iter().map(|path| FileChange {
            path,
            status: FileStatus::Conflicted,
            action: FileAction::Modified,
            lines_added: 0,
            lines_removed: 0,
        });

        file_changes = conflicted_changes.chain(file_changes).collect();
    }

    // Convert HashSet to Vec for downstream processing
    let dirty_paths: Vec<PathBuf> = dirty_set.into_iter().collect();

    // Get HEAD commit SHA and upstream commit
    let (head_sha, origin_commit) = get_commit_refs(repo);

    // Get repository root for absolute paths
    let repo_root = repo.workdir().map(Path::to_path_buf);

    // Build dirty file details with diffs (only when requested). The patch
    // strings were captured in the single diff walk above, so this does not
    // re-run any libgit2 diff machinery.
    let dirty = if include_diffs {
        build_dirty_files_from_patches(
            &dirty_paths,
            &staged_patches,
            &unstaged_patches,
            &head_sha,
            &origin_commit,
            &repo_root,
        )
    } else {
        Vec::new()
    };

    // Build untracked file details (only when requested)
    let untracked = if include_diffs {
        build_untracked_files(&untracked_paths, &repo_root)
    } else {
        Vec::new()
    };

    let repo_status = RepoStatus {
        is_dirty: staged > 0
            || unstaged > 0
            || untracked_count > 0
            || file_changes
                .iter()
                .any(|change| change.status == FileStatus::Conflicted),
        staged_count: staged,
        unstaged_count: unstaged,
        untracked_count,
        dirty,
        untracked,
        is_behind: None, // Populated by detect_git when deep=true
    };

    Ok((repo_status, file_changes))
}

/// Per-file line stats accumulated from a single diff walk.
#[derive(Debug, Clone, Copy, Default)]
struct LineStats {
    added: usize,
    removed: usize,
}

/// Walk a `git2::Diff` exactly once, accumulating per-path line stats and
/// optionally per-path unified patch strings.
///
/// Lines are attributed to the new file path when present, falling back to
/// the old file path. This keeps deletes attributed to the deleted path and
/// renames (when rename detection is off, the default) attributed as
/// add/delete pairs against their respective paths — matching the prior
/// behavior of the per-file `pathspec`-restricted diff loop.
fn aggregate_diff(
    diff: &git2::Diff,
    stats: &mut HashMap<PathBuf, LineStats>,
    mut patches: Option<&mut HashMap<PathBuf, String>>,
) -> Result<()> {
    diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
        let Some(path) = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(PathBuf::from)
        else {
            return true;
        };

        match line.origin() {
            '+' => stats.entry(path.clone()).or_default().added += 1,
            '-' => stats.entry(path.clone()).or_default().removed += 1,
            _ => {}
        }

        if let Some(patches) = patches.as_deref_mut() {
            let entry = patches.entry(path).or_default();
            // Mirror the legacy diff_to_string format: prefix only content
            // lines, leave headers untouched.
            if matches!(line.origin(), '+' | '-' | ' ') {
                entry.push(line.origin());
            }
            if let Ok(content) = std::str::from_utf8(line.content()) {
                entry.push_str(content);
            }
        }
        true
    })?;
    Ok(())
}

/// Assemble per-file `DirtyFile` entries from the staged and unstaged patch
/// strings collected by [`aggregate_diff`].
fn build_dirty_files_from_patches(
    paths: &[PathBuf],
    staged_patches: &HashMap<PathBuf, String>,
    unstaged_patches: &HashMap<PathBuf, String>,
    head_sha: &str,
    origin_commit: &Option<String>,
    repo_root: &Option<PathBuf>,
) -> Vec<DirtyFile> {
    paths
        .iter()
        .map(|filepath| {
            let mut diff = String::new();
            if let Some(staged) = staged_patches.get(filepath)
                && !staged.is_empty()
            {
                diff.push_str(staged);
            }
            if let Some(unstaged) = unstaged_patches.get(filepath)
                && !unstaged.is_empty()
            {
                if !diff.is_empty() {
                    diff.push('\n');
                }
                diff.push_str(unstaged);
            }

            let absolute_filepath = repo_root
                .as_ref()
                .map(|root| root.join(filepath))
                .unwrap_or_else(|| filepath.clone());

            DirtyFile {
                filepath: filepath.clone(),
                absolute_filepath,
                diff,
                last_local_commit: head_sha.to_string(),
                origin_commit: origin_commit.clone(),
            }
        })
        .collect()
}

/// Gets HEAD commit SHA and upstream tracking branch commit SHA.
fn get_commit_refs(repo: &Repository) -> (String, Option<String>) {
    // Get HEAD commit SHA
    let head_sha = repo
        .head()
        .map_err(|e| {
            debug!(error = %e, "could not read HEAD for commit refs");
            e
        })
        .ok()
        .and_then(|h| {
            h.peel_to_commit()
                .map_err(|e| {
                    debug!(error = %e, "could not peel HEAD to commit");
                    e
                })
                .ok()
        })
        .map(|c| c.id().to_string())
        .unwrap_or_default();

    // Get upstream tracking branch commit using dynamic remote discovery
    let origin_commit = get_upstream_commit(repo);

    (head_sha, origin_commit)
}

/// Gets the upstream tracking branch commit SHA using dynamic remote discovery.
fn get_upstream_commit(repo: &Repository) -> Option<String> {
    let head = repo
        .head()
        .map_err(|e| {
            debug!(error = %e, "could not read HEAD for upstream commit");
            e
        })
        .ok()?;

    // Only works for branch references, not detached HEAD
    if !head.is_branch() {
        return None;
    }

    let branch_name = head.shorthand()?;
    let branch = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .map_err(|e| {
            debug!(branch = branch_name, error = %e, "could not find local branch");
            e
        })
        .ok()?;

    // Get the upstream branch (this handles dynamic remote discovery)
    let upstream = branch
        .upstream()
        .map_err(|e| {
            debug!(error = %e, "branch has no upstream");
            e
        })
        .ok()?;
    let upstream_commit = upstream
        .get()
        .peel_to_commit()
        .map_err(|e| {
            debug!(error = %e, "could not peel upstream to commit");
            e
        })
        .ok()?;

    Some(upstream_commit.id().to_string())
}

/// Builds detailed information for untracked files.
fn build_untracked_files(paths: &[PathBuf], repo_root: &Option<PathBuf>) -> Vec<UntrackedFile> {
    paths
        .iter()
        .map(|filepath| {
            let absolute_filepath = repo_root
                .as_ref()
                .map(|root| root.join(filepath))
                .unwrap_or_else(|| filepath.clone());

            UntrackedFile {
                filepath: filepath.clone(),
                absolute_filepath,
            }
        })
        .collect()
}

/// Gets git configuration (user info, GPG, signing).
pub(crate) fn get_git_config(repo: &Repository) -> GitConfig {
    #[allow(unused_mut)]
    let mut config = match repo.config() {
        Ok(c) => c,
        Err(_) => return GitConfig::default(),
    };

    // On macOS, the Developer Tools system gitconfig lives outside libgit2's
    // default search paths. Include it so we pick up credential.helper, etc.
    #[cfg(target_os = "macos")]
    {
        let macos_system = std::path::Path::new(
            "/Library/Developer/CommandLineTools/usr/share/git-core/gitconfig",
        );
        if macos_system.exists() {
            let _ = config.add_file(macos_system, git2::ConfigLevel::ProgramData, false);
        }
    }

    // Git for Windows installs a system-level gitconfig that libgit2 may not
    // find automatically via its ProgramData search.
    #[cfg(target_os = "windows")]
    {
        let git_for_windows = std::path::Path::new(r"C:\Program Files\Git\etc\gitconfig");
        if git_for_windows.exists() {
            let _ = config.add_file(git_for_windows, git2::ConfigLevel::ProgramData, false);
        }
    }

    GitConfig {
        user_name: config.get_string("user.name").ok(),
        user_email: config.get_string("user.email").ok(),
        gpg_use_agent: config.get_bool("gpg.use-agent").ok(),
        gpg_program: config.get_string("gpg.program").ok(),
        credential_helper: config.get_string("credential.helper").ok(),
        signing_key: config.get_string("user.signingkey").ok(),
        commit_sign: config.get_bool("commit.gpgsign").ok(),
        tag_sign: config.get_bool("tag.gpgsign").ok(),
        pager: config.get_string("core.pager").ok(),
        delta_syntax_theme: config.get_string("delta.syntax-theme").ok(),
        delta_light: config.get_bool("delta.light").ok(),
        delta_side_by_side: config.get_bool("delta.side-by-side").ok(),
    }
}

/// Gets all local branches with commit hashes and ahead/behind counts.
///
/// For each branch, resolves the tip commit's short hash and computes
/// ahead/behind relative to the current branch's HEAD. The current branch
/// itself gets ahead=0, behind=0.
pub(crate) fn get_local_branches(
    repo: &Repository,
    current_branch: Option<&str>,
) -> Vec<LocalBranchInfo> {
    let mut branches = Vec::new();

    // Resolve HEAD commit OID for ahead/behind calculations
    let head_oid = repo
        .head()
        .map_err(|e| {
            debug!(error = %e, "could not read HEAD for branch comparisons");
            e
        })
        .ok()
        .and_then(|h| {
            h.peel_to_commit()
                .map_err(|e| {
                    debug!(error = %e, "could not peel HEAD to commit for branch comparisons");
                    e
                })
                .ok()
        })
        .map(|c| c.id());

    if let Ok(branch_iter) = repo.branches(Some(git2::BranchType::Local)) {
        for branch_result in branch_iter {
            if let Ok((branch, _)) = branch_result
                && let Ok(Some(name)) = branch.name()
            {
                let is_current = current_branch.is_some_and(|cb| cb == name);

                // Get short hash from branch tip commit
                let short_hash = branch
                    .get()
                    .peel_to_commit()
                    .map_err(|e| {
                        debug!(branch = name, error = %e, "could not peel branch to commit");
                        e
                    })
                    .ok()
                    .map(|c| {
                        let id = c.id().to_string();
                        id[..8.min(id.len())].to_string()
                    })
                    .unwrap_or_default();

                // Compute ahead/behind relative to HEAD
                let (ahead, behind) = if is_current {
                    (0, 0)
                } else if let Some(head_id) = head_oid {
                    branch
                        .get()
                        .peel_to_commit()
                        .map_err(|e| {
                            debug!(branch = name, error = %e, "could not peel branch for ahead/behind");
                            e
                        })
                        .ok()
                        .and_then(|c| {
                            repo.graph_ahead_behind(c.id(), head_id)
                                .map_err(|e| {
                                    debug!(branch = name, error = %e, "could not compute ahead/behind");
                                    e
                                })
                                .ok()
                        })
                        .unwrap_or((0, 0))
                } else {
                    (0, 0)
                };

                branches.push(LocalBranchInfo {
                    name: name.to_string(),
                    short_hash,
                    ahead,
                    behind,
                });
            }
        }
    }

    branches
}

/// Gets tracking status (ahead/behind) for each remote.
pub(crate) fn get_tracking_status(
    repo: &Repository,
    current_branch: Option<&str>,
) -> Vec<RemoteTrackingStatus> {
    let mut tracking = Vec::new();

    let Some(branch_name) = current_branch else {
        return tracking;
    };

    let Ok(local_branch) = repo.find_branch(branch_name, git2::BranchType::Local) else {
        return tracking;
    };

    let Ok(local_commit) = local_branch.get().peel_to_commit() else {
        return tracking;
    };

    // Check each remote for a tracking branch
    if let Ok(remotes) = repo.remotes() {
        for remote_name in remotes.iter().flatten() {
            // Try to find the remote tracking branch (e.g., origin/main)
            let remote_branch_name = format!("{}/{}", remote_name, branch_name);
            if let Ok(remote_ref) =
                repo.find_reference(&format!("refs/remotes/{}", remote_branch_name))
                && let Ok(remote_commit) = remote_ref.peel_to_commit()
                && let Ok((ahead, behind)) =
                    repo.graph_ahead_behind(local_commit.id(), remote_commit.id())
            {
                tracking.push(RemoteTrackingStatus {
                    remote: remote_name.to_string(),
                    ahead,
                    behind,
                });
            }
        }
    }

    tracking
}

/// Retrieves all configured remotes with their URLs and hosting providers.
///
/// When `include_remote_details` is true, also includes locally known
/// remote-tracking branches and the resolved default branch.
pub(crate) fn get_remotes(repo: &Repository, include_remote_details: bool) -> Vec<RemoteInfo> {
    repo.remotes()
        .map(|names| {
            names
                .iter()
                .flatten()
                .filter_map(|name| {
                    repo.find_remote(name)
                        .map_err(|e| {
                            debug!(remote = name, error = %e, "could not find remote");
                            e
                        })
                        .ok()
                        .map(|remote| {
                            let url = remote.url().map(String::from);
                            let provider = url
                                .as_ref()
                                .map(|u| GitHostingProvider::from_url(u))
                                .unwrap_or(GitHostingProvider::Unknown);

                            let (branches, default_branch) = if include_remote_details {
                                (
                                    get_remote_branches(repo, name),
                                    get_remote_default_branch(repo, name),
                                )
                            } else {
                                (None, None)
                            };

                            RemoteInfo {
                                name: name.to_string(),
                                url,
                                provider,
                                branches,
                                default_branch,
                            }
                        })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Refresh local remote-tracking refs using the user's configured `git` binary.
///
/// When multiple remotes are configured, fetches run with bounded parallelism
/// (up to `max_concurrency`, clamped to 1–3) to reduce latency.  Terminal
/// prompts are disabled so CLI use does not block on credential input.
pub(crate) fn refresh_remote_tracking_refs(repo: &Repository, max_concurrency: usize) {
    let Some(repo_root) = repo.workdir() else {
        return;
    };

    let Ok(remotes) = repo.remotes() else {
        return;
    };

    let remote_names: Vec<String> = remotes.iter().flatten().map(|s| s.to_string()).collect();

    if remote_names.is_empty() {
        return;
    }

    // Serial path for a single remote — avoid threading overhead.
    if remote_names.len() == 1 {
        fetch_single_remote(repo_root, &remote_names[0]);
        return;
    }

    let max_concurrency = max_concurrency.clamp(1, 3);
    let repo_root = repo_root.to_path_buf();

    std::thread::scope(|s| {
        let mut handles = Vec::new();

        for chunk in remote_names.chunks(max_concurrency) {
            // Spawn up to max_concurrency fetches in parallel.
            for name in chunk {
                let name = name.clone();
                let repo_root = &repo_root;
                handles.push(s.spawn(move || {
                    fetch_single_remote(repo_root, &name);
                }));
            }
            // Wait for the current batch before starting the next.
            for handle in handles.drain(..) {
                if let Err(e) = handle.join() {
                    std::panic::resume_unwind(e);
                }
            }
        }
    });
}

/// Run `git fetch --quiet --prune <remote>` for a single remote.
fn fetch_single_remote(repo_root: &std::path::Path, remote_name: &str) {
    let _ = Command::new("git")
        .current_dir(repo_root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["fetch", "--quiet", "--prune", remote_name])
        .status()
        .map_err(|e| {
            warn!(remote = remote_name, error = %e, "git fetch failed");
            e
        });
}

/// Derive the user-facing behind status from per-remote tracking counts.
pub(crate) fn summarize_behind_status(tracking: &[RemoteTrackingStatus]) -> Option<BehindStatus> {
    if tracking.is_empty() {
        return None;
    }

    let mut behind_remotes: Vec<String> = tracking
        .iter()
        .filter(|status| status.behind > 0)
        .map(|status| status.remote.clone())
        .collect();
    behind_remotes.sort();
    behind_remotes.dedup();

    Some(if behind_remotes.is_empty() {
        BehindStatus::NotBehind
    } else {
        BehindStatus::Behind(behind_remotes)
    })
}

/// Populate commit containment data from locally available remote-tracking refs.
///
/// Instead of checking every commit against every remote tip with
/// `graph_descendant_of` (O(commits × branches) graph traversals), this
/// walks the ancestry from each remote tip once and builds a
/// `HashMap<Oid, Vec<remote>>`.  A `max_branches` limit can cap the number
/// of remote-tracking branches inspected.
pub(crate) fn populate_recent_commit_remotes(
    repo: &Repository,
    commits: &mut [CommitInfo],
    max_branches: Option<usize>,
) {
    let mut remote_tips = remote_branch_tips(repo);
    if remote_tips.is_empty() || commits.is_empty() {
        return;
    }

    // Sort for deterministic truncation, then apply branch limit if configured.
    remote_tips.sort_by(|a, b| a.0.cmp(&b.0));
    if let Some(limit) = max_branches {
        remote_tips.truncate(limit);
    }

    // Determine the oldest commit time among the requested commits so we
    // can stop ancestry walks early.
    let oldest_time = commits
        .iter()
        .filter_map(|c| {
            git2::Oid::from_str(&c.sha)
                .ok()
                .and_then(|oid| repo.find_commit(oid).ok())
                .map(|c| c.time().seconds())
        })
        .min()
        .unwrap_or(i64::MIN);

    // Walk ancestry from each remote tip, collecting containment.
    let mut containment: HashMap<git2::Oid, Vec<String>> = HashMap::new();

    for (remote_name, tip_oid) in &remote_tips {
        let Ok(mut revwalk) = repo.revwalk() else {
            continue;
        };
        if revwalk.push(*tip_oid).is_err() {
            continue;
        }

        for oid_result in revwalk {
            let Ok(oid) = oid_result else {
                continue;
            };

            containment
                .entry(oid)
                .or_default()
                .push(remote_name.clone());

            // Stop when we reach commits older than the oldest requested.
            // This is a safe heuristic: any commit older than the oldest
            // requested commit cannot be in the `commits` list.
            if let Ok(commit) = repo.find_commit(oid)
                && commit.time().seconds() < oldest_time
            {
                break;
            }
        }
    }

    for commit in commits {
        let Ok(commit_oid) = git2::Oid::from_str(&commit.sha) else {
            continue;
        };

        if let Some(remotes) = containment.get(&commit_oid) {
            let mut containing = remotes.clone();
            containing.sort();
            containing.dedup();
            if !containing.is_empty() {
                commit.remotes = Some(containing);
            }
        }
    }
}

/// Collect the tip OIDs for all remote-tracking branches keyed by remote name.
fn remote_branch_tips(repo: &Repository) -> Vec<(String, git2::Oid)> {
    let Ok(refs) = repo.references_glob("refs/remotes/*") else {
        return Vec::new();
    };

    refs.flatten()
        .filter_map(|reference| {
            let name = reference.name()?;
            let branch = name.strip_prefix("refs/remotes/")?;
            if branch.ends_with("/HEAD") {
                return None;
            }

            let (remote_name, _) = branch.split_once('/')?;
            let target = reference
                .peel_to_commit()
                .map_err(|e| {
                    debug!(branch = name, error = %e, "could not peel remote ref to commit");
                    e
                })
                .ok()?;
            Some((remote_name.to_string(), target.id()))
        })
        .collect()
}

/// Resolves the default branch for a remote from `refs/remotes/{name}/HEAD`.
///
/// Returns the branch name (e.g., "main") if the symbolic ref exists and can be resolved.
fn get_remote_default_branch(repo: &Repository, remote_name: &str) -> Option<String> {
    let ref_name = format!("refs/remotes/{}/HEAD", remote_name);
    let reference = repo
        .find_reference(&ref_name)
        .map_err(|e| {
            debug!(remote = remote_name, error = %e, "could not find remote HEAD reference");
            e
        })
        .ok()?;
    let target = reference.symbolic_target()?;
    let prefix = format!("refs/remotes/{}/", remote_name);
    target.strip_prefix(&prefix).map(String::from)
}

/// Gets branch names for a remote from local tracking refs (`refs/remotes/<name>/*`).
///
/// Reads locally cached remote branch info (updated on fetch/pull).
/// No network access required.
fn get_remote_branches(repo: &Repository, remote_name: &str) -> Option<Vec<String>> {
    let pattern = format!("refs/remotes/{}/*", remote_name);
    let refs = repo
        .references_glob(&pattern)
        .map_err(|e| {
            debug!(remote = remote_name, error = %e, "could not glob remote branches");
            e
        })
        .ok()?;
    let prefix = format!("refs/remotes/{}/", remote_name);

    let mut branches: Vec<String> = refs
        .flatten()
        .filter_map(|r| {
            let name = r.name()?;
            let branch = name.strip_prefix(&prefix)?;
            if branch == "HEAD" {
                None
            } else {
                Some(branch.to_string())
            }
        })
        .collect();

    branches.sort();

    if branches.is_empty() {
        None
    } else {
        Some(branches)
    }
}

/// Lightweight status check that only counts files by category.
///
/// Avoids the cost of per-file diff stat computation and unified diff
/// generation. Use this when you only need `is_dirty` and file counts.
pub(crate) fn get_repo_status_counts(repo: &Repository) -> (bool, usize) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(_) => return (false, 0),
    };

    let mut staged = 0usize;
    let mut unstaged = 0usize;
    let mut untracked = 0usize;

    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            staged += 1;
        }
        if status.is_wt_modified() || status.is_wt_deleted() {
            unstaged += 1;
        }
        if status.is_wt_new() {
            untracked += 1;
        }
    }

    let total = staged + unstaged + untracked;
    (total > 0, total)
}

/// Lightweight status check returning individual category counts.
pub(crate) fn get_repo_status_counts_detailed(repo: &Repository) -> (bool, usize, usize, usize) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(_) => return (false, 0, 0, 0),
    };

    let mut staged = 0usize;
    let mut unstaged = 0usize;
    let mut untracked = 0usize;

    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            staged += 1;
        }
        if status.is_wt_modified() || status.is_wt_deleted() {
            unstaged += 1;
        }
        if status.is_wt_new() {
            untracked += 1;
        }
    }

    let is_dirty = staged > 0 || unstaged > 0 || untracked > 0;
    (is_dirty, staged, unstaged, untracked)
}

/// Retrieves all linked worktrees for the repository.
///
/// Returns a HashMap keyed by branch name. Anonymous worktrees (without a name)
/// are filtered out. For each worktree, opens it as a Repository to access
/// HEAD commit, dirty status, and ahead/behind counts relative to the base
/// repository's default branch.
pub(crate) fn get_worktrees(repo: &Repository) -> HashMap<String, WorktreeInfo> {
    use rayon::prelude::*;

    let worktree_names = match repo.worktrees() {
        Ok(names) => names,
        Err(_) => return HashMap::new(),
    };

    // Resolve the base branch name and its commit OID for ahead/behind calculations.
    // Try the base repo's HEAD first; fall back to "main" then "master".
    let (base_branch, base_oid) = resolve_base_branch(repo);

    // Collect (name, path) pairs up front — cheap sequential work — so per-worktree
    // analysis can fan out in parallel. git2::Repository is !Sync, so each worker
    // opens its own handles rather than sharing `repo`.
    let base_repo_path = repo.path().to_path_buf();
    let worktree_paths: Vec<(String, PathBuf)> = worktree_names
        .iter()
        .flatten()
        .filter_map(|name| {
            let wt = repo.find_worktree(name).ok()?;
            Some((name.to_string(), wt.path().to_path_buf()))
        })
        .collect();

    // Open the base repo once before the parallel section to avoid N reopens.
    // We keep the opened handle alive for the scope of the parallel work.
    let _base_repo = Repository::open(&base_repo_path).ok();

    worktree_paths
        .par_iter()
        .filter_map(|(name, worktree_path)| {
            let worktree_repo = Repository::open(worktree_path).ok()?;

            // Get branch name and HEAD commit from worktree
            let head = worktree_repo.head().ok();
            let branch = head
                .as_ref()
                .and_then(|h| h.shorthand().map(String::from))
                .unwrap_or_else(|| name.to_string());
            let head_commit = head.and_then(|h| h.peel_to_commit().ok());
            let sha = head_commit
                .as_ref()
                .map(|c| c.id().to_string())
                .unwrap_or_default();

            // Open a per-thread handle on the base repo for graph/merge queries.
            // The outer _base_repo keeps the underlying git structures warm,
            // but git2::Repository is !Sync so each thread still needs its own handle.
            let base_repo = Repository::open(&base_repo_path).ok();

            let (ahead, behind) = base_repo
                .as_ref()
                .zip(base_oid)
                .zip(head_commit.as_ref())
                .and_then(|((base_repo, base), wt_commit)| {
                    base_repo.graph_ahead_behind(wt_commit.id(), base).ok()
                })
                .unwrap_or((0, 0));

            let merged = base_repo
                .as_ref()
                .zip(base_oid)
                .zip(head_commit.as_ref())
                .map(|((base_repo, base), wt_commit)| {
                    base_repo
                        .graph_descendant_of(base, wt_commit.id())
                        .unwrap_or(false)
                })
                .unwrap_or(false);

            let has_conflicts = base_repo
                .as_ref()
                .zip(base_oid)
                .zip(head_commit.as_ref())
                .and_then(|((base_repo, base_id), wt_commit)| {
                    let base_commit = base_repo.find_commit(base_id).ok()?;
                    let index = base_repo
                        .merge_commits(wt_commit, &base_commit, None)
                        .ok()?;
                    Some(index.has_conflicts())
                })
                .unwrap_or(false);

            let (dirty, changed_files) = get_repo_status_counts(&worktree_repo);

            Some((
                branch.clone(),
                WorktreeInfo {
                    branch,
                    filepath: worktree_path.clone(),
                    sha,
                    dirty,
                    ahead,
                    behind,
                    base_branch: base_branch.clone(),
                    has_conflicts,
                    merged,
                    changed_files,
                },
            ))
        })
        .collect()
}

/// Resolves the base branch name and its commit OID for ahead/behind calculations.
///
/// When the repo is a worktree, finds the base repo's current branch. Otherwise
/// uses the current HEAD branch. Falls back to "main" or "master" if HEAD is
/// detached or unavailable.
fn resolve_base_branch(repo: &Repository) -> (String, Option<git2::Oid>) {
    // If we're in a worktree, open the base repo to get its HEAD branch
    let base_repo = if repo.is_worktree() {
        repo.commondir().parent().and_then(|p| {
            Repository::open(p)
                .map_err(|e| {
                    debug!(error = %e, "could not open base repository");
                    e
                })
                .ok()
        })
    } else {
        None
    };
    let effective_repo = base_repo.as_ref().unwrap_or(repo);

    // Try the base repo's current HEAD branch
    if let Ok(head) = effective_repo.head()
        && let Some(name) = head.shorthand()
    {
        let oid = head
            .peel_to_commit()
            .map_err(|e| {
                debug!(error = %e, "could not peel base branch HEAD to commit");
                e
            })
            .ok()
            .map(|c| c.id());
        return (name.to_string(), oid);
    }

    // Fallback: try "main", then "master"
    for candidate in &["main", "master"] {
        let refname = format!("refs/heads/{candidate}");
        if let Ok(reference) = repo.find_reference(&refname) {
            let oid = reference
                .peel_to_commit()
                .map_err(|e| {
                    debug!(branch = candidate, error = %e, "could not peel fallback branch to commit");
                    e
                })
                .ok()
                .map(|c| c.id());
            return (candidate.to_string(), oid);
        }
    }

    ("main".to_string(), None)
}

/// Kind of change a file underwent in a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeltaKind {
    /// File was added.
    Added,
    /// File was modified.
    Modified,
    /// File was deleted.
    Deleted,
    /// File was renamed.
    Renamed,
    /// File was copied.
    Copied,
}

impl std::fmt::Display for DeltaKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Added => write!(f, "added"),
            Self::Modified => write!(f, "modified"),
            Self::Deleted => write!(f, "deleted"),
            Self::Renamed => write!(f, "renamed"),
            Self::Copied => write!(f, "copied"),
        }
    }
}

impl DeltaKind {
    /// Convert a git2 Delta status to a DeltaKind.
    fn from_delta(delta: git2::Delta) -> Self {
        match delta {
            git2::Delta::Added => Self::Added,
            git2::Delta::Deleted => Self::Deleted,
            git2::Delta::Renamed => Self::Renamed,
            git2::Delta::Copied => Self::Copied,
            _ => Self::Modified,
        }
    }
}

/// Look up a single commit by full or abbreviated SHA.
///
/// Uses `repo.revparse_single()` to resolve abbreviated or full SHA strings,
/// then peels to a commit and builds a `CommitInfo` with ref decorations.
///
/// Returns `None` if the SHA doesn't resolve to a valid commit.
pub fn get_commit_by_sha(repo: &Repository, sha_prefix: &str) -> Option<CommitInfo> {
    get_commit_by_sha_with_decorations(repo, sha_prefix, None)
}

/// Look up a single commit by SHA with optional pre-computed ref decorations.
pub(crate) fn get_commit_by_sha_with_decorations(
    repo: &Repository,
    sha_prefix: &str,
    ref_decorations: Option<&HashMap<git2::Oid, Vec<RefDecoration>>>,
) -> Option<CommitInfo> {
    let obj = repo
        .revparse_single(sha_prefix)
        .map_err(|e| {
            debug!(sha = sha_prefix, error = %e, "could not resolve SHA");
            e
        })
        .ok()?;
    let commit = obj
        .peel_to_commit()
        .map_err(|e| {
            debug!(sha = sha_prefix, error = %e, "could not peel object to commit");
            e
        })
        .ok()?;

    let decorations = ref_decorations
        .cloned()
        .unwrap_or_else(|| collect_ref_decorations(repo));
    let oid = commit.id();
    let refs = decorations.get(&oid).cloned().unwrap_or_default();

    let author = commit.author();
    Some(CommitInfo {
        sha: oid.to_string(),
        message: commit.message().unwrap_or("").trim().to_string(),
        author: author.name().unwrap_or("Unknown").to_string(),
        timestamp: DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default(),
        remotes: None,
        refs,
    })
}

/// Get the list of files changed by a specific commit.
///
/// Computes a diff between the commit's tree and its first parent's tree.
/// For the initial commit (no parent), diffs against an empty tree.
///
/// Returns a list of `(relative_path, DeltaKind)` pairs.
pub fn get_commit_files(repo: &Repository, full_sha: &str) -> Vec<(PathBuf, DeltaKind)> {
    let Ok(oid) = git2::Oid::from_str(full_sha) else {
        return Vec::new();
    };
    let Ok(commit) = repo.find_commit(oid) else {
        return Vec::new();
    };
    let Ok(tree) = commit.tree() else {
        return Vec::new();
    };

    let parent_tree = commit
        .parent(0)
        .map_err(|e| {
            debug!(sha = full_sha, error = %e, "could not get parent commit");
            e
        })
        .ok()
        .and_then(|p| {
            p.tree()
                .map_err(|e| {
                    debug!(sha = full_sha, error = %e, "could not get parent tree");
                    e
                })
                .ok()
        });
    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
        .map_err(|e| {
            debug!(sha = full_sha, error = %e, "could not create diff");
            e
        })
        .ok();

    let Some(diff) = diff else {
        return Vec::new();
    };

    diff.deltas()
        .filter_map(|delta| {
            let path = delta.new_file().path().unwrap_or(Path::new(""));
            if path.as_os_str().is_empty() {
                None
            } else {
                Some((path.to_path_buf(), DeltaKind::from_delta(delta.status())))
            }
        })
        .collect()
}

/// Get recent commits that touch files under a specific path prefix.
///
/// Walks the commit history from HEAD, computes diffs for each commit,
/// and includes commits where at least one changed file starts with `path_prefix`.
///
/// Ref decorations are collected once and reused for all matching commits.
pub fn get_commits_for_path(repo: &Repository, path_prefix: &str, count: usize) -> Vec<CommitInfo> {
    get_commits_for_path_with_decorations(repo, path_prefix, count, None)
}

/// Get recent commits for a path with optional pre-computed ref decorations.
pub(crate) fn get_commits_for_path_with_decorations(
    repo: &Repository,
    path_prefix: &str,
    count: usize,
    ref_decorations: Option<&HashMap<git2::Oid, Vec<RefDecoration>>>,
) -> Vec<CommitInfo> {
    let mut commits = Vec::new();

    let Ok(mut revwalk) = repo.revwalk() else {
        return commits;
    };
    if revwalk.push_head().is_err() {
        return commits;
    }

    let cached = ref_decorations.cloned();
    let decorations = cached.unwrap_or_else(|| collect_ref_decorations(repo));

    for oid_result in revwalk {
        if commits.len() >= count {
            break;
        }
        let Ok(oid) = oid_result else {
            continue;
        };
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };
        let Ok(tree) = commit.tree() else {
            continue;
        };

        let parent_tree = commit
            .parent(0)
            .map_err(|e| {
                debug!(sha = %oid, error = %e, "could not get parent for path commit");
                e
            })
            .ok()
            .and_then(|p| {
                p.tree()
                    .map_err(|e| {
                        debug!(sha = %oid, error = %e, "could not get parent tree for path commit");
                        e
                    })
                    .ok()
            });
        let Ok(diff) = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) else {
            continue;
        };

        let touches_path = diff.deltas().any(|delta| {
            let new_path = delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let old_path = delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            new_path.starts_with(path_prefix) || old_path.starts_with(path_prefix)
        });

        if touches_path {
            let refs = decorations.get(&oid).cloned().unwrap_or_default();
            let author = commit.author();
            commits.push(CommitInfo {
                sha: oid.to_string(),
                message: commit.message().unwrap_or("").trim().to_string(),
                author: author.name().unwrap_or("Unknown").to_string(),
                timestamp: DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default(),
                remotes: None,
                refs,
            });
        }
    }

    commits
}

/// Detect unmerged (conflicted) files in the repository index.
///
/// Returns the relative paths of files that have merge conflict markers
/// in the index (i.e., are in an unmerged state from a merge, rebase,
/// cherry-pick, or revert).
pub fn detect_merge_conflicts(repo: &Repository) -> Vec<PathBuf> {
    let index = match repo.index() {
        Ok(idx) => idx,
        Err(_) => return Vec::new(),
    };

    let conflicts = match index.conflicts() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut conflicted = Vec::new();
    for entry in conflicts {
        let Ok(conflict) = entry else {
            continue;
        };
        let path = conflict
            .our
            .as_ref()
            .or(conflict.their.as_ref())
            .or(conflict.ancestor.as_ref());
        if let Some(entry) = path {
            let path = PathBuf::from(std::str::from_utf8(&entry.path).unwrap_or_default());
            if !conflicted.contains(&path) {
                conflicted.push(path);
            }
        }
    }

    conflicted
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Creates a temporary git repo with a single file committed.
    fn setup_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        // Create initial file and commit
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "initial content\n").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    /// Stage a path (`git add <path>`).
    fn stage_path(repo: &Repository, relative: &str) {
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(relative)).unwrap();
        index.write().unwrap();
    }

    /// Stage a deletion (`git rm <path>` equivalent at the index level).
    fn stage_delete(repo: &Repository, relative: &str) {
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new(relative)).unwrap();
        index.write().unwrap();
    }

    fn find_change<'a>(changes: &'a [FileChange], path: &str) -> &'a FileChange {
        changes
            .iter()
            .find(|c| c.path == Path::new(path))
            .unwrap_or_else(|| panic!("expected change for {}", path))
    }

    #[test]
    fn batched_diff_attributes_lines_to_unstaged_only_files() {
        let (dir, repo) = setup_repo();

        // Modify the file (unstaged change only)
        std::fs::write(dir.path().join("test.txt"), "modified content\n").unwrap();

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.unstaged_count, 1);
        assert_eq!(status.staged_count, 0);
        let change = find_change(&changes, "test.txt");
        assert_eq!(change.status, FileStatus::Modified);
        assert_eq!(change.lines_added, 1);
        assert_eq!(change.lines_removed, 1);
    }

    #[test]
    fn batched_diff_sums_staged_and_unstaged_for_combined_changes() {
        let (dir, repo) = setup_repo();

        // Stage one modification, then add additional unstaged edits.
        std::fs::write(dir.path().join("test.txt"), "staged content\n").unwrap();
        stage_path(&repo, "test.txt");
        std::fs::write(
            dir.path().join("test.txt"),
            "unstaged content\nmore lines\n",
        )
        .unwrap();

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 1);
        assert_eq!(status.unstaged_count, 1);

        let change = find_change(&changes, "test.txt");
        assert_eq!(change.status, FileStatus::Both);
        // Staged: 1 add / 1 remove. Unstaged (index→workdir): 2 add / 1 remove.
        // Combined totals must match the legacy per-file path.
        assert_eq!(change.lines_added, 3);
        assert_eq!(change.lines_removed, 2);
    }

    #[test]
    fn batched_diff_handles_staged_deletes() {
        let (dir, repo) = setup_repo();

        // Stage deletion of the committed file.
        std::fs::remove_file(dir.path().join("test.txt")).unwrap();
        stage_delete(&repo, "test.txt");

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 1);
        assert_eq!(status.unstaged_count, 0);

        let change = find_change(&changes, "test.txt");
        assert_eq!(change.status, FileStatus::Staged);
        assert_eq!(change.action, FileAction::Deleted);
        // The deleted file had a single line, so the diff records exactly one removal.
        assert_eq!(change.lines_added, 0);
        assert_eq!(change.lines_removed, 1);
    }

    #[test]
    fn batched_diff_handles_unstaged_deletes_with_concurrent_modify() {
        let (dir, repo) = setup_repo();

        // Add a second file and commit so we can mix delete + modify cases.
        std::fs::write(dir.path().join("other.txt"), "alpha\nbeta\n").unwrap();
        stage_path(&repo, "other.txt");
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let parent = repo.head().unwrap().peel_to_commit().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&parent])
                .unwrap();
        }

        // Unstaged delete of `test.txt` and unstaged modify of `other.txt`.
        std::fs::remove_file(dir.path().join("test.txt")).unwrap();
        std::fs::write(dir.path().join("other.txt"), "alpha\ngamma\n").unwrap();

        let (status, changes) = get_repo_status_with_changes(&repo, true).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.unstaged_count, 2);

        let deleted = find_change(&changes, "test.txt");
        assert_eq!(deleted.action, FileAction::Deleted);
        assert!(deleted.lines_removed >= 1);

        let modified = find_change(&changes, "other.txt");
        assert_eq!(modified.action, FileAction::Modified);
        assert_eq!(modified.lines_added, 1);
        assert_eq!(modified.lines_removed, 1);

        // include_diffs=true must emit per-file unified patches assembled from
        // the same batched diff walk.
        let dirty_test = status
            .dirty
            .iter()
            .find(|d| d.filepath == Path::new("test.txt"))
            .expect("dirty entry for test.txt");
        assert!(dirty_test.diff.contains("-initial content"));

        let dirty_other = status
            .dirty
            .iter()
            .find(|d| d.filepath == Path::new("other.txt"))
            .expect("dirty entry for other.txt");
        assert!(dirty_other.diff.contains("-beta"));
        assert!(dirty_other.diff.contains("+gamma"));
    }

    #[test]
    fn get_repo_status_with_changes_resolves_head_once() {
        let (dir, repo) = setup_repo();

        // Create and stage multiple files
        for i in 0..3 {
            let name = format!("file{}.txt", i);
            let path = dir.path().join(&name);
            std::fs::write(&path, format!("content {}\n", i)).unwrap();
            stage_path(&repo, &name);
        }

        // This should work without error and resolve HEAD only once
        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 3);
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn batched_diff_handles_staged_rename_as_delete_and_add() {
        let (dir, repo) = setup_repo();

        // Rename the committed file on disk and stage the change.
        let old_path = dir.path().join("test.txt");
        let new_path = dir.path().join("renamed.txt");
        std::fs::rename(&old_path, &new_path).unwrap();

        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("test.txt")).unwrap();
        index.add_path(Path::new("renamed.txt")).unwrap();
        index.write().unwrap();

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 2);
        // One delete + one add because rename detection is off by default.
        assert_eq!(changes.len(), 2);

        let deleted = find_change(&changes, "test.txt");
        assert_eq!(deleted.status, FileStatus::Staged);
        assert_eq!(deleted.action, FileAction::Deleted);
        assert_eq!(deleted.lines_removed, 1);
        assert_eq!(deleted.lines_added, 0);

        let created = find_change(&changes, "renamed.txt");
        assert_eq!(created.status, FileStatus::Staged);
        assert_eq!(created.action, FileAction::Created);
        // The added side of the rename carries the original line content.
        assert_eq!(created.lines_added, 1);
        assert_eq!(created.lines_removed, 0);
    }

    #[test]
    fn batched_diff_mixed_binary_and_text_deltas() {
        let (dir, repo) = setup_repo();

        // Add a binary file and a second text file, then commit.
        let binary_path = dir.path().join("data.bin");
        let text_path = dir.path().join("other.txt");
        std::fs::write(&binary_path, b"\x00\x01\x02\x03\n").unwrap();
        std::fs::write(&text_path, "alpha\nbeta\n").unwrap();
        stage_path(&repo, "data.bin");
        stage_path(&repo, "other.txt");
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let parent = repo.head().unwrap().peel_to_commit().unwrap();
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "add binary and text",
                &tree,
                &[&parent],
            )
            .unwrap();
        }

        // Modify both files in the working tree (unstaged).
        std::fs::write(&binary_path, b"\x00\x01\x02\xFF\n").unwrap();
        std::fs::write(&text_path, "alpha\ngamma\n").unwrap();

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.unstaged_count, 2);

        let text_change = find_change(&changes, "other.txt");
        assert_eq!(text_change.status, FileStatus::Modified);
        assert_eq!(text_change.action, FileAction::Modified);
        // text delta: 1 add, 1 remove
        assert_eq!(text_change.lines_added, 1);
        assert_eq!(text_change.lines_removed, 1);

        let binary_change = find_change(&changes, "data.bin");
        assert_eq!(binary_change.status, FileStatus::Modified);
        assert_eq!(binary_change.action, FileAction::Modified);
        // Binary files produce no countable text lines.
        assert_eq!(binary_change.lines_added, 0);
        assert_eq!(binary_change.lines_removed, 0);
    }

    // ── populate_recent_commit_remotes tests ───────────────────────────────

    /// Creates refs/remotes/{remote}/{branch} pointing at `target` so the
    /// containment code sees them as remote-tracking branches.
    fn add_fake_remote(repo: &Repository, remote: &str, branch: &str, target: git2::Oid) {
        let ref_name = format!("refs/remotes/{remote}/{branch}");
        repo.reference(&ref_name, target, true, "test remote")
            .unwrap();
    }

    #[test]
    fn populate_commit_remotes_finds_single_remote() {
        let (dir, repo) = setup_repo();

        // Create a second commit on HEAD.
        std::fs::write(dir.path().join("test.txt"), "second\n").unwrap();
        stage_path(&repo, "test.txt");
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let parent = repo.head().unwrap().peel_to_commit().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&parent])
                .unwrap();
        }

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let head_id = head.id();

        // Fake a remote that points at HEAD.
        add_fake_remote(&repo, "origin", "main", head_id);

        let mut commits = vec![CommitInfo {
            sha: head_id.to_string(),
            message: "second".to_string(),
            author: "Test".to_string(),
            timestamp: chrono::Utc::now(),
            remotes: None,
            refs: vec![],
        }];

        populate_recent_commit_remotes(&repo, &mut commits, None);

        assert_eq!(
            commits[0].remotes,
            Some(vec!["origin".to_string()]),
            "HEAD should be contained by origin"
        );
    }

    #[test]
    fn populate_commit_remotes_distinguishes_multiple_remotes() {
        let (dir, repo) = setup_repo();

        // second commit
        std::fs::write(dir.path().join("test.txt"), "second\n").unwrap();
        stage_path(&repo, "test.txt");
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let parent = repo.head().unwrap().peel_to_commit().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&parent])
                .unwrap();
        }

        // third commit
        std::fs::write(dir.path().join("test.txt"), "third\n").unwrap();
        stage_path(&repo, "test.txt");
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let parent = repo.head().unwrap().peel_to_commit().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "third", &tree, &[&parent])
                .unwrap();
        }

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let head_id = head.id();
        let second_id = head.parent(0).unwrap().id();

        // origin points at HEAD (contains both commits).
        add_fake_remote(&repo, "origin", "main", head_id);
        // upstream points at second commit (contains only that one).
        add_fake_remote(&repo, "upstream", "main", second_id);

        let mut commits = vec![
            CommitInfo {
                sha: head_id.to_string(),
                message: "third".to_string(),
                author: "Test".to_string(),
                timestamp: chrono::Utc::now(),
                remotes: None,
                refs: vec![],
            },
            CommitInfo {
                sha: second_id.to_string(),
                message: "second".to_string(),
                author: "Test".to_string(),
                timestamp: chrono::Utc::now(),
                remotes: None,
                refs: vec![],
            },
        ];

        populate_recent_commit_remotes(&repo, &mut commits, None);

        assert_eq!(
            commits[0].remotes.as_deref(),
            Some(["origin".to_string()].as_slice()),
            "third commit only on origin"
        );
        assert_eq!(
            commits[1].remotes.as_deref(),
            Some(["origin".to_string(), "upstream".to_string()].as_slice()),
            "second commit on both origin and upstream"
        );
    }

    #[test]
    fn populate_commit_remotes_respects_max_branches() {
        let (dir, repo) = setup_repo();

        // Create a second commit.
        std::fs::write(dir.path().join("test.txt"), "second\n").unwrap();
        stage_path(&repo, "test.txt");
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let parent = repo.head().unwrap().peel_to_commit().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&parent])
                .unwrap();
        }

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let head_id = head.id();

        // Add three fake remotes all pointing at HEAD.
        add_fake_remote(&repo, "origin", "main", head_id);
        add_fake_remote(&repo, "upstream", "main", head_id);
        add_fake_remote(&repo, "fork", "main", head_id);

        let mut commits = vec![CommitInfo {
            sha: head_id.to_string(),
            message: "second".to_string(),
            author: "Test".to_string(),
            timestamp: chrono::Utc::now(),
            remotes: None,
            refs: vec![],
        }];

        // With max_branches = 1, only the alphabetically first remote (fork)
        // should be checked.
        populate_recent_commit_remotes(&repo, &mut commits, Some(1));
        assert_eq!(
            commits[0].remotes,
            Some(vec!["fork".to_string()]),
            "only fork should be checked with limit=1 (alphabetical order)"
        );

        // With max_branches = 2, fork and origin should be checked.
        commits[0].remotes = None;
        populate_recent_commit_remotes(&repo, &mut commits, Some(2));
        assert_eq!(
            commits[0].remotes,
            Some(vec!["fork".to_string(), "origin".to_string()]),
            "fork and origin should be checked with limit=2"
        );
    }

    #[test]
    fn refresh_remote_tracking_refs_single_remote_runs_without_panic() {
        let (_dir, repo) = setup_repo();
        // With a single remote, the serial path should execute without error.
        refresh_remote_tracking_refs(&repo, 2);
        // The function has no return value; absence of panic is the test.
    }

    #[test]
    fn refresh_remote_tracking_refs_zero_concurrency_uses_one() {
        let (_dir, repo) = setup_repo();
        // Concurrency of 0 should be clamped to 1.
        refresh_remote_tracking_refs(&repo, 0);
    }
}
