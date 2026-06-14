//! HTML output with syntax highlighting for code blocks and prose.
//!
//! This module provides HTML rendering for markdown documents with full syntax highlighting
//! support for both code blocks and prose elements. It uses syntect for highlighting and
//! supports customizable themes, line numbering, and line highlighting.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::Markdown;
//! use darkmatter::markdown::output::HtmlOptions;
//!
//! let content = "# Hello World\n\n\
//!                ```rust\n\
//!                fn main() {\n    \
//!                    println!(\"Hello!\");\n\
//!                }\n\
//!                ```\n";
//!
//! let md: Markdown = content.into();
//! let html = md.as_html(HtmlOptions::default()).unwrap();
//! assert!(html.contains("<code"));
//! ```

use crate::markdown::highlighting::{CodeBlockMode, CodeHighlighter, ColorMode, ThemePair};
use crate::markdown::output::terminal::MermaidMode;

/// Options for HTML output with sensible defaults.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::output::HtmlOptions;
/// use darkmatter::markdown::highlighting::{ThemePair, ColorMode};
///
/// let mut options = HtmlOptions::default();
/// options.code_theme = ThemePair::Github;
/// options.prose_theme = ThemePair::Github;
/// options.color_mode = ColorMode::Dark;
/// options.include_line_numbers = false;
/// options.include_styles = true;
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct HtmlOptions {
    /// Theme pair for code blocks.
    pub code_theme: ThemePair,
    /// Theme pair for prose elements.
    pub prose_theme: ThemePair,
    /// Color mode (light/dark).
    pub color_mode: ColorMode,
    /// Controls how a fenced code block's theme variant contrasts against the
    /// resolved page [`color_mode`](HtmlOptions::color_mode).
    ///
    /// The browser code path resolves the code `ThemePair` against this mode
    /// (e.g. `Inverse` paints a light panel on a dark page). It feeds both the
    /// highlighted markup and the injected `.code-block` background rule
    /// ([`code_block_background_hex`]) so the stylesheet and markup agree.
    pub code_block_mode: CodeBlockMode,
    /// Global default for line numbering (can be overridden per code block).
    pub include_line_numbers: bool,
    /// Include inline CSS styles.
    pub include_styles: bool,
    /// Controls how Mermaid diagrams are rendered.
    ///
    /// - `Off` (default): Show mermaid blocks as syntax-highlighted code
    /// - `Image`: Render as interactive mermaid diagrams (includes mermaid.js)
    /// - `Text`: Show as fenced code blocks (fallback format)
    pub mermaid_mode: MermaidMode,
    /// Page-level declarations for horizontal-rule CSS custom properties.
    ///
    /// When non-empty, the browser render path emits a `:root` CSS block
    /// declaring each entry as a custom property. Keys match the CSS variable names
    /// *without* the `--` prefix (e.g., `hr-weight`, `hr-color`,
    /// `hr-width`); each becomes a `--{key}: {value};` declaration.
    ///
    /// Components keep their `var(--hr-*, …)` expressions literal in the
    /// emitted SVG — they are never substituted. The declared `:root`
    /// values resolve against those `var()` expressions in the browser,
    /// overriding the per-`var()` fallbacks.
    ///
    /// ## Notes
    ///
    /// The `:root` block is emitted regardless of [`include_styles`], so
    /// the override still applies when the full stylesheet is suppressed.
    /// An empty map (the default) emits no `:root` block — the SVG's
    /// per-`var()` fallbacks then take effect.
    ///
    /// Values must be valid CSS token sequences. Entries whose key or
    /// value contains a `<` character or a newline are ignored, since
    /// they could break out of the `<style>` element or emit invalid CSS.
    ///
    /// [`include_styles`]: HtmlOptions::include_styles
    pub hr_css_variables: std::collections::HashMap<String, String>,
    /// Resolved HR defaults from `style.hr` frontmatter (or `DarkmatterPage`
    /// builder calls). When present, HTML rendering uses these as the
    /// default horizontal-rule style instead of reading the deprecated top-level
    /// `hr:` frontmatter block.
    pub hr_defaults: Option<crate::markdown::inline::HorizontalRuleAttrs>,
    /// Global hyperlink style from `style.hyperlinks.*`.
    pub hyperlink_style: Option<crate::style::schema::CommonStyle>,
    /// Local hyperlink override from `style.hyperlinks.local-style`.
    pub local_hyperlink_style: Option<crate::style::schema::CommonStyle>,
    /// Local image override from `style.images.local-style`.
    pub local_image_style: Option<crate::style::schema::CommonStyle>,
}

impl Default for HtmlOptions {
    fn default() -> Self {
        Self {
            code_theme: ThemePair::Github,
            prose_theme: ThemePair::Github,
            color_mode: ColorMode::Dark,
            code_block_mode: CodeBlockMode::default(),
            include_line_numbers: false,
            include_styles: true,
            mermaid_mode: MermaidMode::default(),
            hr_css_variables: std::collections::HashMap::new(),
            hr_defaults: None,
            hyperlink_style: None,
            local_hyperlink_style: None,
            local_image_style: None,
        }
    }
}

/// Computes the `.code-block` panel background color (`#rrggbb`) for `options`.
///
/// Code blocks contrast against the page, so the code `ThemePair` resolves
/// against the page color mode using the configured
/// [`code_block_mode`](HtmlOptions::code_block_mode) (by default `Inverse`: a
/// dark page gets a light code panel and vice versa — Defect D). The
/// render-tree browser entry point reuses this so its injected `.code-block`
/// rule matches the computed code-panel background, rather than re-deriving the
/// theme/inversion math. Routes through
/// [`ThemePair::resolve_for_surface`](crate::markdown::highlighting::themes::ThemePair::resolve_for_surface)
/// so the same boundary resolver the public `CodeBlock` and `DarkmatterPage`
/// use feeds this helper.
pub(crate) fn code_block_background_hex(options: &HtmlOptions) -> String {
    let resolved = options.code_theme.resolve_for_surface(
        crate::markdown::highlighting::Surface::Mode(options.color_mode),
        Some(options.code_theme),
        options.code_block_mode,
    );
    let highlighter = CodeHighlighter::from_theme(resolved.theme, resolved.color_mode);
    let bg = highlighter
        .theme()
        .settings
        .background
        .unwrap_or(syntect::highlighting::Color {
            r: 40,
            g: 44,
            b: 52,
            a: 255,
        });
    format!("#{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_options_default() {
        let options = HtmlOptions::default();
        assert_eq!(options.code_theme, ThemePair::Github);
        assert_eq!(options.prose_theme, ThemePair::Github);
        assert_eq!(options.color_mode, ColorMode::Dark);
        assert!(!options.include_line_numbers);
        assert!(options.include_styles);
    }
}
