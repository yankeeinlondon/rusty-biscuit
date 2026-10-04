//! Whether Git can still read a linked worktree's checkout.
//!
//! Git marks such an entry `prunable` but its reason is localized text, so the
//! classification here comes from the filesystem alone: [`Availability::Missing`]
//! and [`Availability::Unlinked`] require positive evidence of absence
//! (`NotFound`), and anything else Git cannot read is [`Availability::Other`].
//! A classification is a snapshot for one invocation, never authorization to
//! delete.

use std::io;
use std::path::Path;

use crate::worktree::WorktreeEntry;

/// Whether Git can read a worktree entry's checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// Git does not mark the entry `prunable`.
    Healthy,
    /// The recorded directory is confirmed absent.
    Missing,
    /// The recorded path is a real directory whose `.git` is confirmed absent.
    Unlinked,
    /// Git can't read the checkout, and its state is neither of the above.
    Other(OtherCondition),
}

impl Availability {
    /// Whether Git can't read the checkout (every state but `Healthy`).
    pub fn is_unavailable(&self) -> bool {
        !matches!(self, Availability::Healthy)
    }
}

/// What was observed at a prunable entry that is neither missing nor
/// unlinked. Inspection errors keep the operating system's message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OtherCondition {
    /// The recorded path exists but is not a directory.
    NotADirectory,
    /// The recorded path is a symbolic link or Windows reparse point, dangling
    /// or not; it may have replaced the original checkout.
    Link,
    /// The recorded path could not be inspected.
    PathUninspectable(String),
    /// The checkout's `.git` could not be inspected.
    GitEntryUninspectable(String),
    /// The checkout's `.git` exists, yet Git reports the entry `prunable`.
    GitEntryPresent,
}

/// A filesystem entry's kind, read without following links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Directory,
    File,
    /// A symbolic link or, on Windows, any reparse point (junctions included).
    Link,
}

/// Classifies `entry` from the real filesystem; see [`classify_with`].
pub fn classify(entry: &WorktreeEntry) -> Availability {
    classify_with(entry, inspect)
}

/// Classifies `entry`, reading each path's kind through `inspect`.
///
/// An entry without Git's `prunable` marker is `Healthy` and is not
/// inspected. Only an `inspect` error of kind `NotFound` counts as absence;
/// permission and other I/O errors are [`Availability::Other`].
pub fn classify_with(entry: &WorktreeEntry, inspect: impl Fn(&Path) -> io::Result<EntryKind>) -> Availability {
    if entry.prunable.is_none() {
        return Availability::Healthy;
    }
    match inspect(&entry.path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Availability::Missing,
        Err(error) => Availability::Other(OtherCondition::PathUninspectable(error.to_string())),
        Ok(EntryKind::Link) => Availability::Other(OtherCondition::Link),
        Ok(EntryKind::File) => Availability::Other(OtherCondition::NotADirectory),
        Ok(EntryKind::Directory) => match inspect(&entry.path.join(".git")) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Availability::Unlinked,
            Err(error) => Availability::Other(OtherCondition::GitEntryUninspectable(error.to_string())),
            Ok(_) => Availability::Other(OtherCondition::GitEntryPresent),
        },
    }
}

/// `path`'s kind from `symlink_metadata`, never following a link.
pub fn inspect(path: &Path) -> io::Result<EntryKind> {
    std::fs::symlink_metadata(path).map(|metadata| kind_of(&metadata))
}

fn kind_of(metadata: &std::fs::Metadata) -> EntryKind {
    if metadata.file_type().is_symlink() || is_reparse_point(metadata) {
        EntryKind::Link
    } else if metadata.is_dir() {
        EntryKind::Directory
    } else {
        EntryKind::File
    }
}

