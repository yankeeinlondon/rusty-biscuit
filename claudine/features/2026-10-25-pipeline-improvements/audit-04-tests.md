---
kind: implementation-log
---

# Test audit: 2026-10-14-union-type-constraints

Added **2 test functions / 2 Nextest identities**, both genuinely new coverage.
No existing identity was replaced, split, or removed: gross and net additions
are both **2**. Strengthened existing tests: **0**. Slow tests modified: **0**.
No production behavior changed.

The existing authored-default test exercises bracketed default text, which the
grammar retains as a string. It does not exercise an actual JSON array supplied
through the public schema model. The new tests build that model and call the
public named-type expansion and lowering APIs:

- Equal inherited-arm and container defaults merge after recursive numeric
  normalization (`["a", 3]` equals `["a", 3.0]`). The default appears once on
  the container and disappears from item arms, whose schemas remain unchanged.
- Conflicting array defaults fail with `ConstraintConflict::Default`, naming
  the definition arm and use site. Silently discarding or overwriting either
  default would fail the test.

## Requirement coverage

| Requirement | Evidence |
| --- | --- |
| Content Policy acceptance/rejection and flattened lowering | `schemas_union_postfix`, fixture controls, schema matrix, and real CLI acceptance cases cover mixed lists, absent/null property, null item, wrong outer shape, empty list, unmatched values, required rule, and action values. |
| Allowed postfixes and typed rejection/provenance | Public resolver tests cover bare, `@this`, imported and multi-hop references; markers before/after `[]`; inherited markers/descriptions; hidden array arms; nested arrays; contradictory bounds; unsupported constraints; aliases; and referenced-error wrappers. |
| Container defaults | Existing scalar/default rejection cases plus both new real-array-default regressions. |
| Reporting and discriminants | `schemas_union_report` checks independent items, literal tags, missing/unknown/duplicate/conflicting tags, overlapping arms, problem descriptions and property/item parity. |
| Safe coercion | `schemas_union_coerce` and real CLI composition cases check independent candidates, exact accepted boolish/numberlike values, nested conversion, ambiguous alternatives, failed-list noncommit, and property/item parity. |
| Safe eager-file rewriting | `schemas_union_rewrite` and real CLI cases check unambiguous file/object arms, lazy arms, overlapping eligible arms, read-only validation, and idempotence. |
| Conservative cleanup | `schemas_union_clean` preserves extra keys and mistyped union values, including tagged items; verifies a repairable non-union sibling separately and respects the current whole-document repair gate. |
| Typed consumers | Completion, typed-consumer, and DMLS protocol suites cover applicable keys/values, literal-tag narrowing, matching/ambiguous/invalid/missing hover, phase timing, frontmatter shape, detection, catalogs, and trigger matching. |
| CLI/editor wiring | Hermetic real `md schema validate`, `md schema about`, and composition tests assert actual outputs and file immutability. In-memory LSP sessions exercise diagnostics, completion, hover, related locations, and dependency refresh. |
| Shipped artifacts | The passive shipped-schema corpus runs alongside the union suites. Later pipeline features intentionally extend grammar and lowering, so this audit does not compare the integrated implementation against an obsolete historical byte fingerprint. No lowering code was changed in this audit. |

The schema transformations are synthetic transformation contracts (F1), tested
through the real library APIs. JSON Schema validation uses the actual external
`jsonschema` implementation, and real CLI/protocol consumers exercise wiring.
There is no provider, hardware, terminal-input, or browser assumption in this
feature. No L2 test is applicable; no windows were opened.

## Verification

From `darkmatter/`, with the canonical Nextest-backed recipe:

```sh
BISCUIT_TEST_FILTER='test(schemas_union_) | test(schema_union_) | test(union_item_editor::) | test(named_type_references::) | test(shipped_schema_corpus::)' just test
```

Initial parallel L1 run: **179/179 passed**, total test execution 2.662 seconds.
After adding the two tests, the narrower `schemas_union_postfix::` run passed
**34/34**, then the final parallel run passed **181/181**, total test execution
2.483 seconds; maximum individual duration **1.092 seconds**. Every
individual test completed below five seconds. Logs:
`/tmp/pipeline-s04-tests-initial.log`, `/tmp/pipeline-s04-defaults.log`, and
`/tmp/pipeline-s04-tests.log`.

`git diff --check` passed. No formatting command, full-area test gate,
cross-OS execution, or commit was performed by this slice. The coordinator
owns the final package gates after the serial audits complete.
