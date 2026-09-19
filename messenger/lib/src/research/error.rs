//! Errors that stop a research file from being read at all.
//!
//! A readable file with contract problems is not an error: it yields
//! [`Diagnostic`](super::Diagnostic)s so stale or invalid research stays
//! inspectable. Messages carry repository-relative paths and causes only.

use std::path::PathBuf;

use super::paths::RepoPath;

#[derive(Debug, thiserror::Error)]
pub enum ResearchError {
    #[error("research workspace root must be absolute: {}", root.display())]
    RelativeRoot { root: PathBuf },

    #[error("{} is outside the research workspace", path.display())]
    OutsideWorkspace { path: PathBuf },

    #[error("cannot read {path}: {source}")]
    Io {
        path: RepoPath,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot parse the frontmatter of {path}: {message}")]
    Frontmatter { path: RepoPath, message: String },

    #[error("cannot resolve the schema of {path}: {message}")]
    SchemaResolution { path: RepoPath, message: String },
}
