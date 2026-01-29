use std::path::PathBuf;

use crate::discovery::config_paths::get_terminal_config_path;
use crate::discovery::detection::{
    ColorDepth, ColorMode, Connection, ImageSupport, TerminalApp, UnderlineSupport,
    color_depth, color_mode, detect_connection, get_terminal_app, image_support,
    is_tty, italics_support, osc8_link_support, terminal_height, terminal_width,
    underline_support,
};
use crate::discovery::fonts::{detect_nerd_font, font_ligatures, font_name, font_size, FontLigature};
use crate::discovery::locale::{CharEncoding, TerminalLocale};
use crate::discovery::os_detection::{
    LinuxDistro, OsType, detect_linux_distro, detect_os_type, is_ci,
};

fn new_terminal() -> Terminal {
    let app = get_terminal_app();
    let config_file = get_terminal_config_path(&app);

    Terminal {
        app,
        supports_italic: italics_support(),
        image_support: image_support(),
        underline_support: underline_support(),
        osc_link_support: osc8_link_support(),
        is_tty: is_tty(),
        color_depth: color_depth(),
        os: detect_os_type(),
        distro: detect_linux_distro(),
        config_file,
        is_ci: is_ci(),
        font: font_name(),
        font_size: font_size(),
        font_ligatures: font_ligatures(),
        is_nerd_font: detect_nerd_font(),
        remote: detect_connection(),
        char_encoding: CharEncoding::default(),
        locale: TerminalLocale::default(),
    }
}



/// Represents a detected terminal environment with its capabilities.
///
/// This struct aggregates all detected terminal information including
/// the terminal application, OS details, and various capability flags.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::terminal::Terminal;
///
/// let term = Terminal::new();
/// println!("Terminal: {:?}", term.app);
/// println!("OS: {:?}", term.os);
/// println!("Is CI: {}", term.is_ci);
///
/// if let Some(config) = &term.config_file {
///     println!("Config file: {:?}", config);
/// }
/// ```
pub struct Terminal {
    /// The app/vendor of the terminal
    pub app: TerminalApp,

    /// Whether the terminal supports italicizing text
    pub supports_italic: bool,
    /// The type of image support (if any) the terminal provides
    pub image_support: ImageSupport,
    /// The kind of **underlining** support the terminal provides
    pub underline_support: UnderlineSupport,
    /// Whether the terminal supports OSC8 Links
    pub osc_link_support: bool,

    /// Whether stdout is connected to a TTY
    pub is_tty: bool,
    /// The color depth supported by the terminal
    pub color_depth: ColorDepth,

    /// The operating system type
    pub os: OsType,
    /// Linux distribution details (None on non-Linux)
    pub distro: Option<LinuxDistro>,
    /// Path to terminal config file (if detectable)
    pub config_file: Option<PathBuf>,
    /// Whether running in a CI environment
    pub is_ci: bool,
    /// The font the terminal is using (if accessible)
    pub font: Option<String>,
    /// The font size the terminal is using (if accessible)
    pub font_size: Option<u32>,
    /// The font ligatures the terminal is using (if accessible)
    pub font_ligatures: Option<Vec<FontLigature>>,
    /// Whether the terminal is using a Nerd Font.
    ///
    /// Detection uses:
    /// 1. `NERD_FONT` environment variable (explicit user declaration)
    /// 2. Font name pattern matching against known Nerd Font families
    ///
    /// - `Some(true)`: Nerd Font confirmed
    /// - `Some(false)`: Explicitly disabled via env var
    /// - `None`: Cannot determine
    pub is_nerd_font: Option<bool>,

    /// Information about the remote connection (if it exists)
    pub remote: Connection,

    /// What character encoding is this terminal using (typically UTF-8)
    pub char_encoding: CharEncoding,

    /// The detected locale which the terminal is reporting via environment
    /// variables (`LC_ALL`, `LC_CTYPE`, `LANG`)
    pub locale: TerminalLocale,
}

impl Default for Terminal {
    fn default() -> Terminal {
        new_terminal()
    }
}

