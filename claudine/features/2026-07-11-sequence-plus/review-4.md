---
$schema: feature-review.yaml
ready: false
agent: claude/default
created: 2026-07-18T13:57:22-07:00
spec: 2026-07-11-sequence-plus/spec.md
implemented: false
description: A **feature** review of `2026-07-11-sequence-plus/spec.md`
feature: 2026-07-11-sequence-plus/review-4.md
previous: 2026-07-11-sequence-plus/review-3.md
---

# Review 4: Sequence Plus

## Verdict

**Not ready for production** — but the distance left is much shorter than at
review 3, and its character has changed.

Review 3's five findings produced real work. The Windows interrupt story was
rebuilt rather than patched: a process-scoped `InterruptRegistry` replaces the
global press/force-kill atomics, escalation state became per-registration, the
Job Object gained RAII ownership through a `HandleCloser`-parameterised
`OwnedRawHandle`, the simple wait path now runs the same loop as the
channel-driven ones, and `execute_sequence` finally has a Windows producer for
its shared `interrupted` flag. The Level-3 keyboard test went green through a
harness fix rather than a loosened assertion, and it acquired two containment
guards. A Level-2 prompt fixture now drives a real `claude` stream through the
semantic parser into a real pane.

Three of review 3's five findings are closed on the merits. What blocks release
now is **evidence, not design** — plus one derivation bug the new Windows path
exposes.

The recurring pattern is worth naming: the Windows implementation is the most
carefully reasoned code in this feature and the only code in it that has never
executed. Every Windows test written to prove it is unreachable on every
configuration this repository can currently build.

## Findings

### 1. High — The entire Windows implementation is unverified, and its own tests cannot compile

The design is sound and I could find no correctness defect in it by reading:
`GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid)` is correct for a child in
`CREATE_NEW_PROCESS_GROUP`; the drop-order argument (Job declared before
registration, so deregistration precedes `CloseHandle`) is right and is pinned
by `handle.rs::a_later_declared_guard_releases_before_the_handle_closes`; the
refcounted `SetConsoleCtrlHandler` correctly avoids the stacked-registration
multiply-count; carrying handles as `isize` rather than `HANDLE` across threads
is the right call.

None of that is evidence. Two Windows-host test suites were written for this
code:

- `termination/windows.rs` → `#[cfg(all(test, windows))]` —
  `job_close_terminates_a_descendant_when_the_wait_scope_ends` and
  `repeated_wait_scopes_do_not_grow_the_process_handle_count`
- `cli/tests/level2_windows_sequence_ctrl_c.rs` → `#![cfg(windows)]`

Neither can compile on any configuration this repository can currently build:

- `cargo check -p claudine --tests --target x86_64-pc-windows-gnu` fails on 7
  pre-existing Unix-only APIs in `#[cfg(test)]` code
  (`ExitStatusExt`/`from_raw` in `dispatch/runner/mappers.rs`,
  `os::unix::fs::symlink` in `protect/path.rs` and `protect/service/tests.rs`).
- `cargo check -p claudine-cli --tests --target …-gnu` is blocked by
  `duckdb-sys`' unity build exceeding COFF's section limit under mingw, reached
  only through the `rendezvous-daemon` dev-dependency.

So `level2_windows_sequence_ctrl_c.rs` has never been type-checked. A typo, a
wrong argument order, or a signature drift in it is invisible to every gate.
This is worse than an untested feature: it is an untested feature carrying tests
that create the *appearance* of coverage in the inventory.

**What I did verify**, on this host, this session:

- `RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/cl-win-target cargo check -p claudine-cli --target x86_64-pc-windows-gnu`
  → `Finished dev profile … in 49.01s`, exit `0`. The new production Windows
  code compiles. This is a genuine advance over review 3, and it is the
  strongest Windows evidence that exists.
- `cargo nextest run -p claudine-cli -E 'test(/coordinator|handle|focus_stealing|test_placement/)'`
  → `62 tests run: 62 passed`. The host-independent half of the coordinator —
  press ladder, per-registration escalation, flag fan-out, weak-flag pruning,
  refcount edges, concurrent registration, handle drop order — is covered at
  Level 1 on every platform. Deliberately generic over the child payload so it
  runs where Win32 does not; that was the right structural choice.

