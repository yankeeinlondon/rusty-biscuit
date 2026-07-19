//! Level 2 real-terminal capture for sequence task-stream rendering.
//!
//! Feature: `claudine/features/2026-07-11-sequence-plus/` (spec →
//! *Reporting Concurrency*), review-2 finding 4 and review-3 finding 3.
//!
//! ## Shell fixtures vs. prompt fixtures
//!
//! The shell fixtures here exercise the task decorator directly. The prompt
//! fixtures additionally cross the provider protocol, the semantic parser, and
//! the live semantic sink before reaching the same decorator — the hops
//! review-2 found were bypassing it. They stub `claude`, not `goose`, because
//! `goose` carries no `stream_protocol` and so never reaches the semantic
//! spawn at all.
//!
//! Both surviving provider paths are covered here. A third — a
//! post-parser-failure raw fallback — used to exist and is now gone: no
//! `SemanticStreamParser` implementation ever constructed the
//! `StreamParseError::Fatal` that gated it, so neither scripted input nor any
//! real provider stream could reach it. Review-4 finding 6 removed the whole
//! parser error channel rather than leave an emitter no input can drive.
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

/// A parallel group of two **prompt** tasks, both driven by the same scripted
/// provider stub.
///
/// Prompt tasks, not shell: a `shell:` body reaches the pane through the task
/// decorator directly, whereas a prompt task's text first crosses the provider
/// protocol, the semantic parser, and the stdout/stderr merge. Those are the
/// hops that can strip attribution, and only a prompt fixture walks them.
const PARALLEL_PROMPT_DOC: &str = "\
---
sequence:
  - name: alpha
    group:
      name: bundle
      execution: parallel
      tasks:
        - name: task-one
          prompt: one.md
        - name: task-two
          prompt: two.md
---

Body.
";

/// A single prompt task whose provider holds a partial Markdown block, then
/// stays alive well past the idle-flush window.
const IDLE_FLUSH_DOC: &str = "\
---
sequence:
  - name: alpha
    group:
      name: bundle
      execution: parallel
      tasks:
        - name: slow-task
          prompt: one.md
---

Body.
";

/// Scripted `claude` stream-json emitting one assistant block, one reasoning
/// delta, and one tool call — the three provider-side shapes whose task
/// attribution review-2 repaired.
///
/// `claude` rather than `goose`: `goose` has no `stream_protocol`, so it never
/// reaches the semantic spawn at all. Every pre-existing sequence fixture stubs
/// `goose` and therefore proves nothing about the provider paths.
const CLAUDE_STREAM_STUB: &str = r#"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"init","session_id":"stub-1","model":"stub-model"}'
printf '%s\n' '{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"weighing-options"}}'
printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"body-payload"}]}}'
printf '%s\n' '{"type":"tool_use","name":"Bash","input":{"command":"stub-tool-call"}}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"result":"done"}'
exit 0
"#;

/// Scripted `claude` that emits one *held* Markdown block and then stalls.
///
/// The trailing newline is load-bearing: `AssistantStream::append` streams a
/// partial line immediately, but a complete, non-list, non-sentence-terminated
/// line goes into the block buffer and is held. Only the idle flush can then
/// surface it — which is the path under test.
///
/// The post-stall marker is what makes the test discriminating. The
/// end-of-stream `close()` drain also flushes a held block and also decorates
/// it, so "the block reached the pane with a bar" is true either way. Emitting
/// a later event *after* the stall separates them: an idle flush puts the block
/// ahead of that marker, `close()` puts it behind.
///
/// The stall spans the ticker's *second* tick on purpose. Cadence and silence
/// window are both 30s, so the first tick only flushes if the block was stamped
/// within a hair of the ticker starting; under pane setup and load it usually
/// is not, and the block then waits for t=60s. A 45s stall made that a coin
/// flip — the run ended first and `close()` drained the block.
const CLAUDE_IDLE_STUB: &str = r#"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"init","session_id":"stub-idle","model":"stub-model"}'
printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"held-idle-block\n"}]}}'
sleep 75
printf '%s\n' '{"type":"tool_use","name":"Bash","input":{"command":"post-stall-marker"}}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"result":"done"}'
exit 0
"#;

