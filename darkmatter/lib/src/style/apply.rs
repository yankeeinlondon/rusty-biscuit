//! Apply parsed [`StyleFrontmatter`] onto a [`DarkmatterPage`] builder.
//!
//! This module lowers the page-level subset of the parsed style AST
//! (`style.page.*`) into the existing [`DarkmatterPage`] builder calls. It is
//! the integration point used by `darkmatter-cli` after CLI flags have already
//! been applied: `apply_page_style` honors [`PageStyleOverrides`] so that
//! frontmatter fields claimed by CLI arguments are skipped.
//!
//! ## Length lowering
//!
//! - [`Length::Zero`] → `0`
//! - [`Length::Ch(n)`] → `u16::try_from(n).unwrap_or(u16::MAX)`
//! - [`Length::Percent(p)`] → `round(p / 100 * base)` where `base` is the
//!   captured terminal width for margins/padding and the post-margin /
//!   post-padding content width for `max-width`.
//! - [`Length::Css(_)`] → [`StyleApplyError::InvalidCssLength`].

// `PageAlignment` and `PageBackground` are part of the still-current
// `DarkmatterPage` builder API even though their underlying enums are marked
// deprecated in favor of `renderable::layout::Alignment` / page-layout
// successors. The module-level allow mirrors `layout/page.rs`.
#![allow(deprecated)]

use biscuit_terminal::components::horizontal_rule::{RuleAlignment, RuleStyle, RuleWeight};
use renderable::layout::{Alignment, Length};
use thiserror::Error;

use crate::layout::{DarkmatterPage, PageAlignment, PageComponent, PageFill, WidthUnit};
use crate::style::schema::{CommonStyle, StyleFrontmatter};
use crate::style::schema::hr::{HrAlignment, HrKind, HrWeight};

/// Field-level CLI overrides for page-level style.
///
/// Each `true` value means the corresponding `style.page.*` frontmatter field
/// must be ignored because a CLI flag already claimed it. Constructed by
/// `darkmatter-cli` from the parsed CLI args using the same shorthand
/// expansion rules as `apply_cli_layout_flags` (e.g. `--margin` claims all four
/// margin sides, `--mx` claims left + right).
///
/// The component-specific alignment fields
/// (`align_images`, `align_lists`, `align_block_quotes`, `align_tables`,
/// `align_code_blocks`) record CLI claims made by the corresponding
/// `--align-*` flags. When set, the `style.page.alignment` broadcast skips
/// that component so component-specific CLI alignment is preserved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PageStyleOverrides {
    pub margin_top: bool,
    pub margin_right: bool,
    pub margin_bottom: bool,
    pub margin_left: bool,
    pub padding_top: bool,
    pub padding_right: bool,
    pub padding_bottom: bool,
    pub padding_left: bool,
    pub max_width: bool,
    pub background: bool,
    pub alignment: bool,
    pub align_images: bool,
    pub align_lists: bool,
    pub align_ul: bool,
    pub align_ol: bool,
    pub align_li: bool,
    pub align_block_quotes: bool,
    pub align_tables: bool,
    pub align_code_blocks: bool,
}

/// Field-level CLI overrides for the three non-list component buckets wired by
/// sub-spec #3 (`style.table.*`, `style.images.*`, `style.block-quote.*`).
///
/// Each `true` value means the corresponding frontmatter field must be ignored
/// because a CLI flag already claimed it. Constructed by `darkmatter-cli` from
/// the parsed CLI args using the same global-then-component-specific
/// precedence as [`PageStyleOverrides`]: global `--alignment` sets every
/// `*_alignment` field, global `--fill` sets every `*_fill` field, and
/// component-specific flags (e.g. `--align-tables`, `--fill-images`) set only
/// their own field.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ComponentStyleOverrides {
    pub tables_alignment: bool,
    pub tables_fill: bool,
    pub images_alignment: bool,
    pub images_fill: bool,
    pub block_quotes_alignment: bool,
    pub block_quotes_fill: bool,
}

impl PageStyleOverrides {
    /// Returns `true` when `component` has been claimed by a component-specific
    /// CLI alignment flag and must therefore be skipped by a
    /// `style.page.alignment` broadcast.
    fn alignment_claimed_for(&self, component: PageComponent) -> bool {
        match component {
            PageComponent::Images => self.align_images,
            PageComponent::Ul => self.align_ul || self.align_lists,
            PageComponent::Ol => self.align_ol || self.align_lists,
            PageComponent::Li => self.align_li || self.align_lists,
            PageComponent::Lists => self.align_lists,
            PageComponent::BlockQuotes => self.align_block_quotes,
            PageComponent::Tables => self.align_tables,
            PageComponent::CodeBlocks => self.align_code_blocks,
            PageComponent::Hr => false,
            PageComponent::Hyperlinks => false,
        }
    }
}

/// Field-level CLI overrides for list component style (`style.ul.*`,
/// `style.ol.*`, `style.li.*`).
///
/// Each `true` value means the corresponding frontmatter field must be ignored
/// because a CLI flag already claimed it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ListStyleOverrides {
    pub ul_alignment: bool,
    pub ul_fill: bool,
    pub ul_left_margin: bool,
    pub ol_alignment: bool,
    pub ol_fill: bool,
    pub li_alignment: bool,
    pub li_fill: bool,
}

/// Field-level CLI overrides for HR component style (`style.hr.*`).
///
/// Each `true` value means the corresponding frontmatter field must be ignored
/// because a CLI flag already claimed it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HrStyleOverrides {
    pub alignment: bool,
    pub fill: bool,
    pub color: bool,
    pub bg_color: bool,
}

/// Errors produced when lowering parsed [`StyleFrontmatter`] onto a
/// [`DarkmatterPage`].
///
/// The parser already rejects out-of-range percents and malformed unit
/// strings; these variants surface failures specific to the apply step.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum StyleApplyError {
    /// A `Length::Css(_)` value reached page-level terminal layout, where
    /// `ch`, `%`, and bare cells are the only accepted units.
    #[error("CSS length is not valid for `style.page.{field}` (terminal layout)")]
    InvalidCssLength { field: &'static str },

    /// `style.page.max-width` resolved to zero — `DarkmatterPage` treats
    /// `max_width = 0` as invalid.
    #[error("`style.page.max-width` resolved to 0 cells (must be > 0)")]
    InvalidMaxWidth,

    /// A component bucket (`table`, `images`, or `block-quote`) set both
    /// `width` and `max-width`. `DarkmatterPage` exposes a single
    /// [`PageFill`] slot per component so the two cannot coexist; future
    /// layout-storage migrations can revisit combined semantics.
    ///
    /// [`PageFill`]: crate::layout::PageFill
    #[error(
        "`style.{bucket}.width` and `style.{bucket}.max-width` are mutually exclusive"
    )]
    ComponentWidthConflict { bucket: &'static str },

    /// A [`Length::Css(_)`] value appeared in a component-level fill field
    /// (`style.{bucket}.{field}`). Component fills accept only `ch`, `%`, and
    /// bare cells; CSS lengths cannot be represented by [`WidthUnit`].
    #[error(
        "`style.{bucket}.{field}` uses CSS length which is not supported for component fill"
    )]
    ComponentInvalidCssLength {
        bucket: &'static str,
        field: &'static str,
    },

    /// A list bucket (`ul`, `ol`, or `li`) set both `width` and `max-width`.
    /// `DarkmatterPage` exposes a single [`PageFill`] slot per component so
    /// the two cannot coexist.
    #[error(
        "`style.{bucket}.width` and `style.{bucket}.max-width` are mutually exclusive"
    )]
    WidthMaxWidthConflict { bucket: &'static str },

    /// `with_list_left_margin` was called with a component other than
    /// [`PageComponent::Ul`]. Only unordered lists support the left-margin
    /// channel in this sub-spec.
    #[error("list left margin only accepts `PageComponent::Ul`")]
    InvalidListLeftMarginComponent,

    /// A local stylesheet referenced by `style.page.stylesheet` was not found
    /// on disk.
    #[error("stylesheet not found: {path}")]
    StylesheetNotFound { path: String },

    /// A local stylesheet referenced by `style.page.stylesheet` could not be
    /// read.
    #[error("failed to read stylesheet `{path}`: {reason}")]
    StylesheetRead { path: String, reason: String },

    /// The value of `style.page.stylesheet` was empty.
    #[error("empty stylesheet value")]
    EmptyStylesheet,

    /// `style.page.stylesheet` used a `file://` scheme, which is rejected in
    /// v1.
    #[error("unsupported stylesheet scheme: {scheme}")]
    UnsupportedStylesheetScheme { scheme: String },

    /// `style.page.meta` was not an object, or a nested value had an
    /// unexpected shape.
    #[error("invalid meta shape at `{path}`: {reason}")]
    InvalidMetaShape { path: String, reason: String },

    /// `style.page.code.theme` specified an unknown theme name.
    #[error("invalid code theme: `{value}`")]
    InvalidCodeTheme { value: String },
}