**Required change.** Make the Windows tests reachable, in ascending order of
cost:

1. Gate the 7 pre-existing Unix-only test-code sites so
   `cargo check -p claudine --tests --target x86_64-pc-windows-gnu` goes green.
   They are small and mechanical, and they unblock the `windows.rs` Job-object
   regressions. This is not sequence-plus's mess, but sequence-plus is the first
   feature that needs it.
2. Break the `claudine-cli` → `rendezvous-daemon` (dev) → `duckdb` chain, or
   feature-gate it, so CLI test targets cross-compile. Until then
   `level2_windows_sequence_ctrl_c.rs` is unverifiable dead source.
3. Then obtain one native Windows run of both suites.

Until at least (1) and (2) land, the Windows interruption contract's strongest
verification is *source audit plus production cross-compile*, and the spec's
cross-platform acceptance criterion (AC8) is not met.

### 2. Medium-High — A parallel group's prompt task is labeled `interrupted` by matching exit code `130`, which Windows never produces

`cli/src/commands/wrap/sequence/task_run.rs:371`:

```rust
interrupted: outcome.exit_code == super::SEQUENCE_INTERRUPT_EXIT_CODE,
```

The step-level path already knows this is insufficient.
`cli/src/commands/wrap/sequence/iterate.rs:316-318` reads:

```rust
if outcome.exit_code == SEQUENCE_INTERRUPT_EXIT_CODE
    || run.interrupted.load(Ordering::SeqCst)
```

The group-task path has no such fallback, and the new Windows ladder makes the
gap concrete. Neither Windows rung yields `130`:

- `PressAction::Graceful` → `CTRL_BREAK_EVENT`; a console app terminating on it
  exits `0xC000013A`, not `130`
- `PressAction::Force` → `TerminateJobObject(job, 1)` → exit code `1`

`SingleCompositionOutcome` (`composition/mod.rs:97-126`) carries no
`ProcessTermination`, so the exit code is the only signal available at that
call site — even though `windows_wait_loop` computed
`ProcessTermination::Interrupted` correctly and then discarded it.

**Failure scenario.** Windows, Ctrl+Break during a parallel group of prompt
tasks. The sequence still exits `130` (that comes from the shared flag via
`report.rs:54`), and the later step is still suppressed — the two assertions
`level2_windows_sequence_ctrl_c.rs` calls load-bearing. But every prompt task in
the group is recorded as **failed** rather than **interrupted**, contradicting
the spec's "interrupted tasks are recorded as such, the group is marked
interrupted."

**This is not purely a Windows defect.** On Unix the second press sends
`SIGKILL` → exit `137`, and a provider that traps `SIGINT` and exits `0`/`1`
also misses. The `130` match happens to hold for the single-press, well-behaved
provider — the exact case the Level-3 test exercises.

**It is also untested at any level.** Every case in
`lib/src/composition/sequence/task/tests.rs` reaches this contract through
`FakePrompt`, which takes `interrupted` as a constructor argument
(`tests.rs:452-475`) — including
`an_interrupted_provider_reports_interruption_rather_than_failure`
(`tests.rs:1285`), which asserts that a hardcoded `true` propagates. The
derivation from a real provider exit is the seam that can be wrong, and no test
touches it.

**Required change.** Add `|| self.run.interrupted.load(Ordering::SeqCst)` to
match `iterate.rs`, or — better, and the structural fix — thread
`ProcessTermination` through `SingleCompositionOutcome` so both call sites stop
inferring interruption from a magic number. Cover it with an L1 test that drives
a stub provider exiting non-`130` while the shared flag is set.

### 3. High — AC11's gates have not been run against this round of changes

The recorded gate table in `validation-matrix.md:335-342` is unchanged from
review 3: `3773 tests run` (L1), `142 tests run` (L2). This round adds
approximately 19 L1 tests (13 coordinator, 4 handle, 2 placement) and 2 L2 tests
— the counts alone prove the table predates the work it is meant to certify.

