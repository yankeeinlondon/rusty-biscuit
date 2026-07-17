use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum FileReferenceError {
    #[error("file reference syntax is invalid: {0}")]
    InvalidSyntax(String),

    #[error("environment variable `{name}` is not set")]
    MissingEnvironmentVariable { name: String },

    #[error("could not determine the current working directory: {0}")]
    CurrentDirectory(#[source] std::io::Error),

    #[error("could not inspect git repository state: {0}")]
    Git(String),

    #[error("could not inspect Cargo workspace state: {0}")]
    Workspace(String),

    #[error("vault reference used without any configured vault roots")]
    VaultNotConfigured,

    #[error("`~user` home references are not supported: `{0}`")]
    UnsupportedUserHome(String),

    #[error("home reference used but no home directory is available in the resolution context")]
    MissingHomeContext,

    #[error(
        "repository root `{repository_root}` does not contain the resolution source `{source_path}`"
    )]
    RepositoryRootNotContainingSource {
        repository_root: PathBuf,
        source_path: PathBuf,
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
