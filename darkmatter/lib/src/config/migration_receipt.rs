//! Migration receipts: the record that lets a migration finish deleting (or
//! backing up) the files it converted after a crash between publishing the
//! new configuration and cleaning up.
//!
//! A migration adds its sources to the target's change with
//! [`TargetChange::with_migration`](crate::config::write::TargetChange::with_migration).
//! The transaction writes the receipt under the target's lock, after
//! validation and before the target is replaced, at the sibling
//! `<target-name>.darkmatter-migration.json`. [`complete_cleanup`] later
//! acts on it, and only when the target still holds exactly the published
//! bytes, every source still holds the recorded bytes, and every path is one
//! the caller names as a known legacy file. A receipt never grants deletion
//! authority by itself.
//!
//! The receipt is a closed JSON object, every field required and non-null:
//!
//! ```json
//! {
//!   "format_version": 1,
//!   "target": "/home/me/.darkmatter.yaml",
//!   "published_hash": "<BLAKE3 hex of the published target bytes>",
//!   "sources": [
//!     { "path": "/home/me/.darkmatter-shell-whitelist", "hash": "<BLAKE3 hex>", "action": "delete" },
//!     { "path": "/home/me/old.json5", "hash": "<BLAKE3 hex>", "action": "backup", "backup_path": "/home/me/old.json5.bak" }
//!   ]
//! }
//! ```
//!
//! User-facing guide: `darkmatter/docs/topics/config-writes.md`.

use crate::config::write::{
    ConfigWriteError, LOCK_SUFFIX, PathProblem, acquire_lock, read_optional, replace_file, resolve_target,
    sibling,
};
use biscuit_file::{PathIdentity, canonicalize_simplified, try_portable_string};
use serde_json::{Map, Value, json};
use std::ffi::OsString;
use std::fmt;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

/// Appended to a target's file name to name its receipt, a sibling in the
/// same directory.
pub const RECEIPT_SUFFIX: &str = ".darkmatter-migration.json";

/// The only receipt format this reader accepts.
pub const RECEIPT_FORMAT_VERSION: u64 = 1;

/// The BLAKE3 digest of `bytes` as 64 lowercase hex characters: the hash a
/// receipt records and [`SourceExpectation::Blake3`](crate::config::write::SourceExpectation::Blake3)
/// compares.
pub fn content_hash(bytes: &[u8]) -> String {
    let mut reader = bytes;
    biscuit_hash::blake3_hash_reader(&mut reader).expect("reading a byte slice cannot fail")
}

/// What cleanup does with a migrated source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationAction {
    /// Delete it.
    Delete,
    /// Rename it to `backup_path`, which must not exist.
    Backup {
        /// Where the source is moved.
        backup_path: PathBuf,
    },
}

/// A file a migration converted, as recorded in a receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationSource {
    /// The source file. Its directory is resolved; a symlink at the final
    /// name is the source itself, so cleanup removes the link, not its
    /// destination.
    pub path: PathBuf,
    /// The [`content_hash`] of the bytes that were converted.
    pub hash: String,
    /// What cleanup does with it.
    pub action: MigrationAction,
}

impl MigrationSource {
    /// A source to delete once the target is published; `bytes` are the
    /// bytes the migration converted.
    pub fn delete(path: impl Into<PathBuf>, bytes: &[u8]) -> Self {
        Self { path: path.into(), hash: content_hash(bytes), action: MigrationAction::Delete }
    }

    /// A source to move to `backup_path` once the target is published.
    pub fn backup(path: impl Into<PathBuf>, bytes: &[u8], backup_path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            hash: content_hash(bytes),
            action: MigrationAction::Backup { backup_path: backup_path.into() },
        }
    }
}

/// A parsed receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReceipt {
    /// The resolved target, from its portable text.
    pub target: PathBuf,
    /// The [`content_hash`] of the bytes the transaction published.
    pub published_hash: String,
    /// The recorded sources, never empty.
    pub sources: Vec<MigrationSource>,
}

