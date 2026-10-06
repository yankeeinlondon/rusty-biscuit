//! `gather_listing` against real repositories, with a scripted worker in
//! place of `wt internal-refresh`. The script runs at launch, on the
//! listing's calling thread inside the wait, so holding it holds the worker's
//! outcome: nothing is recorded until it returns.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use biscuit_terminal::discovery::detection::ImageSupport;
use worktree::cache::{CACHE_FORMAT_VERSION, Cache, CacheKey, cache_path};
use worktree::copy_record::{CopyRecord, record_path, write_atomic};
use worktree::fast_forward::{FfRefusal, FfResult};
use worktree::fork_origin::{ForkOrigin, ForkOriginStore, fork_origin_path, record};
use worktree::git::recorder;
use worktree::listing::CaptionState;
use worktree::pull_requests::{origin_digest, unix_now};
use worktree::remote_head::{
    Attempt, HeadStatus, Outcome, PrStatus, Receipt, begin_attempt, finish_attempt, receipt_path_beside,
    remote_head_store_path, write_receipt,
};
use worktree::worktree::{DirtyStatus, parse_worktree_state};

use super::overlap::{self, Mode};
use super::{DirGuard, assert_perf_reconciles, perf_group, perf_shape, run_git};
use crate::commands::git_graph::{self, GatherInput, VerboseData};
use crate::commands::list::wait::HeadEnd;
use crate::commands::list::{LaunchArgs, ListFlags, ListSeams, Listing, WorkerHandle, gather_listing, remote_status};
use crate::commands::list_table::{LastKnown, RemoteStatus};
use crate::perf::PerfCollector;

const ORIGIN: &str = "https://prs.example.invalid/owner/repo.git";
/// Long enough for any scripted worker that finishes.
const BUDGET: Duration = Duration::from_secs(30);

/// When the scripted worker acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hold {
    Nothing,
    /// Until both local gathers have started.
    BothStarted,
    /// Until both local gathers have finished.
    BothFinished,
}

/// What the scripted worker does at launch.
#[derive(Clone, Debug)]
struct Script {
    hold: Hold,
    /// Git commands run in the main checkout once released, as a fetch moves
    /// refs.
    moves: Vec<Vec<String>>,
    /// Records a finished attempt and a receipt; otherwise the worker runs
    /// past any budget without recording anything.
    finishes: bool,
    /// A copy record whose presence is observed while held.
    copy_record: Option<PathBuf>,
}

impl Script {
    fn finishing() -> Self {
        Self { hold: Hold::Nothing, moves: Vec::new(), finishes: true, copy_record: None }
    }

    fn held(hold: Hold) -> Self {
        Self { hold, ..Self::finishing() }
    }

    fn moving(mut self, moves: &[&[&str]]) -> Self {
        self.moves = moves.iter().map(|args| args.iter().map(|arg| arg.to_string()).collect()).collect();
        self
    }
}

/// The persistent state as the held worker saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Persisted {
    cache_saved: bool,
    gone_fork_record: bool,
    copy_record: bool,
}

/// What the scripted worker observed.
#[derive(Debug, Clone)]
struct Observed {
    /// The hold was released by its event, not by the seam's timeout.
    released_by_event: bool,
    persisted: Persisted,
}

static SCRIPT: Mutex<Option<Script>> = Mutex::new(None);
static OBSERVED: Mutex<Option<Observed>> = Mutex::new(None);

fn observed() -> Observed {
    OBSERVED.lock().unwrap_or_else(|e| e.into_inner()).clone().expect("the worker was launched")
}

