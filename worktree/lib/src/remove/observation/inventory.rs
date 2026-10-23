//! The deletion inventory: every entry a removal is approved to delete, as it
//! was when the operation started.
//!
//! [`DeletionInventory::capture`] runs once, after observation is active and
//! before the decisive usage query and the move. It walks the checkout through
//! directory handles without following links and records each entry's native
//! identity, type, and the metadata a write changes (size, modification time,
//! permission bits; a link's target). Only the consent-sensitive paths (dirty
//! entries and protected included files) get a content hash; ignored build
//! output is never read. A FIFO, socket, device, nested mount, or unsupported
//! reparse point refuses before anything is moved, and is never opened. An
//! entry that vanishes while the list is taken is left out, and a folder
//! replaced meanwhile is listed but not walked: the user already agreed to
//! the removal, and deletion removes whatever is there when it gets to it.
//!
//! Directories are compared by identity and permission bits only: the
//! operation's own deletions change a directory's modification time and entry
//! count. A file's change time and link count are not recorded either:
//! deleting one hard link of a file (Cargo hard-links `target/debug/<bin>` to
//! `deps/`) changes both on every other link.
//!
//! [`DeletionInventory::confirm`] is the finishing consistency check: it
//! re-inspects exactly the paths the observer reported during the capture.
//! The inventory is persisted in the deletion journal before the move, so
//! recovery can compare what is left with what was approved.

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::io;
use std::path::{Component, Path, PathBuf};

use cap_std::ambient_authority;
use cap_std::fs::{Dir, Metadata};
use serde::{Deserialize, Serialize};
use sniff::filesystem::query::{FileIdentity, NativeString};

use super::{CoverageLost, Drain, Scope};

/// One approved entry, relative to the checkout root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryEntry {
    pub path: NativeString,
    pub identity: FileIdentity,
    pub shape: Shape,
}

/// An entry's type and the metadata compared before it is deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Shape {
    File { size: u64, modified: Timestamp, mode: u32, content: Content },
    /// `mode` is the Unix permission bits, or on Windows the read-only,
    /// hidden, and system attributes.
    Directory { mode: u32 },
    /// A symlink or junction; deleting it never touches its target.
    Link { target: NativeString },
}

/// A modification time, as seconds and nanoseconds from the Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Timestamp {
    pub secs: i64,
    pub nanos: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Content {
    /// Not consent-sensitive: changes are detected by metadata and
    /// notifications only.
    Unhashed,
    /// BLAKE3 of the bytes, as lowercase hex.
    Blake3(String),
}

/// Why an entry can never be deleted by a removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum Ineligibility {
    #[error("it is a named pipe (FIFO)")]
    Fifo,
    #[error("it is a socket")]
    Socket,
    #[error("it is a device")]
    Device,
    #[error("it is a mount point of another filesystem")]
    MountPoint,
    #[error("it is a reparse point of a kind wt does not remove")]
    ReparsePoint,
    #[error("its file type is not a file, directory, or link")]
    Unknown,
}

/// How an entry differs from its inventory record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Change {
    #[error("it appeared after the inventory was taken")]
    Appeared,
    #[error("it disappeared")]
    Vanished,
    /// Another object (or another type) now has its name.
    #[error("it was replaced")]
    Replaced,
    /// The same object with different content or metadata.
    #[error("it was modified")]
    Modified,
}

/// Why the inventory refuses the removal. Nothing has been moved yet.
#[derive(Debug, thiserror::Error)]
pub enum InventoryRefusal {
    /// Content that cannot be removed safely (exit 3).
    #[error("{} cannot be removed safely: {reason}", path.display())]
    Ineligible { path: PathBuf, reason: Ineligibility },
    /// An entry could not be read, so it cannot be approved (exit 3).
    #[error("{} cannot be inspected: {source}", path.display())]
    Uninspectable { path: PathBuf, source: io::Error },
    /// Something changed while the inventory was taken (exit 3).
    #[error("{} changed while the removal was being prepared: {change}", path.display())]
    Changed { path: PathBuf, change: Change },
    /// Observation stopped covering the checkout (exit 4: the environment
    /// cannot support the lifecycle).
    #[error("activity observation was lost: {0}")]
    Coverage(CoverageLost),
}