// `is_symlink` covers only name-surrogate reparse points; any reparse point
// could stand in for the original checkout, so all of them count as links.
#[cfg(windows)]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &std::fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::remove::test_support::TestRepo;
    use crate::worktree::parse_worktree_list;

    fn prunable_entry(path: &Path) -> WorktreeEntry {
        WorktreeEntry {
            path: path.to_path_buf(),
            branch: Some("feat/x".into()),
            head_sha: None,
            is_main: false,
            is_current: false,
            prunable: Some("gitdir file points to non-existent location".into()),
        }
    }

    /// The real listing's entry for `path`, compared by file name because Git
    /// may spell the temp directory through a different alias.
    fn listed(repo: &TestRepo, path: &Path) -> WorktreeEntry {
        let porcelain = repo.git(&["worktree", "list", "--porcelain"]);
        parse_worktree_list(&porcelain)
            .into_iter()
            .find(|entry| entry.path.file_name() == path.file_name())
            .unwrap_or_else(|| panic!("{path:?} not listed in {porcelain}"))
    }

    #[test]
    fn git_listed_entries_classify_by_what_is_on_disk() {
        let repo = TestRepo::new();
        let healthy = repo.add_worktree("feat/healthy", "healthy", "main");
        let missing = repo.add_worktree("feat/missing", "missing", "main");
        let unlinked = repo.add_worktree("feat/unlinked", "unlinked", "main");
        fs::remove_dir_all(&missing).unwrap();
        fs::remove_file(unlinked.join(".git")).unwrap();

        let healthy = listed(&repo, &healthy);
        assert_eq!(healthy.prunable, None);
        assert_eq!(classify(&healthy), Availability::Healthy);

        let missing = listed(&repo, &missing);
        assert!(missing.prunable.is_some(), "Git marks a deleted checkout prunable");
        assert_eq!(classify(&missing), Availability::Missing);

        let unlinked = listed(&repo, &unlinked);
        assert!(unlinked.prunable.is_some(), "Git marks a checkout without .git prunable");
        assert_eq!(classify(&unlinked), Availability::Unlinked);
        assert!(unlinked.path.is_dir(), "classification leaves the directory in place");
    }

    #[test]
    fn a_healthy_entry_is_not_inspected() {
        let mut entry = prunable_entry(Path::new("/nowhere"));
        entry.prunable = None;
        let availability = classify_with(&entry, |path| panic!("inspected {path:?}"));
        assert_eq!(availability, Availability::Healthy);
    }

    #[test]
    fn a_bare_marker_is_still_classified() {
        let dir = tempfile::tempdir().unwrap();
        let mut entry = prunable_entry(&dir.path().join("gone"));
        entry.prunable = Some(String::new());
        assert_eq!(classify(&entry), Availability::Missing);
    }

    #[test]
    fn a_present_git_entry_is_other_even_when_damaged() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".git"), "garbage\n").unwrap();
        assert_eq!(classify(&prunable_entry(dir.path())), Availability::Other(OtherCondition::GitEntryPresent));
    }

    #[test]
    fn a_file_at_the_recorded_path_is_not_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("checkout");
        fs::write(&file, "").unwrap();
        assert_eq!(classify(&prunable_entry(&file)), Availability::Other(OtherCondition::NotADirectory));
    }

    #[cfg(unix)]
    #[test]
    fn a_link_at_the_recorded_path_is_never_followed() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        fs::create_dir(&real).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert_eq!(classify(&prunable_entry(&link)), Availability::Other(OtherCondition::Link), "link to a directory without .git");

        let dangling = dir.path().join("dangling");
        std::os::unix::fs::symlink(dir.path().join("absent"), &dangling).unwrap();
        assert_eq!(classify(&prunable_entry(&dangling)), Availability::Other(OtherCondition::Link), "dangling link is not absence");
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_at_the_recorded_path_is_a_link() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        fs::create_dir(&real).unwrap();
        let junction = dir.path().join("junction");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&junction)
            .arg(&real)
            .stdout(std::process::Stdio::null())
            .status()
            .expect("cmd runs");
        assert!(status.success(), "mklink /J needs no privilege");
        assert_eq!(classify(&prunable_entry(&junction)), Availability::Other(OtherCondition::Link));
    }

    #[test]
    fn inspection_errors_are_never_absence() {
        let entry = prunable_entry(Path::new("/checkout"));
        let denied = || io::Error::new(io::ErrorKind::PermissionDenied, "denied");

        let path_denied = classify_with(&entry, |_| Err(denied()));
        assert_eq!(path_denied, Availability::Other(OtherCondition::PathUninspectable("denied".into())));

        let git_entry = PathBuf::from("/checkout").join(".git");
        let git_denied = classify_with(&entry, |path| if path == git_entry { Err(denied()) } else { Ok(EntryKind::Directory) });
        assert_eq!(git_denied, Availability::Other(OtherCondition::GitEntryUninspectable("denied".into())));

        let io_error = classify_with(&entry, |_| Err(io::Error::other("disk")));
        assert!(matches!(io_error, Availability::Other(OtherCondition::PathUninspectable(_))));
    }

    #[test]
    fn an_injected_reparse_point_is_a_link() {
        let entry = prunable_entry(Path::new("/checkout"));
        assert_eq!(classify_with(&entry, |_| Ok(EntryKind::Link)), Availability::Other(OtherCondition::Link));
    }
}
