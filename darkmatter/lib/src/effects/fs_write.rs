//! Mutation-root-guarded file writes: atomic replacement and appends.
//!
//! These helpers are the write primitive for the effect verbs
//! (`set_frontmatter`, file/dir mutations) defined in [`super::verbs`].

use crate::effects::EffectError;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Writes `bytes` to `target` atomically (temp file in the same directory +
/// rename), but only if `target` resolves inside `root`. Creates parent
/// directories under `root` as needed.
pub(crate) fn atomic_write_guarded(
    root: &Path,
    target: &Path,
    bytes: &[u8],
) -> Result<(), EffectError> {
    let normalized = normalize_within(root, target)?;
    if let Some(parent) = normalized.parent() {
        std::fs::create_dir_all(parent).map_err(|source| EffectError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut tmp =
        tempfile::NamedTempFile::new_in(normalized.parent().unwrap_or(root)).map_err(|source| {
            EffectError::Io {
                path: normalized.clone(),
                source,
            }
        })?;
    use std::io::Write;
    tmp.write_all(bytes).map_err(|source| EffectError::Io {
        path: normalized.clone(),
        source,
    })?;
    persist(tmp, &normalized, std::thread::sleep).map_err(|source| EffectError::Io {
        path: normalized.clone(),
        source,
    })?;
    Ok(())
}

#[cfg(not(windows))]
fn persist(
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
fn persist(
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

/// Appends `bytes` to `target` through an append-mode handle, but only if
/// `target` resolves inside `root`. Creates the file and its parent directories
/// under `root` as needed.
///
/// The OS positions every append-mode write at the current end of file, so
/// concurrent appenders — parallel sequence members, separate processes — each
/// land their bytes. A read-modify-write replacement loses all but the last
/// writer's line. Unlike [`atomic_write_guarded`], a crash mid-write can leave a
/// partial final line; existing content is never rewritten.
pub(crate) fn append_guarded(root: &Path, target: &Path, bytes: &[u8]) -> Result<(), EffectError> {
    use std::io::Write;

    let normalized = normalize_within(root, target)?;
    if let Some(parent) = normalized.parent() {
        std::fs::create_dir_all(parent).map_err(|source| EffectError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let io_error = |source| EffectError::Io {
        path: normalized.clone(),
        source,
    };
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&normalized)
        .map_err(io_error)?
        .write_all(bytes)
        .map_err(io_error)
}

/// Resolves `target` and verifies it is contained within `root`.
///
/// The deepest existing ancestor is canonicalized so equivalent symlinked
/// spellings compare by filesystem identity and existing symlink escapes are
/// rejected. Any missing tail is checked lexically because mutation targets
/// commonly do not exist yet.
fn normalize_within(root: &Path, target: &Path) -> Result<std::path::PathBuf, EffectError> {
    let joined = if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    };
    let cleaned = lexically_clean(&joined);
    let canonical_root =
        canonicalize_with_missing_tail(root).unwrap_or_else(|| lexically_clean(root));
    let canonical_target =
        canonicalize_with_missing_tail(&cleaned).unwrap_or_else(|| cleaned.clone());
    if !canonical_target.starts_with(&canonical_root) {
        return Err(EffectError::OutsideMutationRoot {
            path: cleaned,
            root: root.to_path_buf(),
        });
    }
    Ok(cleaned)
}

/// Canonicalize the deepest existing ancestor and reattach any missing tail.
///
/// Mutation targets commonly do not exist yet. Canonicalizing the existing
/// prefix still makes symlink aliases (`/var` versus `/private/var` on macOS)
/// comparable and prevents an existing symlink inside the root from redirecting
/// a new child outside it.
fn canonicalize_with_missing_tail(path: &Path) -> Option<PathBuf> {
    let mut ancestor = path;
    let mut tail = Vec::<OsString>::new();
    loop {
        match std::fs::canonicalize(ancestor) {
            Ok(mut canonical) => {
                for component in tail.iter().rev() {
                    canonical.push(component);
                }
                return Some(lexically_clean(&canonical));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tail.push(ancestor.file_name()?.to_os_string());
                ancestor = ancestor.parent()?;
            }
            Err(_) => return None,
        }
    }
}

/// Removes `.` and resolves `..` segments lexically without touching disk.
fn lexically_clean(path: &Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Containment check returning the cleaned in-root path, for verbs that resolve
/// a target before reading or mutating it.
pub(crate) fn ensure_within(
    root: &Path,
    target: &Path,
) -> Result<std::path::PathBuf, EffectError> {
    normalize_within(root, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_inside_root_succeeds_and_outside_is_refused() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let inside = root.join("ok.txt");
        atomic_write_guarded(&root, &inside, b"hi").unwrap();
        assert_eq!(std::fs::read_to_string(&inside).unwrap(), "hi");

        let outside = root.parent().unwrap().join("escape.txt");
        let err = atomic_write_guarded(&root, &outside, b"no").unwrap_err();
        assert!(matches!(
            err,
            crate::effects::EffectError::OutsideMutationRoot { .. }
        ));
    }

    /// A reader holding the document open while it is replaced (a parallel
    /// sequence leg) makes the replace fail with os error 5 on Windows until
    /// the reader closes; the write waits it out instead of failing.
    #[test]
    fn a_replace_waits_out_a_reader_that_holds_the_target_open() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let target = root.join("doc.md");
        std::fs::write(&target, "old").unwrap();
        let reader = std::fs::File::open(&target).unwrap();

        std::thread::scope(|scope| {
            scope.spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(20));
                drop(reader);
            });
            atomic_write_guarded(&root, &target, b"new").unwrap();
        });

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
    }

    #[cfg(windows)]
    fn refused(code: u32, file: tempfile::NamedTempFile) -> tempfile::PersistError {
        tempfile::PersistError { error: std::io::Error::from_raw_os_error(code as i32), file }
    }

    #[cfg(windows)]
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

    #[cfg(windows)]
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

    #[test]
    fn concurrent_appends_keep_every_line() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let log = root.join("logs/events.log");

        std::thread::scope(|scope| {
            for writer in 0..8 {
                let (root, log) = (&root, &log);
                scope.spawn(move || {
                    for line in 0..50 {
                        append_guarded(root, log, format!("{writer}-{line}\n").as_bytes()).unwrap();
                    }
                });
            }
        });

        let mut lines: Vec<String> = std::fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        lines.sort();
        let mut expected: Vec<String> = (0..8)
            .flat_map(|writer| (0..50).map(move |line| format!("{writer}-{line}")))
            .collect();
        expected.sort();
        assert_eq!(lines, expected);
    }

    #[test]
    fn append_keeps_existing_non_utf8_bytes() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let file = root.join("binary.log");
        std::fs::write(&file, [0xff, 0xfe, b'\n']).unwrap();

        append_guarded(&root, &file, b"next\n").unwrap();

        assert_eq!(std::fs::read(&file).unwrap(), b"\xff\xfe\nnext\n");
    }

    #[test]
    fn append_outside_root_is_refused() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        let outside = dir.path().join("escape.log");

        let err = append_guarded(&root, &outside, b"no\n").unwrap_err();

        assert!(matches!(
            err,
            crate::effects::EffectError::OutsideMutationRoot { .. }
        ));
        assert!(!outside.exists());
    }

    #[cfg(unix)]
    #[test]
    fn equivalent_symlinked_root_spellings_are_contained() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::TempDir::new().unwrap();
        let real_root = dir.path().join("real-root");
        let alias_root = dir.path().join("alias-root");
        std::fs::create_dir_all(&real_root).unwrap();
        symlink(&real_root, &alias_root).unwrap();

        let target = real_root.join("nested/new.txt");
        atomic_write_guarded(&alias_root, &target, b"same identity").unwrap();

        assert_eq!(std::fs::read_to_string(target).unwrap(), "same identity");
    }

    #[cfg(unix)]
    #[test]
    fn an_in_root_symlink_cannot_redirect_a_new_file_outside() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("root");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        symlink(&outside, root.join("redirect")).unwrap();

        let target = root.join("redirect/new.txt");
        let error = atomic_write_guarded(&root, &target, b"no").unwrap_err();

        assert!(matches!(error, EffectError::OutsideMutationRoot { .. }));
        assert!(!outside.join("new.txt").exists());
    }
}
