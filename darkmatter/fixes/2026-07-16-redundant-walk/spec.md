---
status: ready for planning and implementation
reviewed: true
created: 2026-07-16
review_iterations: 4
area: darkmatter
packages:
  - darkmatter
inputs:
  - ../../features/2026-07-15-reference-graph/review-4.md
amends:
  - ../../features/2026-07-15-reference-graph/spec.md#validation-contract
---

# Eliminate Redundant Reference-Graph Verification

## Problem

`Markdown::validate_references` builds a full `ReferenceGraph` and immediately
passes it to `validate_with_graph`. That function always runs
`verify_graph_compatibility` before flattening the graph. Compatibility checking
captures and compares the root document, source, mode, and graph-options
identities, then `verify_descendants` reopens and rehashes every unique visited
local child from disk.

Those checks are required for `Markdown::validate_references_with_graph`, where
the caller supplies a graph that may be stale or paired with the wrong request.
They are redundant when the same operation has just constructed the graph from
the same `Markdown` and `ReferenceGraphOptions`. The builder already loaded each
visited child and recorded its identity from that load. Reopening the child only
confirms the identity that was captured microseconds earlier.

The redundant work is material for transclusion-heavy documents. The reference-
graph benchmark's 12-child `multi_transclusion` fixture recorded:

| Function | Median |
|---|---:|
| `build_and_validate` | 10.455 ms |
| `validate_prebuilt` | 4.1522 ms |
| `construct` | 6.0571 ms |

The approximately 4.15 ms prebuilt-validation floor is dominated by reopening
and hashing those 12 children. The ordinary build-and-validate path currently
pays that floor immediately after its approximately 6.06 ms construction work.
By contrast, the larger single-document fixture has no dependency-manifest
entries and recorded `validate_prebuilt` at 105.17 microseconds.

> **Measurement correction (2026-07-18, review 1):** same-session decomposition
> (see `results.md`) shows descendant re-verification accounts for only
> ≈159 µs (≈1.5%) of that floor; the floor is dominated by the shared
> validation engine, which both paths keep. The redundant walk is real but
> smaller than estimated here — see the amended §"Performance acceptance".

The original performance comparison did not measure this regression against its
pre-opacity baseline: its baseline filter selected only `construct`. Review 4
therefore approved the feature but identified `build_and_validate` as a blind
spot that needs a focused baseline-versus-candidate measurement.

### Secondary behavior defect

The redundant check also gives `validate_references` an unintended hard-error
race. If a visited child changes after graph construction but before the
immediate verification read, the ordinary one-step method can return
`ReferenceGraphMismatch` instead of a validation report. The original feature
specification describes this path as compatibility-guaranteed by construction.
The one-step API should validate the coherent graph snapshot it just built; the
caller-supplied prebuilt API should continue to reject a snapshot that is no
longer current.

### A second internally fresh caller

`FileTree::ensure_built` has the same construction guarantee. It builds a graph
for the tree and immediately calls the public
`validate_references_with_graph` method to reuse it. This correctly avoids a
second graph build, but it still reopens every descendant. Because `FileTree`
does not accept a caller-supplied graph, its just-built graph can use the same
trusted fresh-graph seam as `validate_references`.

## Goals

1. `Markdown::validate_references` MUST build one full graph and validate that
   graph without running provenance compatibility checks or rereading the
   dependency manifest.
2. `FileTree::ensure_built` MUST reuse its just-built graph without running the
   redundant compatibility walk.
3. `Markdown::validate_references_with_graph` MUST continue to verify root
   document identity, source, `Full` mode, graph options, and every visited local
   descendant before flattening.
4. Fresh-graph and checked-prebuilt validation MUST share one implementation of
   graph flattening, fragment preparation, local/remote checks, fail-fast
   behavior, and report construction.
5. Public method signatures, report contents, graph contents, error types, and
   CLI behavior MUST remain unchanged.
6. A same-session baseline-versus-candidate measurement MUST demonstrate the
   effect on `build_and_validate`, including the transclusion-heavy fixture.

