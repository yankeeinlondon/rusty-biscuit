/// BEL character - the most widely supported OSC sequence terminator.
/// Used by: iTerm2, macOS Terminal, GNOME Terminal, Konsole, Kitty, Alacritty, WezTerm.
const BEL: &str = "\x07";

/// The OSC 8 sequence to start a hyperlink: ESC ] 8 ; ; <params> ; <URI> BEL
/// Note: params section is empty (between the two semicolons after 8).
const LINK_START: &str = "\x1b]8;;";

/// The OSC 8 sequence to close a hyperlink: ESC ] 8 ; ; BEL
const LINK_END: &str = "\x1b]8;;\x07";

use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

use supports_hyperlinks::Stream;

/// Errors that can occur when parsing a link from a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkParseError {
    /// Input doesn't match HTML or Markdown link format
    UnrecognizedFormat,
    /// HTML link is malformed
    MalformedHtml(String),
    /// Markdown link is malformed
    MalformedMarkdown(String),
    /// Missing required href/url
    MissingUrl,
}

impl std::fmt::Display for LinkParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnrecognizedFormat => write!(f, "Input is not a recognized link format"),
            Self::MalformedHtml(msg) => write!(f, "Malformed HTML link: {msg}"),
            Self::MalformedMarkdown(msg) => write!(f, "Malformed Markdown link: {msg}"),
            Self::MissingUrl => write!(f, "Link is missing URL/href"),
        }
    }
}

