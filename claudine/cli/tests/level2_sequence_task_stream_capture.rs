//! Level 2 real-terminal capture for sequence task-stream rendering.
//!
//! Feature: `claudine/features/2026-07-11-sequence-plus/` (spec →
//! *Reporting Concurrency*), review-2 finding 4.
//!
//! ## Why a real pane, when L1 already asserts the frames
//!
//! The L1 suite (`sequence_groups.rs`, `render::task_stream::tests`) is the
//! authority on frame *atomicity* and exact bytes: it captures stdout and
//! stderr as two separate pipes and compares SGR sequences verbatim. That is
//! precisely what it cannot do here — a reader never sees two pipes. They see
//! one pane in which status framing (stderr) and task data (stdout) are
//! **interleaved in arrival order**, folded at the pane's real width, and
//! re-emitted by the terminal's own SGR handling.
//!
//! These tests therefore assert only what the separated-pipe view cannot
//! reach:
//!
//! - a body line's color bar still matches its header's *after* both channels
//!   have been merged by the terminal (`bar_color`);
//! - nothing overflows the pane, so the terminal never hard-wraps a frame and
//!   drops its gutter — the failure mode that made serial work lurch back to
//!   column 0 while parallel work stayed indented;
//! - the degraded shapes (`NO_COLOR`, non-UTF-8 locale) survive an actual
//!   capability handshake rather than a constructed `Terminal`.
//!
//! L1 keeps the byte-equality and torn-escape assertions. This file
//! deliberately makes none.
//!
//! ## Capture caveats honored here
//!
//! - **tmux collapses SGR.** No assertion compares raw bytes. Color checks go
//!   through [`bar_color`], which extracts the `38;2;R;G;B` triple painting the
//!   bar and compares *triples*, not escape sequences.
//! - **`capture()` sees the visible pane only, never scrollback.** Every
//!   fixture is sized so its whole run fits in the pane, and each run starts
//!   from a cleared pane.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test-l2
//! ```

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use test_toolkit::{Level, require_level};

mod common;
use common::{TestWorkspace, augmented_path, write, write_executable};

/// A parallel group of two named tasks, each printing one identifiable line.
///
/// Two members, not more: the contract under test is "a body line carries its
/// own task's bar", and two tasks with distinct palette entries is the smallest
/// fixture that can fail it.
const PARALLEL_DOC: &str = "\
---
sequence:
  - name: alpha
    group:
      name: bundle
      execution: parallel
      tasks:
        - name: fetch-data
          shell: \"printf 'fetched\\n'\"
        - name: render-page
          shell: \"printf 'rendered\\n'\"
---

Body.
";

/// A serial group followed by a parallel one, each emitting a body line long
/// enough to need folding in a narrow pane.
///
/// The pairing is the point: the same long payload rendered under an invisible
/// bar and under a colored one must fold at the same column and keep the same
/// two-column gutter on every continuation line.
const MIXED_WIDTH_DOC: &str = "\
---
sequence:
  - name: serial-step
    group:
      name: serial-bundle
      execution: serial
      tasks:
        - name: ser-task
          shell: \"printf 'alpha bravo charlie delta echo foxtrot golf hotel india juliett\\n'\"
  - name: parallel-step
    group:
      name: parallel-bundle
      execution: parallel
      tasks:
        - name: par-task
          shell: \"printf 'alpha bravo charlie delta echo foxtrot golf hotel india juliett\\n'\"
---

Body.
";

/// A dynamic source that resolves to nothing — the ratified graceful no-op.
const ZERO_STEP_DOC: &str = "\
---
items: []
sequence: \"{{ items }}\"
---
Work on {{state}}.
";

struct Staged {
    workspace: TestWorkspace,
    bin_dir: PathBuf,
    doc: PathBuf,
}

/// Stage a workspace holding `doc_body` at `seq.md` plus a `goose` stub.
///
/// `.claudine/config.json` is written because a pane is a real TTY: without an
/// existing config the first-run onboarding wizard takes the terminal and the
/// sequence never runs.
fn stage(name: &str, doc_body: &str) -> Staged {
    let workspace = TestWorkspace::named(name);
    let root = workspace.path().to_path_buf();

    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    write_executable(
        &bin_dir.join("goose"),
        "#!/bin/sh\nprintf 'agent-said\\n'\nexit 0\n",
    );

    write(&root.join(".claudine/config.json"), "{}");
    let doc = root.join("seq.md");
    write(&doc, doc_body);

    Staged {
        workspace,
        bin_dir,
        doc,
    }
}

struct Capture {
    frame: CapturedFrame,
    exit_code: i32,
}

