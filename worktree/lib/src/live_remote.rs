//! Non-interactive, deadline-bound git network calls.
//!
//! Every call runs with credential prompts disabled (`GIT_TERMINAL_PROMPT=0`,
//! `credential.interactive=never`, `GCM_INTERACTIVE=never`) and SSH in batch
//! mode, so a missing credential fails instead of prompting. At the deadline
//! the whole process tree is killed: `git` runs its transport (`ssh`,
//! `git-remote-https`) in a grandchild that would otherwise keep the output
//! pipe open, and on Windows the `git.exe` launcher's real git would keep
//! running (spike S2).
//!
//! Output is complete or it is an error. A successful exit whose stdout could
//! not be read to the end (a read error, a pipe still open after the grace
//! period) is `Err`, never `Ok("")`, and [`LsRemote`] rejects any line that is
//! not `<object id>\t<refname>`. So only a complete answer without the exact
//! ref is `Ok(None)`: a failed or truncated request can never read as a
//! deleted branch. Removal and the `wt list` live-head store both rely on that.
//!
//! Git runs with `LC_ALL=C`, so [`classify_git_failure`] can read its fixed
//! English text. A deadline is typed ([`GitFailure::Timeout`]), never
//! matched from a message.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::git::git_from;

/// Why a non-interactive git call gave no answer, as far as git's `LC_ALL=C`
/// text establishes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitFailure {
    /// The deadline passed and the process tree was killed.
    Timeout,
    /// Git said it had no usable credentials or the server refused them.
    Credentials,
    /// Anything else, including a 403, a 404, a refused connection, and a
    /// held ref lock: none of those establishes a reason worth naming.
    Other,
}

/// A failed transport call: its [`GitFailure`] and a readable reason for
/// callers that report one (git's stderr, the deadline, a spawn error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportError {
    pub failure: GitFailure,
    pub reason: String,
}

impl TransportError {
    fn other(reason: impl Into<String>) -> Self {
        Self { failure: GitFailure::Other, reason: reason.into() }
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason)
    }
}

/// `LC_ALL=C` stderr fragments that establish a credentials failure (spike
/// S3). `unable to get password from user` is what git prints under the
/// `credential.interactive=never` every call passes.
const CREDENTIALS_PATTERNS: [&str; 6] = [
    "Authentication failed for",
    "could not read Username",
    "could not read Password",
    "unable to get password from user",
    "Permission denied (publickey",
    "The requested URL returned error: 401",
];

/// Maps git's `LC_ALL=C` stderr to [`GitFailure::Credentials`] or
/// [`GitFailure::Other`]; it never yields [`GitFailure::Timeout`].
pub fn classify_git_failure(stderr: &str) -> GitFailure {
    if CREDENTIALS_PATTERNS.iter().any(|pattern| stderr.contains(pattern)) {
        GitFailure::Credentials
    } else {
        GitFailure::Other
    }
}

/// Removal's ruled deadline for one live `ls-remote` (Decision 21). The
/// `wt list` live-head refresh uses
/// [`REMOTE_HEAD_REFRESH_DEADLINE`](crate::remote_head::REMOTE_HEAD_REFRESH_DEADLINE).
pub const LIVE_CHECK_DEADLINE: Duration = Duration::from_secs(3);

/// Deadline for removal's lease-protected deletion push.
pub const PUSH_DEADLINE: Duration = Duration::from_secs(30);

/// Reads live branch heads.
///
/// A trait so removal's tier logic and the live-head refresh can be tested
/// with scripted answers.
pub trait RemoteHeads: Sync {
    /// The live SHA of `refs/heads/<branch>` at `remote` (a remote name, which
    /// reads its fetch URL, or a URL): `Ok(None)` when it answered completely
    /// and has no such branch, `Err` with a reason when it could not be asked
    /// (unreachable, no credentials, deadline) or its answer was incomplete
    /// or malformed.
    fn live_head(&self, remote: &str, branch: &str) -> Result<Option<String>, String>;
}

/// [`RemoteHeads`] through `git ls-remote`.
#[derive(Debug, Clone)]
pub struct LsRemote<'a> {
    pub base: &'a Path,
    pub deadline: Duration,
}

