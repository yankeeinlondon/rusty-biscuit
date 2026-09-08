---
fix: 2026-08-01-cli-slow-tests
created: 2026-09-07
---

# Deferred performance measurements — `2026-08-01-cli-slow-tests`

Performance verification that a review asked for and an implementation cycle
could not legitimately produce. Each entry names the finding it maps back to,
the review that raised it, and precisely what has to happen before it can close.

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
