//! `question text-input` subcommand.
//!
//! Maps CLI args onto a [`tui_chrome::TextInputState`], runs the
//! component via [`tui_chrome::run_standalone`], and writes the
//! captured value according to the current [`OutputMode`].

use std::io::{self, Write};

use clap::{Args, ValueEnum};
use tui_chrome::{
    CANCELLED_KIND, Label, LabelPosition, TextInput, TextInputState, run_standalone,
};

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

    /// Render inline in `N` rows below the cursor instead of
    /// fullscreen.
    #[arg(long)]
    pub height: Option<u16>,
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
pub fn run(args: TextInputArgs, output: OutputMode) -> io::Result<i32> {
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

    match run_standalone(TextInput::new(), state, args.height) {
        Ok(value) => {
            let stdout = io::stdout();
            let mut lock = stdout.lock();
            write_scalar(&mut lock, &value, output)?;
            lock.flush()?;
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
        assert_eq!(LabelPosition::from(LabelPositionArg::Above), LabelPosition::Above);
        assert_eq!(LabelPosition::from(LabelPositionArg::Below), LabelPosition::Below);
        assert_eq!(LabelPosition::from(LabelPositionArg::Left), LabelPosition::Left);
        assert_eq!(LabelPosition::from(LabelPositionArg::Right), LabelPosition::Right);
    }
}