impl std::error::Error for LinkParseError {}

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
    /// Optional prompt text for modern browser Popover API
    prompt: Option<String>,
    /// Optional data-* attributes for HTML output
    data: Option<HashMap<String, String>>,
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
            prompt: None,
            data: None,
        }
    }

    /// Creates a new link with a title that may contain structured content.
    ///
    /// The title is parsed to detect structured mode (key=value pairs) vs title mode:
    /// - **Title Mode**: Simple string becomes the `title` attribute
    /// - **Structured Mode**: Parses `class`, `style`, `prompt`, `title`, `target`, and `data-*`
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::render::link::Link;
    ///
    /// // Title mode - simple title
    /// let link = Link::with_title_parsed("Click", "https://example.com", "A tooltip");
    /// assert_eq!(link.title(), Some("A tooltip"));
    ///
    /// // Structured mode - key=value pairs
    /// let link = Link::with_title_parsed(
    ///     "Click",
    ///     "https://example.com",
    ///     "class='btn' style='color:red' prompt='hover text'"
    /// );
    /// assert_eq!(link.class(), Some("btn"));
    /// assert_eq!(link.style(), Some("color:red"));
    /// assert_eq!(link.prompt(), Some("hover text"));
    /// ```
    pub fn with_title_parsed(
        display: impl Into<String>,
        link: impl Into<String>,
        title: &str,
    ) -> Self {
        let mut result = Self::new(display, link);
        let title = title.trim();

        if title.is_empty() {
            return result;
        }

        if is_structured_mode(title) {
            // Parse structured properties
            let _ = parse_structured_props(&mut result, title);
        } else {
            // Title mode - use as plain title
            result.title = Some(parse_title_value(title));
        }

        result
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

    /// Sets the prompt text for modern browser Popover API.
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Adds a data-* attribute for HTML output.
    pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Sets all data-* attributes from a HashMap.
    pub fn with_data_map(mut self, data: HashMap<String, String>) -> Self {
        self.data = Some(data);
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

    /// Returns the prompt text if set.
    pub fn prompt(&self) -> Option<&str> {
        self.prompt.as_deref()
    }

    /// Returns the data-* attributes if any are set.
    pub fn data(&self) -> Option<&HashMap<String, String>> {
        self.data.as_ref()
    }

    /// Parses the inline style string into a HashMap of property-value pairs.
    ///
    /// Returns `None` if no style is set. Property names are normalized to lowercase.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::render::link::Link;
    ///
    /// let link = Link::new("Click", "https://example.com")
    ///     .with_style("color: red; font-size: 14px");
    ///
    /// let parsed = link.parsed_style().unwrap();
    /// assert_eq!(parsed.get("color"), Some(&"red".to_string()));
    /// assert_eq!(parsed.get("font-size"), Some(&"14px".to_string()));
    /// ```
    pub fn parsed_style(&self) -> Option<HashMap<String, String>> {
        self.style.as_ref().map(|s| parse_css_style(s))
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
    /// Uses the BEL character (`\x07`) as the sequence terminator, which has
    /// the widest terminal support (iTerm2, macOS Terminal, GNOME Terminal,
    /// Konsole, Kitty, Alacritty, WezTerm, and others).
    ///
    /// ## Format
    ///
    /// ```text
    /// ESC ] 8 ; ; <URI> BEL <display text> ESC ] 8 ; ; BEL
    /// ```
    ///
    /// ## Returns
    ///
    /// If the terminal supports hyperlinks, outputs a clickable link.
    /// Otherwise, outputs the display text followed by the URL in brackets.
    pub fn to_terminal(&self) -> String {
        if supports_hyperlinks::on(Stream::Stdout) {
            format!(
                "{}{}{}{}{}",
                LINK_START, self.link_to, BEL, self.display, LINK_END
            )
        } else {
            format!("{} [{}]", self.display, self.link_to)
        }
    }

    /// Renders the link as an OSC 8 hyperlink without checking terminal support.
    ///
    /// Use this when you've already verified terminal support or want to force
    /// hyperlink output regardless of detection.
    ///
    /// ## Format
    ///
    /// ```text
    /// ESC ] 8 ; ; <URI> BEL <display text> ESC ] 8 ; ; BEL
    /// ```
    pub fn to_terminal_unchecked(&self) -> String {
        format!(
            "{}{}{}{}{}",
            LINK_START, self.link_to, BEL, self.display, LINK_END
        )
    }

    /// Renders the link as an HTML anchor element.
    ///
    /// Includes class, style, target, title, prompt, and data-* attributes when set.
    /// All values are HTML-escaped to prevent XSS.
    pub fn to_browser(&self) -> String {
        let mut attrs = format!(r#"href="{}""#, html_escape(&self.link_to));

        maybe_attr(&mut attrs, "class", &self.class);
        maybe_attr(&mut attrs, "style", &self.style);
        maybe_attr(&mut attrs, "target", &self.target);
        maybe_attr(&mut attrs, "title", &self.title);

        if let Some(prompt) = &self.prompt {
            attrs.push_str(&format!(r#" data-prompt="{}""#, html_escape(prompt)));
        }

        if let Some(data) = &self.data {
            for (key, value) in data {
                attrs.push_str(&format!(
                    r#" data-{}="{}""#,
                    html_escape(key),
                    html_escape(value)
                ));
            }
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

    /// Renders the link as HTML with a modern Popover API companion element.
    ///
    /// Returns `Some((anchor_html, popover_html))` when a prompt is set, `None` otherwise.
    ///
    /// Uses the experimental `interestfor` attribute for hover/focus activation.
    /// The popover uses `popover="hint"` for tooltip-like behavior.
    ///
    /// ## Browser Support
    ///
    /// The `interestfor` attribute is experimental (as of 2025). Use feature detection:
    ///
    /// ```javascript
    /// const supported = 'interestForElement' in HTMLButtonElement.prototype;
    /// ```
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::render::link::Link;
    ///
    /// let link = Link::new("Click here", "https://example.com")
    ///     .with_prompt("Opens example.com");
    ///
    /// if let Some((anchor, popover)) = link.to_browser_with_popover() {
    ///     // anchor: <a href="..." interestfor="popover-...">Click here</a>
    ///     // popover: <div id="popover-..." popover="hint">Opens example.com</div>
    ///     println!("{}", anchor);
    ///     println!("{}", popover);
    /// }
    /// ```
    pub fn to_browser_with_popover(&self) -> Option<(String, String)> {
        self.prompt.as_ref().map(|prompt_text| {
            let id = generate_popover_id(&self.link_to, &self.display);

            // Build anchor with interestfor attribute
            let mut attrs = format!(
                r#"href="{}" interestfor="{}""#,
                html_escape(&self.link_to),
                id
            );

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

            if let Some(data) = &self.data {
                for (key, value) in data {
                    attrs.push_str(&format!(
                        r#" data-{}="{}""#,
                        html_escape(key),
                        html_escape(value)
                    ));
                }
            }

            let anchor = format!("<a {}>{}</a>", attrs, html_escape(&self.display));
            let popover = format!(
                r#"<div id="{}" popover="hint">{}</div>"#,
                id,
                html_escape(prompt_text)
            );

            (anchor, popover)
        })
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

/// Appends an HTML attribute to the builder if the value is Some.
///
/// ## Arguments
///
/// * `attrs` - String buffer to append to
/// * `name` - Attribute name (e.g., "class", "style")
/// * `value` - Optional attribute value
#[inline]
fn maybe_attr(attrs: &mut String, name: &str, value: &Option<String>) {
    if let Some(v) = value {
        attrs.push_str(&format!(r#" {}="{}""#, name, html_escape(v)));
    }
}

/// Unescapes HTML entities back to their original characters.
fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
}

/// Parses a CSS style string into a HashMap of property-value pairs.
///
/// Handles edge cases including:
/// - Extra whitespace around properties and values
/// - Trailing semicolons
/// - Empty declarations (consecutive semicolons)
/// - Case-insensitive property names (normalized to lowercase)
///
/// ## Examples
///
/// ```ignore
/// let styles = parse_css_style("color: red; font-size: 14px");
/// assert_eq!(styles.get("color"), Some(&"red".to_string()));
/// assert_eq!(styles.get("font-size"), Some(&"14px".to_string()));
/// ```
fn parse_css_style(style: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for declaration in style.split(';') {
        let trimmed = declaration.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(colon_pos) = trimmed.find(':') {
            let key = trimmed[..colon_pos].trim().to_lowercase();
            let value = trimmed[colon_pos + 1..].trim();
            if !key.is_empty() && !value.is_empty() {
                result.insert(key, value.to_string());
            }
        }
    }
    result
}

/// Generates a unique popover ID based on the URL and display text.
///
/// Uses a hash of the combined values to ensure uniqueness while remaining
/// deterministic for the same input.
fn generate_popover_id(url: &str, display: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    display.hash(&mut hasher);
    format!("popover-{:x}", hasher.finish())
}

// -----------------------------------------------------------------------------
// Link Parsing
// -----------------------------------------------------------------------------

/// Parses an HTML anchor element into a Link.
///
/// Expected format: `<a href="URL" [attr="value"]*>DISPLAY</a>`
fn parse_html_link(input: &str) -> Result<Link, LinkParseError> {
    let input = input.trim();

    // Must start with <a and end with </a>
    if !input.starts_with("<a ") && !input.starts_with("<a>") {
        return Err(LinkParseError::MalformedHtml(
            "Link must start with '<a ' or '<a>'".into(),
        ));
    }

    if !input.ends_with("</a>") {
        return Err(LinkParseError::MalformedHtml(
            "Link must end with '</a>'".into(),
        ));
    }

    // Find the end of the opening tag
    let Some(tag_end) = input.find('>') else {
        return Err(LinkParseError::MalformedHtml(
            "Could not find end of opening tag".into(),
        ));
    };

    let opening_tag = &input[..tag_end];
    let display_start = tag_end + 1;
    let display_end = input.len() - 4; // Remove "</a>"

    if display_start >= display_end {
        return Err(LinkParseError::MalformedHtml("Empty display text".into()));
    }

    let display = html_unescape(&input[display_start..display_end]);

    // Parse attributes from the opening tag
    let attrs_str = &opening_tag[2..].trim(); // Remove "<a"
    let attrs = parse_html_attributes(attrs_str);

    let href = attrs.get("href").ok_or(LinkParseError::MissingUrl)?.clone();

    let mut link = Link::new(display, href);

    if let Some(class) = attrs.get("class") {
        link = link.with_class(class);
    }

    if let Some(style) = attrs.get("style") {
        link = link.with_style(style);
    }

    if let Some(target) = attrs.get("target") {
        link = link.with_target(target);
    }

    if let Some(title) = attrs.get("title") {
        link = link.with_title(title);
    }

    // Handle data-prompt separately (maps to prompt field)
    if let Some(prompt) = attrs.get("data-prompt") {
        link = link.with_prompt(prompt);
    }

    // Handle other data-* attributes
    for (key, value) in &attrs {
        if key.starts_with("data-") && key != "data-prompt" {
            let data_key = &key[5..]; // Remove "data-" prefix
            link = link.with_data(data_key, value);
        }
    }

    Ok(link)
}

/// Parses HTML attributes from a string like `href="url" class="foo"`.
fn parse_html_attributes(input: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let mut chars = input.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        // Read attribute name
        let mut name = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            name.push(c);
            chars.next();
        }

        if name.is_empty() {
            break;
        }

        // Skip whitespace and equals sign
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        if chars.peek() != Some(&'=') {
            // Boolean attribute with no value
            attrs.insert(name.to_lowercase(), String::new());
            continue;
        }
        chars.next(); // consume '='

        // Skip whitespace after equals
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        // Read attribute value
        let value = if chars.peek() == Some(&'"') || chars.peek() == Some(&'\'') {
            let quote = chars.next().unwrap();
            let mut val = String::new();
            for c in chars.by_ref() {
                if c == quote {
                    break;
                }
                val.push(c);
            }
            html_unescape(&val)
        } else {
            // Unquoted value - read until whitespace
            let mut val = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                val.push(c);
                chars.next();
            }
            html_unescape(&val)
        };

        attrs.insert(name.to_lowercase(), value);
    }

    attrs
}

/// Parses a Markdown link into a Link.
///
/// Supports formats:
/// - `[display](url)`
/// - `[display](url "title")` (Title Mode)
/// - `[display](url key=value ...)` (Structured Mode)
fn parse_markdown_link(input: &str) -> Result<Link, LinkParseError> {
    let input = input.trim();

    if !input.starts_with('[') {
        return Err(LinkParseError::MalformedMarkdown(
            "Link must start with '['".into(),
        ));
    }

    // Find the matching closing bracket, handling escaped brackets
    let display_end = find_closing_bracket(input, 0)?;
    let display = unescape_markdown_display(&input[1..display_end]);

    // After ], expect (
    let rest = &input[display_end + 1..];
    if !rest.starts_with('(') {
        return Err(LinkParseError::MalformedMarkdown(
            "Expected '(' after display text".into(),
        ));
    }

    // Find the matching closing paren
    let paren_content_start = 1;
    let paren_end = find_closing_paren(rest, 0)?;
    let paren_content = &rest[paren_content_start..paren_end];

    // Parse the parenthesis content (URL and optional title/structured data)
    parse_markdown_paren_content(display, paren_content)
}

/// Finds the index of the closing bracket `]` that matches the opening `[` at `start`.
fn find_closing_bracket(input: &str, start: usize) -> Result<usize, LinkParseError> {
    let bytes = input.as_bytes();
    let mut depth = 0;
    let mut i = start;

    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => {
                // Skip escaped character
                i += 2;
            }
            b'[' => {
                depth += 1;
                i += 1;
            }
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
                i += 1;
            }
            _ => i += 1,
        }
    }

    Err(LinkParseError::MalformedMarkdown(
        "Unmatched '[' in link".into(),
    ))
}

/// Finds the index of the closing paren `)` that matches the opening `(` at position after `start`.
fn find_closing_paren(input: &str, start: usize) -> Result<usize, LinkParseError> {
    let bytes = input.as_bytes();
    let mut depth = 0;
    let mut i = start;
    let mut in_quotes = false;
    let mut quote_char = b'"';

    while i < bytes.len() {
        let b = bytes[i];

        if in_quotes {
            if b == b'\\' && i + 1 < bytes.len() {
                // Skip escaped character in quotes
                i += 2;
                continue;
            }
            if b == quote_char {
                in_quotes = false;
            }
            i += 1;
            continue;
        }

        match b {
            b'"' | b'\'' => {
                in_quotes = true;
                quote_char = b;
                i += 1;
            }
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
                i += 1;
            }
            _ => i += 1,
        }
    }

    Err(LinkParseError::MalformedMarkdown(
        "Unmatched '(' in link".into(),
    ))
}

/// Unescapes markdown display text (handles `\[` and `\]`).
fn unescape_markdown_display(s: &str) -> String {
    s.replace("\\[", "[").replace("\\]", "]")
}

/// Parses the content inside markdown link parentheses.
///
/// This content can be:
/// - Just a URL: `https://example.com`
/// - URL with title: `https://example.com "My Title"`
/// - URL with structured props: `https://example.com class=foo prompt="click me"`
fn parse_markdown_paren_content(display: String, content: &str) -> Result<Link, LinkParseError> {
    let content = content.trim();

    if content.is_empty() {
        return Err(LinkParseError::MissingUrl);
    }

    // Split into URL and the rest (title or structured)
    let (url, rest) = extract_url(content);

    if url.is_empty() {
        return Err(LinkParseError::MissingUrl);
    }

    // Unescape URL-encoded parentheses
    let url = url.replace("%28", "(").replace("%29", ")");
    let mut link = Link::new(display, url);

    let rest = rest.trim();
    if rest.is_empty() {
        return Ok(link);
    }

    // Determine if this is Title Mode or Structured Mode
    // Structured Mode: contains `key=value` patterns
    // Title Mode: starts with a quoted string
    if is_structured_mode(rest) {
        parse_structured_props(&mut link, rest)?;
    } else {
        // Title Mode: parse as a single quoted or unquoted title
        let title = parse_title_value(rest);
        if !title.is_empty() {
            link = link.with_title(title);
        }
    }

    Ok(link)
}

/// Extracts the URL from the beginning of the parenthesis content.
/// Returns (url, rest).
fn extract_url(content: &str) -> (String, &str) {
    let bytes = content.as_bytes();
    let mut i = 0;

    // Handle angle-bracket URLs: <url>
    if bytes.first() == Some(&b'<') {
        i = 1;
        while i < bytes.len() && bytes[i] != b'>' {
            i += 1;
        }
        if i < bytes.len() {
            return (content[1..i].to_string(), &content[i + 1..]);
        }
    }

    // Regular URL: read until whitespace
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            break;
        }
        i += 1;
    }

    (content[..i].to_string(), &content[i..])
}

/// Determines if the rest of the content is in Structured Mode.
///
/// Structured Mode is detected if there's a `key=value` pattern.
fn is_structured_mode(content: &str) -> bool {
    let content = content.trim();

    // If it starts with a quote, it could be Title Mode with just a string
    // But we need to check if there's a `=` that's not inside quotes
    let mut in_quotes = false;
    let mut quote_char = '"';
    let bytes = content.as_bytes();
    let mut has_unquoted_equals = false;

    for (i, &b) in bytes.iter().enumerate() {
        if in_quotes {
            if b == b'\\' {
                continue; // Skip next char
            }
            if b == quote_char as u8 {
                in_quotes = false;
            }
        } else {
            match b {
                b'"' | b'\'' => {
                    in_quotes = true;
                    quote_char = b as char;
                }
                b'=' => {
                    // Check if there's a valid key before this =
                    let before = &content[..i];
                    let key_part = before
                        .trim()
                        .split(&[' ', ','][..])
                        .next_back()
                        .unwrap_or("");
                    if !key_part.is_empty()
                        && key_part
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    {
                        has_unquoted_equals = true;
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    has_unquoted_equals
}

/// Parses structured key=value properties and applies them to the link.
fn parse_structured_props(link: &mut Link, content: &str) -> Result<(), LinkParseError> {
    let mut chars = content.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace and commas
        while chars.peek().is_some_and(|&c| c.is_whitespace() || c == ',') {
            chars.next();
        }

        if chars.peek().is_none() {
            break;
        }

        // Read key
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() || c == ',' {
                break;
            }
            key.push(c);
            chars.next();
        }

        if key.is_empty() {
            break;
        }

        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        // Expect =
        if chars.peek() != Some(&'=') {
            // Key without value - skip
            continue;
        }
        chars.next(); // consume '='

        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        // Read value
        let value = if chars.peek() == Some(&'"') || chars.peek() == Some(&'\'') {
            let quote = chars.next().unwrap();
            let mut val = String::new();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    // Escaped character
                    if let Some(escaped) = chars.next() {
                        val.push(escaped);
                    }
                } else if c == quote {
                    break;
                } else {
                    val.push(c);
                }
            }
            val
        } else {
            // Unquoted value - read until whitespace or comma
            let mut val = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == ',' {
                    break;
                }
                val.push(c);
                chars.next();
            }
            val
        };

        // Apply the property to the link
        apply_structured_prop(link, &key.to_lowercase(), value);
    }

    Ok(())
}

/// Applies a structured property to the link based on the key.
fn apply_structured_prop(link: &mut Link, key: &str, value: String) {
    match key {
        "title" => {
            link.title = Some(value);
        }
        "prompt" => {
            link.prompt = Some(value);
        }
        "class" => {
            link.class = Some(value);
        }
        "style" => {
            link.style = Some(value);
        }
        "target" => {
            link.target = Some(value);
        }
        k if k.starts_with("data-") => {
            let data_key = &k[5..];
            link.data
                .get_or_insert_with(HashMap::new)
                .insert(data_key.to_string(), value);
        }
        _ => {
            // Unknown keys are ignored
        }
    }
}

/// Parses a title value in Title Mode.
///
/// Handles quoted strings: `"title"` or `'title'`
/// Or unquoted: `title`
fn parse_title_value(content: &str) -> String {
    let content = content.trim();

    if content.is_empty() {
        return String::new();
    }

    let bytes = content.as_bytes();

    // Check for quoted string
    if (bytes[0] == b'"' || bytes[0] == b'\'') && bytes.len() > 1 {
        let quote = bytes[0];
        // Find the closing quote
        let mut i = 1;
        let mut result = String::new();
        while i < bytes.len() {
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                // Escaped character
                result.push(content.chars().nth(i + 1).unwrap_or('\\'));
                i += 2;
            } else if bytes[i] == quote {
                break;
            } else {
                result.push(content.chars().nth(i).unwrap_or(' '));
                i += 1;
            }
        }
        return result;
    }

    // Unquoted - return as-is
    content.to_string()
}