That matters more than bookkeeping here, because one new test changes the L2
tier's cost profile substantially.
`level2_prompt_idle_flush_keeps_the_task_bar_in_tmux` stalls ~80 s by
construction (`SILENCE_WINDOW` and ticker cadence are both a hardcoded 30 s with
no env override, and the stall must clear the *second* tick), with a
`run_in_pane_within` deadline of 150 s and a bespoke `.config/nextest.toml`
slow-timeout grant of `terminate-after = 6`. A tier that ran in 43.6 s now has a
single test that can consume three times that. Nothing records what
`just test-l2 --no-fail-fast` actually costs or returns now.

The reasoning behind that test is careful and its expense is honestly argued —
the post-stall marker really is what discriminates idle flush from `close()`
drain, and there is no cheaper honest version. But an unmeasured tier cost is
still unmeasured.

**Required change.** Re-run `just lint`, `just test --no-fail-fast`, and
`just test-l2 --no-fail-fast` from `claudine/`, and re-run `just test-l3`.
Record verbatim summaries and the new wall-clock for the L2 tier. Note the host
load, per the repository's drift-bracket convention.

### 4. Medium — `validation-matrix.md` now asserts as current several things the code has fixed

Per this repository's drift rule (code is authoritative; drifted prose is a
defect), the matrix contains stale claims that a reader would take as the
present state:

- `validation-matrix.md:149-152` — "`wrap/exec/termination/windows.rs` has no
  Ctrl+C handling on the simple wait path, so AC1's Ctrl+C/exit-`130` behavior
  is currently Unix-only." Both halves are now false:
  `wait_with_signal_handling` runs `windows_wait_loop`, and
  `register_sequence_interrupt_flag` is the flag's Windows producer.
- `validation-matrix.md:453-478`, "Open Windows defects found by this audit (not
  fixed here)" — defect 1 (simple-path no-op) and defect 2 (Job handle leak) are
  both fixed. Only defect 3 survives (see finding 5).
- `validation-matrix.md:314` — "Windows runtime execution | **Not run** … Windows
  evidence is a successful `--target x86_64-pc-windows-gnu` *compile* of lib+bin
  plus a source audit." Still true, but it now omits that two Windows test
  suites exist and cannot compile, which is the material fact (finding 1).
- The test inventory (`validation-matrix.md:22-40`) lists neither the new
  coordinator/handle suites nor `level2_windows_sequence_ctrl_c.rs`.

**Required change.** Rewrite those sections against the current tree. Where a
defect is fixed but unverified, say exactly that — "fixed in source, no executed
evidence" — rather than either "open" or "closed."

### 5. Low — `mod windows` is gated on `not(unix)` while the `windows` crate is gated on `windows`

Review 3's audit recorded this as Windows defect 3; it is untouched.
`termination/mod.rs:45-47` gates `mod windows` on `#[cfg(not(unix))]`, but
`cli/Cargo.toml:62` supplies the `windows` crate only under
`[target.'cfg(windows)'.dependencies]`. Real Windows is fine; a target that is
neither builds a module whose dependency is absent, failing with an unhelpful
unresolved-crate error rather than a stated non-support.

The new `#[cfg_attr(unix, allow(dead_code))] mod coordinator;` / `mod handle;`
declarations are the right pattern in contrast — compiled everywhere so their
bookkeeping is unit-testable, instantiated only where the platform calls exist.

**Required change.** Gate `mod windows` on `#[cfg(windows)]` and add an explicit
`#[cfg(not(any(unix, windows)))] compile_error!("…")` naming the unsupported
platform.

### 6. Low — `StreamParseError::Fatal` is never constructed, so one repaired path is genuinely unreachable

`level2_sequence_task_stream_capture.rs`'s module doc claims the
post-parser-failure raw fallback "is **not** reachable from any provider
stream: no `SemanticStreamParser` implementation constructs
`StreamParseError::Fatal`." I verified this and it is correct — across
`claudine/lib/src` and `claudine/cli/src`, `Fatal` appears only at its
declaration (`stream/parser.rs:9`) and at two match arms
(`wiring/session.rs:157`, `spawn/semantic.rs:367`). Nothing produces it.

Deciding not to force an unreachable path into a pane test was the right call,
and saying so in the doc rather than quietly skipping it is the right practice.
But the conclusion should propagate: the variant is dead, and the L1 test at
`spawn/semantic.rs:667-696` proves an emitter no input can reach.

**Required change.** Either delete `StreamParseError::Fatal` and its two match
arms, or identify the parser condition that should produce it and make one do
so. Leaving a variant that only tests construct is the outcome to avoid.

