---
fix: 2026-09-07-faster-claudine-tests
phase: 3
created: 2026-09-08
revision: 9fc5151a0df52acb63c5458b1564aa9c01bfe9b1
host: aarch64-apple-darwin (Darwin 27.0.0), 16 cores
toolchain: rustc 1.97.1 (8bab26f4f 2026-07-14)
nextest: cargo-nextest 0.9.136
---

# Cost attribution — the eight Claudine-area packages

Phase 3 of [`plan.md`](plan.md), the first half of RB5. Document-only: no
package source file was changed. It answers the spec's four
[draft decisions](spec.md#draft-decisions-to-resolve-during-baseline-review)
with measurements, and records why the fourth cannot be closed yet.

[`inventory.md`](inventory.md) says what exists and who owns it. This file says
what it costs. The two never disagree about ownership because the attribution
gate joins run logs to the same [`families.json`](families.json) the reconciler
uses, through the reconciler's own matcher.

> **Every number below is local, and local numbers are attribution only.**
> They explain where time goes on this host. They set no target. Budgets are
> [refused](#draft-decision-4--budgets) until Phase 1's CI baseline exists.

## How to reproduce

```bash
# per-family attribution over the recorded gate logs
npx tsx attribution.ts \
  baseline/local-gates/just-test.log \
  enumeration/recipes/just-test-rendezvous.log \
  baseline/local-gates/just-test-l2.log \
  --families families.json

# per-binary attribution, which is also per-process under nextest
npx tsx attribution.ts baseline/local-gates/just-test.log \
  --families families.json --by-binary --top 20

# the budget gate, which refuses until the CI tranche lands
npx tsx attribution.ts baseline/local-gates/just-test-cli.log \
  --families families.json --budgets attribution/budgets-pending.json
```

[`attribution.ts`](attribution.ts) rejects a truncated log, a log that is not a
nextest run at all, a run whose result lines disagree with its own summary, a
run containing any non-passing result, a duplicate identity, and an identity
that no family or two families claim. Its suite is
[`attribution.test.ts`](attribution.test.ts) — 55 tests,
`npx tsx --test attribution.test.ts`.

Isolated per-binary runs are in [`attribution/runs/`](attribution/runs/), their
three-column summary in
[`attribution/isolated-runs.tsv`](attribution/isolated-runs.tsv), the
shared-process comparison in
[`attribution/shared-process-runs.tsv`](attribution/shared-process-runs.tsv),
and the launch-CWD probe's output in
[`attribution/launch-cwd.tsv`](attribution/launch-cwd.tsv).

## The concurrency fact that decides which column matters

`.config/nextest.toml` binds two CI-profile test groups over this area:

| Group | Filter | max-threads |
|---|---|---:|
| `claudine-l1` | `package(claudine)`, non-tiered tests | 4 |
| `claudine-cli-ci-l1` | `package(claudine-cli)`, non-tiered tests | 1 |

**On CI, `claudine-cli`'s whole L1 suite runs one test at a time.** Summed test
duration is therefore that leg's floor, not a number parallelism hides: the
289.12 s this host sums for `just test-cli` behind 18.12 s of elapsed time is
289 s of CI wall clock plus per-process startup. `claudine`'s library suite is
capped at four, so its 259 s summed has a ~65 s floor.

Two consequences run through everything below.

1. **Summed duration is the attribution currency**, and local elapsed is the
   misleading one. A family that is invisible locally because sixteen cores
   absorb it is fully visible on the runner that executes it serially.
2. **Local summed duration is not the CI number either.** Under 16-way local
   parallelism, contention inflates per-test duration — `error_guards` sums
   20.23 s alone and 34.01 s inside the full suite, a 68% inflation of the same
   work. Serial execution removes that inflation but adds per-process startup.
   Neither correction can be applied by arithmetic, which is the concrete
   reason the plan forbids deriving a budget from a local run.

## Attribution by family

From Phase 1's recorded gate logs (`just test`, `just test-l2`) plus Phase 2's
`just test-rendezvous` log — 7,373 results, 1,196.94 s summed, 115.55 s of
runner elapsed. Gate output pasted verbatim.

> **These are `9fc5151a0` numbers and are not restated.** They matched
> [`inventory.md`'s family index](inventory.md#family-index) row for row when
> both were taken. The inventory has since been reconciled to retaken captures
> and six families' identity counts moved, so those six now differ here; the
> other forty still agree. Re-measurement is deferred with a named blocker —
> see [`results.md`](results.md) § Measurement re-run. The gate that produced
> this table replays the same logs against a snapshot of the same-era
> `families.json`, so the table stays reproducible from its own inputs.

| Family | Package | Tests | Summed | Mean | Max | Share |
|---|---|---:|---:|---:|---:|---:|
| `cli-l2-lifecycle` | `claudine-cli` | 114 | 200.35 s | 1.757 s | 6.82 s | 16.7% |
| `lib-unit` | `claudine` | 3833 | 196.58 s | 0.051 s | 6.35 s | 16.4% |
| `cli-l2-render-capture` | `claudine-cli` | 74 | 188.12 s | 2.542 s | 15.33 s | 15.7% |
| `rz-daemon-unit` | `rendezvous-daemon` | 153 | 107.75 s | 0.704 s | 6.80 s | 9.0% |
| `cli-l1-fixture` | `claudine-cli` | 282 | 75.09 s | 0.266 s | 4.33 s | 6.3% |
| `lib-unit-task-shell` | `claudine` | 106 | 58.14 s | 0.548 s | 2.06 s | 4.9% |
| `cli-l1-pty-interactive` | `claudine-cli` | 19 | 54.61 s | 2.874 s | 4.71 s | 4.6% |
| `cli-l2-autocomplete` | `claudine-cli` | 27 | 44.68 s | 1.655 s | 3.59 s | 3.7% |
| `cli-l1-source-scan-guards` | `claudine-cli` | 79 | 39.53 s | 0.500 s | 3.21 s | 3.3% |
| `cli-l1-raw-sequence-loop` | `claudine-cli` | 154 | 34.76 s | 0.226 s | 2.34 s | 2.9% |
| `cli-unit` | `claudine-cli` | 1173 | 34.08 s | 0.029 s | 1.04 s | 2.8% |
| `cli-l1-raw-context-resources` | `claudine-cli` | 83 | 29.07 s | 0.350 s | 2.84 s | 2.4% |
| `rz-daemon-integration` | `rendezvous-daemon` | 16 | 25.46 s | 1.591 s | 2.31 s | 2.1% |
| `cli-l1-pty` | `claudine-cli` | 11 | 19.19 s | 1.745 s | 3.19 s | 1.6% |
| `cli-l1-raw-compose` | `claudine-cli` | 69 | 14.86 s | 0.215 s | 0.92 s | 1.2% |
| `gen-generate-ux` | `claudine-gen` | 10 | 8.34 s | 0.834 s | 1.47 s | 0.7% |
| `cli-unit-completion-engine` | `claudine-cli` | 322 | 8.01 s | 0.025 s | 0.25 s | 0.7% |
| `cli-unit-child-exec` | `claudine-cli` | 199 | 7.59 s | 0.038 s | 1.22 s | 0.6% |
| `cli-l2-ctrl-c` | `claudine-cli` | 3 | 6.90 s | 2.299 s | 2.73 s | 0.6% |
| `gen-artifact-pipeline` | `claudine-gen` | 51 | 5.42 s | 0.106 s | 0.59 s | 0.5% |
| `cli-override-context-width` | `claudine-cli` | 1 | 5.21 s | 5.215 s | 5.21 s | 0.4% |
| `rz-client-integration` | `rendezvous-client` | 7 | 4.21 s | 0.602 s | 0.73 s | 0.4% |
| `cli-override-loop-rate-limit` | `claudine-cli` | 1 | 4.04 s | 4.043 s | 4.04 s | 0.3% |
| `lib-corpus-replay` | `claudine` | 82 | 3.63 s | 0.044 s | 0.21 s | 0.3% |
| `cli-l1-completion-ui` | `claudine-cli` | 79 | 3.16 s | 0.040 s | 0.10 s | 0.3% |
| `gen-l2-report` | `claudine-gen` | 3 | 2.75 s | 0.916 s | 1.55 s | 0.2% |
| `lib-unit-model-catalog` | `claudine` | 71 | 2.66 s | 0.037 s | 1.28 s | 0.2% |
| `cli-l1-fixture-selftest` | `claudine-cli` | 14 | 2.44 s | 0.174 s | 0.26 s | 0.2% |
| `cli-l1-perf-budget` | `claudine-cli` | 12 | 2.34 s | 0.195 s | 0.33 s | 0.2% |
| `lib-unit-atomic-config` | `claudine` | 5 | 1.55 s | 0.311 s | 1.20 s | 0.1% |
| `gen-unit` | `claudine-gen` | 93 | 1.24 s | 0.013 s | 0.04 s | 0.1% |
| `rz-core-unit` | `rendezvous-core` | 82 | 0.91 s | 0.011 s | 0.02 s | 0.1% |
| `cli-override-perf-parity` | `claudine-cli` | 2 | 0.82 s | 0.411 s | 0.45 s | 0.1% |
| `contract-unit` | `claudine-contract` | 48 | 0.78 s | 0.016 s | 0.06 s | 0.1% |
| `cli-l1-shipped-corpus` | `claudine-cli` | 6 | 0.57 s | 0.095 s | 0.42 s | 0.0% |
| `lib-unit-clock-streams` | `claudine` | 24 | 0.52 s | 0.022 s | 0.06 s | 0.0% |
| `lib-structural-guards` | `claudine` | 18 | 0.42 s | 0.023 s | 0.10 s | 0.0% |
| `cli-l1-md-fixture` | `claudine-cli` | 1 | 0.30 s | 0.296 s | 0.30 s | 0.0% |
| `catalog-types-unit` | `claudine-catalog-types` | 21 | 0.29 s | 0.014 s | 0.02 s | 0.0% |
| `rz-client-unit` | `rendezvous-client` | 14 | 0.18 s | 0.013 s | 0.01 s | 0.0% |
| `lib-override-agent-errors-remediation` | `claudine` | 1 | 0.13 s | 0.130 s | 0.13 s | 0.0% |
| `lib-tts-contract` | `claudine` | 5 | 0.07 s | 0.013 s | 0.01 s | 0.0% |
| `lib-agent-errors-fleet` | `claudine` | 1 | 0.04 s | 0.045 s | 0.04 s | 0.0% |
| `cli-override-empty-state-routing` | `claudine-cli` | 1 | 0.04 s | 0.045 s | 0.04 s | 0.0% |
| `lib-model-catalog-integration` | `claudine` | 2 | 0.03 s | 0.017 s | 0.02 s | 0.0% |
| `lib-override-seeded-loop` | `claudine` | 1 | 0.03 s | 0.028 s | 0.03 s | 0.0% |

The four families with no local cost (`contract-real-provider`,
`cli-l3-keyboard`, `cli-real-provider`, and the 27 identities the local recipes
do not select) remain **pending**, as
[`inventory.md`](inventory.md#family-index) records. Nothing here treats a
missing measurement as a cheap one.

### The three costs, kept apart

The area's recipes do not separate build/setup from runner elapsed — Phase 2
recorded that as a limitation of the local evidence, and it still holds for the
full-suite logs. The isolated per-binary runs below **do** separate them,
because the probe times the whole recipe invocation while nextest reports its
own elapsed:

| Cost | What it is here | Where it comes from |
|---|---|---|
| build/setup | `wall − elapsed`: `_ensure-md`, `_storage_preflight`, cargo's freshness check, nextest's list phase | `attribution/isolated-runs.tsv` |
| runner elapsed | nextest's own `Summary [ … ]` | same |
| summed test duration | Σ of the per-test durations nextest prints | same |

**A focused recipe invocation costs ~1.2 s before a single test runs.**
`diagnostic_discovery` (2 tests, 0.009 s elapsed, 0.02 s summed) takes 1.22 s of
wall clock; `run_harness_loop_call_sites` and `shipped_prompt_route_drift` land
in the same place. That floor is the same on every focused run in the table, so
it is a property of the recipe, not of any test. It is not a finding against
the recipe — the freshness check is what makes the run trustworthy — but any
future claim about a single binary getting faster must subtract it.

## Draft decision 1 — which non-spawn families account for the remaining cost

**Answer: five, in this order — and only one of them is a spawn family.**

| Rank | Family | Summed | What the time actually is | Owner |
|---|---|---:|---|---|
| 1 | `cli-l2-lifecycle` + `cli-l2-render-capture` + `cli-l1-pty-interactive` | 443.08 s | Real terminal panes driving a real lifecycle for the first two. The third was measured on the L2 route but owns no emulator session (review-1 finding 1) and now runs at L1. | out of L1 scope for the first two; Phase 6 re-measures |
| 2 | `lib-unit` + `lib-unit-task-shell` | 254.72 s | `composition::sequence` 94.15 s / 257 tests and `composition::schema` 43.43 s / 75 tests dominate; the shell-task cases run real child process trees | Phase 6 (schema corpus), Phase 7 (shell waits) |
| 3 | `rz-daemon-unit` | 107.75 s | 153 unit tests at a 704 ms mean — the highest unit-test mean in the eight packages — in a crate the parent area's `just test` never runs | Phase 7 (endpoints/cleanup) |
| 4 | `cli-l1-source-scan-guards` | 39.53 s | One `syn` parse of `lib/src` + `cli/src` + `contract/src`, repeated once per test process | Phase 6 — see [decision 2](#draft-decision-2--which-source-scans-can-share-work) |
| 5 | `cli-l1-raw-context-resources` (`context_command` alone: 27.87 s) | 29.07 s | 22 of 27 cases launch the binary **inside the checkout**, so each pays a full monorepo context capture | Phase 5C, quantified below |

Everything below rank 5 is under 20 s summed, and the entire
`cli-unit` + `cli-unit-child-exec` + `cli-unit-completion-engine` block —
1,694 identities, the largest count in the area — costs 49.68 s at a 0.029 s
mean. **The count is not where the cost is.**

### The launch-CWD tax, measured

`context_command.rs` calls `current_dir(repo_root())` in 22 of its 27 cases.
[`attribution/launch-cwd-probe.ts`](attribution/launch-cwd-probe.ts) runs the
same `claudine context --values` from the monorepo root and from a directory
outside any repository:

| CWD | 1 process | 8 concurrent | 16 concurrent | 27 concurrent |
|---|---:|---:|---:|---:|
| monorepo root | 672 ms | 1,953 ms | 3,812 ms | 6,040 ms |
| outside a repository | 20 ms | 24 ms | 36 ms | 52 ms |

A single invocation is **34× more expensive** from the checkout; twenty-seven
concurrent invocations — which is exactly what nextest does with this binary —
are **116×**. The same asymmetry is visible inside the test binary without any
probe: the five `context_side_effects_*` cases, which do not capture repository
context, run in 0.027–0.061 s, while the 22 that launch from the checkout run in
0.9–4.0 s.

This is *not* a claim that repository context is wasted work in production. The
checkout run renders 183 package rows, 264 scope rows, 177 document rows and 109
file-change rows that the outside-repository run does not have to produce; the
cost is proportional to real content. It is a claim about the **tests**: they
assert that a column header exists and that `ctx.today` is non-null, and they
buy a full 35-member monorepo capture to do it. Phase 5C's migration to the
fixture is what removes it, and this is the number that migration should move.

### What is *not* the cost

Two plausible suspects were measured and cleared, so Phase 6 does not spend
effort on them:

- **Binary startup is not the problem.** 27 concurrent launches of the 265 MB
  debug binary from outside a repository complete in 52 ms total. Process
  creation, dynamic loading and paging are noise at this scale.
- **A checkout-rooted spawn is not automatically expensive.** `errors_command.rs`
  also uses `current_dir(repo_root())` in all five of its cases and costs
  0.105 s elapsed / 0.50 s summed, because `claudine errors` captures no
  repository context. The tax is paid by *what the command does*, not by the
  CWD alone — so Phase 5C's migration is worth the most on `context_command`
  and should not be justified by CWD hygiene arguments elsewhere.

## Draft decision 2 — which source scans can share work

**Answer: exactly one — the twelve corpus cases inside `error_guards`. Every
other scan should stay where it is.**

Measured in isolation, three repetitions each
(`attribution/isolated-runs.tsv`; medians shown):

| Binary | Processes | Summed | Elapsed | Min case | Parser | Roots |
|---|---:|---:|---:|---:|---|---|
| `error_guards` | 18 | 20.23 s | 1.72 s | 0.012 s | `syn` AST | `lib/src`, `cli/src`, `contract/src` |
| `spawn_inventory` | 2 | 1.65 s | 1.64 s | 0.007 s | `syn` AST | `lib/src`, `cli/src` |
| `composition_seams` | 19 | 1.07 s | 0.36 s | 0.008 s | text | `lib/src`, `cli/src` |
| `dispatch_inventory` | 12 | 0.53 s | 0.22 s | 0.010 s | text | `lib/src`, `cli/src` |
| `test_placement` | 9 | 0.35 s | 0.24 s | 0.007 s | text | `lib/src`, `cli/src`, `cli/tests` |
| `spawn_site_guard` | 15 | 0.26 s | 0.047 s | 0.009 s | text | `cli/tests` |
| `run_harness_loop_call_sites` | 2 | 0.04 s | 0.018 s | 0.019 s | text | `cli/tests` |

The `syn` parse is the whole story. It costs ~1.7 s; a text scan of the same
roots costs 0.05–0.36 s. The split inside `error_guards` is stark: **twelve of
its eighteen cases take 1.74–1.85 s and six take 0.012–0.017 s.** The cheap six
are unit tests of the scanner itself (`scan_text` against synthetic fixtures);
the expensive twelve each call `scan_production_sources()`, whose `OnceLock`
cache is process-local and therefore shares nothing under nextest's
process-per-test model.

The counterfactual is measurable rather than theoretical. `cargo test` runs the
same binary's tests in **one** process, so the `OnceLock` does what it was
written to do:

| Binary | nextest (process per test) | `cargo test` (one process) | Repeated work |
|---|---:|---:|---:|
| `error_guards` | 20.23 s summed / 18 processes | **1.61 s** for the same 18 assertions | ~18.6 s |
| `spawn_inventory` | 1.65 s / 2 processes | 1.64 s | ~0.00 s (one case scans) |
| `dispatch_inventory` | 0.53 s / 12 | 0.21 s | ~0.32 s |
| `composition_seams` | 1.07 s / 19 | 0.36 s | ~0.71 s |
| `test_placement` | 0.35 s / 9 | 0.23 s | ~0.12 s |

(`attribution/shared-process-runs.tsv`. This is a *measurement* of the shared
scan, not a proposal to run the suite under `cargo test` — that would forfeit
per-test isolation, the leak policy and selective execution.)

So the recoverable repeated work in this cohort is **~18.6 s of summed duration
in one binary**, and ~1.2 s in all the others combined. Recommendation for
Phase 6:

- **Consolidate `error_guards`' twelve corpus cases, and nothing else.** The
  house shape applies directly: one passive corpus pass, extended per
  regression rather than rescanned per regression. Whatever form Phase 6
  chooses, it must keep each of the twelve assertions individually named in the
  failure output — they are the reason the guard is usable — and must keep the
  six scanner unit tests independently selectable, since they are the arms that
  prove the scanner is not blind.
- **Leave the text scanners alone.** Merging all five would recover ~1.2 s and
  would cost independent failure attribution and selective execution across
  five unrelated contracts. That is the trade the spec explicitly warns
  against.
- **Do not attempt cross-binary sharing.** `error_guards` and `spawn_inventory`
  are the only two `syn` consumers, and `spawn_inventory` pays the parse once.
  A cross-process cache would recover ~1.6 s and would couple two guards
  through a cache-invalidation contract that has to be right on every platform.

## Draft decision 3 — which technical exceptions remain necessary

**Answer: none.** Every live-child site's remaining need is expressible at its
call site once Phase 4 ships a `std::process::Command` carrying the fixture
policy. The `NEEDS_LIVE_CHILD` reason can close.

| File | What it needs beyond `assert_cmd` | Verdict after Phase 4 |
|---|---|---|
| `wrap_sigint.rs` | live child + `kill` + drain + reap | raw path; call site keeps signal delivery, marker poll, reaping |
| `handle_deadline.rs` | live child to observe the deadline killing it | raw path; one of its two cases stays on `assert_cmd` |
| `compose_ttff_perf.rs` | streaming stdout for time-to-first-frame | raw path; call site keeps the stream read |
| `wrap_ctrl_c_windows.rs` | `creation_flags(CREATE_NEW_PROCESS_GROUP)` + console event | raw path + `creation_flags` at the call site |
| `sequence_ctrl_c_windows.rs` | same | same |
| `level1_compose_autocomplete_failure_pty.rs` | a `std::process::Command` for `Session::spawn` | raw path, unchanged call shape |
| `level1_inline_compose_mismatch_pty.rs` | same | same |
| `sequence_overlay_pty.rs` | same | same |

Two corrections to the plan's grounding facts, both checked in the tree:

1. **`spawn_inventory.rs` is not a live-child file.** Its `.spawn()`
   occurrences (lines 380–390) are inside a raw-string fixture that the
   scanner's own unit test parses. The live-child cohort is **five** files plus
   the three PTY binaries, not six.
2. **`common/pty.rs` does not construct the session command.** It owns the
   draining and marker helpers; the commands are built in the three PTY
   binaries (`compose_command`, `sequence_command`,
   `sequence_command_no_provider`, and two inline `Command::new(cargo_bin(…))`
   sites). Phase 5D's task should name those builders, because editing
   `common/pty.rs` alone would change nothing.

`expectrl`'s `Session::spawn` takes a `std::process::Command` by value, so the
PTY binaries adopt the raw path with no change to their call shape — which is
the strongest single argument that the raw path is the right design and that no
exception survives it.

One item for Phase 4's helper-command audit rather than for the spawn gate:
`wrap_ctrl_c_windows.rs` shells out to a raw `Command::new("rustc")` to build
its stub, and `system_prompt_perf_bench.rs` to a raw `Command::new("git")`.
Isolating the claudine child does not isolate those.

## Draft decision 4 — budgets

**Not ratified. Refused, mechanically, with the refusal recorded.**

The plan requires budgets to be derived from Phase 1's CI baseline and written
into `inventory.md` beside the baseline table. Phase 1's CI half is
[blocked](log.md#blocked-the-ci-half): the predecessor has not merged, so no
post-merge run exists, so there is no baseline to derive from. Local timing may
attribute cost; it may not set a target. Publishing a "provisional budget" from
the tables above would be exactly the act the checkpoint forbids, and would be
harder to retract than to never write.

So the refusal is code. [`attribution/budgets-pending.json`](attribution/budgets-pending.json)
declares the four legs and their zero collected runs, and the gate refuses:

```text
$ npx tsx attribution.ts baseline/local-gates/just-test-cli.log \
    --families families.json --budgets attribution/budgets-pending.json
…
attributed 2489 result(s); summed 289.12 s; runner elapsed 18.12 s
4 violation(s):
  [missing-leg] ubuntu-latest: declared but carries no measurements
  [missing-leg] macos-latest: declared but carries no measurements
  [missing-leg] windows-latest: declared but carries no measurements
  [missing-leg] wsl2-ubuntu: declared but carries no measurements
GATE EXIT=1
```

Handed local measurements instead, it refuses on provenance before it reads a
single number:

```text
1 violation(s):
  [local-derived-budget] budgets are derived from the CI baseline only; local
  timing attributes cost and sets no target
GATE EXIT=1
```

### The ratification procedure, fixed now

Fixing the procedure before the numbers exist is what stops a budget from being
fitted to whatever the candidate run happened to produce.

1. **Input.** For each configured leg — `ubuntu-latest`, `macos-latest`,
   `windows-latest`, `wsl2-ubuntu` — the three consecutive green baseline runs
   Phase 1 collects, gated by `junit-metrics.ts` and grouped by family by
   `attribution.ts aggregate` with the same `families.json` used here.
2. **Statistic.** Per leg and family, `observedMax` = the largest summed
   duration across that leg's three runs. Worst-of-three, not a mean: a budget
   that the baseline itself breaches one run in three is not a budget.
3. **Headroom.** `budget = observedMax × 1.25`, rounded up to 0.01 s. One
   declared constant, applied identically everywhere, so no family can acquire
   a bespoke allowance during review.
4. **Per leg, never merged.** Windows and WSL2 budgets stand on their own
   evidence. Matched tests are compared within an environment; additions and
   platform exclusions are reported separately.
5. **Refusals.** Fewer than three green runs on a leg, a declared leg with no
   measurements, a family with fewer samples than runs, or any non-CI
   provenance yields **no budget** and a named violation.
6. **Publication.** The resulting table lands in
   [`inventory.md` § Budgets](inventory.md#budgets), beside the baseline table,
   so budget review and evidence review are one act.
7. **After the fact.** A miss is reported with its cause. The budget is not
   adjusted to cover it (AC7).

The procedure is executable today — only its input is missing — and each rule
above has a named test in `tools/test-audit/tests/attribute.test.ts`.

### Step 1 shipped, 2026-09-09

The one part of the procedure that had no implementation was the join from a
JUnit `<testcase>` to a family: `attribution.ts` reads nextest *console logs*
and the CI evidence is XML, so `perLegFamilySummed` stayed empty and every leg
was refused with `missing-leg` — the wrong reason. `attribution.ts aggregate`
(shared implementation in `tools/test-audit/src/attribute/aggregate.ts`) closes
it. It walks the stored `<run-id>/<env>/<tier>/<package>.xml` staging trees and
resolves every case through the reconciler's own `familiesMatching`, so the
attribution gate and the reconciliation gate cannot drift on family membership.

It is a gate, not a report. A leg contributes its one sample per family only
when that run's evidence for that leg is clean; a red run (non-zero manifest
`exit_code` or a failed case), a malformed or missing report, a manifest record
naming another environment, a declared leg absent from the tree, or an identity
that zero or two families claim disqualifies the leg for that run rather than
being averaged in. Rule 5's refusals are untouched.

Against the one stored baseline run:

```text
$ npx tsx attribution.ts aggregate baseline/34173378609 --out /tmp/agg.json
… 18 families on each Unix leg, 15 on windows-latest;
  summed duration identical to results.md § CI baseline
GATE EXIT=0

$ npx tsx attribution.ts budgets --runs /tmp/agg.json
4 violation(s):
  [insufficient-runs] ubuntu-latest: 1 green run(s); 3 consecutive are required
  [insufficient-runs] macos-latest: 1 green run(s); 3 consecutive are required
  [insufficient-runs] windows-latest: 1 green run(s); 3 consecutive are required
  [insufficient-runs] wsl2-ubuntu: 1 green run(s); 3 consecutive are required
GATE EXIT=1
```

That is the correct end state and the whole of the change: the refusal moved
from "nothing joins the evidence to the families" to "one run is not three".
`attribution/budgets-pending.json` is left as it was — `runsPerLeg` 1, an empty
`perLegFamilySummed`, and the pre-existing `missing-leg` refusal reproducible
verbatim — because the aggregator's output is what replaces it once the runs
exist, and editing it by hand would fabricate evidence.

## Timeout and clock semantics these numbers were taken under

Every floor and budget recorded by this fix is derived under the clock the
implemented
[startup-stall fix](../_completed/2026-08-31-silent-success-and-startup-stall/spec.md)
installed, not the pre-fix first-event grace. Concretely:

- The silence clock's origin is the child's monotonic spawn instant until the
  first activity signal exists; before the fix, `step_timeout` only started
  counting once `LiveMetrics::last_activity_at()` returned a value, so a
  provider that hung before its first signal was bounded by nothing except the
  opt-in wall-clock `timeout` (which defaults to `None`).
- Detection and signalling are bounded by `step_timeout` + one watchdog
  interval; `kill_grace` sits outside that bound, so a test that observes the
  whole termination path budgets `step_timeout` + interval + `kill_grace`.
- Production defaults are `interval` 5 s and `kill_grace` 10 s
  (`cli/src/commands/wrap/exec/timeouts.rs`), both overridable by
  `CLAUDINE_WATCHDOG_INTERVAL` and `CLAUDINE_KILL_GRACE`.

The families whose floors are these semantics rather than incidental slowness:

| Family / binary | Summed | The contract its floor expresses |
|---|---:|---|
| `wrap_watchdog_startup_stall` (in `cli-l1-fixture`) | 12.75 s / 5 | `CLAUDINE_STEP_TIMEOUT` 2 s, `CLAUDINE_WATCHDOG_INTERVAL` 0.2 s, `CLAUDINE_KILL_GRACE` 0.5 s; its `REAP_DEADLINE_MS` is exactly the sum. The 2 s is measured **from spawn** — this binary could not exist under the pre-fix clock, because its `SilentForever` provider never produces a first event |
| `wrap_watchdog_timeout` (in `cli-l1-fixture`) | 8.83 s / 6 | same three variables, wall-clock rule |
| `cli-override-loop-rate-limit` | 4.04 s | a real rate-limit reset ~3 s in the future, with `CLAUDINE_PAUSE_RESET_MARGIN` trimming production's 5 s margin to 1 s. The pause is the assertion |
| `loop_cli::compose_loop_step_timeout_surfaces_as_iteration_failure` | 2.59 s | per-iteration step timeout under the same clock |
| `handle_deadline` | 0.43 s / 2 | `CLAUDINE_HANDLE_DEADLINE_SECONDS` 5 s and 1 s |

These floors are **not** Phase 6/7 reduction targets. Phase 7's audit is for
sleeps used as readiness proof, not for sleeps that are the contract; each of
the above is the latter and each already parameterises its budget through an
environment variable rather than hard-coding a wall-clock wait.

## Findings

New in this phase, none of them found by looking at a slow-test threshold.

1. **The CI concurrency caps invert the local reading of the suite.**
   `claudine-cli` L1 is `max-threads = 1` on CI, so its summed duration is the
   leg's floor. Any Phase 8/9 comparison that quotes local elapsed as the
   headline will understate both the problem and the improvement by an order of
   magnitude.
2. **`error_guards` repeats one 1.7 s `syn` parse twelve times**; the same
   twelve assertions cost 1.61 s in a single process. This is the largest
   provably recoverable block of repeated work in the L1 suite.
3. **The launch-CWD tax is 34× per process and 116× at the concurrency nextest
   actually uses**, and it is confined to commands that capture repository
   context — `context_command`, not `errors_command`.
4. **The focused-recipe setup floor is ~1.2 s**, which bounds what any
   single-binary improvement can show on a wall-clock measurement.
5. **The plan's live-child cohort was one file too large**, and its Phase 5D
   task names `common/pty.rs` where the work is in three PTY binaries.
6. **The plan's link to the startup-stall spec is stale.** It points at
   `../2026-08-31-silent-success-and-startup-stall/spec.md`; the fix has been
   archived to `_completed/`. Corrected in this document; Phase 10 should fix
   the plan's own link when it closes.

### Production defects surfaced: none filed

The launch-CWD measurement is the only candidate, and it does not meet the bar.
`claudine context --values` costs 672 ms at the monorepo root against 20 ms
outside a repository, but at the monorepo root its four repository-derived
sections carry 733 rows (183 packages, 264 scope entries, 177 documents, 109
file changes) that no run outside a repository has to produce. That is work proportional to output, not a
defect, and this phase has no evidence of a redundant walk behind it. Recorded
here rather than filed, so a later phase can reopen it with a work counter
(Phase 8's mechanism) if the ratio turns out to be worse than the content
explains.

## Validation checkpoint 3

| Requirement | Status |
|---|---|
| Every draft decision has a written answer backed by a measurement | **3 of 4 answered**; decision 4 is answered by a documented, tested refusal plus a fixed procedure, because its input is blocked |
| Budgets are in `inventory.md` next to the baseline | **procedure, tooling and refusal are** — step 1's JUnit → family join shipped 2026-09-09; the numbers are **pending** on two more green runs per leg |
| No budget was derived from a local run alone | **done** — no budget exists, and the gate refuses local provenance by construction |
| Cost attributed by family against the baseline, three columns separate | **done** for the local evidence; the CI baseline's three columns remain pending |
| Source-scan families measured individually, with process counts | **done** — seven binaries, three repetitions, plus the shared-process counterfactual |
| Context/render families and corpus loaders measured | **done** — `context_command`, `composition_seams`, the loop pause tests, `shipped_prompts`, `shipped_prompt_contract`, `shipped_prompt_route_drift` |
| Live-child cohort enumerated, per-file exception verdict | **done** — no exception survives Phase 4 |
| Clock semantics recorded | **done** |
| Production defects filed rather than fixed | **done** — none met the bar; the one candidate is recorded with its evidence |