/// What a [`MigrationError`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MigrationErrorKind {
    /// The target path could not be resolved.
    PathResolution,
    /// Another writer held the target's lock for the whole deadline.
    LockTimeout,
    /// The receipt could not be read, or is not UTF-8.
    Unreadable,
    /// The receipt is not a valid version-1 receipt.
    Malformed,
    /// The receipt names another target.
    TargetMismatch,
    /// A source or backup path is not one of the caller's known legacy files,
    /// or is the target itself.
    UnknownPath,
    /// The target does not hold the published bytes: it was never published,
    /// or has changed since.
    Unpublished,
    /// A source no longer holds the recorded bytes.
    SourceChanged,
    /// A backup destination already exists.
    BackupOccupied,
    /// Deleting, moving, or reading a file failed.
    Io,
}

/// Why [`complete_cleanup`] or [`read_receipt`] stopped. Every source is kept
/// unless an earlier item of the same cleanup already finished.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}{}: {message}", receipt.display(), pointer_text(pointer))]
pub struct MigrationError {
    /// The kind of failure.
    pub kind: MigrationErrorKind,
    /// The receipt file.
    pub receipt: PathBuf,
    /// The JSON pointer of the receipt value concerned (`/sources/0/hash`);
    /// empty for the receipt as a whole.
    pub pointer: String,
    /// A human-readable explanation.
    pub message: String,
}

fn pointer_text(pointer: &str) -> String {
    if pointer.is_empty() { String::new() } else { format!(" at {pointer}") }
}

/// What [`complete_cleanup`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CleanupOutcome {
    /// No receipt exists: nothing was pending, or cleanup already finished.
    NothingPending,
    /// Every recorded source is gone or backed up and the receipt is removed.
    Completed {
        /// Sources deleted now.
        deleted: Vec<PathBuf>,
        /// Sources moved now, with their backup paths.
        backed_up: Vec<(PathBuf, PathBuf)>,
        /// Sources that were already gone.
        already_clean: Vec<PathBuf>,
    },
}

/// The receipt file for the configuration at `target`.
///
/// ## Errors
///
/// The [`ConfigWriteError`] of resolving `target`.
pub fn receipt_path(target: &Path) -> Result<PathBuf, ConfigWriteError> {
    Ok(sibling(&resolve_target(target)?, RECEIPT_SUFFIX))
}

/// Reads the pending receipt for `target` under its lock; `None` when no
/// cleanup is pending.
///
/// ## Errors
///
/// A located [`MigrationError`] for an unreadable or malformed receipt, a
/// lock timeout, or an unresolvable target.
pub fn read_receipt(target: &Path) -> Result<Option<MigrationReceipt>, MigrationError> {
    let (resolved, receipt) = locate(target)?;
    let _lock = lock(&resolved, &receipt)?;
    load(&receipt)
}

