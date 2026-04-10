use rayon::prelude::*;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;
use tracing::{debug, instrument, trace};
use which::which;

/// Finds a program by name in the system PATH.
///
/// Uses the `which` crate for cross-platform executable discovery,
/// handling Windows extensions (.exe, .cmd, etc.) automatically.
///
/// ## Returns
///
/// - `Some(PathBuf)` - The canonical path to the executable if found
/// - `None` - If the program is not found in PATH
///
/// ## Examples
///
/// ```
/// use sniff::programs::find_program::find_program;
///
/// if let Some(path) = find_program("git") {
///     println!("git found at: {}", path.display());
/// }
/// ```
pub fn find_program<P: AsRef<OsStr>>(program: P) -> Option<PathBuf> {
    which(program).ok()
}

/// Checks for the existence of multiple programs in parallel.
/// Returns a HashMap where keys are program names and values are paths (if found).
#[instrument(skip_all, fields(program_count = programs.len()))]
pub fn find_programs_parallel(programs: &[&str]) -> HashMap<String, Option<PathBuf>> {
    programs
        .par_iter() // 1. Convert to a parallel iterator
        .map(|&prog| {
            // 2. Perform the check synchronously on a Rayon thread
            (prog.to_string(), which(prog).ok())
        })
        .collect() // 3. Collect results back into a HashMap
}

use super::macos_bundle::find_macos_app_bundle;
#[cfg(target_os = "macos")]
use super::macos_bundle::get_app_bundle_name;
use super::types::ExecutableSource;

/// Pre-built index of executables available on PATH and macOS app bundles.
///
/// Built once during system detection, then used for O(1) lookups instead of
/// per-program filesystem traversal. Significantly reduces overhead when detecting
/// multiple programs in parallel.
///
/// ## Performance
///
/// Building the index scans all PATH directories once (typically 10-20 dirs) and
/// all known macOS app bundles (15-20 apps on macOS). Subsequent lookups are O(1)
/// HashMap accesses instead of repeated filesystem scans.
///
/// ## Examples
///
/// ```no_run
/// use sniff::programs::find_program::ExecutableIndex;
///
/// let index = ExecutableIndex::build();
/// if let Some((path, source)) = index.find_with_source("git") {
///     println!("Found git at {}", path.display());
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ExecutableIndex {
    /// Maps binary name to resolved path (first occurrence wins = PATH precedence)
    path_executables: HashMap<String, PathBuf>,
    /// Maps binary name to app bundle path (macOS only)
    #[cfg(target_os = "macos")]
    bundle_executables: HashMap<String, PathBuf>,
}

