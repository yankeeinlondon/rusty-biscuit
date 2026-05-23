//! Terminal renderer for the canonical render tree.
//!
//! [`render_terminal_node`] folds a [`RenderNode`] into a terminal string,
//! and [`render_terminal_document`] does the same for a whole [`Document`].
//!
//! The renderer reuses biscuit-terminal's existing components wherever they
//! fit — [`Prose`] for inline runs, [`Table`], [`BlockQuote`], and
//! [`HorizontalRule`] — rather than re-implementing terminal formatting.
//! Headings, sections, and lists are rendered natively. Inline subtrees are
//! projected into [`Prose`] block-tag markup so color, styling, and OSC8
//! hyperlinks are handled by the established component.
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

use renderable::color::TerminalCodeContext;
use renderable::style::Style;
use renderable::tree::{
    ColumnAlign, ColumnConditional, Diagnostic, Document, HintNamespace, NodeKind, ProgressHints,
    RenderError, RenderNode, RenderStrictness, Rendered, Severity, TableColumnHints,
};
use renderable::tree::{ValidationError, ValidationMode, validate};

use crate::components::block_quote::BlockQuote;
use crate::components::horizontal_rule::{HorizontalRule, RuleAlignment, RuleStyle, RuleWeight};
use crate::components::prose::Prose;
use crate::components::renderable::TerminalRenderable;
use crate::components::table::cell::pad_cell;
use crate::components::table::table::{
    BG_RESET, FG_RESET, apply_vertical_padding, build_border, stripe_bg_escape, stripe_fg_escape,
    wrap_cell_content,
};
use crate::components::table::{
    ColumnType, Conditional, Table, TableCellContent, TableColumn, TableWidthPlan, VerticalAlign,
};
use crate::discovery::detection::ColorDepth;
use crate::utils::block_constraint::{split_lines, visible_width, wrap_lines};
use crate::utils::layout::{Alignment, WordWrap};

use super::options::TerminalRenderOptions;
use super::style;

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
        effective: Style::default(),
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
    /// The text appearance (`color`, `emphasis`) inherited from styled
    /// ancestor blocks. Per Spec B D6 only these fields inherit; the
    /// box-painting fields (`background`, `border`, `fill`) never do and are
    /// kept cleared here. A styled node folds its own text appearance into
    /// this before rendering its subtree (see [`Writer::render_styled`]) so
    /// descendant paragraphs and inline spans see the ancestor color/emphasis.
    effective: Style,
}

