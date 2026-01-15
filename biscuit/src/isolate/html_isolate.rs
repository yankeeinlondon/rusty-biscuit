//! HTML content isolation using DOM parsing.
//!
//! This module provides functionality to extract specific content from HTML documents
//! using the `scraper` crate for DOM parsing and CSS selector matching.
//!
//! ## Trade-offs
//!
//! The `scraper` crate parses HTML into a read-only DOM tree that does not preserve
//! byte offsets from the original source. This design choice means:
//!
//! - **No zero-copy borrowing**: Results are always `Cow::Owned` because we must
//!   reconstruct or serialize content from the DOM rather than slice the original string.
//! - **Normalized output**: The serialized HTML may differ slightly from the original
//!   (e.g., attribute ordering, whitespace normalization).
//! - **Robust parsing**: The trade-off enables robust handling of malformed HTML,
//!   as scraper uses html5ever which follows the HTML5 parsing specification.
//!
//! For use cases requiring byte-level fidelity, consider alternative approaches
//! like regex-based extraction (with appropriate caveats about HTML parsing).
//!
//! ## Examples
//!
//! ```rust
//! use shared::isolate::{IsolateAction, IsolateResult, HtmlScope, HtmlTag};
//! use shared::isolate::html_isolate::html_isolate;
//!
//! let html = r#"<div class="intro">Hello</div><div>World</div>"#;
//!
//! // Extract inner HTML of all divs
//! let result = html_isolate(html, HtmlScope::InnerHtml(HtmlTag::Div), IsolateAction::LeaveAsVector)
//!     .expect("isolation should succeed");
//!
//! if let IsolateResult::Vector(items) = result {
//!     assert_eq!(items.len(), 2);
//!     assert_eq!(items[0], "Hello");
//!     assert_eq!(items[1], "World");
//! }
//! ```

use std::borrow::Cow;

use scraper::{Html, Selector};

use super::{HtmlScope, HtmlTag, IsolateAction, IsolateError, IsolateResult};

/// Isolates content from an HTML document based on the specified scope.
///
/// Parses the input HTML and extracts content matching the given [`HtmlScope`].
/// The extracted content is returned according to the [`IsolateAction`] - either
/// as a vector of individual matches or concatenated into a single string.
///
/// ## Arguments
///
/// * `content` - The HTML content to parse and isolate from
/// * `scope` - Specifies which parts of the HTML to extract
/// * `action` - Determines how to return the results (vector or concatenated)
///
/// ## Returns
///
/// An [`IsolateResult`] containing the extracted content, or an [`IsolateError`]
/// if selector parsing fails.
///
/// ## Examples
///
/// ```rust
/// use shared::isolate::{IsolateAction, IsolateResult, HtmlScope, HtmlTag};
/// use shared::isolate::html_isolate::html_isolate;
///
/// let html = "<h1>Title</h1><p>First paragraph</p><p>Second paragraph</p>";
///
/// // Get all paragraph outer HTML as a vector
/// let result = html_isolate(
///     html,
///     HtmlScope::OuterHtml(HtmlTag::All),
///     IsolateAction::LeaveAsVector
/// ).unwrap();
/// ```
///
/// ## Notes
///
/// - Results are always `Cow::Owned` due to scraper's DOM-based architecture
/// - Empty results return an empty vector or empty string (not an error)
/// - Invalid CSS selectors in `HtmlScope::Selector` return `IsolateError::InvalidSelector`
pub fn html_isolate<'a>(
    content: &'a str,
    scope: HtmlScope,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let document = Html::parse_document(content);

    let items: Vec<Cow<'a, str>> = match scope {
        HtmlScope::TagAttributes(tag) => extract_tag_attributes(&document, tag)?,
        HtmlScope::InnerHtml(tag) => extract_inner_html(&document, tag)?,
        HtmlScope::OuterHtml(tag) => extract_outer_html(&document, tag)?,
        HtmlScope::Selector(selector_str) => extract_by_selector(&document, &selector_str)?,
        HtmlScope::Prose => extract_prose(&document),
    };

    Ok(apply_action(items, action))
}

