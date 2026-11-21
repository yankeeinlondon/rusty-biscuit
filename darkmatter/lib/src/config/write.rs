//! Locked configuration transactions.
//!
//! A [`ConfigTransaction`] changes one or more YAML configuration files with
//! [`YamlEdit`]s. For every target it resolves the real file (following
//! symlinks), takes the target's sibling lock file, and holds every lock
//! across the re-read, the edit, the caller's validation, the comparison with
//! the bytes the caller expected, and the atomic replacement. Every program
//! that writes these files through this type therefore serializes with every
//! other; nothing is claimed about an editor that does not take the lock.
//!
//! New collections take the target's indentation: an edit copies the text's
//! own layout where it has an example, and otherwise the preference Sniff
//! resolves for the resolved target, asked at most once per target and only
//! when something is rendered without an example.
//!
//! User-facing guide: `darkmatter/docs/topics/config-writes.md`.

use crate::config::migration_receipt::{self, MigrationSource};
use crate::markdown::hash::node_edit::Layout;
use crate::yaml_edit::{YamlEdit, YamlEditError, edit_yaml_paths_in_layout};
use biscuit_file::{Indentation, PathIdentity, YamlError, canonicalize_simplified, to_portable_string};
use std::cell::{OnceCell, RefCell};
use simplified_schema::PropertyPath;
use std::fmt;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

/// Appended to a target's file name to name its lock file, a sibling in the
/// same directory: `config.yaml` is locked through
/// `config.yaml.darkmatter-lock`. Lock files are retained, never removed, so
/// every writer always opens the same file.
pub const LOCK_SUFFIX: &str = ".darkmatter-lock";

/// How long a writer waits for a target's lock before failing with
/// [`ConfigWriteError::LockTimeout`].
pub const LOCK_DEADLINE: Duration = Duration::from_secs(2);

/// The pause between attempts to take a lock another writer holds.
pub const LOCK_BACKOFF: Duration = Duration::from_millis(20);

/// Why a target path could not be resolved to a file to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathProblem {
    /// The path is relative. Callers resolve paths against their own captured
    /// context; the writer never consults the process directory.
    NotAbsolute,
    /// The path does not end in a file name (it ends in `..`, for example).
    NoFileName,
    /// The target does not exist and neither does its directory. The writer
    /// never creates directories.
    MissingParent,
    /// The path is a symlink whose destination does not exist.
    DanglingSymlink,
    /// The path names something other than a regular file.
    NotAFile,
    /// The resolved path has no portable text spelling that names the same
    /// path again, so a receipt could not record it faithfully.
    NotPortable,
}

impl fmt::Display for PathProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotAbsolute => "the path is not absolute",
            Self::NoFileName => "the path does not end in a file name",
            Self::MissingParent => "its directory does not exist",
            Self::DanglingSymlink => "it is a symlink to a missing file",
            Self::NotAFile => "it is not a regular file",
            Self::NotPortable => "it has no faithful portable spelling",
        })
    }
}

/// A validation callback's refusal of a candidate text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationRefusal {
    /// Why the candidate is invalid.
    pub message: String,
    /// The property the refusal concerns, when it concerns one.
    pub path: Option<PropertyPath>,
}

impl ValidationRefusal {
    /// A refusal of the candidate as a whole.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), path: None }
    }

    /// A refusal of the value at `path`.
    pub fn at(path: PropertyPath, message: impl Into<String>) -> Self {
        Self { message: message.into(), path: Some(path) }
    }
}

impl fmt::Display for ValidationRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{path}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

/// What a validation callback is shown for one target: the edited text that
/// would be published, before anything is written.
#[derive(Debug, Clone, Copy)]
pub struct ValidationCandidate<'a> {
    /// The resolved target file.
    pub target: &'a Path,
    /// The text the target held when it was re-read under the lock; `None`
    /// when the transaction creates it.
    pub original: Option<&'a str>,
    /// The text that would replace it.
    pub candidate: &'a str,
}

