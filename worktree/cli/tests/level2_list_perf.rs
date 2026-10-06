//! Level 2 tests for the `wt list --perf` human report in a real terminal
//! (tmux).
//!
//! The report's rows, shares, and remainders are fixed by Level 1 tests on
//! synthetic trees (`perf::tests`); these scenes prove what only a terminal
//! shows: that the report the shipped command writes to a real pane keeps its
//! tree connectors, `[n git]` suffixes, and right-aligned duration and share
//! columns at 80 and 120 columns without wrapping, that sequential rows carry
//! percentages while concurrent children and every worker diagnostic row,
//! its heading included, carry `—`. The fixture's `origin` is a local bare
//! repository, so the refresh worker runs a real head check with no network.
//! Durations vary between runs, so only structure is asserted.

mod perf_support;
mod remote_fixture;

use std::process::Command;
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use remote_fixture::Fixture;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

/// The diagnostic heading's fixed text, before its status word.
const WORKER_HEADING: &str = "Refresh worker (diagnostic, measured in the worker):";

/// `wt list --perf` from the linked worktree in a fresh pane exactly `cols`
/// wide, with the graph path active when `graph` is set (tmux cannot show
/// the image, but the history is still gathered and timed). Returns the
/// report's rows, gutter included, once the prompt is back.
fn perf_report_rows(fixture: &Fixture, cols: u32, graph: bool) -> Vec<String> {
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    harness.resize(cols, 60).expect("resize the pane");
    assert_eq!(harness.pane_cols().expect("pane width"), cols, "the pane took the requested width");

    let root = fixture.root.path();
    let home = root.join("home").display().to_string();
    let cache = root.join("cache").display().to_string();
    let empty_config = root.join("empty.gitconfig").display().to_string();
    let mut env = vec![
        ("HOME", home.as_str()),
        ("XDG_CACHE_HOME", cache.as_str()),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_GLOBAL", empty_config.as_str()),
        // `origin` is a local path; a refused proxy and disallowed HTTP
        // transports keep anything else off the network.
        ("HTTPS_PROXY", "http://127.0.0.1:9"),
        ("HTTP_PROXY", "http://127.0.0.1:9"),
        ("GIT_CONFIG_COUNT", "2"),
        ("GIT_CONFIG_KEY_0", "protocol.http.allow"),
        ("GIT_CONFIG_VALUE_0", "never"),
        ("GIT_CONFIG_KEY_1", "protocol.https.allow"),
        ("GIT_CONFIG_VALUE_1", "never"),
        ("FORCE_COLOR", "1"),
        ("COLORFGBG", "15;0"),
    ];
    if graph {
        env.push(("TERM_PROGRAM", "ghostty"));
    }
    let unset_term_program = if graph { "" } else { "-u TERM_PROGRAM" };
    let bin = cargo_bin("wt").display().to_string();
    harness
        .send_text(format!("cd '{}'\n", fixture.linked.display()).as_bytes())
        .expect("send cd failed");
    harness
        .send_command_with_env(
            &format!(
                "env {unset_term_program} -u KITTY_WINDOW_ID -u GH_TOKEN -u GITHUB_TOKEN -u GITEA_TOKEN \
                 -u FORGEJO_TOKEN -u CODEBERG_TOKEN -u WT_SHELL_WRAPPER {bin} list --perf"
            ),
            &env,
        )
        .expect("send wt list --perf failed");

    // The report is written in one piece after the listing, so once its last
    // foreground row shows, the whole report is on screen.
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let frame = harness.capture().expect("capture failed");
        if frame.plain.contains("assemble and write output") {
            break;
        }
        assert!(Instant::now() < deadline, "the report never showed.\nplain:\n{}", frame.plain);
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    let plain = capture_with_scrollback(&harness).plain;

    let lines: Vec<&str> = plain.lines().collect();
    let start = lines
        .iter()
        .rposition(|line| line.starts_with('▌') && line.contains("Performance"))
        .unwrap_or_else(|| panic!("no Performance heading.\nplain:\n{plain}"));
    // A row wider than the pane would wrap onto a line without the gutter,
    // ending the block before the worker section.
    let rows: Vec<String> =
        lines[start..].iter().take_while(|line| line.starts_with('▌')).map(|line| line.trim_end().to_string()).collect();
    for row in &rows {
        assert!(row.chars().count() <= cols as usize, "a row overflows {cols} columns: {row:?}\n{plain}");
    }
    rows
}

