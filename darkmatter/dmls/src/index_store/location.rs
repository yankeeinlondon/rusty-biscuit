//! Where stores live, how they are named, and how stale ones are swept.
//!
//! One store per effective workspace root:
//! `<store dir>/<root key>-f<FACTS_VERSION>.sqlite`, plus SQLite's `-wal` and
//! `-shm` and the liveness lock file `.lock`. The store directory is
//! `<user cache dir>/darkmatter/dmls/`, or the `DMLS_CACHE_DIR` override. The
//! directory is always safe to delete by hand; the store is only a cache.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::FACTS_VERSION;
use super::lock::StoreLock;

/// How long a store may go untouched before a sweep deletes it.
pub const STALE_STORE_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// The suffixes of a store's sibling files, the store itself first.
const STORE_SUFFIXES: [&str; 4] = ["", "-wal", "-shm", ".lock"];

/// Where this process keeps its stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreLocation {
    /// The store directory, which exists.
    Directory(PathBuf),
    /// `DMLS_CACHE_DIR=off`: memory for the process lifetime, no store file.
    Disabled,
    /// No usable directory: memory, for the stated reason.
    Unavailable(String),
}

impl StoreLocation {
    /// Resolves the store directory from the `DMLS_CACHE_DIR` value and the
    /// user cache directory, creating a missing directory (`0700` on Unix).
    ///
    /// | `DMLS_CACHE_DIR` | Location |
    /// | --- | --- |
    /// | unset or empty | `<user_cache_dir>/darkmatter/dmls`, or unavailable when there is no user cache directory |
    /// | `off` (any ASCII case) | disabled |
    /// | an absolute path | that directory, created when missing |
    /// | a relative path, or a path naming a file | unavailable: an explicit override that cannot be honored is not ignored |
    pub fn prepare(cache_dir_override: Option<&OsStr>, user_cache_dir: Option<PathBuf>) -> Self {
        match cache_dir_override.filter(|value| !value.is_empty()) {
            Some(value) if value.eq_ignore_ascii_case("off") => Self::Disabled,
            Some(value) => {
                let path = PathBuf::from(value);
                if !path.is_absolute() {
                    return Self::Unavailable(format!("DMLS_CACHE_DIR is not an absolute path: {}", path.display()));
                }
                Self::ensure_directory(path)
            }
            None => match user_cache_dir {
                Some(cache) => Self::ensure_directory(cache.join("darkmatter").join("dmls")),
                None => Self::Unavailable("no user cache directory".to_string()),
            },
        }
    }

    fn ensure_directory(path: PathBuf) -> Self {
        if path.is_dir() {
            return Self::Directory(path);
        }
        if path.exists() {
            return Self::Unavailable(format!("store directory is not a directory: {}", path.display()));
        }
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&path) {
            Ok(()) => Self::Directory(path),
            Err(error) => Self::Unavailable(format!("cannot create store directory {}: {error}", path.display())),
        }
    }
}

/// The store key of `root`: the xxHash of its simplified canonical path, as
/// 16 hex digits. A root that cannot be canonicalized keys as given.
pub fn root_key(root: &Path) -> String {
    let canonical = biscuit_file::canonicalize_simplified(root).unwrap_or_else(|_| root.to_path_buf());
    format!("{:016x}", biscuit_hash::xx_hash_bytes(canonical.as_os_str().as_encoded_bytes()))
}

/// The store file for `root` inside the store directory `dir`.
pub fn store_path(dir: &Path, root: &Path) -> PathBuf {
    dir.join(format!("{}-f{FACTS_VERSION}.sqlite", root_key(root)))
}

/// The liveness lock file of the store at `store`.
pub fn lock_path(store: &Path) -> PathBuf {
    sibling(store, ".lock")
}

/// `store` with `suffix` appended to its file name.
pub(super) fn sibling(store: &Path, suffix: &str) -> PathBuf {
    let mut name = store.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

/// Sets `path`'s modification time to `now`.
///
/// The file is opened for write: on Windows setting a time through a
/// read-only handle fails with os error 5.
pub fn touch_mtime(path: &Path, now: SystemTime) -> io::Result<()> {
    fs::OpenOptions::new().write(true).open(path)?.set_modified(now)
}

/// What one sweep did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SweepReport {
    /// Stale stores deleted.
    pub removed: usize,
    /// Stale stores kept because a process holds them.
    pub skipped_in_use: usize,
}

