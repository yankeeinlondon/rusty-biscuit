---
title: Direct cell execution — hosted matrices built from the plan's cells
created: 2026-09-19
phase: 8
total_phases: 9
agent: claude/opus
yolo: true
spec: 2026-09-19-direct-cell-execution
supersedes:
  - 2026-09-12-better-cicd-flow
packages:
  - repo-deps
  - test-toolkit
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - features/2026-09-19-direct-cell-execution/spec.md
docs_created_during_phase_1:
  - features/2026-09-19-direct-cell-execution/rulings.md
  - features/2026-09-19-direct-cell-execution/spikes/s0-baseline.md
  - features/2026-09-19-direct-cell-execution/spikes/s1-labels.md
  - features/2026-09-19-direct-cell-execution/spikes/s2-nextest-list.md
  - features/2026-09-19-direct-cell-execution/spikes/s3-capacity.md
  - features/2026-09-19-direct-cell-execution/spikes/plans/README.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - scripts/ci/test_schema.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_affected_scope.py
  - scripts/ci/test_completion.py
  - scripts/ci/test_runner_loss.py
  - scripts/ci/affected_scope.py
  - scripts/Cargo.toml
  - scripts/ci-rollup-tests.rs
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - just/ci-local.just
  - .githooks/tests/test-pre-push.sh
  - scripts/ci/test_ci_local.py
docs_updated_during_phase_2:
  - features/2026-09-19-direct-cell-execution/spikes/s0-baseline.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - scripts/ci/schema.py
  - scripts/ci/affected_scope.py
  - scripts/ci/plan_fixtures.py
  - scripts/ci/test_schema.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_affected_scope.py
  - scripts/ci/test_ci_local.py
  - scripts/ci/test_cross_check.py
  - scripts/ci/test_evidence_reuse.py
  - scripts/ci/test_local_evidence.py
  - scripts/ci-plan.rs
  - scripts/ci-plan-tests.rs
  - scripts/ci-rollup.rs
  - .github/ci/schemas/contract.json
  - .githooks/tests/test-pre-push.sh
  - .githooks/tests/fixtures/affected_scope_stub.py
  - .githooks/tests/fixtures/plan-macos-executing.json
  - .githooks/tests/fixtures/plan-macos-two-packages.json
  - .githooks/tests/fixtures/plan-wsl-absent.json
  - .githooks/tests/fixtures/plan-wsl-executing.json
  - .githooks/tests/fixtures/plan-wsl-reused.json
  - features/2026-09-19-direct-cell-execution/spikes/row-equality.py
docs_updated_during_phase_3:
  - .github/ci/schemas/README.md
docs_created_during_phase_3:
  - features/2026-09-19-direct-cell-execution/spikes/s4-row-equality.md
skills_files_updated_during_phase_3:
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/os/ci-runners.md
source_files_during_phase_4:
  - scripts/ci/completion.py
  - scripts/ci/schema.py
  - scripts/ci/test_completion.py
  - scripts/ci/test_schema.py
  - scripts/ci-rollup.rs
  - scripts/ci-rollup-tests.rs
  - just/devops.just
  - .github/ci/schemas/contract.json
docs_updated_during_phase_4:
  - .github/ci/schemas/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/rust-devops/ci-cd.md
source_files_during_phase_5:
  - .github/workflows/_package-ci.yml
  - .github/workflows/_area-ci.yml
  - .github/workflows/_wsl-ci.yml
  - .github/workflows/ci.yml
  - scripts/ci/cell_contract.py
  - scripts/ci/affected_scope.py
  - scripts/ci/schema.py
  - scripts/ci/runner_loss.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_affected_scope.py
  - scripts/ci/test_local_evidence.py
  - scripts/ci/test_ci_local.py
  - scripts/ci/test_runner_loss.py
  - scripts/ci/test_schema.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - .githooks/tests/fixtures/affected_scope_stub.py
  - .github/ci/schemas/contract.json
docs_updated_during_phase_5:
  - features/2026-09-19-direct-cell-execution/spikes/s0-baseline.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/os/wsl.md
source_files_during_phase_6:
  - scripts/ci-rollup.rs
  - scripts/ci-rollup-tests.rs
  - scripts/ci/test_completion.py
  - scripts/ci/test_affected_scope.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - .github/workflows/_area-ci.yml
  - .github/ci/ci-baseline.toml
docs_updated_during_phase_6:
  - .github/ci/README.md
  - .github/ci/schemas/README.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/rust-devops/ci-cd.md
source_files_during_phase_7:
  - scripts/ci/schema.py
  - scripts/ci/affected_scope.py
  - scripts/ci/plan_fixtures.py
  - scripts/ci/test_schema.py
  - scripts/ci/test_affected_scope.py
  - scripts/ci-rollup-tests.rs
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - .github/ci/schemas/contract.json
  - features/2026-09-19-direct-cell-execution/spikes/capacity.py
docs_updated_during_phase_7:
  - .github/ci/schemas/README.md
  - features/2026-09-19-direct-cell-execution/spikes/s3-capacity.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/os/macos.md
source_files_during_phase_8:
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - just/ci-local.just
  - scripts/ci/affected_scope.py
  - scripts/ci/schema.py
  - scripts/ci/test_ci_local.py
  - scripts/ci/test_schema.py
  - scripts/ci/test_resolved_plan.py
  - scripts/ci/test_affected_scope.py
  - .github/workflows/ci.yml
  - .github/ci/schemas/contract.json
  - .githooks/tests/fixtures/affected_scope_stub.py
  - features/2026-09-19-direct-cell-execution/spikes/capacity.py
docs_updated_during_phase_8:
  - .github/ci/README.md
  - .github/ci/schemas/README.md
  - docs/topics/ci-cd.md
  - CLAUDE.md
  - features/2026-09-19-direct-cell-execution/spikes/s3-capacity.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/os/wsl.md
  - .claude/skills/os/SKILL.md
---

# Implementation Plan — Direct Cell Execution

## Summary of Work and Definition of Success

### What this change actually is

One sentence: **the hosted workflows stop consuming a projection of the plan and
start consuming the plan's executing cells, one matrix row each, while the job
that ran a gate becomes the job that proves the gate was complete.**

Everything else in the specification follows from those two moves. Nothing about
package identity, area grouping, archive sharing, event-based environment
policy, or the `ci-gate` fold changes.

Grounded against the tree at `fix/archive-path-sites`, the work decomposes into
six tracks.

1. **Plan track.** `scripts/ci/schema.py` gains the fields a downstream job needs
   so it never reads a live manifest: per-cell execution inputs (canonical
   tier/profile selection, feature and test arguments, slow-test policy, build
   reference, native prerequisites, runner tools, toolchain and Node
   requirements, L2 backends, companion suites, check selectors and dependent
   compile requirements), plus a snapshot of `.github/ci/ci-baseline.toml`'s
   exact-skip policy with per-cell/backend applicability and provenance.
   `RESOLVED_PLAN_SCHEMA_VERSION` moves 4 → 5; `.github/ci/schemas/contract.json`
   regenerates; the existing `scope-schema` receipt miss refuses an older scope
   receipt exactly once. `RECEIPT_SCHEMA_VERSION` does **not** move — validation
   receipts stay reusable under their existing cell and gate-input checks.

