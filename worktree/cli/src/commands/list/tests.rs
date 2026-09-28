
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use worktree::git::recorder;
use worktree::pull_requests::OpenPrSource;
use worktree::worktree::{list_worktrees, parse_worktree_state};

use crate::commands::git_graph;

/// These repositories have no origin, so the PR stage returns at once
/// without asking for a source.
fn no_prs(origin: &str) -> Box<dyn OpenPrSource> {
    panic!("no request expected without an origin, got one for {origin}")
}

/// Without an origin nothing is launched either.
fn no_launch(main: &Path, _: &super::LaunchArgs) -> std::io::Result<super::WorkerHandle> {
    panic!("no worker expected without an origin, got one for {}", main.display())
}

const NO_PRS: super::ListSeams = super::ListSeams {
    connect: no_prs,
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
        !names.contains(&"graph image render (biscuit-terminal)"),
        "graph image render should not be recorded, got: {names:?}"
    );
}

/// Test seam that proves the list and graph gathers overlap.
///
/// `run_pipeline` calls `arrive` as each gather starts. With a
/// rendezvous installed, each side waits (bounded) for the other to arrive, so
/// both succeed only when neither gather has to finish before the other
/// starts. A sequential pipeline times out on one side instead of hanging.
pub(super) mod overlap {
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::Duration;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(in crate::commands::list) enum Gather {
        List,
        Graph,
    }

    #[derive(Default)]
    struct Arrivals {
        list: bool,
        graph: bool,
        list_saw_graph: bool,
        graph_saw_list: bool,
    }

    #[derive(Default)]
    struct Rendezvous {
        arrivals: Mutex<Arrivals>,
        changed: Condvar,
    }

    /// Only a failing (sequential) pipeline waits this long; an overlapping
    /// one is released as soon as the second gather starts.
    const WAIT: Duration = Duration::from_secs(10);

    static INSTALLED: Mutex<Option<Arc<Rendezvous>>> = Mutex::new(None);

    /// Without an installed rendezvous this does nothing, so every other test
    /// runs the pipeline unchanged.
    pub(in crate::commands::list) fn arrive(side: Gather) {
        let Some(rendezvous) = INSTALLED.lock().unwrap_or_else(|e| e.into_inner()).clone() else {
            return;
        };
        let mut arrivals = rendezvous.arrivals.lock().unwrap_or_else(|e| e.into_inner());
        match side {
            Gather::List => arrivals.list = true,
            Gather::Graph => arrivals.graph = true,
        }
        rendezvous.changed.notify_all();
        let (mut arrivals, _) = rendezvous
            .changed
            .wait_timeout_while(arrivals, WAIT, |a| match side {
                Gather::List => !a.graph,
                Gather::Graph => !a.list,
            })
            .unwrap_or_else(|e| e.into_inner());
        match side {
            Gather::List => arrivals.list_saw_graph = arrivals.graph,
            Gather::Graph => arrivals.graph_saw_list = arrivals.list,
        }
    }

    /// Uninstalls on drop, so a failed assertion cannot leak the rendezvous
    /// into a later test in the same process.
    pub(in crate::commands::list) struct Installed(Arc<Rendezvous>);

    impl Installed {
        pub(in crate::commands::list) fn new() -> Self {
            let rendezvous = Arc::new(Rendezvous::default());
            *INSTALLED.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::clone(&rendezvous));
            Installed(rendezvous)
        }

        /// `(list gather saw the graph start, graph gather saw the list start)`.
        pub(in crate::commands::list) fn outcome(&self) -> (bool, bool) {
            let arrivals = self.0.arrivals.lock().unwrap_or_else(|e| e.into_inner());
            (arrivals.list_saw_graph, arrivals.graph_saw_list)
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
    // per branch.
    let input = git_graph::GatherInput::from_list(&parse_worktree_state().expect("parse"));
    recorder::start_recording();
    let t0 = Instant::now();
    let (graph, verbose) = git_graph::gather(&input, true, false);
    let base_elapsed = t0.elapsed();
    let base_calls = recorder::finish_recording();

    let graph = graph.expect("base view");
    assert!(verbose.is_none());
    let is_ancestor = recorder::count_matching(&base_calls, |args| args.get(1).map(String::as_str) == Some("--is-ancestor"));
    assert_eq!(count(&base_calls, "rev-parse"), 1, "got {base_calls:?}");
    assert_eq!(is_ancestor, graph.lines.len(), "got {base_calls:?}");
    assert_eq!(count(&base_calls, "merge-base") - is_ancestor, graph.lines.len(), "got {base_calls:?}");
    assert_eq!(count(&base_calls, "log"), 1 + graph.lines.len(), "got {base_calls:?}");
    eprintln!("base view gather: {base_elapsed:.2?}, {} git calls", base_calls.len());
}

/// `gather_remote` against a real repository and stores, with counting seams
/// in place of the PR provider and the worker. A stub worker "runs" at launch:
/// it writes its attempt (and, when asked, a PR answer) to the real stores.
mod gather {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use worktree::pull_requests::{
        CachedPrs, OpenPrSource, OpenPullRequest, PrListing, RefreshOutcome, fetch_and_publish, origin_digest, refresh,
        select_cached, unix_now,
    };
    use worktree::remote_head::{Attempt, Outcome, PrFailure, begin_attempt, finish_attempt};

