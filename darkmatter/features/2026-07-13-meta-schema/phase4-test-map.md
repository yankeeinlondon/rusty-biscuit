# Phase 4 Requirement-to-Test Map

Phase 4 changes compiled JSON Schema, validator registration, coercion,
trigger matching, and compose persistence. The tests below observe public
conversion, validation, trigger, compose, and filesystem results rather than
custom-keyword implementation details.

| Phase 4 behavior | Concrete test evidence |
|---|---|
| Required semantic atoms compile to the portable carrier domain plus their distinguishing custom keyword; optional atoms retain the outer nullable wrapper | `meta_schema_phase4.rs::semantic_fragments_lower_with_carriers_keywords_and_nullable_wrappers` asserts the exact `type-definition` and `schema` fragments for required and optional properties. |
| Ordinary array lowering places the semantic fragment under `items` and preserves the collection/union boundary | `meta_schema_phase4.rs::semantic_array_lowering_validates_each_independent_item` asserts the exact compiled `items` fragments. `meta_schema_phase1.rs::semantic_arrays_disambiguate_unions_and_survive_two_disk_round_trips` validates flat arrays versus nested union-valued items and repeats write/read/write/read persistence. |
| The custom validators accept exactly the passive parser grammar and perform no reference resolution | `meta_schema_phase1.rs::type_definition_validation_matches_the_property_definition_matrix` and `schema_validation_is_syntax_only_and_rejects_remote_references` cover native and quoted scalar definitions, mappings, non-empty unions, missing local references, malformed constraints, invalid native scalars, empty unions, invalid arms, and HTTP(S) references. |
| Custom-keyword failures retain source position, `ConstraintViolation`, instance path, and a distinguishing keyword schema path | `meta_schema_phase4.rs::semantic_keyword_failures_are_structured_and_distinguishable` asserts all fields for both keyword families through `DarkmatterSchemas::validate`. |
| Validation and compose preserve native mapping/sequence carriers; semantic arrays remain arrays; no native-to-string coercion occurs | `meta_schema_phase4.rs::semantic_carriers_are_validation_and_compose_no_ops` asserts the caller document is unchanged by validation and the normal compose path writes back the same JSON shapes. Unit coverage in `coerce.rs::semantic_keyword_fragments_are_explicit_no_ops` guards both scalar and array fragments at the coercion recognizer boundary. |
| Trigger matching is pure and grammar-backed for both semantic types, including array items | `meta_schema_phase4.rs::semantic_types_match_triggers_by_passive_parse` covers valid scalar/mapping/union/reference values and invalid native scalar, malformed definition, remote reference, and invalid array item variants through the public trigger matcher. |
| Every shipped schema declaration remains accepted by the semantic validators through a normal invocation path | `meta_schema_phase4.rs::shipped_schema_artifacts_validate_through_semantic_keywords` reads all shipped `docs/schemas/*.yaml` declarations plus the tagged `env.yaml` definition catalog and validates their real values through `DarkmatterSchemas`. |
| All shipped schema artifacts remain passively classifiable without resolution | Existing `meta_schema_phase1.rs::shipped_schema_corpus_is_passively_classified_without_resolution` reads every shipped YAML artifact and asserts its expected classification. |

No L2/L3 test is required: Phase 4 has no terminal, browser, device, or OS-input
behavior. The full Darkmatter area `just test` and `just lint` recipes are the
broader gates required by the phase completion contract.
