//! Markdown renderer for the canonical render tree.
//!
//! [`render_markdown_node`] folds a [`RenderNode`] into a Markdown string.
//! [`render_markdown_document`] does the same for a whole [`Document`],
//! prepending any frontmatter block.
//!
//! The renderer aims for *semantic* stability rather than byte-identical
//! source preservation: the output parses back to an equivalent tree, but
//! whitespace and delimiter choices are normalized.
//!
//! ## Examples
//!
//! ```
//! use renderable::tree::{HeadingDepth, RenderNode};
//! use renderable::tree::render::{render_markdown_node, MarkdownRenderOptions};
//!
//! let tree = RenderNode::root(vec![RenderNode::heading(
//!     HeadingDepth::new(2).unwrap(),
//!     vec![RenderNode::text("Title")],
//! )]);
//! let rendered = render_markdown_node(&tree, &MarkdownRenderOptions::default()).unwrap();
//! assert_eq!(rendered.output, "## Title");
//! ```

use crate::tree::diagnostic::{Diagnostic, Severity};
use crate::tree::document::{Document, FrontmatterFormat};
use crate::tree::error::{RenderError, RenderStrictness, Rendered};
use crate::tree::node::{ColumnAlign, NodeKind, RenderNode};
use crate::tree::validate::{ValidationError, ValidationMode, validate};

/// The Markdown dialect a render targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MarkdownDialect {
    /// Standard CommonMark/GFM. Constructs without a Markdown equivalent
    /// (raw HTML, classed spans) are degraded with a diagnostic.
    #[default]
    Markdown,
    /// Markdown enriched with inline HTML for constructs that plain Markdown
    /// cannot express.
    MarkdownPlus,
}

/// Style hints for the Markdown renderer.
///
/// This is a reserved extension point: there is no styling spec for the
/// Markdown target yet, so the renderer currently ignores it. It exists so
/// callers can thread style intent without a future API break.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkdownStyleOptions {}

/// Options controlling a Markdown render.
///
/// The [`Default`] uses [`MarkdownDialect::Markdown`],
/// [`RenderStrictness::Warn`], and no style options.
#[derive(Debug, Clone, Default)]
pub struct MarkdownRenderOptions {
    /// The Markdown dialect to target.
    pub dialect: MarkdownDialect,
    /// How strictly lossy or unsupported content is treated.
    pub strictness: RenderStrictness,
    /// Reserved style hints; currently unused by the renderer.
    pub style: Option<MarkdownStyleOptions>,
}

/// Renders a render tree node to a Markdown string.
///
/// The node is validated with [`validate`] first; an error-severity
/// structural finding causes an immediate [`RenderError::InvalidTree`]
/// regardless of [`MarkdownRenderOptions::strictness`]. Warning-severity
/// findings are folded into [`Rendered::diagnostics`] under
/// [`RenderStrictness::Warn`] and [`RenderStrictness::Lossy`], and escalate to
/// a [`RenderError::InvalidTree`] under [`RenderStrictness::Strict`].
///
/// ## Returns
///
/// A [`Rendered<String>`] carrying the Markdown output and any non-fatal
/// diagnostics.
///
/// ## Errors
///
/// - [`RenderError::InvalidTree`] if the tree fails structural validation, or
///   if [`RenderStrictness::Strict`] meets a warning-severity validation
///   finding (this includes [`NodeKind::Unsupported`] nodes, whose warning is
///   escalated by the validation gate before the writer runs).
/// - [`RenderError::LossyRejected`] if [`RenderStrictness::Strict`] meets a
///   construct that cannot be rendered without loss (raw HTML or a classed
///   span under [`MarkdownDialect::Markdown`]).
pub fn render_markdown_node(
    node: &RenderNode,
    opts: &MarkdownRenderOptions,
) -> Result<Rendered<String>, RenderError> {
    let report = validate(node, ValidationMode::Full);
    if report.has_errors() {
        return Err(ValidationError {
            findings: report.errors().cloned().collect(),
        }
        .into());
    }

    let mut writer = Writer {
        opts,
        diagnostics: Vec::new(),
    };

    // Warning-severity validation findings escalate to an error under Strict
    // and are otherwise folded into the renderer diagnostics.
    for finding in &report.findings {
        if finding.severity != Severity::Warning {
            continue;
        }
        match opts.strictness {
            RenderStrictness::Strict => {
                return Err(RenderError::InvalidTree {
                    findings: report.findings.clone(),
                });
            }
            RenderStrictness::Warn => {
                writer.diagnostics.push(Diagnostic::validation(
                    Severity::Warning,
                    finding.message.clone(),
                    finding.span.clone(),
                ));
            }
            RenderStrictness::Lossy => {}
        }
    }

    let output = writer.render(node)?;
    Ok(Rendered {
        output,
        diagnostics: writer.diagnostics,
    })
}

