//! Mermaid diagram theming and rendering.
//!
//! This module provides support for Mermaid diagram theming, allowing
//! custom color schemes to be applied to diagrams based on syntax highlighting
//! themes.
//!
//! ## Modules
//!
//! - [`theme`] - Mermaid theme color schemes and JSON parsing
//! - [`feature`] - Darkmatter's browser [`FeatureResolver`], the single owner of
//!   the inline ESM bootstrap (a script-only bundle; the palette rides Mermaid
//!   `themeVariables`, not CSS)
//! - [`render_terminal`] - Terminal rendering via local mmdc CLI

pub mod feature;
pub mod render_terminal;
pub mod theme;

pub use feature::{
    DarkmatterFeatureResolver, MERMAID_CDN_FALLBACK_ORIGIN, MERMAID_CDN_PRIMARY_ORIGIN,
    MERMAID_VERSION,
};
pub use render_terminal::MermaidRenderError;
pub use theme::{
    DEFAULT_DARK_THEME, DEFAULT_LIGHT_THEME, MermaidTheme, MermaidThemeError, NEUTRAL_THEME,
    mermaid_theme_for_syntect,
};

use crate::markdown::highlighting::{ColorMode, ThemePair};

/// A Mermaid diagram with theming support.
///
/// This struct represents a Mermaid diagram with customizable theming
/// and metadata. It supports both custom themes and automatic theme
/// resolution from syntect theme pairs.
///
/// ## Examples
///
/// ```rust
/// use darkmatter::mermaid::Mermaid;
///
/// // Create a diagram from a string
/// let diagram = Mermaid::new("flowchart LR\n    A --> B");
///
/// // Use builder pattern for customization
/// let diagram = Mermaid::new("flowchart LR\n    A --> B")
///     .with_title("My Flowchart")
///     .with_footer("Generated 2026-01-03");
/// ```
#[derive(Debug, Clone)]
pub struct Mermaid {
    /// The Mermaid diagram instructions
    instructions: String,
    /// Theme pair enum for lazy resolution
    theme_pair: ThemePair,
    /// Custom themes override (if set, ignores theme_pair)
    custom_theme: Option<(MermaidTheme, MermaidTheme)>,
    /// Optional diagram title
    title: Option<String>,
    /// Optional diagram footer
    footer: Option<String>,
}

impl Mermaid {
    /// Creates a new Mermaid diagram with the given instructions.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter::mermaid::Mermaid;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B");
    /// ```
    #[tracing::instrument(skip(instructions))]
    pub fn new<S: Into<String>>(instructions: S) -> Self {
        let instructions = instructions.into();
        tracing::trace!(
            instructions_len = instructions.len(),
            "Creating Mermaid diagram"
        );
        Self {
            instructions,
            theme_pair: ThemePair::OneHalf,
            custom_theme: None,
            title: None,
            footer: None,
        }
    }

    /// Sets custom themes for light and dark modes.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter::mermaid::{Mermaid, DEFAULT_LIGHT_THEME, DEFAULT_DARK_THEME};
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B")
    ///     .with_theme(DEFAULT_LIGHT_THEME.clone(), DEFAULT_DARK_THEME.clone());
    /// ```
    pub fn with_theme(mut self, light: MermaidTheme, dark: MermaidTheme) -> Self {
        self.custom_theme = Some((light, dark));
        self
    }

    /// Sets the diagram title (also used for alt text).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter::mermaid::Mermaid;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B")
    ///     .with_title("My Flowchart");
    /// ```
    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the diagram footer.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter::mermaid::Mermaid;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B")
    ///     .with_footer("Generated 2026-01-03");
    /// ```
    pub fn with_footer<S: Into<String>>(mut self, footer: S) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Uses a syntect ThemePair for theme resolution.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter::mermaid::Mermaid;
    /// use darkmatter::markdown::highlighting::ThemePair;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B")
    ///     .use_syntect_theme(ThemePair::Gruvbox);
    /// ```
    pub fn use_syntect_theme(mut self, theme_pair: ThemePair) -> Self {
        self.theme_pair = theme_pair;
        self.custom_theme = None;
        self
    }

