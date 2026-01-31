//! Brave Search Tool for rig-core agents
//!
//! Provides web search capabilities using the [Brave Search API](https://api.search.brave.com/).
//!
//! ## Requirements
//!
//! - A Brave Search API key (get one at <https://api.search.brave.com/app/keys>)
//! - Set the `BRAVE_API_KEY` environment variable
//!
//! ## Rate Limiting
//!
//! The tool automatically rate limits requests based on the `BRAVE_PLAN` environment variable:
//! - `free` (default): 1 request per second
//! - `base`: 20 requests per second
//! - `pro`: 50 requests per second
//!
//! ## Example
//!
//! ```rust,ignore
//! use unchained_ai::rigging::tools::{BraveSearchTool, SearchArgs};
//! use rig::tool::Tool;
//!
//! let tool = BraveSearchTool::from_env();
//! let args = SearchArgs {
//!     query: "Rust programming".to_string(),
//!     count: Some(5),
//!     ..Default::default()
//! };
//!
//! let results = tool.call(args).await?;
//! for result in results {
//!     println!("{}: {}", result.title, result.url);
//! }
//! ```

use reqwest::Client;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{Span, debug, info, instrument, warn};

/// Brave Search API plan tier, determines rate limiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BravePlan {
    /// Free tier: 1 request per second
    #[default]
    Free,
    /// Base tier: 20 requests per second
    Base,
    /// Pro tier: 50 requests per second
    Pro,
}

impl BravePlan {
    /// Parse plan from string (case-insensitive).
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "base" => Self::Base,
            "pro" => Self::Pro,
            _ => Self::Free,
        }
    }

    /// Get the minimum interval between requests for this plan.
    pub fn min_interval(&self) -> Duration {
        match self {
            Self::Free => Duration::from_millis(1000), // 1 req/sec
            Self::Base => Duration::from_millis(50),   // 20 req/sec
            Self::Pro => Duration::from_millis(20),    // 50 req/sec
        }
    }

    /// Get the requests per second for this plan.
    pub fn requests_per_second(&self) -> u32 {
        match self {
            Self::Free => 1,
            Self::Base => 20,
            Self::Pro => 50,
        }
    }
}

/// Shared rate limiter for Brave Search requests.
#[derive(Clone)]
struct RateLimiter {
    last_request: Arc<Mutex<Option<Instant>>>,
    min_interval: Duration,
}

impl RateLimiter {
    fn new(plan: BravePlan) -> Self {
        Self {
            last_request: Arc::new(Mutex::new(None)),
            min_interval: plan.min_interval(),
        }
    }

    /// Wait if necessary to respect the rate limit, then mark this request.
    async fn acquire(&self) {
        let mut last = self.last_request.lock().await;
        if let Some(last_time) = *last {
            let elapsed = last_time.elapsed();
            if elapsed < self.min_interval {
                let wait_time = self.min_interval - elapsed;
                debug!(
                    wait_ms = wait_time.as_millis() as u64,
                    "Rate limiting: waiting before next request"
                );
                tokio::time::sleep(wait_time).await;
            }
        }
        *last = Some(Instant::now());
    }
}

/// Configuration for the Brave Search API client.
#[derive(Debug, Clone)]
pub struct BraveSearchConfig {
    /// API key for authentication
    pub api_key: String,
    /// API endpoint URL
    pub endpoint: String,
    /// API plan tier for rate limiting
    pub plan: BravePlan,
}

impl BraveSearchConfig {
    /// Create configuration from environment variables.
    ///
    /// Reads:
    /// - `BRAVE_API_KEY` (required): API key for authentication
    /// - `BRAVE_PLAN` (optional): Plan tier for rate limiting ("free", "base", "pro")
    ///
    /// ## Panics
    ///
    /// Panics if `BRAVE_API_KEY` is not set.
    pub fn from_env() -> Self {
        let plan = env::var("BRAVE_PLAN")
            .map(|s| BravePlan::from_string(&s))
            .unwrap_or_default();

        info!(
            plan = ?plan,
            rate_limit = plan.requests_per_second(),
            "Brave Search configured"
        );

        Self {
            api_key: env::var("BRAVE_API_KEY")
                .expect("BRAVE_API_KEY environment variable must be set"),
            endpoint: "https://api.search.brave.com/res/v1/web/search".to_string(),
            plan,
        }
    }

