---
kind: implementation-log
---

# Test audit: 2026-10-17-content-policy-base-schema

Added **one genuine coverage test function/Nextest identity** (gross and net
increase: one); strengthened **zero existing tests** and modified **zero slow
tests**. No production behavior changed. All **157 distinct selected L1 tests
passed**; the maximum was **2.981 seconds**. The new test took **0.263 seconds**.

## Changes

`content_policy_embedded_baseline::explicit_library_configuration_does_not_adopt_the_builtin_policy_rules`
directly tests the library configuration boundary. The same invalid policy is
accepted without a configured baseline, rejected with the built-in baseline,
and accepted under an explicitly supplied custom baseline. A missing required
property then fails under that custom baseline, proving that the custom
validator was applied rather than silently bypassed. Existing generic
no-schema tests did not distinguish these policy-specific configurations.

The Content Policy schema's header incorrectly assigned impossible-calendar
date and embedded-template path rejection to evaluation. Its declaration
parser already rejects those inputs without evaluating freshness. The comments
now identify that parser as the syntax authority; schema declarations and
runtime behavior are unchanged.

## Requirement coverage

| Requirement | Evidence and boundary |
| --- | --- |
| Default typing, optional omission, accepted/rejected policy shapes | Public library baseline tests, built `md schema validate` and compose, and in-memory DMLS protocol tests; exact paths, kinds, and authored node coordinates are checked. |
| Parser agreement and documented differences | Shared `DECLARATIONS` corpus runs through Content Policy's real parser, disk and embedded schemas, document imports, built CLI, and DMLS. Both expected outcomes are explicit for dates, overflow, path limits, combined Evergreen, and whole-list null. |
| Embedded resolution, source identity, and error paths | Private `embedded_sources` tests use test-owned tables, including bare names, relative imports, missing types/documents, malformed references, invalid virtual paths, cycles, and disk/embedded identity isolation. Shared mutation cells compare embedded and disk loaders. |
| Actual shipped artifacts and cache reuse | Shipped catalog preparation/lowering/compilation, disk/embedded typed and JSON parity, Content Policy export equivalence, and same-process baseline pointer identity checks. |
| Installed operation and editor features | The built `md` and stdio DMLS run outside the checkout with isolated home/configuration and no schema files. DMLS verifies exact compact/rule/action suggestions and rejection locations; in-memory protocol tests also cover hover and long-form keys. |
| Baseline selection and compatibility | Generated CLI decision/spelling rows plus roster guards exercise flags, captured environment, all truthy spellings, broken custom baselines, conflicts, retained document/trigger validation, JSON/pretty/quiet behavior, and exit codes. The added direct API test protects deliberate library configuration. |
| Passive validation and post-resolution checks | Source bytes and policy values remain unchanged, omitted policies stay absent, explicit document declarations retain optional-null materialization, nonexistent watched files need no evidence, instrumented effect/network counters stay unchanged, and shell/template results are judged after resolution. |

The schema transformation/parser assertions are F1 checks of contracts owned
by these packages. Hermetic real executable and stdio protocol tests cross
the compiled entry-point boundary; their assertions inspect results rather
than merely successful launches. There is no terminal input, rendered pane,
browser layout, live provider, or hardware requirement. **L2 is not applicable**
and no application windows were opened.

## Verification

From `darkmatter/`, using the canonical parallel Nextest-backed recipe:

```sh
BISCUIT_TEST_FILTER='test(content_policy) | test(schema_validate_baseline_selection) | test(standalone_baseline) | test(base_schema_end_to_end) | test(embedded_sources) | test(standalone_stdio)' just test
```

Result: **154 passed**, zero failures or over-five-second tests; maximum
**2.981 seconds**, parallel execution total **6.234 seconds**. Log:
`/tmp/pipeline-s07-final.log`. The initial narrower audit run passed 152 tests,
maximum 3.009 seconds. A compile-time assertion mismatch while adding the new
test was corrected to compare the optional missing-property name explicitly.

From `content-policy/`, after the Darkmatter run finished:

```sh
BISCUIT_TEST_FILTER='binary(editor_schema)' just test
```

Result: **three passed** in the exported-schema binary; the CLI had no matching
tests. Log: `/tmp/pipeline-s07-policy.log`. An initial positional
`just test editor_schema` matched no function names; it is not counted as
verification and was replaced by the explicit binary filter.

`git diff --check` passed. No full-area, cross-OS, or lint run is claimed for
this slice; the coordinator owns final package gates.
