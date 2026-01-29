//! Font detection utilities for terminal applications.
//!
//! This module provides functions for detecting font-related terminal capabilities,
//! including font name, font size, Nerd Font detection, and ligature support.
//!
//! ## Detection Strategy
//!
//! Font detection uses config file parsing or system queries for supported terminals:
//! - **Wezterm**: Parses `wezterm.lua` for `config.font` and `config.font_size`
//! - **iTerm2** (macOS): Queries macOS preferences via `defaults read`
//! - **Kitty**: Parses `kitty.conf` for `font_family` and `font_size`
//! - **Alacritty**: Parses TOML for `[font.normal] family` and `[font] size`
//! - **Ghostty**: Queries `ghostty +show-config` for font settings
//!
//! ## Terminal-Specific Support
//!
//! | Terminal | Font Name | Font Size | Notes |
//! |----------|-----------|-----------|-------|
//! | Wezterm | ✅ | ✅ | Full support via config parsing |
//! | iTerm2 | ✅ | ✅ | Full support via macOS preferences |
//! | Kitty | ✅ | ✅ | Parses `font_family` and `font_size` from kitty.conf |
//! | Alacritty | ✅ | ✅ | Parses TOML config; detection improved via config file fallback |
//! | Ghostty | ⚠️ | ⚠️ | Tries `ghostty +show-config`; may not report font settings |
//!
//! ## Fallback Detection
//!
//! When terminal app detection fails (common with Alacritty which doesn't set
//! `TERM_PROGRAM` by default), the library will scan known config file locations:
//! - `~/.config/alacritty/alacritty.toml`
//! - `~/.config/kitty/kitty.conf`
//! - `~/.config/wezterm/wezterm.lua`
//! - `~/.config/ghostty/config`
//! - iTerm2 macOS preferences
//!
//! ## General Limitations
//!
//! - Detection requires the config file to exist and be readable
//! - Only explicit settings are detected (defaults are not known)
//! - Complex font configurations (multiple fonts, fallbacks) may not parse fully
//! - Terminal app detection must succeed for config-based detection to work
//!
//! ## Nerd Font Detection
//!
//! Nerd Fonts are detected via:
//! 1. `NERD_FONT` environment variable (community convention: `NERD_FONT=1`)
//! 2. Explicit "Nerd Font" or "NF" suffix in font name
//! 3. Recognition of 69 known Nerd Font family base names (e.g., "JetBrains Mono", "Fira Code")
//!
//! ## Examples
//!
//! ```
//! use biscuit_terminal::discovery::fonts::{font_name, font_size, detect_nerd_font, ligature_support_likely};
//!
//! if let Some(name) = font_name() {
//!     println!("Font: {}", name);
//! }
//! if let Some(size) = font_size() {
//!     println!("Size: {}pt", size);
//! }
//! if detect_nerd_font() == Some(true) {
//!     println!("Nerd Font icons available!");
//! }
//! if ligature_support_likely() {
//!     println!("Ligatures likely supported");
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::fs;
use crate::discovery::config_paths::get_terminal_config_path;
use crate::discovery::detection::{get_terminal_app, is_tty, TerminalApp};

/// Represents common font ligatures that may be available in terminal fonts.
///
/// Font ligatures combine multiple characters into a single glyph for improved
/// readability. These are commonly found in programming fonts like Fira Code,
/// JetBrains Mono, Cascadia Code, and others.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::FontLigature;
///
/// let ligature = FontLigature::ArrowRight;
/// // Represents the "->" ligature commonly used for returns and member access
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FontLigature {
    // Arrow ligatures
    /// `->` - Right arrow (member access, return type)
    ArrowRight,
    /// `=>` - Double arrow right (fat arrow, lambda, match arms)
    DoubleArrowRight,
    /// `<-` - Left arrow (assignment in Haskell)
    ArrowLeft,
    /// `<=` - Less than or equal to
    LessThanOrEqual,
    /// `>=` - Greater than or equal to
    GreaterThanOrEqual,
    /// `->>` - Long arrow right (pipeline operator)
    LongArrowRight,
    /// `=>>` - Long double arrow right
    LongDoubleArrowRight,
    /// `<<-` - Long arrow left
    LongArrowLeft,

    // Comparison and equality
    /// `==` - Equality comparison
    Equality,
    /// `===` - Strict equality (JavaScript)
    StrictEquality,
    /// `!=` - Not equal
    NotEqual,
    /// `!==` - Strict not equal (JavaScript)
    StrictNotEqual,
    /// `=~` - Regex match
    RegexMatch,
    /// `!~` - Regex not match
    RegexNotMatch,

    // Logical operators
    /// `&&` - Logical AND
    LogicalAnd,
    /// `||` - Logical OR
    LogicalOr,
    /// `!!` - Double negation / force unwrap
    DoubleBang,

    // Mathematical operators
    /// `*` - Multiplication (sometimes ligatured with context)
    Multiply,
    /// `**` - Exponentiation
    Exponentiation,
    /// `***` - Triple star
    TripleStar,
    /// `//` - Integer division or comment
    IntegerDivision,
    /// `///` - Doc comment
    DocComment,
    /// `++` - Increment or concatenation
    Increment,
    /// `+++` - Triple plus
    TriplePlus,
    /// `--` - Decrement
    Decrement,
    /// `---` - Triple minus
    TripleMinus,
    /// `<>` - Diamond operator / empty box
    Diamond,
    /// `><` - Reverse diamond
    ReverseDiamond,
    /// `><>` - Fish operator (Haskell)
    Fish,

    // Assignment operators
    /// `:=` - Walrus operator (assignment expression)
    Walrus,
    /// `=:` - Reverse walrus
    ReverseWalrus,
    /// `+=` - Plus equals
    PlusEquals,
    /// `-=` - Minus equals
    MinusEquals,
    /// `*=` - Multiply equals
    MultiplyEquals,
    /// `/=` - Divide equals
    DivideEquals,
    /// `%=` - Modulo equals
    ModuloEquals,
    /// `&=` - Bitwise AND equals
    BitwiseAndEquals,
    /// `|=` - Bitwise OR equals
    BitwiseOrEquals,
    /// `^=` - Bitwise XOR equals
    BitwiseXorEquals,
    /// `<<=` - Left shift equals
    LeftShiftEquals,
    /// `>>=` - Right shift equals
    RightShiftEquals,

    // Bitwise operators
    /// `<<` - Left shift
    LeftShift,
    /// `>>` - Right shift
    RightShift,
    /// `<<<` - Triple left shift
    TripleLeftShift,
    /// `>>>` - Triple right shift (unsigned)
    TripleRightShift,
    /// `&` - Bitwise AND
    BitwiseAnd,
    /// `|` - Bitwise OR
    BitwiseOr,
    /// `^` - Bitwise XOR
    BitwiseXor,
    /// `~` - Bitwise NOT
    BitwiseNot,

    // Punctuation and brackets
    /// `::` - Scope resolution operator
    ScopeResolution,
    /// `...` - Ellipsis / spread operator
    Ellipsis,
    /// `..` - Range operator
    Range,
    /// `..=` - Inclusive range (Rust)
    InclusiveRange,
    /// `..=` - Half-open range (Swift)
    HalfOpenRange,
    /// `||>` - Pipe operator (Elixir)
    PipeRight,
    /// `<||` - Reverse pipe operator
    PipeLeft,
    /// `|>` - Feed operator (F#, Elixir)
    FeedRight,
    /// `<|` - Feed left operator
    FeedLeft,
    /// `[|` - Opening bracket pipe
    BracketPipe,
    /// `|]` - Closing pipe bracket
    PipeBracket,
    /// `(|` - Opening paren pipe
    ParenPipe,
    /// `|)` - Closing pipe paren
    PipeParen,
    /// `<>` - Template placeholder (PHP, etc.)
    TemplatePlaceholder,
    /// `<~` - Bind operator (Haskell)
    Bind,
    /// `~>` - Reverse bind
    ReverseBind,

    // Hash and other symbols
    /// `#` - Hash / pound / sharp
    Hash,
    /// `##` - Double hash
    DoubleHash,
    /// `###` - Triple hash
    TripleHash,
    /// `#(` - Hash paren
    HashParen,
    /// `#{` - Hash brace
    HashBrace,
    /// `#?` - Hash question (debug macro)
    HashQuestion,
    /// `#!` - Shebang
    Shebang,

    // At sign variants
    /// `@` - At sign
    At,
    /// `@@` - Double at
    DoubleAt,

    // Dollar sign variants
    /// `$` - Dollar sign
    Dollar,
    /// `$$` - Double dollar
    DoubleDollar,

    // Percent variants
    /// `%` - Percent
    Percent,
    /// `%%` - Double percent
    DoublePercent,

    // Question mark variants
    /// `?` - Question mark
    Question,
    /// `??` - Null coalescing / optional chaining
    DoubleQuestion,
    /// `?.` - Optional chaining
    QuestionDot,
    /// `?:` - Elvis operator
    Elvis,
    /// `?!` - Question bang
    QuestionBang,
    /// `?=` - Question equals
    QuestionEquals,

    // Colon variants
    /// `:` - Colon
    Colon,
    /// `:::` - Triple colon
    TripleColon,

    // Semicolon variants
    /// `;` - Semicolon
    Semicolon,
    /// `;;` - Double semicolon
    DoubleSemicolon,

    // Other common ligatures
    /// `///=` - Doc comment equals (Rust)
    DocCommentEquals,
    /// `//=` - Comment equals
    CommentEquals,
    /// `/**` - JSDoc opening
    JsDocOpen,
    /// `*/` - JSDoc closing
    JsDocClose,
    /// `<!--` - HTML comment open
    HtmlCommentOpen,
    /// `-->` - HTML comment close
    HtmlCommentClose,
    /// `</` - HTML closing tag
    HtmlClosingTag,

    /// Catch-all for other ligatures not in this enumeration
    Other(String),
}

