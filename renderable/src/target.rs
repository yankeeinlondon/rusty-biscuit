use serde::{Deserialize, Serialize};

/// **RenderTarget**
///
/// An enumerated list of targets that a _renderable_ component might target
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderTarget {
    /// Markdown is a much more ergonomic way to write content then HTML but it looses a lot of features
    /// when it keeps it's ergonomic shape. Many people don't realize, however, that Markdown is a
    /// **superset** of HTML, not a _subset_.
    ///
    /// That means that while the way Markdown is _typically written_ does produce a a functional subset
    /// it doesn't have to be. However, as soon as you start writing lots of "inline HTML" in your Markdown
    /// you should really question why you're doing this:
    ///
    /// - you have lost the ergnomoics that Markdown authors typically desire
    /// - Markdown readers may support _some_ inline HTML features but they rarely support that well
    ///
    /// However, there **is** a use case where this makes perfect sense:
    ///
    /// - when an author uses a DSL like **Darkmatter** to write both Markdown _plus_ additional features
    ///   that ergonomic Markdown can't support, a library like Darkmatter can inject this inline HTML
    Markdown,
    /// Markdown that intentionally embeds inline HTML to express features that
    /// ergonomic Markdown cannot — the "superset" form produced by DSLs such as
    /// Darkmatter.
    MarkdownPlus,
    /// The browser is the most capable target to render to and also probably the most popular.
    ///
    /// A component which can render to a browser will need to render in two different ways:
    ///
    /// 1. `render_html()`
    ///     -
    ///
    /// 2. `render_html_components()`
    Browser,
    /// Terminal library's support styling (colors, bold fonts, etc.) through the use of escape codes
    /// and when rendering content to
    Terminal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_target_has_no_ast_and_has_markdown_plus() {
        let t = RenderTarget::MarkdownPlus;
        assert_eq!(t, RenderTarget::MarkdownPlus);
    }

    #[test]
    fn render_target_serde_roundtrip_snake_case() {
        let json = serde_json::to_string(&RenderTarget::MarkdownPlus).unwrap();
        assert_eq!(json, "\"markdown_plus\"");
        let back: RenderTarget = serde_json::from_str(&json).unwrap();
        assert_eq!(back, RenderTarget::MarkdownPlus);
    }

    #[test]
    fn render_target_is_ord() {
        let mut v = vec![RenderTarget::Terminal, RenderTarget::Browser];
        v.sort();
        assert_eq!(v, vec![RenderTarget::Browser, RenderTarget::Terminal]);
    }
}