## Non-goals

This fix does not:

- weaken, remove, cache, or make optional the public prebuilt-graph freshness
  contract;
- remove provenance or the dependency manifest from `ReferenceGraph`;
- change what enters the dependency manifest or how identities are hashed;
- change reference extraction, transclusion traversal, graph flattening,
  fragment validation, remote validation, or report rendering;
- expose an unchecked public validation API;
- address Review 4's independent low-priority notes about
  `whole_state_fingerprint` serialization fallback or linear manifest dedup;
- add filesystem watchers, locks, or retry behavior around concurrent edits; or
- alter historical performance results in the completed feature directory.

## Design

### Separate graph freshness from graph validation

Split the current `validate_with_graph` body into two responsibilities:

```text
validate
  -> build_reference_graph
  -> validate_fresh_graph
       -> validate_graph_contents

validate_with_graph
  -> verify_graph_compatibility
  -> validate_graph_contents
```

`validate_graph_contents` is the single validation engine. It starts with graph
flattening and owns all behavior that follows it today. It does not decide
whether a graph is fresh.

`validate_with_graph` remains the checked prebuilt entry point used by
`Markdown::validate_references_with_graph`. It MUST call
`verify_graph_compatibility` before `validate_graph_contents`, preserving the
existing fail-closed ordering.

`validate_fresh_graph` is a narrowly visible internal entry point for a graph
that the current operation just built from the same `Markdown` and graph
options. `validate` and `FileTree::ensure_built` are its only intended production
callers. Its name and documentation MUST state the precondition; it MUST NOT be
publicly exported.

A named seam is preferred over a `verify: bool` argument. A Boolean makes an
incorrect `false` at the caller-supplied prebuilt boundary easy to write and hard
to notice in review. Separate checked and fresh entry points encode the trust
decision at the callsite while retaining one validation engine.

The implementation SHOULD use the narrowest visibility that permits the two
reference-module callers (for example `pub(super)` within
`markdown::reference`). Do not widen this to a general crate API.

### Freshness invariant

The unchecked internal seam is valid only when all of the following are true:

- the graph was returned by `build_reference_graph` or
  `Markdown::reference_graph` in the current operation;
- the same `Markdown` value is passed to validation;
- validation uses the same `ReferenceGraphOptions` value or a clone-stable clone
  of it; and
- no caller-controlled work occurs between building and validating the graph.

`validate` satisfies this invariant directly. `FileTree::ensure_built` satisfies
it after it assigns `validation_options.graph` from the same
`self.graph_options` used for construction. Both callsites MUST remain visibly
adjacent to graph construction so a future refactor cannot insert an external
handoff without reconsidering verification.

Any path that accepts a `ReferenceGraph` from its caller, stores it for later,
or cannot prove these conditions MUST use `validate_with_graph` and pay the full
check.

### Snapshot semantics

Fresh-graph validation uses the child contents held by the newly constructed
graph. A concurrent edit after a child was loaded does not invalidate that
one-step operation mid-call. This restores the intended snapshot behavior of
`validate_references`; it does not weaken the explicit reuse contract of
`validate_references_with_graph`, which continues to compare its older snapshot
against authoritative disk state before use.

### Comments and documentation

Update behavior-adjacent documentation in the same change:

- `validate` must no longer claim it delegates to the checked
  `validate_with_graph` path;
- `validate_with_graph` and `verify_graph_compatibility` must describe only the
  caller-supplied prebuilt contract, not freshly built graphs;
- `FileTree::ensure_built` must explain why its graph is eligible for the fresh
  seam; and
- benchmark comments must remain accurate about what `build_and_validate` and
  `validate_prebuilt` include.

The public `validate_references_with_graph` rustdoc and the Darkmatter skill's
prebuilt-graph guidance remain correct and SHOULD NOT be weakened. No README or
public migration note is required because the public API and its caller-visible
contract do not change.

## Correctness Verification

### Focused mechanism test

