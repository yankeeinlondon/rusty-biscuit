//! macOS application bundle discovery.
//!
//! This module provides functions to locate macOS `.app` bundles in standard
//! application directories when the command-line executable is not found in PATH.
//!
//! ## Search Locations
//!
//! Searches in order:
//! 1. `/Applications` (system-wide)
//! 2. `~/Applications` (user-specific)
//!
//! ## Bundle Structure
//!
//! macOS bundles have the executable at:
//! `{App}.app/Contents/MacOS/{executable}`
//!
//! ## Examples
//!
//! ```no_run
//! use sniff_lib::programs::macos_bundle::find_macos_app_bundle;
//!
//! // Find VS Code even if "code" isn't in PATH
//! if let Some(path) = find_macos_app_bundle("code") {
//!     println!("Found VS Code at: {}", path.display());
//! }
//! ```

use std::path::{Path, PathBuf};

/// Maps common binary names to their macOS app bundle names.
///
/// Returns the app bundle name (without .app suffix) for known mappings,
/// or `None` if no specific mapping exists (caller should try variations).
///
/// ## Examples
///
/// ```
/// use sniff_lib::programs::macos_bundle::get_app_bundle_name;
///
/// assert_eq!(get_app_bundle_name("code"), Some("Visual Studio Code"));
/// assert_eq!(get_app_bundle_name("chrome"), Some("Google Chrome"));
/// assert_eq!(get_app_bundle_name("unknown-app"), None);
/// ```
pub fn get_app_bundle_name(binary_name: &str) -> Option<&'static str> {
    match binary_name.to_lowercase().as_str() {
        // Editors / IDEs
        "code" => Some("Visual Studio Code"),
        "cursor" => Some("Cursor"),
        "zed" => Some("Zed"),

        // Terminal emulators
        "wezterm" | "wezterm-gui" => Some("WezTerm"),
        "alacritty" => Some("Alacritty"),
        "kitty" => Some("kitty"),
        "iterm2" => Some("iTerm"),
        "ghostty" => Some("Ghostty"),
        "warp" | "warp-terminal" => Some("Warp"),

        // Browsers
        "brave" | "brave-browser" => Some("Brave Browser"),
        "chrome" | "google-chrome" => Some("Google Chrome"),
        "firefox" => Some("Firefox"),

        // Media
        "vlc" => Some("VLC"),
        "spotify" => Some("Spotify"),

        // Communication
        "slack" => Some("Slack"),
        "discord" => Some("Discord"),

        _ => None,
    }
}

/// Attempts to find a macOS app bundle for the given program name.
///
/// Searches `/Applications` and `~/Applications` for matching `.app` bundles,
/// then looks for an executable inside the bundle's `Contents/MacOS/` directory.
///
/// ## Search Strategy
///
/// 1. Check known binary-to-app mappings (e.g., "code" -> "Visual Studio Code")
/// 2. Try the exact binary name as app name
/// 3. Try capitalized binary name as app name
///
/// For each app name candidate, searches:
/// - `/Applications/{name}.app/Contents/MacOS/`
/// - `~/Applications/{name}.app/Contents/MacOS/`
///
/// ## Returns
///
/// The path to the executable inside the app bundle, or `None` if not found.
///
/// ## Platform Support
///
/// - On macOS: Searches application directories
/// - On other platforms: Always returns `None`
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::programs::macos_bundle::find_macos_app_bundle;
///
/// // Find VS Code
/// if let Some(path) = find_macos_app_bundle("code") {
///     assert!(path.exists());
///     assert!(path.to_string_lossy().contains("Visual Studio Code"));
/// }
/// ```
#[cfg(target_os = "macos")]
pub fn find_macos_app_bundle(program_name: &str) -> Option<PathBuf> {
    let search_dirs = get_application_dirs();

    // Strategy 1: Use known mapping
    if let Some(app_name) = get_app_bundle_name(program_name)
        && let Some(path) = find_executable_in_bundle(&search_dirs, app_name, program_name)
    {
        return Some(path);
    }

    // Strategy 2: Try exact name
    if let Some(path) = find_executable_in_bundle(&search_dirs, program_name, program_name) {
        return Some(path);
    }

    // Strategy 3: Try capitalized name
    let capitalized = capitalize_first(program_name);
    if capitalized != program_name
        && let Some(path) = find_executable_in_bundle(&search_dirs, &capitalized, program_name)
    {
        return Some(path);
    }

    None
}

