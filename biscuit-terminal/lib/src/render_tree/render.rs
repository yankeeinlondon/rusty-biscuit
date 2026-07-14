//! Terminal renderer for the canonical render tree.
//!
//! [`render_terminal_node`] folds a [`RenderNode`] into a terminal string,
//! and [`render_terminal_document`] does the same for a whole [`Document`].
//!
//! The renderer reuses biscuit-terminal's existing components wherever they
//! fit — [`Table`], [`BlockQuote`], and [`HorizontalRule`] — rather than
//! re-implementing terminal formatting. Headings, sections, and lists are
//! rendered natively. Inline subtrees are lowered directly to ANSI SGR
//! escapes, OSC8 hyperlinks, and markdown fallback syntax.
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
use renderable::style::{Style, TextEmphasis};
use renderable::tree::{
    ColumnAlign, ColumnConditional, Diagnostic, Document, GraphicsMode, HrAlignment, HrKind,
    HrWeight, InheritedStyle, NodeKind, ProgressHints, RenderError, RenderNode, RenderStrictness,
    Rendered, Severity, TableColumnHints, TableTerminalHints, TextLayoutHints, TextOverflow,
};
#[cfg(feature = "image")]
use renderable::tree::TerminalMermaidMode;
use renderable::tree::{ValidationError, ValidationMode, validate};

use crate::components::block_quote::BlockQuote;
use crate::components::horizontal_rule::{HorizontalRule, RuleAlignment, RuleStyle, RuleWeight};
#[cfg(feature = "image")]
use crate::components::mermaid::MermaidDiagram;
use crate::components::prose::Prose;
use crate::components::renderable::TerminalRenderable;
use crate::components::terminal_image::TerminalImage;
use crate::discovery::detection::ImageSupport;
use crate::components::table::cell::pad_cell;
use crate::components::table::table::{
    BG_RESET, FG_RESET, apply_vertical_padding, build_border, stripe_bg_escape, stripe_fg_escape,
    wrap_cell_content,
};
use crate::components::table::{
    ColumnType, Conditional, Table, TableCellContent, TableColumn, TableWidthPlan, VerticalAlign,
};
use crate::discovery::detection::ColorDepth;
use crate::utils::block_constraint::{
    split_lines, split_trailing_escapes, visible_width, wrap_lines,
};
use crate::utils::layout::{Alignment, WordWrap};
use crate::utils::word_wrap::truncate;

use super::options::{ImagePlaceholder, TerminalRenderOptions};
use super::style;

/// Maximum size of an inline terminal image at [`GraphicsMode::Rich`], matching
/// the legacy terminal image renderer's 10 MiB ceiling.
const MAX_TERMINAL_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

/// The ellipsis used when typed [`TextLayoutHints`] truncate a link label or
/// image placeholder that exceeds its resolved field.
const ELLIPSIS: &str = "…";

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
        inherited: InheritedStyle::root(),
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

/// Deserialized payload for the `"icon"` extension token.
#[derive(serde::Deserialize)]
struct IconPayload {
    nerd_font: Option<String>,
    unicode: Option<String>,
    text: String,
    #[serde(default)]
    svg: String,
    #[serde(default)]
    nerd_font_preferred: bool,
}

/// Writes SVG markup to a temporary file and renders it via the terminal image
/// protocol.
fn render_svg_string(svg: &str, term: &crate::terminal::Terminal) -> std::io::Result<String> {
    use std::io::Write;

    let mut file = tempfile::Builder::new().suffix(".svg").tempfile()?;
    file.write_all(svg.as_bytes())?;
    let img = TerminalImage::new(file.path())
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(img.render(term))
}

