use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::fork_origin::{ForkOrigin, ForkOriginStore};
use crate::git::recorder;
use crate::listing::RefTips;

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

fn commits(shas: &[&String]) -> Vec<LaneEntry> {
    shas.iter().map(|sha| LaneEntry::Commit((*sha).clone())).collect()
}

/// The gather input for the repository at `repo`, whose Git calls all run
/// there.
fn input(repo: &Path, current: &str, branches: &[&str], forks: ForkOriginStore) -> GatherInput {
    GatherInput {
        repo: repo.to_path_buf(),
        default_branch: "main".to_string(),
        current_branch: Some(current.to_string()),
        branch_names: branches.iter().map(|b| b.to_string()).collect(),
        refs: RefTips::read_in(repo).expect("for-each-ref"),
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

    let (graph, verbose) = gather(&input(&repo.path, "feature-a", &["main", "feature-a", "feature-b"], ForkOriginStore::default()), true, false);
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

    let (graph, _) = gather(&input(&repo.path, "feat/dark", &["main", "feat/theme", "feat/dark"], forks), true, false);
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
fn lines_past_the_window_start_with_one_elision() {
    let repo = branches();
    run_git(&repo.path, &["checkout", "-q", "feature-a"]);
    let newer: Vec<String> = (2..=7).map(|n| commit(&repo.path, &format!("a{n}"))).collect();
    run_git(&repo.path, &["checkout", "-q", "main"]);

    let (graph, _) = gather(&input(&repo.path, "feature-a", &["main", "feature-a"], ForkOriginStore::default()), true, false);
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
    let origin = add_origin(&repo.path);
    let u1 = advance_origin(&repo.path, &origin, "u1");

    // PR-driven: origin/main is only ahead, so its commit sits on the default
    // lane and both tips are tags.
    let (graph, _) = gather(&input(&repo.path, "feature-a", &["main", "feature-a"], ForkOriginStore::default()), true, false);
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
    let (graph, _) = gather(&input(&repo.path, "feature-a", &["main", "feature-a"], ForkOriginStore::default()), true, false);
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
    let bases = git_output(&path, &["merge-base", "--all", "main", "feature"]);
    assert_eq!(bases.lines().count(), 2, "the fixture needs two best bases");

    let tip_unique: Vec<String> = git_output(&path, &["log", "--format=%H", "--reverse", "feature", "--not", "main", "--"])
        .lines()
        .map(str::to_string)
        .collect();
    let (graph, verbose) = gather(&input(&path, "feature", &["main", "feature"], ForkOriginStore::default()), true, true);

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

    recorder::start_recording();
    let (graph, verbose) = gather(&input(&repo.path, "feature-a", &["main", "feature-a"], ForkOriginStore::default()), true, true);
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

/// Verbose labels describe the snapshot the listing accepted, not Git's live
/// branch refs; tags, which the snapshot does not capture, stay live.
#[test]
#[serial_test::serial]
fn verbose_labels_follow_the_snapshot_and_keep_live_tags() {
    let repo = branches();
    run_git(&repo.path, &["config", "tag.gpgsign", "false"]);
    run_git(&repo.path, &["update-ref", "refs/remotes/origin/main", &repo.c2]);
    run_git(&repo.path, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
    run_git(&repo.path, &["tag", "v1", &repo.c2]);
    let snapshot = input(&repo.path, "feature-a", &["main", "feature-a"], ForkOriginStore::default());
    let labels = |verbose: &VerboseData| {
        (
            verbose.merge_base.as_ref().map(|commit| commit.refs.clone()),
            verbose.branch_commits.iter().map(|commit| commit.refs.clone()).collect::<Vec<_>>(),
        )
    };

    let (_, unmoved) = gather(&snapshot, false, true);
    let live = git_output(&repo.path, &["log", "-1", "--format=%D", &repo.c2]);
    assert_eq!(live, "tag: v1, origin/main, origin/HEAD", "Git's own spelling, in its order");
    assert_eq!(labels(&unmoved.expect("verbose")), (Some(live), vec!["HEAD -> feature-a".to_string()]));

    // A fetch and a new branch after the snapshot; a new tag is live.
    run_git(&repo.path, &["update-ref", "refs/remotes/origin/main", &repo.c3]);
    run_git(&repo.path, &["branch", "late", &repo.a1]);
    run_git(&repo.path, &["tag", "v2", &repo.a1]);
    let (_, moved) = gather(&snapshot, false, true);

    assert_eq!(
        labels(&moved.expect("verbose")),
        (Some("tag: v1, origin/main, origin/HEAD".to_string()), vec!["HEAD -> feature-a, tag: v2".to_string()]),
        "origin/main and origin/HEAD stay at the snapshot's tip; `late` is not in it"
    );
}

/// A gather reads history only through the snapshot's object IDs, so refs
/// that advance, appear, or disappear after it was captured change nothing
/// it returns, in the focused and the base view alike.
#[test]
#[serial_test::serial]
fn refs_moved_after_the_snapshot_leave_the_gather_unchanged() {
    let repo = branches();
    run_git(&repo.path, &["update-ref", "refs/remotes/origin/main", &repo.c2]);
    let names = ["main", "feature-a", "feature-b"];
    let focused = input(&repo.path, "feature-a", &names, ForkOriginStore::default());
    let base = input(&repo.path, "main", &names, ForkOriginStore::default());
    let verbose_shape = |verbose: Option<VerboseData>| {
        let verbose = verbose.expect("verbose");
        let detail = |commit: &CommitDetail| (commit.short_sha.clone(), commit.message.clone(), commit.refs.clone());
        (verbose.merge_base.as_ref().map(detail), verbose.branch_commits.iter().map(detail).collect::<Vec<_>>())
    };

    let (focused_graph, focused_verbose) = gather(&focused, true, true);
    let (base_graph, _) = gather(&base, true, false);
    let (focused_verbose, focused_graph, base_graph) =
        (verbose_shape(focused_verbose), focused_graph.expect("focused view"), base_graph.expect("base view"));

    // Every kind of move: both branches of the focused view advance,
    // origin/main is fetched forward, one branch is deleted, one created.
    run_git(&repo.path, &["checkout", "-q", "feature-a"]);
    commit(&repo.path, "a2");
    run_git(&repo.path, &["checkout", "-q", "main"]);
    let c4 = commit(&repo.path, "c4");
    run_git(&repo.path, &["update-ref", "refs/remotes/origin/main", &c4]);
    run_git(&repo.path, &["branch", "-q", "-D", "feature-b"]);
    run_git(&repo.path, &["branch", "late", &repo.a1]);

    recorder::start_recording();
    let (moved_focused_graph, moved_focused_verbose) = gather(&focused, true, true);
    let (moved_base_graph, _) = gather(&base, true, false);
    let calls = recorder::finish_recording();

    assert_eq!(moved_focused_graph.expect("focused view"), focused_graph);
    assert_eq!(verbose_shape(moved_focused_verbose), focused_verbose);
    assert_eq!(moved_base_graph.expect("base view"), base_graph);
    let ref_names = ["HEAD", "main", "feature-a", "feature-b", "late", "origin/main", "origin/HEAD"];
    let named: Vec<&Vec<String>> = calls
        .iter()
        .filter(|args| {
            args.iter()
                .flat_map(|arg| arg.split(".."))
                .any(|part| ref_names.contains(&part.trim_start_matches('.')))
        })
        .collect();
    assert!(!calls.is_empty() && named.is_empty(), "every revision is an object ID; named: {named:?}");
}

#[test]
#[serial_test::serial]
fn nothing_is_gathered_when_detached_or_not_needed() {
    let repo = branches();
    let mut detached = input(&repo.path, "feature-a", &["main"], ForkOriginStore::default());
    detached.current_branch = None;

    recorder::start_recording();
    let results = [
        gather(&detached, true, true),
        gather(&input(&repo.path, "feature-a", &["main"], ForkOriginStore::default()), false, false),
        // Verbose alone on the default branch has nothing to show.
        gather(&input(&repo.path, "main", &["main"], ForkOriginStore::default()), false, true),
    ];
    let calls = recorder::finish_recording();

    assert!(results.iter().all(|(graph, verbose)| graph.is_none() && verbose.is_none()));
    assert_eq!(count(&calls, "merge-base") + count(&calls, "log"), 0, "got {calls:?}");
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

/// A commit on `tree` with `parents` (first parent first), made without
/// touching the checkout; returns its full SHA.
///
/// Each commit is dated one second after the previous one, because the
/// height cap ranks lanes by their tip's commit time in whole seconds: with
/// wall-clock dates, a fast host gives several tips the same second and the
/// hidden lane changes from host to host.
fn commit_on(path: &Path, tree: &str, parents: &[&str], message: &str) -> String {
    let date = format!("{} +0000", next_commit_time());
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

/// The next fixture commit time: one second after the previous one (see
/// [`commit_on`]).
fn next_commit_time() -> i64 {
    static SEQUENCE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("clock").as_secs() as i64;
    now + SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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

#[test]
#[serial_test::serial]
fn a_recorded_parent_is_selected_with_or_without_its_own_worktree() {
    let repo = branches();
    run_git(&repo.path, &["checkout", "-q", "-b", "feat/child", "feature-a"]);
    let child = commit(&repo.path, "child");
    run_git(&repo.path, &["checkout", "-q", "main"]);
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "feat/child", "feature-a", 20);

    for worktrees in [&["main", "feat/child"][..], &["main", "feature-a", "feat/child"][..]] {
        let (graph, _) = gather(&input(&repo.path, "feat/child", worktrees, forks.clone()), true, false);
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
    let (graph, _) = gather(&input(&repo.path, "main", &["main", "feature-a", "feat/child"], forks.clone()), true, false);
    assert_eq!(line(graph.as_ref().unwrap(), "feat/child").parent.as_deref(), Some("feature-a"));
    let (graph, _) = gather(&input(&repo.path, "main", &["main", "feat/child"], forks), true, false);
    let graph = graph.unwrap();
    let child_line = line(&graph, "feat/child");
    assert_eq!(child_line.parent, None);
    assert_eq!(child_line.fork_sha.as_ref(), Some(&repo.c2));
    assert_eq!(child_line.entries, commits(&[&repo.a1, &child]));
}

/// `t` forked from `s1`, which reached main only through `side`'s merge `M`
/// of `s2` (so not as `M`'s parent): no drawn lane holds the fork, and the
/// facts name `M` as the reason.
#[test]
#[serial_test::serial]
fn a_fork_main_holds_only_through_a_merge_is_explained() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let _r = commit(&path, "r");
    let d1 = commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "side"]);
    let s1 = commit(&path, "s1");
    run_git(&path, &["checkout", "-q", "-b", "t"]);
    commit(&path, "t1");
    run_git(&path, &["checkout", "-q", "side"]);
    commit(&path, "s2");
    run_git(&path, &["checkout", "-q", "main"]);
    let merge = merge_no_ff(&path, "side");

    let (graph, _) = gather(&input(&path, "main", &["main", "t"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");

    assert_eq!(line(&graph, "t").fork_sha.as_ref(), Some(&s1));
    assert!(!graph.default_entries.contains(&LaneEntry::Commit(s1.clone())), "s1 is not on main's own line");
    assert!(graph.default_entries.contains(&LaneEntry::Commit(d1)));
    assert_eq!(
        graph.forked_off_line,
        vec![ForkedOffLine { branch: "t".into(), fork: s1, into: "main".into(), merge }]
    );
}

/// The focused view of `u`, whose parent `t` forked from `s1` on `side`'s
/// line: `side` (no record links it) is drawn too, so `t` hangs from `s1`
/// and nothing is reported as forked off a drawn line.
#[test]
#[serial_test::serial]
fn a_focused_view_draws_the_branch_that_holds_a_lanes_fork() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let _r = commit(&path, "r");
    commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "side"]);
    let s1 = commit(&path, "s1");
    run_git(&path, &["checkout", "-q", "-b", "t"]);
    let t1 = commit(&path, "t1");
    run_git(&path, &["checkout", "-q", "-b", "u"]);
    commit(&path, "u1");
    run_git(&path, &["checkout", "-q", "side"]);
    commit(&path, "s2");
    run_git(&path, &["checkout", "-q", "main"]);
    merge_no_ff(&path, "side");
    let mut forks = ForkOriginStore::default();
    forked_at(&mut forks, "u", "t", &t1, 2);

    let (graph, _) = gather(&input(&path, "u", &["main", "side", "t", "u"], forks), true, false);
    let graph = graph.expect("focused view");

    assert_eq!(line(&graph, "t").fork_sha.as_ref(), Some(&s1));
    assert!(line(&graph, "side").entries.contains(&LaneEntry::Commit(s1)), "side's lane holds the fork");
    assert!(graph.forked_off_line.is_empty(), "{:?}", graph.forked_off_line);
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

    let (graph, _) = gather(&input(&repo.path, "main", &repo.branches(), repo.forks.clone()), true, false);
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

    let (graph, _) = gather(&input(&repo.path, "fix/wt-ux", &repo.branches(), repo.forks.clone()), true, false);
    let graph = graph.expect("focused view from fix/wt-ux");
    assert_eq!(graph.lines.len(), 1, "only the current branch is selected: {:?}", graph.lines);
    assert_continued_lane(&graph, &repo);
    // The window stops just below the oldest connection, the fork `P`.
    assert_eq!(graph.default_entries, commits(&[&repo.d[4], &repo.p, &repo.c]));

    // From the child, its recorded parent is drawn with the reconstructed lane.
    let (graph, _) = gather(&input(&repo.path, "fix/sniff-pr", &repo.branches(), repo.forks.clone()), true, false);
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
    let (graph, _) = gather(&input(&repo.path, "main", &repo.branches(), repo.forks.clone()), true, false);
    let graph = graph.expect("base view, behind");
    assert_continued_lane(&graph, &repo);
    assert_eq!(graph.default_entries.last(), Some(&LaneEntry::Commit(repo.c.clone())), "C is on the default lane");
    assert!(graph.default_entries.contains(&LaneEntry::Commit(repo.p.clone())));
    assert!(graph.lines.iter().all(|line| line.branch != "origin/main"), "no line of its own when only behind");

    let repo = continued_after_merge(LocalMain::Diverged);
    let (graph, _) = gather(&input(&repo.path, "main", &repo.branches(), repo.forks.clone()), true, false);
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

    let (graph, _) = gather(&input(&repo.path, "main", &["main", "b"], ForkOriginStore::default()), true, false);
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

    let (graph, _) = gather(&input(&repo.path, "main", &["main", "t"], ForkOriginStore::default()), true, false);
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
    let (graph, _) = gather(&input(&crossing.clone, "main", &["main", "b"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");
    let b = line(&graph, "b");
    assert!(b.merges.is_empty(), "{b:?}");
    assert_eq!(b.entries, commits(&[&crossing.source.n]));
    assert!(graph.incomplete);

    // Depth 2: the later merge is verified and kept; the older boundary and
    // the fork are past the cutoff.
    let older = shallow_merged_twice(2);
    let repo = &older.source;
    let (graph, _) = gather(&input(&older.clone, "main", &["main", "b"], ForkOriginStore::default()), true, false);
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

    // The record says `new` was created at `b1`: `C` merged another branch.
    let (graph, _) = gather(&input(&repo.path, "main", &["main", "new"], repo.forks.clone()), true, false);
    let graph = graph.expect("base view");
    let new = line(&graph, "new");
    assert_eq!(new.entries, commits(&[&repo.n1]));
    assert_eq!(new.fork_sha.as_ref(), Some(&repo.b1), "the ordinary fork (the old merged tip), which no lane draws");
    assert!(new.merges.is_empty(), "{new:?}");
    assert!(!graph.incomplete, "GitGraph's notice accounts for the undrawn fork");

    // Without the record, the topology is all there is.
    let (graph, _) = gather(&input(&repo.path, "main", &["main", "new"], ForkOriginStore::default()), true, false);
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

fn new_line_facts(repo: &Path, forks: ForkOriginStore) -> (Cutoff, Vec<Vec<String>>) {
    recorder::start_recording();
    let (graph, _) = gather(&input(repo, "main", &["main", "new"], forks), true, false);
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
    for (cell, text, expected) in &cells {
        fs::write(&store_path, text).unwrap();
        let forks = ForkOriginStore::load_from(&store_path);
        let (facts, calls) = new_line_facts(&repo.path, forks);
        assert_eq!(&facts, *expected, "{cell}:\n{text}");
        for value in ["", &repo.b1[..7], &repo.b1.to_uppercase()] {
            assert!(!calls.iter().flatten().any(|arg| arg == value), "{cell}: git was given {value:?}: {calls:?}");
        }
    }

    // A shallow clone cannot tell an unknown record from one past its cut.
    let (_clone_dir, clone) = shallow_clone_of(&repo.path, 3, &["new"]);
    let shallow_cells: [(&str, String, Cutoff); 3] = [
        ("shallow control: no record", saved.replace("\"new\"", "\"deleted\""), (edge.0.clone(), edge.1.clone(), true)),
        ("shallow control: the record at B", saved.clone(), (Vec::new(), cutoff.1.clone(), true)),
        ("shallow: unknown object", quoted(&unknown), (Vec::new(), cutoff.1.clone(), true)),
    ];
    for (cell, text, expected) in &shallow_cells {
        fs::write(&store_path, text).unwrap();
        let (facts, _) = new_line_facts(&clone, ForkOriginStore::load_from(&store_path));
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
    let time = |sha: &str| git_output(&repo.path, &["log", "-1", "--format=%ct", sha]).parse::<i64>().unwrap();

    let (graph, _) = gather(&input(&repo.path, "main", &["main", "feature-a", "feature-b"], ForkOriginStore::default()), true, false);

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
        shallow: false,
        merged_elsewhere: Vec::new(),
        forked_off_line: Vec::new(),
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
    merge_no_ff(&path, "side");
    let after: Vec<String> = (1..=20).map(|n| empty_commit(&path, &format!("c{n}"))).collect();
    OldConnections { _dir: dir, path, a, side, after }
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

    let (graph, _) = gather(&input(&clone, "feature", &["main", "feature"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("a shallow clone still has a graph");
    assert!(graph.incomplete, "a shallow \"no\" is never trusted");
    let feature_line = line(&graph, "feature");
    assert_eq!(feature_line.entries, commits(&[&feature[0], &feature[1]]), "the verified commits remain");
    assert_eq!(feature_line.fork_sha.as_ref(), Some(&main[3]), "a positive merge base is trusted");
    assert_eq!(merge_destinations(feature_line), Vec::<&String>::new());
    assert_eq!(graph.default_entries, commits(&[&main[2], &main[3]]));

    let (graph, _) = gather(&input(&clone, "main", &["main", "feature"], ForkOriginStore::default()), true, false);
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
    let (history, read) = topology::History::read(&clone);
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
            merge: outer.clone(),
            first_parent: o1.clone(),
        }),
        "a gap after an indirect match keeps that match"
    );

    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "feature", "parent", 20);
    let (graph, _) = gather(&input(&clone, "main", &["main", "parent", "feature"], forks), true, false);
    let graph = graph.expect("a shallow clone still has a graph");
    assert!(graph.incomplete);
    let feature = line(&graph, "feature");
    assert_eq!(feature.parent.as_deref(), Some("parent"));
    assert_eq!(merge_destinations(feature), Vec::<&String>::new(), "no merge edge is invented");
    assert!(graph.lines.iter().all(|line| line.merges.is_empty()), "no line draws a merge: {graph:?}");
}

#[test]
#[serial_test::serial]
fn a_deleted_recorded_parent_falls_back_to_the_default_branch() {
    let repo = branches();
    run_git(&repo.path, &["checkout", "-q", "-b", "gone", "main"]);
    let g1 = commit(&repo.path, "g1");
    run_git(&repo.path, &["checkout", "-q", "-b", "orphan"]);
    let o1 = commit(&repo.path, "o1");
    run_git(&repo.path, &["checkout", "-q", "main"]);
    run_git(&repo.path, &["branch", "-q", "-D", "gone"]);
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "orphan", "gone", 20);

    let (graph, _) = gather(&input(&repo.path, "orphan", &["main", "orphan"], forks), true, false);
    let graph = graph.expect("focused view");

    let names: Vec<&str> = graph.lines.iter().map(|line| line.branch.as_str()).collect();
    assert_eq!(names, ["orphan"]);
    let orphan = line(&graph, "orphan");
    assert_eq!(orphan.parent, None);
    assert_eq!(orphan.fork_sha.as_ref(), Some(&repo.c3));
    assert_eq!(orphan.entries, commits(&[&g1, &o1]));
    assert!(!graph.incomplete);
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
    let (history, read) = topology::History::read(&path);
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
            merge: synced.clone(),
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
            merge: tip.clone(),
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
    let history = topology::History::complete(&repo.path);
    let main_tip = repo.after.last().unwrap();
    let side_tip = repo.side.last().unwrap();

    // `side`'s commits are the merge's second parent: not on main's chain.
    let built = history
        .first_parent_entries(main_tip, topology::Extent::Open { window: 3, cap_window: false }, &[&repo.side[1], &repo.a[5]]);
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
        .first_parent_entries(side_tip, topology::Extent::Until(&window), &[&repo.side[2], &repo.a[9]]);
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

    // An unreadable tip is a gap with nothing to draw, never an error.
    let unreadable = history.first_parent_entries(&"0".repeat(40), topology::Extent::Open { window: 3, cap_window: false }, &[]);
    assert!(unreadable.entries.is_empty() && unreadable.gap, "{unreadable:?}");
}

/// Every thread a graph gather spawns (one per selected branch and per lane
/// in `parallel`, one per anchor distance in `History::locate`) hands its
/// count back, so a scope around the gather counts exactly the calls the
/// recorder logs; none of them fails to start.
#[test]
#[serial_test::serial]
fn a_counting_scope_sees_every_call_of_a_threaded_graph_gather() {
    let repo = old_connections();
    run_git(&repo.path, &["checkout", "-q", "-b", "feature", &repo.after[5]]);
    commit(&repo.path, "f1");
    run_git(&repo.path, &["checkout", "-q", "main"]);
    let input = input(&repo.path, "main", &["main", "side", "feature"], ForkOriginStore::default());

    recorder::start_recording();
    let scope = calls::CallScope::enter();
    let (graph, _) = gather(&input, true, false);
    let counted = scope.finish();
    let recorded = recorder::finish_recording();

    let graph = graph.expect("base view");
    assert_eq!(graph.lines.len(), 2, "two branches, so `parallel` spawns per branch: {graph:?}");
    let is_distance = |args: &[String]| args.len() == 5 && args[..3] == ["rev-list", "--first-parent", "--count"] && args[3].contains("..");
    assert!(
        recorder::has_any_matching(&recorded, is_distance),
        "an anchor below the default window is located on its own thread: {recorded:?}"
    );
    assert_eq!(counted, recorded.len() as u64, "{recorded:?}");
}