/// Represents the window size in pixels.
///
/// This is the pixel dimensions of the terminal window, which can be used
/// to calculate cell size (font size) when combined with grid dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSizePixels {
    /// Window width in pixels
    pub width: u32,
    /// Window height in pixels
    pub height: u32,
}

/// Represents the cell size in pixels.
///
/// The cell size is the width and height of a single character cell,
/// which is directly related to the font size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellSize {
    /// Cell width in pixels
    pub width: u32,
    /// Cell height in pixels
    pub height: u32,
}

// =============================================================================
// Nerd Font Detection
// =============================================================================

/// Known Nerd Font family base names (without "Nerd Font" suffix).
///
/// This list is sourced from the [Nerd Fonts GitHub repository](https://github.com/ryanoasis/nerd-fonts).
/// Last updated: 2026-01.
const NERD_FONT_BASE_NAMES: &[&str] = &[
    "0xProto",
    "3270",
    "AdwaitaMono",
    "Agave",
    "AnonymicePro",
    "Arimo",
    "AtkynsonMono",
    "AurulentSansMono",
    "BigBlueTerminal",
    "BitstromWera",
    "BlexMono",
    "CaskaydiaCove",
    "CaskaydiaMono",
    "CodeNewRoman",
    "ComicShannsMono",
    "CommitMono",
    "Cousine",
    "D2Coding",
    "DaddyTimeMono",
    "DepartureMono",
    "DejaVuSansMono",
    "DroidSansMono",
    "EnvyCodeR",
    "FantasqueSansMono",
    "FiraCode",
    "FiraMono",
    "GeistMono",
    "GoMono",
    "Gohu",
    "Hack",
    "Hasklug",
    "HeavyDataMono",
    "Hurmit",
    "iM-Writing",
    "Inconsolata",
    "InconsolataGo",
    "InconsolataLGC",
    "IntoneMono",
    "Iosevka",
    "IosevkaTerm",
    "IosevkaTermSlab",
    "JetBrainsMono",
    "Lekton",
    "Literation",
    "Lilex",
    "MartianMono",
    "Meslo",
    "Monaspice",
    "Monofur",
    "Monoid",
    "Mononoki",
    "MPlus",
    "Noto",
    "OpenDyslexic",
    "Overpass",
    "ProFont",
    "ProggyClean",
    "RecMono",
    "RobotoMono",
    "SauceCodePro",
    "ShureTechMono",
    "SpaceMono",
    "Terminess",
    "Tinos",
    "Ubuntu",
    "UbuntuMono",
    "UbuntuSans",
    "VictorMono",
    "ZedMono",
];

/// Check if a font name is a known Nerd Font variant.
///
/// Detection strategy:
/// 1. Check for explicit "Nerd Font" or "NF" suffix (definite match)
/// 2. Match against known Nerd Font family base names (likely match)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::is_nerd_font_name;
///
/// // Explicit Nerd Font markers
/// assert!(is_nerd_font_name("JetBrainsMono Nerd Font"));
/// assert!(is_nerd_font_name("Hack Nerd Font Mono"));
/// assert!(is_nerd_font_name("FiraCode NF"));
///
/// // Known Nerd Font base names (likely Nerd Fonts)
/// assert!(is_nerd_font_name("JetBrains Mono"));
/// assert!(is_nerd_font_name("JetBrainsMono"));
/// assert!(is_nerd_font_name("Fira Code"));
/// assert!(is_nerd_font_name("Hack"));
///
/// // Unknown fonts
/// assert!(!is_nerd_font_name("Monaco"));
/// assert!(!is_nerd_font_name("SF Mono"));
/// assert!(!is_nerd_font_name("Menlo"));
/// ```
pub fn is_nerd_font_name(font_name: &str) -> bool {
    let lower = font_name.to_lowercase();
    // Normalize: remove spaces for comparison (JetBrains Mono -> jetbrainsmono)
    let normalized = lower.replace(' ', "").replace('-', "");

    // 1. Check for explicit "Nerd Font" or "NF" markers (definite match)
    if lower.contains("nerd font") || lower.ends_with(" nf") || lower.contains(" nf ") {
        return true;
    }

    // 2. Check if the font name matches a known Nerd Font base name
    // These are fonts that have Nerd Font patched versions available
    for base in NERD_FONT_BASE_NAMES {
        let base_lower = base.to_lowercase();
        let base_normalized = base_lower.replace(' ', "").replace('-', "");

        // Exact match (normalized)
        if normalized == base_normalized {
            return true;
        }

        // Font name starts with the base name (e.g., "JetBrainsMono Regular")
        if normalized.starts_with(&base_normalized) {
            return true;
        }

        // Base name with spaces (e.g., "JetBrains Mono" matches "JetBrainsMono")
        if lower.starts_with(&base_lower) {
            return true;
        }
    }

    false
}

