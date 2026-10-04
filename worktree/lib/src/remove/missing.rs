//! Removing the record of a worktree whose directory is already gone.
//!
//! No checkout exists to inspect, but the worktree's own index survives in its
//! administrative record and can still hold staged work. That index is
//! compared with the worktree's recorded HEAD; differences need the same
//! consent as staged files in an existing checkout, and an index that can't
//! be inspected refuses. Removal re-reads that index (its own bytes and Git's
//! reading of every entry, which in a split index also come from the
//! `sharedindex.*` file it references) and the HEAD it was compared with, and
//! refuses if either changed, or can no longer be read, since the inspection
//! that informed the report and any consent. Only `git worktree remove` of this
//! one record is ever run: never `prune`, never a second `--force`.

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
    /// BLAKE3 of the index bytes, read before `staged` was computed. Removal
    /// requires the index to still hash to this, so consent covers only
    /// the staged work that was reported.
    pub index_digest: [u8; 32],
    /// BLAKE3 of Git's reading of the whole index (`ls-files --stage
    /// --debug`), taken alongside `index_digest`. A split index keeps entries
    /// in a `sharedindex.*` file that `index_digest` does not cover; Git reads
    /// it here, and removal requires the same reading again.
    pub entries_digest: [u8; 32],
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
    /// The index no longer holds the bytes or entries that were inspected;
    /// what it stages now was never reported.
    #[error("its index at {} changed after it was checked", .0.display())]
    IndexChanged(PathBuf),
    /// The commit the index was compared with is no longer the recorded HEAD.
    #[error("its recorded HEAD moved from {was} to {now} after it was checked")]
    HeadChanged { was: String, now: String },
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
    let index = admin.index();
    // Hashed before Git reads it: a write in between makes removal refuse,
    // never binds consent to bytes the report did not describe.
    let index_digest = biscuit_hash::blake3_hash_bytes(&index_bytes(&index)?);
    let entries_digest = entries_digest(base, &index)?;
    let staged = staged_changes(base, &index, &head)?;
    Ok(MissingCheckout { admin, head, staged, index_digest, entries_digest })
}

/// The bytes of the surviving `index`, which must be a regular file.
fn index_bytes(index: &Path) -> Result<Vec<u8>, MissingRefusal> {
    let uninspectable = |reason: String| MissingRefusal::IndexUninspectable { path: index.to_path_buf(), reason };
    let absent = |error: &std::io::Error| error.kind() == std::io::ErrorKind::NotFound;
    match availability::inspect(index) {
        Ok(EntryKind::File) => {}
        Ok(_) => return Err(uninspectable("it is not a regular file".into())),
        Err(error) if absent(&error) => return Err(MissingRefusal::IndexAbsent(index.to_path_buf())),
        Err(error) => return Err(uninspectable(error.to_string())),
    }
    std::fs::read(index).map_err(|error| {
        if absent(&error) { MissingRefusal::IndexAbsent(index.to_path_buf()) } else { uninspectable(error.to_string()) }
    })
}

