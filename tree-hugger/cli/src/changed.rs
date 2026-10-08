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
/// [`file_identity`] keys of the changed paths that still exist on disk.
#[derive(Debug, Clone)]
pub(crate) struct ChangedFiles {
    identities: HashSet<PathBuf>,
}

impl ChangedFiles {
    /// Whether `file` — in any spelling — is a changed file. Deleted paths
    /// were dropped while resolving, so an absent path never matches.
    pub(crate) fn contains(&self, file: &Path) -> bool {
        self.identities.contains(&file_identity(file))
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
/// index conflicts anywhere in the repository, or a Git failure.
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
    let identities = paths
        .into_iter()
        .map(|relative| root.join(relative))
        .filter(|path| path.exists())
        .map(|path| file_identity(&path))
        .collect();
    Ok(ChangedFiles { identities })
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
    fn an_empty_changed_set_contains_nothing() {
        let changed = ChangedFiles {
            identities: HashSet::new(),
        };
        assert!(!changed.contains(Path::new("src/lib.rs")));
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
