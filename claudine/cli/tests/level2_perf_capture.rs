//! Level 2 real-terminal capture tests for the `--perf` metrics tree.
//!
//! The L1 unit tests in `perf.rs` and the CLI integration tests assert the
//! report's *semantics* after stripping escape codes, so they cannot catch
//! broken glyph emission, column alignment, or SGR styling in a real terminal.
//! These tests drive the real `claudine compose --perf --dry-run` binary inside
//! a terminal emulator and assert against the bytes the terminal actually
//! displayed (`frame.raw` / `frame.plain`):
//!
//! - box-drawing connector hierarchy (`├─`, `│`, `└─`) — P-1
//! - the unit-aligned duration column — P-4
//! - the visible share-of-wall-clock percent column (`100%`, `<1%`, …) — P-2
//! - the single `▇ HOT` marker on the dominant leaf, rendered bold — P-3
//! - the dry-run `—` agent placeholder — P-5
//! - the yellow `▌ ` block-quote frame — Presentation
//!
//! The fixture contains a `::shell sleep 0.3` directive (`--yolo` auto-approves
//! it) so shell expansion is the unambiguous dominant leaf, making the `HOT`
//! marker deterministic.
//!
//! Stream separation (the `--perf` report is stderr-only, stdout stays clean) is
//! a Level-1 concern proved by the assert-command tests — a captured terminal
//! pane interleaves both streams, so it cannot distinguish them.
//!
//! Skip-clean: each test checks `Harness::available()` first and returns early
//! when the backend is absent (no `#[ignore]`), so CI without the tooling stays
//! green. `BISCUIT_TEST_LEVEL_REQUIRED=2` flips a missing harness into a hard
//! failure. Run via the canonical recipe:
//!
//! ```text
//! just test-l2
//! ```

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::time::Duration;
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::{TestWorkspace, augmented_path, clear_no_color, write_executable};

/// A compose fixture with a slow `::shell` directive so shell expansion is the
/// unambiguous dominant leaf (deterministic `HOT` marker).
const FIXTURE_DOC: &str = "\
---
name: Perf Doc
---
Body before.
::shell sleep 0.3
Body after.
";

/// What the captured frame carries, kept with the workspace so it outlives the
/// capture.
struct PerfCapture {
    frame: CapturedFrame,
    _workspace: TestWorkspace,
}

/// Stage a workspace and run `claudine compose --goose --perf --dry-run --yolo
/// doc.md` inside `harness`, returning the captured pane frame.
///
/// `--goose` resolves the Agent (stubbed on `PATH`, never launched under
/// `--dry-run`); `--yolo` auto-approves the `::shell` directive so the compose
/// pass executes it and the perf tree carries a dominant shell-expansion leaf.
fn run_perf_compose<H: TerminalHarness>(harness: &mut H) -> PerfCapture {
    let workspace = TestWorkspace::named("claudine-perf-l2");
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    write_executable(&bin_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let doc = workspace.path().join("doc.md");
    fs::write(&doc, FIXTURE_DOC).unwrap();

    let claudine = cargo_bin("claudine").display().to_string();
    let path = augmented_path(&bin_dir);
    let path = path.to_string_lossy().into_owned();
    let home = workspace.path().to_string_lossy().into_owned();

    harness
        .send_text(format!("cd {}\n", workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let cmd = format!(
        "{claudine} compose --goose --perf --dry-run --yolo {}",
        doc.display()
    );
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", home.as_str()),
                ("PATH", path.as_str()),
                ("FORCE_COLOR", "1"),
                // Deterministic, unwrapped report width (rows fit well inside 80).
                ("COLUMNS", "80"),
            ],
        )
        .expect("send compose --perf --dry-run");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    // The `::shell sleep 0.3` directive runs during the compose pass; allow the
    // command to finish and the pane to settle before capturing.
    std::thread::sleep(Duration::from_millis(600));

    let frame = harness.capture().expect("capture failed");
    PerfCapture {
        frame,
        _workspace: workspace,
    }
}

/// Return the raw (escape-bearing) capture line whose plain (escape-stripped)
/// form contains `needle`.
fn raw_line<'a>(frame: &'a CapturedFrame, needle: &str) -> Option<&'a str> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    frame
        .plain
        .lines()
        .position(|l| l.contains(needle))
        .and_then(|i| raw_lines.get(i).copied())
}

