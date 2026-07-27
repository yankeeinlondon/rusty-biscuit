---
status: root cause CONFIRMED — awaiting design decision
reviewed: false
created: 2026-07-27
area: darkmatter
packages:
  - darkmatter
  - darkmatter-cli
---

# Alias Resolution Spawns an Interactive Shell and Can Hang Compose Forever

> Opened as `2026-07-27-rayon-deadlock-handling` while the compose cache was the
> suspect; renamed once the root cause was confirmed to be unrelated to rayon.
> The compose-cache defect is real but separate — see §"Split out:
> compose-cache single-flight parks".

## Summary

Every `::shell` directive causes Darkmatter to spawn the user's login shell in
**interactive** mode to look up a possible alias:

```rust
// darkmatter/lib/src/markdown/compose/shell_expansion/alias.rs:50-56
let shell = std::env::var("SHELL").ok()?;
let output = Command::new(&shell)
    .args(["-ic", &format!("alias {}", name)])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .output()          // <- reads stdout to EOF, NO TIMEOUT
    .ok()?;
```

When the composing process has a **controlling terminal** and sits in a
**background process group** of it, `bash -i` performs job-control setup, touches
the terminal, and the kernel stops it with **SIGTTOU**. A stopped child never
exits, so its stdout pipe never closes, so `read_to_end` inside
`Command::output()` blocks **forever**. There is no timeout anywhere on this
path.

This is a product defect, not a test defect. Any user running `md compose` on a
document containing a `::shell` directive hangs indefinitely whenever the process
lands in a background process group of a terminal — backgrounding with `&`, a job
that loses the foreground, or any tool that runs `md` in its own process group.

## Confirmed Evidence

Captured from a live hang during a real `just test` run (gdb attach, main
thread):

```text
#3  std::sys::fd::unix::FileDesc::read ()          library/std/src/sys/fd/unix.rs:112
#7  std::io::Read::read_to_end<...> ()             library/std/src/io/mod.rs:919
#9  std::sys::process::output ()                   library/std/src/sys/process/mod.rs:60
#10 std::process::Command::output ()               library/std/src/process.rs:1113
#11 resolve_alias ()                               shell_expansion/alias.rs:56
#12 resolve_or_passthrough ()                      shell_expansion/mod.rs:728
#13 prepare_directive ()                           shell_expansion/mod.rs:232
#14 execute_directive_detailed ()                  shell_expansion/mod.rs:204
#15 run_stage ()                                   inline/shell_expansion.rs:44
...
#26 <Markdown>::validate_references ()             reference/mod.rs:600
#27 run_compose ()                                 cli/src/commands/compose.rs:288
```

The child process at that moment:

```text
PID 1191912   State: T (stopped)   /bin/bash -ic alias nonexistent_command_xyz
PID    PGID      TPGID     TT      STAT
1191912 1191909  1192097   pts/7   T
```

`PGID` (1191909, inherited from `md`) is not `TPGID` (1192097, the terminal's
foreground group). The child is in a background process group of `pts/7` and is
**stopped**, not running.

### Deterministic reproduction

`bash -ic` is only stopped when a controlling terminal exists *and* the composing
process is in a background process group of it. Both conditions hold under
nextest run from a terminal (nextest places each test in its own process group);
neither holds in a pipe-only context.

```bash
# Hangs: PTY present + own process group (`set -m`)
set -m
md compose <doc-with-::shell> &     # child bash -ic goes to state T, md wedges
```

Observed: `md` state `T`, child `bash -ic` state `T`, elapsed growing without
bound. Without a controlling terminal the same command completes in ~0.5 s.

### It is shell-dependent, NOT platform-dependent

The defect was first reported as "Linux fails, macOS is fine." That framing is
wrong and would have led to a Linux-only fix. Measured on one Linux host, same
binary, same PTY-plus-background-process-group harness, varying only `$SHELL`:

| `$SHELL` | Result |
|---|---|
| `/bin/bash` | **hangs** — child `bash -ic` in state `T` (stopped) |
| `/bin/zsh` | exits normally, no hang |
| `/bin/dash` | observed stopped (`T`) — same failure mode as bash |

The discriminator is the shell's interactive job-control startup, not the
operating system. Bash, when interactive and not in the foreground process group,
deliberately stops itself so it can claim the terminal when later foregrounded;
zsh does not take that path.

The apparent OS split is explained entirely by defaults: **macOS has defaulted to
zsh since Catalina, while Linux distributions default to bash.** POSIX job
control is implemented on macOS too.

Consequences for scope:

- **macOS is affected**, not immune. Any macOS user whose `$SHELL` is bash — a
  common choice — hits the same hang.
