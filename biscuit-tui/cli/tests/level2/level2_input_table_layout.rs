//! Level 2: `question input-table` static-column layout in a real terminal.
//!
//! `InputTable` sizes a static column from its widest value across every row,
//! including rows scrolled out of view, and clips a value that does not fit
//! with `…` without splitting a grapheme. The L1 tests prove that in a
//! headless buffer; this runs the shipped `question` binary in a detached tmux
//! pane, so the emulator's own glyph widths, redraw on resize, and scrolling
//! decide what is on screen.
//!
//! The fixture's labels include a wide (CJK) value, a combining accent, and a
//! joined emoji, each followed by a short tag column (`#a`, `#b`, …) that must
//! stay intact beside it. Tag alignment is asserted in display cells for
//! every row whose width every terminal agrees on; the joined emoji's width
//! is the emulator's choice, so for that row the tag is asserted intact rather
//! than at a column.
//!
//! tmux sessions are detached, so no window opens or takes focus.
//!
//! Run via the canonical recipe: `just test-l2 input_table_layout`.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use biscuit_test_harness::layout_invariants::visible_width;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serde_json::Value;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

/// `(label, tag)` per row, in row order.
const ROWS: [(&str, &str); 8] = [
    ("12 review-5", "#a"),
    ("日本語のラベル名前", "#b"),
    ("Cafe\u{301} cre\u{300}me bru\u{302}le\u{301}e", "#c"),
    ("family \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467} joined", "#d"),
    ("row-five", "#e"),
    ("row-six", "#f"),
    ("row-seven", "#g"),
    ("99 the-longest-row-label-kept-off-screen", "#h"),
];

/// The row whose display width is the emulator's choice.
const EMOJI_TAG: &str = "#d";

/// The geometry the harness spawns tmux panes at, restored after the test.
const FULL: (u32, u32) = (120, 40);

const TIMEOUT: Duration = Duration::from_secs(15);

fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Writes a launcher that runs `question input-table` with its JSON result
/// redirected to `out`, then prints `<marker>:<status>`.
fn write_launcher(dir: &Path, out: &Path, marker: &str) -> String {
    let columns = r#"[{"type":"static-text","id":"label","text":"Label"},{"type":"static-text","id":"tag","text":"Tag"},{"type":"text-input","id":"note"}]"#;
    let rows = Value::Array(
        ROWS.iter()
            .map(|(label, tag)| serde_json::json!([label, tag, "n"]))
            .collect(),
    )
    .to_string();
    let question = biscuit_test_harness::bin_exe!("question");
    let script = format!(
        "#!/bin/sh\nprintf '\\033[2J\\033[H'\n\
         {question} --output json input-table --columns {columns} --rows {rows} > {out}\n\
         printf '\\n{marker}:%s\\n' \"$?\"\n",
        question = sh_quote(&question.display().to_string()),
        columns = sh_quote(columns),
        rows = sh_quote(&rows),
        out = sh_quote(&out.display().to_string()),
    );
    let launcher = dir.join("launch.sh");
    fs::write(&launcher, script).expect("write launcher");
    format!("/bin/sh {}\n", sh_quote(&launcher.display().to_string()))
}

fn wait_until(
    harness: &mut TmuxHarness,
    what: &str,
    ready: impl Fn(&CapturedFrame) -> bool,
) -> CapturedFrame {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let frame = harness.capture().expect("capture pane");
        if ready(&frame) {
            return frame;
        }
        assert!(Instant::now() < deadline, "{what} never rendered; plain:\n{}", frame.plain);
        harness.settle();
    }
}

/// The screen line holding `tag`.
fn line_with<'a>(plain: &'a str, tag: &str) -> &'a str {
    plain
        .lines()
        .find(|line| line.contains(&format!("{tag} n")))
        .unwrap_or_else(|| panic!("the {tag} row is not on screen intact:\n{plain}"))
}

/// The display column of `tag` on its line.
fn tag_column(plain: &str, tag: &str) -> usize {
    let line = line_with(plain, tag);
    visible_width(&line[..line.find(tag).expect("tag")])
}

/// Every on-screen row except the emoji row, with its tag column.
fn measured_columns(plain: &str) -> Vec<(&'static str, usize)> {
    ROWS.iter()
        .filter(|(_, tag)| *tag != EMOJI_TAG && plain.contains(&format!("{tag} n")))
        .map(|(_, tag)| (*tag, tag_column(plain, tag)))
        .collect()
}

fn assert_aligned(plain: &str, expected: usize, what: &str) {
    for (tag, column) in measured_columns(plain) {
        assert_eq!(column, expected, "{what}: the {tag} column is misplaced:\n{plain}");
    }
}

