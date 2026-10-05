//! Integration tests for the `wt list --perf` runtime performance report.

mod perf_support;

use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use perf_support::{FakeGitea, GiteaReply, MixedFixture, WorkerReaper, perf_timings, stage_at};
use serial_test::serial;
use worktree::timing::{Scope, Span, SpanList, Stage, Timings, WorkerReportStatus};

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

/// A well-formed record no real listing writes: one `startup` span.
fn decoy_record() -> String {
    let mut spans = SpanList::sequential();
    spans.push(Span::new(Stage::Startup, Duration::from_micros(3)));
    format!("WT_PERF_JSON {}", Timings::new(Scope::Command, Duration::from_micros(7), spans).to_json())
}

/// Whether [`perf_timings`] refuses `output`.
fn refused(output: &str) -> bool {
    std::panic::catch_unwind(|| perf_timings(output)).is_err()
}

/// The reader takes the final nonempty line, split on LF or CRLF, and
/// nothing earlier: a decoy record above it is never read.
#[test]
fn the_reader_takes_only_the_final_record_with_lf_or_crlf() {
    let repo = temp_repo();
    let (real, _) = json_report(repo.path(), &[]);
    let record = format!("WT_PERF_JSON {}", real.to_json());
    let output = format!("listing\n{}\nmore listing\n\n{record}\n", decoy_record());

    assert_eq!(perf_timings(&output), real);
    assert_eq!(perf_timings(&output.replace('\n', "\r\n")), real, "a pseudo-terminal's CRLF");
    assert_eq!(perf_timings(&format!("{record}\r\n\r\n  \r\n")), real, "trailing blank lines are skipped");
}

/// A missing or malformed final record is refused, even when an earlier
/// line holds a well-formed one.
#[test]
fn the_reader_refuses_a_missing_or_malformed_final_record() {
    let decoy = decoy_record();
    assert!(!refused(&decoy), "control: the decoy alone is a record");

    for (case, output) in [
        ("no output", String::new()),
        ("only blank lines", "\n\r\n \n".to_string()),
        ("a record, then listing text", format!("{decoy}\nlisting\n")),
        ("no prefix", format!("{}\n", decoy.trim_start_matches("WT_PERF_JSON "))),
        ("truncated JSON", format!("listing\n{}\n", &decoy[..decoy.len() - 1])),
        ("trailing content", format!("{decoy} x\n")),
        ("a lowercase prefix", format!("{}\n", decoy.replacen("WT_PERF_JSON", "wt_perf_json", 1))),
    ] {
        assert!(refused(&output), "{case}: {output:?}");
    }
}

/// A commit subject that resembles the record appears in the listing above
/// it and is never taken for it; without `--perf` the same listing has no
/// record at all.
#[test]
fn a_commit_message_resembling_the_record_is_never_taken_for_it() {
    let repo = temp_repo();
    let parent = tempfile::tempdir().expect("create temp dir");
    let feature = parent.path().join("feature");
    run_git(repo.path(), &["worktree", "add", "-b", "feature-a", feature.to_str().unwrap()]);
    run_git(&feature, &["commit", "--allow-empty", "-m", &decoy_record()]);
    let short = "WT_PERF_JSON {\"";

    let (timings, stderr) = json_report(&feature, &["-v"]);

    let (listing, _) = stderr.trim_end().rsplit_once('\n').expect("a listing, then the record");
    assert!(listing.contains(short), "control: the subject is in the listing: {listing}");
    assert!(timings.span(&[Stage::VerboseRender]).is_some(), "the real record: {timings:#?}");

    let output = assert_cmd::Command::cargo_bin("wt")
        .unwrap()
        .current_dir(&feature)
        .env_remove("TERM_PROGRAM")
        .env_remove("KITTY_WINDOW_ID")
        .args(["list", "-v"])
        .output()
        .expect("wt list -v runs");
    let plain = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success() && plain.contains(short), "{plain}");
    assert!(refused(&plain), "no --perf, no record: {plain}");
}

