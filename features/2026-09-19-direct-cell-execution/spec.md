---
kind: feature
name: direct-cell-execution
date: 2026-09-19
status: draft
supersedes:
  - 2026-09-12-better-cicd-flow
related:
  - 2026-09-11-cicd-cleanup
  - 2026-09-19-hosted-evidence-reuse
  - 2026-09-19-nightly-scope
---

# Direct cell execution: hosted matrices built from the plan's cells

## Why

The planner already resolves one `{package, environment, gate}` cell for
every unit of required coverage and decides, per cell, whether hosted CI
executes it, reuses evidence for it, or omits it as an accepted gap. Hosted
CI does not consume those cells. `legacy_scope_document` re-projects them
into a per-package record carrying separate environment lists —
`native_environments`, `check_os`, `l2_environments`,
`browser_environments`, `node_environments`, `companion_environments`,
`toolchain_environments`, `producing_environments`, `wsl` — and
`_area-ci.yml` forwards each list to `_package-ci.yml` as its own input,
where six jobs gate on them. Every list is a place where the plan and the
matrix can disagree:

- PR #76 turned seven macOS cells `MISSING`: excluding an environment edited
  the matrix while the rollup derived expected cells from the policy, and
  nothing reconciled the two.
- Every capability since has cost a list, a workflow input, and a contract
  test pinning the pair: node environments on 2026-09-16, toolchain
  environments on 2026-09-18, producing environments on 2026-09-19. Each
  contract exists only because the projection can drop a side.
