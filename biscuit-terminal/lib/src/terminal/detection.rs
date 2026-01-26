/// The type of image support (if any) of a terminal
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

pub enum ColorMode {
    /// the background color is light, and text characters must be dark
    /// to provide adequate contrast
    Light,
    /// the background color is dark, and text characters must be light
    /// to provide the adequate contrast
    Dark,
    Unknown
}


pub fn color_depth() -> ColorDepth {
    // Check COLORTERM environment variable first
    if let Ok(colorterm) = env::var("COLORTERM") {
        let colorterm_lower = colorterm.to_lowercase();
        if colorterm_lower == "truecolor" || colorterm_lower == "24bit" {
            tracing::info!(
                color_depth = ColorDepth::TrueColor,
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

/// Whether the terminal is in "light" or "dark" mode
pub fn color_mode() -> ColorMode {
    todo!()
}

/// The width of the terminal (in characters)
pub fn terminal_width() ->u32 {
    todo!()
}

/// The height of the terminal (in characters)
pub fn terminal_height() -> u32 {
    todo!()
}

/// the terminal's dimensions (width, height)
pub fn dimensions() -> (u32, u32) {
    (
      width(),
      height()
    )
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

pub fn supports_underline() -> UnderlineSupport {
    todo!()
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