/// Renders a whole [`Document`] to a Markdown string.
///
/// If [`DocumentMetadata::frontmatter`] is present, the raw frontmatter is
/// prepended as a delimited block before the rendered body. YAML and JSON
/// frontmatter use `---` delimiters; TOML frontmatter uses `+++`.
///
/// The body is produced by [`render_markdown_node`] on [`Document::root`].
///
/// ## Errors
///
/// Propagates every error from [`render_markdown_node`].
///
/// [`DocumentMetadata::frontmatter`]: crate::tree::DocumentMetadata::frontmatter
pub fn render_markdown_document(
    doc: &Document,
    opts: &MarkdownRenderOptions,
) -> Result<Rendered<String>, RenderError> {
    let body = render_markdown_node(&doc.root, opts)?;

    let Some(frontmatter) = &doc.metadata.frontmatter else {
        return Ok(body);
    };

    let delimiter = match frontmatter.format {
        FrontmatterFormat::Yaml | FrontmatterFormat::Json => "---",
        FrontmatterFormat::Toml => "+++",
    };
    let raw = frontmatter.raw.trim_end_matches('\n');
    Ok(body.map(|body| format!("{delimiter}\n{raw}\n{delimiter}\n\n{body}")))
}

/// Threads render options and accumulating diagnostics through the recursion.
struct Writer<'a> {
    opts: &'a MarkdownRenderOptions,
    diagnostics: Vec<Diagnostic>,
}

