//! Level-2 tests for image rendering.
//!
//! These tests run the `bt image` command inside a real terminal emulator
//! (WezTerm, Kitty) to validate that image protocol bytes are emitted
//! correctly and that scroll-compensation and rounding branches produce
//! user-visible effects in the captured pane.
//!
//! ## Skip-clean contract
//!
//! Every test checks `harness.available()` before spawning. When the
//! required terminal is absent the test prints `skipping: requires <X>`
//! to stderr and returns immediately. No `#[ignore]` markers are used.
//!
//! ## Assertion strategy
//!
//! Where the host terminal CLI exposes raw escape bytes
//! (`kitty @ get-text --ansi`, `wezterm cli get-text --escapes`), tests
//! assert directly on the image-protocol bytes — Kitty APC `\x1b_G ... \x1b\\`
//! or iTerm2 OSC `\x1b]1337;File= ... \x07`.
//!
//! Some terminals strip image protocol bytes from their text capture
//! (they replace the cells with placeholder spacers). For those cases
//! tests fall back to **pane-row geometry**: capture before and after
//! `bt image`, count the row delta, and compare against the renderer's
//! own debug-reported row count.

mod common;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use common::pane_geometry::{
    find_row_of, parse_debug_cursor_before, parse_debug_cursor_rows, parse_debug_image_height,
};
use common::send_bt_command;
use serial_test::serial;
use std::time::Duration;
use test_toolkit::{Backend, Level, require_level};

/// Process-shared WezTerm pane reused across the WezTerm image tests.
/// A `clear` is sent before each test's first interaction so prior
/// renders cannot leak into the capture window.
static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();

/// Process-shared Kitty window reused across the Kitty image tests.
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();

/// Returns the absolute path to a fixture file.
fn fixture_path(name: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/tests/fixtures/{name}")
}

/// Positions the cursor at a known row using `tput` so that image
/// rendering starts from a predictable position.
fn position_cursor(harness: &mut impl TerminalHarness, row: u32) {
    let cmd = format!("tput cup {row} 0\n");
    harness.send_text(cmd.as_bytes()).expect("tput failed");
    harness.settle();
}

/// Counts the lines that contain at least one printable, non-prompt
/// character — used as a coarse "rows occupied" measure when iTerm2 OSC
/// bytes have been stripped by the host terminal.
fn occupied_row_count(plain: &str) -> usize {
    plain
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter(|l| !looks_like_only_prompt(l))
        .count()
}

fn looks_like_only_prompt(line: &str) -> bool {
    let trimmed = line.trim_end();
    trimmed.ends_with('$') || trimmed.ends_with('%') || trimmed.ends_with('#')
}

/// Returns `true` if `raw` contains either iTerm2 graphics (OSC 1337)
/// or Kitty graphics (APC G) bytes.
fn has_image_protocol_bytes(raw: &str) -> bool {
    raw.contains("\x1b]1337;File=") || raw.contains("\x1b_G")
}

// ------------------------------------------------------------------
// Basic image rendering — WezTerm
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_image_renders_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");

    // Clear screen and position cursor high so image doesn't scroll.
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    position_cursor(harness, 5);

    let pre = harness.capture().expect("pre-capture failed");
    let pre_rows = occupied_row_count(&pre.plain);

    let path = fixture_path("tiny.png");
    send_bt_command(harness, &format!("image --debug {path}"));

    let frame = harness.capture().expect("capture failed");

    // Strategy A: assert iTerm2 OSC bytes are present in the raw
    // capture. WezTerm relays the protocol intact in some host
    // configurations.
    let osc_present = frame.raw.contains("\x1b]1337;File=");

    // Strategy B (fallback): pane-row math. After `bt image` the pane
    // must show at least one new occupied row beyond what was there
    // before, AND the renderer's debug-reported row count must be
    // non-zero.
    let post_rows = occupied_row_count(&frame.plain);
    let observed_delta = post_rows.saturating_sub(pre_rows);
    let debug_rows = parse_debug_cursor_rows(&frame.plain).unwrap_or(0);
    let geometry_ok = observed_delta > 0 && debug_rows > 0;

    assert!(
        osc_present || geometry_ok,
        "WezTerm image render: neither iTerm2 OSC bytes nor pane-row \
         growth observed.\n  osc_present={osc_present}\n  pre_rows={pre_rows} \
         post_rows={post_rows} delta={observed_delta} debug_rows={debug_rows}\n\
         raw_first_400={:?}\n  plain:\n{}",
        frame.raw.chars().take(400).collect::<String>(),
        frame.plain,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_image_renders_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    let path = fixture_path("tiny.png");
    send_bt_command(harness, &format!("image {path}"));

    let frame = harness.capture().expect("capture failed");
    assert!(
        frame.raw.contains("\x1b_G"),
        "expected raw output to contain Kitty graphics protocol APC sequence (\\x1b_G). raw:\n{}",
        frame.raw,
    );
}

