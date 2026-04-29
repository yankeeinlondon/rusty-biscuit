//! Shared CLI argument parsing and source resolution for the
//! `choose-one` and `choose-many` subcommands.
//!
//! Both subcommands share the same set of frame-chrome, ordering, and
//! source-resolution flags. Collecting them here avoids duplicating
//! clap attributes across two structs and gives later phases a single
//! seam to extend when the remaining flags (`--border`, `--margin`,
//! `--height`, `--sort`) come online.
//!
//! ## Source Precedence
//!
//! When no legacy `--options` / `--options-from-file` /
//! `--options-from-dictionary` flag is set, [`resolve_option_strings`]
//! picks the next non-empty source in the order:
//!
//! 1. trailing positional arguments
//! 2. `stdin`, if it is a pipe (non-TTY)
//!
//! and returns an error if none are available.

use clap::{Args, ValueEnum};
use tui_chrome::{
    BorderStyle, ChoiceOption, FrameChromeConfig, HeightSpec, Margin, Padding, SortOrder,
};

use crate::choice_normalize::NamingConvention;

/// Shared clap arguments for the `choose-*` subcommands.
///
/// Phase 3 introduces the source-resolution flag (`--delimiter`);
/// Phase 4 adds `--sort`; Phase 9 adds the `--border*` family. Later
/// phases extend this struct with `--margin*` and `--height`.
#[derive(Debug, Args, Clone, Default)]
pub struct ChooseChromeArgs {
    /// Split each option string into label and value on the first
    /// occurrence of `<CHAR>`.
    ///
    /// Applied after source resolution, so it works uniformly for
    /// STDIN, positional, and legacy sources.
    #[arg(long, value_name = "CHAR")]
    pub delimiter: Option<char>,

    /// Ordering applied to the option list before state construction.
    ///
    /// Runs after `--delimiter` so the sort operates on labels, not on
    /// raw source strings.
    #[arg(long, value_enum, default_value_t = SortOrderArg::Natural)]
    pub sort: SortOrderArg,

    /// Disable inline fuzzy filtering.
    ///
    /// Filtering is enabled by default for CLI choose prompts so
    /// typing alphanumeric characters opens the search prompt instead
    /// of using the legacy first-letter shortcut behaviour.
    #[arg(long)]
    pub no_filter: bool,

    /// Draw a border around the prompt.
    ///
    /// Implies [`BorderStyleArg::Rounded`] when `--border-style` is
    /// not also supplied. Setting `--border-label` or
    /// `--border-style <non-none>` also implies a border without
    /// requiring `--border` explicitly.
    #[arg(long)]
    pub border: bool,

    /// Title rendered in the top-left of the border.
    ///
    /// Implies `--border` (defaults to rounded when no
    /// `--border-style` is supplied). Long labels are silently
    /// truncated by ratatui to fit the border width.
    #[arg(long, value_name = "TEXT")]
    pub border_label: Option<String>,

    /// Border glyph style.
    ///
    /// Any value other than `none` implies `--border`.
    #[arg(long, value_enum, value_name = "STYLE")]
    pub border_style: Option<BorderStyleArg>,

    /// Margin (in cells) applied to all four sides outside the border.
    ///
    /// Per-side flags (`--mt`, `--mb`, `--ml`, `--mr`) override the
    /// umbrella value for that side only — `--margin 2 --mt 0` yields
    /// `Margin { top: 0, bottom: 2, left: 2, right: 2 }`.
    #[arg(long, value_name = "CELLS")]
    pub margin: Option<u16>,

    /// Override the top margin (cells). Takes precedence over
    /// `--margin` for the top side only.
    #[arg(long, value_name = "CELLS")]
    pub mt: Option<u16>,

    /// Override the bottom margin (cells). Takes precedence over
    /// `--margin` for the bottom side only.
    #[arg(long, value_name = "CELLS")]
    pub mb: Option<u16>,

    /// Override the left margin (cells). Takes precedence over
    /// `--margin` for the left side only.
    #[arg(long, value_name = "CELLS")]
    pub ml: Option<u16>,

