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

/// `merge-base` calls that compute a merge base, not `--is-ancestor` checks.
fn count_merge_bases(calls: &[Vec<String>]) -> usize {
    recorder::count_matching(calls, |args| args.first().map(String::as_str) == Some("merge-base") && args.get(1).map(String::as_str) != Some("--is-ancestor"))
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
    // Fast-forwarded into main: no history of its own, so no entries and no
    // fork, and its own tip is where `GitGraph` tags it.
    assert!(line("chore/merged").entries.is_empty());
    assert_eq!(line("chore/merged").fork_sha, None);
    assert_eq!(line("chore/merged").merged_into, None, "a fast-forward has no merge commit");
    assert_eq!(line("chore/merged").tip_sha.as_ref(), Some(&m1));
    assert_eq!(line("chore/merged").last_active, None);
    assert_eq!(line("feature-a").tip_sha.as_ref(), Some(&repo.a1));
    assert_eq!(line("feature-c").tip_sha.as_ref(), Some(&c_1));
    assert!(
        graph
            .to_git_graph(&PrListing::default(), None)
            .mermaid()
            .expect("a graph")
            .contains("tag: \"chore/merged\""),
        "the merged branch is labeled at its tip"
    );
    // A recorded parent that is also drawn nests the line under it.
    assert_eq!(line("feature-c").parent.as_deref(), Some("feature-b"));
    assert_eq!(line("feature-c").fork_sha.as_ref(), Some(&repo.b1));
    assert_eq!(line("feature-c").entries, commits(&[&c_1]));
    assert_eq!(line("feature-c").created_at, Some(30));

    // Classification asks one `--is-ancestor` per candidate lane (feature-c
    // has two: its parent's and the default lane), then the fork of each
    // branch with history of its own is one merge base.
    assert_eq!(count(&calls, "merge-base") - count_merge_bases(&calls), 5, "got {calls:?}");
    assert_eq!(count_merge_bases(&calls), 3, "one per branch with a lane, got {calls:?}");
    assert_eq!(count(&calls, "log"), 4, "the default lane and one per branch lane, got {calls:?}");
    // chore/merged's contained tip needs its first-parent chain, which is
    // empty for a fast-forward; no line reached its window.
    assert_eq!(count(&calls, "rev-list"), 1, "got {calls:?}");
    assert!(!graph.incomplete);
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
    // Classification adds one `merge-base --is-ancestor` (a yes/no question,
    // not a merge base); the unmerged branch's fork reuses verbose's answer.
    assert_eq!(count_merge_bases(&calls), 1, "got {calls:?}");
    assert_eq!(count(&calls, "merge-base"), 2, "got {calls:?}");
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

// ---------------------------------------------------------------------------
// Merged branches, first-parent lanes, anchors, and gaps.
// ---------------------------------------------------------------------------

/// An empty commit, for long chains; returns its full SHA.
fn empty_commit(path: &Path, message: &str) -> String {
    run_git(path, &["commit", "-q", "--allow-empty", "-m", message]);
    git_output(path, &["rev-parse", "HEAD"])
}

/// Merges `branch` into the checked-out branch with a merge commit; returns it.
fn merge_no_ff(path: &Path, branch: &str) -> String {
    run_git(path, &["merge", "-q", "--no-ff", "--no-edit", branch]);
    git_output(path, &["rev-parse", "HEAD"])
}

fn set_origin_main(path: &Path, sha: &str) {
    run_git(path, &["update-ref", "refs/remotes/origin/main", sha]);
}

fn line<'a>(graph: &'a GraphFacts, branch: &str) -> &'a GraphLine {
    graph
        .lines
        .iter()
        .find(|line| line.branch == branch)
        .unwrap_or_else(|| panic!("no line {branch}: {:?}", graph.lines))
}

fn mermaid(graph: &GraphFacts) -> String {
    graph.to_git_graph(&PrListing::default(), None).mermaid().expect("mermaid")
}

fn has_merge(mermaid: &str, lane: &str, sha: &str) -> bool {
    mermaid.contains(&format!("merge {lane} id: \"{}\"", &sha[..7]))
}

/// Observation 1 (PR #103): `fix/wt-ux` forks at `d1` and `origin/main` is
/// its merge; the local `main` stays one commit behind at `d2`.
///
/// ```text
/// r - d1 - d2 (main) - M (origin/main)
///       \             /
///        w1 - w2 ----   (fix/wt-ux)
/// ```
struct MergedViaMergeCommit {
    _dir: tempfile::TempDir,
    path: PathBuf,
    r: String,
    d1: String,
    d2: String,
    w1: String,
    w2: String,
    merge: String,
}