/// BLAKE3 of every entry Git reads from `index`: mode, object ID, stage,
/// path, stat data, and flags such as intent-to-add and skip-worktree.
///
/// Git, not this crate, parses the index, so a split index's entries count
/// from whichever `sharedindex.*` file it resolves. Git refuses a referenced
/// shared index that is missing, empty, truncated, corrupt, carries trailing
/// bytes, or is a directory; that refusal is
/// [`MissingRefusal::IndexUninspectable`].
fn entries_digest(base: &Path, index: &Path) -> Result<[u8; 32], MissingRefusal> {
    let listing = git_from_bytes_with_env(
        base,
        base,
        &["ls-files", "--stage", "--debug", "-z"],
        &[("GIT_INDEX_FILE", index.as_os_str())],
    )
    .map_err(|error| MissingRefusal::IndexUninspectable { path: index.to_path_buf(), reason: error.to_string() })?;
    Ok(biscuit_hash::blake3_hash_bytes(&listing))
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

/// Removes the record of `entry`, which `inspected` describes, then its copy
/// record.
///
/// Immediately before, the state `inspected` reported must still hold: a
/// fresh listing shows the same worktree at `entry.path`, the same
/// administrative entry points to it, its recorded HEAD is still
/// `inspected.head`, the path is still confirmed absent, and the index still
/// has the inspected bytes and Git still reads the inspected entries from it
/// (including any held in a split index's shared file). Any difference, or a fact that can't be read,
/// refuses so a new run reports and asks again; consent to discard covers
/// only what was reported. Runs plain `git worktree remove <path>`, so a lock
/// still stops it. The Windows directory-lock probe is skipped: there is no
/// directory to hold.
///
/// ## Returns
///
/// A warning when the copy record could not be deleted; the worktree record
/// is gone either way.
pub fn remove_missing_record(
    base: &Path,
    entry: &WorktreeEntry,
    inspected: &MissingCheckout,
) -> Result<Option<String>, MissingRefusal> {
    let listing = git_from(base, base, &["worktree", "list", "--porcelain"]).map_err(|error| MissingRefusal::Changed(format!("could not be read: {error}")))?;
    let entries = parse_worktree_list(&listing);
    let found: Vec<&WorktreeEntry> = entries.iter().filter(|listed| same_location(&listed.path, &entry.path)).collect();
    let [listed] = found.as_slice() else {
        return Err(MissingRefusal::Changed(format!("shows this path {} times", found.len())));
    };
    if let Some(change) = identity_change(entry, listed) {
        return Err(MissingRefusal::Changed(change));
    }
    let admin = admin_entry_for(base, &entry.path).map_err(MissingRefusal::Association)?;
    if admin != inspected.admin {
        return Err(MissingRefusal::Changed(format!("now associates this path with another record, {}", admin.dir.display())));
    }
    let head = recorded_head(base, listed).map_err(MissingRefusal::Head)?;
    if head != inspected.head {
        return Err(MissingRefusal::HeadChanged { was: inspected.head.clone(), now: head });
    }
    match availability::inspect(&entry.path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(MissingRefusal::PathUninspectable(error.to_string())),
        Ok(kind) => return Err(MissingRefusal::Reappeared(kind)),
    }
    let index = inspected.admin.index();
    if biscuit_hash::blake3_hash_bytes(&index_bytes(&index)?) != inspected.index_digest
        || entries_digest(base, &index)? != inspected.entries_digest
    {
        return Err(MissingRefusal::IndexChanged(index));
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
        let warning = remove_missing_record(&repo.path(), &entry, &inspected).unwrap();
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
        let inspected = inspect_missing(&repo.path(), &entry).unwrap();
        fs::create_dir(&entry.path).unwrap();
        fs::write(entry.path.join("new-work.txt"), "x\n").unwrap();

        let result = remove_missing_record(&repo.path(), &entry, &inspected);

        assert!(matches!(result, Err(MissingRefusal::Reappeared(EntryKind::Directory))), "{result:?}");
        assert!(entry.path.join("new-work.txt").exists());
        assert!(is_listed(&repo, &entry.path));
    }

    #[cfg(unix)]
    #[test]
    fn a_link_that_appears_is_never_followed_or_removed() {
        let repo = TestRepo::new();
        let entry = missing(&repo, "feat/l", "l", |_| {});
        let inspected = inspect_missing(&repo.path(), &entry).unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(elsewhere.path(), &entry.path).unwrap();

        let result = remove_missing_record(&repo.path(), &entry, &inspected);

        assert!(matches!(result, Err(MissingRefusal::Reappeared(EntryKind::Link))), "{result:?}");
        assert!(is_listed(&repo, &entry.path) && elsewhere.path().is_dir());
    }

    #[test]
    fn a_changed_entry_at_the_path_refuses() {
        let repo = TestRepo::new();
        let mut entry = missing(&repo, "feat/c", "c", |_| {});
        let inspected = inspect_missing(&repo.path(), &entry).unwrap();
        entry.branch = Some("feat/other".into());

        let result = remove_missing_record(&repo.path(), &entry, &inspected);
        assert!(matches!(&result, Err(MissingRefusal::Changed(text)) if text.contains("branch feat/c")), "{result:?}");
        assert!(is_listed(&repo, &entry.path));
    }

    /// A lock still stops removal (one `--force` would not pass it either),
    /// and a failed removal keeps the copy record.
    #[test]
    fn a_failed_record_removal_keeps_the_record_and_its_copy_record() {
        let repo = TestRepo::new();
        let entry = missing(&repo, "feat/k", "k", |_| {});
        let inspected = inspect_missing(&repo.path(), &entry).unwrap();
        repo.git(&["worktree", "lock", "--reason", "kept on purpose", entry.path.to_str().unwrap()]);
        let record = copy_record::record_path(&repo.path(), &entry.path).unwrap();
        fs::create_dir_all(record.parent().unwrap()).unwrap();
        fs::write(&record, "{}").unwrap();

        let result = remove_missing_record(&repo.path(), &entry, &inspected);

        assert!(matches!(&result, Err(MissingRefusal::RecordRemoval(WorktreeError::GitCommand(text))) if text.contains("locked")), "{result:?}");
        assert!(is_listed(&repo, &entry.path));
        assert!(record.exists(), "no cleanup before the record is gone");
    }

    /// A missing worktree on `branch` whose surviving index matches HEAD,
    /// plus the bytes of a version of that index staging `staged.txt`.
    fn missing_with_a_staged_index(repo: &TestRepo, branch: &str, name: &str) -> (WorktreeEntry, PathBuf, Vec<u8>) {
        let mut staged_index = Vec::new();
        let entry = missing(repo, branch, name, |checkout| {
            let index = PathBuf::from(repo.git_in(checkout, &["rev-parse", "--path-format=absolute", "--git-path", "index"]));
            let clean = fs::read(&index).unwrap();
            fs::write(checkout.join("staged.txt"), "staged after the check\n").unwrap();
            repo.git_in(checkout, &["add", "staged.txt"]);
            staged_index = fs::read(&index).unwrap();
            fs::write(&index, clean).unwrap();
        });
        let index = admin_entry_for(&repo.path(), &entry.path).unwrap().index();
        (entry, index, staged_index)
    }

    fn assert_kept(repo: &TestRepo, entry: &WorktreeEntry, index: &Path, branch: &str) {
        assert!(is_listed(repo, &entry.path), "record kept");
        assert!(index.parent().unwrap().is_dir(), "administrative entry kept");
        repo.git(&["rev-parse", "--verify", &format!("refs/heads/{branch}")]);
    }

    /// Consent covers what was reported: an index written between inspection
    /// and removal (here, by staging a file) is never discarded unseen.
    #[test]
    fn an_index_changed_after_inspection_refuses_and_is_kept() {
        let repo = TestRepo::new();
        let (entry, index, staged_index) = missing_with_a_staged_index(&repo, "feat/s", "s");
        let inspected = inspect_missing(&repo.path(), &entry).unwrap();
        assert!(!inspected.needs_consent(), "{:?}", inspected.staged);

        fs::write(&index, &staged_index).unwrap();
        let result = remove_missing_record(&repo.path(), &entry, &inspected);

        assert!(matches!(&result, Err(MissingRefusal::IndexChanged(path)) if *path == index), "{result:?}");
        assert_kept(&repo, &entry, &index, "feat/s");
        assert_eq!(fs::read(&index).unwrap(), staged_index, "the new staged version survives");
    }

    #[test]
    fn an_index_that_disappears_or_is_corrupted_after_inspection_refuses() {
        let repo = TestRepo::new();
        let entry = missing(&repo, "feat/x", "x", |_| {});
        let inspected = inspect_missing(&repo.path(), &entry).unwrap();
        let index = inspected.admin.index();
        let original = fs::read(&index).unwrap();

        fs::write(&index, b"not an index").unwrap();
        let corrupted = remove_missing_record(&repo.path(), &entry, &inspected);
        assert!(matches!(corrupted, Err(MissingRefusal::IndexChanged(_))), "{corrupted:?}");
        assert_eq!(fs::read(&index).unwrap(), b"not an index");

        fs::remove_file(&index).unwrap();
        let disappeared = remove_missing_record(&repo.path(), &entry, &inspected);
        assert!(matches!(disappeared, Err(MissingRefusal::IndexAbsent(_))), "{disappeared:?}");
        assert_kept(&repo, &entry, &index, "feat/x");

        // The same record, restored to the inspected bytes, is removed.
        fs::write(&index, original).unwrap();
        remove_missing_record(&repo.path(), &entry, &inspected).unwrap();
        assert!(!is_listed(&repo, &entry.path));
    }

    /// The index was compared with this tip, and branch safety was judged on
    /// it, so a moved tip invalidates both.
    #[test]
    fn a_branch_tip_moved_after_inspection_refuses() {
        let repo = TestRepo::new();
        let entry = missing(&repo, "feat/t", "t", |_| {});
        let inspected = inspect_missing(&repo.path(), &entry).unwrap();
        let other = repo.add_worktree("feat/other", "other", "main");
        let moved = repo.commit_in(&other, "elsewhere.txt");
        repo.git(&["update-ref", "refs/heads/feat/t", &moved]);

        let result = remove_missing_record(&repo.path(), &entry, &inspected);

        assert!(
            matches!(&result, Err(MissingRefusal::HeadChanged { was, now }) if *was == inspected.head && *now == moved),
            "{result:?}"
        );
        assert_kept(&repo, &entry, &inspected.admin.index(), "feat/t");
        assert_eq!(repo.sha("refs/heads/feat/t"), moved);
    }

    /// Git now lists the path under another administrative entry (here, the
    /// entry's directory renamed), so the index that was inspected is not
    /// the one removal would discard. The CLI renders `Changed` as exit 3.
    #[test]
    fn a_record_under_another_administrative_entry_after_inspection_refuses() {
        let repo = TestRepo::new();
        let entry = missing(&repo, "feat/a", "a", |_| {});
        let inspected = inspect_missing(&repo.path(), &entry).unwrap();
        let renamed = inspected.admin.dir.with_file_name("a-renamed");
        fs::rename(&inspected.admin.dir, &renamed).unwrap();
        assert!(is_listed(&repo, &entry.path), "Git lists the same worktree under the new entry");

        let result = remove_missing_record(&repo.path(), &entry, &inspected);

        assert!(
            matches!(&result, Err(MissingRefusal::Changed(text)) if text.contains("another record")),
            "{result:?}"
        );
        assert_kept(&repo, &entry, &renamed.join("index"), "feat/a");
    }

    /// The control for the refusals above: staged work that was reported
    /// (and so could be consented to) is removed while it is unchanged.
    #[test]
    fn unchanged_reported_staged_work_is_removed() {
        let repo = TestRepo::new();
        let (entry, index, staged_index) = missing_with_a_staged_index(&repo, "feat/u", "u");
        fs::write(&index, &staged_index).unwrap();
        let inspected = inspect_missing(&repo.path(), &entry).unwrap();
        assert!(inspected.needs_consent());

        remove_missing_record(&repo.path(), &entry, &inspected).unwrap();

        assert!(!is_listed(&repo, &entry.path));
        assert!(!inspected.admin.dir.exists());
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

    /// The controls for the split-index refusals below: an unchanged split
    /// index, clean or holding reported staged work, is removed.
    #[test]
    fn an_unchanged_split_index_is_removed() {
        for stage in [false, true] {
            let repo = TestRepo::new();
            let (entry, _) = missing_with_a_split_index(&repo, "feat/split", "split", stage);
            let inspected = inspect_missing(&repo.path(), &entry).unwrap();
            assert_eq!(inspected.needs_consent(), stage, "{:?}", inspected.staged);
            if stage {
                assert_eq!(statuses(&inspected), [("A ".into(), "staged.txt".into())]);
            }

            remove_missing_record(&repo.path(), &entry, &inspected).unwrap();

            assert!(!is_listed(&repo, &entry.path), "staged: {stage}");
            assert!(!inspected.admin.dir.exists());
        }
    }

    /// A split index's entries live in the shared file, not in `index`. Each
    /// edit to that file after inspection leaves `index` byte-identical, yet
    /// what Git would read (and removal discard) is no longer what was
    /// reported, so removal refuses and keeps the record and branch.
    #[test]
    fn a_split_index_whose_shared_file_changes_after_inspection_refuses() {
        type Edit = fn(&Path);
        let edits: [(&str, Edit); 5] = [
            ("corrupted", |shared| fs::write(shared, b"corrupt").unwrap()),
            ("removed", |shared| fs::remove_file(shared).unwrap()),
            ("emptied", |shared| fs::write(shared, b"").unwrap()),
            ("given trailing garbage", |shared| {
                let mut bytes = fs::read(shared).unwrap();
                bytes.extend_from_slice(b"garbage\n");
                fs::write(shared, bytes).unwrap();
            }),
            ("replaced by a directory", |shared| {
                fs::remove_file(shared).unwrap();
                fs::create_dir(shared).unwrap();
            }),
        ];
        for stage in [false, true] {
            for (shape, edit) in edits {
                let repo = TestRepo::new();
                let (entry, shared) = missing_with_a_split_index(&repo, "feat/split", "split", stage);
                let inspected = inspect_missing(&repo.path(), &entry).unwrap();
                let index = inspected.admin.index();
                let primary = fs::read(&index).unwrap();

                edit(&shared);
                let result = remove_missing_record(&repo.path(), &entry, &inspected);

                assert!(
                    matches!(&result, Err(MissingRefusal::IndexUninspectable { path, .. }) if *path == index),
                    "{shape} (staged: {stage}): {result:?}"
                );
                assert_eq!(fs::read(&index).unwrap(), primary, "{shape}: only the shared file changed");
                assert_kept(&repo, &entry, &index, "feat/split");
            }
        }
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
