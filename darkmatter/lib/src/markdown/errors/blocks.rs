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

use crate::markdown::highlighting::highlight_yaml_lines;
use crate::markdown::schemas::{ValidationProblem, ValidationProblemKind};

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

    let mut body = Vec::new();

    // Name the offending file when the context carries a real path (loads from
    // disk do; in-memory/stdin parses leave it as "unknown"). Directory hashing
    // hashes many files, so identifying which one failed is the actionable bit.
    if ctx.display != std::path::Path::new("unknown") {
        body.push(ctx.linked_path_prose());
    }

    body.push(Prose::new(format!("<dim>YAML:</dim> {source}")));

    if let Some(loc) = location {
        body.push(Prose::new("Frontmatter parsing failed here:"));
        body.push(frontmatter_excerpt_prose(
            &ctx,
            frontmatter_doc_line(&ctx, loc.line()),
            1,
        ));
    }

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new(
            "MarkdownError",
            "frontmatter parse failed",
        ))
        .body(body)
        .hint("Check the YAML between the leading `---` markers for syntax errors.")
}

/// Translate a `serde_yaml_ng` error line into a 1-based line in the full
/// source document.
///
/// `serde_yaml_ng` reports the line relative to the YAML it was handed — the
/// text *between* the `---` markers, with the opening delimiter already
/// stripped. When the context's content includes that delimiter (the on-disk
/// load path), the excerpt numbers lines document-absolutely, so the
/// YAML-relative line must be shifted past the opening `---`. When no
/// frontmatter delimiters are detected (e.g. a bare-YAML context), the reported
/// line already addresses the right content line and is returned unchanged.
fn frontmatter_doc_line(ctx: &SourceContext, yaml_line: usize) -> usize {
    match ctx.frontmatter.as_ref() {
        Some(range) => {
            let opening_delim_line = ctx.content[..range.start]
                .bytes()
                .filter(|&b| b == b'\n')
                .count()
                + 1;
            opening_delim_line + yaml_line
        }
        None => yaml_line,
    }
}

/// Render a syntax-highlighted, gutter-numbered excerpt centered on `line`
/// (1-based, document-absolute) with `context` lines above and below.
///
/// The offending line carries a leading `>` gutter marker. YAML content is
/// highlighted through [`highlight_yaml_lines`] — a lexical highlighter that
/// tolerates the malformed input that produced the error — then escaped so
/// literal `<`, `{`, etc. in the source render verbatim instead of being parsed
/// as Prose markup ([`Prose::escape_text`] passes the highlighter's ANSI
/// sequences through untouched). Falls back to unstyled text when the
/// highlighter does not return one line per input line.
fn frontmatter_excerpt_prose(ctx: &SourceContext, line: usize, context: usize) -> Prose {
    use std::fmt::Write as _;

    let lines: Vec<&str> = ctx.content.lines().collect();
    let total = lines.len();
    let start = line.saturating_sub(context + 1).min(total);
    let end = (line + context).min(total);
    let gutter_width = end.to_string().len();

    let window = &lines[start..end];
    let highlighted = highlight_yaml_lines(&window.join("\n"));
    let use_highlight = highlighted.len() == window.len();

    let mut buf = String::new();
    for (idx, raw) in window.iter().enumerate() {
        let n = start + idx + 1;
        let marker = if n == line { ">" } else { " " };
        let content = if use_highlight {
            Prose::escape_text(&highlighted[idx])
        } else {
            Prose::escape_text(raw)
        };
        let _ = writeln!(buf, "<dim>{marker} {n:>gutter_width$} │</dim> {content}");
    }

    Prose::new(buf.trim_end_matches('\n').to_string())
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

/// Build the [`StatusBlock`] for [`MarkdownError::RenderTree`].
pub(crate) fn render_tree_block(message: &str) -> StatusBlock {
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "render failed"))
        .body(message.to_string())
        .hint("The document produced a render tree the target renderer rejected.")
}

/// Build the [`StatusBlock`] for [`MarkdownError::MalformedStoredHash`].
pub(crate) fn malformed_stored_hash_block(property: &str, reason: &str) -> StatusBlock {
    let body = format!(
        "<dim>Property:</dim> <inverse>{}</inverse>\n{}",
        Prose::escape_text(property),
        Prose::escape_text(reason),
    );
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "malformed stored hash"))
        .body(body)
        .hint("Fix or remove the `hash` frontmatter property, or rerun `md hash --save` to rewrite it.")
}