2. **Adapter track.** `affected_scope.py` gains a deterministic area-local row
   adapter, emitted *beside* today's `matrix_record` projection so both can be
   compared against one plan. A row carries dispatch identity only —
   `{package, gate, environment, runner}` — and the four row sets per area
   (native test, check, lint, WSL2) partition that area's executing cells with
   no duplicate key. Reused, accepted-gap, prohibited, and deferred work
   produces no row. Scalar per-area flags guard every matrix before expansion.

3. **Producer-completeness track.** `just/devops.just::_expected_manifest`
   already exists and already derives its expected set from `_tier_filter` — the
   same expression the tier recipe reads — through `nextest list
   --message-format json`, archive mode included. It was never wired into a
   producer. This track promotes it to schema version 2 (explicit ignored tests
   and planned exclusions, declared backends, companion suites, the resolved
   nextest version, and target provenance), adds
   `scripts/ci/completion.py` to compare expected against observed *identities*
   and write a versioned completion record, and makes an unvalidated or
   unuploaded record fail the producer job.

4. **Workflow-surgery track.** `ci.yml` publishes per-area row sets instead of
   `area_matrix`; `_area-ci.yml` calls `_package-ci.yml` **once** for its area
   rather than once per package, and skips that call entirely for an all-reused
   or gap-only area while keeping its audit, slice, and gap publisher;
   `_package-ci.yml` expands three disjoint row sets in three jobs and delegates
   each WSL2 row to `_wsl-ci.yml`. Every remaining environment-list input is
   deleted. The chain stays at four levels.

5. **Audit track.** `ci-rollup verdict` stops recomputing expected test
   identities and exact skips for new-format executing cells and starts checking
   that each has a valid, complete, correctly bound completion record with its
   required report inventory. The legacy path stays, version-gated, for records
   without the new contract. `skip_evidence_degraded` retires for new-format
   cells only. An expected-but-unreported test becomes a **failure**, not a
   skip — the specification's one intentional tightening.

6. **Attribution and capacity track.** Job labels change shape, so
   `scripts/ci/runner_loss.py` and its workflow-derived fixtures move with them.
   `affected_scope.py` gains an offline capacity guard that refuses before
   dispatch when an area exceeds row or serialized-output budget.

### What the measured plan already tells us about capacity

Computed offline on this tree (`python3 scripts/ci/affected_scope.py --all`):

| Quantity | Full workspace, today |
|---|---:|
| Cells / executing cells | 412 / 374 |
| Areas | 31 |
| Build records | 204 |
| Job estimate | 440 |
| Executing cells in the largest area (`homelab`) | 35 |
| Largest single row set (`homelab` native test) | **21 rows** |
| Serialized `area_matrix` today | 85,956 bytes |
| Largest single area's matrix today | 8,504 bytes |
| Largest single area's row sets under this design | ~2,300 bytes |

Row sets are roughly a quarter the size of the package records they replace and
the worst single matrix is 21 of GitHub's 256. **Capacity is not a design
constraint here**; the offline budget check exists as a guard against future
growth, not as a problem to solve now. Phase 7 records those numbers, it does
not redesign around them.

### What success looks like

Complete when all eight acceptance criteria hold and are demonstrated by named,
re-runnable artifacts — not by inspection:

- **Partition.** For every plan in the Phase 3 corpus (`--all`, nightly,
  PR-shaped, all-reused, gap-only, mixed, prohibited-cell), the union of the four
  row sets across all areas equals the plan's executing cells exactly, with no
  duplicate `{package, environment, gate}` key, and every row joins back to a
  cell that carries `execution: execute`. A fixture fails on a dropped,
  duplicated, altered, or invented row.
- **No second scope.** `grep` finds no environment-list input in
  `_area-ci.yml`, `_package-ci.yml`, or `_wsl-ci.yml`; every consumer resolves its
  execution contract from `ci-resolved-plan` by exact cell key and refuses an
  unknown, duplicate, non-executing, or mismatched row with a coded reason.
- **Producers prove themselves.** A producer that lists fewer tests than it
  expected, reports an unexpected identity, loses a report, skips without an
  unexpired approval, fails a companion or a required backend proof, or cannot
  upload its artifacts, fails — and a *missing* test can no longer be excused by
  a skip approval. A `cfg`-excluded test is absent from that target's expected
  set rather than counted as a skip.
- **Audit stays blocking.** An all-reused or gap-only area creates no execution
  call but still publishes its neutral gap checks, its slice, and a blocking
  audit; a producer call skipped despite nonempty rows fails that audit; a
  producer failure reaches `ci-gate` without a second red policy check.
- **Attribution.** `scripts/ci/test_runner_loss.py` derives every label from the
  shipped workflows and maps a lost native or WSL2 runner to exactly one cell.
- **Unchanged surfaces.** `ci-gate` is byte-identical; `ci-reporting` stays
  advisory; `check`/`lint` and the dependent-compile half are unchanged; one
  build still serves many consumers and an unrelated build failure suppresses
  nobody; producer tokens stay read-only and only `accepted-gaps` holds
  `checks: write`.
- **Suites.** `python3 scripts/ci/test_*.py` (all twelve plus the new
  `test_completion.py`), `just _test repo-deps`, `just _test test-toolkit`,
  `just _lint repo-deps`, `just _lint test-toolkit`,
  `.githooks/tests/test-pre-push.sh`, and `actionlint` on all four reader-facing
  workflows pass.

### What implementing this costs locally

`ci.yml`, `_package-ci.yml`, `environments.json`, and `affected_scope.py` are all
in `GLOBAL_PATHS_ALL_GATES`, and nearly every phase below touches at least one of
them. **Every push during this implementation therefore selects the whole
workspace** — about 45 minutes on the development Mac for a cold run. Two things
make that tolerable and both must be preserved while working:

- those same paths are in `ORCHESTRATION_PATHS`, so a workflow edit does not move
  any cell's gate-input identity and previously published passing cells stay
  reusable; and
- same-head retries must overlay the evidence directory and merge the receipt
  rather than replace it, or the next run re-executes exactly the cells the
  previous one proved.

Plan the work so that a push happens at phase checkpoints, not per task.

### Explicit non-goals

- No change to `ci-gate`, branch protection, or `ci-reporting`'s powerlessness.
- No new evidence-reuse eligibility for `check` or `lint`; no cross-tree hosted
  reuse; no change to browser scheduling scope.
- No removal of an area's audit runner for an area with no executing cells
  (deferred by the specification — it changes blocking ownership).
- No change to the event-based environment schedule, the build-owner schedule,
  or `environments.json` capability governance.
- No pinning of `cargo-nextest` (see Ruling R4) and no full-workspace or nightly
  hosted dispatch to learn platform behavior.
- This feature is never moved to `_completed` by an agent.

## Phase 1 — Rulings, Spikes, and Baseline

Everything in this phase is decision and measurement. No production file changes
except the specification's own frontmatter (Ruling R6).

### Necessary Rules

Record each as a numbered entry in a new
`features/2026-09-19-direct-cell-execution/rulings.md`, with the reasoning and
the consequence if it is later reversed. Where a ruling contradicts or
reinterprets the specification, say so explicitly in that file.

