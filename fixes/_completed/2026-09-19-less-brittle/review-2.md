---
$schema: feature-review.yaml
ready: false
findings:
  - title: Deleted-path identity never reaches the production planner
    priority: high
  - title: The scanner can silently leave the repository or skip unreadable inputs
    priority: high
  - title: Relevant guard configuration does not select a guard scan
    priority: high
  - title: The new Messenger development dependency is absent from dependency documentation
    priority: low
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-20T02:36:39-07:00"
spec: 2026-09-19-less-brittle/spec.md
implemented: true
implemented_by: claude/opus
log: fixes/2026-09-19-less-brittle/implementation-log.md
description: "A **fix** review of `2026-09-19-less-brittle/spec.md`"
fix: 2026-09-19-less-brittle/review-2.md
previous: 2026-09-19-less-brittle/review-1.md
next: 2026-09-19-less-brittle/review-3.md
---

# Review 2

**Not production-ready.** The implementation resolves the twelve known path
sites, replaces the old text matcher, adds independent guard scheduling, and
wires the guard-only lint cell through CI and the rollup. However, the
production diff boundaries never identify deletions, the scanner still has
fail-open filesystem paths, and changes to the guard's registry declaration do
not select a real guard scan.

The previous review contains five actionable findings and no blocked-findings
section. Findings 1, 4, and 5 are implemented. Findings 2 and 3 are materially
advanced but remain incomplete for the reasons below.

## Findings

### High — Deleted-path identity never reaches the production planner

`affected_scope.py` adds the repeatable `--deleted` argument and uses it to
distinguish a deletion from a path that is unexpectedly absent. None of the
three production selection boundaries supplies that argument:

- `.github/workflows/ci.yml:211-212` obtains only `git diff --name-only` paths;
- `just/ci-local.just:225-234` obtains only `git diff --name-only` paths; and
- `.githooks/pre-push:380-382` pipes only `git diff --name-only` paths to the
  planner.

Consequently, `change_inventory.deleted` is always empty in ordinary hosted and
local runs. On a pull request, a deleted Rust path remains in the changed scan
list, then `archive_guard::scan` classifies its absence as a likely deletion and
continues. An unexpectedly missing path takes the same route and also passes.
This is the ambiguity the specification explicitly required the planner input
contract to eliminate.

The new tests call `calculate_scope(..., deleted=[...])` directly, so they prove
the internal branch but not that any real caller can reach it. Extract status
and names from the same Git diff at each boundary, pass every deletion through
`--deleted`, and add workflow, hook, and local-recipe fixtures that exercise an
actual deletion and an unexpectedly missing destination.

### High — The scanner can silently leave the repository or skip unreadable inputs

The full-tree walker still converts several inspection failures into successful
omissions. `archive_guard.rs:1154-1157` treats every `canonicalize` error as a
directory that vanished, and lines 1177-1179 treat every `metadata` error as a
broken symlink. Those errors also include permission and I/O failures on paths
that still exist. In changed mode, lines 1235-1240 use `Path::is_file`, which
likewise collapses a metadata error into “missing.” These branches contradict
the hard-error contract documented by `GuardError` and can report a pass after
not reading an eligible input.

The supplied-plan path validation has a related containment defect.
`schema._normalized_path_list` rejects backslashes, `./`, whitespace, sorting,
and duplicates, but accepts absolute paths, Windows drive-prefixed paths, and
`..` components. `scan` then applies `root.join(path)` without verifying that
the resolved path remains under the checkout. A malformed changed-mode plan can
therefore scan outside the repository, while an escaped missing path can be
reported as an ordinary deletion.

The existing unreadable-file and unreadable-directory fixtures reach
`read_to_string` and `read_dir` on this host; they do not exercise the
`canonicalize`, `metadata`, `is_file`, or containment branches above. Preserve
`NotFound` handling only where disappearance is explicitly allowed, propagate
other errors with the affected path, reject non-relative/parent-traversing plan
paths in both schema and Rust readers, and assert canonical containment before
reading a changed target.

### High — Relevant guard configuration does not select a guard scan

The specification requires relevant registry and execution configuration to
select the guard. `ARCHIVE_GUARD_OWN_INPUTS` omits
`tools/test-toolkit/Cargo.toml`, even though that manifest declares
`archive-path-guard` in `companion-suites` and is therefore the guard's registry
binding.

