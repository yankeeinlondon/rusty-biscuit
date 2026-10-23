//! Which deletion failures are worth waiting out.
//!
//! Only an operation that can plausibly become possible is retried: a
//! sharing violation, or a call the OS asks to repeat. Everything the table
//! does not name is [`ErrorClass::Permanent`], so an unfamiliar error ends in
//! one plain sentence instead of a wait that never ends.
//!
//! | OS | Gone | Temporary | Denied | Permanent |
//! | --- | --- | --- | --- | --- |
//! | Unix | `ENOENT` | `EINTR`, `EAGAIN` | `EACCES`, `EPERM` | everything else, `EROFS`, `EIO`, `ENOSPC`, and `EBUSY` included |
//! | Windows | `ERROR_FILE_NOT_FOUND`, `ERROR_PATH_NOT_FOUND` | `ERROR_SHARING_VIOLATION`, `ERROR_LOCK_VIOLATION`, `ERROR_DELETE_PENDING`, `ERROR_USER_MAPPED_FILE` | `ERROR_ACCESS_DENIED` | everything else, `ERROR_WRITE_PROTECT` included |
//!
//! An error without an OS code (an injected one, or one `std` made up) is
//! classified by its [`io::ErrorKind`] the same way.

use std::io;

/// What a failed deletion step means for the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// The entry is already gone, which is what deletion wanted.
    Gone,
    /// Another program holds the entry for now; waiting can clear it.
    Temporary,
    /// The OS refused permission. On Unix the engine makes the directory
    /// writable once and tries again; on Windows a running program's image
    /// is refused this way too, so the engine asks who holds it before
    /// treating it as [`ErrorClass::Permanent`].
    Denied,
    /// Waiting will not help: the disk is read-only or failing, or the error
    /// is not one the table knows.
    Permanent,
}

/// Classifies `error` from one deletion step.
pub fn classify(error: &io::Error) -> ErrorClass {
    match error.raw_os_error() {
        Some(code) => platform::classify(code),
        None => match error.kind() {
            io::ErrorKind::NotFound => ErrorClass::Gone,
            io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock => ErrorClass::Temporary,
            io::ErrorKind::PermissionDenied => ErrorClass::Denied,
            _ => ErrorClass::Permanent,
        },
    }
}

/// The cause of `error` in plain words, finishing the sentence
/// "`<path>` could not be deleted: …".
pub fn plain_cause(error: &io::Error) -> String {
    if let Some(code) = error.raw_os_error()
        && let Some(cause) = platform::plain_cause(code)
    {
        return cause.to_string();
    }
    match error.kind() {
        io::ErrorKind::ReadOnlyFilesystem => "the disk is read-only".to_string(),
        io::ErrorKind::PermissionDenied => "permission was denied".to_string(),
        io::ErrorKind::StorageFull => "the disk is full".to_string(),
        _ => error.to_string(),
    }
}

#[cfg(unix)]
mod platform {
    use super::ErrorClass;

    pub(super) fn classify(code: i32) -> ErrorClass {
        match code {
            libc::ENOENT => ErrorClass::Gone,
            libc::EINTR | libc::EAGAIN => ErrorClass::Temporary,
            libc::EACCES | libc::EPERM => ErrorClass::Denied,
            _ => ErrorClass::Permanent,
        }
    }