    /// Returns the XXH64 hash of the normalized instructions.
    ///
    /// The hash is computed on demand and is based on the instructions
    /// with blank lines removed for normalization.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter::mermaid::Mermaid;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B");
    /// let hash = diagram.hash();
    /// ```
    pub fn hash(&self) -> u64 {
        biscuit_hash::xx_hash_variant(
            &self.instructions,
            vec![biscuit_hash::HashVariant::BlankLine],
        )
    }

    /// Returns the raw instructions.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter::mermaid::Mermaid;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B");
    /// assert_eq!(diagram.instructions(), "flowchart LR\n    A --> B");
    /// ```
    pub fn instructions(&self) -> &str {
        &self.instructions
    }

    /// Returns the theme for the given color mode.
    ///
    /// If custom themes are set, they are used. Otherwise, the theme
    /// is resolved from the syntect theme pair.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter::mermaid::Mermaid;
    /// use darkmatter::markdown::highlighting::ColorMode;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B");
    /// let theme = diagram.theme(ColorMode::Light);
    /// ```
    pub fn theme(&self, mode: ColorMode) -> &MermaidTheme {
        if let Some((ref light, ref dark)) = self.custom_theme {
            match mode {
                ColorMode::Light => light,
                ColorMode::Dark | ColorMode::Unknown => dark,
            }
        } else {
            mermaid_theme_for_syntect(self.theme_pair, mode)
        }
    }

    /// Returns the title if set.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter::mermaid::Mermaid;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B")
    ///     .with_title("My Flowchart");
    /// assert_eq!(diagram.title(), Some("My Flowchart"));
    /// ```
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns the footer if set.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter::mermaid::Mermaid;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B")
    ///     .with_footer("Generated 2026-01-03");
    /// assert_eq!(diagram.footer(), Some("Generated 2026-01-03"));
    /// ```
    pub fn footer(&self) -> Option<&str> {
        self.footer.as_deref()
    }

    /// Returns alt text for accessibility.
    ///
    /// Uses the explicit title if set, otherwise detects the diagram type
    /// from the first line of instructions.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter::mermaid::Mermaid;
    ///
    /// // Explicit title
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B")
    ///     .with_title("My Flowchart");
    /// assert_eq!(diagram.alt_text(), "My Flowchart");
    ///
    /// // Auto-detected
    /// let diagram = Mermaid::new("sequenceDiagram\n    A->>B: Hello");
    /// assert_eq!(diagram.alt_text(), "Sequence diagram");
    /// ```
    pub fn alt_text(&self) -> String {
        if let Some(title) = &self.title {
            title.clone()
        } else {
            detect_diagram_type(&self.instructions).to_string()
        }
    }

    /// Renders the diagram to the terminal using the local mmdc CLI.
    ///
    /// This method executes the `mmdc` CLI tool to render the diagram as a PNG,
    /// then displays it in the terminal using viuer. On error, it falls back to
    /// printing the diagram as a code block.
    ///
    /// ## Icon Pack Support
    ///
    /// This method enables icon packs for diagrams:
    /// - `@iconify-json/fa7-brands` - Font Awesome 7 brand icons
    /// - `@iconify-json/lucide` - Lucide icons
    /// - `@iconify-json/carbon` - Carbon Design icons
    /// - `@iconify-json/system-uicons` - System UI icons
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use darkmatter::mermaid::Mermaid;
    ///
    /// fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let diagram = Mermaid::new("flowchart LR\n    A --> B");
    ///     diagram.render_for_terminal()?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns `MermaidRenderError` if:
    /// - mmdc CLI is not installed (install with `npm install -g @mermaid-js/mermaid-cli`)
    /// - Diagram is too large (> 10KB)
    /// - mmdc execution fails (invalid syntax, etc.)
    /// - Terminal doesn't support image rendering
    ///
    /// ## Error Handling
    ///
    /// Returns an error if rendering fails. The caller is responsible for
    /// handling the fallback (e.g., rendering as a syntax-highlighted code block).
    pub fn render_for_terminal(&self) -> Result<(), MermaidRenderError> {
        render_terminal::render_for_terminal(&self.instructions)
    }
}

