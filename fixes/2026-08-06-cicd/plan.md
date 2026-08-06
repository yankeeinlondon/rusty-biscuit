---
title: Per-package test resolution — retire the package area as a CI concept
status: ready
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
  - claudine/justfile
  - homelab/justfile
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

Deliver `spec.md` (accepted 2026-08-06): the package becomes the unit of
selection, execution, and result identity, and the package area is removed as a
CI concept.

## Shape of the change

```
BEFORE   scope → 31 directory records → one job per directory → recipe loops over its packages
                                                                results keyed {directory, env, tier, shard}

AFTER    scope → gating impacted packages → one L1 job per package per env  → `_test <package>`
                                            + declared L2/browser tier jobs → `_test_l2` / `_test_browser`
                                                                results keyed {package, env, tier}
         package policy            ← its own Cargo.toml, [package.metadata.ci]
         environment capabilities  ← .github/ci/environments.json
```

The per-package execution primitives (`_test`, `_test_l2`, `_test_browser`,
`_lint`) already exist in `just/devops.just`. What is missing is a
package-keyed matrix, package-keyed results, and package-owned requirements.
`_sanity` is not a substitute for the full L1 suite, and compile-check remains
`cargo check --all-targets -p <package>` (R9).

## Landing shape

Development is staged as the phases below, but the checked-in cutover is
atomic (spec § Migration):

- **Phases 1–2 may merge ahead.** They are additive: manifests and
  `environments.json` exist but `areas.json` remains the sole authority, so no
  merged state has two authoritative policy stores.
- **Phases 3, 3a, 4, and 5 land as one merge**, preceded by the full-scope
  bridging run on the branch. The same merge moves all consumers off
  `areas.json`, deletes it, bumps the result schema, and installs the verified
  package baseline. No merged state emits both area-keyed and package-keyed
  result documents.

## Phase 1 — Package policy into manifests; one environment table

**Files:** the manifests of the packages with policy to declare,
`scripts/ci/affected_scope.py`, `scripts/ci/test_affected_scope.py`,
`.github/ci/environments.json` (new)

Add `[package.metadata.ci]` support and migrate package facts out of
`areas.json` and out of the area justfiles:

- `native` → 4 packages
- `backends` → `l2-backends`, 8 packages
- `tiers` — declared L2/browser tier ownership (default is L1 only)
- `features` / `all-features` / `l1-include-slow` — the package test contracts
  currently encoded in area justfiles: `biscuit-hash --all-features`,
  `sniff --features remote`, Darkmatter's inclusion of `slow_`
- `runner-tools` — closed vocabulary: `ai-provider-stubs`,
  `darkmatter-md-fixture`, `node-22`, `pnpm-10`
- `companion-suites` — `homelab-frontend`, owned by `homelab-server`

The ten `ci: false` exclusions are **audited package by package, not copied
wholesale**: the time-bounded zero-or-few-tests exclusions (`visualizer`,
`reaper`, `agent-sandbox`, `tabby`) become gating packages; each remaining
`gates = false` record keeps `exclusion-class`, `owner`, `reason`, and the
expiry rules enforced today.

Metadata validation rejects unknown fields, invalid tier or tool names,
conflicting `features`/`all-features`, expired exclusions, an L2 tier without
backends, and companion-suite names with no registered canonical recipe.

`.github/ci/environments.json` is the one versioned, schema-validated
capability table: runner labels, native-package installer keys, and whether an
environment can host tmux, a headless browser, Node/pnpm, or archive-only
execution. It replaces the 8 `policy_gaps` records. Capability only — package
policy decides which tiers are expected, so an unsupported required tier is an
explicit `POLICY GAP`, never a silent absence.

`areas.json` still exists and is still authoritative — nothing is cut over yet.

**Verification.** A test loads both stores and diffs the resolved policy for
every package. Every intentional difference — exactly the audited exclusions —
is named in the phase notes; everything else must be identical. The test is
deleted in Phase 5.

