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
- Job count: full scope is **63 gating packages × ~6–7 jobs ≈ 460 jobs** (the
  WSL leg spawns two jobs per package — archive plus guest); the package
  matrix itself is 63 entries, well under GitHub's 256-job ceiling. The scope
  summary records the expanded estimate.

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
cache-key decision (Phase 3a) closes here, from this data. Phase 6 must ALSO
measure full-scope hit rates before closing the key decision: ~5 keys × 63
packages ≈ 315 cache entries press GitHub's 10 GB per-repo quota, and eviction
thrash on a full-scope run is the failure mode that count predicts. (The WSL
surface — every gating package provisioning a WSL leg, two jobs each — is
part of the same billed-minutes measurement.)

## Review 1 — findings and resolutions

`review-1.md` delivered 5 blockers, 7 majors, 16 minors, and a list of
test-coverage gaps. All were implemented in this branch; the resolutions are
below. Four items the review marked "needs Ken's ruling" were decided as
follows and are flagged for ratification:

1. **M2 (browser tier)** — decided per the ratified spec's letter: the browser
   absences in `environments.json` are now GOVERNED (owner/reason/expiry) and
   the rollup renders explicit POLICY GAP cells for them, exactly like the
   tmux gaps, instead of the tier disappearing from the grid.
2. **M5 (required backends)** — wired now: the per-package L2 leg sets
   `BISCUIT_TEST_REQUIRED_BACKENDS` to the package's declared backends
   intersected with the provisioned set (tmux), so `_test_l2`'s backend-proof
   bracket is live in CI. The skill and test-toolkit docs claiming "CI passes
   it through unmodified" were corrected.
3. **`l2-parallel-self-spawn`** — ratified into the spec's `runner-tools`
   vocabulary (spec.md § Where the survivors live).
4. **WSL surface** — confirmed: `wsl: true` for every gating package is
   consistent with R1/R6; the cost (2 jobs × ~63 packages) is explicitly part
   of Phase 6's billed-minutes measurement.

### Blockers

- **B1** — `claudine-gen` now declares `tiers = ["L1", "L2"]` with
  `l2-backends = ["tmux"]` (it owns real L2 tests in
  `tests/level2_report_terminal.rs`), and the claudine area's local `test-l2`
  runs `_test_l2 claudine-gen` after `claudine-cli` (the dmls precedent), so
  the declared tier runs locally too — verified: 3 tests, all passing.
- **B2** — `package_policies()` in the contract suite now enumerates EVERY
  workspace member with an empty-object default policy (the default-policy
  packages are what the guard exists for); `an_undeclared_browser_tier_is_rejected`
  added; `nextest_test_count` fails loudly on spawn/listing failure instead of
  returning a vacuous 0.
- **B3** — the WSL guest is provisioned with `jq` (`additional-packages`),
  which its native-prerequisites step requires before anything else installs.
- **B4** — `l1-include-slow` is forwarded `_package-ci.yml` → `_wsl-ci.yml`
  and exported in the guest L1 step, so darkmatter's L1 contract no longer
  differs by environment. Contract test pins both directions.
- **B5** — `has_packages` is computed from `.matrix | length > 0` (the
  impacted list includes non-gating packages; the old derivation crashed a
  `gates = false`-only change at strategy evaluation). Contract test added.

### Majors

- **M1** — `companion_suites` now travels in the rollup-facing policy; the
  producer status records the companion step's outcome on EVERY run of a
  declaring package (not only failures); the rollup downgrades a green
  L1/lint cell whose declared companion produced no success evidence
  (skipped or unreported), test-proven in both places.
- **M2** — see ruling 1 above.
- **M3** — resolved by merge strategy: the cutover lands as a SQUASH MERGE,
  so the broken intermediate tree (`c52eea5b3` deleting `_area-ci.yml` while
  `ci.yml` still referenced it) never reaches `main`. Recorded here rather
  than fixed by rebase so the reviewed commit `e844fc2e9` stays addressable
  for the rest of the review cycle.
- **M4** — `affected_scope.py` maps `scripts/` and `.github/ci/` to a
  `ci_tooling` flag; `ci.yml` runs the scope tests and the rollup's nextest
  suite on a dedicated `ci-tooling` leg, classified in the advisory summary.
  The durable fix for the R11 contract suite remains test-toolkit's promotion
  (`gates = false`, expiry 2026-10-31) — recorded in `.github/ci/README.md`.
- **M5** — see ruling 2 above.
- **M6** — the AC4 no-reader guard now globs all workflows, `just/*.just`,
  `scripts/**`, the root justfile, and every area justfile (with a
  non-vacuity assertion), instead of a hardcoded 7-file list.
- **M7** — `rust-testing/SKILL.md` (sharding removed, per-package gates,
  required-backends wiring; hash + `last_updated` regenerated via `md hash`)
  and `docs/testing-strategy.md` (per-package fan-out, full L1 on macOS, the
  Windows `--all-targets` check, POLICY GAP rows, sharding removal,
  `BISCUIT_TEST_LEVEL_REQUIRED` never set) were brought in line with the
  cutover.

