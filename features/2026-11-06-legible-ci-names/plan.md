---
kind: plan
total_phases: 3
created: 2026-11-08
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
---

# Legible CI names implementation plan

## Summary and Definition of Done

Improve the four CI workflows' titles, infrastructure jobs, named steps, and published capability-gap checks while preserving execution and evidence behavior. Use the specification's approved automatic-matrix-label fallback: executing package jobs retain their stored package, gate, environment, and runner suffixes. Descriptive surrounding names and documentation explain those unavoidable suffixes. Do not flatten reusable workflows or add a matrix label that a static job name would conceal.

Completion means:

- All four workflows follow the exact naming map below, and their execution commands, job/step identifiers, guards, dependencies, permissions, runner selection, timeouts, and artifact identities are unchanged.
- Executing native and WSL2 names retain package, suite, and execution-environment identity. Distinct L1 and L2 rows remain distinguishable; nested areas retain their repository-relative directories. Skipped names contain no expressions.
- Accepted-gap checks use plain-language work and environment names, remain neutral, and preserve governance and evidence links. Reporting remains visibly advisory. `ci-gate` remains its identifier and exact displayed required context.
- Runner-loss attribution and nightly audit discovery accept both renamed and historical names. No additional display-name reader is introduced.
- Focused contracts and the two owning packages' L1 suites pass. Documentation and an implementation log explain the fallback and distinguish observed names from projected examples. Implementation ends ready for review, without moving the feature directory.

## Phase 1 — Freeze the naming contract

### Necessary Rules

1. **Use the approved fallback; do not add `label`.** `scripts/ci/test_runner_loss.py::job_label` explicitly models a declared `name:` as winning over automatic matrix values. `affected_scope.py::row_sets` emits four values and the live Actions jobs API confirms that all four appear in automatic suffixes. A static replacement would lose the stored identity on which the grandfathered readers depend; a matrix-dependent replacement is prohibited by `every_row_expanding_job_carries_a_scalar_guard_and_no_expression_name`. This is the fallback trigger, rather than permission to restructure the pipeline. Keep automatic names on `ci.yml` jobs `area-ci` and `build`, and `_package-ci.yml` jobs `check`, `test`, `lint`, and `wsl2`. Their identifiers cannot change. Stored gate names and repeated runner values in their suffixes are an expressly accepted limitation of the fallback.
2. **No runtime spike or workflow dispatch is needed.** The existing rendering contract and recorded automatic names settle the feasible fallback. Do not create a disposable workflow, run a test matrix for display exploration, or trigger production CI. Existing-run reads are sufficient. The evidence source read during planning is `GET repos/yankeeinlondon/rusty-biscuit/actions/runs/42491863852/jobs`, including `area-ci (messenger) / package-ci / test (messenger, L1, windows-latest, windows-latest)` and `area-ci (schematic) / package-ci / wsl2 (schematic-gen, L1, wsl2-ubuntu, windows-latest) / test (wsl2-ubuntu)`. It also contains `area-ci (claudine/rendezvous)` prefixes and skipped bare `test`, `check`, and `lint` jobs. Those are actual observations, not promises about new names. A separate pull-request job-list read for run `41407518248` returned HTTP 502 during planning; do not describe it as observed PR evidence.
3. **Use these exact workflow titles and job names.** Paths, identifiers, and all execution properties remain fixed. A dash in the table means retain automatic naming, not a literal dash or an empty name.

   | Workflow | Workflow title | Job identifier → displayed name |
   | --- | --- | --- |
   | `ci.yml` | `Continuous integration` | `validation` → `Check prior pull request validation`; `scope` → `Choose required work`; `preflight` → `Verify build prerequisites`; `build` → —; `area-ci` → —; `area-drift` → `Verify package area mapping`; `ci-gate` → `ci-gate`; `certify-nightly` → `Publish nightly coverage evidence`; `publish-observations` → `Publish test observations`; `ci-reporting` → `Report CI results (advisory)` |
   | `_area-ci.yml` | `Package area checks` | `package-ci` → `Run package work`; `accepted-gaps` → `Publish accepted capability gaps`; `coverage-audit` → `Check required results` |
   | `_package-ci.yml` | `Package checks` | `check`, `test`, `lint`, `wsl2` → — |
   | `_wsl-ci.yml` | `WSL2 tests` | `wsl` → `Run guest tests` |

   All declared job names are literal strings. No workflow-input expressions are introduced. The area remains visible through the existing automatic `area-ci (<directory>)` caller prefix. The build owner's automatic `build (<producer>)` name remains necessary for `runner_loss.py::parse_build_job_name`; document it as archive preparation, including Linux archives intended for WSL2. Infrastructure names do not need a matrix suffix to identify a result cell because they own none.
