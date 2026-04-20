//! `question boolean-switch` subcommand.
//!
//! Maps CLI args onto a [`tui_chrome::BooleanSwitchState`], runs the
//! component via [`tui_chrome::run_standalone`], and writes the
//! captured boolean according to the current [`OutputMode`].

use std::io::{self, Write};

use clap::Args;
use tui_chrome::{BooleanSwitch, BooleanSwitchState, CANCELLED_KIND, Label, run_standalone};

use crate::commands::text_input::LabelPositionArg;
use crate::output::{OutputMode, write_scalar};

/// Arguments accepted by the `boolean-switch` subcommand.
#[derive(Debug, Args)]
pub struct BooleanSwitchArgs {
    /// Label text rendered next to the switch.
    #[arg(long)]
    pub label: Option<String>,

    /// Where the label renders relative to the switch body.
    #[arg(long, value_enum, default_value_t = LabelPositionArg::Above)]
    pub label_position: LabelPositionArg,

    /// Initial checked value (`true` or `false`). Defaults to `false`.
    #[arg(long)]
    pub initial: Option<bool>,

    /// Custom on/off captions in `"on,off"` form (e.g. `--labels YES,NO`).
    #[arg(long)]
    pub labels: Option<String>,
}

/// Runs the `boolean-switch` subcommand.
///
/// ## Returns
///
/// `Ok(0)` on submission, `Ok(130)` on cancellation, `Err` on a
/// terminal I/O error.
pub fn run(args: BooleanSwitchArgs, output: OutputMode, height: Option<u16>) -> io::Result<i32> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    run_with_writer(args, output, height, &mut lock, |state, height| {
        run_standalone(BooleanSwitch::new(), state, height)
    })
}

fn run_with_writer<F, W>(
    args: BooleanSwitchArgs,
    output: OutputMode,
    height: Option<u16>,
    writer: &mut W,
    run_prompt: F,
) -> io::Result<i32>
where
    F: FnOnce(BooleanSwitchState, Option<u16>) -> io::Result<bool>,
    W: Write,
{
    let mut state = BooleanSwitchState::new().with_value(args.initial.unwrap_or(false));

    if let Some(text) = args.label {
        state = state.with_label(Label::new(text, args.label_position.into()));
    }
    if let Some(labels) = args.labels.as_deref() {
        let (on, off) = parse_labels(labels)?;
        state = state.with_labels(on, off);
    }

    match run_prompt(state, height) {
        Ok(value) => {
            write_scalar(writer, bool_to_str(value), output)?;
            writer.flush()?;
            Ok(0)
        }
        Err(e) if e.kind() == CANCELLED_KIND => Ok(130),
        Err(e) => Err(e),
    }
}

fn bool_to_str(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn parse_labels(input: &str) -> io::Result<(String, String)> {
    let mut parts = input.splitn(2, ',');
    let on = parts.next().unwrap_or("").trim();
    let off = parts.next().unwrap_or("").trim();
    if on.is_empty() || off.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--labels expects 'on,off' with both values present",
        ));
    }
    Ok((on.to_string(), off.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bool_to_str_maps_true_and_false() {
        assert_eq!(bool_to_str(true), "true");
        assert_eq!(bool_to_str(false), "false");
    }

    #[test]
    fn parse_labels_splits_on_first_comma() {
        let (on, off) = parse_labels("YES,NO").unwrap();
        assert_eq!(on, "YES");
        assert_eq!(off, "NO");
    }

    #[test]
    fn parse_labels_trims_whitespace() {
        let (on, off) = parse_labels(" enabled , disabled ").unwrap();
        assert_eq!(on, "enabled");
        assert_eq!(off, "disabled");
    }

    #[test]
    fn parse_labels_rejects_missing_off() {
        let err = parse_labels("YES").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn parse_labels_rejects_empty_on() {
        let err = parse_labels(",NO").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn run_writes_json_boolean_string_from_initial_state() {
        let args = BooleanSwitchArgs {
            label: Some("Enabled".into()),
            label_position: LabelPositionArg::Right,
            initial: Some(true),
            labels: Some("YES,NO".into()),
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Json,
            Some(3),
            &mut output,
            |state, height| {
                assert_eq!(height, Some(3));
                assert!(state.checked());
                assert_eq!(
                    state.label().map(|label| label.text.as_str()),
                    Some("Enabled")
                );
                Ok(state.checked())
            },
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(String::from_utf8(output).unwrap(), "\"true\"\n");
    }

    #[test]
    fn run_writes_raw_true_with_newline() {
        let args = BooleanSwitchArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            initial: Some(true),
            labels: None,
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| Ok(state.checked()),
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"true\n");
    }

    #[test]
    fn run_writes_raw_false_with_newline() {
        let args = BooleanSwitchArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            initial: Some(false),
            labels: None,
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Raw,
            None,
            &mut output,
            |state, _height| Ok(state.checked()),
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"false\n");
    }

    #[test]
    fn run_writes_null_output_with_nul_terminator() {
        let args = BooleanSwitchArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            initial: Some(true),
            labels: None,
        };
        let mut output = Vec::new();

        let status = run_with_writer(
            args,
            OutputMode::Null,
            None,
            &mut output,
            |state, _height| Ok(state.checked()),
        )
        .unwrap();

        assert_eq!(status, 0);
        assert_eq!(output, b"true\0");
    }

    #[test]
    fn run_returns_130_without_output_on_cancel() {
        let args = BooleanSwitchArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            initial: Some(false),
            labels: None,
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
