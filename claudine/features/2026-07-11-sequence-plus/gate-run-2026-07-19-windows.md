# Gate run — 2026-07-19 — Windows type-check + native runbook (review-8 finding 2)

The executable half of review-8 finding 2 from this host: the Windows
cross-target type-check gates, re-run at the committed candidate. This file
records measurement only. No production code and no test was changed to
produce it. Format follows `gate-run-2026-07-19-linux.md`: nothing is reported
green unless the command ran to completion and its summary line is quoted
verbatim.

**Precision rule of this record** (same as the Linux gate record and the
validation matrix): "Executed" means the code ran to completion on a real
host. "Type-checked" means `cargo check` accepted the source — it proves
signatures and types and **no runtime behavior whatsoever**. Everything green
below is type-check-only. Nothing Windows has ever executed.

## Revision under test

```
git rev-parse HEAD
baba838446cf0e21c33dd17870c462b45c0311e6
```

The working tree is **not** clean, and the dirt is load-bearing: the Windows
and Linux Level-3 fixtures compile only with the uncommitted
biscuit-test-harness injector modules present. `git status --short` at run
time, verbatim:

```
 M .claudine/memory/commits.md
 M CLAUDE.md
 M biscuit-test-harness/src/lib.rs
 M claudine/cli/tests/level3_linux_sequence_ctrl_c.rs
 M claudine/features/2026-07-11-sequence-plus/l3-ctrl-c-runbook.md
 M claudine/features/2026-07-11-sequence-plus/review-7.md
 M claudine/features/2026-07-11-sequence-plus/spec.md
?? biscuit-test-harness/src/win_input.rs
?? biscuit-test-harness/src/xdotool.rs
?? claudine/features/2026-07-11-sequence-plus/gate-run-2026-07-19-l3-linux.md
?? claudine/features/2026-07-11-sequence-plus/review-8.md
?? prompts/_implement/implement-review-findings-plan.md
?? prompts/_implement/review-findings-plan.md
```

Enumerated: the test-harness injector modules (`biscuit-test-harness/src/
win_input.rs`, `xdotool.rs`, and their `lib.rs` declarations) that the L3
fixtures require; the review-8 working set (`review-8.md`, the concurrent
finding-1 artifacts `gate-run-2026-07-19-l3-linux.md` /
`level3_linux_sequence_ctrl_c.rs` / `l3-ctrl-c-runbook.md`, plus `review-7.md`
and `spec.md` doc edits); and repo-level docs/prompts (`CLAUDE.md`, memory,
`prompts/_implement/*`). No `lib/src` or `cli/src` production file is dirty —
the compiled Windows-path sources under review are exactly `baba83844`'s.

## Summary of verdicts

| Gate | Verdict | Exit code | Duration |
|---|---|---|---|
| `just check-windows` (from `claudine/`) — lib + CLI **including all test targets**, `x86_64-pc-windows-gnu` | **Green — compile-only, NOT runtime verification** | `0` | 8.25 s wall (warm dependency cache; both workspace crates freshly re-checked) |
| Native Windows runtime execution (any of it) | **Not run — impossible on this host.** The gap stands. | — | — |

## Environment

Host: macOS 26.5.2 (build 25F84), Apple Silicon. Toolchain, verbatim:

```
rustc 1.96.0 (ac68faa20 2026-05-25)
cargo 1.96.0 (30a34c682 2026-05-25)
x86_64-w64-mingw32-gcc (GCC) 16.1.0
```

`rustup` targets installed: `x86_64-pc-windows-gnu` (used by the gate),
`x86_64-pc-windows-msvc`.

## Gate — `just check-windows` (type-check, lib + CLI test targets)

