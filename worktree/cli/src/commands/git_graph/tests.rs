use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use biscuit_terminal::components::git_graph::{GitGraphPlan, GraphViewport, LaneEntry, LaneMerge, NaturalSize};
use biscuit_terminal::components::mermaid::MermaidTheme;
use biscuit_terminal::components::terminal_image::ImageWidth;
use biscuit_terminal::discovery::fonts::CellSize;
use biscuit_visualized::mermaid::{CommitGeometry, GitGraphGeometry, MermaidDiagram};
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
    assert_eq!(merge_destinations(line("chore/merged")), Vec::<&String>::new(), "a fast-forward has no merge commit");
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
    //
    // Each lane's boundary (the first parent of its oldest commit) is then
    // classified, so no earlier merge goes unseen. The budget for a lane
    // shorter than its window whose boundary is an ordinary fork is two more
    // calls, one `--is-ancestor` and one first-parent chain, plus one
    // `--is-ancestor` per candidate tried before the one holding the boundary
    // (a full window adds one `log` for the boundary). Here: three lanes, so
    // three of each.
    assert_eq!(count(&calls, "merge-base") - count_merge_bases(&calls), 5 + 3, "got {calls:?}");
    assert_eq!(count_merge_bases(&calls), 3, "one per branch with a lane, got {calls:?}");
    assert_eq!(count(&calls, "log"), 4, "the default lane and one per branch lane, got {calls:?}");
    // chore/merged's contained tip needs its first-parent chain, which is
    // empty for a fast-forward; no line reached its window. Each boundary is
    // on its candidate's first-parent chain, so it needs no `--ancestry-path`.
    assert_eq!(count(&calls, "rev-list"), 1 + 3, "got {calls:?}");
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
    // Classifying the tip and the lane's boundary adds one `merge-base
    // --is-ancestor` each (a yes/no question, not a merge base); the
    // unmerged branch's fork reuses verbose's answer.
    assert_eq!(count_merge_bases(&calls), 1, "got {calls:?}");
    assert_eq!(count(&calls, "merge-base"), 3, "got {calls:?}");
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

/// The merge commits `line` merged into, oldest first.
fn merge_destinations(line: &GraphLine) -> Vec<&String> {
    line.merges.iter().map(|merge| &merge.destination).collect()
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
    assert_eq!(
        wt_ux.merges,
        [LaneMerge { source: repo.w2.clone(), destination: repo.merge.clone() }],
        "the merge's source is the branch tip"
    );
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
    assert_eq!(merge_destinations(wt_ux), [&repo.merge]);
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
    assert_eq!(merge_destinations(wt_ux), [&repo.m103]);

    let sniff = line(&graph, "fix/sniff");
    assert_eq!(sniff.parent.as_deref(), Some("fix/wt-ux"));
    assert_eq!(sniff.fork_sha.as_ref(), Some(&repo.w2), "forks at its parent's tip");
    assert_eq!(merge_destinations(sniff), [&repo.m104]);
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

/// A commit on `tree` with `parents` (first parent first), made without
/// touching the checkout; returns its full SHA.
///
/// Each commit is dated one second after the previous one, because the
/// height cap ranks lanes by their tip's commit time in whole seconds: with
/// wall-clock dates, a fast host gives several tips the same second and the
/// hidden lane changes from host to host.
fn commit_on(path: &Path, tree: &str, parents: &[&str], message: &str) -> String {
    static SEQUENCE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("clock").as_secs() as i64;
    let date = format!("{} +0000", now + SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let mut args = vec!["commit-tree", "-m", message];
    for parent in parents {
        args.extend(["-p", *parent]);
    }
    args.push(tree);
    let output = Command::new("git")
        .current_dir(path)
        .args(&args)
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .output()
        .expect("git should be installed");
    assert!(output.status.success(), "git {args:?} failed in {path:?}");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// `count` commits after `from`, oldest first.
fn chain_on(path: &Path, tree: &str, from: &str, prefix: &str, count: usize) -> Vec<String> {
    let mut chain: Vec<String> = Vec::with_capacity(count);
    for n in 1..=count {
        let parent = chain.last().map_or(from, String::as_str).to_string();
        chain.push(commit_on(path, tree, &[&parent], &format!("{prefix}{n}")));
    }
    chain
}

/// `feat/schema-enhancement`'s own commits in [`observed_sparse_lanes`].
const SPARSE_SCHEMA_COMMITS: usize = 55;
/// `fix/wt-ux`'s commits up to `W1`, the tip `M103` merged.
const SPARSE_WT_UX_BEFORE_MERGE: usize = 8;
/// `fix/wt-ux`'s commits after `W1` and before it merges `main` back (`B1`).
const SPARSE_WT_UX_AFTER_MERGE: usize = 64;
/// `fix/wt-ux`'s commits after `B1`.
const SPARSE_WT_UX_AFTER_SYNC: usize = 3;
/// `fix/sniff`'s own commits, forked at `W1`.
const SPARSE_SNIFF_COMMITS: usize = 94;

/// The history `wt list` drew sparsely on 2026-09-27, shape for shape:
///
/// - `main`: `r`, `d1..d12`, `M103` (merges `W1`), `d13`, `M104` (merges
///   `fix/sniff`'s tip); `origin/main` = `main` = `M104`.
/// - `feat/schema-enhancement` forks at `d2` with 55 commits, unmerged.
/// - `fix/wt-ux` (recorded parent `main`) forks at `d5`: `w1..w8` (`W1` =
///   `w8`), then continues after `M103` with `w9..w72`, merges `main` back at
///   `B1` (first parent `w72`, second `M104`), then `w73..w75`.
/// - `fix/sniff` (recorded parent `fix/wt-ux`) forks at `W1` with 94
///   commits, merged into `main` directly by `M104`.
///
/// Every commit shares `r`'s tree and is made with `commit-tree`, so the
/// ~250-commit history costs one Git call per commit.
struct ObservedSparseLanes {
    _dir: tempfile::TempDir,
    path: PathBuf,
    d: Vec<String>,
    schema: Vec<String>,
    /// `w1..w75`, first-parent order; `B1` is not in it.
    wt_ux: Vec<String>,
    sniff: Vec<String>,
    m103: String,
    m104: String,
    b1: String,
    forks: ForkOriginStore,
}

impl ObservedSparseLanes {
    /// `W1`: `fix/wt-ux`'s tip when `M103` merged it, and `fix/sniff`'s fork.
    fn w1(&self) -> &String {
        &self.wt_ux[SPARSE_WT_UX_BEFORE_MERGE - 1]
    }

    fn branches(&self) -> [&'static str; 4] {
        ["main", "feat/schema-enhancement", "fix/wt-ux", "fix/sniff"]
    }
}

fn observed_sparse_lanes() -> ObservedSparseLanes {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let r = commit(&path, "r");
    let tree = git_output(&path, &["rev-parse", "HEAD^{tree}"]);

    let mut d = vec![r.clone()];
    d.extend(chain_on(&path, &tree, &r, "d", 12));
    let schema = chain_on(&path, &tree, &d[2], "s", SPARSE_SCHEMA_COMMITS);
    let mut wt_ux = chain_on(&path, &tree, &d[5], "w", SPARSE_WT_UX_BEFORE_MERGE);
    let w1 = wt_ux.last().unwrap().clone();
    let m103 = commit_on(&path, &tree, &[&d[12], &w1], "Merge pull request #103 from fix/wt-ux");
    let sniff = chain_on(&path, &tree, &w1, "n", SPARSE_SNIFF_COMMITS);
    let continued = chain_on(&path, &tree, &w1, "w", SPARSE_WT_UX_AFTER_MERGE);
    wt_ux.extend(continued);
    d.push(commit_on(&path, &tree, &[&m103], "d13"));
    let m104 = commit_on(&path, &tree, &[&d[13], sniff.last().unwrap()], "Merge pull request #104 from fix/sniff");
    let b1 = commit_on(&path, &tree, &[wt_ux.last().unwrap(), &m104], "Merge branch 'main' into fix/wt-ux");
    let synced = chain_on(&path, &tree, &b1, "x", SPARSE_WT_UX_AFTER_SYNC);
    wt_ux.extend(synced);

    for (branch, tip) in [
        ("main", &m104),
        ("feat/schema-enhancement", schema.last().unwrap()),
        ("fix/wt-ux", wt_ux.last().unwrap()),
        ("fix/sniff", sniff.last().unwrap()),
    ] {
        run_git(&path, &["update-ref", &format!("refs/heads/{branch}"), tip]);
    }
    set_origin_main(&path, &m104);
    run_git(&path, &["reset", "-q", "--hard", "main"]);
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "feat/schema-enhancement", "main", 10);
    forked(&mut forks, "fix/wt-ux", "main", 20);
    forked(&mut forks, "fix/sniff", "fix/wt-ux", 30);
    ObservedSparseLanes { _dir: dir, path, d, schema, wt_ux, sniff, m103, m104, b1, forks }
}

#[test]
#[serial_test::serial]
fn observed_sparse_lanes_fixture_has_the_observed_topology() {
    let repo = observed_sparse_lanes();
    let at = |rev: &str| git_output(&repo.path, &["rev-parse", rev]);
    let count = |range: &str| git_output(&repo.path, &["rev-list", "--count", range]);

    assert_eq!(at("main"), repo.m104);
    assert_eq!(at("refs/remotes/origin/main"), repo.m104, "origin/main is main");
    assert_eq!(at("HEAD"), repo.m104, "the main checkout is on main");
    let parents = |merge: &str| git_output(&repo.path, &["rev-list", "--parents", "-n", "1", merge]);
    assert_eq!(parents(&repo.m103), format!("{} {} {}", repo.m103, repo.d[12], repo.w1()));
    assert_eq!(parents(&repo.m104), format!("{} {} {}", repo.m104, repo.d[13], repo.sniff.last().unwrap()));
    let wt_ux_before_sync = &repo.wt_ux[SPARSE_WT_UX_BEFORE_MERGE + SPARSE_WT_UX_AFTER_MERGE - 1];
    assert_eq!(parents(&repo.b1), format!("{} {} {}", repo.b1, wt_ux_before_sync, repo.m104));

    assert_eq!(at("fix/wt-ux"), *repo.wt_ux.last().unwrap());
    assert_eq!(at("fix/sniff"), *repo.sniff.last().unwrap());
    assert_eq!(at("feat/schema-enhancement"), *repo.schema.last().unwrap());
    assert_eq!(git_output(&repo.path, &["merge-base", "main", "feat/schema-enhancement"]), repo.d[2]);
    assert_eq!(git_output(&repo.path, &["merge-base", "fix/sniff", "fix/wt-ux"]), *repo.sniff.last().unwrap(), "fix/wt-ux contains fix/sniff through main");
    assert_eq!(count("main..feat/schema-enhancement"), SPARSE_SCHEMA_COMMITS.to_string());
    assert_eq!(count(&format!("{}..fix/sniff", repo.w1())), SPARSE_SNIFF_COMMITS.to_string());
    assert_eq!(
        count("main..fix/wt-ux"),
        (SPARSE_WT_UX_AFTER_MERGE + 1 + SPARSE_WT_UX_AFTER_SYNC).to_string(),
        "fix/wt-ux's own commits after W1, and B1"
    );
    // W1 is on neither main's first-parent chain nor fix/wt-ux's own run.
    let first_parents = |tip: &str| git_output(&repo.path, &["rev-list", "--first-parent", tip]);
    assert!(!first_parents("main").contains(repo.w1().as_str()));
    assert!(first_parents("fix/wt-ux").contains(repo.w1().as_str()));
    assert!(!git_output(&repo.path, &["rev-list", "--first-parent", "main..fix/wt-ux"]).contains(repo.w1().as_str()));

    assert_eq!(repo.forks.get("fix/sniff").map(|fork| fork.base_branch.as_str()), Some("fix/wt-ux"));
    assert_eq!(repo.forks.get("fix/wt-ux").map(|fork| fork.base_branch.as_str()), Some("main"));
    for branch in repo.branches() {
        assert_eq!(at(branch).len(), 40, "{branch} exists");
    }
}

/// A plan at `columns`×`rows` whose measurement always fits, so nothing is
/// trimmed and every gathered commit is in the Mermaid text.
fn untrimmed_plan(graph: &GraphFacts, columns: u32, rows: u32) -> GitGraphPlan {
    graph
        .to_git_graph(&PrListing::default(), None)
        .plan_with(viewport(columns, rows), &|_: &str| Some(NaturalSize { width: 400.0, height: 120.0 }))
        .expect("plan")
}

/// The oldest laid-out commit on `lane`.
fn first_on_lane<'g>(geometry: &'g GitGraphGeometry, lane: &str) -> &'g CommitGeometry {
    geometry
        .commits
        .iter()
        .find(|commit| commit.lane == lane)
        .unwrap_or_else(|| panic!("no commit on {lane}: {:?}", geometry.commits))
}