/// Threads render options and accumulating diagnostics through the recursion.
struct Writer<'a> {
    opts: &'a TerminalRenderOptions,
    diagnostics: Vec<Diagnostic>,
    /// The text appearance (`color`, `emphasis`) inherited from styled
    /// ancestor blocks, carried by the shared [`InheritedStyle`] resolver. Per
    /// Spec B D6 only these fields inherit; the box-painting fields
    /// (`background`, `border`) never do. A styled node folds its own text
    /// appearance into this before rendering its subtree (see
    /// [`Writer::render_styled`]) so descendant paragraphs and inline spans see
    /// the ancestor color/emphasis.
    inherited: InheritedStyle,
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
        match node.attrs.layout_ref() {
            Some(layout) if !is_inline_kind(&node.kind) => self.render_with_layout(node, layout),
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
        let Some(style) = node.attrs.style_ref().filter(|s| !s.is_empty()) else {
            return self.render_kind(node);
        };

        let overhead = style::border_horizontal_overhead(style);
        let inner_width = self
            .opts
            .context
            .available_width
            .saturating_sub(overhead)
            .max(1);

        // The painted padding box. `render_with_layout` already subtracted the
        // horizontal padding from the content width, so painting it back here
        // reconstitutes the full box without double-counting.
        let padding = self.resolve_padding(node);

        // Enter this node through the shared resolver: its text appearance is
        // threaded into the child context for the duration of its subtree, so
        // descendant paragraphs and inline spans see the ancestor
        // color/emphasis (Spec B D6). The resolver carries only the inheriting
        // fields forward — the box-painting layers are cleared.
        let (child_ctx, _) = self.inherited.enter(Some(style));
        let prev = std::mem::replace(&mut self.inherited, child_ctx);

        let content = if overhead > 0 {
            self.render_kind_in_width(node, inner_width)
        } else {
            self.render_kind(node)
        };

        self.inherited = prev;
        let content = content?;

        // No `Layout` width mode here, so the box hugs its content (`None`).
        Ok(style::apply_style_with_padding(
            &content,
            style,
            &self.opts.context.terminal,
            None,
            padding,
        ))
    }

    /// Resolves a node's [`Layout`](renderable::layout::Layout) padding into
    /// whole terminal cells, against the current available width. Returns
    /// [`style::Padding::ZERO`] when the node carries no layout.
    fn resolve_padding(&self, node: &RenderNode) -> style::Padding {
        let available = self.opts.context.available_width;
        match node.attrs.layout_ref() {
            Some(layout) => style::Padding {
                top: resolve_cells(&layout.padding.top, available),
                right: resolve_cells(&layout.padding.right, available),
                bottom: resolve_cells(&layout.padding.bottom, available),
                left: resolve_cells(&layout.padding.left, available),
            },
            None => style::Padding::ZERO,
        }
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
            inherited: self.inherited.clone(),
        };
        let result = sub.render_kind(node);
        self.diagnostics.append(&mut sub.diagnostics);
        result
    }

    /// Renders `node`'s content within `content_width` and paints its `Style`
    /// text appearance, `padding` band, and border around it.
    ///
    /// `padding` is supplied pre-resolved (cells, parent basis) so a `%` padding
    /// is never re-resolved against the narrowed content width. The content is
    /// rendered at exactly `content_width`; the caller has already reserved the
    /// padding and border around it in the width budget, so the paint step adds
    /// those cells back. Padding is reserved and painted even when the node
    /// carries no non-empty `Style` — transparent CSS padding still occupies
    /// cells.
    fn render_painted(
        &mut self,
        node: &RenderNode,
        content_width: u32,
        padding: style::Padding,
    ) -> Result<String, RenderError> {
        let Some(style) = node.attrs.style_ref().filter(|s| !s.is_empty()) else {
            let content = self.render_kind_in_width(node, content_width)?;
            return Ok(style::apply_style_with_padding(
                &content,
                &Style::default(),
                &self.opts.context.terminal,
                Some(content_width),
                padding,
            ));
        };

        // Enter this node through the shared resolver: its text appearance is
        // threaded into the child context for the duration of its subtree, so
        // descendant paragraphs and inline spans see the ancestor
        // color/emphasis (Spec B D6). The resolver carries only the inheriting
        // fields forward — the box-painting layers are cleared.
        let (child_ctx, _) = self.inherited.enter(Some(style));
        let prev = std::mem::replace(&mut self.inherited, child_ctx);
        let content = self.render_kind_in_width(node, content_width);
        self.inherited = prev;
        let content = content?;

        Ok(style::apply_style_with_padding(
            &content,
            style,
            &self.opts.context.terminal,
            Some(content_width),
            padding,
        ))
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

        // Resolve `padding` into cells **once**, against the parent available
        // width. The painted box (`style.rs`) reuses these exact counts, so a
        // `%` padding has a single, stable basis and is never re-resolved
        // against the narrowed content width.
        let padding = self.resolve_padding(node);
        let padding_lr = padding.left + padding.right;

        // Drawn vertical border edges, reserved around the content box. The
        // paint step adds these glyph cells back, so the content renders at
        // exactly `content_width` and the border is added *around* it (a
        // `Fixed(n)` box keeps all `n` content columns).
        let border_lr = node
            .attrs
            .style_ref()
            .filter(|s| !s.is_empty())
            .map_or(0, style::border_horizontal_overhead);

        // Largest permitted content box: the space left after margin, padding,
        // and border are reserved (CSS box-order clamp:
        // `margin + border + padding + used ≤ available`). Clamp to at least 1:
        // a width-0 sub-render is degenerate for downstream components.
        let auto_cap = available
            .saturating_sub(left + right + padding_lr + border_lr)
            .max(1);

        // Resolve the content-box width from `layout.width`. `Auto` fills the
        // cap; `Fixed` resolves the requested length, clamped down to the cap;
        // `FitContent` runs a bounded measure pass — render once at the cap,
        // measure the widest visible line, and shrink to it.
        let mut content_width = match &layout.width {
            renderable::layout::Width::Auto => auto_cap,
            renderable::layout::Width::Fixed(tv) => {
                resolve_cells(tv, available).min(auto_cap).max(1)
            }
            renderable::layout::Width::FitContent => {
                self.measure_fit_content(node, auto_cap)?.min(auto_cap).max(1)
            }
        };

        // Apply max_width cap if declared. The resolved cap further narrows
        // the available width so children receive a reduced inner width and
        // alignment is observable within the capped region.
        if let Some(mw) = &layout.max_width {
            let cap = resolve_cells(mw, available);
            if cap > 0 {
                content_width = content_width.min(cap).max(1);
            }
        }

        // Render the content at exactly `content_width` and paint the `Style`,
        // `padding` band, and border around it. Padding is reserved and painted
        // even when the node carries no `Style`.
        let content = self.render_painted(node, content_width, padding)?;

        // Place the painted box within the content area between the margins
        // (`available − margin`). The painted box is `content_width` plus the
        // reserved padding and border. When that box is narrower than the area
        // — a sub-available `Fixed` / `FitContent` box, or an `Auto` box capped
        // by `max_width` — the alignment offset positions the whole box. When
        // the box fills the area, the offset instead centers the visible
        // content within it (an `Auto` block whose short content is centered).
        let box_width = content_width + padding_lr + border_lr;
        let align_area = available.saturating_sub(left + right);
        let widest = content.split('\n').map(visible_width).max().unwrap_or(0);
        let slack = if box_width < align_area {
            align_area - box_width
        } else {
            align_area.saturating_sub(widest)
        };
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

    /// Measures the natural content-box width for a [`Width::FitContent`] node.
    ///
    /// Renders the node's content once at `cap` — the largest permitted
    /// content box — and returns the widest visible line. This is the bounded
    /// measurement pass: `cap` (not an unbounded width) caps the throwaway
    /// render, so wrapping content cannot blow up the measured width. The
    /// caller clamps the result under `max_width` and the cap, then re-renders
    /// the content at the resolved width.
    ///
    /// The pass renders the node's *content* (via [`Self::render_kind`]) rather
    /// than its painted box, so the measured width excludes the `padding` and
    /// `border` that the painted box adds around it. Diagnostics from this
    /// throwaway pass are discarded — the caller's final content render
    /// re-emits them.
    ///
    /// [`Width::FitContent`]: renderable::layout::Width::FitContent
    fn measure_fit_content(&self, node: &RenderNode, cap: u32) -> Result<u32, RenderError> {
        let mut narrowed = self.opts.clone();
        narrowed.context.available_width = cap;
        narrowed.context.width = cap;
        narrowed.context.terminal.fixed_width = Some(cap);
        let mut sub = Writer {
            opts: &narrowed,
            diagnostics: Vec::new(),
            inherited: self.inherited.clone(),
        };
        let rendered = sub.render_kind(node)?;
        Ok(rendered.split('\n').map(visible_width).max().unwrap_or(0))
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
                let effective =
                    heading_effective(depth.get()).inherited_from(self.inherited.effective());
                let markup = self.render_inline(children, &effective)?;
                Ok(self.render_heading_line(depth.get(), &markup))
            }
            NodeKind::Section {
                depth,
                heading,
                children,
            } => {
                let effective =
                    heading_effective(depth.get()).inherited_from(self.inherited.effective());
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
                let effective = self.inherited.effective().clone();
                let markup = self.render_inline(children, &effective)?;
                if let Some(hints) = node.attrs.progress_hints_ref() {
                    // A progress bar is a single fixed-width glyph run; wrapping
                    // it would split the bar, so it is emitted as-is.
                    let bar = render_progress_bar(hints, &markup, self.opts.context.color_depth);
                    Ok(self.render_prose(&bar))
                } else if self.opts.context.wrap_prose {
                    // Top-level paragraphs wrap to the post-margin content width,
                    // matching the legacy `for_terminal` renderer's line wrapping.
                    // Off by default so component callers (which apply their own
                    // post-render wrapping) are unaffected.
                    Ok(self.wrap_prose(&markup))
                } else {
                    Ok(self.render_prose(&markup))
                }
            }
            NodeKind::BlockQuote { children } => {
                if let Some(hints) = node.attrs.columns_hints_ref() {
                    self.render_columns(children, hints)
                } else if node.attrs.style_ref().is_some_and(|s| !s.is_empty()) {
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
                } else if let Some(prefix) = self.opts.context.blockquote_prefix.clone() {
                    // The darkmatter document pipeline asks for its own quarter-
                    // block bar (`▐   `) instead of the component's `│ ` border.
                    // Children render at the reduced width so inner prose wraps
                    // inside the bar, then each line is prefixed; nested quotes
                    // recurse and prepend their own bar, matching the legacy
                    // renderer's per-depth `"▐   ".repeat(depth)`.
                    Ok(self.render_blockquote_with_prefix(children, &prefix)?)
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
            NodeKind::Code { lang, meta, value } => {
                self.render_code_node(lang.as_deref(), value, meta.as_deref(), &node.attrs)
            }
            NodeKind::ThematicBreak => {
                let rule = horizontal_rule_from_attrs(&node.attrs);
                let term = self.effective_terminal();
                // The terminal has no vector HR form, so `Vector` coincides with
                // `Off` (Unicode / ASCII). Only `Rich` rasterizes to an image.
                match self.opts.context.graphics_mode {
                    GraphicsMode::Off | GraphicsMode::Vector => Ok(rule.render_text_tier(&term)),
                    GraphicsMode::Rich => match rule.render_image_tier(&term) {
                        Some(image) => Ok(image),
                        None => Ok(rule.render_text_tier(&term)),
                    },
                }
            }
            NodeKind::Table { align, children } => {
                let table = self.render_table(align, children, node)?;
                // A table title/caption is emitted above the top border. An
                // empty or whitespace-only title is ignored.
                match node.attrs.table_title_ref() {
                    Some(title) if !title.trim().is_empty() => {
                        Ok(format!("{}\n{table}", title.trim()))
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
            | NodeKind::Extended { .. }
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
            NodeKind::Disclosure { summary, children, .. } => {
                self.render_disclosure(summary, children)
            }
            NodeKind::Unsupported { label } => self.render_unsupported(node, label),
        }
    }

    /// Renders a disclosure block: the summary is shown normally and the body is
    /// rendered as a block quote whose text is dim and italic.
    fn render_disclosure(
        &mut self,
        summary: &[RenderNode],
        children: &[RenderNode],
    ) -> Result<String, RenderError> {
        let effective = self.inherited.effective().clone();
        let summary_markup = self.render_inline(summary, &effective)?;
        let summary_line = self.render_prose(&summary_markup);

        let body = if children.is_empty() {
            String::new()
        } else {
            let dim_italic = Style {
                emphasis: TextEmphasis {
                    dim: true,
                    italic: true,
                    ..Default::default()
                },
                ..Style::default()
            };
            let (child_ctx, _) = self.inherited.enter(Some(&dim_italic));
            let prev = std::mem::replace(&mut self.inherited, child_ctx);
            let rendered = self.render_blocks(children);
            self.inherited = prev;
            let rendered = rendered?;
            // The inherited context above only restores dim/italic for nested
            // inline spans; plain body text is painted at the block level (as
            // `render_styled` does), so paint the appearance onto every line
            // before wrapping it as a block quote.
            let styled = style::apply_style(&rendered, &dim_italic, &self.opts.context.terminal);
            BlockQuote::from(styled.as_str()).render(&self.opts.context.terminal)
        };

        match (summary_line.is_empty(), body.is_empty()) {
            (true, true) => Ok(String::new()),
            (true, false) => Ok(body),
            (false, true) => Ok(summary_line),
            (false, false) => Ok(format!("{summary_line}\n\n{body}")),
        }
    }

    /// Renders a sequence of block-level nodes, joined by a blank line.
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
                    let effective = self.inherited.effective().clone();
                    let markup = self.render_inline_node(child, &effective)?;
                    output.push_str(&markup);
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
            inherited: self.inherited.clone(),
        };
        let result = sub.render_blocks(children);
        self.diagnostics.append(&mut sub.diagnostics);
        result
    }

    /// Renders a single `node` through the full [`Self::render`] path with the
    /// context narrowed to `width`.
    ///
    /// Mirrors [`Self::render_kind_in_width`] but keeps the layout/paint wrapper
    /// (`render`, not `render_kind`). Used when a block child is about to be
    /// indented (e.g. a nested list inside a list item): the child must wrap to
    /// the width left *after* that indent, or its lines overrun the terminal by
    /// the indent amount and the terminal hard-wraps the overflow.
    fn render_in_width(&mut self, node: &RenderNode, width: u32) -> Result<String, RenderError> {
        let mut narrowed = self.opts.clone();
        narrowed.context.available_width = width;
        narrowed.context.width = width;
        narrowed.context.terminal.fixed_width = Some(width);
        let mut sub = Writer {
            opts: &narrowed,
            diagnostics: Vec::new(),
            inherited: self.inherited.clone(),
        };
        let result = sub.render(node);
        self.diagnostics.append(&mut sub.diagnostics);
        result
    }

    /// Renders an unstyled block quote with a caller-supplied left-border
    /// `prefix` (the darkmatter `▐   ` bar) instead of the shared
    /// [`BlockQuote`] component's `│ ` border.
    ///
    /// The children render at the width left after the prefix so inner prose
    /// wraps inside the bar; each produced line is then prefixed with the bar
    /// painted in the legacy gray foreground. Nested block quotes recurse
    /// through the same path and prepend their own bar, reproducing the legacy
    /// renderer's per-depth `"▐   ".repeat(depth)`.
    fn render_blockquote_with_prefix(
        &mut self,
        children: &[RenderNode],
        prefix: &str,
    ) -> Result<String, RenderError> {
        let prefix_width = visible_width(prefix);
        let child_width = self
            .opts
            .context
            .available_width
            .saturating_sub(prefix_width)
            .max(1);
        let inner = self.render_blocks_in_width(children, child_width)?;

        // Gray foreground for the bar, matching the legacy renderer's
        // `\x1b[38;2;100;100;100m` quarter-block. The page-frame post-pass owns
        // the subtle background band, so the bar itself only carries its fg.
        let colored_prefix = format!("\x1b[38;2;100;100;100m{prefix}\x1b[39m");

        let mut out = String::new();
        for (idx, line) in inner.split('\n').enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            out.push_str(&colored_prefix);
            out.push_str(line);
        }
        Ok(out)
    }

    /// Renders a sequence of inline nodes into terminal SGR output.
    ///
    /// Each inline node is lowered directly to ANSI escapes without an
    /// intermediate Prose markup pass.
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

    /// Applies a node's typed [`TextLayoutHints`] to a single rendered visible
    /// run (a link label or an image alt-text placeholder).
    ///
    /// `width` establishes an exact field: shorter content is padded per
    /// `alignment`, and longer content is truncated with an ellipsis only when
    /// `overflow` is [`TextOverflow::Truncate`]. `max_width` is a hard ceiling:
    /// content exceeding it is always truncated with an ellipsis. When both are
    /// present, the field is the requested `width` clamped down to the
    /// `max_width` cap. All measurement and truncation is display-width aware —
    /// Unicode columns plus embedded SGR / OSC escapes — via [`visible_width`]
    /// and [`truncate`], so the tree's own text is never mutated; only this
    /// rendered projection is padded or shortened.
    fn apply_text_layout(&self, content: String, hints: &TextLayoutHints) -> String {
        let available = self.opts.context.available_width;
        let resolve = |tv: &renderable::layout::TargetValue<renderable::layout::Length>| {
            let cells = resolve_cells(tv, available);
            (cells > 0).then_some(cells)
        };
        let cap = hints.max_width.as_ref().and_then(resolve);
        let field = hints
            .width
            .as_ref()
            .and_then(resolve)
            .map(|w| cap.map_or(w, |c| w.min(c)));

        let mut content = content;
        // Hard `max_width` ceiling: always truncate content that exceeds it.
        if let Some(cap) = cap
            && visible_width(&content) > cap
        {
            content = truncate_keeping_trailing_escapes(&content, cap);
        }
        // Exact `width` field: pad shorter content per alignment; truncate
        // longer content only when the overflow policy asks for it.
        if let Some(field) = field {
            let now = visible_width(&content);
            if now > field {
                if hints.overflow == TextOverflow::Truncate {
                    content = truncate_keeping_trailing_escapes(&content, field);
                }
            } else if now < field {
                let pad = (field - now) as usize;
                content = match hints.alignment {
                    Alignment::Left => format!("{content}{}", " ".repeat(pad)),
                    Alignment::Right => format!("{}{content}", " ".repeat(pad)),
                    Alignment::Center => {
                        let left = pad / 2;
                        format!("{}{content}{}", " ".repeat(left), " ".repeat(pad - left))
                    }
                };
            }
        }
        content
    }

    /// Projects a single inline node into terminal SGR output.
    fn render_inline_node(
        &mut self,
        node: &RenderNode,
        effective: &Style,
    ) -> Result<String, RenderError> {
        let term = &self.opts.context.terminal;
        match &node.kind {
            NodeKind::Text { value } => Ok(apply_classes(
                value,
                &node.attrs.classes,
                effective,
                term,
            )),
            NodeKind::Emphasis { children } => {
                let inner = self.render_inline(children, effective)?;
                let mut child_effective = effective.clone();
                child_effective.emphasis.italic = true;
                let open = style::text_appearance_sgr(&child_effective, term);
                let close = style::appearance_close(&open, effective, term);
                Ok(apply_classes(&format!("{open}{inner}{close}"), &node.attrs.classes, effective, term))
            }
            NodeKind::Strong { children } => {
                let inner = self.render_inline(children, effective)?;
                let mut child_effective = effective.clone();
                child_effective.emphasis.bold = true;
                let open = style::text_appearance_sgr(&child_effective, term);
                let close = style::appearance_close(&open, effective, term);
                Ok(apply_classes(&format!("{open}{inner}{close}"), &node.attrs.classes, effective, term))
            }
            NodeKind::Delete { children } => {
                let inner = self.render_inline(children, effective)?;
                let mut child_effective = effective.clone();
                child_effective.emphasis.strikethrough = true;
                let open = style::text_appearance_sgr(&child_effective, term);
                let close = style::appearance_close(&open, effective, term);
                Ok(apply_classes(&format!("{open}{inner}{close}"), &node.attrs.classes, effective, term))
            }
            NodeKind::Span { children } => {
                // An inline `Span` may carry a declared `Style`. Its
                // text-appearance layers (color, emphasis) inherit from the
                // enclosing `effective` appearance; box-painting layers
                // (border, fill) and background have no inline meaning here.
                match node.attrs.style_ref().filter(|s| !s.is_empty()) {
                    Some(span_style) => {
                        let child_effective = span_style.inherited_from(effective);
                        let inner = self.render_inline(children, &child_effective)?;
                        let styled = self.render_span_classes(node, &inner, &child_effective)?;
                        let open = style::text_appearance_sgr(&child_effective, term);
                        // Reset, then restore the ancestor appearance so the
                        // run after the span keeps the inherited color/emphasis.
                        // An empty `open` closes to nothing rather than a
                        // stray reset.
                        let close = style::appearance_close(&open, effective, term);
                        Ok(format!("{open}{styled}{close}"))
                    }
                    None => {
                        let inner = self.render_inline(children, effective)?;
                        self.render_span_classes(node, &inner, effective)
                    }
                }
            }
            NodeKind::InlineCode { value } => {
                let mut child_effective = effective.clone();
                // The background (and foreground) come from the prose theme
                // reduced to the page color mode, forwarded by the darkmatter
                // entry point. A caller that builds the context directly leaves
                // them unset, so inline code falls back to a dim run rather than
                // reverse-video.
                match self.opts.context.inline_code_background {
                    Some(bg) => {
                        child_effective.background = Some(rgb_style_color(bg));
                        if let Some(fg) = self.opts.context.inline_code_color {
                            child_effective.color = Some(rgb_style_color(fg));
                        }
                    }
                    None => child_effective.emphasis.dim = true,
                }
                let open = style::text_appearance_sgr(&child_effective, term);
                let close = style::appearance_close(&open, effective, term);
                Ok(apply_classes(&format!("{open}{value}{close}"), &node.attrs.classes, effective, term))
            }
            NodeKind::Link {
                url,
                title: _,
                children,
            } => {
                // A link may carry its own `Style` (e.g. a darkmatter
                // `style.hyperlinks.*` color lowered onto the node). Its text
                // appearance inherits from the enclosing `effective` and wraps
                // the visible link text; the OSC8 escapes are added around the
                // already-styled text so the color rides inside the hyperlink.
                let link_effective = match node.attrs.style_ref().filter(|s| !s.is_empty()) {
                    Some(link_style) => link_style.inherited_from(effective),
                    None => effective.clone(),
                };
                let inner = self.render_inline(children, &link_effective)?;
                let styled = if link_effective == *effective {
                    inner
                } else {
                    let open = style::text_appearance_sgr(&link_effective, term);
                    let close = style::appearance_close(&open, effective, term);
                    format!("{open}{inner}{close}")
                };
                // Typed `text_layout` pads/truncates the visible *label* before
                // the OSC8 escapes wrap it, so the link's structured children
                // stay intact in the tree while the rendered label fits its
                // resolved field.
                let styled = match node.attrs.text_layout_ref() {
                    Some(hints) => self.apply_text_layout(styled, hints),
                    None => styled,
                };
                if url.is_empty() {
                    Ok(styled)
                } else if term.osc_link_support {
                    Ok(format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, styled))
                } else {
                    let desc = styled.replace(']', "\\]");
                    Ok(format!("[{desc}]({url})"))
                }
            }
            NodeKind::Image { url, alt, .. } => {
                // The alt-text fallback inherits the image node's own `Style`
                // (e.g. a darkmatter `style.images.*` color lowered onto the
                // node). A successfully rendered inline image is left unstyled —
                // color has no meaning for a raster cell.
                let fallback = || {
                    // Build the complete placeholder from the intact source alt;
                    // the tree's `Image.alt` is never mutated, only this
                    // projection.
                    let placeholder = match self.opts.context.image_placeholder {
                        ImagePlaceholder::Bracket => format!("[{alt}]"),
                        ImagePlaceholder::Block => format!("▉ IMAGE[{alt}]"),
                    };
                    let styled = match node.attrs.style_ref().filter(|s| !s.is_empty()) {
                        Some(img_style) => {
                            let img_effective = img_style.inherited_from(effective);
                            if img_effective == *effective {
                                placeholder
                            } else {
                                let open = style::text_appearance_sgr(&img_effective, term);
                                let close = style::appearance_close(&open, effective, term);
                                format!("{open}{placeholder}{close}")
                            }
                        }
                        None => placeholder,
                    };
                    // Typed `text_layout` shapes the *complete visible
                    // placeholder* — brackets or `▉ IMAGE[..]` framing included —
                    // so `width` establishes the exact rendered placeholder
                    // width per the spec, not the bare alt run. Mirrors the link
                    // path, which shapes the styled label after its SGR wrap.
                    match node.attrs.text_layout_ref() {
                        Some(hints) => self.apply_text_layout(styled, hints),
                        None => styled,
                    }
                };
                match self.opts.context.graphics_mode {
                    // Off / Vector: alt-text fallback (no raster form).
                    GraphicsMode::Off | GraphicsMode::Vector => Ok(fallback()),
                    // Rich: attempt the inline image protocol, falling back to
                    // alt text when the source is out of contract or unrenderable.
                    GraphicsMode::Rich => {
                        Ok(self.render_terminal_image(url, alt).unwrap_or_else(fallback))
                    }
                }
            }
            NodeKind::FootnoteReference { identifier } => {
                Ok(format!("[^{identifier}]"))
            }
            NodeKind::SoftBreak => Ok(" ".to_string()),
            NodeKind::HardBreak => Ok("\n".to_string()),
            // Built-in tokens lower to direct SGR: `mark` highlights via
            // reverse video (SGR 7) and `dim` dims (SGR 2), mirroring the
            // legacy span-class path in `apply_classes`. An unrecognized token
            // renders its children as plain inline content.
            NodeKind::Extended {
                token, children, payload,
            } => {
                if token.as_ref() == "icon" {
                    return self.render_icon_payload(payload, term);
                }
                let inner = self.render_inline(children, effective)?;
                let mut child_effective = effective.clone();
                match token.as_ref() {
                    "mark" => child_effective.emphasis.inverse = true,
                    "dim" => child_effective.emphasis.dim = true,
                    _ => return Ok(inner),
                }
                let open = style::text_appearance_sgr(&child_effective, term);
                let close = style::appearance_close(&open, effective, term);
                Ok(format!("{open}{inner}{close}"))
            }
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
            | NodeKind::Disclosure { .. }
            | NodeKind::Html { .. }
            | NodeKind::Unsupported { .. } => self.render(node),
        }
    }

    /// Returns markup as-is — inline content is already rendered to terminal
    /// SGR by [`Self::render_inline_node`].
    fn render_prose(&self, markup: &str) -> String {
        markup.to_string()
    }

    /// Word-wraps already-SGR-rendered inline `markup` to the post-margin
    /// content width ([`available_width`](TerminalRenderContext::available_width)).
    ///
    /// Uses the SGR-aware [`wrap_lines`] primitive directly — the same one the
    /// styled-blockquote and two-column paths use — so top-level paragraphs
    /// flow onto extra rows instead of overrunning the content width, matching
    /// the legacy `for_terminal` renderer's line wrapping.
    ///
    /// `wrap_lines` (rather than constructing a [`Prose`] and calling
    /// `render_in_width`) is deliberate: `Prose::render` routes back through
    /// [`render_terminal_node`], which re-enters this very paragraph path —
    /// using `Prose` here would recurse infinitely.
    fn wrap_prose(&self, markup: &str) -> String {
        let width = self.opts.context.available_width;
        if width == 0 {
            return markup.to_string();
        }
        let lines = split_lines(markup);
        let wrapped = wrap_lines(lines, &WordWrap::WrapProse(None, None), width);
        wrapped.join("\n")
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

        let hints = attrs.list_hints_ref();
        let bullet = hints
            .and_then(|h| h.bullet.as_deref())
            .unwrap_or("- ")
            .to_string();
        let origin = start.unwrap_or(1);

        // The indent for nested block children: explicit hint, else the
        // default for the list kind (4 for ordered, bullet width otherwise).
        let default_indent = if ordered { 4 } else { visible_width(&bullet) };
        let indent_children = hints
            .and_then(|h| h.indent_children)
            .unwrap_or(default_indent);
        // Absent list hints fall back to the default `hanging_indent` (`true`).
        let hanging_indent = hints.is_none_or(|h| h.hanging_indent);

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
                hanging_indent,
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
    fn render_marker_free_list(&mut self, children: &[RenderNode]) -> Result<String, RenderError> {
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
                    nested.push(self.render_tree_connector_list(nested_children, &continuation)?);
                } else if is_inline_block(item_child) {
                    // A connector-list item's paragraph carries a per-entry
                    // `Style` (e.g. `FileSystem`'s `fs_entry_style`: red for a
                    // highlight match, italic for a dotfile, cyan for a
                    // symlink). Fold that style into the effective appearance so
                    // the item label inherits its color / emphasis, then wrap
                    // the rendered label with the matching SGR — without this
                    // the projection's `Style` never reaches terminal output.
                    let item_style = item_child.attrs.style_ref().filter(|s| !s.is_empty());
                    let render_effective =
                        item_style.map(|s| s.inherited_from(self.inherited.effective()));
                    let effective = render_effective
                        .as_ref()
                        .unwrap_or_else(|| self.inherited.effective())
                        .clone();
                    let markup = match &item_child.kind {
                        NodeKind::Paragraph { children } => {
                            self.render_inline(children, &effective)?
                        }
                        _ => self.render_inline(std::slice::from_ref(item_child), &effective)?,
                    };
                    let label = self.render_prose(&markup);
                    let label = match item_style {
                        Some(style) => {
                            let open =
                                style::text_appearance_sgr(style, &self.opts.context.terminal);
                            if open.is_empty() {
                                label
                            } else {
                                format!("{open}{label}{}", style::SGR_RESET)
                            }
                        }
                        None => label,
                    };
                    label_parts.push(label);
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
        let check_marker = match node.attrs.task_hints_ref() {
            Some(hints) => task_state_marker(hints.state, &self.opts.context.terminal),
            None => match checked {
                Some(true) => "[x] ".to_string(),
                Some(false) => "[ ] ".to_string(),
                None => String::new(),
            },
        };
        let full_prefix = format!("{prefix}{check_marker}");

        // A typed `text_layout` on the list item lifts the marker to its own
        // line and left-pads the body *block* per its resolved alignment, keeping
        // the marker structurally separate from the body placement. The body
        // block occupies its resolved field (`width`/`max_width`, capped by the
        // available width); that field-wide block is then positioned within the
        // available width — so the pad is the surplus between the available width
        // and the field, not between the field and the rendered content. A
        // `Left`/no-field alignment yields no pad and falls through to the inline
        // rendering below.
        if let Some(hints) = node.attrs.text_layout_ref() {
            let available = self.opts.context.available_width;
            let field = hints
                .width
                .as_ref()
                .or(hints.max_width.as_ref())
                .map(|tv| resolve_cells(tv, available))
                .filter(|cells| *cells > 0)
                .unwrap_or(available)
                .min(available);
            let surplus = available.saturating_sub(field);
            let pad = match hints.alignment {
                Alignment::Left => 0,
                Alignment::Center => surplus / 2,
                Alignment::Right => surplus,
            };
            if pad > 0 {
                let mut out = full_prefix.clone();
                for child in item_children {
                    let body = match &child.kind {
                        NodeKind::Paragraph { children } => {
                            self.render_inline(children, &Style::default())?
                        }
                        _ => self.render(child)?,
                    };
                    out.push('\n');
                    out.push_str(&indent_block(&body, pad));
                }
                return Ok(out);
            }
        }

        let mut out = String::new();
        let mut prefix_used = false;
        let mut first_group = true;
        let mut idx = 0;
        while idx < item_children.len() {
            if !first_group {
                out.push('\n');
            }
            let child = &item_children[idx];
            if is_inline_block(child) {
                // A tight list item carries its content as a flat run of
                // inline siblings (`[Text, InlineCode, Text, …]`) with no
                // wrapping `Paragraph`. Coalesce a maximal run of consecutive
                // bare-inline children into one inline render so the whole run
                // flows and wraps as a single paragraph; a `Paragraph` child is
                // already coalesced and is rendered on its own. Without this,
                // every inline sibling after the first would fall through to the
                // block branch below and render on its own unwrapped line.
                let markup = match &child.kind {
                    NodeKind::Paragraph { children } => {
                        idx += 1;
                        self.render_inline(children, &Style::default())?
                    }
                    _ => {
                        let start = idx;
                        while idx < item_children.len()
                            && is_inline_kind(&item_children[idx].kind)
                        {
                            idx += 1;
                        }
                        self.render_inline(&item_children[start..idx], &Style::default())?
                    }
                };
                if prefix_used {
                    // A trailing inline run after a block child: indent it like
                    // body text instead of re-applying the bullet prefix.
                    out.push_str(&indent_block(&markup, indent_children));
                } else {
                    // First inline run: carries the prefix, with the prefix
                    // width as hanging indent for continuation lines.
                    out.push_str(&self.render_list_text(&full_prefix, &markup, hanging_indent));
                    prefix_used = true;
                }
            } else {
                // Block child: indent by `indent_children`, no prefix. Render
                // it at the width left *after* that indent so a nested list (or
                // other block) wraps within bounds instead of overrunning the
                // terminal by `indent_children` cells.
                let inner_width = self
                    .opts
                    .context
                    .available_width
                    .saturating_sub(indent_children)
                    .max(1);
                let body = self.render_in_width(child, inner_width)?;
                out.push_str(&indent_block(&body, indent_children));
                idx += 1;
            }
            first_group = false;
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
        // Escape Prose-special characters so literal `<b>` etc. in the
        // rendered inline output are not mis-parsed as tags during wrap.
        let safe = Prose::escape_text(markup);
        let prose = Prose::new(safe).with_word_wrap(WordWrap::WrapProse(None, hang));
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
    ///
    /// ## Mermaid promotion
    ///
    /// A `lang="mermaid"` block is promoted to a rasterized image only when the
    /// Mermaid opt-in ([`TerminalRenderContext::mermaid_mode`] is
    /// [`TerminalMermaidMode::Image`]) *and* the fidelity ceiling
    /// ([`TerminalRenderContext::graphics_mode`] is [`GraphicsMode::Rich`]) both
    /// permit it. [`GraphicsMode`] alone never promotes a fence — that keeps the
    /// public default ("Mermaid stays code") intact. When promotion is not
    /// requested the block degrades through the code-render hook and finally to
    /// the built-in plain fallback.
    ///
    /// ## Errors
    ///
    /// Under [`RenderStrictness::Strict`] a failed Mermaid promotion returns
    /// [`RenderError::LossyRejected`] rather than silently falling back; under
    /// [`RenderStrictness::Warn`] it records a lossy diagnostic and falls back.
    fn render_code_node(
        &mut self,
        lang: Option<&str>,
        value: &str,
        meta: Option<&str>,
        attrs: &renderable::tree::NodeAttrs,
    ) -> Result<String, RenderError> {
        let _is_mermaid = lang
            .map(|l| l.eq_ignore_ascii_case("mermaid"))
            .unwrap_or(false);

        // Promotion is gated by the opt-in AND the ceiling: terminal Mermaid
        // rasterizes only at `Rich`, and only when the caller opted in.
        #[cfg(feature = "image")]
        if _is_mermaid
            && self.opts.context.mermaid_mode == TerminalMermaidMode::Image
            && self.opts.context.graphics_mode == GraphicsMode::Rich
        {
            let term = self.effective_terminal();
            let diagram = MermaidDiagram::new(value);
            match diagram.try_render(&term) {
                Ok(result) => return Ok(result.output),
                Err(e) => match self.opts.strictness {
                    RenderStrictness::Strict => {
                        return Err(RenderError::LossyRejected {
                            message: format!("Mermaid promotion failed: {e}"),
                        });
                    }
                    RenderStrictness::Warn => {
                        self.diagnostics.push(Diagnostic::lossy(
                            format!(
                                "Mermaid rasterization failed, falling back to code block: {e}"
                            ),
                            None,
                        ));
                    }
                    RenderStrictness::Lossy => {}
                },
            }
        }

        if let Some(renderer) = &self.opts.code_renderer {
            let context = TerminalCodeContext::new(
                self.opts.context.available_width,
                (&self.opts.context.color_depth).into(),
                (&self.opts.context.color_mode).into(),
            )
            .with_page_surface(
                self.opts.context.page_text_color,
                self.opts.context.page_background_color,
            )
            .with_code_theme_name(self.opts.context.code_theme.clone())
            .with_line_numbers(self.opts.context.line_numbers);
            if let Some(rendered) = renderer.render_terminal_code(lang, value, meta, attrs, context)
            {
                return Ok(rendered);
            }
        }
        Ok(self.render_code(lang, value))
    }

    /// Returns the terminal capability snapshot the graphics paths consult,
    /// applying the [`force_graphics`](TerminalRenderContext::force_graphics)
    /// capability override.
    ///
    /// `TerminalImageMode::Force` (mapped to `force_graphics` at the darkmatter
    /// entry point) attempts image-protocol output even when detection reports
    /// no support: the returned terminal is marked as a TTY and, if it
    /// advertises no image protocol, is upgraded to Kitty. This mirrors the
    /// legacy terminal image renderer's force path so the policy is enforced at
    /// the renderer boundary rather than dropped.
    fn effective_terminal(&self) -> std::borrow::Cow<'_, crate::terminal::Terminal> {
        use std::borrow::Cow;

        if !self.opts.context.force_graphics {
            return Cow::Borrowed(&self.opts.context.terminal);
        }
        let mut forced = self.opts.context.terminal.clone();
        forced.is_tty = true;
        if matches!(forced.image_support, ImageSupport::None) {
            forced.image_support = ImageSupport::Kitty;
        }
        Cow::Owned(forced)
    }

    /// Renders a Markdown image to an inline terminal-image string at
    /// [`GraphicsMode::Rich`], or returns `None` to signal the alt-text
    /// fallback.
    ///
    /// Ports the legacy terminal image renderer's security contract: remote
    /// URLs and absolute paths are rejected, a relative path resolves against
    /// [`image_base_path`](TerminalRenderContext::image_base_path) (the current
    /// working directory when unset) and must stay within it, and the file must
    /// exist and be at most [`MAX_TERMINAL_IMAGE_BYTES`]. The capability
    /// snapshot honors `force_graphics` via [`Self::effective_terminal`].
    fn render_terminal_image(&self, url: &str, alt: &str) -> Option<String> {
        // Remote URLs and absolute paths are out of contract — no fetch, no
        // sandbox escape.
        if url.starts_with("http://") || url.starts_with("https://") {
            return None;
        }
        let rel = std::path::Path::new(url);
        if rel.is_absolute() {
            return None;
        }

        let base = self
            .opts
            .context
            .image_base_path
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let full_path = base.join(rel);

        // Path-traversal guard: when both ends canonicalize, the resolved file
        // must stay within the base directory.
        if let Ok(canonical_base) = base.canonicalize()
            && let Ok(canonical_full) = full_path.canonicalize()
            && !canonical_full.starts_with(&canonical_base)
        {
            return None;
        }
        if !full_path.exists() {
            return None;
        }
        if let Ok(metadata) = std::fs::metadata(&full_path)
            && metadata.len() > MAX_TERMINAL_IMAGE_BYTES
        {
            return None;
        }

        let term = self.effective_terminal();
        // Without graphics support (and without force) there is no image
        // protocol to target — fall back to alt text.
        if !term.is_tty || matches!(term.image_support, ImageSupport::None) {
            return None;
        }

        let image = TerminalImage::new(&full_path).ok()?.with_alt_text(alt);
        let output = image.render(&term);
        (!output.is_empty()).then_some(output)
    }

    /// Renders an `"icon"` extended node by walking the degradation ladder
    /// encoded in the JSON payload.
    fn render_icon_payload(
        &self,
        payload: &Option<String>,
        term: &crate::terminal::Terminal,
    ) -> Result<String, RenderError> {
        let Some(payload) = payload else {
            return Ok(String::new());
        };
        let icon: IconPayload = match serde_json::from_str(payload) {
            Ok(i) => i,
            Err(_) => return Ok(payload.clone()),
        };

        if icon.nerd_font_preferred && let Some(ref nerd) = icon.nerd_font {
            return Ok(nerd.clone());
        }
        if let Some(ref unicode) = icon.unicode {
            return Ok(unicode.clone());
        }
        if !icon.svg.is_empty()
            && !matches!(term.image_support, ImageSupport::None)
            && let Ok(s) = render_svg_string(&icon.svg, term)
        {
            return Ok(s);
        }
        Ok(icon.text)
    }

    /// Renders a code block as a dim, indented panel.
    ///
    /// Syntax highlighting is out of scope for this phase; the language tag
    /// is shown as a header so the lossy projection is visible. Under
    /// [`ColorDepth::None`] no SGR is emitted at all, honoring the no-color
    /// contract (the legacy renderer suppresses every escape in that mode).
    fn render_code(&self, lang: Option<&str>, value: &str) -> String {
        let body = value.trim_end_matches('\n');
        let no_color = self.opts.context.color_depth == ColorDepth::None;
        let dim_open = if no_color { "" } else { "\x1b[2m" };
        let dim_close = if no_color { "" } else { "\x1b[0m" };
        let lang = lang.filter(|l| !l.is_empty()).unwrap_or("");
        // Plain fenced fallback (no syntax highlighter, e.g. `ColorDepth::None`):
        // an opening fence, the verbatim body, and a matching CLOSING fence. The
        // body is not extra-indented — the page frame applies the left margin, so
        // adding spaces here would double it.
        format!(
            "{dim_open}```{lang}{dim_close}\n{dim_open}{body}{dim_close}\n{dim_open}```{dim_close}"
        )
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
                let default_hints = TableColumnHints::default();
                let hints = table_node
                    .attrs
                    .table_column_hints_ref(idx)
                    .unwrap_or(&default_hints);
                let kind = first_data_kinds.get(idx).and_then(|k| k.as_deref());
                build_table_column(text, idx, align, hints, kind)
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

        let default_terminal_hints = TableTerminalHints::default();
        let terminal_hints = table_node
            .attrs
            .table_terminal_hints_ref()
            .unwrap_or(&default_terminal_hints);
        let mut table = Table::new()
            .with_columns(columns.clone())
            .with_data(data.clone());
        table.expand_tabs_in_place(self.opts.context.terminal.tab_width);

        // Carry only the content-box `width` from the node's layout onto the
        // planning input. Margins stay default here: the tree has already
        // reduced `available_width` by the block's margins, so re-applying them
        // would double-count. `width` (e.g. `width: 100%`) drives whether the
        // planner fills to the available width or hugs its content.
        if let Some(layout) = table_node.attrs.layout_ref() {
            table.layout_mut().width = layout.width.clone();
        }

        // ── Pass 1: width planning ─────────────────────────────────────────
        let available = self.opts.context.available_width.max(1);
        let plan = match table.plan_widths(available) {
            Ok(plan) => plan,
            // Width planning failed (table cannot fit): degrade to the
            // structured error message, matching the bespoke component.
            Err(error) => return Ok(error.to_string()),
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
            .and_then(|cell| cell.attrs.style_ref())
            .filter(|s| !s.is_empty())
            .map(|s| style::text_appearance_sgr(s, &self.opts.context.terminal))
            .filter(|sgr| !sgr.is_empty());

        Ok(emit_table(
            table.columns(),
            table.data(),
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
        let Some(cell_style) = cell.attrs.style_ref().filter(|s| !s.is_empty()) else {
            return content;
        };
        let open = style::text_appearance_sgr(cell_style, &self.opts.context.terminal);
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

    /// Maps semantic span classes to direct SGR styling.
    ///
    /// The documented class vocabulary — `mark`, `dim`, `sup`, `sub` — maps
    /// to visual treatment; unknown classes are ignored, with a diagnostic
    /// emitted only under [`RenderStrictness::Warn`].
    fn render_span_classes(
        &mut self,
        node: &RenderNode,
        inner: &str,
        effective: &Style,
    ) -> Result<String, RenderError> {
        let styled = apply_classes(inner, &node.attrs.classes, effective, &self.opts.context.terminal);
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
                let dim_open = "\x1b[2m";
                let dim_close = "\x1b[0m";
                Ok(format!("{dim_open}[unsupported: {label}]{dim_close}"))
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

/// Builds a [`HorizontalRule`] from the typed [`ThematicBreakAttrs`] on a
/// [`NodeKind::ThematicBreak`] node.
///
/// Darkmatter's fold lowers HR-attribute paragraphs (e.g.
/// `--- { kind: waves, weight: thick }`) into a [`NodeKind::ThematicBreak`]
/// whose typed [`NodeAttrs::thematic_break`] field carries the authored styling.
/// The visual rule kind lives under the canonical `kind` field (legacy `style`
/// is normalized to `kind` during the fold). Without consuming those attrs the
/// terminal renderer would emit a plain rule for every HR regardless of
/// authored attributes (review-4 finding 2).
///
/// `kind`/`alignment`/`weight` are the shared [`HrKind`]/[`HrAlignment`]/
/// [`HrWeight`] enums; an unrecognized authored spelling becomes `None` at the
/// fold boundary, so a malformed `kind: dashse` leaves the rule at its default
/// rather than panicking.
///
/// [`ThematicBreakAttrs`]: renderable::tree::ThematicBreakAttrs
/// [`NodeAttrs::thematic_break`]: renderable::tree::NodeAttrs::thematic_break
fn horizontal_rule_from_attrs(attrs: &renderable::tree::NodeAttrs) -> HorizontalRule {
    let mut rule = HorizontalRule::new();
    let Some(hr) = attrs.thematic_break_ref() else {
        return rule;
    };

    if let Some(kind) = hr.kind {
        rule = rule.style(match kind {
            HrKind::Dashes => RuleStyle::Dashes,
            HrKind::Dots => RuleStyle::Dots,
            HrKind::Waves => RuleStyle::Waves,
            HrKind::LineStar => RuleStyle::LineStar,
            HrKind::LineCircle => RuleStyle::LineCircle,
            HrKind::InsetLine => RuleStyle::InsetLine,
            HrKind::CurtainRod => RuleStyle::CurtainRod,
        });
    }
    if let Some(alignment) = hr.alignment {
        rule = rule.alignment(match alignment {
            HrAlignment::Full => RuleAlignment::Full,
            HrAlignment::Center => RuleAlignment::Centered,
            HrAlignment::Left => RuleAlignment::Left,
            HrAlignment::Right => RuleAlignment::Right,
        });
    }
    if let Some(weight) = hr.weight {
        rule = rule.weight(match weight {
            HrWeight::Thin => RuleWeight::Thin,
            HrWeight::Medium => RuleWeight::Medium,
            HrWeight::Thick => RuleWeight::Thick,
        });
    }
    if let Some(width) = hr.width.as_deref() {
        rule = rule.width(width);
    }
    if let Some(color) = hr.color.as_deref() {
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

/// Truncates `content` to `width` visible columns with an ellipsis while
/// keeping the trailing escape envelope.
///
/// A styled link or image label is `open + visible + close`, where `close` is
/// the SGR reset that restores the ancestor appearance. Plain [`truncate`]
/// keeps only the visible prefix and drops the cut tail — including that
/// trailing `close` — so the node's color would bleed into following content.
/// Splitting the trailing escape run off, truncating the body, and re-appending
/// it preserves the reset regardless of where the cut lands. When `content`
/// already fits, this is a no-op.
fn truncate_keeping_trailing_escapes(content: &str, width: u32) -> String {
    let (body, trailing) = split_trailing_escapes(content);
    format!(
        "{}{}",
        truncate(body, &ELLIPSIS.to_string(), &width),
        trailing
    )
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
            | NodeKind::Extended { .. }
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

/// Wraps an opaque RGB triple as a universal terminal [`Style`] color value.
///
/// The ANSI fallback is chosen by luminance so a 16-color terminal degrades a
/// dark band to black and a light band to white.
fn rgb_style_color(
    (r, g, b): (u8, u8, u8),
) -> renderable::layout::TargetValue<renderable::style::PerMode<renderable::style::PaintColor>> {
    use renderable::color::{BasicColor, Color, RgbColor};
    use renderable::layout::TargetValue;
    use renderable::style::PerMode;

    let fallback = if u16::from(r) + u16::from(g) + u16::from(b) < 384 {
        BasicColor::Black
    } else {
        BasicColor::White
    };
    TargetValue::universal(PerMode::universal(Color::Rgb(RgbColor::new(r, g, b, fallback))))
}

/// Applies recognized semantic classes as direct SGR escapes.
///
/// `mark` highlights via reverse video, `dim` dims, and `sup`/`sub` are
/// approximated as dim text since terminals lack super/subscript.
fn apply_classes(inner: &str, classes: &[String], effective: &Style, term: &crate::terminal::Terminal) -> String {
    let mut emphasis = TextEmphasis::default();
    for class in classes {
        match class.as_str() {
            "mark" => emphasis.inverse = true,
            "dim" | "sup" | "sub" => emphasis.dim = true,
            _ => {}
        }
    }
    if emphasis.is_empty() {
        return inner.to_string();
    }
    let style = Style {
        emphasis,
        ..Default::default()
    };
    let child_effective = style.inherited_from(effective);
    let open = style::text_appearance_sgr(&child_effective, term);
    let close = style::appearance_close(&open, effective, term);
    format!("{open}{inner}{close}")
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
fn task_state_marker(
    state: renderable::tree::TaskState,
    term: &crate::terminal::Terminal,
) -> String {
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
        .map(|cell| cell.attrs.table_cell_hints_ref().map(|h| h.kind.clone()))
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
    // Restore the per-column wrap override so the render-tree planner and cell
    // wrapper break long tokens the same way the bespoke planner did.
    if let Some(wrap) = &hints.word_wrap {
        col = col.with_word_wrap(wrap.clone());
    }
    col = col.with_when(conditional_from_hint(hints.conditional));
    col = col.with_uniform_alignment(hints.uniform_alignment);
    // Reconstruct droppability from the authoritative `droppable` flag. A
    // `drop_note` always implies droppable, but the reverse is not true —
    // silent-drop columns (`drop_when_space_is_limited::<String>(None)`)
    // carry `droppable=true` with `drop_note=None`. Honoring the flag
    // ensures the render-tree plan agrees with the bespoke planner about
    // which columns may be dropped to fit the available width.
    if let Some(note) = &hints.drop_note {
        col = col.drop_when_space_is_limited(Some(note.clone()));
    } else if hints.droppable {
        col = col.drop_when_space_is_limited::<String>(None);
    }
    col
}

/// Reconstructs a typed [`TableCellContent`] from cell attributes.
///
/// The cell hint's `kind` and `raw_value` recover the original typed value
/// so numeric/currency cells re-render with their readable formatting.
/// Malformed or absent hints degrade to the rendered `text`.
fn reconstruct_cell(attrs: &renderable::tree::NodeAttrs, text: String) -> TableCellContent {
    let Some(hints) = attrs.table_cell_hints_ref() else {
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
        "styled_prose" => TableCellContent::Text(text),
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
    use renderable::color::TerminalCodeContext;
    use renderable::tree::HeadingDepth;
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::terminal::Terminal;
    use crate::utils::escape_codes::strip_escape_codes;

    fn opts(strictness: RenderStrictness) -> TerminalRenderOptions {
        TerminalRenderOptions::new(&Terminal::new_optimistic(80), strictness)
    }

    fn render(node: &RenderNode) -> Rendered<String> {
        render_terminal_node(node, &opts(RenderStrictness::Warn)).expect("render")
    }

    #[test]
    fn render_tree_code_renderer_receives_page_surface() {
        struct CaptureRenderer {
            seen: Rc<RefCell<Option<TerminalCodeContext>>>,
        }

        impl renderable::tree::CodeRenderer for CaptureRenderer {
            fn render_terminal_code(
                &self,
                _lang: Option<&str>,
                _value: &str,
                _meta: Option<&str>,
                _attrs: &renderable::tree::NodeAttrs,
                context: TerminalCodeContext,
            ) -> Option<String> {
                *self.seen.borrow_mut() = Some(context);
                Some("captured".to_string())
            }

            fn render_browser_code(
                &self,
                _lang: Option<&str>,
                _value: &str,
                _meta: Option<&str>,
                _attrs: &renderable::tree::NodeAttrs,
            ) -> Option<renderable::browser::fragment::BrowserFragment<renderable::browser::fragment::Ready>> {
                None
            }
        }

        let seen = Rc::new(RefCell::new(None));
        let mut opts = opts(RenderStrictness::Warn);
        opts.context.page_text_color = Some((192, 202, 245));
        opts.context.page_background_color = Some((26, 27, 38));
        opts.code_renderer = Some(Rc::new(CaptureRenderer {
            seen: Rc::clone(&seen),
        }));

        let node = RenderNode::code(Some("yaml".to_string()), None, "foo: string");
        let out = render_terminal_node(&node, &opts).expect("render");
        assert_eq!(out.output, "captured");

        let context = seen.borrow().clone().expect("code renderer context");
        assert_eq!(context.page_text_color(), Some((192, 202, 245)));
        assert_eq!(context.page_background_color(), Some((26, 27, 38)));
    }

    #[test]
    fn render_tree_code_renderer_receives_color_context_for_both_modes() {
        struct CaptureRenderer {
            seen: Rc<RefCell<Vec<TerminalCodeContext>>>,
        }

        impl renderable::tree::CodeRenderer for CaptureRenderer {
            fn render_terminal_code(
                &self,
                _lang: Option<&str>,
                _value: &str,
                _meta: Option<&str>,
                _attrs: &renderable::tree::NodeAttrs,
                context: TerminalCodeContext,
            ) -> Option<String> {
                self.seen.borrow_mut().push(context);
                Some("captured".to_string())
            }

            fn render_browser_code(
                &self,
                _lang: Option<&str>,
                _value: &str,
                _meta: Option<&str>,
                _attrs: &renderable::tree::NodeAttrs,
            ) -> Option<renderable::browser::fragment::BrowserFragment<renderable::browser::fragment::Ready>> {
                None
            }
        }

        let seen = Rc::new(RefCell::new(Vec::new()));
        let node = RenderNode::code(Some("yaml".to_string()), None, "foo: string");
        for mode in [
            crate::discovery::detection::ColorMode::Dark,
            crate::discovery::detection::ColorMode::Light,
        ] {
            let mut opts = opts(RenderStrictness::Warn);
            opts.context.color_depth = crate::discovery::detection::ColorDepth::TrueColor;
            opts.context.color_mode = mode;
            opts.context.code_theme = Some("one-half".to_string());
            opts.context.line_numbers = true;
            opts.code_renderer = Some(Rc::new(CaptureRenderer {
                seen: Rc::clone(&seen),
            }));

            let out = render_terminal_node(&node, &opts).expect("render");
            assert_eq!(out.output, "captured");
        }

        let contexts = seen.borrow();
        assert_eq!(contexts.len(), 2);
        assert_eq!(
            contexts[0].color_mode(),
            renderable::color::ColorMode::Dark
        );
        assert_eq!(
            contexts[1].color_mode(),
            renderable::color::ColorMode::Light
        );
        for context in contexts.iter() {
            assert_eq!(context.color_depth(), renderable::color::ColorDepth::TrueColor);
            assert_eq!(context.code_theme_name(), Some("one-half"));
            assert!(context.line_numbers());
        }
    }

    #[test]
    fn render_tree_paragraph_renders_text() {
        let node = RenderNode::paragraph(vec![RenderNode::text("hello world")]);
        let out = render(&node);
        assert!(strip_escape_codes(&out.output).contains("hello world"));
    }

    #[test]
    fn render_tree_inline_code_uses_theme_background() {
        // With a theme-resolved background supplied (as the darkmatter entry
        // point does), inline code paints that background band — not the
        // reverse-video block the regression introduced.
        let node = RenderNode::paragraph(vec![
            RenderNode::text("use "),
            RenderNode::inline_code("code"),
            RenderNode::text(" here"),
        ]);
        let mut opts = opts(RenderStrictness::Warn);
        opts.context.inline_code_color = Some((220, 220, 220));
        opts.context.inline_code_background = Some((50, 50, 55));
        let out = render_terminal_node(&node, &opts).expect("render");

        assert!(
            out.output.contains("\x1b[48;2;50;50;55m"),
            "inline code should paint the theme background band: {:?}",
            out.output,
        );
        assert!(
            !out.output.contains("\x1b[7m"),
            "inline code must not use reverse video: {:?}",
            out.output,
        );
        assert!(
            strip_escape_codes(&out.output).contains("use code here"),
            "inline code text should remain visible: {:?}",
            out.output,
        );
    }

    #[test]
    fn render_tree_inline_code_falls_back_to_dim_without_theme() {
        // A context built directly (no theme surface) keeps inline code distinct
        // with a dim run rather than reverse-video.
        let node = RenderNode::paragraph(vec![
            RenderNode::text("use "),
            RenderNode::inline_code("code"),
            RenderNode::text(" here"),
        ]);
        let out = render(&node);

        assert!(
            out.output.contains("\x1b[2m"),
            "inline code should fall back to a dim run: {:?}",
            out.output,
        );
        assert!(
            !out.output.contains("\x1b[7m"),
            "inline code must not use reverse video: {:?}",
            out.output,
        );
    }

    #[test]
    fn render_tree_extended_unknown_token_renders_children() {
        // An unrecognized token renders its children as plain inline content;
        // the wrapper itself adds nothing to the output.
        let node = RenderNode::paragraph(vec![
            RenderNode::text("before "),
            RenderNode::extended(
                "custom-token",
                vec![RenderNode::strong(vec![RenderNode::text("inner")])],
                None,
            ),
            RenderNode::text(" after"),
        ]);
        let out = strip_escape_codes(&render(&node).output);
        assert_eq!(out, "before inner after");
    }

    #[test]
    fn render_tree_extended_mark_and_dim_emit_sgr() {
        // `mark` highlights via reverse video (SGR 7); `dim` dims (SGR 2). The
        // payload text survives once the escape codes are stripped.
        let mark = RenderNode::paragraph(vec![RenderNode::extended(
            "mark",
            vec![RenderNode::text("highlighted")],
            None,
        )]);
        let out = render(&mark);
        assert!(out.output.contains("\x1b[7m"));
        assert!(strip_escape_codes(&out.output).contains("highlighted"));

        let dim = RenderNode::paragraph(vec![RenderNode::extended(
            "dim",
            vec![RenderNode::text("quiet")],
            None,
        )]);
        let out = render(&dim);
        assert!(out.output.contains("\x1b[2m"));
        assert!(strip_escape_codes(&out.output).contains("quiet"));
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
        let mut root = RenderNode::root(vec![RenderNode::text("foo"), RenderNode::text("bar")]);
        root.attrs
            .set_sequence_join(renderable::tree::SequenceJoin::None);
        let out = strip_escape_codes(&render(&root).output);
        assert_eq!(out, "foobar");
    }

    #[test]
    fn render_tree_sequence_join_mixed_inline_and_block() {
        let mut root = RenderNode::root(vec![
            RenderNode::text("inline"),
            RenderNode::paragraph(vec![RenderNode::text("para")]),
        ]);
        root.attrs
            .set_sequence_join(renderable::tree::SequenceJoin::None);
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
        let list = RenderNode::list(false, None, vec![item("a", vec![]), item("b", vec![])]);
        let out = strip_escape_codes(&render(&list).output);
        assert_eq!(out, "- a\n- b");
    }

    #[test]
    fn render_tree_list_marker_policy_none_emits_no_marker() {
        let mut list = RenderNode::list(false, None, vec![item("a", vec![]), item("b", vec![])]);
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
        assert_eq!(out, "├── dir1\n│   └── child\n└── dir2\n    └── leaf");
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
        let list = RenderNode::list(false, None, vec![task_item(TaskState::Completed, "done")]);
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
                RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("Name")])]),
                RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("Ann")])]),
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
        assert!(plain.contains("```rust"));
        // Both an opening AND a closing fence (regression: the close was dropped).
        assert_eq!(
            plain.matches("```").count(),
            2,
            "expected open + close fence, got:\n{plain}"
        );
        // The body is not extra-indented; the page frame applies any margin.
        assert!(
            !plain.contains("    let a = 1;"),
            "body should not be 4-space indented:\n{plain}"
        );
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

    /// A non-OSC8 terminal at `width`, so a link renders as the
    /// `[label](url)` fallback whose visible label is easy to assert against.
    fn no_osc_opts(width: u32) -> TerminalRenderOptions {
        let term = Terminal::builder()
            .width(width)
            .color_depth(ColorDepth::TrueColor)
            .osc_link_support(false)
            .build();
        TerminalRenderOptions::new(&term, RenderStrictness::Warn)
    }

    fn text_layout_link(label: &str, hints: TextLayoutHints) -> RenderNode {
        let mut link =
            RenderNode::link("https://example.com", None, vec![RenderNode::text(label)]);
        link.attrs.set_text_layout(&hints);
        RenderNode::paragraph(vec![link])
    }

    #[test]
    fn render_tree_link_text_layout_exact_width_pads_per_alignment() {
        use renderable::layout::{Length, TargetValue};
        // `width: 20` establishes an exact field; the short label is padded.
        let node = text_layout_link(
            "HI",
            TextLayoutHints {
                width: Some(TargetValue::universal(Length::ch(20))),
                ..Default::default()
            },
        );
        let out = render_terminal_node(&node, &no_osc_opts(60)).expect("render");
        let stripped = strip_escape_codes(&out.output);
        // The 20-column field is padded left-aligned inside the link label.
        assert_eq!(
            stripped,
            format!("[HI{}](https://example.com)", " ".repeat(18))
        );
    }

    #[test]
    fn render_tree_link_text_layout_max_width_truncates_with_ellipsis() {
        use renderable::layout::{Length, TargetValue};
        // `max_width: 8` only truncates content that exceeds the cap.
        let node = text_layout_link(
            "A very long hyperlink label",
            TextLayoutHints {
                max_width: Some(TargetValue::universal(Length::ch(8))),
                overflow: TextOverflow::Truncate,
                ..Default::default()
            },
        );
        let out = render_terminal_node(&node, &no_osc_opts(60)).expect("render");
        let stripped = strip_escape_codes(&out.output);
        assert_eq!(stripped, "[A very …](https://example.com)");
        // The visible label is exactly the 8-column cap.
        assert_eq!(visible_width("A very …"), 8);
    }

    #[test]
    fn render_tree_link_text_layout_truncation_preserves_style_reset() {
        use renderable::color::BasicColor;
        use renderable::layout::{Length, TargetValue};
        // A colored link truncated by `max_width` must still emit its closing
        // SGR reset; otherwise the truncated tail (which carries the reset) is
        // dropped and the link's color leaks into the following inline text.
        let mut link = RenderNode::link(
            "https://example.com",
            None,
            vec![RenderNode::text("A very long hyperlink label")],
        );
        link.attrs.set_style(&fg_style(BasicColor::Red));
        link.attrs.set_text_layout(&TextLayoutHints {
            max_width: Some(TargetValue::universal(Length::ch(8))),
            overflow: TextOverflow::Truncate,
            ..Default::default()
        });
        let para = RenderNode::paragraph(vec![link, RenderNode::text(" tail")]);
        let out = render_terminal_node(&para, &no_osc_opts(60)).expect("render");
        assert_no_color_bleed(&out.output, "tail");
    }

    #[test]
    fn render_tree_image_text_layout_truncation_preserves_style_reset() {
        use renderable::color::BasicColor;
        use renderable::layout::{Length, TargetValue};
        // A colored image placeholder truncated by `max_width` must likewise
        // keep its closing reset so the following inline text is not colored.
        let mut image = RenderNode::image("pic.png", None, "a very long alt text");
        image.attrs.set_style(&fg_style(BasicColor::Red));
        image.attrs.set_text_layout(&TextLayoutHints {
            max_width: Some(TargetValue::universal(Length::ch(10))),
            overflow: TextOverflow::Truncate,
            ..Default::default()
        });
        let para = RenderNode::paragraph(vec![image, RenderNode::text(" tail")]);
        let out = render_terminal_node(&para, &no_osc_opts(60)).expect("render");
        assert_no_color_bleed(&out.output, "tail");
    }

    #[test]
    fn render_tree_link_text_layout_keeps_structured_children_in_tree() {
        use renderable::layout::{Length, TargetValue};
        // Rendering a padded link must not mutate the link's structured
        // children — the tree keeps the original label nodes.
        let mut link = RenderNode::link(
            "https://example.com",
            None,
            vec![RenderNode::strong(vec![RenderNode::text("HI")])],
        );
        link.attrs.set_text_layout(&TextLayoutHints {
            width: Some(TargetValue::universal(Length::ch(20))),
            ..Default::default()
        });
        let node = RenderNode::paragraph(vec![link.clone()]);
        let _ = render_terminal_node(&node, &no_osc_opts(60)).expect("render");
        // The link node still carries its `Strong` child subtree unchanged.
        match &link.kind {
            NodeKind::Link { children, .. } => {
                assert!(matches!(children[0].kind, NodeKind::Strong { .. }));
            }
            other => panic!("expected a Link, got {other:?}"),
        }
    }

    #[test]
    fn render_tree_image_text_layout_truncates_complete_placeholder() {
        use renderable::layout::{Length, TargetValue};
        let mut image = RenderNode::image("pic.png", None, "a very long alt text");
        image.attrs.set_text_layout(&TextLayoutHints {
            max_width: Some(TargetValue::universal(Length::ch(10))),
            overflow: TextOverflow::Truncate,
            ..Default::default()
        });
        let node = RenderNode::paragraph(vec![image]);
        let out = render_terminal_node(&node, &no_osc_opts(60)).expect("render");
        let stripped = strip_escape_codes(&out.output);
        // `text_layout` shapes the *complete bracket placeholder*: the whole
        // `[a very long alt text]` is truncated to the 10-column cap, so the
        // visible placeholder fills exactly the field — framing included.
        assert_eq!(visible_width(&stripped), 10, "{stripped:?}");
        assert!(stripped.ends_with('…'), "{stripped:?}");
        assert!(stripped.starts_with("[a very"), "{stripped:?}");
    }

    #[test]
    fn render_tree_image_text_layout_exact_width_includes_bracket_framing() {
        use renderable::layout::{Alignment, Length, TargetValue};
        // A six-column exact width must produce a six-column *visible*
        // placeholder — the brackets count toward the field, not on top of it.
        let mut image = RenderNode::image("pic.png", None, "ab");
        image.attrs.set_text_layout(&TextLayoutHints {
            width: Some(TargetValue::universal(Length::ch(6))),
            // Right alignment pads on the left so the assertion is robust to any
            // trailing-whitespace trimming in block rendering.
            alignment: Alignment::Right,
            ..Default::default()
        });
        let node = RenderNode::paragraph(vec![image]);
        let out = render_terminal_node(&node, &no_osc_opts(60)).expect("render");
        let stripped = strip_escape_codes(&out.output);
        assert_eq!(visible_width(&stripped), 6, "{stripped:?}");
        assert!(stripped.ends_with("[ab]"), "{stripped:?}");
    }

    #[test]
    fn render_tree_image_text_layout_exact_width_includes_block_framing() {
        use renderable::layout::{Alignment, Length, TargetValue};
        use super::super::options::ImagePlaceholder;
        // The block placeholder `▉ IMAGE[ab]` is 11 columns wide; a 16-column
        // exact width pads it (right-aligned) to fill the field, framing
        // included, rather than padding only the alt inside the brackets.
        let mut image = RenderNode::image("pic.png", None, "ab");
        image.attrs.set_text_layout(&TextLayoutHints {
            width: Some(TargetValue::universal(Length::ch(16))),
            alignment: Alignment::Right,
            ..Default::default()
        });
        let node = RenderNode::paragraph(vec![image]);
        let mut opts = no_osc_opts(60);
        opts.context.image_placeholder = ImagePlaceholder::Block;
        let out = render_terminal_node(&node, &opts).expect("render");
        let stripped = strip_escape_codes(&out.output);
        assert_eq!(visible_width(&stripped), 16, "{stripped:?}");
        assert!(stripped.ends_with("▉ IMAGE[ab]"), "{stripped:?}");
    }

    #[test]
    fn render_tree_list_item_text_layout_lifts_marker_and_pads_body() {
        use renderable::layout::{Alignment as RAlignment, Length, TargetValue};
        // A right- and a center-aligned item both lift the `- ` marker to its
        // own line and left-pad the body; right pads more than center.
        let build = |alignment: RAlignment| {
            let mut item = RenderNode::list_item(
                None,
                vec![RenderNode::paragraph(vec![RenderNode::text("Item")])],
            );
            item.attrs.set_text_layout(&TextLayoutHints {
                max_width: Some(TargetValue::universal(Length::ch(40))),
                alignment,
                ..Default::default()
            });
            RenderNode::list(false, None, vec![item])
        };

        let render_pad = |alignment: RAlignment| -> usize {
            let out = render_terminal_node(&build(alignment), &no_osc_opts(60)).expect("render");
            let stripped = strip_escape_codes(&out.output);
            let mut lines = stripped.lines();
            // The marker sits on its own line.
            assert_eq!(lines.next().unwrap().trim_end(), "-");
            let body = lines.next().expect("body line");
            assert!(body.trim_start().starts_with("Item"), "{body:?}");
            body.len() - body.trim_start().len()
        };

        let right = render_pad(RAlignment::Right);
        let center = render_pad(RAlignment::Center);
        assert!(right > center, "right pad {right} should exceed center {center}");
        assert!(center > 0, "center pad should be positive");
    }

    #[test]
    fn render_tree_tight_list_item_coalesces_inline_run_and_wraps() {
        // A tight list item carries its content as a flat run of inline
        // siblings (`[Text, InlineCode, Text]`) with no wrapping `Paragraph`.
        // The whole run must flow and wrap as one paragraph — not split with
        // the code span (and the text after it) each on its own line, and not
        // overflow the width. Regression for the prompt-reporting wrap bug.
        let item = RenderNode::list_item(
            None,
            vec![
                RenderNode::text("the spec may include a "),
                RenderNode::inline_code("sub-spec"),
                RenderNode::text(
                    " frontmatter property indicating that this spec is part of a larger series",
                ),
            ],
        );
        let node = RenderNode::list(false, None, vec![item]);
        let out = render_terminal_node(&node, &no_osc_opts(40)).expect("render");
        let stripped = strip_escape_codes(&out.output);

        // No line exceeds the width.
        for line in stripped.lines() {
            assert!(line.chars().count() <= 40, "overflow line {line:?}");
        }
        // The first line carries the bullet and the text that precedes the
        // code span flows onto it (not broken before `sub-spec`).
        let first = stripped.lines().next().unwrap();
        assert!(
            first.starts_with("- the spec may include a sub-spec"),
            "inline run did not coalesce: {first:?}"
        );
        // The trailing text wraps onto continuation lines rather than a single
        // unwrapped overflow row — more than the two lines a broken render would
        // collapse to, and the closing word survives.
        assert!(stripped.contains("series"), "trailing text lost: {stripped:?}");
        assert!(stripped.lines().count() >= 3, "did not wrap: {stripped:?}");
    }

    #[test]
    fn render_tree_nested_list_item_wraps_within_width() {
        // A nested list item is indented by the parent's `indent_children` after
        // it renders. Its content must wrap to the width left *after* that
        // indent, or every line overruns the terminal by the indent amount and
        // the terminal hard-wraps the overflow (the lone-`;` blemish). Regression
        // for the implement-plan prompt nested frontmatter-property list.
        let inner = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(
                None,
                vec![RenderNode::text(
                    "set the source files frontmatter property to every file touched during this phase; put an empty list if none",
                )],
            )],
        );
        let outer_item = RenderNode::list_item(
            None,
            vec![
                RenderNode::text("You must set the following Frontmatter properties:"),
                inner,
            ],
        );
        let node = RenderNode::list(false, None, vec![outer_item]);
        let out = render_terminal_node(&node, &no_osc_opts(60)).expect("render");
        let stripped = strip_escape_codes(&out.output);
        for line in stripped.lines() {
            assert!(
                line.chars().count() <= 60,
                "nested item overran width: {line:?}"
            );
        }
        // The nested bullet is indented under the parent (not at column 0).
        let nested = stripped
            .lines()
            .find(|l| l.contains("set the source files"))
            .expect("nested line");
        assert!(nested.starts_with("  "), "nested bullet not indented: {nested:?}");
    }

    #[test]
    fn render_tree_text_layout_does_not_mutate_tree_across_widths() {
        use renderable::layout::{Length, TargetValue};
        let node = text_layout_link(
            "A very long hyperlink label",
            TextLayoutHints {
                max_width: Some(TargetValue::universal(Length::ch(8))),
                overflow: TextOverflow::Truncate,
                ..Default::default()
            },
        );
        let pristine = node.clone();
        let narrow = render_terminal_node(&node, &no_osc_opts(20)).expect("render");
        let wide = render_terminal_node(&node, &no_osc_opts(80)).expect("render");
        // The same immutable tree renders at both widths; the cap is absolute
        // (ch), so the outputs match and the tree is untouched.
        assert_eq!(narrow.output, wide.output);
        assert_eq!(node, pristine, "rendering must not mutate the tree");
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

    /// An image whose source is out of contract (here a relative path with no
    /// matching file) degrades to its alt text. The default render context is
    /// `Rich`, so this also proves the Rich-tier path falls back cleanly — and
    /// silently, matching the legacy terminal image renderer's contract.
    #[test]
    fn render_tree_image_falls_back_to_alt_text() {
        let node = RenderNode::paragraph(vec![RenderNode::image("pic.png", None, "a cat")]);
        let out = render(&node);
        assert!(strip_escape_codes(&out.output).contains("[a cat]"));
    }

    /// Finding 6 (review-1): at `Off`/`Vector` an image is always alt text;
    /// only `Rich` attempts the image protocol. A non-image terminal at `Rich`
    /// with a real file still degrades to alt text (no protocol to target).
    #[test]
    fn render_tree_image_alt_text_at_off_and_vector() {
        let node = RenderNode::paragraph(vec![RenderNode::image("pic.png", None, "a cat")]);
        for mode in [GraphicsMode::Off, GraphicsMode::Vector] {
            let mut o = opts(RenderStrictness::Warn);
            o.context.graphics_mode = mode;
            let out = render_terminal_node(&node, &o).expect("render");
            assert!(
                strip_escape_codes(&out.output).contains("[a cat]"),
                "{mode:?} must render alt text",
            );
        }
    }

    /// Writes a tiny valid 2×2 PNG into a fresh temp dir. Returns the dir
    /// (kept alive by the caller for the file's lifetime) plus the relative
    /// file name to reference against an `image_base_path`.
    fn temp_png() -> (tempfile::TempDir, String) {
        use image::{ImageBuffer, ImageFormat, Rgb};
        use std::io::Cursor;

        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_x, _y| Rgb([255u8, 0u8, 0u8]));
        let mut buffer = Cursor::new(Vec::new());
        img.write_to(&mut buffer, ImageFormat::Png)
            .expect("encode test png");
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("pic.png"), buffer.into_inner()).expect("write png");
        (dir, "pic.png".to_string())
    }

    /// Finding 3 (review-3): the success path. At `Rich` on an image-capable
    /// TTY, a resolvable image node must emit the inline image protocol — the
    /// fallback-only tests never proved the protocol actually fires.
    #[test]
    fn render_tree_image_rich_emits_image_protocol() {
        use crate::discovery::detection::ImageSupport;

        let (dir, name) = temp_png();
        let node = RenderNode::paragraph(vec![RenderNode::image(&name, None, "a cat")]);

        let term = Terminal::builder()
            .width(80)
            .image_support(ImageSupport::Kitty)
            .is_tty(true)
            .build();
        let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        opts.context.graphics_mode = GraphicsMode::Rich;
        opts.context.image_base_path = Some(dir.path().to_path_buf());

        let out = render_terminal_node(&node, &opts).expect("render").output;
        assert!(
            out.contains("\x1b_G"),
            "Rich + Kitty must emit the Kitty image protocol; got: {out:?}",
        );
        assert!(
            !strip_escape_codes(&out).contains("[a cat]"),
            "the success path must not fall back to bracketed alt text; got: {out:?}",
        );
    }

    /// Finding 3 (review-3): the no-capability fallback. At `Rich` with a real,
    /// resolvable file but a terminal that advertises no image protocol, the
    /// node degrades to alt text — there is no protocol to target.
    #[test]
    fn render_tree_image_rich_no_capability_falls_back_to_alt_text() {
        use crate::discovery::detection::ImageSupport;

        let (dir, name) = temp_png();
        let node = RenderNode::paragraph(vec![RenderNode::image(&name, None, "a cat")]);

        let term = Terminal::builder()
            .width(80)
            .image_support(ImageSupport::None)
            .is_tty(false)
            .build();
        let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        opts.context.graphics_mode = GraphicsMode::Rich;
        opts.context.image_base_path = Some(dir.path().to_path_buf());

        let out = render_terminal_node(&node, &opts).expect("render").output;
        assert!(
            strip_escape_codes(&out).contains("[a cat]"),
            "no image capability at Rich must degrade to alt text; got: {out:?}",
        );
        assert!(
            !out.contains("\x1b_G"),
            "no-capability fallback must not emit image protocol bytes; got: {out:?}",
        );
    }

    /// Finding 3 (review-3): the security contract. At `Rich` on a fully
    /// image-capable terminal, out-of-contract sources — a remote URL, an
    /// absolute path, and a path-traversal escape from the base dir — must all
    /// be rejected and degrade to alt text, never reaching the image pipeline.
    #[test]
    fn render_tree_image_rich_rejects_remote_absolute_and_traversal() {
        use crate::discovery::detection::ImageSupport;

        // A base dir with a sibling file *outside* it, reachable only via `..`.
        let parent = tempfile::tempdir().expect("tempdir");
        let base = parent.path().join("base");
        std::fs::create_dir(&base).expect("mkdir base");
        let (img_dir, _name) = temp_png();
        let outside = parent.path().join("outside.png");
        std::fs::copy(img_dir.path().join("pic.png"), &outside).expect("copy outside");

        let term = Terminal::builder()
            .width(80)
            .image_support(ImageSupport::Kitty)
            .is_tty(true)
            .build();
        let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        opts.context.graphics_mode = GraphicsMode::Rich;
        opts.context.image_base_path = Some(base.clone());

        for url in [
            "https://example.com/cat.png",
            "http://example.com/cat.png",
            "/etc/hosts",
            "../outside.png",
        ] {
            let node = RenderNode::paragraph(vec![RenderNode::image(url, None, "blocked")]);
            let out = render_terminal_node(&node, &opts).expect("render").output;
            assert!(
                strip_escape_codes(&out).contains("[blocked]") && !out.contains("\x1b_G"),
                "out-of-contract source {url:?} must be rejected to alt text; got: {out:?}",
            );
        }
    }

    /// Finding 3 (review-3): `force_graphics`. On a terminal that advertises no
    /// image support, forcing graphics upgrades the capability snapshot so a
    /// resolvable image node still emits the image protocol at `Rich`.
    #[test]
    fn render_tree_image_force_graphics_emits_protocol_on_unsupported_terminal() {
        use crate::discovery::detection::ImageSupport;

        let (dir, name) = temp_png();
        let node = RenderNode::paragraph(vec![RenderNode::image(&name, None, "a cat")]);

        let term = Terminal::builder()
            .width(80)
            .image_support(ImageSupport::None)
            .is_tty(false)
            .build();
        let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        opts.context.graphics_mode = GraphicsMode::Rich;
        opts.context.image_base_path = Some(dir.path().to_path_buf());

        // Without force: no protocol (mirrors the no-capability test).
        let plain = render_terminal_node(&node, &opts).expect("render").output;
        assert!(
            !plain.contains("\x1b_G"),
            "no image support without force must degrade to alt text; got: {plain:?}",
        );

        // With force: the snapshot is upgraded to Kitty and the protocol fires.
        opts.context.force_graphics = true;
        let forced = render_terminal_node(&node, &opts).expect("render").output;
        assert!(
            forced.contains("\x1b_G"),
            "force_graphics must emit the image protocol on an unsupported terminal; got: {forced:?}",
        );
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
        .expect("render")
        .output;
        // The `max_width: 10` box is sub-available, so center alignment places
        // the 10-cell box within the 80-cell width: (80 − 10) / 2 = 35. (A loose
        // `starts_with(' ')` would also pass the old content-centering model;
        // the exact offset pins the box-placement behavior.)
        assert_eq!(lead_spaces_of_line_with(&out, "hi"), 35);
    }

    #[test]
    fn render_tree_max_width_with_margins() {
        use renderable::layout::{Layout, Length, Edges, TargetValue};

        let term = Terminal::new_optimistic(80);
        let mut para = RenderNode::paragraph(vec![RenderNode::text("hello")]);
        para.attrs.set_layout(&Layout {
            margin: Edges::x(Length::ch(4)),
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
        use renderable::layout::{Alignment, Layout, Length, Edges, TargetValue};

        let term = Terminal::new_optimistic(80);

        // A root holding a block with both `max_width` and margins, so the
        // child resolves against the width left by the parent's constraints.
        let mut block =
            RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text(
                "content",
            )])]);
        block.attrs.set_layout(&Layout {
            margin: Edges::x(Length::ch(2)),
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
        // `max_width: 30` caps the box; right alignment then places that
        // sub-available box at the right edge of the content area between the
        // margins. align_area = 80 − 2 − 2 = 76; slack = 76 − 30 = 46; the box's
        // left edge lands at left-margin(2) + 46 = 48.
        assert_eq!(
            lead_spaces_of_line_with(&out.output, "content"),
            48,
            "right-aligned max_width box is pushed to the right edge of the area"
        );
    }

    /// Counts the leading spaces of the first rendered line that carries
    /// `needle`, after stripping ANSI escapes. The alignment offset (slack on
    /// the resolved content box) shows up as this lead, so it is a stable proxy
    /// for the content-box width the renderer resolved.
    fn lead_spaces_of_line_with(out: &str, needle: &str) -> usize {
        let plain = strip_escape_codes(out);
        let line = plain
            .lines()
            .find(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("no line containing {needle:?} in:\n{plain}"))
            .to_string();
        line.len() - line.trim_start().len()
    }

    #[test]
    fn render_tree_width_fixed_sets_content_box() {
        use renderable::layout::{Alignment, Layout, Length, TargetValue, Width};

        // available 80; Fixed(20) center → the 20-cell box is centered within
        // the available width (`margin:auto` semantics): (80 - 20) / 2 = 30
        // leading spaces place the box's left edge, and "hi" sits at it. With
        // the default `Auto` the box would fill 80 and the lead would be 0.
        let term = Terminal::new_optimistic(80);
        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            width: Width::Fixed(TargetValue::universal(Length::ch(20))),
            alignment: Alignment::Center,
            ..Layout::default()
        });
        let tree = RenderNode::root(vec![para]);
        let out = render_terminal_node(
            &tree,
            &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
        )
        .expect("render")
        .output;
        assert_eq!(lead_spaces_of_line_with(&out, "hi"), 30);
    }

    /// A `width: 50%` table fills half the available width — the table's column
    /// fill must use the box the block-layout step already sized (40 of 80), not
    /// re-resolve the 50% against that narrowed box (which would yield 25%, a
    /// double-applied percentage).
    #[test]
    fn render_tree_table_fixed_percent_does_not_double_apply() {
        use crate::components::renderable::TerminalRenderable;
        use renderable::layout::{Length, TargetValue, Width};

        let term = Terminal::new_optimistic(80);
        let mut table = Table::new()
            .with_columns(vec![TableColumn::new("A"), TableColumn::new("B")])
            .with_data(vec![vec!["x".into(), "y".into()]]);
        table.layout_mut().width =
            Width::Fixed(TargetValue::universal(Length::Percent(50.0)));

        let out = table.render(&term);
        let border = out
            .lines()
            .map(visible_width)
            .max()
            .expect("table emits at least one line");
        // The box is 40 (50% of 80); the table fills it. Double-application
        // would collapse it to ~20. Allow a small tolerance for border glyphs.
        assert!(
            (38..=40).contains(&border),
            "width:50% table must fill ~40 cells, got {border}; output:\n{out}"
        );
    }

    #[test]
    fn render_tree_width_auto_fills_after_margin() {
        use renderable::layout::{Alignment, Edges, Layout, Length};

        // Auto + 5ch horizontal margins on available 80: the content box fills
        // the remaining 70 cells, so centering "x" yields (70 - 1) / 2 = 34
        // plus the 5ch left margin = 39 leading spaces.
        let term = Terminal::new_optimistic(80);
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.set_layout(&Layout {
            margin: Edges::x(Length::ch(5)),
            alignment: Alignment::Center,
            ..Layout::default()
        });
        let tree = RenderNode::root(vec![para]);
        let out = render_terminal_node(
            &tree,
            &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
        )
        .expect("render")
        .output;
        assert_eq!(lead_spaces_of_line_with(&out, "x"), 39);
    }

    #[test]
    fn render_tree_fixed_width_is_clamped_by_available_minus_margin_padding() {
        use renderable::layout::{Edges, Layout, Length, TargetValue, Width};

        // Fixed(100) asks for more than the box can hold: the content box clamps
        // to available − 2*margin − 2*padding = 80 − 20 − 10 = 50, so wrapping
        // content reflows to at most 50 columns. The padding subtraction is the
        // discriminator — without it the cap would be 60. (Lead is no longer a
        // proxy: a box that fills its content area has no placement slack.)
        let plain = render_block_with_layout_wrapped(
            "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi",
            Layout {
                margin: Edges::x(Length::ch(10)),
                padding: Edges::x(Length::ch(5)),
                width: Width::Fixed(TargetValue::universal(Length::ch(100))),
                ..Layout::default()
            },
            80,
        );
        assert!(
            content_box_cols(&plain) <= 50,
            "Fixed(100) must clamp the content box to 50 (available − margin − padding): {plain:?}"
        );
    }

    /// Renders a single `"hi"` paragraph carrying `layout` at `available` and
    /// returns the leading-space alignment offset of the content line — the
    /// position of the painted box's left edge within the content area.
    fn alignment_lead(layout: renderable::layout::Layout, available: u32) -> usize {
        let term = Terminal::new_optimistic(available);
        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&layout);
        let tree = RenderNode::root(vec![para]);
        let out = render_terminal_node(
            &tree,
            &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
        )
        .expect("render")
        .output;
        lead_spaces_of_line_with(&out, "hi")
    }

    #[test]
    fn render_tree_fixed_alignment_places_box() {
        use renderable::layout::{Alignment, Layout, Length, TargetValue, Width};
        // A 20-cell `Fixed` box (no margin) is placed within the 80-cell area:
        // left flush (0), centered ((80−20)/2 = 30), right flush (80−20 = 60).
        let fixed = |a| Layout {
            width: Width::Fixed(TargetValue::universal(Length::ch(20))),
            alignment: a,
            ..Layout::default()
        };
        assert_eq!(alignment_lead(fixed(Alignment::Left), 80), 0);
        assert_eq!(alignment_lead(fixed(Alignment::Center), 80), 30);
        assert_eq!(alignment_lead(fixed(Alignment::Right), 80), 60);
    }

    #[test]
    fn render_tree_capped_auto_alignment_places_box() {
        use renderable::layout::{Alignment, Layout, Length, TargetValue};
        // `Auto` capped by `max_width: 20` is sub-available, so it is placed like
        // a 20-cell box — the defect the review flagged for capped `Auto`.
        let capped = |a| Layout {
            max_width: Some(TargetValue::universal(Length::ch(20))),
            alignment: a,
            ..Layout::default()
        };
        assert_eq!(alignment_lead(capped(Alignment::Left), 80), 0);
        assert_eq!(alignment_lead(capped(Alignment::Center), 80), 30);
        assert_eq!(alignment_lead(capped(Alignment::Right), 80), 60);
    }

    #[test]
    fn render_tree_fit_content_alignment_places_box() {
        use renderable::layout::{Alignment, Layout, Width};
        // `FitContent` shrinks the box to the 2-cell content, then places it:
        // left (0), centered ((80−2)/2 = 39), right (80−2 = 78).
        let fit = |a| Layout {
            width: Width::FitContent,
            alignment: a,
            ..Layout::default()
        };
        assert_eq!(alignment_lead(fit(Alignment::Left), 80), 0);
        assert_eq!(alignment_lead(fit(Alignment::Center), 80), 39);
        assert_eq!(alignment_lead(fit(Alignment::Right), 80), 78);
    }

    #[test]
    fn render_tree_percent_padding_resolves_against_parent_width() {
        use renderable::layout::{Edges, Layout, Length};
        // 10% horizontal padding at available 80 resolves to 8 cells per side,
        // against the parent width — once. The buggy double-resolution would
        // re-resolve against the narrowed 64-cell content box and yield 6. With
        // no background the left padding shows as 8 leading spaces before the
        // content, which also proves padding is reserved with an empty `Style`.
        let layout = Layout {
            padding: Edges::x(Length::Percent(10.0)),
            ..Layout::default()
        };
        assert_eq!(alignment_lead(layout, 80), 8);
    }

    #[test]
    fn render_tree_transparent_padding_reserves_cells_without_style() {
        use renderable::layout::{Edges, Layout, Length};
        // A node with `padding` but no `Style` still reserves the cells: the
        // left padding renders as leading spaces even though nothing is painted.
        let layout = Layout {
            padding: Edges::x(Length::ch(4)),
            ..Layout::default()
        };
        assert_eq!(alignment_lead(layout, 80), 4);
    }

    #[test]
    fn render_tree_border_is_added_around_fixed_content_box() {
        use renderable::layout::{Alignment, Layout, Length, TargetValue, Width};
        use renderable::style::{Border, BorderSides, Style};
        // The border is added *around* the `Fixed(20)` content box rather than
        // carved out of it, so the box is 22 cells wide (20 content + 2 drawn
        // edges). Centered in 80 the box's left edge lands at (80 − 22) / 2 = 29
        // — one cell tighter than the borderless box (lead 30), the difference
        // being the reserved border. (Old code subtracted the border from the
        // content, shrinking it to 18 and mis-placing the box.)
        let term = Terminal::new_optimistic(80);
        let mut para = RenderNode::paragraph(vec![RenderNode::text("ab")]);
        para.attrs.set_layout(&Layout {
            width: Width::Fixed(TargetValue::universal(Length::ch(20))),
            alignment: Alignment::Center,
            ..Layout::default()
        });
        para.attrs.set_style(&Style {
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            ..Style::default()
        });
        let tree = RenderNode::root(vec![para]);
        let out = render_terminal_node(
            &tree,
            &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
        )
        .expect("render")
        .output;
        assert_eq!(lead_spaces_of_line_with(&out, "ab"), 29);
    }

    #[test]
    fn render_tree_fixed_background_paints_full_content_box() {
        use renderable::layout::{Layout, Length, TargetValue, Width};
        use renderable::style::{Background, Style};
        // A `Fixed(20)` box with short content paints its full 20-cell band, not
        // the 2-cell text. The painted-band width is the discriminator the review
        // flagged: the box width must follow the resolved content box.
        let layout = Layout {
            width: Width::Fixed(TargetValue::universal(Length::ch(20))),
            ..Layout::default()
        };
        let style = Style {
            background: Some(Background::subtle()),
            ..Style::default()
        };
        let out = render_block_with_layout_and_style("hi", layout, style, 80);
        let line = out.lines().find(|l| l.contains("hi")).expect("content line");
        assert_eq!(
            background_run_width(line),
            20,
            "Fixed(20) background must paint all 20 cells: {line:?}"
        );
    }

    #[test]
    fn render_tree_fixed_border_encloses_full_content_box() {
        use renderable::layout::{Layout, Length, TargetValue, Width};
        use renderable::style::{Border, BorderSides, Style};
        // The direct analogue of the `bt block hi --width 20 --border all`
        // reproduction: the border encloses 20 content cells (box width 22), not
        // the 2-cell text.
        let layout = Layout {
            width: Width::Fixed(TargetValue::universal(Length::ch(20))),
            ..Layout::default()
        };
        let style = Style {
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            ..Style::default()
        };
        let out = render_block_with_layout_and_style("hi", layout, style, 80);
        assert_eq!(
            bordered_box_width(&out),
            22,
            "Fixed(20) + border must enclose 20 content cells (20 + 2 edges): {out:?}"
        );
    }

    #[test]
    fn render_tree_auto_background_fills_available_width() {
        use renderable::layout::{Layout, Width};
        use renderable::style::{Background, Style};
        // An `Auto` box fills the available content width, so its background
        // spans the whole 40-cell area even with 2-cell content.
        let layout = Layout {
            width: Width::Auto,
            ..Layout::default()
        };
        let style = Style {
            background: Some(Background::subtle()),
            ..Style::default()
        };
        let out = render_block_with_layout_and_style("hi", layout, style, 40);
        let line = out.lines().find(|l| l.contains("hi")).expect("content line");
        assert_eq!(
            background_run_width(line),
            40,
            "Auto background must fill the 40-cell available width: {line:?}"
        );
    }

    #[test]
    fn render_tree_auto_border_encloses_available_width() {
        use renderable::layout::{Layout, Width};
        use renderable::style::{Border, BorderSides, Style};
        // An `Auto` box with a border fills available − 2 edges = 38 content
        // cells, so the drawn box is the full 40-cell width.
        let layout = Layout {
            width: Width::Auto,
            ..Layout::default()
        };
        let style = Style {
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            ..Style::default()
        };
        let out = render_block_with_layout_and_style("hi", layout, style, 40);
        assert_eq!(
            bordered_box_width(&out),
            40,
            "Auto + border must enclose available − 2 edges = 38 cells: {out:?}"
        );
    }

    #[test]
    fn render_tree_capped_auto_background_paints_cap() {
        use renderable::layout::{Layout, Length, TargetValue};
        use renderable::style::{Background, Style};
        // `Auto` capped by `max_width: 20` paints a 20-cell band — the cap is the
        // content box, and the painted box follows it.
        let layout = Layout {
            max_width: Some(TargetValue::universal(Length::ch(20))),
            ..Layout::default()
        };
        let style = Style {
            background: Some(Background::subtle()),
            ..Style::default()
        };
        let out = render_block_with_layout_and_style("hi", layout, style, 80);
        let line = out.lines().find(|l| l.contains("hi")).expect("content line");
        assert_eq!(
            background_run_width(line),
            20,
            "max_width-capped Auto background must paint the 20-cell cap: {line:?}"
        );
    }

    #[test]
    fn render_tree_fit_content_background_matches_widest_line() {
        use renderable::layout::{Layout, Width};
        use renderable::style::{Background, Style};
        // `FitContent` shrinks the box to the widest line ("there" = 5), so the
        // painted band is exactly 5 — the floor is a no-op for FitContent.
        let layout = Layout {
            width: Width::FitContent,
            ..Layout::default()
        };
        let style = Style {
            background: Some(Background::subtle()),
            ..Style::default()
        };
        let out = render_block_with_layout_and_style("hi\nthere", layout, style, 40);
        for line in out.lines().filter(|l| l.contains("hi") || l.contains("there")) {
            assert_eq!(
                background_run_width(line),
                5,
                "FitContent band must match the widest line (5): {line:?}"
            );
        }
    }

    #[test]
    fn render_tree_fixed_background_multiline_is_uniform() {
        use renderable::layout::{Layout, Length, TargetValue, Width};
        use renderable::style::{Background, Style};
        // Two lines of differing length inside a `Fixed(20)` box are each painted
        // to a uniform 20-cell band, so the background is a solid rectangle.
        let layout = Layout {
            width: Width::Fixed(TargetValue::universal(Length::ch(20))),
            ..Layout::default()
        };
        let style = Style {
            background: Some(Background::subtle()),
            ..Style::default()
        };
        let out = render_block_with_layout_and_style("hi\nthere", layout, style, 80);
        for line in out.lines().filter(|l| l.contains("hi") || l.contains("there")) {
            assert_eq!(
                background_run_width(line),
                20,
                "every content row of a Fixed(20) box paints 20 cells: {line:?}"
            );
        }
    }

    /// Builds a one-paragraph document, records `layout` and `style` on it, and
    /// renders it through the terminal fold at `width`, returning the raw
    /// (SGR-bearing) output.
    fn render_block_with_layout_and_style(
        text: &str,
        layout: renderable::layout::Layout,
        style: Style,
        width: u32,
    ) -> String {
        let term = Terminal::new_optimistic(width);
        let mut para = RenderNode::paragraph(vec![RenderNode::text(text)]);
        para.attrs.set_layout(&layout);
        para.attrs.set_style(&style);
        let tree = RenderNode::root(vec![para]);
        render_terminal_node(
            &tree,
            &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
        )
        .expect("render")
        .output
    }

    /// Counts the leading spaces of a raw line that appear *before* any SGR
    /// escape. The transparent margin is emitted as raw spaces ahead of the
    /// painted box, so this is a stable proxy for "margin is not painted".
    fn leading_unstyled_spaces(line: &str) -> usize {
        let mut count = 0;
        for ch in line.chars() {
            if ch == ' ' {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// Whether the raw `line` contains a contiguous painted background run of at
    /// least `n` visible cells.
    fn has_background_run_of_width(line: &str, n: usize) -> bool {
        background_run_width(line) >= n
    }

    /// The widest contiguous painted background run on `line`, in visible cells.
    ///
    /// SGR sequences are parsed (rather than matched byte-for-byte, per the L2
    /// capture rule): a `48;…` / basic background code opens the run, a `0`/`49`
    /// reset closes it, and the visible cells between are counted. This is the
    /// exact painted-band width — the discriminator for a fixed/auto box that
    /// must paint its full resolved width rather than shrink to its text.
    fn background_run_width(line: &str) -> usize {
        let chars: Vec<char> = line.chars().collect();
        let mut idx = 0;
        let mut bg_active = false;
        let mut run = 0usize;
        let mut max = 0usize;
        while idx < chars.len() {
            if chars[idx] == '\x1b' && chars.get(idx + 1) == Some(&'[') {
                let start = idx + 2;
                let mut j = start;
                while j < chars.len() && !chars[j].is_ascii_alphabetic() {
                    j += 1;
                }
                if chars.get(j) == Some(&'m') {
                    let params: String = chars[start..j].iter().collect();
                    if let Some(on) = sgr_sets_background(&params) {
                        bg_active = on;
                        if !on {
                            run = 0;
                        }
                    }
                }
                idx = j + 1;
                continue;
            }
            if bg_active {
                run += 1;
                max = max.max(run);
            }
            idx += 1;
        }
        max
    }

    /// The visible width of the bordered box drawn around `needle`'s content,
    /// measured from the top rule of the rendered output.
    ///
    /// Strips ANSI escapes and returns the visible width of the first line that
    /// is a horizontal border rule (begins with a corner glyph). The interior
    /// (content columns) is this width minus the two drawn vertical edges.
    fn bordered_box_width(out: &str) -> usize {
        let plain = strip_escape_codes(out);
        let rule = plain
            .lines()
            .map(str::trim)
            .find(|l| l.starts_with('┌') || l.starts_with('┏') || l.starts_with('╔') || l.starts_with('╭'))
            .unwrap_or_else(|| panic!("no top border rule in:\n{plain}"));
        visible_width(rule) as usize
    }

    /// Interprets an SGR parameter string: `Some(true)` if it opens a
    /// background, `Some(false)` if it resets one, `None` if it is irrelevant.
    fn sgr_sets_background(params: &str) -> Option<bool> {
        let parts: Vec<&str> = params.split(';').collect();
        // A leading `0` (or empty `\x1b[m`) is a full reset.
        if params.is_empty() || parts.first() == Some(&"0") {
            return Some(false);
        }
        // `48` opens an extended background; `49` resets it. RGB/256 components
        // that follow `48` are skipped because `48` is matched first.
        for part in &parts {
            match *part {
                "48" => return Some(true),
                "49" => return Some(false),
                _ => {
                    if let Ok(code) = part.parse::<u16>()
                        && ((40..=47).contains(&code) || (100..=107).contains(&code))
                    {
                        return Some(true);
                    }
                }
            }
        }
        None
    }

    #[test]
    fn render_tree_padding_is_painted_with_background_margin_is_not() {
        use renderable::layout::{Edges, Layout, Length};
        use renderable::style::{Background, Style};

        // margin 2 (transparent) + padding 3 (painted) + content, background
        // subtle. The 2 leading margin cells carry no background SGR; the
        // padding cells plus content do.
        let layout = Layout {
            margin: Edges::x(Length::ch(2)),
            padding: Edges::x(Length::ch(3)),
            ..Layout::default()
        };
        let style = Style {
            background: Some(Background::subtle()),
            ..Style::default()
        };
        let out = render_block_with_layout_and_style("hi", layout, style, 40);
        let first = out.lines().find(|l| l.contains("hi")).expect("line with hi");
        assert_eq!(
            leading_unstyled_spaces(first),
            2,
            "margin must stay transparent: {first:?}"
        );
        assert!(
            has_background_run_of_width(first, 3 + 2 + 3),
            "padding + content must be painted: {first:?}"
        );
    }

    #[test]
    fn render_tree_padding_top_bottom_paint_blank_rows() {
        use renderable::layout::{Edges, Layout, Length};
        use renderable::style::{Background, Style};

        // Vertical padding of 1 paints a blank background row above and below
        // the content row.
        let layout = Layout {
            padding: Edges::all(Length::ch(1)),
            ..Layout::default()
        };
        let style = Style {
            background: Some(Background::subtle()),
            ..Style::default()
        };
        let out = render_block_with_layout_and_style("hi", layout, style, 40);
        let lines: Vec<&str> = out.lines().collect();
        let content_idx = lines
            .iter()
            .position(|l| l.contains("hi"))
            .expect("content line");
        // A painted blank row sits directly above and below the content row.
        let band = 1 + 2 + 1; // left pad + "hi" + right pad
        assert!(content_idx >= 1, "expected a padding row above: {lines:?}");
        assert!(
            has_background_run_of_width(lines[content_idx - 1], band),
            "top padding row painted: {:?}",
            lines[content_idx - 1]
        );
        assert!(
            has_background_run_of_width(lines[content_idx + 1], band),
            "bottom padding row painted: {:?}",
            lines.get(content_idx + 1)
        );
    }

    #[test]
    fn render_tree_layout_on_inline_node_is_a_validation_error() {
        use renderable::layout::Layout;

        let term = Terminal::new_optimistic(80);

        let mut text = RenderNode::text("hello");
        text.attrs.set_layout(&Layout::default());
        let tree = RenderNode::root(vec![RenderNode::paragraph(vec![text])]);

        // Layout on an inline node is an error-severity validation finding, so
        // the validation gate rejects the tree before strictness is consulted —
        // every strictness level fails identically.
        for strictness in [
            RenderStrictness::Warn,
            RenderStrictness::Lossy,
            RenderStrictness::Strict,
        ] {
            let result =
                render_terminal_node(&tree, &TerminalRenderOptions::new(&term, strictness));
            assert!(
                matches!(result, Err(RenderError::InvalidTree { .. })),
                "{strictness:?} should reject inline layout as an error"
            );
        }
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

    /// Asserts the run of `raw` preceding the first occurrence of `marker`
    /// closes any opened SGR color with a reset, so a styled-and-truncated
    /// inline node does not bleed its color into the following text.
    #[cfg(test)]
    fn assert_no_color_bleed(raw: &str, marker: &str) {
        let pos = raw
            .find(marker)
            .unwrap_or_else(|| panic!("marker {marker:?} not found in {raw:?}"));
        assert!(
            raw[..pos].contains(style::SGR_RESET),
            "expected an SGR reset before {marker:?}, got: {raw:?}"
        );
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
        use renderable::style::{Border, BorderSides, Style, TextEmphasis};

        // Border is a box-painting layer with no inline meaning: a `Span`
        // declaring it must not draw box-drawing glyphs inline.
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

    #[test]
    fn render_tree_inline_span_restores_parent_inverse() {
        use renderable::style::{Style, TextEmphasis};

        // A child span resets at its close, then the parent's inverse is
        // re-applied — the tail after the child stays inverse rather than
        // losing all emphasis state.
        let inverse_style = Style {
            emphasis: TextEmphasis {
                inverse: true,
                ..TextEmphasis::default()
            },
            ..Style::default()
        };
        let mut child = RenderNode::span(vec![], vec![RenderNode::text("child")]);
        // The child declares its own color so its close fires a reset.
        child
            .attrs
            .set_style(&fg_style(renderable::color::BasicColor::Green));
        let mut parent = RenderNode::span(
            vec![],
            vec![
                RenderNode::text("head "),
                child,
                RenderNode::text(" tail"),
            ],
        );
        parent.attrs.set_style(&inverse_style);
        let para = RenderNode::paragraph(vec![parent]);

        let out = render(&para).output;
        // The child's close restores the parent inverse (SGR 7), not a bare
        // reset that would drop it.
        let after_child = out.split("child").nth(1).expect("child run");
        assert!(
            after_child.contains("\x1b[7m"),
            "parent inverse not restored after child span: {out:?}"
        );
        assert!(strip_escape_codes(&out).contains("head child tail"));
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
            inherited: InheritedStyle::root(),
        };
        let out = writer.render(&cell).expect("render cell");
        assert!(
            out.contains("\x1b[1m"),
            "standalone styled cell must emit bold SGR: {out:?}"
        );
    }

    /// Review-4 finding 2: a `NodeKind::ThematicBreak` carrying typed
    /// [`ThematicBreakAttrs`](renderable::tree::ThematicBreakAttrs) must render
    /// the corresponding styled rule, not collapse to the default dashed rule.
    /// Without this wiring the fold's HR-attribute storage would never reach the
    /// user's terminal.
    ///
    /// The test uses a text-tier terminal (`ImageSupport::None`) so the
    /// `HorizontalRule`'s SVG-to-Kitty image tier is bypassed and the
    /// glyph-based output is observable.
    #[test]
    fn render_tree_thematic_break_consumes_darkmatter_hr_hints() {
        use crate::discovery::detection::ImageSupport;

        let mut hr = RenderNode::thematic_break();
        hr.attrs.set_thematic_break(&renderable::tree::ThematicBreakAttrs {
            kind: Some(HrKind::Waves),
            ..Default::default()
        });

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

    /// `horizontal_rule_from_attrs` maps the shared [`HrAlignment::Center`]
    /// (the canonical spelling both the `centered` inline alias and the
    /// `style.hr` page default parse to) onto [`RuleAlignment::Centered`].
    /// Without that arm a page-default centered rule would silently fall back
    /// to `Full`.
    #[test]
    fn horizontal_rule_from_attrs_centers_on_hr_alignment_center() {
        let mut attrs = renderable::tree::NodeAttrs::default();
        attrs.set_thematic_break(&renderable::tree::ThematicBreakAttrs {
            alignment: Some(HrAlignment::Center),
            ..Default::default()
        });
        let rule = horizontal_rule_from_attrs(&attrs);
        assert_eq!(*rule.rule_alignment(), RuleAlignment::Centered);
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

    /// GraphicsMode::Off must suppress the image tier and force the text
    /// tier, even on a Kitty-capable TTY.
    #[test]
    fn render_tree_thematic_break_off_mode_suppresses_image_tier() {
        use crate::discovery::detection::ImageSupport;
        let mut hr = RenderNode::thematic_break();
        hr.attrs.set_thematic_break(&renderable::tree::ThematicBreakAttrs {
            kind: Some(HrKind::Waves),
            ..Default::default()
        });

        let term = Terminal::builder()
            .width(40)
            .image_support(ImageSupport::Kitty)
            .is_tty(true)
            .build();
        let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        opts.context.graphics_mode = renderable::tree::GraphicsMode::Off;

        let out = render_terminal_node(&hr, &opts).expect("render hr").output;
        // Image tier is suppressed → no Kitty escape sequences.
        assert!(
            !out.contains("\x1b_G"),
            "GraphicsMode::Off must suppress Kitty image tier: {out:?}"
        );
        // Text tier produces the waves glyph.
        assert!(
            out.contains('\u{224B}') || out.contains('~'),
            "expected waves text tier under Off mode; got: {out:?}"
        );
    }

    /// GraphicsMode::Rich on a Kitty-capable TTY should use the image tier
    /// when available.
    #[test]
    #[cfg(feature = "image")]
    fn render_tree_thematic_break_rich_mode_uses_image_tier_when_available() {
        use crate::discovery::detection::ImageSupport;
        let mut hr = RenderNode::thematic_break();
        hr.attrs.set_thematic_break(&renderable::tree::ThematicBreakAttrs {
            kind: Some(HrKind::Waves),
            ..Default::default()
        });

        let term = Terminal::builder()
            .width(40)
            .image_support(ImageSupport::Kitty)
            .is_tty(true)
            .build();
        let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        opts.context.graphics_mode = renderable::tree::GraphicsMode::Rich;

        let out = render_terminal_node(&hr, &opts).expect("render hr").output;
        // Kitty image tier should be active.
        assert!(
            out.contains("\x1b_G"),
            "GraphicsMode::Rich must use Kitty image tier when available: {out:?}"
        );
    }

    /// Finding 1 (review-1): the terminal has no vector HR form, so
    /// `GraphicsMode::Vector` must use the text tier — never rasterize — even on
    /// a Kitty-capable TTY. Only `Rich` is allowed to emit image escapes.
    #[test]
    fn render_tree_thematic_break_vector_mode_uses_text_tier() {
        use crate::discovery::detection::ImageSupport;
        let mut hr = RenderNode::thematic_break();
        hr.attrs.set_thematic_break(&renderable::tree::ThematicBreakAttrs {
            kind: Some(HrKind::Waves),
            ..Default::default()
        });

        let term = Terminal::builder()
            .width(40)
            .image_support(ImageSupport::Kitty)
            .is_tty(true)
            .build();
        let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        opts.context.graphics_mode = renderable::tree::GraphicsMode::Vector;

        let out = render_terminal_node(&hr, &opts).expect("render hr").output;
        assert!(
            !out.contains("\x1b_G"),
            "GraphicsMode::Vector must not rasterize the HR on the terminal: {out:?}"
        );
        assert!(
            out.contains('\u{224B}') || out.contains('~'),
            "expected waves text tier under Vector mode; got: {out:?}"
        );
    }

    /// Finding 5 (review-1): `force_graphics` must be enforced at the renderer
    /// boundary, not merely stored. On a terminal that advertises no image
    /// support, forcing graphics upgrades the capability snapshot so the HR
    /// image tier still fires at `Rich`.
    #[test]
    #[cfg(feature = "image")]
    fn render_tree_thematic_break_force_graphics_rasterizes_on_unsupported_terminal() {
        use crate::discovery::detection::ImageSupport;
        let mut hr = RenderNode::thematic_break();
        hr.attrs.set_thematic_break(&renderable::tree::ThematicBreakAttrs {
            kind: Some(HrKind::Waves),
            ..Default::default()
        });

        let term = Terminal::builder()
            .width(40)
            .image_support(ImageSupport::None)
            .is_tty(false)
            .build();
        let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        opts.context.graphics_mode = renderable::tree::GraphicsMode::Rich;

        // Without force: no image support → text tier (no Kitty escape).
        let plain = render_terminal_node(&hr, &opts).expect("render hr").output;
        assert!(
            !plain.contains("\x1b_G"),
            "no image support without force must use the text tier: {plain:?}"
        );

        // With force: capability snapshot is upgraded → image tier fires.
        opts.context.force_graphics = true;
        let forced = render_terminal_node(&hr, &opts).expect("render hr").output;
        assert!(
            forced.contains("\x1b_G"),
            "force_graphics must enable the Kitty image tier on an unsupported terminal: {forced:?}"
        );
    }

    /// Finding 7 (review-1): under `RenderStrictness::Strict`, a Mermaid
    /// promotion that cannot complete must escalate to a `RenderError` rather
    /// than silently degrading to a code block. A terminal with no image
    /// support guarantees the promotion fails (no protocol to target, or no
    /// rasterization deps) regardless of host capability.
    #[test]
    #[cfg(feature = "image")]
    fn mermaid_promotion_failure_rejects_under_strict() {
        use crate::discovery::detection::ImageSupport;
        use renderable::tree::{GraphicsMode, TerminalMermaidMode};

        let mermaid_node =
            RenderNode::code(Some("mermaid".to_string()), None, "flowchart LR\n    A --> B");

        let term = Terminal::builder()
            .width(80)
            .image_support(ImageSupport::None)
            .is_tty(false)
            .build();
        let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Strict);
        opts.context.graphics_mode = GraphicsMode::Rich;
        opts.context.mermaid_mode = TerminalMermaidMode::Image;

        let result = render_terminal_node(&mermaid_node, &opts);
        assert!(
            matches!(result, Err(RenderError::LossyRejected { .. })),
            "strict Mermaid promotion failure must reject; got {result:?}",
        );
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
        assert!(
            tree_out.contains("\x1b[1m"),
            "bold SGR missing: {tree_out:?}"
        );
    }

    /// Builds a one-paragraph document, records `layout` on it, and renders it
    /// through the terminal fold at `width` with prose wrapping enabled (so a
    /// reflowed `FitContent` box is observable), returning the ANSI-stripped
    /// output.
    fn render_block_with_layout_wrapped(
        text: &str,
        layout: renderable::layout::Layout,
        width: u32,
    ) -> String {
        let term = Terminal::new_optimistic(width);
        let mut para = RenderNode::paragraph(vec![RenderNode::text(text)]);
        para.attrs.set_layout(&layout);
        let tree = RenderNode::root(vec![para]);
        let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        opts.context.wrap_prose = true;
        let out = render_terminal_node(&tree, &opts).expect("render").output;
        strip_escape_codes(&out)
    }

    /// The widest content-box column count: the maximum trimmed visible width
    /// over all rendered lines. Trimming strips the alignment lead, leaving the
    /// box width.
    fn content_box_cols(plain: &str) -> usize {
        plain
            .lines()
            .map(|l| visible_width(l.trim()))
            .max()
            .unwrap_or(0) as usize
    }

    #[test]
    fn render_tree_fit_content_sizes_box_to_widest_line() {
        use renderable::layout::{Alignment, Layout, Width};
        use renderable::style::{Background, Style};

        // Two lines; FitContent shrinks the painted box to the widest line
        // ("there" = 5) instead of filling the available width, then centers
        // that shrunk box within the available 40 cells. The painted-band
        // width is the discriminator: an `Auto` fallback would paint the full
        // 40-cell band and place it flush-left.
        let layout = Layout {
            width: Width::FitContent,
            alignment: Alignment::Center,
            ..Layout::default()
        };
        let style = Style {
            background: Some(Background::subtle()),
            ..Style::default()
        };
        let out = render_block_with_layout_and_style("hi\nthere", layout, style, 40);
        let line = out
            .lines()
            .find(|l| l.contains("there"))
            .expect("line with content");
        assert!(
            has_background_run_of_width(line, 5),
            "painted band must fit the widest line: {line:?}"
        );
        assert!(
            !has_background_run_of_width(line, 8),
            "painted band must not fill the available width: {line:?}"
        );
        assert!(
            leading_unstyled_spaces(line) > 0,
            "FitContent box must be centered within the available width: {line:?}"
        );
    }

    #[test]
    fn render_tree_fit_content_shrinks_below_max_width() {
        use renderable::layout::{Layout, Length, TargetValue, Width};
        use renderable::style::{Background, Style};

        // A generous `max_width` is only a ceiling: FitContent still shrinks the
        // painted box to the 2-cell content, not the 20-cell cap.
        let layout = Layout {
            width: Width::FitContent,
            max_width: Some(TargetValue::universal(Length::ch(20))),
            ..Layout::default()
        };
        let style = Style {
            background: Some(Background::subtle()),
            ..Style::default()
        };
        let out = render_block_with_layout_and_style("hi", layout, style, 40);
        let line = out
            .lines()
            .find(|l| l.contains("hi"))
            .expect("line with content");
        assert!(
            has_background_run_of_width(line, 2),
            "painted band must fit the content: {line:?}"
        );
        assert!(
            !has_background_run_of_width(line, 5),
            "painted band must shrink below max_width: {line:?}"
        );
    }

    #[test]
    fn render_tree_fit_content_is_capped_by_max_width() {
        use renderable::layout::{Layout, Length, TargetValue, Width};

        // Short words so the box can reflow to the cap; max_width 3 caps the
        // FitContent box below its natural single-line width (13 cells).
        let plain = render_block_with_layout_wrapped(
            "a b c d e f g",
            Layout {
                width: Width::FitContent,
                max_width: Some(TargetValue::universal(Length::ch(3))),
                ..Layout::default()
            },
            80,
        );
        assert!(
            content_box_cols(&plain) <= 3,
            "max_width must cap the FitContent box: {plain:?}"
        );
    }
}