/// Lower a [`Length`] onto a [`WidthUnit`] for component fill application.
///
/// `bucket` and `field` name the source key (in canonical kebab-case) so a
/// rejected [`Length::Css(_)`] surfaces a precise diagnostic.
///
/// ## Returns
///
/// - [`Length::Zero`] → `WidthUnit::Fixed(0)`
/// - [`Length::Ch(n)`] → `WidthUnit::Fixed(u16)` (saturating cast)
/// - [`Length::Percent(p)`] → `WidthUnit::Percent(p)` (the parser already
///   guarantees `0.0..=100.0`; downstream `WidthUnit::resolve` validates again)
///
/// ## Errors
///
/// - [`StyleApplyError::ComponentInvalidCssLength`] when `length` is
///   [`Length::Css(_)`].
pub(crate) fn lower_length_to_fill(
    length: &Length,
    bucket: &'static str,
    field: &'static str,
) -> Result<WidthUnit, StyleApplyError> {
    match length {
        Length::Zero => Ok(WidthUnit::Fixed(0)),
        Length::Ch(n) => Ok(WidthUnit::Fixed(u16::try_from(*n).unwrap_or(u16::MAX))),
        Length::Percent(p) => Ok(WidthUnit::Percent(*p)),
        Length::Css(_) => Err(StyleApplyError::ComponentInvalidCssLength { bucket, field }),
    }
}

/// Lower a [`Length`] onto a [`WidthUnit`] for list left-margin application.
///
/// ## Returns
///
/// - [`Length::Zero`] → `WidthUnit::Fixed(0)`
/// - [`Length::Ch(n)`] → `WidthUnit::Fixed(u16)` (saturating cast)
/// - [`Length::Percent(p)`] → `WidthUnit::Percent(p)`
///
/// ## Errors
///
/// - [`StyleApplyError::ComponentInvalidCssLength`] when `length` is
///   [`Length::Css(_)`].
pub(crate) fn lower_length_to_width_unit(
    length: &Length,
    bucket: &'static str,
    field: &'static str,
) -> Result<WidthUnit, StyleApplyError> {
    match length {
        Length::Zero => Ok(WidthUnit::Fixed(0)),
        Length::Ch(n) => Ok(WidthUnit::Fixed(u16::try_from(*n).unwrap_or(u16::MAX))),
        Length::Percent(p) => Ok(WidthUnit::Percent(*p)),
        Length::Css(_) => Err(StyleApplyError::ComponentInvalidCssLength { bucket, field }),
    }
}

/// Apply parsed page-level style onto a [`DarkmatterPage`] builder.
///
/// CLI overrides suppress frontmatter for overlapping fields. The returned
/// page has active `style.page.*` settings applied. Warnings remain owned by
/// the parser; suppression is handled by the active wiring phase.
///
/// ## Errors
///
/// - [`StyleApplyError::InvalidCssLength`] when a `Length::Css(_)` value
///   appears in a page-level horizontal length field.
/// - [`StyleApplyError::InvalidMaxWidth`] when the resolved
///   `style.page.max-width` is zero.
pub fn apply_page_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: PageStyleOverrides,
) -> Result<DarkmatterPage, StyleApplyError> {
    let Some(page_style) = style.page.as_ref() else {
        return Ok(page);
    };

    let terminal_width = page.terminal_width();
    let mut page = page;

    // Resolve & apply horizontal margins (percent base = terminal width).
    if !overrides.margin_left
        && let Some(len) = page_style.left_margin.as_ref()
    {
        let value = lower_horizontal(len, terminal_width, "left-margin")?;
        page = page.with_margin_left(value);
    }
    if !overrides.margin_right
        && let Some(len) = page_style.right_margin.as_ref()
    {
        let value = lower_horizontal(len, terminal_width, "right-margin")?;
        page = page.with_margin_right(value);
    }

    // Vertical margins (already u16 row counts in the schema).
    if !overrides.margin_top
        && let Some(rows) = page_style.top_margin
    {
        page = page.with_margin_top(rows);
    }
    if !overrides.margin_bottom
        && let Some(rows) = page_style.bottom_margin
    {
        page = page.with_margin_bottom(rows);
    }

    // Horizontal padding (percent base = terminal width).
    if !overrides.padding_left
        && let Some(len) = page_style.left_padding.as_ref()
    {
        let value = lower_horizontal(len, terminal_width, "left-padding")?;
        page = page.with_padding_left(value);
    }
    if !overrides.padding_right
        && let Some(len) = page_style.right_padding.as_ref()
    {
        let value = lower_horizontal(len, terminal_width, "right-padding")?;
        page = page.with_padding_right(value);
    }

    // Vertical padding.
    if !overrides.padding_top
        && let Some(rows) = page_style.top_padding
    {
        page = page.with_padding_top(rows);
    }
    if !overrides.padding_bottom
        && let Some(rows) = page_style.bottom_padding
    {
        page = page.with_padding_bottom(rows);
    }

    // Page background.
    if !overrides.background
        && let Some(bg) = page_style.background
    {
        page = page.with_page_background(bg);
    }

    // Max width — percent resolves against the post-margin / post-padding
    // content width using the page's current state (which already reflects
    // CLI flags applied earlier in the integration order, plus any
    // frontmatter values applied above).
    if !overrides.max_width
        && let Some(len) = page_style.max_width.as_ref()
    {
        let resolved = lower_max_width(len, &page)?;
        if resolved == 0 {
            return Err(StyleApplyError::InvalidMaxWidth);
        }
        page = page.with_max_width(resolved);
    }

    // Alignment broadcasts to every page component not already claimed by a
    // component-specific CLI flag (`--align-images`, `--align-tables`, ...).
    if !overrides.alignment
        && let Some(alignment) = page_style.alignment
    {
        let mapped = map_alignment(alignment);
        for component in PageComponent::ALL {
            if !overrides.alignment_claimed_for(component) {
                page = page.use_alignment(component, mapped);
            }
        }
        #[allow(deprecated)]
        if !overrides.align_lists {
            page = page.use_alignment(PageComponent::Lists, mapped);
        }
    }

    Ok(page)
}