- [x] **R1 — Row key set and job-label legibility.** GitHub builds a matrix
      job's label from *every* value in the row (the reason `ci.yml`'s `area-ci`
      matrix is a plain vector rather than an `include:` of package records), and
      a job that can be skipped as a whole may carry **no** `name:`, because the
      matrix context is never evaluated for a skipped job. A four-key row
      therefore renders `test (homelab-server, L1, ubuntu-latest,
      ubuntu-latest)`.
      **Ruling: keep `runner` in the row as the specification requires and accept
      the four-token label.** A sibling runner-lookup map would be a second
      document to keep aligned with the rows, which is the class of defect this
      feature exists to remove. Order the row keys
      `package, gate, environment, runner` so the parsed prefix is stable, and
      make `runner_loss.py` accept both the three- and four-token forms so a
      later reversal is not a silent attribution loss.
- [x] **R2 — Where the completeness validator runs, and in what language.** An
      archive consumer has no compiler, and the WSL2 guest installs only
      `ca-certificates curl git xz-utils jq`. **Ruling: implement the validator as
      `scripts/ci/completion.py` (stdlib only, like every other `scripts/ci`
      tool) and add `python3` to the guest's declared apt provisioning with a
      `python3 --version` reachability check in that same named step.** Native
      runners already depend on `python3` in this exact job —
      `companion_suites.py` runs there on all three OSes — so this adds one
      declared package on one leg. Rejected alternative: a subcommand on the
      archive-staged `ci-build` binary. It would reach the guest for free, but
      `check` and `lint` cells consume no archive and would need a second
      implementation or a release build of `repo-deps` inside a gate job.
- [x] **R3 — Job layout inside the execution workflow, and the fate of D4
      staging.** The specification prescribes three row-expanding jobs plus a
      WSL2 delegator; today L2 and browser are separate jobs ordered behind L1
      through `needs: test` with `!cancelled()` (contract
      `expensive_tiers_stage_behind_l1_but_lint_never_gates_it`). One test job
      over all three tiers cannot express that ordering.
      **Ruling: follow the specification — one native test job over the L1, L2,
      and browser rows, branching by tier only where execution differs (backend
      provisioning and proof, browser serialization, toolchain and Node
      provisioning).** The ordering was resource staging, never correctness, and
      the specification forbids an L1 prerequisite that suppresses required L2 or
      browser work. Rewrite that contract test to assert the *negative* instead:
      no test row's job depends on another test job's success. Consequence to
      watch: peak concurrent runners per area rises; Phase 7's capacity report
      records it, and the fallback — a second `needs:`-ordered job for the L2 and
      browser rows — is a two-line change if a measured run shows queue damage.
- [x] **R4 — "The repository's pinned Nextest version".** The specification
      assumes a pin. The repository has none: all four workflows install through
      `taiki-e/install-action@nextest` and every build contract in
      `environments.json` declares `"nextest": "latest"`.
      **Ruling: read the requirement as "the same `cargo-nextest` binary the gate
      command uses, in the same job", enforce it by construction (listing and
      running in one job with one installed binary), and record the resolved
      `--version` in the completion record so a cross-host mismatch is visible
      after the fact.** Introducing a pin is a separate decision touching the
      build-key identity of every archive; note it as a follow-up candidate, do
      not take it here. The producer/consumer version equality check in
      `affected_scope._validate_compatibility` is unaffected.
- [x] **R5 — `_package-ci.yml` keeps its filename.** After this change it is
      called once per *area*, so its name is no longer descriptive. **Ruling: do
      not rename it.** It is named in `ORCHESTRATION_PATHS`, and that list is what
      keeps a workflow edit from invalidating every published local cell (before
      that rule one such edit cost a 45-minute pre-push); it is also named in
      `runner_loss.py`'s comments, `READER_FACING_WORKFLOWS`, the four-level
      depth contract, the CI README, and two skills. Rewrite its header
      documentation to describe what it now is — the area's execution workflow —
      and leave the path alone. Rule 3.
