//! Level 2 tests for the declaration commands (`functions`, `types`,
//! `symbols`, `classes`) and the `refs` report rendered in a real terminal.
//!
//! The Level 1 tests pin the plain goldens and the forced-color bytes from a
//! captured pipe. Only a real terminal exercises the path users see: `hug`
//! decides on color from the TTY it is attached to (no `CLICOLOR_FORCE`), and
//! the terminal turns the renderer's escapes into styled cells.
//!
//! Every command runs on the backend's shared pane between its own begin and
//! end markers, and every check reads only the rows between them, captured
//! from the pane's whole history (`support/terminal_report.rs`). Earlier
//! reports stay in that history, so an unscoped check could be decided by
//! them.
//!
//! The fixture tree (`tests/fixtures/declarations/`) has three packages: the
//! root (a TypeScript class with static and instance members), `tools` (one
//! Python declaration far wider than any pane) and `widgets` (a `package.json`
//! package), so listings that span the tree split into package sections.
//!
//! `hug` wraps interactive output at the pane's width, so the layout and
//! styling checks pin a wide `COLUMNS`; `check_wrapping` and `check_table`
//! check `hug`'s own wrapping at the pane width and at an injected one.
//!
//! The report views (`--tree`, `--stats`, `--signatures full|none`, and
//! `classes --format table`) are compared row for row, indentation included,
//! so those checks need a pane at least 80 columns wide. Each view is a
//! separate nextest case on every backend; the broker shares the pane without
//! bundling the views into one long-running test.
//!
//! Skip-clean: when no terminal harness is available each test prints a skip
//! notice and passes.

// Shared with the other terminal binaries; each uses only part of it.
#[allow(dead_code)]
#[path = "support/terminal_report.rs"]
mod terminal_report;

use std::path::{Path, PathBuf};
use std::time::Duration;

use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::TerminalHarness;
use serial_test::serial;
use tempfile::TempDir;
use terminal_report::{Report, has_hyperlink, run_scoped};
use test_toolkit::{Backend, Level, require_level};

static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();

/// Scrollback rows read from WezTerm: far more than one report and the
/// commands around it.
const WEZTERM_HISTORY_ROWS: u32 = 1000;

/// The declaration whose rendered line is wider than any test pane.
const LONG_DECLARATION: &str = "function summarize_quarterly_revenue_projection(\
regional_breakdown: dict[str, list[float]], currency_conversion_table: dict[str, float]) \
-> dict[str, float] tools/long.py:5:1";

/// Absolute path to the built `hug` binary.
fn hug() -> PathBuf {
    biscuit_test_harness::bin_exe!("hug")
}

/// A copy of the fixture tree in a scratch Git repository, plus test-owned
/// home and temp directories so `hug` sees no caller cache or configuration.
struct Fixture {
    repo: TempDir,
    home: TempDir,
    temp: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self::empty();
        copy_tree(
            &biscuit_test_harness::manifest_dir!().join("tests/fixtures/declarations"),
            fixture.repo.path(),
        );
        fixture
    }

    fn empty() -> Self {
        let fixture = Self {
            repo: TempDir::new().expect("scratch repo"),
            home: TempDir::new().expect("scratch home"),
            temp: TempDir::new().expect("scratch temp"),
        };
        let status = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(fixture.repo.path())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("HOME", fixture.home.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .expect("git must be on PATH to create the scratch repository");
        assert!(
            status.success(),
            "git init failed in the scratch repository"
        );
        fixture
    }
}

fn copy_tree(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).expect("read fixture directory") {
        let entry = entry.expect("fixture entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("fixture entry type").is_dir() {
            std::fs::create_dir_all(&target).expect("create fixture directory");
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy fixture file");
        }
    }
}

/// `path` as a single word for the POSIX shell in the pane (slash separators
/// for Git Bash on Windows; single quotes keep spaces together).
fn shell_word(path: &Path) -> String {
    let portable = biscuit_file::to_portable_string(path);
    format!("'{}'", portable.replace('\'', "'\\''"))
}

/// Runs `hug <args>` from the fixture repository and returns only the rows
/// it printed (see [`terminal_report`]).
///
/// `CLICOLOR_FORCE`, `NO_COLOR` and `COLUMNS` are removed, so styling and
/// the wrap width come from `hug` detecting the pane's TTY; `env` (shell
/// assignments such as `COLUMNS=64`) is set after that. The subshell leaves
/// the shared pane's directory alone.
fn run_hug<H: TerminalHarness>(
    harness: &mut H,
    pane: &Pane<H>,
    fixture: &Fixture,
    env: &str,
    args: &str,
) -> Report {
    let config = fixture.home.path().join(".config");
    let command = format!(
        "(cd {repo} && env -u CLICOLOR_FORCE -u NO_COLOR -u COLUMNS HOME={home} XDG_CONFIG_HOME={config} \
         TMPDIR={temp} TMP={temp} TEMP={temp} RAYON_NUM_THREADS=2 {env} {hug} {args})",
        repo = shell_word(fixture.repo.path()),
        home = shell_word(fixture.home.path()),
        config = shell_word(&config),
        temp = shell_word(fixture.temp.path()),
        hug = shell_word(&hug()),
    );
    run_scoped(harness, &command, Duration::from_secs(20), pane.capture)
}

/// How the test reads one backend's pane.
struct Pane<H> {
    /// The capture keeps OSC 8 hyperlinks.
    osc8: bool,
    /// The pane width in columns.
    cols: fn(&mut H) -> usize,
    /// The pane width when the capture splits soft-wrapped rows, so the long
    /// row can be rejoined; `None` when the capture returns them whole.
    wrap_cols: fn(&mut H) -> Option<usize>,
    /// The pane's whole history, scrollback included, with escapes.
    capture: fn(&mut H) -> String,
}

/// A width past every row, so `hug` wraps nothing and the styling checks
/// see whole rows; wrapping has its own checks.
const UNWRAPPED: &str = "COLUMNS=10000";

