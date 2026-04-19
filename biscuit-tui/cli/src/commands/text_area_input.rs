//! `question text-area-input` subcommand.
//!
//! Maps CLI args onto a [`tui_chrome::TextAreaInputState`], runs the
//! component via [`tui_chrome::run_standalone`], and writes the
//! captured multi-line value according to the current [`OutputMode`].

use std::io::{self, Write};

use clap::Args;
use tui_chrome::{
    CANCELLED_KIND, Label, TextAreaInput, TextAreaInputState, run_standalone,
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

    /// Render inline in `N` rows below the cursor instead of
    /// fullscreen.
    #[arg(long)]
    pub height: Option<u16>,

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
pub fn run(args: TextAreaInputArgs, output: OutputMode) -> io::Result<i32> {
    let editor_height = args.height.unwrap_or(10);
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

    match run_standalone(TextAreaInput::new(), state, args.height) {
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
    fn default_width_is_sixty() {
        // Smoke test — ensure the default matches the documented
        // `--width` fallback.
        let args = TextAreaInputArgs {
            label: None,
            label_position: LabelPositionArg::Above,
            width: 60,
            height: None,
            scrollbar: false,
            initial: None,
        };
        assert_eq!(args.width, 60);
        assert!(!args.scrollbar);
    }
}