impl Writer<'_> {
    /// Renders a single block-level node and its subtree.
    ///
    /// When the node carries a [`Layout`](renderable::layout::Layout), its
    /// margins and alignment are applied around the rendered content: the
    /// child width is reduced by the horizontal margins, each produced line is
    /// prefixed by the left margin (plus any alignment offset), and the
    /// vertical margins emit leading/trailing blank lines.
    fn render(&mut self, node: &RenderNode) -> Result<String, RenderError> {
        match node.attrs.layout() {
            Some(layout) if !is_inline_kind(&node.kind) => self.render_with_layout(node, &layout),
            _ => self.render_styled(node),
        }
    }

    /// Renders a node's content and applies its [`Style`], if any.
    ///
    /// The text-appearance, fill, and border layers are lowered by the
    /// `style` module — the same fold step where [`Layout`] is applied. When
    /// the style declares a border, the content is rendered within a width
    /// reduced by the border's horizontal overhead so the bordered block stays
    /// within the available width.
    ///
    /// [`Style`]: renderable::style::Style
    /// [`Layout`]: renderable::layout::Layout
    fn render_styled(&mut self, node: &RenderNode) -> Result<String, RenderError> {
        let Some(style) = node.attrs.style().filter(|s| !s.is_empty()) else {
            return self.render_kind(node);
        };

        let overhead = style::border_horizontal_overhead(&style);
        let inner_width = self
            .opts
            .context
            .available_width
            .saturating_sub(overhead)
            .max(1);

        // Fold this node's text appearance into the inherited `effective`
        // style for the duration of its subtree, so descendant paragraphs and
        // inline spans see the ancestor color/emphasis (Spec B D6). Only the
        // inheriting fields are carried — the box-painting layers are cleared.
        let prev_effective = std::mem::take(&mut self.effective);
        let merged = style.inherited_from(&prev_effective);
        self.effective = Style {
            color: merged.color,
            emphasis: merged.emphasis,
            ..Style::default()
        };

        let content = if overhead > 0 {
            self.render_kind_in_width(node, inner_width)
        } else {
            self.render_kind(node)
        };

        self.effective = prev_effective;
        let content = content?;

        Ok(style::apply_style(
            &content,
            &style,
            &self.opts.context.terminal,
            inner_width,
        ))
    }

    /// Renders a single node by kind within a constrained width.
    ///
    /// Mirrors [`Self::render_blocks_in_width`] for a single node: the
    /// renderer's context is temporarily narrowed so the node's content wraps
    /// to `width`, then the sub-render's diagnostics are merged back.
    fn render_kind_in_width(
        &mut self,
        node: &RenderNode,
        width: u32,
    ) -> Result<String, RenderError> {
        let mut narrowed = self.opts.clone();
        narrowed.context.available_width = width;
        narrowed.context.width = width;
        narrowed.context.terminal.fixed_width = Some(width);
        let mut sub = Writer {
            opts: &narrowed,
            diagnostics: Vec::new(),
            effective: self.effective.clone(),
        };
        let result = sub.render_kind(node);
        self.diagnostics.append(&mut sub.diagnostics);
        result
    }

    /// Renders a node's content within the constraints of a [`Layout`].
    ///
    /// The horizontal margins reduce the available render width; the rendered
    /// content is then padded by the left margin and an alignment offset, and
    /// the vertical margins are emitted as blank lines.
    fn render_with_layout(
        &mut self,
        node: &RenderNode,
        layout: &renderable::layout::Layout,
    ) -> Result<String, RenderError> {
        let available = self.opts.context.available_width;
        let left = resolve_cells(&layout.margin.left, available);
        let right = resolve_cells(&layout.margin.right, available);
        let top = resolve_cells(&layout.margin.top, available);
        let bottom = resolve_cells(&layout.margin.bottom, available);

        // Render the content within the width left after horizontal margins.
        // Clamp to at least 1: a width-0 sub-render is degenerate for the
        // downstream components (matching `render_columns`'s `.max(1)`).
        let mut content_width = available.saturating_sub(left + right).max(1);

        // Apply max_width cap if declared. The resolved cap further narrows
        // the available width so children receive a reduced inner width and
        // alignment is observable within the capped region.
        if let Some(mw) = &layout.max_width {
            let cap = resolve_cells(mw, available);
            if cap > 0 {
                content_width = content_width.min(cap);
            }
        }
        let content = {
            let mut narrowed = self.opts.clone();
            narrowed.context.available_width = content_width;
            narrowed.context.width = content_width;
            narrowed.context.terminal.fixed_width = Some(content_width);
            let mut sub = Writer {
                opts: &narrowed,
                diagnostics: Vec::new(),
                effective: self.effective.clone(),
            };
            let rendered = sub.render_styled(node);
            self.diagnostics.append(&mut sub.diagnostics);
            rendered?
        };

        // Alignment offset: extra left padding when the content is narrower
        // than the space available between the horizontal margins.
        let widest = content.split('\n').map(visible_width).max().unwrap_or(0);
        let slack = content_width.saturating_sub(widest);
        let align_offset = match layout.alignment {
            Alignment::Left => 0,
            Alignment::Center => slack / 2,
            Alignment::Right => slack,
        };

        let lead = " ".repeat((left + align_offset) as usize);
        let mut out = String::new();
        for _ in 0..top {
            out.push('\n');
        }
        for (idx, line) in content.split('\n').enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            // Keep blank lines blank: prefixing an empty line would produce
            // a line of trailing whitespace.
            if !line.is_empty() {
                out.push_str(&lead);
                out.push_str(line);
            }
        }
        for _ in 0..bottom {
            out.push('\n');
        }
        Ok(out)
    }

    /// Renders a single block-level node by its [`NodeKind`], without applying
    /// any node-level [`Layout`].
    fn render_kind(&mut self, node: &RenderNode) -> Result<String, RenderError> {
        match &node.kind {
            NodeKind::Root { children } => {
                // A `Compose`-style sequence joins children in order with no
                // renderer-inserted separator; a normal document root joins
                // children as blocks with blank-line separators.
                match node.attrs.sequence_join() {
                    Some(renderable::tree::SequenceJoin::None) => self.render_sequence(children),
                    None => self.render_blocks(children),
                }
            }
            NodeKind::Heading { depth, children } => {
                let effective = heading_effective(depth.get()).inherited_from(&self.effective);
                let markup = self.render_inline(children, &effective)?;
                Ok(self.render_heading_line(depth.get(), &markup))
            }
            NodeKind::Section {
                depth,
                heading,
                children,
            } => {
                let effective = heading_effective(depth.get()).inherited_from(&self.effective);
                let markup = self.render_inline(heading, &effective)?;
                let heading_output = self.render_heading_line(depth.get(), &markup);
                let body = self.render_blocks(children)?;
                if body.is_empty() {
                    Ok(heading_output)
                } else {
                    Ok(format!("{heading_output}\n\n{body}"))
                }
            }
            NodeKind::Paragraph { children } => {
                // The inherited `effective` style already folds in this
                // paragraph's own appearance (via `render_styled`) and any
                // styled ancestor block, so a nested styled span restores the
                // ancestor color/emphasis after it.
                let effective = self.effective.clone();
                let markup = self.render_inline(children, &effective)?;
                if let Some(hints) = node.attrs.progress_hints() {
                    let bar = render_progress_bar(&hints, &markup, self.opts.context.color_depth);
                    Ok(self.render_prose(&bar))
                } else {
                    Ok(self.render_prose(&markup))
                }
            }
            NodeKind::BlockQuote { children } => {
                if let Some(hints) = node.attrs.columns_hints() {
                    self.render_columns(children, &hints)
                } else if node.attrs.style().is_some_and(|s| !s.is_empty()) {
                    // The node carries a declared `Style` — a migrated
                    // `BlockQuote` component projects its border, fill, and
                    // colors onto the node. `render_styled` lowers that
                    // `Style` (border glyphs, fill band, text color), so the
                    // inner blocks render without a reconstituted bespoke
                    // border that would double up with the styled one.
                    //
                    // The content is still word-wrapped to the width left
                    // after the border, matching the bespoke `BlockQuote`'s
                    // intrinsic `WrapProse` behavior — without it the inner
                    // text would overrun the bordered band. The context width
                    // is already narrowed by the border overhead at this
                    // point (see `render_styled`).
                    let inner = self.render_blocks(children)?;
                    let lines = split_lines(&inner);
                    let wrapped = wrap_lines(
                        lines,
                        &WordWrap::WrapProse(Some(8), None),
                        self.opts.context.available_width,
                    );
                    Ok(wrapped.join("\n"))
                } else {
                    let inner = self.render_blocks(children)?;
                    let quote = BlockQuote::from(inner.as_str());
                    Ok(quote.render(&self.opts.context.terminal))
                }
            }
            NodeKind::List {
                ordered,
                start,
                children,
            } => self.render_list(*ordered, *start, children, &node.attrs),
            NodeKind::ListItem { checked, children } => {
                let body = self.render_blocks(children)?;
                Ok(match checked {
                    Some(true) => format!("[x] {body}"),
                    Some(false) => format!("[ ] {body}"),
                    None => body,
                })
            }
            NodeKind::Code { lang, meta, value } => Ok(self.render_code_node(
                lang.as_deref(),
                value,
                meta.as_deref(),
                &node.attrs,
            )),
            NodeKind::ThematicBreak => {
                let rule = horizontal_rule_from_attrs(&node.attrs);
                Ok(rule.render(&self.opts.context.terminal))
            }
            NodeKind::Table { align, children } => {
                let table = self.render_table(align, children, node)?;
                // A table title/caption is emitted above the top border. An
                // empty or whitespace-only title is ignored.
                match node.attrs.table_title() {
                    Some(title) if !title.trim().is_empty() => {
                        let heading = self.render_prose(&Prose::escape_text(title.trim()));
                        Ok(format!("{heading}\n{table}"))
                    }
                    _ => Ok(table),
                }
            }
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
                // A cell rendered on its own (outside a Table) still honors a
                // declared slot `Style` so styled header/body cells render
                // visibly rather than flattening through `Style::default()`.
                let markup = self.render_inline(children, &Style::default())?;
                let styled = self.apply_cell_style(node, markup);
                Ok(self.render_prose(&styled))
            }
            NodeKind::FootnoteDefinition {
                identifier,
                children,
            } => {
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
                let markup = self.render_inline(std::slice::from_ref(node), &Style::default())?;
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

    /// Renders a sequence of children in order with no inserted separator.
    ///
    /// This is the `Compose`-style join: adjacent children concatenate
    /// directly, preserving the component's no-separator contract instead of
    /// the document-block blank-line spacing of [`Self::render_blocks`].
    ///
    /// Top-level [`NodeKind::Text`] children render their literal `value`
    /// verbatim — including caller-supplied trailing newlines and
    /// whitespace, which Compose's contract requires. Routing them through
    /// [`Self::render_prose`] would trigger Prose word-wrap and trim trailing
    /// whitespace. Other inline kinds (`Emphasis`, `Strong`, `Span`, …)
    /// still flow through [`Self::render_inline_node`] so styling lowers to
    /// SGR, but the resulting markup is then lowered without the
    /// trailing-newline trim. Block kinds (`Section`, `Table`, `List`, …)
    /// continue through the normal block path so their internal layout is
    /// preserved.
    fn render_sequence(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
        let mut output = String::new();
        for child in children {
            match &child.kind {
                // Push `&str` directly — Compose's sequence-join hot path can
                // hold many small text parts, and a `String::clone` per part
                // would be a needless heap allocation. The borrowed value is
                // already owned by the input tree for the duration of this
                // call.
                NodeKind::Text { value } => output.push_str(value),
                kind if is_inline_kind(kind) => {
                    let effective = self.effective.clone();
                    let markup = self.render_inline_node(child, &effective)?;
                    // Lower the Prose markup to terminal SGR without
                    // applying Prose's word-wrap / trailing-newline trim.
                    output.push_str(&Prose::new(&markup).render_optimistic(None));
                }
                _ => {
                    let rendered = self.render(child)?;
                    output.push_str(&rendered);
                }
            }
        }
        Ok(output)
    }

    /// Renders a two-column block quote: a [`NodeKind::BlockQuote`] carrying
    /// [`ColumnsHints`].
    ///
    /// The flat child list is split at [`ColumnsHints::left_count`] into the
    /// left and right column groups, each rendered as its own block sequence.
    /// Column widths are resolved from the context's available width, the
    /// gap, and the left column's width hint. When the available width is too
    /// narrow for both columns, the groups stack vertically. The block
    /// quote's border is never drawn.
    fn render_columns(
        &mut self,
        children: &[RenderNode],
        hints: &renderable::tree::ColumnsHints,
    ) -> Result<String, RenderError> {
        let split = hints.left_count.min(children.len());
        let (left_children, right_children) = children.split_at(split);

        let total = self.opts.context.available_width;

        // Too narrow to host both columns plus the gap: stack vertically.
        let stackable = hints.stack_below;
        if total <= hints.gap || total == 0 {
            return self.render_columns_stacked(left_children, right_children);
        }

        let available = total - hints.gap;
        let mut left_width = match hints.left_width {
            renderable::tree::ColumnWidthKind::Fixed(chars) => chars,
            renderable::tree::ColumnWidthKind::Percent(percent) => {
                (available as f32 * percent.clamp(0.0, 1.0)) as u32
            }
        };
        left_width = left_width.clamp(1, available.saturating_sub(1).max(1));
        let right_width = available.saturating_sub(left_width);

        // A degenerate split (one column would be empty) stacks instead.
        if right_width == 0 && stackable {
            return self.render_columns_stacked(left_children, right_children);
        }

        // Render each column's blocks within its resolved width, then wrap
        // the result to that width. The wrap mirrors the bespoke `TwoColumn`
        // renderer, which wraps every column with `WrapProse`; without it a
        // paragraph (whose `Layout` default is `WordWrap::None`) would overrun
        // a narrow column instead of flowing onto extra rows.
        let left = self.render_blocks_in_width(left_children, left_width.max(1))?;
        let right = self.render_blocks_in_width(right_children, right_width.max(1))?;
        let left = wrap_column_lines(&left, left_width.max(1));
        let right = wrap_column_lines(&right, right_width.max(1));

        let left_lines: Vec<&str> = left.split('\n').collect();
        let right_lines: Vec<&str> = right.split('\n').collect();
        let rows = left_lines.len().max(right_lines.len());
        let gutter = " ".repeat(hints.gap as usize);

        let mut out = Vec::with_capacity(rows);
        for i in 0..rows {
            let l = left_lines.get(i).copied().unwrap_or("");
            let r = right_lines.get(i).copied().unwrap_or("");
            // Pad both columns to their resolved widths so the block stays
            // rectangular and the gutter aligns, matching the bespoke
            // `TwoColumn` renderer.
            let left_pad = left_width.saturating_sub(visible_width(l));
            let right_pad = right_width.saturating_sub(visible_width(r));
            out.push(format!(
                "{l}{}{gutter}{r}{}",
                " ".repeat(left_pad as usize),
                " ".repeat(right_pad as usize),
            ));
        }
        Ok(out.join("\n"))
    }

    /// Renders the two column groups stacked vertically (the narrow fallback).
    fn render_columns_stacked(
        &mut self,
        left_children: &[RenderNode],
        right_children: &[RenderNode],
    ) -> Result<String, RenderError> {
        let left = self.render_blocks(left_children)?;
        let right = self.render_blocks(right_children)?;
        Ok(match (left.is_empty(), right.is_empty()) {
            (true, true) => String::new(),
            (false, true) => left,
            (true, false) => right,
            (false, false) => format!("{left}\n{right}"),
        })
    }

    /// Renders a sequence of block nodes within a constrained width.
    ///
    /// The renderer's context is temporarily narrowed so nested components
    /// wrap to the column width, then restored.
    fn render_blocks_in_width(
        &mut self,
        children: &[RenderNode],
        width: u32,
    ) -> Result<String, RenderError> {
        let mut narrowed = self.opts.clone();
        narrowed.context.available_width = width;
        narrowed.context.width = width;
        narrowed.context.terminal.fixed_width = Some(width);
        let mut sub = Writer {
            opts: &narrowed,
            diagnostics: Vec::new(),
            effective: self.effective.clone(),
        };
        let result = sub.render_blocks(children);
        self.diagnostics.append(&mut sub.diagnostics);
        result
    }

    /// Renders a sequence of inline nodes into [`Prose`] block-tag markup.
    ///
    /// Literal text is escaped with [`Prose::escape_text`] so document text
    /// never accidentally triggers Prose markup; styling wraps the escaped
    /// runs in block tags the [`Prose`] parser already understands.
    ///
    /// `effective` is the text appearance (color, emphasis) inherited from the
    /// enclosing block and ancestor spans. A nested
    /// [`Span`](renderable::tree::NodeKind::Span) with its own [`Style`]
    /// applies its effective appearance and restores `effective` afterwards.
    fn render_inline(
        &mut self,
        children: &[RenderNode],
        effective: &Style,
    ) -> Result<String, RenderError> {
        let mut output = String::new();
        for child in children {
            output.push_str(&self.render_inline_node(child, effective)?);
        }
        Ok(output)
    }

    /// Projects a single inline node into [`Prose`] markup.
    fn render_inline_node(
        &mut self,
        node: &RenderNode,
        effective: &Style,
    ) -> Result<String, RenderError> {
        match &node.kind {
            NodeKind::Text { value } => Ok(apply_classes(
                &Prose::escape_text(value),
                &node.attrs.classes,
            )),
            NodeKind::Emphasis { children } => {
                let inner = self.render_inline(children, effective)?;
                Ok(apply_classes(
                    &format!("<italic>{inner}</italic>"),
                    &node.attrs.classes,
                ))
            }
            NodeKind::Strong { children } => {
                let inner = self.render_inline(children, effective)?;
                Ok(apply_classes(
                    &format!("<bold>{inner}</bold>"),
                    &node.attrs.classes,
                ))
            }
            NodeKind::Delete { children } => {
                let inner = self.render_inline(children, effective)?;
                Ok(apply_classes(
                    &format!("<strikethrough>{inner}</strikethrough>"),
                    &node.attrs.classes,
                ))
            }
            NodeKind::Span { children } => {
                // An inline `Span` may carry a declared `Style`. Its
                // text-appearance layers (color, emphasis) inherit from the
                // enclosing `effective` appearance; box-painting layers
                // (border, fill) and background have no inline meaning here.
                match node.attrs.style().filter(|s| !s.is_empty()) {
                    Some(span_style) => {
                        let child_effective = span_style.inherited_from(effective);
                        let inner = self.render_inline(children, &child_effective)?;
                        let styled = self.render_span_classes(node, &inner)?;
                        let term = &self.opts.context.terminal;
                        let open = style::text_appearance_sgr(&child_effective, term);
                        // Reset, then restore the ancestor appearance so the
                        // run after the span keeps the inherited color/emphasis.
                        let close = format!(
                            "{}{}",
                            style::SGR_RESET,
                            style::text_appearance_sgr(effective, term),
                        );
                        Ok(format!("{open}{styled}{close}"))
                    }
                    None => {
                        let inner = self.render_inline(children, effective)?;
                        self.render_span_classes(node, &inner)
                    }
                }
            }
            NodeKind::InlineCode { value } => Ok(apply_classes(
                &format!("<dim>{}</dim>", Prose::escape_text(value)),
                &node.attrs.classes,
            )),
            NodeKind::Link {
                url,
                title: _,
                children,
            } => {
                let inner = self.render_inline(children, effective)?;
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
            | NodeKind::Section { .. }
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

    /// Renders a heading line from a depth and pre-rendered inline `markup`.
    ///
    /// A Markdown-style prefix (`# `..`###### `) plus the title, wrapped in the
    /// heading's declared [`Style`] — bold for depths 1-3, italic for depths
    /// 4-5, and plain for depth 6. The renderer consumes the declared
    /// [`TextEmphasis`](renderable::style::TextEmphasis) through
    /// [`style::apply_style`] rather than splicing SGR strings directly.
    fn render_heading_line(&self, depth: u8, markup: &str) -> String {
        let prefix = match depth {
            1 => "# ",
            2 => "## ",
            3 => "### ",
            4 => "#### ",
            5 => "##### ",
            _ => "###### ",
        };
        let title = self.render_prose(markup);
        let heading_style = Style {
            emphasis: crate::components::section::heading_emphasis(depth),
            ..Style::default()
        };
        style::apply_style(
            &format!("{prefix}{title}"),
            &heading_style,
            &self.opts.context.terminal,
            self.opts.context.available_width,
        )
    }

    /// Renders a list natively, reproducing the bespoke `OrderedList` /
    /// `UnorderedList` formatting without constructing those components.
    ///
    /// Ordered lists number from `start` (default 1) with a prefix whose
    /// width grows as the index does (`1. ` is 3 chars, `10. ` is 4, etc.).
    /// Unordered lists use the `bullet` list hint, defaulting to `"- "`.
    /// Each item's children are classified as inline-only, block-only, or
    /// mixed and rendered accordingly (see [`Self::render_list_item`]).
    fn render_list(
        &mut self,
        ordered: bool,
        start: Option<u64>,
        children: &[RenderNode],
        attrs: &renderable::tree::NodeAttrs,
    ) -> Result<String, RenderError> {
        // A typed list marker policy can override the normal marker
        // presentation. `Default` keeps the bullet/ordinal rendering below.
        match attrs.list_marker_policy() {
            renderable::tree::ListMarkerPolicy::Default => {}
            renderable::tree::ListMarkerPolicy::None => {
                return self.render_marker_free_list(children);
            }
            renderable::tree::ListMarkerPolicy::TreeConnectors => {
                return self.render_tree_connector_list(children, "");
            }
        }

        let hints = attrs.list_hints();
        let bullet = hints.bullet.unwrap_or_else(|| "- ".to_string());
        let origin = start.unwrap_or(1);

        // The indent for nested block children: explicit hint, else the
        // default for the list kind (4 for ordered, bullet width otherwise).
        let default_indent = if ordered { 4 } else { visible_width(&bullet) };
        let indent_children = hints.indent_children.unwrap_or(default_indent);

        let mut lines = Vec::with_capacity(children.len());
        for (offset, child) in children.iter().enumerate() {
            let prefix = if ordered {
                let index = origin + offset as u64;
                format!("{index}. ")
            } else {
                bullet.clone()
            };
            lines.push(self.render_list_item(
                child,
                &prefix,
                hints.hanging_indent,
                indent_children,
            )?);
        }
        Ok(lines.join("\n"))
    }

    /// Renders a list with no item markers ([`ListMarkerPolicy::None`]).
    ///
    /// Each item's children render as plain blocks; nested lists inherit the
    /// same no-marker policy. No bullet, ordinal, or connector is emitted.
    ///
    /// [`ListMarkerPolicy::None`]: renderable::tree::ListMarkerPolicy::None
    fn render_marker_free_list(
        &mut self,
        children: &[RenderNode],
    ) -> Result<String, RenderError> {
        let mut lines = Vec::with_capacity(children.len());
        for child in children {
            let body = match &child.kind {
                NodeKind::ListItem {
                    children: item_children,
                    ..
                } => self.render_blocks(item_children)?,
                // Defensive: a non-ListItem child is a structural problem the
                // validator rejects; render it as a plain block.
                _ => self.render(child)?,
            };
            lines.push(body);
        }
        Ok(lines.join("\n"))
    }

    /// Renders a list with terminal box-drawing connectors
    /// ([`ListMarkerPolicy::TreeConnectors`]).
    ///
    /// Each item gets a `├── ` branch, or `└── ` when it is the last child.
    /// Depth, last-child state, and ancestor continuation lines (`│   ` for a
    /// non-last ancestor, four spaces for a last ancestor) are inferred from
    /// the nested `List` / `ListItem` structure: `ancestor_prefix` is the
    /// accumulated continuation string for every enclosing list level.
    ///
    /// [`ListMarkerPolicy::TreeConnectors`]: renderable::tree::ListMarkerPolicy::TreeConnectors
    fn render_tree_connector_list(
        &mut self,
        children: &[RenderNode],
        ancestor_prefix: &str,
    ) -> Result<String, RenderError> {
        let mut lines = Vec::with_capacity(children.len());
        let count = children.len();
        for (index, child) in children.iter().enumerate() {
            let is_last = index + 1 == count;
            let branch = if is_last { "└── " } else { "├── " };
            // The continuation prefix for this item's own nested content: a
            // last item's descendants align under blank space, a non-last
            // item's descendants keep a `│` rail.
            let continuation = if is_last {
                format!("{ancestor_prefix}    ")
            } else {
                format!("{ancestor_prefix}│   ")
            };

            let item_children: &[RenderNode] = match &child.kind {
                NodeKind::ListItem {
                    children: item_children,
                    ..
                } => item_children,
                // Defensive: a non-ListItem child is rejected by the
                // validator; render it on a branch line.
                _ => {
                    let body = self.render(child)?;
                    lines.push(format!("{ancestor_prefix}{branch}{body}"));
                    continue;
                }
            };

            // Split the item's own content (inline / paragraph) from any
            // nested lists. The content rides the branch line; nested lists
            // recurse with the deeper continuation prefix.
            let mut label_parts: Vec<String> = Vec::new();
            let mut nested: Vec<String> = Vec::new();
            for item_child in item_children {
                if let NodeKind::List {
                    children: nested_children,
                    ..
                } = &item_child.kind
                {
                    nested.push(
                        self.render_tree_connector_list(nested_children, &continuation)?,
                    );
                } else if is_inline_block(item_child) {
                    let markup = match &item_child.kind {
                        NodeKind::Paragraph { children } => {
                            self.render_inline(children, &Style::default())?
                        }
                        _ => self.render_inline(
                            std::slice::from_ref(item_child),
                            &Style::default(),
                        )?,
                    };
                    label_parts.push(self.render_prose(&markup));
                } else {
                    label_parts.push(self.render(item_child)?);
                }
            }

            let label = label_parts.join(" ");
            lines.push(format!("{ancestor_prefix}{branch}{label}"));
            for nested_list in nested {
                lines.push(nested_list);
            }
        }
        Ok(lines.join("\n"))
    }

    /// Renders a single list item with its `prefix`.
    ///
    /// A [`NodeKind::ListItem`]'s `checked` flag prepends a `[x] `/`[ ] `
    /// marker. The item's children are split into block-level and
    /// inline/paragraph nodes:
    ///
    /// - **Inline-only** — the prefix is followed by wrapped text, with
    ///   continuation lines hanging-indented after the prefix when
    ///   `hanging_indent` is set.
    /// - **Block-only** — every block is indented by `indent_children`
    ///   with no prefix.
    /// - **Mixed** — the first paragraph carries the prefix; the remaining
    ///   blocks are indented by `indent_children`.
    fn render_list_item(
        &mut self,
        node: &RenderNode,
        prefix: &str,
        hanging_indent: bool,
        indent_children: u32,
    ) -> Result<String, RenderError> {
        let (checked, item_children): (Option<bool>, &[RenderNode]) = match &node.kind {
            NodeKind::ListItem { checked, children } => (*checked, children),
            // Defensive: a non-ListItem child of a List is a structural
            // problem the validator rejects; render it as a plain block.
            _ => {
                let body = self.render(node)?;
                return Ok(prefix_first_line(prefix, &body));
            }
        };

        // A projected `Todo` carries typed task hints on the `ListItem`; the
        // task-state glyph replaces the default `[ ] ` / `[x] ` checkbox. The
        // marker applies only to the checkbox — description styling stays in
        // the item's child nodes and `Style`.
        let check_marker = match node.attrs.task_hints() {
            Some(hints) => task_state_marker(hints.state, &self.opts.context.terminal),
            None => match checked {
                Some(true) => "[x] ".to_string(),
                Some(false) => "[ ] ".to_string(),
                None => String::new(),
            },
        };
        let full_prefix = format!("{prefix}{check_marker}");

        let mut out = String::new();
        let mut prefix_used = false;
        for (idx, child) in item_children.iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            if !prefix_used && is_inline_block(child) {
                // Inline/paragraph child: carries the prefix, with the
                // prefix width as hanging indent for continuation lines.
                let markup = match &child.kind {
                    NodeKind::Paragraph { children } => {
                        self.render_inline(children, &Style::default())?
                    }
                    _ => self.render_inline(std::slice::from_ref(child), &Style::default())?,
                };
                out.push_str(&self.render_list_text(&full_prefix, &markup, hanging_indent));
                prefix_used = true;
            } else {
                // Block child: indent by `indent_children`, no prefix.
                let body = self.render(child)?;
                out.push_str(&indent_block(&body, indent_children));
            }
        }

        // An item with no children still occupies a prefixed line.
        if item_children.is_empty() {
            out.push_str(&full_prefix);
        }

        Ok(out)
    }

    /// Renders prefixed, hanging-indented text for an inline list item.
    ///
    /// The inline `markup` is wrapped at `term_width - prefix_width`; the
    /// first line carries the `prefix`, and continuation lines are padded
    /// by the prefix width when `hanging_indent` is set.
    fn render_list_text(&self, prefix: &str, markup: &str, hanging_indent: bool) -> String {
        let prefix_width = visible_width(prefix);
        let term = &self.opts.context.terminal;
        let child_width = term.width().saturating_sub(prefix_width);

        let hang = if hanging_indent {
            Some(prefix_width)
        } else {
            None
        };
        let prose = Prose::new(markup).with_word_wrap(WordWrap::WrapProse(None, hang));
        let rendered = prose.render_in_width(term, child_width);

        let mut out = String::new();
        for (idx, line) in rendered.split('\n').enumerate() {
            if idx == 0 {
                out.push_str(prefix);
            } else {
                out.push('\n');
            }
            out.push_str(line);
        }
        out
    }

    /// Renders a [`NodeKind::Code`] node, consulting the optional code-render
    /// hook before falling back to the built-in plain rendering.
    ///
    /// When [`TerminalRenderOptions::code_renderer`] is set, the hook is given
    /// the language, body, info-string `meta`, node attributes, and a
    /// [`TerminalCodeContext`] containing the available render width, color
    /// depth, and color mode. A `Some` result is used verbatim; a `None`
    /// result falls back to [`Self::render_code`].
    fn render_code_node(
        &self,
        lang: Option<&str>,
        value: &str,
        meta: Option<&str>,
        attrs: &renderable::tree::NodeAttrs,
    ) -> String {
        if let Some(renderer) = &self.opts.code_renderer {
            let context = TerminalCodeContext::new(
                self.opts.context.available_width,
                (&self.opts.context.color_depth).into(),
                (&self.opts.context.color_mode).into(),
            );
            if let Some(rendered) =
                renderer.render_terminal_code(lang, value, meta, attrs, context)
            {
                return rendered;
            }
        }
        self.render_code(lang, value)
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

    /// Renders a [`NodeKind::Table`] node with a native two-pass renderer.
    ///
    /// The renderer does not delegate to [`Table::render`]. Instead it
    /// reconstructs a [`Table`] purely as a width-planning input — applying
    /// per-column [`TableColumnHints`] and reconstructing typed cell values
    /// from cell hints — calls [`Table::plan_widths`] (pass 1), then emits
    /// borders, header, and data rows itself (pass 2). Conditional columns
    /// hidden at the current width are excluded; droppable columns whose
    /// `drop_note` is set append a note after the table.
    ///
    /// The first child row is treated as the header row.
    fn render_table(
        &mut self,
        align: &[ColumnAlign],
        children: &[RenderNode],
        table_node: &RenderNode,
    ) -> Result<String, RenderError> {
        let mut rows = children.iter();
        let Some(header) = rows.next() else {
            return Ok(String::new());
        };

        // ── Reconstruct columns from the header row + per-column hints ──────
        let header_cells = self.table_row_cells(header)?;
        let data_rows: Vec<&RenderNode> = rows.collect();

        // The cell kind hint of the first data row drives column types.
        let first_data_kinds: Vec<Option<String>> = data_rows
            .first()
            .map(|row| table_row_cell_kinds(row))
            .unwrap_or_default();

        let columns: Vec<TableColumn> = header_cells
            .iter()
            .enumerate()
            .map(|(idx, text)| {
                let hints = table_node.attrs.table_column_hints(idx);
                let kind = first_data_kinds.get(idx).and_then(|k| k.as_deref());
                build_table_column(text, idx, align, &hints, kind)
            })
            .collect();

        // ── Reconstruct typed data cells from cell hints ───────────────────
        let mut data: Vec<Vec<TableCellContent>> = Vec::with_capacity(data_rows.len());
        for row in &data_rows {
            data.push(self.table_row_typed_cells(row)?);
        }

        if columns.is_empty() {
            return Ok(String::new());
        }

        let terminal_hints = table_node.attrs.table_terminal_hints();
        let table = Table::new()
            .with_columns(columns.clone())
            .with_data(data.clone());

        // ── Pass 1: width planning ─────────────────────────────────────────
        let available = self.opts.context.available_width.max(1);
        let plan = match table.plan_widths(available) {
            Ok(plan) => plan,
            // Width planning failed (table cannot fit): degrade to the
            // structured error message, matching the bespoke component.
            Err(error) => return Ok(self.render_prose(&Prose::escape_text(&error.to_string()))),
        };

        // ── Pass 2: native emit ────────────────────────────────────────────
        // Striping degrades with the terminal's color depth through the
        // shared color path rather than being gated to truecolor.
        let mode = &self.opts.context.color_mode;
        let depth = self.opts.context.color_depth;
        let stripe_bg = terminal_hints
            .alternate_background
            .then(|| stripe_bg_escape(terminal_hints.stripe_bg, mode, depth))
            .flatten();
        let stripe_fg = terminal_hints
            .alternate_text_color
            .then(|| stripe_fg_escape(terminal_hints.stripe_text, mode, depth))
            .flatten();

        // The table's body slot style rides on every data cell node; resolve
        // it once to an SGR run so `emit_table` paints data cells visibly.
        let body_sgr = data_rows
            .first()
            .and_then(|row| match &row.kind {
                NodeKind::TableRow { children } => children.first(),
                _ => None,
            })
            .and_then(|cell| cell.attrs.style())
            .filter(|s| !s.is_empty())
            .map(|s| style::text_appearance_sgr(&s, &self.opts.context.terminal))
            .filter(|sgr| !sgr.is_empty());

        Ok(emit_table(
            &columns,
            &data,
            &plan,
            stripe_bg.as_deref(),
            stripe_fg.as_deref(),
            body_sgr.as_deref(),
        ))
    }

    /// Extracts the plain-text cells of a table row.
    ///
    /// Each cell's inline children are first rendered to [`Prose`] markup,
    /// then that markup is lowered to a styled terminal string via
    /// [`Self::render_prose`]. Skipping the Prose-rendering step would leak
    /// Prose tags (e.g. `<dim>`) and prose-escape backslashes (e.g.
    /// `\_` for literal underscores) into the final table cell output.
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
            let markup = self.render_inline(children, &Style::default())?;
            let rendered = self.render_prose(&markup);
            cells.push(self.apply_cell_style(cell, rendered));
        }
        Ok(cells)
    }

    /// Wraps a rendered table-cell string in its declared slot [`Style`].
    ///
    /// A `Table`'s header and body appearance slots ride onto the individual
    /// `TableCell` nodes via `set_style`. This lowers a cell's style to ANSI
    /// through the shared [`style::text_appearance_sgr`] path so the styled
    /// cell renders visibly instead of being flattened. A cell with no
    /// declared style (or one that lowers to nothing) is returned unchanged.
    fn apply_cell_style(&self, cell: &RenderNode, content: String) -> String {
        let Some(cell_style) = cell.attrs.style().filter(|s| !s.is_empty()) else {
            return content;
        };
        let open = style::text_appearance_sgr(&cell_style, &self.opts.context.terminal);
        if open.is_empty() {
            return content;
        }
        format!("{open}{content}{}", style::SGR_RESET)
    }

    /// Reconstructs the typed cell values of a data row.
    ///
    /// Each cell's hints supply the kind and the original typed value;
    /// without hints (or with malformed hints) the cell degrades to its
    /// rendered text.
    fn table_row_typed_cells(
        &mut self,
        row: &RenderNode,
    ) -> Result<Vec<TableCellContent>, RenderError> {
        let NodeKind::TableRow { children } = &row.kind else {
            return self.render(row).map(|s| vec![TableCellContent::from(s)]);
        };
        let mut cells = Vec::with_capacity(children.len());
        for cell in children {
            let NodeKind::TableCell {
                children: cell_children,
            } = &cell.kind
            else {
                cells.push(TableCellContent::from(self.render(cell)?));
                continue;
            };
            // Lower Prose markup to a terminal string so cell content does
            // not leak Prose tags or prose-escape backslashes. See
            // `Self::table_row_cells` for the same reasoning.
            let markup = self.render_inline(cell_children, &Style::default())?;
            let text = self.render_prose(&markup);
            cells.push(reconstruct_cell(&cell.attrs, text));
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
            if !is_known_class(class) && self.opts.strictness == RenderStrictness::Warn {
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

/// Renders a progress bar from [`ProgressHints`], reproducing the legacy
/// bespoke progress-bar output.
///
/// `paragraph_text` is the projected paragraph's visible text
/// (`"{label} {pct}%"` or `"{pct}%"`); any label portion is the text before
/// the trailing ` {pct}%` token and is preserved before the bar.
/// Wraps each line of a rendered column to `width`.
///
/// Mirrors the bespoke `TwoColumn` renderer, which flows every column through
/// `WrapProse` so content longer than the column width spills onto extra rows
/// rather than overrunning the gutter. Lines already within `width` pass
/// through unchanged, so columns holding pre-formatted block content (lists,
/// tables) are unaffected.
fn wrap_column_lines(rendered: &str, width: u32) -> String {
    let lines: Vec<String> = rendered.split('\n').map(str::to_string).collect();
    wrap_lines(lines, &WordWrap::WrapProse(None, None), width).join("\n")
}

/// Builds a [`HorizontalRule`] from `darkmatter.hr.*` hints on a
/// [`NodeKind::ThematicBreak`] node.
///
/// Darkmatter's span-aware fold lowers HR-attribute paragraphs (e.g.
/// `--- { style: waves, weight: thick }`) into a [`NodeKind::ThematicBreak`]
/// whose [`NodeAttrs::data`] carries string-valued hints under the
/// `darkmatter.hr` namespace — see
/// `renderable/features/2026-05-20-darkmatter-tree/span-aware-processor-design.md`.
/// Without consuming those hints the terminal renderer would emit a plain
/// rule for every HR regardless of authored attributes (review-4 finding 2).
///
/// Unknown enum values fall back to the [`HorizontalRule`] defaults so a
/// malformed `style: dashse` still produces output rather than panicking.
///
/// [`NodeAttrs::data`]: renderable::tree::NodeAttrs
fn horizontal_rule_from_attrs(attrs: &renderable::tree::NodeAttrs) -> HorizontalRule {
    const HR_NS: HintNamespace = HintNamespace("darkmatter.hr");

    let hint_str = |key: &str| -> Option<String> {
        attrs
            .get_hint(HR_NS, key)
            .and_then(|v| v.as_str().map(str::to_string))
    };

    let mut rule = HorizontalRule::new();
    if let Some(style) = hint_str("style") {
        rule = match style.as_str() {
            "dashes" => rule.style(RuleStyle::Dashes),
            "dots" => rule.style(RuleStyle::Dots),
            "waves" => rule.style(RuleStyle::Waves),
            "line-star" => rule.style(RuleStyle::LineStar),
            "line-circle" => rule.style(RuleStyle::LineCircle),
            "inset-line" => rule.style(RuleStyle::InsetLine),
            "curtain-rod" => rule.style(RuleStyle::CurtainRod),
            _ => rule,
        };
    }
    if let Some(alignment) = hint_str("alignment") {
        rule = match alignment.as_str() {
            "full" => rule.alignment(RuleAlignment::Full),
            "centered" => rule.alignment(RuleAlignment::Centered),
            "left" => rule.alignment(RuleAlignment::Left),
            "right" => rule.alignment(RuleAlignment::Right),
            _ => rule,
        };
    }
    if let Some(weight) = hint_str("weight") {
        rule = match weight.as_str() {
            "thin" => rule.weight(RuleWeight::Thin),
            "medium" => rule.weight(RuleWeight::Medium),
            "thick" => rule.weight(RuleWeight::Thick),
            _ => rule,
        };
    }
    if let Some(width) = hint_str("width") {
        rule = rule.width(width);
    }
    if let Some(color) = hint_str("color") {
        rule = rule.color(color);
    }
    rule
}

fn render_progress_bar(hints: &ProgressHints, paragraph_text: &str, depth: ColorDepth) -> String {
    use crate::components::progress::paint_fg;

    let value = hints.value.clamp(0.0, 1.0);
    let percentage = (value * 100.0).round() as u32;
    let filled_count = ((value * hints.bar_width as f32).round() as u32).min(hints.bar_width);
    let empty_count = hints.bar_width.saturating_sub(filled_count);

    // Each segment's declared slot color is degraded against `depth` through
    // the shared lowering, matching the legacy bespoke progress bar.
    let filled = paint_fg(
        &hints.fill_char.to_string().repeat(filled_count as usize),
        hints.filled_color,
        depth,
    );
    let empty = paint_fg(
        &hints.empty_char.to_string().repeat(empty_count as usize),
        hints.empty_color,
        depth,
    );
    let left = paint_fg(&hints.left_bracket.to_string(), hints.bracket_color, depth);
    let right = paint_fg(&hints.right_bracket.to_string(), hints.bracket_color, depth);
    let percentage_str = format!("{percentage:3}%");

    // The label is everything before the trailing percentage token.
    let pct_suffix = format!(" {percentage}%");
    let label = paragraph_text
        .strip_suffix(&pct_suffix)
        .filter(|label| !label.is_empty());

    match label {
        Some(label) => format!("{label} {left}{filled}{empty}{right} {percentage_str}"),
        None => format!("{left}{filled}{empty}{right} {percentage_str}"),
    }
}

/// Resolves a [`TargetValue<Length>`] to whole terminal cells against `width`.
///
/// [`Length::Percent`] is taken as a fraction of `width`;
/// [`Length::Css`] has no terminal meaning and resolves to `0`.
///
/// [`TargetValue<Length>`]: renderable::layout::TargetValue
pub(crate) fn resolve_cells(
    tv: &renderable::layout::TargetValue<renderable::layout::Length>,
    width: u32,
) -> u32 {
    use renderable::layout::Length;
    use renderable::target::RenderTarget;
    match tv.resolve(RenderTarget::Terminal) {
        Some(Length::Zero) | None => 0,
        Some(Length::Ch(n)) => *n,
        Some(Length::Percent(p)) => ((width as f32) * p / 100.0).round() as u32,
        Some(Length::Css(_)) => 0,
    }
}

/// The inherited text appearance for a heading's inline content.
///
/// A heading's declared emphasis (bold for depths 1-3, italic for 4-5) seeds
/// the `effective` style threaded through inline rendering, so a nested
/// styled [`Span`](renderable::tree::NodeKind::Span) restores the heading
/// emphasis after it.
fn heading_effective(depth: u8) -> Style {
    Style {
        emphasis: crate::components::section::heading_emphasis(depth),
        ..Style::default()
    }
}

/// Returns `true` for a semantic class with a defined terminal treatment.
fn is_known_class(class: &str) -> bool {
    matches!(class, "mark" | "dim" | "sup" | "sub")
}

/// Returns `true` when `kind` is an inline (phrasing-level) node kind.
///
/// Layout is only applied to block-level nodes. When an inline node carries
/// a layout (a warning-severity validation finding), the renderer drops it.
fn is_inline_kind(kind: &NodeKind) -> bool {
    matches!(
        kind,
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
            | NodeKind::HardBreak
    )
}

/// Returns `true` when `node` is a paragraph or an inline-level node.
///
/// Inside a list item, such nodes flow on the prefixed line; every other
/// node is block-level and is indented without a prefix.
fn is_inline_block(node: &RenderNode) -> bool {
    matches!(node.kind, NodeKind::Paragraph { .. }) || is_inline_kind(&node.kind)
}

/// Indents every line of `body` by `indent` spaces.
fn indent_block(body: &str, indent: u32) -> String {
    let pad = " ".repeat(indent as usize);
    body.split('\n')
        .map(|line| format!("{pad}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Prepends `prefix` to the first line of `body`, leaving the rest as-is.
fn prefix_first_line(prefix: &str, body: &str) -> String {
    let mut out = String::new();
    for (idx, line) in body.split('\n').enumerate() {
        if idx == 0 {
            out.push_str(prefix);
        } else {
            out.push('\n');
        }
        out.push_str(line);
    }
    out
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

/// The terminal checkbox marker for a task-list item in a given [`TaskState`].
///
/// Mirrors the bespoke `Todo` component's marker selection: a Nerd Font
/// terminal uses the state's icon glyph; otherwise a colored or no-color
/// ASCII fallback is used. The returned string includes a single trailing
/// space so it slots in where the default `[ ] ` / `[x] ` marker would.
///
/// The marker applies only to the checkbox; description styling (a `Cancelled`
/// item's strikethrough/dim) is carried by the item's child nodes and `Style`.
fn task_state_marker(state: renderable::tree::TaskState, term: &crate::terminal::Terminal) -> String {
    use crate::components::todo::{
        FB_CHECKBOX_BLOCKED_NOCOLOR, FB_CHECKBOX_CANCELLED_NOCOLOR, FB_CHECKBOX_COMPLETED_NOCOLOR,
        FB_CHECKBOX_IN_PROGRESS_NOCOLOR, FB_CHECKBOX_OPEN, TODO_CHAR_LOOKUP, TodoState,
    };
    use renderable::tree::TaskState;

    let todo_state = match state {
        TaskState::Open => TodoState::Open,
        TaskState::InProgress => TodoState::InProgress,
        TaskState::Completed => TodoState::Completed,
        TaskState::Blocked => TodoState::Blocked,
        TaskState::Cancelled => TodoState::Cancelled,
    };
    let icon = &TODO_CHAR_LOOKUP[&todo_state];
    let has_color = term.color_depth != ColorDepth::None;

    let glyph = if term.is_nerd_font == Some(true) {
        icon.nerd.to_string()
    } else if has_color {
        icon.fallback.to_string()
    } else {
        match todo_state {
            TodoState::Open => FB_CHECKBOX_OPEN.to_string(),
            TodoState::InProgress => FB_CHECKBOX_IN_PROGRESS_NOCOLOR.to_string(),
            TodoState::Completed => FB_CHECKBOX_COMPLETED_NOCOLOR.to_string(),
            TodoState::Cancelled => FB_CHECKBOX_CANCELLED_NOCOLOR.to_string(),
            TodoState::Blocked => FB_CHECKBOX_BLOCKED_NOCOLOR.to_string(),
        }
    };
    format!("{glyph} ")
}

/// Maps a render-tree column alignment to a layout alignment.
fn column_alignment(align: ColumnAlign) -> Alignment {
    match align {
        ColumnAlign::Left | ColumnAlign::None => Alignment::Left,
        ColumnAlign::Center => Alignment::Center,
        ColumnAlign::Right => Alignment::Right,
    }
}

/// Maps a [`ColumnConditional`] hint to the table component's [`Conditional`].
fn conditional_from_hint(conditional: ColumnConditional) -> Conditional {
    match conditional {
        ColumnConditional::Always => Conditional::Always,
        ColumnConditional::WidthGreaterThan(n) => Conditional::WidthGreaterThan(n),
        ColumnConditional::LessThanOrEqual(n) => Conditional::LessThanOrEqual(n),
    }
}

/// Maps a cell-kind hint token to a [`ColumnType`].
///
/// An unknown or absent token falls back to [`ColumnType::String`].
fn column_type_from_kind(kind: Option<&str>) -> ColumnType {
    match kind {
        Some("integer") => ColumnType::Integer,
        Some("float") => ColumnType::Float,
        // Currency requires a symbol; the column type only affects alignment
        // and wrap, so `Float` is an adequate stand-in for a currency column.
        Some("currency") => ColumnType::Float,
        _ => ColumnType::String,
    }
}

/// Returns the per-cell kind hints for every cell of a table row.
fn table_row_cell_kinds(row: &RenderNode) -> Vec<Option<String>> {
    let NodeKind::TableRow { children } = &row.kind else {
        return Vec::new();
    };
    children
        .iter()
        .map(|cell| cell.attrs.table_cell_hints().map(|h| h.kind))
        .collect()
}

/// Reconstructs a [`TableColumn`] from a header text and its hints.
fn build_table_column(
    header: &str,
    idx: usize,
    align: &[ColumnAlign],
    hints: &TableColumnHints,
    kind: Option<&str>,
) -> TableColumn {
    let mut col = TableColumn::new(header.to_string()).with_type(column_type_from_kind(kind));
    if let Some(a) = align.get(idx) {
        col = col.with_alignment(column_alignment(*a));
    }
    if let Some(w) = hints.min_width {
        col = col.with_min_width(w as usize);
    }
    if let Some(w) = hints.max_width {
        col = col.with_max_width(w as usize);
    }
    if let Some(w) = hints.fixed_width {
        col = col.with_fixed_width(w as usize);
    }
    col = col.with_when(conditional_from_hint(hints.conditional));
    col = col.with_uniform_alignment(hints.uniform_alignment);
    if let Some(note) = &hints.drop_note {
        col = col.drop_when_space_is_limited(Some(note.clone()));
    }
    col
}

/// Reconstructs a typed [`TableCellContent`] from cell attributes.
///
/// The cell hint's `kind` and `raw_value` recover the original typed value
/// so numeric/currency cells re-render with their readable formatting.
/// Malformed or absent hints degrade to the rendered `text`.
fn reconstruct_cell(attrs: &renderable::tree::NodeAttrs, text: String) -> TableCellContent {
    let Some(hints) = attrs.table_cell_hints() else {
        return TableCellContent::Text(text);
    };
    match hints.kind.as_str() {
        "integer" => hints
            .raw_value
            .as_i64()
            .map(TableCellContent::Integer)
            .unwrap_or(TableCellContent::Text(text)),
        "float" => hints
            .raw_value
            .as_f64()
            .map(TableCellContent::Float)
            .unwrap_or(TableCellContent::Text(text)),
        "currency" => {
            let amount = hints.raw_value.get("amount").and_then(|v| v.as_f64());
            let currency = hints
                .raw_value
                .get("currency")
                .and_then(|v| v.as_str())
                .and_then(currency_from_token);
            match (currency, amount) {
                (Some(c), Some(a)) => TableCellContent::Currency(c, a),
                _ => TableCellContent::Text(text),
            }
        }
        // "text" and anything unrecognized keep the rendered text.
        _ => TableCellContent::Text(text),
    }
}

/// Maps a currency token back to a [`Currency`].
fn currency_from_token(token: &str) -> Option<crate::components::table::Currency> {
    use crate::components::table::Currency;
    match token {
        "USD" => Some(Currency::USD),
        "GBP" => Some(Currency::GBP),
        "EUR" => Some(Currency::EUR),
        _ => None,
    }
}

/// Emits the table body from a resolved [`TableWidthPlan`] (pass 2).
///
/// Mirrors the bespoke `Table::render_content` emit logic — borders, a
/// multi-line header, vertically aligned data rows, and striping that
/// survives SGR resets inside cells — without calling `Table::render`.
fn emit_table(
    columns: &[TableColumn],
    data: &[Vec<TableCellContent>],
    plan: &TableWidthPlan,
    stripe_bg: Option<&str>,
    stripe_fg: Option<&str>,
    body_sgr: Option<&str>,
) -> String {
    let widths: Vec<usize> = plan.columns.iter().map(|c| c.resolved_width).collect();
    if widths.is_empty() {
        return String::new();
    }

    let mut result = String::new();

    // Top border.
    result.push_str(&build_border(&widths, '┌', '┬', '┐'));
    result.push('\n');

    // Header row (explicit newlines only, no word wrap).
    let header_lines: Vec<Vec<String>> = plan
        .columns
        .iter()
        .map(|cp| {
            let header = columns
                .get(cp.original_index)
                .map(|c| c.header.as_str())
                .unwrap_or("");
            wrap_cell_content(header, &WordWrap::None, cp.resolved_width)
        })
        .collect();
    let header_height = header_lines.iter().map(Vec::len).max().unwrap_or(1);
    let padded_headers: Vec<Vec<String>> = header_lines
        .into_iter()
        .enumerate()
        .map(|(i, lines)| {
            let width = widths.get(i).copied().unwrap_or(0);
            apply_vertical_padding(lines, header_height, VerticalAlign::Top, width)
        })
        .collect();

    for line_idx in 0..header_height {
        let mut row = String::from("│ ");
        for (i, cp) in plan.columns.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(cp.resolved_width);
            let alignment = columns
                .get(cp.original_index)
                .map(TableColumn::effective_alignment)
                .unwrap_or(Alignment::Left);
            let line = padded_headers
                .get(i)
                .and_then(|l| l.get(line_idx))
                .map(String::as_str)
                .unwrap_or("");
            row.push_str(&pad_cell(line, width, alignment, None));
            if i + 1 < plan.columns.len() {
                row.push_str(" │ ");
            }
        }
        row.push_str(" │");
        result.push_str(&row);
        result.push('\n');
    }

    // Separator.
    result.push_str(&build_border(&widths, '├', '┼', '┤'));
    result.push('\n');

    // Data rows.
    for (row_idx, row) in data.iter().enumerate() {
        // Per-cell wrapped + vertically padded content.
        let mut cell_lines: Vec<Vec<String>> = Vec::with_capacity(plan.columns.len());
        let mut row_height = 1usize;
        for (i, cp) in plan.columns.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(0);
            let column = columns.get(cp.original_index);
            let strategy = column
                .map(TableColumn::effective_word_wrap)
                .unwrap_or(WordWrap::None);
            let content = row
                .get(cp.original_index)
                .map(ToString::to_string)
                .unwrap_or_default();
            let wrapped = wrap_cell_content(&content, &strategy, width);
            // Apply the table's body slot style as ANSI per wrapped line so
            // each line stays self-contained across padding and striping.
            let wrapped: Vec<String> = match body_sgr {
                Some(sgr) => wrapped
                    .into_iter()
                    .map(|line| {
                        if line.is_empty() {
                            line
                        } else {
                            format!("{sgr}{line}{}", style::SGR_RESET)
                        }
                    })
                    .collect(),
                None => wrapped,
            };
            row_height = row_height.max(wrapped.len());
            cell_lines.push(wrapped);
        }
        let cell_lines: Vec<Vec<String>> = cell_lines
            .into_iter()
            .enumerate()
            .map(|(i, lines)| {
                let cp = &plan.columns[i];
                let width = widths.get(i).copied().unwrap_or(0);
                let vertical = columns
                    .get(cp.original_index)
                    .map(|c| c.vertical_align)
                    .unwrap_or(VerticalAlign::Top);
                apply_vertical_padding(lines, row_height, vertical, width)
            })
            .collect();

        let is_striped = (stripe_bg.is_some() || stripe_fg.is_some()) && row_idx % 2 == 1;
        let active_bg = if is_striped { stripe_bg } else { None };
        let active_fg = if is_striped { stripe_fg } else { None };

        for line_idx in 0..row_height {
            let mut row_str = String::new();
            row_str.push('│');
            if let Some(bg) = active_bg {
                row_str.push_str(bg);
            }
            if let Some(fg) = active_fg {
                row_str.push_str(fg);
            }
            row_str.push(' ');
            for (i, cp) in plan.columns.iter().enumerate() {
                let width = widths.get(i).copied().unwrap_or(0);
                let alignment = columns
                    .get(cp.original_index)
                    .map(TableColumn::effective_alignment)
                    .unwrap_or(Alignment::Left);
                let line = cell_lines
                    .get(i)
                    .and_then(|l| l.get(line_idx))
                    .map(String::as_str)
                    .unwrap_or("");
                if is_striped {
                    let mut restore = String::new();
                    if let Some(bg) = active_bg {
                        restore.push_str(bg);
                    }
                    if let Some(fg) = active_fg {
                        restore.push_str(fg);
                    }
                    let mut patched = line.to_string();
                    if !restore.is_empty() && patched.contains("\x1b[") {
                        patched = patched.replace("\x1b[0m", &format!("\x1b[0m{restore}"));
                        if let Some(bg) = active_bg {
                            patched = patched.replace("\x1b[49m", &format!("\x1b[49m{bg}"));
                        }
                    }
                    patched.push_str(&restore);
                    row_str.push_str(&pad_cell(&patched, width, alignment, None));
                } else {
                    row_str.push_str(&pad_cell(line, width, alignment, None));
                }
                if i + 1 < plan.columns.len() {
                    row_str.push_str(" │ ");
                }
            }
            row_str.push(' ');
            if active_bg.is_some() {
                row_str.push_str(BG_RESET);
            }
            if active_fg.is_some() {
                row_str.push_str(FG_RESET);
            }
            row_str.push('│');
            result.push_str(&row_str);
            result.push('\n');
        }
    }

    // Bottom border.
    result.push_str(&build_border(&widths, '└', '┴', '┘'));

    // Drop notes appended after the table.
    for note in &plan.dropped_notes {
        result.push_str(&format!("\n- {note}"));
    }

    result
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

    /// Builds render options with a terminal of explicit color / nerd-font
    /// capabilities.
    fn opts_with(color_depth: ColorDepth, nerd_font: Option<bool>) -> TerminalRenderOptions {
        let mut term = Terminal::new_optimistic(80);
        term.color_depth = color_depth;
        term.is_nerd_font = nerd_font;
        TerminalRenderOptions::new(&term, RenderStrictness::Warn)
    }

    // ── RT-COMPOSE-001: sequence join ──────────────────────────────────────

    #[test]
    fn render_tree_root_without_sequence_join_keeps_blank_separators() {
        let root = RenderNode::root(vec![
            RenderNode::paragraph(vec![RenderNode::text("foo")]),
            RenderNode::paragraph(vec![RenderNode::text("bar")]),
        ]);
        let out = strip_escape_codes(&render(&root).output);
        assert_eq!(out, "foo\n\nbar");
    }

    #[test]
    fn render_tree_root_with_sequence_join_has_no_separator() {
        let mut root = RenderNode::root(vec![
            RenderNode::text("foo"),
            RenderNode::text("bar"),
        ]);
        root.attrs.set_sequence_join(renderable::tree::SequenceJoin::None);
        let out = strip_escape_codes(&render(&root).output);
        assert_eq!(out, "foobar");
    }

    #[test]
    fn render_tree_sequence_join_mixed_inline_and_block() {
        let mut root = RenderNode::root(vec![
            RenderNode::text("inline"),
            RenderNode::paragraph(vec![RenderNode::text("para")]),
        ]);
        root.attrs.set_sequence_join(renderable::tree::SequenceJoin::None);
        let out = strip_escape_codes(&render(&root).output);
        assert_eq!(out, "inlinepara");
    }

    // ── RT-FILESYSTEM-001: list marker policy ──────────────────────────────

    fn item(text: &str, nested: Vec<RenderNode>) -> RenderNode {
        let mut children = vec![RenderNode::paragraph(vec![RenderNode::text(text)])];
        children.extend(nested);
        RenderNode::list_item(None, children)
    }

    #[test]
    fn render_tree_list_default_policy_unchanged() {
        let list = RenderNode::list(
            false,
            None,
            vec![item("a", vec![]), item("b", vec![])],
        );
        let out = strip_escape_codes(&render(&list).output);
        assert_eq!(out, "- a\n- b");
    }

    #[test]
    fn render_tree_list_marker_policy_none_emits_no_marker() {
        let mut list = RenderNode::list(
            false,
            None,
            vec![item("a", vec![]), item("b", vec![])],
        );
        list.attrs
            .set_list_marker_policy(renderable::tree::ListMarkerPolicy::None);
        let out = strip_escape_codes(&render(&list).output);
        assert_eq!(out, "a\nb");
    }

    #[test]
    fn render_tree_list_tree_connectors_branch_and_last_child() {
        let mut list = RenderNode::list(
            false,
            None,
            vec![item("first", vec![]), item("last", vec![])],
        );
        list.attrs
            .set_list_marker_policy(renderable::tree::ListMarkerPolicy::TreeConnectors);
        let out = strip_escape_codes(&render(&list).output);
        assert_eq!(out, "├── first\n└── last");
    }

    #[test]
    fn render_tree_list_tree_connectors_single_child() {
        let mut list = RenderNode::list(false, None, vec![item("only", vec![])]);
        list.attrs
            .set_list_marker_policy(renderable::tree::ListMarkerPolicy::TreeConnectors);
        let out = strip_escape_codes(&render(&list).output);
        assert_eq!(out, "└── only");
    }

    #[test]
    fn render_tree_list_tree_connectors_nested_continuation_lines() {
        // A nested list under a non-last item keeps a `│` rail; under the last
        // item the descendants align under blank space.
        let inner_under_first = {
            let mut inner = RenderNode::list(false, None, vec![item("child", vec![])]);
            inner
                .attrs
                .set_list_marker_policy(renderable::tree::ListMarkerPolicy::TreeConnectors);
            inner
        };
        let inner_under_last = {
            let mut inner = RenderNode::list(false, None, vec![item("leaf", vec![])]);
            inner
                .attrs
                .set_list_marker_policy(renderable::tree::ListMarkerPolicy::TreeConnectors);
            inner
        };
        let mut list = RenderNode::list(
            false,
            None,
            vec![
                item("dir1", vec![inner_under_first]),
                item("dir2", vec![inner_under_last]),
            ],
        );
        list.attrs
            .set_list_marker_policy(renderable::tree::ListMarkerPolicy::TreeConnectors);
        let out = strip_escape_codes(&render(&list).output);
        assert_eq!(
            out,
            "├── dir1\n│   └── child\n└── dir2\n    └── leaf"
        );
    }

    // ── RT-TODO-001: task-state hints ──────────────────────────────────────

    fn task_item(state: renderable::tree::TaskState, text: &str) -> RenderNode {
        let mut node = RenderNode::list_item(
            Some(matches!(state, renderable::tree::TaskState::Completed)),
            vec![RenderNode::paragraph(vec![RenderNode::text(text)])],
        );
        node.attrs
            .set_task_hints(&renderable::tree::TaskHints { state });
        node
    }

    #[test]
    fn render_tree_task_hint_marker_nerd_font_all_states() {
        use renderable::tree::TaskState;
        let opts = opts_with(ColorDepth::TrueColor, Some(true));
        for state in [
            TaskState::Open,
            TaskState::InProgress,
            TaskState::Completed,
            TaskState::Blocked,
            TaskState::Cancelled,
        ] {
            let list = RenderNode::list(false, None, vec![task_item(state, "task")]);
            let out = render_terminal_node(&list, &opts).expect("render").output;
            let plain = strip_escape_codes(&out);
            // The Nerd Font marker replaces the default `[ ]`/`[x]` checkbox.
            assert!(!plain.contains("[ ]"), "{state:?}: {plain:?}");
            assert!(!plain.contains("[x]"), "{state:?}: {plain:?}");
            assert!(plain.contains("task"), "{state:?}: {plain:?}");
        }
    }

    #[test]
    fn render_tree_task_hint_marker_no_color_fallback() {
        use renderable::tree::TaskState;
        let opts = opts_with(ColorDepth::None, Some(false));
        let expected = [
            (TaskState::Open, "[ ]"),
            (TaskState::InProgress, "[>]"),
            (TaskState::Completed, "[x]"),
            (TaskState::Blocked, "[!]"),
            (TaskState::Cancelled, "[-]"),
        ];
        for (state, marker) in expected {
            let list = RenderNode::list(false, None, vec![task_item(state, "task")]);
            let out = render_terminal_node(&list, &opts).expect("render").output;
            let plain = strip_escape_codes(&out);
            // The list bullet `- ` precedes the state marker.
            assert_eq!(plain, format!("- {marker} task"), "{state:?}");
        }
    }

    #[test]
    fn render_tree_task_hint_marker_color_fallback() {
        use renderable::tree::TaskState;
        // Color but no Nerd Font: the colored ASCII fallback is used.
        let opts = opts_with(ColorDepth::TrueColor, Some(false));
        let list = RenderNode::list(
            false,
            None,
            vec![task_item(TaskState::Completed, "done")],
        );
        let out = render_terminal_node(&list, &opts).expect("render").output;
        // The fallback for Completed contains a ✔ glyph.
        assert!(out.contains('✔'), "{out:?}");
        assert!(strip_escape_codes(&out).contains("done"));
    }

    #[test]
    fn render_tree_task_hint_marker_color_fallback_all_states() {
        use renderable::tree::TaskState;
        // Color but no Nerd Font: the colored ASCII fallback is used for
        // every state. Escape codes are stripped so the glyph is asserted
        // exactly. `InProgress` and `Blocked` share the `⏺` glyph (they
        // differ only by color), the rest are distinct.
        let opts = opts_with(ColorDepth::TrueColor, Some(false));
        let expected = [
            (TaskState::Open, "[ ]"),
            (TaskState::InProgress, "[⏺]"),
            (TaskState::Completed, "[✔]"),
            (TaskState::Blocked, "[⏺]"),
            (TaskState::Cancelled, "[-]"),
        ];
        for (state, marker) in expected {
            let list = RenderNode::list(false, None, vec![task_item(state, "task")]);
            let out = render_terminal_node(&list, &opts).expect("render").output;
            let plain = strip_escape_codes(&out);
            assert_eq!(plain, format!("- {marker} task"), "{state:?}");
        }
    }

    #[test]
    fn render_tree_default_task_list_without_hints_unchanged() {
        // A plain task list with no task hints keeps the GFM checkbox.
        let list = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(
                Some(false),
                vec![RenderNode::paragraph(vec![RenderNode::text("todo")])],
            )],
        );
        let out = strip_escape_codes(&render(&list).output);
        assert_eq!(out, "- [ ] todo");
    }

    // ── RT-TABLE-001: table title ──────────────────────────────────────────

    fn titled_table(title: &str) -> RenderNode {
        let mut table = RenderNode::table(
            vec![ColumnAlign::Left],
            vec![
                RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text(
                    "Name",
                )])]),
                RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text(
                    "Ann",
                )])]),
            ],
        );
        table.attrs.set_table_title(title);
        table
    }

    #[test]
    fn render_tree_table_title_above_top_border() {
        let out = strip_escape_codes(&render(&titled_table("Roster")).output);
        let first = out.lines().next().unwrap_or_default();
        assert_eq!(first.trim(), "Roster");
        // The title line precedes the table's top border.
        assert!(out.contains("Roster\n┌"), "{out:?}");
    }

    #[test]
    fn render_tree_table_whitespace_title_ignored() {
        let out = strip_escape_codes(&render(&titled_table("   ")).output);
        assert!(out.starts_with('┌'), "{out:?}");
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
        let node = RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text(
            "quoted",
        )])]);
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
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.message.contains("mystery"))
        );
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
        let node = RenderNode::paragraph(vec![RenderNode::image("pic.png", None, "a cat")]);
        let out = render(&node);
        assert!(strip_escape_codes(&out.output).contains("[a cat]"));
        assert!(out.diagnostics.iter().any(|d| d.message.contains("image")));
    }

    #[test]
    fn render_tree_invalid_tree_errors_before_output() {
        // An orphaned TableCell inside a Paragraph is a structural error.
        let bad = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::table_cell(
            vec![RenderNode::text("x")],
        )])]);
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
            root: RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("body")])]),
        };
        let out = render_terminal_document(&doc, &opts(RenderStrictness::Warn)).expect("render");
        assert!(strip_escape_codes(&out.output).contains("body"));
    }

    #[test]
    fn render_tree_section_renders_heading_and_body() {
        let section = RenderNode::section(
            HeadingDepth::new(2).unwrap(),
            vec![RenderNode::text("Section Title")],
            vec![RenderNode::paragraph(vec![RenderNode::text(
                "Body content",
            )])],
        );
        let out = render(&section);
        let plain = strip_escape_codes(&out.output);
        assert!(plain.contains("Section Title"));
        assert!(plain.contains("Body content"));
    }

    #[test]
    fn render_tree_section_with_empty_body_renders_heading_only() {
        let section = RenderNode::section(
            HeadingDepth::new(3).unwrap(),
            vec![RenderNode::text("Just a heading")],
            vec![],
        );
        let out = render(&section);
        let plain = strip_escape_codes(&out.output);
        assert!(plain.contains("Just a heading"));
    }

    #[test]
    fn render_tree_section_with_styled_heading() {
        let section = RenderNode::section(
            HeadingDepth::new(1).unwrap(),
            vec![
                RenderNode::text("Hello "),
                RenderNode::strong(vec![RenderNode::text("World")]),
            ],
            vec![],
        );
        let out = render(&section);
        // Check for bold SGR code in the output.
        assert!(out.output.contains("\x1b[1m"));
        let plain = strip_escape_codes(&out.output);
        assert!(plain.contains("Hello"));
        assert!(plain.contains("World"));
    }

    #[test]
    fn render_tree_nested_sections_render_correctly() {
        let tree = RenderNode::root(vec![RenderNode::section(
            HeadingDepth::new(1).unwrap(),
            vec![RenderNode::text("Parent")],
            vec![
                RenderNode::paragraph(vec![RenderNode::text("Intro")]),
                RenderNode::section(
                    HeadingDepth::new(2).unwrap(),
                    vec![RenderNode::text("Child")],
                    vec![RenderNode::paragraph(vec![RenderNode::text("Content")])],
                ),
            ],
        )]);
        let out = render(&tree);
        let plain = strip_escape_codes(&out.output);
        assert!(plain.contains("Parent"));
        assert!(plain.contains("Intro"));
        assert!(plain.contains("Child"));
        assert!(plain.contains("Content"));
    }

    #[test]
    fn render_tree_max_width_caps_content_width() {
        use renderable::layout::{Layout, Length, TargetValue};

        let term = Terminal::new_optimistic(80);
        let mut para = RenderNode::paragraph(vec![RenderNode::text("hello world")]);
        para.attrs.set_layout(&Layout {
            max_width: Some(TargetValue::universal(Length::ch(20))),
            ..Layout::default()
        });
        let tree = RenderNode::root(vec![para]);
        let out = render_terminal_node(
            &tree,
            &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
        )
        .expect("render");
        let plain = strip_escape_codes(&out.output);
        assert!(plain.contains("hello world"));
        let max_line = plain.lines().map(|l| l.len()).max().unwrap_or(0);
        assert!(
            max_line <= 20,
            "max_width=20 should cap lines to 20ch, got {max_line}"
        );
    }

    #[test]
    fn render_tree_max_width_with_center_alignment() {
        use renderable::layout::{Alignment, Layout, Length, TargetValue};

        let term = Terminal::new_optimistic(80);
        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            max_width: Some(TargetValue::universal(Length::ch(10))),
            alignment: Alignment::Center,
            ..Layout::default()
        });
        let tree = RenderNode::root(vec![para]);
        let out = render_terminal_node(
            &tree,
            &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
        )
        .expect("render");
        let plain = strip_escape_codes(&out.output);
        let line = plain.lines().next().unwrap_or("");
        assert!(
            line.starts_with(' '),
            "center-aligned content with max_width should have leading space, got: {line:?}"
        );
    }

    #[test]
    fn render_tree_max_width_with_margins() {
        use renderable::layout::{Layout, Length, Margin, TargetValue};

        let term = Terminal::new_optimistic(80);
        let mut para = RenderNode::paragraph(vec![RenderNode::text("hello")]);
        para.attrs.set_layout(&Layout {
            margin: Margin::x(Length::ch(4)),
            max_width: Some(TargetValue::universal(Length::ch(20))),
            ..Layout::default()
        });
        let tree = RenderNode::root(vec![para]);
        let out = render_terminal_node(
            &tree,
            &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
        )
        .expect("render");
        let plain = strip_escape_codes(&out.output);
        let line = plain.lines().next().unwrap_or("");
        assert!(
            line.starts_with("    "),
            "4ch left margin should produce 4 leading spaces, got: {line:?}"
        );
    }

    #[test]
    fn render_tree_nested_layout_composition_under_max_width() {
        use renderable::layout::{Alignment, Layout, Length, Margin, TargetValue};

        let term = Terminal::new_optimistic(80);

        // A root holding a block with both `max_width` and margins, so the
        // child resolves against the width left by the parent's constraints.
        let mut block =
            RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text(
                "content",
            )])]);
        block.attrs.set_layout(&Layout {
            margin: Margin::x(Length::ch(2)),
            max_width: Some(TargetValue::universal(Length::ch(30))),
            alignment: Alignment::Right,
            ..Layout::default()
        });
        let tree = RenderNode::root(vec![block]);
        let out = render_terminal_node(
            &tree,
            &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
        )
        .expect("render");
        let plain = strip_escape_codes(&out.output);
        assert!(
            plain.contains("content"),
            "nested content should still be rendered"
        );
        let max_line = plain.lines().map(|l| l.len()).max().unwrap_or(0);
        assert!(
            max_line <= 34,
            "inner 2ch margins + max_width 30 should keep lines <= 34, got {max_line}"
        );
    }

    #[test]
    fn render_tree_layout_on_inline_node_warn_strictness() {
        use renderable::layout::Layout;

        let term = Terminal::new_optimistic(80);

        let mut text = RenderNode::text("hello");
        text.attrs.set_layout(&Layout::default());
        let tree = RenderNode::root(vec![RenderNode::paragraph(vec![text])]);

        let out = render_terminal_node(
            &tree,
            &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
        )
        .expect("warn should succeed");
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.message.contains("block-level")),
            "warn strictness should produce a diagnostic about block-level"
        );
        assert!(strip_escape_codes(&out.output).contains("hello"));

        let out = render_terminal_node(
            &tree,
            &TerminalRenderOptions::new(&term, RenderStrictness::Lossy),
        )
        .expect("lossy should succeed");
        assert!(strip_escape_codes(&out.output).contains("hello"));

        let result = render_terminal_node(
            &tree,
            &TerminalRenderOptions::new(&term, RenderStrictness::Strict),
        );
        assert!(
            matches!(result, Err(RenderError::InvalidTree { .. })),
            "strict should escalate inline-layout warning to error"
        );
    }

    #[test]
    fn render_tree_applies_style_color_during_fold() {
        use renderable::color::{BasicColor, Color};
        use renderable::layout::TargetValue;
        use renderable::style::{PerMode, Style};

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hello")]);
        para.attrs.set_style(&Style {
            color: Some(TargetValue::universal(PerMode::universal(
                Color::BasicColor(BasicColor::Red),
            ))),
            ..Style::default()
        });
        let out = render(&para);
        assert!(out.output.contains("\x1b[31m"), "got {:?}", out.output);
        assert!(strip_escape_codes(&out.output).contains("hello"));
    }

    #[test]
    fn render_tree_applies_style_border_during_fold() {
        use renderable::style::{Border, BorderSides, Style};

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_style(&Style {
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            ..Style::default()
        });
        let out = render(&para);
        let plain = strip_escape_codes(&out.output);
        assert!(plain.contains('┌') && plain.contains('┘'), "got {plain:?}");
        // The bordered block stays within the available width.
        let widest = plain.split('\n').map(visible_width).max().unwrap_or(0);
        assert!(widest <= 80, "bordered block overflowed: {widest}");
    }

    /// Builds a universal foreground-color [`Style`] for a [`BasicColor`].
    #[cfg(test)]
    fn fg_style(color: renderable::color::BasicColor) -> Style {
        use renderable::color::Color;
        use renderable::layout::TargetValue;
        use renderable::style::PerMode;
        Style {
            color: Some(TargetValue::universal(PerMode::universal(
                Color::BasicColor(color),
            ))),
            ..Style::default()
        }
    }

    #[test]
    fn render_tree_applies_inline_span_style() {
        use renderable::color::BasicColor;

        // A `Span` carrying its own `Style` lowers that style to SGR around
        // just the span's text — the rest of the paragraph is unaffected.
        let mut span = RenderNode::span(vec![], vec![RenderNode::text("loud")]);
        span.attrs.set_style(&fg_style(BasicColor::Red));
        let para =
            RenderNode::paragraph(vec![RenderNode::text("a "), span, RenderNode::text(" b")]);
        let out = render(&para).output;
        assert!(out.contains("\x1b[31m"), "span color missing: {out:?}");
        let plain = strip_escape_codes(&out);
        assert!(plain.contains("a loud b"), "got {plain:?}");
    }

    #[test]
    fn render_tree_inline_span_inherits_parent_color() {
        use renderable::color::BasicColor;

        // An unstyled child span inside a colored paragraph inherits the
        // paragraph's color; a styled child span overrides it and the run
        // after the span is restored to the inherited color.
        let plain_span = RenderNode::span(vec![], vec![RenderNode::text("inherits")]);
        let mut green_span = RenderNode::span(vec![], vec![RenderNode::text("override")]);
        green_span.attrs.set_style(&fg_style(BasicColor::Green));
        let mut para = RenderNode::paragraph(vec![
            plain_span,
            RenderNode::text(" "),
            green_span,
            RenderNode::text(" tail"),
        ]);
        para.attrs.set_style(&fg_style(BasicColor::Red));

        let out = render(&para).output;
        // The paragraph color (red) and the span override (green) both appear.
        assert!(out.contains("\x1b[31m"), "parent red missing: {out:?}");
        assert!(out.contains("\x1b[32m"), "span green missing: {out:?}");
        // The span's close restores the inherited red so the tail is red again.
        let after_green = out.split("\x1b[32m").nth(1).expect("green run");
        assert!(
            after_green.contains("\x1b[31m"),
            "inherited color not restored after span: {out:?}"
        );
        assert!(strip_escape_codes(&out).contains("inherits override tail"));
    }

    #[test]
    fn render_tree_inline_span_inherits_ancestor_block_color() {
        use renderable::color::BasicColor;

        // A styled ancestor *block* (a red `BlockQuote`) must propagate its
        // color to descendant paragraphs and inline spans: a green child span
        // overrides it, and the run after the span is restored to the
        // ancestor block's red — not to the terminal default.
        let mut green_span = RenderNode::span(vec![], vec![RenderNode::text("override")]);
        green_span.attrs.set_style(&fg_style(BasicColor::Green));
        let paragraph = RenderNode::paragraph(vec![
            RenderNode::text("head "),
            green_span,
            RenderNode::text(" tail"),
        ]);
        let mut quote = RenderNode::block_quote(vec![paragraph]);
        quote.attrs.set_style(&fg_style(BasicColor::Red));

        let out = render(&quote).output;
        assert!(out.contains("\x1b[31m"), "ancestor red missing: {out:?}");
        assert!(out.contains("\x1b[32m"), "span green missing: {out:?}");
        // The span's close restores the inherited ancestor red.
        let after_green = out.split("\x1b[32m").nth(1).expect("green run");
        assert!(
            after_green.contains("\x1b[31m"),
            "ancestor block color not restored after span: {out:?}"
        );
        assert!(strip_escape_codes(&out).contains("head override tail"));
    }

    #[test]
    fn render_tree_inline_span_emphasis_does_not_leak_box_layers() {
        use renderable::style::{Border, BorderSides, Fill, Style, TextEmphasis};

        // Border and fill are box-painting layers with no inline meaning: a
        // `Span` declaring them must not draw box-drawing glyphs inline.
        let mut span = RenderNode::span(vec![], vec![RenderNode::text("word")]);
        span.attrs.set_style(&Style {
            emphasis: TextEmphasis {
                bold: true,
                ..TextEmphasis::default()
            },
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            fill: Some(Fill::default()),
            ..Style::default()
        });
        let para = RenderNode::paragraph(vec![RenderNode::text("x "), span]);
        let out = render(&para).output;
        assert!(out.contains("\x1b[1m"), "span bold missing: {out:?}");
        let plain = strip_escape_codes(&out);
        assert!(
            !plain.contains('┌') && !plain.contains('│'),
            "inline span must not draw a border: {plain:?}"
        );
    }

    // ── Table slot styling (Spec B D5) ─────────────────────────────────

    /// Builds a one-column, one-row table render tree from a `Table`.
    fn table_tree(table: &crate::components::table::Table) -> RenderNode {
        use crate::components::renderable::TerminalRenderable;
        table.render_tree_node().expect("table projects to a node")
    }

    #[test]
    fn render_tree_table_header_slot_emits_bold_sgr() {
        use renderable::style::{Style, TextEmphasis};

        let table = crate::components::table::Table::new()
            .with_columns(vec![crate::components::table::TableColumn::new("Name")])
            .with_data(vec![vec!["Ann".into()]])
            .with_header_style(Style {
                emphasis: TextEmphasis {
                    bold: true,
                    ..TextEmphasis::default()
                },
                ..Style::default()
            });
        let out = render(&table_tree(&table)).output;
        assert!(
            out.contains("\x1b[1m"),
            "styled header must emit bold SGR: {out:?}"
        );
        assert!(strip_escape_codes(&out).contains("Name"));
    }

    #[test]
    fn render_tree_table_body_slot_emits_color_sgr() {
        use renderable::color::{BasicColor, Color};
        use renderable::layout::TargetValue;
        use renderable::style::{PerMode, Style};

        let table = crate::components::table::Table::new()
            .with_columns(vec![crate::components::table::TableColumn::new("Name")])
            .with_data(vec![vec!["Ann".into()]])
            .with_body_style(Style {
                color: Some(TargetValue::universal(PerMode::universal(
                    Color::BasicColor(BasicColor::Red),
                ))),
                ..Style::default()
            });
        let out = render(&table_tree(&table)).output;
        assert!(
            out.contains("\x1b[31m"),
            "styled body must emit red fg SGR: {out:?}"
        );
        assert!(strip_escape_codes(&out).contains("Ann"));
    }

    #[test]
    fn render_tree_table_cell_slot_emits_sgr_for_standalone_cell() {
        use renderable::style::{Style, TextEmphasis};

        // The standalone `TableCell` render arm must honor a declared slot
        // style rather than flattening through `Style::default()`. A bare
        // `TableCell` is rejected by tree validation, so the arm is exercised
        // by driving the `Writer` directly.
        let mut cell = RenderNode::table_cell(vec![RenderNode::text("Cell")]);
        cell.attrs.set_style(&Style {
            emphasis: TextEmphasis {
                bold: true,
                ..TextEmphasis::default()
            },
            ..Style::default()
        });
        let options = opts(RenderStrictness::Warn);
        let mut writer = Writer {
            opts: &options,
            diagnostics: Vec::new(),
            effective: Style::default(),
        };
        let out = writer.render(&cell).expect("render cell");
        assert!(
            out.contains("\x1b[1m"),
            "standalone styled cell must emit bold SGR: {out:?}"
        );
    }

    /// Review-4 finding 2: a `NodeKind::ThematicBreak` carrying
    /// `darkmatter.hr.*` hints must render the corresponding styled rule,
    /// not collapse to the default dashed rule. Without this wiring the
    /// fold's HR-attribute storage would never reach the user's terminal.
    ///
    /// The test uses a text-tier terminal (`ImageSupport::None`) so the
    /// `HorizontalRule`'s SVG-to-Kitty image tier is bypassed and the
    /// glyph-based output is observable.
    #[test]
    fn render_tree_thematic_break_consumes_darkmatter_hr_hints() {
        use crate::discovery::detection::ImageSupport;
        use renderable::tree::HintNamespace;

        let mut hr = RenderNode::thematic_break();
        let ns = HintNamespace("darkmatter.hr");
        hr.attrs.set_hint(ns, "style", serde_json::json!("waves"));

        let term = Terminal::builder()
            .width(40)
            .image_support(ImageSupport::None)
            .build();
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        let out = render_terminal_node(&hr, &opts).expect("render hr").output;
        // The waves rule emits `≋` (U+224B) in Unicode-capable terminals,
        // `~` in ASCII fallback. Either waves marker proves the hint was
        // honored instead of falling back to the default dashed rule.
        assert!(
            out.contains('\u{224B}') || out.contains('~'),
            "expected waves rule glyph; got: {out:?}"
        );
        assert!(
            !out.contains('\u{2500}'),
            "default dashed rule glyph leaked despite waves hint: {out:?}"
        );
    }

    /// A `NodeKind::ThematicBreak` with no hints must still render through
    /// the default `HorizontalRule` path — the hint-consumer change cannot
    /// regress plain rules.
    #[test]
    fn render_tree_thematic_break_without_hints_uses_default_rule() {
        use crate::discovery::detection::ImageSupport;
        let hr = RenderNode::thematic_break();
        let term = Terminal::builder()
            .width(40)
            .image_support(ImageSupport::None)
            .build();
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        let out = render_terminal_node(&hr, &opts).expect("render hr").output;
        assert!(!out.is_empty(), "default rule must produce output");
    }

    #[test]
    fn render_tree_table_new_with_bold_reaches_parity_with_bespoke() {
        use crate::components::renderable::TerminalRenderable;
        use crate::components::table::{Table, TableColumn};

        // A bold-header table must render identically through the render-tree
        // path and the bespoke `Table::render` path.
        let build = || {
            Table::new()
                .with_columns(vec![
                    TableColumn::new_with_bold("Name"),
                    TableColumn::new_with_bold("Score"),
                ])
                .with_data(vec![vec!["Ann".into(), "90".into()]])
        };
        let term = Terminal::new_optimistic(80);
        let bespoke = build().render(&term);

        let tree_out = render(&build().render_tree_node().expect("table node")).output;

        assert_eq!(
            tree_out, bespoke,
            "tree path must match bespoke output for a bold-header table"
        );
        // And the styling must actually be present (not flattened away).
        assert!(tree_out.contains("\x1b[1m"), "bold SGR missing: {tree_out:?}");
    }
}
