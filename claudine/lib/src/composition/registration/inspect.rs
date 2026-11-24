//! A read-only examination of the published registration, the retired
//! layout, and the files a repair would lock or edit, for `claudine doctor`
//! and for the repair's own preflight.
//!
//! Nothing here writes, and no link is followed: every entry is classified
//! from its no-follow metadata ([`entry_at`]), walking down from the home.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, PathIdentity, canonicalize_simplified};
use darkmatter::config::load::ConfigFile;

use super::legacy::legacy_plan;
use super::published::{blake3_hex, portable_path};
use super::{
    CLAUDINE_DIR, CleanupError, Entry, HOST_ID, LOCK_FILE, Layout, MANIFEST_FILE, OUTPUT_DIR, Obstruction,
    PublishError, PublishedFile, REGISTRATIONS_DIR, RegistrationInventory, access_log, entry_at, manifest_path,
    output_root,
};

/// What was found under one home.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Installation {
    /// The output root, `~/.claudine/registrations/darkmatter`.
    pub(crate) output_root: PathBuf,
    /// Whether the output root is a directory.
    pub(crate) root_present: bool,
    /// Every inventory file, the manifest included, in inventory order.
    pub(crate) files: Vec<ExpectedFile>,
    /// For a changed manifest: whether Darkmatter's format 2 admission and
    /// digest check accept it as Claudine's. `None` otherwise.
    pub(crate) manifest_load: Option<ManifestLoad>,
    /// Entries of the output root the inventory does not have, children
    /// before their folder: what a repair removes.
    pub(crate) extras: Vec<Extra>,
    /// The retired manifest and revision tree, where each is present and
    /// removable.
    pub(crate) legacy: Vec<PathBuf>,
    /// What stops the publisher or the cleanup (a link, a special file, an
    /// unreadable folder) inside the tree, the removal set, or the retired
    /// layout. Never repaired automatically.
    pub(crate) blockers: Vec<Blocker>,
    /// What stops only a repair: a link or a misplaced entry at the
    /// publisher lock, the user `.darkmatter.yaml`, or its lock sidecar.
    pub(crate) target_blockers: Vec<Blocker>,
}

/// One inventory file and what its published path holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExpectedFile {
    pub(crate) path: PathBuf,
    pub(crate) manifest: bool,
    pub(crate) state: ExpectedState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpectedState {
    /// The inventory's exact bytes.
    Current,
    /// A regular file with other bytes.
    Changed,
    Missing,
    /// A folder or a special file.
    NotAFile,
    Unreadable(io::ErrorKind),
    /// Behind a [`Blocker`], so not examined.
    Unreachable,
}

/// Whether a changed manifest still passes Darkmatter's format 2 checks as
/// Claudine's registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManifestLoad {
    Loads,
    Refused,
}

/// An entry of the output root that is not the inventory's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Extra {
    pub(crate) path: PathBuf,
    pub(crate) directory: bool,
}