/// Build the [`StatusBlock`] for [`MarkdownError::MalformedDisclosure`].
pub(crate) fn malformed_disclosure_block(reason: &str, range: &std::ops::Range<usize>) -> StatusBlock {
    let body = format!(
        "<dim>Reason:</dim> {}\n<dim>Range:</dim> {}..{}",
        Prose::escape_text(reason),
        range.start,
        range.end
    );
    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "malformed disclosure block"))
        .body(body)
        .hint("Disclosure blocks need `::disclosure`, `::details`, and `::end-disclosure`; the summary must contain only phrasing content.")
}

/// Build the [`StatusBlock`] for [`MarkdownError::SchemaValidationFailed`].
///
/// When `problems` is empty the failure represents a schema *preparation*
/// error (malformed `$schema`, unresolved reference, baseline build failure,
/// etc.) rather than an instance validation failure. In that case the
/// `summary` carries the underlying diagnostic and is rendered as a body
/// line so authors see the root cause instead of an empty problem list.
pub(crate) fn schema_validation_failed_block(
    path: &std::path::Path,
    problems: &[ValidationProblem],
    summary: &str,
    description: &Option<String>,
) -> StatusBlock {
    let path_attr = Prose::quoted_attr(&path.to_string_lossy());
    let path_escaped = Prose::escape_text(&path.to_string_lossy());

    let mut body_lines: Vec<String> = Vec::new();

    // OSC8 link to the source file
    body_lines.push(format!(
        "<blue><a href={}>{}</a></blue>",
        path_attr, path_escaped
    ));

    // Description line when present
    if let Some(desc) = description {
        body_lines.push(format!("<i><dim>{}</dim></i>", Prose::escape_text(desc)));
    }

    // Preparation failures arrive with an empty problem list; render the
    // summary so authors see the actual diagnostic (e.g. "schema could not
    // be prepared: ...") instead of just the path.
    if problems.is_empty() {
        let hint = if summary.is_empty() {
            "schema could not be prepared"
        } else {
            summary
        };
        let (label, detail) = match hint.split_once(':') {
            Some((head, tail)) => (head.trim(), tail.trim()),
            None => ("schema preparation failed", hint),
        };
        body_lines.push(format!(
            "<red>{}</red>: {}",
            Prose::escape_text(label),
            Prose::escape_text(detail),
        ));

        return StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new(
                "MarkdownError",
                "schema validation failed",
            ))
            .body(body_lines.join("\n"))
            .hint(
                "Check that the document's $schema (or the baseline schema) is well-formed and resolvable.",
            );
    }

    // One bullet per problem. The category label is chosen from
    // `ValidationProblem::kind` rather than inferred from `property.is_some()`
    // or substring-matched against `message`, so the renderer cannot drift
    // when the underlying validator surfaces new error shapes.
    for problem in problems {
        let loc = match (problem.line, problem.column) {
            (Some(l), Some(c)) => format!(" at {l}:{c}"),
            (Some(l), None) => format!(" at {l}:1"),
            _ => String::new(),
        };

        let arm = match problem.arm_index {
            Some(idx) => format!(" (schema arm {idx})"),
            None => String::new(),
        };

        let target = if let Some(ref prop) = problem.property {
            Prose::escape_text(prop)
        } else {
            let trimmed = problem.path.trim_start_matches('/');
            if trimmed.is_empty() {
                "<root>".to_string()
            } else {
                Prose::escape_text(trimmed.split('/').next().unwrap_or(trimmed))
            }
        };

        let bullet = match problem.kind {
            ValidationProblemKind::Missing => format!(
                "<red>missing</red> <inverse>{target}</inverse>: required but not provided{loc}{arm}"
            ),
            ValidationProblemKind::Type => format!(
                "<red>type</red> <inverse>{target}</inverse>: {}{loc}{arm}",
                Prose::escape_text(&problem.message)
            ),
            ValidationProblemKind::Invalid => format!(
                "<red>invalid</red> <inverse>{target}</inverse>: {}{loc}{arm}",
                Prose::escape_text(&problem.message)
            ),
        };

        body_lines.push(bullet);

        // Per-problem declared description, reusing the dimmed-italic treatment
        // the document-level `description:` line above already uses. Enrichment
        // suppressed empty / message-equal descriptions, so a `Some` renders.
        if let Some(desc) = &problem.description {
            body_lines.push(format!("<i><dim>{}</dim></i>", Prose::escape_text(desc)));
        }
    }

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("MarkdownError", "schema validation failed"))
        .body(body_lines.join("\n"))
        .hint("Correct the frontmatter so it satisfies the declared $schema (or baseline schema).")
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

    /// Regression: when the context holds the full document (the on-disk load
    /// path), `serde_yaml_ng`'s YAML-relative line must be shifted past the
    /// opening `---` so the gutter marks the real document line — not the
    /// delimiter. Mirrors the `prompt: |--` case from the field report.
    #[test]
    fn frontmatter_parse_block_marks_document_line_not_yaml_line() {
        // Full document INCLUDING delimiters; serde only parses the inner YAML.
        let doc = "---\nprompt: |--\nbody: ok\n---\n";
        let inner = "prompt: |--\nbody: ok";
        let ctx = SourceContext::new(PathBuf::from("/test.md"), PathBuf::from("test.md"), doc);
        assert!(
            ctx.frontmatter.is_some(),
            "delimiters should be detected for the offset shift"
        );
        let err = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(inner).unwrap_err();
        let out = render_block(&frontmatter_parse_block(ctx, &err));

        // The offending `prompt: |--` is document line 2; the gutter marks it.
        assert!(
            out.contains("> 2 │ prompt: |--"),
            "expected marker on document line 2: {out}",
        );
        // The line FOLLOWING the error is shown as trailing context.
        assert!(
            out.contains("3 │ body: ok"),
            "expected following line shown: {out}",
        );
        // The opening delimiter shows as the preceding context line.
        assert!(out.contains("1 │ ---"), "expected preceding line shown: {out}");
    }

    /// The excerpt body carries syntax-highlight SGR (best-effort, tolerant of
    /// the malformed line) while the stripped output still shows a correctly
    /// numbered gutter and the following line.
    #[test]
    fn frontmatter_excerpt_prose_highlights_and_marks_line() {
        use biscuit_terminal::terminal::Terminal;

        let doc = "---\nprompt: |--\nbody: ok\n---\n";
        let ctx = SourceContext::new(PathBuf::from("/t.md"), PathBuf::from("t.md"), doc);
        // Document line 2 is `prompt: |--`.
        let raw = frontmatter_excerpt_prose(&ctx, 2, 1).render(&Terminal::new_optimistic(80));

        assert!(
            raw.contains("\x1b[38;2;"),
            "expected truecolor highlight SGR in excerpt: {raw:?}",
        );

        let plain = strip_escape_codes(&raw);
        assert!(
            plain.contains("> 2 │ prompt: |--"),
            "expected gutter marker on doc line 2: {plain:?}",
        );
        assert!(
            plain.contains("3 │ body: ok"),
            "expected following line in excerpt: {plain:?}",
        );
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

    #[test]
    fn malformed_stored_hash_block_renders_property_and_reason() {
        let out = render_block(&malformed_stored_hash_block(
            "hash",
            "expected one of: fm, body, simple, structured, detailed",
        ));
        assert!(out.contains("MarkdownError"), "missing header type: {out}");
        assert!(out.contains("malformed stored hash"), "missing summary: {out}");
        assert!(out.contains("hash"), "missing property: {out}");
        assert!(
            out.contains("expected one of"),
            "missing reason: {out}"
        );
    }

    /// Preparation failures (malformed `$schema`, unresolved baseline) arrive
    /// with an empty problem list and stash the underlying diagnostic in
    /// `summary`. The rendered block must surface that summary so the user
    /// sees the actual root cause instead of just the path.
    #[test]
    fn schema_validation_failed_block_renders_summary_when_problems_empty() {
        let path = std::path::PathBuf::from("/tmp/test/bad-schema.md");
        let summary = "schema could not be prepared: could not resolve ./missing.yaml";
        let block = schema_validation_failed_block(&path, &[], summary, &None);
        let out = render_block(&block);
        assert!(out.contains("schema could not be prepared"), "missing summary: {out}");
        assert!(out.contains("could not resolve ./missing.yaml"), "missing detail: {out}");
        assert!(out.contains("bad-schema.md"), "missing path: {out}");
    }

    /// When the summary is empty the block should still render a non-empty
    /// body so the user understands schema preparation failed.
    #[test]
    fn schema_validation_failed_block_handles_empty_summary() {
        let path = std::path::PathBuf::from("/tmp/test/empty.md");
        let block = schema_validation_failed_block(&path, &[], "", &None);
        let out = render_block(&block);
        assert!(out.contains("schema could not be prepared"), "missing fallback summary: {out}");
    }

    fn problem_with_description(description: Option<&str>) -> ValidationProblem {
        ValidationProblem {
            path: "/title".to_string(),
            message: "expected string".to_string(),
            kind: ValidationProblemKind::Type,
            property: Some("title".to_string()),
            line: Some(2),
            column: Some(1),
            arm_index: None,
            description: description.map(String::from),
        }
    }

    /// Track C: the per-problem declared description renders as its own sub-line
    /// after the problem bullet, surfacing what the failing property is *for*.
    #[test]
    fn schema_validation_failed_block_renders_problem_description() {
        let path = std::path::PathBuf::from("/tmp/test/post.md");
        let problem = problem_with_description(Some("The headline shown in listing pages"));
        let block = schema_validation_failed_block(&path, std::slice::from_ref(&problem), "", &None);
        let out = render_block(&block);
        assert!(out.contains("type"), "missing problem bullet: {out}");
        assert!(
            out.contains("The headline shown in listing pages"),
            "missing per-problem description sub-line: {out}",
        );
    }

    /// Track C: the document-level `description:` line and the per-problem
    /// description coexist without one clobbering the other.
    #[test]
    fn schema_validation_failed_block_coexists_doc_and_problem_descriptions() {
        let path = std::path::PathBuf::from("/tmp/test/post.md");
        let problem = problem_with_description(Some("The headline shown in listing pages"));
        let doc_description = Some("A blog post about anything".to_string());
        let block = schema_validation_failed_block(
            &path,
            std::slice::from_ref(&problem),
            "",
            &doc_description,
        );
        let out = render_block(&block);
        assert!(
            out.contains("A blog post about anything"),
            "missing document-level description: {out}",
        );
        assert!(
            out.contains("The headline shown in listing pages"),
            "missing per-problem description: {out}",
        );
    }

    /// Track C: a problem without a description emits only its bullet — no
    /// stray description sub-line (Decision #8).
    #[test]
    fn schema_validation_failed_block_omits_absent_problem_description() {
        let path = std::path::PathBuf::from("/tmp/test/post.md");
        let problem = problem_with_description(None);
        let block = schema_validation_failed_block(&path, std::slice::from_ref(&problem), "", &None);
        let out = render_block(&block);
        // The bullet is present, but no extra description text rides beneath it.
        assert!(out.contains("type"), "missing problem bullet: {out}");
        assert!(
            out.contains("expected string"),
            "missing problem message: {out}",
        );
    }

    /// `reqwest::Error` cannot be constructed without firing a real HTTP
    /// request. This smoke test exercises the helper by sending a request to
    /// an address that is guaranteed to refuse connections, producing a real
    /// `reqwest::Error` cheaply.
    ///
    /// ## Notes
    ///
    /// The request targets a loopback port whose listener has been bound
    /// then dropped, guaranteeing `ECONNREFUSED` immediately on both Linux
    /// and macOS. `no_proxy()` skips system-proxy detection (which can be
    /// slow on macOS via SCDynamicStore) and `tcp_nodelay`/short timeouts
    /// ensure the test exits quickly even if the kernel queues the RST.
    #[tokio::test]
    async fn url_fetch_block_renders_with_reqwest_error() {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().expect("local_addr").port();
        drop(listener);
        let url = format!("http://127.0.0.1:{port}");
        let client = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(std::time::Duration::from_millis(500))
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .expect("client builder should not fail");
        let result = client.get(&url).send().await;
        let err = result.expect_err("request to closed port should fail");
        let out = render_block(&url_fetch_block(&err));
        assert!(out.contains("MarkdownError"), "missing header type: {out}");
        assert!(out.contains("URL fetch failed"), "missing summary: {out}");
        assert!(out.contains(&url), "missing URL: {out}");
    }
}
