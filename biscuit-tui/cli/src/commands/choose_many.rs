//! `question choose-many` subcommand.
//!
//! Maps CLI args onto a [`biscuit_tui::ChooseManyState`], runs the
//! component via [`biscuit_tui::run_standalone`], and writes the
//! captured option values according to the current [`OutputMode`].

use std::io::{self, Write};

use clap::Args;
use biscuit_tui::{
    ChoiceInput, ChooseMany, ChooseManyState, HeightSpec, Label, SelectionMode,
    run_standalone_with_chrome,
};

use crate::commands::common_choose::{
    ChooseChromeArgs, ChooseSourceArgs, build_choice_input as build_common_choice_input,
    build_chrome, resolve_hotkey_badges, run_choice_with_writer,
};
use crate::commands::text_input::LabelPositionArg;
use crate::output::{OutputMode, write_list};

/// Arguments accepted by the `choose-many` subcommand.
#[derive(Debug, Args)]
pub struct ChooseManyArgs {
    #[command(flatten)]
    pub source: ChooseSourceArgs,

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
    show_on_exit: bool,
) -> io::Result<i32> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let mut chrome = build_chrome(&args.chrome);
    chrome.show_on_exit = show_on_exit;
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
    let hotkey_override = resolve_hotkey_badges(args.chrome.hotkey_badges);
    let mut state = ChooseManyState::new(input);
    if let Some(text) = args.label {
        state = state.with_label(Label::new(text, args.label_position.into()));
    }
    if !values.is_empty() {
        let refs: Vec<&str> = values.iter().map(String::as_str).collect();
        state = state.with_initial_values(&refs);
    }
    if let Some(mode) = hotkey_override {
        state = state.with_hotkey_display(mode);
    }

    run_choice_with_writer(
        state,
        output,
        height,
        writer,
        run_prompt,
        |writer, values, output| write_list(writer, &values, output),
    )
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
    build_common_choice_input(&args.source, &args.chrome, SelectionMode::Multiple)
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
    use biscuit_tui::{ABORTED_KIND, CANCELLED_KIND};

    fn default_args() -> ChooseManyArgs {
        ChooseManyArgs {
            source: ChooseSourceArgs::default(),
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

    fn source_csv(csv: String) -> ChooseSourceArgs {
        ChooseSourceArgs {
            csv: Some(csv),
            ..ChooseSourceArgs::default()
        }
    }

    fn source_positional(positional: Vec<String>) -> ChooseSourceArgs {
        ChooseSourceArgs {
            positional,
            ..ChooseSourceArgs::default()
        }
    }

    fn source_file(file: std::path::PathBuf) -> ChooseSourceArgs {
        ChooseSourceArgs {
            file: Some(file),
            ..ChooseSourceArgs::default()
        }
    }

    #[test]
    fn build_choice_input_from_csv_uses_multiple_mode() {
        let args = ChooseManyArgs {
            source: source_csv("a,b,c".into()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 3);
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
    }

    #[test]
    fn build_choice_input_from_positional_uses_multiple_mode() {
        let args = ChooseManyArgs {
            source: source_positional(vec!["a".into(), "b".into()]),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
    }

    #[test]
    fn build_choice_input_active_color_default_is_grey() {
        let args = ChooseManyArgs {
            source: source_csv("a,b".into()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.active_color, biscuit_tui::ActiveChoiceColor::Grey);
    }

    #[test]
    fn build_choice_input_active_color_yellow_propagates_to_input() {
        use crate::commands::common_choose::ActiveColorArg;
        let args = ChooseManyArgs {
            source: source_csv("a,b".into()),
            chrome: ChooseChromeArgs {
                active_color: ActiveColorArg::Yellow,
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.active_color, biscuit_tui::ActiveChoiceColor::Yellow);
    }

    #[test]
    fn build_choice_input_enables_filter_for_positional_args_by_default() {
        let args = ChooseManyArgs {
            source: source_positional(vec!["a".into(), "b".into()]),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert!(input.filter_enabled);
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
    }

    #[test]
    fn build_choice_input_enables_filter_for_legacy_csv_by_default() {
        let args = ChooseManyArgs {
            source: source_csv("a,b,c".into()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert!(input.filter_enabled);
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
    }

    #[test]
    fn build_choice_input_respects_no_filter_flag() {
        let args = ChooseManyArgs {
            source: source_positional(vec!["a".into(), "b".into()]),
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
            source: source_positional(vec!["Apple:1".into(), "Berry:2".into()]),
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
            source: source_positional(vec!["A|one,two".into(), "B|three".into()]),
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
        // Labels are chosen to have distinct first alphanumeric chars
        // so the CLI's effective-hotkey duplicate check (`Ctrl+<first
        // alphanumeric>`) does not reject them.
        let args = ChooseManyArgs {
            source: source_positional(vec!["one".into(), "two".into(), "four".into()]),
            selected: vec!["one".into(), "two".into(), "four".into()],
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| {
                assert_eq!(state.selected_ids(), vec!["one", "two", "four"]);
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"one\ntwo\nfour\n");
    }

    #[test]
    fn initial_still_splits_on_commas_for_backward_compat() {
        let args = ChooseManyArgs {
            source: source_positional(vec!["one".into(), "two".into()]),
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
    fn build_choice_input_applies_sort_inverse_across_positional_source() {
        // Sorting now happens inside `ChooseManyState::new`, driven by
        // `ChoiceInput::with_sort`. The CLI builder must therefore
        // configure `sort` so that constructing a state yields the
        // expected order.
        use crate::commands::common_choose::SortOrderArg;
        let args = ChooseManyArgs {
            source: source_positional(vec!["a".into(), "b".into(), "c".into()]),
            chrome: ChooseChromeArgs {
                sort: SortOrderArg::Inverse,
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.sort, Some(biscuit_tui::SortOrder::Inverse));
        let state = ChooseManyState::new(input);
        let labels: Vec<&str> = state.options().iter().map(|o| o.label.as_str()).collect();
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
            source: source_csv("Pepperoni,Mushrooms,Olives".into()),
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
            source: source_csv("A,B,C".into()),
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
            source: source_csv("Red,Green,Blue".into()),
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
            source: source_positional(vec!["alpha".into(), "beta".into(), "gamma".into()]),
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
            source: source_positional(vec!["Apple:1".into(), "Berry:2".into(), "Cherry:3".into()]),
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
            source: source_positional(vec!["Apple:1".into(), "Berry:2".into(), "Cherry:3".into()]),
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
            source: source_positional(vec!["alpha".into(), "beta".into()]),
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
            source: source_positional(vec!["Apple:1".into(), "Berry:2".into(), "Cherry:3".into()]),
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
            source: source_csv("Red,Green,Blue".into()),
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
            source: source_csv("A,B,C".into()),
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
            source: source_csv("Pepperoni,Olives".into()),
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

    // --- Phase 3: object-record source preservation ---------------------

    fn write_temp_file(name: &str, body: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn build_choice_input_from_json_object_array_preserves_value_and_hotkey() {
        let path = write_temp_file(
            "choose_many_phase3_objects.json",
            r#"[{"label":"Red","value":"apple","hotkey":"CTRL+R"},{"label":"Blue","value":"sky"}]"#,
        );
        let args = ChooseManyArgs {
            source: source_file(path.clone()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].value, "apple");
        assert_eq!(
            input.options[0].hotkey,
            Some(biscuit_tui::HotkeySpec::Ctrl('r'))
        );
        assert_eq!(input.options[1].value, "sky");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn build_choice_input_from_yaml_object_array_preserves_value_and_hotkey() {
        let body = "- label: Red\n  value: apple\n  hotkey: CTRL+R\n- label: Blue\n  value: sky\n";
        let path = write_temp_file("choose_many_phase3_objects.yaml", body);
        let args = ChooseManyArgs {
            source: source_file(path.clone()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options[0].value, "apple");
        assert_eq!(
            input.options[0].hotkey,
            Some(biscuit_tui::HotkeySpec::Ctrl('r'))
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn build_choice_input_from_csv_three_columns_preserves_value_and_hotkey() {
        let body = "Red,apple,CTRL+R\nBlue,sky,ALT+B\n";
        let path = write_temp_file("choose_many_phase3_objects.csv", body);
        let args = ChooseManyArgs {
            source: source_file(path.clone()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options[0].value, "apple");
        assert_eq!(
            input.options[0].hotkey,
            Some(biscuit_tui::HotkeySpec::Ctrl('r'))
        );
        assert_eq!(input.options[1].value, "sky");
        assert_eq!(
            input.options[1].hotkey,
            Some(biscuit_tui::HotkeySpec::Alt('b'))
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_emits_object_values_not_labels_for_json_source() {
        let path = write_temp_file(
            "choose_many_phase3_value_emit.json",
            r#"[{"label":"Red","value":"apple"},{"label":"Blue","value":"sky"}]"#,
        );
        let args = ChooseManyArgs {
            source: source_file(path.clone()),
            selected: vec!["apple".into(), "sky".into()],
            ..default_args()
        };
        let mut output = Vec::new();
        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| {
                assert_eq!(state.selected_ids(), vec!["apple", "sky"]);
                Ok(state.selected_values().into_iter().cloned().collect())
            },
        )
        .unwrap();
        assert_eq!(status, 0);
        assert_eq!(output, b"apple\nsky\n");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_returns_1_without_output_on_esc() {
        let args = ChooseManyArgs {
            source: source_csv("Pepperoni,Olives".into()),
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
