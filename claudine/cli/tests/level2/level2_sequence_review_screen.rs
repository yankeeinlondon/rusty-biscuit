//! Level 2: the `claudine sequence` provider review screen in a real terminal.
//!
//! The L1 tests prove the row filter and the merge by original step index with
//! injected review callbacks, and render `InputTable` into a headless buffer.
//! These tests run the shipped binary in a detached tmux pane on the fixture in
//! `common::review_screen`, so the screen is drawn by a terminal emulator, the
//! keys arrive through the pane's input path and the real event loop, and the
//! choices are observed where they are executed: at fake `claude`/`goose`
//! executables and in the files the hidden steps write.
//!
//! - **Layout:** only eligible steps have rows, labeled with their original
//!   position and full name; a narrow pane clips the long label with `…`
//!   without disturbing the next column; a short pane scrolls; restoring the
//!   size restores the full label.
//! - **Submission:** `Ctrl+S` runs every step, each eligible step on the
//!   provider and model chosen for its row, and the hidden steps still run.
//! - **Cancellation:** `Esc` and `Ctrl+C` exit `130` before any step runs.
//!
//! tmux sessions are detached, so no window opens or takes focus. The
//! OS-keyboard presses of the same keys are `level3_sequence_review_screen_keys`.
//!
//! Run via the canonical recipe: `just test-l2 sequence_review_screen`.

use biscuit_test_harness::layout_invariants::visible_width;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::time::Duration;
use test_toolkit::{Backend, Level, require_level};

use crate::common;
use common::review_screen::{
    CLAUDE_MODELS, HIDDEN_STEP_EVENTS, LONG_STEP_NAME, ModelEditor, ROW_LABELS, ReviewFixture,
    STEP_TOKENS, row_block, shows_selected,
};
use common::wait_for_pane_text;

/// Bound on a redraw after a resize or key.
const REDRAW_TIMEOUT: Duration = Duration::from_secs(10);

/// The geometry the harness spawns tmux panes at, restored after each test.
const FULL: (u32, u32) = (120, 40);

fn press(harness: &mut TmuxHarness, key: &str) {
    harness.send_key(key).expect("send key");
    harness.settle();
}

fn pane() -> TmuxHarness {
    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    harness.resize(FULL.0, FULL.1).expect("resize pane");
    harness
}

/// The display column at which `needle` starts on the first line holding
/// `row`.
fn column_of(plain: &str, row: &str, needle: &str) -> usize {
    let line = plain
        .lines()
        .find(|line| line.contains(row))
        .unwrap_or_else(|| panic!("no line holds {row:?}:\n{plain}"));
    let index = line
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} is not on the {row:?} line: {line:?}"));
    visible_width(&line[..index])
}

fn wait_until(
    harness: &mut TmuxHarness,
    what: &str,
    ready: impl Fn(&CapturedFrame) -> bool,
) -> CapturedFrame {
    let deadline = std::time::Instant::now() + REDRAW_TIMEOUT;
    loop {
        let frame = harness.capture().expect("capture pane");
        if ready(&frame) {
            return frame;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{what} never rendered; plain:\n{}",
            frame.plain
        );
        harness.settle();
    }
}

