//! Level-2 tests for Mermaid diagram rendering.
//!
//! These tests run the `bt` diagram subcommands inside a real terminal
//! emulator to validate that image-protocol bytes (or fallback fenced
//! blocks) are emitted correctly.
//!
//! ## Skip-clean contract
//!
//! Every test checks `harness.available()` before spawning. When the
//! required terminal is absent the test prints `skipping: requires <X>`
//! to stderr and returns immediately. No `#[ignore]` markers are used.
//!
//! ## Coverage matrix
//!
//! Phase 4 of the L1/L2 review-fix plan adds one Level-2 test for every
//! diagram subcommand named in spec §5.4 against both Kitty and WezTerm
//! harnesses:
//!
//! - **Kitty path**: asserts `frame.raw` contains `\x1b_G` APC graphics
//!   bytes. Kitty preserves graphics protocol bytes in its `get-text
//!   --ansi --extent=screen` capture, so this is the strong-evidence
//!   path.
//! - **WezTerm path**: WezTerm's `get-text --escapes` strips image
//!   bytes from the captured pane (cells become spacers). The tests
//!   therefore use `bt <subcmd> --meta` and assert on the JSON
//!   metadata keys (`filename`, `render_time_ms`) emitted to stderr,
//!   which proves the renderer ran the image pipeline (not the fenced-
//!   block fallback).
//! - **tmux fallback**: tmux has no image protocol, so a single
//!   negative-control test asserts the fenced ` ```mermaid ` fallback
//!   fires and that no APC/OSC image bytes leak into the capture.
//!
//! The 10 subcommands covered: `flowchart`, `quadrant`, `pie-chart`,
//! `git-graph`, `bar-chart`, `line-chart`, `timeline`, `state-diagram`,
//! `erd`, `graph-expression`.

mod common;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use common::pane_geometry::{extract_kitty_apc_columns, parse_debug_image_width};
use common::{capture_until, send_bt_command};
use serial_test::serial;
use sha2::{Digest, Sha256};
use std::time::Duration;
use test_toolkit::{Backend, Level, require_level};

/// Process-shared Kitty window reused across the Kitty-targeting tests
/// in this file. A `clear` is sent before each command so prior renders
/// cannot leak into the capture window.
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();

/// Process-shared WezTerm pane reused across the WezTerm-targeting
/// tests in this file.
static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();

/// Process-shared tmux session reused across the tmux fallback test(s)
/// in this file.
static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();

/// Upper bound on how long a diagram render may take before its
/// terminal evidence (APC graphics bytes or `--meta` JSON) appears.
///
/// SVG generation plus rasterization can take 500 ms+ on a cold cache.
/// This is a *poll deadline*, not a blind sleep — [`capture_until`]
/// returns as soon as the evidence shows up, so the common (warm)
/// render costs only a poll interval, not the full bound.
const DIAGRAM_RENDER_TIMEOUT: Duration = Duration::from_secs(5);

/// One row of the Phase-4 coverage matrix: a diagram subcommand and the
/// argument string that produces a valid render.
struct DiagramCase {
    /// The `bt` subcommand name (e.g. `"flowchart"`).
    cmd: &'static str,
    /// The argument string after the subcommand. Must be a single
    /// shell-safe argument (already quoted by the caller as needed).
    arg: &'static str,
}

/// Coverage matrix for Phase 4: every diagram subcommand listed in
/// spec §5.4. Each entry is exercised against both Kitty and WezTerm
/// harnesses.
const CASES: &[DiagramCase] = &[
    DiagramCase {
        cmd: "flowchart",
        arg: "\"A --> B\"",
    },
    DiagramCase {
        cmd: "quadrant",
        arg: "\"P1: [0.4, 0.6]\"",
    },
    DiagramCase {
        cmd: "pie-chart",
        arg: "\"A: 1\"",
    },
    DiagramCase {
        cmd: "git-graph",
        arg: "\"commit\"",
    },
    DiagramCase {
        cmd: "bar-chart",
        arg: "\"1,2,3\"",
    },
    DiagramCase {
        cmd: "line-chart",
        arg: "\"1,2,3\"",
    },
    DiagramCase {
        cmd: "timeline",
        arg: "\"2024: launch\"",
    },
    DiagramCase {
        cmd: "state-diagram",
        arg: "\"[*] --> Idle\"",
    },
    DiagramCase {
        cmd: "erd",
        arg: "\"Customer ||--o{ Order : places\"",
    },
    DiagramCase {
        cmd: "graph-expression",
        arg: "\"a -> b\"",
    },
];

