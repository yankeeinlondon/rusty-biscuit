//! Publishing Claudine's `darkmatter-host` registration.
//!
//! Darkmatter (`md` and DMLS) learns Claudine's schemas, globals, and
//! lifecycle from a format 2 `darkmatter-host` manifest that a `register`
//! entry of `.darkmatter.yaml` names ([`MANIFEST_REFERENCE`]). This module
//! writes that manifest and every file it lists from the copies compiled into
//! this binary ([`RegistrationInventory::claudine`]); nothing is ever copied
//! by hand.
//!
//! ## Layout
//!
//! ```text
//! ~/.claudine/registrations/
//! ├── .darkmatter.lock           the publisher lock (retained)
//! └── darkmatter/                the output root
//!     ├── manifest.yaml          every member and its BLAKE3 digest
//!     ├── globals.lifecycle.json
//!     └── schemas/…              the files of `claudine/registration/`
//! ```
//!
//! There is one member tree at fixed paths: no release or version folders.
//! Every published path is the inventory's ([`PublishedFile::path`]), so
//! imports between members resolve as they do in the embedded bundle.
//!
//! ## Contract
//!
//! - **Preflight.** Before anything is created or written, every directory
//!   from `~/.claudine` down to each destination, every destination, the
//!   lock file, and every authorized removal is examined without following
//!   links. A symbolic link, a junction, or another reparse point is an
//!   obstruction ([`PublishError::Obstructed`]): the publish stops, and the
//!   link and its target are left alone. The examination is repeated under
//!   the lock.
//! - **Serialized.** Cooperating publishers (threads or processes) take one
//!   OS file lock, [`lock_path`], waiting at most [`LOCK_DEADLINE`].
//! - **Members, then the manifest.** Under the lock, each member whose bytes
//!   differ from the inventory's is replaced through a sibling temporary file
//!   ([`atomic_write_registration`], with the Windows replacement retry);
//!   entries the previous valid format 2 manifest listed and the inventory no
//!   longer has are removed; regular files named with [`TEMP_PREFIX`] in a
//!   publisher directory (leftovers of an interrupted replacement) are
//!   removed. The manifest is replaced last, and only when its bytes differ.
//! - **Unknown files are kept.** Anything else in the output root is not
//!   the publisher's to remove; doctor reports it.
//! - **No-op.** A publish of what is already installed writes nothing, so
//!   every file keeps its modification time ([`PublishOutcome::Current`]).
//!
//! ## Readers and interruption
//!
//! Darkmatter verifies every member against the manifest's digests
//! (`darkmatter::config::registration::bundle::load_snapshot`). While a
//! publish is in progress, or after one stopped partway, the old manifest
//! still stands and some members no longer match it, so a reader refuses the
//! host with a located `dm.host.registration` warning; it never accepts a mix
//! of two publishes. The next publish converges on the embedded bytes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::config::atomic::{REGISTRATION_TEMP_PREFIX, atomic_write_registration};

/// The host identifier the manifest and the lifecycle artifact record.
pub const HOST_ID: &str = "claudine";

/// The `register` entry that names Claudine's manifest.
pub const MANIFEST_REFERENCE: &str = "~/.claudine/registrations/darkmatter/manifest.yaml";

/// The `register` entry of the retired format 1 publisher, which setup
/// replaces with [`MANIFEST_REFERENCE`].
pub const LEGACY_MANIFEST_REFERENCE: &str = "~/.claudine/darkmatter-host.yaml";

/// The manifest's path inside the output root.
pub const MANIFEST_FILE: &str = "manifest.yaml";

/// The prefix of a publisher temporary file; see
/// [`REGISTRATION_TEMP_PREFIX`].
pub const TEMP_PREFIX: &str = REGISTRATION_TEMP_PREFIX;

/// How long a publisher waits for another to release [`lock_path`] before
/// failing with [`PublishError::LockTimeout`].
pub const LOCK_DEADLINE: Duration = Duration::from_secs(10);

/// The pause between attempts to take a lock another publisher holds.
pub const LOCK_BACKOFF: Duration = Duration::from_millis(20);

/// FILE_ATTRIBUTE_REPARSE_POINT, as Win32 defines it.
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

