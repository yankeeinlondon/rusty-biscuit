//! CLI-neutral style claims and their application to a [`DarkmatterPage`].
//!
//! This module collapses the duplication between the CLI's layout-flag value
//! application and the `style:` frontmatter override bits that keep CLI flags
//! from being overwritten. It is intentionally free of clap or CLI-only
//! wrapper types; callers convert their local CLI types into the neutral
//! [`CliStyleClaims`] model first.
//!
//! ## Precedence
//!
//! Margin and padding follow CSS shorthand precedence:
//! `--margin` (all) > `--mx` / `--my` (axis) > `--mt` / `--ml` / etc. (side).
//! Alignment and fill follow the same global-then-component-specific pattern:
//! `--alignment` / `--fill` (all components) > `--align-lists` / `--fill-lists`
//! (list bucket) > `--align-ul` / `--fill-ul` (single component).

use renderable::layout::{Alignment, Length};
use renderable::style::PaintColor;

use crate::layout::{DarkmatterPage, PageBackground, PageComponent};
use crate::markdown::highlighting::ThemePair;
use crate::style::{
    BespokeStyleOverrides, ComponentStyleOverrides, DisclosureStyleOverrides,
    HrStyleOverrides, ListStyleOverrides, PageStyleOverrides,
};

/// A CLI-neutral fill claim lowered from a CLI-local fill descriptor.
///
/// Mirrors the shape of `darkmatter-cli`'s `CliFill` but uses only
/// [`renderable`] layout types so the library does not depend on clap.
#[derive(Debug, Clone, PartialEq)]
pub enum FillClaim {
    /// Default. Component may use the full content width.
    Full,
    /// Symmetric padding on both sides.
    Pad(Length),
    /// One-sided padding driven by the component's alignment.
    Indent(Length),
    /// Cap on the component's render width.
    Max(Length),
    /// Explicit render width.
    Explicit(Length),
}

/// CLI-neutral summary of every style/layout flag a CLI invocation may claim.
///
/// All fields are `Option`s in library types. The CLI builder
/// (`darkmatter-cli::style_claims::cli_style_claims`) is the single location
/// that knows how to convert clap wrappers (`PageBackgroundArg`,
/// `PageAlignmentArg`, `CliFill`) into these neutral values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CliStyleClaims {
    /// Margin on all sides.
    pub margin: Option<u16>,
    /// Horizontal margin (left + right).
    pub margin_x: Option<u16>,
    /// Vertical margin (top + bottom).
    pub margin_y: Option<u16>,
    /// Top margin.
    pub margin_top: Option<u16>,
    /// Bottom margin.
    pub margin_bottom: Option<u16>,
    /// Left margin.
    pub margin_left: Option<u16>,
    /// Right margin.
    pub margin_right: Option<u16>,

    /// Padding on all sides.
    pub padding: Option<u16>,
    /// Horizontal padding (left + right).
    pub padding_x: Option<u16>,
    /// Vertical padding (top + bottom).
    pub padding_y: Option<u16>,
    /// Top padding.
    pub padding_top: Option<u16>,
    /// Bottom padding.
    pub padding_bottom: Option<u16>,
    /// Left padding.
    pub padding_left: Option<u16>,
    /// Right padding.
    pub padding_right: Option<u16>,

    /// Page background style.
    pub page_background: Option<PageBackground>,
    /// Explicit page background color.
    pub page_bg_color: Option<PaintColor>,
    /// Max content width.
    pub max_width: Option<u16>,

    /// Default alignment for all components.
    pub alignment: Option<Alignment>,
    /// Image alignment.
    pub align_images: Option<Alignment>,
    /// List bucket alignment.
    pub align_lists: Option<Alignment>,
    /// Unordered list alignment.
    pub align_ul: Option<Alignment>,
    /// Ordered list alignment.
    pub align_ol: Option<Alignment>,
    /// List item alignment.
    pub align_li: Option<Alignment>,
    /// Block quote alignment.
    pub align_block_quotes: Option<Alignment>,
    /// Table alignment.
    pub align_tables: Option<Alignment>,
    /// Code block alignment.
    pub align_code_blocks: Option<Alignment>,

    /// Default fill for all components.
    pub fill: Option<FillClaim>,
    /// Image fill.
    pub fill_images: Option<FillClaim>,
    /// List bucket fill.
    pub fill_lists: Option<FillClaim>,
    /// Unordered list fill.
    pub fill_ul: Option<FillClaim>,
    /// Ordered list fill.
    pub fill_ol: Option<FillClaim>,
    /// List item fill.
    pub fill_li: Option<FillClaim>,
    /// Block quote fill.
    pub fill_block_quotes: Option<FillClaim>,
    /// Table fill.
    pub fill_tables: Option<FillClaim>,
    /// Code block fill.
    pub fill_code_blocks: Option<FillClaim>,

    /// Page-level code theme override.
    pub code_theme: Option<ThemePair>,
}

