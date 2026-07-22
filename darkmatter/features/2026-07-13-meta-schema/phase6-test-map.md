# Phase 6 Requirement-to-Test Map

Phase 6 changes DMLS overlay state and content-based schema-authoring
activation. The tests below assert public overlay products and full LSP
behavior; none inspect parser internals.

| Phase 6 behavior | Concrete test evidence |
|---|---|
| A standalone overlay carries the parsed `SimplifiedSchema` and structural `SchemaSourceMap` | `overlay::tests::standalone_pure_and_tagged_models_carry_schema_and_authored_spans` covers pure and tagged envelopes, explicit YAML document markers, mapping keys, type keywords, quoted values, CRLF, and UTF-8 using exact authored substrings. `meta_schema_phase6.rs::shipped_base_schema_exposes_structural_spans_across_anchors_and_aliases` verifies the public parser product on the real shipped base schema, including exact anchor-child and alias spans without invented alias-target tokens. |
| Frontmatter activation is semantic-type driven | `overlay::tests::frontmatter_meta_schema_activation_uses_effective_property_definitions` covers `type-definition` and `schema` atoms in native and quoted values, a union where the semantic arm is not first, the reserved `$schema` control value, and a missing optional value. It also proves an identically named property with a non-semantic definition does not activate. |
| Standalone activation is content-based, never filename-based | `overlay::tests::standalone_activation_is_content_based_not_path_based` covers pure/tagged content under unrelated filenames and ordinary YAML/raw JSON Schema under schema-looking filenames and `schemas/` directories. |
| Malformed standalone edits retain last-good semantics while current text owns errors | `overlay::tests::standalone_last_good_survives_malformed_yaml_and_shape_edits` seeds a valid model, applies both a hard YAML error and a valid-YAML/invalid-schema edit, asserts the same last-good schema/source map remains available, and asserts the current `SchemaError` replaces any stale validity claim. A fresh malformed document activates with an error but no invented semantic model. |
| Closing a document clears standalone retention | `overlay::tests::forget_clears_standalone_last_good_state` seeds, forgets, then applies malformed content and proves no stale model survives. |
| Every shipped schema artifact is classified passively | `overlay::tests::shipped_schema_corpus_uses_content_classification` walks `darkmatter/docs/schemas/*.yaml` and asserts valid pure/tagged models, the intentionally malformed tagged claim, and inactive ordinary/empty YAML without resolving imports or references. |
| The real shipped base schema follows the normal DMLS invocation path | `lsp_session.rs::meta_schema_phase6_shipped_schema_activation_and_current_error` opens the shipped `darkmatter.yaml` through a full in-memory LSP session, observes clean diagnostics, changes it to malformed claimed content, and observes the current schema diagnostic. |

The full LSP conversation is Level 1 under the `rust-testing` taxonomy because
it uses `lsp_server::Connection::memory()` and no terminal, browser, device, or
external service. Mislabeling it `level2_` would exclude it from `just test` and
would not add behavioral evidence. The area `just test-l2` gate remains a
regression gate for the existing real-editor/terminal tests.

No Phase 6 value is persisted by DMLS: last-good state is intentionally
in-memory and is discarded by `OverlayState::forget`. Therefore a filesystem
read/write/read persistence round trip is not applicable. Repeated state reads
and edit transitions are covered directly by the last-good and forget tests.
