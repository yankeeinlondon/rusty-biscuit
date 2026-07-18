//! The library-owned aggregate observation behind bare `sniff repo --json`.
//!
//! Everything bare `sniff repo --json` renders is either already on the
//! `FilesystemInfo` its detection pass produced, or is one of the facts
//! [`RepoAggregate`] supplies here. The CLI therefore discovers no repository,
//! lists no worktrees, queries no branches, reads no conflicts, walks no
//! history, and resolves no cwd-relative context of its own — see the Phase 2
//! sub-spec, contract C2.
//!
//! Cost contract: zero repository discoveries and zero additional status walks
//! per call. Every working-tree scope bucket is a projection over the
//! `GitInfo.file_changes` the detection pass already collected (contract C1),
//! not a fresh observation, and the cwd-relative `context` facts resolve over
//! a single package ownership index (R2.7).

use std::path::Path;
use std::time::Instant;

use crate::filesystem::FilesystemInfo;
use crate::filesystem::git::recent_commits::CommitDescSet;
use crate::filesystem::git::types::GitAggregateEvidence;
use crate::filesystem::git::{BranchInfo, FileStatus, GitInfo, WorktreeEntry};
use crate::filesystem::path_kind::is_source_code_path;
use crate::filesystem::repo::detection::canonicalize_path;
use crate::filesystem::repo::ownership::PackageOwnershipIndex;
use crate::filesystem::repo::types::RepoInfo;
use crate::filesystem::repo::RepoIdentity;
use crate::performance;
use crate::request::{
    FilesystemRequest, GitMetadataRequest, GitRequest, RepoDetailRequest, RepoRequest,
};
use crate::{Result, SniffError};

/// Every repository fact the bare `sniff repo --json` aggregate needs that is
/// not already present on the detection pass's `FilesystemInfo`.
///
/// Produced by [`detect_repo_aggregate`], consumed as a pure projection.
#[derive(Debug, Clone)]
pub struct RepoAggregate {
    pub identity: RepoIdentity,
    /// The detected repository, including the standalone root-package
    /// projection produced by the original filesystem detection pass.
    pub repo: Option<RepoInfo>,
    /// The repo-wide collapse of versions already attributed to packages by
    /// detection. One distinct version is retained; zero or several are not.
    pub version: Option<String>,
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
    /// The cwd-relative `context` block facts, resolved once over a shared
    /// package ownership index so the projection performs no lookups of its
    /// own (R2.7).
    pub context: AggregateCwdContext,
}

/// Opaque result of the dedicated repository-aggregate detection boundary.
///
/// The ordinary filesystem result retains its stable public shapes. The
/// aggregate projection travels beside it, so CLI-specific evidence never
/// becomes a field downstream callers must initialize or serialize.
pub struct RepoAggregateObservation {
    filesystem: FilesystemInfo,
    aggregate: RepoAggregate,
}

impl RepoAggregateObservation {
    /// Split the observation into the ordinary filesystem result and its
    /// repository-aggregate projection.
    pub fn into_parts(self) -> (FilesystemInfo, RepoAggregate) {
        (self.filesystem, self.aggregate)
    }
}

/// The cwd-relative facts behind the aggregate's `context` block.
///
/// Unresolvable facts carry the focused commands' "empty" rendering: an empty
/// string or `false`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AggregateCwdContext {
    /// Name of the package containing the invoking directory.
    pub package: String,
    /// Package-area label for the invoking directory, including the monorepo
    /// directory-structure fallback.
    pub package_area: String,
    /// Package name when inside a package, otherwise the area label; empty
    /// outside a monorepo.
    pub area: String,
    /// Root directory of the package containing the invoking directory.
    pub package_root: String,
    /// Root directory of the containing package area; empty for the `"root"`
    /// area, which has no real directory of its own.
    pub package_area_root: String,
    /// Whether the invoking directory's package area has uncommitted changes.
    pub is_current_package_area_dirty: bool,
    /// Whether those changes include a source-code file.
    pub package_area_has_source_code_changes: bool,
}