/// The approved content of one checkout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "InventoryRecord", into = "InventoryRecord")]
pub struct DeletionInventory {
    root: FileIdentity,
    /// Parents before children; each directory's children in byte order.
    entries: Vec<InventoryEntry>,
    index: HashMap<PathBuf, usize>,
}

/// The persisted form, validated by [`DeletionInventory::try_from`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryRecord {
    root: FileIdentity,
    entries: Vec<InventoryEntry>,
}

impl TryFrom<InventoryRecord> for DeletionInventory {
    type Error = &'static str;

    /// Every path is plain and relative, appears once, and sits in a
    /// directory listed before it (or in the root).
    fn try_from(record: InventoryRecord) -> Result<Self, Self::Error> {
        let mut index = HashMap::with_capacity(record.entries.len());
        for (position, entry) in record.entries.iter().enumerate() {
            let path = entry.path.as_path();
            if !is_plain_relative(path) {
                return Err("an inventory path is not a plain relative path");
            }
            if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
                let listed_directory = index
                    .get(parent)
                    .is_some_and(|&at: &usize| matches!(record.entries[at].shape, Shape::Directory { .. }));
                if !listed_directory {
                    return Err("an inventory entry's directory is not listed before it");
                }
            }
            if index.insert(path.to_path_buf(), position).is_some() {
                return Err("an inventory lists the same path twice");
            }
            match &entry.shape {
                Shape::File { content: Content::Blake3(digest), .. }
                    if digest.len() != 64 || !digest.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')) =>
                {
                    return Err("a content hash is not a BLAKE3 digest");
                }
                Shape::Link { target } if target.as_os_str().is_empty() || target.as_os_str().as_encoded_bytes().contains(&0) => {
                    return Err("a link target is empty or holds a NUL");
                }
                _ => {}
            }
        }
        Ok(Self { root: record.root, entries: record.entries, index })
    }
}

impl From<DeletionInventory> for InventoryRecord {
    fn from(inventory: DeletionInventory) -> Self {
        Self { root: inventory.root, entries: inventory.entries }
    }
}

/// Whether `path` is non-empty and made only of normal components.
pub(crate) fn is_plain_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.as_os_str().as_encoded_bytes().contains(&0)
        && path.components().all(|component| matches!(component, Component::Normal(_)))
}

impl DeletionInventory {
    /// Inventories the checkout at `root`. `sensitive` lists the
    /// consent-sensitive paths, relative to `root`; a file at or below one is
    /// hashed.
    ///
    /// ## Errors
    ///
    /// An [`InventoryRefusal`] naming the first ineligible or unreadable entry.
    pub fn capture(root: &Path, sensitive: &[PathBuf]) -> Result<Self, InventoryRefusal> {
        let (dir, root_identity) = open_root(root, None)?;
        let sensitive: HashSet<&Path> = sensitive.iter().map(PathBuf::as_path).collect();
        let mut walk = Walk { root, root_device: device(&root_identity), sensitive, entries: Vec::new() };
        walk.directory(&dir, Path::new(""))?;
        let index = walk.entries.iter().enumerate().map(|(at, entry)| (entry.path.as_path().to_path_buf(), at)).collect();
        Ok(Self { root: root_identity, entries: walk.entries, index })
    }

    /// An inventory of the checkout `root` that lists nothing: what a retry
    /// approves when the checkout's content is already gone.
    pub(crate) fn empty(root: FileIdentity) -> Self {
        Self { root, entries: Vec::new(), index: HashMap::new() }
    }

    /// The checkout directory's identity, which the move preserves.
    pub fn root(&self) -> &FileIdentity {
        &self.root
    }

    pub fn entries(&self) -> &[InventoryEntry] {
        &self.entries
    }

    pub fn get(&self, path: &Path) -> Option<&InventoryEntry> {
        self.index.get(path).map(|&at| &self.entries[at])
    }

    /// Where `path` is in [`entries`](Self::entries).
    pub fn position(&self, path: &Path) -> Option<usize> {
        self.index.get(path).copied()
    }

