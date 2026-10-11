---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-11T07:59:16-07:00"
spec: "2026-10-11-schema-loader-yaml-spellings/spec.md"
implemented: false
description: "A **feature** review of `2026-10-11-schema-loader-yaml-spellings/spec.md`"
feature: "2026-10-11-schema-loader-yaml-spellings/review-2.md"
previous: "2026-10-11-schema-loader-yaml-spellings/review-1.md"
---

# Review 2

The feature is **production ready** under this review's acceptance rules.
The previous blocking coverage gap is repaired. No new finding, recurring
finding, or human design decision was identified.

Reviewed the specification, plan, both implementation logs, review 1, and
the implementation changes from `3010cee8b` through the current working tree.
The review covered the strict and tolerant locators, scalar decoding,
structural and suggestion projection, standalone loading, editor diagnostics
and cursor lookup, CLI validation, tests, and behavior documentation.

## Previous findings and repair verification

Review 1 has a single finding under `## Findings`, rather than separate
unblocked and blocked sections. Its high finding, **Cross-fold suggestion
ranges lack required regression coverage**, was unblocked and is implemented.
There were no blocked findings to reconsider. The repair log reports no
deferred or disputed findings, and inspection agrees with its account.

Acceptance criterion 4 requires exact structural and suggestion ranges
“on continuation text and across a fold.” The new tests now exercise a quoted
suggestion argument inside a plain YAML scalar whose own range spans two
physical lines. They retain the original buffer and assert LF and CRLF
coordinates. This closes the missing regression coverage without runtime
changes or a new projection layer.

The repaired defect class is missing coverage for a suggestion argument
projected across a YAML fold. The complete consumer sweep is:

| Site in the changed implementation | Shape tested | Observed and expected result |
| --- | --- | --- |
| `darkmatter` standalone loader and located suggestion projection, [library test](../../../darkmatter/lib/tests/l1/schema_yaml_spellings.rs) | `suggest('first` followed by `second')`, with `min(40)`; LF and CRLF | Schema loads; decoded candidate is `first second`; exact enclosing candidate range equals the structural argument and advisory lint ranges; following sibling stays intact |
| `darkmatter` inline-schema suggestion scanner, [library test](../../../darkmatter/lib/tests/l1/schema_yaml_spellings.rs) | Same candidate with caller offset 100; LF and CRLF | Exact enclosing authored range, with the offset applied once |
| `dmls` standalone warning diagnostics, [editor protocol test](../../../darkmatter/dmls/tests/l1/schema_yaml_spellings.rs) | Cross-fold candidate with non-ASCII text; UTF-16 client; LF and CRLF | One advisory warning with the exact two-line range; following sibling remains available to hover |
| `dmls` inline-frontmatter warning projection, [diagnostic module test](../../../darkmatter/dmls/src/diagnostics/frontmatter.rs) | Cross-fold candidate inside Markdown frontmatter; nonzero frontmatter offset, non-ASCII text, LF and CRLF | One lint with the exact original-document byte range; conversion gives the expected two-line UTF-16 range |
| Existing structural and continuation-only suggestion projection | Whole expressions across folds; individual candidates entirely on continuation lines | Exact retained ranges pass; these complement rather than substitute for the new argument-crossing assertions |

Suggestion lint and query consumers use the projected spans; the standalone
test's equality assertions cover their shared coordinates. Expression-relative
grammar errors have their own retained editor tests and do not use suggestion
candidate projection. The repair also corrected the earlier test comment
that overstated its cross-fold coverage.

## Unblocked Findings

None.

## Blocked Findings

None.

## Requirement coverage and verification

Every requirement concerns passive parsing, library results, CLI validation,
or editor protocol results. Level 1 is appropriate for each; this feature
does not promise terminal rendering or keyboard behavior requiring another
verification level.

| Acceptance requirement | Verification level and evidence | Assessment |
| --- | --- | --- |
| 1. Equivalent semantics and referenced CLI validation | L1 structural JSON Schema equality and isolated CLI valid/invalid instances with failure paths and constraint messages | Covered |
| 2. Both envelopes, root/property unions, compact mappings, siblings, rejected `types` sequences | L1 paired fixtures, public loading/validation matrices, and CLI rejection | Covered |
| 3. Mapping and sequence continuations, multiple lines, blank lines, comments, no final newline | L1 comparison with `serde_yaml_ng` and equivalent conventional spellings | Covered |
| 4. Exact structural and suggestion ranges, lint, occurrence identity, CRLF, Unicode, offsets | L1 original-buffer assertions, including all four newly retained cross-fold cases | Previous gap closed |
| 5. Malformed-token diagnostics, hover, completion, partial input, UTF-16 | L1 in-memory language-server sessions and cursor tests | Covered |
| 6. Strict exclusions, error wrapper, tolerant lookup, anchors and aliases | L1 exclusion and supported-presentation cases; existing regression suites in the recorded full run | Covered |
| 7. Corpus, normal invocation, tests/lint, isolated portable fixtures | Recorded full-area gates, fresh focused nextest runs, and fresh tier audit | Covered |

The plan defines the YAML input robustness outcomes for `kind`, `types`,
`$schema`, named/nested definitions, property-union members, and root-union
members. The six retained matrices run conventional and serializer-produced
fixtures through public classification, reference loading, and validation.
Their controls prove the tested definitions affect validation. Absent, null,
wrong whole type, one/all wrong elements, empty, duplicate-key, and invalid or
trailing content cases pass. Reviewed fallback and filtering sites do not
introduce a new permissive-default defect within this change's boundary.

The new tests are compiled by the library and language server's declared
targets and selected by L1. They use explicit line endings, isolated fixtures,
and no terminal or browser windows. Their measured runtimes are below 0.2 s.

Fresh verification on macOS through the existing recipes:

- `just test schema_yaml_spellings`: **32 passed**.
- `just test a_folded_inline_suggestion_converts_to_one_two_line_range`: **1 passed**.
- `just test schema_validate`: **55 passed**.
- `just check-tier-coverage darkmatter`: **0 stranded tests**.

The repair log records the full `darkmatter/` `just test` run as **9,154
passed, 11 skipped, 0 failed**, followed by clean `just lint`. Those full gates
were not rerun for this review; the fresh runs verify the repaired class and
normal CLI consumers. Cross-OS evidence remains a CI responsibility and does
not block this readiness decision.

## Recurrence

None. The previous finding is resolved rather than repeated.

## Observations

None.