4. **Keep row schemas unchanged.** `schema.py::ROW_FIELDS`, `schema.py::_rows`, `.github/ci/schemas/contract.json`'s row field list, and `test_resolved_plan.py::RowAdapterOracleTests.ROW_KEYS` currently require precisely `package, gate, environment, runner`. The fallback changes none of them and needs no schema-version bump. `affected_scope.py::row_sets`, output budgets, selection, and carried-receipt reprojection remain untouched. No new configuration reader or field validator is added, so the Input Robustness Matrix does not apply. Display-name string parsing is not a file-format loader.
5. **Extend only the two permitted job-name readers.** In `runner_loss.py::parse_job_name`, recognize the exact terminal segment `Run guest tests`, then recover the complete cell exclusively from the immediately enclosing `wsl2 (<package>, <gate>, <environment>, <runner>)` segment using existing `WSL_DELEGATION_ROW` and `row_cell`. A missing or malformed enclosing row returns `None`; never infer package or gate from prose. Retain the existing parser branch, regexes, lint environment normalization, historical pre-row names, and four-token names. Add `Report CI results (advisory)` to `NON_PRODUCER_JOBS` without removing any historical spelling. Keep `parse_build_job_name` unchanged because build names are unchanged. `Check required results` remains an independent failure that can veto a runner-loss retry, not a non-producer exemption. Extend `reuse_validation.py::AUDIT_JOB` to `r"(^|/ )(?:coverage-audit|Check required results)$"`; both consumers at nightly validation and coverage certification continue using that one constant. No other display-name parsing is authorized.
6. **Use one exact accepted-gap name template.** In `publish_gaps.py::check_run`, emit `ACCEPTED GAP: <package>: <action> on <environment>`. Define local constant maps for all five gates and four environments using the specification's wording: `L1` → `run Level 1 tests`, `L2` → `run Level 2 tests`, `browser` → `run browser tests`, `lint` → `lint source`, `check` → `check compilation`; `ubuntu-latest` → `Linux`, `macos-latest` → `macOS`, `windows-latest` → `Windows`, `wsl2-ubuntu` → `WSL2`. Unknown values raise the existing `Refusal` with `cell_label(cell)` and the unknown value, before publication. Retain the existing name-length refusal. Keep area information in the existing output summary rather than the check name. Preserve `conclusion`, head revision, details URL, owner, expiry, reason, revocation guidance, and publication selection. No shared abstraction is necessary for mappings consumed by this one publisher.
7. **Rename only steps whose wording needs work.** Retain existing concise verb/object names. Apply these explicit replacements and update all assertions and fixtures that name them:

   | Workflow | Existing step name | Replacement |
   | --- | --- | --- |
   | `_package-ci.yml` | `cargo check (declared kinds no test gate compiles)` | `Check compilation of declared targets` |
   | `_package-ci.yml` | `Tests` | `Run planned tests` |
   | `_package-ci.yml` | `Companion suites` | `Run companion test suites` |
   | `_package-ci.yml` | `Lint` | `Lint source` |
   | `_package-ci.yml` | `Companion suites (lint)` | `Run companion lint suites` |
   | `_package-ci.yml`, `_wsl-ci.yml` | `Certify this cell's completeness` | `Verify all expected results reported` |
   | `_wsl-ci.yml` | `L1 tests from the archive` | `Run Level 1 tests from the archive` |
   | `_wsl-ci.yml` | `Upload wsl2-ubuntu L1 JUnit` | `Upload WSL2 Level 1 test results` |
   | `_wsl-ci.yml` | `Guest post-mortem` | `Collect guest diagnostics` |
   | `_wsl-ci.yml` | `Host post-mortem` | `Collect host diagnostics` |
   | `_area-ci.yml` | `Enforce coverage completeness and exceptions` | `Verify required results and policy exceptions` |

   Keep `Compile unchanged dependents` verbatim: `ci_workflow_contracts.rs` pins it and the wording accurately describes the broad compilation check. Keep compiler measurements and diagnostics recognizable. `runner_loss.py::STAGE_MARKERS` is an existing step-name classifier; preserve its ordering and ensure replacements retain their prior classifications. Do not change stage policy to make new words fit.