    /// The finishing consistency check: `drain` holds what the observer saw
    /// from its start until now, and every path it names under the checkout
    /// at `root` must still match this inventory.
    ///
    /// ## Errors
    ///
    /// [`InventoryRefusal::Coverage`] when observation was lost, and
    /// [`InventoryRefusal::Changed`] or [`InventoryRefusal::Uninspectable`]
    /// for the first path that differs or cannot be read.
    pub fn confirm(&self, root: &Path, drain: &Drain) -> Result<(), InventoryRefusal> {
        if let Some(lost) = &drain.lost {
            return Err(InventoryRefusal::Coverage(lost.clone()));
        }
        let (dir, _) = open_root(root, Some(&self.root))?;
        let mut checked = HashSet::new();
        for event in &drain.events {
            let relative = match &event.scope {
                Scope::Tree(relative) | Scope::Original(relative) => relative,
                // The quarantine does not exist yet (the no-replacement move
                // refuses if it does), and the admin binding is rechecked
                // separately.
                Scope::Quarantine(_) | Scope::Admin(_) => continue,
            };
            if relative.as_os_str().is_empty() || !checked.insert(relative.clone()) {
                continue;
            }
            if let Some(change) = self.check(&dir, root, relative).map_err(|source| InventoryRefusal::Uninspectable {
                path: root.join(relative),
                source,
            })? {
                return Err(InventoryRefusal::Changed { path: root.join(relative), change });
            }
        }
        Ok(())
    }

    /// How the entry at `relative` differs from its record now, reached
    /// through verified directory handles from `root`; `None` when it still
    /// matches (or is still absent and was never recorded).
    pub(crate) fn check(&self, root: &Dir, root_path: &Path, relative: &Path) -> io::Result<Option<Change>> {
        let parent = match self.open_parent(root, root_path, relative)? {
            Ok(parent) => parent,
            Err((_, change)) => return Ok(Some(change)),
        };
        let name = relative.file_name().expect("a plain relative path has a name");
        let recorded = self.get(relative);
        let hash = recorded.is_some_and(|entry| matches!(entry.shape, Shape::File { content: Content::Blake3(_), .. }));
        let now = match look(&parent, name, &root_path.join(relative), hash) {
            Ok(now) => now,
            Err(LookError::Ineligible(_)) => return Ok(Some(if recorded.is_some() { Change::Replaced } else { Change::Appeared })),
            Err(LookError::Io(error)) => return Err(error),
        };
        Ok(match (recorded, now) {
            (None, None) => None,
            (None, Some(_)) => Some(Change::Appeared),
            (Some(_), None) => Some(Change::Vanished),
            (Some(recorded), Some(now)) => compare(recorded, &now),
        })
    }

    /// Opens the directory holding `relative`, checking each directory on
    /// the way against its record. `Ok(Err((path, change)))` names the first
    /// directory that no longer matches.
    pub(crate) fn open_parent(
        &self,
        root: &Dir,
        root_path: &Path,
        relative: &Path,
    ) -> io::Result<Result<Dir, (PathBuf, Change)>> {
        let mut current = root.try_clone()?;
        let mut so_far = PathBuf::new();
        let Some(parent) = relative.parent() else { return Ok(Ok(current)) };
        for component in parent.components() {
            so_far.push(component);
            let Some(recorded) = self.get(&so_far).filter(|entry| matches!(entry.shape, Shape::Directory { .. })) else {
                return Ok(Err((so_far, Change::Appeared)));
            };
            let full = root_path.join(&so_far);
            let now = match look(&current, component.as_os_str(), &full, false) {
                Ok(Some(now)) => now,
                Ok(None) => return Ok(Err((so_far, Change::Vanished))),
                Err(LookError::Ineligible(_)) => return Ok(Err((so_far, Change::Replaced))),
                Err(LookError::Io(error)) => return Err(error),
            };
            if let Some(change) = compare(recorded, &now) {
                return Ok(Err((so_far, change)));
            }
            let opened = current.open_dir(component.as_os_str())?;
            if dir_identity(&opened, &full)? != recorded.identity {
                return Ok(Err((so_far, Change::Replaced)));
            }
            current = opened;
        }
        Ok(Ok(current))
    }
}

