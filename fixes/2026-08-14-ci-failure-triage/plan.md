---
total_phases: 5
created: 2026-08-14
phase: 5
agent: codex/default
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - fixes/2026-08-14-ci-failure-triage/plan.md
  - fixes/2026-08-14-ci-failure-triage/spec.md
  - fixes/2026-08-13-finalize/failing.md
  - fixes/2026-08-13-finalize/spec.md
  - fixes/2026-08-13-finalize/plan.md
docs_created_during_phase_1:
  - fixes/2026-08-14-ci-failure-triage/evidence.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2: []
docs_updated_during_phase_2:
  - fixes/2026-08-14-ci-failure-triage/evidence.md
  - fixes/2026-08-14-ci-failure-triage/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - fixes/2026-08-14-ci-failure-triage/evidence.md
  - fixes/2026-08-14-ci-failure-triage/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/tests/ctx_launch_anchor_baseline.rs
  - claudine/cli/tests/shipped_prompt_contract.rs
docs_updated_during_phase_4:
  - fixes/2026-08-13-finalize/ci-baseline-evidence.md
  - fixes/2026-08-13-finalize/problems.md
  - fixes/2026-08-14-ci-failure-triage/evidence.md
  - fixes/2026-08-14-ci-failure-triage/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - fixes/2026-08-14-ci-failure-triage/evidence.md
  - fixes/2026-08-14-ci-failure-triage/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_code:
  - claudine/cli/tests/ctx_launch_anchor_baseline.rs
  - claudine/cli/tests/shipped_prompt_contract.rs
documentation:
  - fixes/2026-08-13-finalize/ci-baseline-evidence.md
  - fixes/2026-08-13-finalize/failing.md
  - fixes/2026-08-13-finalize/plan.md
  - fixes/2026-08-13-finalize/problems.md
  - fixes/2026-08-13-finalize/spec.md
  - fixes/2026-08-14-ci-failure-triage/evidence.md
  - fixes/2026-08-14-ci-failure-triage/plan.md
  - fixes/2026-08-14-ci-failure-triage/spec.md
packages: []
---

# Execution plan: Attribute and clear CI run 31753281913

Reference: [`spec.md`](spec.md)

## Goal

Produce a complete, identity-aware disposition for every failed producer cell
in CI run `31753281913`, repair only failures caused by
`fix/ctx-launch-anchor`, preserve the canonical WSL2 Level-1 contract, and
apply CI baselines only where current branch-versus-main evidence proves that
no added branch failure is being hidden.

## Completion contract

The work is complete when the final failed-cell catalog reconciles with all
completed run artifacts, every Windows Level-1 and macOS/Linux Level-2 failure
has matched-environment attribution, every branch-caused identity is green,
the WSL2 failures have accurate fixture-level explanations and main-side
handoffs, the parent fix records its Level-2 verification gap and chosen gate,
and one authoritative CI run passes `ci-verdict` without accepting a branch
superset, stale baseline, or unexamined zero-identity diagnostic change.

## Governing decisions and constraints

- Phase 1 will ratify the specification's recommended Open Question 1 Option
  A: canonical Level-2 coverage for package areas whose rendered behavior or
  rendering inputs changed. If review selects Option B or C, update both this
  specification and the parent fix's verification matrix before dependent
  validation begins.
- Treat run `31753281913`'s checked-in count of 601 jobs, 24 failures, and 51
  identities as provisional. Count failed producer jobs separately from the
  expected downstream `ci-verdict` failure.
- Compare branch and `main` on the same operating system, host class, terminal
  backend, dimensions, environment, feature set, filters, and fixture binaries.
  A result from a different host class is context, not attribution.
- Make no baseline edit during Phases 1 or 2. Phase 3 must clear every
  branch-owned regression before Phase 5 can apply or retain a cell-wide
  baseline.
- Use nextest through the canonical `just` recipes; do not use `cargo test` and
  do not run `cargo fmt`. Level-2 tests must use the repository harness and
  must not open or focus a host terminal or browser window.
