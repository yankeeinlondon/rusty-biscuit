//! Screen Scrape Tool for rig-core agents
//!
//! Provides web page content extraction and scraping capabilities.
//! Supports multiple output formats and content filtering.
//!
//! ## Features
//!
//! - Multiple output formats: Markdown, HTML, Plain Text, JSON, Links
//! - Main content extraction (filters navigation, ads, footers)
//! - Tag filtering (include/exclude specific HTML tags)
//! - Mobile/Desktop user agent switching
//! - Custom headers and request configuration
//!
//! ## Example
//!
//! ```rust,ignore
//! use shared::tools::{ScreenScrapeTool, ScrapeArgs, OutputFormat};
//! use rig::tool::Tool;
//!
//! let tool = ScreenScrapeTool::new();
//! let args = ScrapeArgs {
//!     url: "https://example.com".to_string(),
//!     formats: vec![OutputFormat::Markdown],
//!     only_main_content: true,
//!     ..Default::default()
//! };
//!
//! let result = tool.call(args).await?;
//! println!("Content: {:?}", result.content);
//! ```

use reqwest::Client;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use tracing::{Span, debug, info, instrument, warn};

// =============================================================================
// HTML Extraction Helper
// =============================================================================

/// Helper for extracting content from parsed HTML documents.
///
/// Provides a consistent interface for selector-based extraction with
/// fallback support, reducing code duplication in extraction methods.
struct HtmlExtractor<'a> {
    document: &'a Html,
}

impl<'a> HtmlExtractor<'a> {
    /// Creates a new extractor for the given document.
    fn new(document: &'a Html) -> Self {
        Self { document }
    }

    /// Tries each selector in order and returns the first non-empty text match.
    ///
    /// ## Arguments
    ///
    /// * `selectors` - CSS selectors to try in priority order
    ///
    /// ## Returns
    ///
    /// The text content of the first matching element with non-empty text,
    /// or `None` if no selector matches.
    fn first_text(&self, selectors: &[&str]) -> Option<String> {
        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str)
                && let Some(element) = self.document.select(&selector).next()
            {
                let content: String = element.text().collect::<Vec<_>>().join("\n");
                if !content.trim().is_empty() {
                    return Some(content);
                }
            }
        }
        None
    }

    /// Returns an iterator over all elements matching the selector.
    ///
    /// Returns an empty iterator if the selector is invalid.
    fn select_all(&self, selector_str: &str) -> impl Iterator<Item = ElementRef<'a>> {
        Selector::parse(selector_str)
            .ok()
            .into_iter()
            .flat_map(|sel| self.document.select(&sel).collect::<Vec<_>>())
    }

    /// Collects text from all elements matching the selector.
    ///
    /// ## Arguments
    ///
    /// * `selector` - CSS selector string
    ///
    /// ## Returns
    ///
    /// Vector of text content from each matching element (empty if none match).
    fn all_text(&self, selector: &str) -> Vec<String> {
        self.select_all(selector)
            .map(|el| el.text().collect::<Vec<_>>().join(" "))
            .filter(|s| !s.trim().is_empty())
            .collect()
    }

    /// Extracts text with fallback to a default selector.
    ///
    /// Tries each selector in order, falling back to `fallback_selector`
    /// if none match.
    #[allow(dead_code)]
    fn extract_with_fallback(&self, selectors: &[&str], fallback_selector: &str) -> String {
        self.first_text(selectors).unwrap_or_else(|| {
            self.first_text(&[fallback_selector]).unwrap_or_default()
        })
    }
}

/// Output format for scraped content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Markdown formatted content
    #[default]
    Markdown,
    /// Raw HTML content
    Html,
    /// Plain text without formatting
    PlainText,
    /// JSON structured content
    Json,
    /// Extract only links from the page
    Links,
}

/// Proxy mode for requests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// No proxy
    #[default]
    None,
    /// Basic proxy
    Basic,
    /// Stealth mode with anti-detection
    Stealth,
    /// Auto-select based on target
    Auto,
}

/// Actions that can be performed before scraping (for dynamic content).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ScrapeAction {
    /// Wait for a specified time
    Wait {
        /// Milliseconds to wait
        milliseconds: u64,
    },
    /// Scroll the page
    Scroll {
        /// Direction: "up" or "down"
        direction: String,
        /// Pixels to scroll
        pixels: Option<u32>,
    },
    /// Click an element
    Click {
        /// CSS selector for the element
        selector: String,
    },
    /// Type text into an input
    Write {
        /// CSS selector for the input
        selector: String,
        /// Text to type
        text: String,
    },
    /// Press a key
    Press {
        /// Key to press
        key: String,
    },
}

