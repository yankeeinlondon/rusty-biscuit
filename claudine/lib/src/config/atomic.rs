use std::fs;
use std::io::Write;
use std::path::Path;
#[cfg(windows)]
use std::thread;
#[cfg(windows)]
use std::time::Duration;

use tempfile::NamedTempFile;
#[cfg(windows)]
use tempfile::PersistError;
#[cfg(unix)]
use tracing::warn;

#[cfg(windows)]
use windows::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_SHARING_VIOLATION,
};

#[cfg(windows)]
const PERSIST_ATTEMPTS: usize = 8;
#[cfg(windows)]
const INITIAL_PERSIST_RETRY_DELAY: Duration = Duration::from_millis(1);

/// Write content to a file atomically using a sibling temporary file.
///
/// Creates a uniquely named temp file inside the target's parent directory
/// via [`tempfile::NamedTempFile::new_in`], writes and syncs the content, then
/// atomically persists it on the same filesystem. On Unix the parent directory
/// is also synced so the replacement's metadata change survives a power loss.
///
/// ## Concurrency
///
/// Each call uses a unique temp file, so concurrent writers never corrupt each
/// other's in-flight bytes. A successful call leaves the target equal to one
/// complete writer payload. On Windows, the measured transient missing-file,
/// access, and sharing failures during atomic replacement are retried with a
/// bounded backoff.
///
/// ## Errors
///
/// Returns the [`std::io::Error`] raised if the parent directory cannot be
/// created, the temp file cannot be written or synced, or atomic persistence
/// fails. Permanent errors and exhausted Windows retries remain errors. No
/// non-atomic byte-copy fallback is attempted.
///
/// The error type is `io::Error` rather than [`crate::error::ClaudineError`]
/// because every fallible step here *is* an `io::Error`; a wider type would
/// force every caller wanting the real cause to widen with it. Callers
/// returning [`crate::error::Result`] convert for free via `?` and
/// `ClaudineError`'s `#[from] std::io::Error`.
pub fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()> {
    atomic_write_with(path, content, persist_temp_file)
}

fn atomic_write_with<P>(path: &Path, content: &[u8], persist: P) -> std::io::Result<()>
where
    P: FnOnce(NamedTempFile, &Path) -> std::io::Result<()>,
{
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let mut tmp = NamedTempFile::new_in(parent)?;
    tmp.as_file_mut().write_all(content)?;
    tmp.as_file_mut().sync_all()?;

    persist(tmp, path)?;

    #[cfg(unix)]
    fsync_dir(parent);

    Ok(())
}

#[cfg(not(windows))]
fn persist_temp_file(tmp: NamedTempFile, path: &Path) -> std::io::Result<()> {
    tmp.persist(path).map(|_| ()).map_err(|error| error.error)
}

#[cfg(windows)]
fn persist_temp_file(tmp: NamedTempFile, path: &Path) -> std::io::Result<()> {
    persist_with_retry(
        tmp,
        path,
        |file, destination| file.persist(destination).map(|_| ()),
        thread::sleep,
    )
}

#[cfg(windows)]
fn persist_with_retry<P, S>(
    mut tmp: NamedTempFile,
    path: &Path,
    mut persist: P,
    mut sleep: S,
) -> std::io::Result<()>
where
    P: FnMut(NamedTempFile, &Path) -> Result<(), PersistError>,
    S: FnMut(Duration),
{
    let mut delay = INITIAL_PERSIST_RETRY_DELAY;

    for attempt in 1..=PERSIST_ATTEMPTS {
        match persist(tmp, path) {
            Ok(()) => return Ok(()),
            Err(error)
                if attempt < PERSIST_ATTEMPTS
                    && is_transient_windows_persist_error(&error.error) =>
            {
                tmp = error.file;
                sleep(delay);
                delay *= 2;
            }
            Err(error) => return Err(error.error),
        }
    }

    unreachable!("the bounded persist loop always returns")
}

#[cfg(windows)]
fn is_transient_windows_persist_error(error: &std::io::Error) -> bool {
    matches!(
        error.raw_os_error(),
        Some(code)
            if code == ERROR_ACCESS_DENIED.0 as i32
                || code == ERROR_FILE_NOT_FOUND.0 as i32
                || code == ERROR_SHARING_VIOLATION.0 as i32
    )
}

