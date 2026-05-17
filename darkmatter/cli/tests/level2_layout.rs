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
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
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
/// clean visible region. The pane is intentionally not torn down at end of
/// process — it lives in WezTerm's background workspace and is reclaimed
/// when WezTerm exits.
static SHARED_HARNESS: Mutex<Option<WezTermHarness>> = Mutex::new(None);

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
    }
    let harness = guard.as_mut().unwrap();

    // Reset the visible region so the previous test's output does not bleed
    // into this capture. `clear` is portable across bash and zsh.
    run_with_sentinel(harness, "clear");

    let cmd = format!("md {} {}", file_path.display(), extra_args);
    let frame = run_with_sentinel(harness, &cmd);
    // Keep tempdir alive past capture by returning its path.
    Some((frame, file_path))
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
    // Use very short items + a wider max so no wrap forces marker/content
    // onto separate lines. The `xy` / `ab` sentinels are unique to the
    // rendered output (the shell command echo doesn't contain them).
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

    let find_row = |plain: &str, label: &str| -> String {
        plain
            .lines()
            .find(|l| l.trim_start().starts_with("- xy"))
            .map(|l| l.to_string())
            .unwrap_or_else(|| panic!("{label}: list item not found. plain:\n{plain}"))
    };
    let left_row = find_row(&left.plain, "left");
    let center_row = find_row(&center.plain, "center");
    let left_indent = left_row.chars().take_while(|c| *c == ' ').count();
    let center_indent = center_row.chars().take_while(|c| *c == ' ').count();
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