fn case_for(cmd: &str) -> &'static DiagramCase {
    CASES
        .iter()
        .find(|c| c.cmd == cmd)
        .unwrap_or_else(|| panic!("Phase-4 CASES table missing entry for `{cmd}`"))
}

// ------------------------------------------------------------------
// Shared assertion helpers
// ------------------------------------------------------------------

/// Asserts the diagram in `case` renders via the Kitty graphics
/// protocol, verifying `\x1b_G` APC bytes appear in the captured raw
/// pane output.
fn assert_renders_in_kitty(case: &DiagramCase) {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    // Reset the pane so prior tests' rendered output cannot leak into
    // this run's capture window.
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    send_bt_command(harness, &format!("{} {}", case.cmd, case.arg));
    let frame = capture_until(harness, DIAGRAM_RENDER_TIMEOUT, |f| {
        f.raw.contains("\x1b_G")
    });
    assert!(
        frame.raw.contains("\x1b_G"),
        "expected raw output to contain Kitty graphics protocol APC \
         sequence (\\x1b_G) for `bt {} {}`. raw first 400:\n{:?}\n\
         plain:\n{}",
        case.cmd,
        case.arg,
        frame.raw.chars().take(400).collect::<String>(),
        frame.plain,
    );
}

/// Asserts the diagram in `case` renders via the WezTerm path. Because
/// `wezterm cli get-text --escapes` strips image protocol bytes from
/// captured panes, this asserts on the `--meta` JSON keys (`filename`,
/// `render_time_ms`) that the CLI emits to stderr — proving the image
/// pipeline executed end-to-end (not the fenced-block fallback).
fn assert_renders_in_wezterm(case: &DiagramCase) {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    // Reset the pane so prior tests' rendered output cannot leak into
    // this run's capture window.
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    send_bt_command(harness, &format!("{} --meta {}", case.cmd, case.arg));
    let frame = capture_until(harness, DIAGRAM_RENDER_TIMEOUT, |f| {
        let joined: String = f.plain.lines().collect();
        joined.contains("\"filename\"") && joined.contains("\"render_time_ms\"")
    });
    let joined: String = frame.plain.lines().collect();

    // The presence of meta JSON in stderr proves the renderer ran the
    // image pipeline. The fenced-block fallback never emits these
    // keys.
    assert!(
        joined.contains("\"filename\"") && joined.contains("\"render_time_ms\""),
        "expected meta JSON (`filename`, `render_time_ms`) for \
         `bt {} --meta {}`. plain:\n{}",
        case.cmd,
        case.arg,
        frame.plain,
    );

    // True-negative: the fenced fallback must NOT have fired in WezTerm
    // (which has working iTerm2 graphics). If we see ```mermaid in the
    // capture, the renderer fell back and the test should fail.
    assert!(
        !joined.contains("```mermaid"),
        "WezTerm pane unexpectedly shows the fenced-block fallback for \
         `bt {} --meta {}`; image pipeline appears to have failed.\n\
         plain:\n{}",
        case.cmd,
        case.arg,
        frame.plain,
    );
}

// ------------------------------------------------------------------
// Kitty coverage matrix — 10 subcommands
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_flowchart_renders_in_kitty() {
    assert_renders_in_kitty(case_for("flowchart"));
}

#[test]
#[serial(level2_terminal)]
fn level2_quadrant_renders_in_kitty() {
    assert_renders_in_kitty(case_for("quadrant"));
}

#[test]
#[serial(level2_terminal)]
fn level2_pie_chart_renders_in_kitty() {
    assert_renders_in_kitty(case_for("pie-chart"));
}

#[test]
#[serial(level2_terminal)]
fn level2_git_graph_renders_in_kitty() {
    assert_renders_in_kitty(case_for("git-graph"));
}

#[test]
#[serial(level2_terminal)]
fn level2_bar_chart_renders_in_kitty() {
    assert_renders_in_kitty(case_for("bar-chart"));
}

#[test]
#[serial(level2_terminal)]
fn level2_line_chart_renders_in_kitty() {
    assert_renders_in_kitty(case_for("line-chart"));
}

#[test]
#[serial(level2_terminal)]
fn level2_timeline_renders_in_kitty() {
    assert_renders_in_kitty(case_for("timeline"));
}