/// `fix/sniff` is merged into `main` directly by `M104`, and its recorded
/// parent `fix/wt-ux` contains its tip only through `B1`'s merge of `main`.
/// Its fork `W1` is `M103`'s second parent: `fix/wt-ux`'s lane boundary, so
/// `fix/wt-ux` draws it merged into `M103`, and `fix/sniff` hangs from it
/// there.
#[test]
#[serial_test::serial]
fn a_direct_merge_into_the_default_branch_beats_the_parents_indirect_containment() {
    let repo = observed_sparse_lanes();
    let _guard = DirGuard::enter(&repo.path);
    let sniff_tip = repo.sniff.last().expect("sniff commits");

    let (graph, _) = gather(&input("main", &repo.branches(), repo.forks.clone()), true, false);
    let graph = graph.expect("base view");

    let sniff = line(&graph, "fix/sniff");
    assert_eq!(sniff.parent.as_deref(), Some("fix/wt-ux"));
    assert_eq!(merge_destinations(sniff), [&repo.m104]);
    assert_eq!(sniff.fork_sha.as_ref(), Some(repo.w1()), "merge-base(M104^1, tip)");
    let mut own_run = vec![LaneEntry::Elided(SPARSE_SNIFF_COMMITS - LINE_WINDOW)];
    own_run.extend(commits(&repo.sniff[SPARSE_SNIFF_COMMITS - LINE_WINDOW..].iter().collect::<Vec<_>>()));
    assert_eq!(sniff.entries, own_run, "fix/sniff's own first-parent run, nothing of fix/wt-ux or main");
    assert!(graph.default_entries.contains(&LaneEntry::Commit(repo.m104.clone())));
    let w1 = LaneEntry::Commit(repo.w1().clone());
    assert!(!graph.default_entries.contains(&w1), "W1 is not on main's lane");

    let wt_ux = line(&graph, "fix/wt-ux");
    assert!(wt_ux.entries.contains(&w1), "W1 is on fix/wt-ux's lane: {:?}", wt_ux.entries);
    assert_eq!(wt_ux.merges, [LaneMerge { source: repo.w1().clone(), destination: repo.m103.clone() }]);
    assert_eq!(wt_ux.fork_sha.as_ref(), Some(&repo.d[5]), "merge-base(M103^1, W1)");
    assert!(!graph.incomplete);
    assert_no_repeated_commit(&graph);

    let plan = untrimmed_plan(&graph, 200, 60);
    assert!(!plan.incomplete, "every fork and merge is drawn: {}", plan.mermaid);
    assert_eq!(plan.hidden_lanes, 0, "{}", plan.mermaid);
    assert!(has_merge(&plan.mermaid, "fix/sniff", &repo.m104), "{}", plan.mermaid);
    assert!(has_merge(&plan.mermaid, "fix/wt-ux", &repo.m103), "{}", plan.mermaid);
    let geometry = geometry_of(&plan.mermaid);
    let merge = laid_out(&geometry, &repo.m104).expect("M104 drawn");
    assert_eq!(merge.lane, "main", "the merge edge ends on the default lane");
    assert_eq!(merge.parents.len(), 2, "{merge:?}");
    assert!(repo.d[13].starts_with(merge.parents[0].as_str()), "{merge:?}");
    assert!(sniff_tip.starts_with(merge.parents[1].as_str()), "{merge:?}");
    let drawn_w1 = laid_out(&geometry, repo.w1()).expect("W1 drawn");
    assert_eq!(drawn_w1.lane, "fix/wt-ux", "{drawn_w1:?}");
    let start = first_on_lane(&geometry, "fix/sniff");
    assert_eq!(start.parents.len(), 1, "{start:?}");
    assert!(repo.w1().starts_with(start.parents[0].as_str()), "fix/sniff forks at W1: {start:?}");
}

/// The parent was fast-forwarded into `main` at `p2`, where `child` forked,
/// so the deferred direct merge's fork is on the default lane.
///
/// ```text
/// r - d1 - p1 - p2 - d2 - M (main)
///                \  \    /  \
///                 \  c1 - c2  \        (child, merged at M)
///                  p3 -------- B - p4  (parent, merged main back at B)
/// ```
#[test]
#[serial_test::serial]
fn a_deferred_direct_merge_with_a_drawn_fork_is_complete() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let _r = commit(&path, "r");
    let _d1 = commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "parent"]);
    let _p1 = commit(&path, "p1");
    let p2 = commit(&path, "p2");
    run_git(&path, &["checkout", "-q", "main"]);
    run_git(&path, &["merge", "-q", "--ff-only", "parent"]);
    run_git(&path, &["checkout", "-q", "-b", "child"]);
    let c1 = commit(&path, "c1");
    let c2 = commit(&path, "c2");
    run_git(&path, &["checkout", "-q", "main"]);
    let d2 = commit(&path, "d2");
    let merge = merge_no_ff(&path, "child");
    run_git(&path, &["checkout", "-q", "parent"]);
    let p3 = commit(&path, "p3");
    let sync = merge_no_ff(&path, "main");
    let p4 = commit(&path, "p4");
    run_git(&path, &["checkout", "-q", "main"]);
    let _guard = DirGuard::enter(&path);
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "parent", "main", 10);
    forked(&mut forks, "child", "parent", 20);

    let (graph, _) = gather(&input("main", &["main", "parent", "child"], forks), true, false);
    let graph = graph.expect("base view");

    let child = line(&graph, "child");
    assert_eq!(child.parent.as_deref(), Some("parent"));
    assert_eq!(merge_destinations(child), [&merge]);
    assert_eq!(child.fork_sha.as_ref(), Some(&p2), "merge-base(M^1, tip)");
    assert_eq!(child.entries, commits(&[&c1, &c2]));
    assert_eq!(line(&graph, "parent").entries, commits(&[&p3, &sync, &p4]));
    assert!(graph.default_entries.contains(&LaneEntry::Commit(p2.clone())), "{:?}", graph.default_entries);
    assert!(graph.default_entries.contains(&LaneEntry::Commit(d2.clone())));
    assert!(!graph.incomplete);

    let plan = untrimmed_plan(&graph, 120, 40);
    assert!(!plan.incomplete, "every connection is drawn: {}", plan.mermaid);
    assert!(has_merge(&plan.mermaid, "child", &merge), "{}", plan.mermaid);
    let geometry = geometry_of(&plan.mermaid);
    let start = first_on_lane(&geometry, "child");
    assert_eq!(start.parents.len(), 1, "{start:?}");
    assert!(p2.starts_with(start.parents[0].as_str()), "the lane forks at p2: {start:?}");
}

/// `child`'s tip `p1` is on its parent's first-parent chain, and `main` also
/// contains it through its merge of the parent.
///
/// ```text
/// r - d1 - d2 - M (main)
///       \      /
///        p1 - p2   (parent; child = p1)
/// ```
#[test]
#[serial_test::serial]
fn a_tip_on_the_parents_first_parent_chain_is_labeled_there_while_main_contains_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let _r = commit(&path, "r");
    let d1 = commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "parent"]);
    let p1 = commit(&path, "p1");
    let p2 = commit(&path, "p2");
    run_git(&path, &["branch", "child", &p1]);
    run_git(&path, &["checkout", "-q", "main"]);
    commit(&path, "d2");
    let merge = merge_no_ff(&path, "parent");
    let _guard = DirGuard::enter(&path);
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "parent", "main", 10);
    forked(&mut forks, "child", "parent", 20);

    let (graph, _) = gather(&input("main", &["main", "parent", "child"], forks), true, false);
    let graph = graph.expect("base view");

    let child = line(&graph, "child");
    assert!(child.entries.is_empty(), "no lane of its own: {child:?}");
    assert_eq!(merge_destinations(child), Vec::<&String>::new());
    assert_eq!(child.fork_sha, None);
    assert_eq!(child.tip_sha.as_ref(), Some(&p1));
    let parent = line(&graph, "parent");
    assert_eq!(parent.entries, commits(&[&p1, &p2]));
    assert_eq!(parent.fork_sha.as_ref(), Some(&d1));
    assert_eq!(merge_destinations(parent), [&merge]);
    assert!(!graph.incomplete);

    let plan = untrimmed_plan(&graph, 120, 40);
    assert!(!plan.incomplete, "{}", plan.mermaid);
    let geometry = geometry_of(&plan.mermaid);
    let labeled = laid_out(&geometry, &p1).expect("p1 drawn");
    assert_eq!(labeled.lane, "parent", "{labeled:?}");
    assert!(labeled.tags.iter().any(|tag| tag.text == "child"), "{labeled:?}");
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
    assert_eq!(merge_destinations(child), [&merge]);
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
    assert_eq!(merge_destinations(ff), Vec::<&String>::new(), "no fabricated merge");
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
    assert_eq!(merge_destinations(c), [&merge]);
    let d = line(&graph, "d");
    assert!(d.entries.is_empty());
    assert_eq!(merge_destinations(d), Vec::<&String>::new());
    assert_eq!(d.tip_sha.as_ref(), Some(&c2));
    assert_eq!(d.parent.as_deref(), Some("c"));
    assert!(!graph.incomplete);
    let text = mermaid(&graph);
    assert!(has_merge(&text, "c", &merge), "{text}");
    assert!(text.contains(&format!("commit id: \"{}\" tag: \"d\"", &c2[..7])), "{text}");
}

