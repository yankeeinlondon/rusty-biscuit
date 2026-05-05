//! `question choose-one` subcommand.
//!
//! Maps CLI args onto a [`tui_chrome::ChooseOneState`], runs the
//! component via [`tui_chrome::run_standalone`], and writes the
//! captured option value according to the current [`OutputMode`].

use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args;
use tui_chrome::{
    ABORTED_KIND, CANCELLED_KIND, ChoiceInput, ChooseOne, ChooseOneState, HeightSpec, Label,
    run_standalone_with_chrome,
};

use crate::choice_normalize::normalize_options;
use crate::commands::common_choose::{ChooseChromeArgs, build_chrome, resolve_hotkey_badges};
use crate::commands::text_input::LabelPositionArg;
use crate::option_sources::resolve_raw_options;
use crate::output::{OutputMode, write_scalar};

/// Arguments accepted by the `choose-one` subcommand.
#[derive(Debug, Args)]
pub struct ChooseOneArgs {
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

    /// Legacy: path to a YAML/JSON file containing a mapping of label → value.
    #[arg(long, hide = true)]
    pub options_from_dictionary: Option<PathBuf>,

    /// Label text rendered next to the list.
    #[arg(long)]
    pub label: Option<String>,

    /// Where the label renders relative to the list body.
    #[arg(long, value_enum, default_value_t = LabelPositionArg::Above)]
    pub label_position: LabelPositionArg,

    /// Pre-selected option value. Matches by the option's `value`
    /// field (equal to the label when no `--delimiter` is set).
    #[arg(long, conflicts_with = "initial")]
    pub selected: Option<String>,

    /// Deprecated alias for `--selected`. Will be removed in a future
    /// release.
    #[arg(long, hide = true)]
    pub initial: Option<String>,

    /// Submit fails validation when no selection is made.
    #[arg(long)]
    pub required: bool,

    #[command(flatten)]
    pub chrome: ChooseChromeArgs,
}

/// Runs the `choose-one` subcommand.
///
/// ## Returns
///
/// `Ok(0)` on submission, `Ok(1)` when the user pressed `Esc`,
/// `Ok(130)` when the user pressed `Ctrl-C`, `Err` on a terminal I/O
/// error.
pub fn run(
    args: ChooseOneArgs,
    output: OutputMode,
    height: Option<HeightSpec>,
    show_on_exit: bool,
) -> io::Result<i32> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let mut chrome = build_chrome(&args.chrome);
    chrome.show_on_exit = show_on_exit;
    run_with_writer(args, output, height, &mut lock, |state, height| {
        run_standalone_with_chrome(ChooseOne::new(), state, height, chrome)
    })
}

fn run_with_writer<F, W>(
    args: ChooseOneArgs,
    output: OutputMode,
    height: Option<HeightSpec>,
    writer: &mut W,
    run_prompt: F,
) -> io::Result<i32>
where
    F: FnOnce(ChooseOneState, Option<HeightSpec>) -> io::Result<Option<String>>,
    W: Write,
{
    let mut input = build_choice_input(&args)?;
    if args.required {
        input = input.required();
    }

    let selected = effective_selected(&args).map(str::to_string);
    let hotkey_override = resolve_hotkey_badges(args.chrome.hotkey_badges);
    let mut state = ChooseOneState::new(input);
    if let Some(text) = args.label {
        state = state.with_label(Label::new(text, args.label_position.into()));
    }
    if let Some(value) = selected.as_deref() {
        state = state.with_initial_value(value);
    }
    if let Some(mode) = hotkey_override {
        state = state.with_hotkey_display(mode);
    }

    match run_prompt(state, height) {
        Ok(value) => {
            let rendered = value.unwrap_or_default();
            write_scalar(writer, &rendered, output)?;
            writer.flush()?;
            Ok(0)
        }
        Err(e) if e.kind() == CANCELLED_KIND => Ok(130),
        Err(e) if e.kind() == ABORTED_KIND => Ok(1),
        Err(e) => Err(e),
    }
}

fn effective_selected(args: &ChooseOneArgs) -> Option<&str> {
    if let Some(selected) = args.selected.as_deref() {
        return Some(selected);
    }
    if let Some(initial) = args.initial.as_deref() {
        eprintln!("question: warning: --initial is deprecated; use --selected instead");
        return Some(initial);
    }
    None
}

