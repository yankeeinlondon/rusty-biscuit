---
$schema: feature-review.yaml
ready: false
findings:
  - title: Changed-file scans still accept unexpectedly missing inputs
    priority: high
  - title: The Rust plan reader accepts contract-invalid plans
    priority: medium
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-20T07:31:48-07:00"
spec: 2026-09-19-less-brittle/spec.md
implemented: true
implemented_by: claude/opus
log: fixes/2026-09-19-less-brittle/implementation-log.md
description: "A **fix** review of `2026-09-19-less-brittle/spec.md`"
fix: 2026-09-19-less-brittle/review-3.md
previous: 2026-09-19-less-brittle/review-2.md
next: 2026-09-19-less-brittle/review-4.md
---

# Review 3

**Not production-ready.** Review 2's production diff boundaries, filesystem
inspection errors, guard-owned configuration selection, and dependency
documentation have been repaired. However, the planner now removes known
deletions from the guard's changed-path list while the scanner still accepts
any remaining absent path as a likely deletion. The implementation therefore
still has the fail-open ambiguity that the deletion identity was introduced to
remove. The Rust plan reader also accepts several documents that the canonical
resolved-plan contract rejects.

Review 2 contains four unblocked findings and no blocked-findings section. The
configuration-selection and dependency-documentation findings are implemented.
The deletion-boundary and fail-closed-scanner findings are substantially
implemented, but the first finding below remains at their planner/scanner
boundary.

## Findings

### High — Changed-file scans still accept unexpectedly missing inputs

The production boundaries now correctly derive `change_inventory.deleted`
from one NUL-delimited `git diff --name-status -z`, and
`archive_guard_scope` removes those known deletions from
`archive_guard.paths`. That makes every path remaining in a changed scan a
file the planner expects to exist. Nevertheless,
`tools/test-toolkit/src/archive_guard.rs:1378-1382` still handles
`ErrorKind::NotFound` by appending the path to `ScanReport::missing`, and the
driver prints that it was skipped before returning success.

This means a rename destination or ordinary modified Rust file that disappears
after planning is silently omitted from the scan. It is no longer possible for
that absence to mean the deletion recorded by the plan: those paths were
already filtered out. The behavior contradicts the specification's requirement
to distinguish deletions from unexpectedly missing paths and preserves the
same false-pass outcome review 2 identified.

The scanner tests encode the stale contract. In particular,
`a_deleted_listed_path_is_skipped_for_violations_and_counted_in_the_summary`
passes a deletion directly to `ScanMode::Changed`, even though the shipped
planner never emits that shape, and
`a_rename_destination_is_scanned_like_any_other_listed_path` accepts a missing
rename source that the corrected diff parser now deliberately omits. The real
Git boundary tests prove list construction, but no test resolves a plan and
then proves that an unexpectedly absent listed destination fails the guard.

Make `NotFound` a hard error for a listed changed path, or carry explicit
deletion identity into the Rust scan contract if deleted paths are to remain
listable. Update the scanner documentation and replace the stale fixtures with
an L1 planner-to-reader regression in which a known deletion is omitted and an
unexpectedly missing listed path fails.

### Medium — The Rust plan reader accepts contract-invalid plans

`GuardPlan::from_plan_json` validates a useful subset of the
`archive_guard` object, but it does not enforce the resolved-plan contract it
consumes. It does not check `schema_version`; it returns early for
`selected: false` without rejecting `mode` or `paths`; it accepts a missing or
blank `reason`; and changed paths need not be sorted or unique. The Python
validator rejects these shapes, but a plan supplied through
`BISCUIT_ARCHIVE_GUARD_PLAN` goes directly to this Rust reader.

The specification says a malformed supplied plan is an error rather than an
empty or implicit scan, and requires Rust readers to move with the shared plan
contract. An unsupported-version plan or an unselected plan carrying stale
scope can currently run as an empty changed scan and report success. Validate
the supported schema version and the complete `archive_guard` shape in the
Rust reader, including the nonempty reason and normalized sorted unique path
list, and add rejection fixtures corresponding to the Python schema cases.

## Previous-review disposition

- **Deleted-path identity:** implemented at all three production selection
  boundaries, including rename framing and NUL-safe paths. The consumer still
  accepts an unexpected absence after those boundaries, as finding 1 explains.
- **Fail-closed scanner and containment:** permission, inspection, path
  spelling, canonical containment, and non-file cases now fail closed. A
  listed `NotFound` remains a false-success branch.
- **Guard-owned configuration selection:** implemented for the registry
  binding and the two workflow execution surfaces, with real shipped-planner
  fixtures and unchanged cell cardinality.
- **Messenger dependency documentation:** implemented in the repository-level
  dependency record. The related `test-toolkit` dependency promotion is also
  documented.
- **Additional schema-fixture drift:** the pre-push fixtures and stub planner
  now use resolved-plan schema version 5, and the repo-deps suite validates
  those stored documents.

## Requirement-to-verification map

This fix has no terminal rendering, keyboard, mouse, paste, IME, or other
terminal-emulator input requirement. L2 and L3 would not verify its behavior;
L1 logic, filesystem, subprocess, and workflow-contract tests are the correct
levels.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| Twelve path migrations and runtime remapping | L1 package and relocation tests recorded in review 2 | Appropriate level; no related source changed in this iteration. |
| Token-aware matcher and expression-local exemptions | L1 lexer/matcher fixtures plus the canonical full-tree guard | Appropriate level; passed. |
| Production deletion and rename classification | L1 real-Git workflow, local-recipe, and pre-push fixtures | Correctly proves planner input construction; does not prove the scanner rejects a listed path that later disappears. |
| Fail-closed scan and repository boundary | L1 temporary-filesystem fixtures | Gap: containment and inspection errors are covered, but the listed `NotFound` fixture asserts a successful skip contrary to the new plan contract. |
| Supplied-plan validation | L1 Rust reader and Python schema fixtures | Gap: each half passes its own fixtures, but the Rust reader accepts invalid schema versions and shapes rejected by Python. |
| Guard-owned configuration selection | L1 shipped-planner and cross-language contract fixtures | Appropriate level; passed. |
| Guard-only CI execution, missing status, and merge blocking | L1 workflow-source and Rust rollup contracts; `actionlint` | Appropriate level; passed. |

## Verification performed

- `just test repo-deps`: **420 tests run, 420 passed, 1 skipped**.
- `just test test-toolkit`: **294 tests run, 294 passed, 2 skipped**.
- `cd tools/test-toolkit && just archive-path-guard`: **2 passed**; full-tree
  mode checked 3,475 files and reported only the five existing allowlisted
  forms.
- `just test-githooks`: dispatcher **1 passed**; pre-push **67 passed, 0
  failed**.
- `actionlint -no-color .github/workflows/*.yml`: passed.
- `git diff --check`: passed before the review metadata edits.

GitNexus was refreshed against the implementation checkout. Its comparison to
`main` reports 222 changed symbols across 36 files, five affected execution
flows, and overall medium risk. The changed `archive_guard::scan` symbol has
21 direct dependents and a **high** blast-radius rating; text inspection also
confirms the real repository driver calls it. No terminal or browser window was
launched, and cross-OS execution evidence is not part of this readiness gate.

The requested previous-review reference under `prompts/_reviews/fixes/` does
not resolve through biscuit-file's `FileReference` CLI. The existing review
resolves at `fixes/2026-09-19-less-brittle/review-2.md`; its `implemented: true`
is preserved and its `next` now points to this review. The specification records
`review_iterations: 3`.
