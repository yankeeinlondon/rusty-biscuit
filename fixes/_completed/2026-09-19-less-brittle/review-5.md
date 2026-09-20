---
$schema: feature-review.yaml
ready: false
findings:
  - title: The schema-directory ownership rule schedules unrelated files
    priority: medium
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-20T11:29:32-07:00"
spec: 2026-09-19-less-brittle/spec.md
implemented: true
implemented_by: claude/opus
log: fixes/2026-09-19-less-brittle/implementation-log.md
description: "A **fix** review of `2026-09-19-less-brittle/spec.md`"
fix: 2026-09-19-less-brittle/review-5.md
previous: 2026-09-19-less-brittle/review-4.md
next: 2026-09-19-less-brittle/review-6.md
---

# Review 5

**Not production-ready.** Review 4's finding is implemented correctly. The
Rust plan reader now rejects fields outside the canonical `archive_guard` key
set before interpreting the scope, and the new shared corpus exercises the
same accepted and rejected shapes through both the Python validator and the
Rust consumer. The implementation also added a useful distinct diagnostic for
a non-object `archive_guard` value.

Review 4 contains one unblocked finding and no blocked-findings section. No
previously blocked finding needed reevaluation in this iteration.

## Findings

### Medium — The schema-directory ownership rule schedules unrelated files

The new `SUITE_OWNER_PREFIXES` entry assigns every path below
`.github/ci/schemas/` to both `repo-deps` and `test-toolkit`
(`scripts/ci/affected_scope.py:292-303`). Only two files in that directory are
cross-language inputs consumed by `test-toolkit`:
`contract.json` and `archive_guard_cases.json`. The directory also contains
`README.md`, and the prefix automatically captures any unrelated schema added
later. A direct planner probe confirms that both the README and a hypothetical
`.github/ci/schemas/unrelated-future-schema.json` select `test-toolkit`.

This broadens CI scope beyond the inputs the Rust suite actually verifies and
contradicts the specification's requirement that these owned inputs remain a
narrow explicit list. It also makes the new test incomplete: it proves the two
cross-language documents select both owners, but has no negative case proving
that unrelated files in the same directory retain the broader `.github/ci/`
owner only (`scripts/ci/test_affected_scope.py:3818-3826`).

Move the two cross-language documents into `SUITE_OWNER_PATHS`, each with
`("repo-deps", "test-toolkit")`, and remove the directory prefix. Add negative
fixtures for `.github/ci/schemas/README.md` and an unrelated future schema so
the boundary cannot widen again. The existing `.github/ci/` prefix will still
select `repo-deps` for those paths.

## Previous-review disposition

- **The Rust plan reader still accepts unknown guard fields:** implemented.
  `GuardPlan::from_plan_json` checks the closed field set before reading the
  scope, sorts unknown names for deterministic diagnostics, and rejects
  non-object scopes explicitly. The shared 26-case corpus and frozen field-set
  contract pass through both language implementations.
- **Blocked findings:** review 4 contains no blocked-findings section, and
  reviews 2 and 3 likewise recorded none, so no blocked item became newly
  actionable before this implementation.

## Requirement-to-verification map

This fix has no terminal rendering, keyboard, mouse, paste, IME, or other
terminal-emulator input requirement. L2 and L3 would not verify its behavior;
L1 logic, temporary-filesystem, subprocess, real-Git, and workflow-contract
tests are the appropriate levels.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| Twelve path migrations and runtime remapping | L1 package and relocation tests plus the canonical full-tree guard | Appropriate level; no migrated site regressed. |
| Token-aware matcher and expression-local exemptions | L1 lexer/matcher fixtures plus the canonical full-tree guard | Appropriate level; passed. |
| Production deletion and rename classification | L1 real-Git local-recipe, workflow-boundary, and pre-push fixtures from earlier iterations | Appropriate level; unchanged and passing in the affected suites. |
| Fail-closed changed-path scan | L1 temporary-filesystem and shipped-planner-to-reader regressions | Appropriate level; passed. |
| Supplied-plan validation | L1 Rust reader, Python validator, frozen field-set contract, and shared 26-case corpus | Appropriate level; review 4's contract gap is closed. |
| Guard-only CI execution, missing status, and merge blocking | L1 workflow-source and Rust rollup contracts | Appropriate level; passed. |
| Cross-language schema input selection | L1 planner ownership tests | Gap: positive cases pass, but the prefix also selects unrelated documentation and future schemas. |

## Verification performed

- `just test test-toolkit`: **312 tests run, 312 passed, 2 skipped**.
- `just test repo-deps`: warm run **420 tests run, 420 passed, 1 skipped**.
  The first cold run exposed unrelated archive-fixture setup contention: one
  fixture ran before `target/debug/ci-build` existed and 13 compile-heavy
  fixtures hit their 30-second limits. The immediate warm rerun was clean.
- `python3 scripts/ci/test_schema.py`: **134 passed**.
- `python3 scripts/ci/test_affected_scope.py`: **296 passed**.
- `cd tools/test-toolkit && just archive-path-guard`: **2 passed**; full-tree
  mode checked 3,475 files and reported only the five existing allowlisted
  forms.
- `just _lint test-toolkit` and `just _lint repo-deps`: passed with zero
  warnings.
- `git diff --check`: passed.

The GitNexus index was refreshed to `5efb9a4` and matches all covered files.
Upstream impact is **medium** for `GuardPlan::from_plan_json` (25 dependents)
and **low** for `archive_guard_triggered`; working-tree change analysis reports
low aggregate risk and no affected execution process. No terminal or browser
window was launched. Cross-OS execution evidence remains external to this
readiness decision, as requested.

The requested previous-review path under `prompts/_reviews/fixes/` does not
exist in this checkout. The authoritative existing file is
`fixes/2026-09-19-less-brittle/review-4.md`; it is marked implemented and now
points to this review. The specification records `review_iterations: 5` and
remains incomplete because this review is not ready.