#[cfg(test)]
impl DeletionInventory {
    /// An inventory of `files` (and the directories holding them) under a
    /// root with identity `root`, with made-up identities, for tests of the
    /// journal format.
    pub(crate) fn listing(root: FileIdentity, files: &[&str]) -> Self {
        let mut entries: Vec<InventoryEntry> = Vec::new();
        let mut seen = HashSet::new();
        let mut inode = 10;
        let mut push = |path: &Path, shape: Shape, entries: &mut Vec<InventoryEntry>| {
            if seen.insert(path.to_path_buf()) {
                inode += 1;
                entries.push(InventoryEntry {
                    path: NativeString::from(path),
                    identity: FileIdentity::Unix { device: 1, inode },
                    shape,
                });
            }
        };
        for file in files {
            let path = Path::new(file);
            let directories: Vec<&Path> = path.ancestors().skip(1).filter(|a| !a.as_os_str().is_empty()).collect();
            for directory in directories.into_iter().rev() {
                push(directory, Shape::Directory { mode: 0o755 }, &mut entries);
            }
            let shape = Shape::File { size: 1, modified: Timestamp { secs: 1, nanos: 0 }, mode: 0o644, content: Content::Unhashed };
            push(path, shape, &mut entries);
        }
        Self::try_from(InventoryRecord { root, entries }).expect("a valid listing")
    }
}

/// How `now` differs from `recorded`; `None` when it matches.
pub(crate) fn compare(recorded: &InventoryEntry, now: &Snapshot) -> Option<Change> {
    if recorded.identity != now.identity || std::mem::discriminant(&recorded.shape) != std::mem::discriminant(&now.shape) {
        return Some(Change::Replaced);
    }
    let same = match (&recorded.shape, &now.shape) {
        (
            Shape::File { size, modified, mode, content },
            Shape::File { size: now_size, modified: now_modified, mode: now_mode, content: now_content },
        ) => {
            size == now_size
                && modified == now_modified
                && mode == now_mode
                && (matches!(content, Content::Unhashed) || content == now_content)
        }
        (Shape::Directory { mode }, Shape::Directory { mode: now_mode }) => mode == now_mode,
        (Shape::Link { target }, Shape::Link { target: now_target }) => target == now_target,
        _ => unreachable!("discriminants compared above"),
    };
    (!same).then_some(Change::Modified)
}

struct Walk<'a> {
    root: &'a Path,
    root_device: Option<u64>,
    sensitive: HashSet<&'a Path>,
    entries: Vec<InventoryEntry>,
}

impl Walk<'_> {
    fn directory(&mut self, dir: &Dir, relative: &Path) -> Result<(), InventoryRefusal> {
        let uninspectable = |path: PathBuf| move |source| InventoryRefusal::Uninspectable { path, source };
        let mut names = Vec::new();
        for entry in dir.entries().map_err(uninspectable(self.root.join(relative)))? {
            names.push(entry.map_err(uninspectable(self.root.join(relative)))?.file_name());
        }
        names.sort_by(|a, b| a.as_encoded_bytes().cmp(b.as_encoded_bytes()));
        for name in names {
            let child = relative.join(&name);
            let full = self.root.join(&child);
            let hash = self.is_sensitive(&child);
            let snapshot = match look(dir, &name, &full, hash) {
                Ok(Some(snapshot)) => snapshot,
                // Gone while the list was taken (a build cleaning up, say):
                // nothing to approve, and the user agreed to the removal.
                Ok(None) => continue,
                Err(LookError::Ineligible(reason)) => return Err(InventoryRefusal::Ineligible { path: full, reason }),
                Err(LookError::Io(source)) => return Err(InventoryRefusal::Uninspectable { path: full, source }),
            };
            let is_directory = matches!(snapshot.shape, Shape::Directory { .. });
            if is_directory && self.root_device.is_some() && device(&snapshot.identity) != self.root_device {
                return Err(InventoryRefusal::Ineligible { path: full, reason: Ineligibility::MountPoint });
            }
            let identity = snapshot.identity;
            self.entries.push(InventoryEntry { path: NativeString::from(child.as_path()), identity: snapshot.identity, shape: snapshot.shape });
            if is_directory {
                let opened = match dir.open_dir(&name) {
                    Ok(opened) => opened,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(source) => return Err(InventoryRefusal::Uninspectable { path: full, source }),
                };
                // Replaced while the list was taken: its recorded entry no
                // longer matches, so deletion removes what is there then.
                if dir_identity(&opened, &full).map_err(uninspectable(full.clone()))? != identity {
                    continue;
                }
                self.directory(&opened, &child)?;
            }
        }
        Ok(())
    }

    /// Whether `relative` or a directory above it is consent-sensitive.
    fn is_sensitive(&self, relative: &Path) -> bool {
        !self.sensitive.is_empty() && relative.ancestors().any(|ancestor| self.sensitive.contains(ancestor))
    }
}