- A Linux developer using zsh will never see it, which is a second reason two
  engineers on the same code disagree about whether the bug exists.
- The fix must not be conditioned on `cfg(target_os)`.

### Why this was invisible for so long

Three independent conditions must coincide, which is why it survived to now:

| Condition | Needed for the hang |
|---|---|
| Controlling terminal present | yes — absent in CI, pipes, non-TTY harnesses |
| Composing process in a background process group | yes — nextest gives each test its own |
| `$SHELL` is a bash-family shell | yes — zsh does not stop |

| Context | Result |
|---|---|
| `just test` from a terminal pane, `$SHELL=bash` | **hangs, every time** |
| CI, pipes, non-TTY harnesses | passes, ~0.5 s |
| Investigation via a non-TTY tool | passed 130+ consecutive runs |

The failure is fully deterministic per environment, which is why it "has never
passed" for one developer and "never failed" in another context on the same
machine, same commit, same user. It is not a race, and no amount of retrying or
timeout-raising changes the outcome.

## Secondary Defects on the Same Line

Both are worth fixing even though neither causes the hang.

1. **Compose executes the user's interactive shell configuration.** `-i` sources
   `~/.bashrc` / `~/.zshrc` and everything they pull in. Composing a Markdown
   document should not run a developer's arbitrary interactive startup code. It
   is a correctness hazard (rc side effects), a security consideration, and it
   makes composition depend on unrelated machine state.

2. **It costs ~110 ms per directive and repeats.** A `RUST_LOG=trace` timeline of
   a *healthy* two-line document shows four ~110 ms gaps — four interactive-shell
   spawns, because the compose pipeline runs the shell-expansion stage four times
   over the same document. That is the entire measured difference between this
   test (0.49 s) and its non-shell siblings (0.057 s).

## Requirements

### R1 — Compose must never block indefinitely on a child process

Every child spawn in the composition path must be bounded. A child that stops,
wedges, or holds its pipes open must surface as a recoverable error, not an
unbounded wait. `Command::output()` with no timeout is not acceptable on this
path.

### R2 — Alias resolution must not be stoppable by terminal job control

The alias-query child must not be subject to SIGTTOU/SIGTTIN from the parent's
controlling terminal. It has no need for a terminal: stdin is already
`Stdio::null()` and only stdout is read.

### R3 — Composition must not execute interactive shell configuration

Resolving an alias must not require sourcing the user's interactive rc files. If
alias support cannot be provided without that, it must be opt-in and off by
default, with the opt-in documented.

### R4 — Failure must be observable

A shell that is killed by timeout, stopped, or unreadable must produce a
`tracing::warn!` naming the shell, the alias, and the reason. The current code
discards every failure with `.ok()?`, so nothing is reported.

### R5 — Behavior must be identical with and without a controlling terminal

`md compose` on the same document must produce the same result and comparable
timing whether run in a foreground terminal, backgrounded, piped, or under a test
harness. This is the invariant the current code violates.

## Design Options

### O1 — Do not use an interactive shell (recommended)

Drop `-i`. Non-interactive shells do not run job-control setup, so SIGTTOU cannot
occur, no rc files are sourced, and startup cost drops sharply. Satisfies R2 and
R3 outright.

Cost: aliases defined only in interactive rc files stop resolving. **This needs a
product decision** — see open question 1. If alias support must be kept, prefer
reading a declared alias source rather than executing an interactive shell.

### O2 — Detach the child from the controlling terminal

Spawn the query shell in its own session (`setsid` via
`CommandExt::pre_exec`, Unix-only) so it has no controlling terminal and job
control becomes a no-op. Satisfies R2 while keeping `-i`.

Keeps R3 unmet (rc files still execute) and adds a platform-specific spawn path.
Windows is unaffected — `resolve_alias` should be a no-op there regardless.

### O3 — Bounded wait (required regardless)

Replace `.output()` with a spawn plus a bounded wait, killing the child and
returning `None` (with a `tracing::warn!`) on expiry. This is defense in depth for
R1 and should land **whichever** of O1/O2 is chosen — it is the only option that
protects against the general class of "child never closes its pipes".

**Preliminary recommendation:** O1 + O3. O2 only if the product decision is that
interactive-rc aliases must keep working.

## Testing Contract

L1 unless noted. The critical property is that the regression test must fail
against today's code.

1. **Bounded-wait unit test (R1).** A stopped or non-exiting child must cause
   `resolve_alias` to return within a small bound rather than block. Simulate
   with a child that holds its stdout open.
