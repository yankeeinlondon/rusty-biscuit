//! Construction-time policy view consumed by the context-aware Markdown fold.
//!
//! [`TreeBuildContext`] carries the resolved [`ComponentPolicy`] map, page
//! colors, hyperlink / image [`CommonStyle`] overlays, and HR defaults that the
//! fold needs to attach typed attrs as nodes are built.  It is Darkmatter-owned
//! construction input — not a render-time [`LayoutContext`](crate::layout::LayoutContext).
//! The fold reads it once at node close; no second whole-tree decoration walk
//! is needed.

use std::collections::HashMap;

use renderable::layout::{Alignment, Layout, TargetValue, Width};
use renderable::style::{PaintColor, PerMode, Style};
use renderable::tree::{HrAlignment, HrKind, HrWeight, NodeKind, RenderNode};

use crate::layout::{ComponentPolicy, PageComponent};
use crate::markdown::inline::HorizontalRuleAttrs;
use crate::style::bespoke::is_local_image;
use crate::style::schema::CommonStyle;

// ───────────────────────────── TreeBuildContext ─────────────────────────────

/// Construction-time policy the context-aware fold threads through the event
/// stream so that every node is complete (layout, colors, typed attrs) the
/// moment it is built.
///
/// Built from the same [`DarkmatterPage`](crate::layout::DarkmatterPage) +
/// frontmatter data that the render-time [`LayoutContext`](crate::layout::LayoutContext)
/// uses, but this type carries only the unresolved, target-neutral policy the
/// fold needs.  Terminal-width-dependent fields (margins, padding, content
/// width, background resolution) stay on `LayoutContext` for the page frame.
pub(crate) struct TreeBuildContext<'a> {
    /// Per-component layout + color policy from `style:` frontmatter.
    pub component_policies: &'a HashMap<PageComponent, ComponentPolicy>,
    /// Page-level foreground color attached to the root for inheritance.
    pub page_color: Option<PaintColor>,
    /// Page-level background color attached to the root for inheritance.
    pub page_bg_color: Option<PaintColor>,
    /// Global hyperlink style from `style.hyperlinks.*`.
    pub hyperlink_style: Option<&'a CommonStyle>,
    /// Local hyperlink override from `style.hyperlinks.local-style`.
    pub local_hyperlink_style: Option<&'a CommonStyle>,
    /// Local image override from `style.images.local-style`.
    pub local_image_style: Option<&'a CommonStyle>,
    /// HR defaults projected from `style.hr.*` or explicit render options.
    pub hr_defaults: Option<&'a HorizontalRuleAttrs>,
}

impl<'a> TreeBuildContext<'a> {
    /// Whether the context carries no effective policy — the fold skips policy
    /// work entirely when this returns `true`.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.component_policies.is_empty()
            && self.page_color.is_none()
            && self.page_bg_color.is_none()
            && self.hyperlink_style.is_none()
            && self.local_hyperlink_style.is_none()
            && self.local_image_style.is_none()
            && self.hr_defaults.is_none()
    }

    /// Resolve a component's own foreground color from its policy.
    ///
    /// No page fallback: page-level foreground lives only on the root node and
    /// reaches components through [`InheritedStyle`](renderable::tree::InheritedStyle),
    /// never copied onto each component (see [`apply_page_colors`]).
    pub fn component_color(&self, component: PageComponent) -> Option<PaintColor> {
        self.component_policies
            .get(&component)
            .and_then(|p| p.color)
    }

    /// Resolve a component's own background color from its policy.
    ///
    /// No page fallback: page background does not inherit and is painted by the
    /// page frame, not copied onto each component.
    pub fn component_bg_color(&self, component: PageComponent) -> Option<PaintColor> {
        self.component_policies
            .get(&component)
            .and_then(|p| p.bg_color)
    }

    /// Resolve the effective hyperlink [`CommonStyle`] for a link.
    ///
    /// Local links merge `local_hyperlink_style` over `hyperlink_style`; remote
    /// links use `hyperlink_style` only.
    pub fn effective_hyperlink_style(&self, is_local: bool) -> Option<CommonStyle> {
        if is_local {
            self.local_hyperlink_style
                .map(|local| {
                    if let Some(base) = self.hyperlink_style {
                        crate::style::bespoke::merge_common_style(base, local)
                    } else {
                        local.clone()
                    }
                })
                .or_else(|| self.hyperlink_style.cloned())
        } else {
            self.hyperlink_style.cloned()
        }
    }

    /// Resolve effective hyperlink foreground/background colors.
    ///
    /// No page fallback: the page foreground inherits to links through the
    /// styled root ([`InheritedStyle`](renderable::tree::InheritedStyle)), and
    /// the page background is painted once by the page frame. Copying either
    /// onto each link node would defeat inheritance and double-composite an
    /// alpha-bearing page background.
    pub fn hyperlink_color(
        &self,
        is_local: bool,
    ) -> (Option<PaintColor>, Option<PaintColor>) {
        let merged = self.effective_hyperlink_style(is_local);

        let fg = merged
            .as_ref()
            .and_then(|s| s.color.as_ref())
            .map(|c| c.to_paint_color())
            .or_else(|| self.component_color(PageComponent::Hyperlinks));
        let bg = merged
            .as_ref()
            .and_then(|s| s.bg_color.as_ref())
            .map(|c| c.to_paint_color())
            .or_else(|| self.component_bg_color(PageComponent::Hyperlinks));

        (fg, bg)
    }

    /// Resolve effective image foreground/background colors for fallback text.
    ///
    /// No page fallback, for the same reason as [`hyperlink_color`]: the page
    /// foreground inherits to the alt-text placeholder through the styled root,
    /// and the page background is painted by the page frame, not copied onto
    /// each image node.
    pub fn image_color(
        &self,
        is_local: bool,
    ) -> (Option<PaintColor>, Option<PaintColor>) {
        let local = if is_local {
            self.local_image_style
        } else {
            None
        };

        let fg = local
            .and_then(|s| s.color.as_ref())
            .map(|c| c.to_paint_color())
            .or_else(|| self.component_color(PageComponent::Images));
        let bg = local
            .and_then(|s| s.bg_color.as_ref())
            .map(|c| c.to_paint_color())
            .or_else(|| self.component_bg_color(PageComponent::Images));

        (fg, bg)
    }
}

// ─────────────────────────── policy application ────────────────────────────

/// Applies the page-level **foreground** color to the root node so it inherits
/// to all descendant text through
/// [`InheritedStyle`](renderable::tree::InheritedStyle).
///
/// Only the foreground is attached: the generic inheritance contract carries
/// `color` + emphasis but deliberately does not inherit `background`
/// ([`InheritedStyle`](renderable::tree::InheritedStyle)). The page background
/// is therefore painted by the page frame — the browser page wrapper and the
/// terminal row decoration — not by the root node.
pub(crate) fn apply_page_colors(root: &mut RenderNode, ctx: &TreeBuildContext) {
    let Some(fg) = ctx.page_color else {
        return;
    };
    let mut style = root.attrs.style().unwrap_or_default();
    set_style_colors(&mut style, Some(&fg), None);
    root.attrs.set_style(&style);
}