    /// Override the right margin (cells). Takes precedence over
    /// `--margin` for the right side only.
    #[arg(long, value_name = "CELLS")]
    pub mr: Option<u16>,

    /// Padding (in cells) applied to all four sides inside the border.
    ///
    /// Per-side flags (`--pt`, `--pb`, `--pl`, `--pr`) override the
    /// umbrella value for that side only — `--padding 2 --pt 0` yields
    /// `Padding { top: 0, bottom: 2, left: 2, right: 2 }`.
    #[arg(short = 'p', long, value_name = "CELLS")]
    pub padding: Option<u16>,

    /// Override the top padding (cells). Takes precedence over
    /// `--padding` for the top side only.
    #[arg(long, value_name = "CELLS")]
    pub pt: Option<u16>,

    /// Override the bottom padding (cells). Takes precedence over
    /// `--padding` for the bottom side only.
    #[arg(long, value_name = "CELLS")]
    pub pb: Option<u16>,

    /// Override the left padding (cells). Takes precedence over
    /// `--padding` for the left side only.
    #[arg(long, value_name = "CELLS")]
    pub pl: Option<u16>,

    /// Override the right padding (cells). Takes precedence over
    /// `--padding` for the right side only.
    #[arg(long, value_name = "CELLS")]
    pub pr: Option<u16>,

    /// Assign numeric hotkeys (Ctrl+1..9,0 then Alt+1..9,0) to the
    /// first 20 options. Explicit hotkeys are never overwritten.
    #[arg(long)]
    pub numeric_hot_keys: bool,

    /// Naming convention applied to option labels.
    #[arg(long, value_enum, default_value_t = NamingConvention::None)]
    pub label_convention: NamingConvention,

    /// Naming convention applied to option values.
    #[arg(long, value_enum, default_value_t = NamingConvention::None)]
    pub value_convention: NamingConvention,
}

/// CLI-facing sort-order mirror.
///
/// Kept separate from [`SortOrder`] so that clap's derive can render
/// kebab-case help values without the library having to depend on
/// clap.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum SortOrderArg {
    /// Preserve source order.
    #[default]
    Natural,
    /// Reverse source order.
    Reverse,
    /// Sort lexically by label (ascending).
    Asc,
    /// Sort lexically by label (descending).
    Desc,
}

impl From<SortOrderArg> for SortOrder {
    fn from(value: SortOrderArg) -> Self {
        match value {
            SortOrderArg::Natural => SortOrder::Natural,
            SortOrderArg::Reverse => SortOrder::Reverse,
            SortOrderArg::Asc => SortOrder::Asc,
            SortOrderArg::Desc => SortOrder::Desc,
        }
    }
}

/// CLI-facing border-style mirror.
///
/// Kept separate from [`BorderStyle`] so clap renders kebab-case help
/// values without forcing the library to depend on clap. The variants
/// match [`BorderStyle`] one-to-one and convert via [`From`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum BorderStyleArg {
    /// No border drawn (the default).
    #[default]
    None,
    /// Rounded corners on all four sides.
    Rounded,
    /// Single-line border with sharp corners on all four sides.
    Sharp,
    /// Thick single-line border on all four sides.
    Bold,
    /// Double-line border on all four sides.
    Double,
    /// Block border (quadrant-outside) on all four sides.
    Block,
    /// Thin block border (quadrant-inside) on all four sides.
    ThinBlock,
    /// Plain border on top and bottom only.
    Horizontal,
    /// Plain border on left and right only.
    Vertical,
    /// A single horizontal rule on top.
    Line,
    /// Plain border on top only.
    Top,
    /// Plain border on bottom only.
    Bottom,
    /// Plain border on left only.
    Left,
    /// Plain border on right only.
    Right,
}

