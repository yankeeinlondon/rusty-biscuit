//! Error types for the harness module.

use std::path::{Path, PathBuf};

use biscuit_file::{
    DetailedResolution, FileReferenceError, FileReferenceKind, ProbeDisposition, ProbedCandidate,
    RootProvenance,
};
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

/// The structured diagnostic projection of a `biscuit-file` detailed resolution.
///
/// Carried on [`HarnessError::PathResolutionFailed`] so the ordered candidate
/// plan the shared resolver probed reaches `err.detail.*` and the rendered
/// report instead of being discarded by the convenience projection: its parsed
/// kind, the repository root it anchored on, and each attempted candidate's
/// provenance and probe disposition (spec §D8).
#[derive(Debug, Clone)]
pub struct ResolutionDetail {
    kind: FileReferenceKind,
    repository_root: Option<PathBuf>,
    candidates: Vec<ProbedCandidate>,
}

impl ResolutionDetail {
    /// Project the retained detail out of a shared [`DetailedResolution`].
    pub fn from_detailed(detailed: &DetailedResolution) -> Self {
        Self {
            kind: detailed.class().kind,
            repository_root: detailed.repository_root().map(Path::to_path_buf),
            candidates: detailed.candidates().to_vec(),
        }
    }

    /// The ordered candidates the resolver attempted, each with its disposition.
    pub fn candidates(&self) -> &[ProbedCandidate] {
        &self.candidates
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
        /// The shared resolver's retained candidate plan, when a filesystem
        /// probe ran. `None` for failures drawn before resolution
        /// ([`PathResolutionFailure::EmptyReference`],
        /// [`PathResolutionFailure::NoSourceParent`]). Boxed to keep the `Err`
        /// variant small, the same `clippy::result_large_err` trade the boxed
        /// fields below make.
        resolution: Option<Box<ResolutionDetail>>,
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

impl HarnessError {
    /// The ordered candidate plan a resolution failure attempted, when one was
    /// retained. Empty for every other arm and for a failure drawn before a
    /// filesystem probe. The renderer enumerates it as the report's "Tried:"
    /// list so a miss shows repository-then-source order, not just its winner.
    pub fn resolution_candidates(&self) -> &[ProbedCandidate] {
        match self {
            HarnessError::PathResolutionFailed {
                resolution: Some(detail),
                ..
            } => detail.candidates(),
            _ => &[],
        }
    }
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
            // Seeded from the catalog so every declared key is present.
            // `reference`, `source_path`, and the typed `failure` are always
            // known; `kind`, `repository_root`, and the ordered `candidates`
            // are populated from the shared resolver's retained plan when a
            // probe ran, and stay `null` when the failure was drawn before
            // resolution (spec §D8). `failure` is the typed
            // `PathResolutionFailure`, never back-derived from `kind`.
            HarnessError::PathResolutionFailed {
                raw,
                failure,
                source_path,
                resolution,
                ..
            } => {
                let mut base = null_detail_for("composition.invalid_file_reference");
                base["reference"] = json!(raw);
                base["failure"] = json!(failure.as_str());
                base["source_path"] =
                    json!(source_path.as_ref().map(|p| p.to_string_lossy().into_owned()));
                if let Some(detail) = resolution {
                    base["kind"] = json!(file_reference_kind_slug(detail.kind));
                    base["repository_root"] = json!(
                        detail
                            .repository_root
                            .as_ref()
                            .map(|p| p.to_string_lossy().into_owned())
                    );
                    base["candidates"] = resolution_candidates_detail(&detail.candidates);
                }
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

/// Slug for a reference kind, drawn from the `kind` vocabulary the
/// `composition.invalid_file_reference` catalog entry declares.
fn file_reference_kind_slug(kind: FileReferenceKind) -> &'static str {
    use FileReferenceKind as K;
    match kind {
        K::ExplicitRelative => "explicit_relative",
        K::ImplicitRelative => "implicit_relative",
        K::Absolute => "absolute",
        K::Magic => "magic",
        K::Package => "package",
        K::Home => "home",
        K::Vault => "vault",
        K::Url => "url",
    }
}

/// Slug for the root a candidate was built from (spec §D3), so a handler reads
/// provenance as data rather than inferring it from a path prefix.
fn root_provenance_slug(provenance: RootProvenance) -> &'static str {
    use RootProvenance as P;
    match provenance {
        P::Repository => "repository",
        P::Source => "source",
        P::Package => "package",
        P::Home => "home",
        P::Magic => "magic",
        P::Vault => "vault",
        P::Absolute => "absolute",
    }
}

/// Slug for a candidate's probe disposition (spec §D8). `Io` collapses its
/// `ErrorKind` here; the kind is retained on the typed `FileReferenceError`.
fn probe_disposition_slug(disposition: ProbeDisposition) -> &'static str {
    use ProbeDisposition as D;
    match disposition {
        D::Missing => "missing",
        D::NonFile => "non_file",
        D::Matched => "matched",
        D::Io(_) => "io",
        D::SearchRoot => "search_root",
    }
}

/// Project the ordered probe record into the `candidates` detail array: one
/// object per attempt carrying its path, root provenance, and probe
/// disposition, in first-seen (repository-then-source) order.
fn resolution_candidates_detail(candidates: &[ProbedCandidate]) -> Value {
    Value::Array(
        candidates
            .iter()
            .map(|probed| {
                json!({
                    "path": probed.candidate().path().to_string_lossy(),
                    "provenance": root_provenance_slug(probed.candidate().provenance()),
                    "disposition": probe_disposition_slug(probed.disposition()),
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests;