/// Apply the value side of CLI layout claims onto a [`DarkmatterPage`].
///
/// Follows the precedence rules documented on [`CliStyleClaims`]. The returned
/// page reflects every CLI flag that was present in `claims`.
pub fn apply_cli_claims(mut page: DarkmatterPage, claims: &CliStyleClaims) -> DarkmatterPage {
    // Margin precedence: all > axis > side.
    if let Some(n) = claims.margin {
        page = page.with_margin(n);
    }
    if let Some(n) = claims.margin_x {
        page = page.with_margin_x(n);
    }
    if let Some(n) = claims.margin_y {
        page = page.with_margin_y(n);
    }
    if let Some(n) = claims.margin_top {
        page = page.with_margin_top(n);
    }
    if let Some(n) = claims.margin_bottom {
        page = page.with_margin_bottom(n);
    }
    if let Some(n) = claims.margin_left {
        page = page.with_margin_left(n);
    }
    if let Some(n) = claims.margin_right {
        page = page.with_margin_right(n);
    }

    // Padding precedence: all > axis > side.
    if let Some(n) = claims.padding {
        page = page.with_padding(n);
    }
    if let Some(n) = claims.padding_x {
        page = page.with_padding_x(n);
    }
    if let Some(n) = claims.padding_y {
        page = page.with_padding_y(n);
    }
    if let Some(n) = claims.padding_top {
        page = page.with_padding_top(n);
    }
    if let Some(n) = claims.padding_bottom {
        page = page.with_padding_bottom(n);
    }
    if let Some(n) = claims.padding_left {
        page = page.with_padding_left(n);
    }
    if let Some(n) = claims.padding_right {
        page = page.with_padding_right(n);
    }

    // Page background and explicit background color.
    if let Some(bg) = claims.page_background {
        page = page.with_page_background(bg);
    }
    if let Some(color) = claims.page_bg_color {
        page = page.with_page_bg_color(color);
    }

    // Max width.
    if let Some(n) = claims.max_width {
        page = page.with_max_width(n);
    }

    // Alignment precedence: global > component-specific.
    if let Some(align) = claims.alignment {
        for component in PageComponent::ALL {
            page = apply_component_alignment(page, component, align);
        }
    }
    if let Some(align) = claims.align_images {
        page = apply_component_alignment(page, PageComponent::Images, align);
    }
    if let Some(align) = claims.align_lists {
        for component in [PageComponent::Ul, PageComponent::Ol, PageComponent::Li] {
            page = apply_component_alignment(page, component, align);
        }
    }
    if let Some(align) = claims.align_ul {
        page = apply_component_alignment(page, PageComponent::Ul, align);
    }
    if let Some(align) = claims.align_ol {
        page = apply_component_alignment(page, PageComponent::Ol, align);
    }
    if let Some(align) = claims.align_li {
        page = apply_component_alignment(page, PageComponent::Li, align);
    }
    if let Some(align) = claims.align_block_quotes {
        page = apply_component_alignment(page, PageComponent::BlockQuotes, align);
    }
    if let Some(align) = claims.align_tables {
        page = apply_component_alignment(page, PageComponent::Tables, align);
    }
    if let Some(align) = claims.align_code_blocks {
        page = apply_component_alignment(page, PageComponent::CodeBlocks, align);
    }

    // Fill precedence: global > component-specific.
    if let Some(ref fill) = claims.fill {
        for component in PageComponent::ALL {
            page = apply_component_fill(page, component, fill);
        }
    }
    if let Some(ref fill) = claims.fill_images {
        page = apply_component_fill(page, PageComponent::Images, fill);
    }
    if let Some(ref fill) = claims.fill_lists {
        for component in [PageComponent::Ul, PageComponent::Ol, PageComponent::Li] {
            page = apply_component_fill(page, component, fill);
        }
    }
    if let Some(ref fill) = claims.fill_ul {
        page = apply_component_fill(page, PageComponent::Ul, fill);
    }
    if let Some(ref fill) = claims.fill_ol {
        page = apply_component_fill(page, PageComponent::Ol, fill);
    }
    if let Some(ref fill) = claims.fill_li {
        page = apply_component_fill(page, PageComponent::Li, fill);
    }
    if let Some(ref fill) = claims.fill_block_quotes {
        page = apply_component_fill(page, PageComponent::BlockQuotes, fill);
    }
    if let Some(ref fill) = claims.fill_tables {
        page = apply_component_fill(page, PageComponent::Tables, fill);
    }
    if let Some(ref fill) = claims.fill_code_blocks {
        page = apply_component_fill(page, PageComponent::CodeBlocks, fill);
    }

    page
}

