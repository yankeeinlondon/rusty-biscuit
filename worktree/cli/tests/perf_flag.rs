//! Integration tests for the `wt list --perf` runtime performance report.

mod perf_support;

use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use perf_support::{FakeGitea, GiteaReply, MixedFixture, WorkerReaper, perf_rows, perf_timings, stage_at, stage_from_perf};
use serial_test::serial;
use worktree::timing::{Scope, Span, Stage, Timings};

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
        .stderr(predicate::str::contains("startup"))
        .stderr(predicate::str::contains("local listing facts"))
        .stderr(predicate::str::contains("table render"))
        .stderr(predicate::str::contains("WT_PERF_JSON").not());
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
        .stderr(predicate::str::contains("local listing facts").not())
        .stderr(predicate::str::contains("WT_PERF_JSON").not());
}

/// `wt list --perf=json` from `dir` without image support: its stdout must
/// be empty, and its stderr's final line is the record.
fn json_report(dir: &Path, args: &[&str]) -> (Timings, String) {
    let output = assert_cmd::Command::cargo_bin("wt")
        .unwrap()
        .current_dir(dir)
        .env_remove("TERM_PROGRAM")
        .env_remove("KITTY_WINDOW_ID")
        .args(["list", "--perf=json"])
        .args(args)
        .output()
        .expect("wt list --perf=json runs");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{stderr}");
    assert!(output.stdout.is_empty(), "stdout stays empty: {:?}", String::from_utf8_lossy(&output.stdout));
    (perf_timings(&stderr), stderr)
}

/// Every stage anywhere in `timings`.
fn all_stages(timings: &Timings) -> Vec<Stage> {
    fn walk(spans: &[Span], out: &mut Vec<Stage>) {
        for span in spans {
            out.push(span.stage());
            walk(span.children(), out);
        }
    }
    let mut out = Vec::new();
    walk(timings.spans(), &mut out);
    out
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

    let (timings, _) = json_report(repo.path(), &[]);

    let stages = all_stages(&timings);
    for present in [Stage::Startup, Stage::LocalGather, Stage::TableRender, Stage::WriteOutput] {
        assert!(stages.contains(&present), "{present} missing: {stages:?}");
    }
    for absent in [Stage::GraphHistory, Stage::GraphRender] {
        assert!(!stages.contains(&absent), "{absent} without image support: {stages:?}");
    }
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

    let (timings, _) = json_report(&feature_path, &["-v"]);

    assert!(timings.span(&[Stage::LocalReads, Stage::VerboseHistory]).is_some(), "{timings:#?}");
    assert!(timings.span(&[Stage::VerboseRender]).is_some(), "{timings:#?}");
    let stages = all_stages(&timings);
    assert!(!stages.contains(&Stage::GraphHistory) && !stages.contains(&Stage::GraphRender), "{stages:?}");
}

#[test]
fn list_perf_error_path_emits_no_report() {
    let dir = tempfile::tempdir().expect("create temp dir");

    for flag in ["--perf", "--perf=json"] {
        assert_cmd::Command::cargo_bin("wt").unwrap()
            .current_dir(dir.path())
            .args(["list", flag])
            .assert()
            .failure()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("Performance").not())
            .stderr(predicate::str::contains("WT_PERF_JSON").not());
    }
}

/// `--perf` takes its value only with `=`: bare `--perf` is the human report,
/// `--perf json` leaves `json` to be read as a subcommand, and an unknown
/// value is a usage error.
#[test]
fn the_perf_value_needs_an_equals_sign_and_a_known_format() {
    let repo = temp_repo();
    let wt = || {
        let mut command = assert_cmd::Command::cargo_bin("wt").unwrap();
        command.current_dir(repo.path()).env_remove("TERM_PROGRAM").env_remove("KITTY_WINDOW_ID");
        command
    };

    wt().args(["--perf", "list"]).assert().success().stderr(predicate::str::contains("Performance"));
    wt().args(["--perf=human", "list"]).assert().success().stderr(predicate::str::contains("Performance"));
    wt().args(["--perf", "json"]).assert().code(2).stderr(predicate::str::contains("json"));
    wt().args(["list", "--perf=yaml"]).assert().code(2).stderr(predicate::str::contains("yaml"));
}

/// The record follows the listing: a newline, then one line that is the
/// whole report, and nothing after it.
#[test]
fn the_json_record_is_the_final_line_after_the_listing() {
    let repo = temp_repo();

    let (timings, stderr) = json_report(repo.path(), &[]);

    assert_eq!(timings.scope(), Scope::Command);
    let (listing, record) = stderr.trim_end_matches('\n').rsplit_once('\n').expect("a listing, then the record");
    assert!(listing.ends_with('\n'), "a newline separates the listing from the record: {stderr:?}");
    assert!(listing.contains("main"), "the listing comes first: {listing}");
    assert!(record.starts_with("WT_PERF_JSON {"), "{record:?}");
    assert!(stderr.ends_with("}\n"), "{stderr:?}");
    assert!(!record.contains('\u{1b}') && !record.contains('\r'), "{record:?}");
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

#[test]
fn list_perf_reports_a_local_only_group_that_reconciles() {
    let repo = temp_repo();

    let (timings, _) = json_report(repo.path(), &[]);

    let region = timings.span(&[Stage::LocalReads]).expect("a local-only region");
    assert_eq!(region.children().iter().map(Span::stage).collect::<Vec<_>>(), [Stage::LocalGather]);
    assert!(timings.span(&[Stage::RemoteAndLocal]).is_none(), "no wait ran: {timings:#?}");
    // The record decoded, so every level reconciled exactly.
    assert_eq!(timings.spans().first().map(Span::stage), Some(Stage::Startup));
    assert_eq!(timings.spans().last().map(Span::stage), Some(Stage::WriteOutput));
}

/// A real report with remote work, from the shipped command: the wait, the
/// post-wait PR read, and the local gather sit in one concurrent
/// `remote_and_local` region, `origin_lookup` stays a top-level step, and
/// nothing is regathered when no ref moved.
#[test]
#[serial]
fn list_perf_reports_the_remote_group_from_the_real_renderer() {
    let fixture = MixedFixture::new().with_gitea_origin();
    let gitea = FakeGitea::new(GiteaReply::Open(vec![]));
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    let output =
        fixture.wt_command_via_gitea(&gitea).args(["list", "--perf=json"]).output().expect("wt list --perf=json runs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    let timings = perf_timings(&stderr);

    let region = timings.span(&[Stage::RemoteAndLocal]).expect("the wait's region");
    assert_eq!(
        region.children().iter().map(Span::stage).collect::<Vec<_>>(),
        [Stage::RefreshWorker, Stage::PrCacheRead, Stage::LocalGather]
    );
    let top_level: Vec<Stage> = timings.spans().iter().map(Span::stage).collect();
    assert!(top_level.contains(&Stage::OriginLookup), "{top_level:?}");
    for absent in [Stage::LocalReads, Stage::Regather, Stage::FastForward, Stage::CheckoutRefresh] {
        assert!(!top_level.contains(&absent), "`{absent}` without its cause: {top_level:?}");
    }
    let wait = stage_at(&timings, &[Stage::RemoteAndLocal, Stage::RefreshWorker]).expect("refresh worker");
    assert!(wait <= region.elapsed(), "the region spans its wait: {wait:?} > {:?}", region.elapsed());
}
