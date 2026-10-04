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

use crate::browser::fragment::RawRange;
use crate::browser::utils::escape_attribute;
use crate::tree::SourceSpan;
use crate::tree::diagnostic::{Diagnostic, Severity};
use crate::tree::document::{Document, FrontmatterFormat};
use crate::tree::error::{RenderError, RenderStrictness, Rendered};
use crate::tree::node::{ColumnAlign, NodeKind, RenderNode, is_html_comment_only};
use crate::tree::validate::{
    ValidationError, ValidationMode, is_block, is_inline_kind, kind_name, validate,
};

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
///   construct that cannot be rendered without loss (for example raw HTML or
///   a classed span under [`MarkdownDialect::Markdown`], a block other than
///   one paragraph in a table cell, or a footnote identifier no label can
///   spell).
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
        mid_line: 0,
        heading_depth: 0,
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
    /// Inside a table cell every field protects its pipes (see
    /// [`escape_cell_pipes`] and [`encode_generated_attribute`]) and its
    /// line endings, so neither can corrupt the GFM pipe-delimited row.
    table_cell_depth: u32,
    /// Non-zero while rendering the body of a MarkdownPlus inline-HTML span
    /// (a classed or styled [`NodeKind::Span`]). Inside that body, text nodes
    /// HTML-escape `<`, `>`, and `&` instead of backslash-escaping them; the
    /// body is still parsed as Markdown, so other punctuation is escaped as
    /// usual.
    html_depth: u32,
    /// Non-zero while rendering an inline sequence that a reader never sees
    /// at the start of a line (a heading's text, a link label, a delimiter
    /// or HTML span body), so its first character needs no line-start
    /// protection and its last is not the end of a line.
    mid_line: u32,
    /// Non-zero while rendering a heading's text, which must stay on one
    /// line.
    heading_depth: u32,
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
                Ok(format!("{hashes} {}", self.render_heading_text(children)?))
            }
            NodeKind::Section {
                depth,
                heading,
                children,
            } => {
                let hashes = "#".repeat(usize::from(depth.get()));
                let heading_line = format!("{hashes} {}", self.render_heading_text(heading)?);
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
                // emits the same semantic progress HTML as the browser, with
                // the label taken from the children's plain text as the
                // browser takes it. `progress_html` encodes that text for
                // each place it writes it.
                match node.attrs.progress_hints_ref() {
                    Some(hints) if self.opts.dialect == MarkdownDialect::MarkdownPlus => {
                        let html = progress_html(hints, &super::browser::plain_text(children));
                        // All of it is generated and escaped, and the visible
                        // label body has no line endings, so every line
                        // ending and cell pipe left sits in an attribute value
                        // (`aria-label`, the glyph `data-*` attributes).
                        Ok(encode_generated_attribute(
                            &html,
                            self.table_cell_depth > 0,
                        ))
                    }
                    _ => self.render_inline(children),
                }
            }
            NodeKind::BlockQuote { children } => {
                if let Some(hints) = node.attrs.columns_hints_ref() {
                    match self.opts.dialect {
                        MarkdownDialect::Markdown => self.render_columns(children, hints),
                        MarkdownDialect::MarkdownPlus => {
                            self.lower_to_html(std::slice::from_ref(node))
                        }
                    }
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
                let mut fence = code_block_fence(value);
                if let Some(lang) = lang {
                    fence.push_str(lang);
                }
                if let Some(meta) = meta {
                    fence.push(' ');
                    fence.push_str(meta);
                }
                let body = value.trim_end_matches('\n');
                let close = code_block_fence(value);
                Ok(format!("{fence}\n{body}\n{close}"))
            }
            NodeKind::ThematicBreak => Ok("---".to_string()),
            NodeKind::Table { align, children } => {
                let table = self.render_table(align, children)?;
                // A table title/caption is emitted as a literal paragraph on
                // its own line before the table, separated by a blank line.
                // An empty or whitespace-only title is ignored.
                match node.attrs.table_title_ref() {
                    Some(title) if !title.trim().is_empty() => Ok(format!(
                        "{}\n\n{table}",
                        protect_lines(&escape_markdown_text(title.trim(), true), true)
                    )),
                    _ => Ok(table),
                }
            }
            NodeKind::TableRow { children } => self.render_table_row(children),
            NodeKind::TableCell { children } => Ok(self.render_table_cell(children)?.text),
            NodeKind::FootnoteDefinition {
                identifier,
                children,
            } => {
                let label = self.footnote_label(node, identifier)?;
                let body = self.render_blocks(children)?;
                // A GFM reader keeps a line in the definition only when it is
                // blank or indented by four spaces, whatever the label width.
                Ok(format!(
                    "[^{label}]: {}",
                    indent_continuation(&body, FOOTNOTE_CONTINUATION)
                ))
            }
            // A text node outside an inline sequence (a block of its own) is
            // written as a one-piece sequence, so it gets the same line
            // protection.
            NodeKind::Text { .. } => self.render_inline(std::slice::from_ref(node)),
            // A delimiter wrapper's spelling depends on its neighbors, which
            // only the enclosing inline sequence sees.
            NodeKind::Emphasis { .. }
            | NodeKind::Strong { .. }
            | NodeKind::Delete { .. }
            | NodeKind::Extended { .. } => self.render_inline(std::slice::from_ref(node)),
            NodeKind::Span { children } => self.render_span(node, children),
            NodeKind::InlineCode { value } => Ok(if self.table_cell_depth > 0 {
                // A literal pipe inside inline code still breaks a GFM table
                // cell, so it is escaped even though the run is code. The
                // escape goes in before fencing so the fence sees the final
                // content.
                crate::markdown::code_span(&escape_cell_pipes(value))
            } else {
                crate::markdown::code_span(value)
            }),
            NodeKind::Link {
                url,
                title,
                children,
            } => {
                self.mid_line += 1;
                let text = self.render_inline(children);
                self.mid_line -= 1;
                Ok(format!("[{}]({})", text?, self.link_target(url, title)))
            }
            NodeKind::Image { url, title, alt } => {
                // A reader parses alt text as inline Markdown, so it is
                // escaped like a `Text` node, including inside a table cell
                // or heading.
                let alt = escape_markdown_text(alt, false);
                let alt = if self.table_cell_depth > 0 {
                    escape_table_cell_text(&alt)
                } else if self.heading_depth > 0 {
                    encode_heading_line_feeds(&alt)
                } else {
                    protect_lines(&alt, false)
                };
                Ok(format!("![{alt}]({})", self.link_target(url, title)))
            }
            NodeKind::FootnoteReference { identifier } => {
                let label = self.footnote_label(node, identifier)?;
                Ok(if self.table_cell_depth > 0 {
                    // The table reader removes the escape before it matches
                    // the label, so `[^a\|b]` still refers to `[^a|b]: …`.
                    format!("[^{}]", escape_cell_pipes(&label))
                } else {
                    format!("[^{label}]")
                })
            }
            // A break's spelling depends on its position in the enclosing
            // inline sequence: see `Self::write_break`.
            NodeKind::SoftBreak | NodeKind::HardBreak => {
                self.render_inline(std::slice::from_ref(node))
            }
            NodeKind::Html { value, block } => self.render_html(node, value, *block),
            NodeKind::Unsupported { label } => self.render_unsupported(node, label),
            NodeKind::Disclosure { summary, children, .. } => self.render_disclosure(summary, children),
        }
    }

    /// Renders a disclosure block.
    ///
    /// Plain Markdown emits the `::disclosure / ::details / ::end-disclosure`
    /// DSL verbatim so the output remains a clean Darkmatter document.
    /// MarkdownPlus wraps the summary and body with `<details>`/`<summary>`.
    /// The summary line opens a raw HTML block that a reader does not parse
    /// as Markdown, so the summary is written as HTML (see
    /// [`Self::lower_to_html`]); the body after the blank line is Markdown.
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
                let summary_text = self.lower_to_html(summary)?;
                let body_text = self.render_blocks(children)?;
                Ok(format!(
                    "<details><summary>{summary_text}</summary>\n\n{body_text}\n</details>"
                ))
            }
        }
    }

    /// Collects the inline pieces `node` writes, flattening wrappers that
    /// write no markup of their own so a delimiter wrapper inside one still
    /// sees its real neighbors.
    ///
    /// Built-in [`NodeKind::Extended`] tokens roundtrip to their darkmatter
    /// source syntax: `mark` is `==children==` and `dim` is `⌄children⌄`
    /// (U+2304). A token without a Markdown spelling writes its `children`
    /// as plain inline Markdown.
    fn push_pieces(
        &mut self,
        node: &RenderNode,
        pieces: &mut Vec<InlinePiece>,
    ) -> Result<(), RenderError> {
        let pending = match &node.kind {
            NodeKind::Emphasis { children } => {
                self.pending_delimited(DelimitedKind::Emphasis, node, children)?
            }
            NodeKind::Strong { children } => {
                self.pending_delimited(DelimitedKind::Strong, node, children)?
            }
            NodeKind::Delete { children } => {
                self.pending_delimited(DelimitedKind::Delete, node, children)?
            }
            NodeKind::Extended {
                token, children, ..
            } => match &token[..] {
                "mark" => self.pending_delimited(DelimitedKind::Mark, node, children)?,
                "dim" => self.pending_delimited(DelimitedKind::Dim, node, children)?,
                _ => {
                    for child in children {
                        self.push_pieces(child, pieces)?;
                    }
                    return Ok(());
                }
            },
            NodeKind::Span { children } if !self.span_writes_html(node) => {
                self.degrade_plain_span(node)?;
                for child in children {
                    self.push_pieces(child, pieces)?;
                }
                return Ok(());
            }
            NodeKind::Text { value } => {
                pieces.push(InlinePiece::Literal(self.render_text(value)));
                return Ok(());
            }
            NodeKind::SoftBreak => {
                pieces.push(InlinePiece::Break(Break::Soft));
                return Ok(());
            }
            NodeKind::HardBreak => {
                pieces.push(InlinePiece::Break(Break::Hard));
                return Ok(());
            }
            NodeKind::InlineCode { .. } => {
                pieces.push(InlinePiece::Code(self.render(node)?));
                return Ok(());
            }
            kind if (self.table_cell_depth > 0 || self.heading_depth > 0) && is_block(kind) => {
                pieces.push(InlinePiece::Block(self.render_one_line_block(node)?));
                return Ok(());
            }
            _ => {
                pieces.push(InlinePiece::Text(self.render(node)?));
                return Ok(());
            }
        };
        let (leading, pending, trailing) = pending;
        for edge in &leading {
            self.push_pieces(edge, pieces)?;
        }
        pieces.push(InlinePiece::Delimited(pending));
        for edge in &trailing {
            self.push_pieces(edge, pieces)?;
        }
        Ok(())
    }

    /// Renders a delimiter wrapper's body; its delimiters are chosen later
    /// by [`Self::join_pieces`].
    ///
    /// A delimiter run only opens when no whitespace follows it and only
    /// closes when no whitespace precedes it, and a break between the run and
    /// its text has the same effect. Edge whitespace and edge soft/hard
    /// breaks, including those at the edge of a nested wrapper, are therefore
    /// returned as the leading and trailing nodes, which the enclosing
    /// sequence writes outside the delimiters (`a<b> b</b>` → `a **b**`), so
    /// a moved break is spelled for its position there. A wrapper with
    /// nothing else to show writes only its edges.
    fn pending_delimited(
        &mut self,
        kind: DelimitedKind,
        node: &RenderNode,
        children: &[RenderNode],
    ) -> Result<(Vec<RenderNode>, PendingDelimited, Vec<RenderNode>), RenderError> {
        let mut core = None;
        let mut leading = Vec::new();
        let mut trailing = Vec::new();
        if self.has_edge(children, Edge::Leading) || self.has_edge(children, Edge::Trailing) {
            let mut nodes = children.to_vec();
            leading = self.take_edge(&mut nodes, Edge::Leading);
            trailing = self.take_edge(&mut nodes, Edge::Trailing);
            core = Some(nodes);
        }
        let core = core.as_deref().unwrap_or(children);
        self.mid_line += 1;
        let body = self.render_inline_joined(core);
        self.mid_line -= 1;
        let pending = PendingDelimited {
            kind,
            body: body?,
            span: node.span.clone(),
        };
        Ok((leading, pending, trailing))
    }

    /// Concatenates inline pieces, choosing each delimiter wrapper's
    /// spelling from the characters on both sides of its delimiters.
    ///
    /// The first spelling in [`DelimitedKind::spellings`] that a reader can
    /// open and close there is used, so content that already reads back
    /// correctly keeps its usual delimiter. A sequence edge counts as
    /// whitespace: every context that holds an inline sequence (line start,
    /// a delimiter, `[`, `>`, `|`) flanks the same way.
    ///
    /// Line-start and line-end protection is decided on the logical line the
    /// pieces assemble (see [`LineWriter`]), not per piece, so a marker split
    /// across adjacent text values or flattened wrappers is still protected.
    /// A sequence that is a whole block (`mid_line` is zero) also encodes
    /// its trailing whitespace, which a reader would strip.
    ///
    /// Two code spans that would touch, directly or across pieces that write
    /// nothing (an empty value, a flattened wrapper, a wrapper written
    /// unstyled), get [`CODE_SPAN_SEPARATOR`] between them. The returned
    /// [`FenceEdges`] carry the same check across a nested sequence's edges.
    fn join_pieces(&mut self, pieces: &[InlinePiece]) -> Result<Joined, RenderError> {
        let block = self.mid_line == 0;
        let mut output = LineWriter::new(block, self.writes_line_starts());
        // A block written on a table cell's line is set off from whatever
        // the cell writes before and after it by `<br>`.
        let mut after_block = false;
        let mut seq = FenceSequence::default();
        for (index, piece) in pieces.iter().enumerate() {
            if after_block && piece.writes() {
                // A following block writes its own separator.
                if !matches!(piece, InlinePiece::Block(_)) {
                    seq.write(&mut output, FenceEdges::default(), "<br>", false);
                }
                after_block = false;
            }
            // Whether this piece ends its block: nothing after it writes
            // anything, and the sequence is a whole block rather than one a
            // closing `]`, delimiter, or tag follows.
            let ends_block = block && !pieces[index + 1..].iter().any(InlinePiece::writes);
            let pending = match piece {
                InlinePiece::Text(text) => {
                    seq.write(&mut output, FenceEdges::default(), text, false);
                    continue;
                }
                InlinePiece::Code(text) => {
                    let edges = FenceEdges {
                        starts: true,
                        ends: true,
                    };
                    seq.write(&mut output, edges, text, false);
                    continue;
                }
                InlinePiece::Break(kind) => {
                    let before = output.text.len();
                    self.write_break(&mut output, *kind, ends_block);
                    if output.text.len() > before {
                        seq.wrote_other();
                    }
                    continue;
                }
                InlinePiece::Literal(text) => {
                    seq.write(&mut output, FenceEdges::default(), text, true);
                    continue;
                }
                InlinePiece::Block(joined) => {
                    if !output.text.is_empty() {
                        seq.write(&mut output, FenceEdges::default(), "<br>", false);
                    }
                    seq.write(&mut output, joined.fences, &joined.text, false);
                    after_block = true;
                    continue;
                }
                InlinePiece::Delimited(pending) => pending,
            };
            let body = &pending.body;
            if body.text.is_empty() {
                continue;
            }
            let sides = Sides {
                before: Neighbor::before(&output.text),
                first: Neighbor::after(&body.text),
                last: Neighbor::before(&body.text),
                after: Neighbor::following(&pieces[index + 1..]),
            };
            let spelling = pending
                .kind
                .spellings()
                .iter()
                .copied()
                .find(|spelling| spelling.fits(&sides))
                .unwrap_or(Spelling::Unstyled);
            match spelling {
                Spelling::Run(delimiter, _) => {
                    let text = format!("{delimiter}{}{delimiter}", body.text);
                    seq.write(&mut output, FenceEdges::default(), &text, false);
                }
                Spelling::Html(tag) => {
                    let text = format!("<{tag}>{}</{tag}>", body.text);
                    seq.write(&mut output, FenceEdges::default(), &text, false);
                }
                Spelling::Unstyled => {
                    self.record_unstyled(pending)?;
                    // The body was escaped mid-line; written bare, it is
                    // literal content of this line again, and its code
                    // fences are this sequence's neighbors.
                    seq.write(&mut output, body.fences, &body.text, true);
                }
            }
        }
        if block {
            output.finish();
        }
        Ok(Joined {
            text: output.text,
            fences: seq.edges(),
        })
    }

    /// Writes a soft or hard break in the spelling its context reads back.
    ///
    /// | Context | Soft break | Hard break |
    /// |---|---|---|
    /// | table cell | space | `<br>` |
    /// | heading text (one ATX line) | space | `<br>` |
    /// | Markdown line, content on both sides | line ending | `\` + line ending |
    /// | Markdown line, nothing after it in the block | space | `<br>` |
    /// | Markdown line, the line so far is blank | space | `\` + line ending |
    ///
    /// A reader strips a line ending at a block's end, and reads a backslash
    /// there literally; a line ending on a blank line ends the paragraph;
    /// and a line holding only an HTML open tag (a MarkdownPlus span opener)
    /// starts a raw HTML block. The space written instead of a soft break is
    /// literal content, so [`LineWriter`] encodes it as `&#32;` at a line
    /// edge.
    fn write_break(&self, output: &mut LineWriter, kind: Break, ends_block: bool) {
        let one_line = self.table_cell_depth > 0 || self.heading_depth > 0;
        match kind {
            Break::Soft if one_line || output.line_is_blank() || ends_block => {
                output.push_literal(" ");
            }
            Break::Soft => output.end_line("\n"),
            Break::Hard if one_line || ends_block => output.push_markup("<br>"),
            Break::Hard => output.end_line("\\\n"),
        }
    }

    /// Records that a wrapper with no spelling a reader can open and close
    /// at its position was written as plain text, per the strictness model.
    fn record_unstyled(&mut self, pending: &PendingDelimited) -> Result<(), RenderError> {
        let message = format!(
            "{} has no Markdown spelling between these neighbors; written unstyled",
            pending.kind.label()
        );
        match self.opts.strictness {
            RenderStrictness::Strict => Err(RenderError::LossyRejected { message }),
            RenderStrictness::Warn => {
                self.diagnostics
                    .push(Diagnostic::lossy(message, Some(pending.span.clone())));
                Ok(())
            }
            RenderStrictness::Lossy => Ok(()),
        }
    }

    /// Whether `node` writes no markup at its own edges, so whitespace or a
    /// break at its edge also sits at the edge of an enclosing delimiter
    /// wrapper.
    fn passes_edges_through(&self, node: &RenderNode) -> bool {
        match &node.kind {
            NodeKind::Emphasis { .. }
            | NodeKind::Strong { .. }
            | NodeKind::Delete { .. }
            | NodeKind::Extended { .. } => true,
            NodeKind::Span { .. } => !self.span_writes_html(node),
            _ => false,
        }
    }

    /// Whether `node` writes nothing: empty text or code (which has no code
    /// span spelling), or a pass-through wrapper whose children all write
    /// nothing.
    fn is_blank(&self, node: &RenderNode) -> bool {
        match &node.kind {
            NodeKind::Text { value } | NodeKind::InlineCode { value } => value.is_empty(),
            _ => {
                self.passes_edges_through(node)
                    && node.children().iter().all(|child| self.is_blank(child))
            }
        }
    }

    /// The index of the first (or last) child that writes something.
    fn edge_index(&self, nodes: &[RenderNode], edge: Edge) -> Option<usize> {
        match edge {
            Edge::Leading => nodes.iter().position(|node| !self.is_blank(node)),
            Edge::Trailing => nodes.iter().rposition(|node| !self.is_blank(node)),
        }
    }

    /// Whether `nodes` begin (or end) with whitespace or a break.
    fn has_edge(&self, nodes: &[RenderNode], edge: Edge) -> bool {
        let Some(node) = self.edge_index(nodes, edge).map(|index| &nodes[index]) else {
            return false;
        };
        match &node.kind {
            NodeKind::SoftBreak | NodeKind::HardBreak => true,
            NodeKind::Text { value } => match edge {
                Edge::Leading => value.starts_with(char::is_whitespace),
                Edge::Trailing => value.ends_with(char::is_whitespace),
            },
            _ => self.passes_edges_through(node) && self.has_edge(node.children(), edge),
        }
    }

    /// Removes the whitespace and breaks at one edge of `nodes`, descending
    /// into pass-through wrappers, and returns them in document order.
    fn take_edge(&self, nodes: &mut Vec<RenderNode>, edge: Edge) -> Vec<RenderNode> {
        let mut moved: Vec<RenderNode> = Vec::new();
        let place = |moved: &mut Vec<RenderNode>, items: Vec<RenderNode>| match edge {
            Edge::Leading => moved.extend(items),
            Edge::Trailing => {
                moved.splice(0..0, items);
            }
        };
        while let Some(index) = self.edge_index(nodes, edge) {
            let passes_through = self.passes_edges_through(&nodes[index]);
            match &nodes[index].kind {
                NodeKind::SoftBreak | NodeKind::HardBreak => {
                    let node = nodes.remove(index);
                    place(&mut moved, vec![node]);
                }
                NodeKind::Text { value } => {
                    let kept = match edge {
                        Edge::Leading => value.trim_start(),
                        Edge::Trailing => value.trim_end(),
                    };
                    if kept.len() == value.len() {
                        break;
                    }
                    let (kept, outside) = match edge {
                        Edge::Leading => (
                            kept.to_string(),
                            value[..value.len() - kept.len()].to_string(),
                        ),
                        Edge::Trailing => (kept.to_string(), value[kept.len()..].to_string()),
                    };
                    let mut outside_node = nodes[index].clone();
                    outside_node.kind = NodeKind::Text { value: outside };
                    place(&mut moved, vec![outside_node]);
                    if kept.is_empty() {
                        nodes.remove(index);
                    } else {
                        nodes[index].kind = NodeKind::Text { value: kept };
                        break;
                    }
                }
                _ if passes_through => {
                    let Some(children) = nodes[index].children_mut() else {
                        break;
                    };
                    let inner = self.take_edge(children, edge);
                    if inner.is_empty() {
                        break;
                    }
                    place(&mut moved, inner);
                    // A wrapper that still writes something ends the edge; an
                    // emptied one is now blank and the next sibling is tried.
                    if !self.is_blank(&nodes[index]) {
                        break;
                    }
                }
                _ => break,
            }
        }
        moved
    }

    /// Renders a sequence of block-level nodes, joined by blank lines.
    ///
    /// A break child joins the phrasing children beside it into one inline
    /// sequence (see [`break_run`]); written as a block of its own, a reader
    /// would lose it or read its backslash literally. Other phrasing
    /// children stay separate blocks.
    fn render_blocks(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
        let mut parts = Vec::with_capacity(children.len());
        let mut rest = children;
        while !rest.is_empty() {
            let run = break_run(rest);
            parts.push(if run > 1 {
                self.render_inline(&rest[..run])?
            } else {
                self.render(&rest[0])?
            });
            rest = &rest[run..];
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
        self.render_inline(children)
    }

    /// Renders a sequence of inline nodes, concatenated without separators.
    fn render_inline(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
        Ok(self.render_inline_joined(children)?.text)
    }

    /// [`Self::render_inline`], keeping the code-fence edges for a sequence
    /// that writes the result as one of its pieces.
    fn render_inline_joined(&mut self, children: &[RenderNode]) -> Result<Joined, RenderError> {
        let mut pieces = Vec::with_capacity(children.len());
        for child in children {
            self.push_pieces(child, &mut pieces)?;
        }
        self.join_pieces(&pieces)
    }

    /// Renders a heading's text. A reader does not parse it for block
    /// starts, but a closing run of `#` after whitespace would be read as the
    /// optional closing sequence and dropped.
    /// A reader also strips the text's edge whitespace, so it is written as
    /// character references.
    fn render_heading_text(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
        // Only an inline extension can carry a block into a heading.
        if let Some(block) = first_block(children) {
            self.record_one_line_block(block, "heading")?;
        }
        self.mid_line += 1;
        self.heading_depth += 1;
        let text = self.render_inline(children);
        self.heading_depth -= 1;
        self.mid_line -= 1;
        let mut text = text?;
        encode_line_end(&mut text);
        encode_line_start(&mut text);
        let run = text.len() - text.trim_end_matches('#').len();
        let before = &text[..text.len() - run];
        if run > 0 && (before.is_empty() || before.ends_with([' ', '\t'])) {
            Ok(format!("{before}\\{}", &text[before.len()..]))
        } else {
            Ok(text)
        }
    }

    /// Renders a two-column block quote for portable
    /// [`MarkdownDialect::Markdown`], which has no side-by-side layout: the
    /// left column's blocks are emitted first, then a blank line, then the
    /// right column's blocks. MarkdownPlus writes the browser's flex
    /// container instead (see [`Self::lower_to_html`]).
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

    /// Renders a table cell's content on its one line.
    fn render_table_cell(&mut self, children: &[RenderNode]) -> Result<Joined, RenderError> {
        // A cell holding only one paragraph is that paragraph's text; any
        // other block has no GFM cell spelling and is written on the cell's
        // line (see `Self::render_one_line_block`).
        let inline_scope = match children {
            [only] if matches!(only.kind, NodeKind::Paragraph { .. }) => only.children(),
            _ => children,
        };
        if let Some(block) = first_block(inline_scope) {
            self.record_one_line_block(block, "table cell")?;
        }
        // Descendants of a table cell render in cell-escaping mode so
        // literal pipes and newlines cannot break GFM table structure.
        self.table_cell_depth += 1;
        let result = self.render_inline_joined(children);
        self.table_cell_depth -= 1;
        // A table reader trims each cell, so edge whitespace is written as
        // character references. A code fence is not whitespace, so the
        // fence edges are unchanged.
        let mut cell = result?;
        encode_line_end(&mut cell.text);
        encode_line_start(&mut cell.text);
        Ok(cell)
    }

    /// Writes a block that sits inside a table cell, or inside a heading
    /// (through an inline extension), on that one line.
    ///
    /// Both hold only inline content, so a block keeps its text and loses its
    /// block structure: the inline content of each paragraph, heading,
    /// quote, disclosure summary and body, footnote body, and nested table
    /// cell is written as it would be there, and the enclosing inline
    /// sequence sets each block off with `<br>`. A code
    /// block becomes a code span (line endings as spaces, info string
    /// dropped), the form [`crate::markdown::code_span`] gives a fenced block
    /// written inline; a list item is prefixed with its marker as text; a
    /// thematic break writes nothing between its two `<br>`. The cell's or
    /// heading's own protection (pipes, line endings) applies to all of it.
    /// [`Self::record_one_line_block`] reports the loss.
    fn render_one_line_block(&mut self, node: &RenderNode) -> Result<Joined, RenderError> {
        match &node.kind {
            NodeKind::Heading { children, .. }
            | NodeKind::BlockQuote { children }
            | NodeKind::FootnoteDefinition { children, .. } => self.render_inline_joined(children),
            NodeKind::Section {
                heading, children, ..
            } => {
                let heading = self.render_inline_joined(heading)?;
                let body = self.render_inline_joined(children)?;
                Ok(join_one_line_parts([heading, body]))
            }
            NodeKind::Disclosure {
                summary, children, ..
            } => {
                let summary = self.render_inline_joined(summary)?;
                let body = self.render_inline_joined(children)?;
                Ok(join_one_line_parts([summary, body]))
            }
            NodeKind::List {
                ordered,
                start,
                children,
            } => {
                let mut items = Vec::with_capacity(children.len());
                for (offset, item) in children.iter().enumerate() {
                    let mut marker = if *ordered {
                        format!("{}. ", start.unwrap_or(1) + offset as u64)
                    } else {
                        "- ".to_string()
                    };
                    if let NodeKind::ListItem {
                        checked: Some(checked),
                        ..
                    } = item.kind
                    {
                        marker.push_str(if checked { "[x] " } else { "[ ] " });
                    }
                    let body = self.render_inline_joined(item.children())?;
                    items.push(Joined {
                        text: format!("{}{}", self.render_text(&marker), body.text),
                        fences: FenceEdges {
                            starts: false,
                            ends: body.fences.ends,
                        },
                    });
                }
                Ok(join_one_line_parts(items))
            }
            NodeKind::Code { value, .. } => {
                // The final line ending closes the block's last line; it is
                // not content, as in the fenced form.
                let value = value.trim_end_matches(['\r', '\n']);
                let text = crate::markdown::code_span(&if self.table_cell_depth > 0 {
                    escape_cell_pipes(value)
                } else {
                    value.to_string()
                });
                Ok(Joined {
                    text,
                    fences: FenceEdges {
                        starts: true,
                        ends: true,
                    },
                })
            }
            NodeKind::ThematicBreak => Ok(Joined::default()),
            NodeKind::Table { children, .. } => {
                let mut cells = Vec::new();
                for row in children {
                    for cell in row.children() {
                        cells.push(self.render_table_cell(cell.children())?);
                    }
                }
                Ok(join_one_line_parts(cells))
            }
            // A paragraph is already one inline line.
            NodeKind::Paragraph { children } if node.attrs.progress_hints_ref().is_none() => {
                self.render_inline_joined(children)
            }
            _ => Ok(Joined::markup(self.render(node)?)),
        }
    }

    /// Applies the strictness model to a block that a table cell or heading
    /// (`container`) writes on its line (see [`Self::render_one_line_block`]),
    /// once per container.
    fn record_one_line_block(
        &mut self,
        block: &RenderNode,
        container: &str,
    ) -> Result<(), RenderError> {
        let message = format!(
            "{} block in a {container} has no Markdown spelling, since a {container} \
             holds one line of inline content; written on that line",
            kind_name(&block.kind)
        );
        match self.opts.strictness {
            RenderStrictness::Strict => Err(RenderError::LossyRejected { message }),
            RenderStrictness::Warn => {
                self.diagnostics
                    .push(Diagnostic::lossy(message, Some(block.span.clone())));
                Ok(())
            }
            RenderStrictness::Lossy => Ok(()),
        }
    }

    /// The label a footnote reference or definition writes for
    /// `identifier`: the identifier itself when a reader reads it back
    /// exactly, otherwise the [`footnote_label_spelling`] degradation,
    /// applied per the strictness model. The reference and its definition
    /// degrade the same way, so they still pair.
    fn footnote_label(
        &mut self,
        node: &RenderNode,
        identifier: &str,
    ) -> Result<String, RenderError> {
        let label = footnote_label_spelling(identifier);
        if label != identifier {
            let message = format!(
                "footnote identifier {identifier:?} has no exact Markdown label spelling; \
                 written as {label:?}"
            );
            match self.opts.strictness {
                RenderStrictness::Strict => return Err(RenderError::LossyRejected { message }),
                RenderStrictness::Warn => {
                    self.diagnostics
                        .push(Diagnostic::lossy(message, Some(node.span.clone())));
                }
                RenderStrictness::Lossy => {}
            }
        }
        Ok(label)
    }

    /// Writes `nodes` as the HTML the browser renderer writes for them, for a
    /// MarkdownPlus raw HTML block (a disclosure summary, the columns
    /// container) whose content a reader passes through without parsing it
    /// as Markdown. Markdown syntax there would show literally, and code
    /// written as a code span would be read as markup, so the content takes
    /// the browser's HTML: `<code>` with escaped text, `<strong>`, `<a>`,
    /// `<mark>`, and so on.
    ///
    /// A blank line ends a raw HTML block, so none is left inside the
    /// content (see [`keep_html_block_open`]). Raw [`NodeKind::Html`]
    /// payloads are written byte for byte. A payload that would itself leave
    /// a blank line cannot be embedded faithfully: under
    /// [`RenderStrictness::Strict`] that is a [`RenderError::LossyRejected`];
    /// otherwise its line ending is written as a character reference, and
    /// [`RenderStrictness::Warn`] records a lossy diagnostic.
    ///
    /// Both callers are blocks, which a table cell writes on one line
    /// instead (see [`Self::render_one_line_block`]), so this never runs in a
    /// cell.
    fn lower_to_html(&mut self, nodes: &[RenderNode]) -> Result<String, RenderError> {
        let lowered = super::browser::lower_to_html(nodes, self.opts.strictness)?;
        self.diagnostics.extend(lowered.diagnostics);
        let (html, unfaithful) = keep_html_block_open(&lowered.html, &lowered.raw);
        for span in unfaithful {
            let message = "raw HTML holds a blank line, which would end the MarkdownPlus HTML \
                           block around it; its line ending was written as a character \
                           reference, which script, style, comments, and unquoted attributes \
                           do not decode"
                .to_string();
            match self.opts.strictness {
                RenderStrictness::Strict => return Err(RenderError::LossyRejected { message }),
                RenderStrictness::Warn => {
                    self.diagnostics
                        .push(Diagnostic::lossy(message, Some(span)));
                }
                RenderStrictness::Lossy => {}
            }
        }
        Ok(html)
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
    /// with `<`, `>`, and `&` HTML-escaped so literal markup stays inert. The
    /// class value is attribute-escaped, its line endings are written as
    /// character references, and in a table cell its pipes are written
    /// `&#124;` (see [`encode_generated_attribute`]).
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
        if !self.span_writes_html(node) {
            return self.render_inline(std::slice::from_ref(node));
        }
        // The children become the body of an inline HTML element, so
        // descendant text HTML-escapes its markup characters.
        self.html_depth += 1;
        self.mid_line += 1;
        let inner = self.render_inline(children);
        self.mid_line -= 1;
        self.html_depth -= 1;
        let inner = inner?;

        let mut attrs = String::new();
        if !node.attrs.classes.is_empty() {
            let joined = node.attrs.classes.join(" ");
            let classes =
                encode_generated_attribute(&escape_attribute(&joined), self.table_cell_depth > 0);
            attrs.push_str(&format!(" class=\"{classes}\""));
        }
        if let Some(css) = span_style_css(node) {
            attrs.push_str(&format!(" style=\"{css}\""));
        }
        Ok(format!("<span{attrs}>{inner}</span>"))
    }

    /// Whether a span lowers to an HTML element: only in MarkdownPlus, and
    /// only with a class or a style that has a CSS form. Otherwise it writes
    /// just its children.
    fn span_writes_html(&self, node: &RenderNode) -> bool {
        self.opts.dialect == MarkdownDialect::MarkdownPlus
            && (!node.attrs.classes.is_empty() || span_style_css(node).is_some())
    }

    /// Applies the strictness model to a classed or styled span that plain
    /// Markdown writes as its inner text.
    fn degrade_plain_span(&mut self, node: &RenderNode) -> Result<(), RenderError> {
        let classes = &node.attrs.classes;
        let has_style = span_style_css(node).is_some();
        if self.opts.dialect == MarkdownDialect::MarkdownPlus || (classes.is_empty() && !has_style)
        {
            return Ok(());
        }
        let message = span_lossy_message(classes, has_style);
        match self.opts.strictness {
            RenderStrictness::Strict => Err(RenderError::LossyRejected { message }),
            RenderStrictness::Warn => {
                self.diagnostics
                    .push(Diagnostic::lossy(message, Some(node.span.clone())));
                Ok(())
            }
            RenderStrictness::Lossy => Ok(()),
        }
    }

    /// Renders a text value so a reader sees exactly that value: see
    /// [`escape_markdown_text`]. Inside a MarkdownPlus inline-HTML span `<`,
    /// `>`, and `&` are HTML-escaped; inside a table cell pipes and newlines
    /// are made row-safe; inside a heading a line ending is a character
    /// reference. Line-start and line-end protection depend on neighboring
    /// values, so [`Self::join_pieces`] applies it to the assembled line.
    fn render_text(&self, value: &str) -> String {
        let text = escape_markdown_text(value, self.html_depth > 0);
        // The cell escape runs last: its `\|` must stay a pipe escape.
        if self.table_cell_depth > 0 {
            escape_table_cell_text(&text)
        } else if self.heading_depth > 0 {
            encode_heading_line_feeds(&text)
        } else {
            text
        }
    }

    /// Whether text written now can begin a Markdown line: not in a table
    /// cell (newlines become `<br>` there).
    fn writes_line_starts(&self) -> bool {
        self.table_cell_depth == 0
    }

    /// Renders raw HTML, degrading it under plain Markdown.
    ///
    /// Plain Markdown reports raw HTML as not portable, except a comment-only
    /// payload ([`is_html_comment_only`]), which every reader shows as nothing.
    ///
    /// Inside a heading or a table cell, which are one Markdown line, a line
    /// ending in the payload would end the heading or the table row. It
    /// cannot be kept without changing the payload's bytes: under
    /// [`RenderStrictness::Strict`] that is a [`RenderError::LossyRejected`];
    /// otherwise each ending is written as `&#13;` / `&#10;`, and
    /// [`RenderStrictness::Warn`] records a lossy diagnostic. A payload
    /// without a line ending is written byte for byte, except that in a
    /// table cell each `|` gets the cell escape (see [`Self::cell_safe_raw`]).
    fn render_html(
        &mut self,
        node: &RenderNode,
        value: &str,
        _block: bool,
    ) -> Result<String, RenderError> {
        // A comment is read as nothing by every CommonMark reader, so it is as
        // portable as the separator this writer puts between code spans.
        if self.opts.dialect == MarkdownDialect::Markdown && !is_html_comment_only(value) {
            let message = "raw HTML is not portable plain Markdown".to_string();
            match self.opts.strictness {
                RenderStrictness::Strict => return Err(RenderError::LossyRejected { message }),
                // Under Warn/Lossy the raw value is emitted (CommonMark
                // permits raw HTML); Warn additionally records it.
                RenderStrictness::Warn => {
                    self.diagnostics
                        .push(Diagnostic::lossy(message, Some(node.span.clone())));
                }
                RenderStrictness::Lossy => {}
            }
        }
        let one_line = self.table_cell_depth > 0 || self.heading_depth > 0;
        if !one_line || !value.contains(['\r', '\n']) {
            return Ok(self.cell_safe_raw(value.to_string()));
        }
        let message = "raw HTML holds a line ending, which would end the heading or table row \
                       it is in; it was written as a character reference, which script, style, \
                       comments, and unquoted attributes do not decode"
            .to_string();
        match self.opts.strictness {
            RenderStrictness::Strict => return Err(RenderError::LossyRejected { message }),
            RenderStrictness::Warn => {
                self.diagnostics
                    .push(Diagnostic::lossy(message, Some(node.span.clone())));
            }
            RenderStrictness::Lossy => {}
        }
        // A character reference, not the `<br>` a table cell writes for a
        // text line ending: in element text and quoted attribute values an
        // HTML reader decodes it to the payload's own character, where `<br>`
        // would add a line break the payload does not have. Each CR and LF is
        // encoded alone, so a CRLF split across two payloads reads the same
        // as one.
        Ok(self.cell_safe_raw(value.replace('\r', "&#13;").replace('\n', "&#10;")))
    }

    /// Makes raw HTML (an authored payload, or a placeholder carrying an
    /// authored label) safe in a table cell with [`escape_cell_pipes`], the
    /// rule code spans use: a reader that splits the row removes the escape
    /// again, so the payload reaches the HTML reader unchanged even inside a
    /// comment, `<script>`, or attribute, where `&#124;` would not be decoded.
    /// Outside a table cell a pipe has no meaning and the payload is kept.
    fn cell_safe_raw(&self, html: String) -> String {
        if self.table_cell_depth > 0 {
            escape_cell_pipes(&html)
        } else {
            html
        }
    }

    /// Formats a link/image target, appending a quoted title when present.
    ///
    /// The destination is escaped per CommonMark by
    /// [`escape_markdown_destination`] so parentheses, backslashes,
    /// whitespace, and entity spellings cannot truncate or change it, and the
    /// title by [`escape_link_title`], which also keeps it on one line in
    /// every context. Inside a table cell the pipes of both are then escaped
    /// by [`escape_cell_pipes`], since a literal `|` would split the GFM row.
    fn link_target(&self, url: &str, title: &Option<String>) -> String {
        if self.table_cell_depth > 0 {
            let dest = escape_cell_link_destination(url);
            return match title {
                Some(title) => format!(
                    "{dest} \"{}\"",
                    escape_cell_pipes(&escape_link_title(title))
                ),
                None => dest,
            };
        }
        let dest = escape_markdown_destination(url);
        match title {
            Some(title) => format!("{dest} \"{}\"", escape_link_title(title)),
            None => dest,
        }
    }

    /// Renders an unsupported node according to strictness. The `Warn`
    /// placeholder comment carries the label made comment-safe by
    /// [`comment_safe_label`].
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
                Ok(self.cell_safe_raw(format!(
                    "<!-- unsupported: {} -->",
                    comment_safe_label(label)
                )))
            }
            RenderStrictness::Lossy => Ok(String::new()),
        }
    }
}

