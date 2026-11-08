//! The user-level grant store: its location, parser, and writer.

use std::collections::HashSet;
use std::fmt;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use super::{Grant, GrantDecision, GrantScope, GrantTarget};
use crate::markdown::schemas::slots::codes::SLOT_GRANT_STORE_INVALID_CODE;

/// The store's location below the user's home, as path components.
pub const GRANT_STORE_RELATIVE_PATH: [&str; 3] = [".config", "darkmatter", "schema-read-policy"];

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// The persisted grants, and the file they came from.
///
/// Every grant has [`GrantScope::Persisted`]; identical lines are kept once,
/// in first-seen order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrantStore {
    path: Option<PathBuf>,
    grants: Vec<Grant>,
}

impl GrantStore {
    /// The store file for `home`, or `None` when there is no home (and so no
    /// store). Pass the request's `FileResolutionContext::home_dir()`.
    pub fn path_for_home(home: Option<&Path>) -> Option<PathBuf> {
        let mut path = home?.to_path_buf();
        path.extend(GRANT_STORE_RELATIVE_PATH);
        Some(path)
    }

    /// A store with no grants and no file; nothing can be appended to it.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Reads the store at `path`. `None` (no home) is an empty store with no
    /// file; a missing or empty file is an empty store that can be appended to.
    ///
    /// Stored `file` and `dir` paths are canonicalized when they exist and kept
    /// as written otherwise.
    ///
    /// ## Errors
    ///
    /// A [`GrantStoreError`] when the file exists but cannot be read, is not
    /// UTF-8, or has any invalid line. The whole file then grants nothing:
    /// skipping a line could drop a deny.
    pub fn load(path: Option<&Path>) -> Result<Self, GrantStoreError> {
        let Some(path) = path else {
            return Ok(Self::empty());
        };
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                return Err(GrantStoreError {
                    path: path.to_path_buf(),
                    line: 0,
                    message: format!("cannot read the grant store: {error}"),
                    kind: GrantStoreErrorKind::Unreadable,
                });
            }
        };
        let grants = parse(path, &bytes)?;
        Ok(Self { path: Some(path.to_path_buf()), grants })
    }

    /// The persisted grants.
    pub fn grants(&self) -> &[Grant] {
        &self.grants
    }

    /// The store file, `None` when there is no home.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Adds a persisted grant and writes it to the store file.
    ///
    /// The parent folder is created on first write. The file is replaced, not
    /// appended to in place: its current bytes plus the new line (after a
    /// newline, when the file lacks a final one) go to a temporary file in the
    /// same folder that is renamed over the store, so a reader sees the old
    /// or the new file, never a partial one. A grant already in the store is a
    /// no-op. `target` should already be canonical.
    ///
    /// ## Errors
    ///
    /// [`GrantStoreWriteError::NoStore`] when the store has no file,
    /// [`GrantStoreWriteError::Unrepresentable`] when the target cannot be
    /// written as one line that reads back as the same target, and
    /// [`GrantStoreWriteError::Io`] when the file cannot be read or written.
    pub fn append(&mut self, decision: GrantDecision, target: GrantTarget) -> Result<(), GrantStoreWriteError> {
        let Some(path) = self.path.clone() else {
            return Err(GrantStoreWriteError::NoStore);
        };
        let grant = Grant { decision, target, scope: GrantScope::Persisted };
        if self.grants.contains(&grant) {
            return Ok(());
        }
        let line = store_line(&grant)?;
        let io_error = |source| GrantStoreWriteError::Io { path: path.clone(), source };
        let mut contents = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(io_error(error)),
        };
        if !contents.is_empty() && !contents.ends_with(b"\n") {
            contents.push(b'\n');
        }
        contents.extend_from_slice(line.as_bytes());
        contents.push(b'\n');
        let parent = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        std::fs::create_dir_all(parent).map_err(io_error)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
        temp.write_all(&contents).map_err(io_error)?;
        temp.persist(&path).map_err(|error| io_error(error.error))?;
        self.grants.push(grant);
        Ok(())
    }
}

/// The store line for `grant`, refused when reading it back would not give
/// the same target (a path with a line break, surrounding whitespace, or
/// bytes that are not UTF-8).
fn store_line(grant: &Grant) -> Result<String, GrantStoreWriteError> {
    let unrepresentable = |reason: &str| GrantStoreWriteError::Unrepresentable {
        target: grant.target.to_string(),
        reason: reason.to_string(),
    };
    if let GrantTarget::File(path) | GrantTarget::Dir(path) = &grant.target {
        let Some(text) = path.to_str() else {
            return Err(unrepresentable("the path is not valid UTF-8"));
        };
        if text.contains(['\n', '\r']) {
            return Err(unrepresentable("the path contains a line break"));
        }
        if text.trim() != text {
            return Err(unrepresentable("the path starts or ends with whitespace"));
        }
        if !path.is_absolute() {
            return Err(unrepresentable("the path is not absolute"));
        }
    }
    Ok(format!("{} {} {}", grant.decision.word(), grant.target.kind_word(), grant.target))
}