fn scripted_launch(main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
    let script = SCRIPT.lock().unwrap_or_else(|e| e.into_inner()).clone().expect("a script");
    let released_by_event = match script.hold {
        Hold::Nothing => true,
        Hold::BothStarted => overlap::await_both_started(),
        Hold::BothFinished => overlap::await_both_finished(),
    };
    let persisted = Persisted {
        cache_saved: cache_path(main).is_ok_and(|path| path.exists()),
        gone_fork_record: ForkOriginStore::load_from(&fork_origin_path(main).unwrap()).get("gone").is_some(),
        copy_record: script.copy_record.as_ref().is_some_and(|path| path.exists()),
    };
    *OBSERVED.lock().unwrap_or_else(|e| e.into_inner()) = Some(Observed { released_by_event, persisted });
    for args in &script.moves {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        run_git(main, &args);
    }
    if !script.finishes {
        return Ok(WorkerHandle::new(|| false));
    }
    let head_store = remote_head_store_path(main).expect("head store");
    let attempt = Attempt::begin(args.attempt.clone(), origin_digest(ORIGIN), "main".into(), unix_now());
    begin_attempt(&head_store, &attempt).expect("attempt");
    let outcome = if script.moves.is_empty() { Outcome::InSync } else { Outcome::Fetched };
    finish_attempt(&head_store, &args.attempt, outcome, None).expect("outcome");
    let receipt = Receipt {
        attempt_id: args.attempt.clone(),
        origin_digest: origin_digest(ORIGIN),
        branch: "main".into(),
        finished_at: unix_now(),
        head: HeadStatus::Ok,
        prs: PrStatus::Unsupported,
    };
    write_receipt(&receipt_path_beside(&head_store, &args.attempt).expect("path"), &receipt).expect("receipt");
    Ok(WorkerHandle::new(|| true))
}

fn git_out(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git").current_dir(repo).args(args).output().expect("git should be installed");
    assert!(output.status.success(), "git {args:?} failed in {repo:?}");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// `main: c1 - c2` checked out in `main/`, `feature-a: c2 - a1` in
/// `feature/`, an `origin` that no test reaches, and `origin/main` at `c2`.
/// Every per-repository file in the user cache is removed on drop.
struct Repo {
    _dir: tempfile::TempDir,
    main: PathBuf,
    feature: PathBuf,
    c1: String,
    c2: String,
    cache_dir: PathBuf,
    cache_prefix: String,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let main = dir.path().join("main");
        let feature = dir.path().join("feature");
        fs::create_dir(&main).unwrap();
        run_git(&main, &["init", "-q", "-b", "main"]);
        for (key, value) in [
            ("user.email", "test@example.com"),
            ("user.name", "Test User"),
            ("commit.gpgsign", "false"),
            ("tag.gpgsign", "false"),
            ("gc.auto", "0"),
            ("core.fsmonitor", "false"),
            ("core.commitGraph", "false"),
        ] {
            run_git(&main, &["config", key, value]);
        }
        let commit = |file: &str| {
            fs::write(main.join(file), format!("{file}\n")).unwrap();
            run_git(&main, &["add", "--", file]);
            run_git(&main, &["commit", "-q", "-m", file]);
            git_out(&main, &["rev-parse", "HEAD"])
        };
        let c1 = commit("c1.txt");
        let c2 = commit("c2.txt");
        run_git(&main, &["branch", "feature-a"]);
        run_git(&main, &["worktree", "add", "-q", feature.to_str().unwrap(), "feature-a"]);
        fs::write(feature.join("a1.txt"), "a1\n").unwrap();
        run_git(&feature, &["add", "a1.txt"]);
        run_git(&feature, &["commit", "-q", "-m", "a1"]);
        run_git(&main, &["remote", "add", "origin", ORIGIN]);
        run_git(&main, &["update-ref", "refs/remotes/origin/main", &c2]);

        let cache = cache_path(&main).expect("cache path");
        let cache_dir = cache.parent().unwrap().to_path_buf();
        let cache_prefix = format!("{}.", cache.file_stem().unwrap().to_string_lossy());
        let repo = Self { _dir: dir, main, feature, c1, c2, cache_dir, cache_prefix };
        repo.clear_cache();
        repo
    }

    fn clear_cache(&self) {
        let Ok(entries) = fs::read_dir(&self.cache_dir) else { return };
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&self.cache_prefix) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    fn sha(&self, rev: &str) -> String {
        git_out(&self.main, &["rev-parse", rev])
    }

    /// A commit on `parent` that no ref names yet, as a fetch would bring.
    fn upstream_commit(&self, parent: &str) -> String {
        git_out(&self.main, &["commit-tree", &format!("{parent}^{{tree}}"), "-p", parent, "-m", "upstream"])
    }

    /// A fork record for a branch that does not exist, which a commit prunes.
    fn record_gone_branch(&self) {
        let origin = ForkOrigin { base_branch: "main".into(), base_sha: self.c2.clone(), created_at: 4 };
        record(&fork_origin_path(&self.main).unwrap(), "gone", origin).unwrap();
    }

    /// A copy record for a worktree that no longer exists, which a commit
    /// prunes.
    fn stale_copy_record(&self) -> PathBuf {
        let gone = self.main.with_file_name("removed-worktree");
        let record = CopyRecord {
            format_version: 1,
            worktree: gone.clone(),
            admin_dir: gone.join("admin"),
            registration: "nonce".into(),
            source: self.main.clone(),
            source_label: "main".into(),
            files: Vec::new(),
        };
        let path = record_path(&self.main, &gone).unwrap();
        write_atomic(&path, &record).unwrap();
        path
    }

    fn cached(&self, target: &str, branch: &str) -> bool {
        let key = CacheKey { target_tip_sha: target.into(), branch_tip_sha: branch.into(), version: CACHE_FORMAT_VERSION };
        Cache::load_or_default_from(&cache_path(&self.main).unwrap()).get(&key).is_some()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        self.clear_cache();
    }
}