impl LsRemote<'_> {
    /// [`RemoteHeads::live_head`] with the failure typed.
    pub fn head(&self, remote: &str, branch: &str) -> Result<Option<String>, TransportError> {
        let refname = format!("refs/heads/{branch}");
        let output = run_transport(self.base, &["ls-remote", remote, &refname], self.deadline)?;
        parse_live_head(&output, &refname).map_err(TransportError::other)
    }
}

impl RemoteHeads for LsRemote<'_> {
    fn live_head(&self, remote: &str, branch: &str) -> Result<Option<String>, String> {
        self.head(remote, branch).map_err(|error| error.reason)
    }
}

/// Whether `branch` is a branch name git accepts as written
/// (`check-ref-format --branch`). A name git would expand, such as `@{-1}`,
/// is refused.
pub fn is_valid_branch_name(base: &Path, branch: &str) -> bool {
    !branch.starts_with('-')
        && git_from(base, base, &["check-ref-format", "--branch", branch]).is_ok_and(|normalized| normalized == branch)
}

/// The fetch that brings exactly `refs/remotes/origin/<branch>` up to date.
///
/// `--no-write-fetch-head` and `--no-tags` leave `FETCH_HEAD` and tags alone;
/// `--no-recurse-submodules` keeps a submodule's refs out of it, and an empty
/// `--refmap=` stops git updating other tracking refs from a configured
/// `remote.origin.fetch` (spike S3). The leading `+` accepts a remote rewind.
pub fn fetch_argv(branch: &str) -> Vec<String> {
    [
        "-c",
        "maintenance.auto=false",
        "-c",
        "gc.auto=0",
        "fetch",
        "--no-write-fetch-head",
        "--no-tags",
        "--no-recurse-submodules",
        "--refmap=",
        "origin",
    ]
    .into_iter()
    .map(str::to_string)
    .chain([format!("+refs/heads/{branch}:refs/remotes/origin/{branch}")])
    .collect()
}

/// Runs [`fetch_argv`] for `branch` in `base`, killing it at `deadline`.
///
/// ## Errors
///
/// [`GitFailure::Other`] without running the fetch when `branch` is not a
/// valid branch name ([`is_valid_branch_name`]); otherwise the fetch's own
/// failure.
pub fn fetch_tracking_ref(base: &Path, branch: &str, deadline: Duration) -> Result<(), TransportError> {
    if !is_valid_branch_name(base, branch) {
        return Err(TransportError::other("not a valid branch name"));
    }
    let argv = fetch_argv(branch);
    let args: Vec<&str> = argv.iter().map(String::as_str).collect();
    run_transport(base, &args, deadline).map(drop)
}

/// Unix seconds of the last change to `refs/remotes/origin/<branch>`, from
/// its reflog: `None` when the ref, its reflog, or a parsable entry is
/// missing. It dates a change to the ref, not a fetch or a check.
pub fn tracking_ref_changed_at(base: &Path, branch: &str) -> Option<u64> {
    let refname = format!("refs/remotes/origin/{branch}");
    // An existing ref without a reflog prints nothing and exits 0 (S3).
    git_from(base, base, &["reflog", "-1", "--format=%ct", &refname, "--"])
        .ok()
        .and_then(|output| output.trim().parse().ok())
}

/// Whether `value` is a full git object ID: 40 (SHA-1) or 64 (SHA-256)
/// lowercase hex digits.
pub fn is_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The object ID `ls-remote` reported for exactly `refname`. Every non-empty
/// line must be `<object id>\t<refname>`; a line for another ref (a server
/// that ignores the pattern) is skipped, but a malformed one fails the whole
/// answer.
fn parse_live_head(output: &str, refname: &str) -> Result<Option<String>, String> {
    let mut found = None;
    for line in output.lines().filter(|line| !line.is_empty()) {
        let (sha, name) = line
            .split_once('\t')
            .filter(|(sha, name)| is_object_id(sha) && !name.is_empty())
            .ok_or_else(|| format!("unexpected ls-remote output: {line:?}"))?;
        if name == refname && found.is_none() {
            found = Some(sha.to_string());
        }
    }
    Ok(found)
}

/// Runs `git -C <base> -c credential.interactive=never <args>` with prompts
/// disabled and kills its process tree at `deadline`.
///
/// ## Returns
///
/// All of stdout on success.
///
/// ## Errors
///
/// A readable reason: git's stderr on failure, the deadline, or stdout that
/// could not be read completely.
pub fn run_noninteractive(base: &Path, args: &[&str], deadline: Duration) -> Result<String, String> {
    run_transport(base, args, deadline).map_err(|error| error.reason)
}

