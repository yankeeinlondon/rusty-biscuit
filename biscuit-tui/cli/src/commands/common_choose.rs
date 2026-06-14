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

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use biscuit_tui::{
    ABORTED_KIND, ActiveChoiceColor, BorderStyle, CANCELLED_KIND, ChoiceInput, FrameChromeConfig,
    HeightSpec, HotkeyDisplayMode, Margin, Orientation, Padding, SelectionMode, SortOrder,
};
use clap::{Args, ValueEnum};

use crate::choice_normalize::NamingConvention;
use crate::choice_normalize::normalize_options;
use crate::option_sources::resolve_raw_options;
use crate::output::OutputMode;

/// Shared source-resolution clap arguments for the `choose-*` subcommands.
#[derive(Debug, Args, Clone, Default)]
pub struct ChooseSourceArgs {
    /// Option strings. Trailing positional arguments become the list
    /// of options when no explicit source flag is set.
    #[arg(value_name = "OPTIONS")]
    pub positional: Vec<String>,

    /// Comma-separated list of option values.
    #[arg(long = "csv", alias = "options", value_name = "TEXT")]
    pub csv: Option<String>,

    /// Newline-separated list of option values.
    #[arg(long, value_name = "TEXT")]
    pub list: Option<String>,

    /// Newline-separated rows of options.
    #[arg(long, value_name = "TEXT")]
    pub rows: Option<String>,

    /// Path to a file containing options (JSON, JSONL, NDJSON, YAML, TOML, or CSV).
    ///
    /// The file's top level must be an array of strings or an array of
    /// objects with `label` / `value` / `hotkey` / `disabled` keys.
    ///
    /// **TOML note:** standard TOML cannot represent a top-level bare
    /// array (the document root must be a table), so a TOML options file
    /// must use the `options = [...]` table form (e.g.
    /// `options = ["Red", "Green"]`). Other top-level keys (e.g.
    /// `colors = [...]`) are rejected with
    /// `option file must contain an array`.
    #[arg(long, value_name = "PATH")]
    pub file: Option<PathBuf>,

    /// Path to a markdown file and frontmatter property name containing
    /// an array of options.
    #[arg(long, value_names = ["PATH", "PROP"], num_args = 2)]
    pub md: Option<Vec<String>>,

    /// Legacy: path to a markdown file containing a bullet/numbered list.
    #[arg(long, hide = true)]
    pub options_from_file: Option<PathBuf>,

    /// Legacy: path to a YAML/JSON file containing a mapping of label -> value.
    #[arg(long, hide = true)]
    pub options_from_dictionary: Option<PathBuf>,
}

impl ChooseSourceArgs {
    fn markdown_source(&self) -> Option<(&Path, &str)> {
        self.md.as_ref().and_then(|v| {
            if v.len() >= 2 {
                Some((Path::new(&v[0]), v[1].as_str()))
            } else {
                None
            }
        })
    }
}

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
    /// typing alphanumeric characters opens the search prompt. With
    /// `--no-filter` set, those keystrokes are ignored — navigation
    /// is keyboard-arrow / explicit `[CTRL+x]` hotkey only.
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

    /// Background colour applied to the actively hovered option.
    ///
    /// The renderer resolves this against the detected terminal
    /// background (light/dark/unknown) to pick a palette that meets the
    /// spec's contrast rules.
    #[arg(long, value_enum, value_name = "COLOR", default_value_t = ActiveColorArg::Grey)]
    pub active_color: ActiveColorArg,

    /// When to render hotkey badges next to options that carry an
    /// explicit `Ctrl+X` / `Alt+X` shortcut.
    ///
    /// `auto` (the default) shows badges while the matching modifier
    /// is held; `always` keeps them visible for the lifetime of the
    /// prompt; `never` hides them entirely.
    ///
    /// Holding a bare modifier requires a terminal that emits
    /// kitty-protocol bare-modifier events (e.g. WezTerm with
    /// `enable_kitty_keyboard = true` and a recent build, or kitty.app).
    /// As a portable fallback, `Ctrl+Space` and `Alt+Space` toggle
    /// the corresponding emphasis. NOTE: macOS by default binds
    /// `Ctrl+Space` to "Select previous input source" — the chord
    /// will be eaten by the OS unless you uncheck that shortcut in
    /// System Settings → Keyboard → Keyboard Shortcuts → Input Sources.
    #[arg(long, value_enum, value_name = "MODE", default_value_t = HotkeyBadgesArg::Auto)]
    pub hotkey_badges: HotkeyBadgesArg,

    /// Layout direction for the option list.
    ///
    /// `vertical` (the default) stacks one option per row; `horizontal`
    /// packs options left-to-right, wrapping to new rows. Horizontal
    /// mode reserves a sub-row beneath each option for hotkey badges
    /// when [`HotkeyBadgesArg`] is non-`never`.
    #[arg(long, value_enum, value_name = "DIR", default_value_t = OrientationArg::Vertical)]
    pub orientation: OrientationArg,
}

