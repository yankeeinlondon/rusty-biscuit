---
title: Deferred — the AC8 cold and warm performance measurements
kind: deferral
created: 2026-09-15
spec: fixes/2026-09-12-single-os-compile/spec.md
plan: fixes/2026-09-12-single-os-compile/plan.md
baseline: fixes/2026-09-12-single-os-compile/baseline-2026-09-12.md
rollout: fixes/2026-09-12-single-os-compile/rollout-2026-09-12.md
review: fixes/2026-09-12-single-os-compile/review-2.md
finding: The required performance acceptance gate has no usable baseline
tasks:
    - "1.6"
    - "7.4"
    - "7.5"
acceptance_criterion: AC8
revision: 8aa105e7c5a180b2d2a10ffbed2332b88671c996
measured_revision: fb8681b86902bc3bfe67dfee76bb35727b611eee
status: deferred; the instrument was retired 2026-09-16, observations not collected
---

# Deferred — the AC8 cold and warm performance measurements

This document exists because the work it describes was **not done**. It maps to
Review 2's third finding, *"The required performance acceptance gate has no
usable baseline"* ([review-2.md](review-2.md)), and it is the reason plan Tasks
1.6, 7.4, and 7.5 remain unchecked and AC8 remains `NOT MET` in
[rollout-2026-09-12.md](rollout-2026-09-12.md).

## The instrument was retired on 2026-09-16

`scripts/ci/build_baseline_revision.py` and its suite
`scripts/ci/test_build_baseline_revision.py` were **deleted**, and the suite was
removed from `SUITE_REGISTRY` and from `repo-deps`'s `companion-suites`.
Recover either from the commit that removed them.

They could no longer do their job. The instrument constructs an
instrumentation-only revision by copying `CARRIER_PATHS` onto base
`8aa105e7`, and that list named `scripts/Cargo.lock`, which PR #79 deleted when
it made `scripts` a member of the root workspace. Dropping the stale carrier
takes the suite from 8 failures to 1, but it does not make the result usable:
the construction would copy today's `scripts/Cargo.toml` — now a workspace
*member* — onto a base whose root manifest does not list `"scripts"` and which
carries its own `scripts/Cargo.lock`. The constructed revision would be a member
with no workspace and no lockfile, so it could not be dispatched, which is the
only thing it exists for. Keeping the suite would have meant a green test
attesting to an instrument that cannot run — the exact defect this branch spent
its time removing.

**What a replacement has to decide first.** Re-basing onto post-#79 `main` is
mechanically possible, but the measurements taken on 2026-09-16 suggest the
original comparison is no longer the one worth running: the compile-once
architecture captures 24 of 24 available savings, yet that is 10.5% of full-scope
compiles and **zero** for a single-package L1-only change, while every executing
tier cell now pays archive transfer it did not pay before. PR #81, merged the
same day, independently removes compile work on exactly that common case. A
replacement should measure **archive transfer overhead against compiles avoided
on a representative single-package pull request**, and adopting it is an explicit
amendment to AC8 rather than a substitution made quietly.

## What is deferred

Six hosted dispatches per environment on **each** of two revisions — three cold
and three warm — plus the ruling those twelve observations feed. Concretely:

| Deferred item | Plan task | Where its rows live |
|---|---|---|
| Pre-cutover cold and warm observations | 1.6 | [baseline-2026-09-12.md](baseline-2026-09-12.md) |
| Post-cutover cold and warm observations | 7.4 | the same tables, collected on `feat/single-os` |
| The explicit ruling against the 15% band | 7.5 | recorded in the plan and the rollout when made |

Nothing is estimated, extrapolated, or inferred in their place. No number
appears anywhere in this fix cycle that was not measured, and the single-owner
architecture is **not** recorded as accepted.

## Why it could not be performed in this session

Three consecutive green *hosted* runs per environment can only be produced by
pushing a revision and dispatching `ci.yml` against it repeatedly. Every session
that worked this fix cycle was instructed not to commit, not to push, and not to
dispatch hosted CI, and had no authorization to consume hosted runner capacity.
A local `cross-check` run measures a rig rather than a hosted critical path and
is not a substitute; neither is a local control measurement, which is recorded
in the baseline precisely so a reader can tell a broken instrument from a real
change and for no other purpose.

Review 2 found a second, structural reason, which was real and is now removed:
the instrumentation and the ownership cutover arrived as one working-tree
change, so dispatching this branch could only ever measure the post-cutover
schedule, and the merge base it would be compared against carried no instrument.
The pre-cutover half of AC8's comparison had no revision to run on at all.

## What was built instead, so the deferral is executable

`scripts/ci/build_baseline_revision.py` constructs the missing pre-cutover
revision deterministically:

```sh
python3 scripts/ci/build_baseline_revision.py --explain
# fb8681b86902bc3bfe67dfee76bb35727b611eee
```

It replays the Phase 1 instrumentation of Tasks 1.4 and 1.5 — the compiler-work
counter, its `_ci_build_counter` and `_ci_build_report` recipes, and the
per-cell measurement publication — onto the merge base `8aa105e7c` and nothing
else, then writes an unreferenced commit through `git commit-tree`. It moves no
branch, touches no index, and signs nothing.