/// [`run_noninteractive`] with the failure typed: the deadline is
/// [`GitFailure::Timeout`], a failed exit is [`classify_git_failure`] of its
/// stderr, and everything else is [`GitFailure::Other`].
pub fn run_transport(base: &Path, args: &[&str], deadline: Duration) -> Result<String, TransportError> {
    let mut command = Command::new("git");
    command
        .current_dir(base)
        .arg("-C")
        .arg(base)
        .args(["-c", "credential.interactive=never"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("LC_ALL", "C")
        .env("GIT_SSH_COMMAND", batch_ssh_command(base))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        // Its own process group, so one signal reaches the transport too.
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|e| TransportError::other(format!("could not run git: {e}")))?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());

    let expires = Instant::now() + deadline;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // The pipes close when the transport exits too; bound the wait
                // anyway so an orphan holding them cannot hang `wt`.
                let grace = Duration::from_millis(500);
                if status.success() {
                    return collect(&stdout, grace).map_err(TransportError::other);
                }
                let err = collect(&stderr, grace).unwrap_or_default();
                let err = err.trim();
                return Err(if err.is_empty() {
                    TransportError::other(format!("git exited with {status}"))
                } else {
                    TransportError { failure: classify_git_failure(err), reason: err.to_string() }
                });
            }
            Ok(None) if Instant::now() >= expires => {
                kill_tree(&mut child);
                return Err(TransportError {
                    failure: GitFailure::Timeout,
                    reason: format!("origin did not answer within {} s", deadline.as_secs_f32()),
                });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => {
                kill_tree(&mut child);
                return Err(TransportError::other(format!("could not wait for git: {e}")));
            }
        }
    }
}

/// Reads `pipe` to its end on a thread. A read error is sent as `Err`; a
/// missing pipe sends nothing, which [`collect`] reports as incomplete.
fn drain<R: std::io::Read + Send + 'static>(
    pipe: Option<R>,
) -> mpsc::Receiver<Result<String, String>> {
    let (sender, receiver) = mpsc::channel();
    if let Some(mut pipe) = pipe {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let read = match pipe.read_to_end(&mut bytes) {
                Ok(_) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
                Err(e) => Err(format!("could not read git's output: {e}")),
            };
            let _ = sender.send(read);
        });
    }
    receiver
}

/// The drained output, or `Err` when it did not arrive whole within `grace`.
fn collect(
    receiver: &mpsc::Receiver<Result<String, String>>,
    grace: Duration,
) -> Result<String, String> {
    match receiver.recv_timeout(grace) {
        Ok(read) => read,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err("git's output did not finish after it exited".to_string())
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("git's output could not be read".to_string())
        }
    }
}

fn kill_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        let group = format!("-{}", child.id());
        let _ = Command::new("kill")
            .args(["-KILL", "--", &group])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(windows)]
    {
        // A Job Object cannot be nested inside an SSH session's job, so
        // taskkill's tree walk is the one method that works everywhere.
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// The user's SSH command with `-o BatchMode=yes` appended: `$GIT_SSH_COMMAND`,
/// else `core.sshCommand`, else `ssh`. Extending rather than replacing keeps
/// any identity or proxy options the user configured.
fn batch_ssh_command(base: &Path) -> String {
    let configured = std::env::var("GIT_SSH_COMMAND")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            git_from(base, base, &["config", "--get", "core.sshCommand"])
                .ok()
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| "ssh".to_string());
    format!("{configured} -o BatchMode=yes")
}

#[cfg(test)]
pub(crate) mod tests {
    use std::io::{Read as _, Write as _};
    use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread::JoinHandle;

    use super::*;
    use crate::remove::test_support::TestRepo;

    /// Bounds every wait on a loopback event, far above what git needs (S1
    /// measured tens of milliseconds) and far below the 10 s deadline.
    pub(crate) const FAST: Duration = Duration::from_secs(5);

    /// The minimal response S1 found makes git fail at once without asking.
    const UNAUTHORIZED: &[u8] = b"HTTP/1.1 401 Unauthorized\r\n\
        WWW-Authenticate: Basic realm=\"r\"\r\n\
        Content-Length: 0\r\n\
        Connection: close\r\n\r\n";