/// Applies all construction-time policy to a single node after the fold builds
/// it.
///
/// This replaces the post-fold `decorate_document` walk: component layout,
/// colors, typed text-layout hints, structured link/image directives, and HR
/// defaults are all attached here.  The node's semantic content (children, alt
/// text) is never mutated.
pub(crate) fn apply_node_policy(node: &mut RenderNode, ctx: &TreeBuildContext) {
    if let Some(component) = component_for(&node.kind) {
        // Layout is block-only; inline nodes (Image) must not carry it.
        if !matches!(node.kind, NodeKind::Image { .. }) {
            apply_component_layout(node, ctx, component);
            apply_component_style_attrs(node, ctx, component);
        }
        apply_component_color(node, ctx, component);
    } else if let Some(()) = lone_image_alt(&node.kind) {
        apply_lone_image_layout(node, ctx);
    }

    // Disclosure blocks merge inline opener style over component policy.
    if matches!(node.kind, NodeKind::Disclosure { .. }) {
        apply_disclosure_policy(node, ctx);
    }

    // List-item typed text_layout (replaces the old `darkmatter.li` hint).
    if matches!(node.kind, NodeKind::ListItem { .. }) {
        apply_list_item_text_layout(node, ctx);
    }

    // Hyperlink / image colors + text-layout + structured directives.
    apply_link_policy(node, ctx);
    apply_image_policy(node, ctx);

    // HR defaults.
    if matches!(node.kind, NodeKind::ThematicBreak) {
        apply_hr_defaults(node, ctx);
    }
}

/// Maps a [`NodeKind`] to the [`PageComponent`] whose layout settings govern it.
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
        NodeKind::Disclosure { .. } => Some(PageComponent::Disclosure),
        _ => None,
    }
}

/// Writes the component's [`ComponentPolicy`] `layout` onto the node.
fn apply_component_layout(
    node: &mut RenderNode,
    ctx: &TreeBuildContext,
    component: PageComponent,
) {
    let Some(policy) = ctx.component_policies.get(&component) else {
        return;
    };
    let default = Layout::default();
    if policy.layout != default {
        node.attrs.set_layout(&policy.layout);
    }
}

/// Writes the component's [`ComponentPolicy`] emphasis, border, and word-wrap
/// onto the node.
fn apply_component_style_attrs(
    node: &mut RenderNode,
    ctx: &TreeBuildContext,
    component: PageComponent,
) {
    let Some(policy) = ctx.component_policies.get(&component) else {
        return;
    };

    if policy.emphasis.is_some() || policy.border.is_some() {
        let style = node.attrs.style_mut_or_default();
        if let Some(emphasis) = policy.emphasis {
            style.emphasis = emphasis;
        }
        if let Some(border) = policy.border.clone() {
            style.border = Some(border);
        }
        node.attrs.retain_non_default_style();
    }

    if let Some(word_wrap) = policy.word_wrap.clone() {
        if let Some(mut layout) = node.attrs.layout() {
            layout.word_wrap = word_wrap;
            node.attrs.set_layout(&layout);
        } else {
            let layout = Layout {
                word_wrap,
                ..Default::default()
            };
            node.attrs.set_layout(&layout);
        }
    }
}

/// Sets foreground / background from the component's resolved colors.
fn apply_component_color(
    node: &mut RenderNode,
    ctx: &TreeBuildContext,
    component: PageComponent,
) {
    let fg = ctx.component_color(component);
    let bg = ctx.component_bg_color(component);
    if fg.is_none() && bg.is_none() {
        return;
    }
    let mut style = node.attrs.style().unwrap_or_default();
    set_style_colors(&mut style, fg.as_ref(), bg.as_ref());
    node.attrs.set_style(&style);
}

/// Merges inline disclosure style hints over the `style.disclosure` component
/// policy. Inline opener parameters win; frontmatter fills defaults.
fn apply_disclosure_policy(node: &mut RenderNode, ctx: &TreeBuildContext) {
    let inline = match &node.kind {
        NodeKind::Disclosure { style, .. } => style.as_deref().cloned(),
        _ => return,
    };

    let policy = ctx.component_policies.get(&PageComponent::Disclosure);
    let mut layout = policy.map(|p| p.layout.clone()).unwrap_or_default();

    if let Some(hints) = inline.as_ref()
        && let Some(il) = hints.layout.as_ref()
    {
        // `width` and `max-width` are mutually exclusive (see the disclosure
        // styling spec). A higher-priority inline choice clears the lower-priority
        // frontmatter value of the *other* property; keeping both would let a
        // stale frontmatter cap clamp an instance `width`, or a stale fixed width
        // survive an instance `max-width`.
        if il.width != Width::default() {
            layout.width = il.width.clone();
            layout.max_width = None;
        }
        if il.max_width.is_some() {
            layout.max_width = il.max_width.clone();
            layout.width = Width::default();
        }
        if il.alignment != Alignment::default() {
            layout.alignment = il.alignment;
        }
    }

    if layout != Layout::default() {
        node.attrs.set_layout(&layout);
    }

    let fg = inline
        .as_ref()
        .and_then(|h| h.color)
        .or_else(|| ctx.component_color(PageComponent::Disclosure));
    let bg = inline
        .as_ref()
        .and_then(|h| h.bg_color)
        .or_else(|| ctx.component_bg_color(PageComponent::Disclosure));

    if fg.is_some() || bg.is_some() {
        let mut style = node.attrs.style().unwrap_or_default();
        set_style_colors(&mut style, fg.as_ref(), bg.as_ref());
        node.attrs.set_style(&style);
    }

    // Inline border / emphasis / word-wrap (Phase 5).
    if let Some(inline) = inline.as_ref() {
        if inline.border.is_some() || inline.emphasis.is_some() {
            let mut style = node.attrs.style().unwrap_or_default();
            if let Some(border) = inline.border.clone() {
                style.border = Some(border);
            }
            if let Some(emphasis) = inline.emphasis {
                style.emphasis = emphasis;
            }
            node.attrs.set_style(&style);
        }
    if let Some(word_wrap) = inline.word_wrap.clone() {
        let mut layout = node.attrs.layout().unwrap_or_default();
        layout.word_wrap = word_wrap;
        node.attrs.set_layout(&layout);
    }
    }
}