/// Which edge of a delimiter wrapper's content is being examined.
#[derive(Debug, Clone, Copy)]
enum Edge {
    Leading,
    Trailing,
}

/// The CSS a MarkdownPlus span carries for its style, if any.
fn span_style_css(node: &RenderNode) -> Option<String> {
    node.attrs
        .style_ref()
        .filter(|style| !style.is_empty())
        .and_then(markdown_plus_style_css)
}

/// One part of an inline sequence: finished Markdown, or a delimiter wrapper
/// whose delimiters wait on the characters around it.
enum InlinePiece {
    Text(String),
    /// A soft or hard break, spelled by [`Writer::write_break`] once its
    /// position on the line and in the block is known.
    Break(Break),
    /// An escaped text value, which may still need line-start or line-end
    /// protection on the line it joins.
    Literal(String),
    /// A fenced code span; an empty value writes nothing.
    Code(String),
    Delimited(PendingDelimited),
    /// A block inside a table cell or heading, already written on one line by
    /// [`Writer::render_one_line_block`].
    Block(Joined),
}

impl InlinePiece {
    /// Whether the piece writes anything; a delimiter wrapper with an empty
    /// body writes nothing, since its edges are separate pieces.
    fn writes(&self) -> bool {
        match self {
            Self::Text(text) | Self::Literal(text) | Self::Code(text) => !text.is_empty(),
            Self::Break(_) | Self::Block(_) => true,
            Self::Delimited(pending) => !pending.body.text.is_empty(),
        }
    }
}