impl From<BorderStyleArg> for BorderStyle {
    fn from(value: BorderStyleArg) -> Self {
        match value {
            BorderStyleArg::None => BorderStyle::None,
            BorderStyleArg::Rounded => BorderStyle::Rounded,
            BorderStyleArg::Sharp => BorderStyle::Sharp,
            BorderStyleArg::Bold => BorderStyle::Bold,
            BorderStyleArg::Double => BorderStyle::Double,
            BorderStyleArg::Block => BorderStyle::Block,
            BorderStyleArg::ThinBlock => BorderStyle::ThinBlock,
            BorderStyleArg::Horizontal => BorderStyle::Horizontal,
            BorderStyleArg::Vertical => BorderStyle::Vertical,
            BorderStyleArg::Line => BorderStyle::Line,
            BorderStyleArg::Top => BorderStyle::Top,
            BorderStyleArg::Bottom => BorderStyle::Bottom,
            BorderStyleArg::Left => BorderStyle::Left,
            BorderStyleArg::Right => BorderStyle::Right,
        }
    }
}

/// Applies the caller's [`SortOrder`] to `options` in place.
///
/// Sort runs after `--delimiter` parsing so users sort on labels, not
/// on raw input strings. Invoked uniformly for every source (legacy
/// `--options*`, positional, and STDIN) so the CLI's ordering is
/// independent of source precedence.
pub fn apply_sort(options: &mut [ChoiceOption<String>], order: SortOrder) {
    order.apply(options);
}

/// Builds a [`FrameChromeConfig`] from the shared CLI args.
///
/// Phase 9 wires the `--border`, `--border-label`, and
/// `--border-style` flags. Phase 10 wires the `--margin`, `--mt`,
/// `--mb`, `--ml`, `--mr` family. The `--border-label` and a
/// non-`none` `--border-style` both implicitly enable the border,
/// matching the spec's "any border-flag implies a border" rule. Phase
/// 11 populates the height field.
pub fn build_chrome(args: &ChooseChromeArgs) -> FrameChromeConfig {
    let border = resolve_border_style(args);
    FrameChromeConfig {
        border,
        border_label: args.border_label.clone(),
        margin: resolve_margin(args),
        padding: resolve_padding(args),
        ..Default::default()
    }
}

/// Parses a `--height` argument into a [`HeightSpec`].
///
/// Accepts either a bare integer (interpreted as absolute cells) or
/// an integer followed by `%` (interpreted as a percentage of the
/// terminal height).
///
/// ## Errors
///
/// Returns an error message when the input is empty, fails to parse
/// as an unsigned integer, or specifies a percentage outside
/// `1..=100`.
pub fn parse_height_spec(s: &str) -> Result<HeightSpec, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("height must not be empty".to_string());
    }
    if let Some(num) = trimmed.strip_suffix('%') {
        let raw = num.trim();
        let parsed: u8 = raw
            .parse()
            .map_err(|_| format!("invalid height percent: {raw}"))?;
        if !(1..=100).contains(&parsed) {
            return Err("percent must be between 1 and 100".to_string());
        }
        Ok(HeightSpec::Percent(parsed))
    } else {
        let parsed: u16 = trimmed
            .parse()
            .map_err(|_| format!("invalid height: {trimmed}"))?;
        if parsed == 0 {
            return Err("height must be greater than 0".to_string());
        }
        Ok(HeightSpec::Cells(parsed))
    }
}

/// Resolves the effective [`Margin`] from the supplied args.
///
/// `--margin` seeds every side; each per-side flag (`--mt`, `--mb`,
/// `--ml`, `--mr`) overrides the matching side when set. Omitted
/// umbrella and per-side flags fall through to zero, matching the
/// default `Margin`.
fn resolve_margin(args: &ChooseChromeArgs) -> Margin {
    let base = args.margin.unwrap_or(0);
    Margin {
        top: args.mt.unwrap_or(base),
        bottom: args.mb.unwrap_or(base),
        left: args.ml.unwrap_or(base),
        right: args.mr.unwrap_or(base),
    }
}