/// Runs the four declaration commands on one pane and checks their layout and
/// styling, each against only the rows that command printed, then `hug`'s
/// own wrapping and the table layout.
fn check_declarations<H: TerminalHarness>(harness: &mut H, pane: &Pane<H>) {
    let fixture = Fixture::new();

    // functions: package sections in path order, compact signatures with
    // container suffixes, `path:line:col` locations.
    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "functions");
    const CREATE: &str =
        "- public method create(size: number): Shape [in class Shape] geometry.ts:14:3";
    report.assert_rows_in_order(&[
        CREATE,
        "- public method area(scale: number): Area [in class Shape] geometry.ts:18:3",
        "- public function buildShape(): Shape geometry.ts:23:8",
        "tools",
        "- function summarize_quarterly_revenue_projection(",
        "widgets",
        "- public function renderWidget(name) widgets/index.js:4:8",
    ]);
    // The long declaration is one logical row: `COLUMNS` is far past the
    // pane, so only the terminal soft-wraps it.
    let long_row = format!("- {LONG_DECLARATION}");
    let unwrapped = match (pane.wrap_cols)(harness) {
        Some(cols) => report.has_logical_row(&long_row, cols),
        None => report.has_row(&long_row),
    };
    assert!(
        unwrapped,
        "expected the long declaration on one logical row.\nreport:\n{}",
        report.plain(),
    );
    report.assert_styled(CREATE, "public", &["3"], "italic visibility");
    report.assert_styled(CREATE, "method", &["32"], "a green callable label");
    report.assert_styled(
        CREATE,
        "create(size: number)",
        &["1"],
        "a bold name and signature",
    );
    report.assert_styled(CREATE, "geometry.ts:14:3", &["2"], "a dim location");
    report.assert_styled(
        "tools",
        "tools",
        &["1", "34"],
        "a bold blue package heading",
    );
    if pane.osc8 {
        // The location is the text of a link to its file and line.
        let row = report.row(CREATE);
        assert!(
            has_hyperlink(&row.raw, "/geometry.ts#L14", "geometry.ts:14:3"),
            "expected the location linked to geometry.ts#L14.\nrow:\n{:?}",
            row.raw,
        );
    }

    // types: only the root package declares any, but the scan spans three,
    // so the others are sections of their own; the declaring keyword is the
    // label.
    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "types");
    const SIZED: &str = "- public interface Sized { size: number } geometry.ts:4:8";
    report.assert_rows_in_order(&[
        SIZED,
        "- public type alias Area geometry.ts:8:8",
        "- public class Shape { count: number, size: number } geometry.ts:10:8",
        "tools",
        "(no symbols)",
        "widgets",
        "(no symbols)",
    ]);
    report.assert_styled(SIZED, "interface", &["35"], "a magenta type label");
    report.assert_styled(
        SIZED,
        "Sized { size: number }",
        &["1"],
        "a bold name and fields",
    );
    report.assert_styled(SIZED, "geometry.ts:4:8", &["2"], "a dim location");

    // symbols --group-by-file: package and file headings, then `[line:col]`
    // locations; `--all-scopes` lists the parameters, which are locals.
    let report = run_hug(
        harness,
        pane,
        &fixture,
        UNWRAPPED,
        "symbols --all-scopes --group-by-file 'tools/*.py' 'widgets/*.js'",
    );
    const REGIONAL: &str = "- parameter regional_breakdown [5:44]";
    report.assert_rows_in_order(&[
        "tools",
        "tools/long.py",
        "- function summarize_quarterly_revenue_projection(",
        REGIONAL,
        "- parameter currency_conversion_table [5:88]",
        "widgets",
        "widgets/index.js",
        "- public function renderWidget(name) [4:8]",
        "- parameter name [4:30]",
    ]);
    report.assert_styled(
        "tools/long.py",
        "tools/long.py",
        &["1"],
        "a bold file heading",
    );
    report.assert_styled(REGIONAL, "parameter", &["34"], "a blue parameter label");
    report.assert_styled(REGIONAL, "[5:44]", &["2"], "a dim location");

    // functions --group-by-module: a package heading, then its directory
    // heading; the scan root's directory is `(root)`.
    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "functions --group-by-module");
    report.assert_rows_in_order(&[
        "(root)",
        CREATE,
        "tools",
        "tools",
        "- function summarize_quarterly_revenue_projection(",
        "widgets",
        "widgets",
        "- public function renderWidget(name) widgets/index.js:4:8",
    ]);
    report.assert_styled("(root)", "(root)", &["1"], "a bold directory heading");

    // classes: the file header, each container, then its static and instance
    // method and field sections.
    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "classes geometry.ts");
    const HEADER: &str = "geometry.ts (TypeScript)";
    const SHAPE: &str = "class Shape [10:8]";
    const CREATE_MEMBER: &str = "- public create(size: number): Shape [in class Shape] [14:3]";
    const COUNT: &str = "- public count: number";
    report.assert_rows_in_order(&[
        HEADER,
        "interface Sized [4:8]",
        "Instance Fields (1)",
        "- public size: number",
        SHAPE,
        "Static Methods (1)",
        CREATE_MEMBER,
        "Instance Methods (1)",
        "- public area(scale: number): Area [in class Shape] [18:3]",
        "Static Fields (1)",
        COUNT,
        "Instance Fields (1)",
        "- public size: number",
    ]);
    report.assert_styled(HEADER, "geometry.ts", &["1"], "a bold file header");
    report.assert_styled(HEADER, "TypeScript", &["2"], "a dim language");
    report.assert_styled(SHAPE, "class", &["35"], "a magenta container label");
    report.assert_styled(SHAPE, "Shape", &["1"], "a bold container name");
    report.assert_styled(
        "Static Methods (1)",
        "Static Methods",
        &["33"],
        "a yellow section title",
    );
    report.assert_styled(
        CREATE_MEMBER,
        "create(size: number)",
        &["32"],
        "a green method",
    );
    report.assert_styled(CREATE_MEMBER, "[14:3]", &["2"], "a dim location");
    report.assert_styled(COUNT, "count: number", &["36"], "a cyan field");
    if pane.osc8 {
        let row = report.row(SHAPE);
        assert!(
            has_hyperlink(&row.raw, "/geometry.ts#L10", "[10:8]"),
            "expected the container location linked to geometry.ts#L10.\nrow:\n{:?}",
            row.raw,
        );
    }
}

/// `hug` wraps interactive listings itself, with the list's hanging indent:
/// at the pane's own width, and at an injected narrower `COLUMNS`.
fn check_wrapping<H: TerminalHarness>(harness: &mut H, pane: &Pane<H>) {
    let fixture = Fixture::new();
    let cols = (pane.cols)(harness);
    let item = format!("- {LONG_DECLARATION}");
    // A token longer than the space left is hyphenated where it breaks, so
    // the comparison drops whitespace and hyphens on both sides.
    let squeezed = |text: &str| -> String {
        text.chars().filter(|character| !character.is_whitespace() && *character != '-').collect()
    };

    // The pane's width: no row is wider than the pane, so the terminal never
    // soft-wraps, and the rows hold exactly the declaration's text.
    let report = run_hug(harness, pane, &fixture, "", "functions 'tools/*.py'");
    let lines = report.lines();
    for line in &lines {
        assert!(
            line.chars().count() <= cols,
            "a row is wider than the {cols}-column pane: {line:?}\nreport:\n{}",
            report.plain(),
        );
    }
    assert_eq!(squeezed(&lines.concat()), squeezed(&item), "report:\n{}", report.plain());

    // An injected width narrower than any pane: the declaration is broken
    // into several rows, each continuation under the item's text, and the
    // location keeps its dim style on the row it lands on.
    let report = run_hug(harness, pane, &fixture, "COLUMNS=60", "functions 'tools/*.py'");
    let lines: Vec<&str> = report.lines().into_iter().filter(|line| !line.trim().is_empty()).collect();
    assert!(lines.len() >= 3, "expected the declaration on several rows.\nreport:\n{}", report.plain());
    assert!(
        lines[0].trim_start().starts_with("- function"),
        "the item starts the report.\nreport:\n{}",
        report.plain(),
    );
    for line in &lines {
        assert!(line.chars().count() <= 60, "a row is wider than COLUMNS=60: {line:?}");
    }
    for line in &lines[1..] {
        assert!(
            line.starts_with("    ") && !line.trim_start().starts_with("- "),
            "a continuation row must hang under the item's text: {line:?}\nreport:\n{}",
            report.plain(),
        );
    }
    assert_eq!(squeezed(&lines.concat()), squeezed(&item), "report:\n{}", report.plain());
    let last = lines.last().expect("rows").trim();
    assert!(last.ends_with("tools/long.py:5:1"), "the location ends the item: {last:?}");
    let location_row = report.lines().into_iter().find(|line| line.contains("tools/long.py:5:1")).expect("location row");
    let location_style = report.style_in_row(location_row.trim(), "tools/long.py:5:1");
    assert!(
        location_style.as_ref().is_some_and(|style| style.iter().any(|p| p == "2")),
        "expected a dim location on its wrapped row; found {location_style:?}",
    );
}

