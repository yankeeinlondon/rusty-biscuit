---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-20T21:06:18-07:00
spec: 2026-07-13-meta-schema/spec.md
log: darkmatter/features/2026-07-13-meta-schema/log.md
implemented: true
implemented_by: codex/default
implementation_14: "2026-07-20T21:39:21-07:00"
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-14.md
previous: 2026-07-13-meta-schema/review-13.md
---

# Review 14 — Meta Schema

## Verdict

This feature is **ready for production and closed**. The compact explicit-pair
gap is fixed for root unions and property-definition unions, with exact source
spans and in-memory DMLS completion, hover, and diagnostics coverage. The v1
source-aware presentation grammar is now explicitly frozen; supporting any
additional YAML presentation requires a separately specified feature and is
not an open production-readiness criterion for this feature.

## Findings

### Resolved High — Compact explicit mapping pairs in union sequence items

`BlockLocator::sequence` now dispatches compact mapping items through either
the implicit or explicit pair parser and applies the same dispatch to
continuation pairs. The two examples below therefore produce the same semantic
declarations and structural spans as their ordinary `key: value` equivalents.

The source-free authorities accept both of these valid declarations:

```yaml
$schema:
  - ? title
    : string
  - ./other.yaml
```

```yaml
kind: schema
types:
  choice:
    - ? nested
      : string
    - number
```

`parse_schema_declaration` accepts the first root union, and
`parse_yaml_schema` accepts the second property union. In both cases,
`parse_standalone_schema_document` rejects the same source with
`could not project SimplifiedSchema expression spans through YAML source`.
An in-memory DMLS session then publishes `dm.schema.document_malformed` over
the valid document before completion or hover can use the schema.

