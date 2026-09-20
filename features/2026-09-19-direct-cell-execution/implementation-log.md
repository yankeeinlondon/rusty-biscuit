---
kind: implementation-log
feature: 2026-09-19-direct-cell-execution
spec: /Volumes/coding/wt/rusty-biscuit/feat-single-os/features/2026-09-19-direct-cell-execution/spec.md
plan: features/2026-09-19-direct-cell-execution/plan.md
implemented_by: opencode/zai-coding-plan/glm-5.3
started_phase: 1
packages: []
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
---

# Implementation Log for 2026-09-19-direct-cell-execution (9 phases)

> `packages` above is the per-phase record: Phase 1 touched no workspace
> package (decision and measurement only). The plan file's own `packages`
> frontmatter (`repo-deps`, `test-toolkit`) names the feature's target
> packages and is unchanged.

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