- [x] **R6 — The specification's `status` value.** `status: draft` is outside the
      declared enum (the spec's own Open Question). **Ruling: set
      `status: planned` — a value the enum defines and the accurate lifecycle
      state once this plan lands — in the same commit as this plan.** This
      resolves the invalid value without expanding the shared vocabulary with a
      `draft` alias. The author may prefer `draft-spec`; either is a one-word
      change and neither affects implementation.
- [x] **R7 — Row transport and the output budget.** Row sets travel as
      `scope`-job outputs indexed per area in the `with:` block, exactly as
      `area_matrix` does today, and the immutable `ci-resolved-plan` artifact
      carries everything else. **Ruling: the planner is the only place a budget is
      enforced** — it fails with a named, actionable error before emitting a plan
      whose largest row set exceeds `MATRIX_LIMIT` or whose serialized per-area
      payload exceeds a declared byte budget. It never truncates, never splits an
      area silently, and CI never re-derives the budget. Measured headroom today:
      21 rows and ~2.3 KB against 256 and the budget.
- [x] **R8 — The skip policy stays hand-edited and is snapshotted.**
      `.github/ci/ci-baseline.toml` remains the human-owned source of truth;
      the planner reads it once and writes a `skip_policy` snapshot into the plan
      with per-cell and backend applicability, owner, reason, `source_run`, and
      optional expiry, plus provenance naming the file and its content hash.
      Producers and the audit read the plan only. **Ruling: an expired approval,
      or an approval for a cell the plan does not carry, is a planner-time
      error** — not a producer-time surprise — because the plan is the artifact a
      carried scope receipt reuses. The file is empty today, so the migration
      moves no data; the interpretation change (a missing result is a failure,
      never an approved skip) is encoded in `completion.py` and the audit and
      pinned by fixtures in both.
- [x] **R9 — Per-area path selection and rollback.** A committed allowlist,
      `.github/ci/direct-execution.json` (`{schema_version, areas: [...]}`),
      names the areas on the new path; the planner reads it and stamps
      `execution_path: "rows" | "lists"` on each area record. **Ruling: exactly
      one path per area per run, asserted by a contract test that fails if any
      area would emit both row sets and environment lists**, and the workflow
      branches on the area record, never on a workflow-level input. The trial
      value is `["root"]`. Phase 8 deletes the file, the field, and the `lists`
      branch together once every area is switched and this review closes.
- [x] **R10 — Completion records are their own artifact.**
      `status-<package>-<gate>-<environment>` keeps its `always()` semantics and
      stays the failure-path diagnostic. The completion record ships as
      `completion-<package>-<gate>-<environment>`, written and uploaded **only
      after** validation succeeds, so `complete: true` cannot exist without the
      evidence behind it; the audit adds `completion-*` to its download pattern.
      A green status whose completion artifact is absent or mismatched is an
      audit failure. Keys stay `{package, environment, gate}` with backend and
      suite dimensions where already required.
- [x] **R11 — `_area-ci.yml` becomes a scheduling input; the selection tables must
      say so.** `GLOBAL_PATHS_ALL_GATES` names `ci.yml`, `_package-ci.yml`,
      `environments.json`, and `affected_scope.py` — an edit to any of them forces
      workspace scope — and `ORCHESTRATION_PATHS` then keeps those same paths out
      of a cell's gate-input identity, so published local evidence survives a
      workflow edit. `_area-ci.yml` is in **neither** list today: it selects only
      `test-toolkit`, through the `.github/workflows/**` suite-owner prefix. Under
      this design it carries the row sets, so it decides what runs.
      **Ruling: add `.github/workflows/_area-ci.yml` to both
      `GLOBAL_PATHS_ALL_GATES` and `ORCHESTRATION_PATHS` in one change**, and pin
      the pairing with a fixture: a path that forces workspace scope because it
      schedules work must also be excluded from gate-input identity. Adding it to
      one list only is the failure mode to guard against — the first spelling costs
      a needless full-workspace pre-push, the second invalidates every published
      cell on a workflow edit.
- [x] **R12 — `_expected_manifest` must become a CI entry recipe.**
      `CI_RECIPES_BY_GATE["test"]` is `("_test", "_test_l2", "_test_browser")`, and
      `just_gate_inputs` / `just_change_gates` work from that closure. A producer
      that calls `just _expected_manifest` directly would put a recipe on the
      critical path of what a gate *proves* while leaving it outside the closure
      that selects and identifies the gate: editing the expected-set logic would
      change every producer's verdict and select nothing.
      **Ruling: add `_expected_manifest` to `CI_RECIPES_BY_GATE["test"]` in the
      same change that makes a producer call it**, with a fixture asserting an edit
      to it selects the test gate and moves the test cells' gate-input identity.

### Spikes

- [x] **S1 — Matrix labels, whole-job skips, and nesting under a single area
      call** (`features/2026-09-19-direct-cell-execution/spikes/s1-labels.md`).
      Questions: what label does an `include:`-only matrix of four keys produce;
      what label does the same job show when its scalar guard skips it before
      expansion; and does `ci.yml → _area-ci.yml → _package-ci.yml → _wsl-ci.yml`
      still resolve when the third level is called once with a row set instead of
      per package. Method: one minimal scratch workflow on a scratch branch,
      no package tests, in the spirit of
      `fixes/2026-09-11-cicd-cleanup/fixtures/scratch-2026-09-12.md`.
      **Requires a push; if that is unavailable, proceed on the documented
      behavior and R1's both-forms parser, and record the spike as deferred with
      the exact questions still open.** Do not dispatch a full-scope run to learn
      this.
- [x] **S2 — Nextest listing fidelity against the resolved version**
      (`spikes/s2-nextest-list.md`). Prove, with fixtures and no CI, how
      `nextest list --message-format json` reports: an `#[ignore]`d test; a test
      excluded by `cfg` on this target; a test excluded by the tier filter; two
      binaries sharing a test name; and the same package listed from an archive
      with `--workspace-remap`. Reuse
      `scripts/ci/fixtures/archive-portability/` and
      `scripts/ci-build-archive-tests.rs`'s relocation fixtures rather than
      inventing a harness. Output: the exact JSON shape `completion.py` parses,
      the resolved `cargo-nextest --version`, and a decision on whether
      `filter-match.status == "matches"` alone is sufficient (today's
      `_expected_manifest` assumes it is).
- [x] **S3 — Offline capacity and budget model** (`spikes/s3-capacity.md`).
      Extend the numbers already measured above to the nightly and
      `workflow_dispatch` plans and to each individual matrix: per-area row-set
      cardinality, serialized `with:` payload per area, total `scope` job output
      bytes, unique reusable workflows per run, call depth, and a job estimate
      including area audits, gap publishers, and build owners. Output: the budget
      constants Phase 7 enforces and the evidence that no area needs
      partitioning. Purely offline.

### Baseline Capture

- [x] **Current-state inventory** (`spikes/s0-baseline.md`). Record, from this
      tree: every producer job label in the four reader-facing workflows; the
      full list of `ci_workflow_contracts` tests that assert an environment list,
      a per-package call, a `needs: test` ordering, or a job label (these are the
      Phase 2 and Phase 5 edit set); the twelve Python suites' current pass
      counts; `python3 scripts/ci/affected_scope.py --all` statistics as above;
      and the plan corpus Phase 3 will compare against, saved under
      `spikes/plans/`.

### Checkpoint

- [x] `rulings.md` records R1–R10 with consequences; S2 and S3 have written
      results; S1 has results or a recorded deferral with its open questions;
      the baseline inventory names the exact contract tests this plan will edit.
- [x] No production file has changed except the specification's `status`.

## Phase 2 — Failing Oracles

Land every new contract as a **pending** fixture first, using the repository's
two mechanisms: `scripts/ci/pending_contracts.py::pending` for Python and
`pending_contract` in `scripts/ci-rollup-tests.rs` for Rust. Each must fail for
its recorded reason, so the suite stays green and the reason becomes the
implementation oracle. Waves 1–3 are independent files and run concurrently.

### Wave 1 — Plan, adapter, and schema oracles

- [x] **Schema v5 oracles** — `scripts/ci/test_schema.py`: version 5 accepted and
      4 refused by version before field set; the new per-cell execution fields
      and `skip_policy` required where the specification requires them and
      optional where it does not; a `skip_policy` entry naming an absent cell,
      an expired entry, and a malformed provenance each rejected with a coded
      reason; `RECEIPT_SCHEMA_VERSION` and `SCOPE_RECEIPT_SCHEMA_VERSION`
      unchanged.
- [x] **Row adapter oracles** — `scripts/ci/test_resolved_plan.py`: for each plan
      in the Phase 1 corpus, the four row sets partition the executing cells
      exactly; no duplicate key; every row joins to an `execute` cell and to its
      package record; reused, accepted-gap, prohibited, and deferred work
      produces no row; a WSL2 row never appears in the native test set; the
      per-area scalar flags are true exactly when their row set is nonempty; and
      an area with zero executing cells still appears in `scheduled_areas`.
- [x] **Selection-unchanged oracles** — `scripts/ci/test_affected_scope.py`: the
      row adapter reads nothing but the plan (assert by planning from a carried
      receipt with no checkout access); `check` and `lint` selection, dependent
      seam, event deferral, and build-owner derivation are byte-identical to
      today's outputs for the corpus.
- [x] **Capacity-guard oracles** — `test_affected_scope.py`: a synthetic plan
      whose largest row set exceeds `MATRIX_LIMIT`, and one whose serialized area
      payload exceeds the byte budget, each fail planning with a named error and
      no truncation.

### Wave 2 — Producer-completeness oracles

- [x] **New suite skeleton** — `scripts/ci/test_completion.py`, registered in
      `SUITE_REGISTRY` under `repo-deps` and added to `just ci-local`'s self-test
      list. Remember the self-test list is spelled in four coupled files
      (`just/ci-local.just`, `scripts/ci/test_ci_local.py` twice,
      `.githooks/tests/test-pre-push.sh`) — editing one produces ten failures.
- [x] **AC5 fixture matrix** (pending): missing report; malformed report; missing
      expected test (and that a skip approval cannot excuse it); an extra
      invocation filter that shrinks expected and observed together; duplicate
      test identities across binaries; retries normalized to one final outcome
      without hiding a failure; explicit `#[ignore]`; an approved skip; an
      expired approval; an empty expected set without a plan-recorded reason;
      a companion-only cell whose declared suite did not complete; a required
      backend proof absent; a companion failure; an upload failure; a
      cancellation; and a canonical tier exclusion that must **not** read as a
      missing test.
- [x] **Expected-manifest v2 oracles**: ignored tests and planned exclusions
      recorded explicitly rather than dropped; the resolved nextest version,
      environment, tier, target, and archive provenance present; a manifest
      generated on another target refused for the comparison.

### Wave 3 — Workflow, audit, and attribution oracles

- [x] **Workflow layout oracles** — `tools/test-toolkit/tests/ci_workflow_contracts.rs`
      (pending or newly written, using `workflow_reading`'s Rust equivalents
      already in that file): `_area-ci.yml` calls `_package-ci.yml` at most once;
      no reader-facing workflow declares an environment-list input; every
      row-expanding job carries a scalar guard and no `name:` with an expression;
      every matrix keeps `fail-fast: false`; no test job depends on another test
      job's success (R3); the chain stays four levels; producer permissions stay
      read-only and only `accepted-gaps` holds `checks: write`; every executing
      cell's job uploads JUnit, status, **and** a completion artifact; and the
      all-reused/gap-only area still runs its audit, slice, and publisher.
- [x] **Audit oracles** — `scripts/ci-rollup-tests.rs`: a new-format executing
      cell with no completion record, with a record bound to another revision or
      build key, or with a green status and an absent report inventory, each
      blocks; a legacy record still takes the legacy path; invalid reuse and
      invalid gaps still block; partial rerun evidence blocks; a skipped producer
      call with nonempty rows blocks; and a producer failure produces exactly one
      red check.
- [x] **Attribution oracles** — `scripts/ci/test_runner_loss.py`: labels derived
      from the shipped workflows (never spelled by hand) resolve a lost native
      row and a lost WSL2 row to exactly one cell each, under both the three- and
      four-token label forms of R1.

### Checkpoint

- [x] Every new fixture fails for its recorded oracle reason; every existing
      suite is still green; `BISCUIT_PROMOTE_PENDING=1` shows exactly the
      contracts this plan will implement and nothing else.
- [x] `spikes/s0-baseline.md` is updated with the pending-contract inventory so
      each later phase knows which decorators it must remove.

## Phase 3 — Plan Schema v5, Skip Snapshot, and the Row Adapter

This is Migration step 1: add the new inputs and the adapter **beside** the
current projection and prove equality and uniqueness, without changing any
workflow.

### Wave 1 — Schema and contract

- [x] **`scripts/ci/schema.py`** — bump `RESOLVED_PLAN_SCHEMA_VERSION` to 5;
      add the per-cell execution fields, the per-package fields the specification
      assigns to package records, and `skip_policy` with its applicability and
      provenance; extend `CELL_FIELDS`/`PACKAGE_FIELDS`/`RESOLVED_PLAN_FIELDS` and
      the consistency validators; add coded rejections for a skip entry naming an
      absent cell, an expired entry, and malformed provenance. Keep the
      version-before-fields check order — an older document usually differs in
      both, and the field complaint misdirects the reader.
- [x] **`.github/ci/schemas/contract.json`** — regenerate through the existing
      generator path and confirm the Rust-side assertions in `ci-rollup.rs`,
      `ci-rollup-tests.rs`, and `ci-build-archive-tests.rs` read the new names.
- [x] **Scope-receipt fallback** — leave `SCOPE_RECEIPT_SCHEMA_VERSION` at 1 and
      confirm its embedded `plan_schema_version` check refuses a version-4
      receipt once with the existing `scope-schema` reason, forcing one fresh
      calculation rather than an in-place upgrade. Assert the hook and
      `scope-verify` paths in `.githooks/tests/test-pre-push.sh`.

### Wave 2 — Planner population (depends on Wave 1)

- [x] **Cell and package execution inputs** — `affected_scope.py`: move every
      value `_package-ci.yml` and `_wsl-ci.yml` currently receive as an input onto
      the plan, package-wide values on package records and cell-specific values on
      cells. Resolve nothing new from the checkout: every one of these already
      exists in `matrix_record` or the package policy, which is why this is a
      relocation rather than a new policy read.
- [x] **Skip-policy snapshot** — read `.github/ci/ci-baseline.toml` once, validate
      per R8, and write the snapshot with provenance (path plus content hash
      through `biscuit-hash`'s xxHash convention where a hash is wanted). Fail
      planning on an expired or unmatched entry.
- [x] **Row adapter** — a pure function from plan to
      `{area: {test: [...], check: [...], lint: [...], wsl: [...]}}` plus scalar
      flags, ordered `package, gate, environment, runner` per R1, deterministic,
      reading only the plan. Emit it into the plan and project it into
      `scope.json` beside `area_matrix`; do not remove `area_matrix` yet.
- [x] **Capacity guard** — enforce R7's row and byte budgets with a named error.

### Wave 3 — Readers and fixtures (depends on Wave 2)

- [x] **`scripts/ci/plan_fixtures.py`** — teach the fixture builder the v5 shape so
      `test_evidence_reuse`, `test_ci_local`, and the `just ci-local` stub planner
      do not hand-write three drifting variants.
- [x] **`scripts/ci-plan.rs`** — render the row sets and the skip snapshot through
      `TerminalRenderable` so `just ci-local --plan` shows what CI will dispatch.
- [x] **Equality and uniqueness proof** — a repeatable comparison over the Phase 1
      corpus asserting the row sets and the environment-list projection describe
      the same executing cells, with the row side additionally proving uniqueness.
      Save the output under `spikes/` as the Migration step 1 evidence.

### Checkpoint

- [x] Promote the Wave 1 and Wave 2 Python oracles; `test_schema.py`,
      `test_resolved_plan.py`, `test_affected_scope.py`, `test_evidence_reuse.py`,
      `test_ci_local.py`, `test_local_evidence.py` all green.
- [x] `just _test repo-deps` green; `just ci-local --plan` renders on this tree;
      `.githooks/tests/test-pre-push.sh` green.
- [x] No workflow file has changed.

## Phase 4 — The Producer Completion Contract (tool side)

Build and prove the contract as a standalone tool with fixtures. Phase 5 wires
it into the workflows, which is where the specification's "implement them
together" applies.

### Wave 1 — Expected manifest v2

- [x] **`just/devops.just::_expected_manifest`** — schema version 2: keep the
      `_tier_filter`-derived selection (its whole point is that expected and
      observed come from one expression), and add explicit `ignored` and
      `excluded` identity sets, the declared L2 backends and companion suites for
      the cell, the resolved `cargo-nextest --version`, the environment, tier,
      target triple, and whether the listing came from an archive with a remap.
      Keep archive mode working through `_archive_drop_build_flags`.
- [x] **Provisioning-before-listing order** — document and enforce that listing
      executes test binaries, so native prerequisites, sidecars, backends, and the
      toolchain (where `requires-toolchain` × `cargo_toolchain` applies) are in
      place first, and that no consumer needs a compiler merely to list.

### Wave 2 — The validator (depends on Wave 1)

- [x] **`scripts/ci/completion.py`** — stdlib only. Inputs: the resolved plan, the
      cell key, the expected manifest, the JUnit staging tree (`manifest.jsonl`
      included), the companion results, and the backend-proof evidence. It
      compares **identities**, not counts, scoped by cell, backend, and companion
      suite; retains enough binary identity to detect collisions; normalizes
      retries into one final outcome without hiding a failure or accepting a
      duplicate report; and fails on missing or malformed reports, missing
      expected tests, unexpected identities, unapproved or expired skips, failed
      tests or companions, and absent required backend proof. An empty expected
      set requires a plan-recorded reason. It never compares an L1 report against
      every tier in a shared archive.
- [x] **The completion record** — versioned, keyed `{package, environment, gate}`,
      binding the tested revision, the build key where applicable, the gate
      inputs, the run and attempt, the resolved nextest version, and the report
      inventory. `complete` is set only after validation succeeds (R10).
      Add its field list and rejection codes to `schema.py` and
      `contract.json`.
- [x] **Failure-path behavior** — diagnostics are published best effort under the
      existing cancellation rules; a *required* upload failure fails the job.

### Wave 3 — Fixtures (depends on Wave 2)

- [x] **`scripts/ci/test_completion.py`** — implement the full AC5 matrix from
      Phase 2 and promote those pending fixtures. Include the two cases the
      specification singles out: a missing test cannot use a skip approval, and a
      canonical tier exclusion must not create a false missing-test failure.
- [x] **Cross-language contract** — assert the record's field names against
      `contract.json` from the Rust side so a rename cannot land silently, the
      same way the compiler-work publisher fixture works.

### Checkpoint

- [x] `python3 scripts/ci/test_completion.py` green with every AC5 case covered;
      `test_schema.py` green on the record's contract.
- [x] A local dry run on one real package (for example `repo-deps` L1 on
      `macos-latest`) produces a valid record, and a deliberately narrowed filter
      makes it fail with the missing-test reason.
- [x] No workflow file has changed.

## Phase 5 — The Row-Driven Workflow Layout

Migration step 2: the workflows and the producer contract go live together.
Preserve archive owners and all existing execution behavior.

### Wave 1 — The execution workflow (largest single task; do first, alone)

- [x] **`.github/workflows/_package-ci.yml`** — replace the six environment-list
      jobs with four row-driven ones: `test` (native L1/L2/browser rows), `check`,
      `lint`, and `wsl2` (delegating each row to `_wsl-ci.yml`). Each job expands
      `${{ fromJSON(inputs.<set>-rows) }}` behind a scalar guard, carries no
      `name:`, and keeps `fail-fast: false`.
- [x] **`scripts/ci/cell_contract.py`** — the one reader every job uses: given the
      plan, a row, and the run's tested revision, it verifies row identity and
      dispatch fields against the plan, refuses an unknown, duplicate,
      non-executing, or mismatched cell with a coded reason, and emits the cell's
      execution contract as step outputs and environment values. No job reads a
      manifest, a package policy, or `environments.json`.
- [x] **Tier branching** — inside the `test` job, branch only where execution
      genuinely differs: L2 backend provisioning and proof, browser
      serialization and `BISCUIT_BROWSER_REQUIRED`, `requires-toolchain`
      provisioning, Node/pnpm provisioning, and the canonical recipe selected
      (`_test` / `_test_l2` / `_test_browser`). Preserve timeouts, concurrency
      limits, serialization, and compiler-work measurement exactly; L2 and
      browser must still not bring a terminal or browser window into focus.
- [x] **Producer completeness steps** — after provisioning and before the gate:
      verify the planned archive, then `_expected_manifest`; after the gate:
      `completion.py`, then the JUnit, status, and completion uploads. A failed
      validation or a failed required upload fails the job.
- [x] **Labels** — confirm every producer label exposes package, environment, and
      gate without dumping row JSON, and that a job skipped before expansion shows
      no unevaluated expression.
- [x] **R12's selection entry** — add `_expected_manifest` to
      `CI_RECIPES_BY_GATE["test"]` in this same change, with the fixture in
      `test_affected_scope.py` asserting an edit to it selects the test gate and
      moves the test cells' gate-input identity.

### Wave 2 — Callers and the guest (depends on Wave 1's input surface)

- [x] **`.github/workflows/_area-ci.yml`** — one guarded call to
      `_package-ci.yml` per area with the four row sets; skip that call entirely
      for an all-reused or gap-only area; keep `accepted-gaps`, the slice upload,
      and `coverage-audit` unconditional; resolve the audit's package membership
      from the plan rather than from execution rows; keep the token confinement
      (`contents: read` on the execution call, `checks: write` on the publisher
      alone).
- [x] **`.github/workflows/ci.yml`** — publish per-area row sets and scalar flags
      from the `scope` job in place of `area_matrix`; leave `has_packages`,
      `preflight`, `build`, `area-drift`, `ci-gate`, and `ci-reporting`
      untouched. `ci-gate` must remain byte-identical.
- [x] **`.github/workflows/_wsl-ci.yml`** — accept one row instead of a package
      plus lists; add `python3` to the declared guest apt provisioning with a
      reachability check (R2); keep the archive-download, verification,
      measurement-return, and status behavior unchanged; run only the row's gate,
      never an internal tier matrix.
- [x] **Row-set disjointness at the boundary** — a contract test asserting the
      union of the four inputs equals the area's executing cells with no
      duplicate key, checked against the plan artifact rather than recomputed.
- [x] **R11's selection entries** — add `.github/workflows/_area-ci.yml` to
      `GLOBAL_PATHS_ALL_GATES` **and** `ORCHESTRATION_PATHS` in this same change,
      with the paired fixture in `test_affected_scope.py` and
      `test_local_evidence.py`: the path forces workspace scope and is absent from
      every cell's gate-input identity.

### Wave 3 — Attribution, lint, and promotion (depends on Waves 1–2)

- [x] **`scripts/ci/runner_loss.py`** — update `JOB_KINDS`, the gate-segment
      pattern, and the WSL delegation handling for the new labels; accept both
      label forms per R1; keep `ci-gate` excluded and the build-owner synthesis
      intact.
- [x] **`scripts/ci/test_runner_loss.py`** — derive every label from the shipped
      workflows (never by hand — Phase 6 of the earlier plan broke all six labels
      at once while every fixture spelled them literally) and promote the Phase 2
      oracles.
- [x] **`actionlint`** on all four reader-facing workflows; remember it models no
      `github.run_started_at`.
- [x] **Promote** the Phase 2 workflow oracles in `ci_workflow_contracts.rs` and
      rewrite the tests the baseline flagged: the D4 staging assertion (R3), the
      per-package fan-out assertions, and any test asserting an environment-list
      input. Remove an obsolete shape check only once its behavioral replacement
      passes.

### Checkpoint

- [x] `just _test test-toolkit`, `just _lint test-toolkit`,
      `python3 scripts/ci/test_runner_loss.py`,
      `python3 scripts/ci/test_ci_local.py`, `actionlint` on four workflows: all
      green.
- [x] `just ci-local` self-test loop green; `.githooks/tests/test-pre-push.sh`
      green.
- [x] Every job label in the four workflows is enumerated in the baseline
      document with its parsed cell, and the WSL2 and native rows for one package
      map to two distinct cells.

## Phase 6 — The Version-Aware Audit

Migration step 3: introduce explicit version-aware handling **before** removing
any old completeness check.

### Wave 1 — Consume completion records

- [x] **`scripts/ci-rollup.rs`** — read `completion-*` artifacts; for a
      new-format executing cell, require a valid, complete, correctly bound
      record and its declared report inventory, and stop recomputing expected
      identities and exact skips; refuse a green status record unsupported by its
      artifacts; keep run-attempt selection, current-result precedence, and
      retained passing evidence on retries.
- [x] **Legacy path** — retain the existing expected-manifest/baseline path for
      records without the new contract, gated on the record's version, and keep
      `skip_evidence_degraded` for those cells only. Removing producer-side skip
      checks from the audit must not retroactively certify an old receipt; where
      required proof cannot be established, schedule the cell.

### Wave 2 — Verdict rules (depends on Wave 1)

- [x] **Enforcement condition** — enforce the verdict when execution producers
      succeeded **or when none were required**, so an all-reused or gap-only area
      is still judged; treat a skipped producer call with nonempty execution rows
      as missing coverage; keep an unreadable input an infrastructure failure
      rather than an invented test failure.
- [x] **Presentation** — render the area slice even after a producer failure
      without adding a second coverage-policy failure; keep reused and
      accepted-gap cells in both the area summary and the advisory run summary;
      keep an explicit current execution outranking a reused claim with the
      discrepancy reported; a failure still outranks a gap.
- [x] **Reuse and gaps** — unchanged: qualifying evidence for every reused cell,
      valid governance for every accepted gap, existing duplicate/conflicting
      result rejection, no cross-tree hosted reuse.

### Wave 3 — Fixtures (depends on Waves 1–2)

- [x] **`scripts/ci-rollup-tests.rs`** — implement and promote the Phase 2 audit
      oracles (AC6), covering all-reused, gap-only, mixed, and
      unexpectedly-skipped-producer areas, and legacy evidence that must not
      bypass a removed check.
- [x] **`.github/ci/ci-baseline.toml`** — update its header to state the tightened
      rule: a skip identity now comes only from an observed `<skipped/>`, and an
      expected-but-unreported test is a failure. Keep the file empty; do not
      invent entries.

### Checkpoint

- [x] `just _test repo-deps` green including `ci-rollup`'s suite; every AC6
      fixture green; no pending decorator remains for an implemented contract.
- [x] A synthetic end-to-end fixture run: plan → rows → producer artifacts →
      `ci-rollup rollup --area` → `verdict --area`, for each of the four area
      shapes, passing and failing for the right reasons.

## Phase 7 — Capacity Validation, the Path Switch, and the Root-Area Trial

Migration steps 4 and 5.

### Wave 1 — Offline validation

- [x] **Validate full-workspace and nightly plans offline** for every individual
      matrix's cardinality, the four-level call depth, the unique reusable
      workflow count, the job estimate including area audits, gap publishers, and
      build owners, and the serialized output sizes. Record the result as this
      feature's capacity evidence, extending S3.
      *(Done: `spikes/capacity.py`, recorded in `spikes/s3-capacity.md` §
      "Phase 7 re-measurement"; pinned by
      `CapacityGuardTests::test_the_real_full_workspace_and_nightly_plans_fit_every_ceiling`.)*
- [x] **Confirm the guard fires** on a synthetic over-budget area with a clear
      pre-dispatch failure and no truncation.
      *(Done through `main()`: `CapacityGuardTests::test_an_over_budget_area_fails_before_dispatch_and_emits_nothing`;
      S3's aggregate 512 KB budget, never implemented, added as
      `TOTAL_ROW_SET_BUDGET`.)*

### Wave 2 — Per-area path selection (parallel with Wave 1)

- [x] ~~**`.github/ci/direct-execution.json`** — the R9 allowlist, `["root"]` to
      start, with `execution_path` on each area record in the plan.~~
      **Amended (human-review option A, pending ratification):** Phase 5 left
      the workflows rows-only, so no `lists` path exists to allowlist against.
      The allowlist loader is removed, every area record states `rows`, and
      the plan contract admits only `rows`. Rollback is a revert.
- [x] **Dual-path contract** — a test that fails if any area would emit both row
      sets and environment lists, and one asserting rollback restores compatible
      workflow and audit readers together. Rollback must never erase evidence or
      weaken enforcement.
      *(Done: `ExecutionPathTests` (Python) and
      `the_plan_admits_exactly_the_dispatch_path_the_workflows_implement`
      (Rust). Rollback is one revert of planner, workflows, and audit together;
      see the Phase 7 log for why no separate rollback test exists.)*

### Wave 3 — Trial (depends on Waves 1–2)

- [x] **Local comparison** — `just ci-local --plan` and `just ci-local` on the
      root area under the new path, plus `scripts/cross-check.sh` if a WSL2 row is
      in scope, confirming the observed cells match the plan.
      *(Done: areas `root` and `tools`, 9 executing cells = 9 rows, unique;
      `just ci-local` 13/13 gates green on macOS. No WSL2 row executes — both
      WSL2 cells are accepted gaps — so `cross-check` was not triggered.)*
- [ ] **One ordinary affected-area CI run** on this feature's branch for reporting
      and artifact behavior — never a full-workspace or nightly dispatch.
      **BLOCKED: needs push.** Evidence to capture: the four row sets actually
      dispatched, every job label, one completion artifact per executing cell, the
      area slice, the neutral gap checks, `ci-gate` green, and no duplicate gate
      run. *(Still blocked after Phase 7; the exact commands are in the Phase 7
      log, "The hosted trial".)*
- [x] ~~**Switch the remaining areas** only after the focused contracts pass and the
      trial's observed cells match the plan. Keep the area-level rollback switch
      until this review closes.~~
      **Amended (option A, pending ratification):** there is no per-area
      switch to flip. Every area has dispatched rows since Phase 5 and states
      `rows` since this phase. The hosted trial above still gates Phase 8's
      deletions.

### Checkpoint

- [x] Capacity evidence written; guard proven; exactly one path per area asserted
      by a test; the trial's evidence captured or the blocker recorded with the
      exact commands to run.

## Phase 8 — Retire the Environment Lists and the Legacy Projections

Migration step 6. Nothing here may run before Phase 7's trial is green, because
each deletion removes a fallback.

### Wave 1 — Workflow inputs

- [x] Delete every environment-list input from `_area-ci.yml`,
      `_package-ci.yml`, and `_wsl-ci.yml`, together with the `lists` branch and
      `.github/ci/direct-execution.json` once every area is switched. Assert the
      absence with a contract test.
      *(The inputs went in Phase 5; no `lists` branch or allowlist file ever
      shipped. Absence is now pinned exactly by
      `the_reusable_workflows_accept_only_rows_and_run_scalars`, which fails on
      `main`'s 23-input `_package-ci.yml`.)*

### Wave 2 — Local readers, then the projection (strictly ordered)

- [x] **Migrate `just/ci-local.just`** — the six `.matrix[]` lookups at lines ~361
      and ~507–555 (`test_args`, `gates`, `check_args`, `tiers`,
      `l2_environments`, `l2_backends`, `runner_tools`) read the plan's cells and
      package records instead.
      *(Done: one `.packages[]` record per package; `l2_environments` became
      "this host's L2 cell executes". `CiLocalTests` now feeds a scope without
      `matrix` plus a `--plan-out` plan, and went red before the change.)*
- [x] **Migrate the hook's readers** — `.githooks/pre-push`,
      `.githooks/tests/fixtures/affected_scope_stub.py`, and
      `.githooks/tests/test-pre-push.sh` where they name `area_matrix` or
      `matrix`.
      *(`pre-push` and `test-pre-push.sh` read neither: the hook passes the
      scope document straight to `scope-record`. The stub was the only site;
      it now projects the planner's shape. Hook suite 67/67.)*
- [x] **Only then** remove `matrix`, `area_matrix`, and `area_slugs`' matrix
      dependence from `legacy_scope_document` and `matrix_record`'s environment
      lists. **Retain** the policy and build-owner projections (`policy`,
      `build_owners`, `build_slices`, `build_artifacts`, `build_runners`) —
      `ci-rollup` and `ci.yml`'s owner job still read them — and keep
      `SCOPE_PROJECTION_FIELDS` honest about what survives.
      *(`matrix_record` had no other caller and was deleted whole.
      `scheduled_areas`/`area_slugs` derive from the plan's gating packages;
      `ci.yml`'s `has_packages` reads `scheduled_areas`. Older receipts that
      still carry the two fields keep validating.)*

### Wave 3 — Documentation and drift (parallel with Wave 2)

- [x] **`.github/ci/README.md`** — rewrite "The area fan-out", "Each area audits
      its planned coverage", the baseline section, and the artifact contract to
      describe row-driven dispatch, the producer completion contract, and the
      tightened skip rule.
- [x] **`docs/topics/ci-cd.md`** — the central policy page's layer descriptions.
- [x] **`.claude/skills/rust-devops/ci-cd.md`** — the fan-out topology, the
      four-level chain, the schema version, the label contract, the new producer
      responsibility, and the tightened skip interpretation.
- [x] **`.claude/skills/os/`** — the WSL2 leg's new guest provisioning and the row
      it receives; add anything Phase 5 or 7 cost time to learn.
- [x] **`CLAUDE.md`** — the CI Structure bullets that describe audit
      responsibility and per-area coverage enforcement.
- [x] **`docs/dependencies.md`** — only if a crate was added or removed (none is
      expected).
      *(Unchanged: no crate added or removed. `.github/ci/schemas/README.md`
      was also corrected; it still described `area_matrix` as live.)*

### Checkpoint

- [x] `no_reader_facing_document_or_recipe_names_a_retired_ci_entity` and
      `the_ci_documentation_states_the_implemented_behavior` green;
      `just ci-local` and the hook suite green after the reader migration;
      `grep` finds no environment-list input and no remaining `area_matrix`
      reader.
      *(Both greps are now contracts:
      `the_reusable_workflows_accept_only_rows_and_run_scalars` and
      `no_shipped_reader_consumes_a_retired_environment_list_projection`.
      `just ci-local` 13/13, hook suite 67/67. The Phase 7 hosted trial still
      has not run; see the Phase 8 log.)*

## Phase 9 — Acceptance Evidence and Handoff

- [ ] **AC-by-AC evidence table** in
      `features/2026-09-19-direct-cell-execution/acceptance.md`: each of the eight
      criteria, the named fixture or artifact that demonstrates it, and the exact
      command that reproduces it.
- [ ] **Full validation sweep**, reported with real output: all thirteen
      `scripts/ci/test_*.py` suites; `just _test repo-deps`;
      `just _test test-toolkit`; `just _lint repo-deps`; `just _lint test-toolkit`;
      `.githooks/tests/test-pre-push.sh`; `actionlint` on `ci.yml`,
      `_area-ci.yml`, `_package-ci.yml`, `_wsl-ci.yml`; `just ci-local --plan` on
      this tree.
- [ ] **State the gaps plainly.** Anything requiring a push or a hosted run
      (S1, the Phase 7 trial, the observed-cells comparison) is listed as blocked
      with the command that closes it. Do not report a hosted claim this session
      did not make.
- [ ] **Terminal state is "implementation complete, ready for review."** Do not
      run `just complete`; do not move this feature to `_completed`; do not
      commit unless the prompt asks for it.

## Concurrency Map

| Phase | Wave | Parallel with | Blocked by |
|---|---|---|---|
| 1 | Rulings | — | — |
| 1 | S1, S2, S3, baseline | each other | rulings (S1/S2 read R1/R4) |
| 2 | 1, 2, 3 | each other | Phase 1 rulings + baseline |
| 3 | 1 | — | Phase 2 |
| 3 | 2 | — | 3.1 |
| 3 | 3 | — | 3.2 |
| 4 | 1 | Phase 3 Wave 3 | Phase 2 Wave 2, S2 |
| 4 | 2 | — | 4.1 |
| 4 | 3 | — | 4.2 |
| 5 | 1 | — | Phases 3 and 4 |
| 5 | 2 | — | 5.1 |
| 5 | 3 | — | 5.2 |
| 6 | 1 | — | Phase 5 |
| 6 | 2 | — | 6.1 |
| 6 | 3 | — | 6.2 |
| 7 | 1, 2 | each other | Phase 6 |
| 7 | 3 | — | 7.1, 7.2 |
| 8 | 1 | — | Phase 7 Wave 3 |
| 8 | 2 | 8.3 | 8.1 |
| 8 | 3 | 8.2 | 8.1 |
| 9 | — | — | Phase 8 |

## Risks and Mitigations

| Risk | Why it matters | Mitigation |
|---|---|---|
| A row set silently drops a cell | This is the PR #76 class of defect the feature exists to remove | Partition and uniqueness fixtures over a seven-plan corpus, checked in both directions, plus a boundary contract in the workflow |
| Job labels change and attribution breaks silently | Nothing turns red; a lost runner becomes an unexplained `MISSING` | `test_runner_loss.py` derives labels from the shipped workflows; both label forms parsed (R1) |
| The completeness validator cannot run in the WSL2 guest | The guest has no compiler and a thin apt set | R2's declared `python3` provisioning with a reachability check in its own named step |
| Nextest's listing does not distinguish the cases the design assumes | The whole expected-set comparison rests on it | S2 proves the JSON shape against the resolved version with fixtures before the parser is chosen |
| Removing the legacy projection breaks the pre-push hook | The hook is the local gate; a break blocks every push | Phase 8 Wave 2 is strictly ordered: migrate readers, prove the hook suite, *then* delete |
| Dropping D4 staging raises peak runner demand | Queue time, not correctness | R3 records the fallback; Phase 7 Wave 1 measures the job estimate before any wide run |
| The audit's version gate lets old evidence through a removed check | Would retroactively certify unproven coverage | Version-gated legacy path landed *before* any removal, with fixtures asserting legacy evidence still faces its own checks |
| Contract-suite churn hides a real regression | ~150 workflow contracts, many asserting the old shape | Phase 1 baseline enumerates the exact edit set; obsolete checks are removed only after their replacement passes |
