//! Mermaid diagram theming and rendering.
//!
//! This module provides support for Mermaid diagram theming, allowing
//! custom color schemes to be applied to diagrams based on syntax highlighting
//! themes.
//!
//! ## Modules
//!
//! - [`theme`] - Mermaid theme color schemes and JSON parsing
//! - [`render_html`] - HTML rendering with accessibility features
//! - [`render_terminal`] - Terminal rendering via local mmdc CLI

pub mod render_html;
pub mod render_terminal;
pub mod theme;

pub use render_html::MermaidHtml;
pub use render_terminal::MermaidRenderError;
pub use theme::{
    mermaid_theme_for_syntect, MermaidTheme, MermaidThemeError, DEFAULT_DARK_THEME,
    DEFAULT_LIGHT_THEME, NEUTRAL_THEME,
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
/// use shared::mermaid::Mermaid;
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
    /// use shared::mermaid::Mermaid;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B");
    /// ```
    #[tracing::instrument(skip(instructions))]
    pub fn new<S: Into<String>>(instructions: S) -> Self {
        let instructions = instructions.into();
        tracing::trace!(instructions_len = instructions.len(), "Creating Mermaid diagram");
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
    /// use shared::mermaid::{Mermaid, DEFAULT_LIGHT_THEME, DEFAULT_DARK_THEME};
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
    /// use shared::mermaid::Mermaid;
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
    /// use shared::mermaid::Mermaid;
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
    /// use shared::mermaid::Mermaid;
    /// use shared::markdown::highlighting::ThemePair;
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
    /// use shared::mermaid::Mermaid;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B");
    /// let hash = diagram.hash();
    /// ```
    pub fn hash(&self) -> u64 {
        crate::hashing::xx_hash_normalized(&self.instructions)
    }

    /// Returns the raw instructions.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use shared::mermaid::Mermaid;
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
    /// use shared::mermaid::Mermaid;
    /// use shared::markdown::highlighting::ColorMode;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B");
    /// let theme = diagram.theme(ColorMode::Light);
    /// ```
    pub fn theme(&self, mode: ColorMode) -> &MermaidTheme {
        if let Some((ref light, ref dark)) = self.custom_theme {
            match mode {
                ColorMode::Light => light,
                ColorMode::Dark => dark,
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
    /// use shared::mermaid::Mermaid;
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
    /// use shared::mermaid::Mermaid;
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
    /// use shared::mermaid::Mermaid;
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
            render_html::detect_diagram_type(&self.instructions).to_string()
        }
    }

    /// Renders the diagram for HTML output.
    ///
    /// Returns a `MermaidHtml` struct with separate head and body sections.
    /// The head contains:
    /// - Mermaid.js ESM module import from CDN
    /// - CSS variables for theme colors (light/dark via prefers-color-scheme)
    ///
    /// The body contains:
    /// - `<pre class="mermaid">` element with diagram instructions
    /// - ARIA attributes: `role="img"`, `aria-label` with alt text
    /// - Optional `title` attribute if title is set
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use shared::mermaid::Mermaid;
    ///
    /// let diagram = Mermaid::new("flowchart LR\n    A --> B")
    ///     .with_title("My Flowchart");
    /// let html = diagram.render_for_html();
    ///
    /// // Embed in HTML document
    /// println!("<html><head>{}</head><body>{}</body></html>", html.head, html.body);
    /// ```
    pub fn render_for_html(&self) -> MermaidHtml {
        use html_escape::encode_text;

        let light_theme = self.theme(ColorMode::Light);
        let dark_theme = self.theme(ColorMode::Dark);

        // Generate head content
        let css_vars = render_html::generate_css_variables(light_theme, dark_theme);
        let head = format!(
            r#"{css_vars}
<script type="module">
  import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';
  mermaid.registerIconPacks([
    {{ name: 'fa7-brands', loader: () => fetch('https://unpkg.com/@iconify-json/fa7-brands@1/icons.json').then(r => r.json()) }},
    {{ name: 'lucide', loader: () => fetch('https://unpkg.com/@iconify-json/lucide@1/icons.json').then(r => r.json()) }},
    {{ name: 'carbon', loader: () => fetch('https://unpkg.com/@iconify-json/carbon@1/icons.json').then(r => r.json()) }},
    {{ name: 'system-uicons', loader: () => fetch('https://unpkg.com/@iconify-json/system-uicons@1/icons.json').then(r => r.json()) }}
  ]);
  mermaid.initialize({{ startOnLoad: true }});
</script>"#
        );

        // Generate body content
        let alt = self.alt_text();
        let escaped_alt = encode_text(&alt);
        let escaped_instructions = encode_text(&self.instructions);

        let title_attr = if let Some(title) = &self.title {
            format!(r#" title="{}""#, encode_text(title))
        } else {
            String::new()
        };

        let body = format!(
            r#"<pre class="mermaid" role="img" aria-label="{escaped_alt}"{title_attr}>{escaped_instructions}</pre>"#
        );

        MermaidHtml::new(head, body)
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
    /// use shared::mermaid::Mermaid;
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
        let diagram = Mermaid::new("flowchart LR\n    A --> B").with_theme(light.clone(), dark.clone());

        assert_eq!(diagram.theme(ColorMode::Light), &light);
        assert_eq!(diagram.theme(ColorMode::Dark), &dark);
    }

    #[test]
    fn test_mermaid_use_syntect_theme() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B").use_syntect_theme(ThemePair::Gruvbox);

        // Should resolve to syntect themes, not custom
        let light_theme = diagram.theme(ColorMode::Light);
        let dark_theme = diagram.theme(ColorMode::Dark);

        // Verify these are from syntect resolution
        assert_eq!(light_theme, mermaid_theme_for_syntect(ThemePair::Gruvbox, ColorMode::Light));
        assert_eq!(dark_theme, mermaid_theme_for_syntect(ThemePair::Gruvbox, ColorMode::Dark));
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
        assert_eq!(theme, mermaid_theme_for_syntect(ThemePair::OneHalf, ColorMode::Light));
    }

    #[test]
    fn test_mermaid_theme_resolution_dark() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B");
        let theme = diagram.theme(ColorMode::Dark);
        assert_eq!(theme, mermaid_theme_for_syntect(ThemePair::OneHalf, ColorMode::Dark));
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
    fn test_render_html_contains_mermaid_esm() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B");
        let html = diagram.render_for_html();
        assert!(html.head.contains("https://cdn.jsdelivr.net/npm/mermaid"));
        assert!(html.head.contains("type=\"module\""));
    }

    #[test]
    fn test_render_html_has_aria_attributes() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B");
        let html = diagram.render_for_html();
        assert!(html.body.contains(r#"role="img""#));
        assert!(html.body.contains(r#"aria-label=""#));
    }

    #[test]
    fn test_render_html_escapes_instructions() {
        let diagram = Mermaid::new("flowchart LR\n    A[\"<script>alert('xss')</script>\"] --> B");
        let html = diagram.render_for_html();
        // Should escape the HTML entities
        assert!(html.body.contains("&lt;script&gt;"));
        assert!(html.body.contains("&lt;/script&gt;"));
        // Should not contain raw script tags
        assert!(!html.body.contains("<script>alert"));
    }

    #[test]
    fn test_render_html_escapes_title() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B")
            .with_title("<script>alert('xss')</script>");
        let html = diagram.render_for_html();
        // Should escape the title attribute
        assert!(html.body.contains("&lt;script&gt;"));
        // Should not contain raw script tags
        assert!(!html.body.contains("title=\"<script>"));
    }

    #[test]
    fn test_html_head_snapshot() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B").with_title("Test Flowchart");
        let html = diagram.render_for_html();
        insta::assert_snapshot!(html.head);
    }

    #[test]
    fn test_html_body_snapshot() {
        let diagram = Mermaid::new("flowchart LR\n    A --> B").with_title("Test Flowchart");
        let html = diagram.render_for_html();
        insta::assert_snapshot!(html.body);
    }
}
