//! The background refresh behind `wt list`: a detached
//! `wt internal-refresh <main checkout> [--attempt <id>]` that the listing
//! launches and never joins.
//!
//! The worker has two independent halves, run concurrently: the open-PR answer
//! ([`refresh`]) and the update of `origin/<default>`
//! ([`remote_update::run_attempt`]: check, publish, fetch on variance). Each
//! has its own store, lock, and deadlines, so either one finishes while the
//! other is blocked, contended, unsupported, failing, or has panicked. A
//! repository in `~/.wt.json` makes no provider request in either half.
//!
//! The PR half asks whenever it wins the PR lock, and once both halves are
//! done the worker writes the completion receipt the foreground waits for, on
//! every attempt.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;

use worktree::api_preference;
use worktree::git::git_from;
use worktree::pull_requests::{
    OpenPrSource, REFRESH_DEADLINE, RefreshOutcome, SniffOpenPrSource, origin_digest, origin_url, pr_store_path,
    refresh, unix_now,
};
use worktree::remote_head::{
    HeadStatus, PrFailure, PrStatus, Receipt, new_attempt_id, refresh_receipt_path, remote_head_store_path,
    remove_stale_receipts, write_receipt,
};
use worktree::remote_update::{AttemptRequest, GitTransport, Seams, SniffBranchHeads, run_attempt};
use worktree::worktree::default_branch_in;

use super::list::{LaunchArgs, WorkerHandle};

/// The hidden subcommand's name, shared by the launcher and `args.rs`.
pub const SUBCOMMAND: &str = "internal-refresh";

/// Starts the worker for the repository whose main checkout is `main`, for
/// attempt `args.attempt`.
///
/// The worker runs from `main`, so it never holds a linked worktree open, and
/// has no standard streams: on Windows an inherited pipe would make a caller
/// capturing `wt list`'s output wait for the worker. The handle only reports
/// whether it has exited; dropping it neither waits for nor kills the worker.
pub fn launch(main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
    let exe = std::env::current_exe()?;
    spawn_worker(&exe, main, args).map(WorkerHandle::from_child)
}