/// Detects the diagram type from the first line of instructions.
///
/// ## Returns
///
/// A human-readable description of the diagram type, used as alt text when no
/// explicit [`Mermaid::with_title`] is set.
///
/// ## Examples
///
/// ```rust
/// use darkmatter::mermaid::detect_diagram_type;
/// assert_eq!(detect_diagram_type("flowchart LR"), "Flowchart diagram");
/// assert_eq!(detect_diagram_type("sequenceDiagram"), "Sequence diagram");
/// assert_eq!(detect_diagram_type("unknown"), "Mermaid diagram");
/// ```
pub fn detect_diagram_type(instructions: &str) -> &'static str {
    let first_line = instructions.lines().next().unwrap_or("").trim();

    if first_line.starts_with("flowchart") || first_line.starts_with("graph") {
        "Flowchart diagram"
    } else if first_line.starts_with("sequenceDiagram") {
        "Sequence diagram"
    } else if first_line.starts_with("classDiagram") {
        "Class diagram"
    } else if first_line.starts_with("stateDiagram") {
        "State diagram"
    } else if first_line.starts_with("erDiagram") {
        "Entity relationship diagram"
    } else if first_line.starts_with("pie") {
        "Pie chart"
    } else if first_line.starts_with("gantt") {
        "Gantt chart"
    } else if first_line.starts_with("journey") {
        "User journey diagram"
    } else if first_line.starts_with("gitGraph") || first_line.starts_with("gitgraph") {
        "Git graph diagram"
    } else if first_line.starts_with("mindmap") {
        "Mind map diagram"
    } else if first_line.starts_with("timeline") {
        "Timeline diagram"
    } else {
        "Mermaid diagram"
    }
}

impl Default for Mermaid {
    fn default() -> Self {
        Self::new(
            r#"flowchart LR
    A[Start] --> B{Decision}
    B -->|Yes| C[Action]
    B -->|No| D[End]"#,
        )
    }
}

impl From<String> for Mermaid {
    fn from(instructions: String) -> Self {
        Self::new(instructions)
    }
}

impl From<&str> for Mermaid {
    fn from(instructions: &str) -> Self {
        Self::new(instructions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mermaid_new_stores_instructions() {
        let instructions = "flowchart LR\n    A --> B";
        let diagram = Mermaid::new(instructions);
        assert_eq!(diagram.instructions(), instructions);
    }

    #[test]
    fn test_mermaid_from_string() {
        let instructions = String::from("flowchart LR\n    A --> B");
        let diagram = Mermaid::from(instructions.clone());
        assert_eq!(diagram.instructions(), instructions);
    }

    #[test]
    fn test_mermaid_from_str() {
        let instructions = "flowchart LR\n    A --> B";
        let diagram = Mermaid::from(instructions);
        assert_eq!(diagram.instructions(), instructions);
    }

    #[test]
    fn test_mermaid_default_has_flowchart() {
        let diagram = Mermaid::default();
        assert!(diagram.instructions().contains("flowchart"));
        assert!(diagram.instructions().contains("Start"));
        assert!(diagram.instructions().contains("Decision"));
    }

    #[test]
    fn test_mermaid_with_title() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B").with_title("Test Title");
        assert_eq!(diagram.title(), Some("Test Title"));
    }

    #[test]
    fn test_mermaid_with_footer() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B").with_footer("Test Footer");
        assert_eq!(diagram.footer(), Some("Test Footer"));
    }

    #[test]
    fn test_mermaid_with_theme_custom() {
        let light = DEFAULT_LIGHT_THEME.clone();
        let dark = DEFAULT_DARK_THEME.clone();
        let diagram =
            Mermaid::new("flowchart LR\n    A --> B").with_theme(light.clone(), dark.clone());

        assert_eq!(diagram.theme(ColorMode::Light), &light);
        assert_eq!(diagram.theme(ColorMode::Dark), &dark);
    }