/// Finishes a migration's cleanup for the configuration at `target`.
///
/// `known_sources` are the exact legacy files (and backup destinations) the
/// calling program's captured scope may clean up. A receipt path outside that
/// list, or naming the target, is refused. Before touching anything the
/// target must hold the published bytes and every remaining source its
/// recorded bytes; a source already gone counts as cleaned. On success the
/// receipt is removed, so a repeated call returns
/// [`CleanupOutcome::NothingPending`].
///
/// ## Errors
///
/// A [`MigrationError`] located at the receipt value concerned; every source
/// is kept unless it was the file system operation itself that failed part
/// way through.
pub fn complete_cleanup(target: &Path, known_sources: &[PathBuf]) -> Result<CleanupOutcome, MigrationError> {
    let (resolved, receipt_file) = locate(target)?;
    let _lock = lock(&resolved, &receipt_file)?;
    let Some(receipt) = load(&receipt_file)? else {
        return Ok(CleanupOutcome::NothingPending);
    };
    let fail = |kind, pointer: String, message: String| MigrationError {
        kind,
        receipt: receipt_file.clone(),
        pointer,
        message,
    };

    let target_identity = PathIdentity::new(&resolved);
    if PathIdentity::new(&receipt.target) != target_identity {
        return Err(fail(
            MigrationErrorKind::TargetMismatch,
            "/target".into(),
            format!("the receipt names {}, not {}", receipt.target.display(), resolved.display()),
        ));
    }
    let mut known = Vec::with_capacity(known_sources.len());
    for path in known_sources {
        let entry = resolve_entry(path).map_err(|problem| {
            fail(
                MigrationErrorKind::PathResolution,
                String::new(),
                format!("known source {}: {problem}", path.display()),
            )
        })?;
        known.push(PathIdentity::new(&entry));
    }
    let admit = |path: &Path, pointer: String| -> Result<PathBuf, MigrationError> {
        let refused = || {
            fail(
                MigrationErrorKind::UnknownPath,
                pointer.clone(),
                format!("{} is not a known legacy file for this configuration", path.display()),
            )
        };
        let entry = resolve_entry(path).map_err(|_| refused())?;
        let identity = PathIdentity::new(&entry);
        if identity == target_identity || !known.contains(&identity) {
            return Err(refused());
        }
        Ok(entry)
    };

    let mut plan = Vec::with_capacity(receipt.sources.len());
    for (index, source) in receipt.sources.iter().enumerate() {
        let path = admit(&source.path, format!("/sources/{index}/path"))?;
        let backup = match &source.action {
            MigrationAction::Delete => None,
            MigrationAction::Backup { backup_path } => {
                Some(admit(backup_path, format!("/sources/{index}/backup_path"))?)
            }
        };
        plan.push((index, path, backup));
    }

    let published = read_optional(&resolved).map_err(|error| {
        fail(MigrationErrorKind::Io, String::new(), error.to_string())
    })?;
    if published.as_deref().map(content_hash).as_deref() != Some(receipt.published_hash.as_str()) {
        return Err(fail(
            MigrationErrorKind::Unpublished,
            "/published_hash".into(),
            format!("{} does not hold the published configuration; the sources are kept", resolved.display()),
        ));
    }

    let mut pending = Vec::new();
    let mut already_clean = Vec::new();
    for (index, path, backup) in plan {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound && std::fs::symlink_metadata(&path).is_err() => {
                already_clean.push(path);
                continue;
            }
            Err(error) => {
                return Err(fail(MigrationErrorKind::Io, format!("/sources/{index}/path"), format!("{}: {error}", path.display())));
            }
        };
        if content_hash(&bytes) != receipt.sources[index].hash {
            return Err(fail(
                MigrationErrorKind::SourceChanged,
                format!("/sources/{index}/hash"),
                format!("{} changed after it was migrated; it is kept", path.display()),
            ));
        }
        if let Some(backup) = &backup
            && std::fs::symlink_metadata(backup).is_ok()
        {
            return Err(fail(
                MigrationErrorKind::BackupOccupied,
                format!("/sources/{index}/backup_path"),
                format!("{} already exists; {} is kept", backup.display(), path.display()),
            ));
        }
        pending.push((index, path, backup));
    }

    let mut deleted = Vec::new();
    let mut backed_up = Vec::new();
    for (index, path, backup) in pending {
        let result = match &backup {
            None => std::fs::remove_file(&path),
            Some(backup) => std::fs::rename(&path, backup),
        };
        result.map_err(|error| {
            fail(MigrationErrorKind::Io, format!("/sources/{index}/path"), format!("{}: {error}", path.display()))
        })?;
        match backup {
            None => deleted.push(path),
            Some(backup) => backed_up.push((path, backup)),
        }
    }
    std::fs::remove_file(&receipt_file)
        .map_err(|error| fail(MigrationErrorKind::Io, String::new(), format!("removing the receipt: {error}")))?;
    Ok(CleanupOutcome::Completed { deleted, backed_up, already_clean })
}