/// CLI-facing layout-orientation mirror.
///
/// Kept separate from [`Orientation`] so clap can render kebab-case
/// help values without forcing the library to depend on clap.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum OrientationArg {
    /// One option per row, stacked vertically (the default).
    #[default]
    Vertical,
    /// Options packed left-to-right, wrapping to new rows.
    Horizontal,
}

impl From<OrientationArg> for Orientation {
    fn from(value: OrientationArg) -> Self {
        match value {
            OrientationArg::Vertical => Orientation::Vertical,
            OrientationArg::Horizontal => Orientation::Horizontal,
        }
    }
}

/// CLI-facing hotkey badge mode.
///
/// Mirrors [`HotkeyDisplayMode`] for the optional `--hotkey-badges`
/// flag while keeping the CLI free of clap-specific dependencies on
/// the library side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum HotkeyBadgesArg {
    /// Show badges only while the matching modifier is held (with a
    /// brief deadline fallback on unsupported terminals).
    #[default]
    Auto,
    /// Always show Ctrl badges (Alt badges render dim).
    Always,
    /// Never show badges, regardless of modifier state.
    #[clap(alias = "hidden")]
    Never,
    /// Force-show Ctrl badges for the lifetime of the prompt.
    Ctrl,
    /// Force-show Alt badges for the lifetime of the prompt.
    Alt,
}

/// Resolved hotkey badge override, if any.
///
/// Returns `None` for the default `auto` mode (so the state's
/// dynamic detection drives display). For `always` and `never` the
/// caller forces the corresponding [`HotkeyDisplayMode`] on the
/// state via `with_hotkey_display`.
pub fn resolve_hotkey_badges(arg: HotkeyBadgesArg) -> Option<HotkeyDisplayMode> {
    match arg {
        HotkeyBadgesArg::Auto => None,
        HotkeyBadgesArg::Always => Some(HotkeyDisplayMode::CtrlHeld),
        HotkeyBadgesArg::Never => Some(HotkeyDisplayMode::Hidden),
        HotkeyBadgesArg::Ctrl => Some(HotkeyDisplayMode::CtrlHeld),
        HotkeyBadgesArg::Alt => Some(HotkeyDisplayMode::AltHeld),
    }
}

/// CLI-facing active-colour mirror.
///
/// Kept separate from [`ActiveChoiceColor`] so clap can render
/// kebab-case help values without forcing the library to depend on
/// clap.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum ActiveColorArg {
    /// Neutral grey (the default; safe on light and dark terminals).
    #[default]
    Grey,
    /// Green accent.
    Green,
    /// Yellow accent.
    Yellow,
    /// Red accent.
    Red,
}

impl From<ActiveColorArg> for ActiveChoiceColor {
    fn from(value: ActiveColorArg) -> Self {
        match value {
            ActiveColorArg::Grey => ActiveChoiceColor::Grey,
            ActiveColorArg::Green => ActiveChoiceColor::Green,
            ActiveColorArg::Yellow => ActiveChoiceColor::Yellow,
            ActiveColorArg::Red => ActiveChoiceColor::Red,
        }
    }
}

/// CLI-facing sort-order mirror.
///
/// Kept separate from [`SortOrder`] so that clap's derive can render
/// kebab-case help values without the library having to depend on
/// clap.
///
/// ## Notes
///
/// `Inverse` is the canonical clap value. The legacy spelling
/// `reverse` remains accepted as a hidden alias on the same variant
/// for backward compatibility but is deliberately omitted from
/// `--help` output and shell completions so that `inverse` is the
/// only documented choice.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum SortOrderArg {
    /// Preserve source order.
    #[default]
    Natural,
    /// Reverse source order.
    //
    // The `reverse` clap alias is intentional and kept hidden from
    // `--help` and shell completions for backward compatibility — see
    // the type-level rustdoc above. We use a non-doc comment here so
    // the alias does not leak into clap's rendered help text.
    #[clap(alias = "reverse")]
    Inverse,
    /// Sort lexically by label (ascending).
    Asc,
    /// Sort lexically by label (descending).
    Desc,
}

