---
$schema: feature-review.yaml
ready: false
findings:
  - title: The Rust plan reader still accepts unknown guard fields
    priority: medium
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-20T09:51:54-07:00"
spec: 2026-09-19-less-brittle/spec.md
implemented: false
description: "A **fix** review of `2026-09-19-less-brittle/spec.md`"
fix: 2026-09-19-less-brittle/review-4.md
previous: 2026-09-19-less-brittle/review-3.md
---

# Review 4

**Not production-ready.** Review 3's high-priority finding is implemented: the
planner omits known deletions, and a listed path that subsequently disappears
now fails with `MissingListedPath`. Most of its medium-priority finding is also
implemented, including schema-version, reason, selected/scope, mode, path
normalization, ordering, and uniqueness checks. One resolved-plan shape that
the canonical Python contract rejects is still accepted by the Rust reader.

Review 3 contains two unblocked findings and no blocked-findings section. No
previously blocked finding needed reevaluation in this iteration.

## Findings

### Medium — The Rust plan reader still accepts unknown guard fields

The canonical validator calls `_keys` for `archive_guard` and rejects every
field outside `selected`, `mode`, `paths`, and `reason`
(`scripts/ci/schema.py:650-665`, `scripts/ci/schema.py:747-749`). Its regression
fixture proves that an extra `files` field is contract-invalid
(`scripts/ci/test_schema.py:758-759`).

`GuardPlan::from_plan_json`, however, reads the known fields individually and
never compares the object's keys with that closed set
(`tools/test-toolkit/src/archive_guard.rs:514-617`). Consequently, a supplied
plan such as an otherwise valid full scan with `"files": ["a.rs"]` is rejected
by `validate_resolved_plan` but accepted by the process that actually executes
the guard through `BISCUIT_ARCHIVE_GUARD_PLAN`.

This is the remaining edge of review 3's finding that the Rust consumer accepts
documents the canonical contract rejects. Silent acceptance matters because a
renamed or future scope field can be ignored while the guard reports success
under different semantics than the producer intended. Reject unknown
`archive_guard` keys before interpreting the scope, and add a Rust reader
fixture corresponding to Python's `test_an_unknown_guard_field_is_rejected`.
A cross-language table-driven fixture would make future shape additions fail on
both sides instead of requiring two manually synchronized test lists.

## Previous-review disposition

- **Changed-file scans still accept unexpectedly missing inputs:** implemented.
  `scan` now returns `MissingListedPath`, `ScanReport::missing` is gone, and an
  L1 shipped-planner regression proves a declared deletion is omitted while an
  unexpected absence fails.
- **The Rust plan reader accepts contract-invalid plans:** partially
  implemented. The reader now rejects the invalid schema versions, missing or
  blank reasons, stale scope on unselected guards, invalid modes, malformed
  path lists, and unsafe path spellings named in review 3. It still accepts
  unknown guard fields as described above.
- **Blocked findings:** neither review 3 nor review 2 contains a blocked-findings
  section, so no blocked item became newly actionable before this implementation.

## Requirement-to-verification map

This fix has no terminal rendering, keyboard, mouse, paste, IME, or other
terminal-emulator input requirement. L2 and L3 would not verify its behavior;
L1 logic, temporary-filesystem, subprocess, real-Git, and workflow-contract
tests are the appropriate levels.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| Twelve path migrations and runtime remapping | L1 package and relocation tests recorded in earlier iterations; canonical full-tree guard in this iteration | Appropriate level; no migrated site regressed. |
| Token-aware matcher and expression-local exemptions | L1 lexer/matcher fixtures plus the canonical full-tree guard | Appropriate level; passed. |
| Production deletion and rename classification | L1 real-Git local-recipe, workflow-boundary, and pre-push fixtures | Appropriate level; passed. |
| Fail-closed changed-path scan | L1 temporary-filesystem and shipped-planner-to-reader regressions | Appropriate level; the review 3 gap is closed. |
| Supplied-plan validation | L1 Rust reader and Python schema fixtures | Gap: both suites pass independently, but their accepted shapes differ for unknown `archive_guard` fields. |
| Guard-only CI execution, missing status, and merge blocking | L1 workflow-source and Rust rollup contracts; `actionlint` | Appropriate level; passed. |

## Verification performed

- `just test test-toolkit`: **307 tests run, 307 passed, 2 skipped**.
- `just test repo-deps`: **420 tests run, 420 passed, 1 skipped**.
- `cd tools/test-toolkit && just archive-path-guard`: **2 passed**; full-tree
  mode checked 3,475 files and reported only the five existing allowlisted
  forms.
- `just test-githooks`: dispatcher **1 passed**; pre-push **67 passed, 0
  failed**.
- `just _lint test-toolkit`: passed with zero warnings.
- `actionlint -no-color .github/workflows/*.yml`: passed.

The GitNexus index was refreshed against the implementation checkout. Upstream
impact reports 21 direct dependents and **high** risk for
`archive_guard::scan`, so its fail-closed change was checked through the full
toolkit suite, canonical repository scan, real planner regression, and hook
boundary. No terminal or browser window was launched. Cross-OS execution
evidence is external to this readiness decision, as requested.

The requested previous-review path under `prompts/_reviews/fixes/` does not
exist in this checkout. The authoritative existing file is
`fixes/2026-09-19-less-brittle/review-3.md`; it was already marked
`implemented: true`, and its `next` field now points to this review. The
specification records `review_iterations: 4` and remains incomplete because
this review is not ready.
