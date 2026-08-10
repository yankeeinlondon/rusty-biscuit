//! Terminal configuration file path discovery.
//!
//! This module provides functions for locating terminal emulator configuration
//! files across different operating systems.
//!
//! ## Examples
//!
//! ```
//! use biscuit_terminal::discovery::config_paths::get_terminal_config_path;
//! use biscuit_terminal::discovery::detection::TerminalApp;
//!
//! if let Some(config_path) = get_terminal_config_path(&TerminalApp::Wezterm) {
//!     println!("WezTerm config: {:?}", config_path);
//! }
//! ```

use super::app_metadata::default_config_paths;
use super::detection::TerminalApp;
use std::path::PathBuf;

/// Get the primary default configuration file path for a terminal application.
///
/// Thin back-compat wrapper over the [`app_metadata`](super::app_metadata) seed
/// table: it returns the *first* candidate for the current OS target, expanded
/// but **not** existence-checked. For "what config is actually in use on this
/// host" (env overrides, first existing file, provenance), use
/// [`TerminalApp::get_config_file`](super::detection::TerminalApp::get_config_file).
///
/// Returns `None` when the app has no candidate for the current OS target (e.g.
/// GNOME Terminal is dconf-managed, or the app is unknown), or when the primary
/// candidate's path tokens cannot be resolved.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::config_paths::get_terminal_config_path;
/// use biscuit_terminal::discovery::detection::TerminalApp;
///
/// // Get WezTerm config path
/// let wezterm_config = get_terminal_config_path(&TerminalApp::Wezterm);
///
/// // Unknown terminals return None
/// let unknown = get_terminal_config_path(&TerminalApp::Other("unknown".to_string()));
/// assert!(unknown.is_none());
/// ```
pub fn get_terminal_config_path(app: &TerminalApp) -> Option<PathBuf> {
    let result = get_terminal_config_paths(app).into_iter().next();
    tracing::debug!(path = ?result, app = ?app, "Terminal config file path");
    result
}

