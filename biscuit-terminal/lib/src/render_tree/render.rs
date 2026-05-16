//! Terminal renderer for the canonical render tree.
//!
//! [`render_terminal_node`] folds a [`RenderNode`] into a terminal string,
//! and [`render_terminal_document`] does the same for a whole [`Document`].
//!
//! The renderer reuses biscuit-terminal's existing components wherever they
//! fit — [`Prose`] for inline runs, [`OrderedList`]/[`UnorderedList`],
//! [`Table`], [`BlockQuote`], [`HorizontalRule`], and [`Section`] for
//! headings — rather than re-implementing terminal formatting. Inline
//! subtrees are projected into [`Prose`] block-tag markup so color, styling,
//! and OSC8 hyperlinks are handled by the established component.
//!
//! ## Strictness
//!
//! The node is validated with [`validate`] first; an error-severity finding
//! produces an immediate [`RenderError::InvalidTree`] regardless of
//! strictness. Warning-severity findings fold into [`Rendered::diagnostics`]
//! under [`RenderStrictness::Warn`]/[`RenderStrictness::Lossy`] and escalate
//! to an error under [`RenderStrictness::Strict`].
//!
//! ## Examples
//!
//! ```
//! use renderable::tree::{HeadingDepth, RenderNode};
//! use biscuit_terminal::render_tree::{render_terminal_node, TerminalRenderOptions};
//!
//! let tree = RenderNode::root(vec![RenderNode::paragraph(vec![
//!     RenderNode::text("Hello"),
//! ])]);
//! let rendered = render_terminal_node(&tree, &TerminalRenderOptions::default()).unwrap();
//! assert!(rendered.output.contains("Hello"));
//! ```

use renderable::tree::{
    ColumnAlign, Diagnostic, Document, NodeKind, RenderError, RenderNode, RenderStrictness,
    Rendered, Severity,
};
use renderable::tree::{validate, ValidationError, ValidationMode};

use crate::components::block_quote::BlockQuote;
use crate::components::horizontal_rule::HorizontalRule;
use crate::components::list::{OrderedList, UnorderedList};
use crate::components::prose::Prose;
use crate::components::renderable::TerminalRenderable;
use crate::components::section::{HeadingLevel, Section};
use crate::components::table::{Table, TableCellContent, TableColumn};
use crate::utils::layout::Alignment;

use super::options::TerminalRenderOptions;

/// Renders a render-tree node to a terminal string.
///
/// See the [`render_tree`](crate::render_tree) module docs for the
/// validation gate and strictness model.
///
/// ## Returns
///
/// A [`Rendered<String>`] carrying the terminal output and any non-fatal
/// diagnostics.
///
/// ## Errors
///
/// - [`RenderError::InvalidTree`] if the tree fails structural validation, or
///   if [`RenderStrictness::Strict`] meets a warning-severity validation
///   finding (this includes [`NodeKind::Unsupported`] nodes, whose warning is
///   escalated by the validation gate before the writer runs).
/// - [`RenderError::Unsupported`] / [`RenderError::LossyRejected`] if
///   [`RenderStrictness::Strict`] meets content that cannot be rendered
///   faithfully to a terminal (an unsupported node or raw HTML).
pub fn render_terminal_node(
    node: &RenderNode,
    opts: &TerminalRenderOptions,
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
    // and otherwise fold into the renderer diagnostics.
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

/// Renders a whole [`Document`] to a terminal string.
///
/// The body is produced by [`render_terminal_node`] on [`Document::root`].
/// Frontmatter is a source-text concern and is intentionally not rendered to
/// the terminal; the `darkmatter` fold does not populate it for this phase.
///
/// ## Errors
///
/// Propagates every error from [`render_terminal_node`].
pub fn render_terminal_document(
    doc: &Document,
    opts: &TerminalRenderOptions,
) -> Result<Rendered<String>, RenderError> {
    render_terminal_node(&doc.root, opts)
}

/// Threads render options and accumulating diagnostics through the recursion.
struct Writer<'a> {
    opts: &'a TerminalRenderOptions,
    diagnostics: Vec<Diagnostic>,
}

