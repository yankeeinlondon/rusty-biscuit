//! The background refresh behind `wt list`: a detached
//! `wt internal-refresh <main checkout> [--attempt <id>] [--force]` that the
//! listing launches and never joins.
//!
//! The worker has two independent halves, run concurrently: the open-PR answer
//! ([`refresh`]) and the update of `origin/<default>`
//! ([`remote_update::run_attempt`]: check, publish, fetch on variance). Each
//! has its own store, lock, and deadlines, so either one finishes while the
//! other is blocked, contended, unsupported, failing, or has panicked. A
//! repository in `~/.wt.json` makes no provider request in either half.
//!
//! With `--force` (`wt list --refresh`), the PR half ignores its freshness
//! window, and once both halves are done the worker writes the completion
//! receipt the foreground waits for.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use worktree::api_preference;
use worktree::git::git_from;
use worktree::pull_requests::{
    OpenPrSource, REFRESH_DEADLINE, RefreshOutcome, SniffOpenPrSource, origin_digest, origin_url, pr_store_path,
    refresh, unix_now,
};
use worktree::remote_head::{
    HeadStatus, PrFailure, PrStatus, Receipt, new_attempt_id, refresh_receipt_path, remote_head_store_path,
    write_receipt,
};
use worktree::remote_update::{AttemptRequest, GitTransport, Seams, SniffBranchHeads, run_attempt};
use worktree::worktree::default_branch_in;

/// The hidden subcommand's name, shared by the launcher and `args.rs`.
pub const SUBCOMMAND: &str = "internal-refresh";

/// Starts the worker for the repository whose main checkout is `main`.
///
/// The worker runs from `main`, so it never holds a linked worktree open, and
/// has no standard streams: on Windows an inherited pipe would make a caller
/// capturing `wt list`'s output wait for the worker. A spawn failure is
/// ignored; the stored answers stay shown.
pub fn launch(main: &Path) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = spawn_worker(&exe, main);
    }
}