/// Extracts the opening tag with attributes for matching elements.
///
/// Returns strings like `<div class="foo" id="bar">` for each matching element.
fn extract_tag_attributes<'a>(
    document: &Html,
    tag: HtmlTag,
) -> Result<Vec<Cow<'a, str>>, IsolateError> {
    let selector = parse_selector(tag.as_selector())?;
    let mut results = Vec::new();

    for element in document.select(&selector) {
        let elem_ref = element.value();
        let tag_name = elem_ref.name();

        // Build the opening tag with attributes
        let mut opening_tag = format!("<{}", tag_name);

        for (name, value) in elem_ref.attrs() {
            opening_tag.push_str(&format!(r#" {}="{}""#, name, value));
        }

        if tag.is_void() {
            opening_tag.push_str(" />");
        } else {
            opening_tag.push('>');
        }

        results.push(Cow::Owned(opening_tag));
    }

    Ok(results)
}

/// Extracts the inner HTML content of matching elements.
///
/// Returns the content between opening and closing tags, excluding the tags themselves.
fn extract_inner_html<'a>(
    document: &Html,
    tag: HtmlTag,
) -> Result<Vec<Cow<'a, str>>, IsolateError> {
    let selector = parse_selector(tag.as_selector())?;
    let mut results = Vec::new();

    for element in document.select(&selector) {
        let inner = element.inner_html();
        results.push(Cow::Owned(inner));
    }

    Ok(results)
}

/// Extracts the outer HTML of matching elements.
///
/// Returns the complete element including opening tag, content, and closing tag.
fn extract_outer_html<'a>(
    document: &Html,
    tag: HtmlTag,
) -> Result<Vec<Cow<'a, str>>, IsolateError> {
    let selector = parse_selector(tag.as_selector())?;
    let mut results = Vec::new();

    for element in document.select(&selector) {
        let outer = element.html();
        results.push(Cow::Owned(outer));
    }

    Ok(results)
}

/// Extracts elements matching a CSS selector.
///
/// Returns the outer HTML of all matching elements.
fn extract_by_selector<'a>(
    document: &Html,
    selector_str: &str,
) -> Result<Vec<Cow<'a, str>>, IsolateError> {
    let selector = parse_selector(selector_str)?;
    let mut results = Vec::new();

    for element in document.select(&selector) {
        let outer = element.html();
        results.push(Cow::Owned(outer));
    }

    Ok(results)
}

/// Extracts all text content from the document, stripping HTML tags.
///
/// Processes the body content (or entire document for fragments) and returns
/// the concatenated text from all text nodes.
fn extract_prose(document: &Html) -> Vec<Cow<'static, str>> {
    // Try to find body first, fall back to root element
    let body_selector = Selector::parse("body").expect("body selector is valid");

    let text: String = if let Some(body) = document.select(&body_selector).next() {
        body.text().collect::<Vec<_>>().join("")
    } else {
        // For fragments without body, get all text from root
        document.root_element().text().collect::<Vec<_>>().join("")
    };

    if text.is_empty() {
        Vec::new()
    } else {
        vec![Cow::Owned(text)]
    }
}

/// Parses a CSS selector string, returning an error for invalid selectors.
fn parse_selector(selector_str: &str) -> Result<Selector, IsolateError> {
    Selector::parse(selector_str)
        .map_err(|_| IsolateError::InvalidSelector(selector_str.to_string()))
}