/// Written between two code spans that would otherwise touch.
///
/// A code span's fence is a backtick run that no backtick touches, so two
/// fences written side by side merge into one longer run and a reader finds
/// a different code span (`` `a``b` `` is the one value ```` a``b ````).
/// Every visible separator changes the text, and CommonMark has no empty
/// inline construct except raw HTML, so an empty comment is the only
/// lossless boundary. Plain Markdown already writes inline HTML for
/// structure (`<br>`, `<em>`), and the comment is mid-line, after a closing
/// fence, so it can never start an HTML block.
///
/// Readers in this repository treat it as structure, not text: the shared
/// terminal and browser renderers show a comment-only raw HTML node as
/// nothing under every strictness and raw-HTML policy except browser
/// `Allow` (which writes the comment itself), and Prose's grammar reads a
/// comment as no content.
const CODE_SPAN_SEPARATOR: &str = "<!-- -->";

/// Whether written Markdown begins or ends with a code span's fence, so a
/// neighboring code span needs [`CODE_SPAN_SEPARATOR`].
#[derive(Debug, Clone, Copy, Default)]
struct FenceEdges {
    starts: bool,
    ends: bool,
}

/// An inline sequence as written, with the code-fence edges a sequence that
/// embeds it as one piece needs.
#[derive(Debug, Default)]
struct Joined {
    text: String,
    fences: FenceEdges,
}

impl Joined {
    /// Markup that begins and ends with something other than a code fence.
    fn markup(text: String) -> Self {
        Self {
            text,
            fences: FenceEdges::default(),
        }
    }
}

/// A structural line break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Break {
    Soft,
    Hard,
}

/// A rendered delimiter wrapper body before its spelling is chosen.
struct PendingDelimited {
    kind: DelimitedKind,
    body: Joined,
    span: crate::tree::SourceSpan,
}

/// The code-fence state of an inline sequence being joined.
#[derive(Debug, Default)]
struct FenceSequence {
    /// Whether the first piece written begins with a code fence.
    starts: Option<bool>,
    /// Whether the output ends with a code span's closing fence.
    open: bool,
}

impl FenceSequence {
    /// Writes a piece whose own fence edges are `edges`, separating it from
    /// a code span it would touch.
    fn write(&mut self, output: &mut LineWriter, edges: FenceEdges, text: &str, literal: bool) {
        if text.is_empty() {
            return;
        }
        if self.open && edges.starts {
            output.push_markup(CODE_SPAN_SEPARATOR);
        }
        self.starts.get_or_insert(edges.starts);
        if literal {
            output.push_literal(text);
        } else {
            output.push_markup(text);
        }
        self.open = edges.ends;
    }

    /// Records a piece written without a code fence at either edge.
    fn wrote_other(&mut self) {
        self.starts.get_or_insert(false);
        self.open = false;
    }

    fn edges(&self) -> FenceEdges {
        FenceEdges {
            starts: self.starts.unwrap_or(false),
            ends: self.open,
        }
    }
}

/// The node kinds written with a delimiter run.
#[derive(Debug, Clone, Copy)]
enum DelimitedKind {
    Emphasis,
    Strong,
    Delete,
    Mark,
    Dim,
}

impl DelimitedKind {
    /// The spellings to try, in order of preference.
    ///
    /// CommonMark and GFM readers pass inline HTML through, so `Emphasis`,
    /// `Strong`, and `Delete` fall back to `<em>`, `<strong>`, and `<del>`
    /// where no delimiter can open and close (`a**(b)**` is not bold;
    /// `a<strong>(b)</strong>` is). The `mark` and `dim` extensions are read
    /// only by darkmatter, which has no HTML form for them, so they fall back
    /// to plain text.
    fn spellings(self) -> &'static [Spelling] {
        match self {
            Self::Emphasis => &[
                Spelling::Run("_", RunRule::Underscore),
                Spelling::Run("*", RunRule::Star),
                Spelling::Html("em"),
            ],
            Self::Strong => &[
                Spelling::Run("**", RunRule::Star),
                Spelling::Run("__", RunRule::Underscore),
                Spelling::Html("strong"),
            ],
            Self::Delete => &[Spelling::Run("~~", RunRule::Star), Spelling::Html("del")],
            Self::Mark => &[Spelling::Run("==", RunRule::Mark), Spelling::Unstyled],
            Self::Dim => &[Spelling::Run("\u{2304}", RunRule::Dim), Spelling::Unstyled],
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Emphasis => "emphasis",
            Self::Strong => "strong",
            Self::Delete => "strikethrough",
            Self::Mark => "mark",
            Self::Dim => "dim",
        }
    }
}

