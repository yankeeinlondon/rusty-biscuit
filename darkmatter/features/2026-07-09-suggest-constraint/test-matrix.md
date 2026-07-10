# Suggest Constraint Acceptance Test Matrix

Every acceptance criterion has one primary executable owner. Phase 1 tests are
ignored red scaffolds; their ignores are removed as the listed implementation
phase lands. “DMLS L2” rows refer to end-to-end LSP sessions selected by the
package's `level2_` nextest tier. Responsible implementation paths in the table
are relative to the `darkmatter/` package area.

| AC | Primary test | Kind | Responsible implementation |
|---:|---|---|---|
| 1 | `suggest_phase1_eligible_scalar_and_array_forms_parse` | Unit | `lib/src/markdown/schemas/simplified/{grammar.rs,types.rs}` |
| 2 | `suggest_phase1_eligible_scalar_and_array_forms_parse` | Unit | `lib/src/markdown/schemas/simplified/grammar.rs` |
| 3 | `suggest_phase1_cardinality_is_per_complete_property_definition` | Unit | `lib/src/markdown/schemas/simplified/{mod.rs,grammar.rs}` |
| 4 | `suggest_phase1_conversion_preserves_string_interpretation_and_order` | Snapshot | `lib/src/markdown/schemas/simplified/{grammar.rs,convert.rs}` |
| 5 | `suggest_phase1_invalid_decimal_syntax_is_metadata` | Snapshot | `lib/src/markdown/schemas/simplified/{grammar.rs,convert.rs}` |
| 6 | `suggest_phase1_numeric_boundaries_follow_observable_json_round_trip` | Unit | `lib/src/markdown/schemas/simplified/grammar.rs` |
| 7 | `level2_suggest_phase1_standalone_ranges_and_completion` | DMLS L2 | `lib/src/markdown/schemas/simplified/grammar.rs`; `dmls/src/diagnostics/frontmatter.rs` |
| 8 | `suggest_phase1_duplicates_are_rejected_at_the_later_argument` | Unit | `lib/src/markdown/schemas/simplified/grammar.rs` |
| 9 | `suggest_phase1_conversion_preserves_string_interpretation_and_order` | Snapshot | `lib/src/markdown/schemas/simplified/convert.rs` |
| 10 | `suggest_phase1_metadata_does_not_restrict_document_values` | Unit | `lib/src/markdown/schemas/{validate.rs,mod.rs}` |
| 11 | `suggest_phase1_invalid_candidates_remain_loadable_and_lintable` | Unit | `lib/src/markdown/schemas/{resolve.rs,validate.rs}` |
| 12 | `suggest_phase1_invalid_candidates_remain_loadable_and_lintable` | Unit | `lib/src/markdown/schemas/simplified/{types.rs,mod.rs}` |
| 13 | `suggest_phase1_candidate_constraints_target_scalar_or_array_items` | Unit | `lib/src/markdown/schemas/simplified/convert.rs`; `lib/src/markdown/schemas/validate.rs` |
| 14 | `level2_suggest_phase1_inline_warning_has_exact_argument_range` | DMLS L2 | `dmls/src/diagnostics/{frontmatter.rs,codes.rs}` |
| 15 | `level2_suggest_phase1_standalone_ranges_and_completion` | DMLS L2 | `lib/src/markdown/schemas/resolve.rs`; `dmls/src/overlay` |
| 16 | `suggest_phase1_standalone_envelopes_resolve_consistently` | Unit | `lib/src/markdown/schemas/resolve.rs` |
| 17 | `level2_suggest_phase1_union_selection_and_raw_schema_exclusion` | DMLS L2 | `lib/src/markdown/schemas/resolve.rs`; `dmls/src/providers/frontmatter.rs` |
| 18 | `level2_suggest_phase1_completion_positions` | DMLS L2 | `dmls/src/providers/frontmatter.rs` |
| 19 | `level2_suggest_phase1_completion_positions` | DMLS L2 | `lib/src/markdown/schemas/completion.rs`; `dmls/src/providers/frontmatter.rs` |
| 20 | `level2_suggest_phase1_union_selection_and_raw_schema_exclusion` | DMLS L2 | `lib/src/markdown/schemas/completion.rs` |
| 21 | `level2_suggest_phase1_completion_positions` | DMLS L2 | `dmls/src/providers/frontmatter.rs` |
| 22 | `level2_suggest_phase1_union_selection_and_raw_schema_exclusion` | DMLS L2 | `lib/src/markdown/schemas/resolve.rs`; `dmls/src/providers/frontmatter.rs` |
| 23 | `suggest_phase1_numeric_boundaries_follow_observable_json_round_trip` | Unit | `lib/src/markdown/schemas/simplified/grammar.rs` |
| 24 | `suggest_phase1_fixture_newlines_are_explicit` and all `level2_suggest_phase1_*` sessions | Unit + DMLS L2 | Feature fixtures, source-span projection, and `FileReference` resolution |