/// Set `alignment` on `component`, merging with any existing [`ComponentPolicy`].
fn apply_component_alignment(
    page: DarkmatterPage,
    component: PageComponent,
    alignment: Alignment,
) -> DarkmatterPage {
    let mut policy = page.component_policy(component).cloned().unwrap_or_default();
    policy.layout.alignment = alignment;
    page.with_component_policy(component, policy)
}

/// Apply a [`FillClaim`] to `component`, merging with any existing [`ComponentPolicy`].
fn apply_component_fill(
    page: DarkmatterPage,
    component: PageComponent,
    fill: &FillClaim,
) -> DarkmatterPage {
    use renderable::layout::{Edges, TargetValue, Width};

    let mut policy = page.component_policy(component).cloned().unwrap_or_default();
    match fill {
        FillClaim::Full => {
            policy.layout.width = Width::Auto;
            policy.layout.max_width = None;
            policy.layout.padding = Edges::default();
        }
        FillClaim::Pad(length) => {
            policy.layout.padding = Edges::x(length.clone());
        }
        FillClaim::Indent(length) => {
            policy.layout.padding = match policy.layout.alignment {
                Alignment::Left => Edges {
                    left: TargetValue::universal(length.clone()),
                    ..Edges::default()
                },
                Alignment::Right => Edges {
                    right: TargetValue::universal(length.clone()),
                    ..Edges::default()
                },
                Alignment::Center => Edges::x(length.clone()),
            };
        }
        FillClaim::Max(length) => {
            policy.layout.max_width = Some(TargetValue::universal(length.clone()));
        }
        FillClaim::Explicit(length) => {
            policy.layout.width = Width::Fixed(TargetValue::universal(length.clone()));
        }
    }
    page.with_component_policy(component, policy)
}

/// Build a [`PageStyleOverrides`] reflecting which `style.page.*` fields the
/// CLI has already claimed.
///
/// Mirrors the shorthand expansion rules in [`apply_cli_claims`]: `--margin`
/// claims all four sides, `--mx` claims left + right, `--my` claims top +
/// bottom. Padding follows the same pattern. `--max-width`, `--page-bg`, and
/// `--alignment` each claim their corresponding page-level field. The
/// component-specific alignment flags each claim their component so the
/// `style.page.alignment` broadcast does not silently overwrite them.
pub fn page_style_overrides_from_claims(claims: &CliStyleClaims) -> PageStyleOverrides {
    let margin_all = claims.margin.is_some();
    let margin_x = claims.margin_x.is_some();
    let margin_y = claims.margin_y.is_some();
    let padding_all = claims.padding.is_some();
    let padding_x = claims.padding_x.is_some();
    let padding_y = claims.padding_y.is_some();

    PageStyleOverrides {
        margin_top: margin_all || margin_y || claims.margin_top.is_some(),
        margin_right: margin_all || margin_x || claims.margin_right.is_some(),
        margin_bottom: margin_all || margin_y || claims.margin_bottom.is_some(),
        margin_left: margin_all || margin_x || claims.margin_left.is_some(),
        padding_top: padding_all || padding_y || claims.padding_top.is_some(),
        padding_right: padding_all || padding_x || claims.padding_right.is_some(),
        padding_bottom: padding_all || padding_y || claims.padding_bottom.is_some(),
        padding_left: padding_all || padding_x || claims.padding_left.is_some(),
        max_width: claims.max_width.is_some(),
        background: claims.page_background.is_some(),
        background_color: claims.page_bg_color.is_some(),
        alignment: claims.alignment.is_some(),
        align_images: claims.align_images.is_some(),
        align_lists: claims.align_lists.is_some(),
        align_ul: claims.align_ul.is_some(),
        align_ol: claims.align_ol.is_some(),
        align_li: claims.align_li.is_some(),
        align_block_quotes: claims.align_block_quotes.is_some(),
        align_tables: claims.align_tables.is_some(),
        align_code_blocks: claims.align_code_blocks.is_some(),
    }
}

