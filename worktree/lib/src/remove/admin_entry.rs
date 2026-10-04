//! A linked worktree's administrative entry, `<common-git-dir>/worktrees/<id>/`,
//! and its `gitdir` back-reference to the checkout's `.git`.
//!
//! The back-reference is read strictly: removal associates an entry with its
//! target only through a path this reader accepted, so a file Git itself
//! would not have written is refused rather than guessed at.

use std::path::{Path, PathBuf};

use crate::availability::EntryKind;
use crate::copy_record::canonical_worktree_path;
use crate::error::WorktreeError;
use crate::git::git_from;

/// Why a `gitdir` back-reference could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BackReferenceError {
    #[error("the gitdir file is missing")]
    Missing,
    #[error("the gitdir file can't be read: {0}")]
    Unreadable(String),
    #[error("the gitdir file is empty")]
    Empty,
    #[error("the gitdir file holds more than one line")]
    MultipleLines,
    #[error("the gitdir file holds a NUL byte")]
    ContainsNul,
    #[error("the gitdir file does not name a .git entry")]
    NotAGitEntry,
    #[error("the gitdir file is not valid UTF-8")]
    NotUtf8,
}

/// The checkout `.git` path that `admin_dir/gitdir` names, as Git spelled it.
///
/// Trailing `\n` and `\r` are trimmed, as Git trims them. A relative path
/// (Git's `worktree.useRelativePaths`) is joined onto `admin_dir`. The path is
/// never canonicalized, because the checkout it names may be gone. On Unix
/// the bytes become the path unchanged; Windows requires UTF-8.
///
/// ## Errors
///
/// Every [`BackReferenceError`]: absent or unreadable, empty, several lines,
/// a NUL byte, or a last component other than `.git` (which also catches
/// trailing content on the line).
pub fn read_back_reference(admin_dir: &Path) -> Result<PathBuf, BackReferenceError> {
    let bytes = match std::fs::read(admin_dir.join("gitdir")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(BackReferenceError::Missing),
        Err(error) => return Err(BackReferenceError::Unreadable(error.to_string())),
    };
    let end = bytes.iter().rposition(|byte| !matches!(byte, b'\n' | b'\r')).map_or(0, |last| last + 1);
    let line = &bytes[..end];
    if line.is_empty() {
        return Err(BackReferenceError::Empty);
    }
    if line.iter().any(|byte| matches!(byte, b'\n' | b'\r')) {
        return Err(BackReferenceError::MultipleLines);
    }
    if line.contains(&0) {
        return Err(BackReferenceError::ContainsNul);
    }
    let path = path_from_bytes(line)?;
    if path.file_name() != Some(std::ffi::OsStr::new(".git")) {
        return Err(BackReferenceError::NotAGitEntry);
    }
    Ok(if path.is_relative() { admin_dir.join(path) } else { path })
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> Result<PathBuf, BackReferenceError> {
    use std::os::unix::ffi::OsStrExt;
    Ok(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> Result<PathBuf, BackReferenceError> {
    std::str::from_utf8(bytes).map(PathBuf::from).map_err(|_| BackReferenceError::NotUtf8)
}

/// A linked worktree's administrative directory, identified through its
/// back-reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminEntry {
    /// `<common-git-dir>/worktrees/<id>` as listed from the common directory.
    pub dir: PathBuf,
}

impl AdminEntry {
    /// The entry's own index, which outlives a deleted checkout.
    pub fn index(&self) -> PathBuf {
        self.dir.join("index")
    }
}

/// Why no single administrative entry could be associated with a target.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssociationError {
    #[error("Git's common directory could not be found: {0}")]
    CommonDirUnknown(String),
    #[error("Git's worktree records at {} could not be listed: {reason}", .dir.display())]
    Unlistable { dir: PathBuf, reason: String },
    #[error("the worktree record at {} can't be read: {error}", .dir.display())]
    Unreadable { dir: PathBuf, error: BackReferenceError },
    #[error("no worktree record points to this checkout")]
    NotFound,
    #[error("more than one worktree record points to this checkout: {}", display_list(.0))]
    Ambiguous(Vec<PathBuf>),
}

fn display_list(paths: &[PathBuf]) -> String {
    paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>().join(", ")
}

/// This repository's common Git directory, as an absolute path, from Git
/// itself (the base checkout's `.git` may be a file).
pub fn common_git_dir(base: &Path) -> Result<PathBuf, WorktreeError> {
    git_from(base, base, &["rev-parse", "--path-format=absolute", "--git-common-dir"]).map(PathBuf::from)
}

/// The one administrative entry whose back-reference names `target`'s `.git`.
///
/// Every directory under `<common-git-dir>/worktrees/` is read; a single
/// unreadable record (or one that is a link) refuses the association, because it might be the one
/// that points here. Paths compare after canonicalizing their longest
/// existing ancestor, so a missing checkout still matches across symlink
/// aliases and Windows short names.
///
/// ## Errors
///
/// Every [`AssociationError`]: no common directory, an unlistable or
/// unreadable record, no match, or more than one match.
pub fn admin_entry_for(base: &Path, target: &Path) -> Result<AdminEntry, AssociationError> {
    let common = common_git_dir(base).map_err(|error| AssociationError::CommonDirUnknown(error.to_string()))?;
    admin_entry_in(&common, target)
}

/// [`admin_entry_for`] with the common directory already known.
pub fn admin_entry_in(common_dir: &Path, target: &Path) -> Result<AdminEntry, AssociationError> {
    let records = common_dir.join("worktrees");
    let unlistable = |error: std::io::Error| AssociationError::Unlistable { dir: records.clone(), reason: error.to_string() };
    // Checkouts are compared rather than their `.git` entries: the reader
    // proved each back-reference ends in `.git`, and another record's
    // checkout replaced by a file must not make `<file>/.git` unresolvable.
    let wanted = canonical_worktree_path(target).map_err(unlistable)?;
    let mut matches = Vec::new();
    let mut dirs = std::fs::read_dir(&records).map_err(unlistable)?
        .map(|record| record.map(|record| record.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(unlistable)?;
    dirs.sort();
    for dir in dirs {
        let unreadable = |error| AssociationError::Unreadable { dir: dir.clone(), error };
        match crate::availability::inspect(&dir) {
            Ok(EntryKind::Directory) => {}
            // A plain file is no worktree record (Git skips it too).
            Ok(EntryKind::File) => continue,
            Ok(EntryKind::Link) => return Err(unreadable(BackReferenceError::Unreadable("the record is a link".into()))),
            Err(error) => return Err(unreadable(BackReferenceError::Unreadable(error.to_string()))),
        }
        let named = read_back_reference(&dir).map_err(unreadable)?;
        let named = canonical_worktree_path(named.parent().unwrap_or(&named))
            .map_err(|error| unreadable(BackReferenceError::Unreadable(error.to_string())))?;
        if named == wanted {
            matches.push(dir);
        }
    }
    match matches.len() {
        0 => Err(AssociationError::NotFound),
        1 => Ok(AdminEntry { dir: matches.remove(0) }),
        _ => Err(AssociationError::Ambiguous(matches)),
    }
}

/// Whether `a` and `b` name the same location; see [`canonical_worktree_path`].
/// A path that can't be resolved at all compares unequal.
pub(crate) fn same_location(a: &Path, b: &Path) -> bool {
    match (canonical_worktree_path(a), canonical_worktree_path(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::remove::handoff::canonical;
    use crate::remove::test_support::TestRepo;

    /// A real linked worktree and the admin directory Git made for it.
    fn fixture() -> (TestRepo, PathBuf, PathBuf) {
        let repo = TestRepo::new();
        let checkout = repo.add_worktree("feat/x", "x", "main");
        let admin = PathBuf::from(repo.git_in(&checkout, &["rev-parse", "--path-format=absolute", "--git-dir"]));
        (repo, checkout, admin)
    }

    /// Walks the input matrix: one edit of Git's own file per cell, after a
    /// control row proving the unedited file reads back as the checkout.
    #[test]
    fn the_back_reference_matrix_from_a_real_git_fixture() {
        let (_repo, checkout, admin) = fixture();
        let gitdir = admin.join("gitdir");
        let written = fs::read(&gitdir).unwrap();
        let path = String::from_utf8(written.clone()).unwrap().trim_end().to_string();

        let control = read_back_reference(&admin).expect("Git's own file reads");
        assert_eq!(canonical(&control), canonical(&checkout.join(".git")), "control row");

        let cell = |contents: &[u8]| {
            fs::write(&gitdir, contents).unwrap();
            read_back_reference(&admin)
        };
        assert_eq!(cell(format!("{path}\r\n").as_bytes()).map(|p| canonical(&p)), Ok(canonical(&control)), "CRLF");
        assert_eq!(cell(path.as_bytes()).map(|p| canonical(&p)), Ok(canonical(&control)), "no final newline");
        assert_eq!(cell(b""), Err(BackReferenceError::Empty), "empty");
        assert_eq!(cell(b"\n\r\n"), Err(BackReferenceError::Empty), "line endings only");
        assert_eq!(cell(format!("{path}\n{path}\n").as_bytes()), Err(BackReferenceError::MultipleLines), "two lines");
        assert_eq!(cell(format!("{path}\n\n").as_bytes()).map(|p| canonical(&p)), Ok(canonical(&control)), "trailing blank line");
        assert_eq!(cell(format!("{path} trailing\n").as_bytes()), Err(BackReferenceError::NotAGitEntry), "trailing content");
        assert_eq!(cell(b"not a path\n"), Err(BackReferenceError::NotAGitEntry), "non-path content");
        assert_eq!(cell(format!("{path}\0\n").as_bytes()), Err(BackReferenceError::ContainsNul), "NUL byte");

        let relative = cell(b"../../../../wts/x/.git\n").expect("relative path");
        assert_eq!(canonical(&relative), canonical(&control), "relative to the admin directory");

        let mut non_utf8 = b"/tmp/\xff/".to_vec();
        non_utf8.extend_from_slice(b".git\n");
        let read = cell(&non_utf8);
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            assert_eq!(read.unwrap().as_os_str().as_bytes(), &non_utf8[..non_utf8.len() - 1], "raw bytes kept");
        }
        #[cfg(not(unix))]
        assert_eq!(read, Err(BackReferenceError::NotUtf8));

        fs::remove_file(&gitdir).unwrap();
        assert_eq!(read_back_reference(&admin), Err(BackReferenceError::Missing), "absent");

        fs::create_dir(&gitdir).unwrap();
        assert!(matches!(read_back_reference(&admin), Err(BackReferenceError::Unreadable(_))), "a directory, not a file");
    }

    /// Association walks every record: one match is the control row, and
    /// each later cell edits a real Git fixture once.
    #[test]
    fn association_requires_exactly_one_readable_record() {
        let repo = TestRepo::new();
        let target = repo.add_worktree("feat/t", "t", "main");
        repo.add_worktree("feat/o", "o", "main");
        fs::remove_file(target.join(".git")).unwrap();
        let common = common_git_dir(&repo.path()).unwrap();
        let records = common.join("worktrees");
        let own = records.join("t");

        let found = admin_entry_for(&repo.path(), &target).expect("control row");
        assert_eq!(found.dir, own);
        assert_eq!(found.index(), own.join("index"));

        let other = repo.path().parent().unwrap().join("wts").join("o");
        fs::remove_dir_all(&other).unwrap();
        fs::write(&other, "replaced by a file").unwrap();
        assert_eq!(admin_entry_in(&common, &target).map(|entry| entry.dir), Ok(own.clone()), "another record's checkout replaced by a file");

        fs::write(records.join("stray-file"), "not a record").unwrap();
        assert_eq!(admin_entry_in(&common, &target).map(|entry| entry.dir), Ok(own.clone()), "a plain file is skipped");

        let duplicate = records.join("t-copy");
        fs::create_dir(&duplicate).unwrap();
        fs::copy(own.join("gitdir"), duplicate.join("gitdir")).unwrap();
        assert_eq!(admin_entry_in(&common, &target), Err(AssociationError::Ambiguous(vec![own.clone(), duplicate.clone()])), "two records, never last-wins");
        fs::remove_dir_all(&duplicate).unwrap();

        fs::write(records.join("o").join("gitdir"), "").unwrap();
        assert!(matches!(admin_entry_in(&common, &target), Err(AssociationError::Unreadable { ref dir, error: BackReferenceError::Empty }) if *dir == records.join("o")), "any unreadable record refuses");
        fs::remove_file(records.join("o").join("gitdir")).unwrap();
        assert!(matches!(admin_entry_in(&common, &target), Err(AssociationError::Unreadable { error: BackReferenceError::Missing, .. })), "a record without gitdir refuses");
        fs::remove_dir_all(records.join("o")).unwrap();

        fs::write(own.join("gitdir"), b"../../../../wts/t/.git\n").unwrap();
        assert_eq!(admin_entry_in(&common, &target).map(|entry| entry.dir), Ok(own.clone()), "relative back-reference");

        fs::write(own.join("gitdir"), format!("{}\n", repo.path().join("elsewhere").join(".git").display())).unwrap();
        assert_eq!(admin_entry_in(&common, &target), Err(AssociationError::NotFound), "no match is a refusal, never safe");
    }

    #[test]
    fn a_missing_checkout_is_associated_by_its_spelling() {
        let repo = TestRepo::new();
        let target = repo.add_worktree("feat/m", "m", "main");
        fs::remove_dir_all(&target).unwrap();
        let found = admin_entry_for(&repo.path(), &target).unwrap();
        assert!(found.dir.ends_with("worktrees/m"));
    }

    /// The base checkout's `.git` is a file here; the common directory comes
    /// from Git, not from assuming `<base>/.git` is it.
    #[test]
    fn the_common_directory_comes_from_git_not_from_the_base_layout() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let separate = dir.path().join("separate.git");
        let git = |cwd: &Path, args: &[&str]| {
            let status = std::process::Command::new("git").current_dir(cwd).args(args).output().unwrap();
            assert!(status.status.success(), "{args:?}: {}", String::from_utf8_lossy(&status.stderr));
        };
        fs::create_dir(&base).unwrap();
        git(&base, &["init", "-q", "-b", "main", "--separate-git-dir", separate.to_str().unwrap()]);
        git(&base, &["-c", "user.email=t@example.com", "-c", "user.name=T", "-c", "commit.gpgsign=false", "commit", "-q", "--allow-empty", "-m", "init"]);
        let target = dir.path().join("wt");
        git(&base, &["worktree", "add", "-q", "-b", "feat/s", target.to_str().unwrap()]);
        assert!(base.join(".git").is_file());

        let found = admin_entry_for(&base, &target).unwrap();
        assert!(same_location(&found.dir, &separate.join("worktrees").join("wt")), "{found:?}");
    }
}
