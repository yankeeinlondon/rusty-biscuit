# Agent Tool Integration

The `tools` module provides rig-core compatible agent tools with comprehensive tracing and error handling.

## Available Tools

### BraveSearchTool

Web search using the Brave Search API with rate limiting and plan-based configuration:

```rust
use shared::tools::{BraveSearchTool, BravePlan};

// From environment variables
let search = BraveSearchTool::from_env();

// With explicit configuration
let search = BraveSearchTool::new(
    api_key,
    BravePlan::Base, // 20 req/sec
);

// Use with rig-core agent
use rig::tool::ToolSet;
let toolset = ToolSet::builder()
    .static_tool(search)
    .build();
```

**Environment Variables**:
- `BRAVE_API_KEY` - API key (required)
- `BRAVE_PLAN` - Plan tier: `free`, `base`, `pro` (default: `free`)

**Rate Limits**:
- Free: 1 request/second
- Base: 20 requests/second
- Pro: 50 requests/second

### ScreenScrapeTool

Web page content extraction with multiple output formats:

```rust
use shared::tools::{ScreenScrapeTool, OutputFormat};

let scraper = ScreenScrapeTool::new();

// Configure output format
let args = ScrapeArgs {
    url: "https://example.com",
    output_format: Some(OutputFormat::Markdown),
    include_links: Some(true),
    ..Default::default()
};
```

**Output Formats**:
- `Markdown` - Clean markdown representation
- `HTML` - Raw HTML
- `PlainText` - Text only, no formatting
- `JSON` - Structured data
- `Links` - Extract all links

## Tool Implementation

### BraveSearchTool Usage

```rust
use shared::tools::{BraveSearchTool, SearchArgs};

// Create search arguments
let args = SearchArgs {
    query: "rust async programming",
    count: Some(10),
    offset: Some(0),
    safesearch: Some("moderate"),
    freshness: None, // "pd" (day), "pw" (week), "pm" (month)
};

// Execute search (within rig-core agent)
let results = search.call(&args).await?;
```

### ScreenScrapeTool Usage

```rust
use shared::tools::{ScreenScrapeTool, ScrapeArgs, OutputFormat};

let args = ScrapeArgs {
    url: "https://docs.rs",
    output_format: Some(OutputFormat::Markdown),
    include_links: Some(true),
    include_images: Some(false),
    max_length: Some(10000),
};

let content = scraper.call(&args).await?;
```

## Integration with rig-core

### Creating an Agent

```rust
use rig::providers::openai;
use rig::tool::ToolSet;
use shared::tools::{BraveSearchTool, ScreenScrapeTool};

// Create tools
let search = BraveSearchTool::from_env();
let scraper = ScreenScrapeTool::new();

// Build toolset
let tools = ToolSet::builder()
    .static_tool(search)
    .static_tool(scraper)
    .build();

// Create agent
let agent = openai::Client::from_env()
    .agent("gpt-4")
    .preamble("You are a helpful research assistant.")
    .tools(tools)
    .build();
```

### Using Tools in Agents

```rust
// The agent can now use tools via function calling
let response = agent
    .prompt("Search for information about Rust error handling")
    .await?;
```

## Tracing and Observability

All tools include comprehensive OpenTelemetry tracing:

```rust
// Automatic span creation
#[instrument(level = "info", skip(self))]
async fn call(&self, args: &SearchArgs) -> Result<String> {
    // Records tool.name, tool.query, tool.duration_ms
}
```

**Traced Fields**:
- `tool.name` - Tool being called
- `tool.query` - Search query or URL
- `tool.duration_ms` - Execution time
- `tool.results_count` - Number of results
- `http.status_code` - API response code
- `otel.kind` - Always "client"

### Enabling Tracing

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer())
    .with(tracing_subscriber::EnvFilter::from_default_env())
    .init();

// Set RUST_LOG=shared::tools=debug for detailed logs
```

## Error Handling

### BraveSearchError

```rust
use shared::tools::BraveSearchError;

match search.call(&args).await {
    Ok(results) => { /* ... */ }
    Err(BraveSearchError::MissingApiKey) => {
        eprintln!("Set BRAVE_API_KEY environment variable");
    }
    Err(BraveSearchError::RateLimitExceeded) => {
        eprintln!("Too many requests");
    }
    Err(BraveSearchError::ApiError(e)) => {
        eprintln!("API error: {}", e);
    }
    // ... other variants
}
```

### ScrapeError

```rust
use shared::tools::ScrapeError;

match scraper.call(&args).await {
    Ok(content) => { /* ... */ }
    Err(ScrapeError::InvalidUrl(url)) => {
        eprintln!("Invalid URL: {}", url);
    }
    Err(ScrapeError::NetworkError(e)) => {
        eprintln!("Network error: {}", e);
    }
    Err(ScrapeError::ParseError(msg)) => {
        eprintln!("Failed to parse: {}", msg);
    }
}
```

## Advanced Features

### Custom Headers

```rust
let scraper = ScreenScrapeTool::new();

// The tool automatically sets appropriate headers:
// - User-Agent for web compatibility
// - Accept-Language
// - Accept-Encoding for compression
```

### Content Processing

```rust
// Markdown output includes:
// - Heading hierarchy preservation
// - Link extraction with base URL resolution
// - Image alt text
// - Code block detection

// JSON output provides:
// - Title extraction
// - Meta tags
// - Structured content
// - Link analysis
```

## Testing

Both tools include comprehensive test suites:

```rust
#[cfg(test)]
mod tests {
    use wiremock::{MockServer, Mock};

    #[tokio::test]
    async fn test_search_with_mock() {
        let mock_server = MockServer::start().await;
        // Set up mock responses...
    }
}
```

## Best Practices

1. **Rate Limiting**: Respect API limits, use appropriate plan
2. **Error Handling**: Always handle rate limits and network errors
3. **Tracing**: Enable tracing in production for debugging
4. **Caching**: Consider caching results for repeated queries
5. **Timeouts**: Both tools have built-in reasonable timeouts

## Example: Research Agent

```rust
// Complete example of a research agent
use rig::agent::Agent;
use shared::tools::{BraveSearchTool, ScreenScrapeTool};

async fn research_topic(topic: &str) -> Result<String> {
    // Create tools
    let search = BraveSearchTool::from_env();
    let scraper = ScreenScrapeTool::new();

    // Build agent with tools
    let agent = /* ... build agent ... */;

    // Research workflow
    let response = agent
        .prompt(format!(
            "Research '{}'. First search for information, \
             then scrape the most relevant result.",
            topic
        ))
        .await?;

    Ok(response)
}
```