impl ExecutableIndex {
    /// Build the index by scanning all PATH directories and macOS app bundles once.
    ///
    /// ## Process
    ///
    /// 1. Scans all directories in the PATH environment variable
    /// 2. Indexes all executables (first occurrence wins for PATH precedence)
    /// 3. On macOS: scans known app bundles in /Applications and ~/Applications
    ///
    /// ## Returns
    ///
    /// A fully populated index ready for O(1) lookups.
    #[instrument(skip_all)]
    pub fn build() -> Self {
        let mut path_executables = HashMap::new();

        if let Some(path_var) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path_var) {
                trace!(dir = %dir.display(), "scanning PATH directory");
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();

                        // Only index executable files (matches `which` semantics)
                        if !is_executable(&path) {
                            continue;
                        }

                        if let Some(name) = entry.file_name().to_str() {
                            let name = name.to_string();

                            // First occurrence wins (matches PATH precedence)
                            path_executables
                                .entry(name.clone())
                                .or_insert_with(|| path.clone());

                            // On Windows, also index without the extension so that
                            // a lookup for "git" can match "git.exe"
                            #[cfg(windows)]
                            if let Some(normalized) = strip_windows_ext(&name) {
                                path_executables
                                    .entry(normalized)
                                    .or_insert_with(|| path.clone());
                            }
                        }
                    }
                }
            }
        }

        debug!(path_count = path_executables.len(), "PATH scan complete");

        Self {
            path_executables,
            #[cfg(target_os = "macos")]
            bundle_executables: build_bundle_index(),
        }
    }

    /// Look up a program - checks PATH first, then macOS bundles.
    ///
    /// Matches the behavior of [`find_program_with_source`] but uses the pre-built
    /// index instead of filesystem traversal.
    ///
    /// ## Returns
    ///
    /// - `Some((PathBuf, ExecutableSource))` - Path and how it was found
    /// - `None` - Program not found anywhere
    pub fn find_with_source(&self, program: &str) -> Option<(PathBuf, ExecutableSource)> {
        if let Some(path) = self.path_executables.get(program) {
            return Some((path.clone(), ExecutableSource::Path));
        }

        #[cfg(target_os = "macos")]
        if let Some(path) = self.bundle_executables.get(program) {
            return Some((path.clone(), ExecutableSource::MacOsAppBundle));
        }

        None
    }

    /// Look up a program by PATH only.
    ///
    /// ## Returns
    ///
    /// - `Some(PathBuf)` - Path to the executable if found in PATH
    /// - `None` - Program not found in PATH
    pub fn find(&self, program: &str) -> Option<PathBuf> {
        self.path_executables.get(program).cloned()
    }
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
    let mut index = HashMap::new();

    // List of all known binary names that have app bundle mappings
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
            // Check /Applications first
            let path = PathBuf::from(format!("/Applications/{}.app", app_name));
            if let Some(exec_path) = check_bundle_executable(&path, binary) {
                index.insert(binary.to_string(), exec_path);
                continue;
            }

            // Check ~/Applications
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
fn check_bundle_executable(
    bundle_path: &std::path::Path,
    binary_name: &str,
) -> Option<PathBuf> {
    if !bundle_path.exists() {
        return None;
    }

    // Use the existing find_macos_app_bundle logic by extracting just the app name
    // Actually, we can't easily reuse that without circular logic. Let's check directly.
    let macos_dir = bundle_path.join("Contents").join("MacOS");
    if !macos_dir.is_dir() {
        return None;
    }

    // Try to find an executable in the MacOS directory
    // For simplicity, just check if there's a single executable or one matching the binary name
    if let Ok(entries) = std::fs::read_dir(&macos_dir) {
        let executables: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| is_executable_file(&e.path()))
            .collect();

        // If there's exactly one executable, use it
        if executables.len() == 1 {
            return Some(executables[0].path());
        }

        // Otherwise try to find one matching the binary name
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

/// Checks if a path is an executable file (macOS only).
///
/// Delegates to the shared [`is_executable`] helper.
#[cfg(target_os = "macos")]
fn is_executable_file(path: &std::path::Path) -> bool {
    is_executable(path)
}

/// Finds multiple programs using a pre-built index.
///
/// Uses the pre-built `ExecutableIndex` for O(1) lookups instead of filesystem
/// traversal. Significantly faster than [`find_programs_with_source_parallel`]
/// when checking many programs.
///
/// ## Examples
///
/// ```no_run
/// use sniff::programs::find_program::{ExecutableIndex, find_programs_with_source_from_index};
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
    programs
        .iter()
        .map(|&prog| (prog.to_string(), index.find_with_source(prog)))
        .collect()
}

