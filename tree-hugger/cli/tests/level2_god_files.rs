//! Level 2 tests for the `hug god-files` pretty report.
//!
//! The pretty report is rendered through biscuit-terminal `Prose`: the band
//! counts and per-file SLOC carry color (high-risk `<red>`, moderate-risk
//! `<yellow>`) and weight (`<b>`), section headings are bold + underlined
//! (`<b><uu>…</uu></b>`), the signal and refactor-hint lines are `<dim>`, file
//! names become OSC8 hyperlinks, and the body uses Unicode report glyphs
//! (middle-dot signal separator `·`, en-dash block ranges `–`, and the ellipsis
//! `…` truncation marker). Level 1 assert_cmd tests pin the markup and raw
//! forced bytes, but only a real terminal exercises the display path that turns
//! that markup into SGR sequences and a live OSC8 link.
//!
//! Each backend test renders two separate reports on its shared pane — a
//! high-risk fixture and a moderate-risk fixture — so both risk bands' styling
//! is exercised. They are kept as two short reports rather than one combined
//! report because WezTerm's `get-text` capture grabs only the visible viewport:
//! a single report tall enough to carry both bands plus a truncated block list
//! scrolls the top band's heading and color off-screen. Two short reports each
//! fit the viewport on every backend.
//!
//! Skip-clean: when no terminal harness is available each test prints a skip
//! notice and passes, so GitHub-hosted CI (which lacks the tooling) stays green.

use std::path::Path;

use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

/// Absolute path to the built `hug` binary. Driving the real binary keeps the
/// test honest about the production rendering path.
fn hug() -> &'static str {
    static BIN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    BIN.get_or_init(|| {
        biscuit_test_harness::bin_exe!("hug")
            .to_string_lossy()
            .into_owned()
    })
}

static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();

/// Write a high-risk fixture tree under `dir`: a single Python file with
/// 14 top-level functions, each large enough to rank as a block. This produces
/// over 1000 effective SLOC (high risk → `<red>`), 14 ranked blocks (truncated
/// to 8, so the `…and 6 more` ellipsis marker renders), and a 14-symbol top
/// level (so a refactor hint renders) — exercising every high-band styled
/// element in one report.
fn write_god_fixture(dir: &Path) {
    let mut src = String::new();
    for f in 0..14 {
        src.push_str(&format!("def function_number_{f}():\n"));
        for i in 0..75 {
            src.push_str(&format!("    value_{i} = {i} + {f}\n"));
        }
        src.push('\n');
    }
    std::fs::write(dir.join("god_big.py"), src).expect("write high fixture");
}

/// Write a moderate-risk fixture tree under `dir`: a single Python file with
/// 7 top-level functions of 66 effective lines each ≈ 462 effective SLOC,
/// landing in the moderate band (400–999) so the `Moderate risk` section, its
/// bold + underlined heading, and its `<yellow>` file SLOC render. Kept short so
/// the whole report fits a default 24-row viewport on every backend.
fn write_moderate_fixture(dir: &Path) {
    let mut src = String::new();
    for f in 0..7 {
        src.push_str(&format!("def moderate_function_{f}():\n"));
        for i in 0..65 {
            src.push_str(&format!("    value_{i} = {i} + {f}\n"));
        }
        src.push('\n');
    }
    std::fs::write(dir.join("god_moderate.py"), src).expect("write moderate fixture");
}

/// Drive `hug god-files <dir>` in the pane (forcing color + OSC8 via
/// `CLICOLOR_FORCE`) and capture the rendered frame.
fn run_god_files<H: TerminalHarness>(harness: &mut H, dir: &Path) -> CapturedFrame {
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    let cmd = format!("{} god-files {}", hug(), dir.display());
    harness
        .send_command_with_env(&cmd, &[("CLICOLOR_FORCE", "1")])
        .expect("send_command_with_env failed");

    let _ = biscuit_test_harness::wait_for_prompt(harness);
    std::thread::sleep(std::time::Duration::from_millis(200));

    harness.capture().expect("capture failed")
}

/// Tolerant SGR-fragment checks. Multiplexers may merge attributes into a
/// single combined sequence (`[1;33m`) or re-emit them separately (`[1m[33m`),
/// so we match the distinctive parameter fragment rather than a whole sequence.
mod sgr {
    /// Bold (SGR 1), as a standalone or merged parameter.
    pub fn bold(raw: &str) -> bool {
        raw.contains("[1m") || raw.contains(";1m")
    }

    /// Dim (SGR 2) — the signal and refactor-hint lines.
    pub fn dim(raw: &str) -> bool {
        raw.contains("[2m") || raw.contains(";2m")
    }

    /// Underline emitted by a `<uu>` (double-underline) heading. Backends
    /// disagree on how they re-emit it through capture:
    /// - tmux / kitty: the ITU colon form `4:2` (distinctive anywhere in a
    ///   merged sequence).
    /// - WezTerm `get-text`: normalizes it to the legacy ECMA-48 code `21`.
    /// - terminals lacking double-underline support: the single-underline
    ///   fallback (SGR 4).
    pub fn underline(raw: &str) -> bool {
        raw.contains("4:2")
            || raw.contains("[21m")
            || raw.contains(";21m")
            || raw.contains("[4m")
            || raw.contains(";4m")
    }
}

