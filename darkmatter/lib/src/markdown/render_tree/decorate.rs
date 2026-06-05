//! Lowers a [`LayoutContext`]'s per-component layout / color settings onto a
//! folded render tree.
//!
//! The decorated-layout terminal path
//! ([`render_tree_terminal_with_layout`](super::entrypoints::render_tree_terminal_with_layout))
//! folds a [`Markdown`](crate::markdown::Markdown) document, then runs this pass
//! to project each [`PageComponent`]'s alignment, fill width, and colors onto
//! the matching tree node's [`Layout`] / [`Style`]. The shared terminal tree
//! renderer then applies those attributes (left margin = alignment offset +
//! side padding; `max_width` = the component's resolved fill width).
//!
//! Page-level framing (margins, padding, background, vertical rhythm) is **not**
//! handled here — the [`DarkmatterPage`](crate::layout::DarkmatterPage)
//! row-decoration post-pass wraps the rendered body string for that.

// `PageComponent` / `PageFill` are the deprecated page-layout enums that remain
// the internal storage for `LayoutContext`; deprecation is suppressed here while
// the bespoke layout types coexist with `renderable::layout` (matching
// `layout/context.rs`).
#![allow(deprecated)]

use renderable::layout::{Alignment, Length, Edges, TargetValue};
use renderable::style::{PerMode, Style};
use renderable::tree::{HintNamespace, NodeKind, RenderNode};

use crate::layout::{LayoutContext, PageAlignment, PageComponent};
use crate::style::StyleColor;

/// Walks `root`, decorating every node that maps to a [`PageComponent`].
///
/// The page-level foreground / background colors are applied to the root so
/// they inherit to every descendant's text (matching legacy, which pushes a
/// page color scope around the whole body); per-component colors set deeper
/// override them through the tree's style inheritance.
pub(crate) fn decorate_document(root: &mut RenderNode, ctx: &LayoutContext) {
    apply_page_color(root, ctx);
    decorate_node(root, ctx);
}

/// Applies the page-level foreground / background color to the root node so it
/// inherits to all descendant text.
fn apply_page_color(root: &mut RenderNode, ctx: &LayoutContext) {
    let fg = ctx.page_color.as_ref();
    let bg = ctx.page_bg_color.as_ref();
    if fg.is_none() && bg.is_none() {
        return;
    }
    let mut style = root.attrs.style().unwrap_or_default();
    set_style_colors(&mut style, fg, bg);
    root.attrs.set_style(&style);
}

/// Recursively decorates `node` and its descendants.
fn decorate_node(node: &mut RenderNode, ctx: &LayoutContext) {
    if let Some(component) = component_for(&node.kind) {
        apply_component_layout(node, ctx, component);
        apply_component_color(node, ctx, component);
        if matches!(node.kind, NodeKind::ListItem { .. }) {
            apply_list_item_alignment(node, ctx);
        }
    } else if let Some(alt) = lone_image_alt(node) {
        // A paragraph whose only content is an image is the block-level image
        // the legacy renderer aligns / fills / colors. The `Image` node itself
        // is inline (the renderer ignores its `Layout`), so the Images
        // component layout is lifted onto the wrapping paragraph instead.
        apply_lone_image_layout(node, ctx, &alt);
    }
    apply_inline_color(node, ctx);
    apply_inline_label_layout(node, ctx);

    if let Some(children) = node.children_mut() {
        for child in children {
            decorate_node(child, ctx);
        }
    }
}

/// Returns the alt text of the single image when `node` is a paragraph whose
/// only meaningful child is one image, else `None`.
fn lone_image_alt(node: &RenderNode) -> Option<String> {
    let NodeKind::Paragraph { children } = &node.kind else {
        return None;
    };
    let mut images = 0;
    let mut alt = String::new();
    for child in children {
        match &child.kind {
            NodeKind::Image { alt: a, .. } => {
                images += 1;
                alt = a.clone();
            }
            // Allow surrounding whitespace-only text / breaks.
            NodeKind::Text { value } if value.trim().is_empty() => {}
            NodeKind::SoftBreak | NodeKind::HardBreak => {}
            _ => return None,
        }
    }
    (images == 1).then_some(alt)
}

