# Acceptance Criteria Map

This map links each acceptance criterion in `spec.md` (1–59, plus 33a and 33b)
to three things: the `plan.md` task that delivers it, a **proposed** test
identity, and the public result that test observes. Every test identity is a
proposal for later waves. None of these tests exists yet, unless a row says it
extends an existing file. Paths follow the consolidated layout. Each package
has one `l1` test binary whose root is `tests/l1/main.rs`, with
`autotests = false`. A new module must be declared in that `main.rs`, because
`test_layout.rs` rejects an undeclared file. Run a test with nextest as
`l1 <module>::<test>` in its package. The prefixes used below:

- `dm-lib` = `darkmatter/lib/tests/l1/`
- `dmls` = `darkmatter/dmls/tests/l1/`
- `md-cli` = `darkmatter/cli/tests/l1/`
- `cl-cli` = `claudine/cli/tests/l1/`

Grammar-internal unit tests may also live beside the code, in `#[cfg(test)]`
modules under `darkmatter/lib/src/markdown/schemas/simplified/`. The
criterion's public evidence is still the `l1` row named here.

Plan task abbreviations (phase / wave / task title from `plan.md`):

| Code | Task |
|---|---|
| P1W1-INV | Phase 1 / Wave 1 / Contract inventory |
| P1W1-NUM | Phase 1 / Wave 1 / Numeric ingress (spike) |
| P1-MTX | Phase 1 / Matrix fixtures |
| P2W2-GRM | Phase 2 / Wave 2 / Grammar descriptors |
| P2W2-NUM | Phase 2 / Wave 2 / Numeric foundations |
| P3W3-CORE | Phase 3 / Wave 3 / Conversion core |
| P3W4-ADP | Phase 3 / Wave 4 / Schema adapter |
| P3W4-BND | Phase 3 / Wave 4 / Call binder |
| P4W5-CAT | Phase 4 / Wave 5 / Catalog replacement |
| P4W5-PURE | Phase 4 / Wave 5 / Pure handlers |
| P4W5-CTX | Phase 4 / Wave 5 / Context handlers |
| P4W6-REG | Phase 4 / Wave 6 / Registry integration |
| P4W6-CON | Phase 4 / Wave 6 / Consumer migration |
| P5W7-PRJ | Phase 5 / Wave 7 / Variable projection |
| P5W7-API | Phase 5 / Wave 7 / Compatibility API |
| P5W8-CMP | Phase 5 / Wave 8 / Composition precheck |
| P5W8-ED | Phase 5 / Wave 8 / Editor diagnostics |
| P6W9-DOC | Phase 6 / Wave 9 / Current documentation |
| P6W9-CDOC | Phase 6 / Wave 9 / Consumer documentation |
| P6W9-AUD | Phase 6 / Wave 9 / Coverage audit |
| P6W10-REV | Phase 6 / Wave 10 / Review package |

"Counter" means a test-registered handler-entry counter (P4W6-REG instruments
handler entry). "Dispatch" means normal expression evaluation through the
registry, never a direct handler call.

## Grammar and Catalog

