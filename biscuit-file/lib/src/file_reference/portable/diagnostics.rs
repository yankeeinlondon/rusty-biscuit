//! Typed records of how a portable reference was chosen, or why none was.
//!
//! Every value here is data a caller can match on; `Display` exists only for
//! people. Errors are `Clone`, so filesystem failures are kept as path,
//! [`ErrorKind`], and OS code rather than as a [`std::io::Error`].

use std::fmt;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::strategy::PortabilityPreference;
use super::text::TextRejection;
use crate::file_reference::{FileReference, FileReferenceError, ResolutionFailure};

/// One preference that was tried, and what happened.
///
/// `rejected` followed by `outcome` lists every candidate the preference
/// considered, in order: a searched form can be shadowed from one root and
/// match from the next, and an environment anchor attempt records each
/// ineligible variable. When nothing matched, `outcome` is the last rejection.
#[derive(Debug, Clone)]
pub struct Attempt {
    pub strategy: PortabilityPreference,
    pub outcome: AttemptOutcome,
    pub rejected: Vec<AttemptOutcome>,
}

impl Attempt {
    /// Split `outcomes` (never empty in practice) into the earlier rejections
    /// and the final outcome.
    pub(crate) fn from_outcomes(strategy: PortabilityPreference, mut outcomes: Vec<AttemptOutcome>) -> Self {
        let outcome = outcomes
            .pop()
            .unwrap_or(AttemptOutcome::NotApplicable(NotApplicable::NoCandidate));
        Self {
            strategy,
            outcome,
            rejected: outcomes,
        }
    }

    /// The matched reference, when this attempt matched.
    pub fn matched(&self) -> Option<&FileReference> {
        match &self.outcome {
            AttemptOutcome::Matched(reference) => Some(reference),
            _ => None,
        }
    }
}

/// The result of one candidate.
#[derive(Debug, Clone)]
pub enum AttemptOutcome {
    /// The reference verified as naming the target.
    Matched(FileReference),
    /// The preference could not produce a reference for this target.
    NotApplicable(NotApplicable),
    /// It produced one, but that reference resolves somewhere else (for
    /// example `@x.md` found first under an earlier search root).
    Shadowed {
        reference: FileReference,
        resolves_to: PathBuf,
    },
}

/// Why a preference produced no reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotApplicable {
    /// `&` or `^` outside a repository.
    NoRepository,
    /// The target is outside the repository, or below none of the `^` roots.
    OutsideRepository,
    /// An in-tree relative preference for a target outside `base_dir`.
    OutsideBaseDir,
    /// [`ExternalRelativePath`](PortabilityPreference::ExternalRelativePath)
    /// for a target inside `base_dir`.
    InsideBaseDir,
    /// No lexical route from `cwd` to the target exists (another drive or
    /// share).
    NoSharedRoot,
    /// The route from `cwd` does not have this preference's shape.
    RouteShape { parent_hops: usize, names: usize },
    /// The target is not below the preference's eligibility filter.
    OutsideFilter { filter: PathBuf },
    /// A [`MagicPath`](PortabilityPreference::MagicPath) filter names a
    /// directory that is not one of the context's `@` search roots.
    FilterNotASearchRoot { filter: PathBuf },
    /// A [`MagicPath`](PortabilityPreference::MagicPath) filter could not be
    /// resolved in this context (for example `~/…` with no home directory).
    FilterUnavailable {
        filter: String,
        problem: ResolutionProblem,
    },
    /// The context has no home directory.
    HomeUnavailable,
    NotUnderHome,
    NotUnderMagicRoot,
    /// A search form (`@`, `^`) for a target that does not exist yet.
    TargetMissing,
    /// A search form (`@`, `^`) for a target that is not a regular file; the
    /// resolver only finds files.
    TargetNotFile,
    /// No portable environment variables are declared.
    NoPortableVariables,
    /// One declared variable cannot anchor this target.
    EnvAnchor {
        name: String,
        problem: EnvAnchorProblem,
    },
    /// [`AuthoredIntent`](PortabilityPreference::AuthoredIntent) on a path
    /// input.
    NotAReference,
    /// [`AuthoredIntent`](PortabilityPreference::AuthoredIntent) on a position
    /// form, or on an intent form outside its
    /// [`IntentForms`](super::IntentForms).
    PositionForm,
    /// A URL or recursive `%` input, which only `AuthoredIntent` keeps; no
    /// preference rewrites one.
    NotRewritable,
    /// The preference's spelling would not read back as the target's names.
    UnsafeSpelling(SpellingProblem),
    /// The generated reference failed verification in the context: it leaves
    /// the file tree (as written or where it lands), or an anchor is missing.
    Rejected(ResolutionProblem),
    /// The preference considered no candidate at all.
    NoCandidate,
}