/// One way to write a delimiter wrapper.
#[derive(Debug, Clone, Copy)]
enum Spelling {
    /// The same delimiter run on both sides of the body.
    Run(&'static str, RunRule),
    /// An inline HTML element around the body.
    Html(&'static str),
    /// The body alone.
    Unstyled,
}

/// The reader rule that decides whether a delimiter run opens or closes.
#[derive(Debug, Clone, Copy)]
enum RunRule {
    /// CommonMark `*`, also GFM `~~`: opens when left-flanking, closes when
    /// right-flanking.
    Star,
    /// CommonMark `_`: as `Star`, but not inside a word.
    Underscore,
    /// Darkmatter `==`: pairs left to right with no flanking rule.
    Mark,
    /// Darkmatter `⌄`: no whitespace on the inner side, and never between
    /// two alphanumerics.
    Dim,
}

impl Spelling {
    /// Whether a reader parses this spelling back to the wrapper at `sides`.
    fn fits(self, sides: &Sides) -> bool {
        let Self::Run(delimiter, rule) = self else {
            return true;
        };
        let delimiter = delimiter.chars().next();
        // A neighbor of the same character joins the run and changes its
        // length. `==` pairs from the left, so only a body that ends in `=`
        // can steal the closer.
        let merges = |side: &Neighbor| side.delimiter.is_some() && side.delimiter == delimiter;
        if merges(&sides.before) || merges(&sides.last) || merges(&sides.after) {
            return false;
        }
        if !matches!(rule, RunRule::Mark) && merges(&sides.first) {
            return false;
        }
        // Darkmatter reads `\==` as one escaped pair, so an escaped `=` before
        // the opener takes the opener's first `=` with it.
        if matches!(rule, RunRule::Mark) && sides.before.escaped == Some('=') {
            return false;
        }
        match rule {
            RunRule::Mark => true,
            RunRule::Dim => {
                let word = |side: &Neighbor| side.flank == Flank::Word;
                sides.first.flank != Flank::Space
                    && sides.last.flank != Flank::Space
                    && !(word(&sides.before) && word(&sides.first))
                    && !(word(&sides.last) && word(&sides.after))
            }
            RunRule::Star | RunRule::Underscore => sides.resolutions().all(|[b, f, l, a]| {
                let opens = left_flanking(b, f)
                    && (matches!(rule, RunRule::Star)
                        || !right_flanking(b, f)
                        || b == Flank::Punct);
                let closes = right_flanking(l, a)
                    && (matches!(rule, RunRule::Star) || !left_flanking(l, a) || a == Flank::Punct);
                opens && closes
            }),
        }
    }
}

/// CommonMark's left-flanking test for a run between `before` and `after`.
fn left_flanking(before: Flank, after: Flank) -> bool {
    after != Flank::Space
        && (after != Flank::Punct || matches!(before, Flank::Space | Flank::Punct))
}

/// CommonMark's right-flanking test for a run between `before` and `after`.
fn right_flanking(before: Flank, after: Flank) -> bool {
    before != Flank::Space
        && (before != Flank::Punct || matches!(after, Flank::Space | Flank::Punct))
}

/// How a character beside a delimiter run counts for flanking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flank {
    /// Whitespace, a line edge, or the edge of an inline sequence.
    Space,
    Punct,
    /// Alphanumeric (or another character that is neither space nor
    /// punctuation).
    Word,
    /// A non-ASCII symbol or mark: readers disagree on whether it is
    /// punctuation (CommonMark 0.31 counts Unicode symbols, earlier versions
    /// do not), so a spelling must work either way.
    Unsure,
}

impl Flank {
    fn of(c: char) -> Self {
        if c.is_whitespace() {
            Self::Space
        } else if c.is_ascii_punctuation() {
            Self::Punct
        } else if c.is_alphanumeric() || c.is_ascii() {
            Self::Word
        } else {
            Self::Unsure
        }
    }

    /// The concrete classes this one may be read as.
    fn resolutions(self) -> &'static [Flank] {
        match self {
            Self::Unsure => &[Flank::Punct, Flank::Word],
            Self::Space => &[Flank::Space],
            Self::Punct => &[Flank::Punct],
            Self::Word => &[Flank::Word],
        }
    }
}

/// The character on one side of a delimiter run.
#[derive(Debug, Clone, Copy)]
struct Neighbor {
    flank: Flank,
    /// The character, when a reader could take it as part of a delimiter run
    /// (it is not backslash-escaped).
    delimiter: Option<char>,
    /// The character, when it is backslash-escaped.
    escaped: Option<char>,
}

impl Neighbor {
    const EDGE: Self = Self {
        flank: Flank::Space,
        delimiter: None,
        escaped: None,
    };

    /// The last character of `markdown`.
    fn before(markdown: &str) -> Self {
        let Some(last) = markdown.chars().next_back() else {
            return Self::EDGE;
        };
        let rest = &markdown[..markdown.len() - last.len_utf8()];
        let backslashes = rest.chars().rev().take_while(|c| *c == '\\').count();
        if backslashes % 2 == 1 {
            return Self {
                flank: Flank::Punct,
                delimiter: None,
                escaped: Some(last),
            };
        }
        Self {
            flank: Flank::of(last),
            delimiter: Some(last),
            escaped: None,
        }
    }

    /// The first character of `markdown`.
    fn after(markdown: &str) -> Self {
        markdown.chars().next().map_or(Self::EDGE, |first| Self {
            flank: Flank::of(first),
            delimiter: Some(first),
            escaped: None,
        })
    }

    /// The first character the pieces after a wrapper write. A later wrapper
    /// starts with its own delimiter (punctuation), and it is that wrapper
    /// that avoids repeating this one's character.
    ///
    /// A break counts as whitespace: each of its spellings (a line ending, a
    /// space, `\`, `<br>`) closes a run exactly as whitespace does.
    fn following(pieces: &[InlinePiece]) -> Self {
        for piece in pieces {
            match piece {
                InlinePiece::Text(text) | InlinePiece::Literal(text) | InlinePiece::Code(text)
                    if !text.is_empty() =>
                {
                    return Self::after(text);
                }
                InlinePiece::Text(_) | InlinePiece::Literal(_) | InlinePiece::Code(_) => {}
                // A block is set off by `<br>`, which flanks like a break.
                InlinePiece::Break(_) | InlinePiece::Block(_) => return Self::EDGE,
                InlinePiece::Delimited(pending) => {
                    if !pending.body.text.is_empty() {
                        return Self {
                            flank: Flank::Punct,
                            delimiter: None,
                            escaped: None,
                        };
                    }
                }
            }
        }
        Self::EDGE
    }
}

/// The four characters a delimiter wrapper's runs touch.
struct Sides {
    before: Neighbor,
    first: Neighbor,
    last: Neighbor,
    after: Neighbor,
}

impl Sides {
    /// Every concrete reading of the four flanks.
    fn resolutions(&self) -> impl Iterator<Item = [Flank; 4]> + '_ {
        let [b, f, l, a] = [
            self.before.flank,
            self.first.flank,
            self.last.flank,
            self.after.flank,
        ];
        b.resolutions().iter().flat_map(move |&b| {
            f.resolutions().iter().flat_map(move |&f| {
                l.resolutions()
                    .iter()
                    .flat_map(move |&l| a.resolutions().iter().map(move |&a| [b, f, l, a]))
            })
        })
    }
}

/// The number of leading `nodes` written as one block: a phrasing node
/// together with every break that follows it and the phrasing node after
/// each such break, or a lone node otherwise.
fn break_run(nodes: &[RenderNode]) -> usize {
    let is_break = |node: &RenderNode| matches!(node.kind, NodeKind::SoftBreak | NodeKind::HardBreak);
    if !is_inline_kind(&nodes[0].kind) {
        return 1;
    }
    let mut run = 1;
    while let Some(next) = nodes.get(run) {
        let joins = is_break(next) || (is_break(&nodes[run - 1]) && is_inline_kind(&next.kind));
        if !joins {
            break;
        }
        run += 1;
    }
    run
}

/// The first block among `nodes` or inside their inline descendants, in
/// document order.
fn first_block(nodes: &[RenderNode]) -> Option<&RenderNode> {
    nodes.iter().find_map(|node| {
        if is_block(&node.kind) {
            Some(node)
        } else {
            first_block(node.children())
        }
    })
}

/// Joins the non-empty one-line parts of a block in a table cell or heading
/// with `<br>`.
fn join_one_line_parts(parts: impl IntoIterator<Item = Joined>) -> Joined {
    let parts: Vec<Joined> = parts.into_iter().filter(|part| !part.text.is_empty()).collect();
    Joined {
        text: parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>()
            .join("<br>"),
        fences: FenceEdges {
            starts: parts.first().is_some_and(|part| part.fences.starts),
            ends: parts.last().is_some_and(|part| part.fences.ends),
        },
    }
}

/// The footnote label written for `identifier`, equal to it when a reader
/// reads `[^identifier]` back as exactly `identifier`.
///
/// A reader matches a label without processing its escapes, so a character
/// that cannot appear literally has no faithful spelling. The degraded label
/// is chosen so a reader still reads it as one label, and the reference and
/// definition still pair:
///
/// - a run of spaces, tabs, and line endings becomes one space, and edge
///   whitespace is removed, which is the reader's own normalization
///   (`a\n b` → `a b`);
/// - an identifier with nothing else left is `_`;
/// - a `[` or `]` not already escaped by an odd backslash run gets a
///   backslash (`a]b` → `a\]b`), and an odd trailing backslash run gets
///   one more (`a\` → `a\\`), so it cannot escape the closing `]`.
fn footnote_label_spelling(identifier: &str) -> String {
    let words: Vec<&str> = identifier
        .split([' ', '\t', '\r', '\n'])
        .filter(|word| !word.is_empty())
        .collect();
    if words.is_empty() {
        return "_".to_string();
    }
    let mut label = String::with_capacity(identifier.len());
    let mut backslashes = 0;
    for c in words.join(" ").chars() {
        if matches!(c, '[' | ']') && backslashes % 2 == 0 {
            label.push('\\');
        }
        backslashes = if c == '\\' { backslashes + 1 } else { 0 };
        label.push(c);
    }
    if backslashes % 2 == 1 {
        label.push('\\');
    }
    label
}

/// Makes an unsupported-content label safe inside the `Warn` placeholder
/// comment. A `>` becomes `&gt;`, so no `-->` or `--!>` in the label can
/// close the comment early, and a line ending becomes a space, so the
/// placeholder stays on its line (a table row, a heading, or a paragraph a
/// blank line would end). A comment does not decode the reference; the
/// placeholder is already lossy, so it reads `&gt;` instead of `>`.
fn comment_safe_label(label: &str) -> String {
    label
        .replace('>', "&gt;")
        .replace("\r\n", " ")
        .replace(['\r', '\n'], " ")
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

/// Splits `text` into lines as CommonMark does: each line with the ending
/// that closes it (`\n`, `\r`, or `\r\n`; empty for an unterminated last
/// line). Like [`str::lines`], a final line ending does not start another
/// line; unlike it, a lone carriage return ends a line and every ending is
/// kept, so raw content survives a re-join byte for byte.
fn split_line_endings(text: &str) -> impl Iterator<Item = (&str, &str)> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let (line, ending, tail) = match rest.find(['\r', '\n']) {
            None => (rest, "", ""),
            Some(at) => {
                let width = if rest[at..].starts_with("\r\n") { 2 } else { 1 };
                (&rest[..at], &rest[at..at + width], &rest[at + width..])
            }
        };
        rest = tail;
        Some((line, ending))
    })
}

/// Prefixes every line of `text` with `prefix` (trimmed of trailing spaces
/// on an empty line), keeping each line's ending bytes; see
/// [`split_line_endings`] for what ends a line.
fn prefix_lines(text: &str, prefix: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (line, ending) in split_line_endings(text) {
        if line.is_empty() {
            out.push_str(prefix.trim_end());
        } else {
            out.push_str(prefix);
            out.push_str(line);
        }
        out.push_str(ending);
    }
    out
}

/// Keeps `html`, written between markup on the line that opens a raw HTML
/// block and markup that closes it, one raw HTML block: no line in it may be
/// blank (only spaces or tabs). Returns the HTML and the span of each raw
/// payload that could not be kept byte for byte.
///
/// Line endings are read from the concatenated HTML, as a reader does: a
/// carriage return and the line feed after it are one ending even when they
/// come from different raw payloads, or one is raw and the other generated.
/// Which bytes came from where (the `raw` ranges) only decides which may be
/// rewritten.
///
/// Only HTML the browser writer generated is re-encoded, since its text and
/// attribute values are escaped and an HTML reader decodes a character
/// reference there. A generated carriage return is always `&#13;`, so it
/// never starts a line ending, not even before a raw line feed: the
/// reference keeps the generated character, which a reader would otherwise
/// fold into the line ending, and the raw line feed stays one line ending.
/// A line ending is encoded only when every byte of it is generated: a line
/// feed beside a blank line becomes `&#10;`, which makes that line
/// non-blank. The ending before the blank line is encoded when it is wholly
/// generated, otherwise the one after it.
///
/// A raw payload's bytes (a `raw` range) are left alone, including its
/// carriage returns: HTML reads one as a line feed in every context. Where
/// neither ending around a blank line is wholly generated, nothing can hold
/// the block open without changing raw content; the ending before the line
/// is encoded and every payload holding a byte of it is reported. Its first
/// and last lines are never blank, since markup flanks them.
fn keep_html_block_open(html: &str, raw: &[RawRange]) -> (String, Vec<SourceSpan>) {
    /// A line and the ending that follows it, empty for the last line. Each
    /// ending character carries the index in `raw` of the payload holding
    /// it, or `None` when it was generated.
    struct Line {
        text: String,
        ending: Vec<(char, Option<usize>)>,
    }
    // The ranges are in output order and never overlap.
    let raw_at = |at: usize| {
        let index = raw.partition_point(|r| r.range.end <= at);
        raw.get(index)
            .filter(|r| r.range.contains(&at))
            .map(|_| index)
    };
    let mut lines = vec![Line {
        text: String::new(),
        ending: Vec::new(),
    }];
    let mut chars = html.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if c != '\r' && c != '\n' {
            lines.last_mut().expect("one line").text.push(c);
            continue;
        }
        let owner = raw_at(at);
        if c == '\r' && owner.is_none() {
            lines.last_mut().expect("one line").text.push_str("&#13;");
            continue;
        }
        let mut ending = vec![(c, owner)];
        if c == '\r'
            && let Some(&(next, '\n')) = chars.peek()
        {
            chars.next();
            ending.push(('\n', raw_at(next)));
        }
        lines.last_mut().expect("one line").ending = ending;
        lines.push(Line {
            text: String::new(),
            ending: Vec::new(),
        });
    }

    // `encoded[i]` is set when the ending after line `i` is written as a
    // reference, joining line `i + 1` to it.
    let mut encoded = vec![false; lines.len()];
    let mut unfaithful = Vec::new();
    let generated = |line: &Line| line.ending.iter().all(|(_, owner)| owner.is_none());
    let last = lines.len() - 1;
    for index in 1..last {
        let blank = lines[index].text.trim_matches([' ', '\t']).is_empty();
        if !blank || encoded[index - 1] {
            continue;
        }
        let before = &lines[index - 1];
        if generated(before) {
            encoded[index - 1] = true;
        } else if generated(&lines[index]) {
            encoded[index] = true;
        } else {
            encoded[index - 1] = true;
            for &(_, owner) in &before.ending {
                if let Some(owner) = owner {
                    unfaithful.push(raw[owner].span.clone());
                }
            }
        }
    }

    let mut out = String::with_capacity(html.len());
    for (index, line) in lines.iter().enumerate() {
        out.push_str(&line.text);
        for &(c, _) in &line.ending {
            if encoded[index] {
                out.push_str(if c == '\r' { "&#13;" } else { "&#10;" });
            } else {
                out.push(c);
            }
        }
    }
    unfaithful.dedup();
    (out, unfaithful)
}

/// The backtick fence for a code block holding `value`: three backticks, or
/// one more than the longest backtick run that starts a line of `value`, so
/// no body line can close the block early (CommonMark closes a fence on a
/// line holding a run at least as long as the opener).
fn code_block_fence(value: &str) -> String {
    let longest = split_line_endings(value)
        .map(|(line, _)| line.trim_start().chars().take_while(|&c| c == '`').count())
        .max()
        .unwrap_or(0);
    "`".repeat(longest.max(2) + 1)
}