| # | Short name | Delivering task | Proposed test identity | Public result observed |
|---|---|---|---|---|
| 1 | Grammar parses tuple/function/`\|`/`categories:` | P2W2-GRM | `dm-lib schemas_function_grammar.rs::{parses_every_spec_form, rejects_two_spreads, rejects_nonfinal_spread, rejects_optional_before_required, rejects_optional_spread, rejects_named_tuple_element, rejects_function_without_parameters_or_returns}` | Parsed `PropertyAtom`/descriptor through the public `parse_type_expr`/schema-load API; schema-load error text for each rejection |
| 2 | TypeScript precedence and grouping | P2W2-GRM | `dm-lib schemas_tuple_union_lowering.rs::union_and_array_precedence_follows_typescript` | Frontmatter validation result (accept `"a"`, `[1]`; reject `["a"]`; `(string \| number)[]` accepts `["a", 1]`) |
| 3 | Tuples and `\|` lower to JSON Schema | P2W2-GRM | `dm-lib schemas_tuple_union_lowering.rs::{tuple_snapshot, variadic_tuple_snapshot, compact_union_snapshot, compact_and_list_union_lower_identically}` (insta snapshots in `dm-lib snapshots/`) | Compiled JSON Schema (`prefixItems`/`minItems`/`items`, `anyOf`) |
| 4 | Function type under `$schema` fails at load | P2W2-GRM | `dm-lib schemas_function_load.rs::function_property_in_document_schema_fails_at_load`; end-to-end `md-cli compose_schema_enhancements.rs::function_typed_property_is_a_schema_load_error` | Schema-load error (library) and `md compose` exit status plus stderr |
| 5 | Categories load and emit | P2W2-GRM | `dm-lib schemas_function_load.rs::{undeclared_category_fails_with_categories_list, category_unrestricted_without_list, data_category_emits_annotation, category_descriptions_optional_and_multiline}` | Catalog/schema load error or success; `x-darkmatter-category` in compiled JSON Schema |
| 6 | Duplicate overloads fail at load | P2W2-GRM | `dm-lib schemas_function_load.rs::duplicate_pr_list_overloads_fail_catalog_load` | Catalog-load error |
| 7 | New catalog complete and sole | P4W5-CAT, P4W6-CON | `dm-lib function_catalog_parity.rs::{catalog_and_registry_match_both_ways, old_catalog_is_gone_and_unread}`; `cl-cli context_command.rs::context_expressions_reads_darkmatter_catalog_api` (extends existing file); `dmls expression_call_diagnostics.rs::completion_reads_darkmatter_catalog_api` | Name-set equality between the catalog API and registry enumeration (including `keys`/`values`); absence of the old file and of any reader (source scan, as in `md-cli common/source_scan.rs`); consumer output built from the library API |
| 8 | Executable examples pass structurally | P4W5-CAT | `dm-lib function_catalog_examples.rs::{every_executable_example_passes, executable_count_is_at_least_72, number_three_does_not_match_text_three, object_results_compare_structurally, error_expectation_requires_failure}` | Dispatch result versus example expectation; executable count |
| 9 | Display-only shown, never run | P4W5-CAT, P4W6-CON, P6W9-CDOC | `dm-lib function_catalog_examples.rs::display_only_examples_are_skipped_by_design`; `cl-cli context_command.rs::context_expressions_shows_display_only_and_file_examples` | Runner report (skipped-by-design with reason, counter 0); rendered `claudine context --expressions` output |

## One-pass Migration

| # | Short name | Delivering task | Proposed test identity | Public result observed |
|---|---|---|---|---|
| 10 | Every function binds through the engine | P4W6-REG (fixture rows from P4W5-PURE, P4W5-CTX) | `dm-lib function_binding_registry.rs::every_registered_function_has_accepted_rejected_converted_fixtures` (registry-wide table, one row per canonical) | Dispatch result per fixture; counter = 0 on every rejected row; failure for any canonical without a row. **Blocked on the Phase 1 fixture-applicability ruling (plan rule 11)** for void and preservation-only functions |
| 11 | No handler coerces | P4W6-REG | `dm-lib function_binding_registry.rs::no_handler_local_coercion_helpers_remain` (source scan for the deleted arity/type/conversion helpers) | Source-scan result. **Partly non-automated:** the "review confirms no handler inspects an argument's type to convert it" clause is a human/agent code review recorded in the implementation log |
| 12 | `number()`/`round()` charter contract | P3W4-BND (recovery), P4W5-PURE, P4W5-CAT | `dm-lib function_number_round.rs::{number_contract_table, round_contract_table, catalog_names_fallback_and_has_examples}` | Dispatch results/type errors/arity error; catalog descriptor parameter name `fallback`; example runner coverage |
| 13 | Behavior-change ledger complete and reviewed | P1W1-INV (draft), P4W6-REG, P6W9-AUD, P6W10-REV | `dm-lib function_binding_registry.rs::ledger_lists_every_registered_function` (optional completeness check against the registry) | Ledger completeness. **Non-automated:** the human pre-merge ledger review cannot be automated and is recorded as pending, never inferred |

## Coercion Engine: Function Calls

All rows below go through dispatch. They use test-registered functions where
the spec says so.

