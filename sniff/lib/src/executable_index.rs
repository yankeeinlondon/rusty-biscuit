//! Shared executable index for cross-platform program discovery.
//!
//! This module provides [`ExecutableIndex`] as a neutral leaf so that both
//! `programs` and `os` can import it without creating a layer inversion
//! (previously `os` imported from `programs`).

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::{debug, instrument};
use which::which;

use crate::programs::contract::ExecutableSource;

static EAGER_PATH_CACHE: OnceLock<HashMap<OsString, PathBuf>> = OnceLock::new();

#[cfg(target_os = "macos")]
static BUNDLE_INDEX_CACHE: OnceLock<HashMap<String, PathBuf>> = OnceLock::new();

/// How [`ExecutableIndex::build_with`] obtains PATH entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathScan {
    /// Delegate each lookup to `which`, walking PATH once per program name.
    Lazy,
    /// Traverse PATH once up front, then probe an in-memory map.
    Eager,
}

/// Whether [`ExecutableIndex::build_with`] indexes the non-PATH lookup layers
/// (macOS app bundles, Windows App Paths / install roots).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fallbacks {
    Include,
    Exclude,
}

/// Whether the process-global eager PATH cache has been populated.
///
/// Test seam: lets a caller of a bulk detection entry point prove it routed
/// through an eager builder rather than per-name `which` walks.
#[cfg(test)]
pub(crate) fn eager_path_cache_is_populated() -> bool {
    EAGER_PATH_CACHE.get().is_some()
}

/// Lazy lookup index for executables on PATH, macOS app bundles, and Windows
/// fallback layers.
///
/// Unlike the previous eager implementation, this index does **not** scan all
/// PATH directories upfront. Instead, it delegates PATH lookups to the `which`
/// crate on demand, avoiding the startup cost of indexing thousands of
/// executables when only a handful are ever queried.
///
/// macOS app bundles and Windows registry/install-root indexes are still
/// pre-built because they are bounded (tens of entries) and expensive to
/// probe repeatedly.
///
/// ## Performance
///
/// Building the index only scans macOS app bundles (15-20 apps) and Windows
/// fallback sources. PATH is not touched during construction. Each lookup
/// iterates PATH only until the target binary is found, matching the
/// behavior of the `which` crate.
///
/// ## Examples
///
/// ```no_run
/// use sniff::executable_index::ExecutableIndex;
///
/// let index = ExecutableIndex::build();
/// if let Some((path, source)) = index.find_with_source("git") {
///     println!("Found git at {}", path.display());
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ExecutableIndex {
    /// Number of PATH directories available (counted from the environment).
    path_dir_count: usize,
    /// Optional eager PATH index built by [`ExecutableIndex::build_eager_path`].
    ///
    /// When present, lookups consult this map before delegating to `which`,
    /// turning bulk PATH scans (one per program name) into a single up-front
    /// PATH traversal followed by O(1) HashMap probes.
    eager_path: Option<HashMap<OsString, PathBuf>>,
    /// Maps binary name to app bundle path (macOS only).
    #[cfg(target_os = "macos")]
    bundle_executables: HashMap<String, PathBuf>,
    /// Windows-specific fallback index (App Paths + install-root walk).
    #[cfg(target_os = "windows")]
    windows_index: crate::programs::windows_apps::WindowsIndex,
}

impl ExecutableIndex {
    /// Build the index by scanning macOS app bundles and Windows fallback sources.
    ///
    /// PATH is **not** scanned during construction; lookups are delegated to
    /// the `which` crate on demand.
    ///
    /// ## Notes
    ///
    /// macOS bundle discovery is served from a process-global cache, so it runs
    /// at most once per process however many indexes are built.
    #[instrument(skip_all)]
    pub fn build() -> Self {
        Self::build_with(PathScan::Lazy, Fallbacks::Include)
    }

    /// Build an index with lazy PATH lookup only (no bundle or Windows indexes).
    #[instrument(skip_all)]
    pub fn build_path_only() -> Self {
        Self::build_with(PathScan::Lazy, Fallbacks::Exclude)
    }