    /// A loopback HTTP origin that hands every accepted connection to a
    /// handler on its own thread, and stops that thread when dropped.
    pub(crate) struct Loopback {
        addr: SocketAddr,
        accepted: Arc<AtomicUsize>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl Loopback {
        fn serve(handler: impl Fn(TcpStream) + Send + 'static) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
            let addr = listener.local_addr().unwrap();
            let accepted = Arc::new(AtomicUsize::new(0));
            let stop = Arc::new(AtomicBool::new(false));
            let thread = std::thread::spawn({
                let (accepted, stop) = (accepted.clone(), stop.clone());
                move || {
                    for stream in listener.incoming() {
                        if stop.load(Ordering::SeqCst) {
                            break;
                        }
                        if let Ok(stream) = stream {
                            accepted.fetch_add(1, Ordering::SeqCst);
                            handler(stream);
                        }
                    }
                }
            });
            Self { addr, accepted, stop, thread: Some(thread) }
        }

        /// Answers every request `401` with a Basic challenge (Rule 14).
        pub(crate) fn unauthorized() -> Self {
            Self::serve(|mut stream| {
                // Read the whole request head first: closing with unread
                // input would reset the connection on some platforms.
                let _ = stream.set_read_timeout(Some(FAST));
                let mut head = Vec::new();
                let mut buffer = [0_u8; 1024];
                while !head.windows(4).any(|window| window == b"\r\n\r\n") {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => return,
                        Ok(read) => head.extend_from_slice(&buffer[..read]),
                    }
                }
                let _ = stream.write_all(UNAUTHORIZED);
                let _ = stream.shutdown(Shutdown::Write);
            })
        }

        /// Accepts and never answers (Rule 13); each held connection is sent
        /// to the receiver, which keeps it open until the test drops it.
        pub(crate) fn holding() -> (Self, mpsc::Receiver<TcpStream>) {
            let (sender, receiver) = mpsc::channel();
            let server = Self::serve(move |stream| {
                let _ = sender.send(stream);
            });
            (server, receiver)
        }

        pub(crate) fn url(&self) -> String {
            format!("http://{}/r.git", self.addr)
        }

        pub(crate) fn accepted(&self) -> usize {
            self.accepted.load(Ordering::SeqCst)
        }
    }

    impl Drop for Loopback {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            // Wakes the blocked `accept` so the thread sees the flag.
            let _ = TcpStream::connect(self.addr);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    /// A repository whose `origin` is `server`, with no proxy and no
    /// credential helper whatever the user's global or system config says:
    /// an empty `http.proxy` makes git disable proxying outright (it also
    /// overrides `*_proxy` in the inherited environment), and an empty
    /// `credential.helper` resets the helper list, so no keychain is asked or
    /// written. A global `url.*.insteadOf` for `http://127.0.0.1` would still
    /// apply; none exists on the build hosts or in CI.
    pub(crate) fn http_origin(server: &Loopback) -> TestRepo {
        let repo = TestRepo::new();
        repo.git(&["remote", "add", "origin", &server.url()]);
        repo.git(&["config", "http.proxy", ""]);
        repo.git(&["config", "credential.helper", ""]);
        repo
    }

    #[test]
    fn reads_live_heads_and_reports_absent_branches() {
        let repo = TestRepo::with_origin();
        let heads = LsRemote {
            base: &repo.path(),
            deadline: LIVE_CHECK_DEADLINE,
        };
        assert_eq!(heads.live_head("origin", "main").unwrap(), Some(repo.sha("main")));
        assert_eq!(heads.live_head("origin", "no-such-branch").unwrap(), None);

        // A push elsewhere is visible live without a fetch.
        let pushed = repo.push_commit_to_origin("main", "other.txt");
        assert_eq!(heads.live_head("origin", "main").unwrap(), Some(pushed));
    }

    #[test]
    fn a_missing_origin_is_an_error_not_an_absent_branch() {
        let repo = TestRepo::new();
        let heads = LsRemote {
            base: &repo.path(),
            deadline: LIVE_CHECK_DEADLINE,
        };
        assert!(heads.live_head("origin", "main").is_err());

        let unreachable = TestRepo::new();
        unreachable.git(&["remote", "add", "origin", "/nonexistent/origin.git"]);
        let heads = LsRemote {
            base: &unreachable.path(),
            deadline: LIVE_CHECK_DEADLINE,
        };
        assert!(heads.live_head("origin", "main").is_err());
    }

