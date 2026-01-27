use std::env;

use serde::{Deserialize, Serialize};
use terminal_size::{terminal_size, Height, Width};
use termini::{NumberCapability, StringCapability, TermInfo};

/// The type of image support (if any) of a terminal
#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum ImageSupport {
    None,
    /// the highest quality image support comes from the
    /// [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/).
    ///
    /// This is now supported in:
    ///
    /// - Kitty
    /// - WezTerm
    /// - Warp
    /// - iTerm2
    /// - Ghostty
    /// - Konsole
    /// - wast
    Kitty,
    /// one of the earlier image formats but slowly being phased out,
    /// even it's originator iTERM2 now supports the Kitty protocol.
    ITerm,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
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
    Other(String)
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum ColorDepth {
    /// no color support
    None,
    /// 8 colors
    Minimal,
    /// 16 colors (8 normal plus "bright" variants)
    Basic,
    /// 256 color palette (8 bit)
    Enhanced,
    /// 16 million colors (24 bit)
    TrueColor
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum ColorMode {
    /// the background color is light, and text characters must be dark
    /// to provide adequate contrast
    Light,
    /// the background color is dark, and text characters must be light
    /// to provide the adequate contrast
    Dark,
    Unknown
}

/// Represents support for various underline style variants.
///
/// Modern terminals (Kitty, WezTerm, Alacritty, etc.) support extended underline
/// styles using SGR sub-parameters (e.g., `\e[4:3m` for curly underlines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnderlineSupport {
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


pub fn color_depth() -> ColorDepth {
    // Check COLORTERM environment variable first
    if let Ok(colorterm) = env::var("COLORTERM") {
        let colorterm_lower = colorterm.to_lowercase();
        if colorterm_lower == "truecolor" || colorterm_lower == "24bit" {
            tracing::info!(
                color_depth = ?ColorDepth::TrueColor,
                source = "COLORTERM",
                colorterm = %colorterm,
                "Detected truecolor support from COLORTERM env var"
            );
            return ColorDepth::TrueColor;
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

            let color_depth = match depth {
                d if d >= 16_777_216 => ColorDepth::TrueColor,
                d if d >= 256 => ColorDepth::Enhanced,
                d if d >= 16 => ColorDepth::Basic,
                d if d >= 8 => ColorDepth::Minimal,
                _ => ColorDepth::None,
            };

            tracing::info!(
                ?color_depth,
                source = "terminfo",
                "Detected color depth from terminfo"
            );
            color_depth
        }
        Err(e) => {
            tracing::info!(
                color_depth = ?ColorDepth::None,
                source = "fallback",
                error = %e,
                "Failed to query terminfo, defaulting to no color"
            );
            ColorDepth::None
        }
    }
}

/// Whether the terminal is in "light" or "dark" mode
pub fn color_mode() -> ColorMode {
    todo!()
}

/// The width of the terminal (in characters)
pub fn terminal_width() -> u32 {
    dimensions().0
}

/// The height of the terminal (in characters)
pub fn terminal_height() -> u32 {
    dimensions().1
}

/// the terminal's dimensions (width, height)
pub fn dimensions() -> (u32, u32) {
    terminal_size()
        .map(|(Width(w), Height(h))| (w as u32, h as u32))
        .unwrap_or((80, 24))
}



/// Whether the terminal supports images and if so
/// via which standard.
///
/// If multiple standards are supported then
/// the highest quality standard is returned.
pub fn image_support() -> ImageSupport {
    todo!()
}


pub fn osc8_link_support() -> bool {
    todo!()
}




pub fn supports_underline() -> UnderlineSupport {
    use std::io::IsTerminal;

    let none = UnderlineSupport {
        straight: false,
        double: false,
        curly: false,
        dotted: false,
        dashed: false,
        colored: false,
    };

    // If stdout is not a TTY, no styling
    if !std::io::stdout().is_terminal() {
        return UnderlineSupport {
            straight: false,
            double: false,
            curly: false,
            dotted: false,
            dashed: false,
            colored: false
        };
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
    let full_support = || UnderlineSupport {
        straight: true,
        double: true,
        curly: true,
        dotted: true,
        dashed: true,
        colored: true,
    };

    // Helper for terminals with straight underline only
    let basic_only = || UnderlineSupport {
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
                return UnderlineSupport {
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
        "xterm-kitty" | "kitty" | "kitty-direct" | "wezterm" | "alacritty" | "alacritty-direct"
        | "ghostty" | "foot" | "foot-direct" | "contour" => {
            return full_support();
        }
        // Basic underline via common terminal types
        "xterm-256color"
        | "xterm-direct"
        | "tmux-256color"
        | "screen-256color"
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
pub fn italics_support() -> bool {
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