The library scaffold lives in `darkmatter/lib/tests/suggest_constraint_phase1.rs`.
The LSP scaffold and its source fixtures live in
`darkmatter/dmls/tests/suggest_constraint_phase1.rs` and
`darkmatter/dmls/tests/fixtures/suggest_constraint/`.

## Phase 8 Validation Record

Every primary test listed above passes under `just test` (Level-1) and
`just test-l2` (DMLS L2 sessions). Catalog/docs consistency is covered by
`markdown::schemas::about::tests::suggest_descriptor_*` and
`markdown::schemas::simplified::serialize::tests::round_trip_*_suggest`.
No acceptance criterion is justified only by manual inspection.

| AC | Passing location |
|---:|---|
| 1 | `suggest_constraint_phase1::suggest_phase1_eligible_scalar_and_array_forms_parse` |
| 2 | `suggest_constraint_phase1::suggest_phase1_eligible_scalar_and_array_forms_parse` |
| 3 | `suggest_constraint_phase1::suggest_phase1_cardinality_is_per_complete_property_definition` |
| 4 | `suggest_constraint_phase1::suggest_phase1_conversion_preserves_string_interpretation_and_order` (snapshot) |
| 5 | `suggest_constraint_phase1::suggest_phase1_invalid_decimal_syntax_is_metadata` (snapshot) |
| 6 | `suggest_constraint_phase1::suggest_phase1_numeric_boundaries_follow_observable_json_round_trip` |
| 7 | `level2_suggest_phase1_standalone_ranges_and_completion` (DMLS L2) |
| 8 | `suggest_constraint_phase1::suggest_phase1_duplicates_are_rejected_at_the_later_argument` |
| 9 | `suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata` (snapshot) |
| 10 | `suggest_constraint_phase1::suggest_phase1_metadata_does_not_restrict_document_values` |
| 11 | `suggest_constraint_phase1::suggest_phase1_invalid_candidates_remain_loadable_and_lintable`; `suggest_constraint_phase3::suggestion_metadata_neither_restricts_validation_nor_blocks_composition` |
| 12 | `suggest_constraint_phase1::suggest_phase1_invalid_candidates_remain_loadable_and_lintable` |
| 13 | `suggest_constraint_phase1::suggest_phase1_candidate_constraints_target_scalar_or_array_items` |
| 14 | `level2_suggest_phase1_inline_warning_has_exact_argument_range` (DMLS L2) |
| 15 | `level2_suggest_phase1_standalone_ranges_and_completion`; `level2_suggest_phase5_malformed_tagged_envelope_error` (DMLS L2) |
| 16 | `suggest_constraint_phase4::mapping_envelopes_resolve_identically_with_origin_metadata`; `named_imports_share_pure_and_tagged_mapping_namespaces` |
| 17 | `level2_suggest_phase1_union_selection_and_raw_schema_exclusion` (DMLS L2); `suggest_constraint_phase4::raw_json_schema_remains_distinct_and_cannot_supply_named_imports` |
| 18 | `level2_suggest_phase1_completion_positions` (DMLS L2) |
| 19 | `level2_suggest_phase1_completion_positions` (DMLS L2) |
| 20 | `level2_suggest_phase1_union_selection_and_raw_schema_exclusion` (DMLS L2) |
| 21 | `level2_suggest_phase1_completion_positions` (DMLS L2) |
| 22 | `level2_suggest_phase1_union_selection_and_raw_schema_exclusion` (DMLS L2) |
| 23 | `suggest_constraint_phase1::suggest_phase1_numeric_boundaries_follow_observable_json_round_trip` |
| 24 | `suggest_constraint_phase1::suggest_phase1_fixture_newlines_are_explicit` + all `level2_suggest_phase1_*` sessions |
