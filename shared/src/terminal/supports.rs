use std::env;
use termini::{NumberCapability, StringCapability, TermInfo};

/// Returns the maximum number of colors the terminal supports.
///
/// This function checks the COLORTERM environment variable first for truecolor
/// support, then falls back to querying terminfo for the number of colors.
///
/// ## Returns
///
/// The number of colors supported:
/// - 16,777,216 if COLORTERM indicates truecolor/24bit support
/// - The value from terminfo's MaxColors capability if available
/// - 0 if no color support can be detected
///
/// ## Examples
///
/// ```
/// use shared::terminal::color_depth;
///
/// let depth = color_depth();
/// if depth >= 16_777_216 {
///     println!("Terminal supports truecolor!");
/// } else if depth >= 256 {
///     println!("Terminal supports 256 colors");
/// } else if depth >= 8 {
///     println!("Terminal supports basic colors");
/// } else {
///     println!("No color support detected");
/// }
/// ```
pub fn color_depth() -> u32 {
    // Check COLORTERM environment variable first
    if let Ok(colorterm) = env::var("COLORTERM") {
        let colorterm_lower = colorterm.to_lowercase();
        if colorterm_lower == "truecolor" || colorterm_lower == "24bit" {
            tracing::info!(
                color_depth = 16_777_216,
                source = "COLORTERM",
                colorterm = %colorterm,
                "Detected truecolor support from COLORTERM env var"
            );
            return 16_777_216; // 2^24 colors
        }
    }

    // Fallback to terminfo
    match TermInfo::from_env() {
        Ok(term_info) => {
            // Query the MaxColors capability
            let depth = term_info
                .number_cap(NumberCapability::MaxColors)
                .map(|n| n as u32)
                .unwrap_or(0);
            tracing::info!(
                color_depth = depth,
                source = "terminfo",
                "Detected color depth from terminfo"
            );
            depth
        }
        Err(e) => {
            tracing::info!(
                color_depth = 0,
                source = "fallback",
                error = %e,
                "Failed to query terminfo, defaulting to no color"
            );
            0
        }
    }
}

/// Returns whether the terminal supports setting the foreground color.
///
/// This function checks terminfo for the presence of the SetForeground capability.
///
/// ## Returns
///
/// - `true` if the terminal supports setting foreground colors
/// - `false` if the capability is not available or terminfo cannot be queried
///
/// ## Examples
///
/// ```
/// use shared::terminal::supports_setting_foreground;
///
/// if supports_setting_foreground() {
///     println!("\x1b[31mThis text is red!\x1b[0m");
/// } else {
///     println!("This text has no color");
/// }
/// ```
pub fn supports_setting_foreground() -> bool {
    match TermInfo::from_env() {
        Ok(term_info) => {
            // Check for SetForeground capability
            term_info
                .utf8_string_cap(StringCapability::SetForeground)
                .is_some()
        }
        Err(_) => false,
    }
}

/// Returns whether the terminal supports italic text rendering.
///
/// This function uses a multi-layer detection strategy:
///
/// 1. **Terminfo** (authoritative): Checks for the `EnterItalicsMode` (`sitm`) capability
/// 2. **TERM_PROGRAM**: Recognizes modern terminal emulators known to support italics
/// 3. **TERM**: Falls back to pattern matching for common terminal types
///
/// This layered approach compensates for outdated terminfo databases that may lack
/// italic capabilities for terminals that actually support them.
///
/// ## Returns
///
/// - `true` if the terminal supports italic text
/// - `false` if stdout is not a TTY, TERM is "dumb", or no support is detected
///
/// ## Examples
///
/// ```
/// use shared::terminal::supports_italics;
///
/// if supports_italics() {
///     println!("\x1b[3mThis text is italic!\x1b[23m");
/// } else {
///     println!("This text has no styling");
/// }
/// ```
pub fn supports_italics() -> bool {
    use std::io::IsTerminal;

    // If stdout is not a TTY, don't use styling
    if !std::io::stdout().is_terminal() {
        return false;
    }

    // Check for dumb terminal
    let term = env::var("TERM").unwrap_or_default();
    if term == "dumb" {
        return false;
    }

    // 1. Query terminfo for EnterItalicsMode (sitm) capability (authoritative)
    if let Ok(term_info) = TermInfo::from_env() {
        if term_info
            .utf8_string_cap(StringCapability::EnterItalicsMode)
            .is_some()
        {
            return true;
        }
    }

    // 2. Check TERM_PROGRAM for known terminal emulators that support italics
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        let dominated = matches!(
            term_program.as_str(),
            "iTerm.app"
                | "Apple_Terminal"
                | "Alacritty"
                | "kitty"
                | "WezTerm"
                | "vscode"
                | "Hyper"
                | "Tabby"
                | "Rio"
        );
        if dominated {
            return true;
        }
    }

    // 3. Check for Windows Terminal (uses WT_SESSION env var)
    if env::var("WT_SESSION").is_ok() {
        return true;
    }

    // 4. Fallback: check TERM for patterns indicating modern terminals
    let dominated = matches!(
        term.as_str(),
        "xterm-256color"
            | "xterm-direct"
            | "alacritty"
            | "alacritty-direct"
            | "kitty"
            | "kitty-direct"
            | "wezterm"
            | "tmux-256color"
            | "screen-256color"
    );
    if dominated {
        return true;
    }

    false
}