/// Detect if a Nerd Font is being used.
///
/// Detection strategy:
/// 1. Check `NERD_FONT` env var (explicit user declaration)
/// 2. Check detected font name against known patterns
///
/// ## Returns
///
/// - `Some(true)`: Nerd Font confirmed (env var or font name match)
/// - `Some(false)`: Explicitly disabled via env var
/// - `None`: Cannot determine (no env var, unknown font)
///
/// ## Environment Variable
///
/// The `NERD_FONT` environment variable is a community convention:
/// - `NERD_FONT=1` or `NERD_FONT=true` - Explicitly declare Nerd Font usage
/// - `NERD_FONT=0` or `NERD_FONT=false` - Explicitly declare no Nerd Font
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::detect_nerd_font;
///
/// // Returns Some(true), Some(false), or None based on environment
/// let nerd_font = detect_nerd_font();
/// if nerd_font == Some(true) {
///     println!("Nerd Font icons available!");
/// }
/// ```
pub fn detect_nerd_font() -> Option<bool> {
    // Check NERD_FONT environment variable first (explicit user declaration)
    if let Ok(value) = std::env::var("NERD_FONT") {
        let lower = value.to_lowercase();
        if lower == "1" || lower == "true" || lower == "yes" {
            tracing::debug!("detect_nerd_font(): NERD_FONT env var is set to true");
            return Some(true);
        }
        if lower == "0" || lower == "false" || lower == "no" {
            tracing::debug!("detect_nerd_font(): NERD_FONT env var is set to false");
            return Some(false);
        }
        // Non-standard value, ignore
        tracing::debug!(
            "detect_nerd_font(): NERD_FONT env var has non-standard value: {}",
            value
        );
    }

    // Check detected font name
    if let Some(name) = font_name() {
        if is_nerd_font_name(&name) {
            tracing::debug!(
                "detect_nerd_font(): font '{}' detected as Nerd Font",
                name
            );
            return Some(true);
        }
        tracing::debug!(
            "detect_nerd_font(): font '{}' is not a known Nerd Font",
            name
        );
    }

    // Cannot determine
    tracing::debug!("detect_nerd_font(): cannot determine Nerd Font status");
    None
}