Run 2026-07-19 from the `claudine/` package area. The recipe
(`claudine/justfile::check-windows`) executes, with its two load-bearing host
workarounds (`RUSTC_WRAPPER=""` to keep the host's kache rustc-wrapper out of
cc-rs; `-Wa,-mbig-obj` for duckdb's COFF section overflow) and
`CARGO_TARGET_DIR=target/windows-check`:

```
cargo check -p claudine -p claudine-cli --tests --target x86_64-pc-windows-gnu
```

Verbatim closing lines:

```
warning: `claudine` (lib test) generated 16 warnings (2 duplicates) (run `cargo fix --lib -p claudine --tests` to apply 10 suggestions)
warning: `claudine-cli` (bin "claudine" test) generated 2 warnings (run `cargo fix --bin "claudine" -p claudine-cli --tests` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.91s
EXIT=0
```

Wall time 8.25 s (`just check-windows 15.32s user 12.02s system 331% cpu
8.250 total`). The short duration is a warm `target/windows-check` dependency
cache from earlier rounds; cargo nevertheless re-fingerprinted and freshly
re-checked both workspace crates at this tree (`Checking claudine v0.1.0 …`,
`Checking claudine-cli v0.1.0 …` appear in the transcript), including every
CLI integration-test target — the warning replay above spans the test
binaries. The warnings are dead-code/unused-import artifacts of Unix-only
helpers appearing unused under the Windows cfg; there are no errors.

**What this proves:** the Windows arms of the current committed candidate —
`cli/src/commands/wrap/exec/termination/windows.rs` (console-control handler,
`windows_wait_loop`, Job-object wait scopes, and its
`#[cfg(all(test, windows))]` regressions), the `#[cfg(windows)]`
suspended-spawn/Job-assignment path in
`lib/src/composition/sequence/task/shell.rs`, the `#[cfg(windows)]`
process-tree test twins in `task/tests.rs`, and the
`cli/tests/level2_windows_sequence_ctrl_c.rs` /
`level3_windows_sequence_ctrl_c.rs` fixtures — all type-check for
`x86_64-pc-windows-gnu`.

**What this does not prove:** anything about `CreateJobObjectW`,
`SetInformationJobObject`, `AssignProcessToJobObject`, thread
discovery/`ResumeThread`, `GenerateConsoleCtrlEvent`, `TerminateJobObject`,
kill-on-close, or any other runtime behavior. `cargo check` neither links nor
runs. This result must never be cited as runtime verification.

## Native Windows runtime execution: did not happen, cannot happen here

No Windows code executed in this gate run, and none can from this host: there
is no Windows machine available, Windows containers cannot run under Docker on
macOS, and emulation (Wine) is not evidence. Per review-8 finding 2, the
following remain owed and can only be produced on a **native Windows host with
an attached console** at the release-candidate revision:

- success-path descendant cleanup
- ordinary failure
- timeout (direct child and nested tree)
- Ctrl+Break interruption
- runaway output (cap trip with counters)
- ownership-establishment failure (typed, fail-closed)
- inherited-pipe closure (descendant holding stdout)
- Job assignment/resume (suspended spawn → assign → resume, race-free)
- descendant cleanup on every exit path
- plus the attended Windows **Level-3** keyboard fixture
  (`level3_windows_sequence_ctrl_c`) per `l3-ctrl-c-runbook.md`

## Native Windows runbook

For the operator on a native Windows host. Prerequisites: interactive desktop
session (not a disconnected RDP session), attached console, Rust stable
toolchain (repo `rust-toolchain.toml` pins `channel = "stable"`),
`cargo-nextest`, `just`, and a bash on `PATH` (Git Bash/MSYS — every justfile
recipe runs under `bash`). The tree must contain the biscuit-test-harness
injector modules (`win_input.rs` + the `lib.rs` declarations) — i.e. a commit
including them, or this exact dirty tree reproduced. All commands run from the
`claudine/` package area.

For **every** step record: revision (`git rev-parse HEAD` plus
`git status --short` if dirty), host OS and version, terminal and version,
exact command, the verbatim nextest/cargo summary line, and the exit code.
Nothing is green without a verbatim quoted summary line.

### 1. Level-1 suite — process-tree twins + Job-object regressions

```
just test
```

Runs all five area crates under nextest. On a Windows host this **executes**
(not merely compiles) the `#[cfg(windows)]` twins in
`claudine::composition::sequence::task::tests` (success reap, pipeline,
nested-tree timeout, and siblings) and the Job-object regressions in
`claudine-cli`'s bin test target,
`commands::wrap::exec::termination::windows::tests` —
`job_close_terminates_a_descendant_when_the_wait_scope_ends` and
`repeated_wait_scopes_do_not_grow_the_process_handle_count`.

Focused re-runs use the just recipes with **positional** filters (never raw
`cargo nextest -p …`, which drops the tier-exclusion filterset):

```
just test-library composition::sequence::task
just test-cli termination::windows
```

Record: the two nextest `Summary [...]` lines (or the full-suite one) and exit
codes; confirm the named Windows tests appear as `PASS`, not filtered out.

### 2. Attached-console runtime gate — Ctrl+Break ladder

```
just test-windows-ctrl-c
```

The justfile's Windows-host runtime gate: refuses to run unless
`OS=Windows_NT`, then executes the otherwise-`#[ignore]`d
`windows_ctrl_c_verification_record` in `cli/tests/level3_wrap_ctrl_c.rs` via
`cargo test … -- --ignored`. Requires the attached console —
`GenerateConsoleCtrlEvent` has nothing to deliver to without one.

Record: the verbatim `test result: ok. 1 passed; …` line and exit code.

### 3. Level-2 — console-control fan-out through a real sequence

```
just test-l2 level2_windows_sequence_ctrl_c
```

The filter is **positional**, not `-E`: the recipe already passes
`-E 'test(/level2_/)'`, and a second `-E` would *union* with it (running the
whole L2 tier) instead of intersecting (documented in
`l3-ctrl-c-runbook.md`). Executes
`level2_windows_sequence_ctrl_c_fans_out_to_parallel_children`: a real
Claudine process, a real parallel group, a real `CTRL_BREAK_EVENT` into its
process group; asserts child termination, no later step, shell regains
control, exit `130`.

Record: the verbatim nextest `Summary [...] 1 tests run: 1 passed` line and
exit code.

### 4. Attended Level-3 — real OS keyboard chord

Follow `l3-ctrl-c-runbook.md` in full (free machine, single
`claudine`-titled window, launch from inside WezTerm or cold-start its mux
server, `powershell` on `PATH`, prior `cargo build -p claudine-cli`), then:

```
just test-l3 level3_windows_sequence_ctrl_c
```

(positional filter, same union-vs-intersect rule; answer `y` at the TTY
prompt — never set `BISCUIT_L3_TAKE_FOCUS` on an agent's behalf). To make a
missing backend a hard failure instead of a silent skip:
`BISCUIT_TEST_LEVEL_REQUIRED=3 just test-l3 level3_windows_sequence_ctrl_c`.

Record, beyond the summary line and exit code, the runbook's four
observations: the captured pane (including the `L3WINSEQ_0rc=130` sentinel —
exit `130` and proof the shell regained control), absence of
`later-step-ran.txt`, and zero surviving `PING.EXE` descendants (the
Job-object tree-reap check).

Steps 1–3 discharge the finding's runtime list; step 4 discharges the Windows
leg of review-8 finding 1. When recorded, update `validation-matrix.md`'s
Windows rows — until then they stay red, and this file's type-check green must
not be promoted into them as anything more than compile evidence.
