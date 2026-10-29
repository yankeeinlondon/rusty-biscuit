---
kind: implementation-log
---

# Test audit: 2026-10-11-schema-loader-yaml-spellings

Added **1 test**, strengthened **1 existing test**, and modified **0 slow tests**.
All 218 selected tests passed; the slowest took **2.518 seconds** on this host.

The new DMLS test checks completion after non-BMP text on a continuation line
in UTF-8 and UTF-16, with explicit LF and CRLF buffers (four cases in one test).
The existing completion matrix now checks the edit's end and replacement text
as well as its start. An implementation returning the correct start but
deleting arguments or sibling content can no longer pass that test.

## Coverage against the requirements

| Requirement | Evidence and boundary |
| --- | --- |
| Equal schema meaning and application | Library presentation-pair equality, plus CLI valid/invalid instances with asserted problem paths and messages. Hermetic real `md` invocation proves command wiring. |
| Both envelopes, nested lists, compact mappings, siblings | Library robustness matrices and exact span tests; CLI root-union and rejected `types` sequence cases. |
| Folding, blank lines, comments, missing final newline | Decoded values compared with the YAML parser and equivalent single-line schema semantics. Serializer-produced fixtures are checked byte-for-byte against actual `serde_yaml_ng` output, crossing the foreign format boundary with native producer evidence. |
| Exact spans, suggestions, repeated definitions | Structural and suggestion span assertions, advisory-lint checks, explicit line endings, non-ASCII text, and nonzero source offsets. |
| Editor diagnostics, hover, completion | In-memory LSP harness tests check negotiated encodings and exact authored ranges, including malformed and partially typed input. The added test distinguishes byte columns from UTF-16 columns after a non-BMP character. |
| Presentation boundary and fallbacks | Strict rejection and source/value mismatch cases; anchor/alias coverage remains. Literal/folded block scalars are now supported by later work, so the obsolete original exclusion was not reinstated. |
| Shipped artifacts and invocation | Shipped-schema validation/projection cases and normal CLI validation tests passed. |

The schema transformation checks use synthetic inputs where their contract is
the transformation itself. The serializer capture supplies native format
evidence; the real CLI and protocol harness supply invocation evidence. No
requirement concerns terminal rendering, terminal input, a browser, or a live
service, so L2 is not applicable and no windows were opened.

## Build defect repaired

The consolidated library L1 target initially could not compile: its lifecycle
protocol experiment imported two Rust modules from an active feature directory
that had moved. The unchanged model and 15 existing scenario tests now live in
`darkmatter/lib/tests/fixtures/lifecycle_control_flow_spike/`, with imports
updated to that durable location. These are preserved tests, not new tests.
This removes the dependency on a specification's lifecycle location.

An adjacent locator comment incorrectly described block scalars as supported
only by tolerant frontmatter lookup. The comment now agrees with the current
strict locator; runtime behavior is unchanged.

## Verification

From `darkmatter/`, using nextest through the canonical recipe:

```sh
just test schema_yaml_spellings
BISCUIT_TEST_FILTER='test(schema_validate) | test(lifecycle_control_flow_spike) | test(schemas_source_projection) | test(shipped_schema)' just test
```

The first run passed **34 tests** (maximum **0.088 seconds**); the second
passed **184 tests** (maximum **2.518 seconds**). Each run used parallel L1
execution. Logs are `/tmp/pipeline-s01-tests.log` and
`/tmp/pipeline-s01-cli-tests.log`. `git diff --check` passed.

An initial filter containing a shell pipe failed before test execution; it
was replaced by the supported `BISCUIT_TEST_FILTER` environment override.
The new test initially selected the `ma` suffix of `schema` in its own setup;
the final version locates the intended `; ma` token explicitly and passed.
Only these scoped tests were executed; no cross-OS or full-area verification
is claimed by this slice.