const CLAUDINE_DIR: &str = ".claudine";
const REGISTRATIONS_DIR: &str = "registrations";
const OUTPUT_DIR: &str = "darkmatter";
const LOCK_FILE: &str = ".darkmatter.lock";
const LEGACY_MANIFEST_FILE: &str = "darkmatter-host.yaml";
const LEGACY_REVISIONS_DIR: &str = "darkmatter-host";

/// `~/.claudine/registrations`: the lock file and the output root.
#[must_use]
pub fn registrations_dir(home: &Path) -> PathBuf {
    home.join(CLAUDINE_DIR).join(REGISTRATIONS_DIR)
}

/// The output root, `~/.claudine/registrations/darkmatter`.
#[must_use]
pub fn output_root(home: &Path) -> PathBuf {
    registrations_dir(home).join(OUTPUT_DIR)
}

/// The manifest, `~/.claudine/registrations/darkmatter/manifest.yaml`.
#[must_use]
pub fn manifest_path(home: &Path) -> PathBuf {
    output_root(home).join(MANIFEST_FILE)
}

/// The publisher lock, `~/.claudine/registrations/.darkmatter.lock`.
#[must_use]
pub fn lock_path(home: &Path) -> PathBuf {
    registrations_dir(home).join(LOCK_FILE)
}

/// The retired format 1 manifest, `~/.claudine/darkmatter-host.yaml`.
#[must_use]
pub fn legacy_manifest_path(home: &Path) -> PathBuf {
    home.join(CLAUDINE_DIR).join(LEGACY_MANIFEST_FILE)
}

/// The retired format 1 revision tree, `~/.claudine/darkmatter-host/`.
#[must_use]
pub fn legacy_revisions_dir(home: &Path) -> PathBuf {
    home.join(CLAUDINE_DIR).join(LEGACY_REVISIONS_DIR)
}

mod published;

pub use published::{FileKind, PublishedFile, RegistrationInventory};
use published::portable_path;

// ── outcome and errors ──────────────────────────────────────────────────────

/// What [`publish`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishOutcome {
    /// Everything installed already held the inventory's bytes; nothing was
    /// written or removed.
    Current {
        /// The manifest path.
        manifest: PathBuf,
    },
    /// At least one file was written or removed; the manifest now lists the
    /// inventory.
    Published {
        /// The manifest path.
        manifest: PathBuf,
        #[allow(missing_docs)]
        changes: PublishChanges,
    },
}

impl PublishOutcome {
    /// The manifest path.
    #[must_use]
    pub fn manifest(&self) -> &Path {
        match self {
            Self::Current { manifest } | Self::Published { manifest, .. } => manifest,
        }
    }

    /// What changed; `None` when the installation was current.
    #[must_use]
    pub const fn changes(&self) -> Option<&PublishChanges> {
        match self {
            Self::Current { .. } => None,
            Self::Published { changes, .. } => Some(changes),
        }
    }
}

/// The files a [`PublishOutcome::Published`] changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PublishChanges {
    /// Published paths created or replaced, in the order written; the
    /// manifest, when written, is last.
    pub written: Vec<String>,
    /// Paths the previous manifest listed, no longer in the inventory, that
    /// were removed.
    pub retired: Vec<String>,
    /// Leftover temporary files removed.
    pub removed_temps: Vec<PathBuf>,
}

impl PublishChanges {
    /// Whether the manifest was replaced.
    #[must_use]
    pub fn manifest_written(&self) -> bool {
        self.written.iter().any(|path| path == MANIFEST_FILE)
    }

    fn is_empty(&self) -> bool {
        self.written.is_empty() && self.retired.is_empty() && self.removed_temps.is_empty()
    }
}

/// A link the publisher refuses to traverse or replace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Obstruction {
    /// A symbolic link.
    SymbolicLink,
    /// A junction or another reparse point (Windows).
    ReparsePoint,
}

impl std::fmt::Display for Obstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::SymbolicLink => "a symbolic link",
            Self::ReparsePoint => "a junction or other reparse point",
        })
    }
}

