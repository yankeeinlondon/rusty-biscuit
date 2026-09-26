use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use biscuit_terminal::components::git_graph::{GraphViewport, LaneEntry, NaturalSize};
use biscuit_terminal::components::terminal_image::ImageWidth;
use biscuit_terminal::discovery::fonts::CellSize;
use worktree::fork_origin::{ForkOrigin, ForkOriginStore};
use worktree::git::recorder;
use worktree::listing::RefTips;
use worktree::pull_requests::{OpenPullRequest, PrListing};

use super::*;

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {args:?} failed in {repo:?}");
}

fn git_output(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .expect("git should be installed");
    assert!(output.status.success(), "git {args:?} failed in {repo:?}");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn init_repo(path: &Path) {
    run_git(path, &["init", "-q", "-b", "main"]);
    for (key, value) in [
        ("user.email", "test@example.com"),
        ("user.name", "Test User"),
        ("commit.gpgsign", "false"),
        // Suppress optional Git workers so nextest does not report fixture leaks.
        ("gc.auto", "0"),
        ("core.fsmonitor", "false"),
        ("core.commitGraph", "false"),
    ] {
        run_git(path, &["config", key, value]);
    }
}

/// Commits `file` and returns the new full SHA.
fn commit(path: &Path, file: &str) -> String {
    fs::write(path.join(file), format!("{file}\n")).unwrap();
    run_git(path, &["add", "--", file]);
    run_git(path, &["commit", "-q", "-m", file]);
    git_output(path, &["rev-parse", "HEAD"])
}

struct DirGuard(PathBuf);

impl DirGuard {
    fn enter(dir: &Path) -> Self {
        let old = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir).expect("enter repo");
        Self(old)
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn commits(shas: &[&String]) -> Vec<LaneEntry> {
    shas.iter().map(|sha| LaneEntry::Commit((*sha).clone())).collect()
}

fn input(current: &str, branches: &[&str], forks: ForkOriginStore) -> GatherInput {
    GatherInput {
        default_branch: "main".to_string(),
        current_branch: Some(current.to_string()),
        branch_names: branches.iter().map(|b| b.to_string()).collect(),
        refs: RefTips::read().expect("for-each-ref"),
        forks,
    }
}

fn forked(store: &mut ForkOriginStore, branch: &str, parent: &str, created_at: u64) {
    store.insert(
        branch,
        ForkOrigin {
            base_branch: parent.to_string(),
            base_sha: "0".repeat(40),
            created_at,
        },
    );
}

fn count(calls: &[Vec<String>], command: &str) -> usize {
    recorder::count_matching(calls, |args| args.first().map(String::as_str) == Some(command))
}

/// `main: c1 - c2 - c3`, `feature-a: c2 - a1`, `feature-b: c3 - b1`.
struct Branches {
    _dir: tempfile::TempDir,
    path: PathBuf,
    c1: String,
    c2: String,
    c3: String,
    a1: String,
    b1: String,
}

fn branches() -> Branches {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let c1 = commit(&path, "c1");
    let c2 = commit(&path, "c2");
    run_git(&path, &["checkout", "-q", "-b", "feature-a"]);
    let a1 = commit(&path, "a1");
    run_git(&path, &["checkout", "-q", "main"]);
    let c3 = commit(&path, "c3");
    run_git(&path, &["checkout", "-q", "-b", "feature-b"]);
    let b1 = commit(&path, "b1");
    run_git(&path, &["checkout", "-q", "main"]);
    Branches { _dir: dir, path, c1, c2, c3, a1, b1 }
}

#[test]
#[serial_test::serial]
fn a_focused_view_hands_over_full_shas_for_each_line() {
    let repo = branches();
    let _guard = DirGuard::enter(&repo.path);

    let (graph, verbose) = gather(&input("feature-a", &["main", "feature-a", "feature-b"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("focused view");

    assert!(verbose.is_none());
    assert_eq!(graph.current_branch, "feature-a");
    // Two context commits ending at the fork point, then what main has since.
    assert_eq!(graph.default_entries, commits(&[&repo.c1, &repo.c2, &repo.c3]));
    assert_eq!(graph.lines.len(), 1, "an unrelated branch gets no lane: {:?}", graph.lines);
    let line = &graph.lines[0];
    assert_eq!(line.branch, "feature-a");
    assert_eq!(line.parent, None);
    assert_eq!(line.fork_sha.as_ref(), Some(&repo.c2));
    assert_eq!(line.entries, commits(&[&repo.a1]));
    assert_eq!(graph.refs, [("main".to_string(), repo.c3.clone())]);
}

#[test]
#[serial_test::serial]
fn a_fork_parent_gets_its_own_line_in_a_focused_view() {
    let repo = branches();
    let _guard = DirGuard::enter(&repo.path);
    run_git(&repo.path, &["checkout", "-q", "-b", "feat/theme", "main"]);
    let t1 = commit(&repo.path, "t1");
    run_git(&repo.path, &["checkout", "-q", "-b", "feat/dark"]);
    let d1 = commit(&repo.path, "d1");
    run_git(&repo.path, &["checkout", "-q", "feat/theme"]);
    let t2 = commit(&repo.path, "t2");
    run_git(&repo.path, &["checkout", "-q", "main"]);
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "feat/theme", "main", 10);
    forked(&mut forks, "feat/dark", "feat/theme", 20);

    let (graph, _) = gather(&input("feat/dark", &["main", "feat/theme", "feat/dark"], forks), true, false);
    let graph = graph.expect("focused view");

    let names: Vec<&str> = graph.lines.iter().map(|line| line.branch.as_str()).collect();
    assert_eq!(names, ["feat/theme", "feat/dark"]);
    let theme = &graph.lines[0];
    assert_eq!(theme.fork_sha.as_ref(), Some(&repo.c3));
    assert_eq!(theme.entries, commits(&[&t1, &t2]));
    assert_eq!(theme.created_at, Some(10));
    let dark = &graph.lines[1];
    assert_eq!(dark.parent.as_deref(), Some("feat/theme"));
    assert_eq!(dark.fork_sha.as_ref(), Some(&t1));
    assert_eq!(dark.entries, commits(&[&d1]));
    assert!(graph.refs.contains(&("feat/theme".to_string(), t2.clone())));
    // The default lane stops at the parent's fork point.
    assert_eq!(graph.default_entries, commits(&[&repo.c2, &repo.c3]));
}

#[test]
#[serial_test::serial]
fn the_base_view_gives_every_worktree_branch_a_line() {
    let repo = branches();
    let _guard = DirGuard::enter(&repo.path);
    run_git(&repo.path, &["checkout", "-q", "-b", "chore/merged", "main"]);
    let m1 = commit(&repo.path, "m1");
    run_git(&repo.path, &["checkout", "-q", "main"]);
    run_git(&repo.path, &["merge", "-q", "--ff-only", "chore/merged"]);
    run_git(&repo.path, &["checkout", "-q", "-b", "feature-c", "feature-b"]);
    let c_1 = commit(&repo.path, "cc1");
    run_git(&repo.path, &["checkout", "-q", "main"]);
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "feature-c", "feature-b", 30);

    let branch_names = ["main", "feature-a", "feature-b", "chore/merged", "feature-c"];
    recorder::start_recording();
    let (graph, verbose) = gather(&input("main", &branch_names, forks), true, true);
    let calls = recorder::finish_recording();
    let graph = graph.expect("base view");

    assert!(verbose.is_none(), "the base view has no verbose section");
    assert_eq!(graph.default_entries, commits(&[&repo.c1, &repo.c2, &repo.c3, &m1]));
    let line = |name: &str| graph.lines.iter().find(|line| line.branch == name).unwrap_or_else(|| panic!("{name}: {:?}", graph.lines));
    assert_eq!(line("feature-a").entries, commits(&[&repo.a1]));
    assert_eq!(line("feature-a").fork_sha.as_ref(), Some(&repo.c2));
    assert!(line("feature-a").last_active.is_some());
    assert_eq!(line("feature-b").entries, commits(&[&repo.b1]));
    // Already in main: no entries, forked at its own tip, so it becomes a tag.
    assert!(line("chore/merged").entries.is_empty());
    assert_eq!(line("chore/merged").fork_sha.as_ref(), Some(&m1));
    assert_eq!(line("chore/merged").last_active, None);
    // A recorded parent that is also drawn nests the line under it.
    assert_eq!(line("feature-c").parent.as_deref(), Some("feature-b"));
    assert_eq!(line("feature-c").fork_sha.as_ref(), Some(&repo.b1));
    assert_eq!(line("feature-c").entries, commits(&[&c_1]));
    assert_eq!(line("feature-c").created_at, Some(30));

    assert_eq!(count(&calls, "merge-base"), 4, "one per branch, got {calls:?}");
    assert_eq!(count(&calls, "log"), 5, "the default lane and one per branch, got {calls:?}");
    assert_eq!(count(&calls, "rev-list"), 0, "no line reached the window, got {calls:?}");
}

#[test]
#[serial_test::serial]
fn lines_past_the_window_start_with_one_elision() {
    let repo = branches();
    let _guard = DirGuard::enter(&repo.path);
    run_git(&repo.path, &["checkout", "-q", "feature-a"]);
    let newer: Vec<String> = (2..=7).map(|n| commit(&repo.path, &format!("a{n}"))).collect();
    run_git(&repo.path, &["checkout", "-q", "main"]);

    let (graph, _) = gather(&input("feature-a", &["main", "feature-a"], ForkOriginStore::default()), true, false);
    let entries = &graph.expect("focused view").lines[0].entries;

    let mut expected = vec![LaneEntry::Elided(2)];
    expected.extend(newer[1..].iter().map(|sha| LaneEntry::Commit(sha.clone())));
    assert_eq!(entries, &expected);
}

/// A bare origin beside the repository; returns its path.
fn add_origin(repo: &Path) -> PathBuf {
    let origin = repo.parent().unwrap().join(format!(
        "{}-origin.git",
        repo.file_name().unwrap().to_string_lossy()
    ));
    run_git(repo.parent().unwrap(), &["init", "-q", "--bare", "-b", "main", origin.to_str().unwrap()]);
    run_git(repo, &["remote", "add", "origin", origin.to_str().unwrap()]);
    run_git(repo, &["push", "-q", "origin", "main"]);
    run_git(repo, &["fetch", "-q", "origin"]);
    origin
}

/// Pushes one new commit to `origin/main` from a scratch clone and fetches it.
fn advance_origin(repo: &Path, origin: &Path, file: &str) -> String {
    let clone = origin.with_extension("clone");
    if !clone.exists() {
        run_git(origin.parent().unwrap(), &["clone", "-q", origin.to_str().unwrap(), clone.to_str().unwrap()]);
        init_repo_config(&clone);
    }
    run_git(&clone, &["pull", "-q", "origin", "main"]);
    let new = commit(&clone, file);
    run_git(&clone, &["push", "-q", "origin", "main"]);
    run_git(repo, &["fetch", "-q", "origin"]);
    new
}

fn init_repo_config(path: &Path) {
    for (key, value) in [
        ("user.email", "test@example.com"),
        ("user.name", "Test User"),
        ("commit.gpgsign", "false"),
        ("gc.auto", "0"),
    ] {
        run_git(path, &["config", key, value]);
    }
}

#[test]
#[serial_test::serial]
fn origin_ahead_extends_the_default_lane_and_diverged_origin_gets_a_line() {
    let repo = branches();
    let _guard = DirGuard::enter(&repo.path);
    let origin = add_origin(&repo.path);
    let u1 = advance_origin(&repo.path, &origin, "u1");

    // PR-driven: origin/main is only ahead, so its commit sits on the default
    // lane and both tips are tags.
    let (graph, _) = gather(&input("feature-a", &["main", "feature-a"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("focused view");
    assert_eq!(graph.default_entries.last(), Some(&LaneEntry::Commit(u1.clone())));
    assert_eq!(
        graph.refs,
        [("main".to_string(), repo.c3.clone()), ("origin/main".to_string(), u1.clone())]
    );
    assert!(graph.lines.iter().all(|line| line.branch != "origin/main"));

    // Diverged: the default lane is the local branch and origin/main gets a
    // line of its own from the fork point.
    let local = commit(&repo.path, "local");
    let (graph, _) = gather(&input("feature-a", &["main", "feature-a"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("focused view");
    assert_eq!(graph.default_entries.last(), Some(&LaneEntry::Commit(local)));
    let origin_line = graph.lines.iter().find(|line| line.branch == "origin/main").expect("origin line");
    assert_eq!(origin_line.fork_sha.as_ref(), Some(&repo.c3));
    assert_eq!(origin_line.entries, commits(&[&u1]));
}

/// ```text
/// root - main_base ---------- merge(terminal) = main
///    \          \
///     \          feature_root - merge(main_base) - work - merge(terminal) = feature
///      terminal_base
/// ```
///
/// `main` and `feature` have two incomparable best merge bases.
fn criss_cross() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let root = commit(&path, "root");
    let main_base = commit(&path, "main-base");
    run_git(&path, &["checkout", "-q", "-b", "terminal", &root]);
    commit(&path, "terminal-base");
    run_git(&path, &["checkout", "-q", "-b", "feature", &root]);
    commit(&path, "feature-root");
    run_git(&path, &["merge", "-q", "--no-ff", "--no-edit", &main_base]);
    commit(&path, "feature-work");
    run_git(&path, &["checkout", "-q", "main"]);
    run_git(&path, &["merge", "-q", "--no-ff", "--no-edit", "terminal"]);
    run_git(&path, &["checkout", "-q", "feature"]);
    run_git(&path, &["merge", "-q", "--no-ff", "--no-edit", "terminal"]);
    run_git(&path, &["checkout", "-q", "main"]);
    (dir, path)
}

#[test]
#[serial_test::serial]
fn criss_cross_lines_and_verbose_details_hold_tip_unique_commits() {
    let (_dir, path) = criss_cross();
    let _guard = DirGuard::enter(&path);
    let bases = git_output(&path, &["merge-base", "--all", "main", "feature"]);
    assert_eq!(bases.lines().count(), 2, "the fixture needs two best bases");

    let tip_unique: Vec<String> = git_output(&path, &["log", "--format=%H", "--reverse", "feature", "--not", "main", "--"])
        .lines()
        .map(str::to_string)
        .collect();
    let (graph, verbose) = gather(&input("feature", &["main", "feature"], ForkOriginStore::default()), true, true);

    let line = &graph.expect("focused view").lines[0];
    let shown: Vec<String> = line
        .entries
        .iter()
        .filter_map(|entry| match entry {
            LaneEntry::Commit(sha) => Some(sha.clone()),
            LaneEntry::Elided(_) => None,
        })
        .collect();
    assert_eq!(shown, tip_unique, "commits reachable from the tip but not from main");

    let verbose = verbose.expect("verbose details");
    let detailed: Vec<String> = verbose.branch_commits.iter().map(|c| c.short_sha.clone()).collect();
    let expected: Vec<String> = tip_unique
        .iter()
        .map(|full| git_output(&path, &["rev-parse", "--short", full]))
        .collect();
    assert_eq!(detailed, expected);
}

#[test]
#[serial_test::serial]
fn graph_and_verbose_share_one_merge_base() {
    let repo = branches();
    let _guard = DirGuard::enter(&repo.path);

    recorder::start_recording();
    let (graph, verbose) = gather(&input("feature-a", &["main", "feature-a"], ForkOriginStore::default()), true, true);
    let calls = recorder::finish_recording();

    assert!(graph.is_some() && verbose.is_some());
    assert_eq!(count(&calls, "merge-base"), 1, "got {calls:?}");
    assert_eq!(
        recorder::count_matching(&calls, |args| args.len() >= 2 && args[0] == "rev-parse" && args[1] == "--short"),
        0,
        "got {calls:?}"
    );
    let verbose = verbose.unwrap();
    assert_eq!(verbose.merge_base.map(|c| c.message), Some("c2".to_string()));
    assert_eq!(verbose.branch_commits.len(), 1);
}

#[test]
#[serial_test::serial]
fn nothing_is_gathered_when_detached_or_not_needed() {
    let repo = branches();
    let _guard = DirGuard::enter(&repo.path);
    let mut detached = input("feature-a", &["main"], ForkOriginStore::default());
    detached.current_branch = None;

    recorder::start_recording();
    let results = [
        gather(&detached, true, true),
        gather(&input("feature-a", &["main"], ForkOriginStore::default()), false, false),
        // Verbose alone on the default branch has nothing to show.
        gather(&input("main", &["main"], ForkOriginStore::default()), false, true),
    ];
    let calls = recorder::finish_recording();

    assert!(results.iter().all(|(graph, verbose)| graph.is_none() && verbose.is_none()));
    assert_eq!(count(&calls, "merge-base") + count(&calls, "log"), 0, "got {calls:?}");
}

fn pr(number: u64, repo: &str, branch: &str, target: &str) -> OpenPullRequest {
    OpenPullRequest {
        number,
        url: None,
        source_repo: Some(repo.to_string()),
        source_branch: branch.to_string(),
        target_branch: target.to_string(),
    }
}

#[test]
#[serial_test::serial]
fn the_git_graph_tags_own_prs_only_and_takes_the_width_override() {
    let repo = branches();
    let _guard = DirGuard::enter(&repo.path);
    let (graph, _) = gather(&input("feature-a", &["main", "feature-a"], ForkOriginStore::default()), true, false);
    let facts = graph.expect("focused view");
    let prs = PrListing {
        source_repo: Some("owner/repo".to_string()),
        pull_requests: vec![
            pr(5, "owner/repo", "feature-a", "main"),
            pr(6, "someone/fork", "feature-a", "main"),
            pr(7, "owner/repo", "feature-b", "main"),
        ],
        fetched_at: Some(0),
        stale: false,
    };

    let mermaid = facts.to_git_graph(&prs, None).mermaid().expect("mermaid");
    assert!(mermaid.contains("PR #5 → main"), "{mermaid}");
    assert!(!mermaid.contains("#6"), "a fork's PR must not tag the branch: {mermaid}");
    assert!(!mermaid.contains("#7"), "an undrawn branch's PR is left out: {mermaid}");
    assert!(mermaid.contains(&repo.a1[..7]), "{mermaid}");

    let viewport = GraphViewport {
        columns: 120,
        rows: 40,
        cell: CellSize::FALLBACK,
    };
    let measure = |_: &str| Some(NaturalSize { width: 2000.0, height: 200.0 });
    let fitted = facts.to_git_graph(&prs, None).plan_with(viewport, &measure).expect("plan");
    assert!(fitted.trimmed_commits > 0, "a scale-derived width trims to fit: {fitted:?}");
    let overridden = facts
        .to_git_graph(&prs, Some(ImageWidth::Characters(50)))
        .plan_with(viewport, &measure)
        .expect("plan");
    assert_eq!(overridden.columns, 50);
    assert_eq!(overridden.trimmed_commits, 0, "an explicit width is never trimmed to");
}