| # | Short name | Delivering task | Proposed test identity | Public result observed |
|---|---|---|---|---|
| 14 | Text → number | P3W3-CORE, P3W4-BND, P4W6-REG | `dm-lib function_binding_conversions.rs::min_converts_text_to_number` | Result `4`; counter = 1 |
| 15 | Number → text | same | `dm-lib function_binding_conversions.rs::upper_converts_number_to_text` | Result `"2026"` |
| 16 | Complete type errors; no entry | same | `dm-lib function_binding_conversions.rs::min_pear_type_error_names_function_parameter_type_value_reason` | Structured type error fields (`min`, `a`, `number`, text `"pear"`, reason); counter = 0 |
| 17 | Constraints enforced, not repaired | P3W3-CORE, P3W4-BND | `dm-lib function_binding_conversions.rs::{three_letter_acronym_constraints, integer_constraint_rejects_two_point_five}` | Handler receives `"123"`; type errors for `1234`/`"ABCD"`/`"2.5"`; counters 0 on failure |
| 18 | `any` preserves original | P4W5-PURE, P4W6-REG | `dm-lib function_binding_conversions.rs::is_number_sees_original_value` | `false` for `"4"`, `true` for `4` |
| 19 | Rejected conversions rejected | P3W3-CORE, P3W4-BND | `dm-lib function_binding_conversions.rs::boolean_null_and_text_list_are_rejected` | Type errors; counters 0 |
| 20 | Union rule | P3W3-CORE, P3W4-BND | `dm-lib function_binding_conversions.rs::union_rule_table_both_orders` | Value received by each test-registered handler; ambiguous-union error naming both readings with counter 0; option order permuted |
| 21 | Exactness check | P2W2-NUM, P3W3-CORE | `dm-lib function_binding_conversions.rs::{echo_exactness_table, literal_beyond_safe_integer_is_rejected}` | Returned value or type error with reason "cannot be represented exactly"/"not finite"; counter 0; expression-literal rejection |
| 22 | Every argument checked | P3W4-BND | `dm-lib function_binding_conversions.rs::one_error_reports_both_parameters` | Single type error listing `a` and `b`; counter 0 |
| 23 | Overload selection, both orders | P3W4-BND, P4W5-CTX | `dm-lib function_binding_conversions.rs::pr_list_overloads_both_orders` | Selected overload/argument through a fake provider adapter; type error listing both signatures; fake provider request count = 0 |
| 24 | Containers follow structure | P3W3-CORE, P3W4-BND | `dm-lib function_binding_conversions.rs::containers_table_person_and_sum` | Every Containers-table outcome including paths `p[1]`, `values[2]`; caller list unchanged |
| 25 | Native values serialize into `json` | P3W3-CORE | `dm-lib function_binding_conversions.rs::json_parameter_serializes_native_and_keeps_text` | Handler receives `{"a":1}` text; unchanged text; type error for `"{a"` |
| 26 | `numberlike` delivered as sent | P3W3-CORE | `dm-lib function_binding_conversions.rs::numberlike_parameter_delivers_original` | Received values identical to input; type errors with counter 0 |
| 27 | `boolish` delivered as sent | P3W3-CORE | `dm-lib function_binding_conversions.rs::boolish_parameter_delivers_original` | As 26 |

## Coercion Engine: Frontmatter

All rows run through Darkmatter's schema validation and compose paths on
fixture documents. Each also gets an `md compose` end-to-end case in
`md-cli compose_schema_enhancements.rs`.