/// Escapes a literal text value for Markdown inline content so a CommonMark
/// or GFM reader (and darkmatter's `==` mark extension) sees exactly the
/// value: formatting comes only from structural nodes, never from text.
///
/// Each character is escaped only where a reader could take it as syntax, so
/// ordinary prose such as `snake_case`, `C:\dir`, `a < b`, and `AT&T rocks`
/// stays byte-identical:
///
/// - a `\` before ASCII punctuation, a line ending, or the value's end (the
///   next character then comes from a sibling the value cannot see);
/// - a `*`, `_`, or `~` that could open or close a delimiter run, judged by
///   CommonMark flanking with an unknown neighbor at either end of the value;
/// - every `` ` ``, `[`, and `]` (code spans, links, footnotes);
/// - a `==` pair, and a `=` at either end of the value;
/// - a `<` that could start an HTML tag, a URI autolink, or an email
///   autolink (whose local part may start with a digit or punctuation);
/// - a `&` that could start an entity or character reference;
/// - a lone carriage return, which a reader takes as a line ending, as
///   `&#13;`.
///
/// With `html` set (the body of a MarkdownPlus inline-HTML element) `<`,
/// `>`, and `&` are HTML-escaped instead. Line-start block syntax is handled
/// separately by [`LineWriter`] and [`protect_lines`].
fn escape_markdown_text(text: &str, html: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 4);
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        let prev = index.checked_sub(1).map(|i| chars[i]);
        let next = chars.get(index + 1).copied();
        match c {
            '\\' => {
                out.push('\\');
                if next.is_none_or(|n| n.is_ascii_punctuation() || n == '\n' || n == '\r') {
                    out.push('\\');
                }
            }
            '*' | '_' | '~' if delimiter_may_act(c, prev, next) => {
                out.push('\\');
                out.push(c);
            }
            '`' | '[' | ']' => {
                out.push('\\');
                out.push(c);
            }
            '=' if next == Some('=') => {
                out.push_str("\\==");
                index += 2;
                continue;
            }
            '=' if prev.is_none() || next.is_none() => out.push_str("\\="),
            '<' if html => out.push_str("&lt;"),
            '>' if html => out.push_str("&gt;"),
            '&' if html => out.push_str("&amp;"),
            '<' if may_open_tag_or_autolink(&chars[index + 1..]) => out.push_str("\\<"),
            '&' if may_start_reference(&chars[index + 1..]) => out.push_str("\\&"),
            // A lone carriage return ends a line for a reader; a line feed
            // after it is the ordinary CRLF line ending.
            '\r' if next != Some('\n') => out.push_str("&#13;"),
            _ => out.push(c),
        }
        index += 1;
    }
    out
}

/// [`escape_markdown_text`] plus [`protect_lines`] on every line, for a value
/// written on its own as inline content; see [`crate::markdown::escape_text`].
pub(crate) fn escape_literal_text(value: &str) -> String {
    protect_lines(&escape_markdown_text(value, false), true)
}

/// Whether a `<` followed by `rest` could open raw HTML or an autolink.
///
/// Raw HTML and URI autolinks start with a letter, `/`, `!`, or `?`. An email
/// autolink's local part may also start with a digit or one of
/// ``.!#$%&'*+/=?^_`{|}~-``, so such a start counts when the rest could
/// still complete `local@domain>`. Reaching the value's end counts too,
/// since a sibling could supply the rest.
fn may_open_tag_or_autolink(rest: &[char]) -> bool {
    let is_local = |c: char| c.is_ascii_alphanumeric() || ".!#$%&'*+/=?^_`{|}~-".contains(c);
    match rest.first() {
        None => return true,
        Some(&c) if c.is_ascii_alphabetic() || matches!(c, '/' | '!' | '?') => return true,
        Some(&c) if !is_local(c) => return false,
        Some(_) => {}
    }
    let local = rest.iter().take_while(|&&c| is_local(c)).count();
    match rest.get(local) {
        None => return true,
        Some('@') => {}
        Some(_) => return false,
    }
    let domain = &rest[local + 1..];
    let label = domain
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
        .count();
    matches!(domain.get(label), None | Some('>'))
}

/// Whether a `&` followed by `rest` could be read as an entity or numeric
/// character reference: a run of alphanumerics or `#` closed by `;`, or one
/// reaching the value's end, where a sibling could supply the rest.
fn may_start_reference(rest: &[char]) -> bool {
    let run = rest
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric() || **c == '#')
        .count();
    match rest.get(run) {
        None => true,
        Some(';') => run > 0,
        Some(_) => false,
    }
}

/// Whether a `*`, `_`, or `~` between `prev` and `next` could open or close
/// a delimiter run. `None` is the value's edge, which may be any character.
fn delimiter_may_act(c: char, prev: Option<char>, next: Option<char>) -> bool {
    const ANY: &[Flank] = &[Flank::Space, Flank::Punct, Flank::Word];
    let options = |side: Option<char>| side.map_or(ANY, |c| Flank::of(c).resolutions());
    options(prev).iter().any(|&before| {
        options(next).iter().any(|&after| {
            let left = left_flanking(before, after);
            let right = right_flanking(before, after);
            if c == '_' {
                (left && (!right || before == Flank::Punct))
                    || (right && (!left || after == Flank::Punct))
            } else {
                left || right
            }
        })
    })
}

/// Protects each line of already-escaped inline Markdown so a reader keeps
/// it as paragraph text with its whitespace: see [`line_start_edit`] for the
/// start of a line and [`encode_line_end`] for the end before each line
/// ending. The first line's start is checked only when `first_line` is set;
/// the end of the last line is left alone, since text may follow it.
fn protect_lines(text: &str, first_line: bool) -> String {
    let mut out = LineWriter::new(first_line, true);
    out.push_literal(text);
    out.text
}

/// How the start of a line must change so a reader keeps it as paragraph
/// text holding exactly its characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineEdit {
    /// A backslash goes before this byte offset: the line would start a
    /// block (`# `, `> `, `- `, `+ `, `* `, `1. `, `1) `, a setext underline,
    /// or a thematic break).
    Escape(usize),
    /// The leading space or tab becomes a character reference. A reader
    /// strips a line's leading whitespace and reads four columns of it as
    /// indented code; the reference is ordinary text to both rules.
    Entity,
}

/// The edit that keeps `line`, the start of a line of escaped inline
/// Markdown, from starting a block or losing its leading whitespace.
fn line_start_edit(line: &str) -> Option<LineEdit> {
    let bytes = line.as_bytes();
    let first = *bytes.first()?;
    // A marker must be followed by whitespace or the line's end; the end of
    // the text may be followed by a break or nothing at all.
    let ends_marker = |at: usize| matches!(bytes.get(at), None | Some(b' ' | b'\t'));
    let trimmed = line.trim_end();
    let underline = matches!(first, b'-' | b'=') && trimmed.bytes().all(|b| b == first);
    let thematic = matches!(first, b'-' | b'*' | b'_')
        && line.bytes().all(|b| b == first || b == b' ' || b == b'\t')
        && line.bytes().filter(|&b| b == first).count() >= 3;
    match first {
        b' ' | b'\t' => Some(LineEdit::Entity),
        _ if is_table_delimiter_row(line) => Some(LineEdit::Escape(0)),
        b'>' => Some(LineEdit::Escape(0)),
        b'-' | b'+' | b'*' if ends_marker(1) => Some(LineEdit::Escape(0)),
        _ if underline || thematic => Some(LineEdit::Escape(0)),
        b'#' => {
            let run = bytes.iter().take_while(|&&b| b == b'#').count();
            (run <= 6 && ends_marker(run)).then_some(LineEdit::Escape(0))
        }
        b'0'..=b'9' => {
            let run = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
            (run <= 9 && matches!(bytes.get(run), Some(b'.' | b')')) && ends_marker(run + 1))
                .then_some(LineEdit::Escape(run))
        }
        _ => None,
    }
}

/// Whether `line` could still be, or already is, a GFM table delimiter row
/// (`--|--`, `| :- | -: |`): only pipes, hyphens, colons, and whitespace.
fn may_be_table_delimiter_row(line: &str) -> bool {
    line.bytes()
        .all(|b| matches!(b, b'|' | b'-' | b':' | b' ' | b'\t'))
}

/// Whether `line` is a GFM table delimiter row, which turns the line before
/// it into a table header. A backslash before any of its characters breaks
/// the row.
fn is_table_delimiter_row(line: &str) -> bool {
    may_be_table_delimiter_row(line) && line.contains('|') && line.contains('-')
}

/// The character reference for a space or tab.
fn whitespace_entity(c: char) -> &'static str {
    if c == '\t' { "&#9;" } else { "&#32;" }
}

/// Replaces the last character of `text` with its character reference when
/// it is a space or tab (looking past a final `\r` of a CRLF line ending),
/// because a reader strips whitespace at the end of a line, and two spaces
/// before a line ending would make a hard break.
fn encode_line_end(text: &mut String) {
    let cr = usize::from(text.ends_with('\r'));
    let body = &text[..text.len() - cr];
    if let Some(last) = body.chars().next_back().filter(|c| matches!(c, ' ' | '\t')) {
        let at = body.len() - 1;
        let backslashes = body[..at].bytes().rev().take_while(|&b| b == b'\\').count();
        text.replace_range(at..=at, whitespace_entity(last));
        // A literal backslash before the whitespace was left single, since a
        // space is not escapable; before `&` it would escape the reference.
        if backslashes % 2 == 1 {
            text.insert(at, '\\');
        }
    }
}

/// Replaces the first character of `text` with its character reference when
/// it is a space or tab.
fn encode_line_start(text: &mut String) {
    if let Some(first) = text.chars().next().filter(|c| matches!(c, ' ' | '\t')) {
        text.replace_range(0..1, whitespace_entity(first));
    }
}

/// Accumulates inline Markdown while tracking the logical line being
/// written, so block syntax assembled from several adjacent literal values
/// (`1` then `. literal`, or ` ` then `# literal`) is protected as one line.
///
/// Literal content (text values, including whitespace moved outside a
/// delimiter wrapper, and the space a break may be written as) may be
/// edited at a line's start and end; markup written by other nodes is never
/// edited and ends the line's protection.
struct LineWriter {
    text: String,
    /// The byte offset where the current line began, while that line may
    /// still read as a block start. Only a line of literal digits so far
    /// (`12` before `) literal`) or of table-delimiter characters (`|`
    /// before `-|-`) stays open after its first character.
    line: Option<usize>,
    /// The byte offset where the line being written began. A writer that
    /// starts mid-line treats its start as a line start, since it cannot see
    /// what precedes it.
    line_start: usize,
    /// Whether the text is a whole block rather than a mid-line sequence.
    block: bool,
    /// Whether a line begun by a line ending written here can start a block.
    guards_lines: bool,
    /// Whether the text ends with literal content, whose trailing
    /// whitespace may be encoded at a line's end.
    literal_tail: bool,
}

impl LineWriter {
    fn new(at_line_start: bool, guards_lines: bool) -> Self {
        Self {
            text: String::new(),
            line: (at_line_start && guards_lines).then_some(0),
            line_start: 0,
            block: at_line_start,
            guards_lines,
            literal_tail: false,
        }
    }

    /// Whether the line being written holds nothing but spaces or tabs, so
    /// a line ending now would leave a blank line (ending the paragraph) or
    /// a line holding only an HTML open tag (starting a raw HTML block).
    fn line_is_blank(&self) -> bool {
        self.text[self.line_start..]
            .bytes()
            .all(|b| matches!(b, b' ' | b'\t'))
    }

    /// Writes escaped literal content, protecting each line it starts or
    /// continues and the end of each line it finishes.
    ///
    /// Inside a mid-line sequence (a link label, a span or delimiter body) a
    /// literal line ending on a blank line is written as a space: a blank
    /// line there would end the enclosing syntax, and a line holding only a
    /// span's open tag would start a raw HTML block. In a block a literal
    /// blank line is kept; it separates paragraphs by design (`Compose`
    /// places one between the paragraphs of a `Prose`).
    ///
    /// A line feed that follows a carriage return written by markup
    /// completes that line ending rather than ending the (empty) line after
    /// it, since a reader takes the two as one CRLF.
    fn push_literal(&mut self, literal: &str) {
        if literal.is_empty() {
            return;
        }
        let literal = match literal.strip_prefix('\n') {
            Some(rest) if self.text.ends_with('\r') => {
                self.text.push('\n');
                self.line_start = self.text.len();
                self.line = self.guards_lines.then_some(self.text.len());
                rest
            }
            _ => literal,
        };
        let guards_mid_line = self.guards_lines && !self.block;
        let normalized;
        let literal = if guards_mid_line && literal.contains("\r\n") {
            normalized = literal.replace("\r\n", "\n");
            &normalized
        } else {
            literal
        };
        for (index, segment) in literal.split('\n').enumerate() {
            if index > 0 {
                if guards_mid_line && self.line_is_blank() {
                    self.push_segment(" ");
                } else {
                    self.end_line("\n");
                }
            }
            self.push_segment(segment);
        }
    }

    /// Writes literal content holding no line ending.
    fn push_segment(&mut self, segment: &str) {
        if segment.is_empty() {
            return;
        }
        self.literal_tail = true;
        let Some(start) = self.line else {
            self.text.push_str(segment);
            return;
        };
        let written = self.text.len() - start;
        self.text.push_str(segment);
        let line = &self.text[start..];
        match line_start_edit(line) {
            // A delimiter row completed by this segment is broken at the
            // segment's first character, which is not whitespace: the row
            // was not one before it.
            Some(LineEdit::Escape(_)) if written > 0 && is_table_delimiter_row(line) => {
                let skip = segment.len() - segment.trim_start().len();
                self.text.insert(start + written + skip, '\\');
            }
            Some(LineEdit::Escape(at)) if at >= written => self.text.insert(start + at, '\\'),
            Some(LineEdit::Entity) if written == 0 => {
                let mut line = self.text.split_off(start);
                encode_line_start(&mut line);
                self.text.push_str(&line);
            }
            _ => {}
        }
        let line = &self.text[start..];
        if !line.bytes().all(|b| b.is_ascii_digit()) && !may_be_table_delimiter_row(line) {
            self.line = None;
        }
    }

    /// Writes markup from a non-literal node. Its first character settles
    /// the current line, and a line it ends opens the next one. A raw
    /// payload's carriage return is a line ending too, as a reader takes it.
    fn push_markup(&mut self, markup: &str) {
        if markup.is_empty() {
            return;
        }
        self.text.push_str(markup);
        self.literal_tail = false;
        if let Some(at) = markup.rfind(['\n', '\r']) {
            self.line_start = self.text.len() - markup.len() + at + 1;
        }
        self.line =
            (self.guards_lines && markup.ends_with(['\n', '\r'])).then_some(self.text.len());
    }

    /// Finishes the current line with `ending` and opens the next one.
    fn end_line(&mut self, ending: &str) {
        self.finish();
        self.text.push_str(ending);
        self.literal_tail = false;
        self.line_start = self.text.len();
        self.line = self.guards_lines.then_some(self.text.len());
    }

    /// Encodes trailing literal whitespace where a line ends.
    fn finish(&mut self) {
        if self.guards_lines && self.literal_tail {
            encode_line_end(&mut self.text);
        }
    }
}

/// Backslash-escapes each literal `\` a CommonMark reader would otherwise
/// consume as an escape: one before ASCII punctuation or a line ending, or at
/// the end of the value, where a sibling supplies the next character.
fn escape_literal_backslashes(text: &str) -> String {
    if !text.contains('\\') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 2);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c == '\\' {
            match chars.peek() {
                None => out.push('\\'),
                Some(next) if next.is_ascii_punctuation() || *next == '\n' || *next == '\r' => {
                    out.push('\\');
                }
                Some(_) => {}
            }
        }
    }
    out
}

/// Escapes a link title for its double-quoted form: a `"` would end the title
/// early, a trailing `\\` would escape the closing quote, and a reader decodes
/// entity references there (`&copy;` must not become `©`).
///
/// Each CR and LF is written `&#13;` / `&#10;` (a CRLF both), which a reader
/// decodes back into the title value. A literal line ending would end a table
/// row or heading, and a blank line would end the title; a `<br>` would be
/// literal title text. The references are added after the `&` escape, so they
/// are the only ones a reader decodes, and after the backslash escape, which
/// doubles a `\\` before a line ending so it cannot escape the reference's
/// `&`.
fn escape_link_title(title: &str) -> String {
    encode_line_endings(
        &escape_references(&escape_literal_backslashes(title)).replace('"', "\\\""),
    )
}

/// Writes each CR as `&#13;` and each LF as `&#10;`, so a CRLF is both. For
/// fields a reader decodes character references in (a link title, a quoted
/// HTML attribute value), this keeps the value exact and on one line.
fn encode_line_endings(value: &str) -> String {
    if !value.contains(['\r', '\n']) {
        return value.to_string();
    }
    value.replace('\r', "&#13;").replace('\n', "&#10;")
}