/// Attaches typed link policy: colors, text-layout hints, and structured
/// browser directives.
fn apply_link_policy(node: &mut RenderNode, ctx: &TreeBuildContext) {
    // The URL and title are read, never stored, so they are borrowed out of the
    // node instead of cloned for every link in the document — including on the
    // empty-policy path, where the clones were the only work done.
    //
    // Every decision is resolved into an owned value inside this scope so the
    // borrow of `node.kind` ends before the node is mutated below. Application
    // order (colors, then layout hints, then directive) is unchanged;
    // `parse_link_directive` is pure, so computing it earlier is equivalent.
    let (fg, bg, common, directive) = {
        let NodeKind::Link { url, title, .. } = &node.kind else {
            return;
        };

        let is_local = is_local_link(url);
        let (fg, bg) = ctx.hyperlink_color(is_local);
        let common = ctx.effective_hyperlink_style(is_local);

        let directive = title.as_ref().and_then(|raw_title| {
            let frontmatter_css = common
                .as_ref()
                .and_then(|c| c.to_css_overlay())
                .map(|c| c.to_css().replace('\n', " "));
            parse_link_directive(url, raw_title, frontmatter_css.as_deref())
        });

        (fg, bg, common, directive)
    };

    // Colors.
    if fg.is_some() || bg.is_some() {
        let mut style = node.attrs.style().unwrap_or_default();
        set_style_colors(&mut style, fg.as_ref(), bg.as_ref());
        node.attrs.set_style(&style);
    }

    // Text-layout hints (typed, not content mutation).
    if let Some(ref common) = common {
        attach_text_layout(node, common);
    }

    // Structured directive parsing.
    if let Some(directive) = directive {
        directive.apply_to_link_node(node);
    }
}

/// Attaches typed image policy: colors, text-layout hints, and structured
/// directives.
fn apply_image_policy(node: &mut RenderNode, ctx: &TreeBuildContext) {
    // Borrowed for the same reason as `apply_link_policy`: the URL and title are
    // only read, so cloning both for every image — empty policy included — was
    // pure overhead.
    let (fg, bg, is_local, directive) = {
        let NodeKind::Image { url, title, .. } = &node.kind else {
            return;
        };

        let is_local = is_local_image(url);
        let (fg, bg) = ctx.image_color(is_local);

        let directive = title.as_ref().and_then(|raw_title| {
            let frontmatter_css = ctx
                .local_image_style
                .and_then(|c| c.to_css_overlay())
                .map(|c| c.to_css().replace('\n', " "));
            parse_image_directive(url, raw_title, frontmatter_css.as_deref())
        });

        (fg, bg, is_local, directive)
    };

    // Colors.
    if fg.is_some() || bg.is_some() {
        let mut style = node.attrs.style().unwrap_or_default();
        set_style_colors(&mut style, fg.as_ref(), bg.as_ref());
        node.attrs.set_style(&style);
    }

    // Text-layout hints (typed, not content mutation).
    if is_local
        && let Some(common) = ctx.local_image_style
    {
        attach_text_layout(node, common);
    }

    // Structured directive parsing for images (inline CSS / class).
    if let Some(directive) = directive {
        directive.apply_to_image_node(node);
    }
}

/// Attaches [`TextLayoutHints`] to a list-item node from the Li component
/// policy, so the terminal renderer lifts the marker and pads the body per the
/// resolved alignment. Replaces the deleted `darkmatter.li` hint.
fn apply_list_item_text_layout(node: &mut RenderNode, ctx: &TreeBuildContext) {
    use renderable::layout::{Width};
    use renderable::tree::TextLayoutHints;

    let Some(policy) = ctx.component_policies.get(&PageComponent::Li) else {
        return;
    };

    let width = match &policy.layout.width {
        Width::Fixed(tv) => Some(tv.clone()),
        Width::Auto | Width::FitContent => None,
    };
    let max_width = policy.layout.max_width.clone();

    if width.is_none() && max_width.is_none() {
        return;
    }

    let hints = TextLayoutHints {
        // An exact `width` is an exact field: content wider than it is truncated
        // with an ellipsis (the retired `apply_inline_text_layout` contract).
        // Without an exact width, a lone `max_width` is already a hard ceiling
        // the renderer truncates unconditionally, so the policy is moot.
        overflow: exact_width_overflow(width.is_some()),
        width,
        max_width,
        alignment: policy.layout.alignment,
    };
    node.attrs.set_text_layout(&hints);
}

/// Returns `Some(())` when `node` is a paragraph whose only meaningful child
/// is one image — the block-level image the legacy renderer aligns / fills /
/// colors. The `Image` node itself is inline (the renderer ignores its
/// `Layout`), so the Images component layout must be lifted onto the wrapping
/// paragraph instead.
fn lone_image_alt(kind: &NodeKind) -> Option<()> {
    let NodeKind::Paragraph { children } = kind else {
        return None;
    };
    let mut images = 0;
    for child in children {
        match &child.kind {
            NodeKind::Image { .. } => images += 1,
            NodeKind::Text { value } if value.trim().is_empty() => {}
            NodeKind::SoftBreak | NodeKind::HardBreak => {}
            _ => return None,
        }
    }
    (images == 1).then_some(())
}

/// Lifts the Images-component layout onto a lone-image paragraph.
///
/// Only alignment and box offset (padding/margin) are applied; image width
/// constraints are handled by the renderer's own image path, not by the
/// paragraph layout.
fn apply_lone_image_layout(node: &mut RenderNode, ctx: &TreeBuildContext) {
    let component = PageComponent::Images;
    let Some(policy) = ctx.component_policies.get(&component) else {
        return;
    };

    let mut layout = Layout::default();
    layout.alignment = policy.layout.alignment;
    layout.padding = policy.layout.padding.clone();
    layout.margin = policy.layout.margin.clone();

    if layout != Layout::default() {
        node.attrs.set_layout(&layout);
    }
}

/// Sets foreground / background colors on a [`Style`] from [`PaintColor`]s.
fn set_style_colors(style: &mut Style, fg: Option<&PaintColor>, bg: Option<&PaintColor>) {
    if let Some(color) = fg {
        style.color = Some(TargetValue::universal(PerMode::universal(*color)));
    }
    if let Some(color) = bg {
        style.background = Some(TargetValue::universal(PerMode::universal(*color)));
    }
}

/// Attaches [`TextLayoutHints`] from a [`CommonStyle`]'s `width` / `max_width`
/// / `alignment` without mutating the node's children or alt text.
fn attach_text_layout(node: &mut RenderNode, common: &CommonStyle) {
    use renderable::tree::TextLayoutHints;

    let width = common.width.as_ref().and_then(|w| match w.as_width() {
        Width::Fixed(tv) => match tv {
            TargetValue::Universal(len) => Some(TargetValue::universal(len.clone())),
            TargetValue::PerTarget(_) => None,
        },
        _ => None,
    });
    let max_width = common.max_width.clone().map(TargetValue::universal);

    if width.is_none() && max_width.is_none() {
        return;
    }

    let hints = TextLayoutHints {
        // See `apply_list_item_text_layout`: an exact `width` truncates an
        // overflowing label / placeholder; a lone `max_width` is handled as a
        // hard ceiling by the renderer regardless of this policy.
        overflow: exact_width_overflow(width.is_some()),
        width,
        max_width,
        alignment: common.alignment.unwrap_or_default(),
    };
    node.attrs.set_text_layout(&hints);
}

/// Picks the [`TextOverflow`](renderable::tree::TextOverflow) policy for a typed
/// text-layout hint: an exact `width` field truncates overflow, matching the
/// retired bespoke `apply_inline_text_layout` behavior; otherwise content is
/// preserved (a lone `max_width` is a hard ceiling the renderer always applies).
fn exact_width_overflow(has_exact_width: bool) -> renderable::tree::TextOverflow {
    if has_exact_width {
        renderable::tree::TextOverflow::Truncate
    } else {
        renderable::tree::TextOverflow::Preserve
    }
}