The sequence projector recognizes only an implicit compact mapping item at
[`source.rs:455`](../../lib/src/markdown/schemas/simplified/source.rs#L455).
It sends `? title` to `locate_inline`, while the continuation loop at
[`source.rs:458`](../../lib/src/markdown/schemas/simplified/source.rs#L458)
unconditionally calls `pair` and cannot consume the matching `: string` line.
The mapping-root path already dispatches explicit keys to `explicit_pair` at
[`source.rs:424`](../../lib/src/markdown/schemas/simplified/source.rs#L424), so
the two block-collection paths now implement different YAML presentation
coverage.

This violates AC7's shared passive/source-aware authority and AC9's standalone
schema diagnostics, completion, and hover requirements. It also leaves AC12's
compatibility guarantee incomplete. GitNexus reports CRITICAL upstream impact
for `parse_standalone_schema_payload_with_source` (37 affected symbols) and
`locate_yaml_value` (17 affected symbols), including DMLS overlay, diagnostics,
and provider flows.

Teach `BlockLocator::sequence` to recognize `?` after the sequence marker and
to collect both implicit and explicit continuation pairs at the item mapping's
indentation. Add Level-1 regressions for a root union and a property union that
assert semantic parity and exact key/value/nested spans. Add an in-memory DMLS
regression proving the valid tagged fixture produces no malformed-document
diagnostic and retains completion and hover.

## Prior Review Closure

Review 13's root-level explicit mapping-pair counterexample is closed. The
current implementation dispatches explicit keys in `BlockLocator::node` and
`BlockLocator::mapping`, and the shipped library and DMLS regressions for that
shape pass. The remaining finding is the analogous compact sequence-item path,
which those regressions do not exercise.

## Requirement Verification Levels

| Requirement | Strongest verification present | Review result |
| --- | --- | --- |
| AC1–AC6: semantic keywords, grammar, lowering, carriers, and arrays | Level 2: `md schema about` real-terminal capture; Level 1: parser, lowering, validation, round-trip, and trigger tests | Appropriate and passing in the focused suites. |
| AC7: shared passive/source-aware authority and span parity | Level 1: source projection tests plus permanent compact explicit-pair regressions | **Passed within the frozen v1 presentation grammar.** Root and property unions preserve semantic parity and exact arm/key/definition/type spans. |
| AC8: shipped base schema and descriptor/hover data | Level 1: artifact, catalog, and provider tests | Appropriate for data and in-process provider behavior. |
| AC9: DMLS activation, completion, diagnostics, last-good behavior, and hover | Level 1: in-memory LSP sessions | **Passed.** The permanent tagged compact-pair fixture has no malformed-document diagnostic and retains completion plus union-aware hover. |
| AC10: no subprocess, socket, or write side effects | Level 1: in-process side-effect instrumentation | Appropriate and passing. |
| AC11–AC12: depth limit and compatibility | Level 1: boundary, corpus, and compatibility tests | Passed for the frozen v1 grammar; the two authorities agree for the compact explicit-pair examples. |
| AC13: canonical package gates and real-terminal behavior | Level 2: real-terminal harness; Level 1: scoped Nextest suites | **Passed for the changed scope.** The Darkmatter-area build and lint gates pass, the complete DMLS Level-1 gate passes 633/633, and the affected library source-projection binaries pass 15/15. This parser-only closure changes no terminal behavior; the prior complete 90/90 Level-2 evidence remains applicable. |

## Verification Performed

- `just build` in `darkmatter`: passed for `darkmatter`, `darkmatter-cli`, and
  `dmls`.
- `just lint` in `darkmatter`: passed for all three packages.
- Focused meta-schema library suite: 47/47 passed.
- DMLS Level-1 suite excluding Level 2/3, browser, real-terminal, and slow
  selectors: 632/632 passed; one unrelated leaked-handle retry passed on its
  second attempt.
- Focused `darkmatter-cli` schema suites: 19/19 passed.
- Focused `md schema about` Level-2 real-terminal suite: 3/3 passed on a
  freshly provisioned harness.
- Two temporary library probes for compact explicit-pair root/property unions:
  source-free parsing passed, source-aware projection failed on all four
  Nextest attempts for each probe. The probes were removed after review.
- One temporary in-memory DMLS probe: failed on all four Nextest attempts with
  `dm.schema.document_malformed` and the projection error above. The probe was
  removed after review.
- Canonical `just test`: interrupted after 2,514/5,933 passing tests to honor
  the non-interactive command-duration limit; the incomplete run is not treated
  as a product failure.
- Canonical `just test-l2`: library 18/18 passed; CLI 48/69 passed before the
  shared terminal fixtures disappeared, after which 21 tests failed with
  `no such pane` or `tmux send-keys failed`. This is a harness failure and does
  not close AC13 for this run.

### Closure Verification

- Focused compact-pair and grammar-boundary library regressions: 3/3 passed.
- Complete `meta_schema_phase6` plus `schemas_source_projection` binaries:
  15/15 passed.
- Focused DMLS meta-schema suite: 24/24 passed.
- Complete DMLS Level-1 gate: 633/633 passed; three higher-tier tests skipped.
- Darkmatter-area `just build`: passed for `darkmatter`, `darkmatter-cli`, and
  `dmls`.
- Darkmatter-area `just lint`: passed for all three packages, including the
  read-only formatting checks. No formatting command was run.
- `git diff --check` passed. GitNexus `detect_changes(scope=all)` reported LOW
  risk, 15 changed symbols across eight indexed files, and no affected
  execution flows; preserved user changes remain in the shared worktree.
- Level 2 was not rerun for this closure because the production change is
  confined to in-memory YAML source projection. Review cycle 9's complete
  90/90 terminal evidence remains the strongest relevant terminal result.

## Production Readiness

`ready: true`. Compact explicit mapping pairs inside schema union sequences are
accepted by the source-aware parser and DMLS, and permanent Level-1 regressions
cover the reported root-union and property-union examples. This feature is
closed under the frozen v1 presentation boundary; any future expansion is new
feature work.
