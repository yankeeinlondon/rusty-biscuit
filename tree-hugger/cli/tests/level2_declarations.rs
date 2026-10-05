//! Level 2 tests for the declaration commands (`functions`, `types`,
//! `symbols`, `classes`) rendered in a real terminal.
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
use terminal_report::{Report, run_scoped};
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
        let fixture = Self {
            repo: TempDir::new().expect("scratch repo"),
            home: TempDir::new().expect("scratch home"),
            temp: TempDir::new().expect("scratch temp"),
        };
        copy_tree(
            &biscuit_test_harness::manifest_dir!().join("tests/fixtures/declarations"),
            fixture.repo.path(),
        );
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
            row.raw.contains("/geometry.ts#L14\x1b\\geometry.ts:14:3"),
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
            row.raw.contains("/geometry.ts#L10\x1b\\[10:8]"),
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
            report.raw().contains("/geometry.ts#L14\x1b\\geometry.ts:14"),
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

// ------------------------------------------------------------------
// tmux — headless; capture keeps SGR but not OSC 8
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_declarations_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let pane = tmux_pane();
    check_declarations(&mut harness, &pane);
    check_wrapping(&mut harness, &pane);
    check_table(&mut harness, &pane);
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

#[test]
#[serial(level2_terminal)]
fn level2_declarations_in_wezterm() {
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
    check_declarations(harness, &pane);
    check_wrapping(harness, &pane);
    check_table(harness, &pane);
}

// ------------------------------------------------------------------
// Kitty — SGR and OSC 8; `get-text` returns soft-wrapped rows whole
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_declarations_in_kitty() {
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
    check_declarations(harness, &pane);
    check_wrapping(harness, &pane);
    check_table(harness, &pane);
}
