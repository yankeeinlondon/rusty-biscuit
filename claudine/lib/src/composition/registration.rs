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
//!   the lock, and for each destination again just before it is written or
//!   removed.
//! - **Serialized.** Cooperating publishers (threads or processes) take one
//!   OS file lock, [`lock_path`], waiting at most [`LOCK_DEADLINE`].
//! - **Members, then the manifest.** Under the lock, each member whose bytes
//!   differ from the inventory's is replaced through a sibling temporary file
//!   ([`atomic_write_registration`], with the Windows replacement retry);
//!   entries the previous manifest listed and the inventory no longer has
//!   are removed, only when Darkmatter's format 2 admission
//!   (`bundle::admit_manifest`) accepts that manifest as Claudine's (any
//!   refusal removes nothing; old member bytes need not match), except one
//!   that reaches a current member's directory entry (an old spelling that
//!   differs only in case, on a case-insensitive volume); regular
//!   files named with [`TEMP_PREFIX`] in a publisher directory (leftovers
//!   of an interrupted replacement) are removed. The manifest is replaced last, and only when its bytes differ.
//! - **Unknown files are kept.** Anything else in the output root is not
//!   a publish's to remove; doctor reports it, and only doctor's explicit
//!   repair (`repair_registration`) removes it.
//! - **One lock across steps.** `Publisher` holds the lock for a caller
//!   that publishes and then does more under it (doctor's repair: verify,
//!   edit the user `register` entry, verify, remove the retired layout).
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

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::time::Duration;

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
    /// Folders of retired paths that no member needs, removed once they
    /// were empty, by published path (`schemas/old`), deepest first.
    pub removed_folders: Vec<String>,
}

impl PublishChanges {
    /// Whether the manifest was replaced.
    #[must_use]
    pub fn manifest_written(&self) -> bool {
        self.written.iter().any(|path| path == MANIFEST_FILE)
    }

    /// Each change under `home`, in the order [`PublishChanges`]'s fields
    /// list them, as `wrote <path>` or `removed <path>`.
    #[must_use]
    pub fn describe(&self, home: &Path) -> Vec<String> {
        let layout = Layout::new(home);
        let removed = |path: &Path| format!("removed {}", path.display());
        let mut lines: Vec<String> = self.removed_temps.iter().map(|path| removed(path)).collect();
        lines.extend(self.written.iter().map(|path| format!("wrote {}", layout.destination(path).display())));
        lines.extend(self.retired.iter().chain(&self.removed_folders).map(|path| removed(&layout.destination(path))));
        lines
    }

    /// Whether nothing changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.written.is_empty()
            && self.retired.is_empty()
            && self.removed_temps.is_empty()
            && self.removed_folders.is_empty()
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

/// Why [`publish`] stopped. Members may have been replaced, and retired
/// entries or leftover temporary files removed, before the failure
/// ([`publish_recording`] keeps the list), but the manifest was not: the
/// previous manifest is intact, and the next publish converges.
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
    #[error("`{}` is {obstruction}; the publisher never follows or replaces one, so the publish stopped before it. Remove it and publish again", .path.display())]
    Obstructed {
        #[allow(missing_docs)]
        path: PathBuf,
        #[allow(missing_docs)]
        obstruction: Obstruction,
    },
    /// Something other than a directory stands where a directory belongs.
    #[error("`{}` is not a directory, so the publish stopped before it", .path.display())]
    NotADirectory {
        #[allow(missing_docs)]
        path: PathBuf,
    },
    /// Something other than a regular file stands where a file is written.
    #[error("`{}` is not a regular file, so the publish stopped before it", .path.display())]
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
    publish_recording(home, inventory, &mut PublishChanges::default())
}

/// [`publish`], recording each change in `changes` as it is made, so a
/// caller still knows what changed when the publish fails partway.
///
/// ## Errors
///
/// As [`publish`].
pub fn publish_recording(
    home: &Path,
    inventory: &RegistrationInventory,
    changes: &mut PublishChanges,
) -> Result<PublishOutcome, PublishError> {
    Publisher::acquire(home, inventory)?.publish_recording(inventory, changes)
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
    Publisher::acquire_with(home, inventory, hooks.lock_deadline)?.publish_observed(
        inventory,
        hooks.observer,
        &mut PublishChanges::default(),
    )
}

/// The publisher lock, held, with the layout it guards: every operation a
/// caller runs through one `Publisher` is serialized with every other
/// publisher until it is dropped.
///
/// [`publish`] is one acquire and one [`Self::publish`]. A caller that must
/// keep other publishers out across several steps (doctor's repair: publish,
/// verify, edit the `register` entry, verify, remove the retired layout)
/// keeps the value for all of them. Setup does the same
/// (`config::setup::setup_claudine`): `acquire`, `publish`, its permissions
/// handoff and `register` edit, then [`Self::finalize_legacy`] only when that
/// edit succeeded.
pub(crate) struct Publisher {
    layout: Layout,
    _lock: PublisherLock,
}