/// `--format table`: a bordered biscuit-terminal table with the Location,
/// Kind, Visibility, Name and Signature columns, whose Signature column wraps
/// inside its cell at a narrow width.
fn check_table<H: TerminalHarness>(harness: &mut H, pane: &Pane<H>) {
    let fixture = Fixture::new();
    let cells = |line: &str| -> Vec<String> {
        line.trim()
            .trim_matches('│')
            .split('│')
            .map(|cell| cell.trim().to_string())
            .collect()
    };

    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "functions --format table geometry.ts");
    let lines = report.lines();
    let header = lines.iter().find(|line| line.contains("Location")).expect("header row");
    assert_eq!(cells(header), ["Location", "Kind", "Visibility", "Name", "Signature"]);
    let create = lines.iter().find(|line| line.contains("create")).expect("create row");
    assert_eq!(cells(create), ["geometry.ts:14", "method", "public", "create", "(size: number): Shape"]);
    if pane.osc8 {
        // The Location cell is the text of a link to its file and line.
        assert!(
            has_hyperlink(&report.raw(), "/geometry.ts#L14", "geometry.ts:14"),
            "expected the location linked to geometry.ts#L14.\nreport:\n{:?}",
            report.raw(),
        );
    }

    // Narrower than the table's natural width but wide enough for its
    // unwrapped columns: only Signature wraps, every row keeps the table's
    // width, and a signature continues below its row with an empty Location.
    let report = run_hug(harness, pane, &fixture, "COLUMNS=70", "functions --format table geometry.ts");
    let table: Vec<&str> = report
        .lines()
        .into_iter()
        .map(str::trim_end)
        .filter(|line| line.starts_with(['┌', '│', '├', '└']))
        .collect();
    let width = table[0].chars().count();
    assert!(width <= 70, "the table must fit COLUMNS=70: {width}");
    for line in &table {
        assert_eq!(line.chars().count(), width, "rows of one table share its width: {line:?}");
    }
    let mut signatures: Vec<(String, String)> = Vec::new();
    for line in table.iter().filter(|line| line.starts_with('│')).skip(1) {
        let row = cells(line);
        if !row[0].is_empty() {
            signatures.push((row[3].clone(), row[4].clone()));
        } else if let Some((_, signature)) = signatures.last_mut() {
            signature.push(' ');
            signature.push_str(&row[4]);
        }
    }
    let create: String = signatures
        .iter()
        .find(|(name, _)| name == "create")
        .map(|(_, signature)| signature.split_whitespace().collect())
        .expect("a create row");
    assert_eq!(create, "(size:number):Shape", "report:\n{}", report.plain());
    assert!(
        table.iter().filter(|line| line.starts_with('│')).count() > signatures.len() + 1,
        "the signatures must wrap onto continuation rows.\nreport:\n{}",
        report.plain(),
    );
}

/// The rows a report shows, indentation kept, trailing blanks trimmed and
/// blank rows dropped, so a layout compares as exact text.
///
/// The pane's interactive bash intermittently reports `bash: child setpgid
/// (…): Operation not permitted` when it forks the command's subshell. Bash
/// prints it in the child before any redirection applies, so it lands
/// between the markers; it is the shell's, not `hug`'s, and is dropped.
fn shown_rows(report: &Report) -> Vec<&str> {
    report
        .lines()
        .into_iter()
        .map(str::trim_end)
        .filter(|line| !line.is_empty() && !(line.starts_with("bash: child setpgid (") && line.ends_with("not permitted")))
        .collect()
}

/// The exact-layout checks compare whole rows, so the pane must show each
/// on one row of its own: a narrower pane soft-wraps them, and some backends
/// capture the wrapped pieces as separate rows.
fn assert_pane_fits<H: TerminalHarness>(harness: &mut H, pane: &Pane<H>, width: usize) {
    let cols = (pane.cols)(harness);
    assert!(cols >= width, "the exact-layout checks need a pane at least {width} columns wide; this one is {cols}");
}

/// Asserts that `text` in the row `row` carries every SGR parameter in
/// `with` and none in `without`.
fn assert_style(report: &Report, row: &str, text: &str, with: &[&str], without: &[&str], what: &str) {
    let style = report.style_in_row(row, text);
    assert!(
        style.as_ref().is_some_and(|style| with.iter().all(|want| style.iter().any(|p| p == want))
            && without.iter().all(|unwanted| style.iter().all(|p| p != unwanted))),
        "expected {what} (SGR {with:?}, without {without:?}) on {text:?} in row {row:?}; found {style:?}.\nrow:\n{:?}",
        report.row(row).raw,
    );
}

