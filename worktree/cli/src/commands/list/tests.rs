
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use worktree::git::recorder;
use worktree::worktree::{list_worktrees, parse_worktree_state};

use crate::commands::git_graph;
use crate::perf::PerfCollector;

mod pipeline;

/// Each top-level `--perf` row with its children's labels.
fn perf_shape(collector: &PerfCollector) -> Vec<(String, Vec<String>)> {
    let tree = collector.build_perf_tree();
    let labels = |row: &crate::perf::PerfNode| row.children.iter().map(|child| child.label.clone()).collect();
    tree.children.iter().map(|row| (row.label.clone(), labels(row))).collect()
}

/// The children of the one top-level row labeled `group`.
fn perf_group(collector: &PerfCollector, group: &str) -> Vec<String> {
    let shape = perf_shape(collector);
    let mut rows = shape.iter().filter(|(label, _)| label == group);
    let (_, children) = rows.next().unwrap_or_else(|| panic!("no `{group}` row: {shape:?}"));
    assert!(rows.next().is_none(), "one `{group}` row: {shape:?}");
    children.clone()
}

/// Top-level rows plus `unattributed` equal the total exactly, and no child
/// outlasts the group it was measured inside.
fn assert_perf_reconciles(collector: &PerfCollector) {
    let tree = collector.build_perf_tree();
    let top_level: std::time::Duration = tree.children.iter().map(|row| row.total).sum();
    assert_eq!(top_level, tree.total, "{tree:#?}");
    assert!(tree.children.iter().all(|row| row.label != crate::perf::OVER_ATTRIBUTED), "{tree:#?}");
    for row in &tree.children {
        assert!(row.children.iter().all(|child| child.total <= row.total), "{tree:#?}");
    }
}

/// These repositories have no origin, so nothing is launched.
fn no_launch(main: &Path, _: &super::LaunchArgs) -> std::io::Result<super::WorkerHandle> {
    panic!("no worker expected without an origin, got one for {}", main.display())
}

const NO_PRS: super::ListSeams = super::ListSeams {
    launch: no_launch,
    wait_budget: super::ORDINARY_BUDGET,
    forced_budget: super::FORCED_BUDGET,
};

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
    let result = super::run(None, false, false, super::ListFlags::default(), std::time::Instant::now());
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
        super::ListFlags::default(),
        std::time::Instant::now(),
        ImageSupport::None,
        &terminal,
        NO_PRS,
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
        super::ListFlags::default(),
        std::time::Instant::now(),
        ImageSupport::None,
        &terminal,
        NO_PRS,
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
        !names.contains(&"remote wait"),
        "without an origin there is no remote wait row, got: {names:?}"
    );
    let collector = collector.as_ref().expect("collector");
    assert_eq!(perf_group(collector, "local gather"), ["list gather", "verbose gather"]);
    assert_perf_reconciles(collector);
    assert!(
        !names.contains(&"graph image render (biscuit-terminal)"),
        "graph image render should not be recorded, got: {names:?}"
    );
}

/// Test seam that proves what overlaps what in `gather_listing`.
///
/// The pipeline reports each local gather's start (`arrive`) and end
/// (`finished`), and the end of the remote wait (`remote_finished`). Without
/// an installed seam these do nothing, so every other test runs the pipeline
/// unchanged. What `arrive` does depends on the [`Mode`]; every wait is
/// bounded, so a pipeline that does not overlap fails instead of hanging.
pub(super) mod overlap {
    use std::sync::{Arc, Condvar, Mutex, MutexGuard};
    use std::time::Duration;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(in crate::commands::list) enum Gather {
        List,
        Graph,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(in crate::commands::list) enum Mode {
        /// Each local gather waits for the other to start: both start only
        /// when neither has to finish first.
        Rendezvous,
        /// Only records; a stub worker waits on the events.
        Observe,
        /// The list gather starts only once the remote wait has returned, so
        /// it certainly outlasts the wait.
        HoldListUntilRemote,
    }

    #[derive(Default)]
    struct Events {
        list: bool,
        graph: bool,
        list_saw_graph: bool,
        graph_saw_list: bool,
        list_done: bool,
        graph_done: bool,
        remote_done: bool,
        list_saw_remote_done: bool,
    }

    struct Seam {
        mode: Mode,
        events: Mutex<Events>,
        changed: Condvar,
    }

    impl Seam {
        fn lock(&self) -> MutexGuard<'_, Events> {
            self.events.lock().unwrap_or_else(|e| e.into_inner())
        }

        /// Records with `record`, then waits (bounded) while `pending` holds.
        fn update_and_wait(&self, record: impl FnOnce(&mut Events), pending: impl Fn(&Events) -> bool) -> MutexGuard<'_, Events> {
            let mut events = self.lock();
            record(&mut events);
            self.changed.notify_all();
            self.changed
                .wait_timeout_while(events, WAIT, |events| pending(events))
                .unwrap_or_else(|e| e.into_inner())
                .0
        }
    }

    /// Only a failing (non-overlapping) pipeline waits this long; an
    /// overlapping one is released as soon as the awaited event happens.
    const WAIT: Duration = Duration::from_secs(10);

    static INSTALLED: Mutex<Option<Arc<Seam>>> = Mutex::new(None);

    fn installed() -> Option<Arc<Seam>> {
        INSTALLED.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub(in crate::commands::list) fn arrive(side: Gather) {
        let Some(seam) = installed() else {
            return;
        };
        let record = |events: &mut Events| match side {
            Gather::List => events.list = true,
            Gather::Graph => events.graph = true,
        };
        match (seam.mode, side) {
            (Mode::Rendezvous, _) => {
                let mut events = seam.update_and_wait(record, |events| match side {
                    Gather::List => !events.graph,
                    Gather::Graph => !events.list,
                });
                match side {
                    Gather::List => events.list_saw_graph = events.graph,
                    Gather::Graph => events.graph_saw_list = events.list,
                }
            }
            (Mode::HoldListUntilRemote, Gather::List) => {
                let mut events = seam.update_and_wait(record, |events| !events.remote_done);
                events.list_saw_remote_done = events.remote_done;
            }
            (Mode::Observe | Mode::HoldListUntilRemote, _) => drop(seam.update_and_wait(record, |_| false)),
        }
    }

    pub(in crate::commands::list) fn finished(side: Gather) {
        if let Some(seam) = installed() {
            let mut events = seam.lock();
            match side {
                Gather::List => events.list_done = true,
                Gather::Graph => events.graph_done = true,
            }
            seam.changed.notify_all();
        }
    }

    pub(in crate::commands::list) fn remote_finished() {
        if let Some(seam) = installed() {
            seam.lock().remote_done = true;
            seam.changed.notify_all();
        }
    }

    /// For a stub worker holding its outcome: waits (bounded) until `done`
    /// holds; `false` on timeout or without an installed seam.
    fn await_events(done: impl Fn(&Events) -> bool) -> bool {
        let Some(seam) = installed() else {
            return false;
        };
        let events = seam.lock();
        let (events, _) = seam
            .changed
            .wait_timeout_while(events, WAIT, |events| !done(events))
            .unwrap_or_else(|e| e.into_inner());
        done(&events)
    }

    /// Both local gathers have started.
    pub(in crate::commands::list) fn await_both_started() -> bool {
        await_events(|events| events.list && events.graph)
    }

    /// Both local gathers have finished (a gather that does not run, such as
    /// the graph on a non-image path, never finishes).
    pub(in crate::commands::list) fn await_both_finished() -> bool {
        await_events(|events| events.list_done && events.graph_done)
    }

    /// Uninstalls on drop, so a failed assertion cannot leak the seam into a
    /// later test in the same process.
    pub(in crate::commands::list) struct Installed(Arc<Seam>);

    impl Installed {
        pub(in crate::commands::list) fn new() -> Self {
            Self::with_mode(Mode::Rendezvous)
        }

        pub(in crate::commands::list) fn with_mode(mode: Mode) -> Self {
            let seam = Arc::new(Seam { mode, events: Mutex::default(), changed: Condvar::new() });
            *INSTALLED.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::clone(&seam));
            Installed(seam)
        }

        /// `(list gather saw the graph start, graph gather saw the list start)`.
        pub(in crate::commands::list) fn outcome(&self) -> (bool, bool) {
            let events = self.0.lock();
            (events.list_saw_graph, events.graph_saw_list)
        }

        /// `(list gather started, graph gather started)`.
        pub(in crate::commands::list) fn started(&self) -> (bool, bool) {
            let events = self.0.lock();
            (events.list, events.graph)
        }

        /// The list gather started only after the remote wait returned.
        pub(in crate::commands::list) fn list_started_after_the_wait(&self) -> bool {
            self.0.lock().list_saw_remote_done
        }
    }

    impl Drop for Installed {
        fn drop(&mut self) {
            *INSTALLED.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }
}

