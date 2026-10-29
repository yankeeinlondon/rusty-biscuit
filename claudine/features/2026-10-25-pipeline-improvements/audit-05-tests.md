---
kind: implementation-log
---

# Test Audit: 2026-10-15-diagnostic-node-positions

Added **2 test functions / Nextest identities** (gross and net +2), strengthened
**1 existing CLI test**, and modified **0 slow tests**. No scheduling-only
identities were added or replaced, and no production behavior changed.

All **56 selected L1 tests passed** in parallel. The maximum individual
duration was **2.212 seconds**; the complete test execution took **2.317
seconds**, excluding compilation.

## Added and strengthened coverage

- `missing_fields_in_empty_flow_mappings_report_the_opening_brace` validates
  missing-required problems on an empty object and an empty object inside a
  flow sequence, with explicit LF and CRLF input. An empty mapping has no
  first key, so its opening brace must supply its start. Both problems retain
  their missing-field identity and name `id`; their positions are 2:4 and
  3:18. The shared public-validation helper also compares every non-position
  field and problem ordering with source-free validation.
- `an_anchor_without_an_alias_keeps_precise_nested_positions` pins the
  integrated distinction between an anchor declaration and an alias use.
  An anchored mapping with no alias retains the nested value's precise
  position (3:10), rather than falling back to its top-level key. The
  documentation agent used this result to correct a blanket anchor exclusion.
- `assigned_values_use_the_original_node_or_report_null` now crosses the real
  `md schema validate` assignment path for dotted nested assignments. Replacing
  `/m/a` retains its original value start; adding `/m/b` uses the original
  mapping start. The original replaced scalar and newly added top-level
  property assertions remain. The test asserts exactly four problems, exit
  status 1, precise nested coordinates, and null top-level coordinates.

## Requirement coverage

| Requirement | Evidence |
| --- | --- |
| Acceptance `sya` fixture: unchanged parent pointer and selected unknown property, 5:9 in JSON and human output | Public library, shared fixture, real CLI JSON/human, and DMLS parity tests. |
| Value/key/mapping distinctions, block-list dash exclusion, flow mappings/sequences | Locator unit tests and public library tests; real CLI shared fixtures. New empty-flow-map regression covers mappings with no key token. |
| Compose reports the same position | `diagnostic_positions::compose_error_block_shows_line_and_column` invokes real `md compose` through `CliProcessFixture`. |
| DMLS byte-offset parity, supplementary-plane prefix, UTF-8/UTF-16 and CRLF | Existing parity matrix independently converts editor positions to bytes and asserts full ranged text as well as starts. |
| Replaced/new paths, scalar descendants, root missing field, unsupported YAML fallback, empty map | Public mutation tests, strengthened real CLI dotted assignments, locator tests, and source-free validation assertions. |
| Escaped/empty/numeric keys, comments/blank lines, delimiter/BOM offsets, unchanged identity/order | Existing locator/public tests plus byte-identical JSON/pretty CLI baseline tests. |
| One projection per snapshot and passive lookup | Source audit: `positions_for` constructs one map before validation; `collect_problems` and root-union arms borrow it; `PositionMap::locate` only traverses the retained nodes and line index. Existing direct collection/root-union tests passed. |

The internal coordinate transformation is tested directly at the cheapest
boundary. CLI tests exercise actual argument parsing, validation, serialization,
exit status, and formatting. Editor tests exercise DMLS diagnostic projection
with negotiated encodings and an independently calculated byte-offset oracle.
No requirement depends on a real terminal, browser, device, live provider, or
terminal display-cell rendering, so **L2 is not applicable**. No windows were
opened. No new source parser, schema grammar, or persisted representation was
introduced by this audit.

## Verification

From `darkmatter/`, through the canonical Nextest-backed recipe:

```sh
BISCUIT_TEST_FILTER='test(diagnostic_positions::) | test(markdown::schemas::positions::) | test(markdown::schemas::tests::positions::) | test(position_parity_tests::) | test(root_union_problems_are_located_in_the_shared_map) | test(collect_problems_populates_line) | test(schema_validate_baseline::)' just test
```

Final result: **56 passed**, zero failed, zero tests over five seconds.
Maximum **2.212 seconds** (the two eight-case CLI baseline matrices). The new
public regressions took less than 0.1 seconds each; the strengthened CLI test
remained below 0.5 seconds. Final log: `/tmp/pipeline-s05-tests-final.log`.
Initial narrower audit ran 49 existing tests, all passed, maximum 1.053 seconds.
`git diff --check` passed.

No full-area, lint, or cross-OS gate is claimed by this slice; the coordinator
owns the final package gates after all serial audits. Inputs use explicit
line endings, in-memory strings, and the existing isolated CLI fixture. No
platform-specific code or tier configuration changed.