/// `--tree`: members nest under their container; a member whose container
/// the command did not select sits under a dim contextual heading, never a
/// record; `--group-by-module` adds directory headings and `--sort name`
/// orders containers and, separately, each container's members; a wrapped
/// member row hangs under its own text.
fn check_outline<H: TerminalHarness>(harness: &mut H, pane: &Pane<H>) {
    let fixture = Fixture::new();
    assert_pane_fits(harness, pane, 80);

    // `functions` selects no type, so `Shape`'s methods sit under a
    // contextual heading.
    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "functions --tree geometry.ts widgets/index.js");
    const CONTEXT: &str = "[in class Shape]";
    const CREATE: &str = "public method create(size: number): Shape [14:3]";
    assert_eq!(
        shown_rows(&report),
        [
            "geometry.ts",
            "  [in class Shape]",
            "    public method create(size: number): Shape [14:3]",
            "    public method area(scale: number): Area [18:3]",
            "  public function buildShape(): Shape [23:8]",
            "widgets/index.js",
            "  public function renderWidget(name) [4:8]",
        ],
        "report:\n{}",
        report.plain(),
    );
    assert_style(&report, "geometry.ts", "geometry.ts", &["1"], &["2"], "a bold file heading");
    assert_style(&report, CONTEXT, CONTEXT, &["2"], &["1", "35"], "a dim, unlabeled contextual heading");
    assert_style(&report, CREATE, "public", &["3"], &[], "italic visibility");
    assert_style(&report, CREATE, "method", &["32"], &[], "a green callable label");
    assert_style(&report, CREATE, "create(size: number): Shape", &["1"], &["2"], "a bold name and signature");
    assert_style(&report, CREATE, "[14:3]", &["2"], &["1"], "a dim location");
    if pane.osc8 {
        let row = report.row(CREATE);
        assert!(
            has_hyperlink(&row.raw, "/geometry.ts#L14", "[14:3]"),
            "expected the member location linked to geometry.ts#L14.\nrow:\n{:?}",
            row.raw,
        );
    }

    // Selected parents, module headings and name order.
    let report = run_hug(
        harness,
        pane,
        &fixture,
        UNWRAPPED,
        "symbols --tree --sort name --group-by-module geometry.ts widgets/index.js",
    );
    const SHAPE: &str = "public class Shape { count: number, size: number } implements Sized [10:8]";
    assert_eq!(
        shown_rows(&report),
        [
            "(root)",
            "geometry.ts",
            "  public type alias Area [8:8]",
            "  public class Shape { count: number, size: number } implements Sized [10:8]",
            "    public method area(scale: number): Area [18:3]",
            "    public property count [11:3]",
            "    public method create(size: number): Shape [14:3]",
            "    public property size [12:3]",
            "  public interface Sized { size: number } [4:8]",
            "    public property size [5:3]",
            "  public function buildShape(): Shape [23:8]",
            "widgets",
            "widgets/index.js",
            "  public function renderWidget(name) [4:8]",
        ],
        "report:\n{}",
        report.plain(),
    );
    assert_style(&report, "(root)", "(root)", &["1"], &[], "a bold module heading");
    assert_style(&report, "widgets", "widgets", &["1"], &[], "a bold module heading");
    assert_style(&report, SHAPE, "class", &["35"], &["2"], "a magenta label on a selected parent");
    assert_style(&report, SHAPE, "Shape { count: number, size: number }", &["1"], &["2"], "a bold selected parent");
    assert_style(&report, SHAPE, "implements Sized", &[], &["1", "2"], "a plain declared-bases clause");
    assert_style(&report, SHAPE, "[10:8]", &["2"], &["1"], "a dim location after the bases");

    // Narrow: each member wraps under its own text, four columns past the
    // member's indent, and keeps its styles on the continuation row.
    let report = run_hug(harness, pane, &fixture, "COLUMNS=36", "functions --tree geometry.ts");
    assert_eq!(
        shown_rows(&report),
        [
            "geometry.ts",
            "  [in class Shape]",
            "    public method create(size:",
            "        number): Shape [14:3]",
            "    public method area(scale:",
            "        number): Area [18:3]",
            "  public function buildShape():",
            "      Shape [23:8]",
        ],
        "report:\n{}",
        report.plain(),
    );
    assert_style(&report, CONTEXT, CONTEXT, &["2"], &["1"], "a dim contextual heading");
    assert_style(&report, "number): Shape [14:3]", "number): Shape", &["1"], &["2"], "a bold wrapped signature");
    assert_style(&report, "number): Shape [14:3]", "[14:3]", &["2"], &["1"], "a dim location on the wrapped row");

    // A type row's declared bases wrap with it, under the same hanging indent.
    let report = run_hug(harness, pane, &fixture, "COLUMNS=36", "types --tree --name Shape geometry.ts");
    assert_eq!(
        shown_rows(&report),
        [
            "geometry.ts",
            "  public class Shape { count:",
            "      number, size: number }",
            "      implements Sized [10:8]",
        ],
        "report:\n{}",
        report.plain(),
    );
    assert_style(&report, "implements Sized [10:8]", "[10:8]", &["2"], &["1"], "a dim location after wrapped bases");
    std::fs::write(fixture.repo.path().join("mixed.swift"),
        "protocol P {}\nprotocol Q {}\nstruct Box: P {}\nextension Box: Q {}\nclass Unknown: External {}\n"
    ).expect("mixed relation source");
    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "types --tree --signatures none mixed.swift");
    assert!(report.plain().contains("Box implements P, Q"), "{}", report.plain());
    assert!(report.plain().contains("Unknown unclassified bases External"), "{}", report.plain());
    assert_style(&report, "internal struct Box implements P, Q [3:1]", "implements P, Q", &[], &["1"], "relation text outside the bold declaration");
    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "types --tree --signatures full mixed.swift");
    assert!(report.plain().contains("Box: P implements Q"), "{}", report.plain());
    assert!(!report.plain().contains("implements P"), "{}", report.plain());

}

/// `--stats` retains every overall and package section at narrow widths.
/// Each section keeps its natural table width, counters and coverage;
/// package headings are bold literal text, including untrusted manifest names.
fn check_stats<H: TerminalHarness>(harness: &mut H, pane: &Pane<H>) {
    let fixture = Fixture::new();
    assert_pane_fits(harness, pane, 80);
    // Lua has no doc-comment detection, so its declaration is `unavailable`.
    std::fs::write(
        fixture.repo.path().join("tools/helper.lua"),
        "-- Formats a label.\nlocal function label(name)\n  return name\nend\n\nreturn { label = label }\n",
    )
    .expect("write the Lua fixture");
    let expected = [
        "┌────────────┬───────┐",
        "│ Kind       │ Count │",
        "├────────────┼───────┤",
        "│ class      │     1 │",
        "│ function   │     4 │",
        "│ interface  │     1 │",
        "│ method     │     2 │",
        "│ property   │     3 │",
        "│ type alias │     1 │",
        "└────────────┴───────┘",
        "┌────────────────┬───────┐",
        "│ Visibility     │ Count │",
        "├────────────────┼───────┤",
        "│ module-private │     1 │",
        "│ public         │    10 │",
        "│ unknown        │     1 │",
        "└────────────────┴───────┘",
        "┌────────────┬───────┐",
        "│ Language   │ Count │",
        "├────────────┼───────┤",
        "│ JavaScript │     1 │",
        "│ Lua        │     1 │",
        "│ Python     │     1 │",
        "│ TypeScript │     9 │",
        "└────────────┴───────┘",
        "Doc coverage: 0 of 11 documented (0.0%); 1 unavailable",
    ];
    let expected: Vec<&str> = ["All selected declarations"].into_iter()
        .chain(expected.iter().copied())
        .chain(["Unassigned"])
        .chain(expected.iter().copied())
        .collect();
    for env in ["", "COLUMNS=20"] {
        let report = run_hug(harness, pane, &fixture, env, "symbols --stats");
        assert_eq!(shown_rows(&report), expected, "`{env} hug symbols --stats`, report:\n{}", report.plain());
        for heading in ["All selected declarations", "Unassigned"] {
            assert_style(&report, heading, heading, &["1"], &["3"], "a bold stats heading");
        }
        // A blank row separates each table from what follows it.
        let lines: Vec<&str> = report.lines().into_iter().map(str::trim_end).collect();
        for (index, line) in lines.iter().enumerate() {
            if line.starts_with('└') {
                assert_eq!(lines.get(index + 1).copied(), Some(""), "a blank row after each table: {lines:?}");
            }
        }
    }
}