    use super::super::wait::WaitEnd;
    use super::super::{LaunchArgs, ListFlags, ListSeams, RemoteAnswers, Stores, WorkerHandle, gather_remote};
    use super::{recorder, run_git, temp_repo};

    const ORIGIN: &str = "https://prs.example.invalid/owner/repo.git";
    /// Long enough for any stub; a silent worker is waited for this long.
    const BUDGET: Duration = Duration::from_millis(300);

    /// Every seam call, in order: `connect`, `fetch` (the request itself),
    /// and `launch`.
    static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());
    /// What the stub worker writes.
    static WORKER: Mutex<Option<StubWorker>> = Mutex::new(None);

    #[derive(Clone)]
    struct StubWorker {
        head_store: PathBuf,
        pr_store: PathBuf,
        repo: PathBuf,
        /// A PR number to publish before finishing, if any.
        publishes: Option<u64>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Event {
        Connect,
        Fetch,
        Launch { main: PathBuf, force: bool },
    }

    fn record(event: Event) {
        EVENTS.lock().unwrap_or_else(|e| e.into_inner()).push(event);
    }

    fn events() -> Vec<Event> {
        EVENTS.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn requests() -> usize {
        events().iter().filter(|event| **event == Event::Fetch).count()
    }

    fn launches() -> Vec<Event> {
        events().into_iter().filter(|event| matches!(event, Event::Launch { .. })).collect()
    }

    struct Answer(Result<u64, PrFailure>);

    fn open_pr(number: u64) -> Vec<OpenPullRequest> {
        vec![OpenPullRequest {
            number,
            url: None,
            source_repo: Some("owner/repo".into()),
            source_branch: "feat".into(),
            target_branch: "main".into(),
        }]
    }

    impl OpenPrSource for Answer {
        fn source_repo(&self) -> Option<String> {
            Some("owner/repo".into())
        }
        fn fetch(&self) -> Result<Vec<OpenPullRequest>, PrFailure> {
            record(Event::Fetch);
            self.0.clone().map(open_pr)
        }
    }

    /// A seeding source, which records nothing.
    struct Seed(u64);

    impl OpenPrSource for Seed {
        fn source_repo(&self) -> Option<String> {
            Some("owner/repo".into())
        }
        fn fetch(&self) -> Result<Vec<OpenPullRequest>, PrFailure> {
            Ok(open_pr(self.0))
        }
    }

    fn answering(_origin: &str) -> Box<dyn OpenPrSource> {
        record(Event::Connect);
        Box::new(Answer(Ok(7)))
    }

    fn failing(_origin: &str) -> Box<dyn OpenPrSource> {
        record(Event::Connect);
        Box::new(Answer(Err(PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) })))
    }

    /// A worker that finishes its attempt in sync at once (publishing a PR
    /// answer first when asked) and exits.
    fn finishing_launch(main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        record(Event::Launch { main: main.to_path_buf(), force: args.force });
        let worker = WORKER.lock().unwrap_or_else(|e| e.into_inner()).clone().expect("fixture");
        if let Some(number) = worker.publishes {
            fetch_and_publish(&worker.pr_store, &worker.repo, ORIGIN, unix_now(), &Seed(number))
                .expect("published")
                .expect("same origin");
        }
        let attempt = Attempt::begin(args.attempt.clone(), origin_digest(ORIGIN), "main".into(), unix_now());
        begin_attempt(&worker.head_store, &attempt).expect("attempt");
        finish_attempt(&worker.head_store, &args.attempt, Outcome::InSync, None).expect("outcome");
        Ok(WorkerHandle::new(|| true))
    }