/// Fills missing [`ThematicBreakAttrs`] fields on a [`NodeKind::ThematicBreak`]
/// from `defaults`, reproducing the legacy bare-rule default contract: an
/// inline `{:hr …}` directive wins per-field, and page-level defaults fill only
/// the fields the author left unset.
///
/// [`ThematicBreakAttrs`]: renderable::tree::ThematicBreakAttrs
fn apply_hr_defaults(node: &mut RenderNode, ctx: &TreeBuildContext) {
    let Some(defaults) = ctx.hr_defaults else {
        return;
    };
    let tb = node.attrs.thematic_break_mut_or_default();
    if tb.kind.is_none() {
        // The canonical `kind` wins over the deprecated `style:` alias. Author
        // text is parsed to the shared enum here; an unrecognized spelling
        // stays `None`.
        tb.kind = defaults
            .kind
            .as_deref()
            .or(defaults.legacy_style.as_deref())
            .and_then(HrKind::from_authored);
    }
    if tb.alignment.is_none() {
        tb.alignment = defaults
            .alignment
            .as_deref()
            .and_then(HrAlignment::from_authored);
    }
    if tb.weight.is_none() {
        tb.weight = defaults.weight.as_deref().and_then(HrWeight::from_authored);
    }
    if tb.width.is_none() {
        tb.width = defaults.width.clone();
    }
    if tb.color.is_none() {
        tb.color = defaults.color.clone();
    }
    node.attrs.retain_non_default_thematic_break();
}

// ───────────────────── structured link/image directives ────────────────────

/// Parsed structured link directive (`class='...' target='...' style='...'`).
struct LinkDirective {
    clear_title: bool,
    class: Option<String>,
    target: Option<String>,
    title_plain: Option<String>,
    prompt: Option<String>,
    data: Vec<(String, String)>,
    inline_style: Option<String>,
}

impl LinkDirective {
    fn apply_to_link_node(&self, node: &mut RenderNode) {
        use renderable::tree::{DataAttrName, LinkBrowserAttrs, LinkTarget};

        if let Some(class) = &self.class {
            node.attrs.classes.push(class.clone());
        }
        if let Some(style) = &self.inline_style
            && let Ok(css) = renderable::stylesheet::CssStyle::try_from(style.as_str())
        {
            let browser = node.attrs.browser_mut_or_default();
            browser.inline_style = Some(css);
            node.attrs.retain_non_default_browser();
        }
        if self.target.is_some() || self.prompt.is_some() || !self.data.is_empty() {
            let browser = node.attrs.browser_mut_or_default();
            if let Some(target_str) = &self.target {
                let parsed = LinkTarget::parse(target_str);
                if let Some(link_attrs) = browser.link.as_mut() {
                    link_attrs.target = Some(parsed);
                } else {
                    browser.link = Some(LinkBrowserAttrs {
                        target: Some(parsed),
                        ..Default::default()
                    });
                }
            }
            if let Some(prompt) = &self.prompt
                && let Ok(name) = DataAttrName::new("prompt")
            {
                browser.data_attrs.insert(name, prompt.clone());
            }
            for (key, value) in &self.data {
                if let Ok(name) = DataAttrName::new(key.clone()) {
                    browser.data_attrs.insert(name, value.clone());
                }
            }
            node.attrs.retain_non_default_browser();
        }

        // For structured directives, replace the raw title with the plain title
        // (or None) so the browser writer does not leak the raw directive.
        if self.clear_title
            && let NodeKind::Link { title, .. } = &mut node.kind
        {
            *title = self.title_plain.clone();
        }
    }
}

/// Parsed structured image directive (`class='...' style='...' data-*='...'`).
struct ImageDirective {
    clear_title: bool,
    title_plain: Option<String>,
    class: Option<String>,
    data: Vec<(String, String)>,
    inline_style: Option<String>,
}

impl ImageDirective {
    fn apply_to_image_node(&self, node: &mut RenderNode) {
        use renderable::tree::DataAttrName;

        if let Some(class) = &self.class {
            node.attrs.classes.push(class.clone());
        }
        if let Some(style) = &self.inline_style
            && let Ok(css) = renderable::stylesheet::CssStyle::try_from(style.as_str())
        {
            let browser = node.attrs.browser_mut_or_default();
            browser.inline_style = Some(css);
            node.attrs.retain_non_default_browser();
        }
        if !self.data.is_empty() {
            let browser = node.attrs.browser_mut_or_default();
            for (key, value) in &self.data {
                if let Ok(name) = DataAttrName::new(key.clone()) {
                    browser.data_attrs.insert(name, value.clone());
                }
            }
            node.attrs.retain_non_default_browser();
        }

        // For structured directives, replace the raw title with the plain title
        // (or None) so the browser writer does not leak the raw directive.
        if self.clear_title
            && let NodeKind::Image { title, .. } = &mut node.kind
        {
            *title = self.title_plain.clone();
        }
    }
}

