# Phase 3 Requirement-to-Test Map

Phase 3 changes the public SimplifiedSchema vocabulary, grammar constraints,
canonical serialization, type descriptor catalog, and schema detection's
non-inference contract. Tests observe public parse products, serialized source,
catalog data, detected schemas, and CLI output rather than parser internals.

| Phase 3 behavior | Concrete test evidence |
|---|---|
| `type-definition` and `schema` are canonical primitive keywords, including ordinary `[]` postfix forms | `meta_schema_phase3.rs::semantic_type_keywords_round_trip_canonically` parses and serializes all four forms through the public API. `schemas_grammar_proptest.rs::round_trip_random_atoms` adds both enum variants to the 256-case generator/shrinker population. |
| Terminal `Name@file` syntax wins over the new primitive names | `meta_schema_phase3.rs::semantic_keyword_names_remain_valid_import_names` covers the original `schema@file` / `type-definition@file` inputs plus `[]` and constrained postfix variants, asserting the public `TypeExpr::Imported` product and canonical serialization. |
| Definition-level constraints are limited to `required`, `default(...)`, and `generated` | `meta_schema_phase3.rs::semantic_types_accept_only_definition_level_constraints` accepts both semantic keywords with every permitted constraint, including plain and quoted valid defaults, and rejects `min`, `max`, `pattern`, `suggest`, `eager`, `match`, `scheme`, `integer`, `not-empty`, `unique`, `min-keys`, `max-keys`, and `example`. It also proves ordinary array-level `min` / `max` / `unique` constraints remain available after `[]`. |
| Semantic defaults must themselves parse without I/O | `meta_schema_phase3.rs::semantic_defaults_use_the_passive_parser_authority` accepts a scalar type-expression default and a nonexistent local schema reference, then rejects an unknown type definition, native boolean default, and HTTP(S) schema defaults. |
| Public descriptors state carriers, permitted constraints, passive behavior, and DMLS meaning | `meta_schema_phase3.rs::semantic_type_descriptors_are_authoritative` checks both public descriptors field-by-field. Existing `about.rs::simplified_type_keyword_set_matches_descriptor_set` and `descriptor_keywords_are_parseable` enforce vocabulary/catalog parity. |
| `md schema about` exposes both new rows through the normal CLI path | `schema_about.rs::schema_about_lists_every_supported_type_keyword` explicitly requires `type-definition` and `schema` rows, while `schema_about_describes_semantic_meta_types` asserts their carrier, passive-parser, and DMLS-facing wording in real command output. |
| Downstream exhaustive `SimplifiedType` consumers remain compatible | `claudine-cli::commands::context::format::tests::semantic_schema_types_render_as_structural_types` covers the exact new scalar and array variants and asserts the report markup emitted for each. `claudine::composition::schema::classify::semantic_type_tests::semantic_schema_types_are_not_single_widget_values` covers both scalar and array forms and confirms parse-only artifacts do not enter interactive value collection. |
| `md schema detect` never infers either semantic type | `meta_schema_phase3.rs::schema_detection_preserves_carrier_only_inference` uses native and quoted definition-looking strings, a native mapping, and a sequence, then asserts the downstream public `SchemaShape` remains `string`, `object`, and `string[]` and serialized detection output contains neither new keyword. |
| Existing shipped schemas remain passively parseable | Existing `meta_schema_phase1.rs::shipped_schema_corpus_is_passively_classified_without_resolution` reads every shipped `docs/schemas/*.yaml` artifact and asserts its content-based classification. |
| A shipped catalog works end-to-end through normal invocation | The CLI tests above execute the real `md schema about` binary, whose rows are projected from the shipped compiled descriptor catalog. Existing `base_schema_end_to_end.rs::base_schema_file_parses_and_converts` remains the real shipped-schema regression path. |

Phase 3 does not persist semantic values. The feature-level repeated
write/read/write/read contract remains in
`meta_schema_phase1.rs::semantic_arrays_disambiguate_unions_and_survive_two_disk_round_trips`;
it stays ignored until Phase 4 supplies the semantic validators needed for the
round trip to exercise valid persisted values.
