---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-12T11:11:29-07:00
spec: 2026-10-15-diagnostic-node-positions/spec.md
implemented: false
description: "A **feature** review of `2026-10-15-diagnostic-node-positions/spec.md`"
feature: 2026-10-15-diagnostic-node-positions/review-1.md
---

# Review of Diagnostic Node Positions

The feature is production ready under the specification's contract. No high,
medium, or low findings were identified. No additional human review is required.

## Findings

None.

## Implementation review

Reviewed the feature changes from `d6ff65505` through `c2b05b944`, the
specification, implementation log, affected documentation, and repository
call sites of the public position map.

In the Darkmatter library,
[PositionMap](../../../darkmatter/lib/src/markdown/schemas/positions.rs)
retains separate key and value starts and walks decoded pointer segments
according to the source node's kind. The validation problem builder selects
the key for an unknown property, the containing mapping for a missing
property, and the value for other failures. The source tree and line index
are built before collecting problems and shared across union arms.

The reviewed paths include loaded Markdown, retained YAML without loaded
text, constructed Markdown, and source-free validation. Original loaded
bytes supply precise positions; mapping-order checks and alias rejection
prevent unsupported source projections from being used. Failed projection
retains the existing top-level scan. New nested paths fall back to their
authored ancestor, while new top-level paths have no position. Delimiter
offsets are applied once, and CRLF bytes remain intact.

The CLI change uses the existing terminal components to add the column to
human reports. Composition's existing error block already displays nested
coordinates and needed no production change. DMLS production range logic,
the shared source locator, and its hash-writing and interpolation consumers
were unchanged. Repository searches found no position-map construction or
lookup sites outside Darkmatter's library and DMLS.

## Verification

All required behavior has Level 1 verification, the level explicitly required
by acceptance criterion 7. This feature changes diagnostic data and report
text; it introduces no terminal input, scrolling, or styling requirement
that needs a real-terminal or keyboard-injection test.

| Acceptance criterion | Verification present and reviewed |
|---|---|
| 1: nested `sya` retains its identity and reports 5:9 | Library tests assert pointer, code, selected property, and coordinates; CLI tests assert JSON coordinates, human text, and failure exit code. |
| 2: list items, missing fields, keys, and flow values | Public library and CLI tests distinguish key/value columns and exclude list dashes. |
| 3: composition reports the same coordinates | A hermetic `md compose` test asserts the schema-failure block includes 5:9 and exits with failure. |
| 4: agreement with DMLS in both encodings | Shared complete documents cover ASCII and a supplementary-plane character, LF and CRLF, and UTF-8 and UTF-16. Tests compare byte offsets using an independent conversion and pin the text covered by existing editor ranges. Missing fields are checked with DMLS strict mode enabled. |
| 5: replacements, new paths, root errors, scalars, fallback, empty maps | Public validation tests exercise original nodes and ancestor fallback; CLI assignments assert original coordinates and JSON nulls. Tests cover scalar descendants, unsupported aliases without validity changes, and source-free maps. |
| 6: special keys, line endings, offsets, unchanged problem fields | Tests cover quoted, escaped, empty, and numeric string keys; leading comments, blank lines, BOM, and CRLF; root and property unions. Positioned reports are compared with source-free reports after removing only coordinates, preserving outcomes and problem order. |
| 7: tests, lint, portability, and consumers | The area's full test and lint recipes passed locally. New integration modules are declared in their consolidated L1 targets; DMLS tests are compiled unit tests. CLI processes use `CliProcessFixture`; shared fixture inputs are declared for CI. |

Executed on the current macOS host:

- `just test` in `darkmatter/`: **9,510 passed**, 11 skipped; all new position and DMLS parity tests ran and passed.
- `just lint` in `darkmatter/`: passed for the library, CLI, DMLS, and Zed CLI, including the Zed WASM compile check.
- `just check-tier-coverage darkmatter` from the repository root: passed with no stranded tests.

Cross-OS execution remains CI's responsibility, as requested. No source code
was changed during this review.

## Observations

None.