/// The passive check a [`ConfigTransaction`] runs on every candidate text
/// while it holds the locks. It must not perform I/O or change anything;
/// returning a refusal leaves every target untouched.
pub type Validator<'v> = dyn Fn(&ValidationCandidate<'_>) -> Result<(), ValidationRefusal> + 'v;

/// Why a [`ConfigTransaction`] did not save. Unless
/// [`ConfigWriteError::ReplacementFailed`] lists published targets, no target
/// was changed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigWriteError {
    /// The target path could not be resolved to a file to write.
    #[error("cannot write configuration {}: {problem}", path.display())]
    PathResolution {
        /// The path as given.
        path: PathBuf,
        /// What is wrong with it.
        problem: PathProblem,
    },
    /// Another writer held the target's lock for the whole deadline.
    #[error("another writer held {} for {waited:?}; nothing was saved", lock.display())]
    LockTimeout {
        /// The lock file.
        lock: PathBuf,
        /// How long this writer waited.
        waited: Duration,
    },
    /// Reading a target or taking its lock failed.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file the operation concerned.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The target's bytes differ from the ones the caller expected, or
    /// changed while the transaction held the lock (an editor that does not
    /// take it). The save is abandoned, not retried.
    #[error("{} changed after it was read; the change was not saved", path.display())]
    Changed {
        /// The resolved target.
        path: PathBuf,
    },
    /// An edit cannot be made in place; the error carries its location.
    #[error("{}: {source}", path.display())]
    UnsupportedEdit {
        /// The resolved target.
        path: PathBuf,
        /// The editor's located refusal.
        #[source]
        source: Box<YamlEditError>,
    },
    /// The validation callback refused the edited text.
    #[error("{}: the edited configuration is invalid: {refusal}", path.display())]
    ValidationFailed {
        /// The resolved target.
        path: PathBuf,
        /// The callback's refusal.
        refusal: ValidationRefusal,
    },
    /// Replacing a target failed. That target is untouched and its temporary
    /// file removed; targets replaced before it (lock order, with
    /// [`TargetChange::published_last`] targets after the rest) were already
    /// published.
    #[error("could not replace {}: {source}", path.display())]
    ReplacementFailed {
        /// The target that was not replaced.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
        /// Targets already replaced before this one, in replacement order.
        published: Vec<PathBuf>,
    },
    /// A generated document ([`TargetChange::create_yaml`]) could not be laid
    /// out. Nothing was written.
    #[error("{}: the configuration could not be laid out: {source}", path.display())]
    Layout {
        /// The resolved target.
        path: PathBuf,
        /// The writer's error.
        #[source]
        source: Box<YamlError>,
    },
    /// One transaction names the same file twice (perhaps through two
    /// symlinks) with different expectations or two sets of migration sources.
    #[error("{} is named twice in one transaction with conflicting expectations", path.display())]
    ConflictingChanges {
        /// The resolved target.
        path: PathBuf,
    },
}

/// What the caller expects an existing target to hold when it is re-read
/// under the lock.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourceExpectation {
    /// Whatever the file holds when the lock is taken. The edits name single
    /// values, so a cooperating writer's earlier edit to another value is kept.
    Current,
    /// Exactly these bytes, as the caller read them.
    Bytes(Vec<u8>),
    /// Bytes whose BLAKE3 hex digest is this
    /// ([`migration_receipt::content_hash`]).
    Blake3(String),
}

#[derive(Debug, Clone, PartialEq)]
enum Base {
    Existing(SourceExpectation),
    Create { initial: String },
    /// Created from a value laid out with the target's preference; its own
    /// text holds no authored layout, so edits never infer from it.
    Generated { value: serde_json::Value },
}

/// One file's change, added to a [`ConfigTransaction`].
#[derive(Debug, Clone)]
pub struct TargetChange {
    path: PathBuf,
    base: Base,
    edits: Vec<YamlEdit>,
    migration: Option<Vec<MigrationSource>>,
    publish_last: bool,
}

impl TargetChange {
    /// Edits the existing file at `path`, which must hold what `expected`
    /// describes when it is re-read under the lock.
    pub fn edit(path: impl Into<PathBuf>, expected: SourceExpectation, edits: Vec<YamlEdit>) -> Self {
        Self { path: path.into(), base: Base::Existing(expected), edits, migration: None, publish_last: false }
    }

