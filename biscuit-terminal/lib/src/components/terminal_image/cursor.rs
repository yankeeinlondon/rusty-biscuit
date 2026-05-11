//! Cursor row computation and scroll-compensation helpers shared by the
//! [Kitty](super::kitty) and [iTerm2](super::iterm) protocol paths.

use crate::discovery::detection::TerminalApp;

/// Round the raw fractional cell-row count to the cursor-advance row count
/// expected by `term.app`.
///
/// - Warp uses **floor** rounding because its renderer treats partial cells
///   as zero rows.
/// - Every other detected emulator uses **ceil** rounding because partial
///   rows still consume a full terminal row.
pub(super) fn compute_cursor_rows(app: &TerminalApp, raw_height: f32, height_cells: u32) -> u32 {
    match app {
        TerminalApp::Warp => (raw_height.floor() as u32).max(1),
        _ => height_cells,
    }
}

/// Detect whether the image render needs an extra `\n` to compensate for the
/// terminal scrolling under the cursor when the image extends past the
/// viewport bottom.
///
/// Returns `false` for [`TerminalApp::Ghostty`] (its save/restore semantics
/// already settle correctly) and [`TerminalApp::Warp`] (input is anchored at
/// the bottom, so images never trigger scroll). For every other emulator,
/// queries the live cursor position and compares against the terminal
/// height.
pub(super) fn needs_scroll_compensation(
    app: &TerminalApp,
    cursor_rows: u32,
    term_height: u32,
    relative: &str,
) -> bool {
    if matches!(app, TerminalApp::Ghostty | TerminalApp::Warp) {
        return false;
    }

    use crate::discovery::cursor_position::cursor_position;
    if let Some(pos) = cursor_position() {
        pos.row + cursor_rows > term_height
    } else {
        tracing::trace!(
            file = %relative,
            "cursor_position() returned None, scroll compensation disabled"
        );
        false
    }
}