/// Maps a [`NodeKind`] to the [`PageComponent`] whose layout settings govern it.
///
/// `None` for kinds that carry no page-component layout (text, headings,
/// paragraphs, …) — those inherit the page frame only.
fn component_for(kind: &NodeKind) -> Option<PageComponent> {
    match kind {
        NodeKind::Code { .. } => Some(PageComponent::CodeBlocks),
        NodeKind::Table { .. } => Some(PageComponent::Tables),
        NodeKind::BlockQuote { .. } => Some(PageComponent::BlockQuotes),
        NodeKind::List { ordered: true, .. } => Some(PageComponent::Ol),
        NodeKind::List { ordered: false, .. } => Some(PageComponent::Ul),
        NodeKind::ListItem { .. } => Some(PageComponent::Li),
        NodeKind::Image { .. } => Some(PageComponent::Images),
        NodeKind::ThematicBreak => Some(PageComponent::Hr),
        _ => None,
    }
}

/// Sets the node's [`Layout`] from the component's resolved fill + alignment,
/// mirroring the legacy `for_terminal_with_layout` model.
///
/// The component body is capped to [`resolve_component_width`] (which already
/// folds in `Indent` / `Pad` / `Max` / `Explicit`). The left offset is computed
/// per component family — the legacy renderer treats them differently:
///
/// - **Code / Table / Image** (legacy `apply_component_layout`): `Pad` / `Indent`
///   left offset is [`component_side_padding`]'s left; `Full` / `Max` /
///   `Explicit` use [`alignment_padding`] (zero when left-aligned).
/// - **Block quote** (legacy streaming path): [`alignment_padding`] only.
/// - **Lists** (`Ul` / `Ol` / `Li`, legacy streaming path): the list-left-margin
///   plus [`alignment_padding`] of `(body + margin)`.
///
/// The offset is baked into the left margin and the node is left-aligned, so the
/// shared renderer positions the band exactly where the legacy renderer did.
///
/// [`resolve_component_width`]: LayoutContext::resolve_component_width
/// [`alignment_padding`]: LayoutContext::alignment_padding
/// [`component_side_padding`]: LayoutContext::component_side_padding
fn apply_component_layout(node: &mut RenderNode, ctx: &LayoutContext, component: PageComponent) {
    let width = match ctx.resolve_component_width(component) {
        Ok(w) => w,
        // A malformed percent etc. degrades to no layout rather than failing
        // the whole render; the body still renders at the page width.
        Err(_) => return,
    };

    let (left, body_width) = match component {
        // Lists: the left-margin shifts the whole list right and narrows its
        // body; (margin + body) aligns as one block.
        PageComponent::Ul | PageComponent::Ol | PageComponent::Li => {
            let left_margin = list_left_margin_cols(ctx, component);
            let body = width.min(ctx.effective_width.saturating_sub(left_margin));
            let block = body.saturating_add(left_margin);
            let align = ctx.alignment_padding(component, block);
            (left_margin.saturating_add(align), body)
        }
        // Block quotes: alignment offset only (the bar provides the indent for
        // `Indent`, already folded into `width`).
        PageComponent::BlockQuotes => (ctx.alignment_padding(component, width), width),
        // Code / Table / Image: `Pad` / `Indent` use the side padding; otherwise
        // the alignment offset against the rendered width.
        _ => (block_component_left_pad(ctx, component, width), width),
    };

    // Nothing to do when the body fills the full effective width with no offset:
    // that is the renderer's default and pinning a Layout would be redundant.
    if left == 0 && body_width >= ctx.effective_width {
        return;
    }

    let mut layout = node.attrs.layout().unwrap_or_default();
    layout.margin = Edges {
        left: cells(left),
        ..layout.margin
    };
    // The offset is baked into the left margin above, so the node is
    // left-aligned; setting the typed alignment too would double the offset.
    layout.alignment = Alignment::Left;
    layout.max_width = Some(TargetValue::universal(Length::ch(u32::from(body_width))));
    node.attrs.set_layout(&layout);
}

