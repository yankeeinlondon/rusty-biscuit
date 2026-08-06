---
title: Per-package test resolution — implementation notes
status: implemented (Phases 1–5); Phases 4 bridging run and 6 measurement pending CI
created: 2026-08-06
spec: fixes/2026-08-06-cicd/spec.md
plan: fixes/2026-08-06-cicd/plan.md
---

# Implementation notes

This records what the implementation did, the decisions that were not obvious
from the plan, and the two phases that remain gated on a real CI run. Read it
alongside `plan.md`; it does not repeat the plan.

## Phase 1 — package policy into manifests; one environment table

- `[package.metadata.ci]` blocks added to 28 manifests (gating packages with
  policy to declare, plus the 9 excluded packages). A package with no block
  defaults to `gates = true` and L1.
- `.github/ci/environments.json` created: the four environments, with the two
  governed unavailabilities (Windows tmux, WSL2 archive-only) declared once,
  carrying owner/reason/expiry. A plain `false` capability is an UNGOVERNED
  absence that the rollup never excuses — same shape as the old "undeclared
  gap".
- `scripts/ci/affected_scope.py` rewritten to read package policy from `cargo
  metadata` and validate it (unknown fields, invalid tier/tool/backend names,
  features/all-features conflict, expired exclusions, L2-without-backends,
  companion suites whose recipe does not exist).

### Equivalence with `areas.json` (proven, then retired)

A temporary diff (`/tmp/ci_equivalence_check.py`, not committed) loaded both
stores and compared resolved policy for every package. Result:

> equivalence OK: package manifests cover every areas.json fact

Intentional, named differences:

1. **visualizer, reaper, reaper-cli, agent-sandbox-cli, tabby, ui are now
   gating** (AC15). The time-bounded zero-or-few-tests exclusions became
   gating packages that record `NOTHING TO RUN`. `visualizer` keeps its
   `native` declaration (GTK/WebKit/D-Bus).
2. **`ai-provider-stubs` and `darkmatter-md-fixture` narrowed to
   `claudine-cli`** — verified by source inspection as the only package whose
   tests consume them (the `md` fixture via
   `claudine/cli/tests/inline_compose_hash.rs`; provider discovery in
   claudine-cli tests that seed their own fakes). claudine lib tests carry
   their own fixtures and never do PATH provider discovery.
3. **The `node` capability narrowed to `homelab-server`** (spec-mandated owner
   of `homelab-frontend`).
4. **`native` unions over the dependency closure** (R5, wider than before):
   e.g. `biscuit-speaks` now receives `playa`'s ALSA/PulseAudio via its
   `playa` feature edge, plus its own `espeak-ng`.
5. **Sharding removed** (R7a).
6. **L2/browser tier ownership** assigned per the area recipes' actual
   invocations: e.g. sniff's L2 is `sniff-cli` only; biscuit-terminal's
   browser is the library only.

## Phase 2 — derive defaults, prove declarations

- Package identity, manifest path, dependency closure, default L1 ownership,
  and check arguments are derived from Cargo metadata. Tier ownership is
  declared and proved, not derived (nextest tier membership needs a build).
- Sharding removed entirely: `--partition` is passed nowhere, `BISCUIT_CI_SHARD`
  is stamped nowhere, and the manifest record dropped the `shard` field.
- **Tier-contract tests** (`cargo nextest list` against the canonical filter,
  AC12) added in `ci_workflow_contracts.rs`. They proved non-vacuous by
  neutering.

### Finding surfaced by AC12: `dmls` owns L2 tests

`an_undeclared_l2_tier_is_rejected` found that **`dmls` owns three real L2
tests** (`level2_neovim_*` — neovim semantic-token rendering under tmux) that
the old darkmatter area recipe never enumerated (it ran L2 only for the
library and CLI). The conversion surfaced a previously-invisible suite.

Resolution: `dmls` now declares `tiers = ["L1", "L2"]` with
`l2-backends = ["tmux", "wezterm"]`. These tests will run for the first time
on the bridging run; if they fail, they join the baseline like any other
known-red.

The tier-contract helper also had to pass each package's **declared features**
to `cargo nextest list` — sniff-cli's L2 suite lives behind the
`test-fixtures` feature and is invisible to a featureless listing.

## Phase 3 — fan out per package

- `.github/workflows/_area-ci.yml` renamed to `_package-ci.yml`; the matrix
  iterates gating impacted packages and every result-producing job names
  exactly one package.
- `runner-tools` is a closed vocabulary implemented by the reusable workflow:
  `ai-provider-stubs`, `darkmatter-md-fixture`, `node-22`, `pnpm-10`, and
  `l2-parallel-self-spawn` (the last preserves claudine-cli's measured
  `min(cores, 8)` parallel L2 mode that the area recipe used).
