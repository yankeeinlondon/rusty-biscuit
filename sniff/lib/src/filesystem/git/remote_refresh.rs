//! Remote tracking, fetch, branch discovery, and worktree helpers.
//!
//! This module covers configured remotes, refresh of remote-tracking refs via
//! the user's `git` binary, locally cached remote branch lookups, local branch
//! enumeration, tracking status, git config, and linked worktree discovery.

use gix::bstr::ByteSlice;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tracing::warn;

use super::discovery::resolve_base_branch;
use super::status::get_repo_status_counts;
use super::types::*;
use crate::SniffError;
use crate::performance::{self, counters};
use crate::process::{CapturedOutput, ProcessError, run_command_with_timeout, timeouts};

#[derive(Debug, Clone)]
struct ObservedBranch {
    name: String,
    tip: gix::ObjectId,
}

#[derive(Debug, Clone)]
struct ObservedRemoteBranch {
    remote: String,
    branch: String,
    tip: gix::ObjectId,
}

/// One request-scoped observation of the repository's ref store.
///
/// Consumers project branches, decorations, remote tips, and tracking status
/// from the same peeled refs. This is deliberately request-local: refs may
/// change between requests, and a process-global cache would make freshness
/// semantics surprising.
#[derive(Debug, Clone, Default)]
pub(crate) struct RefSnapshot {
    local_branches: Vec<ObservedBranch>,
    remote_branches: Vec<ObservedRemoteBranch>,
    remote_defaults: HashMap<String, String>,
    decorations: HashMap<gix::ObjectId, Vec<RefDecoration>>,
}

impl RefSnapshot {
    pub(crate) fn observe(
        repo: &gix::Repository,
        local_branches: bool,
        remote_branches: bool,
        decorations: bool,
    ) -> crate::Result<Self> {
        let head_target = repo
            .head_name()
            .ok()
            .flatten()
            .map(|name| name.shorten().to_string());

        performance::increment_counter(counters::GIT_REF_WALKS, 1);
        let platform = repo
            .references()
            .map_err(|e| SniffError::git("references", e))?;
        let iter = platform
            .all()
            .map_err(|e| SniffError::git("references", e))?;

        let mut snapshot = Self::default();
        for reference in iter {
            let reference = reference.map_err(|e| SniffError::git("references", e))?;
            let full_name = reference.name().as_bstr().to_string();
            let is_local = full_name.starts_with("refs/heads/");
            let is_remote = full_name.starts_with("refs/remotes/");
            let is_tag = full_name.starts_with("refs/tags/");
            if !((local_branches && is_local)
                || (remote_branches && is_remote)
                || (decorations && (is_local || is_remote || is_tag)))
            {
                continue;
            }
            let symbolic_target = match reference.target() {
                gix::refs::TargetRef::Symbolic(target) => Some(target.as_bstr().to_string()),
                gix::refs::TargetRef::Object(_) => None,
            };
            let tip = reference
                .into_fully_peeled_id()
                .map_err(|e| SniffError::git("peel", e))?
                .detach();

            let (kind, display_name) = if let Some(branch) = full_name.strip_prefix("refs/heads/") {
                if local_branches {
                    snapshot.local_branches.push(ObservedBranch {
                        name: branch.to_string(),
                        tip,
                    });
                }
                (RefKind::LocalBranch, branch.to_string())
            } else if let Some(remote_branch) = full_name.strip_prefix("refs/remotes/") {
                let Some((remote, branch)) = remote_branch.split_once('/') else {
                    continue;
                };
                if branch == "HEAD" {
                    if let Some(target) = symbolic_target {
                        let prefix = format!("refs/remotes/{remote}/");
                        if let Some(default) = target.strip_prefix(&prefix) {
                            snapshot
                                .remote_defaults
                                .insert(remote.to_string(), default.to_string());
                        }
                    }
                } else if remote_branches {
                    snapshot.remote_branches.push(ObservedRemoteBranch {
                        remote: remote.to_string(),
                        branch: branch.to_string(),
                        tip,
                    });
                }
                (RefKind::RemoteBranch, remote_branch.to_string())
            } else if let Some(tag) = full_name.strip_prefix("refs/tags/") {
                (RefKind::Tag, tag.to_string())
            } else {
                continue;
            };

            if decorations {
                snapshot.decorations.entry(tip).or_default().push(RefDecoration {
                    is_head: kind == RefKind::LocalBranch
                        && head_target.as_ref().is_some_and(|head| head == &display_name),
                    name: display_name,
                    kind,
                });
            }
        }

        snapshot.local_branches.sort_by(|a, b| a.name.cmp(&b.name));
        snapshot.remote_branches.sort_by(|a, b| {
            a.remote
                .cmp(&b.remote)
                .then_with(|| a.branch.cmp(&b.branch))
        });
        for decorations in snapshot.decorations.values_mut() {
            decorations.sort_by(|a, b| {
                b.is_head
                    .cmp(&a.is_head)
                    .then_with(|| ref_kind_order(a.kind).cmp(&ref_kind_order(b.kind)))
                    .then_with(|| a.name.cmp(&b.name))
            });
        }
        Ok(snapshot)
    }

    pub(crate) fn decorations(&self) -> &HashMap<gix::ObjectId, Vec<RefDecoration>> {
        &self.decorations
    }

    fn local_tip(&self, branch: &str) -> Option<gix::ObjectId> {
        self.local_branches
            .iter()
            .find(|observed| observed.name == branch)
            .map(|observed| observed.tip)
    }

    fn remote_tip(&self, remote: &str, branch: &str) -> Option<gix::ObjectId> {
        self.remote_branches
            .iter()
            .find(|observed| observed.remote == remote && observed.branch == branch)
            .map(|observed| observed.tip)
    }

    fn remote_tips(&self, remote: &str) -> Vec<gix::ObjectId> {
        self.remote_branches
            .iter()
            .filter(|observed| observed.remote == remote)
            .map(|observed| observed.tip)
            .collect()
    }
}

fn ref_kind_order(kind: RefKind) -> u8 {
    match kind {
        RefKind::LocalBranch => 0,
        RefKind::RemoteBranch => 1,
        RefKind::Tag => 2,
    }
}

/// gix equivalent of git2's `graph_ahead_behind`: `(commits reachable from
/// `local` but not `upstream`, commits reachable from `upstream` but not
/// `local`)`, computed with two hidden-tip revwalks.
fn ahead_behind(
    repo: &gix::Repository,
    local: gix::ObjectId,
    upstream: gix::ObjectId,
) -> crate::Result<(usize, usize)> {
    Ok((
        count_reachable_excluding(repo, local, upstream)?,
        count_reachable_excluding(repo, upstream, local)?,
    ))
}

pub(crate) fn get_branch_info_fallible(
    repo: &gix::Repository,
    current_branch: Option<&str>,
) -> crate::Result<Vec<BranchInfo>> {
    let refs = RefSnapshot::observe(repo, true, true, false)?;
    get_branch_info_from_snapshot(repo, current_branch, &refs)
}

pub(crate) fn get_branch_info_from_snapshot(
    repo: &gix::Repository,
    current_branch: Option<&str>,
    refs: &RefSnapshot,
) -> crate::Result<Vec<BranchInfo>> {
    let config = repo.config_snapshot();
    let mut branches = Vec::new();

    for observed in &refs.local_branches {
        let name = observed.name.clone();
        let tip = observed.tip;
        let sha = tip.to_string();
        let upstream = configured_upstream(&config, &name);
        // ahead/behind are only meaningful against a known upstream tip. With no
        // configured upstream (or one whose tip is not locally known), report
        // `None` so a fresh branch is distinguishable from one that is exactly
        // even with its upstream.
        let (ahead, behind) = match upstream
            .as_deref()
            .and_then(|upstream| upstream.split_once('/'))
            .and_then(|(remote, branch)| refs.remote_tip(remote, branch))
        {
            Some(remote) => {
                let (ahead, behind) = ahead_behind(repo, tip, remote)?;
                (Some(ahead), Some(behind))
            }
            None => (None, None),
        };

        branches.push(BranchInfo {
            current: current_branch.is_some_and(|current| current == name),
            remote_represented: refs
                .remote_branches
                .iter()
                .any(|remote| remote.tip == tip),
            name,
            sha,
            upstream,
            ahead,
            behind,
        });
    }

    branches.sort_by(|a, b| b.current.cmp(&a.current).then_with(|| a.name.cmp(&b.name)));
    Ok(branches)
}