    pub(super) fn plain_cause(code: i32) -> Option<&'static str> {
        Some(match code {
            libc::EROFS => "the disk is read-only",
            libc::EACCES | libc::EPERM => "permission was denied",
            libc::EIO => "the storage device reported an error",
            libc::ENOSPC => "the disk is full",
            libc::EBUSY => "the system is using it (it may be a mount point)",
            _ => return None,
        })
    }
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_CRC, ERROR_DELETE_PENDING, ERROR_DISK_FULL, ERROR_FILE_NOT_FOUND,
        ERROR_LOCK_VIOLATION, ERROR_PATH_NOT_FOUND, ERROR_SHARING_VIOLATION, ERROR_USER_MAPPED_FILE,
        ERROR_WRITE_PROTECT,
    };

    use super::ErrorClass;

    pub(super) fn classify(code: i32) -> ErrorClass {
        match code as u32 {
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => ErrorClass::Gone,
            ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION | ERROR_DELETE_PENDING | ERROR_USER_MAPPED_FILE => {
                ErrorClass::Temporary
            }
            ERROR_ACCESS_DENIED => ErrorClass::Denied,
            _ => ErrorClass::Permanent,
        }
    }

    pub(super) fn plain_cause(code: i32) -> Option<&'static str> {
        Some(match code as u32 {
            ERROR_WRITE_PROTECT => "the disk is read-only",
            ERROR_ACCESS_DENIED => "permission was denied",
            ERROR_CRC => "the storage device reported an error",
            ERROR_DISK_FULL => "the disk is full",
            ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION => "another program has it open",
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(code: i32) -> ErrorClass {
        classify(&io::Error::from_raw_os_error(code))
    }

    #[cfg(unix)]
    #[test]
    fn the_unix_table_retries_only_what_the_os_asks_to_repeat() {
        assert_eq!(os(libc::ENOENT), ErrorClass::Gone);
        assert_eq!(os(libc::EINTR), ErrorClass::Temporary);
        assert_eq!(os(libc::EAGAIN), ErrorClass::Temporary);
        assert_eq!(os(libc::EACCES), ErrorClass::Denied);
        assert_eq!(os(libc::EPERM), ErrorClass::Denied);
        for permanent in [libc::EROFS, libc::EIO, libc::ENOSPC, libc::EBUSY, libc::ENOTDIR, libc::ELOOP, libc::EXDEV] {
            assert_eq!(os(permanent), ErrorClass::Permanent, "errno {permanent}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn unix_causes_read_as_plain_words() {
        assert_eq!(plain_cause(&io::Error::from_raw_os_error(libc::EROFS)), "the disk is read-only");
        assert_eq!(plain_cause(&io::Error::from_raw_os_error(libc::EACCES)), "permission was denied");
        assert_eq!(plain_cause(&io::Error::from_raw_os_error(libc::EIO)), "the storage device reported an error");
        assert_eq!(plain_cause(&io::Error::from_raw_os_error(libc::ENOSPC)), "the disk is full");
    }

    #[cfg(windows)]
    #[test]
    fn the_windows_table_waits_on_sharing_and_lock_violations_only() {
        use windows_sys::Win32::Foundation::*;
        let os = |code: u32| os(code as i32);
        assert_eq!(os(ERROR_FILE_NOT_FOUND), ErrorClass::Gone);
        assert_eq!(os(ERROR_PATH_NOT_FOUND), ErrorClass::Gone);
        for temporary in [ERROR_SHARING_VIOLATION, ERROR_LOCK_VIOLATION, ERROR_DELETE_PENDING, ERROR_USER_MAPPED_FILE] {
            assert_eq!(os(temporary), ErrorClass::Temporary, "code {temporary}");
        }
        assert_eq!(os(ERROR_ACCESS_DENIED), ErrorClass::Denied);
        for permanent in [ERROR_WRITE_PROTECT, ERROR_CRC, ERROR_DISK_FULL, ERROR_DIR_NOT_EMPTY, ERROR_INVALID_PARAMETER] {
            assert_eq!(os(permanent), ErrorClass::Permanent, "code {permanent}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_causes_read_as_plain_words() {
        use windows_sys::Win32::Foundation::*;
        let cause = |code: u32| plain_cause(&io::Error::from_raw_os_error(code as i32));
        assert_eq!(cause(ERROR_WRITE_PROTECT), "the disk is read-only");
        assert_eq!(cause(ERROR_ACCESS_DENIED), "permission was denied");
        assert_eq!(cause(ERROR_SHARING_VIOLATION), "another program has it open");
    }

    /// Errors with no OS code: what an injected fault or `std` itself makes.
    #[test]
    fn an_error_without_an_os_code_is_classified_by_its_kind_and_unknown_kinds_are_permanent() {
        let kind = |kind: io::ErrorKind| classify(&io::Error::from(kind));
        assert_eq!(kind(io::ErrorKind::NotFound), ErrorClass::Gone);
        assert_eq!(kind(io::ErrorKind::Interrupted), ErrorClass::Temporary);
        assert_eq!(kind(io::ErrorKind::WouldBlock), ErrorClass::Temporary);
        assert_eq!(kind(io::ErrorKind::PermissionDenied), ErrorClass::Denied);
        for permanent in [io::ErrorKind::ReadOnlyFilesystem, io::ErrorKind::ResourceBusy, io::ErrorKind::Other, io::ErrorKind::InvalidData] {
            assert_eq!(kind(permanent), ErrorClass::Permanent, "{permanent:?}");
        }
        assert_eq!(plain_cause(&io::Error::from(io::ErrorKind::ReadOnlyFilesystem)), "the disk is read-only");
        assert_eq!(plain_cause(&io::Error::other("the controller fell over")), "the controller fell over");
    }
}