/// Get the terminal window size in pixels.
///
/// This function queries the window pixel dimensions using the
/// XTWINOPS escape sequence (CSI 14 t).
///
/// ## Detection Method
///
/// The CSI 14 t escape sequence queries the terminal for its window size
/// in pixels. The terminal responds with a sequence like:
/// `\033[4;height;widtht`
///
/// ## Returns
///
/// - `Some(WindowSizePixels)` if window pixel size can be detected
/// - `None` if detection fails (not a TTY, terminal doesn't support CSI 14 t, timeout)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::window_size_pixels;
///
/// if let Some(size) = window_size_pixels() {
///     println!("Window size: {}x{} pixels", size.width, size.height);
/// } else {
///     println!("Could not detect window pixel size");
/// }
/// ```
#[cfg(unix)]
pub fn window_size_pixels() -> Option<WindowSizePixels> {
    use std::io::{Read, Write};
    use std::os::unix::io::AsRawFd;
    use std::time::{Duration, Instant};

    // Must be a TTY to query
    if !is_tty() {
        tracing::debug!("window_size_pixels(): not a TTY");
        return None;
    }

    // Open /dev/tty for direct terminal access
    let mut tty = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
    {
        Ok(f) => f,
        Err(e) => {
            tracing::debug!("window_size_pixels(): failed to open /dev/tty: {}", e);
            return None;
        }
    };

    let fd = tty.as_raw_fd();

    // Save current terminal attributes
    let mut orig_termios: libc::termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(fd, &mut orig_termios) } != 0 {
        tracing::debug!("window_size_pixels(): tcgetattr failed");
        return None;
    }

    // Set raw mode
    let mut raw_termios = orig_termios;
    raw_termios.c_lflag &= !(libc::ICANON | libc::ECHO);
    raw_termios.c_cc[libc::VMIN] = 0;
    raw_termios.c_cc[libc::VTIME] = 1; // 100ms timeout

    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw_termios) } != 0 {
        tracing::debug!("window_size_pixels(): tcsetattr failed");
        return None;
    }

    // Helper to restore terminal mode
    let restore = |fd: i32, termios: &libc::termios| {
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, termios) };
    };

    // Write CSI 14 t query
    let query = b"\x1b[14t";
    if tty.write_all(query).is_err() {
        tracing::debug!("window_size_pixels(): failed to write query");
        restore(fd, &orig_termios);
        return None;
    }
    let _ = tty.flush();

    // Read response with timeout
    let timeout = Duration::from_millis(100);
    let start = Instant::now();
    let mut buffer = Vec::with_capacity(32);
    let mut byte = [0u8; 1];

    while start.elapsed() < timeout {
        match tty.read(&mut byte) {
            Ok(1) => {
                buffer.push(byte[0]);
                // Check if we've received the terminating 't'
                if byte[0] == b't' && buffer.len() > 4 {
                    break;
                }
            }
            Ok(0) => {
                // No data available, continue waiting
                std::thread::sleep(Duration::from_millis(5));
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    // Restore terminal mode
    restore(fd, &orig_termios);

    // Parse response: \x1b[4;height;widtht
    let result = parse_csi_14t_response(&buffer);
    if result.is_some() {
        tracing::debug!("window_size_pixels(): detected {:?}", result);
    } else {
        tracing::debug!(
            "window_size_pixels(): failed to parse response: {:?}",
            String::from_utf8_lossy(&buffer)
        );
    }

    result
}

/// Get the terminal window size in pixels (non-Unix stub).
#[cfg(not(unix))]
pub fn window_size_pixels() -> Option<WindowSizePixels> {
    tracing::debug!("window_size_pixels(): not supported on this platform");
    None
}

/// Parse the CSI 14 t response format.
///
/// Expected format: `\x1b[4;height;widtht`
fn parse_csi_14t_response(response: &[u8]) -> Option<WindowSizePixels> {
    // Find the CSI sequence start
    let esc_pos = response.iter().position(|&b| b == 0x1b)?;
    let after_esc = &response[esc_pos..];

    // Validate CSI format: ESC [ 4 ; ... t
    if after_esc.len() < 5 {
        return None;
    }
    if after_esc[1] != b'[' {
        return None;
    }
    if after_esc[2] != b'4' {
        return None;
    }
    if after_esc[3] != b';' {
        return None;
    }

    // Find the terminating 't'
    let t_pos = after_esc.iter().position(|&b| b == b't')?;
    let params = &after_esc[4..t_pos];

    // Parse "height;width" from params
    let params_str = std::str::from_utf8(params).ok()?;
    let parts: Vec<&str> = params_str.split(';').collect();

    if parts.len() != 2 {
        return None;
    }

    let height: u32 = parts[0].parse().ok()?;
    let width: u32 = parts[1].parse().ok()?;

    Some(WindowSizePixels { width, height })
}

/// Calculate the cell size (font dimensions) in pixels.
///
/// Combines window pixel size with grid dimensions to calculate
/// the approximate width and height of a single character cell.
///
/// ## Returns
///
/// - `Some(CellSize)` if both window pixels and grid size can be determined
/// - `None` if either measurement fails
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::cell_size;
///
/// if let Some(size) = cell_size() {
///     println!("Cell size: {}x{} pixels", size.width, size.height);
/// }
/// ```
pub fn cell_size() -> Option<CellSize> {
    let window = window_size_pixels()?;
    let cols = crate::discovery::detection::terminal_width();
    let rows = crate::discovery::detection::terminal_height();

    if cols == 0 || rows == 0 {
        return None;
    }

    Some(CellSize {
        width: window.width / cols,
        height: window.height / rows,
    })
}

/// Get the font name used by the terminal.
///
/// Detects the font by parsing the terminal's configuration file or
/// querying system preferences (for macOS terminals like iTerm2).
///
/// ## Supported Terminals
///
/// | Terminal | Config Format | Font Setting |
/// |----------|--------------|--------------|
/// | Wezterm | Lua | `config.font = wezterm.font("Name")` |
/// | Ghostty | Key=Value | `font-family = Name` |
/// | Kitty | Conf | `font_family Name` |
/// | Alacritty | TOML | `[font.normal] family = "Name"` |
/// | iTerm2 | macOS prefs | `defaults read com.googlecode.iterm2` |
///
/// ## Returns
///
/// - `Some(String)` - The font family name from config
/// - `None` - If config not found, not readable, or font not specified
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::font_name;
///
/// if let Some(font) = font_name() {
///     println!("Terminal font: {}", font);
/// } else {
///     println!("Font not detected - assuming monospace");
/// }
/// ```
pub fn font_name() -> Option<String> {
    let app = get_terminal_app();

    // Handle terminals that use macOS preferences instead of config files
    #[cfg(target_os = "macos")]
    if matches!(app, TerminalApp::ITerm2) {
        let result = query_iterm2_font_name();
        if result.is_some() {
            tracing::debug!("font_name(): detected {:?} from iTerm2 preferences", result);
        }
        return result;
    }

    let config_path = get_terminal_config_path(&app)?;

    if !config_path.exists() {
        tracing::debug!(
            "font_name(): config file does not exist: {:?}",
            config_path
        );
        return None;
    }

    let content = fs::read_to_string(&config_path).ok()?;

    let result = match app {
        TerminalApp::Wezterm => parse_wezterm_font_name(&content),
        TerminalApp::Ghostty => parse_ghostty_font_name(&content),
        TerminalApp::Kitty => parse_kitty_font_name(&content),
        TerminalApp::Alacritty => parse_alacritty_font_name(&content),
        _ => {
            tracing::debug!(
                "font_name(): no parser for terminal {:?}, trying fallback scan",
                app
            );
            // Fallback: scan known config files
            return fallback_font_name_scan();
        }
    };

    if result.is_some() {
        tracing::debug!("font_name(): detected {:?}", result);
        return result;
    }

    // If primary detection failed, try fallback scan
    tracing::debug!("font_name(): primary detection failed, trying fallback scan");
    fallback_font_name_scan()
}

/// Get the font size in points.
///
/// Detects the font size by parsing the terminal's configuration file or
/// querying system preferences (for macOS terminals like iTerm2).
///
/// ## Supported Terminals
///
/// | Terminal | Config Format | Size Setting |
/// |----------|--------------|--------------|
/// | Wezterm | Lua | `config.font_size = 13` |
/// | Ghostty | Key=Value | `font-size = 14` |
/// | Kitty | Conf | `font_size 14.0` |
/// | Alacritty | TOML | `[font] size = 12` |
/// | iTerm2 | macOS prefs | `defaults read com.googlecode.iterm2` |
///
/// ## Returns
///
/// - `Some(u32)` - The font size in points
/// - `None` - If config not found, not readable, or size not specified
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::font_size;
///
/// if let Some(size) = font_size() {
///     println!("Font size: {}pt", size);
/// } else {
///     println!("Font size not detected");
/// }
/// ```
pub fn font_size() -> Option<u32> {
    let app = get_terminal_app();

    // Handle terminals that use macOS preferences instead of config files
    #[cfg(target_os = "macos")]
    if matches!(app, TerminalApp::ITerm2) {
        let result = query_iterm2_font_size();
        if result.is_some() {
            tracing::debug!("font_size(): detected {:?} from iTerm2 preferences", result);
        }
        return result;
    }

    let config_path = get_terminal_config_path(&app)?;

    if !config_path.exists() {
        tracing::debug!(
            "font_size(): config file does not exist: {:?}",
            config_path
        );
        return None;
    }

    let content = fs::read_to_string(&config_path).ok()?;

    let result = match app {
        TerminalApp::Wezterm => parse_wezterm_font_size(&content),
        TerminalApp::Ghostty => parse_ghostty_font_size(&content),
        TerminalApp::Kitty => parse_kitty_font_size(&content),
        TerminalApp::Alacritty => parse_alacritty_font_size(&content),
        _ => {
            tracing::debug!(
                "font_size(): no parser for terminal {:?}, trying fallback scan",
                app
            );
            // Fallback: scan known config files
            return fallback_font_size_scan();
        }
    };

    if result.is_some() {
        tracing::debug!("font_size(): detected {:?}", result);
        return result;
    }

    // If primary detection failed, try fallback scan
    tracing::debug!("font_size(): primary detection failed, trying fallback scan");
    fallback_font_size_scan()
}

/// Get the font ligatures enabled in the terminal.
///
/// Font ligatures are special glyphs that combine multiple characters
/// into a single glyph (e.g., `fi`, `fl`, `!=`, `=>`).
///
/// ## Why Detection Is Difficult
///
/// - The terminal's font rendering engine handles ligatures internally
/// - No standard escape sequence to query ligature support
/// - Support depends on both the terminal AND the font being used
/// - Users can enable/disable ligatures in terminal settings
///
/// ## Detection Strategies (Not Implemented)
///
/// 1. **Cursor Position Heuristic**: Render a known ligature (e.g., `fi`)
///    and check if cursor moves by 1 cell instead of 2 using CSI 6 n.
///    - Complex to implement (requires raw mode)
///    - Unreliable (depends on specific font glyphs)
///    - Can't detect all ligatures
///
/// 2. **Terminal Detection**: Identify terminal and assume ligature support
///    based on known defaults.
///    - Fragile (users can change fonts/settings)
///    - Requires maintaining a mapping of terminals
///
/// ## Recommended Approach
///
/// 1. **Assume support** - Most modern terminals support ligatures
/// 2. **User configuration** - Provide an option to disable ligatures
/// 3. **Document requirements** - State that ligatures require compatible font
///
/// ## Terminal Ligature Support (Known Defaults)
///
/// | Terminal | Default Ligature Support | Notes |
/// |----------|------------------------|-------|
/// | iTerm2   | ✅ Yes (if font supports) | Can be disabled in preferences |
/// | Kitty     | ✅ Yes (if font supports) | Uses custom font renderer |
/// | WezTerm   | ✅ Yes (if font supports) | Highly configurable |
/// | Alacritty | ✅ Yes (if font supports) | Since v0.8.0 |
/// | Ghostty   | ✅ Yes (if font supports) | Modern GPU-accelerated |
/// | Windows Terminal | ✅ Yes (if font supports) | Since v1.6 |
/// | VS Code Terminal | ✅ Yes (if font supports) | Inherits from editor settings |
/// | GNOME Terminal | ❌ Typically no | Uses system font rendering |
/// | Konsole   | ❌ Typically no | Uses system font rendering |
///
/// ## Returns
///
/// - `None` - Ligature support cannot be reliably detected
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::font_ligatures;
///
/// assert!(font_ligatures().is_none());
/// println!("Ligature detection not available - assume support for modern terminals");
/// ```
///
/// ## See Also
///
/// - [`FontLigature`](crate::discovery::detection::FontLigature) - Enumeration of common ligatures
pub fn font_ligatures() -> Option<Vec<FontLigature>> {
    // There is no reliable way to detect which ligatures are enabled.
    // The cursor position heuristic is complex and unreliable:
    // - Requires raw terminal mode
    // - Depends on specific font glyphs being present
    // - Can only test one ligature at a time
    // - May give false positives/negatives
    //
    // The recommended approach is to assume ligature support and provide
    // a user configuration option to disable them if needed.

    tracing::debug!(
        "font_ligatures() returns None - no reliable way to detect enabled ligatures"
    );

    None
}

// =============================================================================
// Config file parsers for font detection
// =============================================================================

/// Parse Wezterm Lua config for font name.
///
/// Looks for patterns like:
/// - `config.font = wezterm.font("JetBrains Mono")`
/// - `config.font = wezterm.font("JetBrains Mono", { weight = "Bold" })`
/// - `config.font = wezterm.font { family = "JetBrains Mono" }`
fn parse_wezterm_font_name(content: &str) -> Option<String> {
    // Pattern 1: config.font = wezterm.font("FontName"
    // Pattern 2: config.font = wezterm.font({ family = "FontName"
    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with("--") {
            continue;
        }

        // Look for config.font = wezterm.font(
        if line.contains("config.font") && line.contains("wezterm.font") {
            // Try to extract font name from wezterm.font("Name" or wezterm.font("Name",
            if let Some(start) = line.find("wezterm.font(\"") {
                let after_quote = &line[start + 14..]; // Skip 'wezterm.font("'
                if let Some(end) = after_quote.find('"') {
                    let font_name = &after_quote[..end];
                    if !font_name.is_empty() {
                        return Some(font_name.to_string());
                    }
                }
            }

            // Try alternate pattern: wezterm.font { family = "Name"
            if let Some(start) = line.find("family") {
                let after_family = &line[start..];
                // Find the quoted value after family =
                if let Some(quote_start) = after_family.find('"') {
                    let after_quote = &after_family[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find('"') {
                        let font_name = &after_quote[..quote_end];
                        if !font_name.is_empty() {
                            return Some(font_name.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Parse Wezterm Lua config for font size.
///
/// Looks for: `config.font_size = 13`
fn parse_wezterm_font_size(content: &str) -> Option<u32> {
    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with("--") {
            continue;
        }

        // Look for config.font_size = N
        if line.contains("config.font_size") {
            // Find the = sign and parse the number after it
            if let Some(eq_pos) = line.find('=') {
                let value_part = line[eq_pos + 1..].trim();
                // Parse as float first (Lua allows 13.0), then convert to u32
                if let Ok(size) = value_part.parse::<f64>() {
                    return Some(size as u32);
                }
            }
        }
    }
    None
}

/// Parse Ghostty config for font name.
///
/// First tries the config file, then falls back to `ghostty +show-config`
/// to get actual running values including defaults.
fn parse_ghostty_font_name(content: &str) -> Option<String> {
    // First try parsing the config file
    if let Some(name) = parse_ghostty_config_value(content, "font-family") {
        return Some(name);
    }

    // Fall back to querying Ghostty for its actual config (includes defaults)
    query_ghostty_config("font-family")
}

/// Parse Ghostty config for font size.
///
/// First tries the config file, then falls back to `ghostty +show-config`
/// to get actual running values including defaults.
fn parse_ghostty_font_size(content: &str) -> Option<u32> {
    // First try parsing the config file
    if let Some(value) = parse_ghostty_config_value(content, "font-size") {
        if let Ok(size) = value.parse::<f64>() {
            return Some(size as u32);
        }
    }

    // Fall back to querying Ghostty for its actual config
    if let Some(value) = query_ghostty_config("font-size") {
        if let Ok(size) = value.parse::<f64>() {
            return Some(size as u32);
        }
    }

    None
}

/// Parse a key-value pair from Ghostty config content.
fn parse_ghostty_config_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with('#') {
            continue;
        }

        // Look for key = value
        if line.starts_with(key) {
            if let Some(eq_pos) = line.find('=') {
                let value = line[eq_pos + 1..].trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// Query Ghostty for a config value using `ghostty +show-config`.
///
/// This returns actual running values including defaults, not just
/// what's in the config file.
fn query_ghostty_config(key: &str) -> Option<String> {
    use std::process::Command;

    let output = Command::new("ghostty")
        .args(["+show-config"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with(key) {
            if let Some(eq_pos) = line.find('=') {
                let value = line[eq_pos + 1..].trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

// =============================================================================
// iTerm2 config queries (macOS only)
// =============================================================================

/// Query iTerm2 for font name using macOS `defaults` command.
///
/// iTerm2 stores font settings in macOS preferences as "Normal Font" = "FontName Size".
/// For example: "Monaco 12" or "JetBrainsMono Nerd Font 14".
#[cfg(target_os = "macos")]
fn query_iterm2_font_name() -> Option<String> {
    use std::process::Command;

    // Query the "New Bookmarks" array which contains profile settings
    let output = Command::new("defaults")
        .args(["read", "com.googlecode.iterm2", "New Bookmarks"])
        .output()
        .ok()?;

    if !output.status.success() {
        tracing::debug!("query_iterm2_font_name(): defaults read failed");
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_iterm2_font_setting(&stdout, true)
}

/// Query iTerm2 for font size using macOS `defaults` command.
#[cfg(target_os = "macos")]
fn query_iterm2_font_size() -> Option<u32> {
    use std::process::Command;

    let output = Command::new("defaults")
        .args(["read", "com.googlecode.iterm2", "New Bookmarks"])
        .output()
        .ok()?;

    if !output.status.success() {
        tracing::debug!("query_iterm2_font_size(): defaults read failed");
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_iterm2_font_setting(&stdout, false).and_then(|s| s.parse::<u32>().ok())
}

/// Parse iTerm2 font setting from `defaults read` output.
///
/// The output format is a plist-like structure with "Normal Font" = "FontName Size".
/// We extract either the font name (without size) or just the size.
///
/// ## Arguments
/// * `content` - The output from `defaults read`
/// * `extract_name` - If true, extract font name; if false, extract size
#[cfg(target_os = "macos")]
fn parse_iterm2_font_setting(content: &str, extract_name: bool) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();

        // Look for "Normal Font" = "FontName Size";
        if line.contains("\"Normal Font\"") {
            // Extract the value between the last pair of quotes
            if let Some(eq_pos) = line.find('=') {
                let value_part = line[eq_pos + 1..].trim();
                // Remove surrounding quotes and semicolon
                let value = value_part
                    .trim_matches(|c| c == '"' || c == ';' || c == ' ');

                if value.is_empty() {
                    return None;
                }

                // Split "FontName Size" - the size is the last space-separated token
                // Handle fonts like "JetBrainsMono Nerd Font 14"
                if let Some(last_space) = value.rfind(' ') {
                    let potential_size = &value[last_space + 1..];
                    // Check if the last part is a number (the size)
                    if potential_size.parse::<f64>().is_ok() {
                        if extract_name {
                            // Return everything before the size
                            return Some(value[..last_space].to_string());
                        } else {
                            // Return just the size
                            return Some(potential_size.to_string());
                        }
                    }
                }

                // If we couldn't parse size, return the whole thing as name
                if extract_name {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// Parse Kitty config for font name.
///
/// Looks for: `font_family FiraCode Nerd Font`
fn parse_kitty_font_name(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with('#') {
            continue;
        }

        // Look for font_family <name> (space-separated, not =)
        if line.starts_with("font_family") {
            let value = line["font_family".len()..].trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Parse Kitty config for font size.
///
/// Looks for: `font_size 14.0`
fn parse_kitty_font_size(content: &str) -> Option<u32> {
    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with('#') {
            continue;
        }

        // Look for font_size N (space-separated)
        if line.starts_with("font_size") {
            let value = line["font_size".len()..].trim();
            if let Ok(size) = value.parse::<f64>() {
                return Some(size as u32);
            }
        }
    }
    None
}

/// Parse Alacritty TOML config for font name.
///
/// Looks for:
/// ```toml
/// [font.normal]
/// family = "JetBrains Mono"
/// ```
fn parse_alacritty_font_name(content: &str) -> Option<String> {
    let mut in_font_normal = false;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with('#') {
            continue;
        }

        // Track sections - we only care about [font.normal]
        if line.starts_with('[') {
            in_font_normal = line == "[font.normal]";
            continue;
        }

        // Look for family = "Name" in font.normal section
        if in_font_normal && line.starts_with("family") {
            if let Some(eq_pos) = line.find('=') {
                let value = line[eq_pos + 1..].trim();
                // Remove quotes
                let value = value.trim_matches('"').trim_matches('\'');
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// Parse Alacritty TOML config for font size.
///
/// Looks for:
/// ```toml
/// [font]
/// size = 12
/// ```
fn parse_alacritty_font_size(content: &str) -> Option<u32> {
    let mut in_font_section = false;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with('#') {
            continue;
        }

        // Track sections - we want [font] but not [font.normal] etc.
        if line.starts_with('[') {
            // [font] section but not subsections
            in_font_section = line == "[font]";
            continue;
        }

        // Look for size = N in [font] section
        if in_font_section && line.starts_with("size") {
            if let Some(eq_pos) = line.find('=') {
                let value = line[eq_pos + 1..].trim();
                if let Ok(size) = value.parse::<f64>() {
                    return Some(size as u32);
                }
            }
        }
    }
    None
}

// =============================================================================
// Fallback config scanning
// =============================================================================

/// Fallback font name detection by scanning known config file locations.
///
/// This is used when terminal detection fails or the detected terminal
/// doesn't have a config parser. It tries common config file locations
/// for popular terminals.
fn fallback_font_name_scan() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let home = std::path::Path::new(&home);

    // Define config files to try with their parsers
    let configs: &[(&std::path::Path, fn(&str) -> Option<String>)] = &[
        // Alacritty (common issue: doesn't set TERM_PROGRAM)
        (&home.join(".config/alacritty/alacritty.toml"), parse_alacritty_font_name),
        (&home.join(".config/alacritty/alacritty.yml"), parse_alacritty_font_name),
        // Kitty
        (&home.join(".config/kitty/kitty.conf"), parse_kitty_font_name),
        // Wezterm
        (&home.join(".config/wezterm/wezterm.lua"), parse_wezterm_font_name),
        (&home.join(".wezterm.lua"), parse_wezterm_font_name),
        // Ghostty
        (&home.join(".config/ghostty/config"), parse_ghostty_font_name),
    ];

    for (path, parser) in configs {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Some(font) = parser(&content) {
                    tracing::debug!(
                        "fallback_font_name_scan(): found font '{}' in {:?}",
                        font,
                        path
                    );
                    return Some(font);
                }
            }
        }
    }

    // Try iTerm2 on macOS
    #[cfg(target_os = "macos")]
    {
        if let Some(font) = query_iterm2_font_name() {
            tracing::debug!(
                "fallback_font_name_scan(): found font '{}' from iTerm2 preferences",
                font
            );
            return Some(font);
        }
    }

    tracing::debug!("fallback_font_name_scan(): no font found in any config files");
    None
}

/// Fallback font size detection by scanning known config file locations.
fn fallback_font_size_scan() -> Option<u32> {
    let home = std::env::var("HOME").ok()?;
    let home = std::path::Path::new(&home);

    // Define config files to try with their parsers
    let configs: &[(&std::path::Path, fn(&str) -> Option<u32>)] = &[
        // Alacritty (common issue: doesn't set TERM_PROGRAM)
        (&home.join(".config/alacritty/alacritty.toml"), parse_alacritty_font_size),
        (&home.join(".config/alacritty/alacritty.yml"), parse_alacritty_font_size),
        // Kitty
        (&home.join(".config/kitty/kitty.conf"), parse_kitty_font_size),
        // Wezterm
        (&home.join(".config/wezterm/wezterm.lua"), parse_wezterm_font_size),
        (&home.join(".wezterm.lua"), parse_wezterm_font_size),
        // Ghostty
        (&home.join(".config/ghostty/config"), parse_ghostty_font_size),
    ];

    for (path, parser) in configs {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Some(size) = parser(&content) {
                    tracing::debug!(
                        "fallback_font_size_scan(): found size {} in {:?}",
                        size,
                        path
                    );
                    return Some(size);
                }
            }
        }
    }

    // Try iTerm2 on macOS
    #[cfg(target_os = "macos")]
    {
        if let Some(size) = query_iterm2_font_size() {
            tracing::debug!(
                "fallback_font_size_scan(): found size {} from iTerm2 preferences",
                size
            );
            return Some(size);
        }
    }

    tracing::debug!("fallback_font_size_scan(): no font size found in any config files");
    None
}

/// Check if the terminal is likely to support font ligatures.
///
/// This is a heuristic based on the detected terminal emulator.
/// It does **not** check if ligatures are actually enabled or if
/// the current font supports them.
///
/// ## Returns
///
/// - `true` - Terminal is likely to support ligatures
/// - `false` - Terminal typically does not support ligatures
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::ligature_support_likely;
///
/// if ligature_support_likely() {
///     println!("Terminal likely supports ligatures (e.g., ->, =>, !=)");
/// } else {
///     println!("Terminal typically does not support ligatures");
/// }
/// ```
pub fn ligature_support_likely() -> bool {
    // If not a TTY, no styling support
    if !is_tty() {
        return false;
    }

    let term_app = get_terminal_app();

    matches!(
        term_app,
        TerminalApp::ITerm2
            | TerminalApp::Kitty
            | TerminalApp::Alacritty
            | TerminalApp::Wezterm
            | TerminalApp::Ghostty
            | TerminalApp::Warp
            | TerminalApp::VsCode
            | TerminalApp::Wast
            | TerminalApp::Contour
            | TerminalApp::Foot
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Wezterm parser tests
    // =========================================================================

    #[test]
    fn test_parse_wezterm_font_name_basic() {
        let config = r#"
            local wezterm = require("wezterm")
            config.font = wezterm.font("JetBrains Mono")
        "#;
        assert_eq!(
            parse_wezterm_font_name(config),
            Some("JetBrains Mono".to_string())
        );
    }

    #[test]
    fn test_parse_wezterm_font_name_with_options() {
        let config = r#"
            config.font = wezterm.font("Fira Code", { weight = "Bold" })
        "#;
        assert_eq!(
            parse_wezterm_font_name(config),
            Some("Fira Code".to_string())
        );
    }

    #[test]
    fn test_parse_wezterm_font_name_ignores_comments() {
        let config = r#"
            -- config.font = wezterm.font("Commented Out")
            config.font = wezterm.font("Actual Font")
        "#;
        assert_eq!(
            parse_wezterm_font_name(config),
            Some("Actual Font".to_string())
        );
    }

    #[test]
    fn test_parse_wezterm_font_name_no_match() {
        let config = r#"
            config.color_scheme = "Dracula"
        "#;
        assert_eq!(parse_wezterm_font_name(config), None);
    }

    #[test]
    fn test_parse_wezterm_font_size_integer() {
        let config = r#"
            config.font_size = 13
        "#;
        assert_eq!(parse_wezterm_font_size(config), Some(13));
    }

    #[test]
    fn test_parse_wezterm_font_size_float() {
        let config = r#"
            config.font_size = 14.5
        "#;
        assert_eq!(parse_wezterm_font_size(config), Some(14));
    }

    #[test]
    fn test_parse_wezterm_font_size_ignores_comments() {
        let config = r#"
            -- config.font_size = 99
            config.font_size = 12
        "#;
        assert_eq!(parse_wezterm_font_size(config), Some(12));
    }

    // =========================================================================
    // Ghostty parser tests
    // =========================================================================

    #[test]
    fn test_parse_ghostty_font_name() {
        let config = r#"
            # Ghostty config
            font-family = Iosevka Term
        "#;
        assert_eq!(
            parse_ghostty_font_name(config),
            Some("Iosevka Term".to_string())
        );
    }

    #[test]
    fn test_parse_ghostty_font_name_ignores_comments() {
        let config = r#"
            # font-family = Commented
            font-family = Active
        "#;
        assert_eq!(
            parse_ghostty_font_name(config),
            Some("Active".to_string())
        );
    }

    #[test]
    fn test_parse_ghostty_font_size() {
        let config = r#"
            font-size = 14
        "#;
        assert_eq!(parse_ghostty_font_size(config), Some(14));
    }

    #[test]
    fn test_parse_ghostty_font_size_float() {
        let config = r#"
            font-size = 13.5
        "#;
        assert_eq!(parse_ghostty_font_size(config), Some(13));
    }

    // =========================================================================
    // Kitty parser tests
    // =========================================================================

    #[test]
    fn test_parse_kitty_font_name() {
        let config = r#"
            # kitty.conf
            font_family FiraCode Nerd Font Mono
        "#;
        assert_eq!(
            parse_kitty_font_name(config),
            Some("FiraCode Nerd Font Mono".to_string())
        );
    }

    #[test]
    fn test_parse_kitty_font_name_ignores_comments() {
        let config = r#"
            #font_family Commented
            font_family Active Font
        "#;
        assert_eq!(
            parse_kitty_font_name(config),
            Some("Active Font".to_string())
        );
    }

    #[test]
    fn test_parse_kitty_font_size() {
        let config = r#"
            font_size 14.0
        "#;
        assert_eq!(parse_kitty_font_size(config), Some(14));
    }

    #[test]
    fn test_parse_kitty_font_size_integer() {
        let config = r#"
            font_size 12
        "#;
        assert_eq!(parse_kitty_font_size(config), Some(12));
    }

    // =========================================================================
    // Alacritty parser tests
    // =========================================================================

    #[test]
    fn test_parse_alacritty_font_name() {
        let config = r#"
            [font.normal]
            family = "JetBrains Mono"
            style = "Regular"
        "#;
        assert_eq!(
            parse_alacritty_font_name(config),
            Some("JetBrains Mono".to_string())
        );
    }

    #[test]
    fn test_parse_alacritty_font_name_single_quotes() {
        let config = r#"
            [font.normal]
            family = 'Fira Code'
        "#;
        assert_eq!(
            parse_alacritty_font_name(config),
            Some("Fira Code".to_string())
        );
    }

    #[test]
    fn test_parse_alacritty_font_name_ignores_other_sections() {
        let config = r#"
            [font.bold]
            family = "Bold Font"

            [font.normal]
            family = "Normal Font"
        "#;
        assert_eq!(
            parse_alacritty_font_name(config),
            Some("Normal Font".to_string())
        );
    }

    #[test]
    fn test_parse_alacritty_font_size() {
        let config = r#"
            [font]
            size = 12
        "#;
        assert_eq!(parse_alacritty_font_size(config), Some(12));
    }

    #[test]
    fn test_parse_alacritty_font_size_float() {
        let config = r#"
            [font]
            size = 11.5
        "#;
        assert_eq!(parse_alacritty_font_size(config), Some(11));
    }

    #[test]
    fn test_parse_alacritty_font_size_not_in_subsection() {
        let config = r#"
            [font.normal]
            size = 99

            [font]
            size = 12
        "#;
        // Should only find the one in [font], not [font.normal]
        assert_eq!(parse_alacritty_font_size(config), Some(12));
    }

    // =========================================================================
    // Public API tests
    // =========================================================================

    #[test]
    fn test_font_name_does_not_panic() {
        // font_name() should not panic regardless of environment
        let _ = font_name();
    }

    #[test]
    fn test_font_size_does_not_panic() {
        // font_size() should not panic regardless of environment
        let _ = font_size();
    }

    #[test]
    fn test_font_ligatures_returns_none() {
        // font_ligatures() should always return None (not implemented)
        assert!(font_ligatures().is_none());
    }

    #[test]
    #[ignore = "opens /dev/tty and sends escape sequences - run manually in real terminal"]
    fn test_window_size_pixels_does_not_panic() {
        // window_size_pixels() should not panic regardless of environment
        // In a non-TTY test environment, it returns None; in a real terminal it may succeed
        let _ = window_size_pixels();
    }

    #[test]
    fn test_ligature_support_likely_does_not_panic() {
        // Should not panic regardless of environment
        let _ = ligature_support_likely();
    }

    #[test]
    fn test_window_size_pixels_struct() {
        let size = WindowSizePixels {
            width: 1920,
            height: 1080,
        };
        assert_eq!(size.width, 1920);
        assert_eq!(size.height, 1080);
    }

    #[test]
    fn test_cell_size_struct() {
        let size = CellSize {
            width: 10,
            height: 20,
        };
        assert_eq!(size.width, 10);
        assert_eq!(size.height, 20);
    }

    // =========================================================================
    // Nerd Font detection tests
    // =========================================================================

    #[test]
    fn test_is_nerd_font_name_with_nerd_font_suffix() {
        assert!(is_nerd_font_name("JetBrainsMono Nerd Font"));
        assert!(is_nerd_font_name("Hack Nerd Font Mono"));
        assert!(is_nerd_font_name("Fira Code Nerd Font"));
        assert!(is_nerd_font_name("Meslo LG S Nerd Font"));
    }

    #[test]
    fn test_is_nerd_font_name_with_nf_suffix() {
        assert!(is_nerd_font_name("FiraCode NF"));
        assert!(is_nerd_font_name("Meslo LG M NF"));
        assert!(is_nerd_font_name("Hack NF"));
        assert!(is_nerd_font_name("JetBrainsMono NF Mono"));
    }

    #[test]
    fn test_is_nerd_font_name_case_insensitive() {
        assert!(is_nerd_font_name("jetbrainsmono nerd font"));
        assert!(is_nerd_font_name("HACK NF"));
        assert!(is_nerd_font_name("FiraCode NERD FONT"));
    }

    #[test]
    fn test_is_nerd_font_name_non_nerd_fonts() {
        // These are fonts that are NOT in the Nerd Font family
        assert!(!is_nerd_font_name("Monaco"));
        assert!(!is_nerd_font_name("SF Mono"));
        assert!(!is_nerd_font_name("Menlo"));
        assert!(!is_nerd_font_name("Courier New"));
        assert!(!is_nerd_font_name("Consolas"));
        assert!(!is_nerd_font_name("Arial"));
        assert!(!is_nerd_font_name("Helvetica"));
    }

    #[test]
    fn test_is_nerd_font_name_base_names_recognized() {
        // Known Nerd Font base names should be recognized
        // (these fonts have Nerd Font patched versions available)
        assert!(is_nerd_font_name("JetBrains Mono"));
        assert!(is_nerd_font_name("JetBrainsMono"));
        assert!(is_nerd_font_name("Fira Code"));
        assert!(is_nerd_font_name("FiraCode"));
        assert!(is_nerd_font_name("Hack"));
        assert!(is_nerd_font_name("Meslo"));
        assert!(is_nerd_font_name("Iosevka"));
        assert!(is_nerd_font_name("Victor Mono"));
    }

    #[test]
    fn test_is_nerd_font_name_with_style_suffix() {
        // Font names with style suffixes should still match
        assert!(is_nerd_font_name("JetBrainsMono Regular"));
        assert!(is_nerd_font_name("Hack Bold"));
        assert!(is_nerd_font_name("FiraCode Light"));
    }

    #[test]
    #[serial_test::serial]
    fn test_detect_nerd_font_env_var_true() {
        // SAFETY: Test runs serially, no concurrent env access
        unsafe { std::env::set_var("NERD_FONT", "1") };
        let result = detect_nerd_font();
        unsafe { std::env::remove_var("NERD_FONT") };
        assert_eq!(result, Some(true));
    }

    #[test]
    #[serial_test::serial]
    fn test_detect_nerd_font_env_var_true_word() {
        // SAFETY: Test runs serially, no concurrent env access
        unsafe { std::env::set_var("NERD_FONT", "true") };
        let result = detect_nerd_font();
        unsafe { std::env::remove_var("NERD_FONT") };
        assert_eq!(result, Some(true));
    }

    #[test]
    #[serial_test::serial]
    fn test_detect_nerd_font_env_var_false() {
        // SAFETY: Test runs serially, no concurrent env access
        unsafe { std::env::set_var("NERD_FONT", "0") };
        let result = detect_nerd_font();
        unsafe { std::env::remove_var("NERD_FONT") };
        assert_eq!(result, Some(false));
    }

    #[test]
    #[serial_test::serial]
    fn test_detect_nerd_font_env_var_false_word() {
        // SAFETY: Test runs serially, no concurrent env access
        unsafe { std::env::set_var("NERD_FONT", "false") };
        let result = detect_nerd_font();
        unsafe { std::env::remove_var("NERD_FONT") };
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_detect_nerd_font_does_not_panic() {
        // Should not panic regardless of environment
        let _ = detect_nerd_font();
    }

    // =========================================================================
    // CSI 14 t response parsing tests
    // =========================================================================

    #[test]
    fn test_parse_csi_14t_response_valid() {
        // Standard response: ESC[4;1080;1920t
        let response = b"\x1b[4;1080;1920t";
        let result = parse_csi_14t_response(response);
        assert_eq!(result, Some(WindowSizePixels { width: 1920, height: 1080 }));
    }

    #[test]
    fn test_parse_csi_14t_response_with_prefix() {
        // Response may have garbage before it
        let response = b"garbage\x1b[4;600;800t";
        let result = parse_csi_14t_response(response);
        assert_eq!(result, Some(WindowSizePixels { width: 800, height: 600 }));
    }

    #[test]
    fn test_parse_csi_14t_response_empty() {
        let response = b"";
        assert_eq!(parse_csi_14t_response(response), None);
    }

    #[test]
    fn test_parse_csi_14t_response_no_esc() {
        let response = b"[4;100;200t";
        assert_eq!(parse_csi_14t_response(response), None);
    }

    #[test]
    fn test_parse_csi_14t_response_wrong_command() {
        // CSI 5 instead of CSI 4
        let response = b"\x1b[5;100;200t";
        assert_eq!(parse_csi_14t_response(response), None);
    }

    #[test]
    fn test_parse_csi_14t_response_missing_semicolon() {
        let response = b"\x1b[4;100t";
        // Only one number, should fail
        assert_eq!(parse_csi_14t_response(response), None);
    }

    #[test]
    fn test_parse_csi_14t_response_non_numeric() {
        let response = b"\x1b[4;abc;deft";
        assert_eq!(parse_csi_14t_response(response), None);
    }

    #[test]
    fn test_cell_size_does_not_panic() {
        // cell_size() should not panic regardless of environment
        let _ = cell_size();
    }

    // =========================================================================
    // iTerm2 config parsing tests (macOS only)
    // =========================================================================

    #[test]
    #[cfg(target_os = "macos")]
    fn test_parse_iterm2_font_setting_basic() {
        let content = r#"
            (
                {
                    "Normal Font" = "Monaco 12";
                }
            )
        "#;
        assert_eq!(
            parse_iterm2_font_setting(content, true),
            Some("Monaco".to_string())
        );
        assert_eq!(
            parse_iterm2_font_setting(content, false),
            Some("12".to_string())
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_parse_iterm2_font_setting_nerd_font() {
        let content = r#"
            (
                {
                    "Normal Font" = "JetBrainsMono Nerd Font 14";
                }
            )
        "#;
        assert_eq!(
            parse_iterm2_font_setting(content, true),
            Some("JetBrainsMono Nerd Font".to_string())
        );
        assert_eq!(
            parse_iterm2_font_setting(content, false),
            Some("14".to_string())
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_parse_iterm2_font_setting_with_spaces() {
        let content = r#"
            (
                {
                    "Normal Font" = "SF Mono 13";
                }
            )
        "#;
        assert_eq!(
            parse_iterm2_font_setting(content, true),
            Some("SF Mono".to_string())
        );
        assert_eq!(
            parse_iterm2_font_setting(content, false),
            Some("13".to_string())
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_parse_iterm2_font_setting_float_size() {
        let content = r#"
            (
                {
                    "Normal Font" = "Menlo 12.5";
                }
            )
        "#;
        assert_eq!(
            parse_iterm2_font_setting(content, true),
            Some("Menlo".to_string())
        );
        // Size parsing handles floats
        assert_eq!(
            parse_iterm2_font_setting(content, false),
            Some("12.5".to_string())
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_parse_iterm2_font_setting_no_match() {
        let content = r#"
            (
                {
                    "Other Setting" = "value";
                }
            )
        "#;
        assert_eq!(parse_iterm2_font_setting(content, true), None);
        assert_eq!(parse_iterm2_font_setting(content, false), None);
    }
}