/// Captured through a pseudo-terminal, every line ends in CRLF; the record is
/// still the final line and decodes.
#[cfg(unix)]
#[test]
fn a_pseudo_terminal_capture_ends_in_the_record() {
    let repo = temp_repo();
    let wt = assert_cmd::cargo::cargo_bin("wt");
    let inner = format!("exec '{}' list --perf=json", wt.display().to_string().replace('\'', r"'\''"));
    let mut script = Command::new("script");
    if cfg!(target_os = "macos") {
        script.args(["-q", "/dev/null", "/bin/sh", "-c", &inner]);
    } else {
        script.args(["-qec", &format!("/bin/sh -c \"{inner}\""), "/dev/null"]);
    }
    let output = script
        .current_dir(repo.path())
        .env_remove("TERM_PROGRAM")
        .env_remove("KITTY_WINDOW_ID")
        .stdin(std::process::Stdio::null())
        .output()
        .expect("script runs");
    let captured = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{captured}");

    assert!(captured.trim_end().contains("\r\nWT_PERF_JSON {"), "a CRLF capture: {captured:?}");
    let timings = perf_timings(&captured);
    assert_eq!(timings.scope(), Scope::Command);
    assert!(timings.span(&[Stage::LocalReads, Stage::LocalGather]).is_some(), "{timings:#?}");
}

/// Two runs of one listing, one per renderer, show the same stages in the
/// same order: every span of the JSON record is a row of the human report.
#[test]
fn the_human_and_json_reports_of_one_listing_show_the_same_stages() {
    let repo = temp_repo();
    let (timings, _) = json_report(repo.path(), &[]);
    let human = assert_cmd::Command::cargo_bin("wt")
        .unwrap()
        .current_dir(repo.path())
        .env_remove("TERM_PROGRAM")
        .env_remove("KITTY_WINDOW_ID")
        .env("NO_COLOR", "1")
        .args(["list", "--perf"])
        .output()
        .expect("wt list --perf runs");
    let human = String::from_utf8_lossy(&human.stderr);
    let report = &human[human.find("Performance").expect("a human report")..];

    let mut rows = report.lines();
    for stage in all_stages(&timings) {
        assert!(rows.any(|row| row.contains(stage.label())), "`{stage}` missing or out of order:\n{report}");
    }
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

/// The worker's own report, end to end: `--perf=json` launches the real
/// `wt internal-refresh` with `--timings`, the wait reads its report from
/// the receipt, and the record carries it beside the foreground spans. The
/// provider fails at once, so the PR half's result reaches the wait only
/// through the receipt and the report is always read.
#[test]
#[serial]
fn a_perf_listing_carries_its_workers_own_report() {
    let fixture = MixedFixture::new().with_gitea_origin();
    let gitea = FakeGitea::new(GiteaReply::Status(500));
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    let output =
        fixture.wt_command_via_gitea(&gitea).args(["list", "--perf=json"]).output().expect("wt list --perf=json runs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    let timings = perf_timings(&stderr);

    assert_eq!(timings.worker_report_status(), Some(WorkerReportStatus::Complete), "{timings:#?}");
    let [report] = timings.worker_reports() else {
        panic!("one launch: {:#?}", timings.worker_reports());
    };
    assert_eq!(report.launch_index, 0);
    let worker = report.timings().expect("a usable report");
    let halves = worker.span(&[Stage::WorkerHalves]).expect("the halves group");
    assert_eq!(halves.children().iter().map(Span::stage).collect::<Vec<_>>(), [Stage::PrRefresh, Stage::HeadRefresh]);
    assert!(worker.span(&[Stage::WorkerHalves, Stage::PrRefresh, Stage::PrRequest]).is_some(), "the request was made");
    assert!(worker.span(&[Stage::WorkerHalves, Stage::HeadRefresh, Stage::HeadCheck]).is_some(), "the check ran");
    // Diagnostics only: no foreground stage is a worker stage.
    let foreground = timings.span(&[Stage::RemoteAndLocal, Stage::RefreshWorker]).expect("the wait");
    assert!(foreground.find(&[Stage::WorkerHalves]).is_none());
}
