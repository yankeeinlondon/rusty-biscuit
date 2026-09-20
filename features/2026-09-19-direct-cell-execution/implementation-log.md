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
