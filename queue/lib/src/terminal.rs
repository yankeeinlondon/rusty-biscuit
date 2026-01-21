//! Terminal detection and capability discovery.
//!
//! This module provides utilities for detecting the current terminal emulator
//! and its capabilities, enabling the queue TUI to adapt its behavior based
//! on available features like pane splitting and window management.
//!
//! ## Examples
//!
//! ```
//! use queue_lib::terminal::{TerminalDetector, TerminalKind};
//!
//! let caps = TerminalDetector::detect();
//! println!("Running in {:?}", caps.kind);
//!
//! if caps.supports_panes {
//!     println!("Pane splitting is available");
//! }
//! ```

use std::env;

/// Detected terminal emulator type.
///
/// Identifies the terminal emulator running the application, enabling
/// terminal-specific feature detection and behavior adaptation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalKind {
    /// WezTerm - feature-rich terminal with pane support
    Wezterm,
    /// macOS Terminal.app
    TerminalApp,
    /// iTerm2 - macOS terminal with pane support
    ITerm2,
    /// GNOME Terminal
    GnomeTerminal,
    /// KDE Konsole
    Konsole,
    /// Xfce4 Terminal
    Xfce4Terminal,
    /// XTerm or xterm-compatible
    Xterm,
    /// Unknown or undetected terminal
    Unknown,
}

impl TerminalKind {
    /// Returns a human-readable name for the terminal.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Wezterm => "WezTerm",
            Self::TerminalApp => "Terminal.app",
            Self::ITerm2 => "iTerm2",
            Self::GnomeTerminal => "GNOME Terminal",
            Self::Konsole => "Konsole",
            Self::Xfce4Terminal => "Xfce4 Terminal",
            Self::Xterm => "XTerm",
            Self::Unknown => "Unknown",
        }
    }
}

/// Capabilities of the current terminal emulator.
///
/// Describes what features are available in the detected terminal,
/// allowing the application to adapt its behavior accordingly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCapabilities {
    /// The detected terminal type.
    pub kind: TerminalKind,
    /// Whether the terminal supports splitting into panes.
    pub supports_panes: bool,
    /// Whether the terminal supports opening new windows.
    pub supports_new_window: bool,
}

impl TerminalCapabilities {
    /// Creates capabilities for a known terminal kind.
    #[must_use]
    fn for_kind(kind: TerminalKind) -> Self {
        let (supports_panes, supports_new_window) = match kind {
            TerminalKind::Wezterm => (true, true),
            TerminalKind::ITerm2 => (true, true),
            TerminalKind::TerminalApp => (false, true),
            TerminalKind::GnomeTerminal => (false, true),
            TerminalKind::Konsole => (false, true),
            TerminalKind::Xfce4Terminal => (false, true),
            TerminalKind::Xterm => (false, true),
            TerminalKind::Unknown => (false, false),
        };

        Self {
            kind,
            supports_panes,
            supports_new_window,
        }
    }
}

impl Default for TerminalCapabilities {
    fn default() -> Self {
        Self::for_kind(TerminalKind::Unknown)
    }
}

/// Detects the current terminal environment.
///
/// Provides methods to identify which terminal emulator is running
/// and what capabilities it supports.
pub struct TerminalDetector;

impl TerminalDetector {
    /// Detects the current terminal from environment variables.
    ///
    /// Checks various environment variables in priority order to identify
    /// the terminal emulator. Returns capabilities based on the detected
    /// terminal type.
    ///
    /// ## Detection Priority
    ///
    /// 1. WezTerm (`WEZTERM_PANE`)
    /// 2. iTerm2 (`ITERM_SESSION_ID`)
    /// 3. GNOME Terminal (`GNOME_TERMINAL_SCREEN` or `VTE_VERSION`)
    /// 4. Konsole (`KONSOLE_VERSION`)
    /// 5. Xfce4 Terminal (`COLORTERM` == "xfce4-terminal")
    /// 6. Terminal.app (`TERM_PROGRAM` == "Apple_Terminal")
    /// 7. XTerm (`TERM` starts with "xterm")
    /// 8. Unknown (fallback)
    #[must_use]
    pub fn detect() -> TerminalCapabilities {
        let kind = Self::detect_kind();
        TerminalCapabilities::for_kind(kind)
    }