/// One listing's accepted results, its git calls, and its perf stage names.
struct Run {
    listing: Listing,
    calls: Vec<Vec<String>>,
    stages: Vec<&'static str>,
    perf: PerfCollector,
}

impl Run {
    fn count(&self, matches: impl Fn(&[String]) -> bool) -> usize {
        recorder::count_matching(&self.calls, matches)
    }

    /// `git status` walks: dirtiness measurements.
    fn status_walks(&self) -> usize {
        self.count(|args| args.iter().any(|arg| arg == "status"))
    }

    fn ref_reads(&self) -> usize {
        self.count(|args| args.first().map(String::as_str) == Some("for-each-ref"))
    }

    fn merge_trees(&self) -> usize {
        self.count(|args| args.first().map(String::as_str) == Some("merge-tree"))
    }

    fn regathered(&self) -> bool {
        self.stages.contains(&"regather")
    }
}

fn run(script: Script, flags: ListFlags, image: ImageSupport, verbose: bool, budget: Duration) -> Run {
    *SCRIPT.lock().unwrap_or_else(|e| e.into_inner()) = Some(script);
    *OBSERVED.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let seams = ListSeams { launch: scripted_launch, wait_budget: budget, forced_budget: budget };
    let mut collector = Some(PerfCollector::new(Instant::now()));
    recorder::start_recording();
    let listing = gather_listing(verbose, flags, image, seams, &mut collector);
    let calls = recorder::finish_recording();
    let listing = listing.expect("gather_listing");
    let perf = collector.expect("collector");
    let stages = perf.recorded_stages().iter().map(|(name, _)| *name).collect();
    Run { listing, calls, stages, perf }
}

type Details = Option<(Option<(String, String)>, Vec<(String, String)>)>;

fn details(verbose: &Option<VerboseData>) -> Details {
    let pair = |commit: &git_graph::CommitDetail| (commit.short_sha.clone(), commit.refs.clone());
    verbose.as_ref().map(|data| (data.merge_base.as_ref().map(pair), data.branch_commits.iter().map(pair).collect()))
}

/// Everything the listing shows equals a listing gathered from scratch now:
/// caption, target, tree, counts, dirtiness, graph, and verbose history all
/// describe the final tips.
fn assert_describes_the_final_state(run: &Run, image: ImageSupport, verbose: bool, context: &str) {
    let fresh = parse_worktree_state().expect("parse");
    let cache = fresh.load_comparison_cache();
    let (dirty, facts) = fresh.gather_local(fresh.refs(), &cache);
    let list = &run.listing.list;
    assert_eq!(list.refs(), fresh.refs(), "{context}: the accepted snapshot is the final one");
    assert_eq!(list.caption, facts.caption, "{context}: caption");
    assert_eq!(list.target, facts.target, "{context}: target");
    assert_eq!(list.tree, facts.tree, "{context}: tree");
    assert_eq!(list.comparisons, facts.comparisons, "{context}: counts");
    assert_eq!(list.statuses.iter().map(|status| status.dirty).collect::<Vec<_>>(), dirty, "{context}: dirtiness");
    let input = GatherInput::from_list(&fresh, fresh.refs());
    let (graph, verbose_data) = git_graph::gather(&input, image != ImageSupport::None, verbose && input.has_verbose());
    assert_eq!(run.listing.graph, graph, "{context}: graph");
    assert_eq!(details(&run.listing.verbose), details(&verbose_data), "{context}: verbose");
}