/// Backslash-escapes each `&` a reader could decode as an entity or numeric
/// character reference, for link titles and destinations, where Markdown
/// punctuation is otherwise literal.
fn escape_references(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 2);
    for (index, &c) in chars.iter().enumerate() {
        if c == '&' && may_start_reference(&chars[index + 1..]) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The one cell rule for a pipe: writes `\|` for every `|` in Markdown that
/// is already correct for its own context (escaped text, a code span body, a
/// raw HTML payload, a footnote identifier, a link destination or title).
///
/// GFM splits a row into cells before it parses anything inline, and while
/// splitting it removes exactly one backslash in front of each pipe, inside
/// code spans and raw HTML as well. So the cell then holds exactly the
/// Markdown this function was given, whatever came before the pipe: a
/// payload's own `\|` is written `\\|` and reads back as `\|`. Applying it
/// twice would add a second backslash, so each site applies it once, last.
///
/// pulldown-cmark and markdown-rs remove that backslash in text and code
/// spans but keep it inside an inline HTML token (a tag or comment), where
/// such a reader shows `\|` for the payload's `|`; the row and its cells
/// still read correctly. Generated attribute values avoid the difference
/// with `&#124;` instead (see [`encode_generated_attribute`]).
fn escape_cell_pipes(markdown: &str) -> String {
    markdown.replace('|', "\\|")
}

/// Keeps a generated, already attribute-escaped quoted HTML attribute value
/// on its Markdown line without changing what an HTML reader decodes, in any
/// context: each CR and LF becomes a character reference (see
/// [`encode_line_endings`]), since a literal one would end a table row or
/// heading and a blank line would end the paragraph holding the inline tag.
/// With `in_cell`, each `|` also becomes `&#124;`. Unlike the `\|` of
/// [`escape_cell_pipes`], that reads the same in readers that keep a pipe
/// escape inside an inline HTML tag.
///
/// Also applied to a whole generated element whose only line endings and
/// pipes sit in such attribute values. Never applied to raw HTML, where a
/// reference is not decoded in every position.
fn encode_generated_attribute(value: &str, in_cell: bool) -> String {
    let value = encode_line_endings(value);
    if in_cell {
        value.replace('|', "&#124;")
    } else {
        value
    }
}

/// Keeps text escaped by [`escape_markdown_text`] on a heading's one ATX
/// line: each line feed is written `&#10;`, and the CR of a CRLF `&#13;`,
/// since left alone it would end the heading by itself (a lone CR is already
/// `&#13;`). A reader decodes both back into the text.
fn encode_heading_line_feeds(text: &str) -> String {
    text.replace("\r\n", "&#13;&#10;").replace('\n', "&#10;")
}

/// Escapes a text node for safe placement inside a GFM table cell.
///
/// Literal pipes follow [`escape_cell_pipes`]; literal newlines become
/// `<br>` so the cell does not split the pipe-delimited row.
fn escape_table_cell_text(text: &str) -> String {
    escape_cell_pipes(text)
        .replace("\r\n", "<br>")
        .replace('\n', "<br>")
}

/// Escapes a link/image destination for placement inside a GFM table cell:
/// the standalone CommonMark escape of [`escape_markdown_destination`], then
/// [`escape_cell_pipes`]. A table reader removes that escape before the
/// destination is parsed.
fn escape_cell_link_destination(url: &str) -> String {
    escape_cell_pipes(&escape_markdown_destination(url))
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
/// `<` and `>`; line endings, invalid in either form, degrade to spaces. A
/// reader decodes entity references in either form, so a `&` that could
/// start one is escaped too (`https://x.io/&copy;` stays exact).
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
        escape_references(&out)
    } else {
        let mut out = String::with_capacity(url.len());
        for (index, c) in url.chars().enumerate() {
            match c {
                '\\' | '(' | ')' => {
                    out.push('\\');
                    out.push(c);
                }
                // A leading `<` would open the angle-bracket form.
                '<' if index == 0 => {
                    out.push('\\');
                    out.push(c);
                }
                _ => out.push(c),
            }
        }
        escape_references(&out)
    }
}

/// Builds the semantic progress-widget HTML emitted by MarkdownPlus.
///
/// The shape mirrors the browser renderer's progress HTML: an outer element
/// with `role="progressbar"` and ARIA attributes, plus `progress-*` classes.
/// Color slots lower to inline CSS; non-default glyphs and brackets are
/// preserved in `data-*` attributes. `Layout` is not applied — Markdown
/// ignores layout by contract — so the outer style carries only color.
///
/// The visible label sits in an inline element whose body a reader parses as
/// Markdown, so it is escaped like a MarkdownPlus span body.
fn progress_html(hints: &crate::tree::ProgressHints, fallback_text: &str) -> String {
    super::shared::progress_html_with_label(hints, fallback_text, "", |label| {
        // A line ending is whitespace in the label's HTML element, and two
        // of them would end the paragraph that holds the widget.
        escape_markdown_text(&label.replace(['\r', '\n'], " "), true)
    })
}

/// The continuation indent of a GFM footnote definition.
const FOOTNOTE_CONTINUATION: &str = "    ";

