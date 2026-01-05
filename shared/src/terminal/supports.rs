use std::env;
use termini::{NumberCapability, StringCapability, TermInfo};

// =============================================================================
// Color Depth Constants
// =============================================================================

/// 24-bit true color (16.7 million colors): 2^24
pub const TRUE_COLOR_DEPTH: u32 = 16_777_216;

/// 256-color mode depth
pub const COLORS_256_DEPTH: u32 = 256;

/// 16-color (basic ANSI) mode depth
pub const COLORS_16_DEPTH: u32 = 16;

/// 8-color (minimal) mode depth
pub const COLORS_8_DEPTH: u32 = 8;

/// Represents basic underline support capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnderlineSupport {
    /// Whether the terminal supports basic underline rendering.
    pub basic: bool,
    /// Whether the terminal supports coloring underlines independently of text.
    pub colored: bool,
}

/// Represents support for various underline style variants.
///
/// Modern terminals (Kitty, WezTerm, Alacritty, etc.) support extended underline
/// styles using SGR sub-parameters (e.g., `\e[4:3m` for curly underlines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnderlineVariants {
    /// Straight/single underline (`\e[4:1m` or `\e[4m`).
    pub straight: bool,
    /// Double underline (`\e[4:2m`).
    pub double: bool,
    /// Curly/squiggly underline (`\e[4:3m`) - commonly used for LSP errors.
    pub curly: bool,
    /// Dotted underline (`\e[4:4m`).
    pub dotted: bool,
    /// Dashed underline (`\e[4:5m`).
    pub dashed: bool,
    /// Whether underlines can be colored independently (`\e[58:2::R:G:Bm`).
    pub colored: bool,
}

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
                color_depth = TRUE_COLOR_DEPTH,
                source = "COLORTERM",
                colorterm = %colorterm,
                "Detected truecolor support from COLORTERM env var"
            );
            return TRUE_COLOR_DEPTH;
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
    if let Ok(term_info) = TermInfo::from_env()
        && term_info
            .utf8_string_cap(StringCapability::EnterItalicsMode)
            .is_some()
    {
        return true;
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

/// Returns whether the terminal supports basic underline rendering.
///
/// This function uses a multi-layer detection strategy:
///
/// 1. **Terminfo** (authoritative): Checks for `EnterUnderlineMode` (`smul`) capability
/// 2. **TERM_PROGRAM**: Recognizes modern terminal emulators known to support underlines
/// 3. **TERM**: Falls back to pattern matching for common terminal types
///
/// ## Returns
///
/// An [`UnderlineSupport`] struct indicating:
/// - `basic`: Whether basic underline is supported
/// - `colored`: Whether colored underlines are supported (requires modern terminal)
///
/// ## Examples
///
/// ```
/// use shared::terminal::supports_underline;
///
/// let support = supports_underline();
/// if support.basic {
///     print!("\x1b[4mUnderlined text\x1b[24m");
/// }
/// if support.colored {
///     print!("\x1b[4m\x1b[58:2::255:0:0mRed underline\x1b[59m\x1b[24m");
/// }
/// ```
pub fn supports_underline() -> UnderlineSupport {
    use std::io::IsTerminal;

    // If stdout is not a TTY, no styling
    if !std::io::stdout().is_terminal() {
        return UnderlineSupport {
            basic: false,
            colored: false,
        };
    }

    // Check for dumb terminal
    let term = env::var("TERM").unwrap_or_default();
    if term == "dumb" {
        return UnderlineSupport {
            basic: false,
            colored: false,
        };
    }

    let mut basic = false;
    let mut colored = false;

    // 1. Query terminfo for EnterUnderlineMode (smul) capability
    if let Ok(term_info) = TermInfo::from_env()
        && term_info
            .utf8_string_cap(StringCapability::EnterUnderlineMode)
            .is_some()
    {
        basic = true;
    }

    // 2. Check TERM_PROGRAM for known terminal emulators with colored underline support
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            // Modern terminals with full underline support including colors
            "kitty" | "WezTerm" | "Alacritty" | "ghostty" | "contour" | "foot" => {
                basic = true;
                colored = true;
            }
            // iTerm2 supports colored underlines since 3.4
            "iTerm.app" => {
                basic = true;
                colored = true;
            }
            // VTE-based terminals (GNOME Terminal, Tilix, etc.)
            "gnome-terminal" | "tilix" => {
                basic = true;
                colored = true;
            }
            // Konsole supports colored underlines
            "konsole" => {
                basic = true;
                colored = true;
            }
            // Apple Terminal has basic underline only
            "Apple_Terminal" => {
                basic = true;
            }
            // VS Code terminal
            "vscode" => {
                basic = true;
                colored = true;
            }
            _ => {}
        }
    }

    // 3. Check for Windows Terminal (uses WT_SESSION env var)
    if env::var("WT_SESSION").is_ok() {
        basic = true;
        colored = true;
    }

    // 4. Fallback: check TERM for patterns indicating modern terminals
    if !colored {
        let colored_terms = matches!(
            term.as_str(),
            "xterm-kitty"
                | "kitty"
                | "kitty-direct"
                | "wezterm"
                | "alacritty"
                | "alacritty-direct"
                | "ghostty"
                | "foot"
                | "foot-direct"
                | "contour"
        );
        if colored_terms {
            basic = true;
            colored = true;
        }
    }

    // Most terminals with 256 colors support basic underline
    if !basic {
        let basic_terms = matches!(
            term.as_str(),
            "xterm-256color"
                | "xterm-direct"
                | "tmux-256color"
                | "screen-256color"
                | "rxvt-unicode-256color"
        );
        if basic_terms {
            basic = true;
        }
    }

    UnderlineSupport { basic, colored }
}

