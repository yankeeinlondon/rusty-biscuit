# Phase 5 Requirement-to-Test Map

Phase 5 changes the real shipped Darkmatter base schema. Coverage therefore
uses the public base-schema accessor and resolver, reads the shipped artifact as
a passive corpus member, and exercises document validation through the normal
library path.

| Phase 5 behavior | Concrete test evidence |
|---|---|
| `$schema` is nominally typed as `schema`, and the description limits raw JSON Schema to referenced files | `meta_schema_phase5.rs::shipped_base_schema_declares_schema_semantics_and_precise_raw_json_wording` reads the real shipped artifact and asserts its compiled public schema carries `x-darkmatter-schema`. |
| Inline, referenced SimplifiedSchema, root-union, and referenced raw-JSON declarations retain resolver acceptance | `meta_schema_phase5.rs::shipped_base_schema_preserves_all_resolver_accepted_declaration_forms` uses real temporary files and `resolve_schema_with_roots`, then asserts every accepted form produces a compiled JSON Schema object. |
| Malformed declarations fail baseline validation with a grammar-specific problem before a caller reaches resolver preparation | `meta_schema_phase5.rs::malformed_declarations_fail_baseline_validation_before_resolver_preparation` covers the original native `$schema: true` input, native number, empty and malformed root unions, quoted `"true"`, optional-null semantics, and missing `$schema`; it asserts the public validation report path/code/message and the downstream resolver result. |
| Every shipped schema artifact remains passively consumable | `meta_schema_phase1.rs::shipped_schema_corpus_is_passively_classified_without_resolution` and `meta_schema_phase4.rs::shipped_schema_artifacts_validate_through_semantic_keywords` cover all shipped `docs/schemas/*.yaml` files. |

No persistence behavior changes in Phase 5, so a read/write/read round trip is
not applicable. No terminal, browser, device, or OS-input behavior changes, so
the targeted coverage is Level 1; the complete Darkmatter-area `just test` and
`just lint` recipes are the broader gates.