    /// Creates the file at `path` from `initial` with `edits` applied. The
    /// file must still be missing under the lock; its directory must exist.
    /// `initial` is authored text, so its collections are layout examples
    /// for the edits.
    pub fn create(path: impl Into<PathBuf>, initial: impl Into<String>, edits: Vec<YamlEdit>) -> Self {
        Self { path: path.into(), base: Base::Create { initial: initial.into() }, edits, migration: None, publish_last: false }
    }

    /// Creates the file at `path` from `value`, serialized under the lock with
    /// the target's indentation preference, then `edits` applied with the
    /// same preference. As with [`Self::create`], the file must still be
    /// missing and its directory must exist.
    ///
    /// A JSON value carries neither YAML tags nor non-string keys; a document
    /// that needs them uses [`Self::create`] with an explicit skeleton.
    pub fn create_yaml(path: impl Into<PathBuf>, value: serde_json::Value, edits: Vec<YamlEdit>) -> Self {
        Self { path: path.into(), base: Base::Generated { value }, edits, migration: None, publish_last: false }
    }

    /// Records `sources` in the target's migration receipt, written under the
    /// lock after validation and before the target is replaced; see
    /// [`migration_receipt`].
    pub fn with_migration(mut self, sources: Vec<MigrationSource>) -> Self {
        self.migration = Some(sources);
        self
    }

    /// Replaces this target only after every target without this mark,
    /// whatever the lock order. A two-file change whose second file only
    /// points at the first (a `permissions` reference) uses it so that a
    /// failure between the two replacements leaves the referring file as it
    /// was.
    #[must_use]
    pub fn published_last(mut self) -> Self {
        self.publish_last = true;
        self
    }
}

#[derive(Debug)]
struct PlannedTarget {
    resolved: PathBuf,
    identity: PathIdentity,
    order_key: String,
    base: Base,
    edit_batches: Vec<Vec<YamlEdit>>,
    migration: Option<Vec<MigrationSource>>,
    publish_last: bool,
}

/// One saved target in a [`CommitOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CommittedTarget {
    /// The resolved file that was saved.
    pub path: PathBuf,
    /// Whether the transaction created it.
    pub created: bool,
    /// Whether its bytes were replaced; `false` when the edits left them
    /// unchanged.
    pub written: bool,
    /// The BLAKE3 hex digest of the bytes the target now holds.
    pub hash: String,
    /// Why the target's indentation preference fell back to the default,
    /// naming the target, when resolving it failed.
    pub indentation_warning: Option<String>,
}

/// The result of a successful [`ConfigTransaction::commit`], in replacement
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitOutcome {
    /// Every target, in replacement order: lock order, with
    /// [`TargetChange::published_last`] targets after the rest.
    pub targets: Vec<CommittedTarget>,
}

/// A set of configuration file changes saved under the targets' locks.
///
/// ## Examples
///
/// ```
/// use darkmatter::config::write::{ConfigTransaction, SourceExpectation, TargetChange};
/// use darkmatter::yaml_edit::YamlEdit;
/// use simplified_schema::PropertyPath;
///
/// let dir = tempfile::tempdir().unwrap();
/// let file = dir.path().join("config.yaml");
/// std::fs::write(&file, "# mine\nnetwork: deny\n").unwrap();
///
/// let mut transaction = ConfigTransaction::new();
/// transaction.add(TargetChange::edit(
///     &file,
///     SourceExpectation::Current,
///     vec![YamlEdit::Set { path: PropertyPath::parse("network").unwrap(), value: "allow".into() }],
/// ))?;
/// transaction.commit(&|_| Ok(()))?;
///
/// assert_eq!(std::fs::read_to_string(&file).unwrap(), "# mine\nnetwork: allow\n");
/// # Ok::<(), darkmatter::config::write::ConfigWriteError>(())
/// ```
#[derive(Debug, Default)]
pub struct ConfigTransaction {
    targets: Vec<PlannedTarget>,
    resolver: Resolver,
}

