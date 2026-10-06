//! [`gather`] against real repositories, with a scripted worker in place of
//! `wt internal-refresh`. The script runs at launch, on the
//! listing's calling thread inside the wait, so holding it holds the worker's
//! outcome: nothing is recorded until it returns.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::cache::{CACHE_FORMAT_VERSION, Cache, CacheKey, cache_path};
use crate::copy_record::{CopyRecord, record_path, write_atomic};
use crate::fast_forward::{FfRefusal, FfResult};
use crate::fork_origin::{ForkOrigin, ForkOriginStore, fork_origin_path, record};
use crate::git::{calls, recorder};
use crate::graph::{self, CommitDetail, GatherInput, VerboseData};
use crate::list::wait::{HeadEnd, LaunchArgs, WorkerHandle};
use crate::list::{ListOptions, Listing, WaitProgress, gather};
use crate::listing::CaptionState;
use crate::pull_requests::{origin_digest, unix_now};
use crate::remote_head::{
    Attempt, HeadStatus, Outcome, PrStatus, Receipt, begin_attempt, finish_attempt, receipt_path_beside,
    remote_head_store_path, write_receipt,
};
use crate::worktree::{DirtyStatus, fill_worktree_statuses, parse_worktree_state_in};

use super::overlap::{self, Mode};
use super::run_git;

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
        // As the real worker: only an attempt asked for timings measures itself.
        durations: if args.timings {
            crate::timing::LaunchReport::from_worker(crate::timing::worker_fixture(&[
                crate::timing::Stage::PrRefresh,
                crate::timing::Stage::HeadRefresh,
            ]))
        } else {
            crate::timing::LaunchReport::Missing
        },
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
        crate::test_support::configure(
            &main,
            &[
                ("user.email", "test@example.com"),
                ("user.name", "Test User"),
                ("commit.gpgsign", "false"),
                ("tag.gpgsign", "false"),
                ("gc.auto", "0"),
                ("core.fsmonitor", "false"),
                ("core.commitGraph", "false"),
                // What `git remote add origin <ORIGIN>` writes.
                ("remote.origin.url", ORIGIN),
                ("remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"),
            ],
        );
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

/// One listing's accepted results and its git calls.
struct Run {
    listing: Listing,
    calls: Vec<Vec<String>>,
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
        self.listing.regathered
    }
}

/// A listing with the scripted worker: the graph when `graph`, verbose
/// details when `verbose`, and `budget` for both waits.
fn options(graph: bool, verbose: bool, budget: Duration) -> ListOptions {
    ListOptions {
        worker: Some(scripted_launch),
        wait_budget: budget,
        forced_budget: budget,
        graph,
        verbose,
        ..ListOptions::default()
    }
}

/// [`gather`] from `at` with `script` as the worker.
fn run(at: &Path, script: Script, options: ListOptions) -> Run {
    *SCRIPT.lock().unwrap_or_else(|e| e.into_inner()) = Some(script);
    *OBSERVED.lock().unwrap_or_else(|e| e.into_inner()) = None;
    recorder::start_recording();
    let listing = gather(at, &options, &mut |_| {});
    let calls = recorder::finish_recording();
    Run { listing: listing.expect("gather"), calls }
}

type Details = Option<(Option<(String, String)>, Vec<(String, String)>)>;

fn details(verbose: &Option<VerboseData>) -> Details {
    let pair = |commit: &CommitDetail| (commit.short_sha.clone(), commit.refs.clone());
    verbose.as_ref().map(|data| (data.merge_base.as_ref().map(pair), data.branch_commits.iter().map(pair).collect()))
}

/// Everything the listing shows equals a listing gathered from scratch at
/// `at` now: caption, target, tree, counts, dirtiness, graph, and verbose
/// history all describe the final tips.
fn assert_describes_the_final_state(run: &Run, at: &Path, options: ListOptions, context: &str) {
    let fresh = parse_worktree_state_in(at).expect("parse");
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
    let (graph, verbose_data) = graph::gather(&input, options.graph, options.verbose && input.has_verbose());
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
    let rendezvous = overlap::Installed::new();

    let run = run(&repo.main, Script::held(Hold::BothStarted), options(true, false, BUDGET));

    assert!(observed().released_by_event, "the worker's outcome was held until both local gathers started");
    assert_eq!(rendezvous.outcome(), (true, true), "and the list and graph gathers still overlap each other");
    let waited = run.listing.remote.waited.as_ref().expect("launched");
    assert!(matches!(waited.head, HeadEnd::Finished(_)) && !waited.timed_out, "{waited:?}");
}

