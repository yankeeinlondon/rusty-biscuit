//! Apply parsed [`StyleFrontmatter`] onto a [`DarkmatterPage`] builder.
//!
//! This module lowers the parsed style AST into [`DarkmatterPage`] builder
//! calls. The style it receives is already effective: precedence between the
//! document, style files, and setters was decided field by field when the
//! layers were merged by [`prepare_style_layers`](super::prepare_style_layers),
//! so every `apply_*` function applies each present field unconditionally.
//!
//! ## Length lowering
//!
//! - [`Length::Zero`] → `0`
//! - [`Length::Ch(n)`] → `u16::try_from(n).unwrap_or(u16::MAX)`
//! - [`Length::Percent(p)`] → `round(p / 100 * base)` where `base` is the
//!   captured terminal width for margins/padding and the post-margin /
//!   post-padding content width for `max-width`.
//! - [`Length::Css(_)`] → [`StyleApplyError::InvalidCssLength`].

use std::sync::Arc;

use biscuit_file::FileReferenceError;
use biscuit_terminal::components::horizontal_rule::{RuleAlignment, RuleStyle, RuleWeight};
use renderable::layout::{Length, TargetValue, Width};
use thiserror::Error;

use crate::layout::{DarkmatterPage, PageComponent};
use crate::style::bespoke::{StyleAssetResolver, apply_bespoke_knobs, apply_stylesheet};
use crate::style::layer::PreparedStyle;
use crate::style::presentation::PresentationDefaults;
use crate::style::schema::{CommonStyle, ComponentEdges, StyleFrontmatter};
#[cfg(test)]
use crate::style::schema::WidthOrMode;
use crate::style::schema::hr::{HrAlignment, HrKind, HrWeight};

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
    /// `width` and `max-width`. `DarkmatterPage` stores both as separate
    /// `renderable::layout::Layout` fields, but the v1 schema treats them as
    /// mutually exclusive within one bucket so one field decides the width.
    #[error(
        "`style.{bucket}.width` and `style.{bucket}.max-width` are mutually exclusive"
    )]
    ComponentWidthConflict { bucket: &'static str },

    /// A [`Length::Css(_)`] value appeared in a component-level length field
    /// (`style.{bucket}.{field}`). Component lengths accept only `ch`, `%`, and
    /// bare cells; CSS lengths are rejected here.
    #[error(
        "`style.{bucket}.{field}` uses CSS length which is not supported for component layout"
    )]
    ComponentInvalidCssLength {
        bucket: &'static str,
        field: &'static str,
    },

    /// A list bucket (`ul`, `ol`, or `li`) set both `width` and `max-width`.
    /// `DarkmatterPage` exposes a single fill slot per component so
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

    /// `style.page.stylesheet` is not a valid file reference, or resolving
    /// it failed (an escape from its tree, a missing anchor, an I/O probe
    /// failure). The [`FileReferenceError`] is the error's source and
    /// [`StyleApplyError::file_reference_error`].
    #[error("could not resolve stylesheet `{reference}`: {error}")]
    StylesheetReference {
        reference: String,
        #[source]
        error: SharedFileReferenceError,
    },

    /// A `style.page.stylesheet` has no recorded origin, so there is no
    /// context to resolve it in.
    #[error("no file context to resolve stylesheet `{reference}` in")]
    StylesheetContextMissing { reference: String },

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

}

impl StyleApplyError {
    pub(crate) fn stylesheet_reference(reference: &str, error: FileReferenceError) -> Self {
        Self::StylesheetReference {
            reference: reference.to_string(),
            error: SharedFileReferenceError(Arc::new(error)),
        }
    }

    /// The file-reference failure behind a stylesheet error, for a diagnostic
    /// that names its class (`ResolutionFailure`).
    pub fn file_reference_error(&self) -> Option<&FileReferenceError> {
        match self {
            Self::StylesheetReference { error, .. } => Some(&error.0),
            _ => None,
        }
    }
}

/// A [`FileReferenceError`] shared so [`StyleApplyError`] stays `Clone` and
/// comparable; two are equal when they say the same thing.
#[derive(Debug, Clone)]
pub struct SharedFileReferenceError(pub Arc<FileReferenceError>);

impl PartialEq for SharedFileReferenceError {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.to_string() == other.0.to_string()
    }
}

impl std::fmt::Display for SharedFileReferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// Transparent: the wrapper displays as its error, so it reports that error's
// own source rather than repeating it in an error chain.
impl std::error::Error for SharedFileReferenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// Lower a [`Length`] onto a [`renderable::layout::Length`] for component
/// layout application.
///
/// ## Returns
///
/// - [`Length::Zero`] / [`Length::Ch`] / [`Length::Percent`] → cloned
///
/// ## Errors
///
/// - [`StyleApplyError::ComponentInvalidCssLength`] when `length` is
///   [`Length::Css(_)`].
fn length_to_layout_length(
    length: &Length,
    bucket: &'static str,
    field: &'static str,
) -> Result<Length, StyleApplyError> {
    match length {
        Length::Zero | Length::Ch(_) | Length::Percent(_) => Ok(length.clone()),
        Length::Css(_) => Err(StyleApplyError::ComponentInvalidCssLength { bucket, field }),
    }
}

/// Lower a bucket's `padding`, then its `padding-left`/`padding-right`, onto
/// `edges`, returning whether anything was set.
///
/// A side overrides only its own edge, so a bucket that sets only
/// `padding-left` keeps the policy's inherited top, bottom, and right
/// padding.
fn lower_padding(
    edges: &mut renderable::layout::Edges,
    style: &CommonStyle,
    bucket: &'static str,
) -> Result<bool, StyleApplyError> {
    lower_padding_sides(
        edges,
        style.padding.as_ref(),
        style.padding_left.as_ref(),
        style.padding_right.as_ref(),
        bucket,
    )
}