fn spawn_worker(exe: &Path, main: &Path, args: &LaunchArgs) -> std::io::Result<Child> {
    let mut command = Command::new(exe);
    command
        .arg(SUBCOMMAND)
        .arg(main)
        .arg("--attempt")
        .arg(&args.attempt)
        .current_dir(main)
        .env_remove("WT_SHELL_WRAPPER")
        .env_remove("COMPLETE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    sniff::process::configure_detached_child(&mut command);
    command.spawn()
}

/// The worker: runs attempt `attempt` (a new id when `None`) for `repo` when
/// it is a main checkout, and does nothing otherwise. It prints nothing and
/// never fails, since nobody reads its output or exit status.
pub fn run(repo: &Path, attempt: Option<&str>) {
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
    let receipt = receipt_target(&main, &id, origin.as_deref());
    run_and_record(
        &main,
        receipt.as_ref(),
        |main| pr_half(main, ignore_api),
        |main| head_half(main, &id, ignore_api),
    );
}

/// Where an attempt's completion receipt goes, and what it binds to.
struct ReceiptTarget {
    path: PathBuf,
    attempt_id: String,
    origin_digest: String,
    branch: String,
}

/// Every attempt's receipt target, bound to the origin and branch the attempt
/// started with. `None` without an origin or a default branch: there is
/// nothing to bind a receipt to, and the foreground's wait is bounded
/// without one.
fn receipt_target(main: &Path, id: &str, origin: Option<&str>) -> Option<ReceiptTarget> {
    let branch = default_branch_in(main).ok()?;
    let path = refresh_receipt_path(main, id).ok()?;
    Some(ReceiptTarget { path, attempt_id: id.to_string(), origin_digest: origin_digest(origin?), branch })
}

/// Runs both halves ([`run_halves`]) and then sweeps stale receipts and
/// writes this attempt's own; a half that panicked is recorded as failed. The
/// write is best effort: the foreground's wait is bounded without it, and a
/// successful PR publication is proven by the store's publication id, not by
/// the receipt.
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
        remove_stale_receipts(&target.path, std::time::SystemTime::now());
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

fn pr_half(main: &Path, ignore_api: bool) -> PrStatus {
    match pr_store_path(main) {
        Ok(store) => pr_status(&store, main, ignore_api, worker_source),
        Err(_) => PrStatus::Failed { failure: PrFailure::Other },
    }
}

/// The PR half: no request at all for an ignored repository, so it shows no
/// badges, otherwise [`refresh`] as the receipt reports it.
fn pr_status(
    store: &Path,
    main: &Path,
    ignore_api: bool,
    connect: impl FnOnce(&str) -> Box<dyn OpenPrSource>,
) -> PrStatus {
    if ignore_api {
        return PrStatus::Ignored;
    }
    match refresh(store, main, unix_now, connect) {
        RefreshOutcome::Refreshed => PrStatus::Ok,
        RefreshOutcome::Unsupported => PrStatus::Unsupported,
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
    use worktree::pull_requests::{CachedPrs, OpenPullRequest, PrRequestError, select_cached};
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

        let args = LaunchArgs { attempt: ID.into() };
        let started = spawn_worker(&missing, root.path(), &args);

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
                path: self.root.path().join(format!("refresh-receipt.{ID}.json")),
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
            pr_status(&self.pr_store(), main, false, move |_| Box::new(source) as Box<dyn OpenPrSource>)
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
        fn fetch(&self) -> Result<Vec<OpenPullRequest>, PrRequestError> {
            match self {
                Pr::Answer => Ok(Vec::new()),
                Pr::Fail => Err(PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) }.into()),
                Pr::Unsupported => Err(PrRequestError::Unsupported),
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
        for (source, expected) in [(Pr::Fail, rejected), (Pr::Unsupported, PrStatus::Unsupported)] {
            let fixture = Fixture::new();

            let (prs, _) = run_halves(
                &fixture.main(),
                |main| fixture.refresh_prs(main, source),
                |main| fixture.refresh_head(main, &present()),
            );

            assert_eq!(prs, Some(expected), "the outcome itself reaches the receipt");
            assert!(!fixture.pr_store().exists(), "nothing is stored");
            assert!(fixture.head_published(), "the head half published anyway");
        }
    }

    #[test]
    fn a_contended_pr_half_is_in_the_receipt_and_leaves_the_head_half_publishing() {
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
            fn fetch(&self) -> Result<Vec<OpenPullRequest>, PrRequestError> {
                let _ = self.entered.send(());
                let _ = self.released.lock().unwrap().recv_timeout(WAIT);
                Err(PrFailure::Other.into())
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

            run_and_record(
                &main,
                Some(&fixture.receipt_target()),
                |main| fixture.refresh_prs(main, Pr::Answer),
                |main| fixture.refresh_head(main, &present()).head_status(),
            );
            assert_eq!(fixture.receipt().map(|receipt| receipt.prs), Some(PrStatus::Contended), "the receipt says so");
            assert!(!fixture.pr_published(), "a contender makes no request");
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
    fn every_attempt_has_a_receipt_target_bound_to_its_origin_and_branch() {
        let fixture = Fixture::new();
        let target = receipt_target(&fixture.main(), ID, Some(ORIGIN)).expect("a target with no --force");
        assert_eq!(
            (target.attempt_id.as_str(), target.origin_digest, target.branch.as_str()),
            (ID, origin_digest(ORIGIN), "main")
        );
        assert!(target.path.to_string_lossy().ends_with(&format!("refresh-receipt.{ID}.json")), "{:?}", target.path);

        assert!(receipt_target(&fixture.main(), ID, None).is_none(), "no origin, nothing to bind");
        assert!(receipt_target(&fixture.main(), "not-an-id", Some(ORIGIN)).is_none(), "an invalid id");
    }

    /// The receipt every attempt writes carries the PR half's own status,
    /// from the real PR half against each kind of source.
    #[test]
    fn every_attempt_writes_a_receipt_with_its_pr_status() {
        let rejected = PrStatus::Failed { failure: PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) } };
        type PrHalf = Box<dyn Fn(&Fixture, &Path) -> PrStatus + Sync>;
        let cases: [(&str, PrHalf, PrStatus); 4] = [
            ("ok", Box::new(|fixture, main| fixture.refresh_prs(main, Pr::Answer)), PrStatus::Ok),
            ("failed", Box::new(|fixture, main| fixture.refresh_prs(main, Pr::Fail)), rejected),
            ("unsupported", Box::new(|fixture, main| fixture.refresh_prs(main, Pr::Unsupported)), PrStatus::Unsupported),
            (
                "ignored",
                Box::new(|fixture, main| {
                    pr_status(&fixture.pr_store(), main, true, |_| -> Box<dyn OpenPrSource> {
                        panic!("an ignored repository makes no PR request")
                    })
                }),
                PrStatus::Ignored,
            ),
        ];
        for (label, pr, expected) in cases {
            let fixture = Fixture::new();
            run_and_record(&fixture.main(), Some(&fixture.receipt_target()), |main| pr(&fixture, main), |_| HeadStatus::Ok);
            let receipt = fixture.receipt().unwrap_or_else(|| panic!("{label}: a receipt"));
            assert_eq!((receipt.head, receipt.prs), (HeadStatus::Ok, expected), "{label}");
        }
    }

    /// Ages `path`'s modification time by `age`.
    fn age_file(path: &Path, age: Duration) {
        let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(std::time::SystemTime::now() - age).unwrap();
    }

    #[test]
    fn writing_a_receipt_sweeps_only_old_receipts() {
        let fixture = Fixture::new();
        let target = fixture.receipt_target();
        let dir = fixture.root.path();
        let old = dir.join("refresh-receipt.11111111111111111111111111111111.json");
        let active = dir.join("refresh-receipt.22222222222222222222222222222222.json");
        let other_repo = dir.join("other.refresh-receipt.33333333333333333333333333333333.json");
        for path in [&old, &active, &other_repo] {
            std::fs::write(path, "{}").unwrap();
        }
        age_file(&old, worktree::remote_head::ATTEMPT_MAX_AGE + Duration::from_secs(5));
        age_file(&other_repo, worktree::remote_head::ATTEMPT_MAX_AGE + Duration::from_secs(5));
        age_file(&active, worktree::remote_head::ATTEMPT_MAX_AGE - Duration::from_secs(5));

        run_and_record(&fixture.main(), Some(&target), |_| PrStatus::Ok, |_| HeadStatus::Ok);

        assert!(!old.exists(), "an old receipt is swept");
        assert!(active.exists(), "another attempt's young receipt is kept");
        assert!(other_repo.exists(), "another repository's receipt is never touched");
        assert!(fixture.receipt().is_some(), "and this attempt's own is written");
    }

    #[test]
    fn a_receipt_that_cannot_be_written_leaves_the_published_answer() {
        let fixture = Fixture::new();
        // The receipt's directory is a file, so the write fails.
        let blocked = fixture.root.path().join("blocked");
        std::fs::write(&blocked, "").unwrap();
        let target = ReceiptTarget { path: blocked.join(format!("refresh-receipt.{ID}.json")), ..fixture.receipt_target() };

        run_and_record(&fixture.main(), Some(&target), |main| fixture.refresh_prs(main, Pr::Answer), |_| HeadStatus::Ok);

        assert!(!target.path.exists());
        assert!(fixture.pr_published(), "the store, not the receipt, holds the answer");
    }

    /// Counts requests and answers with no PRs.
    struct Counting(Arc<AtomicUsize>);

    impl OpenPrSource for Counting {
        fn source_repo(&self) -> Option<String> {
            Some("owner/repo".into())
        }
        fn fetch(&self) -> Result<Vec<OpenPullRequest>, PrRequestError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[test]
    fn the_pr_half_asks_on_every_attempt_even_when_the_answer_is_fresh() {
        let fixture = Fixture::new();
        let main = fixture.main();
        assert_eq!(fixture.refresh_prs(&main, Pr::Answer), PrStatus::Ok);
        assert!(fixture.pr_published(), "control: a fresh answer is stored");
        let calls = Arc::new(AtomicUsize::new(0));
        let status = || {
            let calls = calls.clone();
            pr_status(&fixture.pr_store(), &main, false, move |_| Box::new(Counting(calls)) as Box<dyn OpenPrSource>)
        };

        assert_eq!(status(), PrStatus::Ok);
        assert_eq!(status(), PrStatus::Ok);
        assert_eq!(calls.load(Ordering::SeqCst), 2, "one request per attempt");
    }

    #[test]
    fn an_ignored_repository_makes_no_pr_request() {
        let fixture = Fixture::new();
        let status = pr_status(&fixture.pr_store(), &fixture.main(), true, |_| -> Box<dyn OpenPrSource> {
            panic!("an ignored repository makes no PR request")
        });
        assert_eq!(status, PrStatus::Ignored);
        assert!(!fixture.pr_store().exists());
    }
}