- Preserve `.github/workflows/_wsl-ci.yml` as a full package Level-1 run from a
  Linux-built nextest archive. Do not narrow a WSL2 cell, install a guest Rust
  toolchain, add a generic `shell` environment capability, or turn an ordinary
  Rust test's early return into a nominal skip.
- Keep main-drift product fixes outside this branch. This plan may correct
  evidence, baseline reasons, and handoffs for those packages, but must not
  absorb their implementation.

## Dependency and parallelization map

```text
Phase 1: completed-run inventory and decision gate
    |
    +-- Phase 2A: Windows L1 attribution ---------+
    +-- Phase 2B: macOS L2 attribution -----------+-- Phase 3: conditional fixes
    +-- Phase 2C: Linux L2 attribution -----------+           |
                                                             +-- Phase 4: WSL2 proof and handoffs
                                                                         |
                                                                         +-- Phase 5: baseline and final CI
```

The three Phase 2 host tracks are parallelizable after Phase 1. Independent
branch-owned fixes in Phase 3 are parallelizable after their individual
attribution records are complete. Phase 5 is strictly blocked on both the
branch-fix checkpoint and the WSL2 no-superset checkpoint.

## Phase 1 — Establish evidence integrity

**Objective:** replace the provisional ledger with a reproducible inventory of
all completed failed producer jobs before making attribution or policy changes.

- [x] **Task 1.1 — Freeze the comparison inputs.** Record the workflow name,
  run ID, branch, head SHA, creation/completion time, and artifact availability
  for branch run `31753281913` and the comparable completed `main` run or runs.
  Download `ci-results`, all JUnit/manifest/status artifacts for failed
  producers, and logs for zero-identity producers into a temporary evidence
  workspace; retain source links and checksums or artifact IDs in
  `evidence.md` rather than committing downloaded artifacts.

- [x] **Task 1.2 — Reconcile jobs before tests.** Query the completed job
  inventory for run `31753281913`, identify every failed producer by
  `{package, environment, tier}` and job ID, and list `ci-verdict` separately.
  Account explicitly for the 603 completed jobs reported after the provisional
  catalog was captured and recover the omitted `claudine-cli/wsl2-ubuntu`
  producer.

- [x] **Task 1.3 — Rebuild the identity ledger.** For each JUnit-backed failed
  producer, derive the complete failing identity set from `manifest.jsonl` and
  its staged XML. For lint or any other zero-identity producer, normalize the
  exact diagnostic set by rule/message and source location from the job log.
  Record test/cell totals and surface malformed, missing, or truncated evidence
  as an unresolved block instead of inferring an empty set.

- [x] **Task 1.4 — Annotate policy state and current comparison.** For every
  failed producer, record whether its key was already present in
  `.github/ci/ci-baseline.toml` at `a00ea7c08`, compare the branch evidence with
  the matched `main` cell as equal, subset, superset, or not comparable, and
  link the attribution record or existing main-side handoff. Use normalized
  log diagnostics, not empty `failed_tests`, for lint comparisons.

- [x] **Task 1.5 — Correct the authoritative catalog.** Refresh
  [`../2026-08-13-finalize/failing.md`](../2026-08-13-finalize/failing.md) with
  the completed producer count, failed-cell count, final identity count, job
  IDs, exact identity/diagnostic sets, baseline state, comparison relation,
  and evidence links. Correct the Darkmatter WSL2 ownership/history claims and
  preserve every newly discovered identity even if the final total differs
  from 51.

- [x] **Task 1.6 — Ratify the Level-2 gate.** Resolve Open Question 1 in
  [`spec.md`](spec.md), selecting recommended Option A unless review explicitly
  chooses otherwise. Correct the parent fix's verification record to say that
  its pre-CI local runs were Level 1 only, and add the chosen Level-2 gate to
  the parent verification matrix before Phase 3 acceptance is evaluated.