Add a unit test in the validation module that makes the seam observable without
timing or production instrumentation:

1. Build a graph from a root that transcludes a child.
2. Change the child's contents after construction, adding a broken reference.
3. Validate through `validate_fresh_graph`; it MUST validate the just-built
   snapshot and MUST NOT return `ReferenceGraphMismatch`.
4. Validate the same stale graph through checked `validate_with_graph`; it MUST
   return a changed-dependency `ReferenceGraphMismatch` before flattening.

This paired assertion proves the two trust paths differ only at the intended
freshness gate. Do not add a production global counter or filesystem abstraction
solely for this test.

### Existing invariant coverage

The complete prebuilt compatibility suite MUST stay green, especially:

- document, source, mode, and options mismatch rejection;
- edited, missing, unreadable, and cache-stale child rejection;
- clone-stable options and graph reuse;
- parity between checked prebuilt validation and one-step validation; and
- file-tree validation/report behavior.

Keep or strengthen the existing assertion that compatibility verification occurs
before flattening. The fresh seam MUST NOT become reachable through
`Markdown::validate_references_with_graph`.

## Performance Verification

### Required baseline order

Capture the baseline before implementation changes. Use the existing
`darkmatter/lib/benches/reference_graph.rs` fixture and benchmark functions
without changing fixture content, counts, sample size, or timed boundaries
between baseline and candidate.

Run a named Criterion baseline for all three `build_and_validate` fixtures, then
the candidate in the same session on the same host:

```text
cargo bench -p darkmatter --bench reference_graph -- build_and_validate \
  --save-baseline redundant-walk-before --warm-up-time 1 --measurement-time 4

cargo bench -p darkmatter --bench reference_graph -- build_and_validate \
  --baseline redundant-walk-before --warm-up-time 1 --measurement-time 4
```

If host load makes confidence intervals wide or non-overlapping fixture runs
noncomparable, rerun the baseline and candidate pair. Do not compare the
candidate against the older feature review's measurements as pass/fail evidence.

Record `results.md` beside this specification with:

- baseline and candidate commit/worktree state;
- exact commands and benchmark-source fingerprint;
- OS, architecture, Rust toolchain, and Criterion parameters;
- sample count, median, and confidence interval for each fixture;
- relevant host-load observations; and
- absolute and percentage deltas.

### Performance acceptance

The expected signal is concentrated in `multi_transclusion`, because `small`
and `large` have empty dependency manifests.

> **Amendment (2026-07-18, review 1):** the original ≥10%/≥500 µs improvement
> threshold was derived from the falsified premise that the ~4.15 ms
> `validate_prebuilt` floor was dominated by descendant re-verification.
> Same-run decomposition attributes ≈159 µs (≈1.5% of `build_and_validate`) to
> the removed walk. The acceptance requirement is therefore restated as a
> mechanism guarantee plus guards calibrated to the measured effect. This
> amendment resolves review 1's High finding.

- **Mechanism (primary).** The named call structure — `validate` and
  `FileTree::ensure_built` routing through `validate_fresh_graph`, the public
  prebuilt path routing through `validate_with_graph` into
  `verify_graph_compatibility` before flattening, and both converging on the
  single `validate_graph_contents` engine — plus the focused changed-child
  mechanism test (`fresh_seam_uses_snapshot_while_checked_path_rejects_stale_graph`)
  MUST demonstrate that the one-step path performs no compatibility or
  dependency-manifest verification while the checked path retains it in full.
  This is what prevents a fast result caused by accidentally weakening the
  public checked path.
- **Improvement guard (calibrated).** `multi_transclusion/build_and_validate`
  MUST improve by at least 100 microseconds at the median in the matched
  same-session run. This bar sits below the measured effect (≈159 µs by
  same-run decomposition; 461 µs quiet-window median delta) while remaining
  large enough to reject a noise-only result in a tight-CI quiet-window run.
- **Regression guard.** No `build_and_validate` fixture may regress by both
  more than 5% and more than 100 microseconds at the median.