/// Applies `padding` to every edge, then each horizontal side over it.
fn lower_padding_sides(
    edges: &mut renderable::layout::Edges,
    padding: Option<&ComponentEdges>,
    left: Option<&Length>,
    right: Option<&Length>,
    bucket: &'static str,
) -> Result<bool, StyleApplyError> {
    if let Some(padding) = padding {
        *edges = padding.as_edges().clone();
    }
    if let Some(left) = left {
        edges.left = TargetValue::universal(length_to_layout_length(left, bucket, "padding-left")?);
    }
    if let Some(right) = right {
        edges.right = TargetValue::universal(length_to_layout_length(right, bucket, "padding-right")?);
    }
    Ok(padding.is_some() || left.is_some() || right.is_some())
}

/// Apply parsed page-level style onto a [`DarkmatterPage`] builder.
///
/// Every `style.page.*` field present in `style` is applied. Warnings remain
/// owned by the parser.
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
) -> Result<DarkmatterPage, StyleApplyError> {
    let Some(page_style) = style.page.as_ref() else {
        return Ok(page);
    };

    let mut page = page;

    // Store horizontal margins as the authored `Length` so the frame resolves
    // percentages per target (terminal cells vs. browser `%`).
    if let Some(len) = page_style.margin_left.as_ref() {
        page = page.with_margin_left_length(horizontal_length(len, "margin-left")?);
    }
    if let Some(len) = page_style.margin_right.as_ref() {
        page = page.with_margin_right_length(horizontal_length(len, "margin-right")?);
    }

    // Vertical margins (already u16 row counts in the schema).
    if let Some(rows) = page_style.margin_top {
        page = page.with_margin_top(rows);
    }
    if let Some(rows) = page_style.margin_bottom {
        page = page.with_margin_bottom(rows);
    }

    // Horizontal padding — authored `Length`, resolved per target.
    if let Some(len) = page_style.padding_left.as_ref() {
        page = page.with_padding_left_length(horizontal_length(len, "padding-left")?);
    }
    if let Some(len) = page_style.padding_right.as_ref() {
        page = page.with_padding_right_length(horizontal_length(len, "padding-right")?);
    }

    // Vertical padding.
    if let Some(rows) = page_style.padding_top {
        page = page.with_padding_top(rows);
    }
    if let Some(rows) = page_style.padding_bottom {
        page = page.with_padding_bottom(rows);
    }

    // Page background.
    if let Some(bg) = page_style.background {
        page = page.with_page_background(bg);
    }

    // Page background color.
    if let Some(bg_color) = page_style.bg_color.as_ref() {
        page = page.with_page_bg_color(bg_color.to_paint_color());
    }

    // Max width — stored as the authored `Length`. Percent resolves against the
    // post-margin / post-padding content width; the resolved value is validated
    // here (a percent that rounds to zero cells is rejected) while the `Length`
    // itself is retained so the browser can emit it as `%`.
    if let Some(len) = page_style.max_width.as_ref() {
        let resolved = lower_max_width(len, &page)?;
        if resolved == 0 {
            return Err(StyleApplyError::InvalidMaxWidth);
        }
        page = page.with_max_width_length(horizontal_length(len, "max-width")?);
    }

    // Alignment broadcasts to every page component; a component bucket's own
    // alignment, applied later, overrides it.
    if let Some(alignment) = page_style.alignment {
        for component in PageComponent::ALL {
            let mut policy = page.component_policy(component).cloned().unwrap_or_default();
            policy.layout.alignment = alignment;
            page = page.with_component_policy(component, policy);
        }
    }

    Ok(page)
}

/// Apply parsed component-level style (`style.table.*`, `style.images.*`,
/// `style.block-quote.*`) onto a [`DarkmatterPage`] builder.
///
/// Should be called **after** [`apply_page_style`] so that a component's own
/// alignment overrides a `style.page.alignment` broadcast.
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
) -> Result<DarkmatterPage, StyleApplyError> {
    let mut page = page;

    if let Some(table) = style.table.as_ref() {
        page = apply_common_style(
            page,
            "table",
            PageComponent::Tables,
            &table.common,
        )?;
    }

    if let Some(images) = style.images.as_ref() {
        page = apply_common_style(
            page,
            "images",
            PageComponent::Images,
            &images.common,
        )?;
    }

    if let Some(block_quote) = style.block_quote.as_ref() {
        page = apply_common_style(
            page,
            "block-quote",
            PageComponent::BlockQuotes,
            &block_quote.common,
        )?;
    }

    if let Some(code_block) = style.code_block.as_ref() {
        page = apply_common_style(
            page,
            "code-block",
            PageComponent::CodeBlocks,
            &code_block.common,
        )?;
    }

    Ok(page)
}

/// Apply parsed list-level style (`style.ul.*`, `style.ol.*`, `style.li.*`)
/// onto a [`DarkmatterPage`] builder.
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
) -> Result<DarkmatterPage, StyleApplyError> {
    let mut page = page;

    if let Some(ul) = style.ul.as_ref() {
        page = apply_list_bucket(
            page,
            "ul",
            PageComponent::Ul,
            &ul.common,
        )?;

        if let Some(left_margin) = ul.left_margin.as_ref() {
            let len = length_to_layout_length(left_margin, "ul", "left-margin")?;
            let mut policy = page.component_policy(PageComponent::Ul)
                .cloned()
                .unwrap_or_default();
            policy.layout.margin.left = TargetValue::universal(len);
            page = page.with_component_policy(PageComponent::Ul, policy);
        }
    }

    if let Some(ol) = style.ol.as_ref() {
        page = apply_list_bucket(
            page,
            "ol",
            PageComponent::Ol,
            &ol.common,
        )?;
    }

    if let Some(li) = style.li.as_ref() {
        page = apply_list_bucket(
            page,
            "li",
            PageComponent::Li,
            &li.common,
        )?;
    }

    Ok(page)
}

