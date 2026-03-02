//! Mermaid diagram rendering for terminals.
//!
//! This module provides functionality to render Mermaid diagrams in the terminal
//! by executing the `mmdc` CLI tool locally and displaying the output with viuer.
//! Falls back to code block rendering when image rendering is not supported.
//!
//! ## CLI Detection
//!
//! The module uses a fallback chain for finding the Mermaid CLI:
//!
//! 1. **Direct `mmdc`**: If `mmdc` is in PATH, use it directly
//! 2. **npx fallback**: If `mmdc` is not found but `npx` is available, use `npx mmdc`
//!    with a warning to stderr explaining the temporary installation
//! 3. **Error**: If neither is available, return an error asking the user to install npm
//!
//! ## Image Support
//!
//! Image display uses the `viuer` crate for terminal image rendering.
//! When the terminal does not support images, only the fallback code block is provided.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use biscuit_terminal::components::mermaid::MermaidRenderer;
//!
//! fn example() -> Result<(), biscuit_terminal::components::mermaid::MermaidRenderError> {
//!     let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
//!     renderer.render_for_terminal()?;
//!     Ok(())
//! }
//! ```

use std::io::Write;
use std::path::Path;
use std::process::Command;

use thiserror::Error;

/// Maximum input size for mmdc (10KB should be plenty for diagrams).
///
/// This limit prevents accidentally passing excessively large content to mmdc.
/// Most Mermaid diagrams are well under this size.
const MAX_DIAGRAM_SIZE: usize = 10_000;

/// Icon packs to enable for Mermaid diagrams.
///
/// These icon packs are passed to mmdc via `--iconPacks`:
/// - `@iconify-json/fa7-brands` - Font Awesome 7 brand icons
/// - `@iconify-json/lucide` - Lucide icons
/// - `@iconify-json/carbon` - Carbon Design icons
/// - `@iconify-json/system-uicons` - System UI icons
const ICON_PACKS: &[&str] = &[
    "@iconify-json/fa7-brands",
    "@iconify-json/lucide",
    "@iconify-json/carbon",
    "@iconify-json/system-uicons",
];

/// Default scale factor for rendering (2x for better resolution on modern displays).
const DEFAULT_SCALE: u32 = 2;

/// Minimum recommended mmdc version for optimal feature support.
///
/// Version 10.6.0 introduced significant improvements including:
/// - Better SVG rendering
/// - Improved icon pack support
/// - More stable async diagram generation
///
/// Older versions will work but may have rendering quirks.
pub const MMDC_MIN_VERSION: &str = "10.6.0";

/// Parsed version for comparison operations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MmdcVersion {
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
}

impl MmdcVersion {
    /// Parse a version string in the format "X.Y.Z".
    ///
    /// Returns `None` if the version string cannot be parsed.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::mermaid::MmdcVersion;
    ///
    /// let v = MmdcVersion::parse("10.6.0").unwrap();
    /// assert_eq!(v.major, 10);
    /// assert_eq!(v.minor, 6);
    /// assert_eq!(v.patch, 0);
    ///
    /// assert!(MmdcVersion::parse("invalid").is_none());
    /// ```
    pub fn parse(version: &str) -> Option<Self> {
        let parts: Vec<&str> = version.trim().split('.').collect();
        if parts.len() < 3 {
            return None;
        }
        Some(Self {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }

    /// Returns the minimum recommended version.
    pub fn minimum() -> Self {
        Self::parse(MMDC_MIN_VERSION).expect("MMDC_MIN_VERSION is valid")
    }

    /// Returns true if this version meets the minimum recommended version.
    pub fn meets_minimum(&self) -> bool {
        self >= &Self::minimum()
    }
}

impl std::fmt::Display for MmdcVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Detects the installed mmdc version.
///
/// Runs `mmdc --version` and parses the output. Returns `None` if mmdc
/// is not installed or the version cannot be determined.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_terminal::components::mermaid::{detect_mmdc_version, MmdcVersion};
///
/// if let Some(version) = detect_mmdc_version() {
///     println!("mmdc version: {}", version);
///     if !version.meets_minimum() {
///         eprintln!("Warning: mmdc {} is older than recommended {}", version, MmdcVersion::minimum());
///     }
/// }
/// ```
pub fn detect_mmdc_version() -> Option<MmdcVersion> {
    // Try direct mmdc first, then npx
    let output = Command::new("mmdc")
        .arg("--version")
        .output()
        .or_else(|_| {
            Command::new("npx")
                .args(["--yes", "mmdc", "--version"])
                .output()
        })
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    // mmdc outputs something like "10.9.1" or "@mermaid-js/mermaid-cli 10.9.1"
    // Extract just the version number
    let version_part = version_str.split_whitespace().find(|s| {
        s.chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
    })?;

    MmdcVersion::parse(version_part)
}

/// Mermaid theme options.
///
/// These correspond to the built-in themes available in mermaid-cli.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MermaidTheme {
    /// Dark theme - light text on dark background (default for dark terminals)
    #[default]
    Dark,
    /// Default/light theme - dark text on light background
    Default,
    /// Forest theme - green tones
    Forest,
    /// Neutral theme - grayscale, works well with transparent backgrounds
    Neutral,
}

impl MermaidTheme {
    /// Returns the theme string for mmdc CLI.
    pub fn as_str(&self) -> &'static str {
        match self {
            MermaidTheme::Dark => "dark",
            MermaidTheme::Default => "default",
            MermaidTheme::Forest => "forest",
            MermaidTheme::Neutral => "neutral",
        }
    }

    /// Returns the appropriate theme for a given color mode.
    ///
    /// - Dark terminals get `Dark` theme
    /// - Light terminals get `Default` theme
    /// - Unknown defaults to `Dark`
    pub fn for_color_mode(mode: crate::discovery::detection::ColorMode) -> Self {
        use crate::discovery::detection::ColorMode;
        match mode {
            ColorMode::Light => MermaidTheme::Default,
            ColorMode::Dark | ColorMode::Unknown => MermaidTheme::Dark,
        }
    }

    /// Returns the inverse theme (for solid background rendering).
    ///
    /// - Dark → Default (light)
    /// - Default → Dark
    /// - Forest → Dark
    /// - Neutral → Dark
    pub fn inverse(self) -> Self {
        match self {
            MermaidTheme::Dark => MermaidTheme::Default,
            MermaidTheme::Default => MermaidTheme::Dark,
            MermaidTheme::Forest => MermaidTheme::Dark,
            MermaidTheme::Neutral => MermaidTheme::Dark,
        }
    }
}

/// Errors that can occur during terminal rendering of Mermaid diagrams.
#[derive(Error, Debug)]
pub enum MermaidRenderError {
    /// mmdc CLI not found in PATH (and npx fallback not used).
    #[error("mmdc CLI not found. Install with: npm install -g @mermaid-js/mermaid-cli")]
    MmdcNotFound,

    /// npm/npx not found - cannot render mermaid diagrams.
    #[error("npm not found. Install Node.js and npm to render Mermaid diagrams in the terminal")]
    NpmNotFound,

