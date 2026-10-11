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

/// Prose that arrives a few words at a time, with no newline until the
/// paragraph ends, must still wrap to the narrowed terminal when the caller
/// holds partial lines — the shape a sequence task's gutter needs. Streamed
/// raw, each fragment would bypass rendering and reach the terminal unfolded.
#[test]
fn held_partial_lines_wrap_to_the_narrowed_terminal_width() {
    let term = Terminal::new_optimistic(40);
    let mut opts = TerminalOptions::default();
    opts.image_mode = TerminalImageMode::Never;
    let mut renderer = AssistantStream::new(Some(term), Some(opts)).holding_partial_lines(true);

    let mut flushed = String::new();
    for chunk in [
        "The quick brown fox jumps ",
        "over the lazy dog while the ",
        "eager cat watches carefully ",
        "from the windowsill above.\n\n",
    ] {
        flushed.push_str(&frames(renderer.append(chunk)));
    }
    flushed.push_str(&frames(renderer.close()));

    let visible = biscuit_terminal::prelude::strip_escape_codes(&flushed);
    let widths: Vec<usize> = visible.lines().map(|line| line.chars().count()).collect();
    assert!(
        widths.len() > 1 && widths.iter().all(|width| *width <= 40),
        "held partial lines must render wrapped to the narrowed terminal, got {widths:?}"
    );
    let words: Vec<&str> = visible.split_whitespace().collect();
    assert_eq!(
        words.join(" "),
        "The quick brown fox jumps over the lazy dog while the eager cat watches \
         carefully from the windowsill above."
    );
}

/// A stream that ends mid-line still shows the held fragment on close.
#[test]
fn a_held_partial_line_is_rendered_on_close() {
    let term = Terminal::new_optimistic(40);
    let mut renderer = AssistantStream::new(Some(term), None).holding_partial_lines(true);

    assert!(frames(renderer.append("no newline ever")).is_empty());
    let closed = biscuit_terminal::prelude::strip_escape_codes(frames(renderer.close()));
    assert_eq!(closed.trim(), "no newline ever");
}