/// `b` was merged at `merge` and then got another commit. The lane's
/// boundary `b1` is `merge`'s second parent, so the earlier merge is drawn
/// mid-lane: `b1` merged into `merge`, then `b2`, and the lane forks at `d1`.
#[test]
#[serial_test::serial]
fn a_branch_continued_after_its_merge_draws_its_earlier_merge() {
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
    assert_eq!(b.entries, commits(&[&b1, &b2]));
    assert_eq!(b.fork_sha.as_ref(), Some(&d1));
    assert_eq!(b.merges, [LaneMerge { source: b1.clone(), destination: merge.clone() }]);
    assert!(!graph.default_entries.contains(&LaneEntry::Commit(b1.clone())));
    assert!(graph.default_entries.contains(&LaneEntry::Commit(d1.clone())));
    assert!(graph.default_entries.contains(&LaneEntry::Commit(merge.clone())));
    assert!(!graph.incomplete);
    assert_no_repeated_commit(&graph);

    let plan = untrimmed_plan(&graph, 120, 40);
    assert!(!plan.incomplete, "every connection is drawn: {}", plan.mermaid);
    assert!(has_merge(&plan.mermaid, "b", &merge), "{}", plan.mermaid);
    let geometry = geometry_of(&plan.mermaid);
    let drawn = laid_out(&geometry, &merge).expect("merge drawn");
    assert_eq!(drawn.lane, "main", "{drawn:?}");
    assert_eq!(drawn.parents.len(), 2, "{drawn:?}");
    assert!(d1.starts_with(drawn.parents[0].as_str()), "{drawn:?}");
    assert!(b1.starts_with(drawn.parents[1].as_str()), "the merge's second parent is b1: {drawn:?}");
    let after = laid_out(&geometry, &b2).expect("b2 drawn");
    assert_eq!(after.lane, "b", "{after:?}");
    assert_eq!(after.parents.len(), 1, "{after:?}");
    assert!(b1.starts_with(after.parents[0].as_str()), "b resumes from b1 after the merge: {after:?}");
    let start = first_on_lane(&geometry, "b");
    assert!(d1.starts_with(start.parents[0].as_str()), "the lane forks at d1: {start:?}");
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
    assert_eq!(merge_destinations(t), Vec::<&String>::new());
    assert!(graph.incomplete);
    assert_eq!(graph.default_entries.last(), Some(&LaneEntry::Commit(merge)));
    assert!(!mermaid(&graph).contains("merge "));
}

/// Records `branch` as created from `parent` at `base_sha`, as `wt create`
/// does.
fn forked_at(store: &mut ForkOriginStore, branch: &str, parent: &str, base_sha: &str, created_at: u64) {
    store.insert(
        branch,
        ForkOrigin {
            base_branch: parent.to_string(),
            base_sha: base_sha.to_string(),
            created_at,
        },
    );
}

/// `<sha> <parent>…`, as Git reads the commit in `path`.
fn parent_line(path: &Path, sha: &str) -> String {
    git_output(path, &["rev-list", "--parents", "-n", "1", sha])
}

/// A new repository with one commit `r` on `main`; returns its directory,
/// path, `r`, and `r`'s tree for [`commit_on`].
fn repo_with_root() -> (tempfile::TempDir, PathBuf, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let r = commit(&path, "r");
    let tree = git_output(&path, &["rev-parse", "HEAD^{tree}"]);
    (dir, path, r, tree)
}

/// Points each branch at its tip, then moves the `main` checkout with it.
fn set_branches(path: &Path, tips: &[(&str, &str)]) {
    for (branch, tip) in tips {
        run_git(path, &["update-ref", &format!("refs/heads/{branch}"), tip]);
    }
    run_git(path, &["reset", "-q", "--hard", "main"]);
}

/// Where the local `main` is in [`continued_after_merge`]; `origin/main` is
/// always the merge `C`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalMain {
    /// At `C`, as after `wt --ff`.
    AtMerge,
    /// At `P`, one merge behind `origin/main`.
    Behind,
    /// At `P'`, a commit on `P` that `origin/main` does not have.
    Diverged,
}

/// `fix/wt-ux`'s commits after it merges `main` back (`S`), up to `B`.
const CONTINUED_AFTER_SYNC: usize = 7;

/// PR #105's shape, observed on 2026-09-28 (`b47a046` is `P`, `eeb7154` is
/// `B`, `85852c0` is `C`, `572ef7d` is `N`):
///
/// ```text
/// r - d1 - d2 - d3 - d4 - P ------------------- C   (main, origin/main)
///      \                   \                   /
///       w1 - w2 - w3 ------ S - x1 - … - x7 (B)     (fix/sniff-pr at B)
///                                            \
///                                             N    (fix/wt-ux)
/// ```
///
/// `fix/wt-ux` merged `main` back at `S`, so `merge-base(C^1, B)` is `P`,
/// and it has more than [`LINE_WINDOW`] commits up to `B`. `fix/sniff-pr`
/// has the record `wt create` wrote for it (`fix/wt-ux` at `B`); `fix/wt-ux`
/// has none, as observed.
struct ContinuedAfterMerge {
    _dir: tempfile::TempDir,
    path: PathBuf,
    /// `r`, `d1..d4`.
    d: Vec<String>,
    /// `w1..w3`.
    w: Vec<String>,
    p: String,
    sync: String,
    /// `x1..x7`; `x7` is `B`.
    x: Vec<String>,
    c: String,
    n: String,
    /// `P'`, only in [`LocalMain::Diverged`].
    p_prime: Option<String>,
    local_main: LocalMain,
    forks: ForkOriginStore,
}

impl ContinuedAfterMerge {
    /// `B`: the merged tip, and `fix/sniff-pr`.
    fn b(&self) -> &String {
        self.x.last().expect("x commits")
    }

    fn branches(&self) -> [&'static str; 3] {
        ["main", "fix/wt-ux", "fix/sniff-pr"]
    }
}

fn continued_after_merge(local_main: LocalMain) -> ContinuedAfterMerge {
    let (dir, path, r, tree) = repo_with_root();
    let mut d = vec![r.clone()];
    d.extend(chain_on(&path, &tree, &r, "d", 4));
    let w = chain_on(&path, &tree, &d[1], "w", 3);
    let p = commit_on(&path, &tree, &[&d[4]], "P");
    let sync = commit_on(&path, &tree, &[&w[2], &p], "Merge branch 'main' into fix/wt-ux");
    let x = chain_on(&path, &tree, &sync, "x", CONTINUED_AFTER_SYNC);
    let b = x.last().unwrap().clone();
    let c = commit_on(&path, &tree, &[&p, &b], "Merge pull request #105 from fix/wt-ux");
    let n = commit_on(&path, &tree, &[&b], "N");
    let p_prime = (local_main == LocalMain::Diverged).then(|| commit_on(&path, &tree, &[&p], "P'"));
    let main = match local_main {
        LocalMain::AtMerge => c.clone(),
        LocalMain::Behind => p.clone(),
        LocalMain::Diverged => p_prime.clone().unwrap(),
    };
    set_branches(&path, &[("main", &main), ("fix/wt-ux", &n), ("fix/sniff-pr", &b)]);
    set_origin_main(&path, &c);
    let mut forks = ForkOriginStore::default();
    forked_at(&mut forks, "fix/sniff-pr", "fix/wt-ux", &b, 30);
    ContinuedAfterMerge { _dir: dir, path, d, w, p, sync, x, c, n, p_prime, local_main, forks }
}

#[test]
#[serial_test::serial]
fn continued_after_merge_fixture_has_the_observed_topology() {
    for local_main in [LocalMain::AtMerge, LocalMain::Behind, LocalMain::Diverged] {
        let repo = continued_after_merge(local_main);
        let at = |rev: &str| git_output(&repo.path, &["rev-parse", rev]);
        let first_parents = |tip: &str| git_output(&repo.path, &["rev-list", "--first-parent", tip]);

        assert_eq!(parent_line(&repo.path, &repo.c), format!("{} {} {}", repo.c, repo.p, repo.b()), "{local_main:?}");
        assert_eq!(parent_line(&repo.path, &repo.sync), format!("{} {} {}", repo.sync, repo.w[2], repo.p), "{local_main:?}");
        assert_eq!(parent_line(&repo.path, &repo.n), format!("{} {}", repo.n, repo.b()), "{local_main:?}");
        assert_eq!(parent_line(&repo.path, &repo.w[0]), format!("{} {}", repo.w[0], repo.d[1]), "{local_main:?}");
        assert_eq!(at("refs/remotes/origin/main"), repo.c, "{local_main:?}");
        let expected_main = match repo.local_main {
            LocalMain::AtMerge => &repo.c,
            LocalMain::Behind => &repo.p,
            LocalMain::Diverged => {
                let p_prime = repo.p_prime.as_ref().expect("P'");
                assert_eq!(parent_line(&repo.path, p_prime), format!("{p_prime} {}", repo.p));
                p_prime
            }
        };
        assert_eq!(at("main"), *expected_main, "{local_main:?}");
        assert_eq!(at("HEAD"), *expected_main, "the main checkout is on main: {local_main:?}");
        assert_eq!(at("fix/wt-ux"), repo.n, "{local_main:?}");
        assert_eq!(at("fix/sniff-pr"), *repo.b(), "{local_main:?}");

        // B is on fix/wt-ux's first-parent chain, not on main's or origin/main's.
        assert!(first_parents("fix/wt-ux").lines().any(|sha| sha == repo.b()), "{local_main:?}");
        assert!(!first_parents("refs/remotes/origin/main").lines().any(|sha| sha == repo.b()), "{local_main:?}");
        assert_eq!(git_output(&repo.path, &["merge-base", &repo.p, repo.b()]), repo.p, "the fork measured against B: {local_main:?}");
        let up_to_b: usize = git_output(&repo.path, &["rev-list", "--first-parent", "--count", &format!("{}..{}", repo.d[1], repo.b())]).parse().unwrap();
        assert_eq!(up_to_b, 3 + 1 + CONTINUED_AFTER_SYNC, "{local_main:?}");
        assert!(up_to_b > LINE_WINDOW, "{local_main:?}");

        let sniff = repo.forks.get("fix/sniff-pr").expect("fix/sniff-pr's record");
        assert_eq!((sniff.base_branch.as_str(), &sniff.base_sha), ("fix/wt-ux", repo.b()), "{local_main:?}");
        assert!(repo.forks.get("fix/wt-ux").is_none(), "fix/wt-ux has no record, as observed");
        for branch in repo.branches() {
            assert_eq!(at(branch).len(), 40, "{branch} exists");
        }
    }
}

/// `b` merged twice, then continued:
///
/// ```text
/// r - d1 - p1 - C1 - p2 - C2   (main)
///      \       /         /
///       b1 ----- b2 -----
///                  \
///                   n          (b)
/// ```
struct MergedTwice {
    _dir: tempfile::TempDir,
    path: PathBuf,
    d1: String,
    b1: String,
    p1: String,
    c1: String,
    b2: String,
    p2: String,
    c2: String,
    n: String,
}