impl Terminal {
    /// Create a new Terminal instance with detected capabilities.
    ///
    /// This constructor queries the terminal environment to detect:
    /// - Terminal application (WezTerm, Kitty, iTerm2, etc.)
    /// - Operating system and Linux distribution
    /// - Color depth and mode (light/dark)
    /// - Feature support (italics, images, underlines, OSC8 links)
    /// - Configuration file path
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::terminal::Terminal;
    ///
    /// let term = Terminal::new();
    /// println!("Terminal: {:?}", term.app);
    /// println!("OS: {:?}", term.os);
    ///
    /// if term.supports_italic {
    ///     println!("Italics are supported!");
    /// }
    /// ```
    pub fn new() -> Terminal {
        new_terminal()
    }

    /// Get the terminal width in columns.
    ///
    /// Returns 80 as a fallback if the terminal size cannot be determined.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::terminal::Terminal;
    ///
    /// let width = Terminal::width();
    /// println!("Terminal is {} columns wide", width);
    /// ```
    pub fn width() -> u32 {
        terminal_width()
    }

    /// Get the terminal height in rows.
    ///
    /// Returns 24 as a fallback if the terminal size cannot be determined.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::terminal::Terminal;
    ///
    /// let height = Terminal::height();
    /// println!("Terminal is {} rows tall", height);
    /// ```
    pub fn height() -> u32 {
        terminal_height()
    }

    /// Detect whether the terminal is in "light" or "dark" mode.
    ///
    /// Detection strategy:
    /// 1. Query background color luminance via OSC heuristics
    /// 2. Check `DARK_MODE` environment variable
    /// 3. On macOS, check system `AppleInterfaceStyle`
    /// 4. Default to Dark (most common for terminal users)
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::terminal::Terminal;
    /// use biscuit_terminal::discovery::detection::ColorMode;
    ///
    /// match Terminal::color_mode() {
    ///     ColorMode::Light => println!("Light mode - use dark colors"),
    ///     ColorMode::Dark => println!("Dark mode - use light colors"),
    ///     ColorMode::Unknown => println!("Unknown mode"),
    /// }
    /// ```
    pub fn color_mode() -> ColorMode {
        color_mode()
    }

    /// Render content to the terminal (placeholder for future implementation).
    pub fn render<T: Into<String>>(_content: T) -> () {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_new_creates_valid_instance() {
        let term = Terminal::new();
        // OS should be detected correctly on the current platform
        #[cfg(target_os = "macos")]
        assert_eq!(term.os, OsType::MacOS);
        #[cfg(target_os = "linux")]
        assert_eq!(term.os, OsType::Linux);
        #[cfg(target_os = "windows")]
        assert_eq!(term.os, OsType::Windows);
    }

    #[test]
    fn test_terminal_default_works() {
        let term = Terminal::default();
        // Should have a valid OS type
        assert!(!matches!(term.os, OsType::Unknown));
    }

    #[test]
    fn test_terminal_has_expected_fields() {
        let term = Terminal::new();
        // Verify all new fields are accessible
        let _os = term.os;
        let _distro = &term.distro;
        let _config = &term.config_file;
        let _ci = term.is_ci;
    }

    #[test]
    fn test_terminal_distro_none_on_non_linux() {
        let term = Terminal::new();
        #[cfg(not(target_os = "linux"))]
        assert!(term.distro.is_none());
    }

    #[test]
    fn test_terminal_has_font_fields() {
        let term = Terminal::new();
        // Font fields should be accessible and have Option types
        let _font = &term.font;
        let _font_size = &term.font_size;
        let _font_ligatures = &term.font_ligatures;
    }

    #[test]
    fn test_terminal_font_fields_do_not_panic() {
        let term = Terminal::new();
        // Font detection via config parsing may or may not return values
        // depending on the terminal and config. Just verify no panics.
        let _font = &term.font;
        let _font_size = term.font_size;
        // font_ligatures is still unimplemented (always None)
        assert!(term.font_ligatures.is_none(), "font_ligatures detection is not implemented");
    }

    #[test]
    fn test_terminal_has_is_nerd_font_field() {
        let term = Terminal::new();
        // is_nerd_font field should be accessible
        let _nerd_font = term.is_nerd_font;
    }

    #[test]
    fn test_terminal_is_nerd_font_does_not_panic() {
        let term = Terminal::new();
        // Nerd font detection may return Some(true), Some(false), or None
        // depending on environment. Just verify no panics.
        match term.is_nerd_font {
            Some(true) => {}
            Some(false) => {}
            None => {}
        }
    }
}
