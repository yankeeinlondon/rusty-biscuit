//! The reference report as a real terminal displays it: its groups, styling,
//! columns, and narrow layout, on each route that prints it to a terminal.
//!
//! Each tmux test owns its session, because the narrow tests resize their pane
//! and a pane a broker shares would keep that geometry. tmux's capture does not
//! carry hyperlinks, so the one link check runs on WezTerm.

use crate::common;

use biscuit_test_harness::{TerminalHarness, shell_path};
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use common::level2::{SHARED_HARNESS, md_shim_word, run_with_sentinel};
use std::fs;
use std::time::{Duration, Instant};
use tempfile::{TempDir, tempdir};
use test_toolkit::{Backend, Level, require_level};

/// Bound on one `md` run; only a cold or loaded host comes near it.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
/// More rows than any report here prints, so a tall report is never cut off.
const SCROLLBACK_ROWS: u32 = 500;

/// The root document with a transcluded child, as in the Level 1 report tests.
fn root_and_child_fixture() -> TempDir {
    let dir = tempdir().expect("create fixture directory");
    let docs = dir.path().join("docs");
    fs::create_dir_all(&docs).expect("create docs directory");
    fs::write(docs.join("child.md"), "# Child\n\n[gone](./child-gone.md)\n").expect("write child");
    fs::write(
        docs.join("root.md"),
        "# Root\n\n[gone](./gone.md)\n\n::file child.md\n\n![image](./image.png)\n",
    )
    .expect("write root");
    dir
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct CellStyle {
    foreground: Option<Foreground>,
    background: bool,
    bold: bool,
    dim: bool,
    italic: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Foreground {
    Basic(u16),
    Indexed(u16),
    Rgb(u16, u16, u16),
}

impl Foreground {
    /// Red in whichever form the terminal kept it: a truecolor capture holds
    /// the theme's red-500, a downgraded one a palette entry.
    fn is_red(self) -> bool {
        match self {
            Foreground::Basic(code) => code == 31 || code == 91,
            Foreground::Indexed(index) => matches!(index, 1 | 9 | 160 | 196 | 197 | 202 | 203 | 204 | 209 | 210),
            Foreground::Rgb(red, green, blue) => red > 150 && red > green * 2 && red > blue * 2,
        }
    }

    fn is_yellow(self) -> bool {
        match self {
            Foreground::Basic(code) => code == 33 || code == 93,
            Foreground::Indexed(index) => matches!(index, 3 | 11 | 178 | 184 | 214 | 220 | 221 | 226),
            Foreground::Rgb(red, green, blue) => red > 150 && green > 100 && blue * 2 < green,
        }
    }
}

fn apply_sgr(style: &mut CellStyle, parameters: &str) {
    let mut codes = parameters.split(';').map(|code| code.parse::<u16>().unwrap_or(0));
    while let Some(code) = codes.next() {
        match code {
            0 => *style = CellStyle::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            22 => (style.bold, style.dim) = (false, false),
            23 => style.italic = false,
            30..=37 | 90..=97 => style.foreground = Some(Foreground::Basic(code)),
            39 => style.foreground = None,
            40..=47 | 100..=107 => style.background = true,
            49 => style.background = false,
            38 | 48 | 58 => {
                let color = match codes.next() {
                    Some(5) => codes.next().map(Foreground::Indexed),
                    Some(2) => match (codes.next(), codes.next(), codes.next()) {
                        (Some(red), Some(green), Some(blue)) => Some(Foreground::Rgb(red, green, blue)),
                        _ => None,
                    },
                    _ => None,
                };
                match code {
                    38 => style.foreground = color,
                    48 => style.background = color.is_some(),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// One captured row's characters, each with the style it was drawn in.
struct ScreenRow {
    cells: Vec<(char, CellStyle)>,
}

impl ScreenRow {
    fn text(&self) -> String {
        self.cells.iter().map(|(character, _)| character).collect()
    }

    /// The column where `needle` starts.
    fn column_of(&self, needle: &str) -> Option<usize> {
        let text = self.text();
        text.find(needle).map(|byte| text[..byte].chars().count())
    }

    /// The style of every character of `needle`, which must be on this row.
    fn styles_of(&self, needle: &str) -> Vec<CellStyle> {
        let start = self
            .column_of(needle)
            .unwrap_or_else(|| panic!("{needle:?} is not on row {:?}", self.text()));
        self.cells[start..start + needle.chars().count()].iter().map(|(_, style)| *style).collect()
    }
}

/// Replays a capture into rows. tmux writes only attribute changes and carries
/// them across rows, so a cell's style is known only from the capture's start.
fn screen_rows(raw: &str) -> Vec<ScreenRow> {
    let mut style = CellStyle::default();
    let mut rows = vec![ScreenRow { cells: Vec::new() }];
    let mut characters = raw.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\u{1b}' => match characters.next() {
                Some('[') => {
                    let mut body = String::new();
                    for next in characters.by_ref() {
                        if next.is_ascii_alphabetic() || next == '~' {
                            if next == 'm' {
                                apply_sgr(&mut style, &body);
                            }
                            break;
                        }
                        body.push(next);
                    }
                }
                Some(']') => {
                    while let Some(next) = characters.next() {
                        if next == '\u{7}' {
                            break;
                        }
                        if next == '\u{1b}' {
                            characters.next();
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\n' => rows.push(ScreenRow { cells: Vec::new() }),
            '\r' => {}
            other => rows.last_mut().expect("rows starts non-empty").cells.push((other, style)),
        }
    }
    rows
}

/// A tmux pane at `cols` columns, in the fixture directory.
struct ReportPane {
    harness: TmuxHarness,
    fixture: TempDir,
    runs: usize,
}

impl ReportPane {
    fn new(cols: u32) -> Self {
        let fixture = root_and_child_fixture();
        let mut harness = TmuxHarness::new();
        harness.spawn_shell().expect("spawn tmux shell");
        harness.resize(cols, 60).expect("resize tmux pane");
        assert_eq!(harness.pane_cols().expect("read pane width"), cols);
        let mut pane = Self { harness, fixture, runs: 0 };
        let directory = shell_path(pane.fixture.path());
        pane.run(&format!("cd {directory}"));
        pane
    }

    /// Runs `arguments` through the built `md` and returns the rows it drew.
    fn md(&mut self, arguments: &str) -> Vec<ScreenRow> {
        self.run(&format!("{} {arguments}", md_shim_word()))
    }

    /// The rows strictly between printed markers. The typed command never
    /// spells a marker, since `printf` assembles it, and tmux's shell scrolls a
    /// long command line sideways, so the echo cannot serve as an anchor.
    fn run(&mut self, command: &str) -> Vec<ScreenRow> {
        self.runs += 1;
        let id = self.runs;
        let (begin, end) = (format!("DM-BEGIN-{id}"), format!("DM-END-{id}"));
        self.harness
            .send_command_with_env(
                &format!("printf '%s-%s\\n' DM-BEGIN {id}; {command}; printf '%s-%s\\n' DM-END {id}"),
                &[],
            )
            .expect("send command to tmux");

        let deadline = Instant::now() + COMMAND_TIMEOUT;
        loop {
            let frame = self.harness.capture_scrollback(SCROLLBACK_ROWS).expect("capture tmux pane");
            let mut rows = screen_rows(&frame.raw);
            let line_of = |rows: &[ScreenRow], marker: &str| rows.iter().position(|row| row.text().trim() == marker);
            if let (Some(first), Some(last)) = (line_of(&rows, &begin), line_of(&rows, &end)) {
                assert!(first < last, "markers out of order:\n{}", frame.raw);
                rows.truncate(last);
                return rows.split_off(first + 1);
            }
            assert!(Instant::now() < deadline, "{command} did not finish:\n{}", frame.plain);
            std::thread::sleep(Duration::from_millis(40));
        }
    }
}

fn shown(rows: &[ScreenRow]) -> String {
    rows.iter().map(|row| row.text().trim_end().to_string()).collect::<Vec<_>>().join("\n")
}

/// The one row holding `needle`.
fn row_with<'rows>(rows: &'rows [ScreenRow], needle: &str) -> &'rows ScreenRow {
    let mut matching = rows.iter().filter(|row| row.text().contains(needle));
    match (matching.next(), matching.next()) {
        (Some(row), None) => row,
        (None, _) => panic!("no row holds {needle:?}:\n{}", shown(rows)),
        (Some(_), Some(_)) => panic!("several rows hold {needle:?}:\n{}", shown(rows)),
    }
}

/// The report's groups in order, with each row's reason red, its construct
/// italic, and only the `:line:col` of its location dimmed.
fn assert_styled_report(rows: &[ScreenRow]) {
    let text = shown(rows);
    let order = [
        "Broken references in root.md",
        "docs/root.md",
        "./gone.md",
        "./image.png",
        "docs/child.md (transcluded at root.md:5)",
        "./child-gone.md",
        "Checked 4 references across 2 documents: 1 ok, 3 broken",
    ];
    let mut from = 0;
    for needle in order {
        let found = text[from..].find(needle).unwrap_or_else(|| panic!("{needle:?} is missing or out of order:\n{text}"));
        from += found + needle.len();
    }

    for (reference, construct, location, position) in [
        ("./gone.md", "in hyperlink", "root.md", ":3:1"),
        ("./image.png", "in image", "root.md", ":7:1"),
        ("./child-gone.md", "in hyperlink", "child.md", ":3:1"),
    ] {
        let row = row_with(rows, reference);
        for style in row.styles_of("file not found") {
            let foreground = style.foreground.unwrap_or_else(|| panic!("reason has no color on {:?}", row.text()));
            assert!(foreground.is_red(), "reason is {foreground:?}, not red, on {:?}", row.text());
        }
        assert!(row.styles_of(construct).iter().all(|style| style.italic), "construct is not italic on {:?}", row.text());
        // Inline code: a themed run or a dim one, depending on the fold's context.
        assert!(
            row.styles_of(reference).iter().all(|style| (style.dim || style.background) && !style.foreground.is_some_and(Foreground::is_red)),
            "reference is not set apart as code on {:?}",
            row.text()
        );
        let full_location = format!("{location}{position}");
        assert!(row.text().contains(&full_location), "no {full_location} on {:?}", row.text());
        let start = row.column_of(&full_location).unwrap();
        let (name, numbers) = row.cells[start..start + full_location.chars().count()].split_at(location.chars().count());
        assert!(name.iter().all(|(_, style)| !style.dim), "file name is dim on {:?}", row.text());
        assert!(numbers.iter().all(|(_, style)| style.dim), "{position} is not dim on {:?}", row.text());
    }

    assert!(row_with(rows, "Broken references in root.md").styles_of("Broken references").iter().all(|style| style.bold));
    assert!(row_with(rows, "docs/root.md").styles_of("docs/root.md").iter().all(|style| style.bold));
    let child_heading = row_with(rows, "(transcluded at root.md:5)");
    assert!(child_heading.styles_of("docs/child.md").iter().all(|style| style.bold && !style.dim));
    assert!(child_heading.styles_of("(transcluded at root.md:5)").iter().all(|style| style.dim));
    let summary = row_with(rows, "Checked 4 references");
    for style in summary.styles_of("3 broken") {
        assert!(style.bold && style.foreground.is_some_and(Foreground::is_red), "summary count is {style:?}");
    }
}

fn assert_same_column(rows: &[ScreenRow], needles: &[&str]) {
    let columns: Vec<Option<usize>> = needles.iter().map(|needle| row_with(rows, needle).column_of(needle)).collect();
    assert!(columns[0].is_some() && columns.windows(2).all(|pair| pair[0] == pair[1]), "{needles:?} are at columns {columns:?}:\n{}", shown(rows));
}

#[test]
fn level2_validate_refs_shows_styled_groups_in_aligned_columns() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut pane = ReportPane::new(120);

    let rows = pane.md("validate refs docs/root.md");

    assert_styled_report(&rows);
    assert_same_column(&rows, &["./gone.md", "./image.png"]);
    assert_same_column(&rows, &["root.md:3:1", "root.md:7:1"]);
    assert_eq!(
        row_with(&rows, "./gone.md").column_of("file not found"),
        row_with(&rows, "./child-gone.md").column_of("file not found"),
        "both groups' rows sit under the same indent:\n{}",
        shown(&rows)
    );
}

/// A `COLUMNS`-wide layout: each row stacked as reason, reference, location,
/// with the reference and location whole on rows of their own.
fn assert_stacked_report(rows: &[ScreenRow], columns: u32) {
    let text = shown(rows);
    for row in rows {
        assert!(row.text().trim_end().chars().count() <= columns as usize, "row overflows {columns} columns:\n{text}");
    }
    for (reference, location) in [
        ("./gone.md", "root.md:3:1"),
        ("./image.png", "root.md:7:1"),
        ("./child-gone.md", "child.md:3:1"),
    ] {
        let at = rows
            .iter()
            .position(|row| row.text().trim() == reference)
            .unwrap_or_else(|| panic!("{reference} is split, lost, or shares a row:\n{text}"));
        assert!(rows[at - 1].text().contains("file not found in"), "{reference} is not under its reason:\n{text}");
        assert_eq!(rows[at + 1].text().trim(), location, "{reference}'s location is not beneath it:\n{text}");
        assert_eq!(rows[at - 1].column_of("file not found"), Some(2), "{text}");
        assert_eq!(rows[at].column_of(reference), Some(4), "{text}");
        assert_eq!(rows[at + 1].column_of(location), Some(4), "{text}");
    }
    for needle in ["docs/root.md", "docs/child.md", "(transcluded at", "root.md:5)"] {
        assert!(text.contains(needle), "{needle} is lost:\n{text}");
    }
    let reason_row = rows.iter().find(|row| row.text().contains("file not found")).expect("a reason row");
    assert!(reason_row.styles_of("file not found").iter().all(|style| style.foreground.is_some_and(Foreground::is_red)), "{text}");
    let location_row = row_with(rows, "child.md:3:1");
    assert!(location_row.styles_of(":3:1").iter().all(|style| style.dim), "{text}");
    assert!(location_row.styles_of("child.md").iter().all(|style| !style.dim), "{text}");
}

#[test]
fn level2_narrow_pane_stacks_rows_and_keeps_every_reference_and_location_whole() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    const COLUMNS: u32 = 30;
    let mut pane = ReportPane::new(COLUMNS);

    let rows = pane.md("validate refs docs/root.md");

    assert_stacked_report(&rows, COLUMNS);
}

/// One row per severity, plus a glob hint at a width where the hint must wrap.
#[test]
fn level2_reason_color_follows_severity_and_a_hint_wraps_between_words() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut pane = ReportPane::new(72);
    fs::write(
        pane.fixture.path().join("severities.md"),
        "---\nlink: \"[l](./nope.md)\"\n---\nsee {{ link }}\n\n[mail](mailto:a@example.com)\n\n[star](./gone*.md)\n",
    )
    .expect("write severities fixture");

    let rows = pane.md("validate refs severities.md");
    let text = shown(&rows);

    let reason = |reference: &str, phrase: &str| row_with(&rows, reference).styles_of(phrase);
    assert!(reason("./nope.md", "file not found").iter().all(|style| style.foreground.is_some_and(Foreground::is_yellow)), "warning:\n{text}");
    assert!(reason("mailto:a@example.com", "unsupported scheme").iter().all(|style| style.dim && style.foreground.is_none()), "info:\n{text}");
    assert!(reason("./gone*.md", "file not found").iter().all(|style| style.foreground.is_some_and(Foreground::is_red)), "error:\n{text}");
    assert_same_column(&rows, &["./nope.md", "mailto:a@example.com", "./gone*.md"]);
    assert_same_column(&rows, &["severities.md:4:5", "severities.md:6:1", "severities.md:8:1"]);

    let hint = row_with(&rows, "are literal in a file reference");
    assert!(hint.styles_of("are literal").iter().all(|style| style.dim), "{text}");
    assert_eq!(hint.column_of("`*`"), Some(2), "{text}");
    assert!(text.contains("`::file-links`"), "the hint's token is split:\n{text}");
    assert!(row_with(&rows, "1 warning").styles_of("1 warning").iter().all(|style| style.dim), "{text}");
}

#[test]
fn level2_graph_validate_shows_the_same_styled_report_after_its_tree() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut pane = ReportPane::new(120);

    let rows = pane.md("graph docs/root.md --validate");

    let report_start = rows.iter().position(|row| row.text().contains("Broken references in root.md")).unwrap_or_else(|| panic!("no report:\n{}", shown(&rows)));
    assert!(report_start > 0, "the tree precedes the report:\n{}", shown(&rows));
    assert_styled_report(&rows[report_start..]);
    assert_same_column(&rows[report_start..], &["./gone.md", "./image.png"]);
}

/// stderr is judged by its own terminal, so a redirected deliverable leaves the
/// refusal styled and sized to the pane.
#[test]
fn level2_compose_refusal_styles_the_report_on_stderr_with_stdout_redirected() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut pane = ReportPane::new(120);

    let rows = pane.md("compose docs/root.md > composed.md");

    assert_styled_report(&rows);
    assert_same_column(&rows, &["./gone.md", "./image.png"]);
    let deliverable = fs::read(pane.fixture.path().join("composed.md")).expect("read redirected stdout");
    assert!(deliverable.is_empty(), "a refused document writes no deliverable");
}

/// The narrow layout follows stderr's own columns, whatever stdout is.
#[test]
fn level2_compose_refusal_stacks_at_the_width_of_stderrs_pane() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    const COLUMNS: u32 = 30;
    let mut pane = ReportPane::new(COLUMNS);

    let rows = pane.md("compose docs/root.md > composed.md");

    assert_stacked_report(&rows, COLUMNS);
}

