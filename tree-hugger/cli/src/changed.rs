//! The `--changed` file selection: the set of files Git reports as changed
//! in the repository containing the invocation, keyed so a declaration
//! command's ordinary selection can be narrowed to it.
//!
//! Git paths are candidates, never explicit selectors: the set only removes
//! files from what the scanner already selected, so fixtures, ignored files
//! and out-of-scope paths are never re-included by Git's report.

use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

use sniff::SniffError;
use sniff::filesystem::git::{ChangedBase, GitRepo};
use tree_hugger::file_identity;

/// One `--changed` request: the flag alone, or with an explicit base
/// reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChangedRequest {
    /// `--changed` with no value. The committed comparison contributes
    /// nothing (the effective base is `HEAD`'s own tip), so an unborn
    /// `HEAD` is valid: staged and nonignored untracked paths still select.
    Omitted,
    /// `--changed=<ref>`: the token exactly as written. It must resolve
    /// locally to a commit; its tip is compared directly with `HEAD`'s.
    Reference(String),
}

impl ChangedRequest {
    /// The sniff base this request resolves against.
    fn base(&self) -> ChangedBase {
        match self {
            Self::Omitted => ChangedBase::Omitted,
            Self::Reference(token) => ChangedBase::Reference(token.clone()),
        }
    }
}

impl fmt::Display for ChangedRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Omitted => f.write_str("--changed"),
            Self::Reference(token) => write!(f, "--changed={token}"),
        }
    }
}

/// The resolved changed-file set one run intersects its selection with:
/// [`file_identity`] keys of present changed paths owned by the active checkout.
#[derive(Debug, Clone)]
pub(crate) struct ChangedFiles {
    identities: HashSet<PathBuf>,
}

impl ChangedFiles {
    /// Whether `identity`, a [`file_identity`] such as every scanned file
    /// is, is a changed file. Other spellings of a changed file do not
    /// match. Deleted paths were dropped while resolving, so an absent path
    /// never matches.
    pub(crate) fn contains_identity(&self, identity: &Path) -> bool {
        self.identities.contains(identity)
    }
}

/// Resolves the changed set for `request`, discovering the repository from
/// `cwd` (the active checkout, so a linked worktree's own root and index
/// are used).
///
/// Absent paths are dropped before canonicalization: a deleted path has no
/// current declarations to report, whether Git reported it as deleted or a
/// later category re-listed it.
///
/// ## Errors
///
/// A one-line message naming the repository or reference and the cause: no
/// repository, a bare repository, an unresolved or non-commit reference,
/// index conflicts anywhere in the repository, a Git failure, or filesystem
/// errors while verifying checkout ownership.
pub(crate) fn resolve(request: &ChangedRequest, cwd: &Path) -> Result<ChangedFiles, String> {
    let repo = match GitRepo::discover(cwd) {
        Ok(Some(repo)) => repo,
        Ok(None) => {
            return Err(format!(
                "{request} needs a git repository; {} is not inside one",
                cwd.display()
            ));
        }
        Err(source) => {
            return Err(format!(
                "could not discover a git repository from {}: {source}",
                cwd.display()
            ));
        }
    };
    let root = repo.repo_root().to_path_buf();
    let paths = repo
        .changed_paths(request.base())
        .map_err(|error| failure(&error, &root, request))?;
    let identity_root = file_identity(&root);
    let mut identities = HashSet::new();
    for relative in paths {
        let path = root.join(relative);
        if !path.try_exists().map_err(|error| ownership_error(&path, error))? {
            continue;
        }
        if checkout_owns_path(&path, &identity_root, 40)? {
            identities.insert(file_identity(&path));
        }
    }
    Ok(ChangedFiles { identities })
}

/// Check each visited directory before `..` can remove it. Alias targets
/// must obey the same traversal contract before their canonical identity is used.
fn checkout_owns_path(path: &Path, root: &Path, hops: usize) -> Result<bool, String> {
    Ok(checkout_path(path, root, hops)?.is_some())
}