8. **Preserve truthful reporting rather than rewriting policy.** Existing area rollups and the run report already own no-selection, reused evidence, deferred environments, accepted gaps, and missing/failed/canceled results (`docs/topics/ci-cd.md`, coverage and governance sections). Keep those commands, data, and conclusions intact; test their preservation through existing contracts. Documentation explains that a skipped placeholder proves nothing. Define Level 1 as in-process or hermetic subprocess/filesystem tests, Level 2 as tests requiring a real terminal/PTY, browser as headless-browser tests, and WSL2 as Windows Subsystem for Linux 2, following the `rust-testing` skill's tier definitions.
9. **Keep changes in the session's worktree.** Do not create branches or worktrees, commit, push, modify rulesets, add jobs, change other workflows, or run `cargo fmt`. Use repository testing recipes and focused Python companion tests. No dependencies are added. Update nearby comments that describe rendered names, especially the stale pre-row example and claim of no display-name parsing at the top of `_area-ci.yml`; the code is authoritative and the two exceptions are documented.

### Wave 1 — Contract baseline

- [ ] **Capture naming evidence**
  - Create `implementation.md` beside this plan. Record the fallback decision, the source symbols and observed names in Rules 1–2, the exact templates, and the limitation that automatic suffixes retain stored wording and duplicate runners. Label new-name examples as projected until corroborated by run records.
  - Read existing Actions and PR check-run records through `gh api` only; use the known PR head `5a2760470df3cec36fe8668859efd5549ca4dd12` with the commit check-runs endpoint to capture available historical check names. An unavailable endpoint is recorded with its error, without a retry loop or new run. API evidence is supplementary; the selected fallback and subsequent phases do not depend on network availability.
  - Record complete native, WSL2-shaped, two-level, nested-area, and empty-selection examples. Use fixture examples for cases absent from retrieved records, explicitly labeled as such. Do not fabricate observed names.
- [ ] **Freeze safety assertions**
  - In `tools/test-toolkit/tests/ci_workflow_contracts.rs`, extend the existing contracts with the naming map and invariants: the ten top-level identifiers, literal static names, retained automatic producer/area/build names, the exact `ci-gate` context, and advisory reporting.
  - Keep a baseline of non-presentation workflow structure for comparison: commands and `with` values (including artifact `name` fields), identifiers, guards, `needs`, matrices, permissions, runner selection, and timeouts. Treat only top-level workflow titles, job display names, step display names, and adjacent comments as presentation. Do not erase every YAML `name` field during comparison.
  - This task and evidence capture can run concurrently: they own separate files. New assertions may be red before Phase 2; identify those expected failures explicitly.

Checkpoint: the fallback, all replacement strings, parser behavior, and protected execution properties are written down. No unresolved display experiment remains.

## Phase 2 — Apply names and compatibility

### Wave 2 — Independent implementation slices

- [ ] **Rename workflow surfaces**
  - Apply Rules 3 and 7 in the four scoped workflow files. Keep all unnamed execution matrices unnamed. Update workflow comments describing their complete caller chains and the grandfathered readers.
  - In the same slice, implement Rule 5 in `runner_loss.py` and `reuse_validation.py`, with tests in `test_runner_loss.py` and `test_reuse_validation.py`. The workflow and reader edits are one task because they must agree on exact names.
  - Update `test_runner_loss.py::job_label` and its producer walk only for the new static leaf/caller names; keep its automatic four-value rendering and non-vacuity checks. Prove every status-uploading producer still maps to the status it uploads, with separate native Windows and WSL2 identities.
  - Test the new guest leaf with L1 and L2 delegating rows, malformed/missing rows returning `None`, old native and WSL2 names, old pre-row names, build attribution, all historical advisory names, and renamed coverage audits vetoing retry. Test historical and renamed audits through nightly reuse/certification public helpers, not solely regex matches.
  - Preserve step-classification outcomes for every renamed step through `failure_stage`. Existing archive-first and upload-first precedence remains authoritative.
- [ ] **Rename gap checks**
  - Implement Rule 6 in `scripts/ci/publish_gaps.py`; update `scripts/ci/test_publish_gaps.py` payload and CLI expectations together.
  - Exercise all five action mappings and four environment mappings through `payloads`/`check_run` results. Retain positive governance fixtures and the existing refusal tests. Add unknown-gate and unknown-environment refusal controls, length-limit coverage for the longer names, and assertions that a refused batch posts nothing.
  - Assert all payload fields other than check `name` remain equal for the same governed fixture. Publishing a gap must still be distinct from executing tests.
  - This task can run concurrently with workflow/reader work: it owns only the publisher and its tests.

