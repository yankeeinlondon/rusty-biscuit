# Phase 2 Requirement-to-Test Map

Phase 2 changes only passive SimplifiedSchema parsing and lexical source
projection. The tests below exercise public observable parser products and
authored byte spans; none assert private parser state.

| Phase 2 behavior | Concrete test evidence |
|---|---|
| A public semantic parser accepts exactly one `PropertyDef` | `schemas_source_projection.rs::property_definition_parser_matches_schema_property_matrix` covers the original plain, single-quoted, double-quoted, native mapping, and mixed union inputs, then rejects native boolean, number, null, an empty union, an invalid arm, and a malformed constraint. It compares the public result with the definition produced through `parse_yaml_schema`. |
| The source-aware property parser returns the identical semantic product | `schemas_source_projection.rs::source_aware_property_parser_preserves_semantics_and_authored_spans` compares both public entry points over quoted/unquoted CRLF input containing UTF-8, nested mappings, a property union, constraints and arguments, and a `Name@file` import. |
| Structural spans cover every required source role | The same source-aware test asserts mapping-key, complete-definition, atom, type-keyword, constraint, argument, import-name, import-reference, and union-arm spans against exact authored substrings and the caller-supplied document offset. It covers both postfix scalar constraints and native `$constraints` mapping syntax. |
| Passive schema declarations accept inline mappings, local references, and non-empty mixed root unions without I/O | `schemas_source_projection.rs::schema_declaration_parser_classifies_syntax_without_io` uses the original nonexistent `./schemas/does-not-exist.yaml` and `does-not-exist.yaml` references, an inline mapping, and a mixed union. It rejects HTTP(S), malformed file-reference syntax, native boolean/number/null, empty unions, and invalid arms. Nonexistent local paths succeeding proves classification is lexical. |
| Source-aware schema declarations add outer declaration and file-reference spans | `schemas_source_projection.rs::source_aware_schema_declaration_maps_outer_and_reference_spans` covers a quoted standalone reference and a CRLF mixed union, asserting exact declaration, union-arm, and file-reference substrings plus semantic parity. |
| One nesting limit governs string-form and YAML-native objects | `meta_schema_phase1.rs::native_mapping_depth_uses_the_shared_structured_limit` is unignored and extended to assert `MAX_INLINE_OBJECT_DEPTH` succeeds and one level beyond returns `SchemaError::Grammar`; `schemas_source_projection.rs::string_and_native_objects_share_the_depth_boundary` covers the equivalent string-form boundary and a mapping arm beneath a property union. |
| Existing suggestion projection remains compatible | Existing `meta_schema_phase1.rs::existing_source_projection_covers_union_quote_crlf_and_utf8_variants` and `suggest_constraint_phase2.rs::source_aware_parse_projects_multibyte_and_yaml_escape_ranges` remain regression gates. |
| Shipped artifacts remain passively parseable | Existing corpus test `meta_schema_phase1.rs::shipped_schema_corpus_is_passively_classified_without_resolution` reads every shipped `docs/schemas/*.yaml` artifact. |
| A real shipped artifact works through the normal public path | Existing end-to-end test `base_schema_end_to_end.rs::base_schema_file_parses_and_converts` loads the embedded shipped Darkmatter base schema through `darkmatter_base_schema()` and checks the downstream compiled JSON Schema. |

Phase 2 does not persist parser output, so a read/write/read product round trip
is not applicable to this phase. The feature-level repeated filesystem
round-trip remains covered by the Phase 1 contract
`semantic_arrays_disambiguate_unions_and_survive_two_disk_round_trips`, which
will be enabled in Phase 4 when the semantic validators exist.
