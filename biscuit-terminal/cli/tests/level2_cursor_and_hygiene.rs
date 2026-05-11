//! Level-2 tests for cursor placement, ANSI hygiene, and unicode width handling.
//!
//! These tests run the `bt` CLI inside a real terminal emulator to validate
//! that cursor positioning after image renders is correct, that save/restore
//! sequences are balanced, and that CJK/emoji filenames align correctly in
//! directory listings.
//!
//! ## Skip-clean contract
//!
//! Every test checks `harness.available()` before spawning. When the
//! required terminal is absent the test prints `skipping: requires <X>`
//! to stderr and returns immediately. No `#[ignore]` markers are used.

mod common;

use biscuit_test_harness::{TerminalHarness, skip_with_reason};
use common::pane_geometry::{find_row_of, parse_debug_cursor_before, parse_debug_cursor_rows};
use common::send_bt_command;
use serial_test::serial;
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

/// Extra settle time after spawning a WezTerm shell to avoid racing
/// shell initialization (custom prompts, completions, etc.).
const SHELL_READY_MS: u64 = 1500;

/// Returns the absolute path to a fixture file.
fn fixture_path(name: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/tests/fixtures/{}", manifest_dir, name)
}

/// Positions the cursor at a known row using `tput` so that image
/// rendering starts from a predictable position.
fn position_cursor(harness: &mut impl TerminalHarness, row: u32) {
    let cmd = format!("tput cup {} 0\n", row);
    harness.send_text(cmd.as_bytes()).expect("tput failed");
    harness.settle();
}

// ------------------------------------------------------------------
// Cursor placement
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_cursor_lands_below_rendered_image() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));

    // Clear the screen and position the cursor high in the pane so the
    // image renders without triggering scroll-margin compensation. The
    // scroll-margin path is exercised by
    // `level2_image_scroll_compensation_at_bottom_margin`.
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    position_cursor(&mut harness, 5);

    // Force a tiny render width so the image occupies only ~1 row —
    // this keeps the entire bt invocation + image + debug block inside
    // a single pane height and prevents post-image scrolling from
    // shifting absolute row indices.
    let path = fixture_path("tiny.png");
    send_bt_command(&mut harness, &format!("image --debug --width 2 {path}"));

    // Append a sentinel that must land *below* the rendered image area.
    harness
        .send_text(b"printf 'SENTINEL_BELOW_IMAGE\\n'\n")
        .expect("send_text failed");
    harness.settle();
    std::thread::sleep(Duration::from_millis(300));

    let frame = harness.capture().expect("capture failed");
    let plain = &frame.plain;

    // Parse the renderer's reported geometry from `--debug` output.
    // `cursor BEFORE: row=R col=C` is the 1-based pane row at which the
    // image started rendering. `cursor rows: N (used for CUD)` is the
    // number of rows the renderer advanced past the image (ceil for most
    // terminals, floor for Warp). The contract under test:
    //
    //     row(sentinel) >= row(image_top) + image_rows
    //
    // expressed in capture-relative indices.
    let (cursor_before_row, _col) = parse_debug_cursor_before(plain).unwrap_or_else(|| {
        panic!("could not parse `cursor BEFORE: row=...` from debug output. plain:\n{plain}")
    });
    let image_rows = parse_debug_cursor_rows(plain).unwrap_or_else(|| {
        panic!("could not parse `cursor rows: N (used for CUD)` from debug output. plain:\n{plain}")
    });
    assert!(
        image_rows >= 1,
        "renderer must report at least one image row; got {image_rows}. plain:\n{plain}",
    );

    // Locate the bt invocation row in the *captured* pane. This is our
    // capture-relative anchor for `R` (the row at which bt began
    // rendering). `cursor BEFORE` reports R+1 in absolute pane terms
    // because it is queried after the command-line newline; the image
    // therefore begins on the line directly below the bt invocation.
    let bt_invocation_row = find_row_of(plain, "bt image --debug").unwrap_or_else(|| {
        panic!("could not locate `bt image --debug` invocation row. plain:\n{plain}")
    });
    let image_top_row = bt_invocation_row + 1;
    let expected_min_row = image_top_row + image_rows as usize;

    let sentinel_row = find_row_of(plain, "SENTINEL_BELOW_IMAGE").unwrap_or_else(|| {
        panic!(
            "SENTINEL_BELOW_IMAGE not found in pane after image render.\n\
             cursor_before_row={cursor_before_row} image_rows={image_rows}\n\
             bt_invocation_row={bt_invocation_row} image_top_row={image_top_row}\n\
             plain:\n{plain}"
        )
    });

    // Primary assertion: the sentinel sits at or below the row
    // immediately following the rendered image. The `--- image debug ---`
    // block (~15 rows) is printed BETWEEN the image and the sentinel
    // because debug output is emitted after rendering, so the upper
    // bound is wide.
    assert!(
        sentinel_row >= expected_min_row,
        "sentinel row {sentinel_row} must be >= expected_min_row {expected_min_row} \
         (image_top_row={image_top_row} + image_rows={image_rows}). \
         A row below this bound means the sentinel overlaps the image area.\n\
         cursor_before_row={cursor_before_row}\n\
         plain:\n{plain}",
    );

    // Negative-control assertion: the sentinel must NOT appear inside
    // the image-occupied row range [image_top_row, image_top_row +
    // image_rows). Catches the off-by-one regression flagged by the
    // reviewer (sentinel landing on the last image row).
    let image_end_exclusive = image_top_row + image_rows as usize;
    let lines: Vec<&str> = plain.lines().collect();
    let upper = image_end_exclusive.min(lines.len());
    for (idx, line) in lines.iter().enumerate().take(upper).skip(image_top_row) {
        assert!(
            !line.contains("SENTINEL_BELOW_IMAGE"),
            "sentinel must not appear inside the image-occupied row range \
             [{image_top_row}, {image_end_exclusive}); found at row {idx}. \
             image_rows={image_rows}\n\
             plain:\n{plain}",
        );
    }
}

