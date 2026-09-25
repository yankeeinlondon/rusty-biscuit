
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use worktree::git::recorder;
use worktree::pull_requests::OpenPrSource;
use worktree::worktree::{list_worktrees, parse_worktree_state};

use crate::commands::git_graph;

/// No origin: the PR stage returns at once without a request.
fn no_prs() -> Option<Box<dyn OpenPrSource>> {
    None
}

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {:?} failed in {:?}", args, repo);
}

struct DirGuard {
    old: PathBuf,
}

impl DirGuard {
    fn enter(dir: &Path) -> Self {
        let old = std::env::current_dir().expect("get cwd");
        std::env::set_current_dir(dir).expect("set cwd");
        DirGuard { old }
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.old);
    }
}

fn temp_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path();

    run_git(path, &["init", "-b", "main"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    run_git(path, &["config", "commit.gpgsign", "false"]);
    // Suppress background/detached git work so nextest leak detection
    // sees no lingering child processes after the test returns.
    run_git(path, &["config", "gc.auto", "0"]);
    run_git(path, &["config", "core.fsmonitor", "false"]);
    run_git(path, &["config", "core.commitGraph", "false"]);

    fs::write(path.join("file.txt"), "1\n").unwrap();
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "commit 1"]);

    dir
}

/// Create a temp repo with `main` (2 commits) and `feature-a` (1 commit
/// since divergence), checked out on `main`. Used by `gather_data` tests
/// that need real branch data without the overhead of linked worktrees.
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

fn temp_repo_named_with_linked_feature() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let main = dir.path().join("main");
    let feature = dir.path().join("feature-a");
    fs::create_dir(&main).expect("create main repo dir");

    run_git(&main, &["init", "-b", "main"]);
    run_git(&main, &["config", "user.email", "test@example.com"]);
    run_git(&main, &["config", "user.name", "Test User"]);
    run_git(&main, &["config", "commit.gpgsign", "false"]);
    run_git(&main, &["config", "gc.auto", "0"]);
    run_git(&main, &["config", "core.fsmonitor", "false"]);
    run_git(&main, &["config", "core.commitGraph", "false"]);

    fs::write(main.join("file.txt"), "1\n").unwrap();
    run_git(&main, &["add", "."]);
    run_git(&main, &["commit", "-m", "commit 1"]);

    fs::write(main.join("file.txt"), "2\n").unwrap();
    run_git(&main, &["add", "."]);
    run_git(&main, &["commit", "-m", "commit 2"]);

    run_git(&main, &["checkout", "-b", "feature-a"]);
    fs::write(main.join("a.txt"), "a\n").unwrap();
    run_git(&main, &["add", "."]);
    run_git(&main, &["commit", "-m", "feature a"]);
    run_git(&main, &["checkout", "main"]);
    run_git(
        &main,
        &["worktree", "add", feature.to_str().unwrap(), "feature-a"],
    );

    (dir, main)
}

#[test]
#[serial_test::serial]
fn list_worktrees_resolves_default_branch_once() {
    let repo = temp_repo();
    let _guard = DirGuard::enter(repo.path());

    recorder::start_recording();
    let list = list_worktrees().expect("list_worktrees should succeed in temp repo");
    let calls = recorder::finish_recording();

    let symbolic_ref_count = recorder::count_matching(&calls, |args| {
        args.first().map(String::as_str) == Some("symbolic-ref")
    });
    assert_eq!(
        symbolic_ref_count, 1,
        "expected exactly one symbolic-ref call, got {calls:?}"
    );
    assert!(!list.default_branch.is_empty());
}