#[test]
#[serial(level2_terminal)]
fn level2_state_diagram_renders_in_kitty() {
    assert_renders_in_kitty(case_for("state-diagram"));
}

#[test]
#[serial(level2_terminal)]
fn level2_erd_renders_in_kitty() {
    assert_renders_in_kitty(case_for("erd"));
}

#[test]
#[serial(level2_terminal)]
fn level2_graph_expression_renders_in_kitty() {
    assert_renders_in_kitty(case_for("graph-expression"));
}

// ------------------------------------------------------------------
// WezTerm coverage matrix — 10 subcommands
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_flowchart_renders_in_wezterm() {
    assert_renders_in_wezterm(case_for("flowchart"));
}

#[test]
#[serial(level2_terminal)]
fn level2_quadrant_renders_in_wezterm() {
    assert_renders_in_wezterm(case_for("quadrant"));
}

#[test]
#[serial(level2_terminal)]
fn level2_pie_chart_renders_in_wezterm() {
    assert_renders_in_wezterm(case_for("pie-chart"));
}

#[test]
#[serial(level2_terminal)]
fn level2_git_graph_renders_in_wezterm() {
    assert_renders_in_wezterm(case_for("git-graph"));
}

#[test]
#[serial(level2_terminal)]
fn level2_bar_chart_renders_in_wezterm() {
    assert_renders_in_wezterm(case_for("bar-chart"));
}

#[test]
#[serial(level2_terminal)]
fn level2_line_chart_renders_in_wezterm() {
    assert_renders_in_wezterm(case_for("line-chart"));
}

#[test]
#[serial(level2_terminal)]
fn level2_timeline_renders_in_wezterm() {
    assert_renders_in_wezterm(case_for("timeline"));
}

#[test]
#[serial(level2_terminal)]
fn level2_state_diagram_renders_in_wezterm() {
    assert_renders_in_wezterm(case_for("state-diagram"));
}

#[test]
#[serial(level2_terminal)]
fn level2_erd_renders_in_wezterm() {
    assert_renders_in_wezterm(case_for("erd"));
}

#[test]
#[serial(level2_terminal)]
fn level2_graph_expression_renders_in_wezterm() {
    assert_renders_in_wezterm(case_for("graph-expression"));
}

// ------------------------------------------------------------------
// Width and inverse
// ------------------------------------------------------------------

/// Verifies that `--width 50%` actually resolves to ~half the pane's
/// column count (±1 cell rounding tolerance).
///
/// Strategy: query the WezTerm pane geometry via `wezterm cli list`,
/// then run `bt pie-chart --debug --width 50%` and parse the resolved
/// `image width: N cells` line from the debug output. Compare to
/// `round(pane_cols * 0.5)`.
///
/// This would fail under regressions that:
/// - ignore `--width 50%` and fall back to `$COLUMNS`
/// - hard-code a default cell count
/// - apply percentages against the wrong base (e.g., terminal pixel
///   width instead of column count)
#[test]
#[serial(level2_terminal)]
fn level2_diagram_width_respects_pane_columns() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    let pane_size = harness.pane_size().expect("pane_size failed");
    let pane_cols = pane_size.cols;
    assert!(pane_cols > 0, "pane_size.cols was zero");

    send_bt_command(harness, "pie-chart --debug --width 50% \"A: 1\"");
    let frame = capture_until(harness, DIAGRAM_RENDER_TIMEOUT, |f| {
        parse_debug_image_width(&f.plain).is_some()
    });
    let actual = parse_debug_image_width(&frame.plain).unwrap_or_else(|| {
        panic!(
            "could not parse `image width:` line from `bt pie-chart --debug` output.\n\
             pane cols: {pane_cols}\nplain:\n{}",
            frame.plain
        )
    });
    let expected = (pane_cols as f32 * 0.5).round() as u32;
    let diff = (actual as i32 - expected as i32).abs();
    assert!(
        diff <= 1,
        "expected --width 50% to render ~{expected} cells (±1) for pane cols={pane_cols}; \
         got {actual}. plain:\n{}",
        frame.plain
    );

    let joined: String = frame.plain.lines().collect();
    assert!(
        !joined.contains("```mermaid"),
        "fenced-block fallback fired despite WezTerm having iTerm2 graphics. plain:\n{}",
        frame.plain
    );
}

