---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-19T22:48:19-07:00
spec: 2026-07-16-redundant-walk/spec.md
implemented: false
description: "A **fix** review of `2026-07-16-redundant-walk/spec.md`"
fix: 2026-07-16-redundant-walk/review-7.md
previous: 2026-07-16-redundant-walk/review-6.md
---

# Review 7 — Redundant Walk

## Verdict

This fix is **ready for production**. The one-step and `FileTree` paths use the
internal fresh-graph seam, the caller-supplied prebuilt path retains its full
fail-closed compatibility check, and both paths converge on one validation
engine. The graph-owned heading snapshot also preserves coherent fragment
validation when a transcluded child changes after construction.

## Findings

No blocking findings.

### Low — Review 6's cache-first fragment follow-up was claimed but not implemented

The current implementation still resolves and checks the target, then opens and
parses it with `Markdown::try_from` before `cached_prepared_heading_slugs`
consults the cache (`darkmatter/lib/src/markdown/reference/validate.rs:916-932`).
For a file that is both a graph descendant and a `path#fragment` target, the
graph snapshot has already seeded the canonical cache entry, so this read and
parse are redundant. Repository history contains no reference-validation source
commit after the implementation reviewed in cycles 4–6; the latest cycle commit
only updates planning/review documents.

No focused Level-1 test proves the cache-first mechanism. Existing tests prove
the seeded snapshot wins semantically after a heading mutation, but they do not
prove a cache hit avoids loading the target. This remains a non-blocking
performance and coverage gap: it does not reintroduce compatibility or
dependency-manifest verification, alter reports, or weaken the checked trust
boundary.

Resolve the canonical cache key after target-path resolution, consult the cache
before constructing `Markdown`, retain the current disk fallback for targets
that are not graph descendants, and add a Level-1 test where a transcluded child
is also referenced as `child.md#heading` with observable evidence that the
seeded entry avoids a target read.

## Implementation Assessment

- `validate` builds one full graph and immediately calls
  `validate_fresh_graph`; compatibility and dependency-manifest verification
  are absent from the one-step path.
- `FileTree::ensure_built` keeps graph construction adjacent to the fresh seam
  and copies the same clone-stable graph options into validation.
- `validate_with_graph` runs `verify_graph_compatibility` before
  `validate_graph_contents`, retaining document, source, `Full` mode, options,
  and visited-descendant checks for caller-supplied graphs.
- Both trust paths share `validate_graph_contents`, including flattening,
  fragment preparation, local and remote checks, fail-fast behavior, and report
  construction.
- `PreparedHeadingSnapshot` is private, is populated from the graph build's
  existing prepared TOC parse, and is omitted from `Debug` and serialized graph
  views.
- Public signatures, errors, report contents, CLI behavior, and graph JSON
  shapes remain unchanged.

The implementation uses portable Rust path, collection, and filesystem APIs.
No new macOS-, Windows-, or Linux-specific behavior was introduced.

## Requirement-to-Verification Assessment

All changed behavior is deterministic in-process library/filesystem behavior,
so Level 1 is the appropriate verification tier. The fix adds no terminal
rendering, terminal input encoding, keyboard, paste/IME, mouse, browser, or
scrolling behavior requiring Level 2 or Level 3. Criterion is the appropriate
evidence for the performance requirement.

| AC | Requirement | Strongest verification | Assessment |
|---|---|---|---|
| 1 | One graph build; ordinary validation skips compatibility and descendant verification | Level 1 source routing plus changed-child mechanism test | **Pass.** Construction and the fresh call remain adjacent. |
| 2 | `FileTree::ensure_built` uses the trusted fresh seam | Level 1 source routing and FileTree integration test | **Pass.** Construction and validation use the same graph options without a caller handoff. |
| 3 | Public prebuilt validation remains fail-closed before flattening | Level 1 provenance and changed/missing/unreadable descendant tests | **Pass.** Compatibility verification remains first. |
| 4 | Fresh and checked paths share one validation/report engine | Level 1 source inspection and parity test | **Pass.** Both call `validate_graph_contents`. |
| 5 | Fresh validation uses the build snapshot while checked reuse rejects staleness | Level 1 paired link- and heading-mutation tests | **Pass.** Both fresh/checked divergence cases are covered. |
| 6 | Provenance, mismatch, parity, FileTree, and presentation behavior remains valid | Level 1 focused current tests plus recorded complete area suites | **Pass.** The current focused selector is green and the implementation is unchanged since the recorded complete gates. |
| 7 | Public signatures, errors, reports, CLI output, and graph views remain unchanged | Level 1 API/source and serialization evidence | **Pass.** The snapshot remains private and absent from public views. |
| 8 | Same-session Criterion evidence satisfies the amended guards | Criterion evidence in `results.md` and Review 4's post-snapshot comparison | **Pass.** Mechanism, improvement, regression, and prebuilt-gap guards remain satisfied. |
| 9 | Focused tests, area gates, whitespace, and impact scope pass | Level 1 Nextest, Just build, prior complete test/lint, and repository tooling | **Pass with current-session limits.** Focused tests and builds pass; the current lint attempt exceeded the non-interactive ceiling and is not counted as a pass. |

## Verification and Scope

- `sniff` identifies the affected package area as `darkmatter`, with workspace
  packages `darkmatter`, `darkmatter-cli`, and `dmls`; `zed-dmls` is excluded
  from the Cargo workspace.
- GitNexus reports `validate`, `validate_fresh_graph`,
  `validate_graph_contents`, and `build_reference_graph` as **HIGH** impact,
  reaching 18, 33, 35, and 36 symbols respectively. `FileTree::ensure_built`
  is **MEDIUM** at 13 direct dependents. The checked and fragment helpers are
  **LOW**, and no indexed execution process crosses the reference subsystem.
- Current focused Level-1 Nextest selection: **PASS, 27/27**, covering both
  fresh/checked mutation tests, fragment parity, prebuilt mismatch variants,
  cache-root fragment behavior, and FileTree graph reuse.
- Current `just build` from the Darkmatter area: **PASS** for `darkmatter`,
  `darkmatter-cli`, and `dmls`.
- Current `just lint` was terminated after exceeding the non-interactive
  command ceiling. The Darkmatter library check completed and the CLI check
  had emitted no diagnostic, but the incomplete gate is not reported as a
  pass. Review cycle 3 records a complete area lint gate on the same source.
- Cross-platform execution was unavailable on this macOS host. Source review
  found only cross-platform Rust APIs in the changed implementation.

## Production Readiness

**Ready.** Every specified behavior has verification at the appropriate level,
the checked prebuilt boundary remains fail-closed, and the unimplemented
cache-first follow-up is a low-severity optimization rather than a correctness
or release blocker.
