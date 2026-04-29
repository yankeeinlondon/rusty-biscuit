//! `question choose-many` subcommand.
//!
//! Maps CLI args onto a [`tui_chrome::ChooseManyState`], runs the
//! component via [`tui_chrome::run_standalone`], and writes the
//! captured option values according to the current [`OutputMode`].

use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args;
use tui_chrome::{
    ABORTED_KIND, CANCELLED_KIND, ChoiceInput, ChooseMany, ChooseManyState, HeightSpec, Label,
    SelectionMode, run_standalone_with_chrome,
};

use crate::choice_normalize::normalize_options;
use crate::commands::common_choose::{
    ChooseChromeArgs, apply_sort, build_chrome,
};
use crate::commands::text_input::LabelPositionArg;
use crate::option_sources::resolve_raw_options;
use crate::output::{OutputMode, write_list};

/// Arguments accepted by the `choose-many` subcommand.
#[derive(Debug, Args)]
pub struct ChooseManyArgs {
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

    /// Path to a file containing options (JSON, JSONL, YAML, TOML, or CSV).
    #[arg(long, value_name = "PATH")]
    pub file: Option<PathBuf>,

    /// Path to a markdown file and frontmatter property name containing
    /// an array of options.
    #[arg(long, value_names = ["PATH", "PROP"], num_args = 2)]
    pub md: Option<Vec<String>>,

    /// Legacy: path to a markdown file containing a bullet/numbered list.
    #[arg(long, hide = true)]
    pub options_from_file: Option<PathBuf>,

    /// Legacy: path to a YAML/JSON file containing a mapping of label → value.
    #[arg(long, hide = true)]
    pub options_from_dictionary: Option<PathBuf>,

    /// Label text rendered next to the list.
    #[arg(long)]
    pub label: Option<String>,

    /// Where the label renders relative to the list body.
    #[arg(long, value_enum, default_value_t = LabelPositionArg::Above)]
    pub label_position: LabelPositionArg,

    /// Pre-selected option values.
    ///
    /// Repeatable; pass `--selected foo --selected bar` to pre-select
    /// multiple values. Comma-splitting is **not** applied — each
    /// repetition is treated as a single literal value so option
    /// values containing `,` round-trip intact.
    #[arg(long, conflicts_with = "initial")]
    pub selected: Vec<String>,

    /// Deprecated alias for `--selected`. Will be removed in a future
    /// release.
    #[arg(long, hide = true)]
    pub initial: Option<String>,

    /// Submit fails validation when no selection is made.
    #[arg(long)]
    pub required: bool,

    /// Submit fails validation when fewer than `N` options are selected.
    #[arg(long)]
    pub min_selections: Option<usize>,

    /// Toggle-on is silently blocked once `N` options are selected.
    #[arg(long)]
    pub max_selections: Option<usize>,

    #[command(flatten)]
    pub chrome: ChooseChromeArgs,
}

/// Runs the `choose-many` subcommand.
///
/// ## Returns
///
/// `Ok(0)` on submission, `Ok(1)` when the user pressed `Esc`,
/// `Ok(130)` when the user pressed `Ctrl-C`, `Err` on a terminal I/O
/// error.
pub fn run(
    args: ChooseManyArgs,
    output: OutputMode,
    height: Option<HeightSpec>,
) -> io::Result<i32> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let chrome = build_chrome(&args.chrome);
    run_with_writer(args, output, height, &mut lock, |state, height| {
        run_standalone_with_chrome(ChooseMany::new(), state, height, chrome)
    })
}

fn run_with_writer<F, W>(
    args: ChooseManyArgs,
    output: OutputMode,
    height: Option<HeightSpec>,
    writer: &mut W,
    run_prompt: F,
) -> io::Result<i32>
where
    F: FnOnce(ChooseManyState, Option<HeightSpec>) -> io::Result<Vec<String>>,
    W: Write,
{
    let mut input = build_choice_input(&args)?;
    if args.required {
        input = input.required();
    }
    if let Some(min) = args.min_selections {
        input = input.with_min_selections(min);
    }
    if let Some(max) = args.max_selections {
        input = input.with_max_selections(max);
    }

    let values = effective_selected(&args);
    let mut state = ChooseManyState::new(input);
    if let Some(text) = args.label {
        state = state.with_label(Label::new(text, args.label_position.into()));
    }
    if !values.is_empty() {
        let refs: Vec<&str> = values.iter().map(String::as_str).collect();
        state = state.with_initial_values(&refs);
    }

    match run_prompt(state, height) {
        Ok(values) => {
            write_list(writer, &values, output)?;
            writer.flush()?;
            Ok(0)
        }
        Err(e) if e.kind() == CANCELLED_KIND => Ok(130),
        Err(e) if e.kind() == ABORTED_KIND => Ok(1),
        Err(e) => Err(e),
    }
}

