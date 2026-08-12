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
//! This module is where "belongs to this user" is *defined*, on both targets:
//! `expected_uid` on Unix and `current_user_descriptor` on Windows. The
//! named-pipe transport applies the same descriptor to its endpoint, so the
//! pipe's DACL and the data root's cannot come to disagree about who this user
//! is.
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
    #[cfg(unix)]
    #[error("{path} is owned by uid {owner}, not by this process's uid {expected}")]
    ForeignOwner {
        path: PathBuf,
        owner: u32,
        expected: u32,
    },

    /// The directory belongs to another Windows account. An owner can rewrite
    /// a DACL at will, so this is the check an override has to pass.
    #[cfg(windows)]
    #[error("{path} is owned by {owner}, not by this process's account {expected}")]
    ForeignAccount {
        path: PathBuf,
        owner: String,
        expected: String,
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
    #[error("the host reported {actual}, which cannot own anything on this target")]
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

/// Create `dir` such that it is never, for any instant, reachable by another
/// user.
///
/// Windows has no mode bits: "owner-only" is a DACL naming exactly one
/// principal. Passing it to `CreateDirectoryW` applies it at creation, so — as
/// on Unix — there is no window between creation and hardening, and no
/// inherited ACE from the parent container is ever in force.
#[cfg(windows)]
fn create_private(dir: &Path) -> Result<(), PrivateDirError> {
    use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
    use windows::Win32::Security::SECURITY_ATTRIBUTES;
    use windows::Win32::Storage::FileSystem::CreateDirectoryW;
    use windows::core::PCWSTR;

    let descriptor = current_user_descriptor()?;
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(u32::MAX),
        lpSecurityDescriptor: descriptor.as_ptr(),
        bInheritHandle: false.into(),
    };
    let wide = encode_wide_path(dir);

    // SAFETY: `wide` is NUL-terminated and `attributes` (with its descriptor)
    // outlives the call.
    let result = unsafe { CreateDirectoryW(PCWSTR(wide.as_ptr()), Some(&raw mut attributes)) };
    match result {
        Ok(()) => Ok(()),
        // Someone won the race to create it. Whatever they made is now subject
        // to the same inspection as any pre-existing component.
        Err(error) if error.code() == ERROR_ALREADY_EXISTS.to_hresult() => verify_not_symlink(dir),
        Err(error) => Err(PrivateDirError::Create {
            path: dir.to_path_buf(),
            source: windows_io_error(&error),
        }),
    }
}

#[cfg(windows)]
fn encode_wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
fn windows_io_error(error: &windows::core::Error) -> std::io::Error {
    std::io::Error::from_raw_os_error(error.code().0)
}

/// A security descriptor granting only the current user access.
///
/// The Windows counterpart of [`expected_uid`]: the one place that decides
/// which principal may reach the daemon's private state, shared by the data
/// root and the named-pipe endpoint so the two cannot disagree about who this
/// user is.
///
/// Built from SDDL rather than assembled ACE by ACE: one Win32 call and one
/// allocation to free, instead of a SID conversion, an `EXPLICIT_ACCESS_W`, a
/// `SetEntriesInAclW`, and three lifetimes to keep straight.
///
/// ## Errors
///
/// Returns [`PrivateDirError::UserIdentity`] when the OS user cannot be
/// discovered, [`PrivateDirError::MismatchedIdentity`] when the host reports a
/// Unix UID on a Windows target, and [`PrivateDirError::Create`] when Win32
/// rejects the descriptor.
#[cfg(windows)]
pub(crate) fn current_user_descriptor() -> Result<SecurityDescriptor, PrivateDirError> {
    use sniff::os::StableUserId;
    use windows::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows::Win32::Security::PSECURITY_DESCRIPTOR;
    use windows::core::PCWSTR;

    let StableUserId::WindowsSid(sid) = sniff::os::current_user_id()? else {
        return Err(PrivateDirError::MismatchedIdentity {
            actual: "a Unix UID",
        });
    };

    // O: owner. D:P: a protected DACL, so no inheritable ACE from a container
    // can widen it. (A;;GA;;;<sid>): allow generic-all to this user, and to
    // nobody else — an ACL with no matching ACE denies by default.
    let sddl: Vec<u16> = format!("O:{sid}D:P(A;;GA;;;{sid})")
        .encode_utf16()
        .chain(Some(0))
        .collect();

    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // SAFETY: `sddl` is NUL-terminated and outlives the call; the descriptor is
    // written into a local we immediately hand to the RAII owner.
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl.as_ptr()),
            SDDL_REVISION_1,
            &raw mut descriptor,
            None,
        )
    }
    .map_err(|error| PrivateDirError::Create {
        path: PathBuf::from("the current user's security descriptor"),
        source: windows_io_error(&error),
    })?;

    Ok(SecurityDescriptor(descriptor))
}

/// A `LocalAlloc`-ed security descriptor, freed exactly once.
#[cfg(windows)]
pub(crate) struct SecurityDescriptor(windows::Win32::Security::PSECURITY_DESCRIPTOR);