fn parse(path: &Path, bytes: &[u8]) -> Result<Vec<Grant>, GrantStoreError> {
    let bytes = bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|error| {
        let line = bytes[..error.valid_up_to()].iter().filter(|byte| **byte == b'\n').count() + 1;
        GrantStoreError {
            path: path.to_path_buf(),
            line,
            message: "the grant store is not valid UTF-8".to_string(),
            kind: GrantStoreErrorKind::NotUtf8,
        }
    })?;
    let mut seen = HashSet::new();
    let mut grants = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let grant = parse_line(line).map_err(|message| GrantStoreError {
            path: path.to_path_buf(),
            line: index + 1,
            message,
            kind: GrantStoreErrorKind::InvalidLine,
        })?;
        if seen.insert(grant.clone()) {
            grants.push(grant);
        }
    }
    Ok(grants)
}

/// One trimmed, non-comment line.
fn parse_line(line: &str) -> Result<Grant, String> {
    let (verb, rest) = split_word(line);
    let decision = match verb {
        "allow" => GrantDecision::Allow,
        "deny" => GrantDecision::Deny,
        other => return Err(format!("unknown verb `{other}`; expected `allow` or `deny`")),
    };
    let (kind, argument) = split_word(rest);
    if kind.is_empty() {
        return Err(format!("`{verb}` needs a kind: file, dir, url, url-any-query, or site"));
    }
    let target = match kind {
        "file" | "dir" | "url" | "url-any-query" | "site" if argument.is_empty() => {
            return Err(format!("`{verb} {kind}` needs a target"));
        }
        "file" => GrantTarget::File(stored_path(argument)?),
        "dir" => GrantTarget::Dir(stored_path(argument)?),
        "url" => GrantTarget::Url(http_url(argument)?),
        "url-any-query" => {
            let url = http_url(argument)?;
            if url.query().is_some() {
                return Err(format!("`{argument}` has a query; a `url-any-query` grant is written without one"));
            }
            GrantTarget::UrlAnyQuery(url)
        }
        "site" => site(argument)?,
        other => {
            return Err(format!("unknown kind `{other}`; expected file, dir, url, url-any-query, or site"));
        }
    };
    Ok(Grant { decision, target, scope: GrantScope::Persisted })
}

/// The first whitespace-separated word and the trimmed rest.
fn split_word(text: &str) -> (&str, &str) {
    match text.split_once(char::is_whitespace) {
        Some((word, rest)) => (word, rest.trim()),
        None => (text, ""),
    }
}

fn stored_path(argument: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(argument);
    if !path.is_absolute() {
        return Err(format!("`{argument}` is not an absolute path"));
    }
    // A grant for a path that does not exist (yet) keeps its written form;
    // only an existing path has a canonical one to compare by.
    match biscuit_file::canonicalize_simplified(&path) {
        Ok(canonical) => Ok(canonical),
        Err(_) => Ok(path),
    }
}

fn http_url(argument: &str) -> Result<url::Url, String> {
    let url = url::Url::parse(argument).map_err(|error| format!("`{argument}` is not a URL: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("`{argument}` is not an http or https URL"));
    }
    if url.fragment().is_some() {
        return Err(format!("`{argument}` has a fragment; a grant URL has none"));
    }
    Ok(url)
}

fn site(argument: &str) -> Result<GrantTarget, String> {
    let url = http_url(argument)?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(format!("`{argument}` has user information; a site is `scheme://host[:port]`"));
    }
    if url.path() != "/" || url.query().is_some() {
        return Err(format!("`{argument}` has a path or query; a site is `scheme://host[:port]`"));
    }
    let Some(host) = url.host_str() else {
        return Err(format!("`{argument}` has no host"));
    };
    Ok(GrantTarget::Site { scheme: url.scheme().to_string(), host: host.to_string(), port: url.port() })
}

/// Why the grant store could not be loaded. The whole store then grants
/// nothing.
///
/// `line` is 1-based for [`GrantStoreErrorKind::InvalidLine`] and for
/// [`GrantStoreErrorKind::NotUtf8`] (the line holding the first invalid
/// byte), and `0` for [`GrantStoreErrorKind::Unreadable`], which concerns the
/// whole file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantStoreError {
    /// The store file.
    pub path: PathBuf,
    /// The 1-based line, or `0` for the whole file.
    pub line: usize,
    /// What is wrong, for a person.
    pub message: String,
    /// The failure's shape.
    pub kind: GrantStoreErrorKind,
}

impl GrantStoreError {
    /// The diagnostic code, the same for every kind.
    pub fn code(&self) -> &'static str {
        SLOT_GRANT_STORE_INVALID_CODE
    }
}

impl fmt::Display for GrantStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = biscuit_file::to_portable_string(&self.path);
        match self.line {
            0 => write!(f, "{path}: {}", self.message),
            line => write!(f, "{path}:{line}: {}", self.message),
        }
    }
}

impl std::error::Error for GrantStoreError {}

/// The shape of a [`GrantStoreError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrantStoreErrorKind {
    /// A line is not a grant.
    InvalidLine,
    /// The file exists but cannot be read.
    Unreadable,
    /// The file is not UTF-8.
    NotUtf8,
}

/// Why a grant could not be persisted.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GrantStoreWriteError {
    /// There is no home, so no store to write.
    #[error("there is no home folder, so grants cannot be persisted")]
    NoStore,
    /// The target cannot be written as a line that reads back as itself.
    #[error("cannot persist a grant for `{target}`: {reason}")]
    Unrepresentable {
        /// The target, as displayed.
        target: String,
        /// Why it cannot be written.
        reason: String,
    },
    /// Reading or replacing the store file failed.
    #[error("cannot write the grant store {}: {source}", path.display())]
    Io {
        /// The store file.
        path: PathBuf,
        /// The underlying failure.
        #[source]
        source: io::Error,
    },
}
