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

    /// Render inline in `N` rows below the cursor instead of
    /// fullscreen.
    #[arg(long)]
    pub height: Option<u16>,
}

/// Runs the `boolean-switch` subcommand.
///
/// ## Returns
///
/// `Ok(0)` on submission, `Ok(130)` on cancellation, `Err` on a
/// terminal I/O error.
pub fn run(args: BooleanSwitchArgs, output: OutputMode) -> io::Result<i32> {
    let mut state = BooleanSwitchState::new().with_value(args.initial.unwrap_or(false));

    if let Some(text) = args.label {
        state = state.with_label(Label::new(text, args.label_position.into()));
    }
    if let Some(labels) = args.labels.as_deref() {
        let (on, off) = parse_labels(labels)?;
        state = state.with_labels(on, off);
    }

    match run_standalone(BooleanSwitch::new(), state, args.height) {
        Ok(value) => {
            let stdout = io::stdout();
            let mut lock = stdout.lock();
            write_scalar(&mut lock, bool_to_str(value), output)?;
            lock.flush()?;
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
}
