Creating a tool for `rig-core` that uses Brave Search would involve several components:

## What's Involved

1. **Brave Search API Integration** - You'll need to use Brave's Search API (requires an API key from https://api.search.brave.com/app/keys)

2. **HTTP Client** - Making requests to Brave's API endpoints

3. **JSON Parsing** - Handling the API response structure

4. **Tool Trait Implementation** - Implementing whatever trait `rig-core` expects for tools

5. **Error Handling** - Managing API errors, rate limits, etc.

## Code Demo

Here's what a Brave Search tool for `rig-core` might look like:

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

// Import rig-core types
use rig::completion::ToolDefinition;
use rig::tool::Tool;

/// Configuration for Brave Search
#[derive(Debug, Clone)]
pub struct BraveSearchConfig {
    pub api_key: String,
    pub endpoint: String,
}

impl BraveSearchConfig {
    pub fn from_env() -> Self {
        Self {
            api_key: env::var("BRAVE_API_KEY")
                .expect("BRAVE_API_KEY environment variable must be set"),
            endpoint: "https://api.search.brave.com/res/v1/web/search".to_string(),
        }
    }
}

/// Input parameters for the search tool
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchArgs {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_lang: Option<String>,
}

/// Brave Search API response structures
#[derive(Debug, Deserialize)]
struct BraveSearchResponse {
    web: Option<WebResults>,
    query: QueryInfo,
}

#[derive(Debug, Deserialize)]
struct WebResults {
    results: Vec<SearchResult>,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    title: String,
    url: String,
    description: String,
    #[serde(default)]
    published_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QueryInfo {
    original: String,
}

/// Formatted search result for output
#[derive(Debug, Serialize)]
pub struct SearchResultOutput {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Custom error type for Brave Search
#[derive(Debug, thiserror::Error)]
pub enum BraveSearchError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("API error (status {status}): {message}")]
    ApiError { status: u16, message: String },
    
    #[error("Failed to parse response: {0}")]
    ParseError(String),
    
    #[error("No results found")]
    NoResults,
}

/// The Brave Search tool implementation
pub struct BraveSearchTool {
    config: BraveSearchConfig,
    client: Client,
}

impl BraveSearchTool {
    pub fn new(config: BraveSearchConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub fn from_env() -> Self {
        Self::new(BraveSearchConfig::from_env())
    }

    async fn perform_search(&self, args: &SearchArgs) -> Result<Vec<SearchResultOutput>, BraveSearchError> {
        let count = args.count.unwrap_or(10).min(20); // Max 20 results
        let offset = args.offset.unwrap_or(0);

        // Build query parameters
        let mut params = vec![
            ("q", args.query.as_str()),
            ("count", &count.to_string()),
            ("offset", &offset.to_string()),
        ];

        if let Some(ref country) = args.country {
            params.push(("country", country.as_str()));
        }
        if let Some(ref lang) = args.search_lang {
            params.push(("search_lang", lang.as_str()));
        }

        let response = self
            .client
            .get(&self.config.endpoint)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip")
            .header("X-Subscription-Token", &self.config.api_key)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());
            return Err(BraveSearchError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let search_response: BraveSearchResponse = response
            .json()
            .await
            .map_err(|e| BraveSearchError::ParseError(e.to_string()))?;

        let results = search_response
            .web
            .map(|web| {
                web.results
                    .into_iter()
                    .map(|r| SearchResultOutput {
                        title: r.title,
                        url: r.url,
                        snippet: r.description,
                    })
                    .collect()
            })
            .unwrap_or_default();

        if results.is_empty() {
            return Err(BraveSearchError::NoResults);
        }

        Ok(results)
    }
}

// Implement the Tool trait for rig-core
impl Tool for BraveSearchTool {
    const NAME: &'static str = "brave_search";

    type Error = BraveSearchError;
    type Args = SearchArgs;
    type Output = Vec<SearchResultOutput>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "brave_search".to_string(),
            description: "Search the web using Brave Search API. Returns relevant web pages with titles, URLs, and descriptions. Use this tool when you need to find current information from the internet.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query string"
                    },
                    "count": {
                        "type": "integer",
                        "description": "Number of results to return (1-20, default: 10)",
                        "minimum": 1,
                        "maximum": 20
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Offset for pagination (default: 0)",
                        "minimum": 0
                    },
                    "country": {
                        "type": "string",
                        "description": "Country code for localized results (e.g., US, GB, CA)",
                        "minLength": 2,
                        "maxLength": 2
                    },
                    "search_lang": {
                        "type": "string",
                        "description": "Language code for results (e.g., en, es, fr)",
                        "minLength": 2,
                        "maxLength": 2
                    }
                },
                "required": ["query"]
            })
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.perform_search(&args).await
    }
}

/// Example usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the tool
    let search_tool = BraveSearchTool::from_env();

    // Get the tool definition (useful for LLM function calling)
    let tool_def = search_tool.definition("".to_string()).await;
    println!("Tool Definition:\n{}", serde_json::to_string_pretty(&tool_def)?);

    // Create search arguments
    let args = SearchArgs {
        query: "Rust programming language".to_string(),
        count: Some(5),
        offset: None,
        country: Some("US".to_string()),
        search_lang: Some("en".to_string()),
    };

    // Execute the search
    match search_tool.call(args).await {
        Ok(results) => {
            println!("\nFound {} results:", results.len());
            for (i, result) in results.iter().enumerate() {
                println!("\n{}. {}", i + 1, result.title);
                println!("   URL: {}", result.url);
                println!("   {}", result.snippet);
            }
        }
        Err(e) => {
            eprintln!("Search failed: {}", e);
        }
    }

    Ok(())
}

// Cargo.toml dependencies:
/*
[dependencies]
rig-core = "0.27"
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
thiserror = "1.0"
*/
```

## Key Points

1. **API Key**: You'll need to get a free API key from [Brave Search API](https://api.search.brave.com/app/keys)

2. **Rate Limits**: The free tier has limits (typically 2,000 requests/month)

3. **Error Handling**: The demo includes basic error handling, but you may want to add retries, exponential backoff, etc.

4. **Trait Adaptation**: You'll need to adjust the `Tool` trait implementation to match `rig-core`'s actual trait definition

5. **Features**: You could extend this with:
   - News search endpoint
   - Image search
   - Safe search options
   - Result filtering
   - Caching