    /// mmdc execution failed.
    #[error("mmdc execution failed (exit code {exit_code}): {stderr}")]
    MmdcExecutionFailed {
        /// The exit code from mmdc
        exit_code: i32,
        /// The stderr output from mmdc
        stderr: String,
    },

    /// Diagram content is too large.
    #[error("Diagram too large ({size} bytes, max {max})")]
    ContentTooLarge {
        /// The actual size of the diagram
        size: usize,
        /// The maximum allowed size
        max: usize,
    },

    /// Failed to display image in terminal.
    #[error("Failed to display image: {0}")]
    DisplayError(String),

    /// IO error during file operations.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Terminal does not support image rendering.
    #[error("Terminal does not support image rendering (use fallback_code_block instead)")]
    NoImageSupport,

    /// Path contains invalid UTF-8 characters.
    #[error("Invalid path encoding: {path}")]
    InvalidPath {
        /// The path that contained invalid UTF-8
        path: String,
    },
}

/// Checks if a command exists in the system PATH.
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detects system Chromium/Chrome browser for Puppeteer fallback.
/// Returns the path to the executable if found, or None otherwise.
fn detect_system_chromium() -> Option<String> {
    let browsers = [
        "chromium",
        "chromium-browser",
        "google-chrome",
        "chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome",
        "/snap/bin/chromium",
        "/opt/chromium/chrome",
    ];

    for browser in &browsers {
        if Command::new("which")
            .arg(browser)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            if let Ok(path) = which::which(browser) {
                tracing::info!("Found system Chromium: {}", path.display());
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }
    tracing::debug!("No system Chromium found, Puppeteer will use bundled Chrome");
    None
}

/// Preset themes for quadrant charts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum QuadrantTheme {
    /// Default Mermaid colors (no customization)
    #[default]
    Default,
    /// Magic Quadrangle style (Gartner-inspired): subtle green for top-right (leaders),
    /// subtle red for bottom-left (niche players). Top-left and bottom-right use a
    /// neutral color (darker in dark mode, lighter in light mode).
    /// Colors adapt to terminal color mode.
    #[cfg_attr(feature = "clap", clap(name = "magic-quadrangle"))]
    MagicQuadrangle,
}

impl QuadrantTheme {
    /// Applies this theme's colors to a MermaidConfig.
    ///
    /// Colors are adapted based on the terminal's color mode:
    /// - Dark mode: Uses dark colors with subtle green/red tints
    /// - Light mode: Uses light colors with subtle green/red tints
    ///
    /// The Magic Quadrangle theme uses:
    /// - Top-right (q1): subtle green (leaders)
    /// - Bottom-left (q3): subtle red (niche players)
    /// - Top-left (q2) and bottom-right (q4): same neutral color (darker in dark mode, lighter in light mode)
    pub fn apply(
        self,
        mut config: MermaidConfig,
        color_mode: crate::discovery::detection::ColorMode,
    ) -> MermaidConfig {
        use crate::discovery::detection::ColorMode;

        match self {
            QuadrantTheme::Default => config,
            QuadrantTheme::MagicQuadrangle => {
                match color_mode {
                    ColorMode::Light => {
                        // Light mode: very subtle tints against light background
                        // Green for top-right (quadrant-1, "leaders") - barely visible green tint
                        config.quadrant1_fill = Some("#f6faf6".to_string());
                        // Red for bottom-left (quadrant-3, "niche players") - barely visible red tint
                        config.quadrant3_fill = Some("#faf6f6".to_string());
                        // Top-left (q2) and bottom-right (q4): same light neutral grey
                        let neutral = "#f8f8f8".to_string();
                        config.quadrant2_fill = Some(neutral.clone());
                        config.quadrant4_fill = Some(neutral);
                    }
                    ColorMode::Dark | ColorMode::Unknown => {
                        // Dark mode: dark colors with subtle tints
                        // Green for top-right (quadrant-1, "leaders") - dark with green undertone
                        config.quadrant1_fill = Some("#1e2a1e".to_string());
                        // Red for bottom-left (quadrant-3, "niche players") - dark with red undertone
                        config.quadrant3_fill = Some("#2a1e1e".to_string());
                        // Top-left (q2) and bottom-right (q4): same dark neutral grey
                        let neutral = "#1a1a1a".to_string();
                        config.quadrant2_fill = Some(neutral.clone());
                        config.quadrant4_fill = Some(neutral);
                    }
                }
                config
            }
        }
    }

    /// Returns the theme name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            QuadrantTheme::Default => "default",
            QuadrantTheme::MagicQuadrangle => "magic-quadrangle",
        }
    }

    /// Parses a theme name string into a QuadrantTheme.
    ///
    /// Returns None if the string doesn't match a known theme.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "default" => Some(QuadrantTheme::Default),
            "magic-quadrangle" | "magic_quadrangle" | "magicquadrangle" => {
                Some(QuadrantTheme::MagicQuadrangle)
            }
            _ => None,
        }
    }
}

/// Configuration options for Mermaid diagrams.
///
/// These options are passed to mmdc via a temporary config file.
/// Only non-None values are included in the generated config.
#[derive(Debug, Clone, Default)]
pub struct MermaidConfig {
    /// Quadrant chart: point label font size (default: 12)
    pub point_label_font_size: Option<u32>,
    /// Quadrant chart: default point radius (default: 5)
    pub point_radius: Option<u32>,
    /// Quadrant chart: top-right (quadrant-1) fill color
    pub quadrant1_fill: Option<String>,
    /// Quadrant chart: top-left (quadrant-2) fill color
    pub quadrant2_fill: Option<String>,
    /// Quadrant chart: bottom-left (quadrant-3) fill color
    pub quadrant3_fill: Option<String>,
    /// Quadrant chart: bottom-right (quadrant-4) fill color
    pub quadrant4_fill: Option<String>,
}

impl MermaidConfig {
    /// Creates a new empty configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the point label font size for quadrant charts.
    pub fn with_point_label_font_size(mut self, size: u32) -> Self {
        self.point_label_font_size = Some(size);
        self
    }

    /// Sets the default point radius for quadrant charts.
    pub fn with_point_radius(mut self, radius: u32) -> Self {
        self.point_radius = Some(radius);
        self
    }

    /// Sets the fill color for a specific quadrant.
    ///
    /// Quadrant numbering:
    /// - 1 = top-right
    /// - 2 = top-left
    /// - 3 = bottom-left
    /// - 4 = bottom-right
    pub fn with_quadrant_fill(mut self, quadrant: u8, color: impl Into<String>) -> Self {
        let color = color.into();
        match quadrant {
            1 => self.quadrant1_fill = Some(color),
            2 => self.quadrant2_fill = Some(color),
            3 => self.quadrant3_fill = Some(color),
            4 => self.quadrant4_fill = Some(color),
            _ => {} // Ignore invalid quadrant numbers
        }
        self
    }

    /// Returns true if any configuration options are set.
    pub fn has_options(&self) -> bool {
        self.point_label_font_size.is_some()
            || self.point_radius.is_some()
            || self.quadrant1_fill.is_some()
            || self.quadrant2_fill.is_some()
            || self.quadrant3_fill.is_some()
            || self.quadrant4_fill.is_some()
    }

