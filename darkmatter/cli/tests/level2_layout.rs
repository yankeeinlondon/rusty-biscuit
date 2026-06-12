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
//! Set `BISCUIT_TEST_LEVEL_REQUIRED=2` in the environment to convert a missing
//! WezTerm into a hard failure rather than a silent skip. CI jobs that
//! provision WezTerm should always set this so Level 2 coverage is
//! actually enforced, not just nominally present.
//!
//! ## WezTerm pane-capture pitfalls
//!
//! `harness.capture()` reads the pane via `wezterm cli get-text --escapes`,
//! which **walks the cell grid and emits SGR for transitions only**:
//!
//! - Contiguous same-attribute cells collapse into a single SGR span; the
//!   leading SGR may appear on a previous row and not re-appear on the next.
//! - Truecolor SGR is re-emitted in either semicolon (`\x1b[48;2;R;G;Bm`) or
//!   ITU colon (`\x1b[48:2::R:G:Bm`) form depending on terminfo and version.
//! - `\x1b[0m` in the source may come back as `\x1b[39m\x1b[49m` (or be elided
//!   entirely if the following cell has the same attributes).
//!
//! Consequence: **per-line byte equality across two captures is unreliable**
//! even when the underlying `md` output is byte-identical (verifiable by
//! running the command under `script(1)` and diffing). Prefer semantic
//! assertions on the full `frame.raw` stream — presence of expected SGR bytes
//! (in both semicolon and colon form) and absence of disallowed SGR bytes.
//! See `level2_cli_code_theme_overrides_style_page_code_theme` and commit
//! `be5d0409e` for the canonical pattern.

use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Level, LevelDecision, evaluate_level};

/// Gating decision for Level-2 WezTerm tests.
fn wezterm_decision() -> LevelDecision {
    evaluate_level(Level::L2, WezTermHarness::available(), "WezTerm")
}