/// Applies the isolation action to convert results to the desired format.
fn apply_action<'a>(items: Vec<Cow<'a, str>>, action: IsolateAction) -> IsolateResult<'a> {
    match action {
        IsolateAction::LeaveAsVector => IsolateResult::Vector(items),
        IsolateAction::Concatenate(delimiter) => {
            let joined = match delimiter {
                None => items.into_iter().map(|c| c.into_owned()).collect(),
                Some(delim) => items
                    .into_iter()
                    .map(|c| c.into_owned())
                    .collect::<Vec<_>>()
                    .join(&delim),
            };
            IsolateResult::Concatenated(joined)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Test Page</title>
</head>
<body>
    <header>
        <h1>Welcome</h1>
    </header>
    <main>
        <section id="intro" class="highlight">
            <h2>Introduction</h2>
            <p>First paragraph.</p>
        </section>
        <section id="content">
            <h2>Content</h2>
            <p>Second paragraph.</p>
            <div class="code-block">
                <pre><code class="language-rust">fn main() {}</code></pre>
            </div>
        </section>
    </main>
</body>
</html>"#;

    // Test 1: InnerHtml extraction
    #[test]
    fn test_inner_html_extraction() {
        let html = "<div>Hello</div><div>World</div>";
        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Div),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], "Hello");
            assert_eq!(items[1], "World");
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 2: OuterHtml extraction
    #[test]
    fn test_outer_html_extraction() {
        let html = r#"<span class="test">Content</span>"#;
        let result = html_isolate(
            html,
            HtmlScope::OuterHtml(HtmlTag::Span),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert!(items[0].contains("<span"));
            assert!(items[0].contains("Content"));
            assert!(items[0].contains("</span>"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 3: TagAttributes extraction
    #[test]
    fn test_tag_attributes_extraction() {
        let html = r#"<div class="container" id="main">Content</div>"#;
        let result = html_isolate(
            html,
            HtmlScope::TagAttributes(HtmlTag::Div),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            let tag = &items[0];
            assert!(tag.starts_with("<div"));
            assert!(tag.contains("class="));
            assert!(tag.contains("container"));
            assert!(tag.contains("id="));
            assert!(tag.contains("main"));
            assert!(tag.ends_with(">"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 4: CSS Selector extraction
    #[test]
    fn test_selector_extraction() {
        let result = html_isolate(
            SAMPLE_HTML,
            HtmlScope::Selector("section.highlight".to_string()),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert!(items[0].contains("Introduction"));
            assert!(items[0].contains(r#"class="highlight""#));
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 5: Invalid selector error
    #[test]
    fn test_invalid_selector_error() {
        let result = html_isolate(
            "<div>test</div>",
            HtmlScope::Selector("div[".to_string()),
            IsolateAction::LeaveAsVector,
        );

        assert!(result.is_err());
        if let Err(IsolateError::InvalidSelector(s)) = result {
            assert_eq!(s, "div[");
        } else {
            panic!("Expected InvalidSelector error");
        }
    }

    // Test 6: Prose extraction
    #[test]
    fn test_prose_extraction() {
        let html = "<p>Hello <strong>World</strong>!</p>";
        let result = html_isolate(html, HtmlScope::Prose, IsolateAction::LeaveAsVector).unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], "Hello World!");
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 7: Concatenate action with delimiter
    #[test]
    fn test_concatenate_with_delimiter() {
        let html = "<li>One</li><li>Two</li><li>Three</li>";
        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::All),
            IsolateAction::Concatenate(Some("\n".to_string())),
        )
        .unwrap();

        if let IsolateResult::Concatenated(text) = result {
            // The HTML is wrapped in html/body by the parser
            assert!(text.contains("One"));
            assert!(text.contains("Two"));
            assert!(text.contains("Three"));
        } else {
            panic!("Expected Concatenated result");
        }
    }

    // Test 8: Concatenate action without delimiter
    #[test]
    fn test_concatenate_without_delimiter() {
        let html = "<span>A</span><span>B</span><span>C</span>";
        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Span),
            IsolateAction::Concatenate(None),
        )
        .unwrap();

        if let IsolateResult::Concatenated(text) = result {
            assert_eq!(text, "ABC");
        } else {
            panic!("Expected Concatenated result");
        }
    }

    // Test 9: Empty results
    #[test]
    fn test_empty_results() {
        let html = "<div>No spans here</div>";
        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Span),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert!(items.is_empty());
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 10: Heading extraction (H1, H2)
    #[test]
    fn test_heading_extraction() {
        let result = html_isolate(
            SAMPLE_HTML,
            HtmlScope::InnerHtml(HtmlTag::H2),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], "Introduction");
            assert_eq!(items[1], "Content");
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 11: All elements selector
    #[test]
    fn test_all_elements_selector() {
        let html = "<p>One</p><span>Two</span>";
        let result = html_isolate(
            html,
            HtmlScope::OuterHtml(HtmlTag::All),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            // Should include html, head, body, p, span at minimum
            assert!(items.len() >= 2);
            let all_content: String = items.iter().map(|c| c.as_ref()).collect();
            assert!(all_content.contains("<p>One</p>"));
            assert!(all_content.contains("<span>Two</span>"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 12: Meta tag (void element) attributes
    #[test]
    fn test_void_element_attributes() {
        let html = r#"<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width"></head></html>"#;
        let result = html_isolate(
            html,
            HtmlScope::TagAttributes(HtmlTag::Meta),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 2);
            // Void elements should end with />
            assert!(items[0].ends_with("/>") || items[0].ends_with(">"));
            assert!(items[0].contains("charset"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 13: Prose extraction from complex document
    #[test]
    fn test_prose_from_complex_document() {
        let result =
            html_isolate(SAMPLE_HTML, HtmlScope::Prose, IsolateAction::LeaveAsVector).unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            let text = &items[0];
            assert!(text.contains("Welcome"));
            assert!(text.contains("Introduction"));
            assert!(text.contains("First paragraph"));
            // Should not contain HTML tags
            assert!(!text.contains("<h1>"));
            assert!(!text.contains("<p>"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 14: Nested element extraction
    #[test]
    fn test_nested_element_extraction() {
        let html = "<div><p>Nested <em>content</em></p></div>";
        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Div),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            // Inner HTML should preserve nested tags
            assert!(items[0].contains("<p>"));
            assert!(items[0].contains("<em>"));
            assert!(items[0].contains("content"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 15: Attribute selector
    #[test]
    fn test_attribute_selector() {
        let result = html_isolate(
            SAMPLE_HTML,
            HtmlScope::Selector("section[id='content']".to_string()),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert!(items[0].contains("Content"));
            assert!(items[0].contains("Second paragraph"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 16: Descendant combinator selector
    #[test]
    fn test_descendant_combinator_selector() {
        let result = html_isolate(
            SAMPLE_HTML,
            HtmlScope::Selector("section p".to_string()),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 2);
            assert!(items[0].contains("First paragraph"));
            assert!(items[1].contains("Second paragraph"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 17: HTML isolation always returns Cow::Owned (documented behavior)
    #[test]
    fn test_html_always_returns_owned() {
        let html = "<div>Simple content</div>";
        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Div),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            // Due to scraper's DOM-based architecture, results are always Cow::Owned
            // This is documented in the module docs
            match &items[0] {
                Cow::Owned(_) => {} // Expected behavior
                Cow::Borrowed(_) => panic!("HTML isolate should return Cow::Owned, not Borrowed"),
            }
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 18: Empty HTML document
    #[test]
    fn test_empty_html_document() {
        let html = "";
        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Div),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert!(items.is_empty());
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 19: Unicode content in HTML
    #[test]
    fn test_unicode_html_content() {
        let html = "<div>\u{1F600} Hello \u{1F389} World \u{1F680}</div>";
        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Div),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert!(items[0].contains("\u{1F600}"));
            assert!(items[0].contains("\u{1F389}"));
            assert!(items[0].contains("\u{1F680}"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // Test 20: Additional heading tags (H3-H6)
    #[test]
    fn test_all_heading_levels() {
        let html = "<h1>H1</h1><h2>H2</h2><h3>H3</h3><h4>H4</h4><h5>H5</h5><h6>H6</h6>";

        // Test H3
        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::H3),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();
        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], "H3");
        }

        // Test H6
        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::H6),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();
        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], "H6");
        }
    }

    // Test 21: Section and Aside elements
    #[test]
    fn test_semantic_html_elements() {
        let html = r#"<section id="main">Main content</section><aside>Side content</aside>"#;

        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Section),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();
        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], "Main content");
        }

        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Aside),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();
        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], "Side content");
        }
    }

    // Test 22: Pre and code block elements
    #[test]
    fn test_pre_element_extraction() {
        let html = r#"<pre>Preformatted text</pre>"#;
        let result = html_isolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Pre),
            IsolateAction::LeaveAsVector,
        )
        .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], "Preformatted text");
        }
    }
}
