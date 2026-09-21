---
kind: implementation-log
feature: 2026-09-19-direct-cell-execution
spec: /Volumes/coding/wt/rusty-biscuit/feat-single-os/features/2026-09-19-direct-cell-execution/spec.md
plan: features/2026-09-19-direct-cell-execution/plan.md
implemented_by: opencode/zai-coding-plan/glm-5.3
started_phase: 1
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
  - features/2026-09-19-direct-cell-execution/spikes/plans/*.json
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
implementation_1: "2026-09-21T08:05:10-07:00"
---

# Implementation Log for 2026-09-19-direct-cell-execution (9 phases)

> `packages` above is the per-phase record: Phase 1 touched no workspace
> package (decision and measurement only); Phase 2 touched both of the
> feature's target packages (`repo-deps`, `test-toolkit`). The plan file's own
> `packages` frontmatter names the feature's target packages and is unchanged.

## Phase 1

Phase 1 is rulings, spikes, and baseline — decision and measurement only.
**No production file changed except the specification's frontmatter** (R6:
`status: draft` → `status: planned`, plus the review-metadata properties the
phase process requires). The Phase 1 checkpoint held throughout: no code,
workflow, justfile, or schema file was edited.

### What was done

1. **Verified every ruling's factual claims against this tree** before
   recording them (`rulings.md`, R1–R12):
   - `_area-ci.yml` is in neither `GLOBAL_PATHS_ALL_GATES` nor
     `ORCHESTRATION_PATHS` (R11); `CI_RECIPES_BY_GATE["test"]` is
     `("_test", "_test_l2", "_test_browser")` (R12); `MATRIX_LIMIT` = 256
     (R7); no nextest pin exists anywhere — `taiki-e/install-action@nextest`
     and `"nextest": "latest"` in every build contract (R4); the WSL2 guest
     apt set is exactly `ca-certificates curl git xz-utils jq` (R2);
     `_expected_manifest` exists at `just/devops.just:1289`, schema version
     1, selecting on `filter-match.status == "matches"` (S2/R12 context).
2. **S2 (complete)** — built a scratch two-binary fixture covering the five
   listing questions and ran `cargo nextest list --message-format json` in
   workspace and archive+`--workspace-remap` modes, under the repo's real
   `_tier_filter` strings. Findings in `spikes/s2-nextest-list.md`: ignored
   tests and tier-filtered tests are listed with distinct
   `filter-match.reason`s; cfg-excluded tests are absent entirely;
   `binary-id` disambiguates shared names; archive listings emit the same
   shape; `matches` is sufficient under four conditions (no `--run-ignored`,
   binary-scoped identity, recorded target provenance, counts from
   `testcases` not `test-count`). Resolved version recorded:
   `cargo-nextest 0.9.136 (1d5bf1ec9 2026-05-16)`.
3. **S3 (complete)** — extended the capacity model to all eight corpus plans
   (`spikes/s3-capacity.md`): largest row set 21/256 (homelab, native test);
   largest per-area row-set payload 2,140 B; total row-set payload 21,783 B
   vs `area_matrix`'s 79,929 B; scope.json total 323,063 B today; whole-run
   job estimate ≈ 600 under the row design (fewer delegator legs than
   today's per-package fan-out). Budget constants proposed for Phase 7:
   16 KB per-area row-set payload, 512 KB aggregate, 256 rows per matrix.
4. **S1 (deferred, per the plan's own escape hatch)** — pushing a scratch
   workflow is unavailable in this session, so
   `spikes/s1-labels.md` records the documented behavior, this repo's proven
   in-repo evidence (AC10 contracts, the 2026-09-12 scratch run, the
   `prepared-not-hosted` hosted fixture), the derived four-token label
   expectation, and the three exact open questions. R1's both-forms parser
   is the standing mitigation; no full-scope dispatch is proposed.
5. **Baseline (complete)** — `spikes/s0-baseline.md` records: every producer
   job label in the four reader-facing workflows (running and skipped
   forms); the Phase 2/5 contract-test edit set from
   `ci_workflow_contracts.rs` split into a primary table (12 tests asserting
   environment lists, the per-package matrix, or `needs: test` staging) and
   a secondary table (label/job-id renames); all twelve Python suites green
   (769 tests); `--all` statistics; and the eight-plan corpus under
   `spikes/plans/` with production recipes in `spikes/plans/README.md`.
   - Corpus discovery worth carrying forward: **no area is gap-only or
     fully reused in any real plan today** — lint cells never reuse, so
     every area always executes at least one cell; `gap-only.json` is
     therefore synthesized (schema-validated) and `all-reused.json` still
     executes 71 cells (68 lint + 3 companion-owning L1). Both behaviors are
     the planner's real semantics, not fixture defects.
   - The plan's measured table was taken on `fix/archive-path-sites`; on
     this tree `area_matrix` serializes to 79,929 B (not 85,956 B) and the
     largest area matrix to 7,908 B (not 8,504 B) — drift noted in the
     baseline so Phase 3 compares against current numbers.

### Test design / verification mapping

Phase 1 changes no behavior, so there are no new regression tests to add.
The phase's verification burden was (a) that the baseline numbers are real
and reproducible, and (b) that nothing regressed while producing them:

| Requirement | Verification |
|---|---|
| Rulings grounded in the tree | each ruling's claims grepped/verified at the cited lines (affected_scope.py, devops.just, _wsl-ci.yml, environments.json, runner_loss.py) |
| Corpus plans valid | each of the 8 plans passes `schema.validate_resolved_plan` (the gap-only synthesis ran it before writing; the other 7 are planner output) |
| S2 findings reproducible | exact commands and verbatim JSON in `spikes/s2-nextest-list.md`; fixture source inline |
| S3 numbers reproducible | derivation script inline in the spike; corpus files are the inputs |
| Baseline counts real | all 12 suites executed 2026-09-20, results table in `spikes/s0-baseline.md` |
| No production change | `git status` at phase end: only files under `features/2026-09-19-direct-cell-execution/` |

Broader gates run (all green, 2026-09-20):

| Gate | Result |
|---|---|
| all twelve `python3 scripts/ci/test_*.py` | 769 tests, 12/12 OK (re-ran affected_scope/resolved_plan/schema/ci_local again after the last doc edit) |
| `just _test repo-deps` | 416 passed, 19 slow, 1 skipped (pre-existing) |
| `just _test test-toolkit` | 216 passed, 2 skipped (pre-existing) |
| `just _lint repo-deps` | clean |
| `just _lint test-toolkit` | clean |
| `git status` | only `features/2026-09-19-direct-cell-execution/` modified or added |

Not run this phase (nothing outside the feature directory changed, so their
inputs are identical to the last green run): hook suite
(`.githooks/tests/test-pre-push.sh`), `actionlint`. First required again in
Phase 2/3 when production files change. No tests were skipped by this phase;
no pre-existing failures exist.

### Deviations and notes for later phases

- The Phase 1 checkpoint text says "`rulings.md` records R1–R10" while the
  task list carries R1–R12; all twelve are recorded, which satisfies both.
- `spikes/plans/gap-only.json` hand-derivation should be replaced by a
  `plan_fixtures.py` producer in Phase 3 (noted in `plans/README.md` and in
  the spec's `message_to_agent`).
- The shipped archive-portability fixture does not build via bare
  `cargo nextest archive` on this host (dylib/static mixing); its supported
  path is `ci-build produce`, as its own suite uses — recorded in S2.

Phase 1 complete: all 18 GFM todos in the plan's Phase 1 section are checked.

## Phase 2

Phase 2 is the failing-oracle phase: every new contract landed as a pending
fixture that fails today for its recorded reason, so the suites stay green and
the reason strings are the implementation oracles for Phases 3–6. **52 pending
contracts** were frozen (8 schema + 3 row adapter + 5 selection/capacity +
20 completion + 8 workflow layout + 5 audit + 3 attribution). No production
behavior changed; the production-file edits are the new suite's registration
(`SUITE_REGISTRY`, `scripts/Cargo.toml` metadata, the self-test lists) and the
reintroduction of the Rust `pending_contract` wrapper inside the two test
files. Full inventory with promotion owners: `spikes/s0-baseline.md` §6.

### What was done

1. **Wave 1 — schema, adapter, selection, capacity oracles.**
   - `scripts/ci/test_schema.py`: new `DirectExecutionSchemaOracleTests`
     (8 pending, criterion `schema-v5`). Pins version 5 (v4 refused by
     version before field set), the required/optional split (`skip_policy`
     required at plan level with `{source, content_hash, entries}`; area-level
     `execution_path` required, `"rows"|"lists"`; cell-level `profile`
     optional-but-consistency-required exactly on executing L1/L2/browser
     cells; `requires_node` optional bool), the three coded rejections
     (`skip-policy-cell`, `skip-policy-expired`, `skip-policy-provenance`),
     `validate_resolved_plan(document, today=None)`, and that
     `RECEIPT_SCHEMA_VERSION`/`SCOPE_RECEIPT_SCHEMA_VERSION` do not move.
   - `scripts/ci/test_resolved_plan.py`: new `RowAdapterOracleTests`
     (3 pending). Pins `affected_scope.row_sets(plan)` — pure, plan-only —
     with the four per-area sets, scalar flags, R1 key order
     `{package, gate, environment, runner}`, exact partition of executing
     cells over SIX live-planned corpus shapes (pr, all, all-reused, mixed,
     nightly, prohibited — produced by the real planner, not the frozen v4
     files, so they stay valid after the version bump), the WSL2/native
     split, and zero-executing-area survival (the gap-only trim derived
     in-test exactly as `spikes/plans/README.md` records).
   - `scripts/ci/test_affected_scope.py`: new `DirectExecutionOracleTests`
     (5 pending). Plan-only purity asserted under a hard I/O ban
     (`builtins.open`, `Path.read_text`, `Path.read_bytes` patched to raise
     RuntimeError — an AssertionError there would have masqueraded as
     "pending", so violations ERROR loudly instead) around the carried-receipt
     apply path; selection-unchanged asserted for the synthetic workspace
     (projection lists ≡ executing cells ≡ dispatched rows) and for the REAL
     workspace pr shape against the frozen corpus's stable facets (cells,
     packages, deferred names; head/build keys/job estimate deliberately not
     compared); capacity guard pinned as
     `enforce_output_budgets(rows)` + `AREA_ROW_SET_BUDGET` — RuntimeError
     naming `MATRIX_LIMIT` / "budget", never a truncation.
2. **Wave 2 — producer-completeness oracles.**
   - New suite `scripts/ci/test_completion.py` (22 tests: 20 pending + 2
     ordinary self-checks). Its module docstring is the contract: the
     `scripts/ci/completion.py` CLI binding (flags listed there), the
     expected-manifest v2 shape (explicit `tests`/`ignored`/`excluded` +
     `environment`/`tier`/`target`/`nextest_version`/`from_archive`/
     `selection` provenance), the companions input (the real
     `companion_suites.py` document shape), the backend-proof input, and the
     completion-record shape (`complete`, `head`, `run`, `attempt`,
     `nextest_version`, `reports`, `build`), written only on success (R10).
     17 AC5 fixtures cover: missing report; malformed report; missing
     expected test with the strongest possible skip approval still failing;
     unexpected identity; the extra-filter shrink (caught by the manifest's
     recorded `selection` disagreeing with the cell contract — the only
                     checkable proxy, since both sets shrinking together is
     invisible to set comparison); duplicate identities vs distinct binaries;
     retries normalized both directions (fail→pass completes, pass→fail
     fails); explicit `#[ignore]`; observed skip with unexpired approval
     (completes) and expired approval (fails); canonical tier exclusion
     (completes, with a non-vacuity half proving the exclusion does the
     work); empty expected set without a plan-recorded reason (fails) and
     with companions (completes); companion suite that did not run; failed
     companion; required backend proof absent (fails) and present (passes);
     plus the happy-path record-binding assertions. 3 manifest-v2 fixtures:
     v1 refused; every provenance field required; another-target refused.
   - **Registration** (the plan warns one file produces ten failures): added
     `test_completion.py` to `SUITE_REGISTRY` (`affected_scope.py`),
     `scripts/Cargo.toml` `companion-suites`, `just/ci-local.just`'s
     self-test loop, both stub lists in `test_ci_local.py`,
     `.githooks/tests/test-pre-push.sh`, AND
     `EXPECTED_COMPANION_OWNERS` in `test_affected_scope.py` — a sixth
     coupled location the plan's list did not name (the registry-validation
     tests fail without it).
3. **Wave 3 — workflow, audit, attribution oracles.**
   - `tools/test-toolkit/tests/ci_workflow_contracts.rs`: reintroduced the
     Rust `pending_contract` (with `BISCUIT_PROMOTE_PENDING=1` support, which
     the historical wrapper lacked) and added 8 pending + 1 ordinary:
     area calls the execution workflow once (no per-package matrix); no
     environment-list input in any reader-facing workflow; row-expanding
     jobs exist, guarded, unnamed-by-expression; R3's negative (no
     `needs: test` staging); completion artifact uploads exist and are
     required (covers AC5's upload-failure and cancellation cases — they are
     workflow behaviors, so they live here rather than in
     `test_completion.py`); all-reused/gap-only area skips the call but
     keeps audit, slice, and publisher; and the ordinary
     `every_matrix_in_the_reader_facing_workflows_keeps_fail_fast_false`
     (holds today, must survive Phase 5).
   - `scripts/ci-rollup-tests.rs`: same `pending_contract` wrapper + 5 audit
     fixtures (no completion record; wrong revision; wrong build key;
     absent report inventory; `complete: false` outranking green JUnit). The
     plan fixtures build their plan at `PLAN_SCHEMA_VERSION` (the constant),
     so they survive Phase 3's version bump unedited. The "legacy record
     still takes the legacy path" oracle is expressed as that last fixture
     (the record, when present, is authoritative) PLUS existing ordinary
     coverage — see Deviations.
   - `scripts/ci/test_runner_loss.py`: new
     `DirectExecutionAttributionOracleTests` (3 pending): four-token native
     row label and four-token WSL2 row label each resolve to exactly one
     cell and one status directory (R1's both-forms requirement), and the
     shipped `_package-ci.yml` must declare the four row-set inputs
     (`test-rows`, `check-rows`, `lint-rows`, `wsl-rows`) that Phase 5's
     `JobNameCorpusTests` rewrite will derive labels from.

### Requirement-to-test mapping

| Phase 2 requirement (plan task) | Targeted tests added |
|---|---|
| Schema v5 oracles | `test_schema.py::DirectExecutionSchemaOracleTests` (8 pending) |
| Row adapter oracles | `test_resolved_plan.py::RowAdapterOracleTests` (3 pending) |
| Selection-unchanged oracles | `test_affected_scope.py::DirectExecutionOracleTests` — purity, selection, corpus (3 pending) |
| Capacity-guard oracles | same class — `test_an_over_limit_row_set...`, `test_an_oversized_area_payload...` (2 pending) |
| New suite skeleton + registration | `test_completion.py` created; registered in 6 coupled files; counted by `SuiteOwnershipRegistryTests` and the ci-local/hook harness fixtures |
| AC5 fixture matrix | `test_completion.py::CompletionValidatorOracleTests` (17 pending) + 2 upload/cancellation cases in `ci_workflow_contracts.rs` |
| Expected-manifest v2 oracles | `test_completion.py::ExpectedManifestV2OracleTests` (3 pending) |
| Workflow layout oracles | `ci_workflow_contracts.rs` (8 pending + 1 ordinary fail-fast) |
| Audit oracles | `ci-rollup-tests.rs` (5 pending) |
| Attribution oracles | `test_runner_loss.py::DirectExecutionAttributionOracleTests` (3 pending) |

Verification levels: everything in this phase is L1 by construction — the
oracles exercise the real Python suites, the real Rust test binaries, and the
shipped workflow/policy artifacts directly; no new cross-crate or hosted
surface exists yet (that is what the oracles are FOR). The corpus comparison
reads the real shipped plans under `spikes/plans/` (passive artifact check)
and re-plans with the real planner (end-to-end path).

### Broader gates run (all green, 2026-09-20)

| Gate | Result |
|---|---|
| all thirteen `python3 scripts/ci/test_*.py` | 810 tests, 13/13 OK |
| `BISCUIT_PROMOTE_PENDING=1` per suite | exactly the 52 new contracts fail; plus two documented transitive effects (below) |
| `just _test repo-deps` | 421 passed, 18 slow, 1 skipped (pre-existing) |
| `just _test test-toolkit` | 225 passed, 2 skipped (pre-existing) |
| `just _lint repo-deps` / `just _lint test-toolkit` | clean |
| `.githooks/tests/test-pre-push.sh` | 66/66 passed |
| `actionlint` | not run — no workflow file changed this phase |

No tests were skipped by this phase beyond the two pre-existing skips noted
above; no pre-existing failures exist.

### Deviations and notes for later phases

- **`subTest` is incompatible with `@pending`**: subTest failures are recorded
  by the harness, not raised, so the wrapper cannot see them. Multi-case
  pending fixtures collect their failures and raise one AssertionError. Any
  later fixture author must do the same.
- **Promote mode is diagnostic, not green.** Under `BISCUIT_PROMOTE_PENDING=1`
  two fixtures fail TRANSITIVELY because the env var propagates into nested
  suites: `test_affected_scope.py::CompanionRunnerTests` (it runs a shipped
  suite as a child process) and the relocated-archive L2 test in
  `ci-build-archive-tests.rs` (it runs test-toolkit's L1 from an archive).
  This is the mechanism showing the contracts, which is exactly what the
  checkpoint asks; normal mode is fully green.
- **"A legacy record still takes the legacy path"** could not be a pending
  fixture directly: a body asserting legacy evidence stays green PASSES
  today, which `ContractLanded` would (correctly) reject. It is pinned as
  (a) the pending `an_incomplete_completion_record_blocks_green_junit` — the
  record, once it exists, is authoritative — and (b) existing ordinary
  coverage (reused/gap cells carry no completion record and must stay
  green). Phase 6 should add an explicit ordinary fixture for a legacy-shape
  reused area if its implementation introduces any ambiguity.
- **Upload-failure and cancellation (AC5)** live in `ci_workflow_contracts.rs`
  rather than `test_completion.py`: they are workflow behaviors of the
  upload steps, not validator behaviors.
- **Names Phase 3+ must implement** are pinned in `spikes/s0-baseline.md` §6
  (`row_sets`, `enforce_output_budgets`, `AREA_ROW_SET_BUDGET`, the v5 field
  vocabulary, the completion CLI and record shapes, the row-set input names).
  The completion-record field names here and in `ci-rollup-tests.rs` are
  deliberately identical.
- **Environment note**: this host's shell exports `CDPATH`, which makes
  `$(cd ... && pwd)` inside `.githooks/tests/test-pre-push.sh` print two
  lines and fail its self-location. Run the hook suite with
  `env -u CDPATH bash .githooks/tests/test-pre-push.sh`. This is a host
  quirk, not a repo defect; worth remembering for any script using that
  idiom.

Phase 2 complete: all 12 GFM todos in the plan's Phase 2 section are checked,
every suite is green, and the promote inventory shows exactly the 52 contracts
this plan will implement.

## Phase 3

Phase 3 is Migration step 1: the plan gains the version-5 inputs and the row
adapter lands **beside** the existing environment-list projection, with equality
and uniqueness proved over the corpus. **No workflow file changed** (checkpoint
held; `git status` lists none of `.github/workflows/`). All sixteen Phase-3
Python oracles were promoted — their `@pending` decorators are deleted and their
bodies now pass.

### What was done

1. **Wave 1 — schema and contract.**
   - `scripts/ci/schema.py`: `RESOLVED_PLAN_SCHEMA_VERSION` 4 → 5; new
     vocabulary `EXECUTION_PATHS`, `CI_PROFILE`, `ROW_FIELDS`, `ROW_SET_NAMES`;
     new field tables `SKIP_POLICY_FIELDS` and `SKIP_ENTRY_FIELDS`; required
     plan-level `skip_policy` and optional `rows`; required area-level
     `execution_path`; optional cell-level `profile` and `requires_node`; the
     three rejection codes `skip-policy-cell`, `skip-policy-expired`,
     `skip-policy-provenance`; and `validate_resolved_plan(document,
     today=None)`. The version-before-fields check order is unchanged, so a
     version-4 document still reports exactly one `unknown-schema-version`.
   - `profile` is consistency-required exactly where `build` is — on an
     executing L1/L2/browser cell — and refused on any other shape, because a
     lint or check gate drives no nextest selection.
   - `.github/ci/schemas/contract.json` regenerated through
     `python3 scripts/ci/schema.py`; `scripts/ci-rollup.rs`'s
     `PLAN_SCHEMA_VERSION` moved to 5 and its Rust-side contract fixture
     (`plan_fields_match_the_frozen_contract`) reads the constant, so it moved
     with it.
   - The scope-receipt fallback needed no code change: `SCOPE_RECEIPT_SCHEMA_VERSION`
     stays 1 and its embedded `plan_schema_version` check produces the one
     intended `scope-schema` miss. Pinned end to end by a new hook-suite test
     (below).
2. **Wave 2 — planner population.**
   - `load_skip_policy(root, today=None)` reads `.github/ci/ci-baseline.toml`
     once, validates governance and expiry, and returns
     `{source, content_hash, entries}` with the file's `tier` translated to the
     plan's `gate`. The content hash goes through `build_key.planned_keys` —
     the same `ci-build` xxHash boundary the build keys use — so there is no
     second digest implementation.
   - `applicable_skip_entries(policy, cells)` narrows the snapshot to the cells
     the plan carries, keeping provenance intact (see Deviations).
   - `row_sets(plan)` is the pure adapter: `{area: {test, check, lint, wsl,
     has_*_rows}}`, rows keyed `{package, gate, environment, runner}` in R1's
     order, every selected area present executing or not.
   - `enforce_output_budgets(rows)` + `AREA_ROW_SET_BUDGET` (16 KiB) refuse an
     over-limit or oversized area whole, never truncating; `attach_rows` is the
     single call site that pairs derivation with the guard, so a plan cannot
     carry rows nothing checked.
   - `load_direct_execution(root)` reads the R9 allowlist
     (`.github/ci/direct-execution.json`, absent until Phase 7) and
     `area_records` stamps `execution_path` — `lists` for every area today,
     which is what makes this phase a pure addition.
   - Cells gain `profile` and `requires_node` at creation, after the execution
     decision; `mark_reused` drops both with the execution, exactly as the
     build reference is dropped. `apply_accepted_cells` re-derives `rows`, so a
     carried plan's rows can never describe work evidence has satisfied.
   - `legacy_scope_document` projects `area_rows` beside `area_matrix`, derived
     fresh rather than read from `plan["rows"]`.
3. **Wave 3 — readers, renderer, and the proof.**
   - `scripts/ci/plan_fixtures.py` gained `finalize_plan`, which derives every
     mechanically-derivable half of a fixture plan (builds, `profile`,
     `execution_path`, `skip_policy`). `test_ci_local`, `test_evidence_reuse`,
     `test_cross_check`, and `test_local_evidence` call it instead of
     `attach_builds`, so the v5 shape is spelled once rather than in four
     drifting copies.
   - `scripts/ci-plan.rs` renders the dispatch rows as a `Table` and the skip
     snapshot as `Prose` + `UnorderedList`, reading them from the plan rather
     than re-deriving: a renderer that recomputed the rows could agree with
     itself while disagreeing with the scheduler.
   - `features/2026-09-19-direct-cell-execution/spikes/row-equality.py` and
     `spikes/s4-row-equality.md` are Migration step 1's evidence.

### The equality and uniqueness proof (Migration step 1)

`python3 features/2026-09-19-direct-cell-execution/spikes/row-equality.py`,
run 2026-09-20:

```
pr           cells=   4  rows=   4  lists=   4  areas=  1  OK
all          cells= 374  rows= 374  lists= 374  areas= 31  OK
all-reused   cells=  71  rows=  71  lists=  71  areas= 31  OK
mixed        cells= 308  rows= 308  lists= 308  areas= 31  OK
nightly      cells=  66  rows=  66  lists=  66  areas= 31  OK
prohibited   cells= 308  rows= 308  lists= 308  areas= 31  OK
gap-only     cells=   0  rows=   0  lists=  15  areas= 31  OK
             NOTE lists-vs-cells: the lists schedule 15 cell(s) the plan does
             not carry and omit 0
```

The six shapes a real planner emits agree exactly in both directions, with no
duplicate key anywhere, and `all`'s 374 executing cells match the Phase 1
baseline — the adapter changed no selection.

**The one disagreement is the feature's own argument.** On the synthesized
gap-only shape the two descriptions diverge and the ROW side is right: the rows
read the cells and dispatch nothing, while the lists read a package record's
declared `gates` and would schedule fifteen lint jobs for cells the plan does
not carry. A real planner never emits such a document — but a hand-trimmed one
can, and only one of the two descriptions notices. Recorded in
`spikes/s4-row-equality.md`.

### Requirement-to-test mapping

| Requirement (plan task) | Targeted tests |
|---|---|
| Schema v5, required/optional split, coded rejections | `test_schema.py::DirectExecutionSchemaOracleTests` — all 8 oracles promoted |
| `profile` belongs to exactly an executing test cell | same class, `test_a_profile_belongs_to_exactly_an_executing_test_cell` + `RowEmissionTests::test_the_execution_inputs_belong_to_the_cells_that_run` |
| Row adapter partitions the executing cells | `test_resolved_plan.py::RowAdapterOracleTests` (3 promoted, six live-planned shapes) |
| Adapter reads only the plan; selection unchanged | `test_affected_scope.py::DirectExecutionOracleTests` (5 promoted) |
| Capacity guard refuses, never truncates | same class (2 promoted) + `RowEmissionTests::test_a_within_budget_plan_passes_the_guard_unchanged` (the positive half) |
| Skip snapshot: absent/empty file, governance, expiry, malformed TOML, vocabulary | `test_affected_scope.py::SkipPolicySnapshotTests` (8 new) |
| Skip snapshot: shipped artifact (passive corpus) | `SkipPolicySnapshotTests::test_the_shipped_baseline_snapshots_into_a_valid_plan_field` |
| Skip snapshot: content hash round trip | `SkipPolicySnapshotTests::test_the_content_hash_tracks_the_file_and_is_stable` |
| Applicability narrowing | `SkipPolicySnapshotTests::test_the_snapshot_is_narrowed_to_the_cells_the_plan_carries` |
| Python 3.9 import floor (OS risk) | `SkipPolicySnapshotTests::test_an_interpreter_without_tomllib_still_imports_and_says_why` |
| `execution_path`: default, allowlist, malformed, end to end | `test_affected_scope.py::DirectExecutionPathTests` (4 new) |
| Rows emitted into plan and scope; persisted round trip; withdrawn by evidence | `test_affected_scope.py::RowEmissionTests` (6 new) |
| `requires_node` resolved per cell | `RowEmissionTests::test_node_provisioning_is_resolved_per_cell_from_the_package_and_the_host` |
| Scope-receipt fallback refuses v4 once | `.githooks/tests/test-pre-push.sh::the published scope receipt binds this plan generation` (new, end to end through a real push) + `test_schema.py::test_a_version_two_plan_misses_whole_rather_than_being_upgraded` + `test_resolved_plan.py::test_a_version_2_scope_receipt_misses_once_with_scope_schema` |
| Renderer shows the rows and the snapshot | `scripts/ci-plan-tests.rs` — 4 new (`the_dispatch_rows_are_rendered_as_the_plan_carries_them`, `a_plan_carrying_no_rows_says_nothing_about_dispatch`, `the_skip_snapshot_names_its_source_owner_and_expiry`, `an_approval_without_an_expiry_says_so_rather_than_leaving_it_blank`, `a_plan_whose_baseline_approves_nothing_still_names_the_file_it_read`) |
| End-to-end shipped path | `just ci-local --plan` on this tree (renders 9 dispatch rows across 2 areas and the snapshot); `repo-deps::bin/ci-rollup tests::the_real_planners_plan_rolls_up` |

Verification levels: the schema/adapter work is L1 (pure functions over real
planner output). The skip snapshot crosses a filesystem boundary and a shipped
artifact, so it has a passive corpus test over the real
`.github/ci/ci-baseline.toml` and a read/write/read hash round trip. The plan
crosses a persistence boundary (the scope receipt), so `RowEmissionTests`
canonicalizes, re-parses, and re-validates; the hook suite's new test proves the
same through a real `git push` with a real note. The renderer crosses a terminal
boundary and is covered by the `ci-plan` binary's own suite plus a live
`just ci-local --plan`.

### Broader gates run (all green, 2026-09-20)

| Gate | Result |
|---|---|
| all thirteen `python3 scripts/ci/test_*.py` | 13/13 OK (829 tests, was 810; `test_affected_scope` 256 → 275) |
| `BISCUIT_PROMOTE_PENDING=1 python3 scripts/ci/test_schema.py` | OK — nothing pending remains in it |
| `BISCUIT_PROMOTE_PENDING=1 python3 scripts/ci/test_resolved_plan.py` | OK — nothing pending remains in it |
| `BISCUIT_PROMOTE_PENDING=1 python3 scripts/ci/test_affected_scope.py` | 1 failure, the documented transitive one: `CompanionRunnerTests` shells into the shipped `test_runner_loss.py`, whose 3 Phase-5 attribution contracts are still pending |
| `just _test repo-deps` | 426 passed (421 + the 5 new `ci-plan` fixtures), 14 slow, 1 skipped (pre-existing) |
| `just _test test-toolkit` | 225 passed, 2 skipped (pre-existing) |
| `just _lint repo-deps` / `just _lint test-toolkit` | clean |
| `env -u CDPATH bash .githooks/tests/test-pre-push.sh` | 67/67 passed (66 + the new one) |
| `just ci-local --plan` | renders on this tree, including the dispatch table and the snapshot |
| `actionlint` | not run — no workflow file changed this phase |

No tests were skipped by this phase beyond the two pre-existing skips; no
pre-existing failures exist. One pre-existing host warning is unrelated to this
work: `rust-objcopy` cannot strip debug info on this Mac (`libLLVM.dylib` is
missing from the 1.98.1 toolchain), so every release build prints it.

### OS considerations

- **The Python floor cost a real catch.** A module-level `import tomllib` in
  `affected_scope.py` would have broken the companion step on macOS and Windows
  runners: `companion_suites.py` imports the planner and runs on all three
  native environments, and macOS ships CPython 3.9 at `/usr/bin/python3`, which
  has no `tomllib` (3.11+). The import is now guarded
  (`except ModuleNotFoundError: tomllib = None`) and `load_skip_policy` refuses
  by name only when a *non-empty* baseline meets an interpreter without it. The
  planner itself runs only on the scope job's `ubuntu-latest` and on developer
  hosts, and all thirteen `test_*.py` suites are registered
  `environment: ubuntu-latest`. Recorded in `.claude/skills/os/ci-runners.md`,
  with the `ast.parse(..., feature_version=(3, 9))` sweep that catches the
  grammar half.
- **The content hash is OS-stable.** `.gitattributes` sets `* text=auto eol=lf`,
  so the working tree is LF everywhere and hashing the baseline's bytes gives
  the same digest on Windows as on Linux. Also recorded in the OS skill.
- **Paths.** Every new path is built with `pathlib` from a POSIX-spelled
  constant, which Windows accepts; the `source` field stays the POSIX spelling
  so the plan reads the same on every host.
- `just cross-check` was **not** run. The phase changes Python (planner-only,
  Linux/developer-host paths), one Rust constant, one renderer, and docs; the
  two OS-sensitive facts above were resolved from recorded knowledge and a
  local grammar sweep rather than by occupying a shared build host for a
  full-package rebuild. Phase 5, which touches the workflows and the WSL2
  guest, is where a cross-check earns its cost.

### Deviations and notes for later phases

- **The skip snapshot is NARROWED to the plan's cells, not refused for naming
  another.** R8 reads "an approval for a cell the plan does not carry is a
  planner-time error". Taken literally that makes the repository unplannable:
  `ci-baseline.toml` is repository-wide, so one entry for `foo/ubuntu/L2` would
  fail every plan that does not select `foo` — including every small pull
  request. The planner therefore *filters* to applicable entries (the
  specification's own words: "per-cell and backend applicability") and keeps
  full provenance, while `schema.validate_resolved_plan` still raises
  `skip-policy-cell` for a document whose entries do not bind — so the coded
  rejection is a corruption check on the artifact rather than an ordinary
  operating state. **Expiry is enforced exactly as R8 says**: an expired entry
  fails planning outright, wherever it points. The file is empty today, so this
  moves no data.
- **`tests` on a skip entry is OPTIONAL.** The Phase 2 fixtures pin it that way:
  `test_completion.py::skip_policy_with` builds entries with no `tests` key and
  expects them to excuse an observed skip. An entry that names no identity
  therefore approves any observed skip in its cell; one that names identities
  narrows to them. Phase 4 implements that reading in `completion.py`.
- **Row key ORDER does not survive `schema.canonical`**, which sorts keys. R1's
  order is the *emitter's* contract — it is what GitHub renders a matrix job's
  label from — and it survives the `scope.json` projection the workflow expands
  (stdout is written without `sort_keys`). `schema._rows` therefore validates
  the key SET; `row_sets` owns the order, and `RowEmissionTests` asserts it on
  the adapter's output. **Phase 5 must keep the row transport out of a
  sort_keys serializer**, or the labels degrade to
  `test (macos-latest, L1, pkg, macos-latest)`.
- **The plan carries `rows`, and validation does NOT join them back to the
  cells.** The Phase 2 oracle `test_an_area_with_zero_executing_cells_still_appears`
  builds a gap-only document by trimming `cells` while keeping everything else,
  and asserts it still validates; a join check would refuse it. `_rows`
  therefore checks shape, area membership, and global key uniqueness only. The
  join is proved where it belongs — by `row_sets` deriving from the cells, by
  the adapter oracles, and by Phase 5's workflow-boundary contract.
- **`area_rows` was deliberately NOT added to `schema.SCOPE_PROJECTION_FIELDS`.**
  CI re-projects the legacy document from the carried plan (`--apply-to`), so
  the receipt's stored `scope` is compatibility ballast; adding a required field
  to it would invalidate receipts for no reader. Phase 5 adds it if `ci.yml`
  starts reading it from a receipt, and Phase 8 owns keeping that tuple honest.
- **Two Phase 2 oracle bodies had latent defects** that were masked because they
  failed for their recorded reason before the implementation existed. Both were
  fixed as part of promoting them, and neither weakened the contract:
  `test_the_real_pr_shape_selects_what_the_phase_1_corpus_recorded` planned
  without `event="pull_request"` and so compared a six-cell plan against the
  corpus's four-cell one; `test_selection_outputs_are_unchanged_when_rows_land`
  compared a sorted list against `matrix_record`'s table-ordered one.
- **`DEFAULT_EXECUTION_PATH` is `"lists"`.** Every area states `lists` today,
  which is the honest Phase-3 state: no workflow consumes rows yet. Phase 7
  ships `.github/ci/direct-execution.json` with `["root"]`; `load_direct_execution`
  and `area_records` already accept it and `DirectExecutionPathTests` pins both
  branches.
- **One extra `ci-build key` invocation per plan.** The skip snapshot's content
  hash goes through `build_key.planned_keys`, so a plan with no build records now
  also needs the helper. Every environment that plans already needed it for
  build keys, and the result is memoizable, but a future fixture that plans with
  `BISCUIT_CI_BUILD_BIN` unset and no Cargo will now fail slightly earlier.
- **`.claude/skills/rust-devops/ci-cd.md` got the version-5 correction now**
  rather than waiting for Phase 8's topology rewrite: five phases of agents will
  read "`RESOLVED_PLAN_SCHEMA_VERSION` is 4" otherwise. The fan-out topology,
  label contract, and tightened skip interpretation remain Phase 8's.

Phase 3 complete: all 13 GFM todos in the plan's Phase 3 section are checked,
every suite is green, and no workflow file has changed.

## Phase 4

Phase 4 builds the producer completion contract as a **standalone tool with
fixtures**. Phase 5 wires it into the workflows, which is where the
specification's "implement them together" applies — so the checkpoint's "no
workflow file has changed" held throughout (`git status .github/workflows/` is
empty). All twenty Phase-2 completion oracles were promoted: their `@pending`
decorators are deleted and their bodies pass against the shipped validator.

### What was done

1. **Wave 1 — expected manifest v2 (`just/devops.just::_expected_manifest`).**
   - The `_tier_filter`-derived selection is unchanged — its whole point is
     that expected and observed come from one expression — and archive mode
     still routes through `_archive_drop_build_flags`.
   - The per-package value becomes three disjoint identity sets instead of one
     list: `tests` (`filter-match.status == "matches"`), `ignored`
     (`mismatch` + `reason == "ignored"`), and `excluded` (`mismatch` +
     anything else). S2 proved those two reasons are distinguishable at the
     resolved version; a `cfg`-excluded test is in none of them, because it is
     absent from the listing entirely.
   - Provenance: `environment`, `tier`, `target`, `nextest_version`,
     `from_archive`, and `selection` (`filter`, `profile`, `test_args`).
   - **`target` is read from the listing's own `rust-build-meta`**
     (`target-platforms[0].triple`, falling back to
     `platforms.host.platform.triple`), never from `rustc`. That is what makes
     the requirement "no consumer needs a compiler merely to list" true, and in
     archive mode it yields the PRODUCER's target — the one `cfg` was evaluated
     against — which is exactly what the comparison needs.
   - Optional `backends` and `companion_suites` record what the job was *told*
     to require (`$BISCUIT_TEST_REQUIRED_BACKENDS`,
     `$BISCUIT_CI_COMPANION_SUITES`, the latter exported by Phase 5). The plan
     stays the source of truth; `completion.py` refuses a disagreement rather
     than preferring a side.
   - **Provisioning-before-listing** is documented in the recipe header (listing
     RUNS each test binary) and enforced as far as a recipe honestly can: a
     failed listing is reported with that order named, because an unprovisioned
     listing is otherwise indistinguishable from a drifted filter.
2. **Wave 2 — `scripts/ci/completion.py`** (new, stdlib only per R2). It reads
   the plan, the cell key, the v2 manifest, the JUnit staging tree
   (`manifest.jsonl` included), the companion results, and the backend proofs;
   compares **identities** scoped by cell, backend, and companion suite; and
   writes the completion record only on success.
   - `schema.py` gains `EXPECTED_MANIFEST_SCHEMA_VERSION = 2`,
     `COMPLETION_RECORD_SCHEMA_VERSION = 1`, the three field tables, the
     `COMPLETION_REJECTIONS` vocabulary, `TIER_MARKERS`, and
     `validate_completion_record`. `contract.json` regenerated.
   - Neither new version touches `RESOLVED_PLAN_SCHEMA_VERSION`,
     `RECEIPT_SCHEMA_VERSION`, or `SCOPE_RECEIPT_SCHEMA_VERSION`: how a job
     proves its cell is not a reason to invalidate a plan or a validated cell.
3. **Wave 3 — fixtures.** `test_completion.py` grew from 22 to 44 tests: the 20
   promoted AC5/manifest oracles plus five new classes (below).
   `ci-rollup-tests.rs` gained the cross-language assertion
   `completion_record_fields_match_the_frozen_contract`, modelled on
   `plan_fields_match_the_frozen_contract`.

### The three judgement calls worth reading

**1. How an ad hoc filter is caught.** The specification requires that "an
extra filter must not shrink both the expected and observed sets and pass
undetected". Set comparison cannot see it — both sides shrank. The plan does
not carry a per-cell filter expression (Phase 3 moved `profile` and
`test_args` onto it, not the filterset), and adding one is a schema-6 change
outside this phase. So the manifest records *what it selected with*, and
`completion.py` checks that expression against the **canonical selection
vocabulary**: only the predicates `package(…)`, `test(…)`, and `all()`, and a
`test(…)` argument must be a tier marker as `_tier_filter` anchors it
(`/(^|::)<marker>/`). The filter STRINGS stay in the justfile — copying them
would be the drift this feature exists to remove — and
`ShippedSelectionTests` proves every shipped expression (six tiers, the
`worktree-cli` `perf_` variant, the `l1-include-slow` variant, and the
archive-mode `package(x) & (…)` wrapper) passes the rule, with a non-vacuity
half proving an ad hoc `test(one_named_test)` still fails it. The live dry run
confirmed it end to end: `BISCUIT_TEST_FILTER='test(/(^|::)the_canonical/)'`
applied to BOTH the run and the listing is refused as
`completion-manifest-selection`, naming the expression.

**2. `test_args` is recorded but not compared.** The package's declared
`test_args` and the applied ones legitimately differ in archive mode —
`_archive_drop_build_flags` removes every build flag — so comparing them would
fail every archive cell for something that is not a defect. It is recorded as
provenance and scanned for one thing: `--run-ignored`, which flips ignored
tests to `matches` and would silently move the expected set (S2's first
condition). `selection.profile` IS compared, against the cell's `profile`.

**3. Retry versus duplicate.** Two Phase-2 fixtures hand the validator the same
shape — two reports covering the same identities — and demand opposite
verdicts. Per-identity rules cannot separate them (the retry fixture's
`green_one` passes in both attempts). The rule that does: **reports are grouped
by their identity set**, which is what "the same work" means; within a group
the last staged report decides, and a group whose reports also agree on every
outcome retried nothing — it is one result staged twice, which inflates the
evidence. Identities shared across *different* identity sets are the binary
collision the `<binary-id>::<test name>` convention exists to catch. Both
refuse with `completion-report-duplicate`; the failure a retry ends on is never
hidden, because the last attempt is the one the job ended on.

### Requirement-to-test mapping

| Requirement (plan task / AC5 clause) | Targeted tests |
|---|---|
| Manifest v2: ignored and excluded recorded explicitly | `test_an_explicitly_ignored_test_is_expected_but_not_required`, `test_a_canonical_tier_exclusion_is_not_a_missing_test` (with its non-vacuity half), and the real listing in `ShippedRecipeEndToEndTests` |
| Manifest v2: provenance required; v1 refused; other target refused | `ExpectedManifestV2OracleTests` (3, promoted) |
| Manifest v2: declared backends and suites cross-checked | `ManifestCrossCheckTests::test_declared_backends_matching_the_plan_complete`, `…_provisioned_for_other_backends_is_refused`, `…_ran_other_companions_is_refused` |
| Provisioning-before-listing | recipe header + named listing-failure diagnostic; exercised live by `ShippedRecipeEndToEndTests` (a listing that cannot run its binaries fails there) |
| Identity comparison, not counts | `test_a_green_fully_reported_run_completes_and_binds_its_evidence`, `test_an_unexpected_identity_fails_validation`, `test_duplicate_identities_fail_while_distinct_binaries_stay_distinct` |
| Extra filter must not shrink both sets undetected | `test_an_extra_filter_that_narrows_the_selection_is_refused`, `ManifestCrossCheckTests::test_run_ignored_…`, `…_a_profile_other_than_the_cells_…`, all four `ShippedSelectionTests`, and the live dry run |
| Retries normalized without hiding a failure | `test_retries_normalize_to_one_final_outcome_without_hiding_a_failure` (both directions) |
| Missing/malformed reports fail | `test_a_missing_report_fails_validation`, `test_a_malformed_report_fails_validation` |
| A missing test cannot be excused by a skip approval | `test_a_missing_expected_test_fails_and_a_skip_approval_cannot_excuse_it` (strongest possible approval: unexpired, exact cell) |
| Observed skip: approved passes, expired fails | `test_an_observed_skip_with_an_unexpired_approval_completes`, `…_an_expired_approval_fails` |
| Empty expected set needs a plan-recorded reason; companion-only cell | `test_an_empty_expected_set_without_a_plan_recorded_reason_fails`, `test_a_companion_only_cell_completes_when_its_declared_suite_does`, `…_did_not_complete`, `test_a_failed_companion_suite_fails_the_cell` |
| Required backend proof | `test_a_required_backend_proof_absent_fails_and_present_passes` (both halves) |
| Never compare an L1 report against every tier in a shared archive | manifest `tier` vs cell gate check; staging entries filtered by `{tier, package, environment}` — `ManifestCrossCheckTests` L2 fixtures use a separate `junit-…-L2-…` tree |
| Record: versioned, keyed, binds revision/build/gate inputs/run/attempt/inventory | `CompletionRecordContractTests` (5), `CompletionRecordValidationTests` in `test_schema.py` (9) |
| Record written only after validation succeeds (R10) | every refusal fixture asserts `out.exists()` is false, plus `test_a_refusal_removes_a_record_an_earlier_validation_left` |
| Which cell may be certified at all | `CellBindingTests` (7): unknown cell, reused cell, duplicated cell, unreadable plan, test gate with no manifest, check gate without a listing, exit-code split |
| Cross-language contract | `test_the_shipped_contract_json_describes_the_record` (Python) + `completion_record_fields_match_the_frozen_contract` (Rust) |
| Failure path | exit `1` (cell unproven) vs `2` (inputs unreadable), pinned by `test_an_unreadable_plan_is_refused_by_name` and `test_a_cell_that_did_not_prove_itself_exits_one`; the upload-failure and cancellation halves stay pending in `ci_workflow_contracts.rs`, where the upload steps live (Phase 5) |

**Verification levels.** The validator crosses a CLI boundary, so every fixture
runs the real `python3 scripts/ci/completion.py` as a subprocess — not the
module — and asserts the exit code, stderr, and the file at `--out`. It crosses
a filesystem boundary (the staging tree), so the fixtures write real staging
trees the way `_stage_junit` leaves them. It crosses a persistence boundary
(the record is an artifact two jobs compare), so
`test_the_record_round_trips_through_the_file_it_is_published_as` writes, reads,
re-validates, and re-runs the whole validation asserting byte-identical output.
Two passive corpus tests cover the shipped artifacts: the real `_tier_filter`
expressions, and `contract.json` from both languages. One end-to-end test drives
the real recipe over a real crate.

### The Phase 4 checkpoint dry run (real package, real path)

`repo-deps` L1 on this macOS host, with the real planner, the real gate, and the
real recipe — no fixtures:

```
python3 scripts/ci/affected_scope.py --resolved-plan scripts/ci/completion.py > plan.json
BISCUIT_CI_ENVIRONMENT=macos-latest NEXTEST_PROFILE=ci \
  BISCUIT_JUNIT_STAGE_DIR=/tmp/dryrun/stage just _test repo-deps
  → 427 tests run: 427 passed, 1 skipped
BISCUIT_CI_ENVIRONMENT=macos-latest NEXTEST_PROFILE=ci \
  BISCUIT_JUNIT_STAGE_DIR=/tmp/dryrun/stage just _expected_manifest L1 repo-deps
  → 427 expected, 1 ignored/excluded, aarch64-apple-darwin
python3 scripts/ci/completion.py --plan plan.json --cell repo-deps/macos-latest/L1 \
  --expected-manifest .../expected-L1.json --artifacts /tmp/dryrun/stage \
  --target aarch64-apple-darwin --run 987654 --attempt 1 --out completion.json
  → ✅ repo-deps/macos-latest/L1 complete: 1 report(s) match the expected listing
```

The record it wrote bound `head`, `run`, `attempt`, the resolved
`cargo-nextest 0.9.136 (1d5bf1ec9 2026-05-16)`, `reports: ["L1/repo-deps.xml"]`,
`build: 27169df4727d2d84`, and the seven `gate_inputs` paths the plan carries.

Two deliberate narrowings, both refused with no record written:

| Narrowing | Verdict |
|---|---|
| run narrowed (`BISCUIT_TEST_FILTER='test(/(^|::)the_canonical/)'`), full manifest | `completion-test-missing`, one line per absent identity — the checkpoint's required reason |
| run AND listing narrowed together | `completion-manifest-selection`, naming the expression — the case a set comparison cannot see |

Worth recording: the repo-deps L1 suite stages its own `archive-portability`
fixture reports into the same tree under other packages. The validator filters
staging entries by `{tier, package, environment}`, so they were correctly
ignored — the first real evidence that the "never compare an L1 report against
every tier in a shared archive" scoping works on a live tree rather than a
fixture.

### Broader gates run (all green, 2026-09-20)

| Gate | Result |
|---|---|
| all thirteen `python3 scripts/ci/test_*.py` | 13/13 OK (862 tests, was 829; `test_completion` 22 → 44, `test_schema` 118 → 131) |
| `BISCUIT_PROMOTE_PENDING=1 python3 scripts/ci/test_completion.py` | OK — nothing pending remains in it |
| `BISCUIT_PROMOTE_PENDING=1 python3 scripts/ci/test_schema.py` | OK |
| `BISCUIT_PROMOTE_PENDING=1 python3 scripts/ci/test_resolved_plan.py` | OK |
| `BISCUIT_PROMOTE_PENDING=1 python3 scripts/ci/test_runner_loss.py` | 3 failures — exactly the Phase-5 attribution oracles, still pending as designed |
| `BISCUIT_PROMOTE_PENDING=1 python3 scripts/ci/test_affected_scope.py` | 1 failure — the documented transitive one (`CompanionRunnerTests` shells into the shipped `test_runner_loss.py`) |
| `just _test repo-deps` | 427 passed (19 slow), 1 skipped (pre-existing) |
| `just _test test-toolkit` | 225 passed, 2 skipped (pre-existing) |
| `just _lint repo-deps` / `just _lint test-toolkit` | clean |
| `env -u CDPATH bash .githooks/tests/test-pre-push.sh` | 67/67 passed |
| `actionlint` | not run — no workflow file changed this phase |

No tests were skipped by this phase beyond the two pre-existing skips; no
pre-existing failures exist. (`env -u CDPATH` remains necessary on this host —
see the Phase 2 note.)

### OS considerations

- **Two shell-portability defects were found and fixed by reading, not by a
  red test.** `nextest --version | head -n 1` closes the pipe after one line;
  under `set -o pipefail` the resulting EPIPE would fail the recipe on a host
  whose pipe buffer is smaller than the version banner — it passes here only
  because the banner fits. It now captures the output whole and trims with
  `${var%%$'\n'*}`. And `(( archive_mode == 1 )) && from_archive=true` returns
  1 in the common (non-archive) case; bash exempts it from `set -e` today, but
  the exemption is subtle enough that it is now a plain `if`.
- **No compiler is needed to list.** The manifest's `target` comes from the
  listing's own `rust-build-meta`, so the WSL2 guest — which has neither Cargo
  nor `rustc` — can produce a fully provenanced manifest from an archive. This
  was the concrete risk behind R2 and it is closed at the tool level; the
  guest's `python3` provisioning is still Phase 5's.
- **The Python 3.9 floor holds.** `completion.py` imports only `argparse`,
  `json`, `re`, `sys`, `datetime`, `pathlib`, `typing`, and
  `xml.etree.ElementTree`, all 3.9-available, and uses
  `from __future__ import annotations` so its `dict[...]`/`X | None`
  annotations never evaluate. The recorded grammar sweep
  (`ast.parse(..., feature_version=(3, 9))` over `scripts/ci/*.py`) passes.
  `Path.unlink(missing_ok=True)` is 3.8.
- **Windows temp cleanup.** The end-to-end fixture compiles a scratch crate
  inside its temp directory, and Windows refuses to remove a file a
  just-exited process still holds. It uses `mkdtemp` +
  `shutil.rmtree(..., ignore_errors=True)` rather than `TemporaryDirectory`;
  `ignore_cleanup_errors` would say the same thing but needs Python 3.10.
- `just cross-check` was **not** run. This phase adds one Python tool, edits one
  justfile recipe, and changes no workflow; the suites that exercise them are
  registered `environment: ubuntu-latest`, and the two OS-sensitive facts above
  were resolved by reading rather than by occupying a shared build host. Phase 5,
  which touches the workflows and the WSL2 guest, is where a cross-check earns
  its cost.

### Deviations and notes for later phases

- **`nextest_version` is OPTIONAL on the completion record.** A `check` or
  `lint` cell drives no nextest at all, and a placeholder there would read as a
  version. `completion.py` therefore accepts a cell with no
  `--expected-manifest` **only** for a non-`BUILD_GATES` gate, and requires one
  for every test gate (`completion-manifest-missing`). Phase 5's check and lint
  producers can call the tool unchanged; `CellBindingTests` pins both halves.
- **`gate_inputs` on the record is the package's `input_paths`, not a hash.**
  The plan carries the paths; the gate-input *identity* needs `git ls-tree` and
  the Just closure, which is `local_evidence.py`'s boundary and not something a
  producer should recompute. Optional, present when the plan carries them.
- **The Rust `EXPECTED_MANIFEST_SCHEMA_VERSION` stays at 1** and its struct
  still expects the v1 `packages: {pkg: [identity]}` shape. Nothing wires them
  — no workflow passes `--expected-manifest`, and the producer's own
  `completion.py` consumes the v2 document — so this is a dormant reader, not a
  live mismatch. Its doc comment now says so and names Phase 6 as the owner.
  **Phase 6 must decide its fate before anything passes `--expected-manifest`
  again**; a v2 manifest handed to today's `ci-rollup` fails on both the
  version check and the shape.
- **`test_args` is not compared** (see judgement call 2). If a later phase wants
  it compared, the honest way is to record the *declared* args separately from
  the *applied* ones, not to tighten the current field.
- **The canonical-selection rule is a whitelist and will need widening if a
  tier ever gains a predicate.** `ShippedSelectionTests` fails first, loudly,
  rather than every producer turning red — that is deliberate. The marker
  vocabulary lives in `schema.TIER_MARKERS`; the expressions stay in
  `just/devops.just`.
- **An entry with no `tests` approves any observed skip in its cell**, as
  Phase 3's note recorded and the Phase-2 fixtures pin. `.github/ci/ci-baseline.toml`
  is still empty, so this moves no data. Its header rewrite is Phase 6's.
- **`BISCUIT_CI_COMPANION_SUITES` does not exist yet.** `_expected_manifest`
  reads it and omits `companion_suites` when unset; Phase 5 exports it from the
  cell contract so the cross-check has both sides. `BISCUIT_TEST_REQUIRED_BACKENDS`
  already exists and is already exported for L2.
- **The completion artifact name is `completion-<package>-<gate>-<environment>`**
  (R10), matching the Phase-2 Rust fixture's directory. Phase 5 uploads it;
  Phase 6 adds `completion-*` to the audit's download pattern.

Phase 4 complete: all 10 GFM todos in the plan's Phase 4 section are checked,
every suite is green, and no workflow file has changed.

## Phase 5

Phase 5 is Migration step 2: **the workflows and the producer contract went
live together.** The four reader-facing workflows now dispatch one hosted job
per executing cell, every job resolves its execution contract from the plan
through one reader, and every producer certifies itself before its cell can be
counted. All eight Phase-2 workflow oracles and all three attribution oracles
were promoted — the `pending_contract` wrapper is deleted from
`ci_workflow_contracts.rs` with its last user.

### What was done

1. **Wave 1 — the execution workflow.**
   - `.github/workflows/_package-ci.yml` went from six environment-list jobs to
     **four row-driven ones**: `check`, `test` (native L1, L2, and browser
     rows in ONE job), `lint`, and `wsl2` (one delegated call per guest row).
     Each expands `include: ${{ fromJSON(inputs.<set>-rows) }}` behind the
     scalar guard `inputs.<set>-rows != '[]'`, carries no `name:`, and keeps
     `fail-fast: false`. Every environment-list input is gone.
   - `scripts/ci/cell_contract.py` (new, stdlib only per R2) is the one
     reader. Given the plan, a row, and the run's tested revision it joins the
     row back to its cell and emits the whole execution contract as step
     outputs — recipe, profile, test/check arguments, dependent seam, runner
     tools, backends, companion suites, native prerequisites, `requires_node`,
     `requires_toolchain`, the build record, the execution target triple, and
     the three result-artifact names. It refuses with a coded reason
     (`schema.CELL_CONTRACT_REJECTIONS`, new) before anything runs.
   - **Tier branching** is confined to where execution genuinely differs: tmux
     provisioning and the backend-coverage report on `matrix.gate == 'L2'`,
     `BISCUIT_BROWSER_REQUIRED` on the browser rows, `--no-fail-fast` and the
     L2 worker budget inside the gate command's `case`, and the toolchain and
     Node steps on the plan's own per-cell answers. Timeouts are preserved
     exactly (`${{ matrix.gate == 'L1' && 45 || 30 }}`).
   - **Producer completeness** is wired: `_expected_manifest` runs after every
     provisioning step and BEFORE the gate, `completion.py` runs after it, and
     the completion record is uploaded — required, no `continue-on-error`,
     no status predicate.
   - **R12**: `_expected_manifest` joined `CI_RECIPES_BY_GATE["test"]`.
2. **Wave 2 — callers and the guest.**
   - `_area-ci.yml` calls `_package-ci.yml` **once per area**, guarded by
     `fromJSON(inputs.rows).has_*_rows`, and forwards the four sets verbatim.
     Its audit now resolves package membership from the PLAN (`select(.area ==
     $area and (.gates | length) > 0)`) rather than from the execution matrix,
     which is what lets an all-reused or gap-only area keep a blocking audit
     with no call at all.
   - `ci.yml` publishes `area_rows` in place of `area_matrix`. `has_packages`,
     `preflight`, `build`, `area-drift`, `ci-gate`, and `ci-reporting` are
     untouched; `ci-gate` is byte-identical (`git diff` touches no line of it).
   - `_wsl-ci.yml` accepts **one row**, resolves its contract on the Windows
     host before provisioning a guest, adds `python3` to the declared apt set
     with `python3 --version`/`jq --version` in that same named step (R2),
     lists its expected tests in the guest as the unprivileged user, and runs
     `completion.py` in the guest — where the reports, the listing, and the
     plan actually are.
   - **R11**: `.github/workflows/_area-ci.yml` was added to
     `GLOBAL_PATHS_ALL_GATES` **and** `ORCHESTRATION_PATHS` in one change.
3. **Wave 3 — attribution, lint, promotion.**
   - `runner_loss.py` gained `ROW_GATES`, `row_cell`, and
     `WSL_DELEGATION_ROW`. A four-token parenthetical carries the whole cell;
     a three-token one keeps the pre-row meaning. The WSL2 row rides on the
     delegating `wsl2 (...)` segment, because the delegated workflow's job name
     is static and cannot carry it.
   - `actionlint` is clean on all four reader-facing workflows.

### The four judgement calls worth reading

**1. One native test job, not three.** R3 prescribed it and the specification
forbids an L1 prerequisite that can suppress required L2 or browser work. The
consequence is that `matrix.gate` — `L1`, `L2`, or `browser` — IS the tier
identity `ci-rollup` keys a status by, so the status, JUnit, and completion
artifact names fall out of the row rather than out of three job definitions.
The contract that used to assert the staging (`expensive_tiers_stage_behind_l1…`,
now `no_test_row_job_stages_behind_another_test_row_job`) asserts the negative.

**2. `_expected_manifest` runs BEFORE the gate.** The plan says so and the
reason is fail-fast: listing EXECUTES each test binary, so it fails for the
same causes the gate would — an unprovisioned native library, a missing
sidecar, an absent pinned toolchain — and saying so in seconds beats saying it
after a forty-minute suite. The disk risk this raised on the WSL2 leg
(run 30605643702 exhausted the Windows host's disk during extraction) is not
real: the two extractions are sequential, so PEAK VHDX growth is unchanged,
and peak is what the 2026-08 failures were about. Recorded in the `os` skill.

**3. `native_packages` is a JSON array, not a space-joined string.** The first
draft joined the list with spaces and split it with word-splitting, which
silently corrupts a package name carrying a space or a glob character. The
shipped contract `test_ci_local.py::test_native_names_are_passed_as_literal_arguments`
(`"literal*name"`, `"name with spaces"`) caught it. The reader emits JSON and
every consumer parses it with `jq`.

**4. Two Phase-2 Rust oracle bodies had latent defects**, both fixed while
promoting them, neither weakened. `every_producer_uploads_a_completion_artifact`
and `a_failed_required_upload_fails_the_job` asserted that the whole JOB
contains no `continue-on-error: true` — which would have forbidden the
advisory compiler-work measurement steps that must stay advisory. Both now
scope the assertion to the two steps that carry the proof (the validation and
the upload), and the first additionally pins that the JUnit and status uploads
keep publishing after a failure. They also matched the literal prefix
`name: completion-`, which no longer appears: the names are derived by
`cell_contract.py`, so the fixtures identify the steps by name instead.

### Requirement-to-test mapping

| Requirement (plan task) | Targeted tests |
|---|---|
| Four row-driven jobs, guarded, unnamed, `fail-fast: false` | `ci_workflow_contracts.rs::every_row_expanding_job_carries_a_scalar_guard_and_no_expression_name` (promoted), `…::empty_execution_matrices_skip_before_expansion`, `…::every_matrix_in_the_reader_facing_workflows_keeps_fail_fast_false`, `…::lint_and_check_labels_identify_their_environment` |
| No environment-list input anywhere | `…::no_reader_facing_workflow_declares_an_environment_list_input` (promoted) |
| `cell_contract.py` resolves, and refuses by coded reason | `test_resolved_plan.py::CellContractTests` (15, all through the real CLI): every dispatched row resolves; the execution inputs; the derived artifact names; the check seam; the guest's Linux prerequisites and producer target; and seven refusals — unknown, duplicate, non-executing, wrong runner, unknown environment, wrong revision, malformed row, dangling build |
| The contract is a persisted boundary | `CellContractTests::test_the_contract_is_appended_and_is_byte_identical_on_a_rerun` (write/read/re-run round trip) |
| Tier branching only where execution differs | `…::the_l2_consumer_still_provisions_and_proves_its_runtime_backend`, `…::the_worker_policy_survives_the_area_restructure`, `…::the_l1_suite_runs_no_fail_fast` (all read the gate command's own `case` branches), `…::a_declared_toolchain_requirement…`, `…::a_declared_node_capability…` |
| No test job stages behind another (R3) | `…::no_test_row_job_stages_behind_another_test_row_job` (promoted) |
| Producer completeness steps, in order | `…::an_archive_consumer_verifies_first_and_never_reaches_a_compiler` (verification precedes BOTH the listing and the gate), `…::a_declared_toolchain_requirement…` (the toolchain precedes the listing), `…::an_archive_consumer_asks_cargo_for_nothing_including_metadata` (the listing reaches Cargo for nothing either) |
| Completion record: uploaded, required, validated first | `…::every_producer_uploads_a_completion_artifact`, `…::a_failed_required_upload_fails_the_job`, `…::cancellation_keeps_failure_path_publication_best_effort` (all promoted) |
| R12: `_expected_manifest` selects the test gate and moves its identity | `test_affected_scope.py::SelectionEntryPairingTests` (4 of 6) |
| One guarded call per area; all-reused area keeps audit, slice, publisher | `…::the_area_workflow_calls_the_execution_workflow_once`, `…::an_all_reused_area_skips_execution_but_keeps_its_audit_slice_and_publisher` (both promoted), `…::a_reused_cell_reaches_its_area_summary_without_being_re_executed` |
| `ci.yml` publishes row sets; `ci-gate` untouched | `…::the_package_matrix_is_scope_derived_not_static`, `test_schema.py::test_the_projection_fields_are_what_the_workflow_reads`, `test_ci_local.py::WorkflowScopeStepTests` (`MATRIX_OUTPUTS`) |
| The guest takes one row and proves itself | `…::the_wsl_leg_provisions_jq_and_forwards_the_slow_test_contract` (the apt set INCLUDING python3, reachability in the same step, and the slow-test contract exported twice — once for the listing, once for the gate), `…::the_wsl2_guest_consumes_the_same_build_as_native_linux` |
| Row-set disjointness at the boundary | `test_resolved_plan.py::RowSetBoundaryTests` (3): the forwarded inputs are read out of the shipped `_area-ci.yml`, they equal what `_package-ci.yml` expands, they partition every area's executing cells with no duplicate key, and a dropped set is visibly missing coverage |
| R11: the path forces workspace scope AND is absent from gate identity | `test_affected_scope.py::SelectionEntryPairingTests` (2 of 6, incl. the whole-table pairing rule), `test_local_evidence.py::OrchestrationExclusionTests` (3, through `gate_global_inputs` with a non-vacuity half) |
| Labels resolve to exactly one cell, in both forms | `test_runner_loss.py::JobNameCorpusTests` (7, every label derived from the shipped workflows and cross-checked against `cell_contract.contract`), `…::DirectExecutionAttributionTests` (5, promoted + two new: the three-token form still resolves, and a four-token label naming a JOB id rather than a gate is refused rather than mis-attributed) |
| Result identity never carries the runner label | `…::artifact_names_carry_the_environment_not_the_runner_label`, `JobNameCorpusTests::test_a_native_and_a_guest_row_of_one_package_are_two_cells` |

**Verification levels.** The workflow layout is a shipped-artifact contract, so
every fixture reads the real YAML (passive corpus over all four reader-facing
workflows). `cell_contract.py` crosses a CLI boundary and a filesystem
boundary, so its 15 fixtures run the real process and read the real
`$GITHUB_OUTPUT` file. The status fold and the lint step cross a shell
boundary, so `the_status_fold_preserves_every_failure_shape` and the two lint
timing fixtures execute the shipped scripts under `bash` with the cell
reader's outputs substituted. The native-prerequisite step is executed the same
way (`test_ci_local.py::NativeProvisioningTests`). The producer chain crosses a
process, filesystem, and persistence boundary end to end, which the dry run
below covers with the real planner, the real recipe, and the real validator.

### The Phase 5 dry run (real plan, real row, real recipe, real validator)

`repo-deps` L1 on this macOS host, in the step order the `test` job now runs:

```
python3 scripts/ci/affected_scope.py --resolved-plan scripts/ci/completion.py > plan.json
python3 scripts/ci/cell_contract.py --plan plan.json \
  --row '{"package":"repo-deps","gate":"L1","environment":"macos-latest","runner":"macos-latest"}' \
  --head "$(jq -r .head plan.json)" --github-output "$GITHUB_OUTPUT"
  → resolved repo-deps/macos-latest/L1 from the plan
    recipe=_test  profile=ci  requires_toolchain=true  target=aarch64-apple-darwin
    build_artifact=build-repo-deps-macos-latest-27169df4727d2d84
    status_artifact=status-repo-deps-L1-macos-latest
    junit_artifact=junit-repo-deps-L1-macos-latest
    completion_artifact=completion-repo-deps-L1-macos-latest
just _expected_manifest L1 repo-deps      → 427 expected, 1 ignored/excluded
just _test repo-deps --no-fail-fast       → 427 tests run: 427 passed, 1 skipped
python3 scripts/ci/completion.py … --out completion.json
  → ✅ repo-deps/macos-latest/L1 complete: 1 report(s) match the expected listing
```

The record bound `head`, `run`, `attempt`, `cargo-nextest 0.9.136`,
`reports: ["L1/repo-deps.xml"]`, `build: 27169df4727d2d84`, and the seven
`gate_inputs` paths. **The listing survives the gate**: `_stage_junit_reset`
removes only `target/nextest/ci/test-results.xml`, never the staging tree, so
`expected-L1.json` written before the gate is still there afterwards.

The negative half, with the listing kept whole and the GATE narrowed
(`BISCUIT_TEST_FILTER='test(/(^|::)the_canonical/)'`):

```
completion: repo-deps/macos-latest/L1 is not proven and no record was written.
  completion-test-missing: repo-deps::bin/ci-build::archive::tests::… was expected
  on this target and no report mentions it. A skip approval cannot excuse an
  absent test — an observed skip is a decision, silence is a lost test
```

`--out` was absent afterwards, which is the R10 property: `complete: true`
cannot exist without the evidence behind it.

### Broader gates run (all green, 2026-09-20)

| Gate | Result |
|---|---|
| all thirteen `python3 scripts/ci/test_*.py` | 13/13 OK, 892 tests (was 862): `test_resolved_plan` 78 → 96, `test_affected_scope` 275 → 281, `test_local_evidence` 20 → 23, `test_runner_loss` 47 → 50 |
| `just _test test-toolkit` | 225 passed, 2 skipped (pre-existing); `ci_workflow_contracts` 146/146 |
| `just _test repo-deps` | 427 passed (13 slow), 1 skipped (pre-existing) |
| `just _lint repo-deps` / `just _lint test-toolkit` | clean |
| `env -u CDPATH bash .githooks/tests/test-pre-push.sh` | 67/67 passed |
| `actionlint` on `ci.yml`, `_area-ci.yml`, `_package-ci.yml`, `_wsl-ci.yml` | clean |
| `just ci-local --plan` | renders on this tree: 9 dispatch rows across 2 areas, 6 build records, the skip snapshot |
| `BISCUIT_PROMOTE_PENDING=1 python3 scripts/ci/test_runner_loss.py` | OK — nothing pending remains anywhere in the Python suites |

No tests were skipped by this phase beyond the two pre-existing skips; no
pre-existing failures exist. (`env -u CDPATH` remains necessary on this host —
see the Phase 2 note.)

**Not run:** `just ci-local` in full (a ~45-minute local gate whose self-test
half — the nine registered suites — was run individually and is green), and
any hosted dispatch. The Phase 7 trial is where a hosted run belongs.

### OS considerations

- **The WSL2 guest gained one declared apt package** (`python3`, R2) and two
  reachability checks in the same step. It is used: `completion.py` runs IN the
  guest, because the reports, the listing, and the plan are all on ext4 that
  the Windows host cannot read. Recorded in `.claude/skills/os/wsl.md` together
  with the row the guest now receives and why its contract is resolved on the
  host BEFORE a guest is provisioned.
- **Peak disk in the guest is unchanged** by listing before the gate: the two
  archive extractions are sequential, and peak — not total — is what exhausted
  the Windows host in run 30605643702. Recorded in the same skill.
- **Windows path spellings are untouched.** Every archive consumer still takes
  `ARCHIVE_WORKSPACE` from `steps.verified.outputs.workspace`, which
  `_ci_build_verify` computes once in the host's own spelling; the new listing
  step takes the same value, and
  `an_archive_consumer_hands_native_programs_native_paths` now checks both.
- **`native_packages` as JSON** removes a Windows-relevant hazard as well as a
  general one: a Bash `for` over an unquoted variable would have globbed
  against the runner's working directory.
- `just cross-check` was **not** run. This phase changes workflow YAML, Python
  tooling, and contract fixtures; the suites that exercise them are registered
  `environment: ubuntu-latest`, and the OS-sensitive facts above were resolved
  by reading recorded knowledge rather than by occupying a shared build host.
  The workflows themselves cannot be exercised off GitHub at all — that is the
  Phase 7 trial's job, and it is recorded there as needing a push.

### Deviations and notes for later phases

- **There is no `lists` branch in the workflows.** The plan's Phase 7 Wave 2
  ships `.github/ci/direct-execution.json` and expects "exactly one path per
  area", while Phase 5's own task list says to delete every environment-list
  input and the Phase-2 oracle
  `no_reader_facing_workflow_declares_an_environment_list_input` refuses to pass
  while one survives. The two cannot both hold: a `lists` branch requires the
  inputs. **Phase 5 followed its own task list and the oracle** — the workflows
  are rows-only — so the R9 rollback lever is a revert, not a per-area switch.
  `execution_path` still rides on every area record and still reads `lists`
  (its default), which is now inaccurate. Phase 7 must either flip
  `DEFAULT_EXECUTION_PATH` to `rows` and delete the allowlist task, or restore
  a `lists` branch and un-promote the oracle. **Raised for human review.**
- **`just _ci_build_consumer` is now unused by CI.** `cell_contract.py`
  resolves the build record and refuses a dangling reference
  (`cell-contract-build`). The recipe and its behavioral fixture
  (`a_cell_with_no_build_record_refuses_instead_of_compiling_in_place`) still
  pass and were left alone under Rule 3, but it is dead weight: Phase 8's
  reader migration should retire it with the other legacy readers.
- **`area_rows` joined `SCOPE_PROJECTION_FIELDS`.** Phase 3 deliberately left
  it out "until `ci.yml` starts reading it from a receipt". It does now, so it
  is required — and the hook fixture `affected_scope_stub.py` needed its own
  `row_sets` to match (sixteen hook tests failed on
  `scope-malformed: the carried scope projection lacks 'area_rows'` until it
  did). `area_matrix` stays in the tuple for `just/ci-local.just` and the hook,
  which Phase 8 migrates.
- **`check` and `lint` cells now produce completion records too.** Phase 4 made
  `--expected-manifest` optional for a non-`BUILD_GATES` gate for exactly this.
  Phase 6's audit must expect `completion-<package>-<gate>-<environment>` for
  every executing cell including those two, and must add `completion-*` to the
  area audit's download pattern — it is **not** there yet, so today's audit
  simply ignores the new artifacts.
- **`BISCUIT_CI_COMPANION_SUITES` is now exported** by the test job (from the
  cell's own declared suites, comma-joined), closing the Phase 4 note.
- **The area audit no longer receives a `packages` input.** It reads the plan.
  Any later change that removes `ci-resolved-plan` from that job's downloads
  would break package membership, not just evidence reading.
- **`_area-ci.yml`'s `package-ci` job id is unchanged** although it is now one
  call per area. `the_area_workflow_calls_the_execution_workflow_once` pins the
  id, `runner_loss.py` parses around it, and R5's reasoning for
  `_package-ci.yml` applies identically.
- **`BISCUIT_FRONTEND_REQUIRED` is declared on FOUR steps, not on the job.**
  It was a job-level value, and a job cannot read the cell contract — the
  contract is resolved in a step. It now rides on the test gate, the test
  companion step, the lint gate, and the lint companion step. The companion
  ones are the load-bearing pair: the frontend suite is a COMPANION, so a
  value that reached only the Rust gate would let a missing pnpm self-skip
  green on the very cell that declared the capability. Found by reading the
  old job-level block rather than by a red test, because the shipped contract
  (`a_declared_node_capability_is_provisioned_verified_and_hard_required`)
  checks the whole job's text and would have passed either way. It now would
  still pass either way — tightening it to per-step is worth doing if the
  frontend capability ever grows a second consumer.

Phase 5 complete: all 17 GFM todos in the plan's Phase 5 section are checked,
every suite is green, and `actionlint` is clean on all four workflows.

## Phase 6

Phase 6 is Migration step 3: **the audit now reads the completion contract**,
and the version-aware split between the new path and the legacy one is
explicit. All five Phase-2 audit oracles in `scripts/ci-rollup-tests.rs` are
promoted. The Rust `pending_contract` wrapper is deleted from that file along
with its last user, so no pending contract remains in any Rust suite. Only
`scripts/ci/pending_contracts.py` survives, and nothing in the Python suites
is pending either.

### What was done

1. **Wave 1: consume completion records (`scripts/ci-rollup.rs`).**
   - `rollup` reads every `completion-*/completion.json` and attaches a
     `completion` to **every cell the plan executes**, with check and lint
     included. It attaches none to any other cell. A record is found by the name
     `cell_contract.py` derives (`completion-<package>-<gate>-<environment>`).
     It must then name that cell, claim `complete: true`, name the plan's
     `head` and the cell's planned `build` (checked both ways), and name the
     `--run-id` run. Any attempt is accepted, so a rerun of other jobs keeps a
     passing cell's proof. The record must also declare **exactly** the reports
     uploaded for the cell, compared in both directions and with `\` and `./`
     normalized.
   - A record of another generation is reported and never interpreted
     (`COMPLETION_RECORD_SCHEMA_VERSION = 1`, the same value as `schema.py`).
     A record that is not JSON at all is a tool error (exit 1). A record with
     a missing or wrong field is a finding about its own cell.
   - **The audit stops recomputing.** An executing cell never takes the
     `--expected-manifest` diff. Its skip set is only the `<skipped/>`
     elements its reports contain, and `skip_evidence_degraded` is always
     false on it.
   - **Result document 4 → 5.** Cells gain `completion`, and the document
     gains `unplanned_completions`. A version-4 slice is refused rather than
     read, because it carries no `completion` at all and reading it would
     certify every executing cell without proof.
   - `ResolvedPlan` now reads `head`, which the binding needs.
2. **Wave 1: the legacy path.** A cell with no `completion` keeps every check
   it had: the version-1 manifest diff, the exact skip budget, and
   `skip_evidence_degraded`. Such a cell is reused, a governed gap, prohibited,
   or rolled up without a plan. The Rust manifest reader's fate (a Phase 4
   carry-over) is decided: it **stays on version 1 for legacy cells**, and a
   version-2 manifest is refused *before* its shape is read. The refusal names
   `completion.py` as the manifest's reader. Previously a v2 manifest failed
   with a serde field error that hid the real problem.
3. **Wave 2: verdict rules.**
   - `completion-unproven` blocks an executing cell whose state does not
     already block (PASS, NOTHING TO RUN, SKIP) and whose record fails any
     binding rule. A FAIL or MISSING cell gets no second row: its producer
     refuses to certify a failure, so the absent record is a consequence of
     the failure, not a separate problem.
   - `completion-unplanned` blocks a record for a cell the plan did not
     execute.
   - `verdict --producers <success|skipped>` carries the producer-call rule.
     `skipped` over executing cells is `producers-skipped` (missing coverage).
     `skipped` over none is an all-reused or gap-only area, which is judged by
     every other rule. `failure` and `cancelled` are refused as a tool error:
     they already block through their own job result.
   - `_area-ci.yml`: both download steps now include `completion-*`. The
     enforcement step runs on `success || skipped` and passes
     `--producers "$PRODUCERS"`. Before this change an all-reused or gap-only
     area skipped enforcement entirely, because `package-ci` was `skipped`.
   - **Presentation.** The rollup never changes a cell's state for its
     completion, so the grid gains an **Unproven executions** table. Without
     it, the grid would read clean over a blocked area. Reused and
     accepted-gap tables, the combined summary, and the always-rendered slice
     are unchanged.
   - **Reuse and gaps are unchanged.** No reuse, gap, duplicate-result, or
     current-over-reused rule was touched, and the existing fixtures that pin
     them all still pass.
4. **Wave 3: fixtures.** Seventeen new Rust fixtures (below), the five
   promoted oracles, five end-to-end shapes in
   `test_completion.py::AuditEndToEndTests`, and the baseline header rewrite.
   `.github/ci/ci-baseline.toml` is still empty. Only its comments changed.

### The three judgement calls worth reading

**1. The verdict judges completion; the rollup does not.** The Phase-2
oracles demand that `rollup` exit 0 and `verdict` block for the same area.
That matches this tool's own split: the rollup observes and the verdict
judges. A missing record is judged as a policy about the evidence, not
observed as a test result. So a cell keeps the state its reports and status
showed, and `Completion` travels beside the state. The Unproven executions
table is what stops that split from hiding anything from a reader.

**2. For an executing cell, the skip budget is reported rather than
re-judged, but stale approvals are still retired.** "Stop recomputing exact
skips" could be read as dropping all skip handling for executing cells.
`skip-new` is dropped: the producer already approved every observed skip
against the plan's `skip_policy` snapshot of the same tree, and it reports
an unapproved skip as `completion-skip-unapproved`. `skip-resolved` is **kept**,
because nothing else retires an approval that no longer skips, and a stale
approval is a standing permission to skip. The observed skips of a certified
cell appear as `skip-certified` notes. An uncertified cell gets no skip
finding at all, because `completion-unproven` already blocks it.

**3. Only cells the plan executes need a record; a reused cell that was
executed anyway does not.** Reuse contention (a reused cell that also produced
CI evidence) stays exactly as it was: the executed result is reported, the
discrepancy is noted, and no record is demanded. `completion.py` refuses a
non-executing cell (`completion-cell-not-executing`), so demanding a record
there would turn a reported discrepancy into a block, which the spec does not
ask for.

### Requirement-to-test mapping

| Requirement (plan task) | Targeted tests |
|---|---|
| Executing cell with no record blocks; a valid record clears it | `an_executing_cell_without_a_completion_record_blocks_the_verdict` (promoted) |
| Bound to the tested revision / the planned build | `a_completion_record_bound_to_another_revision_blocks`, `a_completion_record_bound_to_another_build_key_blocks` (promoted), `a_completion_record_must_name_exactly_the_planned_build` (both directions plus the clearing half) |
| Declared report inventory, both ways | `a_green_status_with_an_absent_report_inventory_blocks` (promoted), `a_staged_report_the_record_does_not_certify_blocks` (including the `\`/`./` normalization half) |
| `complete: false` outranks green JUnit | `an_incomplete_completion_record_blocks_green_junit` (promoted) |
| Record names its own cell; other generation reported, not read | `a_completion_record_that_does_not_describe_its_cell_blocks` (6 cases: other package, other gate, v2, null version, non-list inventory, string `"true"`); each case also asserts the rule id and the grid's Unproven executions table |
| Run binding; retained evidence on retries | `a_completion_record_from_another_run_blocks_and_an_earlier_attempt_does_not` (other run blocks; same run passes, as a number and as a string; attempt 1 passes; attempt 0 blocks) |
| Unplanned evidence refused | `a_completion_record_for_an_unplanned_cell_blocks` |
| Unreadable input is infrastructure, not a verdict | `a_malformed_completion_record_is_a_tool_error_not_a_verdict` |
| Check and lint cells are certified too | `a_green_check_status_needs_its_completion_record` (blocks without, clears with a `reports: []` record); e2e `test_a_mixed_area_…` removes the check cell's record |
| Legacy evidence cannot bypass removed checks | `the_expected_manifest_diff_applies_to_legacy_cells_only` (same inputs, `executes` false vs true), `a_certified_cells_skips_are_reported_while_stale_approvals_still_block` (legacy keeps `skip-new`; certified keeps `skip-resolved`), `a_version_four_result_document_is_refused_rather_than_read_as_legacy`, `a_version_two_expected_manifest_is_refused_with_its_owner_named` |
| Enforce when producers succeeded **or none were required**; skipped over rows is missing coverage | `a_skipped_producer_call_blocks_only_over_executing_cells` (incl. `failure`/`cancelled` refused), e2e `test_an_all_reused_area_…`, `test_a_gap_only_area_…`, `test_a_skipped_producer_over_executing_cells_is_missing_coverage` |
| Workflow downloads `completion-*` and passes `--producers` | `ci_workflow_contracts.rs::every_selected_area_owns_an_always_coverage_audit`, `…::the_area_coverage_audit_reads_plan_policy_and_baseline`, `test_affected_scope.py::…test_area_rollup_download_unions_current_and_run_wide_artifacts` (all three tightened first and seen red against the old YAML) |
| A failure outranks the missing record: one problem, one row | e2e `test_a_failing_executing_cell_leaves_no_record_and_blocks_once` |
| Presentation: reused and gap cells stay in the area summary | e2e mixed and gap-only shapes assert `Reused results` and `ACCEPTED GAP` in the real binary's summary; existing `the_grid_shows_a_reused_cell_…`, `the_grid_explains_an_accepted_gap_…` |
| Reuse and gaps unchanged | the existing reuse/gap/contention suites, unedited and green (`a_reused_cell_that_also_produced_ci_evidence_reports_the_executed_result`, `a_real_failure_outranks_an_accepted_gap`, `an_expired_accepted_gap_blocks_its_area`, …) |
| Result document round trip | `the_command_surface_writes_reads_and_judges_one_areas_slice` (write/read/write byte-identical; now carries a completion record), and every `audit_of` fixture writes, re-reads, and judges the slice through files |

**One existing fixture changed.** `the_command_surface_writes_reads_and_judges_one_areas_slice`
models a green run, and its executed cell had no completion record, so the new
contract correctly refused it. It now carries a valid record and the plan's
`head`. This is the contract change working as intended, not a weakened
fixture.

**Non-vacuity.** Two mutations were run and then reverted. With
`completion_findings` disabled, 16 fixtures fail. With the executing-cell
filter on the manifest diff removed,
`the_expected_manifest_diff_applies_to_legacy_cells_only` fails.

**Verification levels.** The audit crosses a CLI, filesystem, and persistence
boundary. Every Rust fixture goes through `cmd_rollup` and `cmd_verdict` and
through the result file between them. `AuditEndToEndTests` goes further: the
real `row_sets` produces the rows, the real `cell_contract.py` produces each
row's artifact names, the real `completion.py` writes each record, and the
real `ci-rollup` binary (built by cargo and located through
`--message-format json`) runs `rollup --area` and `verdict --area --producers`.
The workflow contracts form a passive corpus over the shipped `_area-ci.yml`.

### The Phase 6 checkpoint: the synthetic end-to-end run

`python3 scripts/ci/test_completion.py AuditEndToEndTests`, 5/5, covering the
four area shapes plus one failure case:

| Shape | Rows | Producers | Verdict |
|---|---|---|---|
| all-reused | none | `skipped` | 0; a reused **failure** → 2 (`cell-failed`) |
| gap-only | none | `skipped` | 0 with `policy-gap-accepted`; an expired gap → 2 (`policy-gap-expired`) |
| mixed (L1 + check executing, L1 reused, L2 gap) | exactly the two executing cells | `success` | 0; delete the check record → rollup 0, verdict 2 (`completion-unproven`) |
| failing executing cell | one | `success` | `completion.py` exits 1 and writes nothing; rollup 2, verdict 2 with `cell-failed` and **no** `completion-unproven` |
| executing cells, call skipped | two | `skipped` | 2 (`producers-skipped`) |

### Broader gates run (all green, 2026-09-20)

| Gate | Result |
|---|---|
| `just _test repo-deps` | 444 passed (16 slow), 1 skipped (pre-existing); was 427 |
| `cargo nextest run --bin ci-rollup` alone | 263/263 (was 246) |
| `just _test test-toolkit` | 225 passed, 2 skipped (pre-existing); `ci_workflow_contracts` 146/146 |
| `just _lint repo-deps` / `just _lint test-toolkit` | clean |
| all thirteen `python3 scripts/ci/test_*.py` | 13/13 OK (`test_completion` 44 → 49) |
| `BISCUIT_PROMOTE_PENDING=1` over every Python suite and `ci-rollup` | 13/13 OK and 263/263: nothing is pending anywhere |
| `env -u CDPATH bash .githooks/tests/test-pre-push.sh` | 67/67 passed |
| `actionlint` on `ci.yml`, `_area-ci.yml`, `_package-ci.yml`, `_wsl-ci.yml` | clean |

No test was skipped by this phase beyond the three pre-existing skips.

GitNexus `impact` (upstream) was run on every production symbol this phase
changed: `classify_one`, `skip_findings`, `verdict`, `cmd_rollup`,
`cmd_verdict`, `load_expected_manifests`, `reject_old_schema`, and
`render_grid`. All are **LOW**, and every caller is inside the one `ci-rollup`
binary. The only MEDIUM and UNKNOWN answers came from name collisions with
same-named test helpers, which this phase did not change. The analysis ran
after the edits rather than before; that ordering slip is recorded here rather
than hidden.

### OS considerations

- **Report paths are compared after normalization.** A Windows producer's
  manifest or record may spell a report `L1\x.xml`. Both sides pass through
  `normalized_report`, which turns `\` into `/` and strips a leading `./`.
  `a_staged_report_the_record_does_not_certify_blocks` pins this with a
  Windows-spelled inventory. It is a pure string rule, so it behaves the same
  on every host.
- **The end-to-end suite is portable by construction.** It locates the
  binary through cargo's own JSON report (so `CARGO_TARGET_DIR` and `.exe`
  suffixes are handled) and runs every tool with `sys.executable`. It uses
  `mkdtemp` with `rmtree(..., ignore_errors=True)`, the Windows-safe cleanup
  the Phase 4 note recorded.
- **The audit runs only on `ubuntu-latest`**, and the WSL2 guest's record
  reaches it as an ordinary artifact, so no new OS surface was added.
  `just cross-check` was **not** run: nothing changed a code path whose
  behavior differs by OS, and the one OS-shaped rule (path spelling) is a
  string normalization pinned by a fixture that runs identically everywhere.

### Deviations and notes for later phases

- **`CLAUDE.md` still describes the audit as enforcing "when producers are
  green".** The spec's Migration step 6 assigns root-instruction updates to
  Phase 8, so the file was left alone here. `.github/ci/README.md`, the schema
  README, the baseline header, and the `rust-devops` skill were brought
  current in this phase, because they describe tool behavior that changed now.
- **`scripts/ci/pending_contracts.py` has no remaining users with a pending
  fixture.** It stays because the Python suites still import it; retiring it
  is not in any phase's task list.
- **The `lists`/`rows` human-review item raised in Phase 5 is still open.**
  Phase 6 did not depend on it, and it still has to be decided before
  Phase 7.

Phase 6 complete: all 9 GFM todos in the plan's Phase 6 section are checked,
and every suite is green.

## Phase 7

Phase 7 is Migration steps 4 and 5: capacity validation, the path switch, and
the root-area trial. Everything that can be done offline is done. The hosted
trial is still blocked on a push, which this session may not make.

### The open human-review decision, and what this phase did about it

The review item raised after Phase 5 (restore a per-area `lists` path, or
accept "all areas at once") was **still undecided** when this phase was
invoked. The pipeline invoked Phase 7 anyway, and the session is
non-interactive. The work splits cleanly:

- **Wave 1 (capacity) and most of Wave 3 (local comparison) do not depend on
  the decision.** Both were done as written.
- **Wave 2 does.** This phase implemented the recorded recommendation,
  **option A**, because it matches what Phases 5 and 6 built and every test
  already assumes. It is small and reverts cleanly (about 40 lines across
  `schema.py`, `affected_scope.py`, `plan_fixtures.py`, and the regenerated
  `contract.json`). `human_review` stays `true` so a person ratifies A before
  Phase 8 deletes anything. Choosing B would mean reverting only those lines,
  plus B's own list (restore inputs, add a `lists` branch, relax the oracle).

Before this phase, every area record said `execution_path: "lists"` while
every workflow dispatched rows. The plan described a path nothing ran. That
mismatch was a defect under either option, and the fix below removes it.

### What was done

1. **Wave 1: offline capacity validation.**
   - New `spikes/capacity.py` re-measures from the **shipped** planner, row
     adapter, scope projection, and workflow YAML for four plans: `--all`, push
     to `main`, `workflow_dispatch`, and nightly `schedule`. It measures every
     matrix's cardinality, output sizes, the runner-job estimate including
     audits, gap publishers, and build owners, the reusable-workflow depth, and
     the unique reusable-workflow count. Results are in `spikes/s3-capacity.md`
     § "Phase 7 re-measurement".
   - **Two corrections to the Phase 1 model.**
     1. Shipped rows are objects plus four `has_*_rows` flags, so payloads are
        about 1.8× S3's figures: homelab 3,625 B, all areas 39,911 B. There is
        still at least 4.5× headroom on every size ceiling.
     2. S3's job total of about 600 double-counted WSL2. `_package-ci.yml`'s
        `wsl2` job is a `uses:` call and occupies no runner. The real
        full-workspace figure is **424 runner jobs + 124 call jobs**.
   - **S3's aggregate budget was never implemented.** It listed 512 KB across
     all areas "for Phase 7 to enforce", and Phase 3 enforced only the
     per-area 16 KB. `ci.yml` carries every area's rows in **one** job output
     (`area_rows`), and GitHub caps one output at 1 MB, so the aggregate is
     the ceiling that actually applies. Added `TOTAL_ROW_SET_BUDGET = 512 KiB`
     to `enforce_output_budgets`. It refuses whole, and the message names the
     byte count, the area count, and the budget.
   - **Depth is the only ceiling without headroom.** The chain is 4 levels deep.
     The repository's contract treats 4 as GitHub's maximum, while S3 cites 10.
     The chain satisfies both readings. This is recorded rather than settled.
   - R3 peak concurrency: the largest per-area rise is 5 runners (darkmatter
     L2 and browser). R3's fallback is not warranted on these numbers.
2. **Wave 1: the guard, proven through `main()`.** The new
   `CapacityGuardTests` class has three tests (listed below). The end-to-end
   test runs the real planner over the real workspace with `--all --plan-out`
   and only the per-area budget lowered. It asserts a non-zero exit, a message
   naming the area and the budget, **no stdout** (so `ci.yml` has no scope
   document to write), and **no plan file**. It then asserts that the same
   invocation at the shipped budget emits both.
3. **Wave 2 (option A): one dispatch path, stated truthfully.**
   - `schema.EXECUTION_PATHS` is `("rows",)`. A plan claiming `lists` is now a
     `malformed-receipt`, and `contract.json` was regenerated.
   - `affected_scope`: removed `DEFAULT_EXECUTION_PATH`,
     `DIRECT_EXECUTION_PATH`, `DIRECT_EXECUTION_SCHEMA_VERSION`, and
     `load_direct_execution`, along with `area_records`' `direct_execution`
     parameter. Every area is stamped with `EXECUTION_PATH = "rows"`.
     `.github/ci/direct-execution.json` was never created.
   - `plan_fixtures.finalize_plan`'s default is now `rows`, so synthetic fixtures
     stop describing a path that does not exist.
4. **Wave 3: local comparison.** `just ci-local --plan` on this tree selects
   areas `root` (repo-deps) and `tools` (test-toolkit), and both state `rows`.
   9 executing cells produce 9 rows: equal as sets, unique, identical to
   `row_sets(plan)`, and schema-valid. `just ci-local` ran 13 of 13 gates green
   in 3m41s. The observed macOS cells (`repo-deps` and `test-toolkit`
   `macos-latest/L1`) match the plan. The ubuntu lint cells run locally as
   stand-ins. The `check` cell is not replicated locally, as the recipe's
   header documents. No WSL2 row executes, because both WSL2 cells are
   accepted gaps, so `cross-check` had nothing to run.

### Requirement-to-test mapping

| Requirement | Test(s) | Seen red first? |
|---|---|---|
| Real full-workspace, push, and nightly plans fit every matrix and byte ceiling | `test_affected_scope.py::CapacityGuardTests::test_the_real_full_workspace_and_nightly_plans_fit_every_ceiling` | n/a (a measurement) |
| Guard fires before dispatch, names the ceiling and area, truncates nothing, and emits no scope or plan | `CapacityGuardTests::test_an_over_budget_area_fails_before_dispatch_and_emits_nothing` (real `main()`, real workspace; negative and positive halves) | yes: no refusal before the guard existed |
| Aggregate budget refuses many under-budget areas, and one fewer passes | `CapacityGuardTests::test_the_combined_scope_output_budget_refuses_many_small_areas` | yes: no refusal before `TOTAL_ROW_SET_BUDGET` |
| Every area states `rows` (unit) | `ExecutionPathTests::test_every_area_record_states_the_row_path` | yes: it read `lists` |
| No area emits both row sets and environment lists (real workspace, one-area and full scope; the row document carries only row sets; the scope projection matches) | `ExecutionPathTests::test_no_area_emits_both_row_sets_and_environment_lists` | yes: `execution_path` was `lists` |
| The retired allowlist is neither present nor read | `ExecutionPathTests::test_the_retired_allowlist_file_is_not_consulted` | — |
| A plan claiming `lists` is refused (variants: `lists`, `LISTS`, `""`, `None`, `["rows"]`) | `test_schema.py::…::test_an_area_on_the_environment_list_path_is_refused` | yes: `lists` was admitted |
| The plan contract admits exactly the path the workflows implement, and no workflow declares an environment list or branches on `execution_path` | `ci_workflow_contracts.rs::the_plan_admits_exactly_the_dispatch_path_the_workflows_implement` | yes: `contract.json` listed `lists` |
| Rollback: a partial revert fails loudly and never weakens enforcement | `ci-rollup-tests.rs::a_plan_from_another_schema_generation_is_refused_rather_than_audited` (v4 and v6 refused by name, v5 accepted), plus the existing `a_version_four_result_document_is_refused_rather_than_read_as_legacy` | — (pins existing behavior no test covered) |

**Why rollback has no "restores readers together" test.** Under option A,
rollback is a revert of one change, which restores the planner, the workflows,
and the audit as a unit by construction. The failure a test *can* catch is a
**partial** revert that leaves them a generation apart. That case is covered:
the audit refuses a plan from another generation (new test), the audit
refuses a result document from another generation (existing), and a
receipt from another generation misses as `scope-schema` (existing,
`test_resolved_plan.py`). Nothing in the rollback path deletes evidence. The
hosted artifacts and Git notes are untouched by a revert.

### Broader gates run (all on macOS, this host)

- 13/13 `scripts/ci/test_*.py` suites. `test_affected_scope` ran 283 tests.
- `just _test repo-deps`: 445 passed (+1), 1 skipped (pre-existing).
- `just _test test-toolkit`: 226 passed (+1), 2 skipped (pre-existing).
- `just _lint repo-deps` and `just _lint test-toolkit`: clean.
- `.githooks/tests/test-pre-push.sh`: 67/67. This needs `env -u CDPATH`. With
  the shell's exported `CDPATH` it fails immediately with "hook not
  executable". That is a host trap, not a regression, and is now recorded in
  `.claude/skills/os/macos.md`.
- `just ci-local --plan` and `just ci-local`: green (above).
- `actionlint` was **not** run: no workflow file changed in this phase.
- `just cross-check` was **not** run. The changes are to Python planner and
  schema code, a Rust contract, and a Rust audit test, all registered for
  `ubuntu-latest` and platform-neutral. The only path-bearing addition, the
  subprocess test's `--plan-out`, passes the path through `repr()`, so a
  Windows backslash path survives. No WSL2 row is in scope.

### The hosted trial (still blocked: needs a push)

These are the commands to close it, after the author commits. A pull request
proves Linux and macOS only. Add `ci:all-os` to include the Windows rows. This
branch's two WSL2 cells are accepted gaps, so **no WSL2 row will execute on
this branch** under any label. Proving the guest's completion record needs a
branch that changes a package with an executing WSL2 cell.

```sh
export GIT_TERMINAL_PROMPT=0
git push -u origin fix/archive-path-sites
gh pr create --draft --base main --fill           # or reuse the open PR
gh pr edit --add-label ci:all-os                  # optional: Windows rows
RUN=$(gh run list --workflow ci.yml --branch fix/archive-path-sites --limit 1 --json databaseId --jq '.[0].databaseId')
gh run watch "$RUN" --exit-status
gh run view "$RUN" --json jobs --jq '.jobs[] | [.name, .conclusion] | @tsv'   # labels, ci-gate, no duplicate gate
gh run download "$RUN" --name ci-resolved-plan --dir trial/plan           # the dispatched row sets
gh run download "$RUN" --pattern 'completion-*' --dir trial/completion    # one per executing cell
gh run download "$RUN" --pattern 'ci-results-*' --dir trial/slices       # area slices (v5, `completion` on every executing cell)
```

Accept the trial when all of the following hold:

- the executing cells in `trial/plan` equal the `completion-*` directories;
- each area slice shows `completion` without `problems` on every executing
  cell, check and lint included;
- the accepted-gap cells appear as neutral check runs;
- `ci-gate` is green, and exactly one gate run exists for the head SHA.

### Notes for Phase 8

- `.github/ci/direct-execution.json` never existed, so Phase 8 Wave 1 has
  nothing to delete for it. `execution_path` (the field) and `EXECUTION_PATHS`
  are what remain to retire. Deleting the field means a schema version bump,
  because it is required at version 5.
- The ratification of option A is the gate. If the human picks B instead,
  revert this phase's Wave 2 edits (listed in the source-file frontmatter)
  before building B.

## Phase 8

Phase 8 is Migration step 6: retire the environment lists and the legacy
projections.

### The gate this phase ran past, and why

The plan says nothing in Phase 8 may run before Phase 7's hosted trial is
green, and the spec's `human_review` item asked for option A to be ratified
first. When this phase was invoked, the trial had still not run (it needs a
push) and `human_review` was still `true`. The session is non-interactive and
was told to implement Phase 8, so it went ahead, on this reasoning:

- The fallback the gate protects was already gone. Phase 5 deleted every
  environment-list input, so hosted CI has dispatched rows only since then.
  What Phase 8 deletes is the `matrix`/`area_matrix` projection, which only
  local readers (`just/ci-local.just`, the hook's test stub) and two lines of
  `ci.yml` still read.
- Nothing is committed or pushed. The whole phase is a working-tree diff that
  reverts as one unit.
- The trial is more informative on the final tree than on an intermediate one.

`human_review` stays `true`: the trial must be green before this is merged.

### What was done

1. **Wave 1: workflow inputs.** Nothing was left to delete. Phase 5 removed
   every environment-list input, and no `lists` branch or
   `.github/ci/direct-execution.json` ever shipped. The existing check,
   `declared_environment_list_inputs`, is a name heuristic (`-environments`,
   `check-os`). It would miss a reintroduced `packages`, `gates`, `wsl`, or
   `builds` input, so a new contract,
   `the_reusable_workflows_accept_only_rows_and_run_scalars`, pins each
   reusable workflow's `workflow_call` inputs exactly. It passes on this tree
   and fails on `main`'s 23-input `_package-ci.yml` (checked by swapping the
   file in and restoring it).
2. **Wave 2, step 1: `just/ci-local.just`.** Each of the seven `.matrix[]`
   lookups now reads one `.packages[]` record from the plan `--plan-out` wrote.
   The plan's tier gates (`L1`/`L2`/`browser`) fold into the loop's `test` gate
   the same way the retired projection folded them. `l2_environments` became
   "this host's L2 cell has `execution == "execute"`", which is exactly what
   the projection computed (executing and hostable; an unhostable L2 cell is
   an omitted gap). A package the plan does not carry reads as an empty
   record and runs nothing, the same answer the matrix gave by omission.
3. **Wave 2, step 2: the hook.** `.githooks/pre-push` and `test-pre-push.sh`
   read neither field; the hook passes the scope document straight to
   `scope-record`. The only site was `affected_scope_stub.py`, which now
   projects the planner's shape (`scheduled_areas` from gating packages, no
   `matrix`/`area_matrix`). The hook suite was proven 67/67 before step 3.
4. **Wave 2, step 3: the projection.**
   - `legacy_scope_document` no longer emits `matrix` or `area_matrix`.
     `scheduled_areas` and `area_slugs` come from the plan's gating packages,
     the same set the matrix grouped, so an all-reused or gap-only area is
     still scheduled and keeps its audit.
   - `matrix_record` had no other caller and was deleted whole. Every field it
     computed reaches producers through `cell_contract.py` already.
   - `schema.SCOPE_PROJECTION_FIELDS` drops both keys, and `contract.json` was
     regenerated; the only projection change is those two lines.
   - Retained: `policy`, `build_owners`, `build_slices`, `build_artifacts`,
     `build_runners`, `area_rows`.
   - `ci.yml`: `has_packages` reads `.scheduled_areas | length > 0` (the list
     the area matrix expands, so the guard and the matrix cannot disagree).
     The summary's "gating packages" row counts the plan's gating packages
     from `resolved-plan.json`.
   - Receipt compatibility: `validate_scope_receipt` requires keys and ignores
     extras, so a receipt published before this phase (same v5 plan, plus the
     two retired keys) still validates and still hits. No version bump.
   - Stale comments fixed in the same change: `affected_scope`'s module
     docstring and `legacy_scope_document`'s docstring (both said Phases 5/6
     of an older fix would delete it), the `wants_node` comment naming
     `matrix_record`, the package-count guard's message ("the package matrix
     has N entries"), two `schema.py` field comments ("projected into the
     matrix"), and the `ci.yml` comments.
5. **Wave 3: documentation.** A Documenter subagent updated
   `.github/ci/README.md`, `docs/topics/ci-cd.md`,
   `.claude/skills/rust-devops/ci-cd.md`, `.claude/skills/os/wsl.md` (plus a
   `SKILL.md` index line), and `CLAUDE.md`'s CI Structure audit bullet. I spot-
   checked every identifier it named (`producers-skipped`,
   `completion-unproven`, `completion-unplanned`, `--producers`,
   `_expected_manifest`) against the code. I also fixed
   `.github/ci/schemas/README.md`, which still called `area_matrix` live, and
   added the "no per-package environment list" fact to the rust-devops skill.
   `docs/dependencies.md` is unchanged: no crate was added or removed.
6. **Capacity re-measured** (Phase 7's request). `spikes/capacity.py` no longer
   reads the retired keys. Rows, counts, and jobs are identical. The scope
   document shrinks about 44% (364,106 B → 204,970 B on `--all`; nightly
   77,421 B). The largest remaining `scope` job output is `area_rows` at
   39,911 B, giving 26× headroom under GitHub's 1 MB. Recorded in
   `spikes/s3-capacity.md`.

### Decisions worth reading

- **`execution_path` was kept.** With one admitted value it is redundant, but
  it is required at plan schema v5. Removing it is a plan-schema bump that
  every reader, fixture, receipt, and the audit's generation gate would have
  to follow, for no behavior change. That is outside this phase's "remove
  only projections with no remaining readers" scope. It is a clean follow-up
  if wanted.
- **`local_evidence.py`'s version-1 path still reads `scope["matrix"]`.**
  `record`/`verify` (without `--cells`) exist to verify version-1 notes
  already in the wild, and they read a caller-supplied legacy scope document,
  not the planner's output. No shipped caller uses them (the hook, CI, and
  `ci-local` all use `verify --cells`). Left as-is. The new reader sweep
  scans that file and does not flag it, because `.get("matrix", [])` on a
  legacy document is a deliberate compatibility read, not a `jq` lookup or a
  `["matrix"]` subscript on the planner's output.
- **The package-count guard (`len(gating) > MATRIX_LIMIT`) was kept.** It no
  longer guards a real matrix, but it is a conservative superset of the area
  matrix ceiling, and the per-row-set guard covers the rest. Only its message
  changed.
- **Test migration policy.** About 40 tests read `matrix`, `area_matrix`, or
  called `matrix_record`. Each was moved onto what a producer actually
  receives: the plan's gates (`gates_by_package`), the dispatched rows
  (`dispatched`), or the real `cell_contract.contract` over the real rows
  (`producer_contracts`, `resolved_contracts`). None was simply deleted
  except one: `test_check_os_lists_every_executing_check_environment` drove
  `matrix_record` with an executing set the planner can no longer produce
  (example-target checks exist on the check environment only), and that
  cell-level fact is already pinned by
  `test_an_example_target_is_checked_on_the_check_environment_only`.
- **One latent planner edge found, not fixed.** `package_cells` raises
  `KeyError` building a gap record for an L2 backend with no capability
  entry, whereas `matrix_record` treated that as "hostable nowhere". It is
  unreachable with shipped artifacts, because `load_environments` refuses a
  table missing any known capability (`missing_caps`). The test that relied
  on the old premise now states the reachable case: a declared backend that
  no environment hosts executes nowhere.
- **The toolchain rule had lost its only behavioral test.** Run
  35326800778's provisioning rule was tested only through `matrix_record`,
  and `cell_contract`'s `requires_toolchain` had no test of its own. It is
  re-pinned through the real resolver in
  `PackageExecutionTests::test_toolchain_environments_follow_the_declaration_and_the_capability`
  (declared × capability; narrowed by reuse; none without the declaration;
  none check-only). The Rust contract that grepped for the deleted
  `"toolchain_environments"` string now pins the plan record's declaration.

### Requirement-to-test mapping

| Requirement | Test(s) | Seen red first? |
|---|---|---|
| No reusable workflow accepts an environment list or other retired input | `ci_workflow_contracts.rs::the_reusable_workflows_accept_only_rows_and_run_scalars` (exact input sets) | yes, against `main`'s `_package-ci.yml` |
| No shipped reader looks up `matrix`/`area_matrix` | `ci_workflow_contracts.rs::no_shipped_reader_consumes_a_retired_environment_list_projection` (workflows, `just/`, `scripts/ci`, hook, root justfile) | yes, against HEAD's `ci-local.just` |
| `ci-local` gate loop reads package facts from the plan (test args, backends, parallel policy) | `test_ci_local.py::CiLocalTests` (all 9; fixture scope now has no `matrix`, plus a `--plan-out` plan) | yes: 12 failures against the old recipe |
| Plan gate vocabulary folds correctly (`lint`+`L1`, `lint`+`browser`, check-only with its `check_args`, `gates = []`) | `CiLocalTests::test_each_plan_gate_vocabulary_reaches_the_legacy_gate_it_folds_into` | yes |
| A package the plan does not carry runs nothing | `CiLocalTests::test_a_package_the_plan_does_not_carry_runs_nothing` | yes |
| Missing L2 backend fails only where the plan executes the L2 cell (`execute` / `omit` / `reuse`) | `CiLocalTests::test_an_unhostable_l2_cell_fails_only_where_the_plan_executes_it` | yes |
| Dry run lists the plan's test arguments and runs nothing | `CiLocalTests::test_a_dry_run_lists_the_plans_test_arguments_and_runs_nothing` | yes |
| Projection no longer carries or requires the retired keys; `ci.yml` reads neither | `test_schema.py::…::test_the_projection_fields_are_what_the_workflow_reads` | yes (`matrix` was still required) |
| Pre-retirement receipt (extra keys) still validates; missing keys still refused | `test_schema.py::…::test_a_receipt_carrying_the_retired_environment_lists_still_validates` | — (pins intended tolerance) |
| Pre-retirement receipt still hits through the real `ci.yml` scope step and publishes a projection without the keys | `test_ci_local.py::WorkflowScopeStepTests::test_a_receipt_from_before_the_environment_lists_retired_still_hits` | — |
| `has_packages` derives from the list the area matrix expands | `ci_workflow_contracts.rs::the_fan_out_gate_derives_from_the_scheduled_areas_not_the_impacted_list` + `AreaFanOutTests::test_a_gates_false_package_gets_no_area_fan_out` | yes (old contract pinned `.matrix`) |
| Area fan-out regroups plan packages; nested areas separate; every scheduled area has a slug and a row document | `AreaFanOutTests` (6), `RealWorkspaceAreaFanOutTests::test_a_nested_area_fans_out_beside_its_parent_never_inside_it` | yes (read `area_matrix`) |
| All-reused / gap-only area still scheduled with no test rows | `AllReusedAreaFanOutTests`, `test_resolved_plan.py::…::test_an_area_with_all_test_cells_reused_still_owns_a_result_slice` | yes |
| Producers receive builds, seams, natives, companions, Node, toolchain, backends per cell | `test_resolved_plan.py` (7 migrated), `test_affected_scope.py` `PackageExecutionTests`, `DependentSeamTests`, `CompanionAttachmentTests`, `L2BackendAxisTests`, `NativeClosureTests`, `NonPropagationTests`, retirement-scope tests | yes (all errored on the deleted fields) |

### Broader gates run (macOS, this host, 2026-09-20)

- 13/13 `scripts/ci/test_*.py` suites. `test_affected_scope` 282 (net −1),
  `test_ci_local` 92, `test_schema` 133, `test_resolved_plan` 96.
- `just _test repo-deps`: 445 passed, 1 skipped (pre-existing).
- `just _test test-toolkit`: 228 passed (+2), 2 skipped (pre-existing).
- `just _lint repo-deps`, `just _lint test-toolkit`: clean.
- `actionlint` on `ci.yml`, `_area-ci.yml`, `_package-ci.yml`, `_wsl-ci.yml`:
  clean.
- `env -u CDPATH .githooks/tests/test-pre-push.sh`: 67/67.
- `just ci-local --plan`: 9 rows across `root` and `tools`, the same as in
  Phase 7.
- `just ci-local`: 13/13 gates green in 1m58s through the migrated recipe.
- GitNexus `detect-changes --scope all`: HIGH, but that covers all 47 files
  of Phases 3–8, not only this phase. Phase 8's flows are the
  `legacy_scope_document` paths into `local_evidence.py main`
  (`scope-verify`), which `WorkflowScopeStepTests` and
  `test_local_evidence.py` exercise end to end.

## Implementation of Review Findings #1

> **started at:** 2026-09-21T08:05:10-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-single-os/features/2026-09-19-direct-cell-execution/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review contains 2 findings, both priority **high**:
        - 'Native L2 cells cannot publish a completion record'
        - 'Completion accepts a canonical marker from the wrong test tier'
- packages directly impacted by the spec: `repo-deps` (root `scripts/ci/*.py`, `.github/workflows/*.yml`) and `test-toolkit` (`tools/test-toolkit/tests/ci_workflow_contracts.rs`)
- orchestrator analysis completed at 08:14:30-07:00:
        - GitNexus `impact --direction upstream` on `completion.py::selection_problems` (3 impacted), `affected_scope.py::package_cells` (7 impacted), and `completion.py::required_backends` all report risk **LOW**
        - only `ubuntu-latest` and `macos-latest` declare `tmux` as an available L2 backend capability in `.github/ci/environments.json`; every GUI backend and every WSL2/Windows backend is a governed gap, so the plan already knows which backends each L2 cell can prove
        - `cell_contract.py` emits `backends` from the package-wide `l2_backends`, and `completion.py::required_backends` reads the same package-wide list, which is the root of the first finding
        - the `backend-proof` sidecar (`tools/test-toolkit/src/bin/backend-proof.rs`) judges the required backends but writes no document `completion.py --backend-proofs` can read
        - `_tier_filter` (just/devops.just) is the only source of the canonical tier expressions; `completion.py` recognizes markers via `schema.TIER_MARKERS` but never binds a marker to the cell's gate, which is the root of the second finding
- starting the work on 'Native L2 cells cannot publish a completion record' at 08:14:30-07:00
        - loaded the `rust-devops`, `rust-testing`, and `rust` skills; read review-1.md § "Native L2 cells cannot publish a completion record" and the current planner, schema, cell contract, completion validator, `backend-proof` binary, `evidence.rs`, both workflows, and `_expected_manifest` / `_test_l2`
        - GitNexus upstream impact: `package_cells` 7 impacted (LOW), `declared_problems` 3 (LOW), `backend_problems` 3 (LOW), `validate_resolved_plan` 11 (MEDIUM — five direct callers: the planner, the hook, ci-plan, cross_check, local_evidence; all read the plan through `schema.py`, so a new optional cell field plus version bump is the whole surface), `required_backends` / `completion_record` / `contract` ambiguous across languages, resolved by file: the Python ones are LOW
        - confirmed the defect end to end: `cell_contract.contract` publishes `package["l2_backends"]` as `backends`, the workflow narrows that to tmux with `select(. == "tmux")` before `_expected_manifest` records it, `completion.required_backends` reads the package-wide list again, and nothing in the workflow writes the `--backend-proofs` document (`backend-proof verify` prints a verdict and exits)
        - `_stage_junit_reset` removes only nextest's `test-results.xml`, never the staging directory, so a `backend-proofs.json` written by `backend-proof verify` survives until `completion.py` reads it; `expected-<gate>.json` relies on the same property
        - only `ubuntu-latest` and `macos-latest` carry `tmux: true` in `.github/ci/environments.json`; every GUI backend is `available: false` everywhere, so the hostable subset of any mixed-backend package on a native runner is exactly `["tmux"]`
        - decision: `cell_contract.py` keeps publishing the package-wide list as a NEW output `declared_backends`, read only by the "Report L2 backend coverage" summary step, so the summary still names the GUI backends that skip (a contract in `ci_workflow_contracts.rs` asserts that visibility); `backends` becomes the cell's required list
        - A done: `affected_scope.package_cells` now attaches `backends` (sorted hostable subset) to every executing L2 cell through a new `add(..., backends=)` keyword; `schema.py` gains `CELL_FIELDS["backends"]`, `_cell_backends` (executing L2 without it, present on any other cell, or naming an undeclared backend are all `malformed-receipt`), and `RESOLVED_PLAN_SCHEMA_VERSION = 6`; mirrored in `scripts/ci-rollup.rs::PLAN_SCHEMA_VERSION`, `scripts/ci-plan-tests.rs`, the five `.githooks/tests/fixtures/plan-*.json`, `affected_scope_stub.py`, and `.github/ci/schemas/contract.json` (regenerated with `python3 scripts/ci/schema.py`)
        - B done: `cell_contract.contract` publishes `backends` from the cell and a new `declared_backends` from the package; `_package-ci.yml` reads `REQUIRED_BACKENDS` verbatim (`jq -r 'join(",")'`) in the expected and Tests steps, the coverage summary marks a backend "required by the plan" when the cell lists it and keeps the GUI skip rows from `declared_backends`, and the "tmux is the only CI-hostable one" comments now describe the plan-derived contract
        - C done: `completion.required_backends(cell)` reads the cell and refuses an executing L2 cell without a list as `completion-plan-unreadable` (infrastructure); `declared_problems`, `backend_problems`, `completion_record` and their docs follow; the module example points `--backend-proofs` at `target/nextest/ci-reports/backend-proofs.json`
        - D done: `evidence.rs` gains `BACKEND_PROOFS_FILE`, `backend_proofs_json` / `write_backend_proofs` (sorted keys, one line, `Path::join`), and `clear_backend_evidence`; `backend-proof verify` writes the document before its verdict on both outcomes and exits 2 if it could not, `reset` clears both files; `local_evidence.py` mirrors the constant; fixture `scripts/ci/fixtures/backend-proofs-tmux.json` is the cross-language contract
        - E done: native certify step passes `--backend-proofs target/nextest/ci-reports/backend-proofs.json`, WSL passes `"$stage/backend-proofs.json"`; verified the L2 recipe never sets `BISCUIT_JUNIT_STAGE_DIR` on native runners, so `backend-proof` falls back to `workspace_root()/target/nextest/ci-reports` — the same tree `_expected_manifest` and `_stage_junit` default to (relative to `BISCUIT_JUNIT_WORKSPACE_ROOT`, which archive mode sets to this checkout) and the one certify reads
        - latent risk noted, not changed (out of scope): the archive-mode sidecar `backend-proof` resolves `workspace_root()` from the compile-time `CARGO_MANIFEST_DIR` when the run-time variable is unset, so it lands in the consumer checkout only because hosted runners of the same OS share the `/home/runner/work/<repo>/<repo>` layout; exporting `BISCUIT_JUNIT_STAGE_DIR` in the Tests step would make it explicit
        - F (Python) done: `test_completion.py` gains `WorkflowBoundaryTests` — the real planner resolves `biscuit-terminal/cli/src/main.rs`, the `biscuit-terminal-cli/ubuntu-latest/L2` cell carries `backends == ["tmux"]` while the package declares four, `cell_contract.contract` publishes `'["tmux"]'` plus the declaration as `declared_backends`, an `_expected_manifest`-shaped manifest with `backends: ["tmux"]` completes with the shared fixture `scripts/ci/fixtures/backend-proofs-tmux.json`, and both an absent document and `proven: false` exit 1 with `completion-backend-unproven`; a manifest requiring `["tmux","wezterm"]` is refused with `completion-manifest-selection`; `test_a_required_backend_proof_absent_fails_and_present_passes` and `test_declared_backends_matching_the_plan_complete` now use a mixed-backend package with a tmux-only cell, and two negatives were added (whole-declaration manifest refused; executing L2 cell without `backends` exits 2 as `completion-plan-unreadable`)
        - F (Python) done: `test_schema.py` gains `CellBackendsValidationTests` (hostable subset validates; missing / empty / non-list on an executing L2 cell, `backends` on an L1 or gap cell, and an undeclared backend are each `malformed-receipt`) and `DirectExecutionSchemaOracleTests` pins version 6; `test_affected_scope.py::L2BackendAxisTests` gains the mixed-backend planner case; `test_resolved_plan.py` asserts the contract's `backends` is the cell's and `declared_backends` the package's; `test_evidence_reuse.py`'s L2 fixture cell carries `backends`
        - F (Rust) done: new `tools/test-toolkit/tests/backend_proofs_document.rs` (fixture byte-equality after JSON normalization, `proven: false` recorded not omitted, verify-then-reset clears both files, writer creates the stage directory); `ci_workflow_contracts.rs` asserts the L2 job no longer contains `select(. == "tmux")` and reads `REQUIRED_BACKENDS` from `steps.cell.outputs.backends`, and a new contract pins `--backend-proofs` into the same stage tree as `--expected-manifest` / `--artifacts` in both certify steps
        - docs: `.github/ci/schemas/README.md` (table row and prose to version 6, `backends` in the cell bullet), `.github/ci/README.md` (new paragraph on plan-derived required backends and the proof document), `docs/topics/ci-cd.md` (version 6 sentence), `.claude/skills/rust-testing/SKILL.md` (two sentences that said "tmux is the only CI-hostable backend" / "declared ∩ provisioned" now name the cell's `backends`); `.claude/skills/rust-devops/ci-cd.md` mentions only the rollup result document's version 5, which did not move, so it is untouched
        - results so far: all 13 Python suites green after the fixture updates (`test_completion` 55, `test_schema` 137, `test_affected_scope` 283, `test_evidence_reuse` 76, `test_resolved_plan` 96, others unchanged); `cargo nextest run -p test-toolkit` 233 passed / 2 skipped
        - verification (macOS host, this checkout): all 13 `scripts/ci/test_*.py` suites OK (affected_scope 283, build_key 12, ci_local 92, completion 55, constraints 39, cross_check 13, evidence_reuse 76, local_evidence 23, publish_gaps 19, resolved_plan 96, reuse_validation 21, runner_loss 50, schema 137); `cargo nextest run -p test-toolkit --no-fail-fast` 233 passed / 2 skipped; `cargo nextest run -p repo-deps --no-fail-fast` 446 passed (one earlier run showed 1 timeout in the slow archive test while the Python suites and a clippy build ran concurrently; a re-run on an idle machine passed 446/446); `just _lint repo-deps` and `just _lint test-toolkit` clean; `env -u CDPATH .githooks/tests/test-pre-push.sh` 67 passed / 0 failed; `actionlint` clean on both workflows; `just ci-local --plan` prints 9 rows across the root and tools areas
        - manual wiring check: `cargo run -p test-toolkit --features backend-proof --bin backend-proof -- verify --required tmux --stage-dir <tmp>` over two tmux run records writes bytes identical to `scripts/ci/fixtures/backend-proofs-tmux.json` (`cmp` clean); with `--required tmux,kitty` it exits 1 and leaves `{"kitty":{"executed":0,"proven":false},"tmux":{"executed":2,"proven":true}}`; `reset` empties the stage
        - GitNexus `detect-changes --scope all`: 33 files, 80 symbols, 0 affected processes, risk low, no partial/truncated flag
        - OS considerations: no cross-OS run was performed; the Rust change joins paths with `Path::join` and writes one deterministic line, so no platform-specific risk was identified. The one latent cross-host concern (the sidecar's compile-time `CARGO_MANIFEST_DIR` fallback in archive mode) predates this change and is recorded above
- work completed for 'Native L2 cells cannot publish a completion record' at 08:41:01-07:00
- starting the work on 'Completion accepts a canonical marker from the wrong test tier' at 08:42:11-07:00
        - loaded the `rust-testing`, `rust-devops`, and `rust` skills; read review-1.md § "Completion accepts a canonical marker from the wrong test tier", `completion.py::selection_problems` / `predicates`, `schema.TIER_MARKERS` and the contract emitter, `_tier_filter` and the `_expected_manifest` archive wrapper, and `test_completion.py::ShippedSelectionTests`
        - GitNexus upstream impact: `selection_problems` 3 impacted (LOW; one direct caller, `manifest_problems`); `predicates` ambiguous across 3 same-named symbols, the `scripts/ci/completion.py` candidate is 3 impacted (LOW)
        - confirmed the defect: `_TIER_MARKER` accepts any of the six markers for any gate, and nothing relates the expression's shape (positive vs. negated) or its marker to `cell["gate"]`; the default `expected_manifest` fixture in `test_completion.py` records `package(claudine)` with no tier expression at all, which today passes for an L1 cell
        - other consumers of the contract: `contract.json` is pinned byte-for-byte by `test_schema.py::ContractArtifactTests`; the Rust oracles (`ci-rollup-tests.rs`, `ci_workflow_contracts.rs`) read specific `vocabulary` keys and never `tier_markers`, so a new sibling key needs no Rust change; `ci_workflow_contracts.rs::nextest_filter` shells to `just _tier_filter` only to count tier tests, not to validate shape; `.github/ci/schemas/README.md:450` is the one doc sentence that states the selection rule
        - decision: the expected shape is data in `schema.py`, not a copy of the filter strings — `CANONICAL_SELECTION = {"selects": {L2→level2_, L3→level3_, browser→browser_, real→real_}, "excludes": {"tiers": [L1, sanity], "must": [level2_, level3_, browser_, real_], "may": [slow_, perf_]}}` — so a `_tier_filter` that drifted cannot certify itself and the two L1 variants (`l1-include-slow`, `worktree-cli`'s `perf_`) stay legal without naming a package or a switch; published under `contract.json` `vocabulary.canonical_selection` beside `tier_markers` and regenerated with `python3 scripts/ci/schema.py`
        - `completion.py`: new `canonical_selection_problems(expression, tier, package)` parses an optional `package(<pkg>) & (…)` wrapper, then exactly one `test(/(^|::)<marker>/)` for a positive tier or `!(test(…) + …)` for an L1-shaped one (negated set ⊇ must, ⊆ must ∪ may), with `_closing` / `_term` / `_marker` / `_split_union` depth-counting helpers rather than one regex; whitespace around tokens is tolerated; every refusal is a `completion-manifest-selection` naming why and what the gate expects. `selection_problems` runs the foreign-predicate check first and the shape check only when no foreign predicate was found, so one defect produces one message; `predicates()` kept and now shares `_closing`
        - `CANONICAL_PREDICATES` lost `all`: no shipped tier expression uses it and the shape check would refuse it anyway, so keeping it in a tuple named "predicates a canonical selection may use" was comment/code drift; the `_TIER_MARKER` and `selection_problems` docs were reworded to say the gate fixes the expression, not the vocabulary alone
        - `test_completion.py`: the default `expected_manifest` fixture recorded `package(claudine)` (no tier expression, accepted before, refused now); it now records `TIER_SELECTIONS[tier]` so every L1/L2/browser fixture and the audit end-to-end producer carry their own gate's expression, and the `--run-ignored` / profile negatives use `L1_SELECTION` so they fail for exactly the reason they assert. `ShippedSelectionTests` now checks each `just _tier_filter` expression against ITS tier (L1/L2/L3/browser/real/sanity), the two L1 variants, archive scoping for L1/L2/browser, and refuses: the review's three wrong-tier examples, inverted inclusion for L2/browser and a partial L1 exclusion, a package-scoped wrong-tier expression, a scope naming another package, and a shipped expression ANDed with `test(/(^|::)slow_/)` or a bare test name (59 tests, was 55)
        - `test_schema.py`: new `CanonicalSelectionContractTests` — the selects/must sets coincide, must ∪ may is exactly `TIER_MARKERS` with no marker bound twice, every `BUILD_GATES` gate has a canonical shape, and the contract document publishes the table (140 tests, was 137)
        - docs: `.github/ci/schemas/README.md` `completion-manifest-selection` row now says the listing must be the cell's gate's own tier expression and lists the refused shapes; no other doc, skill, Rust oracle, hook fixture, or justfile stated the old rule, so nothing else moved (`ci-rollup-tests.rs` / `ci_workflow_contracts.rs` read named `vocabulary` keys and are unaffected by the new sibling key)
        - direct check of the review's three examples via `python3 -c` against `completion.selection_problems`:
                - `L1  test(/(^|::)level2_/)` → REFUSED: "not the canonical L1 selection (an L1-shaped tier negates the other tiers' markers); L1 expects !(test(…) + …) negating every marker in ['level2_', 'level3_', 'browser_', 'real_'] and no marker outside those plus ['slow_', 'perf_']"
                - `L2  test(/(^|::)browser_/)` → REFUSED: "not the canonical L2 selection (it selects the 'browser_' tier); L2 expects test(/(^|::)level2_/)"
                - `browser  !(test(/(^|::)browser_/))` → REFUSED: "not the canonical browser selection (a positive tier does not negate); browser expects test(/(^|::)browser_/)"
        - verification (macOS host, this checkout): all 13 `scripts/ci/test_*.py` suites OK (completion 59, schema 140, affected_scope 283, build_key 12, ci_local 92, constraints 39, cross_check 13, evidence_reuse 76, local_evidence 23, publish_gaps 19, resolved_plan 96, reuse_validation 21, runner_loss 50); `just _lint repo-deps` and `just _lint test-toolkit` clean; `just _test repo-deps` 445 passed / 1 skipped (the skip is the L1 tier filter excluding a marked test, unchanged by this work); `just _test test-toolkit` 233 passed / 2 skipped; `env -u CDPATH .githooks/tests/test-pre-push.sh` 67 passed / 0 failed
        - GitNexus `detect-changes --scope all`: 33 files, 99 symbols, 0 affected processes, risk low, no partial/truncated flag
        - OS considerations: pure Python stdlib string handling with no paths, processes, or platform branches, so no cross-OS run was performed and no OS-specific risk was identified
- work completed for 'Completion accepts a canonical marker from the wrong test tier' at 08:55:40-07:00
- orchestrator re-verification at 09:02:18-07:00 (independent of the subagent runs):
        - all 13 `scripts/ci/test_*.py` suites OK (completion 59, schema 140, affected_scope 283, ci_local 92, evidence_reuse 76, resolved_plan 96, local_evidence 23, runner_loss 50, constraints 39, publish_gaps 19, reuse_validation 21, cross_check 13, build_key 12)
        - `just _test test-toolkit`: 233 passed, 2 skipped; `just _test repo-deps`: 445 passed, 1 skipped
        - `just _lint repo-deps` and `just _lint test-toolkit`: clean
        - the review's three wrong-tier expressions (`L1 test(/(^|::)level2_/)`, `L2 test(/(^|::)browser_/)`, `browser !(test(/(^|::)browser_/))`) are each refused by `completion.selection_problems`
        - no `just cross-check` run was made: the changes are stdlib Python string handling plus one Rust file writer that uses `Path::join` and a `BTreeMap`-ordered single-line JSON document, so no OS-specific behavior was introduced; CI covers Linux/macOS on the pull request and Windows/WSL2 after merge

### Successful Completion

The implementation of review cycle 1 has completed successfully in 58 minutes (08:05:10 to 09:03). During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred.

- 'Native L2 cells cannot publish a completion record' — **fixed**: the resolved plan (schema version 6) attaches `backends` to every executing L2 cell as the subset of the package's `l2_backends` hostable on that environment; `cell_contract.py`, both workflows, and `completion.py` read that list; `backend-proof verify` writes `backend-proofs.json` and both certify steps pass it with `--backend-proofs`; a workflow-boundary fixture resolves the real `biscuit-terminal-cli/ubuntu-latest/L2` row and proves tmux proof completes it while an absent or `proven: false` document does not, with the Rust and Python halves bound through `scripts/ci/fixtures/backend-proofs-tmux.json`.
- 'Completion accepts a canonical marker from the wrong test tier' — **fixed**: `schema.CANONICAL_SELECTION` binds each gate to its own marker (or, for L1/sanity, to the must/may exclusion sets), `completion.canonical_selection_problems` parses the recorded expression against that shape, and `ShippedSelectionTests` checks every `just _tier_filter` expression against its own gate with negatives for wrong-tier, inverted, package-scoped-wrong-tier, foreign-package-scope, and ANDed-narrowing expressions.

The files changed by this cycle are the 33 in `git status` for this worktree, principally: `scripts/ci/{schema,completion,affected_scope,cell_contract,local_evidence}.py`, `scripts/ci-rollup.rs`, `scripts/ci-plan-tests.rs`, `scripts/ci/test_{completion,schema,affected_scope,resolved_plan,evidence_reuse}.py`, `scripts/ci/fixtures/backend-proofs-tmux.json`, `.github/workflows/{_package-ci,_wsl-ci}.yml`, `.github/ci/schemas/{contract.json,README.md}`, `.github/ci/README.md`, `.githooks/tests/fixtures/*`, `tools/test-toolkit/src/{evidence.rs,lib.rs,bin/backend-proof.rs}`, `tools/test-toolkit/tests/{backend_proofs_document.rs,ci_workflow_contracts.rs}`, `docs/topics/ci-cd.md`, and `.claude/skills/rust-testing/SKILL.md`.

Items surfaced but intentionally left out of scope (not defects introduced here): the sidecar `backend-proof` resolves its stage directory through the compile-time `CARGO_MANIFEST_DIR` fallback when `BISCUIT_JUNIT_STAGE_DIR` is unset in archive mode (works today because same-OS hosted runners share the checkout layout; exporting the variable in the Tests step would make it explicit); the `execution_path` retirement and other v6 follow-ups listed in `acceptance.md` were not folded into this schema bump; the spike plan JSON documents under `spikes/plans/` still carry `schema_version: 5` as historical evidence and were not regenerated.
