---
kind: implementation-log
---

# Test Audit: 2026-10-12-shared-type-tables

## Changes and counts

- Added **2 test functions** in `darkmatter/lib/src/markdown/schemas/resolve.rs`.
- Strengthened **1 existing test function**.
- Modified **0 slow tests**; every selected test finished below 5 seconds.
- No production behavior changed. Corrected stale resolver comments about
  acyclic-only expansion and cache lifetime to describe the integrated
  implementation.

`array_modifiers_do_not_change_another_use_of_the_shared_type` checks exact
array schema, scalar schema, description, item-count constraint, and required
properties across both standalone mapping envelopes and file/root-local
references. Its four parameter cases count as one added test. A modifier
leaking through a shared definition would change the second scalar or the
original declaration and fail the test.

`a_whole_file_reference_cannot_supply_a_cached_named_namespace` checks the
typed error, file path, exact rejection message, and absence of a cache entry
when a named namespace points at a whole-file-reference document. The target
of that whole-file reference contains a valid named mapping, so incorrectly
following the reference would make this test fail.

`first_load_cache_and_later_imports_share_one_table` now clears the observed
dependency set before each cache hit and asserts that the dependency is
recorded again. Its old final deduplication assertion could pass if dependency
recording moved below the cache-hit return, since the first load had already
inserted the edge. Allocation-identity assertions remain unchanged.

## Requirement coverage

| Requirement | Evidence |
| --- | --- |
| First load shares the cache's allocation; later names and spellings reuse it | Private `shared_type_tables` identity test calls the actual namespace resolver and checks `Arc::ptr_eq`, including after expansion. |
| Root and loaded-file `@this` preserve allocation and namespace identity | Two private identity tests check repeated lookups and distinguish root from file allocations. |
| Use-site constraints, descriptions, and arrays remain independent | Existing cross-file, root-local, and imported-file-local exact-output tests, plus the new array/scalar regression. Both mapping envelopes are exercised. |
| Dependency recording occurs on cache hits | Strengthened identity test clears and reobserves each cache-hit edge. Existing public dependency/deduplication tests also pass. |
| Errors, namespace classification, cycles, depth limits, and relative context remain intact | Resolver suite, standalone-envelope import suite, new whole-file-reference rejection test, and union-postfix cycle/depth regression. |
| Shipped schemas remain usable | Existing `meta_schema_repo_schemas` tests resolve and validate shipped examples. |
| No unconditional root-table allocation for schemas without imports | Source audit confirms the early return remains before root-table construction. |

Allocation identity and independent expansion are internal transformation
contracts, so direct resolver assertions are the cheapest sufficient boundary
(F1). Public resolution tests exercise the real filesystem and parser with
hermetic temporary files; they assert outputs and typed failures rather than
only success. This ownership-only change assumes no terminal, browser, live
provider, or device behavior. **L2 is not applicable**; no windows were opened.

Later pipeline work added process-wide parsed-file-revision and shipped-table
memos. The audit preserves that integrated behavior rather than enforcing the
original snapshot's now-superseded cross-call lifetime description. The
per-expansion engine still holds immutable shared tables, and per-use
modifiers still apply to independent copies.

## Verification

From `darkmatter/`, through the canonical nextest-backed recipe:

```sh
just test shared_type_tables
BISCUIT_TEST_FILTER='test(markdown::schemas::resolve::) | test(suggest_constraint_phase4::) | test(meta_schema_repo_schemas::) | test(schemas_union_postfix::cycles_and_depth_limits)' just test
```

The first run passed **9 tests**, maximum **0.037 seconds**, before the
whole-file-reference rejection test was added. The final run passed **115
tests**, including all **10** `shared_type_tables` tests, with **0 failures**.
The slowest test was **0.074 seconds**; the new tests took **0.022** and
**0.029 seconds**. Nextest's parallel L1 execution took **0.270 seconds**;
compilation took **24.98 seconds**, separately from test execution. The final
run log is `/tmp/pipeline-s02-tests.log`. `git diff --check` passed.

No full-area or cross-OS run is claimed by this slice. The coordinator owns
the final package gate after the serial audits finish.
