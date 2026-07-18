# Phase 7 Requirement-to-Test Map

Phase 7 changes only DMLS's observable schema-authoring intelligence. The
tests below exercise provider results and full LSP requests; they do not assert
private parser or cache mechanics.

| Phase 7 behavior | Concrete test evidence |
|---|---|
| `$schema` hover uses the nominal base-schema type and description | `lsp_session.rs::meta_schema_phase7_shipped_schema_provider_path` opens the real shipped `darkmatter.yaml` through the normal LSP path and hovers its `$schema` definition. The pre-existing `meta_schema_phase1_schema_hover_uses_nominal_type` regression is enabled as a focused guard. |
| Definition hover separates the semantic artifact from every denoted union arm | `providers::frontmatter::tests::meta_schema_hover_renders_every_denoted_arm_and_constraints` covers `foo: string(required)` and the exact mixed native union from the specification (`string` plus `{ bar: string }`). `lsp_session.rs::meta_schema_phase7_inline_hover_completion_and_diagnostics` asserts the rendered LSP Markdown and dependent hover range. |
| Parser-state completion works for inline values, constraints, scaffolds, semantic arrays, and standalone pure/tagged documents | `providers::frontmatter::tests::meta_schema_completion_catalog_is_descriptor_driven` proves every shipped type descriptor is offered and valid constraints are filtered by the selected type. `lsp_session.rs::meta_schema_phase7_inline_hover_completion_and_diagnostics` covers quoted/native `type-definition`, missing/present values, `type-definition[]` block items, outer `schema` scaffolds, and a malformed partial. `lsp_session.rs::meta_schema_phase7_standalone_pure_and_tagged_completion` covers pure `$schema` and tagged `kind: schema` → `types` through real completion requests. |
| Invalid definitions and outer declarations receive specialized, non-duplicated codes at the smallest authored range | `diagnostics::frontmatter::tests::meta_schema_diagnostics_replace_generic_keyword_failures` uses the exact malformed scalar `string(nope)`, a malformed native mapping arm, an empty union, a remote schema reference, and a missing local reference. It asserts code, source, range, and absence of a duplicate `dm.schema.constraint`. The inline LSP session asserts the same result after publication. |
| Last-good authoring data remains usable while current malformed text owns diagnostics | `lsp_session.rs::meta_schema_phase7_standalone_last_good_keeps_completion_and_current_diagnostic` performs valid-open → malformed-change → completion/hover/diagnostic requests and verifies the current error does not erase retained semantic assistance. |
| Typed activation is available to the deferred semantic-token family without fine-grained tokenization | `overlay::schema::tests::semantic_type_regions_project_existing_activation_state` covers frontmatter `type-definition`, `schema`, `type-definition[]`, and standalone definitions, asserting exact document byte spans and semantic kinds. |
| Analysis remains passive | `no_side_effects.rs::dsl_requests_spawn_no_processes_and_open_no_sockets` is extended with type definitions containing nonexistent imports/examples, local and remote schema references, interpolation-looking text, shell-looking text, and standalone schema requests. It continues to assert unchanged process/socket state after diagnostics, hover, completion, definition, document-link, and semantic-token requests. |
| Every shipped schema artifact remains passively classified | The existing `overlay::tests::shipped_schema_corpus_uses_content_classification` walks all shipped `darkmatter/docs/schemas/*.yaml` artifacts. `meta_schema_phase7_shipped_schema_provider_path` adds the end-to-end provider path using the real shipped base schema. |

The LSP fixture uses `lsp_server::Connection::memory()`, so these are Level 1
tests under `rust-testing`; no real terminal, browser, device, or external
service is involved. The package area's existing `just test-l2` remains a
regression gate for real-editor/terminal behavior.

Phase 7 does not persist values or change a serialization format. Overlay
last-good state is intentionally memory-only and is discarded on close, so a
filesystem read/write/read persistence round trip is not applicable. Repeated
open/change/request transitions exercise the relevant state round trip.