/// Without an origin there is no wait to overlap, and the two local gathers
/// still run side by side: each starts before the other finishes.
#[test]
#[serial_test::serial]
fn without_an_origin_the_list_and_graph_gathers_still_overlap() {
    let repo = Repo::new();
    run_git(&repo.main, &["remote", "remove", "origin"]);
    let rendezvous = overlap::Installed::new();

    let run = run(&repo.main, Script::finishing(), options(true, false, BUDGET));

    assert!(run.listing.remote.waited.is_none(), "nothing was launched");
    assert_eq!(
        rendezvous.outcome(),
        (true, true),
        "(list gather saw graph start, graph gather saw list start): each gather must start before the other finishes"
    );
}

#[test]
#[serial_test::serial]
fn unchanged_tips_accept_the_first_gather_and_measure_everything_once() {
    let repo = Repo::new();

    let run = run(&repo.main, Script::finishing(), options(true, false, BUDGET));

    assert!(!run.regathered());
    assert_eq!(run.ref_reads(), 2, "the parse step's read and the final one: {:?}", run.calls);
    assert_eq!(run.status_walks(), 2, "one per worktree: {:?}", run.calls);
    assert_describes_the_final_state(&run, &repo.main, options(true, false, BUDGET), "unchanged");
    // A single cold gather of the same state makes as many comparisons.
    repo.clear_cache();
    recorder::start_recording();
    let mut single = parse_worktree_state_in(&repo.main).expect("parse");
    fill_worktree_statuses(&mut single).expect("list");
    let single = recorder::finish_recording();
    let single_merge_trees =
        recorder::count_matching(&single, |args| args.first().map(String::as_str) == Some("merge-tree"));
    assert!(single_merge_trees > 0);
    assert_eq!(run.merge_trees(), single_merge_trees, "{:?}", run.calls);
}

#[test]
#[serial_test::serial]
fn without_an_origin_or_a_graph_the_single_gather_is_accepted() {
    // (label, removes origin, graph, ref reads)
    for (label, removes_origin, graph, ref_reads) in [("no origin", true, true, 1), ("no graph", false, false, 2)] {
        let repo = Repo::new();
        if removes_origin {
            run_git(&repo.main, &["remote", "remove", "origin"]);
        }

        let run = run(&repo.main, Script::finishing(), options(graph, false, BUDGET));

        let launched = OBSERVED.lock().unwrap_or_else(|e| e.into_inner()).is_some();
        assert_eq!(launched, !removes_origin, "{label}: a worker is launched only with an origin");
        assert_eq!(run.listing.remote.waited.is_some(), !removes_origin, "{label}");
        assert!(!run.regathered(), "{label}");
        assert_eq!(run.ref_reads(), ref_reads, "{label}: no second read without a wait: {:?}", run.calls);
        assert_eq!(run.status_walks(), 2, "{label}: {:?}", run.calls);
        assert_eq!(run.listing.graph.is_some(), graph, "{label}");
        let history = run.count(|args| matches!(args.first().map(String::as_str), Some("merge-base") | Some("log")));
        assert_eq!(history > 0, graph, "{label}: history is read only for the graph: {:?}", run.calls);
        // Removing `origin` removes its tracking refs, and with them the caption.
        let caption = (!removes_origin).then_some(CaptionState::InSync);
        assert_eq!(caption_state(&run), caption, "{label}");
        assert_describes_the_final_state(&run, &repo.main, options(graph, false, BUDGET), label);
    }
}

#[test]
#[serial_test::serial]
fn a_failed_preference_write_starts_no_local_gather_and_launches_nothing() {
    let repo = Repo::new();
    repo.record_gone_branch();
    run_git(&repo.main, &["remote", "set-url", "origin", "/srv/git/repo.git"]);
    let seam = overlap::Installed::with_mode(Mode::Observe);
    *SCRIPT.lock().unwrap_or_else(|e| e.into_inner()) = Some(Script::finishing());
    *OBSERVED.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let options = ListOptions { ignore_api: true, ..options(true, false, BUDGET) };

    let result = gather(&repo.main, &options, &mut |_| {});

    assert!(
        matches!(result, Err(crate::error::WorktreeError::NoRepositoryIdentity(_))),
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
        let _seam = overlap::Installed::with_mode(Mode::Observe);
        let moves = moves(&repo);
        let mut script = Script::held(Hold::BothFinished);
        script.moves = moves;

        let run = run(&repo.feature, script, options(true, true, BUDGET));

        assert!(observed().released_by_event, "{label}: the first gather finished before refs moved");
        assert!(run.regathered(), "{label}");
        assert_eq!(run.status_walks(), 2, "{label}: a fetch never measures dirtiness again: {:?}", run.calls);
        assert_describes_the_final_state(&run, &repo.feature, options(true, true, BUDGET), label);
        check(&run, &repo);
    }
}

