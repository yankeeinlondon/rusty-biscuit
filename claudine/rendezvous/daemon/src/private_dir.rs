//! Where the daemon's durable state lives, and what makes a directory one the
//! daemon may put it in.
//!
//! Two directories carry the same requirement: the Unix runtime directory
//! holding the endpoint, and the durable data root holding the node-identity
//! seed and the session databases. Both must belong to the same OS user that
//! [`rendezvous_core::default_local_endpoint`] qualified the endpoint with,
//! and neither may be reachable through a component another user can re-point.
//! One helper serves both rather than two near-identical owner/type/symlink/mode
//! checks drifting apart.
//!
//! ## Notes
//!
//! The legacy `<tempdir>/rendezvous-data` root is not consulted, migrated, or
//! read. A shared temp directory is not an ownership boundary, so a node
//! identity found there could have been planted by any local user; adopting it
//! automatically would let an attacker choose the daemon's signing key. Any
//! recovery of development data is a deliberate manual step.

use std::path::{Path, PathBuf};

/// Failures preparing or validating a directory the daemon must own privately.
#[derive(Debug, thiserror::Error)]
pub enum PrivateDirError {
    /// The platform reported no local data directory, so there is nowhere
    /// per-user and durable to put daemon state.
    #[error("this platform reports no local data directory for the current user")]
    NoLocalDataDir,

    /// A relative root cannot be validated: the components it traverses depend
    /// on the process's working directory at the moment of the check.
    #[error("{path} must be an absolute path")]
    NotAbsolute { path: PathBuf },

    /// A component of the path is a symlink. Its target's ownership says
    /// nothing about the link's, and the link can be re-pointed between the
    /// check and the open.
    #[error("{path} is a symlink; the daemon will not traverse one")]
    SymlinkComponent { path: PathBuf },

    /// A component of the path exists but is not a directory.
    #[error("{path} exists but is not a directory")]
    NotADirectory { path: PathBuf },

    /// The directory belongs to another user.
    #[error("{path} is owned by uid {owner}, not by this process's uid {expected}")]
    ForeignOwner {
        path: PathBuf,
        owner: u32,
        expected: u32,
    },

    /// The directory grants access beyond its owner.
    #[error("{path} has mode {mode:04o}; the daemon requires owner-only access")]
    NotPrivate { path: PathBuf, mode: u32 },