- [x] **Validation checkpoint 1 — Prove the ledger closes.** Independently
  total completed jobs, failed producers, JUnit identities, and zero-identity
  diagnostics from the downloaded artifacts and verify those totals match the
  refreshed catalog. Require every failed producer to have one and only one
  disposition row, with no classification based solely on package ownership or
  subsystem scope.

## Phase 2 — Attribute Windows and Level-2 failures

**Objective:** obtain matched-host branch-versus-main evidence for every
currently unattributed identity without changing code or baseline policy.

- [ ] **Task 2.1 — Reproduce the Windows identity on the branch.** On the same
  native Windows host used for the recorded 655-test result, run
  `cargo nextest run -p darkmatter-cli --test schema_validate_baseline --color never`
  from a clean checkout of `a00ea7c08` or the current branch revision. Prove
  from nextest output that
  `schema_validate_legacy_pretty_output_is_byte_identical` executed, and record
  the nextest profile/filter, feature flags, `NO_COLOR`, terminal/hyperlink
  state, working-directory spelling, fixture inputs, and actual-versus-expected
  bytes for every differing case.

- [ ] **Task 2.2 — Run the matched Windows control (parallelizable with the L2
  tracks).** On that same Windows host and with identical command and
  environment inputs, run the focused test from the comparable `main` revision.
  Compare both native results with the `windows-latest` job evidence and assign
  a byte-level cause: branch regression, main drift, or a documented native
  Windows versus GitHub-runner environmental difference. Do not use aggregate
  package counts as proof that the identity ran.

- [x] **Task 2.3 — Run the macOS Level-2 branch suites (parallelizable).** From
  the Claudine, Darkmatter, and Biscuit Terminal package areas on the same
  macOS host/backend used for comparison, run
  `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2 --no-fail-fast`. Capture the
  backend, pane dimensions, color/background variables, fixture executable
  paths and versions, rendered bytes or screen frames, and the disposition
  delta for every spec-listed failure plus any Phase 1 discovery.

- [x] **Task 2.4 — Run the macOS Level-2 main controls.** Repeat the three
  canonical suites from a clean checkout of the selected `main` revision on
  the same macOS host with the recorded inputs held constant. Classify each red
  identity as branch regression, main drift, harness/environment defect, or
  candidate flake and link its exact frame/byte comparison in `evidence.md`.

- [x] **Task 2.5 — Run the Linux Level-2 branch suites (parallelizable).** On
  the matched Linux host/backend, run
  `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2 --no-fail-fast` in the Claudine,
  Darkmatter, and Biscuit Terminal package areas. Capture the same environment,
  fixture, byte, and screen-state fields as the macOS track, including all four
  known `claudine-cli` context identities and the two known Ubuntu rendering
  identities.

- [x] **Task 2.6 — Run the Linux Level-2 main controls.** Repeat the three
  canonical suites from the selected `main` revision on the same Linux host
  and classify every red identity from matched evidence. Do not substitute a
  green expectation from macOS or another Linux machine.

- [x] **Task 2.7 — Prove any flake classification.** For an identity proposed
  as flaky, state the suspected trigger and run at least three focused repeats
  under both the triggering and controlled conditions on the same revision.
  Record the pass/fail sequence and observed state transition; otherwise
  classify the identity as unresolved rather than flaky.

- [ ] **Validation checkpoint 2 — Close attribution before edits.** Require a
  byte- or screen-level explanation for the Windows identity and a matched-host
  disposition for every macOS/Linux Level-2 identity. Verify that neither code
  nor `.github/ci/ci-baseline.toml` changed during this phase.

## Phase 3 — Repair branch-owned regressions

**Objective:** make only the failures attributed to this branch green, with
narrow regression coverage and no weakening of terminal contracts.