fn merged_twice_and_continued() -> MergedTwice {
    let (dir, path, r, tree) = repo_with_root();
    let d1 = commit_on(&path, &tree, &[&r], "d1");
    let b1 = commit_on(&path, &tree, &[&d1], "b1");
    let p1 = commit_on(&path, &tree, &[&d1], "p1");
    let c1 = commit_on(&path, &tree, &[&p1, &b1], "Merge branch 'b'");
    let b2 = commit_on(&path, &tree, &[&b1], "b2");
    let p2 = commit_on(&path, &tree, &[&c1], "p2");
    let c2 = commit_on(&path, &tree, &[&p2, &b2], "Merge branch 'b' again");
    let n = commit_on(&path, &tree, &[&b2], "n");
    set_branches(&path, &[("main", &c2), ("b", &n)]);
    MergedTwice { _dir: dir, path, d1, b1, p1, c1, b2, p2, c2, n }
}

#[test]
#[serial_test::serial]
fn merged_twice_fixture_has_two_merges_of_one_lane() {
    let repo = merged_twice_and_continued();
    let first_parents = |tip: &str| git_output(&repo.path, &["rev-list", "--first-parent", tip]);

    assert_eq!(parent_line(&repo.path, &repo.c1), format!("{} {} {}", repo.c1, repo.p1, repo.b1));
    assert_eq!(parent_line(&repo.path, &repo.c2), format!("{} {} {}", repo.c2, repo.p2, repo.b2));
    assert_eq!(parent_line(&repo.path, &repo.b1), format!("{} {}", repo.b1, repo.d1));
    assert_eq!(parent_line(&repo.path, &repo.b2), format!("{} {}", repo.b2, repo.b1));
    assert_eq!(parent_line(&repo.path, &repo.n), format!("{} {}", repo.n, repo.b2));
    assert_eq!(git_output(&repo.path, &["rev-parse", "main"]), repo.c2);
    assert_eq!(git_output(&repo.path, &["rev-parse", "b"]), repo.n);
    let main_chain = first_parents("main");
    let b_chain = first_parents("b");
    for source in [&repo.b1, &repo.b2] {
        assert!(!main_chain.lines().any(|sha| sha == source), "{source} is off main's first-parent chain");
        assert!(b_chain.lines().any(|sha| sha == source), "{source} is on b's first-parent chain");
    }
}

/// A branch created at an already merged tip: `old` was merged at `C` and
/// deleted, and `new` was then created at its tip `b1` (the record says so)
/// and got `n1`. Topology alone reads the same as `old` continued.
///
/// ```text
/// r - d1 - p - C   (main)
///      \      /
///       b1 ---     (old, deleted)
///         \
///          n1      (new)
/// ```
struct NewBranchAtMergedTip {
    _dir: tempfile::TempDir,
    path: PathBuf,
    d1: String,
    b1: String,
    p: String,
    c: String,
    n1: String,
    /// `new`'s record: `main` at `b1`.
    forks: ForkOriginStore,
}

fn new_branch_at_merged_tip() -> NewBranchAtMergedTip {
    let (dir, path, r, tree) = repo_with_root();
    let d1 = commit_on(&path, &tree, &[&r], "d1");
    let b1 = commit_on(&path, &tree, &[&d1], "b1");
    let p = commit_on(&path, &tree, &[&d1], "p");
    let c = commit_on(&path, &tree, &[&p, &b1], "Merge branch 'old'");
    let n1 = commit_on(&path, &tree, &[&b1], "n1");
    set_branches(&path, &[("main", &c), ("new", &n1)]);
    let mut forks = ForkOriginStore::default();
    forked_at(&mut forks, "new", "main", &b1, 20);
    NewBranchAtMergedTip { _dir: dir, path, d1, b1, p, c, n1, forks }
}

#[test]
#[serial_test::serial]
fn new_branch_at_merged_tip_fixture_records_its_creation_at_the_merged_tip() {
    let repo = new_branch_at_merged_tip();

    assert_eq!(parent_line(&repo.path, &repo.c), format!("{} {} {}", repo.c, repo.p, repo.b1));
    assert_eq!(parent_line(&repo.path, &repo.b1), format!("{} {}", repo.b1, repo.d1));
    assert_eq!(parent_line(&repo.path, &repo.n1), format!("{} {}", repo.n1, repo.b1));
    assert_eq!(git_output(&repo.path, &["rev-parse", "new"]), repo.n1);
    assert!(git_output(&repo.path, &["branch", "--list", "old"]).is_empty(), "the merged branch is gone");
    let record = repo.forks.get("new").expect("new's record");
    assert_eq!((record.base_branch.as_str(), &record.base_sha), ("main", &repo.b1));
    assert_eq!(
        git_output(&repo.path, &["rev-list", "--first-parent", "--count", &format!("{}..new", repo.b1)]),
        "1",
        "the record is new's lane boundary, one commit below its tip"
    );
}

/// `t`'s old tip `t1` reached `main` only through `other`'s merge `O`, and
/// `t` then continued with `t2`.
///
/// ```text
/// r - d1 - d2 ---------- C   (main)
///      \ \              /
///       \ o1 ------- O       (other)
///        \          /
///         t1 -------
///           \
///            t2              (t)
/// ```
struct IndirectBoundary {
    _dir: tempfile::TempDir,
    path: PathBuf,
    d1: String,
    d2: String,
    t1: String,
    o1: String,
    o: String,
    c: String,
    t2: String,
}

fn indirect_boundary() -> IndirectBoundary {
    let (dir, path, r, tree) = repo_with_root();
    let d1 = commit_on(&path, &tree, &[&r], "d1");
    let t1 = commit_on(&path, &tree, &[&d1], "t1");
    let o1 = commit_on(&path, &tree, &[&d1], "o1");
    let o = commit_on(&path, &tree, &[&o1, &t1], "Merge branch 't' into other");
    let d2 = commit_on(&path, &tree, &[&d1], "d2");
    let c = commit_on(&path, &tree, &[&d2, &o], "Merge branch 'other'");
    let t2 = commit_on(&path, &tree, &[&t1], "t2");
    set_branches(&path, &[("main", &c), ("other", &o), ("t", &t2)]);
    IndirectBoundary { _dir: dir, path, d1, d2, t1, o1, o, c, t2 }
}

#[test]
#[serial_test::serial]
fn indirect_boundary_fixture_reaches_main_only_through_another_merge() {
    let repo = indirect_boundary();

    assert_eq!(parent_line(&repo.path, &repo.c), format!("{} {} {}", repo.c, repo.d2, repo.o));
    assert_eq!(parent_line(&repo.path, &repo.o), format!("{} {} {}", repo.o, repo.o1, repo.t1));
    assert_eq!(parent_line(&repo.path, &repo.t2), format!("{} {}", repo.t2, repo.t1));
    assert_eq!(parent_line(&repo.path, &repo.t1), format!("{} {}", repo.t1, repo.d1));
    assert_eq!(git_output(&repo.path, &["rev-parse", "t"]), repo.t2);
    let main_chain = git_output(&repo.path, &["rev-list", "--first-parent", "main"]);
    assert!(!main_chain.lines().any(|sha| sha == repo.t1 || sha == repo.o), "neither t1 nor O is on main's first-parent chain");
    assert_eq!(git_output(&repo.path, &["merge-base", "main", "t"]), repo.t1, "t's old tip is in main");
}

/// A `--depth <depth>` clone of [`merged_twice_and_continued`], with local
/// `main` and `b`.
///
/// - depth 1: the clone has `C2` and `n` only, so `b`'s own boundary `b2`
///   is past the shallow cutoff.
/// - depth 2: the clone has `C2`, `p2`, `b2`, and `n`, so the later merge
///   (`b2` into `C2`) can be verified and the older boundary `b1` is past the
///   cutoff.
struct ShallowMergedTwice {
    source: MergedTwice,
    _dir: tempfile::TempDir,
    clone: PathBuf,
}

fn shallow_merged_twice(depth: usize) -> ShallowMergedTwice {
    let source = merged_twice_and_continued();
    let dir = tempfile::tempdir().unwrap();
    let clone = dir.path().join("clone");
    // `file:///C:/…` on Windows, `file:///tmp/…` elsewhere.
    let source_path = source.path.to_string_lossy().replace('\\', "/");
    let url = format!("file://{}{source_path}", if source_path.starts_with('/') { "" } else { "/" });
    let depth = depth.to_string();
    run_git(dir.path(), &["clone", "-q", "--depth", &depth, "--no-single-branch", &url, clone.to_str().unwrap()]);
    run_git(&clone, &["branch", "b", "origin/b"]);
    ShallowMergedTwice { source, _dir: dir, clone }
}

#[test]
#[serial_test::serial]
fn shallow_merged_twice_fixtures_cut_history_where_expected() {
    let has = |clone: &Path, sha: &str| {
        Command::new("git")
            .current_dir(clone)
            .args(["cat-file", "-e", &format!("{sha}^{{commit}}")])
            .status()
            .expect("git should be installed")
            .success()
    };
    let visible_parents = |clone: &Path, sha: &str| git_output(clone, &["log", "--no-walk", "--format=%P", sha]);

    let crossing = shallow_merged_twice(1);
    let repo = &crossing.source;
    assert_eq!(git_output(&crossing.clone, &["rev-parse", "--is-shallow-repository"]), "true");
    assert_eq!(git_output(&crossing.clone, &["rev-parse", "main", "b"]), format!("{}\n{}", repo.c2, repo.n));
    assert!(!has(&crossing.clone, &repo.b2), "b's boundary is past the cutoff");
    assert_eq!(visible_parents(&crossing.clone, &repo.n), "", "n's parent is cut");

    let older = shallow_merged_twice(2);
    let repo = &older.source;
    assert_eq!(git_output(&older.clone, &["rev-parse", "--is-shallow-repository"]), "true");
    for present in [&repo.c2, &repo.p2, &repo.b2, &repo.n] {
        assert!(has(&older.clone, present), "{present} is in the depth-2 clone");
    }
    for absent in [&repo.b1, &repo.c1] {
        assert!(!has(&older.clone, absent), "{absent} is past the cutoff");
    }
    assert_eq!(visible_parents(&older.clone, &repo.c2), format!("{} {}", repo.p2, repo.b2), "the later merge is verifiable");
    assert_eq!(visible_parents(&older.clone, &repo.b2), "", "the older boundary b1 is cut");
}

/// No commit is drawn on two lanes.
fn assert_no_repeated_commit(graph: &GraphFacts) {
    let mut seen: HashMap<&String, &str> = HashMap::new();
    let lanes = std::iter::once((graph.default_branch.as_str(), &graph.default_entries)).chain(graph.lines.iter().map(|line| (line.branch.as_str(), &line.entries)));
    for (lane, entries) in lanes {
        for entry in entries {
            if let LaneEntry::Commit(sha) = entry
                && let Some(other) = seen.insert(sha, lane)
            {
                panic!("{sha} is on {other} and {lane}: {graph:?}");
            }
        }
    }
}

/// `fix/wt-ux`'s lane in [`continued_after_merge`] once its merge is
/// reconstructed: `S`, `w1..w3`, and `x1..x3` fold into `+7`, then `x4..x6`,
/// `B`, and `N`.
fn continued_lane(repo: &ContinuedAfterMerge) -> Vec<LaneEntry> {
    let mut lane = vec![LaneEntry::Elided(3 + 1 + 3)];
    lane.extend(commits(&[&repo.x[3], &repo.x[4], &repo.x[5], repo.b(), &repo.n]));
    lane
}