/// Apply parsed component-level style (`style.table.*`, `style.images.*`,
/// `style.block-quote.*`) onto a [`DarkmatterPage`] builder.
///
/// `overrides` carries the field-level CLI claims constructed by
/// `darkmatter-cli`; for any field where the corresponding bool is `true`,
/// the matching frontmatter value is skipped so the CLI flag stays in effect.
///
/// Should be called **after** [`apply_page_style`] so that component-specific
/// frontmatter alignment can override a `style.page.alignment` broadcast
/// (page broadcast is applied first as the default; component frontmatter
/// then overrides it for any component not claimed by a CLI flag).
///
/// ## Errors
///
/// - [`StyleApplyError::ComponentWidthConflict`] when both `width` and
///   `max-width` appear inside the same component bucket.
/// - [`StyleApplyError::ComponentInvalidCssLength`] when `width` or
///   `max-width` is a [`Length::Css(_)`] value.
pub fn apply_component_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: ComponentStyleOverrides,
) -> Result<DarkmatterPage, StyleApplyError> {
    let mut page = page;

    if let Some(table) = style.table.as_ref() {
        page = apply_common_style(
            page,
            "table",
            PageComponent::Tables,
            &table.common,
            overrides.tables_alignment,
            overrides.tables_fill,
        )?;
    }

    if let Some(images) = style.images.as_ref() {
        page = apply_common_style(
            page,
            "images",
            PageComponent::Images,
            &images.common,
            overrides.images_alignment,
            overrides.images_fill,
        )?;
    }

    if let Some(block_quote) = style.block_quote.as_ref() {
        page = apply_common_style(
            page,
            "block-quote",
            PageComponent::BlockQuotes,
            &block_quote.common,
            overrides.block_quotes_alignment,
            overrides.block_quotes_fill,
        )?;
    }

    Ok(page)
}

/// Apply parsed list-level style (`style.ul.*`, `style.ol.*`, `style.li.*`)
/// onto a [`DarkmatterPage`] builder.
///
/// `overrides` carries the field-level CLI claims constructed by
/// `darkmatter-cli`; for any field where the corresponding bool is `true`,
/// the matching frontmatter value is skipped so the CLI flag stays in effect.
///
/// Should be called **after** [`apply_page_style`] and [`apply_component_style`]
/// so that list-specific frontmatter can override broader broadcasts.
///
/// ## Errors
///
/// - [`StyleApplyError::WidthMaxWidthConflict`] when both `width` and
///   `max-width` appear inside the same list bucket.
/// - [`StyleApplyError::ComponentInvalidCssLength`] when `width`, `max-width`,
///   or `left-margin` is a [`Length::Css(_)`] value.
pub fn apply_list_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: ListStyleOverrides,
) -> Result<DarkmatterPage, StyleApplyError> {
    let mut page = page;

    if let Some(ul) = style.ul.as_ref() {
        page = apply_list_bucket(
            page,
            "ul",
            PageComponent::Ul,
            &ul.common,
            overrides.ul_alignment,
            overrides.ul_fill,
        )?;

        if !overrides.ul_left_margin
            && let Some(left_margin) = ul.left_margin.as_ref()
        {
            let unit = lower_length_to_width_unit(left_margin, "ul", "left-margin")?;
            page = page
                .try_with_list_left_margin(PageComponent::Ul, unit)
                .map_err(|_| StyleApplyError::InvalidListLeftMarginComponent)?;
        }
    }

    if let Some(ol) = style.ol.as_ref() {
        page = apply_list_bucket(
            page,
            "ol",
            PageComponent::Ol,
            &ol.common,
            overrides.ol_alignment,
            overrides.ol_fill,
        )?;
    }

    if let Some(li) = style.li.as_ref() {
        page = apply_list_bucket(
            page,
            "li",
            PageComponent::Li,
            &li.common,
            overrides.li_alignment,
            overrides.li_fill,
        )?;
    }

    Ok(page)
}

/// Apply parsed color and background-color style onto a [`DarkmatterPage`]
/// builder.
///
/// Color has no CLI override in this sub-spec, so frontmatter is the only
/// source. Should be called **after** [`apply_page_style`],
/// [`apply_component_style`], and [`apply_list_style`].
///
/// Page-level `color`/`bg-color` are applied first as inherited defaults;
/// component-level values then override them for the specific component.
pub fn apply_color_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
) -> Result<DarkmatterPage, StyleApplyError> {
    let mut page = page;

    // Page-level inherited colors.
    if let Some(page_style) = style.page.as_ref() {
        if let Some(color) = page_style.color.clone() {
            page = page.with_page_color(color);
        }
        if let Some(bg_color) = page_style.bg_color.clone() {
            page = page.with_page_bg_color(bg_color);
        }
    }

    // Component-level colors override page-level inheritance.
    if let Some(table) = style.table.as_ref() {
        page = apply_common_color(page, PageComponent::Tables, &table.common);
    }
    if let Some(images) = style.images.as_ref() {
        page = apply_common_color(page, PageComponent::Images, &images.common);
    }
    if let Some(block_quote) = style.block_quote.as_ref() {
        page = apply_common_color(page, PageComponent::BlockQuotes, &block_quote.common);
    }
    if let Some(hyperlinks) = style.hyperlinks.as_ref() {
        page = apply_common_color(page, PageComponent::Hyperlinks, &hyperlinks.common);
    }
    if let Some(ul) = style.ul.as_ref() {
        page = apply_common_color(page, PageComponent::Ul, &ul.common);
    }
    if let Some(ol) = style.ol.as_ref() {
        page = apply_common_color(page, PageComponent::Ol, &ol.common);
    }
    if let Some(li) = style.li.as_ref() {
        page = apply_common_color(page, PageComponent::Li, &li.common);
    }

    Ok(page)
}

/// Apply `color` and `bg-color` from a [`CommonStyle`] onto `page` for a
/// single component.
fn apply_common_color(
    page: DarkmatterPage,
    component: PageComponent,
    style: &CommonStyle,
) -> DarkmatterPage {
    let mut page = page;
    if let Some(color) = style.color.clone() {
        page = page.with_component_color(component, color);
    }
    if let Some(bg_color) = style.bg_color.clone() {
        page = page.with_component_bg_color(component, bg_color);
    }
    page
}

/// Apply parsed HR style (`style.hr.*`) onto a [`DarkmatterPage`] builder.
///
/// `overrides` carries the field-level CLI claims; for any field where the
/// corresponding bool is `true`, the matching frontmatter value is skipped so
/// the CLI flag stays in effect.
///
/// ## Errors
///
/// - [`StyleApplyError::ComponentWidthConflict`] when both `width` and
///   `max-width` appear in the `style.hr` bucket.
/// - [`StyleApplyError::ComponentInvalidCssLength`] when `width` or
///   `max-width` is a [`Length::Css(_)`] value.
pub fn apply_hr_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: HrStyleOverrides,
) -> Result<DarkmatterPage, StyleApplyError> {
    let Some(hr) = style.hr.as_ref() else {
        return Ok(page);
    };
    let mut page = page;

    // Validate exclusivity of width vs max-width unconditionally.
    if hr.width.is_some() && hr.max_width.is_some() {
        return Err(StyleApplyError::ComponentWidthConflict { bucket: "hr" });
    }

    if !overrides.fill {
        if let Some(width) = hr.width.as_ref() {
            let s = lower_length_to_hr_width(width, "hr", "width")?;
            page = page.with_hr_width(s);
        } else if let Some(max_width) = hr.max_width.as_ref() {
            let s = lower_length_to_hr_width(max_width, "hr", "max-width")?;
            page = page.with_hr_width(s);
        }
    }

    if !overrides.alignment
        && let Some(alignment) = hr.alignment
    {
        page = page.with_hr_alignment(alignment);
    }

    if let Some(kind) = hr.kind {
        page = page.with_hr_kind(kind);
    }

    if let Some(weight) = hr.weight {
        page = page.with_hr_weight(weight);
    }

    if !overrides.color
        && let Some(color) = hr.color.clone()
    {
        page = page.with_component_color(PageComponent::Hr, color);
    }

    if !overrides.bg_color
        && let Some(bg_color) = hr.bg_color.clone()
    {
        page = page.with_component_bg_color(PageComponent::Hr, bg_color);
    }

    Ok(page)
}

/// Lower a [`Length`] onto an HR width string.
///
/// ## Returns
///
/// - [`Length::Zero`] → `"0"`
/// - [`Length::Ch(n)`] → `"{n}ch"`
/// - [`Length::Percent(p)`] → `"{p}%"`
///
/// ## Errors
///
/// - [`StyleApplyError::ComponentInvalidCssLength`] when `length` is
///   [`Length::Css(_)`].
pub(crate) fn lower_length_to_hr_width(
    length: &Length,
    bucket: &'static str,
    field: &'static str,
) -> Result<String, StyleApplyError> {
    match length {
        Length::Zero => Ok("0".to_string()),
        Length::Ch(n) => Ok(format!("{}ch", n)),
        Length::Percent(p) => Ok(format!("{}%", p)),
        Length::Css(_) => Err(StyleApplyError::ComponentInvalidCssLength { bucket, field }),
    }
}