/// Something a repair must not traverse, replace, or remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Blocker {
    pub(crate) path: PathBuf,
    pub(crate) problem: BlockerProblem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockerProblem {
    Link(Obstruction),
    /// Something other than a folder where a folder belongs.
    NotADirectory,
    /// Something other than a regular file where a file is written.
    NotAFile,
    /// A socket, device, or anything else neither a file nor a folder.
    Special,
    /// `operation` (`inspect`, `list`, `read`) failed with this kind.
    Unreadable { operation: &'static str, kind: io::ErrorKind },
}

impl Installation {
    /// Every reason a repair would refuse to start, in report order.
    pub(crate) fn manual_blockers(&self) -> Vec<Blocker> {
        let files = self.files.iter().filter_map(|file| {
            let problem = match file.state {
                ExpectedState::NotAFile => BlockerProblem::NotAFile,
                ExpectedState::Unreadable(kind) => BlockerProblem::Unreadable { operation: "read", kind },
                _ => return None,
            };
            Some(Blocker { path: file.path.clone(), problem })
        });
        self.blockers.iter().cloned().chain(files).chain(self.target_blockers.iter().cloned()).collect()
    }

    /// Whether every inventory file holds its bytes and nothing else is in
    /// the output root.
    pub(crate) fn is_current(&self) -> bool {
        self.blockers.is_empty()
            && self.extras.is_empty()
            && self.files.iter().all(|file| file.state == ExpectedState::Current)
    }

    fn block(&mut self, blocker: Blocker) {
        if !self.blockers.contains(&blocker) {
            self.blockers.push(blocker);
        }
    }
}

/// Examines `home`'s published registration against `inventory`, the
/// retired layout, and the files a repair would lock or edit: the publisher
/// lock and, when given, `config` (the user `.darkmatter.yaml`) and its
/// writer's lock sidecar.
pub(crate) fn inspect_installation(
    home: &Path,
    config: Option<&Path>,
    inventory: &RegistrationInventory,
) -> Installation {
    let layout = Layout::new(home);
    let mut found = Installation {
        output_root: output_root(home),
        root_present: false,
        files: Vec::new(),
        manifest_load: None,
        extras: Vec::new(),
        legacy: Vec::new(),
        blockers: Vec::new(),
        target_blockers: Vec::new(),
    };

    for file in inventory.files() {
        let path = layout.destination(file.path());
        let state = match probe(&layout, &Layout::output_names(file.path())) {
            Err(blocker) => {
                found.block(blocker);
                ExpectedState::Unreachable
            }
            Ok(Entry::Absent) => ExpectedState::Missing,
            Ok(Entry::File) => {
                access_log::note("read", &path);
                match fs::read(&path) {
                    Ok(bytes) if bytes == file.bytes() => ExpectedState::Current,
                    Ok(bytes) => {
                        if file.path() == MANIFEST_FILE {
                            found.manifest_load = Some(manifest_load(&layout, &path, &bytes));
                        }
                        ExpectedState::Changed
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => ExpectedState::Missing,
                    Err(error) => ExpectedState::Unreadable(error.kind()),
                }
            }
            Ok(Entry::Obstruction(obstruction)) => {
                found.block(Blocker { path: path.clone(), problem: BlockerProblem::Link(obstruction) });
                ExpectedState::Unreachable
            }
            Ok(Entry::Directory | Entry::Other) => ExpectedState::NotAFile,
        };
        found.files.push(ExpectedFile { path, manifest: file.path() == MANIFEST_FILE, state });
    }

    match probe(&layout, &[CLAUDINE_DIR, REGISTRATIONS_DIR, OUTPUT_DIR]) {
        Ok(Entry::Directory) => {
            found.root_present = true;
            let (files, folders) = expected_names(inventory);
            let mut extras = Vec::new();
            let mut blockers = Vec::new();
            scan(&found.output_root, "", &files, &folders, &mut extras, &mut blockers);
            found.extras = extras;
            for blocker in blockers {
                found.block(blocker);
            }
        }
        // The walk to an expected file already recorded a link or a file in
        // the way; anything else here is absent.
        Ok(Entry::Obstruction(obstruction)) => {
            found.block(Blocker { path: found.output_root.clone(), problem: BlockerProblem::Link(obstruction) });
        }
        Ok(Entry::File | Entry::Other) => {
            found.block(Blocker { path: found.output_root.clone(), problem: BlockerProblem::NotADirectory });
        }
        Ok(Entry::Absent) => {}
        Err(blocker) => found.block(blocker),
    }

    match legacy_plan(home) {
        Ok(plan) => {
            found.legacy.extend(plan.manifest);
            if !plan.tree.is_empty() {
                found.legacy.push(super::legacy_revisions_dir(home));
            }
        }
        Err(error) => found.block(cleanup_blocker(error)),
    }

    let lock = super::lock_path(home);
    match probe(&layout, &[CLAUDINE_DIR, REGISTRATIONS_DIR, LOCK_FILE]) {
        Ok(Entry::Absent | Entry::File) | Err(_) => {}
        Ok(Entry::Obstruction(obstruction)) => {
            found.target_blockers.push(Blocker { path: lock, problem: BlockerProblem::Link(obstruction) });
        }
        Ok(Entry::Directory | Entry::Other) => {
            found.target_blockers.push(Blocker { path: lock, problem: BlockerProblem::NotAFile });
        }
    }
    if let Some(config) = config {
        found.target_blockers.extend(config_target_blockers(config));
    }
    found
}

/// The obstructions at a configuration file a repair asks Darkmatter's
/// writer to edit (the user `.darkmatter.yaml`, or either file of a
/// `permissions` handoff) and the lock sidecar the writer takes beside it;
/// checked again just before each edit.
pub(crate) fn config_target_blockers(config: &Path) -> Vec<Blocker> {
    let mut sidecar = config.as_os_str().to_os_string();
    sidecar.push(darkmatter::config::write::LOCK_SUFFIX);
    let mut blockers = Vec::new();
    for path in [config.to_path_buf(), PathBuf::from(sidecar)] {
        let problem = match entry_at(&path) {
            Ok(Entry::Obstruction(obstruction)) => BlockerProblem::Link(obstruction),
            Ok(Entry::Directory | Entry::Other) => BlockerProblem::NotAFile,
            // A missing configuration file is the precondition's to report.
            Ok(Entry::Absent | Entry::File) => continue,
            Err(error) => BlockerProblem::Unreadable { operation: "inspect", kind: kind_of(&error) },
        };
        blockers.push(Blocker { path, problem });
    }
    blockers
}

/// The entry at the end of `names` below the home, as [`Layout::route`]
/// finds it, with the folder that stops the walk as a [`Blocker`].
fn probe(layout: &Layout, names: &[&str]) -> Result<Entry, Blocker> {
    match layout.route(names) {
        Ok(entry) => Ok(entry),
        Err(PublishError::Obstructed { path, obstruction }) => {
            // A link as the last name is reported by the caller with what it
            // stands for; one on the way is a folder that cannot be crossed.
            let last = names.iter().fold(layout.home.clone(), |path, name| path.join(name));
            if path == last { Ok(Entry::Obstruction(obstruction)) } else { Err(Blocker { path, problem: BlockerProblem::Link(obstruction) }) }
        }
        Err(PublishError::NotADirectory { path }) => Err(Blocker { path, problem: BlockerProblem::NotADirectory }),
        Err(PublishError::Io { path, source, .. }) => {
            Err(Blocker { path, problem: BlockerProblem::Unreadable { operation: "inspect", kind: source.kind() } })
        }
        Err(_) => Err(Blocker {
            path: layout.home.clone(),
            problem: BlockerProblem::Unreadable { operation: "inspect", kind: io::ErrorKind::Other },
        }),
    }
}

fn kind_of(error: &PublishError) -> io::ErrorKind {
    match error {
        PublishError::Io { source, .. } => source.kind(),
        _ => io::ErrorKind::Other,
    }
}

fn cleanup_blocker(error: CleanupError) -> Blocker {
    match error {
        CleanupError::Obstructed { path, obstruction } => Blocker { path, problem: BlockerProblem::Link(obstruction) },
        CleanupError::Unexpected { path } => Blocker { path, problem: BlockerProblem::Special },
        CleanupError::Io { action, path, source } => {
            let operation = if action == "read" { "list" } else { "inspect" };
            Blocker { path, problem: BlockerProblem::Unreadable { operation, kind: source.kind() } }
        }
    }
}

/// The inventory's file paths and every folder that holds one (`""` is the
/// output root).
fn expected_names(inventory: &RegistrationInventory) -> (BTreeSet<String>, BTreeSet<String>) {
    let files: BTreeSet<String> = inventory.files().iter().map(|file| file.path().to_string()).collect();
    let mut folders = BTreeSet::from([String::new()]);
    for path in inventory.files().iter().map(PublishedFile::path) {
        let mut folder = path;
        while let Some((parent, _)) = folder.rsplit_once('/') {
            folders.insert(parent.to_string());
            folder = parent;
        }
    }
    (files, folders)
}

/// Lists the directory `directory` (published path `relative`), adding
/// every entry outside the inventory to `extras`, children before their
/// folder, and every link, special file, or unreadable folder to `blockers`.
/// A folder removed while it is listed is skipped.
fn scan(
    directory: &Path,
    relative: &str,
    files: &BTreeSet<String>,
    folders: &BTreeSet<String>,
    extras: &mut Vec<Extra>,
    blockers: &mut Vec<Blocker>,
) {
    access_log::note("list", directory);
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return,
        Err(error) => {
            blockers.push(Blocker {
                path: directory.to_path_buf(),
                problem: BlockerProblem::Unreadable { operation: "list", kind: error.kind() },
            });
            return;
        }
    };
    let mut children: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => children.push((entry.file_name().to_string_lossy().into_owned(), entry.path())),
            Err(error) => {
                blockers.push(Blocker {
                    path: directory.to_path_buf(),
                    problem: BlockerProblem::Unreadable { operation: "list", kind: error.kind() },
                });
                return;
            }
        }
    }
    children.sort();
    for (name, path) in children {
        let published = if relative.is_empty() { name } else { format!("{relative}/{name}") };
        match entry_at(&path) {
            Ok(Entry::Absent) => {}
            Ok(Entry::Obstruction(obstruction)) => {
                blockers.push(Blocker { path, problem: BlockerProblem::Link(obstruction) });
            }
            Ok(Entry::Other) => blockers.push(Blocker { path, problem: BlockerProblem::Special }),
            // An expected path holding the wrong kind of entry is the
            // inventory file's own finding.
            Ok(Entry::File | Entry::Directory) if files.contains(&published) => {}
            Ok(Entry::File) => extras.push(Extra { path, directory: false }),
            Ok(Entry::Directory) => {
                scan(&path, &published, files, folders, extras, blockers);
                if !folders.contains(&published) {
                    extras.push(Extra { path, directory: true });
                }
            }
            Err(error) => {
                blockers.push(Blocker { path, problem: BlockerProblem::Unreadable { operation: "inspect", kind: kind_of(&error) } });
            }
        }
    }
}