/// Finds a resolved target's indentation preference; `Err` carries why it
/// could not be found.
pub type IndentationResolver = dyn Fn(&Path) -> Result<Indentation, String> + Send + Sync;

#[derive(Default)]
struct Resolver(Option<Box<IndentationResolver>>);

impl fmt::Debug for Resolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.0.is_some() { "Resolver(custom)" } else { "Resolver(sniff)" })
    }
}

impl Resolver {
    fn resolve(&self, path: &Path) -> Result<Indentation, String> {
        match &self.0 {
            Some(resolver) => resolver(path),
            None => resolve_destination_indentation(path),
        }
    }
}

/// The indentation preference Sniff resolves for the file `path` (which need
/// not exist yet); `Err` says why the destination or its worktree could not
/// be identified.
pub fn resolve_destination_indentation(path: &Path) -> Result<Indentation, String> {
    sniff::filesystem::indentation::resolve_for_path(path)
        .map(|resolved| resolved.indentation())
        .map_err(|error| error.to_string())
}

/// [`resolve_destination_indentation`], or [`Indentation::default`] with a
/// warning naming `path` when it fails. For a writer outside a
/// [`ConfigTransaction`] that saves one file.
pub fn destination_indentation(path: &Path) -> Indentation {
    resolve_destination_indentation(path).unwrap_or_else(|reason| {
        tracing::warn!("{}", fallback_warning(path, &reason));
        Indentation::default()
    })
}

fn fallback_warning(path: &Path, reason: &str) -> String {
    format!(
        "{}: could not resolve the indentation preference ({reason}); using {} spaces",
        path.display(),
        Indentation::default().width
    )
}

impl ConfigTransaction {
    /// An empty transaction.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the Sniff resolution of each target's indentation preference
    /// with `resolver`, for a caller that already knows it or a test that
    /// needs a failure.
    #[must_use]
    pub fn with_indentation_resolver(
        mut self,
        resolver: impl Fn(&Path) -> Result<Indentation, String> + Send + Sync + 'static,
    ) -> Self {
        self.resolver = Resolver(Some(Box::new(resolver)));
        self
    }

    /// Resolves `change`'s target and adds it. A file already in the
    /// transaction under another spelling (a symlink alias) is the same
    /// target with one lock: its edits are applied after the earlier ones.
    ///
    /// ## Errors
    ///
    /// [`ConfigWriteError::PathResolution`] or [`ConfigWriteError::Io`] when
    /// the path cannot be resolved; [`ConfigWriteError::ConflictingChanges`]
    /// when the same file was added with a different expectation or with
    /// migration sources twice.
    pub fn add(&mut self, change: TargetChange) -> Result<&mut Self, ConfigWriteError> {
        let resolved = resolve_target(&change.path)?;
        let identity = PathIdentity::new(&resolved);
        if let Some(existing) = self.targets.iter_mut().find(|target| target.identity == identity) {
            if existing.base != change.base || (existing.migration.is_some() && change.migration.is_some()) {
                return Err(ConfigWriteError::ConflictingChanges { path: resolved });
            }
            existing.edit_batches.push(change.edits);
            existing.publish_last |= change.publish_last;
            if change.migration.is_some() {
                existing.migration = change.migration;
            }
            return Ok(self);
        }
        let order_key = to_portable_string(&resolved);
        let position = self.targets.partition_point(|target| target.order_key <= order_key);
        self.targets.insert(
            position,
            PlannedTarget {
                resolved,
                identity,
                order_key,
                base: change.base,
                edit_batches: vec![change.edits],
                migration: change.migration,
                publish_last: change.publish_last,
            },
        );
        Ok(self)
    }

    /// The resolved targets in the order their locks are taken: ascending by
    /// portable path text, so two transactions over the same files never wait
    /// on each other in opposite orders.
    pub fn lock_order(&self) -> Vec<&Path> {
        self.targets.iter().map(|target| target.resolved.as_path()).collect()
    }

