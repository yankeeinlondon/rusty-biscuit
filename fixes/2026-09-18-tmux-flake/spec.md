---
created: 2026-09-18
status: draft
implemented: false
area: biscuit-test-harness
related:
  - 2026-09-18-ci-cadence
  - 2026-09-18-long-hooks-dropped
---

# The shared tmux session can vanish between spawn and first use

## Problem

Under the pre-push hook on 2026-09-18 (branch `feat/single-os`, head
269fc596c), two L2 suites failed on their **first** test with the same error
and then passed unchanged when run on their own:

```
darkmatter-cli::level2_code_block_styling level2_align_code_block_center_indents_more_than_left
    panicked at darkmatter/cli/tests/common/level2.rs:198:10:
    send_command_with_env failed: Custom { kind: Other, error: "tmux send-keys failed" }
    FAIL [ 0.069s ] ( 1/69 )

sniff-cli::level2_cicd_styling level2_cicd_status_cells_render_styled_in_tmux
    panicked at sniff/cli/tests/level2_cicd_styling.rs:52:10:
    send_command_with_env failed: Custom { kind: Other, error: "tmux send-keys failed" }
    FAIL [ 0.066s ] ( 1/2 )
```

Both cost a blocked push and a full re-run of the hook. In the same hook run
the L2 suites of `claudine-cli` and `dmls`, which use the same broker and the
same backend, passed. Each of the two failing suites passed 69 of 69 and 2 of
2 when run directly afterwards with `just _test_l2 <pkg>`.

The failure is a harness fault, not a test result: the test's first
`send_text` reached a session that no longer existed, less than a tenth of a
second after the test binary started.

## Evidence

From the hook log of that push (`_test_l2 darkmatter-cli`):

```
✅ all tests passing in darkmatter-cli package
🖥  Level-2 tests for darkmatter-cli
backend-proof: cleared …/backend-executions.jsonl
Connection to ssh.github.com closed by remote host.
  spawned wezterm pane 716
  spawned tmux session biscuit_test_9438_0
  spawned apple-terminal window 7155
    Blocking waiting for file lock on package cache
  … (nextest builds the test binaries)
FAIL [ 0.069s ] ( 1/69 ) … tmux send-keys failed
backend-proof: tmux run=1 skip=0 panic=0
```

So the broker's `tmux new-session -d` succeeded and printed the session
name, the recipe exported it as `BISCUIT_SHARED_TMUX_SESSION`, nextest spent
some minutes compiling, and by the time the first test attached, the session
was gone. `tmux ls` on the host afterwards reported `no server running`.

What the code does today:

- `just _test_l2` (`just/devops.just`) spawns one shared pane per backend
  through `biscuit-harness-broker spawn tmux` and registers an `EXIT` trap
  that runs `broker kill tmux <id>` for it. `broker kill` is
  `tmux::kill_session_by_name`, a best-effort `tmux kill-session -t <name>`
  whose errors are swallowed.
- `TmuxHarness::spawn_shell` builds `tmux new-session -d -s <name> -x -y -e …
  <login shell>` (`biscuit-test-harness/src/tmux.rs`, `new_session_args`).
  `run_new_session` retries **once** when `new-session` itself fails with
  "server exited unexpectedly". Nothing verifies the session after
  `new-session` returns.
- `send_text` runs `tmux send-keys -t <session> -l …` and turns any non-zero
  exit into the opaque `tmux send-keys failed`, discarding tmux's stderr, so
  the log does not say *why* (`no server running` versus `session not
  found` versus a pane that lost its shell).
- A tmux server exits on its own when its last session closes. The hook runs
  the L2 recipe once per package, back to back, so each package's trap kills
  the only session and the server begins shutting down just before the next
  package's broker spawns a new one.

Host: macOS 27.0, tmux 3.7b, the development Mac under the pre-push hook
(so with other L2 backends, WezTerm and Apple Terminal, being spawned in
the same seconds, and Cargo contending for the package cache lock).

## Cause, established by PR #85

PR #85 (`feat/better-sniff`, opened 2026-09-18 from a parallel session)
reproduced and fixed the mechanism before this spec was reviewed: the shared
session is spawned by `biscuit-harness-broker`, which exits at once, so its
name carried a dead pid; every harness spawn runs a stale-resource reaper
that kills tagged sessions whose owning pid is dead, guarded only for the
reaping process's own shared session. A concurrent test run from another
worktree on the same host reaped this run's session under its tests. That is
hypothesis 3 below, in its harness-owned form. #85 tags shared panes with the
run's `BISCUIT_HARNESS_OWNER_PID` and makes a rejected `send-keys` report
tmux's own diagnostic and the live session list — items (2) and (3) of the
fix below in different clothing.

