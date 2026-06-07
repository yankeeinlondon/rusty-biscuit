//! Remote tracking, fetch, branch discovery, and worktree helpers.
//!
//! This module covers configured remotes, refresh of remote-tracking refs via
//! the user's `git` binary, locally cached remote branch lookups, local branch
//! enumeration, tracking status, git config, and linked worktree discovery.

use git2::Repository;
use gix::bstr::ByteSlice;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, warn};

use super::discovery::resolve_base_branch;
use super::status::get_repo_status_counts;
use super::types::*;

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
///
/// `ahead` is push-relevant: it counts commits reachable from the local
/// branch that are not reachable from **any** `refs/remotes/<remote>/*`
/// ref. This matches what `git push` would actually transmit, so a
/// merge-forward from `origin/main` into a feature branch is not
/// double-counted when `origin/<branch>` happens to be stale.
///
/// `behind` is the standard graph count: commits reachable from
/// `refs/remotes/<remote>/<branch>` that the local branch does not yet
/// have.
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

    let Ok(remotes) = repo.remotes() else {
        return tracking;
    };

    for remote_name in remotes.iter().flatten() {
        let remote_branch_name = format!("{}/{}", remote_name, branch_name);
        let Ok(remote_ref) = repo.find_reference(&format!("refs/remotes/{}", remote_branch_name))
        else {
            continue;
        };
        let Ok(remote_commit) = remote_ref.peel_to_commit() else {
            continue;
        };

        let behind = match repo.graph_ahead_behind(local_commit.id(), remote_commit.id()) {
            Ok((_, b)) => b,
            Err(e) => {
                debug!(remote = remote_name, error = %e, "could not compute behind count");
                continue;
            }
        };

        let ahead = match push_relevant_ahead(repo, local_commit.id(), remote_name) {
            Ok(n) => n,
            Err(e) => {
                debug!(remote = remote_name, error = %e, "could not compute push-relevant ahead");
                continue;
            }
        };

        tracking.push(RemoteTrackingStatus {
            remote: remote_name.to_string(),
            ahead,
            behind,
        });
    }

    tracking
}

/// Count commits reachable from `local` that are not reachable from any
/// `refs/remotes/<remote>/*` ref.
///
/// Mirrors what `git push <remote>` would have to transmit: commits the
/// remote does not already have on some branch. Avoids inflating the
/// count when the local branch has merged-forward commits that already
/// exist on `origin/main` (or other remote branches), even if
/// `origin/<branch>` itself is stale.
fn push_relevant_ahead(
    repo: &Repository,
    local: git2::Oid,
    remote_name: &str,
) -> Result<usize, git2::Error> {
    let mut walk = repo.revwalk()?;
    walk.push(local)?;

    let glob = format!("refs/remotes/{}/*", remote_name);
    let refs = repo.references_glob(&glob)?;
    for r in refs.flatten() {
        // `target()` returns None for symbolic refs (e.g. refs/remotes/origin/HEAD);
        // the underlying concrete ref is iterated separately, so skipping is safe.
        if let Some(oid) = r.target() {
            let _ = walk.hide(oid);
        }
    }

    Ok(walk.count())
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
/// merge-base tests (O(commits × branches) graph traversals), this
/// walks the ancestry from each remote tip once and builds a
/// `HashMap<ObjectId, Vec<remote>>`.  A `max_branches` limit can cap the
/// number of remote-tracking branches inspected.
///
/// Uses gix's commit-graph aware revwalk for timestamp-and-parent-only
/// gates, falling back to the object database when no graph is present.
pub(crate) fn populate_recent_commit_remotes(
    repo: &gix::Repository,
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
            gix::ObjectId::from_hex(c.sha.as_bytes())
                .ok()
                .and_then(|oid| repo.find_object(oid).ok())
                .and_then(|o| o.try_into_commit().ok())
                .and_then(|c| c.time().ok())
                .map(|t| t.seconds)
        })
        .min()
        .unwrap_or(i64::MIN);

    // Walk ancestry from each remote tip, collecting containment.
    let mut containment: HashMap<gix::ObjectId, Vec<String>> = HashMap::new();

    for (remote_name, tip_oid) in &remote_tips {
        let Ok(walk) = repo
            .rev_walk(Some(*tip_oid))
            .sorting(gix::revision::walk::Sorting::ByCommitTime(
                gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
            ))
            .all()
        else {
            continue;
        };

        for info_result in walk {
            let Ok(info) = info_result else {
                continue;
            };

            containment
                .entry(info.id)
                .or_default()
                .push(remote_name.clone());

            // Stop when we reach commits older than the oldest requested.
            // This is a safe heuristic: any commit older than the oldest
            // requested commit cannot be in the `commits` list.
            if info.commit_time() < oldest_time {
                break;
            }
        }
    }

    for commit in commits {
        let Ok(commit_oid) = gix::ObjectId::from_hex(commit.sha.as_bytes()) else {
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
fn remote_branch_tips(repo: &gix::Repository) -> Vec<(String, gix::ObjectId)> {
    let Ok(refs) = repo.references() else {
        return Vec::new();
    };

    let Ok(iter) = refs.prefixed("refs/remotes/") else {
        return Vec::new();
    };

    iter.flatten()
        .filter_map(|reference| {
            let name = reference.name().as_bstr().to_str_lossy().to_string();
            let branch = name.strip_prefix("refs/remotes/")?;
            if branch.ends_with("/HEAD") {
                return None;
            }

            let (remote_name, _) = branch.split_once('/')?;
            let target = reference
                .into_fully_peeled_id()
                .map_err(|e| {
                    debug!(branch = %name, error = %e, "could not peel remote ref to commit");
                    e
                })
                .ok()?;
            Some((remote_name.to_string(), target.detach()))
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

            let (dirty, changed_files) = gix::open(worktree_path)
                .ok()
                .map(|gix_repo| get_repo_status_counts(&gix_repo))
                .unwrap_or((false, 0));

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
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

    /// Creates refs/remotes/{remote}/{branch} pointing at `target` so the
    /// containment code sees them as remote-tracking branches.
    fn add_fake_remote(repo: &Repository, remote: &str, branch: &str, target: git2::Oid) {
        let ref_name = format!("refs/remotes/{remote}/{branch}");
        repo.reference(&ref_name, target, true, "test remote")
            .unwrap();
    }

    /// Open a gix handle for the same on-disk repo used by git2 fixture builders.
    fn open_gix(dir: &tempfile::TempDir) -> gix::Repository {
        gix::open(dir.path()).expect("open with gix")
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

        let gix_repo = open_gix(&dir);
        populate_recent_commit_remotes(&gix_repo, &mut commits, None);

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

        let gix_repo = open_gix(&dir);
        populate_recent_commit_remotes(&gix_repo, &mut commits, None);

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

        let gix_repo = open_gix(&dir);

        // With max_branches = 1, only the alphabetically first remote (fork)
        // should be checked.
        populate_recent_commit_remotes(&gix_repo, &mut commits, Some(1));
        assert_eq!(
            commits[0].remotes,
            Some(vec!["fork".to_string()]),
            "only fork should be checked with limit=1 (alphabetical order)"
        );

        // With max_branches = 2, fork and origin should be checked.
        commits[0].remotes = None;
        populate_recent_commit_remotes(&gix_repo, &mut commits, Some(2));
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