impl Writer<'_> {
    /// Renders a single block-level node and its subtree.
    fn render(&mut self, node: &RenderNode) -> Result<String, RenderError> {
        match &node.kind {
            NodeKind::Root { children } => self.render_blocks(children),
            NodeKind::Heading { depth, children } => {
                let level = match depth.get() {
                    1 => HeadingLevel::h1,
                    2 => HeadingLevel::h2,
                    3 => HeadingLevel::h3,
                    4 => HeadingLevel::h4,
                    5 => HeadingLevel::h5,
                    _ => HeadingLevel::h6,
                };
                let markup = self.render_inline(children)?;
                let section = Section::new(level, markup);
                Ok(section.render(&self.opts.context.terminal))
            }
            NodeKind::Paragraph { children } => {
                let markup = self.render_inline(children)?;
                Ok(self.render_prose(&markup))
            }
            NodeKind::BlockQuote { children } => {
                let inner = self.render_blocks(children)?;
                let quote = BlockQuote::from(inner.as_str());
                Ok(quote.render(&self.opts.context.terminal))
            }
            NodeKind::List { ordered, start, children } => {
                self.render_list(*ordered, *start, children)
            }
            NodeKind::ListItem { checked, children } => {
                let body = self.render_blocks(children)?;
                Ok(match checked {
                    Some(true) => format!("[x] {body}"),
                    Some(false) => format!("[ ] {body}"),
                    None => body,
                })
            }
            // `meta` (info-string beyond the language) is intentionally
            // ignored: syntax highlighting and meta handling are out of
            // scope for this phase.
            NodeKind::Code { lang, meta: _, value } => Ok(self.render_code(lang.as_deref(), value)),
            NodeKind::ThematicBreak => {
                let rule = HorizontalRule::new();
                Ok(rule.render(&self.opts.context.terminal))
            }
            NodeKind::Table { align, children } => self.render_table(align, children),
            NodeKind::TableRow { children } => {
                // A row rendered on its own (outside a Table) degrades to a
                // tab-joined line of its cells.
                let mut cells = Vec::with_capacity(children.len());
                for child in children {
                    cells.push(self.render(child)?);
                }
                Ok(cells.join("\t"))
            }
            NodeKind::TableCell { children } => {
                let markup = self.render_inline(children)?;
                Ok(self.render_prose(&markup))
            }
            NodeKind::FootnoteDefinition { identifier, children } => {
                let body = self.render_blocks(children)?;
                Ok(format!("[^{identifier}]: {body}"))
            }
            // Inline kinds rendered as a standalone block: project the inline
            // subtree into Prose markup and render it.
            NodeKind::Text { .. }
            | NodeKind::Emphasis { .. }
            | NodeKind::Strong { .. }
            | NodeKind::Delete { .. }
            | NodeKind::Span { .. }
            | NodeKind::InlineCode { .. }
            | NodeKind::Link { .. }
            | NodeKind::Image { .. }
            | NodeKind::FootnoteReference { .. }
            | NodeKind::SoftBreak
            | NodeKind::HardBreak => {
                let markup = self.render_inline(std::slice::from_ref(node))?;
                Ok(self.render_prose(&markup))
            }
            NodeKind::Html { value, block } => self.render_html(node, value, *block),
            NodeKind::Unsupported { label } => self.render_unsupported(node, label),
        }
    }

    /// Renders a sequence of block-level nodes, joined by a blank line.
    fn render_blocks(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
        let mut parts = Vec::with_capacity(children.len());
        for child in children {
            parts.push(self.render(child)?);
        }
        Ok(parts.join("\n\n"))
    }

    /// Renders a sequence of inline nodes into [`Prose`] block-tag markup.
    ///
    /// Literal text is escaped with [`Prose::escape_text`] so document text
    /// never accidentally triggers Prose markup; styling wraps the escaped
    /// runs in block tags the [`Prose`] parser already understands.
    fn render_inline(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
        let mut output = String::new();
        for child in children {
            output.push_str(&self.render_inline_node(child)?);
        }
        Ok(output)
    }

    /// Projects a single inline node into [`Prose`] markup.
    fn render_inline_node(&mut self, node: &RenderNode) -> Result<String, RenderError> {
        match &node.kind {
            NodeKind::Text { value } => Ok(apply_classes(
                &Prose::escape_text(value),
                &node.attrs.classes,
            )),
            NodeKind::Emphasis { children } => {
                let inner = self.render_inline(children)?;
                Ok(apply_classes(
                    &format!("<italic>{inner}</italic>"),
                    &node.attrs.classes,
                ))
            }
            NodeKind::Strong { children } => {
                let inner = self.render_inline(children)?;
                Ok(apply_classes(
                    &format!("<bold>{inner}</bold>"),
                    &node.attrs.classes,
                ))
            }
            NodeKind::Delete { children } => {
                let inner = self.render_inline(children)?;
                Ok(apply_classes(
                    &format!("<strikethrough>{inner}</strikethrough>"),
                    &node.attrs.classes,
                ))
            }
            NodeKind::Span { children } => {
                let inner = self.render_inline(children)?;
                self.render_span_classes(node, &inner)
            }
            NodeKind::InlineCode { value } => Ok(apply_classes(
                &format!("<dim>{}</dim>", Prose::escape_text(value)),
                &node.attrs.classes,
            )),
            NodeKind::Link { url, title: _, children } => {
                let inner = self.render_inline(children)?;
                Ok(format!(
                    "<a href=\"{}\">{inner}</a>",
                    Prose::escape_text(url)
                ))
            }
            NodeKind::Image { alt, .. } => {
                // Terminal inline images are out of scope for this phase
                // (visual components keep bespoke renderers). The alt text
                // stands in for the image.
                self.diagnostics.push(Diagnostic::lossy(
                    "image rendered as alt text; inline terminal images are out of scope",
                    Some(node.span.clone()),
                ));
                Ok(format!("[{}]", Prose::escape_text(alt)))
            }
            NodeKind::FootnoteReference { identifier } => {
                Ok(format!("[^{}]", Prose::escape_text(identifier)))
            }
            NodeKind::SoftBreak => Ok(" ".to_string()),
            NodeKind::HardBreak => Ok("\n".to_string()),
            // A non-inline node appearing in an inline position is a
            // structural problem the validator would have rejected; treat it
            // defensively by rendering it as a block. Enumerated explicitly
            // so a future inline NodeKind variant forces a deliberate choice.
            NodeKind::Root { .. }
            | NodeKind::Heading { .. }
            | NodeKind::Paragraph { .. }
            | NodeKind::BlockQuote { .. }
            | NodeKind::List { .. }
            | NodeKind::ListItem { .. }
            | NodeKind::Code { .. }
            | NodeKind::ThematicBreak
            | NodeKind::Table { .. }
            | NodeKind::TableRow { .. }
            | NodeKind::TableCell { .. }
            | NodeKind::FootnoteDefinition { .. }
            | NodeKind::Html { .. }
            | NodeKind::Unsupported { .. } => self.render(node),
        }
    }

    /// Renders [`Prose`] markup to a styled terminal string.
    fn render_prose(&self, markup: &str) -> String {
        Prose::new(markup).render(&self.opts.context.terminal)
    }

    /// Renders a list via the [`OrderedList`]/[`UnorderedList`] components.
    ///
    /// [`OrderedList`] numbers from 1. When the tree requests a different
    /// origin, the items are numbered explicitly instead.
    fn render_list(
        &mut self,
        ordered: bool,
        start: Option<u64>,
        children: &[RenderNode],
    ) -> Result<String, RenderError> {
        let mut items = Vec::with_capacity(children.len());
        for child in children {
            items.push(self.render(child)?);
        }

        if ordered {
            let origin = start.unwrap_or(1);
            if origin != 1 {
                return Ok(self.render_ordered_from(origin, &items));
            }
            let list = OrderedList::from(items);
            Ok(list.render(&self.opts.context.terminal))
        } else {
            let list = UnorderedList::from(items);
            Ok(list.render(&self.opts.context.terminal))
        }
    }

    /// Renders an ordered list whose numbering starts at `start`.
    fn render_ordered_from(&self, start: u64, items: &[String]) -> String {
        let mut lines = Vec::with_capacity(items.len());
        for (offset, item) in items.iter().enumerate() {
            let index = start + offset as u64;
            lines.push(format!("{index}. {item}"));
        }
        lines.join("\n")
    }

    /// Renders a code block as a dim, indented panel.
    ///
    /// Syntax highlighting is out of scope for this phase; the language tag
    /// is shown as a header so the lossy projection is visible.
    fn render_code(&self, lang: Option<&str>, value: &str) -> String {
        let body = value.trim_end_matches('\n');
        let header = lang
            .filter(|l| !l.is_empty())
            .map(|l| format!("<dim>```{l}</dim>\n"))
            .unwrap_or_default();
        let escaped: String = body
            .lines()
            .map(|line| format!("    {}", Prose::escape_text(line)))
            .collect::<Vec<_>>()
            .join("\n");
        let markup = format!("{header}<dim>{escaped}</dim>");
        self.render_prose(&markup)
    }

    /// Renders a table via the [`Table`] component.
    ///
    /// The first child row is treated as the header row.
    fn render_table(
        &mut self,
        align: &[ColumnAlign],
        children: &[RenderNode],
    ) -> Result<String, RenderError> {
        let mut rows = children.iter();
        let Some(header) = rows.next() else {
            return Ok(String::new());
        };

        let header_cells = self.table_row_cells(header)?;
        let columns: Vec<TableColumn> = header_cells
            .iter()
            .enumerate()
            .map(|(idx, text)| {
                let mut col = TableColumn::new(text.clone());
                if let Some(a) = align.get(idx) {
                    col = col.with_alignment(column_alignment(*a));
                }
                col
            })
            .collect();

        let mut data: Vec<Vec<TableCellContent>> = Vec::new();
        for row in rows {
            let cells = self.table_row_cells(row)?;
            data.push(cells.into_iter().map(TableCellContent::from).collect());
        }

        let table = Table::new().with_columns(columns).with_data(data);
        Ok(table.render(&self.opts.context.terminal))
    }

    /// Extracts the plain-text cells of a table row.
    fn table_row_cells(&mut self, row: &RenderNode) -> Result<Vec<String>, RenderError> {
        let NodeKind::TableRow { children } = &row.kind else {
            // The validator rejects malformed tables before this runs.
            return self.render(row).map(|s| vec![s]);
        };
        let mut cells = Vec::with_capacity(children.len());
        for cell in children {
            let NodeKind::TableCell { children } = &cell.kind else {
                cells.push(self.render(cell)?);
                continue;
            };
            cells.push(self.render_inline(children)?);
        }
        Ok(cells)
    }

    /// Maps semantic span classes to Prose styling.
    ///
    /// The documented class vocabulary — `mark`, `dim`, `sup`, `sub` — maps
    /// to visual treatment; unknown classes are ignored, with a diagnostic
    /// emitted only under [`RenderStrictness::Warn`].
    fn render_span_classes(
        &mut self,
        node: &RenderNode,
        inner: &str,
    ) -> Result<String, RenderError> {
        let styled = apply_classes(inner, &node.attrs.classes);
        for class in &node.attrs.classes {
            if !is_known_class(class)
                && self.opts.strictness == RenderStrictness::Warn
            {
                self.diagnostics.push(Diagnostic::lossy(
                    format!("span class '{class}' has no terminal treatment"),
                    Some(node.span.clone()),
                ));
            }
        }
        Ok(styled)
    }

    /// Renders raw HTML according to strictness.
    ///
    /// A terminal cannot interpret HTML. Under [`RenderStrictness::Strict`]
    /// this is a [`RenderError::LossyRejected`]; under
    /// [`RenderStrictness::Warn`] the raw value is emitted verbatim with a
    /// diagnostic; under [`RenderStrictness::Lossy`] the node is dropped.
    fn render_html(
        &mut self,
        node: &RenderNode,
        value: &str,
        _block: bool,
    ) -> Result<String, RenderError> {
        match self.opts.strictness {
            RenderStrictness::Strict => Err(RenderError::LossyRejected {
                message: "raw HTML cannot be rendered to a terminal".to_string(),
            }),
            RenderStrictness::Warn => {
                self.diagnostics.push(Diagnostic::lossy(
                    "raw HTML emitted verbatim; terminals cannot interpret HTML",
                    Some(node.span.clone()),
                ));
                Ok(value.to_string())
            }
            RenderStrictness::Lossy => Ok(String::new()),
        }
    }

    /// Renders an unsupported node according to strictness.
    fn render_unsupported(
        &mut self,
        node: &RenderNode,
        label: &str,
    ) -> Result<String, RenderError> {
        match self.opts.strictness {
            // Defensive fallback: for trees entered via `render_terminal_node`
            // this arm is preempted because the validation gate escalates the
            // `Unsupported` warning to an error first.
            RenderStrictness::Strict => Err(RenderError::Unsupported {
                label: label.to_string(),
            }),
            RenderStrictness::Warn => {
                self.diagnostics.push(Diagnostic::unsupported(
                    format!("unsupported content dropped: {label}"),
                    Some(node.span.clone()),
                ));
                Ok(self.render_prose(&format!(
                    "<dim>[unsupported: {}]</dim>",
                    Prose::escape_text(label)
                )))
            }
            RenderStrictness::Lossy => Ok(String::new()),
        }
    }
}