    const SHA1: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn only_the_exact_ref_counts_and_an_empty_answer_is_absence() {
        let sha256 = "a".repeat(64);
        let output = format!(
            "{SHA1}\trefs/heads/x/main\n{sha256}\trefs/heads/main\n"
        );
        assert_eq!(
            parse_live_head(&output, "refs/heads/main").unwrap(),
            Some(sha256)
        );
        // `refs/heads/x/main` alone is not `refs/heads/main`.
        let nested = format!("{SHA1}\trefs/heads/x/main\n");
        assert_eq!(parse_live_head(&nested, "refs/heads/main").unwrap(), None);
        assert_eq!(parse_live_head("", "refs/heads/main").unwrap(), None);
    }

    #[test]
    fn a_malformed_line_is_an_error_not_an_absent_branch() {
        for output in [
            "garbage\n",
            &format!("{SHA1} refs/heads/main\n"),
            &format!("{SHA1}\t\n"),
            // A truncated line from a cut-off answer.
            &SHA1[..20],
            // Right line present, but another line is malformed.
            &format!("{SHA1}\trefs/heads/main\nnot-a-line\n"),
        ] {
            assert!(
                parse_live_head(output, "refs/heads/main").is_err(),
                "{output:?} must be an error"
            );
        }
    }

    #[test]
    fn a_wrong_length_or_uppercase_object_id_is_an_error() {
        for sha in [
            &SHA1[..39],
            &format!("{SHA1}0"),
            &SHA1.to_uppercase(),
            &"a".repeat(63),
            &"g".repeat(40),
        ] {
            let output = format!("{sha}\trefs/heads/main\n");
            assert!(
                parse_live_head(&output, "refs/heads/main").is_err(),
                "{sha} must be rejected"
            );
        }
        assert!(is_object_id(SHA1));
        assert!(is_object_id(&"f".repeat(64)));
    }

