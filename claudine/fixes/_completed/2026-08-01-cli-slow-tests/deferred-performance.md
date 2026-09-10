---
fix: 2026-08-01-cli-slow-tests
created: 2026-09-07
updated: 2026-09-08
---

# Deferred performance measurements — `2026-08-01-cli-slow-tests`

Performance verification that a review asked for and an implementation cycle
could not legitimately produce. Each entry names the finding it maps back to,
the review that raised it, and precisely what has to happen before it can close.

## Status as of 2026-09-07 (successor's Phase 1)

Read by
[`2026-09-07-faster-claudine-tests` Phase 1](../../2026-09-07-faster-claudine-tests/log.md),
whose gate is to resolve both items in writing before any of its own code lands.

| Item | Status |
|---|---|
| 1 — AC4 CI targets, four environments × three green runs | **OPEN, 1 of 3 collected.** Merged to `main` as `444213eb5` (PR #69, 2026-09-08); the first post-merge run is green on all four legs and stored. See the 2026-09-08 addendum. |
| 2 — Windows compilation of the `#[cfg(windows)]` arms | **CLOSED** on 2026-09-08: the `windows-latest` leg (MSVC) compiled and ran the arms green. See the 2026-09-08 addendum. |

## 1. Acceptance criterion 4 — CI targets on four environments × three green runs

- **Maps back to:** finding 2 of
  [review-3.md](review-3.md) (high) — "Acceptance criterion 4 is still
  unstarted; the Outcome table has no evidence". Re-raised from finding 2 of
  [review-2.md](review-2.md).
- **Also blocks:** acceptance criterion 6's CI half (timeout budgets proven by
  CI duration, not local wall clock) and acceptance criterion 8's
  exercised-on-four-environments half.
- **Deferred during:** implementation cycle 3, 2026-09-07.

### What is required

The spec's [Outcome table](spec.md) and acceptance criterion 4 demand, read from
the `junit-claudine-cli-L1-<environment>` JUnit artifacts of **three consecutive
green runs** on each of `ubuntu-latest`, `macos-latest`, `windows-latest`, and
`wsl2-ubuntu` — twelve data points:

- zero migrated tests at or over 5 s on any environment;
- zero non-timeout migrated tests at or over 2 s on the native runners, at most
  ten on WSL2;
- slowest non-timeout migrated test at or under 1.5 s native, 4 s WSL2;
- each timeout-shaped test within budget + tick + 1 s (native) or + 2 s (WSL2);
- serial sums at or under the Outcome table for both binary groups.

### Why it was deferred, and why no local number can stand in

Not a CPU-load problem — a **committed-state** problem. Nothing on this branch
is committed, so no post-change CI run exists, and committing and pushing are
separate human-driven steps this session is not authorized to take. The
implementation cycle produced code, not history.

Even with an idle host, a local run cannot substitute. The spec forbids it in
terms: *"Local runs are for attributing cost, never for proving a target."* The
reference environments are 3–4 vCPU hosted runners with `max-threads = 1` for
this test group, and the WSL2 leg reads its workspace through the Windows disk
at 20–50× the local cost for exactly these test shapes. A 16-core Mac measures a
different machine.

### What is already in place

- **The measurement is written and baseline-validated.**
  [junit-metrics.ts](junit-metrics.ts) carries the binary lists, the nine
  timeout-shaped tests with their `budget`/`tick` floors, and environment
  discovery. Run against the baseline run `33440897014`'s four still-live
  artifacts it reproduces the spec's Problem-section table to the decimal
  (237.9 / 158.7 s Ubuntu, 178.2 / 118.4 s macOS, 4.9 / 63.5 s Windows,
  1207.3 / 846.7 s WSL2; 6 / 3 / 4 / 89 tests ≥ 5 s; 14.3 / 9.6 / 8.8 / 77.1 s
  slowest).
- **[inventory.md](inventory.md)'s "2026-09 follow-up" section** holds the
  reproduction recipe, the computed baseline table, and an explicitly empty
  post-change table naming what blocks it.