/// Writes the receipt for the resolved `target`, whose lock the caller holds.
pub(crate) fn write_locked(
    target: &Path,
    sources: &[MigrationSource],
    published_hash: &str,
) -> Result<(), ConfigWriteError> {
    let portable = |path: &Path| -> Result<String, ConfigWriteError> {
        let entry = resolve_entry(path)
            .map_err(|problem| ConfigWriteError::PathResolution { path: path.to_path_buf(), problem })?;
        faithful_text(&entry)
            .ok_or(ConfigWriteError::PathResolution { path: path.to_path_buf(), problem: PathProblem::NotPortable })
    };
    if sources.is_empty() {
        return Err(ConfigWriteError::Io {
            path: target.to_path_buf(),
            source: std::io::Error::new(ErrorKind::InvalidInput, "a migration receipt needs at least one source"),
        });
    }
    let mut recorded = Vec::with_capacity(sources.len());
    for source in sources {
        let mut item = Map::new();
        item.insert("path".into(), portable(&source.path)?.into());
        item.insert("hash".into(), source.hash.clone().into());
        match &source.action {
            MigrationAction::Delete => {
                item.insert("action".into(), "delete".into());
            }
            MigrationAction::Backup { backup_path } => {
                item.insert("action".into(), "backup".into());
                item.insert("backup_path".into(), portable(backup_path)?.into());
            }
        }
        recorded.push(Value::Object(item));
    }
    let target_text = faithful_text(target)
        .ok_or(ConfigWriteError::PathResolution { path: target.to_path_buf(), problem: PathProblem::NotPortable })?;
    let receipt = json!({
        "format_version": RECEIPT_FORMAT_VERSION,
        "target": target_text,
        "published_hash": published_hash,
        "sources": recorded,
    });
    let mut text = serde_json::to_string_pretty(&receipt).expect("a JSON value always serializes");
    text.push('\n');
    let path = sibling(target, RECEIPT_SUFFIX);
    replace_file(&path, text.as_bytes()).map_err(|source| ConfigWriteError::Io { path, source })
}

/// `path` as portable text, only when that text names `path` again.
fn faithful_text(path: &Path) -> Option<String> {
    let text = try_portable_string(path)?;
    (PathIdentity::new(Path::new(&text)) == PathIdentity::new(path)).then_some(text)
}

/// `path` with its directory resolved (its nearest existing ancestor
/// canonicalized and the missing names appended) and its final name kept.
fn resolve_entry(path: &Path) -> Result<PathBuf, PathProblem> {
    if !path.is_absolute() {
        return Err(PathProblem::NotAbsolute);
    }
    let Some(Component::Normal(name)) = path.components().next_back() else {
        return Err(PathProblem::NoFileName);
    };
    let mut missing = vec![name.to_os_string()];
    let mut ancestor = path.parent().ok_or(PathProblem::NoFileName)?;
    loop {
        match canonicalize_simplified(ancestor) {
            Ok(mut resolved) => {
                for name in missing.iter().rev() {
                    resolved.push(name);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                let Some(Component::Normal(name)) = ancestor.components().next_back() else {
                    return Err(PathProblem::NoFileName);
                };
                missing.push(OsString::from(name));
                ancestor = ancestor.parent().ok_or(PathProblem::MissingParent)?;
            }
            Err(_) => return Err(PathProblem::MissingParent),
        }
    }
}

fn locate(target: &Path) -> Result<(PathBuf, PathBuf), MigrationError> {
    let resolved = resolve_target(target).map_err(|error| MigrationError {
        kind: MigrationErrorKind::PathResolution,
        receipt: target.to_path_buf(),
        pointer: String::new(),
        message: error.to_string(),
    })?;
    let receipt = sibling(&resolved, RECEIPT_SUFFIX);
    Ok((resolved, receipt))
}

fn lock(resolved: &Path, receipt: &Path) -> Result<crate::config::write::TargetLock, MigrationError> {
    acquire_lock(&sibling(resolved, LOCK_SUFFIX)).map_err(|error| MigrationError {
        kind: match error {
            ConfigWriteError::LockTimeout { .. } => MigrationErrorKind::LockTimeout,
            _ => MigrationErrorKind::Io,
        },
        receipt: receipt.to_path_buf(),
        pointer: String::new(),
        message: error.to_string(),
    })
}

fn load(receipt: &Path) -> Result<Option<MigrationReceipt>, MigrationError> {
    let unreadable = |message: String| MigrationError {
        kind: MigrationErrorKind::Unreadable,
        receipt: receipt.to_path_buf(),
        pointer: String::new(),
        message,
    };
    let bytes = match std::fs::read(receipt) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(unreadable(error.to_string())),
    };
    let text = String::from_utf8(bytes).map_err(|_| unreadable("the receipt is not UTF-8".into()))?;
    parse(&text, receipt).map(Some)
}

struct Shape<'a> {
    receipt: &'a Path,
}

