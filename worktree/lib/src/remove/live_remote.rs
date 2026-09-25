//! Non-interactive, deadline-bound git network calls against `origin`.
//!
//! Every call runs with credential prompts disabled (`GIT_TERMINAL_PROMPT=0`,
//! `credential.interactive=never`, `GCM_INTERACTIVE=never`) and SSH in batch
//! mode, so a missing credential fails instead of prompting. At the deadline
//! the whole process tree is killed: `git` runs its transport (`ssh`,
//! `git-remote-https`) in a grandchild that would otherwise keep the output
//! pipe open, and on Windows the `git.exe` launcher's real git would keep
//! running (spike S2).

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::git::git_from;

/// The ruled deadline for one live `ls-remote` (Decision 21).
pub const LIVE_CHECK_DEADLINE: Duration = Duration::from_secs(3);

/// Deadline for the lease-protected deletion push.
pub const PUSH_DEADLINE: Duration = Duration::from_secs(30);

/// Reads live branch heads on `origin`.
///
/// A trait so the tier logic can be tested with scripted answers.
pub trait RemoteHeads: Sync {
    /// The live SHA of `refs/heads/<branch>` on origin: `Ok(None)` when origin
    /// answered and has no such branch, `Err` with a reason when origin could
    /// not be asked (unreachable, no credentials, deadline).
    fn live_head(&self, branch: &str) -> Result<Option<String>, String>;
}

/// [`RemoteHeads`] through `git ls-remote origin`.
#[derive(Debug, Clone)]
pub struct LsRemote<'a> {
    pub base: &'a Path,
    pub deadline: Duration,
}

impl RemoteHeads for LsRemote<'_> {
    fn live_head(&self, branch: &str) -> Result<Option<String>, String> {
        let refname = format!("refs/heads/{branch}");
        let output = run_noninteractive(
            self.base,
            &["ls-remote", "origin", &refname],
            self.deadline,
        )?;
        Ok(output.lines().find_map(|line| {
            let (sha, name) = line.split_once('\t')?;
            (name == refname).then(|| sha.to_string())
        }))
    }
}

/// Runs `git -C <base> -c credential.interactive=never <args>` with prompts
/// disabled and kills its process tree at `deadline`.
///
/// ## Returns
///
/// Stdout on success.
///
/// ## Errors
///
/// A readable reason: git's stderr on failure, or the deadline.
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
                let out = stdout.recv_timeout(grace).unwrap_or_default();
                if status.success() {
                    return Ok(out);
                }
                let err = stderr.recv_timeout(grace).unwrap_or_default();
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

fn drain<R: std::io::Read + Send + 'static>(pipe: Option<R>) -> mpsc::Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    if let Some(mut pipe) = pipe {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            let _ = sender.send(String::from_utf8_lossy(&bytes).into_owned());
        });
    }
    receiver
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
mod tests {
    use super::*;
    use crate::remove::test_support::TestRepo;

    #[test]
    fn reads_live_heads_and_reports_absent_branches() {
        let repo = TestRepo::with_origin();
        let heads = LsRemote {
            base: &repo.path(),
            deadline: LIVE_CHECK_DEADLINE,
        };
        assert_eq!(heads.live_head("main").unwrap(), Some(repo.sha("main")));
        assert_eq!(heads.live_head("no-such-branch").unwrap(), None);

        // A push elsewhere is visible live without a fetch.
        let pushed = repo.push_commit_to_origin("main", "other.txt");
        assert_eq!(heads.live_head("main").unwrap(), Some(pushed));
    }

    #[test]
    fn a_missing_origin_is_an_error_not_an_absent_branch() {
        let repo = TestRepo::new();
        let heads = LsRemote {
            base: &repo.path(),
            deadline: LIVE_CHECK_DEADLINE,
        };
        assert!(heads.live_head("main").is_err());

        let unreachable = TestRepo::new();
        unreachable.git(&["remote", "add", "origin", "/nonexistent/origin.git"]);
        let heads = LsRemote {
            base: &unreachable.path(),
            deadline: LIVE_CHECK_DEADLINE,
        };
        assert!(heads.live_head("main").is_err());
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
}