| # | Short name | Delivering task | Proposed test identity | Public result observed |
|---|---|---|---|---|
| 28 | Number → text | P3W4-ADP | `dm-lib frontmatter_coercion_engine.rs::number_version_composes_as_text` | Validation passes; composed frontmatter `"1.2"` |
| 29 | Boolean words | P3W4-ADP | `dm-lib frontmatter_coercion_engine.rs::yes_and_on_become_true` | Composed `true` |
| 30 | Lenient numbers and precision | P2W2-NUM, P3W4-ADP | `dm-lib frontmatter_coercion_engine.rs::{plus_and_underscore_numbers_convert, unsafe_integers_fail_validation}` | Composed `4`/`1000`; validation errors for the text and the native YAML integer |
| 31 | Root unions order-independent | P3W4-ADP | `dm-lib frontmatter_coercion_engine.rs::root_union_result_is_order_independent` | Identical composed output with arms in either order |
| 32 | Nested objects converted | P3W4-ADP | `dm-lib frontmatter_coercion_engine.rs::nested_object_count_converts` | Composed `count: 3` |
| 33 | Native → `json`/`yaml` | P3W4-ADP, P2W2-NUM (`preserve_order`) | `dm-lib frontmatter_coercion_engine.rs::{json_property_serializes_mapping, json_or_yaml_keeps_written_key_order, null_is_not_serialized}` | Composed JSON text in authored key order; null kept as null |
| 33a | Completed Changes table | P3W4-ADP | `dm-lib frontmatter_coercion_engine.rs::{enum_number_composes_as_text, pattern_key_dictionary_converts, nested_union_numberlike_and_patterns}`; `dm-lib raw_schema_conversion.rs::ref_defs_type_array_oneof_convert_referenced_number` | Composed values (`"2"`, `3`, `numberlike` kept as written); raw-schema conversion result |
| 33b | Custom formats agree with engine | P3W4-ADP, P2W2-NUM | `dm-lib frontmatter_coercion_engine.rs::numberlike_boolish_formats_agree_with_engine_on_probe_list`; `md-cli schema_detect.rs` snapshot update (existing file) | Compiled schema `format` values; validator versus engine acceptance per probe; `md schema detect --format json` snapshot |
| 34 | `numberlike`/`boolish` validated, not normalized | P3W4-ADP | `dm-lib frontmatter_coercion_engine.rs::preservation_types_compose_unchanged` | Composed `"02134"`/`"On"` unchanged; validation error for `"pear"` |
| 35 | One engine | P3W3-CORE, P3W4-ADP, P3W4-BND | `dm-lib coercion_engine_parity.rs::frontmatter_and_call_outcomes_match_for_shared_pairs`; `dm-lib coercion_engine_parity.rs::old_frontmatter_rule_code_is_gone` (source scan) | Identical keep/convert/error outcomes through compose and dispatch over the shared value/type table; absence of the old rule code in `markdown/schemas/coerce.rs` |

## Design-time Checking

