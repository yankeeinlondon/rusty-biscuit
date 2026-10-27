use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::remove::admin_entry::admin_entry_for;
use crate::remove::test_support::TestRepo;

const FILE_PROTOCOL: [&str; 2] = ["-c", "protocol.file.allow=always"];

/// A repository with a linked worktree `wt` and a source repository the
/// tests add as submodules (one commit on `main`).
struct Fixture {
    repo: TestRepo,
    source: TestRepo,
    checkout: PathBuf,
    admin: AdminEntry,
}

impl Fixture {
    fn new() -> Self {
        let repo = TestRepo::new();
        let source = TestRepo::new();
        let checkout = repo.add_worktree("feat/sub", "wt", "main");
        let admin = admin_entry_for(&repo.path(), &checkout).expect("admin entry");
        Self { repo, source, checkout, admin }
    }

    /// `git submodule add <url> <path>` in the worktree; returns the
    /// submodule's checkout, configured to commit.
    fn add_submodule(&self, url: &Path, path: &str) -> PathBuf {
        let args = [&FILE_PROTOCOL[..], &["submodule", "add", "-q", url.to_str().unwrap(), path]].concat();
        self.repo.git_in(&self.checkout, &args);
        self.configure_module(path);
        self.checkout.join(path)
    }

    fn configure_module(&self, name: &str) {
        crate::test_support::append_config(
            &self.admin.dir.join("modules").join(name).join("config"),
            &[("user.email", "test@example.com"), ("user.name", "Test User"), ("commit.gpgsign", "false")],
        );
    }

    fn inspect(&self) -> SubmoduleReport {
        inspect(&self.repo.path(), &self.admin, &self.checkout)
    }

    /// A gitlink at `path` in the worktree's index, with no checkout.
    fn add_gitlink(&self, path: &str) {
        let head = self.source.sha("HEAD");
        self.repo.git_in(&self.checkout, &["update-index", "--add", "--cacheinfo", &format!("160000,{head},{path}")]);
    }
}

fn canonical(path: &Path) -> PathBuf {
    canonical_worktree_path(path).unwrap()
}

fn only(report: SubmoduleReport) -> Submodule {
    assert_eq!(report.unlisted, None);
    let [submodule]: [Submodule; 1] = report.submodules.try_into().expect("exactly one submodule");
    submodule
}

#[test]
fn a_clean_submodule_is_clean_and_owned_by_the_admin_record() {
    let fixture = Fixture::new();
    fixture.add_submodule(&fixture.source.path(), "sub");

    let submodule = only(fixture.inspect());

    assert_eq!(submodule.path, PathBuf::from("sub"));
    assert_eq!(submodule.git_dir, GitDirOwner::Owned(canonical(&fixture.admin.dir.join("modules").join("sub"))));
    assert_eq!(submodule.finding, Finding::Clean);
}

#[test]
fn an_uncommitted_file_and_an_unpushed_commit_are_at_risk() {
    let fixture = Fixture::new();
    let sub = fixture.add_submodule(&fixture.source.path(), "sub");
    fixture.repo.commit_in(&sub, "unpushed.txt");
    fs::write(sub.join("draft.txt"), "draft").unwrap();

    let submodule = only(fixture.inspect());

    assert_eq!(
        submodule.finding,
        Finding::AtRisk { dirty: vec![PathBuf::from("draft.txt")], unique_commits: 1, no_remote: false }
    );
}

#[test]
fn a_submodule_with_no_remote_counts_every_local_commit() {
    let fixture = Fixture::new();
    let sub = fixture.add_submodule(&fixture.source.path(), "sub");
    fixture.repo.git_in(&sub, &["remote", "remove", "origin"]);

    let submodule = only(fixture.inspect());

    // The source's single commit is now on no remote.
    assert_eq!(submodule.finding, Finding::AtRisk { dirty: vec![], unique_commits: 1, no_remote: true });
}

#[test]
fn an_uninitialized_submodule_has_no_entry() {
    let fixture = Fixture::new();
    fixture.add_gitlink("unsub");
    fs::create_dir(fixture.checkout.join("unsub")).unwrap();

    let report = fixture.inspect();

    assert_eq!(report, SubmoduleReport::default());
}