/// The pane including scrollback, so the table above the report never pushes
/// the report's heading out of the capture.
fn capture_with_scrollback(harness: &TmuxHarness) -> CapturedFrame {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", harness.session_name(), "-p", "-e", "-S", "-200", "-E", "-"])
        .output()
        .expect("tmux capture-pane should succeed");
    CapturedFrame::from_raw(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// One report row split into its columns, with the char columns at which the
/// duration's mantissa and the share end.
#[derive(Debug)]
struct Row {
    /// Connector prefix and label, gutter removed.
    label: String,
    share: String,
    mantissa_end: usize,
    share_end: usize,
}

impl Row {
    fn parse(line: &str) -> Self {
        let text = line.strip_prefix("▌ ").unwrap_or_else(|| panic!("no gutter: {line:?}"));
        let chars: Vec<char> = text.chars().collect();
        let token_before = |end: usize| {
            let end = chars[..end].iter().rposition(|c| !c.is_whitespace()).map_or(0, |i| i + 1);
            let start = chars[..end].iter().rposition(|c| c.is_whitespace()).map_or(0, |i| i + 1);
            (start, end)
        };
        let (share_start, share_end) = token_before(chars.len());
        let (value_start, value_end) = token_before(share_start);
        let value: String = chars[value_start..value_end].iter().collect();
        let mantissa_len = value.chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '—').count();
        assert!(mantissa_len > 0, "no duration in {line:?}");
        let gutter = 2;
        Self {
            label: chars[..value_start].iter().collect::<String>().trim_end().to_string(),
            share: chars[share_start..share_end].iter().collect(),
            mantissa_end: gutter + value_start + mantissa_len,
            share_end: gutter + share_end,
        }
    }

    fn is_percentage(&self) -> bool {
        self.share == "<1%"
            || self.share.strip_suffix('%').is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    }

    /// The label without its tree connectors.
    fn name(&self) -> &str {
        self.label.trim_start_matches(['│', '├', '└', '─', ' '])
    }

    /// Tree depth: each level is one three-column connector.
    fn depth(&self) -> usize {
        (self.label.chars().count() - self.name().chars().count()) / 3
    }
}

/// Whether `label` ends in a `[n git]` suffix.
fn has_git_count(label: &str) -> bool {
    label
        .rsplit_once("  [")
        .and_then(|(_, suffix)| suffix.strip_suffix(" git]"))
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

/// The direct children of the row named `name` (with or without its git
/// suffix), checked to hang from it by `├─` connectors closed by one `└─`.
fn children_of<'a>(section: &'a [Row], name: &str, report: &str) -> Vec<&'a Row> {
    let parent = section
        .iter()
        .position(|row| row.name() == name || row.name().strip_prefix(name).is_some_and(|rest| rest.starts_with("  [")))
        .unwrap_or_else(|| panic!("no `{name}` row:\n{report}"));
    let depth = section[parent].depth();
    let children: Vec<&Row> = section[parent + 1..]
        .iter()
        .take_while(|row| row.depth() > depth)
        .filter(|row| row.depth() == depth + 1)
        .collect();
    assert!(!children.is_empty(), "`{name}` has children:\n{report}");
    for (index, child) in children.iter().enumerate() {
        let connector = if index + 1 == children.len() { "└─ " } else { "├─ " };
        let own: String = child.label.chars().skip(depth * 3).take(3).collect();
        assert_eq!(own, connector, "`{}` under `{name}`:\n{report}", child.name());
    }
    children
}

/// Every row's duration mantissa and share end in one column each.
fn assert_aligned(section: &[Row], what: &str, report: &str) {
    for row in section {
        assert_eq!(row.mantissa_end, section[0].mantissa_end, "{what}: duration column of {row:?}\n{report}");
        assert_eq!(row.share_end, section[0].share_end, "{what}: share column of {row:?}\n{report}");
    }
}