/// Deletes every store in `dir` (all its sibling files) whose newest file
/// modification time is at least `max_age` before `now`.
///
/// A store is kept when another holder has its lock file, or, on Windows,
/// when a delete fails because a file is open (os error 32). A store is never
/// deleted because its root is missing: an unmounted volume would otherwise
/// wipe valid stores.
///
/// ## Errors
///
/// Only when `dir` cannot be listed.
pub fn sweep_stale_stores(dir: &Path, max_age: Duration, now: SystemTime) -> io::Result<SweepReport> {
    let mut stores: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let name = entry?.file_name();
        if let Some(store) = name.to_str().and_then(store_file_name) {
            let store = dir.join(store);
            if !stores.contains(&store) {
                stores.push(store);
            }
        }
    }
    stores.sort();

    let mut report = SweepReport::default();
    for store in stores {
        if !is_stale(&store, max_age, now) {
            continue;
        }
        match remove_store(&store) {
            Ok(true) => report.removed += 1,
            Ok(false) => report.skipped_in_use += 1,
            Err(error) => tracing::debug!(store = %store.display(), %error, "index store sweep could not delete a store"),
        }
    }
    Ok(report)
}

/// The store file name a sibling file name belongs to, when it is one of
/// `<16 hex>-f<digits>.sqlite` and its suffixes.
fn store_file_name(name: &str) -> Option<String> {
    let base_end = name.find(".sqlite")? + ".sqlite".len();
    let (base, suffix) = name.split_at(base_end);
    if !STORE_SUFFIXES.contains(&suffix) {
        return None;
    }
    let stem = base.strip_suffix(".sqlite")?;
    let (key, version) = stem.split_once("-f")?;
    let well_formed = key.len() == 16
        && key.bytes().all(|byte| byte.is_ascii_hexdigit())
        && !version.is_empty()
        && version.bytes().all(|byte| byte.is_ascii_digit());
    well_formed.then(|| base.to_string())
}

/// Whether every present file of `store` is at least `max_age` old. A file
/// whose time cannot be read keeps the store.
fn is_stale(store: &Path, max_age: Duration, now: SystemTime) -> bool {
    let mut newest: Option<SystemTime> = None;
    for suffix in STORE_SUFFIXES {
        match fs::metadata(sibling(store, suffix)).and_then(|meta| meta.modified()) {
            Ok(modified) => newest = newest.max(Some(modified)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return false,
        }
    }
    match newest {
        Some(newest) => now.duration_since(newest).is_ok_and(|age| age >= max_age),
        None => false,
    }
}

/// Deletes the store's files under its exclusive lock.
///
/// ## Returns
///
/// `false` when the store is in use and nothing was deleted.
pub(super) fn remove_store(store: &Path) -> io::Result<bool> {
    let lock_file = lock_path(store);
    let Some(lock) = StoreLock::try_exclusive(&lock_file)? else {
        return Ok(false);
    };
    if !remove_database_files(store)? {
        return Ok(false);
    }
    // Deleted while held, so no other process can lock it in between.
    let removed = remove_if_present(lock.path());
    drop(lock);
    removed.map(|_| true)
}

/// Deletes the database, `-wal`, and `-shm` files of `store`.
///
/// ## Returns
///
/// `false` when Windows refuses a delete because the file is open (os error
/// 32): the store is in use.
pub(super) fn remove_database_files(store: &Path) -> io::Result<bool> {
    for suffix in &STORE_SUFFIXES[..3] {
        match remove_if_present(&sibling(store, suffix)) {
            Ok(()) => {}
            Err(error) if is_sharing_violation(&error) => return Ok(false),
            Err(error) => return Err(error),
        }
    }
    Ok(true)
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Whether `error` is Windows' "file in use by another process" (os error
/// 32). Its [`io::ErrorKind`] is uncategorized, so the raw code is matched.
fn is_sharing_violation(error: &io::Error) -> bool {
    cfg!(windows) && error.raw_os_error() == Some(32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sibling_names_map_to_their_store_and_others_are_ignored() {
        let base = "3f9a0c1d2e4b5a69-f1.sqlite";
        for suffix in STORE_SUFFIXES {
            assert_eq!(store_file_name(&format!("{base}{suffix}")).as_deref(), Some(base));
        }
        for other in ["notes.sqlite", "3f9a0c1d2e4b5a69-f1.sqlite-journal", "3f9a0c1d2e4b5a6-f1.sqlite", "3f9a0c1d2e4b5a69-fx.sqlite"] {
            assert_eq!(store_file_name(other), None, "{other}");
        }
    }
}
