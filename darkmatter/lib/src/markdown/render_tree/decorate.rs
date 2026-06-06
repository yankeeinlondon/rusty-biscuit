//! Lowers per-component `style:` settings from a [`LayoutContext`] onto a
//! folded render tree.
//!
//! The decorated-layout terminal path
//! ([`render_tree_terminal_with_layout`](super::entrypoints::render_tree_terminal_with_layout))
//! folds a [`Markdown`](crate::markdown::Markdown) document, then runs this pass
//! to project each [`PageComponent`]'s alignment, fill width, and colors onto
//! the matching tree node's [`Layout`] / [`Style`]. The shared terminal tree
//! renderer then applies those attributes.

use renderable::layout::{Alignment, Length, TargetValue};
use renderable::style::{PerMode, Style};
use renderable::target::RenderTarget;
use renderable::tree::{HintNamespace, NodeKind, RenderNode};

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
        // Layout is block-only; inline nodes (e.g. Image) must not carry it.
        if !matches!(node.kind, NodeKind::Image { .. }) {
            apply_component_layout(node, ctx, component);
        }
        apply_component_color(node, ctx, component);
        if matches!(node.kind, NodeKind::ListItem { .. }) {
            apply_list_item_alignment(node, ctx, component);
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

/// Writes the component's [`ComponentPolicy`](crate::layout::ComponentPolicy)
/// `layout` onto the node. The renderer fold resolves widths, padding, and
/// alignment — no width math is done here.
fn apply_component_layout(node: &mut RenderNode, ctx: &LayoutContext, component: PageComponent) {
    let Some(policy) = ctx.component_policies.get(&component) else {
        return;
    };

    let default = renderable::layout::Layout::default();
    if policy.layout != default {
        node.attrs.set_layout(&policy.layout);
    }
}

/// Lifts the Images-component layout onto a lone-image paragraph.
///
/// The renderer fold handles alignment and width for the paragraph node;
/// no bespoke placeholder-width math is needed.
fn apply_lone_image_layout(node: &mut RenderNode, ctx: &LayoutContext, _alt: &str) {
    let component = PageComponent::Images;
    let Some(policy) = ctx.component_policies.get(&component) else {
        return;
    };

    // Only apply alignment and box offset (padding/margin) to the wrapping
    // paragraph.  max_width / width cap the *paragraph* content box, which
    // would force the alt-text fallback to wrap — image width constraints are
    // handled by the renderer's own image path, not by the paragraph layout.
    let mut layout = renderable::layout::Layout::default();
    layout.alignment = policy.layout.alignment;
    layout.padding = policy.layout.padding.clone();
    layout.margin = policy.layout.margin.clone();

    let default = renderable::layout::Layout::default();
    if layout != default {
        node.attrs.set_layout(&layout);
    }
}

/// Sets the node's [`Style`] foreground / background from the component colors.
fn apply_component_color(node: &mut RenderNode, ctx: &LayoutContext, component: PageComponent) {
    let fg = ctx.component_color(component);
    let bg = ctx.component_bg_color(component);
    if fg.is_none() && bg.is_none() {
        return;
    }
    let mut style = node.attrs.style().unwrap_or_default();
    set_style_colors(&mut style, fg, bg);
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
/// The pad is computed from the component's [`ComponentPolicy`] layout
/// (`max_width` / `width` / `alignment`), precomputed here so the renderer
/// stays free of darkmatter's page-layout types. `Left`-aligned lists (the
/// default) set no hint, so the renderer keeps its inline rendering byte-for-byte.
fn apply_list_item_alignment(node: &mut RenderNode, ctx: &LayoutContext, component: PageComponent) {
    let Some(policy) = ctx.component_policies.get(&component) else {
        return;
    };

    let label = match policy.layout.alignment {
        Alignment::Left => return,
        Alignment::Center => "center",
        Alignment::Right => "right",
    };

    let available = ctx.effective_width as u32;
    let width = match &policy.layout.width {
        renderable::layout::Width::Fixed(tv) => resolve_length(tv, available),
        _ => match &policy.layout.max_width {
            Some(mw) => resolve_length(mw, available),
            None => available,
        },
    };

    let surplus = available.saturating_sub(width);
    let pad = match policy.layout.alignment {
        Alignment::Left => 0,
        Alignment::Center => surplus / 2,
        Alignment::Right => surplus,
    };

    let ns = HintNamespace("darkmatter.li");
    node.attrs.set_hint(ns, "alignment", serde_json::json!(label));
    node.attrs.set_hint(ns, "pad", serde_json::json!(pad));
}

/// Resolves a [`TargetValue<Length>`] to whole cells against `width`.
fn resolve_length(tv: &TargetValue<Length>, width: u32) -> u32 {
    match tv.resolve(RenderTarget::Terminal) {
        Some(Length::Zero) | None => 0,
        Some(Length::Ch(n)) => *n,
        Some(Length::Percent(p)) => ((width as f32) * p / 100.0).round() as u32,
        Some(Length::Css(_)) => 0,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;

    const TABLE_MD: &str = "| A | B |\n|---|---|\n| 1 | 2 |\n";

    fn decorate_for_test(md_content: &str, style_yaml: &str) -> RenderNode {
        let full = format!("---\nstyle:\n{}---\n\n{}", indent(style_yaml, 4), md_content);
        let md = Markdown::try_from_content(&full).expect("parse markdown with style frontmatter");
        let (mut doc, _diags) = super::super::to_render_document(&md);

        // Build a LayoutContext with the component policies from the style.
        let (style, _warnings) = crate::style::from_frontmatter(md.frontmatter()).expect("parse style");
        let term = biscuit_terminal::terminal::Terminal::new_optimistic(80);
        let page = crate::layout::DarkmatterPage::new(&term);
        let page = crate::style::apply_page_style(page, &style, crate::style::PageStyleOverrides::default()).unwrap();
        let page = crate::style::apply_component_style(page, &style, crate::style::ComponentStyleOverrides::default()).unwrap();
        let page = crate::style::apply_list_style(page, &style, crate::style::ListStyleOverrides::default()).unwrap();
        let page = crate::style::apply_color_style(page, &style).unwrap();
        let page = crate::style::apply_hr_style(page, &style, crate::style::HrStyleOverrides::default()).unwrap();
        let page = crate::style::apply_bespoke_style(page, &style, crate::style::BespokeStyleOverrides::default(), None).unwrap();

        let ctx = crate::layout::LayoutContext::from_page(
            80,
            page.page_margin().clone(),
            page.page_padding().clone(),
            page.page_background(),
            page.page_max_width().cloned(),
            &biscuit_terminal::discovery::detection::ColorMode::Dark,
            crate::markdown::highlighting::ColorMode::Dark,
            page.page_color().cloned(),
            page.page_bg_color().cloned(),
            page.component_colors().clone(),
            page.component_bg_colors().clone(),
            page.component_policies().clone(),
            page.hyperlink_style().cloned(),
            page.local_hyperlink_style().cloned(),
            page.local_image_style().cloned(),
        )
        .expect("layout context");

        decorate_document(&mut doc.root, &ctx);
        doc.root
    }

    fn indent(text: &str, spaces: usize) -> String {
        let prefix = " ".repeat(spaces);
        text.lines()
            .map(|line| {
                if line.trim().is_empty() {
                    line.to_string()
                } else {
                    format!("{}{}", prefix, line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }

    fn find_node<'a>(node: &'a RenderNode, predicate: &dyn Fn(&RenderNode) -> bool) -> Option<&'a RenderNode> {
        if predicate(node) {
            return Some(node);
        }
        for child in node.children() {
            if let Some(found) = find_node(child, predicate) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn decorate_writes_component_layout_onto_nodes() {
        let doc = decorate_for_test(TABLE_MD, "table:\n  alignment: center\n");
        let table = find_node(&doc, &|n| matches!(n.kind, NodeKind::Table { .. })).unwrap();
        assert_eq!(table.attrs.layout_ref().unwrap().alignment, Alignment::Center);
    }
}