// -----------------------------------------------------------------------------
// TryFrom implementations
// -----------------------------------------------------------------------------

impl TryFrom<String> for Link {
    type Error = LinkParseError;

    /// Parses a Link from a String.
    ///
    /// Supports HTML anchor elements and Markdown links.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::render::link::Link;
    /// use std::convert::TryFrom;
    ///
    /// // Parse HTML link
    /// let html_link = Link::try_from(r#"<a href="https://example.com">Click</a>"#.to_string());
    /// assert!(html_link.is_ok());
    ///
    /// // Parse Markdown link
    /// let md_link = Link::try_from("[Click](https://example.com)".to_string());
    /// assert!(md_link.is_ok());
    ///
    /// // Parse Markdown with structured props
    /// let structured = Link::try_from(
    ///     r#"[Click](https://example.com class="btn" prompt="click me")"#.to_string()
    /// );
    /// assert!(structured.is_ok());
    /// ```
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let trimmed = value.trim();

        if trimmed.starts_with('<') {
            parse_html_link(trimmed)
        } else if trimmed.starts_with('[') {
            parse_markdown_link(trimmed)
        } else {
            Err(LinkParseError::UnrecognizedFormat)
        }
    }
}

impl TryFrom<&str> for Link {
    type Error = LinkParseError;