### Wave 3 — Integrate contracts

- [ ] **Align contract fixtures**
  - After both Wave 2 tasks, update the Phase 1 contracts and existing step-name assertions in `ci_workflow_contracts.rs`, including `GATE_COMMAND_STEPS`, the WSL archive ordering assertion, and the tests comparing positions of `Tests` and toolchain/build verification. Search all four workflow names and old step strings across `scripts/ci/test_*.py` and `tools/test-toolkit/tests/ci_workflow_contracts.rs` to update executable expectations. Preserve historical parser fixtures intentionally.
  - Add complete projected examples: `area-ci (research) / Run package work / test (research, L1, ubuntu-latest, ubuntu-latest)`; the same package with `L2`; `area-ci (schematic) / Run package work / wsl2 (schematic-gen, L1, wsl2-ubuntu, windows-latest) / Run guest tests`; and `area-ci (claudine/rendezvous) / Check required results`.
  - Cover empty-selection/skipped names as static names or bare automatic identifiers without `${{ ... }}`. Retain `RowAdapterOracleTests` and row-budget assertions unchanged to prove the fallback added no matrix field.
  - Compare the protected workflow properties against the Phase 1 baseline. Resolve any non-presentation difference by restoring the baseline behavior.

Checkpoint: focused naming, attribution, audit-discovery, and publisher tests pass; execution structure and stored identities remain unchanged.

## Phase 3 — Document and verify

### Wave 4 — Documentation and final checks

- [ ] **Document visible behavior**
  - Update `docs/topics/ci-cd.md` with the exact naming convention, action/environment table, tier definitions, full examples from Wave 3, and the fallback's unavoidable caller prefixes and stored suffix wording. Explain archive preparation versus WSL2 execution.
  - Explain each skipped state with a compact example: nothing selected, evidence reused with its link, environment not scheduled for this event, neutral governed `ACCEPTED GAP`, and blocking missing/failed/canceled work. Include a small Mermaid flow showing selection → execution/reuse/defer/gap → coverage audit → `ci-gate`, with deferred environments outside required cells and failed/missing required results blocking.
  - Correct the existing runner-loss/name-reader explanation to describe the two grandfathered readers and static guest-leaf lookup. Do not imply display names are a general identity API. Do not link or name a dated feature in the topic page.
  - Update `.claude/skills/rust-devops/ci-cd.md` where its explanation says these jobs have no names: distinguish retained automatic package matrices from newly static infrastructure names. Preserve the prohibition on expression-based skipped names. No README update is required because this changes internal pipeline presentation, not package/CLI behavior.
- [ ] **Verify preservation**
  - Run the focused companion suites from repository root: `python3 scripts/ci/test_runner_loss.py`, `python3 scripts/ci/test_reuse_validation.py`, `python3 scripts/ci/test_publish_gaps.py`, `python3 scripts/ci/test_resolved_plan.py`, `python3 scripts/ci/test_schema.py`, and `python3 scripts/ci/test_ci_local.py`. Run independent commands concurrently; inspect every result.
  - Run the owning L1 suites through `just test repo-deps test-toolkit`. These use nextest; do not substitute `cargo test`. Do not run L2, browser, Windows/WSL2 runtime suites, or a full-workspace CI run merely to validate presentation. The WSL2-shaped name fixtures exercise nesting without launching a guest.
  - Recheck the protected workflow properties and the exact required context `ci-gate`. Search for newly added name readers and matrix expressions in job names. Confirm `schema.ROW_FIELDS`, schema versions, stored artifact names, and all matrices are unchanged.
  - Finish `implementation.md` with test outcomes, observed versus projected name lists, the fallback departure from the preferred label design, preserved execution properties, and any unavailable read-only API evidence. This is a review artifact, not an intermediate approval gate. Fix failing implementation checks within this phase before completion.
  - Documentation and verification can run concurrently because they own separate files; finish by reading the updated documentation against the verified behavior. Leave the feature active and ready for review.

Final checkpoint: all three phases complete without a human gate, all focused checks pass, and the implementation log candidly records the approved fallback and the limits of remotely observed display evidence.