/// The continued lane forks at `P` (`merge-base(C^1, B)`, not the old merged
/// tip), draws `B` merged into `C`, and continues with `N`.
fn assert_continued_lane(graph: &GraphFacts, repo: &ContinuedAfterMerge) {
    let wt_ux = line(graph, "fix/wt-ux");
    assert_eq!(wt_ux.entries, continued_lane(repo), "{:?}", repo.local_main);
    assert_eq!(wt_ux.fork_sha.as_ref(), Some(&repo.p), "{:?}", repo.local_main);
    assert_eq!(
        wt_ux.merges,
        [LaneMerge { source: repo.b().clone(), destination: repo.c.clone() }],
        "{:?}",
        repo.local_main
    );
    assert_eq!(wt_ux.tip_sha.as_ref(), Some(&repo.n));
    assert!(!graph.incomplete, "{:?}: {graph:?}", repo.local_main);
    assert_no_repeated_commit(graph);
}

#[test]
#[serial_test::serial]
fn a_continued_branch_draws_its_earlier_merge_and_its_child_label_in_the_base_view() {
    let repo = continued_after_merge(LocalMain::AtMerge);
    let _guard = DirGuard::enter(&repo.path);

    let (graph, _) = gather(&input("main", &repo.branches(), repo.forks.clone()), true, false);
    let graph = graph.expect("base view");

    assert_continued_lane(&graph, &repo);
    let sniff = line(&graph, "fix/sniff-pr");
    assert_eq!(sniff.parent.as_deref(), Some("fix/wt-ux"));
    assert!(sniff.entries.is_empty(), "no history of its own: {sniff:?}");
    assert_eq!(sniff.tip_sha.as_ref(), Some(repo.b()), "labeled at B, on fix/wt-ux's lane");
    assert_eq!(sniff.fork_sha, None);
    assert!(sniff.merges.is_empty());
    assert_eq!(graph.default_entries, commits(&[&repo.d[0], &repo.d[1], &repo.d[2], &repo.d[3], &repo.d[4], &repo.p, &repo.c]));
}

#[test]
#[serial_test::serial]
fn a_continued_branch_draws_its_earlier_merge_in_both_focused_views() {
    let repo = continued_after_merge(LocalMain::AtMerge);
    let _guard = DirGuard::enter(&repo.path);

    let (graph, _) = gather(&input("fix/wt-ux", &repo.branches(), repo.forks.clone()), true, false);
    let graph = graph.expect("focused view from fix/wt-ux");
    assert_eq!(graph.lines.len(), 1, "only the current branch is selected: {:?}", graph.lines);
    assert_continued_lane(&graph, &repo);
    // The window stops just below the oldest connection, the fork `P`.
    assert_eq!(graph.default_entries, commits(&[&repo.d[4], &repo.p, &repo.c]));

    // From the child, its recorded parent is drawn with the reconstructed lane.
    let (graph, _) = gather(&input("fix/sniff-pr", &repo.branches(), repo.forks.clone()), true, false);
    let graph = graph.expect("focused view from fix/sniff-pr");
    assert_continued_lane(&graph, &repo);
    let sniff = line(&graph, "fix/sniff-pr");
    assert_eq!(sniff.parent.as_deref(), Some("fix/wt-ux"));
    assert!(sniff.entries.is_empty());
    assert_eq!(sniff.tip_sha.as_ref(), Some(repo.b()));
    assert!(graph.refs.contains(&("fix/wt-ux".to_string(), repo.n.clone())), "{:?}", graph.refs);
}

/// Before `wt --ff`: the merge is on `origin/main`. When local `main` is
/// behind, `C` is on the default lane (which runs to `origin/main`); when the
/// two have diverged, `C` is on the `origin/main` line.
#[test]
#[serial_test::serial]
fn a_continued_branch_merges_into_origin_main_before_a_fast_forward() {
    let repo = continued_after_merge(LocalMain::Behind);
    let _guard = DirGuard::enter(&repo.path);
    let (graph, _) = gather(&input("main", &repo.branches(), repo.forks.clone()), true, false);
    let graph = graph.expect("base view, behind");
    assert_continued_lane(&graph, &repo);
    assert_eq!(graph.default_entries.last(), Some(&LaneEntry::Commit(repo.c.clone())), "C is on the default lane");
    assert!(graph.default_entries.contains(&LaneEntry::Commit(repo.p.clone())));
    assert!(graph.lines.iter().all(|line| line.branch != "origin/main"), "no line of its own when only behind");
    drop(_guard);

    let repo = continued_after_merge(LocalMain::Diverged);
    let _guard = DirGuard::enter(&repo.path);
    let (graph, _) = gather(&input("main", &repo.branches(), repo.forks.clone()), true, false);
    let graph = graph.expect("base view, diverged");
    assert_continued_lane(&graph, &repo);
    let origin = line(&graph, "origin/main");
    assert_eq!(origin.entries, commits(&[&repo.c]), "C is on the origin/main line");
    assert_eq!(origin.fork_sha.as_ref(), Some(&repo.p));
    assert!(!graph.default_entries.contains(&LaneEntry::Commit(repo.c.clone())));
    assert_eq!(graph.default_entries.last(), Some(&LaneEntry::Commit(repo.p_prime.clone().expect("P'"))));
}

