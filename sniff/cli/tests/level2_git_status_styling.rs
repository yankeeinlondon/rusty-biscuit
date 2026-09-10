//! Level 2 tests for `sniff repo git-status` worktree-section terminal styling.
//!
//! The Worktrees/Status/Meta headers carry `<b><uu>…</uu></b>` (double
//! underline) and the worktree locations carry blue OSC8 hyperlinks
//! (`<blue><a href>…`). Level 1 pins the markup and the byte-exact blank-row
//! layout, but only a real terminal exercises the display path that turns that
//! markup into SGR sequences and decides how `<uu>`/links degrade. This test
//! renders the `render_git_status_fixture` helper binary inside a real terminal
//! (tmux — the portable, headless backend) and asserts the captured pane.
//!
//! tmux advertises `tmux-256color`, which supports straight but not double
//! underline and no OSC8 hyperlinks, so it exercises the *graceful-degradation*
//! path: `<uu>` collapses to straight underline (`\e[4m`) and links collapse to
//! a `[label](url)` fallback. Assertions accept either the full or degraded
//! form so the test is correct on richer backends too.
//!
//! Skip-clean: when no terminal harness is available the test prints a skip
//! notice and passes, so GitHub-hosted CI (which lacks the tooling) stays green.
//!
//! Gated behind `test-fixtures`: the helper binary this test spawns is only
//! built with that feature. The `test-l2` recipe passes `--features
//! test-fixtures`.
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
    "Status",
    "Worktrees",
    "Meta",
    "main:",
    "Current Worktree:",
    "Other Worktrees:",
    "login-fix",
];

fn final_git_status_frame_is_visible(frame: &CapturedFrame) -> bool {
    let visible = VISIBLE_EVIDENCE
        .iter()
        .all(|needle| frame.plain.contains(needle));
    let underline =
        frame.raw.contains("4:2m") || frame.raw.contains("[4m") || frame.raw.contains(";4m");
    let link = frame.raw.contains("]8;;") || frame.plain.contains("](file");
    let lines: Vec<&str> = frame.plain.lines().collect();
    let layout = lines
        .iter()
        .position(|line| line.contains("Worktrees"))
        .is_some_and(|index| {
            index >= 2 && lines[index - 1].trim().is_empty() && !lines[index - 2].trim().is_empty()
        });

    visible && underline && link && layout
}

#[test]
fn level2_git_status_headers_and_links_render_styled_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");

    let bin_path = cargo_bin("render_git_status_fixture").display().to_string();
    harness
        .send_command_with_env(&format!("clear; {bin_path}"), &[("FORCE_COLOR", "1")])
        .expect("send_command_with_env failed");

    let frame = common::capture_until(
        &mut harness,
        RENDER_DEADLINE,
        final_git_status_frame_is_visible,
    );

    // Visible text: all three section headers plus the worktree-section bodies.
    for needle in VISIBLE_EVIDENCE {
        assert!(
            frame.plain.contains(needle),
            "expected '{needle}' in captured pane.\nplain:\n{}",
            frame.plain,
        );
    }

    // Double-underline headers: `\e[4:2m` (ITU double) on capable terminals, or
    // straight underline (`\e[4m` / `;4m`) after graceful degradation in tmux.
    // Matched precisely so a blue SGR (`[34m`) cannot satisfy it.
    let has_underline =
        frame.raw.contains("4:2m") || frame.raw.contains("[4m") || frame.raw.contains(";4m");
    assert!(
        has_underline,
        "expected a double/straight underline SGR (section headers) in raw capture.\nraw:\n{}",
        frame.raw,
    );

    // Worktree locations render as hyperlinks: OSC8 (`\e]8;;`) on capable
    // terminals, or the `[label](file://…)` markdown fallback in tmux.
    let has_link = frame.raw.contains("]8;;") || frame.plain.contains("](file");
    assert!(
        has_link,
        "expected an OSC8 hyperlink or its fallback (worktree paths) in capture.\nplain:\n{}\nraw:\n{}",
        frame.plain, frame.raw,
    );

    // Layout: exactly one blank row separates a section header from the next
    // section's content. Find the Worktrees header and assert the row directly
    // above it is blank while the row two above (Status content) is not — the
    // single-blank-row contract, observed through a real terminal.
    let lines: Vec<&str> = frame.plain.lines().collect();
    if let Some(wt_idx) = lines.iter().position(|l| l.contains("Worktrees")) {
        assert!(
            wt_idx >= 2,
            "Worktrees header should have content + blank above it.\nplain:\n{}",
            frame.plain,
        );
        assert!(
            lines[wt_idx - 1].trim().is_empty(),
            "exactly one blank row must precede the Worktrees header.\nplain:\n{}",
            frame.plain,
        );
        assert!(
            !lines[wt_idx - 2].trim().is_empty(),
            "no second blank row before the Worktrees header.\nplain:\n{}",
            frame.plain,
        );
    }
}