fn merged_via_merge_commit() -> MergedViaMergeCommit {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let r = commit(&path, "r");
    let d1 = commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "fix/wt-ux"]);
    let w1 = commit(&path, "w1");
    let w2 = commit(&path, "w2");
    run_git(&path, &["checkout", "-q", "main"]);
    let d2 = commit(&path, "d2");
    run_git(&path, &["checkout", "-q", "--detach"]);
    let merge = merge_no_ff(&path, "fix/wt-ux");
    set_origin_main(&path, &merge);
    run_git(&path, &["checkout", "-q", "main"]);
    MergedViaMergeCommit { _dir: dir, path, r, d1, d2, w1, w2, merge }
}

#[test]
#[serial_test::serial]
fn a_merged_current_branch_keeps_its_lane_and_merges_at_its_merge_commit() {
    let repo = merged_via_merge_commit();
    let _guard = DirGuard::enter(&repo.path);

    let (graph, _) = gather(&input("fix/wt-ux", &["main", "fix/wt-ux"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("focused view");

    let wt_ux = line(&graph, "fix/wt-ux");
    assert_eq!(wt_ux.entries, commits(&[&repo.w1, &repo.w2]), "the branch's own commits are its lane");
    assert_eq!(wt_ux.fork_sha.as_ref(), Some(&repo.d1));
    assert_eq!(wt_ux.merged_into.as_ref(), Some(&repo.merge));
    assert_eq!(wt_ux.tip_sha.as_ref(), Some(&repo.w2));
    // First-parent history only: no branch commit is presented as main's.
    assert_eq!(graph.default_entries, commits(&[&repo.r, &repo.d1, &repo.d2, &repo.merge]));
    assert_eq!(
        graph.refs,
        [("main".to_string(), repo.d2.clone()), ("origin/main".to_string(), repo.merge.clone())]
    );
    assert!(!graph.incomplete);
    let text = mermaid(&graph);
    assert!(has_merge(&text, "fix/wt-ux", &repo.merge), "{text}");
    assert!(text.contains("tag: \"main\""), "the local main is labeled: {text}");
}

#[test]
#[serial_test::serial]
fn a_merged_branch_keeps_its_lane_in_the_base_view() {
    let repo = merged_via_merge_commit();
    let _guard = DirGuard::enter(&repo.path);

    let (graph, _) = gather(&input("main", &["main", "fix/wt-ux"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");

    let wt_ux = line(&graph, "fix/wt-ux");
    assert_eq!(wt_ux.entries, commits(&[&repo.w1, &repo.w2]));
    assert_eq!(wt_ux.fork_sha.as_ref(), Some(&repo.d1));
    assert_eq!(wt_ux.merged_into.as_ref(), Some(&repo.merge));
    assert_eq!(graph.default_entries, commits(&[&repo.r, &repo.d1, &repo.d2, &repo.merge]));
    assert!(!graph.incomplete);

    // A viewport tall enough for both lanes keeps the merged one.
    let viewport = GraphViewport {
        columns: 120,
        rows: 40,
        cell: CellSize::FALLBACK,
    };
    let measure = |_: &str| Some(NaturalSize { width: 400.0, height: 120.0 });
    let plan = graph.to_git_graph(&PrListing::default(), None).plan_with(viewport, &measure).expect("plan");
    assert_eq!(plan.hidden_lanes, 0);
    assert!(has_merge(&plan.mermaid, "fix/wt-ux", &repo.merge), "{}", plan.mermaid);
    assert!(!plan.incomplete);
}

/// Observation 2 (PR #104): `fix/wt-ux` was merged at `m103`; `fix/sniff`
/// forked at `fix/wt-ux`'s tip (its recorded parent), ran longer than the
/// base window, and was merged at `m104` (`origin/main`). The local `main` is
/// one merge behind.
struct NestedParentMerged {
    _dir: tempfile::TempDir,
    path: PathBuf,
    r: String,
    d1: String,
    w2: String,
    w1: String,
    sniff: Vec<String>,
    m103: String,
    m104: String,
    forks: ForkOriginStore,
}

fn nested_parent_merged_into_default() -> NestedParentMerged {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let r = commit(&path, "r");
    let d1 = commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "fix/wt-ux"]);
    let w1 = commit(&path, "w1");
    let w2 = commit(&path, "w2");
    run_git(&path, &["checkout", "-q", "-b", "fix/sniff"]);
    let sniff: Vec<String> = (1..=BASE_DEFAULT_WINDOW + 2).map(|n| empty_commit(&path, &format!("s{n}"))).collect();
    run_git(&path, &["checkout", "-q", "main"]);
    let m103 = merge_no_ff(&path, "fix/wt-ux");
    run_git(&path, &["checkout", "-q", "--detach"]);
    let m104 = merge_no_ff(&path, "fix/sniff");
    set_origin_main(&path, &m104);
    run_git(&path, &["checkout", "-q", "main"]);
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "fix/wt-ux", "main", 10);
    forked(&mut forks, "fix/sniff", "fix/wt-ux", 20);
    NestedParentMerged { _dir: dir, path, r, d1, w1, w2, sniff, m103, m104, forks }
}

#[test]
#[serial_test::serial]
fn a_child_merged_into_the_default_branch_forks_from_its_parent_in_the_base_view() {
    let repo = nested_parent_merged_into_default();
    let _guard = DirGuard::enter(&repo.path);

    let (graph, _) = gather(&input("main", &["main", "fix/wt-ux", "fix/sniff"], repo.forks.clone()), true, false);
    let graph = graph.expect("base view");

    // The default lane is main's own history: no side commit fills its window.
    assert_eq!(graph.default_entries, commits(&[&repo.r, &repo.d1, &repo.m103, &repo.m104]));
    let wt_ux = line(&graph, "fix/wt-ux");
    assert_eq!(wt_ux.entries, commits(&[&repo.w1, &repo.w2]));
    assert_eq!(wt_ux.fork_sha.as_ref(), Some(&repo.d1));
    assert_eq!(wt_ux.merged_into.as_ref(), Some(&repo.m103));

    let sniff = line(&graph, "fix/sniff");
    assert_eq!(sniff.parent.as_deref(), Some("fix/wt-ux"));
    assert_eq!(sniff.fork_sha.as_ref(), Some(&repo.w2), "forks at its parent's tip");
    assert_eq!(sniff.merged_into.as_ref(), Some(&repo.m104));
    assert_eq!(sniff.tip_sha.as_ref(), repo.sniff.last());
    let mut expected = vec![LaneEntry::Elided(repo.sniff.len() - LINE_WINDOW)];
    expected.extend(commits(&repo.sniff[repo.sniff.len() - LINE_WINDOW..].iter().collect::<Vec<_>>()));
    assert_eq!(sniff.entries, expected, "the long side is a `+N` square and its newest commits");

    assert_eq!(
        graph.refs,
        [("main".to_string(), repo.m103.clone()), ("origin/main".to_string(), repo.m104.clone())]
    );
    assert!(!graph.incomplete);
    let text = mermaid(&graph);
    assert!(has_merge(&text, "fix/wt-ux", &repo.m103), "{text}");
    assert!(has_merge(&text, "fix/sniff", &repo.m104), "{text}");
    assert!(text.contains("tag: \"main\""), "the local main keeps its label: {text}");
}

/// ```text
/// r - d1 (main)
///       \
///        p1 - p2 - PM - p3   (parent)
///          \      /
///           c1 - c2          (child, merged into its parent)
/// ```
#[test]
#[serial_test::serial]
fn a_child_merged_into_its_parent_merges_on_the_parent_lane() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let r = commit(&path, "r");
    let d1 = commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "parent"]);
    let p1 = commit(&path, "p1");
    run_git(&path, &["checkout", "-q", "-b", "child"]);
    let c1 = commit(&path, "c1");
    let c2 = commit(&path, "c2");
    run_git(&path, &["checkout", "-q", "parent"]);
    let p2 = commit(&path, "p2");
    let merge = merge_no_ff(&path, "child");
    let p3 = commit(&path, "p3");
    run_git(&path, &["checkout", "-q", "main"]);
    let _guard = DirGuard::enter(&path);
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "parent", "main", 10);
    forked(&mut forks, "child", "parent", 20);

    let (graph, _) = gather(&input("child", &["main", "child"], forks), true, false);
    let graph = graph.expect("focused view");

    let child = line(&graph, "child");
    assert_eq!(child.parent.as_deref(), Some("parent"));
    assert_eq!(child.entries, commits(&[&c1, &c2]));
    assert_eq!(child.fork_sha.as_ref(), Some(&p1), "measured against the commit before the merge, not the parent's tip");
    assert_eq!(child.merged_into.as_ref(), Some(&merge));
    let parent = line(&graph, "parent");
    assert_eq!(parent.entries, commits(&[&p1, &p2, &merge, &p3]), "the parent lane holds the merge");
    assert_eq!(parent.fork_sha.as_ref(), Some(&d1));
    assert_eq!(graph.default_entries, commits(&[&r, &d1]));
    assert!(!graph.incomplete);
    assert!(has_merge(&mermaid(&graph), "child", &merge), "{}", mermaid(&graph));
}

