//! Inspecting the record of a worktree whose directory is already gone.
//!
//! No checkout exists to inspect, but the worktree's own index survives in its
//! administrative record and can still hold staged work. That index is
//! compared with the worktree's recorded HEAD; its differences are shown in
//! the removal question like staged files in an existing checkout, and an
//! index that can't be inspected is a warning there. Git reads the index, so
//! a split index's entries in its `sharedindex.*` file count too.

use std::path::{Path, PathBuf};

use crate::availability::{self, Availability, EntryKind};
use crate::error::WorktreeError;
use crate::git::git_from_bytes_with_env;
use crate::worktree::WorktreeEntry;

use super::admin_entry::{AdminEntry, AssociationError, admin_entry_for};
use super::inventory::{DirtyEntry, path_from_git};

/// A worktree whose directory is gone: what its surviving record holds.
///
/// There is no file inventory. `staged` lists the index's differences from
/// the recorded HEAD; empty means the index was read and matches HEAD, never
/// that a checkout was examined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingCheckout {
    pub admin: AdminEntry,
    /// The commit the worktree had checked out: its branch's tip, or the
    /// recorded HEAD when detached.
    pub head: String,
    pub staged: Vec<DirtyEntry>,
}

impl MissingCheckout {
    /// Whether removing the record would discard staged work.
    pub fn needs_consent(&self) -> bool {
        !self.staged.is_empty()
    }
}

/// Why a missing worktree's record can't be inspected.
#[derive(Debug, thiserror::Error)]
pub enum MissingRefusal {
    /// The target is not (or no longer) a confirmed-absent directory.
    #[error("the worktree's directory is not confirmed gone")]
    NotMissing(Availability),
    #[error(transparent)]
    Association(AssociationError),
    #[error("its recorded HEAD can't be resolved: {0}")]
    Head(WorktreeError),
    #[error("its index at {} is missing, so staged work can't be ruled out", .0.display())]
    IndexAbsent(PathBuf),
    #[error("its index at {} can't be inspected: {reason}", .path.display())]
    IndexUninspectable { path: PathBuf, reason: String },
}

/// The commit `entry` had checked out, from this repository's refs: its
/// branch's tip, or the recorded HEAD when detached. Reads no checkout.
pub fn recorded_head(base: &Path, entry: &WorktreeEntry) -> Result<String, WorktreeError> {
    match &entry.branch {
        Some(branch) => super::branch_tip(base, branch),
        None => entry.head_sha.clone().ok_or_else(|| WorktreeError::GitParse("worktree has no HEAD".into())),
    }
}

/// Inspects the record of `entry`, whose directory must be confirmed absent.
///
/// The surviving index must exist and be a regular file (Git reads an absent
/// index as empty, which would hide nothing and prove nothing).
pub fn inspect_missing(base: &Path, entry: &WorktreeEntry) -> Result<MissingCheckout, MissingRefusal> {
    let availability = availability::classify(entry);
    if availability != Availability::Missing {
        return Err(MissingRefusal::NotMissing(availability));
    }
    let admin = admin_entry_for(base, &entry.path).map_err(MissingRefusal::Association)?;
    let head = recorded_head(base, entry).map_err(MissingRefusal::Head)?;
    let index = admin.index();
    check_index(&index)?;
    let staged = staged_changes(base, &index, &head)?;
    Ok(MissingCheckout { admin, head, staged })
}

/// Requires the surviving `index` to be a regular file.
fn check_index(index: &Path) -> Result<(), MissingRefusal> {
    let uninspectable = |reason: String| MissingRefusal::IndexUninspectable { path: index.to_path_buf(), reason };
    let absent = |error: &std::io::Error| error.kind() == std::io::ErrorKind::NotFound;
    match availability::inspect(index) {
        Ok(EntryKind::File) => Ok(()),
        Ok(_) => Err(uninspectable("it is not a regular file".into())),
        Err(error) if absent(&error) => Err(MissingRefusal::IndexAbsent(index.to_path_buf())),
        Err(error) => Err(uninspectable(error.to_string())),
    }
}

/// The index's differences from `head`'s tree, as `status` `"A "`, `"M "`,
/// `"D "`, ... (the index column of `git status`).
fn staged_changes(base: &Path, index: &Path, head: &str) -> Result<Vec<DirtyEntry>, MissingRefusal> {
    let uninspectable = |reason: String| MissingRefusal::IndexUninspectable { path: index.to_path_buf(), reason };
    let output = git_from_bytes_with_env(
        base,
        base,
        &["diff-index", "--cached", "--no-renames", "--name-status", "-z", head, "--"],
        &[("GIT_INDEX_FILE", index.as_os_str())],
    )
    .map_err(|error| uninspectable(error.to_string()))?;
    parse_name_status_z(&output).map_err(uninspectable)
}

