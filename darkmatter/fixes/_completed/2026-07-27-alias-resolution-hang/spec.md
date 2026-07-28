---
status: root cause confirmed — design approved
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-07-27
review_iterations: 1
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

> **Reader's note — reviewed design decision:** remove implicit shell-alias
> resolution from composition. Darkmatter's established shell-expansion
> contract executes a parsed executable and argument vector directly; consulting
> interactive, machine-local shell state contradicts that contract and makes
> composition depend on state outside `ComposeOptions`. A non-interactive
> `alias` query is not a substitute because a fresh non-interactive shell
> normally has no interactive aliases to report. Users who need reusable
> commands can place an executable wrapper on `PATH`.

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

### R1 — Alias lookup must not spawn a child process

Composition and preflight must not invoke a login or interactive shell to
resolve a command name. If `which::which` cannot resolve an executable, preserve
the existing typed `CommandNotFound` path.

This specification fixes the only currently identified unbounded process spawn
in alias lookup. It does not redefine the timeout contract for ordinary
`::shell` execution, which already uses `ShellExpansionOptions::timeout`.

### R2 — Preflight and execution must resolve commands identically

Remove alias expansion from both the condition-blind preflight collector and
the execution preparation path. Approval, policy normalization, displayed
commands, and execution must all use the executable and arguments authored in
the directive.

### R3 — Composition must not execute interactive shell configuration

No compose or preflight path may source `.bashrc`, `.zshrc`, or another
interactive startup file merely to discover a command. This applies on macOS,
Windows, and Linux.

### R4 — Missing commands must remain observable

An unresolved executable is not an internal warning condition; it is normal
input to the existing `CommandNotFound` error contract. Do not add a warning for
"not an alias." The CLI error must identify the authored executable and source
origin without mentioning aliases.

### R5 — Behavior must not depend on terminal or shell configuration

The same document and captured `ComposeOptions` must produce the same command
resolution, approval request, and result whether composition runs in a
foreground terminal, a background process group, a pipe, or a test harness.
Changing `$SHELL` or interactive rc contents must not change those results.

## Chosen Design

### Remove automatic alias resolution

Delete the alias-query process and all automatic alias rewriting. This includes:

1. `shell_expansion/alias.rs` and its `ResolvedAlias` / `resolve_alias`
   re-exports;
2. runtime alias rewriting in `resolve_or_passthrough`;
3. preflight alias rewriting in `resolve_executable`;
4. alias-only display and approval metadata, including
   `ShellApprovalRequest::alias_name`; and
5. alias-specific tests and documentation.

After removal, command resolution is the direct-executable contract already
used by the executor:

1. preserve the parsed executable and argv;
2. resolve the executable with `which::which`;
3. apply approval and policy checks to that same command; and
4. execute it directly with the existing bounded executor.

This intentionally removes support for aliases defined in interactive startup
files. It is a breaking library API change because alias symbols and a public
approval-request field currently exist. The repository has no established
users, so removing the misleading surface now is preferable to retaining a
permanently empty compatibility API. Before implementation, run GitNexus impact
analysis on `resolve_alias`, `ResolvedAlias`,
`ShellApprovalRequest::alias_name`, and `resolve_or_passthrough`; include every
identified downstream package in verification.

#### Confirmed downstream consumer: claudine

A grep-level blast-radius check already identifies one concrete out-of-area
consumer, so the verification scope is **darkmatter + claudine**, not darkmatter
alone:

- `claudine/lib/src/harness/shell.rs:255` constructs
  `darkmatter::markdown::compose::shell_expansion::ShellApprovalRequest` with an
  explicit `alias_name: None` field. Removing the field breaks this build.
- `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/shell_approval.rs`
  imports `ShellApprovalRequest` and the approval traits.

Both must be updated in the same change, and claudine's `just build` / `just test`
/ `just lint` must be part of the gate set.

Two same-named symbols are **false positives** and must not be touched:
`claudine/lib/src/model_catalog/families.rs::resolve_alias` (model-family alias
resolution, unrelated) and the `resolve_alias_*` test names in
`darkmatter/lib/src/markdown/language_grammar.rs` (language-grammar aliases).
GitNexus impact analysis should confirm this split rather than a bare grep.

### Rejected alternatives

#### Query without `-i`

- **Pros:** avoids interactive job control and startup files; small code change.
- **Cons:** a new non-interactive shell normally knows none of the user's
  aliases, so the feature becomes misleading and still adds a process spawn per
  unresolved executable.

#### Detach and bound the interactive query

- **Pros:** retains aliases from interactive startup files and prevents an
  indefinite wait.
