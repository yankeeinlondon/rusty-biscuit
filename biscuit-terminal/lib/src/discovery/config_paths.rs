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

use super::detection::TerminalApp;
use super::os_detection::{detect_os_type, OsType};
use std::env;
use std::path::PathBuf;

/// Get the configuration file path for a terminal application.
///
/// Returns the primary configuration file path for the given terminal,
/// taking into account the current operating system. Returns `None` if:
///
/// - The terminal doesn't have a file-based configuration (e.g., GNOME Terminal uses dconf)
/// - The terminal is unknown
/// - The home directory cannot be determined
///
/// ## Platform-Specific Paths
///
/// | Terminal | Linux/macOS | Windows |
/// |----------|-------------|---------|
/// | WezTerm | `~/.config/wezterm/wezterm.lua` | `%USERPROFILE%\.config\wezterm\wezterm.lua` |
/// | Kitty | `~/.config/kitty/kitty.conf` | N/A |
/// | Ghostty | `~/.config/ghostty/config` | N/A |
/// | Alacritty | `~/.config/alacritty/alacritty.toml` | `%APPDATA%\alacritty\alacritty.toml` |
/// | iTerm2 | `~/Library/Preferences/com.googlecode.iterm2.plist` | N/A |
/// | Apple Terminal | `~/Library/Preferences/com.apple.Terminal.plist` | N/A |
/// | VS Code | Handled via VS Code settings | N/A |
/// | GNOME Terminal | dconf (not file-based) | N/A |
/// | Konsole | `~/.local/share/konsole/` (profiles) | N/A |
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
    let os = detect_os_type();
    let home = home_dir()?;

    match app {
        TerminalApp::Wezterm => Some(wezterm_config_path(&home, os)),
        TerminalApp::Kitty => kitty_config_path(&home, os),
        TerminalApp::Ghostty => ghostty_config_path(&home, os),
        TerminalApp::Alacritty => alacritty_config_path(&home, os),
        TerminalApp::ITerm2 => iterm2_config_path(&home, os),
        TerminalApp::AppleTerminal => apple_terminal_config_path(&home, os),
        TerminalApp::Konsole => konsole_config_path(&home, os),
        TerminalApp::Foot => foot_config_path(&home, os),
        TerminalApp::Contour => contour_config_path(&home, os),
        TerminalApp::Warp => warp_config_path(&home, os),
        // GNOME Terminal uses dconf, not a config file
        TerminalApp::GnomeTerminal => None,
        // VS Code terminal settings are managed via VS Code settings.json
        TerminalApp::VsCode => vscode_settings_path(&home, os),
        // Wast doesn't have a standard config location yet
        TerminalApp::Wast => None,
        // Unknown terminals
        TerminalApp::Other(_) => None,
    }
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
    let os = detect_os_type();
    let Some(home) = home_dir() else {
        return Vec::new();
    };

    match app {
        TerminalApp::Alacritty => alacritty_config_paths(&home, os),
        TerminalApp::Konsole => konsole_profile_paths(&home, os),
        _ => get_terminal_config_path(app)
            .map(|p| vec![p])
            .unwrap_or_default(),
    }
}

/// Get the user's home directory.
fn home_dir() -> Option<PathBuf> {
    // Try HOME first (Unix-like systems)
    if let Ok(home) = env::var("HOME") {
        return Some(PathBuf::from(home));
    }

    // Try USERPROFILE (Windows)
    if let Ok(profile) = env::var("USERPROFILE") {
        return Some(PathBuf::from(profile));
    }

    // Try combining HOMEDRIVE and HOMEPATH (Windows fallback)
    if let (Ok(drive), Ok(path)) = (env::var("HOMEDRIVE"), env::var("HOMEPATH")) {
        return Some(PathBuf::from(format!("{}{}", drive, path)));
    }

    None
}

/// Get XDG config directory, with fallback to ~/.config
fn config_dir(home: &PathBuf) -> PathBuf {
    env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join(".config"))
}

