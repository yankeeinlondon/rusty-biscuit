//! Level 2 real-terminal capture tests for `claudine_gen::report`.
//!
//! The L1 tests in `src/report/tests.rs` render against a synthetic
//! `Terminal` whose TTY-ness and color depth are chosen by hand, so they
//! prove what the component *would* emit — not what a terminal displays. These
//! drive the report functions inside a real tmux pane and assert against the
//! bytes the emulator actually painted (`CapturedFrame`).
//!
//! ## The probe
//!
//! `report` is a library module with no CLI surface of its own, and an L2
//! probe must never ship as a production binary. So this test binary is its
//! own probe: [`level2_report_probe`] is an ordinary `#[test]` that does
//! nothing unless [`PROBE_ENV`] names a mode, in which case it prints that
//! mode's report to stdout. The driving tests re-exec `current_exe()` inside
//! the pane with `--exact level2_report_probe --nocapture`, which puts the
//! report on a real TTY without a shipped binary and without fabricating a
//! synthetic package area to drive `claudine-gen` against.
//!
//! Each test spawns its **own** fixed-size tmux session rather than attaching
//! to the `just test-l2` broker pane: the assertions are width-sensitive
//! (list wrapping, hanging indent) and the shared pane's geometry is not
//! ours to pick. Sessions are self-isolating and torn down per test.
//!
//! ## Coverage
//!
//! - **status styling** — `<green>clean</green>`, `<red>DRIFT</red>`, and
//!   `<yellow>…</yellow>` reach the emulator as SGR under `FORCE_COLOR=1`,
//!   with the surrounding prose left unstyled.
//! - **list indentation** — `capped_diff` / `detail_list` items carry the 2ch
//!   left margin from `TargetValue::universal(Length::ch(2))`.
//! - **cap + elision** — a 25-line diff shows 20 items and the elision line.
//! - **wrapping** — an over-long detail item wraps with a hanging indent past
//!   its `- ` marker instead of overflowing the pane.
//! - **placeholders** — `<slug>` in a path and `provenance`'s
//!   `<source unresolvable>` render literally, not swallowed as Prose markup.
//!
//! tmux is the portable headless backend and re-emits SGR faithfully; the
//! reports use no OSC8 or graphics, so a GUI emulator adds nothing here.
//!
//! Skip-clean: `TmuxHarness::available()` gates every test.
//! `BISCUIT_TEST_LEVEL_REQUIRED=2` turns a missing tmux into a hard failure.
//! Run via `just test-l2`.

#![cfg(unix)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use biscuit_test_harness::tmux::{TmuxHarness, kill_session_by_name, spawn_shell_session};
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use claudine_gen::generate::{CheckOutcome, Generation, Provenance, ResolvedField};
use claudine_gen::offerings::OfferingJoinReport;
use claudine_gen::report;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

/// Selects the probe mode when this binary is re-exec'd inside the pane.
/// Unset in a normal test run, which makes the probe entrypoint a no-op.
const PROBE_ENV: &str = "CLAUDINE_GEN_L2_PROBE";

/// The over-long unjoined offering id: long enough that its `- ` list item
/// must wrap inside a 100-cell pane, so the hanging-indent assertion is
/// exercised rather than trivially satisfied.
const LONG_OFFERING_ID: &str =
    "anthropic/claude-sonnet-with-a-deliberately-long-identifier-that-cannot-fit-on-one-line-at-all";

/// A path carrying an angle-bracket placeholder, which `esc` must keep
/// literal through Prose's markup pass.
const PLACEHOLDER_PATH: &str = "/repo/<slug>/data.rs";

/// Diff lines fed to the drift probe. 25 exceeds `capped_diff`'s 20-line cap,
/// so the rendered list must elide the last 5.
fn drift_details() -> Vec<String> {
    (0..25).map(|i| format!("line {i}: aaa vs bbb")).collect()
}

