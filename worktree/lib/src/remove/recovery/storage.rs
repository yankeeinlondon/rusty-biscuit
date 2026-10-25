//! Owner-only directories and files that are read without following links.
//!
//! On Unix a directory is created `0o700` and a file `0o600`, and reading
//! either requires the current user as owner and no group or other access.
//! Windows has no mode bits; the files inherit the Git directory's ACL, and
//! reading refuses any reparse point (symlink or junction). A Windows drive
//! seen from WSL (drvfs) is the same: its mode bits are made up and `chmod`
//! does not change them, so there only ownership is checked.

use std::fs::{self, File, Metadata};
use std::io::{self, Read, Write};
use std::path::Path;

#[cfg(unix)]
use crate::remove::mounts::{Mount, MountInfo, NativeMounts, containing};

/// Why a directory or file in the store cannot be trusted. Nothing in it is
/// read once one of these is found.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StorageProblem {
    #[error("it does not exist")]
    Missing,
    #[error("it is a link or reparse point")]
    Link,
    #[error("it is not a directory")]
    NotDirectory,
    #[error("it is not a regular file")]
    NotRegularFile,
    #[error("it is owned by another user")]
    WrongOwner,
    #[error("other users can access it")]
    OpenPermissions,
    #[error("it was replaced while being opened")]
    Replaced,
    #[error("{0}")]
    Io(String),
}

impl From<io::Error> for StorageProblem {
    fn from(error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::NotFound { Self::Missing } else { Self::Io(error.to_string()) }
    }
}

/// Creates `path` as an owner-only directory; it must not exist.
pub(super) fn create_private_dir(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(0o700).create(path)
    }
    #[cfg(not(unix))]
    fs::DirBuilder::new().create(path)
}

/// Creates `path` as an owner-only directory unless it already exists, then
/// validates whichever directory is there.
pub(super) fn ensure_private_dir(path: &Path) -> Result<(), StorageProblem> {
    match create_private_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    validate_dir(path).map(drop)
}

/// The metadata of `path`, which must be a real, owner-only directory.
pub(super) fn validate_dir(path: &Path) -> Result<Metadata, StorageProblem> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link(&metadata) {
        return Err(StorageProblem::Link);
    }
    if !metadata.is_dir() {
        return Err(StorageProblem::NotDirectory);
    }
    check_owner(path, &metadata)?;
    Ok(metadata)
}

