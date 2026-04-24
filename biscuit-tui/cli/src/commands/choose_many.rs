//! `question choose-many` subcommand.
//!
//! Maps CLI args onto a [`tui_chrome::ChooseManyState`], runs the
//! component via [`tui_chrome::run_standalone`], and writes the
//! captured option values according to the current [`OutputMode`].

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args;
use tui_chrome::helpers::choice_builders::{
    choose_many_from_csv, choose_many_from_markdown_list, choose_one_from_dictionary,
};
use tui_chrome::{
    ABORTED_KIND, CANCELLED_KIND, ChoiceInput, ChooseMany, ChooseManyState, Label, SelectionMode,
    run_standalone,
};

use crate::commands::common_choose::{
    ChooseChromeArgs, apply_sort, build_options, resolve_option_strings,
};
use crate::commands::text_input::LabelPositionArg;
use crate::output::{OutputMode, write_list};

/// Arguments accepted by the `choose-many` subcommand.
#[derive(Debug, Args)]
pub struct ChooseManyArgs {
    /// Option strings. Trailing positional arguments become the list
    /// of options when no legacy `--options*` flag is set.
    #[arg(value_name = "OPTIONS")]
    pub positional: Vec<String>,

    /// Comma-separated list of option values.
    #[arg(long, conflicts_with_all = ["options_from_file", "options_from_dictionary"])]
    pub options: Option<String>,

    /// Path to a markdown file containing a bullet/numbered list of
    /// options.
    #[arg(long, conflicts_with_all = ["options", "options_from_dictionary"])]
    pub options_from_file: Option<PathBuf>,

    /// Path to a YAML/JSON file containing a mapping of label → value.
    #[arg(long, conflicts_with_all = ["options", "options_from_file"])]
    pub options_from_dictionary: Option<PathBuf>,

    /// Label text rendered next to the list.
    #[arg(long)]
    pub label: Option<String>,

    /// Where the label renders relative to the list body.
    #[arg(long, value_enum, default_value_t = LabelPositionArg::Above)]
    pub label_position: LabelPositionArg,

    /// Pre-selected option values. Repeatable and/or comma-separated;
    /// each repetition is split on `,` so `--selected a,b --selected c`
    /// yields `[a, b, c]`.
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
/// `Ok(0)` on submission, `Ok(130)` on cancellation, `Err` on a
/// terminal I/O error.
pub fn run(args: ChooseManyArgs, output: OutputMode, height: Option<u16>) -> io::Result<i32> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    run_with_writer(args, output, height, &mut lock, |state, height| {
        run_standalone(ChooseMany::new(), state, height)
    })
}

fn run_with_writer<F, W>(
    args: ChooseManyArgs,
    output: OutputMode,
    height: Option<u16>,
    writer: &mut W,
    run_prompt: F,
) -> io::Result<i32>
where
    F: FnOnce(ChooseManyState, Option<u16>) -> io::Result<Vec<String>>,
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
        Err(e) if e.kind() == CANCELLED_KIND || e.kind() == ABORTED_KIND => Ok(130),
        Err(e) => Err(e),
    }
}

fn effective_selected(args: &ChooseManyArgs) -> Vec<String> {
    if !args.selected.is_empty() {
        return flatten_selected(&args.selected);
    }
    if let Some(initial) = args.initial.as_deref() {
        eprintln!("question: warning: --initial is deprecated; use --selected instead");
        return parse_initial_ids(initial);
    }
    Vec::new()
}

fn flatten_selected(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn build_choice_input(args: &ChooseManyArgs) -> io::Result<ChoiceInput<String>> {
    let mut input = if let Some(csv) = args.options.as_deref() {
        choose_many_from_csv("choice", "", csv)
    } else if let Some(path) = args.options_from_file.as_ref() {
        let body = fs::read_to_string(path)?;
        choose_many_from_markdown_list("choice", "", &body)
    } else if let Some(path) = args.options_from_dictionary.as_ref() {
        let body = fs::read_to_string(path)?;
        // choose_many variant: reuse choose_one_from_dictionary then
        // flip selection mode.
        let input = choose_one_from_dictionary("choice", "", &body)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        input.with_selection_mode(SelectionMode::Multiple)
    } else {
        let resolved = resolve_option_strings(false, args.positional.clone())?
            .expect("resolve_option_strings returns Some when no legacy source is set");
        ChoiceInput::new("choice", "")
            .with_selection_mode(SelectionMode::Multiple)
            .with_options(build_options(resolved, args.chrome.delimiter))
    };
    apply_sort(&mut input.options, args.chrome.sort.into());
    Ok(input)
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
            options: None,
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
            options: Some("a,b,c".into()),
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
    fn flatten_selected_splits_on_commas_and_joins_repetitions() {
        let values = vec!["a,b".to_string(), "c".to_string(), "d, e".to_string()];
        assert_eq!(
            flatten_selected(&values),
            vec!["a", "b", "c", "d", "e"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        );
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
            options: Some("Pepperoni,Mushrooms,Olives".into()),
            selected: vec!["Pepperoni".into(), "Olives".into()],
            required: true,
            min_selections: Some(1),
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Null,
            Some(7),
            &mut output,
            |state, height| {
                assert_eq!(height, Some(7));
                assert_eq!(state.selected_ids(), vec!["Pepperoni", "Olives"]);
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"Pepperoni\0Olives\0");
    }

    #[test]
    fn run_writes_raw_newline_separated_values() {
        let args = ChooseManyArgs {
            options: Some("Red,Green,Blue".into()),
            selected: vec!["Red,Blue".into()],
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
    fn run_writes_json_array_of_selected_values() {
        let args = ChooseManyArgs {
            options: Some("A,B,C".into()),
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
    fn run_returns_130_without_output_on_cancel() {
        let args = ChooseManyArgs {
            options: Some("Pepperoni,Olives".into()),
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |_state, _height| Err(io::Error::new(CANCELLED_KIND, "cancelled")),
        )
        .unwrap();

        assert_eq!(status, 130);
        assert!(output.is_empty());
    }
}