// ------------------------------------------------------------------
// ANSI hygiene — balanced save/restore
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_no_orphan_save_restore_sequences() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));

    // Test with image rendering (known to use save/restore).
    let path = fixture_path("tiny.png");
    send_bt_command(&mut harness, &format!("image --debug {}", path));

    let frame = harness.capture().expect("capture failed");
    assert_balanced_save_restore(&frame.raw);

    // Test with a prose command (should not produce orphans either).
    let mut harness2 = WezTermHarness::new();
    harness2.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));
    send_bt_command(&mut harness2, "prose \"<red>hello</red>\"");

    let frame2 = harness2.capture().expect("capture failed");
    assert_balanced_save_restore(&frame2.raw);

    // Test with two-column layout (uses save/restore internally).
    let mut harness3 = WezTermHarness::new();
    harness3.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));
    send_bt_command(&mut harness3, "columns \"left\" \"right\"");

    let frame3 = harness3.capture().expect("capture failed");
    assert_balanced_save_restore(&frame3.raw);
}

// ------------------------------------------------------------------
// Unicode width alignment in dir command
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_dir_command_unicode_widths_in_capture() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));

    let dir_path = fixture_path("unicode_dir");
    send_bt_command(&mut harness, &format!("dir {dir_path}"));

    let frame = harness.capture().expect("capture failed");
    let plain = &frame.plain;

    // Verify that the fixture filenames appear in the output.
    assert!(
        plain.contains("中文文件.txt"),
        "expected CJK filename in dir output. plain:\n{plain}",
    );
    assert!(
        plain.contains("emoji_🎉.txt"),
        "expected emoji filename in dir output. plain:\n{plain}",
    );
    assert!(
        plain.contains("regular.txt"),
        "expected regular filename in dir output. plain:\n{plain}",
    );

    // Verify that tree-branch characters appear (indicating the FileSystem
    // component rendered a tree structure).
    assert!(
        plain.contains("├──") || plain.contains("└──") || plain.contains("│"),
        "expected tree branch characters in dir output. plain:\n{plain}",
    );

    // ----------------------------------------------------------------
    // Geometric column-alignment assertion
    // ----------------------------------------------------------------
    //
    // The tree renderer prefixes each filename row with `├── ` or
    // `└── ` followed by an icon (single-cell glyph) and a space, then
    // the filename. The filename column index — measured in display
    // cells via unicode-width — must be identical across all three
    // fixture rows regardless of the filename's own glyph mix
    // (ASCII / CJK / emoji).

    let regular_row = find_row_of(plain, "regular.txt")
        .unwrap_or_else(|| panic!("could not locate regular.txt row.\nplain:\n{plain}"));
    let cjk_row = find_row_of(plain, "中文文件.txt")
        .unwrap_or_else(|| panic!("could not locate CJK filename row.\nplain:\n{plain}"));
    let emoji_row = find_row_of(plain, "emoji_🎉.txt")
        .unwrap_or_else(|| panic!("could not locate emoji filename row.\nplain:\n{plain}"));

    let lines: Vec<&str> = plain.lines().collect();
    let regular_line = lines[regular_row];
    let cjk_line = lines[cjk_row];
    let emoji_line = lines[emoji_row];

    // Each row must use one of the documented tree-glyph prefixes.
    for (label, line) in [
        ("regular", regular_line),
        ("cjk", cjk_line),
        ("emoji", emoji_line),
    ] {
        assert!(
            line.contains("├──") || line.contains("└──") || line.contains("│"),
            "expected tree glyph prefix on {label} row.\nrow: {line:?}",
        );
    }

    // Compute the *display column* at which the filename starts on each
    // row. We locate the byte offset of the filename, take the prefix
    // before it, and pass that prefix through `UnicodeWidthStr::width` to
    // convert chars (some of which may be wide) into display cells.
    let filename_start_col = |line: &str, name: &str| -> usize {
        let byte_idx = line
            .find(name)
            .unwrap_or_else(|| panic!("name {name:?} not in line {line:?}"));
        let prefix = &line[..byte_idx];
        UnicodeWidthStr::width(prefix)
    };

    let regular_col = filename_start_col(regular_line, "regular.txt");
    let cjk_col = filename_start_col(cjk_line, "中文文件.txt");
    let emoji_col = filename_start_col(emoji_line, "emoji_🎉.txt");

    assert_eq!(
        regular_col, cjk_col,
        "filename column misaligned between regular and CJK rows: \
         regular_col={regular_col}, cjk_col={cjk_col}.\n\
         regular: {regular_line:?}\ncjk:     {cjk_line:?}",
    );
    assert_eq!(
        regular_col, emoji_col,
        "filename column misaligned between regular and emoji rows: \
         regular_col={regular_col}, emoji_col={emoji_col}.\n\
         regular: {regular_line:?}\nemoji:   {emoji_line:?}",
    );
}

// ------------------------------------------------------------------
// Assertion helpers
// ------------------------------------------------------------------

/// Asserts that `raw` contains balanced `\x1b[s` (save) and `\x1b[u` (restore)
/// sequences. Every save must have a matching restore.
fn assert_balanced_save_restore(raw: &str) {
    let saves = raw.matches("\x1b[s").count();
    let restores = raw.matches("\x1b[u").count();
    assert_eq!(
        saves, restores,
        "expected balanced save/restore sequences: {} saves vs {} restores. raw:\n{}",
        saves, restores, raw
    );
}
