//! Type definitions for the page blocks feature.

use std::ops::Range;

/// Errors from page block parsing or evaluation.
#[derive(Debug, thiserror::Error)]
pub enum PageBlockError {
    /// Failed to parse a page block directive.
    #[error("Failed to parse page block at line {line}: {message}")]
    ParseDirective {
        ctx: biscuit_terminal::errors::SourceContext,
        line: usize,
        message: String,
    },

    /// Encountered `::end-block` without a matching `::block`.
    #[error("Unmatched ::end-block at line {line}")]
    UnmatchedEnd {
        ctx: biscuit_terminal::errors::SourceContext,
        line: usize,
    },

    /// Reached end of file with an unclosed `::block`.
    #[error("Unterminated ::block starting at line {opening_line}")]
    UnterminatedBlock {
        ctx: biscuit_terminal::errors::SourceContext,
        opening_line: usize,
        opening_text: String,
    },

    /// Condition parsing or evaluation failed.
    #[error("{0}")]
    Condition(#[from] super::super::conditions::ConditionError),
}

impl biscuit_terminal::errors::BlockError for PageBlockError {
    fn status_block(
        &self,
        term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            PageBlockError::ParseDirective { ctx, line, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("PageBlockError", "directive parse failed"))
                .body(vec![
                    Prose::new(format!("<dim>Message:</dim> {message}")),
                    ctx.excerpt_prose(*line, 1, "md"),
                ])
                .hint("Opening syntax: <cyan>::block when=\"expr\"</cyan> — close with <cyan>::end-block</cyan>."),

            PageBlockError::UnmatchedEnd { ctx, line } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("PageBlockError", "unmatched ::end-block"))
                .body(vec![
                    Prose::new("This end-block has no matching start:"),
                    ctx.excerpt_prose(*line, 1, "md"),
                ])
                .hint("Add a matching <cyan>::block</cyan> directive above this closing line."),

            PageBlockError::UnterminatedBlock { ctx, opening_line, .. } => {
                let mut body = vec![
                    Prose::new("The opening page block was found here:"),
                    ctx.excerpt_prose(*opening_line, 2, "md"),
                ];
                if let Some(fm) = ctx.frontmatter_prose() {
                    body.insert(0, fm);
                    body.insert(0, Prose::new("The Frontmatter of this document was:"));
                }
                StatusBlock::new(StatusState::Error)
                    .error_header(
                        ErrorHeader::new(
                            "PageBlockError",
                            &format!(
                                "The file {} has an unterminated ::block / ::end-block block definition.",
                                ctx.linked_path_prose().content()
                            ),
                        )
                    )
                    .body(body)
                    .hint("Add a matching <inverse>::end-block</inverse> directive to close the region.")
            }

            PageBlockError::Condition(inner) => {
                biscuit_terminal::errors::BlockError::status_block(inner, term)
            }
        }
    }

    fn block_source(&self) -> Option<&(dyn biscuit_terminal::errors::BlockError + 'static)> {
        None
    }
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
