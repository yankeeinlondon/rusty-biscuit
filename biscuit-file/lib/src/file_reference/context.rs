use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::{debug, trace};

use crate::file_reference::error::FileReferenceError;

/// Runtime state captured for file reference resolution.
pub(crate) struct ResolutionContext {
    pub cwd: PathBuf,
    pub home_dir: Option<PathBuf>,
    pub env: HashMap<String, String>,
}

impl ResolutionContext {
    /// Build from live process state.
    pub fn from_ambient() -> Result<Self, FileReferenceError> {
        let cwd = std::env::current_dir().map_err(FileReferenceError::CurrentDirectory)?;
        let home_dir = home_dir();
        let env: HashMap<String, String> = std::env::vars().collect();

        debug!(
            ?cwd,
            home_dir_set = home_dir.is_some(),
            env_var_count = env.len(),
            "built resolution context"
        );

        Ok(Self { cwd, home_dir, env })
    }

    /// Build a context that treats `base` as the working directory, while
    /// still reading HOME and environment variables from the live process
    /// state.
    ///
    /// If `base` is a relative path, it is joined onto the ambient CWD so
    /// that git and workspace discovery always operate on an absolute
    /// location.
    pub fn from_base(base: &Path) -> Result<Self, FileReferenceError> {
        let cwd = if base.is_absolute() {
            base.to_path_buf()
        } else {
            let ambient = std::env::current_dir().map_err(FileReferenceError::CurrentDirectory)?;
            ambient.join(base)
        };
        let home_dir = home_dir();
        let env = std::env::vars().collect();

        Ok(Self { cwd, home_dir, env })
    }
}

/// Find the git repository root starting from `from`.
///
/// Returns `Ok(None)` if no git repository is found.
pub(crate) fn find_git_root(from: &Path) -> Result<Option<PathBuf>, FileReferenceError> {
    use gix::discover::upwards::Error as Up;
    trace!(?from, "searching for git root");
    match gix::discover(from) {
        Ok(repo) => {
            let workdir = repo.workdir().ok_or_else(|| {
                FileReferenceError::Git("bare repository has no working directory".to_string())
            })?;
            debug!(?workdir, "found git root");
            Ok(Some(workdir.to_path_buf()))
        }
        // Upward-search exhaustion is the only "not a repository" outcome;
        // trust, permission, and corruption failures propagate as errors.
        Err(gix::discover::Error::Discover(
            Up::NoGitRepository { .. }
            | Up::NoGitRepositoryWithinCeiling { .. }
            | Up::NoGitRepositoryWithinFs { .. },
        )) => {
            trace!("no git repository found");
            Ok(None)
        }
        Err(e) => Err(FileReferenceError::Git(e.to_string())),
    }
}

/// Find the package area (first path component of a workspace member) for
/// the current working directory within a Cargo workspace.
///
/// Given a workspace root and a CWD within it, this identifies which workspace
/// member contains the CWD and returns its "area" directory (the first path
/// component under the workspace root).
///
/// For a single-crate repo (no workspace members), returns `None`.
pub(crate) fn find_package_area(
    repo_root: &Path,
    cwd: &Path,
) -> Result<Option<PathBuf>, FileReferenceError> {
    trace!(?repo_root, ?cwd, "searching for package area");
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(repo_root.join("Cargo.toml"))
        .no_deps()
        .exec()
        .map_err(|e| FileReferenceError::Workspace(e.to_string()))?;

    let workspace_root = metadata.workspace_root.as_std_path();

    // If there are no workspace members beyond the root, this is a single-crate repo
    let members: Vec<_> = metadata
        .workspace_packages()
        .into_iter()
        .filter_map(|pkg| {
            let manifest = pkg.manifest_path.as_std_path();
            let pkg_dir = manifest.parent()?;
            pkg_dir.strip_prefix(workspace_root).ok().map(|rel| {
                let first_component = rel
                    .components()
                    .next()
                    .map(|c| PathBuf::from(c.as_os_str()))
                    .unwrap_or_default();
                (pkg_dir.to_path_buf(), first_component)
            })
        })
        .collect();

    // Find the member whose directory is an ancestor of CWD
    let cwd_normalized = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());

    for (pkg_dir, area) in &members {
        let pkg_normalized = pkg_dir
            .canonicalize()
            .unwrap_or_else(|_| pkg_dir.to_path_buf());

        if cwd_normalized.starts_with(&pkg_normalized) && !area.as_os_str().is_empty() {
            let found = workspace_root.join(area);
            debug!(?found, "found package area");
            return Ok(Some(found));
        }
    }

    // Fallback: CWD may sit at the area root itself (between workspace_root and
    // the package manifest directory), e.g. `repo/claudine` when packages are at
    // `repo/claudine/lib` and `repo/claudine/cli`. Match against area dirs.
    let workspace_root_normalized = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());

    for (_, area) in &members {
        if area.as_os_str().is_empty() {
            continue;
        }
        let area_root = workspace_root_normalized.join(area);
        if cwd_normalized.starts_with(&area_root) {
            return Ok(Some(workspace_root.join(area)));
        }
    }

    trace!("no package area found");
    Ok(None)
}

/// Get the user's home directory.
pub(crate) fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_ambient_succeeds() {
        let ctx = ResolutionContext::from_ambient().unwrap();
        assert!(ctx.cwd.is_absolute());
    }

    #[test]
    fn from_base_absolute_path_is_preserved() {
        let abs = Path::new("/tmp");
        let ctx = ResolutionContext::from_base(abs).unwrap();
        assert_eq!(ctx.cwd, PathBuf::from("/tmp"));
    }

    #[test]
    fn from_base_relative_path_is_joined_to_ambient_cwd() {
        let ctx = ResolutionContext::from_base(Path::new("sub/dir")).unwrap();
        assert!(ctx.cwd.is_absolute());
        assert!(ctx.cwd.ends_with("sub/dir"));
    }

    #[test]
    fn find_git_root_inside_repo() {
        // We're inside the rusty-biscuit repo
        let root = find_git_root(&std::env::current_dir().unwrap()).unwrap();
        assert!(root.is_some());
    }

    #[test]
    fn find_git_root_outside_repo() {
        // /tmp is unlikely to be in a git repo
        let root = find_git_root(Path::new("/tmp")).unwrap();
        assert!(root.is_none());
    }
}
