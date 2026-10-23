# Acceptance Criteria Map (final evidence)

Coverage audit, Phase 6 / Wave 9, 2026-10-23. This map resolves every
acceptance criterion in `spec.md` (1–59, plus 33a and 33b) to the tests that
prove it, and records the review items and the human item. It replaces the
Phase 1 proposal; the proposed test identities that were never created
(`function_catalog_examples.rs`, `expression_compatibility_api.rs`, and the
like) are gone, and each row names the test that actually exists.

## How to read the evidence column

Evidence is written `package::binary test-path`.

| Prefix | Meaning | Run with |
|---|---|---|
| `dm::l1` | `darkmatter` integration binary `l1` (`darkmatter/lib/tests/l1/`) | `cargo nextest run -p darkmatter --features effects-instrumentation --test l1` |
| `dm::lib` | `darkmatter` library unit tests (`#[cfg(test)]` under `darkmatter/lib/src/`) | `cargo nextest run -p darkmatter --features effects-instrumentation --lib` |
| `dmls::l1` / `dmls::lib` | `dmls` binary `l1` / library unit tests | `cargo nextest run -p dmls --features effects-instrumentation --test l1` / `--lib` |
| `md-cli::l1` | `darkmatter-cli` binary `l1` (spawns the real `md`) | `cargo nextest run -p darkmatter-cli --features test-fixtures --test l1` |
| `cl-cli::l1` / `cl-cli::bin` | `claudine-cli` binary `l1` / unit tests in the `claudine` binary | `cargo nextest run -p claudine-cli --test l1` / `--bins` |
| `cl::lib` / `cl::l1` | `claudine` library unit tests / binary `l1` | `cargo nextest run -p claudine --lib` / `--test l1` |
| `sniff-cli::l1` | `sniff-cli` binary `l1` | `cargo nextest run -p sniff-cli --test l1` |

`{a, b}` lists sibling tests in one module; `*` is every test in the module.
"Counter" is the handler-entry counter `function_test_hooks::handler_entries`.

Status values:

- **pass**: every cited test exists and passed in the audit run below.
- **pass + review**: the automated part passed, and the review clause was
  performed in this audit (result under [Review items](#review-items)).
- **human-pending**: needs the author; never inferred.

## Audit run

Every cited test was listed by name (`cargo nextest list`) and then run in
one filtered invocation per package, on 2026-10-23, on macOS:

| Package (targets) | Result |
|---|---|
| `darkmatter` (`--lib --test l1`, `effects-instrumentation`) | 515 run, 515 passed |
| `dmls` (`--lib --test l1`, `effects-instrumentation`) | 22 run, 22 passed |
| `darkmatter-cli` (`--lib --bins --test l1`, `test-fixtures`) | 17 run, 17 passed |
| `claudine-cli` (`--bins --test l1`) | 42 run, 42 passed |
| `claudine` (`--lib --test l1`) | 4 run, 4 passed |
| `sniff-cli` (`--test l1`) | 1 run, 1 passed |

The `darkmatter` count includes the two criterion 8/9 tests this audit
added and the runner it touched (see
[Tests added](#tests-added-by-this-audit)). No test failed.

## Grammar and Catalog

| # | Criterion | Evidence | Status |
|---|---|---|---|
| 1 | Grammar parses `tuple`, `function`, `\|`, `categories:`; rejects the listed bad forms | `dm::l1 schema_grammar_descriptors::{tuple_forms_parse, function_type_with_every_constraint_parses, union_operator_parses_everywhere_a_type_is_written, categories_list_parses_in_order_with_optional_multi_line_descriptions, invalid_tuples_and_repeated_categories_are_rejected_at_the_offending_text, invalid_function_types_are_rejected_at_the_offending_text, a_union_inside_a_yaml_list_arm_and_a_question_mark_on_a_property_are_rejected, every_new_form_round_trips_through_serialization}` | pass |
| 2 | TypeScript precedence and grouping | `dm::l1 schema_grammar_descriptors::array_suffix_binds_tighter_than_the_union_operator` | pass |
| 3 | Tuples and `\|` lower to standard JSON Schema; operator and YAML list identical | `dm::l1 schema_grammar_descriptors::{snapshot_fixed_tuple, snapshot_variadic_tuple_with_optional_position, snapshot_union_operator, snapshot_empty_tuple_and_category_annotation, union_operator_and_yaml_list_are_the_same_definition}` | pass |
| 4 | Function-typed property under `$schema` fails at load | `dm::l1 schema_grammar_descriptors::a_document_schema_that_reaches_a_function_fails_to_load_naming_the_path`; `md-cli::l1 schema_grammar_types::schema_validate_rejects_a_function_typed_property_as_a_schema_error` | pass |
| 5 | Categories load and emit | `dm::l1 schema_grammar_descriptors::{a_declared_categories_list_limits_category_names, a_category_on_a_data_type_is_an_annotation, categories_list_parses_in_order_with_optional_multi_line_descriptions}`; `dm::l1 schema_enhancements_matrix::envelope::documented_categories_matrix` | pass |
| 6 | Duplicate `pr_list` overloads fail at load | `dm::l1 schema_grammar_descriptors::overloads_with_one_binding_contract_fail_to_load` | pass |
| 7 | New catalog complete and the only one; consumers read the library API | `dm::l1 function_dispatch::the_embedded_catalog_and_the_registry_have_the_same_functions`; `dm::lib markdown::compose::expression::functions::registration_tests::{catalog_and_runtime_bindings_have_bidirectional_canonical_parity, each_registration_compiles_its_catalog_overloads}`; `dm::l1 function_binding_registry::every_registered_function_has_a_row`; `cl-cli::l1 context_command::context_expressions_includes_every_function`; review R7 | pass + review |
| 8 | Executable examples pass structurally; at least 72; `=> 3` vs `"3"`, key order, `=> error` | `dm::lib markdown::compose::expression::catalog::tests::{every_executable_example_evaluates_to_its_declared_result, every_embedded_example_file_evaluates_to_its_returns, executable_examples_number_at_least_72_and_display_only_ones_are_skipped_by_design, example_results_compare_structurally}` (100 executable examples) | pass |
| 9 | Display-only and file examples shown; display-only never run | `dm::lib markdown::compose::expression::catalog::tests::{executable_examples_number_at_least_72_and_display_only_ones_are_skipped_by_design, every_display_only_example_carries_a_non_empty_reason, examples_keep_their_authored_text_and_files_are_embedded, the_generated_table_shows_inline_display_only_and_file_examples, narrative_doc_function_table_matches_catalog}`; `cl-cli::bin commands::context::tests::function_examples_render_both_forms_one_per_line` | pass |

## One-pass Migration

| # | Criterion | Evidence | Status |
|---|---|---|---|
| 10 | Every registered function binds through the engine (accepted / rejected / converted, counter 0 on rejection) | `dm::l1 function_binding_registry::rows::*` (116 rows, one per canonical; impossible cells per ruling R1) and the guard `function_binding_registry::every_registered_function_has_a_row`, which compares the rows with `dispatchable_canonical_names()` both ways | pass |
| 11 | No function performs its own coercion | `dm::l1 function_binding_registry::no_handler_local_coercion_helpers_remain`; `dm::l1 function_dispatch::a_type_error_never_enters_the_handler`; review R11 | pass + review |
| 12 | `number()` and `round()` charter contract; catalog names `fallback` with examples | `dm::l1 pure_function_contracts::{number_converts_numberlike_and_recovers_through_its_fallback, round_rounds_half_away_from_zero_and_recovers_through_its_fallback}` (every listed case, counters); catalog signatures `number(x: numberlike, fallback?: number)` and `round(x: numberlike, fallback?: number)` with executable examples run by `dm::lib …catalog::tests::every_executable_example_evaluates_to_its_declared_result`. the arity case is the catalog example `number() => error` (added in Phase 6 by the integrator) | pass |
| 13 | Behavior-change ledger complete and reviewed | `behavior-ledger.md`: 116 rows, no undecided function, final reconciliation section; author review recorded 2026-10-22 for rows 1–113; rows 114–116 and the reconciliation added 2026-10-23 await review | human-pending |

## Coercion Engine: Function Calls

| # | Criterion | Evidence | Status |
|---|---|---|---|
| 14 | Text → number; counter exactly 1 | `dm::l1 pure_function_contracts::min_converts_numeric_text_and_rejects_booleans_and_null`; `dm::l1 function_call_binding::text_converts_to_a_number_and_the_handler_runs_once` | pass |
| 15 | Number → text | `dm::l1 pure_function_contracts::upper_converts_a_number_to_text` | pass |
| 16 | Complete type error; never reaches the function | `dm::l1 function_binding_conversions::min_pear_type_error_names_function_parameter_type_value_reason` | pass |
| 17 | Constraints enforced, not repaired | `dm::l1 function_binding_conversions::{three_letter_acronym_constraints, integer_constraint_rejects_two_point_five}` | pass |
| 18 | `any` preserves the original value | `dm::l1 pure_function_contracts::is_number_inspects_the_original_value` | pass |
| 19 | Rejected conversions rejected | `dm::l1 function_binding_conversions::boolean_null_and_text_list_are_rejected`; `dm::l1 pure_function_contracts::min_converts_numeric_text_and_rejects_booleans_and_null` | pass |
| 20 | Union rule, every option order | `dm::l1 function_binding_conversions::union_rule_table_both_orders` | pass |
| 21 | Exactness check, including the expression literal | `dm::l1 function_binding_conversions::{echo_exactness_table, literal_beyond_safe_integer_is_rejected}`; `dm::l1 numeric_admission::an_inexact_expression_literal_is_rejected_at_the_literal` | pass |
| 22 | Every argument checked, one error for both | `dm::l1 function_binding_conversions::one_error_reports_both_parameters` | pass |
| 23 | Overload selection in both orders; no provider request | `dm::l1 function_call_binding::{overloads_select_count_for_numeric_text_and_numbers, overload_failure_lists_every_signature_and_reason}` (both signature orders); `dm::lib markdown::compose::expression::functions::{pull_requests,cicd}::tests::counts_select_the_count_overload_or_fail_before_the_handler`; `dm::lib markdown::compose::tests::provider_network::list_counts_bind_before_any_provider_request` (fake provider request count 0) | pass |
| 24 | Containers follow the declared structure | `dm::l1 function_binding_conversions::containers_table_person_and_sum` | pass |
| 25 | Native values serialize into `json` | `dm::l1 function_binding_conversions::json_parameter_serializes_native_and_keeps_text` | pass |
| 26 | `numberlike` delivered as sent | `dm::l1 function_binding_conversions::numberlike_parameter_delivers_original` | pass |
| 27 | `boolish` delivered as sent | `dm::l1 function_binding_conversions::boolish_parameter_delivers_original` | pass |

## Coercion Engine: Frontmatter

| # | Criterion | Evidence | Status |
|---|---|---|---|
| 28 | Number → text | `dm::l1 frontmatter_coercion_engine::number_version_composes_as_text`; `md-cli::l1 compose_schema::compose_frontmatter_writes_engine_conversions` | pass |
| 29 | Boolean words | `dm::l1 frontmatter_coercion_engine::yes_and_on_become_true` | pass |
| 30 | Lenient numbers and precision | `dm::l1 frontmatter_coercion_engine::{plus_and_underscore_numbers_convert, unsafe_integers_fail_validation}`; `dm::l1 numeric_admission::native_frontmatter_numbers_no_f64_holds_fail_to_load_with_location` | pass |
| 31 | Root unions order-independent | `dm::l1 frontmatter_coercion_engine::root_union_result_is_order_independent` | pass |
| 32 | Nested objects converted | `dm::l1 frontmatter_coercion_engine::nested_object_count_converts` | pass |
| 33 | Native values serialize into `json`/`yaml`; written key order; null not serialized | `dm::l1 frontmatter_coercion_engine::{json_property_serializes_mapping, json_or_yaml_keeps_written_key_order, null_is_not_serialized}` | pass |
| 33a | The completed Changes table | `dm::l1 frontmatter_coercion_engine::{enum_number_composes_as_text, pattern_key_dictionary_converts, nested_union_numberlike_and_patterns}`; `dm::l1 raw_schema_conversion::ref_defs_type_array_oneof_convert_referenced_number` | pass |
| 33b | `numberlike`/`boolish` formats agree with the engine; detect snapshot | `dm::l1 frontmatter_coercion_engine::numberlike_boolish_formats_agree_with_engine_on_probe_list`; `dm::l1 schemas_convert_snapshots::{snapshot_atom_numberlike, snapshot_atom_boolish}`; `md-cli::l1 schema_detect::schema_detect_json_emits_json_schema`; review R33b | pass + review |
| 34 | `numberlike`/`boolish` validated, not normalized | `dm::l1 frontmatter_coercion_engine::preservation_types_compose_unchanged` | pass |
| 35 | One engine; old frontmatter-only rule code gone | `dm::l1 coercion_engine_parity::*` (17 shared tables through compose and dispatch, `both_sides_observe_keep_convert_and_reject`, `old_frontmatter_rule_code_is_gone`); `dm::l1 coercion_translator_conformance::*` | pass |

## Design-time Checking

| # | Criterion | Evidence | Status |
|---|---|---|---|
| 36 | Compatibility API returns the five categories | `dm::l1 design_time_call_check::categories::{the_worked_example_rows, criterion_36_pairs, unions_quantify_over_their_options, containers_reason_about_their_elements}` | pass |
| 37 | Category 5 rejected at composition, for every function | `dm::l1 compose_call_precheck::rejection::{a_proven_bad_call_rejects_before_any_expression_runs, the_message_reads_like_the_type_error_and_names_the_key, a_bad_body_call_rejects_before_frontmatter_runs, every_registered_function_rejects_a_proven_bad_call}`; `md-cli::l1 compose_schema_enhancements::a_proven_bad_call_in_a_dead_branch_rejects_before_any_evaluation` | pass |
| 38 | Category 1 conversions compose; `1234` fails at runtime | `dm::l1 compose_call_precheck::conversions::a_category_1_conversion_composes_and_fails_by_value_at_runtime`; `dm::l1 design_time_call_check::categories::three_letter_acronym_takes_a_typed_number` | pass |
| 39 | `claudine context --expressions` grouping | `cl-cli::l1 context_command::context_expressions_function_order_snapshot`; `cl-cli::bin commands::context::tests::expression_groups_follow_category_list_order_and_sort_alphabetically` | pass |
| 40 | DMLS rows a–e under both settings, from file and editor | `dmls::l1 expression_call_diagnostics::settings::{rows_a_to_e_by_default, rows_a_to_e_from_the_config_file_and_its_reload, rows_a_to_e_from_editor_settings_flipped_at_runtime}` | pass |
| 41 | Category 5 underlined, in agreement with composition | `dmls::l1 expression_call_diagnostics::ranges::{an_argument_error_ranges_the_argument_and_an_arity_error_the_call, frontmatter_and_condition_calls_range_the_argument}`; `dmls::l1 call_check_agreement::composition_and_the_editor_reject_the_same_documents`; `dmls::lib diagnostics::call_check::tests::the_severity_ladder_follows_the_category_table`; review R41 | pass + review |
| 42 | `file_exists` binds like every function | `dm::l1 context_function_contracts::file_exists_inspects_its_argument_without_converting_it`; `dm::l1 design_time_call_check::categories::null_is_accepted_only_by_an_explicit_null_arm`; `dmls::l1 expression_call_diagnostics::verdicts::file_exists_takes_a_number_silently`; `dmls::lib config::tests::expressions_strict::strict_is_the_only_expressions_setting`; review R42 | pass + review |
| 43 | Ledger null rule | `dm::l1 pure_function_contracts::the_ledger_null_rule_holds_for_pure_functions`; `dm::l1 context_function_contracts::file_exists_inspects_its_argument_without_converting_it`; ledger null column complete for all 116 functions and observed through `md compose` (ledger "Final reconciliation") | pass |
| 44 | Charter-only contracts | `dm::l1 pure_function_contracts::length_and_contains_follow_the_charter`; `dm::l1 design_time_call_check::categories::source_exclusions_are_part_of_classification`; `dm::l1 context_function_contracts::{validate_schema_override_repairs_an_incomplete_page, validate_schema_invalid_override_fails_a_valid_page, validate_schema_empty_override_equals_the_one_argument_form, validate_schema_is_nonmutating_across_repeated_calls, has_command_true_looks_for_a_command_named_true}` | pass |
| 45 | Map functions | `dm::l1 pure_function_contracts::keys_and_values_keep_insertion_order`; `dm::lib markdown::compose::expression::functions::tests::fn_collection::keys_and_values_keep_insertion_order`; catalog examples `keys({b: 1, a: 2}) => ["b", "a"]`, `values({b: 1, a: 2}) => [1, 2]` run by the example runner | pass |
| 46 | Numeric operators and presentation | `dm::l1 numeric_admission::{non_finite_results_are_errors, equality_and_ordering_agree_on_represented_values, whole_numbers_never_pass_through_an_integer_cast}`; `dm::lib markdown::numeric::tests::presentation_never_narrows` | pass |
| 47 | `(A \| B)[]` is the list-of-union representation | `dm::l1 schema_grammar_descriptors::grouped_union_list_matches_a_named_union_list` (identical lowering; compiled schema accepts `["a", 1]`, rejects `[true]`); `dm::l1 frontmatter_coercion_engine::list_union_converts_element_by_element` | pass |
| 48 | Variable-type projection | `dm::l1 design_time_call_check::projection::{the_worked_example_projects_its_declared_types, no_typed_projection_makes_every_variable_unknown, a_mixed_root_union_projects_nothing, projection_is_deterministic_and_reads_only_the_schema}`; review R48 | pass + review |
| 49 | Calls in branches that do not run are checked | `dm::l1 design_time_call_check::walk::branches_that_do_not_run_are_checked`; `dm::l1 compose_call_precheck::branches::{a_dead_ternary_branch_is_checked, a_short_circuited_operand_is_checked, the_body_of_a_false_block_is_checked, a_block_condition_is_checked, a_transclusion_condition_is_checked}`; `dmls::l1 expression_call_diagnostics::verdicts::branches_that_do_not_run_are_checked` | pass |

## Review-added Contract Checks

| # | Criterion | Evidence | Status |
|---|---|---|---|
| 50 | Projection agrees with property presence; `fm.` spellings | `dm::l1 design_time_call_check::projection::presence_follows_the_shared_rule`; `dm::l1 design_time_call_check::walk::fm_spellings_reject_like_the_bare_name`; `dm::l1 compose_call_precheck::rejection::frontmatter_namespace_access_rejects_like_the_bare_name`; `dm::l1 design_time_call_check::categories::unions_quantify_over_their_options` (`number` → `number \| null` is category 1) | pass |
| 51 | Recovery agrees between composition and editor | `dm::l1 design_time_call_check::whole_calls::numeric_recovery`; `dm::l1 compose_call_precheck::conversions::numeric_recovery_composes_and_an_unrecoverable_call_is_rejected`; `dmls::l1 expression_call_diagnostics::verdicts::numeric_recovery` | pass |
| 52 | Content-format classification matches binding | `dm::l1 design_time_call_check::categories::content_formats` | pass |
| 53 | Lazy evaluation preserved; skipped bad call still rejected | `dm::l1 function_dispatch::a_skipped_lazy_operand_is_never_entered`; `dm::l1 function_call_binding::{lazy_binding_never_binds_unevaluated_operands, lazy_binding_reports_each_operand_as_it_binds}`; `dm::l1 compose_call_precheck::branches::lazy_operands_stay_lazy_and_are_still_checked`; `dm::l1 design_time_call_check::walk::checking_never_enters_a_handler`; `dmls::lib diagnostics::call_check::tests::diagnostics_never_enter_a_handler` | pass |
| 54 | Checking precedes effects; respects the requested surface | `dm::l1 compose_call_precheck::rejection::a_proven_bad_call_rejects_before_any_expression_runs`; `dm::l1 compose_call_precheck::surface::{non_executable_syntax_is_not_checked, frontmatter_only_composition_leaves_an_invalid_body_untouched, frontmatter_only_composition_still_checks_frontmatter}`; `dmls::l1 expression_call_diagnostics::verdicts::fenced_code_and_literals_are_not_checked` | pass |
| 55 | Raw schema assertions survive conversion | `dm::l1 raw_schema_conversion::{oneof_overlap, ref_siblings, referenced_number_with_sibling_constraint, overlapping_literal_and_pattern_keys, typed_additional_keys, unmodeled_multiple_of, cyclic_local_reference, failure_leaves_input_unchanged, alternative_and_pattern_order_is_irrelevant}` | pass |
| 56 | Grammar closes indirect invalid forms | `dm::l1 schema_grammar_descriptors::{invalid_tuples_and_repeated_categories_are_rejected_at_the_offending_text, invalid_function_types_are_rejected_at_the_offending_text, a_document_schema_that_reaches_a_function_fails_to_load_naming_the_path, tuples_validate_positions_omission_and_length}`; `dm::l1 schema_enhancements_matrix::envelope::documented_function_type_matrix` | pass |
| 57 | Catalog identity and return contracts survive migration | `dm::l1 function_catalog_parity::{the_registered_aliases_are_the_inventoried_ones, aliases_resolve_as_before, canonical_names_resolve_case_insensitively, documentation_lists_canonicals_once}`; `dm::lib markdown::compose::expression::catalog::tests::semantically_duplicate_overloads_fail_catalog_load`; `dm::l1 function_dispatch::a_wrong_return_is_an_implementation_contract_error_and_is_not_converted`; `dm::l1 function_call_binding::wrong_return_reports_implementation_contract_error` | pass |
| 58 | Map ordering does not silently alter existing contracts | Insertion order: `dm::lib markdown::frontmatter::tests::parsed_frontmatter_retains_nested_mapping_order_as_metadata`, `dm::l1 frontmatter_coercion_engine::json_or_yaml_keeps_written_key_order`, `dm::l1 pure_function_contracts::keys_and_values_keep_insertion_order`, `dm::lib …functions::tests::fn_collection::as_json5_uses_the_formatter_object_idiom`. Stable-order consumers kept sorted: `md-cli::l1 schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`, `dm::lib markdown::hash::tests::non_strict_frontmatter_hash_ignores_nested_key_order`, `dm::l1 benchmark_fixtures::benchmark_manifest_matches_recorded_identities`, `cl::lib opencode_config::tests::yolo_permission_block_serializes_independent_of_insertion_order`, `cl::l1 action_shape_control::proxy_with_iteration_follows_authored_order_and_is_complete`, `sniff-cli::l1 snapshots::json_output_keys_are_sorted_at_every_depth`; review R58 | pass + review |
| 59 | Whole-number admission in every spelling | `dm::l1 frontmatter_coercion_engine::whole_number_admission_every_spelling`; `dm::l1 schema_grammar_descriptors::numeric_metadata_rejects_what_an_f64_cannot_hold` (defaults, enum members, literals, constraints); `dm::lib markdown::numeric::tests::{whole_numbers_are_exact_in_every_spelling, json_numbers_and_supplied_floats, decimals_round_and_extremes_are_rejected}`; `dm::l1 numeric_admission::quoted_inexact_text_against_a_number_fails_validation_with_the_reason` | pass |

## Summary

- **61 criteria: 53 pass, 7 pass + review (7, 11, 33b, 41, 42, 48, 58),
  1 human-pending (13).**
- **No criterion lacks evidence, and no cited test fails.**
- Criterion 12's arity case, open when the audit ran, is closed: the
  integrator added `example(number() => error)` to the catalog.

## Review items

- **R7 (criterion 7).** `darkmatter/schemas/partials/expression-functions.yaml`
  no longer exists, and no source, test, or shipped document reads it (the
  only remaining mentions are artifacts of two completed features). The
  catalog is embedded from `functions.yaml` (`catalog/mod.rs`
  `include_str!`). `claudine context --expressions` reads
  `darkmatter::markdown::compose::expression::{ExpressionFunctionDescriptor, function_descriptors}`;
  DMLS reads `expression_function_descriptors()` (`dmls/src/overlay/expressions.rs`).
- **R11 (criterion 11).** Grepped every handler file in
  `darkmatter/lib/src/markdown/compose/expression/functions/` for parsing
  and type-inspecting conversion. The remaining parses are domain parsing,
  not argument coercion: ISO dates and durations (`dates.rs`, `mod.rs`),
  filename index digits (`fs.rs`, `mod.rs`), and an IP literal already
  admitted by the `ip-address` pattern (`network.rs`). Type-inspecting
  handlers whose documented purpose is the inspection: `number`/`round`
  (`numberlike_value`, through the engine's one numeric-text parser),
  `contains` (renders the needle, M10), the list renderers, and
  `ensure_leading`/`ensure_trailing` (return a number when the joined text
  is a number, per their `string | number | null` return type). This agrees
  with the Phase 4 integrator review.
- **R33b (criterion 33b).** `md schema detect` never infers `numberlike` or
  `boolish` (`schemas/detect.rs` has no reference to either), so its
  `--format json` output did not change and there is no detect snapshot to
  update; `schema_detect_json_emits_json_schema` still passes. The formats
  are pinned by the convert snapshots `snapshot_atom_numberlike` and
  `snapshot_atom_boolish`.
- **R41 (criterion 41).** `dmls/src/diagnostics/call_check.rs` imports the
  categories from `darkmatter::markdown::compose::expression` and only maps
  each `ArgumentCategory` to a severity under `expressions.strict`. It has no
  conversion code (no `coerce`, numeric admission, or `Shape` use) and no
  category rules.
- **R42 (criterion 42).** `ExpressionsConfig` (`dmls/src/config/mod.rs`) has
  one field, `strict`; nothing in `dmls/src/config/` names `file_exists`.
- **R48 (criterion 48).** The projection API takes only a schema reference,
  so it has no file, value, or expression input through which it could do
  I/O or mutate frontmatter; `projection_is_deterministic_and_reads_only_the_schema`
  pins the observable part.
- **R58 (criterion 58).** The Phase 2 triage (implementation log, Phase 2
  "Integrator step 1") classified each of the 48 changed expectations by
  contract, sorted the three stable-order consumers explicitly, and
  refreshed the 41 conversion snapshots only after checking each old/new
  pair is JSON-equal. The audit found no snapshot accepted without that
  check.

## Cross-cutting evidence

- **Passive shipped-artifact corpus test:** `dm::l1 shipped_schema_corpus::*`
  (7 tests: shipped schema artifacts parse passively, loadable schemas
  resolve and lower, Markdown inline schemas and named-type references
  resolve, research contracts, trigger fixtures, migrated definitions);
  `functions.yaml` and `mutations.yaml` are classified as function catalogs
  that refuse to apply as document schemas. All pass.
- **Normal CLI paths, not only internal tests:**
  - `md compose` end to end: `md-cli::l1 compose_schema_enhancements::{converted_calls_compose_to_their_results, an_unbindable_call_fails_compose_with_the_type_error, a_proven_bad_call_in_a_dead_branch_rejects_before_any_evaluation}`,
    `md-cli::l1 schema_grammar_types::{compose_accepts_a_document_matching_a_tuple_and_a_union, compose_rejects_a_document_whose_tuple_has_too_many_positions}`,
    `md-cli::l1 compose_schema::{compose_frontmatter_writes_engine_conversions, compose_reports_the_engine_reason_for_a_failed_conversion}`.
  - `claudine context --expressions`: `cl-cli::l1 context_command::context_expressions_function_order_snapshot`
    and the other `context_expressions_*` tests. All pass.

## Tests added by this audit

Criterion 8 had no test for the example count or for the structural
comparison, and criterion 9 had no report of the skipped display-only
examples. The comparison lives in the in-source catalog tests, so the new
tests sit beside it in
`darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs`:

- `executable_examples_number_at_least_72_and_display_only_ones_are_skipped_by_design`
  (100 executable examples; 43 display-only examples skipped, each with its
  reason, none executed)
- `example_results_compare_structurally` (`=> 3` fails against `"3"` and
  holds against `3.0`; `{a: 1, b: 2}` holds against `{b: 2, a: 1}` and fails
  on missing or extra keys; `=> error` holds only on failure)

To share one definition with the runner, the runner
`every_executable_example_evaluates_to_its_declared_result` now uses the new
helpers `partition_call_examples` and `example_holds`; its behavior is
unchanged. Command:
`cargo nextest run -p darkmatter --features effects-instrumentation --lib -E 'test(/expression::catalog::tests::(executable_examples_number|example_results_compare|every_executable_example)/)'`
→ 3 passed. `cargo clippy -p darkmatter --lib --tests --features effects-instrumentation` is clean.

## Matrix review

Sources: the plan's Phase 1 "Input Robustness Matrix", the
`inventory/reader-matrix.md` "Cell coverage" appendix (which lists the cells
blocked at Phase 1), the matrix and config tests as they now stand, and the
Phase 5 `expressions.strict` review. Results by reader, then the code scan.

### (a) YAML catalog/standalone envelope

Closed since Phase 1: the whole `categories` column
(`envelope::documented_categories_matrix`), the function-type controls
(`envelope::documented_function_type_matrix`), and the `types` and root
`$schema` columns (`shipped_tagged_envelope_matrix`,
`shipped_pure_envelope_matrix`). Ruling R5 keeps the documented `Declines`
behavior for general schema files, pinned in `schema_yaml_spellings`
(`TAGGED_CELLS`).

**Closed by the Phase 6 integrator after this audit** (the three bullets
below record what the audit found): `dm::lib …catalog::tests::a_missing_null_or_wrong_kind_does_not_load`
drives the `kind` column (absent, `null`, `~`, `""`, wrong string, number,
duplicate key, a second document, trailing invalid content) through
`load_catalog`; the loader now rejects an entry written as `null`, `~`, or
empty with "`types.<name>` is null, not a function signature", pinned by
`…catalog::tests::an_entry_written_as_null_does_not_load`. A missing
canonical keeps the registry-build error (`BindingWithoutCatalog`): the
loader cannot know which names are registered, and the shipped catalog is
pinned by the parity test.

Found by the audit:

- **`kind` column, rows Absent, Explicit null, Wrong whole type, Empty,
  Duplicate key, Trailing/invalid content, for the catalog loader.**
  `load_catalog` (`expression/catalog/mod.rs`) does return E: no envelope →
  "not a SimplifiedSchema document; expected `kind: schema`", a non-tagged
  envelope → "expected the tagged envelope", and YAML errors propagate. The
  only loader-level test is `a_document_without_categories_does_not_load`
  (the `categories` row). The cells are implemented but unasserted.
- **`types.<canonical>` column, row Explicit null, for the catalog loader.**
  R5 says E ("not a function signature"). The loader instead treats
  `types.upper: null` as a named type, the null type, and loads. The E
  arrives one step later, when the registry is built:
  `RegistryError::BindingWithoutCatalog` ("runtime binding has no catalog
  function"), which panics on first use of the embedded registry. So the
  outcome is E, but with a different message and at a different stage than
  R5 describes. No test feeds a null canonical. The parity tests prove only
  that the shipped catalog has none.
- **`types.<canonical>` column, row Absent (missing registered canonical).**
  Same mechanism (`BindingWithoutCatalog`), proven for the shipped catalog by
  `catalog_and_runtime_bindings_have_bidirectional_canonical_parity`. No
  fixture exercises the error path.

### (b) DMLS `expressions` configuration

Covered by `dmls::lib config::tests::expressions_strict::*` (8 tests) and
the LSP-session tests: positive control, absent, and empty table or object
on both sides; explicit null, `"yes"`, `1`, and `expressions = 5` on both
sides; TOML duplicate key and duplicate table; editor override in both
directions; whole-settings `null`; invalid merge falling back to the file
layer; malformed file recovering on reload.

**Closed by the Phase 6 integrator after this audit:**
`dmls::lib config::tests::expressions_strict::{empty_values_are_wrong_types_not_absent, a_list_section_is_read_positionally_as_every_section_is, a_syntactically_broken_file_is_rejected_whole, an_unknown_key_is_ignored}`
cover `""` and `[]` for `strict` and `expressions` (file: E, editor: file
layer with an error), lists (the shared sequence-as-struct reading, pinned:
`[true]` sets strict, `["yes"]` is an error), syntactically broken TOML
(the whole file is rejected), and ruling R6 (an unknown key is ignored on
both sides). The settings-root columns stay with the shared overlay tests.

Found by the audit:

- **Rows One wrong element and Every element wrong, all four `expressions`
  / `expressions.strict` columns (TOML and editor JSON).** No test supplies
  a collection. The plan expects E for TOML and "file layer" for the editor.
  Phase 5 recorded a known departure without a test: editor
  `"expressions": [true]` deserializes as the struct `{strict: true}`
  (serde's sequence-as-struct form), so a collection *sets* strict instead
  of falling back. The TOML `expressions = [true]` cell is unverified and
  likely behaves the same way.
- **Row Empty, columns TOML `expressions.strict` and JSON editor
  `expressions.strict`.** The plan expects E or file layer for `""` and
  `[]`. No test covers them. The empty table or object for the
  `expressions` column is covered.
- **Row Trailing/invalid content, TOML columns.** The reload test uses a
  type error (`strict = "yes"`), not syntactically invalid TOML. The
  generic file-syntax path is pinned elsewhere
  (`workspace_formatting_settings`), but not for `[expressions]`.
- **Ruling R6 cell (unknown key inside `[expressions]`, silently
  ignored).** R6 asked for an explicit matrix cell. No test asserts that
  `strcit = true` leaves strict false with no diagnostic.
- **Columns JSON settings root / `dmls` section, rows Wrong whole type, One
  wrong element, Every element wrong.** These are covered only by the
  pre-existing generic overlay tests, not re-asserted for `expressions`.
  They are low risk because the code path is shared.

Serde review: `ExpressionsConfig` carries `#[serde(default)]` at struct
level, the same as `SchemaConfig` and `StyleConfig`. Absent `strict`
therefore means `false`, which the plan intends. A present `null` or a
wrong type still fails to deserialize and is reported, which is tested.
This is not a defect.

### (c) Raw JSON Schema

Closed since Phase 1: `.json` duplicate keys (R3) and whole-document cells
now fail as located `SchemaDocument` errors; `enum: []`/`prefixItems: []`
follow the meta-schema (R7) inside the keyword rows; conversion positive
controls (`raw_json_schema_conversion_controls`,
`raw_yaml_schema_conversion_controls`); numeric tokens in bounds,
`multipleOf`, `enum`, `const`, `default`
(`raw_json_schema_numeric_token_matrix`,
`raw_yaml_schema_numeric_token_matrix`). The documented unobservable
controls (`$schema`, `$vocabulary`) remain E-only by design. Nothing open.

### (d) Frontmatter values

Closed since Phase 1: numeric exactness, overflow, and underflow (18
numeric cells in `frontmatter_value_matrix`); `numberlike`/`boolish`
through containers, numeric text in list and object members, tuple slots,
pattern-key values, a whole-value union, and the `json` content target
including duplicate keys in its text (`frontmatter_conversion_matrix`).
Nothing open.

### Code scan of the feature's changed files

Scope: the 121 schema, expression, coercion, numeric, call-check, catalog,
frontmatter, and DMLS-config source files changed since the merge base
(committed and uncommitted), filtered to added lines outside tests.
Patterns: `#[serde(default)]`, `unwrap_or_default()`,
`filter_map(.. as_str())`, `.ok()`, and `Option<T>` fields where absent and
null must differ.

- `#[serde(default)]`: only `dmls` `ExpressionsConfig`, reviewed above (not
  a defect).
- `Option<T>` where absent and null must differ: the catalog's
  `categories` (absent → unrestricted; `null` → "`categories` must be a
  list", tested), `display_only`, and DMLS overlay sections (null tested).
  No conflation found.
- `filter_map(Value::as_str)` in `schemas/json_shape.rs` (`required`
  names) and every `as_u64().and_then(usize::try_from(..).ok())` count
  (`minLength`, `maxLength`, `minItems`, `maxItems`, `minProperties`): each
  clears the shape's `exact` flag when it drops a value, so the complete
  validator still judges the instance (the assertion stays in force, V).
  Not a defect.
- `.ok()` on parses: `call_precheck.rs` skips an expression that does not
  parse; its evaluating stage reports the parse error (documented at
  `first_rejected`). `schemas/coerce.rs` treats a conversion validator that
  cannot be built as `Conversion::Unavailable`; the same schema then fails
  to build its main validator, which is E. `grammar.rs` keeps an
  inadmissible numeric spelling as a bare word, and the consumer reports
  why (pinned by `numeric_metadata_rejects_what_an_f64_cannot_hold`).
  `expression::to_number` and `ensure_*` use the canonical numeric-text
  parser by documented contract. The `json_shape.rs` URI and percent-decoding
  `.ok()?` fall back to an unresolved reference, and the validator build
  rejects that. The date, duration, and filename-index parses are domain
  parsing.
- `unwrap_or_default()`: display text (`call_binding/error.rs`, catalog
  descriptions), completion candidates (`dmls/src/providers/frontmatter.rs`),
  verdict assembly (`call_check.rs`), and path helpers. None is
  load-bearing.
- Out of scope but flagged by the pattern: `schemas/catalog.rs`
  `from_host_file`, which reads declared `$schema` keys with
  `.ok()…filter_map(as_str)…unwrap_or_default()`, comes from the earlier
  lifecycle-ownership work, not this feature. It only screens reserved
  names before the real loader parses the file.

**Result:** the scan found no silent-data-loss defect. The open items are
the untested matrix cells listed in (a) and (b), including two behavioral
departures from the plan's expected outcome (the null canonical's E stage
and message; the editor collection-as-struct quirk).

## Downstream gates

Mechanism: `darkmatter/lib/Cargo.toml` enables `serde_json` `preserve_order`
(not the workspace `Cargo.toml`). Cargo unifies features per build graph, so
every package whose graph contains `darkmatter` now builds **every** crate
in that graph (for example `sniff`, `playa`, `biscuit-speaks`, and
`messenger` inside their CLIs) with insertion-ordered `serde_json::Map`.
`cargo tree -e features -i serde_json` shows `darkmatter` as the enabler in
each graph below. A package outside this closure is unaffected when built
alone, and workspace-wide builds already had the feature through
`tauri-build` and `tree-sitter`. The same packages also compile against
Darkmatter's public API, which this feature changed.

Reverse-dependency closure of `darkmatter` (from `cargo metadata`), beyond
the `darkmatter` and `claudine` areas:

| Package | Why it needs a gate | Gate |
|---|---|---|
| `sniff-cli` | JSON output with insta snapshots (5 JSON-shaped); keys now sorted explicitly by 5cf0c5ff9 | L1 (`snapshots::*` and the rest) |
| `messenger-cli` | Insta snapshots, 3 of them JSON (`info_snapshot__snapshot_{full,empty}_json_rendering`, `info::tests::snapshot_json_rendering`); calls `darkmatter::markdown::compose`. Phase 5 only ran `cargo check` | L1 test run |
| `messenger` | `serde_json::Value`/`Map` in 25 source files; uses `darkmatter::markdown::compose`. Passed in Phase 5 (738); rerun on the final tree | L1 test run |
| `research` | Heavy `serde_json` serialization (36 call sites) including persisted metadata and schema migration through `serde_json::Value`, so written key order can change. Phase 5 assumed only API use mattered and ran `cargo check` only | test run |
| `research-cli` | Links `research`; no test targets of its own | `cargo check --tests` |
| `biscuit-icon-cli` | Uses `darkmatter::markdown::output`; its graph's `serde_json` changes order | L1 (`cli`) |
| `playa-cli` | Uses `darkmatter::markdown` and `darkmatter::testing`; serializes JSON | L1 (`detached_cli`) |
| `biscuit-speaks-cli` | Uses `darkmatter::markdown::output`; links `biscuit-speaks`, which serializes provider JSON | L1 (`cli_test`) |

The rest of the closure (`claudine`, `claudine-cli`, `claudine-contract`,
`claudine-gen`, `dmls`, `darkmatter-cli`) is covered by the `darkmatter` and
`claudine` area gates. `claudine-contract` and `claudine-gen` belong in the
`claudine` gate because both sit in the closure.

Spot check run by this audit: `cargo check --tests -p messenger-cli -p
messenger -p research` compiles cleanly on the current tree. No downstream
test gate was run; the integrator owns them.
