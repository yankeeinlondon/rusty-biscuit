//! Shared test utilities.
//!
//! Environment-variable manipulation: `std::env::set_var` and
//! `std::env::remove_var` are unsafe in Rust 2024 (not thread-safe), and Cargo
//! runs tests in parallel by default, so all environment-modifying tests must
//! use a shared mutex to prevent race conditions across modules.
//!
//! Executable fixtures: [`make_executable_fixture`] creates a runnable file
//! the host's executable resolvers will find, encapsulating the Unix mode-bit
//! versus Windows `.cmd`-suffix difference so individual test modules never
//! reach for `std::os::unix::fs::PermissionsExt` directly.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub(crate) static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Create a runnable executable fixture and return the path the host resolver
/// will actually find.
///
/// `base` is the desired fixture path *without* a platform extension. On Unix
/// the file is written verbatim with `0o755` mode bits, so the returned path
/// equals `base`. On Windows the file is created with a `.cmd` extension — a
/// suffix the executable resolvers probe for — and the returned path carries
/// that suffix.
///
/// ## Notes
///
/// Tests should assert against the *returned* path rather than `base`, so the
/// Windows suffix is handled without per-test `cfg`. Centralizing fixture
/// creation here keeps test modules free of `std::os::unix::fs::PermissionsExt`
/// imports that fail to compile on Windows.
pub(crate) fn make_executable_fixture(base: &Path) -> PathBuf {
    std::fs::create_dir_all(base.parent().expect("fixture path has a parent"))
        .expect("create fixture parent dir");
    write_executable_fixture(base)
}

#[cfg(unix)]
fn write_executable_fixture(base: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(base, b"#!/bin/sh\nexit 0\n").expect("write fixture");
    let mut perms = std::fs::metadata(base)
        .expect("fixture metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(base, perms).expect("set fixture mode");
    base.to_path_buf()
}

#[cfg(windows)]
fn write_executable_fixture(base: &Path) -> PathBuf {
    // Windows resolvers probe `.exe`/`.cmd`/`.ps1`; `.cmd` is the cheapest
    // runnable shim to fabricate and is resolved by mere existence.
    let mut name = base
        .file_name()
        .expect("fixture path has a file name")
        .to_os_string();
    name.push(".cmd");
    let path = base.with_file_name(name);
    std::fs::write(&path, b"@echo off\r\nexit /b 0\r\n").expect("write fixture");
    path
}

#[cfg(not(any(unix, windows)))]
fn write_executable_fixture(base: &Path) -> PathBuf {
    std::fs::write(base, b"#!/bin/sh\nexit 0\n").expect("write fixture");
    base.to_path_buf()
}

/// Build a `PATH`-style `OsString` that prepends `extra_dirs` to the current
/// process `PATH`.
///
/// Existing `PATH` entries are parsed with `std::env::split_paths` and the
/// combined list is serialized with `std::env::join_paths`, so callers never
/// hardcode the platform separator (`:` on Unix, `;` on Windows). This keeps
/// PATH-mutating tests compiling and behaving correctly on every target.
///
/// ## Panics
///
/// Panics if any directory contains the platform path separator, which
/// `std::env::join_paths` rejects. Test fixture directories never do.
pub(crate) fn prepend_to_path<I, P>(extra_dirs: I) -> OsString
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let existing = std::env::var_os("PATH").unwrap_or_default();
    let prefix: Vec<PathBuf> = extra_dirs
        .into_iter()
        .map(|p| p.as_ref().to_path_buf())
        .collect();
    let combined = prefix.into_iter().chain(std::env::split_paths(&existing));
    std::env::join_paths(combined).expect("PATH dirs should join without inner separators")
}

pub(crate) struct ScopedEnv {
    vars: Vec<(String, Option<OsString>)>,
}

impl ScopedEnv {
    pub(crate) fn new() -> Self {
        Self { vars: Vec::new() }
    }

    pub(crate) fn set(&mut self, key: &str, value: &str) -> &mut Self {
        let original = std::env::var_os(key);
        self.vars.push((key.to_string(), original));
        unsafe { std::env::set_var(key, value) };
        self
    }

    pub(crate) fn set_os(&mut self, key: &str, value: &OsStr) -> &mut Self {
        let original = std::env::var_os(key);
        self.vars.push((key.to_string(), original));
        unsafe { std::env::set_var(key, value) };
        self
    }

    pub(crate) fn remove(&mut self, key: &str) -> &mut Self {
        let original = std::env::var_os(key);
        self.vars.push((key.to_string(), original));
        unsafe { std::env::remove_var(key) };
        self
    }
}

impl Drop for ScopedEnv {
    fn drop(&mut self) {
        for (key, original) in self.vars.iter().rev() {
            match original {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::{ENV_MUTEX, ScopedEnv, make_executable_fixture, prepend_to_path};

    #[test]
    fn prepend_to_path_round_trips_through_split_paths() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().expect("temp dir");
        let extra = tmp.path().join("extra-bin");

        let mut env = ScopedEnv::new();
        env.set("PATH", "");

        let joined = prepend_to_path([&extra]);
        let parsed: Vec<_> = std::env::split_paths(&joined).collect();
        assert!(
            parsed.contains(&extra),
            "prepended dir should survive a split_paths round-trip"
        );
    }

    #[test]
    fn prepend_to_path_preserves_existing_entries() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().expect("temp dir");
        let existing = tmp.path().join("existing-bin");
        let extra = tmp.path().join("extra-bin");

        let mut env = ScopedEnv::new();
        let existing_path =
            std::env::join_paths([&existing]).expect("join existing dir into a PATH value");
        env.set_os("PATH", &existing_path);

        let joined = prepend_to_path([&extra]);
        let parsed: Vec<_> = std::env::split_paths(&joined).collect();
        assert_eq!(
            parsed.first(),
            Some(&extra),
            "extra dir should be prepended ahead of existing entries"
        );
        assert!(
            parsed.contains(&existing),
            "existing PATH entries should be preserved"
        );
    }

    #[test]
    fn make_executable_fixture_creates_a_resolvable_file() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let base = tmp.path().join("bin").join("runme");
        let created = make_executable_fixture(&base);

        assert!(created.is_file(), "fixture file should exist on disk");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // On Unix the resolver keys off the executable bit at the bare path.
            assert_eq!(created, base);
            let mode = std::fs::metadata(&created)
                .expect("metadata")
                .permissions()
                .mode();
            assert_ne!(mode & 0o111, 0, "fixture should carry the executable bit");
        }

        #[cfg(windows)]
        {
            // On Windows the resolver keys off a runnable extension.
            assert_eq!(created.extension().and_then(|e| e.to_str()), Some("cmd"));
        }
    }

    #[test]
    fn make_executable_fixture_creates_missing_parent_dirs() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let base = tmp.path().join("a").join("b").join("c").join("tool");
        let created = make_executable_fixture(&base);
        assert!(created.is_file());
    }
}