/// Get all possible configuration file paths for a terminal application.
///
/// Some terminals support multiple configuration file locations or formats.
/// This function returns all possible paths that might contain configuration.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::config_paths::get_terminal_config_paths;
/// use biscuit_terminal::discovery::detection::TerminalApp;
///
/// // Alacritty can have .toml or .yml config
/// let paths = get_terminal_config_paths(&TerminalApp::Alacritty);
/// ```
pub fn get_terminal_config_paths(app: &TerminalApp) -> Vec<PathBuf> {
    default_config_paths(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wezterm_config_path() {
        let path = get_terminal_config_path(&TerminalApp::Wezterm);
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("wezterm"));
        assert!(path.to_string_lossy().contains(".lua"));
    }

    #[test]
    fn test_kitty_config_path() {
        let path = get_terminal_config_path(&TerminalApp::Kitty);
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            assert!(path.is_some());
            let path = path.unwrap();
            assert!(path.to_string_lossy().contains("kitty"));
            assert!(path.to_string_lossy().contains(".conf"));
        }
        #[cfg(target_os = "windows")]
        {
            assert!(path.is_none(), "Kitty doesn't support Windows");
        }
    }

    #[test]
    fn test_ghostty_config_path() {
        let path = get_terminal_config_path(&TerminalApp::Ghostty);
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            assert!(path.is_some());
            let path = path.unwrap();
            assert!(path.to_string_lossy().contains("ghostty"));
        }
        #[cfg(target_os = "windows")]
        {
            assert!(path.is_none(), "Ghostty doesn't support Windows");
        }
    }

    #[test]
    fn test_alacritty_config_path() {
        let path = get_terminal_config_path(&TerminalApp::Alacritty);
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("alacritty"));
    }

    #[test]
    fn test_alacritty_config_paths_multiple() {
        let paths = get_terminal_config_paths(&TerminalApp::Alacritty);
        assert!(
            paths.len() >= 2,
            "Alacritty should have multiple config paths"
        );
        // Should include both .toml and .yml options
        let has_toml = paths.iter().any(|p| p.to_string_lossy().contains(".toml"));
        let has_yml = paths.iter().any(|p| p.to_string_lossy().contains(".yml"));
        assert!(has_toml, "Should include .toml path");
        assert!(has_yml, "Should include .yml path");
    }

    #[test]
    fn test_iterm2_config_path_macos_only() {
        let path = get_terminal_config_path(&TerminalApp::ITerm2);
        #[cfg(target_os = "macos")]
        {
            assert!(path.is_some());
            let path = path.unwrap();
            assert!(path.to_string_lossy().contains("iterm2"));
            assert!(path.to_string_lossy().contains("plist"));
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert!(path.is_none(), "iTerm2 is macOS-only");
        }
    }

    #[test]
    fn test_apple_terminal_config_path_macos_only() {
        let path = get_terminal_config_path(&TerminalApp::AppleTerminal);
        #[cfg(target_os = "macos")]
        {
            assert!(path.is_some());
            let path = path.unwrap();
            assert!(path.to_string_lossy().contains("Terminal"));
            assert!(path.to_string_lossy().contains("plist"));
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert!(path.is_none(), "Apple Terminal is macOS-only");
        }
    }

    #[test]
    fn test_gnome_terminal_returns_none() {
        // GNOME Terminal uses dconf, not a config file
        let path = get_terminal_config_path(&TerminalApp::GnomeTerminal);
        assert!(path.is_none());
    }

    #[test]
    fn test_konsole_config_path() {
        let _path = get_terminal_config_path(&TerminalApp::Konsole);
        #[cfg(target_os = "linux")]
        {
            assert!(_path.is_some());
            let path = _path.unwrap();
            assert!(path.to_string_lossy().contains("konsole"));
        }
    }

    #[test]
    fn test_foot_config_path() {
        let _path = get_terminal_config_path(&TerminalApp::Foot);
        #[cfg(target_os = "linux")]
        {
            assert!(_path.is_some());
            let path = _path.unwrap();
            assert!(path.to_string_lossy().contains("foot"));
        }
    }

    #[test]
    fn test_contour_config_path() {
        let path = get_terminal_config_path(&TerminalApp::Contour);
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("contour"));
    }

    #[test]
    fn test_vscode_settings_path() {
        let path = get_terminal_config_path(&TerminalApp::VsCode);
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("Code"));
        assert!(path.to_string_lossy().contains("settings.json"));
    }

    #[test]
    fn test_other_terminal_returns_none() {
        let path = get_terminal_config_path(&TerminalApp::Other("unknown".to_string()));
        assert!(path.is_none());
    }

    #[test]
    fn test_wast_returns_none() {
        let path = get_terminal_config_path(&TerminalApp::Wast);
        assert!(path.is_none());
    }

    #[test]
    fn test_warp_config_path() {
        let path = get_terminal_config_path(&TerminalApp::Warp);
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            assert!(path.is_some());
            let path = path.unwrap();
            assert!(path.to_string_lossy().contains(".warp"));
        }
        #[cfg(target_os = "windows")]
        assert!(path.is_none(), "Warp has no Windows config candidate");
    }

    #[test]
    fn test_get_terminal_config_paths_wezterm() {
        // WezTerm now exposes an ordered candidate list (XDG path + ~/.wezterm.lua);
        // the primary candidate is the XDG-based .lua file.
        let paths = get_terminal_config_paths(&TerminalApp::Wezterm);
        assert!(!paths.is_empty());
        assert!(paths[0].to_string_lossy().contains("wezterm"));
        assert!(paths[0].extension().map(|e| e == "lua").unwrap_or(false));
    }

    #[test]
    fn test_get_terminal_config_paths_unknown_terminal() {
        let paths = get_terminal_config_paths(&TerminalApp::Other("unknown".to_string()));
        assert!(paths.is_empty());
    }

    // === Edge case tests ===

    #[test]
    fn test_all_terminal_apps_have_defined_behavior() {
        // Every TerminalApp variant should either return Some or None consistently
        // (should not panic)
        let apps = vec![
            TerminalApp::AppleTerminal,
            TerminalApp::Contour,
            TerminalApp::Foot,
            TerminalApp::GnomeTerminal,
            TerminalApp::Kitty,
            TerminalApp::Alacritty,
            TerminalApp::Wezterm,
            TerminalApp::Konsole,
            TerminalApp::ITerm2,
            TerminalApp::Warp,
            TerminalApp::Ghostty,
            TerminalApp::Wast,
            TerminalApp::VsCode,
            TerminalApp::Other("unknown".to_string()),
            TerminalApp::Other("".to_string()),
            TerminalApp::Other("a".repeat(1000)),
        ];

        for app in &apps {
            // Should not panic
            let _ = get_terminal_config_path(app);
            let _ = get_terminal_config_paths(app);
        }
    }

    #[test]
    fn test_config_paths_are_absolute() {
        let apps_with_paths = [
            TerminalApp::Wezterm,
            TerminalApp::Alacritty,
            TerminalApp::Contour,
            TerminalApp::VsCode,
        ];

        for app in apps_with_paths {
            if let Some(path) = get_terminal_config_path(&app) {
                assert!(
                    path.is_absolute(),
                    "Path for {:?} should be absolute: {:?}",
                    app,
                    path
                );
            }
        }
    }

    #[test]
    fn test_config_paths_have_expected_extensions() {
        // Wezterm uses .lua
        if let Some(path) = get_terminal_config_path(&TerminalApp::Wezterm) {
            assert!(
                path.extension().map(|e| e == "lua").unwrap_or(false),
                "Wezterm should use .lua extension"
            );
        }

        // Alacritty uses .toml (primary)
        if let Some(path) = get_terminal_config_path(&TerminalApp::Alacritty) {
            assert!(
                path.extension().map(|e| e == "toml").unwrap_or(false),
                "Alacritty primary should use .toml extension"
            );
        }

        // VsCode uses .json
        if let Some(path) = get_terminal_config_path(&TerminalApp::VsCode) {
            assert!(
                path.extension().map(|e| e == "json").unwrap_or(false),
                "VSCode should use .json extension"
            );
        }
    }

    #[test]
    fn test_konsole_paths_includes_profile_directory() {
        #[cfg(target_os = "linux")]
        {
            let paths = get_terminal_config_paths(&TerminalApp::Konsole);
            assert!(!paths.is_empty(), "Konsole should have at least one path");
            // Should include the konsole directory
            assert!(
                paths
                    .iter()
                    .any(|p| p.to_string_lossy().contains("konsole")),
                "Konsole paths should include 'konsole' directory"
            );
        }
    }

    #[test]
    fn test_app_coverage_floor_no_regression() {
        use super::super::app_metadata::ConfigOsTarget;

        let floor: &[(TerminalApp, &[ConfigOsTarget])] = &[
            (
                TerminalApp::Wezterm,
                &[
                    ConfigOsTarget::Linux,
                    ConfigOsTarget::MacOS,
                    ConfigOsTarget::Windows,
                ],
            ),
            (
                TerminalApp::Kitty,
                &[ConfigOsTarget::Linux, ConfigOsTarget::MacOS],
            ),
            (
                TerminalApp::Ghostty,
                &[ConfigOsTarget::Linux, ConfigOsTarget::MacOS],
            ),
            (
                TerminalApp::Alacritty,
                &[
                    ConfigOsTarget::Linux,
                    ConfigOsTarget::MacOS,
                    ConfigOsTarget::Windows,
                ],
            ),
            (TerminalApp::ITerm2, &[ConfigOsTarget::MacOS]),
            (TerminalApp::AppleTerminal, &[ConfigOsTarget::MacOS]),
            (TerminalApp::Konsole, &[ConfigOsTarget::Linux]),
            (TerminalApp::Foot, &[ConfigOsTarget::Linux]),
            (
                TerminalApp::Contour,
                &[
                    ConfigOsTarget::Linux,
                    ConfigOsTarget::MacOS,
                    ConfigOsTarget::Windows,
                ],
            ),
            (
                TerminalApp::Warp,
                &[ConfigOsTarget::Linux, ConfigOsTarget::MacOS],
            ),
            (
                TerminalApp::VsCode,
                &[
                    ConfigOsTarget::Linux,
                    ConfigOsTarget::MacOS,
                    ConfigOsTarget::Windows,
                ],
            ),
        ];

        for (app, targets) in floor {
            let metadata = app
                .metadata()
                .unwrap_or_else(|| panic!("{app:?} regressed to uncovered metadata"));

            for target in *targets {
                assert!(
                    !metadata.config.locations.for_target(*target).is_empty(),
                    "{app:?} regressed to no {target:?} config candidates"
                );
            }
        }
    }

    #[test]
    fn test_terminals_without_config_files() {
        // These terminals don't have file-based configs
        assert!(
            get_terminal_config_path(&TerminalApp::GnomeTerminal).is_none(),
            "GNOME Terminal uses dconf, not a config file"
        );
        assert!(
            get_terminal_config_path(&TerminalApp::Wast).is_none(),
            "Wast doesn't have a standard config location"
        );
        assert!(
            get_terminal_config_path(&TerminalApp::Other("anything".to_string())).is_none(),
            "Unknown terminals should return None"
        );
    }
}