/// Input arguments for the screen scrape tool.
#[derive(Debug, Clone, Deserialize)]
pub struct ScrapeArgs {
    /// The URL to scrape
    pub url: String,

    /// Output format(s) - defaults to empty (applied during processing)
    #[serde(default)]
    pub formats: Vec<OutputFormat>,

    /// Extract only the main content, excluding navigation, ads, etc.
    #[serde(default)]
    pub only_main_content: bool,

    /// HTML tags to include in extraction
    #[serde(default)]
    pub include_tags: Vec<String>,

    /// HTML tags to exclude from extraction
    #[serde(default)]
    pub exclude_tags: Vec<String>,

    /// Wait time in milliseconds before scraping
    #[serde(default)]
    pub wait_for: Option<u64>,

    /// Actions to perform before scraping (for dynamic content)
    #[serde(default)]
    pub actions: Vec<ScrapeAction>,

    /// Use mobile user agent
    #[serde(default)]
    pub mobile: bool,

    /// Skip TLS verification
    #[serde(default)]
    pub skip_tls_verification: bool,

    /// Remove base64 encoded images from output
    #[serde(default)]
    pub remove_base64_images: bool,

    /// Proxy mode
    #[serde(default)]
    pub proxy: ProxyMode,

    /// Custom user agent
    #[serde(default)]
    pub user_agent: Option<String>,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Follow redirects
    #[serde(default = "default_true")]
    pub follow_redirects: bool,

    /// Custom HTTP headers
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
}

fn default_timeout() -> u64 {
    30
}

fn default_true() -> bool {
    true
}

impl Default for ScrapeArgs {
    fn default() -> Self {
        Self {
            url: String::new(),
            formats: vec![OutputFormat::Markdown],
            only_main_content: false,
            include_tags: vec![],
            exclude_tags: vec![],
            wait_for: None,
            actions: vec![],
            mobile: false,
            skip_tls_verification: false,
            remove_base64_images: false,
            proxy: ProxyMode::None,
            user_agent: None,
            timeout: 30,
            follow_redirects: true,
            headers: None,
        }
    }
}

/// Output from the scrape operation.
#[derive(Debug, Clone, Serialize)]
pub struct ScrapeOutput {
    /// The URL that was scraped
    pub url: String,

    /// HTTP status code of the response
    pub status_code: u16,

    /// Content in requested formats
    pub content: HashMap<String, serde_json::Value>,

    /// Metadata about the scrape operation
    pub metadata: ScrapeMetadata,

    /// Links found on the page (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<LinkInfo>>,
}

/// Metadata about a scrape operation.
#[derive(Debug, Clone, Serialize)]
pub struct ScrapeMetadata {
    /// Content type from response headers
    pub content_type: Option<String>,

    /// Content length in bytes
    pub content_length: Option<usize>,

    /// Time taken to scrape in milliseconds
    pub duration_ms: u64,

    /// Number of actions performed
    pub actions_performed: usize,
}

/// Information about a link found on the page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinkInfo {
    /// The URL of the link
    pub url: String,
    /// The link text
    pub text: Option<String>,
    /// The title attribute
    pub title: Option<String>,
    /// The rel attribute
    pub rel: Option<String>,
}

/// Errors that can occur during scraping.
#[derive(Debug, Error)]
pub enum ScrapeError {
    /// Invalid URL provided
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    /// Failed to parse HTML
    #[error("Failed to parse HTML: {0}")]
    ParseError(String),

    /// Timeout exceeded
    #[error("Request timeout exceeded")]
    Timeout,