#[test]
#[serial_test::serial]
fn a_recorded_parent_is_selected_with_or_without_its_own_worktree() {
    let repo = branches();
    let _guard = DirGuard::enter(&repo.path);
    run_git(&repo.path, &["checkout", "-q", "-b", "feat/child", "feature-a"]);
    let child = commit(&repo.path, "child");
    run_git(&repo.path, &["checkout", "-q", "main"]);
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "feat/child", "feature-a", 20);

    for worktrees in [&["main", "feat/child"][..], &["main", "feature-a", "feat/child"][..]] {
        let (graph, _) = gather(&input("feat/child", worktrees, forks.clone()), true, false);
        let graph = graph.expect("focused view");
        let names: Vec<&str> = graph.lines.iter().map(|line| line.branch.as_str()).collect();
        assert_eq!(names, ["feature-a", "feat/child"], "worktrees {worktrees:?}");
        let child_line = line(&graph, "feat/child");
        assert_eq!(child_line.parent.as_deref(), Some("feature-a"));
        assert_eq!(child_line.fork_sha.as_ref(), Some(&repo.a1));
        assert_eq!(child_line.entries, commits(&[&child]));
        assert_eq!(line(&graph, "feature-a").fork_sha.as_ref(), Some(&repo.c2));
    }

    // The base view nests the child only under a drawn parent.
    let (graph, _) = gather(&input("main", &["main", "feature-a", "feat/child"], forks.clone()), true, false);
    assert_eq!(line(graph.as_ref().unwrap(), "feat/child").parent.as_deref(), Some("feature-a"));
    let (graph, _) = gather(&input("main", &["main", "feat/child"], forks), true, false);
    let graph = graph.unwrap();
    let child_line = line(&graph, "feat/child");
    assert_eq!(child_line.parent, None);
    assert_eq!(child_line.fork_sha.as_ref(), Some(&repo.c2));
    assert_eq!(child_line.entries, commits(&[&repo.a1, &child]));
}