/// The structure of one rendered report; `graph` when the history was read.
fn assert_report(lines: &[String], graph: bool) {
    let report = lines.join("\n");
    let rows: Vec<Row> = lines.iter().map(|line| Row::parse(line)).collect();
    let split = rows
        .iter()
        .position(|row| row.label.starts_with(WORKER_HEADING))
        .unwrap_or_else(|| panic!("no worker diagnostics section:\n{report}"));
    let (foreground, worker) = rows.split_at(split);

    // The foreground tree: a total, sequential top-level steps with
    // percentages, one closing connector.
    assert_eq!((foreground[0].label.as_str(), foreground[0].share.as_str()), ("Performance", "100%"), "{report}");
    let top: Vec<&Row> = foreground[1..].iter().filter(|row| !row.label.starts_with('│')).collect();
    for row in &top {
        assert!(row.label.starts_with("├─ ") || row.label.starts_with("└─ "), "a top-level connector: {row:?}\n{report}");
        assert!(row.is_percentage(), "a sequential step shows its share: {row:?}\n{report}");
    }
    assert_eq!(top.iter().filter(|row| row.label.starts_with("└─ ")).count(), 1, "{report}");
    assert!(top.last().unwrap().label.starts_with("└─ "), "the last top-level row closes the tree:\n{report}");
    let top_labels: Vec<&str> = top.iter().map(|row| row.name()).collect();
    for expected in ["startup", "table render", "assemble and write output"] {
        assert!(top_labels.iter().any(|label| label.starts_with(expected)), "`{expected}` at the top:\n{report}");
    }
    for counted in ["read worktrees and refs", "refresh worker ‖ local reads"] {
        let row = top
            .iter()
            .find(|row| row.name().starts_with(counted))
            .unwrap_or_else(|| panic!("no `{counted}` row:\n{report}"));
        assert!(has_git_count(&row.label), "`{counted}` carries a git count: {row:?}\n{report}");
    }

    // The concurrent group: its children show no share, but a sequential
    // child's own children (the launch steps, the graph history's steps) do.
    let children = |name: &str| children_of(foreground, name, &report);
    let group = children("refresh worker ‖ local reads");
    let group_names: Vec<&str> = group.iter().map(|row| row.name()).collect();
    let mut expected_children = vec!["launch and wait for refresh", "local listing facts"];
    if graph {
        expected_children.push("graph history");
    }
    for expected in expected_children {
        assert!(group_names.iter().any(|name| name.starts_with(expected)), "`{expected}` in the group:\n{report}");
    }
    for child in group.iter().chain(&children("local listing facts")) {
        assert_eq!(child.share, "—", "a concurrent child shows no share: {child:?}\n{report}");
    }
    let launch = children("launch and wait for refresh");
    assert!(launch.iter().any(|row| row.name() == "launch worker"), "the launch step:\n{report}");
    let history = if graph { children("graph history") } else { Vec::new() };
    assert_eq!(graph, !history.is_empty(), "the history read's steps:\n{report}");
    for row in launch.iter().chain(&history) {
        assert!(row.is_percentage(), "a sequential parent's child shows its share: {row:?}\n{report}");
    }

    // The worker diagnostics: measured elsewhere, so no row, heading
    // included, shows a share of anything.
    let heading = &worker[0];
    assert_eq!(heading.label, format!("{WORKER_HEADING} complete"), "{report}");
    assert!(!lines[split].contains('%'), "the worker heading shows no percentage: {:?}\n{report}", lines[split]);
    for row in worker {
        assert_eq!(row.share, "—", "a worker row shows no share: {row:?}\n{report}");
    }
    assert!(worker.iter().any(|row| row.label == "└─ launch 0 (complete)"), "{report}");
    assert!(worker.iter().any(|row| row.label.ends_with("─ head check")), "the worker's head check:\n{report}");

    assert_aligned(foreground, "foreground", &report);
    assert_aligned(worker, "worker diagnostics", &report);
}

/// The report at 80 and 120 columns on a plain terminal: the hierarchy,
/// shares, git counts, and worker diagnostics, each in aligned columns.
#[test]
#[serial(level2_terminal)]
fn level2_list_perf_report_keeps_its_tree_and_columns_at_80_and_120_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = Fixture::new();
    for cols in [80, 120] {
        let rows = perf_report_rows(&fixture, cols, false);
        assert_report(&rows, false);
    }
}

/// With the graph path active, the history read is a concurrent child of the
/// local reads and the report keeps the same structure.
#[test]
#[serial(level2_terminal)]
fn level2_list_perf_report_shows_the_graph_history_read_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = Fixture::new();
    let rows = perf_report_rows(&fixture, 100, true);
    assert_report(&rows, true);
}
