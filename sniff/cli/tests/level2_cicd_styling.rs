//! Level 2 tests for `sniff repo remote` CI/CD status-cell styling.
//!
//! The `render_cicd` status column carries biscuit-terminal markup
//! (`<green>✓</green>`, `<red>✗</red>`, `<dim>⊘</dim>`, dim status text). Level 1
//! tests pin the markup, but only a real terminal exercises the display path
//! that turns that markup into SGR sequences. These tests render the
//! `render_cicd_fixture` helper binary inside a real terminal (tmux — the
//! portable, headless backend) and assert the captured pane carries the
//! expected glyphs (plain) and color/dim SGR codes (raw).
//!
//! Skip-clean: when no terminal harness is available the test prints a skip
//! notice and passes, so GitHub-hosted CI (which lacks the tooling) stays green.
//!
//! Gated behind `test-fixtures`: the `render_cicd_fixture` helper binary this
//! test spawns is only built with that feature, so the file compiles to nothing
//! (and the missing `CARGO_BIN_EXE_*` env var never breaks `just test`) unless
//! the feature is enabled. The `test-l2` recipe passes `--features test-fixtures`.
#![cfg(feature = "test-fixtures")]

use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::CapturedFrame;
use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use std::time::Duration;
use test_toolkit::{Backend, Level, require_level};

mod common;

const RENDER_DEADLINE: Duration = Duration::from_secs(5);
const VISIBLE_EVIDENCE: &[&str] = &[
    "CI/CD", "Build", "Lint", "Deploy", "Release", "✓", "✗", "⊘", "queued",
];

fn final_cicd_frame_is_visible(frame: &CapturedFrame) -> bool {
    VISIBLE_EVIDENCE
        .iter()
        .all(|needle| frame.plain.contains(needle))
        && frame.raw.contains("32m")
        && frame.raw.contains("31m")
        && (frame.raw.contains("[2m") || frame.raw.contains(";2m"))
}

#[test]
fn level2_cicd_status_cells_render_styled_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");

    let bin_path = cargo_bin("render_cicd_fixture").display().to_string();
    harness
        .send_command_with_env(&format!("clear; {bin_path}"), &[("FORCE_COLOR", "1")])
        .expect("send_command_with_env failed");

    let frame = common::capture_until(&mut harness, RENDER_DEADLINE, final_cicd_frame_is_visible);

    // Visible text: the four run rows, each status indicator, and — proving
    // active runs render their dim status word rather than a glyph — "queued".
    for needle in VISIBLE_EVIDENCE {
        assert!(
            frame.plain.contains(needle),
            "expected '{needle}' in captured pane.\nplain:\n{}",
            frame.plain,
        );
    }

    // success → green (SGR 32), failure → red (SGR 31). Match tolerantly across
    // SGR forms (`\x1b[32m`, `;32m`) since multiplexers may merge attributes.
    assert!(
        frame.raw.contains("32m"),
        "expected a green SGR (success ✓) in raw capture.\nraw:\n{}",
        frame.raw,
    );
    assert!(
        frame.raw.contains("31m"),
        "expected a red SGR (failure ✗) in raw capture.\nraw:\n{}",
        frame.raw,
    );
    // skipped/cancelled and active status text are dim (SGR 2).
    assert!(
        frame.raw.contains("[2m") || frame.raw.contains(";2m"),
        "expected a dim SGR (skipped ⊘ / queued) in raw capture.\nraw:\n{}",
        frame.raw,
    );
}