/// Returns `true` for a semantic class with a defined terminal treatment.
fn is_known_class(class: &str) -> bool {
    matches!(class, "mark" | "dim" | "sup" | "sub")
}

/// Wraps `inner` in Prose markup for any recognized semantic classes.
///
/// `mark` highlights via reverse video, `dim` dims, and `sup`/`sub` are
/// approximated as dim text since terminals lack super/subscript.
fn apply_classes(inner: &str, classes: &[String]) -> String {
    let mut out = inner.to_string();
    for class in classes {
        out = match class.as_str() {
            "mark" => format!("<reverse>{out}</reverse>"),
            "dim" | "sup" | "sub" => format!("<dim>{out}</dim>"),
            _ => out,
        };
    }
    out
}

/// Maps a render-tree column alignment to a layout alignment.
fn column_alignment(align: ColumnAlign) -> Alignment {
    match align {
        ColumnAlign::Left | ColumnAlign::None => Alignment::Left,
        ColumnAlign::Center => Alignment::Center,
        ColumnAlign::Right => Alignment::Right,
    }
}

#[cfg(test)]
mod render_tree_tests {
    use super::*;
    use renderable::tree::HeadingDepth;

    use crate::terminal::Terminal;
    use crate::utils::escape_codes::strip_escape_codes;

