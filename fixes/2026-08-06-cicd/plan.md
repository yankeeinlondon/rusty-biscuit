---
title: Per-package test resolution — retire the package area as a CI concept
status: draft
created: 2026-08-06
spec: fixes/2026-08-06-cicd/spec.md
builds_on:
  - fixes/2026-07-30-ci-cd-stabilization
source_code:
  - .github/ci/areas.json
  - .github/ci/ci-baseline.toml
  - .github/workflows/_area-ci.yml
  - .github/workflows/ci.yml
  - just/devops.just
  - scripts/ci-rollup.rs
  - scripts/ci-rollup-tests.rs
  - scripts/ci/affected_scope.py
  - scripts/ci/test_affected_scope.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs:
  - .github/ci/README.md
  - fixes/2026-08-06-cicd/plan.md
  - fixes/2026-08-06-cicd/spec.md
---

## Objective

Deliver `spec.md`: the package becomes the unit of selection, execution, and
result identity, and the package area is removed as a CI concept.

## Shape of the change

```
BEFORE   scope → 31 directory records → one job per directory → recipe loops over its packages
                                                                results keyed {directory, env, tier}

AFTER    scope → impacted packages    → one job per package    → `_test <package>`
                                                                results keyed {package, env, tier}
         package needs ← its own Cargo.toml
         environment capabilities ← one table
```

The per-package execution primitives (`_test`, `_test_l2`, `_sanity`) already
exist. What is missing is a package-keyed matrix, package-keyed results, and
package-owned requirements.

## Phase 1 — Move package requirements into manifests

**Files:** the manifests of the ~20 packages with a declaration to move,
`scripts/ci/affected_scope.py`, `scripts/ci/test_affected_scope.py`

Add `[package.metadata.ci]` support and migrate the three genuine package
facts out of `areas.json`:

- `native` → 4 packages
- `backends` → `l2-backends`, 8 packages
- `ci: false` + `reason`/`owner`/`expiry`/`exclusion_class` → `gates = false`,
  10 packages

Also add the single environment-capability table, replacing 8 `policy_gaps`
records that restate two facts.

`areas.json` still exists and is still read — nothing is cut over yet.

**Verification.** The scope calculator resolves identical policy from manifests
as it does from `areas.json`, for every package. Assert equivalence
mechanically rather than by inspection: a test that loads both and diffs them,
which then gets deleted in Phase 5.

**Why first:** it is additive, reversible, and its correctness is provable
against the thing it replaces while that thing is still present.

## Phase 2 — Derive what is currently declared

**Files:** `scripts/ci/affected_scope.py`, `scripts/ci/test_affected_scope.py`

Replace declaration with derivation (R7):

- L2 / browser / node capability — from the package's own test files
- `check_args` — from the package name plus any manifest-declared features

Remove sharding entirely (R7a): drop the `shards` records, stop passing
`--partition`, and drop shard from every job name. Measured justification is in
`spec.md § Sharding` — compilation is ~85% of a shard and every shard pays it in
full, so four shards cost ~3.2× the compute to save ~2.4 minutes.

**Verification.** Derived values match today's declared values for all 72
packages, with every difference explained in the phase notes rather than
absorbed. A difference is either a bug in the derivation or a stale
declaration; both need naming.

For sharding: no workflow passes `--partition`, and the `darkmatter` and
`claudine` suites still run every test they ran before — removing sharding must
drop the partitioning, never the tests. Compare executed-test counts against the
pre-change run, not just job outcomes.

## Phase 3 — Fan out per package

**Files:** `.github/workflows/ci.yml`, `.github/workflows/_area-ci.yml` (renamed),
`tools/test-toolkit/tests/ci_workflow_contracts.rs`

The matrix iterates impacted packages. The reusable workflow takes a package and
invokes `_test <package>` rather than a directory recipe. The workflow file is
renamed away from `_area-`.

**Verification.** Contract tests: matrix is package-derived; every job names one
package; manifest-declared requirements reach their jobs. Contract suite
otherwise unchanged — confirm the three known pre-existing WSL failures are *the
same three* by diffing against a clean worktree at `origin/main`, not by reading
the count.

## Phase 3a — Re-key the build cache

**Files:** `.github/workflows/_area-ci.yml` (renamed)

`Swatinem/rust-cache` is keyed per directory —
`area-ci-${area}-test-${environment}`. With the package as the unit that key is
wrong, and what replaces it is the single biggest influence on whether this work
reduces runtime at all: compilation is ~85% of a test job.

Start from a package-scoped key. Do not settle it by argument — Phase 6 measures
it, and a key that produces worse cache hit rates than today is a regression
regardless of how tidy the matrix looks.

