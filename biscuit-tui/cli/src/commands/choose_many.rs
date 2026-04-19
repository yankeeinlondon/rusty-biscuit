//! `question choose-many` subcommand.
//!
//! Maps CLI args onto a [`tui_chrome::ChooseManyState`], runs the
//! component via [`tui_chrome::run_standalone`], and writes the
//! captured option values according to the current [`OutputMode`].

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args;
use tui_chrome::{
    CANCELLED_KIND, ChooseMany, ChooseManyState, Label, run_standalone,
};
use tui_chrome::helpers::choice_builders::{
    choose_many_from_csv, choose_many_from_markdown_list, choose_one_from_dictionary,
};

use crate::commands::text_input::LabelPositionArg;
use crate::output::{OutputMode, write_list};

/// Arguments accepted by the `choose-many` subcommand.
#[derive(Debug, Args)]
pub struct ChooseManyArgs {
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

    /// Comma-separated list of pre-selected option ids.
    #[arg(long)]
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

    /// Render inline in `N` rows below the cursor instead of
    /// fullscreen.
    #[arg(long)]
    pub height: Option<u16>,
}

/// Runs the `choose-many` subcommand.
///
/// ## Returns
///
/// `Ok(0)` on submission, `Ok(130)` on cancellation, `Err` on a
/// terminal I/O error.
pub fn run(args: ChooseManyArgs, output: OutputMode) -> io::Result<i32> {
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

    let mut state = ChooseManyState::new(input);
    if let Some(text) = args.label {
        state = state.with_label(Label::new(text, args.label_position.into()));
    }
    if let Some(initial) = args.initial.as_deref() {
        let ids = parse_initial_ids(initial);
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        state = state.with_initial_selection(&refs);
    }

    match run_standalone(ChooseMany::new(), state, args.height) {
        Ok(values) => {
            let stdout = io::stdout();
            let mut lock = stdout.lock();
            write_list(&mut lock, &values, output)?;
            lock.flush()?;
            Ok(0)
        }
        Err(e) if e.kind() == CANCELLED_KIND => Ok(130),
        Err(e) => Err(e),
    }
}

fn build_choice_input(args: &ChooseManyArgs) -> io::Result<tui_chrome::ChoiceInput<String>> {
    if let Some(csv) = args.options.as_deref() {
        return Ok(choose_many_from_csv("choice", "", csv));
    }
    if let Some(path) = args.options_from_file.as_ref() {
        let body = fs::read_to_string(path)?;
        return Ok(choose_many_from_markdown_list("choice", "", &body));
    }
    if let Some(path) = args.options_from_dictionary.as_ref() {
        let body = fs::read_to_string(path)?;
        // choose_many variant: reuse choose_one_from_dictionary then
        // flip selection mode.
        let input = choose_one_from_dictionary("choice", "", &body)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        return Ok(input.with_selection_mode(tui_chrome::SelectionMode::Multiple));
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "one of --options, --options-from-file, or --options-from-dictionary is required",
    ))
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
            options: None,
            options_from_file: None,
            options_from_dictionary: None,
            label: None,
            label_position: LabelPositionArg::Above,
            initial: None,
            required: false,
            min_selections: None,
            max_selections: None,
            height: None,
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
        assert_eq!(input.selection_mode, tui_chrome::SelectionMode::Multiple);
    }

    #[test]
    fn parse_initial_ids_trims_and_drops_empty() {
        let ids = parse_initial_ids("a, b , ,c");
        assert_eq!(ids, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[test]
    fn build_choice_input_without_source_errors() {
        let args = default_args();
        let err = build_choice_input(&args).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