fn spawn_worker(exe: &Path, main: &Path) -> std::io::Result<()> {
    let mut command = Command::new(exe);
    command
        .arg(SUBCOMMAND)
        .arg(main)
        .current_dir(main)
        .env_remove("WT_SHELL_WRAPPER")
        .env_remove("COMPLETE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    sniff::process::configure_detached_child(&mut command);
    // Dropping the handle neither waits for nor kills the worker.
    command.spawn().map(drop)
}

/// The worker: runs attempt `attempt` (a new id when `None`) for `repo` when
/// it is a main checkout, and does nothing otherwise. It prints nothing and
/// never fails, since nobody reads its output or exit status.
pub fn run(repo: &Path, attempt: Option<&str>, force: bool) {
    let Some(main) = main_checkout(repo) else {
        return;
    };
    let Some(id) = attempt.map(str::to_string).or_else(|| new_attempt_id().ok()) else {
        return;
    };
    let origin = origin_url(&main);
    let ignore_api = origin.as_deref().is_some_and(|origin| {
        api_preference::preference_path().is_some_and(|path| api_preference::load(&path).ignores_origin(origin))
    });
    // The receipt speaks for the origin and branch the attempt started with.
    let receipt = match (force, origin, default_branch_in(&main), refresh_receipt_path(&main)) {
        (true, Some(origin), Ok(branch), Ok(path)) => {
            Some(ReceiptTarget { path, attempt_id: id.clone(), origin_digest: origin_digest(&origin), branch })
        }
        _ => None,
    };
    run_and_record(
        &main,
        receipt.as_ref(),
        |main| pr_half(main, force, ignore_api),
        |main| head_half(main, &id, ignore_api),
    );
}

/// Where a forced run's completion receipt goes, and what it binds to.
struct ReceiptTarget {
    path: PathBuf,
    attempt_id: String,
    origin_digest: String,
    branch: String,
}

/// Runs both halves ([`run_halves`]) and then, for a forced run, writes the
/// receipt; a half that panicked is recorded as failed. The write is best
/// effort: the foreground's wait is bounded without it.
fn run_and_record(
    main: &Path,
    receipt: Option<&ReceiptTarget>,
    pr: impl FnOnce(&Path) -> PrStatus + Send,
    head: impl FnOnce(&Path) -> HeadStatus + Send,
) {
    let (prs, head) = run_halves(main, pr, head);
    if let Some(target) = receipt {
        let receipt = Receipt {
            attempt_id: target.attempt_id.clone(),
            origin_digest: target.origin_digest.clone(),
            branch: target.branch.clone(),
            finished_at: unix_now(),
            head: head.unwrap_or(HeadStatus::Failed),
            prs: prs.unwrap_or(PrStatus::Failed { failure: PrFailure::Other }),
        };
        let _ = write_receipt(&target.path, &receipt);
    }
}

/// Runs both halves on their own threads and returns once both are done,
/// with `None` for a half that panicked.
///
/// Each half is joined separately and a panic is discarded, so one half's
/// panic neither stops nor hides the other's publication.
fn run_halves<P: Send, H: Send>(
    main: &Path,
    pr: impl FnOnce(&Path) -> P + Send,
    head: impl FnOnce(&Path) -> H + Send,
) -> (Option<P>, Option<H>) {
    std::thread::scope(|scope| {
        let pr = scope.spawn(|| pr(main));
        let head = scope.spawn(|| head(main));
        (pr.join().ok(), head.join().ok())
    })
}

fn pr_half(main: &Path, force: bool, ignore_api: bool) -> PrStatus {
    match pr_store_path(main) {
        Ok(store) => pr_status(&store, main, force, ignore_api, worker_source),
        Err(_) => PrStatus::Failed { failure: PrFailure::Other },
    }
}

/// The PR half: no request at all for an ignored repository, so it shows no
/// badges, otherwise [`refresh`] as the receipt reports it.
fn pr_status(
    store: &Path,
    main: &Path,
    force: bool,
    ignore_api: bool,
    connect: impl FnOnce(&str) -> Box<dyn OpenPrSource>,
) -> PrStatus {
    if ignore_api {
        return PrStatus::Ignored;
    }
    match refresh(store, main, unix_now, force, connect) {
        RefreshOutcome::Refreshed => PrStatus::Ok,
        RefreshOutcome::AlreadyFresh => PrStatus::SkippedFresh,
        RefreshOutcome::Contended => PrStatus::Contended,
        RefreshOutcome::Failed(failure) => PrStatus::Failed { failure },
        RefreshOutcome::LockFailed
        | RefreshOutcome::NoOrigin
        | RefreshOutcome::OriginChanged
        | RefreshOutcome::PublishFailed => PrStatus::Failed { failure: PrFailure::Other },
    }
}

fn head_half(main: &Path, id: &str, ignore_api: bool) -> HeadStatus {
    let Ok(store) = remote_head_store_path(main) else {
        return HeadStatus::Failed;
    };
    let started = Instant::now();
    let monotonic = || started.elapsed();
    let seams = Seams { api: &SniffBranchHeads, git: &GitTransport { main }, now: &unix_now, monotonic: &monotonic };
    run_attempt(AttemptRequest { store: &store, main, id, ignore_api }, &seams).head_status()
}

fn worker_source(origin: &str) -> Box<dyn OpenPrSource> {
    Box::new(SniffOpenPrSource {
        remote_url: origin.to_string(),
        deadline: REFRESH_DEADLINE,
    })
}

/// `repo` itself when it is the top level of a repository's main checkout
/// (not a linked worktree, not a subdirectory).
fn main_checkout(repo: &Path) -> Option<PathBuf> {
    let canonical = std::fs::canonicalize(repo).ok()?;
    let out = git_from(
        &canonical,
        &canonical,
        &["rev-parse", "--path-format=absolute", "--show-toplevel", "--git-dir", "--git-common-dir"],
    )
    .ok()?;
    let mut lines = out.lines().map(|line| std::fs::canonicalize(line.trim()).ok());
    let (Some(Some(top)), Some(Some(git_dir)), Some(Some(common))) = (lines.next(), lines.next(), lines.next()) else {
        return None;
    };
    (top == canonical && git_dir == common).then_some(canonical)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
    use std::time::Duration;

    use sniff::remote::blocking::PrUnavailable;
    use worktree::live_remote::GitFailure;
    use worktree::pull_requests::{CachedPrs, OpenPullRequest, select_cached};
    use worktree::remote_head::{Attempt, load_receipt, read_store};
    use worktree::remote_update::{AttemptEnd, BranchHeadSource, GitRemote};

    use super::*;

    const ORIGIN: &str = "https://refresh.example.invalid/owner/repo.git";
    const HEAD_SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    const ID: &str = "0123456789abcdef0123456789abcdef";
    /// Only a failing (sequential) worker waits this long.
    const WAIT: Duration = Duration::from_secs(10);

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git").current_dir(dir).args(args).status().expect("git");
        assert!(status.success(), "git {args:?}");
    }

    #[test]
    fn only_the_top_level_of_a_main_checkout_is_accepted() {
        let root = tempfile::tempdir().expect("temp dir");
        let main = root.path().join("main");
        std::fs::create_dir_all(main.join("sub")).unwrap();
        git(&main, &["init", "-q", "-b", "main"]);
        git(&main, &["-c", "user.email=t@example.com", "-c", "user.name=T", "-c", "commit.gpgsign=false", "commit", "-q", "--allow-empty", "-m", "c"]);
        let linked = root.path().join("linked");
        git(&main, &["worktree", "add", "-q", "-b", "feat", linked.to_str().unwrap()]);

        assert_eq!(main_checkout(&main), Some(std::fs::canonicalize(&main).unwrap()));
        assert_eq!(main_checkout(&linked), None, "a linked worktree");
        assert_eq!(main_checkout(&main.join("sub")), None, "a subdirectory");
        assert_eq!(main_checkout(&root.path().join("missing")), None, "a missing path");
        assert_eq!(main_checkout(root.path()), None, "not a repository");
    }

    #[test]
    fn a_worker_that_cannot_start_is_an_error_launch_discards() {
        let root = tempfile::tempdir().expect("temp dir");
        let missing = root.path().join("no-such-wt");

        let started = spawn_worker(&missing, root.path());

        assert!(started.is_err(), "{started:?}");
    }

    /// A main checkout on `main` with an `origin`, and a private store
    /// directory for both halves and the receipt.
    struct Fixture {
        root: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            let root = tempfile::tempdir().expect("temp dir");
            let main = root.path().join("main");
            std::fs::create_dir(&main).unwrap();
            git(&main, &["init", "-q", "-b", "main"]);
            git(&main, &["-c", "user.email=t@example.com", "-c", "user.name=T", "-c", "commit.gpgsign=false", "commit", "-q", "--allow-empty", "-m", "c"]);
            git(&main, &["remote", "add", "origin", ORIGIN]);
            Self { root }
        }

        fn main(&self) -> PathBuf {
            self.root.path().join("main")
        }

        fn pr_store(&self) -> PathBuf {
            self.root.path().join("prs.json")
        }

        fn head_store(&self) -> PathBuf {
            self.root.path().join("remote-head.json")
        }

        fn receipt_target(&self) -> ReceiptTarget {
            ReceiptTarget {
                path: self.root.path().join("refresh-receipt.json"),
                attempt_id: ID.into(),
                origin_digest: origin_digest(ORIGIN),
                branch: "main".into(),
            }
        }

        fn receipt(&self) -> Option<Receipt> {
            let attempt = Attempt::begin(ID.into(), origin_digest(ORIGIN), "main".into(), 0);
            load_receipt(&self.receipt_target().path, &attempt)
        }

        fn pr_published(&self) -> bool {
            matches!(select_cached(&self.pr_store(), Some(ORIGIN), unix_now()), CachedPrs::Fresh(_))
        }

        fn head_published(&self) -> bool {
            read_store(&self.head_store()).answer.is_some_and(|answer| answer.sha.as_deref() == Some(HEAD_SHA))
        }

        /// The PR half against this fixture's store, with `source`.
        fn refresh_prs(&self, main: &Path, source: impl OpenPrSource + 'static) -> PrStatus {
            self.refresh_prs_with(main, false, source)
        }

        fn refresh_prs_with(&self, main: &Path, force: bool, source: impl OpenPrSource + 'static) -> PrStatus {
            pr_status(&self.pr_store(), main, force, false, move |_| Box::new(source) as Box<dyn OpenPrSource>)
        }

        /// The real update attempt against this fixture's store, with the
        /// provider scripted and Git unable to fetch.
        fn refresh_head(&self, main: &Path, head: &Head) -> AttemptEnd {
            let now = unix_now;
            let monotonic = || Duration::ZERO;
            run_attempt(
                AttemptRequest { store: &self.head_store(), main, id: ID, ignore_api: false },
                &Seams { api: head, git: &NoGit, now: &now, monotonic: &monotonic },
            )
        }
    }

    enum Pr {
        Answer,
        Fail,
        Unsupported,
    }

    impl OpenPrSource for Pr {
        fn source_repo(&self) -> Option<String> {
            match self {
                Pr::Unsupported => None,
                _ => Some("owner/repo".into()),
            }
        }
        fn fetch(&self) -> Result<Vec<OpenPullRequest>, PrFailure> {
            match self {
                Pr::Answer => Ok(Vec::new()),
                Pr::Fail => Err(PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) }),
                Pr::Unsupported => Err(PrFailure::Other),
            }
        }
    }

    /// A scripted provider branch head.
    struct Head(Result<String, PrUnavailable>);

    impl BranchHeadSource for Head {
        fn branch_head(&self, _origin: &str, _branch: &str, _deadline: Duration) -> Result<String, PrUnavailable> {
            self.0.clone()
        }
        fn key_in_use(&self, _origin: &str) -> Option<String> {
            None
        }
    }

    /// Git that can neither list nor fetch: the fixture's origin is not real.
    struct NoGit;

    impl GitRemote for NoGit {
        fn live_head(&self, _branch: &str, _deadline: Duration) -> Result<Option<String>, GitFailure> {
            Err(GitFailure::Other)
        }
        fn fetch(&self, _branch: &str, _deadline: Duration) -> Result<(), GitFailure> {
            Err(GitFailure::Other)
        }
    }

    fn present() -> Head {
        Head(Ok(HEAD_SHA.to_string()))
    }

    /// Blocks until the other half signals, bounded by [`WAIT`]; `true` when
    /// the signal came, which only a concurrent worker can deliver.
    fn wait_for(signal: &Receiver<()>) -> bool {
        match signal.recv_timeout(WAIT) {
            Ok(()) => true,
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => false,
        }
    }

    fn signal(to: &Sender<()>) {
        let _ = to.send(());
    }

    fn signal_result(to: &Sender<bool>, value: bool) {
        let _ = to.send(value);
    }

    #[test]
    fn the_head_half_publishes_while_the_pr_half_is_blocked() {
        let fixture = Fixture::new();
        let (published, head_done) = mpsc::channel();
        let (saw, seen) = mpsc::channel();
        let fixture = &fixture;

        run_halves(
            &fixture.main(),
            move |_| {
                // Blocked until the head half has published.
                signal_result(&saw, wait_for(&head_done));
            },
            move |main| {
                // No tracking ref here, so the check is followed by a fetch,
                // which fails; the check was published first.
                fixture.refresh_head(main, &present());
                signal(&published);
            },
        );

        assert_eq!(seen.recv_timeout(WAIT), Ok(true), "the head half published during the PR half");
        assert!(fixture.head_published());
    }

    #[test]
    fn the_pr_half_publishes_while_the_head_half_is_blocked() {
        let fixture = Fixture::new();
        let (published, pr_done) = mpsc::channel();
        let (saw, seen) = mpsc::channel();
        let fixture = &fixture;

        run_halves(
            &fixture.main(),
            move |main| {
                assert_eq!(fixture.refresh_prs(main, Pr::Answer), PrStatus::Ok);
                signal(&published);
            },
            move |_| {
                signal_result(&saw, wait_for(&pr_done));
            },
        );

        assert_eq!(seen.recv_timeout(WAIT), Ok(true), "the PR half published during the head half");
        assert!(fixture.pr_published());
    }

    #[test]
    fn a_failing_or_unsupported_pr_half_leaves_the_head_half_publishing() {
        let rejected = PrStatus::Failed { failure: PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) } };
        let other = PrStatus::Failed { failure: PrFailure::Other };
        for (source, expected) in [(Pr::Fail, rejected), (Pr::Unsupported, other)] {
            let fixture = Fixture::new();

            let (prs, _) = run_halves(
                &fixture.main(),
                |main| fixture.refresh_prs(main, source),
                |main| fixture.refresh_head(main, &present()),
            );

            assert_eq!(prs, Some(expected), "the failure itself reaches the receipt");
            assert!(!fixture.pr_store().exists(), "a failure is never stored");
            assert!(fixture.head_published(), "the head half published anyway");
        }
    }

    #[test]
    fn a_contended_pr_half_leaves_the_head_half_publishing() {
        let fixture = Fixture::new();
        let main = fixture.main();
        // Another worker holds the PR lock, blocked in its request.
        let (entered, holder_entered) = mpsc::channel();
        let (release, released) = mpsc::channel::<()>();
        struct Blocked {
            entered: Sender<()>,
            released: std::sync::Mutex<Receiver<()>>,
        }
        impl OpenPrSource for Blocked {
            fn source_repo(&self) -> Option<String> {
                Some("owner/repo".into())
            }
            fn fetch(&self) -> Result<Vec<OpenPullRequest>, PrFailure> {
                let _ = self.entered.send(());
                let _ = self.released.lock().unwrap().recv_timeout(WAIT);
                Err(PrFailure::Other)
            }
        }

        std::thread::scope(|scope| {
            let holder = scope.spawn(|| {
                fixture.refresh_prs(
                    &main,
                    Blocked {
                        entered,
                        released: std::sync::Mutex::new(released),
                    },
                )
            });
            holder_entered.recv_timeout(WAIT).expect("the holder is in its request");

            let (prs, _) = run_halves(
                &main,
                |main| fixture.refresh_prs(main, Pr::Answer),
                |main| fixture.refresh_head(main, &present()),
            );
            assert_eq!(prs, Some(PrStatus::Contended));
            assert!(fixture.head_published(), "contention did not stop the head half");

            signal(&release);
            assert_eq!(holder.join().expect("holder"), PrStatus::Failed { failure: PrFailure::Other });
        });
    }

    #[test]
    fn a_failing_head_half_leaves_the_pr_half_publishing() {
        let failures = [
            PrUnavailable::Network { message: "could not read from remote".into() },
            PrUnavailable::Other { message: "malformed".into() },
        ];
        for failure in failures {
            let fixture = Fixture::new();

            let (_, head) = run_halves(
                &fixture.main(),
                |main| fixture.refresh_prs(main, Pr::Answer),
                |main| fixture.refresh_head(main, &Head(Err(failure))),
            );

            assert_eq!(head.map(AttemptEnd::head_status), Some(HeadStatus::Failed));
            assert_eq!(read_store(&fixture.head_store()).answer, None, "a failure is never an answer");
            assert!(fixture.pr_published(), "the PR half published anyway");
        }
    }

    #[test]
    fn a_panicking_half_does_not_stop_the_other() {
        let fixture = Fixture::new();
        let (prs, head) = run_halves(
            &fixture.main(),
            |_| -> PrStatus { panic!("the PR half panicked") },
            |main| fixture.refresh_head(main, &present()),
        );
        assert_eq!(prs, None);
        assert!(head.is_some());
        assert!(fixture.head_published());

        let fixture = Fixture::new();
        let (prs, head) = run_halves(
            &fixture.main(),
            |main| fixture.refresh_prs(main, Pr::Answer),
            |_| -> AttemptEnd { panic!("the head half panicked") },
        );
        assert_eq!((prs, head), (Some(PrStatus::Ok), None));
        assert!(fixture.pr_published());
    }

    #[test]
    fn a_receipt_is_written_only_after_both_halves_finish() {
        let fixture = Fixture::new();
        let target = fixture.receipt_target();
        let (head_done, head_finished) = mpsc::channel();
        let path = &target.path;

        run_and_record(
            &fixture.main(),
            Some(&target),
            move |_| {
                // Still no receipt once the head half is done: this half is not.
                assert!(wait_for(&head_finished), "the head half finished");
                assert!(!path.exists(), "no receipt while the PR half runs");
                PrStatus::Failed { failure: PrFailure::RateLimited { authenticated: false, key: None } }
            },
            move |_| {
                assert!(!path.exists(), "no receipt while the head half runs");
                signal(&head_done);
                HeadStatus::Ok
            },
        );

        let receipt = fixture.receipt().expect("a receipt after both halves");
        assert_eq!(receipt.head, HeadStatus::Ok);
        assert_eq!(receipt.prs, PrStatus::Failed { failure: PrFailure::RateLimited { authenticated: false, key: None } });
        assert_eq!((receipt.attempt_id.as_str(), receipt.branch.as_str()), (ID, "main"));
    }

    #[test]
    fn a_panicking_half_is_recorded_as_failed_in_the_receipt() {
        let fixture = Fixture::new();
        run_and_record(
            &fixture.main(),
            Some(&fixture.receipt_target()),
            |_| -> PrStatus { panic!("the PR half panicked") },
            |_| HeadStatus::AdoptedElsewhere,
        );
        let receipt = fixture.receipt().expect("a receipt");
        assert_eq!(
            (receipt.head, receipt.prs),
            (HeadStatus::AdoptedElsewhere, PrStatus::Failed { failure: PrFailure::Other })
        );

        let fixture = Fixture::new();
        run_and_record(
            &fixture.main(),
            Some(&fixture.receipt_target()),
            |_| PrStatus::Ok,
            |_| -> HeadStatus { panic!("the head half panicked") },
        );
        let receipt = fixture.receipt().expect("a receipt");
        assert_eq!((receipt.head, receipt.prs), (HeadStatus::Failed, PrStatus::Ok));
    }

    #[test]
    fn an_unforced_run_writes_no_receipt() {
        let fixture = Fixture::new();
        run_and_record(&fixture.main(), None, |_| PrStatus::Ok, |_| HeadStatus::Ok);
        assert!(!fixture.receipt_target().path.exists());
    }

    /// Counts requests and answers with no PRs.
    struct Counting(Arc<AtomicUsize>);

    impl OpenPrSource for Counting {
        fn source_repo(&self) -> Option<String> {
            Some("owner/repo".into())
        }
        fn fetch(&self) -> Result<Vec<OpenPullRequest>, PrFailure> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[test]
    fn a_forced_pr_half_asks_even_when_the_answer_is_fresh() {
        let fixture = Fixture::new();
        let main = fixture.main();
        assert_eq!(fixture.refresh_prs(&main, Pr::Answer), PrStatus::Ok);
        let calls = Arc::new(AtomicUsize::new(0));
        let status = |force| {
            let calls = calls.clone();
            pr_status(&fixture.pr_store(), &main, force, false, move |_| Box::new(Counting(calls)) as Box<dyn OpenPrSource>)
        };

        assert_eq!(status(false), PrStatus::SkippedFresh);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(status(true), PrStatus::Ok);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn an_ignored_repository_makes_no_pr_request() {
        let fixture = Fixture::new();
        let status = pr_status(&fixture.pr_store(), &fixture.main(), true, true, |_| -> Box<dyn OpenPrSource> {
            panic!("an ignored repository makes no PR request")
        });
        assert_eq!(status, PrStatus::Ignored);
        assert!(!fixture.pr_store().exists());
    }
}
