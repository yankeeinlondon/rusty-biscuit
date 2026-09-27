
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

/// Without an origin there is no stored answer to refresh either.
fn no_launch(main: &Path) {
    panic!("no refresh expected without an origin, got one for {}", main.display())
}

const NO_PRS: super::ListSeams = super::ListSeams {
    connect: no_prs,
    launch: no_launch,
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

/// `gather_remote` against a real repository and stores, with counting seams
/// in place of the PR provider and the background launch.
mod gather {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use worktree::pull_requests::{OpenPrSource, OpenPullRequest, PrListing, fetch_and_publish, origin_digest, unix_now};
    use worktree::remote_head::PrFailure;
    use worktree::remote_head::CachedRemoteHead;

    use super::{recorder, run_git, temp_repo};

    const ORIGIN: &str = "https://prs.example.invalid/owner/repo.git";
    const HEAD_SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    /// Every seam call, in order: `connect`, `fetch` (the request itself),
    /// and `launch`.
    static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Event {
        Connect,
        Fetch,
        Launch(PathBuf),
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

    fn launches() -> Vec<PathBuf> {
        events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Launch(main) => Some(main),
                _ => None,
            })
            .collect()
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

    fn counting_launch(main: &Path) {
        record(Event::Launch(main.to_path_buf()));
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
            Self {
                repo,
                _cache: cache,
                store,
                head_store,
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

        /// Stores a live head of `main` on the current origin, checked `age`
        /// seconds ago. There is no public writer, so this writes the format
        /// `worktree::remote_head` reads.
        fn seed_head(&self, age: u64) {
            let document = serde_json::json!({
                "format_version": 1,
                "origin_digest": origin_digest(ORIGIN),
                "branch": "main",
                "sha": HEAD_SHA,
                "checked_at": unix_now() - age,
            });
            std::fs::write(&self.head_store, document.to_string()).expect("seeded head");
        }

        fn gather(&self, connect: super::super::PrConnect) -> super::super::RemoteAnswers {
            let seams = super::super::ListSeams {
                connect,
                launch: counting_launch,
            };
            let stores = super::super::Stores {
                prs: &self.store,
                head: &self.head_store,
            };
            super::super::gather_remote(stores, self.main(), "main", seams)
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

    #[test]
    #[serial_test::serial]
    fn a_stale_answer_is_shown_at_once_and_refreshed_in_the_background() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, 99);
        fixture.seed_head(10);
        let stored = std::fs::read(&fixture.store).expect("store");

        let (listing, git_calls) = git_calls_of(|| fixture.gather_prs(answering));

        assert_eq!(git_calls, origin_lookup());
        assert_eq!(numbers(&listing), [99], "the stored badges, not a new answer");
        assert!(listing.is_stale_at(unix_now()), "shown with its age");
        assert_eq!(requests(), 0, "no request in the foreground");
        assert_eq!(launches(), [fixture.main().to_path_buf()], "one refresh, for the main checkout");
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
        fixture.seed_head(10);

        let listing = fixture.gather_prs(answering);

        assert!(listing.pull_requests.is_empty());
        assert!(listing.is_stale_at(unix_now()));
        assert_eq!(requests(), 0, "an empty answer is not a miss");
        assert_eq!(launches().len(), 1);
    }

    #[test]
    #[serial_test::serial]
    fn two_fresh_answers_neither_request_nor_refresh() {
        let fixture = Fixture::new();
        fixture.seed(10, 99);
        fixture.seed_head(10);

        let (answers, git_calls) = git_calls_of(|| fixture.gather(answering));

        assert_eq!(git_calls, origin_lookup(), "one local git call, no request");
        assert_eq!(numbers(&answers.prs), [99]);
        assert!(!answers.prs.is_stale_at(unix_now()));
        assert!(matches!(answers.head, Some(CachedRemoteHead::Fresh(_))), "{:?}", answers.head);
        assert!(answers.origin_present);
        assert!(events().is_empty(), "{:?}", events());
    }

    #[test]
    #[serial_test::serial]
    fn a_stale_pr_answer_and_a_missing_head_launch_exactly_once() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, 99);

        let answers = fixture.gather(answering);

        assert_eq!(answers.head, Some(CachedRemoteHead::Miss));
        assert_eq!(events(), [Event::Launch(fixture.main().to_path_buf())], "one launch serves both halves");
    }

    #[test]
    #[serial_test::serial]
    fn a_missing_or_stale_head_launches_without_any_foreground_request() {
        for head_age in [None, Some(120)] {
            let fixture = Fixture::new();
            fixture.seed(10, 99);
            if let Some(age) = head_age {
                fixture.seed_head(age);
            }

            let (answers, git_calls) = git_calls_of(|| fixture.gather(answering));

            assert_eq!(git_calls, origin_lookup(), "no ls-remote in the foreground ({head_age:?})");
            assert_eq!(
                events(),
                [Event::Launch(fixture.main().to_path_buf())],
                "no connect, one launch ({head_age:?})"
            );
            match head_age {
                None => assert_eq!(answers.head, Some(CachedRemoteHead::Miss)),
                Some(_) => assert!(matches!(answers.head, Some(CachedRemoteHead::Stale(_))), "{:?}", answers.head),
            }
        }
    }

    #[test]
    #[serial_test::serial]
    fn a_pr_miss_with_a_fresh_head_requests_in_the_foreground_and_launches_nothing() {
        let fixture = Fixture::new();
        fixture.seed_head(10);

        let listing = fixture.gather_prs(answering);

        assert_eq!(numbers(&listing), [7]);
        assert_eq!(events(), [Event::Connect, Event::Fetch], "a PR miss alone never starts a worker");
        // The next run reads the stored answer without a request.
        assert_eq!(numbers(&fixture.gather_prs(answering)), [7]);
        assert_eq!(requests(), 1);
    }

    #[test]
    #[serial_test::serial]
    fn a_pr_miss_settles_before_the_worker_is_launched() {
        for head_age in [None, Some(120)] {
            let fixture = Fixture::new();
            if let Some(age) = head_age {
                fixture.seed_head(age);
            }

            let listing = fixture.gather_prs(answering);

            assert_eq!(numbers(&listing), [7]);
            assert_eq!(
                events(),
                [Event::Connect, Event::Fetch, Event::Launch(fixture.main().to_path_buf())],
                "the foreground PR request finishes before the worker starts ({head_age:?})"
            );
        }
    }

    #[test]
    #[serial_test::serial]
    fn a_failed_miss_shows_no_badges_and_stores_nothing() {
        let fixture = Fixture::new();
        fixture.seed_head(10);

        let listing = fixture.gather_prs(failing);

        assert_eq!(listing, PrListing::default(), "an unavailable answer has no badges and no age");
        assert_eq!(requests(), 1);
        assert!(!fixture.store.exists(), "a failure is never stored");
        assert!(launches().is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn a_changed_origin_never_shows_the_old_answers() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, 99);
        fixture.seed_head(10);
        run_git(fixture.main(), &["remote", "set-url", "origin", "https://prs.example.invalid/other/repo.git"]);

        let answers = fixture.gather(failing);

        assert_eq!(answers.prs, PrListing::default(), "the old answer belongs to another origin");
        assert_eq!(answers.head, Some(CachedRemoteHead::Miss), "so does the old live head");
        assert_eq!(requests(), 1, "a miss for the new origin requests");
        assert_eq!(launches().len(), 1, "the new origin's live head is due");
    }

    #[test]
    #[serial_test::serial]
    fn without_an_origin_stored_answers_are_ignored_and_nothing_is_requested_or_launched() {
        let fixture = Fixture::new();
        fixture.seed(12 * 60, 99);
        fixture.seed_head(120);
        run_git(fixture.main(), &["remote", "remove", "origin"]);

        let answers = fixture.gather(answering);

        assert_eq!(answers.prs, PrListing::default());
        assert_eq!(answers.head, None);
        assert!(!answers.origin_present);
        assert!(events().is_empty(), "no connect and no launch: {:?}", events());
    }
}