/// Finds a program and reports its discovery source.
///
/// Searches for a program by name and returns both the path to the executable
/// and how it was discovered (PATH or macOS app bundle).
///
/// ## Search Order
///
/// 1. System PATH (priority) - returns `ExecutableSource::Path`
/// 2. macOS app bundles (fallback on macOS) - returns `ExecutableSource::MacOsAppBundle`
///
/// ## Returns
///
/// - `Some((PathBuf, ExecutableSource))` - Path and how it was found
/// - `None` - Program not found anywhere
///
/// ## Examples
///
/// ```
/// use sniff::programs::find_program::find_program_with_source;
/// use sniff::programs::ExecutableSource;
///
/// if let Some((path, source)) = find_program_with_source("git") {
///     assert_eq!(source, ExecutableSource::Path);
///     println!("Found at: {}", path.display());
/// }
/// ```
pub fn find_program_with_source<P: AsRef<OsStr>>(
    program: P,
) -> Option<(PathBuf, ExecutableSource)> {
    let program_str = program.as_ref().to_string_lossy();

    // Priority 1: Check PATH first
    if let Ok(path) = which(&program) {
        return Some((path, ExecutableSource::Path));
    }

    // Priority 2: Check macOS app bundles (macOS only)
    if let Some(path) = find_macos_app_bundle(&program_str) {
        return Some((path, ExecutableSource::MacOsAppBundle));
    }

    None
}

