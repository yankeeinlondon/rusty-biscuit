# Suggest Constraint Acceptance Test Matrix

Every acceptance criterion has one primary executable owner. All tests are
enabled and run under the ordinary `just test` Level-1 gate. "DMLS L1" rows are
in-memory LSP session tests (`Connection::memory()` + in-process server thread —
no real terminal or terminal harness). Responsible implementation paths in the
table are relative to the `darkmatter/` package area.

| AC | Primary test | Kind | Responsible implementation |
|---:|---|---|---|
| 1 | `suggest_phase1_eligible_scalar_and_array_forms_parse` | Unit | `lib/src/markdown/schemas/simplified/{grammar.rs,types.rs}` |
| 2 | `suggest_phase1_eligible_scalar_and_array_forms_parse` | Unit | `lib/src/markdown/schemas/simplified/grammar.rs` |
| 3 | `suggest_phase1_cardinality_is_per_complete_property_definition` | Unit | `lib/src/markdown/schemas/simplified/{mod.rs,grammar.rs}` |
| 4 | `suggest_phase1_conversion_preserves_string_interpretation_and_order` | Snapshot | `lib/src/markdown/schemas/simplified/{grammar.rs,convert.rs}` |
| 5 | `suggest_phase1_invalid_decimal_syntax_is_metadata` | Snapshot | `lib/src/markdown/schemas/simplified/{grammar.rs,convert.rs}` |
| 6 | `suggest_phase1_numeric_boundaries_follow_observable_json_round_trip` | Unit | `lib/src/markdown/schemas/simplified/grammar.rs` |
| 7 | `suggest_phase1_standalone_ranges` | DMLS L1 | `lib/src/markdown/schemas/simplified/grammar.rs`; `dmls/src/diagnostics/frontmatter.rs` |
| 8 | `suggest_phase1_duplicates_are_rejected_at_the_later_argument` | Unit | `lib/src/markdown/schemas/simplified/grammar.rs` |
| 9 | `suggest_phase1_conversion_preserves_string_interpretation_and_order` | Snapshot | `lib/src/markdown/schemas/simplified/convert.rs` |
| 10 | `suggest_phase1_metadata_does_not_restrict_document_values` | Unit | `lib/src/markdown/schemas/{validate.rs,mod.rs}` |
| 11 | `suggest_phase1_invalid_candidates_remain_loadable_and_lintable`; `inline_nested_inline_object_resolves_with_exact_candidate_span` | Unit | `lib/src/markdown/schemas/{resolve.rs,validate.rs}`; `dmls/src/overlay/suggestions.rs` |
| 12 | `projects_nested_inline_object_candidates_through_containing_scalar`; `standalone_nested_inline_objects_resolve_with_exact_candidate_spans` | Unit | `lib/src/markdown/schemas/simplified/{source.rs,types.rs}`; `dmls/src/overlay/suggestions.rs` |
| 13 | `suggest_phase1_candidate_constraints_target_scalar_or_array_items` | Unit | `lib/src/markdown/schemas/simplified/convert.rs`; `lib/src/markdown/schemas/validate.rs` |
| 14 | `suggest_phase1_inline_warning_has_exact_argument_range`; `suggest_phase1_decoy_field_does_not_steal_diagnostic_span` | DMLS L1 | `dmls/src/diagnostics/{frontmatter.rs,codes.rs}` |
| 15 | `suggest_phase1_standalone_ranges`; `standalone_nested_inline_objects_resolve_with_exact_candidate_spans`; `suggest_phase5_malformed_tagged_envelope_error` | Unit + DMLS L1 | `lib/src/markdown/schemas/resolve.rs`; `dmls/src/overlay` |
| 16 | `suggest_phase1_standalone_envelopes_resolve_consistently`; `suggest_phase1_pure_whole_file_reference_completion`; `suggest_phase1_tagged_whole_file_reference_completion` | Unit + DMLS L1 | `lib/src/markdown/schemas/resolve.rs`; `dmls/src/providers/frontmatter.rs` |
| 17 | `suggest_phase1_union_selection_and_raw_schema_exclusion` | DMLS L1 | `lib/src/markdown/schemas/resolve.rs`; `dmls/src/providers/frontmatter.rs` |
| 18 | `suggest_phase1_completion_positions`; `suggest_phase1_bare_block_array_dash`; `suggest_phase1_block_array_dash_space`; `suggest_phase1_block_array_partial_value`; `suggest_phase1_nested_property_from_later_root_arm` | DMLS L1 | `dmls/src/providers/frontmatter.rs` |
| 19 | `suggest_phase1_completion_positions`; `sibling_lint_does_not_hide_identical_valid_candidate`; `nested_lint_does_not_hide_identical_valid_candidate`; `suggest_phase1_numeric_prefix_uses_decoded_text` | Unit + DMLS L1 | `lib/src/markdown/schemas/simplified/query.rs`; `dmls/src/providers/frontmatter.rs` |
| 20 | `suggest_phase1_union_selection_and_raw_schema_exclusion`; `root_union_property_lint_does_not_hide_identical_valid_candidate`; `suggest_phase1_nested_property_from_later_root_arm` | Unit + DMLS L1 | `lib/src/markdown/schemas/simplified/query.rs` |
| 21 | `suggest_phase1_completion_positions`; `suggest_phase1_bare_block_array_dash`; `suggest_phase1_block_array_dash_space`; `suggest_phase1_block_array_partial_value` | DMLS L1 | `dmls/src/providers/frontmatter.rs` |
| 22 | `suggest_phase1_union_selection_and_raw_schema_exclusion` | DMLS L1 | `lib/src/markdown/schemas/resolve.rs`; `dmls/src/providers/frontmatter.rs` |
| 23 | `suggest_phase1_numeric_boundaries_follow_observable_json_round_trip` | Unit | `lib/src/markdown/schemas/simplified/grammar.rs` |
| 24 | `suggest_phase1_fixture_newlines_are_explicit` and all `suggest_phase*` sessions | Unit + DMLS L1 | Feature fixtures, source-span projection, and `FileReference` resolution |