**Verification.** Cache hit rate and total job wall-clock, compared against the
pre-change run for the same commit content. Record both in the phase notes.

## Phase 4 — Re-key results and the known-failure list

**Files:** `scripts/ci-rollup.rs`, `scripts/ci-rollup-tests.rs`,
`.github/ci/ci-baseline.toml`

`CellKey` becomes `{package, environment, tier}` through the rollup, verdict,
and comparison paths. The producers already record the package in every JUnit
report; it is currently discarded in favour of the directory.

Handle R10: a package with no tests records "nothing to run", distinguishable
from both a pass and a missing result.

Then re-key all 32 known-failure entries to the packages that actually fail,
read from the first run producing package-keyed results.

**These land together.** Between re-keying results and re-keying the list, every
entry is unmatched and everything blocks. Identify the bridging run before
starting this phase — it must be full-scope to cover every entry.

**Verification.** The verdict on that run reports zero `baseline-no-result` and
zero `baseline-now-passing`. The set of *excused failing tests* is identical
before and after — same tests, different keys.

## Phase 5 — Delete `areas.json`

**Files:** `.github/ci/areas.json`, `.github/ci/README.md`,
`scripts/ci/affected_scope.py`, `tools/test-toolkit/tests/ci_workflow_contracts.rs`

Remove the file, its loader, its schema validation, and the Phase 1 equivalence
test. Rewrite the README around the package as the unit.

**Verification.** A contract test asserts no workflow, recipe, or script reads
`areas.json`. `just check-canonical` still passes — it validates that
directories expose canonical recipes for developers, which is unaffected by how
CI selects work.

## Phase 6 — Prove it on a real run

**Files:** none

Measurement is a phase, not a footnote. On a branch touching exactly one package
of a multi-package directory, record before and after:

1. number of test jobs scheduled;
2. packages actually tested;
3. wall-clock from run start to verdict;
4. that a dependent package elsewhere still runs (R2);
5. **build-cache hit rate**, against the pre-change run.

Item 5 is not optional. Compilation is ~85% of a test job, so a per-package
matrix sitting on a worse cache key can schedule fewer jobs and still take
longer. Fewer jobs is not the objective; less time is.

**Exit condition:** a reorganised matrix that does not reduce observed runtime
has not met the objective. Argument-list inspection does not close this.

## Risks

**Losing reverse-dependency expansion.** The most damaging misreading available:
narrowing the dependency closure instead of the directory fan-out would silently
stop testing consumers of a changed package. R2, Phase 1's closure test, and
Phase 6's fourth check exist for this.

**The Phase 4 window.** Results and the known-failure list must be re-keyed
together or the merge gate is unusable in between. Plan the bridging run first.

**Dropping tests while dropping shards.** Removing `--partition` must remove the
partitioning and nothing else. The failure mode is a change that silently
narrows the suite rather than reuniting it — four shards each running a quarter
looks very like one job running a quarter. Phase 2 compares executed-test counts
against the pre-change run for exactly this reason.

**Silent zero-test jobs.** A package whose job runs nothing and exits 0 reports
as a pass. R10 and Phase 4's handling are the guard — the same class of
inversion as an empty `-p` list checking the whole workspace.

**Derivation disagreeing with declaration.** Phase 2 will find packages where
the derived capability differs from the declared one. Each difference must be
named as either a derivation bug or a stale declaration. Silently accepting the
derived value would let a real capability regression land inside a refactor.

**Full-scope runs get larger.** ~72 packages × environments rather than ~31 ×
environments. Acceptable because full scope is now rare, but measured in Phase 6
rather than assumed harmless.

**A worse build cache eating the gain.** The largest risk to the objective, and
the least visible. Compilation is ~85% of a test job; more jobs against a
colder or more fragmented cache can take *longer* in total while looking like
progress on every other metric. Phase 3a starts the key, Phase 6 measures it,
and neither closes on job count alone.

## Non-goals

- Making the specialised workflows (`messenger`, `playa`, `biscuit-tui`,
  `rendezvous`) visible to the merge verdict. Real gap, different problem.
- Wiring up the expected-test manifest. Referenced by no workflow today.
- Reducing the 32 known failures. They are re-keyed, not fixed.
- Changing what developers run locally.

## Open questions for review

1. **`gates = false`.** Ten exclusions carry `reason`, `owner`, `expiry`.
   Moving them to manifests preserves that governance. Confirm that is wanted
   rather than treating non-gating as derivable.
2. **Cutover shape.** Phases 1–2 keep `areas.json` alive as a reference to prove
   equivalence against. Is that transitional period acceptable, or should the
   cutover be atomic?

Sharding is settled: removed (`spec.md § Sharding`), with build-once/run-many
recorded there as a future option rather than a follow-up commitment.