/// Bracket- and glob-free exit marker (see `level2_typed_error_render_capture`).
const EXIT_MARKER: &str = "claudine_rc:";

fn parse_exit_marker(plain: &str) -> Option<i32> {
    plain.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix(EXIT_MARKER)?;
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    })
}

fn wait_for_exit_marker(harness: &mut TmuxHarness, deadline: Duration) -> Capture {
    let stop = Instant::now() + deadline;
    loop {
        let frame = harness.capture().expect("capture pane");
        if let Some(exit_code) = parse_exit_marker(&frame.plain) {
            return Capture { frame, exit_code };
        }
        if Instant::now() >= stop {
            panic!(
                "the `{EXIT_MARKER}<code>` exit marker did not appear within \
                 {deadline:?}.\nplain:\n{}",
                frame.plain
            );
        }
        harness.settle();
    }
}

/// Run `claudine sequence --goose --yolo <doc>` in a pane of `cols`x`rows` and
/// capture the finished frame.
///
/// `extra_env` is the capability lever each test pulls; everything else is held
/// fixed so a difference in the capture is attributable to it.
fn run_in_pane(
    harness: &mut TmuxHarness,
    staged: &Staged,
    cols: u32,
    rows: u32,
    extra_env: &[(&str, &str)],
) -> Capture {
    harness.resize(cols, rows).expect("resize pane");

    let claudine = cargo_bin!("claudine").display().to_string();
    let home = staged.workspace.path().to_string_lossy().into_owned();
    let path = augmented_path(&staged.bin_dir);
    let path = path.to_string_lossy().into_owned();

    harness.send_text(b"clear\n").expect("clear pane");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    harness
        .send_text(format!("cd {}\n", staged.workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    harness.send_text(b"clear\n").expect("clear pane");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let cmd = format!(
        "{claudine} sequence --goose --yolo {}; echo {EXIT_MARKER}$?",
        staged.doc.display()
    );
    let mut env: Vec<(&str, &str)> = vec![("HOME", home.as_str()), ("PATH", path.as_str())];
    env.extend_from_slice(extra_env);
    harness
        .send_command_with_env(&cmd, &env)
        .expect("send claudine command");

    let capture = wait_for_exit_marker(harness, Duration::from_secs(45));
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    capture
}

/// The `R;G;B` triple of the truecolor foreground painting this line's bar.
///
/// tmux re-emits SGR in its own form, so the *sequence* is not stable across
/// captures — the color triple is. Returns `None` for a line whose bar is not
/// colored, which is the whole assertion under `NO_COLOR`.
fn bar_color(line: &str) -> Option<String> {
    let bar = line.find('│')?;
    let open = line[..bar].rfind("\u{1b}[38;2;")?;
    let start = open + "\u{1b}[38;2;".len();
    let end = start + line[start..].find('m')?;
    Some(line[start..end].to_string())
}

/// The column `needle` starts at, counting characters rather than bytes.
///
/// Columns, not bytes: `│` is one column and three UTF-8 bytes, so a byte
/// offset would report the two gutters as different widths when they are not.
fn column_of(line: &str, needle: &str) -> Option<usize> {
    line.find(needle).map(|byte| line[..byte].chars().count())
}

/// Every visible line of the run, from the command echo to the exit marker.
///
/// Bounding the region matters: the shell prompt and the echoed command line
/// both contain the task names, and an unbounded search would let them satisfy
/// an assertion the rendered frames were supposed to.
fn framed_region(plain: &str) -> Vec<&str> {
    plain
        .lines()
        .skip_while(|line| !line.contains("Starting pre-flight checks"))
        .take_while(|line| !line.trim_start().starts_with(EXIT_MARKER))
        .collect()
}

/// Every *rendered* line of the run, keeping escapes.
fn framed_region_raw<'a>(raw: &'a str, plain_marker: &str) -> Vec<&'a str> {
    raw.lines()
        .skip_while(|line| !line.contains(plain_marker))
        .take_while(|line| !line.contains(EXIT_MARKER))
        .collect()
}

