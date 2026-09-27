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

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::git::git_from;

/// The ruled deadline for one live `ls-remote` (Decision 21).
pub const LIVE_CHECK_DEADLINE: Duration = Duration::from_secs(3);

/// Deadline for the lease-protected deletion push.
pub const PUSH_DEADLINE: Duration = Duration::from_secs(30);

/// Reads live branch heads.
///
/// A trait so the tier logic can be tested with scripted answers.
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

impl RemoteHeads for LsRemote<'_> {
    fn live_head(&self, remote: &str, branch: &str) -> Result<Option<String>, String> {
        let refname = format!("refs/heads/{branch}");
        let output = run_noninteractive(
            self.base,
            &["ls-remote", remote, &refname],
            self.deadline,
        )?;
        parse_live_head(&output, &refname)
    }
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
    let mut command = Command::new("git");
    command
        .current_dir(base)
        .arg("-C")
        .arg(base)
        .args(["-c", "credential.interactive=never"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
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
    let mut child = command.spawn().map_err(|e| format!("could not run git: {e}"))?;
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
                    return collect(&stdout, grace);
                }
                let err = collect(&stderr, grace).unwrap_or_default();
                let err = err.trim();
                return Err(if err.is_empty() {
                    format!("git exited with {status}")
                } else {
                    err.to_string()
                });
            }
            Ok(None) if Instant::now() >= expires => {
                kill_tree(&mut child);
                return Err(format!(
                    "origin did not answer within {} s",
                    deadline.as_secs_f32()
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => {
                kill_tree(&mut child);
                return Err(format!("could not wait for git: {e}"));
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