fn build_choice_input(args: &ChooseOneArgs) -> io::Result<ChoiceInput<String>> {
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

    let input = ChoiceInput::new("choice", "")
        .with_options(options)
        .with_sort(args.chrome.sort.into())
        .with_active_color(args.chrome.active_color.into())
        .with_orientation(args.chrome.orientation.into());
    Ok(input.with_filter_enabled(!args.chrome.no_filter))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_args() -> ChooseOneArgs {
        ChooseOneArgs {
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
            selected: None,
            initial: None,
            required: false,
            chrome: ChooseChromeArgs::default(),
        }
    }

    #[test]
    fn build_choice_input_from_csv_returns_options() {
        let args = ChooseOneArgs {
            csv: Some("Red,Green,Blue".into()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 3);
    }

    #[test]
    fn build_choice_input_from_list_strips_markdown_bullets() {
        let args = ChooseOneArgs {
            list: Some("- Red\n* Green\n1) Blue\n".into()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        let labels: Vec<&str> = input.options.iter().map(|o| o.label.as_str()).collect();
        let values: Vec<&str> = input.options.iter().map(|o| o.value.as_str()).collect();
        assert_eq!(labels, vec!["Red", "Green", "Blue"]);
        assert_eq!(values, vec!["Red", "Green", "Blue"]);
    }

    #[test]
    fn build_choice_input_from_positional_args() {
        let args = ChooseOneArgs {
            positional: vec!["alpha".into(), "beta".into(), "gamma".into()],
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 3);
        assert_eq!(input.options[0].label, "alpha");
        assert_eq!(input.options[0].value, "alpha");
    }

    #[test]
    fn build_choice_input_enables_filter_for_positional_args_by_default() {
        let args = ChooseOneArgs {
            positional: vec!["alpha".into(), "beta".into()],
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert!(input.filter_enabled);
    }

    #[test]
    fn build_choice_input_enables_filter_for_legacy_csv_by_default() {
        let args = ChooseOneArgs {
            csv: Some("alpha,beta".into()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert!(input.filter_enabled);
    }

    #[test]
    fn build_choice_input_respects_no_filter_flag() {
        let args = ChooseOneArgs {
            positional: vec!["alpha".into(), "beta".into()],
            chrome: ChooseChromeArgs {
                no_filter: true,
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert!(!input.filter_enabled);
    }

    #[test]
    fn build_choice_input_from_positional_with_delimiter_splits_label_value() {
        let args = ChooseOneArgs {
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
    fn build_choice_input_applies_sort_ascending_across_positional_source() {
        // Sorting now happens inside `ChooseOneState::new`, driven by
        // `ChoiceInput::with_sort`. The CLI builder must therefore
        // configure `sort` so that constructing a state yields the
        // expected order.
        use crate::commands::common_choose::SortOrderArg;
        let args = ChooseOneArgs {
            positional: vec!["Berry".into(), "Apple".into(), "Cherry".into()],
            chrome: ChooseChromeArgs {
                sort: SortOrderArg::Asc,
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.sort, Some(tui_chrome::SortOrder::Asc));
        let state = ChooseOneState::new(input);
        let labels: Vec<&str> = state.options().iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, vec!["Apple", "Berry", "Cherry"]);
    }

    #[test]
    fn build_choice_input_applies_sort_ascending_across_legacy_csv_source() {
        use crate::commands::common_choose::SortOrderArg;
        let args = ChooseOneArgs {
            csv: Some("Berry,Apple,Cherry".into()),
            chrome: ChooseChromeArgs {
                sort: SortOrderArg::Asc,
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.sort, Some(tui_chrome::SortOrder::Asc));
        let state = ChooseOneState::new(input);
        let labels: Vec<&str> = state.options().iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, vec!["Apple", "Berry", "Cherry"]);
    }

    #[test]
    fn build_choice_input_without_source_errors() {
        let args = default_args();
        let err = build_choice_input(&args).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn effective_selected_prefers_selected_over_initial() {
        let args = ChooseOneArgs {
            selected: Some("new".into()),
            initial: None,
            ..default_args()
        };
        assert_eq!(effective_selected(&args), Some("new"));
    }

    #[test]
    fn effective_selected_falls_back_to_initial_when_only_initial_set() {
        let args = ChooseOneArgs {
            selected: None,
            initial: Some("legacy".into()),
            ..default_args()
        };
        assert_eq!(effective_selected(&args), Some("legacy"));
    }

    #[test]
    fn effective_selected_is_none_when_neither_set() {
        let args = default_args();
        assert!(effective_selected(&args).is_none());
    }

    #[test]
    fn run_writes_json_selected_value_from_initial_option() {
        let args = ChooseOneArgs {
            csv: Some("Red,Green,Blue".into()),
            selected: Some("Green".into()),
            required: true,
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Json,
            Some(HeightSpec::Cells(5)),
            &mut output,
            |state, height| {
                assert_eq!(height, Some(HeightSpec::Cells(5)));
                assert_eq!(state.selected_id(), Some("Green"));
                Ok(state.selected_value().cloned())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(String::from_utf8(output).unwrap(), "\"Green\"\n");
    }

    #[test]
    fn run_propagates_percent_height_to_prompt() {
        let args = ChooseOneArgs {
            csv: Some("A,B,C".into()),
            selected: Some("B".into()),
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            Some(HeightSpec::Percent(50)),
            &mut output,
            |state, height| {
                assert_eq!(height, Some(HeightSpec::Percent(50)));
                Ok(state.selected_value().cloned())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"B\n");
    }

    #[test]
    fn run_writes_raw_selected_value_with_newline() {
        let args = ChooseOneArgs {
            csv: Some("Alpha,Beta,Gamma".into()),
            selected: Some("Beta".into()),
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| Ok(state.selected_value().cloned()),
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"Beta\n");
    }

    #[test]
    fn run_writes_hovered_value_from_positional_args() {
        let args = ChooseOneArgs {
            positional: vec!["alpha".into(), "beta".into(), "gamma".into()],
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| {
                assert_eq!(state.hover(), Some(0));
                assert_eq!(state.options()[0].label, "alpha");
                assert_eq!(state.options()[0].value, "alpha");
                Ok(state.options()[state.hover().unwrap()].value.clone().into())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"alpha\n");
    }

    #[test]
    fn run_writes_delimited_positional_value() {
        let args = ChooseOneArgs {
            positional: vec!["Apple:1".into(), "Berry:2".into()],
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
                assert_eq!(state.options()[0].label, "Apple");
                assert_eq!(state.options()[0].id, "1");
                assert_eq!(state.options()[0].value, "1");
                Ok(state.options()[0].value.clone().into())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"1\n");
    }

    #[test]
    fn run_selected_default_matches_delimited_value() {
        let args = ChooseOneArgs {
            positional: vec!["Apple:1".into(), "Berry:2".into()],
            selected: Some("2".into()),
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
                assert_eq!(state.selected_id(), Some("2"));
                assert_eq!(state.hover(), Some(1));
                assert_eq!(state.options()[1].label, "Berry");
                Ok(state.selected_value().cloned())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(String::from_utf8(output).unwrap(), "\"2\"\n");
    }

    #[test]
    fn run_builds_filter_enabled_state_by_default() {
        let args = ChooseOneArgs {
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
                Ok(Some(state.options()[0].value.clone()))
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"alpha\n");
    }

    #[test]
    fn run_writes_null_output_with_nul_terminator() {
        let args = ChooseOneArgs {
            csv: Some("X,Y,Z".into()),
            selected: Some("Z".into()),
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Null,
            None,
            &mut output,
            |state, _height| Ok(state.selected_value().cloned()),
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"Z\0");
    }

    #[test]
    fn run_returns_130_without_output_on_ctrl_c() {
        let args = ChooseOneArgs {
            csv: Some("Red,Green".into()),
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

    /// Helper that writes the given body to a temp file with the given
    /// extension and returns the resulting path. The caller is
    /// responsible for cleanup.
    fn write_temp_file(name: &str, body: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn build_choice_input_from_json_object_array_preserves_value_and_hotkey() {
        let path = write_temp_file(
            "choose_one_phase3_objects.json",
            r#"[{"label":"Red","value":"apple","hotkey":"CTRL+R"},{"label":"Blue","value":"sky"}]"#,
        );
        let args = ChooseOneArgs {
            file: Some(path.clone()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].label, "Red");
        assert_eq!(input.options[0].value, "apple");
        assert_eq!(
            input.options[0].hotkey,
            Some(tui_chrome::HotkeySpec::Ctrl('r'))
        );
        assert_eq!(input.options[1].label, "Blue");
        assert_eq!(input.options[1].value, "sky");
        assert!(input.options[1].hotkey.is_none());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn build_choice_input_from_yaml_object_array_preserves_value_and_hotkey() {
        let body = "- label: Red\n  value: apple\n  hotkey: CTRL+R\n- label: Blue\n  value: sky\n  hotkey: ALT+B\n";
        let path = write_temp_file("choose_one_phase3_objects.yaml", body);
        let args = ChooseOneArgs {
            file: Some(path.clone()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].label, "Red");
        assert_eq!(input.options[0].value, "apple");
        assert_eq!(
            input.options[0].hotkey,
            Some(tui_chrome::HotkeySpec::Ctrl('r'))
        );
        assert_eq!(input.options[1].label, "Blue");
        assert_eq!(input.options[1].value, "sky");
        assert_eq!(
            input.options[1].hotkey,
            Some(tui_chrome::HotkeySpec::Alt('b'))
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn build_choice_input_from_jsonl_object_lines_preserves_value_and_hotkey() {
        let body = "{\"label\":\"Red\",\"value\":\"apple\",\"hotkey\":\"CTRL+R\"}\n{\"label\":\"Blue\",\"value\":\"sky\"}\n";
        let path = write_temp_file("choose_one_phase3_objects.jsonl", body);
        let args = ChooseOneArgs {
            file: Some(path.clone()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].value, "apple");
        assert_eq!(
            input.options[0].hotkey,
            Some(tui_chrome::HotkeySpec::Ctrl('r'))
        );
        assert_eq!(input.options[1].value, "sky");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn build_choice_input_from_ndjson_object_lines_preserves_value_and_hotkey() {
        let body = "{\"label\":\"Red\",\"value\":\"apple\",\"hotkey\":\"CTRL+R\"}\n";
        let path = write_temp_file("choose_one_phase3_objects.ndjson", body);
        let args = ChooseOneArgs {
            file: Some(path.clone()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 1);
        assert_eq!(input.options[0].value, "apple");
        assert_eq!(
            input.options[0].hotkey,
            Some(tui_chrome::HotkeySpec::Ctrl('r'))
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn build_choice_input_from_csv_three_columns_preserves_value_and_hotkey() {
        let body = "Red,apple,CTRL+R\nBlue,sky,ALT+B\n";
        let path = write_temp_file("choose_one_phase3_objects.csv", body);
        let args = ChooseOneArgs {
            file: Some(path.clone()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].label, "Red");
        assert_eq!(input.options[0].value, "apple");
        assert_eq!(
            input.options[0].hotkey,
            Some(tui_chrome::HotkeySpec::Ctrl('r'))
        );
        assert_eq!(input.options[1].label, "Blue");
        assert_eq!(input.options[1].value, "sky");
        assert_eq!(
            input.options[1].hotkey,
            Some(tui_chrome::HotkeySpec::Alt('b'))
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn build_choice_input_from_markdown_frontmatter_object_array_preserves_fields() {
        let body = "---\nitems:\n  - label: Red\n    value: apple\n    hotkey: CTRL+R\n  - label: Blue\n    value: sky\n---\n# Title\n";
        let path = write_temp_file("choose_one_phase3_objects.md", body);
        let args = ChooseOneArgs {
            md: Some(vec![
                path.to_string_lossy().to_string(),
                "items".to_string(),
            ]),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].label, "Red");
        assert_eq!(input.options[0].value, "apple");
        assert_eq!(
            input.options[0].hotkey,
            Some(tui_chrome::HotkeySpec::Ctrl('r'))
        );
        assert_eq!(input.options[1].label, "Blue");
        assert_eq!(input.options[1].value, "sky");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_emits_object_value_not_label_for_json_source() {
        // Anchor for review-1 finding 3: when the JSON object supplies
        // value="apple", the CLI must echo "apple" — not the label
        // "Red" — when the option is selected.
        let path = write_temp_file(
            "choose_one_phase3_value_emit.json",
            r#"[{"label":"Red","value":"apple"}]"#,
        );
        let args = ChooseOneArgs {
            file: Some(path.clone()),
            selected: Some("apple".into()),
            ..default_args()
        };
        let mut output = Vec::new();
        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| {
                assert_eq!(state.options()[0].label, "Red");
                assert_eq!(state.options()[0].value, "apple");
                assert_eq!(state.selected_id(), Some("apple"));
                Ok(state.selected_value().cloned())
            },
        )
        .unwrap();
        assert_eq!(status, 0);
        assert_eq!(output, b"apple\n");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn build_choice_input_active_color_default_is_grey() {
        // Phase 5: omitting --active-color resolves to ActiveChoiceColor::Grey
        let args = ChooseOneArgs {
            csv: Some("Red,Green".into()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.active_color, tui_chrome::ActiveChoiceColor::Grey);
    }

    #[test]
    fn build_choice_input_active_color_green_propagates_to_input() {
        use crate::commands::common_choose::ActiveColorArg;
        let args = ChooseOneArgs {
            csv: Some("Red,Green".into()),
            chrome: ChooseChromeArgs {
                active_color: ActiveColorArg::Green,
                ..ChooseChromeArgs::default()
            },
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.active_color, tui_chrome::ActiveChoiceColor::Green);
    }

    #[test]
    fn run_returns_0_on_esc_with_restored_value() {
        // Phase 5: ChooseOne Esc restores the initial selection and
        // submits, so the CLI exits 0 rather than 1.
        let args = ChooseOneArgs {
            csv: Some("Red,Green".into()),
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |_state, _height| Ok(Some("Red".into())),
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"Red\n");
    }
}