/// Map [`HrKind`] to [`RuleStyle`].
pub fn map_hr_kind(kind: HrKind) -> RuleStyle {
    match kind {
        HrKind::Dashes => RuleStyle::Dashes,
        HrKind::Dots => RuleStyle::Dots,
        HrKind::Waves => RuleStyle::Waves,
        HrKind::LineStar => RuleStyle::LineStar,
        HrKind::LineCircle => RuleStyle::LineCircle,
        HrKind::InsetLine => RuleStyle::InsetLine,
        HrKind::CurtainRod => RuleStyle::CurtainRod,
    }
}

/// Map [`HrWeight`] to [`RuleWeight`].
pub fn map_hr_weight(weight: HrWeight) -> RuleWeight {
    match weight {
        HrWeight::Thin => RuleWeight::Thin,
        HrWeight::Medium => RuleWeight::Medium,
        HrWeight::Thick => RuleWeight::Thick,
    }
}

/// Map [`HrAlignment`] to [`RuleAlignment`].
pub fn map_hr_alignment(alignment: HrAlignment) -> RuleAlignment {
    match alignment {
        HrAlignment::Full => RuleAlignment::Full,
        HrAlignment::Left => RuleAlignment::Left,
        HrAlignment::Center => RuleAlignment::Centered,
        HrAlignment::Right => RuleAlignment::Right,
    }
}

/// Apply one list bucket's [`CommonStyle`] onto `page`.
///
/// `bucket` is the canonical kebab-case label (`"ul"`, `"ol"`, `"li"`).
/// `component` is the matching [`PageComponent`] for alignment/fill storage.
fn apply_list_bucket(
    page: DarkmatterPage,
    bucket: &'static str,
    component: PageComponent,
    style: &CommonStyle,
    alignment_claimed: bool,
    fill_claimed: bool,
) -> Result<DarkmatterPage, StyleApplyError> {
    let mut page = page;

    // Validate exclusivity of width vs max-width unconditionally.
    match (style.width.as_ref(), style.max_width.as_ref()) {
        (Some(_), Some(_)) => {
            return Err(StyleApplyError::WidthMaxWidthConflict { bucket });
        }
        (Some(width), None) if !fill_claimed => {
            let unit = lower_length_to_fill(width, bucket, "width")?;
            page = page.with_fill(component, PageFill::Explicit(unit));
        }
        (None, Some(max_width)) if !fill_claimed => {
            let unit = lower_length_to_fill(max_width, bucket, "max-width")?;
            page = page.with_fill(component, PageFill::Max(unit));
        }
        _ => {}
    }

    if !alignment_claimed
        && let Some(alignment) = style.alignment
    {
        page = page.use_alignment(component, map_alignment(alignment));
    }

    Ok(page)
}

/// Apply one bucket's [`CommonStyle`] onto `page`.
///
/// `bucket` is the canonical kebab-case label (`"table"`, `"images"`,
/// `"block-quote"`) used in diagnostic messages. `component` is the matching
/// [`PageComponent`] for alignment/fill storage.
///
/// `alignment_claimed` and `fill_claimed` are the bucket's two CLI claim
/// bits from [`ComponentStyleOverrides`].
fn apply_common_style(
    page: DarkmatterPage,
    bucket: &'static str,
    component: PageComponent,
    style: &CommonStyle,
    alignment_claimed: bool,
    fill_claimed: bool,
) -> Result<DarkmatterPage, StyleApplyError> {
    let mut page = page;

    // Validate exclusivity of width vs max-width unconditionally — a CLI fill
    // claim chooses which value wins for rendering, but it never makes an
    // ambiguous frontmatter bucket valid.
    match (style.width.as_ref(), style.max_width.as_ref()) {
        (Some(_), Some(_)) => {
            return Err(StyleApplyError::ComponentWidthConflict { bucket });
        }
        (Some(width), None) if !fill_claimed => {
            let unit = lower_length_to_fill(width, bucket, "width")?;
            page = page.with_fill(component, PageFill::Explicit(unit));
        }
        (None, Some(max_width)) if !fill_claimed => {
            let unit = lower_length_to_fill(max_width, bucket, "max-width")?;
            page = page.with_fill(component, PageFill::Max(unit));
        }
        _ => {}
    }

    if !alignment_claimed
        && let Some(alignment) = style.alignment
    {
        page = page.use_alignment(component, map_alignment(alignment));
    }

    Ok(page)
}

/// Lower a horizontal [`Length`] onto a `u16` cell count using `base` as the
/// percent denominator.
fn lower_horizontal(
    length: &Length,
    base: u16,
    field: &'static str,
) -> Result<u16, StyleApplyError> {
    match length {
        Length::Zero => Ok(0),
        Length::Ch(n) => Ok(u16::try_from(*n).unwrap_or(u16::MAX)),
        Length::Percent(p) => Ok(resolve_percent(*p, base)),
        Length::Css(_) => Err(StyleApplyError::InvalidCssLength { field }),
    }
}

/// Resolve `style.page.max-width` against the content width after margins
/// and padding have been applied.
///
/// Reads the page's current margin + padding state (which already reflects
/// CLI flags and any frontmatter margins/padding applied above in
/// [`apply_page_style`]).
fn lower_max_width(length: &Length, page: &DarkmatterPage) -> Result<u16, StyleApplyError> {
    match length {
        Length::Zero => Ok(0),
        Length::Ch(n) => Ok(u16::try_from(*n).unwrap_or(u16::MAX)),
        Length::Percent(p) => {
            let terminal_width = page.terminal_width();
            let margin = page.margin();
            let padding = page.padding();
            let consumed = margin.horizontal().saturating_add(padding.horizontal());
            let content = terminal_width.saturating_sub(consumed);
            Ok(resolve_percent(*p, content))
        }
        Length::Css(_) => Err(StyleApplyError::InvalidCssLength { field: "max-width" }),
    }
}

/// Resolve a percent value against `base`, clamped to `u16` with rounding.
fn resolve_percent(p: f32, base: u16) -> u16 {
    let resolved = (f32::from(base) * (p / 100.0)).round();
    resolved.clamp(0.0, f32::from(u16::MAX)) as u16
}