/// Parses a link title as a structured directive using darkmatter's [`Link`]
/// parser.  `frontmatter_css` is the merged hyperlink `CommonStyle` CSS overlay
/// (already lowered to a declaration string) so per-node declarations can win
/// property-by-property.
fn parse_link_directive(
    url: &str,
    title: &str,
    frontmatter_css: Option<&str>,
) -> Option<LinkDirective> {
    use crate::render::Link;

    let link = Link::with_title_parsed("", url, title).ok()?;
    let is_structured = link.title_plain().as_deref() != Some(title);

    let own_css = link.style().map(|s| s.to_css().replace('\n', " "));

    let merged_style = match (&own_css, frontmatter_css) {
        (None, None) => None,
        (Some(own), None) => Some(own.clone()),
        (None, Some(fm)) => Some(fm.to_string()),
        (Some(own), Some(fm)) => {
            let combined = format!("{fm}\n{own}");
            renderable::stylesheet::CssStyle::try_from(combined.as_str())
                .ok()
                .map(|c| c.to_css().replace('\n', " "))
                .or_else(|| Some(own.clone()))
        }
    };

    // If not structured and no inline style, nothing to do.
    if !is_structured && merged_style.is_none() {
        return None;
    }

    Some(LinkDirective {
        clear_title: is_structured,
        class: if is_structured {
            link.class().map(str::to_string)
        } else {
            None
        },
        target: if is_structured {
            link.target_attr()
        } else {
            None
        },
        title_plain: if is_structured {
            link.title_plain()
        } else {
            None
        },
        prompt: if is_structured {
            link.prompt().map(str::to_string)
        } else {
            None
        },
        data: if is_structured {
            link.data().iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else {
            Vec::new()
        },
        inline_style: merged_style,
    })
}

/// Parses an image title as a structured directive using darkmatter's
/// [`ImageRef`](crate::render::ImageRef) parser.
fn parse_image_directive(
    url: &str,
    title: &str,
    frontmatter_css: Option<&str>,
) -> Option<ImageDirective> {
    use crate::render::ImageRef;

    // Parse the title as a structured directive (the same parser the production
    // Markdown helper uses), so a per-image `style='...'` becomes typed inline
    // CSS instead of leaking the raw directive into the HTML `title`.
    let image_ref = ImageRef::with_title_parsed(url, "", title).ok()?;

    // The title was a structured directive (or metadata) when the parser did not
    // round-trip it back as the literal title — mirrors the link directive path.
    let is_structured = image_ref.title() != Some(title.trim());

    let own_css = image_ref.style().map(|s| s.to_css().replace('\n', " "));

    let merged_style = match (&own_css, frontmatter_css) {
        (None, None) => None,
        (Some(own), None) => Some(own.clone()),
        (None, Some(fm)) => Some(fm.to_string()),
        (Some(own), Some(fm)) => {
            let combined = format!("{fm}\n{own}");
            renderable::stylesheet::CssStyle::try_from(combined.as_str())
                .ok()
                .map(|c| c.to_css().replace('\n', " "))
                .or_else(|| Some(own.clone()))
        }
    };

    // Nothing to do for a plain title with no frontmatter overlay.
    if !is_structured && merged_style.is_none() {
        return None;
    }

    Some(ImageDirective {
        clear_title: is_structured,
        title_plain: if is_structured {
            image_ref.title().map(str::to_string)
        } else {
            None
        },
        class: if is_structured {
            image_ref.class().map(str::to_string)
        } else {
            None
        },
        data: if is_structured {
            image_ref
                .data()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            Vec::new()
        },
        inline_style: merged_style,
    })
}

/// Whether a link URL targets a local file (heuristic shared with legacy).
fn is_local_link(url: &str) -> bool {
    use crate::render::Link;
    Link::with_title_parsed("", url, "")
        .map(|l| l.is_file())
        .unwrap_or(false)
}

// ── structural tests (Phase 6) ─────────────────────────────────────────────

#[cfg(test)]
mod structural_tests {
    use super::*;
    use crate::markdown::Markdown;

    /// Folds `md_content` through the context-aware fold.
    fn fold_test(md_content: &str, ctx: &TreeBuildContext) -> renderable::tree::Document {
        let md = Markdown::try_from_content(md_content).expect("parse markdown");
        let source = renderable::tree::SourceDescriptor::Virtual {
            name: "test".into(),
        };
        let (doc, diags) = super::super::fold::fold_markdown_spanned_with_context(source, &md, ctx)
            .expect("context-aware fold must succeed");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        doc
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

    fn empty_policies() -> HashMap<PageComponent, ComponentPolicy> {
        HashMap::new()
    }

    fn ctx_for<'a>(policies: &'a HashMap<PageComponent, ComponentPolicy>) -> TreeBuildContext<'a> {
        TreeBuildContext {
            component_policies: policies,
            page_color: None,
            page_bg_color: None,
            hyperlink_style: None,
            local_hyperlink_style: None,
            local_image_style: None,
            hr_defaults: None,
        }
    }

    // ── component layout ───────────────────────────────────────────────────

    #[test]
    fn context_fold_bakes_table_alignment() {
        use renderable::layout::Layout;

        let mut policies = empty_policies();
        policies.insert(
            PageComponent::Tables,
            ComponentPolicy {
                layout: Layout {
                    alignment: renderable::layout::Alignment::Center,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let ctx = ctx_for(&policies);
        let doc = fold_test("| A | B |\n|---|---|\n| 1 | 2 |\n", &ctx);
        let table = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Table { .. }))
            .expect("table node");
        assert_eq!(
            table.attrs.layout_ref().unwrap().alignment,
            renderable::layout::Alignment::Center
        );
    }

    // ── disclosure layout precedence ───────────────────────────────────────

    /// Inline `width` and frontmatter `max-width` are a mutually exclusive
    /// layout choice across precedence layers, not just within one bucket. An
    /// instance `width=60ch` must clear the lower-priority frontmatter
    /// `max-width: 24ch`, otherwise the stale cap clamps the instance width on
    /// both terminal and browser.
    #[test]
    fn context_fold_inline_width_clears_frontmatter_max_width() {
        use renderable::layout::{Layout, Length, TargetValue, Width};

        let mut policies = empty_policies();
        policies.insert(
            PageComponent::Disclosure,
            ComponentPolicy {
                layout: Layout {
                    max_width: Some(TargetValue::universal(Length::Ch(24))),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let ctx = ctx_for(&policies);
        let doc = fold_test(
            "::disclosure width=60ch Summary\n::details\nBody.\n::end-disclosure\n",
            &ctx,
        );
        let disclosure = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Disclosure { .. }))
            .expect("disclosure node");
        let layout = disclosure.attrs.layout_ref().expect("layout is set");
        assert_eq!(
            layout.width,
            Width::Fixed(TargetValue::universal(Length::Ch(60))),
            "inline width must win"
        );
        assert!(
            layout.max_width.is_none(),
            "frontmatter max-width must be cleared by inline width: {:?}",
            layout.max_width
        );
    }

    /// The symmetric case: an instance `max-width` must reset a lower-priority
    /// frontmatter fixed `width` back to `Auto` before applying the cap.
    #[test]
    fn context_fold_inline_max_width_clears_frontmatter_width() {
        use renderable::layout::{Layout, Length, TargetValue, Width};

        let mut policies = empty_policies();
        policies.insert(
            PageComponent::Disclosure,
            ComponentPolicy {
                layout: Layout {
                    width: Width::Fixed(TargetValue::universal(Length::Ch(60))),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let ctx = ctx_for(&policies);
        let doc = fold_test(
            "::disclosure max-width=24ch Summary\n::details\nBody.\n::end-disclosure\n",
            &ctx,
        );
        let disclosure = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Disclosure { .. }))
            .expect("disclosure node");
        let layout = disclosure.attrs.layout_ref().expect("layout is set");
        assert_eq!(layout.width, Width::Auto, "inline max-width must reset width to Auto");
        assert_eq!(
            layout.max_width,
            Some(TargetValue::universal(Length::Ch(24))),
            "inline max-width must win"
        );
    }

    // ── component colors ───────────────────────────────────────────────────

    #[test]
    fn context_fold_bakes_component_colors() {
        let mut policies = empty_policies();
        policies.insert(
            PageComponent::Tables,
            ComponentPolicy {
                color: Some(PaintColor::new(renderable::color::Color::Tailwind(
                    renderable::color::Tailwind::Blue500,
                ))),
                bg_color: Some(PaintColor::new(renderable::color::Color::Tailwind(
                    renderable::color::Tailwind::Red500,
                ))),
                ..Default::default()
            },
        );
        let ctx = ctx_for(&policies);
        let doc = fold_test("| A | B |\n|---|---|\n| 1 | 2 |\n", &ctx);
        let table = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Table { .. }))
            .expect("table node");
        let style = table.attrs.style().expect("style is set");
        assert!(style.color.is_some(), "fg color set");
        assert!(style.background.is_some(), "bg color set");
    }

    #[test]
    fn context_fold_preserves_alpha_in_style() {
        let semi = crate::style::StyleColor {
            color: renderable::color::Color::Tailwind(renderable::color::Tailwind::Red500),
            opacity: Some(50),
        }
        .to_paint_color();

        let mut policies = empty_policies();
        policies.insert(
            PageComponent::BlockQuotes,
            ComponentPolicy {
                bg_color: Some(semi),
                ..Default::default()
            },
        );
        let ctx = ctx_for(&policies);
        let doc = fold_test("> quote\n", &ctx);
        let quote = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::BlockQuote { .. }))
            .expect("blockquote node");
        let style = quote.attrs.style().expect("style is set");
        let bg = style
            .background
            .as_ref()
            .and_then(|tv| tv.resolve(renderable::target::RenderTarget::Terminal))
            .expect("background is set");
        let bg_paint = bg.resolve(renderable::color::ColorMode::Dark);
        assert_ne!(
            bg_paint.opacity,
            renderable::style::Opacity::OPAQUE,
            "opacity must survive construction"
        );
    }

    // ── page colors on root ────────────────────────────────────────────────

    #[test]
    fn context_fold_attaches_page_foreground_to_root_only() {
        // Only the page foreground rides the root (it inherits via
        // `InheritedStyle`); the page background does not inherit and is painted
        // by the page frame, so it must NOT be copied onto the root.
        let policies = empty_policies();
        let mut ctx = ctx_for(&policies);
        ctx.page_color = Some(PaintColor::new(renderable::color::Color::Tailwind(
            renderable::color::Tailwind::Red500,
        )));
        ctx.page_bg_color = Some(PaintColor::new(renderable::color::Color::Tailwind(
            renderable::color::Tailwind::Blue500,
        )));
        let doc = fold_test("hello\n", &ctx);
        let style = doc.root.attrs.style().expect("root has style");
        assert!(style.color.is_some(), "page foreground on root");
        assert!(
            style.background.is_none(),
            "page background must not be copied onto the root"
        );
    }

    #[test]
    fn context_fold_does_not_copy_page_color_onto_components() {
        // Review-1 finding 3: with only a page color set (no table-specific
        // color), a component node must NOT carry a copied page color — the page
        // foreground reaches it through root inheritance, not a per-node copy.
        let policies = empty_policies();
        let mut ctx = ctx_for(&policies);
        ctx.page_color = Some(PaintColor::new(renderable::color::Color::Tailwind(
            renderable::color::Tailwind::Red500,
        )));
        ctx.page_bg_color = Some(PaintColor::new(renderable::color::Color::Tailwind(
            renderable::color::Tailwind::Blue500,
        )));
        let doc = fold_test("| A | B |\n|---|---|\n| 1 | 2 |\n", &ctx);
        let table = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Table { .. }))
            .expect("table node");
        assert!(
            table.attrs.style_ref().is_none_or(|s| s.is_empty()),
            "page color must not be copied onto the table component; got {:?}",
            table.attrs.style_ref()
        );
    }

    #[test]
    fn context_fold_does_not_copy_page_color_onto_links() {
        // Review-2 finding: with only page colors set (no hyperlink-specific
        // color), a link node must NOT carry a copied page style — the page
        // foreground reaches it through root inheritance and the page background
        // is painted once by the page frame.
        let policies = empty_policies();
        let mut ctx = ctx_for(&policies);
        ctx.page_color = Some(PaintColor::new(renderable::color::Color::Tailwind(
            renderable::color::Tailwind::Red500,
        )));
        ctx.page_bg_color = Some(PaintColor::new(renderable::color::Color::Tailwind(
            renderable::color::Tailwind::Blue500,
        )));
        let doc = fold_test("[label](https://example.com)\n", &ctx);
        let link = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Link { .. }))
            .expect("link node");
        assert!(
            link.attrs.style_ref().is_none_or(|s| s.is_empty()),
            "page color must not be copied onto the link; got {:?}",
            link.attrs.style_ref()
        );
    }

    #[test]
    fn context_fold_does_not_copy_page_color_onto_images() {
        // Same contract for images: a page-only color must not become an
        // explicit image-node style.
        let policies = empty_policies();
        let mut ctx = ctx_for(&policies);
        ctx.page_color = Some(PaintColor::new(renderable::color::Color::Tailwind(
            renderable::color::Tailwind::Red500,
        )));
        ctx.page_bg_color = Some(PaintColor::new(renderable::color::Color::Tailwind(
            renderable::color::Tailwind::Blue500,
        )));
        let doc = fold_test("![alt](pic.png)\n", &ctx);
        let image = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Image { .. }))
            .expect("image node");
        assert!(
            image.attrs.style_ref().is_none_or(|s| s.is_empty()),
            "page color must not be copied onto the image; got {:?}",
            image.attrs.style_ref()
        );
    }

    // ── HR defaults ────────────────────────────────────────────────────────

    #[test]
    fn context_fold_applies_hr_defaults() {
        let hr_defaults = HorizontalRuleAttrs {
            kind: Some("waves".to_string()),
            ..Default::default()
        };
        let policies = empty_policies();
        let mut ctx = ctx_for(&policies);
        ctx.hr_defaults = Some(&hr_defaults);
        let doc = fold_test("---\n", &ctx);
        let hr = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::ThematicBreak))
            .expect("thematic break");
        assert_eq!(
            hr.attrs.thematic_break_ref().and_then(|h| h.kind),
            Some(renderable::tree::HrKind::Waves)
        );
    }

    // ── structured link directives ─────────────────────────────────────────

    #[test]
    fn context_fold_parses_structured_link_class() {
        let policies = empty_policies();
        let ctx = ctx_for(&policies);
        let doc = fold_test("[label](https://example.com \"class='btn'\")\n", &ctx);
        let link = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Link { .. }))
            .expect("link node");
        assert!(
            link.attrs.classes.iter().any(|c| c == "btn"),
            "class 'btn' attached: {:?}",
            link.attrs.classes
        );
        if let NodeKind::Link { title, .. } = &link.kind {
            assert!(
                title.as_deref() != Some("class='btn'"),
                "raw directive title must be cleared"
            );
        }
    }

    #[test]
    fn context_fold_parses_structured_link_target() {
        let policies = empty_policies();
        let ctx = ctx_for(&policies);
        let doc = fold_test(
            "[x](https://example.com \"target='_blank'\")\n",
            &ctx,
        );
        let link = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Link { .. }))
            .expect("link node");
        let browser = link.attrs.browser_ref().expect("browser attrs");
        let link_attrs = browser.link.as_ref().expect("link browser attrs");
        assert_eq!(
            link_attrs.target,
            Some(renderable::tree::LinkTarget::Blank)
        );
    }

    #[test]
    fn context_fold_parses_structured_link_data_prompt() {
        let policies = empty_policies();
        let ctx = ctx_for(&policies);
        let doc = fold_test(
            "[x](https://example.com \"prompt='explain'\")\n",
            &ctx,
        );
        let link = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Link { .. }))
            .expect("link node");
        let browser = link.attrs.browser_ref().expect("browser attrs");
        let prompt_name = renderable::tree::DataAttrName::new("prompt").unwrap();
        assert_eq!(
            browser.data_attrs.get(&prompt_name).map(|s| s.as_str()),
            Some("explain")
        );
    }

    #[test]
    fn context_fold_parses_structured_link_inline_style() {
        let policies = empty_policies();
        let ctx = ctx_for(&policies);
        let doc = fold_test(
            "[x](https://example.com \"style='color: red'\")\n",
            &ctx,
        );
        let link = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Link { .. }))
            .expect("link node");
        let browser = link.attrs.browser_ref().expect("browser attrs");
        let css = browser.inline_style.as_ref().expect("inline_style");
        assert!(
            css.to_css().contains("color: red"),
            "css: {}",
            css.to_css()
        );
    }

    // ── link children preserved ────────────────────────────────────────────

    #[test]
    fn context_fold_preserves_link_children() {
        let policies = empty_policies();
        let ctx = ctx_for(&policies);
        let doc = fold_test("[**bold** label](https://example.com)\n", &ctx);
        let link = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Link { .. }))
            .expect("link node");
        assert!(
            link.children()
                .iter()
                .any(|c| matches!(c.kind, NodeKind::Strong { .. })),
            "strong child preserved in link"
        );
    }

    // ── image alt preserved ────────────────────────────────────────────────

    #[test]
    fn context_fold_preserves_image_alt() {
        let policies = empty_policies();
        let ctx = ctx_for(&policies);
        let doc = fold_test("![alt text](img.png)\n", &ctx);
        let image = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Image { .. }))
            .expect("image node");
        if let NodeKind::Image { alt, .. } = &image.kind {
            assert_eq!(alt, "alt text", "alt text must be unchanged");
        }
    }

    // ── text layout hints ──────────────────────────────────────────────────

    #[test]
    fn context_fold_attaches_text_layout_from_hyperlink_style() {
        use crate::style::schema::{CommonStyle, WidthOrMode};
        use renderable::layout::{Length, TargetValue, Width};

        let common = CommonStyle {
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(20))))),
            ..Default::default()
        };
        let policies = empty_policies();
        let mut ctx = ctx_for(&policies);
        ctx.hyperlink_style = Some(&common);
        let doc = fold_test("[label](https://example.com)\n", &ctx);
        let link = find_node(&doc.root, &|n| matches!(n.kind, NodeKind::Link { .. }))
            .expect("link node");
        let hints = link.attrs.text_layout_ref().expect("text_layout hints");
        assert!(hints.width.is_some(), "width hint attached");
    }
}