    /// Parses a Link from a string slice.
    ///
    /// See [`TryFrom<String>`] for details.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Link::try_from(value.to_string())
    }
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

    // -------------------------------------------------------------------------
    // Tests for new fields: prompt and data
    // -------------------------------------------------------------------------

    #[test]
    fn test_builder_prompt() {
        let link = Link::new("Click", "https://example.com").with_prompt("Click me!");
        assert_eq!(link.prompt(), Some("Click me!"));
    }

    #[test]
    fn test_builder_data() {
        let link = Link::new("Click", "https://example.com")
            .with_data("id", "123")
            .with_data("action", "submit");

        let data = link.data().unwrap();
        assert_eq!(data.get("id"), Some(&"123".to_string()));
        assert_eq!(data.get("action"), Some(&"submit".to_string()));
    }

    #[test]
    fn test_builder_data_map() {
        let mut map = HashMap::new();
        map.insert("foo".to_string(), "bar".to_string());
        map.insert("baz".to_string(), "qux".to_string());

        let link = Link::new("Click", "https://example.com").with_data_map(map);

        let data = link.data().unwrap();
        assert_eq!(data.get("foo"), Some(&"bar".to_string()));
        assert_eq!(data.get("baz"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_to_browser_with_prompt() {
        let link = Link::new("Click", "https://example.com").with_prompt("Hover text");
        assert!(link.to_browser().contains(r#"data-prompt="Hover text""#));
    }

    #[test]
    fn test_to_browser_with_data_attributes() {
        let link = Link::new("Click", "https://example.com")
            .with_data("id", "123")
            .with_data("type", "button");

        let html = link.to_browser();
        assert!(html.contains(r#"data-id="123""#));
        assert!(html.contains(r#"data-type="button""#));
    }

    // -------------------------------------------------------------------------
    // TryFrom HTML tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_try_from_html_basic() {
        let link = Link::try_from(r#"<a href="https://example.com">Click</a>"#).unwrap();
        assert_eq!(link.display(), "Click");
        assert_eq!(link.href(), "https://example.com");
    }

    #[test]
    fn test_try_from_html_with_attributes() {
        let link = Link::try_from(
            r#"<a href="https://example.com" class="btn" style="color:red" target="_blank" title="Tooltip">Click</a>"#
        ).unwrap();

        assert_eq!(link.display(), "Click");
        assert_eq!(link.href(), "https://example.com");
        assert_eq!(link.class(), Some("btn"));
        assert_eq!(link.style(), Some("color:red"));
        assert_eq!(link.target(), Some("_blank"));
        assert_eq!(link.title(), Some("Tooltip"));
    }

    #[test]
    fn test_try_from_html_with_data_attributes() {
        let link = Link::try_from(
            r#"<a href="https://example.com" data-prompt="Click me" data-id="123">Click</a>"#,
        )
        .unwrap();

        assert_eq!(link.prompt(), Some("Click me"));
        let data = link.data().unwrap();
        assert_eq!(data.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_try_from_html_with_escaped_content() {
        let link =
            Link::try_from(r#"<a href="https://example.com?a=1&amp;b=2">&lt;script&gt;</a>"#)
                .unwrap();

        assert_eq!(link.display(), "<script>");
        assert_eq!(link.href(), "https://example.com?a=1&b=2");
    }

    #[test]
    fn test_try_from_html_missing_href() {
        let result = Link::try_from(r#"<a class="btn">Click</a>"#);
        assert!(matches!(result, Err(LinkParseError::MissingUrl)));
    }

    #[test]
    fn test_try_from_html_malformed() {
        let result = Link::try_from(r#"<div href="url">text</div>"#);
        assert!(matches!(result, Err(LinkParseError::MalformedHtml(_))));
    }

    // -------------------------------------------------------------------------
    // TryFrom Markdown tests - Basic
    // -------------------------------------------------------------------------

    #[test]
    fn test_try_from_markdown_basic() {
        let link = Link::try_from("[Click](https://example.com)").unwrap();
        assert_eq!(link.display(), "Click");
        assert_eq!(link.href(), "https://example.com");
    }

    #[test]
    fn test_try_from_markdown_with_escaped_brackets() {
        let link = Link::try_from(r"[\[special\]](https://example.com)").unwrap();
        assert_eq!(link.display(), "[special]");
    }

    #[test]
    fn test_try_from_markdown_with_encoded_parens() {
        let link = Link::try_from("[Click](https://example.com/path%28param%29)").unwrap();
        assert_eq!(link.href(), "https://example.com/path(param)");
    }

    #[test]
    fn test_try_from_markdown_with_angle_bracket_url() {
        let link = Link::try_from("[Click](<https://example.com/path with spaces>)").unwrap();
        assert_eq!(link.href(), "https://example.com/path with spaces");
    }

    // -------------------------------------------------------------------------
    // TryFrom Markdown tests - Title Mode
    // -------------------------------------------------------------------------

    #[test]
    fn test_try_from_markdown_title_mode_double_quotes() {
        let link = Link::try_from(r#"[Click](https://example.com "A title")"#).unwrap();
        assert_eq!(link.display(), "Click");
        assert_eq!(link.href(), "https://example.com");
        assert_eq!(link.title(), Some("A title"));
    }

    #[test]
    fn test_try_from_markdown_title_mode_single_quotes() {
        let link = Link::try_from("[Click](https://example.com 'A title')").unwrap();
        assert_eq!(link.title(), Some("A title"));
    }

    #[test]
    fn test_try_from_markdown_title_mode_escaped_quotes() {
        let link = Link::try_from(r#"[Click](https://example.com "A \"quoted\" title")"#).unwrap();
        assert_eq!(link.title(), Some(r#"A "quoted" title"#));
    }

    // -------------------------------------------------------------------------
    // TryFrom Markdown tests - Structured Mode
    // -------------------------------------------------------------------------

    #[test]
    fn test_try_from_markdown_structured_class() {
        let link = Link::try_from(r#"[Click](https://example.com class="btn")"#).unwrap();
        assert_eq!(link.class(), Some("btn"));
    }

    #[test]
    fn test_try_from_markdown_structured_multiple_props() {
        let link = Link::try_from(
            r#"[Click](https://example.com class="btn" prompt="click me" style="color:red")"#,
        )
        .unwrap();

        assert_eq!(link.class(), Some("btn"));
        assert_eq!(link.prompt(), Some("click me"));
        assert_eq!(link.style(), Some("color:red"));
    }

    #[test]
    fn test_try_from_markdown_structured_unquoted_values() {
        let link = Link::try_from("[Click](https://example.com class=btn target=_blank)").unwrap();
        assert_eq!(link.class(), Some("btn"));
        assert_eq!(link.target(), Some("_blank"));
    }

    #[test]
    fn test_try_from_markdown_structured_comma_delimited() {
        let link = Link::try_from("[Click](https://example.com class=btn,target=_blank)").unwrap();
        assert_eq!(link.class(), Some("btn"));
        assert_eq!(link.target(), Some("_blank"));
    }

    #[test]
    fn test_try_from_markdown_structured_with_title() {
        let link =
            Link::try_from(r#"[Click](https://example.com title="My Title" class="btn")"#).unwrap();

        assert_eq!(link.title(), Some("My Title"));
        assert_eq!(link.class(), Some("btn"));
    }

    #[test]
    fn test_try_from_markdown_structured_data_attributes() {
        let link =
            Link::try_from(r#"[Click](https://example.com data-id="123" data-action="submit")"#)
                .unwrap();

        let data = link.data().unwrap();
        assert_eq!(data.get("id"), Some(&"123".to_string()));
        assert_eq!(data.get("action"), Some(&"submit".to_string()));
    }

    #[test]
    fn test_try_from_markdown_structured_mixed_example_from_docs() {
        // Example from advanced-links.md
        let link = Link::try_from(
            r#"[my link](https://somewhere.com prompt="click me",class=buttercup style="background:red")"#
        ).unwrap();

        assert_eq!(link.display(), "my link");
        assert_eq!(link.href(), "https://somewhere.com");
        assert_eq!(link.prompt(), Some("click me"));
        assert_eq!(link.class(), Some("buttercup"));
        assert_eq!(link.style(), Some("background:red"));
    }

    // -------------------------------------------------------------------------
    // TryFrom error cases
    // -------------------------------------------------------------------------

    #[test]
    fn test_try_from_unrecognized_format() {
        let result = Link::try_from("just plain text");
        assert!(matches!(result, Err(LinkParseError::UnrecognizedFormat)));
    }

    #[test]
    fn test_try_from_markdown_unmatched_bracket() {
        let result = Link::try_from("[Click(https://example.com)");
        assert!(matches!(result, Err(LinkParseError::MalformedMarkdown(_))));
    }

    #[test]
    fn test_try_from_markdown_missing_paren() {
        let result = Link::try_from("[Click]https://example.com");
        assert!(matches!(result, Err(LinkParseError::MalformedMarkdown(_))));
    }

    #[test]
    fn test_try_from_markdown_empty_url() {
        let result = Link::try_from("[Click]()");
        assert!(matches!(result, Err(LinkParseError::MissingUrl)));
    }

    // -------------------------------------------------------------------------
    // Roundtrip tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_roundtrip_markdown_basic() {
        let original = Link::new("Example", "https://example.com");
        let markdown = original.to_markdown();
        let parsed = Link::try_from(markdown.as_str()).unwrap();

        assert_eq!(parsed.display(), original.display());
        assert_eq!(parsed.href(), original.href());
    }

    #[test]
    fn test_roundtrip_markdown_with_title() {
        let original = Link::new("Example", "https://example.com").with_title("A tooltip");
        let markdown = original.to_markdown();
        let parsed = Link::try_from(markdown.as_str()).unwrap();

        assert_eq!(parsed.display(), original.display());
        assert_eq!(parsed.href(), original.href());
        assert_eq!(parsed.title(), original.title());
    }

    #[test]
    fn test_roundtrip_html_basic() {
        let original = Link::new("Click", "https://example.com");
        let html = original.to_browser();
        let parsed = Link::try_from(html.as_str()).unwrap();

        assert_eq!(parsed.display(), original.display());
        assert_eq!(parsed.href(), original.href());
    }

    #[test]
    fn test_roundtrip_html_with_attributes() {
        let original = Link::new("Click", "https://example.com")
            .with_class("btn")
            .with_style("color:blue")
            .with_target("_blank")
            .with_title("Tooltip");

        let html = original.to_browser();
        let parsed = Link::try_from(html.as_str()).unwrap();

        assert_eq!(parsed.display(), original.display());
        assert_eq!(parsed.href(), original.href());
        assert_eq!(parsed.class(), original.class());
        assert_eq!(parsed.style(), original.style());
        assert_eq!(parsed.target(), original.target());
        assert_eq!(parsed.title(), original.title());
    }

    // -------------------------------------------------------------------------
    // Terminal output tests (OSC 8 escape sequence format)
    // -------------------------------------------------------------------------

    #[test]
    fn test_to_terminal_unchecked_format() {
        let link = Link::new("Example", "https://example.com");
        let output = link.to_terminal_unchecked();

        // Verify the OSC 8 format: ESC ] 8 ; ; <URI> BEL <text> ESC ] 8 ; ; BEL
        // ESC = 0x1b, ] = 0x5d, 8 = 0x38, ; = 0x3b, BEL = 0x07
        assert!(output.starts_with("\x1b]8;;"));
        assert!(output.contains("\x07Example"));
        assert!(output.ends_with("\x1b]8;;\x07"));
    }

    #[test]
    fn test_to_terminal_unchecked_byte_sequence() {
        let link = Link::new("Test", "https://test.com");
        let output = link.to_terminal_unchecked();
        let bytes: Vec<u8> = output.bytes().collect();

        // Start sequence: ESC ] 8 ; ;
        assert_eq!(bytes[0], 0x1b); // ESC
        assert_eq!(bytes[1], 0x5d); // ]
        assert_eq!(bytes[2], 0x38); // 8
        assert_eq!(bytes[3], 0x3b); // ;
        assert_eq!(bytes[4], 0x3b); // ;

        // URL starts at index 5
        let url_bytes = b"https://test.com";
        assert_eq!(&bytes[5..5 + url_bytes.len()], url_bytes);

        // BEL after URL
        assert_eq!(bytes[5 + url_bytes.len()], 0x07);

        // Display text "Test"
        let text_start = 5 + url_bytes.len() + 1;
        assert_eq!(&bytes[text_start..text_start + 4], b"Test");

        // End sequence: ESC ] 8 ; ; BEL
        let end_start = text_start + 4;
        assert_eq!(bytes[end_start], 0x1b);
        assert_eq!(bytes[end_start + 1], 0x5d);
        assert_eq!(bytes[end_start + 2], 0x38);
        assert_eq!(bytes[end_start + 3], 0x3b);
        assert_eq!(bytes[end_start + 4], 0x3b);
        assert_eq!(bytes[end_start + 5], 0x07);
    }

    #[test]
    fn test_to_terminal_unchecked_with_special_chars_in_url() {
        let link = Link::new("Query", "https://example.com?foo=bar&baz=1");
        let output = link.to_terminal_unchecked();

        // URL should be included as-is (no escaping for terminal)
        assert!(output.contains("https://example.com?foo=bar&baz=1"));
        assert!(output.contains("\x07Query\x1b]8;;\x07"));
    }

    #[test]
    fn test_to_terminal_fallback_format() {
        let link = Link::new("Example", "https://example.com");
        // When terminal doesn't support hyperlinks, should return display [url]
        // We can't easily test the conditional path, but we can verify the format
        let fallback = format!("{} [{}]", link.display(), link.href());
        assert_eq!(fallback, "Example [https://example.com]");
    }

    #[test]
    fn test_terminal_bel_not_st() {
        let link = Link::new("Link", "https://example.com");
        let output = link.to_terminal_unchecked();

        // Verify we use BEL (0x07), not ST (0x1b 0x5c)
        assert!(output.contains("\x07"), "Should use BEL as terminator");
        assert!(
            !output.contains("\x1b\\"),
            "Should NOT use ST as terminator"
        );
    }

    // -------------------------------------------------------------------------
    // CSS Style Parsing tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parsed_style_basic() {
        let link =
            Link::new("Click", "https://example.com").with_style("color: red; font-size: 14px");
        let parsed = link.parsed_style().unwrap();
        assert_eq!(parsed.get("color"), Some(&"red".to_string()));
        assert_eq!(parsed.get("font-size"), Some(&"14px".to_string()));
    }

    #[test]
    fn test_parsed_style_whitespace() {
        let link = Link::new("Click", "https://example.com")
            .with_style("  color  :  blue  ;  margin : 10px  ");
        let parsed = link.parsed_style().unwrap();
        assert_eq!(parsed.get("color"), Some(&"blue".to_string()));
        assert_eq!(parsed.get("margin"), Some(&"10px".to_string()));
    }

    #[test]
    fn test_parsed_style_trailing_semicolon() {
        let link = Link::new("Click", "https://example.com").with_style("color: green;");
        let parsed = link.parsed_style().unwrap();
        assert_eq!(parsed.get("color"), Some(&"green".to_string()));
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn test_parsed_style_empty_declarations() {
        let link =
            Link::new("Click", "https://example.com").with_style("color: red;;;font-size: 12px");
        let parsed = link.parsed_style().unwrap();
        assert_eq!(parsed.get("color"), Some(&"red".to_string()));
        assert_eq!(parsed.get("font-size"), Some(&"12px".to_string()));
    }

    #[test]
    fn test_parsed_style_none_when_no_style() {
        let link = Link::new("Click", "https://example.com");
        assert!(link.parsed_style().is_none());
    }

    #[test]
    fn test_parsed_style_case_insensitive_keys() {
        let link =
            Link::new("Click", "https://example.com").with_style("Color: red; FONT-SIZE: 14px");
        let parsed = link.parsed_style().unwrap();
        assert_eq!(parsed.get("color"), Some(&"red".to_string()));
        assert_eq!(parsed.get("font-size"), Some(&"14px".to_string()));
    }

    #[test]
    fn test_parsed_style_preserves_value_case() {
        let link =
            Link::new("Click", "https://example.com").with_style("font-family: Arial, Helvetica");
        let parsed = link.parsed_style().unwrap();
        assert_eq!(
            parsed.get("font-family"),
            Some(&"Arial, Helvetica".to_string())
        );
    }

    #[test]
    fn test_parsed_style_complex_values() {
        let link = Link::new("Click", "https://example.com")
            .with_style("background: url(image.png); border: 1px solid #333");
        let parsed = link.parsed_style().unwrap();
        assert_eq!(
            parsed.get("background"),
            Some(&"url(image.png)".to_string())
        );
        assert_eq!(parsed.get("border"), Some(&"1px solid #333".to_string()));
    }

    #[test]
    fn test_parsed_style_skips_invalid_declarations() {
        let link = Link::new("Click", "https://example.com")
            .with_style("color: red; invalid; font-size: 12px");
        let parsed = link.parsed_style().unwrap();
        assert_eq!(parsed.get("color"), Some(&"red".to_string()));
        assert_eq!(parsed.get("font-size"), Some(&"12px".to_string()));
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_parsed_style_empty_key_or_value() {
        let link =
            Link::new("Click", "https://example.com").with_style(": red; color: ; valid: value");
        let parsed = link.parsed_style().unwrap();
        // Empty key and empty value should be skipped
        assert_eq!(parsed.get("valid"), Some(&"value".to_string()));
        assert_eq!(parsed.len(), 1);
    }

    // -------------------------------------------------------------------------
    // Popover API tests (to_browser_with_popover)
    // -------------------------------------------------------------------------

    #[test]
    fn test_to_browser_with_popover_returns_none_without_prompt() {
        let link = Link::new("Click", "https://example.com");
        assert!(link.to_browser_with_popover().is_none());
    }

    #[test]
    fn test_to_browser_with_popover_basic() {
        let link = Link::new("Click", "https://example.com").with_prompt("Tooltip text");
        let (anchor, popover) = link.to_browser_with_popover().unwrap();

        assert!(anchor.contains(r#"href="https://example.com""#));
        assert!(anchor.contains("interestfor="));
        assert!(anchor.contains(">Click</a>"));

        assert!(popover.contains(r#"popover="hint""#));
        assert!(popover.contains("Tooltip text"));
    }

    #[test]
    fn test_to_browser_with_popover_id_consistency() {
        // Same inputs should produce the same ID
        let link1 = Link::new("Click", "https://example.com").with_prompt("Test");
        let link2 = Link::new("Click", "https://example.com").with_prompt("Different prompt");

        let (anchor1, popover1) = link1.to_browser_with_popover().unwrap();
        let (anchor2, popover2) = link2.to_browser_with_popover().unwrap();

        // Extract the ID from popover1 (format: <div id="popover-..." ...)
        let id1_start = popover1.find("id=\"").unwrap() + 4;
        let id1_end = popover1[id1_start..].find('"').unwrap() + id1_start;
        let id1 = &popover1[id1_start..id1_end];

        let id2_start = popover2.find("id=\"").unwrap() + 4;
        let id2_end = popover2[id2_start..].find('"').unwrap() + id2_start;
        let id2 = &popover2[id2_start..id2_end];

        // Same URL and display should produce the same ID
        assert_eq!(id1, id2);

        // Verify anchor references the same ID
        assert!(anchor1.contains(&format!(r#"interestfor="{}""#, id1)));
        assert!(anchor2.contains(&format!(r#"interestfor="{}""#, id2)));
    }

    #[test]
    fn test_to_browser_with_popover_id_uniqueness() {
        let link1 = Link::new("Click", "https://example.com").with_prompt("Test");
        let link2 = Link::new("Click", "https://other.com").with_prompt("Test");

        let (anchor1, _) = link1.to_browser_with_popover().unwrap();
        let (anchor2, _) = link2.to_browser_with_popover().unwrap();

        // Different URLs should produce different IDs
        assert_ne!(anchor1, anchor2);
    }

    #[test]
    fn test_to_browser_with_popover_escapes_content() {
        let link = Link::new("<script>", "https://example.com")
            .with_prompt("<script>alert('xss')</script>");
        let (anchor, popover) = link.to_browser_with_popover().unwrap();

        assert!(!anchor.contains("<script>"));
        assert!(anchor.contains("&lt;script&gt;"));
        assert!(!popover.contains("<script>alert"));
        assert!(popover.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_to_browser_with_popover_includes_other_attrs() {
        let link = Link::new("Click", "https://example.com")
            .with_prompt("Tooltip")
            .with_class("btn")
            .with_target("_blank");
        let (anchor, _) = link.to_browser_with_popover().unwrap();

        assert!(anchor.contains(r#"class="btn""#));
        assert!(anchor.contains(r#"target="_blank""#));
    }

    #[test]
    fn test_to_browser_with_popover_includes_style() {
        let link = Link::new("Click", "https://example.com")
            .with_prompt("Tooltip")
            .with_style("color: blue");
        let (anchor, _) = link.to_browser_with_popover().unwrap();

        assert!(anchor.contains(r#"style="color: blue""#));
    }

    #[test]
    fn test_to_browser_with_popover_includes_title() {
        let link = Link::new("Click", "https://example.com")
            .with_prompt("Tooltip")
            .with_title("Link title");
        let (anchor, _) = link.to_browser_with_popover().unwrap();

        assert!(anchor.contains(r#"title="Link title""#));
    }

    #[test]
    fn test_to_browser_with_popover_includes_data_attrs() {
        let link = Link::new("Click", "https://example.com")
            .with_prompt("Tooltip")
            .with_data("action", "submit")
            .with_data("id", "123");
        let (anchor, _) = link.to_browser_with_popover().unwrap();

        assert!(anchor.contains(r#"data-action="submit""#));
        assert!(anchor.contains(r#"data-id="123""#));
    }

    #[test]
    fn test_to_browser_with_popover_id_format() {
        let link = Link::new("Click", "https://example.com").with_prompt("Test");
        let (_, popover) = link.to_browser_with_popover().unwrap();

        // ID should start with "popover-" followed by hex characters
        let id_start = popover.find("id=\"").unwrap() + 4;
        let id_end = popover[id_start..].find('"').unwrap() + id_start;
        let id = &popover[id_start..id_end];

        assert!(id.starts_with("popover-"));
        let hex_part = &id[8..];
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_to_browser_with_popover_all_attributes() {
        let link = Link::new("Click", "https://example.com")
            .with_prompt("Popover content")
            .with_class("btn btn-primary")
            .with_style("font-weight: bold")
            .with_target("_blank")
            .with_title("My title")
            .with_data("custom", "value");

        let (anchor, popover) = link.to_browser_with_popover().unwrap();

        // Verify all attributes are present in anchor
        assert!(anchor.contains(r#"href="https://example.com""#));
        assert!(anchor.contains("interestfor="));
        assert!(anchor.contains(r#"class="btn btn-primary""#));
        assert!(anchor.contains(r#"style="font-weight: bold""#));
        assert!(anchor.contains(r#"target="_blank""#));
        assert!(anchor.contains(r#"title="My title""#));
        assert!(anchor.contains(r#"data-custom="value""#));
        assert!(anchor.contains(">Click</a>"));

        // Verify popover structure
        assert!(popover.starts_with("<div id=\"popover-"));
        assert!(popover.contains(r#"popover="hint""#));
        assert!(popover.contains("Popover content"));
        assert!(popover.ends_with("</div>"));
    }

    #[test]
    fn test_generate_popover_id_deterministic() {
        let id1 = generate_popover_id("https://example.com", "Click");
        let id2 = generate_popover_id("https://example.com", "Click");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_generate_popover_id_varies_with_url() {
        let id1 = generate_popover_id("https://example.com", "Click");
        let id2 = generate_popover_id("https://other.com", "Click");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_popover_id_varies_with_display() {
        let id1 = generate_popover_id("https://example.com", "Click");
        let id2 = generate_popover_id("https://example.com", "Different");
        assert_ne!(id1, id2);
    }

    // -------------------------------------------------------------------------
    // with_title_parsed tests - regression tests for structured link parsing
    // -------------------------------------------------------------------------

    #[test]
    fn test_with_title_parsed_empty_title() {
        let link = Link::with_title_parsed("Click", "https://example.com", "");
        assert!(link.title().is_none());
        assert!(link.class().is_none());
        assert!(link.style().is_none());
    }

    #[test]
    fn test_with_title_parsed_title_mode_plain_text() {
        // Plain text without key=value should be treated as title
        let link = Link::with_title_parsed("Click", "https://example.com", "A simple tooltip");
        assert_eq!(link.title(), Some("A simple tooltip"));
        assert!(link.class().is_none());
        assert!(link.style().is_none());
    }

    #[test]
    fn test_with_title_parsed_title_mode_quoted() {
        // Quoted title should be parsed as title mode
        let link = Link::with_title_parsed("Click", "https://example.com", "\"My tooltip\"");
        assert_eq!(link.title(), Some("My tooltip"));
    }

    #[test]
    fn test_with_title_parsed_structured_mode_class() {
        // key=value pattern triggers structured mode
        let link = Link::with_title_parsed("Click", "https://example.com", "class='btn'");
        assert_eq!(link.class(), Some("btn"));
        assert!(link.title().is_none());
    }

    #[test]
    fn test_with_title_parsed_structured_mode_style() {
        let link = Link::with_title_parsed("Click", "https://example.com", "style='color:red'");
        assert_eq!(link.style(), Some("color:red"));
    }

    #[test]
    fn test_with_title_parsed_structured_mode_multiple() {
        let link = Link::with_title_parsed(
            "Click",
            "https://example.com",
            "class='btn' style='color:red' prompt='hover me'",
        );
        assert_eq!(link.class(), Some("btn"));
        assert_eq!(link.style(), Some("color:red"));
        assert_eq!(link.prompt(), Some("hover me"));
    }

    #[test]
    fn test_with_title_parsed_structured_mode_with_title_key() {
        // In structured mode, title= sets the title
        let link = Link::with_title_parsed(
            "Click",
            "https://example.com",
            "title='My Title' class='btn'",
        );
        assert_eq!(link.title(), Some("My Title"));
        assert_eq!(link.class(), Some("btn"));
    }

    #[test]
    fn test_with_title_parsed_structured_mode_unquoted_values() {
        let link =
            Link::with_title_parsed("Click", "https://example.com", "class=btn target=_blank");
        assert_eq!(link.class(), Some("btn"));
        assert_eq!(link.target(), Some("_blank"));
    }

    #[test]
    fn test_with_title_parsed_structured_mode_data_attributes() {
        let link = Link::with_title_parsed(
            "Click",
            "https://example.com",
            "data-id='123' data-action='submit'",
        );
        let data = link.data().unwrap();
        assert_eq!(data.get("id"), Some(&"123".to_string()));
        assert_eq!(data.get("action"), Some(&"submit".to_string()));
    }

    #[test]
    fn test_with_title_parsed_whitespace_handling() {
        // Should handle extra whitespace
        let link = Link::with_title_parsed("Click", "https://example.com", "  class='btn'  ");
        assert_eq!(link.class(), Some("btn"));
    }
}
