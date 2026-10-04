//! A linked worktree's administrative entry, `<common-git-dir>/worktrees/<id>/`,
//! and its `gitdir` back-reference to the checkout's `.git`.
//!
//! The back-reference is read strictly: removal associates an entry with its
//! target only through a path this reader accepted, so a file Git itself
//! would not have written is refused rather than guessed at.

use std::path::{Path, PathBuf};

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
}