/// Why a generated spelling was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellingProblem {
    /// A name is not valid Unicode.
    NotUnicode,
    /// A Windows device or unreducible verbatim path has no portable absolute
    /// spelling.
    NoPortableSpelling,
    /// The text would read back as different names: a Unix name containing
    /// `\`, a literal verbatim `.`/`..`, or a Windows name that changes
    /// meaning without its `\\?\` prefix.
    ChangesComponents,
    /// The text would not parse as the intended form: `{{…}}` or a leading
    /// sigil inside a name.
    GrammarMismatch,
}

impl From<TextRejection> for SpellingProblem {
    fn from(rejection: TextRejection) -> Self {
        match rejection {
            TextRejection::Unrenderable => Self::NotUnicode,
            TextRejection::NoPortableSpelling => Self::NoPortableSpelling,
            TextRejection::ChangesComponents => Self::ChangesComponents,
            TextRejection::GrammarMismatch => Self::GrammarMismatch,
        }
    }
}

/// Why a declared portable variable cannot anchor a target here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvAnchorProblem {
    Unset,
    /// A relative value, including the empty string.
    NotAbsolute { value: String },
    /// Absolute only on another OS (`C:\config` on macOS, `/opt` on Windows).
    ForeignAbsolute { value: String },
    /// A valid anchor that is not a whole-component prefix of the target.
    NotAPrefix { value: PathBuf },
}

/// Something worth reporting about the returned reference, most often one kept
/// as authored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// The leading portable variable of a kept reference is unusable here.
    PortableVariableUnusable {
        name: String,
        problem: EnvAnchorProblem,
    },
    /// A variable in a kept reference (`&{{PKG}}/…`) is not portable.
    NonPortableVariable { name: String },
    /// The reference names a file that does not exist.
    TargetMissing,
    /// The reference names a directory or other non-file; the resolver matches
    /// regular files.
    TargetNotFile,
    /// A declared portable name is not a valid `{{VAR}}` name; it was skipped.
    InvalidPortableVariableName { name: String },
    /// Looking the reference up failed for a reason other than absence.
    ResolutionFailed(ResolutionProblem),
}

/// A resolver failure, as `Clone` data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionProblem {
    /// A relative reference leaves the file tree, as written or where it lands.
    TreeEscape { base_dir: PathBuf, candidate: PathBuf },
    /// A `&`/`^` reference leaves the repository.
    RepositoryEscape {
        repository_root: PathBuf,
        candidate: PathBuf,
    },
    /// `&`/`^` used outside a repository.
    OutsideRepository,
    MissingEnvironmentVariable { name: String },
    MissingHome,
    VaultNotConfigured,
    /// A filesystem probe failed for a reason other than absence.
    Probe(ProbeError),
    /// Any other failure, with the resolver's classification and message.
    Other {
        failure: ResolutionFailure,
        message: String,
    },
}

impl ResolutionProblem {
    pub(crate) fn from_error(error: &FileReferenceError) -> Self {
        use FileReferenceError as E;
        match error {
            E::RelativeTreeEscape {
                base_dir, candidate, ..
            } => Self::TreeEscape {
                base_dir: base_dir.clone(),
                candidate: candidate.clone(),
            },
            E::RepositoryEscape {
                repository_root,
                escaped_candidate,
                ..
            } => Self::RepositoryEscape {
                repository_root: repository_root.clone(),
                candidate: escaped_candidate.clone(),
            },
            E::OutsideRepository { .. } => Self::OutsideRepository,
            E::MissingEnvironmentVariable { name } => {
                Self::MissingEnvironmentVariable { name: name.clone() }
            }
            E::MissingHomeContext => Self::MissingHome,
            E::VaultNotConfigured => Self::VaultNotConfigured,
            E::Io { path, source } => Self::Probe(ProbeError::new(path, source)),
            other => Self::Other {
                failure: classify(other),
                message: other.to_string(),
            },
        }
    }
}

