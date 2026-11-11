//! Runtime worktrees: created detached at the reviewed commit, removed with
//! their administrative record and nothing else, and reported by hand-removal
//! command when they cannot be removed.

use std::path::Path;

use crate::ci_push::git::Repository;
use crate::ci_push::worktree::{ArtifactKind, CleanupFailure, RetainedArtifact, RuntimeWorktree};

use super::git::Fixture;

/// A repository with two commits, `first` and the checked-out `second`.
fn two_commits() -> (Fixture, gix::ObjectId, gix::ObjectId) {
    let fixture = Fixture::new();
    fixture.write("sub/kept.txt", "first\n");
    let first = fixture.commit("first");
    fixture.write("sub/kept.txt", "second\n");
    fixture.write("only-in-second.txt", "second\n");
    let second = fixture.commit("second");
    (fixture, first, second)
}

fn record(fixture: &Fixture, worktree: &RuntimeWorktree) -> std::path::PathBuf {
    fixture.root.join(".git/worktrees").join(worktree.name())
}

#[test]
fn a_runtime_worktree_is_detached_at_the_commit_with_no_temporary_branch_left() {
    let (fixture, first, second) = two_commits();

    let mut worktree = RuntimeWorktree::create(&fixture.root, first).unwrap();

    let checkout = Repository::discover(worktree.path()).unwrap();
    let head = checkout.head().unwrap();
    assert_eq!((head.commit, head.branch), (first, None));
    assert!(!checkout.is_dirty().unwrap());
    assert_eq!(
        std::fs::read_to_string(worktree.path().join("sub/kept.txt")).unwrap(),
        "first\n"
    );
    assert!(!worktree.path().join("only-in-second.txt").exists());
    assert_eq!(
        fixture
            .repository()
            .resolve_ref(&worktree.temporary_ref())
            .unwrap(),
        None
    );
    assert_eq!(fixture.repository().head().unwrap().commit, second);
    drop(checkout);
    let path = worktree.path().to_path_buf();
    let record = record(&fixture, &worktree);
    assert!(record.is_dir());

    worktree.remove().unwrap();

    assert!(!path.exists() && !path.parent().unwrap().exists());
    assert!(!record.exists());
}

#[test]
fn removal_leaves_another_invocations_worktree_alone() {
    let (fixture, first, second) = two_commits();
    let mut ours = RuntimeWorktree::create(&fixture.root, first).unwrap();
    let theirs = RuntimeWorktree::create(&fixture.root, second).unwrap();
    assert_ne!(ours.name(), theirs.name());
    assert_eq!(
        theirs.path().parent().unwrap().file_name().unwrap(),
        theirs.name()
    );

    ours.remove().unwrap();

    assert!(theirs.path().join("only-in-second.txt").is_file());
    assert!(record(&fixture, &theirs).is_dir());
    let listed = String::from_utf8(fixture.git(&["worktree", "list", "--porcelain"])).unwrap();
    assert!(listed.contains(theirs.name()), "{listed}");
    assert!(!listed.contains(ours.name()), "{listed}");
}

#[test]
fn dropping_a_runtime_worktree_removes_it() {
    let (fixture, first, _) = two_commits();
    let worktree = RuntimeWorktree::create(&fixture.root, first).unwrap();
    let path = worktree.path().to_path_buf();
    let record = record(&fixture, &worktree);

    drop(worktree);

    assert!(!path.exists() && !record.exists());
}

#[test]
fn a_commit_that_does_not_exist_leaves_nothing_behind() {
    let (fixture, _, _) = two_commits();
    let missing = gix::ObjectId::from_hex(b"0123456789abcdef0123456789abcdef01234567").unwrap();

    let error = RuntimeWorktree::create(&fixture.root, missing).unwrap_err();

    assert!(error.cleanup.is_none(), "{error}");
    assert!(error.to_string().contains(&missing.to_string()), "{error}");
    let records = fixture.root.join(".git/worktrees");
    assert!(!records.exists() || std::fs::read_dir(&records).unwrap().next().is_none());
    let branches =
        String::from_utf8(fixture.git(&["branch", "--list", "ci-push-review-*"])).unwrap();
    assert_eq!(branches, "");
}

#[cfg(unix)]
#[test]
fn a_worktree_that_cannot_be_deleted_is_reported_with_its_removal_command() {
    use std::os::unix::fs::PermissionsExt;
    // Root ignores directory permissions, so the failure cannot be staged.
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let (fixture, first, _) = two_commits();
    let mut worktree = RuntimeWorktree::create(&fixture.root, first).unwrap();
    let locked = worktree.path().join("sub");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
    let parent = worktree.path().parent().unwrap().to_path_buf();

    let failure = worktree.remove().unwrap_err();

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    let directory = failure
        .retained
        .iter()
        .find(|artifact| artifact.kind == ArtifactKind::Directory)
        .expect("the temporary directory is retained");
    assert_eq!(directory.location, parent.display().to_string());
    assert_eq!(directory.removal, format!("rm -rf '{}'", parent.display()));
    assert!(!failure.errors.is_empty());
    assert!(
        failure.to_string().contains(&directory.removal),
        "{failure}"
    );
    worktree.remove().unwrap();
    assert!(!parent.exists() && !record(&fixture, &worktree).exists());
}

#[test]
fn a_cleanup_report_names_each_artifact_and_its_command() {
    let failure = CleanupFailure {
        retained: vec![
            RetainedArtifact {
                kind: ArtifactKind::Worktree,
                location: "/tmp/ci-push-review-1/tree".to_owned(),
                removal: "git -C '/repo' worktree remove --force '/tmp/ci-push-review-1/tree'"
                    .to_owned(),
            },
            RetainedArtifact {
                kind: ArtifactKind::Ref,
                location: "refs/heads/ci-push-review-1".to_owned(),
                removal: "git -C '/repo' update-ref -d 'refs/heads/ci-push-review-1'".to_owned(),
            },
        ],
        errors: vec!["permission denied".to_owned()],
    };

    assert_eq!(
        failure.to_string(),
        "The temporary review worktree could not be removed completely (permission denied). \
         Remove what remains by hand:\n  \
         /tmp/ci-push-review-1/tree\n    git -C '/repo' worktree remove --force '/tmp/ci-push-review-1/tree'\n  \
         refs/heads/ci-push-review-1\n    git -C '/repo' update-ref -d 'refs/heads/ci-push-review-1'"
    );
}

#[test]
fn a_runtime_worktree_can_be_created_from_a_linked_checkout() {
    let (fixture, first, second) = two_commits();
    let linked = fixture.scratch("linked");
    fixture.git(&[
        "worktree",
        "add",
        "-q",
        "--detach",
        linked.to_str().unwrap(),
        &second.to_string(),
    ]);

    let mut worktree = RuntimeWorktree::create(Path::new(&linked), first).unwrap();

    assert!(record(&fixture, &worktree).is_dir());
    assert_eq!(
        Repository::discover(worktree.path())
            .unwrap()
            .head()
            .unwrap()
            .commit,
        first
    );
    worktree.remove().unwrap();
    assert!(linked.join(".git").is_file());
}