    /// A worker that runs past any budget without recording anything.
    fn silent_launch(main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        record(Event::Launch { main: main.to_path_buf(), force: args.force });
        Ok(WorkerHandle::new(|| false))
    }

    fn failing_launch(main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        record(Event::Launch { main: main.to_path_buf(), force: args.force });
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no wt"))
    }

    struct Fixture {
        repo: tempfile::TempDir,
        _cache: tempfile::TempDir,
        store: PathBuf,
        head_store: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            EVENTS.lock().unwrap_or_else(|e| e.into_inner()).clear();
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
            });
            Self { repo, _cache: cache, store, head_store }
        }

        /// The stub worker also publishes PR `number`.
        fn worker_publishes(&self, number: u64) {
            if let Some(worker) = WORKER.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                worker.publishes = Some(number);
            }
        }

        fn main(&self) -> &Path {
            self.repo.path()
        }

        /// Stores PR `number` for the current origin, fetched `age` seconds ago.
        fn seed(&self, age: u64, number: u64) {
            fetch_and_publish(&self.store, self.main(), ORIGIN, unix_now() - age, &Seed(number))
                .expect("seeded answer")
                .expect("origin is unchanged");
        }

        fn gather_with(
            &self,
            connect: super::super::PrConnect,
            launch: super::super::WorkerLaunch,
            flags: ListFlags,
        ) -> RemoteAnswers {
            let seams = ListSeams { connect, launch, wait_budget: BUDGET, forced_budget: BUDGET };
            let stores = Stores { prs: &self.store, head: &self.head_store };
            gather_remote(stores, self.main(), "main", flags, seams).expect("gathered")
        }

        fn gather(&self, connect: super::super::PrConnect) -> RemoteAnswers {
            self.gather_with(connect, finishing_launch, ListFlags::default())
        }

        fn gather_prs(&self, connect: super::super::PrConnect) -> PrListing {
            self.gather(connect).prs
        }
    }

    /// The git calls `gather` made: a cache hit costs exactly the origin
    /// lookup that binds both stored answers to the current `origin`.
    fn git_calls_of<T>(gather: impl FnOnce() -> T) -> (T, Vec<Vec<String>>) {
        recorder::start_recording();
        let answer = gather();
        (answer, recorder::finish_recording())
    }

    fn origin_lookup() -> Vec<Vec<String>> {
        vec![["remote", "get-url", "origin"].map(String::from).to_vec()]
    }

    fn numbers(listing: &PrListing) -> Vec<u64> {
        listing.pull_requests.iter().map(|pr| pr.number).collect()
    }

    fn outcome(answers: &RemoteAnswers) -> Option<Outcome> {
        match &answers.waited.as_ref().expect("launched").end {
            WaitEnd::Finished { attempt, .. } => attempt.outcome,
            _ => None,
        }
    }

    #[test]
    #[serial_test::serial]
    fn every_listing_with_an_origin_launches_once_and_waits_for_the_outcome() {
        let fixture = Fixture::new();
        fixture.seed(10, 99);

        let (answers, git_calls) = git_calls_of(|| fixture.gather(answering));

        assert_eq!(git_calls, origin_lookup(), "no ls-remote or fetch in the foreground");
        assert_eq!(launches(), [Event::Launch { main: fixture.main().to_path_buf(), force: false }]);
        assert_eq!(requests(), 0, "a fresh PR answer is not requested");
        assert_eq!(outcome(&answers), Some(Outcome::InSync), "the attempt's outcome was waited for");
        assert_eq!(numbers(&answers.prs), [99]);
    }

    #[test]
    #[serial_test::serial]
    fn a_stale_answer_is_shown_and_the_parent_never_writes_it() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, 99);
        let stored = std::fs::read(&fixture.store).expect("store");

        let listing = fixture.gather_prs(answering);

        assert_eq!(numbers(&listing), [99], "the stored badges, not a new answer");
        assert!(listing.is_stale_at(unix_now()), "shown with its age");
        assert_eq!(requests(), 0, "no request in the foreground");
        assert_eq!(launches().len(), 1);
        assert_eq!(std::fs::read(&fixture.store).expect("store"), stored, "the parent writes nothing");
    }

    #[test]
    #[serial_test::serial]
    fn a_stale_empty_answer_is_still_an_answer() {
        let fixture = Fixture::new();
        struct Empty;
        impl OpenPrSource for Empty {
            fn source_repo(&self) -> Option<String> {
                Some("owner/repo".into())
            }
            fn fetch(&self) -> Result<Vec<OpenPullRequest>, PrFailure> {
                Ok(Vec::new())
            }
        }
        fetch_and_publish(&fixture.store, fixture.main(), ORIGIN, unix_now() - 120, &Empty)
            .expect("seeded")
            .expect("origin is unchanged");

        let listing = fixture.gather_prs(answering);

        assert!(listing.pull_requests.is_empty());
        assert!(listing.is_stale_at(unix_now()));
        assert_eq!(requests(), 0, "an empty answer is not a miss");
    }

    #[test]
    #[serial_test::serial]
    fn a_pr_answer_the_worker_publishes_during_the_wait_is_shown() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, 99);
        fixture.worker_publishes(8);

        let listing = fixture.gather_prs(answering);

        assert_eq!(numbers(&listing), [8]);
        assert!(!listing.is_stale_at(unix_now()));
    }

    /// Answers PR 7 only after a forced worker (another listing's `wt -r`)
    /// has stored PR 8 under the refresh lock.
    struct Overtaken;

    impl OpenPrSource for Overtaken {
        fn source_repo(&self) -> Option<String> {
            Some("owner/repo".into())
        }
        fn fetch(&self) -> Result<Vec<OpenPullRequest>, PrFailure> {
            record(Event::Fetch);
            let worker = WORKER.lock().unwrap_or_else(|e| e.into_inner()).clone().expect("fixture");
            let outcome = refresh(&worker.pr_store, &worker.repo, unix_now, true, |_| {
                Box::new(Seed(8)) as Box<dyn OpenPrSource>
            });
            assert_eq!(outcome, RefreshOutcome::Refreshed);
            Ok(open_pr(7))
        }
    }

    fn overtaken(_origin: &str) -> Box<dyn OpenPrSource> {
        record(Event::Connect);
        Box::new(Overtaken)
    }

    /// Review 3: the miss request's older answer must not replace the one a
    /// forced worker published while it was in flight.
    #[test]
    #[serial_test::serial]
    fn a_miss_answer_overtaken_by_a_forced_worker_leaves_the_newer_answer_shown_and_stored() {
        let fixture = Fixture::new();

        let listing = fixture.gather_prs(overtaken);

        assert_eq!(requests(), 1);
        assert_eq!(numbers(&listing), [8], "the listing shows the newer answer");
        let CachedPrs::Fresh(stored) = select_cached(&fixture.store, Some(ORIGIN), unix_now()) else {
            panic!("an answer is stored");
        };
        assert_eq!(numbers(&stored), [8], "and the store keeps it");
    }

    #[test]
    #[serial_test::serial]
    fn a_pr_miss_settles_before_the_worker_is_launched() {
        let fixture = Fixture::new();

        let listing = fixture.gather_prs(answering);

        assert_eq!(numbers(&listing), [7]);
        assert_eq!(
            events(),
            [Event::Connect, Event::Fetch, Event::Launch { main: fixture.main().to_path_buf(), force: false }],
            "the foreground PR request finishes before the worker starts"
        );
        // The next run reads the stored answer without a request.
        assert_eq!(numbers(&fixture.gather_prs(answering)), [7]);
        assert_eq!(requests(), 1);
    }

    #[test]
    #[serial_test::serial]
    fn a_failed_miss_shows_no_badges_stores_nothing_and_is_this_runs_failure() {
        let fixture = Fixture::new();

        let answers = fixture.gather(failing);

        assert_eq!(answers.prs, PrListing::default(), "an unavailable answer has no badges and no age");
        assert_eq!(
            answers.pr_failure,
            Some(PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) }),
            "kept for the credentials line"
        );
        assert_eq!(requests(), 1);
        assert!(!fixture.store.exists(), "a failure is never stored");
        assert_eq!(launches().len(), 1, "the update still runs");
    }

    #[test]
    #[serial_test::serial]
    fn a_forced_listing_leaves_a_pr_miss_to_its_forced_worker() {
        for flags in [
            ListFlags { refresh: true, ..ListFlags::default() },
            ListFlags { fast_forward: true, ..ListFlags::default() },
        ] {
            let fixture = Fixture::new();

            let answers = fixture.gather_with(answering, finishing_launch, flags);

            assert_eq!(
                events(),
                [Event::Launch { main: fixture.main().to_path_buf(), force: true }],
                "no foreground request ({flags:?})"
            );
            assert_eq!(answers.pr_failure, None);
        }
    }

    #[test]
    #[serial_test::serial]
    fn a_changed_origin_never_shows_the_old_answers() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, 99);
        run_git(fixture.main(), &["remote", "set-url", "origin", "https://prs.example.invalid/other/repo.git"]);

        let answers = fixture.gather_with(failing, silent_launch, ListFlags::default());

        assert_eq!(answers.prs, PrListing::default(), "the old answer belongs to another origin");
        assert_eq!(requests(), 1, "a miss for the new origin requests");
        assert_eq!(launches().len(), 1);
    }

    #[test]
    #[serial_test::serial]
    fn a_worker_that_cannot_launch_is_unavailable_without_a_wait() {
        let fixture = Fixture::new();
        fixture.seed(10, 99);

        let started = Instant::now();
        let answers = fixture.gather_with(answering, failing_launch, ListFlags::default());

        assert!(matches!(answers.waited.as_ref().map(|waited| &waited.end), Some(WaitEnd::Unavailable)));
        assert!(started.elapsed() < BUDGET, "{:?}", started.elapsed());
        assert_eq!(numbers(&answers.prs), [99], "stored answers stay shown");
    }

    #[test]
    #[serial_test::serial]
    fn a_silent_worker_is_waited_for_only_until_the_budget() {
        let fixture = Fixture::new();
        fixture.seed(10, 99);

        let started = Instant::now();
        let answers = fixture.gather_with(answering, silent_launch, ListFlags::default());

        let elapsed = started.elapsed();
        assert!(matches!(answers.waited.as_ref().map(|waited| &waited.end), Some(WaitEnd::TimedOut { last: None })));
        assert!(elapsed >= BUDGET && elapsed < BUDGET * 4, "{elapsed:?}");
    }

    #[test]
    #[serial_test::serial]
    fn without_an_origin_stored_answers_are_ignored_and_nothing_is_requested_or_launched() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, 99);
        run_git(fixture.main(), &["remote", "remove", "origin"]);

        let answers = fixture.gather(answering);

        assert_eq!(answers.prs, PrListing::default());
        assert!(answers.origin.is_none());
        assert!(answers.waited.is_none());
        assert!(events().is_empty(), "no connect and no launch: {:?}", events());
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
    use super::super::wait::WaitEnd;
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

    fn finished(outcome: Outcome) -> WaitEnd {
        WaitEnd::Finished { attempt: attempt(Phase::Checking, Some(outcome), None), receipt: None }
    }

    fn note(condition: ApiCondition, key: Option<&str>, fallback_answered: bool) -> Option<ApiNote> {
        Some(ApiNote { condition, key: key.map(str::to_string), fallback_answered })
    }

    #[test]
    fn every_ending_maps_to_its_caption_row() {
        let status = |end: &WaitEnd| remote_status(end, || LAST);
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
        assert_eq!(status(&WaitEnd::Unavailable), RemoteStatus::CheckFailed { reason: CheckFailure::Other, last: LAST });
        assert_eq!(
            status(&WaitEnd::TimedOut { last: Some(attempt(Phase::Fetching, None, None)) }),
            RemoteStatus::StillPulling
        );
        let fallback = Phase::CheckingFallback { reason: FallbackReason::NoKey };
        for last in [None, Some(attempt(Phase::Checking, None, None)), Some(attempt(fallback, None, None))] {
            assert_eq!(status(&WaitEnd::TimedOut { last }), RemoteStatus::StillChecking { last: LAST });
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
        let line = |outcome, api| credential_line(GITHUB, Some(&attempt(Phase::Checking, outcome, api)), None);
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
        let line = |failure: PrFailure| credential_line(GITHUB, None, Some(&failure)).map(|line| line.condition);
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
        // Nothing observed this run, nothing said: a worker's later failure
        // reaches only the next listing's store.
        assert_eq!(credential_line(GITHUB, None, None), None);
        // A remote that is no supported provider has no line at all.
        let rejected = PrFailure::CredentialsRejected { key: None };
        assert_eq!(credential_line("/srv/git/repo.git", None, Some(&rejected)), None);
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
        let line = credential_line(GITHUB, Some(&attempt(Phase::Checking, failed, api)), None).expect("a line");
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
