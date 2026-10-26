//! How much space the calling user can write on the filesystem holding a path.
//!
//! [`available_space`] is a focused, path-based reading: it resolves the path
//! to the filesystem that actually holds it and asks that filesystem once. It
//! spawns no subprocess and runs no storage-device inventory (see
//! [`crate::hardware`] for that), so it is cheap enough to call before a
//! write.
//!
//! A reading is not a reservation. Another writer, a quota change, or an
//! allocation failure can still make a later write fail, so a caller that
//! checks first must still handle a disk-full error when it writes.
//!
//! See `sniff/docs/topics/available-space.md` for the contract.

use crate::performance::{self, counters};
use std::io;
use std::path::{Path, PathBuf};

/// One available-space reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableSpace {
    /// The path the caller asked about, as given.
    pub requested: PathBuf,
    /// The existing location that was measured: the requested path, or its
    /// nearest existing ancestor, with every symlink, junction, and other
    /// alias resolved.
    pub measured: PathBuf,
    /// Bytes the calling user can write there: POSIX `f_bavail`, or the
    /// caller's quota-aware figure from `GetDiskFreeSpaceExW`. This excludes
    /// blocks reserved for the superuser and can be less than the
    /// filesystem's total free space.
    pub available_bytes: u64,
}

/// A reading that could not be taken.
#[derive(Debug, thiserror::Error)]
pub enum SpaceError {
    /// Neither the path nor any ancestor could be resolved to an existing
    /// location.
    #[error("cannot resolve the location of {}: {source}", path.display())]
    Resolve {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The location resolved, but its filesystem did not report its space.
    #[error(
        "cannot read the available space for {} (measured at {}): {source}",
        path.display(),
        measured.display()
    )]
    Reading {
        path: PathBuf,
        measured: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Reads the space available to the calling user on the filesystem holding
/// `path`.
///
/// `path` need not exist yet: a missing leaf is measured at its nearest
/// existing ancestor, which is where it would be created. The location is
/// found by resolving the path itself, so a symlinked, junctioned, or
/// firmlinked ancestor and a mount point beneath a parent volume each report
/// the filesystem that really holds the path, never one chosen by matching
/// path text against a mount list.
///
/// ## Examples
///
/// ```no_run
/// use sniff::filesystem::space::available_space;
///
/// let reading = available_space(std::path::Path::new("target/new-dir"))?;
/// println!("{} bytes free at {}", reading.available_bytes, reading.measured.display());
/// # Ok::<(), sniff::filesystem::space::SpaceError>(())
/// ```
///
/// ## Errors
///
/// [`SpaceError::Resolve`] when no ancestor of `path` resolves (every one is
/// missing, or one cannot be read), and [`SpaceError::Reading`] when the
/// filesystem's own query fails.
pub fn available_space(path: &Path) -> Result<AvailableSpace, SpaceError> {
    read(path, &mut Native)
}

/// The two native calls a reading needs, so tests can script filesystems.
pub(crate) trait SpaceSource {
    /// Resolves `path` to its canonical existing location.
    fn canonicalize(&mut self, path: &Path) -> io::Result<PathBuf>;

    /// Bytes available to the caller on the filesystem holding `resolved`.
    fn available_bytes(&mut self, resolved: &Path) -> io::Result<u64>;
}

pub(crate) fn read<S: SpaceSource>(
    path: &Path,
    source: &mut S,
) -> Result<AvailableSpace, SpaceError> {
    let measured = resolve_existing(path, source)?;
    performance::increment_counter(counters::FS_SPACE_READINGS, 1);
    let available_bytes = source
        .available_bytes(&measured)
        .map_err(|source| SpaceError::Reading {
            path: path.to_path_buf(),
            measured: measured.clone(),
            source,
        })?;
    Ok(AvailableSpace {
        requested: path.to_path_buf(),
        measured,
        available_bytes,
    })
}

/// Canonicalizes `path`, or the nearest ancestor that exists.
///
/// Only `NotFound` moves up a level: any other failure (permission denied, a
/// symlink loop) means the location cannot be established, and measuring a
/// higher ancestor could name a different filesystem.
fn resolve_existing<S: SpaceSource>(path: &Path, source: &mut S) -> Result<PathBuf, SpaceError> {
    let error = |source| SpaceError::Resolve {
        path: path.to_path_buf(),
        source,
    };
    let mut probe = path;
    loop {
        // `Path::new("dir").parent()` is `""`, which names the current
        // directory for a relative path.
        let spelled = if probe.as_os_str().is_empty() {
            Path::new(".")
        } else {
            probe
        };
        match source.canonicalize(spelled) {
            Ok(resolved) => return Ok(resolved),
            Err(e) if e.kind() == io::ErrorKind::NotFound => match probe.parent() {
                Some(parent) => probe = parent,
                None => return Err(error(e)),
            },
            Err(e) => return Err(error(e)),
        }
    }
}

struct Native;

impl SpaceSource for Native {
    fn canonicalize(&mut self, path: &Path) -> io::Result<PathBuf> {
        performance::increment_counter(counters::FS_CANONICALIZATIONS, 1);
        biscuit_file::canonicalize_simplified(path)
    }

    #[cfg(unix)]
    fn available_bytes(&mut self, resolved: &Path) -> io::Result<u64> {
        use std::os::unix::ffi::OsStrExt;
        let text = std::ffi::CString::new(resolved.as_os_str().as_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        // SAFETY: `text` is NUL-terminated and `stat` is a valid out-pointer
        // that `statvfs` fully initializes on success.
        if unsafe { libc::statvfs(text.as_ptr(), stat.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `statvfs` returned success.
        let stat = unsafe { stat.assume_init() };
        // The field widths differ by platform.
        #[allow(clippy::unnecessary_cast)]
        let (available, fragment, block) =
            (stat.f_bavail as u64, stat.f_frsize as u64, stat.f_bsize as u64);
        // `f_bavail` counts fragments; a filesystem that leaves the fragment
        // size zero counts in blocks.
        let unit = if fragment == 0 { block } else { fragment };
        Ok(available.saturating_mul(unit))
    }

    #[cfg(windows)]
    fn available_bytes(&mut self, resolved: &Path) -> io::Result<u64> {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        use windows::core::PCWSTR;
        // The API takes a directory, and a UNC root needs its trailing
        // separator; both are harmless on a drive path.
        performance::increment_counter(counters::FS_METADATA_PROBES, 1);
        let directory = if std::fs::metadata(resolved)?.is_dir() {
            resolved
        } else {
            resolved.parent().unwrap_or(resolved)
        };
        let mut wide: Vec<u16> = directory.as_os_str().encode_wide().collect();
        if wide.last() != Some(&u16::from(b'\\')) {
            wide.push(u16::from(b'\\'));
        }
        wide.push(0);
        let mut available = 0u64;
        // SAFETY: `wide` is NUL-terminated and outlives the call; `available`
        // is a valid out-pointer.
        unsafe { GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut available), None, None) }
            .map_err(io::Error::from)?;
        Ok(available)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::testing::measure;
    use std::collections::HashMap;

    /// Scripted filesystems: `aliases` maps a spelling to its real location,
    /// `exists` lists real locations, and `filesystems` maps each mount point
    /// to its available bytes (the deepest mount containing a path wins, the
    /// way the kernel resolves it).
    #[derive(Default)]
    struct Script {
        aliases: HashMap<PathBuf, PathBuf>,
        exists: Vec<PathBuf>,
        filesystems: Vec<(PathBuf, io::Result<u64>)>,
        canonicalize_errors: HashMap<PathBuf, io::ErrorKind>,
        canonicalized: Vec<PathBuf>,
        measured: Vec<PathBuf>,
    }

    impl Script {
        fn alias(mut self, from: &str, to: &str) -> Self {
            self.aliases.insert(from.into(), to.into());
            self
        }

        fn existing(mut self, path: &str) -> Self {
            self.exists.push(path.into());
            self
        }

        fn filesystem(mut self, mount: &str, available: io::Result<u64>) -> Self {
            self.filesystems.push((mount.into(), available));
            self
        }
    }

    impl SpaceSource for Script {
        fn canonicalize(&mut self, path: &Path) -> io::Result<PathBuf> {
            self.canonicalized.push(path.to_path_buf());
            if let Some(kind) = self.canonicalize_errors.get(path) {
                return Err(io::Error::from(*kind));
            }
            let mut real = path.to_path_buf();
            for (from, to) in &self.aliases {
                if let Ok(rest) = path.strip_prefix(from) {
                    real = to.join(rest);
                }
            }
            if self.exists.contains(&real) {
                Ok(real)
            } else {
                Err(io::Error::from(io::ErrorKind::NotFound))
            }
        }

        fn available_bytes(&mut self, resolved: &Path) -> io::Result<u64> {
            self.measured.push(resolved.to_path_buf());
            let (_, result) = self
                .filesystems
                .iter()
                .filter(|(mount, _)| resolved.starts_with(mount))
                .max_by_key(|(mount, _)| mount.components().count())
                .expect("a scripted filesystem holds every path");
            match result {
                Ok(bytes) => Ok(*bytes),
                Err(e) => Err(io::Error::new(e.kind(), e.to_string())),
            }
        }
    }

    fn disk() -> Script {
        Script::default()
            .existing("/")
            .existing("/home")
            .existing("/home/dev")
            .existing("/volumes/fast")
            .existing("/volumes/fast/store")
            .filesystem("/", Ok(1_000))
            .filesystem("/volumes/fast", Ok(5_000))
    }

    #[test]
    fn an_existing_path_is_measured_on_its_own_filesystem() {
        let mut source = disk();
        let reading = read(Path::new("/home/dev"), &mut source).unwrap();
        assert_eq!(reading.requested, PathBuf::from("/home/dev"));
        assert_eq!(reading.measured, PathBuf::from("/home/dev"));
        assert_eq!(reading.available_bytes, 1_000);
    }

    #[test]
    fn a_path_under_a_nested_mount_reads_that_mount_not_its_parent_volume() {
        let mut source = disk();
        let reading = read(Path::new("/volumes/fast/store"), &mut source).unwrap();
        assert_eq!(reading.available_bytes, 5_000);
    }

    #[test]
    fn a_store_on_another_filesystem_from_the_checkout_reads_its_own_filesystem() {
        let mut source = disk();
        let checkout = read(Path::new("/home/dev"), &mut source).unwrap();
        let store = read(Path::new("/volumes/fast/store/journal.jsonl"), &mut source).unwrap();
        assert_eq!(checkout.available_bytes, 1_000);
        assert_eq!(store.available_bytes, 5_000);
        assert_eq!(store.measured, PathBuf::from("/volumes/fast/store"));
    }

    #[test]
    fn an_aliased_ancestor_reads_the_filesystem_it_points_to_not_the_one_its_text_names() {
        // `/home/dev/link` is spelled under `/`, but it is a symlink (or
        // junction, or firmlink) onto the fast volume.
        let mut source = disk().alias("/home/dev/link", "/volumes/fast/store");
        let reading = read(Path::new("/home/dev/link/new/journal.jsonl"), &mut source).unwrap();
        assert_eq!(reading.measured, PathBuf::from("/volumes/fast/store"));
        assert_eq!(reading.available_bytes, 5_000);
        assert_eq!(source.measured, vec![PathBuf::from("/volumes/fast/store")]);
    }

    #[test]
    fn a_missing_leaf_is_measured_at_its_nearest_existing_ancestor() {
        let mut source = disk();
        let reading = read(Path::new("/home/dev/a/b/c"), &mut source).unwrap();
        assert_eq!(reading.requested, PathBuf::from("/home/dev/a/b/c"));
        assert_eq!(reading.measured, PathBuf::from("/home/dev"));
        assert_eq!(
            source.canonicalized,
            ["/home/dev/a/b/c", "/home/dev/a/b", "/home/dev/a", "/home/dev"]
                .map(PathBuf::from)
        );
    }

    #[test]
    fn a_relative_path_with_no_existing_component_is_measured_at_the_current_directory() {
        let mut source = disk().alias(".", "/home/dev");
        let reading = read(Path::new("missing"), &mut source).unwrap();
        assert_eq!(reading.measured, PathBuf::from("/home/dev"));
        assert_eq!(source.canonicalized, ["missing", "."].map(PathBuf::from));
    }

    #[test]
    fn a_resolution_failure_other_than_not_found_does_not_climb_to_a_higher_ancestor() {
        let mut source = disk();
        source
            .canonicalize_errors
            .insert("/home/dev/private".into(), io::ErrorKind::PermissionDenied);
        let error = read(Path::new("/home/dev/private/x"), &mut source).unwrap_err();
        let SpaceError::Resolve { path, source: cause } = &error else {
            panic!("expected a resolve error, got {error:?}");
        };
        assert_eq!(path, Path::new("/home/dev/private/x"));
        assert_eq!(cause.kind(), io::ErrorKind::PermissionDenied);
        assert!(source.measured.is_empty(), "no filesystem was read");
        assert!(error.to_string().contains("/home/dev/private/x"), "{error}");
    }

    #[test]
    fn a_path_with_no_existing_ancestor_is_a_resolve_error() {
        let mut source = Script::default().filesystem("/", Ok(1));
        let error = read(Path::new("/gone/x"), &mut source).unwrap_err();
        assert!(
            matches!(&error, SpaceError::Resolve { source, .. } if source.kind() == io::ErrorKind::NotFound),
            "{error:?}"
        );
        assert!(source.measured.is_empty());
    }

    #[test]
    fn a_failed_filesystem_query_is_a_reading_error_naming_both_paths() {
        let mut source = disk().filesystem(
            "/volumes/fast",
            Err(io::Error::other("device not ready")),
        );
        let error = read(Path::new("/volumes/fast/store/new"), &mut source).unwrap_err();
        let SpaceError::Reading { path, measured, source: cause } = &error else {
            panic!("expected a reading error, got {error:?}");
        };
        assert_eq!(path, Path::new("/volumes/fast/store/new"));
        assert_eq!(measured, Path::new("/volumes/fast/store"));
        assert!(cause.to_string().contains("device not ready"));
        let message = error.to_string();
        assert!(message.contains("/volumes/fast/store/new") && message.contains("device not ready"), "{message}");
    }

    #[test]
    fn a_zero_byte_reading_is_reported_as_zero_not_an_error() {
        let mut source = disk().filesystem("/volumes/fast", Ok(0));
        let reading = read(Path::new("/volumes/fast/store"), &mut source).unwrap();
        assert_eq!(reading.available_bytes, 0);
    }

    /// The real native path: one canonicalization per probed level, one
    /// reading, and nothing else: no subprocess (the device inventory spawns
    /// `diskutil` on macOS) and no other counted work.
    #[test]
    fn a_native_reading_spawns_nothing_and_does_only_its_own_work() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("not-yet/journal.jsonl");
        let (reading, counts) = measure(|| available_space(&missing));
        let reading = reading.unwrap();
        assert_eq!(
            reading.measured,
            biscuit_file::canonicalize_simplified(temp.path()).unwrap()
        );
        assert_eq!(counts.get(counters::PROC_SPAWNS), 0, "{:#?}", counts.all());
        assert_eq!(counts.get(counters::FS_SPACE_READINGS), 1);
        assert_eq!(counts.get(counters::FS_CANONICALIZATIONS), 3);
        let expected: &[&str] = if cfg!(windows) {
            &[counters::FS_CANONICALIZATIONS, counters::FS_METADATA_PROBES, counters::FS_SPACE_READINGS]
        } else {
            &[counters::FS_CANONICALIZATIONS, counters::FS_SPACE_READINGS]
        };
        assert_eq!(
            counts.all().keys().map(String::as_str).collect::<Vec<_>>(),
            expected,
            "only the reading's own work is counted"
        );
    }

    #[test]
    fn a_native_reading_of_a_file_measures_the_file_location() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("journal.jsonl");
        std::fs::write(&file, b"x").unwrap();
        let reading = available_space(&file).unwrap();
        assert_eq!(reading.measured, biscuit_file::canonicalize_simplified(&file).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn a_native_reading_through_a_symlinked_ancestor_measures_the_link_target() {
        let target = tempfile::tempdir().unwrap();
        let links = tempfile::tempdir().unwrap();
        let link = links.path().join("store-link");
        std::os::unix::fs::symlink(target.path(), &link).unwrap();
        let reading = available_space(&link.join("missing/leaf")).unwrap();
        assert_eq!(
            reading.measured,
            biscuit_file::canonicalize_simplified(target.path()).unwrap()
        );
    }
}
