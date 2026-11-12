//! The persisted execution-constraint store: where it lives, how its records
//! are read, which of them bind a push, and whether a resolved plan already
//! satisfies them.
//!
//! A port of `scripts/ci/constraints.py`, which `just ci-local` and the
//! planner still use; the two readers agree case by case through the shared
//! parity fixture `scripts/ci/fixtures/pre-push/constraints/parity.json`.
//! Record and plan shapes are validated by [`parse_constraint_record`] and
//! [`parse_constraint_plan`]; this module adds the store walk and matching.
//!
//! A malformed record binds: an instruction that cannot be read is not one
//! that can be ignored, so it is never filtered by repository, branch, or
//! expiry, and no plan satisfies it.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;

use super::documents::{
    ConstraintCell, ConstraintRecord, DocumentError, parse_constraint_plan,
    parse_constraint_record,
};

/// Overrides the store directory when set and nonempty.
pub const ENVIRONMENT_VARIABLE: &str = "BISCUIT_CI_CONSTRAINTS_DIR";

/// Characters a repository identity may carry that some filesystem refuses in
/// a path segment: a port (`host:2222`), a Windows drive (`C:`), and the rest
/// of the NTFS reserved set.
const UNPORTABLE_SEGMENT_CHARACTERS: &str = "<>:\"|?*";

/// `host/path` for a remote URL in any Git spelling, without scheme,
/// credentials, trailing slashes, or one `.git` suffix; text that is neither
/// URL nor scp-like is returned trimmed.
///
/// The SSH and HTTPS spellings of one remote, and every worktree of one clone,
/// share an identity. `""` means the repository is unknown.
pub fn repository_identity(remote: &str) -> String {
    // Python's `str.strip()` also removes U+001C..U+001F, which Rust's
    // `char::is_whitespace` does not.
    let mut remote = remote
        .trim_matches(|character: char| character.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&character))
        .to_owned();
    if let Some((_, rest)) = remote.split_once("://") {
        remote = rest.to_owned();
    } else if remote.split_once(':').is_some_and(|(host, _)| !host.contains('/')) {
        // scp-like `git@host:owner/name`
        remote = remote.replacen(':', "/", 1);
    }
    if remote.split('/').next().is_some_and(|first| first.contains('@')) {
        if let Some((_, rest)) = remote.split_once('@') {
            remote = rest.to_owned();
        }
    }
    let remote = remote.trim_end_matches('/');
    remote.strip_suffix(".git").unwrap_or(remote).to_owned()
}

/// A repository identity as nested directory names under the store root.
///
/// A filesystem remote's identity is an absolute path, so the segments are
/// always relative: joining an absolute identity would replace the root and
/// put a record outside the store. Empty, `.`, and `..` segments are dropped,
/// and each unportable character becomes `_` so one identity is one directory
/// on every OS.
pub fn store_segments(identity: &str) -> Vec<String> {
    identity
        .replace('\\', "/")
        .split('/')
        .map(|segment| {
            segment
                .chars()
                .map(|character| {
                    if UNPORTABLE_SEGMENT_CHARACTERS.contains(character) || u32::from(character) < 32 {
                        '_'
                    } else {
                        character
                    }
                })
                .collect::<String>()
        })
        .filter(|segment| !matches!(segment.as_str(), "" | "." | ".."))
        .collect()
}

/// `<home>/.rusty-biscuit/ci-constraints`, beside the evidence directory.
///
/// `home` is the caller's: `USERPROFILE` on native Windows, `HOME` elsewhere.
pub fn store_root(home: &Path) -> PathBuf {
    home.join(".rusty-biscuit").join("ci-constraints")
}

/// The store directory for `repository`, a remote URL in any spelling.
///
/// An unknown repository resolves to the store root, which [`load`] reads
/// recursively, so every repository's records bind.
pub fn default_directory(home: &Path, repository: &str) -> PathBuf {
    let mut directory = store_root(home);
    directory.extend(store_segments(&repository_identity(repository)));
    directory
}

