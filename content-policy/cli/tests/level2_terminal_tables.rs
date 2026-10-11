//! The styled `policy check` report and `policy renew` preview in a real
//! terminal (tmux, 80 columns).
//!
//! Level 1 (`check.rs`, `renew.rs`, `file_changed.rs`) pins the bytes: JSON
//! fields, exact text, ANSI presence, and a table line's `char` count. Only a
//! terminal shows what those bytes become, so these tests read the rendered
//! pane instead: every table row keeps its right border (a row the terminal
//! wrapped loses it), every row is as wide in display cells as the top border
//! (a double-width character counts twice), a long rule and fingerprint
//! survive wrapping, and the status word carries its color.
//!
//! Each test owns its tmux session, so nothing here touches a shared pane or
//! the user's windows.

mod common;

use std::path::Path;
use std::time::{Duration, Instant};

use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use common::{LIFECYCLE, Workspace};
use test_toolkit::{Backend, Level, require_level};
use unicode_width::UnicodeWidthStr;

const PANE_COLS: u32 = 80;
const PANE_ROWS: u32 = 60;
const RUN_DEADLINE: Duration = Duration::from_secs(15);
const EXIT_MARKER: &str = "policy-exit-code:";

/// One `policy` invocation's captured pane.
struct Rendered {
    frame: CapturedFrame,
    exit_code: i32,
}

struct Pane {
    harness: TmuxHarness,
}

impl Pane {
    fn open() -> Self {
        let mut harness = TmuxHarness::new();
        harness.spawn_shell().expect("spawn tmux shell");
        // A detached session keeps its creation size until resized; the
        // tables must be judged at a width narrow enough to force wrapping.
        harness.resize(PANE_COLS, PANE_ROWS).expect("resize pane");
        assert_eq!(harness.pane_cols().expect("pane width"), PANE_COLS);
        Self { harness }
    }

    /// Runs `policy <args>` in `dir` on a cleared screen and returns the pane
    /// once the shell has printed the exit marker.
    fn run(&mut self, dir: &Path, args: &str) -> Rendered {
        let policy = biscuit_test_harness::bin_exe!("policy");
        let line = format!(
            "clear; cd '{}' && '{}' {args}; echo \"{EXIT_MARKER}$?\"\n",
            dir.display(),
            policy.display()
        );
        self.harness.send_text(line.as_bytes()).expect("send command");

        let deadline = Instant::now() + RUN_DEADLINE;
        loop {
            let frame = self.harness.capture().expect("capture pane");
            if let Some(exit_code) = exit_code(&frame.plain) {
                return Rendered { frame, exit_code };
            }
            assert!(
                Instant::now() < deadline,
                "policy did not finish within {RUN_DEADLINE:?}; pane:\n{}",
                frame.plain
            );
            std::thread::sleep(Duration::from_millis(40));
        }
    }
}

/// The code the shell echoed after the command. The typed command line holds
/// `$?` where the finished one holds a number, so only a finished run matches.
fn exit_code(plain: &str) -> Option<i32> {
    plain.lines().find_map(|line| {
        line.trim_end()
            .strip_prefix(EXIT_MARKER)
            .and_then(|code| code.parse().ok())
    })
}

const TABLE_TOP: char = '┌';
const TABLE_GLYPHS: [char; 4] = ['┌', '│', '├', '└'];
const TABLE_RIGHT_EDGES: [char; 4] = ['┐', '│', '┤', '┘'];

/// The pane's table, top border through bottom border.
fn table_lines(plain: &str) -> Vec<&str> {
    let mut table = Vec::new();
    let lines = plain
        .lines()
        .map(str::trim_end)
        .skip_while(|line| !line.starts_with(TABLE_TOP));
    for line in lines {
        if !line.starts_with(TABLE_GLYPHS) {
            break;
        }
        table.push(line);
        if line.starts_with('└') {
            break;
        }
    }
    table
}

/// Every row closes on its own line and is exactly as wide, in display cells,
/// as the top border, which must itself fit the pane.
#[track_caller]
fn assert_table_intact(plain: &str) -> Vec<&str> {
    let table = table_lines(plain);
    assert!(table.len() > 3, "no table in the pane:\n{plain}");
    let width = table[0].width();
    assert!(
        width <= PANE_COLS as usize,
        "the table is {width} cells wide in a {PANE_COLS}-column pane:\n{plain}"
    );
    for line in &table {
        assert!(
            line.ends_with(TABLE_RIGHT_EDGES),
            "a row lost its right border, so the terminal wrapped it: {line:?}\n{plain}"
        );
        assert_eq!(line.width(), width, "ragged row {line:?}\n{plain}");
    }
    assert!(
        table.last().is_some_and(|line| line.starts_with('└')),
        "the table has no bottom border:\n{plain}"
    );
    table
}