- **Local attribution, labeled local and not offered as proof:** one
  `NEXTEST_PROFILE=ci` run on this host gives 0 tests ≥ 5 s, 0 ≥ 2 s
  non-timeout, 0.6 s slowest, 28.5 s / 7.0 s serial sums, and all nine timeout
  tests inside budget + tick + 1 s.

### How it closes

1. Commit and push the branch.
2. Collect three consecutive green runs across the four environments
   (PR runs and `main` runs both count; `NEXTEST_PROFILE=ci` JUnit artifacts are
   the record).
3. `gh run download` the artifacts and run `junit-metrics.ts` over them.
4. Fill [inventory.md](inventory.md)'s post-change table. **A run that misses a
   target is recorded as a miss with its cause, never adjusted to fit** (spec,
   Required behavior 6).

The riskiest row is the WSL2 leg: `sequence_per_step_step_timeout_override`'s
`0.5s` step budget has the thinnest margin under contention, and the plan
already flags it for re-measurement when the 2026-08-31 startup-stall spec's
silence clock lands.

## 2. Windows compilation of the `#[cfg(windows)]` arms

- **Maps back to:** finding 3 of [review-3.md](review-3.md) (high) — "The
  Windows arms cannot be compiled on this host — CI is the only compiler".
- **Deferred during:** implementation cycle 3, 2026-09-07. Only the *compile*
  half is deferred; the two locally-actionable halves of finding 3 (the
  bare-name utility audit and the `inherit_no_env` console-variable restoration)
  were implemented in this cycle.

### Why it was deferred

`cargo check --target x86_64-pc-windows-msvc -p claudine-cli --tests` fails
before it reaches the test crate:

```
error occurred in cc-rs: aws-lc-sys … jitterentropy-base-windows.h:49:
fatal error: 'windows.h' file not found
```

The target is installed, but a transitive native dependency needs the Windows
SDK headers, which this macOS host does not have. This is not a load or timing
constraint and no amount of host idleness changes it.

### What is unverified anywhere

- The three `.env()` calls inside `restore_windows_console_variables`'
  `#[cfg(windows)]` block (`PATHEXT`, `COMSPEC`, `SystemRoot`).
- The Windows arm of `inherit_no_env_keeps_the_defaults_and_drops_everything_else`,
  which lost its `#[cfg(unix)]` gate in this cycle precisely so that acceptance
  criterion 8's claim about that escape becomes evidence rather than assertion.
- `minimal_system_path()`'s `%SystemRoot%\System32` arm and the `.cmd` recording
  stub in `cli_process_fixture.rs`.

The honest risk is that a cleared Windows environment is missing a *fourth*
thing claudine needs, in which case one named test is the first red on the
`windows-latest` leg, with a recorded environment dump. The cfg-independent
decisions (the `C:\Windows` fallback, the variable names) were deliberately
hoisted out of `cfg` blocks so a typo is a macOS compile error rather than a
`windows-latest` surprise.

### How it closes

The same push that closes item 1. The `windows-latest` leg is the first
compiler these arms will ever see.

### Addendum, 2026-09-07 — the compile half closed without the push

The paragraph above was wrong on one point, and the successor's Phase 1 found it
while running the gate: `windows-latest` is *not* the only compiler available.
The failing command recorded above targets **MSVC**
(`x86_64-pc-windows-msvc`), which drags in `aws-lc-sys` and its Windows SDK
header requirement. The area's own `just check-windows` recipe targets **mingw**
(`x86_64-pc-windows-gnu`, `--tests`, `-Wa,-mbig-obj`), which has no such
dependency. That recipe was never run during implementation cycle 3.

Run on the same macOS host at `9fc5151a0`, after
`rustup target add x86_64-pc-windows-gnu` supplied the missing standard library:

```text
cargo check -p claudine -p claudine-cli --tests --target x86_64-pc-windows-gnu
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 37s
EXIT=0
```

