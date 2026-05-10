use rayon::prelude::*;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;
use tracing::instrument;
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
        .par_iter()
        .map(|&prog| (prog.to_string(), which(prog).ok()))
        .collect()
}

use super::contract::ExecutableSource;
use super::macos_bundle::find_macos_app_bundle;

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

    // Priority 1: PATH (authoritative).
    if let Ok(path) = which(&program) {
        return Some((path, ExecutableSource::Path));
    }

    // Priority 2: macOS app bundles.
    if let Some(path) = find_macos_app_bundle(&program_str) {
        return Some((path, ExecutableSource::MacOsAppBundle));
    }

    // Priority 3 + 4: Windows fallback layers.
    #[cfg(target_os = "windows")]
    {
        let key = program_str.to_ascii_lowercase();
        let idx = super::windows_apps::get_or_build_windows_index();
        if let Some(path) = idx.app_paths.get(&key) {
            return Some((path.clone(), ExecutableSource::WindowsAppPaths));
        }
        if let Some(path) = idx.install_roots.get(&key) {
            return Some((path.clone(), ExecutableSource::WindowsInstallRoot));
        }
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
            if which("code").is_ok() {
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
        }

        #[test]
        fn test_find_program_with_source_path_priority_over_bundle() {
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
            let programs = &["ls", "wezterm"];

            let results = find_programs_with_source_parallel(programs);

            assert_eq!(results.len(), 2);
            let ls_result = results.get("ls").unwrap();
            assert!(ls_result.is_some());
            assert_eq!(ls_result.as_ref().unwrap().1, ExecutableSource::Path);

            if let Some((_, source)) = results.get("wezterm").unwrap() {
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

    #[test]
    fn test_executable_index_parity_with_which() {
        use crate::executable_index::ExecutableIndex;
        let index = ExecutableIndex::build();

        #[cfg(unix)]
        let programs = &["ls", "cat", "sh", "env"];
        #[cfg(windows)]
        let programs = &["cmd", "powershell"];

        for &prog in programs {
            let which_result = find_program_with_source(prog);
            let index_result = index.find_with_source(prog);

            match (&which_result, &index_result) {
                (Some(_), Some(_)) => {}
                (None, None) => {}
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
}