/// A `Generation` whose override has no resolvable suppressed source, which is
/// what drives `provenance`'s `<source unresolvable>` placeholder.
fn probe_generation() -> Generation {
    Generation {
        slug: "claude".to_string(),
        fields: vec![ResolvedField {
            field: "stream_protocol",
            value: serde_json::Value::String("json".to_string()),
            provenance: Provenance::Override {
                reason: "upstream is wrong".to_string(),
                suppressed: None,
                stale: false,
            },
        }],
        data_rs: String::new(),
        skips: Vec::new(),
        research: Default::default(),
        offering_join: OfferingJoinReport {
            joined: 3,
            total: 4,
            unjoined: vec![LONG_OFFERING_ID.to_string()],
            ambiguous_aliases: Vec::new(),
        },
        artifact_warning: None,
    }
}

/// The env-gated probe entrypoint (see the module docs). A no-op unless
/// re-exec'd with [`PROBE_ENV`] set.
#[test]
fn level2_report_probe() {
    let Ok(mode) = std::env::var(PROBE_ENV) else {
        return;
    };
    // The real production terminal builder: this is the code path
    // `FORCE_COLOR=1` and the pane's TTY-ness must flow through.
    let term = report::output_terminal();
    let out = match mode.as_str() {
        "status" => {
            let mut out = report::provider_check(&term, "claude", &CheckOutcome::Clean);
            out.push_str(&report::provider_check(
                &term,
                "kimi",
                &CheckOutcome::Drift {
                    details: drift_details(),
                },
            ));
            out.push_str(&report::provider_check(
                &term,
                "goose",
                &CheckOutcome::MissingCommitted {
                    path: PathBuf::from(PLACEHOLDER_PATH),
                },
            ));
            out
        }
        "provenance" => report::provenance(&term, &probe_generation()),
        other => panic!("unknown probe mode `{other}`"),
    };
    print!("{out}");
}

/// Runs the probe in `mode` inside a fresh `cols` × `rows` tmux session and
/// returns what the pane displayed.
fn capture_probe(mode: &str, cols: u32, rows: u32) -> CapturedFrame {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let session = format!(
        "claudine_gen_report_l2_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    // POSIX shell, not the developer's `$SHELL`: a custom login prompt (e.g.
    // Starship's `❯`) never ends in `$`/`#`/`%`, so `wait_for_prompt` would
    // never match and burn its full timeout twice.
    spawn_shell_session(&session, cols, rows).expect("failed to spawn tmux session");

    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let exe = std::env::current_exe().expect("current_exe");
    let cols_s = cols.to_string();
    let cmd = format!("'{}' --exact level2_report_probe --nocapture", exe.display());
    let send = harness.send_command_with_env(
        &cmd,
        &[
            (PROBE_ENV, mode),
            ("FORCE_COLOR", "1"),
            ("COLUMNS", cols_s.as_str()),
        ],
    );
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(Duration::from_millis(250));
    let frame = harness.capture();
    kill_session_by_name(&session);

    send.expect("send probe command");
    frame.expect("capture failed")
}

/// The report lines, with the shell's echo of the typed command and libtest's
/// own harness chatter dropped.
///
/// The echoed line carries the absolute test-binary path and can exceed the
/// pane width; it is shell noise, not report output, so it must not reach the
/// content assertions.
fn report_lines(frame: &CapturedFrame) -> Vec<&str> {
    frame
        .plain
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.contains("level2_report_probe"))
        .filter(|l| !l.starts_with("running ") && !l.starts_with("test result:"))
        .collect()
}

/// Asserts `needle` reached the pane inside a `\x1b[<code>m` SGR run.
///
/// tmux re-emits SGR rather than echoing the source bytes, and may split or
/// merge runs, so this looks for the opening code anywhere on the needle's
/// line rather than demanding byte-equality. Both the semicolon form and the
/// ITU colon form are accepted.
fn assert_styled(frame: &CapturedFrame, needle: &str, code: &str) {
    let line = frame
        .raw
        .lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("`{needle}` not found in raw capture:\n{}", frame.raw));
    let forms = [
        format!("\x1b[{code}m"),
        format!("\x1b[{code};"),
        format!("\x1b[{code}:"),
    ];
    assert!(
        forms.iter().any(|f| line.contains(f.as_str())),
        "`{needle}` must carry SGR {code} in a real terminal.\nline: {line:?}",
    );
}