#[test]
#[serial_test::serial]
fn a_fast_forwarded_branch_is_a_label_at_its_commit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let _r = commit(&path, "r");
    let d1 = commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "ff"]);
    let f1 = commit(&path, "f1");
    run_git(&path, &["checkout", "-q", "main"]);
    run_git(&path, &["merge", "-q", "--ff-only", "ff"]);
    let d2 = commit(&path, "d2");
    let _guard = DirGuard::enter(&path);

    let (graph, _) = gather(&input("ff", &["main", "ff"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("focused view");

    let ff = line(&graph, "ff");
    assert!(ff.entries.is_empty(), "no empty lane: {ff:?}");
    assert_eq!(ff.merged_into, None, "no fabricated merge");
    assert_eq!(ff.fork_sha, None);
    assert_eq!(ff.tip_sha.as_ref(), Some(&f1));
    assert_eq!(graph.default_entries, commits(&[&d1, &f1, &d2]));
    assert!(!graph.incomplete);
    let text = mermaid(&graph);
    assert!(text.contains(&format!("commit id: \"{}\" tag: \"ff\"", &f1[..7])), "{text}");
    assert!(!text.contains("merge "), "{text}");
}

/// `c` was merged at `merge`; `d` was then created at `c`'s tip with `c` as
/// its recorded parent. Equal tips do not make `c` label-only.
#[test]
#[serial_test::serial]
fn equal_tips_keep_the_merged_lane_and_label_the_new_branch() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let _r = commit(&path, "r");
    let d1 = commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "c"]);
    let c1 = commit(&path, "c1");
    let c2 = commit(&path, "c2");
    run_git(&path, &["checkout", "-q", "main"]);
    commit(&path, "d2");
    let merge = merge_no_ff(&path, "c");
    run_git(&path, &["branch", "d", "c"]);
    let _guard = DirGuard::enter(&path);
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "d", "c", 20);

    let (graph, _) = gather(&input("main", &["main", "c", "d"], forks), true, false);
    let graph = graph.expect("base view");

    let c = line(&graph, "c");
    assert_eq!(c.entries, commits(&[&c1, &c2]));
    assert_eq!(c.fork_sha.as_ref(), Some(&d1));
    assert_eq!(c.merged_into.as_ref(), Some(&merge));
    let d = line(&graph, "d");
    assert!(d.entries.is_empty());
    assert_eq!(d.merged_into, None);
    assert_eq!(d.tip_sha.as_ref(), Some(&c2));
    assert_eq!(d.parent.as_deref(), Some("c"));
    assert!(!graph.incomplete);
    let text = mermaid(&graph);
    assert!(has_merge(&text, "c", &merge), "{text}");
    assert!(text.contains(&format!("commit id: \"{}\" tag: \"d\"", &c2[..7])), "{text}");
}