    /// Takes every lock, re-reads and edits every target, validates each
    /// candidate with `validate`, writes any migration receipts, checks every
    /// target still holds what was read, and replaces the changed targets.
    ///
    /// ## Errors
    ///
    /// See [`ConfigWriteError`]. Before replacement begins, an error leaves
    /// every target untouched.
    pub fn commit(self, validate: &Validator<'_>) -> Result<CommitOutcome, ConfigWriteError> {
        let _locks = self
            .targets
            .iter()
            .map(|target| acquire_lock(&sibling(&target.resolved, LOCK_SUFFIX)))
            .collect::<Result<Vec<_>, _>>()?;

        let mut staged = Vec::with_capacity(self.targets.len());
        for target in &self.targets {
            staged.push(stage(target, validate, &self.resolver)?);
        }

        for (target, stage) in self.targets.iter().zip(&staged) {
            if let Some(sources) = &target.migration {
                let hash = migration_receipt::content_hash(stage.candidate.as_bytes());
                migration_receipt::write_locked(&target.resolved, sources, &hash)?;
            }
        }

        // The last look before replacing: an editor that ignores the lock may
        // have saved since the re-read. Nothing has been replaced yet.
        for (target, stage) in self.targets.iter().zip(&staged) {
            if read_optional(&target.resolved)? != stage.current {
                return Err(ConfigWriteError::Changed { path: target.resolved.clone() });
            }
        }

        let mut published = Vec::new();
        let mut targets = Vec::with_capacity(self.targets.len());
        // Lock order, except that a `published_last` target waits for the rest.
        let mut order: Vec<(&PlannedTarget, Staged)> = self.targets.iter().zip(staged).collect();
        order.sort_by_key(|(target, _)| target.publish_last);
        for (target, stage) in order {
            let bytes = stage.candidate.as_bytes();
            let written = stage.current.as_deref() != Some(bytes);
            if written {
                replace_target(&target.resolved, bytes, stage.current.is_some()).map_err(|source| {
                    ConfigWriteError::ReplacementFailed {
                        path: target.resolved.clone(),
                        source,
                        published: published.clone(),
                    }
                })?;
                published.push(target.resolved.clone());
            }
            targets.push(CommittedTarget {
                path: target.resolved.clone(),
                created: stage.current.is_none(),
                written,
                hash: migration_receipt::content_hash(bytes),
                indentation_warning: stage.indentation_warning.clone(),
            });
        }
        Ok(CommitOutcome { targets })
    }
}

struct Staged {
    current: Option<Vec<u8>>,
    candidate: String,
    indentation_warning: Option<String>,
}

fn stage(target: &PlannedTarget, validate: &Validator<'_>, resolver: &Resolver) -> Result<Staged, ConfigWriteError> {
    let path = &target.resolved;
    let changed = || ConfigWriteError::Changed { path: path.clone() };
    // Resolved once for the target, and only if a value is rendered with no
    // example to copy; a discovery failure falls back to the default.
    let preference = OnceCell::new();
    let warning = RefCell::new(None);
    let fallback = || {
        *preference.get_or_init(|| {
            resolver.resolve(path).unwrap_or_else(|reason| {
                let message = fallback_warning(path, &reason);
                tracing::warn!("{message}");
                *warning.borrow_mut() = Some(message);
                Indentation::default()
            })
        })
    };
    let current = read_optional(path)?;
    let (original, mut candidate) = match (&target.base, &current) {
        (Base::Create { initial }, None) => (None, initial.clone()),
        (Base::Generated { value }, None) => {
            let text = biscuit_file::to_yaml_string(value, fallback())
                .map_err(|source| ConfigWriteError::Layout { path: path.clone(), source: Box::new(source) })?;
            (None, text)
        }
        (Base::Create { .. } | Base::Generated { .. }, Some(_)) => return Err(changed()),
        (Base::Existing(SourceExpectation::Current), None) => {
            return Err(ConfigWriteError::Io {
                path: path.clone(),
                source: std::io::Error::new(ErrorKind::NotFound, "the configuration file does not exist"),
            });
        }
        (Base::Existing(_), None) => return Err(changed()),
        (Base::Existing(expected), Some(bytes)) => {
            let matches = match expected {
                SourceExpectation::Current => true,
                SourceExpectation::Bytes(expected) => expected == bytes,
                SourceExpectation::Blake3(expected) => *expected == migration_receipt::content_hash(bytes),
            };
            if !matches {
                return Err(changed());
            }
            let text = std::str::from_utf8(bytes).map_err(|_| ConfigWriteError::Io {
                path: path.clone(),
                source: std::io::Error::new(ErrorKind::InvalidData, "the configuration file is not UTF-8"),
            })?;
            (Some(text), text.to_string())
        }
    };
    let layout = Layout::new(&fallback, !matches!(target.base, Base::Generated { .. }));
    for edits in &target.edit_batches {
        if !edits.is_empty() {
            candidate = edit_yaml_paths_in_layout(&candidate, edits, &layout)
                .map_err(|source| ConfigWriteError::UnsupportedEdit { path: path.clone(), source: Box::new(source) })?;
        }
    }
    validate(&ValidationCandidate { target: path, original, candidate: &candidate })
        .map_err(|refusal| ConfigWriteError::ValidationFailed { path: path.clone(), refusal })?;
    Ok(Staged { current, candidate, indentation_warning: warning.into_inner() })
}