fn check_package_stats<H: TerminalHarness>(harness: &mut H, pane: &Pane<H>) {
    let fixture = Fixture::empty();
    let name = "é <i>x</i> [x](z)\u{1b}[31m\u{7}\n";
    let heading = "é <i>x</i> [x](z)\\u{1b}[31m\\u{7}\\u{a} (c)";
    let files = [
        ("Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\", \"c\", \"d\"]\n".to_string()),
        ("a/Cargo.toml", "[package]\nname = \"utils\"\nversion = \"0.1.0\"\n".to_string()),
        ("b/Cargo.toml", "[package]\nname = \"utils\"\nversion = \"0.1.0\"\n".to_string()),
        ("c/Cargo.toml", format!("[package]\nname = {}\nversion = \"0.1.0\"\n", serde_json::to_string(name).unwrap())),
        ("d/Cargo.toml", "[package]\nname = \"unselected\"\nversion = \"0.1.0\"\n".to_string()),
        ("a/src/lib.rs", "/// Documented.\npub fn first() {}\npub fn second() {}\n".to_string()),
        ("b/src/lib.rs", "pub fn third() {}\n".to_string()),
        ("c/src/lib.rs", "pub struct Empty;\n".to_string()),
        ("d/src/lib.rs", "pub fn excluded() {}\n".to_string()),
        ("loose.rs", "pub fn loose() {}\n".to_string()),
    ];
    for (path, contents) in files {
        let path = fixture.repo.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).expect("package directory");
        std::fs::write(path, contents).expect("package stats fixture");
    }
    let sections = [
        ("All selected declarations", 4, 1),
        ("utils (a)", 2, 1),
        ("utils (b)", 1, 0),
        (heading, 0, 0),
        ("Unassigned", 1, 0),
    ];
    let mut expected = Vec::new();
    for (title, count, documented) in sections {
        expected.push(title.to_string());
        for (category, label) in [("Kind", "function"), ("Visibility", "public"), ("Language", "Rust")] {
            let width = category.len().max(if count == 0 { 0 } else { label.len() });
            let border = "─".repeat(width + 2);
            expected.push(format!("┌{border}┬───────┐"));
            expected.push(format!("│ {category:<width$} │ Count │"));
            expected.push(format!("├{border}┼───────┤"));
            if count > 0 {
                expected.push(format!("│ {label:<width$} │ {count:>5} │"));
            }
            expected.push(format!("└{border}┴───────┘"));
        }
        let ratio = match count { 0 => "n/a", 2 => "50.0%", 4 => "25.0%", _ => "0.0%" };
        expected.push(format!("Doc coverage: {documented} of {count} documented ({ratio}); 0 unavailable"));
    }
    for env in ["", "COLUMNS=20"] {
        let report = run_hug(harness, pane, &fixture, env, "functions --stats --exclude-files 'd/**'");
        assert_eq!(shown_rows(&report), expected, "`{env} hug functions --stats`, report:\n{}", report.plain());
        for (title, _, _) in sections {
            assert_style(&report, title, title, &["1"], &["3", "31"], "a literal bold package heading");
        }
        assert_style(&report, heading, "x</i>", &["1"], &["3", "31"], "markup remains literal without italic cells");
        assert_style(&report, heading, "(c)", &["1"], &["3", "31"], "controls cannot change later heading cells");
        let raw = &report.row(heading).raw;
        assert!(!raw.contains("\u{1b}]8;") && !raw.contains('\u{7}'), "manifest controls must remain visible text: {raw:?}");
    }
}

/// `--signatures full` shows the declaration header as written in place of
/// visibility and label, and `--signatures none` only names; both in styled
/// lists, wrapped with the list's hanging indent, and in `--format table`.
fn check_signature_modes<H: TerminalHarness>(harness: &mut H, pane: &Pane<H>) {
    let fixture = Fixture::new();
    assert_pane_fits(harness, pane, 80);

    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "functions --signatures full geometry.ts");
    const FULL: &str = "- static create(size: number): Shape [in class Shape] geometry.ts:14:3";
    assert_eq!(
        shown_rows(&report),
        [
            "  - static create(size: number): Shape [in class Shape] geometry.ts:14:3",
            "  - area(scale: number): Area [in class Shape] geometry.ts:18:3",
            "  - function buildShape(): Shape geometry.ts:23:8",
        ],
        "report:\n{}",
        report.plain(),
    );
    assert_style(&report, FULL, "static create(size: number): Shape", &["1"], &["3", "32"], "a bold header, no label");
    assert_style(&report, FULL, "geometry.ts:14:3", &["2"], &["1"], "a dim location");
    if pane.osc8 {
        assert!(
            has_hyperlink(&report.row(FULL).raw, "/geometry.ts#L14", "geometry.ts:14:3"),
            "expected the location linked to geometry.ts#L14.\nrow:\n{:?}",
            report.row(FULL).raw,
        );
    }

    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "functions --signatures none geometry.ts");
    const NONE: &str = "- public method create [in class Shape] geometry.ts:14:3";
    assert_eq!(
        shown_rows(&report),
        [
            "  - public method create [in class Shape] geometry.ts:14:3",
            "  - public method area [in class Shape] geometry.ts:18:3",
            "  - public function buildShape geometry.ts:23:8",
        ],
        "report:\n{}",
        report.plain(),
    );
    assert_style(&report, NONE, "public", &["3"], &[], "italic visibility");
    assert_style(&report, NONE, "method", &["32"], &[], "a green callable label");
    assert_style(&report, NONE, "create", &["1"], &[], "a bold name");
    assert_style(&report, NONE, "geometry.ts:14:3", &["2"], &["1"], "a dim location");

    // A long full header wraps under the item's text, hyphenating the token
    // that does not fit, and the location moves to a row of its own.
    let report = run_hug(harness, pane, &fixture, "COLUMNS=50", "functions --signatures full 'tools/*.py'");
    const CONTINUED: &str = "onal_breakdown: dict[str, list[float]],";
    assert_eq!(
        shown_rows(&report),
        [
            "  - def",
            "    summarize_quarterly_revenue_projection(regi-",
            "    onal_breakdown: dict[str, list[float]],",
            "    currency_conversion_table: dict[str,",
            "    float]) -> dict[str, float]",
            "    tools/long.py:5:1",
        ],
        "report:\n{}",
        report.plain(),
    );
    assert_style(&report, CONTINUED, CONTINUED, &["1"], &["2"], "the bold header on a continuation row");
    assert_style(&report, "tools/long.py:5:1", "tools/long.py:5:1", &["2"], &["1"], "a dim location");

    let report = run_hug(harness, pane, &fixture, "COLUMNS=50", "functions --signatures none 'tools/*.py'");
    assert_eq!(
        shown_rows(&report),
        ["  - function", "    summarize_quarterly_revenue_projection", "    tools/long.py:5:1"],
        "report:\n{}",
        report.plain(),
    );
    assert_style(
        &report,
        "summarize_quarterly_revenue_projection",
        "summarize",
        &["1"],
        &["2"],
        "a bold name on its own row",
    );

    // Tables: the full header wraps inside the Signature cell; `none` drops
    // the column.
    let report = run_hug(
        harness,
        pane,
        &fixture,
        "COLUMNS=80",
        "functions --format table --signatures full geometry.ts",
    );
    assert_eq!(
        shown_rows(&report),
        [
            "┌────────────────┬──────────┬────────────┬────────────┬────────────────────────┐",
            "│ Location       │ Kind     │ Visibility │ Name       │ Signature              │",
            "├────────────────┼──────────┼────────────┼────────────┼────────────────────────┤",
            "│ geometry.ts:14 │ method   │ public     │ create     │ static create(size:    │",
            "│                │          │            │            │ number): Shape         │",
            "│ geometry.ts:18 │ method   │ public     │ area       │ area(scale: number):   │",
            "│                │          │            │            │ Area                   │",
            "│ geometry.ts:23 │ function │ public     │ buildShape │ function               │",
            "│                │          │            │            │ buildShape(): Shape    │",
            "└────────────────┴──────────┴────────────┴────────────┴────────────────────────┘",
        ],
        "report:\n{}",
        report.plain(),
    );

    let report = run_hug(
        harness,
        pane,
        &fixture,
        UNWRAPPED,
        "functions --format table --signatures none geometry.ts",
    );
    assert_eq!(
        shown_rows(&report),
        [
            "┌────────────────┬──────────┬────────────┬────────────┐",
            "│ Location       │ Kind     │ Visibility │ Name       │",
            "├────────────────┼──────────┼────────────┼────────────┤",
            "│ geometry.ts:14 │ method   │ public     │ create     │",
            "│ geometry.ts:18 │ method   │ public     │ area       │",
            "│ geometry.ts:23 │ function │ public     │ buildShape │",
            "└────────────────┴──────────┴────────────┴────────────┘",
        ],
        "report:\n{}",
        report.plain(),
    );
    if pane.osc8 {
        assert!(
            has_hyperlink(&report.raw(), "/geometry.ts#L14", "geometry.ts:14"),
            "expected the location linked to geometry.ts#L14.\nreport:\n{:?}",
            report.raw(),
        );
    }
}

