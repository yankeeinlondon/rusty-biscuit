//! Level 2 tests for dirty-file tree terminal rendering.
//!
//! Verifies that `dirty_tree::render_markup` output, when processed through
//! `Prose::new(...).render(terminal)`, keeps its box-drawing layout and its
//! per-file colors in a real terminal (tmux): source files orange, other files
//! yellow, directories dim, connectors unstyled.

mod styled_capture;

use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use serial_test::serial;
use styled_capture::{Color, StyledScreen};
use test_toolkit::{Backend, Level, require_level};

/// `<orange>`, as `styles_follow_the_design` in `list_table.rs` expects.
const ORANGE: Color = Color::Rgb(255, 165, 0);
/// `<yellow>`: the basic ANSI yellow.
const YELLOW: Color = Color::Indexed(3);

#[test]
#[serial(level2_terminal)]
fn level2_dirty_tree_renders_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    let bin_path = cargo_bin("render_dirty_tree").display().to_string();
    harness
        .send_text(format!("clear; FORCE_COLOR=1 {bin_path}\n").as_bytes())
        .expect("send_text failed");

    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(std::time::Duration::from_millis(200));

    let frame = harness.capture().expect("capture failed");
    let screen = StyledScreen::parse(&frame.raw);

    // Layout: the whole tree, row by row, with every connector glyph.
    let first = screen.row_with(&["├── README.md"]);
    let expected = ["├── README.md", "├── docs/", "│   └── guide.md", "└── src/", "    └── lib.rs"];
    for (offset, line) in expected.iter().enumerate() {
        assert_eq!(
            screen.text(first + offset).trim_end(),
            *line,
            "tree row {offset}.\nscreen:\n{}",
            screen.plain()
        );
    }

    // Colors: source orange, other files yellow, directories dim.
    screen.assert_span(first + 4, "lib.rs", "orange", |s| s.fg_is(ORANGE));
    screen.assert_span(first, "README.md", "yellow", |s| s.fg_is(YELLOW));
    screen.assert_span(first + 2, "guide.md", "yellow", |s| s.fg_is(YELLOW));
    screen.assert_span(first + 1, "docs/", "dim", |s| s.dim);
    screen.assert_span(first + 3, "src/", "dim", |s| s.dim);

    // Connectors carry no color and no emphasis.
    for (row, glyphs) in [(first, "├──"), (first + 2, "│   └──"), (first + 4, "    └──")] {
        screen.assert_span(row, glyphs, "unstyled", |s| {
            s.fg.is_none() && s.bg.is_none() && !s.dim && !s.bold
        });
    }
}