#[cfg(unix)]
fn fsync_dir(dir: &Path) {
    match fs::File::open(dir) {
        Ok(f) => {
            if let Err(err) = f.sync_all() {
                warn!(%err, directory = %dir.display(), "fsync on parent directory failed");
            }
        }
        Err(err) => {
            warn!(%err, directory = %dir.display(), "opening parent directory for fsync failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::sync::{Arc, Barrier};
    use std::thread;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn writes_content_atomically() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("output.json");

        let content = b"hello world";
        atomic_write(&target, content).unwrap();

        assert_eq!(fs::read(&target).unwrap(), content);
    }

    #[test]
    fn creates_parent_directories() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("nested").join("dir").join("file.txt");

        atomic_write(&target, b"test").unwrap();

        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "test");
    }

    #[test]
    fn no_stray_tmp_sibling_after_write() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("config.json");

        atomic_write(&target, b"{}").unwrap();

        let entries: Vec<_> = fs::read_dir(tmp.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, [OsString::from("config.json")]);
    }

    #[test]
    fn overwrites_existing_file() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("settings.json");

        atomic_write(&target, b"original").unwrap();
        atomic_write(&target, b"updated").unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "updated");
    }

    #[test]
    fn concurrent_writers_produce_intact_payload() {
        let tmp = TempDir::new().unwrap();
        let target = Arc::new(tmp.path().join("shared.json"));

        for round in 0..12 {
            let payloads: Vec<Vec<u8>> = (0..8)
                .map(|writer| {
                    format!("payload-{round:02}-{writer:02}-{}", "x".repeat(4096)).into_bytes()
                })
                .collect();
            let barrier = Arc::new(Barrier::new(payloads.len()));

            let handles: Vec<_> = payloads
                .clone()
                .into_iter()
                .map(|payload| {
                    let barrier = Arc::clone(&barrier);
                    let target = Arc::clone(&target);
                    thread::spawn(move || {
                        atomic_write_with(&target, &payload, |tmp, path| {
                            barrier.wait();
                            persist_temp_file(tmp, path)
                        })
                    })
                })
                .collect();

            let joined: Vec<_> = handles.into_iter().map(thread::JoinHandle::join).collect();
            for worker in joined {
                assert!(worker.is_ok(), "atomic-write worker panicked");
                assert!(
                    worker.unwrap().is_ok(),
                    "atomic-write worker returned an error"
                );
            }

            let final_bytes = fs::read(&*target).expect("final file readable");
            assert!(
                payloads.iter().any(|payload| payload == &final_bytes),
                "final content must be exactly one input payload"
            );
        }

        let entries: Vec<_> = fs::read_dir(tmp.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, [OsString::from("shared.json")]);
    }

    #[cfg(windows)]
    fn windows_error(code: windows::Win32::Foundation::WIN32_ERROR) -> std::io::Error {
        std::io::Error::from_raw_os_error(code.0 as i32)
    }

    #[cfg(windows)]
    #[test]
    fn transient_windows_persist_errors_are_narrowly_classified() {
        use windows::Win32::Foundation::{ERROR_LOCK_VIOLATION, ERROR_PATH_NOT_FOUND};

        for (code, expected) in [
            (ERROR_ACCESS_DENIED, true),
            (ERROR_SHARING_VIOLATION, true),
            (ERROR_FILE_NOT_FOUND, true),
            (ERROR_PATH_NOT_FOUND, false),
            (ERROR_LOCK_VIOLATION, false),
        ] {
            assert_eq!(
                is_transient_windows_persist_error(&windows_error(code)),
                expected,
                "unexpected classification for Windows error {}",
                code.0
            );
        }

        assert!(!is_transient_windows_persist_error(
            &std::io::Error::other("not an OS error")
        ));
    }

    #[cfg(windows)]
    #[test]
    fn non_transient_windows_persist_error_is_not_retried() {
        use windows::Win32::Foundation::ERROR_PATH_NOT_FOUND;

        let dir = TempDir::new().unwrap();
        let tmp = NamedTempFile::new_in(dir.path()).unwrap();
        let target = dir.path().join("target.json");
        let mut attempts = 0;
        let mut delays = Vec::new();

        let error = persist_with_retry(
            tmp,
            &target,
            |file, _| {
                attempts += 1;
                Err(PersistError {
                    error: windows_error(ERROR_PATH_NOT_FOUND),
                    file,
                })
            },
            |delay| delays.push(delay),
        )
        .unwrap_err();

        assert_eq!(attempts, 1);
        assert!(delays.is_empty());
        assert_eq!(error.raw_os_error(), Some(ERROR_PATH_NOT_FOUND.0 as i32));
    }

    #[cfg(windows)]
    #[test]
    fn transient_windows_persist_retries_are_bounded() {
        let dir = TempDir::new().unwrap();
        let tmp = NamedTempFile::new_in(dir.path()).unwrap();
        let target = dir.path().join("target.json");
        let mut attempts = 0;
        let mut delays = Vec::new();

        let error = persist_with_retry(
            tmp,
            &target,
            |file, _| {
                attempts += 1;
                Err(PersistError {
                    error: windows_error(ERROR_ACCESS_DENIED),
                    file,
                })
            },
            |delay| delays.push(delay),
        )
        .unwrap_err();

        assert_eq!(attempts, PERSIST_ATTEMPTS);
        assert_eq!(
            delays,
            [1, 2, 4, 8, 16, 32, 64].map(Duration::from_millis)
        );
        assert_eq!(delays.iter().sum::<Duration>(), Duration::from_millis(127));
        assert_eq!(error.raw_os_error(), Some(ERROR_ACCESS_DENIED.0 as i32));
    }

    #[cfg(windows)]
    #[test]
    fn transient_windows_retries_reuse_the_written_temp_file() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.json");
        let payload = b"complete-synced-payload";
        let mut attempts = 0;
        let mut temp_paths = Vec::new();
        let mut delays = Vec::new();

        atomic_write_with(&target, payload, |tmp, destination| {
            persist_with_retry(
                tmp,
                destination,
                |file, path| {
                    attempts += 1;
                    temp_paths.push(file.path().to_owned());
                    match attempts {
                        1 => Err(PersistError {
                            error: windows_error(ERROR_ACCESS_DENIED),
                            file,
                        }),
                        2 => Err(PersistError {
                            error: windows_error(ERROR_SHARING_VIOLATION),
                            file,
                        }),
                        _ => file.persist(path).map(|_| ()),
                    }
                },
                |delay| delays.push(delay),
            )
        })
        .unwrap();

        assert_eq!(attempts, 3);
        assert_eq!(delays, [1, 2].map(Duration::from_millis));
        assert!(temp_paths.windows(2).all(|pair| pair[0] == pair[1]));
        assert_eq!(fs::read(target).unwrap(), payload);
    }
}
