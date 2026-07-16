//! Level-2 real-terminal evidence for the per-process OSC 10 foreground-color
//! cache (Findings 2, 3 & 21 of the 2026-07-12 performance review).
//!
//! These tests run the `discovery_probe` example inside a **real WezTerm pane**
//! driven through the shared `biscuit-test-harness`. WezTerm — not the test —
//! parses the `ESC ] 10 ; ? BEL` query and writes the answer. That is the whole
//! point: the sibling `level1_terminal_osc_cache.rs` manufactures its own reply
//! bytes, so it can only verify the cache and the request count. This file
//! verifies that a terminal emulator actually answers, and that the library
//! consumes that answer once.
//!
//! ## Why WezTerm and not tmux
//!
//! tmux is normally the preferred L2 backend (headless, portable). It is
//! **structurally unusable here**: `query_osc_color_with_timeout` gates the
//! actual query on `detect_multiplexer().is_none()`, so inside tmux the library
//! never emits OSC 10 at all and falls back to the compiled-in per-app default.
//! A tmux run would report a plausible-looking color that no terminal ever
//! sent — precisely the self-referential trap this file exists to avoid. The
//! backend must therefore be a non-multiplexer emulator on the library's
//! supported list (WezTerm, Kitty, iTerm2, Alacritty, Ghostty, Foot, Contour).
//!
//! ## What proves what
//!
//! * **A real round-trip happened** — the reported color must differ from the
//!   value the library would have invented had the query failed or been
//!   skipped. See [`is_wezterm_default_fallback`].
//! * **The response was reused, not re-fetched** — every construction reports
//!   the identical color *and* the library's own actual-round-trip counter
//!   stays at 1. Color equality alone is insufficient: a broken cache would
//!   re-query and still get the same answer from the same terminal.
//!
//! The counter comes from the `osc-query-counter` feature. A real emulator
//! consumes the query and answers on the wire, so — unlike a manufactured PTY —
//! there is no master side for the test to count bytes on. `just test-l2`
//! enables the feature; running this binary without it fails loudly rather than
//! quietly dropping the assertion.

#![cfg(unix)]

use std::time::Duration;

use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{TerminalHarness, strip_ansi};
use test_toolkit::{Level, require_level};

/// The foreground `query.rs::get_terminal_default_color` invents for WezTerm
/// when no real answer arrives (`RgbValue::new(229, 229, 229)`).
///
/// Observing this value means the OSC 10 round-trip did **not** happen — the
/// query was skipped, timed out, or the pane was not ready — so the run carries
/// no terminal-owned evidence and must fail rather than pass on a fallback that
/// merely looks like a color.
const WEZTERM_DEFAULT_FALLBACK: (u8, u8, u8) = (229, 229, 229);

fn is_wezterm_default_fallback(rgb: (u8, u8, u8)) -> bool {
    rgb == WEZTERM_DEFAULT_FALLBACK
}

/// Absolute path to the built `discovery_probe` example.
///
/// Derived from the test executable rather than `CARGO_BIN_EXE_` (cargo sets
/// that only for binaries, not examples).
fn discovery_probe_path() -> std::path::PathBuf {
    let exe = std::env::current_exe().expect("current_exe unavailable");
    let dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("unexpected test exe layout");
    dir.join("examples").join("discovery_probe")
}

/// Parse `RgbValue { r: 192, g: 202, b: 245 }` out of a probe line's tail.
fn parse_rgb(value: &str) -> Option<(u8, u8, u8)> {
    let inner = value.split_once("RgbValue {")?.1;
    let inner = inner.split_once('}')?.0;
    let mut rgb = [None; 3];
    for part in inner.split(',') {
        let (key, num) = part.split_once(':')?;
        let num: u8 = num.trim().parse().ok()?;
        match key.trim() {
            "r" => rgb[0] = Some(num),
            "g" => rgb[1] = Some(num),
            "b" => rgb[2] = Some(num),
            _ => return None,
        }
    }
    Some((rgb[0]?, rgb[1]?, rgb[2]?))
}

