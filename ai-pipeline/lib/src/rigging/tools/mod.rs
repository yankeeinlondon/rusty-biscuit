//! Rig-core agent tools for web search and content extraction.
//!
//! This module provides tools that integrate with rig-core's agent framework:
//!
//! - [`BraveSearchTool`] - Web search using the Brave Search API
//! - [`ScreenScrapeTool`] - Web page content extraction and scraping
//!
//! ## Usage with rig-core agents
//!
//! ```rust,ignore
//! use ai_pipeline::rigging::tools::{BraveSearchTool, ScreenScrapeTool};
//! use rig::tool::Tool;
//!
//! // Create tools
//! let search = BraveSearchTool::from_env();
//! let scraper = ScreenScrapeTool::new();
//!
//! // Use with an agent
//! let agent = client.agent("gpt-4o")
//!     .tool(search)
//!     .tool(scraper)
//!     .build();
//! ```

mod brave_search;
mod screen_scrape;

pub use brave_search::{
    BravePlan, BraveSearchConfig, BraveSearchError, BraveSearchTool, SearchArgs,
    SearchResultOutput,
};
pub use screen_scrape::{
    LinkInfo, OutputFormat, ProxyMode, ScrapeAction, ScrapeArgs, ScrapeError, ScrapeMetadata,
    ScrapeOutput, ScreenScrapeTool,
};
