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
use std::process::{Command, Stdio};

/// Detected terminal emulator type.
///
/// Identifies the terminal emulator running the application, enabling
/// terminal-specific feature detection and behavior adaptation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalKind {
    /// WezTerm - feature-rich terminal with pane support
    Wezterm,
    /// Ghostty - modern GPU-accelerated terminal
    Ghostty,
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
            Self::Ghostty => "Ghostty",
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
            TerminalKind::Ghostty => (false, true),
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

/// Result of setting up the TUI pane layout.
///
/// Contains information needed for proper TUI operation after layout setup.
#[derive(Debug, Clone, Default)]
pub struct TuiLayoutResult {
    /// The pane ID where the TUI is running (if in Wezterm).
    pub tui_pane_id: Option<String>,
    /// The pane ID where tasks should execute (if in Wezterm with pane support).
    /// This is the "parent" pane that was split to create the TUI pane.
    pub task_pane_id: Option<String>,
    /// Whether the layout was successfully created.
    pub layout_created: bool,
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

        // Ghostty sets TERM_PROGRAM=ghostty
        if Self::is_ghostty() {
            return TerminalKind::Ghostty;
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

    /// Checks if running in Ghostty.
    ///
    /// Ghostty sets `TERM_PROGRAM=ghostty`.
    #[must_use]
    pub fn is_ghostty() -> bool {
        env::var("TERM_PROGRAM")
            .map(|v| v.eq_ignore_ascii_case("ghostty"))
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

    /// Gets the current Wezterm pane ID from the environment.
    ///
    /// Returns `None` if not running in Wezterm.
    #[must_use]
    pub fn get_wezterm_pane_id() -> Option<String> {
        env::var("WEZTERM_PANE").ok()
    }

    /// Sets up the TUI layout for Wezterm.
    ///
    /// When running in Wezterm, this creates a split layout with:
    /// - Top pane (~80%): Where tasks will execute
    /// - Bottom pane (max(12 rows, 20%)): Where the TUI will run
    ///
    /// The function returns information about the created layout so the TUI
    /// can coordinate task execution properly.
    ///
    /// ## Errors
    ///
    /// Returns an error if the Wezterm CLI commands fail.
    pub fn setup_tui_layout() -> Result<TuiLayoutResult, String> {
        if !Self::is_wezterm() {
            // Not in Wezterm - TUI will run fullscreen
            return Ok(TuiLayoutResult::default());
        }

        // Get the current pane ID - this will become the task execution pane
        let current_pane_id = Self::get_wezterm_pane_id()
            .ok_or_else(|| "WEZTERM_PANE not set".to_string())?;

        let tui_rows = match env::var("LINES").ok().and_then(|value| value.parse::<u16>().ok()) {
            Some(rows) => {
                let percent_rows = rows.saturating_mul(20) / 100;
                let desired = percent_rows.max(12);
                let max_rows = rows.saturating_sub(1).max(1);
                desired.min(max_rows)
            }
            None => 12,
        };
        let tui_rows_arg = tui_rows.to_string();

        // Create a new pane at the bottom for the TUI
        // The command returns the new pane ID
        let output = Command::new("wezterm")
            .args([
                "cli",
                "split-pane",
                "--bottom",
                "--cells",
                &tui_rows_arg,
                "--pane-id",
                &current_pane_id,
                "--",
                // We need to spawn something that immediately exits so we can
                // capture the pane ID. The actual TUI will be started separately.
                "true",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map_err(|e| format!("failed to run wezterm cli: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "wezterm cli split-pane failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Parse the new pane ID from stdout
        let tui_pane_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if tui_pane_id.is_empty() {
            return Err("wezterm did not return a pane ID".to_string());
        }

        // Move focus to the new TUI pane
        let focus_result = Command::new("wezterm")
            .args(["cli", "activate-pane", "--pane-id", &tui_pane_id])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if let Err(e) = focus_result {
            // Non-fatal - continue even if focus fails
            eprintln!("warning: failed to focus TUI pane: {e}");
        }

        Ok(TuiLayoutResult {
            tui_pane_id: Some(tui_pane_id),
            task_pane_id: Some(current_pane_id),
            layout_created: true,
        })
    }

    /// Creates a new pane for task execution.
    ///
    /// In Wezterm, this creates a new split pane in the task execution area
    /// (top 80% of the window). Each task gets its own pane.
    ///
    /// The command runs directly in the new pane with full terminal access,
    /// supporting interactive programs. The pane closes when the command exits.
    ///
    /// ## Arguments
    ///
    /// * `task_pane_id` - The pane ID where tasks should be created (the original pane)
    /// * `command` - The command to execute in the new pane
    ///
    /// ## Returns
    ///
    /// The pane ID of the newly created task pane.
    pub fn create_task_pane(task_pane_id: Option<&str>, command: &str) -> Result<String, String> {
        if !Self::is_wezterm() {
            return Err("Not in Wezterm".to_string());
        }

        // Build the command arguments - run the command directly without wrapping
        let mut args = vec!["cli", "split-pane", "--top"];

        // If we have a specific pane ID, target that pane
        let pane_id_str;
        if let Some(pane_id) = task_pane_id {
            pane_id_str = pane_id.to_string();
            args.extend(["--pane-id", &pane_id_str]);
        }

        args.extend(["--", "/bin/sh", "-c", command]);

        let output = Command::new("wezterm")
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map_err(|e| format!("failed to create task pane: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "wezterm cli split-pane failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let new_pane_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(new_pane_id)
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
    fn test_ghostty_detection() {
        with_env("TERM_PROGRAM", "ghostty", || {
            assert!(TerminalDetector::is_ghostty());
            let caps = TerminalDetector::detect();
            assert_eq!(caps.kind, TerminalKind::Ghostty);
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
        assert_eq!(TerminalKind::Ghostty.display_name(), "Ghostty");
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

    // =========================================================================
    // Regression tests for bug: TUI takes over fullscreen instead of splitting
    // =========================================================================

    #[test]
    fn test_tui_layout_result_default_has_no_pane_ids() {
        // Regression test: TuiLayoutResult::default() should not have any pane IDs
        // This is the expected state when NOT in Wezterm - TUI runs fullscreen
        let result = TuiLayoutResult::default();
        assert!(result.tui_pane_id.is_none());
        assert!(result.task_pane_id.is_none());
        assert!(!result.layout_created);
    }

    #[test]
    fn test_setup_tui_layout_returns_default_when_not_in_wezterm() {
        // Regression test: When not in Wezterm, setup_tui_layout should return
        // a default result (no split), not attempt to split
        with_clean_env(|| {
            let result = TerminalDetector::setup_tui_layout();
            assert!(result.is_ok());
            let layout = result.unwrap();
            assert!(layout.tui_pane_id.is_none());
            assert!(layout.task_pane_id.is_none());
            assert!(!layout.layout_created);
        });
    }

    #[test]
    fn test_get_wezterm_pane_id_returns_value_when_set() {
        // Regression test: get_wezterm_pane_id should return the pane ID
        // from WEZTERM_PANE environment variable
        with_env("WEZTERM_PANE", "42", || {
            let pane_id = TerminalDetector::get_wezterm_pane_id();
            assert!(pane_id.is_some());
            assert_eq!(pane_id.unwrap(), "42");
        });
    }

    #[test]
    fn test_get_wezterm_pane_id_returns_none_when_not_set() {
        // Regression test: get_wezterm_pane_id should return None when
        // not running in Wezterm
        with_clean_env(|| {
            let pane_id = TerminalDetector::get_wezterm_pane_id();
            assert!(pane_id.is_none());
        });
    }

    #[test]
    fn test_create_task_pane_fails_when_not_in_wezterm() {
        // Regression test: create_task_pane should fail when not in Wezterm
        // This ensures tasks don't accidentally try to create panes outside Wezterm
        with_clean_env(|| {
            let result = TerminalDetector::create_task_pane(Some("42"), "echo test");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Not in Wezterm"));
        });
    }

    #[test]
    fn test_tui_layout_result_clone() {
        // Regression test: TuiLayoutResult should be clonable for passing
        // between TUI and executor
        let layout = TuiLayoutResult {
            tui_pane_id: Some("tui-123".to_string()),
            task_pane_id: Some("task-456".to_string()),
            layout_created: true,
        };
        let cloned = layout.clone();
        assert_eq!(layout.tui_pane_id, cloned.tui_pane_id);
        assert_eq!(layout.task_pane_id, cloned.task_pane_id);
        assert_eq!(layout.layout_created, cloned.layout_created);
    }

    // =========================================================================
    // Regression tests for Bug 2: Tasks should run independently without
    // "Press Enter to close" prompts blocking interactive commands
    // =========================================================================

    // Note: The actual test for create_task_pane not wrapping commands would
    // require mocking the wezterm CLI, which is not practical in unit tests.
    // Instead, we verify the API contract: create_task_pane takes the command
    // as-is and should pass it directly to the shell without modification.
    //
    // The implementation fix removed the command wrapping that previously added:
    //   echo ''; echo '─────────────────────────────────────';
    //   echo 'Command completed. Press Enter to close...'; read
    //
    // This is a code-level change verified by code review and integration testing.

    // =========================================================================
    // Regression tests for Bug 3: Default target based on terminal capabilities
    // =========================================================================

    #[test]
    fn test_wezterm_capabilities_support_panes() {
        // Regression test: Wezterm should report pane support
        // This ensures the UI can correctly default to NewPane target
        with_env("WEZTERM_PANE", "123", || {
            let caps = TerminalDetector::detect();
            assert!(
                caps.supports_panes,
                "Wezterm should support panes"
            );
            assert!(
                caps.supports_new_window,
                "Wezterm should support new windows"
            );
        });
    }

    #[test]
    fn test_terminal_app_capabilities_no_panes() {
        // Regression test: Terminal.app should NOT report pane support
        // This ensures the UI correctly defaults to NewWindow instead of NewPane
        with_env("TERM_PROGRAM", "Apple_Terminal", || {
            let caps = TerminalDetector::detect();
            assert!(
                !caps.supports_panes,
                "Terminal.app should NOT support panes"
            );
            assert!(
                caps.supports_new_window,
                "Terminal.app should support new windows"
            );
        });
    }

    #[test]
    fn test_unknown_terminal_capabilities_minimal() {
        // Regression test: Unknown terminals should have minimal capabilities
        // This ensures the UI correctly defaults to Background
        with_clean_env(|| {
            let caps = TerminalDetector::detect();
            assert!(
                !caps.supports_panes,
                "Unknown terminal should NOT support panes"
            );
            // Note: on macOS, even unknown terminals can open Terminal.app
            // so supports_new_window may be true on macOS
        });
    }
}