**Why first:** additive, reversible, and provable against the thing it
replaces while that thing is still present.

## Phase 2 — Derive defaults, prove declarations

**Files:** `scripts/ci/affected_scope.py`, `scripts/ci/test_affected_scope.py`,
`tools/test-toolkit/tests/ci_workflow_contracts.rs`

R7 splits the old "derive everything" idea in two:

- **Derived from Cargo metadata:** package identity, manifest path, dependency
  closure, default L1 ownership, and check arguments (package name plus
  manifest-declared features).
- **Declared and proved, not derived:** L2/browser tier ownership. Tier
  markers are test names and module paths (`level2_*`, nextest filtersets),
  not necessarily files, and discovering them with nextest requires building —
  so the scope job cannot derive them. Contract tests instead use
  `cargo nextest list` with the canonical filter to prove each declared tier
  is non-vacuous, and to fail when a package owns such tests without declaring
  the tier (AC12).

Remove sharding entirely (R7a): drop the `shards` records, stop passing
`--partition`, and drop shard from every job name. Measured justification is in
`spec.md § Sharding` — compilation is ~85% of a shard and every shard pays it in
full, so four shards cost ~3.2× the compute to save ~2.4 minutes.

**Verification.** Derived values match today's declared values for all 72
packages, with every difference explained in the phase notes rather than
absorbed — a difference is either a derivation bug or a stale declaration, and
both need naming. Tier-contract tests fail for both an undeclared non-default
test and a declared-but-empty tier, each proven non-vacuous by neutering.

For sharding: no workflow passes `--partition`, and the `darkmatter` and
`claudine` suites still run every test they ran before — removing sharding must
drop the partitioning, never the tests. Compare executed-test counts against
the pre-change run, not just job outcomes.

## Phase 3 — Fan out per package

**Files:** `.github/workflows/ci.yml`, `.github/workflows/_area-ci.yml`
(renamed), `tools/test-toolkit/tests/ci_workflow_contracts.rs`

The matrix iterates gating impacted packages. The reusable workflow takes a
package: L1, L2, browser, and lint invoke `_test`, `_test_l2`, `_test_browser`,
and `_lint`; compile-check stays `cargo check --all-targets -p <package>`. The
workflow file is renamed away from `_area-`.

Package-specific behavior currently embedded in area recipes is preserved
explicitly (R12, AC13):

- `runner-tools` is implemented by the reusable workflow as a closed
  vocabulary, not an arbitrary command surface. `darkmatter-md-fixture`
  preserves Claudine's clean-checkout `md` binary setup (`_ensure-md`), which a
  direct `_test claudine` would otherwise lose. `ai-provider-stubs` keeps
  `claudine-cli`'s inert provider-discovery stubs.
- `homelab-frontend` invokes the existing non-focusing frontend recipe and
  attributes its producer status to `homelab-server`/L1. It must emit
  machine-readable evidence or a producer failure — a green Rust JUnit report
  must not hide a failed or skipped companion suite.

Native provisioning uses the **union of `native` requirements across the
selected package's target-relevant dependency closure** — a dependent job that
compiles `playa` needs ALSA even though it is not testing `playa`. Tiers,
runner tools, and companion suites do **not** propagate. For WSL2, native
requirements reach both the Linux archive builder and the guest runtime;
satisfying only the builder is insufficient when an archived binary
dynamically links a library the guest lacks.

Calculate and record the exact expanded full-scope job count and assert it
stays below GitHub Actions matrix/reusable-workflow limits — up to 72 packages
across default environments plus declared tier jobs can exceed the 256-job
matrix ceiling if left unchecked.

**Verification.** Contract tests: matrix is package-derived; every
result-producing job names exactly one package; manifest-declared requirements
(including closure-collected native ones) reach their jobs; companion suites
execute. Contract suite otherwise unchanged — confirm the three known
pre-existing WSL failures are *the same three* by diffing against a clean
worktree at `origin/main`, not by reading the count.

## Phase 3a — Re-key the build cache

**Files:** `.github/workflows/_area-ci.yml` (renamed)