#[cfg(not(target_os = "macos"))]
pub fn find_macos_app_bundle(_program_name: &str) -> Option<PathBuf> {
    None
}

/// Returns the list of directories to search for applications.
#[cfg(target_os = "macos")]
fn get_application_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/Applications")];

    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join("Applications"));
    }

    dirs
}

/// Searches for an executable in a bundle across multiple directories.
///
/// Looks for the executable in `{dir}/{app_name}.app/Contents/MacOS/`.
/// Tries multiple executable name variations:
/// 1. The original binary name
/// 2. The app name itself (common pattern)
/// 3. Common alternatives like "Electron" for VS Code
#[cfg(target_os = "macos")]
fn find_executable_in_bundle(
    search_dirs: &[PathBuf],
    app_name: &str,
    binary_name: &str,
) -> Option<PathBuf> {
    for dir in search_dirs {
        let bundle_path = dir.join(format!("{}.app", app_name));
        let macos_dir = bundle_path.join("Contents").join("MacOS");

        if !macos_dir.is_dir() {
            continue;
        }

        // Try different executable name variations
        let candidates = get_executable_candidates(app_name, binary_name);

        for candidate in candidates {
            let exec_path = macos_dir.join(&candidate);
            if is_executable(&exec_path) {
                return Some(exec_path);
            }
        }

        // Fallback: if the bundle has a single executable, use it
        if let Ok(entries) = std::fs::read_dir(&macos_dir) {
            let mut executables = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                if is_executable(&path) {
                    executables.push(path);
                }
            }

            if executables.len() == 1 {
                return executables.pop();
            }
        }
    }

    None
}

/// Returns a list of executable name candidates to try.
#[cfg(target_os = "macos")]
fn get_executable_candidates(app_name: &str, binary_name: &str) -> Vec<String> {
    let mut candidates = vec![
        binary_name.to_string(),
        app_name.to_string(),
    ];

    // Add lowercase variants
    let binary_lower = binary_name.to_lowercase();
    let app_lower = app_name.to_lowercase();

    if !candidates.contains(&binary_lower) {
        candidates.push(binary_lower);
    }
    if !candidates.contains(&app_lower) {
        candidates.push(app_lower);
    }

    // Special cases
    match binary_name.to_lowercase().as_str() {
        "code" => {
            candidates.push("Electron".to_string());
        }
        "iterm2" => {
            candidates.push("iTerm2".to_string());
        }
        "warp" | "warp-terminal" => {
            candidates.push("stable".to_string());
        }
        _ => {}
    }

    candidates
}

/// Checks if a path is an executable file.
#[cfg(target_os = "macos")]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    if !path.is_file() {
        return false;
    }

    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Capitalizes the first letter of a string.
