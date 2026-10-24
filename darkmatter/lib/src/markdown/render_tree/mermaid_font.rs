//! The page-level embedded Mermaid font ([`HtmlOptions::embed_mermaid_font`]).
//!
//! Each static diagram is rendered on its own, but the font covers the whole
//! page: one `@font-face` rule per face and weight, subset to the union of
//! every diagram's characters. The page is streamed before that union is
//! known, so the first opt-in diagram writes [`PAGE_FONT_PLACEHOLDER`] ahead
//! of its SVG, and the entry point replaces it once the page is rendered
//! ([`place_page_font`]).
//!
//! [`HtmlOptions::embed_mermaid_font`]: crate::markdown::output::HtmlOptions::embed_mermaid_font

use biscuit_visualized::mermaid::{DiagramFont, FontEmbeddingError, PageFont, page_font};

use super::svg_sanitizer::style_sheet_is_safe;

/// Marks where the page font goes. Only Darkmatter's own diagram fragment
/// writes it: these pages escape the author's raw HTML, and the sanitized
/// SVG keeps no comments, so nothing else on the page can spell it.
pub(crate) const PAGE_FONT_PLACEHOLDER: &str = "<!--darkmatter:mermaid-font-->";

/// The page's font markup for `diagrams`: the per-family comments followed
/// by the `<style>` of `@font-face` rules, or nothing when the font cannot be
/// built or its CSS fails the sanitizer's style-sheet check.
pub(crate) fn page_font_markup(diagrams: &[DiagramFont]) -> String {
    checked_page_font(page_font(diagrams))
}

/// [`page_font_markup`] from an already built (or failed) page font.
fn checked_page_font(page: Result<PageFont, FontEmbeddingError>) -> String {
    // On `Err` biscuit-visualized has already logged the warning.
    let Ok(page) = page else {
        return String::new();
    };
    checked_markup(page.markup(), page.css())
}

/// `markup` when `css`, the content of its `<style>`, passes the same check
/// the sanitizer applies to a diagram's `<style>`; otherwise nothing, with a
/// warning.
fn checked_markup(markup: String, css: &str) -> String {
    if style_sheet_is_safe(css) {
        markup
    } else {
        tracing::warn!("the embedded Mermaid font failed the style-sheet check; the diagrams are shown without it");
        String::new()
    }
}

/// `html` with its page-font placeholder replaced by `markup`.
pub(crate) fn place_page_font(html: &str, markup: &str) -> String {
    html.replacen(PAGE_FONT_PLACEHOLDER, markup, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_page_font_emits_nothing() {
        let failed = Err(FontEmbeddingError::FaceData {
            family: "Inter".to_string(),
            weight: 400,
        });
        assert_eq!(checked_page_font(failed), "");
        assert_eq!(checked_page_font(Ok(PageFont::default())), "");
    }

    #[test]
    #[tracing_test::traced_test]
    fn css_that_fails_the_style_sheet_check_is_not_emitted() {
        let markup = "<!-- c --><style>x</style>".to_string();
        let external = "@font-face{src:url(https://attacker.example/f.ttf) format(\"truetype\")}";
        assert_eq!(checked_markup(markup.clone(), external), "");
        assert!(logs_contain("failed the style-sheet check"));
        assert_eq!(checked_markup(markup.clone(), "rect{fill:url(#g)}"), markup);
    }

    #[test]
    fn the_placeholder_is_replaced_once() {
        let html = format!("<p>a</p>{PAGE_FONT_PLACEHOLDER}<svg/>");
        assert_eq!(place_page_font(&html, "<style>s</style>"), "<p>a</p><style>s</style><svg/>");
        assert_eq!(place_page_font(&html, ""), "<p>a</p><svg/>");
        assert_eq!(place_page_font("<svg/>", "<style>s</style>"), "<svg/>");
    }
}