### 7. Low — The Windows console handler writes interrupt feedback outside the synchronized render sink

`windows.rs:88-95`, `emit_interrupt_feedback`, writes directly to
`std::io::stderr()` from the handler thread. The spec's Reporting Concurrency
section requires "one synchronized render sink so ANSI sequences and wrapped
lines cannot be torn by sibling writers," and a press during a parallel group is
exactly when sibling writers are active.

Unix has an excuse — its emission happens inside a signal handler where
allocation and locking are unsafe. The Windows handler runs on a dedicated
thread with the full Rust API available, so it can take the sink lock. The
docstring's "both hosts therefore show the same wording" justifies the *bytes*
but inherits a constraint that does not apply.

Low severity: the write is a short static byte string, so the practical tear
window is small. Worth a comment recording the deliberate choice if it is not
changed.

### 8. Low — `install_user_interrupt_guard` remains a no-op on Windows

Not sequence-plus's contract, but the Windows cross-compile now emits it as
evidence: `mark_user_interrupted` and `wait_loop_active`
(`cli/src/output/mod.rs:618,644`) are `never used` warnings on the Windows
target, because `compose/interrupt.rs:93-97` compiles to an empty guard there.
So `claudine compose` / `inline-compose` on Windows has no `USER_INTERRUPTED`
marking and no second-press force-exit — while `windows.rs:370` still installs
`WaitLoopActiveGuard`, whose only reader is Unix-only.

The new coordinator makes this cheap to fix: `register_sequence_interrupt_flag`
is already the general shape, and a compose-scoped flag registration would reuse
it. Flagged because AC1 retains "Ctrl+C exit `130`" as covered behavior and the
sequence path is now ahead of the compose path on this host.

## Requirement Verification Levels

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| State normalization, generated fields, neighbors, `{{state}}` name coercion | L1 unit + CLI | Appropriate |
| Dynamic expression/shell/file sources, `ListFormat`, offsets, operators, typed source errors | L1 unit + CLI | Appropriate |
| Direct and referenced formal documents share template/schema behavior | L1 unit + CLI parity | Appropriate |
| Static preflight, approved-byte parity, JIT rereads, runtime `set`, reserved precedence, `outputs` chaining | L1 unit + CLI | Appropriate |
| Serial/parallel scheduling, `max_parallel`, snapshots, deterministic merge, failure policy | L1 concurrency + CLI E2E overlap timing | Appropriate |
| Interactive missing-property collection | L1 pseudo-TTY | Appropriate |
| Shell-task colored bars, narrow-pane geometry, invisible-bar alignment, no-color labels, glyph fallback, zero-step notice | L2 tmux capture | Appropriate — and it found a real `TaskStreamFrame::quote()` wrapping defect |
| Prompt/provider assistant, reasoning, and tool lines keep task attribution in a real pane | **L2 tmux capture (new)** | **Closed.** `level2_parallel_prompt_streams_keep_task_attribution_in_tmux` stubs `claude` (not `goose`, which has no `stream_protocol`), extracts per-line `38;2;R;G;B` triples, and requires the bar set painting each marker to equal the task-header bar set |
| Assistant idle flush surfaces a held block under the owning task's bar | **L2 tmux capture (new)** | **Closed.** The post-stall marker discriminates idle flush from `close()` drain — the assertion a cheaper test could not make |
| Post-parser-failure raw fallback keeps attribution | L1 only, path unreachable | Acceptable given finding 6; resolve by deleting the variant |
| Data/status channel split for prompt tasks | L1 CLI E2E (`parallel_prompt_task_splits_data_and_status_across_channels`) | Appropriate — pipes are the only place channel identity is observable |
| User Ctrl+C stops a sequence, fans out to every parallel child, suppresses later work, exits `130` (macOS) | **L3 OS keyboard, passing** | **Closed.** SIGINT-immune children (`trap '' INT`) keep terminal process-group delivery from producing a false positive; the harness was fixed, not the assertion |
| The same contract on Windows | Source audit + production cross-compile (verified green this session) | **Gap.** Design-complete, zero executed evidence; its own tests cannot compile — finding 1 |
| A parallel group's prompt task is *recorded* as interrupted | L1 against a stub that hardcodes the answer | **Gap.** The derivation is untested and wrong on Windows and on non-`130` Unix exits — finding 2 |
| Job Object kill-on-close fires at the step boundary; no handle leak | L1 drop-order/ownership (cross-platform) + Windows-host tests that cannot compile | **Partial.** The bookkeeping is proven everywhere; the Win32 behavior is proven nowhere |
| macOS / Windows / Linux release gates | macOS gates recorded but stale; Linux partial; Windows compile-only | **Gap** — findings 1 and 3 |