/// Build a [`ListStyleOverrides`] reflecting which list-bucket frontmatter
/// fields the CLI has already claimed.
///
/// Mirrors the global/component-specific precedence in [`apply_cli_claims`]:
/// the global `--alignment` flag claims every `*_alignment` field, the global
/// `--fill` flag claims every `*_fill` field, the broadcast `--align-lists` /
/// `--fill-lists` flags claim all three list components, and the granular flags
/// each claim only their own field.
pub fn list_style_overrides_from_claims(claims: &CliStyleClaims) -> ListStyleOverrides {
    let alignment_all = claims.alignment.is_some();
    let fill_all = claims.fill.is_some();
    let align_lists_broadcast = claims.align_lists.is_some();
    let fill_lists_broadcast = claims.fill_lists.is_some();

    ListStyleOverrides {
        ul_alignment: alignment_all || align_lists_broadcast || claims.align_ul.is_some(),
        ul_fill: fill_all || fill_lists_broadcast || claims.fill_ul.is_some(),
        ul_left_margin: false,
        ol_alignment: alignment_all || align_lists_broadcast || claims.align_ol.is_some(),
        ol_fill: fill_all || fill_lists_broadcast || claims.fill_ol.is_some(),
        li_alignment: alignment_all || align_lists_broadcast || claims.align_li.is_some(),
        li_fill: fill_all || fill_lists_broadcast || claims.fill_li.is_some(),
    }
}

/// Build a [`ComponentStyleOverrides`] reflecting which component-bucket
/// frontmatter fields the CLI has already claimed.
///
/// Mirrors the global/component-specific precedence in [`apply_cli_claims`]:
/// the global `--alignment` flag claims every `*_alignment` field, the global
/// `--fill` flag claims every `*_fill` field, and the component-specific flags
/// each claim only their own field.
pub fn component_style_overrides_from_claims(claims: &CliStyleClaims) -> ComponentStyleOverrides {
    let alignment_all = claims.alignment.is_some();
    let fill_all = claims.fill.is_some();

    ComponentStyleOverrides {
        tables_alignment: alignment_all || claims.align_tables.is_some(),
        tables_fill: fill_all || claims.fill_tables.is_some(),
        images_alignment: alignment_all || claims.align_images.is_some(),
        images_fill: fill_all || claims.fill_images.is_some(),
        block_quotes_alignment: alignment_all || claims.align_block_quotes.is_some(),
        block_quotes_fill: fill_all || claims.fill_block_quotes.is_some(),
        code_blocks_alignment: alignment_all || claims.align_code_blocks.is_some(),
        code_blocks_fill: fill_all || claims.fill_code_blocks.is_some(),
    }
}

/// Build an [`HrStyleOverrides`] reflecting which HR frontmatter fields the CLI
/// has already claimed.
///
/// There are currently no HR-specific CLI flags, so this always returns the
/// default (no overrides). It exists for symmetry with the other override
/// helpers and to make adding HR CLI flags a one-line change.
pub fn hr_style_overrides_from_claims(_claims: &CliStyleClaims) -> HrStyleOverrides {
    HrStyleOverrides::default()
}

/// Build a [`DisclosureStyleOverrides`] reflecting which disclosure frontmatter
/// fields the CLI has already claimed.
///
/// There are currently no disclosure-specific CLI flags, so this always returns
/// the default (no overrides). It exists for symmetry with the other override
/// helpers and to make adding disclosure CLI flags a one-line change.
pub fn disclosure_style_overrides_from_claims(_claims: &CliStyleClaims) -> DisclosureStyleOverrides {
    DisclosureStyleOverrides::default()
}