fn caption_state(run: &Run) -> Option<CaptionState> {
    run.listing.list.caption.as_ref().map(|caption| caption.state())
}

#[test]
#[serial_test::serial]
fn the_local_gathers_start_while_the_worker_outcome_is_held() {
    let repo = Repo::new();
    let _dir = DirGuard::enter(&repo.main);
    let rendezvous = overlap::Installed::new();

    let run = run(Script::held(Hold::BothStarted), ListFlags::default(), ImageSupport::Kitty, false, BUDGET);

    assert!(observed().released_by_event, "the worker's outcome was held until both local gathers started");
    assert_eq!(rendezvous.outcome(), (true, true), "and the list and graph gathers still overlap each other");
    let waited = run.listing.remote.waited.as_ref().expect("launched");
    assert!(matches!(waited.head, HeadEnd::Finished(_)) && !waited.timed_out, "{waited:?}");
}

#[test]
#[serial_test::serial]
fn unchanged_tips_accept_the_first_gather_and_measure_everything_once() {
    let repo = Repo::new();
    let _dir = DirGuard::enter(&repo.main);

    let run = run(Script::finishing(), ListFlags::default(), ImageSupport::Kitty, false, BUDGET);

    assert!(!run.regathered(), "{:?}", run.stages);
    let group = ["remote wait", "pr reread", "list gather", "graph gather"].map(String::from).to_vec();
    assert_eq!(
        perf_shape(&run.perf),
        [("pr gather".into(), vec![]), ("remote wait ‖ local gather".into(), group), ("unattributed".into(), vec![])],
        "one measured group holds the overlapping work"
    );
    assert_perf_reconciles(&run.perf);
    assert_eq!(run.ref_reads(), 2, "the parse step's read and the final one: {:?}", run.calls);
    assert_eq!(run.status_walks(), 2, "one per worktree: {:?}", run.calls);
    assert_describes_the_final_state(&run, ImageSupport::Kitty, false, "unchanged");
    // A single cold gather of the same state makes as many comparisons.
    repo.clear_cache();
    recorder::start_recording();
    let _ = worktree::worktree::list_worktrees().expect("list");
    let single = recorder::finish_recording();
    let single_merge_trees =
        recorder::count_matching(&single, |args| args.first().map(String::as_str) == Some("merge-tree"));
    assert!(single_merge_trees > 0);
    assert_eq!(run.merge_trees(), single_merge_trees, "{:?}", run.calls);
}

#[test]
#[serial_test::serial]
fn without_an_origin_or_an_image_the_single_gather_is_accepted() {
    // (label, removes origin, image support, top-level shape)
    let no_origin = [("pr gather", vec![]), ("local gather", vec!["list gather", "graph gather"]), ("unattributed", vec![])];
    let no_image = [
        ("pr gather", vec![]),
        ("remote wait ‖ local gather", vec!["remote wait", "pr reread", "list gather"]),
        ("unattributed", vec![]),
    ];
    for (label, removes_origin, image, shape, ref_reads) in
        [("no origin", true, ImageSupport::Kitty, no_origin, 1), ("no image", false, ImageSupport::None, no_image, 2)]
    {
        let repo = Repo::new();
        if removes_origin {
            run_git(&repo.main, &["remote", "remove", "origin"]);
        }
        let _dir = DirGuard::enter(&repo.main);

        let run = run(Script::finishing(), ListFlags::default(), image.clone(), false, BUDGET);

        let launched = OBSERVED.lock().unwrap_or_else(|e| e.into_inner()).is_some();
        assert_eq!(launched, !removes_origin, "{label}: a worker is launched only with an origin");
        assert_eq!(run.listing.remote.waited.is_some(), !removes_origin, "{label}");
        let shape: Vec<(String, Vec<String>)> = shape
            .into_iter()
            .map(|(row, children)| (row.to_string(), children.into_iter().map(String::from).collect()))
            .collect();
        assert_eq!(perf_shape(&run.perf), shape, "{label}");
        assert_perf_reconciles(&run.perf);
        assert!(!run.regathered(), "{label}: {:?}", run.stages);
        assert_eq!(run.ref_reads(), ref_reads, "{label}: no second read without a wait: {:?}", run.calls);
        assert_eq!(run.status_walks(), 2, "{label}: {:?}", run.calls);
        assert_eq!(run.listing.graph.is_some(), image != ImageSupport::None, "{label}");
        // Removing `origin` removes its tracking refs, and with them the caption.
        let caption = (!removes_origin).then_some(CaptionState::InSync);
        assert_eq!(caption_state(&run), caption, "{label}");
        assert_describes_the_final_state(&run, image, false, label);
    }
}