/// Get XDG data directory, with fallback to ~/.local/share
fn data_dir(home: &PathBuf) -> PathBuf {
    env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join(".local").join("share"))
}

// Terminal-specific path functions

fn wezterm_config_path(home: &PathBuf, os: OsType) -> PathBuf {
    match os {
        OsType::Windows => home.join(".config").join("wezterm").join("wezterm.lua"),
        OsType::MacOS => config_dir(home).join("wezterm").join("wezterm.lua"),
        _ => config_dir(home).join("wezterm").join("wezterm.lua"),
    }
}

fn kitty_config_path(home: &PathBuf, os: OsType) -> Option<PathBuf> {
    match os {
        OsType::Windows => None, // Kitty doesn't officially support Windows
        OsType::MacOS => Some(config_dir(home).join("kitty").join("kitty.conf")),
        OsType::Linux => Some(config_dir(home).join("kitty").join("kitty.conf")),
        _ => None,
    }
}

fn ghostty_config_path(home: &PathBuf, os: OsType) -> Option<PathBuf> {
    match os {
        OsType::MacOS | OsType::Linux => Some(config_dir(home).join("ghostty").join("config")),
        _ => None,
    }
}

fn alacritty_config_path(home: &PathBuf, os: OsType) -> Option<PathBuf> {
    match os {
        OsType::Windows => Some(
            env::var("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join("AppData").join("Roaming"))
                .join("alacritty")
                .join("alacritty.toml"),
        ),
        _ => Some(config_dir(home).join("alacritty").join("alacritty.toml")),
    }
}

fn alacritty_config_paths(home: &PathBuf, os: OsType) -> Vec<PathBuf> {
    let config = config_dir(home);

    match os {
        OsType::Windows => {
            let appdata = env::var("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join("AppData").join("Roaming"));
            vec![
                appdata.join("alacritty").join("alacritty.toml"),
                appdata.join("alacritty").join("alacritty.yml"),
            ]
        }
        _ => {
            vec![
                config.join("alacritty").join("alacritty.toml"),
                config.join("alacritty").join("alacritty.yml"),
                home.join(".alacritty.toml"),
                home.join(".alacritty.yml"),
            ]
        }
    }
}

fn iterm2_config_path(home: &PathBuf, os: OsType) -> Option<PathBuf> {
    match os {
        OsType::MacOS => Some(
            home.join("Library")
                .join("Preferences")
                .join("com.googlecode.iterm2.plist"),
        ),
        _ => None, // iTerm2 is macOS-only
    }
}

fn apple_terminal_config_path(home: &PathBuf, os: OsType) -> Option<PathBuf> {
    match os {
        OsType::MacOS => Some(
            home.join("Library")
                .join("Preferences")
                .join("com.apple.Terminal.plist"),
        ),
        _ => None, // Apple Terminal is macOS-only
    }
}

fn konsole_config_path(home: &PathBuf, os: OsType) -> Option<PathBuf> {
    match os {
        OsType::Linux | OsType::FreeBSD | OsType::NetBSD | OsType::OpenBSD => {
            // Konsole uses profiles in the data directory
            // Return the profile directory; specific profiles are *.profile files
            Some(data_dir(home).join("konsole"))
        }
        _ => None, // Konsole is primarily Linux/BSD
    }
}

fn konsole_profile_paths(home: &PathBuf, os: OsType) -> Vec<PathBuf> {
    match os {
        OsType::Linux | OsType::FreeBSD | OsType::NetBSD | OsType::OpenBSD => {
            let data = data_dir(home);
            vec![
                data.join("konsole"), // Profile directory
                config_dir(home).join("konsolerc"), // Main config
            ]
        }
        _ => Vec::new(),
    }
}

fn foot_config_path(home: &PathBuf, os: OsType) -> Option<PathBuf> {
    match os {
        OsType::Linux | OsType::FreeBSD => Some(config_dir(home).join("foot").join("foot.ini")),
        _ => None, // Foot is Wayland-only, primarily Linux
    }
}

fn contour_config_path(home: &PathBuf, os: OsType) -> Option<PathBuf> {
    match os {
        OsType::Windows => {
            let local_appdata = env::var("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join("AppData").join("Local"));
            Some(local_appdata.join("contour").join("contour.yml"))
        }
        OsType::MacOS => Some(
            home.join("Library")
                .join("Application Support")
                .join("contour")
                .join("contour.yml"),
        ),
        OsType::Linux => Some(config_dir(home).join("contour").join("contour.yml")),
        _ => None,
    }
}

fn warp_config_path(home: &PathBuf, os: OsType) -> Option<PathBuf> {
    match os {
        OsType::MacOS => Some(home.join(".warp")),
        OsType::Linux => Some(home.join(".warp")),
        _ => None, // Warp is macOS/Linux only
    }
}

fn vscode_settings_path(home: &PathBuf, os: OsType) -> Option<PathBuf> {
    match os {
        OsType::Windows => {
            let appdata = env::var("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join("AppData").join("Roaming"));
            Some(appdata.join("Code").join("User").join("settings.json"))
        }
        OsType::MacOS => Some(
            home.join("Library")
                .join("Application Support")
                .join("Code")
                .join("User")
                .join("settings.json"),
        ),
        OsType::Linux => Some(config_dir(home).join("Code").join("User").join("settings.json")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_dir_returns_some() {
        // Should return Some on all platforms with proper environment
        let home = home_dir();
        assert!(home.is_some(), "home_dir() should return Some");
    }

    #[test]
    fn test_config_dir_respects_xdg() {
        let home = PathBuf::from("/home/test");
        // Default behavior without XDG_CONFIG_HOME
        let config = config_dir(&home);
        // Should either be XDG_CONFIG_HOME or ~/.config
        assert!(
            config.to_string_lossy().contains("config")
                || config == home.join(".config")
        );
    }

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
        #[cfg(not(target_os = "windows"))]
        {
            assert!(paths.len() >= 2, "Alacritty should have multiple config paths");
            // Should include both .toml and .yml options
            let has_toml = paths.iter().any(|p| p.to_string_lossy().contains(".toml"));
            let has_yml = paths.iter().any(|p| p.to_string_lossy().contains(".yml"));
            assert!(has_toml, "Should include .toml path");
            assert!(has_yml, "Should include .yml path");
        }
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
        let path = get_terminal_config_path(&TerminalApp::Konsole);
        #[cfg(target_os = "linux")]
        {
            assert!(path.is_some());
            let path = path.unwrap();
            assert!(path.to_string_lossy().contains("konsole"));
        }
    }

    #[test]
    fn test_foot_config_path() {
        let path = get_terminal_config_path(&TerminalApp::Foot);
        #[cfg(target_os = "linux")]
        {
            assert!(path.is_some());
            let path = path.unwrap();
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
    }

    #[test]
    fn test_get_terminal_config_paths_single_path_terminal() {
        // Terminals with single config path should return vec with one element
        let paths = get_terminal_config_paths(&TerminalApp::Wezterm);
        assert_eq!(paths.len(), 1);
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
            assert!(paths.len() >= 1, "Konsole should have at least one path");
            // Should include the konsole directory
            assert!(
                paths.iter().any(|p| p.to_string_lossy().contains("konsole")),
                "Konsole paths should include 'konsole' directory"
            );
        }
    }

    #[test]
    fn test_data_dir_respects_xdg() {
        let home = PathBuf::from("/home/test");
        // Default behavior without XDG_DATA_HOME
        let data = data_dir(&home);
        // Should either be XDG_DATA_HOME or ~/.local/share
        assert!(
            data.to_string_lossy().contains("share")
                || data == home.join(".local").join("share")
                || data.to_string_lossy().starts_with("/")
        );
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