/// Normal, short, narrow, scrolled, and restored layouts, then submission
/// returns every label unclipped.
#[test]
#[serial(level2)]
fn level2_tmux_input_table_static_columns_fit_clip_scroll_and_restore() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let marker = format!("INPUT_TABLE_{}", std::process::id());
    let workspace = std::env::temp_dir().join(marker.to_lowercase());
    fs::create_dir_all(&workspace).expect("workspace");
    let out = workspace.join("out.json");
    let launch = write_launcher(&workspace, &out, &marker);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    harness.resize(FULL.0, FULL.1).expect("resize pane");
    harness.send_text(launch.as_bytes()).expect("launch question");

    // Normal: every label whole, every tag in one column just past the longest
    // label.
    let normal = wait_until(&mut harness, "the table", |frame| frame.plain.contains("#h n"));
    for (label, tag) in ROWS {
        assert!(
            line_with(&normal.plain, tag).contains(label),
            "{label:?} must be drawn whole:\n{}",
            normal.plain
        );
    }
    let longest = ROWS[7];
    let full_column = tag_column(&normal.plain, longest.1);
    let longest_line = line_with(&normal.plain, longest.1);
    let label_start = visible_width(&longest_line[..longest_line.find(longest.0).expect("label")]);
    assert_eq!(
        full_column,
        label_start + longest.0.len(),
        "the tag column must start right after the longest label:\n{}",
        normal.plain
    );
    assert_aligned(&normal.plain, full_column, "normal width");

    // Short: the longest row is scrolled out of view but still sizes the
    // column.
    harness.resize(FULL.0, 5).expect("resize short");
    let short = wait_until(&mut harness, "the short table", |frame| {
        frame.plain.contains('▼') && !frame.plain.contains("#h n")
    });
    assert_aligned(&short.plain, full_column, "an off-screen row must keep sizing the column");

    // Narrow: labels clip with `…` at a grapheme boundary and the tag column
    // follows each one intact, inside the pane.
    harness.resize(34, 6).expect("resize narrow");
    let narrow = wait_until(&mut harness, "the narrow table", |frame| {
        frame.plain.contains("12 revie…")
    });
    for line in narrow.plain.lines() {
        assert!(visible_width(line) <= 34, "a line overflows the 34-column pane: {line:?}");
    }
    let narrow_column = tag_column(&narrow.plain, "#a");
    assert!(narrow_column < full_column, "the label column must shrink:\n{}", narrow.plain);
    for (tag, prefix) in [("#a", "12 revie…"), ("#b", "日本語の…"), ("#c", "Cafe\u{301} cre\u{300}…")] {
        assert!(
            line_with(&narrow.plain, tag).contains(&format!("{prefix}{tag}")),
            "the {tag} label must clip to {prefix:?} right before its tag:\n{}",
            narrow.plain
        );
    }
    // The joined emoji is one grapheme: it is dropped whole, never split.
    let emoji_line = line_with(&narrow.plain, EMOJI_TAG);
    assert!(
        emoji_line.contains("family …") && !emoji_line.contains('\u{200D}'),
        "the joined emoji must be clipped whole: {emoji_line:?}"
    );
    assert_aligned(&narrow.plain, narrow_column, "narrow width");
    assert_eq!(tag_column(&narrow.plain, EMOJI_TAG), narrow_column, "narrow emoji row:\n{}", narrow.plain);

    // Scroll to the last row: clipped, at the same column.
    for _ in 0..7 {
        harness.send_key("Down").expect("press Down");
        harness.settle();
    }
    let scrolled = wait_until(&mut harness, "the scrolled table", |frame| {
        frame.plain.contains('▲') && frame.plain.contains("#h n")
    });
    assert!(
        line_with(&scrolled.plain, "#h").contains("99 the-l…#h"),
        "the longest label must clip beside its tag:\n{}",
        scrolled.plain
    );
    assert_aligned(&scrolled.plain, narrow_column, "scrolling must not change widths");

    // Restore: whole labels at the original column.
    harness.resize(FULL.0, FULL.1).expect("restore pane");
    let restored = wait_until(&mut harness, "the restored table", |frame| {
        frame.plain.contains(longest.0)
    });
    assert_aligned(&restored.plain, full_column, "restored width");

    // Submit: clipping was presentation only.
    harness.send_key("C-s").expect("press Ctrl+S");
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        let frame = harness.capture().expect("capture pane");
        if let Some(status) = frame
            .plain
            .lines()
            .find_map(|line| line.trim().strip_prefix(&format!("{marker}:")))
        {
            break status.trim().to_string();
        }
        assert!(Instant::now() < deadline, "question never exited:\n{}", frame.plain);
        std::thread::sleep(Duration::from_millis(50));
    };
    harness.resize(FULL.0, FULL.1).expect("restore pane");
    assert_eq!(status, "0", "Ctrl+S must submit");
    let submitted: Value =
        serde_json::from_str(&fs::read_to_string(&out).expect("read output")).expect("JSON output");
    let labels: Vec<&str> = submitted
        .as_array()
        .expect("row array")
        .iter()
        .map(|row| row["label"].as_str().expect("label"))
        .collect();
    let expected: Vec<&str> = ROWS.iter().map(|(label, _)| *label).collect();
    assert_eq!(labels, expected, "submitted labels must be the full originals");
    let _ = fs::remove_dir_all(&workspace);
}