    #[test]
    fn test_mermaid_use_syntect_theme() {
        let diagram =
            Mermaid::new("flowchart LR\n    A --> B").use_syntect_theme(ThemePair::Gruvbox);

        // Should resolve to syntect themes, not custom
        let light_theme = diagram.theme(ColorMode::Light);
        let dark_theme = diagram.theme(ColorMode::Dark);

        // Verify these are from syntect resolution
        assert_eq!(
            light_theme,
            mermaid_theme_for_syntect(ThemePair::Gruvbox, ColorMode::Light)
        );
        assert_eq!(
            dark_theme,
            mermaid_theme_for_syntect(ThemePair::Gruvbox, ColorMode::Dark)
        );
    }

    #[test]
    fn test_mermaid_hash_computed_on_demand() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B");
        let hash1 = diagram.hash();
        let hash2 = diagram.hash();
        assert_eq!(hash1, hash2); // Same diagram = same hash
    }

    #[test]
    fn test_mermaid_theme_resolution_light() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B");
        let theme = diagram.theme(ColorMode::Light);
        assert_eq!(
            theme,
            mermaid_theme_for_syntect(ThemePair::OneHalf, ColorMode::Light)
        );
    }

    #[test]
    fn test_mermaid_theme_resolution_dark() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B");
        let theme = diagram.theme(ColorMode::Dark);
        assert_eq!(
            theme,
            mermaid_theme_for_syntect(ThemePair::OneHalf, ColorMode::Dark)
        );
    }

    #[test]
    fn test_mermaid_clone() {
        let diagram1 = Mermaid::new("flowchart LR\n    A --> B")
            .with_title("Test")
            .with_footer("Footer");
        let diagram2 = diagram1.clone();

        assert_eq!(diagram1.instructions(), diagram2.instructions());
        assert_eq!(diagram1.title(), diagram2.title());
        assert_eq!(diagram1.footer(), diagram2.footer());
        assert_eq!(diagram1.hash(), diagram2.hash());
    }

    // HTML rendering tests
    #[test]
    fn test_alt_text_with_explicit_title() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B").with_title("My Custom Title");
        assert_eq!(diagram.alt_text(), "My Custom Title");
    }

    #[test]
    fn test_alt_text_flowchart() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B");
        assert_eq!(diagram.alt_text(), "Flowchart diagram");
    }

    #[test]
    fn test_alt_text_sequence() {
        let diagram = Mermaid::new("sequenceDiagram\n    A->>B: Hello");
        assert_eq!(diagram.alt_text(), "Sequence diagram");
    }

    #[test]
    fn test_alt_text_class() {
        let diagram = Mermaid::new("classDiagram\n    class Animal");
        assert_eq!(diagram.alt_text(), "Class diagram");
    }

    #[test]
    fn test_alt_text_unknown_type() {
        let diagram = Mermaid::new("unknown\n    foo bar");
        assert_eq!(diagram.alt_text(), "Mermaid diagram");
    }

    #[test]
    fn detect_diagram_type_covers_known_and_unknown_types() {
        assert_eq!(detect_diagram_type("flowchart LR"), "Flowchart diagram");
        assert_eq!(detect_diagram_type("graph LR"), "Flowchart diagram");
        assert_eq!(detect_diagram_type("sequenceDiagram"), "Sequence diagram");
        assert_eq!(detect_diagram_type("classDiagram"), "Class diagram");
        assert_eq!(detect_diagram_type("stateDiagram"), "State diagram");
        assert_eq!(
            detect_diagram_type("erDiagram"),
            "Entity relationship diagram"
        );
        assert_eq!(detect_diagram_type("pie"), "Pie chart");
        assert_eq!(detect_diagram_type("gantt"), "Gantt chart");
        assert_eq!(detect_diagram_type("journey"), "User journey diagram");
        assert_eq!(detect_diagram_type("gitGraph"), "Git graph diagram");
        assert_eq!(detect_diagram_type("gitgraph"), "Git graph diagram");
        assert_eq!(detect_diagram_type("mindmap"), "Mind map diagram");
        assert_eq!(detect_diagram_type("timeline"), "Timeline diagram");
        assert_eq!(detect_diagram_type("unknown"), "Mermaid diagram");
        assert_eq!(detect_diagram_type(""), "Mermaid diagram");
    }
}