/// Lifts the Images-component layout onto a lone-image paragraph.
///
/// Mirrors legacy `apply_component_layout(PageComponent::Images)`: for `Pad` /
/// `Indent` fills the left offset is the side padding; for `Full` / `Max` /
/// `Explicit` with a non-Left alignment the offset is computed against the
/// **rendered placeholder width** (`▉ IMAGE[{alt}]`), not the fill width — legacy
/// centers / right-aligns the placeholder text itself. The `Image` node is
/// inline (the renderer ignores its `Layout`), so the offset is baked into the
/// wrapping paragraph's left margin.
fn apply_lone_image_layout(node: &mut RenderNode, ctx: &LayoutContext, alt: &str) {
    use crate::layout::{PageAlignment, PageFill};
    use biscuit_terminal::utils::UnicodeWidthStr;

    let component = PageComponent::Images;
    let left = match ctx.component_fill(component) {
        PageFill::Pad(_) | PageFill::Indent(_) => ctx
            .component_side_padding(component)
            .map(|(l, _)| l)
            .unwrap_or(0),
        PageFill::Full | PageFill::Max(_) | PageFill::Explicit(_) => {
            if ctx.component_alignment(component) == PageAlignment::Left {
                0
            } else {
                // `▉ IMAGE[{alt}]` — the placeholder's visible width is what the
                // legacy renderer aligns against.
                let placeholder = format!("▉ IMAGE[{alt}]");
                let visible = UnicodeWidthStr::width(placeholder.as_str()) as u16;
                ctx.alignment_padding(component, visible)
            }
        }
    };

    if left == 0 {
        return;
    }
    let mut layout = node.attrs.layout().unwrap_or_default();
    layout.margin = Edges {
        left: cells(left),
        ..layout.margin
    };
    layout.alignment = Alignment::Left;
    node.attrs.set_layout(&layout);
}

/// Computes the left offset for a Code / Table / Image component, mirroring the
/// legacy `apply_component_layout`: `Pad` / `Indent` use the side padding's left,
/// otherwise the alignment offset against the rendered `width`.
fn block_component_left_pad(ctx: &LayoutContext, component: PageComponent, width: u16) -> u16 {
    use crate::layout::PageFill;
    match ctx.component_fill(component) {
        PageFill::Pad(_) | PageFill::Indent(_) => ctx
            .component_side_padding(component)
            .map(|(l, _)| l)
            .unwrap_or(0),
        PageFill::Full | PageFill::Max(_) | PageFill::Explicit(_) => {
            ctx.alignment_padding(component, width)
        }
    }
}

/// Returns the list-left-margin (in columns) for an unordered list, else 0.
///
/// Mirrors legacy, which applies `style.ul.left-margin` to [`PageComponent::Ul`]
/// only and resolves it against `effective_width`.
fn list_left_margin_cols(ctx: &LayoutContext, component: PageComponent) -> u16 {
    if component != PageComponent::Ul {
        return 0;
    }
    ctx.list_left_margin(component)
        .and_then(|unit| unit.resolve(ctx.effective_width).ok())
        .unwrap_or(0)
}

/// Sets the node's [`Style`] foreground / background from the component colors.
fn apply_component_color(node: &mut RenderNode, ctx: &LayoutContext, component: PageComponent) {
    let fg = ctx.component_color(component).cloned();
    let bg = ctx.component_bg_color(component).cloned();
    if fg.is_none() && bg.is_none() {
        return;
    }
    let mut style = node.attrs.style().unwrap_or_default();
    set_style_colors(&mut style, fg.as_ref(), bg.as_ref());
    node.attrs.set_style(&style);
}

/// Applies hyperlink / image inline colors to link / image nodes.
///
/// Links and images are *inline* and carry no [`PageComponent`] layout, so
/// their colors are projected here (separately from the block-component pass).
fn apply_inline_color(node: &mut RenderNode, ctx: &LayoutContext) {
    match &node.kind {
        NodeKind::Link { url, .. } => {
            let is_local = is_local_link(url);
            let (fg, bg) = ctx.hyperlink_color(is_local);
            if fg.is_some() || bg.is_some() {
                let mut style = node.attrs.style().unwrap_or_default();
                set_style_colors(&mut style, fg.as_ref(), bg.as_ref());
                node.attrs.set_style(&style);
            }
        }
        NodeKind::Image { url, .. } => {
            let is_local = crate::style::bespoke::is_local_image(url);
            let (fg, bg) = ctx.image_color(is_local);
            if fg.is_some() || bg.is_some() {
                let mut style = node.attrs.style().unwrap_or_default();
                set_style_colors(&mut style, fg.as_ref(), bg.as_ref());
                node.attrs.set_style(&style);
            }
        }
        _ => {}
    }
}

