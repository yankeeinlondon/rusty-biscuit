//! Restored tool-written prefixes test status errors and metadata preservation.

use super::{commit_files, discover, init_repo, MAIN_BRANCH};
use crate::{SniffError, request::GitRequest};
use crate::filesystem::git::{self, ChangedBase, RecentCommits, RecentCommitsOptions};
use std::path::{Path, PathBuf};

const PREFIXES: &[usize] = &[0, 1, 4, 8, 11, 12, 13, 16, 20, 31, 84, 103];

fn git_error<T>(label: &str, read: impl FnOnce() -> crate::Result<T>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(read));
    assert!(result.is_ok(), "{label}: no unwind may escape");
    assert!(matches!(result.unwrap(), Err(SniffError::Git { .. })), "{label}: typed Git error required");
}

fn metadata_controls(path: &Path) {
    for branches in [discover(path).try_branches().unwrap(), discover(path).branches()] {
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
        assert_eq!((branches[0].ahead, branches[0].behind), (0, 0));
    }
    let branches = discover(path).branch_info(false).unwrap();
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].name, "main");
    assert!(branches[0].current);
    assert!(discover(path).try_tracking_status().unwrap().is_empty());
    assert!(discover(path).tracking_status().is_empty());
    let commits = discover(path).recent_commits(1);
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].message, "index fixture");
    let repo = discover(path);
    let listed = git::list_worktrees_with_repo(&repo).unwrap();
    assert_eq!(listed.len(), 1);
    assert!(!listed[0].is_current, "fixture is outside the process working directory");
    assert_eq!(listed[0].branch.as_deref(), Some("main"));
    assert_eq!(git::current_worktree_name_with_repo(&discover(path)), None);
    assert_eq!(git::api::preferred_remote_url_with_repo(&discover(path)), None);
    let identity = discover(path).detect_with_request(&GitRequest::identity()).unwrap();
    assert_eq!(identity.current_branch.as_deref(), Some("main"));
    assert!(identity.status.is_none());
}

fn assert_readers_reject(path: &Path) {
    use git::status;
    git_error("changed omitted", || discover(path).changed_paths(ChangedBase::Omitted));
    git_error("changed explicit HEAD", || discover(path).changed_paths(ChangedBase::Reference("HEAD".into())));
    git_error("file changes", || discover(path).file_changes());
    git_error("repo status", || discover(path).repo_status());
    git_error("public conflicts", || discover(path).merge_conflicts());
    git_error("rich direct", || status::get_repo_status_with_changes(&gix::open(path).unwrap(), true));
    git_error("path direct", || status::collect_path_status(&gix::open(path).unwrap()));
    git_error("dirty direct", || status::is_repo_dirty(&gix::open(path).unwrap()));
    git_error("counts direct", || status::get_repo_status_counts(&gix::open(path).unwrap()));
    git_error("category counts direct", || status::get_repo_status_counts_detailed(&gix::open(path).unwrap()));
    git_error("conflicts direct", || status::detect_merge_conflicts_fallible(&gix::open(path).unwrap()));
    assert!(status::detect_merge_conflicts(&gix::open(path).unwrap()).is_empty());
    let mut counts = GitRequest::minimal();
    counts.commit_count = 1;
    for request in [GitRequest::minimal(), GitRequest::summary(), counts, GitRequest::full()] {
        git_error("status request", || discover(path).detect_with_request(&request));
    }
    assert!(discover(path).worktrees().unwrap().is_empty(), "no linked worktrees means no detail status read");
    git_error("structured commits", || RecentCommits::collect(&discover(path), &RecentCommitsOptions::new()));
    git_error("aggregate", || discover(path).observe_aggregate_evidence(None));
    let repo = gix::open(path).unwrap();
    let tip = repo.head_id().unwrap().detach();
    git_error("tip diff direct", || git::discovery::tip_diff_paths(&repo, tip, None));
    assert!(git::merge_conflicts::merge_conflicts_between(&repo, tip, tip).unwrap().is_empty(), "committed-only merge ignores live index corruption");
    git_error("commit files direct", || git::discovery::get_commit_files_fallible(&repo, tip));
    git_error("commit files public", || git::commit_files_at(path, &tip.to_string()));
    assert!(git::get_commit_files(&repo, tip).is_empty(), "compatibility suppresses unreadable attribute index");
    git_error("path history direct", || git::discovery::get_commits_for_path_fallible(&repo, "leaf.rs", git::PathHistoryOptions::new(1), None));
    git_error("path history public", || git::commits_for_path_at(path, "leaf.rs", git::PathHistoryOptions::new(1)));
    assert!(git::get_commits_for_path(&repo, "leaf.rs", git::PathHistoryOptions::new(1)).commits.is_empty());
    let mut repo = gix::open(path).unwrap();
    git::open::configure_cache(&mut repo);
    metadata_controls(path);
}