/// Status report in a real terminal: the three status keywords arrive styled,
/// the surrounding prose is intact, the drift diff renders as a 2ch-indented
/// list capped at 20 items with an elision line, and the `<slug>` placeholder
/// survives literally.
#[test]
#[serial(level2_terminal)]
fn level2_report_status_styled_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let frame = capture_probe("status", 100, 60);
    let lines = report_lines(&frame);

    assert!(
        lines.contains(&"claude: clean (inputs match the committed data.rs)"),
        "clean line must render with its prose intact.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("kimi: DRIFT — regenerate with")),
        "drift header must render with its prose intact.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("goose: committed data.rs missing at ")),
        "missing-committed line must render with its prose intact.\nplain:\n{}",
        frame.plain,
    );

    assert_styled(&frame, "clean", "32");
    assert_styled(&frame, "DRIFT", "31");
    assert_styled(&frame, "committed data.rs missing", "33");

    // The `<slug>` placeholder is dynamic text: Prose must render it
    // literally, never consume it as markup.
    assert!(
        lines.iter().any(|l| l.contains(PLACEHOLDER_PATH)),
        "the `<slug>` placeholder must render literally.\nplain:\n{}",
        frame.plain,
    );

    // The diff list: 20 of 25 items at the 2ch left margin, then the elision.
    let items: Vec<&&str> = lines
        .iter()
        .filter(|l| l.starts_with("  - line "))
        .collect();
    assert_eq!(
        items.len(),
        20,
        "diff list must show 20 items at the 2ch left margin.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        lines.contains(&"  - … 5 more differing lines"),
        "diff list must elide the 5 lines past the cap.\nplain:\n{}",
        frame.plain,
    );
}

/// Provenance report in a real terminal: the `<source unresolvable>`
/// placeholder renders literally, and an over-long detail item wraps with a
/// hanging indent past its `- ` marker rather than overflowing the pane.
#[test]
#[serial(level2_terminal)]
fn level2_report_provenance_wraps_and_keeps_placeholder_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let cols = 100usize;
    let frame = capture_probe("provenance", cols as u32, 40);
    let lines = report_lines(&frame);

    assert!(
        lines.iter().any(|l| l
            == &"  override active: stream_protocol (suppressing <source unresolvable>) — upstream is wrong"),
        "the `<source unresolvable>` placeholder must render literally.\nplain:\n{}",
        frame.plain,
    );

    // The long unjoined id cannot fit on one line, so its list item must wrap
    // onto a continuation indented past the `- ` marker (never truncate, never
    // overflow the pane).
    let marker = lines
        .iter()
        .position(|l| l.starts_with("  - unjoined: "))
        .unwrap_or_else(|| panic!("no unjoined list item.\nplain:\n{}", frame.plain));
    let continuation = lines
        .get(marker + 1)
        .unwrap_or_else(|| panic!("unjoined item did not wrap.\nplain:\n{}", frame.plain));
    assert!(
        continuation.starts_with("    ") && !continuation.trim_start().starts_with("- "),
        "wrapped list items must hang-indent past the `- ` bullet.\nline: {continuation:?}\nplain:\n{}",
        frame.plain,
    );
    let rejoined = format!(
        "{}{}",
        lines[marker].trim_start_matches("  - unjoined: ").trim_end(),
        continuation.trim_start()
    );
    assert_eq!(
        rejoined, LONG_OFFERING_ID,
        "the wrapped id must be split, not truncated.\nplain:\n{}",
        frame.plain,
    );

    for line in &lines {
        assert!(
            line.chars().count() <= cols,
            "no report line may exceed the {cols}-col pane.\nline: {line:?}",
        );
    }
}