#[test]
#[serial_test::serial]
fn a_branch_merged_twice_draws_both_merges_oldest_first() {
    let repo = merged_twice_and_continued();
    let _guard = DirGuard::enter(&repo.path);

    let (graph, _) = gather(&input("main", &["main", "b"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");

    let b = line(&graph, "b");
    assert_eq!(b.entries, commits(&[&repo.b1, &repo.b2, &repo.n]));
    assert_eq!(b.fork_sha.as_ref(), Some(&repo.d1), "merge-base(C1^1, b1) of the oldest merge");
    assert_eq!(
        b.merges,
        [
            LaneMerge { source: repo.b1.clone(), destination: repo.c1.clone() },
            LaneMerge { source: repo.b2.clone(), destination: repo.c2.clone() },
        ]
    );
    for destination in [&repo.c1, &repo.c2, &repo.d1] {
        assert!(graph.default_entries.contains(&LaneEntry::Commit(destination.clone())), "{destination}: {:?}", graph.default_entries);
    }
    assert!(!graph.incomplete);
    assert_no_repeated_commit(&graph);
}

/// `t1` reached `main` through `other`'s merge, so no merge of `t` is drawn:
/// the facts are today's, and `GitGraph`'s notice accounts for the undrawn
/// fork.
#[test]
#[serial_test::serial]
fn a_boundary_integrated_through_another_merge_is_not_reconstructed() {
    let repo = indirect_boundary();
    let _guard = DirGuard::enter(&repo.path);

    let (graph, _) = gather(&input("main", &["main", "t"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");

    let t = line(&graph, "t");
    assert_eq!(t.entries, commits(&[&repo.t2]));
    assert_eq!(t.fork_sha.as_ref(), Some(&repo.t1), "the old tip, which no lane draws");
    assert!(t.merges.is_empty(), "{t:?}");
    assert!(!graph.incomplete, "a verified indirect answer is no gap");
    assert!(!graph.default_entries.contains(&LaneEntry::Commit(repo.t1.clone())));
}

#[test]
#[serial_test::serial]
fn a_shallow_boundary_invents_no_merge_and_keeps_the_verified_one() {
    // Depth 1: `b`'s boundary is past the cutoff, so nothing is reconstructed.
    let crossing = shallow_merged_twice(1);
    let _guard = DirGuard::enter(&crossing.clone);
    let (graph, _) = gather(&input("main", &["main", "b"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");
    let b = line(&graph, "b");
    assert!(b.merges.is_empty(), "{b:?}");
    assert_eq!(b.entries, commits(&[&crossing.source.n]));
    assert!(graph.incomplete);
    drop(_guard);

    // Depth 2: the later merge is verified and kept; the older boundary and
    // the fork are past the cutoff.
    let older = shallow_merged_twice(2);
    let repo = &older.source;
    let _guard = DirGuard::enter(&older.clone);
    let (graph, _) = gather(&input("main", &["main", "b"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");
    let b = line(&graph, "b");
    assert_eq!(b.merges, [LaneMerge { source: repo.b2.clone(), destination: repo.c2.clone() }]);
    assert_eq!(b.entries, commits(&[&repo.b2, &repo.n]));
    assert_eq!(b.fork_sha, None, "merge-base(p2, b2) is past the cutoff");
    assert!(graph.incomplete);
    assert!(!graph.default_entries.contains(&LaneEntry::Commit(repo.c1.clone())), "C1 is not in the clone");
    assert_no_repeated_commit(&graph);
}

#[test]
#[serial_test::serial]
fn a_branch_created_at_a_merged_tip_does_not_claim_the_old_merge() {
    let repo = new_branch_at_merged_tip();
    let _guard = DirGuard::enter(&repo.path);

    // The record says `new` was created at `b1`: `C` merged another branch.
    let (graph, _) = gather(&input("main", &["main", "new"], repo.forks.clone()), true, false);
    let graph = graph.expect("base view");
    let new = line(&graph, "new");
    assert_eq!(new.entries, commits(&[&repo.n1]));
    assert_eq!(new.fork_sha.as_ref(), Some(&repo.b1), "the ordinary fork (the old merged tip), which no lane draws");
    assert!(new.merges.is_empty(), "{new:?}");
    assert!(!graph.incomplete, "GitGraph's notice accounts for the undrawn fork");

    // Without the record, the topology is all there is.
    let (graph, _) = gather(&input("main", &["main", "new"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");
    let new = line(&graph, "new");
    assert_eq!(new.entries, commits(&[&repo.b1, &repo.n1]));
    assert_eq!(new.fork_sha.as_ref(), Some(&repo.d1));
    assert_eq!(new.merges, [LaneMerge { source: repo.b1.clone(), destination: repo.c.clone() }]);
    assert!(!graph.incomplete);
}

/// A `--depth <depth>` clone of `source` with every branch local.
fn shallow_clone_of(source: &Path, depth: usize, branches: &[&str]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let clone = dir.path().join("clone");
    let source_path = source.to_string_lossy().replace('\\', "/");
    let url = format!("file://{}{source_path}", if source_path.starts_with('/') { "" } else { "/" });
    run_git(dir.path(), &["clone", "-q", "--depth", &depth.to_string(), "--no-single-branch", &url, clone.to_str().unwrap()]);
    for branch in branches {
        run_git(&clone, &["branch", branch, &format!("origin/{branch}")]);
    }
    (dir, clone)
}

/// `new`'s gathered merges, fork, and whether the facts are incomplete.
type Cutoff = (Vec<LaneMerge>, Option<String>, bool);

fn new_line_facts(forks: ForkOriginStore) -> (Cutoff, Vec<Vec<String>>) {
    recorder::start_recording();
    let (graph, _) = gather(&input("main", &["main", "new"], forks), true, false);
    let calls = recorder::finish_recording();
    let graph = graph.expect("base view");
    let new = line(&graph, "new");
    ((new.merges.clone(), new.fork_sha.clone(), graph.incomplete), calls)
}

/// Every shape of `new`'s `base_sha` in the fork-origin file, written by
/// [`ForkOriginStore::save_atomic`] as `wt create` writes it, edited once per
/// cell, and read back by [`ForkOriginStore::load_from`]. The unedited file is
/// the cutoff and a file without `new`'s record is the reconstructed edge, so
/// every other cell is one of the two, or a gap.
#[test]
#[serial_test::serial]
fn fork_origin_cutoff_matrix() {
    let repo = new_branch_at_merged_tip();
    let store_dir = tempfile::tempdir().unwrap();
    let store_path = store_dir.path().join("fork-origins.json");
    repo.forks.save_atomic(&store_path).expect("save");
    let saved = fs::read_to_string(&store_path).unwrap();
    let field = format!("\"base_sha\": \"{}\"", repo.b1);
    assert_eq!(saved.matches(&field).count(), 1, "{saved}");
    let tree = git_output(&repo.path, &["rev-parse", "HEAD^{tree}"]);
    let unknown = "e".repeat(40);

    let cutoff: Cutoff = (Vec::new(), Some(repo.b1.clone()), false);
    let edge: Cutoff = (vec![LaneMerge { source: repo.b1.clone(), destination: repo.c.clone() }], Some(repo.d1.clone()), false);
    let with_value = |value: &str| saved.replace(&field, &format!("\"base_sha\": {value}"));
    let quoted = |sha: &str| with_value(&format!("\"{sha}\""));
    let cells: Vec<(&str, String, &Cutoff)> = vec![
        ("control: the unedited record, at B", saved.clone(), &cutoff),
        ("control: no record for new", saved.replace("\"new\"", "\"deleted\""), &edge),
        ("absent", saved.replace(&format!("{field},"), ""), &edge),
        ("explicit null", with_value("null"), &edge),
        ("wrong type", with_value("123"), &edge),
        ("empty", quoted(""), &edge),
        ("duplicate key", saved.replace(&field, &format!("{field}, {field}")), &edge),
        ("trailing content", format!("{saved}garbage"), &edge),
        ("abbreviated", quoted(&repo.b1[..7]), &edge),
        ("uppercase", quoted(&repo.b1.to_uppercase()), &edge),
        ("unknown object", quoted(&unknown), &edge),
        ("not a commit", quoted(&tree), &edge),
        ("off the chain: the parent's final tip", quoted(&repo.c), &edge),
        ("off the chain: a sibling", quoted(&repo.p), &edge),
        ("ancestor of B on the chain", quoted(&repo.d1), &edge),
        ("newer than B on the chain", quoted(&repo.n1), &cutoff),
    ];
    let _guard = DirGuard::enter(&repo.path);
    for (cell, text, expected) in &cells {
        fs::write(&store_path, text).unwrap();
        let forks = ForkOriginStore::load_from(&store_path);
        let (facts, calls) = new_line_facts(forks);
        assert_eq!(&facts, *expected, "{cell}:\n{text}");
        for value in ["", &repo.b1[..7], &repo.b1.to_uppercase()] {
            assert!(!calls.iter().flatten().any(|arg| arg == value), "{cell}: git was given {value:?}: {calls:?}");
        }
    }
    drop(_guard);

    // A shallow clone cannot tell an unknown record from one past its cut.
    let (_clone_dir, clone) = shallow_clone_of(&repo.path, 3, &["new"]);
    let _guard = DirGuard::enter(&clone);
    let shallow_cells: [(&str, String, Cutoff); 3] = [
        ("shallow control: no record", saved.replace("\"new\"", "\"deleted\""), (edge.0.clone(), edge.1.clone(), true)),
        ("shallow control: the record at B", saved.clone(), (Vec::new(), cutoff.1.clone(), true)),
        ("shallow: unknown object", quoted(&unknown), (Vec::new(), cutoff.1.clone(), true)),
    ];
    for (cell, text, expected) in &shallow_cells {
        fs::write(&store_path, text).unwrap();
        let (facts, _) = new_line_facts(ForkOriginStore::load_from(&store_path));
        assert_eq!(&facts, expected, "{cell}:\n{text}");
    }
}

/// An ordinary unmerged branch forked from the default lane gathers exactly
/// the facts it did before boundaries were classified (written from the
/// output of the code before that change).
#[test]
#[serial_test::serial]
fn an_ordinary_unmerged_branch_gathers_unchanged_facts() {
    let repo = branches();
    let _guard = DirGuard::enter(&repo.path);
    let time = |sha: &str| git_output(&repo.path, &["log", "-1", "--format=%ct", sha]).parse::<i64>().unwrap();

    let (graph, _) = gather(&input("main", &["main", "feature-a", "feature-b"], ForkOriginStore::default()), true, false);

    let expected = GraphFacts {
        default_branch: "main".to_string(),
        default_entries: commits(&[&repo.c1, &repo.c2, &repo.c3]),
        lines: vec![
            GraphLine::new("feature-a")
                .with_tip(repo.a1.clone())
                .forked_at(repo.c2.clone())
                .with_entries(commits(&[&repo.a1]))
                .with_last_active(time(&repo.a1)),
            GraphLine::new("feature-b")
                .with_tip(repo.b1.clone())
                .forked_at(repo.c3.clone())
                .with_entries(commits(&[&repo.b1]))
                .with_last_active(time(&repo.b1)),
        ],
        refs: vec![("main".to_string(), repo.c3.clone())],
        current_branch: "main".to_string(),
        incomplete: false,
    };
    assert_eq!(graph, Some(expected));
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
    assert_eq!(merge_destinations(side), [&repo.merge]);
    let mut side_expected = vec![LaneEntry::Elided(3)];
    side_expected.extend(commits(&repo.side[3..].iter().collect::<Vec<_>>()));
    assert_eq!(side.entries, side_expected);
    assert!(!graph.incomplete);

    let (graph, _) = gather(&input("main", &["main", "side"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");
    let mut expected = vec![LaneEntry::Commit(context.clone()), LaneEntry::Commit(fork.clone()), LaneEntry::Elided(3), LaneEntry::Commit(repo.merge.clone()), LaneEntry::Elided(10)];
    expected.extend(commits(&repo.after[10..].iter().collect::<Vec<_>>()));
    assert_eq!(graph.default_entries, expected);
    assert_eq!(merge_destinations(line(&graph, "side")), [&repo.merge]);
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
    assert_eq!(merge_destinations(feature_line), Vec::<&String>::new());
    assert_eq!(graph.default_entries, commits(&[&main[2], &main[3]]));

    let (graph, _) = gather(&input("main", &["main", "feature"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");
    assert!(graph.incomplete);
    assert_eq!(graph.default_entries, commits(&[&main[1], &main[2], &main[3]]), "only the commits the clone has");
    assert_eq!(line(&graph, "feature").entries, commits(&[&feature[0], &feature[1]]));
}

/// A `--depth 3` clone where `feature` is merged into `main` directly, and
/// its recorded parent `parent` does not contain it, which only a complete
/// clone could prove. `outer` contains `feature` indirectly, through
/// `carrier`'s merge.
///
/// ```text
/// d1 - d2 - d3 - d4 ------------- d5 - M (main)
///            \    \              /
///             p1   f1 - f2 ------   (feature)
///                   \    \
///                    \    k1 - K    (carrier)
///                     o1 ------ O   (outer)
/// ```
#[test]
#[serial_test::serial]
fn a_shallow_unknown_earlier_candidate_is_a_gap_even_beside_a_later_direct_merge() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    fs::create_dir(&source).unwrap();
    init_repo(&source);
    let d: Vec<String> = (1..=4).map(|n| commit(&source, &format!("d{n}"))).collect();
    run_git(&source, &["checkout", "-q", "-b", "feature"]);
    commit(&source, "f1");
    let f2 = commit(&source, "f2");
    run_git(&source, &["checkout", "-q", "-b", "carrier", &d[3]]);
    commit(&source, "k1");
    merge_no_ff(&source, "feature");
    run_git(&source, &["checkout", "-q", "-b", "outer", &d[3]]);
    let o1 = commit(&source, "o1");
    let outer = merge_no_ff(&source, "carrier");
    run_git(&source, &["checkout", "-q", "-b", "parent", &d[2]]);
    let p1 = commit(&source, "p1");
    run_git(&source, &["checkout", "-q", "main"]);
    let d5 = commit(&source, "d5");
    let merge = merge_no_ff(&source, "feature");
    let clone = dir.path().join("clone");
    // `file:///C:/…` on Windows, `file:///tmp/…` elsewhere.
    let source_path = source.to_string_lossy().replace('\\', "/");
    let url = format!("file://{}{source_path}", if source_path.starts_with('/') { "" } else { "/" });
    run_git(dir.path(), &["clone", "-q", "--depth", "3", "--no-single-branch", &url, clone.to_str().unwrap()]);
    for branch in ["feature", "parent", "outer"] {
        run_git(&clone, &["branch", branch, &format!("origin/{branch}")]);
    }
    assert_eq!(git_output(&clone, &["rev-parse", "--is-shallow-repository"]), "true");
    let _guard = DirGuard::enter(&clone);
    let (history, read) = topology::History::read();
    assert_eq!(read, Ok(()));

    assert_eq!(
        history.classify(&f2, &[&merge]),
        Ok(topology::Integration::MergedDirectly {
            candidate: 0,
            merge: merge.clone(),
            first_parent: d5.clone(),
            after_indirect: false,
        }),
        "the later candidate alone has a direct merge"
    );
    assert_eq!(history.classify(&f2, &[&p1, &merge]), Err(topology::GatherGap), "a shallow \"no\" is unknown");
    assert_eq!(
        history.classify(&f2, &[&outer, &p1]),
        Ok(topology::Integration::IntegratedOtherwise {
            candidate: 0,
            first_parent: o1.clone(),
        }),
        "a gap after an indirect match keeps that match"
    );

    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "feature", "parent", 20);
    let (graph, _) = gather(&input("main", &["main", "parent", "feature"], forks), true, false);
    let graph = graph.expect("a shallow clone still has a graph");
    assert!(graph.incomplete);
    let feature = line(&graph, "feature");
    assert_eq!(feature.parent.as_deref(), Some("parent"));
    assert_eq!(merge_destinations(feature), Vec::<&String>::new(), "no merge edge is invented");
    assert!(!mermaid(&graph).contains("merge "), "{}", mermaid(&graph));
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
    assert_eq!(merge_destinations(line(&graph, "merged")), [&merge]);

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
    let k1 = commit(&path, "k1");
    let carried = merge_no_ff(&path, "indirect");
    run_git(&path, &["checkout", "-q", "-b", "unmerged", &d1]);
    let u1 = commit(&path, "u1");
    run_git(&path, &["checkout", "-q", "main"]);
    let d2 = commit(&path, "d2");
    let merge = merge_no_ff(&path, "merged");
    merge_no_ff(&path, "carrier");
    let tip = git_output(&path, &["rev-parse", "HEAD"]);
    // `synced` contains everything main has, only through its merge of main;
    // `extended` has `m1` on its first-parent chain.
    run_git(&path, &["checkout", "-q", "-b", "synced", &d1]);
    let y1 = commit(&path, "y1");
    let synced = merge_no_ff(&path, "main");
    run_git(&path, &["checkout", "-q", "-b", "extended", &m1]);
    let extended = commit(&path, "e1");
    run_git(&path, &["checkout", "-q", "main"]);
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
            after_indirect: false,
        }),
        "the first candidate that contains the tip decides"
    );
    assert_eq!(
        history.classify(&m1, &[&synced, &tip]),
        Ok(topology::Integration::MergedDirectly {
            candidate: 1,
            merge: merge.clone(),
            first_parent: d2.clone(),
            after_indirect: true,
        }),
        "an indirect match defers to a later direct merge"
    );
    assert_eq!(
        history.classify(&i1, &[&synced, &tip]),
        Ok(topology::Integration::IntegratedOtherwise {
            candidate: 0,
            first_parent: y1.clone(),
        }),
        "with no stronger result, the first indirect match stands"
    );
    assert_eq!(
        history.classify(&m1, &[&extended, &tip]),
        Ok(topology::Integration::NoSeparateHistory { candidate: 0 }),
        "a first-parent match wins over a later direct merge"
    );
    assert_eq!(
        history.classify(&i1, &[&carried, &tip]),
        Ok(topology::Integration::MergedDirectly {
            candidate: 0,
            merge: carried.clone(),
            first_parent: k1.clone(),
            after_indirect: false,
        }),
        "a direct merge wins over a later indirect match"
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
        .first_parent_entries(main_tip, topology::Extent::Open { window: 3, cap_window: false }, &[&repo.side[1], &repo.a[5]])
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
    let window = history.lane_window(side_tip, &stop, 2).unwrap();
    let built = history
        .first_parent_entries(side_tip, topology::Extent::Until(&window), &[&repo.side[2], &repo.a[9]])
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
        history.first_parent_entries(&"0".repeat(40), topology::Extent::Open { window: 3, cap_window: false }, &[]),
        Err(topology::GatherGap)
    );
}

// ---------------------------------------------------------------------------
// End to end: gathered facts, planned with the real measurement, checked
// against the layout the image is drawn from (never the Mermaid text).
// ---------------------------------------------------------------------------

const ALPHA: &str = "feature/very-long-exact-branch-reference-alpha";
/// A label as long as `origin/very-long-exact-branch-reference-beta`; `wt list`
/// draws no remote-tracking ref but `origin/<default>`, so it is a local branch.
const BETA: &str = "fix/very-long-exact-branch-reference-beta";

/// Long labels as `wt list` meets them: `ALPHA` is an unmerged lane
/// with an open PR, `BETA` is a label on the default lane beside `main`, and
/// `origin/main` is one commit ahead of `main`.
///
/// ```text
/// r - d1 - d2 - d3 (BETA) - d4 (main) - d5 (origin/main)
///       \
///        a1 - a2   (ALPHA, PR #104 → main)
/// ```
struct LongLabels {
    _dir: tempfile::TempDir,
    path: PathBuf,
    d3: String,
    d4: String,
    d5: String,
    a2: String,
}

fn long_labels() -> LongLabels {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    commit(&path, "r");
    commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", ALPHA]);
    commit(&path, "a1");
    let a2 = commit(&path, "a2");
    run_git(&path, &["checkout", "-q", "main"]);
    commit(&path, "d2");
    let d3 = commit(&path, "d3");
    run_git(&path, &["branch", BETA]);
    let d4 = commit(&path, "d4");
    let d5 = commit(&path, "d5");
    set_origin_main(&path, &d5);
    run_git(&path, &["reset", "-q", "--hard", &d4]);
    LongLabels { _dir: dir, path, d3, d4, d5, a2 }
}

/// What the laid-out graph must show, by full SHA.
struct Evidence<'a> {
    /// `(tag, SHA)`: the label must be on that commit's emitted ID.
    tags: Vec<(&'a str, &'a str)>,
    /// `(merge, first parent, second parent)`.
    merges: Vec<(&'a str, &'a str, &'a str)>,
    /// `(child, merge source)`: the next commit on the source's lane after a
    /// merge source hangs from that source (a `+N` square once trimmed).
    post_merge: Vec<(&'a str, &'a str)>,
    /// The plan's notice: some fork, merge, or tagged commit is not drawn.
    incomplete: bool,
    /// Lanes the height cap leaves out at 120×40 and at 56×60.
    hidden_lanes: [usize; 2],
}

/// The laid-out commit emitted for `sha`. Display IDs are SHA prefixes of at
/// least seven characters; `+N` squares never match.
fn laid_out<'g>(geometry: &'g GitGraphGeometry, sha: &str) -> Option<&'g CommitGeometry> {
    geometry
        .commits
        .iter()
        .find(|commit| commit.id.len() >= 7 && sha.starts_with(commit.id.as_str()))
}

fn geometry_of(mermaid: &str) -> GitGraphGeometry {
    MermaidDiagram::new(mermaid)
        .with_theme(MermaidTheme::Default)
        .gitgraph_geometry()
        .unwrap_or_else(|error| panic!("{error}: {mermaid}"))
        .expect("a gitGraph")
}

fn viewport(columns: u32, rows: u32) -> GraphViewport {
    GraphViewport {
        columns,
        rows,
        cell: CellSize::FALLBACK,
    }
}

/// Plans `facts` at 120×40 and 56×60 with the real measurement and checks the
/// layout that renders: no two tags intersect, every expected tag is on its
/// SHA's emitted ID, merge parents are exact (a first parent may be a `+N`
/// square once trimmed), no tag text changes with the viewport, and a graph
/// wider than 56 columns is trimmed or shrunk there. `--width 40` is never
/// trimmed. Returns one report line per viewport for the implementation log.
fn assert_laid_out(name: &str, facts: &GraphFacts, prs: &PrListing, expected: &Evidence) -> Vec<String> {
    let graph = facts.to_git_graph(prs, None).with_theme(MermaidTheme::Default);
    let mut report = Vec::new();
    let mut tag_texts: Option<Vec<String>> = None;
    let mut ordinary_columns = None;
    for (vp, hidden_lanes) in [viewport(120, 40), viewport(56, 60)].into_iter().zip(expected.hidden_lanes) {
        let planned = graph.plan(vp).expect("a plan");
        let context = format!("{name} {}x{}:\n{}", vp.columns, vp.rows, planned.mermaid);
        assert_eq!(planned.incomplete, expected.incomplete, "{context}");
        assert_eq!(planned.hidden_lanes, hidden_lanes, "{context}");
        let geometry = geometry_of(&planned.mermaid);
        assert_eq!(geometry.tag_overlaps(), Vec::<(String, String)>::new(), "{context}");

        for (tag, sha) in &expected.tags {
            let commit = laid_out(&geometry, sha).unwrap_or_else(|| panic!("{tag}'s commit {sha} drawn: {context}"));
            assert!(
                commit.tags.iter().any(|placed| placed.text == *tag),
                "{tag} on {}: {:?}\n{context}",
                commit.id,
                commit.tags
            );
        }
        for (merge, first, second) in &expected.merges {
            let commit = laid_out(&geometry, merge).unwrap_or_else(|| panic!("merge {merge} drawn: {context}"));
            assert_eq!(commit.parents.len(), 2, "{:?}\n{context}", commit.parents);
            assert!(
                first.starts_with(commit.parents[0].as_str()) || (planned.trimmed_commits > 0 && commit.parents[0].starts_with('+')),
                "first parent of {}: {:?}\n{context}",
                commit.id,
                commit.parents
            );
            assert!(second.starts_with(commit.parents[1].as_str()), "second parent of {}: {:?}\n{context}", commit.id, commit.parents);
        }
        for (child, source) in &expected.post_merge {
            let drawn_source = laid_out(&geometry, source).unwrap_or_else(|| panic!("merge source {source} drawn: {context}"));
            let next = geometry
                .commits
                .iter()
                .find(|commit| commit.lane == drawn_source.lane && commit.index > drawn_source.index)
                .unwrap_or_else(|| panic!("a commit after {} on {}: {context}", drawn_source.id, drawn_source.lane));
            assert!(
                child.starts_with(next.id.as_str()) || (planned.trimmed_commits > 0 && next.id.starts_with('+')),
                "the commit after {} is {child}: {next:?}\n{context}",
                drawn_source.id
            );
            assert_eq!(next.parents, std::slice::from_ref(&drawn_source.id), "{} hangs from {}\n{context}", next.id, drawn_source.id);
        }

        let mut texts: Vec<String> = geometry.tag_boxes().iter().map(|(_, text, _)| text.to_string()).collect();
        texts.sort();
        match &tag_texts {
            None => tag_texts = Some(texts),
            Some(ordinary) => assert_eq!(&texts, ordinary, "no label is shortened or dropped: {context}"),
        }
        match ordinary_columns {
            None => ordinary_columns = Some(planned.columns),
            Some(ordinary) if ordinary > vp.columns => assert!(
                planned.trimmed_commits > 0 || planned.columns > vp.columns,
                "a {ordinary}-column graph is trimmed or shrunk at {}: {planned:?}",
                vp.columns
            ),
            Some(_) => {}
        }
        if planned.columns > vp.columns {
            let floor = graph.plan(viewport(1, vp.rows)).expect("a plan");
            assert_eq!(planned.mermaid, floor.mermaid, "wider than the viewport only once fully trimmed: {context}");
        }
        report.push(format!(
            "{name} {}x{}: columns={} rows={} trimmed={} step={:.1} natural_width={:.0}",
            vp.columns, vp.rows, planned.columns, planned.rows, planned.trimmed_commits, geometry.commit_step, geometry.width
        ));
    }

    // 60 rows: tall enough that the height cap hides no lane of any fixture.
    let explicit = facts
        .to_git_graph(prs, Some(ImageWidth::Characters(40)))
        .with_theme(MermaidTheme::Default)
        .plan(viewport(120, 60))
        .expect("a plan");
    assert_eq!(explicit.hidden_lanes, 0, "{name}");
    assert_eq!(explicit.columns, 40, "{name}");
    assert_eq!(explicit.trimmed_commits, 0, "{name}: an explicit width is never trimmed to");
    assert_eq!(explicit.mermaid, graph.mermaid().expect("mermaid"), "{name}: the whole graph, scaled");
    report
}

#[test]
#[serial_test::serial]
fn gathered_graphs_lay_out_with_exact_merges_and_no_overlapping_tags() {
    let mut report = Vec::new();

    let repo = merged_via_merge_commit();
    {
        let _guard = DirGuard::enter(&repo.path);
        let expected = Evidence {
            tags: vec![("main", &repo.d2), ("origin/main", &repo.merge)],
            merges: vec![(&repo.merge, &repo.d2, &repo.w2)],
            post_merge: vec![],
            incomplete: false,
            hidden_lanes: [0, 0],
        };
        for (view, current) in [("observation-1 focused", "fix/wt-ux"), ("observation-1 base", "main")] {
            let (graph, _) = gather(&input(current, &["main", "fix/wt-ux"], ForkOriginStore::default()), true, false);
            report.extend(assert_laid_out(view, &graph.expect("a graph"), &PrListing::default(), &expected));
        }
    }

    let repo = nested_parent_merged_into_default();
    {
        let _guard = DirGuard::enter(&repo.path);
        let (graph, _) = gather(&input("main", &["main", "fix/wt-ux", "fix/sniff"], repo.forks.clone()), true, false);
        let sniff_tip = repo.sniff.last().expect("sniff commits");
        report.extend(assert_laid_out(
            "observation-2 base",
            &graph.expect("a graph"),
            &PrListing::default(),
            &Evidence {
                tags: vec![("main", &repo.m103), ("origin/main", &repo.m104)],
                merges: vec![(&repo.m103, &repo.d1, &repo.w2), (&repo.m104, &repo.m103, sniff_tip)],
                post_merge: vec![],
                incomplete: false,
                hidden_lanes: [0, 0],
            },
        ));
    }

    let repo = long_labels();
    {
        let _guard = DirGuard::enter(&repo.path);
        let prs = PrListing {
            source_repo: Some("owner/repo".to_string()),
            pull_requests: vec![pr(104, "owner/repo", ALPHA, "main")],
            fetched_at: Some(0),
        };
        let (graph, _) = gather(&input("main", &["main", ALPHA, BETA], ForkOriginStore::default()), true, false);
        report.extend(assert_laid_out(
            "long-labels base",
            &graph.expect("a graph"),
            &prs,
            &Evidence {
                tags: vec![(BETA, &repo.d3), ("main", &repo.d4), ("origin/main", &repo.d5), ("PR #104 → main", &repo.a2)],
                merges: vec![],
                post_merge: vec![],
                incomplete: false,
                hidden_lanes: [0, 0],
            },
        ));
    }

    // Every connection is drawn: `fix/wt-ux` merges into `M103` from `W1`,
    // where `fix/sniff` forks. At 40 rows the height cap leaves out the least
    // active lane, `feat/schema-enhancement` (its tip is the oldest; see
    // `commit_on`).
    let repo = observed_sparse_lanes();
    {
        let _guard = DirGuard::enter(&repo.path);
        let (graph, _) = gather(&input("main", &repo.branches(), repo.forks.clone()), true, false);
        report.extend(assert_laid_out(
            "sparse-lanes base",
            &graph.expect("a graph"),
            &PrListing::default(),
            &Evidence {
                tags: vec![("main", &repo.m104), ("origin/main", &repo.m104)],
                merges: vec![(&repo.m103, &repo.d[12], repo.w1()), (&repo.m104, &repo.d[13], repo.sniff.last().expect("sniff commits"))],
                post_merge: vec![(&repo.wt_ux[SPARSE_WT_UX_BEFORE_MERGE], repo.w1())],
                incomplete: false,
                hidden_lanes: [1, 0],
            },
        ));
    }

    // PR #105's shape: `B` merged into `C`, then `N`; `fix/sniff-pr` is a
    // label at `B`. With `main` behind, `C` is on the default lane; diverged,
    // on the `origin/main` line.
    for local_main in [LocalMain::AtMerge, LocalMain::Behind, LocalMain::Diverged] {
        let repo = continued_after_merge(local_main);
        let _guard = DirGuard::enter(&repo.path);
        let main_tip = match local_main {
            LocalMain::AtMerge => &repo.c,
            LocalMain::Behind => &repo.p,
            LocalMain::Diverged => repo.p_prime.as_ref().expect("P'"),
        };
        let views: &[(&str, &str)] = match local_main {
            LocalMain::AtMerge => &[("base", "main"), ("focused wt-ux", "fix/wt-ux"), ("focused sniff-pr", "fix/sniff-pr")],
            _ => &[("base", "main")],
        };
        for (view, current) in views {
            let (graph, _) = gather(&input(current, &repo.branches(), repo.forks.clone()), true, false);
            // Diverged, `origin/main` is a lane of its own, named rather than tagged.
            let mut tags = vec![("main", main_tip.as_str())];
            if local_main != LocalMain::Diverged {
                tags.push(("origin/main", repo.c.as_str()));
            }
            if *current != "fix/wt-ux" {
                tags.push(("fix/sniff-pr", repo.b()));
            }
            report.extend(assert_laid_out(
                &format!("continued-{local_main:?} {view}"),
                &graph.expect("a graph"),
                &PrListing::default(),
                &Evidence {
                    tags,
                    merges: vec![(&repo.c, &repo.p, repo.b())],
                    post_merge: vec![(&repo.n, repo.b())],
                    incomplete: false,
                    hidden_lanes: [0, 0],
                },
            ));
        }
    }

    let repo = merged_twice_and_continued();
    {
        let _guard = DirGuard::enter(&repo.path);
        let (graph, _) = gather(&input("main", &["main", "b"], ForkOriginStore::default()), true, false);
        report.extend(assert_laid_out(
            "merged-twice base",
            &graph.expect("a graph"),
            &PrListing::default(),
            &Evidence {
                tags: vec![("main", &repo.c2)],
                merges: vec![(&repo.c1, &repo.p1, &repo.b1), (&repo.c2, &repo.p2, &repo.b2)],
                post_merge: vec![(&repo.b2, &repo.b1), (&repo.n, &repo.b2)],
                incomplete: false,
                hidden_lanes: [0, 0],
            },
        ));
    }

    // With its record, `new` does not claim `C`: its fork `b1` is undrawn, so
    // the plan has the notice. Without it, the topology gives the merge.
    let repo = new_branch_at_merged_tip();
    {
        let _guard = DirGuard::enter(&repo.path);
        let (graph, _) = gather(&input("main", &["main", "new"], repo.forks.clone()), true, false);
        report.extend(assert_laid_out(
            "new-at-merged-tip recorded",
            &graph.expect("a graph"),
            &PrListing::default(),
            &Evidence {
                tags: vec![("main", &repo.c)],
                merges: vec![],
                post_merge: vec![],
                incomplete: true,
                hidden_lanes: [0, 0],
            },
        ));
        let (graph, _) = gather(&input("main", &["main", "new"], ForkOriginStore::default()), true, false);
        report.extend(assert_laid_out(
            "new-at-merged-tip unrecorded",
            &graph.expect("a graph"),
            &PrListing::default(),
            &Evidence {
                tags: vec![("main", &repo.c)],
                merges: vec![(&repo.c, &repo.p, &repo.b1)],
                post_merge: vec![(&repo.n1, &repo.b1)],
                incomplete: false,
                hidden_lanes: [0, 0],
            },
        ));
    }

    // Recorded in the implementation log.
    eprintln!("{}", report.join("\n"));
}

/// The laid-out commits on `lane` after its last `+N` square.
fn after_square<'g>(geometry: &'g GitGraphGeometry, lane: &str) -> Vec<&'g CommitGeometry> {
    let on_lane: Vec<&CommitGeometry> = geometry.commits.iter().filter(|commit| commit.lane == lane).collect();
    let square = on_lane
        .iter()
        .rposition(|commit| commit.id.starts_with('+'))
        .unwrap_or_else(|| panic!("{lane} has no +N square: {on_lane:?}"));
    on_lane[square + 1..].to_vec()
}

/// The observed history at 200×60 with the real measurement: an isolated
/// `main`/`origin/main` stack no longer widens the step, so every long lane
/// keeps more than its tip after its last `+N` square; `fix/wt-ux` merges
/// into `main` at `M103` from `W1`, where `fix/sniff` forks, and `fix/sniff`
/// merges into `main` at `M104`.
#[test]
#[serial_test::serial]
fn the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60() {
    let repo = observed_sparse_lanes();
    let _guard = DirGuard::enter(&repo.path);
    let sniff_tip = repo.sniff.last().expect("sniff commits");

    let (graph, _) = gather(&input("main", &repo.branches(), repo.forks.clone()), true, false);
    let graph = graph.expect("base view");
    assert!(!graph.incomplete, "gathering verified every connection it reports");

    let plan = graph
        .to_git_graph(&PrListing::default(), None)
        .with_theme(MermaidTheme::Default)
        .plan(viewport(200, 60))
        .expect("a plan");
    let context = plan.mermaid.clone();
    let geometry = geometry_of(&plan.mermaid);
    assert_eq!(geometry.commit_step, biscuit_visualized::mermaid::default_gitgraph_commit_step(), "{context}");
    assert!(plan.columns <= 200, "{plan:?}");
    assert_eq!(plan.hidden_lanes, 0, "{context}");

    for lane in ["feat/schema-enhancement", "fix/sniff"] {
        let kept = after_square(&geometry, lane);
        assert!(kept.len() >= 2, "{lane} keeps more than its tip after its +N square: {kept:?}\n{context}");
    }
    // `fix/wt-ux` has an anchor, `W1`, between its squares. Width trimming
    // folds commits beside a square first, so its recent run folds into the
    // last square before `main`'s unfolded `d6..d12` run does.
    let wt_ux_tip = repo.wt_ux.last().expect("wt-ux commits");
    let kept = after_square(&geometry, "fix/wt-ux");
    assert!(wt_ux_tip.starts_with(kept.last().expect("the tip is kept").id.as_str()), "{kept:?}\n{context}");
    assert!(plan.mermaid.contains(&repo.w1()[..7]), "W1 is drawn: {context}");
    let sniff_kept = after_square(&geometry, "fix/sniff");
    assert!(sniff_tip.starts_with(sniff_kept.last().expect("kept").id.as_str()), "{context}");

    assert!(has_merge(&plan.mermaid, "fix/sniff", &repo.m104), "{context}");
    let merge = laid_out(&geometry, &repo.m104).expect("M104 drawn");
    assert_eq!(merge.lane, "main", "{context}");
    assert_eq!(merge.parents.len(), 2, "{merge:?}");
    assert!(sniff_tip.starts_with(merge.parents[1].as_str()), "{merge:?}");

    assert!(has_merge(&plan.mermaid, "fix/wt-ux", &repo.m103), "{context}");
    let earlier = laid_out(&geometry, &repo.m103).expect("M103 drawn");
    assert_eq!(earlier.lane, "main", "{context}");
    assert_eq!(earlier.parents.len(), 2, "{earlier:?}");
    assert!(repo.w1().starts_with(earlier.parents[1].as_str()), "M103's second parent is W1: {earlier:?}\n{context}");

    assert_eq!(geometry.tag_overlaps(), Vec::<(String, String)>::new(), "{context}");
    let mut tags: Vec<&str> = merge.tags.iter().map(|tag| tag.text.as_str()).collect();
    tags.sort_unstable();
    assert_eq!(tags, ["main", "origin/main"], "both refs label M104: {context}");

    // Every branch lane starts from a drawn commit, so there is no notice.
    assert!(!plan.incomplete, "{context}");
    for lane in ["feat/schema-enhancement", "fix/wt-ux", "fix/sniff"] {
        assert!(!first_on_lane(&geometry, lane).parents.is_empty(), "{lane} is connected: {context}");
    }
    let sniff_start = first_on_lane(&geometry, "fix/sniff");
    assert!(repo.w1().starts_with(sniff_start.parents[0].as_str()), "fix/sniff forks at W1: {sniff_start:?}\n{context}");
}
