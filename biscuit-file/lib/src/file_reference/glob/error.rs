use std::path::PathBuf;

use crate::file_reference::{FileReferenceError, ResolutionFailure};

/// Why a [`GlobReference`](super::GlobReference) could not be built or
/// resolved.
///
/// Every variant that concerns one pattern names it as authored, `!`
/// included, so a consumer can report the pattern without re-deriving it.
#[derive(Debug, thiserror::Error)]
pub enum GlobReferenceError {
    /// The pattern uses a prefix a glob never accepts: a remote URL (nothing
    /// local to walk) or the recursive `%` modifier (a glob is already
    /// recursive).
    #[error("glob reference `{pattern}` cannot use {prefix}: {reason}")]
    RejectedPrefix {
        pattern: String,
        prefix: &'static str,
        reason: &'static str,
    },

    /// The reference grammar rejected the pattern's prefix or text, as it
    /// would a single-file reference with the same text.
    #[error("glob reference `{pattern}` is malformed: {source}")]
    MalformedPrefix {
        pattern: String,
        #[source]
        source: FileReferenceError,
    },

    /// The text after the prefix is not a valid glob.
    #[error("glob reference `{pattern}` is not a valid glob: {message}")]
    InvalidGlob { pattern: String, message: String },

    /// Every pattern is a `!` exclusion, so nothing could ever match.
    #[error("a glob reference needs at least one pattern that is not a `!` exclusion")]
    NoPositivePattern,

    /// A bare, `./`, or `../` pattern would search outside the file tree.
    #[error(
        "glob reference `{pattern}` leaves file tree `{base_dir}` through `{candidate}`"
    )]
    RelativeTreeEscape {
        pattern: String,
        base_dir: PathBuf,
        candidate: PathBuf,
    },

    /// A `&` or `^` pattern was resolved where there is no repository.
    #[error(
        "glob reference `{pattern}` uses `{sigil}`, which needs a repository containing `{reference_cwd}`"
    )]
    OutsideRepository {
        pattern: String,
        sigil: char,
        reference_cwd: PathBuf,
    },

    /// The pattern's roots could not be resolved in this context: a missing
    /// home, vault, or environment variable, a repository escape, or an
    /// interpolation that injected a sigil.
    #[error("glob reference `{pattern}` cannot be resolved: {source}")]
    Unresolvable {
        pattern: String,
        #[source]
        source: FileReferenceError,
    },

    /// The resolution context itself failed validation.
    #[error("the file resolution context is invalid: {0}")]
    InvalidContext(#[source] FileReferenceError),

    /// A search root exists but could not be read.
    #[error("filesystem error while searching `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl GlobReferenceError {
    /// The [`ResolutionFailure`] class of this error, the same class a
    /// [`FileReference`](crate::FileReference) failure of the same kind
    /// reports.
    #[must_use]
    pub fn resolution_failure(&self) -> ResolutionFailure {
        match self {
            Self::RejectedPrefix { prefix, .. } if *prefix == REMOTE_PREFIX => {
                ResolutionFailure::UnsupportedRemote
            }
            Self::RejectedPrefix { .. }
            | Self::InvalidGlob { .. }
            | Self::NoPositivePattern
            | Self::RelativeTreeEscape { .. } => ResolutionFailure::InvalidReference,
            Self::OutsideRepository { .. } => ResolutionFailure::MissingContext,
            Self::MalformedPrefix { source, .. }
            | Self::Unresolvable { source, .. }
            | Self::InvalidContext(source) => source.resolution_failure(),
            Self::Io { .. } => ResolutionFailure::Io,
        }
    }

    /// Attach `pattern` to a resolution error, giving the tree-escape and
    /// missing-repository failures their own variants.
    pub(crate) fn from_reference(pattern: &str, source: FileReferenceError) -> Self {
        match source {
            FileReferenceError::RelativeTreeEscape {
                base_dir,
                candidate,
                ..
            } => Self::RelativeTreeEscape {
                pattern: pattern.to_string(),
                base_dir,
                candidate,
            },
            FileReferenceError::OutsideRepository {
                sigil,
                reference_cwd,
            } => Self::OutsideRepository {
                pattern: pattern.to_string(),
                sigil,
                reference_cwd,
            },
            source => Self::Unresolvable {
                pattern: pattern.to_string(),
                source,
            },
        }
    }

    /// The single-file error a recursive (`%`) reference reports for this
    /// failure. The `%` search builds its glob itself, so only resolution
    /// failures reach here.
    pub(crate) fn into_reference_error(self) -> FileReferenceError {
        match self {
            Self::RelativeTreeEscape {
                pattern,
                base_dir,
                candidate,
            } => FileReferenceError::RelativeTreeEscape {
                base_dir,
                candidate,
                reference: pattern,
            },
            Self::OutsideRepository {
                sigil,
                reference_cwd,
                ..
            } => FileReferenceError::OutsideRepository {
                sigil,
                reference_cwd,
            },
            Self::MalformedPrefix { source, .. }
            | Self::Unresolvable { source, .. }
            | Self::InvalidContext(source) => source,
            Self::Io { path, source } => FileReferenceError::Io { path, source },
            other @ (Self::RejectedPrefix { .. }
            | Self::InvalidGlob { .. }
            | Self::NoPositivePattern) => FileReferenceError::InvalidSyntax(other.to_string()),
        }
    }
}

/// The `prefix` a [`GlobReferenceError::RejectedPrefix`] names for a remote
/// URL.
pub(crate) const REMOTE_PREFIX: &str = "a remote URL";

/// The `prefix` a [`GlobReferenceError::RejectedPrefix`] names for `%`.
pub(crate) const RECURSIVE_PREFIX: &str = "the `%` recursive modifier";