struct Staged {
    workspace: TestWorkspace,
    bin_dir: PathBuf,
    doc: PathBuf,
    /// The `--<provider>` selector `run_in_pane` passes to `claudine sequence`.
    provider_flag: &'static str,
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
        provider_flag: "--goose",
    }
}

/// Stage a workspace whose prompt tasks are served by `stub`, a scripted
/// `claude` stream-json script on `PATH`.
///
/// A shell script, not a test-only binary: an L2 probe must not ship as a
/// production bin, and `PATH` interposition is how every other provider
/// fixture in this crate injects a deterministic stream.
fn stage_prompt(name: &str, doc_body: &str, stub: &str) -> Staged {
    let mut staged = stage(name, doc_body);
    let root = staged.workspace.path().to_path_buf();

    write_executable(&staged.bin_dir.join("claude"), stub);
    for member in ["one.md", "two.md"] {
        write(&root.join(member), "---\nprompt: run\n---\n\nMember.\n");
    }
    write(&staged.doc, doc_body);
    staged.provider_flag = "--claude";
    staged
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
    run_in_pane_within(harness, staged, cols, rows, extra_env, Duration::from_secs(45))
}

/// [`run_in_pane`] with an explicit deadline, for fixtures that deliberately
/// outlast the default (the idle-flush stall).
fn run_in_pane_within(
    harness: &mut TmuxHarness,
    staged: &Staged,
    cols: u32,
    rows: u32,
    extra_env: &[(&str, &str)],
    deadline: Duration,
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
        "{claudine} sequence {} --yolo {}; echo {EXIT_MARKER}$?",
        staged.provider_flag,
        staged.doc.display()
    );
    let mut env: Vec<(&str, &str)> = vec![("HOME", home.as_str()), ("PATH", path.as_str())];
    env.extend_from_slice(extra_env);
    harness
        .send_command_with_env(&cmd, &env)
        .expect("send claudine command");

    let capture = wait_for_exit_marker(harness, deadline);
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

/// Every line a *prompt* task emits — assistant body, reasoning, tool status —
/// must still name its owner once it has crossed the provider protocol and the
/// stdout/stderr merge.
///
/// This is the review-3 finding-3 gap. The shell fixtures above prove the bar
/// renderer; they never reach `run_child_stream_semantic`, because `goose` has
/// no stream protocol. A `claude` stub does, so this is the only case in which
/// the parser, the live semantic sink, and the task decorator are all in the
/// path at once — and the pane is the only place their combined output is
/// judged, since a reader has neither pipe.
///
/// Reasoning and tool lines are *status* (stderr) while the body is *data*
/// (stdout). The channel split itself is asserted at L1
/// (`stream_io.rs`, `sequence_groups.rs::parallel_prompt_task_splits_data_and_status`);
/// what only a pane can show is that both channels still carry the same bar
/// after being interleaved.
#[test]
#[serial(level2_terminal)]
fn level2_parallel_prompt_streams_keep_task_attribution_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage_prompt(
        "claudine-l2-prompt-attribution",
        PARALLEL_PROMPT_DOC,
        CLAUDE_STREAM_STUB,
    );
    // A tall pane, because `capture()` has no scrollback and a prompt run
    // renders two system-prompt panels on top of the task stream.
    let capture = run_in_pane(&mut harness, &staged, 110, 130, &[]);

    assert_eq!(
        capture.exit_code, 0,
        "the parallel prompt group must succeed.\nplain:\n{}",
        capture.frame.plain
    );

    let lines = framed_region_raw(&capture.frame.raw, "Starting pre-flight checks");
    let mut header_colors = Vec::new();
    for task in ["task-one", "task-two"] {
        let header = lines
            .iter()
            .find(|line| line.contains('▶') && line.contains(task))
            .unwrap_or_else(|| panic!("no header for `{task}`.\nraw region:\n{lines:#?}"));
        header_colors.push(
            bar_color(header)
                .unwrap_or_else(|| panic!("header for `{task}` drew no colored bar: {header:?}")),
        );
    }
    assert_ne!(
        header_colors[0], header_colors[1],
        "two concurrent prompt tasks painted the same bar, so their interleaved \
         provider output is indistinguishable in the pane"
    );

    // Each of the three repaired provider paths emits one identifiable marker.
    // Both tasks run the same stub, so each marker must appear once per task —
    // and the set of bars painting it must be exactly the set of task bars.
    for (path, marker) in [
        ("assistant body (data)", "body-payload"),
        ("provider reasoning (status)", "weighing-options"),
        ("provider tool status (status)", "stub-tool-call"),
    ] {
        let painted: Vec<String> = lines
            .iter()
            .filter(|line| line.contains(marker))
            .map(|line| {
                bar_color(line).unwrap_or_else(|| {
                    panic!("`{path}` reached the pane with no bar at all: {line:?}")
                })
            })
            .collect();
        let mut distinct: Vec<String> = painted.clone();
        distinct.sort();
        distinct.dedup();
        let mut expected = header_colors.clone();
        expected.sort();
        assert_eq!(
            distinct, expected,
            "`{path}` (marker `{marker}`) is not attributed to both owning \
             tasks: saw bars {painted:?}, task bars are {header_colors:?}\
             \nraw region:\n{lines:#?}"
        );
    }

    // Textual attribution must survive alongside the color, per the spec's
    // degradation rule — the footer names the task that produced the stream.
    let plain = framed_region(&capture.frame.plain).join("\n");
    for task in ["task-one", "task-two"] {
        assert!(
            plain
                .lines()
                .any(|line| line.contains(task) && line.contains("succeeded")),
            "`{task}` has no named outcome footer.\nregion:\n{plain}"
        );
    }
}