/// Detect every fact needed by bare `sniff repo --json` through one repository
/// discovery and return the aggregate beside the stable filesystem result.
///
/// ## Errors
///
/// Returns [`SniffError::NotARepository`] when `dir` is outside a Git
/// repository, and propagates Git or repository-detection failures.
///
/// ## Notes
///
/// The request intentionally uses only approved public metadata controls.
/// Branch, worktree, and file-aware history shapes are collected through a
/// crate-private companion while the original Git handle is still available.
pub fn detect_repo_aggregate(dir: &Path) -> Result<RepoAggregateObservation> {
    let started = Instant::now();
    let request = FilesystemRequest::new()
        .git(
            GitRequest::full().commit_count(10).metadata(
                GitMetadataRequest::none()
                    .remotes(true)
                    .config(true),
            ),
        )
        .repo(RepoRequest::focused(RepoDetailRequest::all()))
        .without_file_inventory();
    let filesystem_started = Instant::now();
    let detected = crate::filesystem::detect_filesystem_for_aggregate(dir, &request)?;
    performance::record_logged_stage(
        "detect.filesystem",
        filesystem_started.elapsed(),
        tracing::Level::INFO,
    );
    performance::record_logged_stage("detect.total", started.elapsed(), tracing::Level::INFO);
    let evidence = detected
        .git
        .as_ref()
        .ok_or_else(|| SniffError::NotARepository(dir.to_path_buf()))?;
    let aggregate = project_repo_aggregate(dir, Some(&detected.filesystem), evidence)?;

    Ok(RepoAggregateObservation {
        filesystem: detected.filesystem,
        aggregate,
    })
}

fn project_repo_aggregate(
    dir: &Path,
    filesystem: Option<&FilesystemInfo>,
    evidence: &GitAggregateEvidence,
) -> Result<RepoAggregate> {
    // Absolutize before discovery: gix reports a workdir relative to the
    // discovery path, so discovering from the CLI's default "." would yield a
    // relative `repo_root` and surface as a relative `structure.root`. This is
    // the same guard `git::api::repo_root` applies, kept here because the
    // aggregate discovers once and reuses that handle for every fact.
    let dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let dir = dir.as_path();
    let detected_git = filesystem
        .and_then(|fs| fs.git.as_ref())
        .ok_or_else(|| SniffError::NotARepository(dir.to_path_buf()))?;
    ensure_detailed_status(detected_git)?;

    let repo = filesystem.and_then(|fs| fs.repo.clone());
    let version = collapse_detected_repo_version(repo.as_ref());

    let file_changes = detected_git.file_changes.as_slice();
    let mut commits = evidence.commits.clone();
    commits.attribute_from_repo(repo.as_ref());

    let context = observe_cwd_context(
        dir,
        repo.as_ref(),
        Some(detected_git),
    );

    Ok(RepoAggregate {
        identity: identity_from_detected(detected_git, repo.as_ref(), version.as_deref()),
        branches: evidence.branches.clone(),
        worktrees: evidence.worktrees.clone(),
        current_worktree: evidence.current_worktree.clone(),
        has_merge_conflict: file_changes
            .iter()
            .any(|change| change.status == FileStatus::Conflicted),
        repo,
        version,
        commits,
        context,
    })
}