/// The body a task actually printed must carry that task's own bar color,
/// **after** the terminal has merged the status and data channels into one
/// visible stream.
///
/// This is the assertion the L1 suite structurally cannot make. `sequence_groups.rs`
/// compares a header captured from the stderr pipe against a body captured from
/// the stdout pipe; a reader has neither pipe. Here both lines come out of the
/// same pane, in arrival order, re-rendered by tmux — which is the only place
/// the claim "you can tell whose output this is" is actually true or false.
#[test]
#[serial(level2_terminal)]
fn level2_parallel_task_bodies_carry_their_own_bar_color_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-taskstream-color", PARALLEL_DOC);
    let capture = run_in_pane(&mut harness, &staged, 100, 50, &[]);

    assert_eq!(
        capture.exit_code, 0,
        "the parallel group must succeed.\nplain:\n{}",
        capture.frame.plain
    );

    let lines = framed_region_raw(&capture.frame.raw, "Starting pre-flight checks");
    let mut colors = Vec::new();
    for (task, payload) in [("fetch-data", "fetched"), ("render-page", "rendered")] {
        let header = lines
            .iter()
            .find(|line| line.contains('▶') && line.contains(task))
            .unwrap_or_else(|| panic!("no header for `{task}`.\nraw region:\n{lines:#?}"));
        let body = lines
            .iter()
            .find(|line| line.contains(payload))
            .unwrap_or_else(|| panic!("`{payload}` never reached the pane.\nraw region:\n{lines:#?}"));

        let header_color = bar_color(header)
            .unwrap_or_else(|| panic!("header for `{task}` drew no colored bar: {header:?}"));
        let body_color = bar_color(body)
            .unwrap_or_else(|| panic!("body for `{task}` drew no colored bar: {body:?}"));
        assert_eq!(
            header_color, body_color,
            "`{task}`'s body line is attributed to a different task than its \
             header once both channels reach the same pane"
        );
        colors.push(body_color);
    }

    assert_ne!(
        colors[0], colors[1],
        "two concurrent tasks painted the same bar, so their interleaved body \
         lines are indistinguishable in the pane"
    );
}

/// Serial and parallel work must occupy identical geometry at a narrow width.
///
/// Regression guard, at the only level that could see the defect. The invisible
/// bar takes `BlockQuote`'s custom-prefix path, which folds nothing on its own;
/// with `Layout::default()`'s `word_wrap: None` a serial body line ran past the
/// pane edge and the *terminal* wrapped it, dropping the two-column gutter and
/// restarting the continuation at column 0. Separated pipes cannot show this —
/// they have no width to overflow. A 44-column pane can.
#[test]
#[serial(level2_terminal)]
fn level2_serial_and_parallel_share_one_left_edge_at_narrow_width_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    const COLS: usize = 44;

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-taskstream-narrow", MIXED_WIDTH_DOC);
    let capture = run_in_pane(&mut harness, &staged, COLS as u32, 60, &[]);

    assert_eq!(
        capture.exit_code, 0,
        "the mixed serial/parallel sequence must succeed.\nplain:\n{}",
        capture.frame.plain
    );

    let lines = framed_region(&capture.frame.plain);
    assert!(
        !lines.is_empty(),
        "no framed region in the pane.\nplain:\n{}",
        capture.frame.plain
    );

    // Nothing may reach the pane edge. A frame that does was folded by the
    // terminal rather than the renderer, and the terminal does not re-emit the
    // gutter.
    for line in &lines {
        assert!(
            line.chars().count() <= COLS,
            "a frame overflowed the {COLS}-column pane, so the terminal — not \
             the renderer — folded it: {line:?}\nregion:\n{lines:#?}"
        );
    }

    // Both payloads fold, and every fragment keeps its own gutter.
    let serial_body: Vec<&&str> = lines
        .iter()
        .filter(|line| line.starts_with("  ") && line.contains("alpha bravo"))
        .collect();
    let parallel_fragments: Vec<&&str> = lines
        .iter()
        .filter(|line| line.starts_with("│ "))
        .filter(|line| line.contains("alpha bravo") || line.contains("juliett"))
        .collect();
    assert!(
        !serial_body.is_empty(),
        "the serial task's body never rendered under an invisible bar.\nregion:\n{lines:#?}"
    );
    assert!(
        !parallel_fragments.is_empty(),
        "the parallel task's body never rendered under a colored bar.\nregion:\n{lines:#?}"
    );

    // The alignment contract: the first content column is the same in both
    // modes, so the stream does not lurch sideways when execution switches.
    let serial_column = column_of(serial_body[0], "alpha bravo")
        .expect("the serial body line contains its payload");
    let parallel_column = lines
        .iter()
        .find(|line| line.starts_with("│ ") && line.contains("alpha bravo"))
        .and_then(|line| column_of(line, "alpha bravo"))
        .expect("the parallel body line contains its payload");
    assert_eq!(
        serial_column, parallel_column,
        "serial and parallel bodies start at different columns, so the stream \
         lurches when execution switches modes.\nregion:\n{lines:#?}"
    );

    // The continuation of the folded serial line must still be inside the
    // gutter — this is the exact byte pattern the defect produced (a fragment
    // beginning at column 0).
    let juliett = lines
        .iter()
        .find(|line| line.contains("juliett") && !line.contains("alpha bravo"))
        .unwrap_or_else(|| panic!("the long payload never folded.\nregion:\n{lines:#?}"));
    assert!(
        juliett.starts_with("  ") || juliett.starts_with("│ "),
        "a folded continuation escaped its gutter and restarted at column 0: \
         {juliett:?}\nregion:\n{lines:#?}"
    );
}