/// Resolves the effective [`Padding`] from the supplied args.
///
/// `--padding` seeds every side; each per-side flag (`--pt`, `--pb`,
/// `--pl`, `--pr`) overrides the matching side when set. When no
/// padding flag is set, the default `Padding::uniform(1)` from
/// [`FrameChromeConfig::default`] is used.
fn resolve_padding(args: &ChooseChromeArgs) -> Padding {
    let base = args.padding;
    let has_any_padding_flag = base.is_some()
        || args.pt.is_some()
        || args.pb.is_some()
        || args.pl.is_some()
        || args.pr.is_some();
    if !has_any_padding_flag {
        return Padding::uniform(1);
    }
    let base = base.unwrap_or(0);
    Padding {
        top: args.pt.unwrap_or(base),
        bottom: args.pb.unwrap_or(base),
        left: args.pl.unwrap_or(base),
        right: args.pr.unwrap_or(base),
    }
}

/// Resolves the effective [`BorderStyle`] from the supplied args.
///
/// Precedence:
///
/// 1. An explicit non-`none` `--border-style` always wins.
/// 2. An explicit `--border-style none` wins even if `--border` /
///    `--border-label` are set, so users can suppress an inherited
///    style without removing the other flags.
/// 3. `--border` or `--border-label` with no `--border-style`
///    defaults to [`BorderStyle::Rounded`].
/// 4. Otherwise, no border.
fn resolve_border_style(args: &ChooseChromeArgs) -> BorderStyle {
    if let Some(style) = args.border_style {
        return style.into();
    }
    if args.border || args.border_label.is_some() {
        return BorderStyle::Rounded;
    }
    BorderStyle::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tui_chrome::Padding;

    #[test]
    fn build_chrome_returns_default_config_with_padding() {
        let args = ChooseChromeArgs::default();
        let chrome = build_chrome(&args);
        // Default padding is uniform(1), so the config is not empty.
        assert!(!chrome.is_empty());
        assert_eq!(chrome.padding, Padding::uniform(1));
    }

    #[test]
    fn build_chrome_border_flag_defaults_to_rounded() {
        let args = ChooseChromeArgs {
            border: true,
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(chrome.border, BorderStyle::Rounded);
        assert!(chrome.border_label.is_none());
        assert!(!chrome.is_empty());
    }

    #[test]
    fn build_chrome_border_label_implies_border() {
        let args = ChooseChromeArgs {
            border_label: Some("Pick".to_string()),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(chrome.border, BorderStyle::Rounded);
        assert_eq!(chrome.border_label.as_deref(), Some("Pick"));
    }

    #[test]
    fn build_chrome_explicit_border_style_overrides_default() {
        let args = ChooseChromeArgs {
            border: true,
            border_style: Some(BorderStyleArg::Double),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(chrome.border, BorderStyle::Double);
    }

    #[test]
    fn build_chrome_border_style_alone_implies_border() {
        let args = ChooseChromeArgs {
            border_style: Some(BorderStyleArg::Bold),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(chrome.border, BorderStyle::Bold);
    }

    #[test]
    fn build_chrome_border_style_none_suppresses_border() {
        // Explicit --border-style none overrides --border even when
        // both are set, so users can keep the rest of their flag set
        // and just kill the border.
        let args = ChooseChromeArgs {
            border: true,
            border_label: Some("Pick".to_string()),
            border_style: Some(BorderStyleArg::None),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(chrome.border, BorderStyle::None);
    }

    #[test]
    fn border_style_arg_default_is_none() {
        assert_eq!(BorderStyleArg::default(), BorderStyleArg::None);
    }

    #[test]
    fn border_style_arg_maps_to_library_enum() {
        assert_eq!(BorderStyle::from(BorderStyleArg::None), BorderStyle::None);
        assert_eq!(
            BorderStyle::from(BorderStyleArg::Rounded),
            BorderStyle::Rounded
        );
        assert_eq!(BorderStyle::from(BorderStyleArg::Bold), BorderStyle::Bold);
        assert_eq!(
            BorderStyle::from(BorderStyleArg::Double),
            BorderStyle::Double
        );
        assert_eq!(BorderStyle::from(BorderStyleArg::Block), BorderStyle::Block);
        assert_eq!(
            BorderStyle::from(BorderStyleArg::ThinBlock),
            BorderStyle::ThinBlock
        );
        assert_eq!(
            BorderStyle::from(BorderStyleArg::Horizontal),
            BorderStyle::Horizontal
        );
        assert_eq!(
            BorderStyle::from(BorderStyleArg::Vertical),
            BorderStyle::Vertical
        );
        assert_eq!(BorderStyle::from(BorderStyleArg::Line), BorderStyle::Line);
        assert_eq!(BorderStyle::from(BorderStyleArg::Top), BorderStyle::Top);
        assert_eq!(
            BorderStyle::from(BorderStyleArg::Bottom),
            BorderStyle::Bottom
        );
        assert_eq!(BorderStyle::from(BorderStyleArg::Left), BorderStyle::Left);
        assert_eq!(BorderStyle::from(BorderStyleArg::Right), BorderStyle::Right);
        assert_eq!(BorderStyle::from(BorderStyleArg::Sharp), BorderStyle::Sharp);
    }

    #[test]
    fn sort_order_arg_maps_to_library_enum() {
        assert_eq!(SortOrder::from(SortOrderArg::Natural), SortOrder::Natural);
        assert_eq!(SortOrder::from(SortOrderArg::Reverse), SortOrder::Reverse);
        assert_eq!(SortOrder::from(SortOrderArg::Asc), SortOrder::Asc);
        assert_eq!(SortOrder::from(SortOrderArg::Desc), SortOrder::Desc);
    }

    #[test]
    fn sort_order_arg_default_is_natural() {
        assert_eq!(SortOrderArg::default(), SortOrderArg::Natural);
    }

    #[test]
    fn build_chrome_default_margin_is_zero() {
        let args = ChooseChromeArgs::default();
        let chrome = build_chrome(&args);
        assert_eq!(chrome.margin, Margin::default());
    }

    #[test]
    fn build_chrome_margin_sets_all_sides() {
        let args = ChooseChromeArgs {
            margin: Some(2),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(chrome.margin, Margin::uniform(2));
        // Margin alone is enough to mark the chrome as non-empty so
        // the render path wraps the widget with a FrameChrome.
        assert!(!chrome.is_empty());
    }

    #[test]
    fn build_chrome_per_side_overrides_umbrella_margin() {
        // `--margin 2 --mt 0` should produce a margin with zero on top
        // and two on the other three sides, per spec §7.2.
        let args = ChooseChromeArgs {
            margin: Some(2),
            mt: Some(0),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(
            chrome.margin,
            Margin {
                top: 0,
                bottom: 2,
                left: 2,
                right: 2,
            }
        );
    }

    #[test]
    fn build_chrome_per_side_only_margins_default_other_sides_to_zero() {
        let args = ChooseChromeArgs {
            ml: Some(3),
            mr: Some(4),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(
            chrome.margin,
            Margin {
                top: 0,
                bottom: 0,
                left: 3,
                right: 4,
            }
        );
    }

    #[test]
    fn build_chrome_all_per_side_overrides_applied() {
        let args = ChooseChromeArgs {
            margin: Some(5),
            mt: Some(1),
            mb: Some(2),
            ml: Some(3),
            mr: Some(4),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(
            chrome.margin,
            Margin {
                top: 1,
                bottom: 2,
                left: 3,
                right: 4,
            }
        );
    }

    #[test]
    fn build_chrome_combines_border_and_margin() {
        let args = ChooseChromeArgs {
            border: true,
            margin: Some(2),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(chrome.border, BorderStyle::Rounded);
        assert_eq!(chrome.margin, Margin::uniform(2));
    }

    #[test]
    fn apply_sort_reorders_labels_lexically_when_asc() {
        let mut options = vec![
            ChoiceOption::new("Berry", "Berry", "Berry"),
            ChoiceOption::new("Apple", "Apple", "Apple"),
            ChoiceOption::new("Cherry", "Cherry", "Cherry"),
        ];
        apply_sort(&mut options, SortOrder::Asc);
        let labels: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, vec!["Apple", "Berry", "Cherry"]);
    }

    #[test]
    fn apply_sort_on_labels_not_raw_strings_when_delimiter_present() {
        // With `--delimiter :` the label is the part before the colon.
        // Sorting ascending should order by label ("Apple" < "Berry"),
        // not by raw string ("Apple:zzz" vs "Berry:aaa").
        let mut options = vec![
            ChoiceOption::new("aaa", "Berry", "aaa"),
            ChoiceOption::new("zzz", "Apple", "zzz"),
        ];
        apply_sort(&mut options, SortOrder::Asc);
        assert_eq!(options[0].label, "Apple");
        assert_eq!(options[0].value, "zzz");
        assert_eq!(options[1].label, "Berry");
        assert_eq!(options[1].value, "aaa");
    }

    #[test]
    fn apply_sort_natural_is_no_op() {
        let mut options = vec![
            ChoiceOption::new("Berry", "Berry", "Berry"),
            ChoiceOption::new("Apple", "Apple", "Apple"),
            ChoiceOption::new("Cherry", "Cherry", "Cherry"),
        ];
        apply_sort(&mut options, SortOrder::Natural);
        let labels: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, vec!["Berry", "Apple", "Cherry"]);
    }

    #[test]
    fn parse_height_spec_accepts_bare_integer_as_cells() {
        assert_eq!(parse_height_spec("10"), Ok(HeightSpec::Cells(10)));
        assert_eq!(parse_height_spec(" 5 "), Ok(HeightSpec::Cells(5)));
    }

    #[test]
    fn parse_height_spec_accepts_percent_suffix() {
        assert_eq!(parse_height_spec("50%"), Ok(HeightSpec::Percent(50)));
        assert_eq!(parse_height_spec("100%"), Ok(HeightSpec::Percent(100)));
        assert_eq!(parse_height_spec("1%"), Ok(HeightSpec::Percent(1)));
    }

    #[test]
    fn parse_height_spec_rejects_empty_input() {
        assert!(parse_height_spec("").is_err());
        assert!(parse_height_spec("   ").is_err());
    }

    #[test]
    fn parse_height_spec_rejects_zero_cells() {
        // HeightSpec::Cells(0) would collapse the inline viewport to
        // nothing; reject it at parse time with a clear message.
        assert!(parse_height_spec("0").is_err());
    }

    #[test]
    fn parse_height_spec_rejects_out_of_range_percent() {
        assert!(parse_height_spec("0%").is_err());
        assert!(parse_height_spec("101%").is_err());
    }

    #[test]
    fn parse_height_spec_rejects_non_numeric_input() {
        assert!(parse_height_spec("tall").is_err());
        assert!(parse_height_spec("twenty%").is_err());
    }

    #[test]
    fn build_chrome_padding_sets_all_sides() {
        let args = ChooseChromeArgs {
            padding: Some(2),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(chrome.padding, Padding::uniform(2));
    }

    #[test]
    fn build_chrome_per_side_overrides_umbrella_padding() {
        let args = ChooseChromeArgs {
            padding: Some(2),
            pt: Some(0),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(
            chrome.padding,
            Padding {
                top: 0,
                bottom: 2,
                left: 2,
                right: 2,
            }
        );
    }

    #[test]
    fn build_chrome_per_side_only_padding_defaults_other_sides_to_zero() {
        let args = ChooseChromeArgs {
            pl: Some(3),
            pr: Some(4),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(
            chrome.padding,
            Padding {
                top: 0,
                bottom: 0,
                left: 3,
                right: 4,
            }
        );
    }

    #[test]
    fn build_chrome_all_per_side_padding_overrides_applied() {
        let args = ChooseChromeArgs {
            padding: Some(5),
            pt: Some(1),
            pb: Some(2),
            pl: Some(3),
            pr: Some(4),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(
            chrome.padding,
            Padding {
                top: 1,
                bottom: 2,
                left: 3,
                right: 4,
            }
        );
    }

    #[test]
    fn build_chrome_padding_combined_with_border_and_margin() {
        let args = ChooseChromeArgs {
            border: true,
            margin: Some(2),
            padding: Some(1),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(chrome.border, BorderStyle::Rounded);
        assert_eq!(chrome.margin, Margin::uniform(2));
        assert_eq!(chrome.padding, Padding::uniform(1));
    }
}