/// Process-wide shared WezTerm pane reused across every test in this file.
///
/// Spawning a fresh WezTerm window costs ≈2–3 s plus prompt readiness. Every
/// test in this file is `#[serial(level2_terminal)]`, so a single pane is
/// safe to share. [`SharedHarness`] wraps the `Mutex<Option<T>>` +
/// `libc::atexit` cleanup pattern so the pane is killed at process exit
/// instead of leaking into the `biscuit-bg` workspace (Rust does not run
/// `Drop` on `static` values).
static SHARED_HARNESS: SharedHarness<WezTermHarness> = SharedHarness::new();

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
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return None;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("layout.md");
    fs::write(&file_path, file_body).unwrap();

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
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
    // Indent(20) sets padding-left = 20ch. The content box is 60 cols,
    // and the prefix consumes 4 cols, leaving 56 cols for text.
    // Total line width = 20 (pad) + 4 (prefix) + text ≤ 80.
    assert!(
        max_len <= 80,
        "blockquote lines must be capped to 80 cols by Indent(20) padding; got max={max_len}. plain:\n{}",
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

fn min_fg_luma_on_line(raw: &str, needle: &str) -> Option<f32> {
    let re = regex_lite_fg();
    for line in raw.lines().rev() {
        let plain = biscuit_test_harness::strip_ansi(line);
        if plain.contains(needle) {
            let mut best: Option<f32> = None;
            for (r, g, b) in re(line) {
                let l = luma(r, g, b);
                best = Some(best.map_or(l, |m: f32| m.min(l)));
            }
            return best;
        }
    }
    None
}

fn max_fg_luma_on_line(raw: &str, needle: &str) -> Option<f32> {
    let re = regex_lite_fg();
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

fn regex_lite_fg() -> impl Fn(&str) -> Vec<(u32, u32, u32)> {
    |line: &str| {
        let mut out = Vec::new();
        for chunk in line.split("\x1b[").skip(1) {
            let Some(mend) = chunk.find('m') else {
                continue;
            };
            let nums: Vec<u32> = chunk[..mend]
                .split([';', ':'])
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();
            if nums.len() >= 5 && nums[0] == 38 && nums[1] == 2 {
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

#[test]
#[serial(level2_terminal)]
fn level2_default_code_block_inverts_background_and_foreground() {
    let mut captured: Option<CapturedFrame> = None;
    let mut bg: Option<f32> = None;
    let mut min_fg: Option<f32> = None;
    let mut max_fg: Option<f32> = None;

    for _ in 0..3 {
        let Some((frame, _)) = run_md_env(
            CODE_DOC,
            "--max-width 60",
            &[("COLORFGBG", "15;0")], // bg index 0 => dark terminal
        ) else {
            return;
        };

        bg = max_bg_luma_on_line(&frame.raw, "FooBar");
        min_fg = min_fg_luma_on_line(&frame.raw, "FooBar");
        max_fg = max_fg_luma_on_line(&frame.raw, "FooBar");
        captured = Some(frame);
        if bg.is_some() && min_fg.is_some() && max_fg.is_some() {
            break;
        }
    }

    let frame = captured.expect("capture should be present");
    let code_bg = bg.unwrap_or_else(|| {
        panic!(
            "no truecolor background found on the default-theme code line. raw:\n{}",
            frame.raw
        )
    });
    let darkest_fg = min_fg.unwrap_or_else(|| {
        panic!(
            "no truecolor foreground found on the default-theme code line. raw:\n{}",
            frame.raw
        )
    });
    let brightest_fg = max_fg.unwrap_or_else(|| {
        panic!(
            "no truecolor foreground found on the default-theme code line. raw:\n{}",
            frame.raw
        )
    });

    assert!(
        code_bg > 175.0,
        "default code block should use a light page-inverted background in a dark terminal, got luma {code_bg:.0}. \
         plain:\n{}",
        frame.plain
    );
    assert!(
        darkest_fg < 140.0,
        "default code block should use dark token foregrounds on the light inverted background, got darkest luma {darkest_fg:.0}. \
         plain:\n{}",
        frame.plain
    );
    assert!(
        brightest_fg < 190.0,
        "default code block should not keep bright dark-theme foregrounds on the light inverted background, got brightest luma {brightest_fg:.0}. \
         plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_code_block_clears_inherited_dim_before_theme_colors() {
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("dim-code.md");
    fs::write(&file_path, CODE_DOC).unwrap();

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();

    let cmd = format!(
        "printf '\\033[2m'; COLORFGBG='15;0' md {} --max-width 60",
        file_path.display()
    );

    let mut captured: Option<CapturedFrame> = None;
    let mut bg: Option<f32> = None;
    for _ in 0..3 {
        run_with_sentinel(harness, "clear");
        let frame = run_with_sentinel(harness, &cmd);
        bg = max_bg_luma_on_line(&frame.raw, "FooBar");
        captured = Some(frame);
        if bg.is_some() {
            break;
        }
    }

    let frame = captured.expect("capture should be present");
    let code_bg = bg.unwrap_or_else(|| {
        panic!(
            "no truecolor background found on the dim-inherited code line. raw:\n{}",
            frame.raw
        )
    });

    assert!(
        code_bg > 175.0,
        "code block must clear inherited dim before applying the page-inverted background, got luma {code_bg:.0}. \
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

fn foreground_at_text(raw: &str, needle: &str) -> Option<Option<(u8, u8, u8)>> {
    let target = raw.find(needle)?;
    let mut fg = None;
    let mut i = 0;
    while i < target {
        let rest = &raw[i..];
        if let Some(after_csi) = rest.strip_prefix("\x1b[")
            && let Some(end) = after_csi.find('m')
        {
            apply_sgr_foreground(&after_csi[..end], &mut fg);
            i += 2 + end + 1;
            continue;
        }
        i += rest.chars().next()?.len_utf8();
    }
    Some(fg)
}

fn apply_sgr_foreground(params: &str, fg: &mut Option<(u8, u8, u8)>) {
    if params.is_empty() {
        *fg = None;
        return;
    }

    for param in params.split(';') {
        match param {
            "0" | "39" => *fg = None,
            colon if colon.starts_with("38:2:") => {
                let values: Vec<u8> = colon
                    .split(':')
                    .filter_map(|part| part.parse::<u8>().ok())
                    .collect();
                if values.len() >= 5 {
                    *fg = Some((values[2], values[3], values[4]));
                }
            }
            _ => {}
        }
    }

    let mut semicolon = params.split(';');
    while let Some(param) = semicolon.next() {
        if param == "38" && semicolon.next() == Some("2") {
            let Some(r) = semicolon.next().and_then(|value| value.parse::<u8>().ok()) else {
                continue;
            };
            let Some(g) = semicolon.next().and_then(|value| value.parse::<u8>().ok()) else {
                continue;
            };
            let Some(b) = semicolon.next().and_then(|value| value.parse::<u8>().ok()) else {
                continue;
            };
            *fg = Some((r, g, b));
        }
    }
}

/// Visible terminal capture for the fixed list inheritance: `style.ul.color`
/// must surface on list item bodies even when `style.li.color` is unset.
#[test]
#[serial(level2_terminal)]
fn level2_ul_color_inherits_into_li_body() {
    let body = "---\n\
style:\n  ul:\n    color: red-500\n---\n\
- listbodyalpha\n- listbodybeta\n";

    let Some((frame, _)) = run_md(body, "--max-width 40") else {
        return;
    };

    let red_500 = Some((251, 44, 54));
    assert_eq!(
        foreground_at_text(&frame.raw, "listbodyalpha").flatten(),
        red_500,
        "first list body must inherit ul.color. raw={:?}",
        frame.raw
    );
    assert_eq!(
        foreground_at_text(&frame.raw, "listbodybeta").flatten(),
        red_500,
        "second list body must inherit ul.color. raw={:?}",
        frame.raw
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

    // WezTerm re-emits truecolor SGR as either semicolon or ITU colon form.
    let red_semi = "\x1b[38;2;251;44;54m";
    let red_colon = "\x1b[38:2::251:44:54m";
    assert!(
        frame.raw.contains(red_semi) || frame.raw.contains(red_colon),
        "hyperlink color must appear inside table cell. raw={:?}, plain={:?}",
        frame.raw,
        frame.plain,
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

// =============================================================================
//   HR STYLE FRONTMATTER — canonical `style.hr.*` path (sub-spec #6, review-6)
// =============================================================================
//
// Review-6 finding 3: the canonical `style.hr.*` frontmatter path needs Level 2
// real-terminal coverage. Below tests exercise the canonical path (NOT the
// legacy top-level `hr:` block, NOT inline `--- { ... }` attributes) through
// the real `md` CLI in a WezTerm pane.

#[test]
#[serial(level2_terminal)]
fn level2_style_hr_kind_waves_renders_in_real_terminal() {
    // `style.hr.kind: waves` must reach the HR renderer through the canonical
    // path. Unicode-capable terminals print `≋`; ASCII fallback prints `~`.
    let body = r#"---
style:
    hr:
        kind: waves
---

hr_waves_lead_anchor

---

hr_waves_tail_anchor
"#;

    // Force the text tier: in a graphics-capable terminal the styled HR
    // rasterizes to an image and no glyph reaches a text row (review-1
    // finding 3). The assertion is also anchored between sentinels so a stray
    // `~` from the shell prompt cannot satisfy it.
    let Some((frame, _)) = run_md_env(body, "--max-width 60", &[("TERMINAL_IMAGES", "0")]) else {
        return;
    };

    let Some((plain, _)) =
        locate_hr_between_sentinels(&frame, "hr_waves_lead_anchor", "hr_waves_tail_anchor")
    else {
        panic!(
            "expected a waves HR rule row between the sentinels but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame.plain, frame.raw
        );
    };

    assert!(
        plain.contains('\u{224B}') || plain.contains('~'),
        "style.hr.kind: waves must produce the waves glyph (`≋` or `~`) on the rule row; got: {plain:?}",
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_hr_weight_thick_differs_from_thin_in_real_terminal() {
    // `style.hr.weight: thick` vs `thin` must produce visibly different bytes
    // in the captured pane (verified separately via terminal_text_options in
    // Level 1; this proves the difference survives a real terminal).
    let body_thick = r#"---
style:
    hr:
        kind: dashes
        weight: thick
---

hr_weight_lead_anchor

---

hr_weight_tail_anchor
"#;
    let body_thin = r#"---
style:
    hr:
        kind: dashes
        weight: thin
---

hr_weight_lead_anchor

---

hr_weight_tail_anchor
"#;

    // Force the text tier so the weight difference appears as glyphs rather than
    // pixels, and isolate the rule row between sentinels so the comparison
    // cannot accidentally match the (per-invocation distinct) command echo
    // (review-1 finding 3).
    let Some((frame_thick, _)) =
        run_md_env(body_thick, "--max-width 60", &[("TERMINAL_IMAGES", "0")])
    else {
        return;
    };
    let Some((frame_thin, _)) =
        run_md_env(body_thin, "--max-width 60", &[("TERMINAL_IMAGES", "0")])
    else {
        return;
    };

    let Some((thick_rule_line, _)) =
        locate_hr_between_sentinels(&frame_thick, "hr_weight_lead_anchor", "hr_weight_tail_anchor")
    else {
        panic!(
            "expected a thick HR rule row but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame_thick.plain, frame_thick.raw
        );
    };
    let Some((thin_rule_line, _)) =
        locate_hr_between_sentinels(&frame_thin, "hr_weight_lead_anchor", "hr_weight_tail_anchor")
    else {
        panic!(
            "expected a thin HR rule row but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame_thin.plain, frame_thin.raw
        );
    };

    assert_ne!(
        thick_rule_line.trim(),
        thin_rule_line.trim(),
        "thick and thin HR weights must render visibly different glyphs",
    );
}

/// Find the captured rule line between the unique sentinels `LEAD_SENTINEL`
/// and `TAIL_SENTINEL`. Returns the matching `(plain_line, raw_line)` pair
/// or `None` when the rule is missing or scrolled out of the capture.
fn locate_hr_between_sentinels<'a>(
    frame: &'a CapturedFrame,
    lead: &str,
    tail: &str,
) -> Option<(&'a str, &'a str)> {
    let plain_lines: Vec<&str> = frame.plain.lines().collect();
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let lead_idx = plain_lines.iter().position(|l| l.contains(lead))?;
    let tail_idx = plain_lines.iter().position(|l| l.contains(tail))?;
    if lead_idx >= tail_idx {
        return None;
    }
    // The rule glyph lives on some line strictly between the sentinels.
    for i in (lead_idx + 1)..tail_idx {
        let line = plain_lines.get(i)?;
        // Skip blank rows; the rule itself carries visible glyphs.
        if line.trim().is_empty() {
            continue;
        }
        let raw_line = raw_lines.get(i).copied().unwrap_or("");
        return Some((line, raw_line));
    }
    None
}

#[test]
#[serial(level2_terminal)]
fn level2_style_hr_color_emits_sgr_in_real_terminal() {
    // `style.hr.color: red-500` must emit a red SGR escape on the rule row.
    // We use unique sentinels around the rule to isolate the captured row
    // and accept both WezTerm SGR re-emission forms (semicolon, colon).
    let body = r#"---
style:
    hr:
        color: red-500
---

hr_color_lead_anchor

---

hr_color_tail_anchor
"#;

    // Force the text tier: in a graphics-capable terminal (WezTerm supports the
    // Kitty graphics protocol) a styled HR rasterizes to an image, so the text
    // rule row — and the foreground SGR this test asserts — never appears. The
    // color is a text-rule property, so it must be exercised on the text tier
    // (review-1 finding 3).
    let Some((frame, _)) = run_md_env(body, "--max-width 60", &[("TERMINAL_IMAGES", "0")]) else {
        return;
    };

    let Some((_plain, raw)) =
        locate_hr_between_sentinels(&frame, "hr_color_lead_anchor", "hr_color_tail_anchor")
    else {
        // The harness was available and `md` completed (we have a frame), so a
        // missing rule row is a real failure of a terminal-visible requirement,
        // not an environment skip (review-1 finding 3).
        panic!(
            "expected an HR rule row between the sentinels but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame.plain, frame.raw
        );
    };

    let red_semi = "\x1b[38;2;251;44;54m";
    let red_colon = "\x1b[38:2::251:44:54m";
    assert!(
        raw.contains(red_semi) || raw.contains(red_colon),
        "style.hr.color must reach the rule row as a foreground SGR. \
         raw row:\n{raw}\nfull raw:\n{}",
        frame.raw
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_hr_bg_color_emits_background_sgr_in_real_terminal() {
    // `style.hr.bg-color: blue-500` must paint a background SGR on the rule
    // row. WezTerm re-emits truecolor backgrounds as `48;2;…` or `48:2:…`.
    let body = r#"---
style:
    hr:
        bg-color: blue-500
---

hr_bg_lead_anchor

---

hr_bg_tail_anchor
"#;

    // Force the text tier so the rule paints as a real row (see the color test):
    // a graphics-capable terminal would rasterize the styled HR to an image and
    // the background SGR would never reach a text row (review-1 finding 3).
    let Some((frame, _)) = run_md_env(body, "--max-width 60", &[("TERMINAL_IMAGES", "0")]) else {
        return;
    };

    let Some((_plain, raw)) =
        locate_hr_between_sentinels(&frame, "hr_bg_lead_anchor", "hr_bg_tail_anchor")
    else {
        // Harness available + `md` completed: a missing rule row is a real
        // failure, not an environment skip (review-1 finding 3).
        panic!(
            "expected an HR rule row between the sentinels but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame.plain, frame.raw
        );
    };

    let bg_present = raw.contains("\x1b[48;2;") || raw.contains("\x1b[48:2:");
    assert!(
        bg_present,
        "style.hr.bg-color must paint a background SGR on the rule row. \
         raw row:\n{raw}\nfull raw:\n{}",
        frame.raw
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_hr_alignment_center_offsets_rule_from_left_in_real_terminal() {
    // `style.hr.alignment: center` plus a narrow `width: 20` must offset
    // the rule from the left edge. We compare the leading-space count of
    // the rule row in a centered render against a left-aligned render.
    let centered = r#"---
style:
    hr:
        kind: dashes
        alignment: center
        width: 20
---

hr_align_lead_anchor

---

hr_align_tail_anchor
"#;
    let left = r#"---
style:
    hr:
        kind: dashes
        alignment: left
        width: 20
---

hr_align_lead_anchor

---

hr_align_tail_anchor
"#;

    // Force the text tier: a rasterized HR encodes alignment in pixels, not in
    // leading whitespace, so the indent comparison this test makes is only
    // meaningful on the text rule (review-1 finding 3).
    let Some((frame_center, _)) =
        run_md_env(centered, "--max-width 60", &[("TERMINAL_IMAGES", "0")])
    else {
        return;
    };
    let Some((frame_left, _)) = run_md_env(left, "--max-width 60", &[("TERMINAL_IMAGES", "0")])
    else {
        return;
    };
    let Some((plain_center, _)) = locate_hr_between_sentinels(
        &frame_center,
        "hr_align_lead_anchor",
        "hr_align_tail_anchor",
    ) else {
        // Harness available + `md` completed: a missing rule row is a real
        // failure, not an environment skip (review-1 finding 3).
        panic!(
            "expected a centered HR rule row but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame_center.plain, frame_center.raw
        );
    };
    let Some((plain_left, _)) =
        locate_hr_between_sentinels(&frame_left, "hr_align_lead_anchor", "hr_align_tail_anchor")
    else {
        panic!(
            "expected a left-aligned HR rule row but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame_left.plain, frame_left.raw
        );
    };

    let center_indent = plain_center.chars().take_while(|c| *c == ' ').count();
    let left_indent = plain_left.chars().take_while(|c| *c == ' ').count();
    assert!(
        center_indent > left_indent,
        "centered HR must have more leading whitespace than left-aligned; \
         center={center_indent}, left={left_indent}\ncentered row: {plain_center:?}\nleft row: {plain_left:?}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_style_hr_width_caps_visible_columns_in_real_terminal() {
    // `style.hr.width: 20` must produce a rule whose visible glyphs span no
    // more than 20 columns, regardless of the surrounding page width.
    let body = r#"---
style:
    hr:
        kind: dashes
        width: 20
---

hr_width_lead_anchor

---

hr_width_tail_anchor
"#;

    // Force the text tier: a rasterized HR encodes its width in pixels, so the
    // visible-glyph-count cap this test asserts only applies to the text rule
    // (review-1 finding 3).
    let Some((frame, _)) = run_md_env(body, "--max-width 60", &[("TERMINAL_IMAGES", "0")]) else {
        return;
    };
    let Some((plain, _)) =
        locate_hr_between_sentinels(&frame, "hr_width_lead_anchor", "hr_width_tail_anchor")
    else {
        // Harness available + `md` completed: a missing rule row is a real
        // failure, not an environment skip (review-1 finding 3).
        panic!(
            "expected an HR rule row between the sentinels but none was captured.\nfull plain:\n{}\nfull raw:\n{}",
            frame.plain, frame.raw
        );
    };

    // Count only the visible rule glyphs (skip the left padding). Dashes
    // render as `╌` (Unicode) or `-` (ASCII fallback). The rule glyphs are
    // contiguous; non-rule characters are spaces.
    let rule_glyph_count = plain
        .chars()
        .filter(|c| !c.is_whitespace())
        .count();
    assert!(
        rule_glyph_count > 0,
        "expected visible rule glyphs in row:\n{plain}\nfull plain:\n{}",
        frame.plain
    );
    assert!(
        rule_glyph_count <= 20,
        "style.hr.width: 20 must cap the visible rule to <=20 glyphs; got {rule_glyph_count} \
         in row:\n{plain}"
    );
}

// =============================================================================
//  SUB-SPEC #7 (review-7) — page code theme, hyperlink layout, local-image
//  fallback styling. Real-terminal captures for behaviours that previously had
//  Level 1 coverage only.
// =============================================================================

/// `style.page.code.theme: dracula` must change visible code-block bytes
/// relative to the same document rendered with a different theme. We compare
/// the SGR raw streams; identical bytes would prove the frontmatter was
/// ignored.
#[test]
#[serial(level2_terminal)]
fn level2_style_page_code_theme_changes_terminal_rendering() {
    let doc_dracula = "---\nstyle:\n  page:\n    code:\n      theme: dracula\n---\n\n\
        ```rust\nfn _theme_marker_dm() { let x = 1; }\n```\n";
    let doc_nord = "---\nstyle:\n  page:\n    code:\n      theme: nord\n---\n\n\
        ```rust\nfn _theme_marker_dm() { let x = 1; }\n```\n";

    let Some((dracula, _)) = run_md(doc_dracula, "--max-width 60") else {
        return;
    };
    let Some((nord, _)) = run_md(doc_nord, "--max-width 60") else {
        return;
    };

    assert!(
        dracula.plain.contains("_theme_marker_dm"),
        "code body missing from dracula capture:\n{}",
        dracula.plain
    );
    assert!(
        nord.plain.contains("_theme_marker_dm"),
        "code body missing from nord capture:\n{}",
        nord.plain
    );
    assert_ne!(
        dracula.raw, nord.raw,
        "style.page.code.theme: dracula vs nord must produce different SGR bytes"
    );
}

/// CLI `--code-theme` must beat `style.page.code.theme`. With frontmatter set
/// to `dracula` and CLI passing `--code-theme nord`, the rendered output must
/// use the nord theme — its panel background and at least one of its signature
/// syntax foregrounds — and must not use the dracula panel background.
#[test]
#[serial(level2_terminal)]
fn level2_cli_code_theme_overrides_style_page_code_theme() {
    let doc_with_fm = "---\nstyle:\n  page:\n    code:\n      theme: dracula\n---\n\n\
        ```rust\nfn _cli_override_marker() { let x = 1; }\n```\n";

    let Some((with_fm, _)) = run_md(doc_with_fm, "--code-theme nord --max-width 60") else {
        return;
    };

    assert!(
        with_fm.plain.contains("_cli_override_marker"),
        "fm-with-cli plain missing body:\n{}",
        with_fm.plain
    );

    // Nord panel background `#2e3440` = rgb(46,52,64). Dracula panel
    // background `#282a36` = rgb(40,42,54). WezTerm's `get-text --escapes`
    // re-emits SGR in either semicolon or ITU colon form and collapses
    // contiguous same-attribute cells into a single span, so per-line byte
    // equality is unreliable — assert on the presence of the nord SGR and
    // the absence of the dracula SGR in the full captured stream instead.
    let nord_bg_semi = "\x1b[48;2;46;52;64m";
    let nord_bg_colon = "\x1b[48:2::46:52:64m";
    let dracula_bg_semi = "\x1b[48;2;40;42;54m";
    let dracula_bg_colon = "\x1b[48:2::40:42:54m";

    assert!(
        with_fm.raw.contains(nord_bg_semi) || with_fm.raw.contains(nord_bg_colon),
        "expected nord panel bg (46,52,64) from CLI override. raw={:?}",
        with_fm.raw
    );
    assert!(
        !with_fm.raw.contains(dracula_bg_semi) && !with_fm.raw.contains(dracula_bg_colon),
        "frontmatter dracula panel bg (40,42,54) must not appear when CLI --code-theme \
         claims the slot. raw={:?}",
        with_fm.raw
    );

    // Nord's "frost" Blue `#81a1c1` = rgb(129,161,193) highlights the `fn`
    // and `let` keywords in our rust snippet; dracula's pink `#ff79c6` =
    // rgb(255,121,198) would color those instead. Asserting that the nord
    // keyword color is present and the dracula one is absent is a sharper
    // signal than panel bg alone (panel bg can match between themes that
    // share `#2e3440`, but nord/dracula have distinct keyword palettes).
    let nord_kw_semi = "\x1b[38;2;129;161;193m";
    let nord_kw_colon = "\x1b[38:2::129:161:193m";
    let dracula_kw_semi = "\x1b[38;2;255;121;198m";
    let dracula_kw_colon = "\x1b[38:2::255:121:198m";

    assert!(
        with_fm.raw.contains(nord_kw_semi) || with_fm.raw.contains(nord_kw_colon),
        "expected nord keyword fg (129,161,193) from CLI override. raw={:?}",
        with_fm.raw
    );
    assert!(
        !with_fm.raw.contains(dracula_kw_semi) && !with_fm.raw.contains(dracula_kw_colon),
        "frontmatter dracula keyword fg (255,121,198) must not appear when CLI --code-theme \
         claims the slot. raw={:?}",
        with_fm.raw
    );
}

/// `style.hyperlinks.color` + `style.hyperlinks.local-style.color` must
/// produce visibly different SGR streams between a local link and a remote
/// link in the same document.
#[test]
#[serial(level2_terminal)]
fn level2_local_hyperlink_color_differs_from_remote_in_terminal() {
    let body = "---\nstyle:\n  hyperlinks:\n    color: red-500\n    local-style:\n      color: blue-500\n---\n\n\
        [LOCAL_LINK](./somewhere.md) [REMOTE_LINK](https://example.com)\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    // WezTerm may re-emit truecolor SGR as either semicolon (`;`) or ITU
    // colon (`:`) form. Accept both.
    let red_semi = "38;2;251;44;54";
    let red_colon = "38:2::251:44:54";
    let blue_semi = "38;2;43;127;255";
    let blue_colon = "38:2::43:127:255";

    let has_red = frame.raw.contains(red_semi) || frame.raw.contains(red_colon);
    let has_blue = frame.raw.contains(blue_semi) || frame.raw.contains(blue_colon);
    assert!(
        has_red && has_blue,
        "expected both remote red and local blue SGR. has_red={has_red}, has_blue={has_blue}\nraw:\n{}",
        frame.raw
    );
    // Plain labels must still appear.
    assert!(
        frame.plain.contains("LOCAL_LINK") && frame.plain.contains("REMOTE_LINK"),
        "link labels missing from plain capture:\n{}",
        frame.plain
    );
}

/// `style.hyperlinks.width: 20` must produce a label box padded to that exact
/// width before the OSC8 close. We can't compare visible widths without a
/// stable column probe, so we assert the raw stream pads the label.
#[test]
#[serial(level2_terminal)]
fn level2_style_hyperlinks_width_pads_label_in_terminal() {
    let body = "---\nstyle:\n  hyperlinks:\n    width: 20\n---\n\n\
        [HI](https://example.com)\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    // Padded label width = 20 cells, label "HI" is 2 cells, so 18 trailing
    // spaces precede the OSC8 close. Look for the label followed by at least
    // 10 spaces and the OSC8 terminator. (Be tolerant of any ANSI bytes that
    // a terminal may inject for cursor positioning; the padding spaces are
    // the visible signal.)
    assert!(
        frame.plain.contains("HI                  "),
        "expected padded label `HI` followed by 18 spaces. plain:\n{}",
        frame.plain
    );
}

/// Regression (review-1, finding 1): an exact `style.hyperlinks.width` is an
/// exact field, so a label wider than the field must be truncated in a real
/// terminal — the visible field must not overflow the five columns.
#[test]
#[serial(level2_terminal)]
fn level2_style_hyperlinks_exact_width_truncates_label_in_terminal() {
    let body = "---\nstyle:\n  hyperlinks:\n    width: 5\n---\n\n\
        [A very long hyperlink label](https://example.com)\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    // The five-column field truncates with an ellipsis; the overflowing tail of
    // the label must be absent from the visible capture.
    assert!(
        frame.plain.contains('…'),
        "expected the long label truncated to an ellipsis. plain:\n{}",
        frame.plain
    );
    assert!(
        !frame.plain.contains("hyperlink label"),
        "the overflowing label tail must not appear in the visible field. plain:\n{}",
        frame.plain
    );
}

/// Regression (review-3): truncating a colored hyperlink label must keep its
/// closing SGR reset, so inline text following the truncated link does not
/// inherit the link's color in a real terminal.
#[test]
#[serial(level2_terminal)]
fn level2_style_hyperlinks_truncation_does_not_bleed_color_in_terminal() {
    // A red link with an exact 8-cell width truncates, immediately followed by
    // an unstyled trailing marker on the same line.
    let body = "---\nstyle:\n  hyperlinks:\n    color: red-500\n    width: 8\n---\n\n\
        [A very long hyperlink label](https://example.com) ZZTRAIL\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    let red_semi = "38;2;251;44;54";
    let red_colon = "38:2::251:44:54";
    assert!(
        frame.raw.contains(red_semi) || frame.raw.contains(red_colon),
        "expected the link's red foreground SGR in the capture. raw len={}",
        frame.raw.len()
    );

    // The trailing marker must not sit inside the link's red run: there must be
    // an SGR reset (or default-foreground) between the last red introduction and
    // the marker. WezTerm reconstructs SGR per cell, so a leaked color would
    // wrap the marker cells with red and no intervening reset.
    let trail_pos = frame
        .raw
        .find("ZZTRAIL")
        .unwrap_or_else(|| panic!("trailing marker missing in raw capture. plain:\n{}", frame.plain));
    let before = &frame.raw[..trail_pos];
    let red_idx = before
        .rfind(red_semi)
        .or_else(|| before.rfind(red_colon))
        .unwrap_or_else(|| {
            panic!("link's red SGR must precede the trailing marker. raw:\n{}", frame.raw)
        });
    let between = &before[red_idx..];
    assert!(
        between.contains("\x1b[0m")
            || between.contains("\x1b[m")
            || between.contains("\x1b[39m"),
        "trailing text inherits the truncated link color: no reset between the \
         red SGR and the marker. raw:\n{}",
        frame.raw
    );
}

/// `style.images.local-style.color` + `bg-color` must color a local image's
/// fallback alt text in a real terminal. Remote images must not pick this up.
#[test]
#[serial(level2_terminal)]
fn level2_style_images_local_style_colors_fallback_in_terminal() {
    let body = "---\nstyle:\n  images:\n    local-style:\n      color: red-500\n---\n\n\
        ![ALT_LOCAL](./no-such-image.png)\n\n![ALT_REMOTE](https://example.com/x.png)\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    let red_semi = "38;2;251;44;54";
    let red_colon = "38:2::251:44:54";
    let has_red = frame.raw.contains(red_semi) || frame.raw.contains(red_colon);
    assert!(
        has_red,
        "expected red foreground SGR for local image fallback. raw:\n{}",
        frame.raw
    );
    // The local fallback line carries the red bytes; the remote line must
    // not. We only check the raw stream for the presence of red SGR; the
    // remote line shouldn't add a second red occurrence.
    let red_hits = frame.raw.matches(red_semi).count() + frame.raw.matches(red_colon).count();
    assert!(
        (1..=4).contains(&red_hits),
        "unexpected red SGR hit count {red_hits} (heuristic). raw len={}",
        frame.raw.len()
    );
    assert!(
        frame.plain.contains("ALT_LOCAL") && frame.plain.contains("ALT_REMOTE"),
        "alt fallbacks missing from plain capture:\n{}",
        frame.plain
    );
}

/// `style.images.local-style.width: 40` + `alignment: right` must right-align
/// the *complete* fallback placeholder within 40 visible cells.
#[test]
#[serial(level2_terminal)]
fn level2_style_images_local_style_width_alignment_in_terminal() {
    let body = "---\nstyle:\n  images:\n    local-style:\n      width: 40\n      alignment: right\n---\n\n\
        ![A](./no-such-image.png)\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    // The tree path shapes the *complete* placeholder: `▉ IMAGE[A]` is
    // right-aligned within the 40-cell field, so the padding precedes the
    // placeholder and the alt inside the brackets is untouched.
    let fallback_line = frame
        .plain
        .lines()
        .find(|l| l.contains("▉ IMAGE["))
        .unwrap_or_else(|| panic!("fallback line missing in plain capture:\n{}", frame.plain));
    let inner = fallback_line
        .split_once("▉ IMAGE[")
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(inner, _)| inner)
        .unwrap_or("");
    assert_eq!(
        inner, "A",
        "alt inside the brackets must be untouched: {fallback_line:?}"
    );
    let leading_spaces = fallback_line.chars().take_while(|c| *c == ' ').count();
    let field_width = fallback_line.trim_end().chars().count();
    assert!(
        leading_spaces >= 28 && field_width == 40,
        "expected the complete placeholder right-aligned within 40 cells, got {leading_spaces} leading, width {field_width}: {fallback_line:?}"
    );
}

/// A long alt under an exact `width` must truncate the *complete* placeholder
/// to the field in a real terminal — the visible field must not overflow.
#[test]
#[serial(level2_terminal)]
fn level2_style_images_exact_width_truncates_long_alt_in_terminal() {
    let body = "---\nstyle:\n  images:\n    local-style:\n      width: 12\n---\n\n\
        ![A very long image alt text](./no-such-image.png)\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    // The exact 12-column field truncates with an ellipsis; the overflowing
    // tail of the alt must be absent and the visible placeholder must fill
    // exactly the field, framing included.
    let placeholder_line = frame
        .plain
        .lines()
        .find(|l| l.contains('…'))
        .unwrap_or_else(|| panic!("placeholder line missing in plain capture:\n{}", frame.plain));
    assert!(
        !placeholder_line.contains("image alt text"),
        "the overflowing alt tail must not appear in the visible field: {placeholder_line:?}"
    );
    assert_eq!(
        placeholder_line.trim_end().chars().count(),
        12,
        "the complete visible placeholder must fill exactly the 12-column field: {placeholder_line:?}"
    );
}

/// Regression (review-4): truncating a colored local-image placeholder must keep
/// its closing SGR reset, so inline text following the truncated image does not
/// inherit the image's color in a real terminal. Links and images use distinct
/// renderer branches, so the hyperlink color-bleed regression
/// (`level2_style_hyperlinks_truncation_does_not_bleed_color_in_terminal`) does
/// not cover the image placeholder's reset — this verifies it separately.
#[test]
#[serial(level2_terminal)]
fn level2_style_images_truncation_does_not_bleed_color_in_terminal() {
    // A red local-image placeholder with an exact 12-cell width truncates the
    // long alt, immediately followed by an unstyled trailing marker on the same
    // line.
    let body = "---\nstyle:\n  images:\n    local-style:\n      color: red-500\n      width: 12\n---\n\n\
        ![A very long image alt text](./no-such-image.png) ZZTRAIL\n";

    let Some((frame, _)) = run_md(body, "--max-width 60") else {
        return;
    };

    let red_semi = "38;2;251;44;54";
    let red_colon = "38:2::251:44:54";
    assert!(
        frame.raw.contains(red_semi) || frame.raw.contains(red_colon),
        "expected the local image's red foreground SGR in the capture. raw len={}",
        frame.raw.len()
    );

    // The trailing marker must not sit inside the image's red run: there must be
    // an SGR reset (or default-foreground) between the last red introduction and
    // the marker. WezTerm reconstructs SGR per cell, so a leaked color would
    // wrap the marker cells with red and no intervening reset.
    let trail_pos = frame
        .raw
        .find("ZZTRAIL")
        .unwrap_or_else(|| panic!("trailing marker missing in raw capture. plain:\n{}", frame.plain));
    let before = &frame.raw[..trail_pos];
    let red_idx = before
        .rfind(red_semi)
        .or_else(|| before.rfind(red_colon))
        .unwrap_or_else(|| {
            panic!("image's red SGR must precede the trailing marker. raw:\n{}", frame.raw)
        });
    let between = &before[red_idx..];
    assert!(
        between.contains("\x1b[0m")
            || between.contains("\x1b[m")
            || between.contains("\x1b[39m"),
        "trailing text inherits the truncated image color: no reset between the \
         red SGR and the marker. raw:\n{}",
        frame.raw
    );
}