    /// Detects just the terminal kind without full capabilities.
    #[must_use]
    pub fn detect_kind() -> TerminalKind {
        // Check in priority order - most specific first

        // WezTerm sets WEZTERM_PANE
        if Self::is_wezterm() {
            return TerminalKind::Wezterm;
        }

        // iTerm2 sets ITERM_SESSION_ID
        if Self::is_iterm2() {
            return TerminalKind::ITerm2;
        }

        // GNOME Terminal sets GNOME_TERMINAL_SCREEN or VTE_VERSION
        if Self::is_gnome_terminal() {
            return TerminalKind::GnomeTerminal;
        }

        // Konsole sets KONSOLE_VERSION
        if Self::is_konsole() {
            return TerminalKind::Konsole;
        }

        // Xfce4 Terminal sets COLORTERM=xfce4-terminal
        if Self::is_xfce4_terminal() {
            return TerminalKind::Xfce4Terminal;
        }

        // macOS Terminal.app sets TERM_PROGRAM=Apple_Terminal
        if Self::is_terminal_app() {
            return TerminalKind::TerminalApp;
        }

        // XTerm or xterm-compatible terminals
        if Self::is_xterm() {
            return TerminalKind::Xterm;
        }

        TerminalKind::Unknown
    }

    /// Checks if running in WezTerm.
    ///
    /// WezTerm sets the `WEZTERM_PANE` environment variable.
    #[must_use]
    pub fn is_wezterm() -> bool {
        env::var("WEZTERM_PANE").is_ok()
    }

    /// Checks if running in iTerm2.
    ///
    /// iTerm2 sets the `ITERM_SESSION_ID` environment variable.
    #[must_use]
    pub fn is_iterm2() -> bool {
        env::var("ITERM_SESSION_ID").is_ok()
    }

    /// Checks if running in GNOME Terminal.
    ///
    /// GNOME Terminal sets `GNOME_TERMINAL_SCREEN` or `VTE_VERSION`.
    #[must_use]
    pub fn is_gnome_terminal() -> bool {
        env::var("GNOME_TERMINAL_SCREEN").is_ok() || env::var("VTE_VERSION").is_ok()
    }

    /// Checks if running in KDE Konsole.
    ///
    /// Konsole sets the `KONSOLE_VERSION` environment variable.
    #[must_use]
    pub fn is_konsole() -> bool {
        env::var("KONSOLE_VERSION").is_ok()
    }

    /// Checks if running in Xfce4 Terminal.
    ///
    /// Xfce4 Terminal sets `COLORTERM=xfce4-terminal`.
    #[must_use]
    pub fn is_xfce4_terminal() -> bool {
        env::var("COLORTERM")
            .map(|v| v == "xfce4-terminal")
            .unwrap_or(false)
    }

    /// Checks if running in macOS Terminal.app.
    ///
    /// Terminal.app sets `TERM_PROGRAM=Apple_Terminal`.
    #[must_use]
    pub fn is_terminal_app() -> bool {
        env::var("TERM_PROGRAM")
            .map(|v| v == "Apple_Terminal")
            .unwrap_or(false)
    }