#[test]
#[serial_test::serial]
fn run_pipeline_gathers_the_graph_while_list_gather_is_unfinished() {
    let repo = temp_repo_with_feature_branch();
    let _guard = DirGuard::enter(repo.path());
    let terminal = Terminal::builder().width(120).build();
    let rendezvous = overlap::Installed::new();

    let result = super::run_pipeline(
        None,
        false,
        false,
        super::ListFlags::default(),
        std::time::Instant::now(),
        ImageSupport::Kitty,
        &terminal,
        NO_PRS,
    );

    assert!(result.is_ok(), "run_pipeline should succeed: {:?}", result.err());
    assert_eq!(
        rendezvous.outcome(),
        (true, true),
        "(list gather saw graph start, graph gather saw list start): \
         each gather must start before the other finishes"
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
        super::ListFlags::default(),
        std::time::Instant::now(),
        ImageSupport::Kitty,
        &narrow,
        NO_PRS,
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
    let collector = collector.as_ref().expect("collector");
    let top_level: Vec<String> = perf_shape(collector).into_iter().map(|(label, _)| label).collect();
    assert_eq!(top_level[..3], ["pre-dispatch", "pr gather", "local gather"], "{top_level:?}");
    assert_eq!(top_level.last().map(String::as_str), Some("unattributed"));
    assert_eq!(perf_group(collector, "local gather"), ["list gather", "graph gather"]);
    assert!(!names.contains(&"remote wait"), "a local-only listing has no remote wait row: {names:?}");
    assert_perf_reconciles(collector);
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
        super::ListFlags::default(),
        std::time::Instant::now(),
        ImageSupport::None,
        &terminal,
        NO_PRS,
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
    let collector = collector.as_ref().expect("collector");
    assert_eq!(perf_group(collector, "local gather"), ["list gather"]);
    assert_perf_reconciles(collector);
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

    // Base view from the main checkout: one shallow check; per unmerged
    // branch, one `merge-base --is-ancestor` (its only candidate lane is the
    // default one) and one merge base; one log for the default lane plus one
    // per branch. Each branch lane's boundary is classified too, which costs
    // one more `--is-ancestor` and one first-parent chain (`rev-list`) when
    // the boundary is an ordinary fork.
    let parsed = parse_worktree_state().expect("parse");
    let input = git_graph::GatherInput::from_list(&parsed, parsed.refs());
    recorder::start_recording();
    let t0 = Instant::now();
    let (graph, verbose) = git_graph::gather(&input, true, false);
    let base_elapsed = t0.elapsed();
    let base_calls = recorder::finish_recording();

    let graph = graph.expect("base view");
    assert!(verbose.is_none());
    let is_ancestor = recorder::count_matching(&base_calls, |args| args.get(1).map(String::as_str) == Some("--is-ancestor"));
    assert_eq!(count(&base_calls, "rev-parse"), 1, "got {base_calls:?}");
    assert_eq!(is_ancestor, 2 * graph.lines.len(), "got {base_calls:?}");
    assert_eq!(count(&base_calls, "merge-base") - is_ancestor, graph.lines.len(), "got {base_calls:?}");
    assert_eq!(count(&base_calls, "rev-list"), graph.lines.len(), "got {base_calls:?}");
    assert_eq!(count(&base_calls, "log"), 1 + graph.lines.len(), "got {base_calls:?}");
    eprintln!("base view gather: {base_elapsed:.2?}, {} git calls", base_calls.len());
}

/// `gather_remote` against a real repository and stores, with a counting stub
/// in place of the worker. The stub "runs" at launch: it writes its attempt,
/// a PR answer through the real `refresh` when asked, and its receipt, to the
/// real stores.
mod gather {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use worktree::pull_requests::{
        FetchedPrs, OpenPrSource, OpenPullRequest, PrListing, PrRequestError, RefreshOutcome, origin_digest, refresh,
        unix_now,
    };
    use worktree::remote_head::{
        ApiCondition, ApiNote, Attempt, CheckFailure, CredentialEvidence, HeadStatus, Outcome, PrFailure, PrStatus, Receipt, begin_attempt,
        finish_attempt, receipt_path_beside, set_credentials, write_receipt,
    };

    use super::super::list_table::{CredentialCondition, LastKnown, PrOutcome, RemoteStatus};
    use super::super::wait::{HeadEnd, PrEnd};
    use super::super::{
        LaunchArgs, ListFlags, ListSeams, RemoteAnswers, RequestNotices, Stores, WorkerHandle, caption_status,
        gather_remote, observed_pr_failure, request_notices,
    };
    use super::{recorder, run_git, temp_repo};

    const ORIGIN: &str = "https://prs.example.invalid/owner/repo.git";
    const ELSEWHERE: &str = "https://prs.example.invalid/other/repo.git";
    const LAST: LastKnown = LastKnown::Never;
    /// Long enough for any stub; a silent worker is waited for this long.
    const BUDGET: Duration = Duration::from_millis(300);

    /// Every launch, by main checkout, in order.
    static LAUNCHES: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    /// What the stub worker does.
    static WORKER: Mutex<Option<StubWorker>> = Mutex::new(None);

    #[derive(Clone)]
    struct StubWorker {
        head_store: PathBuf,
        pr_store: PathBuf,
        repo: PathBuf,
        /// An answer to publish before finishing, if any: PR `n`, or no PRs.
        publishes: Option<Option<u64>>,
        /// The PR half's status in the receipt.
        prs: PrStatus,
        /// What happens to `origin` after publishing, as if changed meanwhile.
        changes_origin: Option<OriginChange>,
        /// The `origin` the worker was launched for, which its attempt,
        /// receipt, and publication are bound to.
        origin: String,
        /// How the head attempt ends: its outcome, its API note, and the
        /// credentials its check was sent with.
        head: (Outcome, Option<ApiNote>, CredentialEvidence),
        /// The credentials the published PR answer records.
        pr_credentials: CredentialEvidence,
    }

    #[derive(Clone, Copy, Debug)]
    enum OriginChange {
        Replaced(&'static str),
        Removed,
    }

    fn record(main: &Path) {
        LAUNCHES.lock().unwrap_or_else(|e| e.into_inner()).push(main.to_path_buf());
    }

    fn launches() -> Vec<PathBuf> {
        LAUNCHES.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn open_pr(number: u64) -> Vec<OpenPullRequest> {
        vec![OpenPullRequest {
            number,
            url: None,
            source_repo: Some("owner/repo".into()),
            source_branch: "feat".into(),
            target_branch: "main".into(),
        }]
    }

    /// Answers PR `number`, or no PRs for `None`, with the given credentials.
    struct Seed(Option<u64>, CredentialEvidence);

    impl OpenPrSource for Seed {
        fn source_repo(&self) -> Option<String> {
            Some("owner/repo".into())
        }
        fn fetch(&self) -> Result<FetchedPrs, PrRequestError> {
            Ok(FetchedPrs { pull_requests: self.0.map(open_pr).unwrap_or_default(), credentials: self.1.clone() })
        }
    }

    /// Publishes `answer` through the worker's own writer, stamped `age`
    /// seconds ago.
    fn publish(store: &Path, repo: &Path, age: u64, answer: Option<u64>, credentials: CredentialEvidence) {
        let outcome = refresh(store, repo, || unix_now() - age, move |_| {
            Box::new(Seed(answer, credentials.clone())) as Box<dyn OpenPrSource>
        });
        assert_eq!(outcome, RefreshOutcome::Refreshed);
    }

    /// A worker that finishes its attempt in sync at once (publishing a PR
    /// answer first when asked), writes its receipt, and exits.
    fn finishing_launch(main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        record(main);
        let worker = WORKER.lock().unwrap_or_else(|e| e.into_inner()).clone().expect("fixture");
        if let Some(answer) = worker.publishes {
            publish(&worker.pr_store, &worker.repo, 0, answer, worker.pr_credentials.clone());
        }
        match worker.changes_origin {
            Some(OriginChange::Replaced(origin)) => run_git(&worker.repo, &["remote", "set-url", "origin", origin]),
            Some(OriginChange::Removed) => run_git(&worker.repo, &["remote", "remove", "origin"]),
            None => {}
        }
        let attempt = Attempt::begin(args.attempt.clone(), origin_digest(&worker.origin), "main".into(), unix_now());
        begin_attempt(&worker.head_store, &attempt).expect("attempt");
        let (outcome, api, credentials) = worker.head;
        set_credentials(&worker.head_store, &args.attempt, credentials).expect("credentials");
        finish_attempt(&worker.head_store, &args.attempt, outcome, api).expect("outcome");
        let receipt = Receipt {
            attempt_id: args.attempt.clone(),
            origin_digest: origin_digest(&worker.origin),
            branch: "main".into(),
            finished_at: unix_now(),
            head: HeadStatus::Ok,
            prs: worker.prs,
        };
        write_receipt(&receipt_path_beside(&worker.head_store, &args.attempt).expect("path"), &receipt).expect("receipt");
        Ok(WorkerHandle::new(|| true))
    }

    /// A worker that runs past any budget without recording anything.
    fn silent_launch(main: &Path, _: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        record(main);
        Ok(WorkerHandle::new(|| false))
    }

    fn failing_launch(main: &Path, _: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        record(main);
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no wt"))
    }

    struct Fixture {
        repo: tempfile::TempDir,
        cache: tempfile::TempDir,
        store: PathBuf,
        head_store: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            LAUNCHES.lock().unwrap_or_else(|e| e.into_inner()).clear();
            let repo = temp_repo();
            run_git(repo.path(), &["remote", "add", "origin", ORIGIN]);
            let cache = tempfile::tempdir().expect("cache dir");
            let store = cache.path().join("prs.json");
            let head_store = cache.path().join("remote-head.json");
            *WORKER.lock().unwrap_or_else(|e| e.into_inner()) = Some(StubWorker {
                head_store: head_store.clone(),
                pr_store: store.clone(),
                repo: repo.path().to_path_buf(),
                publishes: None,
                prs: PrStatus::Ok,
                changes_origin: None,
                origin: ORIGIN.into(),
                head: (Outcome::InSync, None, CredentialEvidence::Unknown),
                pr_credentials: CredentialEvidence::Unknown,
            });
            Self { repo, cache, store, head_store }
        }

        fn worker(&self, change: impl FnOnce(&mut StubWorker)) {
            if let Some(worker) = WORKER.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                change(worker);
            }
        }

        fn main(&self) -> &Path {
            self.repo.path()
        }

        /// Stores PR `number` (or no PRs) for the current origin, fetched
        /// `age` seconds ago.
        fn seed(&self, age: u64, number: Option<u64>) {
            publish(&self.store, self.main(), age, number, CredentialEvidence::Unknown);
        }

        fn gather_with(&self, launch: super::super::WorkerLaunch, flags: ListFlags) -> RemoteAnswers {
            let seams = ListSeams { launch, wait_budget: BUDGET, forced_budget: BUDGET };
            let stores = Stores { prs: &self.store, head: &self.head_store };
            gather_remote(stores, self.main(), "main", flags, seams).expect("gathered")
        }

        fn gather(&self) -> RemoteAnswers {
            self.gather_with(finishing_launch, ListFlags::default())
        }

        /// The receipt files left beside the stores.
        fn receipts_left(&self) -> Vec<String> {
            std::fs::read_dir(self.cache.path())
                .expect("cache dir")
                .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
                .filter(|name| name.contains("receipt"))
                .collect()
        }
    }

    fn numbers(listing: &PrListing) -> Vec<u64> {
        listing.pull_requests.iter().map(|pr| pr.number).collect()
    }

    fn head(answers: &RemoteAnswers) -> &HeadEnd {
        &answers.waited.as_ref().expect("launched").head
    }

    fn prs(answers: &RemoteAnswers) -> &PrEnd {
        &answers.waited.as_ref().expect("launched").prs
    }

    /// The PR failure the credentials line would read, behind the identity
    /// guard.
    fn pr_failure(answers: &RemoteAnswers) -> Option<&PrFailure> {
        answers.observed().and_then(|observed| observed_pr_failure(&observed))
    }

    fn in_sync(answers: &RemoteAnswers) -> bool {
        matches!(head(answers), HeadEnd::Finished(attempt) if attempt.outcome == Some(Outcome::InSync))
    }

    /// The origin lookup before the launch and the recheck after the wait:
    /// the foreground makes no other git call, no `ls-remote` or fetch.
    fn origin_lookups() -> Vec<Vec<String>> {
        vec![["remote", "get-url", "origin"].map(String::from).to_vec(); 2]
    }

    #[test]
    #[serial_test::serial]
    fn every_listing_with_an_origin_launches_once_and_waits_for_both_halves() {
        let fixture = Fixture::new();
        fixture.seed(10, Some(99));

        recorder::start_recording();
        let answers = fixture.gather();
        let git_calls = recorder::finish_recording();

        assert_eq!(git_calls, origin_lookups());
        assert_eq!(launches(), [fixture.main().to_path_buf()], "a fresh answer still launches the worker");
        assert!(in_sync(&answers), "the head outcome was waited for");
        assert_eq!(prs(&answers), &PrEnd::Published, "and the receipt");
        assert!(!answers.waited.as_ref().unwrap().timed_out);
        assert_eq!(request_notices(&answers).pr_outcome, Some(PrOutcome::Published));
        assert_eq!(numbers(&answers.prs), [99]);
        assert!(fixture.receipts_left().is_empty(), "the receipt is discarded: {:?}", fixture.receipts_left());
    }

    #[test]
    #[serial_test::serial]
    fn a_pr_answer_the_worker_publishes_during_the_wait_is_shown() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, Some(99));
        fixture.worker(|worker| worker.publishes = Some(Some(8)));

        let answers = fixture.gather();

        assert_eq!(numbers(&answers.prs), [8]);
        assert!(!answers.prs.is_stale_at(unix_now()));
        assert_eq!(request_notices(&answers).pr_outcome, Some(PrOutcome::Published));
    }

    #[test]
    #[serial_test::serial]
    fn an_empty_answer_published_during_the_wait_clears_the_badges() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, Some(99));
        fixture.worker(|worker| worker.publishes = Some(None));

        let answers = fixture.gather();

        assert!(answers.prs.pull_requests.is_empty(), "{:?}", answers.prs);
        assert!(answers.prs.fetched_at.is_some(), "an empty answer is an answer");
        assert_eq!(request_notices(&answers).pr_outcome, Some(PrOutcome::Published));
    }

    #[test]
    #[serial_test::serial]
    fn a_failed_refresh_keeps_the_stored_answer_and_is_this_runs_failure() {
        let rejected = PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) };
        for seeded in [Some(Some(99)), Some(None), None] {
            let fixture = Fixture::new();
            if let Some(answer) = seeded {
                fixture.seed(10, answer);
            }
            let stored = std::fs::read(&fixture.store).ok();
            let failure = rejected.clone();
            fixture.worker(move |worker| worker.prs = PrStatus::Failed { failure });

            let answers = fixture.gather();

            assert_eq!(request_notices(&answers).pr_outcome, Some(PrOutcome::Failed), "{seeded:?}");
            assert_eq!(pr_failure(&answers), Some(&rejected), "kept for the credentials line");
            assert_eq!(numbers(&answers.prs), seeded.flatten().into_iter().collect::<Vec<_>>(), "{seeded:?}");
            assert_eq!(answers.prs.fetched_at.is_some(), seeded.is_some(), "an empty answer is still an answer");
            assert_eq!(std::fs::read(&fixture.store).ok(), stored, "the parent writes nothing");
            assert!(in_sync(&answers), "the head outcome is independent");
        }
    }

    #[test]
    #[serial_test::serial]
    fn an_ignored_or_unsupported_pr_half_is_this_runs_outcome_and_no_failure() {
        for (status, expected) in [(PrStatus::Ignored, PrOutcome::Ignored), (PrStatus::Unsupported, PrOutcome::Unsupported)] {
            let fixture = Fixture::new();
            fixture.seed(10, Some(99));
            let worker_status = status.clone();
            fixture.worker(move |worker| worker.prs = worker_status);

            let answers = fixture.gather();

            assert_eq!(launches().len(), 1, "the worker is launched and decides: {status:?}");
            assert_eq!(request_notices(&answers).pr_outcome, Some(expected), "{status:?}");
            assert_eq!(pr_failure(&answers), None, "{status:?} is not a failure");
            assert!(in_sync(&answers), "the head half still answers: {status:?}");
            assert!(!answers.waited.as_ref().unwrap().timed_out, "{status:?}");
        }
    }

    #[test]
    #[serial_test::serial]
    fn a_changed_origin_never_shows_the_old_answers() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, Some(99));
        run_git(fixture.main(), &["remote", "set-url", "origin", ELSEWHERE]);

        let answers = fixture.gather();

        assert_eq!(answers.prs, PrListing::default(), "the old answer belongs to another origin");
        assert_eq!(launches().len(), 1);
    }

    #[test]
    #[serial_test::serial]
    fn an_origin_changed_during_the_wait_shows_neither_its_badges_nor_its_failure() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, Some(99));
        fixture.worker(|worker| {
            worker.publishes = Some(Some(8));
            worker.prs = PrStatus::Failed { failure: PrFailure::CredentialsRejected { key: None } };
            worker.changes_origin = Some(OriginChange::Replaced(ELSEWHERE));
        });

        let answers = fixture.gather();

        assert!(answers.origin_changed);
        assert_eq!(answers.prs, PrListing::default(), "the old origin's answer is not shown");
        assert_eq!(request_notices(&answers).pr_outcome, None, "nor its PR item");
        assert_eq!(pr_failure(&answers), None, "nor its credentials line");
        assert_eq!(launches().len(), 1, "no second refresh for the new origin");
    }

    type Shape = Box<dyn Fn(&mut StubWorker)>;
    type Shows = Box<dyn Fn(&RemoteAnswers) -> bool>;

    /// One worker shape per projection of request evidence, with the check
    /// that the projection shows that evidence.
    fn projections() -> Vec<(&'static str, Shape, Shows)> {
        let line_is = |expected: CredentialCondition| {
            Box::new(move |answers: &RemoteAnswers| {
                request_notices(answers).credential_line.is_some_and(|line| line.condition == expected)
            }) as Shows
        };
        let head_note = |outcome: Outcome, condition: ApiCondition, key: Option<&'static str>| {
            Box::new(move |worker: &mut StubWorker| {
                let note = ApiNote { condition, key: key.map(str::to_string), fallback_answered: false };
                worker.head = (outcome, Some(note), CredentialEvidence::Unknown);
            }) as Shape
        };
        let answered = Outcome::InSync;
        let failed = Outcome::CheckFailed { reason: CheckFailure::Other };
        vec![
            (
                "caption head status",
                Box::new(|worker| worker.head = (Outcome::Fetched, None, CredentialEvidence::Unknown)),
                Box::new(|answers| caption_status(answers, || LAST) == Some(RemoteStatus::Fetched)),
            ),
            (
                "head warning: rejected key",
                head_note(answered, ApiCondition::CredentialsRejected, Some("GITHUB_TOKEN")),
                line_is(CredentialCondition::Rejected),
            ),
            (
                "head warning: insufficient key",
                head_note(answered, ApiCondition::CredentialsInsufficient, Some("GITHUB_TOKEN")),
                line_is(CredentialCondition::Insufficient),
            ),
            (
                "head warning: keyed rate limit",
                head_note(answered, ApiCondition::RateLimited { authenticated: true }, Some("GITHUB_TOKEN")),
                line_is(CredentialCondition::RateLimited { authenticated: true }),
            ),
            (
                "head warning: anonymous rate limit",
                head_note(answered, ApiCondition::RateLimited { authenticated: false }, None),
                line_is(CredentialCondition::RateLimited { authenticated: false }),
            ),
            (
                "head warning: repository not visible",
                head_note(failed, ApiCondition::CredentialsRequired, None),
                line_is(CredentialCondition::NotVisible),
            ),
            (
                "PR warning",
                Box::new(|worker| {
                    worker.prs = PrStatus::Failed { failure: PrFailure::CredentialsRejected { key: None } };
                }),
                line_is(CredentialCondition::Rejected),
            ),
            (
                "PR item and badges",
                Box::new(|worker| worker.publishes = Some(Some(8))),
                Box::new(|answers| {
                    request_notices(answers).pr_outcome == Some(PrOutcome::Published) && numbers(&answers.prs) == [8]
                }),
            ),
            (
                "keyless notice: head",
                Box::new(|worker| worker.head = (Outcome::InSync, None, CredentialEvidence::Anonymous)),
                line_is(CredentialCondition::AnsweredWithoutKey { higher_limits: true }),
            ),
            (
                "keyless notice: PR",
                Box::new(|worker| {
                    worker.publishes = Some(None);
                    worker.pr_credentials = CredentialEvidence::Anonymous;
                }),
                line_is(CredentialCondition::AnsweredWithoutKey { higher_limits: true }),
            ),
            (
                "closing fallback notice",
                Box::new(|worker| {
                    let note = ApiNote { condition: ApiCondition::CredentialsRequired, key: None, fallback_answered: true };
                    worker.head = (Outcome::InSync, Some(note), CredentialEvidence::Unknown);
                }),
                Box::new(|answers| request_notices(answers).fallback_notice.is_some()),
            ),
        ]
    }

    /// Every projection of request evidence goes through the one identity
    /// guard: each shows its evidence while `origin` is unchanged, and none
    /// does once `origin` was replaced or removed during the wait. The
    /// caption then reads as a check that could not be made.
    #[test]
    #[serial_test::serial]
    fn a_replaced_or_removed_origin_suppresses_every_request_notice() {
        const PROVIDER_ORIGIN: &str = "https://github.com/owner/repo.git";
        for (projection, shape, shows) in projections() {
            for change in [None, Some(OriginChange::Replaced(ELSEWHERE)), Some(OriginChange::Removed)] {
                let fixture = Fixture::new();
                run_git(fixture.main(), &["remote", "set-url", "origin", PROVIDER_ORIGIN]);
                fixture.worker(|worker| {
                    worker.origin = PROVIDER_ORIGIN.into();
                    worker.changes_origin = change;
                    shape(worker);
                });

                let answers = fixture.gather();

                assert!(answers.waited.is_some(), "{projection}, {change:?}: launched and waited for");
                if change.is_none() {
                    assert!(!answers.origin_changed, "{projection}");
                    assert!(shows(&answers), "{projection}: the control shows its evidence");
                    continue;
                }
                assert!(answers.origin_changed, "{projection}, {change:?}");
                assert!(!shows(&answers), "{projection}, {change:?}");
                assert_eq!(request_notices(&answers), RequestNotices::default(), "{projection}, {change:?}");
                assert_eq!(answers.prs, PrListing::default(), "{projection}, {change:?}: no badges");
                assert_eq!(
                    caption_status(&answers, || LAST),
                    Some(RemoteStatus::CheckFailed { reason: CheckFailure::Other, last: LAST }),
                    "{projection}, {change:?}: the caption says nothing the old check found"
                );
            }
        }
    }

    #[test]
    #[serial_test::serial]
    fn a_worker_that_cannot_launch_is_unavailable_without_a_wait() {
        let fixture = Fixture::new();
        fixture.seed(10, Some(99));

        let started = Instant::now();
        let answers = fixture.gather_with(failing_launch, ListFlags::default());

        assert_eq!(head(&answers), &HeadEnd::Unavailable);
        assert_eq!(prs(&answers), &PrEnd::Failed(PrFailure::Other));
        assert!(started.elapsed() < BUDGET, "{:?}", started.elapsed());
        assert_eq!(numbers(&answers.prs), [99], "stored answers stay shown");
        assert_eq!(request_notices(&answers).pr_outcome, Some(PrOutcome::Failed));
    }

    #[test]
    #[serial_test::serial]
    fn a_silent_worker_is_waited_for_only_until_the_budget() {
        for flags in [ListFlags::default(), ListFlags { refresh: true, ..ListFlags::default() }] {
            let fixture = Fixture::new();
            fixture.seed(10, Some(99));

            let started = Instant::now();
            let answers = fixture.gather_with(silent_launch, flags);

            let elapsed = started.elapsed();
            let waited = answers.waited.as_ref().expect("launched");
            assert_eq!((&waited.head, &waited.prs, waited.timed_out), (&HeadEnd::Running { last: None }, &PrEnd::Pending, true));
            assert!(elapsed >= BUDGET && elapsed < BUDGET * 4, "{elapsed:?}");
            assert_eq!(request_notices(&answers).pr_outcome, Some(PrOutcome::Pending));
            assert_eq!(numbers(&answers.prs), [99], "the stored answer as of the timeout");
        }
    }

    #[test]
    #[serial_test::serial]
    fn without_an_origin_stored_answers_are_ignored_and_nothing_is_launched() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, Some(99));
        run_git(fixture.main(), &["remote", "remove", "origin"]);

        let answers = fixture.gather();

        assert_eq!(answers.prs, PrListing::default());
        assert!(answers.origin.is_none());
        assert!(answers.waited.is_none());
        assert_eq!(request_notices(&answers).pr_outcome, None);
        assert!(launches().is_empty(), "no launch: {:?}", launches());
    }
}