    /// Generates the JSON configuration for mmdc.
    ///
    /// Returns None if no options are set.
    pub fn to_json(&self) -> Option<String> {
        if !self.has_options() {
            return None;
        }

        let mut quadrant_config = Vec::new();
        let mut theme_vars = Vec::new();

        // Quadrant chart specific options
        if let Some(size) = self.point_label_font_size {
            quadrant_config.push(format!("\"pointLabelFontSize\": {}", size));
        }
        if let Some(radius) = self.point_radius {
            quadrant_config.push(format!("\"pointRadius\": {}", radius));
        }

        // Theme variables for quadrant fills
        if let Some(ref color) = self.quadrant1_fill {
            theme_vars.push(format!("\"quadrant1Fill\": \"{}\"", color));
        }
        if let Some(ref color) = self.quadrant2_fill {
            theme_vars.push(format!("\"quadrant2Fill\": \"{}\"", color));
        }
        if let Some(ref color) = self.quadrant3_fill {
            theme_vars.push(format!("\"quadrant3Fill\": \"{}\"", color));
        }
        if let Some(ref color) = self.quadrant4_fill {
            theme_vars.push(format!("\"quadrant4Fill\": \"{}\"", color));
        }

        // Build the JSON structure
        let mut sections = Vec::new();

        if !quadrant_config.is_empty() {
            sections.push(format!(
                "\"quadrantChart\": {{\n    {}\n  }}",
                quadrant_config.join(",\n    ")
            ));
        }

        if !theme_vars.is_empty() {
            sections.push(format!(
                "\"themeVariables\": {{\n    {}\n  }}",
                theme_vars.join(",\n    ")
            ));
        }

        if sections.is_empty() {
            return None;
        }

        Some(format!("{{\n  {}\n}}", sections.join(",\n  ")))
    }
}

/// A Mermaid diagram renderer for terminal output.
///
/// This struct handles rendering Mermaid diagrams to the terminal using the
/// `mmdc` CLI tool. It supports automatic CLI detection with npx fallback,
/// icon packs, and graceful fallback to code blocks when image rendering
/// is not available.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_terminal::components::mermaid::MermaidRenderer;
///
/// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
///
/// // Try to render as an image, or get the fallback code block
/// match renderer.render_for_terminal() {
///     Ok(()) => println!("Diagram rendered successfully!"),
///     Err(e) => {
///         eprintln!("Render failed: {}", e);
///         println!("{}", renderer.fallback_code_block());
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct MermaidRenderer {
    /// The Mermaid diagram instructions
    instructions: String,
    /// Optional title for alt text
    title: Option<String>,
    /// Theme for rendering
    theme: MermaidTheme,
    /// Scale factor for output resolution (default: 2)
    scale: u32,
    /// Use transparent background
    transparent_background: bool,
    /// Additional configuration options
    config: MermaidConfig,
}

impl MermaidRenderer {
    /// Creates a new MermaidRenderer with the given diagram instructions.
    ///
    /// Uses default settings: dark theme, 2x scale, opaque background.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    /// ```
    pub fn new<S: Into<String>>(instructions: S) -> Self {
        Self {
            instructions: instructions.into(),
            title: None,
            theme: MermaidTheme::default(),
            scale: DEFAULT_SCALE,
            transparent_background: false,
            config: MermaidConfig::default(),
        }
    }

    /// Creates a MermaidRenderer configured for the current terminal.
    ///
    /// Automatically detects color mode and sets appropriate theme.
    /// Uses transparent background for better terminal integration.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::for_terminal("flowchart LR\n    A --> B");
    /// // Theme and background are automatically configured
    /// ```
    pub fn for_terminal<S: Into<String>>(instructions: S) -> Self {
        use crate::terminal::Terminal;

        let color_mode = Terminal::color_mode();
        Self {
            instructions: instructions.into(),
            title: None,
            theme: MermaidTheme::for_color_mode(color_mode),
            scale: DEFAULT_SCALE,
            transparent_background: true,
            config: MermaidConfig::default(),
        }
    }

