//! Removing the record of a worktree whose directory is already gone.
//!
//! No checkout exists to inspect, but the worktree's own index survives in its
//! administrative record and can still hold staged work. That index is
//! compared with the worktree's recorded HEAD; differences need the same
//! consent as staged files in an existing checkout, and an index that can't
//! be inspected refuses. Only `git worktree remove` of this one record is
//! ever run: never `prune`, never a second `--force`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::availability::{self, Availability, EntryKind};
use crate::copy_record;
use crate::error::WorktreeError;
use crate::git::{git_from, git_from_bytes_with_env, git_from_output};
use crate::worktree::{WorktreeEntry, parse_worktree_list};

use super::admin_entry::{AdminEntry, AssociationError, admin_entry_for, same_location};
use super::inventory::{DirtyEntry, path_from_git};
use super::repair::identity_change;

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

/// Why a missing worktree's record can't be inspected or removed.
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
    /// A fresh listing no longer shows the same worktree at the path.
    #[error("Git's worktree list {0}")]
    Changed(String),
    /// Something appeared at the path after it was checked; its contents
    /// have not been checked.
    #[error("something now exists at its path again")]
    Reappeared(EntryKind),
    #[error("its path can't be inspected: {0}")]
    PathUninspectable(String),
    /// `git worktree remove` failed; nothing was removed.
    #[error("git worktree remove failed: {0}")]
    RecordRemoval(WorktreeError),
}