/// Kitty companion to [`level2_diagram_width_respects_pane_columns`].
///
/// Uses the strong-evidence Level-2 path: parse the `c=N` parameter
/// from the Kitty graphics protocol APC payload that the diagram
/// renderer streams to the terminal, and compare to
/// `round(pane_cols * 0.5)` (±1 cell tolerance).
///
/// This proves the actual user-visible image-cell width matches the
/// requested 50% spec — independent of any debug stderr scaffolding.
#[test]
#[serial(level2_terminal)]
fn level2_diagram_width_kitty_apc_columns() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    let pane_cols = harness.pane_cols().expect("pane_cols failed");
    assert!(pane_cols > 0, "kitty pane_cols was zero");

    send_bt_command(harness, "pie-chart --width 50% \"A: 1\"");
    let frame = capture_until(harness, DIAGRAM_RENDER_TIMEOUT, |f| {
        extract_kitty_apc_payload(&f.raw).is_some()
    });
    let payload = extract_kitty_apc_payload(&frame.raw).unwrap_or_else(|| {
        panic!(
            "could not extract Kitty APC payload from --width 50% render.\n\
             pane cols: {pane_cols}\nraw first 400:\n{:?}",
            frame.raw.chars().take(400).collect::<String>(),
        )
    });

    let expected = (pane_cols as f32 * 0.5).round() as u32;

    if let Some(actual) = extract_kitty_apc_columns(&payload) {
        let diff = (actual as i32 - expected as i32).abs();
        assert!(
            diff <= 1,
            "Kitty APC c={actual}, expected ~{expected} (±1) for pane cols={pane_cols}.\n\
             payload first 200:\n{:?}",
            payload.chars().take(200).collect::<String>(),
        );
    } else {
        // Some Kitty versions emit width-only protocol bytes without an
        // explicit `c=` parameter. In that case fall back to the same
        // `--debug` parsing strategy used by the WezTerm test, run
        // under the Kitty harness so the protocol path is unchanged.
        send_bt_command(harness, "pie-chart --debug --width 50% \"A: 1\"");
        let frame_dbg = capture_until(harness, DIAGRAM_RENDER_TIMEOUT, |f| {
            parse_debug_image_width(&f.plain).is_some()
        });
        let actual = parse_debug_image_width(&frame_dbg.plain).unwrap_or_else(|| {
            panic!(
                "Kitty APC payload had no `c=` parameter AND `--debug` did not emit \
                 an `image width:` line.\npayload first 200:\n{:?}\nplain:\n{}",
                payload.chars().take(200).collect::<String>(),
                frame_dbg.plain,
            )
        });
        let diff = (actual as i32 - expected as i32).abs();
        assert!(
            diff <= 1,
            "Kitty --debug image width={actual}, expected ~{expected} (±1) for pane \
             cols={pane_cols}.\nplain:\n{}",
            frame_dbg.plain,
        );
    }

    let joined: String = frame.plain.lines().collect();
    assert!(
        !joined.contains("```mermaid"),
        "fenced-block fallback fired in Kitty despite graphics support. plain:\n{}",
        frame.plain
    );
}

/// Phase-4 rewrite: compare sha256 hashes of the Kitty APC graphics
/// payloads emitted by `bt flowchart` vs `bt flowchart --inverse`.
///
/// The previous implementation compared `--meta` cache filenames, which
/// only proved the **PNG cache key** differs, not that the rendered
/// bytes streamed to the terminal differ. This test now slices the APC
/// payload (`\x1b_G ... \x1b\\`) out of each capture, hashes it, and
/// asserts the hashes diverge — guaranteeing the user-visible bytes
/// differ, not just metadata.
#[test]
#[serial(level2_terminal)]
fn level2_inverse_flag_changes_background_in_capture() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");

    // Default render. Clear so the prior test's APC payload cannot leak
    // into the capture window.
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    send_bt_command(harness, "flowchart \"A --> B\"");
    let frame1 = capture_until(harness, DIAGRAM_RENDER_TIMEOUT, |f| {
        extract_kitty_apc_payload(&f.raw).is_some()
    });
    let payload1 = extract_kitty_apc_payload(&frame1.raw).unwrap_or_else(|| {
        panic!(
            "could not extract Kitty APC payload from default render.\n\
             raw first 400:\n{:?}",
            frame1.raw.chars().take(400).collect::<String>(),
        )
    });

    // Inverse render. Clear again so the default-render APC bytes do
    // not survive into this command's capture window.
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    send_bt_command(harness, "flowchart --inverse \"A --> B\"");
    let frame2 = capture_until(harness, DIAGRAM_RENDER_TIMEOUT, |f| {
        extract_kitty_apc_payload(&f.raw).is_some()
    });
    let payload2 = extract_kitty_apc_payload(&frame2.raw).unwrap_or_else(|| {
        panic!(
            "could not extract Kitty APC payload from --inverse render.\n\
             raw first 400:\n{:?}",
            frame2.raw.chars().take(400).collect::<String>(),
        )
    });

    let hash1 = sha256_hex(payload1.as_bytes());
    let hash2 = sha256_hex(payload2.as_bytes());

    assert_ne!(
        hash1, hash2,
        "expected --inverse to produce a different rendered byte stream \
         than default. sha256(default)={hash1} sha256(inverse)={hash2}",
    );
}