/// An entry as it is now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub identity: FileIdentity,
    pub shape: Shape,
}

pub(crate) enum LookError {
    Ineligible(Ineligibility),
    Io(io::Error),
}

impl From<io::Error> for LookError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// The entry `name` of `parent` (at `full`), without following a link;
/// `None` when nothing has that name. A file is hashed only when `hash`.
pub(crate) fn look(parent: &Dir, name: &OsStr, full: &Path, hash: bool) -> Result<Option<Snapshot>, LookError> {
    let metadata = match parent.symlink_metadata(name) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if let Some(reason) = platform::ineligible(&metadata) {
        return Err(LookError::Ineligible(reason));
    }
    let identity = match platform::identity(&metadata, full) {
        Ok(identity) => identity,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let file_type = metadata.file_type();
    let shape = if file_type.is_symlink() {
        let target = match std::fs::read_link(full) {
            Ok(target) => target,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if platform::is_volume_mount(&target) {
            return Err(LookError::Ineligible(Ineligibility::MountPoint));
        }
        Shape::Link { target: NativeString::from(target.as_path()) }
    } else if file_type.is_dir() {
        Shape::Directory { mode: platform::mode(&metadata) }
    } else if file_type.is_file() {
        let content = if hash { Content::Blake3(hash_file(parent, name, &identity, full)?) } else { Content::Unhashed };
        Shape::File { size: metadata.len(), modified: timestamp(&metadata)?, mode: platform::mode(&metadata), content }
    } else {
        return Err(LookError::Ineligible(Ineligibility::Unknown));
    };
    Ok(Some(Snapshot { identity, shape }))
}

/// Hashes the regular file `name`, refusing if what was opened is not the
/// object with `identity`.
fn hash_file(parent: &Dir, name: &OsStr, identity: &FileIdentity, full: &Path) -> io::Result<String> {
    let mut file = platform::open_no_follow(parent, name)?.into_std();
    if platform::file_identity(&file, full)? != *identity {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "the file changed while it was opened"));
    }
    biscuit_hash::blake3_hash_reader(&mut file)
}

fn timestamp(metadata: &Metadata) -> io::Result<Timestamp> {
    let modified = metadata.modified()?.into_std();
    Ok(match modified.duration_since(std::time::UNIX_EPOCH) {
        Ok(after) => Timestamp { secs: after.as_secs() as i64, nanos: after.subsec_nanos() },
        Err(before) => {
            let before = before.duration();
            let (secs, nanos) = (before.as_secs() as i64, before.subsec_nanos());
            if nanos == 0 { Timestamp { secs: -secs, nanos: 0 } } else { Timestamp { secs: -secs - 1, nanos: 1_000_000_000 - nanos } }
        }
    })
}

/// Opens the real directory at `root`, checking it against `expected` when
/// given. Returns the handle and the directory's identity.
pub(crate) fn open_root(root: &Path, expected: Option<&FileIdentity>) -> Result<(Dir, FileIdentity), InventoryRefusal> {
    let uninspectable = |source| InventoryRefusal::Uninspectable { path: root.to_path_buf(), source };
    let metadata = std::fs::symlink_metadata(root).map_err(uninspectable)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || platform::is_reparse_point(&metadata) {
        return Err(InventoryRefusal::Changed { path: root.to_path_buf(), change: Change::Replaced });
    }
    let dir = Dir::open_ambient_dir(root, ambient_authority()).map_err(uninspectable)?;
    let identity = dir_identity(&dir, root).map_err(uninspectable)?;
    let before = crate::remove::quarantine::rename::identity(root).map_err(uninspectable)?;
    if identity != before || expected.is_some_and(|expected| *expected != identity) {
        return Err(InventoryRefusal::Changed { path: root.to_path_buf(), change: Change::Replaced });
    }
    Ok((dir, identity))
}

/// The identity of the directory `dir` refers to (`full` names it, for
/// platforms that read identity by path).
pub(crate) fn dir_identity(dir: &Dir, full: &Path) -> io::Result<FileIdentity> {
    platform::dir_identity(dir, full)
}