/// Why [`publish`] stopped. Members may have been replaced before the
/// failure, but the manifest was not: the previous manifest is intact, and
/// the next publish converges.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PublishError {
    /// An inventory path cannot be published as given.
    #[error("the registration path `{path}` {reason}, so nothing was published")]
    InvalidPath {
        #[allow(missing_docs)]
        path: String,
        #[allow(missing_docs)]
        reason: String,
    },
    /// A link stands where the publisher would traverse, write, or remove.
    #[error("`{}` is {obstruction}; the publisher never follows or replaces one, so nothing was published. Remove it and publish again", .path.display())]
    Obstructed {
        #[allow(missing_docs)]
        path: PathBuf,
        #[allow(missing_docs)]
        obstruction: Obstruction,
    },
    /// Something other than a directory stands where a directory belongs.
    #[error("`{}` is not a directory, so nothing was published", .path.display())]
    NotADirectory {
        #[allow(missing_docs)]
        path: PathBuf,
    },
    /// Something other than a regular file stands where a file is written.
    #[error("`{}` is not a regular file, so nothing was published", .path.display())]
    NotAFile {
        #[allow(missing_docs)]
        path: PathBuf,
    },
    /// Another publisher held the lock for the whole deadline.
    #[error("another publisher held `{}` for {waited:?}; nothing was published", .lock.display())]
    LockTimeout {
        #[allow(missing_docs)]
        lock: PathBuf,
        #[allow(missing_docs)]
        waited: Duration,
    },
    /// A file-system operation failed.
    #[error("could not {action} `{}`: {source}", .path.display())]
    Io {
        /// What was being done (`read`, `write`, `remove`, …).
        action: &'static str,
        #[allow(missing_docs)]
        path: PathBuf,
        #[allow(missing_docs)]
        source: io::Error,
    },
    /// A test observer stopped the publish ([`publish_with`]).
    #[doc(hidden)]
    #[error("the publish was stopped after {step}")]
    Interrupted {
        #[allow(missing_docs)]
        step: String,
    },
}

fn io_error(action: &'static str, path: &Path) -> impl FnOnce(io::Error) -> PublishError + use<> {
    let path = path.to_path_buf();
    move |source| PublishError::Io { action, path, source }
}

// ── test seams ──────────────────────────────────────────────────────────────

/// A point in a publish an observer of [`publish_with`] sees.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PublishStep {
    /// The lock is held and the second preflight passed; nothing written yet.
    Locked,
    /// The member at this path was replaced.
    MemberWritten(String),
    /// The retired entry at this path was removed.
    Retired(String),
    /// Every member is in place; the manifest is next.
    BeforeManifest,
}

/// Options for [`publish_with`].
#[doc(hidden)]
pub struct PublishHooks<'a> {
    /// Replaces [`LOCK_DEADLINE`].
    pub lock_deadline: Duration,
    /// Sees each [`PublishStep`]; `Break` stops the publish there with
    /// [`PublishError::Interrupted`], as a crash at that point would.
    pub observer: &'a mut dyn FnMut(&PublishStep) -> ControlFlow<()>,
}

// ── publishing ──────────────────────────────────────────────────────────────

/// Publishes `inventory` under `home`, or does nothing when the installation
/// already holds it. See the module docs for the layout and the contract.
///
/// `home` is the user's home as the caller captured it; neither the process
/// environment nor the working directory is read.
///
/// ## Errors
///
/// A [`PublishError`]: an inventory path that cannot be published, an
/// obstruction or misplaced entry found by the preflight (before anything
/// is written), a lock held past [`LOCK_DEADLINE`], or a failed file-system
/// operation. The previous manifest stays in place in every case.
pub fn publish(home: &Path, inventory: &RegistrationInventory) -> Result<PublishOutcome, PublishError> {
    let mut observer = |_: &PublishStep| ControlFlow::Continue(());
    publish_with(home, inventory, PublishHooks { lock_deadline: LOCK_DEADLINE, observer: &mut observer })
}