fn classify(error: &FileReferenceError) -> ResolutionFailure {
    use FileReferenceError as E;
    match error {
        E::CurrentDirectory(_) | E::Io { .. } => ResolutionFailure::Io,
        E::RemoteNotLocal(_) => ResolutionFailure::UnsupportedRemote,
        E::InvalidSyntax(_)
        | E::UnsupportedScheme { .. }
        | E::ForeignAbsolutePath { .. }
        | E::RepositoryEscape { .. }
        | E::RelativeTreeEscape { .. }
        | E::RelativePath { .. } => ResolutionFailure::InvalidReference,
        _ => ResolutionFailure::MissingContext,
    }
}

/// A filesystem failure kept as data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeError {
    pub path: PathBuf,
    pub kind: ErrorKind,
    pub os_code: Option<i32>,
    /// Human-readable detail; never parse it.
    pub message: String,
}

impl ProbeError {
    pub(crate) fn new(path: &Path, error: &std::io::Error) -> Self {
        Self {
            path: path.to_path_buf(),
            kind: error.kind(),
            os_code: error.raw_os_error(),
            message: error.to_string(),
        }
    }
}

/// Why a path input cannot be a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidTarget {
    Relative,
    /// Absolute only on another OS.
    ForeignAbsolute,
}

/// Contradictory or invalid builder settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigurationProblem {
    /// `with_ctx` combined with `with_cwd`; derive the context instead.
    ContextWithCwd,
    /// `with_ctx` combined with `with_base_dir`; derive the context instead.
    ContextWithBaseDir,
    /// A `with_cwd` or `with_base_dir` directory that is not an absolute host
    /// path.
    RelativeDirectory { path: PathBuf },
    /// A `base_dir` inside a repository that is not the repository root.
    BaseDirNotRepositoryRoot {
        base_dir: PathBuf,
        repository_root: PathBuf,
    },
    /// The tree does not contain `cwd`.
    CwdOutsideBaseDir { base_dir: PathBuf, cwd: PathBuf },
    /// Any other context validation failure.
    InvalidContext(ResolutionProblem),
    /// A preference's eligibility filter has invalid syntax.
    InvalidFilter {
        strategy: PortabilityPreference,
        problem: FilterProblem,
    },
}

impl ConfigurationProblem {
    pub(crate) fn from_validation(error: &FileReferenceError) -> Self {
        match error {
            FileReferenceError::BaseDirNotRepositoryRoot {
                base_dir,
                repository_root,
            } => Self::BaseDirNotRepositoryRoot {
                base_dir: base_dir.clone(),
                repository_root: repository_root.clone(),
            },
            FileReferenceError::CwdOutsideBaseDir { base_dir, cwd } => Self::CwdOutsideBaseDir {
                base_dir: base_dir.clone(),
                cwd: cwd.clone(),
            },
            FileReferenceError::RepositoryRootNotContainingSource {
                repository_root,
                source_path,
            } => Self::CwdOutsideBaseDir {
                base_dir: repository_root.clone(),
                cwd: source_path.clone(),
            },
            other => Self::InvalidContext(ResolutionProblem::from_error(other)),
        }
    }
}

/// Why an eligibility filter is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterProblem {
    Empty,
    /// A `RepoRoot`/`RepoMultiPath` filter must be relative and stay below its
    /// root (no root, no leading `..`).
    NotARelativeSubdirectory,
    /// A `MagicPath` filter that does not parse as a local, non-recursive file
    /// reference.
    InvalidReference { message: String },
}

