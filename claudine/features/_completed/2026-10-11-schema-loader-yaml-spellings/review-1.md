---
$schema: feature-review.yaml
ready: false
findings:
  - "high: Cross-fold suggestion ranges lack required regression coverage"
observations: []
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-11T07:44:15-07:00"
spec: "2026-10-11-schema-loader-yaml-spellings/spec.md"
implemented: true
next: "2026-10-11-schema-loader-yaml-spellings/review-2.md"
implemented_by: claude/opus
log: claudine/features/2026-10-11-schema-loader-yaml-spellings/log.md
description: "A **feature** review of `2026-10-11-schema-loader-yaml-spellings/spec.md`"
feature: "2026-10-11-schema-loader-yaml-spellings/review-1.md"
---

# Review 1

The implementation handles the two YAML presentations correctly in the cases
reviewed. It is **not production ready** under the review's acceptance rules
because one explicitly required source-range case has no retained regression
test. No runtime defect was reproduced, and no human design decision is needed.

Reviewed the feature changes from `3010cee8b` through the working tree, including
the source locator, both suggestion-projection paths, standalone loading,
language-server diagnostics and cursor lookup, CLI tests, and documentation.

## Findings

### High: Cross-fold suggestion ranges lack required regression coverage

**Authority — spec:** acceptance criterion 4 requires “Check exact structural
and suggestion ranges on continuation text and across a fold.” The
implementation plan also explicitly calls for folds within suggestion
candidates. The retained tests cover candidates located on continuation lines,
but no candidate's own range crosses a physical line break.

**Defect class:** missing regression coverage for a nonempty suggestion argument
whose decoded range projects across a YAML fold.

In the `darkmatter` library,
[the new suggestion-range test](../../../darkmatter/lib/tests/l1/schema_yaml_spellings.rs)
uses `suggest(no,` followed by `valid)` on the next line. Its expected candidate
ranges are `no` and `valid`, each on one physical line. The repeated-definition
and caller-offset cases have the same shape. The test's comment says it checks
ranges “across a fold,” but its assertions do not exercise that case.
Structural expression ranges cross folds in a separate test; those assertions
do not protect the suggestion argument's range.

This is a small testing change, not a request for a new projection layer.
A quoted argument inside a plain YAML scalar gives a realistic reproduction:

```yaml
kind: schema
types:
  mode: string(min(40); suggest('first
    second'))
  next: boolean
```

YAML decodes the candidate as `first second`. Its authored range must include
the opening quote, the physical line break and indentation, and the closing
quote. The candidate violates `min(40)`, so the advisory lint can assert the
same range while proving the schema still loads.

The sweep covered both changed library projection paths and their editor
consumer. Temporary L1 probes used the existing fixtures and public APIs,
tested LF and CRLF, and were removed after verification. They all passed:

| Site and purpose | Shape tested | Observed result | Expected result / retained coverage |
| --- | --- | --- | --- |
| `darkmatter` standalone loader, through [source-aware structural and suggestion projection](../../../darkmatter/lib/src/markdown/schemas/simplified/source.rs) | Two-line quoted candidate inside a plain definition scalar | Loads; candidate range includes both lines; structural `Argument` range and advisory lint range equal it | Behavior correct; retain this assertion in the feature suite |
| `darkmatter` [parse_yaml_schema_with_source](../../../darkmatter/lib/src/markdown/schemas/simplified/source.rs), used to project inline-schema suggestions | Same candidate, with a caller offset of 100 | Candidate range encloses both authored lines, with the offset applied once | Behavior correct; retain coverage for this separate scanner path |
| `dmls` [schema diagnostics](../../../darkmatter/dmls/src/diagnostics/frontmatter.rs), which turn candidate byte ranges into editor positions | Same standalone schema, UTF-16 client | One `dm.schema.invalid_suggestion` warning; exact two-line range | Behavior correct; retained editor test currently covers only a candidate on one line |
| Existing structural source-map test | Entire type expression crossing a fold | Exact enclosing expression range; focused test passes | Clean and already covered |
| Existing continuation-candidate and editor-warning tests | Candidate wholly on a continuation line | Exact single-line range; focused tests pass | Clean and already covered |

**Recommended fix:** extend the existing L1 tests with the two-line candidate
above. Assert the exact original-buffer candidate range, equality with the
structural argument and advisory lint ranges, and the caller offset through
the separate inline-schema projection API. Retain a compact editor warning
assertion for its multiline conversion. No new fixture framework, test tier,
or CI cell is needed.

## Requirement coverage and verification

All requirements here concern file parsing, passive APIs, or editor protocol
results. L1 is the appropriate verification level; real terminal rendering or
OS keyboard injection is not part of this feature.

| Acceptance requirement | Strongest retained verification | Assessment |
| --- | --- | --- |
| Equivalent semantics and referenced CLI validation | L1 library structural equality; hermetic CLI valid/invalid instances with reported schema and failure paths | Covered |
| Both envelopes, nested unions, compact mappings, siblings, and rejected `types` sequences | L1 paired presentation fixtures, public loading/validation matrix, and CLI cases | Covered |
| Multiple continuations, blank lines, comments, and no final newline | L1 comparison with `serde_yaml_ng` and conventional semantics | Covered |
| Exact structural/suggestion ranges, occurrence identity, CRLF, Unicode, and offsets | L1 exact-byte assertions | Covered except a suggestion candidate itself crossing a fold, as reported above |
| Editor grammar-error range, hover, completion, partial input, and UTF-16 | L1 in-memory language-server sessions | Covered |
| Strict exclusions, typed wrapper, tolerant lookup, anchors and aliases | L1 exclusion tests and existing regression suites | Covered by retained tests and recorded full-suite run |
| Corpus, normal invocation, full tests and lint, portable isolated tests | Recorded full-area gates; fresh focused runs and tier audit | No additional gap identified |

The plan's input robustness table defines outcomes for `kind`, `types`,
`$schema`, named and nested definitions, and property/root union members.
The retained matrix edits conventional and serializer-produced fixtures one
cell at a time and checks classification, normal loading, and validation.
Reviewed absent, null, wrong-type, empty, duplicate-key, and invalid/trailing
content cases and the touched readers' fallback behavior; no additional
finding emerged.

Fresh review verification on macOS:

- `just test schema_yaml_spellings`: **29 passed**.
- `just test schema_validate`: **55 passed**, including the new CLI cases.
- Two temporary L1 probes: **both passed**, then removed without changing the
  implementation or retained tests.
- `just check-tier-coverage darkmatter`: **0 stranded tests**. The library,
  CLI, and language-server tests are declared in their L1 targets.

The implementation log records the full `darkmatter/` test run as 9,150 passed
and 11 skipped, with clean lint, plus passing Claudine downstream gates.
Those full gates were not rerun during this review. Cross-OS evidence remains
the responsibility of CI and does not affect this readiness decision.

The recorded departures are reasonable: single-line standalone grammar errors
also receive precise token ranges, with a retained test; the description-only
fold previously loaded with a truncated span rather than always failing.
Neither departure changes the decided behavior or needs human approval.

## Observations

None.
