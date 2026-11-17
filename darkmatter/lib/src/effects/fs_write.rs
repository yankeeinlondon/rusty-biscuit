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
    crate::atomic_replace::persist(tmp, &normalized, std::thread::sleep).map_err(|source| {
        EffectError::Io {
            path: normalized.clone(),
            source,
        }
    })?;
    Ok(())
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

/// What [`create_new_guarded`] found at its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreateOutcome {
    /// The name was free and an empty file now holds it.
    Created,
    /// Something already holds the name: a file, a directory, or a symlink
    /// (even a dangling one). Nothing was opened, truncated, or followed.
    Existing,
}

/// Reserves `target` as a new empty file, but only if it resolves inside
/// `root`; returns the cleaned in-root path and what was found there.
///
/// The reservation is one exclusive create (`O_EXCL` / `CREATE_NEW`), so of
/// two concurrent callers exactly one sees [`CreateOutcome::Created`], and an
/// entry that appears first is never truncated or replaced. Missing parent
/// directories are created under `root`. The handle is closed before
/// returning, so a Windows program can replace the file afterwards.
///
/// Unlike `EffectEngine::ensure_file`, which checks for the file and then
/// writes it, nothing here can overwrite a name another process claims in
/// between.
pub(crate) fn create_new_guarded(root: &Path, target: &Path) -> Result<(PathBuf, CreateOutcome), EffectError> {
    let normalized = normalize_within(root, target)?;
    if let Some(parent) = normalized.parent() {
        std::fs::create_dir_all(parent).map_err(|source| EffectError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    match std::fs::OpenOptions::new().write(true).create_new(true).open(&normalized) {
        Ok(_created) => Ok((normalized, CreateOutcome::Created)),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok((normalized, CreateOutcome::Existing)),
        // Windows refuses `CREATE_NEW` on an existing directory with "access
        // denied" rather than "already exists"; any entry holding the name is
        // still `Existing`.
        Err(_) if std::fs::symlink_metadata(&normalized).is_ok() => Ok((normalized, CreateOutcome::Existing)),
        Err(source) => Err(EffectError::Io { path: normalized, source }),
    }
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
pub(crate) fn canonicalize_with_missing_tail(path: &Path) -> Option<PathBuf> {
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

    #[test]
    fn create_new_makes_an_empty_file_under_missing_parents() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let target = root.join("a/b/new.md");

        let (path, outcome) = create_new_guarded(&root, &target).unwrap();

        assert_eq!(outcome, CreateOutcome::Created);
        assert_eq!(path, target);
        assert_eq!(std::fs::read(&target).unwrap(), b"");
    }

    #[test]
    fn create_new_leaves_an_existing_file_untouched() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let target = root.join("kept.md");
        std::fs::write(&target, b"keep me\r\n  ").unwrap();

        let (_, outcome) = create_new_guarded(&root, &target).unwrap();

        assert_eq!(outcome, CreateOutcome::Existing);
        assert_eq!(std::fs::read(&target).unwrap(), b"keep me\r\n  ");
    }

    #[test]
    fn create_new_reports_a_directory_as_existing() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let target = root.join("folder");
        std::fs::create_dir(&target).unwrap();

        let (_, outcome) = create_new_guarded(&root, &target).unwrap();

        assert_eq!(outcome, CreateOutcome::Existing);
        assert!(target.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn create_new_does_not_follow_a_symlink_even_a_dangling_one() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let real = root.join("real.md");
        std::fs::write(&real, b"real").unwrap();
        let live = root.join("live.md");
        symlink(&real, &live).unwrap();
        let dangling = root.join("dangling.md");
        symlink(root.join("absent.md"), &dangling).unwrap();

        assert_eq!(create_new_guarded(&root, &live).unwrap().1, CreateOutcome::Existing);
        assert_eq!(create_new_guarded(&root, &dangling).unwrap().1, CreateOutcome::Existing);
        assert_eq!(std::fs::read(&real).unwrap(), b"real");
        assert!(!root.join("absent.md").exists(), "the dangling link's target was not created");
    }

    #[cfg(unix)]
    #[test]
    fn create_new_refuses_a_symlink_that_escapes_the_root_before_creating() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("root");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        symlink(&outside, root.join("redirect")).unwrap();

        let error = create_new_guarded(&root, &root.join("redirect/sub/new.md")).unwrap_err();

        assert!(matches!(error, EffectError::OutsideMutationRoot { .. }));
        assert!(!outside.join("sub").exists(), "no parent directory was created outside");
    }

    #[test]
    fn create_new_outside_the_root_is_refused_and_creates_nothing() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        let outside = dir.path().join("elsewhere/new.md");

        let error = create_new_guarded(&root, &outside).unwrap_err();

        assert!(matches!(error, EffectError::OutsideMutationRoot { .. }));
        assert!(!dir.path().join("elsewhere").exists());
    }

    #[test]
    fn racing_creators_see_exactly_one_created() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let target = root.join("contested/name.md");
        let barrier = std::sync::Barrier::new(2);

        let outcomes: Vec<CreateOutcome> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..2)
                .map(|_| {
                    let (root, target, barrier) = (&root, &target, &barrier);
                    scope.spawn(move || {
                        barrier.wait();
                        create_new_guarded(root, target).unwrap().1
                    })
                })
                .collect();
            handles.into_iter().map(|handle| handle.join().unwrap()).collect()
        });

        assert_eq!(outcomes.iter().filter(|outcome| **outcome == CreateOutcome::Created).count(), 1, "{outcomes:?}");
        assert_eq!(outcomes.iter().filter(|outcome| **outcome == CreateOutcome::Existing).count(), 1, "{outcomes:?}");
    }
}