/// Finding 35.7 regression coverage.
///
/// `apply_link_policy` / `apply_image_policy` stopped cloning the node's URL and
/// title before deciding anything and now borrow them, resolving each decision
/// into an owned value before the node is mutated. Both are pure performance
/// changes, so the contract is exact equality with the previous appliers —
/// proven differentially rather than by re-asserting hand-picked attrs.
#[cfg(test)]
mod finding_35_7 {
    use super::*;
    use std::collections::HashMap;

    /// The pre-optimization `apply_link_policy`, verbatim.
    fn baseline_apply_link_policy(node: &mut RenderNode, ctx: &TreeBuildContext) {
        let (url, title) = match &node.kind {
            NodeKind::Link { url, title, .. } => (url.clone(), title.clone()),
            _ => return,
        };

        let is_local = is_local_link(&url);

        let (fg, bg) = ctx.hyperlink_color(is_local);
        if fg.is_some() || bg.is_some() {
            let mut style = node.attrs.style().unwrap_or_default();
            set_style_colors(&mut style, fg.as_ref(), bg.as_ref());
            node.attrs.set_style(&style);
        }

        let common = ctx.effective_hyperlink_style(is_local);
        if let Some(ref common) = common {
            attach_text_layout(node, common);
        }

        if let Some(raw_title) = title.as_ref() {
            let frontmatter_css = common
                .as_ref()
                .and_then(|c| c.to_css_overlay())
                .map(|c| c.to_css().replace('\n', " "));
            if let Some(directive) = parse_link_directive(&url, raw_title, frontmatter_css.as_deref())
            {
                directive.apply_to_link_node(node);
            }
        }
    }