/// What this run observed, turned into the caption row, the §5 line, and the
/// §8 notice.
mod observations {
    use worktree::remote_head::{
        ApiCondition, ApiNote, Attempt, CheckFailure, FallbackReason, FetchFailure, Outcome, Phase, PrFailure,
        UnavailableReason,
    };

    use super::super::list_table::{CredentialCondition, LastKnown, RemoteStatus};
    use super::super::wait::HeadEnd;
    use super::super::{credential_line, fallback_notice, remote_status};

    const GITHUB: &str = "https://github.com/owner/repo.git";
    const LAST: LastKnown = LastKnown::Answer { checked_at: 1_790_000_000 };

    fn attempt(phase: Phase, outcome: Option<Outcome>, api: Option<ApiNote>) -> Attempt {
        Attempt {
            phase,
            outcome,
            api,
            ..Attempt::begin("0".repeat(32), "digest".into(), "main".into(), 1_790_000_000)
        }
    }

    fn finished(outcome: Outcome) -> HeadEnd {
        HeadEnd::Finished(attempt(Phase::Checking, Some(outcome), None))
    }

    fn note(condition: ApiCondition, key: Option<&str>, fallback_answered: bool) -> Option<ApiNote> {
        Some(ApiNote { condition, key: key.map(str::to_string), fallback_answered })
    }