/// Applies hyperlink-label and image-alt width/alignment/truncation to inline
/// link / image nodes, mirroring legacy `for_terminal_with_layout`.
///
/// - **Link.** When the effective `style.hyperlinks.*` [`CommonStyle`] sets
///   `width` or `max-width`, the link's accumulated plain label is padded /
///   truncated / aligned via
///   [`apply_inline_text_layout`](crate::style::bespoke::apply_inline_text_layout)
///   and the link's children are replaced with a single text node carrying the
///   transformed label. Legacy padded the accumulated plain `current_link_text`,
///   so flattening to plain text matches; the renderer still wraps the padded
///   label in the OSC8 sequence. Alignment-only styles are a no-op on the label
///   (legacy padded only when a width was set).
/// - **Image.** When the local `style.images.local-style` sets `width` or
///   `max-width`, the image node's `alt` is transformed the same way. The
///   block-level Images alignment / fill is handled separately by
///   [`apply_lone_image_layout`].
fn apply_inline_label_layout(node: &mut RenderNode, ctx: &LayoutContext) {
    match &node.kind {
        NodeKind::Link { url, .. } => {
            let is_local = is_local_link(url);
            let Some(common) = ctx.effective_hyperlink_style(is_local) else {
                return;
            };
            if common.width.is_none() && common.max_width.is_none() {
                return;
            }
            let label = inline_plain_text(node);
            let transformed = crate::style::bespoke::apply_inline_text_layout(
                &label,
                Some(&common),
                ctx.effective_width,
            );
            if let NodeKind::Link { children, .. } = &mut node.kind {
                *children = vec![RenderNode::text(transformed)];
            }
        }
        NodeKind::Image { url, alt, .. } => {
            if !crate::style::bespoke::is_local_image(url) {
                return;
            }
            let Some(common) = ctx.local_image_style.as_ref() else {
                return;
            };
            if common.width.is_none() && common.max_width.is_none() {
                return;
            }
            let transformed = crate::style::bespoke::apply_inline_text_layout(
                alt,
                Some(common),
                ctx.effective_width,
            );
            if let NodeKind::Image { alt, .. } = &mut node.kind {
                *alt = transformed;
            }
        }
        _ => {}
    }
}

/// Collects the visible plain text of an inline node's children (link label),
/// ignoring styling. Matches legacy's accumulated `current_link_text`.
fn inline_plain_text(node: &RenderNode) -> String {
    fn walk(node: &RenderNode, out: &mut String) {
        if let NodeKind::Text { value } = &node.kind {
            out.push_str(value);
        }
        for child in node.children() {
            walk(child, out);
        }
    }
    let mut out = String::new();
    for child in node.children() {
        walk(child, &mut out);
    }
    out
}

/// Sets the `darkmatter.li` alignment hint on a `Center` / `Right`-aligned list
/// item so the shared renderer lifts the marker onto its own line and left-pads
/// the body block.
///
/// The pad is the resolved left offset
/// ([`alignment_padding`](LayoutContext::alignment_padding) of the `Li`
/// component width), precomputed here so the renderer stays free of darkmatter's
/// page-layout types. `Left`-aligned lists (the default) set no hint, so the
/// renderer keeps its inline rendering byte-for-byte.
fn apply_list_item_alignment(node: &mut RenderNode, ctx: &LayoutContext) {
    let alignment = ctx.component_alignment(PageComponent::Li);
    let label = match alignment {
        PageAlignment::Left => return,
        PageAlignment::Center => "center",
        PageAlignment::Right => "right",
    };
    let li_width = ctx
        .resolve_component_width(PageComponent::Li)
        .unwrap_or(ctx.effective_width);
    let pad = ctx.alignment_padding(PageComponent::Li, li_width);
    let ns = HintNamespace("darkmatter.li");
    node.attrs.set_hint(ns, "alignment", serde_json::json!(label));
    node.attrs.set_hint(ns, "pad", serde_json::json!(pad));
}

/// Whether a link URL targets a local file (heuristic shared with legacy).
fn is_local_link(url: &str) -> bool {
    use crate::render::Link;
    Link::with_title_parsed("", url, "")
        .map(|l| l.is_file())
        .unwrap_or(false)
}

/// Sets the foreground / background colors on a [`Style`] from
/// [`StyleColor`]s, leaving the other appearance layers untouched.
fn set_style_colors(style: &mut Style, fg: Option<&StyleColor>, bg: Option<&StyleColor>) {
    if let Some(color) = fg {
        style.color = Some(TargetValue::universal(PerMode::universal(color.color)));
    }
    if let Some(color) = bg {
        style.background = Some(TargetValue::universal(PerMode::universal(color.color)));
    }
}

/// Builds a universal [`TargetValue<Length>`] for `n` character cells.
fn cells(n: u16) -> TargetValue<Length> {
    if n == 0 {
        TargetValue::universal(Length::Zero)
    } else {
        TargetValue::universal(Length::ch(u32::from(n)))
    }
}