    /// Create configuration with explicit values.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            endpoint: "https://api.search.brave.com/res/v1/web/search".to_string(),
            plan: BravePlan::default(),
        }
    }

    /// Set a custom endpoint (useful for testing).
    #[must_use]
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Set the API plan tier for rate limiting.
    #[must_use]
    pub fn with_plan(mut self, plan: BravePlan) -> Self {
        self.plan = plan;
        self
    }
}

/// Input parameters for the Brave Search tool.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchArgs {
    /// The search query string
    pub query: String,

    /// Number of results to return (1-20, default: 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,

    /// Offset for pagination (default: 0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,

    /// Country code for localized results (e.g., "US", "GB")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Language code for results (e.g., "en", "es")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_lang: Option<String>,

    /// Safe search mode ("off", "moderate", "strict")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safesearch: Option<String>,

    /// Freshness filter ("pd" = past day, "pw" = past week, "pm" = past month)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness: Option<String>,
}

/// Brave Search API response structure.
#[derive(Debug, Deserialize)]
struct BraveSearchResponse {
    web: Option<WebResults>,
    #[allow(dead_code)]
    query: Option<QueryInfo>,
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
    #[allow(dead_code)]
    published_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QueryInfo {
    #[allow(dead_code)]
    original: String,
}

/// A single search result returned by the tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResultOutput {
    /// Title of the web page
    pub title: String,
    /// URL of the web page
    pub url: String,
    /// Snippet/description of the page content
    pub snippet: String,
}

/// Errors that can occur during Brave Search operations.
#[derive(Debug, Error)]
pub enum BraveSearchError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// API returned an error response
    #[error("API error (status {status}): {message}")]
    ApiError {
        /// HTTP status code
        status: u16,
        /// Error message from the API
        message: String,
    },

    /// Failed to parse the API response
    #[error("Failed to parse response: {0}")]
    ParseError(String),

    /// No results found for the query
    #[error("No results found for query")]
    NoResults,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

/// Brave Search tool for rig-core agents.
///
/// This tool enables AI agents to search the web using the Brave Search API.
/// It returns structured search results with titles, URLs, and snippets.
///
/// The tool implements rate limiting based on the configured plan tier.
/// When cloned, all instances share the same rate limiter to ensure
/// the rate limit is respected across concurrent usage.
#[derive(Clone)]
pub struct BraveSearchTool {
    config: BraveSearchConfig,
    client: Client,
    rate_limiter: RateLimiter,
}

impl BraveSearchTool {
    /// Create a new Brave Search tool with the given configuration.
    pub fn new(config: BraveSearchConfig) -> Self {
        let rate_limiter = RateLimiter::new(config.plan);
        Self {
            config,
            client: Client::new(),
            rate_limiter,
        }
    }

    /// Create a new Brave Search tool from environment variables.
    ///
    /// ## Panics
    ///
    /// Panics if `BRAVE_API_KEY` is not set.
    pub fn from_env() -> Self {
        Self::new(BraveSearchConfig::from_env())
    }

    /// Create a tool with a custom HTTP client (useful for testing).
    #[cfg(test)]
    pub fn with_client(config: BraveSearchConfig, client: Client) -> Self {
        let rate_limiter = RateLimiter::new(config.plan);
        Self {
            config,
            client,
            rate_limiter,
        }
    }