#[test]
#[serial_test::serial]
fn a_failed_preference_write_starts_no_local_gather_and_launches_nothing() {
    let repo = Repo::new();
    repo.record_gone_branch();
    run_git(&repo.main, &["remote", "set-url", "origin", "/srv/git/repo.git"]);
    let _dir = DirGuard::enter(&repo.main);
    let seam = overlap::Installed::with_mode(Mode::Observe);
    *SCRIPT.lock().unwrap_or_else(|e| e.into_inner()) = Some(Script::finishing());
    *OBSERVED.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let seams = ListSeams { launch: scripted_launch, wait_budget: BUDGET, forced_budget: BUDGET };
    let flags = ListFlags { ignore_api: true, ..ListFlags::default() };

    let result = gather_listing(false, flags, ImageSupport::Kitty, seams, &mut None);

    assert!(
        matches!(result, Err(worktree::error::WorktreeError::NoRepositoryIdentity(_))),
        "a local-path origin cannot be recorded: {:?}",
        result.err()
    );
    assert!(OBSERVED.lock().unwrap_or_else(|e| e.into_inner()).is_none(), "no worker was launched");
    assert_eq!(seam.started(), (false, false), "no speculative local gather started");
    assert!(!cache_path(&repo.main).unwrap().exists(), "no comparison cache was saved");
    assert!(
        ForkOriginStore::load_from(&fork_origin_path(&repo.main).unwrap()).get("gone").is_some(),
        "nothing was pruned"
    );
}

#[test]
#[serial_test::serial]
fn a_ref_change_during_the_wait_is_regathered_from_the_final_tips() {
    type Moves = fn(&Repo) -> Vec<Vec<String>>;
    type Check = fn(&Run, &Repo);
    let cases: [(&str, Moves, Check); 4] = [
        (
            "advance",
            |repo| vec![args(&["update-ref", "refs/remotes/origin/main", &repo.upstream_commit("main")])],
            |run, _| assert_eq!(caption_state(run), Some(CaptionState::Behind(1))),
        ),
        (
            "rewind",
            |repo| vec![args(&["update-ref", "refs/remotes/origin/main", &repo.c1])],
            |run, _| assert_eq!(caption_state(run), Some(CaptionState::Ahead(1))),
        ),
        (
            "addition",
            |repo| vec![args(&["update-ref", "refs/remotes/origin/feature-a", &repo.sha("feature-a")])],
            |run, _| {
                let (_, commits) = details(&run.listing.verbose).expect("verbose");
                assert_eq!(commits.last().map(|(_, refs)| refs.as_str()), Some("HEAD -> feature-a, origin/feature-a"));
            },
        ),
        (
            "deletion",
            |_| vec![args(&["update-ref", "-d", "refs/remotes/origin/main"])],
            |run, repo| {
                assert_eq!(caption_state(run), None, "no tracking ref, no caption");
                let target = run.listing.list.target.as_ref().expect("target");
                assert_eq!((target.reference.as_str(), target.sha.as_str()), ("main", repo.c2.as_str()));
            },
        ),
    ];
    for (label, moves, check) in cases {
        let repo = Repo::new();
        let _dir = DirGuard::enter(&repo.feature);
        let _seam = overlap::Installed::with_mode(Mode::Observe);
        let moves = moves(&repo);
        let mut script = Script::held(Hold::BothFinished);
        script.moves = moves;

        let run = run(script, ListFlags::default(), ImageSupport::Kitty, true, BUDGET);

        assert!(observed().released_by_event, "{label}: the first gather finished before refs moved");
        assert!(run.regathered(), "{label}: {:?}", run.stages);
        assert_eq!(perf_group(&run.perf, "regather"), ["list regather", "graph regather"], "{label}");
        assert_perf_reconciles(&run.perf);
        assert_eq!(run.status_walks(), 2, "{label}: a fetch never measures dirtiness again: {:?}", run.calls);
        assert_describes_the_final_state(&run, ImageSupport::Kitty, true, label);
        check(&run, &repo);
    }
}

