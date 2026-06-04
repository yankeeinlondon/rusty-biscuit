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
// This pass is the ready-but-not-yet-active decorated terminal route: the
// `Some(ctx)` path in `Markdown::as_terminal_with_layout` still uses the legacy
// serializer until the three remaining decorated-layout features (hyperlink
// label width, image `▉ IMAGE[]` placeholder, right-aligned li body) land on
// the tree. The pass is exercised by `render_tree_terminal_with_layout`'s tests.
#![allow(dead_code)]

use renderable::layout::{Alignment, Length, Margin, TargetValue};
use renderable::style::{PerMode, Style};
use renderable::tree::{NodeKind, RenderNode};

use crate::layout::{LayoutContext, PageComponent};
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
    } else if is_lone_image_paragraph(node) {
        // A paragraph whose only content is an image is the block-level image
        // the legacy renderer aligns / fills / colors. The `Image` node itself
        // is inline (the renderer ignores its `Layout`), so the Images
        // component layout is lifted onto the wrapping paragraph instead.
        apply_component_layout(node, ctx, PageComponent::Images);
    }
    apply_inline_color(node, ctx);

    if let Some(children) = node.children_mut() {
        for child in children {
            decorate_node(child, ctx);
        }
    }
}

/// Whether `node` is a paragraph whose only meaningful child is an image.
fn is_lone_image_paragraph(node: &RenderNode) -> bool {
    let NodeKind::Paragraph { children } = &node.kind else {
        return false;
    };
    let mut images = 0;
    for child in children {
        match &child.kind {
            NodeKind::Image { .. } => images += 1,
            // Allow surrounding whitespace-only text / breaks.
            NodeKind::Text { value } if value.trim().is_empty() => {}
            NodeKind::SoftBreak | NodeKind::HardBreak => {}
            _ => return false,
        }
    }
    images == 1
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
    layout.margin = Margin {
        left: cells(left),
        ..layout.margin
    };
    // The offset is baked into the left margin above, so the node is
    // left-aligned; setting the typed alignment too would double the offset.
    layout.alignment = Alignment::Left;
    layout.max_width = Some(TargetValue::universal(Length::ch(u32::from(body_width))));
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