    fn opts(strictness: RenderStrictness) -> TerminalRenderOptions {
        TerminalRenderOptions::new(&Terminal::new_optimistic(80), strictness)
    }

    fn render(node: &RenderNode) -> Rendered<String> {
        render_terminal_node(node, &opts(RenderStrictness::Warn)).expect("render")
    }

    #[test]
    fn render_tree_paragraph_renders_text() {
        let node = RenderNode::paragraph(vec![RenderNode::text("hello world")]);
        let out = render(&node);
        assert!(strip_escape_codes(&out.output).contains("hello world"));
    }

    #[test]
    fn render_tree_heading_renders_with_section() {
        let node = RenderNode::heading(
            HeadingDepth::new(2).unwrap(),
            vec![RenderNode::text("Title")],
        );
        let out = render(&node);
        assert!(out.output.contains("Title"));
    }

    #[test]
    fn render_tree_inline_styles_emit_escape_codes() {
        let node = RenderNode::paragraph(vec![
            RenderNode::strong(vec![RenderNode::text("bold")]),
            RenderNode::text(" "),
            RenderNode::emphasis(vec![RenderNode::text("em")]),
            RenderNode::text(" "),
            RenderNode::delete(vec![RenderNode::text("del")]),
        ]);
        let out = render(&node);
        // Bold, italic, and strikethrough SGR sequences.
        assert!(out.output.contains("\x1b[1m"));
        assert!(out.output.contains("\x1b[3m"));
        assert!(out.output.contains("\x1b[9m"));
        let plain = strip_escape_codes(&out.output);
        assert!(plain.contains("bold"));
        assert!(plain.contains("em"));
        assert!(plain.contains("del"));
    }

