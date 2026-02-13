use std::path::{Path, PathBuf};

use sniff::filesystem::git::detect_git;
use sniff::filesystem::repo::Package;
use sniff::filesystem::repo::detect_repo;

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
    let git_info = detect_git(Path::new("."), false, 0).ok().flatten();
    let repo_root = git_info.as_ref().map(|info| info.repo_root.clone());
    let in_repo = repo_root.is_some();
    let repo_info = repo_root
        .as_ref()
        .and_then(|root| detect_repo(root).ok().flatten());
    let in_monorepo = repo_info
        .as_ref()
        .map(|info| info.is_monorepo)
        .unwrap_or(false);
    let cwd = std::env::current_dir().ok();
    let package_root = match (cwd.as_ref(), repo_root.as_ref()) {
        (Some(cwd), Some(repo_root)) => compute_package_root(
            cwd,
            repo_root,
            in_monorepo,
            repo_info
                .as_ref()
                .and_then(|info| info.packages.as_ref())
                .map(|packages| packages.as_slice()),
        ),
        _ => None,
    };

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
        in_repo,
        in_monorepo,
        repo_root,
        package_root,
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
#[derive(Debug, Clone)]
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
    /// Whether the current directory is inside a git repository
    pub in_repo: bool,
    /// Whether the current repository is a monorepo
    pub in_monorepo: bool,
    /// Root path of the git repository (if detected)
    pub repo_root: Option<PathBuf>,
    /// Root path of the package containing the current working directory (monorepos only)
    pub package_root: Option<String>,
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
            underline_support: value.underline_support,
            osc_link_support: value.osc_link_support,
            is_tty: value.is_tty,
            color_depth: value.color_depth.clone(),
            os: value.os,
            distro: value.distro.clone(),
            config_file: value.config_file.clone(),
            is_ci: value.is_ci,
            in_repo: value.in_repo,
            in_monorepo: value.in_monorepo,
            repo_root: value.repo_root.clone(),
            package_root: value.package_root.clone(),
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

    /// Creates an optimistic Terminal with fixed width and full capabilities enabled.
    ///
    /// This constructor creates a Terminal that assumes all modern terminal
    /// capabilities are available, without performing actual detection. This is
    /// useful for:
    ///
    /// - `render()` methods that need a terminal for rendering calculations
    /// - Testing with predictable terminal capabilities
    /// - Generating output intended for modern terminals
    ///
    /// The returned Terminal has:
    /// - Fixed width set to the provided value
    /// - Kitty image support enabled
    /// - TrueColor depth enabled
    /// - Italics, underlines, and OSC8 links enabled
    /// - TTY mode enabled
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::terminal::Terminal;
    /// use biscuit_terminal::discovery::detection::{ImageSupport, ColorDepth};
    ///
    /// let term = Terminal::new_optimistic(80);
    /// assert_eq!(term.width(), 80);
    /// assert_eq!(term.image_support, ImageSupport::Kitty);
    /// assert_eq!(term.color_depth, ColorDepth::TrueColor);
    /// assert!(term.supports_italic);
    /// assert!(term.osc_link_support);
    /// ```
    pub fn new_optimistic(width: u32) -> Terminal {
        Terminal {
            app: TerminalApp::Other("Optimistic".to_string()),
            supports_italic: true,
            image_support: ImageSupport::Kitty,
            underline_support: UnderlineSupport {
                straight: true,
                double: true,
                curly: true,
                dotted: true,
                dashed: true,
                colored: true,
            },
            osc_link_support: true,
            is_tty: true,
            color_depth: ColorDepth::TrueColor,
            os: OsType::Unknown,
            distro: None,
            config_file: None,
            is_ci: false,
            font: None,
            font_size: None,
            font_ligatures: None,
            is_nerd_font: None,
            in_repo: false,
            in_monorepo: false,
            repo_root: None,
            package_root: None,
            remote: Connection::Local,
            char_encoding: CharEncoding::default(),
            locale: TerminalLocale::default(),
            fixed_width: Some(width),
            fixed_height: None,
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
        use crate::utils::layout::Layout;

        let term = Terminal::new();
        let layout = Layout::default();
        let output = layout.apply_layout(&content.into(), term.width());
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
            in_repo: detected.in_repo,
            in_monorepo: detected.in_monorepo,
            repo_root: detected.repo_root,
            package_root: detected.package_root,
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
    use tempfile::TempDir;

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
        let _in_repo = term.in_repo;
        let _in_monorepo = term.in_monorepo;
        let _repo_root = &term.repo_root;
        let _package_root = &term.package_root;
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

    #[test]
    fn test_terminal_repo_fields_are_consistent() {
        let term = Terminal::new();

        if term.in_repo {
            assert!(term.repo_root.is_some());
        } else {
            assert!(term.repo_root.is_none());
            assert!(!term.in_monorepo);
            assert!(term.package_root.is_none());
        }
    }

    #[test]
    fn test_compute_package_root_none_for_non_monorepo() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let cwd = root.join("project");
        std::fs::create_dir_all(&cwd).unwrap();

        let result = compute_package_root(&cwd, root, false, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_compute_package_root_repo_root_when_at_root() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let result = compute_package_root(root, root, true, None);
        assert_eq!(result, Some(root.to_string_lossy().to_string()));
    }

    #[test]
    fn test_terminal_new_optimistic_has_correct_width() {
        let term = Terminal::new_optimistic(80);
        assert_eq!(term.width(), 80);
        assert_eq!(term.fixed_width, Some(80));

        let term = Terminal::new_optimistic(120);
        assert_eq!(term.width(), 120);
    }

    #[test]
    fn test_terminal_new_optimistic_has_full_capabilities() {
        let term = Terminal::new_optimistic(80);
        assert_eq!(term.image_support, ImageSupport::Kitty);
        assert_eq!(term.color_depth, ColorDepth::TrueColor);
        // All underline styles enabled
        assert!(term.underline_support.straight);
        assert!(term.underline_support.double);
        assert!(term.underline_support.curly);
        assert!(term.underline_support.dotted);
        assert!(term.underline_support.dashed);
        assert!(term.underline_support.colored);
        assert!(term.supports_italic);
        assert!(term.osc_link_support);
        assert!(term.is_tty);
        assert!(!term.is_ci);
    }

    #[test]
    fn test_terminal_new_optimistic_no_detection() {
        // new_optimistic should NOT run detection (fast, predictable)
        let term = Terminal::new_optimistic(80);
        assert_eq!(term.os, OsType::Unknown);
        assert!(term.distro.is_none());
        assert!(term.config_file.is_none());
        assert!(!term.in_repo);
        assert!(!term.in_monorepo);
        assert!(term.repo_root.is_none());
        assert!(term.package_root.is_none());
    }

    #[test]
    fn test_compute_package_root_matches_package() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let package_path = root.join("packages").join("app");
        std::fs::create_dir_all(&package_path).unwrap();

        let packages = vec![Package {
            name: "app".to_string(),
            relative: "packages/app".to_string(),
            package_area: "packages".to_string(),
            path: package_path.clone(),
            primary_language: None,
            languages: Vec::new(),
            configuration: Vec::new(),
            documentation: Vec::new(),
            editor_config: None,
            command_runner: Vec::new(),
            package_managers: Vec::new(),
            version: None,
            features: Vec::new(),
            depends_on: Vec::new(),
            used_by: Vec::new(),
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            is_updatable: None,
        }];

        let cwd = package_path.join("src");
        std::fs::create_dir_all(&cwd).unwrap();

        let result = compute_package_root(&cwd, root, true, Some(&packages));
        assert_eq!(result, Some(package_path.to_string_lossy().to_string()));
    }
}

fn compute_package_root(
    cwd: &Path,
    repo_root: &Path,
    in_monorepo: bool,
    packages: Option<&[Package]>,
) -> Option<String> {
    if !in_monorepo {
        return None;
    }

    if cwd == repo_root {
        return Some(repo_root.to_string_lossy().to_string());
    }

    packages.and_then(|packages| {
        packages
            .iter()
            .find(|package| cwd.starts_with(&package.path))
            .map(|package| package.path.to_string_lossy().to_string())
    })
}
