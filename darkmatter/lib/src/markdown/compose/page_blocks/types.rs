//! Type definitions for the page blocks feature.

use std::ops::Range;

/// Errors from page block parsing or evaluation.
#[derive(Debug, thiserror::Error)]
pub enum PageBlockError {
    /// Failed to parse a page block directive.
    ///
    /// For this and the next two variants, the line is one-based in `ctx`:
    /// the loaded file's when `authored` is set, otherwise the body's (a
    /// document with no file); `0` means unknown (a file whose body no
    /// longer proves where the line was written), and the message then
    /// names no line.
    #[error("Failed to parse page block{}: {message}{}", at_line(" at line", *line), super::super::block_pairs::note_suffix(note))]
    ParseDirective {
        ctx: Box<biscuit_terminal::errors::SourceContext>,
        line: usize,
        /// The reason, without a line.
        message: String,
        /// Another line the reason refers to, counted like `line` (such as
        /// where a region a closer misses was opened).
        note: Option<String>,
        /// Where `line` was written in `ctx`, when proven: the position a
        /// diagnostic publishes.
        authored: Option<crate::markdown::types::AuthoredSpan>,
    },

    /// Encountered `::end-block` without a matching `::block`.
    #[error("Unmatched ::end-block{}", at_line(" at line", *line))]
    UnmatchedEnd {
        ctx: Box<biscuit_terminal::errors::SourceContext>,
        line: usize,
        authored: Option<crate::markdown::types::AuthoredSpan>,
    },

    /// Reached end of file with an unclosed `::block` (or `::shell-block`).
    #[error("Unterminated ::block{}", at_line(" starting at line", *opening_line))]
    UnterminatedBlock {
        ctx: Box<biscuit_terminal::errors::SourceContext>,
        opening_line: usize,
        opening_text: String,
        authored: Option<crate::markdown::types::AuthoredSpan>,
    },

    /// Condition parsing or evaluation failed.
    #[error("{0}")]
    Condition(#[from] Box<super::super::conditions::ConditionError>),
}

impl From<super::super::conditions::ConditionError> for PageBlockError {
    fn from(err: super::super::conditions::ConditionError) -> Self {
        PageBlockError::Condition(Box::new(err))
    }
}

/// `prefix` and `line`, or nothing for the unknown line `0`.
fn at_line(prefix: &str, line: usize) -> String {
    if line == 0 { String::new() } else { format!("{prefix} {line}") }
}

impl PageBlockError {
    /// A structural failure placed where its line was written in
    /// `markdown`'s loaded file: the parser counted `markdown`'s current body
    /// (over `data`, its inserted bytes), which the frontmatter split and
    /// earlier stages shift away from the file, so the line, its context,
    /// and a note's line move only where `origin` proves them (see
    /// [`DirectiveSite`](super::super::site_scope::DirectiveSite)). A
    /// condition failure is located by its own stage and is unchanged.
    #[must_use]
    pub(crate) fn at_authored_lines(
        mut self,
        markdown: &crate::markdown::Markdown,
        origin: Option<&super::super::body_origin::BodyOrigin>,
        data: Option<&super::super::body_origin::DataRanges>,
    ) -> Self {
        use super::super::site_scope::{scan_note, DirectiveSite};
        let site = |line: usize| DirectiveSite::of_line(markdown, origin, line);
        match &mut self {
            Self::ParseDirective { ctx, line, note, authored, .. } => {
                if note.is_some() && !matches!(site(*line), DirectiveSite::Body) {
                    *note = scan_note(markdown, data, *line, |at| DirectiveSite::file_line(markdown, origin, at));
                }
                site(*line).place_in(ctx, line, authored);
            }
            Self::UnmatchedEnd { ctx, line, authored } => site(*line).place_in(ctx, line, authored),
            Self::UnterminatedBlock { ctx, opening_line, authored, .. } => {
                site(*opening_line).place_in(ctx, opening_line, authored);
            }
            Self::Condition(_) => {}
        }
        self
    }
}

/// `ctx`'s excerpt around `line`, absent for the unknown line `0`.
fn excerpt(ctx: &biscuit_terminal::errors::SourceContext, line: usize, context: usize) -> Option<biscuit_terminal::components::prose::Prose> {
    (line != 0).then(|| ctx.excerpt_prose(line, context, "md"))
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
            PageBlockError::ParseDirective { ctx, line, message, note, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("PageBlockError", "directive parse failed"))
                .body(
                    [
                        Some(Prose::new(format!(
                            "<dim>Message:</dim> {message}{}",
                            super::super::block_pairs::note_suffix(note)
                        ))),
                        excerpt(ctx, *line, 1),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>(),
                )
                .hint("Opening syntax: <cyan>::block when=\"expr\"</cyan> — close with <cyan>::end-block</cyan>."),

            PageBlockError::UnmatchedEnd { ctx, line, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("PageBlockError", "unmatched ::end-block"))
                .body(
                    [Some(Prose::new("This end-block has no matching start:")), excerpt(ctx, *line, 1)]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>(),
                )
                .hint("Add a matching <cyan>::block</cyan> directive above this closing line."),

            PageBlockError::UnterminatedBlock { ctx, opening_line, .. } => {
                let mut body: Vec<_> = [Some(Prose::new("The opening page block was found here:")), excerpt(ctx, *opening_line, 2)]
                    .into_iter()
                    .flatten()
                    .collect();
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
                inner.status_block(term)
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