- [x] **Task 3.1 — Freeze the implementation scope.** Convert Phase 2's branch
  regression rows into an explicit file/test worklist. If no identity is
  branch-caused, record that evidence-backed outcome and skip directly to the
  Phase 3 validation checkpoint; do not manufacture cleanup work.

- [x] **Task 3.2 — Repair the Windows cause when branch-owned
  (parallelizable).** Change the owning rendering or path-projection layer,
  update behavior comments/docs in the touched surface, and add the narrowest
  OS-independent test that asserts the explained byte delta. If expected
  output legitimately changed, re-derive it through
  `schema_validate_baseline`'s documented review workflow and review the
  semantic delta before changing fixtures; never bless current CI output merely
  because it makes the test pass.

- [x] **Task 3.3 — Repair each branch-owned Level-2 cause
  (parallelizable).** Fix the production or harness defect identified by the
  matched comparison while retaining the original terminal assertion. Add a
  deterministic lower-level regression where possible and keep package-area
  boundaries surgical; do not absorb main-red fixes for unrelated identities.

- [x] **Task 3.4 — Run focused regression tests.** Re-run each corrected
  identity on its failing operating system and host class, then on every other
  supported local host where the test is applicable. Require the Windows
  baseline to be byte-identical for the reviewed reason and the Level-2 frame
  assertions to pass without host-window focus.

- [x] **Task 3.5 — Run affected package gates.** For every source package
  changed in Tasks 3.2-3.3, run `just test` and `just lint` in its package area.
  Run `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2 --no-fail-fast` for Claudine
  and Darkmatter under ratified Option A, plus Biscuit Terminal if its code or
  harness changed. Perform the applicable Windows Level-1 run for
  `darkmatter-cli` and preserve macOS, Windows, and Linux compatibility in all
  new fixtures and path handling.

- [x] **Validation checkpoint 3 — Clear branch ownership.** Confirm every
  branch-caused identity is green through its canonical recipe on the original
  failing environment, all affected Level-1/lint gates pass, and no terminal
  assertion, global timeout, or environment contract was weakened. Phase 5
  remains blocked while any branch-owned row is red or unresolved.

## Phase 4 — Preserve WSL2 and record main-side handoffs

**Objective:** prove the branch adds no WSL2 failure, correct the environmental
attribution, and leave actionable fixture work for `main` without redesigning
the WSL2 tier.

- [x] **Task 4.1 — Compare canonical WSL2 cells.** Using artifacts produced by
  `.github/workflows/_wsl-ci.yml`, compare current branch and matched `main`
  Level-1 identity sets for `darkmatter`, `claudine`, and `claudine-cli` in the
  toolchain-free `wsl2-ubuntu` guest. Include the omitted Claudine CLI producer
  and reject evidence from a toolchain-equipped development WSL2 host as a
  substitute.

- [x] **Task 4.2 — Separate the Darkmatter fixture classes.** Record that the
  three parser-only `no_cache` tests consult `PATH` because their lone bare
  `rustc` token is ambiguous, while the interpolation and two
  coordinate/diagnostic tests actually execute `rustc`. Verify all six
  identities predate or reproduce on `main` and that none is newly added by the
  branch.

- [x] **Task 4.3 — Record durable fixture handoffs.** Update the applicable
  main-side handoff document so parser tests use an unambiguous path-bearing
  dummy command and runtime tests use a repository-owned, cross-platform
  fixture executable carried in the nextest archive, or an equally hermetic
  mechanism that preserves subprocess-result coverage. Record the equivalent
  rustc-probe handoff for the three Claudine identities and the recovered
  Claudine CLI set.

- [x] **Task 4.4 — Prepare accurate baseline reasons.** Draft corrections for
  the existing Darkmatter WSL2 entry and the parent-ratified Claudine entry so
  each names the concrete missing-toolchain/fixture assumption and links the
  fresh source run. Do not change existing expiration dates, add a generic
  environment capability, or apply any new entry before Phase 5's gate.