/// Read `key=value` for the first line whose text contains `key`.
///
/// Matches mid-line, not at line start: the pane also holds the echoed command
/// line and shell prompt.
fn field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let idx = line.find(key)?;
        Some(line[idx + key.len()..].trim().to_string())
    })
}

/// Run `discovery_probe` in the shared WezTerm pane and return the settled,
/// ANSI-stripped scrollback once `marker` appears.
///
/// stdout is deliberately **not** redirected: `is_tty()` keys on stdout, and a
/// redirect suppresses the OSC query entirely (that suppression is itself
/// correct behavior — it is what `darkmatter/cli/tests/compose_terminal_detection.rs`
/// covers for Finding 5). Output is therefore read back off the pane.
fn run_probe(harness: &mut WezTermHarness, env: &[(&str, &str)], marker: &str) -> String {
    let probe = discovery_probe_path();
    assert!(
        probe.exists(),
        "discovery_probe example not found at {}. Run \
         `cargo build -p biscuit-terminal --example discovery_probe --features osc-query-counter`.",
        probe.display()
    );

    // `clear` must be its own command: `send_command_with_env` prefixes
    // `KEY='v' ` onto the line it is given, so folding it into `clear; probe`
    // would scope the whole environment to `clear` and leave the probe with
    // `PROBE` unset — which silently selects the blocking `all` mode.
    harness.send_command_with_env("clear", &[]).expect("clear");
    harness
        .send_command_with_env(&probe.display().to_string(), env)
        .expect("send probe command");

    // Poll for the terminating marker in the same loop that produces the text
    // we assert on — capturing separately can grab a half-painted frame.
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        // Scrollback, not `capture()`: the latency probe's `raw_ns` line wraps
        // well past a pane's visible height.
        let text = strip_ansi(
            &harness
                .capture_scrollback(400)
                .expect("capture_scrollback")
                .raw,
        );
        if text.contains(marker) {
            return text;
        }
        if std::time::Instant::now() > deadline {
            panic!("probe marker {marker:?} not seen within 30s; pane text:\n{text}");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Fetch the library's actual-round-trip count for `code`, or fail loudly when
/// the probe was built without `osc-query-counter`.
fn actual_queries(text: &str, code: u8) -> usize {
    let key = format!("osc{code}_actual_queries=");
    let raw = field(text, &key).unwrap_or_else(|| {
        panic!(
            "probe printed no {key:?} line — the `osc-query-counter` feature was not enabled, so \
             the single-query assertion cannot be made. Run this tier via `just test-l2` (which \
             passes `--features osc-query-counter`). Pane text:\n{text}"
        )
    });
    raw.parse()
        .unwrap_or_else(|e| panic!("unparseable {key:?} value {raw:?}: {e}"))
}

/// Shared WezTerm pane, settled and ready to answer OSC queries.
///
/// The pane must be settled before the probe runs: WezTerm does not answer OSC
/// 10 until it is up, and the library's timeout is only 100ms, so a probe fired
/// at a cold pane silently records the default fallback instead. `spawn_shell`
/// already waits for a prompt; `settle` covers the shared-pane attach path.
fn shared_pane() -> Option<WezTermHarness> {
    let harness = WezTermHarness::shared_or_spawn().ok()?;
    harness.settle();
    Some(harness)
}

/// Three `Terminal` constructions inside a live WezTerm pane report WezTerm's
/// real foreground color, and the library performs exactly one OSC 10
/// round-trip to learn it.
#[test]
#[serial_test::serial(level2_terminal)]
fn level2_wezterm_answers_osc10_once_across_repeated_terminal_construction() {
    require_level!(Level::L2, WezTermHarness::available(), "WezTerm");
    let Some(mut harness) = shared_pane() else {
        eprintln!("skipping: requires WezTerm (shared pane unavailable)");
        return;
    };

    let text = run_probe(
        &mut harness,
        &[
            ("PROBE", "terminal_cache"),
            ("PROBE_TERM_CONSTRUCTIONS", "3"),
            ("NO_COLOR", "1"),
        ],
        "terminal_cache_done",
    );

    // (1) every construction reported a color.
    let colors: Vec<(u8, u8, u8)> = text
        .lines()
        .filter(|l| l.contains("terminal_text_color["))
        .filter_map(parse_rgb)
        .collect();
    assert_eq!(
        colors.len(),
        3,
        "expected 3 terminal_text_color lines; pane text:\n{text}"
    );

    // (2) the color came off WezTerm's wire, not the library's fallback table.
    //     Without this the test would pass identically on a terminal that never
    //     answered — the exact failure mode the L1 sibling cannot detect.
    assert!(
        !is_wezterm_default_fallback(colors[0]),
        "reported foreground {:?} is the compiled-in WezTerm default fallback \
         {WEZTERM_DEFAULT_FALLBACK:?} — no real OSC 10 answer was observed, so this run proves \
         nothing about the terminal. Pane text:\n{text}",
        colors[0]
    );

    // (3) the cached response is reused across constructions.
    for (i, c) in colors.iter().enumerate() {
        assert_eq!(
            *c, colors[0],
            "construction {i} reported {c:?} but construction 0 reported {:?} — the cached OSC 10 \
             response was not reused. Pane text:\n{text}",
            colors[0]
        );
    }

    // (4) reuse means *one* round-trip, not three that happened to agree.
    let osc10 = actual_queries(&text, 10);
    assert_eq!(
        osc10, 1,
        "expected exactly 1 actual OSC 10 round-trip across 3 Terminal constructions, saw \
         {osc10}. Pane text:\n{text}"
    );

    eprintln!(
        "level2 wezterm evidence: foreground={:?} across {} constructions, osc10_actual_queries={osc10}",
        colors[0],
        colors.len()
    );
}

/// Record repeated-construction latency against a live WezTerm pane, with
/// warm-up, sample count, and dispersion.
///
/// The warm-up absorbs the one-time cold OSC round-trip so the samples measure
/// the cached path. Statistics are echoed to stderr for evidence harvesting.
#[test]
#[serial_test::serial(level2_terminal)]
fn level2_wezterm_repeated_terminal_construction_latency() {
    require_level!(Level::L2, WezTermHarness::available(), "WezTerm");
    let Some(mut harness) = shared_pane() else {
        eprintln!("skipping: requires WezTerm (shared pane unavailable)");
        return;
    };

    let text = run_probe(
        &mut harness,
        &[
            ("PROBE", "terminal_latency"),
            ("PROBE_TERM_WARMUP", "3"),
            ("PROBE_TERM_SAMPLES", "50"),
            ("NO_COLOR", "1"),
        ],
        "terminal_latency_done",
    );

    let num = |key: &str| -> f64 {
        field(&text, key)
            .unwrap_or_else(|| panic!("probe printed no {key:?} line; pane text:\n{text}"))
            .parse()
            .unwrap_or_else(|e| panic!("unparseable {key:?}: {e}"))
    };

    let warmup = num("terminal_latency_warmup=");
    let samples = num("terminal_latency_samples=");
    let median = num("terminal_latency_median_ns=");
    let mean = num("terminal_latency_mean_ns=");
    let stddev = num("terminal_latency_stddev_ns=");
    let min = num("terminal_latency_min_ns=");
    let max = num("terminal_latency_max_ns=");

    assert_eq!(warmup, 3.0, "expected 3 warm-up constructions");
    assert_eq!(samples, 50.0, "expected 50 timed samples");

    // The warm-up paid the single round-trip; the timed samples must not add
    // any. This is the latency-side statement of the same cache invariant.
    let osc10 = actual_queries(&text, 10);
    assert_eq!(
        osc10, 1,
        "expected exactly 1 actual OSC 10 round-trip across warm-up + 50 timed constructions, \
         saw {osc10}. Pane text:\n{text}"
    );

    // Loose bound: a dropped cache would re-pay a real tty round-trip per
    // construction. Kept generous because a shared pane is not a quiet host.
    assert!(
        median < 50_000_000.0,
        "repeated-construction median {median} ns is implausibly high — cache may be broken"
    );

    eprintln!(
        "level2 wezterm latency: warmup={warmup} samples={samples} min={min}ns median={median}ns \
         mean={mean:.1}ns max={max}ns stddev={stddev:.1}ns osc10_actual_queries={osc10}"
    );
}