/// A held target lock, released when dropped. The lock file stays.
#[derive(Debug)]
pub(crate) struct TargetLock {
    _file: File,
}

/// Takes the exclusive lock on `lock`, creating the file when needed, waiting
/// up to [`LOCK_DEADLINE`] in [`LOCK_BACKOFF`] steps.
pub(crate) fn acquire_lock(lock: &Path) -> Result<TargetLock, ConfigWriteError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock)
        .map_err(|source| ConfigWriteError::Io { path: lock.to_path_buf(), source })?;
    let deadline = lock_deadline();
    let started = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(TargetLock { _file: file }),
            Err(TryLockError::WouldBlock) => {
                let waited = started.elapsed();
                if waited >= deadline {
                    return Err(ConfigWriteError::LockTimeout { lock: lock.to_path_buf(), waited });
                }
                std::thread::sleep(LOCK_BACKOFF.min(deadline - waited));
            }
            Err(TryLockError::Error(source)) => {
                return Err(ConfigWriteError::Io { path: lock.to_path_buf(), source });
            }
        }
    }
}

/// The real file `path` names: symlinks followed, or for a missing file its
/// canonical directory joined with its name.
pub(crate) fn resolve_target(path: &Path) -> Result<PathBuf, ConfigWriteError> {
    let problem = |problem| ConfigWriteError::PathResolution { path: path.to_path_buf(), problem };
    let io = |source| ConfigWriteError::Io { path: path.to_path_buf(), source };
    if !path.is_absolute() {
        return Err(problem(PathProblem::NotAbsolute));
    }
    let Some(Component::Normal(name)) = path.components().next_back() else {
        return Err(problem(PathProblem::NoFileName));
    };
    match canonicalize_simplified(path) {
        Ok(real) => {
            if std::fs::metadata(&real).map_err(io)?.is_file() {
                Ok(real)
            } else {
                Err(problem(PathProblem::NotAFile))
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            if std::fs::symlink_metadata(path).is_ok() {
                return Err(problem(PathProblem::DanglingSymlink));
            }
            let parent = path.parent().ok_or_else(|| problem(PathProblem::NoFileName))?;
            match canonicalize_simplified(parent) {
                Ok(directory) if directory.is_dir() => Ok(directory.join(name)),
                Ok(_) => Err(problem(PathProblem::MissingParent)),
                Err(error) if error.kind() == ErrorKind::NotFound => Err(problem(PathProblem::MissingParent)),
                Err(error) => Err(io(error)),
            }
        }
        Err(error) => Err(io(error)),
    }
}

/// `resolved`'s sibling named by appending `suffix` to its file name.
pub(crate) fn sibling(resolved: &Path, suffix: &str) -> PathBuf {
    let mut name = resolved.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    resolved.with_file_name(name)
}

pub(crate) fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, ConfigWriteError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(ConfigWriteError::Io { path: path.to_path_buf(), source }),
    }
}