/// The table's body rows, each column's wrapped fragments joined without the
/// spaces the wrapping consumed. A row starts where `start_column` has text.
fn body_rows(table: &[&str], start_column: usize) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut in_body = false;
    for line in table {
        if line.starts_with('├') {
            in_body = true;
            continue;
        }
        if !in_body || !line.starts_with('│') {
            continue;
        }
        let cells: Vec<String> = line
            .trim_matches('│')
            .split('│')
            .map(|cell| cell.split_whitespace().collect::<String>())
            .collect();
        match rows.last_mut() {
            Some(row) if cells[start_column].is_empty() => {
                for (joined, fragment) in row.iter_mut().zip(cells) {
                    joined.push_str(&fragment);
                }
            }
            _ => rows.push(cells),
        }
    }
    rows
}

/// The SGR parameters of the last styling sequence before the first
/// occurrence of `needle`: what the terminal applied to that text.
#[track_caller]
fn sgr_before(raw: &str, needle: &str) -> Vec<u32> {
    let at = raw
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} is not in the pane:\n{raw:?}"));
    let prefix = &raw[..at];
    let mut params = None;
    for (start, _) in prefix.match_indices("\x1b[") {
        let rest = &prefix[start + 2..];
        if let Some(end) = rest.find(|c: char| c.is_ascii_alphabetic())
            && rest[end..].starts_with('m')
        {
            params = Some(rest[..end].to_string());
        }
    }
    let params = params.unwrap_or_else(|| panic!("no styling before {needle:?}:\n{raw:?}"));
    params
        .split([';', ':'])
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

const SGR_BOLD: u32 = 1;
const SGR_DIM: u32 = 2;
const SGR_RED: u32 = 31;
const SGR_GREEN: u32 = 32;
const SGR_YELLOW: u32 = 33;
const SGR_MAGENTA: u32 = 35;

const WIDE_RULE_PATH: &str = "src/設定/読み込み/configuration_loader_implementation.rs";
const OLD_FINGERPRINT: &str =
    "blake3-lf:aaaaaaaa00000000000000000000000000000000000000000000000000000000";

fn file_rule_document() -> String {
    format!(
        "---\ntitle: Terminal\nlast_updated: 2026-01-01\nconfig_fingerprint: {OLD_FINGERPRINT}\n\
         content_policy:\n  - ValidFor(3mo, @last_updated)\n  - rule: FileChanged({WIDE_RULE_PATH}, @config_fingerprint)\n    action: refresh\n---\n\nBody.\n"
    )
}

fn file_rule_workspace() -> Workspace {
    let workspace = Workspace::new();
    let source = workspace.path().join(WIDE_RULE_PATH);
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(&source, "pub fn load() {}\n").unwrap();
    workspace.write("doc.md", file_rule_document());
    workspace
}

#[test]
fn level2_check_report_wraps_a_long_wide_character_rule_inside_the_pane() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let workspace = file_rule_workspace();
    let mut pane = Pane::open();

    let Rendered { frame, exit_code } =
        pane.run(workspace.path(), "check --at 2026-12-28 doc.md");

    assert_eq!(exit_code, 0, "pane:\n{}", frame.plain);
    let summary = frame.plain.lines().find(|line| line.starts_with("doc.md:")).unwrap();
    assert!(summary.starts_with("doc.md: stale, action refresh"), "{summary}");
    let table = assert_table_intact(&frame.plain);
    let rows = body_rows(&table, 0);
    assert_eq!(rows.len(), 2, "one row per rule:\n{}", frame.plain);
    // The wrapped rule loses no character; only the spaces it broke at.
    assert_eq!(
        rows[1][1],
        format!("FileChanged({WIDE_RULE_PATH},@config_fingerprint)"),
        "{}",
        frame.plain
    );
    assert_eq!(rows[1][3], "triggered");
    // The evidence cell shows the stored fingerprint cut to 8 hex digits.
    assert_eq!(rows[1][4], "blake3-lf:aaaaaaaa…", "{}", frame.plain);
    assert!(table.len() > 6, "the long rule should wrap onto several lines");
}

#[test]
fn level2_check_report_styles_each_status_word() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let workspace = Workspace::new();
    workspace.write("lifecycle.md", LIFECYCLE);
    workspace.write("no-baseline.md", "---\nlast_updated:\ncontent_policy:\n  - ValidFor(3mo)\n---\n");
    let mut pane = Pane::open();

    for (args, status, color) in [
        ("check --at 2026-12-27 lifecycle.md", "fresh", SGR_GREEN),
        ("check --at 2026-12-28 lifecycle.md", "stale", SGR_YELLOW),
        ("check --at 2027-01-01 lifecycle.md", "expired", SGR_RED),
        ("check --at 2027-01-01 no-baseline.md", "unknown", SGR_MAGENTA),
    ] {
        let Rendered { frame, exit_code } = pane.run(workspace.path(), args);

        assert_eq!(exit_code, 0, "{args}; pane:\n{}", frame.plain);
        assert!(
            frame.plain.lines().any(|line| line.contains(&format!(": {status}, "))),
            "{args}: no {status} summary in:\n{}",
            frame.plain
        );
        assert_table_intact(&frame.plain);
        let status_style = sgr_before(&frame.raw, status);
        assert!(
            status_style.contains(&color),
            "{args}: {status} should carry SGR {color}, got {status_style:?}"
        );
        let document_style = sgr_before(&frame.raw, ".md");
        assert!(
            document_style.contains(&SGR_BOLD),
            "{args}: the document name should be bold, got {document_style:?}"
        );
    }
}

