//! The library-owned aggregate observation behind bare `sniff repo --json`.
//!
//! Everything bare `sniff repo --json` renders is either already on the
//! `FilesystemInfo` its detection pass produced, or is one of the six facts
//! [`RepoAggregate`] supplies here. The CLI therefore discovers no repository,
//! lists no worktrees, queries no branches, reads no conflicts, and walks no
//! history of its own — see the Phase 2 sub-spec, contract C2.
//!
//! Cost contract: one `GitRepo::discover` and zero additional status walks per
//! call. Every working-tree scope bucket is a projection over the
//! `GitInfo.file_changes` the detection pass already collected (contract C1),
//! not a fresh observation.

use std::path::Path;

use chrono::Duration;

use crate::filesystem::FilesystemInfo;
use crate::filesystem::git::recent_commits::CommitDescSet;
use crate::filesystem::git::{
    BranchInfo, FileStatus, GitInfo, GitRepo, WorktreeEntry, current_worktree_name_with_repo,
    get_recent_commits_by_duration_with_repo, list_worktrees_with_repo,
};
use crate::filesystem::repo::types::{RepoInfo, detect_repo_with_request_or_root_package};
use crate::filesystem::repo::{RepoIdentity, detect_repo_identity_with_repo};
use crate::request::{RepoDetailRequest, RepoRequest};
use crate::{Result, SniffError};

/// The default commit window for the bare aggregate.
///
/// Matches `sniff repo recent-commits`'s default period so the aggregate's
/// commit families agree with the focused command's default invocation.
const DEFAULT_COMMIT_WINDOW_DAYS: i64 = 3;

/// Every repository fact the bare `sniff repo --json` aggregate needs that is
/// not already present on the detection pass's `FilesystemInfo`.
///
/// Produced by [`observe_repo_aggregate`], consumed as a pure projection.
#[derive(Debug, Clone)]
pub struct RepoAggregate {
    pub identity: RepoIdentity,
    /// The detected repository, or the root-package fallback for a
    /// single-package repo that yielded no `RepoInfo`.
    pub repo: Option<RepoInfo>,
    /// Branches with ahead/behind measured against each branch's **upstream**.
    ///
    /// A different fact from `GitInfo.branches`, whose `LocalBranchInfo`
    /// counts are measured against `HEAD`.
    pub branches: Vec<BranchInfo>,
    /// Worktrees in name order, carrying `is_current` / `is_detached`.
    ///
    /// A different shape from `GitInfo.worktrees`, which is a `HashMap` keyed
    /// by branch and so has no stable order and no current/detached flags.
    pub worktrees: Vec<WorktreeEntry>,
    pub current_worktree: Option<String>,
    pub has_merge_conflict: bool,
    /// One history observation. All three commit-family projections
    /// (`recent_commits`, `source_code_changes`, `documentation_changes`) are
    /// filters over this set rather than three separate history walks.
    pub commits: CommitDescSet,
}

/// Observe the facts bare `sniff repo --json` needs beyond its detection pass.
///
/// `filesystem` is the already-detected result for `dir`. Passing it is what
/// makes this cheap: the `GitInfo` supplies working-tree scope and conflict
/// state, and the `RepoInfo` supplies the package catalog that commit
/// attribution would otherwise re-detect.
///
/// ## Errors
///
/// Returns [`SniffError::NotARepository`] when `dir` is not inside a git
/// repository, and [`SniffError::SystemInfo`] when `filesystem` carries a
/// `GitInfo` whose detection request did not collect per-file changes — see
/// Notes.
///
/// ## Notes
///
/// **Precondition:** the caller's detection request must be at or above
/// `GitRequest::full()`, the floor at which `include_file_changes` is set. A
/// request below that leaves `GitInfo.file_changes` empty, which is
/// indistinguishable from a clean tree by inspection alone. Rather than
/// silently emit empty scope buckets for a dirty repository, this rejects the
/// combination it *can* detect: a status that reports dirty while carrying no
/// file changes.
pub fn observe_repo_aggregate(
    dir: &Path,
    filesystem: Option<&FilesystemInfo>,
) -> Result<RepoAggregate> {
    // Absolutize before discovery: gix reports a workdir relative to the
    // discovery path, so discovering from the CLI's default "." would yield a
    // relative `repo_root` and surface as a relative `structure.root`. This is
    // the same guard `git::api::repo_root` applies, kept here because the
    // aggregate discovers once and reuses that handle for every fact.
    let dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let dir = dir.as_path();
    let git_repo =
        GitRepo::discover(dir)?.ok_or_else(|| SniffError::NotARepository(dir.to_path_buf()))?;

    let detected_git = filesystem.and_then(|fs| fs.git.as_ref());
    if let Some(git) = detected_git {
        ensure_detailed_status(git)?;
    }

    // A single-package repo (a bare `Cargo.toml` / `package.json` with no
    // workspace) yields no `RepoInfo` from the detection pass, which would
    // leave every repo-wide fact empty. Recover the root package's facts.
    let repo = match filesystem.and_then(|fs| fs.repo.as_ref()) {
        Some(repo) => Some(repo.clone()),
        None => detect_repo_with_request_or_root_package(
            git_repo.repo_root(),
            &RepoRequest::focused(RepoDetailRequest::all()),
        )?,
    };

    let file_changes = detected_git.map(|git| git.file_changes.as_slice()).unwrap_or(&[]);

    let commits = get_recent_commits_by_duration_with_repo(
        &git_repo,
        Duration::days(DEFAULT_COMMIT_WINDOW_DAYS),
        &format!("last {DEFAULT_COMMIT_WINDOW_DAYS}d"),
        repo.as_ref(),
    )?;

    Ok(RepoAggregate {
        identity: detect_repo_identity_with_repo(&git_repo)?,
        branches: git_repo.branch_info(false)?,
        worktrees: list_worktrees_with_repo(&git_repo)
            .map_err(|e| SniffError::git("list_worktrees", GitAggregateError(e.to_string())))?,
        current_worktree: current_worktree_name_with_repo(&git_repo),
        has_merge_conflict: file_changes
            .iter()
            .any(|change| change.status == FileStatus::Conflicted),
        repo,
        commits,
    })
}