    /// Sets the theme for rendering.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::{MermaidRenderer, MermaidTheme};
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_theme(MermaidTheme::Neutral);
    /// ```
    pub fn with_theme(mut self, theme: MermaidTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Sets the scale factor for output resolution.
    ///
    /// Higher values produce sharper images but larger files.
    /// Default is 2 (good for most modern displays).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_scale(3); // Extra sharp
    /// ```
    pub fn with_scale(mut self, scale: u32) -> Self {
        self.scale = scale.max(1); // Minimum scale of 1
        self
    }

    /// Enables transparent background for better terminal integration.
    ///
    /// When enabled, the diagram background will be transparent,
    /// allowing it to blend with the terminal's background color.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_transparent_background(true);
    /// ```
    pub fn with_transparent_background(mut self, transparent: bool) -> Self {
        self.transparent_background = transparent;
        self
    }

    /// Sets additional Mermaid configuration options.
    ///
    /// These options are passed to mmdc via a temporary config file.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::{MermaidRenderer, MermaidConfig};
    ///
    /// let config = MermaidConfig::new()
    ///     .with_point_label_font_size(16)
    ///     .with_point_radius(10);
    ///
    /// let renderer = MermaidRenderer::new("quadrantChart\n    A: [0.5, 0.5]")
    ///     .with_config(config);
    /// ```
    pub fn with_config(mut self, config: MermaidConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets a title for the diagram (used for alt text).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_title("My Flowchart");
    /// ```
    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Returns the diagram instructions.
    pub fn instructions(&self) -> &str {
        &self.instructions
    }

    /// Returns the title if set.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns alt text for the diagram.
    ///
    /// Uses the explicit title if set, otherwise detects the diagram type
    /// from the first line of instructions.
    pub fn alt_text(&self) -> String {
        if let Some(ref title) = self.title {
            title.clone()
        } else {
            detect_diagram_type(&self.instructions)
        }
    }

    /// Returns a fallback code block string for the diagram.
    ///
    /// This is used when terminal rendering fails or is not supported.
    /// Returns the instructions formatted as a fenced code block.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    /// let fallback = renderer.fallback_code_block();
    /// assert!(fallback.contains("```mermaid"));
    /// ```
    pub fn fallback_code_block(&self) -> String {
        format!("```mermaid\n{}\n```", self.instructions)
    }

    /// Prints the fallback code block to stdout.
    ///
    /// This is a convenience method for when terminal rendering fails.
    pub fn print_fallback(&self) {
        println!("{}", self.fallback_code_block());
    }

    /// Checks if the current terminal supports image rendering.
    ///
    /// Returns `true` if either Kitty or iTerm2 image protocols are supported.
    pub fn terminal_supports_images() -> bool {
        use crate::discovery::detection::ImageSupport;
        use crate::terminal::Terminal;

        let term = Terminal::new();
        !matches!(term.image_support, ImageSupport::None)
    }

    /// Renders the diagram to the terminal using the local mmdc CLI.
    ///
    /// This method:
    /// 1. Validates diagram size (< 10KB)
    /// 2. Checks if the terminal supports image rendering
    /// 3. **Checks cache for existing render** (cache hit avoids mmdc invocation)
    /// 4. If cache miss, renders via mmdc and stores in cache
    /// 5. Displays the output PNG with viuer
    ///
    /// ## Caching
    ///
    /// Rendered diagrams are cached based on all render parameters:
    /// - Diagram source, theme, scale, config, transparency, title, mmdc version
    /// - Cache is stored in OS temp directory and managed by the OS
    /// - On cache hit, mmdc is not invoked (significant performance improvement)
    ///
    /// ## Icon Pack Support
    ///
    /// This method enables the following icon packs via `--iconPacks`:
    /// - `@iconify-json/fa7-brands` - Font Awesome 7 brand icons
    /// - `@iconify-json/lucide` - Lucide icons
    /// - `@iconify-json/carbon` - Carbon Design icons
    /// - `@iconify-json/system-uicons` - System UI icons
    ///
    /// Icons can be used in diagrams like: `A[icon:fa7-brands:github]`
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// fn example() -> Result<(), biscuit_terminal::components::mermaid::MermaidRenderError> {
    ///     let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    ///     renderer.render_for_terminal()?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns `MermaidRenderError` if:
    /// - Terminal doesn't support image rendering
    /// - mmdc is not installed or not in PATH
    /// - Diagram is too large (> 10KB)
    /// - mmdc execution fails (invalid syntax, etc.)
    #[tracing::instrument(skip(self))]
    pub fn render_for_terminal(&self) -> Result<(), MermaidRenderError> {
        use super::mermaid_cache::{MermaidCache, MermaidCacheKey};

        // 1. Validate size
        if self.instructions.len() > MAX_DIAGRAM_SIZE {
            tracing::error!(
                size = self.instructions.len(),
                max = MAX_DIAGRAM_SIZE,
                "Diagram too large for mmdc"
            );
            return Err(MermaidRenderError::ContentTooLarge {
                size: self.instructions.len(),
                max: MAX_DIAGRAM_SIZE,
            });
        }

        // 2. Check terminal support
        if !Self::terminal_supports_images() {
            tracing::debug!("Terminal does not support image rendering");
            return Err(MermaidRenderError::NoImageSupport);
        }

        // 3. Get mmdc version (with warning for old versions)
        let mmdc_version = self.get_mmdc_version_with_warning();

        // 4. Check cache
        let cache = MermaidCache::new();
        let cache_key = MermaidCacheKey::new(
            &self.instructions,
            self.theme,
            self.scale,
            &self.config,
            self.transparent_background,
            self.title.as_deref(),
            &mmdc_version,
        );

        let output_path = if let Some(cached_path) = cache.get(&cache_key) {
            // Cache hit - use cached render
            tracing::info!(path = ?cached_path, "Using cached mermaid render");
            cached_path
        } else {
            // Cache miss - render and store
            let rendered_path = self.render_to_temp_png()?;

            // Store in cache (ignore errors - cache is optional)
            match cache.store(&cache_key, &rendered_path) {
                Ok(cached_path) => {
                    // Clean up the temp file, use cached version
                    let _ = std::fs::remove_file(&rendered_path);
                    cached_path
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to cache mermaid render");
                    rendered_path
                }
            }
        };

        // 5. Display with viuer
        let config = viuer::Config {
            absolute_offset: false,
            ..Default::default()
        };

        tracing::info!(path = ?output_path, "Displaying diagram in terminal");

        viuer::print_from_file(&output_path, &config)
            .map_err(|e| MermaidRenderError::DisplayError(e.to_string()))?;

        tracing::debug!("Displayed diagram in terminal");

        Ok(())
    }

    /// Renders the diagram to a cached PNG file, returning the path and cache hit status.
    ///
    /// This method checks the cache first and only renders if needed:
    /// - On cache hit: Returns the cached path and `true`
    /// - On cache miss: Renders via mmdc, stores in cache, returns the path and `false`
    ///
    /// The returned path is in the cache directory and should NOT be deleted by the caller.
    ///
    /// ## Errors
    ///
    /// Returns error if:
    /// - Terminal doesn't support image rendering
    /// - mmdc is not available or execution fails
    /// - Diagram is too large
    #[tracing::instrument(skip(self))]
    pub fn render_to_cached_png(&self) -> Result<(std::path::PathBuf, bool), MermaidRenderError> {
        use super::mermaid_cache::{MermaidCache, MermaidCacheKey};

        // 1. Validate size
        if self.instructions.len() > MAX_DIAGRAM_SIZE {
            tracing::error!(
                size = self.instructions.len(),
                max = MAX_DIAGRAM_SIZE,
                "Diagram too large for mmdc"
            );
            return Err(MermaidRenderError::ContentTooLarge {
                size: self.instructions.len(),
                max: MAX_DIAGRAM_SIZE,
            });
        }

        // 2. Get mmdc version
        let mmdc_version = self.get_mmdc_version_with_warning();

        // 3. Check cache
        let cache = MermaidCache::new();
        let cache_key = MermaidCacheKey::new(
            &self.instructions,
            self.theme,
            self.scale,
            &self.config,
            self.transparent_background,
            self.title.as_deref(),
            &mmdc_version,
        );

        if let Some(cached_path) = cache.get(&cache_key) {
            // Cache hit
            tracing::info!(path = ?cached_path, "Using cached mermaid render");
            return Ok((cached_path, true));
        }

        // Cache miss - render and store
        let rendered_path = self.render_to_temp_png()?;

        // Store in cache
        match cache.store(&cache_key, &rendered_path) {
            Ok(cached_path) => {
                // Clean up the temp file, use cached version
                let _ = std::fs::remove_file(&rendered_path);
                Ok((cached_path, false))
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to cache mermaid render");
                // Return the temp file path if caching failed
                Ok((rendered_path, false))
            }
        }
    }

    /// Gets the mmdc version, caching the result and warning if it's below minimum.
    fn get_mmdc_version_with_warning(&self) -> String {
        use std::sync::OnceLock;

        static MMDC_VERSION: OnceLock<String> = OnceLock::new();
        static VERSION_WARNING_SHOWN: OnceLock<bool> = OnceLock::new();

        let version = MMDC_VERSION.get_or_init(|| {
            detect_mmdc_version()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

        // Show warning once if version is known and below minimum
        if version != "unknown" {
            VERSION_WARNING_SHOWN.get_or_init(|| {
                if let Some(parsed) = MmdcVersion::parse(version) && !parsed.meets_minimum() {
                    eprintln!(
                        "Warning: mmdc {} is older than recommended {}. Consider updating: npm update -g @mermaid-js/mermaid-cli",
                        parsed, MmdcVersion::minimum()
                    );
                    return true;
                }
                false
            });
        }

        version.clone()
    }

    /// Renders the diagram to a temporary PNG file.
    ///
    /// Returns the path to the generated PNG file. The caller is responsible
    /// for cleaning up the file after use.
    ///
    /// ## Errors
    ///
    /// Returns error if mmdc is not available or execution fails.
    #[tracing::instrument(skip(self))]
    pub fn render_to_temp_png(&self) -> Result<std::path::PathBuf, MermaidRenderError> {
        use std::io::Write as _;
        use tempfile::Builder;

        // Validate size
        if self.instructions.len() > MAX_DIAGRAM_SIZE {
            return Err(MermaidRenderError::ContentTooLarge {
                size: self.instructions.len(),
                max: MAX_DIAGRAM_SIZE,
            });
        }

        // Create temp files with tempfile crate (RAII cleanup for input)
        let input_file = Builder::new().suffix(".mmd").tempfile()?;

        tracing::debug!(path = ?input_file.path(), "Created temporary input file");

        // Write instructions to input file
        std::fs::write(input_file.path(), &self.instructions)?;

        // Create config file if we have configuration options
        let config_file = if let Some(config_json) = self.config.to_json() {
            let cf = Builder::new().suffix(".json").tempfile()?;
            std::fs::write(cf.path(), &config_json)?;
            tracing::debug!(path = ?cf.path(), "Created temporary config file");
            Some(cf)
        } else {
            None
        };

        // Output path (alongside input, will be returned to caller for cleanup)
        let output_path = input_file.path().with_extension("png");

        tracing::debug!(
            input = ?input_file.path(),
            output = ?output_path,
            "Prepared file paths for mmdc"
        );

        // Determine how to run mmdc (direct or via npx)
        let use_npx = if command_exists("mmdc") {
            tracing::debug!("Found mmdc in PATH, using directly");
            false
        } else if command_exists("npx") {
            tracing::info!("mmdc not found, falling back to npx");
            // Print warning to stderr about temporary installation
            let _ = writeln!(
                std::io::stderr(),
                "- Mermaid diagrams require mmdc to render to the terminal\n\
                 - You do not have the Mermaid CLI installed, using npx to install temporarily\n\
                 - To install permanently: npm install -g @mermaid-js/mermaid-cli"
            );
            true
        } else {
            tracing::error!("Neither mmdc nor npx found in PATH");
            return Err(MermaidRenderError::NpmNotFound);
        };

        // Build and execute mmdc command
        tracing::info!(use_npx, "Executing mmdc CLI");
        let mut cmd = if use_npx {
            let mut c = Command::new("npx");
            c.args(["-p", "@mermaid-js/mermaid-cli", "mmdc"]);
            c
        } else {
            Command::new("mmdc")
        };

        // Set PUPPETEER_EXECUTABLE_PATH if system Chromium is available
        // This fixes Puppeteer issues on Linux where downloaded Chrome binary fails
        if let Some(chromium_path) = detect_system_chromium() {
            let _ = writeln!(
                std::io::stderr(),
                "- Using system Chromium at: {}",
                chromium_path
            );
            cmd.env("PUPPETEER_EXECUTABLE_PATH", &chromium_path);
            cmd.env("PUPPETEER_SKIP_CHROMIUM_DOWNLOAD", "true");
        }

        // Add common arguments
        let input_path_str =
            input_file
                .path()
                .to_str()
                .ok_or_else(|| MermaidRenderError::InvalidPath {
                    path: input_file.path().to_string_lossy().into_owned(),
                })?;
        let output_path_str =
            output_path
                .to_str()
                .ok_or_else(|| MermaidRenderError::InvalidPath {
                    path: output_path.to_string_lossy().into_owned(),
                })?;
        cmd.args(["-i", input_path_str])
            .args(["-o", output_path_str])
            .args(["--theme", self.theme.as_str()])
            .args(["--scale", &self.scale.to_string()]);

        // Add transparent background if requested
        if self.transparent_background {
            cmd.args(["--backgroundColor", "transparent"]);
        }

        // Add config file if we have one
        if let Some(ref cf) = config_file {
            let config_path_str =
                cf.path()
                    .to_str()
                    .ok_or_else(|| MermaidRenderError::InvalidPath {
                        path: cf.path().to_string_lossy().into_owned(),
                    })?;
            cmd.args(["--configFile", config_path_str]);
        }

        // Add icon packs
        cmd.arg("--iconPacks").args(ICON_PACKS);

        let output = cmd.output();

        // Handle errors
        let output = match output {
            Ok(o) => o,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::error!("Command not found despite prior check");
                return Err(if use_npx {
                    MermaidRenderError::NpmNotFound
                } else {
                    MermaidRenderError::MmdcNotFound
                });
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to execute mmdc");
                return Err(MermaidRenderError::IoError(e));
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);

            tracing::error!(
                exit_code,
                stderr = %stderr,
                "mmdc execution failed"
            );

            // Clean up output file if it exists
            let _ = std::fs::remove_file(&output_path);

            return Err(MermaidRenderError::MmdcExecutionFailed { exit_code, stderr });
        }

        tracing::debug!(
            exit_code = output.status.code().unwrap_or(0),
            "mmdc execution succeeded"
        );

        Ok(output_path)
    }

    /// Renders the diagram to a PNG file at the specified path.
    ///
    /// ## Arguments
    ///
    /// * `output_path` - The path where the PNG file should be written
    ///
    /// ## Errors
    ///
    /// Returns error if mmdc is not available or execution fails.
    #[tracing::instrument(skip(self))]
    pub fn render_to_file(&self, output_path: &Path) -> Result<(), MermaidRenderError> {
        use tempfile::Builder;

        // Validate size
        if self.instructions.len() > MAX_DIAGRAM_SIZE {
            return Err(MermaidRenderError::ContentTooLarge {
                size: self.instructions.len(),
                max: MAX_DIAGRAM_SIZE,
            });
        }

        // Create temp input file
        let input_file = Builder::new().suffix(".mmd").tempfile()?;
        std::fs::write(input_file.path(), &self.instructions)?;

        // Create config file if we have configuration options
        let config_file = if let Some(config_json) = self.config.to_json() {
            let cf = Builder::new().suffix(".json").tempfile()?;
            std::fs::write(cf.path(), &config_json)?;
            Some(cf)
        } else {
            None
        };

        // Determine how to run mmdc
        let use_npx = if command_exists("mmdc") {
            false
        } else if command_exists("npx") {
            let _ = writeln!(
                std::io::stderr(),
                "- Using npx to run mmdc temporarily\n\
                 - To install permanently: npm install -g @mermaid-js/mermaid-cli"
            );
            true
        } else {
            return Err(MermaidRenderError::NpmNotFound);
        };

        // Build command
        let mut cmd = if use_npx {
            let mut c = Command::new("npx");
            c.args(["-p", "@mermaid-js/mermaid-cli", "mmdc"]);
            c
        } else {
            Command::new("mmdc")
        };

        // Set PUPPETEER_EXECUTABLE_PATH if system Chromium is available
        if let Some(chromium_path) = detect_system_chromium() {
            tracing::debug!("Setting PUPPETEER_EXECUTABLE_PATH to {}", chromium_path);
            cmd.env("PUPPETEER_EXECUTABLE_PATH", &chromium_path);
            cmd.env("PUPPETEER_SKIP_CHROMIUM_DOWNLOAD", "true");
        }

        // Add common arguments
        let input_path_str =
            input_file
                .path()
                .to_str()
                .ok_or_else(|| MermaidRenderError::InvalidPath {
                    path: input_file.path().to_string_lossy().into_owned(),
                })?;
        let output_path_str =
            output_path
                .to_str()
                .ok_or_else(|| MermaidRenderError::InvalidPath {
                    path: output_path.to_string_lossy().into_owned(),
                })?;
        cmd.args(["-i", input_path_str])
            .args(["-o", output_path_str])
            .args(["--theme", self.theme.as_str()])
            .args(["--scale", &self.scale.to_string()]);

        // Add transparent background if requested
        if self.transparent_background {
            cmd.args(["--backgroundColor", "transparent"]);
        }

        // Add config file if we have one
        if let Some(ref cf) = config_file {
            let config_path_str =
                cf.path()
                    .to_str()
                    .ok_or_else(|| MermaidRenderError::InvalidPath {
                        path: cf.path().to_string_lossy().into_owned(),
                    })?;
            cmd.args(["--configFile", config_path_str]);
        }

        // Add icon packs
        cmd.arg("--iconPacks").args(ICON_PACKS);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);
            return Err(MermaidRenderError::MmdcExecutionFailed { exit_code, stderr });
        }

        Ok(())
    }
}

impl From<String> for MermaidRenderer {
    fn from(instructions: String) -> Self {
        Self::new(instructions)
    }
}

impl From<&str> for MermaidRenderer {
    fn from(instructions: &str) -> Self {
        Self::new(instructions)
    }
}

/// Detects the diagram type from the first line of instructions.
///
/// Returns a human-readable string like "Flowchart diagram" or "Sequence diagram".
fn detect_diagram_type(instructions: &str) -> String {
    let first_line = instructions.lines().next().unwrap_or("").to_lowercase();

    if first_line.starts_with("flowchart") || first_line.starts_with("graph") {
        "Flowchart diagram".to_string()
    } else if first_line.starts_with("sequencediagram") {
        "Sequence diagram".to_string()
    } else if first_line.starts_with("classdiagram") {
        "Class diagram".to_string()
    } else if first_line.starts_with("statediagram") {
        "State diagram".to_string()
    } else if first_line.starts_with("erdiagram") {
        "Entity-Relationship diagram".to_string()
    } else if first_line.starts_with("gantt") {
        "Gantt chart".to_string()
    } else if first_line.starts_with("pie") {
        "Pie chart".to_string()
    } else if first_line.starts_with("journey") {
        "User journey diagram".to_string()
    } else if first_line.starts_with("gitgraph") {
        "Git graph".to_string()
    } else if first_line.starts_with("mindmap") {
        "Mind map".to_string()
    } else if first_line.starts_with("timeline") {
        "Timeline".to_string()
    } else if first_line.starts_with("quadrantchart") {
        "Quadrant chart".to_string()
    } else if first_line.starts_with("sankey") {
        "Sankey diagram".to_string()
    } else if first_line.starts_with("xychart") {
        "XY chart".to_string()
    } else {
        "Mermaid diagram".to_string()
    }
}

/// Returns a fallback code block string for the given instructions.
///
/// This is a standalone function for use when you don't need the full
/// `MermaidRenderer` struct.
///
/// ## Examples
///
/// ```rust
/// use biscuit_terminal::components::mermaid::fallback_code_block;
///
/// let output = fallback_code_block("flowchart LR\n    A --> B");
/// assert!(output.starts_with("```mermaid\n"));
/// assert!(output.ends_with("\n```"));
/// ```
pub fn fallback_code_block(instructions: &str) -> String {
    format!("```mermaid\n{}\n```", instructions)
}

/// Prints a fallback code block for the given instructions to stdout.
///
/// This is a standalone function for use when you don't need the full
/// `MermaidRenderer` struct.
pub fn print_fallback_code_block(instructions: &str) {
    println!("{}", fallback_code_block(instructions));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mermaid_renderer_new() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
        assert_eq!(renderer.instructions(), "flowchart LR\n    A --> B");
        assert!(renderer.title().is_none());
    }