/// The store to read: `explicit`, else the [`ENVIRONMENT_VARIABLE`] value,
/// else [`default_directory`]. An empty value counts as unset.
pub fn resolve_directory(
    home: &Path,
    repository: &str,
    explicit: Option<&Path>,
    environment_value: Option<&OsStr>,
) -> PathBuf {
    if let Some(explicit) = explicit.filter(|path| !path.as_os_str().is_empty()) {
        return explicit.to_path_buf();
    }
    if let Some(value) = environment_value.filter(|value| !value.is_empty()) {
        return PathBuf::from(value);
    }
    default_directory(home, repository)
}

/// One record file in the store, valid or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    pub path: PathBuf,
    /// The record, or why it cannot be trusted; the error names `path` and
    /// the JSON field.
    pub record: Result<ConstraintRecord, DocumentError>,
}

impl Constraint {
    pub fn is_malformed(&self) -> bool {
        self.record.is_err()
    }

    /// Whether this record governs a push to `repository` under any of
    /// `branches`.
    ///
    /// An unset `repository` or `branch` means "everywhere". An empty or
    /// unknown `repository`, and a `branches` with no nonempty name, are an
    /// unknown identity, which cannot prove exemption; a malformed record
    /// applies everywhere.
    pub fn applies_to<S: AsRef<str>>(&self, repository: &str, branches: &[S]) -> bool {
        let Ok(record) = &self.record else {
            return true;
        };
        let declared = repository_identity(&record.repository);
        let actual = repository_identity(repository);
        if !declared.is_empty() && !actual.is_empty() && declared != actual {
            return false;
        }
        let mut names = branches.iter().map(AsRef::as_ref).filter(|name| !name.is_empty()).peekable();
        if !record.branch.is_empty() && names.peek().is_some() && !names.any(|name| name == record.branch) {
            return false;
        }
        true
    }

    /// Whether the record stopped binding before `today`: a record expiring
    /// today still binds today. A malformed record never expires.
    pub fn expired(&self, today: NaiveDate) -> bool {
        self.record.as_ref().is_ok_and(|record| record.expiry < today)
    }

    /// One line for a refusal or an expiry notice.
    pub fn describe(&self) -> String {
        match &self.record {
            Err(error) => error.to_string(),
            Ok(record) => {
                let gate = if record.gate.is_empty() {
                    String::new()
                } else {
                    format!(" {}", record.gate)
                };
                format!(
                    "{}{gate}: {} (set by {}, expires {})",
                    record.environment,
                    record.reason,
                    record.owner,
                    record.expiry.format("%Y-%m-%d")
                )
            }
        }
    }
}

/// A store that exists but could not be walked or read. It blocks the push:
/// the records it hides might bind.
#[derive(Debug)]
pub struct StoreError {
    pub path: PathBuf,
    pub source: io::Error,
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot read the constraint store at {}: {}",
            self.path.display(),
            self.source
        )
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Every record under `directory`, recursively, ordered by path component.
///
/// A missing `directory`, or one that is not a directory, holds no records.
/// Every entry named `*.json` (ASCII case-insensitively on Windows, matching
/// its filesystem) is a record; symlinked directories are not descended.
///
/// # Errors
///
/// A traversal failure or an entry that cannot be read. A record that is read
/// but invalid is not an error: it is returned malformed.
pub fn load(directory: &Path) -> Result<Vec<Constraint>, StoreError> {
    match fs::metadata(directory) {
        Err(error) if matches!(error.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory) => {
            return Ok(Vec::new());
        }
        Err(source) => {
            return Err(StoreError {
                path: directory.to_path_buf(),
                source,
            });
        }
        Ok(metadata) if !metadata.is_dir() => return Ok(Vec::new()),
        Ok(_) => {}
    }
    let mut paths = Vec::new();
    collect_record_paths(directory, &mut paths)?;
    paths.sort_by_cached_key(|path| sort_key(directory, path));
    paths
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).map_err(|source| StoreError {
                path: path.clone(),
                source,
            })?;
            let record = parse_constraint_record(&bytes, &path.display().to_string());
            Ok(Constraint { path, record })
        })
        .collect()
}