/// Atomically replaces (or creates) `path` with `bytes` through a synced
/// temporary file in the same directory.
pub(crate) fn replace_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    replace_with(path, bytes, None, || Ok(()))
}

/// [`replace_file`] for a configuration target, keeping an existing target's
/// permissions.
fn replace_target(path: &Path, bytes: &[u8], existing: bool) -> std::io::Result<()> {
    let permissions = if existing { Some(std::fs::metadata(path)?.permissions()) } else { None };
    // Windows refuses to rename over a read-only file; refuse before creating
    // a temporary file rather than clear the attribute the user set.
    #[cfg(windows)]
    if permissions.as_ref().is_some_and(std::fs::Permissions::readonly) {
        return Err(std::io::Error::new(ErrorKind::PermissionDenied, "the configuration file is read-only"));
    }
    replace_with(path, bytes, permissions, injected_replacement_failure)
}

fn replace_with(
    path: &Path,
    bytes: &[u8],
    permissions: Option<std::fs::Permissions>,
    before_persist: impl FnOnce() -> std::io::Result<()>,
) -> std::io::Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidInput, "the path has no directory"))?;
    // Every early return drops `temporary`, which deletes its file.
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(bytes)?;
    if let Some(permissions) = permissions {
        temporary.as_file().set_permissions(permissions)?;
    }
    temporary.as_file().sync_all()?;
    before_persist()?;
    crate::atomic_replace::persist(temporary, path, std::thread::sleep)
}

fn lock_deadline() -> Duration {
    #[cfg(feature = "effects-instrumentation")]
    if let Some(deadline) = test_hooks::LOCK_DEADLINE.with(std::cell::Cell::get) {
        return deadline;
    }
    LOCK_DEADLINE
}

fn injected_replacement_failure() -> std::io::Result<()> {
    #[cfg(feature = "effects-instrumentation")]
    match test_hooks::FAIL_REPLACEMENT.with(std::cell::Cell::get) {
        Some(0) => {
            test_hooks::FAIL_REPLACEMENT.with(|cell| cell.set(None));
            return Err(std::io::Error::other("injected replacement failure"));
        }
        Some(remaining) => test_hooks::FAIL_REPLACEMENT.with(|cell| cell.set(Some(remaining - 1))),
        None => {}
    }
    Ok(())
}

/// Test-only seams for the writer, behind the `effects-instrumentation`
/// feature. Each setting applies to the calling thread until its guard drops.
#[cfg(feature = "effects-instrumentation")]
pub mod test_hooks {
    use std::cell::Cell;
    use std::time::Duration;

    thread_local! {
        pub(super) static LOCK_DEADLINE: Cell<Option<Duration>> = const { Cell::new(None) };
        /// How many replacements succeed before the injected failure.
        pub(super) static FAIL_REPLACEMENT: Cell<Option<usize>> = const { Cell::new(None) };
    }

    /// Restores a hook's default when dropped.
    #[must_use = "the hook is reset when the guard drops"]
    pub struct HookGuard(fn());

    impl Drop for HookGuard {
        fn drop(&mut self) {
            (self.0)();
        }
    }

    /// Replaces [`super::LOCK_DEADLINE`] so a timeout test need not wait two
    /// seconds.
    pub fn lock_deadline(deadline: Duration) -> HookGuard {
        LOCK_DEADLINE.with(|cell| cell.set(Some(deadline)));
        HookGuard(|| LOCK_DEADLINE.with(|cell| cell.set(None)))
    }

    /// Makes the next target replacement fail after its temporary file is
    /// written and synced, just before the rename.
    pub fn fail_next_replacement() -> HookGuard {
        fail_replacement_after(0)
    }

    /// Lets `succeeding` replacements through, then makes the next one fail
    /// as [`fail_next_replacement`] does: a two-file transaction interrupted
    /// between its replacements.
    pub fn fail_replacement_after(succeeding: usize) -> HookGuard {
        FAIL_REPLACEMENT.with(|cell| cell.set(Some(succeeding)));
        HookGuard(|| FAIL_REPLACEMENT.with(|cell| cell.set(None)))
    }
}
