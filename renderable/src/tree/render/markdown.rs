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
        table_cell_depth: 0,
        html_depth: 0,
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
        // Markdown-family output never receives feature assets (spec: Markdown
        // neutrality), so the feature side channel stays empty here.
        features: Vec::new(),
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
    /// Non-zero while rendering descendants of a [`NodeKind::TableCell`].
    /// Inside a table cell, text is escaped so literal pipes and newlines
    /// cannot corrupt the GFM pipe-delimited table structure.
    table_cell_depth: u32,
    /// Non-zero while rendering the body of a MarkdownPlus inline-HTML span
    /// (a classed or styled [`NodeKind::Span`]). Inside that body, text nodes
    /// HTML-escape `<`, `>`, and `&` so literal markup stays inert, while
    /// generated Markdown sigils and nested span tags pass through unescaped.
    html_depth: u32,
}

impl Writer<'_> {
    /// Renders a single node and its subtree.
    fn render(&mut self, node: &RenderNode) -> Result<String, RenderError> {
        match &node.kind {
            NodeKind::Root { children } => {
                // A `Compose`-style sequence joins children in order with no
                // renderer-inserted separator; a normal document root joins
                // children as blocks with blank-line separators.
                match node.attrs.sequence_join() {
                    Some(crate::tree::SequenceJoin::None) => self.render_sequence(children),
                    None => self.render_blocks(children),
                }
            }
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
            NodeKind::Paragraph { children } => {
                // A projected `Progress` widget carries `ProgressHints`. Plain
                // Markdown renders the paragraph fallback text; MarkdownPlus
                // emits the same semantic progress HTML as the browser.
                match node.attrs.progress_hints_ref() {
                    Some(hints) if self.opts.dialect == MarkdownDialect::MarkdownPlus => {
                        let text = self.render_inline(children)?;
                        Ok(progress_html(hints, &text))
                    }
                    _ => self.render_inline(children),
                }
            }
            NodeKind::BlockQuote { children } => {
                if let Some(hints) = node.attrs.columns_hints_ref() {
                    self.render_columns(children, hints)
                } else {
                    let inner = self.render_blocks(children)?;
                    Ok(prefix_lines(&inner, "> "))
                }
            }
            NodeKind::List {
                ordered,
                start,
                children,
            } => self.render_list(node, *ordered, *start, children),
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
            NodeKind::Table { align, children } => {
                let table = self.render_table(align, children)?;
                // A table title/caption is emitted as escaped plain text on
                // its own line before the table, separated by a blank line.
                // An empty or whitespace-only title is ignored.
                match node.attrs.table_title_ref() {
                    Some(title) if !title.trim().is_empty() => {
                        Ok(format!("{}\n\n{table}", escape_text(title.trim())))
                    }
                    _ => Ok(table),
                }
            }
            NodeKind::TableRow { children } => self.render_table_row(children),
            NodeKind::TableCell { children } => {
                // Descendants of a table cell render in cell-escaping mode so
                // literal pipes and newlines cannot break GFM table structure.
                self.table_cell_depth += 1;
                let result = self.render_inline(children);
                self.table_cell_depth -= 1;
                result
            }
            NodeKind::FootnoteDefinition {
                identifier,
                children,
            } => {
                let body = self.render_blocks(children)?;
                Ok(format!("[^{identifier}]: {body}"))
            }
            NodeKind::Text { value } => Ok(self.render_text(value)),
            NodeKind::Emphasis { children } => Ok(format!("_{}_", self.render_inline(children)?)),
            NodeKind::Strong { children } => Ok(format!("**{}**", self.render_inline(children)?)),
            NodeKind::Delete { children } => Ok(format!("~~{}~~", self.render_inline(children)?)),
            NodeKind::Span { children } => self.render_span(node, children),
            NodeKind::InlineCode { value } => Ok(if self.table_cell_depth > 0 {
                // A literal pipe inside inline code still breaks a GFM table
                // cell, so it is escaped even though the run is code.
                format!("`{}`", value.replace('|', "\\|"))
            } else {
                format!("`{value}`")
            }),
            NodeKind::Link {
                url,
                title,
                children,
            } => {
                let text = self.render_inline(children)?;
                Ok(format!("[{text}]({})", self.link_target(url, title)))
            }
            NodeKind::Image { url, title, alt } => {
                // The alt text is a literal Markdown segment, so inside a
                // table cell it is escaped just like a `Text` node.
                let alt = if self.table_cell_depth > 0 {
                    escape_table_cell_text(alt)
                } else {
                    alt.clone()
                };
                Ok(format!("![{alt}]({})", self.link_target(url, title)))
            }
            NodeKind::FootnoteReference { identifier } => Ok(format!("[^{identifier}]")),
            // A soft break is rendered as a newline; the surrounding block
            // is responsible for any further wrapping. Inside a table cell a
            // soft break collapses to a single space.
            NodeKind::SoftBreak => Ok(if self.table_cell_depth > 0 {
                " ".to_string()
            } else {
                "\n".to_string()
            }),
            // A hard break is two trailing spaces followed by a newline.
            // Inside a table cell it becomes `<br>` so the row stays valid.
            NodeKind::HardBreak => Ok(if self.table_cell_depth > 0 {
                "<br>".to_string()
            } else {
                "  \n".to_string()
            }),
            NodeKind::Html { value, block } => self.render_html(node, value, *block),
            NodeKind::Extended {
                token, children, ..
            } => self.render_extended(token, children),
            NodeKind::Unsupported { label } => self.render_unsupported(node, label),
            NodeKind::Disclosure { summary, children, .. } => self.render_disclosure(summary, children),
        }
    }

    /// Renders a disclosure block.
    ///
    /// Plain Markdown emits the `::disclosure / ::details / ::end-disclosure`
    /// DSL verbatim so the output remains a clean Darkmatter document.
    /// MarkdownPlus renders the summary and body to Markdown first, then wraps
    /// them with `<details>`/`<summary>`; the summary is rendered inside an
    /// inline-HTML context so `<`, `>`, and `&` are escaped.
    fn render_disclosure(
        &mut self,
        summary: &[RenderNode],
        children: &[RenderNode],
    ) -> Result<String, RenderError> {
        match self.opts.dialect {
            MarkdownDialect::Markdown => {
                let summary_text = self.render_inline(summary)?;
                let body_text = self.render_blocks(children)?;
                Ok(format!(
                    "::disclosure\n{summary_text}\n::details\n{body_text}\n::end-disclosure"
                ))
            }
            MarkdownDialect::MarkdownPlus => {
                self.html_depth += 1;
                let summary_text = self.render_inline(summary);
                self.html_depth -= 1;
                let summary_text = summary_text?;
                let body_text = self.render_blocks(children)?;
                Ok(format!(
                    "<details><summary>{summary_text}</summary>\n\n{body_text}\n</details>"
                ))
            }
        }
    }

    /// Renders a [`NodeKind::Extended`] node.
    ///
    /// Built-in tokens roundtrip to their darkmatter source syntax: `mark`
    /// emits `==children==` and `dim` emits `⌄children⌄` (U+2304). A token
    /// without a Markdown spelling falls back to rendering its `children` as
    /// plain inline Markdown.
    fn render_extended(
        &mut self,
        token: &str,
        children: &[RenderNode],
    ) -> Result<String, RenderError> {
        let inner = self.render_inline(children)?;
        Ok(match token {
            "mark" => format!("=={inner}=="),
            "dim" => format!("\u{2304}{inner}\u{2304}"),
            _ => inner,
        })
    }

    /// Renders a sequence of block-level nodes, joined by blank lines.
    fn render_blocks(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
        let mut parts = Vec::with_capacity(children.len());
        for child in children {
            parts.push(self.render(child)?);
        }
        let mut result = String::new();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                if part.is_empty() || parts[i - 1].is_empty() {
                    result.push('\n');
                } else {
                    result.push_str("\n\n");
                }
            }
            result.push_str(part);
        }
        Ok(result)
    }

    /// Renders a sequence of children in order with no inserted separator.
    ///
    /// This is the `Compose`-style join: adjacent children concatenate
    /// directly, preserving the component's no-separator contract instead of
    /// the document-block blank-line spacing of [`Self::render_blocks`].
    fn render_sequence(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
        let mut output = String::new();
        for child in children {
            output.push_str(&self.render(child)?);
        }
        Ok(output)
    }

    /// Renders a sequence of inline nodes, concatenated without separators.
    fn render_inline(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
        let mut output = String::new();
        for child in children {
            output.push_str(&self.render(child)?);
        }
        Ok(output)
    }

    /// Renders a two-column block quote.
    ///
    /// Portable [`MarkdownDialect::Markdown`] has no side-by-side layout: the
    /// left column's blocks are emitted first, then a blank line, then the
    /// right column's blocks. [`MarkdownDialect::MarkdownPlus`] emits a block
    /// HTML flex container equivalent to the browser shape.
    fn render_columns(
        &mut self,
        children: &[RenderNode],
        hints: &crate::tree::ColumnsHints,
    ) -> Result<String, RenderError> {
        let split = hints.left_count.min(children.len());
        let (left, right) = children.split_at(split);

        if self.opts.dialect == MarkdownDialect::MarkdownPlus {
            return self.render_columns_html(left, right, hints);
        }

        let left = self.render_blocks(left)?;
        let right = self.render_blocks(right)?;
        Ok(match (left.is_empty(), right.is_empty()) {
            (true, true) => String::new(),
            (false, true) => left,
            (true, false) => right,
            (false, false) => format!("{left}\n\n{right}"),
        })
    }

    /// Renders a two-column layout as a block HTML flex container for
    /// MarkdownPlus.
    ///
    /// The shape mirrors the browser renderer: an outer
    /// `<div class="columns" style="display:flex;gap:{gap}ch">` with two
    /// `<div class="column">` children. The left column carries the width CSS
    /// from [`ColumnsHints::left_width`]; the right column flexes to fill.
    /// `Layout` is not applied — Markdown ignores layout by contract.
    ///
    /// Child blocks are rendered through the active Markdown renderer and
    /// joined with blank lines, the same as any block sequence. A column with
    /// multiple blocks therefore contains a blank line inside the raw HTML
    /// container. This is the accepted parser constraint: a strict CommonMark
    /// parser may treat that blank line as terminating the raw HTML block, so
    /// the trailing `</div>` markup can leak into the rendered output. The
    /// shape is correct for non-strict/MarkdownPlus consumers; single-block
    /// columns are unaffected.
    fn render_columns_html(
        &mut self,
        left: &[RenderNode],
        right: &[RenderNode],
        hints: &crate::tree::ColumnsHints,
    ) -> Result<String, RenderError> {
        let container_css = super::shared::columns_container_css(hints, "");
        let left_css = super::shared::left_column_css(hints.left_width);
        let right_css = super::shared::right_column_css();

        let left_body = self.render_blocks(left)?;
        let right_body = self.render_blocks(right)?;

        Ok(format!(
            concat!(
                r#"<div class="columns" style="{container}">"#,
                r#"<div class="column" style="{left_css}">{left}</div>"#,
                r#"<div class="column" style="{right_css}">{right}</div>"#,
                "</div>",
            ),
            container = container_css,
            left_css = left_css,
            left = left_body,
            right_css = right_css,
            right = right_body,
        ))
    }

    /// Renders a list, numbering ordered items from `start`.
    ///
    /// A typed list marker policy other than
    /// [`ListMarkerPolicy::Default`](crate::tree::ListMarkerPolicy::Default)
    /// cannot be faithfully represented in portable Markdown — Markdown has no
    /// no-marker list and no connector geometry. The list degrades to native
    /// CommonMark list syntax: under [`RenderStrictness::Strict`] this is a
    /// [`RenderError::LossyRejected`]; under [`RenderStrictness::Warn`] a
    /// lossy diagnostic is recorded; under [`RenderStrictness::Lossy`] it is
    /// silent.
    fn render_list(
        &mut self,
        node: &RenderNode,
        ordered: bool,
        start: Option<u64>,
        children: &[RenderNode],
    ) -> Result<String, RenderError> {
        let policy = node.attrs.list_marker_policy();
        if policy != crate::tree::ListMarkerPolicy::Default {
            let message = format!(
                "list marker policy '{}' has no portable Markdown equivalent; \
                 degraded to a native list",
                policy.to_token()
            );
            match self.opts.strictness {
                RenderStrictness::Strict => {
                    return Err(RenderError::LossyRejected { message });
                }
                RenderStrictness::Warn => {
                    self.diagnostics
                        .push(Diagnostic::lossy(message, Some(node.span.clone())));
                }
                RenderStrictness::Lossy => {}
            }
        }

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

    /// Renders an inline span.
    ///
    /// Under [`MarkdownDialect::MarkdownPlus`] a span carrying CSS-bearing
    /// classes or a concrete inline [`Style`](crate::style::Style) (foreground
    /// color, background color, or an underline variant) lowers to a single
    /// inline `<span>` carrying a `class` and/or `style` attribute, with both
    /// attributes coalesced onto one element when present. Its body is rendered
    /// with `<`, `>`, and `&` HTML-escaped so literal markup stays inert.
    /// Emphasis layers without a MarkdownPlus CSS form here (`dim`, `blink`,
    /// `inverse`) carry no inline style and degrade to inner text.
    ///
    /// Under plain [`MarkdownDialect::Markdown`] a classed or styled span has
    /// no portable equivalent and degrades per the strictness model: rejected
    /// under [`RenderStrictness::Strict`], a lossy diagnostic plus inner text
    /// under [`RenderStrictness::Warn`], and silent inner text under
    /// [`RenderStrictness::Lossy`].
    fn render_span(
        &mut self,
        node: &RenderNode,
        children: &[RenderNode],
    ) -> Result<String, RenderError> {
        let classes = &node.attrs.classes;
        let style_css = node
            .attrs
            .style_ref()
            .filter(|style| !style.is_empty())
            .and_then(markdown_plus_style_css);

        match self.opts.dialect {
            MarkdownDialect::MarkdownPlus => {
                if classes.is_empty() && style_css.is_none() {
                    return self.render_inline(children);
                }
                // The children become the body of an inline HTML element, so
                // descendant text HTML-escapes its markup characters.
                self.html_depth += 1;
                let inner = self.render_inline(children);
                self.html_depth -= 1;
                let inner = inner?;

                let mut attrs = String::new();
                if !classes.is_empty() {
                    attrs.push_str(&format!(" class=\"{}\"", classes.join(" ")));
                }
                if let Some(css) = &style_css {
                    attrs.push_str(&format!(" style=\"{css}\""));
                }
                Ok(format!("<span{attrs}>{inner}</span>"))
            }
            MarkdownDialect::Markdown => {
                let inner = self.render_inline(children)?;
                if classes.is_empty() && style_css.is_none() {
                    return Ok(inner);
                }
                let message = span_lossy_message(classes, style_css.is_some());
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

    /// Renders a text node, applying HTML-body escaping inside a MarkdownPlus
    /// inline-HTML span and GFM table-cell escaping inside a table cell.
    fn render_text(&self, value: &str) -> String {
        let mut text = if self.html_depth > 0 {
            escape_inline_html_body(value)
        } else {
            value.to_string()
        };
        if self.table_cell_depth > 0 {
            text = escape_table_cell_text(&text);
        }
        text
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

    /// Formats a link/image target, appending a quoted title when present.
    ///
    /// Inside a table cell the URL and title are escaped the same way a
    /// `Text` node is — a literal `|` or newline in either value would
    /// otherwise split the GFM pipe-delimited row. Outside a table cell the
    /// destination is escaped per CommonMark by
    /// [`escape_markdown_destination`] so parentheses, backslashes, and
    /// whitespace cannot truncate or break the destination.
    fn link_target(&self, url: &str, title: &Option<String>) -> String {
        if self.table_cell_depth > 0 {
            // A cell destination faces both concerns: CommonMark bare-destination
            // safety (escape `(`, `)`, `\` — matching standalone Prose) and GFM
            // row safety (`|` → `\|`, newline → `<br>`). The title keeps the
            // GFM-only escaping it already had.
            let dest = escape_cell_link_destination(url);
            return match title {
                Some(title) => format!("{dest} \"{}\"", escape_table_cell_text(title)),
                None => dest,
            };
        }
        let dest = escape_markdown_destination(url);
        match title {
            Some(title) => format!("{dest} \"{title}\""),
            None => dest,
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

/// HTML-escapes text for embedding in inline/block HTML or as plain text.
fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escapes a text node for safe placement inside a GFM table cell.
///
/// Literal pipes are escaped as `\|`; literal newlines become `<br>` so the
/// cell does not split the pipe-delimited row.
fn escape_table_cell_text(text: &str) -> String {
    text.replace('|', "\\|")
        .replace("\r\n", "<br>")
        .replace('\n', "<br>")
}

/// Escapes a link/image destination for placement inside a GFM table cell.
///
/// Combines the two concerns a cell destination faces: CommonMark bare-
/// destination safety (a literal `(`, `)`, or `\` is backslash-escaped, matching
/// standalone Prose via [`escape_markdown_destination`]) and GFM row safety (a
/// literal `|` becomes `\|`, a newline becomes `<br>`). An ordinary destination
/// is therefore byte-identical to a standalone Prose link while a pipe or
/// newline can never split the pipe-delimited row.
fn escape_cell_link_destination(url: &str) -> String {
    let mut out = String::with_capacity(url.len());
    for c in url.chars() {
        match c {
            '\\' | '(' | ')' => {
                out.push('\\');
                out.push(c);
            }
            '|' => out.push_str("\\|"),
            '\n' => out.push_str("<br>"),
            '\r' => {}
            _ => out.push(c),
        }
    }
    out
}

/// HTML-escapes `<`, `>`, and `&` for text placed inside the body of a
/// MarkdownPlus inline-HTML element, so literal markup stays inert.
///
/// Markdown sigils are intentionally left untouched — CommonMark still parses
/// Markdown inside inline HTML, so a nested `Strong`/`Emphasis` node's `**` /
/// `_` must survive. `"` is not escaped because the body is element content,
/// not an attribute value.
fn escape_inline_html_body(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Lowers the MarkdownPlus-supported layers of a [`Style`](crate::style::Style)
/// to one inline CSS declaration string: foreground color, background color,
/// and the underline variant. Returns `None` when none of those layers is set.
///
/// Colors share the browser's [`paint_to_css_color`](super::shared) lowering:
/// an opaque RGB color is `rgb(r, g, b)`, alpha lowers to `rgba(...)`, the
/// `transparent` / `currentColor` / `inherit` keywords pass through, and the
/// terminal default / reset colors emit no declaration. `dim`, `blink`, and
/// `inverse` have no MarkdownPlus CSS form here and are omitted, so a span
/// carrying only those degrades to inner text.
fn markdown_plus_style_css(style: &crate::style::Style) -> Option<String> {
    use crate::color::ColorMode;
    use crate::target::RenderTarget;

    let resolve = |tv: &crate::layout::TargetValue<
        crate::style::PerMode<crate::style::PaintColor>,
    >|
     -> Option<String> {
        tv.resolve(RenderTarget::MarkdownPlus)
            .map(|per_mode| *per_mode.resolve(ColorMode::Dark))
            .and_then(super::shared::paint_to_css_color)
            .map(|css| css.to_string())
    };

    let mut decls: Vec<String> = Vec::new();
    if let Some(color) = style.color.as_ref().and_then(&resolve) {
        decls.push(format!("color: {color}"));
    }
    if let Some(bg) = style.background.as_ref().and_then(&resolve) {
        decls.push(format!("background-color: {bg}"));
    }
    if let Some(underline) = style.emphasis.underline {
        decls.push(underline.css_declaration().to_string());
    }
    if decls.is_empty() {
        None
    } else {
        Some(decls.join("; "))
    }
}

/// Builds the lossy-degradation message for a classed and/or styled span that
/// has no portable plain-Markdown equivalent.
fn span_lossy_message(classes: &[String], has_style: bool) -> String {
    match (classes.is_empty(), has_style) {
        (false, true) => format!(
            "span classes [{}] and inline style have no plain Markdown equivalent",
            classes.join(", ")
        ),
        (false, false) => format!(
            "span classes [{}] have no plain Markdown equivalent",
            classes.join(", ")
        ),
        (true, _) => "inline span style has no plain Markdown equivalent".to_string(),
    }
}

/// Escapes a link/image destination so it survives intact as a CommonMark
/// link destination.
///
/// A *bare* destination may not contain ASCII whitespace, and any parentheses
/// or backslashes are backslash-escaped so the first unescaped `)` cannot
/// truncate the destination. A destination containing whitespace is emitted in
/// *angle-bracket* form (`<…>`), which permits spaces but forbids unescaped
/// `<` and `>`; line endings, invalid in either form, degrade to spaces.
fn escape_markdown_destination(url: &str) -> String {
    if url.chars().any(|c| c.is_ascii_whitespace()) {
        let mut out = String::with_capacity(url.len() + 2);
        out.push('<');
        for c in url.chars() {
            match c {
                '\\' | '<' | '>' => {
                    out.push('\\');
                    out.push(c);
                }
                '\n' | '\r' | '\t' => out.push(' '),
                _ => out.push(c),
            }
        }
        out.push('>');
        out
    } else {
        let mut out = String::with_capacity(url.len());
        for c in url.chars() {
            match c {
                '\\' | '(' | ')' => {
                    out.push('\\');
                    out.push(c);
                }
                _ => out.push(c),
            }
        }
        out
    }
}

/// Builds the semantic progress-widget HTML emitted by MarkdownPlus.
///
/// The shape mirrors the browser renderer's progress HTML: an outer element
/// with `role="progressbar"` and ARIA attributes, plus `progress-*` classes.
/// Color slots lower to inline CSS; non-default glyphs and brackets are
/// preserved in `data-*` attributes. `Layout` is not applied — Markdown
/// ignores layout by contract — so the outer style carries only color.
fn progress_html(hints: &crate::tree::ProgressHints, fallback_text: &str) -> String {
    super::shared::progress_html(hints, fallback_text, "")
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
    fn extended_unknown_token_renders_children_transparently() {
        // An unrecognized token has no Markdown spelling, so it drops its
        // wrapper and renders its children.
        let node =
            RenderNode::extended("custom-token", vec![RenderNode::text("highlighted")], None);
        assert_eq!(render(&node).output, "highlighted");

        // Nested inline content is preserved through the fallback.
        let nested = RenderNode::extended(
            "custom-token",
            vec![
                RenderNode::text("a "),
                RenderNode::strong(vec![RenderNode::text("b")]),
            ],
            None,
        );
        assert_eq!(render(&nested).output, "a **b**");
    }

    #[test]
    fn extended_mark_and_dim_roundtrip_to_source_syntax() {
        // `mark` roundtrips to `==…==` and `dim` to `⌄…⌄` (U+2304).
        let mark = RenderNode::extended("mark", vec![RenderNode::text("highlighted")], None);
        assert_eq!(render(&mark).output, "==highlighted==");

        let dim = RenderNode::extended("dim", vec![RenderNode::text("quiet")], None);
        assert_eq!(render(&dim).output, "\u{2304}quiet\u{2304}");

        // Nested mark/dim preserves the inner markdown.
        let nested = RenderNode::extended(
            "mark",
            vec![RenderNode::extended(
                "dim",
                vec![RenderNode::strong(vec![RenderNode::text("b")])],
                None,
            )],
            None,
        );
        assert_eq!(render(&nested).output, "==\u{2304}**b**\u{2304}==");
    }

    #[test]
    fn inverse_style_degrades_to_inner_text_in_both_dialects() {
        use crate::style::{Style, TextEmphasis};
        // A `Span` carrying inverse emphasis and no classes has no Markdown
        // sigil: both dialects degrade it to its inner text.
        let mut span = RenderNode::span(vec![], vec![RenderNode::text("loud")]);
        span.attrs.set_style(&Style {
            emphasis: TextEmphasis {
                inverse: true,
                ..Default::default()
            },
            ..Default::default()
        });
        for dialect in [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus] {
            let out = render_with(&span, &opts(dialect, RenderStrictness::Lossy)).output;
            assert_eq!(out, "loud", "dialect {dialect:?}");
        }
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
    fn browser_only_attrs_are_dropped_in_both_markdown_dialects() {
        // Browser-only typed attributes (link target/rel/download, image
        // loading/decoding, data-*, inline_style) have no portable Markdown
        // spelling and no MarkdownPlus HTML form on links/images, so both
        // dialects emit the plain `[text](url)` / `![alt](url)` form,
        // byte-identical to the same nodes without browser attrs.
        let mut link =
            RenderNode::link("https://example.com", None, vec![RenderNode::text("here")]);
        let mut image = RenderNode::image("img.png", None, "alt");
        let mut browser = crate::tree::BrowserAttrs {
            link: Some(crate::tree::LinkBrowserAttrs {
                target: Some(crate::tree::LinkTarget::Blank),
                rel: vec![crate::tree::LinkRelation::NoOpener],
                download: Some("f.txt".into()),
            }),
            inline_style: Some(crate::stylesheet::CssStyle::new().add(
                crate::stylesheet::CssColorProp::Color,
                crate::stylesheet::CssColor::rgb(1, 2, 3),
            )),
            ..Default::default()
        };
        browser.data_attrs.insert(
            crate::tree::DataAttrName::new("prompt").unwrap(),
            "x".into(),
        );
        link.attrs.set_browser(&browser);

        let image_browser = crate::tree::BrowserAttrs {
            image: Some(crate::tree::ImageBrowserAttrs {
                loading: Some(crate::tree::ImageLoading::Lazy),
                decoding: Some(crate::tree::ImageDecoding::Async),
            }),
            ..Default::default()
        };
        image.attrs.set_browser(&image_browser);

        for dialect in [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus] {
            let o = opts(dialect, RenderStrictness::Warn);
            assert_eq!(
                render_with(&link, &o).output,
                "[here](https://example.com)",
                "dialect {dialect:?}"
            );
            assert_eq!(
                render_with(&image, &o).output,
                "![alt](img.png)",
                "dialect {dialect:?}"
            );
        }
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
        use crate::layout::{Layout, Length, Edges};

        let plain = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("hi")])]);

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            margin: Edges::all(Length::ch(4)),
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

    // ── RT-COMPOSE-001: sequence join ──────────────────────────────────────

    #[test]
    fn root_without_sequence_join_keeps_blank_line_separators() {
        let root = RenderNode::root(vec![
            RenderNode::paragraph(vec![RenderNode::text("foo")]),
            RenderNode::paragraph(vec![RenderNode::text("bar")]),
        ]);
        assert_eq!(render(&root).output, "foo\n\nbar");
    }

    #[test]
    fn root_with_sequence_join_concatenates_without_separator() {
        let mut root = RenderNode::root(vec![RenderNode::text("foo"), RenderNode::text("bar")]);
        root.attrs
            .set_sequence_join(crate::tree::SequenceJoin::None);
        assert_eq!(render(&root).output, "foobar");
    }

    #[test]
    fn nested_sequence_children_keep_own_block_semantics() {
        // A sequence join is Root-only and does not propagate: a nested
        // BlockQuote inside a sequence still joins its own blocks normally.
        let inner = RenderNode::block_quote(vec![
            RenderNode::paragraph(vec![RenderNode::text("one")]),
            RenderNode::paragraph(vec![RenderNode::text("two")]),
        ]);
        let mut outer = RenderNode::root(vec![RenderNode::text("lead"), inner]);
        outer
            .attrs
            .set_sequence_join(crate::tree::SequenceJoin::None);
        // The two text/blockquote children concatenate with no separator;
        // the block quote internally keeps its `>` blank-line spacing.
        assert_eq!(render(&outer).output, "lead> one\n>\n> two");
    }

    #[test]
    fn sequence_join_mixed_inline_and_block_children() {
        let mut root = RenderNode::root(vec![
            RenderNode::text("inline"),
            RenderNode::paragraph(vec![RenderNode::text("para")]),
        ]);
        root.attrs
            .set_sequence_join(crate::tree::SequenceJoin::None);
        assert_eq!(render(&root).output, "inlinepara");
    }

    // ── RT-PROGRESS-002: MarkdownPlus progress ─────────────────────────────

    #[test]
    fn progress_plain_markdown_renders_fallback_text() {
        let mut para = RenderNode::paragraph(vec![RenderNode::text("Loading 60%")]);
        para.attrs.set_progress_hints(&crate::tree::ProgressHints {
            value: 0.6,
            ..Default::default()
        });
        let rendered = render_with(
            &para,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Warn),
        );
        assert_eq!(rendered.output, "Loading 60%");
    }

    #[test]
    fn progress_markdown_plus_emits_progress_html() {
        let mut para = RenderNode::paragraph(vec![RenderNode::text("Loading 60%")]);
        para.attrs.set_progress_hints(&crate::tree::ProgressHints {
            value: 0.6,
            ..Default::default()
        });
        let rendered = render_with(
            &para,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        );
        let out = rendered.output;
        assert!(out.contains(r#"role="progressbar""#), "{out}");
        assert!(out.contains(r#"aria-valuenow="60""#), "{out}");
        assert!(out.contains(r#"aria-label="Loading""#), "{out}");
        assert!(out.contains("progress-percentage"), "{out}");
    }

    #[test]
    fn progress_markdown_plus_preserves_custom_glyphs_in_data_attrs() {
        let mut para = RenderNode::paragraph(vec![RenderNode::text("75%")]);
        para.attrs.set_progress_hints(&crate::tree::ProgressHints {
            value: 0.75,
            fill_char: '#',
            empty_char: '-',
            ..Default::default()
        });
        let rendered = render_with(
            &para,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        );
        assert!(rendered.output.contains(r##"data-fill-char="#""##));
        assert!(rendered.output.contains(r#"data-empty-char="-""#));
    }

    #[test]
    fn progress_markdown_plus_lowers_color_slots() {
        use crate::color::{BasicColor, Color};
        let mut para = RenderNode::paragraph(vec![RenderNode::text("50%")]);
        para.attrs.set_progress_hints(&crate::tree::ProgressHints {
            value: 0.5,
            filled_color: Some(Color::BasicColor(BasicColor::Green)),
            ..Default::default()
        });
        let rendered = render_with(
            &para,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        );
        assert!(
            rendered.output.contains("background-color:#"),
            "{}",
            rendered.output
        );
    }

    #[test]
    fn progress_markdown_plus_preserves_bracket_color_as_data_attribute() {
        use crate::color::{BasicColor, Color};
        let mut para = RenderNode::paragraph(vec![RenderNode::text("50%")]);
        para.attrs.set_progress_hints(&crate::tree::ProgressHints {
            value: 0.5,
            bracket_color: Some(Color::BasicColor(BasicColor::Cyan)),
            ..Default::default()
        });
        let rendered = render_with(
            &para,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        );
        assert!(
            rendered.output.contains("data-bracket-color=\"#"),
            "bracket color preserved as data attribute in MarkdownPlus: {}",
            rendered.output
        );
    }

    #[test]
    fn progress_markdown_plus_omits_bracket_color_attribute_when_unset() {
        let mut para = RenderNode::paragraph(vec![RenderNode::text("50%")]);
        para.attrs.set_progress_hints(&crate::tree::ProgressHints {
            value: 0.5,
            ..Default::default()
        });
        let rendered = render_with(
            &para,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        );
        assert!(
            !rendered.output.contains("data-bracket-color"),
            "no bracket color attribute in MarkdownPlus when unset: {}",
            rendered.output
        );
    }

    #[test]
    fn plain_paragraph_without_progress_unchanged_in_markdown_plus() {
        let para = RenderNode::paragraph(vec![RenderNode::text("ordinary")]);
        let rendered = render_with(
            &para,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        );
        assert_eq!(rendered.output, "ordinary");
    }

    // ── RT-TABLE-001: table title ──────────────────────────────────────────

    fn sample_table() -> RenderNode {
        RenderNode::table(
            vec![ColumnAlign::Left],
            vec![
                RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("H")])]),
                RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("a")])]),
            ],
        )
    }

    #[test]
    fn table_title_emitted_as_plain_text_before_table() {
        let mut table = sample_table();
        table.attrs.set_table_title("Results & Notes");
        let out = render(&table).output;
        // The title appears, escaped, on its own line before the table.
        assert!(out.starts_with("Results &amp; Notes\n\n| H |"), "{out}");
    }

    #[test]
    fn whitespace_only_table_title_is_ignored() {
        let mut table = sample_table();
        table.attrs.set_table_title("   ");
        let out = render(&table).output;
        assert!(out.starts_with("| H |"), "{out}");
    }

    // ── RT-TABLE-002: table-cell escaping ──────────────────────────────────

    fn one_cell_table(cell: RenderNode) -> RenderNode {
        RenderNode::table(
            vec![ColumnAlign::None],
            vec![
                RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("H")])]),
                RenderNode::table_row(vec![RenderNode::table_cell(vec![cell])]),
            ],
        )
    }

    #[test]
    fn table_cell_escapes_literal_pipe() {
        let table = one_cell_table(RenderNode::text("a | b"));
        let out = render(&table).output;
        assert!(out.contains(r"a \| b"), "{out}");
    }

    #[test]
    fn table_cell_normalizes_literal_newline_to_br() {
        let table = one_cell_table(RenderNode::text("line1\nline2"));
        let out = render(&table).output;
        assert!(out.contains("line1<br>line2"), "{out}");
    }

    #[test]
    fn table_cell_soft_break_collapses_to_space() {
        let table = one_cell_table(RenderNode::span(
            vec![],
            vec![
                RenderNode::text("a"),
                RenderNode::soft_break(),
                RenderNode::text("b"),
            ],
        ));
        let out = render(&table).output;
        assert!(out.contains("| a b |"), "{out}");
    }

    #[test]
    fn table_cell_hard_break_becomes_br() {
        let table = one_cell_table(RenderNode::span(
            vec![],
            vec![
                RenderNode::text("a"),
                RenderNode::hard_break(),
                RenderNode::text("b"),
            ],
        ));
        let out = render(&table).output;
        assert!(out.contains("a<br>b"), "{out}");
    }

    #[test]
    fn table_cell_inline_code_escapes_pipe() {
        let table = one_cell_table(RenderNode::inline_code("a|b"));
        let out = render(&table).output;
        assert!(out.contains(r"`a\|b`"), "{out}");
    }

    #[test]
    fn text_outside_table_keeps_literal_pipe_and_newline() {
        let para = RenderNode::paragraph(vec![RenderNode::text("a | b\nc")]);
        assert_eq!(render(&para).output, "a | b\nc");
    }

    #[test]
    fn table_cell_escapes_pipe_in_link_url_and_title() {
        let link = RenderNode::link("a|b", Some("t|t".into()), vec![RenderNode::text("label")]);
        let out = render(&one_cell_table(link)).output;
        assert!(out.contains(r#"[label](a\|b "t\|t")"#), "{out}");
    }

    #[test]
    fn table_cell_escapes_newline_in_link_url() {
        let link = RenderNode::link("a\nb", None, vec![RenderNode::text("x")]);
        let out = render(&one_cell_table(link)).output;
        assert!(out.contains("(a<br>b)"), "{out}");
    }

    #[test]
    fn table_cell_escapes_pipe_in_image_alt_and_url() {
        let image = RenderNode::image("img|x.png", None, "a|b");
        let out = render(&one_cell_table(image)).output;
        assert!(out.contains(r"![a\|b](img\|x.png)"), "{out}");
    }

    #[test]
    fn link_outside_table_keeps_literal_pipe() {
        let link = RenderNode::link("a|b", Some("t|t".into()), vec![RenderNode::text("x")]);
        assert_eq!(render(&link).output, r#"[x](a|b "t|t")"#);
    }

    // ── RT-TWOCOLUMN-002: MarkdownPlus columns ─────────────────────────────

    fn columns_node(hints: crate::tree::ColumnsHints, children: Vec<RenderNode>) -> RenderNode {
        let mut bq = RenderNode::block_quote(children);
        bq.attrs.set_columns_hints(&hints);
        bq
    }

    #[test]
    fn columns_plain_markdown_stays_sequential() {
        let node = columns_node(
            crate::tree::ColumnsHints {
                left_count: 1,
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("left")]),
                RenderNode::paragraph(vec![RenderNode::text("right")]),
            ],
        );
        let rendered = render_with(
            &node,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Warn),
        );
        assert_eq!(rendered.output, "left\n\nright");
    }

    #[test]
    fn columns_markdown_plus_emits_flex_container() {
        let node = columns_node(
            crate::tree::ColumnsHints {
                left_count: 1,
                gap: 4,
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("left")]),
                RenderNode::paragraph(vec![RenderNode::text("right")]),
            ],
        );
        let rendered = render_with(
            &node,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        );
        let out = rendered.output;
        assert!(
            out.contains(r#"<div class="columns" style="display:flex;gap:4ch">"#),
            "{out}"
        );
        assert!(out.contains(r#"<div class="column""#), "{out}");
        assert!(out.contains("left"), "{out}");
        assert!(out.contains("right"), "{out}");
    }

    #[test]
    fn columns_markdown_plus_fixed_left_width() {
        let node = columns_node(
            crate::tree::ColumnsHints {
                left_count: 1,
                left_width: crate::tree::ColumnWidthKind::Fixed(30),
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("L")]),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        let out = render_with(
            &node,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        assert!(out.contains("flex:0 0 30ch;max-width:30ch"), "{out}");
        assert!(out.contains("flex:1 1 0"), "{out}");
    }

    #[test]
    fn columns_markdown_plus_percent_left_width() {
        let node = columns_node(
            crate::tree::ColumnsHints {
                left_count: 1,
                left_width: crate::tree::ColumnWidthKind::Percent(0.4),
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("L")]),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        let out = render_with(
            &node,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        assert!(out.contains("flex:0 0 40%"), "{out}");
    }

    #[test]
    fn columns_markdown_plus_empty_columns() {
        let node = columns_node(
            crate::tree::ColumnsHints {
                left_count: 0,
                ..Default::default()
            },
            vec![],
        );
        let out = render_with(
            &node,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        assert!(out.contains(r#"<div class="columns""#), "{out}");
        assert!(out.contains(r#"<div class="column""#), "{out}");
    }

    #[test]
    fn columns_markdown_plus_emphasis_and_prose_content() {
        let node = columns_node(
            crate::tree::ColumnsHints {
                left_count: 1,
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![
                    RenderNode::text("plain "),
                    RenderNode::strong(vec![RenderNode::text("bold")]),
                ]),
                RenderNode::paragraph(vec![RenderNode::emphasis(vec![RenderNode::text("italic")])]),
            ],
        );
        let out = render_with(
            &node,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        assert!(out.contains("**bold**"), "{out}");
        assert!(out.contains("_italic_"), "{out}");
    }

    #[test]
    fn columns_markdown_plus_nested_block_content() {
        let node = columns_node(
            crate::tree::ColumnsHints {
                left_count: 1,
                ..Default::default()
            },
            vec![
                RenderNode::list(
                    false,
                    None,
                    vec![RenderNode::list_item(
                        None,
                        vec![RenderNode::paragraph(vec![RenderNode::text("li")])],
                    )],
                ),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        let out = render_with(
            &node,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        // The container is well-formed: one opening and one closing div trio.
        assert_eq!(out.matches("<div").count(), 3, "{out}");
        assert_eq!(out.matches("</div>").count(), 3, "{out}");
        assert!(out.contains("- li"), "{out}");
    }

    #[test]
    fn columns_markdown_plus_image_column_renders_without_malformed_html() {
        // An image is valid Markdown image syntax embedded in the HTML
        // container; the column markup stays well-formed under Warn.
        let warn = columns_node(
            crate::tree::ColumnsHints {
                left_count: 1,
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::image("pic.png", None, "alt")]),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        let out = render_with(
            &warn,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        assert!(out.contains("![alt](pic.png)"), "{out}");
        assert_eq!(out.matches("<div").count(), 3, "{out}");
        assert_eq!(out.matches("</div>").count(), 3, "{out}");

        // Under Strict the same content renders without error — an image is a
        // portable Markdown construct, so no lossy rejection is triggered.
        let strict = columns_node(
            crate::tree::ColumnsHints {
                left_count: 1,
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::image("pic.png", None, "alt")]),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        let result = render_markdown_node(
            &strict,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Strict),
        );
        assert!(result.is_ok(), "image column must not fail under Strict");
    }

    #[test]
    fn columns_markdown_plus_multiple_blocks_per_column_documents_parser_constraint() {
        // A column with two blocks renders them joined by a blank line, the
        // same as any block sequence. That blank line lives inside the raw
        // HTML container. This test documents the accepted parser constraint:
        // a strict CommonMark parser may treat the blank line as ending the
        // raw HTML block, so the markup is well-formed at the renderer level
        // but a downstream strict parser may not nest the second block.
        let node = columns_node(
            crate::tree::ColumnsHints {
                left_count: 2,
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("left-one")]),
                RenderNode::paragraph(vec![RenderNode::text("left-two")]),
                RenderNode::paragraph(vec![RenderNode::text("right")]),
            ],
        );
        let out = render_with(
            &node,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        // The renderer output itself is well-formed: one div trio, both left
        // blocks present, joined by the standard blank-line block separator.
        assert_eq!(out.matches("<div").count(), 3, "{out}");
        assert_eq!(out.matches("</div>").count(), 3, "{out}");
        assert!(out.contains("left-one\n\nleft-two"), "{out}");
        assert!(out.contains("right"), "{out}");
    }

    #[test]
    fn block_quote_without_columns_unchanged() {
        let bq = RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text("q")])]);
        let rendered = render_with(
            &bq,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        );
        assert_eq!(rendered.output, "> q");
    }

    // ── RT-FILESYSTEM-001: list marker policy degradation ──────────────────

    #[test]
    fn list_marker_policy_degrades_with_diagnostic_under_warn() {
        let mut list = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(
                None,
                vec![RenderNode::paragraph(vec![RenderNode::text("a")])],
            )],
        );
        list.attrs
            .set_list_marker_policy(crate::tree::ListMarkerPolicy::TreeConnectors);
        let rendered = render_with(
            &list,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Warn),
        );
        // Degrades to a native list, with a lossy diagnostic.
        assert_eq!(rendered.output, "- a");
        assert_eq!(rendered.diagnostics.len(), 1);
    }

    #[test]
    fn list_marker_policy_rejected_under_strict() {
        let mut list = RenderNode::list(false, None, vec![RenderNode::list_item(None, vec![])]);
        list.attrs
            .set_list_marker_policy(crate::tree::ListMarkerPolicy::None);
        let result = render_markdown_node(
            &list,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Strict),
        );
        assert!(matches!(result, Err(RenderError::LossyRejected { .. })));
    }

    #[test]
    fn default_list_marker_policy_renders_unchanged() {
        let list = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(
                None,
                vec![RenderNode::paragraph(vec![RenderNode::text("a")])],
            )],
        );
        let rendered = render(&list);
        assert_eq!(rendered.output, "- a");
        assert!(rendered.diagnostics.is_empty());
    }

    // ── RT-TODO-001: Markdown task-list degradation ────────────────────────

    #[test]
    fn task_hint_item_keeps_gfm_checkbox_in_markdown() {
        // Completed → [x]; every other state → [ ]. Markdown reads the
        // ListItem `checked` field, not the task hint.
        let completed = {
            let mut item = RenderNode::list_item(
                Some(true),
                vec![RenderNode::paragraph(vec![RenderNode::text("done")])],
            );
            item.attrs.set_task_hints(&crate::tree::TaskHints {
                state: crate::tree::TaskState::Completed,
            });
            RenderNode::list(false, None, vec![item])
        };
        assert_eq!(render(&completed).output, "- [x] done");

        let blocked = {
            let mut item = RenderNode::list_item(
                Some(false),
                vec![RenderNode::paragraph(vec![RenderNode::text("stuck")])],
            );
            item.attrs.set_task_hints(&crate::tree::TaskHints {
                state: crate::tree::TaskState::Blocked,
            });
            RenderNode::list(false, None, vec![item])
        };
        assert_eq!(render(&blocked).output, "- [ ] stuck");
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

    // ── RT-PROSE-002: MarkdownPlus inline-style lowering ───────────────────

    fn styled_span(style: crate::style::Style, children: Vec<RenderNode>) -> RenderNode {
        let mut span = RenderNode::span(vec![], children);
        span.attrs.set_style(&style);
        span
    }

    fn color_style(color: crate::color::Color) -> crate::style::Style {
        use crate::layout::TargetValue;
        use crate::style::{PerMode, Style};
        Style {
            color: Some(TargetValue::universal(PerMode::universal(color))),
            ..Default::default()
        }
    }

    #[test]
    fn markdown_plus_lowers_foreground_color_to_inline_html() {
        use crate::color::{BasicColor, Color};
        let span = styled_span(
            color_style(Color::BasicColor(BasicColor::Red)),
            vec![RenderNode::text("x")],
        );
        let out = render_with(
            &span,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        assert_eq!(out, "<span style=\"color: rgb(128, 0, 0)\">x</span>");
    }

    #[test]
    fn markdown_plus_lowers_foreground_alpha_to_rgba() {
        use crate::color::{BasicColor, Color, RgbColor};
        use crate::layout::TargetValue;
        use crate::style::{Opacity, PaintColor, PerMode, Style};
        let paint = PaintColor::new(Color::Rgb(RgbColor::new(255, 0, 0, BasicColor::Red)))
            .with_opacity(Opacity::from_percent(50).unwrap());
        let span = styled_span(
            Style {
                color: Some(TargetValue::universal(PerMode::universal(paint))),
                ..Default::default()
            },
            vec![RenderNode::text("x")],
        );
        let out = render_with(
            &span,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        assert!(out.contains("color: rgba(255, 0, 0,"), "{out}");
    }

    #[test]
    fn markdown_plus_lowers_background_color_to_inline_html() {
        use crate::color::{BasicColor, Color};
        use crate::layout::TargetValue;
        use crate::style::{PerMode, Style};
        let span = styled_span(
            Style {
                background: Some(TargetValue::universal(PerMode::universal(Color::BasicColor(
                    BasicColor::Red,
                )))),
                ..Default::default()
            },
            vec![RenderNode::text("y")],
        );
        let out = render_with(
            &span,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        assert_eq!(out, "<span style=\"background-color: rgb(128, 0, 0)\">y</span>");
    }

    #[test]
    fn markdown_plus_lowers_underline_variants_to_inline_html() {
        use crate::style::{Style, TextEmphasis, UnderlineStyle};
        let straight = styled_span(
            Style {
                emphasis: TextEmphasis {
                    underline: Some(UnderlineStyle::Straight),
                    ..Default::default()
                },
                ..Default::default()
            },
            vec![RenderNode::text("x")],
        );
        assert_eq!(
            render_with(
                &straight,
                &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn)
            )
            .output,
            "<span style=\"text-decoration: underline\">x</span>"
        );

        let curly = styled_span(
            Style {
                emphasis: TextEmphasis {
                    underline: Some(UnderlineStyle::Curly),
                    ..Default::default()
                },
                ..Default::default()
            },
            vec![RenderNode::text("x")],
        );
        assert_eq!(
            render_with(
                &curly,
                &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn)
            )
            .output,
            "<span style=\"text-decoration: underline; text-decoration-style: wavy\">x</span>"
        );
    }

    #[test]
    fn markdown_plus_html_escapes_styled_span_body() {
        use crate::color::{BasicColor, Color};
        let span = styled_span(
            color_style(Color::BasicColor(BasicColor::Red)),
            vec![RenderNode::text("<script>alert(1)</script>")],
        );
        let out = render_with(
            &span,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        assert_eq!(
            out,
            "<span style=\"color: rgb(128, 0, 0)\">\
             &lt;script&gt;alert(1)&lt;/script&gt;</span>"
        );
    }

    #[test]
    fn markdown_plus_html_escapes_ampersand_in_styled_span_body() {
        use crate::color::{BasicColor, Color};
        let span = styled_span(
            color_style(Color::BasicColor(BasicColor::Red)),
            vec![RenderNode::text("a & b")],
        );
        let out = render_with(
            &span,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        assert_eq!(out, "<span style=\"color: rgb(128, 0, 0)\">a &amp; b</span>");
    }

    #[test]
    fn markdown_plus_preserves_markdown_sigils_inside_styled_span() {
        use crate::color::{BasicColor, Color};
        // A nested `Strong` node's `**` sigils are structural output, not body
        // text, so HTML-body escaping leaves them intact.
        let span = styled_span(
            color_style(Color::BasicColor(BasicColor::Red)),
            vec![RenderNode::strong(vec![RenderNode::text("<x>")])],
        );
        let out = render_with(
            &span,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        assert_eq!(
            out,
            "<span style=\"color: rgb(128, 0, 0)\">**&lt;x&gt;**</span>"
        );
    }

    #[test]
    fn markdown_plus_coalesces_class_and_style_into_one_span() {
        use crate::color::{BasicColor, Color};
        let mut span = RenderNode::span(vec!["hl".into()], vec![RenderNode::text("x")]);
        span.attrs
            .set_style(&color_style(Color::BasicColor(BasicColor::Red)));
        let out = render_with(
            &span,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        assert_eq!(
            out,
            "<span class=\"hl\" style=\"color: rgb(128, 0, 0)\">x</span>"
        );
    }

    #[test]
    fn styled_span_degrades_in_plain_markdown_with_diagnostic() {
        use crate::color::{BasicColor, Color};
        let span = styled_span(
            color_style(Color::BasicColor(BasicColor::Red)),
            vec![RenderNode::text("important")],
        );
        let rendered = render_with(
            &span,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Warn),
        );
        assert_eq!(rendered.output, "important");
        assert_eq!(rendered.diagnostics.len(), 1);
    }

    #[test]
    fn styled_span_emits_inner_text_in_lossy_plain_markdown() {
        use crate::color::{BasicColor, Color};
        let span = styled_span(
            color_style(Color::BasicColor(BasicColor::Red)),
            vec![RenderNode::text("important")],
        );
        let rendered = render_with(
            &span,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Lossy),
        );
        assert_eq!(rendered.output, "important");
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn styled_span_rejected_in_strict_plain_markdown() {
        use crate::color::{BasicColor, Color};
        let span = styled_span(
            color_style(Color::BasicColor(BasicColor::Red)),
            vec![RenderNode::text("important")],
        );
        let result = render_markdown_node(
            &span,
            &opts(MarkdownDialect::Markdown, RenderStrictness::Strict),
        );
        assert!(matches!(result, Err(RenderError::LossyRejected { .. })));
    }

    // ── RT-PROSE-003: link destination escaping ────────────────────────────

    #[test]
    fn link_destination_escapes_parentheses() {
        let link = RenderNode::link(
            "https://example.com/a(b)c",
            None,
            vec![RenderNode::text("go")],
        );
        assert_eq!(render(&link).output, r"[go](https://example.com/a\(b\)c)");
    }

    #[test]
    fn link_destination_escapes_backslash() {
        let link = RenderNode::link(r"https://example.com/a\b", None, vec![RenderNode::text("go")]);
        assert_eq!(render(&link).output, r"[go](https://example.com/a\\b)");
    }

    #[test]
    fn link_destination_with_whitespace_uses_angle_brackets() {
        let link = RenderNode::link(
            "https://example.com/a b",
            None,
            vec![RenderNode::text("go")],
        );
        assert_eq!(render(&link).output, "[go](<https://example.com/a b>)");
    }

    #[test]
    fn link_destination_line_ending_degrades_to_space() {
        let link = RenderNode::link(
            "https://example.com/a\nb",
            None,
            vec![RenderNode::text("go")],
        );
        assert_eq!(render(&link).output, "[go](<https://example.com/a b>)");
    }
}
