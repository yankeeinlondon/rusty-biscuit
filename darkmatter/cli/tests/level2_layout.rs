//! Level-2 (real-terminal) tests for darkmatter page layout rendering.
//!
//! These tests run inside a WezTerm pane via the shared `biscuit-test-harness`
//! so we observe the actual rendered terminal output — SGR background fills,
//! visible column widths, line-number gutters, and component alignment/fill.
//!
//! Each test:
//! 1. Writes a Markdown fixture to a temp file.
//! 2. Spawns a WezTerm pane, runs `md <file> --max-width N <layout flags>`.
//! 3. Captures the rendered frame and asserts on visible structure.
//!
//! Tests skip silently when `WEZTERM_UNIX_SOCKET` is not set or the
//! `wezterm` binary is missing, matching the pattern in `level2_errors.rs`.
//!
//! ## CI enforcement
//!
//! Set `DARKMATTER_LEVEL2_REQUIRED=1` in the environment to convert a missing
//! WezTerm into a hard failure rather than a silent skip. CI jobs that
//! provision WezTerm should always set this so Level 2 coverage is
//! actually enforced, not just nominally present.

use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness, skip_with_reason};
use serial_test::serial;
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, Once};
use std::time::{Duration, Instant};
use tempfile::tempdir;

/// Returns `true` when the environment requires Level 2 tests to run (CI mode).
fn level2_required() -> bool {
    matches!(
        std::env::var("DARKMATTER_LEVEL2_REQUIRED").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

/// Process-wide shared WezTerm pane reused across every test in this file.
///
/// Spawning a fresh WezTerm window is the dominant cost in Level 2 layout
/// tests (≈2–3 s per spawn plus the prompt-readiness wait). All tests in
/// this file are `#[serial(level2_terminal)]`, so they execute one at a
/// time and can safely share a single pane.
///
/// Between invocations the pane is reset with `clear` so each test sees a
/// clean visible region. At process exit, [`ensure_atexit_cleanup`]
/// registers a `libc::atexit` handler that takes the harness out of the
/// mutex and drops it — `WezTermHarness::drop` calls `wezterm cli kill-pane`
/// so the pane is reclaimed instead of leaking into the
/// `biscuit-bg` workspace. Rust never runs `Drop` on `static` values, so
/// the atexit hook is what prevents pty exhaustion across many test runs.
static SHARED_HARNESS: Mutex<Option<WezTermHarness>> = Mutex::new(None);

/// Ensures the shared WezTerm pane is killed at process exit, even though
/// Rust does not run `Drop` on `static` values.
///
/// Called from [`run_md_env`] the first time the shared harness is spawned.
/// The registered handler is idempotent — running it again after the harness
/// has been taken is a no-op.
fn ensure_atexit_cleanup() {
    static REGISTER_ATEXIT: Once = Once::new();
    REGISTER_ATEXIT.call_once(|| {
        extern "C" fn cleanup_shared_harness() {
            // atexit handlers must not unwind. The lock may be poisoned by
            // a panicking test; recover and proceed so the pane is still
            // killed. `WezTermHarness::drop` shells out to `wezterm cli
            // kill-pane`, which is itself best-effort.
            let _ = std::panic::catch_unwind(|| {
                let mut guard = SHARED_HARNESS
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner());
                // `take` moves the harness out so its `Drop` runs here at
                // process exit instead of being skipped for the static.
                drop(guard.take());
            });
        }
        // SAFETY: `cleanup_shared_harness` is `extern "C" fn()` with no
        // captures, matching `libc::atexit`'s required signature. Registration
        // itself is thread-safe; macOS allows up to 32 atexit handlers.
        unsafe {
            libc::atexit(cleanup_shared_harness);
        }
    });
}

/// Monotonic counter for sentinel uniqueness across tests in this binary.
static SENTINEL_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Maximum wall time we'll spend waiting for a single command's completion
/// sentinel to appear in the pane. Generous — most `md` invocations finish
/// in well under a second; this is a safety net for first-run cold builds.
const SENTINEL_TIMEOUT: Duration = Duration::from_secs(30);

/// Polls the pane every 50 ms looking for `sentinel`, returning the final
/// captured frame once it appears. Returns the last attempted capture on
/// timeout so callers can include it in panic diagnostics.
fn wait_for_sentinel(
    harness: &mut WezTermHarness,
    sentinel: &str,
) -> Result<CapturedFrame, CapturedFrame> {
    let deadline = Instant::now() + SENTINEL_TIMEOUT;
    let mut last = CapturedFrame::from_raw(String::new());
    while Instant::now() < deadline {
        if let Ok(frame) = harness.capture() {
            // The sentinel also appears inline in the command echo
            // (e.g. `$ md ...; printf '\n__DM_DONE_0__\n'`). Only treat the
            // sentinel as completion when it appears on a line of its own,
            // which only happens after `printf` actually runs.
            if frame.plain.lines().any(|l| l.trim() == sentinel) {
                return Ok(frame);
            }
            last = frame;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(last)
}

/// Runs `cmd` in the shared pane, appending a unique completion sentinel so
/// we can detect when `cmd` has finished without depending on the user's
/// shell prompt format.
fn run_with_sentinel(harness: &mut WezTermHarness, cmd: &str) -> CapturedFrame {
    let id = SENTINEL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let sentinel = format!("__DM_LVL2_DONE_{id}__");
    // `printf` is portable across bash/zsh and emits the sentinel on its own
    // line so it never visually fuses with `md`'s last row.
    let wrapped = format!("{cmd}; printf '\\n{sentinel}\\n'");
    harness
        .send_command_with_env(&wrapped, &[])
        .expect("send_command_with_env failed");
    match wait_for_sentinel(harness, &sentinel) {
        Ok(frame) => frame,
        Err(last) => panic!(
            "timed out waiting for sentinel {sentinel} after {SENTINEL_TIMEOUT:?}. \
             last plain capture:\n{}",
            last.plain
        ),
    }
}

/// Helper: write a markdown fixture, run `md` with the given flags inside the
/// shared harness pane, and return the captured frame.
fn run_md(file_body: &str, extra_args: &str) -> Option<(CapturedFrame, std::path::PathBuf)> {
    run_md_env(file_body, extra_args, &[])
}

/// Like [`run_md`] but injects inline environment assignments onto the `md`
/// invocation (e.g. `COLORFGBG` to force light/dark color-mode detection
/// deterministically).
fn run_md_env(
    file_body: &str,
    extra_args: &str,
    env: &[(&str, &str)],
) -> Option<(CapturedFrame, std::path::PathBuf)> {
    if !WezTermHarness::available() {
        if level2_required() {
            panic!(
                "DARKMATTER_LEVEL2_REQUIRED=1 set but WezTerm is unavailable. \
                 Provision WezTerm in this environment or unset the variable."
            );
        }
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return None;
    }

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("layout.md");
    fs::write(&file_path, file_body).unwrap();

    // Recover from a previous test's panic — poisoned mutexes are fine here
    // since the harness state is just a pane id we re-validate via `clear`.
    let mut guard = SHARED_HARNESS
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    if guard.is_none() {
        let mut harness = WezTermHarness::new();
        harness.spawn_shell().expect("spawn_shell failed");
        *guard = Some(harness);
        // Register the process-exit cleanup the first time we actually own a
        // pane. Registering before spawn would leave a no-op handler in place
        // for runs where every test skips (WezTerm absent), which is harmless
        // but wasteful — registering here keeps the hook paired with the
        // resource it cleans up.
        ensure_atexit_cleanup();
    }
    let harness = guard.as_mut().unwrap();

    // Reset the visible region so the previous test's output does not bleed
    // into this capture. `clear` is portable across bash and zsh.
    run_with_sentinel(harness, "clear");

    let cmd = format!("md {} {}", file_path.display(), extra_args);
    let frame = run_with_sentinel_env(harness, &cmd, env);
    // Keep tempdir alive past capture by returning its path.
    Some((frame, file_path))
}

/// Like [`run_with_sentinel`] but applies inline env assignments to `cmd`.
fn run_with_sentinel_env(
    harness: &mut WezTermHarness,
    cmd: &str,
    env: &[(&str, &str)],
) -> CapturedFrame {
    let id = SENTINEL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let sentinel = format!("__DM_LVL2_DONE_{id}__");
    let wrapped = format!("{cmd}; printf '\\n{sentinel}\\n'");
    harness
        .send_command_with_env(&wrapped, env)
        .expect("send_command_with_env failed");
    match wait_for_sentinel(harness, &sentinel) {
        Ok(frame) => {
            // The sentinel only guarantees `md` finished; the pane grid may
            // still be settling (scroll, redraw). Settle and re-capture so
            // assertions see the final stable frame, not a transitional one.
            std::thread::sleep(Duration::from_millis(250));
            harness.capture().unwrap_or(frame)
        }
        Err(last) => panic!(
            "timed out waiting for sentinel {sentinel} after {SENTINEL_TIMEOUT:?}. \
             last plain capture:\n{}",
            last.plain
        ),
    }
}

/// Strip trailing whitespace from a line for visible-width comparisons.
fn rtrim(s: &str) -> &str {
    s.trim_end_matches([' ', '\t'])
}

#[test]
#[serial(level2_terminal)]
fn level2_margin_renders_blank_top_rows() {
    // Use a recognizable heading marker so we can anchor the output without
    // depending on parsing the command echo (which may wrap across pane lines).
    let Some((frame, _)) = run_md("# UniqueMarginMarker\n", "-m 2 --max-width 40") else {
        return;
    };

    let lines: Vec<&str> = frame.plain.lines().collect();
    let marker_idx = lines
        .iter()
        .position(|l| l.contains("UniqueMarginMarker"))
        .unwrap_or_else(|| {
            panic!(
                "marker line not found in captured pane. plain:\n{}",
                frame.plain
            )
        });

    // Margin-top=2 means the heading must be preceded by at least 2 blank
    // rows above it (within the rendered output region).
    assert!(
        marker_idx >= 2,
        "marker must have at least 2 preceding rows, found at idx {marker_idx}"
    );
    let row_above = rtrim(lines[marker_idx - 1]);
    let row_two_above = rtrim(lines[marker_idx - 2]);
    assert!(
        row_above.is_empty(),
        "row directly above heading should be blank (margin), got: {row_above:?}"
    );
    assert!(
        row_two_above.is_empty(),
        "row two above heading should be blank (margin), got: {row_two_above:?}"
    );
}

/// Returns `true` when the captured raw stream contains any background SGR
/// attribute (24-bit `48;2;...`, 256-color `48;5;...`, or basic `40-47`/
/// `100-107` palette codes). WezTerm's `get-text --escapes` re-emits cell
/// attributes; the exact form depends on terminal/runtime settings.
fn raw_has_background_sgr(raw: &str) -> bool {
    raw.contains("\x1b[48;2;") || raw.contains("\x1b[48;5;") || raw.contains("\x1b[48:")
}

#[test]
#[serial(level2_terminal)]
fn level2_page_bg_subtle_emits_bg_sgr() {
    // Use a multi-row body so padded background rows are guaranteed to fall
    // inside the captured viewport.
    let body = "# Subtle\n\nLine one.\n\nLine two.\n";
    let Some((frame, _)) = run_md(body, "--page-bg subtle --padding 2 --max-width 40") else {
        return;
    };

    assert!(
        raw_has_background_sgr(&frame.raw),
        "expected a background SGR for --page-bg subtle. raw len={}, plain:\n{}",
        frame.raw.len(),
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_page_bg_pronounced_emits_bg_sgr() {
    let body = "# Pronounced\n\nLine one.\n\nLine two.\n";
    let Some((frame, _)) = run_md(body, "--page-bg pronounced --padding 2 --max-width 40") else {
        return;
    };

    assert!(
        raw_has_background_sgr(&frame.raw),
        "expected a background SGR for --page-bg pronounced. raw len={}",
        frame.raw.len()
    );
    // Pronounced should also reset attributes so following content (the next
    // shell prompt) is not stuck with the page bg.
    assert!(
        frame.raw.contains("\x1b[0m") || frame.raw.contains("\x1b[m"),
        "expected a reset SGR after the pronounced page surface"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_max_width_caps_visible_content_columns() {
    // Long paragraph; with --max-width 40 every wrapped line must fit in 40
    // columns. Anchor on a unique sentinel so we can locate the output region.
    let body = "Sentinel_paragraph Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore.\n";
    let Some((frame, _)) = run_md(body, "--max-width 40") else {
        return;
    };

    let lines: Vec<&str> = frame.plain.lines().collect();
    let sentinel_idx = lines
        .iter()
        .position(|l| l.contains("Sentinel_paragraph"))
        .unwrap_or_else(|| {
            panic!(
                "sentinel line not found in pane capture. plain:\n{}",
                frame.plain
            )
        });

    // Check the sentinel line and the next two lines (wrap continuation) all
    // fit within 40 visible columns.
    for (offset, line) in lines.iter().skip(sentinel_idx).take(3).copied().enumerate() {
        let visible = rtrim(line).chars().count();
        assert!(
            visible <= 40,
            "wrap line +{offset} exceeds --max-width 40, got {visible} cols: {line:?}"
        );
    }
}

#[test]
#[serial(level2_terminal)]
fn level2_line_numbers_true_renders_gutter() {
    let body = "```rust\nfn main() {}\nlet x = 1;\n```\n";
    let Some((frame, _)) = run_md(body, "--line-numbers=true --max-width 60") else {
        return;
    };

    // Line-number gutter uses the box-drawing "│" separator.
    assert!(
        frame.plain.contains('│'),
        "expected line-number gutter '│' in plain output. plain:\n{}",
        frame.plain
    );
    // And the gutter shows numeric line numbers.
    assert!(
        frame.plain.contains("1 │") || frame.plain.contains("1 │ "),
        "expected '1 │' gutter prefix in plain output. plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_line_numbers_false_omits_gutter() {
    let body = "```rust\nfn main() {}\nlet x = 1;\n```\n";
    let Some((frame, _)) = run_md(body, "--line-numbers=false --max-width 60") else {
        return;
    };

    // The "│" gutter separator should not appear without line numbers.
    // Pages of regular code don't otherwise emit U+2502.
    assert!(
        !frame.plain.contains('│'),
        "did not expect gutter '│' with --line-numbers=false. plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_code_block_fill_pad_adds_left_indent() {
    let body = "```rust\nfn main() {}\n```\n";
    let Some((frame, _)) = run_md(
        body,
        "--fill-code-blocks pad=4 --max-width 40 --page-bg subtle",
    ) else {
        return;
    };

    // With Pad(4) on a 40-col page, the code block renders at 32 cols and is
    // shifted right by 4 cols of left padding (Left alignment is the default
    // since the layout fix). Locate the header line (contains "rust") and
    // verify it starts with at least 4 leading spaces.
    let rust_line = frame
        .plain
        .lines()
        .find(|l| l.contains("rust"))
        .expect("expected a line containing the 'rust' code-block label");
    let leading = rust_line.chars().take_while(|c| *c == ' ').count();
    assert!(
        leading >= 4,
        "expected >=4 leading spaces for Pad(4) left padding, got {leading}: {rust_line:?}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_code_block_max_fill_constrains_body_width() {
    let body = "```rust\nfn really_long_function_name_that_would_overflow() {}\n```\n";
    let Some((frame, _)) = run_md(
        body,
        "--fill-code-blocks max=30 --max-width 60 --page-bg subtle",
    ) else {
        return;
    };

    // With Max(30), neither the header line (containing "rust") nor the body
    // line should have visible content exceeding 30 columns.
    //
    // We locate the header via its "rust" tag and assert its visible width
    // (after rtrim) is <= 30 plus any leading whitespace. Then we find the
    // function-name line and assert the same.
    let lines: Vec<&str> = frame.plain.lines().collect();

    let header = lines
        .iter()
        .find(|l| l.contains("rust"))
        .expect("expected a header line containing 'rust'");

    let leading = header.chars().take_while(|c| *c == ' ').count();
    let content_visible = rtrim(header).chars().count() - leading;
    assert!(
        content_visible <= 30,
        "header content visible width should be <=30, got {content_visible}: {header:?}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_align_code_block_center_indents_more_than_left() {
    let body = "```rust\nfn main() {}\n```\n";

    // Left-aligned baseline.
    let Some((left_frame, _)) = run_md(
        body,
        "--align-code-blocks left --fill-code-blocks max=20 --max-width 60",
    ) else {
        return;
    };
    let left_header = left_frame
        .plain
        .lines()
        .find(|l| l.contains("rust"))
        .expect("left: expected a header line containing 'rust'");
    let left_indent = left_header.chars().take_while(|c| *c == ' ').count();

    // Center-aligned: same fill, same page width, so the only difference is
    // the alignment surplus added as additional left padding.
    let Some((center_frame, _)) = run_md(
        body,
        "--align-code-blocks center --fill-code-blocks max=20 --max-width 60",
    ) else {
        return;
    };
    let center_header = center_frame
        .plain
        .lines()
        .find(|l| l.contains("rust"))
        .expect("center: expected a header line containing 'rust'");
    let center_indent = center_header.chars().take_while(|c| *c == ' ').count();

    assert!(
        center_indent > left_indent,
        "center alignment should indent more than left: left={left_indent}, center={center_indent}\nleft header: {left_header:?}\ncenter header: {center_header:?}"
    );
}

// =============================================================================
//                  TABLES / IMAGES / BLOCKQUOTES / LISTS
// =============================================================================
//
// Real-terminal captures for component-specific alignment, fill, and wrapping.
// These cover the gap called out in review-1: code-block coverage existed
// already, but tables, images, blockquotes, and lists were Level 1 only.

#[test]
#[serial(level2_terminal)]
fn level2_table_max_fill_constrains_visible_width() {
    // Use unique sentinel column values so we can anchor on a single body row
    // regardless of header/separator framing. The table needs enough render
    // width to keep all columns; we cap at 50 (well under page width 80) and
    // assert the rendered row never exceeds that cap.
    let body = "\
| ColA | ColB | ColC |\n\
| ---- | ---- | ---- |\n\
| sentinel_alpha | beta | gamma |\n";

    let Some((frame, _)) = run_md(body, "--fill-tables max=50 --max-width 80") else {
        return;
    };

    // Locate the body row by sentinel (it's only present in rendered output,
    // never in the shell command echo).
    let row = frame
        .plain
        .lines()
        .find(|l| l.contains("sentinel_alpha"))
        .unwrap_or_else(|| {
            panic!(
                "expected a table row containing 'sentinel_alpha' in:\n{}",
                frame.plain
            )
        });
    let visible = rtrim(row).chars().count();
    assert!(
        visible <= 50,
        "table row visible width should be capped to 50 cols, got {visible}: {row:?}"
    );
    // Also ensure the cap actually constrains (visible < page width).
    assert!(
        visible < 80,
        "table row should be narrower than the page (80 cols), got {visible}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_table_center_alignment_indents_more_than_left() {
    // Use a unique sentinel cell value so we anchor on the rendered body row
    // and never accidentally match the shell command echo (which contains the
    // temp file path and CLI numerics like `max=20`/`60`).
    let body = "\
| A | B |\n\
| - | - |\n\
| sentinelA | sentinelB |\n";

    // `max` must be wide enough for the table to actually render — each
    // wrapping column needs ~9 cols of content plus pipes/padding, so a
    // 2-column table needs roughly 25 cols minimum. We pick 40 so the cap
    // still leaves visible alignment surplus on the 60-col page.
    let Some((left, _)) = run_md(
        body,
        "--align-tables left --fill-tables max=40 --max-width 60",
    ) else {
        return;
    };
    let Some((center, _)) = run_md(
        body,
        "--align-tables center --fill-tables max=40 --max-width 60",
    ) else {
        return;
    };

    let row_left = left
        .plain
        .lines()
        .find(|l| l.contains("sentinelA"))
        .unwrap_or_else(|| panic!("left: data row missing. plain:\n---\n{}\n---", left.plain));
    let row_center = center
        .plain
        .lines()
        .find(|l| l.contains("sentinelA"))
        .unwrap_or_else(|| {
            panic!(
                "center: data row missing. plain:\n---\n{}\n---",
                center.plain
            )
        });
    let left_indent = row_left.chars().take_while(|c| *c == ' ').count();
    let center_indent = row_center.chars().take_while(|c| *c == ' ').count();
    assert!(
        center_indent > left_indent,
        "center alignment must indent more than left: left={left_indent}, center={center_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_blockquote_indent_fill_caps_wrap_width() {
    // Long quoted prose so the wrap point is observable in the captured pane.
    let body = "> This is a fairly long quoted paragraph that should wrap onto a second visible line once Indent(20) forces the blockquote render width below the page width.\n";

    let Some((frame, _)) = run_md(body, "--fill-block-quotes indent=20 --max-width 80") else {
        return;
    };

    // Strip ANSI-stripped, trim trailing background fill. Blockquote lines are
    // prefixed with the `▐` indicator glyph.
    let quote_lines: Vec<String> = frame
        .plain
        .lines()
        .filter(|l| l.contains('▐'))
        .map(|l| rtrim(l).to_string())
        .collect();
    assert!(
        quote_lines.len() >= 2,
        "blockquote should wrap onto multiple lines under Indent(20). plain:\n{}",
        frame.plain
    );
    let max_len = quote_lines.iter().map(|l| l.chars().count()).max().unwrap();
    assert!(
        max_len <= 60,
        "blockquote lines should be capped to 60 cols (80 - 20 indent), got max={max_len}. plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_blockquote_center_alignment_indents_more_than_left() {
    let body = "> Centered quoted line.\n";
    let Some((left, _)) = run_md(
        body,
        "--align-block-quotes left --fill-block-quotes max=20 --max-width 60",
    ) else {
        return;
    };
    let Some((center, _)) = run_md(
        body,
        "--align-block-quotes center --fill-block-quotes max=20 --max-width 60",
    ) else {
        return;
    };

    let left_quote = left
        .plain
        .lines()
        .find(|l| l.contains('▐'))
        .expect("left: blockquote line");
    let center_quote = center
        .plain
        .lines()
        .find(|l| l.contains('▐'))
        .expect("center: blockquote line");
    let left_indent = left_quote.chars().take_while(|c| *c == ' ').count();
    let center_indent = center_quote.chars().take_while(|c| *c == ' ').count();
    assert!(
        center_indent > left_indent,
        "center alignment should indent more than left: left={left_indent}, center={center_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_list_max_fill_caps_wrap_width() {
    // Long bullet so wrap is observable.
    let body = "- This is a notably long list item that has to wrap to a second visible row once Max(40) constrains the list render width to forty columns.\n- Follow-up.\n";

    let Some((frame, _)) = run_md(body, "--fill-lists max=40 --max-width 80") else {
        return;
    };

    // Locate the start of list output via a stable anchor, then collect the
    // list region until the next blank line. This avoids measuring shell
    // prompt + command-echo lines which span the full pane width.
    let lines: Vec<&str> = frame.plain.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_start().starts_with("- This is"))
        .unwrap_or_else(|| panic!("missing list anchor. plain:\n{}", frame.plain));
    let list_region: Vec<&str> = lines
        .iter()
        .skip(start)
        .take_while(|l| !l.trim().is_empty())
        .copied()
        .collect();
    let max_len = list_region
        .iter()
        .map(|l| rtrim(l).chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        max_len <= 40,
        "list lines should be capped to 40 cols, got max={max_len}. region:\n{}\nfull plain:\n{}",
        list_region.join("\n"),
        frame.plain
    );
    // Sanity-check: confirm wrap actually occurred (>= 3 list rows: long line
    // wraps to multiple rows + the Follow-up item).
    assert!(
        list_region.len() >= 3,
        "expected list to wrap onto multiple rows, got {} rows:\n{}",
        list_region.len(),
        list_region.join("\n")
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_list_center_alignment_indents_more_than_left() {
    // Use short items so we can compare the marker line's leading indent.
    // Per sub-spec #4, when `style.li.alignment` (or the broadcast `--align-lists`
    // that writes Li alignment) shifts the body, the body becomes its own block
    // on a fresh line. We anchor on the body sentinel `xy` so the test works
    // regardless of whether the marker and body share a line.
    let body = "- xy\n- ab\n";
    let Some((left, _)) = run_md(
        body,
        "--align-lists left --fill-lists max=30 --max-width 60",
    ) else {
        return;
    };
    let Some((center, _)) = run_md(
        body,
        "--align-lists center --fill-lists max=30 --max-width 60",
    ) else {
        return;
    };

    let find_body_indent = |plain: &str, label: &str| -> usize {
        let line = plain
            .lines()
            .find(|l| l.trim_start() == "xy" || l.trim_start().starts_with("- xy"))
            .unwrap_or_else(|| panic!("{label}: list item not found. plain:\n{plain}"));
        line.chars().take_while(|c| *c == ' ').count()
    };
    let left_indent = find_body_indent(&left.plain, "left");
    let center_indent = find_body_indent(&center.plain, "center");
    assert!(
        center_indent > left_indent,
        "list center alignment must indent more than left: left={left_indent}, center={center_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_image_fallback_text_respects_alignment() {
    // Use a non-existent image so the renderer emits its alt-text/path
    // fallback rather than attempting an inline image protocol — that gives
    // us a stable plain-text anchor in the captured pane regardless of the
    // host terminal's image support.
    let body = "![Alt text fallback](./does-not-exist.png)\n";

    let Some((left, _)) = run_md(
        body,
        "--align-images left --fill-images max=20 --max-width 60",
    ) else {
        return;
    };
    let Some((center, _)) = run_md(
        body,
        "--align-images center --fill-images max=20 --max-width 60",
    ) else {
        return;
    };

    // Anchor on the alt text; both captures must contain it.
    let line_left = left
        .plain
        .lines()
        .find(|l| l.contains("Alt text fallback"))
        .unwrap_or_else(|| {
            panic!(
                "left: expected an alt-text anchor in image fallback. plain:\n{}",
                left.plain
            )
        });
    let line_center = center
        .plain
        .lines()
        .find(|l| l.contains("Alt text fallback"))
        .unwrap_or_else(|| {
            panic!(
                "center: expected an alt-text anchor in image fallback. plain:\n{}",
                center.plain
            )
        });

    let left_indent = line_left.chars().take_while(|c| *c == ' ').count();
    let center_indent = line_center.chars().take_while(|c| *c == ' ').count();
    assert!(
        center_indent > left_indent,
        "image fallback center alignment must indent more than left: left={left_indent}, center={center_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_end_to_end_layout_dimensions() {
    // Cross-component end-to-end capture: a small page with multiple
    // components rendered under a complete set of layout flags. Verifies
    // visible row dimensions and the top/bottom margin behavior at once.
    let body = "\
# E2E heading\n\
\n\
Some prose paragraph.\n\
\n\
- list item alpha\n\
- list item beta\n\
\n\
> A quoted observation.\n\
";

    let Some((frame, _)) = run_md(
        body,
        "-m 2 --padding 1 --max-width 40 --page-bg subtle --align-block-quotes left",
    ) else {
        return;
    };

    // Locate the heading anchor.
    let lines: Vec<&str> = frame.plain.lines().collect();
    let head_idx = lines
        .iter()
        .position(|l| l.contains("E2E heading"))
        .unwrap_or_else(|| panic!("missing heading anchor in pane:\n{}", frame.plain));

    // Margin top = 2: two blank rows above the (left-padded) heading.
    assert!(
        head_idx >= 2,
        "heading should sit at row index >= 2 to allow 2 margin rows above, got {head_idx}"
    );
    assert!(
        rtrim(lines[head_idx - 1]).is_empty(),
        "row above heading must be blank (margin), got: {:?}",
        lines[head_idx - 1]
    );
    assert!(
        rtrim(lines[head_idx - 2]).is_empty(),
        "two rows above heading must be blank (margin), got: {:?}",
        lines[head_idx - 2]
    );

    // Visible content rows should not exceed margin_x(2*2) + padding_x(1*2)
    // + max_width(40) = 46 columns.
    let max_visible = lines
        .iter()
        .skip(head_idx)
        .take(10)
        .map(|l| rtrim(l).chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        max_visible <= 46,
        "content rows should fit in 4 margin + 2 padding + 40 max-width = 46 cols, got {max_visible}. plain:\n{}",
        frame.plain
    );

    // Every component anchor must be present in the captured pane.
    for anchor in [
        "Some prose paragraph",
        "list item alpha",
        "list item beta",
        "A quoted observation",
    ] {
        assert!(
            frame.plain.contains(anchor),
            "expected '{anchor}' in captured plain output. plain:\n{}",
            frame.plain
        );
    }
}

// ---------------------------------------------------------------------------
// Code-block rendering (fixes from 2026-05-22-darkmatter-failures)
//
// These exercise the real `md` CLI in a real WezTerm pane: the renderer must
// detect the terminal's color mode, invert the *code* theme for contrast, fill
// the code background across the content rectangle (never to the physical
// edge), and not leave a constant trailing-blank offset.
// ---------------------------------------------------------------------------

/// Rec. 601 luma of an sRGB triple.
fn luma(r: u32, g: u32, b: u32) -> f32 {
    0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32
}

/// Maximum truecolor-background luma found on the first captured line whose
/// ANSI-stripped text contains `needle`. `None` when no such line or no
/// `48;2;r;g;b` background is present.
fn max_bg_luma_on_line(raw: &str, needle: &str) -> Option<f32> {
    let re = regex_lite_bg();
    // Scan from the bottom so we read the *current* render, not stale scrollback
    // from a previous test sharing the pane.
    for line in raw.lines().rev() {
        let plain = biscuit_test_harness::strip_ansi(line);
        if plain.contains(needle) {
            let mut best: Option<f32> = None;
            for (r, g, b) in re(line) {
                let l = luma(r, g, b);
                best = Some(best.map_or(l, |m: f32| m.max(l)));
            }
            return best;
        }
    }
    None
}

/// Tiny hand-rolled scan for truecolor background SGRs, handling both the
/// legacy `\x1b[48;2;R;G;Bm` (semicolon) and the ITU `\x1b[48:2::R:G:Bm`
/// (colon, with empty colorspace) forms WezTerm emits in `get-text --escapes`.
fn regex_lite_bg() -> impl Fn(&str) -> Vec<(u32, u32, u32)> {
    |line: &str| {
        let mut out = Vec::new();
        // Split on the CSI introducer; each chunk starts with SGR params.
        for chunk in line.split("\x1b[").skip(1) {
            let Some(mend) = chunk.find('m') else {
                continue;
            };
            // Numeric params, ignoring empty fields (the colon form has an
            // empty colorspace slot: `48:2::R:G:B`).
            let nums: Vec<u32> = chunk[..mend]
                .split([';', ':'])
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();
            // Background truecolor: leading `48 2` then R G B.
            if nums.len() >= 5 && nums[0] == 48 && nums[1] == 2 {
                out.push((nums[2], nums[3], nums[4]));
            }
        }
        out
    }
}

/// True when a raw (ANSI-bearing) captured line is genuinely blank: no visible
/// glyphs AND no background fill. A code-block / page padding row paints `48`
/// background spaces and is content, not blank — the same rule the library-side
/// invariants use.
fn raw_line_is_blank(line: &str) -> bool {
    biscuit_test_harness::strip_ansi(line).trim().is_empty() && !line.contains("\x1b[48")
}

/// True when a line is *exactly* a completion sentinel (`__DM_LVL2_DONE_<n>__`)
/// on its own — not a wrapped fragment of the echoed command line.
fn is_sentinel_line(line: &str) -> bool {
    let t = line.trim();
    t.strip_prefix("__DM_LVL2_DONE_")
        .and_then(|r| r.strip_suffix("__"))
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// The rendered `md` output region: from the line containing `head_needle`
/// (inclusive) up to the *last* real sentinel line (exclusive). Excludes the
/// shell prompt and the echoed command (whose long temp path would otherwise
/// pollute width/blank-line scans).
fn rendered_region<'a>(lines: &'a [&'a str], head_needle: &str) -> (usize, usize) {
    let head = lines
        .iter()
        .position(|l| l.contains(head_needle))
        .unwrap_or(0);
    let sentinel = lines
        .iter()
        .rposition(|l| is_sentinel_line(l))
        .unwrap_or(lines.len());
    (head, sentinel)
}

const CODE_DOC: &str = "\
# Heading\n\
\n\
Some prose line.\n\
\n\
```rust\n\
pub struct FooBar {\n\
    foo: String,\n\
}\n\
```\n\
";

/// #0 — In a dark terminal the code panel must invert to a *light* theme so it
/// contrasts against the page. Forces dark detection via `COLORFGBG` and a
/// paired theme (`github`) that has a light variant.
#[test]
#[serial(level2_terminal)]
fn level2_code_block_inverts_to_light_in_dark_terminal() {
    let Some((frame, _)) = run_md_env(
        CODE_DOC,
        "--code-theme github --max-width 60",
        &[("COLORFGBG", "15;0")], // bg index 0 => dark terminal
    ) else {
        return;
    };

    let code_luma = max_bg_luma_on_line(&frame.raw, "rust").unwrap_or_else(|| {
        panic!(
            "no truecolor background found on the code line. raw:\n{}",
            frame.raw
        )
    });
    assert!(
        code_luma > 140.0,
        "code panel should be LIGHT (high luma) in a dark terminal, got luma {code_luma:.0}. \
         plain:\n{}",
        frame.plain
    );
}

/// #0 mirror — In a light terminal the code panel must invert to a *dark* theme.
#[test]
#[serial(level2_terminal)]
fn level2_code_block_inverts_to_dark_in_light_terminal() {
    let Some((frame, _)) = run_md_env(
        CODE_DOC,
        "--code-theme github --max-width 60",
        &[("COLORFGBG", "0;15")], // bg index 15 => light terminal
    ) else {
        return;
    };

    let code_luma = max_bg_luma_on_line(&frame.raw, "rust").unwrap_or_else(|| {
        panic!(
            "no truecolor background found on the code line. raw:\n{}",
            frame.raw
        )
    });
    assert!(
        code_luma < 120.0,
        "code panel should be DARK (low luma) in a light terminal, got luma {code_luma:.0}. \
         plain:\n{}",
        frame.plain
    );
}

/// #1/#2 — With left+right margins the code panel must stay within the content
/// rectangle: no rendered line exceeds the content width, and the right-margin
/// columns are blank (the old `\x1b[K` bug painted to the physical edge and
/// left a margin-width unstyled gap).
#[test]
#[serial(level2_terminal)]
fn level2_code_block_respects_right_margin() {
    let Some((frame, _)) = run_md_env(
        CODE_DOC,
        "--ml 4 --mr 4 --max-width 50",
        &[("COLORFGBG", "15;0")],
    ) else {
        return;
    };

    // Anchor on the code line and verify it carries a background fill.
    assert!(
        max_bg_luma_on_line(&frame.raw, "rust").is_some(),
        "code line must carry a background fill. raw:\n{}",
        frame.raw
    );

    // Content rectangle = ml(4) + max_width(50) + mr(4) = 58 columns. No
    // rendered line may exceed it (the bug overflowed to the physical edge).
    // Scope to the rendered region so the echoed command (long temp path) does
    // not pollute the width scan.
    let lines: Vec<&str> = frame.plain.lines().collect();
    let (head, sentinel) = rendered_region(&lines, "FooBar");
    let max_visible = lines[head..sentinel]
        .iter()
        .map(|l| rtrim(l).chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        max_visible <= 58,
        "no rendered line may exceed ml+max_width+mr = 58 cols, got {max_visible}. plain:\n{}",
        frame.plain
    );
}

/// #3 — A document ending in a code block must not accrue the old constant
/// trailing blank-line offset (the body emitted `mb + 2` blank rows). The
/// `printf` sentinel injects one trailing newline of its own, so the robust
/// bound above the sentinel is `mb + 1`; the pre-fix bug would produce `mb + 3`.
#[test]
#[serial(level2_terminal)]
fn level2_no_trailing_blank_offset_after_code() {
    let Some((frame, _)) = run_md_env(CODE_DOC, "--mb 1 --max-width 60", &[("COLORFGBG", "15;0")])
    else {
        return;
    };

    // Use raw lines so a bg-filled code padding row is not mistaken for a blank
    // margin row.
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let sentinel_idx = raw_lines
        .iter()
        .rposition(|l| is_sentinel_line(l))
        .unwrap_or_else(|| panic!("sentinel line not found. raw:\n{}", frame.raw));

    // Count genuinely-blank rows immediately above the sentinel. With mb=1 this
    // must be at most 2 (1 bottom margin + 1 printf newline); the pre-fix
    // constant +2 offset would have made it 4.
    let trailing_blanks = raw_lines[..sentinel_idx]
        .iter()
        .rev()
        .take_while(|l| raw_line_is_blank(l))
        .count();
    assert!(
        trailing_blanks <= 2,
        "trailing blank rows must not pile up (mb=1 ⇒ at most 2 with the printf \
         newline); the old +2 offset bug produced 4. got {trailing_blanks}. plain:\n{}",
        frame.plain
    );
}

// Interior vertical rhythm (no run of >=2 blank rows) is verified
// deterministically and across every shape by the library-side `I5` invariant
// in `darkmatter/lib/tests/render_invariants.rs`. A real-terminal version is
// omitted here because transient mid-scroll captures make it flaky without
// adding coverage the deterministic invariant does not already provide.

// =============================================================================
//   STYLE FRONTMATTER FIXTURE (sub-spec #2 acceptance — review-2 finding #2)
// =============================================================================
//
// These tests run the canonical `style-prop.md` fixture in a real WezTerm pane
// to verify the user-visible margin layout actually reaches the terminal. The
// pipe-captured CLI test (`cli.rs::style_fixture_cli_pipe_smoke_passes`) only
// exercises the markdown pass-through path because `OutputFormat::Auto` falls
// back to that path when stdout is not a TTY.

/// Frontmatter + body matching `darkmatter/example-docs/rendering/style-prop.md`.
/// Kept inline so the Level 2 helper's tempfile pattern works without rewiring
/// to an absolute fixture path. Uses a raw string so YAML indentation survives
/// (Rust's `\<newline>` line continuation would strip the leading whitespace).
const STYLE_PROP_FIXTURE: &str = r#"---
style:
    page:
        left-margin: 2ch
        right-margin: 4ch
        top-margin: 1
        bottom-margin: 0
---

# StylePropFixtureHeading

Sentinel_paragraph_for_left_margin_check.
"#;

#[test]
#[serial(level2_terminal)]
fn level2_style_fixture_applies_top_and_left_margins_in_real_terminal() {
    // Acceptance: `style.page.top-margin: 1` adds a blank row above the
    // rendered title, and `style.page.left-margin: 2ch` leaves >=2 leading
    // columns on non-empty content rows when rendered through the real
    // terminal pipeline. The fixture is composed inline to mirror
    // `darkmatter/example-docs/rendering/style-prop.md` page-level frontmatter.
    let Some((frame, _)) = run_md(STYLE_PROP_FIXTURE, "--max-width 60") else {
        return;
    };

    let lines: Vec<&str> = frame.plain.lines().collect();
    let marker_idx = lines
        .iter()
        .position(|l| l.contains("StylePropFixtureHeading"))
        .unwrap_or_else(|| panic!("heading not found in pane capture:\n{}", frame.plain));

    // Top-margin: 1 — at least one row above the heading must be blank
    // (after rtrim of left-margin spaces).
    assert!(
        marker_idx >= 1,
        "top-margin: 1 must leave at least one row above the heading, got idx {marker_idx}. plain:\n{}",
        frame.plain
    );
    let above = rtrim(lines[marker_idx - 1]);
    assert!(
        above.is_empty(),
        "row directly above heading should be blank (top-margin: 1), got: {above:?}. plain:\n{}",
        frame.plain
    );

    // Left-margin: 2ch — the heading line itself starts with >=2 leading
    // columns (the heading row carries the left-margin prefix too).
    let head_leading = lines[marker_idx].chars().take_while(|c| *c == ' ').count();
    assert!(
        head_leading >= 2,
        "expected >=2 leading columns for left-margin: 2ch on heading, got {head_leading}: {:?}. plain:\n{}",
        lines[marker_idx],
        frame.plain
    );
}

// =============================================================================
//   COMPONENT STYLE FRONTMATTER (sub-spec #3 acceptance — review-3 finding #2)
// =============================================================================
//
// These tests exercise the new `style.table.*`, `style.images.*`, and
// `style.block-quote.*` frontmatter path through the real `md` CLI in a real
// WezTerm pane. The Level 1 tests in `cli.rs` cover the resolved
// `DarkmatterPage` state and a single in-process render; these confirm the
// visible terminal layout for the same buckets actually reaches the user.

#[test]
#[serial(level2_terminal)]
fn level2_style_frontmatter_table_max_width_caps_visible_row() {
    // `style.table.max-width: 50%` at --max-width 80 must cap rendered table
    // rows at 40 visible columns. Anchored on a unique cell value so we never
    // accidentally match the shell command echo.
    let body = r#"---
style:
    table:
        max-width: 50%
---

| ColA | ColB | ColC |
| ---- | ---- | ---- |
| sentinel_fm_alpha | beta | gamma |
"#;

    let Some((frame, _)) = run_md(body, "--max-width 80") else {
        return;
    };

    let row = frame
        .plain
        .lines()
        .find(|l| l.contains("sentinel_fm_alpha"))
        .unwrap_or_else(|| {
            panic!(
                "expected table row containing 'sentinel_fm_alpha' in:\n{}",
                frame.plain
            )
        });
    let visible = rtrim(row).chars().count();
    assert!(
        visible <= 40,
        "frontmatter style.table.max-width: 50% must cap row to 40 cols (50% of 80), got {visible}: {row:?}"
    );
    assert!(
        visible < 80,
        "frontmatter style.table.max-width: 50% must constrain below page width, got {visible}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_frontmatter_table_right_alignment_pushes_row_to_right_edge() {
    // `style.table.alignment: right` plus `style.table.max-width: 40%` must
    // push the rendered table to the right edge of the page so its leading
    // indent exceeds what a left-aligned table at the same fill would have.
    let right = r#"---
style:
    table:
        alignment: right
        max-width: 40%
---

| A | B |
| - | - |
| sentinelR | xx |
"#;
    let left = r#"---
style:
    table:
        alignment: left
        max-width: 40%
---

| A | B |
| - | - |
| sentinelR | xx |
"#;

    let Some((right_frame, _)) = run_md(right, "--max-width 60") else {
        return;
    };
    let Some((left_frame, _)) = run_md(left, "--max-width 60") else {
        return;
    };

    let row_right = right_frame
        .plain
        .lines()
        .find(|l| l.contains("sentinelR"))
        .unwrap_or_else(|| {
            panic!(
                "right: data row missing. plain:\n---\n{}\n---",
                right_frame.plain
            )
        });
    let row_left = left_frame
        .plain
        .lines()
        .find(|l| l.contains("sentinelR"))
        .unwrap_or_else(|| {
            panic!(
                "left: data row missing. plain:\n---\n{}\n---",
                left_frame.plain
            )
        });
    let right_indent = row_right.chars().take_while(|c| *c == ' ').count();
    let left_indent = row_left.chars().take_while(|c| *c == ' ').count();
    assert!(
        right_indent > left_indent,
        "frontmatter style.table.alignment: right must indent more than left: right={right_indent}, left={left_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_frontmatter_images_alignment_indents_fallback_text() {
    // `style.images.alignment` flows through to image fallback rendering for
    // missing images, the same path Level 1 covers structurally.
    let right = r#"---
style:
    images:
        alignment: right
        max-width: 20ch
---

![Sentinel image alt](./does-not-exist.png)
"#;
    let left = r#"---
style:
    images:
        alignment: left
        max-width: 20ch
---

![Sentinel image alt](./does-not-exist.png)
"#;

    let Some((right_frame, _)) = run_md(right, "--max-width 60") else {
        return;
    };
    let Some((left_frame, _)) = run_md(left, "--max-width 60") else {
        return;
    };

    let line_right = right_frame
        .plain
        .lines()
        .find(|l| l.contains("Sentinel image alt"))
        .unwrap_or_else(|| {
            panic!(
                "right: expected an alt-text anchor in image fallback. plain:\n{}",
                right_frame.plain
            )
        });
    let line_left = left_frame
        .plain
        .lines()
        .find(|l| l.contains("Sentinel image alt"))
        .unwrap_or_else(|| {
            panic!(
                "left: expected an alt-text anchor in image fallback. plain:\n{}",
                left_frame.plain
            )
        });

    let right_indent = line_right.chars().take_while(|c| *c == ' ').count();
    let left_indent = line_left.chars().take_while(|c| *c == ' ').count();
    assert!(
        right_indent > left_indent,
        "frontmatter style.images.alignment: right must indent more than left: right={right_indent}, left={left_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_frontmatter_block_quote_max_width_caps_wrap_width() {
    // `style.block-quote.max-width: 50%` at --max-width 80 must wrap quoted
    // text under 40 visible columns. Block-quote lines are prefixed by the
    // `▐` indicator glyph.
    let body = r#"---
style:
    block-quote:
        max-width: 50%
---

> This is a fairly long quoted paragraph that should wrap onto a second visible line once the frontmatter max-width cap forces the blockquote render width to half the page width.
"#;

    let Some((frame, _)) = run_md(body, "--max-width 80") else {
        return;
    };

    let quote_lines: Vec<String> = frame
        .plain
        .lines()
        .filter(|l| l.contains('▐'))
        .map(|l| rtrim(l).to_string())
        .collect();
    assert!(
        quote_lines.len() >= 2,
        "blockquote should wrap onto multiple lines under style.block-quote.max-width: 50%. plain:\n{}",
        frame.plain
    );
    let max_len = quote_lines.iter().map(|l| l.chars().count()).max().unwrap();
    assert!(
        max_len <= 40,
        "blockquote lines should be capped to 40 cols (50% of 80), got max={max_len}. plain:\n{}",
        frame.plain
    );
}

// =============================================================================
//   LIST STYLE FRONTMATTER + CLI (sub-spec #4 acceptance — review-4 finding #3)
// =============================================================================
//
// These tests exercise the split `style.{ul,ol,li}.*` frontmatter path and the
// matching CLI `--align-lists` / `--align-ul` flags through the real `md` CLI
// in a real WezTerm pane. Level 1 (in-process) coverage exists for these
// already; these confirm the visible terminal layout for the user-observable
// list behaviors actually reaches the user.

#[test]
#[serial(level2_terminal)]
fn level2_style_ul_left_margin_offsets_bullet_in_real_terminal() {
    // `style.ul.left-margin: 4ch` must offset the bullet marker 4 columns from
    // the left when rendered through the real terminal pipeline.
    let body = r#"---
style:
    ul:
        left-margin: 4ch
---

- sentinel_ul_lm item
"#;

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    let list_line = frame
        .plain
        .lines()
        .find(|l| l.contains("sentinel_ul_lm"))
        .unwrap_or_else(|| {
            panic!(
                "expected list item with sentinel in pane capture:\n{}",
                frame.plain
            )
        });
    let leading = list_line.chars().take_while(|c| *c == ' ').count();
    assert!(
        leading >= 4,
        "style.ul.left-margin: 4ch must offset the marker by >=4 cols, got {leading}: {list_line:?}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_ul_max_width_caps_wrap_width_in_real_terminal() {
    // `style.ul.max-width: 40` must wrap the bullet body at no more than 40
    // visible columns through the real terminal pipeline.
    let body = r#"---
style:
    ul:
        max-width: 40
---

- sentinel_ul_mw This is a notably long bullet item that has to wrap to a second visible row once the frontmatter cap constrains the list render width to forty columns.
"#;

    let Some((frame, _)) = run_md(body, "--max-width 80") else {
        return;
    };

    let lines: Vec<&str> = frame.plain.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.contains("sentinel_ul_mw"))
        .unwrap_or_else(|| panic!("missing list anchor. plain:\n{}", frame.plain));
    let list_region: Vec<&str> = lines
        .iter()
        .skip(start)
        .take_while(|l| !l.trim().is_empty())
        .copied()
        .collect();
    let max_len = list_region
        .iter()
        .map(|l| rtrim(l).chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        max_len <= 40,
        "style.ul.max-width: 40 must cap list lines at 40 cols, got max={max_len}. region:\n{}",
        list_region.join("\n")
    );
    assert!(
        list_region.len() >= 2,
        "expected list to wrap onto multiple rows, got {}:\n{}",
        list_region.len(),
        list_region.join("\n")
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_ul_left_margin_plus_max_width_stack_in_real_terminal() {
    // `style.ul.left-margin: 4ch` plus `style.ul.max-width: 40` must stack
    // correctly: 4-cell offset outside the body, body wrapping at <= 40 cols.
    let body = r#"---
style:
    ul:
        left-margin: 4ch
        max-width: 40
---

- sentinel_ul_stack This is a notably long bullet item that has to wrap to a second visible row once the frontmatter max-width cap constrains the list render width to forty columns total.
"#;

    let Some((frame, _)) = run_md(body, "--max-width 80") else {
        return;
    };

    let lines: Vec<&str> = frame.plain.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.contains("sentinel_ul_stack"))
        .unwrap_or_else(|| panic!("missing list anchor. plain:\n{}", frame.plain));
    let list_region: Vec<&str> = lines
        .iter()
        .skip(start)
        .take_while(|l| !l.trim().is_empty())
        .copied()
        .collect();

    // First line must carry the 4-cell left margin.
    let first = list_region[0];
    let leading = first.chars().take_while(|c| *c == ' ').count();
    assert!(
        leading >= 4,
        "first list line must carry >=4ch left margin, got {leading}: {first:?}"
    );
    // Body (after the 4-cell margin) must wrap at <= 40 cols.
    let max_body = list_region
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let trimmed = l.strip_prefix("    ").unwrap_or(l);
            rtrim(trimmed).chars().count()
        })
        .max()
        .unwrap_or(0);
    assert!(
        max_body <= 40,
        "body (after 4ch margin) must wrap at <= 40 cols, got max body={max_body}. region:\n{}",
        list_region.join("\n")
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_ol_alignment_right_indents_more_than_left_in_real_terminal() {
    // `style.ol.alignment: right` plus `style.ol.max-width: 40` must push the
    // ordered list to the right edge versus the left-aligned baseline.
    let right = r#"---
style:
    ol:
        alignment: right
        max-width: 40
---

1. sentinel_ol_align item
"#;
    let left = r#"---
style:
    ol:
        alignment: left
        max-width: 40
---

1. sentinel_ol_align item
"#;

    let Some((right_frame, _)) = run_md(right, "--max-width 80") else {
        return;
    };
    let Some((left_frame, _)) = run_md(left, "--max-width 80") else {
        return;
    };

    let row_right = right_frame
        .plain
        .lines()
        .find(|l| l.contains("sentinel_ol_align"))
        .unwrap_or_else(|| {
            panic!(
                "right: ordered list row missing. plain:\n{}",
                right_frame.plain
            )
        });
    let row_left = left_frame
        .plain
        .lines()
        .find(|l| l.contains("sentinel_ol_align"))
        .unwrap_or_else(|| {
            panic!(
                "left: ordered list row missing. plain:\n{}",
                left_frame.plain
            )
        });
    let right_indent = row_right.chars().take_while(|c| *c == ' ').count();
    let left_indent = row_left.chars().take_while(|c| *c == ' ').count();
    assert!(
        right_indent > left_indent,
        "style.ol.alignment: right must indent more than left: right={right_indent}, left={left_indent}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_li_alignment_right_aligns_body_in_real_terminal() {
    // Per spec, `style.li.alignment: right` affects the item body only — the
    // marker stays at the column dictated by the containing Ul. The body
    // becomes a block on its own line that is right-aligned within
    // `effective_width - body_width`.
    let body = r#"---
style:
    li:
        alignment: right
        max-width: 40
---

- sentinel_li_body
"#;

    let Some((frame, _)) = run_md(body, "--max-width 80") else {
        return;
    };

    let lines: Vec<&str> = frame.plain.lines().collect();
    // Marker stays at column 0 (or wherever Ul column is, which is 0 here
    // since no Ul overrides).
    let marker_line = lines
        .iter()
        .find(|l| l.trim_start().starts_with('-') && !l.contains("sentinel_li_body"))
        .unwrap_or_else(|| panic!("marker line not found. plain:\n{}", frame.plain));
    let marker_leading = marker_line.chars().take_while(|c| *c == ' ').count();
    assert!(
        marker_leading == 0,
        "marker should stay at Ul column 0 (style.li.* affects body only), got {marker_leading}: {marker_line:?}"
    );
    // Body line is right-aligned within the effective width, well past column 0.
    let body_line = lines
        .iter()
        .find(|l| l.contains("sentinel_li_body"))
        .unwrap_or_else(|| panic!("body line not found. plain:\n{}", frame.plain));
    let body_leading = body_line.chars().take_while(|c| *c == ' ').count();
    assert!(
        body_leading >= 30,
        "li body must be right-aligned, got {body_leading} leading spaces: {body_line:?}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_cli_align_lists_broadcast_indents_in_real_terminal() {
    // `--align-lists center` plus `--fill-lists max=30` must broadcast to Ul/Ol/Li
    // so both bullet and numbered items render with extra leading indent vs
    // left-aligned baseline.
    let body = "- sentinel_alpha\n1. sentinel_numbered\n";

    let Some((left, _)) = run_md(
        body,
        "--align-lists left --fill-lists max=30 --max-width 60",
    ) else {
        return;
    };
    let Some((center, _)) = run_md(
        body,
        "--align-lists center --fill-lists max=30 --max-width 60",
    ) else {
        return;
    };

    let find_row = |plain: &str, needle: &str, label: &str| -> String {
        plain
            .lines()
            .find(|l| l.contains(needle))
            .map(|l| l.to_string())
            .unwrap_or_else(|| panic!("{label}: row '{needle}' not found in plain:\n{plain}"))
    };
    let left_ul = find_row(&left.plain, "sentinel_alpha", "left ul");
    let center_ul = find_row(&center.plain, "sentinel_alpha", "center ul");
    let left_ol = find_row(&left.plain, "sentinel_numbered", "left ol");
    let center_ol = find_row(&center.plain, "sentinel_numbered", "center ol");

    let lead = |s: &str| s.chars().take_while(|c| *c == ' ').count();
    assert!(
        lead(&center_ul) > lead(&left_ul),
        "--align-lists center must indent Ul more than left: left={}, center={}",
        lead(&left_ul),
        lead(&center_ul)
    );
    assert!(
        lead(&center_ol) > lead(&left_ol),
        "--align-lists center must indent Ol more than left: left={}, center={}",
        lead(&left_ol),
        lead(&center_ol)
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_cli_align_ul_granular_indents_only_ul_in_real_terminal() {
    // `--align-ul center` plus `--fill-ul max=30` must indent the unordered list
    // versus the left baseline, while leaving an ordered list unaffected.
    let body = "- sentinel_ul_only\n\n1. sentinel_ol_only\n";

    let Some((left, _)) = run_md(
        body,
        "--align-ul left --fill-ul max=30 --max-width 60",
    ) else {
        return;
    };
    let Some((center, _)) = run_md(
        body,
        "--align-ul center --fill-ul max=30 --max-width 60",
    ) else {
        return;
    };

    let find_row = |plain: &str, needle: &str, label: &str| -> String {
        plain
            .lines()
            .find(|l| l.contains(needle))
            .map(|l| l.to_string())
            .unwrap_or_else(|| panic!("{label}: row '{needle}' not found in plain:\n{plain}"))
    };
    let left_ul = find_row(&left.plain, "sentinel_ul_only", "left ul");
    let center_ul = find_row(&center.plain, "sentinel_ul_only", "center ul");
    let left_ol = find_row(&left.plain, "sentinel_ol_only", "left ol");
    let center_ol = find_row(&center.plain, "sentinel_ol_only", "center ol");

    let lead = |s: &str| s.chars().take_while(|c| *c == ' ').count();
    assert!(
        lead(&center_ul) > lead(&left_ul),
        "--align-ul center must indent Ul more than left: left={}, center={}",
        lead(&left_ul),
        lead(&center_ul)
    );
    // Ol must NOT shift: --align-ul only targets Ul.
    assert_eq!(
        lead(&center_ol),
        lead(&left_ol),
        "--align-ul must not affect Ol: left_ol={}, center_ol={}",
        lead(&left_ol),
        lead(&center_ol)
    );
}

// ---------------------------------------------------------------------------
// Review-5 follow-ups: sub-spec #5 color behavior in a real terminal
// ---------------------------------------------------------------------------

/// `style.page.color` plus the absence of a color-capable terminal must
/// still render layout (heading, list, table) — the renderer no longer
/// falls back to raw Markdown source when the captured terminal cannot
/// interpret SGR.
///
/// The harness reports `xterm-256color`, so `ColorDepth::auto_detect`
/// resolves to a color-capable depth; we exercise the no-color
/// safety net by forcing `TERM=dumb`, which downgrades depth to
/// `ColorDepth::None`. The visible text must still appear.
#[test]
#[serial(level2_terminal)]
fn level2_color_depth_none_preserves_visible_layout() {
    let body = "---\n\
style:\n  page:\n    color: red-500\n---\n\
# UniqueNoColorHeading\n\n\
- BulletItem\n\n\
| Hdr |\n|---|\n| Cell |\n";

    let Some((frame, _)) = run_md_env(body, "--max-width 40", &[("TERM", "dumb")]) else {
        return;
    };

    // All structural anchors must survive ColorDepth::None.
    assert!(
        frame.plain.contains("UniqueNoColorHeading"),
        "heading text must render under ColorDepth::None. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("BulletItem"),
        "list body must render under ColorDepth::None. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("Hdr") && frame.plain.contains("Cell"),
        "table cells must render under ColorDepth::None. plain:\n{}",
        frame.plain
    );
}

/// Visible terminal capture for the fixed list inheritance: `style.ul.color`
/// must surface on list item bodies even when `style.li.color` is unset.
/// We anchor on the SGR byte sequence in the raw stream because the
/// `plain` view strips ANSI codes.
#[test]
#[serial(level2_terminal)]
fn level2_ul_color_inherits_into_li_body() {
    let body = "---\n\
style:\n  ul:\n    color: red-500\n---\n\
- listbodyalpha\n- listbodybeta\n";

    let Some((frame, _)) = run_md(body, "--max-width 40") else {
        return;
    };

    // Tailwind Red-500 lowers to `\x1b[38;2;251;44;54m` (see
    // `darkmatter::style::lower_to_sgr`). At least two occurrences are
    // expected: one per body anchor.
    let red_sgr = "\x1b[38;2;251;44;54m";
    let occurrences = frame.raw.matches(red_sgr).count();
    assert!(
        occurrences >= 2,
        "ul.color must wrap each item body (>=2 red SGR). got {} occurrences. raw len {}",
        occurrences,
        frame.raw.len()
    );
    // Layout must also still show the bodies in the plain view.
    assert!(
        frame.plain.contains("listbodyalpha") && frame.plain.contains("listbodybeta"),
        "list bodies missing in plain capture:\n{}",
        frame.plain
    );
}

/// Visible terminal capture for hyperlink color routing inside table cells:
/// `style.hyperlinks.color` must wrap the link's label even when the link
/// lives inside a `<table>` cell, and the OSC8 sequence must be intact.
#[test]
#[serial(level2_terminal)]
fn level2_hyperlink_color_applies_inside_table() {
    let body = "---\n\
style:\n  hyperlinks:\n    color: red-500\n  table:\n    color: blue-500\n---\n\
| col |\n|---|\n| [clickanchor](https://example.com) |\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    let red_sgr = "\x1b[38;2;251;44;54m";
    assert!(
        frame.raw.contains(red_sgr),
        "hyperlink color must appear inside table cell. raw stream:\n{}",
        frame.raw
    );
    // The OSC8 wrapping must remain so the link is clickable.
    assert!(
        frame.raw.contains("\x1b]8;;https://example.com"),
        "OSC8 link must be preserved in table cell. raw stream:\n{}",
        frame.raw
    );
    // Visible label must remain in the plain capture.
    assert!(
        frame.plain.contains("clickanchor"),
        "link label must render in plain capture:\n{}",
        frame.plain
    );
}