/// Reads `path`, which must be a real, owner-only regular file, and must
/// still be the same file once opened.
pub(super) fn read_private_file(path: &Path) -> Result<Vec<u8>, StorageProblem> {
    let before = fs::symlink_metadata(path)?;
    if is_link(&before) {
        return Err(StorageProblem::Link);
    }
    if !before.is_file() {
        return Err(StorageProblem::NotRegularFile);
    }
    check_owner(path, &before)?;
    let mut file = open_no_follow(path)?;
    let after = file.metadata()?;
    if !after.is_file() || !same_file(&before, &after) {
        return Err(StorageProblem::Replaced);
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Replaces `path` with `bytes`: an owner-only temporary file beside it is
/// written and flushed, renamed over `path`, and the directory is flushed.
pub(super) fn write_durable(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp = temp_beside(path)?;
    let result = (|| {
        let mut file = create_private_file(&temp)?;
        file.write_all(bytes)?;
        flush(&file)?;
        drop(file);
        publish(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// An unused hidden name beside `path` for building its replacement. It is
/// never an operation ID, so nothing reads it.
fn temp_beside(path: &Path) -> io::Result<std::path::PathBuf> {
    let parent = path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    let name = path.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let mut nonce = [0_u8; 8];
    getrandom::fill(&mut nonce).map_err(|e| io::Error::other(format!("no OS randomness: {e}")))?;
    let nonce: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(parent.join(format!(".{}.{nonce}.tmp", name.to_string_lossy())))
}

/// Renames the flushed, closed file `temp` over `path` and flushes the
/// directory: a reader of `path` sees the old file or the new one, never
/// neither. Std's rename replaces an existing file on Windows too.
fn publish(temp: &Path, path: &Path) -> io::Result<()> {
    // Replacement is the point here; the no-replacement rule is for the
    // checkout move, not for the store's own files.
    fs::rename(temp, path)?;
    sync_dir(path.parent().unwrap_or(path))
}

/// Creates `path` as a new owner-only file; it must not exist.
fn create_private_file(path: &Path) -> io::Result<File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// Flushes `file`'s data to the device.
///
/// On Apple platforms this is plain `fsync` rather than std's `F_FULLFSYNC`,
/// which also empties the drive's own cache: a power-loss guarantee the
/// store does not claim, at roughly 25 ms per call.
pub(crate) fn flush(file: &File) -> io::Result<()> {
    #[cfg(target_vendor = "apple")]
    {
        use std::os::fd::AsRawFd;
        // SAFETY: the descriptor is open for the lifetime of `file`.
        if unsafe { libc::fsync(file.as_raw_fd()) } == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
    }
    #[cfg(not(target_vendor = "apple"))]
    {
        file.sync_data()
    }
}

/// Flushes `dir`'s entries, so a rename or creation in it survives a crash.
/// A no-op on Windows, which cannot open a directory as a file.
pub(crate) fn sync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        flush(&File::open(dir)?)
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
        Ok(())
    }
}

fn is_link(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

#[cfg(unix)]
fn check_owner(path: &Path, metadata: &Metadata) -> Result<(), StorageProblem> {
    check_owner_with(path, metadata, &NativeMounts)
}

#[cfg(unix)]
fn check_owner_with(path: &Path, metadata: &Metadata, mounts: &dyn MountInfo) -> Result<(), StorageProblem> {
    use std::os::unix::fs::MetadataExt;
    // SAFETY: `geteuid` has no preconditions and cannot fail.
    if metadata.uid() != unsafe { libc::geteuid() } {
        return Err(StorageProblem::WrongOwner);
    }
    // The mount table is read only when the bits are open, which they never
    // are for a store `wt` created anywhere but drvfs.
    if metadata.mode() & 0o077 != 0 && !on_drvfs(path, mounts) {
        return Err(StorageProblem::OpenPermissions);
    }
    Ok(())
}

/// Whether `path` lives on WSL's drvfs. An unreadable table or path is not.
#[cfg(unix)]
fn on_drvfs(path: &Path, mounts: &dyn MountInfo) -> bool {
    let (Ok(table), Ok(real)) = (mounts.mounts(), biscuit_file::canonicalize_simplified(path)) else { return false };
    containing(&table, &real).is_some_and(Mount::is_drvfs)
}

#[cfg(not(unix))]
fn check_owner(_path: &Path, _metadata: &Metadata) -> Result<(), StorageProblem> {
    Ok(())
}

fn open_no_follow(path: &Path) -> io::Result<File> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    no_follow(&mut options).open(path)
}

fn no_follow(options: &mut fs::OpenOptions) -> &mut fs::OpenOptions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    options
}

#[cfg(unix)]
fn same_file(before: &Metadata, after: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    before.dev() == after.dev() && before.ino() == after.ino()
}

// Std exposes no stable file ID on Windows; the open refuses reparse points
// and size plus modification time catch an ordinary replacement.
#[cfg(not(unix))]
fn same_file(before: &Metadata, after: &Metadata) -> bool {
    before.len() == after.len() && before.modified().ok() == after.modified().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_durable_write_replaces_the_file_and_leaves_no_temporary() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");
        ensure_private_dir(&store).unwrap();
        let path = store.join("record.json");
        write_durable(&path, b"one").unwrap();
        write_durable(&path, b"two").unwrap();
        assert_eq!(read_private_file(&path).unwrap(), b"two");
        let names: Vec<_> = fs::read_dir(&store).unwrap().map(|entry| entry.unwrap().file_name()).collect();
        assert_eq!(names, vec![std::ffi::OsString::from("record.json")]);
    }

    #[cfg(unix)]
    #[test]
    fn created_storage_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");
        ensure_private_dir(&store).unwrap();
        write_durable(&store.join("f"), b"x").unwrap();
        assert_eq!(fs::metadata(&store).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(fs::metadata(store.join("f")).unwrap().permissions().mode() & 0o777, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn storage_others_can_reach_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");
        ensure_private_dir(&store).unwrap();
        let file = store.join("f");
        write_durable(&file, b"x").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(read_private_file(&file), Err(StorageProblem::OpenPermissions));
        fs::set_permissions(&store, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(ensure_private_dir(&store), Err(StorageProblem::OpenPermissions));
    }

    /// WSL's drvfs shows every entry as `0o777` and ignores `chmod`, so open
    /// bits there are not a refusal; on any other filesystem, or when the
    /// mount table cannot be read, they still are.
    #[cfg(unix)]
    #[test]
    fn open_permissions_are_accepted_only_on_drvfs() {
        use crate::remove::test_support::FixedMounts;
        use std::os::unix::fs::PermissionsExt;

        struct Unreadable;
        impl MountInfo for Unreadable {
            fn mounts(&self) -> io::Result<Vec<Mount>> {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");
        ensure_private_dir(&store).unwrap();
        fs::set_permissions(&store, fs::Permissions::from_mode(0o777)).unwrap();
        let metadata = fs::symlink_metadata(&store).unwrap();
        let real = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let mount = |fs_type: &str, super_options: &str| {
            FixedMounts(vec![Mount { mount_point: real.clone(), source: "C:\\".into(), fs_type: fs_type.into(), super_options: super_options.into() }])
        };

        let drvfs = mount("9p", "rw,aname=drvfs;path=C:\\;uid=1000;gid=1000");
        assert_eq!(check_owner_with(&store, &metadata, &drvfs), Ok(()));
        assert_eq!(check_owner_with(&store, &metadata, &mount("drvfs", "")), Ok(()));
        for (cell, mounts) in [
            ("ext4", &mount("ext4", "rw") as &dyn MountInfo),
            ("9p that is not drvfs", &mount("9p", "rw,trans=virtio")),
            ("an unreadable table", &Unreadable),
            ("no containing mount", &FixedMounts(Vec::new())),
        ] {
            assert_eq!(check_owner_with(&store, &metadata, mounts), Err(StorageProblem::OpenPermissions), "{cell}");
        }
    }

    #[test]
    fn links_are_never_followed() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        ensure_private_dir(&real).unwrap();
        write_durable(&real.join("f"), b"x").unwrap();
        let linked_dir = dir.path().join("linked");
        let linked_file = dir.path().join("linked-file");
        if symlink_dir(&real, &linked_dir).is_err() || symlink_file(&real.join("f"), &linked_file).is_err() {
            // Windows without symlink privilege cannot build this fixture.
            return;
        }
        assert_eq!(validate_dir(&linked_dir).unwrap_err(), StorageProblem::Link);
        assert_eq!(read_private_file(&linked_file), Err(StorageProblem::Link));
    }

    #[test]
    fn a_missing_file_is_reported_as_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_private_file(&dir.path().join("absent")), Err(StorageProblem::Missing));
    }

    #[cfg(unix)]
    fn symlink_dir(target: &Path, link: &Path) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn symlink_dir(target: &Path, link: &Path) -> io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    #[cfg(unix)]
    fn symlink_file(target: &Path, link: &Path) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn symlink_file(target: &Path, link: &Path) -> io::Result<()> {
        std::os::windows::fs::symlink_file(target, link)
    }
}