    /// Perform a web search with the given arguments.
    #[instrument(
        name = "brave_search",
        skip(self, args),
        fields(
            tool.name = "brave_search",
            tool.query = %args.query,
            tool.count = args.count.unwrap_or(10),
            otel.kind = "client"
        )
    )]
    async fn perform_search(
        &self,
        args: &SearchArgs,
    ) -> Result<Vec<SearchResultOutput>, BraveSearchError> {
        let start = std::time::Instant::now();

        if args.query.trim().is_empty() {
            warn!("Search query is empty");
            return Err(BraveSearchError::ConfigError(
                "Query cannot be empty".to_string(),
            ));
        }

        // Acquire rate limit before making request
        self.rate_limiter.acquire().await;

        let count = args.count.unwrap_or(10).clamp(1, 20);
        let offset = args.offset.unwrap_or(0);

        debug!(
            country = ?args.country,
            freshness = ?args.freshness,
            safesearch = ?args.safesearch,
            "Executing search"
        );

        let mut request = self
            .client
            .get(&self.config.endpoint)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip")
            .header("X-Subscription-Token", &self.config.api_key)
            .query(&[
                ("q", args.query.as_str()),
                ("count", &count.to_string()),
                ("offset", &offset.to_string()),
            ]);

        // Add optional parameters
        if let Some(ref country) = args.country {
            request = request.query(&[("country", country.as_str())]);
        }
        if let Some(ref lang) = args.search_lang {
            request = request.query(&[("search_lang", lang.as_str())]);
        }
        if let Some(ref safesearch) = args.safesearch {
            request = request.query(&[("safesearch", safesearch.as_str())]);
        }
        if let Some(ref freshness) = args.freshness {
            request = request.query(&[("freshness", freshness.as_str())]);
        }

        let response = request.send().await;

        match &response {
            Ok(resp) => {
                let status = resp.status().as_u16();
                Span::current().record("http.status_code", status);
                debug!(http.status_code = status, "Received API response");
            }
            Err(e) => {
                warn!(error = %e, "Search request failed");
            }
        }

        let response = response?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());
            warn!(status, %message, "API returned error");
            return Err(BraveSearchError::ApiError { status, message });
        }

        let search_response: BraveSearchResponse = response
            .json()
            .await
            .map_err(|e| BraveSearchError::ParseError(e.to_string()))?;

        let results: Vec<SearchResultOutput> = search_response
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

        let elapsed = start.elapsed();

        if results.is_empty() {
            debug!(
                query = %args.query,
                tool.duration_ms = elapsed.as_millis() as u64,
                "Search returned no results (this is not an error)"
            );
        } else {
            info!(
                tool.results_count = results.len(),
                tool.duration_ms = elapsed.as_millis() as u64,
                "Search completed"
            );
        }

        Ok(results)
    }
}

impl std::fmt::Debug for BraveSearchTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BraveSearchTool")
            .field("endpoint", &self.config.endpoint)
            .finish_non_exhaustive()
    }
}

impl Tool for BraveSearchTool {
    const NAME: &'static str = "brave_search";

