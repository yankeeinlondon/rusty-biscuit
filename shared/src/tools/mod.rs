//! Agent Tools for rig-core
//!
//! This module provides reusable tool implementations for AI agents built with
//! the `rig-core` framework. These tools implement the [`rig::tool::Tool`] trait
//! and can be used with any rig-core agent.
//!
//! ## Available Tools
//!
//! - [`BraveSearchTool`] - Web search using the Brave Search API
//! - [`ScreenScrapeTool`] - Web page content extraction and scraping
//!
//! ## Usage
//!
//! ```rust,ignore
//! use shared::tools::{BraveSearchTool, ScreenScrapeTool};
//! use rig::tool::ToolSet;
//!
//! // Create tools
//! let search = BraveSearchTool::from_env();
//! let scraper = ScreenScrapeTool::new();
//!
//! // Add to a ToolSet for use with agents
//! let toolset = ToolSet::builder()
//!     .static_tool(search)
//!     .static_tool(scraper)
//!     .build();
//! ```

mod brave_search;
mod screen_scrape;

pub use brave_search::{
    BraveSearchConfig, BraveSearchError, BraveSearchTool, SearchArgs, SearchResultOutput,
};
pub use screen_scrape::{
    LinkInfo, OutputFormat, ProxyMode, ScrapeAction, ScrapeArgs, ScrapeError, ScrapeMetadata,
    ScrapeOutput, ScreenScrapeTool,
};