    #[test]
    fn test_mermaid_renderer_with_title() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B").with_title("My Flowchart");
        assert_eq!(renderer.title(), Some("My Flowchart"));
    }

    #[test]
    fn test_mermaid_renderer_from_string() {
        let instructions = String::from("flowchart LR\n    A --> B");
        let renderer = MermaidRenderer::from(instructions.clone());
        assert_eq!(renderer.instructions(), instructions);
    }

    #[test]
    fn test_mermaid_renderer_from_str() {
        let renderer = MermaidRenderer::from("flowchart LR\n    A --> B");
        assert_eq!(renderer.instructions(), "flowchart LR\n    A --> B");
    }

    #[test]
    fn test_mermaid_renderer_clone() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B").with_title("Test");
        let cloned = renderer.clone();
        assert_eq!(renderer.instructions(), cloned.instructions());
        assert_eq!(renderer.title(), cloned.title());
    }

    #[test]
    fn test_fallback_code_block() {
        let output = fallback_code_block("flowchart LR\n    A --> B");
        assert!(output.starts_with("```mermaid\n"));
        assert!(output.ends_with("\n```"));
        assert!(output.contains("A --> B"));
    }

    #[test]
    fn test_mermaid_renderer_fallback_code_block() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
        let output = renderer.fallback_code_block();
        assert!(output.starts_with("```mermaid\n"));
        assert!(output.ends_with("\n```"));
    }

    #[test]
    fn test_detect_diagram_type_flowchart() {
        assert_eq!(
            detect_diagram_type("flowchart LR\n    A --> B"),
            "Flowchart diagram"
        );
        assert_eq!(
            detect_diagram_type("graph TD\n    A --> B"),
            "Flowchart diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_sequence() {
        assert_eq!(
            detect_diagram_type("sequenceDiagram\n    A->>B: Hello"),
            "Sequence diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_class() {
        assert_eq!(
            detect_diagram_type("classDiagram\n    class Animal"),
            "Class diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_state() {
        assert_eq!(
            detect_diagram_type("stateDiagram-v2\n    [*] --> State1"),
            "State diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_er() {
        assert_eq!(
            detect_diagram_type("erDiagram\n    CUSTOMER ||--o{ ORDER"),
            "Entity-Relationship diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_gantt() {
        assert_eq!(
            detect_diagram_type("gantt\n    title A Gantt Diagram"),
            "Gantt chart"
        );
    }

    #[test]
    fn test_detect_diagram_type_pie() {
        assert_eq!(detect_diagram_type("pie\n    \"Dogs\" : 386"), "Pie chart");
    }

    #[test]
    fn test_detect_diagram_type_journey() {
        assert_eq!(
            detect_diagram_type("journey\n    title My working day"),
            "User journey diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_gitgraph() {
        assert_eq!(detect_diagram_type("gitGraph\n    commit"), "Git graph");
    }

    #[test]
    fn test_detect_diagram_type_mindmap() {
        assert_eq!(
            detect_diagram_type("mindmap\n    root((mindmap))"),
            "Mind map"
        );
    }

    #[test]
    fn test_detect_diagram_type_timeline() {
        assert_eq!(
            detect_diagram_type("timeline\n    title History"),
            "Timeline"
        );
    }

    #[test]
    fn test_detect_diagram_type_unknown() {
        assert_eq!(
            detect_diagram_type("unknown\n    foo bar"),
            "Mermaid diagram"
        );
    }

    #[test]
    fn test_alt_text_with_title() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B").with_title("Custom Title");
        assert_eq!(renderer.alt_text(), "Custom Title");
    }

    #[test]
    fn test_alt_text_auto_detect() {
        let renderer = MermaidRenderer::new("sequenceDiagram\n    A->>B: Hello");
        assert_eq!(renderer.alt_text(), "Sequence diagram");
    }

    #[test]
    fn test_error_display_mmdc_not_found() {
        let error = MermaidRenderError::MmdcNotFound;
        assert_eq!(
            error.to_string(),
            "mmdc CLI not found. Install with: npm install -g @mermaid-js/mermaid-cli"
        );
    }

    #[test]
    fn test_error_display_npm_not_found() {
        let error = MermaidRenderError::NpmNotFound;
        assert_eq!(
            error.to_string(),
            "npm not found. Install Node.js and npm to render Mermaid diagrams in the terminal"
        );
    }

    #[test]
    fn test_error_display_mmdc_execution_failed() {
        let error = MermaidRenderError::MmdcExecutionFailed {
            exit_code: 1,
            stderr: "Invalid syntax".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "mmdc execution failed (exit code 1): Invalid syntax"
        );
    }

    #[test]
    fn test_error_display_content_too_large() {
        let error = MermaidRenderError::ContentTooLarge {
            size: 15000,
            max: 10000,
        };
        assert_eq!(
            error.to_string(),
            "Diagram too large (15000 bytes, max 10000)"
        );
    }

    #[test]
    fn test_error_display_display_error() {
        let error = MermaidRenderError::DisplayError("failed to render".to_string());
        assert_eq!(
            error.to_string(),
            "Failed to display image: failed to render"
        );
    }

    #[test]
    fn test_error_display_no_image_support() {
        let error = MermaidRenderError::NoImageSupport;
        assert!(
            error
                .to_string()
                .contains("does not support image rendering")
        );
    }

    #[test]
    fn test_max_diagram_size_constant() {
        assert_eq!(MAX_DIAGRAM_SIZE, 10_000);
    }

    #[test]
    fn test_small_diagram_under_limit() {
        let instructions = "flowchart LR\n    A --> B";
        assert!(instructions.len() < MAX_DIAGRAM_SIZE);
    }

    #[test]
    fn test_command_exists_with_common_command() {
        // 'which' should exist on all Unix systems
        assert!(command_exists("which"));
    }

    #[test]
    fn test_command_exists_with_nonexistent_command() {
        assert!(!command_exists(
            "this_command_definitely_does_not_exist_xyz123"
        ));
    }

    #[test]
    fn test_icon_packs_constant() {
        assert_eq!(ICON_PACKS.len(), 4);
        assert!(ICON_PACKS.contains(&"@iconify-json/fa7-brands"));
        assert!(ICON_PACKS.contains(&"@iconify-json/lucide"));
        assert!(ICON_PACKS.contains(&"@iconify-json/carbon"));
        assert!(ICON_PACKS.contains(&"@iconify-json/system-uicons"));
    }

    #[test]
    fn test_render_to_temp_png_rejects_large_content() {
        // Create a diagram that exceeds the size limit
        let large_instructions = "A".repeat(MAX_DIAGRAM_SIZE + 1);
        let renderer = MermaidRenderer::new(large_instructions);

        let result = renderer.render_to_temp_png();
        assert!(matches!(
            result,
            Err(MermaidRenderError::ContentTooLarge { .. })
        ));
    }

    #[test]
    fn test_render_to_file_rejects_large_content() {
        let large_instructions = "B".repeat(MAX_DIAGRAM_SIZE + 1);
        let renderer = MermaidRenderer::new(large_instructions);

        let result = renderer.render_to_file(std::path::Path::new("/tmp/test.png"));
        assert!(matches!(
            result,
            Err(MermaidRenderError::ContentTooLarge { .. })
        ));
    }

    #[test]
    fn test_detect_diagram_type_quadrant() {
        assert_eq!(
            detect_diagram_type("quadrantChart\n    title Test"),
            "Quadrant chart"
        );
    }

    #[test]
    fn test_detect_diagram_type_sankey() {
        assert_eq!(
            detect_diagram_type("sankey-beta\n    A[Source]"),
            "Sankey diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_xychart() {
        assert_eq!(
            detect_diagram_type("xychart-beta\n    title Test"),
            "XY chart"
        );
    }

    #[test]
    fn test_render_for_terminal_rejects_large_content() {
        let large_instructions = "C".repeat(MAX_DIAGRAM_SIZE + 1);
        let renderer = MermaidRenderer::new(large_instructions);

        let result = renderer.render_for_terminal();
        assert!(matches!(
            result,
            Err(MermaidRenderError::ContentTooLarge { .. })
        ));
    }

    #[test]
    fn test_mermaid_renderer_debug() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B").with_title("Test");
        let debug_str = format!("{:?}", renderer);
        assert!(debug_str.contains("MermaidRenderer"));
        assert!(debug_str.contains("flowchart"));
    }

    #[test]
    fn test_mermaid_config_default() {
        let config = MermaidConfig::new();
        assert!(config.point_label_font_size.is_none());
        assert!(config.point_radius.is_none());
        assert!(!config.has_options());
        assert!(config.to_json().is_none());
    }

    #[test]
    fn test_mermaid_config_with_point_label_font_size() {
        let config = MermaidConfig::new().with_point_label_font_size(16);
        assert_eq!(config.point_label_font_size, Some(16));
        assert!(config.has_options());
        let json = config.to_json().unwrap();
        assert!(json.contains("\"pointLabelFontSize\": 16"));
    }

    #[test]
    fn test_mermaid_config_with_point_radius() {
        let config = MermaidConfig::new().with_point_radius(10);
        assert_eq!(config.point_radius, Some(10));
        assert!(config.has_options());
        let json = config.to_json().unwrap();
        assert!(json.contains("\"pointRadius\": 10"));
    }

    #[test]
    fn test_mermaid_config_with_both_options() {
        let config = MermaidConfig::new()
            .with_point_label_font_size(14)
            .with_point_radius(8);
        assert_eq!(config.point_label_font_size, Some(14));
        assert_eq!(config.point_radius, Some(8));
        assert!(config.has_options());
        let json = config.to_json().unwrap();
        assert!(json.contains("\"pointLabelFontSize\": 14"));
        assert!(json.contains("\"pointRadius\": 8"));
        assert!(json.contains("\"quadrantChart\""));
    }

    #[test]
    fn test_mermaid_renderer_with_config() {
        let config = MermaidConfig::new()
            .with_point_label_font_size(16)
            .with_point_radius(12);
        let renderer = MermaidRenderer::new("quadrantChart\n    A: [0.5, 0.5]").with_config(config);
        // Just verify it builds without panicking
        assert!(renderer.instructions().contains("quadrantChart"));
    }

    #[test]
    fn test_quadrant_theme_default() {
        use crate::discovery::detection::ColorMode;
        assert_eq!(QuadrantTheme::Default.as_str(), "default");
        let config = QuadrantTheme::Default.apply(MermaidConfig::new(), ColorMode::Dark);
        assert!(config.quadrant1_fill.is_none());
        assert!(config.quadrant3_fill.is_none());
    }

    #[test]
    fn test_quadrant_theme_magic_quadrangle_dark() {
        use crate::discovery::detection::ColorMode;
        assert_eq!(QuadrantTheme::MagicQuadrangle.as_str(), "magic-quadrangle");
        let config = QuadrantTheme::MagicQuadrangle.apply(MermaidConfig::new(), ColorMode::Dark);
        // Dark mode: dark colors with subtle tints
        // q1 (top-right): green tint for "leaders"
        assert_eq!(config.quadrant1_fill, Some("#1e2a1e".to_string()));
        // q3 (bottom-left): red tint for "niche players"
        assert_eq!(config.quadrant3_fill, Some("#2a1e1e".to_string()));
        // q2 (top-left) and q4 (bottom-right): same dark neutral grey
        assert_eq!(config.quadrant2_fill, Some("#1a1a1a".to_string()));
        assert_eq!(config.quadrant4_fill, Some("#1a1a1a".to_string()));
    }

    #[test]
    fn test_quadrant_theme_magic_quadrangle_light() {
        use crate::discovery::detection::ColorMode;
        let config = QuadrantTheme::MagicQuadrangle.apply(MermaidConfig::new(), ColorMode::Light);
        // Light mode: light colors with subtle tints
        // q1 (top-right): green tint for "leaders"
        assert_eq!(config.quadrant1_fill, Some("#f6faf6".to_string()));
        // q3 (bottom-left): red tint for "niche players"
        assert_eq!(config.quadrant3_fill, Some("#faf6f6".to_string()));
        // q2 (top-left) and q4 (bottom-right): same light neutral grey
        assert_eq!(config.quadrant2_fill, Some("#f8f8f8".to_string()));
        assert_eq!(config.quadrant4_fill, Some("#f8f8f8".to_string()));
    }

    #[test]
    fn test_quadrant_theme_from_str() {
        assert_eq!(
            QuadrantTheme::parse("default"),
            Some(QuadrantTheme::Default)
        );
        assert_eq!(
            QuadrantTheme::parse("magic-quadrangle"),
            Some(QuadrantTheme::MagicQuadrangle)
        );
        assert_eq!(
            QuadrantTheme::parse("magic_quadrangle"),
            Some(QuadrantTheme::MagicQuadrangle)
        );
        assert_eq!(
            QuadrantTheme::parse("MAGIC-QUADRANGLE"),
            Some(QuadrantTheme::MagicQuadrangle)
        );
        assert_eq!(QuadrantTheme::parse("unknown"), None);
    }

    #[test]
    fn test_mermaid_config_with_quadrant_fill() {
        let config = MermaidConfig::new()
            .with_quadrant_fill(1, "#ff0000")
            .with_quadrant_fill(3, "#00ff00");
        assert_eq!(config.quadrant1_fill, Some("#ff0000".to_string()));
        assert_eq!(config.quadrant3_fill, Some("#00ff00".to_string()));
        assert!(config.quadrant2_fill.is_none());
        assert!(config.quadrant4_fill.is_none());
    }

    #[test]
    fn test_mermaid_config_quadrant_fill_json() {
        let config = MermaidConfig::new()
            .with_quadrant_fill(1, "#e8f5e9")
            .with_quadrant_fill(3, "#ffebee");
        let json = config.to_json().unwrap();
        assert!(json.contains("\"themeVariables\""));
        assert!(json.contains("\"quadrant1Fill\": \"#e8f5e9\""));
        assert!(json.contains("\"quadrant3Fill\": \"#ffebee\""));
    }

    #[test]
    fn test_mermaid_config_combined_options_json() {
        let config = MermaidConfig::new()
            .with_point_label_font_size(18)
            .with_quadrant_fill(1, "#e8f5e9");
        let json = config.to_json().unwrap();
        assert!(json.contains("\"quadrantChart\""));
        assert!(json.contains("\"pointLabelFontSize\": 18"));
        assert!(json.contains("\"themeVariables\""));
        assert!(json.contains("\"quadrant1Fill\": \"#e8f5e9\""));
    }

    // === MmdcVersion Tests ===

    #[test]
    fn test_mmdc_version_parse_valid() {
        let v = MmdcVersion::parse("10.6.0").unwrap();
        assert_eq!(v.major, 10);
        assert_eq!(v.minor, 6);
        assert_eq!(v.patch, 0);

        let v2 = MmdcVersion::parse("11.0.1").unwrap();
        assert_eq!(v2.major, 11);
        assert_eq!(v2.minor, 0);
        assert_eq!(v2.patch, 1);
    }

    #[test]
    fn test_mmdc_version_parse_with_whitespace() {
        let v = MmdcVersion::parse("  10.9.1  ").unwrap();
        assert_eq!(v.major, 10);
        assert_eq!(v.minor, 9);
        assert_eq!(v.patch, 1);
    }

    #[test]
    fn test_mmdc_version_parse_invalid() {
        assert!(MmdcVersion::parse("invalid").is_none());
        assert!(MmdcVersion::parse("10.6").is_none());
        assert!(MmdcVersion::parse("10").is_none());
        assert!(MmdcVersion::parse("").is_none());
        assert!(MmdcVersion::parse("a.b.c").is_none());
    }

    #[test]
    fn test_mmdc_version_comparison() {
        let v10_6_0 = MmdcVersion::parse("10.6.0").unwrap();
        let v10_6_1 = MmdcVersion::parse("10.6.1").unwrap();
        let v10_7_0 = MmdcVersion::parse("10.7.0").unwrap();
        let v11_0_0 = MmdcVersion::parse("11.0.0").unwrap();
        let v9_9_9 = MmdcVersion::parse("9.9.9").unwrap();

        assert!(v10_6_1 > v10_6_0);
        assert!(v10_7_0 > v10_6_0);
        assert!(v11_0_0 > v10_6_0);
        assert!(v9_9_9 < v10_6_0);
    }

    #[test]
    fn test_mmdc_version_minimum() {
        let min = MmdcVersion::minimum();
        assert_eq!(min.to_string(), MMDC_MIN_VERSION);
    }

    #[test]
    fn test_mmdc_version_meets_minimum() {
        let old = MmdcVersion::parse("9.5.0").unwrap();
        let exact = MmdcVersion::parse(MMDC_MIN_VERSION).unwrap();
        let newer = MmdcVersion::parse("11.0.0").unwrap();

        assert!(!old.meets_minimum());
        assert!(exact.meets_minimum());
        assert!(newer.meets_minimum());
    }

    #[test]
    fn test_mmdc_version_display() {
        let v = MmdcVersion::parse("10.9.1").unwrap();
        assert_eq!(v.to_string(), "10.9.1");
    }
}