fn map_alignment(alignment: Alignment) -> PageAlignment {
    match alignment {
        Alignment::Left => PageAlignment::Left,
        Alignment::Center => PageAlignment::Center,
        Alignment::Right => PageAlignment::Right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::PageBackground;
    use crate::style::schema::PageStyle;
    use biscuit_terminal::terminal::Terminal;
    use renderable::layout::{Alignment, Length};

    fn page(width: u32) -> DarkmatterPage {
        let term = Terminal::new_optimistic(width);
        DarkmatterPage::new(&term)
    }

    fn style_with_page(page_style: PageStyle) -> StyleFrontmatter {
        StyleFrontmatter {
            page: Some(page_style),
            ..StyleFrontmatter::default()
        }
    }

    #[test]
    fn no_page_block_returns_page_unchanged() {
        let p = page(80);
        let style = StyleFrontmatter::default();
        let out = apply_page_style(p.clone(), &style, PageStyleOverrides::default()).unwrap();
        assert_eq!(out.margin(), p.margin());
        assert_eq!(out.padding(), p.padding());
        assert_eq!(out.max_width(), p.max_width());
    }

    #[test]
    fn length_ch_lowers_to_u16() {
        let style = style_with_page(PageStyle {
            left_margin: Some(Length::Ch(4)),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
        assert_eq!(out.margin().left, 4);
    }

    #[test]
    fn length_zero_lowers_to_zero() {
        let style = style_with_page(PageStyle {
            left_margin: Some(Length::Zero),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
        assert_eq!(out.margin().left, 0);
    }

    #[test]
    fn length_percent_resolves_against_terminal_width_for_margin() {
        // 10% of 80 = 8.
        let style = style_with_page(PageStyle {
            left_margin: Some(Length::Percent(10.0)),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
        assert_eq!(out.margin().left, 8);
    }

    #[test]
    fn length_css_for_terminal_layout_returns_error() {
        use renderable::stylesheet::CssSizing;
        let style = style_with_page(PageStyle {
            left_margin: Some(Length::Css(CssSizing::px(10.0))),
            ..PageStyle::default()
        });
        let err = apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::InvalidCssLength {
                field: "left-margin"
            }
        );
    }

    #[test]
    fn cli_override_true_skips_frontmatter_field() {
        // Frontmatter says 4ch; override claims margin_left so it should be
        // ignored.
        let style = style_with_page(PageStyle {
            left_margin: Some(Length::Ch(4)),
            ..PageStyle::default()
        });
        let overrides = PageStyleOverrides {
            margin_left: true,
            ..PageStyleOverrides::default()
        };
        // Start with margin_left already set by some prior CLI step.
        let p = page(80).with_margin_left(7);
        let out = apply_page_style(p, &style, overrides).unwrap();
        assert_eq!(out.margin().left, 7, "CLI override should win");
    }

    #[test]
    fn cli_override_false_applies_frontmatter() {
        let style = style_with_page(PageStyle {
            left_margin: Some(Length::Ch(4)),
            ..PageStyle::default()
        });
        let overrides = PageStyleOverrides::default();
        let out = apply_page_style(page(80), &style, overrides).unwrap();
        assert_eq!(out.margin().left, 4);
    }

    #[test]
    fn vertical_margin_row_count_applied() {
        let style = style_with_page(PageStyle {
            top_margin: Some(1),
            bottom_margin: Some(0),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
        assert_eq!(out.margin().top, 1);
        assert_eq!(out.margin().bottom, 0);
    }

    #[test]
    fn percent_max_width_resolves_post_margin_and_padding() {
        // 10% left margin + 10% right margin of 100 = 10 + 10 = 20.
        // Content width = 100 - 20 = 80. 50% of 80 = 40.
        let style = style_with_page(PageStyle {
            left_margin: Some(Length::Percent(10.0)),
            right_margin: Some(Length::Percent(10.0)),
            max_width: Some(Length::Percent(50.0)),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(100), &style, PageStyleOverrides::default()).unwrap();
        assert_eq!(out.max_width(), Some(40));
    }

    #[test]
    fn percent_max_width_resolved_to_zero_returns_error() {
        // Terminal too narrow once margins/padding consume everything.
        let p = page(10).with_margin_x(5);
        let style = style_with_page(PageStyle {
            max_width: Some(Length::Percent(50.0)),
            ..PageStyle::default()
        });
        let err = apply_page_style(p, &style, PageStyleOverrides::default()).unwrap_err();
        assert_eq!(err, StyleApplyError::InvalidMaxWidth);
    }

    #[test]
    fn page_alignment_broadcast_skips_components_claimed_by_cli() {
        // Component-specific CLI alignment (`--align-tables right`) must
        // survive a `style.page.alignment: center` broadcast — the broadcast
        // is a default for unclaimed components, not an override.
        use crate::layout::PageComponent;
        let style = style_with_page(PageStyle {
            alignment: Some(Alignment::Center),
            ..PageStyle::default()
        });
        let overrides = PageStyleOverrides {
            align_tables: true,
            ..PageStyleOverrides::default()
        };
        // Simulate the CLI having applied `--align-tables right` before us.
        let starting = page(80).use_alignment(PageComponent::Tables, PageAlignment::Right);
        let out = apply_page_style(starting, &style, overrides).unwrap();
        assert_eq!(
            out.alignment_for(PageComponent::Tables),
            PageAlignment::Right,
            "component-specific CLI alignment must survive page broadcast",
        );
        // Unclaimed components still receive the page default.
        for component in [
            PageComponent::Images,
            PageComponent::Lists,
            PageComponent::BlockQuotes,
            PageComponent::CodeBlocks,
        ] {
            assert_eq!(
                out.alignment_for(component),
                PageAlignment::Center,
                "unclaimed component should adopt page broadcast: {:?}",
                component,
            );
        }
    }

    #[test]
    fn page_alignment_broadcasts_to_all_components() {
        use crate::layout::PageComponent;
        let style = style_with_page(PageStyle {
            alignment: Some(Alignment::Center),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
        for component in PageComponent::ALL {
            assert_eq!(
                out.alignment_for(component),
                PageAlignment::Center,
                "alignment should broadcast to {:?}",
                component
            );
        }
    }

    #[test]
    fn background_applies_when_not_overridden() {
        let style = style_with_page(PageStyle {
            background: Some(PageBackground::Subtle),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
        assert_eq!(out.page_background(), PageBackground::Subtle);
    }

    #[test]
    fn background_override_skips_frontmatter() {
        let style = style_with_page(PageStyle {
            background: Some(PageBackground::Subtle),
            ..PageStyle::default()
        });
        let overrides = PageStyleOverrides {
            background: true,
            ..PageStyleOverrides::default()
        };
        let out = apply_page_style(page(80), &style, overrides).unwrap();
        // Original page_background was Transparent (default).
        assert_eq!(out.page_background(), PageBackground::Transparent);
    }

    #[test]
    fn lower_length_to_fill_zero_maps_to_fixed_zero() {
        let unit = lower_length_to_fill(&Length::Zero, "table", "width").unwrap();
        assert_eq!(unit, WidthUnit::Fixed(0));
    }

    #[test]
    fn lower_length_to_fill_ch_maps_to_fixed() {
        let unit = lower_length_to_fill(&Length::Ch(30), "table", "width").unwrap();
        assert_eq!(unit, WidthUnit::Fixed(30));
    }

    #[test]
    fn lower_length_to_fill_ch_saturates_when_oversized() {
        // u32::MAX is larger than u16::MAX, so the saturating cast must clamp
        // to u16::MAX rather than wrap.
        let unit = lower_length_to_fill(&Length::Ch(u32::MAX), "images", "max-width").unwrap();
        assert_eq!(unit, WidthUnit::Fixed(u16::MAX));
    }

    #[test]
    fn lower_length_to_fill_ch_at_u16_boundary() {
        let unit = lower_length_to_fill(&Length::Ch(u32::from(u16::MAX)), "table", "width")
            .unwrap();
        assert_eq!(unit, WidthUnit::Fixed(u16::MAX));
    }

    #[test]
    fn lower_length_to_fill_percent_maps_to_percent() {
        let unit = lower_length_to_fill(&Length::Percent(50.0), "table", "max-width").unwrap();
        assert_eq!(unit, WidthUnit::Percent(50.0));
    }

    #[test]
    fn lower_length_to_fill_percent_preserves_fractional() {
        let unit = lower_length_to_fill(&Length::Percent(33.5), "images", "width").unwrap();
        assert_eq!(unit, WidthUnit::Percent(33.5));
    }

    #[test]
    fn lower_length_to_fill_css_returns_component_error() {
        use renderable::stylesheet::CssSizing;
        let err =
            lower_length_to_fill(&Length::Css(CssSizing::px(10.0)), "block-quote", "max-width")
                .unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentInvalidCssLength {
                bucket: "block-quote",
                field: "max-width",
            }
        );
    }

    #[test]
    fn lower_length_to_fill_css_uses_kebab_case_bucket_in_error() {
        use renderable::stylesheet::CssSizing;
        // The bucket label propagates verbatim, so callers must pass the
        // canonical kebab-case spelling (`block-quote`, not `block_quote`).
        let err = lower_length_to_fill(&Length::Css(CssSizing::px(1.0)), "block-quote", "width")
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("`style.block-quote.width`"),
            "expected kebab-case bucket in message, got: {msg}"
        );
    }

    #[test]
    fn padding_lowers_horizontal_and_vertical() {
        let style = style_with_page(PageStyle {
            left_padding: Some(Length::Ch(3)),
            right_padding: Some(Length::Ch(3)),
            top_padding: Some(1),
            bottom_padding: Some(1),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
        assert_eq!(out.padding().left, 3);
        assert_eq!(out.padding().right, 3);
        assert_eq!(out.padding().top, 1);
        assert_eq!(out.padding().bottom, 1);
    }

    // ----- apply_component_style -----

    use crate::style::schema::{BlockQuoteStyle, CommonStyle, ImageStyle, TableStyle};

    fn style_with_table(common: CommonStyle) -> StyleFrontmatter {
        StyleFrontmatter {
            table: Some(TableStyle { common }),
            ..StyleFrontmatter::default()
        }
    }

    fn style_with_images(common: CommonStyle) -> StyleFrontmatter {
        StyleFrontmatter {
            images: Some(ImageStyle {
                common,
                local_style: None,
            }),
            ..StyleFrontmatter::default()
        }
    }

    fn style_with_block_quote(common: CommonStyle) -> StyleFrontmatter {
        StyleFrontmatter {
            block_quote: Some(BlockQuoteStyle { common }),
            ..StyleFrontmatter::default()
        }
    }

    #[test]
    fn component_style_empty_returns_page_unchanged() {
        let p = page(80);
        let style = StyleFrontmatter::default();
        let out =
            apply_component_style(p.clone(), &style, ComponentStyleOverrides::default()).unwrap();
        for component in PageComponent::ALL {
            assert_eq!(out.alignment_for(component), p.alignment_for(component));
            assert_eq!(out.fill_for(component), p.fill_for(component));
        }
    }

    #[test]
    fn table_alignment_applied() {
        let style = style_with_table(CommonStyle {
            alignment: Some(Alignment::Right),
            ..CommonStyle::default()
        });
        let out =
            apply_component_style(page(80), &style, ComponentStyleOverrides::default()).unwrap();
        assert_eq!(out.alignment_for(PageComponent::Tables), PageAlignment::Right);
    }

    #[test]
    fn table_max_width_applied() {
        let style = style_with_table(CommonStyle {
            max_width: Some(Length::Percent(50.0)),
            ..CommonStyle::default()
        });
        let out =
            apply_component_style(page(80), &style, ComponentStyleOverrides::default()).unwrap();
        assert_eq!(
            out.fill_for(PageComponent::Tables),
            PageFill::Max(WidthUnit::Percent(50.0))
        );
    }

    #[test]
    fn table_width_applied() {
        let style = style_with_table(CommonStyle {
            width: Some(Length::Ch(40)),
            ..CommonStyle::default()
        });
        let out =
            apply_component_style(page(80), &style, ComponentStyleOverrides::default()).unwrap();
        assert_eq!(
            out.fill_for(PageComponent::Tables),
            PageFill::Explicit(WidthUnit::Fixed(40))
        );
    }

    #[test]
    fn image_alignment_and_fill_applied() {
        let style = style_with_images(CommonStyle {
            alignment: Some(Alignment::Center),
            max_width: Some(Length::Ch(60)),
            ..CommonStyle::default()
        });
        let out =
            apply_component_style(page(80), &style, ComponentStyleOverrides::default()).unwrap();
        assert_eq!(
            out.alignment_for(PageComponent::Images),
            PageAlignment::Center
        );
        assert_eq!(
            out.fill_for(PageComponent::Images),
            PageFill::Max(WidthUnit::Fixed(60))
        );
    }

    #[test]
    fn block_quote_max_width_applied() {
        let style = style_with_block_quote(CommonStyle {
            max_width: Some(Length::Percent(75.0)),
            ..CommonStyle::default()
        });
        let out =
            apply_component_style(page(80), &style, ComponentStyleOverrides::default()).unwrap();
        assert_eq!(
            out.fill_for(PageComponent::BlockQuotes),
            PageFill::Max(WidthUnit::Percent(75.0))
        );
    }

    #[test]
    fn width_and_max_width_together_rejected() {
        let style = style_with_table(CommonStyle {
            width: Some(Length::Ch(40)),
            max_width: Some(Length::Percent(50.0)),
            ..CommonStyle::default()
        });
        let err =
            apply_component_style(page(80), &style, ComponentStyleOverrides::default()).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentWidthConflict { bucket: "table" }
        );
    }

    #[test]
    fn block_quote_width_conflict_uses_kebab_case_bucket() {
        let style = style_with_block_quote(CommonStyle {
            width: Some(Length::Ch(40)),
            max_width: Some(Length::Ch(60)),
            ..CommonStyle::default()
        });
        let err =
            apply_component_style(page(80), &style, ComponentStyleOverrides::default()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("`style.block-quote.width`"),
            "expected kebab-case bucket in message, got: {msg}"
        );
    }

    #[test]
    fn css_length_in_component_fill_rejected() {
        use renderable::stylesheet::CssSizing;
        let style = style_with_images(CommonStyle {
            max_width: Some(Length::Css(CssSizing::px(120.0))),
            ..CommonStyle::default()
        });
        let err =
            apply_component_style(page(80), &style, ComponentStyleOverrides::default()).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentInvalidCssLength {
                bucket: "images",
                field: "max-width",
            }
        );
    }

    #[test]
    fn cli_alignment_override_suppresses_frontmatter() {
        let style = style_with_table(CommonStyle {
            alignment: Some(Alignment::Right),
            ..CommonStyle::default()
        });
        let overrides = ComponentStyleOverrides {
            tables_alignment: true,
            ..ComponentStyleOverrides::default()
        };
        // Simulate CLI having already applied `--align-tables left` earlier.
        let starting = page(80).use_alignment(PageComponent::Tables, PageAlignment::Left);
        let out = apply_component_style(starting, &style, overrides).unwrap();
        assert_eq!(
            out.alignment_for(PageComponent::Tables),
            PageAlignment::Left,
            "CLI alignment claim should suppress frontmatter alignment",
        );
    }

    #[test]
    fn cli_fill_override_does_not_mask_frontmatter_width_conflict() {
        // Regression: a document with both `width` and `max-width` in the same
        // component bucket is always invalid, even when the user passes a CLI
        // fill flag that would otherwise claim the bucket. The CLI overrides
        // the rendered value but does not paper over ambiguous frontmatter.
        let style = style_with_table(CommonStyle {
            width: Some(Length::Ch(40)),
            max_width: Some(Length::Percent(50.0)),
            ..CommonStyle::default()
        });
        let overrides = ComponentStyleOverrides {
            tables_fill: true,
            ..ComponentStyleOverrides::default()
        };
        let err = apply_component_style(page(80), &style, overrides).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentWidthConflict { bucket: "table" }
        );
    }

    #[test]
    fn cli_fill_images_override_does_not_mask_frontmatter_width_conflict() {
        // Same regression as `cli_fill_override_does_not_mask_frontmatter_width_conflict`
        // but for the component-specific `--fill-images` claim path.
        let style = style_with_images(CommonStyle {
            width: Some(Length::Ch(30)),
            max_width: Some(Length::Ch(40)),
            ..CommonStyle::default()
        });
        let overrides = ComponentStyleOverrides {
            images_fill: true,
            ..ComponentStyleOverrides::default()
        };
        let err = apply_component_style(page(80), &style, overrides).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentWidthConflict { bucket: "images" }
        );
    }

    #[test]
    fn cli_fill_override_suppresses_frontmatter() {
        let style = style_with_images(CommonStyle {
            max_width: Some(Length::Percent(50.0)),
            ..CommonStyle::default()
        });
        let overrides = ComponentStyleOverrides {
            images_fill: true,
            ..ComponentStyleOverrides::default()
        };
        // Simulate CLI having already applied a fill earlier.
        let starting =
            page(80).with_fill(PageComponent::Images, PageFill::Max(WidthUnit::Fixed(30)));
        let out = apply_component_style(starting, &style, overrides).unwrap();
        assert_eq!(
            out.fill_for(PageComponent::Images),
            PageFill::Max(WidthUnit::Fixed(30)),
            "CLI fill claim should suppress frontmatter width / max-width",
        );
    }

    #[test]
    fn component_alignment_overrides_page_broadcast() {
        // Simulate the integration order: `apply_page_style` ran first and
        // broadcast `center` to every component, then `apply_component_style`
        // sees `style.table.alignment: right` with no CLI claim and must
        // override the broadcast for tables only.
        let page_style = PageStyle {
            alignment: Some(Alignment::Center),
            ..PageStyle::default()
        };
        let mut style = style_with_table(CommonStyle {
            alignment: Some(Alignment::Right),
            ..CommonStyle::default()
        });
        style.page = Some(page_style);

        let p = apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
        let out = apply_component_style(p, &style, ComponentStyleOverrides::default()).unwrap();

        assert_eq!(
            out.alignment_for(PageComponent::Tables),
            PageAlignment::Right,
            "component frontmatter should override page broadcast",
        );
        for component in [
            PageComponent::Images,
            PageComponent::Lists,
            PageComponent::BlockQuotes,
            PageComponent::CodeBlocks,
        ] {
            assert_eq!(
                out.alignment_for(component),
                PageAlignment::Center,
                "untouched components keep page broadcast: {component:?}",
            );
        }
    }

    #[test]
    fn page_broadcast_wins_when_cli_claimed_component_alignment() {
        // If the CLI claimed `--align-tables`, `apply_page_style` already
        // skipped the broadcast for tables; `apply_component_style` must also
        // honor the claim and skip the frontmatter alignment, leaving the
        // pre-existing CLI value intact.
        let page_style = PageStyle {
            alignment: Some(Alignment::Center),
            ..PageStyle::default()
        };
        let mut style = style_with_table(CommonStyle {
            alignment: Some(Alignment::Right),
            ..CommonStyle::default()
        });
        style.page = Some(page_style);
        let page_overrides = PageStyleOverrides {
            align_tables: true,
            ..PageStyleOverrides::default()
        };
        let component_overrides = ComponentStyleOverrides {
            tables_alignment: true,
            ..ComponentStyleOverrides::default()
        };

        let starting = page(80).use_alignment(PageComponent::Tables, PageAlignment::Left);
        let p = apply_page_style(starting, &style, page_overrides).unwrap();
        let out = apply_component_style(p, &style, component_overrides).unwrap();

        assert_eq!(
            out.alignment_for(PageComponent::Tables),
            PageAlignment::Left,
            "CLI alignment must survive both broadcast and component frontmatter",
        );
    }

    #[test]
    fn all_three_buckets_applied_independently() {
        let mut style = style_with_table(CommonStyle {
            alignment: Some(Alignment::Right),
            max_width: Some(Length::Percent(50.0)),
            ..CommonStyle::default()
        });
        style.images = Some(ImageStyle {
            common: CommonStyle {
                alignment: Some(Alignment::Center),
                width: Some(Length::Ch(40)),
                ..CommonStyle::default()
            },
            local_style: None,
        });
        style.block_quote = Some(BlockQuoteStyle {
            common: CommonStyle {
                max_width: Some(Length::Ch(60)),
                ..CommonStyle::default()
            },
        });

        let out =
            apply_component_style(page(80), &style, ComponentStyleOverrides::default()).unwrap();

        assert_eq!(
            out.alignment_for(PageComponent::Tables),
            PageAlignment::Right
        );
        assert_eq!(
            out.fill_for(PageComponent::Tables),
            PageFill::Max(WidthUnit::Percent(50.0))
        );
        assert_eq!(
            out.alignment_for(PageComponent::Images),
            PageAlignment::Center
        );
        assert_eq!(
            out.fill_for(PageComponent::Images),
            PageFill::Explicit(WidthUnit::Fixed(40))
        );
        assert_eq!(
            out.fill_for(PageComponent::BlockQuotes),
            PageFill::Max(WidthUnit::Fixed(60))
        );
    }

    // ----- apply_list_style -----

    use crate::style::schema::{LiStyle, OlStyle, UlStyle};

    fn style_with_ul(common: CommonStyle, left_margin: Option<Length>) -> StyleFrontmatter {
        StyleFrontmatter {
            ul: Some(UlStyle { common, left_margin }),
            ..StyleFrontmatter::default()
        }
    }

    fn style_with_ol(common: CommonStyle) -> StyleFrontmatter {
        StyleFrontmatter {
            ol: Some(OlStyle { common }),
            ..StyleFrontmatter::default()
        }
    }

    fn style_with_li(common: CommonStyle) -> StyleFrontmatter {
        StyleFrontmatter {
            li: Some(LiStyle { common }),
            ..StyleFrontmatter::default()
        }
    }

    #[test]
    fn ul_left_margin_applied() {
        let style = style_with_ul(
            CommonStyle::default(),
            Some(Length::Ch(4)),
        );
        let out = apply_list_style(page(80), &style, ListStyleOverrides::default()).unwrap();
        assert_eq!(
            out.list_left_margin_for(PageComponent::Ul),
            Some(WidthUnit::Fixed(4))
        );
    }

    #[test]
    fn ol_alignment_applied() {
        let style = style_with_ol(CommonStyle {
            alignment: Some(Alignment::Right),
            ..CommonStyle::default()
        });
        let out = apply_list_style(page(80), &style, ListStyleOverrides::default()).unwrap();
        assert_eq!(
            out.alignment_for(PageComponent::Ol),
            PageAlignment::Right
        );
    }

    #[test]
    fn li_width_applied() {
        let style = style_with_li(CommonStyle {
            width: Some(Length::Ch(30)),
            ..CommonStyle::default()
        });
        let out = apply_list_style(page(80), &style, ListStyleOverrides::default()).unwrap();
        assert_eq!(
            out.fill_for(PageComponent::Li),
            PageFill::Explicit(WidthUnit::Fixed(30))
        );
    }

    #[test]
    fn ul_width_max_width_conflict() {
        let style = style_with_ul(
            CommonStyle {
                width: Some(Length::Ch(40)),
                max_width: Some(Length::Ch(60)),
                ..CommonStyle::default()
            },
            None,
        );
        let err = apply_list_style(page(80), &style, ListStyleOverrides::default()).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::WidthMaxWidthConflict { bucket: "ul" }
        );
    }

    #[test]
    fn ol_width_max_width_conflict() {
        let style = style_with_ol(CommonStyle {
            width: Some(Length::Ch(40)),
            max_width: Some(Length::Ch(60)),
            ..CommonStyle::default()
        });
        let err = apply_list_style(page(80), &style, ListStyleOverrides::default()).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::WidthMaxWidthConflict { bucket: "ol" }
        );
    }

    #[test]
    fn li_width_max_width_conflict() {
        let style = style_with_li(CommonStyle {
            width: Some(Length::Ch(40)),
            max_width: Some(Length::Ch(60)),
            ..CommonStyle::default()
        });
        let err = apply_list_style(page(80), &style, ListStyleOverrides::default()).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::WidthMaxWidthConflict { bucket: "li" }
        );
    }

    #[test]
    fn ul_left_margin_css_rejected() {
        use renderable::stylesheet::CssSizing;
        let style = style_with_ul(
            CommonStyle::default(),
            Some(Length::Css(CssSizing::px(10.0))),
        );
        let err = apply_list_style(page(80), &style, ListStyleOverrides::default()).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentInvalidCssLength {
                bucket: "ul",
                field: "left-margin",
            }
        );
    }

    #[test]
    fn overrides_suppress_frontmatter() {
        let style = style_with_ul(
            CommonStyle {
                alignment: Some(Alignment::Right),
                width: Some(Length::Ch(40)),
                ..CommonStyle::default()
            },
            Some(Length::Ch(4)),
        );
        let overrides = ListStyleOverrides {
            ul_alignment: true,
            ul_fill: true,
            ul_left_margin: true,
            ..ListStyleOverrides::default()
        };
        let starting = page(80)
            .use_alignment(PageComponent::Ul, PageAlignment::Left)
            .with_fill(PageComponent::Ul, PageFill::Max(WidthUnit::Fixed(30)));
        let out = apply_list_style(starting, &style, overrides).unwrap();
        assert_eq!(out.alignment_for(PageComponent::Ul), PageAlignment::Left);
        assert_eq!(
            out.fill_for(PageComponent::Ul),
            PageFill::Max(WidthUnit::Fixed(30))
        );
        assert_eq!(out.list_left_margin_for(PageComponent::Ul), None);
    }

    // ----- apply_hr_style -----

    use crate::style::schema::hr::{HrAlignment, HrKind, HrWeight, HrStyle};
    use biscuit_terminal::components::horizontal_rule::{RuleAlignment, RuleStyle, RuleWeight};

    fn style_with_hr(hr: HrStyle) -> StyleFrontmatter {
        StyleFrontmatter {
            hr: Some(hr),
            ..StyleFrontmatter::default()
        }
    }

    #[test]
    fn hr_style_empty_returns_page_unchanged() {
        let p = page(80);
        let style = StyleFrontmatter::default();
        let out = apply_hr_style(p.clone(), &style, HrStyleOverrides::default()).unwrap();
        assert_eq!(out.hr_kind(), None);
        assert_eq!(out.hr_weight(), None);
        assert_eq!(out.hr_alignment(), None);
        assert_eq!(out.hr_width(), None);
    }

    #[test]
    fn hr_kind_applied() {
        let style = style_with_hr(HrStyle {
            kind: Some(HrKind::Waves),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style, HrStyleOverrides::default()).unwrap();
        assert_eq!(out.hr_kind(), Some(HrKind::Waves));
    }

    #[test]
    fn hr_weight_applied() {
        let style = style_with_hr(HrStyle {
            weight: Some(HrWeight::Thick),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style, HrStyleOverrides::default()).unwrap();
        assert_eq!(out.hr_weight(), Some(HrWeight::Thick));
    }

    #[test]
    fn hr_alignment_applied() {
        let style = style_with_hr(HrStyle {
            alignment: Some(HrAlignment::Center),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style, HrStyleOverrides::default()).unwrap();
        assert_eq!(out.hr_alignment(), Some(HrAlignment::Center));
    }

    #[test]
    fn hr_width_applied() {
        let style = style_with_hr(HrStyle {
            width: Some(Length::Percent(50.0)),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style, HrStyleOverrides::default()).unwrap();
        assert_eq!(out.hr_width(), Some("50%"));
    }

    #[test]
    fn hr_max_width_applied() {
        let style = style_with_hr(HrStyle {
            max_width: Some(Length::Ch(40)),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style, HrStyleOverrides::default()).unwrap();
        assert_eq!(out.hr_width(), Some("40ch"));
    }

    #[test]
    fn hr_width_and_max_width_conflict() {
        let style = style_with_hr(HrStyle {
            width: Some(Length::Ch(40)),
            max_width: Some(Length::Percent(50.0)),
            ..HrStyle::default()
        });
        let err = apply_hr_style(page(80), &style, HrStyleOverrides::default()).unwrap_err();
        assert_eq!(err, StyleApplyError::ComponentWidthConflict { bucket: "hr" });
    }

    #[test]
    fn hr_color_applied() {
        use crate::style::color::StyleColor;
        use renderable::color::{Color, Tailwind};
        let color = StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: None,
        };
        let style = style_with_hr(HrStyle {
            color: Some(color.clone()),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style, HrStyleOverrides::default()).unwrap();
        assert_eq!(
            out.color_for(PageComponent::Hr),
            Some(&color)
        );
    }

    #[test]
    fn hr_bg_color_applied() {
        use crate::style::color::StyleColor;
        use renderable::color::{Color, Tailwind};
        let color = StyleColor {
            color: Color::Tailwind(Tailwind::Blue500),
            opacity: None,
        };
        let style = style_with_hr(HrStyle {
            bg_color: Some(color.clone()),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style, HrStyleOverrides::default()).unwrap();
        assert_eq!(
            out.bg_color_for(PageComponent::Hr),
            Some(&color)
        );
    }

    #[test]
    fn hr_css_length_rejected() {
        use renderable::stylesheet::CssSizing;
        let style = style_with_hr(HrStyle {
            width: Some(Length::Css(CssSizing::px(10.0))),
            ..HrStyle::default()
        });
        let err = apply_hr_style(page(80), &style, HrStyleOverrides::default()).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentInvalidCssLength {
                bucket: "hr",
                field: "width",
            }
        );
    }

    #[test]
    fn hr_overrides_suppress_frontmatter() {
        let style = style_with_hr(HrStyle {
            alignment: Some(HrAlignment::Right),
            width: Some(Length::Ch(40)),
            color: Some(crate::style::color::StyleColor {
                color: renderable::color::Color::Tailwind(renderable::color::Tailwind::Red500),
                opacity: None,
            }),
            bg_color: Some(crate::style::color::StyleColor {
                color: renderable::color::Color::Tailwind(renderable::color::Tailwind::Blue500),
                opacity: None,
            }),
            ..HrStyle::default()
        });
        let overrides = HrStyleOverrides {
            alignment: true,
            fill: true,
            color: true,
            bg_color: true,
        };
        let starting = page(80)
            .with_hr_alignment(HrAlignment::Left)
            .with_hr_width("30ch")
            .with_component_color(PageComponent::Hr, crate::style::color::StyleColor {
                color: renderable::color::Color::Tailwind(renderable::color::Tailwind::Green500),
                opacity: None,
            })
            .with_component_bg_color(PageComponent::Hr, crate::style::color::StyleColor {
                color: renderable::color::Color::Tailwind(renderable::color::Tailwind::Yellow500),
                opacity: None,
            });
        let out = apply_hr_style(starting, &style, overrides).unwrap();
        assert_eq!(out.hr_alignment(), Some(HrAlignment::Left));
        assert_eq!(out.hr_width(), Some("30ch"));
        assert_eq!(
            out.color_for(PageComponent::Hr),
            Some(&crate::style::color::StyleColor {
                color: renderable::color::Color::Tailwind(renderable::color::Tailwind::Green500),
                opacity: None,
            })
        );
        assert_eq!(
            out.bg_color_for(PageComponent::Hr),
            Some(&crate::style::color::StyleColor {
                color: renderable::color::Color::Tailwind(renderable::color::Tailwind::Yellow500),
                opacity: None,
            })
        );
    }

    #[test]
    fn map_hr_kind_variants() {
        assert_eq!(map_hr_kind(HrKind::Dashes), RuleStyle::Dashes);
        assert_eq!(map_hr_kind(HrKind::Dots), RuleStyle::Dots);
        assert_eq!(map_hr_kind(HrKind::Waves), RuleStyle::Waves);
        assert_eq!(map_hr_kind(HrKind::LineStar), RuleStyle::LineStar);
        assert_eq!(map_hr_kind(HrKind::LineCircle), RuleStyle::LineCircle);
        assert_eq!(map_hr_kind(HrKind::InsetLine), RuleStyle::InsetLine);
        assert_eq!(map_hr_kind(HrKind::CurtainRod), RuleStyle::CurtainRod);
    }

    #[test]
    fn map_hr_weight_variants() {
        assert_eq!(map_hr_weight(HrWeight::Thin), RuleWeight::Thin);
        assert_eq!(map_hr_weight(HrWeight::Medium), RuleWeight::Medium);
        assert_eq!(map_hr_weight(HrWeight::Thick), RuleWeight::Thick);
    }

    #[test]
    fn map_hr_alignment_variants() {
        assert_eq!(map_hr_alignment(HrAlignment::Full), RuleAlignment::Full);
        assert_eq!(map_hr_alignment(HrAlignment::Left), RuleAlignment::Left);
        assert_eq!(map_hr_alignment(HrAlignment::Center), RuleAlignment::Centered);
        assert_eq!(map_hr_alignment(HrAlignment::Right), RuleAlignment::Right);
    }
}
