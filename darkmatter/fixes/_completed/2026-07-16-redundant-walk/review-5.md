---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-19T22:08:10-07:00
spec: 2026-07-16-redundant-walk/spec.md
implemented: true
description: "A **fix** review of `2026-07-16-redundant-walk/spec.md`"
fix: 2026-07-16-redundant-walk/review-5.md
previous: 2026-07-16-redundant-walk/review-4.md
next: 2026-07-16-redundant-walk/review-6.md
---

# Review 5 — Redundant Walk

## Verdict

This fix is **ready for production**. The ordinary one-step and `FileTree`
paths use the internal fresh-graph seam, the caller-supplied prebuilt path
retains its complete fail-closed compatibility check, and both paths converge
on one validation engine. The private prepared-heading snapshot also preserves
the intended one-step semantics when a transcluded child's headings change
after graph construction.

## Findings

No blocking findings.

### Low — The Review 4 cache-first optimization remains unimplemented and untested

`collect_composed_heading_slugs` seeds `HeadingSlugCache` from the graph-owned
snapshot, but `validate_cross_doc_fragment` still performs the target existence
and extension checks and then opens and parses the target with
`Markdown::try_from` before `cached_prepared_heading_slugs` checks that cache
(`darkmatter/lib/src/markdown/reference/validate.rs:916-932`). Therefore a file
that is both a graph descendant and a `path#fragment` target still incurs a
redundant open and parse even though its prepared slugs are already available.

The existing cross-document tests verify fragment correctness, and the paired
heading-mutation test verifies same-document composed-heading snapshot
semantics, but no Level-1 test covers the precise Review 4 scenario: a
transcluded child also referenced as `child.md#heading`, with evidence that the
snapshot entry is consulted before a disk load.

This is a residual optimization and coverage gap, not a correctness or trust-
boundary failure, so it does not block production readiness. Resolve the
canonical cache key after path resolution, check the cache before constructing
`Markdown`, retain the disk fallback for non-descendant targets, and add the
focused Level-1 test proposed by Review 4.

## Implementation Assessment

- `validate` constructs one full graph and immediately calls
  `validate_fresh_graph`; no compatibility or dependency-manifest verification
  occurs on the ordinary one-step path.
- `FileTree::ensure_built` keeps construction adjacent to the same internal
  seam and aligns `validation_options.graph` with the construction options.
- `validate_with_graph` calls `verify_graph_compatibility` before
  `validate_graph_contents`, preserving root/source/mode/options and descendant
  freshness checks for caller-supplied graphs.
- Fresh and checked paths share `validate_graph_contents`, including
  flattening, fail-fast behavior, local/remote/fragment validation, and report
  construction.
- `PreparedHeadingSnapshot` is private, omitted from `Debug` and serialized
  graph views, and populated from the build's existing prepared TOC parse.
- Public Rust signatures, error variants, reports, CLI wiring, and graph JSON
  views remain unchanged.

The implementation is portable across macOS, Windows, and Linux: it uses Rust
`Path`/`PathBuf`, `HashMap`, and filesystem APIs, with best-effort
canonicalization falling back to the original path.

## Requirement-to-Verification Assessment

This fix changes in-process library and filesystem behavior. Level 1 is the
appropriate tier for all user-observable requirements. No terminal rendering,
terminal input encoding, browser behavior, keyboard input, paste/IME, mouse,
or scrolling behavior requires Level 2 or Level 3.

| AC | Requirement | Strongest verification | Assessment |
|---|---|---|---|
| 1 | One graph build; ordinary validation skips compatibility and descendant verification | Level 1 source routing plus changed-child mechanism test | **Pass.** Construction and the fresh seam are adjacent. |
| 2 | `FileTree::ensure_built` uses the trusted fresh seam | Level 1 source routing and file-tree tests | **Pass.** The graph and validation options are visibly paired. |
| 3 | Public prebuilt validation remains fail-closed before flattening | Level 1 provenance and changed/missing/unreadable descendant tests | **Pass.** Verification precedes the shared engine. |
| 4 | Both trust paths share one validation/report implementation | Level 1 source inspection and parity tests | **Pass.** Both converge on `validate_graph_contents`. |
| 5 | Fresh validation uses the build snapshot while checked reuse rejects staleness | Level 1 paired link-mutation and heading-mutation tests | **Pass.** The heading variant enables fragment validation and proves the intended divergence. |
| 6 | Existing provenance, mismatch, parity, file-tree, and presentation behavior remains valid | Level 1 unit and integration suites recorded in review cycles 3 and 4, plus the current seam selector | **Pass.** Review 3 records 568/568 focused tests and complete area lint; the current seam tests pass 2/2. |
| 7 | Public signatures, errors, reports, CLI output, and graph views are unchanged | Level 1 API/source inspection, serialization tests, and downstream builds | **Pass.** The snapshot is an internal graph artifact. |
| 8 | Recorded Criterion evidence satisfies the amended guards | Criterion baseline/candidate evidence in `results.md`, plus Review 4's current comparison | **Pass.** The mechanism, >=100-microsecond improvement, regression, and prebuilt-gap guards are satisfied. |
| 9 | Focused tests, area gates, whitespace, and change-scope checks pass | Level 1 Nextest/Just evidence, `git diff --check`, GitNexus, and `sniff` | **Pass with current-session note.** The exact seam selector passes 2/2 and prior complete gates are recorded; the broader selector was interrupted by a concurrent Cargo build and is not counted as a pass. |

## Verification and Scope

- `sniff` reports the affected package area as `darkmatter`, containing
  `darkmatter`, `darkmatter-cli`, `dmls`, and workspace-excluded `zed-dmls`.
- GitNexus reports `validate` as **HIGH** impact (15 direct / 18 total),
  `FileTree::ensure_built` as **MEDIUM** (13 direct), and the checked/fragment
  helpers as **LOW**; no indexed execution process crosses the Reference
  boundary. The broad compare result is polluted by unrelated commits and
  working-tree changes, so it is not used to enlarge this fix's scope.
- `git diff --check` for the implementation and review documents passes.
- Current focused Level-1 seam selector: **PASS, 2/2**
  (`fresh_seam_uses_snapshot_while_checked_path_rejects_stale_graph` and
  `fresh_seam_uses_heading_snapshot_while_checked_path_rejects_stale_headings`).
- Review cycle 3 records the focused seam tests at 2/2, the broader reference
  selection at 568/568, and package-area lint for `darkmatter`,
  `darkmatter-cli`, and `dmls` with no warnings.
- Review 4 records all three package-area builds passing and a current
  Criterion comparison that retains the amended performance guards.
- This review also attempted the broader reference/validation selector, but a
  concurrent Darkmatter test session shared the Cargo build directory. That
  run exceeded the non-interactive 60-second ceiling and was terminated; it
  produced no final Nextest summary and is not reported as a pass.

## Production Readiness

**Ready.** All specified behavior has verification at the appropriate level,
the public checked boundary remains fail-closed, and the remaining cache-first
item is a low-severity follow-up optimization rather than a production blocker.