pub(crate) fn device(identity: &FileIdentity) -> Option<u64> {
    match identity {
        FileIdentity::Unix { device, .. } => Some(*device),
        FileIdentity::Windows { .. } => None,
    }
}

#[cfg(unix)]
mod platform {
    use std::io;
    use std::path::Path;

    use cap_std::fs::{Dir, File, FileTypeExt, Metadata, MetadataExt, OpenOptions, OpenOptionsExt};
    use sniff::filesystem::query::FileIdentity;

    use super::Ineligibility;

    pub(super) fn ineligible(metadata: &Metadata) -> Option<Ineligibility> {
        let file_type = metadata.file_type();
        if file_type.is_fifo() {
            Some(Ineligibility::Fifo)
        } else if file_type.is_socket() {
            Some(Ineligibility::Socket)
        } else if file_type.is_block_device() || file_type.is_char_device() {
            Some(Ineligibility::Device)
        } else {
            None
        }
    }

    pub(super) fn identity(metadata: &Metadata, _full: &Path) -> io::Result<FileIdentity> {
        Ok(FileIdentity::Unix { device: metadata.dev(), inode: metadata.ino() })
    }

    pub(super) fn dir_identity(dir: &Dir, _full: &Path) -> io::Result<FileIdentity> {
        let metadata = dir.dir_metadata()?;
        Ok(FileIdentity::Unix { device: metadata.dev(), inode: metadata.ino() })
    }

    pub(super) fn file_identity(file: &std::fs::File, _full: &Path) -> io::Result<FileIdentity> {
        use std::os::unix::fs::MetadataExt as _;
        let metadata = file.metadata()?;
        Ok(FileIdentity::Unix { device: metadata.dev(), inode: metadata.ino() })
    }

    pub(super) fn mode(metadata: &Metadata) -> u32 {
        metadata.mode() & 0o7777
    }

    pub(super) fn open_no_follow(parent: &Dir, name: &std::ffi::OsStr) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        parent.open_with(name, &options)
    }

    pub(super) fn is_reparse_point(_metadata: &std::fs::Metadata) -> bool {
        false
    }

    pub(super) fn is_volume_mount(_target: &Path) -> bool {
        false
    }
}

#[cfg(windows)]
mod platform {
    use std::io;
    use std::os::windows::io::AsRawHandle;
    use std::path::Path;

    use cap_std::fs::{Dir, File, Metadata, MetadataExt, OpenOptions, OpenOptionsExt};
    use sniff::filesystem::query::FileIdentity;

    use super::Ineligibility;
    use crate::remove::quarantine::rename;

    const FILE_ATTRIBUTE_READONLY: u32 = 0x1;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

    pub(super) fn ineligible(metadata: &Metadata) -> Option<Ineligibility> {
        // Symlinks and junctions are links; any other reparse point (cloud
        // placeholders, deduplicated files) is not removed.
        let reparse = metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
        (reparse && !metadata.file_type().is_symlink()).then_some(Ineligibility::ReparsePoint)
    }

    /// By path: the 128-bit file ID, since the 64-bit index is not unique on
    /// ReFS.
    pub(super) fn identity(_metadata: &Metadata, full: &Path) -> io::Result<FileIdentity> {
        rename::identity(full)
    }

    pub(super) fn dir_identity(dir: &Dir, _full: &Path) -> io::Result<FileIdentity> {
        rename::handle_identity(dir.as_raw_handle())
    }

    pub(super) fn file_identity(file: &std::fs::File, _full: &Path) -> io::Result<FileIdentity> {
        rename::handle_identity(file.as_raw_handle())
    }

    pub(super) fn mode(metadata: &Metadata) -> u32 {
        metadata.file_attributes() & (FILE_ATTRIBUTE_READONLY | FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM)
    }

    pub(super) fn open_no_follow(parent: &Dir, name: &std::ffi::OsStr) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
        parent.open_with(name, &options)
    }

    pub(super) fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
        use std::os::windows::fs::MetadataExt as _;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }

    /// A junction to a volume (`\\?\Volume{…}\`) is a mounted filesystem, not
    /// a link this removal may delete.
    pub(super) fn is_volume_mount(target: &Path) -> bool {
        let bytes = target.as_os_str().as_encoded_bytes();
        bytes.starts_with(br"\\?\Volume{") || bytes.starts_with(br"\??\Volume{")
    }
}

#[cfg(test)]
mod tests;