/// `classes --format table`: each container row followed by its displayed
/// members, the Signature cell wrapping at a narrow width.
fn check_classes_table<H: TerminalHarness>(harness: &mut H, pane: &Pane<H>) {
    let fixture = Fixture::new();
    assert_pane_fits(harness, pane, 80);
    let report = run_hug(harness, pane, &fixture, "COLUMNS=72", "classes --format table geometry.ts");
    assert_eq!(
        shown_rows(&report),
        [
            "┌────────────────┬───────────┬────────────┬────────┬───────────────────┐",
            "│ Location       │ Kind      │ Visibility │ Name   │ Signature         │",
            "├────────────────┼───────────┼────────────┼────────┼───────────────────┤",
            "│ geometry.ts:4  │ interface │ public     │ Sized  │ { size: number }  │",
            "│ geometry.ts:10 │ class     │ public     │ Shape  │ { count: number,  │",
            "│                │           │            │        │ size: number }    │",
            "│ geometry.ts:14 │ method    │ public     │ create │ (size: number):   │",
            "│                │           │            │        │ Shape             │",
            "│ geometry.ts:18 │ method    │ public     │ area   │ (scale: number):  │",
            "│                │           │            │        │ Area              │",
            "└────────────────┴───────────┴────────────┴────────┴───────────────────┘",
        ],
        "report:\n{}",
        report.plain(),
    );
    if pane.osc8 {
        assert!(
            has_hyperlink(&report.raw(), "/geometry.ts#L10", "geometry.ts:10"),
            "expected the container location linked to geometry.ts#L10.\nreport:\n{:?}",
            report.raw(),
        );
    }
}

/// The boundary every `refs` report states.
const REFS_BOUNDARY: &str = "References are resolved within one file: only the declaration's own file is \
searched, so a caller in another file is never listed, even when it uses the same name.";

/// Whether `text` is one logical row of `report`, rejoining the rows a
/// backend's capture split at the pane edge.
fn has_logical<H: TerminalHarness>(harness: &mut H, pane: &Pane<H>, report: &Report, text: &str) -> bool {
    match (pane.wrap_cols)(harness) {
        Some(cols) => report.has_logical_row(text, cols),
        None => report.has_row(text),
    }
}

/// `refs`: the heading, the boundary (also on an empty report), established
/// rows grouped under their owners with bold names, dim locations linked to
/// their lines, the callees view's dim explanations, and `hug`'s own
/// wrapping of the boundary at an injected width.
fn check_refs<H: TerminalHarness>(harness: &mut H, pane: &Pane<H>) {
    let fixture = Fixture::new();

    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "refs Shape geometry.ts");
    const CALL: &str = "- Call Shape 15:16";
    report.assert_rows_in_order(&[
        "Established (Scoped to this declaration)",
        "geometry.ts#Shape.create 14:3",
        "- Usage Shape 14:32",
        CALL,
        "geometry.ts#buildShape 23:8",
        "- Usage Shape 23:31",
        "- Usage Shape 24:10",
    ]);
    for row in [
        "References to public class Shape { count: number, size: number } geometry.ts:10:8",
        REFS_BOUNDARY,
    ] {
        assert!(has_logical(harness, pane, &report, row), "expected the row {row:?}.\nreport:\n{}", report.plain());
    }
    report.assert_styled("Established (Scoped to this declaration)", "Established", &["1"], "a bold group heading");
    report.assert_styled("Established (Scoped to this declaration)", "(Scoped", &["2"], "a dim group note");
    report.assert_styled("geometry.ts#Shape.create 14:3", "geometry.ts#Shape.create", &["1"], "a bold owner");
    report.assert_styled(CALL, "Shape", &["1"], "a bold target");
    report.assert_styled(CALL, "15:16", &["2"], "a dim location");
    if pane.osc8 {
        let row = report.row(CALL);
        assert!(
            has_hyperlink(&row.raw, "/geometry.ts#L15", "15:16"),
            "expected the location linked to geometry.ts#L15.\nrow:\n{:?}",
            row.raw,
        );
    }

    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "refs buildShape --callees geometry.ts");
    const CALLEE: &str = "- Call Shape.create 24:16";
    report.assert_rows_in_order(&["coverage: limited (indirect_dispatch)", CALLEE]);
    report.assert_styled(CALLEE, "— NameOnly", &["2"], "a dim explanation");

    let report = run_hug(harness, pane, &fixture, UNWRAPPED, "refs nowhere geometry.ts");
    assert!(report.has_row("No declaration named nowhere in the scanned files."), "report:\n{}", report.plain());
    assert!(
        has_logical(harness, pane, &report, REFS_BOUNDARY),
        "an empty report states the boundary.\nreport:\n{}",
        report.plain()
    );

    // `hug` wraps the boundary itself at an injected width.
    let report = run_hug(harness, pane, &fixture, "COLUMNS=60", "refs nowhere geometry.ts");
    let rows = shown_rows(&report);
    assert!(rows.len() > 2, "the boundary wraps at 60 columns: {rows:?}");
    assert!(rows.iter().all(|row| row.chars().count() <= 60), "no row past 60 columns: {rows:?}");
    let joined = rows[1..].iter().map(|row| row.trim()).collect::<Vec<_>>().join(" ");
    assert_eq!(joined, REFS_BOUNDARY);
}

// ------------------------------------------------------------------
// tmux — headless; capture keeps SGR but not OSC 8
// ------------------------------------------------------------------

macro_rules! declaration_view_tests {
    ($backend:ident => $run:ident) => {
        mod $backend {
            use super::*;
            #[test]
            #[serial(level2_terminal)]
            fn declarations() { $run(check_declarations); }
            #[test]
            #[serial(level2_terminal)]
            fn wrapping() { $run(check_wrapping); }
            #[test]
            #[serial(level2_terminal)]
            fn table() { $run(check_table); }
            #[test]
            #[serial(level2_terminal)]
            fn outline() { $run(check_outline); }
            #[test]
            #[serial(level2_terminal)]
            fn stats() { $run(check_stats); }
            #[test]
            #[serial(level2_terminal)]
            fn package_stats() { $run(check_package_stats); }
            #[test]
            #[serial(level2_terminal)]
            fn signature_modes() { $run(check_signature_modes); }
            #[test]
            #[serial(level2_terminal)]
            fn classes_table() { $run(check_classes_table); }
            #[test]
            #[serial(level2_terminal)]
            fn refs() { $run(check_refs); }
        }
    };
}

declaration_view_tests!(level2_declarations_in_tmux => run_tmux_view);

fn run_tmux_view(check: fn(&mut TmuxHarness, &Pane<TmuxHarness>)) {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let pane = tmux_pane();
    check(&mut harness, &pane);
}
/// tmux: `-S -` reads the whole history and `-J` rejoins rows tmux
/// soft-wrapped (rows `hug` broke stay apart).
fn tmux_pane() -> Pane<TmuxHarness> {
    Pane {
        osc8: false,
        cols: |harness| harness.pane_cols().expect("tmux pane width") as usize,
        wrap_cols: |_| None,
        capture: |harness| {
            let output = std::process::Command::new("tmux")
                .args(["capture-pane", "-p", "-e", "-J", "-S", "-"])
                .args(["-t", harness.session_name()])
                .output()
                .expect("run tmux capture-pane");
            assert!(output.status.success(), "tmux capture-pane failed");
            String::from_utf8_lossy(&output.stdout).into_owned()
        },
    }
}

