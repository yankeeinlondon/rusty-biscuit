//! A read-only examination of the published registration, the retired
//! layout, and the files a repair would lock or edit, for `claudine doctor`
//! and for the repair's own preflight.
//!
//! Nothing here writes, and no link is followed: every entry is classified
//! from its no-follow metadata ([`entry_at`]), walking down from the home.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, PathIdentity, canonicalize_simplified};
use darkmatter::config::init as dm_init;
use darkmatter::config::load::ConfigFile;
use darkmatter::config::schema::ConfigScope as DarkmatterScope;
use darkmatter::config::write::TargetChange;
use darkmatter::yaml_edit::{YamlEditError, YamlEditErrorKind};
use darkmatter::config::registration::{bundle, load_verified, md_owned_names};

use super::legacy::legacy_plan;
use super::published::{blake3_hex, portable_path};
use super::{
    CLAUDINE_DIR, CleanupError, Entry, HOST_ID, LOCK_FILE, Layout, MANIFEST_FILE, MANIFEST_REFERENCE, OUTPUT_DIR, Obstruction,
    PublishError, PublishedFile, REGISTRATIONS_DIR, RegistrationInventory, access_log, entry_at, manifest_path,
    legacy_manifest_path, one_entry, output_root,
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
    /// For a changed manifest judged by [`Installation::judge_manifest`]:
    /// whether Darkmatter loads the whole bundle it lists as Claudine's
    /// registration. `None` when the manifest is not changed or not judged.
    pub(crate) manifest_load: Option<ManifestLoad>,
    /// The bytes inspection read from the manifest and from each changed
    /// member, by published path, kept so [`Installation::judge_manifest`]
    /// judges exactly what was inspected.
    read: BTreeMap<PathBuf, Vec<u8>>,
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
    /// A member whose bytes differ from the digest the installed manifest
    /// lists for it, which Darkmatter's loader refuses. Set only by
    /// [`Installation::judge_manifest`]; `false` when they match, the
    /// manifest does not list it, or the manifest is unreadable or not
    /// admitted (that is the manifest's own finding).
    pub(crate) digest_mismatch: bool,
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

/// Whether Darkmatter loads a changed manifest's bundle as Claudine's
/// registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ManifestLoad {
    Loads,
    /// `at` is the file Darkmatter's first refusal is located in, when that
    /// is not the manifest.
    Refused { at: Option<PathBuf> },
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
    /// Judges the installed manifest as Darkmatter's loader would, with its
    /// members located by the loader's own resolution
    /// ([`bundle::admit_installed_manifest`]): fills
    /// [`ExpectedFile::digest_mismatch`] for each changed member, and, for a
    /// changed manifest, [`Installation::manifest_load`] by preparing the
    /// bundle it lists ([`manifest_load`]). Repair never calls this: it
    /// requires the inventory's exact bytes, which every process already
    /// prepares.
    pub(crate) fn judge_manifest(&mut self, home: &Path, context: &FileResolutionContext) {
        let layout = Layout::new(home);
        let path = layout.destination(MANIFEST_FILE);
        let Some(bytes) = self.read.get(&path) else { return };
        let admitted = bundle::admit_installed_manifest(&path, bytes, context);
        if let Ok(admitted) = &admitted {
            mark_digest_mismatches(&layout, admitted, &self.read, &mut self.files);
        }
        if self.files.iter().any(|file| file.manifest && file.state == ExpectedState::Changed) {
            self.manifest_load = Some(manifest_load(&layout, &path, bytes, admitted, context));
        }
    }

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
///
/// Files are only compared by bytes here; digest agreement and whether
/// Darkmatter loads a changed manifest are [`Installation::judge_manifest`]'s
/// questions.
pub(crate) fn inspect_installation(home: &Path, config: Option<&Path>, inventory: &RegistrationInventory) -> Installation {
    let layout = Layout::new(home);
    let mut found = Installation {
        output_root: output_root(home),
        root_present: false,
        files: Vec::new(),
        manifest_load: None,
        read: BTreeMap::new(),
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
                    Ok(bytes) => {
                        let state = if bytes == file.bytes() { ExpectedState::Current } else { ExpectedState::Changed };
                        if state == ExpectedState::Changed || file.path() == MANIFEST_FILE {
                            found.read.insert(path.clone(), bytes);
                        }
                        state
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
        found.files.push(ExpectedFile { path, manifest: file.path() == MANIFEST_FILE, state, digest_mismatch: false });
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
        found.target_blockers.extend(config_target_blockers(home, config));
    }
    found
}

/// The obstructions at a configuration file a repair asks Darkmatter's
/// writer to edit (the user `.darkmatter.yaml`, or either file of a
/// `permissions` handoff) and the lock sidecar the writer takes beside it;
/// checked again just before each edit.
///
/// `root` is the trusted scope root (the home, or the repository root) and
/// is not itself examined; every component below it on the way to `config`
/// and its sidecar is, as the publisher walks its tree, so a linked folder
/// on the way (a linked `.claudine`) blocks the edit as a linked file does.
/// A path that is not spelled below `root` is refused as uninspectable.
pub(crate) fn config_target_blockers(root: &Path, config: &Path) -> Vec<Blocker> {
    let mut sidecar = config.as_os_str().to_os_string();
    sidecar.push(darkmatter::config::write::LOCK_SUFFIX);
    let layout = Layout::new(root);
    let uninspectable = |kind| BlockerProblem::Unreadable { operation: "inspect", kind };
    let mut blockers = Vec::new();
    for path in [config.to_path_buf(), PathBuf::from(sidecar)] {
        let names: Option<Vec<&str>> = path.strip_prefix(root).ok().and_then(|relative| {
            relative
                .components()
                .map(|component| match component {
                    std::path::Component::Normal(name) => name.to_str(),
                    _ => None,
                })
                .collect()
        });
        let Some(names) = names.filter(|names| !names.is_empty()) else {
            blockers.push(Blocker { path, problem: uninspectable(io::ErrorKind::InvalidInput) });
            continue;
        };
        let blocker = match layout.route(&names) {
            // A missing configuration file is the precondition's to report.
            Ok(Entry::Absent | Entry::File) => continue,
            Ok(Entry::Directory | Entry::Other) => Blocker { path, problem: BlockerProblem::NotAFile },
            Ok(Entry::Obstruction(obstruction)) => Blocker { path, problem: BlockerProblem::Link(obstruction) },
            Err(PublishError::Obstructed { path, obstruction }) => Blocker { path, problem: BlockerProblem::Link(obstruction) },
            Err(PublishError::NotADirectory { path }) => Blocker { path, problem: BlockerProblem::NotADirectory },
            Err(PublishError::Io { path, source, .. }) => Blocker { path, problem: uninspectable(source.kind()) },
            Err(_) => Blocker { path, problem: uninspectable(io::ErrorKind::Other) },
        };
        blockers.push(blocker);
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
    let join = |name: &str| if relative.is_empty() { name.to_string() } else { format!("{relative}/{name}") };
    // The inventory names of this folder it does not list as spelled: on a
    // case-insensitive volume one may still reach a listed entry.
    let unlisted: Vec<&str> = files
        .iter()
        .chain(folders.iter())
        .filter_map(|path| match path.rsplit_once('/') {
            Some((parent, name)) => (parent == relative).then_some(name),
            None => (relative.is_empty() && !path.is_empty()).then_some(path.as_str()),
        })
        .filter(|name| !children.iter().any(|(listed, _)| listed == name))
        .collect();
    for (name, path) in children {
        let mut published = join(&name);
        let entry = entry_at(&path);
        if matches!(entry, Ok(Entry::File | Entry::Directory)) && !files.contains(&published) && !folders.contains(&published) {
            match expected_alias(directory, &path, &unlisted) {
                Ok(Some(expected)) => published = join(expected),
                Ok(None) => {}
                Err(error) => {
                    blockers.push(Blocker { path, problem: BlockerProblem::Unreadable { operation: "inspect", kind: kind_of(&error) } });
                    continue;
                }
            }
        }
        match entry {
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

/// The inventory name in `unlisted` (names of `directory` it does not list
/// as spelled) that reaches the listed entry `found`, as a case-only alias
/// does on a case-insensitive volume ([`one_entry`]).
fn expected_alias<'a>(directory: &Path, found: &Path, unlisted: &[&'a str]) -> Result<Option<&'a str>, PublishError> {
    for name in unlisted {
        if one_entry(found, &directory.join(name))? {
            return Ok(Some(name));
        }
    }
    Ok(None)
}

/// Sets [`ExpectedFile::digest_mismatch`] on each changed member from the
/// digest `admitted` (the installed manifest, its members resolved as the
/// loader resolves them) lists for it; `read` holds the bytes inspection
/// read, by published path.
fn mark_digest_mismatches(
    layout: &Layout,
    admitted: &bundle::AdmittedManifest,
    read: &BTreeMap<PathBuf, Vec<u8>>,
    files: &mut [ExpectedFile],
) {
    for file in files.iter_mut() {
        if file.manifest || file.state != ExpectedState::Changed {
            continue;
        }
        // The manifest may list the member under a spelling the volume
        // resolves to the same entry, which is the one Darkmatter's loader
        // reads and checks.
        let listed = admitted.members().find(|member| layout.destination(&member.path) == file.path).or_else(|| {
            admitted.members().find(|member| {
                portable_path(&member.path).is_ok() && one_entry(&layout.destination(&member.path), &file.path).unwrap_or(false)
            })
        });
        if let (Some(member), Some(bytes)) = (listed, read.get(&file.path)) {
            file.digest_mismatch = blake3_hex(bytes) != member.digest;
        }
    }
}

/// Whether Darkmatter loads the bundle the changed manifest `bytes` at
/// `path` lists as Claudine's registration.
///
/// `admitted` is the manifest's admission with its members located by the
/// loader's resolution, so a spelling the loader confines to the bundle
/// (re-entering the output root through `..`) names the same member here.
/// The manifest must register the `claudine` host and every member be a
/// regular file of the output root, reached without following a link; the
/// members are then read once and handed, with the manifest, to
/// Darkmatter's verifier over already-read bytes and to the same
/// preparation its `register` loader runs (member kinds, payloads, declared
/// types, the catalog, and the lifecycle and function artifacts' agreement
/// with it). Byte integrity alone never counts as loading.
fn manifest_load(
    layout: &Layout,
    path: &Path,
    bytes: &[u8],
    admitted: Result<bundle::AdmittedManifest, Box<bundle::SnapshotRefusal>>,
    context: &FileResolutionContext,
) -> ManifestLoad {
    let refused = |at: &Path| ManifestLoad::Refused { at: (at != path).then(|| at.to_path_buf()) };
    let admitted = match admitted {
        Ok(admitted) => admitted,
        Err(refusal) => return refused(&refusal.file),
    };
    if admitted.host() != HOST_ID {
        return refused(path);
    }
    let mut members: BTreeMap<String, Arc<[u8]>> = BTreeMap::new();
    for member in admitted.members() {
        let destination = layout.destination(&member.path);
        if member.path == MANIFEST_FILE || portable_path(&member.path).is_err() {
            return refused(path);
        }
        if layout.route(&Layout::output_names(&member.path)).ok() != Some(Entry::File) {
            return refused(&destination);
        }
        access_log::note("read", &destination);
        match fs::read(&destination) {
            Ok(found) => members.insert(member.path.clone(), Arc::from(found)),
            Err(_) => return refused(&destination),
        };
    }
    let snapshot = match bundle::verify_read(path, bytes, context, &members) {
        Ok(snapshot) => snapshot,
        Err(refusal) => return refused(&refusal.file),
    };
    match load_verified(snapshot, context, md_owned_names()) {
        Ok(_) => ManifestLoad::Loads,
        Err(refusals) => refused(refusals.first().map_or(path, |refusal| refusal.file.as_path())),
    }
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
    /// The entry edit the repair needs reaches a part of the file that
    /// Darkmatter's format-preserving editor cannot change in place.
    Layout(EditLayout),
}

/// The part of a file whose authored layout stops Darkmatter's
/// format-preserving editor from making a planned edit in place: a flow
/// collection (`{…}`, `[…]`) the edit must walk into, or an anchor, alias,
/// or tag on the way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditLayout {
    /// The root mapping, written inline.
    Root,
    /// The named top-level setting.
    Setting(&'static str),
}

/// The layout obstacle the editor meets applying `change`'s edits to the
/// bytes it expects ([`TargetChange::edit_refusal`]), attributed to the
/// root or to `setting`, the only top-level setting the change edits.
/// `None` when the edits apply, or when the refusal is not about layout (a
/// parse or path problem, which the file's own validation reports).
pub(crate) fn edit_layout(change: &TargetChange, setting: &'static str) -> Option<EditLayout> {
    layout_of(&change.edit_refusal()?, setting)
}

fn layout_of(refusal: &YamlEditError, setting: &'static str) -> Option<EditLayout> {
    match refusal.kind {
        YamlEditErrorKind::Parse | YamlEditErrorKind::NotAMapping | YamlEditErrorKind::InvalidPath => None,
        // Only the flow-root refusal names no property.
        _ if refusal.path.is_none() => Some(EditLayout::Root),
        _ => Some(EditLayout::Setting(setting)),
    }
}

/// Whether the edit the repair would make to the user `register` list
/// ([`dm_init::replace_manifest_refusal`]) meets a layout obstacle, judged
/// before anything is published. A file that needs no edit (its entry is
/// already in place) has none, whatever its layout. A file that cannot be
/// observed or planned is left to [`entry_precondition`] and the writer.
pub(crate) fn entry_layout(context: &FileResolutionContext) -> Result<(), EntryPrecondition> {
    let Some(home) = context.home_dir() else { return Ok(()) };
    let Ok(observation) = dm_init::observe(context, DarkmatterScope::User) else { return Ok(()) };
    if observation.bytes.is_none() {
        return Ok(());
    }
    let refusal = dm_init::replace_manifest_refusal(&observation, context, MANIFEST_REFERENCE, &legacy_manifest_path(home));
    match refusal.ok().flatten().and_then(|refusal| layout_of(&refusal, "register")) {
        Some(layout) => Err(EntryPrecondition::Layout(layout)),
        None => Ok(()),
    }
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
