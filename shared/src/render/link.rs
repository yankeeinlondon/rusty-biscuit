/// The String Terminator used to end the sequence
const ST: &str = "\x1b\\";

/// The specific OSC 8 sequence to start a link
const LINK_START: &str = "\x1b]8;;";

/// The specific OSC 8 sequence to close a link
const LINK_END: &str = "\x1b]8;;\x1b\\";

use std::fmt;
use std::hash::{Hash, Hasher};

use supports_hyperlinks::Stream;

/// The type of resource a link points to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LinkType {
    File,
    Url,
}

/// Determines the resource type by how the string starts.
fn type_from_string(input: &str) -> LinkType {
    if input.starts_with("http://") || input.starts_with("https://") {
        LinkType::Url
    } else {
        LinkType::File
    }
}

/// A container for pairing display text with a link to some resource.
///
/// Supports multiple output formats: terminal (OSC 8), browser (HTML), and markdown.
///
/// ## Examples
///
/// ```
/// use shared::render::link::Link;
///
/// // Basic usage
/// let link = Link::new("Click here", "https://example.com");
///
/// // With builder methods
/// let styled_link = Link::new("Styled", "https://example.com")
///     .with_class("btn btn-primary")
///     .with_target("_blank")
///     .with_title("Opens in new tab");
///
/// // From tuple
/// let link: Link = ("Display", "https://url.com").into();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// The type of resource being linked to
    kind: LinkType,
    /// The text to display to the target device
    display: String,
    /// The link destination (URL or file path)
    link_to: String,
    /// Optional CSS class for HTML output
    class: Option<String>,
    /// Optional inline style for HTML output
    style: Option<String>,
    /// Optional target attribute for HTML (e.g., "_blank", "_self")
    target: Option<String>,
    /// Optional title/tooltip text (used in HTML and markdown)
    title: Option<String>,
}

impl Link {
    /// Creates a new link with the given display text and destination.
    pub fn new(display: impl Into<String>, link: impl Into<String>) -> Self {
        let link_to = link.into();
        let kind = type_from_string(&link_to);

        Self {
            kind,
            display: display.into(),
            link_to,
            class: None,
            style: None,
            target: None,
            title: None,
        }
    }

    // -------------------------------------------------------------------------
    // Builder methods
    // -------------------------------------------------------------------------

    /// Sets the CSS class attribute for HTML output.
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    /// Sets the inline style attribute for HTML output.
    pub fn with_style(mut self, style: impl Into<String>) -> Self {
        self.style = Some(style.into());
        self
    }

    /// Sets the target attribute for HTML output (e.g., "_blank").
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Sets the title/tooltip text (used in HTML and markdown).
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------

    /// Returns the link destination (URL or file path).
    pub fn href(&self) -> &str {
        &self.link_to
    }

    /// Returns the display text.
    pub fn display(&self) -> &str {
        &self.display
    }

    /// Returns the link type.
    pub fn kind(&self) -> LinkType {
        self.kind
    }

    /// Returns the CSS class if set.
    pub fn class(&self) -> Option<&str> {
        self.class.as_deref()
    }

    /// Returns the inline style if set.
    pub fn style(&self) -> Option<&str> {
        self.style.as_deref()
    }