/// Build a [`BespokeStyleOverrides`] reflecting which bespoke frontmatter
/// fields the CLI has already claimed.
///
/// Mirrors the CLI precedence in [`apply_cli_claims`]: `--code-theme` claims
/// the `style.page.code.theme` field so the frontmatter value is skipped.
pub fn bespoke_style_overrides_from_claims(claims: &CliStyleClaims) -> BespokeStyleOverrides {
    BespokeStyleOverrides {
        code_theme: claims.code_theme.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::terminal::Terminal;
    use renderable::layout::{Edges, Length, TargetValue, Width};

    fn claims() -> CliStyleClaims {
        CliStyleClaims::default()
    }

    fn page(width: u32) -> DarkmatterPage {
        let term = Terminal::new_optimistic(width);
        DarkmatterPage::new(&term)
    }

    #[test]
    fn margin_shorthand_claims_all_sides() {
        let mut c = claims();
        c.margin = Some(4);
        let p = apply_cli_claims(page(80), &c);
        assert_eq!(edge_ch(&p.page_margin().top), 4);
        assert_eq!(edge_ch(&p.page_margin().bottom), 4);
        assert_eq!(edge_ch(&p.page_margin().left), 4);
        assert_eq!(edge_ch(&p.page_margin().right), 4);
    }

    #[test]
    fn margin_axis_claims_only_that_axis() {
        let mut c = claims();
        c.margin_x = Some(2);
        let p = apply_cli_claims(page(80), &c);
        assert_eq!(edge_ch(&p.page_margin().left), 2);
        assert_eq!(edge_ch(&p.page_margin().right), 2);
        assert_eq!(edge_ch(&p.page_margin().top), 0);
        assert_eq!(edge_ch(&p.page_margin().bottom), 0);
    }

    #[test]
    fn margin_side_specific_only_claims_that_side() {
        let mut c = claims();
        c.margin_top = Some(1);
        c.margin_left = Some(3);
        let p = apply_cli_claims(page(80), &c);
        assert_eq!(edge_ch(&p.page_margin().top), 1);
        assert_eq!(edge_ch(&p.page_margin().left), 3);
        assert_eq!(edge_ch(&p.page_margin().right), 0);
        assert_eq!(edge_ch(&p.page_margin().bottom), 0);
    }

    #[test]
    fn margin_shorthand_wins_over_axis_and_side() {
        let mut c = claims();
        c.margin = Some(4);
        c.margin_x = Some(2);
        c.margin_left = Some(1);
        let p = apply_cli_claims(page(80), &c);
        // `with_margin` runs first, then `with_margin_x`, then `with_margin_left`.
        assert_eq!(edge_ch(&p.page_margin().left), 1);
        assert_eq!(edge_ch(&p.page_margin().right), 2);
        assert_eq!(edge_ch(&p.page_margin().top), 4);
        assert_eq!(edge_ch(&p.page_margin().bottom), 4);
    }

    #[test]
    fn padding_shorthand_claims_all_sides() {
        let mut c = claims();
        c.padding = Some(3);
        let p = apply_cli_claims(page(80), &c);
        assert_eq!(edge_ch(&p.page_padding().top), 3);
        assert_eq!(edge_ch(&p.page_padding().bottom), 3);
        assert_eq!(edge_ch(&p.page_padding().left), 3);
        assert_eq!(edge_ch(&p.page_padding().right), 3);
    }

    #[test]
    fn padding_axis_claims_only_that_axis() {
        let mut c = claims();
        c.padding_y = Some(2);
        let p = apply_cli_claims(page(80), &c);
        assert_eq!(edge_ch(&p.page_padding().top), 2);
        assert_eq!(edge_ch(&p.page_padding().bottom), 2);
        assert_eq!(edge_ch(&p.page_padding().left), 0);
        assert_eq!(edge_ch(&p.page_padding().right), 0);
    }

    #[test]
    fn max_width_claim_applied() {
        let mut c = claims();
        c.max_width = Some(72);
        let p = apply_cli_claims(page(80), &c);
        assert_eq!(p.max_width(), Some(72));
    }

    #[test]
    fn page_background_claim_applied() {
        let mut c = claims();
        c.page_background = Some(PageBackground::Subtle);
        let p = apply_cli_claims(page(80), &c);
        assert_eq!(p.page_background(), PageBackground::Subtle);
    }

    #[test]
    fn global_alignment_broadcasts_to_all_components() {
        let mut c = claims();
        c.alignment = Some(Alignment::Center);
        let p = apply_cli_claims(page(80), &c);
        for component in PageComponent::ALL {
            assert_eq!(
                p.component_policy(component).unwrap().layout.alignment,
                Alignment::Center,
                "alignment should broadcast to {:?}",
                component
            );
        }
    }

    #[test]
    fn align_lists_claims_all_three_list_components() {
        let mut c = claims();
        c.align_lists = Some(Alignment::Right);
        let p = apply_cli_claims(page(80), &c);
        for component in [PageComponent::Ul, PageComponent::Ol, PageComponent::Li] {
            assert_eq!(
                p.component_policy(component).unwrap().layout.alignment,
                Alignment::Right
            );
        }
    }

    #[test]
    fn align_ul_claims_only_unordered_list() {
        let mut c = claims();
        c.align_ul = Some(Alignment::Right);
        let p = apply_cli_claims(page(80), &c);
        assert_eq!(
            p.component_policy(PageComponent::Ul).unwrap().layout.alignment,
            Alignment::Right
        );
        assert_eq!(
            p.component_policy(PageComponent::Ol)
                .map(|p| p.layout.alignment)
                .unwrap_or(Alignment::Left),
            Alignment::Left
        );
        assert_eq!(
            p.component_policy(PageComponent::Li)
                .map(|p| p.layout.alignment)
                .unwrap_or(Alignment::Left),
            Alignment::Left
        );
    }

    #[test]
    fn global_alignment_does_not_touch_unrelated_components_twice() {
        let mut c = claims();
        c.alignment = Some(Alignment::Center);
        c.align_images = Some(Alignment::Left);
        let p = apply_cli_claims(page(80), &c);
        assert_eq!(
            p.component_policy(PageComponent::Images).unwrap().layout.alignment,
            Alignment::Left
        );
        assert_eq!(
            p.component_policy(PageComponent::Tables).unwrap().layout.alignment,
            Alignment::Center
        );
    }

    #[test]
    fn fill_pad_applies_symmetric_padding() {
        let mut c = claims();
        c.fill = Some(FillClaim::Pad(Length::ch(4)));
        let p = apply_cli_claims(page(80), &c);
        for component in PageComponent::ALL {
            assert_eq!(
                p.component_policy(component).unwrap().layout.padding,
                Edges::x(Length::ch(4)),
                "pad should apply symmetric padding to {:?}",
                component
            );
        }
    }

    #[test]
    fn fill_indent_applies_left_padding_for_left_alignment() {
        let mut c = claims();
        c.fill = Some(FillClaim::Indent(Length::ch(4)));
        let p = apply_cli_claims(page(80), &c);
        for component in PageComponent::ALL {
            assert_eq!(
                p.component_policy(component).unwrap().layout.padding,
                Edges {
                    left: TargetValue::universal(Length::ch(4)),
                    ..Edges::default()
                },
                "indent should apply left padding to {:?}",
                component
            );
        }
    }

    #[test]
    fn fill_max_applies_max_width() {
        let mut c = claims();
        c.fill_tables = Some(FillClaim::Max(Length::ch(60)));
        let p = apply_cli_claims(page(80), &c);
        assert_eq!(
            p.component_policy(PageComponent::Tables)
                .unwrap()
                .layout
                .max_width,
            Some(TargetValue::universal(Length::ch(60)))
        );
    }

    #[test]
    fn fill_explicit_applies_fixed_width() {
        let mut c = claims();
        c.fill_code_blocks = Some(FillClaim::Explicit(Length::ch(40)));
        let p = apply_cli_claims(page(80), &c);
        assert_eq!(
            p.component_policy(PageComponent::CodeBlocks)
                .unwrap()
                .layout
                .width,
            Width::Fixed(TargetValue::universal(Length::ch(40)))
        );
    }

    #[test]
    fn fill_lists_broadcasts_to_all_list_components() {
        let mut c = claims();
        c.fill_lists = Some(FillClaim::Pad(Length::ch(2)));
        let p = apply_cli_claims(page(80), &c);
        for component in [PageComponent::Ul, PageComponent::Ol, PageComponent::Li] {
            assert_eq!(
                p.component_policy(component).unwrap().layout.padding,
                Edges::x(Length::ch(2))
            );
        }
    }

    #[test]
    fn page_override_empty_when_no_claims() {
        let o = page_style_overrides_from_claims(&claims());
        assert_eq!(o, PageStyleOverrides::default());
    }

    #[test]
    fn page_override_margin_shorthand_claims_all_sides() {
        let mut c = claims();
        c.margin = Some(4);
        let o = page_style_overrides_from_claims(&c);
        assert!(o.margin_top && o.margin_right && o.margin_bottom && o.margin_left);
        assert!(!o.padding_top && !o.padding_left);
    }

    #[test]
    fn page_override_mx_claims_left_and_right_only() {
        let mut c = claims();
        c.margin_x = Some(2);
        let o = page_style_overrides_from_claims(&c);
        assert!(o.margin_left && o.margin_right);
        assert!(!o.margin_top && !o.margin_bottom);
    }

    #[test]
    fn page_override_my_claims_top_and_bottom_only() {
        let mut c = claims();
        c.margin_y = Some(1);
        let o = page_style_overrides_from_claims(&c);
        assert!(o.margin_top && o.margin_bottom);
        assert!(!o.margin_left && !o.margin_right);
    }

    #[test]
    fn page_override_ml_claims_only_left_margin() {
        let mut c = claims();
        c.margin_left = Some(2);
        let o = page_style_overrides_from_claims(&c);
        assert!(o.margin_left);
        assert!(!o.margin_right && !o.margin_top && !o.margin_bottom);
    }

    #[test]
    fn page_override_padding_shorthand_claims_all_sides() {
        let mut c = claims();
        c.padding = Some(2);
        let o = page_style_overrides_from_claims(&c);
        assert!(o.padding_top && o.padding_right && o.padding_bottom && o.padding_left);
    }

    #[test]
    fn page_override_px_claims_left_and_right_padding() {
        let mut c = claims();
        c.padding_x = Some(1);
        let o = page_style_overrides_from_claims(&c);
        assert!(o.padding_left && o.padding_right);
        assert!(!o.padding_top && !o.padding_bottom);
    }

    #[test]
    fn page_override_max_width_claims_max_width() {
        let mut c = claims();
        c.max_width = Some(80);
        let o = page_style_overrides_from_claims(&c);
        assert!(o.max_width);
    }

    #[test]
    fn page_override_page_bg_claims_background() {
        let mut c = claims();
        c.page_background = Some(PageBackground::Subtle);
        let o = page_style_overrides_from_claims(&c);
        assert!(o.background);
    }

    #[test]
    fn page_override_alignment_claims_alignment() {
        let mut c = claims();
        c.alignment = Some(Alignment::Center);
        let o = page_style_overrides_from_claims(&c);
        assert!(o.alignment);
    }

    #[test]
    fn page_override_component_specific_alignment_flags_claim_their_component() {
        let mut c = claims();
        c.align_images = Some(Alignment::Left);
        c.align_lists = Some(Alignment::Right);
        c.align_block_quotes = Some(Alignment::Center);
        c.align_tables = Some(Alignment::Right);
        c.align_code_blocks = Some(Alignment::Left);
        let o = page_style_overrides_from_claims(&c);
        assert!(o.align_images && o.align_lists && o.align_block_quotes);
        assert!(o.align_tables && o.align_code_blocks);
        assert!(!o.alignment);
    }

    #[test]
    fn page_override_granular_list_alignment_flags_claim_their_component() {
        let mut c = claims();
        c.align_ul = Some(Alignment::Left);
        c.align_ol = Some(Alignment::Center);
        c.align_li = Some(Alignment::Right);
        let o = page_style_overrides_from_claims(&c);
        assert!(o.align_ul && o.align_ol && o.align_li);
        assert!(!o.align_lists);
    }

    #[test]
    fn list_override_empty_when_no_claims() {
        let o = list_style_overrides_from_claims(&claims());
        assert_eq!(o, ListStyleOverrides::default());
    }

    #[test]
    fn list_override_global_alignment_claims_all_list_alignments() {
        let mut c = claims();
        c.alignment = Some(Alignment::Center);
        let o = list_style_overrides_from_claims(&c);
        assert!(o.ul_alignment && o.ol_alignment && o.li_alignment);
    }

    #[test]
    fn list_override_align_lists_claims_all_list_alignments() {
        let mut c = claims();
        c.align_lists = Some(Alignment::Right);
        let o = list_style_overrides_from_claims(&c);
        assert!(o.ul_alignment && o.ol_alignment && o.li_alignment);
    }

    #[test]
    fn list_override_align_ul_claims_only_ul_alignment() {
        let mut c = claims();
        c.align_ul = Some(Alignment::Left);
        let o = list_style_overrides_from_claims(&c);
        assert!(o.ul_alignment);
        assert!(!o.ol_alignment && !o.li_alignment);
    }

    #[test]
    fn list_override_global_fill_claims_all_list_fills() {
        let mut c = claims();
        c.fill = Some(FillClaim::Full);
        let o = list_style_overrides_from_claims(&c);
        assert!(o.ul_fill && o.ol_fill && o.li_fill);
    }

    #[test]
    fn list_override_fill_lists_claims_all_list_fills() {
        let mut c = claims();
        c.fill_lists = Some(FillClaim::Pad(Length::ch(2)));
        let o = list_style_overrides_from_claims(&c);
        assert!(o.ul_fill && o.ol_fill && o.li_fill);
    }

    #[test]
    fn list_override_fill_ul_claims_only_ul_fill() {
        let mut c = claims();
        c.fill_ul = Some(FillClaim::Max(Length::ch(40)));
        let o = list_style_overrides_from_claims(&c);
        assert!(o.ul_fill);
        assert!(!o.ol_fill && !o.li_fill);
    }

    #[test]
    fn component_override_empty_when_no_claims() {
        let o = component_style_overrides_from_claims(&claims());
        assert_eq!(o, ComponentStyleOverrides::default());
    }

    #[test]
    fn component_override_global_alignment_claims_all_component_alignments() {
        let mut c = claims();
        c.alignment = Some(Alignment::Center);
        let o = component_style_overrides_from_claims(&c);
        assert!(
            o.tables_alignment && o.images_alignment && o.block_quotes_alignment
        );
    }

    #[test]
    fn component_override_global_fill_claims_all_component_fills() {
        let mut c = claims();
        c.fill = Some(FillClaim::Full);
        let o = component_style_overrides_from_claims(&c);
        assert!(o.tables_fill && o.images_fill && o.block_quotes_fill);
    }

    #[test]
    fn component_override_align_tables_claims_only_tables_alignment() {
        let mut c = claims();
        c.align_tables = Some(Alignment::Right);
        let o = component_style_overrides_from_claims(&c);
        assert!(o.tables_alignment);
        assert!(!o.images_alignment && !o.block_quotes_alignment);
    }

    #[test]
    fn component_override_fill_images_claims_only_images_fill() {
        let mut c = claims();
        c.fill_images = Some(FillClaim::Pad(Length::ch(2)));
        let o = component_style_overrides_from_claims(&c);
        assert!(o.images_fill);
        assert!(!o.tables_fill && !o.block_quotes_fill);
    }

    #[test]
    fn hr_and_disclosure_overrides_default_empty() {
        let hr = hr_style_overrides_from_claims(&claims());
        let disclosure = disclosure_style_overrides_from_claims(&claims());
        assert_eq!(hr, HrStyleOverrides::default());
        assert_eq!(disclosure, DisclosureStyleOverrides::default());
    }

    #[test]
    fn bespoke_override_code_theme_claim() {
        let mut c = claims();
        c.code_theme = Some(ThemePair::Dracula);
        let o = bespoke_style_overrides_from_claims(&c);
        assert!(o.code_theme);
    }

    #[test]
    fn bespoke_override_empty_when_no_code_theme() {
        let o = bespoke_style_overrides_from_claims(&claims());
        assert!(!o.code_theme);
    }

    fn edge_ch(tv: &TargetValue<Length>) -> u16 {
        match tv {
            TargetValue::Universal(Length::Ch(n)) => u16::try_from(*n).unwrap_or(u16::MAX),
            _ => 0,
        }
    }
}