impl Publisher {
    /// Takes the publisher lock under `home`, waiting at most
    /// [`LOCK_DEADLINE`], after the preflight of `inventory`'s tree found
    /// nothing in the way; the lock's folder is the only thing created.
    pub(crate) fn acquire(home: &Path, inventory: &RegistrationInventory) -> Result<Self, PublishError> {
        Self::acquire_with(home, inventory, LOCK_DEADLINE)
    }

    fn acquire_with(
        home: &Path,
        inventory: &RegistrationInventory,
        deadline: Duration,
    ) -> Result<Self, PublishError> {
        if let Some((path, reason)) = inventory.invalid_path() {
            return Err(PublishError::InvalidPath { path: path.to_string(), reason });
        }
        let layout = Layout::new(home);
        // Before anything is created, the lock file and its folder included.
        layout.preflight(inventory)?;
        access_log::note("create", &layout.registrations());
        fs::create_dir_all(layout.registrations()).map_err(io_error("create", &layout.registrations()))?;
        let lock = acquire_lock(&lock_path(home), deadline)?;
        Ok(Self { layout, _lock: lock })
    }

    /// The home this publisher writes under.
    fn home(&self) -> &Path {
        &self.layout.home
    }

    /// Publishes `inventory` under the held lock, as [`publish_recording`]
    /// describes: `changes` lists what changed even when the publish fails.
    pub(crate) fn publish_recording(
        &self,
        inventory: &RegistrationInventory,
        changes: &mut PublishChanges,
    ) -> Result<PublishOutcome, PublishError> {
        self.publish_observed(inventory, &mut |_: &PublishStep| ControlFlow::Continue(()), changes)
    }

    fn publish_observed(
        &self,
        inventory: &RegistrationInventory,
        observer: &mut dyn FnMut(&PublishStep) -> ControlFlow<()>,
        changes: &mut PublishChanges,
    ) -> Result<PublishOutcome, PublishError> {
        let manifest = self.publish_into(inventory, observer, changes)?;
        let changes = changes.clone();
        if changes.is_empty() {
            tracing::debug!(manifest = %manifest.display(), "the host registration is current");
            Ok(PublishOutcome::Current { manifest })
        } else {
            tracing::debug!(manifest = %manifest.display(), ?changes, "published the host registration");
            Ok(PublishOutcome::Published { manifest, changes })
        }
    }

    /// The publish itself, recording each change in `changes` as it is
    /// made, so a caller still knows what changed when it fails partway.
    /// Returns the manifest path.
    fn publish_into(
        &self,
        inventory: &RegistrationInventory,
        observer: &mut dyn FnMut(&PublishStep) -> ControlFlow<()>,
        changes: &mut PublishChanges,
    ) -> Result<PathBuf, PublishError> {
        let layout = &self.layout;
        // Another publisher may have changed the tree while this one waited.
        let plan = layout.preflight(inventory)?;
        let mut stop = |step: PublishStep| match observer(&step) {
            ControlFlow::Continue(()) => Ok(()),
            ControlFlow::Break(()) => Err(PublishError::Interrupted { step: format!("{step:?}") }),
        };
        stop(PublishStep::Locked)?;

        for directory in &plan.directories {
            remove_temps(directory, &mut changes.removed_temps)?;
        }
        for file in inventory.members() {
            let destination = layout.destination(file.path());
            if holds(&destination, file.bytes())? {
                continue;
            }
            // The preflight is repeated at each mutation: a link created
            // since is refused, never written through.
            layout.writable(&Layout::output_names(file.path()))?;
            access_log::note("write", &destination);
            atomic_write_registration(&destination, file.bytes()).map_err(io_error("write", &destination))?;
            changes.written.push(file.path().to_string());
            stop(PublishStep::MemberWritten(file.path().to_string()))?;
        }
        for path in &plan.retired {
            let destination = layout.destination(path);
            if layout.route(&Layout::output_names(path))? != Entry::File {
                continue;
            }
            access_log::note("remove", &destination);
            match remove_entry(&destination, false) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(io_error("remove", &destination)(error)),
            }
            changes.retired.push(path.clone());
            remove_empty_parents(layout, path, inventory, &mut changes.removed_folders);
            stop(PublishStep::Retired(path.clone()))?;
        }
        stop(PublishStep::BeforeManifest)?;
        let manifest = layout.destination(MANIFEST_FILE);
        if !holds(&manifest, inventory.manifest().bytes())? {
            layout.writable(&Layout::output_names(MANIFEST_FILE))?;
            access_log::note("write", &manifest);
            atomic_write_registration(&manifest, inventory.manifest().bytes()).map_err(io_error("write", &manifest))?;
            changes.written.push(MANIFEST_FILE.to_string());
        }
        Ok(manifest)
    }

    /// Removes the retired format 1 registration
    /// ([`remove_legacy_registration`]) while the lock is held. Call it only
    /// once the user `register` entry names the new manifest.
    pub(crate) fn finalize_legacy(&self) -> Result<LegacyCleanup, CleanupFailure> {
        remove_legacy_registration(&self.layout.home)
    }
}

