//! Error types for the harness module.

use std::path::PathBuf;

use biscuit_file::FileReferenceError;
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::compose::shell_expansion::ShellExpansionError;
use serde_json::{Value, json};

use crate::diagnostics::{Category, Diagnostic, Disposition, Origin, code_spec, null_detail_for};

/// Why a harness path reference could not be resolved to a usable file.
///
/// The three arms are the resolver's *own* distinctions, each taken at a
/// different stage: the reference is rejected before resolution
/// ([`EmptyReference`]), the anchor it needs is unavailable
/// ([`NoSourceParent`]), or resolution succeeded and the probe found nothing
/// ([`TargetMissing`]). Because the resolver draws these lines itself, they
/// project to `err.detail.failure` — unlike Darkmatter's `FileRefFailure`,
/// which folds permission and missing-context failures into `NotFound` and so
/// cannot honestly claim `no_match` (see the feature's `decisions.md` §D-5).
///
/// [`EmptyReference`]: PathResolutionFailure::EmptyReference
/// [`NoSourceParent`]: PathResolutionFailure::NoSourceParent
/// [`TargetMissing`]: PathResolutionFailure::TargetMissing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathResolutionFailure {
    /// The reference was empty or whitespace-only.
    EmptyReference,
    /// The source document has no parent directory to anchor a relative
    /// reference against.
    NoSourceParent,
    /// The reference resolved, but nothing exists at the resolved path.
    TargetMissing,
}

impl PathResolutionFailure {
    /// Stable snake_case slug, drawn from the `failure` vocabulary the
    /// `composition.invalid_file_reference` catalog entry declares.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyReference => "invalid_syntax",
            Self::NoSourceParent => "missing_context",
            Self::TargetMissing => "no_match",
        }
    }
}

/// The `Display` tail for [`HarnessError::PathResolutionFailed`].
///
/// Kept out of the `#[error]` attribute because each arm reads a different
/// optional field.
fn path_resolution_detail(
    failure: PathResolutionFailure,
    source_path: Option<&PathBuf>,
    resolved: Option<&PathBuf>,
) -> String {
    match failure {
        PathResolutionFailure::EmptyReference => "path is empty".to_string(),
        PathResolutionFailure::NoSourceParent => match source_path {
            Some(path) => format!("source path \"{}\" has no parent directory", path.display()),
            None => "source path has no parent directory".to_string(),
        },
        PathResolutionFailure::TargetMissing => match resolved {
            Some(path) => format!("target does not exist: {}", path.display()),
            None => "target does not exist".to_string(),
        },
    }
}

/// Why a shell command's execution failed.
///
/// The four arms are the four failure modes of
/// [`crate::harness::shell::execute_approved_command`], each carrying the typed
/// error that stage raised. No arm takes `#[from]`: the arm *is* the stage, and
/// `Spawn` and `Wait` both hold an `io::Error` that cannot say which of the two
/// produced it, so every construction names its stage explicitly.
#[derive(Debug, thiserror::Error)]
pub enum ShellExecCause {
    /// The executable was not found on `PATH`.
    #[error(transparent)]
    Which(which::Error),

    /// The child process could not be spawned.
    #[error(transparent)]
    Spawn(std::io::Error),

    /// Waiting on the spawned child failed.
    #[error(transparent)]
    Wait(std::io::Error),

    /// The command exceeded its timeout and was killed.
    #[error(transparent)]
    Timeout(tokio::time::error::Elapsed),
}