// ------------------------------------------------------------------
// WezTerm — SGR and OSC 8
// ------------------------------------------------------------------

declaration_view_tests!(level2_declarations_in_wezterm => run_wezterm_view);

fn run_wezterm_view(check: fn(&mut WezTermHarness, &Pane<WezTermHarness>)) {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    let pane: Pane<WezTermHarness> = Pane {
        osc8: true,
        cols: |harness| harness.pane_size().expect("WezTerm pane size").cols as usize,
        wrap_cols: |harness| Some(harness.pane_size().expect("WezTerm pane size").cols as usize),
        capture: |harness| {
            harness
                .capture_scrollback(WEZTERM_HISTORY_ROWS)
                .expect("capture failed")
                .raw
        },
    };
    check(harness, &pane);
}
// ------------------------------------------------------------------
// Kitty — SGR and OSC 8; `get-text` returns soft-wrapped rows whole
// ------------------------------------------------------------------

declaration_view_tests!(level2_declarations_in_kitty => run_kitty_view);

fn run_kitty_view(check: fn(&mut KittyHarness, &Pane<KittyHarness>)) {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard =
        SHARED_KITTY.get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn Kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    let pane: Pane<KittyHarness> = Pane {
        osc8: true,
        cols: |harness| harness.pane_cols().expect("Kitty pane width") as usize,
        wrap_cols: |_| None,
        capture: |harness| harness.capture_extent("all").expect("capture failed").raw,
    };
    check(harness, &pane);
}

/// Sources of the tags reader fixture, relative to its scan root. A path
/// starting with `../` sits outside the root and is selected with an absolute
/// `--file`, so its tag carries a `../` route.
const READER_SOURCES: &[(&str, &str)] = &[
    ("one.rs", "pub fn duplicate() {}\n"),
    ("two.rs", "// second file\npub fn duplicate() {}\n"),
    ("dir with space/λ.rs", "pub struct Ω;\nimpl Ω {\n    pub fn inside() {}\n}\n"),
    ("../outside.rs", "// outside the scan root\npub fn outside_fn() {}\n"),
    ("bank.php", "<?php\nnamespace Outer\\Inner;\nclass Account {\n    public function deposit() {}\n}\n"),
    (" lead.rs", "// leading space\npub fn leading_space() {}\n"),
    ("pct%.rs", "pub fn percent() {}\n"),
    ("cost$.rs", "// dollar before the extension\npub fn dollar() {}\n"),
    ("hash#.rs", "pub fn hash() {}\n"),
];

/// A backslash is an ordinary file-name character only on Unix.
#[cfg(unix)]
const UNIX_READER_SOURCES: &[(&str, &str)] = &[("back\\slash.rs", "// literal backslash\npub fn backslash() {}\n")];
#[cfg(not(unix))]
const UNIX_READER_SOURCES: &[(&str, &str)] = &[];

/// One reader expectation: `taglist()` finds exactly `matches` entries named
/// `name`, and the `nth` of them (1-based, file order) has `kind`, `class`
/// (`None` = no container field) and line `line`; `:{nth}tag! {name}` then
/// opens `file` (relative to the scan root) with the cursor on `line`.
#[derive(Clone, Copy)]
struct ReaderCase {
    name: &'static str,
    matches: usize,
    nth: usize,
    file: &'static str,
    line: u32,
    kind: &'static str,
    class: Option<&'static str>,
}

const READER_CASES: &[ReaderCase] = &[
    ReaderCase { name: "duplicate", matches: 2, nth: 1, file: "one.rs", line: 1, kind: "function", class: None },
    ReaderCase { name: "duplicate", matches: 2, nth: 2, file: "two.rs", line: 2, kind: "function", class: None },
    ReaderCase { name: "Ω", matches: 1, nth: 1, file: "dir with space/λ.rs", line: 1, kind: "type", class: None },
    ReaderCase { name: "inside", matches: 1, nth: 1, file: "dir with space/λ.rs", line: 3, kind: "method", class: Some("Ω") },
    ReaderCase { name: "outside_fn", matches: 1, nth: 1, file: "../outside.rs", line: 2, kind: "function", class: None },
    // Neovim compares the whole field, so these are exact: 11 and 19 bytes.
    ReaderCase { name: "Outer\\Inner", matches: 1, nth: 1, file: "bank.php", line: 2, kind: "namespace", class: None },
    ReaderCase { name: "deposit", matches: 1, nth: 1, file: "bank.php", line: 4, kind: "method", class: Some("Outer\\Inner\\Account") },
    ReaderCase { name: "leading_space", matches: 1, nth: 1, file: " lead.rs", line: 2, kind: "function", class: None },
    ReaderCase { name: "percent", matches: 1, nth: 1, file: "pct%.rs", line: 1, kind: "function", class: None },
    ReaderCase { name: "dollar", matches: 1, nth: 1, file: "cost$.rs", line: 2, kind: "function", class: None },
    ReaderCase { name: "hash", matches: 1, nth: 1, file: "hash#.rs", line: 1, kind: "function", class: None },
];

#[cfg(unix)]
const UNIX_READER_CASES: &[ReaderCase] =
    &[ReaderCase { name: "backslash", matches: 1, nth: 1, file: "back\\slash.rs", line: 2, kind: "function", class: None }];
#[cfg(not(unix))]
const UNIX_READER_CASES: &[ReaderCase] = &[];

/// Sources whose tags the writer must skip, because Neovim would expand the
/// file name (or drop the pseudo-tag name) and so open another file or lose
/// the entry. Each must leave a warning and exit 1 rather than a tag line.
const SKIPPED_READER_SOURCES: &[(&str, &str, &str)] = &[
    ("a$HOME.rs", "pub fn env_name() {}\n", "file name Neovim would expand"),
    ("a[b].rs", "pub fn bracket() {}\n", "file name Neovim would expand"),
    ("~home.rs", "pub fn tilde() {}\n", "file name Neovim would expand"),
    ("pseudo.sh", "!_TAG_X() { :; }\n", "pseudo-tag name starting with !_"),
];

/// Checks every case in `cases` (defined before this text). Any failed
/// `assert` or Vim error (such as E429 for a missing file) raises, the
/// caller's `pcall` turns it into `cquit 1`, and the completion line is
/// printed only after the last case.
const READER_LUA: &str = r#"
vim.fn.chdir(root)
vim.o.tags = vim.fn.escape(root .. '/tags', ' ,\\')
local function same_file(actual, expected)
  if vim.fn.has('win32') == 1 then
    return actual:lower() == expected:lower()
  end
  return actual == expected