Its guarantees, each enforced by `scripts/ci/test_build_baseline_revision.py`
against a temporary repository seeded from the real merge base:

- The counter tool is copied byte for byte from the post-cutover tree, so the
  instrument is the same object on both sides of the comparison and its own
  build cost cancels rather than approximately cancelling.
- Every measured command is byte-identical to the merge base's. Timing comes
  from sibling steps; the wrapper arrives through the gate step's `env:` map.
- The construction is additive: it removes exactly one line from the merge
  base's scheduling surface, `workflow_dispatch: {}`, which has to grow an
  input. A cutover cannot hide in a tree that deletes nothing, so no owner job,
  build record, or archive production is present.
- Re-running the script yields the same commit id. The published id is pinned
  by a test, so a changed counter tool fails rather than silently invalidating
  the baseline; `--update-documents` refreshes every document that names it.

This is the review's first option — an instrumentation-only pre-cutover
revision — not its fallback. No replacement comparison is claimed or needed.

## The exact procedure to perform it

Run the whole procedure twice, once per revision, changing only `REF`.

1. **Publish the pre-cutover revision.** It is unreferenced until someone names
   it:

   ```sh
   git push origin \
     fb8681b86902bc3bfe67dfee76bb35727b611eee:refs/heads/baseline/ac8-pre-cutover
   ```

2. **Collect three cold observations.** Cold is controlled by cache deletion and
   by nothing else:

   ```sh
   REF=baseline/ac8-pre-cutover     # then repeat the whole procedure with feat/single-os
   env HOME=/Users/ken gh cache list --limit 100 --json key,id \
       --jq '.[] | select(.key | startswith("package-ci-")) | .id' \
     | while read -r id; do env HOME=/Users/ken gh cache delete "$id"; done
   env HOME=/Users/ken gh workflow run ci.yml --ref "$REF" -f measure-compiler-work=true
   ```

3. **Collect three warm observations.** Dispatch again without clearing the
   cache, immediately after a green run on the same revision, so
   `Swatinem/rust-cache` restores the key that run saved.

4. **Download the measurements and the job timings** for each run:

   ```sh
   env HOME=/Users/ken gh run download <run-id> -p 'measurement-*' -D ./measurements
   env HOME=/Users/ken gh api \
       repos/yankeeinlondon/rusty-biscuit/actions/runs/<run-id>/jobs \
       --paginate --jq '.jobs[] | {name, started_at, completed_at}'
   ```

5. **Fill the tables** in [baseline-2026-09-12.md](baseline-2026-09-12.md) and
   the matching Task 7.4 tables, then apply the rule below and record the
   ruling.

[baseline-2026-09-12.md](baseline-2026-09-12.md) carries the field-by-field
schema, the `queued_seconds` derivation, and the WSL2 two-row split those steps
write into.

## The acceptance rule

- **15% band.** A delta within 15% is noise. A regression above it requires an
  explicit design ruling under Task 7.5 and may not be explained away by
  transfer time, fewer tests, or a warm cache.
- **Three consecutive green runs per environment**, in each of the cold and warm
  conditions, on each revision. Consecutive is ordered by `run_number` on one
  revision with no intervening dispatch of the same condition; green per
  environment means every `{package, environment, gate}` cell of that
  environment concluded `success`. A red or cancelled run restarts the count for
  every environment it touched.
- **Matched identities.** The same packages, the same `[package.metadata.ci]`
  feature arguments, the same Nextest profile, and the same cache condition on
  both sides. An unmatched pair is not a comparison and may not be ruled on.
- **Two reported quantities, never folded together.** Total runner compute — the
  sum of `completed_at - started_at` over every job, with Windows reported
  separately because its minutes bill at a different multiplier — and the
  end-to-end critical path, `run_started_at` to the last job's `completed_at`.
- **Compiler invocations are the primary column, not duration.** A restored
  cache that still recompiles is fast and wrong; only the invocation count
  separates "reused the producer's binaries" from "recompiled them quickly".

## What closes each task

Neither a first hosted run nor merging this branch closes anything here, and any
document that says otherwise is wrong — the plan's Task 7.5 note is the
authority.

- **Task 1.6** closes when the pre-cutover tables hold three consecutive green
  cold observations and three warm ones per environment.
- **Task 7.4** closes on the same condition for the post-cutover revision, with
  transfer and setup cost identified rather than folded into test time.
- **Task 7.5** closes only on an explicit recorded ruling against the 15% band,
  once both halves exist. If the measurements accept single-owner, that
  acceptance is recorded and compatibility cohorts stay out. If they reject it,
  the specification's reviewed alternatives are chosen among — deterministic
  planner-declared compatibility cohorts first, one owner per build key
  retained, dependencies duplicated across cohorts reported — and neither a
  return to one owner per package nor a remote cache is adopted silently.

Until all three close, nobody should claim this change made CI faster.