/// Whether the manifest `bytes` at `path` passes the checks Darkmatter's
/// format 2 loader applies before it reads any payload: the manifest's
/// admission (envelope, member paths, digest map), the `claudine` host, and
/// every member a regular file of the output root whose bytes match its
/// digest.
fn manifest_load(layout: &Layout, path: &Path, bytes: &[u8]) -> ManifestLoad {
    let Ok(admitted) = darkmatter::config::registration::bundle::admit_manifest(path, bytes) else {
        return ManifestLoad::Refused;
    };
    if admitted.host() != HOST_ID {
        return ManifestLoad::Refused;
    }
    for member in admitted.members() {
        if member.path == MANIFEST_FILE || portable_path(&member.path).is_err() {
            return ManifestLoad::Refused;
        }
        if layout.route(&Layout::output_names(&member.path)).ok() != Some(Entry::File) {
            return ManifestLoad::Refused;
        }
        let destination = layout.destination(&member.path);
        access_log::note("read", &destination);
        match fs::read(&destination) {
            Ok(found) if blake3_hex(&found) == member.digest => {}
            _ => return ManifestLoad::Refused,
        }
    }
    ManifestLoad::Loads
}

/// Why the user `register` entry cannot be edited automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryPrecondition {
    /// There is no launch context to locate the file in.
    NoContext,
    /// `~/.darkmatter.yaml` does not exist.
    Missing,
    /// It cannot be read.
    Unreadable,
    /// It is not a valid Darkmatter configuration.
    Invalid,
    /// Its `register` setting, or an entry of it, is invalid.
    RegisterInvalid,
    /// Its inline `permissions` block is invalid, which the writer refuses.
    PermissionsInvalid,
}

