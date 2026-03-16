//! Type definitions for the markdown module.

use biscuit_file::YamlParseError;
use indexmap::IndexMap;
use thiserror::Error;

/// Type alias for frontmatter data.
///
/// Uses `IndexMap` to preserve insertion order so that frontmatter keys
/// are serialized in the same order they appeared in the source document.
pub type FrontmatterMap = IndexMap<String, serde_json::Value>;

/// Errors that can occur when working with Markdown documents.
#[derive(Error, Debug)]
pub enum MarkdownError {
    /// Failed to parse frontmatter YAML.
    #[error("Failed to parse frontmatter: {0}")]
    FrontmatterParse(#[from] YamlParseError),

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

    /// Transform pipeline error.
    #[error("Transform error: {0}")]
    Transform(String),

    /// Transclusion pipeline error.
    #[error("Transclusion error: {0}")]
    Transclusion(#[from] crate::markdown::transform::TransclusionError),

    /// TOC linking pipeline error.
    #[error("TOC linking error: {0}")]
    TocLinking(#[from] crate::markdown::transform::TocLinkingError),

    /// Shell expansion pipeline error.
    #[error("Shell expansion failed: {0}")]
    ShellExpansion(#[from] crate::markdown::transform::ShellExpansionError),

    /// Page block pipeline error.
    #[error("Page block error: {0}")]
    PageBlock(#[from] crate::markdown::transform::page_blocks::PageBlockError),
}

/// Result type for markdown operations.
pub type MarkdownResult<T> = Result<T, MarkdownError>;
