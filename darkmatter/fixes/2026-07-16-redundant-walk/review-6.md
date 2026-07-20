---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-19T22:29:54-07:00
spec: 2026-07-16-redundant-walk/spec.md
implemented: false
description: "A **fix** review of `2026-07-16-redundant-walk/spec.md`"
fix: 2026-07-16-redundant-walk/review-6.md
previous: 2026-07-16-redundant-walk/review-5.md
---

# Review 6 — Redundant Walk

## Verdict

This fix is **ready for production**. The ordinary one-step and `FileTree`
paths validate their just-built graph through the internal fresh seam, the
caller-supplied prebuilt path retains its fail-closed provenance and descendant
checks, and both paths converge on one validation engine. Fragment validation
uses the graph's private build-time heading snapshot, so post-build child edits
cannot change the fresh report's composed heading set.

## Findings

No blocking findings.

### Low — Review 5's cache-first fragment optimization was not implemented

The prior review's only suggestion remains present in the current source.
`collect_composed_heading_slugs` seeds `HeadingSlugCache` from the graph-owned
snapshot, but `validate_cross_doc_fragment` still performs the target existence
and extension checks and then opens and parses the target with
`Markdown::try_from` before `cached_prepared_heading_slugs` consults the cache
(`darkmatter/lib/src/markdown/reference/validate.rs:916-932`). A file that is
both a graph descendant and a `path#fragment` target therefore incurs an
unnecessary read and parse even though its prepared slugs are already cached.

No focused Level-1 test covers that exact cache-first mechanism. The existing
tests prove the cached snapshot wins semantically, including after a heading
mutation, but they do not prove that a seeded entry avoids disk loading. This
does not weaken the checked trust boundary or violate the specified redundant-
compatibility-walk removal, so it remains a non-blocking performance and
coverage gap.

Resolve the canonical cache key after path resolution, consult the cache before
constructing `Markdown`, retain the disk fallback for non-descendant targets,
and add a Level-1 test where a transcluded child is also referenced as
`child.md#heading` with observable evidence that the seeded entry is used
without a target read.

## Implementation Assessment

- `validate` constructs one full graph and immediately calls
  `validate_fresh_graph`; it does not run compatibility or dependency-manifest
  verification.
- `FileTree::ensure_built` keeps construction adjacent to the same internal
  seam and aligns validation graph options with construction options.
- `validate_with_graph` calls `verify_graph_compatibility` before
  `validate_graph_contents`, retaining root, source, mode, options, and visited-
  descendant freshness checks for caller-supplied graphs.
- Both trust paths share `validate_graph_contents`, including flattening,
  fragment preparation, local/remote validation, fail-fast behavior, and
  report construction.
- `PreparedHeadingSnapshot` is private, omitted from `Debug` and serialized
  graph views, and populated from the graph build's existing prepared TOC
  parse.
- Public signatures, errors, report shapes, CLI behavior, and serialized graph
  views remain unchanged.

The implementation uses portable Rust filesystem and path APIs. No new
platform-specific behavior was introduced for macOS, Windows, or Linux.

## Requirement-to-Verification Assessment

This fix changes deterministic in-process library and filesystem behavior.
Level 1 is appropriate for every user-observable requirement. It introduces no
terminal rendering, terminal input encoding, keyboard, paste/IME, mouse,
browser, or scrolling behavior requiring Level 2 or Level 3. Criterion is the
appropriate evidence for the performance requirement.

| AC | Requirement | Strongest verification | Assessment |
|---|---|---|---|
| 1 | One graph build; ordinary validation skips compatibility and descendant verification | Level 1 source routing plus changed-child mechanism test | **Pass.** Construction and the fresh call are adjacent. |
| 2 | `FileTree::ensure_built` uses the trusted fresh seam | Level 1 source routing and FileTree integration test | **Pass.** The graph and clone-stable options remain visibly paired. |
| 3 | Public prebuilt validation remains fail-closed before flattening | Level 1 document/source/options and changed/missing/unreadable descendant tests | **Pass.** Compatibility verification remains first. |
| 4 | Fresh and checked paths share one validation/report engine | Level 1 source inspection and parity test | **Pass.** Both call `validate_graph_contents`. |
| 5 | Fresh validation uses the build snapshot while checked reuse rejects staleness | Level 1 paired link- and heading-mutation tests | **Pass.** Both fresh/checked divergence scenarios pass. |
| 6 | Provenance, mismatch, parity, FileTree, and presentation behavior remains valid | Level 1 focused current tests plus recorded complete area suites | **Pass.** Current focused coverage is green and the implementation source is unchanged since the recorded full gates. |
| 7 | Public signatures, errors, reports, CLI output, and graph views remain unchanged | Level 1 build/API/source and serialization evidence | **Pass.** The snapshot remains a private graph artifact. |
| 8 | Same-session Criterion evidence satisfies the amended guards | Criterion evidence in `results.md` and Review 4's follow-up comparison | **Pass.** The mechanism, improvement, regression, and prebuilt-gap guards remain satisfied. |
| 9 | Focused tests, area gates, whitespace, and impact scope pass | Level 1 Nextest, Just build, prior complete area test/lint, and repository tooling | **Pass with current-session limits.** Focused tests and builds pass; current full test/lint attempts were stopped at the non-interactive ceiling and are not counted as passes. |

## Verification and Scope

- `sniff` identifies the affected package area as `darkmatter`, with workspace
  packages `darkmatter`, `darkmatter-cli`, and `dmls`; `zed-dmls` is excluded
  from the Cargo workspace.
- GitNexus reports `validate` as **HIGH** impact (15 direct / 18 total, dominated
  by tests), while `validate_cross_doc_fragment`,
  `collect_composed_heading_slugs`, and `cached_prepared_heading_slugs` are
  **LOW** impact. No indexed execution process crosses the reference subsystem.
- Worktree-scoped GitNexus change detection reports low risk and no affected
  process. Its indexed changed symbols are from the user's unrelated dirty
  `biscuit-file` work; this review changes Markdown lifecycle documents only.
- Current focused Level-1 Nextest selection: **PASS, 27/27**, covering both
  fresh/checked mutation tests, fragment parity, prebuilt mismatch variants,
  cache-root fragment behavior, and FileTree graph reuse.
- Current `just build` from the Darkmatter area: **PASS** for `darkmatter`,
  `darkmatter-cli`, and `dmls`.
- Current `just test` was stopped after exceeding the non-interactive ~60-second
  ceiling: 2,023/5,862 Darkmatter tests had passed with no failure; the partial
  run is not reported as a pass. Earlier review cycles record complete area
  Level-1 suites on the same implementation.
- Current `just lint` and a narrower Darkmatter Clippy attempt were likewise
  stopped at the ceiling while compiling dependencies; neither emitted a
  diagnostic, but neither is reported as a pass. Earlier review cycles record
  a complete area lint gate on the same implementation.
- `md get` reads back every required review, previous-review, and specification
  frontmatter value. Review-schema validation is blocked by the repository's
  existing `schemas/feature-review.yaml` drift: it combines unsupported
  `$schema` and `description` keys with a tagged `kind`/`types` schema.
- Cross-platform execution was unavailable on this macOS host. The reviewed
  code uses cross-platform `Path`/`PathBuf`, `HashMap`, and filesystem APIs.

## Production Readiness

**Ready.** Every specified user-observable behavior has verification at the
appropriate level, the public checked boundary remains fail-closed, and the
unimplemented Review 5 cache-first item is a low-severity follow-up rather than
a correctness or release blocker.
