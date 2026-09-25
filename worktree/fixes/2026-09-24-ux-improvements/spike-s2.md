---
spike: S2
date: 2026-09-24
---

# Spike S2: non-interactive `git ls-remote` under a deadline

A throwaway probe crate spawned `git -c credential.interactive=never ls-remote <url> refs/heads/main` with
`GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=never`, `GIT_SSH_COMMAND="<base> -o BatchMode=yes"`, stdin null,
stdout and stderr piped to reader threads, polled `try_wait` against a 3 s deadline, and killed on timeout.
The same binary ran on this macOS host (git 2.55.0), `$BUILD_LINUX` (git 2.47.3), and `$BUILD_WIN`
(git 2.55.0.windows.3, Git Credential Manager as the system `credential.helper`). The probe was deleted afterwards.
Unreachable hosts used `192.0.2.1` (TEST-NET-1, which drops packets, so connect hangs).

## Results

| Case | macOS | Linux | Windows |
|---|---|---|---|
| HTTPS public repo | 0 in 0.36 s | 0 in 0.34 s | 0 in 0.84 s |
| HTTPS, repo needs credentials (401) | 128 in 0.14 s, no prompt | 128 in 0.12 s, no prompt | 128 in 0.68 s, no GCM dialog |
| SSH, agent key works | 0 in 1.1 s | 0 in 0.96 s | 0 in 1.6 s |
| SSH, unknown host key (`UserKnownHostsFile=/dev/null`) | 128 in 0.31 s ("Host key verification failed") | — | — |
| SSH, passphrase-protected key, no agent (`-F /dev/null -o IdentityAgent=none`) | 128 in 0.50 s ("Permission denied (publickey)"), no passphrase prompt | — | — |
| SSH/HTTPS to a black hole, plain `Child::kill` at 3 s | child reaped at 3.0 s, **stderr pipe held 5 s+** | same | same, **plus orphaned `git.exe` and `git-remote-https.exe` kept running** |
| Same, killing the whole process tree | pipes closed at 3.0 s | pipes closed at 3.0 s | pipes closed at 3.1 s |

## Findings

1. **The R2 environment prevents every prompt observed.** `GIT_TERMINAL_PROMPT=0` stops git's own
   username/password prompt; `BatchMode=yes` stops SSH passphrase and host-key prompts and fails fast;
   `credential.interactive=never` (and `GCM_INTERACTIVE=never`) stops Git Credential Manager's GUI on Windows.
   R2 should add the credential setting; it was not in the plan's wording.
2. **Killing `git` alone is not enough, on every OS.** `git ls-remote` runs its transport in a grandchild
   (`ssh`, `git-remote-https`). `Child::kill` reaps `git`, but the grandchild inherited the stderr pipe, so a
   reader that waits for EOF blocks until the grandchild gives up on its own (about 20 s on Windows, longer
   for a TCP connect timeout on Unix).
3. **Windows is worse:** `git.exe` on `PATH` is the `Git\cmd\git.exe` launcher, which starts the real
   `mingw64\bin\git.exe`. Killing the launcher leaves the real git **and** its transport running. The orphans
   also hold their working directory (the scratch directory could not be deleted until they exited), which
   matters for `wt remove`: an orphaned git started inside a worktree locks it.
4. **Fix that worked:** kill the process tree.
   - Unix: spawn with `CommandExt::process_group(0)` and send `SIGKILL` to the negative PID
     (`libc::killpg`, or `kill -KILL -<pid>` as the probe did).
   - Windows: `taskkill /T /F /PID <pid>` worked, and it is what ran under SSH here. A Job Object with
     `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` avoids the extra process, but the `os` skill records that an
     SSH session's processes already sit in a Job that forbids nesting (`AssignProcessToJobObject`
     returns access denied), so a Job Object needs the `taskkill /T /F` fallback anyway. Simplest
     correct choice: `taskkill /T /F` alone. `windows-sys` is already in the workspace graph if the Job
     Object path is wanted. Either way, never join the reader threads without a timeout after a kill.
5. `GIT_SSH_COMMAND` overrides `core.sshCommand`. To *extend* rather than replace, the base command is
   `$GIT_SSH_COMMAND`, else `git config core.sshCommand`, else `ssh`, followed by ` -o BatchMode=yes`.
6. Run the query with `git -C <base repo>` and a working directory outside every worktree, so even a leaked
   child cannot hold a worktree on Windows.

## Impact on R2

Confirmed, with two additions: add `-c credential.interactive=never` plus `GCM_INTERACTIVE=never`, and kill
the process tree (process group on Unix, Job Object on Windows) at the 3 s deadline. A timeout is
"unavailable", as recommended. The same helper serves `wt remove`'s live `origin/*` check and
`--force-remote`'s pre-deletion query.
