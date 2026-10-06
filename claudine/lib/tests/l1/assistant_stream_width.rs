//! A narrowed terminal — the sequence task gutter inset built by
//! `new_assistant_stream_inset` — must reach the Markdown the wrapper
//! renders, on both the streaming path and the final-message path, while the
//! plain (non-TTY) path stays byte-exact raw text.

use biscuit_terminal::terminal::Terminal;
use claudine::render::{AssistantStream, StreamRenderable};
use darkmatter::markdown::output::terminal::{TerminalImageMode, TerminalOptions};

fn frames(v: Vec<String>) -> String {
    v.concat()
}

/// The plain (non-TTY) path must stay byte-exact raw passthrough — no
/// Markdown reflow, no width involvement — so piped output keeps the
/// provider's bytes exactly.
#[test]
fn plain_stream_output_is_byte_exact_raw_text() {
    let mut renderer = AssistantStream::new(None, None);
    let source = "Some prose the agent wrote.\n\n```rust\nlet x = 1;\n```\n";

    let mut flushed = frames(renderer.append(source));
    flushed.push_str(&frames(renderer.close()));

    assert_eq!(flushed, source);
}

/// The streaming path renders each block through the final-message component
/// with the same terminal, so a narrowed (gutter-inset) terminal wraps
/// streamed Markdown exactly like the final render. Width 40 sits far below
/// any detected default (80+), so a render that re-opened detection instead
/// of honoring the supplied terminal overflows it.
#[test]
fn markdown_stream_honors_the_narrowed_terminal_width() {
    let term = Terminal::new_optimistic(40);
    let mut opts = TerminalOptions::default();
    opts.image_mode = TerminalImageMode::Never;
    let mut renderer = AssistantStream::new(Some(term), Some(opts));

    let flushed = frames(renderer.append(
        "The quick brown fox jumps over the lazy dog while the eager cat \
         watches carefully from the windowsill above.\n\n",
    ));
    let widths: Vec<usize> = biscuit_terminal::prelude::strip_escape_codes(&flushed)
        .lines()
        .map(|line| line.chars().count())
        .collect();
    assert!(
        widths.len() > 1 && widths.iter().all(|width| *width <= 40),
        "streamed Markdown must wrap to the narrowed terminal, got {widths:?}"
    );
}