    /// The pre-optimization `apply_image_policy`, verbatim.
    fn baseline_apply_image_policy(node: &mut RenderNode, ctx: &TreeBuildContext) {
        let (url, title) = match &node.kind {
            NodeKind::Image { url, title, .. } => (url.clone(), title.clone()),
            _ => return,
        };

        let is_local = is_local_image(&url);

        let (fg, bg) = ctx.image_color(is_local);
        if fg.is_some() || bg.is_some() {
            let mut style = node.attrs.style().unwrap_or_default();
            set_style_colors(&mut style, fg.as_ref(), bg.as_ref());
            node.attrs.set_style(&style);
        }

        if is_local
            && let Some(common) = ctx.local_image_style
        {
            attach_text_layout(node, common);
        }

        if let Some(raw_title) = title.as_ref() {
            let frontmatter_css = ctx
                .local_image_style
                .and_then(|c| c.to_css_overlay())
                .map(|c| c.to_css().replace('\n', " "));
            if let Some(directive) =
                parse_image_directive(&url, raw_title, frontmatter_css.as_deref())
            {
                directive.apply_to_image_node(node);
            }
        }
    }

    fn node(kind: NodeKind) -> RenderNode {
        RenderNode {
            kind,
            span: renderable::tree::SourceSpan::synthetic(),
            attrs: renderable::tree::NodeAttrs::default(),
        }
    }

    fn text(value: &str) -> RenderNode {
        node(NodeKind::Text {
            value: value.to_string(),
        })
    }

    fn styled(color: &str) -> CommonStyle {
        CommonStyle {
            color: Some(crate::style::StyleColor {
                color: renderable::color::Color::Tailwind(
                    renderable::color::Tailwind::from_kebab_name(color).unwrap(),
                ),
                opacity: None,
            }),
            ..Default::default()
        }
    }

    /// URL / title pairs spanning local vs remote, absent vs present titles, and
    /// titles that do and do not parse as structured directives.
    fn url_title_cases() -> Vec<(&'static str, Option<&'static str>)> {
        vec![
            ("./local/doc.md", None),
            ("./local/doc.md", Some("A plain title")),
            ("./local/doc.md#anchor", Some("Titled with anchor")),
            ("../sibling/doc.md", None),
            ("/absolute/doc.md", Some("Absolute")),
            ("https://example.com/page", None),
            ("https://example.com/page", Some("Remote title")),
            ("https://example.com/page", Some("class=hero")),
            ("./img/pic.png", Some("style=color:red")),
            ("./img/pic.png", Some("")),
            ("mailto:someone@example.com", None),
            ("#in-page-anchor", Some("Anchor only")),
            ("", None),
            ("./unicode/é—ü.md", Some("Ünïcödé — title")),
        ]
    }