/// A held Markdown block surfaced by the idle flush must still carry its task's
/// bar when it lands, mid-run, in a real pane.
///
/// The stub emits one complete non-terminal line and then stalls, so the block
/// buffer sits untouched and only `flush_idle` can release it. The stall is
/// what makes the assertion honest: the marker appears while the provider is
/// still running, so it cannot have come from the end-of-stream drain.
///
/// Slow by construction — `SILENCE_WINDOW` and the ticker cadence are both a
/// hardcoded 30s with no env override, and the stall has to clear the second
/// tick to be reliable, so ~80s is the cheapest honest version of this test.
/// `.config/nextest.toml` carries the matching slow-timeout grant.
#[test]
#[serial(level2_terminal)]
fn level2_prompt_idle_flush_keeps_the_task_bar_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage_prompt(
        "claudine-l2-prompt-idleflush",
        IDLE_FLUSH_DOC,
        CLAUDE_IDLE_STUB,
    );
    let capture = run_in_pane_within(
        &mut harness,
        &staged,
        110,
        130,
        &[],
        Duration::from_secs(150),
    );

    assert_eq!(
        capture.exit_code, 0,
        "the stalled prompt task must still succeed.\nplain:\n{}",
        capture.frame.plain
    );

    let lines = framed_region_raw(&capture.frame.raw, "Starting pre-flight checks");
    let header = lines
        .iter()
        .find(|line| line.contains('▶') && line.contains("slow-task"))
        .unwrap_or_else(|| panic!("no header for `slow-task`.\nraw region:\n{lines:#?}"));
    let header_color = bar_color(header)
        .unwrap_or_else(|| panic!("`slow-task`'s header drew no colored bar: {header:?}"));

    let flushed_at = lines
        .iter()
        .position(|line| line.contains("held-idle-block"))
        .unwrap_or_else(|| {
            panic!("the held block never reached the pane.\nraw region:\n{lines:#?}")
        });
    let flushed_color = bar_color(lines[flushed_at]).unwrap_or_else(|| {
        panic!(
            "the idle-flushed block reached the pane with no bar: {:?}",
            lines[flushed_at]
        )
    });
    assert_eq!(
        header_color, flushed_color,
        "the idle flush surfaced the held block under a different task's bar \
         than the header that announced it"
    );

    // Proof that this was the idle flush and not the end-of-stream drain: the
    // block is ahead of an event the provider only emitted after its stall.
    let post_stall_at = lines
        .iter()
        .position(|line| line.contains("post-stall-marker"))
        .unwrap_or_else(|| {
            panic!("the post-stall marker never reached the pane.\nraw region:\n{lines:#?}")
        });
    assert!(
        flushed_at < post_stall_at,
        "the held block surfaced only after the provider resumed, so `close()` \
         drained it and the idle flush never ran.\nraw region:\n{lines:#?}"
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
