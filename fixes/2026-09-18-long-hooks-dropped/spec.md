---
created: 2026-09-18
status: draft
implemented: false
area: repository-ci
related:
  - 2026-09-18-ci-cadence
  - 2026-09-18-tmux-flake
---

# A long pre-push hook outlives the SSH transport, and the push silently fails

## Problem

When the pre-push hook runs long enough — three times on 2026-09-18, each
after a whole-workspace or near-whole-workspace hook run of 45 to 55 minutes
— `git push` ends like this:

```
Published 28 macos-latest cell(s) of evidence for 97b50fd6f (base a94c61669). …
Pre-push validation passed.
$ echo $?
141
$ git ls-remote --heads origin feat/single-os
269fc596c…   refs/heads/feat/single-os        # the OLD head
```

The hook passed. The evidence note it pushes went through, on its own
connection. The branch ref did not move, and Git exited 141 (`SIGPIPE`).
Nothing in the output says the push failed; the only signal is the exit code,
which the hook's own wrapper does not surface, and the remote's head. A
developer who trusts "Pre-push validation passed" walks away with an unpushed
branch, and every pull request that depended on it waits.

A second `git push` immediately afterwards succeeds in about a minute: the
hook reuses the receipt it just published and the transfer completes.

## Evidence

Observed on the development Mac (macOS 27.0, git 2.55.0, OpenSSH 10.3p1),
remote `git@github.com:yankeeinlondon/rusty-biscuit.git`, no `ServerAlive*`
or `ControlMaster` settings in `~/.ssh/config`:

| push | hook duration | hook result | git exit | ref moved |
|---|---|---|---|---|
| 22207a18d (first) | ~45 min, whole workspace | passed | 141 | no |
| 407386afc | ~50 min | **failed** on a suite | 141 | no (correct) |
| 97b50fd6f (first) | ~55 min after a main merge | passed | 141 | no |
| the retry of each | ~1 min, evidence reused | passed | 0 | yes |
| every push under ~15 min | — | passed | 0 | yes |

One hook log captured the moment it happened, about twenty minutes in,
between two L2 suites:

```
🖥  Level-2 tests for darkmatter-cli
backend-proof: cleared …
Connection to ssh.github.com closed by remote host.
  spawned wezterm pane 716
```

That line is OpenSSH's, printed by the `ssh` process Git started for the
push. Git opens the transport **before** running `pre-push`: it needs the
remote's ref advertisement to hand the hook its `<remote sha>` column. The
connection then sits idle, carrying nothing, for the whole hook. GitHub's SSH
front end closes an idle connection well inside an hour. When the hook
finally returns 0, Git writes the pack to a closed pipe and dies with
`SIGPIPE`; when the hook returns 1, Git still gets 141 from tearing the
transport down (second row), which is why the exit code alone cannot say
whether the hook or the transport failed.

The `Connection … closed by remote host` line also explains the 2026-09-17
handoff note "the wrapper exits 0 even when the hook blocks": the wrapper's
exit was never the hook's, and the dispatcher installed in `.git/hooks` is a
plain `exec` that adds nothing of its own.

## What this is not

- Not the hook's fault: it completed, published its evidence on a fresh
  connection (`push_note`), and printed its verdict.
- Not fixed by `--no-verify`: that skips the validation, which is the wrong
  trade; the rust-devops skill's bypass-mode table already says so.
- Not a GitHub outage: the same push succeeds a minute later.

## Research first (budget: one focused session, about half a day)

The fix depends on facts this session did not establish, and the cheapest
candidate is a one-line SSH setting whose effect has to be *measured*, not
assumed. Spend the research time before writing code:

1. **Measure GitHub's idle timeout.** `ssh -v git@github.com` held open
   with a no-op (`ssh git@github.com` prints the greeting and exits, so use
   a `git push` against a scratch branch with a `pre-push` hook that sleeps
   N minutes) for N = 10, 20, 30, 45. Record the largest N that still pushes.
   Repeat with `ssh.github.com` on port 443, since the captured line names
   that host and the two front ends may differ.
2. **Test client keepalives.** Add to `~/.ssh/config`:

   ```
   Host github.com ssh.github.com
     ServerAliveInterval 60
     ServerAliveCountMax 120
   ```

   and repeat step 1. If a 60-minute sleeping hook now pushes, this is the
   fix and the rest of this spec is documentation. Record whether GitHub
   answers the keepalives at all — some front ends count them as idle.
3. **Confirm Git's transport ordering** from `GIT_TRACE=1 GIT_TRACE_PACKET=1
   git push` on the scratch branch: the ref advertisement arrives before
   `pre-push` starts, and no traffic follows until the hook exits. If Git
   ever reconnects on its own after a dead transport (newer Git may), the
   symptom changes and this spec should say so.
4. **Decide the fallback surface** if keepalives are not honored: whether
   Git's `core.sshCommand` can carry the options per repository (so no
   developer has to edit `~/.ssh/config`), and whether `just init` should
   set it.

Each step is a shell session against a scratch branch; none needs CI.

## Fix candidates, in order of preference

1. **Client keepalives, set by the repository.** If research step 2 holds,
   `just init` sets `core.sshCommand` to `ssh -o ServerAliveInterval=60 -o
   ServerAliveCountMax=120` for this repository, and `.claude/skills/
   rust-devops/ci-cd.md` documents it beside the existing bypass-modes
   note. No hook change. Zero cost on short pushes.
2. **Make the failure loud.** Independently of (1): the hook already knows
   how long it ran. When it passes after more than a configurable threshold
   (say ten minutes), it prints one line naming the risk and the exact
   command to verify — `git ls-remote --heads origin <branch>` — and the
   `pre-push` recipe's summary says `pushed` or `NOT pushed` by re-reading
   the remote after Git returns. Git cannot tell the hook the push failed,
   but `just push`-style wrappers can check.
3. **Shorten what the transport has to survive.** The
   `2026-09-18-ci-cadence` decisions already cut the common case; the
   remaining long hooks come from the closure rule that re-tests every
   dev-dependent of a changed test-toolkit or test-harness file. That is a
   separate decision (narrow the closure to non-test paths of a dependency)
   and belongs in its own fix; it reduces exposure but does not remove it,
   because a genuine whole-workspace change still runs an hour.
4. **Reconnect after the hook.** Git offers no way to re-establish the
   transport post-hook. Do not build a wrapper that runs the hook itself and
   then pushes with `--no-verify`: it would recreate the receipt-then-push
   contract the hook already implements, and one drift between the two
   would ship untested code.

## Acceptance criteria

- A `pre-push` hook that sleeps for 60 minutes on a scratch branch, with the
  chosen fix in place, is followed by a push whose ref moves, on both
  `github.com:22` and `ssh.github.com:443`. Record the measured idle timeout
  without the fix in this spec.
- After a hook run longer than the threshold, the terminal states whether
  the branch reached the remote, in words, before the shell prompt returns.
- The rust-devops skill's bypass-modes section replaces the 2026-09-17
  "retry the push" paragraph with the fix and the measured numbers.
- The `.githooks/tests/test-pre-push.sh` suite pins the loud-failure line
  (it stubs Git, so it can simulate a passed hook whose push did not land).

## Out of scope

- HTTPS remotes: the transport is created per request there and the symptom
  cannot occur; switching the repository's remote is not proposed.
- The 2026-09-17 note in the skill stays until the acceptance criteria
  replace it; it is correct as far as it goes.