    #[test]
    fn unreadable_or_unfinished_output_is_an_error() {
        struct Broken;
        impl std::io::Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("pipe broke"))
            }
        }
        let grace = Duration::from_secs(5);
        assert!(collect(&drain(Some(Broken)), grace).is_err());
        assert!(collect(&drain(None::<Broken>), grace).is_err());

        // A sender that never finishes (a transport still holding the pipe).
        let (_held, receiver) = mpsc::channel();
        assert!(collect(&receiver, Duration::from_millis(20)).is_err());

        assert_eq!(
            collect(&drain(Some(&b"complete"[..])), grace).unwrap(),
            "complete"
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_deadline_kills_a_hung_transport() {
        let repo = TestRepo::new();
        // An SSH command that never answers stands in for a black-hole host.
        let hang = repo.path().join("hang.sh");
        std::fs::write(&hang, "#!/bin/sh\nsleep 30\n").unwrap();
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&hang, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        repo.git(&["remote", "add", "origin", "ssh://example.invalid/repo.git"]);
        repo.git(&["config", "core.sshCommand", hang.to_str().unwrap()]);

        let started = Instant::now();
        let result = run_noninteractive(
            &repo.path(),
            &["ls-remote", "origin"],
            Duration::from_millis(500),
        );
        let elapsed = started.elapsed();
        assert!(result.unwrap_err().contains("did not answer"));
        assert!(elapsed < Duration::from_secs(5), "took {elapsed:?}");
    }

    #[test]
    fn an_unauthorized_origin_fails_fast_without_a_prompt() {
        let server = Loopback::unauthorized();
        let repo = http_origin(&server);
        let heads = LsRemote { base: &repo.path(), deadline: Duration::from_secs(10) };

        // stdin is null in the transport and nextest gives no TTY, so a
        // prompt could only show up as a hang until the deadline.
        let started = Instant::now();
        let error = heads.live_head("origin", "main").unwrap_err();
        let elapsed = started.elapsed();

        assert!(elapsed < FAST, "took {elapsed:?}: {error}");
        assert!(!error.contains("did not answer"), "{error}");
        assert!(server.accepted() >= 1, "git never reached the server: {error}");
    }

    /// Reads `stream` until the peer closes it, or fails after [`FAST`].
    fn wait_for_close(mut stream: TcpStream) -> Duration {
        let started = Instant::now();
        stream.set_read_timeout(Some(FAST)).unwrap();
        let mut buffer = [0_u8; 1024];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => return started.elapsed(),
                // The request itself, still buffered.
                Ok(_) => {}
                Err(e) if matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                ) => return started.elapsed(),
                Err(e) => panic!("the connection stayed open after the deadline: {e}"),
            }
        }
    }

    #[test]
    fn an_unauthorized_origin_is_a_typed_credentials_failure() {
        let server = Loopback::unauthorized();
        let repo = http_origin(&server);
        let heads = LsRemote { base: &repo.path(), deadline: Duration::from_secs(10) };

        let error = heads.head("origin", "main").unwrap_err();
        assert_eq!(error.failure, GitFailure::Credentials, "{error}");

        let error = fetch_tracking_ref(&repo.path(), "main", Duration::from_secs(10)).unwrap_err();
        assert_eq!(error.failure, GitFailure::Credentials, "{error}");
    }

    #[test]
    fn the_classifier_maps_each_recorded_sample_and_defaults_to_other() {
        // Spike S3's `LC_ALL=C` first lines; URLs as git prints them.
        let credentials = [
            "fatal: unable to get password from user",
            "fatal: could not read Username for 'https://h.example/r.git': terminal prompts disabled",
            "fatal: could not read Password for 'https://u@h.example/r.git': terminal prompts disabled",
            "fatal: Authentication failed for 'https://h.example/r.git/'",
            "git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.",
            "fatal: unable to access 'https://h.example/r.git/': The requested URL returned error: 401",
        ];
        for stderr in credentials {
            assert_eq!(classify_git_failure(stderr), GitFailure::Credentials, "{stderr}");
        }
        let other = [
            "error: cannot lock ref 'refs/remotes/origin/main': Unable to create '/r/.git/refs/remotes/origin/main.lock': File exists.",
            "fatal: unable to access 'https://h.example/r.git/': The requested URL returned error: 403",
            "fatal: repository 'https://h.example/r.git/' not found",
            "fatal: unable to access 'https://h.example/r.git/': Failed to connect to h.example port 443: Couldn't connect to server",
            "fatal: unable to access 'https://h.example/r.git/': Could not resolve host: h.example",
            "Host key verification failed.\nfatal: Could not read from remote repository.",
            // A deadline has no stderr; its message is never classified.
            "origin did not answer within 10 s",
            "",
        ];
        for stderr in other {
            assert_eq!(classify_git_failure(stderr), GitFailure::Other, "{stderr}");
        }
    }

    #[test]
    fn the_fetch_argv_is_the_ruled_command_with_one_refspec_argument() {
        assert_eq!(
            fetch_argv("feat/x"),
            [
                "-c",
                "maintenance.auto=false",
                "-c",
                "gc.auto=0",
                "fetch",
                "--no-write-fetch-head",
                "--no-tags",
                "--no-recurse-submodules",
                "--refmap=",
                "origin",
                "+refs/heads/feat/x:refs/remotes/origin/feat/x",
            ]
        );
    }

    /// Every ref that is not a symbolic ref (`origin/HEAD` follows
    /// `origin/main`), and `FETCH_HEAD`'s bytes, to compare around a fetch.
    fn snapshot(repo: &TestRepo) -> (String, Option<Vec<u8>>) {
        let refs = repo.git(&["for-each-ref", "--format=%(if)%(symref)%(then)%(else)%(refname) %(objectname)%(end)"]);
        let fetch_head = std::fs::read(repo.path().join(".git").join("FETCH_HEAD")).ok();
        (refs, fetch_head)
    }

    #[test]
    fn a_fetch_updates_only_the_one_tracking_ref() {
        let repo = TestRepo::with_origin();
        repo.git(&["push", "-q", "origin", "main:refs/heads/feature"]);
        repo.git(&["fetch", "-q", "origin"]);
        std::fs::write(repo.path().join(".git").join("FETCH_HEAD"), "sentinel\n").unwrap();
        // A configured extra refspec and pruning must not widen the fetch.
        repo.git(&["config", "--add", "remote.origin.fetch", "+refs/heads/*:refs/remotes/mirror/*"]);
        repo.git(&["config", "fetch.prune", "true"]);

        let main = repo.push_commit_to_origin("main", "upstream.txt");
        repo.push_commit_to_origin("feature", "feature.txt");
        let pusher = repo.path().parent().unwrap().join("pusher");
        repo.git_in(&pusher, &["tag", "v2"]);
        repo.git_in(&pusher, &["push", "-q", "origin", "v2"]);
        let (before_refs, before_head) = snapshot(&repo);

        fetch_tracking_ref(&repo.path(), "main", Duration::from_secs(10)).unwrap();

        let (after_refs, after_head) = snapshot(&repo);
        assert_eq!(repo.sha("origin/main"), main);
        assert_eq!(after_head, before_head, "FETCH_HEAD is untouched");
        let changed: Vec<_> = after_refs
            .lines()
            .filter(|line| !line.is_empty() && !before_refs.lines().any(|before| before == *line))
            .collect();
        assert_eq!(changed, [format!("refs/remotes/origin/main {main}")], "only origin/main moved");
        assert!(repo.try_git(&["rev-parse", "--verify", "--quiet", "refs/tags/v2"]).is_err(), "no tag arrived");
    }

    #[test]
    fn a_remote_rewind_is_applied() {
        let repo = TestRepo::with_origin();
        let first = repo.sha("main");
        repo.push_commit_to_origin("main", "upstream.txt");
        fetch_tracking_ref(&repo.path(), "main", Duration::from_secs(10)).unwrap();
        assert_ne!(repo.sha("origin/main"), first);

        repo.git_in(&repo.origin_path(), &["update-ref", "refs/heads/main", &first]);
        fetch_tracking_ref(&repo.path(), "main", Duration::from_secs(10)).unwrap();

        assert_eq!(repo.sha("origin/main"), first, "the leading + accepted the rewind");
    }

    #[test]
    fn an_invalid_branch_name_is_refused_before_any_request() {
        let server = Loopback::unauthorized();
        let repo = http_origin(&server);
        for branch in ["ma..in", "-x", "@{-1}", "a b", "main.lock", "", "feat/x:refs/heads/y"] {
            assert!(!is_valid_branch_name(&repo.path(), branch), "{branch:?}");
            let error = fetch_tracking_ref(&repo.path(), branch, Duration::from_secs(10)).unwrap_err();
            assert_eq!(error.failure, GitFailure::Other, "{branch:?}");
        }
        assert_eq!(server.accepted(), 0, "no invalid name reached the origin");
        for branch in ["main", "feat/x", "ü"] {
            assert!(is_valid_branch_name(&repo.path(), branch), "{branch:?}");
        }
    }

    #[test]
    fn a_fetch_past_its_deadline_is_a_typed_timeout() {
        let (server, _held) = Loopback::holding();
        let repo = http_origin(&server);

        let started = Instant::now();
        let error = fetch_tracking_ref(&repo.path(), "main", Duration::from_millis(500)).unwrap_err();

        assert_eq!(error.failure, GitFailure::Timeout, "{error}");
        assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
    }

    #[test]
    fn the_tracking_ref_reflog_dates_its_last_change_or_is_none() {
        let repo = TestRepo::with_origin();
        let changed = tracking_ref_changed_at(&repo.path(), "main").expect("push -u logged origin/main");
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        assert!(changed <= now && now - changed < 600, "{changed} vs {now}");

        assert_eq!(tracking_ref_changed_at(&repo.path(), "no-such-branch"), None, "no ref");

        // A ref whose reflog is gone (deleted, or logging disabled) is None.
        let log = repo.path().join(".git").join("logs").join("refs").join("remotes").join("origin").join("main");
        std::fs::remove_file(log).unwrap();
        repo.git(&["config", "core.logAllRefUpdates", "false"]);
        assert!(repo.try_git(&["rev-parse", "--verify", "origin/main"]).is_ok(), "the ref itself remains");
        assert_eq!(tracking_ref_changed_at(&repo.path(), "main"), None, "no reflog");
    }

    #[test]
    fn the_deadline_kills_the_http_transport_and_closes_its_connection() {
        let (server, held) = Loopback::holding();
        let repo = http_origin(&server);

        let started = Instant::now();
        let result = run_noninteractive(
            &repo.path(),
            &["ls-remote", "origin", "refs/heads/main"],
            Duration::from_millis(500),
        );
        let elapsed = started.elapsed();
        assert!(result.unwrap_err().contains("did not answer"));
        assert!(elapsed < FAST, "took {elapsed:?}");

        // `git-remote-http` holds the socket, not `git`: only a tree kill
        // closes it (S1).
        let connection = held.recv_timeout(FAST).expect("git connected to the origin");
        let closed_after = wait_for_close(connection);
        assert!(closed_after < FAST, "closed after {closed_after:?}");
        assert_eq!(server.accepted(), 1);
    }
}