What remains for this spec once #85 is merged: the post-spawn and
on-attach `has-session` verification (fix item 1), which turns any future
loss of the session, whatever its cause, into a retried spawn or a named
failure at the point of attachment rather than an opaque first-test failure.
Re-run the non-vacuous proof against the merged harness before deciding
whether that item is still worth its closure cost.

## Hypotheses

Ranked by how well they fit the evidence as this spec was first written; #85
settled it on the third. Kept so the next reader sees what the evidence did
and did not distinguish.

1. **Server hand-off race.** The previous package's `kill-session` leaves a
   server that is still tearing down when the next `new-session -d` connects
   to its socket. The session is created on the dying server and disappears
   with it. This is the same class of failure that the existing one-shot
   retry in `run_new_session` was written for, but that retry only covers the
   case where `new-session` reports the exit; it does not cover a server that
   exits *after* answering.
2. **The login shell exits.** `new-session` runs the user's login shell
   (`login_shell_argv`). If that shell exits — a profile that fails under the
   hook's environment, a `SIGHUP` from the terminal the hook was launched in
   (note the SSH connection closing in the same seconds), or an `exec` that
   fails — the session closes and, being the last, takes the server with it.
3. **Cross-backend interference.** WezTerm and Apple Terminal spawns run in
   the same window; an Apple Terminal cleanup sweep or a WezTerm workspace
   operation that touches the default tmux socket, or the harness's own
   `Drop` from a *different* process reaping by name, could kill the session.
   Least likely: the session names are unique per process and sequence.

## Fix

Three small, independent changes in `biscuit-test-harness`, all
behavior-preserving on the success path:

1. **Verify after spawn, and once more before first use.** After
   `new-session` succeeds, `spawn_shell` runs `tmux has-session -t <name>`
   and, on failure, re-creates the session once (a second server start is
   cheap). `TmuxHarness::attach` (the `BISCUIT_SHARED_*` path every shared
   test takes) performs the same check when it constructs, so the *test*
   fails with "shared session `<name>` is gone" rather than the first
   `send-keys` failing opaquely.
2. **Keep the server alive across packages.** `_test_l2` starts, and its
   trap ends, a `biscuit-broker-keepalive` session (`tmux new-session -d -s
   … 'sleep infinity'`) around the whole tier when it runs under
   `just ci-local`, so killing a package's session never makes the server the
   last-session-out. Equivalent: the broker's `kill` can `kill-session` and
   then, if that was the last session, wait for the server socket to
   disappear before returning. The keepalive is simpler to reason about and
   costs one idle process.
3. **Say why.** `send_text`, `capture`, and `kill_session_by_name` keep
   tmux's stderr in the error (`no server running on /private/tmp/tmux-501/
   default`, `can't find session`), and `spawn_shell` logs the tmux server
   PID it created. The next occurrence then names its hypothesis.

Together: (1) turns a lost session into a retried spawn or a clear message,
(2) removes the race in hypothesis 1 outright, and (3) is what tells us if
hypothesis 2 or 3 is real.

## Non-vacuous proof

- A unit test drives `run_new_session` against a fake `tmux` on `PATH` whose
  `new-session` succeeds and whose first `has-session` fails: the harness
  retries once and succeeds. The same fake with a second failure yields an
  error naming the session and tmux's stderr, not `tmux send-keys failed`.
- An L2 test under `just test-l2` that kills its own shared session
  (`tmux kill-session`) and then calls `send_text` must fail with the
  named error, proving the diagnostics reach a real backend.
- The hook-shaped reproduction: `for p in dmls darkmatter-cli sniff-cli; do
  just _test_l2 $p …; done` on the development Mac, ten iterations, zero
  first-test failures. Record the count in this spec when done.

## Cost

The change sits in `biscuit-test-harness`, a dev-dependency of nearly every
package, so the push that lands it re-runs most of the hook's cells locally
(the closure rule includes a dependency's tree). Land it alone, on a quiet
branch, and not stacked with unrelated work.

## Out of scope

- Parallel L2 (`BISCUIT_L2_THREADS`): the shared-session hazard is different
  there and already documented in the rust-testing skill.
- The `Blocking waiting for file lock on package cache` line: Cargo lock
  contention with a concurrent build on the same host slows the spawn window
  but cannot kill a session; noted only because it lengthens the window in
  which hypothesis 1 and 2 can occur.