2. **Controlling-terminal regression (R2, R5).** This is the honest reproduction
   and it needs a real PTY plus a background process group, so it belongs in
   **L2** (`level2_` prefix, gated on harness availability per the `rust-testing`
   skill). Assert `md compose` on a `::shell` document completes in seconds when
   run in a background process group of a PTY. It must fail against today's code.
   Existing L1 tests cannot cover this — every current test runs without a
   controlling terminal, which is exactly why the bug shipped.

   **Parameterize over `$SHELL`.** The test must cover a bash-family shell, since
   zsh does not reproduce the failure and a zsh-only test would pass against the
   broken code. Skip cleanly when the shell under test is unavailable rather than
   assuming a fixed path.
3. **No interactive rc execution (R3).** Assert the spawned command line contains
   no `-i`, or that a sentinel written by a fixture rc file is absent.
4. **Warning on failure (R4).** Capture the `tracing::warn!` on timeout.
5. **Suites green.** `just test`, `just test-l2`, `just lint` for darkmatter —
   **run at least once from a real terminal pane**, not only from a non-TTY
   context, or the regression is not actually verified.

## Acceptance Criteria

- [ ] `md compose` on a `::shell` document cannot hang, in any process-group or
      terminal configuration (R1, R2, R5).
- [ ] `test_compose_with_nonexistent_command_fails` passes when `just test` is run
      from a terminal pane, in well under the 5 s `SLOW` marker.
- [ ] Composition no longer sources interactive shell configuration, or does so
      only behind a documented opt-in (R3).
- [ ] Alias-resolution failures emit a `tracing::warn!` instead of a silent
      `.ok()?` (R4).
- [ ] L2 regression test exists that fails against the current implementation.
- [ ] Per-directive alias-resolution cost measured before/after; the four repeated
      stage invocations are recorded (see open question 3).

## Split Out: Compose-Cache Single-Flight Parks

The earlier investigation found a genuine, separate defect that should keep its
own topic:

`cache/runtime.rs:91` defines `INFLIGHT_TIMEOUT = 30s`, used at `runtime.rs:529`
and `:587`. A thread that parks on an in-flight slot waits up to 30 seconds and
then silently recomputes; the timeout branch calls `record_error()` with no log,
`CacheStats` conflates it with unrelated errors, and no CLI surface reads
`CacheStats` at all. The same 30 s pattern is mirrored in
`shell_expansion/types.rs:959` (`RESERVATION_WAIT_TIMEOUT`), reachable from the
library API but not the CLI.

That is worth fixing on its own merits — parking 30 s to avoid duplicating a
~110 ms computation is a poor trade, and the doc comment concedes it only
*mitigates* a rayon deadlock risk. But it is **not** the cause of this hang, and
combining the two would confuse both. Recommend a separate fix topic.

## Relationship to the Nextest Ceiling

`.config/nextest.toml` local-dev `slow-timeout` was raised from 30 s to 45 s
(`terminate-after = 9`) during the investigation; `ci` stays at 30 s.

That change is orthogonal and should be kept on its own merits (headroom for
genuine cross-worktree contention, measured at roughly 3x). It never had any
bearing on this defect: the hang is **unbounded**, so no ceiling can absorb it.
Raising the ceiling only changed how long the suite waited before reporting.

## Open Questions

1. **Must interactive-rc aliases keep working?** This determines O1 vs O2 and is
   a product decision, not a technical one. If aliases in `~/.bashrc` must
   resolve, O1 is off the table and R3 needs rewording.
2. Should `resolve_alias` be a no-op on Windows? `$SHELL` is meaningless there and
   the current code presumably no-ops by accident when the var is unset — that
   should be explicit.
3. Why does the compose pipeline run the shell-expansion stage four times over one
   document? It multiplies this cost by four. Possibly its own fix topic.
4. ~~Does macOS genuinely not reproduce?~~ **Answered.** It is a shell
   difference, not a platform one — bash and dash hang, zsh does not, measured on
   a single Linux host varying only `$SHELL` (see §"It is shell-dependent").
   macOS is affected whenever `$SHELL` is bash. Do not gate the fix on
   `cfg(target_os)`.

## Provenance

- Root cause confirmed 2026-07-27 by gdb attach to a live hang during a real
  `just test`, plus deterministic reproduction under a PTY with `set -m`.
- Superseded reasoning, recorded so it is not re-derived: (a) host CPU contention
  — wrong, measured factor is ~3x, and the failure occurs on an idle machine;
  (b) compose-cache `INFLIGHT_TIMEOUT` parks — a real defect but not this one,
  ruled out by the backtrace and by the failure being deterministic rather than a
  race. Both were pursued before the deterministic nature of the failure was
  known; that single fact would have excluded every race-based hypothesis
  immediately.
