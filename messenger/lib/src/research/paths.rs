//! Repository-relative path spelling and the fixed contract layout.

use std::fmt;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

use super::error::ResearchError;

/// A repository-relative path with `/` separators on every OS, so findings
/// and provenance never carry host-specific spelling.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct RepoPath(String);

impl RepoPath {
    /// Builds from already-portable text (for example a mapping input).
    pub fn from_portable(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RepoPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The repository whose `messenger/` area holds the research contract.
///
/// All contract paths derive from one explicit root; nothing here reads the
/// process working directory or probes Git.
#[derive(Debug, Clone)]
pub struct Workspace {
    repo_root: PathBuf,
}

impl Workspace {
    /// A workspace rooted at `repo_root` (made absolute against nothing: the
    /// caller must pass an absolute path).
    ///
    /// ## Errors
    ///
    /// Returns [`ResearchError::RelativeRoot`] for a relative root.
    pub fn new(repo_root: impl Into<PathBuf>) -> Result<Self, ResearchError> {
        let repo_root = repo_root.into();
        if !repo_root.is_absolute() {
            return Err(ResearchError::RelativeRoot { root: repo_root });
        }
        Ok(Self { repo_root: normalize(&repo_root) })
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// `messenger/docs/platforms.yaml`.
    pub fn roster(&self) -> PathBuf {
        self.join("messenger/docs/platforms.yaml")
    }

    /// `messenger/docs/platforms.schema.yaml`.
    pub fn roster_schema(&self) -> PathBuf {
        self.join("messenger/docs/platforms.schema.yaml")
    }

    /// `messenger/docs/research/platforms/`, home of accepted documents.
    pub fn documents_dir(&self) -> PathBuf {
        self.join("messenger/docs/research/platforms")
    }

    /// `messenger/docs/research/platforms/_schema.yaml`.
    pub fn document_schema(&self) -> PathBuf {
        self.join("messenger/docs/research/platforms/_schema.yaml")
    }

    /// `messenger/docs/research/platforms/_types.yaml`.
    pub fn types_schema(&self) -> PathBuf {
        self.join("messenger/docs/research/platforms/_types.yaml")
    }

    /// `messenger/docs/research/platforms/_overrides.schema.yaml`.
    pub fn overrides_schema(&self) -> PathBuf {
        self.join("messenger/docs/research/platforms/_overrides.schema.yaml")
    }

    /// `messenger/docs/research/platforms/_overrides.yaml` (optional).
    pub fn overrides(&self) -> PathBuf {
        self.join("messenger/docs/research/platforms/_overrides.yaml")
    }

    /// `messenger/docs/research/implementation/_schema.yaml`.
    pub fn mappings_schema(&self) -> PathBuf {
        self.join("messenger/docs/research/implementation/_schema.yaml")
    }

    /// `messenger/docs/research/implementation/mappings.yaml` (optional).
    pub fn mappings(&self) -> PathBuf {
        self.join("messenger/docs/research/implementation/mappings.yaml")
    }

    /// The repository-relative spelling of a path inside the workspace.
    ///
    /// ## Errors
    ///
    /// Returns [`ResearchError::OutsideWorkspace`] when `path` is not under
    /// the repository root after lexical normalization.
    pub fn repo_path(&self, path: &Path) -> Result<RepoPath, ResearchError> {
        let absolute = if path.is_absolute() {
            normalize(path)
        } else {
            normalize(&self.repo_root.join(path))
        };
        let relative = absolute
            .strip_prefix(&self.repo_root)
            .map_err(|_| ResearchError::OutsideWorkspace { path: path.to_path_buf() })?;
        let segments: Vec<String> = relative
            .components()
            .filter_map(|component| match component {
                Component::Normal(segment) => Some(segment.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();
        Ok(RepoPath(segments.join("/")))
    }

    /// Resolves a portable repository-relative path to a host path.
    pub fn resolve(&self, path: &RepoPath) -> PathBuf {
        self.join(path.as_str())
    }

    fn join(&self, portable: &str) -> PathBuf {
        portable
            .split('/')
            .fold(self.repo_root.clone(), |acc, segment| acc.join(segment))
    }
}

/// Lexically removes `.` and resolves `..` without touching the filesystem,
/// so a symlinked temporary directory keeps the spelling the caller used.
pub(crate) fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}
