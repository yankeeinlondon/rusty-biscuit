//! Terminal multiplexer functions for WezTerm and tmux.
//!
//! These functions provide cross-terminal support for pane splitting,
//! window naming, and focus switching. They detect the terminal type
//! and execute the appropriate commands.

use std::process::Command;

use crate::{discovery::detection::TerminalApp, terminal::Terminal};

/// Result type for multiplex operations.
#[derive(Debug)]
pub enum MultiplexResult {
    /// Operation completed successfully
    Success,
    /// Operation failed with an error message
    Error(String),
    /// Terminal doesn't support this operation
    Unsupported(String),
}

/// Check if tmux is available and we're inside a tmux session.
fn is_in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

/// Check if we're in a WezTerm pane.
fn is_in_wezterm(term: &Terminal) -> bool {
    matches!(term.app, TerminalApp::Wezterm)
}

/// Execute a command and return the result.
fn run_command(cmd: &str, args: &[&str]) -> MultiplexResult {
    match Command::new(cmd).args(args).output() {
        Ok(output) => {
            if output.status.success() {
                MultiplexResult::Success
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                MultiplexResult::Error(format!("Command failed: {}", stderr.trim()))
            }
        }
        Err(e) => MultiplexResult::Error(format!("Failed to execute {}: {}", cmd, e)),
    }
}

/// Split the current window pane vertically (new pane on the right).
///
/// ## Terminal Support
///
/// - **WezTerm**: Uses `wezterm cli split-pane --right`
/// - **tmux**: Uses `tmux split-window -h`
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::terminal::Terminal;
/// use biscuit_terminal::utils::multiplex::split_vertical;
///
/// let term = Terminal::new();
/// split_vertical(term);
/// ```
pub fn split_vertical(term: Terminal) -> MultiplexResult {
    if is_in_wezterm(&term) {
        run_command("wezterm", &["cli", "split-pane", "--right"])
    } else if is_in_tmux() {
        run_command("tmux", &["split-window", "-h"])
    } else {
        MultiplexResult::Unsupported(format!(
            "Terminal {:?} does not support pane splitting",
            term.app
        ))
    }
}

/// Split the current window pane horizontally (new pane below).
///
/// ## Terminal Support
///
/// - **WezTerm**: Uses `wezterm cli split-pane --bottom`
/// - **tmux**: Uses `tmux split-window -v`
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::terminal::Terminal;
/// use biscuit_terminal::utils::multiplex::split_horizontal;
///
/// let term = Terminal::new();
/// split_horizontal(term);
/// ```
pub fn split_horizontal(term: Terminal) -> MultiplexResult {
    if is_in_wezterm(&term) {
        run_command("wezterm", &["cli", "split-pane", "--bottom"])
    } else if is_in_tmux() {
        run_command("tmux", &["split-window", "-v"])
    } else {
        MultiplexResult::Unsupported(format!(
            "Terminal {:?} does not support pane splitting",
            term.app
        ))
    }
}

/// Rename the current window/workspace.
///
/// ## Terminal Support
///
/// - **WezTerm**: Uses `wezterm cli rename-workspace`
/// - **tmux**: Uses `tmux rename-window`
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::terminal::Terminal;
/// use biscuit_terminal::utils::multiplex::name_window;
///
/// let term = Terminal::new();
/// name_window(term, "my-window");
/// ```
pub fn name_window(term: Terminal, name: &str) -> MultiplexResult {
    if is_in_wezterm(&term) {
        run_command("wezterm", &["cli", "rename-workspace", name])
    } else if is_in_tmux() {
        run_command("tmux", &["rename-window", name])
    } else {
        MultiplexResult::Unsupported(format!(
            "Terminal {:?} does not support window renaming",
            term.app
        ))
    }
}

/// Switch focus to the pane on the right.
///
/// ## Terminal Support
///
/// - **WezTerm**: Uses `wezterm cli activate-pane-direction right`
/// - **tmux**: Uses `tmux select-pane -R`
pub fn switch_focus_right(term: Terminal) -> MultiplexResult {
    if is_in_wezterm(&term) {
        run_command("wezterm", &["cli", "activate-pane-direction", "right"])
    } else if is_in_tmux() {
        run_command("tmux", &["select-pane", "-R"])
    } else {
        MultiplexResult::Unsupported(format!(
            "Terminal {:?} does not support pane focus switching",
            term.app
        ))
    }
}

/// Switch focus to the pane on the left.
///
/// ## Terminal Support
///
/// - **WezTerm**: Uses `wezterm cli activate-pane-direction left`
/// - **tmux**: Uses `tmux select-pane -L`
pub fn switch_focus_left(term: Terminal) -> MultiplexResult {
    if is_in_wezterm(&term) {
        run_command("wezterm", &["cli", "activate-pane-direction", "left"])
    } else if is_in_tmux() {
        run_command("tmux", &["select-pane", "-L"])
    } else {
        MultiplexResult::Unsupported(format!(
            "Terminal {:?} does not support pane focus switching",
            term.app
        ))
    }
}

/// Switch focus to the pane above.
///
/// ## Terminal Support
///
/// - **WezTerm**: Uses `wezterm cli activate-pane-direction up`
/// - **tmux**: Uses `tmux select-pane -U`
pub fn switch_focus_up(term: Terminal) -> MultiplexResult {
    if is_in_wezterm(&term) {
        run_command("wezterm", &["cli", "activate-pane-direction", "up"])
    } else if is_in_tmux() {
        run_command("tmux", &["select-pane", "-U"])
    } else {
        MultiplexResult::Unsupported(format!(
            "Terminal {:?} does not support pane focus switching",
            term.app
        ))
    }
}

/// Switch focus to the pane below.
///
/// ## Terminal Support
///
/// - **WezTerm**: Uses `wezterm cli activate-pane-direction down`
/// - **tmux**: Uses `tmux select-pane -D`
pub fn switch_focus_down(term: Terminal) -> MultiplexResult {
    if is_in_wezterm(&term) {
        run_command("wezterm", &["cli", "activate-pane-direction", "down"])
    } else if is_in_tmux() {
        run_command("tmux", &["select-pane", "-D"])
    } else {
        MultiplexResult::Unsupported(format!(
            "Terminal {:?} does not support pane focus switching",
            term.app
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_in_tmux_without_env() {
        // Remove TMUX env var if present for the test
        // SAFETY: This is a test and we're modifying our own process environment
        unsafe {
            std::env::remove_var("TMUX");
        }
        assert!(!is_in_tmux());
    }

    #[test]
    fn test_multiplex_result_debug() {
        let success = MultiplexResult::Success;
        let error = MultiplexResult::Error("test error".to_string());
        let unsupported = MultiplexResult::Unsupported("not supported".to_string());

        // Just verify Debug works
        assert!(format!("{:?}", success).contains("Success"));
        assert!(format!("{:?}", error).contains("Error"));
        assert!(format!("{:?}", unsupported).contains("Unsupported"));
    }
}