    /// Returns the target attribute if set.
    pub fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }

    /// Returns the title/tooltip if set.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    // -------------------------------------------------------------------------
    // Convenience methods
    // -------------------------------------------------------------------------

    /// Returns true if this link points to a URL (http/https).
    pub fn is_url(&self) -> bool {
        self.kind == LinkType::Url
    }

    /// Returns true if this link points to a file.
    pub fn is_file(&self) -> bool {
        self.kind == LinkType::File
    }

    // -------------------------------------------------------------------------
    // Output methods
    // -------------------------------------------------------------------------

    /// Renders the link for terminal output using OSC 8 escape sequences.
    ///
    /// If the terminal supports hyperlinks, outputs a clickable link.
    /// Otherwise, outputs the display text followed by the URL in brackets.
    pub fn to_terminal(&self) -> String {
        if supports_hyperlinks::on(Stream::Stdout) {
            format!(
                "{}{}{}{}{}",
                LINK_START, self.link_to, ST, self.display, LINK_END
            )
        } else {
            format!("{} [{}]", self.display, self.link_to)
        }
    }

    /// Renders the link as an HTML anchor element.
    ///
    /// Includes class, style, target, and title attributes when set.
    /// All values are HTML-escaped to prevent XSS.
    pub fn to_browser(&self) -> String {
        let mut attrs = format!(r#"href="{}""#, html_escape(&self.link_to));

        if let Some(class) = &self.class {
            attrs.push_str(&format!(r#" class="{}""#, html_escape(class)));
        }

        if let Some(style) = &self.style {
            attrs.push_str(&format!(r#" style="{}""#, html_escape(style)));
        }

        if let Some(target) = &self.target {
            attrs.push_str(&format!(r#" target="{}""#, html_escape(target)));
        }

        if let Some(title) = &self.title {
            attrs.push_str(&format!(r#" title="{}""#, html_escape(title)));
        }

        format!("<a {}>{}</a>", attrs, html_escape(&self.display))
    }

    /// Renders the link as markdown: `[display](url)` or `[display](url "title")`.
    ///
    /// Special characters in display and URL are escaped appropriately.
    pub fn to_markdown(&self) -> String {
        let display = self.display.replace('[', "\\[").replace(']', "\\]");
        let link = self.link_to.replace('(', "%28").replace(')', "%29");

        if let Some(title) = &self.title {
            let escaped_title = title.replace('"', "\\\"");
            format!("[{}]({} \"{}\")", display, link, escaped_title)
        } else {
            format!("[{}]({})", display, link)
        }
    }
}

// -----------------------------------------------------------------------------
// Trait implementations
// -----------------------------------------------------------------------------

impl fmt::Display for Link {
    /// Formats the link using terminal representation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_terminal())
    }
}

impl Hash for Link {
    /// Hashes based on kind, display, and link_to (the identity fields).
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.display.hash(state);
        self.link_to.hash(state);
    }
}

impl PartialOrd for Link {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Link {
    /// Orders by link destination, then by display text.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.link_to
            .cmp(&other.link_to)
            .then_with(|| self.display.cmp(&other.display))
    }
}

impl<S1, S2> From<(S1, S2)> for Link
where
    S1: Into<String>,
    S2: Into<String>,
{
    /// Creates a Link from a tuple of (display, url).
    fn from((display, link): (S1, S2)) -> Self {
        Link::new(display, link)
    }
}

// -----------------------------------------------------------------------------
// Helper functions
// -----------------------------------------------------------------------------

/// Escapes HTML special characters in a string.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_type_detection() {
        let http = Link::new("test", "http://example.com");
        assert!(http.is_url());
        assert!(!http.is_file());

        let https = Link::new("test", "https://example.com");
        assert!(https.is_url());

        let file = Link::new("test", "/path/to/file");
        assert!(file.is_file());
        assert!(!file.is_url());
    }

    #[test]
    fn test_builder_methods() {
        let link = Link::new("text", "https://example.com")
            .with_class("my-class")
            .with_style("color: red")
            .with_target("_blank")
            .with_title("A title");

        assert_eq!(link.class(), Some("my-class"));
        assert_eq!(link.style(), Some("color: red"));
        assert_eq!(link.target(), Some("_blank"));
        assert_eq!(link.title(), Some("A title"));
    }

    #[test]
    fn test_getters() {
        let link = Link::new("Display Text", "https://example.com/path");

        assert_eq!(link.display(), "Display Text");
        assert_eq!(link.href(), "https://example.com/path");
        assert_eq!(link.kind(), LinkType::Url);
    }

    #[test]
    fn test_to_browser_basic() {
        let link = Link::new("Click", "https://example.com");
        assert_eq!(
            link.to_browser(),
            r#"<a href="https://example.com">Click</a>"#
        );
    }

    #[test]
    fn test_to_browser_with_attributes() {
        let link = Link::new("Click", "https://example.com")
            .with_class("btn")
            .with_style("color: blue")
            .with_target("_blank")
            .with_title("Tooltip");

        assert_eq!(
            link.to_browser(),
            r#"<a href="https://example.com" class="btn" style="color: blue" target="_blank" title="Tooltip">Click</a>"#
        );
    }

    #[test]
    fn test_to_browser_escapes_html() {
        let link = Link::new("<script>", "https://example.com?a=1&b=2");
        assert_eq!(
            link.to_browser(),
            r#"<a href="https://example.com?a=1&amp;b=2">&lt;script&gt;</a>"#
        );
    }

    #[test]
    fn test_to_markdown_basic() {
        let link = Link::new("Example", "https://example.com");
        assert_eq!(link.to_markdown(), "[Example](https://example.com)");
    }

    #[test]
    fn test_to_markdown_with_title() {
        let link = Link::new("Example", "https://example.com").with_title("A tooltip");
        assert_eq!(
            link.to_markdown(),
            r#"[Example](https://example.com "A tooltip")"#
        );
    }

    #[test]
    fn test_to_markdown_escapes_special_chars() {
        let link = Link::new("[text]", "https://example.com/path(1)");
        assert_eq!(
            link.to_markdown(),
            r"[\[text\]](https://example.com/path%281%29)"
        );
    }

    #[test]
    fn test_from_tuple() {
        let link: Link = ("Display", "https://url.com").into();
        assert_eq!(link.display(), "Display");
        assert_eq!(link.href(), "https://url.com");
    }

    #[test]
    fn test_hash_equality() {
        use std::collections::HashSet;

        let link1 = Link::new("Text", "https://example.com");
        let link2 = Link::new("Text", "https://example.com").with_class("different");

        // Same identity fields, different styling - should hash the same
        let mut set = HashSet::new();
        set.insert(link1);
        // link2 has same hash but different Eq (class differs), so it's a different entry
        // Actually, our Eq derives from all fields, so they're different
        set.insert(link2);
        assert_eq!(set.len(), 2); // They're different because Eq includes all fields
    }

    #[test]
    fn test_ordering() {
        let a = Link::new("Z", "https://aaa.com");
        let b = Link::new("A", "https://bbb.com");
        let c = Link::new("A", "https://aaa.com");

        // Orders by link_to first, then display
        assert!(c < a); // aaa < aaa, but A < Z... wait, same link_to
        assert!(a < b); // aaa < bbb
        assert!(c < a); // same link_to, A < Z
    }
}