// ------------------------------------------------------------------
// tmux fallback
// ------------------------------------------------------------------

/// True-negative coverage for the fenced-block fallback: under tmux
/// (where image protocols are typically disabled) the renderer must
/// emit a ` ```mermaid ` block AND the capture must contain zero
/// Kitty APC or iTerm2 OSC image bytes.
#[test]
#[serial(level2_terminal)]
fn level2_diagram_fallback_when_no_image_protocol() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    send_bt_command(harness, "flowchart \"A --> B\"");

    let frame = capture_until(harness, DIAGRAM_RENDER_TIMEOUT, |f| {
        f.plain.contains("```mermaid")
    });
    assert!(
        frame.plain.contains("```mermaid"),
        "expected fallback fenced code block in tmux (no image protocol). plain:\n{}",
        frame.plain
    );
    // Negative-control: image-protocol bytes must NOT appear in the
    // tmux capture. If they did, the fenced fallback was bypassed.
    assert!(
        !frame.raw.contains("\x1b_G"),
        "Kitty APC bytes leaked into tmux capture; fallback path failed. raw:\n{:?}",
        frame.raw.chars().take(400).collect::<String>(),
    );
    assert!(
        !frame.raw.contains("\x1b]1337;File="),
        "iTerm2 OSC bytes leaked into tmux capture; fallback path failed. raw:\n{:?}",
        frame.raw.chars().take(400).collect::<String>(),
    );
}

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

/// Extracts the first Kitty APC graphics payload from `raw`.
///
/// Kitty graphics commands are framed by `\x1b_G` (APC introducer) and
/// terminated by `\x1b\\` (ST). This helper returns the bytes between
/// those markers — including the control parameters and base64 image
/// data. Returns `None` when no complete APC frame is present.
fn extract_kitty_apc_payload(raw: &str) -> Option<String> {
    let start = raw.find("\x1b_G")? + 3;
    let rest = &raw[start..];
    let end = rest.find("\x1b\\")?;
    Some(rest[..end].to_string())
}

/// Returns the lowercase hexadecimal sha256 digest of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest.iter() {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn extract_kitty_apc_payload_pulls_bytes_between_markers() {
        let raw = "noise\x1b_Ga=T,f=100;PAYLOAD\x1b\\trailing";
        let payload = extract_kitty_apc_payload(raw).unwrap();
        assert_eq!(payload, "a=T,f=100;PAYLOAD");
    }

    #[test]
    fn extract_kitty_apc_payload_returns_none_without_marker() {
        assert!(extract_kitty_apc_payload("just plain text").is_none());
    }

    #[test]
    fn extract_kitty_apc_payload_returns_none_without_terminator() {
        assert!(extract_kitty_apc_payload("\x1b_Gunterminated").is_none());
    }

    #[test]
    fn sha256_hex_matches_known_digest() {
        // sha256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_hex_distinguishes_inputs() {
        let a = sha256_hex(b"alpha");
        let b = sha256_hex(b"beta");
        assert_ne!(a, b);
    }

    #[test]
    fn cases_table_covers_all_ten_subcommands() {
        let names: Vec<&str> = CASES.iter().map(|c| c.cmd).collect();
        for expected in [
            "flowchart",
            "quadrant",
            "pie-chart",
            "git-graph",
            "bar-chart",
            "line-chart",
            "timeline",
            "state-diagram",
            "erd",
            "graph-expression",
        ] {
            assert!(names.contains(&expected), "missing case: {expected}");
        }
        assert_eq!(CASES.len(), 10);
    }
}