/// `b` was merged at `merge` and then got another commit. Its current
/// relationship is an unmerged lane forked at the old merged tip, which is
/// not on any drawn lane (it is `merge`'s second parent), so the lane is
/// unconnected and the graph says history is missing.
#[test]
#[serial_test::serial]
fn a_branch_continued_after_its_merge_is_an_unmerged_lane() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let _r = commit(&path, "r");
    let d1 = commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "b"]);
    let b1 = commit(&path, "b1");
    run_git(&path, &["checkout", "-q", "main"]);
    let merge = merge_no_ff(&path, "b");
    run_git(&path, &["checkout", "-q", "b"]);
    let b2 = commit(&path, "b2");
    run_git(&path, &["checkout", "-q", "main"]);
    let _guard = DirGuard::enter(&path);

    let (graph, _) = gather(&input("b", &["main", "b"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("focused view");

    let b = line(&graph, "b");
    assert_eq!(b.entries, commits(&[&b2]));
    assert_eq!(b.fork_sha.as_ref(), Some(&b1));
    assert_eq!(b.merged_into, None, "no earlier merge is reconstructed");
    assert!(graph.default_entries.iter().all(|entry| *entry != LaneEntry::Commit(b1.clone())));
    assert!(graph.default_entries.contains(&LaneEntry::Commit(d1.clone())));
    assert!(graph.default_entries.contains(&LaneEntry::Commit(merge.clone())));
    let plan = graph
        .to_git_graph(&PrListing::default(), None)
        .plan_with(
            GraphViewport {
                columns: 120,
                rows: 40,
                cell: CellSize::FALLBACK,
            },
            &|_: &str| Some(NaturalSize { width: 400.0, height: 120.0 }),
        )
        .expect("plan");
    assert!(plan.incomplete, "the fork is not drawn, so the notice accounts for it");
    assert!(!plan.mermaid.contains("merge "), "{}", plan.mermaid);
}

/// `t` reached main only through `other`'s merge.
#[test]
#[serial_test::serial]
fn an_indirectly_integrated_branch_gets_a_lane_without_a_merge_and_the_notice() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let _r = commit(&path, "r");
    let d1 = commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "t"]);
    let t1 = commit(&path, "t1");
    run_git(&path, &["checkout", "-q", "-b", "other", &d1]);
    commit(&path, "o1");
    merge_no_ff(&path, "t");
    run_git(&path, &["checkout", "-q", "main"]);
    let merge = merge_no_ff(&path, "other");
    let _guard = DirGuard::enter(&path);

    let (graph, _) = gather(&input("t", &["main", "t"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("focused view");

    let t = line(&graph, "t");
    assert_eq!(t.entries, commits(&[&t1]));
    assert_eq!(t.fork_sha.as_ref(), Some(&d1));
    assert_eq!(t.merged_into, None);
    assert!(graph.incomplete);
    assert_eq!(graph.default_entries.last(), Some(&LaneEntry::Commit(merge)));
    assert!(!mermaid(&graph).contains("merge "));
}

/// Main: `r`, `a1..a10`, then `side` forks, main gets `b1..b3`, merges
/// `side` (8 commits), and moves on 20 commits. Fork and merge are far
/// outside every window.
struct OldConnections {
    _dir: tempfile::TempDir,
    path: PathBuf,
    a: Vec<String>,
    side: Vec<String>,
    merge: String,
    after: Vec<String>,
}

fn old_connections() -> OldConnections {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let mut a = vec![empty_commit(&path, "r")];
    a.extend((1..=10).map(|n| empty_commit(&path, &format!("a{n}"))));
    run_git(&path, &["checkout", "-q", "-b", "side"]);
    let side: Vec<String> = (1..=8).map(|n| empty_commit(&path, &format!("s{n}"))).collect();
    run_git(&path, &["checkout", "-q", "main"]);
    for n in 1..=3 {
        empty_commit(&path, &format!("b{n}"));
    }
    let merge = merge_no_ff(&path, "side");
    let after: Vec<String> = (1..=20).map(|n| empty_commit(&path, &format!("c{n}"))).collect();
    OldConnections { _dir: dir, path, a, side, merge, after }
}

#[test]
#[serial_test::serial]
fn old_forks_and_merges_stay_drawn_with_exact_elided_runs() {
    let repo = old_connections();
    let _guard = DirGuard::enter(&repo.path);
    let fork = &repo.a[10];
    let context = &repo.a[9];

    let (graph, _) = gather(&input("side", &["main", "side"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("focused view");
    // Positions from main's tip: c20..c1 are 0..19, the merge 20, b3..b1
    // 21..23, the fork a10 24, and one context commit below it.
    let mut expected = vec![LaneEntry::Commit(context.clone()), LaneEntry::Commit(fork.clone()), LaneEntry::Elided(3), LaneEntry::Commit(repo.merge.clone()), LaneEntry::Elided(15)];
    expected.extend(commits(&repo.after[15..].iter().collect::<Vec<_>>()));
    assert_eq!(graph.default_entries, expected);
    let side = line(&graph, "side");
    assert_eq!(side.fork_sha.as_ref(), Some(fork));
    assert_eq!(side.merged_into.as_ref(), Some(&repo.merge));
    let mut side_expected = vec![LaneEntry::Elided(3)];
    side_expected.extend(commits(&repo.side[3..].iter().collect::<Vec<_>>()));
    assert_eq!(side.entries, side_expected);
    assert!(!graph.incomplete);

    let (graph, _) = gather(&input("main", &["main", "side"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");
    let mut expected = vec![LaneEntry::Commit(context.clone()), LaneEntry::Commit(fork.clone()), LaneEntry::Elided(3), LaneEntry::Commit(repo.merge.clone()), LaneEntry::Elided(10)];
    expected.extend(commits(&repo.after[10..].iter().collect::<Vec<_>>()));
    assert_eq!(graph.default_entries, expected);
    assert_eq!(line(&graph, "side").merged_into.as_ref(), Some(&repo.merge));
    assert!(!graph.incomplete);
    assert!(has_merge(&mermaid(&graph), "side", &repo.merge));
}

/// A `--depth 3` clone of a repository with an unmerged `feature` (`d1..d4`,
/// then `f1`, `f2`): the clone has `d2..d4` and `f1`, `f2`, so `feature`'s
/// merge base with `main` is present but `d1` is not.
fn shallow_clone() -> (tempfile::TempDir, PathBuf, Vec<String>, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    fs::create_dir(&source).unwrap();
    init_repo(&source);
    let main: Vec<String> = (1..=4).map(|n| commit(&source, &format!("d{n}"))).collect();
    run_git(&source, &["checkout", "-q", "-b", "feature"]);
    let feature: Vec<String> = (1..=2).map(|n| commit(&source, &format!("f{n}"))).collect();
    run_git(&source, &["checkout", "-q", "main"]);
    let clone = dir.path().join("clone");
    // `file:///C:/…` on Windows, `file:///tmp/…` elsewhere.
    let source_path = source.to_string_lossy().replace('\\', "/");
    let url = format!("file://{}{source_path}", if source_path.starts_with('/') { "" } else { "/" });
    run_git(dir.path(), &["clone", "-q", "--depth", "3", "--no-single-branch", &url, clone.to_str().unwrap()]);
    run_git(&clone, &["branch", "feature", "origin/feature"]);
    assert_eq!(git_output(&clone, &["rev-parse", "--is-shallow-repository"]), "true");
    (dir, clone, main, feature)
}

#[test]
#[serial_test::serial]
fn a_shallow_clone_draws_what_it_can_verify_and_reports_the_rest() {
    let (_dir, clone, main, feature) = shallow_clone();
    let _guard = DirGuard::enter(&clone);

    let (graph, _) = gather(&input("feature", &["main", "feature"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("a shallow clone still has a graph");
    assert!(graph.incomplete, "a shallow \"no\" is never trusted");
    let feature_line = line(&graph, "feature");
    assert_eq!(feature_line.entries, commits(&[&feature[0], &feature[1]]), "the verified commits remain");
    assert_eq!(feature_line.fork_sha.as_ref(), Some(&main[3]), "a positive merge base is trusted");
    assert_eq!(feature_line.merged_into, None);
    assert_eq!(graph.default_entries, commits(&[&main[2], &main[3]]));

    let (graph, _) = gather(&input("main", &["main", "feature"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");
    assert!(graph.incomplete);
    assert_eq!(graph.default_entries, commits(&[&main[1], &main[2], &main[3]]), "only the commits the clone has");
    assert_eq!(line(&graph, "feature").entries, commits(&[&feature[0], &feature[1]]));
}

#[test]
#[serial_test::serial]
fn a_deleted_recorded_parent_falls_back_to_the_default_branch() {
    let repo = branches();
    let _guard = DirGuard::enter(&repo.path);
    run_git(&repo.path, &["checkout", "-q", "-b", "gone", "main"]);
    let g1 = commit(&repo.path, "g1");
    run_git(&repo.path, &["checkout", "-q", "-b", "orphan"]);
    let o1 = commit(&repo.path, "o1");
    run_git(&repo.path, &["checkout", "-q", "main"]);
    run_git(&repo.path, &["branch", "-q", "-D", "gone"]);
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "orphan", "gone", 20);

    let (graph, _) = gather(&input("orphan", &["main", "orphan"], forks), true, false);
    let graph = graph.expect("focused view");

    let names: Vec<&str> = graph.lines.iter().map(|line| line.branch.as_str()).collect();
    assert_eq!(names, ["orphan"]);
    let orphan = line(&graph, "orphan");
    assert_eq!(orphan.parent, None);
    assert_eq!(orphan.fork_sha.as_ref(), Some(&repo.c3));
    assert_eq!(orphan.entries, commits(&[&g1, &o1]));
    assert!(!graph.incomplete);
}

/// Four worktree branches, one merged; a viewport that fits the default lane
/// and two lanes keeps the two most recently active, merged or not.
#[test]
#[serial_test::serial]
fn a_merged_lane_competes_under_the_height_cap_by_activity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    commit(&path, "r");
    let branch_at = |name: &str, time: i64| {
        run_git(&path, &["checkout", "-q", "-b", name, "main"]);
        let date = format!("@{time} +0000");
        let status = Command::new("git")
            .current_dir(&path)
            .args(["commit", "-q", "--allow-empty", "-m", name])
            .env("GIT_COMMITTER_DATE", &date)
            .env("GIT_AUTHOR_DATE", &date)
            .status()
            .unwrap();
        assert!(status.success());
        run_git(&path, &["checkout", "-q", "main"]);
    };
    branch_at("old", 1_000);
    branch_at("merged", 3_000);
    branch_at("newest", 4_000);
    branch_at("middle", 2_000);
    let merge = merge_no_ff(&path, "merged");
    let _guard = DirGuard::enter(&path);

    let (graph, _) = gather(&input("main", &["main", "old", "merged", "newest", "middle"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");
    assert_eq!(line(&graph, "merged").merged_into.as_ref(), Some(&merge));

    // 16 units per lane (default included) at the 1.25 scale is
    // `ceil(1.25 × lanes)` rows. An 8-row terminal caps the graph at 4 rows:
    // the default lane and two more.
    let viewport = GraphViewport {
        columns: 200,
        rows: 8,
        cell: CellSize::FALLBACK,
    };
    let measure = |mermaid: &str| {
        let lanes = 1 + mermaid.lines().filter(|line| line.trim_start().starts_with("branch ")).count();
        Some(NaturalSize { width: 100.0, height: 16.0 * lanes as f32 })
    };
    let plan = graph.to_git_graph(&PrListing::default(), None).plan_with(viewport, &measure).expect("plan");
    assert_eq!(plan.hidden_lanes, 2, "{}", plan.mermaid);
    assert!(plan.mermaid.contains("branch newest"), "{}", plan.mermaid);
    assert!(plan.mermaid.contains("branch merged"), "the merged lane is the second most active: {}", plan.mermaid);
    assert!(has_merge(&plan.mermaid, "merged", &merge), "{}", plan.mermaid);
    assert!(!plan.mermaid.contains("branch middle") && !plan.mermaid.contains("branch old"), "{}", plan.mermaid);
}

#[test]
#[serial_test::serial]
fn classify_names_every_integration_and_tries_candidates_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let _r = commit(&path, "r");
    let d1 = commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "merged"]);
    let m1 = commit(&path, "m1");
    run_git(&path, &["checkout", "-q", "-b", "indirect", &d1]);
    let i1 = commit(&path, "i1");
    run_git(&path, &["checkout", "-q", "-b", "carrier", &d1]);
    commit(&path, "k1");
    merge_no_ff(&path, "indirect");
    run_git(&path, &["checkout", "-q", "-b", "unmerged", &d1]);
    let u1 = commit(&path, "u1");
    run_git(&path, &["checkout", "-q", "main"]);
    let d2 = commit(&path, "d2");
    let merge = merge_no_ff(&path, "merged");
    merge_no_ff(&path, "carrier");
    let tip = git_output(&path, &["rev-parse", "HEAD"]);
    let _guard = DirGuard::enter(&path);
    let (history, read) = topology::History::read();
    assert_eq!(read, Ok(()));

    assert_eq!(history.classify(&u1, &[&tip]), Ok(topology::Integration::Unmerged));
    assert_eq!(
        history.classify(&m1, &[&u1, &tip]),
        Ok(topology::Integration::MergedDirectly {
            candidate: 1,
            merge: merge.clone(),
            first_parent: d2.clone(),
        }),
        "the first candidate that contains the tip decides"
    );
    assert_eq!(
        history.classify(&i1, &[&tip]),
        Ok(topology::Integration::IntegratedOtherwise {
            candidate: 0,
            first_parent: merge.clone(),
        })
    );
    // On the first-parent chain, below the tip and at it.
    assert_eq!(history.classify(&d1, &[&tip]), Ok(topology::Integration::NoSeparateHistory { candidate: 0 }));
    assert_eq!(history.classify(&tip, &[&tip]), Ok(topology::Integration::NoSeparateHistory { candidate: 0 }));
    // A git failure is unknown, never "unmerged".
    assert_eq!(history.classify(&"0".repeat(40), &[&tip]), Err(topology::GatherGap));
    assert_eq!(history.merge_base(&u1, &"f".repeat(40)), Err(topology::GatherGap));
}

#[test]
#[serial_test::serial]
fn first_parent_entries_place_only_anchors_on_the_lane() {
    let repo = old_connections();
    let _guard = DirGuard::enter(&repo.path);
    let history = topology::History::complete();
    let main_tip = repo.after.last().unwrap();
    let side_tip = repo.side.last().unwrap();

    // `side`'s commits are the merge's second parent: not on main's chain.
    let built = history
        .first_parent_entries(main_tip, topology::Extent::Open { cap_window: false }, 3, &[&repo.side[1], &repo.a[5]])
        .unwrap();
    assert!(built.placed.contains(&repo.a[5]));
    assert!(!built.placed.contains(&repo.side[1]), "an anchor off the chain is not placed");
    assert!(!built.entries.contains(&LaneEntry::Commit(repo.side[1].clone())));
    // c20..c18 shown (0..2), a5 at 29, a4 below it; 3..28 folded.
    assert_eq!(
        built.entries,
        vec![
            LaneEntry::Commit(repo.a[4].clone()),
            LaneEntry::Commit(repo.a[5].clone()),
            LaneEntry::Elided(26),
            LaneEntry::Commit(repo.after[17].clone()),
            LaneEntry::Commit(repo.after[18].clone()),
            LaneEntry::Commit(repo.after[19].clone()),
        ]
    );
    assert!(built.last_active.is_some());
    assert!(!built.gap);

    // A branch lane ends where the stop's history begins, so an anchor below
    // it is not on the lane even though it is on the tip's first-parent chain.
    let stop = [repo.a[10].as_str()];
    let built = history
        .first_parent_entries(side_tip, topology::Extent::Until(&stop), 2, &[&repo.side[2], &repo.a[9]])
        .unwrap();
    assert_eq!(
        built.entries,
        vec![
            LaneEntry::Elided(2),
            LaneEntry::Commit(repo.side[2].clone()),
            LaneEntry::Elided(3),
            LaneEntry::Commit(repo.side[6].clone()),
            LaneEntry::Commit(repo.side[7].clone()),
        ]
    );
    assert_eq!(built.placed, HashSet::from([repo.side[2].clone()]));

    // An unreadable tip is the only error.
    assert_eq!(
        history.first_parent_entries(&"0".repeat(40), topology::Extent::Open { cap_window: false }, 3, &[]),
        Err(topology::GatherGap)
    );
}