#[cfg(windows)]
impl SecurityDescriptor {
    pub(crate) fn as_ptr(&self) -> *mut std::ffi::c_void {
        self.0.0
    }
}

#[cfg(windows)]
impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        use windows::Win32::Foundation::{HLOCAL, LocalFree};

        if !self.0.0.is_null() {
            // SAFETY: the pointer came from
            // `ConvertStringSecurityDescriptorToSecurityDescriptorW`, which
            // allocates with `LocalAlloc`, and this is the only owner.
            // `LocalFree` returns the handle on failure; there is nothing a
            // `Drop` could do about that.
            let _ = unsafe { LocalFree(Some(HLOCAL(self.0.0))) };
        }
    }
}

// SAFETY: the descriptor is treated as immutable after construction — Win32
// only reads it — and the allocation is not thread-affine.
#[cfg(windows)]
unsafe impl Send for SecurityDescriptor {}
#[cfg(windows)]
unsafe impl Sync for SecurityDescriptor {}

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

/// The uid every directory and endpoint the daemon owns must belong to.
///
/// The same discovery that qualified the endpoint name, so the endpoint, the
/// data root, and the identity seed inside it cannot end up keyed to different
/// users.
///
/// ## Errors
///
/// Returns [`PrivateDirError::UserIdentity`] when the OS user cannot be
/// discovered, and [`PrivateDirError::MismatchedIdentity`] when the host
/// reports a Windows SID on a Unix target.
#[cfg(unix)]
pub(crate) fn expected_uid() -> Result<u32, PrivateDirError> {
    use sniff::os::StableUserId;

    match sniff::os::current_user_id()? {
        StableUserId::UnixUid(uid) => Ok(uid),
        StableUserId::WindowsSid(_) => Err(PrivateDirError::MismatchedIdentity {
            actual: "a Windows SID",
        }),
    }
}

#[cfg(unix)]
fn verify_private(path: &Path) -> Result<(), PrivateDirError> {
    use std::os::unix::fs::MetadataExt;

    verify_not_symlink(path)?;
    let meta = std::fs::symlink_metadata(path).map_err(|source| PrivateDirError::Inspect {
        path: path.to_path_buf(),
        source,
    })?;

    let expected = expected_uid()?;
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

/// Confirm `path` is a directory this account owns.
///
/// Ownership is the check that matters, and it is the one an override has to
/// pass: an owner can rewrite a DACL at will, so a directory owned by another
/// account cannot be trusted however its DACL reads right now. A directory this
/// account owns but has widened is left alone rather than silently narrowed —
/// the same rule as the Unix mode check.
#[cfg(windows)]
fn verify_private(path: &Path) -> Result<(), PrivateDirError> {
    use sniff::os::StableUserId;

    verify_not_symlink(path)?;

    let StableUserId::WindowsSid(expected) = sniff::os::current_user_id()? else {
        return Err(PrivateDirError::MismatchedIdentity {
            actual: "a Unix UID",
        });
    };
    let owner = owner_sid(path)?;
    if owner != expected {
        return Err(PrivateDirError::ForeignAccount {
            path: path.to_path_buf(),
            owner,
            expected,
        });
    }
    Ok(())
}

/// The SID of `path`'s owner, in canonical `S-1-...` form.
#[cfg(windows)]
fn owner_sid(path: &Path) -> Result<String, PrivateDirError> {
    use windows::Win32::Foundation::{ERROR_SUCCESS, HLOCAL, LocalFree, WIN32_ERROR};
    use windows::Win32::Security::Authorization::{
        ConvertSidToStringSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT,
    };
    use windows::Win32::Security::{OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID};
    use windows::core::{PCWSTR, PWSTR};

    let wide = encode_wide_path(path);
    let mut owner = PSID::default();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();

    // SAFETY: `wide` is NUL-terminated and outlives the call. `owner` points
    // into `descriptor`, which Win32 allocates and we free below.
    let status: WIN32_ERROR = unsafe {
        GetNamedSecurityInfoW(
            PCWSTR(wide.as_ptr()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            Some(&raw mut owner),
            None,
            None,
            None,
            &raw mut descriptor,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(PrivateDirError::Inspect {
            path: path.to_path_buf(),
            source: std::io::Error::from_raw_os_error(i32::try_from(status.0).unwrap_or(i32::MAX)),
        });
    }

    let mut text = PWSTR::null();
    // SAFETY: `owner` is a valid SID for as long as `descriptor` is live, which
    // is until the `LocalFree` below.
    let converted = unsafe { ConvertSidToStringSidW(owner, &raw mut text) };
    let rendered = converted.map(|()| {
        // SAFETY: on success Win32 wrote a NUL-terminated UTF-16 string.
        let sid = unsafe { text.to_string() };
        let _ = unsafe { LocalFree(Some(HLOCAL(text.0.cast()))) };
        sid
    });
    let _ = unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };

    match rendered {
        Ok(Ok(sid)) => Ok(sid),
        Ok(Err(_)) | Err(_) => Err(PrivateDirError::Inspect {
            path: path.to_path_buf(),
            source: std::io::Error::other("the owner SID could not be rendered"),
        }),
    }
}

#[cfg(test)]
mod tests;