fn identity_from_detected(
    git: &GitInfo,
    repo: Option<&RepoInfo>,
    version: Option<&str>,
) -> RepoIdentity {
    let root_package_name = repo
        .and_then(|repo| repo.packages.as_deref().map(|packages| (repo, packages)))
        .and_then(|(repo, packages)| packages.iter().find(|package| package.path == repo.root))
        .map(|package| package.name.clone());
    let name = root_package_name
        .or_else(|| git.repo.clone())
        .or_else(|| {
            git.repo_root
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string());

    RepoIdentity {
        name,
        version: version.map(str::to_string),
        language: None,
        is_monorepo: repo.is_some_and(|repo| repo.is_monorepo),
        package_count: repo
            .filter(|repo| repo.is_monorepo)
            .and_then(|repo| repo.packages.as_ref().map(Vec::len)),
    }
}

/// Collapse versions that package detection already resolved from manifests.
///
/// This intentionally consumes `Package.version` rather than resolving source
/// manifests again: the aggregate needs only the repo-wide string/null value,
/// not the focused `repo version` command's source-attribution detail.
fn collapse_detected_repo_version(repo: Option<&RepoInfo>) -> Option<String> {
    let mut versions = repo?
        .packages
        .as_deref()?
        .iter()
        .filter_map(|package| package.version.as_deref());
    let first = versions.next()?;
    versions.all(|version| version == first).then(|| first.to_string())
}

/// Resolve the cwd-relative `context` facts for one aggregate observation.
///
/// `repo` is the detection pass's `RepoInfo` and `git` its `GitInfo`; both are
/// borrowed, never re-observed. Every lookup shares one ownership index and
/// one canonicalization of `dir`, which is what lets the downstream projection
/// stay counter-free (R2.7).
fn observe_cwd_context(
    dir: &Path,
    repo: Option<&RepoInfo>,
    git: Option<&GitInfo>,
) -> AggregateCwdContext {
    let mut context = AggregateCwdContext::default();
    let Some(repo) = repo else {
        return context;
    };

    let ownership_index =
        PackageOwnershipIndex::from_packages(&repo.root, repo.packages.as_deref().unwrap_or(&[]));
    let dir = canonicalize_path(dir);

    if let Some(pkg) = repo.package_for_dir_with_index(&ownership_index, &dir) {
        context.package = pkg.name.clone();
        context.package_root = pkg.path.display().to_string();
    }
    if let Some(label) = repo.package_area_label_for_dir_with_index(&ownership_index, &dir) {
        context.package_area = label.into_owned();
    }
    if repo.is_monorepo {
        context.area = repo
            .area_for_dir_with_index(&ownership_index, &dir)
            .into_owned();
    }
    if let Some(area) = repo.package_area_for_dir_with_index(&ownership_index, &dir) {
        if area != "root" {
            context.package_area_root = repo.root.join(area).display().to_string();
        }
        let (dirty, has_source_changes) = area_change_facts(repo, area, git);
        context.is_current_package_area_dirty = dirty;
        context.package_area_has_source_code_changes = has_source_changes;
    }
    context
}

/// The `dirty` / `has_source_code_changes` pair for one resolved package
/// area, projected from the already-observed status and file changes.
///
/// A path counts when its components place it under the area path — or, for
/// the `"root"` area, when it sits under no other area. Without a computed
/// status (an identity-only request) both facts are indeterminate and report
/// `false`, mirroring the focused commands.
fn area_change_facts(repo: &RepoInfo, area: &str, git: Option<&GitInfo>) -> (bool, bool) {
    let Some(git) = git else {
        return (false, false);
    };
    let Some(status) = git.status.as_ref() else {
        return (false, false);
    };

    let area_path = (area != "root").then(|| Path::new(area));
    let in_area = |path: &Path| {
        if let Some(area_path) = area_path {
            path.starts_with(area_path)
        } else {
            !repo.packages.as_ref().is_some_and(|packages| {
                packages
                    .iter()
                    .any(|p| {
                        p.package_area != "root"
                            && path.starts_with(Path::new(&p.package_area))
                    })
            })
        }
    };

    let mut dirty = false;
    let mut has_source_changes = false;
    let paths = status
        .dirty
        .iter()
        .map(|d| d.filepath.as_path())
        .chain(
            status
                .untracked
                .iter()
                .map(|u| u.filepath.as_path()),
        )
        .chain(
            git.file_changes
                .iter()
                .map(|change| change.path.as_path()),
        );
    for path in paths {
        if in_area(path) {
            dirty = true;
            if is_source_code_path(path) {
                has_source_changes = true;
            }
        }
    }
    (dirty, has_source_changes)
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
    use crate::filesystem::repo::Package;
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

    fn empty_evidence() -> GitAggregateEvidence {
        GitAggregateEvidence {
            branches: Vec::new(),
            worktrees: Vec::new(),
            current_worktree: None,
            commits: CommitDescSet {
                commits: Vec::new(),
                period_label: "last 3d".to_string(),
                repo_root: PathBuf::new(),
                packages: None,
            },
        }
    }

    fn repo_with_versions(versions: &[Option<&str>]) -> RepoInfo {
        RepoInfo {
            packages: Some(
                versions
                    .iter()
                    .enumerate()
                    .map(|(index, version)| Package {
                        name: format!("package-{index}"),
                        version: version.map(str::to_string),
                        ..Package::default()
                    })
                    .collect(),
            ),
            ..RepoInfo::default()
        }
    }

    #[test]
    fn detected_versions_collapse_only_when_uniform() {
        let uniform = repo_with_versions(&[Some("1.2.3"), Some("1.2.3")]);
        assert_eq!(
            collapse_detected_repo_version(Some(&uniform)),
            Some("1.2.3".to_string())
        );

        let variant = repo_with_versions(&[Some("1.2.3"), Some("2.0.0")]);
        assert_eq!(collapse_detected_repo_version(Some(&variant)), None);
    }

    #[test]
    fn missing_detected_versions_do_not_create_an_aggregate_version() {
        let missing = repo_with_versions(&[None, None]);
        assert_eq!(collapse_detected_repo_version(Some(&missing)), None);
        assert_eq!(collapse_detected_repo_version(None), None);
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
        //! Library-side R2 evidence. The CLI command-path test measures
        //! detection and aggregate completion together; these tests pin the
        //! underlying status and linked-worktree behavior in isolation.

        use super::*;
        use crate::filesystem::{FilesystemRequest, detect_filesystem_with_request};
        use crate::filesystem::git::GitRepo;
        use crate::performance::{counters, testing::measure};
        use crate::request::{GitMetadataRequest, GitRequest};
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

        /// Add a real linked checkout using libgit2's portable fixture API.
        fn linked_fixture() -> (TempDir, PathBuf) {
            let dir = fixture();
            let repo = git2::Repository::open(dir.path()).unwrap();
            let linked_path = dir.path().join("current-linked");
            repo.worktree("current-linked", &linked_path, None)
                .unwrap();
            (dir, linked_path)
        }

        /// Detection's git stage walks status exactly once.
        #[test]
        fn detection_git_stage_walks_status_once() {
            let dir = fixture();
            let (_, counts) = measure(|| {
                let repo = GitRepo::discover(dir.path())
                    .expect("discovery succeeds")
                    .expect("fixture is a repository");
                repo.detect_with_request(
                    &GitRequest::full()
                        .commit_count(10)
                        .metadata(GitMetadataRequest::none().config(true)),
                )
                    .expect("git detection succeeds")
            });

            assert_eq!(
                counts.get(counters::GIT_STATUS_WALKS),
                1,
                "counters: {:?}",
                counts.all()
            );
        }

        /// A linked checkout used to walk status once for detailed detection
        /// and again while projecting the current worktree. Focused aggregate
        /// metadata keeps the command-wide bound at one discovery and one walk.
        #[test]
        fn linked_worktree_aggregate_walks_status_and_discovers_once() {
            let (_dir, linked_path) = linked_fixture();
            let (_, counts) = measure(|| {
                detect_repo_aggregate(&linked_path)
                    .expect("aggregate detection succeeds")
            });

            assert_eq!(
                counts.get(counters::GIT_STATUS_WALKS),
                1,
                "linked aggregate status must be observed exactly once: {:?}",
                counts.all()
            );
            assert_eq!(
                counts.get(counters::GIT_DISCOVERIES),
                1,
                "linked aggregate detection must share one discovery: {:?}",
                counts.all()
            );
        }

        /// Splitting the opaque result performs no repository observation.
        #[test]
        fn opaque_result_projection_adds_no_status_walk_or_discovery() {
            let dir = fixture();
            let observation = detect_repo_aggregate(dir.path()).expect("detection succeeds");

            let ((_, aggregate), counts) = measure(|| observation.into_parts());

            assert_eq!(
                counts.get(counters::GIT_STATUS_WALKS),
                0,
                "scope buckets and has_merge_conflict are projections over the \
                 GitInfo.file_changes detection already collected; counters: {:?}",
                counts.all()
            );
            assert_eq!(
                counts.get(counters::GIT_DISCOVERIES),
                0,
                "identity, branches, worktrees, and history must come from the \
                 original detection evidence; counters: {:?}",
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
            let (fs, _) = detect_repo_aggregate(dir.path())
                .expect("detection succeeds")
                .into_parts();
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

            let evidence = empty_evidence();
            let result = project_repo_aggregate(dir.path(), Some(&fs), &evidence);
            assert!(
                result.is_err(),
                "a dirty tree with no file changes must not yield empty buckets"
            );
        }
    }

    mod cwd_context {
        //! The R2.7 counterpart on the observation side: the seven
        //! cwd-relative `context` facts resolve once, here, over a single
        //! package ownership index — never inside the CLI projection.

        use super::*;
        use crate::filesystem::repo::types::Package;
        use crate::filesystem::{FilesystemRequest, detect_filesystem_with_request};
        use crate::request::{GitMetadataRequest, GitRequest};
        use tempfile::TempDir;

        /// A Cargo workspace with one member in each of two areas plus a
        /// root-level member, committed clean.
        fn workspace_fixture() -> (TempDir, PathBuf) {
            let dir = TempDir::new().unwrap();
            let root = dir.path();
            std::fs::write(
                root.join("Cargo.toml"),
                "[workspace]\nmembers = [\"alpha/pkg-a\", \"beta/pkg-b\", \"rootpkg\"]\n",
            )
            .unwrap();
            for (area, name) in [("alpha", "pkg-a"), ("beta", "pkg-b"), (".", "rootpkg")] {
                let pkg_dir = root.join(area).join(name);
                std::fs::create_dir_all(pkg_dir.join("src")).unwrap();
                std::fs::write(
                    pkg_dir.join("Cargo.toml"),
                    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
                )
                .unwrap();
                std::fs::write(pkg_dir.join("src/lib.rs"), "pub fn f() {}\n").unwrap();
            }

            let repo = git2::Repository::init(root).unwrap();
            let sig = git2::Signature::now("Test", "test@example.com").unwrap();
            let mut index = repo.index().unwrap();
            index
                .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
                .unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            {
                let tree = repo.find_tree(tree_id).unwrap();
                repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                    .unwrap();
            }
            let root = root.to_path_buf();
            (dir, root)
        }

        fn detect(root: &Path) -> (FilesystemInfo, RepoAggregate) {
            detect_repo_aggregate(root)
                .expect("detection succeeds")
                .into_parts()
        }

        #[test]
        fn observation_resolves_context_facts_for_the_invoking_directory() {
            let (_temp, root) = workspace_fixture();
            let (_, aggregate) = detect(&root.join("alpha/pkg-a"));
            let context = &aggregate.context;

            assert_eq!(context.package, "pkg-a");
            assert_eq!(context.package_area, "alpha");
            assert_eq!(context.area, "pkg-a");
            assert!(
                Path::new(&context.package_root).ends_with(Path::new("alpha").join("pkg-a")),
                "package_root must be the member directory: {}",
                context.package_root
            );
            assert_eq!(
                context.package_area_root,
                aggregate
                    .repo
                    .as_ref()
                    .expect("workspace detection yields a repo")
                    .root
                    .join("alpha")
                    .display()
                    .to_string()
            );
            assert!(!context.is_current_package_area_dirty);
            assert!(!context.package_area_has_source_code_changes);
        }

        #[test]
        fn area_changes_attribute_to_the_owning_area() {
            let (_temp, root) = workspace_fixture();
            std::fs::write(root.join("alpha/pkg-a/src/lib.rs"), "pub fn f() { }\n").unwrap();
            let (_, in_a) = detect(&root.join("alpha/pkg-a"));
            assert!(in_a.context.is_current_package_area_dirty);
            assert!(in_a.context.package_area_has_source_code_changes);

            let (_, in_b) = detect(&root.join("beta/pkg-b"));
            assert!(!in_b.context.is_current_package_area_dirty);
            assert!(!in_b.context.package_area_has_source_code_changes);
        }

        #[test]
        fn documentation_changes_are_not_source_changes() {
            let (_temp, root) = workspace_fixture();
            std::fs::write(root.join("alpha/pkg-a/notes.md"), "# notes\n").unwrap();
            let (_, in_a) = detect(&root.join("alpha/pkg-a"));
            assert!(in_a.context.is_current_package_area_dirty);
            assert!(!in_a.context.package_area_has_source_code_changes);
        }

        #[test]
        fn root_area_excludes_paths_under_named_areas() {
            let (_temp, root) = workspace_fixture();
            std::fs::write(root.join("alpha/pkg-a/src/lib.rs"), "pub fn f() { }\n").unwrap();
            let (_, at_root_pkg) = detect(&root.join("rootpkg"));
            assert_eq!(at_root_pkg.context.package, "rootpkg");
            assert_eq!(at_root_pkg.context.package_area, "root");
            assert!(!at_root_pkg.context.is_current_package_area_dirty);
        }

        #[test]
        fn root_area_counts_paths_outside_named_areas() {
            let (_temp, root) = workspace_fixture();
            std::fs::write(root.join("rootpkg/src/lib.rs"), "pub fn f() { }\n").unwrap();
            let (_, at_root_pkg) = detect(&root.join("rootpkg"));
            assert!(at_root_pkg.context.is_current_package_area_dirty);
            assert!(at_root_pkg.context.package_area_has_source_code_changes);
            assert_eq!(at_root_pkg.context.package_area_root, "");
        }

        #[test]
        fn area_membership_compares_whole_path_components() {
            let (_temp, root) = workspace_fixture();
            std::fs::create_dir_all(root.join("alpha2")).unwrap();
            std::fs::write(root.join("alpha2/collision.rs"), "pub fn collision() {}\n")
                .unwrap();
            let (_, in_alpha) = detect(&root.join("alpha/pkg-a"));
            assert!(!in_alpha.context.is_current_package_area_dirty);
            assert!(!in_alpha.context.package_area_has_source_code_changes);

            let (_, at_root_pkg) = detect(&root.join("rootpkg"));
            assert!(at_root_pkg.context.is_current_package_area_dirty);
            assert!(at_root_pkg.context.package_area_has_source_code_changes);
        }

        #[test]
        fn context_facts_default_without_repo_detection() {
            let (_temp, root) = workspace_fixture();
            let request = FilesystemRequest::new()
                .git(
                    GitRequest::full().metadata(
                        GitMetadataRequest::none()
                            .remotes(true)
                            .config(true),
                    ),
                )
                .without_repo()
                .without_docs()
                .without_formatting()
                .without_file_inventory();
            let filesystem = detect_filesystem_with_request(&root, &request)
                .expect("detection succeeds");

            let evidence = empty_evidence();
            let aggregate = project_repo_aggregate(&root, Some(&filesystem), &evidence)
                .expect("observation succeeds");

            assert_eq!(aggregate.context, AggregateCwdContext::default());
        }

        #[test]
        fn area_fact_stays_empty_outside_a_monorepo() {
            let dir = TempDir::new().unwrap();
            let root = dir.path();
            let repo = git2::Repository::init(root).unwrap();
            let sig = git2::Signature::now("Test", "test@example.com").unwrap();
            let tree_id = repo.index().unwrap().write_tree().unwrap();
            {
                let tree = repo.find_tree(tree_id).unwrap();
                repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                    .unwrap();
            }

            let (mut fs, _) = detect(root);
            fs.repo = Some(RepoInfo {
                is_monorepo: false,
                root: root.to_path_buf(),
                packages: Some(vec![Package {
                    path: root.to_path_buf(),
                    relative: String::new(),
                    package_area: "root".to_string(),
                    name: "solo".to_string(),
                    ..Package::default()
                }]),
                ..RepoInfo::default()
            });

            let evidence = empty_evidence();
            let aggregate = project_repo_aggregate(root, Some(&fs), &evidence)
                .expect("observation succeeds");
            let context = &aggregate.context;

            assert_eq!(context.package, "solo");
            assert_eq!(context.package_area, "root");
            assert_eq!(context.area, "");
            assert_eq!(context.package_root, root.display().to_string());
            assert_eq!(context.package_area_root, "");
            assert!(!context.is_current_package_area_dirty);
            assert!(!context.package_area_has_source_code_changes);
        }
    }
}