- [x] **Task 4.5 — Audit the WSL2 contract.** Verify the implementation diff
  does not narrow or remove a WSL2 package cell, install Cargo/rustc in the
  guest, silently return early from tests, or modify
  `.github/ci/environments.json` to govern an executable prerequisite.

- [ ] **Validation checkpoint 4 — Prove no WSL2 superset.** Require exact
  JUnit evidence that each of the three branch WSL2 cell identity sets is equal
  to or a subset of its comparable `main` cell. Keep Phase 5 blocked for a
  missing, malformed, or branch-superset result.

## Phase 5 — Apply gate policy and run authoritative acceptance

**Objective:** use current evidence to apply only justified baseline policy,
then prove the complete branch through one authoritative CI run and an
identity-aware review.

- [ ] **Task 5.1 — Satisfy the baseline entry gate.** Confirm Validation
  Checkpoints 3 and 4 are complete and select the newest completed branch run
  at or after `a00ea7c08` that contains all Phase 3 changes, together with a
  comparable current `main` run for every environment. Do not reuse branch run
  `31651014023` or apply policy against uncommitted/undispatched fixes.

- [ ] **Task 5.2 — Re-run comparison across all red cells.** Download both
  `ci-results` artifacts and run the same `ci-rollup compare` path used by
  `just ci-diff`. Review every failed cell, including cells already baselined,
  against exact JUnit identities; separately compare normalized raw diagnostics
  for lint and other zero-identity cells. Reject every branch superset and
  every non-comparable cell.

- [ ] **Task 5.3 — Apply only evidence-backed baseline edits.** Add the parent
  spec's ratified main-drift entries only when Task 5.2 proves equality or
  subset status. Give each new entry the ratified owner, an exact current
  `source_run`, a reason naming the observed failure/assumption, and
  `expiry = "2026-09-30"`. Apply the WSL2 reason corrections from Task 4.4,
  avoid duplicate keys, do not extend an existing expiration, and remove rather
  than accept an entry whose cell now passes.

- [ ] **Task 5.4 — Validate the policy locally against downloaded results.**
  Build `ci-rollup` with the same no-default-feature command used by
  `just ci-diff`, run `ci-rollup verdict` against the edited baseline, and
  inspect every finding. Require no `baseline-no-result`,
  `baseline-now-passing`, expired, missing, cancelled, policy-gap, or
  unapproved failure finding.

- [ ] **Task 5.5 — Run the authoritative full CI workflow.** Dispatch one full
  CI run containing all fixes, evidence-driven policy edits, and documentation
  corrections. Require every branch-owned Windows Level-1 and macOS/Linux
  Level-2 identity to pass, all three WSL2 cells to produce complete evidence,
  and the single required `ci-verdict` check to succeed.

- [ ] **Task 5.6 — Perform the final identity and diagnostic review.** Run
  `just ci-diff` against the authoritative branch run and current `main`; exit
  status 0 must be accompanied by manual review of existing and newly added
  baseline cells. Re-download logs for every accepted zero-identity producer
  and require its normalized diagnostics to be equal to or a subset of
  `main`, since `ci-diff` cannot prove that property from an empty identity set.

- [ ] **Task 5.7 — Close documentation and scope.** Record final run IDs,
  commits, job/cell/identity totals, targeted test commands/results, Level-2
  gate decision, Windows byte explanation, WSL2 comparisons, baseline changes,
  and `ci-diff` outcome in `evidence.md` and the parent handoff. Review touched
  `///`, `//!`, and inline comments for drift and confirm no out-of-scope
  main-drift product fix or WSL2 redesign entered the final diff.

- [ ] **Validation checkpoint 5 — Final acceptance.** Verify all seven success
  criteria in [`spec.md`](spec.md): complete producer attribution, no remaining
  branch-caused red identity, a byte-level Windows explanation, concrete WSL2
  assumptions, intact full-Level-1 WSL2 coverage, corrected parent Level-2
  verification, and no baseline accepting a branch-superset identity or
  diagnostic set.