`Swatinem/rust-cache` is keyed per directory —
`area-ci-${area}-test-${environment}`. With the package as the unit that key is
wrong, and what replaces it is the single biggest influence on whether this work
reduces runtime at all: compilation is ~85% of a test job.

Start from a package-scoped key. Do not settle it by argument — Phase 6
measures it, and the implementation is not complete until one key strategy is
selected from measured before/after data and recorded in the measurement
artifact.

**Verification.** Cache restore result, cache size per package/environment
key, and total job wall-clock, compared against the pre-change run for the
same commit content. Record all three in the phase notes.

## Phase 4 — Re-key results and the known-failure list

**Files:** `scripts/ci-rollup.rs`, `scripts/ci-rollup-tests.rs`,
`.github/ci/ci-baseline.toml`

`CellKey` becomes `{package, environment, tier}` through the rollup, verdict,
baseline, skip-budget, policy-gap, and comparison paths. `RunRecord.area`,
producer-status `area`, scope `area_names`, artifact fallback parsing, and
shard identity are removed. The result schema version is bumped, and readers
reject the old version with an explicit migration error.

Result states (R10): a scheduled package whose invocation selects zero tests
records `NOTHING TO RUN`; a `gates = false` package records `NOT SCHEDULED`
with its governance metadata. Neither is a pass, neither is a blocking missing
result, and the two are never conflated.

**The 32 baseline entries are not a one-to-one mechanical rename:**

- `claudine-gen-drift` and `coverage` are synthetic identities, not Cargo
  packages — they stay outside the package baseline until their specialized
  producers join the result contract, without being renamed or dropped.
- The thirteen shard-keyed entries (eight Darkmatter, five Claudine) collapse
  into shard-free package entries.
- An area cell may split into failures from more than one package, and lint
  has producer status but no JUnit package evidence.

JUnit-backed failures seed candidate package entries because the invocation
manifest records a package, but every candidate is verified against a
package-keyed run before landing.

**Sequencing — the bridging run.** Re-key from the *same* full-scope run that
first produces package-keyed results, executed on the branch before the
cutover merge. Generate a candidate baseline from it, review the one-to-many
area splits and collapsed shard entries by hand, then land the baseline with
the workflow and schema cutover. Earlier is guessing; later leaves entries
unmatched, and an unmatched scheduled entry blocks.

**Verification.** The verdict on the bridging run reports zero
`baseline-no-result` and zero `baseline-now-passing` for scheduled cells. The
set of excused failing tests is identical before and after for every
JUnit-backed entry — same tests, package keys, shard identity gone — and the
synthetic entries are carried forward outside the package baseline rather than
silently dropped.

## Phase 5 — Delete `areas.json`

**Files:** `.github/ci/areas.json`, `.github/ci/README.md`,
`scripts/ci/affected_scope.py`, `tools/test-toolkit/tests/ci_workflow_contracts.rs`

Remove the file, its loader, its schema validation, and the Phase 1 equivalence
test. Rewrite the README around the package as the unit. Lands in the same
merge as Phases 3–4.

**Verification.** A contract test asserts no workflow, recipe, or script reads
`areas.json`. `just check-canonical` still passes — it validates that
directories expose canonical recipes for developers, which is unaffected by how
CI selects work.

## Phase 6 — Prove it on a real run

**Files:** none

Measurement is a phase, not a footnote. The full-scope dry run happened as the
Phase 4 bridging run (AC14: matrix within platform limits, every scheduled
cell emits evidence, baseline valid). This phase measures the narrow case the
change exists for. On a branch touching exactly one package of a
multi-package directory, record before and after:

1. number of test jobs scheduled;
2. packages actually tested;
3. wall-clock from run start to verdict;
4. total billed runner minutes;
5. that a dependent package elsewhere still runs (R2 — the `biscuit-speaks`
   closure is the named probe);
6. **build-cache restore result and cache size** per package/environment key,
   against the pre-change run.