/// Whether Darkmatter's writer can edit `file`'s `register` list: the file
/// exists, its `register` setting has no invalid entry, and it holds no
/// invalid inline `permissions` block (a reference, even a broken one, is
/// kept as written and does not stop the edit).
pub(crate) fn entry_precondition(file: &ConfigFile) -> Result<(), EntryPrecondition> {
    if !file.is_present() {
        return Err(EntryPrecondition::Missing);
    }
    if !file.registration_findings().is_empty() {
        return Err(EntryPrecondition::RegisterInvalid);
    }
    let inline = file.authored().and_then(|document| document.get("permissions")).is_some_and(|value| !value.is_string());
    if inline && file.permissions().is_err() {
        return Err(EntryPrecondition::PermissionsInvalid);
    }
    Ok(())
}

/// Whether a valid `register` entry of `file` names `home`'s published
/// manifest, compared by Darkmatter's manifest identity, so any spelling of
/// the same file counts. `None` when the manifest is not there to compare
/// against.
pub(crate) fn entry_registered(context: &FileResolutionContext, home: &Path, file: &ConfigFile) -> Option<bool> {
    let manifest = manifest_path(home);
    if Layout::new(home).route(&Layout::output_names(MANIFEST_FILE)).ok() != Some(Entry::File) {
        return None;
    }
    let expected = PathIdentity::new(&canonicalize_simplified(&manifest).ok()?);
    Some(file.registrations().iter().any(|entry| {
        darkmatter::config::registration::manifest_identity(&entry.reference, &entry.declared_in, context)
            .is_ok_and(|identity| identity == expected)
    }))
}
