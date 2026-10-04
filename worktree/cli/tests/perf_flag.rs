//! Integration tests for the `wt list --perf` runtime performance report.

mod perf_support;

use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use perf_support::{PerfRow, perf_rows, stage_from_perf};

fn temp_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path();

    run_git(path, &["init", "-b", "main"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    run_git(path, &["config", "commit.gpgsign", "false"]);

    fs::write(path.join("README.md"), "# test\n").unwrap();
    run_git(path, &["add", "README.md"]);
    run_git(path, &["commit", "-m", "initial"]);

    dir
}

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {:?} failed in {:?}", args, repo);
}

#[test]
fn list_perf_emits_report_on_stderr_and_empty_stdout() {
    let repo = temp_repo();

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .args(["list", "--perf"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Performance"))
        .stderr(predicate::str::contains("pre-dispatch"))
        .stderr(predicate::str::contains("list gather"))
        .stderr(predicate::str::contains("table render"));
}

#[test]
fn list_without_perf_emits_no_report() {
    let repo = temp_repo();

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Performance").not())
        .stderr(predicate::str::contains("pre-dispatch").not())
        .stderr(predicate::str::contains("list gather").not());
}

fn temp_repo_with_feature_branch() -> tempfile::TempDir {
    let dir = temp_repo();
    let path = dir.path();

    fs::write(path.join("file.txt"), "2\n").unwrap();
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "commit 2"]);

    run_git(path, &["checkout", "-b", "feature-a"]);
    fs::write(path.join("a.txt"), "a\n").unwrap();
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "feature a"]);
    run_git(path, &["checkout", "main"]);

    dir
}

#[test]
fn list_perf_non_image_terminal_omits_graph_stages() {
    let repo = temp_repo();

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .env_remove("TERM_PROGRAM")
        .env_remove("KITTY_WINDOW_ID")
        .args(["list", "--perf"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("pre-dispatch"))
        .stderr(predicate::str::contains("list gather"))
        .stderr(predicate::str::contains("table render"))
        .stderr(predicate::str::contains("graph gather").not())
        .stderr(predicate::str::contains("graph image render").not());
}

#[test]
fn list_perf_non_image_verbose_includes_verbose_gather() {
    let repo = temp_repo_with_feature_branch();
    let repo_path = repo.path();
    let feature_path = repo_path
        .parent()
        .expect("temp dir has a parent")
        .join(format!(
            "{}-feature",
            repo_path.file_name().unwrap().to_string_lossy()
        ));
    run_git(repo_path, &["worktree", "add", feature_path.to_str().unwrap(), "feature-a"]);

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(&feature_path)
        .env_remove("TERM_PROGRAM")
        .env_remove("KITTY_WINDOW_ID")
        .args(["list", "-v", "--perf"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("pre-dispatch"))
        .stderr(predicate::str::contains("list gather"))
        .stderr(predicate::str::contains("table render"))
        .stderr(predicate::str::contains("verbose gather"))
        .stderr(predicate::str::contains("verbose render"))
        .stderr(predicate::str::contains("graph gather").not())
        .stderr(predicate::str::contains("graph image render").not());
}

#[test]
fn list_perf_error_path_emits_no_report() {
    let dir = tempfile::tempdir().expect("create temp dir");

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(dir.path())
        .args(["list", "--perf"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Performance").not());
}

/// The shape `wt list --perf` renders for a listing with remote work: the
/// group's label contains its first child's.
const NESTED: &str = "\
\u{1b}[33m▌\u{1b}[0m \u{1b}[1mPerformance\u{1b}[0m                      330.0ms  100%
\u{1b}[33m▌\u{1b}[0m ├─ pr gather                       11.7ms    4%
\u{1b}[33m▌\u{1b}[0m ├─ remote wait ‖ local gather     300.2ms   91%
\u{1b}[33m▌\u{1b}[0m │  ├─ remote wait                 290.1ms     —
\u{1b}[33m▌\u{1b}[0m │  ├─ pr reread                     0.3ms     —
\u{1b}[33m▌\u{1b}[0m │  ├─ list gather                 150.0ms     —
\u{1b}[33m▌\u{1b}[0m │  └─ graph gather                175.0ms     —
\u{1b}[33m▌\u{1b}[0m ├─ table render                     0.5ms   \u{1b}[2m<1%\u{1b}[0m
\u{1b}[33m▌\u{1b}[0m └─ unattributed                    17.6ms    5%
";

#[test]
fn the_stage_reader_picks_the_nested_child_never_the_group_containing_its_name() {
    assert_eq!(stage_from_perf(NESTED, "remote wait"), Some(Duration::from_micros(290_100)));
    assert_eq!(stage_from_perf(NESTED, "remote wait ‖ local gather"), Some(Duration::from_micros(300_200)));
    assert_eq!(stage_from_perf(NESTED, "list gather"), Some(Duration::from_micros(150_000)));
    assert_eq!(stage_from_perf(NESTED, "gather"), None, "labels match whole, never as substrings");
}

#[test]
fn report_rows_carry_their_depth() {
    let shape: Vec<_> = perf_rows(NESTED).into_iter().map(|row| (row.depth, row.label)).collect();
    let expected = [
        (0, "Performance"),
        (1, "pr gather"),
        (1, "remote wait ‖ local gather"),
        (2, "remote wait"),
        (2, "pr reread"),
        (2, "list gather"),
        (2, "graph gather"),
        (1, "table render"),
        (1, "unattributed"),
    ];
    assert_eq!(shape, expected.map(|(depth, label)| (depth, label.to_string())));
}

/// A real local-only report: one `local gather` group whose children are the
/// overlapping gathers, no `remote wait` row, and top-level rows that add up
/// to the total (within the report's display rounding).
#[test]
fn list_perf_reports_a_local_only_group_that_reconciles() {
    let repo = temp_repo();

    let output = assert_cmd::Command::cargo_bin("wt")
        .unwrap()
        .current_dir(repo.path())
        .args(["list", "--perf"])
        .output()
        .expect("wt list --perf runs");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);

    let rows = perf_rows(&stderr);
    let group = rows.iter().position(|row| row.label == "local gather").unwrap_or_else(|| panic!("{rows:#?}"));
    assert_eq!(rows[group].depth, 1, "{rows:#?}");
    let children: Vec<&str> =
        rows[group + 1..].iter().take_while(|row| row.depth == 2).map(|row| row.label.as_str()).collect();
    assert_eq!(children, ["list gather"], "{rows:#?}");
    assert!(rows.iter().all(|row| row.label != "remote wait"), "no remote work, no remote wait row: {rows:#?}");
    assert!(stage_from_perf(&stderr, "list gather").is_some());

    let root = rows.iter().find(|row| row.depth == 0).expect("root row").duration;
    let top_level: Vec<&PerfRow> = rows.iter().filter(|row| row.depth == 1).collect();
    assert_eq!(top_level.last().map(|row| row.label.as_str()), Some("unattributed"), "{rows:#?}");
    let sum: Duration = top_level.iter().map(|row| row.duration).sum();
    // Each printed value is rounded to a tenth of its unit.
    let rounding = |d: Duration| if d >= Duration::from_secs(1) { Duration::from_millis(50) } else { Duration::from_micros(50) };
    let slack: Duration = top_level.iter().map(|row| rounding(row.duration)).sum::<Duration>() + rounding(root);
    assert!(sum.abs_diff(root) <= slack, "top-level rows {sum:?} vs total {root:?}: {rows:#?}");
}