Item 6 is not optional. Compilation is ~85% of a test job, so a per-package
matrix sitting on a worse cache key can schedule fewer jobs and still take
longer. Fewer jobs is not the objective; less time is. The cache-key decision
from Phase 3a is closed here, from this data, and recorded in the measurement
artifact.

**Exit condition:** a reorganized matrix that does not reduce observed runtime
has not met the objective. Argument-list inspection does not close this.

## Risks

**Losing reverse-dependency expansion.** The most damaging misreading
available: narrowing the dependency closure instead of the directory fan-out
would silently stop testing consumers of a changed package. R2 names the exact
`biscuit-speaks` closure; Phase 6's fifth check re-proves it.

**Dropping package-specific behavior in the conversion.** The Homelab
frontend, Claudine `md` fixture, provider stubs, Cargo feature selections, and
Darkmatter slow-test policy are all reached today through area recipes that
stop being invoked. R12, the Phase 1 extraction, and AC13 exist for this — a
conversion that passes every Rust JUnit report can still be a regression.

**Feature-set drift.** A package that compile-checks one feature set and tests
another defeats both. `features`/`all-features` are forwarded consistently to
check, archive construction, and the canonical recipe; Phase 3's contract
tests cover the forwarding.

**The cutover merge is large.** Phases 3–5 plus the baseline land together by
design — the alternative is a merged state where the merge gate is unusable.
Mitigation is rehearsal, not decomposition: the bridging run exercises the
entire package-keyed path on the branch before anything merges.

**Dropping tests while dropping shards.** Removing `--partition` must remove
the partitioning and nothing else — four shards each running a quarter looks
very like one job running a quarter. Phase 2 compares executed-test counts
against the pre-change run for exactly this reason.

**Silent zero-test jobs.** A package whose job runs nothing and exits 0 would
report as a pass. `NOTHING TO RUN` vs `NOT SCHEDULED` (R10) and Phase 4's
handling are the guard — the same class of inversion as an empty `-p` list
checking the whole workspace.

**Undeclared tiers.** With derivation gone, a package that gains L2 tests
without declaring the tier would silently skip them. The Phase 2 tier-contract
tests (`cargo nextest list` against declarations, AC12) are the guard, and
they must be proven non-vacuous like every other contract test.

**Full-scope runs get larger.** Up to the gating subset of 72 packages ×
environments plus declared tier jobs. Phase 3 computes the exact count against
GitHub's matrix limits; the bridging run proves it in practice.

**A worse build cache eating the gain.** The largest risk to the objective,
and the least visible. More jobs against a colder or more fragmented cache can
take *longer* in total while looking like progress on every other metric.
Phase 3a starts the key, Phase 6 measures it, and neither closes on job count
alone.

## Non-goals

- Wiring the specialized workflows' extra scenarios (`messenger`, `playa`,
  `biscuit-tui`, `rendezvous`) into the merge verdict. Coverage problem, not
  scope resolution — but standard package L1 ownership is decided by this spec
  independently, and newly gating Rendezvous packages are not "covered" merely
  because the specialized workflow exists.
- Wiring up the expected-test manifest producer. Referenced by no workflow
  today; any future producer generates on the target environment, per package
  and tier.
- Reducing the known failures. They are re-keyed, not fixed.
- Changing what developers run locally (R8).

## Decisions from review — settled

1. **Build-cache key:** chosen from measured before/after data in Phase 6, and
   recorded in the measurement artifact; not chosen by argument.
2. **`gates = false`:** governance metadata is preserved for exclusions that
   survive the package-level audit. Non-gating is never inferred from zero
   observed tests — that would silently exempt a package and miss its first
   test. The time-bounded zero-test exclusions become gating packages.
3. **Migration order:** dual-read code may exist during development, but the
   repository cutover is atomic — one merge deletes `areas.json`, moves all
   consumers, bumps the result schema, and installs the verified baseline.

Sharding is settled: removed (`spec.md § Sharding`), with build-once/run-many
recorded there as a future option rather than a follow-up commitment.