fn collect_record_paths(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<(), StoreError> {
    let failed = |path: &Path| {
        let path = path.to_path_buf();
        move |source| StoreError { path, source }
    };
    for entry in fs::read_dir(directory).map_err(failed(directory))? {
        let entry = entry.map_err(failed(directory))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(failed(&path))?;
        if is_record_name(&entry.file_name()) {
            paths.push(path.clone());
        }
        if file_type.is_dir() {
            collect_record_paths(&path, paths)?;
        }
    }
    Ok(())
}

fn is_record_name(name: &OsStr) -> bool {
    let bytes = name.as_encoded_bytes();
    let Some(suffix) = bytes.len().checked_sub(5).map(|start| &bytes[start..]) else {
        return false;
    };
    if cfg!(windows) {
        suffix.eq_ignore_ascii_case(b".json")
    } else {
        suffix == b".json"
    }
}

/// Path components below the store root, compared by code point (bytes on
/// Unix); Windows compares them lowercased, as Python's `PureWindowsPath`
/// ordering does. Every walked path is under `root`, so the `unwrap_or` only
/// keeps an impossible case total.
fn sort_key(root: &Path, path: &Path) -> Vec<OsString> {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| {
            let name = component.as_os_str();
            if cfg!(windows) {
                OsString::from(name.to_string_lossy().to_lowercase())
            } else {
                name.to_os_string()
            }
        })
        .collect()
}

/// A store's records split by whether they bind this push.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Selection {
    pub active: Vec<Constraint>,
    /// Records that apply but expired before today; reported, not binding.
    pub expired: Vec<Constraint>,
    /// Records that bind whatever the repository, branch, date, or plan.
    pub malformed: Vec<Constraint>,
}

/// Splits `constraints` for a push to `repository` under `branches` on
/// `today`. A valid record that does not apply is dropped.
pub fn select<S: AsRef<str>>(
    constraints: Vec<Constraint>,
    repository: &str,
    branches: &[S],
    today: NaiveDate,
) -> Selection {
    let mut selection = Selection::default();
    for constraint in constraints {
        if constraint.is_malformed() {
            selection.malformed.push(constraint);
        } else if !constraint.applies_to(repository, branches) {
            continue;
        } else if constraint.expired(today) {
            selection.expired.push(constraint);
        } else {
            selection.active.push(constraint);
        }
    }
    selection
}

/// The cells of the resolved plan at `path`.
///
/// # Errors
///
/// A plan that cannot be read or is not a valid plan view. It blocks the push
/// even when the store is empty: an unreadable plan proves nothing.
pub fn read_plan(path: &Path) -> Result<Vec<ConstraintCell>, DocumentError> {
    let document = path.display().to_string();
    let bytes = fs::read(path).map_err(|error| DocumentError {
        document: document.clone(),
        location: String::new(),
        problem: format!("cannot be read: {error}"),
    })?;
    parse_constraint_plan(&bytes, &document)
}

/// The constraints `cells` does not already satisfy.
///
/// A valid constraint is violated by a cell in its environment (and its gate,
/// when it names one) that executes or that the planner already resolved to
/// `prohibited`; a reused, accepted-gap, or absent cell satisfies it. A
/// malformed constraint is never satisfied, and with no plan (`None`) nothing
/// is.
pub fn unsatisfied<'a>(constraints: &'a [Constraint], cells: Option<&[ConstraintCell]>) -> Vec<&'a Constraint> {
    let Some(cells) = cells else {
        return constraints.iter().collect();
    };
    let executing: Vec<&ConstraintCell> = cells
        .iter()
        .filter(|cell| cell.execution == "execute" || cell.state.as_deref() == Some("prohibited"))
        .collect();
    constraints
        .iter()
        .filter(|constraint| match &constraint.record {
            Err(_) => true,
            Ok(record) => executing
                .iter()
                .any(|cell| cell.environment == record.environment && (record.gate.is_empty() || cell.gate == record.gate)),
        })
        .collect()
}