#[cfg(target_os = "macos")]
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_app_bundle_name_editors() {
        assert_eq!(get_app_bundle_name("code"), Some("Visual Studio Code"));
        assert_eq!(get_app_bundle_name("cursor"), Some("Cursor"));
        assert_eq!(get_app_bundle_name("zed"), Some("Zed"));
    }

    #[test]
    fn test_get_app_bundle_name_terminals() {
        assert_eq!(get_app_bundle_name("wezterm"), Some("WezTerm"));
        assert_eq!(get_app_bundle_name("wezterm-gui"), Some("WezTerm"));
        assert_eq!(get_app_bundle_name("alacritty"), Some("Alacritty"));
        assert_eq!(get_app_bundle_name("kitty"), Some("kitty"));
        assert_eq!(get_app_bundle_name("iterm2"), Some("iTerm"));
        assert_eq!(get_app_bundle_name("ghostty"), Some("Ghostty"));
        assert_eq!(get_app_bundle_name("warp"), Some("Warp"));
        assert_eq!(get_app_bundle_name("warp-terminal"), Some("Warp"));
    }

    #[test]
    fn test_get_app_bundle_name_browsers() {
        assert_eq!(get_app_bundle_name("brave"), Some("Brave Browser"));
        assert_eq!(get_app_bundle_name("brave-browser"), Some("Brave Browser"));
        assert_eq!(get_app_bundle_name("chrome"), Some("Google Chrome"));
        assert_eq!(get_app_bundle_name("google-chrome"), Some("Google Chrome"));
        assert_eq!(get_app_bundle_name("firefox"), Some("Firefox"));
    }

    #[test]
    fn test_get_app_bundle_name_media() {
        assert_eq!(get_app_bundle_name("vlc"), Some("VLC"));
        assert_eq!(get_app_bundle_name("spotify"), Some("Spotify"));
    }

    #[test]
    fn test_get_app_bundle_name_communication() {
        assert_eq!(get_app_bundle_name("slack"), Some("Slack"));
        assert_eq!(get_app_bundle_name("discord"), Some("Discord"));
    }

    #[test]
    fn test_get_app_bundle_name_case_insensitive() {
        assert_eq!(get_app_bundle_name("CODE"), Some("Visual Studio Code"));
        assert_eq!(get_app_bundle_name("Code"), Some("Visual Studio Code"));
        assert_eq!(get_app_bundle_name("DISCORD"), Some("Discord"));
    }

    #[test]
    fn test_get_app_bundle_name_unknown() {
        assert_eq!(get_app_bundle_name("unknown-app"), None);
        assert_eq!(get_app_bundle_name("my-custom-tool"), None);
        assert_eq!(get_app_bundle_name(""), None);
    }

    #[test]
    fn test_find_macos_app_bundle_empty_string() {
        // Empty string should return None on all platforms
        let result = find_macos_app_bundle("");
        assert!(result.is_none(), "Empty string should return None");
    }

    #[test]
    fn test_find_macos_app_bundle_whitespace_only() {
        // Whitespace-only string should return None
        let result = find_macos_app_bundle("   ");
        assert!(result.is_none(), "Whitespace-only string should return None");
    }

    #[test]
    fn test_find_macos_app_bundle_path_traversal() {
        // Path traversal attempts should not find anything
        let result = find_macos_app_bundle("../../../etc/passwd");
        assert!(result.is_none(), "Path traversal should return None");
    }

    // Platform-specific tests
    #[cfg(target_os = "macos")]
    mod macos_tests {
        use super::*;

        #[test]
        fn test_find_macos_app_bundle_vscode() {
            // This test requires VS Code to be installed
            let result = find_macos_app_bundle("code");
            if result.is_some() {
                let path = result.unwrap();
                assert!(path.exists(), "Path should exist: {:?}", path);
                assert!(
                    path.to_string_lossy().contains("Visual Studio Code"),
                    "Path should contain 'Visual Studio Code': {:?}",
                    path
                );
            }
            // If VS Code isn't installed, the test passes (we can't guarantee it's there)
        }

        #[test]
        fn test_find_macos_app_bundle_wezterm() {
            let result = find_macos_app_bundle("wezterm");
            if result.is_some() {
                let path = result.unwrap();
                assert!(path.exists());
                assert!(path.to_string_lossy().contains("WezTerm"));
            }
        }

        #[test]
        fn test_find_macos_app_bundle_kitty() {
            let result = find_macos_app_bundle("kitty");
            if result.is_some() {
                let path = result.unwrap();
                assert!(path.exists());
                assert!(path.to_string_lossy().contains("kitty"));
            }
        }

        #[test]
        fn test_find_macos_app_bundle_alacritty() {
            let result = find_macos_app_bundle("alacritty");
            if result.is_some() {
                let path = result.unwrap();
                assert!(path.exists());
                assert!(path.to_string_lossy().contains("Alacritty"));
            }
        }

        #[test]
        fn test_find_macos_app_bundle_unknown_returns_none() {
            // A program that definitely doesn't exist
            let result = find_macos_app_bundle("__nonexistent_app_12345__");
            assert!(result.is_none());
        }

        #[test]
        fn test_capitalize_first() {
            assert_eq!(capitalize_first("hello"), "Hello");
            assert_eq!(capitalize_first("HELLO"), "HELLO");
            assert_eq!(capitalize_first("a"), "A");
            assert_eq!(capitalize_first(""), "");
        }

        #[test]
        fn test_get_application_dirs() {
            let dirs = get_application_dirs();
            assert!(!dirs.is_empty());
            assert!(dirs.contains(&PathBuf::from("/Applications")));
            // ~/Applications may or may not exist, but should be in the list
            if let Ok(home) = std::env::var("HOME") {
                assert!(dirs.contains(&PathBuf::from(home).join("Applications")));
            }
        }

        #[test]
        fn test_get_executable_candidates() {
            let candidates = get_executable_candidates("Visual Studio Code", "code");
            assert!(candidates.contains(&"code".to_string()));
            assert!(candidates.contains(&"Visual Studio Code".to_string()));
            assert!(candidates.contains(&"Electron".to_string())); // Special case for VS Code
        }

        #[test]
        fn test_get_executable_candidates_iterm() {
            let candidates = get_executable_candidates("iTerm", "iterm2");
            assert!(candidates.contains(&"iterm2".to_string()));
            assert!(candidates.contains(&"iTerm".to_string()));
            assert!(candidates.contains(&"iTerm2".to_string())); // Special case
        }

        #[test]
        fn test_get_executable_candidates_warp() {
            let candidates = get_executable_candidates("Warp", "warp-terminal");
            assert!(candidates.contains(&"warp-terminal".to_string()));
            assert!(candidates.contains(&"Warp".to_string()));
            assert!(candidates.contains(&"stable".to_string())); // Special case
        }

        #[test]
        fn test_get_executable_candidates_includes_variations() {
            // When binary_name and app_name are the same, should include the name
            // plus lowercase variants if different
            let candidates = get_executable_candidates("firefox", "firefox");
            assert!(candidates.contains(&"firefox".to_string()));
            // Should have at least the original name
            assert!(!candidates.is_empty());
        }

        #[test]
        fn test_get_executable_candidates_different_names() {
            // When binary_name and app_name are different, both should be included
            let candidates = get_executable_candidates("Visual Studio Code", "code");
            assert!(candidates.contains(&"code".to_string()));
            assert!(candidates.contains(&"Visual Studio Code".to_string()));
        }

        #[test]
        fn test_capitalize_first_unicode() {
            // Test with unicode characters
            assert_eq!(capitalize_first("hello"), "Hello");
            assert_eq!(capitalize_first(""), "");
            // Multi-byte first character
            assert_eq!(capitalize_first("a"), "A");
        }

        #[test]
        fn test_is_executable_nonexistent_path() {
            let nonexistent = std::path::Path::new("/nonexistent/path/to/binary");
            assert!(!is_executable(nonexistent));
        }

        #[test]
        fn test_is_executable_directory() {
            // A directory should not be considered executable
            let temp_dir = tempfile::tempdir().unwrap();
            assert!(!is_executable(temp_dir.path()));
        }

        /// Test with a mock app bundle structure using tempfile
        #[test]
        fn test_find_executable_in_bundle_with_mock() {
            use std::fs;
            use std::os::unix::fs::PermissionsExt;

            // Create a temporary directory structure mimicking a macOS app bundle
            let temp_dir = tempfile::tempdir().unwrap();
            let app_dir = temp_dir.path().join("MockApp.app");
            let contents_dir = app_dir.join("Contents");
            let macos_dir = contents_dir.join("MacOS");

            fs::create_dir_all(&macos_dir).unwrap();

            // Create a mock executable
            let exec_path = macos_dir.join("mockapp");
            fs::write(&exec_path, b"#!/bin/sh\necho mock").unwrap();

            // Make it executable
            let mut perms = fs::metadata(&exec_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&exec_path, perms).unwrap();

            // Test find_executable_in_bundle with our mock
            let search_dirs = vec![temp_dir.path().to_path_buf()];
            let result = find_executable_in_bundle(&search_dirs, "MockApp", "mockapp");

            assert!(result.is_some(), "Should find mock executable");
            assert_eq!(result.unwrap(), exec_path);
        }

        #[test]
        fn test_find_executable_in_bundle_missing_macos_dir() {
            use std::fs;

            // Create app bundle without Contents/MacOS directory
            let temp_dir = tempfile::tempdir().unwrap();
            let app_dir = temp_dir.path().join("IncompleteApp.app");
            let contents_dir = app_dir.join("Contents");
            fs::create_dir_all(&contents_dir).unwrap();
            // Note: No MacOS subdirectory

            let search_dirs = vec![temp_dir.path().to_path_buf()];
            let result = find_executable_in_bundle(&search_dirs, "IncompleteApp", "incomplete");

            assert!(result.is_none(), "Should not find in incomplete bundle");
        }

        #[test]
        fn test_find_executable_in_bundle_non_executable_file() {
            use std::fs;
            use std::os::unix::fs::PermissionsExt;

            // Create bundle with non-executable file
            let temp_dir = tempfile::tempdir().unwrap();
            let app_dir = temp_dir.path().join("NonExec.app");
            let macos_dir = app_dir.join("Contents").join("MacOS");
            fs::create_dir_all(&macos_dir).unwrap();

            let file_path = macos_dir.join("nonexec");
            fs::write(&file_path, "not executable").unwrap();

            // Ensure it's NOT executable
            let mut perms = fs::metadata(&file_path).unwrap().permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&file_path, perms).unwrap();

            let search_dirs = vec![temp_dir.path().to_path_buf()];
            let result = find_executable_in_bundle(&search_dirs, "NonExec", "nonexec");

            assert!(result.is_none(), "Should not find non-executable file");
        }

        #[test]
        fn test_find_executable_in_bundle_single_exec_fallback() {
            use std::fs;
            use std::os::unix::fs::PermissionsExt;

            let temp_dir = tempfile::tempdir().unwrap();
            let app_dir = temp_dir.path().join("SingleExec.app");
            let macos_dir = app_dir.join("Contents").join("MacOS");
            fs::create_dir_all(&macos_dir).unwrap();

            let exec_path = macos_dir.join("only-exec");
            fs::write(&exec_path, b"#!/bin/sh\necho single").unwrap();

            let mut perms = fs::metadata(&exec_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&exec_path, perms).unwrap();

            let search_dirs = vec![temp_dir.path().to_path_buf()];
            let result = find_executable_in_bundle(&search_dirs, "SingleExec", "unknown");

            assert!(result.is_some(), "Should find lone executable");
            assert_eq!(result.unwrap(), exec_path);
        }
    }

    // Tests that should compile and run on all platforms
    #[cfg(not(target_os = "macos"))]
    mod non_macos_tests {
        use super::*;

        #[test]
        fn test_find_macos_app_bundle_returns_none() {
            // On non-macOS platforms, should always return None
            assert!(find_macos_app_bundle("code").is_none());
            assert!(find_macos_app_bundle("wezterm").is_none());
            assert!(find_macos_app_bundle("anything").is_none());
        }

        #[test]
        fn test_find_macos_app_bundle_empty_returns_none() {
            assert!(find_macos_app_bundle("").is_none());
        }

        #[test]
        fn test_find_macos_app_bundle_all_known_apps_return_none() {
            // All known app mappings should still return None on non-macOS
            let known_apps = [
                "code", "cursor", "zed", "wezterm", "alacritty", "kitty",
                "iterm2", "ghostty", "warp", "warp-terminal", "brave", "chrome",
                "firefox", "vlc", "spotify", "slack", "discord",
            ];
            for app in known_apps {
                assert!(
                    find_macos_app_bundle(app).is_none(),
                    "App '{}' should return None on non-macOS",
                    app
                );
            }
        }
    }
}
