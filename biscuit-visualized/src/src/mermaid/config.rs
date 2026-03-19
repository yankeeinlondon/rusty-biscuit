/// Theme selection for Mermaid diagrams.
///
/// Determines the color scheme used for diagram rendering.
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::mermaid::MermaidTheme;
///
/// // Select theme based on terminal color mode
/// let theme = MermaidTheme::for_color_mode(true); // true = dark mode
/// assert_eq!(theme, MermaidTheme::Dark);
///
/// // Get inverse theme
/// let light_theme = MermaidTheme::Dark.inverse();
/// assert_eq!(light_theme, MermaidTheme::Default);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MermaidTheme {
    /// Dark theme - light text on dark background (default for dark terminals).
    #[default]
    Dark,
    /// Default/light theme - dark text on light background.
    Default,
    /// Forest theme - green tones.
    Forest,
    /// Neutral theme - grayscale, works well with transparent backgrounds.
    Neutral,
}

impl MermaidTheme {
    /// Returns the theme string for rendering.
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
    /// ## Arguments
    ///
    /// * `is_dark` - Whether the terminal/environment is in dark mode
    ///
    /// ## Returns
    ///
    /// - Dark mode (`is_dark = true`) returns `Dark` theme
    /// - Light mode (`is_dark = false`) returns `Default` theme
    pub fn for_color_mode(is_dark: bool) -> Self {
        if is_dark {
            MermaidTheme::Dark
        } else {
            MermaidTheme::Default
        }
    }

    /// Returns the inverse theme (for solid background rendering).
    ///
    /// ## Returns
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

/// Preset themes for quadrant charts.
///
/// Quadrant themes customize the color scheme for quadrant charts (such as
/// Gartner-style magic quadrant visualizations).
///
/// ## Variants
///
/// - **`Default`**: Uses Mermaid's default colors with no customization.
/// - **`MagicQuadrangle`**: Gartner-inspired theme with semantic colors:
///   - Top-right (leaders): subtle green
///   - Bottom-left (niche players): subtle red
///   - Top-left and bottom-right: neutral color (adapts to light/dark mode)
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::mermaid::{MermaidConfig, QuadrantTheme};
///
/// // Use default theme
/// let config = QuadrantTheme::Default.apply(MermaidConfig::new(), true);
///
/// // Use Gartner-style Magic Quadrangle theme
/// let config = QuadrantTheme::MagicQuadrangle.apply(MermaidConfig::new(), true);
/// // Quadrant 1 (top-right) gets subtle green
/// // Quadrant 3 (bottom-left) gets subtle red
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum QuadrantTheme {
    /// Default Mermaid colors (no customization).
    #[default]
    Default,
    /// Magic Quadrangle style (Gartner-inspired): subtle green for top-right (leaders),
    /// subtle red for bottom-left (niche players). Top-left and bottom-right use a
    /// neutral color (darker in dark mode, lighter in light mode).
    /// Colors adapt to color mode.
    #[cfg_attr(feature = "clap", clap(name = "magic-quadrangle"))]
    MagicQuadrangle,
}

impl QuadrantTheme {
    /// Applies this theme's colors to a MermaidConfig.
    ///
    /// Colors are adapted based on the color mode:
    /// - Dark mode: Uses dark colors with subtle green/red tints
    /// - Light mode: Uses light colors with subtle green/red tints
    ///
    /// The Magic Quadrangle theme uses:
    /// - Top-right (q1): subtle green (leaders)
    /// - Bottom-left (q3): subtle red (niche players)
    /// - Top-left (q2) and bottom-right (q4): same neutral color (darker in dark mode, lighter in light mode)
    ///
    /// ## Arguments
    ///
    /// * `config` - The configuration to apply colors to
    /// * `is_dark` - Whether to use dark mode colors
    pub fn apply(self, mut config: MermaidConfig, is_dark: bool) -> MermaidConfig {
        match self {
            QuadrantTheme::Default => config,
            QuadrantTheme::MagicQuadrangle => {
                if is_dark {
                    // Dark mode: dark colors with subtle tints
                    // Green for top-right (quadrant-1, "leaders") - dark with green undertone
                    config.quadrant1_fill = Some("#1e2a1e".to_string());
                    // Red for bottom-left (quadrant-3, "niche players") - dark with red undertone
                    config.quadrant3_fill = Some("#2a1e1e".to_string());
                    // Top-left (q2) and bottom-right (q4): same dark neutral grey
                    let neutral = "#1a1a1a".to_string();
                    config.quadrant2_fill = Some(neutral.clone());
                    config.quadrant4_fill = Some(neutral);
                } else {
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
    /// ## Returns
    ///
    /// Returns `None` if the string doesn't match a known theme.
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
/// These options are passed to the Mermaid renderer via JSON configuration.
/// Only non-None values are included in the generated config.
///
/// ## Supported Diagrams
///
/// Currently, these options primarily affect **quadrant charts**:
/// - `point_label_font_size` - Font size for point labels
/// - `point_radius` - Radius of data points
/// - `quadrant1_fill` through `quadrant4_fill` - Background fill colors per quadrant
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::mermaid::MermaidConfig;
///
/// // Empty config (uses renderer defaults)
/// let config = MermaidConfig::new();
///
/// // Customized quadrant chart config
/// let config = MermaidConfig::new()
///     .with_point_label_font_size(14)
///     .with_point_radius(8)
///     .with_quadrant_fill(1, "#ff0000")   // top-right: red
///     .with_quadrant_fill(2, "#00ff00")   // top-left: green
///     .with_quadrant_fill(3, "#0000ff")   // bottom-left: blue
///     .with_quadrant_fill(4, "#ffff00");  // bottom-right: yellow
///
/// // Generate JSON config
/// let json = config.to_json();
/// ```
///
/// ## Quadrant Numbering
///
/// ```text
///     │
///  2  │  1
/// ────┼────
///  3  │  4
///     │
/// ```
#[derive(Debug, Clone, Default)]
pub struct MermaidConfig {
    /// Quadrant chart: point label font size (default: 12).
    pub point_label_font_size: Option<u32>,
    /// Quadrant chart: default point radius (default: 5).
    pub point_radius: Option<u32>,
    /// Quadrant chart: top-right (quadrant-1) fill color.
    pub quadrant1_fill: Option<String>,
    /// Quadrant chart: top-left (quadrant-2) fill color.
    pub quadrant2_fill: Option<String>,
    /// Quadrant chart: bottom-left (quadrant-3) fill color.
    pub quadrant3_fill: Option<String>,
    /// Quadrant chart: bottom-right (quadrant-4) fill color.
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

    /// Generates the JSON configuration for the Mermaid renderer.
    ///
    /// ## Returns
    ///
    /// Returns `None` if no options are set.
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
