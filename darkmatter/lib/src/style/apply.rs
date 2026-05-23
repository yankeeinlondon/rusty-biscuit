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

use renderable::layout::{Alignment, Length};
use thiserror::Error;

use crate::layout::{DarkmatterPage, PageAlignment, PageComponent};
use crate::style::schema::StyleFrontmatter;

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
    pub align_block_quotes: bool,
    pub align_tables: bool,
    pub align_code_blocks: bool,
}

impl PageStyleOverrides {
    /// Returns `true` when `component` has been claimed by a component-specific
    /// CLI alignment flag and must therefore be skipped by a
    /// `style.page.alignment` broadcast.
    fn alignment_claimed_for(&self, component: PageComponent) -> bool {
        match component {
            PageComponent::Images => self.align_images,
            PageComponent::Lists => self.align_lists,
            PageComponent::BlockQuotes => self.align_block_quotes,
            PageComponent::Tables => self.align_tables,
            PageComponent::CodeBlocks => self.align_code_blocks,
        }
    }
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
            let consumed = margin
                .horizontal()
                .saturating_add(padding.horizontal());
            let content = terminal_width.saturating_sub(consumed);
            Ok(resolve_percent(*p, content))
        }
        Length::Css(_) => Err(StyleApplyError::InvalidCssLength {
            field: "max-width",
        }),
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
        let out =
            apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
        assert_eq!(out.margin().left, 4);
    }

    #[test]
    fn length_zero_lowers_to_zero() {
        let style = style_with_page(PageStyle {
            left_margin: Some(Length::Zero),
            ..PageStyle::default()
        });
        let out =
            apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
        assert_eq!(out.margin().left, 0);
    }

    #[test]
    fn length_percent_resolves_against_terminal_width_for_margin() {
        // 10% of 80 = 8.
        let style = style_with_page(PageStyle {
            left_margin: Some(Length::Percent(10.0)),
            ..PageStyle::default()
        });
        let out =
            apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
        assert_eq!(out.margin().left, 8);
    }

    #[test]
    fn length_css_for_terminal_layout_returns_error() {
        use renderable::stylesheet::CssSizing;
        let style = style_with_page(PageStyle {
            left_margin: Some(Length::Css(CssSizing::px(10.0))),
            ..PageStyle::default()
        });
        let err =
            apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap_err();
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
        let out =
            apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
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
        let out =
            apply_page_style(page(100), &style, PageStyleOverrides::default()).unwrap();
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
        let err =
            apply_page_style(p, &style, PageStyleOverrides::default()).unwrap_err();
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
        let out =
            apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
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
        let out =
            apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
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
    fn padding_lowers_horizontal_and_vertical() {
        let style = style_with_page(PageStyle {
            left_padding: Some(Length::Ch(3)),
            right_padding: Some(Length::Ch(3)),
            top_padding: Some(1),
            bottom_padding: Some(1),
            ..PageStyle::default()
        });
        let out =
            apply_page_style(page(80), &style, PageStyleOverrides::default()).unwrap();
        assert_eq!(out.padding().left, 3);
        assert_eq!(out.padding().right, 3);
        assert_eq!(out.padding().top, 1);
        assert_eq!(out.padding().bottom, 1);
    }
}