| # | Short name | Delivering task | Proposed test identity | Public result observed |
|---|---|---|---|---|
| 36 | Compatibility API five categories | P5W7-API | `dm-lib expression_compatibility_api.rs::category_table_rows_a_to_e_and_listed_pairs` | Category returned by the public compatibility API per pair |
| 37 | Category 5 rejected at composition, every function | P5W8-CMP | `dm-lib compose_call_precheck.rs::{min_flag_rejected_before_evaluation, generated_unreachable_calls_rejected_for_every_function}`; `md-cli compose_schema_enhancements.rs::category_five_call_fails_compose` | Compose error before evaluation; counter 0; no expression evaluated (observer); `md compose` exit status |
| 38 | Category 1 conversions compose | P5W8-CMP | `dm-lib compose_call_precheck.rs::three_letter_acronym_lookup_composes_and_fails_at_runtime_on_1234` | Successful compose output; runtime type error for `1234` |
| 39 | Documentation grouping | P4W6-CON, P6W9-CDOC | `cl-cli context_command.rs::context_expressions_groups_by_category_order_snapshot` (extends existing file; insta snapshot) | `claudine context --expressions` stdout snapshot |
| 40 | DMLS categories under both settings | P5W8-ED | `dmls expression_call_diagnostics.rs::{rows_a_to_e_strict_off, rows_a_to_e_strict_on, strict_from_dmls_toml, strict_from_workspace_configuration}` | Published `textDocument/publishDiagnostics` (severity per row) through an LSP session |
| 41 | Category 5 underlined; agrees with compose | P5W8-ED, P5W8-CMP | `dmls expression_call_diagnostics.rs::row_d_error_ranged_at_argument_and_compose_rejects` | Published diagnostic severity and range; composing the same fixture fails. **Partly non-automated:** "review confirms DMLS obtains every category from the API and has no rules of its own" is a code review (a source-scan guard, as in the existing `type_scope_ownership_guard.rs`, can support it) |
| 42 | `file_exists` binds like any function | P4W5-CTX, P5W7-API, P5W8-ED | `dm-lib function_binding_context.rs::file_exists_inspects_without_conversion`; `dm-lib expression_compatibility_api.rs::number_to_any_is_category_one`; `dmls expression_call_diagnostics.rs::file_exists_number_has_no_diagnostic` | `false`/`false`/`true` results with a private filesystem fixture named `42`; category 1; no published diagnostic. **Partly non-automated:** "no `file_exists`-specific DMLS setting" is a review (a config-model source scan can support it) |
| 43 | Ledger null rule | P4W5-PURE, P4W5-CTX, P4W6-REG | `dm-lib expression_null_contracts.rs::null_contract_examples` | Dispatch results (`null`, `false`, type error with counter 0); ledger null column complete (audited in P6W9-AUD) |
| 44 | Charter-only contracts | P4W5-PURE, P4W5-CTX, P5W7-API | `dm-lib expression_charter_contracts.rs::{length_contract, validate_schema_m9_examples, contains_contract, has_command_true_is_a_command_name}`; `dm-lib expression_compatibility_api.rs::length_boolean_is_category_five` | Dispatch results/type errors; category 5; file and inputs unchanged after repeated `validate_schema` calls; fake command lookup |
| 45 | Map functions | P4W5-PURE, P4W5-CAT | `dm-lib expression_charter_contracts.rs::keys_and_values_preserve_insertion_order` | Dispatch results `["b","a"]`, `[1,2]`; executable catalog examples present |
| 46 | Numeric operators and presentation | P2W2-NUM | `dm-lib expression_numeric_operators.rs::{overflow_and_division_by_zero_error, mixed_equality_and_ordering, no_saturating_cast, whole_numbers_render_without_suffix}` | Evaluation errors/results; composed body `4` and `2.5` |
| 47 | `(A \| B)[]` is list-of-union | P2W2-GRM, P3W4-ADP | `dm-lib schemas_tuple_union_lowering.rs::{inline_list_union_equals_named_union_list, list_union_accepts_and_rejects}`; `dm-lib frontmatter_coercion_engine.rs::list_union_converts_element_by_element` | Identical compiled JSON Schema (`items: { anyOf }`); validation results; composed `[2, true]` |
| 48 | Variable-type projection | P5W7-PRJ | `dm-lib expression_variable_projection.rs::{worked_example_projection, raw_and_mixed_root_project_unknown, projection_is_passive_and_nonmutating}` | Projected types from the public projection API; failing access stubs never called; frontmatter unchanged |
| 49 | Dead branches are checked | P5W7-API, P5W8-CMP, P5W8-ED | `dm-lib compose_call_precheck.rs::dead_ternary_fallback_and_block_calls_are_rejected`; `dmls expression_call_diagnostics.rs::dead_branch_calls_report_flag_error` | Compose error with counter 0 and no evaluation; published error on `flag` |

## Review-added Contract Checks