impl Shape<'_> {
    fn error(&self, pointer: &str, message: impl fmt::Display) -> MigrationError {
        MigrationError {
            kind: MigrationErrorKind::Malformed,
            receipt: self.receipt.to_path_buf(),
            pointer: pointer.to_string(),
            message: message.to_string(),
        }
    }

    fn object<'v>(&self, value: &'v Value, pointer: &str, keys: &[&str]) -> Result<&'v Map<String, Value>, MigrationError> {
        let object = value.as_object().ok_or_else(|| self.error(pointer, "must be a JSON object"))?;
        if let Some(unknown) = object.keys().find(|key| !keys.contains(&key.as_str())) {
            return Err(self.error(&child(pointer, unknown), format_args!("unknown field `{unknown}`")));
        }
        Ok(object)
    }

    fn field<'v>(&self, object: &'v Map<String, Value>, pointer: &str, key: &str) -> Result<&'v Value, MigrationError> {
        match object.get(key) {
            None => Err(self.error(&child(pointer, key), format_args!("missing required field `{key}`"))),
            Some(Value::Null) => Err(self.error(&child(pointer, key), format_args!("`{key}` must not be null"))),
            Some(value) => Ok(value),
        }
    }

    fn text<'v>(&self, object: &'v Map<String, Value>, pointer: &str, key: &str) -> Result<&'v str, MigrationError> {
        match self.field(object, pointer, key)? {
            Value::String(text) if !text.is_empty() => Ok(text),
            _ => Err(self.error(&child(pointer, key), format_args!("`{key}` must be a non-empty string"))),
        }
    }

    fn hash(&self, object: &Map<String, Value>, pointer: &str, key: &str) -> Result<String, MigrationError> {
        let text = self.text(object, pointer, key)?;
        if text.len() == 64 && text.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')) {
            Ok(text.to_string())
        } else {
            Err(self.error(&child(pointer, key), format_args!("`{key}` must be a 64-character lowercase BLAKE3 hex digest")))
        }
    }
}

fn child(pointer: &str, key: &str) -> String {
    format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"))
}

pub(crate) fn parse(text: &str, receipt: &Path) -> Result<MigrationReceipt, MigrationError> {
    let shape = Shape { receipt };
    let value = crate::strict_json::parse(text).map_err(|error| shape.error("", format_args!("the receipt {error}")))?;
    let root = shape.object(&value, "", &["format_version", "target", "published_hash", "sources"])?;

    match shape.field(root, "", "format_version")? {
        Value::Number(number) if number.as_u64() == Some(RECEIPT_FORMAT_VERSION) => {}
        Value::Number(number) => {
            return Err(shape.error(
                "/format_version",
                format_args!("unsupported format_version {number}; this reader accepts {RECEIPT_FORMAT_VERSION}"),
            ));
        }
        _ => return Err(shape.error("/format_version", "`format_version` must be the integer 1")),
    }
    let target = PathBuf::from(shape.text(root, "", "target")?);
    let published_hash = shape.hash(root, "", "published_hash")?;
    let Value::Array(items) = shape.field(root, "", "sources")? else {
        return Err(shape.error("/sources", "`sources` must be an array"));
    };
    if items.is_empty() {
        return Err(shape.error("/sources", "`sources` must name at least one source"));
    }
    let mut sources = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let pointer = format!("/sources/{index}");
        let object = shape.object(item, &pointer, &["path", "hash", "action", "backup_path"])?;
        let path = PathBuf::from(shape.text(object, &pointer, "path")?);
        let hash = shape.hash(object, &pointer, "hash")?;
        let action = match shape.text(object, &pointer, "action")? {
            "delete" => {
                if object.contains_key("backup_path") {
                    return Err(shape.error(
                        &child(&pointer, "backup_path"),
                        "`backup_path` is allowed only when `action` is `backup`",
                    ));
                }
                MigrationAction::Delete
            }
            "backup" => MigrationAction::Backup { backup_path: PathBuf::from(shape.text(object, &pointer, "backup_path")?) },
            other => {
                return Err(shape.error(
                    &child(&pointer, "action"),
                    format_args!("unknown action `{other}`; expected `delete` or `backup`"),
                ));
            }
        };
        sources.push(MigrationSource { path, hash, action });
    }
    Ok(MigrationReceipt { target, published_hash, sources })
}
