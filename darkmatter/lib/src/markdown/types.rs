//! Type definitions for the markdown module.

use std::collections::HashMap;
use thiserror::Error;

/// Type alias for frontmatter data.
pub type FrontmatterMap = HashMap<String, serde_json::Value>;

/// Errors that can occur when working with Markdown documents.
#[derive(Error, Debug)]
pub enum MarkdownError {
    /// Failed to parse frontmatter YAML.
    #[error("Failed to parse frontmatter: {0}")]
    FrontmatterParse(#[from] serde_yaml::Error),

    /// Failed to merge frontmatter.
    #[error("Failed to merge frontmatter: {0}")]
    FrontmatterMerge(String),

    /// Failed to load file.
    #[error("Failed to load file: {0}")]
    FileLoad(#[from] std::io::Error),

    /// Failed to fetch URL.
    #[error("Failed to fetch URL: {0}")]
    UrlFetch(#[from] reqwest::Error),

    /// Failed to load theme.
    #[error("Failed to load theme: {0}")]
    ThemeLoad(String),

    /// Failed to parse AST.
    #[error("Failed to parse AST: {0}")]
    AstParse(String),

    /// Invalid line range.
    #[error("Invalid line range: {0}")]
    InvalidLineRange(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type for markdown operations.
pub type MarkdownResult<T> = Result<T, MarkdownError>;