#[test]
fn level2_renew_preview_wraps_a_long_fingerprint_inside_the_pane() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let workspace = file_rule_workspace();
    let mut pane = Pane::open();

    let Rendered { frame, exit_code } =
        pane.run(workspace.path(), "renew --today 2026-09-28 doc.md");

    assert_eq!(exit_code, 0, "pane:\n{}", frame.plain);
    let summary = frame.plain.lines().find(|line| line.starts_with("doc.md:")).unwrap();
    assert_eq!(
        summary.trim_end(),
        "doc.md: renewal on 2026-09-28 (preview; pass --write to apply)"
    );
    let table = assert_table_intact(&frame.plain);
    let rows = body_rows(&table, 1);
    assert_eq!(rows.len(), 2, "one row per baseline:\n{}", frame.plain);

    let date = rows.iter().find(|row| row[0] == "last_updated").expect("date row");
    assert_eq!(&date[2..], ["renewed", "2026-01-01", "2026-09-28"], "{}", frame.plain);

    let fingerprint = rows.iter().find(|row| row[0] == "config_fingerprint").expect("fingerprint row");
    assert_eq!(fingerprint[2], "renewed");
    assert_eq!(fingerprint[3], "blake3-lf:aaaaaaaa…", "{}", frame.plain);
    assert!(
        fingerprint[4].starts_with("blake3-lf:") && fingerprint[4].ends_with('…'),
        "{}",
        frame.plain
    );
    assert_ne!(fingerprint[3], fingerprint[4], "the fingerprint should change");
}

#[test]
fn level2_renew_preview_dims_the_preview_note_and_bolds_the_date() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let workspace = Workspace::new();
    workspace.write("lifecycle.md", LIFECYCLE);
    let mut pane = Pane::open();

    let preview = pane.run(workspace.path(), "renew --today 2026-12-29 lifecycle.md");

    assert_eq!(preview.exit_code, 0, "pane:\n{}", preview.frame.plain);
    assert_table_intact(&preview.frame.plain);
    let raw = &preview.frame.raw;
    assert!(sgr_before(raw, "preview;").contains(&SGR_DIM), "{raw:?}");
    assert!(sgr_before(raw, "2026-12-29").contains(&SGR_BOLD), "{raw:?}");

    let written = pane.run(workspace.path(), "renew --write --today 2026-12-29 lifecycle.md");

    assert_eq!(written.exit_code, 0, "pane:\n{}", written.frame.plain);
    assert!(written.frame.plain.contains("(written)"), "{}", written.frame.plain);
    assert!(
        sgr_before(&written.frame.raw, "written").contains(&SGR_GREEN),
        "{:?}",
        written.frame.raw
    );
}

// -- the assertions themselves: hermetic, so they run in Level 1 ------------

fn synthetic_table(body_row: &str) -> String {
    format!("┌────┬────┐\n│ A  │ B  │\n├────┼────┤\n{body_row}\n└────┴────┘\n")
}

#[test]
fn an_intact_table_passes_the_terminal_checks() {
    assert_table_intact(&synthetic_table("│ 1  │ 2  │"));
}

#[test]
fn double_width_cells_are_measured_in_cells_not_chars() {
    // One char fewer than the top border, and the same eleven cells wide.
    assert_table_intact(&synthetic_table("│ 設 │ 2  │"));
}

#[test]
#[should_panic(expected = "lost its right border")]
fn a_row_the_terminal_wrapped_is_rejected() {
    assert_table_intact(&synthetic_table("│ 1  │ 2\n  │"));
}

#[test]
#[should_panic(expected = "ragged row")]
fn a_row_of_the_wrong_display_width_is_rejected() {
    assert_table_intact(&synthetic_table("│ 設定設 │ 2  │"));
}

#[test]
fn wrapped_fragments_rejoin_into_their_row() {
    let plain = "┌───┬──────┐\n│ # │ Rule │\n├───┼──────┤\n│ 1 │ Vali │\n│   │ dFor │\n│ 2 │ Ever │\n└───┴──────┘\n";
    let rows = body_rows(&table_lines(plain), 0);
    assert_eq!(rows, [["1", "ValidFor"], ["2", "Ever"]]);
}

#[test]
fn the_style_before_a_word_is_the_last_sequence_that_precedes_it() {
    let raw = "\x1b[1mdoc.md\x1b[0m: \x1b[0;33mstale\x1b[39m, action";
    assert_eq!(sgr_before(raw, "stale"), [0, 33]);
    assert_eq!(sgr_before(raw, "doc"), [1]);
}