### Minors

1. Baseline `schema_version` is now a required field (a version-less file
   dies as a parse error, never a silent self-upgrade). Test added.
2. `FailureEntry` is `deny_unknown_fields` — a stale `shard`/`area` key is
   rejected. Test added.
3. A `gates = false` package without parseable exclusion metadata now renders
   NOT SCHEDULED with an "ungoverned" backstop record instead of vanishing.
   Test added.
4. `estimate_jobs` counts the WSL leg as two jobs; the docstring no longer
   claims exactness. Test added; Phase 3 note above corrected.
5. Phase 6 must measure full-scope cache hit rates against the 10 GB quota
   (see the Phase 6 section above).
6. `messenger-desktop` selection is prefix-matched in the scope step
   (`messenger` flag), so a messenger-cli-only change selects it.
7. The native-union/feature coupling is now documented on `build_closure`
   and pinned by a scope test asserting the biscuit-speaks → playa edge
   against the real metadata.
8. `biscuit-visualized` records `all-features = true` in its manifest, so
   promotion cannot silently drop its 56 feature-gated tests.
9. `biscuit-clipboard`'s three exclusion records carry package-specific
   reasons (counts and per-package blocking rationale).
10. The companion-recipe existence check matches a recipe DEFINITION
    (`^recipe(:|\s)`), not a substring.
11. `l2_environments` and the rollup's L2 expected-cells now derive from
    every declared backend (each backend is a capability under its own name);
    `tmux` is no longer hardcoded.
12. The scope.json `native` arrays are sorted (byte-stable output).
13. Baseline header drift fixed ("claudine library" → "claudine CLI").
14. Stale area/shard prose fixed in `ci-rollup.rs` (status_cells doc,
    degraded-scope note, `validate_date` parameter), `just/devops.just`
    (`_expected_manifest` comments), `darkmatter/dmls/Cargo.toml`, and
    `memory/just.md` (causal sentence restored).
15. `.github/ci/README.md` documents the real cache-key scheme (check/lint
    keys; L2/browser/WSL-archive reuse the test key) and names the `wsl`
    producer correctly.
16. The md-fixture probe also checks `target/debug/md.exe` (Windows).

### Test-coverage gaps closed

- **Scope:** gates=false path (excluded-from-matrix + present-in-policy);
  non-propagation of tiers/tools/companions; `build_closure` dev-dep edge
  rules; the `Cargo.lock` branches through `calculate_scope` (decidable,
  undecidable, irrelevant); the top-level-directory fallback; MATRIX_LIMIT
  overflow; `estimate_jobs`; the `ci_tooling` flag; the companion-recipe
  definition check; the L2 backend axis.
- **Rollup:** `status_cells` result mapping (+ test-tier/out-of-scope
  exclusion); a `tier = "lint"` baseline entry excusing a lint FAIL;
  `claudine-gen-drift`/`coverage` reported `baseline-out-of-scope` by name;
  `NotScheduled` added to the blocking-states rstest; stray-key rejection;
  version-less baseline refusal; companion evidence (success/skipped/absent,
  L1 and lint); the gates=false backstop.
- **Contracts:** everything under B2; `has_packages` derives from the matrix
  (B5); the companion-downgrade check no longer matches comment lines.

## Verification run locally

- `python3 scripts/ci/test_affected_scope.py` — **53 pass**.
- `cargo nextest run --no-default-features --bin ci-rollup` (scripts) — **133
  pass**.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts` — **57
  pass** (including the slow tier-contract sweeps, which now cover all 63
  gating packages under a 20-minute nextest override in `.config/nextest.toml`).
- `just check-canonical` — passes (27/27).
- `actionlint` on ci.yml, _package-ci.yml, _wsl-ci.yml — clean (pre-existing
  SC2086 infos only).
- End-to-end rollup+verdict smoke test (synthetic artifacts) — package-keyed
  grid, MISSING detection, governed browser POLICY GAP acceptance, and the
  skipped-companion downgrade, all rendering as designed.
- Live scope checks: a `gates = false`-only change now yields an empty matrix
  (`has_packages=false`, B5); `scripts/ci-rollup.rs` sets `ci_tooling` (M4).

### Pre-existing, unrelated failures observed

`s/test junit_staging_contracts` has 6 failing tests on **clean `main`**
(verified via a `git worktree` of `HEAD`): the fixture runs the `test` recipe,
which depends on `_storage_preflight`, which shells out to
`scripts/storage-preflight.sh` via `git rev-parse --show-toplevel` — but the
temp-dir fixture is not a git repository. This predates and is orthogonal to
this change; not fixed here (surgical scope). The manifest-schema assertions
those tests make (when they reach them) were updated for the new 7-key record.
