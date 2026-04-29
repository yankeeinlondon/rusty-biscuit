//! Type definitions for the markdown module.

use biscuit_file::YamlParseError;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::terminal::Terminal;
use indexmap::IndexMap;
use thiserror::Error;

use crate::markdown::errors::blocks;

/// Type alias for frontmatter data.
///
/// Uses `IndexMap` to preserve insertion order so that frontmatter keys
/// are serialized in the same order they appeared in the source document.
pub type FrontmatterMap = IndexMap<String, serde_json::Value>;

/// Errors that can occur when working with Markdown documents.
#[derive(Error, Debug)]
pub enum MarkdownError {
    /// Failed to parse frontmatter YAML.
    ///
    /// Carries the original YAML body (between the leading `---` markers) so
    /// renderers can surface the offending line in error reports.
    #[error("Failed to parse frontmatter: {source}")]
    FrontmatterParse {
        #[source]
        source: YamlParseError,
        yaml: String,
    },

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
    Transclusion(#[from] crate::markdown::compose::TransclusionError),

    /// TOC linking pipeline error.
    #[error("TOC linking error: {0}")]
    TocLinking(#[from] crate::markdown::compose::TocLinkingError),

    /// Shell expansion pipeline error.
    #[error("Shell expansion failed: {0}")]
    ShellExpansion(#[from] crate::markdown::compose::ShellExpansionError),

    /// Page block pipeline error.
    #[error("Page block error: {0}")]
    PageBlock(#[from] crate::markdown::compose::page_blocks::PageBlockError),

    /// Shell block pipeline error.
    #[error("Shell block error: {0}")]
    ShellBlock(#[from] Box<crate::markdown::compose::ShellBlockError>),

    /// Reference analysis error.
    #[error("Reference error: {0}")]
    Reference(#[from] crate::markdown::reference::ReferenceError),

    /// Context merge error (invalid user ctx).
    #[error("Context error: {0}")]
    CtxMerge(#[from] crate::markdown::compose::context::merge::CtxMergeError),
}

impl From<crate::markdown::compose::ShellBlockError> for MarkdownError {
    fn from(err: crate::markdown::compose::ShellBlockError) -> Self {
        MarkdownError::ShellBlock(Box::new(err))
    }
}

/// Result type for markdown operations.
pub type MarkdownResult<T> = Result<T, MarkdownError>;

impl BlockError for MarkdownError {
    fn status_block(&self, term: &Terminal) -> StatusBlock {
        match self {
            // Delegating variants: surface the sub-error's block directly.
            MarkdownError::Transclusion(inner) => inner.status_block(term),
            MarkdownError::ShellExpansion(inner) => inner.status_block(term),
            MarkdownError::PageBlock(inner) => inner.status_block(term),
            MarkdownError::ShellBlock(inner) => inner.status_block(term),
            MarkdownError::TocLinking(inner) => inner.status_block(term),
            MarkdownError::Reference(inner) => inner.status_block(term),
            MarkdownError::CtxMerge(inner) => inner.status_block(term),

            // Leaf variants own their block shape.
            MarkdownError::FrontmatterParse { source, yaml } => {
                blocks::frontmatter_parse_block(source, yaml)
            }
            MarkdownError::FrontmatterMerge(message) => blocks::frontmatter_merge_block(message),
            MarkdownError::FileLoad(source) => blocks::file_load_block(source),
            MarkdownError::UrlFetch(source) => blocks::url_fetch_block(source),
            MarkdownError::ThemeLoad(message) => blocks::theme_load_block(message),
            MarkdownError::AstParse(message) => blocks::ast_parse_block(message),
            MarkdownError::InvalidLineRange(message) => blocks::invalid_line_range_block(message),
            MarkdownError::Serialization(source) => blocks::serialization_block(source),
            MarkdownError::Transform(message) => blocks::transform_block(message),
        }
    }

    fn block_source(&self) -> Option<&(dyn BlockError + 'static)> {
        // Delegating variants already return the inner block from
        // `status_block`, so returning `Some(inner)` here would double-render
        // the same block under a "Caused by:" caption. Only leaf variants
        // that wrap a foreign error type and have no sub-block would surface
        // a cause — none of today's leaf variants do.
        None
    }
}