// ------------------------------------------------------------------
// Scroll compensation: prove sentinel lands below image
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_image_scroll_compensation_at_bottom_margin() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");

    let pane_size = harness.pane_size().expect("pane_size failed");
    // Position the cursor near the bottom margin so the image render
    // forces scroll compensation. We aim for "image will extend past
    // the bottom row" by at least a couple of rows.
    let target_row = pane_size.rows.saturating_sub(3);

    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    position_cursor(harness, target_row);

    let path = fixture_path("tiny.png");
    send_bt_command(harness, &format!("image --debug {path}"));

    // Append a sentinel printf so we can locate the next user-visible
    // line *after* the image renders.
    harness
        .send_text(b"printf 'SENTINEL_AFTER_SCROLL\\n'\n")
        .expect("send_text failed");
    harness.settle();
    std::thread::sleep(Duration::from_millis(400));

    let frame = harness.capture().expect("capture failed");

    // The renderer's debug output must report that scroll compensation
    // was needed (it predicts the image would extend past the bottom
    // row given our cursor position).
    assert!(
        frame.plain.contains("SCROLL needed"),
        "expected renderer to detect scroll-margin condition. plain:\n{}",
        frame.plain,
    );

    // User-visible proof that scroll compensation worked: after the
    // image was rendered AND the pane scrolled, the sentinel `printf`
    // ran AND its output is visible in the final pane capture (not
    // clobbered by image cells, not lost off-screen). The sentinel row
    // must also sit strictly below the debug-output region — i.e. the
    // shell prompt re-drew below the image area.
    let sentinel_row = find_row_of(&frame.plain, "SENTINEL_AFTER_SCROLL").unwrap_or_else(|| {
        panic!(
            "SENTINEL_AFTER_SCROLL not found in pane after scroll-margin image render. plain:\n{}",
            frame.plain
        )
    });
    let debug_header_row = find_row_of(&frame.plain, "--- image debug ---").unwrap_or_else(|| {
        panic!(
            "expected `--- image debug ---` header in capture. plain:\n{}",
            frame.plain
        )
    });
    assert!(
        sentinel_row > debug_header_row,
        "sentinel row {sentinel_row} should be below debug header row \
         {debug_header_row} (proves the shell prompt and printf landed AFTER \
         the image area). plain:\n{}",
        frame.plain,
    );

    let debug_rows = parse_debug_cursor_rows(&frame.plain).unwrap_or_else(|| {
        panic!(
            "could not parse `cursor rows:` from debug output. plain:\n{}",
            frame.plain
        )
    });
    assert!(
        debug_rows >= 1,
        "renderer should report at least 1 image row; got {debug_rows}",
    );
}

// ------------------------------------------------------------------
// Warp `floor` rounding vs default `ceil`
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_warp_uses_floor_rounding() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");

    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    position_cursor(harness, 5);

    // Spoof Warp detection so bt uses floor rounding. Scoping the env
    // var to a single command via send_command_with_env avoids POSIX
    // `export` syntax leaking into the test (portable across shells).
    //
    // We force an odd character width (41) so that the raw height in
    // cells is almost always non-integer, ensuring ceil != floor and
    // the test actually distinguishes the rounding branches.
    let path = fixture_path("13x13.png");
    harness
        .send_command_with_env(
            &format!("bt image --debug --width 41 {path}"),
            &[("TERM_PROGRAM", "WarpTerminal")],
        )
        .expect("send_command_with_env failed");

    let frame = harness.capture().expect("capture failed");

    let (_raw, ceil, floor) = parse_debug_image_height(&frame.plain).unwrap_or_else(|| {
        panic!(
            "could not parse `image height:` from debug output. plain:\n{}",
            frame.plain
        )
    });
    let cursor_rows = parse_debug_cursor_rows(&frame.plain).unwrap_or_else(|| {
        panic!(
            "could not parse `cursor rows:` from debug output. plain:\n{}",
            frame.plain
        )
    });

    assert_eq!(
        cursor_rows,
        floor.max(1),
        "Warp branch must round 13px@41ch image height ({} ceil / {} floor) using floor()\n\
         plain:\n{}",
        ceil,
        floor,
        frame.plain,
    );
    // Sanity: ensure the chosen width actually exercises a non-integer row
    // count on this host. If ceil == floor, the test would not actually
    // be distinguishing the two branches.
    assert!(
        ceil != floor || floor == 1,
        "13x13@41ch fixture produced ceil ({ceil}) == floor ({floor}); \
         test loses distinguishing power on this host's font metrics. \
         Adjust width or fixture size."
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_image_default_uses_ceil_rounding() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");

    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    position_cursor(harness, 5);

    // Default WezTerm path → ceil rounding. We invoke via
    // send_command_with_env with an empty env list so no Warp-spoofing
    // env can leak from a previous test in the same shell session
    // (each command runs in a freshly forked subshell that does not
    // inherit a polluted parent env).
    let path = fixture_path("13x13.png");
    harness
        .send_command_with_env(&format!("bt image --debug {path}"), &[])
        .expect("send_command_with_env failed");

    let frame = harness.capture().expect("capture failed");
    let (_raw, ceil, floor) = parse_debug_image_height(&frame.plain).unwrap_or_else(|| {
        panic!(
            "could not parse `image height:` from debug output. plain:\n{}",
            frame.plain
        )
    });
    let cursor_rows = parse_debug_cursor_rows(&frame.plain).unwrap_or_else(|| {
        panic!(
            "could not parse `cursor rows:` from debug output. plain:\n{}",
            frame.plain
        )
    });

    assert_eq!(
        cursor_rows, ceil,
        "default branch must round 13px image height ({} ceil / {} floor) using ceil()\n\
         plain:\n{}",
        ceil, floor, frame.plain,
    );
}