#[test]
fn truncated_indexes_are_typed_errors_across_readers_and_cache_consumers() {
    let dir = tempfile::TempDir::new().unwrap();
    init_repo(dir.path());
    commit_files(dir.path(), "index fixture", MAIN_BRANCH, &[("leaf.rs", "fn original() {}\n")]);
    std::fs::write(dir.path().join("leaf.rs"), "fn edited() {}\n").unwrap();
    let index = dir.path().join(".git/index");
    let original = std::fs::read(&index).unwrap();
    assert_eq!(original.len(), 104, "known libgit2-written fixture index");
    metadata_controls(dir.path());
    assert_eq!(discover(dir.path()).changed_paths(ChangedBase::Omitted).unwrap(), vec![PathBuf::from("leaf.rs")]);
    let changes = discover(dir.path()).file_changes().unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, PathBuf::from("leaf.rs"));
    assert_eq!(changes[0].status, git::FileStatus::Modified);
    assert_eq!(git::status::get_repo_status_counts_detailed(&gix::open(dir.path()).unwrap()).unwrap(), (true, 0, 1, 0));
    assert_eq!(discover(dir.path()).changed_paths(ChangedBase::Reference("HEAD".into())).unwrap(), vec![PathBuf::from("leaf.rs")]);
    assert!(git::status::is_repo_dirty(&gix::open(dir.path()).unwrap()).unwrap());
    assert_eq!(git::status::get_repo_status_counts(&gix::open(dir.path()).unwrap()).unwrap(), (true, 1));
    let rich = discover(dir.path()).repo_status().unwrap();
    assert!(rich.is_dirty);
    assert_eq!((rich.staged_count, rich.unstaged_count, rich.untracked_count), (0, 1, 0));
    for request in [GitRequest::minimal(), GitRequest::summary()] {
        let status = discover(dir.path()).detect_with_request(&request).unwrap().status.unwrap();
        assert!(status.is_dirty);
        assert_eq!((status.staged_count, status.unstaged_count, status.untracked_count), (0, 0, 0));
    }
    let mut counts = GitRequest::minimal();
    counts.commit_count = 1;
    assert!(!counts.is_minimal(), "this request reaches the category-count reader");
    for request in [counts, GitRequest::full()] {
        let status = discover(dir.path()).detect_with_request(&request).unwrap().status.unwrap();
        assert!(status.is_dirty);
        assert_eq!((status.staged_count, status.unstaged_count, status.untracked_count), (0, 1, 0));
    }
    assert!(discover(dir.path()).merge_conflicts().unwrap().is_empty());
    assert!(discover(dir.path()).worktrees().unwrap().is_empty());
    assert_eq!(RecentCommits::collect(&discover(dir.path()), &RecentCommitsOptions::new()).unwrap().len(), 1);
    assert_eq!(discover(dir.path()).observe_aggregate_evidence(None).unwrap().commits.len(), 1);
    let tip = discover(dir.path()).head_id().unwrap();
    let gix = gix::open(dir.path()).unwrap();
    assert_eq!(git::discovery::tip_diff_paths(&gix, gix.head_id().unwrap().detach(), None).unwrap(), vec![gix::bstr::BString::from("leaf.rs")]);
    assert_eq!(git::commit_files_at(dir.path(), &tip).unwrap(), vec![(PathBuf::from("leaf.rs"), git::DeltaKind::Added)]);
    let history = git::commits_for_path_at(dir.path(), "leaf.rs", git::PathHistoryOptions::new(1)).unwrap();
    assert_eq!(history.commits.len(), 1);
    assert_eq!(history.commits[0].message, "index fixture");
    let mut captured_cache = git::open::tree_diff_cache(&gix).unwrap();
    let commit = gix.head_id().unwrap().detach();
    for &length in PREFIXES {
        std::fs::write(&index, &original).unwrap();
        std::fs::write(&index, &original[..length]).unwrap();
        assert_readers_reject(dir.path());
        assert_eq!(git::discovery::get_commit_files_with_cache_fallible(&gix, commit, &mut captured_cache).unwrap(), vec![(PathBuf::from("leaf.rs"), git::DeltaKind::Added)]);
        assert_eq!(git::get_commit_files_with_cache(&gix, commit, &mut captured_cache), vec![(PathBuf::from("leaf.rs"), git::DeltaKind::Added)]);
        let enriched = git::discovery::committed_file_changes_with_cache(&gix, commit, &mut captured_cache).unwrap();
        assert_eq!(enriched.len(), 1);
        assert_eq!(enriched[0].path, PathBuf::from("leaf.rs"));
        assert_eq!(enriched[0].kind, git::DeltaKind::Added);

    }
    std::fs::write(&index, original).unwrap();
    assert_eq!(discover(dir.path()).changed_paths(ChangedBase::Omitted).unwrap(), vec![PathBuf::from("leaf.rs")]);
}

