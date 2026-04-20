//! `question text-input` subcommand.
//!
//! Maps CLI args onto a [`tui_chrome::TextInputState`], runs the
//! component via [`tui_chrome::run_standalone`], and writes the
//! captured value according to the current [`OutputMode`].

use std::io::{self, Write};

use clap::{Args, ValueEnum};
use tui_chrome::{CANCELLED_KIND, Label, LabelPosition, TextInput, TextInputState, run_standalone};

use crate::output::{OutputMode, write_scalar};

/// Arguments accepted by the `text-input` subcommand.
#[derive(Debug, Args)]
pub struct TextInputArgs {
    /// Label text rendered next to the input.
    #[arg(long)]
    pub label: Option<String>,

    /// Where the label renders relative to the input body.
    #[arg(long, value_enum, default_value_t = LabelPositionArg::Above)]
    pub label_position: LabelPositionArg,

    /// Maximum number of characters the input will accept.
    #[arg(long)]
    pub max_length: Option<usize>,

    /// Initial buffer contents.
    #[arg(long)]
    pub initial: Option<String>,
}

/// CLI-facing label position mirror.
///
/// Kept separate from [`LabelPosition`] so that clap's derive can
/// render kebab-case help values without us depending on clap in the
/// library.
#[derive(Debug, Clone, Copy, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum LabelPositionArg {
    Above,
    Below,
    Left,
    Right,
}

impl From<LabelPositionArg> for LabelPosition {
    fn from(value: LabelPositionArg) -> Self {
        match value {
            LabelPositionArg::Above => LabelPosition::Above,
            LabelPositionArg::Below => LabelPosition::Below,
            LabelPositionArg::Left => LabelPosition::Left,
            LabelPositionArg::Right => LabelPosition::Right,
        }
    }
}

/// Runs the `text-input` subcommand.
///
/// ## Returns
///
/// `Ok(0)` on submission, `Ok(130)` on cancellation, `Err` on a
/// terminal I/O error.
pub fn run(args: TextInputArgs, output: OutputMode, height: Option<u16>) -> io::Result<i32> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    run_with_writer(args, output, height, &mut lock, |state, height| {
        run_standalone(TextInput::new(), state, height)
    })
}

fn run_with_writer<F, W>(
    args: TextInputArgs,
    output: OutputMode,
    height: Option<u16>,
    writer: &mut W,
    run_prompt: F,
) -> io::Result<i32>
where
    F: FnOnce(TextInputState, Option<u16>) -> io::Result<String>,
    W: Write,
{
    let mut state = TextInputState::new();

    if let Some(text) = args.label {
        state = state.with_label(Label::new(text, args.label_position.into()));
    }
    if let Some(max) = args.max_length {
        state = state.with_max_length(max);
    }
    if let Some(initial) = args.initial {
        state = state.with_value(&initial);
    }

    match run_prompt(state, height) {
        Ok(value) => {
            write_scalar(writer, &value, output)?;
            writer.flush()?;
            Ok(0)
        }
        Err(e) if e.kind() == CANCELLED_KIND => Ok(130),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_position_arg_maps_to_library_enum() {
        assert_eq!(
            LabelPosition::from(LabelPositionArg::Above),
            LabelPosition::Above
        );
        assert_eq!(
            LabelPosition::from(LabelPositionArg::Below),
            LabelPosition::Below
        );
        assert_eq!(
            LabelPosition::from(LabelPositionArg::Left),
            LabelPosition::Left
        );
        assert_eq!(
            LabelPosition::from(LabelPositionArg::Right),
            LabelPosition::Right
        );
    }

    #[test]
    fn run_writes_json_output_from_initial_value() {
        let args = TextInputArgs {
            label: Some("Name".into()),
            label_position: LabelPositionArg::Left,
            max_length: Some(10),
            initial: Some("Ada".into()),
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Json,
            Some(4),
            &mut output,
            |state, height| {
                assert_eq!(height, Some(4));
                assert_eq!(state.value(), "Ada");
                assert_eq!(state.label().map(|label| label.text.as_str()), Some("Name"));
                Ok(state.value().to_string())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(String::from_utf8(output).unwrap(), "\"Ada\"\n");
    }

    #[test]
    fn run_writes_raw_output_with_trailing_newline() {
        let args = TextInputArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            max_length: None,
            initial: Some("hello".into()),
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| Ok(state.value().to_string()),
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"hello\n");
    }

    #[test]
    fn run_writes_null_output_with_nul_terminator() {
        let args = TextInputArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            max_length: None,
            initial: Some("data".into()),
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Null,
            None,
            &mut output,
            |state, _height| Ok(state.value().to_string()),
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"data\0");
    }

    #[test]
    fn run_returns_130_without_output_on_cancel() {
        let args = TextInputArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            max_length: None,
            initial: Some("ignored".into()),
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