/// Rows, clipping, scrolling, and restoration; then per-row provider and
/// catalog model choices submitted with `Ctrl+S` reach the executables.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_review_screen_lays_out_eligible_rows_and_submits_catalog_choices() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let review = ReviewFixture::stage("review-screen-catalog", ModelEditor::CatalogChoice);
    let mut harness = pane();
    let open = review.open(&mut harness);

    // Only eligible steps, in order, labeled with their original position and
    // full name. The hidden first and last steps and the side effect have no row.
    let labeled: Vec<&str> = open
        .plain
        .lines()
        .filter_map(|line| {
            ROW_LABELS
                .into_iter()
                .find(|label| line.trim_start().starts_with(label))
        })
        .collect();
    assert_eq!(labeled, ROW_LABELS, "rows on the opened screen:\n{}", open.plain);
    for hidden in ["stage-first", "note", "stage-last"] {
        assert!(
            !open.plain.contains(hidden),
            "the hidden step {hidden:?} has a row:\n{}",
            open.plain
        );
    }
    for label in ROW_LABELS {
        let block = row_block(&open.plain, label);
        assert!(shows_selected(&block, "Claude"), "{label} does not start on Claude:\n{}", open.plain);
        assert!(shows_selected(&block, "(default)"), "{label} does not start on the default model:\n{}", open.plain);
        for model in CLAUDE_MODELS {
            assert!(
                block.iter().any(|line| line.contains(model)),
                "{label}'s model cell does not offer {model}:\n{}",
                open.plain
            );
        }
    }
    let provider_column = column_of(&open.plain, ROW_LABELS[0], "● Claude");

    // Narrow and short: the label column shrinks, and only the first two rows
    // fit. The off-screen long label still counts toward the column's width,
    // so `2 review` keeps room to its right.
    harness.resize(60, 8).expect("resize narrow");
    let narrow = wait_until(&mut harness, "the narrow, scrolled-to-top table", |frame| {
        frame.plain.contains('▼') && frame.plain.contains("4 review")
    });
    assert!(
        !narrow.plain.contains("summarize-"),
        "the fourth row must be off-screen on an 8-row pane:\n{}",
        narrow.plain
    );
    for line in narrow.plain.lines() {
        assert!(visible_width(line) <= 60, "a line overflows the 60-column pane: {line:?}");
    }
    let narrow_column = column_of(&narrow.plain, "2 review", "● Claude");
    assert!(
        narrow_column < provider_column,
        "the label column must shrink on a narrow pane ({narrow_column} vs {provider_column}):\n{}",
        narrow.plain
    );

    // Scroll to the last row. Its label is clipped with `…` and the provider
    // cell beside it starts where every other row's does.
    for _ in 0..6 {
        press(&mut harness, "Tab");
    }
    let scrolled = wait_until(&mut harness, "the scrolled table", |frame| {
        frame.plain.contains('▲') && frame.plain.contains("6 summarize-")
    });
    let clipped = scrolled
        .plain
        .lines()
        .find(|line| line.contains("6 summarize-"))
        .expect("the clipped row");
    assert!(
        clipped.contains("…▶ ● Claude"),
        "a clipped label must end in `…` beside an intact provider cell: {clipped:?}"
    );
    assert!(!clipped.contains(LONG_STEP_NAME), "the long label must be clipped: {clipped:?}");
    assert_eq!(
        column_of(&scrolled.plain, "6 summarize-", "● Claude"),
        column_of(&scrolled.plain, "5 build", "● Claude"),
        "clipping must not move the provider column:\n{}",
        scrolled.plain
    );
    assert_eq!(
        column_of(&scrolled.plain, "5 build", "● Claude"),
        narrow_column,
        "scrolling must not change the column widths:\n{}",
        scrolled.plain
    );

    // Restore: the full label returns at the original column widths.
    harness.resize(FULL.0, FULL.1).expect("restore pane");
    let restored = wait_until(&mut harness, "the restored full label", |frame| {
        frame.plain.contains(ROW_LABELS[3])
    });
    assert_eq!(
        column_of(&restored.plain, ROW_LABELS[3], "● Claude"),
        provider_column,
        "restoring the width must restore the column widths:\n{}",
        restored.plain
    );

    // Choices, in tab order from the first row's provider cell:
    // row 2 → Goose; row 4 → Claude with fake-claude-b; row 5 → Goose with
    // fake-claude-a; row 6 untouched.
    for _ in 0..6 {
        press(&mut harness, "BTab");
    }
    for key in [
        "Down", "Enter", // row 2 provider: Goose
        "Tab", "Tab", "Tab", "Down", "Down", "Enter", // row 4 model: fake-claude-b
        "Tab", "Down", "Enter", // row 5 provider: Goose
        "Tab", "Down", "Enter", // row 5 model: fake-claude-a
    ] {
        press(&mut harness, key);
    }
    let chosen = wait_for_pane_text(&mut harness, "● fake-claude-a", REDRAW_TIMEOUT);
    let expected = [
        (ROW_LABELS[0], "Goose", "(default)"),
        (ROW_LABELS[1], "Claude", CLAUDE_MODELS[1]),
        (ROW_LABELS[2], "Goose", CLAUDE_MODELS[0]),
        (ROW_LABELS[3], "Claude", "(default)"),
    ];
    for (label, provider, model) in expected {
        let block = row_block(&chosen.plain, label);
        assert!(
            shows_selected(&block, provider) && shows_selected(&block, model),
            "{label} must show {provider} / {model} selected:\n{}",
            chosen.plain
        );
    }

    press(&mut harness, "C-s");
    let (frame, status) = review.wait_for_exit(&mut harness);
    harness.resize(FULL.0, FULL.1).expect("restore pane");
    assert_eq!(status, "0", "the submitted sequence must succeed:\n{frame}");

    let mut expected_targets: Vec<(&str, String, Option<String>)> = vec![
        (STEP_TOKENS[0], "goose".into(), None),
        (STEP_TOKENS[1], "claude".into(), Some(CLAUDE_MODELS[1].into())),
        (STEP_TOKENS[2], "goose".into(), Some(CLAUDE_MODELS[0].into())),
        (STEP_TOKENS[3], "claude".into(), None),
    ];
    expected_targets.sort();
    assert_eq!(
        review.targets_by_step(),
        expected_targets,
        "each eligible step must launch once, on its row's provider and model:\n{frame}"
    );
    assert_eq!(
        review.events(),
        HIDDEN_STEP_EVENTS,
        "the hidden shell and side-effect steps must still run, in order:\n{frame}"
    );
    review.assert_no_audio();
}