## Prior Review Closure

- **Review 3 Finding 1 (Windows Ctrl+C and parallel fan-out) — closed in design,
  open in evidence.** Every required change was made and made well: a real
  Windows sequence-scoped producer, the simple wait path routed through the
  shared machinery, and process-global press/force-kill atomics replaced by a
  per-registration coordinator. The required Windows-host integration test was
  written. It cannot compile. Downgraded from "confirmed defect" to "unverified
  fix" — a genuine improvement, not a closure.
- **Review 3 Finding 2 (Level-3 keyboard evidence) — closed for macOS.** Green
  through `just test-l3`, fixed in the harness (frontmost-polling +
  AppleScript-modifier chord) rather than by weakening the test, and fenced by
  two new guards. Windows OS-keyboard evidence remains out of reach and depends
  on finding 1.
- **Review 3 Finding 3 (prompt/provider attribution at the wrong level) —
  closed.** Two of three paths now have real-pane coverage; the third is
  unreachable by construction (finding 6). The `claude`-not-`goose` observation
  is the crux: every pre-existing sequence fixture stubbed a provider with no
  stream protocol and therefore proved nothing about the semantic spawn.
- **Review 3 Finding 4 (Job handle leak) — closed in design, open in evidence.**
  `OwnedRawHandle` is the right shape, and parameterising the closer so
  drop-order is testable off-Windows is a better answer than the review asked
  for. The two Windows-host regressions it needs cannot compile.
- **Review 3 Finding 5 (release gates) — not closed.** Findings 1 and 3 above.

## Verification Performed

- Read `spec.md`, `review-3.md`, `validation-matrix.md`, and every changed and
  added file in the working tree (`git status --porcelain`, `git diff`).
- `RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/cl-win-target cargo check -p claudine-cli --target x86_64-pc-windows-gnu --color=never`
  → `Finished dev profile [unoptimized + debuginfo] target(s) in 49.01s`, exit
  `0`. Two `never used` warnings on `cli/src/output/mod.rs:618,644` are the
  evidence for finding 8.
- `cargo nextest run -p claudine-cli --no-fail-fast -E 'test(/coordinator|handle|focus_stealing|test_placement/)' --color=never`
  → `Summary [1.878s] 62 tests run: 62 passed, 2260 skipped`.
- Traced the interrupt-to-exit-code path by reading
  `sequence/mod.rs`, `iterate.rs`, `report.rs`, `task_run.rs`, and
  `composition/mod.rs::SingleCompositionOutcome` — finding 2.
- Grepped `claudine/lib/src` and `claudine/cli/src` for every
  `StreamParseError::Fatal` occurrence to test the L2 module doc's
  unreachability claim — finding 6.
- Did **not** run `just test`, `just test-l2`, or `just test-l3`. The L2 tier now
  carries a test that stalls ~80 s and the L3 tier takes over the desktop, which
  the new `_test_l3` guard correctly refuses to let a non-interactive agent do.
  Running them is the developer's step, and is finding 3.
- Did not run the Windows suites; no Windows host or emulation is available, and
  they do not compile from here regardless.

## Production Readiness Closure

Readiness requires findings 1, 2, and 3 to close; 4 through 8 are cleanups that
should ride along.

Concretely, the shortest honest path is: fix the `interrupted` derivation
(finding 2) and cover it; gate the 7 pre-existing Unix-only test sites and break
the duckdb dev-dependency so the Windows test targets compile (finding 1); get
one native Windows run; then re-run the three canonical gates and record them
verbatim (finding 3).

The functional design of Sequence Plus is, at this point, in good shape. What
stands between it and production is that its most delicate platform path has
never executed — and that the tests written to make it execute cannot yet be
built.