A real planner invocation for a pull-request change to that file emits
`archive_guard.selected: false`. The ordinary `test-toolkit` package selection
still attaches the lint companion, but the companion reads that unselected
scope and performs an empty changed-file violation scan; the package L1 suite
also happens to run the integration-test binary. That incidental execution is
not the explicit, plan-owned guard selection the contract requires and gives
the plan a false account of why and how the guard ran.

Add every configuration surface that controls the guard's registration or
execution to the owned-input policy, then cover the real manifest with a
planner fixture that asserts `archive_guard.selected: true` and the intended
scan mode. Keep unrelated manifests and workflow files unselected as required.

### Low — The new Messenger development dependency is absent from dependency documentation

`messenger/lib/Cargo.toml` now directly depends on `biscuit-test-harness` for
the migrated research fixtures. `docs/dependencies.md` documents the existing
Messenger CLI harness dependency but does not record the new Messenger library
development dependency. Update that dependency record alongside the manifest,
as required by the repository drift-maintenance policy and the specification.

## Previous-review disposition

- **Twelve known violations:** implemented. All twelve sites now use
  `manifest_dir!()`, Messenger declares the direct development dependency, and
  the full guard reports no non-allowlisted violation.
- **Token-aware matching and expression-scoped fallbacks:** implemented for the
  required expression fixtures. Filesystem fail-closed behavior remains
  incomplete as described above.
- **Planner selection and scan scope:** the resolved-plan model, schema, event
  policy, guard-only cell, and direct unit fixtures exist. Production deletion
  input and one required owned configuration surface remain missing.
- **CI ownership, reporting, and local validation:** implemented. The lint-only
  cell skips Clippy, consumes the resolved plan, records companion status, and
  fails closed in rollup when status or companion evidence is missing.
- **Regression coverage:** the matcher, scanner, planner, workflow, rollup, and
  relocation suites now exist. The remaining gaps are specifically the real
  deletion callers and the scanner error/containment branches in the findings.

## Requirement-to-verification map

This fix has no terminal rendering, keyboard, mouse, paste, IME, or other
terminal-emulator input requirement. L2 and L3 would not verify its behavior;
L1 logic, filesystem, subprocess, and workflow-contract tests are the correct
levels.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| Twelve path migrations and runtime remapping | L1 package tests plus a relocated Messenger fixture with an absent producer path | Appropriate level; passed. |
| Token-aware matcher and expression-local exemptions | L1 lexer/matcher fixtures | Appropriate level; passed. |
| Fail-closed scan and repository boundary | L1 temporary-filesystem fixtures | Gap: canonicalization, metadata, changed-path inspection, and escape cases are untested and defective. |
| Event-specific planner scope, explicit empty inventory, and proven-environment behavior | L1 planner/schema fixtures | Internal logic passes; real deletion inputs and registry-config selection are not wired. |
| Guard-only CI execution, missing status, and merge blocking | L1 workflow-source and Rust rollup contracts; `actionlint` | Appropriate level; passed. |
| Migrated Claudine, DMLS, and Messenger consumers | L1 canonical package recipes | Appropriate level; passed. |

## Verification performed

- `just test repo-deps`: **420 tests run, 420 passed, 1 skipped**.
- `just test test-toolkit`: **283 tests run, 283 passed, 2 skipped**.
- Python CI suites: `test_affected_scope.py` (**275 passed**),
  `test_schema.py` (**129 passed**), `test_resolved_plan.py` (**79 passed**),
  `test_ci_local.py` (**92 passed**), `test_evidence_reuse.py` (**76 passed**),
  and `test_local_evidence.py` (**20 passed**).
- `cd tools/test-toolkit && just archive-path-guard`: **2 passed**; full-tree
  mode checked 3,475 files and reported only the five existing allowlisted
  forms.
- `just _test claudine-cli --test compose_initialize_acceptance`: **23 passed**.
- `just _test claudine --lib`: **4,216 passed**.
- `just _test dmls`: **715 passed**.
- `just _test messenger --features research`: **316 passed, 2 skipped**.
- `just _test messenger-cli --test research_cli --test research_lifecycle_cli`:
  **19 passed**.
- Focused Messenger relocation test: **1 passed**.
- `just _lint repo-deps` and `just _lint test-toolkit`: passed.
- `actionlint -no-color` on the two changed reusable workflows: passed.
- `git diff --check`: passed.

The GitNexus repository index was refreshed to commit `cd80765` before review;
the relevant query resolved the planner's `matrix_record` flow and the new
archive-guard/companions-only symbols. No terminal or browser window was
launched, and no cross-OS execution evidence is required for this readiness
decision.