impl Writer<'_> {
    /// Renders a single node and its subtree.
    fn render(&mut self, node: &RenderNode) -> Result<String, RenderError> {
        match &node.kind {
            NodeKind::Root { children } => self.render_blocks(children),
            NodeKind::Heading { depth, children } => {
                let hashes = "#".repeat(usize::from(depth.get()));
                Ok(format!("{hashes} {}", self.render_inline(children)?))
            }
            NodeKind::Section {
                depth,
                heading,
                children,
            } => {
                let hashes = "#".repeat(usize::from(depth.get()));
                let heading_line = format!("{hashes} {}", self.render_inline(heading)?);
                let body = self.render_blocks(children)?;
                if body.is_empty() {
                    Ok(heading_line)
                } else {
                    Ok(format!("{heading_line}\n\n{body}"))
                }
            }
            NodeKind::Paragraph { children } => self.render_inline(children),
            NodeKind::BlockQuote { children } => {
                if let Some(hints) = node.attrs.columns_hints() {
                    self.render_columns(children, &hints)
                } else {
                    let inner = self.render_blocks(children)?;
                    Ok(prefix_lines(&inner, "> "))
                }
            }
            NodeKind::List {
                ordered,
                start,
                children,
            } => self.render_list(*ordered, *start, children),
            NodeKind::ListItem { checked, children } => {
                let body = self.render_blocks(children)?;
                Ok(match checked {
                    Some(true) => format!("[x] {body}"),
                    Some(false) => format!("[ ] {body}"),
                    None => body,
                })
            }
            NodeKind::Code { lang, meta, value } => {
                let mut fence = String::from("```");
                if let Some(lang) = lang {
                    fence.push_str(lang);
                }
                if let Some(meta) = meta {
                    fence.push(' ');
                    fence.push_str(meta);
                }
                let body = value.trim_end_matches('\n');
                Ok(format!("{fence}\n{body}\n```"))
            }
            NodeKind::ThematicBreak => Ok("---".to_string()),
            NodeKind::Table { align, children } => self.render_table(align, children),
            NodeKind::TableRow { children } => self.render_table_row(children),
            NodeKind::TableCell { children } => self.render_inline(children),
            NodeKind::FootnoteDefinition {
                identifier,
                children,
            } => {
                let body = self.render_blocks(children)?;
                Ok(format!("[^{identifier}]: {body}"))
            }
            NodeKind::Text { value } => Ok(value.clone()),
            NodeKind::Emphasis { children } => Ok(format!("_{}_", self.render_inline(children)?)),
            NodeKind::Strong { children } => Ok(format!("**{}**", self.render_inline(children)?)),
            NodeKind::Delete { children } => Ok(format!("~~{}~~", self.render_inline(children)?)),
            NodeKind::Span { children } => self.render_span(node, children),
            NodeKind::InlineCode { value } => Ok(format!("`{value}`")),
            NodeKind::Link {
                url,
                title,
                children,
            } => {
                let text = self.render_inline(children)?;
                Ok(format!("[{text}]({})", link_target(url, title)))
            }
            NodeKind::Image { url, title, alt } => {
                Ok(format!("![{alt}]({})", link_target(url, title)))
            }
            NodeKind::FootnoteReference { identifier } => Ok(format!("[^{identifier}]")),
            // A soft break is rendered as a newline; the surrounding block
            // is responsible for any further wrapping.
            NodeKind::SoftBreak => Ok("\n".to_string()),
            // A hard break is two trailing spaces followed by a newline.
            NodeKind::HardBreak => Ok("  \n".to_string()),
            NodeKind::Html { value, block } => self.render_html(node, value, *block),
            NodeKind::Unsupported { label } => self.render_unsupported(node, label),
        }
    }

    /// Renders a sequence of block-level nodes, joined by blank lines.
    fn render_blocks(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
        let mut parts = Vec::with_capacity(children.len());
        for child in children {
            parts.push(self.render(child)?);
        }
        Ok(parts.join("\n\n"))
    }

    /// Renders a sequence of inline nodes, concatenated without separators.
    fn render_inline(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
        let mut output = String::new();
        for child in children {
            output.push_str(&self.render(child)?);
        }
        Ok(output)
    }

    /// Renders a two-column block quote as two sequential sections.
    ///
    /// Markdown has no side-by-side layout; the left column's blocks are
    /// emitted first, then a blank line, then the right column's blocks.
    fn render_columns(
        &mut self,
        children: &[RenderNode],
        hints: &crate::tree::ColumnsHints,
    ) -> Result<String, RenderError> {
        let split = hints.left_count.min(children.len());
        let (left, right) = children.split_at(split);
        let left = self.render_blocks(left)?;
        let right = self.render_blocks(right)?;
        Ok(match (left.is_empty(), right.is_empty()) {
            (true, true) => String::new(),
            (false, true) => left,
            (true, false) => right,
            (false, false) => format!("{left}\n\n{right}"),
        })
    }

    /// Renders a list, numbering ordered items from `start`.
    fn render_list(
        &mut self,
        ordered: bool,
        start: Option<u64>,
        children: &[RenderNode],
    ) -> Result<String, RenderError> {
        let mut lines = Vec::with_capacity(children.len());
        let first_index = start.unwrap_or(1);
        for (offset, child) in children.iter().enumerate() {
            let body = self.render(child)?;
            let marker = if ordered {
                let index = first_index + offset as u64;
                format!("{index}. ")
            } else {
                "- ".to_string()
            };
            // Continuation lines are indented to align under the marker.
            let indent = " ".repeat(marker.len());
            lines.push(format!("{marker}{}", indent_continuation(&body, &indent)));
        }
        Ok(lines.join("\n"))
    }

    /// Renders a GFM table; the first child row is treated as the header.
    fn render_table(
        &mut self,
        align: &[ColumnAlign],
        children: &[RenderNode],
    ) -> Result<String, RenderError> {
        let mut lines = Vec::with_capacity(children.len() + 1);
        let mut rows = children.iter();

        if let Some(header) = rows.next() {
            lines.push(self.render(header)?);
            lines.push(delimiter_row(align));
        }
        for row in rows {
            lines.push(self.render(row)?);
        }
        Ok(lines.join("\n"))
    }

    /// Renders a table row as a pipe-delimited line.
    fn render_table_row(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
        let mut cells = Vec::with_capacity(children.len());
        for child in children {
            cells.push(self.render(child)?);
        }
        Ok(format!("| {} |", cells.join(" | ")))
    }

    /// Renders an inline span, degrading classed spans in plain Markdown.
    fn render_span(
        &mut self,
        node: &RenderNode,
        children: &[RenderNode],
    ) -> Result<String, RenderError> {
        let inner = self.render_inline(children)?;
        let classes = &node.attrs.classes;
        if classes.is_empty() {
            return Ok(inner);
        }
        match self.opts.dialect {
            MarkdownDialect::MarkdownPlus => Ok(format!(
                "<span class=\"{}\">{inner}</span>",
                classes.join(" ")
            )),
            MarkdownDialect::Markdown => {
                let message = format!(
                    "span classes [{}] have no plain Markdown equivalent",
                    classes.join(", ")
                );
                match self.opts.strictness {
                    RenderStrictness::Strict => Err(RenderError::LossyRejected { message }),
                    RenderStrictness::Warn => {
                        self.diagnostics
                            .push(Diagnostic::lossy(message, Some(node.span.clone())));
                        Ok(inner)
                    }
                    RenderStrictness::Lossy => Ok(inner),
                }
            }
        }
    }

    /// Renders raw HTML, degrading it under plain Markdown.
    fn render_html(
        &mut self,
        node: &RenderNode,
        value: &str,
        _block: bool,
    ) -> Result<String, RenderError> {
        match self.opts.dialect {
            // Inline HTML is valid MarkdownPlus, so it is emitted verbatim.
            MarkdownDialect::MarkdownPlus => Ok(value.to_string()),
            MarkdownDialect::Markdown => {
                let message = "raw HTML is not portable plain Markdown".to_string();
                match self.opts.strictness {
                    RenderStrictness::Strict => Err(RenderError::LossyRejected { message }),
                    // Under Warn/Lossy the raw value is emitted (CommonMark
                    // permits raw HTML); Warn additionally records it.
                    RenderStrictness::Warn => {
                        self.diagnostics
                            .push(Diagnostic::lossy(message, Some(node.span.clone())));
                        Ok(value.to_string())
                    }
                    RenderStrictness::Lossy => Ok(value.to_string()),
                }
            }
        }
    }

    /// Renders an unsupported node according to strictness.
    fn render_unsupported(
        &mut self,
        node: &RenderNode,
        label: &str,
    ) -> Result<String, RenderError> {
        match self.opts.strictness {
            // Defensive fallback: for trees entered via `render_markdown_node`
            // this arm is preempted because the validation gate escalates the
            // `Unsupported` warning to `RenderError::InvalidTree` first. It
            // still guards any entry into the writer that bypasses that gate.
            RenderStrictness::Strict => Err(RenderError::Unsupported {
                label: label.to_string(),
            }),
            RenderStrictness::Warn => {
                self.diagnostics.push(Diagnostic::unsupported(
                    format!("unsupported content dropped: {label}"),
                    Some(node.span.clone()),
                ));
                Ok(format!("<!-- unsupported: {label} -->"))
            }
            RenderStrictness::Lossy => Ok(String::new()),
        }
    }
}

