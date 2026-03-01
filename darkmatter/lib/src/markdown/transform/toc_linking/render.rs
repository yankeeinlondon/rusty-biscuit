//! Link list rendering for toc-linking output.

use super::cleanup::apply_cleanup;
use super::filter::HeadingFilter;
use super::types::TocLinkingOptions;
use crate::markdown::toc::MarkdownTocNode;

/// Renders a list of markdown links from TOC headings.
///
/// Returns the replacement text for the directive. Returns the `empty_text`
/// (or empty string) when no headings match.
pub fn render_toc_links(
    headings: &[&MarkdownTocNode],
    file_path: &str,
    options: &TocLinkingOptions,
    filter: &HeadingFilter,
) -> String {
    let links: Vec<String> = headings
        .iter()
        .filter(|h| options.levels.includes(h.level))
        .filter(|h| filter.should_include(&h.title))
        .map(|h| {
            let display = if options.cleanup_services.is_empty() {
                h.title.clone()
            } else {
                apply_cleanup(&h.title, &options.cleanup_services)
            };
            format!("- [{}]({}#{})", display, file_path, h.slug)
        })
        .collect();

    if links.is_empty() {
        options.empty_text.clone().unwrap_or_default()
    } else {
        links.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::toc::MarkdownTocNode;
    use crate::markdown::transform::toc_linking::types::CleanupService;

    fn make_heading(level: u8, title: &str, slug: &str) -> MarkdownTocNode {
        MarkdownTocNode::new(level, title.to_string(), slug.to_string(), (0, 0), (0, 0))
    }

    fn default_filter() -> HeadingFilter {
        HeadingFilter::new(&[], &[], 1).unwrap()
    }

    #[test]
    fn basic_link_generation() {
        let h1 = make_heading(2, "Getting Started", "getting-started");
        let h2 = make_heading(2, "Usage", "usage");
        let headings: Vec<&MarkdownTocNode> = vec![&h1, &h2];
        let options = TocLinkingOptions::default();

        let result = render_toc_links(&headings, "./doc.md", &options, &default_filter());
        assert_eq!(
            result,
            "- [Getting Started](./doc.md#getting-started)\n- [Usage](./doc.md#usage)"
        );
    }

    #[test]
    fn level_filtering() {
        let h2 = make_heading(2, "Section", "section");
        let h3 = make_heading(3, "Subsection", "subsection");
        let headings: Vec<&MarkdownTocNode> = vec![&h2, &h3];

        let mut options = TocLinkingOptions::default();
        options.levels.levels.insert(2);

        let result = render_toc_links(&headings, "./doc.md", &options, &default_filter());
        assert_eq!(result, "- [Section](./doc.md#section)");
    }

    #[test]
    fn empty_result_returns_empty_string() {
        let headings: Vec<&MarkdownTocNode> = vec![];
        let options = TocLinkingOptions::default();

        let result = render_toc_links(&headings, "./doc.md", &options, &default_filter());
        assert_eq!(result, "");
    }

    #[test]
    fn empty_result_with_text() {
        let headings: Vec<&MarkdownTocNode> = vec![];
        let mut options = TocLinkingOptions::default();
        options.empty_text = Some("no results".to_string());

        let result = render_toc_links(&headings, "./doc.md", &options, &default_filter());
        assert_eq!(result, "no results");
    }

    #[test]
    fn cleanup_applied_to_display_text() {
        let h = make_heading(2, "🚀 Getting Started", "getting-started");
        let headings: Vec<&MarkdownTocNode> = vec![&h];

        let mut options = TocLinkingOptions::default();
        options.cleanup_services = vec![CleanupService::EmojiLeader];

        let result = render_toc_links(&headings, "./doc.md", &options, &default_filter());
        // Display text has emoji stripped, but slug is unchanged
        assert_eq!(result, "- [Getting Started](./doc.md#getting-started)");
    }

    #[test]
    fn file_path_preserved() {
        let h = make_heading(2, "Test", "test");
        let headings: Vec<&MarkdownTocNode> = vec![&h];
        let options = TocLinkingOptions::default();

        let result = render_toc_links(
            &headings,
            "../relative/path.md",
            &options,
            &default_filter(),
        );
        assert!(result.contains("../relative/path.md#test"));
    }
}