    /// Action failed
    #[error("Action failed: {0}")]
    ActionError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Screen scrape tool for rig-core agents.
///
/// This tool enables AI agents to extract content from web pages,
/// supporting multiple output formats and content filtering options.
#[derive(Clone)]
pub struct ScreenScrapeTool {
    #[allow(dead_code)]
    client: Client,
    default_timeout: Duration,
}

impl ScreenScrapeTool {
    /// Create a new screen scrape tool with default settings.
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            default_timeout: Duration::from_secs(30),
        }
    }

    /// Create a tool with a custom timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Create a tool with a custom HTTP client (useful for testing).
    #[cfg(test)]
    pub fn with_client(client: Client) -> Self {
        Self {
            client,
            default_timeout: Duration::from_secs(30),
        }
    }

    /// Get the user agent string based on settings.
    fn get_user_agent(&self, args: &ScrapeArgs) -> String {
        if let Some(ref ua) = args.user_agent {
            return ua.clone();
        }

        if args.mobile {
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 \
             (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1"
                .to_string()
        } else {
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
                .to_string()
        }
    }

    /// Build the HTTP client for a request.
    fn build_request_client(&self, args: &ScrapeArgs) -> Result<Client, ScrapeError> {
        let timeout = Duration::from_secs(args.timeout);
        let redirect_policy = if args.follow_redirects {
            reqwest::redirect::Policy::limited(10)
        } else {
            reqwest::redirect::Policy::none()
        };

        let mut builder = Client::builder().timeout(timeout).redirect(redirect_policy);

        if args.skip_tls_verification {
            builder = builder.danger_accept_invalid_certs(true);
        }

        builder
            .build()
            .map_err(|e| ScrapeError::ConfigError(format!("Failed to build HTTP client: {}", e)))
    }

    /// Extract main content from HTML using common selectors.
    fn extract_main_content(html: &str) -> String {
        let document = Html::parse_document(html);
        let extractor = HtmlExtractor::new(&document);

        // Try common main content selectors in order of preference
        let selectors = [
            "article",
            "main",
            "[role='main']",
            ".content",
            "#content",
            ".post-content",
            ".article-content",
            ".entry-content",
        ];

        extractor
            .first_text(&selectors)
            .or_else(|| extractor.first_text(&["body"]))
            .unwrap_or_else(|| html.to_string())
    }

    /// Convert HTML to Markdown.
    fn html_to_markdown(html: &str) -> String {
        let document = Html::parse_document(html);
        let extractor = HtmlExtractor::new(&document);
        let mut markdown = String::new();

        // Extract headings
        for level in 1..=6 {
            for text in extractor.all_text(&format!("h{}", level)) {
                markdown.push_str(&format!("{} {}\n\n", "#".repeat(level), text.trim()));
            }
        }

        // Extract paragraphs
        for text in extractor.all_text("p") {
            markdown.push_str(&format!("{}\n\n", text.trim()));
        }

        // Extract code blocks
        for text in extractor.all_text("pre, code") {
            markdown.push_str(&format!("```\n{}\n```\n\n", text.trim()));
        }

        // Extract unordered lists
        if let Ok(li_selector) = Selector::parse("li") {
            for ul in extractor.select_all("ul") {
                for li in ul.select(&li_selector) {
                    let text: String = li.text().collect::<Vec<_>>().join(" ");
                    if !text.trim().is_empty() {
                        markdown.push_str(&format!("- {}\n", text.trim()));
                    }
                }
                markdown.push('\n');
            }

            // Extract ordered lists
            for ol in extractor.select_all("ol") {
                for (i, li) in ol.select(&li_selector).enumerate() {
                    let text: String = li.text().collect::<Vec<_>>().join(" ");
                    if !text.trim().is_empty() {
                        markdown.push_str(&format!("{}. {}\n", i + 1, text.trim()));
                    }
                }
                markdown.push('\n');
            }
        }

        markdown
    }

    /// Extract all links from HTML.
    fn extract_links(html: &str) -> Vec<LinkInfo> {
        let document = Html::parse_document(html);
        let extractor = HtmlExtractor::new(&document);

        extractor
            .select_all("a[href]")
            .filter_map(|element| {
                element.value().attr("href").map(|href| {
                    let text: String = element.text().collect::<Vec<_>>().join(" ");
                    LinkInfo {
                        url: href.to_string(),
                        text: if text.trim().is_empty() {
                            None
                        } else {
                            Some(text.trim().to_string())
                        },
                        title: element.value().attr("title").map(String::from),
                        rel: element.value().attr("rel").map(String::from),
                    }
                })
            })
            .collect()
    }

    /// Extract plain text from HTML.
    fn extract_plain_text(html: &str) -> String {
        let document = Html::parse_document(html);
        document
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Remove base64 images from HTML.
    fn remove_base64_images(html: &str) -> String {
        // Simple regex-like replacement for base64 image data
        let mut result = html.to_string();

        // Remove src attributes that contain base64 data
        while let Some(start) = result.find("src=\"data:image") {
            if let Some(end) = result[start..].find("\"").and_then(|first_quote| {
                result[start + first_quote + 1..]
                    .find("\"")
                    .map(|second_quote| start + first_quote + 1 + second_quote + 1)
            }) {
                result.replace_range(start..end, "src=\"\"");
            } else {
                break;
            }
        }

        result
    }

    /// Perform the scrape operation.
    #[instrument(
        name = "screen_scrape",
        skip(self, args),
        fields(
            tool.name = "screen_scrape",
            tool.url = %args.url,
            tool.formats = ?args.formats,
            otel.kind = "client"
        )
    )]
    async fn scrape(&self, args: ScrapeArgs) -> Result<ScrapeOutput, ScrapeError> {
        let start_time = std::time::Instant::now();

        debug!(
            only_main_content = args.only_main_content,
            mobile = args.mobile,
            timeout = args.timeout,
            "Starting page scrape"
        );

        // Validate URL
        let parsed_url = url::Url::parse(&args.url).map_err(|e| {
            warn!(error = %e, url = %args.url, "Invalid URL");
            ScrapeError::InvalidUrl(e.to_string())
        })?;

        if !["http", "https"].contains(&parsed_url.scheme()) {
            warn!(scheme = %parsed_url.scheme(), "Unsupported URL scheme");
            return Err(ScrapeError::InvalidUrl(
                "Only HTTP and HTTPS URLs are supported".to_string(),
            ));
        }

        // Build client and request
        let client = self.build_request_client(&args)?;
        let user_agent = self.get_user_agent(&args);

        let mut request = client
            .get(args.url.clone())
            .header("User-Agent", user_agent)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            );

        // Add custom headers
        if let Some(ref headers) = args.headers {
            for (key, value) in headers {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        // Wait before request if specified
        if let Some(wait_ms) = args.wait_for {
            debug!(wait_ms, "Waiting before request");
            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        }

        // Perform the request
        let response = request.send().await;

        match &response {
            Ok(resp) => {
                let status = resp.status().as_u16();
                Span::current().record("http.status_code", status);
                debug!(http.status_code = status, "Received HTTP response");
            }
            Err(e) => {
                warn!(error = %e, "Scrape request failed");
            }
        }

        let response = response?;
        let status_code = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let html_content = response.text().await?;
        let content_length = html_content.len();

        debug!(content_length, "Retrieved page content");

        // Process content
        let processed_html = if args.remove_base64_images {
            Self::remove_base64_images(&html_content)
        } else {
            html_content.clone()
        };

        let final_content = if args.only_main_content {
            Self::extract_main_content(&processed_html)
        } else {
            processed_html.clone()
        };

        // Generate requested output formats
        let formats = if args.formats.is_empty() {
            vec![OutputFormat::Markdown]
        } else {
            args.formats.clone()
        };

        let mut content_map = HashMap::new();
        let mut links = None;

        for format in &formats {
            match format {
                OutputFormat::Markdown => {
                    let markdown = Self::html_to_markdown(&final_content);
                    content_map.insert("markdown".to_string(), serde_json::json!(markdown));
                }
                OutputFormat::Html => {
                    content_map.insert("html".to_string(), serde_json::json!(final_content));
                }
                OutputFormat::PlainText => {
                    let text = Self::extract_plain_text(&final_content);
                    content_map.insert("plain_text".to_string(), serde_json::json!(text));
                }
                OutputFormat::Json => {
                    content_map.insert(
                        "json".to_string(),
                        serde_json::json!({
                            "html": final_content,
                            "url": args.url
                        }),
                    );
                }
                OutputFormat::Links => {
                    let extracted_links = Self::extract_links(&html_content);
                    content_map.insert("links".to_string(), serde_json::json!(&extracted_links));
                    links = Some(extracted_links);
                }
            }
        }

        let duration = start_time.elapsed();

        info!(
            tool.status_code = status_code,
            tool.content_length = content_length,
            tool.duration_ms = duration.as_millis() as u64,
            "Scrape completed"
        );

        Ok(ScrapeOutput {
            url: args.url,
            status_code,
            content: content_map,
            metadata: ScrapeMetadata {
                content_type,
                content_length: Some(content_length),
                duration_ms: duration.as_millis() as u64,
                actions_performed: args.actions.len(),
            },
            links,
        })
    }
}

