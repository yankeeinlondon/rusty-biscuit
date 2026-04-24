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

use std::io::{self, IsTerminal, Read};

use clap::{Args, ValueEnum};
use tui_chrome::{BorderStyle, ChoiceOption, FrameChromeConfig, SortOrder};

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

/// Resolves the raw option strings from the CLI's source-precedence
/// chain.
///
/// ## Returns
///
/// - `Ok(None)` when the caller is using a legacy source
///   (`--options` / `--options-from-file` /
///   `--options-from-dictionary`) and should build the choice input
///   through the existing builder helpers.
/// - `Ok(Some(strings))` when either positional args or piped STDIN
///   supplied the option list.
///
/// ## Errors
///
/// Returns [`io::ErrorKind::InvalidInput`] when no source is
/// available — no legacy flag, no positionals, and STDIN is a TTY —
/// so the user is told which flag to set.
pub fn resolve_option_strings(
    has_legacy_source: bool,
    positional: Vec<String>,
) -> io::Result<Option<Vec<String>>> {
    if has_legacy_source {
        return Ok(None);
    }
    if !positional.is_empty() {
        return Ok(Some(positional));
    }
    if io::stdin().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no options provided: pass options as positional args, via stdin, or use one of \
             --options, --options-from-file, --options-from-dictionary",
        ));
    }
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    let lines: Vec<String> = buf
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    if lines.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no options provided: pass options as positional args, via stdin, or use one of \
             --options, --options-from-file, --options-from-dictionary",
        ));
    }
    Ok(Some(lines))
}

/// Splits an option string into `(label, value)` on the first
/// occurrence of `delimiter`.
///
/// When `delimiter` is `None`, or the string does not contain the
/// delimiter, both label and value are the original string. Trim is
/// applied after the split so `"Apple : 1"` with `':'` yields
/// `("Apple", "1")`.
pub fn parse_label_value(s: &str, delimiter: Option<char>) -> (String, String) {
    match delimiter {
        Some(ch) => match s.split_once(ch) {
            Some((label, value)) => (label.trim().to_string(), value.trim().to_string()),
            None => (s.to_string(), s.to_string()),
        },
        None => (s.to_string(), s.to_string()),
    }
}

/// Builds a list of [`ChoiceOption<String>`] from raw option strings.
///
/// The `id` field of each option is set to the **value** (not the
/// label) so `--selected` matches by value. This is the behaviour
/// change spec callers expect when a `--delimiter` splits a
/// `label⟂value` pair.
pub fn build_options(
    raw_strings: Vec<String>,
    delimiter: Option<char>,
) -> Vec<ChoiceOption<String>> {
    raw_strings
        .into_iter()
        .map(|s| {
            let (label, value) = parse_label_value(&s, delimiter);
            ChoiceOption::new(value.clone(), label, value)
        })
        .collect()
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
/// `--border-style` flags. The `--border-label` and a non-`none`
/// `--border-style` both implicitly enable the border, matching the
/// spec's "any border-flag implies a border" rule. Phases 10 and 11
/// populate margin and height fields.
pub fn build_chrome(args: &ChooseChromeArgs) -> FrameChromeConfig {
    let border = resolve_border_style(args);
    FrameChromeConfig {
        border,
        border_label: args.border_label.clone(),
        ..Default::default()
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

    #[test]
    fn parse_label_value_no_delimiter_returns_identity() {
        let (label, value) = parse_label_value("Apple", None);
        assert_eq!(label, "Apple");
        assert_eq!(value, "Apple");
    }

    #[test]
    fn parse_label_value_splits_on_first_delimiter_only() {
        let (label, value) = parse_label_value("Apple:1:2", Some(':'));
        assert_eq!(label, "Apple");
        assert_eq!(value, "1:2");
    }

    #[test]
    fn parse_label_value_trims_around_delimiter() {
        let (label, value) = parse_label_value("Apple : 1", Some(':'));
        assert_eq!(label, "Apple");
        assert_eq!(value, "1");
    }

    #[test]
    fn parse_label_value_falls_back_to_identity_when_delimiter_missing() {
        let (label, value) = parse_label_value("Apple", Some(':'));
        assert_eq!(label, "Apple");
        assert_eq!(value, "Apple");
    }

    #[test]
    fn build_options_sets_id_to_value() {
        let opts = build_options(vec!["Apple:1".into(), "Berry:2".into()], Some(':'));
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[0].id, "1");
        assert_eq!(opts[0].label, "Apple");
        assert_eq!(opts[0].value, "1");
        assert_eq!(opts[1].id, "2");
        assert_eq!(opts[1].label, "Berry");
        assert_eq!(opts[1].value, "2");
    }

    #[test]
    fn build_options_without_delimiter_sets_id_equal_to_label_equal_to_value() {
        let opts = build_options(vec!["alpha".into(), "beta".into()], None);
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[0].id, "alpha");
        assert_eq!(opts[0].label, "alpha");
        assert_eq!(opts[0].value, "alpha");
    }

    #[test]
    fn resolve_option_strings_returns_none_for_legacy_source() {
        let resolved = resolve_option_strings(true, vec![]).unwrap();
        assert!(resolved.is_none());
    }

    #[test]
    fn resolve_option_strings_prefers_positional_over_stdin() {
        let resolved = resolve_option_strings(false, vec!["a".into(), "b".into()]).unwrap();
        assert_eq!(resolved, Some(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn build_chrome_returns_default_config() {
        let args = ChooseChromeArgs::default();
        let chrome = build_chrome(&args);
        assert!(chrome.is_empty());
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
    fn apply_sort_reorders_labels_lexically_when_asc() {
        let mut options = build_options(
            vec!["Berry".into(), "Apple".into(), "Cherry".into()],
            None,
        );
        apply_sort(&mut options, SortOrder::Asc);
        let labels: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, vec!["Apple", "Berry", "Cherry"]);
    }

    #[test]
    fn apply_sort_on_labels_not_raw_strings_when_delimiter_present() {
        // With `--delimiter :` the label is the part before the colon.
        // Sorting ascending should order by label ("Apple" < "Berry"),
        // not by raw string ("Apple:zzz" vs "Berry:aaa").
        let mut options = build_options(
            vec!["Berry:aaa".into(), "Apple:zzz".into()],
            Some(':'),
        );
        apply_sort(&mut options, SortOrder::Asc);
        assert_eq!(options[0].label, "Apple");
        assert_eq!(options[0].value, "zzz");
        assert_eq!(options[1].label, "Berry");
        assert_eq!(options[1].value, "aaa");
    }

    #[test]
    fn apply_sort_natural_is_no_op() {
        let mut options = build_options(
            vec!["Berry".into(), "Apple".into(), "Cherry".into()],
            None,
        );
        apply_sort(&mut options, SortOrder::Natural);
        let labels: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, vec!["Berry", "Apple", "Cherry"]);
    }
}