fn checkout_path(path: &Path, root: &Path, hops: usize) -> Result<Option<PathBuf>, String> {
    if hops == 0 {
        return Ok(None);
    }
    let mut resolved = PathBuf::new();
    let mut entered = false;
    for component in path.components() {
        if matches!(component, std::path::Component::Prefix(_)) {
            resolved.push(component);
            continue;
        }
        if component == std::path::Component::CurDir {
            continue;
        }
        if component == std::path::Component::ParentDir {
            if !entered || resolved == root {
                return Ok(None);
            }
            resolved.pop();
        } else {
            resolved.push(component);
            if !entered {
                resolved = biscuit_file::canonicalize_simplified(&resolved)
                    .map_err(|error| ownership_error(&resolved, error))?;
                // Filesystem aliases above the checkout (macOS /var, Windows
                // short names) are allowed only along the root's own ancestry.
                if !root.starts_with(&resolved) {
                    return Ok(None);
                }
                entered = resolved == root;
                continue;
            }
            let metadata = std::fs::symlink_metadata(&resolved)
                .map_err(|error| ownership_error(&resolved, error))?;
            if metadata.file_type().is_symlink() {
                let target = std::fs::read_link(&resolved)
                    .map_err(|error| ownership_error(&resolved, error))?;
                let target = if target.is_absolute() { target } else { resolved.parent().unwrap().join(target) };
                let Some(target) = checkout_path(&target, root, hops - 1)? else {
                    return Ok(None);
                };
                resolved = target;
            } else {
                resolved = biscuit_file::canonicalize_simplified(&resolved)
                    .map_err(|error| ownership_error(&resolved, error))?;
            }
        }
        if !resolved.starts_with(root) {
            return Ok(None);
        }
        let metadata = std::fs::metadata(&resolved)
            .map_err(|error| ownership_error(&resolved, error))?;
        let directory = if metadata.is_dir() { resolved.as_path() } else { resolved.parent().unwrap() };
        // Canonicalized reparse points may skip destination ancestors that
        // were never components of the input spelling.
        for ancestor in directory.ancestors().take_while(|ancestor| *ancestor != root) {
            let marker = ancestor.join(".git");
            match std::fs::symlink_metadata(&marker) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(ownership_error(&marker, error)),
                Ok(_) => return Ok(None),
            }
        }
    }
    Ok(entered.then_some(resolved))
}

fn ownership_error(path: &Path, error: std::io::Error) -> String {
    format!("--changed could not verify checkout ownership of {}: {error}", path.display())
}

/// The exit-1 message for a failed changed-set resolution.
///
/// The typed sniff variants already name the repository (and, for conflicts,
/// every conflicted path), so they carry the request's spelling; a raw Git
/// failure names neither, so both are added here.
fn failure(error: &SniffError, root: &Path, request: &ChangedRequest) -> String {
    match error {
        SniffError::Git { operation, source } => format!(
            "{request} could not read the repository at {}: git {operation} failed: {source}",
            root.display()
        ),
        _ => format!("{request} failed: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_displays_the_form_the_user_wrote() {
        assert_eq!(ChangedRequest::Omitted.to_string(), "--changed");
        assert_eq!(
            ChangedRequest::Reference("main".to_string()).to_string(),
            "--changed=main"
        );
    }

    #[test]
    fn an_omitted_request_stays_omitted_down_to_sniff() {
        // The unborn-HEAD exception is decided by the Omitted/Reference
        // split, so the mapping must not collapse the flag alone into an
        // explicit "HEAD".
        assert_eq!(ChangedRequest::Omitted.base(), ChangedBase::Omitted);
        assert_eq!(
            ChangedRequest::Reference("HEAD".to_string()).base(),
            ChangedBase::Reference("HEAD".to_string())
        );
    }

    #[test]
    fn ownership_inspection_failure_is_an_error_naming_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("unavailable.rs");
        let error = checkout_owns_path(&missing, &file_identity(dir.path()), 40).unwrap_err();
        assert!(error.contains("--changed could not verify checkout ownership"), "{error}");
        assert!(error.contains("unavailable.rs"), "{error}");
    }

    #[test]
    fn an_empty_changed_set_contains_nothing() {
        let changed = ChangedFiles {
            identities: HashSet::new(),
        };
        assert!(!changed.contains_identity(Path::new("src/lib.rs")));
    }

    /// Every typed sniff failure keeps the request's spelling in the
    /// message, and a raw Git failure names the repository.
    #[test]
    fn failure_messages_name_the_request_and_the_repository() {
        let request = ChangedRequest::Reference("nope".to_string());
        let bare = SniffError::BareRepository {
            path: PathBuf::from("/repos/bare.git"),
        };
        let git = SniffError::Git {
            operation: "status",
            source: "corrupt index".into(),
        };
        let message = failure(&bare, Path::new("/repos/bare.git"), &request);
        assert!(message.contains("--changed=nope"), "{message}");
        let message = failure(&git, Path::new("/repos/app"), &request);
        assert!(message.contains("--changed=nope"), "{message}");
        assert!(message.contains("/repos/app"), "{message}");
        assert!(message.contains("git status failed"), "{message}");
    }

    /// Discovery outside any repository is the one failure sniff reports as
    /// `Ok(None)`; the message must still name the directory searched.
    #[test]
    fn resolving_outside_a_repository_names_the_directory() {
        // A system temp directory is outside any repository the test
        // controls; should one happen to sit inside a work tree, the
        // assertion still holds only for the error arm, so skip when
        // discovery finds something.
        if let Ok(dir) = tempfile::tempdir()
            && GitRepo::discover(dir.path()).ok().flatten().is_none()
        {
            let error = resolve(&ChangedRequest::Omitted, dir.path()).unwrap_err();
            assert!(error.contains("--changed"), "{error}");
            assert!(error.contains(&dir.path().display().to_string()), "{error}");
        }
    }
}