fn args(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
}

#[test]
#[serial_test::serial]
fn fast_forward_measures_again_only_the_checkout_it_moved() {
    let fast_forward = ListOptions { fast_forward: true, ..options(true, false, BUDGET) };
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

        let run = run(&repo.main, Script::finishing(), fast_forward);

        let ff = run.listing.ff.as_ref().expect("--ff ran");
        assert!(expected(ff), "{label}: {ff:?}");
        assert_eq!(run.status_walks(), walks, "{label}: {:?}", run.calls);
        assert_eq!(run.regathered(), matches!(ff, FfResult::Moved { .. }), "{label}");
        assert_describes_the_final_state(&run, &repo.main, fast_forward, label);
    }
}

#[test]
#[serial_test::serial]
fn a_failed_ref_read_never_counts_as_unchanged_and_a_failed_final_read_never_prunes() {
    for (failing_read, label) in [(1, "initial"), (2, "final")] {
        let repo = Repo::new();
        repo.record_gone_branch();
        let reads = AtomicUsize::new(0);
        let _failure = recorder::fail_matching(move |args| {
            args.first() == Some(&"for-each-ref") && reads.fetch_add(1, Ordering::SeqCst) + 1 == failing_read
        });

        let run = run(&repo.main, Script::finishing(), options(true, false, BUDGET));
        drop(_failure);

        assert!(run.regathered(), "{label}: a failed read establishes nothing");
        let pruned = ForkOriginStore::load_from(&fork_origin_path(&repo.main).unwrap()).get("gone").is_none();
        if failing_read == 1 {
            assert!(run.listing.list.ref_snapshot().succeeded(), "{label}");
            assert_eq!(caption_state(&run), Some(CaptionState::InSync), "{label}: the final read's facts");
            assert!(pruned, "{label}: the final read succeeded");
            assert_describes_the_final_state(&run, &repo.main, options(true, false, BUDGET), label);
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
    let _seam = overlap::Installed::with_mode(Mode::Observe);
    let initial_origin = repo.sha("origin/main");
    let upstream = repo.upstream_commit("main");
    let mut script = Script::held(Hold::BothFinished).moving(&[&["update-ref", "refs/remotes/origin/main", &upstream]]);
    script.copy_record = Some(copy_record.clone());

    let run = run(&repo.main, script, options(true, false, BUDGET));

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
        let seam = overlap::Installed::with_mode(Mode::HoldListUntilRemote);
        let mut script = Script { finishes: false, ..Script::finishing() };
        if moves {
            script = script.moving(&[&["update-ref", "refs/remotes/origin/main", &repo.upstream_commit("main")]]);
        }

        let run = run(&repo.main, script, options(false, false, Duration::from_millis(100)));

        let waited = run.listing.remote.waited.as_ref().expect("launched");
        assert!(waited.timed_out, "{label}");
        assert_eq!(waited.head, HeadEnd::Running { last: None }, "{label}: the detached worker is not joined");
        assert!(seam.list_started_after_the_wait(), "{label}: the local gather outlasted the wait");
        assert_eq!(run.status_walks(), 2, "{label}: {:?}", run.calls);
        assert_eq!(run.regathered(), moves, "{label}");
        let expected = if moves { CaptionState::Behind(1) } else { CaptionState::InSync };
        assert_eq!(caption_state(&run), Some(expected), "{label}");
        assert_eq!(
            run.listing.list.statuses.iter().map(|status| status.dirty).collect::<Vec<_>>(),
            [DirtyStatus::Clean, DirtyStatus::Clean],
            "{label}"
        );
    }
}

/// Without a worker the stored answers are read as they are: nothing is
/// launched or waited for, and refs are read once.
#[test]
#[serial_test::serial]
fn without_a_worker_nothing_is_launched_and_refs_are_read_once() {
    let repo = Repo::new();
    *SCRIPT.lock().unwrap_or_else(|e| e.into_inner()) = Some(Script::finishing());
    *OBSERVED.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let options = ListOptions { worker: None, ..options(true, false, BUDGET) };
    let mut progress = Vec::new();

    recorder::start_recording();
    let listing = gather(&repo.main, &options, &mut |event| progress.push(event)).expect("gather");
    let calls = recorder::finish_recording();

    assert!(OBSERVED.lock().unwrap_or_else(|e| e.into_inner()).is_none(), "no worker was launched");
    assert!(progress.is_empty(), "no wait to report: {progress:?}");
    assert_eq!(listing.remote.origin.as_deref(), Some(ORIGIN));
    assert!(listing.remote.waited.is_none());
    assert!(!listing.regathered);
    let ref_reads = recorder::count_matching(&calls, |args| args.first().map(String::as_str) == Some("for-each-ref"));
    assert_eq!(ref_reads, 1, "{calls:?}");
    assert_eq!(caption_state(&Run { listing, calls }), Some(CaptionState::InSync));
}

/// A launched worker's wait is bracketed for the caller's progress display.
#[test]
#[serial_test::serial]
fn a_followed_wait_is_reported_as_started_then_finished() {
    let repo = Repo::new();
    *SCRIPT.lock().unwrap_or_else(|e| e.into_inner()) = Some(Script::finishing());
    let mut progress = Vec::new();

    gather(&repo.main, &options(false, false, BUDGET), &mut |event| progress.push(event)).expect("gather");

    assert_eq!(progress.first(), Some(&WaitProgress::Started), "{progress:?}");
    assert_eq!(progress.last(), Some(&WaitProgress::Finished), "{progress:?}");
    assert!(
        progress[1..progress.len() - 1].iter().all(|event| matches!(event, WaitProgress::Phase(_))),
        "{progress:?}"
    );
}

/// Every Git process the pipeline starts, on any of its threads, counts in a
/// scope the caller opened.
#[test]
#[serial_test::serial]
fn a_counting_scope_sees_every_call_of_the_threaded_pipeline() {
    let repo = Repo::new();
    let _seam = overlap::Installed::with_mode(Mode::Observe);
    let upstream = repo.upstream_commit("main");
    let script = Script::held(Hold::BothFinished).moving(&[&["update-ref", "refs/remotes/origin/main", &upstream]]);
    *SCRIPT.lock().unwrap_or_else(|e| e.into_inner()) = Some(script);

    recorder::start_recording();
    let scope = calls::CallScope::enter();
    let listing = gather(&repo.feature, &options(true, true, BUDGET), &mut |_| {}).expect("gather");
    let counted = scope.finish();
    let recorded = recorder::finish_recording();

    assert!(listing.regathered, "the regather's threads ran too");
    assert!(listing.graph.is_some() && listing.verbose.is_some());
    // The scripted worker's own `git` processes (`run_git`) are not the
    // library's, so they are neither recorded nor counted.
    assert_eq!(counted, recorded.len() as u64, "{recorded:?}");
}

/// The listing reads only the repository it is given: two listings of
/// different repositories at once each describe their own, and neither reads
/// nor changes the process's current directory.
#[test]
#[serial_test::serial]
fn concurrent_listings_of_two_repositories_stay_apart_and_leave_the_cwd_alone() {
    let first = Repo::new();
    let second = Repo::new();
    run_git(&second.feature, &["commit", "-q", "--allow-empty", "-m", "a2"]);
    let elsewhere = tempfile::tempdir().expect("temp dir");
    let before = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(elsewhere.path()).expect("enter a directory that is no repository");
    let local = ListOptions { graph: true, verbose: true, ..ListOptions::default() };

    let (a, b) = std::thread::scope(|scope| {
        let a = scope.spawn(|| gather(&first.feature, &local, &mut |_| {}));
        let b = scope.spawn(|| gather(&second.main, &local, &mut |_| {}));
        (a.join().expect("first"), b.join().expect("second"))
    });
    let during = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&before).expect("restore cwd");

    assert_eq!(during.canonicalize().ok(), elsewhere.path().canonicalize().ok(), "the cwd is unchanged");
    let (a, b) = (a.expect("first listing"), b.expect("second listing"));
    let current = |listing: &Listing| {
        listing.list.entries().iter().find(|entry| entry.is_current).map(|entry| entry.branch.clone())
    };
    assert_eq!(current(&a), Some(Some("feature-a".into())), "the first listing is from its feature checkout");
    assert_eq!(current(&b), Some(Some("main".into())), "the second from its main checkout");
    // Git reports the resolved path (`/private/var/...` for a macOS temp dir).
    let canonical = |path: Option<&Path>| path.and_then(|path| path.canonicalize().ok());
    assert_eq!(canonical(a.main_checkout.as_deref()), canonical(Some(&first.main)));
    assert_eq!(canonical(b.main_checkout.as_deref()), canonical(Some(&second.main)));
    assert_eq!(a.list.refs().local("feature-a"), Some(first.sha("feature-a").as_str()));
    assert_eq!(b.list.refs().local("feature-a"), Some(second.sha("feature-a").as_str()));
    assert!(a.verbose.is_some(), "a feature checkout has verbose details");
    assert!(b.graph.is_some() && b.verbose.is_none(), "a main checkout has a base view and no verbose details");
}

mod timings;