fn configured_upstream(config: &gix::config::File<'_>, branch: &str) -> Option<String> {
    let remote = config
        .string(format!("branch.{branch}.remote").as_str())
        .map(|value| value.to_string())?;
    let merge = config
        .string(format!("branch.{branch}.merge").as_str())
        .map(|value| value.to_string())?;
    let upstream_branch = merge.strip_prefix("refs/heads/").unwrap_or(&merge);
    Some(format!("{remote}/{upstream_branch}"))
}

/// Count commits reachable from `tip` once everything reachable from `hide` is
/// excluded — the building block for ahead/behind.
fn count_reachable_excluding(
    repo: &gix::Repository,
    tip: gix::ObjectId,
    hide: gix::ObjectId,
) -> crate::Result<usize> {
    let walk = repo
        .rev_walk(Some(tip))
        .with_hidden(Some(hide))
        .use_commit_graph(Some(true))
        .all()
        .map_err(|e| SniffError::git("revwalk", e))?;
    let mut count = 0;
    for item in walk {
        item.map_err(|e| SniffError::git("revwalk", e))?;
        performance::increment_counter(counters::GIT_COMMIT_VISITS, 1);
        count += 1;
    }
    Ok(count)
}

/// True when `ancestor` is reachable from `descendant` (i.e. already merged):
/// their merge base is `ancestor` itself.
fn is_ancestor(
    repo: &gix::Repository,
    ancestor: gix::ObjectId,
    descendant: gix::ObjectId,
) -> crate::Result<bool> {
    repo.merge_base(ancestor, descendant)
        .map(|base| base.detach() == ancestor)
        .map_err(|e| SniffError::git("merge_base", e))
}

/// Gets git configuration (user info, GPG, signing).
///
/// Reads from gix's layered snapshot (system/global/local). gix does not search
/// the platform-specific system gitconfig that ships with Apple's Command Line
/// Tools (macOS) or Git for Windows, so that file is parsed separately and used
/// as a lowest-precedence (ProgramData-equivalent) fallback — matching the
/// extra file libgit2 was given.
pub(crate) fn get_git_config(repo: &gix::Repository) -> GitConfig {
    get_git_config_with_extra(repo, extra_system_config())
}

/// Like [`get_git_config`], but accepts an injectable extra-system fallback so
/// tests can verify all keys through the fallback path without depending on the
/// production platform config file.
fn get_git_config_with_extra(
    repo: &gix::Repository,
    extra: Option<gix::config::File<'static>>,
) -> GitConfig {
    let snapshot = repo.config_snapshot();

    let string = |key: &str| -> Option<String> {
        snapshot.string(key).map(|v| v.to_string()).or_else(|| {
            extra
                .as_ref()
                .and_then(|f| f.string(key))
                .map(|v| v.to_string())
        })
    };
    let boolean = |key: &str| -> Option<bool> {
        snapshot.boolean(key).or_else(|| {
            extra
                .as_ref()
                .and_then(|f| f.boolean(key))
                .and_then(Result::ok)
        })
    };

    GitConfig {
        user_name: string("user.name"),
        user_email: string("user.email"),
        gpg_use_agent: boolean("gpg.use-agent"),
        gpg_program: string("gpg.program"),
        credential_helper: string("credential.helper"),
        signing_key: string("user.signingkey"),
        commit_sign: boolean("commit.gpgsign"),
        tag_sign: boolean("tag.gpgsign"),
        pager: string("core.pager"),
        delta_syntax_theme: string("delta.syntax-theme"),
        delta_light: boolean("delta.light"),
        delta_side_by_side: boolean("delta.side-by-side"),
    }
}

/// Parse the platform-specific system gitconfig that gix's standard search does
/// not include, for use as a lowest-precedence fallback.
fn extra_system_config() -> Option<gix::config::File<'static>> {
    static CACHE: std::sync::OnceLock<Option<gix::config::File<'static>>> =
        std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| extra_system_config_at(platform_extra_system_config_path()))
        .clone()
}

#[cfg(target_os = "macos")]
fn platform_extra_system_config_path() -> Option<&'static std::path::Path> {
    Some(std::path::Path::new(
        "/Library/Developer/CommandLineTools/usr/share/git-core/gitconfig",
    ))
}

#[cfg(target_os = "windows")]
fn platform_extra_system_config_path() -> Option<&'static std::path::Path> {
    Some(std::path::Path::new(r"C:\Program Files\Git\etc\gitconfig"))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_extra_system_config_path() -> Option<&'static std::path::Path> {
    None
}

fn extra_system_config_at(path: Option<&std::path::Path>) -> Option<gix::config::File<'static>> {
    let path = path?;
    if !path.exists() {
        return None;
    }
    gix::config::File::from_path_no_includes(path.to_path_buf(), gix::config::Source::System).ok()
}

/// Fallible local branch enumeration.
///
/// Propagates ref-store, ref-iteration, and peel failures as
/// [`SniffError::Git`] rather than returning empty or partial data.
/// `include_divergence` gates the per-branch ahead/behind computation, which
/// costs two graph reachability walks for every non-current branch. It is
/// `true` for every preset — those values are part of the published preset
/// contract — so only a caller with explicit metadata controls turns it off,
/// and then `ahead`/`behind` report `0`.
pub(crate) fn get_local_branches_fallible(
    repo: &gix::Repository,
    current_branch: Option<&str>,
    include_divergence: bool,
) -> crate::Result<Vec<LocalBranchInfo>> {
    let refs = RefSnapshot::observe(repo, true, false, false)?;
    get_local_branches_from_snapshot(repo, current_branch, include_divergence, &refs)
}

pub(crate) fn get_local_branches_from_snapshot(
    repo: &gix::Repository,
    current_branch: Option<&str>,
    include_divergence: bool,
    refs: &RefSnapshot,
) -> crate::Result<Vec<LocalBranchInfo>> {
    let mut branches = Vec::new();

    // HEAD commit OID for ahead/behind calculations. An unborn HEAD is `None`
    // (no commits to compare against); a malformed/unreadable HEAD surfaces
    // rather than zeroing ahead/behind on a successful-looking branch list.
    let head_oid = if include_divergence {
        super::discovery::head_id_opt(repo)?
    } else {
        None
    };

    for observed in &refs.local_branches {
        let name = observed.name.clone();
        let is_current = current_branch.is_some_and(|cb| cb == name);
        let tip = observed.tip;

        let short_hash = {
            let id = tip.to_string();
            id[..8.min(id.len())].to_string()
        };

        let (ahead, behind) = match (is_current, Some(tip), head_oid) {
            (true, _, _) => (0, 0),
            (false, Some(tip), Some(head)) => ahead_behind(repo, tip, head)?,
            _ => (0, 0),
        };

        branches.push(LocalBranchInfo {
            name,
            short_hash,
            ahead,
            behind,
        });
    }

    Ok(branches)
}

/// Fallible tracking status enumeration.
///
/// Propagates ref-store, ref-iteration, and peel failures as
/// [`SniffError::Git`] rather than returning empty or partial data.
pub(crate) fn get_tracking_status_fallible(
    repo: &gix::Repository,
    current_branch: Option<&str>,
) -> crate::Result<Vec<RemoteTrackingStatus>> {
    let refs = RefSnapshot::observe(repo, true, true, false)?;
    get_tracking_status_from_snapshot(repo, current_branch, &refs)
}