/// All errors that can occur within the harness subsystem.
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    // --- Parse / configuration ---
    /// Frontmatter property is structurally invalid or has an unexpected type.
    #[error("{source_path}: invalid `{property}` frontmatter property: {detail}")]
    InvalidFrontmatter {
        source_path: PathBuf,
        property: String,
        detail: String,
    },

    /// A timeout string could not be parsed.
    #[error("{source_path}: invalid timeout \"{raw}\": {detail}")]
    InvalidTimeout {
        source_path: PathBuf,
        raw: String,
        detail: String,
    },

    /// A shell command failed during execution.
    #[error("shell command execution failed: {detail}")]
    ShellCommandExecutionFailed {
        detail: String,
        /// Which stage failed, and the typed error it raised.
        #[source]
        source: ShellExecCause,
    },

    // --- Shell approval failures ---
    /// A shell command was denied by the approval system.
    #[error("shell command denied: {command}")]
    ShellCommandDenied { command: String },

    /// A shell command matched the blacklist.
    #[error("shell command blacklisted: \"{command}\" — {reason}")]
    ShellCommandBlacklisted { command: String, reason: String },

    // --- Path resolution ---
    /// A `@`-prefixed path requires a repo root, but none was available.
    #[error("repo root required to resolve path \"{path}\"")]
    RepoRootRequired { path: String },

    /// Path resolution failed for another reason.
    #[error(
        "path resolution failed for \"{raw}\": {}",
        path_resolution_detail(*failure, source_path.as_ref(), resolved.as_ref())
    )]
    PathResolutionFailed {
        /// The reference exactly as authored.
        raw: String,
        /// Which of the resolver's own distinctions was drawn.
        failure: PathResolutionFailure,
        /// The document the reference was anchored against, when known.
        source_path: Option<PathBuf>,
        /// The path the reference resolved to, when resolution got that far.
        resolved: Option<PathBuf>,
    },

    /// The shared `biscuit-file` resolver rejected a reference for a reason
    /// other than a plain missing target: invalid syntax, an absent context
    /// anchor (interpolation variable, home, repository), a repository
    /// containment violation, an I/O probe failure, or a remote URL. Carries
    /// the typed [`FileReferenceError`] cause.
    ///
    /// Boxed for the same `clippy::result_large_err` reason as
    /// [`ShellAuditParseError`], and with the same D-7 trade: the box publishes
    /// `Box<FileReferenceError>` to the cause chain, so recovery matches this
    /// variant on the concrete `HarnessError` rather than downcasting
    /// `Error::source()` to `FileReferenceError`.
    ///
    /// [`ShellAuditParseError`]: HarnessError::ShellAuditParseError
    #[error("could not resolve file reference \"{reference}\": {source}")]
    FileReferenceUnresolvable {
        /// The reference exactly as authored (trimmed).
        reference: String,
        /// The document the reference was anchored against, when known.
        source_path: Option<PathBuf>,
        /// The typed resolution failure from `biscuit-file`.
        #[source]
        source: Box<FileReferenceError>,
    },

    // --- Shell audit ---
    /// Failed to parse shell directives from source page during audit.
    #[error("shell audit parse error: {detail}")]
    ShellAuditParseError {
        detail: String,
        /// Darkmatter's structured directive-parse failure.
        ///
        /// Boxed, mirroring the otherwise-identical
        /// `CompositionError::ShellExpansionFailed`, and for the same reason:
        /// `ShellExpansionError` is ~160 bytes, and `CompositionError` holds a
        /// `HarnessError` unboxed, so an unboxed field here trips
        /// `clippy::result_large_err` on 376 call sites across both enums.
        ///
        /// The cost is the D-7 trap: a `Box<T>` publishes `Box<T>` to the cause
        /// chain, so `Error::source()` cannot be downcast to
        /// `ShellExpansionError` and a chain walk skips it. Recovery is by
        /// matching this variant on the concrete `HarnessError` instead. No
        /// discovery seam is harmed — `ShellExpansionError` is a Darkmatter
        /// type, not a registered Claudine `Diagnostic`, so `as_diagnostic`
        /// never needed to downcast it.
        #[source]
        source: Box<ShellExpansionError>,
    },
}

impl BlockError for HarnessError {
    fn status_block(&self, _term: &Terminal) -> StatusBlock {
        StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("HarnessError", self.code()))
            .body(self.to_string())
    }
}