- **Cons:** still executes arbitrary startup code before approval, remains
  machine-dependent, needs different Unix/Windows process handling, and adds
  timeout and process-tree cleanup complexity.

#### Explicit alias configuration in `ComposeOptions`

- **Pros:** deterministic, testable, and does not source startup files.
- **Cons:** adds configuration and policy semantics not needed to fix the hang;
  executable wrapper scripts already provide a portable solution.

The chosen removal is the smallest design that satisfies the direct-execution,
security, determinism, and cross-platform contracts. Explicit aliases may be
proposed later as a separate feature if a concrete use case justifies them.

## Scope

In scope:

- remove automatic alias resolution from library execution and passive
  preflight collection;
- remove the associated public API and stale alias documentation;
- preserve the existing `CommandNotFound` diagnostic for unresolved authored
  executables;
- verify policy/approval identity is unchanged between preflight and execution;
  and
- add regression coverage for terminal, `$SHELL`, and rc-file independence.

Out of scope:

- changing the existing timeout behavior of approved `::shell` commands;
- adding an alias configuration format;
- fixing repeated shell-expansion stage invocation or compose-cache
  single-flight waits; and
- changing shell pipeline, redirection, approval, or policy semantics.

## Testing Contract

L1 unless noted. The critical property is that the regression test must fail
against today's code.

1. **No alias-query process (R1, R3).** With `$SHELL` set to a fixture program
   that records invocation, compose and preflight an unresolved command and
   assert the fixture was never invoked. Assert the operation returns the
   existing `CommandNotFound` error.
2. **Controlling-terminal regression (R1, R5).** This is the honest reproduction
   and it needs a real PTY plus a background process group, so it belongs in
   **L2** (`level2_` prefix, gated on harness availability per the `rust-testing`
   skill). Assert `md compose` on a `::shell` document completes in seconds when
   run in a background process group of a PTY and reports `CommandNotFound`. It
   must fail against today's code.

   Use the shared `biscuit-test-harness` primitives and a bash-family shell when
   available. Skip cleanly when the required PTY/job-control capability is
   unavailable. The test must not focus or inject input into a host terminal.
3. **Configuration independence (R3, R5).** Run the same L1 compose/preflight
   cases with distinct `$SHELL` values and fixture rc files; assert identical
   approval entries and errors and no rc sentinel side effect. Serialize
   environment-mutating tests so parallel tests cannot race.
4. **Preflight/runtime parity (R2).** For a missing executable in a single
   command, pipeline, and chain, assert preflight records the authored
   executable/argv and execution reports that same executable rather than a
   rewritten alias target.
5. **Public-surface cleanup.** Compile-time/API tests and generated documentation
   must contain no `ResolvedAlias`, `resolve_alias`, or `alias_name` approval
   field.
6. **Gates.** After Sniff package-area discovery and GitNexus impact analysis,
   run `just build`, `just test`, and `just lint` in each affected package area,
   plus `just test-l2` for Darkmatter. The gate set is known to include **both
   darkmatter and claudine** (see §"Confirmed downstream consumer"). Linux is the
   available host; code review and CI must cover macOS and Windows branches
   without OS-specific alias logic.

   **Run the darkmatter gates at least once from a real terminal pane.** Every
   pre-existing test passed while this bug was live precisely because the harness
   had no controlling terminal; a non-TTY-only verification would repeat that
   mistake.

## Acceptance Criteria

- [ ] `md compose` on a `::shell` document cannot hang, in any process-group or
      terminal configuration (R1, R2, R5).
- [ ] `test_compose_with_nonexistent_command_fails` passes in well under the 5 s
      `SLOW` marker, and the L2 equivalent passes under the PTY harness with a
      background process group.
- [ ] Composition and preflight never spawn `$SHELL` or source interactive shell
      configuration (R1, R3).
- [ ] Unresolved commands produce the existing typed `CommandNotFound`
      diagnostic for the authored executable (R4).
- [ ] Preflight approval identity and runtime execution identity agree for
      single commands, pipelines, and chains (R2).
- [ ] Public alias-resolution symbols and alias-only approval metadata are
      removed, with downstream impact verified.
- [ ] L2 regression test exists that fails against the current implementation.
- [ ] A before/after measurement records removal of alias-query process spawns;
      repeated stage invocation remains explicitly out of scope.

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

1. Why does the compose pipeline run the shell-expansion stage four times over one
   document? It multiplies command-discovery cost even after alias lookup is
   removed. Track this as a separate performance fix because changing stage
   scheduling has a broader semantic blast radius.
2. ~~Does macOS genuinely not reproduce?~~ **Answered.** It is a shell
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