end
for index, case in ipairs(cases) do
  local where = ('case %d (%s #%d)'):format(index, case.name, case.nth)
  local entries = vim.fn.taglist('\\V\\^' .. vim.fn.escape(case.name, '\\') .. '\\$')
  assert(#entries == case.matches, ('%s: %d taglist entries, expected %d'):format(where, #entries, case.matches))
  local entry = entries[case.nth]
  assert(entry.cmd == tostring(case.line), ('%s: address %s, expected %d'):format(where, entry.cmd, case.line))
  assert(entry.kind == case.kind, ('%s: kind %s, expected %s'):format(where, tostring(entry.kind), case.kind))
  assert(entry.class == case.class, ('%s: class %s, expected %s'):format(where, tostring(entry.class), tostring(case.class)))
  vim.cmd('silent! %bwipeout!')
  vim.cmd({ cmd = 'tag', args = { case.name }, range = { case.nth }, bang = true, mods = { silent = true } })
  local opened = vim.fs.normalize(vim.fn.resolve(vim.fn.expand('%:p')))
  assert(same_file(opened, case.file), ('%s: opened %s, expected %s'):format(where, opened, case.file))
  assert(vim.fn.line('.') == case.line, ('%s: cursor line %d, expected %d'):format(where, vim.fn.line('.'), case.line))
end
io.stdout:write(('tags reader verified %d cases\n'):format(#cases))
"#;

/// `value` as a Lua string literal; Lua strings are bytes, so UTF-8 passes
/// through unchanged.
fn lua_string(value: &str) -> String {
    let mut literal = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => literal.push_str("\\\\"),
            '"' => literal.push_str("\\\""),
            control if control.is_control() => {
                for byte in control.to_string().bytes() {
                    literal.push_str(&format!("\\{byte:03}"));
                }
            }
            other => literal.push(other),
        }
    }
    literal.push('"');
    literal
}

/// The canonical path the reader should open, `/`-separated and without a
/// Windows verbatim prefix (macOS temp dirs canonicalize `/var` to
/// `/private/var`, which Neovim's `resolve()` also does). On Unix a `\` is a
/// file-name character that the portable spelling would make a separator.
fn reader_path(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).expect("canonical reader fixture path");
    if cfg!(unix) {
        return canonical.to_string_lossy().into_owned();
    }
    let portable = biscuit_file::to_portable_string(&canonical);
    portable.strip_prefix("//?/").map(str::to_owned).unwrap_or(portable)
}

/// Runs the reader over `cases` in headless Neovim with no config, no
/// shada and no window. Lua assertion failures do not change Neovim's exit
/// status by themselves (E5108, exit 0), so the script runs under `pcall`
/// and a failure exits through `cquit 1`.
fn run_tags_reader(root: &Path, scratch: &Path, cases: &[ReaderCase]) -> std::process::Output {
    let rows: Vec<String> = cases
        .iter()
        .map(|case| {
            format!(
                "{{ name = {}, matches = {}, nth = {}, file = {}, line = {}, kind = {}, class = {} }}",
                lua_string(case.name),
                case.matches,
                case.nth,
                lua_string(&reader_path(&root.join(case.file))),
                case.line,
                lua_string(case.kind),
                case.class.map_or_else(|| "nil".to_string(), lua_string),
            )
        })
        .collect();
    let script = format!(
        "local root = {}\nlocal cases = {{\n  {}\n}}\n{READER_LUA}",
        lua_string(&reader_path(root)),
        rows.join(",\n  "),
    );
    let script_path = scratch.join("tags-reader.lua");
    std::fs::write(&script_path, script).expect("write reader script");
    let run = format!(
        "lua local ok, err = pcall(dofile, {}); if not ok then io.stderr:write(tostring(err) .. '\\n'); vim.cmd('cquit 1') end",
        lua_string(&biscuit_file::to_portable_string(&script_path)),
    );
    std::process::Command::new("nvim")
        .args(["--headless", "-u", "NONE", "-i", "NONE", "-n", "-c", &run, "-c", "qa!"])
        .current_dir(root)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run headless nvim")
}

#[test]
#[serial(level2_terminal)]
fn level2_ctags_reader_resolves_duplicates_and_numeric_lines_headlessly() {
    let available = std::process::Command::new("nvim").arg("--version").output().is_ok();
    require_level!(Level::L2, available, "Neovim");
    let fixture = Fixture::empty();
    let root = fixture.repo.path().join("root");
    let mut args = vec!["symbols".to_string(), "--format".into(), "ctags".into(), "--file".into(), "**/*.rs".into()];
    args.extend(["--file".to_string(), "*.php".into()]);
    for (relative, source) in READER_SOURCES.iter().chain(UNIX_READER_SOURCES) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, source).unwrap();
        if let Some(outside) = relative.strip_prefix("../") {
            args.push("--file".into());
            args.push(root.parent().unwrap().join(outside).to_string_lossy().into_owned());
        }
    }
    let tags = run_hug_tags(&fixture, &root, &args);
    assert!(tags.status.success(), "{}", String::from_utf8_lossy(&tags.stderr));
    let tags_text = String::from_utf8(tags.stdout).expect("UTF-8 tags");
    // Saved at the scan root, where its root-relative paths resolve.
    std::fs::write(root.join("tags"), &tags_text).unwrap();

    let cases: Vec<ReaderCase> = READER_CASES.iter().chain(UNIX_READER_CASES).copied().collect();
    let reader = run_tags_reader(&root, fixture.temp.path(), &cases);
    let stdout = String::from_utf8_lossy(&reader.stdout);
    assert!(
        reader.status.success() && stdout.contains(&format!("tags reader verified {} cases", cases.len())),
        "reader failed ({:?})\nstdout: {stdout}\nstderr: {}\ntags:\n{tags_text}",
        reader.status,
        String::from_utf8_lossy(&reader.stderr),
    );

    // The harness itself: a wrong expectation, in the metadata or in the
    // navigation, must fail the reader.
    for (wrong, message) in [
        (ReaderCase { line: 1, ..READER_CASES[1] }, "address 2, expected 1"),
        (ReaderCase { file: "one.rs", ..READER_CASES[1] }, "opened "),
    ] {
        let failed = run_tags_reader(&root, fixture.temp.path(), &[wrong]);
        let stderr = String::from_utf8_lossy(&failed.stderr);
        assert!(!failed.status.success(), "a wrong expectation passed the reader: {stderr}");
        assert!(stderr.contains(message), "expected `{message}` in the reader failure: {stderr}");
        assert!(!String::from_utf8_lossy(&failed.stdout).contains("tags reader verified"));
    }

    let skipped_root = fixture.repo.path().join("skipped");
    for (relative, source, _) in SKIPPED_READER_SOURCES {
        let path = skipped_root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, source).unwrap();
    }
    let skipped = run_hug_tags(&fixture, &skipped_root, &["symbols".into(), "--format".into(), "ctags".into()]);
    let skipped_tags = String::from_utf8(skipped.stdout).expect("UTF-8 tags");
    let warnings = String::from_utf8_lossy(&skipped.stderr);
    assert_eq!(skipped.status.code(), Some(1), "{warnings}");
    assert_eq!(skipped_tags, ctags_header(), "a skipped entry reached the tags file");
    for (relative, _, reason) in SKIPPED_READER_SOURCES {
        let warning = format!("warning: ctags skipped {reason} in {relative} at line 1");
        assert!(warnings.lines().any(|line| line == warning), "missing `{warning}` in:\n{warnings}");
    }
}

fn ctags_header() -> &'static str {
    "!_TAG_FILE_FORMAT\t2\t/extended format/\n!_TAG_FILE_SORTED\t0\t/0=unsorted/\n"
}

/// Runs `hug` in `root` with the fixture's isolated home, config and temp.
fn run_hug_tags(fixture: &Fixture, root: &Path, args: &[String]) -> std::process::Output {
    std::process::Command::new(hug())
        .args(args)
        .current_dir(root)
        .env("HOME", fixture.home.path())
        .env("USERPROFILE", fixture.home.path())
        .env("XDG_CONFIG_HOME", fixture.home.path().join(".config"))
        .env("TMPDIR", fixture.temp.path())
        .env("TMP", fixture.temp.path())
        .env("TEMP", fixture.temp.path())
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
}