/// Returns the supported underline style variants for the current terminal.
///
/// This function detects support for extended underline styles introduced by Kitty
/// and adopted by many modern terminals. These styles use SGR sub-parameters
/// (colon-separated values like `\e[4:3m` for curly underlines).
///
/// ## Detection Strategy
///
/// 1. **TERM_PROGRAM**: Identifies terminal emulator by name
/// 2. **WT_SESSION**: Detects Windows Terminal
/// 3. **TERM**: Falls back to terminal type patterns
///
/// Note: Terminfo's non-standard `Su` capability is not widely available in
/// standard terminfo databases, so terminal identification is the primary method.
///
/// ## Returns
///
/// An [`UnderlineVariants`] struct indicating support for each underline style:
/// - `straight`: Standard single underline (widely supported)
/// - `double`: Double underline
/// - `curly`: Curly/squiggly underline (LSP errors)
/// - `dotted`: Dotted underline
/// - `dashed`: Dashed underline
/// - `colored`: Independent underline coloring
///
/// ## Examples
///
/// ```
/// use shared::terminal::supported_underline_variants;
///
/// let variants = supported_underline_variants();
/// if variants.curly && variants.colored {
///     // Red squiggly underline for errors (LSP-style)
///     print!("\x1b[4:3m\x1b[58:2::255:0:0mError text\x1b[59m\x1b[4:0m");
/// } else if variants.straight {
///     // Fallback to basic underline
///     print!("\x1b[4mError text\x1b[24m");
/// }
/// ```
pub fn supported_underline_variants() -> UnderlineVariants {
    use std::io::IsTerminal;

    let none = UnderlineVariants {
        straight: false,
        double: false,
        curly: false,
        dotted: false,
        dashed: false,
        colored: false,
    };

    // If stdout is not a TTY, no styling
    if !std::io::stdout().is_terminal() {
        return none;
    }

    // Check for dumb terminal
    let term = env::var("TERM").unwrap_or_default();
    if term == "dumb" {
        return none;
    }

    // Check if basic underline is supported via terminfo
    let has_basic_underline = TermInfo::from_env()
        .map(|ti| {
            ti.utf8_string_cap(StringCapability::EnterUnderlineMode)
                .is_some()
        })
        .unwrap_or(false);

    // Helper for terminals with full extended underline support
    let full_support = || UnderlineVariants {
        straight: true,
        double: true,
        curly: true,
        dotted: true,
        dashed: true,
        colored: true,
    };

    // Helper for terminals with straight underline only
    let basic_only = || UnderlineVariants {
        straight: true,
        double: false,
        curly: false,
        dotted: false,
        dashed: false,
        colored: false,
    };

    // 1. Check TERM_PROGRAM for known terminal emulators
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            // Full Kitty-style underline support
            "kitty" | "WezTerm" | "Alacritty" | "ghostty" | "contour" | "foot" | "iTerm.app" => {
                return full_support();
            }
            // VTE-based terminals (GNOME Terminal 3.44+, Tilix) - full support
            "gnome-terminal" | "tilix" => {
                return full_support();
            }
            // Konsole has colored underlines but limited style support
            "konsole" => {
                return UnderlineVariants {
                    straight: true,
                    double: true,
                    curly: false, // Konsole doesn't support curly as of 2024
                    dotted: false,
                    dashed: false,
                    colored: true,
                };
            }
            // Apple Terminal - basic underline only
            "Apple_Terminal" => {
                return basic_only();
            }
            // VS Code terminal - full support
            "vscode" => {
                return full_support();
            }
            _ => {}
        }
    }

    // 2. Check for Windows Terminal
    if env::var("WT_SESSION").is_ok() {
        return full_support();
    }

    // 3. Check TERM for known terminal patterns
    match term.as_str() {
        // Full extended underline support
        "xterm-kitty" | "kitty" | "kitty-direct" | "wezterm" | "alacritty"
        | "alacritty-direct" | "ghostty" | "foot" | "foot-direct" | "contour" => {
            return full_support();
        }
        // Basic underline via common terminal types
        "xterm-256color" | "xterm-direct" | "tmux-256color" | "screen-256color"
        | "rxvt-unicode-256color" => {
            // These may or may not support extended underlines depending on
            // the actual terminal behind them. Return basic only to be safe.
            if has_basic_underline {
                return basic_only();
            }
        }
        _ => {}
    }

    // 4. Fall back to terminfo for basic support
    if has_basic_underline {
        return basic_only();
    }

    none
}