/// `--name-status -z` output: a status letter field, then a path field.
fn parse_name_status_z(output: &[u8]) -> Result<Vec<DirtyEntry>, String> {
    let mut fields = output.split(|&byte| byte == 0);
    let mut entries = Vec::new();
    while let Some(status) = fields.next() {
        if status.is_empty() {
            continue;
        }
        let [letter] = status else {
            return Err(format!("unexpected status {:?}", String::from_utf8_lossy(status)));
        };
        let Some(path) = fields.next().filter(|path| !path.is_empty()) else {
            return Err("a status without a path".into());
        };
        let path = path_from_git(path).map_err(|error| error.to_string())?;
        entries.push(DirtyEntry {
            status: format!("{} ", *letter as char),
            is_source: sniff::filesystem::path_kind::is_source_code_path(&path),
            path,
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::remove::admin_entry::same_location;
    use crate::remove::test_support::TestRepo;
    use crate::worktree::parse_worktree_list;

    fn listed(repo: &TestRepo, path: &Path) -> WorktreeEntry {
        parse_worktree_list(&repo.git(&["worktree", "list", "--porcelain"]))
            .into_iter()
            .find(|entry| same_location(&entry.path, path))
            .expect("listed")
    }

    fn is_listed(repo: &TestRepo, path: &Path) -> bool {
        parse_worktree_list(&repo.git(&["worktree", "list", "--porcelain"])).iter().any(|entry| same_location(&entry.path, path))
    }

    /// A linked worktree on `branch`, after `edit` ran in it, with its
    /// directory deleted.
    fn missing(repo: &TestRepo, branch: &str, name: &str, edit: impl FnOnce(&Path)) -> WorktreeEntry {
        let checkout = repo.add_worktree(branch, name, "main");
        edit(&checkout);
        fs::remove_dir_all(&checkout).unwrap();
        let entry = listed(repo, &checkout);
        assert!(entry.prunable.is_some());
        entry
    }

    fn statuses(missing: &MissingCheckout) -> Vec<(String, String)> {
        missing.staged.iter().map(|entry| (entry.status.clone(), entry.path.display().to_string())).collect()
    }

    /// Untracked files were lost with the directory; only the index's
    /// differences from the recorded HEAD remain to report.
    #[test]
    fn a_record_without_staged_work_needs_no_consent() {
        let repo = TestRepo::new();
        let entry = missing(&repo, "feat/gone", "gone", |checkout| {
            fs::write(checkout.join("untracked.txt"), "lost with the directory\n").unwrap();
        });

        let inspected = inspect_missing(&repo.path(), &entry).unwrap();

        assert!(!inspected.needs_consent(), "{:?}", inspected.staged);
        assert_eq!(inspected.head, repo.sha("refs/heads/feat/gone"));
        assert!(is_listed(&repo, &entry.path), "inspection removes nothing");
    }

    #[test]
    fn staged_work_in_the_surviving_index_is_reported() {
        let repo = TestRepo::new();
        let entry = missing(&repo, "feat/staged", "staged", |checkout| {
            fs::write(checkout.join("new.rs"), "fn main() {}\n").unwrap();
            fs::write(checkout.join("README.md"), "changed\n").unwrap();
            fs::write(checkout.join("unstaged.txt"), "x\n").unwrap();
            repo.git_in(checkout, &["add", "new.rs", "README.md"]);
        });

        let inspected = inspect_missing(&repo.path(), &entry).unwrap();

        assert!(inspected.needs_consent());
        assert_eq!(statuses(&inspected), [("M ".into(), "README.md".into()), ("A ".into(), "new.rs".into())]);
        assert!(inspected.staged.iter().any(|entry| entry.is_source));
    }

    #[test]
    fn a_detached_record_is_compared_with_its_recorded_head() {
        let repo = TestRepo::new();
        let entry = missing(&repo, "feat/d", "d", |checkout| {
            repo.commit_in(checkout, "on-branch.txt");
            repo.git_in(checkout, &["checkout", "-q", "--detach", "HEAD~1"]);
        });
        assert_eq!(entry.branch, None);
        repo.git(&["branch", "-D", "feat/d"]);

        let inspected = inspect_missing(&repo.path(), &entry).unwrap();
        assert_eq!(Some(inspected.head.clone()), entry.head_sha);
        assert!(!inspected.needs_consent());
    }

    #[test]
    fn an_index_that_cannot_prove_staged_work_disposable_refuses() {
        let repo = TestRepo::new();
        let entry = missing(&repo, "feat/i", "i", |_| {});
        let index = admin_entry_for(&repo.path(), &entry.path).unwrap().index();

        fs::write(&index, b"not an index").unwrap();
        assert!(matches!(inspect_missing(&repo.path(), &entry), Err(MissingRefusal::IndexUninspectable { .. })));

        fs::remove_file(&index).unwrap();
        assert!(matches!(inspect_missing(&repo.path(), &entry), Err(MissingRefusal::IndexAbsent(_))));

        fs::create_dir(&index).unwrap();
        assert!(matches!(inspect_missing(&repo.path(), &entry), Err(MissingRefusal::IndexUninspectable { .. })));
        assert!(is_listed(&repo, &entry.path), "record kept");
    }

    #[test]
    fn only_a_confirmed_absent_directory_is_inspected() {
        let repo = TestRepo::new();
        let checkout = repo.add_worktree("feat/u", "u", "main");
        fs::remove_file(checkout.join(".git")).unwrap();
        let entry = listed(&repo, &checkout);
        assert!(matches!(inspect_missing(&repo.path(), &entry), Err(MissingRefusal::NotMissing(Availability::Unlinked))));
    }

    /// A missing worktree on `branch` whose index Git keeps split (`update-index
    /// --split-index`), optionally with `staged.txt` staged first, and the
    /// `sharedindex.*` file its index references.
    fn missing_with_a_split_index(repo: &TestRepo, branch: &str, name: &str, stage: bool) -> (WorktreeEntry, PathBuf) {
        let entry = missing(repo, branch, name, |checkout| {
            if stage {
                fs::write(checkout.join("staged.txt"), "staged\n").unwrap();
                repo.git_in(checkout, &["add", "staged.txt"]);
            }
            repo.git_in(checkout, &["update-index", "--split-index"]);
        });
        let admin = admin_entry_for(&repo.path(), &entry.path).unwrap();
        let shared: Vec<PathBuf> = fs::read_dir(&admin.dir)
            .unwrap()
            .map(|dirent| dirent.unwrap().path())
            .filter(|path| path.file_name().unwrap().to_string_lossy().starts_with("sharedindex."))
            .collect();
        let [shared] = shared.as_slice() else { panic!("one shared index: {shared:?}") };
        (entry, shared.clone())
    }

    /// Git, not this crate, reads a split index, so staged work kept in its
    /// shared file is reported.
    #[test]
    fn staged_work_in_a_split_index_is_reported() {
        for stage in [false, true] {
            let repo = TestRepo::new();
            let (entry, _) = missing_with_a_split_index(&repo, "feat/split", "split", stage);

            let inspected = inspect_missing(&repo.path(), &entry).unwrap();

            assert_eq!(inspected.needs_consent(), stage, "{:?}", inspected.staged);
            if stage {
                assert_eq!(statuses(&inspected), [("A ".into(), "staged.txt".into())]);
            }
        }
    }

    /// The index file itself is intact, but the shared file it references is
    /// not: Git refuses to read it, so staged work cannot be ruled out.
    #[test]
    fn a_split_index_whose_shared_file_is_corrupt_cannot_be_inspected() {
        let repo = TestRepo::new();
        let (entry, shared) = missing_with_a_split_index(&repo, "feat/split", "split", true);
        fs::write(&shared, b"corrupt").unwrap();

        let result = inspect_missing(&repo.path(), &entry);

        assert!(matches!(&result, Err(MissingRefusal::IndexUninspectable { .. })), "{result:?}");
        assert!(is_listed(&repo, &entry.path), "record kept");
    }

    #[test]
    fn name_status_output_is_parsed_strictly() {
        assert_eq!(parse_name_status_z(b""), Ok(Vec::new()));
        let parsed = parse_name_status_z(b"D\0gone.txt\0A\0dir/new file.md\0").unwrap();
        let pairs: Vec<_> = parsed.iter().map(|entry| (entry.status.as_str(), entry.path.to_str().unwrap())).collect();
        assert_eq!(pairs, [("D ", "gone.txt"), ("A ", "dir/new file.md")]);
        assert!(parse_name_status_z(b"M\0").is_err(), "status without a path");
        assert!(parse_name_status_z(b"MM\0a\0").is_err(), "two-letter status");
    }
}