/// [`publish`] with a lock deadline and an observer, for tests that hold
/// the lock or stop a publish partway.
///
/// ## Errors
///
/// As [`publish`], plus [`PublishError::Interrupted`] when the observer
/// breaks.
#[doc(hidden)]
pub fn publish_with(
    home: &Path,
    inventory: &RegistrationInventory,
    hooks: PublishHooks<'_>,
) -> Result<PublishOutcome, PublishError> {
    if let Some((path, reason)) = inventory.invalid_path() {
        return Err(PublishError::InvalidPath { path: path.to_string(), reason });
    }
    let layout = Layout::new(home);
    // Before anything is created, the lock file and its folder included.
    layout.preflight(inventory)?;
    fs::create_dir_all(layout.registrations()).map_err(io_error("create", &layout.registrations()))?;
    let _lock = acquire_lock(&lock_path(home), hooks.lock_deadline)?;
    // Another publisher may have changed the tree while this one waited.
    let plan = layout.preflight(inventory)?;
    let observer = hooks.observer;
    let stop = |step: PublishStep, observer: &mut dyn FnMut(&PublishStep) -> ControlFlow<()>| match observer(&step) {
        ControlFlow::Continue(()) => Ok(()),
        ControlFlow::Break(()) => Err(PublishError::Interrupted { step: format!("{step:?}") }),
    };
    stop(PublishStep::Locked, observer)?;

    let mut changes = PublishChanges::default();
    for directory in &plan.directories {
        changes.removed_temps.extend(remove_temps(directory)?);
    }
    for file in inventory.members() {
        let destination = layout.destination(file.path());
        if holds(&destination, file.bytes())? {
            continue;
        }
        atomic_write_registration(&destination, file.bytes()).map_err(io_error("write", &destination))?;
        changes.written.push(file.path().to_string());
        stop(PublishStep::MemberWritten(file.path().to_string()), observer)?;
    }
    for path in &plan.retired {
        let destination = layout.destination(path);
        match fs::remove_file(&destination) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(io_error("remove", &destination)(error)),
        }
        remove_empty_parents(&layout, path, inventory);
        changes.retired.push(path.clone());
        stop(PublishStep::Retired(path.clone()), observer)?;
    }
    stop(PublishStep::BeforeManifest, observer)?;
    let manifest = layout.destination(MANIFEST_FILE);
    if !holds(&manifest, inventory.manifest().bytes())? {
        atomic_write_registration(&manifest, inventory.manifest().bytes()).map_err(io_error("write", &manifest))?;
        changes.written.push(MANIFEST_FILE.to_string());
    }

    if changes.is_empty() {
        tracing::debug!(manifest = %manifest.display(), "the host registration is current");
        Ok(PublishOutcome::Current { manifest })
    } else {
        tracing::debug!(manifest = %manifest.display(), ?changes, "published the host registration");
        Ok(PublishOutcome::Published { manifest, changes })
    }
}

/// Whether the regular file `path` holds exactly `bytes`. The preflight
/// already refused anything at `path` but a regular file or nothing.
fn holds(path: &Path, bytes: &[u8]) -> Result<bool, PublishError> {
    match fs::read(path) {
        Ok(found) => Ok(found == bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error("read", path)(error)),
    }
}

/// Removes every regular file named with [`TEMP_PREFIX`] directly in
/// `directory`, when it exists.
fn remove_temps(directory: &Path) -> Result<Vec<PathBuf>, PublishError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_error("list", directory)(error)),
    };
    let mut removed = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io_error("list", directory))?;
        if !entry.file_name().to_string_lossy().starts_with(TEMP_PREFIX) {
            continue;
        }
        let path = entry.path();
        if entry_at(&path)? != Entry::File {
            continue;
        }
        match fs::remove_file(&path) {
            Ok(()) => removed.push(path),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error("remove", &path)(error)),
        }
    }
    Ok(removed)
}

/// Removes the folders of the retired `path` that no member needs and that
/// are now empty; a folder that still holds anything stays.
fn remove_empty_parents(layout: &Layout, path: &str, inventory: &RegistrationInventory) {
    let mut folder = path;
    while let Some((parent, _)) = folder.rsplit_once('/') {
        let prefix = format!("{parent}/");
        if inventory.files().iter().any(|file| file.path().starts_with(&prefix)) {
            return;
        }
        if fs::remove_dir(layout.destination(parent)).is_err() {
            return;
        }
        folder = parent;
    }
}

/// A held publisher lock, released when dropped. The lock file stays.
struct PublisherLock {
    _file: File,
}

