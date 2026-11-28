//! Configuration diagnostics: the codes, the located problems, and the error
//! a load or a setting accessor returns.

use std::fmt;
use std::path::{Path, PathBuf};

/// A configuration file could not be loaded, or one of its settings is
/// invalid.
pub const CONFIG_INVALID_CODE: &str = "dm.config.invalid";
/// A `permissions` file reference is malformed, remote, or (once followed)
/// names a missing or invalid target.
pub const CONFIG_REFERENCE_CODE: &str = "dm.config.reference";
/// An approval could not be saved to configuration.
pub const CONFIG_NOT_SAVED_CODE: &str = "dm.config.not_saved";
/// A temporary conversion of a legacy policy or configuration file.
pub const CONFIG_MIGRATION_CODE: &str = "dm.config.migration";
/// The user configuration file does not exist, and was not created; defaults
/// apply.
pub const CONFIG_MISSING_CODE: &str = "dm.config.missing";
/// A writing CLI's startup load created the missing user configuration file
/// with every setting at its default.
pub const CONFIG_CREATED_CODE: &str = "dm.config.created";

/// Which observation of a configuration file a problem was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigRevision {
    /// `~/.darkmatter.yaml`.
    User,
    /// `{repo-root}/.darkmatter.yaml` as committed at the captured `HEAD`.
    RepositoryHead,
    /// `{repo-root}/.darkmatter.yaml` in the working copy.
    RepositoryWorktree,
}

impl fmt::Display for ConfigRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::User => "user configuration",
            Self::RepositoryHead => "repository configuration at HEAD",
            Self::RepositoryWorktree => "repository configuration",
        })
    }
}

/// One located finding in a configuration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigProblem {
    /// The file the problem is in. For [`ConfigRevision::RepositoryHead`] it
    /// is the working-copy path of the committed file.
    pub file: PathBuf,
    /// Which observation of `file`.
    pub revision: ConfigRevision,
    /// The JSON pointer of the offending value (`/permissions/file-read/2`);
    /// empty for the whole document.
    pub pointer: String,
    /// 1-based line, when the problem has a position in the text.
    pub line: Option<usize>,
    /// 1-based column in characters, when `line` is known.
    pub column: Option<usize>,
    /// What is wrong.
    pub message: String,
    /// For a problem with a `permissions` reference that names a file, that
    /// file, as resolved (or as written when it could not be resolved): a
    /// host that reports that file's own problems can tell this one follows
    /// from them. `None` for every other problem.
    pub reference_target: Option<PathBuf>,
}

impl fmt::Display for ConfigProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file.display())?;
        if self.revision == ConfigRevision::RepositoryHead {
            f.write_str(" (HEAD)")?;
        }
        if let (Some(line), Some(column)) = (self.line, self.column) {
            write!(f, ":{line}:{column}")?;
        }
        if !self.pointer.is_empty() {
            write!(f, " at `{}`", self.pointer)?;
        }
        write!(f, ": {}", self.message)
    }
}

/// What kind of failure a [`ConfigError`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigErrorKind {
    /// The text is not one admissible YAML document (malformed, duplicate or
    /// non-string keys, several documents, an inadmissible number, invalid
    /// UTF-8). Stops every consumer of the file.
    Syntax,
    /// The document is not a mapping (an empty file included). Stops every
    /// consumer of the file.
    NotAMapping,
    /// The document has a key the schema does not declare. Stops every
    /// consumer of the file.
    UnknownKey,
    /// One setting's value is invalid; only consumers of that setting stop.
    InvalidSetting,
    /// A `permissions` file reference is blank, malformed, remote, or leaves
    /// the repository.
    InvalidReference,
    /// The context names a repository but no committed-tree snapshot was
    /// supplied, so the repository's committed policy cannot be read.
    RepositorySnapshotRequired,
    /// A file or the committed tree could not be read.
    Read,
    /// A legacy policy file could not be migrated (an unreadable file, a line
    /// its grammar rejects, an unfinishable migration receipt); nothing was
    /// written and every legacy file is kept.
    Migration,
}

/// A configuration failure: what kind it is and every located problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    kind: ConfigErrorKind,
    problems: Vec<ConfigProblem>,
}

impl ConfigError {
    pub(crate) fn new(kind: ConfigErrorKind, problems: Vec<ConfigProblem>) -> Self {
        Self { kind, problems }
    }

    /// This error, its problems naming `target` as the file their
    /// `permissions` reference names.
    pub(crate) fn with_reference_target(mut self, target: &Path) -> Self {
        for problem in &mut self.problems {
            problem.reference_target = Some(target.to_path_buf());
        }
        self
    }

    pub(crate) fn one(kind: ConfigErrorKind, problem: ConfigProblem) -> Self {
        Self::new(kind, vec![problem])
    }

    pub(crate) fn unlocated(kind: ConfigErrorKind, file: &Path, revision: ConfigRevision, message: String) -> Self {
        Self::one(
            kind,
            ConfigProblem {
                file: file.to_path_buf(),
                revision,
                pointer: String::new(),
                line: None,
                column: None,
                message,
                reference_target: None,
            },
        )
    }

    #[allow(missing_docs)]
    #[must_use]
    pub const fn kind(&self) -> ConfigErrorKind {
        self.kind
    }

    /// The diagnostic code: [`CONFIG_REFERENCE_CODE`] for a reference,
    /// [`CONFIG_MIGRATION_CODE`] for a migration, otherwise
    /// [`CONFIG_INVALID_CODE`].
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self.kind {
            ConfigErrorKind::InvalidReference => CONFIG_REFERENCE_CODE,
            ConfigErrorKind::Migration => CONFIG_MIGRATION_CODE,
            _ => CONFIG_INVALID_CODE,
        }
    }

    /// Every problem, in document order; never empty.
    #[must_use]
    pub fn problems(&self) -> &[ConfigProblem] {
        &self.problems
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut problems = self.problems.iter();
        if let Some(first) = problems.next() {
            write!(f, "{first}")?;
        }
        for problem in problems {
            write!(f, "; {problem}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigError {}

/// A finding a load records without failing: the snapshot carries it for
/// callers to report through their own result channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    /// The diagnostic code (`dm.config.missing`, `dm.config.invalid`, …).
    pub code: &'static str,
    /// The file it concerns.
    pub file: PathBuf,
    /// What happened, for a reader.
    pub message: String,
    /// The located problems behind it; empty for a file-level notice.
    pub problems: Vec<ConfigProblem>,
}