    #[test]
    fn render_tree_unordered_list_renders_items() {
        let list = RenderNode::list(
            false,
            None,
            vec![
                RenderNode::list_item(
                    None,
                    vec![RenderNode::paragraph(vec![RenderNode::text("first")])],
                ),
                RenderNode::list_item(
                    None,
                    vec![RenderNode::paragraph(vec![RenderNode::text("second")])],
                ),
            ],
        );
        let out = render(&list);
        let plain = strip_escape_codes(&out.output);
        assert!(plain.contains("first"));
        assert!(plain.contains("second"));
    }

    #[test]
    fn render_tree_ordered_list_renders_numbers() {
        let list = RenderNode::list(
            true,
            None,
            vec![
                RenderNode::list_item(
                    None,
                    vec![RenderNode::paragraph(vec![RenderNode::text("alpha")])],
                ),
                RenderNode::list_item(
                    None,
                    vec![RenderNode::paragraph(vec![RenderNode::text("beta")])],
                ),
            ],
        );
        let out = render(&list);
        let plain = strip_escape_codes(&out.output);
        assert!(plain.contains("1."));
        assert!(plain.contains("alpha"));
        assert!(plain.contains("beta"));
    }

    #[test]
    fn render_tree_ordered_list_honors_start() {
        let list = RenderNode::list(
            true,
            Some(5),
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
        let out = render(&list);
        let plain = strip_escape_codes(&out.output);
        assert!(plain.contains("5."));
        assert!(plain.contains("6."));
    }

    #[test]
    fn render_tree_code_block_renders() {
        let node = RenderNode::code(Some("rust".into()), None, "let a = 1;");
        let out = render(&node);
        let plain = strip_escape_codes(&out.output);
        assert!(plain.contains("let a = 1;"));
        assert!(plain.contains("rust"));
    }

    #[test]
    fn render_tree_link_emits_osc8_when_hyperlinks_supported() {
        let node = RenderNode::paragraph(vec![RenderNode::link(
            "https://example.com",
            None,
            vec![RenderNode::text("site")],
        )]);
        // optimistic terminal advertises osc_link_support.
        let out = render(&node);
        assert!(out.output.contains("\x1b]8;;https://example.com"));
        assert!(strip_escape_codes(&out.output).contains("site"));
    }

    #[test]
    fn render_tree_link_drops_osc8_without_hyperlink_support() {
        let term = Terminal::builder()
            .width(80)
            .osc_link_support(false)
            .build();
        let node = RenderNode::paragraph(vec![RenderNode::link(
            "https://example.com",
            None,
            vec![RenderNode::text("site")],
        )]);
        let out = render_terminal_node(
            &node,
            &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
        )
        .expect("render");
        assert!(!out.output.contains("\x1b]8;;"));
        assert!(strip_escape_codes(&out.output).contains("site"));
    }

    #[test]
    fn render_tree_table_uses_header_row() {
        let table = RenderNode::table(
            vec![ColumnAlign::Left, ColumnAlign::Right],
            vec![
                RenderNode::table_row(vec![
                    RenderNode::table_cell(vec![RenderNode::text("Name")]),
                    RenderNode::table_cell(vec![RenderNode::text("Score")]),
                ]),
                RenderNode::table_row(vec![
                    RenderNode::table_cell(vec![RenderNode::text("Ann")]),
                    RenderNode::table_cell(vec![RenderNode::text("42")]),
                ]),
            ],
        );
        let out = render(&table);
        let plain = strip_escape_codes(&out.output);
        assert!(plain.contains("Name"));
        assert!(plain.contains("Score"));
        assert!(plain.contains("Ann"));
        assert!(plain.contains("42"));
    }

    #[test]
    fn render_tree_block_quote_renders_border() {
        let node = RenderNode::block_quote(vec![RenderNode::paragraph(vec![
            RenderNode::text("quoted"),
        ])]);
        let out = render(&node);
        assert!(out.output.contains('│'));
        assert!(strip_escape_codes(&out.output).contains("quoted"));
    }

    #[test]
    fn render_tree_thematic_break_renders_rule() {
        let out = render(&RenderNode::thematic_break());
        assert!(!out.output.trim().is_empty());
    }

    #[test]
    fn render_tree_mark_class_emits_reverse_video() {
        let node = RenderNode::paragraph(vec![RenderNode::span(
            vec!["mark".into()],
            vec![RenderNode::text("highlighted")],
        )]);
        let out = render(&node);
        // reverse-video SGR.
        assert!(out.output.contains("\x1b[7m"));
    }

    #[test]
    fn render_tree_unknown_class_emits_diagnostic_under_warn() {
        let node = RenderNode::paragraph(vec![RenderNode::span(
            vec!["mystery".into()],
            vec![RenderNode::text("text")],
        )]);
        let out = render(&node);
        assert!(out
            .diagnostics
            .iter()
            .any(|d| d.message.contains("mystery")));
    }

    #[test]
    fn render_tree_html_errors_under_strict() {
        let node = RenderNode::root(vec![RenderNode::html("<div>x</div>", true)]);
        let result = render_terminal_node(&node, &opts(RenderStrictness::Strict));
        assert!(matches!(result, Err(RenderError::LossyRejected { .. })));
    }

    #[test]
    fn render_tree_html_emits_diagnostic_under_warn() {
        let node = RenderNode::root(vec![RenderNode::html("<br>", false)]);
        let out = render(&node);
        assert_eq!(out.output, "<br>");
        assert!(out.diagnostics.iter().any(|d| d.message.contains("HTML")));
    }

    #[test]
    fn render_tree_unsupported_errors_under_strict() {
        let node = RenderNode::root(vec![RenderNode::unsupported("widget")]);
        let result = render_terminal_node(&node, &opts(RenderStrictness::Strict));
        // The validation gate escalates the Unsupported warning first.
        assert!(matches!(result, Err(RenderError::InvalidTree { .. })));
    }

    #[test]
    fn render_tree_unsupported_emits_diagnostic_under_warn() {
        let node = RenderNode::root(vec![RenderNode::unsupported("widget")]);
        let out = render(&node);
        assert!(strip_escape_codes(&out.output).contains("[unsupported: widget]"));
        // One validation-warning diagnostic plus one renderer diagnostic.
        assert!(out.diagnostics.len() >= 2);
    }

    #[test]
    fn render_tree_image_renders_alt_text_with_diagnostic() {
        let node = RenderNode::paragraph(vec![RenderNode::image(
            "pic.png",
            None,
            "a cat",
        )]);
        let out = render(&node);
        assert!(strip_escape_codes(&out.output).contains("[a cat]"));
        assert!(out.diagnostics.iter().any(|d| d.message.contains("image")));
    }

    #[test]
    fn render_tree_invalid_tree_errors_before_output() {
        // An orphaned TableCell inside a Paragraph is a structural error.
        let bad = RenderNode::root(vec![RenderNode::paragraph(vec![
            RenderNode::table_cell(vec![RenderNode::text("x")]),
        ])]);
        for strictness in [
            RenderStrictness::Strict,
            RenderStrictness::Warn,
            RenderStrictness::Lossy,
        ] {
            let result = render_terminal_node(&bad, &opts(strictness));
            assert!(matches!(result, Err(RenderError::InvalidTree { .. })));
        }
    }

    #[test]
    fn render_tree_document_renders_root() {
        use renderable::tree::{DocumentMetadata, SourceRegistry};

        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text(
                "body",
            )])]),
        };
        let out = render_terminal_document(&doc, &opts(RenderStrictness::Warn)).expect("render");
        assert!(strip_escape_codes(&out.output).contains("body"));
    }
}
