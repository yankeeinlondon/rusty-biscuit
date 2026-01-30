//! Terminal capability detection via biscuit-terminal.
//!
//! This module provides thin wrappers around biscuit-terminal's detection functions,
//! maintaining API compatibility with the existing darkmatter interface.
//!
//! ## Migration Note
//!
//! The underlying detection logic has been consolidated into biscuit-terminal.
//! This module re-exports and wraps those functions for API stability.

use termini::{StringCapability, TermInfo};

// Re-export biscuit-terminal's ColorDepth for conversion
use biscuit_terminal::discovery::detection::ColorDepth as BtColorDepth;

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
///
/// This type is maintained for API compatibility with existing darkmatter code.
/// For new code, consider using [`UnderlineVariants`] which provides more detail.
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
/// This function delegates to biscuit-terminal's detection and converts
/// the result to a u32 for API compatibility.
///
/// ## Returns
///
/// The number of colors supported:
/// - 16,777,216 if terminal supports truecolor/24bit
/// - 256 for 256-color terminals
/// - 16 for basic ANSI color support
/// - 8 for minimal color support
/// - 0 if no color support detected
///
/// ## Examples
///
/// ```
/// use darkmatter_lib::terminal::color_depth;
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
    let bt_depth = biscuit_terminal::discovery::detection::color_depth();
    match bt_depth {
        BtColorDepth::TrueColor => TRUE_COLOR_DEPTH,
        BtColorDepth::Enhanced => COLORS_256_DEPTH,
        BtColorDepth::Basic => COLORS_16_DEPTH,
        BtColorDepth::Minimal => COLORS_8_DEPTH,
        BtColorDepth::None => 0,
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
/// use darkmatter_lib::terminal::supports_setting_foreground;
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
/// This function delegates to biscuit-terminal's `italics_support()` function,
/// which uses a multi-layer detection strategy:
///
/// 1. **Terminfo** (authoritative): Checks for `EnterItalicsMode` (`sitm`) capability
/// 2. **TERM_PROGRAM**: Recognizes modern terminal emulators known to support italics
/// 3. **TERM**: Falls back to pattern matching for common terminal types
///
/// ## Returns
///
/// - `true` if the terminal supports italic text
/// - `false` if stdout is not a TTY, TERM is "dumb", or no support is detected
///
/// ## Examples
///
/// ```
/// use darkmatter_lib::terminal::supports_italics;
///
/// if supports_italics() {
///     println!("\x1b[3mThis text is italic!\x1b[23m");
/// } else {
///     println!("This text has no styling");
/// }
/// ```
pub fn supports_italics() -> bool {
    biscuit_terminal::discovery::detection::italics_support()
}

/// Returns whether the terminal supports basic underline rendering.
///
/// This function delegates to biscuit-terminal's `underline_support()` and
/// converts the result to the darkmatter-compatible `UnderlineSupport` type.
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
/// use darkmatter_lib::terminal::supports_underline;
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
    let bt_support = biscuit_terminal::discovery::detection::underline_support();
    UnderlineSupport {
        basic: bt_support.straight,
        colored: bt_support.colored,
    }
}

/// Returns the supported underline style variants for the current terminal.
///
/// This function delegates to biscuit-terminal's `underline_support()` and
/// converts the result to the darkmatter-compatible `UnderlineVariants` type.
///
/// Modern terminals support extended underline styles introduced by Kitty
/// and adopted by many modern terminals. These styles use SGR sub-parameters
/// (colon-separated values like `\e[4:3m` for curly underlines).
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
/// use darkmatter_lib::terminal::supported_underline_variants;
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
    let bt_support = biscuit_terminal::discovery::detection::underline_support();
    UnderlineVariants {
        straight: bt_support.straight,
        double: bt_support.double,
        curly: bt_support.curly,
        dotted: bt_support.dotted,
        dashed: bt_support.dashed,
        colored: bt_support.colored,
    }
}