fn effective_selected(args: &ChooseManyArgs) -> Vec<String> {
    if !args.selected.is_empty() {
        // `--selected` is repeatable at the clap layer; we preserve each
        // repetition verbatim so option values containing literal `,`
        // characters are not silently split apart. Callers that want
        // CSV semantics must use the deprecated `--initial` flag.
        return args
            .selected
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect();
    }
    if let Some(initial) = args.initial.as_deref() {
        eprintln!("question: warning: --initial is deprecated; use --selected instead");
        return parse_initial_ids(initial);
    }
    Vec::new()
}

fn build_choice_input(args: &ChooseManyArgs) -> io::Result<ChoiceInput<String>> {
    let md = args.md.as_ref().and_then(|v| {
        if v.len() >= 2 {
            Some((std::path::Path::new(&v[0]), v[1].as_str()))
        } else {
            None
        }
    });

    let raw_options = resolve_raw_options(
        args.csv.as_deref(),
        args.list.as_deref(),
        args.rows.as_deref(),
        args.file.as_deref(),
        md,
        args.options_from_file.as_deref(),
        args.options_from_dictionary.as_deref(),
        args.positional.clone(),
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;

    let options = normalize_options(
        raw_options,
        args.chrome.label_convention,
        args.chrome.value_convention,
        args.chrome.numeric_hot_keys,
        args.chrome.delimiter,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;

    let mut input = ChoiceInput::new("choice", "")
        .with_selection_mode(SelectionMode::Multiple)
        .with_options(options);
    apply_sort(&mut input.options, args.chrome.sort.into());
    Ok(input.with_filter_enabled(!args.chrome.no_filter))
}

fn parse_initial_ids(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_args() -> ChooseManyArgs {
        ChooseManyArgs {
            positional: Vec::new(),
            csv: None,
            list: None,
            rows: None,
            file: None,
            md: None,
            options_from_file: None,
            options_from_dictionary: None,
            label: None,
            label_position: LabelPositionArg::Above,
            selected: Vec::new(),
            initial: None,
            required: false,
            min_selections: None,
            max_selections: None,
            chrome: ChooseChromeArgs::default(),
        }
    }

    #[test]
    fn build_choice_input_from_csv_uses_multiple_mode() {
        let args = ChooseManyArgs {
            csv: Some("a,b,c".into()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 3);
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
    }

    #[test]
    fn build_choice_input_from_positional_uses_multiple_mode() {
        let args = ChooseManyArgs {
            positional: vec!["a".into(), "b".into()],
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
    }

    #[test]
    fn build_choice_input_enables_filter_for_positional_args_by_default() {
        let args = ChooseManyArgs {
            positional: vec!["a".into(), "b".into()],
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert!(input.filter_enabled);
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
    }

    #[test]
    fn build_choice_input_enables_filter_for_legacy_csv_by_default() {
        let args = ChooseManyArgs {
            csv: Some("a,b,c".into()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert!(input.filter_enabled);
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
    }

    #[test]
    fn build_choice_input_respects_no_filter_flag() {
        let args = ChooseManyArgs {
            positional: vec!["a".into(), "b".into()],
            chrome: ChooseChromeArgs {
                no_filter: true,
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert!(!input.filter_enabled);
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
    }

    #[test]
    fn build_choice_input_from_positional_with_delimiter_splits_label_value() {
        let args = ChooseManyArgs {
            positional: vec!["Apple:1".into(), "Berry:2".into()],
            chrome: ChooseChromeArgs {
                delimiter: Some(':'),
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].label, "Apple");
        assert_eq!(input.options[0].value, "1");
        assert_eq!(input.options[0].id, "1");
    }

    #[test]
    fn parse_initial_ids_trims_and_drops_empty() {
        let ids = parse_initial_ids("a, b , ,c");
        assert_eq!(ids, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[test]
    fn effective_selected_prefers_selected_over_initial() {
        let args = ChooseManyArgs {
            selected: vec!["new1".into(), "new2".into()],
            initial: None,
            ..default_args()
        };
        assert_eq!(effective_selected(&args), vec!["new1", "new2"]);
    }

    #[test]
    fn selected_value_containing_comma_is_preserved() {
        // Regression test for review-2 finding #1: `--selected` must
        // NOT split on commas, so a literal `one,two` value passes
        // through unchanged and matches the option whose value is
        // `one,two`.
        let args = ChooseManyArgs {
            positional: vec!["A|one,two".into(), "B|three".into()],
            selected: vec!["one,two".into()],
            chrome: ChooseChromeArgs {
                delimiter: Some('|'),
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| {
                assert_eq!(state.options()[0].label, "A");
                assert_eq!(state.options()[0].value, "one,two");
                assert_eq!(state.options()[0].id, "one,two");
                assert_eq!(state.selected_ids(), vec!["one,two"]);
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"one,two\n");
    }

    #[test]
    fn selected_repeated_flag_collects_all_values() {
        let args = ChooseManyArgs {
            positional: vec!["one".into(), "two".into(), "three".into()],
            selected: vec!["one".into(), "two".into(), "three".into()],
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| {
                assert_eq!(state.selected_ids(), vec!["one", "two", "three"]);
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"one\ntwo\nthree\n");
    }

    #[test]
    fn initial_still_splits_on_commas_for_backward_compat() {
        let args = ChooseManyArgs {
            positional: vec!["one".into(), "two".into()],
            selected: Vec::new(),
            initial: Some("one,two".into()),
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| {
                assert_eq!(state.selected_ids(), vec!["one", "two"]);
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"one\ntwo\n");
    }

    #[test]
    fn effective_selected_falls_back_to_initial_when_only_initial_set() {
        let args = ChooseManyArgs {
            selected: Vec::new(),
            initial: Some("legacy1, legacy2".into()),
            ..default_args()
        };
        assert_eq!(effective_selected(&args), vec!["legacy1", "legacy2"]);
    }

    #[test]
    fn effective_selected_empty_when_neither_set() {
        let args = default_args();
        assert!(effective_selected(&args).is_empty());
    }

    #[test]
    fn build_choice_input_applies_sort_reverse_across_positional_source() {
        use crate::commands::common_choose::SortOrderArg;
        let args = ChooseManyArgs {
            positional: vec!["a".into(), "b".into(), "c".into()],
            chrome: ChooseChromeArgs {
                sort: SortOrderArg::Reverse,
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        let labels: Vec<&str> = input.options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, vec!["c", "b", "a"]);
    }

    #[test]
    fn build_choice_input_without_source_errors() {
        let args = default_args();
        let err = build_choice_input(&args).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn run_writes_nul_separated_selected_values_from_initial_ids() {
        let args = ChooseManyArgs {
            csv: Some("Pepperoni,Mushrooms,Olives".into()),
            selected: vec!["Pepperoni".into(), "Olives".into()],
            required: true,
            min_selections: Some(1),
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Null,
            Some(HeightSpec::Cells(7)),
            &mut output,
            |state, height| {
                assert_eq!(height, Some(HeightSpec::Cells(7)));
                assert_eq!(state.selected_ids(), vec!["Pepperoni", "Olives"]);
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"Pepperoni\0Olives\0");
    }

    #[test]
    fn run_propagates_percent_height_to_prompt() {
        let args = ChooseManyArgs {
            csv: Some("A,B,C".into()),
            selected: vec!["A".into()],
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            Some(HeightSpec::Percent(40)),
            &mut output,
            |state, height| {
                assert_eq!(height, Some(HeightSpec::Percent(40)));
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"A\n");
    }

    #[test]
    fn run_writes_raw_newline_separated_values() {
        let args = ChooseManyArgs {
            csv: Some("Red,Green,Blue".into()),
            selected: vec!["Red".into(), "Blue".into()],
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| Ok(state.selected_values().into_iter().cloned().collect()),
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"Red\nBlue\n");
    }

    #[test]
    fn run_writes_selected_values_from_positional_args() {
        let args = ChooseManyArgs {
            positional: vec!["alpha".into(), "beta".into(), "gamma".into()],
            selected: vec!["alpha".into(), "gamma".into()],
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| {
                assert_eq!(state.options()[0].label, "alpha");
                assert_eq!(state.options()[2].label, "gamma");
                assert_eq!(state.selected_ids(), vec!["alpha", "gamma"]);
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"alpha\ngamma\n");
    }

    #[test]
    fn run_writes_delimited_positional_values() {
        let args = ChooseManyArgs {
            positional: vec!["Apple:1".into(), "Berry:2".into(), "Cherry:3".into()],
            selected: vec!["1".into(), "3".into()],
            chrome: ChooseChromeArgs {
                delimiter: Some(':'),
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Json,
            None,
            &mut output,
            |state, _height| {
                assert_eq!(state.options()[0].label, "Apple");
                assert_eq!(state.options()[0].id, "1");
                assert_eq!(state.options()[0].value, "1");
                assert_eq!(state.selected_ids(), vec!["1", "3"]);
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(String::from_utf8(output).unwrap(), "[\"1\",\"3\"]\n");
    }

    #[test]
    fn run_selected_defaults_match_delimited_values() {
        let args = ChooseManyArgs {
            positional: vec!["Apple:1".into(), "Berry:2".into(), "Cherry:3".into()],
            selected: vec!["2".into(), "3".into()],
            chrome: ChooseChromeArgs {
                delimiter: Some(':'),
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| {
                assert_eq!(state.selected_ids(), vec!["2", "3"]);
                assert_eq!(state.selected_count(), 2);
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"2\n3\n");
    }

    #[test]
    fn run_builds_filter_enabled_state_by_default() {
        let args = ChooseManyArgs {
            positional: vec!["alpha".into(), "beta".into()],
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| {
                assert!(state.filter_pattern().is_empty());
                assert!(!state.filter_visible());
                assert_eq!(state.visible_indices(), &[0, 1]);
                Ok(Vec::new())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(output.is_empty());
    }

    #[test]
    fn run_select_all_outputs_all_values() {
        let args = ChooseManyArgs {
            positional: vec!["Apple:1".into(), "Berry:2".into(), "Cherry:3".into()],
            chrome: ChooseChromeArgs {
                delimiter: Some(':'),
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |mut state, _height| {
                state.select_all();
                assert_eq!(state.selected_ids(), vec!["1", "2", "3"]);
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"1\n2\n3\n");
    }

    #[test]
    fn run_deselect_all_outputs_no_values() {
        let args = ChooseManyArgs {
            csv: Some("Red,Green,Blue".into()),
            selected: vec!["Red".into(), "Green".into(), "Blue".into()],
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |mut state, _height| {
                assert_eq!(state.selected_count(), 3);
                state.deselect_all();
                assert_eq!(state.selected_count(), 0);
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(output.is_empty());
    }

    #[test]
    fn run_writes_json_array_of_selected_values() {
        let args = ChooseManyArgs {
            csv: Some("A,B,C".into()),
            selected: vec!["A".into(), "C".into()],
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Json,
            None,
            &mut output,
            |state, _height| Ok(state.selected_values().into_iter().cloned().collect()),
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(String::from_utf8(output).unwrap(), "[\"A\",\"C\"]\n");
    }

    #[test]
    fn run_returns_130_without_output_on_ctrl_c() {
        let args = ChooseManyArgs {
            csv: Some("Pepperoni,Olives".into()),
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |_state, _height| Err(io::Error::new(CANCELLED_KIND, "interrupted")),
        )
        .unwrap();

        assert_eq!(status, 130);
        assert!(output.is_empty());
    }

    #[test]
    fn run_returns_1_without_output_on_esc() {
        let args = ChooseManyArgs {
            csv: Some("Pepperoni,Olives".into()),
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |_state, _height| Err(io::Error::new(ABORTED_KIND, "cancelled")),
        )
        .unwrap();

        assert_eq!(status, 1);
        assert!(output.is_empty());
    }
}