/// Apply parsed color and background-color style onto a [`DarkmatterPage`]
/// builder.
///
/// Should be called **after** [`apply_page_style`],
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
        if let Some(color) = page_style.color.as_ref() {
            page = page.with_page_color(color.to_paint_color());
        }
        if let Some(bg_color) = page_style.bg_color.as_ref() {
            page = page.with_page_bg_color(bg_color.to_paint_color());
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
    if let Some(disclosure) = style.disclosure.as_ref() {
        page = apply_common_color(page, PageComponent::Disclosure, &disclosure.common);
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
    if let Some(code_block) = style.code_block.as_ref() {
        page = apply_common_color(page, PageComponent::CodeBlocks, &code_block.common);
    }

    Ok(page)
}

/// Apply `color` and `bg-color` from a [`CommonStyle`] onto `page` for a
/// single component.
///
/// The [`StyleColor`](crate::style::StyleColor) is lowered to an alpha-bearing
/// [`PaintColor`](renderable::style::PaintColor) at this boundary so the
/// component policy stores the unified paint representation and opacity survives
/// the render tree without a side channel.
fn apply_common_color(
    page: DarkmatterPage,
    component: PageComponent,
    style: &CommonStyle,
) -> DarkmatterPage {
    let mut page = page;
    if let Some(color) = style.color.as_ref() {
        page = page.with_component_color(component, color.to_paint_color());
    }
    if let Some(bg_color) = style.bg_color.as_ref() {
        page = page.with_component_bg_color(component, bg_color.to_paint_color());
    }
    page
}

/// Apply parsed HR style (`style.hr.*`) onto a [`DarkmatterPage`] builder.
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
) -> Result<DarkmatterPage, StyleApplyError> {
    let Some(hr) = style.hr.as_ref() else {
        return Ok(page);
    };
    let mut page = page;

    if hr.width.is_some() && hr.max_width.is_some() {
        return Err(StyleApplyError::ComponentWidthConflict { bucket: "hr" });
    }

    if let Some(width) = hr.width.as_ref() {
        let s = lower_length_to_hr_width(width, "hr", "width")?;
        page = page.with_hr_width(s);
    } else if let Some(max_width) = hr.max_width.as_ref() {
        let s = lower_length_to_hr_width(max_width, "hr", "max-width")?;
        page = page.with_hr_width(s);
    }

    if let Some(alignment) = hr.alignment {
        page = page.with_hr_alignment(alignment);
    }

    // A rule's padding moves it inside the content width.
    let mut policy = page.component_policy(PageComponent::Hr).cloned().unwrap_or_default();
    if lower_padding_sides(
        &mut policy.layout.padding,
        hr.padding.as_ref(),
        hr.padding_left.as_ref(),
        hr.padding_right.as_ref(),
        "hr",
    )? {
        page = page.with_component_policy(PageComponent::Hr, policy);
    }

    if let Some(kind) = hr.kind {
        page = page.with_hr_kind(kind);
    }

    if let Some(weight) = hr.weight {
        page = page.with_hr_weight(weight);
    }

    if let Some(color) = hr.color.as_ref() {
        page = page.with_component_color(PageComponent::Hr, color.to_paint_color());
    }

    if let Some(bg_color) = hr.bg_color.as_ref() {
        page = page.with_component_bg_color(PageComponent::Hr, bg_color.to_paint_color());
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

/// Apply parsed disclosure component style (`style.disclosure.*`) onto a
/// [`DarkmatterPage`] builder.
///
/// ## Errors
///
/// - [`StyleApplyError::ComponentWidthConflict`] when both `width` and
///   `max-width` appear in the `style.disclosure` bucket.
/// - [`StyleApplyError::ComponentInvalidCssLength`] when `width` or
///   `max-width` is a [`Length::Css(_)`] value.
pub fn apply_disclosure_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
) -> Result<DarkmatterPage, StyleApplyError> {
    let Some(disclosure) = style.disclosure.as_ref() else {
        return Ok(page);
    };

    apply_common_style(
        page,
        "disclosure",
        PageComponent::Disclosure,
        &disclosure.common,
    )
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
) -> Result<DarkmatterPage, StyleApplyError> {
    let mut policy = page.component_policy(component).cloned().unwrap_or_default();
    let mut changed = false;

    match (style.width.as_ref(), style.max_width.as_ref()) {
        (Some(_), Some(_)) => {
            return Err(StyleApplyError::WidthMaxWidthConflict { bucket });
        }
        (Some(width), None) => {
            let width = width.as_width().clone();
            policy.layout.width = match width {
                Width::Fixed(tv) => {
                    let len = match tv {
                        renderable::layout::TargetValue::Universal(len) => len,
                        renderable::layout::TargetValue::PerTarget(_) => {
                            unreachable!("component width is always universal")
                        }
                    };
                    let len = length_to_layout_length(&len, bucket, "width")?;
                    Width::Fixed(TargetValue::universal(len))
                }
                mode => mode,
            };
            changed = true;
        }
        (None, Some(max_width)) => {
            let len = length_to_layout_length(max_width, bucket, "max-width")?;
            policy.layout.max_width = Some(TargetValue::universal(len));
            changed = true;
        }
        _ => {}
    }

    if let Some(alignment) = style.alignment {
        policy.layout.alignment = alignment;
        changed = true;
    }

    if let Some(margin) = style.margin.as_ref() {
        policy.layout.margin = margin.as_edges().clone();
        changed = true;
    }

    if lower_padding(&mut policy.layout.padding, style, bucket)? {
        changed = true;
    }

    if let Some(word_wrap) = style.word_wrap {
        policy.layout.word_wrap = word_wrap.to_word_wrap();
        changed = true;
    }

    if let Some(border) = style.border.as_ref() {
        policy.border = Some(border.as_border().clone());
        changed = true;
    }

    if let Some(emphasis) = style.emphasis.as_ref() {
        policy.emphasis = Some(emphasis.to_text_emphasis());
        changed = true;
    }

    if changed {
        Ok(page.with_component_policy(component, policy))
    } else {
        Ok(page)
    }
}

/// Apply one bucket's [`CommonStyle`] onto `page`.
///
/// `bucket` is the canonical kebab-case label (`"table"`, `"images"`,
/// `"block-quote"`) used in diagnostic messages. `component` is the matching
/// [`PageComponent`] for alignment/fill storage.
///
fn apply_common_style(
    page: DarkmatterPage,
    bucket: &'static str,
    component: PageComponent,
    style: &CommonStyle,
) -> Result<DarkmatterPage, StyleApplyError> {
    let mut policy = page.component_policy(component).cloned().unwrap_or_default();
    let mut changed = false;

    match (style.width.as_ref(), style.max_width.as_ref()) {
        (Some(_), Some(_)) => {
            return Err(StyleApplyError::ComponentWidthConflict { bucket });
        }
        (Some(width), None) => {
            let width = width.as_width().clone();
            policy.layout.width = match width {
                Width::Fixed(tv) => {
                    let len = match tv {
                        renderable::layout::TargetValue::Universal(len) => len,
                        renderable::layout::TargetValue::PerTarget(_) => {
                            unreachable!("component width is always universal")
                        }
                    };
                    let len = length_to_layout_length(&len, bucket, "width")?;
                    Width::Fixed(TargetValue::universal(len))
                }
                mode => mode,
            };
            changed = true;
        }
        (None, Some(max_width)) => {
            let len = length_to_layout_length(max_width, bucket, "max-width")?;
            policy.layout.max_width = Some(TargetValue::universal(len));
            changed = true;
        }
        _ => {}
    }

    if let Some(alignment) = style.alignment {
        policy.layout.alignment = alignment;
        changed = true;
    }

    if let Some(margin) = style.margin.as_ref() {
        policy.layout.margin = margin.as_edges().clone();
        changed = true;
    }

    if lower_padding(&mut policy.layout.padding, style, bucket)? {
        changed = true;
    }

    if let Some(word_wrap) = style.word_wrap {
        policy.layout.word_wrap = word_wrap.to_word_wrap();
        changed = true;
    }

    if let Some(border) = style.border.as_ref() {
        policy.border = Some(border.as_border().clone());
        changed = true;
    }

    if let Some(emphasis) = style.emphasis.as_ref() {
        policy.emphasis = Some(emphasis.to_text_emphasis());
        changed = true;
    }

    if changed {
        Ok(page.with_component_policy(component, policy))
    } else {
        Ok(page)
    }
}

/// Apply an effective style, prepared from its layers, onto a
/// [`DarkmatterPage`].
///
/// This is the page-construction entry for a [`PreparedStyle`]: every
/// layer was validated and merged by
/// [`prepare_style_layers`](super::prepare_style_layers), passively, and this
/// is where assets are executed. It sets, in order:
///
/// 1. the presentation `defaults` resolves over the merged style (prose and
///    code themes, code-block mode, line numbers; see
///    [`PresentationDefaults`]);
/// 2. page, component, list, rule, disclosure, and color style;
/// 3. the bespoke knobs, with `style.page.stylesheet` resolved by `assets`
///    in the context of the layer that won that field: the document's for a
///    document value, a style file's own, or the launch context for a
///    setter. A later style file that sets other fields does not move it.
///
/// Precedence was decided field by field by the layer order when the style
/// was prepared; nothing here overrides it.
///
/// ## Errors
///
/// Any [`StyleApplyError`] the individual `apply_*` functions return,
/// including a stylesheet that cannot be resolved or read by a target that
/// reads one.
pub fn apply_prepared_style(
    page: DarkmatterPage,
    prepared: &PreparedStyle,
    assets: StyleAssetResolver,
    defaults: &PresentationDefaults,
) -> Result<DarkmatterPage, StyleApplyError> {
    let style = prepared.style();
    let page = defaults.resolve(style).apply_to(page);
    let page = apply_page_style(page, style)?;
    let page = apply_component_style(page, style)?;
    let page = apply_list_style(page, style)?;
    let page = apply_hr_style(page, style)?;
    let page = apply_disclosure_style(page, style)?;
    let page = apply_color_style(page, style)?;
    let page = apply_bespoke_knobs(page, style)?;
    let Some(sheet) = style.page.as_ref().and_then(|page| page.stylesheet.as_deref()) else {
        return Ok(page);
    };
    // Preparation records an origin for every merged leaf; a style built some
    // other way has no context to resolve its stylesheet in.
    let origin = prepared
        .origin("page.stylesheet")
        .ok_or_else(|| StyleApplyError::StylesheetContextMissing { reference: sheet.trim().to_string() })?;
    apply_stylesheet(page, style, assets, &origin.context)
}

/// Validate a horizontal page-frame [`Length`] for storage, rejecting CSS units.
///
/// `Zero` / `Ch` / `Percent` pass through unchanged so the frame can resolve
/// them per target; [`Length::Css`] is rejected (terminal layout has no CSS
/// length).
fn horizontal_length(length: &Length, field: &'static str) -> Result<Length, StyleApplyError> {
    match length {
        Length::Zero | Length::Ch(_) | Length::Percent(_) => Ok(length.clone()),
        Length::Css(_) => Err(StyleApplyError::InvalidCssLength { field }),
    }
}

/// Resolve `style.page.max-width` against the content width after margins
/// and padding have been applied.
///
/// Reads the page's current margin + padding state, which already reflects
/// the margins and padding applied above in [`apply_page_style`].
fn lower_max_width(length: &Length, page: &DarkmatterPage) -> Result<u16, StyleApplyError> {
    match length {
        Length::Zero => Ok(0),
        Length::Ch(n) => Ok(u16::try_from(*n).unwrap_or(u16::MAX)),
        Length::Percent(p) => {
            // Without a width there is no base for the percent and nothing to
            // reject; the frame resolves it to no cap.
            let Some(columns) = page.terminal_width().as_columns() else {
                return Ok(u16::MAX);
            };
            let terminal_width = u16::try_from(columns).unwrap_or(u16::MAX);
            let consumed = edges_horizontal_ch(page.page_margin(), terminal_width)
                .saturating_add(edges_horizontal_ch(page.page_padding(), terminal_width));
            let content = terminal_width.saturating_sub(consumed);
            Ok(resolve_percent(*p, content))
        }
        Length::Css(_) => Err(StyleApplyError::InvalidCssLength { field: "max-width" }),
    }
}

/// Sum the horizontal sides of renderable [`Edges`] as whole terminal cells,
/// resolving any percent side against `base` (the terminal width).
fn edges_horizontal_ch(edges: &renderable::layout::Edges, base: u16) -> u16 {
    use renderable::layout::{Length, TargetValue};
    let cells = |tv: &TargetValue<Length>| match tv {
        TargetValue::Universal(Length::Ch(n)) => u16::try_from(*n).unwrap_or(u16::MAX),
        TargetValue::Universal(Length::Percent(p)) => resolve_percent(*p, base),
        _ => 0,
    };
    cells(&edges.left).saturating_add(cells(&edges.right))
}

/// Resolve a percent value against `base`, clamped to `u16` with rounding.
fn resolve_percent(p: f32, base: u16) -> u16 {
    let resolved = (f32::from(base) * (p / 100.0)).round();
    resolved.clamp(0.0, f32::from(u16::MAX)) as u16
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::ComponentPolicy;
    use crate::layout::PageBackground;
    use crate::style::schema::PageStyle;
    use biscuit_terminal::terminal::Terminal;
    use renderable::layout::{Alignment, Length};

    fn edge_ch(tv: &renderable::layout::TargetValue<renderable::layout::Length>) -> u16 {
        match tv {
            renderable::layout::TargetValue::Universal(renderable::layout::Length::Ch(n)) => u16::try_from(*n).unwrap_or(u16::MAX),
            _ => 0,
        }
    }

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

    fn apply_for_test(yaml: &str) -> DarkmatterPage {
        let style: StyleFrontmatter = serde_yaml_ng::from_str(yaml).expect("valid test yaml");
        let page = page(80);
        let page = apply_page_style(page, &style).unwrap();
        let page = apply_component_style(page, &style).unwrap();
        let page = apply_list_style(page, &style).unwrap();
        let page = apply_color_style(page, &style).unwrap();
        apply_hr_style(page, &style).unwrap()
    }

    // ---------- Phase 2: ComponentPolicy direct-lowering tests ----------

    #[test]
    fn apply_lowers_component_style_directly_to_layout() {
        let page = apply_for_test("table:\n  alignment: center\n  max-width: 60ch\n");
        let policy = page.component_policy(PageComponent::Tables).unwrap();
        assert_eq!(
            policy.layout.alignment,
            renderable::layout::Alignment::Center
        );
        assert_eq!(
            policy.layout.max_width,
            Some(renderable::layout::TargetValue::universal(
                renderable::layout::Length::ch(60)
            ))
        );
    }

    #[test]
    fn apply_lowers_width_to_fixed_layout() {
        let page = apply_for_test("images:\n  width: 40ch\n");
        let policy = page.component_policy(PageComponent::Images).unwrap();
        assert_eq!(
            policy.layout.width,
            renderable::layout::Width::Fixed(
                renderable::layout::TargetValue::universal(renderable::layout::Length::ch(40))
            )
        );
    }

    #[test]
    fn apply_lowers_list_left_margin_to_layout_margin() {
        let page = apply_for_test("ul:\n  left-margin: 4ch\n");
        let policy = page.component_policy(PageComponent::Ul).unwrap();
        assert_eq!(
            policy.layout.margin.left,
            renderable::layout::TargetValue::universal(renderable::layout::Length::ch(4))
        );
    }

    #[test]
    fn apply_lowers_color_to_policy() {
        use renderable::color::{Color, Tailwind};
        let page = apply_for_test("table:\n  color: red-500\n");
        let policy = page.component_policy(PageComponent::Tables).unwrap();
        let color = policy.color.as_ref().unwrap();
        assert_eq!(color.color, Color::Tailwind(Tailwind::Red500));
        assert_eq!(color.opacity, renderable::style::Opacity::OPAQUE);
    }

    #[test]
    fn apply_preserves_component_color_opacity() {
        let page = apply_for_test("table:\n  bg-color: red-500/50\n");
        let policy = page.component_policy(PageComponent::Tables).unwrap();
        let bg = policy.bg_color.as_ref().unwrap();
        assert_ne!(bg.opacity, renderable::style::Opacity::OPAQUE, "component bg-color opacity must survive lowering");
    }

    #[test]
    fn apply_lowers_disclosure_color_to_policy() {
        use renderable::color::{Color, Tailwind};
        let page = apply_for_test("disclosure:\n  color: red-500\n  bg-color: blue-500\n");
        let policy = page.component_policy(PageComponent::Disclosure).unwrap();
        let color = policy.color.as_ref().expect("disclosure color must reach policy");
        assert_eq!(color.color, Color::Tailwind(Tailwind::Red500));
        let bg = policy.bg_color.as_ref().expect("disclosure bg-color must reach policy");
        assert_eq!(bg.color, Color::Tailwind(Tailwind::Blue500));
    }

    #[test]
    fn apply_lowers_alignment_broadcast_to_all_components() {
        let page = apply_for_test("page:\n  alignment: center\n");
        for component in PageComponent::ALL {
            let policy = page.component_policy(component).unwrap();
            assert_eq!(
                policy.layout.alignment,
                renderable::layout::Alignment::Center,
                "alignment should broadcast to {:?}",
                component
            );
        }
    }

    #[test]
    fn apply_component_alignment_overrides_page_broadcast() {
        let page = apply_for_test(
            "page:\n  alignment: center\ntable:\n  alignment: right\n",
        );
        let table_policy = page.component_policy(PageComponent::Tables).unwrap();
        assert_eq!(
            table_policy.layout.alignment,
            renderable::layout::Alignment::Right
        );
        let images_policy = page.component_policy(PageComponent::Images).unwrap();
        assert_eq!(
            images_policy.layout.alignment,
            renderable::layout::Alignment::Center
        );
    }

    #[test]
    fn apply_lowers_pad_to_padding() {
        use renderable::layout::Edges;
        let policy = ComponentPolicy {
            layout: renderable::layout::Layout {
                padding: Edges::x(renderable::layout::Length::ch(4)),
                ..renderable::layout::Layout::default()
            },
            ..ComponentPolicy::default()
        };
        let page = page(80).with_component_policy(PageComponent::CodeBlocks, policy);
        let applied = page.component_policy(PageComponent::CodeBlocks).unwrap();
        assert_eq!(
            applied.layout.padding,
            Edges::x(renderable::layout::Length::ch(4))
        );
    }

    #[test]
    fn no_page_block_returns_page_unchanged() {
        let p = page(80);
        let style = StyleFrontmatter::default();
        let out = apply_page_style(p.clone(), &style).unwrap();
        assert_eq!(out.page_margin(), p.page_margin());
        assert_eq!(out.page_padding(), p.page_padding());
        assert_eq!(out.max_width(), p.max_width());
    }

    #[test]
    fn length_ch_lowers_to_u16() {
        let style = style_with_page(PageStyle {
            margin_left: Some(Length::Ch(4)),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style).unwrap();
        assert_eq!(edge_ch(&out.page_margin().left), 4);
    }

    #[test]
    fn length_zero_lowers_to_zero() {
        let style = style_with_page(PageStyle {
            margin_left: Some(Length::Zero),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style).unwrap();
        assert_eq!(edge_ch(&out.page_margin().left), 0);
    }

    #[test]
    fn percent_margin_is_retained_as_length_not_resolved() {
        // The page frame now retains the authored `Length`; the terminal
        // resolves the percent (10% of 80 = 8) and the browser emits `10%`.
        let style = style_with_page(PageStyle {
            margin_left: Some(Length::Percent(10.0)),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style).unwrap();
        assert_eq!(
            out.page_margin().left,
            renderable::layout::TargetValue::universal(Length::Percent(10.0)),
            "horizontal percent margin must be stored as a Length, not pre-resolved to cells",
        );
    }

    #[test]
    fn length_css_for_terminal_layout_returns_error() {
        use renderable::stylesheet::CssSizing;
        let style = style_with_page(PageStyle {
            margin_left: Some(Length::Css(CssSizing::px(10.0))),
            ..PageStyle::default()
        });
        let err = apply_page_style(page(80), &style).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::InvalidCssLength {
                field: "margin-left"
            }
        );
    }

    #[test]
    fn margin_left_applies_frontmatter() {
        let style = style_with_page(PageStyle {
            margin_left: Some(Length::Ch(4)),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style).unwrap();
        assert_eq!(edge_ch(&out.page_margin().left), 4);
    }

    #[test]
    fn vertical_margin_row_count_applied() {
        let style = style_with_page(PageStyle {
            margin_top: Some(1),
            margin_bottom: Some(0),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style).unwrap();
        assert_eq!(edge_ch(&out.page_margin().top), 1);
        assert_eq!(edge_ch(&out.page_margin().bottom), 0);
    }

    #[test]
    fn percent_max_width_resolves_post_margin_and_padding() {
        // 10% left margin + 10% right margin of 100 = 10 + 10 = 20.
        // Content width = 100 - 20 = 80. 50% of 80 = 40.
        let style = style_with_page(PageStyle {
            margin_left: Some(Length::Percent(10.0)),
            margin_right: Some(Length::Percent(10.0)),
            max_width: Some(Length::Percent(50.0)),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(100), &style).unwrap();
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
        let err = apply_page_style(p, &style).unwrap_err();
        assert_eq!(err, StyleApplyError::InvalidMaxWidth);
    }

    #[test]
    fn page_alignment_broadcasts_to_all_components() {
        use crate::layout::PageComponent;
        let style = style_with_page(PageStyle {
            alignment: Some(Alignment::Center),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style).unwrap();
        for component in PageComponent::ALL {
            assert_eq!(
                out.component_policy(component).map(|p| p.layout.alignment).unwrap_or_default(),
                renderable::layout::Alignment::Center,
                "alignment should broadcast to {:?}",
                component
            );
        }
    }

    #[test]
    fn background_applies() {
        let style = style_with_page(PageStyle {
            background: Some(PageBackground::Subtle),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style).unwrap();
        assert_eq!(out.page_background(), PageBackground::Subtle);
    }

    #[test]
    fn padding_lowers_horizontal_and_vertical() {
        let style = style_with_page(PageStyle {
            padding_left: Some(Length::Ch(3)),
            padding_right: Some(Length::Ch(3)),
            padding_top: Some(1),
            padding_bottom: Some(1),
            ..PageStyle::default()
        });
        let out = apply_page_style(page(80), &style).unwrap();
        assert_eq!(edge_ch(&out.page_padding().left), 3);
        assert_eq!(edge_ch(&out.page_padding().right), 3);
        assert_eq!(edge_ch(&out.page_padding().top), 1);
        assert_eq!(edge_ch(&out.page_padding().bottom), 1);
    }

    // ----- apply_component_style -----

    use crate::style::schema::{
        BlockQuoteStyle, CommonStyle, DisclosureStyle, ImageStyle, TableStyle,
    };

    fn style_with_table(common: CommonStyle) -> StyleFrontmatter {
        StyleFrontmatter {
            table: Some(TableStyle { common }),
            ..StyleFrontmatter::default()
        }
    }

    fn style_with_disclosure(common: CommonStyle) -> StyleFrontmatter {
        StyleFrontmatter {
            disclosure: Some(DisclosureStyle { common }),
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
            apply_component_style(p.clone(), &style).unwrap();
        for component in PageComponent::ALL {
            assert_eq!(
                out.component_policy(component).map(|p| p.layout.alignment),
                p.component_policy(component).map(|p| p.layout.alignment),
                "alignment mismatch for {:?}",
                component
            );
            assert_eq!(
                out.component_policy(component).map(|p| p.layout.max_width.clone()),
                p.component_policy(component).map(|p| p.layout.max_width.clone()),
                "max_width mismatch for {:?}",
                component
            );
            assert_eq!(
                out.component_policy(component).map(|p| p.layout.width.clone()),
                p.component_policy(component).map(|p| p.layout.width.clone()),
                "width mismatch for {:?}",
                component
            );
        }
    }

    #[test]
    fn table_alignment_applied() {
        let style = style_with_table(CommonStyle {
            alignment: Some(Alignment::Right),
            ..CommonStyle::default()
        });
        let out =
            apply_component_style(page(80), &style).unwrap();
        assert_eq!(
            out.component_policy(PageComponent::Tables).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Right
        );
    }

    #[test]
    fn table_max_width_applied() {
        let style = style_with_table(CommonStyle {
            max_width: Some(Length::Percent(50.0)),
            ..CommonStyle::default()
        });
        let out =
            apply_component_style(page(80), &style).unwrap();
        assert_eq!(
            out.component_policy(PageComponent::Tables).unwrap().layout.max_width,
            Some(renderable::layout::TargetValue::universal(renderable::layout::Length::Percent(50.0)))
        );
    }

    #[test]
    fn table_width_applied() {
        let style = style_with_table(CommonStyle {
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
            ..CommonStyle::default()
        });
        let out =
            apply_component_style(page(80), &style).unwrap();
        assert_eq!(
            out.component_policy(PageComponent::Tables).unwrap().layout.width,
            renderable::layout::Width::Fixed(renderable::layout::TargetValue::universal(renderable::layout::Length::ch(40)))
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
            apply_component_style(page(80), &style).unwrap();
        assert_eq!(
            out.component_policy(PageComponent::Images).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Center
        );
        assert_eq!(
            out.component_policy(PageComponent::Images).unwrap().layout.max_width,
            Some(renderable::layout::TargetValue::universal(renderable::layout::Length::ch(60)))
        );
    }

    #[test]
    fn block_quote_max_width_applied() {
        let style = style_with_block_quote(CommonStyle {
            max_width: Some(Length::Percent(75.0)),
            ..CommonStyle::default()
        });
        let out =
            apply_component_style(page(80), &style).unwrap();
        assert_eq!(
            out.component_policy(PageComponent::BlockQuotes).unwrap().layout.max_width,
            Some(renderable::layout::TargetValue::universal(renderable::layout::Length::Percent(75.0)))
        );
    }

    #[test]
    fn width_and_max_width_together_rejected() {
        let style = style_with_table(CommonStyle {
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
            max_width: Some(Length::Percent(50.0)),
            ..CommonStyle::default()
        });
        let err =
            apply_component_style(page(80), &style).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentWidthConflict { bucket: "table" }
        );
    }

    #[test]
    fn block_quote_width_conflict_uses_kebab_case_bucket() {
        let style = style_with_block_quote(CommonStyle {
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
            max_width: Some(Length::Ch(60)),
            ..CommonStyle::default()
        });
        let err =
            apply_component_style(page(80), &style).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("`style.block-quote.width`"),
            "expected kebab-case bucket in message, got: {msg}"
        );
    }

    /// The `disclosure` bucket delegates to the same `apply_common_style` helper
    /// as `table`/`block-quote`, but the spec calls out the `width` + `max-width`
    /// conflict explicitly, so assert it directly for `disclosure` rather than
    /// relying on helper reuse (review-5 finding #3).
    #[test]
    fn disclosure_width_and_max_width_together_rejected() {
        let style = style_with_disclosure(CommonStyle {
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
            max_width: Some(Length::Percent(50.0)),
            ..CommonStyle::default()
        });
        let err = apply_disclosure_style(page(80), &style)
            .unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentWidthConflict {
                bucket: "disclosure"
            }
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
            apply_component_style(page(80), &style).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentInvalidCssLength {
                bucket: "images",
                field: "max-width",
            }
        );
    }

    #[test]
    fn images_width_and_max_width_conflict() {
        let style = style_with_images(CommonStyle {
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(30))))),
            max_width: Some(Length::Ch(40)),
            ..CommonStyle::default()
        });
        let err = apply_component_style(page(80), &style).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentWidthConflict { bucket: "images" }
        );
    }

    #[test]
    fn component_alignment_overrides_page_broadcast() {
        // `apply_page_style` runs first and broadcasts `center` to every
        // component; `style.table.alignment: right` then overrides the
        // broadcast for tables only.
        let page_style = PageStyle {
            alignment: Some(Alignment::Center),
            ..PageStyle::default()
        };
        let mut style = style_with_table(CommonStyle {
            alignment: Some(Alignment::Right),
            ..CommonStyle::default()
        });
        style.page = Some(page_style);

        let p = apply_page_style(page(80), &style).unwrap();
        let out = apply_component_style(p, &style).unwrap();

        assert_eq!(
            out.component_policy(PageComponent::Tables).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Right,
            "component frontmatter should override page broadcast",
        );
        for component in [
            PageComponent::Images,
            PageComponent::Ul,
            PageComponent::Ol,
            PageComponent::Li,
            PageComponent::BlockQuotes,
            PageComponent::CodeBlocks,
        ] {
            assert_eq!(
                out.component_policy(component).map(|p| p.layout.alignment).unwrap_or_default(),
                renderable::layout::Alignment::Center,
                "untouched components keep page broadcast: {component:?}",
            );
        }
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
                width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
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
            apply_component_style(page(80), &style).unwrap();

        assert_eq!(
            out.component_policy(PageComponent::Tables).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Right
        );
        assert_eq!(
            out.component_policy(PageComponent::Tables).unwrap().layout.max_width,
            Some(renderable::layout::TargetValue::universal(renderable::layout::Length::Percent(50.0)))
        );
        assert_eq!(
            out.component_policy(PageComponent::Images).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Center
        );
        assert_eq!(
            out.component_policy(PageComponent::Images).unwrap().layout.width,
            renderable::layout::Width::Fixed(renderable::layout::TargetValue::universal(renderable::layout::Length::ch(40)))
        );
        assert_eq!(
            out.component_policy(PageComponent::BlockQuotes).unwrap().layout.max_width,
            Some(renderable::layout::TargetValue::universal(renderable::layout::Length::ch(60)))
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
        let out = apply_list_style(page(80), &style).unwrap();
        let policy = out.component_policy(PageComponent::Ul).unwrap();
        assert_eq!(
            policy.layout.margin.left,
            renderable::layout::TargetValue::universal(renderable::layout::Length::ch(4))
        );
    }

    #[test]
    fn ol_alignment_applied() {
        let style = style_with_ol(CommonStyle {
            alignment: Some(Alignment::Right),
            ..CommonStyle::default()
        });
        let out = apply_list_style(page(80), &style).unwrap();
        assert_eq!(
            out.component_policy(PageComponent::Ol).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Right
        );
    }

    #[test]
    fn li_width_applied() {
        let style = style_with_li(CommonStyle {
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(30))))),
            ..CommonStyle::default()
        });
        let out = apply_list_style(page(80), &style).unwrap();
        assert_eq!(
            out.component_policy(PageComponent::Li).unwrap().layout.width,
            renderable::layout::Width::Fixed(renderable::layout::TargetValue::universal(renderable::layout::Length::ch(30)))
        );
    }

    #[test]
    fn ul_width_max_width_conflict() {
        let style = style_with_ul(
            CommonStyle {
                width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
                max_width: Some(Length::Ch(60)),
                ..CommonStyle::default()
            },
            None,
        );
        let err = apply_list_style(page(80), &style).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::WidthMaxWidthConflict { bucket: "ul" }
        );
    }

    #[test]
    fn ol_width_max_width_conflict() {
        let style = style_with_ol(CommonStyle {
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
            max_width: Some(Length::Ch(60)),
            ..CommonStyle::default()
        });
        let err = apply_list_style(page(80), &style).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::WidthMaxWidthConflict { bucket: "ol" }
        );
    }

    #[test]
    fn li_width_max_width_conflict() {
        let style = style_with_li(CommonStyle {
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
            max_width: Some(Length::Ch(60)),
            ..CommonStyle::default()
        });
        let err = apply_list_style(page(80), &style).unwrap_err();
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
        let err = apply_list_style(page(80), &style).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentInvalidCssLength {
                bucket: "ul",
                field: "left-margin",
            }
        );
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
        let out = apply_hr_style(p.clone(), &style).unwrap();
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
        let out = apply_hr_style(page(80), &style).unwrap();
        assert_eq!(out.hr_kind(), Some(HrKind::Waves));
    }

    #[test]
    fn hr_weight_applied() {
        let style = style_with_hr(HrStyle {
            weight: Some(HrWeight::Thick),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style).unwrap();
        assert_eq!(out.hr_weight(), Some(HrWeight::Thick));
    }

    #[test]
    fn hr_alignment_applied() {
        let style = style_with_hr(HrStyle {
            alignment: Some(HrAlignment::Center),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style).unwrap();
        assert_eq!(out.hr_alignment(), Some(HrAlignment::Center));
    }

    #[test]
    fn hr_width_applied() {
        let style = style_with_hr(HrStyle {
            width: Some(Length::Percent(50.0)),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style).unwrap();
        assert_eq!(out.hr_width(), Some("50%"));
    }

    #[test]
    fn hr_max_width_applied() {
        let style = style_with_hr(HrStyle {
            max_width: Some(Length::Ch(40)),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style).unwrap();
        assert_eq!(out.hr_width(), Some("40ch"));
    }

    #[test]
    fn hr_width_and_max_width_conflict() {
        let style = style_with_hr(HrStyle {
            width: Some(Length::Ch(40)),
            max_width: Some(Length::Percent(50.0)),
            ..HrStyle::default()
        });
        let err = apply_hr_style(page(80), &style).unwrap_err();
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
        let paint = color.to_paint_color();
        let style = style_with_hr(HrStyle {
            color: Some(color.clone()),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style).unwrap();
        assert_eq!(
            out.color_for(PageComponent::Hr),
            Some(&paint)
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
        let paint = color.to_paint_color();
        let style = style_with_hr(HrStyle {
            bg_color: Some(color.clone()),
            ..HrStyle::default()
        });
        let out = apply_hr_style(page(80), &style).unwrap();
        assert_eq!(
            out.bg_color_for(PageComponent::Hr),
            Some(&paint)
        );
    }

    #[test]
    fn hr_css_length_rejected() {
        use renderable::stylesheet::CssSizing;
        let style = style_with_hr(HrStyle {
            width: Some(Length::Css(CssSizing::px(10.0))),
            ..HrStyle::default()
        });
        let err = apply_hr_style(page(80), &style).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentInvalidCssLength {
                bucket: "hr",
                field: "width",
            }
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
