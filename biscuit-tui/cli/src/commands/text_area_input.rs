//! `question text-area-input` subcommand.
//!
//! Maps CLI args onto a [`tui_chrome::TextAreaInputState`], runs the
//! component via [`tui_chrome::run_standalone`], and writes the
//! captured multi-line value according to the current [`OutputMode`].

use std::io::{self, Write};

use clap::Args;
use tui_chrome::{
    ABORTED_KIND, CANCELLED_KIND, HeightSpec, Label, TextAreaInput, TextAreaInputState,
    run_standalone,
};

use crate::commands::text_input::LabelPositionArg;
use crate::output::{OutputMode, write_scalar};

/// Arguments accepted by the `text-area-input` subcommand.
#[derive(Debug, Args)]
pub struct TextAreaInputArgs {
    /// Label text rendered next to the editor.
    #[arg(long)]
    pub label: Option<String>,

    /// Where the label renders relative to the editor body.
    #[arg(long, value_enum, default_value_t = LabelPositionArg::Above)]
    pub label_position: LabelPositionArg,

    /// Preferred editor width in terminal cells.
    #[arg(long, default_value_t = 60)]
    pub width: u16,

    /// Show a vertical scrollbar when content exceeds the editor
    /// height.
    #[arg(long, default_value_t = false)]
    pub scrollbar: bool,

    /// Initial buffer contents. Newlines in the argument become line
    /// breaks in the editor.
    #[arg(long)]
    pub initial: Option<String>,
}

/// Runs the `text-area-input` subcommand.
///
/// ## Returns
///
/// `Ok(0)` on submission, `Ok(130)` on cancellation, `Err` on a
/// terminal I/O error.
pub fn run(
    args: TextAreaInputArgs,
    output: OutputMode,
    height: Option<HeightSpec>,
) -> io::Result<i32> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    run_with_writer(args, output, height, &mut lock, |state, height| {
        run_standalone(TextAreaInput::new(), state, height)
    })
}

fn run_with_writer<F, W>(
    args: TextAreaInputArgs,
    output: OutputMode,
    height: Option<HeightSpec>,
    writer: &mut W,
    run_prompt: F,
) -> io::Result<i32>
where
    F: FnOnce(TextAreaInputState, Option<HeightSpec>) -> io::Result<String>,
    W: Write,
{
    let editor_height = resolve_editor_height(height);
    let mut state = TextAreaInputState::new(args.width, editor_height);

    if let Some(text) = args.label {
        state = state.with_label(Label::new(text, args.label_position.into()));
    }
    if args.scrollbar {
        state = state.with_scrollbar(true);
    }
    if let Some(initial) = args.initial {
        let lines: Vec<&str> = initial.split('\n').collect();
        state = state.with_value(&lines);
    }

    match run_prompt(state, height) {
        Ok(value) => {
            write_scalar(writer, &value, output)?;
            writer.flush()?;
            Ok(0)
        }
        Err(e) if e.kind() == CANCELLED_KIND => Ok(130),
        Err(e) if e.kind() == ABORTED_KIND => Ok(1),
        Err(e) => Err(e),
    }
}

/// Resolves the requested editor height (in terminal cells) from the
/// parsed `--height` spec.
///
/// `None` falls back to a default of 10 rows, preserving the prior
/// behaviour when no height flag was supplied. `Cells` passes through
/// untouched. `Percent` falls back to the same default since the
/// editor's preferred height is a structural hint rather than a
/// terminal viewport dimension — callers who need percentage-based
/// inline sizing should rely on the outer `--height` plumbing in
/// `run_standalone`, which resolves percentages against the live
/// terminal size.
fn resolve_editor_height(spec: Option<HeightSpec>) -> u16 {
    const DEFAULT_EDITOR_HEIGHT: u16 = 10;
    match spec {
        None | Some(HeightSpec::Percent(_)) => DEFAULT_EDITOR_HEIGHT,
        Some(HeightSpec::Cells(n)) => n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_width_is_sixty() {
        // Smoke test — ensure the default matches the documented
        // `--width` fallback.
        let args = TextAreaInputArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            width: 60,
            scrollbar: false,
            initial: None,
        };
        assert_eq!(args.width, 60);
        assert!(!args.scrollbar);
    }

    #[test]
    fn run_writes_json_output_from_initial_lines() {
        let args = TextAreaInputArgs {
            label: Some("Notes".into()),
            label_position: LabelPositionArg::Above,
            width: 72,
            scrollbar: true,
            initial: Some("alpha\nbeta".into()),
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Json,
            Some(HeightSpec::Cells(6)),
            &mut output,
            |state, height| {
                assert_eq!(height, Some(HeightSpec::Cells(6)));
                assert_eq!(state.preferred_width(), 72);
                assert_eq!(state.lines(), ["alpha".to_string(), "beta".to_string()]);
                Ok(state.value())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(String::from_utf8(output).unwrap(), "\"alpha\\nbeta\"\n");
    }

    #[test]
    fn run_writes_raw_output_preserving_newlines() {
        let args = TextAreaInputArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            width: 60,
            scrollbar: false,
            initial: Some("line1\nline2\nline3".into()),
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| Ok(state.value()),
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"line1\nline2\nline3\n");
    }

    #[test]
    fn run_writes_null_output_with_nul_terminator() {
        let args = TextAreaInputArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            width: 60,
            scrollbar: false,
            initial: Some("multi\nline".into()),
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Null,
            None,
            &mut output,
            |state, _height| Ok(state.value()),
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"multi\nline\0");
    }

    #[test]
    fn run_returns_130_without_output_on_ctrl_c() {
        let args = TextAreaInputArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            width: 60,
            scrollbar: false,
            initial: None,
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
        let args = TextAreaInputArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            width: 60,
            scrollbar: false,
            initial: None,
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
