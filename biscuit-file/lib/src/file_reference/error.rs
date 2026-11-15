use std::path::PathBuf;

use super::ResolutionFailure;

#[derive(Debug, thiserror::Error)]
pub enum FileReferenceError {
    #[error("file reference syntax is invalid: {0}")]
    InvalidSyntax(String),

    #[error("unsupported file reference scheme `{scheme}` in `{reference}`")]
    UnsupportedScheme { scheme: String, reference: String },

    #[error(
        "`{path}` is absolute only on another operating system and cannot be located on this host"
    )]
    ForeignAbsolutePath { path: String },

    #[error("environment variable `{name}` is not set")]
    MissingEnvironmentVariable { name: String },

    #[error("could not determine the current working directory: {0}")]
    CurrentDirectory(#[source] std::io::Error),

    #[cfg(feature = "file-reference")]
    #[error("could not inspect git repository state: {0}")]
    Git(#[source] Box<gix::Error>),

    #[error("git repository is bare and has no working directory")]
    BareRepository,

    #[error("vault reference used without any configured vault roots")]
    VaultNotConfigured,

    #[error("`~user` home references are not supported: `{0}`")]
    UnsupportedUserHome(String),

    #[error("home reference used but no home directory is available in the resolution context")]
    MissingHomeContext,

    #[error(
        "`{sigil}` repository reference requires a repository containing reference CWD `{reference_cwd}`"
    )]
    OutsideRepository {
        sigil: char,
        reference_cwd: PathBuf,
    },

    #[error(
        "`{sigil}` reference `{reference}` escapes repository root `{repository_root}` through candidate `{escaped_candidate}`"
    )]
    RepositoryEscape {
        sigil: char,
        reference: String,
        repository_root: PathBuf,
        escaped_candidate: PathBuf,
    },

    #[error("reference `{reference}` escapes boundary `{boundary}` through candidate `{escaped_candidate}`")]
    BoundaryEscape {
        reference: String,
        boundary: PathBuf,
        escaped_candidate: PathBuf,
    },

    #[error(
        "repository root `{repository_root}` does not contain the resolution source `{source_path}`"
    )]
    RepositoryRootNotContainingSource {
        repository_root: PathBuf,
        source_path: PathBuf,
    },

    /// A non-repository file tree does not contain a required working
    /// directory. The repository counterpart is
    /// [`RepositoryRootNotContainingSource`](Self::RepositoryRootNotContainingSource).
    #[error("file tree `{base_dir}` does not contain the working directory `{cwd}`")]
    CwdOutsideBaseDir { base_dir: PathBuf, cwd: PathBuf },

    /// A caller compiled without file-reference resolution (a passive,
    /// syntax-only build) was asked to resolve `reference`. The reference is
    /// not judged to exist or not: the check is unavailable in this build.
    #[error("file reference `{reference}` cannot be resolved: this build has no file-reference resolution")]
    ResolutionUnavailable { reference: String },

    /// A context directory or tree anchor is not an absolute host path.
    ///
    /// Probing a relative candidate would read the live process directory, so
    /// a captured context could resolve differently after that directory
    /// changes. Environment values and configured magic and vault roots are
    /// not anchors in this sense and may stay relative.
    #[error("context {anchor} `{path}` is not an absolute path")]
    RelativeContextDirectory {
        anchor: super::ContextAnchor,
        path: PathBuf,
    },

    /// An explicit tree root disagrees with the supplied repository root.
    /// Inside a repository the tree root is always the repository root.
    #[error(
        "explicit base directory `{base_dir}` conflicts with repository root `{repository_root}`; inside a repository the base directory must be the repository root"
    )]
    BaseDirNotRepositoryRoot {
        base_dir: PathBuf,
        repository_root: PathBuf,
    },

    /// A relative reference leaves the file tree, as written or where it
    /// really lands through a symlink, junction, or reparse point.
    #[error(
        "relative reference `{reference}` leaves file tree `{base_dir}` through candidate `{candidate}`"
    )]
    RelativeTreeEscape {
        base_dir: PathBuf,
        candidate: PathBuf,
        reference: String,
    },

    #[error("could not produce a relative path from `{from}` to `{to}`")]
    RelativePath { from: PathBuf, to: PathBuf },

    #[error("filesystem error while resolving `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("remote URL reference cannot be resolved to a local path: {0}")]
    RemoteNotLocal(String),

    #[cfg(feature = "url")]
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
}

impl FileReferenceError {
    /// The [`ResolutionFailure`] class of this error.
    ///
    /// This is the same classification
    /// [`DetailedResolution`](crate::file_reference::DetailedResolution) reports for a
    /// failed outcome. It is never [`ResolutionFailure::NoMatch`], which is an
    /// outcome without an error.
    #[must_use]
    pub fn resolution_failure(&self) -> ResolutionFailure {
        classify_error(self)
    }
}

/// Map a typed [`FileReferenceError`] onto its resolution-failure class.
///
/// The class must not be re-derived from the reference kind downstream (D8);
/// permission/I/O failures in particular must stay distinct from `no_match`.
pub(crate) fn classify_error(error: &FileReferenceError) -> ResolutionFailure {
    use FileReferenceError as E;
    match error {
        E::RemoteNotLocal(_) => ResolutionFailure::UnsupportedRemote,
        #[cfg(feature = "url")]
        E::InvalidUrl(_) => ResolutionFailure::UnsupportedRemote,
        E::InvalidSyntax(_)
        | E::ForeignAbsolutePath { .. }
        | E::UnsupportedScheme { .. }
        | E::RepositoryEscape { .. }
        | E::BoundaryEscape { .. }
        | E::RelativeTreeEscape { .. }
        | E::RelativePath { .. } => ResolutionFailure::InvalidReference,
        E::MissingEnvironmentVariable { .. }
        | E::VaultNotConfigured
        | E::MissingHomeContext
        | E::OutsideRepository { .. }
        | E::UnsupportedUserHome(_)
        | E::RepositoryRootNotContainingSource { .. }
        | E::CwdOutsideBaseDir { .. }
        | E::RelativeContextDirectory { .. }
        | E::BaseDirNotRepositoryRoot { .. }
        | E::BareRepository
        // A build without resolution has no context to resolve in.
        | E::ResolutionUnavailable { .. } => ResolutionFailure::MissingContext,
        #[cfg(feature = "file-reference")]
        E::Git(_) => ResolutionFailure::MissingContext,
        E::Io { .. } | E::CurrentDirectory(_) => ResolutionFailure::Io,
    }
}

#[cfg(feature = "fetch")]
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("host `{host}` denied by fetch policy")]
    PolicyDenied { host: String },

    #[error("unsupported URL scheme: {0}")]
    UnsupportedScheme(String),

    #[error("failed to build fetch client: {0}")]
    ClientBuild(#[source] reqwest::Error),

    #[error("HTTP request failed: {0}")]
    RequestFailed(#[source] reqwest::Error),

    #[error("HTTP error status {status} from {url}")]
    HttpError { status: u16, url: String },

    #[error("redirect (HTTP {status}) to `{location}` blocked by fetch policy")]
    RedirectBlocked { status: u16, location: String },

    #[error("failed to read response body: {0}")]
    BodyReadFailed(#[source] reqwest::Error),

    #[error("invalid response header `{header}`: {value}")]
    InvalidHeader { header: String, value: String },
}