impl From<SortOrderArg> for SortOrder {
    fn from(value: SortOrderArg) -> Self {
        match value {
            SortOrderArg::Natural => SortOrder::Natural,
            SortOrderArg::Inverse => SortOrder::Inverse,
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

/// Resolves raw source flags, normalizes options, and builds the shared
/// library choice input.
pub fn build_choice_input(
    source: &ChooseSourceArgs,
    chrome: &ChooseChromeArgs,
    selection_mode: SelectionMode,
) -> io::Result<ChoiceInput<String>> {
    let raw_options = resolve_raw_options(
        source.csv.as_deref(),
        source.list.as_deref(),
        source.rows.as_deref(),
        source.file.as_deref(),
        source.markdown_source(),
        source.options_from_file.as_deref(),
        source.options_from_dictionary.as_deref(),
        source.positional.clone(),
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;

    let options = normalize_options(
        raw_options,
        chrome.label_convention,
        chrome.value_convention,
        chrome.numeric_hot_keys,
        chrome.delimiter,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;

    Ok(ChoiceInput::new("choice", "")
        .with_selection_mode(selection_mode)
        .with_options(options)
        .with_sort(chrome.sort.into())
        .with_active_color(chrome.active_color.into())
        .with_orientation(chrome.orientation.into())
        .with_filter_enabled(!chrome.no_filter))
}

/// Runs a choose prompt and serializes the submitted value.
pub fn run_choice_with_writer<State, Value, RunPrompt, WriteValue, W>(
    state: State,
    output: OutputMode,
    height: Option<HeightSpec>,
    writer: &mut W,
    run_prompt: RunPrompt,
    write_value: WriteValue,
) -> io::Result<i32>
where
    RunPrompt: FnOnce(State, Option<HeightSpec>) -> io::Result<Value>,
    WriteValue: FnOnce(&mut W, Value, OutputMode) -> io::Result<()>,
    W: Write,
{
    match run_prompt(state, height) {
        Ok(value) => {
            write_value(writer, value, output)?;
            writer.flush()?;
            Ok(0)
        }
        Err(e) if e.kind() == CANCELLED_KIND => Ok(130),
        Err(e) if e.kind() == ABORTED_KIND => Ok(1),
        Err(e) => Err(e),
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
/// padding flag is set, the library default — [`Padding::default`],
/// equivalent to `Padding::uniform(1)` — is used. Explicit
/// `--padding 0` produces [`Padding::zero`] (no interior spacing).
fn resolve_padding(args: &ChooseChromeArgs) -> Padding {
    let base = args.padding;
    let has_any_padding_flag = base.is_some()
        || args.pt.is_some()
        || args.pb.is_some()
        || args.pl.is_some()
        || args.pr.is_some();
    if !has_any_padding_flag {
        return Padding::default();
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
    use biscuit_tui::Padding;

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
        assert_eq!(SortOrder::from(SortOrderArg::Inverse), SortOrder::Inverse);
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
    fn build_chrome_no_padding_flag_uses_library_default_uniform_one() {
        // Phase 2: when no --padding/--pt/--pb/--pl/--pr is supplied,
        // the resolver should return the library default
        // (Padding::uniform(1)) — *not* Padding::zero(). This
        // exercises the "unset flags use library default" rule and
        // mirrors the new `Padding::default()` behaviour.
        let args = ChooseChromeArgs::default();
        let chrome = build_chrome(&args);
        assert_eq!(chrome.padding, Padding::uniform(1));
        assert_eq!(chrome.padding, Padding::default());
        // Default padding alone makes the chrome non-empty because
        // padding affects layout; only Padding::zero() is "empty".
        assert!(!chrome.is_empty());
    }

    #[test]
    fn build_chrome_explicit_padding_zero_produces_zero_padding() {
        // Phase 2: --padding 0 must produce Padding::zero(), even
        // though the library default is now uniform(1). Combined with
        // no border/margin, this restores the FrameChromeConfig to
        // its empty state.
        let args = ChooseChromeArgs {
            padding: Some(0),
            ..ChooseChromeArgs::default()
        };
        let chrome = build_chrome(&args);
        assert_eq!(chrome.padding, Padding::zero());
        assert!(chrome.is_empty());
    }

    #[test]
    fn hotkey_badges_arg_default_is_auto() {
        assert_eq!(HotkeyBadgesArg::default(), HotkeyBadgesArg::Auto);
    }

    #[test]
    fn resolve_hotkey_badges_auto_returns_none() {
        assert!(resolve_hotkey_badges(HotkeyBadgesArg::Auto).is_none());
    }

    #[test]
    fn resolve_hotkey_badges_always_forces_ctrl_held() {
        assert_eq!(
            resolve_hotkey_badges(HotkeyBadgesArg::Always),
            Some(HotkeyDisplayMode::CtrlHeld)
        );
    }

    #[test]
    fn resolve_hotkey_badges_never_forces_hidden() {
        assert_eq!(
            resolve_hotkey_badges(HotkeyBadgesArg::Never),
            Some(HotkeyDisplayMode::Hidden)
        );
    }

    #[test]
    fn resolve_hotkey_badges_ctrl_forces_ctrl_held() {
        assert_eq!(
            resolve_hotkey_badges(HotkeyBadgesArg::Ctrl),
            Some(HotkeyDisplayMode::CtrlHeld)
        );
    }

    #[test]
    fn resolve_hotkey_badges_alt_forces_alt_held() {
        assert_eq!(
            resolve_hotkey_badges(HotkeyBadgesArg::Alt),
            Some(HotkeyDisplayMode::AltHeld)
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
