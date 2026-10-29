---
kind: implementation-log
---

# Test audit: 2026-10-28-schema-enhancements

Added **2 genuine test functions/Nextest identities**, with **0 existing tests
strengthened** and **0 slow tests modified**. Gross and net additions are both
two; no existing identities were removed or split. The final scoped runs passed
**673 distinct L1 tests**, with a maximum duration of **3.950 seconds**.
No production behavior changed.

## Added coverage

`darkmatter/lib/tests/l1/coercion_engine_parity.rs` already compared frontmatter
composition, argument binding, and normal instrumented function dispatch for
fixed tuples with optional slots. Variadic tuples had direct rule-core and
translator checks, but lacked the same end-to-end parity through composition
and dispatch. Two bounded tests close that gap:

- `variadic_tuple_constraints_agree_through_composition_and_dispatch` checks
  eight cases: an empty rest section, exact and converted rest values,
  integer and minimum failures, malformed text, a missing required prefix,
  and rejection of a scalar in place of a tuple.
- `nested_tuple_spreads_keep_optional_slots_distinct_from_null` checks five
  cases with a required object prefix and tuple-valued rest members. It asserts
  exact nested conversions, omitted optional slots, rejected explicit null,
  missing required tuple positions, and missing required object properties.

Each case has an independent expected keep/convert/reject result asserted
through all three entry points, rather than merely requiring two adapters to
agree. Existing dispatch instrumentation verifies one handler entry for an
accepted call and zero for a rejected call. The new cases passed without a
production change; they are coverage additions, not claimed bug repairs.
The initial targeted run took 0.035 and 0.061 seconds for the two new tests.

## Requirement coverage

The completed specification's existing criteria map names tests for criteria
1–59, including 33a and 33b. This audit inspected that map against the current
test modules and exercised the implementation at these boundaries:

| Criteria | Coverage checked |
| --- | --- |
| 1–6, 47, 56 | Tuple/function/category/union parsing, precedence, exact grammar errors, serialization, JSON Schema lowering, function exclusion from document schemas, empty/optional/spread tuple validation. |
| 7–12, 39, 57 | Catalog/registry parity, canonical names and aliases, duplicate overloads, structural executable examples and display-only examples, every registered function's binding row, handler-entry rejection counters, wrong-return implementation errors, Claudine catalog grouping and output snapshots. |
| 14–27 | Shared conversion rules, structured errors, constraints, any/preservation types, union permutations and ambiguity, exact numeric admission, all-argument checking, overload selection, nested containers, native content serialization. |
| 28–35, 55 | Real frontmatter composition through the shared engine, root/property/list unions, nested objects and dictionaries, preservation formats, translator conformance, raw references and sibling assertions, overlap, additional keys, unmodeled assertions, recursive references, order independence, and nonmutation. |
| 36–38, 40–42, 48–54 | Five compatibility categories, passive prechecks before effects, real CLI rejection, protocol-level editor ranges/settings/reload/composition agreement, declared-variable projection, namespace reads, numeric recovery, content types, dead branches, lazy operands, and requested composition surfaces. |
| 43–46, 58–59 | Null contracts, charter function behavior, insertion-order maps, stable serialized CLI output, numeric arithmetic/rendering/admission at all ingress points, expression literals, and constraint/default/enum metadata. |

The final Darkmatter selection also includes the canonical rule-core suite,
catalog registration tests, alias inventory, numeric unit tests, provider
request counter, real `md` schema commands, and legacy JSON-output parity.
Claudine's selection exercises its real catalog-output command and authored
overlay/stable-order consumers. The existing Sniff stable-order test was
inspected as part of the historical reader inventory; this slice did not rerun
Sniff because neither its code nor shared serialization configuration changed.

There is no terminal input, visual layout, browser, live provider, or audio
requirement introduced by this specification. CLI output is a text/structure
contract; editor behavior crosses the in-memory LSP protocol harness. **L2 is
not applicable**, and no terminal or browser windows were opened. All runs
above are parallel L1; no other tier ran concurrently.

## Verification

From `darkmatter/`, using the canonical Nextest-backed recipe:

```sh
just test coercion_engine_parity
BISCUIT_TEST_FILTER='test(schema_grammar_descriptors::) | test(schema_enhancements_matrix::) | test(function_binding_registry::) | test(function_dispatch::) | test(function_binding_conversions::) | test(function_call_binding::) | test(frontmatter_coercion_engine::) | test(raw_schema_conversion::) | test(coercion_engine_parity::) | test(coercion_translator_conformance::) | test(coercion_core::) | test(numeric_admission::) | test(design_time_call_check::) | test(compose_call_precheck::) | test(pure_function_contracts::) | test(context_function_contracts::) | test(expression_call_diagnostics::) | test(call_check_agreement::) | test(compose_schema_enhancements::) | test(variable_type_projection::) | test(catalog::tests::) | test(function_catalog_parity::) | test(registration_tests::) | test(markdown::numeric::tests::) | test(provider_network::list_counts_bind_before_any_provider_request) | test(schema_grammar_types::) | test(schema_detect_json_emits_json_schema) | test(schema_validate_legacy_json_output_is_byte_identical)' just test
```

The targeted parity run passed 21 tests, maximum 0.065 seconds. The final
expanded run passed **661 tests**, maximum **3.950 seconds**. An earlier subset
passed 584 tests, maximum 3.713 seconds; these repeated observations are not
added to the distinct-test total. Logs are `/tmp/pipeline-s18-new.log`,
`/tmp/pipeline-s18-final-dm.log`, and `/tmp/pipeline-s18-initial.log`.

From `claudine/`:

```sh
BISCUIT_TEST_FILTER='test(context_expressions) | test(expression_groups_follow_category_list_order_and_sort_alphabetically) | test(function_examples_render_both_forms_one_per_line) | test(action_shape_control::proxy_with_iteration) | test(opencode_config::tests::yolo_permission_block_serializes_independent_of_insertion_order)' just test
```

This passed **12 tests**, maximum **0.384 seconds**. Log:
`/tmp/pipeline-s18-claudine.log`.

From the repository root, the exact changed test target passed lint:

```sh
cargo clippy -p darkmatter --test l1 --features effects-instrumentation -- -D warnings
git diff --check
```

Clippy finished successfully in 12.79 seconds after all test runs had finished;
its compile time is not test duration. Log: `/tmp/pipeline-s18-lint.log`.

## Limits

Criterion 13 includes author review of the behavior-change ledger. The existing
criteria map records the final ledger additions as human-pending. Automated
tests cannot establish that approval, and this audit does not infer it from
the specification's completed location.

Local execution evidence is from this macOS host. The new tests use existing
isolated temporary-file and in-memory dispatch helpers, no OS-specific syntax
or gates, and no ambient repository, provider, or user configuration. No
cross-OS execution or full-area gate is claimed; the coordinator owns final
broader verification.
