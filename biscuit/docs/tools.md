# Agent Tools for rig-core

The shared library provides reusable tool implementations for AI agents built with the [rig-core](https://crates.io/crates/rig-core) framework. These tools implement the `rig::tool::Tool` trait and can be used with any rig-core agent.

## Available Tools

| Tool | Description | API Key Required |
|------|-------------|------------------|
| `BraveSearchTool` | Web search using Brave Search API | Yes (`BRAVE_API_KEY`) |
| `ScreenScrapeTool` | Web page content extraction | No |

## Quick Start

```rust
use shared::tools::{BraveSearchTool, ScreenScrapeTool};
use rig::tool::ToolSet;

// Create tools
let search = BraveSearchTool::from_env();
let scraper = ScreenScrapeTool::new();

// Add to a ToolSet for use with agents
let toolset = ToolSet::builder()
    .static_tool(search)
    .static_tool(scraper)
    .build();
```

---

## BraveSearchTool

Web search using the [Brave Search API](https://api.search.brave.com/).

### Requirements

1. Get an API key from [Brave Search API](https://api.search.brave.com/app/keys)
2. Set the `BRAVE_API_KEY` environment variable

### Configuration

```rust
use shared::tools::{BraveSearchTool, BraveSearchConfig};

// From environment variable
let tool = BraveSearchTool::from_env();

// With explicit configuration
let config = BraveSearchConfig::new("your-api-key");
let tool = BraveSearchTool::new(config);

// With custom endpoint (for testing)
let config = BraveSearchConfig::new("key")
    .with_endpoint("http://localhost:8080");
```

### Search Arguments

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | `String` | Yes | - | Search query string |
| `count` | `u32` | No | 10 | Number of results (1-20) |
| `offset` | `u32` | No | 0 | Pagination offset |
| `country` | `String` | No | - | Country code (e.g., "US", "GB") |
| `search_lang` | `String` | No | - | Language code (e.g., "en", "es") |
| `safesearch` | `String` | No | - | "off", "moderate", or "strict" |
| `freshness` | `String` | No | - | "pd" (day), "pw" (week), "pm" (month), "py" (year) |

### Usage Example

```rust
use shared::tools::{BraveSearchTool, SearchArgs};
use rig::tool::Tool;

let tool = BraveSearchTool::from_env();

let args = SearchArgs {
    query: "Rust async programming".to_string(),
    count: Some(5),
    country: Some("US".to_string()),
    freshness: Some("pm".to_string()), // Past month
    ..Default::default()
};

let results = tool.call(args).await?;

for result in results {
    println!("Title: {}", result.title);
    println!("URL: {}", result.url);
    println!("Snippet: {}", result.snippet);
    println!("---");
}
```

### Response Structure

```rust
pub struct SearchResultOutput {
    pub title: String,    // Page title
    pub url: String,      // Page URL
    pub snippet: String,  // Description/snippet
}
```

### Error Handling

```rust
use shared::tools::BraveSearchError;

match tool.call(args).await {
    Ok(results) => { /* process results */ }
    Err(BraveSearchError::NoResults) => {
        println!("No results found");
    }
    Err(BraveSearchError::ApiError { status, message }) => {
        println!("API error {}: {}", status, message);
    }
    Err(BraveSearchError::HttpError(e)) => {
        println!("Network error: {}", e);
    }
    Err(e) => {
        println!("Other error: {}", e);
    }
}
```

---

## ScreenScrapeTool

Web page content extraction with multiple output formats.

### Features

- **Multiple Output Formats**: Markdown, HTML, Plain Text, JSON, Links
- **Main Content Extraction**: Filter navigation, ads, footers
- **Tag Filtering**: Include/exclude specific HTML elements
- **User Agent Selection**: Desktop or mobile
- **Custom Headers**: Add authentication or other headers
- **Configurable Timeouts**: Control request duration

### Configuration

```rust
use shared::tools::ScreenScrapeTool;
use std::time::Duration;

// Default configuration
let tool = ScreenScrapeTool::new();

// With custom timeout
let tool = ScreenScrapeTool::new()
    .with_timeout(Duration::from_secs(60));
```

### Scrape Arguments

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `url` | `String` | Yes | - | URL to scrape |
| `formats` | `Vec<OutputFormat>` | No | `[Markdown]` | Output formats |
| `only_main_content` | `bool` | No | `false` | Extract main content only |
| `include_tags` | `Vec<String>` | No | `[]` | Tags to include |
| `exclude_tags` | `Vec<String>` | No | `[]` | Tags to exclude |
| `wait_for` | `u64` | No | - | Wait time in ms before scraping |
| `mobile` | `bool` | No | `false` | Use mobile user agent |
| `skip_tls_verification` | `bool` | No | `false` | Skip TLS verification |
| `remove_base64_images` | `bool` | No | `false` | Remove embedded images |
| `user_agent` | `String` | No | - | Custom user agent |
| `timeout` | `u64` | No | 30 | Request timeout in seconds |
| `follow_redirects` | `bool` | No | `true` | Follow HTTP redirects |
| `headers` | `HashMap<String, String>` | No | - | Custom HTTP headers |

### Output Formats

```rust
use shared::tools::OutputFormat;

// Available formats
let formats = vec![
    OutputFormat::Markdown,   // Converted to Markdown
    OutputFormat::Html,       // Raw HTML
    OutputFormat::PlainText,  // Text only
    OutputFormat::Json,       // Structured JSON
    OutputFormat::Links,      // Extract all links
];
```

### Usage Examples

#### Basic Scraping

```rust
use shared::tools::{ScreenScrapeTool, ScrapeArgs, OutputFormat};
use rig::tool::Tool;

let tool = ScreenScrapeTool::new();

let args = ScrapeArgs {
    url: "https://example.com".to_string(),
    formats: vec![OutputFormat::Markdown],
    only_main_content: true,
    ..Default::default()
};

let result = tool.call(args).await?;

// Access the markdown content
if let Some(markdown) = result.content.get("markdown") {
    println!("Content: {}", markdown);
}

// Check metadata
println!("Status: {}", result.status_code);
println!("Duration: {}ms", result.metadata.duration_ms);
```

#### Extracting Links

```rust
let args = ScrapeArgs {
    url: "https://example.com".to_string(),
    formats: vec![OutputFormat::Links],
    ..Default::default()
};

let result = tool.call(args).await?;

if let Some(links) = result.links {
    for link in links {
        println!("URL: {}", link.url);
        if let Some(text) = &link.text {
            println!("Text: {}", text);
        }
    }
}
```

#### Custom Headers (Authentication)

```rust
use std::collections::HashMap;

let mut headers = HashMap::new();
headers.insert("Authorization".to_string(), "Bearer token123".to_string());

let args = ScrapeArgs {
    url: "https://api.example.com/page".to_string(),
    headers: Some(headers),
    ..Default::default()
};
```

#### Mobile View

```rust
let args = ScrapeArgs {
    url: "https://example.com".to_string(),
    mobile: true,
    ..Default::default()
};
```

### Response Structure

```rust
pub struct ScrapeOutput {
    pub url: String,                           // Scraped URL
    pub status_code: u16,                      // HTTP status
    pub content: HashMap<String, Value>,       // Content by format
    pub metadata: ScrapeMetadata,              // Operation metadata
    pub links: Option<Vec<LinkInfo>>,          // Extracted links
}

pub struct ScrapeMetadata {
    pub content_type: Option<String>,          // Response content type
    pub content_length: Option<usize>,         // Content size in bytes
    pub duration_ms: u64,                      // Time taken
    pub actions_performed: usize,              // Actions executed
}

pub struct LinkInfo {
    pub url: String,                           // Link URL
    pub text: Option<String>,                  // Link text
    pub title: Option<String>,                 // Title attribute
    pub rel: Option<String>,                   // Rel attribute
}
```

### Error Handling

```rust
use shared::tools::ScrapeError;

match tool.call(args).await {
    Ok(result) => { /* process result */ }
    Err(ScrapeError::InvalidUrl(msg)) => {
        println!("Invalid URL: {}", msg);
    }
    Err(ScrapeError::RequestError(e)) => {
        println!("Request failed: {}", e);
    }
    Err(ScrapeError::Timeout) => {
        println!("Request timed out");
    }
    Err(e) => {
        println!("Other error: {}", e);
    }
}
```

---

## Using with rig-core Agents

### Adding Tools to an Agent

```rust
use rig::agent::AgentBuilder;
use rig::tool::ToolSet;
use shared::tools::{BraveSearchTool, ScreenScrapeTool};

// Create your tools
let search_tool = BraveSearchTool::from_env();
let scrape_tool = ScreenScrapeTool::new();

// Build a toolset
let toolset = ToolSet::builder()
    .static_tool(search_tool)
    .static_tool(scrape_tool)
    .build();

// Create an agent with the tools
let agent = AgentBuilder::new()
    .preamble("You are a helpful research assistant with web search and scraping capabilities.")
    .toolset(toolset)
    .build();
```

### Tool Definitions for LLMs

Both tools provide JSON Schema definitions via the `definition()` method:

```rust
use rig::tool::Tool;

let tool = BraveSearchTool::from_env();
let definition = tool.definition(String::new()).await;

println!("Name: {}", definition.name);
println!("Description: {}", definition.description);
println!("Parameters: {}", serde_json::to_string_pretty(&definition.parameters)?);
```

---

## Testing

Both tools include comprehensive unit tests. Run them with:

```bash
# From the shared directory
cargo test -p shared

# Run specific tool tests
cargo test -p shared brave_search
cargo test -p shared screen_scrape

# Run with output
cargo test -p shared -- --nocapture
```

### Mock Server Testing

The tools use [wiremock](https://crates.io/crates/wiremock) for integration testing:

```rust
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

let mock_server = MockServer::start().await;

Mock::given(method("GET"))
    .and(path("/search"))
    .respond_with(ResponseTemplate::new(200).set_body_json(&response))
    .mount(&mock_server)
    .await;

let config = BraveSearchConfig::new("test-key")
    .with_endpoint(format!("{}/search", mock_server.uri()));
let tool = BraveSearchTool::new(config);
```

---

## Rate Limits & Best Practices

### Brave Search API

- **Free tier**: ~2,000 requests/month
- **Paid tiers**: Higher limits available
- Use `freshness` parameter to get recent results
- Use `safesearch` for content filtering

### Web Scraping

- **Respect robots.txt**: Check site policies before scraping
- **Rate limiting**: Add delays between requests to the same domain
- **User agent**: Use appropriate user agent strings
- **Timeouts**: Set reasonable timeouts for slow sites
- **Error handling**: Handle network failures gracefully

### Performance Tips

1. Use `only_main_content: true` to reduce processing time
2. Request only needed `formats` to minimize computation
3. Set appropriate `timeout` values
4. Use `wait_for` only when necessary for dynamic content

---

## Exports

```rust
// Main module
use shared::tools;

// Individual types
use shared::tools::{
    // Brave Search
    BraveSearchTool,
    BraveSearchConfig,
    BraveSearchError,
    SearchArgs,
    SearchResultOutput,

    // Screen Scrape
    ScreenScrapeTool,
    ScrapeArgs,
    ScrapeOutput,
    ScrapeError,
    ScrapeMetadata,
    OutputFormat,
    ProxyMode,
    ScrapeAction,
    LinkInfo,
};
```