/// Formats a link/image target, appending a quoted title when present.
fn link_target(url: &str, title: &Option<String>) -> String {
    match title {
        Some(title) => format!("{url} \"{title}\""),
        None => url.to_string(),
    }
}

/// Builds a GFM table delimiter row from per-column alignments.
fn delimiter_row(align: &[ColumnAlign]) -> String {
    let cells: Vec<&str> = align
        .iter()
        .map(|a| match a {
            ColumnAlign::Left => ":--",
            ColumnAlign::Center => ":-:",
            ColumnAlign::Right => "--:",
            ColumnAlign::None => "---",
        })
        .collect();
    format!("| {} |", cells.join(" | "))
}

/// Prefixes every line of `text` with `prefix`.
fn prefix_lines(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| {
            if line.is_empty() {
                prefix.trim_end().to_string()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Indents continuation lines (every line after the first) by `indent`.
fn indent_continuation(text: &str, indent: &str) -> String {
    let mut lines = text.lines();
    let Some(first) = lines.next() else {
        return String::new();
    };
    let mut output = first.to_string();
    for line in lines {
        output.push('\n');
        if !line.is_empty() {
            output.push_str(indent);
        }
        output.push_str(line);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::DocumentMetadata;
    use crate::tree::Frontmatter;
    use crate::tree::SourceRegistry;
    use crate::tree::node::HeadingDepth;

    fn render(node: &RenderNode) -> Rendered<String> {
        render_markdown_node(node, &MarkdownRenderOptions::default()).expect("render")
    }

    fn render_with(node: &RenderNode, opts: &MarkdownRenderOptions) -> Rendered<String> {
        render_markdown_node(node, opts).expect("render")
    }

    fn opts(dialect: MarkdownDialect, strictness: RenderStrictness) -> MarkdownRenderOptions {
        MarkdownRenderOptions {
            dialect,
            strictness,
            style: None,
        }
    }

    #[test]
    fn heading_renders_hashes() {
        let node = RenderNode::heading(
            HeadingDepth::new(3).unwrap(),
            vec![RenderNode::text("Hello")],
        );
        assert_eq!(render(&node).output, "### Hello");
    }

    #[test]
    fn paragraph_renders_inline_children() {
        let node = RenderNode::paragraph(vec![
            RenderNode::text("a "),
            RenderNode::strong(vec![RenderNode::text("b")]),
        ]);
        assert_eq!(render(&node).output, "a **b**");
    }

    #[test]
    fn emphasis_strong_delete() {
        let em = RenderNode::emphasis(vec![RenderNode::text("e")]);
        let st = RenderNode::strong(vec![RenderNode::text("s")]);
        let de = RenderNode::delete(vec![RenderNode::text("d")]);
        assert_eq!(render(&em).output, "_e_");
        assert_eq!(render(&st).output, "**s**");
        assert_eq!(render(&de).output, "~~d~~");
    }

    #[test]
    fn inline_code_and_code_block() {
        assert_eq!(render(&RenderNode::inline_code("x")).output, "`x`");
        let code = RenderNode::code(Some("rust".into()), None, "let a = 1;");
        assert_eq!(render(&code).output, "```rust\nlet a = 1;\n```");
        let meta = RenderNode::code(Some("rust".into()), Some("ignore".into()), "x");
        assert_eq!(render(&meta).output, "```rust ignore\nx\n```");
    }

    #[test]
    fn link_and_image() {
        let link = RenderNode::link(
            "https://example.com",
            Some("Site".into()),
            vec![RenderNode::text("here")],
        );
        assert_eq!(render(&link).output, "[here](https://example.com \"Site\")");
        let image = RenderNode::image("img.png", None, "alt text");
        assert_eq!(render(&image).output, "![alt text](img.png)");
    }

    #[test]
    fn unordered_and_ordered_lists() {
        let ul = RenderNode::list(
            false,
            None,
            vec![
                RenderNode::list_item(
                    None,
                    vec![RenderNode::paragraph(vec![RenderNode::text("a")])],
                ),
                RenderNode::list_item(
                    None,
                    vec![RenderNode::paragraph(vec![RenderNode::text("b")])],
                ),
            ],
        );
        assert_eq!(render(&ul).output, "- a\n- b");

        let ol = RenderNode::list(
            true,
            Some(3),
            vec![
                RenderNode::list_item(
                    None,
                    vec![RenderNode::paragraph(vec![RenderNode::text("x")])],
                ),
                RenderNode::list_item(
                    None,
                    vec![RenderNode::paragraph(vec![RenderNode::text("y")])],
                ),
            ],
        );
        assert_eq!(render(&ol).output, "3. x\n4. y");
    }

    #[test]
    fn task_list_items() {
        let list = RenderNode::list(
            false,
            None,
            vec![
                RenderNode::list_item(
                    Some(true),
                    vec![RenderNode::paragraph(vec![RenderNode::text("done")])],
                ),
                RenderNode::list_item(
                    Some(false),
                    vec![RenderNode::paragraph(vec![RenderNode::text("todo")])],
                ),
            ],
        );
        assert_eq!(render(&list).output, "- [x] done\n- [ ] todo");
    }

    #[test]
    fn thematic_break() {
        assert_eq!(render(&RenderNode::thematic_break()).output, "---");
    }

    #[test]
    fn block_quote_prefixes_lines() {
        let bq = RenderNode::block_quote(vec![
            RenderNode::paragraph(vec![RenderNode::text("one")]),
            RenderNode::paragraph(vec![RenderNode::text("two")]),
        ]);
        assert_eq!(render(&bq).output, "> one\n>\n> two");
    }

    #[test]
    fn table_renders_with_delimiter_row() {
        let table = RenderNode::table(
            vec![ColumnAlign::Left, ColumnAlign::Right],
            vec![
                RenderNode::table_row(vec![
                    RenderNode::table_cell(vec![RenderNode::text("H1")]),
                    RenderNode::table_cell(vec![RenderNode::text("H2")]),
                ]),
                RenderNode::table_row(vec![
                    RenderNode::table_cell(vec![RenderNode::text("a")]),
                    RenderNode::table_cell(vec![RenderNode::text("b")]),
                ]),
            ],
        );
        assert_eq!(
            render(&table).output,
            "| H1 | H2 |\n| :-- | --: |\n| a | b |"
        );
    }

    #[test]
    fn footnotes() {
        let reference = RenderNode {
            kind: NodeKind::FootnoteReference {
                identifier: "1".into(),
            },
            span: crate::tree::SourceSpan::synthetic(),
            attrs: crate::tree::NodeAttrs::default(),
        };
        assert_eq!(render(&reference).output, "[^1]");

        let definition = RenderNode {
            kind: NodeKind::FootnoteDefinition {
                identifier: "1".into(),
                children: vec![RenderNode::paragraph(vec![RenderNode::text("note")])],
            },
            span: crate::tree::SourceSpan::synthetic(),
            attrs: crate::tree::NodeAttrs::default(),
        };
        assert_eq!(render(&definition).output, "[^1]: note");
    }

    #[test]
    fn soft_and_hard_breaks() {
        assert_eq!(render(&RenderNode::soft_break()).output, "\n");
        assert_eq!(render(&RenderNode::hard_break()).output, "  \n");
    }

    #[test]
    fn root_joins_blocks_with_blank_lines() {
        let root = RenderNode::root(vec![
            RenderNode::heading(HeadingDepth::new(1).unwrap(), vec![RenderNode::text("T")]),
            RenderNode::paragraph(vec![RenderNode::text("body")]),
        ]);
        assert_eq!(render(&root).output, "# T\n\nbody");
    }

    #[test]
    fn span_without_classes_renders_children() {
        let span = RenderNode::span(vec![], vec![RenderNode::text("plain")]);
        let rendered = render(&span);
        assert_eq!(rendered.output, "plain");
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn classed_span_degrades_in_plain_markdown_with_diagnostic() {
        let span = RenderNode::span(vec!["hl".into()], vec![RenderNode::text("x")]);
        let rendered = render_with(
            &span,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Warn),
        );
        assert_eq!(rendered.output, "x");
        assert_eq!(rendered.diagnostics.len(), 1);
    }

    #[test]
    fn classed_span_emits_html_in_markdown_plus() {
        let span = RenderNode::span(vec!["hl".into()], vec![RenderNode::text("x")]);
        let rendered = render_with(
            &span,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        );
        assert_eq!(rendered.output, "<span class=\"hl\">x</span>");
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn classed_span_rejected_in_strict_plain_markdown() {
        let span = RenderNode::span(vec!["hl".into()], vec![RenderNode::text("x")]);
        let result = render_markdown_node(
            &span,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Strict),
        );
        assert!(matches!(result, Err(RenderError::LossyRejected { .. })));
    }

    #[test]
    fn html_degrades_in_plain_markdown_with_diagnostic() {
        let html = RenderNode::html("<br>", false);
        let rendered = render_with(
            &html,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Warn),
        );
        assert_eq!(rendered.output, "<br>");
        assert_eq!(rendered.diagnostics.len(), 1);
    }

    #[test]
    fn html_emits_raw_in_markdown_plus() {
        let html = RenderNode::html("<div>x</div>", true);
        let rendered = render_with(
            &html,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        );
        assert_eq!(rendered.output, "<div>x</div>");
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn html_rejected_in_strict_plain_markdown() {
        let html = RenderNode::html("<br>", false);
        let result = render_markdown_node(
            &html,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Strict),
        );
        assert!(matches!(result, Err(RenderError::LossyRejected { .. })));
    }

    #[test]
    fn unsupported_fails_in_strict_mode() {
        // An Unsupported node yields a warning-severity validation finding,
        // which escalates to an InvalidTree error under Strict before the
        // node-level Unsupported path is reached.
        let node = RenderNode::root(vec![RenderNode::unsupported("custom")]);
        let result = render_markdown_node(
            &node,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Strict),
        );
        assert!(matches!(result, Err(RenderError::InvalidTree { .. })));
    }

    #[test]
    fn unsupported_emits_diagnostic_in_warn_mode() {
        let node = RenderNode::root(vec![RenderNode::unsupported("custom")]);
        let rendered = render_with(
            &node,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Warn),
        );
        assert_eq!(rendered.output, "<!-- unsupported: custom -->");
        // One validation-warning diagnostic plus one renderer Unsupported
        // diagnostic.
        assert_eq!(rendered.diagnostics.len(), 2);
    }

    #[test]
    fn unsupported_emits_nothing_in_lossy_mode() {
        let node = RenderNode::root(vec![RenderNode::unsupported("custom")]);
        let rendered = render_with(
            &node,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Lossy),
        );
        assert_eq!(rendered.output, "");
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn invalid_tree_fails_before_output_regardless_of_strictness() {
        // An orphaned TableCell inside a Paragraph: a structural error.
        let bad = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::table_cell(
            vec![RenderNode::text("x")],
        )])]);
        for strictness in [
            RenderStrictness::Strict,
            RenderStrictness::Warn,
            RenderStrictness::Lossy,
        ] {
            let result = render_markdown_node(&bad, &opts(MarkdownDialect::Markdown, strictness));
            assert!(matches!(result, Err(RenderError::InvalidTree { .. })));
        }
    }

    #[test]
    fn warning_validation_finding_folds_into_diagnostics_under_warn() {
        // An Unsupported node yields a warning-severity validation finding.
        let node = RenderNode::root(vec![RenderNode::unsupported("custom")]);
        let rendered = render_with(
            &node,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Warn),
        );
        assert!(
            rendered
                .diagnostics
                .iter()
                .any(|d| d.kind == crate::tree::DiagnosticKind::Validation
                    && d.severity == crate::tree::Severity::Warning
                    && d.message.contains("Unsupported node"))
        );
    }

    #[test]
    fn warning_validation_finding_fails_under_strict() {
        let node = RenderNode::root(vec![RenderNode::unsupported("custom")]);
        let result = render_markdown_node(
            &node,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Strict),
        );
        assert!(matches!(result, Err(RenderError::InvalidTree { .. })));
    }

    #[test]
    fn document_prepends_yaml_frontmatter() {
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata {
                frontmatter: Some(Frontmatter {
                    format: FrontmatterFormat::Yaml,
                    raw: "title: Example".into(),
                }),
            },
            root: RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("Body")])]),
        };
        let rendered =
            render_markdown_document(&doc, &MarkdownRenderOptions::default()).expect("render");
        assert_eq!(rendered.output, "---\ntitle: Example\n---\n\nBody");
    }

    #[test]
    fn document_prepends_toml_frontmatter() {
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata {
                frontmatter: Some(Frontmatter {
                    format: FrontmatterFormat::Toml,
                    raw: "title = \"Example\"".into(),
                }),
            },
            root: RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("Body")])]),
        };
        let rendered =
            render_markdown_document(&doc, &MarkdownRenderOptions::default()).expect("render");
        assert_eq!(rendered.output, "+++\ntitle = \"Example\"\n+++\n\nBody");
    }

    #[test]
    fn document_without_frontmatter_renders_body_only() {
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("Body")])]),
        };
        let rendered =
            render_markdown_document(&doc, &MarkdownRenderOptions::default()).expect("render");
        assert_eq!(rendered.output, "Body");
    }

    #[test]
    fn default_options_use_markdown_warn() {
        let opts = MarkdownRenderOptions::default();
        assert_eq!(opts.dialect, MarkdownDialect::Markdown);
        assert_eq!(opts.strictness, RenderStrictness::Warn);
        assert!(opts.style.is_none());
    }

    #[test]
    fn section_renders_heading_then_body() {
        let section = RenderNode::section(
            HeadingDepth::new(2).unwrap(),
            vec![RenderNode::text("Title")],
            vec![RenderNode::paragraph(vec![RenderNode::text(
                "Body paragraph",
            )])],
        );
        assert_eq!(render(&section).output, "## Title\n\nBody paragraph");
    }

    #[test]
    fn section_with_empty_body_renders_heading_only() {
        let section = RenderNode::section(
            HeadingDepth::new(3).unwrap(),
            vec![RenderNode::text("Just a heading")],
            vec![],
        );
        assert_eq!(render(&section).output, "### Just a heading");
    }

    #[test]
    fn section_with_inline_heading_styles() {
        let section = RenderNode::section(
            HeadingDepth::new(1).unwrap(),
            vec![
                RenderNode::text("Hello "),
                RenderNode::strong(vec![RenderNode::text("World")]),
            ],
            vec![RenderNode::paragraph(vec![RenderNode::text("Content")])],
        );
        assert_eq!(render(&section).output, "# Hello **World**\n\nContent");
    }

    #[test]
    fn markdown_body_is_unchanged_when_layout_is_present() {
        use crate::layout::{Layout, Length, Margin};

        let plain = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("hi")])]);

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            margin: Margin::all(Length::ch(4)),
            ..Layout::default()
        });
        let with_layout = RenderNode::root(vec![para]);

        let opts = MarkdownRenderOptions::default();
        let a = render_markdown_node(&plain, &opts).unwrap();
        let b = render_markdown_node(&with_layout, &opts).unwrap();

        assert_eq!(a.output, b.output, "Markdown body must ignore Layout");
        assert!(
            b.diagnostics.is_empty(),
            "dropping layout from the Markdown body is by design — no diagnostics"
        );
    }

    #[test]
    fn nested_sections_render_correctly() {
        let tree = RenderNode::root(vec![RenderNode::section(
            HeadingDepth::new(1).unwrap(),
            vec![RenderNode::text("Parent")],
            vec![
                RenderNode::paragraph(vec![RenderNode::text("Intro")]),
                RenderNode::section(
                    HeadingDepth::new(2).unwrap(),
                    vec![RenderNode::text("Child")],
                    vec![RenderNode::paragraph(vec![RenderNode::text("Body")])],
                ),
            ],
        )]);
        assert_eq!(
            render(&tree).output,
            "# Parent\n\nIntro\n\n## Child\n\nBody"
        );
    }
}
