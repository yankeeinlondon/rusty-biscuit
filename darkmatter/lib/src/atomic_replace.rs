//! The one atomic file replacement: a temporary file in the destination's
//! directory renamed over the destination with `NamedTempFile::persist`.
//!
//! Shared by the effect verbs' guarded writes (`effects::fs_write`) and the
//! configuration writer (`config::write`). The destination is never deleted
//! first, and a dropped or refused temporary file removes itself.

use std::path::Path;

/// Renames `tmp` over `target`. On failure the temporary file has already been
/// removed (it is dropped with the error) and `target` is untouched.
#[cfg(not(windows))]
pub(crate) fn persist(
    tmp: tempfile::NamedTempFile,
    target: &Path,
    _sleep: impl FnMut(std::time::Duration),
) -> std::io::Result<()> {
    tmp.persist(target).map(|_| ()).map_err(|error| error.error)
}

/// Replace `target` with `tmp`, retrying the transient refusals Windows gives
/// while another handle has `target` open: a parallel sequence leg reading or
/// replacing the same document, or a host scanner. The replacement stays atomic
/// and the destination is never deleted first.
#[cfg(windows)]
pub(crate) fn persist(
    tmp: tempfile::NamedTempFile,
    target: &Path,
    sleep: impl FnMut(std::time::Duration),
) -> std::io::Result<()> {
    persist_retrying(tmp, target, |file, target| file.persist(target).map(|_| ()), sleep)
}

#[cfg(windows)]
const PERSIST_ATTEMPTS: u32 = 11;

#[cfg(windows)]
fn persist_retrying(
    mut tmp: tempfile::NamedTempFile,
    target: &Path,
    mut persist: impl FnMut(tempfile::NamedTempFile, &Path) -> Result<(), tempfile::PersistError>,
    mut sleep: impl FnMut(std::time::Duration),
) -> std::io::Result<()> {
    use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION};

    let mut delay = std::time::Duration::from_millis(1);
    let mut attempt = 1;
    loop {
        match persist(tmp, target) {
            Ok(()) => return Ok(()),
            Err(error)
                if attempt < PERSIST_ATTEMPTS
                    && matches!(error.error.raw_os_error(), Some(code)
                        if code == ERROR_ACCESS_DENIED.0 as i32
                            || code == ERROR_SHARING_VIOLATION.0 as i32) =>
            {
                tmp = error.file;
                sleep(delay);
                delay *= 2;
                attempt += 1;
            }
            Err(error) => return Err(error.error),
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn refused(code: u32, file: tempfile::NamedTempFile) -> tempfile::PersistError {
        tempfile::PersistError { error: std::io::Error::from_raw_os_error(code as i32), file }
    }

    #[test]
    fn transient_refusals_retry_with_a_bounded_backoff() {
        use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION};

        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("doc.md");
        let mut attempts = 0;
        let mut delays = Vec::new();
        persist_retrying(
            tempfile::NamedTempFile::new_in(dir.path()).unwrap(),
            &target,
            |file, target| {
                attempts += 1;
                match attempts {
                    1 => Err(refused(ERROR_ACCESS_DENIED.0, file)),
                    2 => Err(refused(ERROR_SHARING_VIOLATION.0, file)),
                    _ => file.persist(target).map(|_| ()),
                }
            },
            |delay| delays.push(delay.as_millis()),
        )
        .unwrap();

        assert_eq!(attempts, 3);
        assert_eq!(delays, [1, 2]);
        assert!(target.exists());

        let mut attempts = 0;
        let error = persist_retrying(
            tempfile::NamedTempFile::new_in(dir.path()).unwrap(),
            &target,
            |file, _| {
                attempts += 1;
                Err(refused(ERROR_ACCESS_DENIED.0, file))
            },
            |_| {},
        )
        .unwrap_err();
        assert_eq!(attempts, PERSIST_ATTEMPTS, "the retry is bounded");
        assert_eq!(error.raw_os_error(), Some(ERROR_ACCESS_DENIED.0 as i32));
    }

    #[test]
    fn a_permanent_refusal_is_not_retried() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut attempts = 0;
        let error = persist_retrying(
            tempfile::NamedTempFile::new_in(dir.path()).unwrap(),
            &dir.path().join("doc.md"),
            |file, _| {
                attempts += 1;
                Err(refused(3, file))
            },
            |_| panic!("a permanent error never sleeps"),
        )
        .unwrap_err();

        assert_eq!(attempts, 1);
        assert_eq!(error.raw_os_error(), Some(3));
    }
}
