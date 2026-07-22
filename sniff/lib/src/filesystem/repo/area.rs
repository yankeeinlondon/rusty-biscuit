use std::path::Path;

use crate::SniffError;
use crate::filesystem::repo::types::detect_repo;

/// Errors returned by [`detect_area`].
///
/// The CLI matches on [`NotInRepo`](AreaError::NotInRepo) and
/// [`NotMonorepo`](AreaError::NotMonorepo) to choose the right user-facing
/// message; underlying I/O failures during repo detection are surfaced
/// through [`Io`](AreaError::Io).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AreaError {
    /// `dir` is not inside any detected repository.
    #[error("not in a repo")]
    NotInRepo,

    /// `dir` is inside a repo, but the repo is not a monorepo.
    #[error("in a repo but not a monorepo")]
    NotMonorepo,

    /// An I/O failure occurred while detecting the repo.
    #[error("io error while detecting repo: {0}")]
    Io(#[from] std::io::Error),
}

impl From<SniffError> for AreaError {
    fn from(err: SniffError) -> Self {
        match err {
            SniffError::Io(io) => AreaError::Io(io),
            other => AreaError::Io(std::io::Error::other(other.to_string())),
        }
    }
}

/// Detect the area name for `dir`.
///
/// Convenience wrapper around [`detect_repo`] +
/// [`RepoInfo::area_for_dir`](crate::filesystem::repo::RepoInfo::area_for_dir).
/// The returned string is owned to free the caller from juggling the
/// `RepoInfo` lifetime.
///
/// ## Errors
///
/// - [`AreaError::NotInRepo`] when `dir` is not inside any repo.
/// - [`AreaError::NotMonorepo`] when the surrounding repo is not a monorepo.
/// - [`AreaError::Io`] when repo detection itself fails.
pub fn detect_area(dir: &Path) -> Result<String, AreaError> {
    let repo = detect_repo(dir)?.ok_or(AreaError::NotInRepo)?;
    if !repo.is_monorepo {
        return Err(AreaError::NotMonorepo);
    }
    Ok(repo.area_for_dir(dir).to_string())
}

#[cfg(test)]
mod tests {
    use crate::filesystem::repo::{AreaError, detect_area};

    #[test]
    fn area_error_displays_not_in_repo() {
        assert_eq!(AreaError::NotInRepo.to_string(), "not in a repo");
    }

    #[test]
    fn area_error_displays_not_monorepo() {
        assert_eq!(
            AreaError::NotMonorepo.to_string(),
            "in a repo but not a monorepo"
        );
    }

    #[test]
    fn area_error_wraps_io_via_from() {
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "nope");
        let err: AreaError = io.into();
        assert!(matches!(err, AreaError::Io(_)));
    }

    #[test]
    fn detect_area_errors_when_not_in_repo() {
        let tmp = tempfile::tempdir().expect("isolated temp directory");
        let err = detect_area(tmp.path()).expect_err("temp dir should not be a repo");
        // Accept either NotInRepo or NotMonorepo depending on host setup;
        // the contract is "fails when no monorepo present".
        assert!(
            matches!(err, AreaError::NotInRepo | AreaError::NotMonorepo),
            "got unexpected variant: {err:?}"
        );
    }
}