    type Error = BraveSearchError;
    type Args = SearchArgs;
    type Output = Vec<SearchResultOutput>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "brave_search".to_string(),
            description: "Search the web using Brave Search API. Returns relevant web pages \
                with titles, URLs, and descriptions. Use this tool when you need to find \
                current information from the internet, research topics, or verify facts."
                .to_string(),
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
                    },
                    "safesearch": {
                        "type": "string",
                        "description": "Safe search mode",
                        "enum": ["off", "moderate", "strict"]
                    },
                    "freshness": {
                        "type": "string",
                        "description": "Freshness filter: pd=past day, pw=past week, pm=past month, py=past year",
                        "enum": ["pd", "pw", "pm", "py"]
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.perform_search(&args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================
    // Tests for BraveSearchConfig
    // ===========================================

    #[test]
    fn test_config_new() {
        let config = BraveSearchConfig::new("test-api-key");
        assert_eq!(config.api_key, "test-api-key");
        assert!(config.endpoint.contains("api.search.brave.com"));
        assert_eq!(config.plan, BravePlan::Free);
    }

    #[test]
    fn test_config_with_endpoint() {
        let config = BraveSearchConfig::new("test-key").with_endpoint("http://localhost:8080");
        assert_eq!(config.endpoint, "http://localhost:8080");
    }

    #[test]
    fn test_config_with_plan() {
        let config = BraveSearchConfig::new("test-key").with_plan(BravePlan::Pro);
        assert_eq!(config.plan, BravePlan::Pro);
    }

    // ===========================================
    // Tests for BravePlan
    // ===========================================

    #[test]
    fn test_brave_plan_default() {
        assert_eq!(BravePlan::default(), BravePlan::Free);
    }

    #[test]
    fn test_brave_plan_from_str() {
        assert_eq!(BravePlan::from_string("free"), BravePlan::Free);
        assert_eq!(BravePlan::from_string("FREE"), BravePlan::Free);
        assert_eq!(BravePlan::from_string("base"), BravePlan::Base);
        assert_eq!(BravePlan::from_string("BASE"), BravePlan::Base);
        assert_eq!(BravePlan::from_string("pro"), BravePlan::Pro);
        assert_eq!(BravePlan::from_string("PRO"), BravePlan::Pro);
        // Unknown values default to Free
        assert_eq!(BravePlan::from_string("unknown"), BravePlan::Free);
        assert_eq!(BravePlan::from_string(""), BravePlan::Free);
    }

    #[test]
    fn test_brave_plan_min_interval() {
        assert_eq!(BravePlan::Free.min_interval(), Duration::from_millis(1000));
        assert_eq!(BravePlan::Base.min_interval(), Duration::from_millis(50));
        assert_eq!(BravePlan::Pro.min_interval(), Duration::from_millis(20));
    }

    #[test]
    fn test_brave_plan_requests_per_second() {
        assert_eq!(BravePlan::Free.requests_per_second(), 1);
        assert_eq!(BravePlan::Base.requests_per_second(), 20);
        assert_eq!(BravePlan::Pro.requests_per_second(), 50);
    }

    // ===========================================
    // Tests for SearchArgs
    // ===========================================

    #[test]
    fn test_search_args_default() {
        let args = SearchArgs::default();
        assert!(args.query.is_empty());
        assert!(args.count.is_none());
        assert!(args.offset.is_none());
        assert!(args.country.is_none());
        assert!(args.search_lang.is_none());
        assert!(args.safesearch.is_none());
        assert!(args.freshness.is_none());
    }

    #[test]
    fn test_search_args_serialization() {
        let args = SearchArgs {
            query: "test query".to_string(),
            count: Some(5),
            country: Some("US".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&args).unwrap();
        assert!(json.contains("test query"));
        assert!(json.contains("5"));
        assert!(json.contains("US"));
        // Optional None fields should be skipped
        assert!(!json.contains("offset"));
    }

    #[test]
    fn test_search_args_deserialization() {
        let json = r#"{"query": "rust programming", "count": 10}"#;
        let args: SearchArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.query, "rust programming");
        assert_eq!(args.count, Some(10));
        assert!(args.offset.is_none());
    }

    // ===========================================
    // Tests for SearchResultOutput
    // ===========================================

    #[test]
    fn test_search_result_output_equality() {
        let result1 = SearchResultOutput {
            title: "Test".to_string(),
            url: "https://example.com".to_string(),
            snippet: "A test result".to_string(),
        };
        let result2 = result1.clone();
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_search_result_output_serialization() {
        let result = SearchResultOutput {
            title: "Rust Programming".to_string(),
            url: "https://rust-lang.org".to_string(),
            snippet: "A systems programming language".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Rust Programming"));
        assert!(json.contains("rust-lang.org"));

        let deserialized: SearchResultOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }

    // ===========================================
    // Tests for BraveSearchError
    // ===========================================

    #[test]
    fn test_error_display_api_error() {
        let error = BraveSearchError::ApiError {
            status: 401,
            message: "Unauthorized".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("401"));
        assert!(display.contains("Unauthorized"));
    }

    #[test]
    fn test_error_display_config_error() {
        let error = BraveSearchError::ConfigError("Missing API key".to_string());
        let display = format!("{}", error);
        assert!(display.contains("configuration"));
        assert!(display.contains("Missing API key"));
    }

    // ===========================================
    // Tests for BraveSearchTool
    // ===========================================

    #[test]
    fn test_tool_debug() {
        let config = BraveSearchConfig::new("test-key");
        let tool = BraveSearchTool::new(config);
        let debug = format!("{:?}", tool);
        assert!(debug.contains("BraveSearchTool"));
        assert!(debug.contains("endpoint"));
    }

    #[test]
    fn test_tool_name_constant() {
        assert_eq!(BraveSearchTool::NAME, "brave_search");
    }

    #[tokio::test]
    async fn test_tool_definition() {
        let config = BraveSearchConfig::new("test-key");
        let tool = BraveSearchTool::new(config);
        let definition = tool.definition(String::new()).await;

        assert_eq!(definition.name, "brave_search");
        assert!(definition.description.contains("Search the web"));

        // Check parameters
        let params = definition.parameters;
        assert!(params["properties"]["query"].is_object());
        assert!(params["properties"]["count"].is_object());
        assert_eq!(params["required"], serde_json::json!(["query"]));
    }

    #[tokio::test]
    async fn test_empty_query_error() {
        let config = BraveSearchConfig::new("test-key").with_endpoint("http://localhost:1234");
        let tool = BraveSearchTool::new(config);

        let args = SearchArgs {
            query: "   ".to_string(),
            ..Default::default()
        };

        let result = tool.call(args).await;
        assert!(matches!(result, Err(BraveSearchError::ConfigError(_))));
    }

    // ===========================================
    // Tests for count clamping
    // ===========================================

    #[test]
    fn test_count_clamping_logic() {
        // This tests the clamping logic: count.min(20).max(1)
        assert_eq!(Some(25u32).unwrap_or(10).min(20).max(1), 20);
        assert_eq!(Some(0u32).unwrap_or(10).min(20).max(1), 1);
        assert_eq!(Some(10u32).unwrap_or(10).min(20).max(1), 10);
        assert_eq!(None::<u32>.unwrap_or(10).min(20).max(1), 10);
    }

    // ===========================================
    // Integration test with mock server
    // ===========================================

    #[tokio::test]
    async fn test_successful_search_with_mock() {
        use wiremock::matchers::{header, method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "web": {
                "results": [
                    {
                        "title": "Rust Programming Language",
                        "url": "https://www.rust-lang.org/",
                        "description": "A language empowering everyone to build reliable software."
                    },
                    {
                        "title": "Rust by Example",
                        "url": "https://doc.rust-lang.org/rust-by-example/",
                        "description": "Learn Rust with examples."
                    }
                ]
            },
            "query": {
                "original": "rust programming"
            }
        });

        Mock::given(method("GET"))
            .and(path("/search"))
            .and(query_param("q", "rust programming"))
            .and(header("X-Subscription-Token", "test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&mock_server)
            .await;

        let config = BraveSearchConfig::new("test-api-key")
            .with_endpoint(format!("{}/search", mock_server.uri()));
        let tool = BraveSearchTool::new(config);

        let args = SearchArgs {
            query: "rust programming".to_string(),
            ..Default::default()
        };

        let results = tool.call(args).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Rust Programming Language");
        assert_eq!(results[0].url, "https://www.rust-lang.org/");
        assert!(results[0].snippet.contains("reliable software"));
    }

    #[tokio::test]
    async fn test_api_error_response() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Invalid API key"))
            .mount(&mock_server)
            .await;

        let config = BraveSearchConfig::new("invalid-key")
            .with_endpoint(format!("{}/search", mock_server.uri()));
        let tool = BraveSearchTool::new(config);

        let args = SearchArgs {
            query: "test".to_string(),
            ..Default::default()
        };

        let result = tool.call(args).await;

        match result {
            Err(BraveSearchError::ApiError { status, message }) => {
                assert_eq!(status, 401);
                assert!(message.contains("Invalid API key"));
            }
            _ => panic!("Expected ApiError"),
        }
    }

    #[tokio::test]
    async fn test_no_results_response() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "web": {
                "results": []
            }
        });

        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&mock_server)
            .await;

        let config = BraveSearchConfig::new("test-key")
            .with_endpoint(format!("{}/search", mock_server.uri()));
        let tool = BraveSearchTool::new(config);

        let args = SearchArgs {
            query: "xyznonexistentquery123".to_string(),
            ..Default::default()
        };

        let result = tool.call(args).await;
        // Empty results should be Ok with an empty vector, not an error
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_optional_parameters_included() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "web": { "results": [{"title": "Test", "url": "http://test.com", "description": "Test"}] }
        });

        Mock::given(method("GET"))
            .and(path("/search"))
            .and(query_param("country", "US"))
            .and(query_param("search_lang", "en"))
            .and(query_param("safesearch", "strict"))
            .and(query_param("freshness", "pw"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&mock_server)
            .await;

        let config = BraveSearchConfig::new("test-key")
            .with_endpoint(format!("{}/search", mock_server.uri()));
        let tool = BraveSearchTool::new(config);

        let args = SearchArgs {
            query: "test".to_string(),
            country: Some("US".to_string()),
            search_lang: Some("en".to_string()),
            safesearch: Some("strict".to_string()),
            freshness: Some("pw".to_string()),
            ..Default::default()
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());
    }

    // ===========================================
    // Tracing tests
    // ===========================================

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_search_emits_tracing_events() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "web": {
                "results": [
                    {
                        "title": "Test Result",
                        "url": "https://example.com",
                        "description": "A test result"
                    }
                ]
            }
        });

        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&mock_server)
            .await;

        let config = BraveSearchConfig::new("test-key")
            .with_endpoint(format!("{}/search", mock_server.uri()));
        let tool = BraveSearchTool::new(config);

        let args = SearchArgs {
            query: "test query".to_string(),
            ..Default::default()
        };

        let _ = tool.call(args).await;

        // Assert tracing events were emitted
        assert!(logs_contain("brave_search"));
        assert!(logs_contain("Search completed"));
        assert!(logs_contain("tool.results_count"));
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_search_emits_warning_on_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Invalid API key"))
            .mount(&mock_server)
            .await;

        let config = BraveSearchConfig::new("test-key")
            .with_endpoint(format!("{}/search", mock_server.uri()));
        let tool = BraveSearchTool::new(config);

        let args = SearchArgs {
            query: "test".to_string(),
            ..Default::default()
        };

        let _ = tool.call(args).await;

        // Assert warning was emitted
        assert!(logs_contain("API returned error"));
    }
}