| # | Short name | Delivering task | Proposed test identity | Public result observed |
|---|---|---|---|---|
| 50 | Projection agrees with presence | P5W7-PRJ, P5W7-API | `dm-lib expression_variable_projection.rs::{required_union_has_no_null_arm, nullable_union_stays_optional, inherited_and_unknown_required_match_lowering, fm_static_access_projects_like_bare}`; `dm-lib compose_call_precheck.rs::fm_dot_and_bracket_reject_like_bare`; `dm-lib expression_compatibility_api.rs::nullable_to_null_admitting_parameter_is_category_one` | Projected types; compose rejection parity for `min(fm.flag, 5)`/`min(fm["flag"], 5)`; reserved roots unchanged; category 1 |
| 51 | Recovery agrees, compose and editor | P3W4-BND, P5W7-API, P5W8-CMP, P5W8-ED | `dm-lib compose_call_precheck.rs::number_and_round_recovery_table`; `dmls expression_call_diagnostics.rs::number_and_round_recovery_table` | Compose success or error per call; published diagnostics (none on accepted rows, an error on rejected rows), including the typed-boolean first argument variant |
| 52 | Content-format classification | P5W7-API | `dm-lib expression_compatibility_api.rs::content_format_classification` | Categories 3/1/5/1/5 per pair |
| 53 | Lazy evaluation preserved | P3W4-BND, P4W6-REG, P5W7-API | `dm-lib compose_call_precheck.rs::and_or_skip_effect_but_precheck_rejects_bad_skipped_call` | Effect counter 0 on skip; compose rejection for a proven bad call in the skipped argument |
| 54 | Checking precedes effects; surface respected | P5W8-CMP | `dm-lib compose_call_precheck.rs::{frontmatter_bad_call_rejects_before_sibling_effect, fences_literals_and_inserted_data_not_checked, frontmatter_only_leaves_invalid_body_untouched}` | Sibling effect counter 0; compose success when the bad call sits only in a non-executable surface; frontmatter-only versus full-compose outcomes |
| 55 | Raw schema assertions survive | P3W4-ADP, P1-MTX | `dm-lib raw_schema_conversion.rs::{oneof_overlap, ref_siblings, referenced_number_with_sibling_constraint, overlapping_literal_and_pattern_keys, typed_additional_keys, unmodeled_multiple_of, cyclic_local_reference, failure_leaves_input_unchanged, alternative_and_pattern_order_is_irrelevant}` | Validation/composition outcome per fixture; input unchanged on failure; permutation invariance |
| 56 | Grammar closes indirect invalid forms | P2W2-GRM | `dm-lib schemas_function_grammar.rs::{rejects_scalar_spread, rejects_duplicate_parameter_names, rejects_bad_rest_parameters, rejects_void_with_parameters, empty_tuple_accepts_only_empty_list}`; `dm-lib schemas_function_load.rs::document_schema_reaching_function_through_every_container_and_reference_fails`; `dm-lib frontmatter_coercion_engine.rs::optional_numeric_tuple_slot_rejects_null` | Schema-load errors; validation results |
| 57 | Catalog identity and return contracts | P4W5-CAT, P3W4-BND, P4W6-REG | `dm-lib function_catalog_parity.rs::{aliases_resolve_as_before, documentation_lists_canonicals_once, semantic_duplicate_overloads_fail}`; `dm-lib function_binding_registry.rs::wrong_return_reports_implementation_contract_error` | Alias dispatch results; catalog listing; catalog-load error; implementation-contract error with the result unconverted |
| 58 | Map ordering preserves contracts | P2W2-NUM, P4W5-PURE | `dm-lib map_insertion_order.rs::{nested_and_function_built_maps_keep_insertion_order, stable_order_consumers_stay_sorted}` | Serialized output order; sorted consumer output. **Partly non-automated:** "review changed serialized-output expectations for intentional behavior" is a review recorded in the implementation log (snapshot diffs are inspected, not bulk-accepted) |
| 59 | Whole-number admission in every spelling | P1W1-NUM, P2W2-NUM, P3W4-ADP | `dm-lib frontmatter_coercion_engine.rs::whole_number_admission_every_spelling`; `dm-lib expression_numeric_operators.rs::{numeric_metadata_uses_same_admission, external_finite_f64_accepted_nonfinite_rejected}` | Validation errors with reason "cannot be represented exactly"; accepted boundary values; schema-load errors for numeric defaults, enum members, and constraints; existing `0.1`, overflow, and underflow outcomes |

## Cross-cutting Evidence the Spec Requires

- **Passive shipped-artifact corpus test** for grammar, schema, and catalog
  changes: extend `dm-lib shipped_schema_corpus.rs`, which already enumerates
  shipped artifacts and names `expression-functions.yaml`. Update it to the
  new `darkmatter/schemas/partials/functions.yaml` (P4W5-CAT, P6W9-AUD).
- **End-to-end through `md compose`:** proposed `md-cli
  compose_schema_enhancements.rs`, using the existing `CliProcessFixture`
  pattern in `md-cli md_process_fixture.rs` and `compose_schema.rs`.
- **Input-robustness matrices** (plan, Phase 1): proposed `dm-lib
  schema_enhancement_readers_matrix.rs` (YAML catalog/envelope, raw JSON
  Schema keywords, frontmatter/numeric) and `dmls
  expressions_strict_config_matrix.rs` (TOML and editor JSON), following the
  existing `md-cli schema_robustness_matrix.rs` and `dmls
  workspace_formatting_settings.rs` styles (P1-MTX, P5W8-ED).

## Summary

- 61 criteria mapped (1–59, 33a, 33b). Every criterion has a delivering task
  and a proposed public-result test.
- **Non-automated parts:** 13 (human ledger review, by design), and review
  clauses in 11, 41, 42, and 58. Each of those also has an automated part.
- **Blocked mapping:** criterion 10 needs the Phase 1 author ruling on
  converted-success cells for void and preservation-only functions
  (plan rule 11) before it can be declared complete.