    /// Build an index that eagerly scans every directory on `PATH`.
    ///
    /// Suitable for bulk detection paths (e.g. `ProgramsInfo::detect`) where
    /// many program names are looked up at once. PATH is traversed once into a
    /// `HashMap<OsString, PathBuf>` so subsequent lookups avoid per-name PATH
    /// scans. Single-shot callers should prefer [`Self::build`] (lazy `which`).
    #[instrument(skip_all)]
    pub fn build_eager_path() -> Self {
        Self::build_with(PathScan::Eager, Fallbacks::Include)
    }

    /// Build an eager PATH index with no bundle or Windows fallback layers.
    ///
    /// The lookup surface is exactly [`Self::build_path_only`]'s — PATH and
    /// nothing else — so results are identical; only the work differs, since
    /// PATH is traversed once rather than once per program name. Bulk callers
    /// that must not resolve a program outside PATH (e.g.
    /// `HostCapabilities::detect`) use this instead of [`Self::build_eager_path`].
    #[instrument(skip_all)]
    pub fn build_eager_path_only() -> Self {
        Self::build_with(PathScan::Eager, Fallbacks::Exclude)
    }

    fn build_with(path_scan: PathScan, fallbacks: Fallbacks) -> Self {
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let _ = fallbacks;

        let path_dir_count = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).count())
            .unwrap_or(0);

        let eager_path = match path_scan {
            PathScan::Eager => {
                let map = EAGER_PATH_CACHE.get_or_init(scan_path_executables).clone();
                debug!(entries = map.len(), "eager PATH index built");
                Some(map)
            }
            PathScan::Lazy => {
                debug!(path_dir_count, "lazy PATH index created");
                None
            }
        };

        Self {
            path_dir_count,
            eager_path,
            #[cfg(target_os = "macos")]
            bundle_executables: match fallbacks {
                Fallbacks::Include => BUNDLE_INDEX_CACHE.get_or_init(build_bundle_index).clone(),
                Fallbacks::Exclude => HashMap::new(),
            },
            #[cfg(target_os = "windows")]
            windows_index: match fallbacks {
                Fallbacks::Include => crate::programs::windows_apps::build_windows_index(),
                Fallbacks::Exclude => crate::programs::windows_apps::WindowsIndex::default(),
            },
        }
    }

    /// Look up `program` in the eager PATH index when one is present.
    ///
    /// Returns `None` if no eager index has been built or the program is not
    /// present in the cached map.
    fn eager_lookup(&self, program: &str) -> Option<&PathBuf> {
        let map = self.eager_path.as_ref()?;
        // Direct match first (Unix executables have no extension).
        if let Some(path) = map.get(OsStr::new(program)) {
            return Some(path);
        }
        // On Windows, PATHEXT-based filenames (e.g. `git.exe`) are stored;
        // probe the bare name plus each PATHEXT extension.
        #[cfg(windows)]
        {
            for ext in get_pathext_extensions() {
                let mut candidate = OsString::from(program);
                candidate.push(&ext);
                if let Some(path) = map.get(candidate.as_os_str()) {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Look up a program - checks PATH first, then macOS bundles / Windows fallbacks.
    ///
    /// PATH lookups are performed on demand via the `which` crate rather than
    /// using a pre-built cache.
    ///
    /// ## Returns
    ///
    /// - `Some((PathBuf, ExecutableSource))` - Path and how it was found
    /// - `None` - Program not found anywhere
    pub fn find_with_source(&self, program: &str) -> Option<(PathBuf, ExecutableSource)> {
        // Layer 1: PATH (authoritative).
        // Use the eager PATH index when present; fall back to `which`.
        if let Some(path) = self.eager_lookup(program) {
            return Some((path.clone(), ExecutableSource::Path));
        }
        if let Ok(path) = which(program) {
            return Some((path, ExecutableSource::Path));
        }

        // Layer 2: macOS bundles.
        #[cfg(target_os = "macos")]
        if let Some(path) = self.bundle_executables.get(program) {
            return Some((path.clone(), ExecutableSource::MacOsAppBundle));
        }

        // Layers 3 + 4: Windows App Paths registry, then shallow install-root walk.
        #[cfg(target_os = "windows")]
        {
            let key = program.to_ascii_lowercase();
            if let Some(path) = self.windows_index.app_paths.get(&key) {
                return Some((path.clone(), ExecutableSource::WindowsAppPaths));
            }
            if let Some(path) = self.windows_index.install_roots.get(&key) {
                return Some((path.clone(), ExecutableSource::WindowsInstallRoot));
            }
        }

        None
    }

    /// Look up a program by PATH only.
    ///
    /// Consults the eager PATH index when one was built; otherwise delegates
    /// to `which` for on-demand PATH lookup.
    ///
    /// ## Returns
    ///
    /// - `Some(PathBuf)` - Path to the executable if found in PATH
    /// - `None` - Program not found in PATH
    pub fn find(&self, program: &str) -> Option<PathBuf> {
        if let Some(path) = self.eager_lookup(program) {
            return Some(path.clone());
        }
        which(program).ok()
    }

    pub fn path_dir_count(&self) -> usize {
        self.path_dir_count
    }

    /// Build an index backed by a synthetic eager PATH map.
    ///
    /// Used only in tests so binary-resolution logic can be exercised without
    /// requiring real monorepo binaries to be installed on the host.
    #[cfg(test)]
    pub fn for_test(entries: std::collections::HashMap<std::ffi::OsString, PathBuf>) -> Self {
        Self {
            path_dir_count: 1,
            eager_path: Some(entries),
            #[cfg(target_os = "macos")]
            bundle_executables: HashMap::new(),
            #[cfg(target_os = "windows")]
            windows_index: crate::programs::windows_apps::WindowsIndex::default(),
        }
    }

    /// Bulk lookup of multiple programs, choosing the optimal strategy.
    ///
    /// When an eager PATH index is present, lookups are sequential (each
    /// is ~50 ns — thread-pool overhead would dominate).  When PATH must
    /// be traversed lazily via `which`, the work is dispatched across the
    /// Rayon thread-pool so the per-program I/O overlaps.
    pub fn find_programs_with_source(
        &self,
        programs: &[&str],
    ) -> HashMap<String, Option<(PathBuf, ExecutableSource)>> {
        if self.eager_path.is_some() {
            // Eager path: O(1) HashMap hits — keep it sequential to avoid
            // parallel task overhead.
            let mut results = HashMap::with_capacity(programs.len());
            for &prog in programs {
                results.insert(prog.to_string(), self.find_with_source(prog));
            }
            results
        } else {
            // Lazy path: each lookup may walk PATH on disk — parallelise.
            use rayon::prelude::*;
            programs
                .par_iter()
                .map(|&prog| (prog.to_string(), self.find_with_source(prog)))
                .collect()
        }
    }
}

/// Scan every directory on `PATH` once and collect executables by filename.
///
/// On Unix, the file's basename is the lookup key. On Windows, the basename
/// (including the PATHEXT extension) is the key, so callers should probe the
/// bare program name plus each PATHEXT extension.
///
/// Earlier PATH entries take precedence: a name already present in the map is
/// not overwritten by a later entry, matching `which`'s left-to-right semantics.
fn scan_path_executables() -> HashMap<OsString, PathBuf> {
    let mut index: HashMap<OsString, PathBuf> = HashMap::new();
    let Some(path_var) = std::env::var_os("PATH") else {
        return index;
    };

    for dir in std::env::split_paths(&path_var) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_executable(&path) {
                continue;
            }
            let Some(name) = path.file_name() else {
                continue;
            };
            index
                .entry(name.to_os_string())
                .or_insert_with(|| path.clone());
        }
    }

    index
}

/// Checks whether a path is an executable file.
///
/// On Unix, verifies both that the path is a regular file and that at least
/// one execute bit is set. On Windows, checks that the file has a PATHEXT
/// extension (read from the environment, falling back to a standard set).
fn is_executable(path: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.is_file()
            && path
                .metadata()
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
    }

    #[cfg(windows)]
    {
        if !path.is_file() {
            return false;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            return false;
        };
        let dot_ext = format!(".{}", ext.to_ascii_lowercase());
        get_pathext_extensions().iter().any(|pe| *pe == dot_ext)
    }
}