- **Prebuilt-gap guard.** A final unfiltered reference-graph benchmark run MUST
  still show `validate_prebuilt` materially faster than `build_and_validate`;
  the safe prebuilt reuse win remains part of the original feature contract.

The benchmark is evidence of the optimization, not the primary guard. The
mechanism requirement above is now the primary guard: the named call structure
and focused mechanism test are what prevent a fast result caused by
accidentally weakening the public checked path.

## Impact and Verification Scope

`sniff` identifies `darkmatter` as the package area and reports the area members
`darkmatter`, `darkmatter-cli`, `dmls`, and the workspace-excluded `zed-dmls`.
The implementation changes only the `darkmatter` library. Its declared workspace
consumers include `darkmatter-cli`, `dmls`, Claudine packages, and several other
CLI/library packages, but this fix changes no public signature or type.

GitNexus analysis on the pre-fix revision reports:

- `validate`: HIGH, 17 symbols / 15 direct dependents, dominated by unit tests;
- `validate_with_graph`: LOW, three direct callers;
- `verify_graph_compatibility`: LOW, one direct caller;
- `validate_references_with_graph`: LOW, no indexed upstream callers; and
- `FileTree::ensure_built`: MEDIUM, 13 direct dependents across file-tree and
  integration tests.

No indexed execution process crosses the reference/file-tree boundary. Rerun
impact analysis before implementation because the index may have changed, and
run `detect_changes` after implementation to confirm that only the expected
reference-validation and file-tree flows moved.

Required gates from the `darkmatter` package area:

- focused validation and reference integration tests;
- `just build`;
- `just test`;
- `just lint`;
- `git diff --check`; and
- GitNexus `detect_changes` for the implementation diff.

These area recipes cover the affected library plus `darkmatter-cli` and `dmls`.
Additional downstream package gates are required only if the refreshed impact or
change analysis finds a public or cross-area effect. Level 2 and Level 3 tests
are not required because this fix changes no terminal query, rendering bytes,
browser behavior, or host input handling.

## Acceptance Criteria

1. `validate_references` performs one graph build and no compatibility or
   dependency-manifest verification before validating that fresh graph.
2. `FileTree::ensure_built` uses the same trusted fresh-graph validation seam and
   does not route its just-built graph through the public checked-prebuilt path.
3. `validate_references_with_graph` still performs the complete fail-closed
   compatibility check before graph flattening.
4. Fresh and checked paths share one post-verification validation engine; no
   validation/report logic is duplicated.
5. The focused changed-child test proves the fresh path uses its snapshot while
   the checked path rejects the same stale graph.
6. All existing reference-graph provenance, mismatch, parity, file-tree, and
   presentation tests remain green.
7. Public Rust signatures, errors, report contents, CLI output, and serialized
   graph views are unchanged.
8. Same-session Criterion evidence is recorded in `results.md` and satisfies the
   mechanism, improvement, regression, and prebuilt-gap guards in
   §"Performance acceptance".
9. Focused tests, Darkmatter build/test/lint, whitespace, and GitNexus scope
   gates pass.

## Amendments

- **2026-07-18 (review 1, High finding).** Per `review-1.md`, §"Performance
  acceptance" and acceptance criterion 8 were amended to replace the disproven
  ≥10%/≥500 µs improvement threshold with a mechanism-based requirement plus a
  benchmark guard calibrated to the measured effect (≈159 µs / ≈1.5% by
  same-run decomposition; 461 µs / 4.4% quiet-window median delta). The
  falsified ~4.15 ms walk-cost premise in §"Problem" was annotated with a
  measurement-correction note, not rewritten, to preserve the audit trail.
  Under the amended guards the evidence already recorded in `results.md`
  satisfies acceptance criterion 8: the mechanism guard is met by the Phase-4
  mechanism test and the named seams; the improvement guard is met
  (461 µs ≥ 100 µs); the regression guard is met (all fixtures improved or
  flat); and the prebuilt-gap guard is met (2.4×–15×).