    /// Checks if running in XTerm or xterm-compatible terminal.
    ///
    /// Checks if `TERM` starts with "xterm" when no other terminal is detected.
    #[must_use]
    pub fn is_xterm() -> bool {
        env::var("TERM")
            .map(|v| v.starts_with("xterm"))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to ensure tests don't run concurrently and interfere with each other's env vars
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Helper to run a test with a specific environment variable set.
    ///
    /// # Safety
    /// Uses unsafe env::set_var/remove_var. The ENV_MUTEX ensures these are not
    /// called concurrently from multiple threads.
    fn with_env<F: FnOnce()>(key: &str, value: &str, f: F) {
        let _guard = ENV_MUTEX.lock().unwrap();
        // Clear all terminal-related vars first
        clear_terminal_env_vars();
        // SAFETY: We hold ENV_MUTEX, ensuring single-threaded access to env vars
        unsafe {
            env::set_var(key, value);
        }
        f();
        // SAFETY: We hold ENV_MUTEX, ensuring single-threaded access to env vars
        unsafe {
            env::remove_var(key);
        }
    }

    /// Helper to run a test with multiple environment variables set.
    ///
    /// # Safety
    /// Uses unsafe env::set_var/remove_var. The ENV_MUTEX ensures these are not
    /// called concurrently from multiple threads.
    fn with_envs<F: FnOnce()>(vars: &[(&str, &str)], f: F) {
        let _guard = ENV_MUTEX.lock().unwrap();
        // Clear all terminal-related vars first
        clear_terminal_env_vars();
        for (key, value) in vars {
            // SAFETY: We hold ENV_MUTEX, ensuring single-threaded access to env vars
            unsafe {
                env::set_var(key, value);
            }
        }
        f();
        for (key, _) in vars {
            // SAFETY: We hold ENV_MUTEX, ensuring single-threaded access to env vars
            unsafe {
                env::remove_var(key);
            }
        }
    }

    /// Helper to run a test with no terminal environment variables.
    fn with_clean_env<F: FnOnce()>(f: F) {
        let _guard = ENV_MUTEX.lock().unwrap();
        clear_terminal_env_vars();
        f();
    }

    /// Clears all terminal-related environment variables.
    ///
    /// # Safety
    /// This function must only be called while holding ENV_MUTEX.
    fn clear_terminal_env_vars() {
        let vars = [
            "WEZTERM_PANE",
            "ITERM_SESSION_ID",
            "GNOME_TERMINAL_SCREEN",
            "VTE_VERSION",
            "KONSOLE_VERSION",
            "COLORTERM",
            "TERM_PROGRAM",
            "TERM",
        ];
        for var in vars {
            // SAFETY: Caller must hold ENV_MUTEX to ensure single-threaded access
            unsafe {
                env::remove_var(var);
            }
        }
    }

    #[test]
    fn test_wezterm_detection() {
        with_env("WEZTERM_PANE", "0", || {
            assert!(TerminalDetector::is_wezterm());
            let caps = TerminalDetector::detect();
            assert_eq!(caps.kind, TerminalKind::Wezterm);
            assert!(caps.supports_panes);
            assert!(caps.supports_new_window);
        });
    }

    #[test]
    fn test_iterm2_detection() {
        with_env("ITERM_SESSION_ID", "w0t0p0:12345", || {
            assert!(TerminalDetector::is_iterm2());
            let caps = TerminalDetector::detect();
            assert_eq!(caps.kind, TerminalKind::ITerm2);
            assert!(caps.supports_panes);
            assert!(caps.supports_new_window);
        });
    }

    #[test]
    fn test_gnome_terminal_detection_via_screen() {
        with_env("GNOME_TERMINAL_SCREEN", "/org/gnome/Terminal/screen/0", || {
            assert!(TerminalDetector::is_gnome_terminal());
            let caps = TerminalDetector::detect();
            assert_eq!(caps.kind, TerminalKind::GnomeTerminal);
            assert!(!caps.supports_panes);
            assert!(caps.supports_new_window);
        });
    }

    #[test]
    fn test_gnome_terminal_detection_via_vte() {
        with_env("VTE_VERSION", "6003", || {
            assert!(TerminalDetector::is_gnome_terminal());
            let caps = TerminalDetector::detect();
            assert_eq!(caps.kind, TerminalKind::GnomeTerminal);
        });
    }

    #[test]
    fn test_konsole_detection() {
        with_env("KONSOLE_VERSION", "220401", || {
            assert!(TerminalDetector::is_konsole());
            let caps = TerminalDetector::detect();
            assert_eq!(caps.kind, TerminalKind::Konsole);
            assert!(!caps.supports_panes);
            assert!(caps.supports_new_window);
        });
    }

    #[test]
    fn test_xfce4_terminal_detection() {
        with_env("COLORTERM", "xfce4-terminal", || {
            assert!(TerminalDetector::is_xfce4_terminal());
            let caps = TerminalDetector::detect();
            assert_eq!(caps.kind, TerminalKind::Xfce4Terminal);
            assert!(!caps.supports_panes);
            assert!(caps.supports_new_window);
        });
    }

    #[test]
    fn test_terminal_app_detection() {
        with_env("TERM_PROGRAM", "Apple_Terminal", || {
            assert!(TerminalDetector::is_terminal_app());
            let caps = TerminalDetector::detect();
            assert_eq!(caps.kind, TerminalKind::TerminalApp);
            assert!(!caps.supports_panes);
            assert!(caps.supports_new_window);
        });
    }

    #[test]
    fn test_xterm_detection() {
        with_env("TERM", "xterm-256color", || {
            assert!(TerminalDetector::is_xterm());
            let caps = TerminalDetector::detect();
            assert_eq!(caps.kind, TerminalKind::Xterm);
            assert!(!caps.supports_panes);
            assert!(caps.supports_new_window);
        });
    }

    #[test]
    fn test_xterm_detection_plain() {
        with_env("TERM", "xterm", || {
            assert!(TerminalDetector::is_xterm());
            let caps = TerminalDetector::detect();
            assert_eq!(caps.kind, TerminalKind::Xterm);
        });
    }

    #[test]
    fn test_unknown_terminal() {
        with_clean_env(|| {
            let caps = TerminalDetector::detect();
            assert_eq!(caps.kind, TerminalKind::Unknown);
            assert!(!caps.supports_panes);
            assert!(!caps.supports_new_window);
        });
    }

    #[test]
    fn test_wezterm_priority_over_xterm() {
        // WezTerm also sets TERM=xterm-256color, but WEZTERM_PANE should take priority
        with_envs(&[("WEZTERM_PANE", "0"), ("TERM", "xterm-256color")], || {
            let caps = TerminalDetector::detect();
            assert_eq!(caps.kind, TerminalKind::Wezterm);
        });
    }

    #[test]
    fn test_iterm2_priority_over_xterm() {
        // iTerm2 also sets TERM, but ITERM_SESSION_ID should take priority
        with_envs(
            &[("ITERM_SESSION_ID", "session"), ("TERM", "xterm-256color")],
            || {
                let caps = TerminalDetector::detect();
                assert_eq!(caps.kind, TerminalKind::ITerm2);
            },
        );
    }

    #[test]
    fn test_colorterm_not_xfce4() {
        // Other values of COLORTERM should not trigger xfce4 detection
        with_env("COLORTERM", "truecolor", || {
            assert!(!TerminalDetector::is_xfce4_terminal());
        });
    }

    #[test]
    fn test_terminal_kind_display_name() {
        assert_eq!(TerminalKind::Wezterm.display_name(), "WezTerm");
        assert_eq!(TerminalKind::TerminalApp.display_name(), "Terminal.app");
        assert_eq!(TerminalKind::ITerm2.display_name(), "iTerm2");
        assert_eq!(TerminalKind::GnomeTerminal.display_name(), "GNOME Terminal");
        assert_eq!(TerminalKind::Konsole.display_name(), "Konsole");
        assert_eq!(TerminalKind::Xfce4Terminal.display_name(), "Xfce4 Terminal");
        assert_eq!(TerminalKind::Xterm.display_name(), "XTerm");
        assert_eq!(TerminalKind::Unknown.display_name(), "Unknown");
    }

    #[test]
    fn test_capabilities_default() {
        let caps = TerminalCapabilities::default();
        assert_eq!(caps.kind, TerminalKind::Unknown);
        assert!(!caps.supports_panes);
        assert!(!caps.supports_new_window);
    }

    #[test]
    fn test_capabilities_equality() {
        let caps1 = TerminalCapabilities::for_kind(TerminalKind::Wezterm);
        let caps2 = TerminalCapabilities::for_kind(TerminalKind::Wezterm);
        assert_eq!(caps1, caps2);

        let caps3 = TerminalCapabilities::for_kind(TerminalKind::ITerm2);
        assert_ne!(caps1, caps3);
    }
}
