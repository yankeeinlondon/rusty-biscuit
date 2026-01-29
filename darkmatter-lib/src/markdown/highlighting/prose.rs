//! Prose highlighting for markdown documents.
//!
//! Provides a highlighter for prose elements (headings, bold, italic, links, etc.)
//! using syntect themes. Uses a functional style - no ScopeStack mutation,
//! styles are computed on-demand from scope slices.

use super::scope_cache::ScopeCache;
use pulldown_cmark::Tag;
use syntect::highlighting::{Highlighter, Style, Theme as SyntectTheme};
use syntect::parsing::Scope;

/// Highlighter for prose elements in markdown.
///
/// Uses a functional style where styles are computed on-demand without mutating
/// any internal state. Parent scopes are passed as slices for each style computation.
///
/// ## Examples
///
/// ```
/// use darkmatter_lib::markdown::highlighting::{ThemePair, ColorMode};
/// use darkmatter_lib::markdown::highlighting::prose::ProseHighlighter;
///
/// let theme = ThemePair::Github;
/// let color_mode = ColorMode::Dark;
/// // Load theme using internal API
/// // let highlighter = ProseHighlighter::new(&syntect_theme);
/// ```
pub struct ProseHighlighter<'a> {
    highlighter: Highlighter<'a>,
    scope_cache: &'static ScopeCache,
}

impl<'a> ProseHighlighter<'a> {
    /// Creates a new prose highlighter with the given theme.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use darkmatter_lib::markdown::highlighting::prose::ProseHighlighter;
    ///
    /// let highlighter = ProseHighlighter::new(&syntect_theme);
    /// ```
    pub fn new(theme: &'a SyntectTheme) -> Self {
        Self {
            highlighter: Highlighter::new(theme),
            scope_cache: ScopeCache::global(),
        }
    }

    /// Returns the style for a tag, given the current parent scope stack.
    ///
    /// Uses functional style - does not mutate any state. The parent scopes
    /// are passed as a slice and a new scope stack is built for each call.
    ///
    /// ## Arguments
    ///
    /// * `tag` - The pulldown_cmark tag to get the style for
    /// * `parent_scopes` - The current stack of parent scopes
    ///
    /// ## Returns
    ///
    /// The computed style for the given tag and scope context.
    pub fn style_for_tag(&self, tag: &Tag, parent_scopes: &[Scope]) -> Style {
        let mut stack_vec: Vec<Scope> = parent_scopes.to_vec();

        if let Some(scope) = self.scope_cache.scope_for_tag(tag) {
            stack_vec.push(scope);
        }

        self.highlighter.style_for_stack(&stack_vec)
    }

    /// Returns the base style (no additional scopes).
    ///
    /// This is the default style used when no specific tag styling applies.
    pub fn base_style(&self) -> Style {
        self.highlighter.style_for_stack(&[self.scope_cache.base])
    }

    /// Returns the base scope for markdown documents.
    pub fn base_scope(&self) -> Scope {
        self.scope_cache.base
    }

    /// Returns the scope for inline code.
    pub fn code_inline_scope(&self) -> Scope {
        self.scope_cache.code_inline
    }