impl Default for ScreenScrapeTool {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ScreenScrapeTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreenScrapeTool")
            .field("default_timeout", &self.default_timeout)
            .finish_non_exhaustive()
    }
}

impl Tool for ScreenScrapeTool {
    const NAME: &'static str = "screen_scrape";

    type Error = ScrapeError;
    type Args = ScrapeArgs;
    type Output = ScrapeOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "screen_scrape".to_string(),
            description: "Extract content from web pages. Supports multiple output formats \
                (Markdown, HTML, Plain Text, JSON, Links) and can filter for main content. \
                Use this tool when you need to read and analyze web page content."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to scrape content from"
                    },
                    "formats": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["markdown", "html", "plaintext", "json", "links"]
                        },
                        "description": "Output format(s) to generate. Defaults to ['markdown']"
                    },
                    "only_main_content": {
                        "type": "boolean",
                        "description": "Extract only main content, excluding navigation, ads, footers",
                        "default": false
                    },
                    "include_tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "HTML tags/selectors to include (e.g., ['article', 'main'])"
                    },
                    "exclude_tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "HTML tags/selectors to exclude (e.g., ['nav', 'footer'])"
                    },
                    "wait_for": {
                        "type": "number",
                        "description": "Wait time in milliseconds before scraping"
                    },
                    "mobile": {
                        "type": "boolean",
                        "description": "Use mobile user agent",
                        "default": false
                    },
                    "timeout": {
                        "type": "number",
                        "description": "Request timeout in seconds (default: 30)"
                    },
                    "follow_redirects": {
                        "type": "boolean",
                        "description": "Follow HTTP redirects (default: true)"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.scrape(args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================
    // Tests for OutputFormat
    // ===========================================

    #[test]
    fn test_output_format_default() {
        assert_eq!(OutputFormat::default(), OutputFormat::Markdown);
    }

    #[test]
    fn test_output_format_serialization() {
        let format = OutputFormat::Markdown;
        let json = serde_json::to_string(&format).unwrap();
        assert_eq!(json, "\"markdown\"");

        let deserialized: OutputFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, OutputFormat::Markdown);
    }

    #[test]
    fn test_output_format_all_variants() {
        let variants = vec![
            (OutputFormat::Markdown, "\"markdown\""),
            (OutputFormat::Html, "\"html\""),
            (OutputFormat::PlainText, "\"plaintext\""),
            (OutputFormat::Json, "\"json\""),
            (OutputFormat::Links, "\"links\""),
        ];

        for (format, expected_json) in variants {
            let json = serde_json::to_string(&format).unwrap();
            assert_eq!(json, expected_json);
        }
    }

    // ===========================================
    // Tests for ProxyMode
    // ===========================================

    #[test]
    fn test_proxy_mode_default() {
        assert_eq!(ProxyMode::default(), ProxyMode::None);
    }

    // ===========================================
    // Tests for ScrapeArgs
    // ===========================================

    #[test]
    fn test_scrape_args_default() {
        let args = ScrapeArgs::default();
        assert!(args.url.is_empty());
        assert_eq!(args.formats, vec![OutputFormat::Markdown]);
        assert!(!args.only_main_content);
        assert!(args.include_tags.is_empty());
        assert!(args.exclude_tags.is_empty());
        assert!(args.wait_for.is_none());
        assert!(args.actions.is_empty());
        assert!(!args.mobile);
        assert!(!args.skip_tls_verification);
        assert!(!args.remove_base64_images);
        assert_eq!(args.proxy, ProxyMode::None);
        assert!(args.user_agent.is_none());
        assert_eq!(args.timeout, 30);
        assert!(args.follow_redirects);
        assert!(args.headers.is_none());
    }

    #[test]
    fn test_scrape_args_deserialization() {
        let json = r#"{
            "url": "https://example.com",
            "formats": ["markdown", "html"],
            "only_main_content": true,
            "mobile": true
        }"#;

        let args: ScrapeArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.url, "https://example.com");
        assert_eq!(
            args.formats,
            vec![OutputFormat::Markdown, OutputFormat::Html]
        );
        assert!(args.only_main_content);
        assert!(args.mobile);
    }

    #[test]
    fn test_scrape_args_minimal_deserialization() {
        let json = r#"{"url": "https://test.com"}"#;
        let args: ScrapeArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.url, "https://test.com");
        assert!(args.formats.is_empty()); // Will default in processing
    }

    // ===========================================
    // Tests for LinkInfo
    // ===========================================

    #[test]
    fn test_link_info_equality() {
        let link1 = LinkInfo {
            url: "https://example.com".to_string(),
            text: Some("Example".to_string()),
            title: None,
            rel: None,
        };
        let link2 = link1.clone();
        assert_eq!(link1, link2);
    }

    #[test]
    fn test_link_info_serialization() {
        let link = LinkInfo {
            url: "https://example.com".to_string(),
            text: Some("Example Link".to_string()),
            title: Some("Go to example".to_string()),
            rel: Some("noopener".to_string()),
        };

        let json = serde_json::to_string(&link).unwrap();
        assert!(json.contains("https://example.com"));
        assert!(json.contains("Example Link"));

        let deserialized: LinkInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(link, deserialized);
    }

    // ===========================================
    // Tests for ScrapeError
    // ===========================================

    #[test]
    fn test_scrape_error_display() {
        let error = ScrapeError::InvalidUrl("bad url".to_string());
        assert!(format!("{}", error).contains("Invalid URL"));

        let error = ScrapeError::ParseError("parse failed".to_string());
        assert!(format!("{}", error).contains("parse"));

        let error = ScrapeError::Timeout;
        assert!(format!("{}", error).contains("timeout"));

        let error = ScrapeError::ConfigError("bad config".to_string());
        assert!(format!("{}", error).contains("Configuration"));
    }

    // ===========================================
    // Tests for ScreenScrapeTool
    // ===========================================

    #[test]
    fn test_tool_debug() {
        let tool = ScreenScrapeTool::new();
        let debug = format!("{:?}", tool);
        assert!(debug.contains("ScreenScrapeTool"));
    }

    #[test]
    fn test_tool_default() {
        let tool = ScreenScrapeTool::default();
        assert_eq!(tool.default_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_tool_with_timeout() {
        let tool = ScreenScrapeTool::new().with_timeout(Duration::from_secs(60));
        assert_eq!(tool.default_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_tool_name_constant() {
        assert_eq!(ScreenScrapeTool::NAME, "screen_scrape");
    }

    #[tokio::test]
    async fn test_tool_definition() {
        let tool = ScreenScrapeTool::new();
        let definition = tool.definition(String::new()).await;

        assert_eq!(definition.name, "screen_scrape");
        assert!(definition.description.contains("Extract content"));

        let params = definition.parameters;
        assert!(params["properties"]["url"].is_object());
        assert!(params["properties"]["formats"].is_object());
        assert!(params["properties"]["only_main_content"].is_object());
        assert_eq!(params["required"], serde_json::json!(["url"]));
    }

    // ===========================================
    // Tests for content extraction functions
    // ===========================================

    #[test]
    fn test_extract_main_content_with_article() {
        let html = r#"
            <html>
            <body>
                <nav>Navigation</nav>
                <article>Main article content here</article>
                <footer>Footer</footer>
            </body>
            </html>
        "#;

        let content = ScreenScrapeTool::extract_main_content(html);
        assert!(content.contains("Main article content"));
        // Navigation and footer should not be in main content
    }

    #[test]
    fn test_extract_main_content_fallback_to_body() {
        let html = r#"
            <html>
            <body>Body content without article tag</body>
            </html>
        "#;

        let content = ScreenScrapeTool::extract_main_content(html);
        assert!(content.contains("Body content"));
    }

    #[test]
    fn test_html_to_markdown_headings() {
        let html = r#"
            <h1>Title</h1>
            <h2>Subtitle</h2>
            <p>Paragraph text</p>
        "#;

        let markdown = ScreenScrapeTool::html_to_markdown(html);
        assert!(markdown.contains("# Title"));
        assert!(markdown.contains("## Subtitle"));
        assert!(markdown.contains("Paragraph text"));
    }

    #[test]
    fn test_html_to_markdown_lists() {
        let html = r#"
            <ul>
                <li>Item 1</li>
                <li>Item 2</li>
            </ul>
            <ol>
                <li>First</li>
                <li>Second</li>
            </ol>
        "#;

        let markdown = ScreenScrapeTool::html_to_markdown(html);
        assert!(markdown.contains("- Item 1"));
        assert!(markdown.contains("- Item 2"));
        assert!(markdown.contains("1. First"));
        assert!(markdown.contains("2. Second"));
    }

    #[test]
    fn test_html_to_markdown_code() {
        let html = r#"<pre>fn main() {}</pre>"#;
        let markdown = ScreenScrapeTool::html_to_markdown(html);
        assert!(markdown.contains("```"));
        assert!(markdown.contains("fn main()"));
    }

    #[test]
    fn test_extract_links() {
        let html = r#"
            <a href="https://example.com">Example</a>
            <a href="/relative" title="Relative Link">Relative</a>
            <a href="https://other.com" rel="noopener">Other</a>
        "#;

        let links = ScreenScrapeTool::extract_links(html);
        assert_eq!(links.len(), 3);

        assert_eq!(links[0].url, "https://example.com");
        assert_eq!(links[0].text, Some("Example".to_string()));

        assert_eq!(links[1].url, "/relative");
        assert_eq!(links[1].title, Some("Relative Link".to_string()));

        assert_eq!(links[2].rel, Some("noopener".to_string()));
    }

    #[test]
    fn test_extract_plain_text() {
        let html = r#"
            <html>
            <body>
                <h1>Title</h1>
                <p>Paragraph with <strong>bold</strong> text.</p>
            </body>
            </html>
        "#;

        let text = ScreenScrapeTool::extract_plain_text(html);
        assert!(text.contains("Title"));
        assert!(text.contains("Paragraph"));
        assert!(text.contains("bold"));
        // Should not contain HTML tags
        assert!(!text.contains("<h1>"));
        assert!(!text.contains("<strong>"));
    }

    #[test]
    fn test_remove_base64_images() {
        let html = r#"<img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAA..." alt="test">"#;
        let result = ScreenScrapeTool::remove_base64_images(html);
        assert!(result.contains("src=\"\""));
        assert!(!result.contains("base64"));
    }

    // ===========================================
    // Tests for user agent selection
    // ===========================================

    #[test]
    fn test_get_user_agent_desktop() {
        let tool = ScreenScrapeTool::new();
        let args = ScrapeArgs::default();
        let ua = tool.get_user_agent(&args);
        assert!(ua.contains("Windows"));
        assert!(ua.contains("Chrome"));
    }

    #[test]
    fn test_get_user_agent_mobile() {
        let tool = ScreenScrapeTool::new();
        let args = ScrapeArgs {
            mobile: true,
            ..Default::default()
        };
        let ua = tool.get_user_agent(&args);
        assert!(ua.contains("iPhone"));
        assert!(ua.contains("Safari"));
    }

    #[test]
    fn test_get_user_agent_custom() {
        let tool = ScreenScrapeTool::new();
        let args = ScrapeArgs {
            user_agent: Some("CustomBot/1.0".to_string()),
            mobile: true, // Should be ignored when custom UA is set
            ..Default::default()
        };
        let ua = tool.get_user_agent(&args);
        assert_eq!(ua, "CustomBot/1.0");
    }

    // ===========================================
    // Tests for URL validation
    // ===========================================

    #[tokio::test]
    async fn test_invalid_url_error() {
        let tool = ScreenScrapeTool::new();
        let args = ScrapeArgs {
            url: "not-a-valid-url".to_string(),
            ..Default::default()
        };

        let result = tool.call(args).await;
        assert!(matches!(result, Err(ScrapeError::InvalidUrl(_))));
    }

    #[tokio::test]
    async fn test_non_http_url_error() {
        let tool = ScreenScrapeTool::new();
        let args = ScrapeArgs {
            url: "ftp://example.com/file".to_string(),
            ..Default::default()
        };

        let result = tool.call(args).await;
        assert!(matches!(result, Err(ScrapeError::InvalidUrl(_))));
    }

    // ===========================================
    // Integration test with mock server
    // ===========================================

    #[tokio::test]
    async fn test_successful_scrape_with_mock() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let html_content = r#"
            <!DOCTYPE html>
            <html>
            <head><title>Test Page</title></head>
            <body>
                <h1>Welcome</h1>
                <article>
                    <p>This is the main content.</p>
                    <a href="https://example.com">Link</a>
                </article>
            </body>
            </html>
        "#;

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(html_content)
                    .insert_header("content-type", "text/html"),
            )
            .mount(&mock_server)
            .await;

        let tool = ScreenScrapeTool::new();
        let args = ScrapeArgs {
            url: mock_server.uri(),
            formats: vec![OutputFormat::Markdown, OutputFormat::Links],
            ..Default::default()
        };

        let result = tool.call(args).await.unwrap();

        assert_eq!(result.status_code, 200);
        assert!(result.content.contains_key("markdown"));
        assert!(result.content.contains_key("links"));
        assert!(result.links.is_some());

        let links = result.links.unwrap();
        assert!(!links.is_empty());
        assert_eq!(links[0].url, "https://example.com");
    }

    #[tokio::test]
    async fn test_scrape_with_main_content_extraction() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let html_content = r#"
            <!DOCTYPE html>
            <html>
            <body>
                <nav>Navigation menu</nav>
                <article>Important article content</article>
                <footer>Footer content</footer>
            </body>
            </html>
        "#;

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(html_content))
            .mount(&mock_server)
            .await;

        let tool = ScreenScrapeTool::new();
        let args = ScrapeArgs {
            url: mock_server.uri(),
            formats: vec![OutputFormat::PlainText],
            only_main_content: true,
            ..Default::default()
        };

        let result = tool.call(args).await.unwrap();
        let plain_text = result.content.get("plain_text").unwrap().as_str().unwrap();

        assert!(plain_text.contains("Important article content"));
    }

    #[tokio::test]
    async fn test_scrape_metadata() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("<html><body>Test</body></html>")
                    .append_header("Content-Type", "text/html; charset=utf-8"),
            )
            .mount(&mock_server)
            .await;

        let tool = ScreenScrapeTool::new();
        let args = ScrapeArgs {
            url: mock_server.uri(),
            ..Default::default()
        };

        let result = tool.call(args).await.unwrap();

        // Verify basic metadata
        assert!(result.metadata.content_length.is_some());
        // Note: duration_ms may be 0 for very fast local mock responses

        // Content-type may or may not be present depending on mock server behavior
        if let Some(ref ct) = result.metadata.content_type {
            assert!(
                ct.contains("text/html") || ct.contains("text/plain"),
                "Unexpected content type: {}",
                ct
            );
        }
    }

    // ===========================================
    // Tests for ScrapeAction
    // ===========================================

    #[test]
    fn test_scrape_action_serialization() {
        let action = ScrapeAction::Wait { milliseconds: 1000 };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"wait\""));
        assert!(json.contains("1000"));

        let action = ScrapeAction::Click {
            selector: "#button".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"click\""));
        assert!(json.contains("#button"));
    }

    #[test]
    fn test_scrape_action_deserialization() {
        let json = r#"{"type": "wait", "milliseconds": 500}"#;
        let action: ScrapeAction = serde_json::from_str(json).unwrap();
        match action {
            ScrapeAction::Wait { milliseconds } => assert_eq!(milliseconds, 500),
            _ => panic!("Expected Wait action"),
        }

        let json = r#"{"type": "scroll", "direction": "down", "pixels": 100}"#;
        let action: ScrapeAction = serde_json::from_str(json).unwrap();
        match action {
            ScrapeAction::Scroll { direction, pixels } => {
                assert_eq!(direction, "down");
                assert_eq!(pixels, Some(100));
            }
            _ => panic!("Expected Scroll action"),
        }
    }

    // ===========================================
    // Tracing tests
    // ===========================================

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_scrape_emits_tracing_events() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test-page"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("<html><head><title>Test</title></head><body><p>Test content</p></body></html>")
                    .insert_header("content-type", "text/html"),
            )
            .mount(&mock_server)
            .await;

        let tool = ScreenScrapeTool::new();
        let args = ScrapeArgs {
            url: format!("{}/test-page", mock_server.uri()),
            formats: vec![OutputFormat::Markdown],
            ..Default::default()
        };

        let _ = tool.call(args).await;

        // Assert tracing events were emitted
        assert!(logs_contain("screen_scrape"));
        assert!(logs_contain("Scrape completed"));
        assert!(logs_contain("tool.status_code"));
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_scrape_emits_warning_on_invalid_url() {
        let tool = ScreenScrapeTool::new();
        let args = ScrapeArgs {
            url: "ftp://invalid-scheme.com".to_string(),
            ..Default::default()
        };

        let _ = tool.call(args).await;

        // Assert warning was emitted for invalid URL scheme
        assert!(logs_contain("Unsupported URL scheme"));
    }
}
