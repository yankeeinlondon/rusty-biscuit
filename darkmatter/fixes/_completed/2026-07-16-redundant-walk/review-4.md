---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-19T21:29:43-07:00
spec: 2026-07-16-redundant-walk/spec.md
implemented: true
description: "A **fix** review of `2026-07-16-redundant-walk/spec.md`"
fix: 2026-07-16-redundant-walk/review-4.md
previous: 2026-07-16-redundant-walk/review-3.md
next: 2026-07-16-redundant-walk/review-5.md
---

# Review 4 — Redundant Walk

## Verdict

This fix is **ready for production**. The prior High finding is resolved:
reference-graph construction now retains each visited file child's prepared
heading slugs in a private `PreparedHeadingSnapshot`, and fragment validation
uses that build-time snapshot rather than post-build child contents. The public
checked-prebuilt path still verifies provenance and every visited local
descendant before it reaches the shared validation engine.

## Findings

No blocking findings.

### Low — Cross-document fragment validation opens a descendant before using its seeded snapshot entry

`collect_composed_heading_slugs` seeds `HeadingSlugCache` from the graph-owned
snapshot, but `validate_cross_doc_fragment` still checks the target on disk and
constructs `Markdown` before calling `cached_prepared_heading_slugs`
(`validate.rs:911-929`). When a file is both a graph descendant and a
`path#fragment` target, the cached snapshot correctly wins for the heading
result, but the preceding file read and parse are unnecessary.

This does not violate the fix's required compatibility-walk removal and does
not undermine the tested snapshot behavior, so it is not production-blocking.
As a follow-up optimization, consult the cache by `heading_slug_key` before
loading the target, while retaining the existing disk path for targets that
were not graph descendants. A Level-1 test with a transcluded child also linked
as `child.md#heading` would be sufficient.

## Implementation Assessment

The implementation matches the intended trust-boundary design:

- `validate` and `FileTree::ensure_built` build a graph and immediately use the
  narrowly visible `validate_fresh_graph` seam.
- `validate_with_graph` performs `verify_graph_compatibility` before the shared
  `validate_graph_contents` engine.
- `build_node` derives heading slugs from the same prepared content already
  used for reference extraction and records them under a cross-platform
  canonical path key.
- The heading snapshot is private, omitted from `Debug` and serialized graph
  views, and cloned only as part of the opaque graph artifact. Public method
  signatures, errors, report contents, CLI output, and graph JSON remain
  unchanged.
- The paired heading-mutation test enables fragment validation, changes the
  child heading after construction, proves the fresh report retains the
  build-time heading, and proves checked reuse fails with a changed-dependency
  mismatch.

The extra snapshot storage is proportional to visited headings, avoids storing
full duplicate document contents, and is a reasonable performance/ergonomics
trade-off for coherent fresh validation.

## Requirement-to-Verification Assessment

This fix changes in-process library and filesystem behavior. Level 1 is the
appropriate level for every user-observable requirement; there is no terminal
rendering, terminal input encoding, keyboard, paste/IME, mouse, or scrolling
behavior requiring Level 2 or Level 3. Criterion is the appropriate evidence
for the performance requirement.

| AC | Requirement | Strongest verification | Assessment |
|---|---|---|---|
| 1 | One graph build; ordinary validation skips compatibility and dependency-manifest verification | Level 1 source routing plus changed-child mechanism test | **Pass.** `validate` routes directly from construction to `validate_fresh_graph`. |
| 2 | `FileTree::ensure_built` uses the trusted fresh seam | Level 1 source routing and existing file-tree tests | **Pass.** Construction and validation remain adjacent with clone-stable options. |
| 3 | Public prebuilt validation remains fail-closed before flattening | Level 1 stale-descendant/provenance tests and source ordering | **Pass.** Compatibility verification remains the first checked-prebuilt operation. |
| 4 | Fresh and checked paths share one post-verification engine | Level 1 source inspection and parity tests | **Pass.** Both paths converge on `validate_graph_contents`. |
| 5 | Changed-child tests prove fresh snapshot behavior while checked validation rejects staleness | Level 1 paired link-mutation and heading-mutation tests | **Pass.** Both focused mechanism tests pass, including `validate_fragments: true`. |
| 6 | Existing provenance, mismatch, parity, file-tree, and presentation behavior remains valid | Level 1 existing suites plus focused current mechanism run | **Pass.** The new private artifact does not change public report or presentation shapes. |
| 7 | Public signatures, errors, reports, CLI output, and graph views remain unchanged | Level 1 compile/API inspection, serialization design, and downstream builds | **Pass.** The snapshot is private and omitted from public views. |
| 8 | Same-session evidence satisfies all performance guards | Criterion baseline comparison and unfiltered multi-transclusion run | **Pass.** Current `multi_transclusion/build_and_validate` improved about 235 microseconds at the median; no fixture crossed the regression guard; checked prebuilt validation remains about 2.6x faster. |
| 9 | Focused tests, area build/test/lint, whitespace, and change-scope gates pass | Level 1 Nextest, area builds, repository tooling, and prior complete area gates | **Pass with environment note.** Current focused tests and all three area builds pass, and `git diff --check` is clean. The current aggregate lint attempt emitted no diagnostics but was terminated at the non-interactive command ceiling while compiling dependencies; review 3 records the preceding complete area lint/test gates. |

## Verification Performed

- Focused current Level-1 mechanism tests: **PASS**, 2/2.
- Current package-area builds: **PASS** for `darkmatter`, `darkmatter-cli`, and
  `dmls`.
- Current Criterion comparison against `redundant-walk-before`:
  - small median 219.14 microseconds, 28.77% improved;
  - large median 6.6032 milliseconds, 1.88% slower and within the regression
    guard;
  - multi-transclusion median 10.292 milliseconds, 4.60% / approximately
    235 microseconds improved, satisfying the 100-microsecond guard.
- Current unfiltered multi-transclusion run: `build_and_validate` median
  10.693 milliseconds versus `validate_prebuilt` 4.1787 milliseconds, retaining
  a material approximately 2.6x prebuilt-reuse advantage.
- `git diff --check`: **PASS**.
- GitNexus impact for the new build/snapshot/collection symbols: **LOW**, no
  affected indexed execution processes; affected production scope remains the
  Darkmatter reference subsystem.
- `sniff` scope: `darkmatter`, `darkmatter-cli`, and `dmls`; public downstream
  types and signatures did not change.

The aggregate `just lint` command was stopped after it exceeded the session's
non-interactive duration ceiling; it had produced no diagnostic. A subsequent
narrow Clippy attempt was blocked by another Cargo process sharing the build
directory. `cargo fmt --check` was unavailable because this session's active
toolchain does not have the `rustfmt` component. No tool installation or
workspace-wide fallback was attempted.

Cross-platform execution was unavailable in this macOS session. The added code
uses portable `Path`/`PathBuf`, `HashMap`, and Rust filesystem APIs; best-effort
canonicalization preserves the existing fallback on macOS, Windows, and Linux.

## Production Readiness

**Ready.** Every behavior changed by this fix has Level-1 verification at the
appropriate boundary, the previous fragment-snapshot gap is closed, the public
checked path remains fail-closed, and the current implementation still meets
the amended performance guards.