// ------------------------------------------------------------------
// Meta JSON to stderr — must coexist with rendered image bytes
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_image_meta_to_stderr() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    let path = fixture_path("tiny.png");
    send_bt_command(harness, &format!("image --meta {path}"));

    let frame = harness.capture().expect("capture failed");
    let joined: String = frame.plain.lines().collect();
    assert!(
        joined.contains("\"cache_hit\"") && joined.contains("\"render_time_ms\""),
        "expected stderr metadata to contain JSON keys. plain:\n{}",
        frame.plain,
    );
    // Tighten: ensure --meta did not suppress image rendering. Either
    // the image protocol bytes survive in `raw`, OR the pane has
    // additional rows after the meta JSON line — never both absent.
    let bt_row = find_row_of(&frame.plain, "image --meta").unwrap_or(0);
    let post_rows = occupied_row_count(&frame.plain);
    let bytes_present = has_image_protocol_bytes(&frame.raw);
    assert!(
        bytes_present || post_rows > bt_row + 2,
        "--meta appears to have suppressed image rendering: \
         no protocol bytes in raw and pane shows no rows past the bt invocation.\n\
         bytes_present={bytes_present} bt_row={bt_row} post_rows={post_rows}\n\
         plain:\n{}",
        frame.plain,
    );
}

// ------------------------------------------------------------------
// Kitty cursor advance
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_image_kitty_row_advance() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    let path = fixture_path("tiny.png");
    send_bt_command(harness, &format!("image --debug {path}"));

    // Append a sentinel that should land directly below the rendered
    // image (one row below, accounting for the renderer's row advance).
    harness
        .send_text(b"printf 'SENTINEL_KITTY_BELOW\\n'\n")
        .expect("send_text failed");
    harness.settle();
    std::thread::sleep(Duration::from_millis(300));

    let frame = harness.capture().expect("capture failed");

    // Strong evidence the renderer used the Kitty graphics protocol.
    assert!(
        frame.raw.contains("\x1b_G"),
        "expected Kitty APC graphics bytes. raw first 400:\n{:?}",
        frame.raw.chars().take(400).collect::<String>(),
    );

    let cursor_rows = parse_debug_cursor_rows(&frame.plain).unwrap_or_else(|| {
        panic!(
            "could not parse `cursor rows:` from debug output. plain:\n{}",
            frame.plain
        )
    });
    let cursor_before = parse_debug_cursor_before(&frame.plain);

    // Sentinel must be present and below the bt invocation row.
    let bt_row = find_row_of(&frame.plain, "image --debug").unwrap_or_else(|| {
        panic!(
            "could not locate `bt image` invocation row. plain:\n{}",
            frame.plain
        )
    });
    let sentinel_row = find_row_of(&frame.plain, "SENTINEL_KITTY_BELOW").unwrap_or_else(|| {
        panic!(
            "SENTINEL_KITTY_BELOW not found in capture. plain:\n{}",
            frame.plain
        )
    });

    assert!(
        sentinel_row > bt_row,
        "sentinel row {sentinel_row} must be below bt invocation row {bt_row}",
    );

    // The renderer must have advanced at least 1 row for a non-empty
    // image; otherwise scroll compensation/CUD math is broken.
    assert!(
        cursor_rows >= 1,
        "renderer reported zero cursor rows for non-empty image; \
         cursor_before={cursor_before:?}",
    );
}