So every arm in the "what is unverified anywhere" list above now compiles:
`restore_windows_console_variables`' three `.env()` calls, the Windows arm of
`inherit_no_env_keeps_the_defaults_and_drops_everything_else`,
`minimal_system_path()`'s `%SystemRoot%\System32` arm, and the `.cmd` recording
stub in `cli_process_fixture.rs`. A typo in those arms is now a local error
rather than a `windows-latest` surprise.

**Still open, and not weakened by this.** Compiling is not running. The honest
risk stated above — that a cleared Windows environment is missing a *fourth*
thing claudine needs — is a runtime question that only the `windows-latest` leg
answers. mingw is also not MSVC, so anything MSVC-specific in the native
dependency graph is still unproven. Item 2 therefore closes fully with item 1's
push, not before.

**Incidental finding.** The mingw check emits three unused-import warnings the
host build does not, all residue of `#[cfg(unix)]`-gated cases:
`claudine/cli/tests/wrap_basics.rs:7` (`std::fs`),
`claudine/cli/tests/wrap_basics.rs:9` (`common::wrap::*`), and
`claudine/cli/tests/compose_caller_file_provenance.rs:5` (`write_executable`).
Warnings, not errors; carried to the successor's Phase 5, which edits both
files.

### Addendum, 2026-09-08 — the push happened; one run of three is in

Written by the successor's Phase 9
([log](../../2026-09-07-faster-claudine-tests/log.md) § Phase 9).

The branch was merged to `main` as `444213eb5` (PR #69, 00:27 UTC); the merge
tree is identical to the PR head `a9e88c069`. The `ci` run on that push,
`34173378609`, is green on every leg, and its four
`junit-claudine-cli-L1-<env>` artifacts are stored under the successor's
[`baseline/34173378609/`](../../2026-09-07-faster-claudine-tests/baseline/34173378609/)
with the gate's verbatim output beside them:

| Leg | Build/setup | Runner elapsed | Summed | Tests | Failures |
|---|---:|---:|---:|---:|---:|
| `ubuntu-latest` | 695.1 s | 324.9 s | 324.7 s | 2466 | 0 |
| `macos-latest` | 832.3 s | 753.7 s | 753.5 s | 2466 | 0 |
| `windows-latest` | 1012.9 s | 356.1 s | 355.8 s | 2105 | 0 |
| `wsl2-ubuntu` | 81.3 s | 692.7 s | 692.2 s | 2466 | 0 |

All nine timeout-shaped tests are inside budget + tick + allowance on the three
Unix legs (worst margin: `sequence_per_step_step_timeout_override` at 1.3 s
against 1.6 s on macOS). They are `#![cfg(unix)]` and do not exist on
`windows-latest`, which the successor's gate now records as a declared platform
exclusion rather than a missing test.

**Item 1 stays open — one run of three.** `main` moved thirteen hours later
(PR #70, `6504747e2`, which touches `claudine/lib` and four `claudine/cli/tests`
files), so subsequent `main` pushes are a different source state. Two more
samples at `444213eb5` itself need `gh run rerun 34173378609`, an operator call.
The PR's own `pull_request` run `34159725015` (same tree) is stored as a
supplementary sample and not counted.

**Item 2 closes.** `claudine-cli / check (claudine-cli on windows-latest)` and
`claudine-cli / test (claudine-cli on windows-latest)` both succeeded on the
MSVC runner, and the two live-child console-control tests that no local host
could execute — `wrap_ctrl_c_windows::ctrl_c_terminates_wrapped_child_on_windows`
and `sequence_ctrl_c_windows::sequence_ctrl_c_fans_out_to_parallel_children_on_windows`
— ran and passed there. The "fourth thing a cleared Windows environment might
need" did not materialize. The two `wrap_basics.rs` unused-import warnings from
the mingw check are still emitted (`just check-windows`, 2026-09-08, exit 0);
the `compose_caller_file_provenance.rs` one is gone.
