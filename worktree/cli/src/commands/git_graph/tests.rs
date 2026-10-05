use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use biscuit_terminal::components::git_graph::{GitGraphPlan, GraphViewport, NaturalSize};
use biscuit_terminal::components::mermaid::MermaidTheme;
use biscuit_terminal::discovery::fonts::CellSize;
use biscuit_visualized::mermaid::{CommitGeometry, GitGraphGeometry, MermaidDiagram};
use worktree::fork_origin::{ForkOrigin, ForkOriginStore};
use worktree::git::recorder;
use worktree::graph::{BASE_DEFAULT_WINDOW, GatherInput, LINE_WINDOW, MergedElsewhere, gather};
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
fn the_base_view_gives_every_worktree_branch_a_line() {
    let repo = branches();
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
    let (graph, verbose) = gather(&input(&repo.path, "main", &branch_names, forks), true, true);
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
        to_git_graph(&graph, &PrListing::default(), None)
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
    //
    // A branch not merged directly into those lanes then asks one more
    // `--is-ancestor` per other drawn branch (not its parent) whose question
    // is not already answered, for a merge into that lane: feature-a and
    // feature-b ask about each other and feature-c; feature-c asks about
    // feature-a and chore/merged, whose tip is main's but was asked with a
    // different candidate list. Six.
    assert_eq!(count(&calls, "merge-base") - count_merge_bases(&calls), 5 + 3 + 6, "got {calls:?}");
    assert_eq!(count_merge_bases(&calls), 3, "one per branch with a lane, got {calls:?}");
    assert_eq!(count(&calls, "log"), 4, "the default lane and one per branch lane, got {calls:?}");
    // chore/merged's contained tip needs its first-parent chain, which is
    // empty for a fast-forward; no line reached its window. Each boundary is
    // on its candidate's first-parent chain, so it needs no `--ancestry-path`.
    // feature-b's tip is on feature-c's line (it forked there), so asking
    // whether feature-c merged it walks that chain once and finds no merge.
    assert_eq!(count(&calls, "rev-list"), 1 + 3 + 1, "got {calls:?}");
    assert!(!graph.incomplete);
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
    let (graph, _) = gather(&input(&repo.path, "feature-a", &["main", "feature-a"], ForkOriginStore::default()), true, false);
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

    let mermaid = to_git_graph(&facts, &prs, None).mermaid().expect("mermaid");
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
    let fitted = to_git_graph(&facts, &prs, None).plan_with(viewport, &measure).expect("plan");
    assert!(fitted.trimmed_commits > 0, "a scale-derived width trims to fit: {fitted:?}");
    let overridden = to_git_graph(&facts, &prs, Some(ImageWidth::Characters(50)))
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
    to_git_graph(graph, &PrListing::default(), None).mermaid().expect("mermaid")
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

    let (graph, _) = gather(&input(&repo.path, "fix/wt-ux", &["main", "fix/wt-ux"], ForkOriginStore::default()), true, false);
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

    let (graph, _) = gather(&input(&repo.path, "main", &["main", "fix/wt-ux"], ForkOriginStore::default()), true, false);
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
    let plan = to_git_graph(&graph, &PrListing::default(), None).plan_with(viewport, &measure).expect("plan");
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

    let (graph, _) = gather(&input(&repo.path, "main", &["main", "fix/wt-ux", "fix/sniff"], repo.forks.clone()), true, false);
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

/// [`commit_on`] and [`chain_on`] for fixtures with hundreds of commits: one
/// `git fast-import` writes them all instead of a `commit-tree` process per
/// commit. Every commit keeps its first parent's tree and is dated by
/// [`next_commit_time`]; `get-mark` returns each SHA as it is made. Objects
/// are readable by other Git commands only after [`BulkCommits::finish`].
struct BulkCommits {
    path: PathBuf,
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: std::io::BufReader<std::process::ChildStdout>,
    marks: std::collections::HashMap<String, usize>,
}

impl BulkCommits {
    fn new(path: &Path) -> Self {
        let mut child = Command::new("git")
            .current_dir(path)
            .args(["fast-import", "--quiet", "--done"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("git should be installed");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = std::io::BufReader::new(child.stdout.take().expect("stdout"));
        Self { path: path.to_path_buf(), child, stdin, stdout, marks: Default::default() }
    }

    fn commit(&mut self, parents: &[&str], message: &str) -> String {
        use std::io::{BufRead, Write};
        let mark = self.marks.len() + 1;
        // A commit made in this session is named by its mark.
        let name = |sha: &str| self.marks.get(sha).map_or_else(|| sha.to_string(), |mark| format!(":{mark}"));
        let mut command = format!(
            "commit refs/fixture/bulk\nmark :{mark}\ncommitter Test User <test@example.com> {} +0000\ndata {}\n{message}\n",
            next_commit_time(),
            message.len()
        );
        for (index, parent) in parents.iter().enumerate() {
            command.push_str(&format!("{} {}\n", if index == 0 { "from" } else { "merge" }, name(parent)));
        }
        command.push_str(&format!("\nget-mark :{mark}\n"));
        self.stdin.write_all(command.as_bytes()).expect("write to git fast-import");
        self.stdin.flush().expect("flush git fast-import");
        let mut sha = String::new();
        self.stdout.read_line(&mut sha).expect("read a mark from git fast-import");
        let sha = sha.trim().to_string();
        assert_eq!(sha.len(), 40, "git fast-import gave {sha:?} for {message}");
        self.marks.insert(sha.clone(), mark);
        sha
    }

    fn chain(&mut self, from: &str, prefix: &str, count: usize) -> Vec<String> {
        let mut chain: Vec<String> = Vec::with_capacity(count);
        for n in 1..=count {
            let parent = chain.last().map_or(from, String::as_str).to_string();
            chain.push(self.commit(&[&parent], &format!("{prefix}{n}")));
        }
        chain
    }

    /// Ends the session, writing its objects, and drops its scratch ref.
    fn finish(mut self) {
        use std::io::Write;
        self.stdin.write_all(b"done\n").expect("finish git fast-import");
        drop(self.stdin);
        assert!(self.child.wait().expect("git fast-import").success(), "git fast-import failed in {:?}", self.path);
        run_git(&self.path, &["update-ref", "-d", "refs/fixture/bulk"]);
    }
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

    // Over 200 commits: one `git fast-import`, not a process per commit.
    let mut bulk = BulkCommits::new(&path);
    let mut d = vec![r.clone()];
    d.extend(bulk.chain(&r, "d", 12));
    let schema = bulk.chain(&d[2], "s", SPARSE_SCHEMA_COMMITS);
    let mut wt_ux = bulk.chain(&d[5], "w", SPARSE_WT_UX_BEFORE_MERGE);
    let w1 = wt_ux.last().unwrap().clone();
    let m103 = bulk.commit(&[&d[12], &w1], "Merge pull request #103 from fix/wt-ux");
    let sniff = bulk.chain(&w1, "n", SPARSE_SNIFF_COMMITS);
    let continued = bulk.chain(&w1, "w", SPARSE_WT_UX_AFTER_MERGE);
    wt_ux.extend(continued);
    d.push(bulk.commit(&[&m103], "d13"));
    let m104 = bulk.commit(&[&d[13], sniff.last().unwrap()], "Merge pull request #104 from fix/sniff");
    let b1 = bulk.commit(&[wt_ux.last().unwrap(), &m104], "Merge branch 'main' into fix/wt-ux");
    let synced = bulk.chain(&b1, "x", SPARSE_WT_UX_AFTER_SYNC);
    wt_ux.extend(synced);
    bulk.finish();

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
    to_git_graph(graph, &PrListing::default(), None)
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
    let sniff_tip = repo.sniff.last().expect("sniff commits");

    let (graph, _) = gather(&input(&repo.path, "main", &repo.branches(), repo.forks.clone()), true, false);
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
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "parent", "main", 10);
    forked(&mut forks, "child", "parent", 20);

    let (graph, _) = gather(&input(&path, "main", &["main", "parent", "child"], forks), true, false);
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
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "parent", "main", 10);
    forked(&mut forks, "child", "parent", 20);

    let (graph, _) = gather(&input(&path, "main", &["main", "parent", "child"], forks), true, false);
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
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "parent", "main", 10);
    forked(&mut forks, "child", "parent", 20);

    let (graph, _) = gather(&input(&path, "child", &["main", "child"], forks), true, false);
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

    let (graph, _) = gather(&input(&path, "ff", &["main", "ff"], ForkOriginStore::default()), true, false);
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
    let mut forks = ForkOriginStore::default();
    forked(&mut forks, "d", "c", 20);

    let (graph, _) = gather(&input(&path, "main", &["main", "c", "d"], forks), true, false);
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

    let (graph, _) = gather(&input(&path, "b", &["main", "b"], ForkOriginStore::default()), true, false);
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
fn an_indirectly_integrated_branch_gets_a_lane_without_a_merge_and_no_notice() {
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

    let (graph, _) = gather(&input(&path, "t", &["main", "t"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("focused view");

    let t = line(&graph, "t");
    assert_eq!(t.entries, commits(&[&t1]));
    assert_eq!(t.fork_sha.as_ref(), Some(&d1));
    assert_eq!(merge_destinations(t), Vec::<&String>::new());
    assert!(!graph.incomplete, "a verified indirect integration is complete history");
    assert_eq!(
        graph.merged_elsewhere,
        vec![MergedElsewhere { branch: "t".into(), tip: t1.clone(), into: "main".into(), through: None }],
        "other is not drawn, so it isn't named"
    );
    assert!(mermaid(&graph).contains(r#"tag: "in main""#), "t's tip is tagged as merged: {}", mermaid(&graph));
    assert_eq!(graph.default_entries.last(), Some(&LaneEntry::Commit(merge)));
    assert!(!mermaid(&graph).contains("merge "));
}

/// `a`'s tip was merged directly into its sibling `b` (both forked from
/// main): `a` keeps its fork on main and gains the merge into `b`'s lane,
/// though `b` is neither its parent nor the default lane.
#[test]
#[serial_test::serial]
fn a_tip_merged_into_another_drawn_lane_is_drawn_merging_there() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    init_repo(&path);
    let _r = commit(&path, "r");
    let d1 = commit(&path, "d1");
    run_git(&path, &["checkout", "-q", "-b", "a"]);
    let a1 = commit(&path, "a1");
    run_git(&path, &["checkout", "-q", "-b", "b", &d1]);
    commit(&path, "b1");
    let merge = merge_no_ff(&path, "a");
    commit(&path, "b2");
    run_git(&path, &["checkout", "-q", "main"]);

    let (graph, _) = gather(&input(&path, "main", &["main", "a", "b"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");

    let a = line(&graph, "a");
    assert_eq!(a.fork_sha.as_ref(), Some(&d1), "the fork stays on main");
    assert_eq!(a.entries, commits(&[&a1]));
    assert_eq!(merge_destinations(a), vec![&merge]);
    assert!(line(&graph, "b").entries.contains(&LaneEntry::Commit(merge.clone())), "b draws its merge commit");
    assert!(has_merge(&mermaid(&graph), "a", &merge), "{}", mermaid(&graph));
    assert!(!graph.incomplete);
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

// ---------------------------------------------------------------------------
// A Git read that fails after an edge was accepted keeps every verified fact.
// ---------------------------------------------------------------------------

/// Whether `args` is the `log --first-parent` that reads the newest commits
/// of `tip`'s lane before `stop` (none for the default lane).
fn is_lane_read(args: &[&str], tip: &str, stop: &[&str]) -> bool {
    let mut rest: Vec<&str> = Vec::new();
    if !stop.is_empty() {
        rest.push("--not");
        rest.extend_from_slice(stop);
    }
    rest.push("--");
    args.len() == 6 + rest.len() && args[..2] == ["log", "--first-parent"] && args[5] == tip && args[6..] == rest[..]
}

/// Gathers the base view while `fails` makes matching Git calls fail, and
/// asserts that at least one call was failed.
fn gather_failing(repo: &Path, branches: &[&str], forks: ForkOriginStore, fails: impl Fn(&[&str]) -> bool + Send + Sync + Clone + 'static) -> GraphFacts {
    recorder::start_recording();
    let (graph, _) = {
        let _failing = recorder::fail_matching(fails.clone());
        gather(&input(repo, "main", branches, forks), true, false)
    };
    let calls = recorder::finish_recording();
    assert!(
        recorder::has_any_matching(&calls, |args| fails(&args.iter().map(String::as_str).collect::<Vec<_>>())),
        "the injected failure never fired: {calls:?}"
    );
    graph.expect("a failed read keeps the graph")
}

/// `fix/wt-ux` still merges `B` into `C`, with both commits drawn, and the
/// graph and its plan carry the notice.
fn assert_edge_kept(graph: &GraphFacts, repo: &ContinuedAfterMerge) {
    let wt_ux = line(graph, "fix/wt-ux");
    assert_eq!(wt_ux.merges, [LaneMerge { source: repo.b().clone(), destination: repo.c.clone() }], "{graph:?}");
    assert!(wt_ux.entries.contains(&LaneEntry::Commit(repo.b().clone())), "the source B is drawn: {:?}", wt_ux.entries);
    assert!(graph.default_entries.contains(&LaneEntry::Commit(repo.c.clone())), "the destination C is drawn: {:?}", graph.default_entries);
    assert!(graph.incomplete, "{graph:?}");
    assert_no_repeated_commit(graph);
    let plan = untrimmed_plan(graph, 120, 40);
    assert!(has_merge(&plan.mermaid, "fix/wt-ux", &repo.c), "{}", plan.mermaid);
    assert!(plan.incomplete, "{}", plan.mermaid);
}

/// The reread past an accepted merge fails: the lane keeps the window read
/// before it, ending at the source `B`, and the edge.
#[test]
#[serial_test::serial]
fn a_failed_reread_after_an_accepted_merge_keeps_the_edge_and_its_source() {
    let repo = continued_after_merge(LocalMain::AtMerge);
    let (n, p) = (repo.n.clone(), repo.p.clone());

    let graph = gather_failing(&repo.path, &repo.branches(), repo.forks.clone(), move |args| is_lane_read(args, &n, &[&p]));

    assert_edge_kept(&graph, &repo);
    let wt_ux = line(&graph, "fix/wt-ux");
    assert_eq!(wt_ux.entries, commits(&[repo.b(), &repo.n]), "the last verified window, with B below it");
    assert_eq!(wt_ux.fork_sha.as_ref(), Some(&repo.p), "merge-base(C^1, B)");
    let sniff = line(&graph, "fix/sniff-pr");
    assert_eq!(sniff.tip_sha.as_ref(), Some(repo.b()), "the child keeps its label at B");
}

/// Placing an anchor on a branch lane fails after its merge was accepted:
/// only the child's fork there is omitted; the lane, its source, and the edge
/// stay.
#[test]
#[serial_test::serial]
fn a_failed_anchor_lookup_on_a_merged_lane_omits_only_that_connection() {
    let repo = continued_after_merge(LocalMain::AtMerge);
    let tree = git_output(&repo.path, &["rev-parse", &format!("{}^{{tree}}", repo.n)]);
    let k = commit_on(&repo.path, &tree, &[&repo.x[1]], "K");
    set_branches(&repo.path, &[("child", &k)]);
    let mut forks = repo.forks.clone();
    forked_at(&mut forks, "child", "fix/wt-ux", &repo.x[1], 40);
    let branches = ["main", "fix/wt-ux", "fix/sniff-pr", "child"];

    // Without the failure, the child's fork `x2` splits the lane's `+7`.
    let (graph, _) = gather(&input(&repo.path, "main", &branches, forks.clone()), true, false);
    let graph = graph.expect("base view");
    assert!(line(&graph, "fix/wt-ux").entries.contains(&LaneEntry::Commit(repo.x[1].clone())), "{graph:?}");
    assert!(!graph.incomplete, "{graph:?}");

    let range = format!("{}..{}", repo.x[1], repo.n);
    let graph = gather_failing(&repo.path, &branches, forks, move |args| args == ["rev-list", "--first-parent", "--count", range.as_str(), "--"]);

    assert_edge_kept(&graph, &repo);
    let wt_ux = line(&graph, "fix/wt-ux");
    assert_eq!(wt_ux.entries, continued_lane(&repo), "verified entries are kept");
    let child = line(&graph, "child");
    assert_eq!(child.entries, commits(&[&k]));
    assert_eq!(child.fork_sha.as_ref(), Some(&repo.x[1]), "the fork is verified, only its position is not");
}

/// The default lane's newest commits cannot be read: the lane still draws
/// every anchor at its verified position, so the graph keeps the merge.
#[test]
#[serial_test::serial]
fn a_failed_default_lane_read_keeps_the_graph_and_its_verified_anchors() {
    let repo = continued_after_merge(LocalMain::AtMerge);
    let c = repo.c.clone();

    let graph = gather_failing(&repo.path, &repo.branches(), repo.forks.clone(), move |args| is_lane_read(args, &c, &[]));

    assert_edge_kept(&graph, &repo);
    assert_eq!(graph.default_entries, commits(&[&repo.d[4], &repo.p, &repo.c]), "the tip, the fork, and the fork's first parent");
    let wt_ux = line(&graph, "fix/wt-ux");
    assert_eq!(wt_ux.entries, continued_lane(&repo));
    assert_eq!(wt_ux.fork_sha.as_ref(), Some(&repo.p));
}

/// The diverged `origin/main` line's window cannot be read. That lane has no
/// verified commit, so its merge `C` is not substituted anywhere: the edge is
/// not drawn and the notice accounts for it, while `fix/wt-ux` keeps its lane.
#[test]
#[serial_test::serial]
fn a_failed_origin_line_read_draws_no_substitute_merge() {
    let repo = continued_after_merge(LocalMain::Diverged);
    let (c, p_prime) = (repo.c.clone(), repo.p_prime.clone().expect("P'"));

    let graph = gather_failing(&repo.path, &repo.branches(), repo.forks.clone(), move |args| is_lane_read(args, &c, &[&p_prime]));

    let wt_ux = line(&graph, "fix/wt-ux");
    assert_eq!(wt_ux.entries, continued_lane(&repo));
    assert_eq!(wt_ux.merges, [LaneMerge { source: repo.b().clone(), destination: repo.c.clone() }]);
    assert!(line(&graph, "origin/main").entries.is_empty(), "{graph:?}");
    assert!(!graph.default_entries.contains(&LaneEntry::Commit(repo.c.clone())), "C is not substituted onto main");
    assert!(graph.incomplete);
    assert_no_repeated_commit(&graph);
    let plan = untrimmed_plan(&graph, 120, 40);
    assert!(!has_merge(&plan.mermaid, "fix/wt-ux", &repo.c), "{}", plan.mermaid);
    assert!(plan.incomplete);
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
    let fork = &repo.a[10];
    let context = &repo.a[9];

    let (graph, _) = gather(&input(&repo.path, "side", &["main", "side"], ForkOriginStore::default()), true, false);
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

    let (graph, _) = gather(&input(&repo.path, "main", &["main", "side"], ForkOriginStore::default()), true, false);
    let graph = graph.expect("base view");
    let mut expected = vec![LaneEntry::Commit(context.clone()), LaneEntry::Commit(fork.clone()), LaneEntry::Elided(3), LaneEntry::Commit(repo.merge.clone()), LaneEntry::Elided(10)];
    expected.extend(commits(&repo.after[10..].iter().collect::<Vec<_>>()));
    assert_eq!(graph.default_entries, expected);
    assert_eq!(merge_destinations(line(&graph, "side")), [&repo.merge]);
    assert!(!graph.incomplete);
    assert!(has_merge(&mermaid(&graph), "side", &repo.merge));
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

    let (graph, _) = gather(&input(&path, "main", &["main", "old", "merged", "newest", "middle"], ForkOriginStore::default()), true, false);
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
    let plan = to_git_graph(&graph, &PrListing::default(), None).plan_with(viewport, &measure).expect("plan");
    assert_eq!(plan.hidden_lanes, 2, "{}", plan.mermaid);
    assert!(plan.mermaid.contains("branch newest"), "{}", plan.mermaid);
    assert!(plan.mermaid.contains("branch merged"), "the merged lane is the second most active: {}", plan.mermaid);
    assert!(has_merge(&plan.mermaid, "merged", &merge), "{}", plan.mermaid);
    assert!(!plan.mermaid.contains("branch middle") && !plan.mermaid.contains("branch old"), "{}", plan.mermaid);
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
    let graph = to_git_graph(facts, prs, None).with_theme(MermaidTheme::Default);
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
    let explicit = to_git_graph(facts, prs, Some(ImageWidth::Characters(40)))
        .with_theme(MermaidTheme::Default)
        .plan(viewport(120, 60))
        .expect("a plan");
    assert_eq!(explicit.hidden_lanes, 0, "{name}");
    assert_eq!(explicit.columns, 40, "{name}");
    assert_eq!(explicit.trimmed_commits, 0, "{name}: an explicit width is never trimmed to");
    assert_eq!(explicit.mermaid, graph.mermaid().expect("mermaid"), "{name}: the whole graph, scaled");
    report
}

// Gathered graphs lay out with exact merges and no overlapping tags. One
// test per fixture, so nextest builds the fixtures and runs them in parallel.

#[test]
#[serial_test::serial]
fn gathered_graph_lays_out_observation_1() {
    let mut report = Vec::new();
    let repo = merged_via_merge_commit();
    {
        let expected = Evidence {
            tags: vec![("main", &repo.d2), ("origin/main", &repo.merge)],
            merges: vec![(&repo.merge, &repo.d2, &repo.w2)],
            post_merge: vec![],
            incomplete: false,
            hidden_lanes: [0, 0],
        };
        for (view, current) in [("observation-1 focused", "fix/wt-ux"), ("observation-1 base", "main")] {
            let (graph, _) = gather(&input(&repo.path, current, &["main", "fix/wt-ux"], ForkOriginStore::default()), true, false);
            report.extend(assert_laid_out(view, &graph.expect("a graph"), &PrListing::default(), &expected));
        }
    }
    // Recorded in the implementation log.
    eprintln!("{}", report.join("\n"));
}

#[test]
#[serial_test::serial]
fn gathered_graph_lays_out_observation_2() {
    let mut report = Vec::new();
    let repo = nested_parent_merged_into_default();
    {
        let (graph, _) = gather(&input(&repo.path, "main", &["main", "fix/wt-ux", "fix/sniff"], repo.forks.clone()), true, false);
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
    // Recorded in the implementation log.
    eprintln!("{}", report.join("\n"));
}

#[test]
#[serial_test::serial]
fn gathered_graph_lays_out_long_labels() {
    let mut report = Vec::new();
    let repo = long_labels();
    {
        let prs = PrListing {
            source_repo: Some("owner/repo".to_string()),
            pull_requests: vec![pr(104, "owner/repo", ALPHA, "main")],
            fetched_at: Some(0),
        };
        let (graph, _) = gather(&input(&repo.path, "main", &["main", ALPHA, BETA], ForkOriginStore::default()), true, false);
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
    // Recorded in the implementation log.
    eprintln!("{}", report.join("\n"));
}

#[test]
#[serial_test::serial]
fn gathered_graph_lays_out_sparse_lanes() {
    let mut report = Vec::new();
    // Every connection is drawn: `fix/wt-ux` merges into `M103` from `W1`,
    // where `fix/sniff` forks. At 40 rows the height cap leaves out the least
    // active lane, `feat/schema-enhancement` (its tip is the oldest; see
    // `commit_on`).
    let repo = observed_sparse_lanes();
    {
        let (graph, _) = gather(&input(&repo.path, "main", &repo.branches(), repo.forks.clone()), true, false);
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
    // Recorded in the implementation log.
    eprintln!("{}", report.join("\n"));
}

#[test]
#[serial_test::serial]
fn gathered_graph_lays_out_continued_after_merge_at_merge() {
    assert_continued_after_merge_lays_out(LocalMain::AtMerge);
}

#[test]
#[serial_test::serial]
fn gathered_graph_lays_out_continued_after_merge_behind() {
    assert_continued_after_merge_lays_out(LocalMain::Behind);
}

#[test]
#[serial_test::serial]
fn gathered_graph_lays_out_continued_after_merge_diverged() {
    assert_continued_after_merge_lays_out(LocalMain::Diverged);
}

/// PR #105's shape: `B` merged into `C`, then `N`; `fix/sniff-pr` is a label
/// at `B`. With `main` behind, `C` is on the default lane; diverged, on the
/// `origin/main` line.
fn assert_continued_after_merge_lays_out(local_main: LocalMain) {
    let mut report = Vec::new();
    {
        let repo = continued_after_merge(local_main);
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
            let (graph, _) = gather(&input(&repo.path, current, &repo.branches(), repo.forks.clone()), true, false);
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
    // Recorded in the implementation log.
    eprintln!("{}", report.join("\n"));
}

#[test]
#[serial_test::serial]
fn gathered_graph_lays_out_merged_twice() {
    let mut report = Vec::new();
    let repo = merged_twice_and_continued();
    {
        let (graph, _) = gather(&input(&repo.path, "main", &["main", "b"], ForkOriginStore::default()), true, false);
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
    // Recorded in the implementation log.
    eprintln!("{}", report.join("\n"));
}

#[test]
#[serial_test::serial]
fn gathered_graph_lays_out_new_branch_at_merged_tip() {
    let mut report = Vec::new();
    // With its record, `new` does not claim `C`: its fork `b1` is undrawn, so
    // the plan has the notice. Without it, the topology gives the merge.
    let repo = new_branch_at_merged_tip();
    {
        let (graph, _) = gather(&input(&repo.path, "main", &["main", "new"], repo.forks.clone()), true, false);
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
        let (graph, _) = gather(&input(&repo.path, "main", &["main", "new"], ForkOriginStore::default()), true, false);
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
    let sniff_tip = repo.sniff.last().expect("sniff commits");

    let (graph, _) = gather(&input(&repo.path, "main", &repo.branches(), repo.forks.clone()), true, false);
    let graph = graph.expect("base view");
    assert!(!graph.incomplete, "gathering verified every connection it reports");

    let plan = to_git_graph(&graph, &PrListing::default(), None)
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

/// Commit messages, scopes, and ref names are Git data, so the verbose
/// section shows them literally rather than as Prose markup.
#[test]
fn verbose_commit_lines_show_messages_and_refs_literally() {
    use biscuit_terminal::components::renderable::TerminalRenderable as _;
    let render = |message: &str, refs: &str| {
        let commit = CommitDetail {
            short_sha: "abc1234".to_string(),
            message: message.to_string(),
            committed_at: Local::now().timestamp(),
            refs: refs.to_string(),
        };
        biscuit_test_harness::strip_ansi(&Prose::new(format_commit(&commit)).render_optimistic(Some(400)))
    };
    let conventional = render("fix(<red>s</red>): a_b_c <red>d</red>", "HEAD -> feat/<b>x, tag: v_1_, origin/<i>y");
    for literal in ["<red>s</red>", "a_b_c <red>d</red>", "feat/<b>x", "v_1_", "origin/<i>y"] {
        assert!(conventional.contains(literal), "{literal:?} in {conventional:?}");
    }
    let plain = render("<red>plain</red> *message*", "");
    assert!(plain.contains("<red>plain</red> *message*"), "{plain:?}");
}
