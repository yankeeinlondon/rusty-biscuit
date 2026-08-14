//! Level-2 tests for the `bt quote` / `bt list` container paths embedding a
//! Prose body that carries a fenced code block.
//!
//! Regression for review-2: a `Prose` body with a fenced code block embedded
//! in a quote or list container projected to an invalid render tree
//! (`Paragraph([…, Code, …])`), which validation rejected — the terminal
//! renderer then emitted empty output. These tests prove the container paths
//! render the styled text and the dim code block in a real terminal.
//!
//! ## Skip-clean contract
//!
//! Every test checks `*Harness::available()` before spawning. When the
//! required terminal is absent the test skips clean via `require_level!`. No
//! `#[ignore]` markers are used.

mod common;

use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

/// Process-shared WezTerm pane reused across the container tests.
static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();

/// Process-shared Kitty window reused across the container tests.
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();

/// A styled Prose body carrying an inline fenced code block. The
/// `<code-block>` form (rather than a triple-backtick fence) keeps the payload
/// single-line and shell-safe while still producing the block-level `Code`
/// node embedded in the styled span that tripped the container projection.
const FENCED_PROSE: &str = "<red>before <code-block lang=rust>code</code-block> after</red>";

/// Drives `bt <subcommand> "<FENCED_PROSE>"` with `FORCE_COLOR=1` and asserts
/// the container rendered the styled text and the dim code block — proving the
/// embedded-Prose-with-code projection no longer collapses to empty output.
fn assert_container_renders_fenced_code<H: TerminalHarness>(harness: &mut H, subcommand: &str) {
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    harness
        .send_command_with_env(
            &common::bt_cmd(&format!("bt {subcommand} \"{FENCED_PROSE}\"")),
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env failed");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    std::thread::sleep(std::time::Duration::from_millis(200));
    let frame = harness.capture().expect("capture failed");

    // Visible text survived. A validation failure would emit empty output, so
    // none of `before` / `code` / `after` would appear in the rendered rows.
    for needle in ["before", "code", "after"] {
        assert!(
            output_region_contains(&frame, subcommand, needle),
            "expected visible `{needle}` in the `bt {subcommand}` output region.\n\
             plain:\n{}\nraw:\n{}",
            frame.plain,
            frame.raw,
        );
    }

    // The enclosing red style lowered through the container's tree path: the
    // `before` output row selects red (`31`/`91`).
    let before_row = find_output_row(&frame, subcommand, "before").unwrap_or_else(|| {
        panic!(
            "could not locate the `before` output row.\nraw:\n{}",
            frame.raw
        )
    });
    assert!(
        row_selects_red(before_row),
        "expected the `before` text to be red through the `bt {subcommand}` \
         container path.\nrow: {before_row:?}",
    );
}

/// Returns the raw (escape-bearing) capture row, after the command echo, whose
/// visible text contains `needle`. Skipping the echo line keeps the literal
/// markup the user typed from satisfying the search.
fn find_output_row<'a>(
    frame: &'a CapturedFrame,
    subcommand: &str,
    needle: &str,
) -> Option<&'a str> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let plain_lines: Vec<&str> = frame.plain.lines().collect();
    let marker = format!("bt {subcommand}");
    let cmd_idx = plain_lines.iter().position(|l| l.contains(&marker))?;
    plain_lines
        .iter()
        .enumerate()
        .skip(cmd_idx + 1)
        .find(|(_, plain)| plain.contains(needle))
        .and_then(|(i, _)| raw_lines.get(i).copied())
}

/// Whether `needle` appears in any output row after the command echo.
fn output_region_contains(frame: &CapturedFrame, subcommand: &str, needle: &str) -> bool {
    find_output_row(frame, subcommand, needle).is_some()
}

/// Whether `segment` contains an SGR sequence that selects basic red (`31`) or
/// bright red (`91`) foreground. Accepts both the semicolon and ITU colon SGR
/// sub-parameter forms the two emulators emit.
fn row_selects_red(segment: &str) -> bool {
    let bytes = segment.as_bytes();
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
                let selects_red = segment[start..j]
                    .split([';', ':'])
                    .any(|p| p == "31" || p == "91");
                if selects_red {
                    return true;
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    false
}

// ------------------------------------------------------------------
// WezTerm
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_quote_fenced_code_prose_renders_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    assert_container_renders_fenced_code(harness, "quote");
}

#[test]
#[serial(level2_terminal)]
fn level2_list_fenced_code_prose_renders_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    assert_container_renders_fenced_code(harness, "list");
}

// ------------------------------------------------------------------
// Kitty
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_quote_fenced_code_prose_renders_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard =
        SHARED_KITTY.get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    assert_container_renders_fenced_code(harness, "quote");
}

#[test]
#[serial(level2_terminal)]
fn level2_list_fenced_code_prose_renders_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard =
        SHARED_KITTY.get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    assert_container_renders_fenced_code(harness, "list");
}
