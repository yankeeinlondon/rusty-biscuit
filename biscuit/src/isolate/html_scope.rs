//! HTML scoping types for isolate operations.
//!
//! This module provides enums for specifying which parts of an HTML document
//! should be isolated for find/replace or extraction operations.

/// HTML tag types for scoping isolate operations.
///
/// Used to target specific HTML elements when performing isolation.
/// The variants cover common structural and semantic HTML elements.
///
/// ## Examples
///
/// ```rust
/// use shared::isolate::html_scope::{HtmlTag, HtmlScope};
///
/// // Target all div elements
/// let scope = HtmlScope::InnerHtml(HtmlTag::Div);
///
/// // Target heading elements
/// let h1_scope = HtmlScope::OuterHtml(HtmlTag::H1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HtmlTag {
    /// Match all elements in the document.
    ///
    /// Equivalent to the CSS universal selector `*`.
    All,

    /// The `<body>` element.
    ///
    /// Contains the displayable content of the HTML document.
    Body,

    /// The `<head>` element.
    ///
    /// Contains metadata, links, scripts, and other non-displayed content.
    Head,

    /// The `<div>` element.
    ///
    /// Generic container for flow content, commonly used for layout.
    Div,

    /// The `<span>` element.
    ///
    /// Generic inline container for phrasing content.
    Span,

    /// The `<section>` element.
    ///
    /// Represents a standalone section of content, typically with a heading.
    Section,

    /// The `<aside>` element.
    ///
    /// Represents content tangentially related to the main content,
    /// such as sidebars or pull quotes.
    Aside,

    /// The `<header>` element.
    ///
    /// Represents introductory content or navigational aids,
    /// typically containing headings, logos, or navigation.
    Header,

    /// The `<h1>` element.
    ///
    /// Top-level heading, representing the most important heading.
    H1,

    /// The `<h2>` element.
    ///
    /// Second-level heading.
    H2,

    /// The `<h3>` element.
    ///
    /// Third-level heading.
    H3,

    /// The `<h4>` element.
    ///
    /// Fourth-level heading.
    H4,

    /// The `<h5>` element.
    ///
    /// Fifth-level heading.
    H5,

    /// The `<h6>` element.
    ///
    /// Sixth-level heading, representing the least important heading.
    H6,

    /// The `<meta>` element.
    ///
    /// Represents metadata that cannot be represented by other meta elements
    /// like `<title>`, `<link>`, `<script>`, or `<style>`.
    Meta,

    /// The `<script>` element.
    ///
    /// Contains executable code or references to external scripts.
    Script,

    /// The `<pre>` element.
    ///
    /// Represents preformatted text, preserving whitespace and line breaks.
    Pre,

    /// A `<pre>` element containing block-level code.
    ///
    /// Typically used for multi-line code examples with syntax highlighting.
    /// Detected by the presence of a `<code>` child with a language class.
    PreBlock,

    /// A `<pre>` element containing inline code.
    ///
    /// Used for short code snippets that don't require block formatting.
    /// Distinguished from `PreBlock` by lack of language class or structure.
    PreInline,
}

impl HtmlTag {
    /// Returns the CSS selector string for this tag type.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use shared::isolate::html_scope::HtmlTag;
    ///
    /// assert_eq!(HtmlTag::Div.as_selector(), "div");
    /// assert_eq!(HtmlTag::All.as_selector(), "*");
    /// ```
    pub fn as_selector(&self) -> &'static str {
        match self {
            Self::All => "*",
            Self::Body => "body",
            Self::Head => "head",
            Self::Div => "div",
            Self::Span => "span",
            Self::Section => "section",
            Self::Aside => "aside",
            Self::Header => "header",
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::H4 => "h4",
            Self::H5 => "h5",
            Self::H6 => "h6",
            Self::Meta => "meta",
            Self::Script => "script",
            Self::Pre => "pre",
            Self::PreBlock => "pre:has(code[class*='language-'])",
            Self::PreInline => "pre:not(:has(code[class*='language-']))",
        }
    }

    /// Returns whether this tag type represents a void element (self-closing).
    ///
    /// Void elements cannot have child content and do not have closing tags.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use shared::isolate::html_scope::HtmlTag;
    ///
    /// assert!(HtmlTag::Meta.is_void());
    /// assert!(!HtmlTag::Div.is_void());
    /// ```
    pub fn is_void(&self) -> bool {
        matches!(self, Self::Meta)
    }
}