/// Finds multiple programs in parallel with source information.
///
/// Uses Rayon for parallel execution. Returns a `HashMap` where keys are
/// program names and values are tuples of (path, source) if found.
///
/// ## Search Order (per program)
///
/// 1. System PATH (priority) - returns `ExecutableSource::Path`
/// 2. macOS app bundles (fallback on macOS) - returns `ExecutableSource::MacOsAppBundle`
///
/// ## Examples
///
/// ```no_run
/// use sniff::programs::find_program::find_programs_with_source_parallel;
///
/// let results = find_programs_with_source_parallel(&["git", "code", "vim"]);
/// for (name, result) in &results {
///     if let Some((path, source)) = result {
///         println!("{}: {} ({})", name, path.display(), source);
///     }
/// }
/// ```
pub fn find_programs_with_source_parallel(
    programs: &[&str],
) -> HashMap<String, Option<(PathBuf, ExecutableSource)>> {
    programs
        .par_iter()
        .map(|&prog| (prog.to_string(), find_program_with_source(prog)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================
    // find_program tests
    // ============================================

    #[test]
    fn test_find_program_existing() {
        // "ls" should exist on all Unix-like systems, "cmd" on Windows
        #[cfg(unix)]
        let result = find_program("ls");
        #[cfg(windows)]
        let result = find_program("cmd");

        assert!(result.is_some(), "Common system command should be found");
    }

    #[test]
    fn test_find_program_nonexistent() {
        let result = find_program("__nonexistent_program_xyz_12345__");
        assert!(result.is_none(), "Nonexistent program should return None");
    }

    #[test]
    fn test_find_program_empty_string() {
        let result = find_program("");
        assert!(result.is_none(), "Empty string should return None");
    }

    #[test]
    fn test_find_program_whitespace() {
        let result = find_program("   ");
        assert!(result.is_none(), "Whitespace-only should return None");
    }

    // ============================================
    // find_programs_parallel tests
    // ============================================

    #[test]
    fn test_find_programs_parallel() {
        #[cfg(unix)]
        let programs = &["ls", "cat", "__nonexistent__"];
        #[cfg(windows)]
        let programs = &["cmd", "dir", "__nonexistent__"];

        let results = find_programs_parallel(programs);

        assert_eq!(results.len(), 3);
        assert!(results.get(programs[0]).unwrap().is_some());
        assert!(results.get("__nonexistent__").unwrap().is_none());
    }

    // ============================================
    // find_program_with_source tests
    // ============================================

    #[test]
    fn test_find_program_with_source_path_program() {
        // Test with a program that should be in PATH
        #[cfg(unix)]
        let result = find_program_with_source("ls");
        #[cfg(windows)]
        let result = find_program_with_source("cmd");

        assert!(result.is_some(), "Common system command should be found");
        let (path, source) = result.unwrap();
        assert!(path.exists(), "Path should exist");
        assert_eq!(source, ExecutableSource::Path, "Should be found via PATH");
    }

    #[test]
    fn test_find_program_with_source_nonexistent() {
        let result = find_program_with_source("__nonexistent_program_xyz_12345__");
        assert!(result.is_none(), "Nonexistent program should return None");
    }

    #[test]
    fn test_find_program_with_source_empty_string() {
        let result = find_program_with_source("");
        assert!(result.is_none(), "Empty string should return None");
    }

    #[test]
    fn test_find_program_with_source_path_takes_priority() {
        // If a program is in PATH, it should return Path source even if
        // a macOS app bundle might exist
        #[cfg(unix)]
        let program = "ls";
        #[cfg(windows)]
        let program = "cmd";

        let result = find_program_with_source(program);
        assert!(result.is_some());
        let (_, source) = result.unwrap();
        assert_eq!(
            source,
            ExecutableSource::Path,
            "PATH should take priority over app bundles"
        );
    }

    // macOS-specific tests for app bundle discovery
    #[cfg(target_os = "macos")]
    mod macos_tests {
        use super::*;

        #[test]
        fn test_find_program_with_source_macos_bundle() {
            // Test with VS Code which typically is NOT in PATH but has an app bundle
            // Skip if VS Code is in PATH (user has shell command installed)
            if which("code").is_ok() {
                // VS Code CLI is in PATH, can't test bundle fallback
                return;
            }

            let result = find_program_with_source("code");
            if let Some((path, source)) = result {
                assert_eq!(
                    source,
                    ExecutableSource::MacOsAppBundle,
                    "VS Code without PATH should be found via app bundle"
                );
                assert!(
                    path.to_string_lossy().contains("Visual Studio Code"),
                    "Path should be inside VS Code bundle"
                );
            }
            // If VS Code isn't installed at all, test passes (can't guarantee it's there)
        }

        #[test]
        fn test_find_program_with_source_path_priority_over_bundle() {
            // Test that git (which is in PATH) returns Path source,
            // even though we could theoretically find it elsewhere
            if let Some((_, source)) = find_program_with_source("git") {
                assert_eq!(
                    source,
                    ExecutableSource::Path,
                    "git should be found via PATH, not app bundle"
                );
            }
        }
    }

    // ============================================
    // find_programs_with_source_parallel tests
    // ============================================

    #[test]
    fn test_find_programs_with_source_parallel_basic() {
        #[cfg(unix)]
        let programs = &["ls", "cat"];
        #[cfg(windows)]
        let programs = &["cmd", "dir"];

        let results = find_programs_with_source_parallel(programs);

        assert_eq!(results.len(), 2);
        for prog in programs {
            let result = results.get(*prog).unwrap();
            assert!(result.is_some(), "{} should be found", prog);
            let (path, source) = result.as_ref().unwrap();
            assert!(path.exists(), "Path for {} should exist", prog);
            assert_eq!(
                *source,
                ExecutableSource::Path,
                "{} should be via PATH",
                prog
            );
        }
    }

    #[test]
    fn test_find_programs_with_source_parallel_mixed() {
        #[cfg(unix)]
        let programs = &["ls", "__nonexistent_xyz__"];
        #[cfg(windows)]
        let programs = &["cmd", "__nonexistent_xyz__"];

        let results = find_programs_with_source_parallel(programs);

        assert_eq!(results.len(), 2);
        assert!(
            results.get(programs[0]).unwrap().is_some(),
            "System command should be found"
        );
        assert!(
            results.get("__nonexistent_xyz__").unwrap().is_none(),
            "Nonexistent should not be found"
        );
    }

    #[test]
    fn test_find_programs_with_source_parallel_empty() {
        let results = find_programs_with_source_parallel(&[]);
        assert!(results.is_empty(), "Empty input should return empty map");
    }

    #[test]
    fn test_find_programs_parallel_empty() {
        let results = find_programs_parallel(&[]);
        assert!(results.is_empty(), "Empty input should return empty map");
    }

    #[test]
    fn test_find_programs_parallel_with_empty_string() {
        let programs = &["ls", ""];
        let results = find_programs_parallel(programs);
        assert_eq!(results.len(), 2);
        assert!(
            results.get("").unwrap().is_none(),
            "Empty string should not find anything"
        );
    }

    #[test]
    fn test_find_programs_with_source_parallel_single_item() {
        #[cfg(unix)]
        let programs = &["ls"];
        #[cfg(windows)]
        let programs = &["cmd"];

        let results = find_programs_with_source_parallel(programs);
        assert_eq!(results.len(), 1);
        let result = results.get(programs[0]).unwrap();
        assert!(result.is_some(), "Single common program should be found");
    }

    #[test]
    fn test_find_program_with_source_returns_canonical_path() {
        // Test that the returned path is valid when a program is found
        #[cfg(unix)]
        let program = "ls";
        #[cfg(windows)]
        let program = "cmd";

        if let Some((path, _)) = find_program_with_source(program) {
            assert!(path.is_absolute(), "Path should be absolute");
            assert!(path.exists(), "Path should exist");
        }
    }

    #[cfg(target_os = "macos")]
    mod macos_parallel_tests {
        use super::*;

        #[test]
        fn test_find_programs_with_source_parallel_includes_bundles() {
            // This test verifies that parallel lookup can find both PATH and bundle programs
            // We use "ls" (should be PATH) and check if wezterm exists (might be bundle)
            let programs = &["ls", "wezterm"];

            let results = find_programs_with_source_parallel(programs);

            assert_eq!(results.len(), 2);
            // ls should always be found via PATH
            let ls_result = results.get("ls").unwrap();
            assert!(ls_result.is_some());
            assert_eq!(ls_result.as_ref().unwrap().1, ExecutableSource::Path);

            // wezterm might or might not be installed; if it is, check the source
            if let Some((_, source)) = results.get("wezterm").unwrap() {
                // Either PATH or MacOsAppBundle is valid depending on installation
                assert!(
                    *source == ExecutableSource::Path
                        || *source == ExecutableSource::MacOsAppBundle,
                    "wezterm should be found via PATH or app bundle"
                );
            }
        }
    }

    // ============================================
    // ExecutableIndex parity tests
    // ============================================

    /// Verifies that `ExecutableIndex::find_with_source()` agrees with
    /// `find_program_with_source()` (which delegates to `which`) for
    /// representative programs found on every platform.
    #[test]
    fn test_executable_index_parity_with_which() {
        let index = ExecutableIndex::build();

        // Programs that should exist on every CI / dev machine
        #[cfg(unix)]
        let programs = &["ls", "cat", "sh", "env"];
        #[cfg(windows)]
        let programs = &["cmd", "powershell"];

        for &prog in programs {
            let which_result = find_program_with_source(prog);
            let index_result = index.find_with_source(prog);

            match (&which_result, &index_result) {
                (Some(_), Some(_)) => {
                    // Both found — good
                }
                (None, None) => {
                    // Both agree it's missing — acceptable on exotic systems
                }
                (Some((which_path, _)), None) => {
                    panic!(
                        "which found '{}' at {} but ExecutableIndex missed it",
                        prog,
                        which_path.display()
                    );
                }
                (None, Some((idx_path, _))) => {
                    panic!(
                        "ExecutableIndex found '{}' at {} but which did not — \
                         likely a non-executable file was indexed",
                        prog,
                        idx_path.display()
                    );
                }
            }
        }
    }

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

        // Set permissions to non-executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&non_exec, fs::Permissions::from_mode(0o644)).unwrap();
        }

        // Temporarily prepend our dir to PATH and build the index
        let mut env = ScopedEnv::new();
        let original_path = std::env::var_os("PATH").unwrap_or_default();
        let mut new_path = std::ffi::OsString::from(dir.path());
        new_path.push(":");
        new_path.push(&original_path);
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
        let original_path = std::env::var_os("PATH").unwrap_or_default();
        let mut new_path = std::ffi::OsString::from(dir.path());
        new_path.push(":");
        new_path.push(&original_path);
        env.set_os("PATH", &new_path);

        let index = ExecutableIndex::build();

        assert!(
            index.find("test-exec-prog").is_some(),
            "Executable file should appear in the index"
        );
    }
}
