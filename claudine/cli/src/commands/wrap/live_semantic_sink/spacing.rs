//! Spacing state machine — tracks blank-row state across stdout and stderr.

use super::LiveSemanticSink;

impl LiveSemanticSink {
    /// Extend the cumulative stdout trailing-newline counter by the tail of
    /// `text`. If `text` is entirely newline bytes (or empty), the counter
    /// is extended. Otherwise the counter is reset to the count of newline
    /// bytes at the very end of `text`.
    ///
    /// Also updates the unified [`Self::at_blank_row`] state: any
    /// non-newline content clears the blank-row flag; purely-newline text
    /// may set it when the cumulative count reaches 2+.
    pub(crate) fn update_stdout_trailing_newlines(&mut self, text: &str) {
        let trailing = text.bytes().rev().take_while(|b| *b == b'\n').count();
        if trailing == text.len() {
            self.stdout_trailing_newlines += trailing;
        } else {
            self.stdout_trailing_newlines = trailing;
        }
        if text.bytes().any(|b| b != b'\n' && b != b'\r') {
            self.at_blank_row = false;
        }
        if self.stdout_trailing_newlines >= 2 {
            self.at_blank_row = true;
        }
    }

    /// True when the combined visual output (stdout + stderr) is currently
    /// sitting at a visually blank row. In that state the section-transition
    /// separator blank is redundant and would produce a visually doubled
    /// blank line.
    ///
    /// Tracks both streams:
    /// - **Stdout**: two or more trailing `\n` bytes indicate a paragraph
    ///   break that already provides visual separation.
    /// - **Stderr**: the [`Self::at_blank_row`] flag is set when a blank
    ///   stderr line is emitted and cleared when non-blank content follows.
    pub(crate) fn at_visual_blank(&self) -> bool {
        self.at_blank_row
    }
}