- An area whose cells are all reused or accepted gaps still fans out a
  runner so the audit has something to render ("an all-reused area must still
  fan out", cicd-cleanup phase 5). PR #83's final run showed 104 skipped
  checks beside 127 passes.
- The coverage audit owns the verdict on expected tests, skips, and missing
  evidence after execution, so a defect in the audit tool reds every area at
  once: the environments schema version on run 35405580517 (14 areas) and
  the fractional upload window on run 35412170320 (18 areas) were both
  audit-tool failures presented as coverage failures.

2026-09-12-better-cicd-flow described this fix. Its decisions 1, 4, 5, and 6
have since shipped through other work (the plan's execution states and
cumulative receipts, planner-decided `neutral` gaps, the policy-free
`ci-gate`, advisory reporting). Its decisions 2 and 3 — matrices from
executing cells, and producer-side completeness — did not, because
cicd-cleanup's phase 6 stopped on rulings that have since been made and on
a presentation fixture that live runs have since proven. This spec
supersedes that draft with those two decisions, restated against the
workflows as they are today.

## What exists today

| surface | lines | role |
|---|---|---|
| `ci.yml` | 1,070 | validation, scope, preflight, producers, area fan-out, gate, reporting |
| `_area-ci.yml` | 329 | one call per area; `package-ci` matrix over the area's package records, `accepted-gaps`, `coverage-audit` |
| `_package-ci.yml` | 1,710 | 21 inputs; `check`, `test`, `lint`, `test-l2`, `test-browser`, `wsl2`, each gated on its own environment list |
| `_wsl-ci.yml` | 816 | the archive-only guest, called from `_package-ci.yml` |

The chain is four reusable-workflow levels deep, which is GitHub's limit.
The planner's `cells[]` already carry everything a producer needs except the
feature arguments, which sit on the package record. The rollup already keys
every stored result on `{package, environment, tier}`.

## Decisions

1. **The matrix is the executing cells.** The scope job publishes, per area,
   a matrix whose rows are exactly `cells[] | select(.execution ==
   "execute")` for that area, each row carrying the cell identity, its gate,
   the package's feature and test arguments, the build record it consumes,
   its declared L2 backends, runner tools, companion suites, and native
   packages, and the exact-skip entries attached to it. No environment list
   survives; `legacy_scope_document`'s matrix projection is deleted once
   nothing reads it, and the planner's Python contracts prove
   `planned execute cells == matrix rows` per area as one set equality.
2. **One producer job per cell kind, dimensioned by the row.** `_package-ci`
   becomes a cell-level workflow: a `test` job whose matrix is the area's
   executing L1, L2, and browser rows (tier as a row field, not a job), a
   `check` job over check rows, a `lint` job over lint rows, and the WSL2
   guest over its rows. The three near-identical consumer jobs collapse into
   one whose steps branch on the row's tier only where the tiers genuinely
   differ (the L2 backend proof, the browser serialization).
3. **Areas stay as the chunking unit and nothing else.** GitHub caps a matrix
   at 256 entries and a full-workspace plan exceeds that, so `_area-ci` keeps
   fanning out one call per area. An area with no executing cell schedules no
   `package-ci` call at all: its reused and neutral cells are reported from
   the plan by the audit alone. The chain stays at four levels; the change
   flattens the package layer's inputs, it adds no workflow.
4. **Producers validate their own completeness.** Before a test producer may
   succeed it lists the expected tests on the target (`cargo nextest list
   --archive-file`, which the consumer can run without a toolchain), runs the
   complete planned gate with no unplanned filter, retains its JUnit and
   status under the cell identity, compares expected identities with observed
   pass, fail, and skip, and applies exactly the skip entries the row carries.
   A missing expected test, an unexpected skip, a failed companion, or a
   missing report fails the producer. Lint and check producers validate their
   own status document.
5. **The audit shrinks to what only a post-execution view can see.** With
   producers truthful, `coverage-audit` keeps three questions: is any planned
   executing cell missing a result (a lost runner, a dropped artifact); is
   every omitted cell a governed, unexpired gap; and did a reused cell's
   evidence match the plan. It stops re-deriving expected tests and skips. A
   producer failure reaches `ci-gate` through the producer and creates no
   second red check, as today.
6. **`ci-gate` is unchanged.** It folds blocking job results and reads no
   plan.

## What it buys

- The PR #76 class of defect becomes impossible rather than tested for: the
  matrix is the plan's cell list, not a projection of it.
- Adding a capability costs a row field, not a list, an input, and a contract.
- An audit-tool defect can no longer red every area; the audit decides only
  missing evidence and gap policy.
- Areas with nothing to execute cost no runner and no skipped checks.
- `_package-ci.yml` loses its three-way duplication; the inputs drop from 21
  to the row and the run-wide values.
- 2026-09-19-hosted-evidence-reuse gets simpler: the thing reused across
  trees is the thing scheduled.

## Migration

1. **Cell rows.** The planner emits `area_rows` (or the existing
   `area_matrix` reshaped) from executing cells, with a Python contract that
   the union of rows equals the executing cells and that every row's fields
   come from the plan. Keep `legacy_scope_document` beside it until step 3.
2. **The producer job.** Rewrite `_package-ci.yml`'s `test`, `test-l2`, and
   `test-browser` into one row-driven job, and `check` and `lint` into
   row-driven jobs, keeping every step whose contract test exists today
   (build resolution and verification, sidecar provisioning, runner tools,
   node provisioning, toolchain provisioning, the compiler-work counter).
   Move each existing `ci_workflow_contracts` assertion onto the new shape
   in the same change; a contract that pinned a list is replaced by one that
   pins the row field.
3. **The fan-out.** `_area-ci.yml` passes rows, not lists; `ci.yml` skips the
   area call when its row set is empty. Delete `legacy_scope_document`'s
   matrix half and the environment lists; the policy half stays until the
   rollup reads policy from the plan.
4. **Producer completeness.** Add the expected-test listing and comparison
   to the test producer; move exact-skip validation there; reduce the audit.
   The rollup's baseline handling is unchanged, since exact skips remain the
   only pardonable state.
5. **Prove it on one area first.** Land steps 1 to 3 behind the existing
   `_package-ci` for every area but one (the root area: `repo-deps` and the
   tools packages, whose suites are the pipeline's own), compare its results
   with the projection's for three runs, then switch the rest.

## Not done

- Replacing `ci-rollup`'s policy document with the plan (cicd-cleanup's
  remaining rollup work). The audit still reads `scope.json`'s policy list.
- Hosted-evidence reuse across trees: 2026-09-19-hosted-evidence-reuse, which
  this feature should land before.
- Changing which environments an event schedules; the cadence policy stands.

## Open questions

1. **Row granularity.** One row per cell, or one row per `{package,
   environment}` carrying the cell's tiers, so a package's L1 and L2 on one
   environment share a runner and an archive download? Sharing halves the
   downloads for L2-owning packages but re-couples tiers that today fail
   independently. The compile-once specification wanted tiers staged behind
   L1; a shared row keeps that as step order rather than job order.
2. **Expected-test manifest for `cfg`-excluded tests.** `nextest list` on
   the target names what compiled for it, so a test compiled out by `cfg` is
   absent rather than skipped. Is absence acceptable as long as the source
   marker count (`declared_test_tiers_match_owned_tests`) still holds, or
   should the producer receive the producer-side list to diff against?
3. **Where reused and neutral cells render** once their area fans out no
   runner: the area audit only, or also a run-level summary in
   `ci-reporting`?
4. **Skip policy in the row or read by the producer** from the plan
   artifact. The row is self-contained and testable; the plan is one fewer
   copy of the entries.

## Acceptance criteria

1. For every area in a resolved plan, the published matrix rows are exactly
   the area's cells with `execution == "execute"`, proven by a planner
   contract and by a workflow contract over the shipped fan-out.
2. No environment list remains in `_area-ci.yml` or `_package-ci.yml`
   inputs; every producer condition reads a row field.
3. An area with no executing cell schedules no `package-ci` call, and its
   reused and accepted-gap cells still appear in the area audit and the run
   summary.
4. A test producer fails on a missing expected test, an unexpected skip, a
   failed companion, or a missing report, with the reason in the job's own
   log; the audit no longer evaluates those.
5. A full-workspace `workflow_dispatch` run and a nightly both stay under
   GitHub's matrix and nesting limits, measured on a real run.
6. `ci-gate` is byte-identical to today.
7. The Python suites, `ci_workflow_contracts`, the rollup suite, the hook
   suite, and actionlint pass, and the root area's results match the
   projection's across three consecutive runs before the switch.
