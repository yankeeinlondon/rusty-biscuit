//! Browser (HTML) renderer for the canonical render tree.
//!
//! [`render_browser_node`] folds a [`RenderNode`] into a typed
//! [`BrowserFragment<Ready>`]; [`render_browser_document`] folds a whole
//! [`Document`] into an [`HtmlPage`].
//!
//! The renderer emits *safe, typed* HTML through the [`BrowserFragment`]
//! builders. Text content is escaped on emit by the fragment renderer, so the
//! only path that produces unescaped markup is [`NodeKind::Html`], governed by
//! [`RawHtmlPolicy`].
//!
//! ## Examples
//!
//! ```
//! use renderable::tree::{HeadingDepth, RenderNode};
//! use renderable::tree::render::{render_browser_node, BrowserRenderOptions};
//!
//! let tree = RenderNode::root(vec![RenderNode::heading(
//!     HeadingDepth::new(2).unwrap(),
//!     vec![RenderNode::text("Title")],
//! )]);
//! let rendered = render_browser_node(&tree, &BrowserRenderOptions::default()).unwrap();
//! assert_eq!(rendered.output.render(), "<div><h2>Title</h2></div>");
//! ```

use crate::browser::PageOptions;
use crate::browser::fragment::{BrowserFragment, ComposableNode, Ready};
use crate::html::HtmlPage;
use crate::html::attribute::{ClassDefinition, DomId};
use crate::html::tag::{BlockTag, HtmlAttribute, HtmlType, VoidTag};
use crate::tree::attrs::NodeAttrs;
use crate::tree::diagnostic::{Diagnostic, Severity};
use crate::tree::document::Document;
use crate::tree::error::{RenderError, RenderStrictness, Rendered};
use crate::tree::node::{ColumnAlign, HeadingDepth, NodeKind, RenderNode};
use crate::tree::validate::{ValidationError, ValidationMode, validate};

/// How a [`NodeKind::Html`] node is treated by the browser renderer.
///
/// The [`Default`] is [`RawHtmlPolicy::Escape`]: the safe choice. Raw HTML
/// from an untrusted source is a script-injection vector, so the renderer
/// never passes it through verbatim unless the caller explicitly opts in with
/// [`RawHtmlPolicy::Allow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RawHtmlPolicy {
    /// Emit the raw HTML verbatim. The caller vouches for its safety.
    Allow,
    /// Emit the HTML source as escaped text and record a lossy diagnostic.
    /// This is the safe default.
    #[default]
    Escape,
    /// Reject raw HTML. Under [`RenderStrictness::Strict`] this is a
    /// [`RenderError::LossyRejected`]; under [`RenderStrictness::Warn`] /
    /// [`RenderStrictness::Lossy`] it degrades to escaped text (the same
    /// behavior as [`RawHtmlPolicy::Escape`]) so the strictness model stays
    /// consistent.
    Reject,
}

/// Options controlling a browser render.
///
/// The [`Default`] uses [`RenderStrictness::Warn`], [`RawHtmlPolicy::Escape`]
/// (the safe choice — see [`RawHtmlPolicy`]), and no [`PageOptions`].
#[derive(Debug, Default)]
pub struct BrowserRenderOptions {
    /// How strictly lossy or unsupported content is treated.
    pub strictness: RenderStrictness,
    /// How [`NodeKind::Html`] nodes are handled.
    pub raw_html: RawHtmlPolicy,
    /// Optional page options applied by [`render_browser_document`].
    pub page: Option<PageOptions>,
}

/// Renders a render tree node to a typed [`BrowserFragment<Ready>`].
///
/// The node is validated with [`validate`] first; an error-severity
/// structural finding causes an immediate [`RenderError::InvalidTree`]
/// regardless of [`BrowserRenderOptions::strictness`]. Warning-severity
/// findings are folded into [`Rendered::diagnostics`] under
/// [`RenderStrictness::Warn`] and [`RenderStrictness::Lossy`], and escalate to
/// a [`RenderError::InvalidTree`] under [`RenderStrictness::Strict`].
///
/// ## Returns
///
/// A [`Rendered<BrowserFragment<Ready>>`] carrying the fragment and any
/// non-fatal diagnostics.
///
/// ## Errors
///
/// - [`RenderError::InvalidTree`] if the tree fails structural validation, or
///   if [`RenderStrictness::Strict`] meets a warning-severity validation
///   finding (this includes [`NodeKind::Unsupported`] nodes, whose warning is
///   escalated by the validation gate before the writer runs).
/// - [`RenderError::LossyRejected`] if [`RawHtmlPolicy::Reject`] meets a
///   [`NodeKind::Html`] node under [`RenderStrictness::Strict`].
pub fn render_browser_node(
    node: &RenderNode,
    opts: &BrowserRenderOptions,
) -> Result<Rendered<BrowserFragment<Ready>>, RenderError> {
    let mut writer = gate(node, opts)?;
    let output = writer.render(node)?;
    Ok(Rendered {
        output,
        diagnostics: writer.diagnostics,
    })
}

/// Renders a whole [`Document`] to an [`HtmlPage`].
///
/// [`Document::root`] is expected to be a [`NodeKind::Root`]; each of its
/// children becomes a page fragment. When [`BrowserRenderOptions::page`] is
/// present it is applied to the page via [`HtmlPage::apply_page_options`].
///
/// Document frontmatter is **not** wired into page metadata here: the
/// canonical-tree fold does not yet populate
/// [`DocumentMetadata::frontmatter`], so mapping it to `<head>` tags is out of
/// scope for this renderer.
///
/// ## Errors
///
/// Propagates every error from [`render_browser_node`].
///
/// [`DocumentMetadata::frontmatter`]: crate::tree::DocumentMetadata::frontmatter
pub fn render_browser_document(
    doc: &Document,
    opts: &BrowserRenderOptions,
) -> Result<Rendered<HtmlPage>, RenderError> {
    let mut writer = gate(&doc.root, opts)?;

    // The document body is the root's children rendered as page fragments.
    // A non-`Root` top-level node renders as a single wrapping fragment.
    let fragments = match &doc.root.kind {
        NodeKind::Root { children } => writer.render_each(children)?,
        _ => vec![writer.render(&doc.root)?],
    };

    let mut page = HtmlPage::from_fragments(fragments);
    if let Some(page_options) = opts.page.clone() {
        page.apply_page_options(page_options);
    }
    Ok(Rendered {
        output: page,
        diagnostics: writer.diagnostics,
    })
}

/// Validates `node` and builds a [`Writer`], folding (or escalating) every
/// warning-severity validation finding per the strictness model.
fn gate<'a>(node: &RenderNode, opts: &'a BrowserRenderOptions) -> Result<Writer<'a>, RenderError> {
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
    Ok(writer)
}

/// Threads render options and accumulating diagnostics through the recursion.
struct Writer<'a> {
    opts: &'a BrowserRenderOptions,
    diagnostics: Vec<Diagnostic>,
}