    #[test]
    fn every_ending_maps_to_its_caption_row() {
        let status = |head: &HeadEnd| remote_status(head, || LAST);
        assert_eq!(status(&finished(Outcome::InSync)), RemoteStatus::CheckedNow);
        assert_eq!(status(&finished(Outcome::Fetched)), RemoteStatus::Fetched);
        assert_eq!(status(&finished(Outcome::Absent)), RemoteStatus::Absent);
        assert_eq!(
            status(&finished(Outcome::FetchFailed { reason: FetchFailure::Timeout })),
            RemoteStatus::FetchFailed { reason: FetchFailure::Timeout }
        );
        assert_eq!(
            status(&finished(Outcome::CheckFailed { reason: CheckFailure::Credentials })),
            RemoteStatus::CheckFailed { reason: CheckFailure::Credentials, last: LAST }
        );
        assert_eq!(
            status(&finished(Outcome::Unavailable { reason: UnavailableReason::OriginChanged })),
            RemoteStatus::CheckFailed { reason: CheckFailure::Other, last: LAST },
            "a discarded answer reads as an unavailable check"
        );
        assert_eq!(status(&HeadEnd::Unavailable), RemoteStatus::CheckFailed { reason: CheckFailure::Other, last: LAST });
        assert_eq!(
            status(&HeadEnd::Running { last: Some(attempt(Phase::Fetching, None, None)) }),
            RemoteStatus::StillPulling
        );
        let fallback = Phase::CheckingFallback { reason: FallbackReason::NoKey };
        for last in [None, Some(attempt(Phase::Checking, None, None)), Some(attempt(fallback, None, None))] {
            assert_eq!(status(&HeadEnd::Running { last }), RemoteStatus::StillChecking { last: LAST });
        }
    }