    /// The directory could not be created.
    #[error("failed to create {path}")]
    Create {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The directory could not be inspected.
    #[error("failed to inspect {path}")]
    Inspect {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The OS user this process runs as could not be discovered, so ownership
    /// cannot be checked against anything.
    #[error("cannot verify directory ownership without a stable OS user identity")]
    UserIdentity(#[from] sniff::SniffError),

    /// The host reported an identity of the wrong kind for this target.
    /// Unreachable in practice; kept typed so a future identity source cannot
    /// silently skip the ownership check.
    #[error("the host reported {actual}, which cannot own a Unix directory")]
    MismatchedIdentity { actual: &'static str },
}

/// The default durable data root: `<local-data-dir>/claudine/rendezvous`.
///
/// ## Errors
///
/// Returns [`PrivateDirError::NoLocalDataDir`] when the platform reports no
/// per-user local data directory.
///
/// ## Notes
///
/// Resolution only; the caller passes the result to [`ensure_private_dir`]
/// before opening anything under it.
pub fn default_data_dir() -> Result<PathBuf, PrivateDirError> {
    let base = dirs::data_local_dir().ok_or(PrivateDirError::NoLocalDataDir)?;
    Ok(base.join("claudine").join("rendezvous"))
}

/// Create `path` if absent and confirm it is a private directory owned by this
/// process's user.
///
/// `path` itself must be a real directory — never a symlink — owned by this
/// user with no group or other access. Components the daemon has to create are
/// made that way, and the deepest one that already exists is checked for the
/// same. Ancestors above it may be shared and may be symlinks: `/tmp` is
/// world-writable, and macOS reaches every temp directory through the `/var`
/// symlink, so rejecting those would reject the platforms this has to run on.
///
/// ## Errors
///
/// Returns a [`PrivateDirError`] when the path is relative, traverses a symlink
/// or a non-directory, cannot be created or inspected, or resolves to a
/// directory owned by another user or readable beyond its owner.
pub fn ensure_private_dir(path: &Path) -> Result<(), PrivateDirError> {
    if !path.is_absolute() {
        return Err(PrivateDirError::NotAbsolute {
            path: path.to_path_buf(),
        });
    }

    for missing in missing_ancestors(path)?.iter().rev() {
        create_private(missing)?;
    }
    verify_private(path)
}

/// The components of `path` that do not exist yet, deepest first, after
/// confirming every component that *does* exist is a plain directory.
fn missing_ancestors(path: &Path) -> Result<Vec<PathBuf>, PrivateDirError> {
    let mut missing = Vec::new();
    let mut cursor = path;
    loop {
        match std::fs::symlink_metadata(cursor) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(PrivateDirError::SymlinkComponent {
                    path: cursor.to_path_buf(),
                });
            }
            Ok(meta) if !meta.is_dir() => {
                return Err(PrivateDirError::NotADirectory {
                    path: cursor.to_path_buf(),
                });
            }
            Ok(_) => return Ok(missing),
            // `NotADirectory` means some component above `cursor` is a file, so
            // `cursor` cannot exist. Walking up reaches that component and
            // reports it by name, which is what an operator has to fix.
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                missing.push(cursor.to_path_buf());
                cursor = cursor.parent().ok_or(PrivateDirError::NotAbsolute {
                    path: path.to_path_buf(),
                })?;
            }
            Err(source) => {
                return Err(PrivateDirError::Inspect {
                    path: cursor.to_path_buf(),
                    source,
                });
            }
        }
    }
}

/// Create `dir` such that it is never, for any instant, more permissive than
/// owner-only.
#[cfg(unix)]
fn create_private(dir: &Path) -> Result<(), PrivateDirError> {
    use std::os::unix::fs::DirBuilderExt;

    // mode() on the builder is applied by mkdir(2) itself, so there is no
    // window between creation and chmod in which another user could enter the
    // directory. (The umask can only subtract, never add.)
    let result = std::fs::DirBuilder::new().mode(0o700).create(dir);
    match result {
        Ok(()) => Ok(()),
        // Someone won the race to create it. Whatever they made is now subject
        // to the same inspection as any pre-existing component.
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => verify_not_symlink(dir),
        Err(source) => Err(PrivateDirError::Create {
            path: dir.to_path_buf(),
            source,
        }),
    }
}

// The Windows current-user DACL lands with the named-pipe transport, which is
// the first code on that target able to exercise it.
#[cfg(windows)]
fn create_private(dir: &Path) -> Result<(), PrivateDirError> {
    match std::fs::create_dir(dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => verify_not_symlink(dir),
        Err(source) => Err(PrivateDirError::Create {
            path: dir.to_path_buf(),
            source,
        }),
    }
}

fn verify_not_symlink(dir: &Path) -> Result<(), PrivateDirError> {
    let meta = std::fs::symlink_metadata(dir).map_err(|source| PrivateDirError::Inspect {
        path: dir.to_path_buf(),
        source,
    })?;
    if meta.file_type().is_symlink() {
        return Err(PrivateDirError::SymlinkComponent {
            path: dir.to_path_buf(),
        });
    }
    if !meta.is_dir() {
        return Err(PrivateDirError::NotADirectory {
            path: dir.to_path_buf(),
        });
    }
    Ok(())
}

#[cfg(unix)]
fn verify_private(path: &Path) -> Result<(), PrivateDirError> {
    use sniff::os::StableUserId;
    use std::os::unix::fs::MetadataExt;

    verify_not_symlink(path)?;
    let meta = std::fs::symlink_metadata(path).map_err(|source| PrivateDirError::Inspect {
        path: path.to_path_buf(),
        source,
    })?;

    // The same discovery that qualified the endpoint name, so the endpoint,
    // this root, and the identity seed inside it cannot end up keyed to
    // different users.
    let StableUserId::UnixUid(expected) = sniff::os::current_user_id()? else {
        return Err(PrivateDirError::MismatchedIdentity {
            actual: "a Windows SID",
        });
    };
    if meta.uid() != expected {
        return Err(PrivateDirError::ForeignOwner {
            path: path.to_path_buf(),
            owner: meta.uid(),
            expected,
        });
    }
    let mode = meta.mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(PrivateDirError::NotPrivate {
            path: path.to_path_buf(),
            mode,
        });
    }
    Ok(())
}

#[cfg(windows)]
fn verify_private(path: &Path) -> Result<(), PrivateDirError> {
    verify_not_symlink(path)
}

#[cfg(test)]
mod tests;
