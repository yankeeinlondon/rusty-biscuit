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
use renderable::tree::{
    ColumnAlign, ColumnConditional, Diagnostic, Document, NodeKind, ProgressHints, RenderError,
    RenderNode, RenderStrictness, Rendered, Severity, TableColumnHints,
};
use renderable::tree::{ValidationError, ValidationMode, validate};

use crate::components::block_quote::BlockQuote;
use crate::components::horizontal_rule::HorizontalRule;
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
use crate::utils::block_constraint::visible_width;
use crate::utils::layout::{Alignment, WordWrap};

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
    ///
    /// When the node carries a [`Layout`](renderable::layout::Layout), its
    /// margins and alignment are applied around the rendered content: the
    /// child width is reduced by the horizontal margins, each produced line is
    /// prefixed by the left margin (plus any alignment offset), and the
    /// vertical margins emit leading/trailing blank lines.
    fn render(&mut self, node: &RenderNode) -> Result<String, RenderError> {
        match node.attrs.layout() {
            Some(layout) => self.render_with_layout(node, &layout),
            None => self.render_kind(node),
        }
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
        let content_width = available.saturating_sub(left + right).max(1);
        let content = {
            let mut narrowed = self.opts.clone();
            narrowed.context.available_width = content_width;
            narrowed.context.width = content_width;
            narrowed.context.terminal.fixed_width = Some(content_width);
            let mut sub = Writer {
                opts: &narrowed,
                diagnostics: Vec::new(),
            };
            let rendered = sub.render_kind(node);
            self.diagnostics.append(&mut sub.diagnostics);
            rendered?
        };

        // Alignment offset: extra left padding when the content is narrower
        // than the space available between the horizontal margins.
        let widest = content
            .split('\n')
            .map(visible_width)
            .max()
            .unwrap_or(0);
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
            NodeKind::Root { children } => self.render_blocks(children),
            NodeKind::Heading { depth, children } => {
                let markup = self.render_inline(children)?;
                Ok(self.render_heading_line(depth.get(), &markup))
            }
            NodeKind::Section {
                depth,
                heading,
                children,
            } => {
                let markup = self.render_inline(heading)?;
                let heading_output = self.render_heading_line(depth.get(), &markup);
                let body = self.render_blocks(children)?;
                if body.is_empty() {
                    Ok(heading_output)
                } else {
                    Ok(format!("{heading_output}\n\n{body}"))
                }
            }
            NodeKind::Paragraph { children } => {
                let markup = self.render_inline(children)?;
                if let Some(hints) = node.attrs.progress_hints() {
                    Ok(self.render_prose(&render_progress_bar(&hints, &markup)))
                } else {
                    Ok(self.render_prose(&markup))
                }
            }
            NodeKind::BlockQuote { children } => {
                if let Some(hints) = node.attrs.columns_hints() {
                    self.render_columns(children, &hints)
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
            // `meta` (info-string beyond the language) is intentionally
            // ignored: syntax highlighting and meta handling are out of
            // scope for this phase.
            NodeKind::Code {
                lang,
                meta: _,
                value,
            } => Ok(self.render_code_node(lang.as_deref(), value, &node.attrs)),
            NodeKind::ThematicBreak => {
                let rule = HorizontalRule::new();
                Ok(rule.render(&self.opts.context.terminal))
            }
            NodeKind::Table { align, children } => self.render_table(align, children, node),
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

        let left = self.render_blocks_in_width(left_children, left_width.max(1))?;
        let right = self.render_blocks_in_width(right_children, right_width.max(1))?;

        let left_lines: Vec<&str> = left.split('\n').collect();
        let right_lines: Vec<&str> = right.split('\n').collect();
        let rows = left_lines.len().max(right_lines.len());
        let gutter = " ".repeat(hints.gap as usize);

        let mut out = Vec::with_capacity(rows);
        for i in 0..rows {
            let l = left_lines.get(i).copied().unwrap_or("");
            let r = right_lines.get(i).copied().unwrap_or("");
            let pad = left_width.saturating_sub(visible_width(l));
            out.push(format!("{l}{}{gutter}{r}", " ".repeat(pad as usize)));
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
            NodeKind::Link {
                url,
                title: _,
                children,
            } => {
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
    /// Matches the bespoke `Section` heading output: a Markdown-style prefix
    /// (`# `..`###### `) plus the title, wrapped in a heading SGR style — bold
    /// (`\x1b[1m`/`\x1b[22m`) for depths 1-3, italic (`\x1b[3m`/`\x1b[23m`) for
    /// depths 4-5, and plain for depth 6.
    fn render_heading_line(&self, depth: u8, markup: &str) -> String {
        let (prefix, style_open, style_close) = match depth {
            1 => ("# ", "\x1b[1m", "\x1b[22m"),
            2 => ("## ", "\x1b[1m", "\x1b[22m"),
            3 => ("### ", "\x1b[1m", "\x1b[22m"),
            4 => ("#### ", "\x1b[3m", "\x1b[23m"),
            5 => ("##### ", "\x1b[3m", "\x1b[23m"),
            _ => ("###### ", "", ""),
        };
        let title = self.render_prose(markup);
        format!("{style_open}{prefix}{title}{style_close}")
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

        let check_marker = match checked {
            Some(true) => "[x] ",
            Some(false) => "[ ] ",
            None => "",
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
                    NodeKind::Paragraph { children } => self.render_inline(children)?,
                    _ => self.render_inline(std::slice::from_ref(child))?,
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
    /// the language, body, node attributes, and a [`TerminalCodeContext`]
    /// containing the available render width, color depth, and color mode.
    /// A `Some` result is used verbatim; a `None` result falls back to
    /// [`Self::render_code`].
    fn render_code_node(
        &self,
        lang: Option<&str>,
        value: &str,
        attrs: &renderable::tree::NodeAttrs,
    ) -> String {
        if let Some(renderer) = &self.opts.code_renderer {
            let context = TerminalCodeContext::new(
                self.opts.context.available_width,
                (&self.opts.context.color_depth).into(),
                (&self.opts.context.color_mode).into(),
            );
            if let Some(rendered) = renderer.render_terminal_code(lang, value, attrs, context) {
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
        let has_true_color = self.opts.context.color_depth == ColorDepth::TrueColor;
        let stripe_bg = if terminal_hints.alternate_background && has_true_color {
            Some(stripe_bg_escape(&self.opts.context.color_mode))
        } else {
            None
        };
        let stripe_fg = if terminal_hints.alternate_text_color && has_true_color {
            Some(stripe_fg_escape(&self.opts.context.color_mode))
        } else {
            None
        };

        Ok(emit_table(&columns, &data, &plan, stripe_bg, stripe_fg))
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
            let text = self.render_inline(cell_children)?;
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

/// Renders a progress bar from [`ProgressHints`], reproducing the bespoke
/// `Progress::render_bar()` output.
///
/// `paragraph_text` is the projected paragraph's visible text
/// (`"{label} {pct}%"` or `"{pct}%"`); any label portion is the text before
/// the trailing ` {pct}%` token and is preserved before the bar.
fn render_progress_bar(hints: &ProgressHints, paragraph_text: &str) -> String {
    let value = hints.value.clamp(0.0, 1.0);
    let percentage = (value * 100.0).round() as u32;
    let filled_count = ((value * hints.bar_width as f32).round() as u32).min(hints.bar_width);
    let empty_count = hints.bar_width.saturating_sub(filled_count);

    let bar = format!(
        "{}{}",
        hints.fill_char.to_string().repeat(filled_count as usize),
        hints.empty_char.to_string().repeat(empty_count as usize),
    );
    let percentage_str = format!("{percentage:3}%");

    // The label is everything before the trailing percentage token.
    let pct_suffix = format!(" {percentage}%");
    let label = paragraph_text
        .strip_suffix(&pct_suffix)
        .filter(|label| !label.is_empty());

    match label {
        Some(label) => format!(
            "{label} {}{bar}{} {percentage_str}",
            hints.left_bracket, hints.right_bracket
        ),
        None => format!(
            "{}{bar}{} {percentage_str}",
            hints.left_bracket, hints.right_bracket
        ),
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

/// Returns `true` for a semantic class with a defined terminal treatment.
fn is_known_class(class: &str) -> bool {
    matches!(class, "mark" | "dim" | "sup" | "sub")
}

/// Returns `true` when `node` is a paragraph or an inline-level node.
///
/// Inside a list item, such nodes flow on the prefixed line; every other
/// node is block-level and is indented without a prefix.
fn is_inline_block(node: &RenderNode) -> bool {
    matches!(
        node.kind,
        NodeKind::Paragraph { .. }
            | NodeKind::Text { .. }
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
}