/// Indents continuation lines (every line after the first) by `indent`,
/// keeping each line's ending bytes; see [`split_line_endings`] for what
/// ends a line. Used for list items and footnote definitions. A final line
/// feed is dropped, since the enclosing join writes one after the container;
/// a carriage return before it stays, completing a CRLF with the joining
/// line feed.
fn indent_continuation(text: &str, indent: &str) -> String {
    let text = text.strip_suffix('\n').unwrap_or(text);
    let mut output = String::with_capacity(text.len());
    for (index, (line, ending)) in split_line_endings(text).enumerate() {
        if index > 0 && !line.is_empty() {
            output.push_str(indent);
        }
        output.push_str(line);
        output.push_str(ending);
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
    fn drops_browser_only_attrs_in_both_dialects() {
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
        let between = |node: RenderNode| {
            render(&RenderNode::paragraph(vec![
                RenderNode::text("a"),
                node,
                RenderNode::text("b"),
            ]))
            .output
        };
        assert_eq!(between(RenderNode::soft_break()), "a\nb");
        assert_eq!(between(RenderNode::hard_break()), "a\\\nb");
        // With nothing after it in its block, a break has no line-ending
        // spelling: a reader strips the line ending and reads the backslash.
        assert_eq!(render(&RenderNode::soft_break()).output, "&#32;");
        assert_eq!(render(&RenderNode::hard_break()).output, "<br>");
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
    fn progress_markdown_plus_label_stays_literal_in_attribute_and_body() {
        let mut para =
            RenderNode::paragraph(vec![RenderNode::text(r"**a** &copy; <em>\ 60%")]);
        para.attrs.set_progress_hints(&crate::tree::ProgressHints {
            value: 0.6,
            ..Default::default()
        });
        let out = render_with(
            &para,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn),
        )
        .output;
        // The attribute is raw HTML; the label element's body is Markdown.
        assert!(
            out.contains(r#"aria-label="**a** &amp;copy; &lt;em&gt;\""#),
            "{out}"
        );
        assert!(
            out.contains(r#"<span class="progress-label">\*\*a\*\* &amp;copy; &lt;em&gt;\\</span>"#),
            "{out}"
        );
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
    fn paragraph_block_element_does_not_change_markdown() {
        let plain = RenderNode::root(vec![
            RenderNode::paragraph(vec![RenderNode::text("one")]),
            RenderNode::paragraph(vec![RenderNode::text("two")]),
        ]);
        for dialect in [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus] {
            let opts = MarkdownRenderOptions {
                dialect,
                ..Default::default()
            };
            let expected = render_markdown_node(&plain, &opts).unwrap().output;
            for element in crate::tree::BlockElement::ALL {
                let mut tagged = plain.clone();
                for child in tagged.children_mut().unwrap() {
                    child.attrs.browser_mut_or_default().block_element = element;
                }
                let rendered = render_markdown_node(&tagged, &opts).unwrap();
                assert_eq!(rendered.output, expected, "{dialect:?} {element:?}");
                assert!(rendered.diagnostics.is_empty(), "{dialect:?} {element:?}");
            }
        }
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

    /// `(value, outside a table, inside a table cell)` for directly constructed
    /// inline-code nodes. The table column applies the pipe escape before the
    /// fence is chosen.
    const INLINE_CODE_FENCES: &[(&str, &str, &str)] = &[
        ("plain", "`plain`", "`plain`"),
        ("a`b", "``a`b``", "``a`b``"),
        ("`a`", "`` `a` ``", "`` `a` ``"),
        ("a``b", "```a``b```", "```a``b```"),
        (" a ", "`  a  `", "`  a  `"),
        (" a", "` a`", "` a`"),
        ("  ", "`  `", "`  `"),
        ("a|b", "`a|b`", r"`a\|b`"),
        ("|`|", "``|`|``", r"``\|`\|``"),
        ("`|", "`` `| ``", r"`` `\| ``"),
        ("one\ntwo", "`one two`", "`one two`"),
        ("one\r\ntwo", "`one two`", "`one two`"),
        ("", "", ""),
        (r"a\_b", r"`a\_b`", r"`a\_b`"),
    ];

    #[test]
    fn inline_code_uses_a_safe_fence_in_both_dialects() {
        for dialect in [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus] {
            let opts = opts(dialect, RenderStrictness::Warn);
            for (value, outside, in_cell) in INLINE_CODE_FENCES {
                let node = RenderNode::inline_code(*value);
                assert_eq!(render_with(&node, &opts).output, *outside, "{dialect:?} {value:?}");

                let para = RenderNode::paragraph(vec![
                    RenderNode::text("see "),
                    RenderNode::inline_code(*value),
                    RenderNode::text(" here"),
                ]);
                assert_eq!(
                    render_with(&para, &opts).output,
                    format!("see {outside} here"),
                    "{dialect:?} {value:?}"
                );

                let table = render_with(&one_cell_table(node), &opts).output;
                let row = table.lines().last().unwrap();
                let expected_row = if in_cell.is_empty() {
                    "|  |".to_string()
                } else {
                    format!("| {in_cell} |")
                };
                assert_eq!(row, expected_row, "{dialect:?} {value:?}\n{table}");
            }
        }
    }

    #[test]
    fn hard_break_uses_backslash_outside_tables_and_br_inside() {
        for dialect in [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus] {
            let opts = opts(dialect, RenderStrictness::Warn);
            let broken = || {
                vec![
                    RenderNode::text("first"),
                    RenderNode::hard_break(),
                    RenderNode::text("second"),
                ]
            };
            let para = render_with(&RenderNode::paragraph(broken()), &opts).output;
            assert_eq!(para, "first\\\nsecond", "{dialect:?}");
            // Never the trailing-space form.
            assert!(!para.contains("  \n"), "{dialect:?}");

            let cell = render_with(&one_cell_table(RenderNode::span(vec![], broken())), &opts);
            assert_eq!(
                cell.output.lines().last().unwrap(),
                "| first<br>second |",
                "{dialect:?}"
            );
        }
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
    fn table_cell_link_url_newline_degrades_to_space_as_outside_a_cell() {
        let link = RenderNode::link("a\nb", None, vec![RenderNode::text("x")]);
        let out = render(&one_cell_table(link)).output;
        assert!(out.contains("(<a b>)"), "{out}");
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
        // The container is one raw HTML block, which a reader does not parse
        // as Markdown, so its content is the browser's HTML.
        assert!(out.contains("<p>plain <strong>bold</strong></p>"), "{out}");
        assert!(out.contains("<p><em>italic</em></p>"), "{out}");
        assert!(!out.contains("**"), "{out}");
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
        assert!(out.contains("<ul><li><p>li</p></li></ul>"), "{out}");
    }

    #[test]
    fn columns_markdown_plus_image_column_renders_without_malformed_html() {
        // An image in the HTML container is an `<img>` element; the column
        // markup stays well-formed under Warn.
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
        assert!(out.contains(r#"<img src="pic.png" alt="alt">"#), "{out}");
        assert_eq!(out.matches("<div").count(), 3, "{out}");
        assert_eq!(out.matches("</div>").count(), 3, "{out}");

        // Under Strict the same content renders without error: an image has
        // an HTML form, so no lossy rejection is triggered.
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
    fn columns_markdown_plus_multiple_blocks_per_column_stay_one_html_block() {
        // A blank line would end the raw HTML block, so two blocks in a
        // column are adjacent HTML elements with no blank line between them.
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
        assert_eq!(out.matches("<div").count(), 3, "{out}");
        assert_eq!(out.matches("</div>").count(), 3, "{out}");
        assert!(out.contains("<p>left-one</p><p>left-two</p>"), "{out}");
        assert!(!out.contains("\n\n"), "{out}");
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

/// Literal backslashes in text must survive a CommonMark reader no matter
/// which break or wrapper syntax follows them. Every assertion parses the
/// rendered Markdown with `pulldown-cmark`, an independent reader, and
/// compares the visible text and break kinds.
#[cfg(test)]
mod literal_backslash_tests {
    use super::*;
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

    const DIALECTS: [MarkdownDialect; 2] =
        [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

    fn render_in(node: &RenderNode, dialect: MarkdownDialect) -> String {
        let opts = MarkdownRenderOptions {
            dialect,
            strictness: RenderStrictness::Warn,
            style: None,
        };
        render_markdown_node(node, &opts).expect("render").output
    }

    /// Reads `markdown` back and returns its visible text, with `{SB}` for a
    /// soft break, `{HB}` for a hard break, `<strong>`/`<em>`/`<del>` markers, `|` at each
    /// table cell, and inline HTML verbatim.
    fn read_back(markdown: &str) -> String {
        let mut out = String::new();
        let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
        for event in Parser::new_ext(markdown, options) {
            match event {
                Event::Text(text) => out.push_str(&text),
                Event::SoftBreak => out.push_str("{SB}"),
                Event::HardBreak => out.push_str("{HB}"),
                Event::InlineHtml(html) => out.push_str(&html),
                Event::Start(Tag::Strong) => out.push_str("<strong>"),
                Event::End(TagEnd::Strong) => out.push_str("</strong>"),
                Event::Start(Tag::Emphasis) => out.push_str("<em>"),
                Event::End(TagEnd::Emphasis) => out.push_str("</em>"),
                Event::Start(Tag::Strikethrough) => out.push_str("<del>"),
                Event::End(TagEnd::Strikethrough) => out.push_str("</del>"),
                Event::Start(Tag::TableCell) => out.push('|'),
                Event::Start(Tag::Link { title, .. } | Tag::Image { title, .. }) => {
                    out.push_str(&format!("[title={title}]"));
                }
                _ => {}
            }
        }
        out
    }

    fn breaks() -> [(RenderNode, &'static str); 2] {
        [
            (RenderNode::soft_break(), "{SB}"),
            (RenderNode::hard_break(), "{HB}"),
        ]
    }

    #[test]
    fn text_ending_in_backslashes_keeps_them_before_either_break() {
        for dialect in DIALECTS {
            for literal in ["a\\", "a\\\\"] {
                for (brk, marker) in breaks() {
                    let para = RenderNode::paragraph(vec![
                        RenderNode::text(literal),
                        brk,
                        RenderNode::text("b"),
                    ]);
                    let markdown = render_in(&para, dialect);
                    assert_eq!(
                        read_back(&markdown),
                        format!("{literal}{marker}b"),
                        "{dialect:?} {literal:?} {marker}\n{markdown}"
                    );
                }
            }
        }
    }

    #[test]
    fn text_with_embedded_backslash_newline_stays_literal() {
        for dialect in DIALECTS {
            let para = RenderNode::paragraph(vec![RenderNode::text("a\\\nb")]);
            let markdown = render_in(&para, dialect);
            assert_eq!(read_back(&markdown), "a\\{SB}b", "{dialect:?}\n{markdown}");
        }
    }

    #[test]
    fn strong_text_ending_in_backslash_keeps_valid_emphasis_before_either_break() {
        for dialect in DIALECTS {
            for (brk, marker) in breaks() {
                let para = RenderNode::paragraph(vec![
                    RenderNode::strong(vec![RenderNode::text("a\\")]),
                    brk,
                    RenderNode::text("b"),
                ]);
                let markdown = render_in(&para, dialect);
                assert_eq!(
                    read_back(&markdown),
                    format!("<strong>a\\</strong>{marker}b"),
                    "{dialect:?} {marker}\n{markdown}"
                );
            }
        }
    }

    fn cell_table(cell: Vec<RenderNode>) -> RenderNode {
        RenderNode::table(
            vec![ColumnAlign::None],
            vec![
                RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("H")])]),
                RenderNode::table_row(vec![RenderNode::table_cell(cell)]),
            ],
        )
    }

    #[test]
    fn table_cell_text_ending_in_backslash_keeps_it_before_either_break() {
        for dialect in DIALECTS {
            for (brk, marker) in breaks() {
                let cell_marker = if marker == "{SB}" { " " } else { "<br>" };
                let table = cell_table(vec![RenderNode::text("a\\"), brk, RenderNode::text("b")]);
                let markdown = render_in(&table, dialect);
                assert_eq!(
                    read_back(&markdown),
                    format!("|H|a\\{cell_marker}b"),
                    "{dialect:?} {marker}\n{markdown}"
                );
            }
        }
    }

    #[test]
    fn strong_in_table_cell_keeps_trailing_backslash_inside_valid_emphasis() {
        for dialect in DIALECTS {
            for (brk, marker) in breaks() {
                let cell_marker = if marker == "{SB}" { " " } else { "<br>" };
                let table = cell_table(vec![
                    RenderNode::strong(vec![RenderNode::text("a\\")]),
                    brk,
                    RenderNode::text("b"),
                ]);
                let markdown = render_in(&table, dialect);
                assert_eq!(
                    read_back(&markdown),
                    format!("|H|<strong>a\\</strong>{cell_marker}b"),
                    "{dialect:?} {marker}\n{markdown}"
                );
            }
        }
    }

    #[test]
    fn table_cell_backslash_before_pipe_stays_literal() {
        for dialect in DIALECTS {
            let table = cell_table(vec![RenderNode::text("a\\|b")]);
            let markdown = render_in(&table, dialect);
            assert_eq!(read_back(&markdown), "|H|a\\|b", "{dialect:?}\n{markdown}");
        }
    }

    #[test]
    fn styled_span_body_keeps_backslash_before_escaped_markup() {
        let span = RenderNode::span(vec!["x".to_string()], vec![RenderNode::text("a\\<b>\\")]);
        let para =
            RenderNode::paragraph(vec![span, RenderNode::hard_break(), RenderNode::text("c")]);
        let markdown = render_in(&para, MarkdownDialect::MarkdownPlus);
        assert_eq!(
            read_back(&markdown),
            "<span class=\"x\">a\\<b>\\</span>{HB}c",
            "{markdown}"
        );
    }

    #[test]
    fn link_text_image_alt_and_titles_keep_trailing_backslashes() {
        for dialect in DIALECTS {
            let para = RenderNode::paragraph(vec![
                RenderNode::link(
                    "https://e.io",
                    Some("say \"hi\"\\".to_string()),
                    vec![RenderNode::text("go\\")],
                ),
                RenderNode::text(" "),
                RenderNode::image("i.png", None, "alt\\"),
            ]);
            let markdown = render_in(&para, dialect);
            assert_eq!(
                read_back(&markdown),
                "[title=say \"hi\"\\]go\\ [title=]alt\\",
                "{dialect:?}\n{markdown}"
            );
        }
    }

    #[test]
    fn backslash_before_ordinary_characters_is_byte_identical() {
        for dialect in DIALECTS {
            let para = RenderNode::paragraph(vec![RenderNode::text("C:\\dir\\file.txt")]);
            assert_eq!(
                render_in(&para, dialect),
                "C:\\dir\\file.txt",
                "{dialect:?}"
            );
        }
    }

    #[test]
    fn plain_breaks_without_backslashes_are_unchanged() {
        for dialect in DIALECTS {
            for (brk, expected) in [
                (RenderNode::soft_break(), "a\nb"),
                (RenderNode::hard_break(), "a\\\nb"),
            ] {
                let para =
                    RenderNode::paragraph(vec![RenderNode::text("a"), brk, RenderNode::text("b")]);
                assert_eq!(render_in(&para, dialect), expected, "{dialect:?}");
            }
        }
    }

    type Wrap = fn(Vec<RenderNode>) -> RenderNode;

    const WRAPPERS: [(Wrap, &str); 3] = [
        (RenderNode::emphasis, "em"),
        (RenderNode::strong, "strong"),
        (RenderNode::delete, "del"),
    ];

    /// Every delimiter character at the leading edge, the trailing edge, both
    /// edges, and as the whole text.
    fn edge_texts() -> Vec<String> {
        ['*', '_', '~']
            .into_iter()
            .flat_map(|c| {
                [
                    format!("{c}a"),
                    format!("a{c}"),
                    format!("{c}a{c}"),
                    c.to_string(),
                ]
            })
            .collect()
    }

    #[test]
    fn delimiter_at_a_wrapper_edge_stays_literal_inside_the_wrapper() {
        for dialect in DIALECTS {
            for (wrap, tag) in WRAPPERS {
                for text in edge_texts() {
                    let para = RenderNode::paragraph(vec![
                        RenderNode::text("x "),
                        wrap(vec![RenderNode::text(text.as_str())]),
                        RenderNode::text(" y"),
                    ]);
                    let markdown = render_in(&para, dialect);
                    assert_eq!(
                        read_back(&markdown),
                        format!("x <{tag}>{text}</{tag}> y"),
                        "{dialect:?} {tag} {text:?}\n{markdown}"
                    );
                }
            }
        }
    }

    #[test]
    fn delimiter_at_a_wrapper_edge_stays_literal_in_a_table_cell() {
        for dialect in DIALECTS {
            for (wrap, tag) in WRAPPERS {
                for text in edge_texts() {
                    let table = cell_table(vec![wrap(vec![RenderNode::text(text.as_str())])]);
                    let markdown = render_in(&table, dialect);
                    assert_eq!(
                        read_back(&markdown),
                        format!("|H|<{tag}>{text}</{tag}>"),
                        "{dialect:?} {tag} {text:?}\n{markdown}"
                    );
                }
            }
        }
    }

    #[test]
    fn edge_delimiter_and_trailing_backslash_combine() {
        for dialect in DIALECTS {
            let para = RenderNode::paragraph(vec![
                RenderNode::strong(vec![RenderNode::text("*a\\*")]),
                RenderNode::hard_break(),
                RenderNode::text("b"),
            ]);
            let markdown = render_in(&para, dialect);
            assert_eq!(
                read_back(&markdown),
                "<strong>*a\\*</strong>{HB}b",
                "{dialect:?}\n{markdown}"
            );
        }
    }

    #[test]
    fn nested_wrapper_edges_are_escaped_at_the_innermost_text() {
        for dialect in DIALECTS {
            let para =
                RenderNode::paragraph(vec![RenderNode::strong(vec![RenderNode::emphasis(vec![
                    RenderNode::text("*a*"),
                ])])]);
            let markdown = render_in(&para, dialect);
            assert_eq!(
                read_back(&markdown),
                "<strong><em>*a*</em></strong>",
                "{dialect:?}\n{markdown}"
            );
        }
    }

    #[test]
    fn delimiter_characters_are_escaped_only_where_they_could_act() {
        for dialect in DIALECTS {
            let para = RenderNode::paragraph(vec![
                RenderNode::text("*x* snake_case 2 * 3 "),
                RenderNode::strong(vec![RenderNode::text("a*b_c~d")]),
            ]);
            let markdown = render_in(&para, dialect);
            assert_eq!(
                markdown, "\\*x\\* snake_case 2 * 3 **a\\*b_c\\~d**",
                "{dialect:?}"
            );
            assert_eq!(
                read_back(&markdown),
                "*x* snake_case 2 * 3 <strong>a*b_c~d</strong>",
                "{dialect:?}"
            );
        }
    }
}

#[cfg(test)]
mod code_block_fence_tests {
    use super::*;
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};

    /// Each code block's text and language as a CommonMark reader sees them.
    fn read_code_blocks(markdown: &str) -> Vec<(String, String)> {
        let mut blocks = Vec::new();
        let mut current: Option<(String, String)> = None;
        for event in Parser::new(markdown) {
            match event {
                Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Fenced(info))) => {
                    current = Some((info.to_string(), String::new()));
                }
                Event::Text(text) => {
                    if let Some((_, body)) = current.as_mut() {
                        body.push_str(&text);
                    }
                }
                Event::End(TagEnd::CodeBlock) => blocks.extend(current.take()),
                _ => {}
            }
        }
        blocks
    }

    #[test]
    fn code_block_body_with_fence_lines_reads_back_whole() {
        for dialect in [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus] {
            let opts = MarkdownRenderOptions {
                dialect,
                strictness: RenderStrictness::Warn,
                style: None,
            };
            for (lang, value) in [
                (None, "a\n```\nb\n```\nc"),
                (Some("md"), "  ````rust\nx\n````"),
                (None, "plain"),
                (None, "inline ``` run"),
            ] {
                let node = RenderNode::code(lang.map(String::from), None, value);
                let markdown = render_markdown_node(&node, &opts).expect("render").output;
                assert_eq!(
                    read_code_blocks(&markdown),
                    [(lang.unwrap_or_default().to_string(), format!("{value}\n"))],
                    "{dialect:?} {markdown:?}"
                );
            }
        }
    }

    #[test]
    fn code_block_fence_is_three_backticks_unless_a_line_needs_more() {
        assert_eq!(code_block_fence("plain"), "```");
        assert_eq!(code_block_fence("inline ``` run"), "```");
        assert_eq!(code_block_fence("a\n```\nb"), "````");
        assert_eq!(code_block_fence("  `````x"), "``````");
    }

    #[test]
    fn code_block_fence_reads_a_carriage_return_as_a_line_ending() {
        // A reader starts a line after a lone CR or a CRLF, so a backtick
        // run there could close a three-backtick fence.
        assert_eq!(code_block_fence("a\r```\rb"), "````");
        assert_eq!(code_block_fence("a\r\n````"), "`````");
        for dialect in [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus] {
            let opts = MarkdownRenderOptions {
                dialect,
                strictness: RenderStrictness::Warn,
                style: None,
            };
            let node = RenderNode::code(None, None, "a\r```\rb");
            let markdown = render_markdown_node(&node, &opts).expect("render").output;
            // pulldown-cmark does not end a code block line at a lone CR,
            // so it reads the output with that ending spelled as LF.
            assert_eq!(
                read_code_blocks(&markdown.replace('\r', "\n")),
                [(String::new(), "a\n```\nb\n".to_string())],
                "{dialect:?} {markdown:?}"
            );
        }
    }
}

#[cfg(test)]
mod container_line_ending_tests {
    use super::*;

    #[test]
    fn prefix_lines_keeps_every_line_ending_and_prefixes_after_each() {
        assert_eq!(prefix_lines("a\nb\n", "> "), "> a\n> b\n");
        assert_eq!(prefix_lines("a\r\nb", "> "), "> a\r\n> b");
        assert_eq!(prefix_lines("a\rb\r", "> "), "> a\r> b\r");
        assert_eq!(prefix_lines("", "> "), "");
    }

    #[test]
    fn prefix_lines_writes_a_bare_marker_on_a_blank_line_for_every_ending() {
        assert_eq!(prefix_lines("a\n\nb", "> "), "> a\n>\n> b");
        assert_eq!(prefix_lines("a\r\rb", "> "), "> a\r>\r> b");
        assert_eq!(prefix_lines("a\r\n\r\nb", "> "), "> a\r\n>\r\n> b");
    }

    #[test]
    fn indent_continuation_keeps_endings_and_indents_after_each() {
        assert_eq!(indent_continuation("a\nb\n", "  "), "a\n  b");
        assert_eq!(indent_continuation("a\r\nb", "  "), "a\r\n  b");
        assert_eq!(indent_continuation("a\rb", "  "), "a\r  b");
        assert_eq!(indent_continuation("a\r\rb", "  "), "a\r\r  b");
        // Only a final LF goes; the CR of a final CRLF stays.
        assert_eq!(indent_continuation("a\r\n", "  "), "a\r");
        assert_eq!(indent_continuation("", "  "), "");
    }
}

/// Delimiter wrappers whose content starts or ends with whitespace or a break.
///
/// A delimiter run followed (or preceded) by whitespace or a line ending does
/// not open (or close) emphasis, so these tests read the output back with
/// `pulldown-cmark` and check that the emphasis covers exactly the text.
#[cfg(test)]
mod delimiter_edge_tests {
    use super::*;
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

    const DIALECTS: [MarkdownDialect; 2] =
        [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

    fn render_in(node: &RenderNode, dialect: MarkdownDialect) -> String {
        let opts = MarkdownRenderOptions {
            dialect,
            strictness: RenderStrictness::Warn,
            style: None,
        };
        render_markdown_node(node, &opts).expect("render").output
    }

    /// Reads `markdown` back as visible text with `{SB}`/`{HB}` for breaks,
    /// `<strong>`/`<em>`/`<del>` around emphasis, and inline HTML verbatim.
    fn read_back(markdown: &str) -> String {
        let mut out = String::new();
        let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
        for event in Parser::new_ext(markdown, options) {
            match event {
                Event::Text(text) | Event::Code(text) => out.push_str(&text),
                Event::SoftBreak => out.push_str("{SB}"),
                Event::HardBreak => out.push_str("{HB}"),
                Event::InlineHtml(html) => out.push_str(&html),
                Event::Start(Tag::Strong) => out.push_str("<strong>"),
                Event::End(TagEnd::Strong) => out.push_str("</strong>"),
                Event::Start(Tag::Emphasis) => out.push_str("<em>"),
                Event::End(TagEnd::Emphasis) => out.push_str("</em>"),
                Event::Start(Tag::Strikethrough) => out.push_str("<del>"),
                Event::End(TagEnd::Strikethrough) => out.push_str("</del>"),
                _ => {}
            }
        }
        out
    }

    type Wrap = fn(Vec<RenderNode>) -> RenderNode;

    /// Each CommonMark delimiter wrapper with its read-back tag.
    fn wrappers() -> [(Wrap, &'static str); 3] {
        [
            (RenderNode::emphasis, "em"),
            (RenderNode::strong, "strong"),
            (RenderNode::delete, "del"),
        ]
    }

    /// An edge node with its read-back outside and inside a table cell.
    fn edges() -> Vec<(RenderNode, &'static str, &'static str)> {
        vec![
            (RenderNode::text(" "), " ", " "),
            (RenderNode::text("\t"), "\t", "\t"),
            (RenderNode::soft_break(), "{SB}", " "),
            (RenderNode::hard_break(), "{HB}", "<br>"),
        ]
    }

    fn cell_table(cell: Vec<RenderNode>) -> RenderNode {
        RenderNode::table(
            vec![ColumnAlign::None],
            vec![
                RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("H")])]),
                RenderNode::table_row(vec![RenderNode::table_cell(cell)]),
            ],
        )
    }

    /// Renders `inline` in a paragraph or a table cell and reads it back,
    /// dropping the table header cell.
    fn render_and_read(
        inline: Vec<RenderNode>,
        in_cell: bool,
        dialect: MarkdownDialect,
    ) -> (String, String) {
        let node = if in_cell {
            cell_table(inline)
        } else {
            RenderNode::paragraph(inline)
        };
        let markdown = render_in(&node, dialect);
        let read = read_back(&markdown);
        let read = if in_cell {
            read.strip_prefix('H').unwrap_or(&read).to_string()
        } else {
            read
        };
        (markdown, read)
    }

    #[test]
    fn leading_edges_are_written_outside_every_delimiter() {
        for dialect in DIALECTS {
            for (wrap, tag) in wrappers() {
                for (edge, outside, in_table) in edges() {
                    for in_cell in [false, true] {
                        let inline = vec![
                            RenderNode::text("a"),
                            wrap(vec![edge.clone(), RenderNode::text("b")]),
                            RenderNode::text(" c"),
                        ];
                        let (markdown, read) = render_and_read(inline, in_cell, dialect);
                        let shown = if in_cell { in_table } else { outside };
                        assert_eq!(
                            read,
                            format!("a{shown}<{tag}>b</{tag}> c"),
                            "{dialect:?} {tag} {edge:?} in_cell={in_cell}: {markdown:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn trailing_edges_are_written_outside_every_delimiter() {
        for dialect in DIALECTS {
            for (wrap, tag) in wrappers() {
                for (edge, outside, in_table) in edges() {
                    for in_cell in [false, true] {
                        let inline = vec![
                            RenderNode::text("a "),
                            wrap(vec![RenderNode::text("b"), edge.clone()]),
                            RenderNode::text("c"),
                        ];
                        let (markdown, read) = render_and_read(inline, in_cell, dialect);
                        let shown = if in_cell { in_table } else { outside };
                        assert_eq!(
                            read,
                            format!("a <{tag}>b</{tag}>{shown}c"),
                            "{dialect:?} {tag} {edge:?} in_cell={in_cell}: {markdown:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn text_with_edge_whitespace_keeps_emphasis_on_its_words() {
        for dialect in DIALECTS {
            for (wrap, tag) in wrappers() {
                for in_cell in [false, true] {
                    let inline = vec![
                        RenderNode::text("a"),
                        wrap(vec![RenderNode::text(" \tb c\t ")]),
                        RenderNode::text("d"),
                    ];
                    let (markdown, read) = render_and_read(inline, in_cell, dialect);
                    assert!(
                        read.contains(&format!("<{tag}>b c</{tag}>")),
                        "{dialect:?} {tag} in_cell={in_cell}: {markdown:?} read {read:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn edges_of_nested_wrappers_move_outside_the_outer_delimiters() {
        // `<b><i> a\n</i></b>`: the space and the break leave both wrappers.
        let inline = vec![
            RenderNode::text("x"),
            RenderNode::strong(vec![RenderNode::emphasis(vec![
                RenderNode::text(" a"),
                RenderNode::soft_break(),
            ])]),
            RenderNode::text("y"),
        ];
        for dialect in DIALECTS {
            let (markdown, read) = render_and_read(inline.clone(), false, dialect);
            assert_eq!(markdown, "x **_a_**\ny", "{dialect:?}");
            assert_eq!(read, "x <strong><em>a</em></strong>{SB}y", "{dialect:?}");
        }
    }

    #[test]
    fn a_wrapper_with_only_whitespace_or_breaks_writes_no_delimiters() {
        for dialect in DIALECTS {
            for (wrap, _) in wrappers() {
                let inline = vec![
                    RenderNode::text("a"),
                    wrap(vec![RenderNode::text(" "), RenderNode::soft_break()]),
                    RenderNode::text("b"),
                ];
                let (markdown, read) = render_and_read(inline, false, dialect);
                // The space ends a line, so it is a reference a reader keeps.
                assert_eq!(markdown, "a&#32;\nb", "{dialect:?}");
                assert_eq!(read, "a {SB}b", "{dialect:?}");
            }
        }
    }

    #[test]
    fn an_edge_delimiter_character_is_still_escaped_after_the_whitespace_moves() {
        let node = RenderNode::paragraph(vec![
            RenderNode::text("a"),
            RenderNode::strong(vec![RenderNode::text(" *b")]),
        ]);
        for dialect in DIALECTS {
            let markdown = render_in(&node, dialect);
            assert_eq!(markdown, "a **\\*b**", "{dialect:?}");
            assert_eq!(read_back(&markdown), "a <strong>*b</strong>");
        }
    }

    #[test]
    fn mark_and_dim_delimiters_touch_their_text() {
        // Darkmatter reads `==`/`⌄` only when no whitespace follows the opener
        // or precedes the closer, and only within one line.
        for (token, delimiter) in [("mark", "=="), ("dim", "\u{2304}")] {
            for (edge, ..) in edges() {
                let leading = RenderNode::paragraph(vec![
                    RenderNode::text("a"),
                    RenderNode::extended(token, vec![edge.clone(), RenderNode::text("b")], None),
                ]);
                let trailing = RenderNode::paragraph(vec![
                    RenderNode::extended(token, vec![RenderNode::text("b"), edge.clone()], None),
                    RenderNode::text("c"),
                ]);
                for dialect in DIALECTS {
                    let between = RenderNode::paragraph(vec![
                        RenderNode::text("a"),
                        edge.clone(),
                        RenderNode::text("c"),
                    ]);
                    let between = render_in(&between, dialect);
                    let shown = &between[1..between.len() - 1];
                    assert_eq!(
                        render_in(&leading, dialect),
                        format!("a{shown}{delimiter}b{delimiter}"),
                        "{token} {edge:?} {dialect:?}"
                    );
                    assert_eq!(
                        render_in(&trailing, dialect),
                        format!("{delimiter}b{delimiter}{shown}c"),
                        "{token} {edge:?} {dialect:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn wrappers_without_edge_whitespace_are_byte_identical() {
        let cases = [
            (RenderNode::strong(vec![RenderNode::text("a")]), "**a**"),
            (RenderNode::emphasis(vec![RenderNode::text("a b")]), "_a b_"),
            (RenderNode::delete(vec![RenderNode::text("a")]), "~~a~~"),
            (
                RenderNode::strong(vec![
                    RenderNode::text("a"),
                    RenderNode::soft_break(),
                    RenderNode::text("b"),
                ]),
                "**a\nb**",
            ),
            (
                RenderNode::extended("mark", vec![RenderNode::text("a")], None),
                "==a==",
            ),
        ];
        for (node, expected) in cases {
            for dialect in DIALECTS {
                assert_eq!(
                    render_in(&RenderNode::paragraph(vec![node.clone()]), dialect),
                    expected
                );
            }
        }
    }

    /// A wrapper between outer neighbors where the usual delimiter cannot
    /// open or close, with the inline nodes around it and the expected
    /// read-back (`{tag}` stands for the wrapper's tag).
    fn flanking_cases(wrap: Wrap) -> Vec<(&'static str, Vec<RenderNode>, &'static str)> {
        let punct = || wrap(vec![RenderNode::text("(b)")]);
        vec![
            (
                "punctuation inside, letter outside the opener",
                vec![RenderNode::text("a"), punct(), RenderNode::text(" c")],
                "a<{tag}>(b)</{tag}> c",
            ),
            (
                "punctuation inside, letter outside the closer",
                vec![RenderNode::text("a "), punct(), RenderNode::text("c")],
                "a <{tag}>(b)</{tag}>c",
            ),
            (
                "punctuation inside, letters outside both",
                vec![RenderNode::text("a"), punct(), RenderNode::text("c")],
                "a<{tag}>(b)</{tag}>c",
            ),
            (
                "inline code inside, letters outside",
                vec![
                    RenderNode::text("a"),
                    wrap(vec![RenderNode::inline_code("b")]),
                    RenderNode::text("c"),
                ],
                "a<{tag}>b</{tag}>c",
            ),
            (
                "intraword",
                vec![
                    RenderNode::text("a"),
                    wrap(vec![RenderNode::text("b")]),
                    RenderNode::text("c"),
                ],
                "a<{tag}>b</{tag}>c",
            ),
            (
                "an unescaped `*` in the text outside",
                vec![
                    RenderNode::text("a*"),
                    wrap(vec![RenderNode::text("b")]),
                    RenderNode::text("*c"),
                ],
                "a*<{tag}>b</{tag}>*c",
            ),
            (
                "an unescaped `_` in the text outside",
                vec![
                    RenderNode::text("a_"),
                    wrap(vec![RenderNode::text("b")]),
                    RenderNode::text("_c"),
                ],
                "a_<{tag}>b</{tag}>_c",
            ),
            (
                "an unescaped `~` in the text outside",
                vec![
                    RenderNode::text("a~"),
                    wrap(vec![RenderNode::text("b")]),
                    RenderNode::text("~c"),
                ],
                "a~<{tag}>b</{tag}>~c",
            ),
            (
                "inside a span that writes no markup",
                vec![
                    RenderNode::text("a"),
                    RenderNode::span(Vec::new(), vec![punct()]),
                    RenderNode::text("c"),
                ],
                "a<{tag}>(b)</{tag}>c",
            ),
        ]
    }

    #[test]
    fn every_wrapper_reads_back_where_its_delimiter_cannot_flank() {
        for dialect in DIALECTS {
            for (wrap, tag) in wrappers() {
                for (label, inline, expected) in flanking_cases(wrap) {
                    for in_cell in [false, true] {
                        let (markdown, read) = render_and_read(inline.clone(), in_cell, dialect);
                        assert_eq!(
                            read,
                            expected.replace("{tag}", tag),
                            "{dialect:?} {tag} {label} in_cell={in_cell}: {markdown:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn flanking_fallbacks_use_the_first_spelling_that_reads_back() {
        let a = || RenderNode::text("a");
        let c = || RenderNode::text("c");
        let paren = || RenderNode::text("(b)");
        let b = || RenderNode::text("b");
        let cases = [
            // `*` opens and closes inside a word; `_` does not.
            (vec![a(), RenderNode::emphasis(vec![b()]), c()], "a*b*c"),
            (vec![a(), RenderNode::strong(vec![b()]), c()], "a**b**c"),
            (vec![a(), RenderNode::delete(vec![b()]), c()], "a~~b~~c"),
            // No delimiter flanks punctuation against a letter.
            (
                vec![a(), RenderNode::emphasis(vec![paren()]), c()],
                "a<em>(b)</em>c",
            ),
            (
                vec![a(), RenderNode::strong(vec![paren()]), c()],
                "a<strong>(b)</strong>c",
            ),
            (
                vec![a(), RenderNode::delete(vec![paren()]), c()],
                "a<del>(b)</del>c",
            ),
            // Text beside a wrapper escapes its delimiter characters, so the
            // usual run fits; a run beside an unescaped one would merge.
            (
                vec![RenderNode::text("x*"), RenderNode::strong(vec![b()])],
                "x\\***b**",
            ),
            (
                vec![RenderNode::text("x_"), RenderNode::emphasis(vec![b()])],
                "x\\__b_",
            ),
            (
                vec![RenderNode::strong(vec![a()]), RenderNode::strong(vec![b()])],
                "**a**__b__",
            ),
            // The outer wrapper sees its body's first and last characters.
            (
                vec![
                    a(),
                    RenderNode::emphasis(vec![RenderNode::strong(vec![b()])]),
                    c(),
                ],
                "a<em>**b**</em>c",
            ),
            (
                vec![RenderNode::strong(vec![
                    a(),
                    RenderNode::emphasis(vec![b()]),
                ])],
                "__a*b*__",
            ),
        ];
        for (inline, expected) in cases {
            for dialect in DIALECTS {
                let para = RenderNode::paragraph(inline.clone());
                let markdown = render_in(&para, dialect);
                assert_eq!(markdown, expected, "{dialect:?}");
                assert_eq!(read_back(&markdown), read_back(expected), "{dialect:?}");
            }
        }
    }

    #[test]
    fn nested_and_adjacent_wrappers_read_back_with_their_structure() {
        let cases = [
            (
                vec![
                    RenderNode::text("a"),
                    RenderNode::emphasis(vec![RenderNode::strong(vec![RenderNode::text("b")])]),
                    RenderNode::text("c"),
                ],
                "a<em><strong>b</strong></em>c",
            ),
            (
                vec![RenderNode::strong(vec![
                    RenderNode::text("a"),
                    RenderNode::emphasis(vec![RenderNode::text("b")]),
                ])],
                "<strong>a<em>b</em></strong>",
            ),
            (
                vec![
                    RenderNode::strong(vec![RenderNode::text("a")]),
                    RenderNode::strong(vec![RenderNode::text("b")]),
                ],
                "<strong>a</strong><strong>b</strong>",
            ),
            (
                vec![
                    RenderNode::text("a"),
                    RenderNode::emphasis(vec![RenderNode::text("b")]),
                    RenderNode::strong(vec![RenderNode::text("c")]),
                    RenderNode::text("d"),
                ],
                "a<em>b</em><strong>c</strong>d",
            ),
        ];
        for dialect in DIALECTS {
            for (inline, expected) in &cases {
                for in_cell in [false, true] {
                    let (markdown, read) = render_and_read(inline.clone(), in_cell, dialect);
                    assert_eq!(
                        read, *expected,
                        "{dialect:?} in_cell={in_cell}: {markdown:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn well_formed_delimiters_are_byte_identical() {
        let text = RenderNode::text;
        let cases = [
            (
                vec![
                    text("x "),
                    RenderNode::emphasis(vec![text("(b)")]),
                    text(" y"),
                ],
                "x _(b)_ y",
            ),
            (
                vec![text("x "), RenderNode::strong(vec![text("b")]), text(".")],
                "x **b**.",
            ),
            (
                vec![text("("), RenderNode::strong(vec![text("(b)")]), text(")")],
                "(**(b)**)",
            ),
            (
                vec![RenderNode::emphasis(vec![text("a")]), text(", b")],
                "_a_, b",
            ),
            (
                vec![text("x "), RenderNode::delete(vec![text("b")]), text(" y")],
                "x ~~b~~ y",
            ),
            (
                vec![RenderNode::strong(vec![RenderNode::emphasis(vec![text(
                    "a",
                )])])],
                "**_a_**",
            ),
            (
                vec![RenderNode::emphasis(vec![RenderNode::strong(vec![text(
                    "a",
                )])])],
                "_**a**_",
            ),
            (
                vec![RenderNode::link(
                    "u",
                    None,
                    vec![RenderNode::strong(vec![text("(a)")])],
                )],
                "[**(a)**](u)",
            ),
            (
                vec![RenderNode::strong(vec![text("é")]), text("x")],
                "**é**x",
            ),
        ];
        for (inline, expected) in cases {
            for dialect in DIALECTS {
                assert_eq!(
                    render_in(&RenderNode::paragraph(inline.clone()), dialect),
                    expected,
                    "{dialect:?}"
                );
            }
        }
    }

    #[test]
    fn mark_and_dim_fall_back_to_plain_text_where_darkmatter_cannot_read_them() {
        let dim = |text: &str| RenderNode::extended("dim", vec![RenderNode::text(text)], None);
        let mark = |text: &str| RenderNode::extended("mark", vec![RenderNode::text(text)], None);
        let a = || RenderNode::text("a");
        let c = || RenderNode::text("c");
        let readable = [
            (vec![a(), mark("(b)"), c()], "a==(b)==c"),
            (vec![a(), mark("b"), c()], "a==b==c"),
            (vec![a(), dim("(b)"), c()], "a\u{2304}(b)\u{2304}c"),
            (
                vec![RenderNode::text("a "), dim("b"), RenderNode::text(".")],
                "a \u{2304}b\u{2304}.",
            ),
        ];
        let unreadable = [
            // `⌄` never opens or closes between two alphanumerics.
            (vec![a(), dim("b"), c()], "abc"),
            // `==` pairs from the left, so a preceding `=`, escaped or not,
            // steals the opener.
            (vec![RenderNode::text("a="), mark("b")], "a\\=b"),
        ];
        for dialect in DIALECTS {
            for (inline, expected) in &readable {
                let para = RenderNode::paragraph(inline.clone());
                let rendered = render_markdown_node(&para, &opts(dialect, RenderStrictness::Warn))
                    .expect("render");
                assert_eq!(rendered.output, *expected, "{dialect:?}");
                assert!(rendered.diagnostics.is_empty(), "{dialect:?}");
            }
            for (inline, expected) in &unreadable {
                let para = RenderNode::paragraph(inline.clone());
                let warn = render_markdown_node(&para, &opts(dialect, RenderStrictness::Warn))
                    .expect("render");
                assert_eq!(warn.output, *expected, "{dialect:?}");
                assert_eq!(warn.diagnostics.len(), 1, "{dialect:?}");
                assert!(matches!(
                    render_markdown_node(&para, &opts(dialect, RenderStrictness::Strict)),
                    Err(RenderError::LossyRejected { .. })
                ));
            }
        }
    }

    fn opts(dialect: MarkdownDialect, strictness: RenderStrictness) -> MarkdownRenderOptions {
        MarkdownRenderOptions {
            dialect,
            strictness,
            style: None,
        }
    }
}