    /// Every meaningful context shape the appliers branch on.
    fn contexts<'a>(
        policies: &'a HashMap<PageComponent, ComponentPolicy>,
        global: &'a CommonStyle,
        local: &'a CommonStyle,
        image: &'a CommonStyle,
    ) -> Vec<(&'static str, TreeBuildContext<'a>)> {
        let base = || TreeBuildContext {
            component_policies: policies,
            page_color: None,
            page_bg_color: None,
            hyperlink_style: None,
            local_hyperlink_style: None,
            local_image_style: None,
            hr_defaults: None,
        };

        vec![
            ("empty policy", base()),
            (
                "global hyperlink style",
                TreeBuildContext {
                    hyperlink_style: Some(global),
                    ..base()
                },
            ),
            (
                "local hyperlink override",
                TreeBuildContext {
                    hyperlink_style: Some(global),
                    local_hyperlink_style: Some(local),
                    ..base()
                },
            ),
            (
                "local hyperlink only",
                TreeBuildContext {
                    local_hyperlink_style: Some(local),
                    ..base()
                },
            ),
            (
                "local image style",
                TreeBuildContext {
                    local_image_style: Some(image),
                    ..base()
                },
            ),
        ]
    }

    #[test]
    fn link_policy_matches_the_pre_optimization_applier() {
        let policies = HashMap::new();
        let (global, local, image) = (styled("red-500"), styled("blue-500"), styled("green-500"));

        for (ctx_label, ctx) in contexts(&policies, &global, &local, &image) {
            for (url, title) in url_title_cases() {
                let make = || {
                    node(NodeKind::Link {
                        url: url.to_string(),
                        title: title.map(str::to_string),
                        children: vec![text("link text")],
                    })
                };

                let mut actual = make();
                let mut expected = make();
                apply_link_policy(&mut actual, &ctx);
                baseline_apply_link_policy(&mut expected, &ctx);

                assert_eq!(
                    format!("{actual:?}"),
                    format!("{expected:?}"),
                    "link policy differs under {ctx_label:?} for url={url:?} title={title:?}"
                );
            }
        }
    }

    #[test]
    fn image_policy_matches_the_pre_optimization_applier() {
        let policies = HashMap::new();
        let (global, local, image) = (styled("red-500"), styled("blue-500"), styled("green-500"));

        for (ctx_label, ctx) in contexts(&policies, &global, &local, &image) {
            for (url, title) in url_title_cases() {
                let make = || {
                    node(NodeKind::Image {
                        url: url.to_string(),
                        title: title.map(str::to_string),
                        alt: "alt text".to_string(),
                    })
                };

                let mut actual = make();
                let mut expected = make();
                apply_image_policy(&mut actual, &ctx);
                baseline_apply_image_policy(&mut expected, &ctx);

                assert_eq!(
                    format!("{actual:?}"),
                    format!("{expected:?}"),
                    "image policy differs under {ctx_label:?} for url={url:?} title={title:?}"
                );
            }
        }
    }

    /// The appliers are called for every node in the fold, so a non-link/image
    /// node must still be left untouched by both.
    #[test]
    fn unrelated_nodes_are_untouched() {
        let policies = HashMap::new();
        let (global, local, image) = (styled("red-500"), styled("blue-500"), styled("green-500"));

        for (ctx_label, ctx) in contexts(&policies, &global, &local, &image) {
            let mut n = text("just text");
            let before = format!("{n:?}");
            apply_link_policy(&mut n, &ctx);
            apply_image_policy(&mut n, &ctx);
            assert_eq!(format!("{n:?}"), before, "text node mutated under {ctx_label:?}");
        }
    }

    /// 1000 synthetic link nodes shaped like `toc_large`'s TOC entries, per the
    /// profile record's fixture description.
    fn synthetic_links(with_title: bool) -> Vec<RenderNode> {
        (0..1000)
            .map(|n| {
                node(NodeKind::Link {
                    url: format!("./docs/chapter-{n}/section-{n}.md#heading-{n}"),
                    title: with_title.then(|| format!("Section {n}")),
                    children: vec![text(&format!("Section {n}"))],
                })
            })
            .collect()
    }

    /// Retained raw-sample harness for Finding 35.7 (run record:
    /// `benchmarks/raw/f35-residuals/`).
    ///
    /// Replaces the deleted `f35_7_profile` module whose capture left the
    /// finding's claim unreproducible. Ignored *and* gated on `DM_PERF_RAW_DIR`,
    /// so `just test` neither runs nor is slowed by it.
    ///
    /// Each timed batch applies the policy to all 1000 nodes in place. The two
    /// arms own separate node vectors, so neither observes the other's
    /// mutations; because the appliers are equivalent (gated below), both
    /// vectors evolve through identical states across samples.
    #[test]
    #[ignore = "measurement harness; opt in with DM_PERF_RAW_DIR"]
    fn f35_7_link_policy_raw_samples() {
        let Some(harness) = crate::perf_harness::Harness::from_env(50, 1) else {
            return;
        };

        let policies = HashMap::new();
        let global = styled("red-500");
        let base = || TreeBuildContext {
            component_policies: &policies,
            page_color: None,
            page_bg_color: None,
            hyperlink_style: None,
            local_hyperlink_style: None,
            local_image_style: None,
            hr_defaults: None,
        };
        let empty_policy = base();
        let hyperlink_policy = TreeBuildContext {
            hyperlink_style: Some(&global),
            ..base()
        };

        let cases: Vec<(&str, &TreeBuildContext, bool)> = vec![
            ("empty-policy-no-title", &empty_policy, false),
            ("empty-policy-with-title", &empty_policy, true),
            ("hyperlink-policy-no-title", &hyperlink_policy, false),
            ("hyperlink-policy-with-title", &hyperlink_policy, true),
        ];

        // Equivalence gate: a ratio between two appliers that disagree is not a
        // result. Every measured node is compared by `Debug`, before any timing.
        for (label, ctx, with_title) in &cases {
            let mut candidate_nodes = synthetic_links(*with_title);
            let mut baseline_nodes = synthetic_links(*with_title);
            for (candidate, baseline) in candidate_nodes.iter_mut().zip(baseline_nodes.iter_mut()) {
                apply_link_policy(candidate, ctx);
                baseline_apply_link_policy(baseline, ctx);
                assert_eq!(
                    format!("{candidate:?}"),
                    format!("{baseline:?}"),
                    "F35.7 baseline and candidate must agree on every {label} node"
                );
            }
        }

        for (label, ctx, with_title) in &cases {
            let mut baseline_nodes = synthetic_links(*with_title);
            let mut candidate_nodes = synthetic_links(*with_title);
            harness.interleaved_pair(
                &format!("f35_7-{label}-baseline"),
                || {
                    for n in baseline_nodes.iter_mut() {
                        baseline_apply_link_policy(n, ctx);
                    }
                },
                &format!("f35_7-{label}-candidate"),
                || {
                    for n in candidate_nodes.iter_mut() {
                        apply_link_policy(n, ctx);
                    }
                },
            );
        }
    }
}
