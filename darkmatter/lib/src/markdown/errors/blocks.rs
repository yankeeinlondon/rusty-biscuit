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
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{ErrorHeader, SourceContext, StatusBlockExt};

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
///
/// When `serde_yaml_ng` reports a [`Location`](serde_yaml_ng::Location), the
/// body includes a one-line snippet of the offending YAML so users can see
/// exactly which line broke parsing without re-running the command.
pub(crate) fn frontmatter_parse_block(ctx: SourceContext, source: &YamlParseError) -> StatusBlock {
    let location = source.location();
    let location_line = location.map(|loc| loc.line());

    let mut body = vec![Prose::new(format!("<dim>YAML:</dim> {source}"))];

    if let Some(line) = location_line {
        body.push(Prose::new("Frontmatter parsing failed here:"));
        body.push(ctx.excerpt_prose(line, 1, "yaml"));
    }

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new(
            "MarkdownError",
            "frontmatter parse failed",
        ))
        .body(body)
        .hint("Check the YAML between the leading `---` markers for syntax errors.")
}

/// Build the [`StatusBlock`] for [`MarkdownError::FrontmatterMerge`].
pub(crate) fn frontmatter_merge_block(message: &str) -> StatusBlock {
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new(
            "MarkdownError",
            "frontmatter merge failed",
        ))
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    use super::*;

    fn render_block(block: &StatusBlock) -> String {
        strip_escape_codes(block.render_optimistic(Some(80)))
    }

    #[test]
    fn file_load_block_renders_io_kind_and_message() {
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let out = render_block(&file_load_block(&err));
        assert!(out.contains("MarkdownError"), "missing header type: {out}");
        assert!(out.contains("file load failed"), "missing summary: {out}");
        assert!(out.contains("NotFound"), "missing I/O kind: {out}");
        assert!(out.contains("no such file"), "missing error message: {out}");
    }

    #[test]
    fn frontmatter_parse_block_renders_yaml_error() {
        let yaml = ": [broken";
        let ctx = SourceContext::new(PathBuf::from("/test.md"), PathBuf::from("test.md"), yaml);
        let err = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml).unwrap_err();
        let out = render_block(&frontmatter_parse_block(ctx, &err));
        assert!(out.contains("MarkdownError"), "missing header type: {out}");
        assert!(
            out.contains("frontmatter parse failed"),
            "missing summary: {out}"
        );
        assert!(
            out.contains("error") || out.contains("Error"),
            "missing YAML error detail: {out}",
        );
    }

    /// Regression: a quoted scalar followed by trailing unquoted text is a
    /// real-world YAML error pattern (e.g. `- finding: '@' magic ...`). The
    /// rendered block should include the offending line so the user can see
    /// exactly what to fix.
    #[test]
    fn frontmatter_parse_block_includes_offending_line() {
        let yaml = "phases: 5\nfindings:\n  - id: '@' magic lookup emits results\n";
        let ctx = SourceContext::new(PathBuf::from("/test.md"), PathBuf::from("test.md"), yaml);
        let err = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml).unwrap_err();
        let out = render_block(&frontmatter_parse_block(ctx, &err));

        assert!(
            out.contains("'@' magic lookup emits results"),
            "missing offending line snippet: {out}",
        );
        assert!(out.contains(">"), "missing gutter marker: {out}");
    }

    #[test]
    fn frontmatter_merge_block_renders_message() {
        let out = render_block(&frontmatter_merge_block("conflict in 'title'"));
        assert!(out.contains("MarkdownError"), "missing header type: {out}");
        assert!(
            out.contains("frontmatter merge failed"),
            "missing summary: {out}"
        );
        assert!(
            out.contains("conflict in 'title'"),
            "missing message: {out}"
        );
    }

    #[test]
    fn theme_load_block_renders_message() {
        let out = render_block(&theme_load_block("unknown theme `neon`"));
        assert!(out.contains("MarkdownError"), "missing header type: {out}");
        assert!(out.contains("theme load failed"), "missing summary: {out}");
        assert!(
            out.contains("unknown theme `neon`"),
            "missing message: {out}"
        );
    }

    #[test]
    fn ast_parse_block_renders_message() {
        let out = render_block(&ast_parse_block("line 3: unexpected token"));
        assert!(out.contains("MarkdownError"), "missing header type: {out}");
        assert!(out.contains("AST parse failed"), "missing summary: {out}");
        assert!(
            out.contains("line 3: unexpected token"),
            "missing message: {out}"
        );
    }

    #[test]
    fn invalid_line_range_block_renders_message() {
        let out = render_block(&invalid_line_range_block("start > end"));
        assert!(out.contains("MarkdownError"), "missing header type: {out}");
        assert!(out.contains("invalid line range"), "missing summary: {out}");
        assert!(out.contains("start > end"), "missing message: {out}");
    }

    #[test]
    fn serialization_block_renders_position() {
        let err = serde_json::from_str::<serde_json::Value>("{ bogus }").unwrap_err();
        let out = render_block(&serialization_block(&err));
        assert!(out.contains("MarkdownError"), "missing header type: {out}");
        assert!(
            out.contains("serialization failed"),
            "missing summary: {out}"
        );
        assert!(out.contains("line"), "missing line position: {out}");
        assert!(out.contains("column"), "missing column position: {out}");
    }

    #[test]
    fn transform_block_renders_message() {
        let out = render_block(&transform_block("pipeline stalled"));
        assert!(out.contains("MarkdownError"), "missing header type: {out}");
        assert!(out.contains("transform failed"), "missing summary: {out}");
        assert!(out.contains("pipeline stalled"), "missing message: {out}");
    }

    /// `reqwest::Error` cannot be constructed without firing a real HTTP
    /// request. This smoke test exercises the helper by sending a request to
    /// an address that is guaranteed to refuse connections, producing a real
    /// `reqwest::Error` cheaply.
    ///
    /// ## Notes
    ///
    /// The request targets `http://0.0.0.0:1` which has no listener and fails
    /// instantly. If this proves flaky in CI, the test can be replaced with a
    /// compile-time assertion that `url_fetch_block` accepts `&reqwest::Error`.
    #[tokio::test]
    async fn url_fetch_block_renders_with_reqwest_error() {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(100))
            .build()
            .expect("client builder should not fail");
        let result = client.get("http://0.0.0.0:1").send().await;
        let err = result.expect_err("request to 0.0.0.0:1 should fail");
        let out = render_block(&url_fetch_block(&err));
        assert!(out.contains("MarkdownError"), "missing header type: {out}");
        assert!(out.contains("URL fetch failed"), "missing summary: {out}");
        assert!(out.contains("http://0.0.0.0:1"), "missing URL: {out}");
    }
}