/// Reject a `GitInfo` whose request did not collect per-file changes.
///
/// A dirty status with no `file_changes` can only mean the request was below
/// `GitRequest::full()`; every scope bucket projected from it would be empty
/// and wrong. A clean tree legitimately has no file changes and passes.
fn ensure_detailed_status(git: &GitInfo) -> Result<()> {
    let reports_dirty = git.status.as_ref().is_some_and(|status| status.is_dirty);
    if reports_dirty && git.file_changes.is_empty() {
        return Err(SniffError::SystemInfo {
            domain: "repo_aggregate",
            message: "detected git status reports a dirty tree but carries no file changes; \
                      the aggregate needs a GitRequest at or above full() so that \
                      include_file_changes is set"
                .to_string(),
        });
    }
    Ok(())
}

/// Adapts `list_worktrees`'s boxed error into the `std::error::Error` bound
/// [`SniffError::git`] requires.
#[derive(Debug)]
struct GitAggregateError(String);

impl std::fmt::Display for GitAggregateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GitAggregateError {}

/// Repository-relative paths in `file_changes` that belong to `scope`.
///
/// The scope predicate and the first-wins-by-path deduplication reproduce
/// `blast_radius::collect_working_tree_paths` exactly (Phase 2 sub-spec, C1),
/// which is what lets the aggregate project all four buckets from the one
/// collection detection already made instead of walking status four more times.
/// `file_changes` order is preserved; callers that need sorting sort after.
pub fn scope_paths(
    file_changes: &[crate::filesystem::git::FileChange],
    scope: crate::filesystem::blast_radius::ChangeScope,
) -> Vec<std::path::PathBuf> {
    use crate::filesystem::blast_radius::ChangeScope;

    let mut seen = std::collections::HashSet::new();
    let mut paths = Vec::new();
    for change in file_changes {
        let include = matches!(
            (scope, change.status),
            (ChangeScope::Dirty, _)
                | (ChangeScope::Staged, FileStatus::Staged | FileStatus::Both)
                | (
                    ChangeScope::Unstaged,
                    FileStatus::Modified | FileStatus::Both | FileStatus::Conflicted,
                )
                | (ChangeScope::Untracked, FileStatus::Untracked)
        );
        if include && seen.insert(change.path.clone()) {
            paths.push(change.path.clone());
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::blast_radius::ChangeScope;
    use crate::filesystem::git::{FileAction, FileChange, GitConfig, RepoStatus};
    use std::path::PathBuf;

    fn change(path: &str, status: FileStatus) -> FileChange {
        FileChange {
            path: PathBuf::from(path),
            status,
            action: FileAction::Modified,
            lines_added: 0,
            lines_removed: 0,
        }
    }

    fn git_info(file_changes: Vec<FileChange>, is_dirty: bool) -> GitInfo {
        GitInfo {
            repo_root: PathBuf::from("/tmp/repo"),
            org: None,
            repo: None,
            current_branch: Some("main".to_string()),
            head_id: None,
            branches: Vec::new(),
            in_worktree: false,
            base_repo_root: None,
            recent: Vec::new(),
            status: Some(RepoStatus {
                is_dirty,
                ..Default::default()
            }),
            remotes: Vec::new(),
            worktrees: std::collections::HashMap::new(),
            config: GitConfig::default(),
            tracking: Vec::new(),
            file_changes,
        }
    }

    /// The C1 truth table. `Dirty` takes every status including `Untracked`;
    /// `Unstaged` includes `Conflicted`; `Staged` does not.
    #[test]
    fn scope_predicate_matches_the_c1_truth_table() {
        let changes = vec![
            change("staged.rs", FileStatus::Staged),
            change("modified.rs", FileStatus::Modified),
            change("both.rs", FileStatus::Both),
            change("untracked.rs", FileStatus::Untracked),
            change("conflicted.rs", FileStatus::Conflicted),
        ];

        let names = |scope| -> Vec<String> {
            scope_paths(&changes, scope)
                .iter()
                .map(|p| p.display().to_string())
                .collect()
        };

        assert_eq!(
            names(ChangeScope::Dirty),
            vec![
                "staged.rs",
                "modified.rs",
                "both.rs",
                "untracked.rs",
                "conflicted.rs"
            ]
        );
        assert_eq!(names(ChangeScope::Staged), vec!["staged.rs", "both.rs"]);
        assert_eq!(
            names(ChangeScope::Unstaged),
            vec!["modified.rs", "both.rs", "conflicted.rs"]
        );
        assert_eq!(names(ChangeScope::Untracked), vec!["untracked.rs"]);
    }

    /// Deduplication is first-wins by path and preserves `file_changes` order,
    /// so a path staged *and* modified appears once, at its first position.
    #[test]
    fn scope_paths_dedups_first_wins_preserving_order() {
        let changes = vec![
            change("z.rs", FileStatus::Staged),
            change("a.rs", FileStatus::Modified),
            change("z.rs", FileStatus::Modified),
        ];
        let paths = scope_paths(&changes, ChangeScope::Dirty);
        assert_eq!(
            paths,
            vec![PathBuf::from("z.rs"), PathBuf::from("a.rs")],
            "order must follow file_changes, not sort"
        );
    }

    #[test]
    fn clean_tree_without_file_changes_is_a_valid_precondition() {
        assert!(ensure_detailed_status(&git_info(Vec::new(), false)).is_ok());
    }

    /// The precondition R2/C1 exists for: a below-`full()` request leaves
    /// `file_changes` empty, which would silently project four empty buckets
    /// for a repository that is in fact dirty.
    #[test]
    fn dirty_status_without_file_changes_is_rejected_not_projected_empty() {
        let err = ensure_detailed_status(&git_info(Vec::new(), true))
            .expect_err("dirty-but-no-file-changes must not be projected as empty buckets");
        assert!(
            err.to_string().contains("full()"),
            "error should name the request floor it needs, got: {err}"
        );
    }

    #[test]
    fn dirty_status_with_file_changes_passes() {
        let git = git_info(vec![change("a.rs", FileStatus::Modified)], true);
        assert!(ensure_detailed_status(&git).is_ok());
    }

    mod work_counts {
        //! The R2 acceptance evidence: bare `sniff repo --json` performs one
        //! status walk and one repository discovery *in total*, where it
        //! previously performed one of each in detection plus eight more
        //! afterwards (four scopes x two path kinds).
        //!
        //! The claim is proven as a sum of two independently measured arms —
        //! detection's git stage, then [`observe_repo_aggregate`] — rather than
        //! by measuring `detect_filesystem_with_request` as a whole. That is
        //! deliberate: the filesystem planner runs each domain on a
        //! `std::thread::scope` thread that installs no collector, so every
        //! counter recorded inside the git stage is dropped when measured
        //! through the planner. Summing the arms measures the same work without
        //! depending on that gap being closed first.

        use super::*;
        use crate::filesystem::{FilesystemRequest, detect_filesystem_with_request};
        use crate::performance::{counters, testing::measure};
        use crate::request::GitRequest;
        use tempfile::TempDir;

        /// A repository with one file per working-tree scope, so a projection
        /// that silently dropped a bucket would show up as an empty bucket.
        fn fixture() -> TempDir {
            let dir = TempDir::new().unwrap();
            let repo = git2::Repository::init(dir.path()).unwrap();
            let sig = git2::Signature::now("Test", "test@example.com").unwrap();

            std::fs::write(dir.path().join("tracked.rs"), "fn a() {}\n").unwrap();
            std::fs::write(dir.path().join("staged.rs"), "fn b() {}\n").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("tracked.rs")).unwrap();
            index.add_path(Path::new("staged.rs")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            {
                let tree = repo.find_tree(tree_id).unwrap();
                repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                    .unwrap();
            }

            // unstaged, staged, and untracked, respectively.
            std::fs::write(dir.path().join("tracked.rs"), "fn a() { }\n").unwrap();
            std::fs::write(dir.path().join("staged.rs"), "fn b() { }\n").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("staged.rs")).unwrap();
            index.write().unwrap();
            std::fs::write(dir.path().join("untracked.rs"), "fn c() {}\n").unwrap();

            dir
        }

        /// The request bare `sniff repo --json` builds: `GitRequest::full()`,
        /// which is the floor at which `include_file_changes` is set (C1).
        fn aggregate_request() -> FilesystemRequest {
            FilesystemRequest::new()
                .git(GitRequest::full().commit_count(10))
                .without_file_inventory()
        }

        /// Arm 1 — detection's git stage walks status exactly once. Measured
        /// directly rather than through the planner; see the module note.
        #[test]
        fn detection_git_stage_walks_status_once() {
            let dir = fixture();
            let (_, counts) = measure(|| {
                let repo = GitRepo::discover(dir.path())
                    .expect("discovery succeeds")
                    .expect("fixture is a repository");
                repo.detect_with_request(&GitRequest::full().commit_count(10))
                    .expect("git detection succeeds")
            });

            assert_eq!(
                counts.get(counters::GIT_STATUS_WALKS),
                1,
                "counters: {:?}",
                counts.all()
            );
        }

        /// Arm 2 — the aggregate observation adds no status walk on top of
        /// detection's one, because every working-tree fact it needs is a
        /// projection over `GitInfo.file_changes`. This is the assertion that
        /// fails if the eight `collect_changed_paths` calls ever come back.
        #[test]
        fn observation_adds_no_status_walk_and_one_discovery() {
            let dir = fixture();
            let fs = detect_filesystem_with_request(dir.path(), &aggregate_request())
                .expect("detection succeeds");

            let (aggregate, counts) = measure(|| {
                observe_repo_aggregate(dir.path(), Some(&fs)).expect("aggregate observation")
            });

            assert_eq!(
                counts.get(counters::GIT_STATUS_WALKS),
                0,
                "scope buckets and has_merge_conflict are projections over the \
                 GitInfo.file_changes detection already collected; counters: {:?}",
                counts.all()
            );
            assert_eq!(
                counts.get(counters::GIT_DISCOVERIES),
                1,
                "one discovery context must serve identity, branches, worktrees, \
                 and history; counters: {:?}",
                counts.all()
            );
            assert_eq!(
                counts.get(counters::REMOTE_REQUESTS),
                0,
                "the aggregate is local-only; counters: {:?}",
                counts.all()
            );
            assert!(
                aggregate.commits.commits.iter().any(|c| !c.hash.is_empty()),
                "the one shared history observation must be populated"
            );
        }

        /// Every working-tree scope bucket is reachable from the projection, so
        /// a bucket silently lost would not read as a legitimately empty one.
        #[test]
        fn projection_reproduces_every_scope_from_detected_changes() {
            use crate::filesystem::blast_radius::ChangeScope;

            let dir = fixture();
            let fs = detect_filesystem_with_request(dir.path(), &aggregate_request())
                .expect("detection succeeds");
            let changes = &fs.git.as_ref().expect("fixture is a repository").file_changes;

            let names = |scope| -> Vec<String> {
                scope_paths(changes, scope)
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect()
            };

            assert_eq!(names(ChangeScope::Staged), vec!["staged.rs"]);
            assert_eq!(names(ChangeScope::Unstaged), vec!["tracked.rs"]);
            assert_eq!(names(ChangeScope::Untracked), vec!["untracked.rs"]);

            let mut dirty = names(ChangeScope::Dirty);
            dirty.sort();
            assert_eq!(dirty, vec!["staged.rs", "tracked.rs", "untracked.rs"]);
        }

        /// The C1 precondition end-to-end: a below-`full()` request leaves
        /// `file_changes` empty for a genuinely dirty tree, and the entry point
        /// must reject that rather than return four empty buckets.
        #[test]
        fn request_below_full_is_rejected_rather_than_projected_empty() {
            let dir = fixture();
            let summary_request = FilesystemRequest::new()
                .git(GitRequest::summary())
                .without_file_inventory();
            let fs = detect_filesystem_with_request(dir.path(), &summary_request)
                .expect("detection succeeds");

            let git = fs.git.as_ref().expect("fixture is a repository");
            assert!(
                git.file_changes.is_empty(),
                "precondition of this test: summary() collects no file changes"
            );

            let result = observe_repo_aggregate(dir.path(), Some(&fs));
            assert!(
                result.is_err(),
                "a dirty tree with no file changes must not yield empty buckets"
            );
        }
    }
}
