use std::path::PathBuf;

use crate::discovery::config_paths::get_terminal_config_path;
use crate::discovery::detection::{
    ColorDepth, ColorMode, ImageSupport, TerminalApp, UnderlineSupport, color_depth, color_mode,
    get_terminal_app, image_support, is_tty, italics_support, osc8_link_support, terminal_height,
    terminal_width, underline_support,
};
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
}