/// With color gone, the pane must still say whose output each frame is.
///
/// The spec's degradation rule: attribution never depends on color alone. A
/// real `NO_COLOR` pane is where that is decided, because the capability
/// handshake — not a constructed `Terminal` — chooses the render path.
#[test]
#[serial(level2_terminal)]
fn level2_no_color_pane_keeps_textual_attribution_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-taskstream-nocolor", PARALLEL_DOC);
    let capture = run_in_pane(&mut harness, &staged, 100, 50, &[("NO_COLOR", "1")]);

    assert_eq!(
        capture.exit_code, 0,
        "the parallel group must succeed under NO_COLOR.\nplain:\n{}",
        capture.frame.plain
    );

    let raw_region = framed_region_raw(&capture.frame.raw, "Starting pre-flight checks");
    for line in &raw_region {
        assert!(
            bar_color(line).is_none(),
            "a bar was still painted under NO_COLOR: {line:?}"
        );
    }

    // Attribution has to survive in the text itself: each task announces and
    // closes by name.
    let plain_region = framed_region(&capture.frame.plain);
    let joined = plain_region.join("\n");
    for task in ["fetch-data", "render-page"] {
        assert!(
            plain_region
                .iter()
                .any(|line| line.contains('▶') && line.contains(task)),
            "`{task}` has no named header without color.\nregion:\n{joined}"
        );
        assert!(
            plain_region
                .iter()
                .any(|line| line.contains(task) && line.contains("succeeded")),
            "`{task}` has no named outcome footer without color.\nregion:\n{joined}"
        );
    }
    for payload in ["fetched", "rendered"] {
        assert!(
            joined.contains(payload),
            "`{payload}` never reached the pane without color.\nregion:\n{joined}"
        );
    }
}

/// A non-UTF-8 locale must degrade the header marker to its ASCII twin.
///
/// The capability is locale-derived, so only a process that actually inherits
/// the locale exercises the branch — a constructed `Terminal` sets the flag
/// directly and proves nothing about the wiring. Note this lever works only
/// *without* `NO_COLOR`: that path builds an optimistic terminal which hardcodes
/// `supports_unicode`, so the two degradations cannot be tested in one run.
#[test]
#[serial(level2_terminal)]
fn level2_non_utf8_locale_uses_the_ascii_header_glyph_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-taskstream-ascii", PARALLEL_DOC);
    let capture = run_in_pane(
        &mut harness,
        &staged,
        100,
        50,
        &[("LC_ALL", "C"), ("LANG", "C"), ("LC_CTYPE", "C")],
    );

    assert_eq!(
        capture.exit_code, 0,
        "the parallel group must succeed under a C locale.\nplain:\n{}",
        capture.frame.plain
    );

    let region = framed_region(&capture.frame.plain);
    let joined = region.join("\n");
    for task in ["fetch-data", "render-page"] {
        assert!(
            region
                .iter()
                .any(|line| line.contains("> ") && line.contains(task)),
            "`{task}`'s header did not fall back to the ASCII marker.\nregion:\n{joined}"
        );
    }
    assert!(
        !joined.contains('▶'),
        "the Unicode header marker survived a non-UTF-8 locale.\nregion:\n{joined}"
    );
}

/// A dynamic source resolving to nothing renders a styled notice and exits `0`.
///
/// The notice is a terminal-rendered `Prose`, so "styled" is a claim about what
/// a terminal emits. Asserted semantically — that the line carries *some* SGR —
/// rather than by byte, because tmux re-emits escapes in its own form.
#[test]
#[serial(level2_terminal)]
fn level2_zero_step_sequence_renders_a_styled_notice_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-taskstream-zerostep", ZERO_STEP_DOC);
    let capture = run_in_pane(&mut harness, &staged, 100, 50, &[]);

    assert_eq!(
        capture.exit_code, 0,
        "an empty dynamic source is a graceful no-op.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        capture.frame.plain.contains("0 steps"),
        "the zero-step notice never reached the pane.\nplain:\n{}",
        capture.frame.plain
    );

    let notice = capture
        .frame
        .raw
        .lines()
        .find(|line| line.contains("0 steps"))
        .expect("the notice is present in the raw capture");
    assert!(
        notice.contains('\u{1b}'),
        "the zero-step notice rendered unstyled: {notice:?}"
    );
}