pub(crate) fn get_tracking_status_from_snapshot(
    repo: &gix::Repository,
    current_branch: Option<&str>,
    refs: &RefSnapshot,
) -> crate::Result<Vec<RemoteTrackingStatus>> {
    let mut tracking = Vec::new();

    let Some(branch_name) = current_branch else {
        return Ok(tracking);
    };

    let local = refs
        .local_tip(branch_name)
        .ok_or_else(|| SniffError::git("find_reference", "current branch ref is missing"))?;

    for remote_name in repo.remote_names() {
        let remote_name = remote_name.to_string();
        let Some(remote) = refs.remote_tip(&remote_name, branch_name) else {
            continue;
        };

        // behind: commits on the remote-tracking branch the local does not have.
        let behind = count_reachable_excluding(repo, remote, local)?;
        let ahead = push_relevant_ahead(repo, local, refs.remote_tips(&remote_name))?;

        tracking.push(RemoteTrackingStatus {
            remote: remote_name,
            ahead,
            behind,
        });
    }

    Ok(tracking)
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
    repo: &gix::Repository,
    local: gix::ObjectId,
    hidden: Vec<gix::ObjectId>,
) -> crate::Result<usize> {
    let walk = repo
        .rev_walk(Some(local))
        .with_hidden(hidden)
        .use_commit_graph(Some(true))
        .all()
        .map_err(|e| SniffError::git("revwalk", e))?;
    let mut count = 0;
    for item in walk {
        item.map_err(|e| SniffError::git("revwalk", e))?;
        performance::increment_counter(counters::GIT_COMMIT_VISITS, 1);
        count += 1;
    }
    Ok(count)
}

/// Retrieves all configured remotes with their URLs and hosting providers.
///
/// When `include_remote_details` is true, also includes locally known
/// remote-tracking branches and the resolved default branch.
pub(crate) fn get_remotes(repo: &gix::Repository, include_remote_details: bool) -> Vec<RemoteInfo> {
    let refs = include_remote_details
        .then(|| RefSnapshot::observe(repo, false, true, false).unwrap_or_default());
    get_remotes_from_snapshot(repo, include_remote_details, refs.as_ref())
}

pub(crate) fn get_remotes_from_snapshot(
    repo: &gix::Repository,
    include_remote_details: bool,
    refs: Option<&RefSnapshot>,
) -> Vec<RemoteInfo> {
    // Read URLs straight from config (`remote.<name>.url`) so the exact stored
    // string is preserved rather than gix's reserialized `Url` form.
    let cfg = repo.config_snapshot();
    repo.remote_names()
        .into_iter()
        .map(|name| {
            let name = name.to_string();
            let url = cfg
                .string(format!("remote.{name}.url").as_str())
                .map(|v| v.to_string());
            let provider = url
                .as_ref()
                .map(|u| GitHostingProvider::from_url(u))
                .unwrap_or(GitHostingProvider::Unknown);

            let (branches, default_branch) = if include_remote_details {
                let branches = refs.and_then(|snapshot| {
                    let branches: Vec<String> = snapshot
                        .remote_branches
                        .iter()
                        .filter(|observed| observed.remote == name)
                        .map(|observed| observed.branch.clone())
                        .collect();
                    (!branches.is_empty()).then_some(branches)
                });
                let default_branch = refs
                    .and_then(|snapshot| snapshot.remote_defaults.get(&name))
                    .cloned();
                (branches, default_branch)
            } else {
                (None, None)
            };

            RemoteInfo {
                name,
                url,
                provider,
                branches,
                default_branch,
            }
        })
        .collect()
}

/// Refresh local remote-tracking refs using the user's configured `git` binary.
///
/// When multiple remotes are configured, fetches run with bounded parallelism
/// (up to `max_concurrency`, clamped to 1–3) to reduce latency.  Terminal
/// prompts are disabled so CLI use does not block on credential input.
pub(crate) fn refresh_remote_tracking_refs(repo: &gix::Repository, max_concurrency: usize) {
    refresh_remote_tracking_refs_with_runner(
        repo,
        max_concurrency,
        timeouts::REMOTE_REFRESH,
        &run_command_with_timeout,
    );
}

type CommandRunner<'a> =
    dyn Fn(&mut Command, Duration) -> Result<CapturedOutput, ProcessError> + Sync + 'a;