#[test]
#[serial_test::serial]
fn run_skips_graph_git_calls_when_image_unavailable() {
    let repo = temp_repo();
    let _dir = DirGuard::enter(repo.path());

    let old_term = std::env::var("TERM_PROGRAM").ok();
    let old_kitty = std::env::var("KITTY_WINDOW_ID").ok();
    unsafe {
        std::env::remove_var("TERM_PROGRAM");
        std::env::remove_var("KITTY_WINDOW_ID");
    }

    recorder::start_recording();
    let result = super::run(None, false, false, std::time::Instant::now());
    let calls = recorder::finish_recording();

    unsafe {
        match old_term {
            Some(v) => std::env::set_var("TERM_PROGRAM", v),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
        match old_kitty {
            Some(v) => std::env::set_var("KITTY_WINDOW_ID", v),
            None => std::env::remove_var("KITTY_WINDOW_ID"),
        }
    }

    assert!(result.is_ok(), "run should succeed: {result:?}");
    let graph_calls = recorder::count_matching(&calls, |args| {
        matches!(
            args.first().map(String::as_str),
            Some("merge-base") | Some("log")
        )
    });
    assert_eq!(
        graph_calls, 0,
        "expected zero graph-path git calls when image support is unavailable, got {calls:?}"
    );
}

#[test]
#[serial_test::serial]
fn run_pipeline_without_perf_produces_no_collector() {
    let repo = temp_repo();
    let _dir = DirGuard::enter(repo.path());
    let terminal = Terminal::default();

    let collector = super::run_pipeline(
        None,
        false,
        false,
        std::time::Instant::now(),
        ImageSupport::None,
        &terminal,
        no_prs,
    )
    .expect("run_pipeline should succeed");

    assert!(
        collector.is_none(),
        "no collector should be produced when perf is false"
    );
}

#[test]
#[serial_test::serial]
fn run_pipeline_non_image_verbose_includes_verbose_gather_stage() {
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
    let _guard = DirGuard::enter(&feature_path);
    let terminal = Terminal::default();

    let collector = super::run_pipeline(
        None,
        true,
        true,
        std::time::Instant::now(),
        ImageSupport::None,
        &terminal,
        no_prs,
    )
    .expect("run_pipeline should succeed");

    let stages = collector
        .as_ref()
        .expect("collector should be present when perf is true")
        .recorded_stages();
    let names: Vec<_> = stages.iter().map(|(name, _)| *name).collect();
    assert!(
        names.contains(&"verbose gather"),
        "verbose gather should be recorded on non-image verbose path, got: {names:?}"
    );
    assert!(
        names.contains(&"verbose render"),
        "verbose render should be recorded, got: {names:?}"
    );
    assert!(
        !names.contains(&"graph gather"),
        "graph gather should not be recorded on non-image path, got: {names:?}"
    );
    assert!(
        !names.contains(&"graph image render (biscuit-terminal)"),
        "graph image render should not be recorded, got: {names:?}"
    );
}

#[test]
#[serial_test::serial]
fn run_pipeline_graph_git_calls_begin_before_list_gather_completes() {
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
    let _guard = DirGuard::enter(repo_path);
    let terminal = Terminal::builder().width(120).build();

    recorder::start_recording();
    let result = super::run_pipeline(
        None,
        false,
        false,
        std::time::Instant::now(),
        ImageSupport::Kitty,
        &terminal,
        no_prs,
    );
    let calls = recorder::finish_recording();

    assert!(result.is_ok(), "run_pipeline should succeed");

    let first_graph = calls.iter().position(|args| {
        matches!(
            args.first().map(String::as_str),
            Some("merge-base") | Some("log")
        )
    });
    let last_list = calls.iter().rposition(|args| {
        matches!(
            args.first().map(String::as_str),
            Some("rev-list") | Some("merge-tree")
        )
    });

    let first_graph = first_graph.expect("graph gather should issue git calls");
    let last_list = last_list.expect("list gather should issue branch-comparison git calls");
    assert!(
        first_graph < last_list,
        "expected graph git calls to begin before list gather completed, got {calls:?}"
    );
}

#[test]
#[serial_test::serial]
fn run_pipeline_gathers_the_graph_on_a_narrow_image_terminal() {
    // The 80-column gate is gone: `GitGraph` trims, then clamps, so a narrow
    // terminal shows a smaller graph rather than none.
    let repo = temp_repo_with_feature_branch();
    let _dir = DirGuard::enter(repo.path());
    let narrow = Terminal::builder().width(40).build();

    let collector = super::run_pipeline(
        None,
        false,
        true,
        std::time::Instant::now(),
        ImageSupport::Kitty,
        &narrow,
        no_prs,
    )
    .expect("run_pipeline should succeed");

    let names: Vec<_> = collector
        .as_ref()
        .expect("collector")
        .recorded_stages()
        .iter()
        .map(|(name, _)| *name)
        .collect();
    for stage in ["pre-dispatch", "list gather", "pr gather", "graph gather", "table render"] {
        assert!(names.contains(&stage), "{stage} should be recorded, got {names:?}");
    }
}

#[test]
#[serial_test::serial]
fn run_pipeline_without_image_support_or_verbose_gathers_no_graph() {
    let repo = temp_repo_with_feature_branch();
    let _dir = DirGuard::enter(repo.path());
    let terminal = Terminal::builder().width(120).build();

    recorder::start_recording();
    let collector = super::run_pipeline(
        None,
        false,
        true,
        std::time::Instant::now(),
        ImageSupport::None,
        &terminal,
        no_prs,
    )
    .expect("run_pipeline should succeed");
    let calls = recorder::finish_recording();

    let names: Vec<_> = collector
        .as_ref()
        .expect("collector")
        .recorded_stages()
        .iter()
        .map(|(name, _)| *name)
        .collect();
    assert!(!names.contains(&"graph gather"), "got {names:?}");
    assert!(!names.contains(&"verbose gather"), "got {names:?}");
    let graph_calls = recorder::count_matching(&calls, |args| {
        matches!(args.first().map(String::as_str), Some("merge-base") | Some("log"))
    });
    assert_eq!(graph_calls, 0, "got {calls:?}");
}

/// Subprocess counts for the `wt list` gather pieces on a linked-worktree
/// fixture, with timings printed for observability (`--nocapture`).
///
/// The binding wall-clock SLA is `perf_full_command_non_image_meets_sla` in
/// the integration tests. Rasterization is excluded: nothing here renders.
#[test]
#[serial_test::serial]
fn perf_subprocess_counts_meet_sla() {
    use std::time::Instant;

    let (_repo, main) = temp_repo_named_with_linked_feature();
    let _guard = DirGuard::enter(&main);
    let _ = list_worktrees().expect("list_worktrees should succeed");

    recorder::start_recording();
    let t0 = Instant::now();
    let _ = list_worktrees();
    let list_elapsed = t0.elapsed();
    let list_calls = recorder::finish_recording();

    let count = |calls: &[Vec<String>], command: &str| {
        recorder::count_matching(calls, |args| args.first().map(String::as_str) == Some(command))
    };
    assert_eq!(count(&list_calls, "symbolic-ref"), 1, "got {list_calls:?}");
    assert_eq!(count(&list_calls, "for-each-ref"), 1, "got {list_calls:?}");
    eprintln!("list_worktrees: {list_elapsed:.2?}, {} git calls", list_calls.len());

    // Base view from the main checkout: one merge-base per branch, one log
    // for the default lane plus one per branch.
    let input = git_graph::GatherInput::from_list(&parse_worktree_state().expect("parse"));
    recorder::start_recording();
    let t0 = Instant::now();
    let (graph, verbose) = git_graph::gather(&input, true, false);
    let base_elapsed = t0.elapsed();
    let base_calls = recorder::finish_recording();

    let graph = graph.expect("base view");
    assert!(verbose.is_none());
    assert_eq!(count(&base_calls, "merge-base"), graph.lines.len(), "got {base_calls:?}");
    assert_eq!(count(&base_calls, "log"), 1 + graph.lines.len(), "got {base_calls:?}");
    eprintln!("base view gather: {base_elapsed:.2?}, {} git calls", base_calls.len());
}