/// Why [`PortablePath::file_reference`](super::PortablePath::file_reference)
/// produced no reference.
///
/// Every variant exposes the attempts made before it was returned through
/// [`attempts`](Self::attempts), empty when preparation failed before any
/// preference ran.
#[derive(Debug, Clone)]
pub enum PortablePathError {
    /// No preference matched and `AbsolutePath` was not in the strategy.
    NoStrategyMatched {
        target: PathBuf,
        attempts: Vec<Attempt>,
        findings: Vec<Finding>,
    },
    /// A path input that cannot be a target.
    InvalidTarget { target: PathBuf, reason: InvalidTarget },
    /// A reference input that cannot be resolved to one target (a broken
    /// multi-candidate link, a missing anchor, a boundary denial, a probe
    /// failure). The caller keeps it as authored; `findings` says why.
    ///
    /// The reference is boxed here and in `NormalizationUnsupported` only to
    /// keep the error small (`clippy::result_large_err`).
    UnresolvableInput {
        reference: Box<FileReference>,
        attempts: Vec<Attempt>,
        findings: Vec<Finding>,
    },
    /// A URL or recursive `%` input that no `AuthoredIntent` kept.
    NormalizationUnsupported {
        reference: Box<FileReference>,
        attempts: Vec<Attempt>,
    },
    InvalidConfiguration(ConfigurationProblem),
    /// The target cannot be written faithfully as reference text (a
    /// non-Unicode path), even as an absolute path.
    UnrenderableTarget {
        target: PathBuf,
        attempts: Vec<Attempt>,
    },
    /// Probing the target or a candidate failed for a reason other than
    /// absence; a permission error is never treated as a missing file.
    ProbeFailed {
        target: PathBuf,
        error: ProbeError,
        attempts: Vec<Attempt>,
    },
    /// No context or `with_cwd` was given and the process working directory
    /// cannot be read.
    CwdUnavailable(ProbeError),
    /// Repository discovery from the effective `cwd` failed.
    RepositoryDiscoveryFailed {
        cwd: PathBuf,
        problem: ResolutionProblem,
    },
}

impl PortablePathError {
    /// The attempts made before the error, in strategy order.
    pub fn attempts(&self) -> &[Attempt] {
        match self {
            Self::NoStrategyMatched { attempts, .. }
            | Self::UnresolvableInput { attempts, .. }
            | Self::NormalizationUnsupported { attempts, .. }
            | Self::UnrenderableTarget { attempts, .. }
            | Self::ProbeFailed { attempts, .. } => attempts,
            Self::InvalidTarget { .. }
            | Self::InvalidConfiguration(_)
            | Self::CwdUnavailable(_)
            | Self::RepositoryDiscoveryFailed { .. } => &[],
        }
    }

    /// Findings recorded before the error, including invalid portable names.
    pub fn findings(&self) -> &[Finding] {
        match self {
            Self::NoStrategyMatched { findings, .. } | Self::UnresolvableInput { findings, .. } => {
                findings
            }
            _ => &[],
        }
    }

    fn headline(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoStrategyMatched { target, .. } => {
                write!(f, "no portable reference matched `{}`", target.display())
            }
            Self::InvalidTarget { target, reason } => {
                write!(f, "`{}` cannot be a target: {reason:?}", target.display())
            }
            Self::UnresolvableInput { reference, findings, .. } => write!(
                f,
                "`{}` does not resolve to one target: {findings:?}",
                reference.raw()
            ),
            Self::NormalizationUnsupported { reference, .. } => {
                write!(f, "`{}` is a URL or recursive search; no strategy rewrites it", reference.raw())
            }
            Self::InvalidConfiguration(problem) => write!(f, "invalid portable path configuration: {problem:?}"),
            Self::UnrenderableTarget { target, .. } => {
                write!(f, "`{}` cannot be written as reference text", target.display())
            }
            Self::ProbeFailed { error, .. } => {
                write!(f, "probing `{}` failed: {}", error.path.display(), error.message)
            }
            Self::CwdUnavailable(error) => write!(f, "the working directory is unavailable: {}", error.message),
            Self::RepositoryDiscoveryFailed { cwd, problem } => {
                write!(f, "repository discovery from `{}` failed: {problem:?}", cwd.display())
            }
        }
    }
}

impl fmt::Display for PortablePathError {
    /// A one-line headline, then one line per attempt.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.headline(f)?;
        for attempt in self.attempts() {
            write!(f, "\n  {attempt}")?;
        }
        Ok(())
    }
}

impl std::error::Error for PortablePathError {}

impl fmt::Display for Attempt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.strategy, self.outcome)?;
        if !self.rejected.is_empty() {
            f.write_str(" (after ")?;
            for (index, rejection) in self.rejected.iter().enumerate() {
                if index > 0 {
                    f.write_str("; ")?;
                }
                write!(f, "{rejection}")?;
            }
            f.write_str(")")?;
        }
        Ok(())
    }
}

impl fmt::Display for AttemptOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Matched(reference) => write!(f, "matched `{}`", reference.raw()),
            Self::NotApplicable(reason) => write!(f, "not applicable ({reason:?})"),
            Self::Shadowed {
                reference,
                resolves_to,
            } => write!(
                f,
                "`{}` resolves to `{}` instead",
                reference.raw(),
                resolves_to.display()
            ),
        }
    }
}