impl Writer<'_> {
    /// Renders a single node and its subtree to a [`BrowserFragment<Ready>`].
    ///
    /// A node's bold / italic / strikethrough [`Style`](crate::style::Style)
    /// emphasis is lowered differently by element category: an **inline** node
    /// has its fragment wrapped in the matching semantic elements
    /// (`<strong>`, `<em>`, `<s>`); a **block** node lowers the same emphasis
    /// to inline CSS via [`node_attributes`], because wrapping a block element
    /// inside `<strong>`/`<em>`/`<s>` is invalid HTML. The color, background,
    /// underline, dim, and blink layers always lower to CSS.
    fn render(&mut self, node: &RenderNode) -> Result<BrowserFragment<Ready>, RenderError> {
        let fragment = self.render_kind(node)?;
        if is_inline_node_kind(&node.kind) {
            Ok(wrap_style_emphasis(&node.attrs, fragment))
        } else {
            // Block emphasis is already lowered to CSS by `node_attributes`.
            Ok(fragment)
        }
    }

    /// Renders a single node by its [`NodeKind`], without applying the
    /// semantic emphasis wrappers of a declared [`Style`](crate::style::Style).
    fn render_kind(&mut self, node: &RenderNode) -> Result<BrowserFragment<Ready>, RenderError> {
        match &node.kind {
            NodeKind::Root { children } => self.block(BlockTag::Div, &node.attrs, children),
            NodeKind::Heading { depth, children } => {
                self.block(heading_tag(*depth), &node.attrs, children)
            }
            NodeKind::Section {
                depth,
                heading,
                children,
            } => self.render_section(node, *depth, heading, children),
            NodeKind::Paragraph { children } => {
                // A projected `Progress` widget carries `ProgressHints`. The
                // browser emits a semantic CSS progress bar before falling
                // back to normal paragraph rendering.
                if let Some(hints) = node.attrs.progress_hints() {
                    self.render_progress(node, &hints, children)
                } else {
                    self.block(BlockTag::P, &node.attrs, children)
                }
            }
            NodeKind::BlockQuote { children } => {
                if let Some(hints) = node.attrs.columns_hints() {
                    self.render_columns(node, &hints, children)
                } else {
                    self.block(BlockTag::Blockquote, &node.attrs, children)
                }
            }
            NodeKind::List {
                ordered,
                start,
                children,
            } => self.render_list(node, *ordered, *start, children),
            NodeKind::ListItem { checked, children } => {
                self.render_list_item(node, *checked, children)
            }
            NodeKind::Code {
                lang,
                meta: _,
                value,
            } => Ok(self.render_code_block(node, lang.as_deref(), value)),
            NodeKind::ThematicBreak => Ok(self.void(VoidTag::Hr, &node.attrs)),
            NodeKind::Table { align, children } => self.render_table(node, align, children),
            NodeKind::TableRow { children } => self.block(BlockTag::Tr, &node.attrs, children),
            NodeKind::TableCell { children } => self.block(BlockTag::Td, &node.attrs, children),
            NodeKind::FootnoteDefinition {
                identifier,
                children,
            } => self.render_footnote_definition(node, identifier, children),
            NodeKind::Text { value } => Ok(text_fragment(value)),
            NodeKind::Emphasis { children } => self.block(BlockTag::Em, &node.attrs, children),
            NodeKind::Strong { children } => self.block(BlockTag::Strong, &node.attrs, children),
            NodeKind::Delete { children } => self.block(BlockTag::S, &node.attrs, children),
            NodeKind::Span { children } => self.block(BlockTag::Span, &node.attrs, children),
            NodeKind::InlineCode { value } => Ok(self.render_inline_code(node, value)),
            NodeKind::Link {
                url,
                title,
                children,
            } => self.render_link(node, url, title.as_deref(), children),
            NodeKind::Image { url, title, alt } => {
                Ok(self.render_image(node, url, title.as_deref(), alt))
            }
            NodeKind::FootnoteReference { identifier } => {
                Ok(self.render_footnote_reference(node, identifier))
            }
            // A soft break is a single space between phrasing runs.
            NodeKind::SoftBreak => Ok(text_fragment(" ")),
            NodeKind::HardBreak => Ok(self.void(VoidTag::Br, &node.attrs)),
            NodeKind::Html { value, block } => self.render_html(node, value, *block),
            NodeKind::Unsupported { label } => self.render_unsupported(node, label),
        }
    }

    /// Renders every node in `children` to its own fragment.
    fn render_each(
        &mut self,
        children: &[RenderNode],
    ) -> Result<Vec<BrowserFragment<Ready>>, RenderError> {
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            out.push(self.render(child)?);
        }
        Ok(out)
    }

    /// Builds a block-tag fragment, attaching `attrs` and rendered children.
    fn block(
        &mut self,
        tag: BlockTag,
        attrs: &NodeAttrs,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let inline = is_inline_block_tag(&tag);
        let mut fragment = BrowserFragment::new().define_as_block_tag(tag, "");
        for attr in node_attributes(attrs, inline) {
            fragment = fragment.add_attribute(attr);
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Builds a void-tag fragment carrying `attrs`.
    fn void(&self, tag: VoidTag, attrs: &NodeAttrs) -> BrowserFragment<Ready> {
        let inline = is_inline_void_tag(&tag);
        let mut fragment = BrowserFragment::new().define_as_void_tag(tag);
        for attr in node_attributes(attrs, inline) {
            fragment = fragment.add_attribute(attr);
        }
        fragment.finalize()
    }

    /// Renders a section as `<section>` containing a heading tag and body.
    fn render_section(
        &mut self,
        node: &RenderNode,
        depth: HeadingDepth,
        heading: &[RenderNode],
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::Section, "");
        for attr in node_attributes(&node.attrs, false) {
            fragment = fragment.add_attribute(attr);
        }

        // Render the heading as h1-h6.
        let mut heading_fragment =
            BrowserFragment::new().define_as_block_tag(heading_tag(depth), "");
        for child in heading {
            heading_fragment = heading_fragment.add_component(self.render(child)?);
        }
        fragment = fragment.add_component(heading_fragment.finalize());

        // Render the body children.
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a two-column block quote as a `<div class="columns">` flex
    /// container holding two `<div class="column">` children.
    ///
    /// The flat child list is split at [`ColumnsHints::left_count`]; the left
    /// children fill the first column `<div>`, the rest fill the second. The
    /// `columns` / `column` classes are preserved as external-CSS hooks. The
    /// container additionally gets inline `display:flex` and `gap` CSS, plus
    /// any layout CSS from [`NodeAttrs::layout`]. The left column carries the
    /// width CSS from [`ColumnsHints::left_width`]; the right column flexes to
    /// fill the remaining width.
    ///
    /// [`ColumnsHints::left_count`]: crate::tree::ColumnsHints::left_count
    /// [`ColumnsHints::left_width`]: crate::tree::ColumnsHints::left_width
    fn render_columns(
        &mut self,
        node: &RenderNode,
        hints: &crate::tree::ColumnsHints,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let split = hints.left_count.min(children.len());
        let (left, right) = children.split_at(split);

        // The node's `id` / `class` / layout / `Style` attributes are lowered
        // through the shared `node_attributes` helper so user-supplied class
        // hooks and node `Style` are preserved alongside the literal `columns`
        // class and the inline `display`/`gap` flex CSS.
        let mut container = BrowserFragment::new().define_as_block_tag(BlockTag::Div, "");
        let attributes = node_attributes(&node.attrs, false);
        let mut class_emitted = false;
        let mut style_emitted = false;
        for attr in attributes {
            match attr {
                // Merge the literal `columns` class ahead of any user classes.
                HtmlAttribute::Class(class) => {
                    container = container.add_attribute(HtmlAttribute::Class(
                        ClassDefinition::new(format!("columns {}", class.as_str())),
                    ));
                    class_emitted = true;
                }
                // Merge node layout / `Style` CSS into the flex container
                // `style` without overwriting the `display`/`gap` declarations.
                HtmlAttribute::Other(key, value) if key == "style" => {
                    container = container.add_attribute(HtmlAttribute::Other(
                        "style".into(),
                        super::shared::columns_container_css(hints, &value),
                    ));
                    style_emitted = true;
                }
                other => container = container.add_attribute(other),
            }
        }
        if !class_emitted {
            container =
                container.add_attribute(HtmlAttribute::Class(ClassDefinition::new("columns")));
        }
        if !style_emitted {
            container = container.add_attribute(HtmlAttribute::Other(
                "style".into(),
                super::shared::columns_container_css(hints, ""),
            ));
        }

        for (group, column_css) in [
            (left, super::shared::left_column_css(hints.left_width)),
            (right, super::shared::right_column_css().to_string()),
        ] {
            let mut column = BrowserFragment::new().define_as_block_tag(BlockTag::Div, "");
            column = column.add_attribute(HtmlAttribute::Class(ClassDefinition::new("column")));
            column = column.add_attribute(HtmlAttribute::Other("style".into(), column_css));
            for child in group {
                column = column.add_component(self.render(child)?);
            }
            container = container.add_component(column.finalize());
        }
        Ok(container.finalize())
    }

    /// Renders a projected `Progress` widget as a semantic CSS progress bar.
    ///
    /// The HTML carries `role="progressbar"` plus ARIA value attributes and
    /// `progress-*` classes. Color slots lower to inline `background-color`;
    /// non-default glyphs and brackets are preserved as `data-*` attributes.
    /// Node [`Layout`](crate::layout::Layout) is lowered onto the outer
    /// element through the existing browser layout lowering.
    fn render_progress(
        &mut self,
        node: &RenderNode,
        hints: &crate::tree::ProgressHints,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        // The accessible label is recovered from the paragraph's fallback
        // text — the same `"{label} {pct}%"` shape the terminal renderer uses.
        // The *plain text* of the children is collected (not their rendered
        // HTML) so the label is escaped exactly once by `progress_html`.
        let fallback_text = plain_text(children);
        let layout_css = node
            .attrs
            .layout()
            .map(|layout| layout_to_css(&layout))
            .filter(|css| !css.is_empty())
            .unwrap_or_default();
        let html = super::shared::progress_html(hints, &fallback_text, &layout_css);
        Ok(BrowserFragment::new().define_as_raw_html(html).finalize())
    }

    /// Renders a list as `<ol>` (respecting `start`) or `<ul>`.
    fn render_list(
        &mut self,
        node: &RenderNode,
        ordered: bool,
        start: Option<u64>,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let tag = if ordered { BlockTag::Ol } else { BlockTag::Ul };
        let mut fragment = BrowserFragment::new().define_as_block_tag(tag, "");
        // A typed list marker policy lowers per its browser fidelity:
        //
        // - `None` is *faithful* — the browser genuinely has a no-marker list
        //   (`list-style:none`), so it degrades silently with no diagnostic.
        // - `TreeConnectors` is *lossy* — the browser cannot draw terminal
        //   box-drawing connector geometry, so it degrades to a plain
        //   no-marker list and reports the loss per the strictness model.
        let marker_css = match node.attrs.list_marker_policy() {
            crate::tree::ListMarkerPolicy::Default => None,
            crate::tree::ListMarkerPolicy::None => Some("list-style:none"),
            crate::tree::ListMarkerPolicy::TreeConnectors => {
                let message = "list marker policy 'tree_connectors' has no \
                               browser equivalent; degraded to a plain \
                               no-marker list"
                    .to_string();
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
                Some("list-style:none")
            }
        };
        let attributes = node_attributes(&node.attrs, false);
        let has_style = attributes
            .iter()
            .any(|a| matches!(a, HtmlAttribute::Other(k, _) if k == "style"));
        for attr in attributes {
            // Merge the marker-policy CSS into the existing style attribute
            // rather than emitting a second, conflicting one.
            match (&attr, marker_css) {
                (HtmlAttribute::Other(key, value), Some(extra)) if key == "style" => {
                    fragment = fragment.add_attribute(HtmlAttribute::Other(
                        "style".into(),
                        format!("{value};{extra}"),
                    ));
                }
                _ => fragment = fragment.add_attribute(attr),
            }
        }
        // When no style attribute was emitted, add one for the marker policy.
        if let Some(extra) = marker_css.filter(|_| !has_style) {
            fragment =
                fragment.add_attribute(HtmlAttribute::Other("style".into(), extra.to_string()));
        }
        // `<ol start="N">` controls the first item's number.
        if ordered && let Some(start) = start.filter(|s| *s != 1) {
            fragment =
                fragment.add_attribute(HtmlAttribute::Other("start".into(), start.to_string()));
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a list item as `<li>`. A task item (`checked` is `Some`) gets a
    /// leading disabled `<input type=checkbox>` and the `task-list-item` class.
    fn render_list_item(
        &mut self,
        node: &RenderNode,
        checked: Option<bool>,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let mut attrs = node.attrs.clone();
        if checked.is_some() {
            attrs.classes.push("task-list-item".to_string());
        }

        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::Li, "");
        for attr in node_attributes(&attrs, false) {
            fragment = fragment.add_attribute(attr);
        }
        if let Some(checked) = checked {
            fragment = fragment.add_component(checkbox(checked));
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a fenced code block as `<pre><code>`; a language is carried as
    /// a `language-<lang>` class on the `<code>` element.
    fn render_code_block(
        &self,
        node: &RenderNode,
        lang: Option<&str>,
        value: &str,
    ) -> BrowserFragment<Ready> {
        let mut code = BrowserFragment::new().define_as_block_tag(BlockTag::Code, "");
        if let Some(lang) = lang.filter(|lang| !lang.is_empty()) {
            code = code.add_attribute(HtmlAttribute::Class(ClassDefinition::new(format!(
                "language-{lang}"
            ))));
        }
        let code = code.add_child(ComposableNode::TextFragment(value.to_string()));

        let mut pre = BrowserFragment::new().define_as_block_tag(BlockTag::Pre, "");
        for attr in node_attributes(&node.attrs, false) {
            pre = pre.add_attribute(attr);
        }
        pre.add_component(code.finalize()).finalize()
    }

    /// Renders inline code as `<code>` carrying escaped text.
    fn render_inline_code(&self, node: &RenderNode, value: &str) -> BrowserFragment<Ready> {
        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::Code, "");
        for attr in node_attributes(&node.attrs, true) {
            fragment = fragment.add_attribute(attr);
        }
        fragment
            .add_child(ComposableNode::TextFragment(value.to_string()))
            .finalize()
    }

    /// Renders a GFM table. The first child row is the header (`<thead>` with
    /// `<th>` cells); the rest go into `<tbody>`. Per-column [`ColumnAlign`]
    /// is applied as a `text-align` inline style on every cell.
    fn render_table(
        &mut self,
        node: &RenderNode,
        align: &[ColumnAlign],
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let mut table = BrowserFragment::new().define_as_block_tag(BlockTag::Table, "");
        for attr in node_attributes(&node.attrs, false) {
            table = table.add_attribute(attr);
        }

        // A table title/caption is emitted as a `<caption>` before `<thead>`
        // and `<tbody>`. An empty or whitespace-only title is ignored.
        if let Some(title) = node.attrs.table_title()
            && !title.trim().is_empty()
        {
            let caption = BrowserFragment::new()
                .define_as_block_tag(BlockTag::Caption, "")
                .add_component(text_fragment(title.trim()))
                .finalize();
            table = table.add_component(caption);
        }

        let mut rows = children.iter();
        if let Some(header) = rows.next() {
            let header_row = self.render_table_row(header, align, true)?;
            let thead = BrowserFragment::new()
                .define_as_block_tag(BlockTag::Thead, "")
                .add_component(header_row)
                .finalize();
            table = table.add_component(thead);
        }

        let mut tbody = BrowserFragment::new().define_as_block_tag(BlockTag::Tbody, "");
        for row in rows {
            tbody = tbody.add_component(self.render_table_row(row, align, false)?);
        }
        table = table.add_component(tbody.finalize());
        Ok(table.finalize())
    }

    /// Renders a table row; `header` selects `<th>` vs `<td>` cells.
    fn render_table_row(
        &mut self,
        row: &RenderNode,
        align: &[ColumnAlign],
        header: bool,
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let cells = match &row.kind {
            NodeKind::TableRow { children } => children.as_slice(),
            // Validation guarantees table children are rows; this is a
            // defensive empty fallback for trees bypassing the gate.
            _ => &[],
        };
        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::Tr, "");
        for attr in node_attributes(&row.attrs, false) {
            fragment = fragment.add_attribute(attr);
        }
        for (index, cell) in cells.iter().enumerate() {
            fragment = fragment.add_component(self.render_table_cell(
                cell,
                align.get(index).copied().unwrap_or(ColumnAlign::None),
                header,
            )?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a table cell as `<th>` or `<td>`, applying column alignment.
    fn render_table_cell(
        &mut self,
        cell: &RenderNode,
        align: ColumnAlign,
        header: bool,
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let children = match &cell.kind {
            NodeKind::TableCell { children } => children.as_slice(),
            _ => &[],
        };
        let tag = if header { BlockTag::Th } else { BlockTag::Td };
        let mut fragment = BrowserFragment::new().define_as_block_tag(tag, "");
        // Column alignment merges into the cell's existing `style` attribute
        // (from node `Style` / `Layout`) rather than emitting a second,
        // conflicting `style` — the same merge the columns and list paths use.
        let align_css = align_value(align).map(|value| format!("text-align:{value}"));
        let mut style_emitted = false;
        for attr in node_attributes(&cell.attrs, false) {
            match (&attr, &align_css) {
                (HtmlAttribute::Other(key, value), Some(extra)) if key == "style" => {
                    fragment = fragment.add_attribute(HtmlAttribute::Other(
                        "style".into(),
                        format!("{value};{extra}"),
                    ));
                    style_emitted = true;
                }
                _ => fragment = fragment.add_attribute(attr),
            }
        }
        if let Some(extra) = align_css.filter(|_| !style_emitted) {
            fragment = fragment.add_attribute(HtmlAttribute::Other("style".into(), extra));
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a footnote definition as a `<div>` carrying the identifier as
    /// its `id` (`fn-<identifier>`).
    fn render_footnote_definition(
        &mut self,
        node: &RenderNode,
        identifier: &str,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::Div, "");
        fragment =
            fragment.add_attribute(HtmlAttribute::Id(DomId::new(format!("fn-{identifier}"))));
        fragment = fragment.add_attribute(HtmlAttribute::Class(ClassDefinition::new(
            "footnote-definition",
        )));
        for attr in node_attributes(&node.attrs, false) {
            fragment = fragment.add_attribute(attr);
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a footnote reference as an `<a>` anchor to the definition.
    fn render_footnote_reference(
        &self,
        node: &RenderNode,
        identifier: &str,
    ) -> BrowserFragment<Ready> {
        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::A, "");
        for attr in node_attributes(&node.attrs, true) {
            fragment = fragment.add_attribute(attr);
        }
        fragment
            .add_attribute(HtmlAttribute::Other(
                "href".into(),
                format!("#fn-{identifier}"),
            ))
            .add_child(ComposableNode::TextFragment(identifier.to_string()))
            .finalize()
    }

    /// Renders a hyperlink as `<a href title>`.
    fn render_link(
        &mut self,
        node: &RenderNode,
        url: &str,
        title: Option<&str>,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::A, "");
        for attr in node_attributes(&node.attrs, true) {
            fragment = fragment.add_attribute(attr);
        }
        // `href` carries an arbitrary (possibly relative) URL, which the
        // typed `HtmlAttribute::Href(Url)` cannot represent — the `Other`
        // outlet keeps the value escaped without forcing absolute-URL parsing.
        fragment = fragment.add_attribute(HtmlAttribute::Other("href".into(), url.to_string()));
        if let Some(title) = title {
            fragment = fragment.add_attribute(HtmlAttribute::Title(title.to_string()));
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Renders an image as `<img src alt title>`.
    fn render_image(
        &self,
        node: &RenderNode,
        url: &str,
        title: Option<&str>,
        alt: &str,
    ) -> BrowserFragment<Ready> {
        let mut fragment = BrowserFragment::new().define_as_void_tag(VoidTag::Img);
        for attr in node_attributes(&node.attrs, true) {
            fragment = fragment.add_attribute(attr);
        }
        // `src` carries an arbitrary (possibly relative) URL — see the note
        // on `render_link` for why the `Other` outlet is used.
        fragment = fragment
            .add_attribute(HtmlAttribute::Other("src".into(), url.to_string()))
            .add_attribute(HtmlAttribute::Alt(alt.to_string()));
        if let Some(title) = title {
            fragment = fragment.add_attribute(HtmlAttribute::Title(title.to_string()));
        }
        fragment.finalize()
    }

    /// Renders a [`NodeKind::Html`] node per [`RawHtmlPolicy`].
    ///
    /// | Policy   | Strict                     | Warn / Lossy            |
    /// |----------|----------------------------|-------------------------|
    /// | `Allow`  | raw verbatim               | raw verbatim            |
    /// | `Escape` | escaped text + diagnostic* | escaped text + diag.*   |
    /// | `Reject` | `RenderError::LossyRejected` | escaped text + diag.* |
    ///
    /// *No diagnostic is recorded under [`RenderStrictness::Lossy`].
    fn render_html(
        &mut self,
        node: &RenderNode,
        value: &str,
        _block: bool,
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        match self.opts.raw_html {
            RawHtmlPolicy::Allow => Ok(BrowserFragment::new()
                .define_as_raw_html(value.to_string())
                .finalize()),
            RawHtmlPolicy::Escape => {
                self.note_lossy("raw HTML emitted as escaped text", node);
                Ok(text_fragment(value))
            }
            RawHtmlPolicy::Reject => match self.opts.strictness {
                RenderStrictness::Strict => Err(RenderError::LossyRejected {
                    message: "raw HTML rejected by RawHtmlPolicy::Reject".to_string(),
                }),
                RenderStrictness::Warn | RenderStrictness::Lossy => {
                    self.note_lossy("raw HTML rejected and emitted as escaped text", node);
                    Ok(text_fragment(value))
                }
            },
        }
    }

    /// Renders an [`NodeKind::Unsupported`] node according to strictness.
    fn render_unsupported(
        &mut self,
        node: &RenderNode,
        label: &str,
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        match self.opts.strictness {
            // Defensive fallback: trees entered through `render_browser_node`
            // are preempted by the validation gate, which escalates the
            // `Unsupported` warning to `InvalidTree` first.
            RenderStrictness::Strict => Err(RenderError::Unsupported {
                label: label.to_string(),
            }),
            RenderStrictness::Warn => {
                self.diagnostics.push(Diagnostic::unsupported(
                    format!("unsupported content dropped: {label}"),
                    Some(node.span.clone()),
                ));
                Ok(BrowserFragment::new()
                    .define_as_raw_html(format!("<!-- unsupported: {label} -->"))
                    .finalize())
            }
            RenderStrictness::Lossy => Ok(BrowserFragment::new()
                .define_as_text_fragment("")
                .finalize()),
        }
    }

    /// Records a lossy diagnostic, unless strictness is [`RenderStrictness::Lossy`].
    fn note_lossy(&mut self, message: &str, node: &RenderNode) {
        if self.opts.strictness != RenderStrictness::Lossy {
            self.diagnostics.push(Diagnostic::lossy(
                message.to_string(),
                Some(node.span.clone()),
            ));
        }
    }
}

/// Maps a heading depth `1..=6` to its `<h1>`..`<h6>` block tag.
fn heading_tag(depth: HeadingDepth) -> BlockTag {
    match depth.get() {
        1 => BlockTag::H1,
        2 => BlockTag::H2,
        3 => BlockTag::H3,
        4 => BlockTag::H4,
        5 => BlockTag::H5,
        // `HeadingDepth` is constrained to `1..=6`; `6` is the only
        // remaining value.
        _ => BlockTag::H6,
    }
}

/// The `text-align` value for a column alignment, or `None` for
/// [`ColumnAlign::None`].
fn align_value(align: ColumnAlign) -> Option<&'static str> {
    match align {
        ColumnAlign::Left => Some("left"),
        ColumnAlign::Center => Some("center"),
        ColumnAlign::Right => Some("right"),
        ColumnAlign::None => None,
    }
}

/// Returns `true` for [`NodeKind`] variants that are inline (phrasing-level).
///
/// Inline nodes have their bold / italic / strikethrough `Style` emphasis
/// lowered to semantic element wrappers; block nodes lower the same emphasis
/// to CSS, because wrapping a block element in `<strong>`/`<em>`/`<s>` is
/// invalid HTML.
fn is_inline_node_kind(kind: &NodeKind) -> bool {
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

/// Collects the plain text content of a phrasing subtree.
///
/// Used to recover a `Progress` widget's accessible label from its paragraph
/// children without HTML-escaping noise — the collected text is escaped
/// exactly once, by `progress_html`.
fn plain_text(nodes: &[RenderNode]) -> String {
    fn collect(node: &RenderNode, out: &mut String) {
        match &node.kind {
            NodeKind::Text { value } | NodeKind::InlineCode { value } => out.push_str(value),
            NodeKind::Image { alt, .. } => out.push_str(alt),
            NodeKind::SoftBreak | NodeKind::HardBreak => out.push(' '),
            _ => {
                for child in node.children() {
                    collect(child, out);
                }
            }
        }
    }
    let mut out = String::new();
    for node in nodes {
        collect(node, &mut out);
    }
    out
}

/// Returns `true` for [`BlockTag`] variants that represent inline elements.
fn is_inline_block_tag(tag: &BlockTag) -> bool {
    matches!(
        tag,
        BlockTag::Em
            | BlockTag::Strong
            | BlockTag::S
            | BlockTag::Span
            | BlockTag::Code
            | BlockTag::A
    )
}

/// Returns `true` for [`VoidTag`] variants that represent inline elements.
fn is_inline_void_tag(tag: &VoidTag) -> bool {
    matches!(tag, VoidTag::Br | VoidTag::Img)
}

/// Translates a node's [`NodeAttrs`] into HTML attributes: `id` to the `id`
/// attribute, `classes` to a `class` attribute, and a stored
/// [`Layout`](crate::layout::Layout) plus the CSS-bearing layers of a stored
/// [`Style`](crate::style::Style) to an inline `style` attribute.
///
/// Layout is skipped for inline nodes; the validation gate records a warning
/// when an inline node carries a layout, and the renderer drops it per D5.
/// `Style` is applied to both block nodes and inline `Span` nodes. The
/// layout and style declarations share a single `style` attribute and never
/// overwrite each other.
fn node_attributes(attrs: &NodeAttrs, inline: bool) -> Vec<HtmlAttribute> {
    let mut out = Vec::new();
    if let Some(id) = &attrs.id {
        out.push(HtmlAttribute::Id(DomId::new(id.clone())));
    }
    if !attrs.classes.is_empty() {
        out.push(HtmlAttribute::Class(ClassDefinition::new(
            attrs.classes.join(" "),
        )));
    }

    let mut decls: Vec<String> = Vec::new();
    if !inline
        && let Some(layout) = attrs.layout()
    {
        let css = layout_to_css(&layout);
        if !css.is_empty() {
            decls.push(css);
        }
    }
    // `Style` text-appearance/box-color CSS rides on both block and inline
    // nodes. Bold / italic / strikethrough emphasis lowers to CSS here for
    // block nodes; for inline nodes it is applied as semantic wrappers by
    // `wrap_style_emphasis` instead, so the CSS form is suppressed.
    if let Some(style) = attrs.style().filter(|s| !s.is_empty()) {
        let css = style_css_declarations(&style, !inline);
        if !css.is_empty() {
            decls.push(css);
        }
    }
    if !decls.is_empty() {
        out.push(HtmlAttribute::Other("style".into(), decls.join(";")));
    }
    out
}

/// Lowers the CSS-bearing layers of a [`Style`](crate::style::Style) to an
/// inline CSS declaration string.
///
/// Always covers `color`, `background` (as `background-color`), and the
/// underline / dim / blink emphasis layers. When `emit_emphasis` is `true`
/// (a block node) the bold / italic / strikethrough layers are *also* lowered
/// here, to `font-weight` / `font-style` / `text-decoration-line` — wrapping a
/// block element in `<strong>`/`<em>`/`<s>` would be invalid HTML. When it is
/// `false` (an inline node) those three layers are left to the semantic
/// wrappers applied by [`wrap_style_emphasis`].
///
/// `border` and `fill` are intentionally ignored until the broader Browser
/// Style box-painting work defines those semantics.
fn style_css_declarations(style: &crate::style::Style, emit_emphasis: bool) -> String {
    use crate::color::ColorMode;
    use crate::target::RenderTarget;

    let mut decls: Vec<String> = Vec::new();

    let resolve_color = |tv: &crate::layout::TargetValue<crate::style::PerMode<crate::color::Color>>|
     -> Option<String> {
        tv.resolve(RenderTarget::Browser)
            .map(|per_mode| *per_mode.resolve(ColorMode::Dark))
            .and_then(super::shared::color_to_css)
    };

    if let Some(color) = style.color.as_ref().and_then(&resolve_color) {
        decls.push(format!("color:{color}"));
    }
    if let Some(bg) = style.background.as_ref().and_then(&resolve_color) {
        decls.push(format!("background-color:{bg}"));
    }
    if let Some(underline) = style.emphasis.underline {
        // `css_declaration()` is human-spaced (`prop: value; prop: value`);
        // normalize to the compact `prop:value;prop:value` form used here.
        decls.push(
            underline
                .css_declaration()
                .replace("; ", ";")
                .replace(": ", ":"),
        );
    }
    if style.emphasis.dim {
        decls.push("opacity:0.6".into());
    }
    if style.emphasis.blink {
        decls.push("text-decoration:blink".into());
    }
    // Block emphasis: lower bold / italic / strikethrough to CSS rather than
    // to the semantic element wrappers used for inline nodes.
    if emit_emphasis {
        if style.emphasis.bold {
            decls.push("font-weight:bold".into());
        }
        if style.emphasis.italic {
            decls.push("font-style:italic".into());
        }
        if style.emphasis.strikethrough {
            decls.push("text-decoration-line:line-through".into());
        }
    }
    decls.join(";")
}

/// Wraps `fragment` in semantic emphasis tags (`<strong>`, `<em>`, `<s>`) for
/// a node's declared [`Style`](crate::style::Style), preserving nesting.
///
/// This applies only the bold / italic / strikethrough layers and is invoked
/// only for **inline** nodes — wrapping a block element in these tags is
/// invalid HTML, so a block node lowers the same layers to CSS through
/// [`style_css_declarations`]. The underline / dim / blink layers are always
/// applied as CSS by [`style_css_declarations`].
fn wrap_style_emphasis(attrs: &NodeAttrs, fragment: BrowserFragment<Ready>) -> BrowserFragment<Ready> {
    let Some(style) = attrs.style().filter(|s| !s.is_empty()) else {
        return fragment;
    };
    let emphasis = style.emphasis;
    let mut wrapped = fragment;
    // Innermost-first so the rendered nesting is `<strong><em><s>…`.
    if emphasis.strikethrough {
        wrapped = BrowserFragment::new()
            .define_as_block_tag(BlockTag::S, "")
            .add_component(wrapped)
            .finalize();
    }
    if emphasis.italic {
        wrapped = BrowserFragment::new()
            .define_as_block_tag(BlockTag::Em, "")
            .add_component(wrapped)
            .finalize();
    }
    if emphasis.bold {
        wrapped = BrowserFragment::new()
            .define_as_block_tag(BlockTag::Strong, "")
            .add_component(wrapped)
            .finalize();
    }
    wrapped
}

/// Lowers a [`Layout`](crate::layout::Layout) to an inline CSS declaration
/// string for the browser target.
///
/// Margin sides are resolved with [`TargetValue::resolve`] for
/// [`RenderTarget::Browser`]. Vertical sides (`top` / `bottom`) lower a
/// [`Length::Ch`] to `lh` (line-height units); horizontal sides lower it to
/// `ch`. A `max_width` adds `margin-left` / `margin-right: auto` per the
/// node's [`Alignment`]. [`WordWrap::None`] adds `white-space:nowrap`; any
/// wrapping variant adds `overflow-wrap:break-word`.
fn layout_to_css(layout: &crate::layout::Layout) -> String {
    use crate::layout::{Alignment, Length, TargetValue, WordWrap};
    use crate::target::RenderTarget;

    fn resolve(tv: &TargetValue<Length>) -> Option<&Length> {
        tv.resolve(RenderTarget::Browser)
    }
    fn css_len(len: &Length, vertical: bool) -> String {
        match len {
            Length::Zero => "0".into(),
            Length::Ch(n) if vertical => format!("{n}lh"),
            Length::Ch(n) => format!("{n}ch"),
            Length::Percent(p) => format!("{p}%"),
            Length::Css(sizing) => sizing.to_string(),
        }
    }

    let m = &layout.margin;
    let mut decls: Vec<String> = Vec::new();
    if let Some(l) = resolve(&m.top) {
        decls.push(format!("margin-top:{}", css_len(l, true)));
    }
    if let Some(l) = resolve(&m.bottom) {
        decls.push(format!("margin-bottom:{}", css_len(l, true)));
    }
    if let Some(l) = resolve(&m.left) {
        decls.push(format!("margin-left:{}", css_len(l, false)));
    }
    if let Some(l) = resolve(&m.right) {
        decls.push(format!("margin-right:{}", css_len(l, false)));
    }
    if let Some(mw) = layout.max_width.as_ref().and_then(resolve) {
        decls.push(format!("max-width:{}", css_len(mw, false)));
        match layout.alignment {
            Alignment::Center => {
                decls.push("margin-left:auto".into());
                decls.push("margin-right:auto".into());
            }
            Alignment::Right => {
                decls.push("margin-left:auto".into());
            }
            Alignment::Left => {}
        }
    }
    match layout.word_wrap {
        WordWrap::None => decls.push("white-space:nowrap".into()),
        WordWrap::WrapProse(..) | WordWrap::BespokeProse(..) | WordWrap::Truncate(_) => {
            decls.push("overflow-wrap:break-word".into());
        }
    }
    decls.join(";")
}

/// Builds a finalized text fragment. The fragment renderer HTML-escapes the
/// content on emit.
fn text_fragment(value: &str) -> BrowserFragment<Ready> {
    BrowserFragment::new()
        .define_as_text_fragment(value.to_string())
        .finalize()
}

/// Builds a disabled `<input type=checkbox>` reflecting a task item's state.
fn checkbox(checked: bool) -> BrowserFragment<Ready> {
    BrowserFragment::new()
        .define_as_void_tag(VoidTag::Input)
        .add_attribute(HtmlAttribute::Type(HtmlType::Checkbox))
        .add_attribute(HtmlAttribute::Disabled(true))
        .add_attribute(HtmlAttribute::Checked(checked))
        .finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::document::{DocumentMetadata, FrontmatterFormat};
    use crate::tree::node::HeadingDepth;
    use crate::tree::{Frontmatter, SourceRegistry};

    fn opts(strictness: RenderStrictness, raw_html: RawHtmlPolicy) -> BrowserRenderOptions {
        BrowserRenderOptions {
            strictness,
            raw_html,
            page: None,
        }
    }

    fn render(node: &RenderNode) -> Rendered<BrowserFragment<Ready>> {
        render_browser_node(node, &BrowserRenderOptions::default()).expect("render")
    }

    fn html(node: &RenderNode) -> String {
        render(node).output.render()
    }

    #[test]
    fn default_options_use_warn_and_escape() {
        let opts = BrowserRenderOptions::default();
        assert_eq!(opts.strictness, RenderStrictness::Warn);
        assert_eq!(opts.raw_html, RawHtmlPolicy::Escape);
        assert!(opts.page.is_none());
    }

    #[test]
    fn paragraph_and_heading() {
        let para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        assert_eq!(html(&para), "<p>hi</p>");

        let heading = RenderNode::heading(
            HeadingDepth::new(3).unwrap(),
            vec![RenderNode::text("Title")],
        );
        assert_eq!(html(&heading), "<h3>Title</h3>");
    }

    #[test]
    fn emphasis_strong_delete() {
        let em = RenderNode::emphasis(vec![RenderNode::text("e")]);
        let st = RenderNode::strong(vec![RenderNode::text("s")]);
        let de = RenderNode::delete(vec![RenderNode::text("d")]);
        assert_eq!(html(&em), "<em>e</em>");
        assert_eq!(html(&st), "<strong>s</strong>");
        assert_eq!(html(&de), "<s>d</s>");
    }

    #[test]
    fn text_is_escaped() {
        let para = RenderNode::paragraph(vec![RenderNode::text("a < b & c")]);
        assert_eq!(html(&para), "<p>a &lt; b &amp; c</p>");
    }

    #[test]
    fn unordered_and_ordered_lists() {
        let ul = RenderNode::list(
            false,
            None,
            vec![
                RenderNode::list_item(None, vec![RenderNode::text("a")]),
                RenderNode::list_item(None, vec![RenderNode::text("b")]),
            ],
        );
        assert_eq!(html(&ul), "<ul><li>a</li><li>b</li></ul>");

        let ol = RenderNode::list(
            true,
            Some(3),
            vec![RenderNode::list_item(None, vec![RenderNode::text("x")])],
        );
        assert_eq!(html(&ol), r#"<ol start="3"><li>x</li></ol>"#);
    }

    #[test]
    fn task_list_item_emits_disabled_checkbox() {
        let item = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(
                Some(true),
                vec![RenderNode::text("done")],
            )],
        );
        let out = html(&item);
        assert!(out.contains(r#"class="task-list-item""#), "{out}");
        assert!(out.contains(r#"type="checkbox""#), "{out}");
        assert!(out.contains("disabled"), "{out}");
        assert!(out.contains("checked"), "{out}");
    }

    #[test]
    fn code_block_and_inline_code() {
        let block = RenderNode::code(Some("rust".into()), None, "let a = 1;");
        assert_eq!(
            html(&block),
            r#"<pre><code class="language-rust">let a = 1;</code></pre>"#
        );

        let inline = RenderNode::inline_code("x < y");
        assert_eq!(html(&inline), "<code>x &lt; y</code>");
    }

    #[test]
    fn link_and_image() {
        let link = RenderNode::link(
            "https://example.com",
            Some("Site".into()),
            vec![RenderNode::text("here")],
        );
        assert_eq!(
            html(&link),
            r#"<a href="https://example.com" title="Site">here</a>"#
        );

        let image = RenderNode::image("img.png", None, "alt text");
        assert_eq!(html(&image), r#"<img src="img.png" alt="alt text">"#);
    }

    #[test]
    fn thematic_break_and_breaks() {
        assert_eq!(html(&RenderNode::thematic_break()), "<hr>");
        assert_eq!(html(&RenderNode::hard_break()), "<br>");
        assert_eq!(html(&RenderNode::soft_break()), " ");
    }

    #[test]
    fn block_quote() {
        let bq =
            RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text("quote")])]);
        assert_eq!(html(&bq), "<blockquote><p>quote</p></blockquote>");
    }

    #[test]
    fn span_carries_classes() {
        let span = RenderNode::span(vec!["hl".into(), "big".into()], vec![RenderNode::text("x")]);
        assert_eq!(html(&span), r#"<span class="hl big">x</span>"#);
    }

    #[test]
    fn node_id_maps_to_id_attribute() {
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.id = Some("intro".into());
        assert_eq!(html(&para), r#"<p id="intro">x</p>"#);
    }

    #[test]
    fn table_header_row_becomes_thead() {
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
        let out = html(&table);
        assert!(out.starts_with("<table><thead><tr>"), "{out}");
        assert!(
            out.contains(r#"<th style="text-align:left">H1</th>"#),
            "{out}"
        );
        assert!(
            out.contains(r#"<th style="text-align:right">H2</th>"#),
            "{out}"
        );
        assert!(out.contains("<tbody><tr>"), "{out}");
        assert!(
            out.contains(r#"<td style="text-align:left">a</td>"#),
            "{out}"
        );
        assert!(out.ends_with("</tbody></table>"), "{out}");
    }

    #[test]
    fn footnotes() {
        let reference = RenderNode::footnote_reference("1");
        assert_eq!(html(&reference), r##"<a href="#fn-1">1</a>"##);

        let definition = RenderNode::footnote_definition(
            "1",
            vec![RenderNode::paragraph(vec![RenderNode::text("note")])],
        );
        // The fragment renderer emits the merged `class` ahead of `id`.
        assert_eq!(
            html(&definition),
            r#"<div class="footnote-definition" id="fn-1"><p>note</p></div>"#
        );
    }

    #[test]
    fn root_wraps_children_in_a_div() {
        let root = RenderNode::root(vec![
            RenderNode::heading(HeadingDepth::new(1).unwrap(), vec![RenderNode::text("T")]),
            RenderNode::paragraph(vec![RenderNode::text("body")]),
        ]);
        assert_eq!(html(&root), "<div><h1>T</h1><p>body</p></div>");
    }

    #[test]
    fn raw_html_allow_emits_verbatim() {
        let node = RenderNode::html("<b>raw</b>", false);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Warn, RawHtmlPolicy::Allow))
                .expect("render");
        assert_eq!(rendered.output.render(), "<b>raw</b>");
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn raw_html_escape_emits_escaped_text_with_diagnostic() {
        let node = RenderNode::html("<b>raw</b>", false);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Warn, RawHtmlPolicy::Escape))
                .expect("render");
        assert_eq!(rendered.output.render(), "&lt;b&gt;raw&lt;/b&gt;");
        assert_eq!(rendered.diagnostics.len(), 1);
    }

    #[test]
    fn raw_html_escape_is_silent_under_lossy() {
        let node = RenderNode::html("<b>raw</b>", false);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Lossy, RawHtmlPolicy::Escape))
                .expect("render");
        assert_eq!(rendered.output.render(), "&lt;b&gt;raw&lt;/b&gt;");
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn raw_html_reject_errors_under_strict() {
        let node = RenderNode::html("<b>raw</b>", false);
        let result = render_browser_node(
            &node,
            &opts(RenderStrictness::Strict, RawHtmlPolicy::Reject),
        );
        assert!(matches!(result, Err(RenderError::LossyRejected { .. })));
    }

    #[test]
    fn raw_html_reject_degrades_under_warn() {
        let node = RenderNode::html("<b>raw</b>", false);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Warn, RawHtmlPolicy::Reject))
                .expect("render");
        assert_eq!(rendered.output.render(), "&lt;b&gt;raw&lt;/b&gt;");
        assert_eq!(rendered.diagnostics.len(), 1);
    }

    #[test]
    fn unsupported_fails_in_strict_mode() {
        // The validation gate escalates the `Unsupported` warning to
        // `InvalidTree` before the node-level path is reached.
        let node = RenderNode::root(vec![RenderNode::unsupported("custom")]);
        let result = render_browser_node(
            &node,
            &opts(RenderStrictness::Strict, RawHtmlPolicy::Escape),
        );
        assert!(matches!(result, Err(RenderError::InvalidTree { .. })));
    }

    #[test]
    fn unsupported_emits_comment_in_warn_mode() {
        let node = RenderNode::root(vec![RenderNode::unsupported("custom")]);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Warn, RawHtmlPolicy::Escape))
                .expect("render");
        assert_eq!(
            rendered.output.render(),
            "<div><!-- unsupported: custom --></div>"
        );
        // One validation-warning diagnostic plus one renderer Unsupported
        // diagnostic.
        assert_eq!(rendered.diagnostics.len(), 2);
    }

    #[test]
    fn unsupported_emits_nothing_in_lossy_mode() {
        let node = RenderNode::root(vec![RenderNode::unsupported("custom")]);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Lossy, RawHtmlPolicy::Escape))
                .expect("render");
        assert_eq!(rendered.output.render(), "<div></div>");
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn diagnostics_surface_in_rendered_result() {
        let node = RenderNode::root(vec![RenderNode::unsupported("thing")]);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Warn, RawHtmlPolicy::Escape))
                .expect("render");
        assert!(rendered.diagnostics.iter().any(|d| {
            d.kind == crate::tree::DiagnosticKind::Validation
                && d.message.contains("Unsupported node")
        }));
    }

    #[test]
    fn invalid_tree_errors_before_output() {
        let bad = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::table_cell(
            vec![RenderNode::text("x")],
        )])]);
        for strictness in [
            RenderStrictness::Strict,
            RenderStrictness::Warn,
            RenderStrictness::Lossy,
        ] {
            let result = render_browser_node(&bad, &opts(strictness, RawHtmlPolicy::Escape));
            assert!(matches!(result, Err(RenderError::InvalidTree { .. })));
        }
    }

    #[test]
    fn document_renders_root_children_as_page_fragments() {
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![
                RenderNode::heading(
                    HeadingDepth::new(1).unwrap(),
                    vec![RenderNode::text("Title")],
                ),
                RenderNode::paragraph(vec![RenderNode::text("Body")]),
            ]),
        };
        let rendered =
            render_browser_document(&doc, &BrowserRenderOptions::default()).expect("render");
        let html = rendered.output.render();
        assert!(
            html.contains("<body><h1>Title</h1><p>Body</p></body>"),
            "{html}"
        );
    }

    #[test]
    fn document_applies_page_options() {
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("x")])]),
        };
        let mut sheet = crate::stylesheet::Stylesheet::new();
        sheet.push(crate::stylesheet::CssRule::new(
            "body",
            crate::stylesheet::CssStyle::new(),
        ));
        let opts = BrowserRenderOptions {
            strictness: RenderStrictness::Warn,
            raw_html: RawHtmlPolicy::Escape,
            page: Some(PageOptions {
                stylesheet: Some(sheet),
                css_variables: Some(vec![("primary".into(), "#336699".into())]),
                external_stylesheet: None,
                external_code: None,
            }),
        };
        let rendered = render_browser_document(&doc, &opts).expect("render");
        let html = rendered.output.render();
        assert!(html.contains("--primary: #336699;"), "{html}");
    }

    #[test]
    fn document_ignores_frontmatter() {
        // Frontmatter is intentionally not wired into page metadata.
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata {
                frontmatter: Some(Frontmatter {
                    format: FrontmatterFormat::Yaml,
                    raw: "title: Ignored".into(),
                }),
            },
            root: RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("Body")])]),
        };
        let rendered =
            render_browser_document(&doc, &BrowserRenderOptions::default()).expect("render");
        let html = rendered.output.render();
        assert!(html.contains("<body><p>Body</p></body>"), "{html}");
        assert!(!html.contains("Ignored"), "{html}");
    }

    #[test]
    fn section_renders_as_section_element() {
        let section = RenderNode::section(
            HeadingDepth::new(2).unwrap(),
            vec![RenderNode::text("Title")],
            vec![RenderNode::paragraph(vec![RenderNode::text("Body")])],
        );
        let out = html(&section);
        assert!(out.starts_with("<section>"), "{out}");
        assert!(out.contains("<h2>Title</h2>"), "{out}");
        assert!(out.contains("<p>Body</p>"), "{out}");
        assert!(out.ends_with("</section>"), "{out}");
    }

    #[test]
    fn section_heading_uses_correct_depth() {
        for depth in 1..=6 {
            let section = RenderNode::section(
                HeadingDepth::new(depth).unwrap(),
                vec![RenderNode::text("T")],
                vec![],
            );
            let out = html(&section);
            assert!(out.contains(&format!("<h{depth}>T</h{depth}>")), "{out}");
        }
    }

    #[test]
    fn section_with_styled_heading() {
        let section = RenderNode::section(
            HeadingDepth::new(1).unwrap(),
            vec![
                RenderNode::text("Hello "),
                RenderNode::strong(vec![RenderNode::text("World")]),
            ],
            vec![],
        );
        let out = html(&section);
        assert!(
            out.contains("<h1>Hello <strong>World</strong></h1>"),
            "{out}"
        );
    }

    #[test]
    fn section_carries_node_attrs() {
        let mut section = RenderNode::section(
            HeadingDepth::new(2).unwrap(),
            vec![RenderNode::text("Title")],
            vec![],
        );
        section.attrs.id = Some("intro".into());
        section.attrs.classes = vec!["featured".into()];
        let out = html(&section);
        assert!(out.contains(r#"id="intro""#), "{out}");
        assert!(out.contains(r#"class="featured""#), "{out}");
    }

    #[test]
    fn browser_renderer_lowers_layout_to_css() {
        use crate::layout::{Alignment, Layout, Length, Margin, TargetValue};

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            margin: Margin::x(Length::ch(2)),
            alignment: Alignment::Center,
            max_width: Some(TargetValue::universal(Length::Percent(80.0))),
            ..Layout::default()
        });
        let root = RenderNode::root(vec![para]);

        let rendered = render_browser_node(&root, &BrowserRenderOptions::default()).unwrap();
        let html = rendered.output.render();
        assert!(
            html.contains("margin-left:2ch") || html.contains("margin-left: 2ch"),
            "{html}"
        );
        assert!(
            html.contains("max-width:80%") || html.contains("max-width: 80%"),
            "{html}"
        );
        assert!(
            html.contains("margin-left:auto")
                || html.contains("margin-right:auto")
                || html.contains("margin: 0 auto"),
            "{html}"
        );
    }

    #[test]
    fn browser_renderer_lowers_vertical_margin_to_lh() {
        use crate::layout::{Layout, Length, Margin};

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            margin: Margin::y(Length::ch(1)),
            ..Layout::default()
        });
        let root = RenderNode::root(vec![para]);
        let html = render_browser_node(&root, &BrowserRenderOptions::default())
            .unwrap()
            .output
            .render();
        assert!(html.contains("1lh"), "vertical Ch margin must lower to lh: {html}");
    }

    // ── RT-PROGRESS-001: browser progress ──────────────────────────────────

    fn progress_para(text: &str, hints: crate::tree::ProgressHints) -> RenderNode {
        let mut para = RenderNode::paragraph(vec![RenderNode::text(text)]);
        para.attrs.set_progress_hints(&hints);
        para
    }

    #[test]
    fn progress_emits_progressbar_role_and_aria() {
        let node = progress_para(
            "Loading 60%",
            crate::tree::ProgressHints { value: 0.6, ..Default::default() },
        );
        let out = html(&node);
        assert!(out.contains(r#"role="progressbar""#), "{out}");
        assert!(out.contains(r#"aria-valuemin="0""#), "{out}");
        assert!(out.contains(r#"aria-valuemax="100""#), "{out}");
        assert!(out.contains(r#"aria-valuenow="60""#), "{out}");
        assert!(out.contains(r#"aria-label="Loading""#), "{out}");
    }

    #[test]
    fn progress_uses_stable_classes_and_clamps_value() {
        let node = progress_para(
            "150%",
            crate::tree::ProgressHints { value: 1.5, ..Default::default() },
        );
        let out = html(&node);
        assert!(out.contains(r#"class="progress""#), "{out}");
        assert!(out.contains("progress-track"), "{out}");
        assert!(out.contains("progress-filled"), "{out}");
        assert!(out.contains("progress-percentage"), "{out}");
        // The value clamps to 1.0 → 100%.
        assert!(out.contains("width:100%"), "{out}");
    }

    #[test]
    fn progress_track_width_uses_bar_width_in_ch() {
        let node = progress_para(
            "25%",
            crate::tree::ProgressHints { value: 0.25, bar_width: 30, ..Default::default() },
        );
        let out = html(&node);
        assert!(out.contains("width:30ch"), "{out}");
    }

    #[test]
    fn progress_lowers_color_slots_to_background_color() {
        use crate::color::{BasicColor, Color};
        let node = progress_para(
            "50%",
            crate::tree::ProgressHints {
                value: 0.5,
                filled_color: Some(Color::BasicColor(BasicColor::Green)),
                empty_color: Some(Color::BasicColor(BasicColor::Red)),
                ..Default::default()
            },
        );
        let out = html(&node);
        // Two distinct background-color declarations.
        assert_eq!(out.matches("background-color:#").count(), 2, "{out}");
    }

    #[test]
    fn progress_preserves_custom_glyphs_in_data_attributes() {
        let node = progress_para(
            "10%",
            crate::tree::ProgressHints {
                value: 0.1,
                fill_char: '#',
                left_bracket: '(',
                right_bracket: ')',
                ..Default::default()
            },
        );
        let out = html(&node);
        assert!(out.contains(r##"data-fill-char="#""##), "{out}");
        assert!(out.contains(r#"data-left-bracket="(""#), "{out}");
        assert!(out.contains(r#"data-right-bracket=")""#), "{out}");
    }

    #[test]
    fn progress_applies_node_layout_to_outer_element() {
        use crate::layout::{Layout, Length, Margin};
        let mut node = progress_para(
            "40%",
            crate::tree::ProgressHints { value: 0.4, ..Default::default() },
        );
        node.attrs.set_layout(&Layout {
            margin: Margin::x(Length::ch(3)),
            ..Layout::default()
        });
        let out = html(&node);
        assert!(out.contains("margin-left:3ch"), "{out}");
    }

    #[test]
    fn plain_paragraph_without_progress_unchanged() {
        let para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        assert_eq!(html(&para), "<p>hi</p>");
    }

    /// Extracts the attribute list of the `<span class="progress-filled">`
    /// element so its `style` attributes can be asserted exactly.
    fn progress_filled_attrs(out: &str) -> &str {
        let after = out
            .split(r#"class="progress-filled""#)
            .nth(1)
            .expect("progress-filled span");
        &after[..after.find('>').expect("filled span close")]
    }

    #[test]
    fn progress_filled_color_and_width_share_one_style_attribute() {
        use crate::color::{BasicColor, Color};
        let node = progress_para(
            "50%",
            crate::tree::ProgressHints {
                value: 0.5,
                filled_color: Some(Color::BasicColor(BasicColor::Green)),
                ..Default::default()
            },
        );
        let out = html(&node);
        let filled = progress_filled_attrs(&out);
        // Exactly one `style` attribute, carrying both width and color.
        assert_eq!(filled.matches("style=").count(), 1, "{out}");
        assert!(filled.contains("width:50%"), "{out}");
        assert!(filled.contains("background-color:#"), "{out}");
    }

    #[test]
    fn progress_filled_without_color_has_single_width_style() {
        let node = progress_para(
            "25%",
            crate::tree::ProgressHints { value: 0.25, ..Default::default() },
        );
        let out = html(&node);
        let filled = progress_filled_attrs(&out);
        assert_eq!(filled.matches("style=").count(), 1, "{out}");
        assert!(filled.contains("width:25%"), "{out}");
        assert!(!filled.contains("background-color"), "{out}");
    }

    #[test]
    fn progress_label_with_special_chars_is_escaped_once() {
        let node = progress_para(
            r#"A < B & "C" 50%"#,
            crate::tree::ProgressHints { value: 0.5, ..Default::default() },
        );
        let out = html(&node);
        assert!(
            out.contains(r#"aria-label="A &lt; B &amp; &quot;C&quot;""#),
            "{out}"
        );
        // The label must be escaped exactly once — never double-escaped.
        assert!(!out.contains("&amp;lt;"), "double-escaped label: {out}");
    }

    #[test]
    fn progress_label_survives_value_clamp_mismatch() {
        // The fallback text says 150% but the value clamps to 100%; the label
        // is stripped by token shape, not by the clamped percentage, so it
        // does not disappear.
        let node = progress_para(
            "Loading 150%",
            crate::tree::ProgressHints { value: 1.5, ..Default::default() },
        );
        let out = html(&node);
        assert!(out.contains(r#"aria-label="Loading""#), "{out}");
        assert!(out.contains(r#"aria-valuenow="100""#), "{out}");
    }

    // ── RT-TEXTBLOCK-001: browser Style lowering ───────────────────────────

    fn styled_para(style: crate::style::Style) -> RenderNode {
        let mut para = RenderNode::paragraph(vec![RenderNode::text("text")]);
        para.attrs.set_style(&style);
        para
    }

    fn universal_color(c: crate::color::Color) -> crate::layout::TargetValue<crate::style::PerMode<crate::color::Color>> {
        crate::layout::TargetValue::universal(crate::style::PerMode::universal(c))
    }

    #[test]
    fn style_foreground_lowers_to_css_color() {
        use crate::color::{BasicColor, Color};
        let node = styled_para(crate::style::Style {
            color: Some(universal_color(Color::BasicColor(BasicColor::Red))),
            ..Default::default()
        });
        let out = html(&node);
        assert!(out.contains("color:#"), "{out}");
    }

    #[test]
    fn style_background_lowers_to_background_color() {
        use crate::color::{BasicColor, Color};
        let node = styled_para(crate::style::Style {
            background: Some(universal_color(Color::BasicColor(BasicColor::Blue))),
            ..Default::default()
        });
        let out = html(&node);
        assert!(out.contains("background-color:#"), "{out}");
    }

    #[test]
    fn style_emphasis_on_block_node_lowers_to_css() {
        use crate::style::{Style, TextEmphasis};
        // A styled *block* node lowers bold / italic / strikethrough to CSS —
        // wrapping a `<p>` in `<strong>`/`<em>`/`<s>` is invalid HTML.
        let node = styled_para(Style {
            emphasis: TextEmphasis {
                bold: true,
                italic: true,
                strikethrough: true,
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(
            html(&node),
            r#"<p style="font-weight:bold;font-style:italic;text-decoration-line:line-through">text</p>"#
        );
    }

    #[test]
    fn style_emphasis_on_inline_span_uses_semantic_wrappers() {
        use crate::style::{Style, TextEmphasis};
        // A styled *inline* `Span` keeps the semantic emphasis wrappers, with
        // `<strong>` outermost so the nesting is preserved.
        let mut span = RenderNode::span(vec![], vec![RenderNode::text("x")]);
        span.attrs.set_style(&Style {
            emphasis: TextEmphasis {
                bold: true,
                italic: true,
                strikethrough: true,
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(
            html(&span),
            "<strong><em><s><span>x</span></s></em></strong>"
        );
    }

    #[test]
    fn style_underline_variants_lower_with_css_declaration() {
        use crate::style::{Style, TextEmphasis, UnderlineStyle};
        for variant in [
            UnderlineStyle::Straight,
            UnderlineStyle::Double,
            UnderlineStyle::Curly,
            UnderlineStyle::Dotted,
            UnderlineStyle::Dashed,
        ] {
            let node = styled_para(Style {
                emphasis: TextEmphasis {
                    underline: Some(variant),
                    ..Default::default()
                },
                ..Default::default()
            });
            let out = html(&node);
            assert!(
                out.contains("text-decoration:underline"),
                "{variant:?}: {out}"
            );
        }
    }

    #[test]
    fn style_dim_lowers_to_opacity() {
        use crate::style::{Style, TextEmphasis};
        let node = styled_para(Style {
            emphasis: TextEmphasis { dim: true, ..Default::default() },
            ..Default::default()
        });
        assert!(html(&node).contains("opacity:0.6"));
    }

    #[test]
    fn style_blink_lowers_to_text_decoration_blink() {
        use crate::style::{Style, TextEmphasis};
        let node = styled_para(Style {
            emphasis: TextEmphasis { blink: true, ..Default::default() },
            ..Default::default()
        });
        assert!(html(&node).contains("text-decoration:blink"));
    }

    #[test]
    fn style_and_layout_share_one_style_attribute() {
        use crate::color::{BasicColor, Color};
        use crate::layout::{Layout, Length, Margin};
        let mut para = RenderNode::paragraph(vec![RenderNode::text("text")]);
        para.attrs.set_style(&crate::style::Style {
            color: Some(universal_color(Color::BasicColor(BasicColor::Red))),
            ..Default::default()
        });
        para.attrs.set_layout(&Layout {
            margin: Margin::x(Length::ch(2)),
            ..Layout::default()
        });
        let out = html(&RenderNode::root(vec![para]));
        // A single style attribute carrying both layout and style CSS.
        assert_eq!(out.matches("style=").count(), 1, "{out}");
        assert!(out.contains("margin-left:2ch"), "{out}");
        assert!(out.contains("color:#"), "{out}");
    }

    #[test]
    fn style_applies_to_inline_span() {
        use crate::color::{BasicColor, Color};
        let mut span = RenderNode::span(vec![], vec![RenderNode::text("x")]);
        span.attrs.set_style(&crate::style::Style {
            color: Some(universal_color(Color::BasicColor(BasicColor::Red))),
            ..Default::default()
        });
        let out = html(&span);
        assert!(out.contains("color:#"), "{out}");
    }

    #[test]
    fn paragraph_without_style_unchanged() {
        let para = RenderNode::paragraph(vec![RenderNode::text("plain")]);
        assert_eq!(html(&para), "<p>plain</p>");
    }

    // ── RT-TWOCOLUMN-001: browser columns CSS ──────────────────────────────

    fn columns_bq(hints: crate::tree::ColumnsHints, children: Vec<RenderNode>) -> RenderNode {
        let mut bq = RenderNode::block_quote(children);
        bq.attrs.set_columns_hints(&hints);
        bq
    }

    #[test]
    fn columns_default_emits_flex_container_and_classes() {
        let node = columns_bq(
            crate::tree::ColumnsHints { left_count: 1, ..Default::default() },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("L")]),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        let out = html(&node);
        assert!(out.contains(r#"<div class="columns""#), "{out}");
        assert!(out.contains("display:flex"), "{out}");
        assert!(out.contains("gap:3ch"), "{out}");
        assert_eq!(out.matches(r#"class="column""#).count(), 2, "{out}");
    }

    #[test]
    fn columns_fixed_left_width_lowers_to_flex_and_max_width() {
        let node = columns_bq(
            crate::tree::ColumnsHints {
                left_count: 1,
                left_width: crate::tree::ColumnWidthKind::Fixed(40),
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("L")]),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        let out = html(&node);
        assert!(out.contains("flex:0 0 40ch;max-width:40ch"), "{out}");
        assert!(out.contains("flex:1 1 0"), "{out}");
    }

    #[test]
    fn columns_percent_left_width_clamps_and_lowers_to_flex() {
        let node = columns_bq(
            crate::tree::ColumnsHints {
                left_count: 1,
                left_width: crate::tree::ColumnWidthKind::Percent(1.5),
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("L")]),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        let out = html(&node);
        // The fraction clamps to 1.0 → 100%.
        assert!(out.contains("flex:0 0 100%"), "{out}");
    }

    #[test]
    fn columns_custom_gap_lowers_to_gap_ch() {
        let node = columns_bq(
            crate::tree::ColumnsHints { gap: 8, ..Default::default() },
            vec![],
        );
        assert!(html(&node).contains("gap:8ch"));
    }

    #[test]
    fn columns_layout_and_column_css_coexist() {
        use crate::layout::{Layout, Length, Margin};
        let mut node = columns_bq(
            crate::tree::ColumnsHints::default(),
            vec![],
        );
        node.attrs.set_layout(&Layout {
            margin: Margin::x(Length::ch(2)),
            ..Layout::default()
        });
        let out = html(&node);
        assert!(out.contains("display:flex"), "{out}");
        assert!(out.contains("margin-left:2ch"), "{out}");
    }

    #[test]
    fn columns_empty_left_and_right() {
        let node = columns_bq(
            crate::tree::ColumnsHints { left_count: 0, ..Default::default() },
            vec![],
        );
        let out = html(&node);
        assert_eq!(out.matches(r#"class="column""#).count(), 2, "{out}");
    }

    #[test]
    fn columns_preserve_user_supplied_classes() {
        // A columns node carrying user classes keeps them as external-CSS
        // hooks, merged after the literal `columns` class.
        let mut node = columns_bq(
            crate::tree::ColumnsHints { left_count: 1, ..Default::default() },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("L")]),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        node.attrs.classes = vec!["hero".into(), "wide".into()];
        let out = html(&node);
        assert!(out.contains(r#"class="columns hero wide""#), "{out}");
        // The flex CSS still rides on the container alongside the user classes.
        assert!(out.contains("display:flex"), "{out}");
    }

    #[test]
    fn block_quote_without_columns_renders_normally() {
        let bq =
            RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text("q")])]);
        assert_eq!(html(&bq), "<blockquote><p>q</p></blockquote>");
    }

    // ── RT-TABLE-001: browser caption ──────────────────────────────────────

    #[test]
    fn table_title_emitted_as_caption_before_thead() {
        let mut table = RenderNode::table(
            vec![ColumnAlign::Left],
            vec![
                RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text(
                    "H",
                )])]),
            ],
        );
        table.attrs.set_table_title("Sales <2024>");
        let out = html(&table);
        assert!(
            out.contains("<table><caption>Sales &lt;2024&gt;</caption><thead>"),
            "{out}"
        );
    }

    #[test]
    fn whitespace_only_table_title_emits_no_caption() {
        let mut table = RenderNode::table(
            vec![ColumnAlign::Left],
            vec![RenderNode::table_row(vec![RenderNode::table_cell(vec![
                RenderNode::text("H"),
            ])])],
        );
        table.attrs.set_table_title("  ");
        assert!(!html(&table).contains("<caption>"));
    }

    #[test]
    fn table_cell_alignment_merges_with_existing_style() {
        use crate::color::{BasicColor, Color};
        // A body cell carrying a foreground `Style` and a column alignment
        // must emit exactly one `style` attribute holding both.
        let mut cell = RenderNode::table_cell(vec![RenderNode::text("a")]);
        cell.attrs.set_style(&crate::style::Style {
            color: Some(universal_color(Color::BasicColor(BasicColor::Red))),
            ..Default::default()
        });
        let table = RenderNode::table(
            vec![ColumnAlign::Right],
            vec![
                RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("H")])]),
                RenderNode::table_row(vec![cell]),
            ],
        );
        let out = html(&table);
        let after = out.split("<td").nth(1).expect("body cell");
        let td_attrs = &after[..after.find('>').expect("td close")];
        assert_eq!(td_attrs.matches("style=").count(), 1, "{out}");
        assert!(td_attrs.contains("text-align:right"), "{out}");
        assert!(td_attrs.contains("color:#"), "{out}");
    }

    // ── RT-FILESYSTEM-001: browser marker policy ───────────────────────────

    #[test]
    fn list_marker_policy_none_suppresses_bullet_via_css() {
        let mut list = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(None, vec![RenderNode::text("a")])],
        );
        list.attrs
            .set_list_marker_policy(crate::tree::ListMarkerPolicy::None);
        let out = html(&list);
        assert!(out.contains("list-style:none"), "{out}");
        // No terminal box-drawing connector text.
        assert!(!out.contains("├"), "{out}");
    }

    #[test]
    fn list_marker_policy_tree_connectors_degrades_to_native_list() {
        let mut list = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(None, vec![RenderNode::text("a")])],
        );
        list.attrs
            .set_list_marker_policy(crate::tree::ListMarkerPolicy::TreeConnectors);
        let out = html(&list);
        assert!(out.starts_with("<ul"), "{out}");
        assert!(!out.contains("└──"), "{out}");
        assert!(out.contains("list-style:none"), "{out}");
    }

    #[test]
    fn default_list_marker_policy_emits_plain_list() {
        let list = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(None, vec![RenderNode::text("a")])],
        );
        assert_eq!(html(&list), "<ul><li>a</li></ul>");
    }

    fn marker_policy_list(policy: crate::tree::ListMarkerPolicy) -> RenderNode {
        let mut list = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(None, vec![RenderNode::text("a")])],
        );
        list.attrs.set_list_marker_policy(policy);
        list
    }

    #[test]
    fn marker_policy_none_is_faithful_with_no_diagnostic() {
        // `None` maps faithfully to the browser's own no-marker list, so it
        // degrades silently in every strictness mode.
        for strictness in [
            RenderStrictness::Strict,
            RenderStrictness::Warn,
            RenderStrictness::Lossy,
        ] {
            let list = marker_policy_list(crate::tree::ListMarkerPolicy::None);
            let rendered =
                render_browser_node(&list, &opts(strictness, RawHtmlPolicy::Escape))
                    .expect("None marker policy is faithful in every mode");
            assert!(rendered.output.render().contains("list-style:none"));
            assert!(
                rendered.diagnostics.is_empty(),
                "None is faithful — no diagnostic: {strictness:?}"
            );
        }
    }

    #[test]
    fn marker_policy_tree_connectors_rejected_under_strict() {
        let list = marker_policy_list(crate::tree::ListMarkerPolicy::TreeConnectors);
        let result =
            render_browser_node(&list, &opts(RenderStrictness::Strict, RawHtmlPolicy::Escape));
        assert!(matches!(result, Err(RenderError::LossyRejected { .. })));
    }

    #[test]
    fn marker_policy_tree_connectors_degrades_with_diagnostic_under_warn() {
        let list = marker_policy_list(crate::tree::ListMarkerPolicy::TreeConnectors);
        let rendered =
            render_browser_node(&list, &opts(RenderStrictness::Warn, RawHtmlPolicy::Escape))
                .expect("Warn degrades TreeConnectors");
        assert!(rendered.output.render().contains("list-style:none"));
        assert_eq!(rendered.diagnostics.len(), 1, "one lossy diagnostic");
    }

    #[test]
    fn marker_policy_tree_connectors_silent_under_lossy() {
        let list = marker_policy_list(crate::tree::ListMarkerPolicy::TreeConnectors);
        let rendered =
            render_browser_node(&list, &opts(RenderStrictness::Lossy, RawHtmlPolicy::Escape))
                .expect("Lossy degrades TreeConnectors silently");
        assert!(rendered.output.render().contains("list-style:none"));
        assert!(rendered.diagnostics.is_empty());
    }

    // ── RT-TODO-001: browser task-state ────────────────────────────────────

    #[test]
    fn task_state_cancelled_and_in_progress_keep_portable_checkbox() {
        use crate::tree::{TaskHints, TaskState};
        for state in [TaskState::Cancelled, TaskState::InProgress] {
            let mut item = RenderNode::list_item(Some(false), vec![RenderNode::text("t")]);
            item.attrs.set_task_hints(&TaskHints { state });
            let list = RenderNode::list(false, None, vec![item]);
            let out = html(&list);
            assert!(out.contains(r#"type="checkbox""#), "{state:?}: {out}");
            assert!(out.contains(r#"class="task-list-item""#), "{state:?}: {out}");
            assert!(out.contains("disabled"), "{state:?}: {out}");
            // Terminal-only task glyphs must never leak into browser output.
            for glyph in ['✔', '✗', '⏺', '☐', '⊝'] {
                assert!(
                    !out.contains(glyph),
                    "{state:?} leaked terminal glyph {glyph}: {out}"
                );
            }
        }
    }

    // ── RT-COMPOSE-001: browser sequence join ──────────────────────────────

    #[test]
    fn root_with_sequence_join_renders_children_in_div() {
        // The browser ignores the sequence-join policy (it has no block
        // separators to suppress); children still render in order in a `div`.
        let mut root = RenderNode::root(vec![
            RenderNode::text("inline"),
            RenderNode::paragraph(vec![RenderNode::text("para")]),
        ]);
        root.attrs.set_sequence_join(crate::tree::SequenceJoin::None);
        let rendered =
            render_browser_node(&root, &BrowserRenderOptions::default()).expect("render");
        assert_eq!(rendered.output.render(), "<div>inline<p>para</p></div>");
    }

    #[test]
    fn document_with_sequence_join_root_renders_through_entry_point() {
        let mut root = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("x")])]);
        root.attrs.set_sequence_join(crate::tree::SequenceJoin::None);
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root,
        };
        let rendered =
            render_browser_document(&doc, &BrowserRenderOptions::default()).expect("render");
        assert!(
            rendered.output.render().contains("<body><p>x</p></body>"),
            "{}",
            rendered.output.render()
        );
    }

    #[test]
    fn nested_sections_render_correctly() {
        let tree = RenderNode::root(vec![RenderNode::section(
            HeadingDepth::new(1).unwrap(),
            vec![RenderNode::text("Parent")],
            vec![RenderNode::section(
                HeadingDepth::new(2).unwrap(),
                vec![RenderNode::text("Child")],
                vec![RenderNode::paragraph(vec![RenderNode::text("Content")])],
            )],
        )]);
        let out = html(&tree);
        assert!(
            out.contains(
                "<section><h1>Parent</h1><section><h2>Child</h2><p>Content</p></section></section>"
            ),
            "{out}"
        );
    }
}