The library tests live in `darkmatter/lib/tests/suggest_constraint_phase1.rs`.
The LSP session tests and their source fixtures live in
`darkmatter/dmls/tests/suggest_constraint_phase1.rs` and
`darkmatter/dmls/tests/fixtures/suggest_constraint/`.

## Phase 8 Validation Record

Every primary test listed above passes under `just test` (Level-1). Catalog/docs
consistency is covered by `markdown::schemas::about::tests::suggest_descriptor_*`
and `markdown::schemas::simplified::serialize::tests::round_trip_*_suggest`.
No acceptance criterion is justified only by manual inspection.

| AC | Passing location |
|---:|---|
| 1 | `suggest_constraint_phase1::suggest_phase1_eligible_scalar_and_array_forms_parse` |
| 2 | `suggest_constraint_phase1::suggest_phase1_eligible_scalar_and_array_forms_parse` |
| 3 | `suggest_constraint_phase1::suggest_phase1_cardinality_is_per_complete_property_definition` |
| 4 | `suggest_constraint_phase1::suggest_phase1_conversion_preserves_string_interpretation_and_order` (snapshot) |
| 5 | `suggest_constraint_phase1::suggest_phase1_invalid_decimal_syntax_is_metadata` (snapshot) |
| 6 | `suggest_constraint_phase1::suggest_phase1_numeric_boundaries_follow_observable_json_round_trip` |
| 7 | `suggest_constraint_phase1::suggest_phase1_standalone_ranges` (DMLS L1) |
| 8 | `suggest_constraint_phase1::suggest_phase1_duplicates_are_rejected_at_the_later_argument` |
| 9 | `suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata` (snapshot) |
| 10 | `suggest_constraint_phase1::suggest_phase1_metadata_does_not_restrict_document_values` |
| 11 | `suggest_constraint_phase1::suggest_phase1_invalid_candidates_remain_loadable_and_lintable`; `suggest_constraint_phase3::suggestion_metadata_neither_restricts_validation_nor_blocks_composition`; `overlay::suggestions::inline_nested_inline_object_resolves_with_exact_candidate_span` |
| 12 | `simplified::source::projects_nested_inline_object_candidates_through_containing_scalar`; `overlay::suggestions::standalone_nested_inline_objects_resolve_with_exact_candidate_spans` |
| 13 | `suggest_constraint_phase1::suggest_phase1_candidate_constraints_target_scalar_or_array_items` |
| 14 | `suggest_constraint_phase1::suggest_phase1_inline_warning_has_exact_argument_range`; `suggest_phase1_decoy_field_does_not_steal_diagnostic_span` (DMLS L1) |
| 15 | `suggest_constraint_phase1::suggest_phase1_standalone_ranges`; `overlay::suggestions::standalone_nested_inline_objects_resolve_with_exact_candidate_spans`; `suggest_phase5_malformed_tagged_envelope_error` (Unit + DMLS L1) |
| 16 | `suggest_constraint_phase1::suggest_phase1_standalone_envelopes_resolve_consistently`; `suggest_phase1_pure_whole_file_reference_completion`; `suggest_phase1_tagged_whole_file_reference_completion`; `suggest_constraint_phase4::mapping_envelopes_resolve_identically_with_origin_metadata`; `named_imports_share_pure_and_tagged_mapping_namespaces` |
| 17 | `suggest_constraint_phase1::suggest_phase1_union_selection_and_raw_schema_exclusion` (DMLS L1); `suggest_constraint_phase4::raw_json_schema_remains_distinct_and_cannot_supply_named_imports` |
| 18 | `suggest_constraint_phase1::suggest_phase1_completion_positions`; `suggest_phase1_bare_block_array_dash`; `suggest_phase1_block_array_dash_space`; `suggest_phase1_block_array_partial_value`; `suggest_phase1_nested_property_from_later_root_arm` (DMLS L1) |
| 19 | `simplified::query::{sibling_lint_does_not_hide_identical_valid_candidate,nested_lint_does_not_hide_identical_valid_candidate}`; `suggest_phase1_numeric_prefix_uses_decoded_text` (Unit + DMLS L1) |
| 20 | `suggest_constraint_phase1::suggest_phase1_union_selection_and_raw_schema_exclusion`; `simplified::query::root_union_property_lint_does_not_hide_identical_valid_candidate`; `suggest_phase1_nested_property_from_later_root_arm` (Unit + DMLS L1) |
| 21 | `suggest_constraint_phase1::suggest_phase1_completion_positions`; `suggest_phase1_bare_block_array_dash`; `suggest_phase1_block_array_dash_space`; `suggest_phase1_block_array_partial_value` (DMLS L1) |
| 22 | `suggest_constraint_phase1::suggest_phase1_union_selection_and_raw_schema_exclusion` (DMLS L1) |
| 23 | `suggest_constraint_phase1::suggest_phase1_numeric_boundaries_follow_observable_json_round_trip` |
| 24 | `suggest_constraint_phase1::suggest_phase1_fixture_newlines_are_explicit` + all `suggest_phase*` sessions (DMLS L1) |