fn acquire_lock(lock: &Path, deadline: Duration) -> Result<PublisherLock, PublishError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock)
        .map_err(io_error("open the lock", lock))?;
    let started = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(PublisherLock { _file: file }),
            Err(TryLockError::WouldBlock) => {
                let waited = started.elapsed();
                if waited >= deadline {
                    return Err(PublishError::LockTimeout { lock: lock.to_path_buf(), waited });
                }
                std::thread::sleep(LOCK_BACKOFF.min(deadline - waited));
            }
            Err(TryLockError::Error(source)) => {
                return Err(PublishError::Io { action: "lock", path: lock.to_path_buf(), source });
            }
        }
    }
}

// ── preflight ───────────────────────────────────────────────────────────────

/// What a path holds, read without following links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    Absent,
    File,
    Directory,
    Obstruction(Obstruction),
    /// A socket, device, or anything else.
    Other,
}

/// Classifies an entry from its no-follow file type and its Windows file
/// attributes (`0` elsewhere). A symbolic link wins over the reparse bit it
/// also carries on Windows; a junction carries only the bit.
fn obstruction_of(is_symlink: bool, attributes: u32) -> Option<Obstruction> {
    if is_symlink {
        Some(Obstruction::SymbolicLink)
    } else if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        Some(Obstruction::ReparsePoint)
    } else {
        None
    }
}

#[cfg(windows)]
fn attributes(metadata: &fs::Metadata) -> u32 {
    std::os::windows::fs::MetadataExt::file_attributes(metadata)
}

#[cfg(not(windows))]
const fn attributes(_: &fs::Metadata) -> u32 {
    0
}

fn entry_at(path: &Path) -> Result<Entry, PublishError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Entry::Absent),
        Err(error) => return Err(io_error("inspect", path)(error)),
    };
    Ok(match obstruction_of(metadata.file_type().is_symlink(), attributes(&metadata)) {
        Some(obstruction) => Entry::Obstruction(obstruction),
        None if metadata.is_dir() => Entry::Directory,
        None if metadata.is_file() => Entry::File,
        None => Entry::Other,
    })
}

/// The publisher's paths under one home.
struct Layout {
    home: PathBuf,
}

/// What the preflight found.
struct Plan {
    /// Every existing publisher directory: the output root and each folder
    /// holding a member or a retired entry.
    directories: Vec<PathBuf>,
    /// Retired regular files to remove, by published path.
    retired: Vec<String>,
}

impl Layout {
    fn new(home: &Path) -> Self {
        Self { home: home.to_path_buf() }
    }

    fn registrations(&self) -> PathBuf {
        registrations_dir(&self.home)
    }

    fn destination(&self, path: &str) -> PathBuf {
        let mut destination = output_root(&self.home);
        destination.extend(path.split('/'));
        destination
    }

    /// The entry at the end of `names` below the home, after checking that
    /// every folder on the way is a real directory. An absent folder ends
    /// the walk: nothing below it exists.
    fn route(&self, names: &[&str]) -> Result<Entry, PublishError> {
        let mut path = self.home.clone();
        let Some((last, folders)) = names.split_last() else {
            return Ok(Entry::Directory);
        };
        for folder in folders {
            path.push(folder);
            match entry_at(&path)? {
                Entry::Directory => {}
                Entry::Absent => return Ok(Entry::Absent),
                Entry::Obstruction(obstruction) => return Err(PublishError::Obstructed { path, obstruction }),
                Entry::File | Entry::Other => return Err(PublishError::NotADirectory { path }),
            }
        }
        path.push(last);
        match entry_at(&path)? {
            Entry::Obstruction(obstruction) => Err(PublishError::Obstructed { path, obstruction }),
            entry => Ok(entry),
        }
    }

    fn output_names(path: &str) -> Vec<&str> {
        [CLAUDINE_DIR, REGISTRATIONS_DIR, OUTPUT_DIR].into_iter().chain(path.split('/')).collect()
    }

    /// A file the publisher writes: absent, or a regular file.
    fn writable(&self, names: &[&str]) -> Result<(), PublishError> {
        match self.route(names)? {
            Entry::Absent | Entry::File => Ok(()),
            _ => Err(PublishError::NotAFile { path: names.iter().fold(self.home.clone(), |path, name| path.join(name)) }),
        }
    }