fn args(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
}

#[test]
#[serial_test::serial]
fn fast_forward_measures_again_only_the_checkout_it_moved() {
    let fast_forward = ListFlags { fast_forward: true, ..ListFlags::default() };
    type Setup = fn(&Repo);
    type Expected = fn(&FfResult) -> bool;
    let cases: [(&str, Setup, Expected, usize); 4] = [
        (
            "moved with a holder",
            |repo| run_git(&repo.main, &["update-ref", "refs/remotes/origin/main", &repo.upstream_commit("main")]),
            |ff| matches!(ff, FfResult::Moved { checkout: Some(_), .. }),
            3,
        ),
        (
            "moved without a holder",
            |repo| {
                run_git(&repo.main, &["update-ref", "refs/remotes/origin/main", &repo.upstream_commit("main")]);
                run_git(&repo.main, &["checkout", "-q", "-b", "elsewhere"]);
            },
            |ff| matches!(ff, FfResult::Moved { checkout: None, .. }),
            2,
        ),
        ("up to date", |_| {}, |ff| matches!(ff, FfResult::UpToDate), 2),
        (
            "refused",
            |repo| run_git(&repo.main, &["update-ref", "refs/remotes/origin/main", &repo.upstream_commit(&repo.c1)]),
            |ff| matches!(ff, FfResult::Refused(FfRefusal::Diverged)),
            2,
        ),
    ];
    for (label, setup, expected, walks) in cases {
        let repo = Repo::new();
        setup(&repo);
        let _dir = DirGuard::enter(&repo.main);

        let run = run(Script::finishing(), fast_forward, ImageSupport::Kitty, false, BUDGET);

        let ff = run.listing.ff.as_ref().expect("--ff ran");
        assert!(expected(ff), "{label}: {ff:?}");
        assert_eq!(run.status_walks(), walks, "{label}: {:?}", run.calls);
        assert_eq!(run.stages.contains(&"checkout status refresh"), walks == 3, "{label}: {:?}", run.stages);
        assert_eq!(run.regathered(), matches!(ff, FfResult::Moved { .. }), "{label}: {:?}", run.stages);
        assert!(run.stages.contains(&"fast-forward"), "{label}: {:?}", run.stages);
        assert_perf_reconciles(&run.perf);
        assert_describes_the_final_state(&run, ImageSupport::Kitty, false, label);
    }
}

#[test]
#[serial_test::serial]
fn a_failed_ref_read_never_counts_as_unchanged_and_a_failed_final_read_never_prunes() {
    for (failing_read, label) in [(1, "initial"), (2, "final")] {
        let repo = Repo::new();
        repo.record_gone_branch();
        let _dir = DirGuard::enter(&repo.main);
        let reads = AtomicUsize::new(0);
        let _failure = recorder::fail_matching(move |args| {
            args.first() == Some(&"for-each-ref") && reads.fetch_add(1, Ordering::SeqCst) + 1 == failing_read
        });

        let run = run(Script::finishing(), ListFlags::default(), ImageSupport::Kitty, false, BUDGET);
        drop(_failure);

        assert!(run.regathered(), "{label}: a failed read establishes nothing: {:?}", run.stages);
        let pruned = ForkOriginStore::load_from(&fork_origin_path(&repo.main).unwrap()).get("gone").is_none();
        if failing_read == 1 {
            assert!(run.listing.list.ref_snapshot().succeeded(), "{label}");
            assert_eq!(caption_state(&run), Some(CaptionState::InSync), "{label}: the final read's facts");
            assert!(pruned, "{label}: the final read succeeded");
            assert_describes_the_final_state(&run, ImageSupport::Kitty, false, label);
        } else {
            assert!(!run.listing.list.ref_snapshot().succeeded(), "{label}");
            assert_eq!(caption_state(&run), None, "{label}: the degraded listing, as before");
            assert!(!pruned, "{label}: an empty failed read would make every record look deleted");
        }
    }
}

