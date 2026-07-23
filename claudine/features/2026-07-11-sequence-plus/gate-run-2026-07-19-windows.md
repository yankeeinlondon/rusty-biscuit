# Gate run — Windows type-check + native runbook (review-8 finding 2)

**Run date: 2026-07-20.** The filename retains its original `2026-07-19` date so
that inbound links from `validation-matrix.md` keep resolving; the authoritative
dating and revision anchor are the ones stated in this file, not in its name.
This record supersedes the 2026-07-19 run at `baba83844` — see
[Revision under test](#revision-under-test).

The executable half of review-8 finding 2 from this host: the Windows
cross-target type-check gates, re-run at the release candidate. This file
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
96679f516184722e858d0e2fa8778a541c8b0b4e
```

`96679f516` is the release candidate. **The load-bearing-dirt caveat that the
previous revision of this record carried is gone, and that is the point of this
update.** The prior run (anchored at `baba83844`) had to state that the working
tree was not clean and that the dirt was load-bearing, because the Windows and
Linux Level-3 fixtures compiled only with the then-uncommitted
biscuit-test-harness injector modules present. Those modules were committed in
`b965938e2` (`feat(biscuit-test-harness): add Linux and Windows L3 keyboard
injectors`), which is an ancestor of the release candidate:

```
git ls-tree HEAD biscuit-test-harness/src/
100644 blob 5d93ad2e441fc0e53df2be96df772d55cc407cf6	biscuit-test-harness/src/win_input.rs
100644 blob 224737e8400fcaace8eb32d2f328ad02d7babbd9	biscuit-test-harness/src/xdotool.rs
```

(Abridged to the two injector entries; the full listing also contains the
pre-existing harness modules. `git merge-base --is-ancestor b965938e2 HEAD`
succeeds.)

`git status --short` at run time, verbatim:

```
 M .claude/skills/biscuit-test-harness/SKILL.md
 M .claudine/memory/commits.md
 M CLAUDE.md
 M biscuit-test-harness/README.md
 M biscuit-test-harness/src/cliclick.rs
 M biscuit-test-harness/src/lib.rs
 M claudine/features/2026-07-11-sequence-plus/review-9.md
 M claudine/features/2026-07-11-sequence-plus/spec.md
?? claudine/features/2026-07-11-sequence-plus/review-10.md
?? prompts/_implement/implement-review-findings-plan.md
?? prompts/_implement/review-findings-plan.md
```

Enumerated: two dirty `.rs` files (`biscuit-test-harness/src/lib.rs` and
`src/cliclick.rs`); the review working set (`review-9.md`, `review-10.md`,
`spec.md`); and repo-level docs/prompts (`CLAUDE.md`, the harness `SKILL.md`
and `README.md`, memory, `prompts/_implement/*`). No `lib/src` or `cli/src`
production file is dirty — the compiled Windows-path sources under review are
exactly `96679f516`'s.

**Reproducibility property.** The two dirty `.rs` files were checked
mechanically and contain **comment-only** changes: every added or removed
non-blank line is a `//`, `///`, or `//!` line. Re-verify with:

```
for f in biscuit-test-harness/src/lib.rs biscuit-test-harness/src/cliclick.rs; do
  git diff -- "$f" | grep -E '^[+-]' | grep -vE '^[+-]{3}' \
    | grep -vE '^[+-]\s*(//|///|//!)' | grep -vE '^[+-]\s*$'
done
```

Empty output means comment-only. The working tree is therefore behaviorally
identical to a clean checkout of `96679f516`, and the dirty remainder is
documentation. Consequently a **clean checkout of the release candidate does
build the Level-3 fixtures** — which is exactly the claim the previous anchor
could not make. This gate result is reproducible from a named revision alone,
with no tree reconstruction required.

## Summary of verdicts

| Gate | Verdict | Exit code | Duration |
|---|---|---|---|
| `just check-windows` (from `claudine/`) — lib + CLI **including all test targets**, `x86_64-pc-windows-gnu` | **Green — compile-only, NOT runtime verification** | `0` | 3 m 25 s cold (the meaningful figure); 1.05 s on an immediate warm re-run |
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

Run 2026-07-20 from the `claudine/` package area. The recipe
(`claudine/justfile::check-windows`) executes, with its two load-bearing host
workarounds (`RUSTC_WRAPPER=""` to keep the host's kache rustc-wrapper out of
cc-rs; `-Wa,-mbig-obj` for duckdb's COFF section overflow) and
`CARGO_TARGET_DIR=target/windows-check`:

```
cargo check -p claudine -p claudine-cli --tests --target x86_64-pc-windows-gnu
```

Two runs happened, back to back. The first was cold and finished in **3 m 25 s**
— that is the meaningful duration. An immediate warm re-run finished in
**1.05 s** and is what captured the exit code, `0`.

Verbatim closing lines of the captured log (`/tmp/check-windows-r4.log`, the
warm re-run's transcript):

```
warning: `claudine-cli` (test "loop_cli") generated 4 warnings (run `cargo fix --test "loop_cli" -p claudine-cli` to apply 4 suggestions)
warning: `claudine-cli` (test "wrap_antigravity_exit_signal") generated 5 warnings (run `cargo fix --test "wrap_antigravity_exit_signal" -p claudine-cli` to apply 5 suggestions)
warning: `claudine-cli` (test "wrap_opencode") generated 7 warnings (run `cargo fix --test "wrap_opencode" -p claudine-cli` to apply 7 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.05s
```

Two provenance notes, stated rather than smoothed over. First, the exit code
`0` was observed as the shell status of the warm re-run; the captured log ends
at cargo's `Finished` line and carries no `EXIT=` marker of its own, so the
exit code is reported here, not quoted from the log. Second, because the log is
the warm re-run, it contains no `Checking claudine …` lines — cargo had nothing
left to re-fingerprint. The cold run immediately preceding it is what actually
type-checked both workspace crates and every CLI integration-test target; the
warning replay quoted above spans those test binaries and is cargo's cached
report of that work. The warnings are dead-code/unused-import artifacts of
Unix-only helpers appearing unused under the Windows cfg; there are no errors.

**What this proves:** the Windows arms of the release candidate `96679f516` —
`cli/src/commands/wrap/exec/termination/windows.rs` (console-control handler,
`windows_wait_loop`, Job-object wait scopes, and its
`#[cfg(all(test, windows))]` regressions), the `#[cfg(windows)]`
suspended-spawn/Job-assignment path in
`lib/src/composition/sequence/task/shell.rs`, the `#[cfg(windows)]`
process-tree test twins in `task/tests.rs`, and the
`cli/tests/level2_windows_sequence_ctrl_c.rs` /
`level3_windows_sequence_ctrl_c.rs` fixtures — all type-check for
`x86_64-pc-windows-gnu`, and do so from a clean checkout of that revision.

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
recipe runs under `bash`). A clean checkout of `96679f516` is sufficient: the
biscuit-test-harness injector modules the fixtures require (`win_input.rs` and
its `lib.rs` declaration) are committed as of `b965938e2`, so no dirty tree
needs reproducing. All commands run from the `claudine/` package area.

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