    fn preflight(&self, inventory: &RegistrationInventory) -> Result<Plan, PublishError> {
        self.writable(&[CLAUDINE_DIR, REGISTRATIONS_DIR, LOCK_FILE])?;
        match self.route(&[CLAUDINE_DIR, REGISTRATIONS_DIR, OUTPUT_DIR])? {
            Entry::Absent | Entry::Directory => {}
            _ => return Err(PublishError::NotADirectory { path: output_root(&self.home) }),
        }
        for file in inventory.files() {
            self.writable(&Self::output_names(file.path()))?;
        }

        let known: BTreeSet<&str> = inventory.files().iter().map(PublishedFile::path).collect();
        let mut retired = Vec::new();
        for path in self.previous_members()? {
            if known.contains(path.as_str()) {
                continue;
            }
            // A folder or anything else that is not a regular file is not
            // the publisher's: it is left for doctor to report.
            if self.route(&Self::output_names(&path))? == Entry::File {
                retired.push(path);
            }
        }

        let mut folders: BTreeSet<String> = BTreeSet::from([String::new()]);
        for path in inventory.files().iter().map(PublishedFile::path).chain(retired.iter().map(String::as_str)) {
            let mut folder = path;
            while let Some((parent, _)) = folder.rsplit_once('/') {
                folders.insert(parent.to_string());
                folder = parent;
            }
        }
        let mut directories = Vec::new();
        for folder in folders {
            let names: Vec<&str> = [CLAUDINE_DIR, REGISTRATIONS_DIR, OUTPUT_DIR]
                .into_iter()
                .chain(folder.split('/').filter(|name| !name.is_empty()))
                .collect();
            if self.route(&names)? == Entry::Directory {
                directories.push(self.destination_folder(&folder));
            }
        }
        Ok(Plan { directories, retired })
    }

    fn destination_folder(&self, folder: &str) -> PathBuf {
        if folder.is_empty() { output_root(&self.home) } else { self.destination(folder) }
    }

    /// The published paths the installed manifest lists, when it is a valid
    /// format 2 Claudine manifest; nothing otherwise. Only entries that name
    /// a portable path inside the output root are returned, so a removal can
    /// never leave it.
    fn previous_members(&self) -> Result<Vec<String>, PublishError> {
        let path = self.destination(MANIFEST_FILE);
        if self.route(&Self::output_names(MANIFEST_FILE))? != Entry::File {
            return Ok(Vec::new());
        }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(io_error("read", &path)(error)),
        };
        let Some(manifest) = PreviousManifest::parse(&bytes) else {
            tracing::debug!(manifest = %path.display(), "the installed manifest is not a valid format 2 Claudine manifest; nothing is retired");
            return Ok(Vec::new());
        };
        Ok(manifest
            .references()
            .filter_map(|reference| reference.strip_prefix("./"))
            .filter(|relative| portable_path(relative).is_ok())
            .map(str::to_string)
            .collect())
    }
}

/// The installed manifest, read only to find retired members: the closed
/// format 2 envelope with Claudine's host.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviousManifest {
    kind: String,
    format_version: u64,
    host: String,
    #[serde(rename = "version")]
    _version: String,
    files: Vec<String>,
    lifecycle: String,
    functions: Vec<PreviousFunctionPair>,
    digests: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviousFunctionPair {
    catalog: String,
    facts: String,
}

impl PreviousManifest {
    fn parse(bytes: &[u8]) -> Option<Self> {
        let manifest: Self = serde_yaml_ng::from_slice(bytes).ok()?;
        let valid = manifest.kind == "darkmatter-host"
            && manifest.format_version == 2
            && manifest.host == HOST_ID
            && manifest.references().all(|reference| manifest.digests.contains_key(reference))
            && manifest.digests.len() == manifest.references().count();
        valid.then_some(manifest)
    }

    fn references(&self) -> impl Iterator<Item = &str> {
        self.files
            .iter()
            .map(String::as_str)
            .chain([self.lifecycle.as_str()])
            .chain(self.functions.iter().flat_map(|pair| [pair.catalog.as_str(), pair.facts.as_str()]))
    }
}

mod legacy;

pub use legacy::{CleanupError, LegacyCleanup, remove_legacy_registration};