    #[test]
    fn only_a_row_without_a_current_answer_asks_what_was_last_known() {
        let asked = std::cell::Cell::new(false);
        let last = || {
            asked.set(true);
            LAST
        };
        remote_status(&finished(Outcome::InSync), last);
        assert!(!asked.get(), "no reflog read for a finished check");
    }

    #[test]
    fn each_confirmed_condition_gives_its_line_with_the_key_used_or_the_keys_accepted() {
        let failed = Some(Outcome::CheckFailed { reason: CheckFailure::Other });
        let answered = Some(Outcome::InSync);
        let line = |outcome, api| credential_line(GITHUB, Some(&attempt(Phase::Checking, outcome, api)), None, false);
        let accepted = "GH_TOKEN or GITHUB_TOKEN";

        let cases = [
            (note(ApiCondition::CredentialsRequired, None, false), failed, Some((CredentialCondition::NotVisible, accepted))),
            (note(ApiCondition::NotFoundOrNotPermitted, None, false), failed, Some((CredentialCondition::NotVisible, accepted))),
            // The fallback answered: §8 applies instead.
            (note(ApiCondition::CredentialsRequired, None, true), answered, None),
            (note(ApiCondition::NotFoundOrNotPermitted, None, true), answered, None),
            // A 404 with a key set is ambiguous and says nothing.
            (note(ApiCondition::NotFoundOrNotPermitted, Some("GITHUB_TOKEN"), false), failed, None),
            (
                note(ApiCondition::CredentialsRejected, Some("GITHUB_TOKEN"), true),
                answered,
                Some((CredentialCondition::Rejected, "GITHUB_TOKEN")),
            ),
            (
                note(ApiCondition::CredentialsInsufficient, Some("GH_TOKEN"), true),
                answered,
                Some((CredentialCondition::Insufficient, "GH_TOKEN")),
            ),
            (
                note(ApiCondition::RateLimited { authenticated: false }, None, true),
                answered,
                Some((CredentialCondition::RateLimited { authenticated: false }, accepted)),
            ),
            (
                note(ApiCondition::RateLimited { authenticated: true }, Some("GITHUB_TOKEN"), true),
                answered,
                Some((CredentialCondition::RateLimited { authenticated: true }, "GITHUB_TOKEN")),
            ),
            (None, answered, None),
        ];
        for (api, outcome, expected) in cases {
            let got = line(outcome, api.clone()).map(|line| {
                assert_eq!(line.provider, "GitHub");
                (line.condition, line.key)
            });
            assert_eq!(got, expected.map(|(condition, key)| (condition, key.to_string())), "{api:?} {outcome:?}");
        }
    }