/// Assert the captured frame carries the styled, glyph-bearing high-risk report.
/// `expect_osc8` is gated to backends that preserve OSC8 through capture.
fn assert_styled_report(frame: &CapturedFrame, expect_osc8: bool) {
    // Visible text: the high-risk heading, the linked file name, the SLOC
    // call-out, a block symbol, and the three Unicode report glyphs (middle
    // dot, en dash, ellipsis) plus the truncation count.
    for needle in [
        "High risk",
        "god_big.py",
        "lines of code",
        "function_number_",
        "·",
        "–",
        "…",
        "and 6 more",
    ] {
        assert!(
            frame.plain.contains(needle),
            "expected '{needle}' in captured pane.\nplain:\n{}",
            frame.plain,
        );
    }

    // High-risk count and SLOC render red (SGR 31).
    assert!(
        frame.raw.contains("31m"),
        "expected a red SGR (high-risk band) in raw capture.\nraw:\n{}",
        frame.raw,
    );
    // The `<b>high risk</b>` heading phrase and bold SLOC render bold (SGR 1).
    assert!(
        sgr::bold(&frame.raw),
        "expected a bold SGR (heading/SLOC) in raw capture.\nraw:\n{}",
        frame.raw,
    );
    // The `High risk` section heading (`<b><uu>…</uu></b>`) renders underlined.
    assert!(
        sgr::underline(&frame.raw),
        "expected an underline SGR (section heading) in raw capture.\nraw:\n{}",
        frame.raw,
    );
    // Signal and refactor-hint lines render dim (SGR 2).
    assert!(
        sgr::dim(&frame.raw),
        "expected a dim SGR (signals/hints) in raw capture.\nraw:\n{}",
        frame.raw,
    );

    if expect_osc8 {
        assert!(
            frame.raw.contains("\x1b]8;;file://"),
            "expected an OSC8 file hyperlink in raw capture.\nraw:\n{}",
            frame.raw,
        );
    }
}

/// Assert the captured frame carries the styled moderate-risk report — the band
/// whose styling Level 1 alone cannot verify: the `Moderate risk` section
/// heading (bold + underlined) and the moderate file's `<yellow>` SLOC.
/// `expect_osc8` is gated to backends that preserve OSC8 through capture.
fn assert_moderate_report(frame: &CapturedFrame, expect_osc8: bool) {
    for needle in ["Moderate risk", "god_moderate.py", "lines of code"] {
        assert!(
            frame.plain.contains(needle),
            "expected '{needle}' in captured pane.\nplain:\n{}",
            frame.plain,
        );
    }

    // The moderate count and the moderate file SLOC render yellow (SGR 33).
    assert!(
        frame.raw.contains("33m"),
        "expected a yellow SGR (moderate-risk band) in raw capture.\nraw:\n{}",
        frame.raw,
    );
    // The moderate file SLOC (`<b><yellow>…`) renders bold (SGR 1).
    assert!(
        sgr::bold(&frame.raw),
        "expected a bold SGR (heading/SLOC) in raw capture.\nraw:\n{}",
        frame.raw,
    );
    // The `Moderate risk` section heading (`<b><uu>…</uu></b>`) renders underlined.
    assert!(
        sgr::underline(&frame.raw),
        "expected an underline SGR (section heading) in raw capture.\nraw:\n{}",
        frame.raw,
    );
    // Signal and refactor-hint lines render dim (SGR 2).
    assert!(
        sgr::dim(&frame.raw),
        "expected a dim SGR (signals/hints) in raw capture.\nraw:\n{}",
        frame.raw,
    );

    if expect_osc8 {
        assert!(
            frame.raw.contains("\x1b]8;;file://"),
            "expected an OSC8 file hyperlink in raw capture.\nraw:\n{}",
            frame.raw,
        );
    }
}

// ------------------------------------------------------------------
// tmux — portable, headless: SGR + glyphs (no OSC8 assertion)
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_god_files_pretty_report_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let dir = tempfile::TempDir::new().unwrap();
    write_god_fixture(dir.path());

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let frame = run_god_files(&mut harness, dir.path());
    assert_styled_report(&frame, false);

    let moderate_dir = tempfile::TempDir::new().unwrap();
    write_moderate_fixture(moderate_dir.path());
    let moderate_frame = run_god_files(&mut harness, moderate_dir.path());
    assert_moderate_report(&moderate_frame, false);
}

// ------------------------------------------------------------------
// WezTerm — SGR + glyphs + OSC8
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_god_files_pretty_report_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let dir = tempfile::TempDir::new().unwrap();
    write_god_fixture(dir.path());

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    let frame = run_god_files(harness, dir.path());
    assert_styled_report(&frame, true);

    let moderate_dir = tempfile::TempDir::new().unwrap();
    write_moderate_fixture(moderate_dir.path());
    let moderate_frame = run_god_files(harness, moderate_dir.path());
    assert_moderate_report(&moderate_frame, true);
}

// ------------------------------------------------------------------
// Kitty — SGR + glyphs + OSC8
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_god_files_pretty_report_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let dir = tempfile::TempDir::new().unwrap();
    write_god_fixture(dir.path());

    let mut guard =
        SHARED_KITTY.get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn Kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    let frame = run_god_files(harness, dir.path());
    assert_styled_report(&frame, true);

    let moderate_dir = tempfile::TempDir::new().unwrap();
    write_moderate_fixture(moderate_dir.path());
    let moderate_frame = run_god_files(harness, moderate_dir.path());
    assert_moderate_report(&moderate_frame, true);
}