#[test]
fn linked_indexes_contain_cache_and_status_failures_with_absence_and_empty_controls() {
    let dir = tempfile::TempDir::new().unwrap();
    init_repo(dir.path());
    commit_files(dir.path(), "index fixture", MAIN_BRANCH, &[("leaf.rs", "fn original() {}\n")]);
    let linked_parent = tempfile::TempDir::new().unwrap();
    let linked = linked_parent.path().join("linked");
    git2::Repository::open(dir.path()).unwrap().worktree("linked", &linked, None).unwrap();
    std::fs::write(linked.join("leaf.rs"), "fn edited() {}\n").unwrap();
    let index = discover(&linked).git_dir().join("index");
    let original = std::fs::read(&index).unwrap();
    assert_eq!(discover(&linked).changed_paths(ChangedBase::Omitted).unwrap(), vec![PathBuf::from("leaf.rs")]);
    let details = discover(dir.path()).worktrees().unwrap();
    assert_eq!(details.len(), 1);
    assert!(details.values().next().unwrap().dirty);
    for &length in PREFIXES {
        std::fs::write(&index, &original).unwrap();
        std::fs::write(&index, &original[..length]).unwrap();
        git_error("linked changed", || discover(&linked).changed_paths(ChangedBase::Omitted));
        git_error("main reads linked details", || discover(dir.path()).worktrees());
        assert_eq!(discover(dir.path()).changed_paths(ChangedBase::Omitted).unwrap(), Vec::<PathBuf>::new());
    }
    std::fs::write(&index, &original).unwrap();
    std::fs::remove_file(&index).unwrap();
    assert_eq!(discover(&linked).changed_paths(ChangedBase::Omitted).unwrap(), vec![PathBuf::from("leaf.rs")]);
    assert!(discover(dir.path()).worktrees().unwrap().values().next().unwrap().dirty);
    std::fs::remove_file(dir.path().join(".git/index")).unwrap();
    assert_eq!(discover(dir.path()).changed_paths(ChangedBase::Omitted).unwrap(), vec![PathBuf::from("leaf.rs")]);
    let unborn = tempfile::TempDir::new().unwrap();
    init_repo(unborn.path());
    std::fs::write(unborn.path().join("first.rs"), "fn first() {}\n").unwrap();
    assert_eq!(discover(unborn.path()).changed_paths(ChangedBase::Omitted).unwrap(), vec![PathBuf::from("first.rs")]);
    git2::Repository::open(unborn.path()).unwrap().index().unwrap().write().unwrap();
    assert_eq!(discover(unborn.path()).changed_paths(ChangedBase::Omitted).unwrap(), vec![PathBuf::from("first.rs")]);
}