    #[test]
    fn a_pr_failure_this_run_observed_gives_a_line_only_for_a_confirmed_condition() {
        let line = |failure: PrFailure| credential_line(GITHUB, None, Some(&failure), false).map(|line| line.condition);
        assert_eq!(
            line(PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) }),
            Some(CredentialCondition::Rejected)
        );
        assert_eq!(line(PrFailure::CredentialsInsufficient { key: None }), Some(CredentialCondition::Insufficient));
        assert_eq!(
            line(PrFailure::RateLimited { authenticated: true, key: None }),
            Some(CredentialCondition::RateLimited { authenticated: true })
        );
        for silent in [PrFailure::CredentialsRequired, PrFailure::NotFoundOrNotPermitted, PrFailure::Other] {
            assert_eq!(line(silent.clone()), None, "{silent:?}");
        }
        // Nothing observed this run (no receipt by the end of the wait),
        // nothing said.
        assert_eq!(credential_line(GITHUB, None, None, false), None);
        // A remote that is no supported provider has no line at all.
        let rejected = PrFailure::CredentialsRejected { key: None };
        assert_eq!(credential_line("/srv/git/repo.git", None, Some(&rejected), false), None);
    }

    #[test]
    fn a_confirmed_head_condition_outranks_this_runs_pr_failure() {
        let answered = Some(Outcome::InSync);
        let failed = Some(Outcome::CheckFailed { reason: CheckFailure::Other });
        let pr = PrFailure::RateLimited { authenticated: true, key: Some("GH_TOKEN".into()) };
        let line = |outcome, api| {
            credential_line(GITHUB, Some(&attempt(Phase::Checking, outcome, api)), Some(&pr), false)
                .map(|line| (line.condition, line.key))
        };

        assert_eq!(
            line(answered, note(ApiCondition::CredentialsRejected, Some("GITHUB_TOKEN"), true)),
            Some((CredentialCondition::Rejected, "GITHUB_TOKEN".into())),
            "the head's confirmed condition is the one line"
        );
        assert_eq!(
            line(failed, note(ApiCondition::CredentialsRequired, None, false)),
            Some((CredentialCondition::NotVisible, "GH_TOKEN or GITHUB_TOKEN".into()))
        );
        let from_pr = Some((CredentialCondition::RateLimited { authenticated: true }, "GH_TOKEN".into()));
        assert_eq!(line(answered, None), from_pr, "no head note: the PR failure speaks");
        assert_eq!(
            line(answered, note(ApiCondition::NotFoundOrNotPermitted, Some("GITHUB_TOKEN"), true)),
            from_pr,
            "an ambiguous head note asserts nothing, so the PR failure speaks"
        );
    }

    #[test]
    #[serial_test::serial]
    fn a_credentials_line_names_variables_and_never_their_values() {
        const SECRET: &str = "ghp_sentinel_value_that_must_never_print";
        // SAFETY: serial; restored below.
        let previous = std::env::var("GITHUB_TOKEN").ok();
        unsafe { std::env::set_var("GITHUB_TOKEN", SECRET) };
        let api = note(ApiCondition::CredentialsRejected, Some("GITHUB_TOKEN"), false);
        let failed = Some(Outcome::CheckFailed { reason: CheckFailure::Other });
        let line = credential_line(GITHUB, Some(&attempt(Phase::Checking, failed, api)), None, false).expect("a line");
        use biscuit_terminal::components::renderable::TerminalRenderable as _;
        let terminal = biscuit_terminal::terminal::Terminal::builder().is_tty(false).width(400).build();
        let rendered = biscuit_terminal::components::prose::Prose::new(super::super::list_table::credential_markup(&line))
            .render(&terminal);
        match previous {
            Some(value) => unsafe { std::env::set_var("GITHUB_TOKEN", value) },
            None => unsafe { std::env::remove_var("GITHUB_TOKEN") },
        }
        assert!(rendered.contains("GITHUB_TOKEN"), "{rendered}");
        assert!(!rendered.contains(SECRET), "{rendered}");
    }

    mod keyless {
        use worktree::remote_head::{
            ApiCondition, CheckFailure, CredentialEvidence, Outcome, Phase, PrFailure, UnavailableReason,
        };

        use super::super::super::list_table::{CredentialCondition, CredentialLine};
        use super::super::super::wait::{HeadEnd, PrEnd, WaitEnd};
        use super::super::super::{RemoteAnswers, request_notices};
        use super::{GITHUB, attempt, note};

        const GITEA: &str = "http://gitea.test/o/r.git";

        fn keyed() -> CredentialEvidence {
            CredentialEvidence::Keyed { variables: vec!["GH_TOKEN".into()] }
        }

        fn head(credentials: CredentialEvidence) -> HeadEnd {
            HeadEnd::Finished(worktree::remote_head::Attempt { credentials, ..attempt(Phase::Checking, Some(Outcome::InSync), None) })
        }

        fn answers(head: HeadEnd, prs: PrEnd, pr_credentials: CredentialEvidence) -> RemoteAnswers {
            RemoteAnswers {
                origin: Some(GITHUB.into()),
                waited: Some(WaitEnd { head, prs, pr_credentials, timed_out: false }),
                ..RemoteAnswers::default()
            }
        }

        /// The one credentials line `run_pipeline` would print for `remote`
        /// with `origin` as its origin.
        fn line_for(origin: &str, remote: RemoteAnswers) -> Option<CredentialLine> {
            request_notices(&RemoteAnswers { origin: Some(origin.into()), ..remote }).credential_line
        }

        fn condition(remote: RemoteAnswers) -> Option<CredentialCondition> {
            line_for(GITHUB, remote).map(|line| line.condition)
        }

        const NOTICE: Option<CredentialCondition> = Some(CredentialCondition::AnsweredWithoutKey { higher_limits: true });
        use CredentialEvidence::{Anonymous, Unknown};

        #[test]
        fn either_half_alone_or_both_give_one_notice() {
            let published = || PrEnd::Published;
            // Head only (the PR half failed generically, or answered keyed).
            assert_eq!(condition(answers(head(Anonymous), PrEnd::Failed(PrFailure::Other), Unknown)), NOTICE);
            assert_eq!(condition(answers(head(Anonymous), published(), keyed())), NOTICE, "a keyed PR answer hides nothing");
            // PR only, an empty answer included (the head check failed generically).
            let failed = HeadEnd::Finished(attempt(Phase::Checking, Some(Outcome::CheckFailed { reason: CheckFailure::Other }), None));
            assert_eq!(condition(answers(failed, published(), Anonymous)), NOTICE);
            assert_eq!(condition(answers(head(keyed()), published(), Anonymous)), NOTICE, "a keyed head answer hides nothing");
            assert_eq!(condition(answers(HeadEnd::Unavailable, published(), Anonymous)), NOTICE);
            // Both.
            assert_eq!(condition(answers(head(Anonymous), published(), Anonymous)), NOTICE);
            // Neither.
            assert_eq!(condition(answers(head(keyed()), published(), keyed())), None);
        }

        #[test]
        fn unknown_cached_or_receipt_only_evidence_never_gives_the_notice() {
            assert_eq!(condition(answers(head(Unknown), PrEnd::Published, Unknown)), None, "a success known only from a receipt");
            assert_eq!(condition(answers(head(Unknown), PrEnd::Pending, Anonymous)), None, "credentials of no accepted publication");
            assert_eq!(condition(answers(head(Unknown), PrEnd::Failed(PrFailure::Other), Unknown)), None);
            // No wait at all: only cached answers.
            let cached = RemoteAnswers { origin: Some(GITHUB.into()), ..RemoteAnswers::default() };
            assert_eq!(request_notices(&cached).credential_line, None);
            // An anonymous API failure alone adds no line.
            let no_key = attempt(Phase::Checking, Some(Outcome::InSync), note(ApiCondition::CredentialsRequired, None, true));
            assert_eq!(condition(answers(HeadEnd::Finished(no_key), PrEnd::Failed(PrFailure::CredentialsRequired), Unknown)), None);
        }

        #[test]
        fn an_answer_still_fetching_or_from_an_adopted_attempt_counts() {
            let fetching = worktree::remote_head::Attempt { credentials: Anonymous, ..attempt(Phase::Fetching, None, None) };
            assert_eq!(condition(answers(HeadEnd::Running { last: Some(fetching) }, PrEnd::Pending, Unknown)), NOTICE);
            // Another run's attempt, sent from that worker's environment.
            let adopted = worktree::remote_head::Attempt {
                id: "f".repeat(32),
                credentials: Anonymous,
                ..attempt(Phase::Checking, Some(Outcome::Fetched), None)
            };
            assert_eq!(condition(answers(HeadEnd::Finished(adopted), PrEnd::Pending, Unknown)), NOTICE);
        }

        #[test]
        fn ignored_changed_origin_unsupported_and_local_path_remotes_never_give_the_notice() {
            let both = || answers(head(Anonymous), PrEnd::Published, Anonymous);
            assert_eq!(condition(RemoteAnswers { ignored: true, ..both() }), None);
            assert_eq!(condition(RemoteAnswers { origin_changed: true, ..both() }), None);
            let unavailable = worktree::remote_head::Attempt {
                credentials: Anonymous,
                ..attempt(Phase::Checking, Some(Outcome::Unavailable { reason: UnavailableReason::OriginChanged }), None)
            };
            assert_eq!(condition(answers(HeadEnd::Finished(unavailable), PrEnd::Pending, Unknown)), None, "the worker saw origin change");
            for origin in ["/srv/git/repo.git", "https://git.internal.example/o/r.git"] {
                assert_eq!(line_for(origin, both()), None, "{origin}");
            }
        }

        #[test]
        fn a_confirmed_warning_outranks_the_notice() {
            let rejected = note(ApiCondition::CredentialsRejected, Some("GITHUB_TOKEN"), true);
            let warned = HeadEnd::Finished(attempt(Phase::Checking, Some(Outcome::InSync), rejected));
            assert_eq!(condition(answers(warned, PrEnd::Published, Anonymous)), Some(CredentialCondition::Rejected));

            let limited = PrFailure::RateLimited { authenticated: false, key: None };
            assert_eq!(
                condition(answers(head(Anonymous), PrEnd::Failed(limited), Unknown)),
                Some(CredentialCondition::RateLimited { authenticated: false })
            );
        }

        #[test]
        fn the_notice_promises_higher_limits_only_where_the_provider_gives_them() {
            let both = || answers(head(Anonymous), PrEnd::Published, Anonymous);
            for (origin, provider, variables, higher_limits) in [
                (GITHUB, "GitHub", "GH_TOKEN or GITHUB_TOKEN", true),
                ("https://gitlab.com/g/r.git", "GitLab", "GITLAB_TOKEN or GITLAB_PRIVATE_TOKEN", true),
                ("git@bitbucket.org:o/r.git", "Bitbucket", "BITBUCKET_TOKEN", true),
                (GITEA, "Gitea", "GITEA_TOKEN or FORGEJO_TOKEN or CODEBERG_TOKEN", false),
                ("https://codeberg.org/o/r.git", "Forgejo", "GITEA_TOKEN or FORGEJO_TOKEN or CODEBERG_TOKEN", false),
            ] {
                let line = line_for(origin, both()).unwrap_or_else(|| panic!("{origin}: a line"));
                assert_eq!(
                    (line.provider.as_str(), line.key.as_str(), line.condition),
                    (provider, variables, CredentialCondition::AnsweredWithoutKey { higher_limits }),
                    "{origin}"
                );
            }
        }
    }

    #[test]
    fn the_fallback_notice_follows_only_a_no_key_fallback_that_answered() {
        let notice = |api| fallback_notice(GITHUB, Some(&attempt(Phase::Checking, Some(Outcome::InSync), api)));
        let keys = Some(vec!["GH_TOKEN".to_string(), "GITHUB_TOKEN".to_string()]);
        assert_eq!(notice(note(ApiCondition::CredentialsRequired, None, true)), keys);
        assert_eq!(notice(note(ApiCondition::NotFoundOrNotPermitted, None, true)), keys, "a 404 without a key too");
        assert_eq!(notice(note(ApiCondition::CredentialsRequired, None, false)), None, "Git failed too");
        assert_eq!(notice(note(ApiCondition::RateLimited { authenticated: false }, None, true)), None, "never for a rate limit");
        assert_eq!(notice(note(ApiCondition::NotFoundOrNotPermitted, Some("GITHUB_TOKEN"), true)), None);
        assert_eq!(notice(None), None, "an ignored or unsupported remote asks no API");
        assert_eq!(fallback_notice(GITHUB, None), None);
    }
}