#[test]
fn a_nested_submodule_is_reported_with_its_full_path() {
    let fixture = Fixture::new();
    let outer = TestRepo::new();
    let source = fixture.source.path();
    let args = [&FILE_PROTOCOL[..], &["submodule", "add", "-q", source.to_str().unwrap(), "inner"]].concat();
    outer.git(&args);
    outer.git(&["commit", "-q", "-m", "add inner"]);
    let sub = fixture.add_submodule(&outer.path(), "outer");
    fixture.repo.git_in(&sub, &[&FILE_PROTOCOL[..], &["submodule", "update", "-q", "--init"]].concat());
    fs::write(sub.join("inner").join("draft.txt"), "draft").unwrap();

    let report = fixture.inspect();

    assert_eq!(report.unlisted, None);
    let modules = fixture.admin.dir.join("modules").join("outer");
    assert_eq!(
        report.submodules,
        vec![
            Submodule { path: PathBuf::from("outer"), git_dir: GitDirOwner::Owned(canonical(&modules)), finding: Finding::Clean },
            Submodule {
                path: PathBuf::from("outer").join("inner"),
                git_dir: GitDirOwner::Owned(canonical(&modules.join("modules").join("inner"))),
                finding: Finding::AtRisk { dirty: vec![PathBuf::from("draft.txt")], unique_commits: 0, no_remote: false },
            },
        ]
    );
}

#[test]
fn a_git_file_pointing_outside_the_admin_record_is_external() {
    let fixture = Fixture::new();
    let external = fixture.source.path().parent().unwrap().join("external.git");
    let sub = fixture.checkout.join("ext");
    let source = fixture.source.path();
    fixture.repo.git_in(
        &fixture.checkout,
        &["clone", "-q", "--separate-git-dir", external.to_str().unwrap(), source.to_str().unwrap(), sub.to_str().unwrap()],
    );
    fixture.repo.git_in(&fixture.checkout, &["add", "ext"]);

    let submodule = only(fixture.inspect());

    assert_eq!(submodule.path, PathBuf::from("ext"));
    assert_eq!(submodule.git_dir, GitDirOwner::External(canonical(&external)));
    assert_eq!(submodule.finding, Finding::Clean);
}

#[test]
fn a_failed_inspection_is_unchecked_with_gits_reason() {
    let fixture = Fixture::new();
    fixture.add_gitlink("broken");
    let missing = fixture.source.path().parent().unwrap().join("missing.git");
    fs::create_dir(fixture.checkout.join("broken")).unwrap();
    fs::write(fixture.checkout.join("broken").join(".git"), format!("gitdir: {}\n", missing.to_str().unwrap())).unwrap();

    let submodule = only(fixture.inspect());

    assert_eq!(submodule.path, PathBuf::from("broken"));
    let Finding::Unchecked(reason) = &submodule.finding else {
        panic!("expected Unchecked, got {:?}", submodule.finding);
    };
    // Git words this per version ("not a git repository", "gitfile does not
    // point to a valid repository"); it is Git's own message either way.
    assert!(reason.starts_with("fatal:"), "{reason}");
}

#[test]
fn an_unreadable_index_leaves_the_submodules_unlisted() {
    let fixture = Fixture::new();
    // Longer than an index header and its checksum: gix-index 0.52 panics
    // (subtraction overflow) reading a shorter one.
    fs::write(fixture.admin.index(), [b'j'; 64]).unwrap();

    let report = fixture.inspect();

    assert!(report.submodules.is_empty());
    assert!(report.unlisted.is_some_and(|reason| !reason.is_empty()));
}

#[test]
fn status_paths_skip_a_renames_original_and_keep_untracked_directories() {
    let status = b"R  new.txt\0old.txt\0?? dir/\0 M a b.txt\0";

    let paths = dirty_paths(status).unwrap();

    assert_eq!(paths, vec![PathBuf::from("new.txt"), PathBuf::from("dir"), PathBuf::from("a b.txt")]);
}