/// Scope for HTML isolation operations.
///
/// Defines what part of matching HTML elements should be isolated
/// for find/replace or extraction operations.
///
/// ## Examples
///
/// ```rust
/// use shared::isolate::html_scope::{HtmlScope, HtmlTag};
///
/// // Isolate just the text content from body
/// let prose_scope = HtmlScope::Prose;
///
/// // Isolate inner HTML of all section elements
/// let section_scope = HtmlScope::InnerHtml(HtmlTag::Section);
///
/// // Use a custom CSS selector
/// let custom_scope = HtmlScope::Selector("article > p.intro".to_string());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HtmlScope {
    /// Isolate tag attributes for a specific tag type.
    ///
    /// Allows modification of the tag name and all attributes.
    /// If the tag name is changed, both opening and closing tags
    /// are updated (for non-void elements).
    ///
    /// ## Returns
    ///
    /// The opening tag including its attributes, e.g., `<div class="foo" id="bar">`.
    TagAttributes(HtmlTag),

    /// Isolate the inner HTML content of matching tags.
    ///
    /// Returns the content between the opening and closing tags,
    /// excluding the tags themselves.
    ///
    /// ## Returns
    ///
    /// The inner HTML content, preserving nested elements and text.
    InnerHtml(HtmlTag),

    /// Isolate the outer HTML including the tags themselves.
    ///
    /// Returns the complete element including opening tag, content,
    /// and closing tag.
    ///
    /// ## Returns
    ///
    /// The complete outer HTML of matching elements.
    OuterHtml(HtmlTag),

    /// Isolate elements matching a CSS selector.
    ///
    /// Uses the `scraper` crate's CSS selector syntax, which supports
    /// most CSS3 selectors including combinators, pseudo-classes,
    /// and attribute selectors.
    ///
    /// ## Returns
    ///
    /// The outer HTML of elements matching the selector.
    ///
    /// ## Notes
    ///
    /// Invalid selectors will result in an error during isolation.
    Selector(String),

    /// Isolate text content only, excluding all tags.
    ///
    /// Returns the concatenated text content from the document body,
    /// stripping all HTML tags. For fragments without an `<html>` root,
    /// processes the entire content as body.
    ///
    /// ## Returns
    ///
    /// Plain text content with tags removed.
    ///
    /// ## Notes
    ///
    /// Whitespace is preserved as it appears in the original text nodes.
    Prose,
}

impl HtmlScope {
    /// Creates a new `InnerHtml` scope for the given tag.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use shared::isolate::html_scope::{HtmlScope, HtmlTag};
    ///
    /// let scope = HtmlScope::inner(HtmlTag::Div);
    /// assert_eq!(scope, HtmlScope::InnerHtml(HtmlTag::Div));
    /// ```
    pub fn inner(tag: HtmlTag) -> Self {
        Self::InnerHtml(tag)
    }

    /// Creates a new `OuterHtml` scope for the given tag.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use shared::isolate::html_scope::{HtmlScope, HtmlTag};
    ///
    /// let scope = HtmlScope::outer(HtmlTag::Section);
    /// assert_eq!(scope, HtmlScope::OuterHtml(HtmlTag::Section));
    /// ```
    pub fn outer(tag: HtmlTag) -> Self {
        Self::OuterHtml(tag)
    }

    /// Creates a new `Selector` scope with the given CSS selector.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use shared::isolate::html_scope::HtmlScope;
    ///
    /// let scope = HtmlScope::selector("nav ul > li.active");
    /// ```
    pub fn selector(selector: impl Into<String>) -> Self {
        Self::Selector(selector.into())
    }

    /// Creates a new `TagAttributes` scope for the given tag.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use shared::isolate::html_scope::{HtmlScope, HtmlTag};
    ///
    /// let scope = HtmlScope::attributes(HtmlTag::Meta);
    /// ```
    pub fn attributes(tag: HtmlTag) -> Self {
        Self::TagAttributes(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_tag_selectors() {
        assert_eq!(HtmlTag::All.as_selector(), "*");
        assert_eq!(HtmlTag::Body.as_selector(), "body");
        assert_eq!(HtmlTag::Div.as_selector(), "div");
        assert_eq!(HtmlTag::H1.as_selector(), "h1");
        assert_eq!(
            HtmlTag::PreBlock.as_selector(),
            "pre:has(code[class*='language-'])"
        );
    }

    #[test]
    fn html_tag_void_elements() {
        assert!(HtmlTag::Meta.is_void());
        assert!(!HtmlTag::Div.is_void());
        assert!(!HtmlTag::Body.is_void());
        assert!(!HtmlTag::Pre.is_void());
    }

    #[test]
    fn html_scope_constructors() {
        assert_eq!(
            HtmlScope::inner(HtmlTag::Div),
            HtmlScope::InnerHtml(HtmlTag::Div)
        );
        assert_eq!(
            HtmlScope::outer(HtmlTag::Section),
            HtmlScope::OuterHtml(HtmlTag::Section)
        );
        assert_eq!(
            HtmlScope::selector("p.intro"),
            HtmlScope::Selector("p.intro".to_string())
        );
        assert_eq!(
            HtmlScope::attributes(HtmlTag::Meta),
            HtmlScope::TagAttributes(HtmlTag::Meta)
        );
    }

    #[test]
    fn html_tag_derives() {
        // Test Clone
        let tag = HtmlTag::Div;
        let cloned = tag.clone();
        assert_eq!(tag, cloned);

        // Test Copy
        let tag2 = HtmlTag::H1;
        let copied: HtmlTag = tag2;
        assert_eq!(tag2, copied);
    }

    #[test]
    fn html_scope_derives() {
        // Test Clone
        let scope = HtmlScope::Prose;
        let cloned = scope.clone();
        assert_eq!(scope, cloned);

        // Test with String variant
        let selector_scope = HtmlScope::Selector("div.test".to_string());
        let cloned_selector = selector_scope.clone();
        assert_eq!(selector_scope, cloned_selector);
    }
}