/// The free-text model editor: typed text is drawn in the row's model cell
/// and reaches that row's executable, beside another row's provider choice.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_review_screen_accepts_a_free_text_model() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let review = ReviewFixture::stage("review-screen-free-text", ModelEditor::FreeText);
    let mut harness = pane();
    let open = review.open(&mut harness);
    for model in CLAUDE_MODELS {
        assert!(!open.plain.contains(model), "an empty catalog offers no choices:\n{}", open.plain);
    }

    // Row 2's provider → Goose; row 4's model (text input) → typed text.
    for key in ["Down", "Enter", "Tab", "Tab", "Tab"] {
        press(&mut harness, key);
    }
    harness.send_text(b"typed-model-x").expect("type model");
    let typed = wait_for_pane_text(&mut harness, "typed-model-x", REDRAW_TIMEOUT);
    let row = row_block(&typed.plain, ROW_LABELS[1]);
    assert!(
        row[0].contains("typed-model-x"),
        "the typed model must be drawn in row 4's model cell:\n{}",
        typed.plain
    );
    assert!(
        shows_selected(&row_block(&typed.plain, ROW_LABELS[0]), "Goose"),
        "row 2 must show Goose selected:\n{}",
        typed.plain
    );

    press(&mut harness, "C-s");
    let (frame, status) = review.wait_for_exit(&mut harness);
    assert_eq!(status, "0", "the submitted sequence must succeed:\n{frame}");
    let mut expected: Vec<(&str, String, Option<String>)> = vec![
        (STEP_TOKENS[0], "goose".into(), None),
        (STEP_TOKENS[1], "claude".into(), Some("typed-model-x".into())),
        (STEP_TOKENS[2], "claude".into(), None),
        (STEP_TOKENS[3], "claude".into(), None),
    ];
    expected.sort();
    assert_eq!(review.targets_by_step(), expected, "targets per step:\n{frame}");
    assert_eq!(review.events(), HIDDEN_STEP_EVENTS, "hidden steps:\n{frame}");
    review.assert_no_audio();
}

fn assert_key_cancels(name: &str, key: &str) {
    let review = ReviewFixture::stage(name, ModelEditor::CatalogChoice);
    let mut harness = pane();
    review.open(&mut harness);
    press(&mut harness, key);
    let (frame, status) = review.wait_for_exit(&mut harness);
    assert_eq!(status, "130", "{key} on the review screen must exit 130:\n{frame}");
    review.assert_no_work(&frame);
}

/// `Esc` leaves the screen, starts no step, and exits `130`.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_review_screen_escape_exits_130_without_work() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    assert_key_cancels("review-screen-escape", "Escape");
}

/// `Ctrl+C` leaves the screen, starts no step, and exits `130`.
#[test]
#[serial(level2_terminal)]
fn level2_tmux_review_screen_ctrl_c_exits_130_without_work() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    assert_key_cancels("review-screen-ctrl-c", "C-c");
}