/// The commit `entry` had checked out, from this repository's refs: its
/// branch's tip, or the recorded HEAD when detached. Reads no checkout.
pub fn recorded_head(base: &Path, entry: &WorktreeEntry) -> Result<String, WorktreeError> {
    match &entry.branch {
        Some(branch) => git_from(base, base, &["rev-parse", "--verify", &format!("refs/heads/{branch}^{{commit}}")]),
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
    let staged = staged_changes(base, &admin.index(), &head)?;
    Ok(MissingCheckout { admin, head, staged })
}

/// The index's differences from `head`'s tree, as `status` `"A "`, `"M "`,
/// `"D "`, ... (the index column of `git status`).
fn staged_changes(base: &Path, index: &Path, head: &str) -> Result<Vec<DirtyEntry>, MissingRefusal> {
    let uninspectable = |reason: String| MissingRefusal::IndexUninspectable { path: index.to_path_buf(), reason };
    match availability::inspect(index) {
        Ok(EntryKind::File) => {}
        Ok(_) => return Err(uninspectable("it is not a regular file".into())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(MissingRefusal::IndexAbsent(index.to_path_buf())),
        Err(error) => return Err(uninspectable(error.to_string())),
    }
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

/// Removes the record of `entry`, then its copy record.
///
/// Immediately before, a fresh listing must show the same worktree at
/// `entry.path` and the path must still be confirmed absent; anything that
/// has appeared there refuses, so its contents get checked by a new run.
/// Runs plain `git worktree remove <path>`, so a lock still stops it. The
/// Windows directory-lock probe is skipped: there is no directory to hold.
///
/// ## Returns
///
/// A warning when the copy record could not be deleted; the worktree record
/// is gone either way.
pub fn remove_missing_record(base: &Path, entry: &WorktreeEntry) -> Result<Option<String>, MissingRefusal> {
    let listing = git_from(base, base, &["worktree", "list", "--porcelain"]).map_err(|error| MissingRefusal::Changed(format!("could not be read: {error}")))?;
    let entries = parse_worktree_list(&listing);
    let found: Vec<&WorktreeEntry> = entries.iter().filter(|listed| same_location(&listed.path, &entry.path)).collect();
    let [listed] = found.as_slice() else {
        return Err(MissingRefusal::Changed(format!("shows this path {} times", found.len())));
    };
    if let Some(change) = identity_change(entry, listed) {
        return Err(MissingRefusal::Changed(change));
    }
    match availability::inspect(&entry.path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(MissingRefusal::PathUninspectable(error.to_string())),
        Ok(kind) => return Err(MissingRefusal::Reappeared(kind)),
    }

    let output = git_from_output(base, base, &[OsStr::new("worktree"), OsStr::new("remove"), entry.path.as_os_str()])
        .map_err(|error| MissingRefusal::RecordRemoval(WorktreeError::GitCommand(error.to_string())))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(MissingRefusal::RecordRemoval(WorktreeError::GitCommand(stderr)));
    }
    Ok(copy_record::delete_for(base, &entry.path))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::git::recorder;
    use crate::remove::test_support::TestRepo;

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

    #[test]
    fn a_disposable_record_is_removed_alone_without_inspecting_a_checkout() {
        let repo = TestRepo::new();
        let keep = repo.add_worktree("feat/keep", "keep", "main");
        let entry = missing(&repo, "feat/gone", "gone", |checkout| {
            fs::write(checkout.join("untracked.txt"), "lost with the directory\n").unwrap();
        });

        let inspected = inspect_missing(&repo.path(), &entry).unwrap();
        assert!(!inspected.needs_consent(), "{:?}", inspected.staged);
        assert_eq!(inspected.head, repo.sha("refs/heads/feat/gone"));

        let record = copy_record::record_path(&repo.path(), &entry.path).unwrap();
        fs::create_dir_all(record.parent().unwrap()).unwrap();
        fs::write(&record, "{}").unwrap();

        recorder::start_recording();
        let warning = remove_missing_record(&repo.path(), &entry).unwrap();
        let calls = recorder::finish_recording();

        assert_eq!(warning, None);
        assert!(!is_listed(&repo, &entry.path), "record removed");
        assert!(!inspected.admin.dir.exists());
        assert!(!record.exists(), "copy record cleaned up after the record");
        assert!(is_listed(&repo, &keep) && keep.join("README.md").exists(), "other worktree intact");
        repo.git(&["rev-parse", "--verify", "refs/heads/feat/gone"]);
        let removes: Vec<_> = calls.iter().filter(|args| args.starts_with(&["worktree".into(), "remove".into()])).collect();
        assert_eq!(removes.len(), 1, "{calls:?}");
        assert!(!removes[0].iter().any(|arg| arg == "--force" || arg == "-f"), "no --force: {calls:?}");
        assert!(!calls.iter().any(|args| args.iter().any(|arg| arg == "prune" || arg == "status")), "{calls:?}");
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

    #[test]
    fn a_directory_that_reappears_is_never_removed() {
        let repo = TestRepo::new();
        let entry = missing(&repo, "feat/r", "r", |_| {});
        inspect_missing(&repo.path(), &entry).unwrap();
        fs::create_dir(&entry.path).unwrap();
        fs::write(entry.path.join("new-work.txt"), "x\n").unwrap();

        let result = remove_missing_record(&repo.path(), &entry);

        assert!(matches!(result, Err(MissingRefusal::Reappeared(EntryKind::Directory))), "{result:?}");
        assert!(entry.path.join("new-work.txt").exists());
        assert!(is_listed(&repo, &entry.path));
    }

    #[cfg(unix)]
    #[test]
    fn a_link_that_appears_is_never_followed_or_removed() {
        let repo = TestRepo::new();
        let entry = missing(&repo, "feat/l", "l", |_| {});
        let elsewhere = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(elsewhere.path(), &entry.path).unwrap();

        let result = remove_missing_record(&repo.path(), &entry);

        assert!(matches!(result, Err(MissingRefusal::Reappeared(EntryKind::Link))), "{result:?}");
        assert!(is_listed(&repo, &entry.path) && elsewhere.path().is_dir());
    }

    #[test]
    fn a_changed_entry_at_the_path_refuses() {
        let repo = TestRepo::new();
        let mut entry = missing(&repo, "feat/c", "c", |_| {});
        entry.branch = Some("feat/other".into());

        let result = remove_missing_record(&repo.path(), &entry);
        assert!(matches!(&result, Err(MissingRefusal::Changed(text)) if text.contains("branch feat/c")), "{result:?}");
        assert!(is_listed(&repo, &entry.path));
    }

    /// A lock still stops removal (one `--force` would not pass it either),
    /// and a failed removal keeps the copy record.
    #[test]
    fn a_failed_record_removal_keeps_the_record_and_its_copy_record() {
        let repo = TestRepo::new();
        let entry = missing(&repo, "feat/k", "k", |_| {});
        repo.git(&["worktree", "lock", "--reason", "kept on purpose", entry.path.to_str().unwrap()]);
        let record = copy_record::record_path(&repo.path(), &entry.path).unwrap();
        fs::create_dir_all(record.parent().unwrap()).unwrap();
        fs::write(&record, "{}").unwrap();

        let result = remove_missing_record(&repo.path(), &entry);

        assert!(matches!(&result, Err(MissingRefusal::RecordRemoval(WorktreeError::GitCommand(text))) if text.contains("locked")), "{result:?}");
        assert!(is_listed(&repo, &entry.path));
        assert!(record.exists(), "no cleanup before the record is gone");
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
