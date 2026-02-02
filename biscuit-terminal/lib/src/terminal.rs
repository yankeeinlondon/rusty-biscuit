use std::path::PathBuf;

use crate::discovery::config_paths::get_terminal_config_path;
use crate::discovery::detection::{
    ColorDepth, ColorMode, Connection, ImageSupport, TerminalApp, UnderlineSupport, color_depth,
    color_mode, detect_connection, get_terminal_app, image_support, is_tty, italics_support,
    osc8_link_support, terminal_height, terminal_width, underline_support,
};
use crate::discovery::fonts::{
    FontLigature, detect_nerd_font, font_ligatures, font_name, font_size,
};
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
        fixed_width: None,
        fixed_height: None,
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
#[derive(Debug,Clone)]
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

    /// Fixed terminal width (columns). When `Some`, this overrides dynamic detection.
    /// When `None`, width is queried dynamically via `terminal_width()`.
    pub fixed_width: Option<u32>,

    /// Fixed terminal height (rows). When `Some`, this overrides dynamic detection.
    /// When `None`, height is queried dynamically via `terminal_height()`.
    pub fixed_height: Option<u32>,
}

impl Default for Terminal {
    fn default() -> Terminal {
        new_terminal()
    }
}

impl From<&Terminal> for Terminal {
    fn from(value: &Terminal) -> Self {
        Terminal {
            app: value.app.clone(),
            supports_italic: value.supports_italic,
            image_support: value.image_support.clone(),
            underline_support: value.underline_support.clone(),
            osc_link_support: value.osc_link_support,
            is_tty: value.is_tty,
            color_depth: value.color_depth.clone(),
            os: value.os.clone(),
            distro: value.distro.clone(),
            config_file: value.config_file.clone(),
            is_ci: value.is_ci,
            font: value.font.clone(),
            font_size: value.font_size,
            font_ligatures: value.font_ligatures.clone(),
            is_nerd_font: value.is_nerd_font,
            remote: value.remote.clone(),
            char_encoding: value.char_encoding.clone(),
            locale: value.locale.clone(),
            fixed_width: value.fixed_width,
            fixed_height: value.fixed_height,
        }
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

    /// Creates a new Terminal which sets the `is_tty` property to `true`.
    pub fn new_tty() -> Terminal {
        Terminal {
            is_tty: true,
            ..Terminal::default()
        }
    }

    /// Get the terminal width in columns.
    ///
    /// If a fixed width was set via the builder, returns that value.
    /// Otherwise, queries the terminal dynamically.
    /// Returns 80 as a fallback if the terminal size cannot be determined.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::terminal::Terminal;
    ///
    /// let term = Terminal::new();
    /// let width = term.width();
    /// println!("Terminal is {} columns wide", width);
    /// ```
    pub fn width(&self) -> u32 {
        self.fixed_width.unwrap_or_else(terminal_width)
    }

    /// Get the terminal height in rows.
    ///
    /// If a fixed height was set via the builder, returns that value.
    /// Otherwise, queries the terminal dynamically.
    /// Returns 24 as a fallback if the terminal size cannot be determined.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::terminal::Terminal;
    ///
    /// let term = Terminal::new();
    /// let height = term.height();
    /// println!("Terminal is {} rows tall", height);
    /// ```
    pub fn height(&self) -> u32 {
        self.fixed_height.unwrap_or_else(terminal_height)
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

    /// Render content to the terminal with default layout.
    ///
    /// This is a convenience method that:
    /// 1. Creates a new Terminal instance to detect capabilities
    /// 2. Creates a default Layout
    /// 3. Applies the layout with fallback rendering (respecting terminal capabilities)
    /// 4. Prints the output to stdout
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use biscuit_terminal::terminal::Terminal;
    ///
    /// Terminal::render("Hello, world!");
    /// Terminal::render("Formatted {{bold}}text{{reset}}");
    /// ```
    pub fn render<T: Into<String>>(content: T) {
        use crate::components::renderable::RenderableWrapper;
        use crate::utils::layout::Layout;

        let term = Terminal::new();
        let layout = Layout::default();
        let output = layout.fallback_render(content, &term);
        print!("{}", output);
    }

    /// Create a builder for constructing Terminal with explicit values.
    ///
    /// The builder allows overriding auto-detected values, useful for testing
    /// or when detection doesn't match the actual terminal capabilities.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::terminal::Terminal;
    /// use biscuit_terminal::discovery::detection::ImageSupport;
    ///
    /// let term = Terminal::builder()
    ///     .is_tty(true)
    ///     .image_support(ImageSupport::Kitty)
    ///     .build();
    ///
    /// assert!(term.is_tty);
    /// assert_eq!(term.image_support, ImageSupport::Kitty);
    /// ```
    pub fn builder() -> TerminalBuilder {
        TerminalBuilder::default()
    }
}

/// Builder for constructing [`Terminal`] with explicit values.
///
/// Any field not explicitly set will use the auto-detected value.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::terminal::Terminal;
/// use biscuit_terminal::discovery::detection::{ImageSupport, ColorDepth};
///
/// // Override specific values, auto-detect the rest
/// let term = Terminal::builder()
///     .is_tty(true)
///     .image_support(ImageSupport::Kitty)
///     .color_depth(ColorDepth::TrueColor)
///     .build();
/// ```
#[derive(Default)]
pub struct TerminalBuilder {
    app: Option<TerminalApp>,
    supports_italic: Option<bool>,
    image_support: Option<ImageSupport>,
    underline_support: Option<UnderlineSupport>,
    osc_link_support: Option<bool>,
    is_tty: Option<bool>,
    color_depth: Option<ColorDepth>,
    is_ci: Option<bool>,
    is_nerd_font: Option<Option<bool>>,
    fixed_width: Option<u32>,
    fixed_height: Option<u32>,
}

impl TerminalBuilder {
    /// Set the terminal application.
    pub fn app(mut self, value: TerminalApp) -> Self {
        self.app = Some(value);
        self
    }

    /// Set whether italics are supported.
    pub fn supports_italic(mut self, value: bool) -> Self {
        self.supports_italic = Some(value);
        self
    }

    /// Set the image support level.
    pub fn image_support(mut self, value: ImageSupport) -> Self {
        self.image_support = Some(value);
        self
    }

    /// Set the underline support level.
    pub fn underline_support(mut self, value: UnderlineSupport) -> Self {
        self.underline_support = Some(value);
        self
    }

    /// Set whether OSC8 links are supported.
    pub fn osc_link_support(mut self, value: bool) -> Self {
        self.osc_link_support = Some(value);
        self
    }

    /// Set whether stdout is connected to a TTY.
    pub fn is_tty(mut self, value: bool) -> Self {
        self.is_tty = Some(value);
        self
    }

    /// Set the color depth.
    pub fn color_depth(mut self, value: ColorDepth) -> Self {
        self.color_depth = Some(value);
        self
    }

    /// Set whether running in CI environment.
    pub fn is_ci(mut self, value: bool) -> Self {
        self.is_ci = Some(value);
        self
    }

    /// Set whether using a Nerd Font.
    ///
    /// - `Some(true)`: Nerd Font confirmed
    /// - `Some(false)`: Explicitly not a Nerd Font
    /// - `None`: Cannot determine
    pub fn is_nerd_font(mut self, value: Option<bool>) -> Self {
        self.is_nerd_font = Some(value);
        self
    }

    /// Set a fixed terminal width (columns).
    ///
    /// When set, `Terminal::width()` returns this value instead of
    /// querying the terminal dynamically. Useful for testing or
    /// rendering to a specific width.
    pub fn width(mut self, value: u32) -> Self {
        self.fixed_width = Some(value);
        self
    }

    /// Set a fixed terminal height (rows).
    ///
    /// When set, `Terminal::height()` returns this value instead of
    /// querying the terminal dynamically. Useful for testing or
    /// rendering to a specific height.
    pub fn height(mut self, value: u32) -> Self {
        self.fixed_height = Some(value);
        self
    }

    /// Build the Terminal, using auto-detected values for unset fields.
    pub fn build(self) -> Terminal {
        let detected = new_terminal();
        Terminal {
            app: self.app.unwrap_or(detected.app),
            supports_italic: self.supports_italic.unwrap_or(detected.supports_italic),
            image_support: self.image_support.unwrap_or(detected.image_support),
            underline_support: self.underline_support.unwrap_or(detected.underline_support),
            osc_link_support: self.osc_link_support.unwrap_or(detected.osc_link_support),
            is_tty: self.is_tty.unwrap_or(detected.is_tty),
            color_depth: self.color_depth.unwrap_or(detected.color_depth),
            is_ci: self.is_ci.unwrap_or(detected.is_ci),
            is_nerd_font: self.is_nerd_font.unwrap_or(detected.is_nerd_font),
            fixed_width: self.fixed_width,
            fixed_height: self.fixed_height,
            // Fields that aren't overridable via builder (OS/system detection)
            os: detected.os,
            distro: detected.distro,
            config_file: detected.config_file,
            font: detected.font,
            font_size: detected.font_size,
            font_ligatures: detected.font_ligatures,
            remote: detected.remote,
            char_encoding: detected.char_encoding,
            locale: detected.locale,
        }
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
        assert!(
            term.font_ligatures.is_none(),
            "font_ligatures detection is not implemented"
        );
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

    #[test]
    fn test_terminal_builder_overrides_is_tty() {
        let term = Terminal::builder().is_tty(true).build();
        assert!(term.is_tty);

        let term = Terminal::builder().is_tty(false).build();
        assert!(!term.is_tty);
    }

    #[test]
    fn test_terminal_builder_overrides_image_support() {
        let term = Terminal::builder()
            .image_support(ImageSupport::Kitty)
            .build();
        assert_eq!(term.image_support, ImageSupport::Kitty);

        let term = Terminal::builder()
            .image_support(ImageSupport::None)
            .build();
        assert_eq!(term.image_support, ImageSupport::None);
    }

    #[test]
    fn test_terminal_builder_overrides_color_depth() {
        let term = Terminal::builder()
            .color_depth(ColorDepth::TrueColor)
            .build();
        assert_eq!(term.color_depth, ColorDepth::TrueColor);
    }

    #[test]
    fn test_terminal_builder_overrides_multiple_fields() {
        let term = Terminal::builder()
            .is_tty(true)
            .image_support(ImageSupport::Kitty)
            .color_depth(ColorDepth::TrueColor)
            .is_ci(false)
            .build();

        assert!(term.is_tty);
        assert_eq!(term.image_support, ImageSupport::Kitty);
        assert_eq!(term.color_depth, ColorDepth::TrueColor);
        assert!(!term.is_ci);
    }

    #[test]
    fn test_terminal_builder_preserves_os_detection() {
        let term = Terminal::builder().is_tty(true).build();
        // OS detection should still work
        #[cfg(target_os = "macos")]
        assert_eq!(term.os, OsType::MacOS);
        #[cfg(target_os = "linux")]
        assert_eq!(term.os, OsType::Linux);
    }
}
