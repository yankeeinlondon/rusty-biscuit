//! Type definitions for the page blocks feature.

use std::ops::Range;

/// Errors from page block parsing or evaluation.
#[derive(Debug, thiserror::Error)]
pub enum PageBlockError {
    /// Failed to parse a page block directive.
    #[error("Failed to parse page block at line {line}: {message}")]
    ParseDirective { line: usize, message: String },

    /// Encountered `::end-block` without a matching `::block`.
    #[error("Unmatched ::end-block at line {line}")]
    UnmatchedEnd { line: usize },

    /// Reached end of file with an unclosed `::block`.
    #[error("Unterminated ::block starting at line {line}")]
    UnterminatedBlock { line: usize },

    /// Condition parsing or evaluation failed.
    #[error("{0}")]
    Condition(#[from] super::super::conditions::ConditionError),
}

/// Parsed options from a `::block` directive line.
#[derive(Debug, Clone)]
pub struct PageBlockOptions {
    /// The `when` condition expression, if present.
    pub when_expr: Option<String>,
    /// Any unrecognized option keys for warning reporting.
    pub unknown_options: Vec<String>,
}

/// A parsed page block region with exact byte spans.
#[derive(Debug, Clone)]
pub struct PageBlockRegion {
    /// Full span including `::block` and `::end-block` lines.
    pub span: Range<usize>,
    /// Body span excluding wrapper lines.
    pub body_span: Range<usize>,
    /// 1-based line number of the `::block` directive.
    pub start_line: usize,
    /// 1-based line number of the `::end-block` directive.
    pub end_line: usize,
    /// Parsed options from the start directive.
    pub options: PageBlockOptions,
    /// Nested child blocks.
    pub children: Vec<PageBlockRegion>,
}