#[cfg(test)]
mod tests {
    use super::published::blake3_hex;
    use super::*;
    use crate::composition::inventory;

    #[test]
    fn registration_reparse_classification_reads_the_attribute_bits() {
        const READONLY: u32 = 0x1;
        const DIRECTORY: u32 = 0x10;
        assert_eq!(obstruction_of(false, 0), None);
        assert_eq!(obstruction_of(false, DIRECTORY | READONLY), None);
        assert_eq!(obstruction_of(false, DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT), Some(Obstruction::ReparsePoint));
        assert_eq!(obstruction_of(false, FILE_ATTRIBUTE_REPARSE_POINT), Some(Obstruction::ReparsePoint));
        assert_eq!(obstruction_of(true, FILE_ATTRIBUTE_REPARSE_POINT), Some(Obstruction::SymbolicLink));
        assert_eq!(obstruction_of(true, 0), Some(Obstruction::SymbolicLink));
    }

    #[test]
    fn registration_paths_must_be_portable_and_unreserved() {
        for path in ["manifest.yaml", "schemas/seq.yaml", "schemas/nested/a_b-2.yaml", "globals.lifecycle.json"] {
            assert_eq!(portable_path(path), Ok(()), "{path}");
        }
        let reserved = format!("schemas/{TEMP_PREFIX}x");
        for path in ["", "/abs.yaml", "a//b", "./a", "../a", "a\\b", "a:b", "a b", "trailing.", "schemas/con.yaml", &reserved] {
            assert!(portable_path(path).is_err(), "{path:?}");
        }
    }

    #[test]
    fn registration_inventory_lists_every_embedded_member_and_derives_its_manifest() {
        let inventory = RegistrationInventory::claudine();
        let paths: Vec<&str> = inventory.files().iter().map(PublishedFile::path).collect();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        assert_eq!(paths, sorted, "path order");
        assert_eq!(inventory.members().count(), inventory::members().len() + 1, "every member and the lifecycle artifact");
        assert_eq!(inventory.invalid_path(), None);
        let manifest = inventory.manifest().text().unwrap();
        for file in inventory.members() {
            let line = format!("    \"./{}\": \"{}\"\n", file.path(), blake3_hex(file.bytes()));
            assert!(manifest.contains(&line), "{line}");
        }
        let parsed = PreviousManifest::parse(inventory.manifest().bytes()).expect("its own manifest is a valid previous one");
        assert_eq!(parsed.files.len(), inventory::members().len());
        assert_eq!(parsed.lifecycle, format!("./{}", inventory::LIFECYCLE_BUNDLE_PATH));
    }

    #[test]
    fn registration_manifest_changes_with_any_member_byte_and_the_version() {
        let base = RegistrationInventory::claudine().clone();
        let renamed = base.clone().with_version("9.9.9");
        let edited = base.clone().with_member(PublishedFile::new(
            "schemas/shared.yaml",
            FileKind::Library,
            None::<&'static str>,
            b"kind: schema\ntypes:\n    x: string\n".to_vec(),
        ));
        assert_ne!(base.manifest(), renamed.manifest());
        assert_ne!(base.manifest(), edited.manifest());
        assert_eq!(base.files().len(), edited.files().len());
    }

    #[test]
    fn registration_previous_manifest_must_be_claudines_closed_format_2() {
        let valid = "kind: darkmatter-host\nformat_version: 2\nhost: claudine\nversion: \"1\"\nfiles: [./schemas/a.yaml]\n\
                     lifecycle: ./globals.lifecycle.json\nfunctions: []\ndigests:\n  ./schemas/a.yaml: x\n  ./globals.lifecycle.json: y\n";
        assert!(PreviousManifest::parse(valid.as_bytes()).is_some());
        for invalid in [
            valid.replace("format_version: 2", "format_version: 1"),
            valid.replace("host: claudine", "host: other"),
            valid.replace("functions: []", "functions: []\nidentify: ./x.yaml"),
            valid.replace("  ./schemas/a.yaml: x\n", ""),
            "not: [a manifest".to_string(),
        ] {
            assert!(PreviousManifest::parse(invalid.as_bytes()).is_none(), "{invalid}");
        }
    }
}
