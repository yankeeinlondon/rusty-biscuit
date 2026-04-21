//! Leaf-block helper constructors for [`MarkdownError`] variants.
//!
//! These helpers build fully configured [`StatusBlock`] values for the
//! `MarkdownError` variants whose inner error type does not (or cannot)
//! implement [`BlockError`] directly — typically foreign `std`/third-party
//! error types such as [`std::io::Error`], [`reqwest::Error`], and
//! [`serde_json::Error`].
//!
//! Helpers live in their own module so they can be unit-tested in isolation
//! and swapped as the error surface evolves.
//!
//! [`MarkdownError`]: crate::markdown::MarkdownError
//! [`BlockError`]: biscuit_terminal::errors::BlockError
//! [`StatusBlock`]: biscuit_terminal::components::status_block::StatusBlock

use biscuit_file::YamlParseError;
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

/// Build the [`StatusBlock`] for [`MarkdownError::FileLoad`].
pub(crate) fn file_load_block(source: &std::io::Error) -> StatusBlock {
    let kind = format!("{:?}", source.kind());
    let body = format!(
        "<dim>I/O:</dim> <b>{}</b>\n<dim>Kind:</dim> {}",
        source, kind
    );
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "file load failed"))
        .body(body)
        .hint("Confirm the path exists and your process has permission to read it.")
}

/// Build the [`StatusBlock`] for [`MarkdownError::UrlFetch`].
pub(crate) fn url_fetch_block(source: &reqwest::Error) -> StatusBlock {
    let url = source
        .url()
        .map(|u| u.to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    let status = source
        .status()
        .map(|s| format!("HTTP {s}"))
        .unwrap_or_else(|| "(no response)".to_string());

    let body = format!("<dim>URL:</dim> <cyan>{url}</cyan>\n<dim>Status:</dim> {status}\n{source}");

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "URL fetch failed"))
        .body(body)
        .hint("Verify the URL is reachable and that any required auth headers are set.")
}

/// Build the [`StatusBlock`] for [`MarkdownError::FrontmatterParse`].
pub(crate) fn frontmatter_parse_block(source: &YamlParseError) -> StatusBlock {
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "frontmatter parse failed"))
        .body(format!("{source}"))
        .hint("Check the YAML between the leading `---` markers for syntax errors.")
}

/// Build the [`StatusBlock`] for [`MarkdownError::FrontmatterMerge`].
pub(crate) fn frontmatter_merge_block(message: &str) -> StatusBlock {
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "frontmatter merge failed"))
        .body(message.to_string())
        .hint("Review the frontmatter merge strategy for conflicting keys.")
}

/// Build the [`StatusBlock`] for [`MarkdownError::ThemeLoad`].
pub(crate) fn theme_load_block(message: &str) -> StatusBlock {
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "theme load failed"))
        .body(message.to_string())
        .hint("Use `md --list-themes` to see available theme names.")
}

/// Build the [`StatusBlock`] for [`MarkdownError::AstParse`].
pub(crate) fn ast_parse_block(message: &str) -> StatusBlock {
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "AST parse failed"))
        .body(message.to_string())
        .hint("The document is not well-formed GFM markdown — check the reported location.")
}

/// Build the [`StatusBlock`] for [`MarkdownError::InvalidLineRange`].
pub(crate) fn invalid_line_range_block(message: &str) -> StatusBlock {
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "invalid line range"))
        .body(message.to_string())
        .hint("Line ranges are 1-based and must satisfy `start <= end` within the file.")
}

/// Build the [`StatusBlock`] for [`MarkdownError::Serialization`].
pub(crate) fn serialization_block(source: &serde_json::Error) -> StatusBlock {
    let body = format!(
        "{source}\n<dim>Position:</dim> line {}, column {}",
        source.line(),
        source.column()
    );
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "serialization failed"))
        .body(body)
        .hint("Check the input for invalid JSON tokens or unsupported types.")
}

/// Build the [`StatusBlock`] for [`MarkdownError::Transform`].
pub(crate) fn transform_block(message: &str) -> StatusBlock {
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "transform failed"))
        .body(message.to_string())
        .hint("Review the transform pipeline inputs and any configured rules.")
}
