//! `question choose-one` subcommand.
//!
//! Maps CLI args onto a [`tui_chrome::ChooseOneState`], runs the
//! component via [`tui_chrome::run_standalone`], and writes the
//! captured option value according to the current [`OutputMode`].

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args;
use tui_chrome::helpers::choice_builders::{
    choose_one_from_csv, choose_one_from_dictionary, choose_one_from_markdown_list,
};
use tui_chrome::{CANCELLED_KIND, ChooseOne, ChooseOneState, Label, run_standalone};

use crate::commands::text_input::LabelPositionArg;
use crate::output::{OutputMode, write_scalar};

/// Arguments accepted by the `choose-one` subcommand.
#[derive(Debug, Args)]
pub struct ChooseOneArgs {
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

    /// Pre-selected option id.
    #[arg(long)]
    pub initial: Option<String>,

    /// Submit fails validation when no selection is made.
    #[arg(long)]
    pub required: bool,
}

/// Runs the `choose-one` subcommand.
///
/// ## Returns
///
/// `Ok(0)` on submission, `Ok(130)` on cancellation, `Err` on a
/// terminal I/O error.
pub fn run(args: ChooseOneArgs, output: OutputMode, height: Option<u16>) -> io::Result<i32> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    run_with_writer(args, output, height, &mut lock, |state, height| {
        run_standalone(ChooseOne::new(), state, height)
    })
}

fn run_with_writer<F, W>(
    args: ChooseOneArgs,
    output: OutputMode,
    height: Option<u16>,
    writer: &mut W,
    run_prompt: F,
) -> io::Result<i32>
where
    F: FnOnce(ChooseOneState, Option<u16>) -> io::Result<Option<String>>,
    W: Write,
{
    let mut input = build_choice_input(&args)?;
    if args.required {
        input = input.required();
    }

    let mut state = ChooseOneState::new(input);
    if let Some(text) = args.label {
        state = state.with_label(Label::new(text, args.label_position.into()));
    }
    if let Some(id) = args.initial.as_deref() {
        state = state.with_initial_selection(id);
    }

    match run_prompt(state, height) {
        Ok(value) => {
            let rendered = value.unwrap_or_default();
            write_scalar(writer, &rendered, output)?;
            writer.flush()?;
            Ok(0)
        }
        Err(e) if e.kind() == CANCELLED_KIND => Ok(130),
        Err(e) => Err(e),
    }
}

fn build_choice_input(args: &ChooseOneArgs) -> io::Result<tui_chrome::ChoiceInput<String>> {
    if let Some(csv) = args.options.as_deref() {
        return Ok(choose_one_from_csv("choice", "", csv));
    }
    if let Some(path) = args.options_from_file.as_ref() {
        let body = fs::read_to_string(path)?;
        return Ok(choose_one_from_markdown_list("choice", "", &body));
    }
    if let Some(path) = args.options_from_dictionary.as_ref() {
        let body = fs::read_to_string(path)?;
        return choose_one_from_dictionary("choice", "", &body)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()));
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "one of --options, --options-from-file, or --options-from-dictionary is required",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_args() -> ChooseOneArgs {
        ChooseOneArgs {
            options: None,
            options_from_file: None,
            options_from_dictionary: None,
            label: None,
            label_position: LabelPositionArg::Above,
            initial: None,
            required: false,
        }
    }

    #[test]
    fn build_choice_input_from_csv_returns_options() {
        let args = ChooseOneArgs {
            options: Some("Red,Green,Blue".into()),
            ..default_args()
        };
        let input = build_choice_input(&args).unwrap();
        assert_eq!(input.options.len(), 3);
    }

    #[test]
    fn build_choice_input_without_source_errors() {
        let args = default_args();
        let err = build_choice_input(&args).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn run_writes_json_selected_value_from_initial_option() {
        let args = ChooseOneArgs {
            options: Some("Red,Green,Blue".into()),
            initial: Some("Green".into()),
            required: true,
            ..default_args()
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Json,
            Some(5),
            &mut output,
            |state, height| {
                assert_eq!(height, Some(5));
                assert_eq!(state.selected_id(), Some("Green"));
                Ok(state.selected_value().cloned())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(String::from_utf8(output).unwrap(), "\"Green\"\n");
    }

    #[test]
    fn run_writes_raw_selected_value_with_newline() {
        let args = ChooseOneArgs {
            options: Some("Alpha,Beta,Gamma".into()),
            initial: Some("Beta".into()),
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
    fn run_writes_null_output_with_nul_terminator() {
        let args = ChooseOneArgs {
            options: Some("X,Y,Z".into()),
            initial: Some("Z".into()),
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
    fn run_returns_130_without_output_on_cancel() {
        let args = ChooseOneArgs {
            options: Some("Red,Green".into()),
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