/// Returns the lowercased PATHEXT extensions from the environment.
///
/// Falls back to `.exe;.cmd;.bat;.com` when the variable is unset.
#[cfg(windows)]
fn get_pathext_extensions() -> Vec<String> {
    let pathext = std::env::var_os("PATHEXT").unwrap_or(".EXE;.CMD;.BAT;.COM".into());
    pathext
        .to_string_lossy()
        .split(';')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_ascii_lowercase())
        .collect()
}

/// Strips a Windows executable extension, returning the normalized name.
///
/// Returns `Some(stem)` if the filename ends with a PATHEXT extension
/// (read from the environment). Returns `None` if no match.
#[cfg(windows)]
#[allow(dead_code)]
fn strip_windows_ext(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    for ext in &get_pathext_extensions() {
        if lower.ends_with(ext.as_str()) {
            return Some(name[..name.len() - ext.len()].to_string());
        }
    }
    None
}

/// Builds the macOS app bundle index by checking known app mappings.
///
/// For each known binary-to-app mapping, checks if the .app bundle exists in
/// /Applications or ~/Applications and stores the path to the executable.
#[cfg(target_os = "macos")]
fn build_bundle_index() -> HashMap<String, PathBuf> {
    use crate::programs::macos_bundle::get_app_bundle_name;

    let mut index = HashMap::new();

    let known_binaries = [
        // Editors / IDEs
        "code",
        "cursor",
        "zed",
        "subl",
        "sublime",
        "sublime-text",
        // Terminal emulators
        "wezterm",
        "wezterm-gui",
        "alacritty",
        "kitty",
        "iterm2",
        "ghostty",
        "warp",
        "warp-terminal",
        // Browsers
        "brave",
        "brave-browser",
        "chrome",
        "google-chrome",
        "firefox",
        // Media
        "vlc",
        "spotify",
        // Communication
        "slack",
        "discord",
    ];

    for binary in known_binaries {
        if let Some(app_name) = get_app_bundle_name(binary) {
            let path = PathBuf::from(format!("/Applications/{}.app", app_name));
            if let Some(exec_path) = check_bundle_executable(&path, binary) {
                index.insert(binary.to_string(), exec_path);
                continue;
            }

            if let Ok(home) = std::env::var("HOME") {
                let home_path = PathBuf::from(home).join(format!("Applications/{}.app", app_name));
                if let Some(exec_path) = check_bundle_executable(&home_path, binary) {
                    index.insert(binary.to_string(), exec_path);
                }
            }
        }
    }

    index
}