fn refresh_remote_tracking_refs_with_runner(
    repo: &gix::Repository,
    max_concurrency: usize,
    timeout: Duration,
    runner: &CommandRunner<'_>,
) {
    let Some(repo_root) = repo.workdir() else {
        return;
    };

    let remote_names: Vec<String> = repo
        .remote_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    if remote_names.is_empty() {
        return;
    }

    // Serial path for a single remote — avoid threading overhead.
    if remote_names.len() == 1 {
        fetch_single_remote(repo_root, &remote_names[0], timeout, runner);
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
                    fetch_single_remote(repo_root, &name, timeout, runner);
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
fn fetch_single_remote(
    repo_root: &std::path::Path,
    remote_name: &str,
    timeout: Duration,
    runner: &CommandRunner<'_>,
) {
    let mut command = Command::new("git");
    command
        .current_dir(repo_root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["fetch", "--quiet", "--prune", remote_name]);
    let _ = runner(&mut command, timeout).map_err(|e| {
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
#[cfg(test)]
pub(crate) fn populate_recent_commit_remotes(
    repo: &gix::Repository,
    commits: &mut [CommitInfo],
    max_branches: Option<usize>,
) {
    let refs = RefSnapshot::observe(repo, false, true, false).unwrap_or_default();
    populate_recent_commit_remotes_from_snapshot(repo, commits, max_branches, &refs);
}

pub(crate) fn populate_recent_commit_remotes_from_snapshot(
    repo: &gix::Repository,
    commits: &mut [CommitInfo],
    max_branches: Option<usize>,
    refs: &RefSnapshot,
) {
    let mut remote_tips: Vec<(String, gix::ObjectId)> = refs
        .remote_branches
        .iter()
        .map(|observed| (observed.remote.clone(), observed.tip))
        .collect();
    if remote_tips.is_empty() || commits.is_empty() {
        return;
    }

    // Sort for deterministic truncation, then apply branch limit if configured.
    remote_tips.sort_by(|a, b| a.0.cmp(&b.0));
    if let Some(limit) = max_branches {
        remote_tips.truncate(limit);
    }

    // The only commits this function can ever answer for. Building the target
    // set up front is what bounds the walks below: previously every visited
    // commit was recorded, so a 10,000-commit repo with 50 remote tips built a
    // map of up to 500,000 entries to answer ~10 questions.
    let targets: HashSet<gix::ObjectId> = commits
        .iter()
        .filter_map(|c| gix::ObjectId::from_hex(c.sha.as_bytes()).ok())
        .collect();
    if targets.is_empty() {
        return;
    }

    // Remote *names* are deduplicated into a table and referenced by index
    // during traversal, so a hit costs a `u32` push rather than a `String`
    // clone. Two tips of the same remote (origin/main, origin/dev) share an
    // index, which is what makes the dedup at assembly equivalent to cloning
    // the name per tip.
    let mut remote_names: Vec<String> = Vec::new();
    let tips: Vec<(u32, gix::ObjectId)> = remote_tips
        .iter()
        .map(|(name, oid)| {
            let idx = match remote_names.iter().position(|n| n == name) {
                Some(i) => i,
                None => {
                    remote_names.push(name.clone());
                    remote_names.len() - 1
                }
            };
            (idx as u32, *oid)
        })
        .collect();

    // Walk ancestry from each remote tip, recording only target commits.
    //
    // We do NOT stop early based on commit time: gix's ByCommitTime walk is a
    // lazy frontier, not a globally monotonic sequence. An old-dated child can
    // have a newer-dated parent, so a time-based `break` would incorrectly
    // discard unseen requested commits (see skewed-timestamp test). The
    // target-count stop below is a *reachability* bound and is safe precisely
    // because it makes no assumption about ordering: once every target has been
    // seen on this walk, no further ancestor can change this walk's answer.
    let mut containment: HashMap<gix::ObjectId, Vec<u32>> = HashMap::new();

    for (remote_idx, tip_oid) in &tips {
        let Ok(walk) = repo
            .rev_walk(Some(*tip_oid))
            .sorting(gix::revision::walk::Sorting::ByCommitTime(
                gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
            ))
            .use_commit_graph(Some(true))
            .all()
        else {
            continue;
        };

        let mut found_here = 0usize;
        for info_result in walk {
            let Ok(info) = info_result else {
                continue;
            };
            performance::increment_counter(counters::GIT_COMMIT_VISITS, 1);

            if !targets.contains(&info.id) {
                continue;
            }
            containment.entry(info.id).or_default().push(*remote_idx);
            found_here += 1;
            if found_here == targets.len() {
                break;
            }
        }
    }

    for commit in commits {
        let Ok(commit_oid) = gix::ObjectId::from_hex(commit.sha.as_bytes()) else {
            continue;
        };

        if let Some(indices) = containment.get(&commit_oid) {
            // Names resolved here, once per result commit, rather than cloned
            // once per visited commit during traversal.
            let mut containing: Vec<String> = indices
                .iter()
                .map(|i| remote_names[*i as usize].clone())
                .collect();
            containing.sort();
            containing.dedup();
            if !containing.is_empty() {
                commit.remotes = Some(containing);
            }
        }
    }
}

/// Retrieves all linked worktrees for the repository.
///
/// Returns a HashMap keyed by branch name. Anonymous worktrees (without a name)
/// are filtered out. Paths, branches, and HEAD commits come directly from gix's
/// worktree proxy and administrative `HEAD`. Metadata-only inspection validates
/// the linked checkout marker without opening the repository; repository opens,
/// status walks, and graph probes run only when details are requested.
///
/// When `full_details` is `false`, the expensive per-worktree probes — the
/// commit-graph walks (ahead/behind, merge-conflict detection) and the
/// working-tree status scan — are skipped for any worktree that is not the one
/// identified by `current_worktree_path`; those worktrees report `ahead`,
/// `behind`, and `changed_files` as `0` and `dirty` as `false`. This is the
/// default for [`GitRequest::full()`] and keeps enumeration fast on checkouts
/// with many linked worktrees.
///
/// Registered worktrees whose checkout target is absent are omitted. Git
/// intentionally retains these stale registrations until they are pruned. An
/// existing target that does not open as a repository is corrupt and fails a
/// focused request.
///
/// `current_worktree_path` should be the canonical (or at least absolute) path
/// to the worktree the calling process is running inside. `None` means "no
/// current worktree" (e.g. the caller is outside any git worktree).
pub(crate) fn get_worktrees(
    repo: &gix::Repository,
    full_details: bool,
    current_worktree_path: Option<&Path>,
) -> crate::Result<HashMap<String, WorktreeInfo>> {
    get_worktrees_from_snapshot(repo, full_details, current_worktree_path, None)
}

#[derive(Debug)]
struct LinkedWorktreeMetadata {
    name: String,
    path: PathBuf,
    branch: Option<String>,
    tip: Option<gix::ObjectId>,
}

pub(crate) fn get_worktrees_from_snapshot(
    repo: &gix::Repository,
    full_details: bool,
    current_worktree_path: Option<&Path>,
    refs: Option<&RefSnapshot>,
) -> crate::Result<HashMap<String, WorktreeInfo>> {
    use rayon::prelude::*;

    let proxies = repo
        .worktrees()
        .map_err(|e| SniffError::git("worktrees", e))?;

    // Base branch name and tip for ahead/behind: the main worktree's HEAD,
    // falling back to "main"/"master".
    let (base_branch, base_oid) = resolve_base_branch(repo)?;

    // Canonicalize the current worktree path once so every worker can do a
    // cheap path comparison without repeated disk access.
    let current_canonical = current_worktree_path.and_then(|p| std::fs::canonicalize(p).ok());

    // HEAD and checkout location live in each proxy's administrative directory,
    // so metadata-only projections do not need to open the checkout as a full
    // repository. The checkout marker is still validated so a present but
    // structurally corrupt linked checkout is not reported as healthy.
    let mut worktree_metadata = Vec::new();
    for proxy in proxies {
        let name = proxy.id().to_str_lossy().into_owned();
        let path = proxy
            .base()
            .map_err(|e| SniffError::git("worktree_base", e))?;
        if !path.is_absolute() {
            return Err(SniffError::git(
                "worktree_base",
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "registered worktree path is not absolute",
                ),
            ));
        }
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        let (branch, direct_tip) = read_worktree_head(proxy.git_dir());
        let tip = match branch.as_deref() {
            Some(branch) => refs
                .and_then(|snapshot| snapshot.local_tip(branch))
                .or_else(|| peel_branch_tip(repo, branch)),
            None => direct_tip,
        };
        worktree_metadata.push(LinkedWorktreeMetadata {
            name,
            path,
            branch,
            tip,
        });
    }

    // R4: open the base repository once and share it across Rayon workers via
    // a thread-safe handle; each worker derives a cheap thread-local view
    // rather than reopening the base repo N times.
    let base_sync = repo.clone().into_sync();

    let collector = performance::current_collector();
    let results: crate::Result<Vec<Option<(String, WorktreeInfo)>>> = worktree_metadata
        .par_iter()
        .map(|metadata| {
            let _worker = performance::pooled_worker(collector.as_ref());
            if !metadata.path.exists() {
                return Ok(None);
            }
            let mut base = base_sync.to_thread_local();
            let branch = metadata
                .branch
                .clone()
                .unwrap_or_else(|| metadata.name.clone());
            let wt_head = metadata.tip;
            let sha = wt_head.map(|o| o.to_string()).unwrap_or_default();

            // Determine whether this worktree is the current one.
            let is_current = current_canonical.as_ref().is_some_and(|current| {
                metadata.path.as_path() == current.as_path()
            });

            // Skip expensive commit-graph walks for non-current worktrees when
            // the caller has not requested full details.
            let compute_full = full_details || is_current;
            let base_workdir = base
                .workdir()
                .map(|path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()));
            let base_is_worktree = base_workdir.as_deref() == Some(metadata.path.as_path());
            if !base_is_worktree && !compute_full && !metadata.path.join(".git").is_file() {
                return Err(SniffError::git(
                    "open",
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "registered worktree is missing its .git file",
                    ),
                ));
            }
            let mut opened_worktree = if !base_is_worktree && compute_full {
                performance::increment_counter(counters::GIT_WORKTREE_OPENS, 1);
                let Some(mut opened) = super::open::trusted_open_registered_worktree(
                    &metadata.path,
                )? else {
                    return Ok(None);
                };
                super::open::configure_cache(&mut opened);
                Some(opened)
            } else {
                None
            };
            let detail_repo = opened_worktree.as_mut().unwrap_or(&mut base);

            let (ahead, behind) = if compute_full {
                match (wt_head, base_oid) {
                    (Some(wt), Some(base_id)) => ahead_behind(detail_repo, wt, base_id)?,
                    _ => (0, 0),
                }
            } else {
                (0, 0)
            };

            // `merged` when the base already contains the worktree tip.
            let merged = if compute_full {
                match (wt_head, base_oid) {
                    (Some(wt), Some(base_id)) => is_ancestor(detail_repo, wt, base_id)?,
                    _ => false,
                }
            } else {
                false
            };

            // R5: a merged branch cannot conflict, so skip the merge probe;
            // only unmerged branches are merged in-memory to detect conflicts.
            let has_conflicts = if compute_full {
                if merged {
                    false
                } else {
                    match (wt_head, base_oid) {
                        (Some(wt), Some(base_id)) => {
                            !super::merge_conflicts::merge_conflicts_between(
                                detail_repo,
                                wt,
                                base_id,
                            )?
                            .is_empty()
                        }
                        _ => false,
                    }
                }
            } else {
                false
            };

            // The working-tree status walk is the single most expensive
            // per-worktree probe after the commit-graph walks. Skip it for
            // non-current worktrees unless full detail was requested: the
            // default `git-status` renders only a count for other worktrees, so
            // their dirty state is never shown and must not cost a status scan.
            let (dirty, changed_files) = if compute_full {
                get_repo_status_counts(detail_repo)?
            } else {
                (false, 0)
            };

            Ok(Some((
                branch.clone(),
                WorktreeInfo {
                    branch,
                    filepath: metadata.path.clone(),
                    sha,
                    dirty,
                    ahead,
                    behind,
                    base_branch: base_branch.clone(),
                    has_conflicts,
                    merged,
                    changed_files,
                    is_current,
                },
            )))
        })
        .collect();

    Ok(results?.into_iter().flatten().collect())
}

fn read_worktree_head(git_dir: &Path) -> (Option<String>, Option<gix::ObjectId>) {
    let Ok(bytes) = std::fs::read(git_dir.join("HEAD")) else {
        return (None, None);
    };
    let head = bytes.trim();
    if let Some(reference) = head.strip_prefix(b"ref: ") {
        let reference = reference.as_bstr().to_str_lossy();
        let branch = reference
            .strip_prefix("refs/heads/")
            .unwrap_or(&reference)
            .to_string();
        (Some(branch), None)
    } else {
        (None, gix::ObjectId::from_hex(head).ok())
    }
}

fn peel_branch_tip(repo: &gix::Repository, branch: &str) -> Option<gix::ObjectId> {
    repo.find_reference(&format!("refs/heads/{branch}"))
        .ok()?
        .into_fully_peeled_id()
        .ok()
        .map(|id| id.detach())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    const REFRESH_TIMEOUT_CHILD: &str =
        "filesystem::git::remote_refresh::tests::refresh_timeout_child";

    fn refresh_fixture_args() -> Vec<std::ffi::OsString> {
        [REFRESH_TIMEOUT_CHILD, "--exact", "--ignored", "--nocapture"]
            .into_iter()
            .map(Into::into)
            .collect()
    }

    #[test]
    #[ignore = "subprocess fixture invoked by the remote refresh timeout test"]
    fn refresh_timeout_child() {
        std::thread::sleep(Duration::from_secs(30));
    }

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

    /// Commit `content` to `rel_path` with the given `parents`, returning the
    /// new commit Oid without moving any ref.
    fn commit_detached(
        repo: &Repository,
        rel_path: &str,
        content: &str,
        msg: &str,
        parents: &[git2::Oid],
    ) -> git2::Oid {
        let sig = git2::Signature::now("T", "t@e.com").unwrap();
        let mut index = repo.index().unwrap();
        // Reset the (shared) index to the first parent's tree so each commit
        // starts clean instead of inheriting stray index state.
        match parents.first() {
            Some(&p) => {
                let parent_tree = repo.find_commit(p).unwrap().tree().unwrap();
                index.read_tree(&parent_tree).unwrap();
            }
            None => index.clear().unwrap(),
        }
        std::fs::write(repo.workdir().unwrap().join(rel_path), content).unwrap();
        index.add_path(Path::new(rel_path)).unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent_commits: Vec<git2::Commit> = parents
            .iter()
            .map(|p| repo.find_commit(*p).unwrap())
            .collect();
        let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();
        repo.commit(None, &sig, &sig, msg, &tree, &parent_refs)
            .unwrap()
    }

    fn to_gix(oid: git2::Oid) -> gix::ObjectId {
        gix::ObjectId::from_hex(oid.to_string().as_bytes()).unwrap()
    }

    /// Commit `content` to `rel_path` with the given `parents` and `seconds`
    /// timestamp, returning the new commit Oid without moving any ref.
    fn commit_detached_at(
        repo: &Repository,
        rel_path: &str,
        content: &str,
        msg: &str,
        parents: &[git2::Oid],
        seconds: i64,
    ) -> git2::Oid {
        let sig = git2::Signature::new("T", "t@e.com", &git2::Time::new(seconds, 0)).unwrap();
        let mut index = repo.index().unwrap();
        match parents.first() {
            Some(&p) => {
                let parent_tree = repo.find_commit(p).unwrap().tree().unwrap();
                index.read_tree(&parent_tree).unwrap();
            }
            None => index.clear().unwrap(),
        }
        std::fs::write(repo.workdir().unwrap().join(rel_path), content).unwrap();
        index.add_path(Path::new(rel_path)).unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent_commits: Vec<git2::Commit> = parents
            .iter()
            .map(|p| repo.find_commit(*p).unwrap())
            .collect();
        let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();
        repo.commit(None, &sig, &sig, msg, &tree, &parent_refs)
            .unwrap()
    }

    #[test]
    fn populate_commit_remotes_skewed_timestamp_ancestor_not_pruned() {
        let (dir, repo) = setup_repo();

        // Create a skewed-timestamp chain:
        //   C (time 200) <- A (time 50) <- B (time 100, HEAD, remote tip)
        //
        // ByCommitTime(NewestFirst) walk order from B:
        //   B (100) → A (50) → C (200)
        //
        // An old `break` on A (50 < oldest_time=200) would prune C before it
        // is visited. The fix removes time-based stopping so C is found.
        let base = repo.head().unwrap().peel_to_commit().unwrap().id();
        let c = commit_detached_at(&repo, "c.txt", "c", "c", &[base], 200);
        let a = commit_detached_at(&repo, "a.txt", "a", "a", &[c], 50);
        let b = commit_detached_at(&repo, "b.txt", "b", "b", &[a], 100);

        // Point HEAD at B so the walk starts from the remote tip.
        repo.reference("refs/heads/main", b, true, "set main")
            .unwrap();
        repo.set_head("refs/heads/main").unwrap();

        // Fake remote pointing at B.
        add_fake_remote(&repo, "origin", "main", b);

        // Request commit C (the skewed ancestor with timestamp 200).
        let mut commits = vec![CommitInfo {
            sha: c.to_string(),
            message: "c".to_string(),
            author: "T".to_string(),
            timestamp: chrono::Utc::now(),
            remotes: None,
            refs: vec![],
        }];

        let gix_repo = open_gix(&dir);
        populate_recent_commit_remotes(&gix_repo, &mut commits, None);

        assert_eq!(
            commits[0].remotes,
            Some(vec!["origin".to_string()]),
            "skewed ancestor C must be found via origin even though its parent A is older"
        );
    }

    #[test]
    fn has_merge_conflicts_matches_git2_oracle() {
        let (dir, repo) = setup_repo();
        let base = repo.head().unwrap().peel_to_commit().unwrap().id();

        // Two commits that both rewrite test.txt from the base → conflict.
        let ours = commit_detached(&repo, "test.txt", "ours\n", "ours", &[base]);
        let theirs = commit_detached(&repo, "test.txt", "theirs\n", "theirs", &[base]);

        // A commit that only adds a new file → no overlap, merges cleanly.
        let clean = commit_detached(&repo, "other.txt", "clean\n", "clean", &[base]);

        let gix_repo = gix::open(dir.path()).unwrap();

        // gix probe vs git2 oracle: conflicting pair.
        let git2_conflict = repo
            .merge_commits(
                &repo.find_commit(ours).unwrap(),
                &repo.find_commit(theirs).unwrap(),
                None,
            )
            .unwrap()
            .has_conflicts();
        assert!(git2_conflict, "git2 oracle: ours/theirs should conflict");
        assert!(
            !crate::filesystem::git::merge_conflicts::merge_conflicts_between(
                &gix_repo,
                to_gix(ours),
                to_gix(theirs),
            )
            .unwrap()
            .is_empty(),
            "gix probe must agree: ours/theirs conflict"
        );

        // gix probe vs git2 oracle: clean pair.
        let git2_clean_conflict = repo
            .merge_commits(
                &repo.find_commit(ours).unwrap(),
                &repo.find_commit(clean).unwrap(),
                None,
            )
            .unwrap()
            .has_conflicts();
        assert!(
            !git2_clean_conflict,
            "git2 oracle: ours/clean should not conflict"
        );
        assert!(
            crate::filesystem::git::merge_conflicts::merge_conflicts_between(
                &gix_repo,
                to_gix(ours),
                to_gix(clean),
            )
            .unwrap()
            .is_empty(),
            "gix probe must agree: ours/clean is conflict-free"
        );
    }

    #[test]
    fn refresh_remote_tracking_refs_single_remote_runs_without_panic() {
        let (_dir, repo) = setup_repo();
        let gix_repo = gix::open(repo.workdir().unwrap()).unwrap();
        // With a single remote, the serial path should execute without error.
        refresh_remote_tracking_refs(&gix_repo, 2);
        // The function has no return value; absence of panic is the test.
    }

    #[test]
    fn refresh_remote_tracking_refs_zero_concurrency_uses_one() {
        let (_dir, repo) = setup_repo();
        let gix_repo = gix::open(repo.workdir().unwrap()).unwrap();
        // Concurrency of 0 should be clamped to 1.
        refresh_remote_tracking_refs(&gix_repo, 0);
    }

    #[test]
    fn remote_refresh_preserves_builder_configuration_and_honors_timeout() {
        let (dir, repo) = setup_repo();
        repo.remote("origin", "https://example.invalid/repo")
            .unwrap();
        let gix_repo = gix::open(dir.path()).unwrap();
        let calls = AtomicUsize::new(0);
        let runner = |command: &mut Command, timeout| {
            calls.fetch_add(1, Ordering::Relaxed);
            assert_eq!(timeout, Duration::ZERO);
            assert_eq!(command.get_program(), "git");
            assert_eq!(command.get_current_dir(), Some(dir.path()));
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                ["fetch", "--quiet", "--prune", "origin"].map(std::ffi::OsStr::new)
            );
            assert!(command.get_envs().any(|(key, value)| {
                key == "GIT_TERMINAL_PROMPT" && value == Some(std::ffi::OsStr::new("0"))
            }));

            let executable =
                std::env::current_exe().expect("current test executable should resolve");
            let mut fixture = Command::new(executable);
            fixture.args(refresh_fixture_args());
            run_command_with_timeout(&mut fixture, timeout)
        };

        let start = std::time::Instant::now();
        refresh_remote_tracking_refs_with_runner(&gix_repo, 2, Duration::ZERO, &runner);

        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn extra_system_config_macos_does_not_panic_when_clt_file_absent() {
        // If the CLT gitconfig does not exist, extra_system_config must return
        // None without panicking. When it does exist, it must parse successfully.
        let path = std::path::Path::new(
            "/Library/Developer/CommandLineTools/usr/share/git-core/gitconfig",
        );
        if path.exists() {
            let file = gix::config::File::from_path_no_includes(
                path.to_path_buf(),
                gix::config::Source::System,
            );
            assert!(file.is_ok(), "CLT gitconfig must parse when present");
        } else {
            assert!(
                extra_system_config().is_none(),
                "extra_system_config must return None when CLT file is absent"
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn extra_system_config_windows_does_not_panic_when_file_absent() {
        let path = std::path::Path::new(r"C:\Program Files\Git\etc\gitconfig");
        if path.exists() {
            let file = gix::config::File::from_path_no_includes(
                path.to_path_buf(),
                gix::config::Source::System,
            );
            assert!(
                file.is_ok(),
                "Git for Windows gitconfig must parse when present"
            );
        } else {
            assert!(
                extra_system_config().is_none(),
                "extra_system_config must return None when Git for Windows config is absent"
            );
        }
    }

    #[test]
    fn extra_system_config_at_reads_file_when_present() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("extra.gitconfig");
        std::fs::write(
            &path,
            "[user]\n\tname = Extra Tester\n\temail = extra@sniff.test\n",
        )
        .unwrap();

        let file = extra_system_config_at(Some(&path));
        assert!(file.is_some(), "must parse existing file");
        let file = file.unwrap();
        assert_eq!(
            file.string("user.name").map(|s| s.to_string()),
            Some("Extra Tester".to_string())
        );
        assert_eq!(
            file.string("user.email").map(|s| s.to_string()),
            Some("extra@sniff.test".to_string())
        );
    }

    #[test]
    fn extra_system_config_at_returns_none_when_path_missing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.gitconfig");
        assert!(
            extra_system_config_at(Some(&path)).is_none(),
            "must return None for missing file"
        );
    }

    #[test]
    fn extra_system_config_at_returns_none_when_path_is_none() {
        assert!(
            extra_system_config_at(None).is_none(),
            "must return None when no path is provided"
        );
    }

    #[test]
    fn extra_system_config_fallback_has_lowest_precedence() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .unwrap();

        // Set a local value.
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Local Tester").unwrap();
        }

        // Create an extra-system config with a different value.
        let extra_path = dir.path().join("extra.gitconfig");
        std::fs::write(&extra_path, "[user]\n\tname = Extra Tester\n").unwrap();

        let gix_repo = gix::open(repo.workdir().unwrap()).unwrap();
        // Temporarily swap the production path to our fake file.
        let file = extra_system_config_at(Some(&extra_path));
        assert!(file.is_some());

        // get_git_config_with_extra uses the snapshot (local) first, then falls
        // back to the injected extra config. Local must win.
        let config = get_git_config_with_extra(&gix_repo, file);
        assert_eq!(
            config.user_name.as_deref(),
            Some("Local Tester"),
            "local config must override extra system fallback"
        );
    }

    #[serial_test::serial]
    #[test]
    fn extra_system_config_fallback_reads_all_12_keys() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .unwrap();

        // Create an extra-system config with all 12 keys; the repo has no local
        // overrides, so every key must surface from the fallback.
        let extra_path = dir.path().join("extra.gitconfig");
        std::fs::write(
            &extra_path,
            "[user]\n\
             \tname = Extra Name\n\
             \temail = extra@sniff.test\n\
             \tsigningkey = EXTRAKEY\n\
             [gpg]\n\
             \tuse-agent = true\n\
             \tprogram = gpg-extra\n\
             [credential]\n\
             \thelper = store-extra\n\
             [commit]\n\
             \tgpgsign = true\n\
             [tag]\n\
             \tgpgsign = false\n\
             [core]\n\
             \tpager = less-extra\n\
             [delta]\n\
             \tsyntax-theme = DraculaExtra\n\
             \tlight = true\n\
             \tside-by-side = false\n",
        )
        .unwrap();

        // Isolate from host config: point HOME to an empty directory,
        // GIT_CONFIG_GLOBAL to a nonexistent file, and GIT_CONFIG_SYSTEM to an
        // empty file so gix sees no system/global config.
        let fake_home = TempDir::new().unwrap();
        let empty_system = fake_home.path().join("system.gitconfig");
        std::fs::write(&empty_system, "").unwrap();

        let _home_guard = test_toolkit::EnvGuard::set_safe("HOME", fake_home.path());
        let _global_guard = test_toolkit::EnvGuard::set_safe("GIT_CONFIG_GLOBAL", "/dev/null");
        let _system_guard = test_toolkit::EnvGuard::set_safe("GIT_CONFIG_SYSTEM", &empty_system);

        let gix_repo = gix::open(repo.workdir().unwrap()).unwrap();
        let file = extra_system_config_at(Some(&extra_path));
        assert!(file.is_some());

        let config = get_git_config_with_extra(&gix_repo, file);

        assert_eq!(config.user_name.as_deref(), Some("Extra Name"));
        assert_eq!(config.user_email.as_deref(), Some("extra@sniff.test"));
        assert_eq!(config.gpg_use_agent, Some(true));
        assert_eq!(config.gpg_program.as_deref(), Some("gpg-extra"));
        assert_eq!(config.credential_helper.as_deref(), Some("store-extra"));
        assert_eq!(config.signing_key.as_deref(), Some("EXTRAKEY"));
        assert_eq!(config.commit_sign, Some(true));
        assert_eq!(config.tag_sign, Some(false));
        assert_eq!(config.pager.as_deref(), Some("less-extra"));
        assert_eq!(config.delta_syntax_theme.as_deref(), Some("DraculaExtra"));
        assert_eq!(config.delta_light, Some(true));
        assert_eq!(config.delta_side_by_side, Some(false));
    }

    #[test]
    fn corrupt_index_status_propagates_error() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .unwrap();

        // Corrupt the index checksum so status detects a hash mismatch rather
        // than hitting an early parse panic.
        let index_path = dir.path().join(".git").join("index");
        let mut index_bytes = std::fs::read(&index_path).unwrap();
        if index_bytes.len() >= 20 {
            let len = index_bytes.len();
            index_bytes[len - 1] = index_bytes[len - 1].wrapping_add(1);
        }
        std::fs::write(&index_path, index_bytes).unwrap();

        let gix_repo = gix::open(dir.path()).unwrap();
        let result = crate::filesystem::git::status::get_repo_status_counts(&gix_repo);
        assert!(
            result.is_err(),
            "corrupt index must cause status to return an error, not (false, 0)"
        );
    }

    #[test]
    fn corrupt_object_revwalk_propagates_error() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let initial = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .unwrap();

        // Second commit so we have something to walk.
        std::fs::write(&file_path, "second\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let second = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Second",
                &tree,
                &[&repo.find_commit(initial).unwrap()],
            )
            .unwrap();

        // Corrupt the initial commit object (make writable first — git creates
        // objects read-only).
        let obj_hex = initial.to_string();
        let obj_path = dir
            .path()
            .join(".git")
            .join("objects")
            .join(&obj_hex[..2])
            .join(&obj_hex[2..]);
        let mut perms = std::fs::metadata(&obj_path).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o644);
        }
        #[cfg(windows)]
        {
            perms.set_readonly(false);
        }
        std::fs::set_permissions(&obj_path, perms).unwrap();
        std::fs::write(&obj_path, b"garbage").unwrap();

        let gix_repo = gix::open(dir.path()).unwrap();
        let result = count_reachable_excluding(&gix_repo, to_gix(second), to_gix(initial));
        assert!(
            result.is_err(),
            "corrupt object must cause revwalk to return an error, not 0"
        );
    }

    #[test]
    fn corrupt_object_ancestry_propagates_error() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let initial = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .unwrap();

        std::fs::write(&file_path, "second\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let second = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Second",
                &tree,
                &[&repo.find_commit(initial).unwrap()],
            )
            .unwrap();

        // Corrupt the initial commit object so merge-base cannot read it.
        let obj_hex = initial.to_string();
        let obj_path = dir
            .path()
            .join(".git")
            .join("objects")
            .join(&obj_hex[..2])
            .join(&obj_hex[2..]);
        let mut perms = std::fs::metadata(&obj_path).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o644);
        }
        #[cfg(windows)]
        {
            perms.set_readonly(false);
        }
        std::fs::set_permissions(&obj_path, perms).unwrap();
        std::fs::write(&obj_path, b"garbage").unwrap();

        let gix_repo = gix::open(dir.path()).unwrap();
        let result = is_ancestor(&gix_repo, to_gix(initial), to_gix(second));
        assert!(
            result.is_err(),
            "corrupt object must cause ancestry check to return an error, not false"
        );
    }

    #[test]
    fn merge_failure_propagates_error() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let base = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .unwrap();

        let ours = commit_detached(&repo, "test.txt", "ours\n", "ours", &[base]);
        let theirs = commit_detached(&repo, "test.txt", "theirs\n", "theirs", &[base]);

        // Corrupt one of the tip objects so merge_commits cannot read it.
        let obj_hex = ours.to_string();
        let obj_path = dir
            .path()
            .join(".git")
            .join("objects")
            .join(&obj_hex[..2])
            .join(&obj_hex[2..]);
        let mut perms = std::fs::metadata(&obj_path).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o644);
        }
        #[cfg(windows)]
        {
            perms.set_readonly(false);
        }
        std::fs::set_permissions(&obj_path, perms).unwrap();
        std::fs::write(&obj_path, b"garbage").unwrap();

        let gix_repo = gix::open(dir.path()).unwrap();
        let result = crate::filesystem::git::merge_conflicts::merge_conflicts_between(
            &gix_repo,
            to_gix(ours),
            to_gix(theirs),
        );
        assert!(
            result.is_err(),
            "corrupt object must cause merge-conflict probe to return an error, not false"
        );
    }

    #[test]
    fn corrupt_ref_branch_enumeration_propagates_error() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .unwrap();

        // Corrupt the packed-refs file (if any) by creating a malformed one
        // that gix will fail to parse when listing refs.
        let packed_refs = dir.path().join(".git").join("packed-refs");
        std::fs::write(
            &packed_refs,
            "# pack-refs with: peeled fully-peeled\n^garbage\n",
        )
        .unwrap();

        let gix_repo = gix::open(dir.path()).unwrap();
        let result = get_local_branches_fallible(&gix_repo, Some("main"), true);
        assert!(
            result.is_err(),
            "corrupt refs must cause branch enumeration to return an error, not empty vec"
        );
    }

    #[test]
    fn corrupt_ref_item_branch_enumeration_propagates_error() {
        // A malformed loose ref: enumerating refs/heads succeeds, but reading
        // this individual item fails *during iteration* (not at construction).
        // The previous `iter.flatten()` silently dropped such items.
        let (dir, repo) = setup_repo();
        let branch = repo.head().unwrap().shorthand().unwrap().to_string();

        let bad_ref = dir
            .path()
            .join(".git")
            .join("refs")
            .join("heads")
            .join("bad");
        std::fs::write(&bad_ref, "not-a-valid-ref\n").unwrap();

        let gix_repo = open_gix(&dir);
        let result = get_local_branches_fallible(&gix_repo, Some(&branch), true);
        assert!(
            result.is_err(),
            "a ref item that fails during iteration must propagate, not be dropped"
        );
    }

    #[test]
    fn corrupt_remote_ref_peel_tracking_status_propagates_error() {
        // The current branch and its origin tracking ref are valid, so the
        // behind-count lookup succeeds and `push_relevant_ahead` is reached.
        // A second remote-tracking ref points at a missing object: peeling it
        // inside `push_relevant_ahead` must propagate rather than be swallowed
        // into an empty hidden-ref set.
        let (dir, repo) = setup_repo();
        repo.remote("origin", "https://example.com/repo").unwrap();
        let branch = repo.head().unwrap().shorthand().unwrap().to_string();
        let head = repo.head().unwrap().target().unwrap();
        add_fake_remote(&repo, "origin", &branch, head);

        let stale = dir
            .path()
            .join(".git")
            .join("refs")
            .join("remotes")
            .join("origin")
            .join("stale");
        std::fs::create_dir_all(stale.parent().unwrap()).unwrap();
        // A syntactically valid SHA that names no object → peel fails.
        std::fs::write(&stale, "1111111111111111111111111111111111111111\n").unwrap();

        let gix_repo = open_gix(&dir);
        let result = get_tracking_status_fallible(&gix_repo, Some(&branch));
        assert!(
            result.is_err(),
            "a remote-ref peel failure inside push_relevant_ahead must propagate"
        );
    }

    #[test]
    fn corrupt_ref_tracking_status_propagates_error() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .unwrap();

        // Set up a remote-tracking ref so tracking status has something to query.
        repo.reference(
            "refs/remotes/origin/main",
            repo.head().unwrap().target().unwrap(),
            false,
            "remote",
        )
        .unwrap();

        // Corrupt the packed-refs file to break ref iteration.
        let packed_refs = dir.path().join(".git").join("packed-refs");
        std::fs::write(
            &packed_refs,
            "# pack-refs with: peeled fully-peeled\n^garbage\n",
        )
        .unwrap();

        let gix_repo = gix::open(dir.path()).unwrap();
        let result = get_tracking_status_fallible(&gix_repo, Some("main"));
        assert!(
            result.is_err(),
            "corrupt refs must cause tracking status to return an error, not empty vec"
        );
    }

    // ------------------------------------------------------------------
    // get_worktrees lazy-detail tests
    // ------------------------------------------------------------------

    /// Creates a repo with one commit on master and two linked worktrees
    /// (`feature` and `other`). Each worktree has one additional commit on
    /// its own branch, making it one commit ahead of master.
    fn setup_repo_with_worktrees() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "initial\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
                .unwrap();
        }

        // Create linked worktrees.
        let feature_path = dir.path().join("feature");
        let _wt_feature = repo.worktree("feature", &feature_path, None).unwrap();
        let other_path = dir.path().join("other");
        let _wt_other = repo.worktree("other", &other_path, None).unwrap();

        // Add a commit to the feature worktree.
        {
            let wt_repo = Repository::open(&feature_path).unwrap();
            let head = wt_repo.head().unwrap().peel_to_commit().unwrap();
            wt_repo.branch("feature", &head, false).unwrap();
            wt_repo.set_head("refs/heads/feature").unwrap();
            std::fs::write(feature_path.join("feat.txt"), "feat\n").unwrap();
            let mut index = wt_repo.index().unwrap();
            index.add_path(Path::new("feat.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = wt_repo.find_tree(tree_id).unwrap();
            wt_repo
                .commit(Some("HEAD"), &sig, &sig, "Feature", &tree, &[&head])
                .unwrap();
        }

        // Add a commit to the other worktree.
        {
            let wt_repo = Repository::open(&other_path).unwrap();
            let head = wt_repo.head().unwrap().peel_to_commit().unwrap();
            wt_repo.branch("other", &head, false).unwrap();
            wt_repo.set_head("refs/heads/other").unwrap();
            std::fs::write(other_path.join("other.txt"), "other\n").unwrap();
            let mut index = wt_repo.index().unwrap();
            index.add_path(Path::new("other.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = wt_repo.find_tree(tree_id).unwrap();
            wt_repo
                .commit(Some("HEAD"), &sig, &sig, "Other", &tree, &[&head])
                .unwrap();
        }

        (dir, repo)
    }

    #[test]
    fn get_worktrees_default_mode_skips_ahead_behind_for_non_current() {
        let (dir, _repo) = setup_repo_with_worktrees();
        let feature_path = dir.path().join("feature");
        let gix_repo = gix::open(dir.path()).unwrap();

        let worktrees = get_worktrees(&gix_repo, false, Some(&feature_path)).unwrap();

        assert_eq!(
            worktrees.len(),
            2,
            "both linked worktrees must be enumerated"
        );

        let feature = worktrees
            .get("feature")
            .expect("feature worktree must exist");
        assert!(
            feature.is_current,
            "feature worktree must be marked current"
        );
        assert!(
            feature.ahead > 0,
            "current worktree must have ahead computed"
        );

        let other = worktrees.get("other").expect("other worktree must exist");
        assert!(
            !other.is_current,
            "other worktree must not be marked current"
        );
        assert_eq!(
            other.ahead, 0,
            "non-current worktree must skip ahead in default mode"
        );
        assert_eq!(
            other.behind, 0,
            "non-current worktree must skip behind in default mode"
        );
        assert!(
            !other.has_conflicts,
            "non-current worktree must skip conflict probe in default mode"
        );
        assert!(
            !other.merged,
            "non-current worktree must skip merge check in default mode"
        );
    }

    #[test]
    fn get_worktrees_full_detail_computes_ahead_behind_for_all() {
        let (dir, _repo) = setup_repo_with_worktrees();
        let feature_path = dir.path().join("feature");
        let gix_repo = gix::open(dir.path()).unwrap();

        let collector = crate::performance::PerformanceCollector::new_shared();
        let worktrees = crate::performance::with_current_collector(
            Some(collector.clone()),
            || get_worktrees(&gix_repo, true, Some(&feature_path)).unwrap(),
        );

        let feature = worktrees
            .get("feature")
            .expect("feature worktree must exist");
        assert!(
            feature.ahead > 0,
            "current worktree must have ahead in full-detail mode"
        );

        let other = worktrees.get("other").expect("other worktree must exist");
        assert!(
            other.ahead > 0,
            "non-current worktree must also have ahead in full-detail mode"
        );
        let counters = collector
            .snapshot(std::time::Duration::ZERO)
            .counters;
        assert_eq!(
            counters
                .get(counters::GIT_WORKTREE_OPENS)
                .copied()
                .unwrap_or(0),
            2,
            "full details open only the two checkouts unavailable through the main handle"
        );
    }

    #[test]
    fn get_worktrees_omits_an_absent_registered_target() {
        let (dir, _repo) = setup_repo_with_worktrees();
        std::fs::remove_dir_all(dir.path().join("feature")).unwrap();
        let gix_repo = gix::open(dir.path()).unwrap();

        let worktrees = get_worktrees(&gix_repo, true, None).unwrap();

        assert!(!worktrees.contains_key("feature"));
        assert!(worktrees.contains_key("other"));
    }

    #[test]
    fn get_worktrees_surfaces_an_existing_corrupt_target() {
        let (dir, _repo) = setup_repo_with_worktrees();
        std::fs::write(dir.path().join("feature").join(".git"), "invalid gitdir\n").unwrap();
        let gix_repo = gix::open(dir.path()).unwrap();

        let error = get_worktrees(&gix_repo, true, None).unwrap_err();

        assert!(matches!(error, SniffError::Git { .. }));
    }

    #[test]
    fn get_worktrees_no_current_path_skips_all_ahead_behind() {
        let (dir, _repo) = setup_repo_with_worktrees();
        let gix_repo = gix::open(dir.path()).unwrap();

        let worktrees = get_worktrees(&gix_repo, false, None).unwrap();

        let feature = worktrees
            .get("feature")
            .expect("feature worktree must exist");
        assert!(
            !feature.is_current,
            "without a current path, no worktree is current"
        );
        assert_eq!(
            feature.ahead, 0,
            "all worktrees skip ahead when no current path is given"
        );

        let other = worktrees.get("other").expect("other worktree must exist");
        assert_eq!(
            other.ahead, 0,
            "all worktrees skip ahead when no current path is given"
        );
    }

    #[test]
    fn get_worktrees_main_worktree_current_skips_linked_ahead_behind() {
        let (dir, _repo) = setup_repo_with_worktrees();
        let gix_repo = gix::open(dir.path()).unwrap();

        // Current path is the main worktree, not a linked worktree.
        let worktrees = get_worktrees(&gix_repo, false, Some(dir.path())).unwrap();

        let feature = worktrees
            .get("feature")
            .expect("feature worktree must exist");
        assert!(
            !feature.is_current,
            "linked worktree must not be current when main is current"
        );
        assert_eq!(
            feature.ahead, 0,
            "linked worktree must skip ahead when main is current"
        );

        let other = worktrees.get("other").expect("other worktree must exist");
        assert_eq!(
            other.ahead, 0,
            "linked worktree must skip ahead when main is current"
        );
    }

    #[test]
    fn branch_info_reports_some_ahead_behind_with_configured_upstream() {
        let (dir, repo) = setup_repo();
        let branch = repo.head().unwrap().shorthand().unwrap().to_string();
        let base = repo.head().unwrap().peel_to_commit().unwrap().id();

        // origin/<branch> stays at the base commit.
        add_fake_remote(&repo, "origin", &branch, base);

        // Advance the local branch one commit past origin.
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

        // Configure the upstream so the branch tracks origin/<branch>.
        let mut config = repo.config().unwrap();
        config
            .set_str(&format!("branch.{branch}.remote"), "origin")
            .unwrap();
        config
            .set_str(
                &format!("branch.{branch}.merge"),
                &format!("refs/heads/{branch}"),
            )
            .unwrap();

        let gix_repo = open_gix(&dir);
        let branches = get_branch_info_fallible(&gix_repo, Some(&branch)).unwrap();
        let info = branches
            .iter()
            .find(|b| b.name == branch)
            .expect("branch present");

        assert_eq!(
            info.upstream.as_deref(),
            Some(format!("origin/{branch}").as_str())
        );
        assert_eq!(info.ahead, Some(1), "one commit ahead of origin");
        assert_eq!(info.behind, Some(0), "zero behind origin");
    }

    #[test]
    fn branch_info_reports_null_tracking_without_upstream() {
        let (dir, repo) = setup_repo();
        let branch = repo.head().unwrap().shorthand().unwrap().to_string();

        let gix_repo = open_gix(&dir);
        let branches = get_branch_info_fallible(&gix_repo, Some(&branch)).unwrap();
        let info = branches
            .iter()
            .find(|b| b.name == branch)
            .expect("branch present");

        // No configured upstream: tracking facts are absent, not zeroed.
        assert_eq!(info.upstream, None);
        assert_eq!(info.ahead, None);
        assert_eq!(info.behind, None);
    }
}