impl Diagnostic for HarnessError {
    fn code(&self) -> &'static str {
        match self {
            // Author-authored harness frontmatter (timeouts, typed properties).
            HarnessError::InvalidFrontmatter { .. } | HarnessError::InvalidTimeout { .. } => {
                "composition.lifecycle_invalid"
            }
            // Shell directives the author wrote in the prompt document.
            HarnessError::ShellCommandExecutionFailed { .. }
            | HarnessError::ShellCommandDenied { .. }
            | HarnessError::ShellCommandBlacklisted { .. }
            | HarnessError::ShellAuditParseError { .. } => "composition.shell_expansion",
            // A reference the author wrote could not be resolved. This is an
            // authoring mistake in the prompt document, not an environment
            // failure: the file the run wanted to read is named by a value the
            // author typed. Classifying it `io.read_failed` (`Category::Io` /
            // `Origin::Operator`) sent the reader to check the filesystem when
            // the fix is in their frontmatter.
            HarnessError::PathResolutionFailed { .. }
            | HarnessError::FileReferenceUnresolvable { .. } => {
                "composition.invalid_file_reference"
            }
            // Still environmental: the run needed a repo root and the process
            // was launched outside one.
            HarnessError::RepoRootRequired { .. } => "io.read_failed",
        }
    }

    fn category(&self) -> Category {
        code_spec(self.code())
            .map(|spec| spec.category)
            .unwrap_or(Category::Composition)
    }

    fn disposition(&self) -> Disposition {
        code_spec(self.code())
            .map(|spec| spec.disposition)
            .unwrap_or(Disposition::Correctable)
    }

    fn origin(&self) -> Origin {
        code_spec(self.code())
            .map(|spec| spec.origin)
            .unwrap_or(Origin::Author)
    }

    fn detail(&self) -> Value {
        match self {
            HarnessError::InvalidFrontmatter {
                property, detail, ..
            } => json!({ "property": property, "message": detail }),
            HarnessError::InvalidTimeout { detail, .. } => {
                json!({ "property": "timeout", "message": detail })
            }
            HarnessError::ShellCommandExecutionFailed { detail, .. } => {
                json!({ "command": detail })
            }
            HarnessError::ShellCommandDenied { command }
            | HarnessError::ShellCommandBlacklisted { command, .. } => {
                json!({ "command": command })
            }
            HarnessError::ShellAuditParseError { detail, .. } => json!({ "command": detail }),
            HarnessError::RepoRootRequired { path } => json!({ "path": path }),
            // Seeded from the catalog so every declared key is present. Only
            // `reference`, `source_path`, and `failure` are things this
            // resolver actually knows; the rest stay `null` rather than being
            // invented (spec §D3). `failure` is populated from the typed
            // `PathResolutionFailure`, never back-derived from `kind` — which
            // is exactly why `kind` itself stays `null` here.
            HarnessError::PathResolutionFailed {
                raw,
                failure,
                source_path,
                ..
            } => {
                let mut base = null_detail_for("composition.invalid_file_reference");
                base["reference"] = json!(raw);
                base["failure"] = json!(failure.as_str());
                base["source_path"] =
                    json!(source_path.as_ref().map(|p| p.to_string_lossy().into_owned()));
                base
            }
            // Same projection as `PathResolutionFailed`, but `failure` is
            // mapped from the typed `FileReferenceError` rather than a resolver
            // distinction. Only the keys this adapter actually knows are
            // populated; the rest stay `null` (spec §D3).
            HarnessError::FileReferenceUnresolvable {
                reference,
                source_path,
                source,
            } => {
                let mut base = null_detail_for("composition.invalid_file_reference");
                base["reference"] = json!(reference);
                base["failure"] = json!(file_reference_failure_slug(source));
                base["source_path"] =
                    json!(source_path.as_ref().map(|p| p.to_string_lossy().into_owned()));
                base
            }
        }
    }
}

/// Map a `biscuit-file` resolution failure to the closed `failure` vocabulary
/// the `composition.invalid_file_reference` catalog entry declares.
///
/// The slug is derived from the typed error, never back-derived from the
/// reference kind. A future `FileReferenceError` variant defaults to
/// `invalid_syntax`.
fn file_reference_failure_slug(error: &FileReferenceError) -> &'static str {
    use FileReferenceError as E;
    match error {
        E::MissingEnvironmentVariable { .. }
        | E::MissingHomeContext
        | E::VaultNotConfigured
        | E::RepositoryRootNotContainingSource { .. }
        | E::BareRepository => "missing_context",
        E::RemoteNotLocal(_) => "unsupported_remote",
        E::CurrentDirectory(_)
        | E::Git(_)
        | E::Workspace(_)
        | E::RelativePath { .. }
        | E::Io { .. } => "permission_io",
        _ => "invalid_syntax",
    }
}

#[cfg(test)]
mod tests;