/// Checks if a bundle exists and returns the path to its executable.
#[cfg(target_os = "macos")]
fn check_bundle_executable(bundle_path: &std::path::Path, binary_name: &str) -> Option<PathBuf> {
    if !bundle_path.exists() {
        return None;
    }

    let macos_dir = bundle_path.join("Contents").join("MacOS");
    if !macos_dir.is_dir() {
        return None;
    }

    if let Ok(entries) = std::fs::read_dir(&macos_dir) {
        let executables: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| is_executable(&e.path()))
            .collect();

        if executables.len() == 1 {
            return Some(executables[0].path());
        }

        for entry in executables {
            if let Some(name) = entry.file_name().to_str()
                && name.to_lowercase() == binary_name.to_lowercase()
            {
                return Some(entry.path());
            }
        }
    }

    None
}

/// Finds multiple programs using a pre-built index.
///
/// Uses the pre-built `ExecutableIndex` for O(1) lookups instead of filesystem
/// traversal. Significantly faster than parallel `which` calls
/// when checking many programs.
///
/// ## Examples
///
/// ```no_run
/// use sniff::executable_index::{ExecutableIndex, find_programs_with_source_from_index};
///
/// let index = ExecutableIndex::build();
/// let results = find_programs_with_source_from_index(&index, &["git", "vim", "code"]);
/// for (name, result) in &results {
///     if let Some((path, source)) = result {
///         println!("{}: {} ({})", name, path.display(), source);
///     }
/// }
/// ```
pub fn find_programs_with_source_from_index(
    index: &ExecutableIndex,
    programs: &[&str],
) -> HashMap<String, Option<(PathBuf, ExecutableSource)>> {
    index.find_programs_with_source(programs)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies the index does not include non-executable files.
    #[test]
    fn test_executable_index_excludes_non_executable() {
        use crate::test_helpers::{ENV_MUTEX, ScopedEnv};
        use std::fs;
        use tempfile::tempdir;

        let _lock = ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let non_exec = dir.path().join("not-a-program");
        fs::write(&non_exec, "data").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&non_exec, fs::Permissions::from_mode(0o644)).unwrap();
        }

        let mut env = ScopedEnv::new();
        let new_path = crate::test_helpers::prepend_to_path([dir.path()]);
        env.set_os("PATH", &new_path);

        let index = ExecutableIndex::build();

        assert!(
            index.find("not-a-program").is_none(),
            "Non-executable file should not appear in the index"
        );
    }

    /// Verifies the index *does* include executable files.
    #[cfg(unix)]
    #[test]
    fn test_executable_index_includes_executable() {
        use crate::test_helpers::{ENV_MUTEX, ScopedEnv};
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        use tempfile::tempdir;

        let _lock = ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let exec = dir.path().join("test-exec-prog");
        fs::write(&exec, "#!/bin/sh\ntrue").unwrap();
        fs::set_permissions(&exec, fs::Permissions::from_mode(0o755)).unwrap();

        let mut env = ScopedEnv::new();
        let new_path = crate::test_helpers::prepend_to_path([dir.path()]);
        env.set_os("PATH", &new_path);

        let index = ExecutableIndex::build();

        assert!(
            index.find("test-exec-prog").is_some(),
            "Executable file should appear in the index"
        );
    }

    #[test]
    fn test_eager_path_index_scans_path_directories() {
        use crate::test_helpers::{ENV_MUTEX, ScopedEnv};
        use std::fs;
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;
        use tempfile::tempdir;

        let _lock = ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();

        for name in ["eager-a", "eager-b", "eager-c"] {
            let exec = dir.path().join(name);
            fs::write(&exec, "#!/bin/sh\ntrue").unwrap();
            #[cfg(unix)]
            fs::set_permissions(&exec, fs::Permissions::from_mode(0o755)).unwrap();
            #[cfg(windows)]
            {
                let _ = std::fs::rename(&exec, dir.path().join(format!("{}.exe", name)));
            }
        }

        let mut env = ScopedEnv::new();
        let new_path = crate::test_helpers::prepend_to_path([dir.path()]);
        env.set_os("PATH", &new_path);

        let eager_path = scan_path_executables();
        let index = ExecutableIndex {
            path_dir_count: std::env::var_os("PATH")
                .map(|p| std::env::split_paths(&p).count())
                .unwrap_or(0),
            eager_path: Some(eager_path),
            #[cfg(target_os = "macos")]
            bundle_executables: HashMap::new(),
            #[cfg(target_os = "windows")]
            windows_index: crate::programs::windows_apps::WindowsIndex::default(),
        };

        #[cfg(unix)]
        {
            assert!(
                index.find("eager-a").is_some(),
                "eager-a should be in the eager PATH index"
            );
            assert!(
                index.find("eager-b").is_some(),
                "eager-b should be in the eager PATH index"
            );
            assert!(
                index.find("eager-c").is_some(),
                "eager-c should be in the eager PATH index"
            );
        }
        #[cfg(windows)]
        {
            assert!(
                index.find("eager-a.exe").is_some(),
                "eager-a.exe should be in the eager PATH index"
            );
        }

        assert!(index.path_dir_count() > 0);
    }

    #[test]
    fn test_eager_path_index_respects_path_precedence() {
        use crate::test_helpers::{ENV_MUTEX, ScopedEnv};
        use std::fs;
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;
        use tempfile::tempdir;

        let _lock = ENV_MUTEX.lock().unwrap();
        let dir_first = tempdir().unwrap();
        let dir_second = tempdir().unwrap();

        let exec_first = dir_first.path().join("precedence-test");
        let exec_second = dir_second.path().join("precedence-test");

        fs::write(&exec_first, "#!/bin/sh\necho first").unwrap();
        fs::write(&exec_second, "#!/bin/sh\necho second").unwrap();

        #[cfg(unix)]
        {
            fs::set_permissions(&exec_first, fs::Permissions::from_mode(0o755)).unwrap();
            fs::set_permissions(&exec_second, fs::Permissions::from_mode(0o755)).unwrap();
        }

        let mut env = ScopedEnv::new();
        let new_path = std::env::join_paths([dir_first.path(), dir_second.path()])
            .expect("join PATH dirs");
        env.set_os("PATH", &new_path);

        let eager_path = scan_path_executables();
        let index = ExecutableIndex {
            path_dir_count: 2,
            eager_path: Some(eager_path),
            #[cfg(target_os = "macos")]
            bundle_executables: HashMap::new(),
            #[cfg(target_os = "windows")]
            windows_index: crate::programs::windows_apps::WindowsIndex::default(),
        };

        #[cfg(unix)]
        {
            let found = index.find("precedence-test");
            assert!(found.is_some());
            assert_eq!(
                found.unwrap().parent().unwrap(),
                dir_first.path(),
                "PATH precedence should be respected in eager index"
            );
        }
    }

    #[test]
    fn test_eager_lookup_returns_path_source() {
        use crate::test_helpers::{ENV_MUTEX, ScopedEnv};
        use std::fs;
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;
        use tempfile::tempdir;

        let _lock = ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let exec = dir.path().join("eager-lookup-test");
        fs::write(&exec, "#!/bin/sh\ntrue").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&exec, fs::Permissions::from_mode(0o755)).unwrap();

        let mut env = ScopedEnv::new();
        let new_path = crate::test_helpers::prepend_to_path([dir.path()]);
        env.set_os("PATH", &new_path);

        let eager_path = scan_path_executables();
        let index = ExecutableIndex {
            path_dir_count: std::env::var_os("PATH")
                .map(|p| std::env::split_paths(&p).count())
                .unwrap_or(0),
            eager_path: Some(eager_path),
            #[cfg(target_os = "macos")]
            bundle_executables: HashMap::new(),
            #[cfg(target_os = "windows")]
            windows_index: crate::programs::windows_apps::WindowsIndex::default(),
        };

        #[cfg(unix)]
        {
            let result = index.find_with_source("eager-lookup-test");
            assert!(result.is_some());
            let (path, source) = result.unwrap();
            assert_eq!(source, ExecutableSource::Path);
            assert!(path.ends_with("eager-lookup-test"));
        }
    }

    #[test]
    fn test_lazy_index_does_not_scan_path() {
        let index = ExecutableIndex::build();
        assert!(
            index.eager_path.is_none(),
            "Lazy index should not have an eager PATH map"
        );
    }

    #[test]
    fn test_eager_index_has_populated_map() {
        let index = ExecutableIndex::build_eager_path();
        assert!(
            index.eager_path.is_some(),
            "Eager index should have a PATH map"
        );
    }

    /// PATH must be traversed at most once per process.
    ///
    /// The second build runs under a PATH that shares no directory with the
    /// first, so a re-scan could not reproduce the first map.
    #[test]
    fn eager_path_is_traversed_once_per_process() {
        use crate::test_helpers::{ENV_MUTEX, ScopedEnv};
        use std::fs;
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;
        use tempfile::tempdir;

        let _lock = ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let exec = dir.path().join("scan-once-shim");
        fs::write(&exec, "#!/bin/sh\ntrue").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&exec, fs::Permissions::from_mode(0o755)).unwrap();

        let mut env = ScopedEnv::new();
        env.set_os("PATH", &crate::test_helpers::prepend_to_path([dir.path()]));
        let first = ExecutableIndex::build_eager_path_only();
        let first_map = first
            .eager_path
            .clone()
            .expect("eager builder builds a map");

        let empty = tempdir().unwrap();
        env.set_os(
            "PATH",
            &std::env::join_paths([empty.path()]).expect("join PATH dirs"),
        );
        let second = ExecutableIndex::build_eager_path_only();

        assert_eq!(
            second.eager_path.as_ref(),
            Some(&first_map),
            "second build re-scanned PATH instead of reusing the cached traversal"
        );
        assert!(
            eager_path_cache_is_populated(),
            "eager build must populate the shared PATH cache"
        );
    }

    #[test]
    fn eager_path_only_index_has_no_fallback_layers() {
        let index = ExecutableIndex::build_eager_path_only();
        assert!(
            index.eager_path.is_some(),
            "eager PATH-only index should have a PATH map"
        );
        #[cfg(target_os = "macos")]
        assert!(
            index.bundle_executables.is_empty(),
            "PATH-only index must not resolve programs through app bundles"
        );
        #[cfg(target_os = "windows")]
        {
            assert!(index.windows_index.app_paths.is_empty());
            assert!(index.windows_index.install_roots.is_empty());
        }
    }

    /// Lookups must not change when PATH is scanned eagerly rather than lazily.
    #[cfg(unix)]
    #[test]
    fn eager_path_only_agrees_with_lazy_path_only() {
        use crate::test_helpers::{ENV_MUTEX, ScopedEnv};
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        use tempfile::tempdir;

        let _lock = ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let exec = dir.path().join("agreement-shim");
        fs::write(&exec, "#!/bin/sh\ntrue").unwrap();
        fs::set_permissions(&exec, fs::Permissions::from_mode(0o755)).unwrap();

        let mut env = ScopedEnv::new();
        env.set_os("PATH", &crate::test_helpers::prepend_to_path([dir.path()]));

        let eager = ExecutableIndex::build_eager_path_only();
        let lazy = ExecutableIndex::build_path_only();

        for name in ["agreement-shim", "sh", "no-such-program-anywhere-xyz123"] {
            assert_eq!(
                eager.find_with_source(name).is_some(),
                lazy.find_with_source(name).is_some(),
                "eager and lazy PATH-only indexes disagree on {name}"
            );
        }
    }

    /// Bundle discovery is cache-backed, so it runs at most once per process.
    #[cfg(target_os = "macos")]
    #[test]
    fn bundle_index_is_discovered_once_per_process() {
        let first = ExecutableIndex::build();
        assert!(
            BUNDLE_INDEX_CACHE.get().is_some(),
            "build() must route bundle discovery through the shared cache"
        );

        let second = ExecutableIndex::build();
        let third = ExecutableIndex::build_eager_path();
        assert_eq!(first.bundle_executables, second.bundle_executables);
        assert_eq!(
            first.bundle_executables, third.bundle_executables,
            "build() and build_eager_path() must share one bundle index"
        );
        assert_eq!(
            BUNDLE_INDEX_CACHE.get().map(HashMap::len),
            Some(first.bundle_executables.len())
        );
    }

    #[test]
    fn test_build_eager_path_returns_consistent_results() {
        let first = ExecutableIndex::build_eager_path();
        let second = ExecutableIndex::build_eager_path();

        let first_len = first.eager_path.as_ref().map(|m| m.len());
        let second_len = second.eager_path.as_ref().map(|m| m.len());
        assert_eq!(
            first_len, second_len,
            "Cached build_eager_path should return consistent entry counts"
        );

        if let (Some(map1), Some(map2)) = (&first.eager_path, &second.eager_path) {
            for key in map1.keys() {
                assert!(
                    map2.contains_key(key),
                    "Key {:?} missing from second call",
                    key
                );
            }
        }
    }
}