/// A location is one clickable link to the file holding the problem.
#[test]
fn level2_location_reaches_the_terminal_as_a_hyperlink() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
    let fixture = root_and_child_fixture();
    let mut guard = SHARED_HARNESS.get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("WezTerm harness present");

    run_with_sentinel(harness, &format!("cd {}; clear", shell_path(fixture.path())));
    let frame = run_with_sentinel(harness, &format!("{} validate refs docs/root.md", md_shim_word()));

    let root_row = frame
        .raw
        .lines()
        .find(|line| line.contains("./gone.md"))
        .unwrap_or_else(|| panic!("no row of ./gone.md:\n{}", frame.plain));
    let uri = root_row.find("file://").unwrap_or_else(|| panic!("the location is not a link: {root_row:?}"));
    let opener = root_row[..uri].rfind("\x1b]8;").unwrap_or_else(|| panic!("the location is not a link: {root_row:?}"));
    // WezTerm stores the parameters as a map, so their order is its own.
    let parameters: Vec<&str> = root_row[opener + 4..uri].trim_end_matches(';').split(':').collect();
    for parameter in ["line=3", "column=1", "heading=root"] {
        assert!(parameters.contains(&parameter), "no {parameter} in {parameters:?}");
    }
    let destination = root_row[uri..].split('\x1b').next().unwrap_or_default();
    assert!(destination.ends_with("/docs/root.md"), "the link names a bare file: {destination:?}");
}