- `companion-suites`: `homelab-frontend` invokes `homelab/justfile::test-
  frontend` (and `lint-frontend` in the lint job), attributes its producer
  status to `homelab-server`/L1, and **downgrades a green Rust JUnit report on
  failure** (R12's guard — a new rollup rule).
- Native provisioning uses the **union of `native` over the selected package's
  dependency closure**, computed by the scope job and passed as an explicit
  list to every building job (including the WSL archive builder and guest).
- Job count: full scope is **63 gating packages × ~6 jobs ≈ 400 jobs**; the
  package matrix itself is 63 entries, well under GitHub's 256-job ceiling. The
  scope summary records the expanded estimate.

## Phase 3a — re-key the build cache

Cache keys are package-scoped (`package-ci-<package>-<tier>-<environment>`).
This is the **starting** strategy; Phase 6 measures it against a real run and
records the selected strategy. Not closed by argument.

## Phase 4 — re-key results and the baseline

- `CellKey` is `{package, environment, tier}` through the rollup, verdict,
  baseline, skip-budget, policy-gap, and comparison paths. `RunRecord.area`,
  producer-status `area`, shard identity, and **artifact-name fallback parsing
  are removed** — the manifest is now the only identity source.
- Schema version bumped to **2**; readers reject version 1 with an explicit
  migration error naming the area→package re-key.
- Result states (R10): `NOTHING TO RUN` (scheduled, zero tests selected) and
  `NOT SCHEDULED` (governed `gates = false`) are distinct and never conflated.
  `N/A` is retired.
- `NOTHING TO RUN` does not block; `NOT SCHEDULED` does not block; both are
  visible in the grid.

### The baseline is CANDIDATE, pending the bridging run

`ci-baseline.toml` was re-keyed by attributing each entry to the package the
**recorded example failing test** names (every JUnit-backed failing test
carries a `<package>::<module>::test` identity), with the shard dimension
collapsed. The 13 shard-keyed entries (8 darkmatter, 5 claudine) collapsed to
package entries; the darkmatter WSL2 shard 2/4 split into a separate `darkmatter`
(library) entry because its example names `darkmatter::`.

**Every entry below is a candidate pending the Phase 4 bridging run** — the
first full-scope run that produces package-keyed results. `ci-rollup verdict`
will report `baseline-no-result` for a candidate whose package did not fail on
that environment, and `baseline-now-passing` for one whose failure no longer
reproduces. Both block, which is the intended way to retire a wrong guess.
Correct each against the bridging run's rollup **before the cutover merge**;
an unmatched scheduled entry blocks.

The unverified area-keyed entries (source run 30323254931, no example tests)
were attributed by the owning area's known package shape and marked so in each
entry's `reason`. The two synthetic identities (`claudine-gen-drift`,
`coverage`) are carried forward outside the package baseline — never in any
package scope, reported `baseline-out-of-scope`, neither blocking nor passing.

## Phase 5 — delete `areas.json`

- `.github/ci/areas.json` deleted. The loader, schema validation, and the
  Phase 1 equivalence check are gone. `areas_json_is_gone_and_has_no_readers`
  asserts no workflow, recipe, or script reads it.
- `.github/ci/README.md` rewritten around the package as the unit.
- `just check-canonical` still passes — it validates that directories expose
  canonical recipes for developers, unaffected by how CI selects work.

## Remaining (gated on CI)

### Phase 4 bridging run

Run a full-scope package-keyed run on this branch before the cutover merge.
Generate a candidate baseline from it, review the one-to-many area splits and
collapsed shard entries by hand, then land the verified baseline with the
workflow and schema cutover. Until then, the checked-in baseline is candidate.

### Phase 6 — measurement

On a branch touching exactly one package of a multi-package directory, record
before/after: jobs scheduled, packages tested, wall-clock, billed minutes, and
**build-cache restore result and size per package/environment key**. The
cache-key decision (Phase 3a) closes here, from this data.

## Verification run locally

- `python3 scripts/ci/test_affected_scope.py` — **35 pass**.
- `cargo nextest run --no-default-features --bin ci-rollup` (scripts) — **114
  pass**.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts` — **53
  pass** (including the slow tier-contract builds).
- `just check-canonical` — passes.
- End-to-end rollup+verdict smoke test (synthetic artifacts) — package-keyed
  grid, MISSING detection, correct verdict.

### Pre-existing, unrelated failures observed

`s/test junit_staging_contracts` has 6 failing tests on **clean `main`**
(verified via a `git worktree` of `HEAD`): the fixture runs the `test` recipe,
which depends on `_storage_preflight`, which shells out to
`scripts/storage-preflight.sh` via `git rev-parse --show-toplevel` — but the
temp-dir fixture is not a git repository. This predates and is orthogonal to
this change; not fixed here (surgical scope). The manifest-schema assertions
those tests make (when they reach them) were updated for the new 7-key record.