/// Whether the regular file `path` holds exactly `bytes`. The preflight
/// already refused anything at `path` but a regular file or nothing.
fn holds(path: &Path, bytes: &[u8]) -> Result<bool, PublishError> {
    access_log::note("read", path);
    match fs::read(path) {
        Ok(found) => Ok(found == bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error("read", path)(error)),
    }
}

// ── preflight ───────────────────────────────────────────────────────────────

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
            if self.route(&Self::output_names(&path))? != Entry::File {
                continue;
            }
            // On a case-insensitive volume an old spelling can reach a
            // current member's entry; removing it would remove the member.
            let destination = self.destination(&path);
            let mut current = false;
            for member in &known {
                if self.route(&Self::output_names(member))? == Entry::File
                    && one_entry(&destination, &self.destination(member))?
                {
                    current = true;
                    break;
                }
            }
            if !current {
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

    /// The published paths the installed manifest lists, when Darkmatter's
    /// format 2 admission accepts it as a Claudine manifest and every member
    /// is a path this publisher writes; nothing otherwise. An inadmissible
    /// manifest authorizes no removal at all: its other entries are never
    /// trusted on their own. Member bytes are not checked against the
    /// digests, so a publish interrupted after replacing members still
    /// retires what the standing manifest listed.
    fn previous_members(&self) -> Result<Vec<String>, PublishError> {
        let path = self.destination(MANIFEST_FILE);
        if self.route(&Self::output_names(MANIFEST_FILE))? != Entry::File {
            return Ok(Vec::new());
        }
        access_log::note("read", &path);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(io_error("read", &path)(error)),
        };
        Ok(previous_member_paths(&path, &bytes).unwrap_or_default())
    }
}

/// The member paths of the installed manifest `bytes` at `path`, when it
/// authorizes retirement: Darkmatter admits it, it registers [`HOST_ID`],
/// and every member is a path this publisher writes (never the manifest
/// itself).
fn previous_member_paths(path: &Path, bytes: &[u8]) -> Option<Vec<String>> {
    let admitted = match darkmatter::config::registration::bundle::admit_manifest(path, bytes) {
        Ok(admitted) => admitted,
        Err(refusal) => {
            tracing::debug!(manifest = %path.display(), ?refusal, "the installed manifest is refused; nothing is retired");
            return None;
        }
    };
    let publishable = |member: &str| member != MANIFEST_FILE && portable_path(member).is_ok();
    if admitted.host() != HOST_ID || !admitted.members().all(|member| publishable(&member.path)) {
        tracing::debug!(manifest = %path.display(), host = admitted.host(), "the installed manifest is not one Claudine publishes; nothing is retired");
        return None;
    }
    Some(admitted.members().map(|member| member.path.clone()).collect())
}

mod access_log;
mod entries;
mod inspect;
mod legacy;
mod lock;
mod removal;
mod repair;

use entries::{Entry, entry_at, one_entry};
#[cfg(test)]
use entries::obstruction_of;
use lock::{PublisherLock, acquire_lock};
use removal::{remove_empty_parents, remove_entry, remove_temps};
#[cfg(test)]
pub(crate) use removal::fail_removals;

pub(crate) use inspect::{
    Blocker, BlockerProblem, EditLayout, EntryPrecondition, ExpectedState, Installation, ManifestLoad,
    config_target_blockers, edit_layout, entry_layout, entry_precondition, entry_registered, inspect_installation,
};
pub use legacy::{CleanupError, CleanupFailure, LegacyCleanup, remove_legacy_registration};
pub(crate) use repair::{EditFailure, RepairFailure, RepairOperation, repair_registration};

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
        let previous: BTreeSet<String> = previous_member_paths(Path::new("/r/manifest.yaml"), inventory.manifest().bytes())
            .expect("its own manifest authorizes retirement")
            .into_iter()
            .collect();
        assert_eq!(previous, inventory.members().map(|file| file.path().to_string()).collect());
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
}