    /// Returns the style for inline code.
    ///
    /// ## Arguments
    ///
    /// * `parent_scopes` - The current stack of parent scopes
    pub fn style_for_inline_code(&self, parent_scopes: &[Scope]) -> Style {
        let mut stack_vec: Vec<Scope> = parent_scopes.to_vec();
        stack_vec.push(self.scope_cache.code_inline);
        self.highlighter.style_for_stack(&stack_vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::highlighting::{ColorMode, ThemePair};
    use pulldown_cmark::HeadingLevel;

    fn load_test_theme() -> SyntectTheme {
        crate::markdown::highlighting::themes::load_theme(ThemePair::Github, ColorMode::Dark)
    }

    #[test]
    fn test_prose_highlighter_new() {
        let theme = load_test_theme();
        let _highlighter = ProseHighlighter::new(&theme);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_base_style() {
        let theme = load_test_theme();
        let highlighter = ProseHighlighter::new(&theme);

        let style = highlighter.base_style();
        // Style should have foreground color
        assert!(style.foreground.r > 0 || style.foreground.g > 0 || style.foreground.b > 0);
    }

    #[test]
    fn test_base_scope() {
        let theme = load_test_theme();
        let highlighter = ProseHighlighter::new(&theme);

        let scope = highlighter.base_scope();
        assert_eq!(scope.to_string(), "text.html.markdown");
    }

    #[test]
    fn test_style_for_tag_heading() {
        let theme = load_test_theme();
        let highlighter = ProseHighlighter::new(&theme);

        let tag = Tag::Heading {
            level: HeadingLevel::H1,
            id: None,
            classes: vec![],
            attrs: vec![],
        };
        let style = highlighter.style_for_tag(&tag, &[highlighter.base_scope()]);

        // Style should have some properties set
        assert!(style.foreground.a > 0);
    }

    #[test]
    fn test_style_for_tag_strong() {
        let theme = load_test_theme();
        let highlighter = ProseHighlighter::new(&theme);

        let tag = Tag::Strong;
        let base = highlighter.base_scope();
        let style = highlighter.style_for_tag(&tag, &[base]);

        // Bold should have font_style set (if theme supports it)
        // At minimum, it should return a valid style
        assert!(style.foreground.a > 0);
    }

    #[test]
    fn test_style_for_tag_emphasis() {
        let theme = load_test_theme();
        let highlighter = ProseHighlighter::new(&theme);

        let tag = Tag::Emphasis;
        let style = highlighter.style_for_tag(&tag, &[highlighter.base_scope()]);

        assert!(style.foreground.a > 0);
    }

    #[test]
    fn test_style_for_tag_nested_scopes() {
        let theme = load_test_theme();
        let highlighter = ProseHighlighter::new(&theme);

        // Simulate nested context: base -> heading -> strong
        let base = highlighter.base_scope();
        let heading_tag = Tag::Heading {
            level: HeadingLevel::H1,
            id: None,
            classes: vec![],
            attrs: vec![],
        };
        let heading_scope = ScopeCache::global().scope_for_tag(&heading_tag).unwrap();

        let strong_tag = Tag::Strong;
        let style = highlighter.style_for_tag(&strong_tag, &[base, heading_scope]);

        assert!(style.foreground.a > 0);
    }

    #[test]
    fn test_style_for_inline_code() {
        let theme = load_test_theme();
        let highlighter = ProseHighlighter::new(&theme);

        let style = highlighter.style_for_inline_code(&[highlighter.base_scope()]);
        assert!(style.foreground.a > 0);
    }

    #[test]
    fn test_code_inline_scope() {
        let theme = load_test_theme();
        let highlighter = ProseHighlighter::new(&theme);

        let scope = highlighter.code_inline_scope();
        assert_eq!(scope.to_string(), "markup.raw.inline.markdown");
    }

    #[test]
    fn test_functional_style_no_mutation() {
        let theme = load_test_theme();
        let highlighter = ProseHighlighter::new(&theme);

        // Call style_for_tag multiple times with same inputs
        let tag = Tag::Strong;
        let scopes = vec![highlighter.base_scope()];

        let style1 = highlighter.style_for_tag(&tag, &scopes);
        let style2 = highlighter.style_for_tag(&tag, &scopes);

        // Should return identical results (no state mutation)
        assert_eq!(style1.foreground, style2.foreground);
        assert_eq!(style1.background, style2.background);
        assert_eq!(style1.font_style, style2.font_style);
    }

    #[test]
    fn test_initialization_performance() {
        use std::time::Instant;

        let theme = load_test_theme();

        let start = Instant::now();
        let _highlighter = ProseHighlighter::new(&theme);
        let duration = start.elapsed();

        // Initialization should be fast (< 1ms)
        assert!(
            duration.as_millis() < 1,
            "Initialization took {}ms",
            duration.as_millis()
        );
    }

    #[test]
    fn test_style_for_tag_link() {
        // Test that link styling is returned correctly from the highlighter.
        // Note: Whether the theme provides a distinct link color depends on the theme.
        // Github theme has a distinct link color, OneHalf doesn't.
        let theme = load_test_theme(); // Github theme
        let highlighter = ProseHighlighter::new(&theme);

        let link_tag = Tag::Link {
            link_type: pulldown_cmark::LinkType::Inline,
            dest_url: "".into(),
            title: "".into(),
            id: "".into(),
        };
        let style = highlighter.style_for_tag(&link_tag, &[highlighter.base_scope()]);

        // Style should have some foreground color
        assert!(
            style.foreground.a > 0,
            "Link style should have foreground color"
        );
    }

    #[test]
    fn test_link_style_varies_by_theme() {
        // Different themes may or may not define link-specific colors.
        // This test documents the behavior for themes used in the terminal output.

        // Github theme has a distinct link color
        let github_theme =
            crate::markdown::highlighting::themes::load_theme(ThemePair::Github, ColorMode::Dark);
        let github_highlighter = ProseHighlighter::new(&github_theme);
        let github_base = github_highlighter.base_style();
        let link_tag = Tag::Link {
            link_type: pulldown_cmark::LinkType::Inline,
            dest_url: "".into(),
            title: "".into(),
            id: "".into(),
        };
        let github_link =
            github_highlighter.style_for_tag(&link_tag, &[github_highlighter.base_scope()]);

        // Github theme should have different colors for base and link
        assert_ne!(
            github_base.foreground, github_link.foreground,
            "Github theme should have distinct link color"
        );

        // OneHalf theme does NOT have a distinct link color
        let onehalf_theme =
            crate::markdown::highlighting::themes::load_theme(ThemePair::OneHalf, ColorMode::Dark);
        let onehalf_highlighter = ProseHighlighter::new(&onehalf_theme);
        let onehalf_base = onehalf_highlighter.base_style();
        let onehalf_link =
            onehalf_highlighter.style_for_tag(&link_tag, &[onehalf_highlighter.base_scope()]);

        // OneHalf theme has the same color for base and link
        // (this is why we need fallback styling in terminal.rs)
        assert_eq!(
            onehalf_base.foreground, onehalf_link.foreground,
            "OneHalf theme should NOT have distinct link color (requires fallback in renderer)"
        );
    }
}
