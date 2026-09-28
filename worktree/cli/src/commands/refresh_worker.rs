//! The background refresh behind `wt list`: a detached
//! `wt internal-refresh <main checkout>` that the listing launches at most once
//! and never waits for.
//!
//! The worker has two independent halves, run concurrently: the open-PR answer
//! ([`refresh`]) and the live head of `origin`'s default branch
//! ([`refresh_remote_head`]). Each has its own store, lock, freshness recheck,
//! and deadline, so either one publishes while the other is blocked, contended,
//! unsupported, failing, or has panicked.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use worktree::git::git_from;
use worktree::live_remote::LsRemote;
use worktree::pull_requests::{
    OpenPrSource, REFRESH_DEADLINE, SniffOpenPrSource, pr_store_path, refresh, unix_now,
};
use worktree::remote_head::{REMOTE_HEAD_REFRESH_DEADLINE, refresh_remote_head, remote_head_store_path};

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

/// The worker: refreshes both stored answers for `repo` when it is a main
/// checkout, and does nothing otherwise. It prints nothing and never fails,
/// since nobody reads its output or exit status.
pub fn run(repo: &Path) {
    let Some(main) = main_checkout(repo) else {
        return;
    };
    run_halves(&main, pr_half, head_half);
}

/// Runs both halves on their own threads and returns once both are done.
///
/// Each half is joined separately and a panic is discarded, so one half's
/// panic neither stops nor hides the other's publication.
fn run_halves(main: &Path, pr: impl FnOnce(&Path) + Send, head: impl FnOnce(&Path) + Send) {
    std::thread::scope(|scope| {
        let pr = scope.spawn(|| pr(main));
        let head = scope.spawn(|| head(main));
        let _ = pr.join();
        let _ = head.join();
    });
}

fn pr_half(main: &Path) {
    if let Ok(store) = pr_store_path(main) {
        refresh(&store, main, unix_now, worker_source);
    }
}

fn head_half(main: &Path) {
    if let Ok(store) = remote_head_store_path(main) {
        let heads = LsRemote {
            base: main,
            deadline: REMOTE_HEAD_REFRESH_DEADLINE,
        };
        refresh_remote_head(&store, main, unix_now, &heads);
    }
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
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
    use std::time::Duration;

    use worktree::live_remote::RemoteHeads;
    use worktree::pull_requests::{CachedPrs, OpenPullRequest, RefreshOutcome, select_cached};
    use worktree::remote_head::{CachedRemoteHead, select_cached_head};

    use super::*;

    const ORIGIN: &str = "https://refresh.example.invalid/owner/repo.git";
    const HEAD_SHA: &str = "0123456789abcdef0123456789abcdef01234567";
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
    /// directory for both halves.
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

        fn pr_published(&self) -> bool {
            matches!(select_cached(&self.pr_store(), Some(ORIGIN), unix_now()), CachedPrs::Fresh(_))
        }

        fn head_published(&self) -> bool {
            matches!(
                select_cached_head(&self.head_store(), Some(ORIGIN), Some("main"), unix_now()),
                CachedRemoteHead::Fresh(_)
            )
        }

        /// The real PR refresh against this fixture's store, with `source`.
        fn refresh_prs(&self, main: &Path, source: impl OpenPrSource + 'static) -> RefreshOutcome {
            refresh(&self.pr_store(), main, unix_now, move |_| Box::new(source) as Box<dyn OpenPrSource>)
        }

        /// The real live-head refresh against this fixture's store.
        fn refresh_head(&self, main: &Path, heads: &dyn RemoteHeads) -> RefreshOutcome {
            refresh_remote_head(&self.head_store(), main, unix_now, heads)
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
        fn fetch(&self) -> Result<Vec<OpenPullRequest>, String> {
            match self {
                Pr::Answer => Ok(Vec::new()),
                Pr::Fail => Err("401 unauthorized".into()),
                Pr::Unsupported => Err("unsupported provider".into()),
            }
        }
    }

    struct Head(Result<Option<String>, String>);

    impl RemoteHeads for Head {
        fn live_head(&self, _remote: &str, _branch: &str) -> Result<Option<String>, String> {
            self.0.clone()
        }
    }

    fn present() -> Head {
        Head(Ok(Some(HEAD_SHA.to_string())))
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
                assert_eq!(fixture.refresh_head(main, &present()), RefreshOutcome::Refreshed);
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
                assert_eq!(fixture.refresh_prs(main, Pr::Answer), RefreshOutcome::Refreshed);
                signal(&published);
            },
            move |_| {
                signal_result(&saw, wait_for(&pr_done));
            },
        );

        assert_eq!(seen.recv_timeout(WAIT), Ok(true), "the PR half published during the head half");
        assert!(fixture.pr_published());
    }

    fn signal_result(to: &Sender<bool>, value: bool) {
        let _ = to.send(value);
    }

    #[test]
    fn a_failing_or_unsupported_pr_half_leaves_the_head_half_publishing() {
        for (source, expected) in [(Pr::Fail, RefreshOutcome::Failed), (Pr::Unsupported, RefreshOutcome::Failed)] {
            let fixture = Fixture::new();
            let (outcome, outcomes) = mpsc::channel();

            run_halves(
                &fixture.main(),
                |main| {
                    let _ = outcome.send(fixture.refresh_prs(main, source));
                },
                |main| {
                    fixture.refresh_head(main, &present());
                },
            );

            assert_eq!(outcomes.recv_timeout(WAIT), Ok(expected));
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
            fn fetch(&self) -> Result<Vec<OpenPullRequest>, String> {
                let _ = self.entered.send(());
                let _ = self.released.lock().unwrap().recv_timeout(WAIT);
                Err("released".into())
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

            let (outcome, outcomes) = mpsc::channel();
            run_halves(
                &main,
                |main| {
                    let _ = outcome.send(fixture.refresh_prs(main, Pr::Answer));
                },
                |main| {
                    fixture.refresh_head(main, &present());
                },
            );
            assert_eq!(outcomes.recv_timeout(WAIT), Ok(RefreshOutcome::Contended));
            assert!(fixture.head_published(), "contention did not stop the head half");

            signal(&release);
            assert_eq!(holder.join().expect("holder"), RefreshOutcome::Failed);
        });
    }

    #[test]
    fn a_failing_head_half_leaves_the_pr_half_publishing() {
        for head in [Head(Err("could not read from remote".into())), Head(Err("malformed".into()))] {
            let fixture = Fixture::new();
            let (outcome, outcomes) = mpsc::channel();

            run_halves(
                &fixture.main(),
                |main| {
                    fixture.refresh_prs(main, Pr::Answer);
                },
                |main| {
                    let _ = outcome.send(fixture.refresh_head(main, &head));
                },
            );

            assert_eq!(outcomes.recv_timeout(WAIT), Ok(RefreshOutcome::Failed));
            assert!(!fixture.head_store().exists(), "a failure is never stored");
            assert!(fixture.pr_published(), "the PR half published anyway");
        }
    }

    #[test]
    fn a_panicking_half_does_not_stop_the_other() {
        let fixture = Fixture::new();
        run_halves(
            &fixture.main(),
            |_| panic!("the PR half panicked"),
            |main| {
                fixture.refresh_head(main, &present());
            },
        );
        assert!(fixture.head_published());

        let fixture = Fixture::new();
        run_halves(
            &fixture.main(),
            |main| {
                fixture.refresh_prs(main, Pr::Answer);
            },
            |_| panic!("the head half panicked"),
        );
        assert!(fixture.pr_published());
    }
}
