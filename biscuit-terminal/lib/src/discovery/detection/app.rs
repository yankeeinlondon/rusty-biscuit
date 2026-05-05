use std::env;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalApp {
    AppleTerminal,
    Contour,
    Foot,
    GnomeTerminal,
    Kitty,
    Alacritty,
    Wezterm,
    Konsole,
    ITerm2,
    Warp,
    Ghostty,
    Wast,
    VsCode,
    Other(String),
}

impl std::fmt::Display for TerminalApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(name) => write!(f, "{}", name),
            other => write!(f, "{:?}", other),
        }
    }
}

/// Detect the current terminal emulator application.
///
/// Detection uses environment variables in this order:
/// 1. `TERM_PROGRAM` - Set by most modern terminals
/// 2. `WT_SESSION` - Windows Terminal indicator
/// 3. `TERM` - Fallback for terminals that set this
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{get_terminal_app, TerminalApp};
///
/// match get_terminal_app() {
///     TerminalApp::Wezterm => println!("Running in WezTerm"),
///     TerminalApp::Kitty => println!("Running in Kitty"),
///     TerminalApp::ITerm2 => println!("Running in iTerm2"),
///     TerminalApp::Ghostty => println!("Running in Ghostty"),
///     TerminalApp::Other(name) => println!("Running in: {}", name),
///     _ => println!("Running in another terminal"),
/// }
/// ```
pub fn get_terminal_app() -> TerminalApp {
    // 1. Check TERM_PROGRAM environment variable (most reliable when set)
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "Apple_Terminal" => return TerminalApp::AppleTerminal,
            "iterm2" | "iTerm.app" => return TerminalApp::ITerm2,
            "vscode" => return TerminalApp::VsCode,
            "warp" | "WarpTerminal" => return TerminalApp::Warp,
            "ghostty" => return TerminalApp::Ghostty,
            "kitty" => return TerminalApp::Kitty,
            "Alacritty" => return TerminalApp::Alacritty,
            "WezTerm" => return TerminalApp::Wezterm,
            "gnome-terminal" => return TerminalApp::GnomeTerminal,
            "konsole" => return TerminalApp::Konsole,
            _ => {}
        }
    }

    // 2. Check terminal-specific environment variables
    if env::var("WT_SESSION").is_ok() {
        return TerminalApp::Other("Windows Terminal".to_string());
    }
    if env::var("KITTY_WINDOW_ID").is_ok() || env::var("KITTY_PID").is_ok() {
        return TerminalApp::Kitty;
    }
    if env::var("WEZTERM_PANE").is_ok() || env::var("WEZTERM_UNIX_SOCKET").is_ok() {
        return TerminalApp::Wezterm;
    }
    if env::var("ITERM_SESSION_ID").is_ok() || env::var("ITERM_PROFILE").is_ok() {
        return TerminalApp::ITerm2;
    }
    if env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
        return TerminalApp::Ghostty;
    }
    // Alacritty sets these environment variables
    if env::var("ALACRITTY_WINDOW_ID").is_ok()
        || env::var("ALACRITTY_SOCKET").is_ok()
        || env::var("ALACRITTY_LOG").is_ok()
    {
        return TerminalApp::Alacritty;
    }

    // 3. Check TERM variable
    let term = env::var("TERM").unwrap_or_default();
    match term.as_str() {
        "xterm-kitty" | "kitty" => return TerminalApp::Kitty,
        "alacritty" => return TerminalApp::Alacritty,
        "wezterm" => return TerminalApp::Wezterm,
        "ghostty" => return TerminalApp::Ghostty,
        "foot" | "foot-extra" => return TerminalApp::Foot,
        "contour" => return TerminalApp::Contour,
        _ => {}
    }

    TerminalApp::Other(term)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_app_display() {
        assert_eq!(format!("{}", TerminalApp::Kitty), "Kitty");
        assert_eq!(
            format!("{}", TerminalApp::Other("Custom".to_string())),
            "Custom"
        );
    }

    #[test]
    fn test_terminal_app_debug() {
        let debug = format!("{:?}", TerminalApp::Wezterm);
        assert!(debug.contains("Wezterm"));
    }

    #[test]
    fn test_terminal_app_clone() {
        let app = TerminalApp::Kitty;
        let cloned = app.clone();
        assert!(matches!(cloned, TerminalApp::Kitty));
        assert!(matches!(app, TerminalApp::Kitty));
    }

    #[test]
    fn test_terminal_app_serialize() {
        let app = TerminalApp::Alacritty;
        let json = serde_json::to_string(&app).unwrap();
        assert!(json.contains("Alacritty"));
    }
}