#[test]
#[serial_test::serial]
fn nothing_persists_before_the_accepted_gather_is_committed_once() {
    let repo = Repo::new();
    repo.record_gone_branch();
    let copy_record = repo.stale_copy_record();
    let _dir = DirGuard::enter(&repo.main);
    let _seam = overlap::Installed::with_mode(Mode::Observe);
    let initial_origin = repo.sha("origin/main");
    let upstream = repo.upstream_commit("main");
    let mut script = Script::held(Hold::BothFinished).moving(&[&["update-ref", "refs/remotes/origin/main", &upstream]]);
    script.copy_record = Some(copy_record.clone());

    let run = run(script, ListFlags::default(), ImageSupport::Kitty, false, BUDGET);

    let held = observed();
    assert!(held.released_by_event, "observed after the first gather finished");
    assert_eq!(
        held.persisted,
        Persisted { cache_saved: false, gone_fork_record: true, copy_record: true },
        "the speculative gather saved and pruned nothing"
    );
    assert!(run.regathered());
    assert!(ForkOriginStore::load_from(&fork_origin_path(&repo.main).unwrap()).get("gone").is_none(), "pruned once accepted");
    assert!(!copy_record.exists(), "pruned once accepted");
    let feature = repo.sha("feature-a");
    assert!(repo.cached(&initial_origin, &feature), "the discarded gather's comparison is kept");
    assert!(repo.cached(&upstream, &feature), "and the final gather's");
}

#[test]
#[serial_test::serial]
fn a_timed_out_wait_keeps_pending_status_and_lets_a_longer_local_gather_finish() {
    for (moves, label) in [(false, "tips unchanged"), (true, "a fetch landed before the final read")] {
        let repo = Repo::new();
        let _dir = DirGuard::enter(&repo.main);
        let seam = overlap::Installed::with_mode(Mode::HoldListUntilRemote);
        let mut script = Script { finishes: false, ..Script::finishing() };
        if moves {
            script = script.moving(&[&["update-ref", "refs/remotes/origin/main", &repo.upstream_commit("main")]]);
        }

        let run = run(script, ListFlags::default(), ImageSupport::None, false, Duration::from_millis(100));

        let waited = run.listing.remote.waited.as_ref().expect("launched");
        assert!(waited.timed_out, "{label}");
        assert_eq!(waited.head, HeadEnd::Running { last: None }, "{label}: the detached worker is not joined");
        assert!(
            matches!(remote_status(&waited.head, || LastKnown::Never), RemoteStatus::StillChecking { .. }),
            "{label}: the caption still says it is checking"
        );
        assert!(seam.list_started_after_the_wait(), "{label}: the local gather outlasted the wait");
        assert_eq!(perf_group(&run.perf, "remote wait ‖ local gather"), ["remote wait", "pr reread", "list gather"]);
        let tree = run.perf.build_perf_tree();
        let group = tree.children.iter().find(|row| row.label == "remote wait ‖ local gather").expect("group");
        let child = |name: &str| group.children.iter().find(|row| row.label == name).expect(name).total;
        assert!(
            group.total >= child("remote wait") + child("list gather"),
            "{label}: the group is its measured span, not its longest child: {group:#?}"
        );
        assert_perf_reconciles(&run.perf);
        assert_eq!(run.status_walks(), 2, "{label}: {:?}", run.calls);
        assert_eq!(run.regathered(), moves, "{label}: {:?}", run.stages);
        let expected = if moves { CaptionState::Behind(1) } else { CaptionState::InSync };
        assert_eq!(caption_state(&run), Some(expected), "{label}");
        assert_eq!(
            run.listing.list.statuses.iter().map(|status| status.dirty).collect::<Vec<_>>(),
            [DirtyStatus::Clean, DirtyStatus::Clean],
            "{label}"
        );
    }
}