/// Whether `line` carries the bold attribute (SGR `1`).
///
/// Decodes CSI `m` sequences accepting both `;` and ITU `:` separators (terminal
/// capture paths re-emit attributes in either form) and skips the operands of
/// `38`/`48` truecolor/indexed selectors so a color component is never misread.
fn has_bold(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'[') {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len()
                && (bytes[j].is_ascii_digit() || bytes[j] == b';' || bytes[j] == b':')
            {
                j += 1;
            }
            if bytes.get(j) == Some(&b'm') {
                let params: Vec<i64> = line[start..j]
                    .split([';', ':'])
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| s.parse().ok())
                    .collect();
                let mut k = 0;
                while k < params.len() {
                    match params[k] {
                        1 => return true,
                        38 | 48 => match params.get(k + 1) {
                            Some(&2) => k += 5,
                            Some(&5) => k += 3,
                            _ => k += 1,
                        },
                        _ => k += 1,
                    }
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    false
}

/// Assert the structural contract shared by every backend, read from the plain
/// (escape-stripped) capture.
///
/// The full report is taller than the harness's 40-row pane, so the
/// `Performance … 100%` headline scrolls above the captured viewport — its
/// correctness is locked by the L1 snapshot test (`render_perf_report_snapshot_…`)
/// instead. Everything asserted here lives in the visible bottom region: the
/// connector hierarchy, the unit-aligned value column, the percent column, the
/// `HOT` marker on the dominant shell directive, and the dry-run placeholder.
fn assert_tree_structure(frame: &CapturedFrame) {
    let plain = &frame.plain;

    // Box-drawing connectors (P-1).
    assert!(
        plain.contains("├─") && plain.contains("└─"),
        "expected box-drawing tree connectors (├─ / └─).\nplain:\n{plain}",
    );

    // The yellow block-quote frame glyph reached the pane.
    assert!(
        plain.contains('▌'),
        "expected the `▌` block-quote frame.\nplain:\n{plain}",
    );

    // The dominant leaf is the slow shell directive, flagged `▇ HOT` (P-3).
    let hot = plain
        .lines()
        .find(|l| l.contains("▇ HOT"))
        .unwrap_or_else(|| panic!("expected a `▇ HOT` marker.\nplain:\n{plain}"));
    assert!(
        hot.contains("shell · sleep"),
        "HOT must flag the dominant `::shell` directive; got: {hot:?}",
    );

    // Dry-run agent placeholder (P-5): an `—` leaf annotated `(dry run)`.
    let agent = plain
        .lines()
        .find(|l| l.contains("agent execution"))
        .unwrap_or_else(|| panic!("expected an `agent execution` row.\nplain:\n{plain}"));
    assert!(
        agent.contains('—') && agent.contains("(dry run)"),
        "dry-run agent must render as an `—` leaf annotated `(dry run)`; got: {agent:?}",
    );

    // The percent column is visible beyond just the root (P-2): at least one
    // sub-percent sliver renders `<1%` rather than a column full of `0%`.
    assert!(
        plain.contains("<1%"),
        "expected a `<1%` sliver in the share column.\nplain:\n{plain}",
    );

    // The duration column aligns at the unit boundary across the tree (P-4):
    // every `ms`/`µs` value's unit suffix begins at the same display column,
    // regardless of label depth.
    let unit_columns: Vec<usize> = plain
        .lines()
        .filter_map(|l| {
            l.find("ms")
                .or_else(|| l.find("µs"))
                .map(|byte_idx| l[..byte_idx].chars().count())
        })
        .collect();
    assert!(
        unit_columns.len() >= 2,
        "expected multiple ms/µs value rows to check alignment.\nplain:\n{plain}",
    );
    assert!(
        unit_columns.iter().all(|c| *c == unit_columns[0]),
        "duration units are not column-aligned: {unit_columns:?}\nplain:\n{plain}",
    );
}

/// tmux is the portable, headless backend and faithfully re-emits SGR, so it
/// covers the structural contract plus the bold `HOT` marker and the colored
/// frame on every host that has `tmux`.
#[test]
#[serial(level2_terminal)]
fn level2_perf_tree_renders_styled_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    // This fixture asserts a *colored* surface under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(&mut harness);
    let capture = run_perf_compose(&mut harness);
    assert_tree_structure(&capture.frame);

    // The `▇ HOT` marker is emitted bold (`<b>…</b>` → SGR 1).
    let hot = raw_line(&capture.frame, "▇ HOT")
        .unwrap_or_else(|| panic!("HOT row not found.\nraw:\n{}", capture.frame.raw));
    assert!(
        has_bold(hot),
        "expected the `▇ HOT` marker to render bold (SGR 1).\nrow: {hot:?}",
    );

    // The `▌` frame carries a foreground color (the yellow left block). We assert
    // a truecolor fg selector is present rather than an exact RGB triple, which
    // capture paths may renormalize. Read from a row that is always inside the
    // captured viewport (the headline scrolls off — see `assert_tree_structure`).
    let frame_line = raw_line(&capture.frame, "environment setup")
        .unwrap_or_else(|| panic!("environment setup row not found.\nraw:\n{}", capture.frame.raw));
    assert!(
        frame_line.contains("38;2;"),
        "expected the `▌` frame to render a truecolor foreground.\nrow: {frame_line:?}",
    );
}

/// WezTerm is the second backend; it re-emits the same SGR contract, so it
/// independently confirms the structure and the bold `HOT` marker. (WezTerm's
/// capture path can renormalize color SGR, so the truecolor-frame assertion is
/// left to the faithful tmux path.)
#[test]
#[serial(level2_terminal)]
fn level2_perf_tree_renders_styled_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    // This fixture asserts a *colored* surface under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(&mut harness);
    let capture = run_perf_compose(&mut harness);
    assert_tree_structure(&capture.frame);

    let hot = raw_line(&capture.frame, "▇ HOT")
        .unwrap_or_else(|| panic!("HOT row not found.\nraw:\n{}", capture.frame.raw));
    assert!(
        has_bold(hot),
        "expected the `▇ HOT` marker to render bold (SGR 1).\nrow: {hot:?}",
    );
}
