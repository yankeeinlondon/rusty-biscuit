---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-18-schematic-fidelity-polish/spec.md"
plan: "schematic/features/2026-10-18-schematic-fidelity-polish/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/extension_reimport.rs
    - schematic/gen/tests/l1/upstream_hygiene.rs
    - schematic/definitions/tests/schema_fidelity_baseline.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - schematic/features/2026-10-18-schematic-fidelity-polish/contracts.md
skills_files_updated_during_phase_1: []
packages:
    - schematic-gen
    - schematic-definitions
    - schematic-define
    - schematic-schema
human_review: false
source_files_during_phase_2:
    - schematic/define/src/openapi/export/conversion.rs
    - schematic/define/src/openapi/export/components.rs
    - schematic/define/src/openapi/export/mod.rs
    - schematic/define/src/components.rs
    - schematic/define/src/registration.rs
    - schematic/define/src/lib.rs
    - schematic/define/src/builders.rs
    - schematic/define/src/bundle.rs
    - schematic/define/tests/schema_closure.rs
    - schematic/define/tests/openapi_tests.rs
    - schematic/gen/src/validation.rs
    - schematic/definitions/Cargo.toml
    - schematic/definitions/tests/schema_fidelity.rs
    - schematic/definitions/tests/schema_fidelity_baseline.rs
    - schematic/definitions/tests/schema_values.rs
    - Cargo.lock
    - schematic/openapi/anthropic.json
    - schematic/openapi/artificial_analysis.json
    - schematic/openapi/bitbucket.json
    - schematic/openapi/elevenlabs.json
    - schematic/openapi/emqx.json
    - schematic/openapi/eversolo.json
    - schematic/openapi/gitea.json
    - schematic/openapi/github.json
    - schematic/openapi/gitlab.json
    - schematic/openapi/huggingface.json
    - schematic/openapi/lmstudio.json
    - schematic/openapi/ollama.json
    - schematic/openapi/openai.json
    - schematic/openapi/samsung_smart_tv.json
    - schematic/openapi/unfolded_circle_core_rest.json
docs_updated_during_phase_2:
    - schematic/docs/io/export-openapi.md
    - schematic/docs/generation/validation.md
    - schematic/docs/typed-registration.md
    - schematic/docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/schematic-define/schematic/SKILL.md
    - .claude/skills/schematic-define/schematic/define.md
    - .claude/skills/schematic-define/schematic/openapi.md
source_files_during_phase_3:
    - schematic/define/src/openapi/export/headers.rs
    - schematic/define/src/openapi/export/mod.rs
    - schematic/define/src/openapi/export/paths.rs
    - schematic/define/src/openapi/mod.rs
    - schematic/gen/src/diagnostics.rs
    - schematic/gen/src/export/vocabulary.rs
    - schematic/gen/tests/l1/fixed_header_export.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/output_validation.rs
    - schematic/openapi/anthropic.json
    - schematic/openapi/github.json
    - schematic/openapi/gitlab.json
docs_updated_during_phase_3:
    - schematic/docs/io/export-openapi.md
    - schematic/docs/generation/validation.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/schematic-define/schematic/openapi.md
source_files_during_phase_4:
    - schematic/define/src/builders.rs
    - schematic/define/src/types.rs
    - schematic/define/src/openapi/mod.rs
    - schematic/define/src/openapi/extensions.rs
    - schematic/define/src/openapi/export/components.rs
    - schematic/define/src/openapi/export/mod.rs
    - schematic/define/src/openapi/export/parameters.rs
    - schematic/define/src/openapi/export/paths.rs
    - schematic/define/src/openapi/export/request_body.rs
    - schematic/define/src/openapi/export/responses.rs
    - schematic/define/src/openapi/export/security.rs
    - schematic/define/src/openapi/import.rs
    - schematic/define/src/openapi/import/auth.rs
    - schematic/define/src/openapi/import/builder.rs
    - schematic/define/src/openapi/import/extension.rs
    - schematic/define/src/openapi/import/reconcile.rs
    - schematic/define/src/openapi/import/query_paths.rs
    - schematic/define/src/openapi/import/mappings/mod.rs
    - schematic/define/src/openapi/import/mappings/parameters.rs
    - schematic/define/src/openapi/import/mappings/responses.rs
    - schematic/define/src/openapi/import/mappings/discriminator.rs
    - schematic/define/tests/builders.rs
    - schematic/define/tests/response_openapi.rs
    - schematic/define/tests/openapi_import_parameters.rs
    - schematic/define/tests/openapi_multipart_import.rs
    - schematic/define/tests/openapi_extension_matrix.rs
    - schematic/define/tests/openapi_standard_matrix.rs
    - schematic/define/tests/matrix_support/mod.rs
    - schematic/gen/src/export/openapi.rs
    - schematic/gen/src/import_pipeline.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/extension_reimport.rs
    - schematic/gen/tests/l1/extension_round_trip.rs
    - schematic/gen/tests/l1/import_overlay.rs
    - schematic/gen/tests/l1/vocabulary_exports.rs
    - schematic/openapi/anthropic.json
    - schematic/openapi/artificial_analysis.json
    - schematic/openapi/bitbucket.json
    - schematic/openapi/elevenlabs.json
    - schematic/openapi/emqx.json
    - schematic/openapi/eversolo.json
    - schematic/openapi/gitea.json
    - schematic/openapi/github.json
    - schematic/openapi/gitlab.json
    - schematic/openapi/huggingface.json
    - schematic/openapi/lmstudio.json
    - schematic/openapi/ollama.json
    - schematic/openapi/openai.json
    - schematic/openapi/samsung_smart_tv.json
    - schematic/openapi/unfolded_circle_core_rest.json
docs_updated_during_phase_4:
    - schematic/docs/io/openapi-extensions.md
    - schematic/docs/io/import-openapi.md
    - schematic/docs/io/export-openapi.md
    - schematic/docs/generation/generated-clients.md
    - schematic/docs/definition-builders.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/schematic-define/schematic/openapi.md
    - .claude/skills/schematic-define/schematic/gen.md
source_files_during_phase_5:
    - schematic/definitions/src/registry/inventory.rs
    - schematic/definitions/src/registry/lookup.rs
    - schematic/definitions/src/registry/mod.rs
    - schematic/definitions/src/lib.rs
    - schematic/definitions/tests/api_inventory.rs
    - schematic/definitions/tests/ws_conformance.rs
    - schematic/definitions/tests/ws_definition_json.rs
    - schematic/gen/src/pipeline.rs
    - schematic/gen/src/validation.rs
    - schematic/gen/src/output/mod.rs
    - schematic/gen/src/output/inventory.rs
    - schematic/gen/src/output/ws_modules.rs
    - schematic/gen/src/output/assemble/helpers.rs
    - schematic/gen/src/output/assemble/lib_rs.rs
    - schematic/gen/src/output/assemble/mod.rs
    - schematic/gen/src/output/assemble/prelude.rs
    - schematic/gen/src/ws_codegen/plan.rs
    - schematic/gen/tests/l1/api_inventory.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/openapi_strict_completeness.rs
    - schematic/gen/tests/l1/ws_codegen.rs
    - schematic/gen/tests/l1/ws_validation.rs
    - schematic/gen/tests/l1/partial_generation.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
docs_updated_during_phase_5:
    - schematic/docs/generation/workflow.md
    - schematic/docs/generation/generated-clients.md
    - schematic/definitions/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/schematic-define/schematic/definitions.md
    - .claude/skills/schematic-define/schematic/gen.md
source_files_during_phase_6:
    - schematic/definitions/src/naming.rs
    - schematic/definitions/src/lib.rs
    - schematic/definitions/tests/api_inventory.rs
    - schematic/definitions/tests/pagination_coverage/main.rs
    - schematic/define/src/types.rs
    - schematic/gen/src/inference.rs
    - schematic/gen/src/export/naming.rs
    - schematic/gen/src/export/mod.rs
    - schematic/gen/src/export/openapi.rs
    - schematic/gen/src/export/postman/mod.rs
    - schematic/gen/src/export/postman/tests.rs
    - schematic/gen/src/import_pipeline.rs
    - schematic/gen/src/lib.rs
    - schematic/gen/src/output/assemble/api.rs
    - schematic/gen/src/output/assemble/helpers.rs
    - schematic/gen/src/output/assemble/lib_rs.rs
    - schematic/gen/src/output/assemble/mod.rs
    - schematic/gen/src/output/assemble/prelude.rs
    - schematic/gen/src/output/mod.rs
    - schematic/gen/src/pipeline.rs
    - schematic/gen/src/validation.rs
    - schematic/gen/tests/l1/definition_bundles.rs
    - schematic/gen/tests/l1/e2e_generation.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/module_naming.rs
docs_updated_during_phase_6:
    - schematic/docs/generation/generated-clients.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/schematic-define/schematic/definitions.md
source_files_during_phase_7:
    - schematic/gen/src/cargo_gen.rs
    - schematic/gen/src/output/mod.rs
    - schematic/gen/src/output/assemble/lib_rs.rs
    - schematic/gen/src/output/assemble/prelude.rs
    - schematic/gen/src/validation.rs
    - schematic/gen/tests/common/fresh_output.rs
    - schematic/gen/tests/l1/e2e_generation.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/module_features.rs
    - schematic/schema/Cargo.toml
    - schematic/schema/src/lib.rs
    - schematic/schema/src/prelude.rs
docs_updated_during_phase_7:
    - schematic/docs/generation/generated-clients.md
    - schematic/docs/generation/partial-generation.md
    - schematic/docs/generation/validation.md
    - schematic/schema/README.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/schematic-define/schematic/gen.md
source_files_during_phase_8:
    - schematic/gen/src/codegen/endpoint_methods/mod.rs
    - schematic/gen/src/codegen/endpoint_methods/tests.rs
    - schematic/gen/src/codegen/module_docs.rs
    - schematic/gen/src/codegen/request_structs/mod.rs
    - schematic/gen/src/codegen/request_structs/tests.rs
    - schematic/gen/src/output/assemble/api.rs
    - schematic/gen/src/output/assemble/lib_rs.rs
    - schematic/gen/src/output/assemble/prelude.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/justfile
    - schematic/schema/src/anthropic.rs
    - schematic/schema/src/artificial_analysis.rs
    - schematic/schema/src/bitbucket/mod.rs
    - schematic/schema/src/bitbucket/requests.rs
    - schematic/schema/src/elevenlabs/mod.rs
    - schematic/schema/src/elevenlabs/requests.rs
    - schematic/schema/src/emqx/mod.rs
    - schematic/schema/src/emqx/requests.rs
    - schematic/schema/src/eversolo/mod.rs
    - schematic/schema/src/eversolo/requests.rs
    - schematic/schema/src/gitea/mod.rs
    - schematic/schema/src/gitea/requests.rs
    - schematic/schema/src/github/mod.rs
    - schematic/schema/src/github/requests.rs
    - schematic/schema/src/gitlab/mod.rs
    - schematic/schema/src/gitlab/requests.rs
    - schematic/schema/src/huggingface/mod.rs
    - schematic/schema/src/huggingface/requests.rs
    - schematic/schema/src/lib.rs
    - schematic/schema/src/lmstudio.rs
    - schematic/schema/src/ollama/mod.rs
    - schematic/schema/src/ollama/requests.rs
    - schematic/schema/src/openai/mod.rs
    - schematic/schema/src/openai/requests.rs
    - schematic/schema/src/prelude.rs
    - schematic/schema/src/samsung_smart_tv.rs
    - schematic/schema/src/unfolded_circle_core_rest.rs
    - schematic/gen/tests/l1/generated_docs.rs
docs_updated_during_phase_8:
    - schematic/docs/generation/generated-clients.md
    - schematic/docs/generation/workflow.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
    - .claude/skills/schematic-define/schematic/SKILL.md
    - .claude/skills/schematic-define/schematic/gen.md
source_files_during_phase_9:
    - schematic/gen/Cargo.toml
    - schematic/gen/src/diagnostics.rs
    - schematic/gen/src/export/auth.rs
    - schematic/gen/src/export/body.rs
    - schematic/gen/src/export/examples.rs
    - schematic/gen/src/export/examples/tests.rs
    - schematic/gen/src/export/mod.rs
    - schematic/gen/src/export/postman/auth.rs
    - schematic/gen/src/export/postman/collection.rs
    - schematic/gen/src/export/postman/environment.rs
    - schematic/gen/src/export/postman/examples.rs
    - schematic/gen/src/export/postman/graphql.rs
    - schematic/gen/src/export/postman/item.rs
    - schematic/gen/src/export/postman/mod.rs
    - schematic/gen/src/export/postman/request.rs
    - schematic/gen/src/export/postman/responses.rs
    - schematic/gen/src/export/postman/tests.rs
    - schematic/gen/src/export/postman/types.rs
    - schematic/gen/src/export/postman/variables.rs
    - schematic/gen/src/export/vocabulary.rs
    - schematic/gen/src/main.rs
    - schematic/gen/src/pipeline.rs
    - schematic/gen/src/validation.rs
    - schematic/gen/tests/fixtures/postman/golden/api_key_query_auth.json
    - schematic/gen/tests/fixtures/postman/golden/auth_variables_openai.json
    - schematic/gen/tests/fixtures/postman/golden/file_upload_elevenlabs.json
    - schematic/gen/tests/fixtures/postman/golden/grouped_module_ollama.json
    - schematic/gen/tests/fixtures/postman/golden/mixed_auth_emqx.json
    - schematic/gen/tests/fixtures/postman/golden/oauth2_grants.json
    - schematic/gen/tests/fixtures/postman/golden/path_query_params_github.json
    - schematic/gen/tests/l1/api_inventory.rs
    - schematic/gen/tests/l1/artifact_drift.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/module_naming.rs
    - schematic/gen/tests/l1/output_validation.rs
    - schematic/gen/tests/l1/pagination_validation.rs
    - schematic/gen/tests/l1/partial_generation.rs
    - schematic/gen/tests/l1/postman_golden.rs
    - schematic/gen/tests/l1/postman_runnable.rs
    - schematic/gen/tests/l1/response_adapters.rs
    - schematic/gen/tests/l1/vocabulary_exports.rs
    - schematic/justfile
    - schematic/postman/anthropic.postman_collection.json
    - schematic/postman/anthropic.postman_environment.json
    - schematic/postman/artificial_analysis.postman_collection.json
    - schematic/postman/artificial_analysis.postman_environment.json
    - schematic/postman/bitbucket.postman_collection.json
    - schematic/postman/bitbucket.postman_environment.json
    - schematic/postman/elevenlabs.postman_collection.json
    - schematic/postman/elevenlabs.postman_environment.json
    - schematic/postman/emqx.postman_collection.json
    - schematic/postman/emqx.postman_environment.json
    - schematic/postman/eversolo.postman_collection.json
    - schematic/postman/eversolo.postman_environment.json
    - schematic/postman/gitea.postman_collection.json
    - schematic/postman/gitea.postman_environment.json
    - schematic/postman/github.postman_collection.json
    - schematic/postman/github.postman_environment.json
    - schematic/postman/gitlab.postman_collection.json
    - schematic/postman/gitlab.postman_environment.json
    - schematic/postman/huggingface.postman_collection.json
    - schematic/postman/huggingface.postman_environment.json
    - schematic/postman/lmstudio.postman_collection.json
    - schematic/postman/lmstudio.postman_environment.json
    - schematic/postman/ollama.postman_collection.json
    - schematic/postman/ollama.postman_environment.json
    - schematic/postman/openai.postman_collection.json
    - schematic/postman/openai.postman_environment.json
    - schematic/postman/samsung_smart_tv.postman_collection.json
    - schematic/postman/samsung_smart_tv.postman_environment.json
    - schematic/postman/unfolded_circle_core_rest.postman_collection.json
    - schematic/postman/unfolded_circle_core_rest.postman_environment.json
docs_updated_during_phase_9:
    - schematic/docs/dependencies.md
    - schematic/docs/generation/partial-generation.md
    - schematic/docs/generation/validation.md
    - schematic/docs/generation/workflow.md
    - schematic/docs/graphql.md
    - schematic/docs/io/export-postman.md
    - schematic/docs/typed-registration.md
docs_created_during_phase_9: []
skills_files_updated_during_phase_9:
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/schematic-define/schematic/gen.md
    - .claude/skills/schematic-define/schematic/responses.md
source_files_during_phase_10:
    - schematic/gen/src/asyncapi_import.rs
    - schematic/gen/src/asyncapi_import/extensions.rs
    - schematic/gen/src/asyncapi_import/reader.rs
    - schematic/gen/src/asyncapi_import/v3.rs
    - schematic/gen/src/commands.rs
    - schematic/gen/src/diagnostics.rs
    - schematic/gen/src/export/asyncapi/mod.rs
    - schematic/gen/src/export/mod.rs
    - schematic/gen/src/main.rs
    - schematic/gen/tests/common/asyncapi.rs
    - schematic/gen/tests/common/mod.rs
    - schematic/gen/tests/l1/artifact_drift.rs
    - schematic/gen/tests/l1/asyncapi_export.rs
    - schematic/gen/tests/l1/asyncapi_import_matrix.rs
    - schematic/gen/tests/l1/asyncapi_persist.rs
    - schematic/gen/tests/l1/asyncapi_round_trip.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/fixtures/asyncapi/spec-json-schemas/3.0.0.json
    - schematic/gen/tests/fixtures/asyncapi/spec-json-schemas/LICENSE
    - schematic/gen/tests/fixtures/asyncapi/spec-json-schemas/NOTICE
    - schematic/justfile
    - schematic/asyncapi/elevenlabs_ws.json
    - schematic/asyncapi/samsung_smart_tv_remote_ws.json
    - schematic/asyncapi/unfolded_circle_core_ws.json
    - schematic/asyncapi/unfolded_circle_dock_ws.json
    - schematic/asyncapi/unfolded_circle_integration_ws.json
docs_updated_during_phase_10:
    - schematic/docs/dependencies.md
    - schematic/docs/generation/validation.md
    - schematic/docs/generation/workflow.md
    - schematic/docs/io/export-openapi.md
    - schematic/docs/websocket-definitions.md
docs_created_during_phase_10:
    - schematic/docs/io/asyncapi.md
    - schematic/gen/tests/fixtures/asyncapi/spec-json-schemas/PROVENANCE.md
skills_files_updated_during_phase_10:
    - .claude/skills/schematic-define/schematic/gen.md
    - .claude/skills/schematic-define/schematic/websocket.md
source_files_during_phase_11:
    - schematic/define/src/lib.rs
    - schematic/gen/src/validation.rs
docs_updated_during_phase_11:
    - schematic/define/README.md
docs_created_during_phase_11: []
skills_files_updated_during_phase_11:
    - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_12:
    - schematic/define/src/openapi/export/conversion.rs
    - schematic/define/src/openapi/export/parameters.rs
    - schematic/define/src/openapi/import.rs
    - schematic/define/src/openapi/import/mappings/responses.rs
    - schematic/define/src/openapi/import/reconcile.rs
    - schematic/definitions/src/naming.rs
    - schematic/definitions/src/registry/mod.rs
    - schematic/gen/src/asyncapi_import.rs
    - schematic/gen/src/export/asyncapi/mod.rs
    - schematic/gen/src/export/postman/auth.rs
    - schematic/gen/src/export/postman/collection.rs
    - schematic/gen/src/export/postman/item.rs
    - schematic/gen/src/export/postman/mod.rs
    - schematic/gen/src/export/postman/request.rs
    - schematic/gen/src/export/postman/responses.rs
    - schematic/gen/src/export/vocabulary.rs
    - schematic/gen/src/validation.rs
docs_updated_during_phase_12:
    - schematic/README.md
    - schematic/gen/README.md
    - schematic/definitions/README.md
    - schematic/docs/generation/generated-clients.md
    - schematic/docs/generation/partial-generation.md
    - schematic/docs/generation/workflow.md
    - schematic/docs/io/crates-openapi.md
    - schematic/docs/io/export-openapi.md
    - schematic/docs/io/openapi-extensions.md
    - schematic/docs/typed-registration.md
    - .claude/commands/create-schematic-api.md
docs_created_during_phase_12: []
skills_files_updated_during_phase_12:
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/schematic-define/schematic/definitions.md
    - .claude/skills/schematic-define/schematic/gen.md
source_files_during_phase_13:
    - schematic/gen/tests/l1/asyncapi_persist.rs
docs_updated_during_phase_13: []
docs_created_during_phase_13: []
skills_files_updated_during_phase_13: []
source_code:
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/extension_reimport.rs
    - schematic/gen/tests/l1/upstream_hygiene.rs
    - schematic/definitions/tests/schema_fidelity_baseline.rs
    - schematic/define/src/openapi/export/conversion.rs
    - schematic/define/src/openapi/export/components.rs
    - schematic/define/src/openapi/export/mod.rs
    - schematic/define/src/components.rs
    - schematic/define/src/registration.rs
    - schematic/define/src/lib.rs
    - schematic/define/src/builders.rs
    - schematic/define/src/bundle.rs
    - schematic/define/tests/schema_closure.rs
    - schematic/define/tests/openapi_tests.rs
    - schematic/gen/src/validation.rs
    - schematic/definitions/Cargo.toml
    - schematic/definitions/tests/schema_fidelity.rs
    - schematic/definitions/tests/schema_values.rs
    - Cargo.lock
    - schematic/openapi/anthropic.json
    - schematic/openapi/artificial_analysis.json
    - schematic/openapi/bitbucket.json
    - schematic/openapi/elevenlabs.json
    - schematic/openapi/emqx.json
    - schematic/openapi/eversolo.json
    - schematic/openapi/gitea.json
    - schematic/openapi/github.json
    - schematic/openapi/gitlab.json
    - schematic/openapi/huggingface.json
    - schematic/openapi/lmstudio.json
    - schematic/openapi/ollama.json
    - schematic/openapi/openai.json
    - schematic/openapi/samsung_smart_tv.json
    - schematic/openapi/unfolded_circle_core_rest.json
    - schematic/define/src/openapi/export/headers.rs
    - schematic/define/src/openapi/export/paths.rs
    - schematic/define/src/openapi/mod.rs
    - schematic/gen/src/diagnostics.rs
    - schematic/gen/src/export/vocabulary.rs
    - schematic/gen/tests/l1/fixed_header_export.rs
    - schematic/gen/tests/l1/output_validation.rs
    - schematic/define/src/types.rs
    - schematic/define/src/openapi/extensions.rs
    - schematic/define/src/openapi/export/parameters.rs
    - schematic/define/src/openapi/export/request_body.rs
    - schematic/define/src/openapi/export/responses.rs
    - schematic/define/src/openapi/export/security.rs
    - schematic/define/src/openapi/import.rs
    - schematic/define/src/openapi/import/auth.rs
    - schematic/define/src/openapi/import/builder.rs
    - schematic/define/src/openapi/import/extension.rs
    - schematic/define/src/openapi/import/reconcile.rs
    - schematic/define/src/openapi/import/query_paths.rs
    - schematic/define/src/openapi/import/mappings/mod.rs
    - schematic/define/src/openapi/import/mappings/parameters.rs
    - schematic/define/src/openapi/import/mappings/responses.rs
    - schematic/define/src/openapi/import/mappings/discriminator.rs
    - schematic/define/tests/builders.rs
    - schematic/define/tests/response_openapi.rs
    - schematic/define/tests/openapi_import_parameters.rs
    - schematic/define/tests/openapi_multipart_import.rs
    - schematic/define/tests/openapi_extension_matrix.rs
    - schematic/define/tests/openapi_standard_matrix.rs
    - schematic/define/tests/matrix_support/mod.rs
    - schematic/gen/src/export/openapi.rs
    - schematic/gen/src/import_pipeline.rs
    - schematic/gen/tests/l1/extension_round_trip.rs
    - schematic/gen/tests/l1/import_overlay.rs
    - schematic/gen/tests/l1/vocabulary_exports.rs
    - schematic/definitions/src/registry/inventory.rs
    - schematic/definitions/src/registry/lookup.rs
    - schematic/definitions/src/registry/mod.rs
    - schematic/definitions/src/lib.rs
    - schematic/definitions/tests/api_inventory.rs
    - schematic/definitions/tests/ws_conformance.rs
    - schematic/definitions/tests/ws_definition_json.rs
    - schematic/gen/src/pipeline.rs
    - schematic/gen/src/output/mod.rs
    - schematic/gen/src/output/inventory.rs
    - schematic/gen/src/output/ws_modules.rs
    - schematic/gen/src/output/assemble/helpers.rs
    - schematic/gen/src/output/assemble/lib_rs.rs
    - schematic/gen/src/output/assemble/mod.rs
    - schematic/gen/src/output/assemble/prelude.rs
    - schematic/gen/src/ws_codegen/plan.rs
    - schematic/gen/tests/l1/api_inventory.rs
    - schematic/gen/tests/l1/openapi_strict_completeness.rs
    - schematic/gen/tests/l1/ws_codegen.rs
    - schematic/gen/tests/l1/ws_validation.rs
    - schematic/gen/tests/l1/partial_generation.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/definitions/src/naming.rs
    - schematic/definitions/tests/pagination_coverage/main.rs
    - schematic/gen/src/inference.rs
    - schematic/gen/src/export/naming.rs
    - schematic/gen/src/export/mod.rs
    - schematic/gen/src/export/postman/mod.rs
    - schematic/gen/src/export/postman/tests.rs
    - schematic/gen/src/lib.rs
    - schematic/gen/src/output/assemble/api.rs
    - schematic/gen/tests/l1/definition_bundles.rs
    - schematic/gen/tests/l1/e2e_generation.rs
    - schematic/gen/tests/l1/module_naming.rs
    - schematic/gen/src/cargo_gen.rs
    - schematic/gen/tests/common/fresh_output.rs
    - schematic/gen/tests/l1/module_features.rs
    - schematic/schema/Cargo.toml
    - schematic/schema/src/lib.rs
    - schematic/schema/src/prelude.rs
    - schematic/gen/src/codegen/endpoint_methods/mod.rs
    - schematic/gen/src/codegen/endpoint_methods/tests.rs
    - schematic/gen/src/codegen/module_docs.rs
    - schematic/gen/src/codegen/request_structs/mod.rs
    - schematic/gen/src/codegen/request_structs/tests.rs
    - schematic/justfile
    - schematic/schema/src/anthropic.rs
    - schematic/schema/src/artificial_analysis.rs
    - schematic/schema/src/bitbucket/mod.rs
    - schematic/schema/src/bitbucket/requests.rs
    - schematic/schema/src/elevenlabs/mod.rs
    - schematic/schema/src/elevenlabs/requests.rs
    - schematic/schema/src/emqx/mod.rs
    - schematic/schema/src/emqx/requests.rs
    - schematic/schema/src/eversolo/mod.rs
    - schematic/schema/src/eversolo/requests.rs
    - schematic/schema/src/gitea/mod.rs
    - schematic/schema/src/gitea/requests.rs
    - schematic/schema/src/github/mod.rs
    - schematic/schema/src/github/requests.rs
    - schematic/schema/src/gitlab/mod.rs
    - schematic/schema/src/gitlab/requests.rs
    - schematic/schema/src/huggingface/mod.rs
    - schematic/schema/src/huggingface/requests.rs
    - schematic/schema/src/lmstudio.rs
    - schematic/schema/src/ollama/mod.rs
    - schematic/schema/src/ollama/requests.rs
    - schematic/schema/src/openai/mod.rs
    - schematic/schema/src/openai/requests.rs
    - schematic/schema/src/samsung_smart_tv.rs
    - schematic/schema/src/unfolded_circle_core_rest.rs
    - schematic/gen/tests/l1/generated_docs.rs
    - schematic/gen/Cargo.toml
    - schematic/gen/src/export/auth.rs
    - schematic/gen/src/export/body.rs
    - schematic/gen/src/export/examples.rs
    - schematic/gen/src/export/examples/tests.rs
    - schematic/gen/src/export/postman/auth.rs
    - schematic/gen/src/export/postman/collection.rs
    - schematic/gen/src/export/postman/environment.rs
    - schematic/gen/src/export/postman/examples.rs
    - schematic/gen/src/export/postman/graphql.rs
    - schematic/gen/src/export/postman/item.rs
    - schematic/gen/src/export/postman/request.rs
    - schematic/gen/src/export/postman/responses.rs
    - schematic/gen/src/export/postman/types.rs
    - schematic/gen/src/export/postman/variables.rs
    - schematic/gen/src/main.rs
    - schematic/gen/tests/fixtures/postman/golden/api_key_query_auth.json
    - schematic/gen/tests/fixtures/postman/golden/auth_variables_openai.json
    - schematic/gen/tests/fixtures/postman/golden/file_upload_elevenlabs.json
    - schematic/gen/tests/fixtures/postman/golden/grouped_module_ollama.json
    - schematic/gen/tests/fixtures/postman/golden/mixed_auth_emqx.json
    - schematic/gen/tests/fixtures/postman/golden/oauth2_grants.json
    - schematic/gen/tests/fixtures/postman/golden/path_query_params_github.json
    - schematic/gen/tests/l1/artifact_drift.rs
    - schematic/gen/tests/l1/pagination_validation.rs
    - schematic/gen/tests/l1/postman_golden.rs
    - schematic/gen/tests/l1/postman_runnable.rs
    - schematic/gen/tests/l1/response_adapters.rs
    - schematic/postman/anthropic.postman_collection.json
    - schematic/postman/anthropic.postman_environment.json
    - schematic/postman/artificial_analysis.postman_collection.json
    - schematic/postman/artificial_analysis.postman_environment.json
    - schematic/postman/bitbucket.postman_collection.json
    - schematic/postman/bitbucket.postman_environment.json
    - schematic/postman/elevenlabs.postman_collection.json
    - schematic/postman/elevenlabs.postman_environment.json
    - schematic/postman/emqx.postman_collection.json
    - schematic/postman/emqx.postman_environment.json
    - schematic/postman/eversolo.postman_collection.json
    - schematic/postman/eversolo.postman_environment.json
    - schematic/postman/gitea.postman_collection.json
    - schematic/postman/gitea.postman_environment.json
    - schematic/postman/github.postman_collection.json
    - schematic/postman/github.postman_environment.json
    - schematic/postman/gitlab.postman_collection.json
    - schematic/postman/gitlab.postman_environment.json
    - schematic/postman/huggingface.postman_collection.json
    - schematic/postman/huggingface.postman_environment.json
    - schematic/postman/lmstudio.postman_collection.json
    - schematic/postman/lmstudio.postman_environment.json
    - schematic/postman/ollama.postman_collection.json
    - schematic/postman/ollama.postman_environment.json
    - schematic/postman/openai.postman_collection.json
    - schematic/postman/openai.postman_environment.json
    - schematic/postman/samsung_smart_tv.postman_collection.json
    - schematic/postman/samsung_smart_tv.postman_environment.json
    - schematic/postman/unfolded_circle_core_rest.postman_collection.json
    - schematic/postman/unfolded_circle_core_rest.postman_environment.json
    - schematic/gen/src/asyncapi_import.rs
    - schematic/gen/src/asyncapi_import/extensions.rs
    - schematic/gen/src/asyncapi_import/reader.rs
    - schematic/gen/src/asyncapi_import/v3.rs
    - schematic/gen/src/commands.rs
    - schematic/gen/src/export/asyncapi/mod.rs
    - schematic/gen/tests/common/asyncapi.rs
    - schematic/gen/tests/common/mod.rs
    - schematic/gen/tests/l1/asyncapi_export.rs
    - schematic/gen/tests/l1/asyncapi_import_matrix.rs
    - schematic/gen/tests/l1/asyncapi_persist.rs
    - schematic/gen/tests/l1/asyncapi_round_trip.rs
    - schematic/gen/tests/fixtures/asyncapi/spec-json-schemas/3.0.0.json
    - schematic/gen/tests/fixtures/asyncapi/spec-json-schemas/LICENSE
    - schematic/gen/tests/fixtures/asyncapi/spec-json-schemas/NOTICE
    - schematic/asyncapi/elevenlabs_ws.json
    - schematic/asyncapi/samsung_smart_tv_remote_ws.json
    - schematic/asyncapi/unfolded_circle_core_ws.json
    - schematic/asyncapi/unfolded_circle_dock_ws.json
    - schematic/asyncapi/unfolded_circle_integration_ws.json
documentation:
    - schematic/features/2026-10-18-schematic-fidelity-polish/contracts.md
    - schematic/docs/io/export-openapi.md
    - schematic/docs/generation/validation.md
    - schematic/docs/typed-registration.md
    - schematic/docs/dependencies.md
    - .claude/skills/schematic-define/schematic/SKILL.md
    - .claude/skills/schematic-define/schematic/define.md
    - .claude/skills/schematic-define/schematic/openapi.md
    - schematic/docs/io/openapi-extensions.md
    - schematic/docs/io/import-openapi.md
    - schematic/docs/generation/generated-clients.md
    - schematic/docs/definition-builders.md
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/schematic-define/schematic/gen.md
    - schematic/docs/generation/workflow.md
    - schematic/definitions/README.md
    - .claude/skills/schematic-define/schematic/definitions.md
    - schematic/docs/generation/partial-generation.md
    - schematic/schema/README.md
    - schematic/docs/graphql.md
    - schematic/docs/io/export-postman.md
    - .claude/skills/schematic-define/schematic/responses.md
    - schematic/docs/websocket-definitions.md
    - schematic/docs/io/asyncapi.md
    - schematic/gen/tests/fixtures/asyncapi/spec-json-schemas/PROVENANCE.md
    - .claude/skills/schematic-define/schematic/websocket.md
    - schematic/define/README.md
    - schematic/README.md
    - schematic/gen/README.md
    - schematic/docs/io/crates-openapi.md
    - .claude/commands/create-schematic-api.md
completed_phase: 13
implemented: true
---

# Implementation Log for 2026-10-18-schematic-fidelity-polish (13 phases)

## Phase 1

### Dependency gate (Wave 1)

Started 2026-10-23 on host macOS (Darwin 27.2.0), worktree
`feat-better-schematics`, branch `feat/better-schematics`, HEAD `09de6ee35`.

Every prerequisite reports `implemented: true` and its latest review is
`ready: true`, `blocked: false`, with no open findings:

| Prerequisite | Lifecycle | Latest review |
| --- | --- | --- |
| 2026-10-11-schematic-generation-integrity | `_completed` | review-3, ready |
| 2026-10-12-schematic-runtime-envelope | `_completed` | review-3, ready |
| 2026-10-13-schematic-pagination | `_completed` | review-2, ready |
| 2026-10-14-schematic-definition-api | active | review-3, ready |
| 2026-10-15-schematic-definition-vocabulary | active | review-1, ready |
| 2026-10-16-schematic-response-model-graphql | active | review-2, ready |
| 2026-10-17-schematic-websocket | active | review-2, ready |

Existing user changes: at session start the working tree carried 24
modified files plus untracked review/log files, all belonging to the
`2026-10-17-schematic-websocket` review-repair cycle (websocket hints,
Unfolded Circle WS definitions, `ws_codegen`, generated `*_ws.rs` schema
files, `ws_*` tests). This phase does not edit any of those files. Because of
them, the uncommitted tree is **not** exactly this phase's change set, so the
affected-scope skip rule cannot apply; the full gates run regardless.

Execution authority: the plan records the author's confirmation of the spec's
pre-authorizations (2026-10-21). Phase 1 makes no commits, provider calls,
lifecycle moves, dependent edits, or CI changes.

Prerequisite contract refresh: done against real code, recorded in
[contracts.md](./contracts.md) ("Prerequisite contracts verified"). Typed send,
serializable definitions and snapshots, response v2 plus GraphQL, WebSocket
selectors/fallback/role/`HeartbeatHints`, partial generation, `MutationPlan`
staged outputs (OpenAPI import only, D14), isolated fresh-output compile checks,
and `OpenApiDiagnostic` all exist. Departures from the plan's assumptions are
contracts.md D1–D25; the ones later phases must act on are summarized under
"Adopted defaults and plan discrepancies" below.

### Wave 2: Fidelity baseline (R5, R6, R14)

The bridge is **`schematic/define/src/openapi/export/conversion.rs`**, not the
`definitions/src/registry/conversion.rs` that the spec and plan name. Phase 2
works there.

Method: each of the 18 registries exported through
`schematic_define::openapi::export`; every exported position (component root,
property, array item, composition member, map value) paired with its
schemars source and classified. *Deliberate* = untyped output from `true`,
`{}`, or annotation-only source. *Lost* = untyped output whose source carried
`type`, `enum`, `const`, composition, `$ref`, `properties`, `items`,
`additionalProperties` or a type array. *Typed loss* = typed output that
dropped `enum`, `const`, `additionalProperties`, `items`, composition,
`required`, `nullable`, `format`, a bound or `default` (bracketed: losses
beyond `default`).

| API | props | untyped | deliberate | lost | typed loss | components lost |
| --- | --- | --- | --- | --- | --- | --- |
| Anthropic | 71 | 28 | 1 | 27 | 9 (8) | 13 of 29 |
| ArtificialAnalysisData | 61 | 7 | 2 | 5 | 0 | 0 of 13 |
| ArtificialAnalysisCritPt | 61 | 7 | 2 | 5 | 0 | 0 of 13 |
| Bitbucket | 219 | 110 | 0 | 110 | 98 (40) | 0 of 36 |
| OpenAI | 5122 | 1403 | 1175 | 228 | 0 | 121 of 1373 |
| ElevenLabs | 233 | 67 | 0 | 67 | 26 (5) | 1 of 65 |
| Gitea | 122 | 30 | 1 | 29 | 90 (0) | 0 of 19 |
| GitHub | 183 | 53 | 1 | 52 | 70 (16) | 0 of 25 |
| GitLab | 273 | 61 | 0 | 61 | 139 (38) | 0 of 27 |
| HuggingFaceHub | 227 | 87 | 2 | 85 | 34 (21) | 0 of 44 |
| LmStudio | 70 | 12 | 1 | 11 | 7 (7) | 3 of 26 |
| OllamaNative | 195 | 29 | 1 | 28 | 30 (30) | 4 of 47 |
| OllamaOpenAI | 195 | 29 | 1 | 28 | 30 (30) | 4 of 47 |
| EmqxBasic | 148 | 13 | 8 | 5 | 42 (42) | 0 of 40 |
| EmqxBearer | 148 | 13 | 8 | 5 | 42 (42) | 0 of 40 |
| Eversolo | 54 | 16 | 1 | 15 | 2 (0) | 0 of 13 |
| SamsungSmartTv | 21 | 1 | 0 | 1 | 0 | 0 of 3 |
| UnfoldedCircleCoreRest | 30 | 4 | 0 | 4 | 5 (5) | 4 of 12 |
| **Sum (18 APIs)** | **7433** | **1970** | **1204** | **766** | **624 (284)** | **150 of 1872** |

- Shared module groups (Emqx, Ollama, ArtificialAnalysis) double-count their
  types; counted once: 7029 props, 1921 untyped, 1193 deliberate, 728 lost,
  552 typed losses (212 beyond default), 146 of 1772 components lost.
- Array items / members / map values: 481 positions; 69 deliberate (all
  OpenAI `Vec<serde_json::Value>`), none lost.
- Loss causes: `anyOf` (mostly `Option<Struct>`) 531, nullable type array
  331, `allOf` 36, `oneOf` 18, `additionalProperties` 18, `items` 12.
  Typed losses dropped: `default` 413, `minimum` 273 (mostly unsigned
  `minimum: 0`), `enum` 128, `additionalProperties` 54, `maximum` 18.
- Sample lost paths: Bitbucket `Repository.mainbranch/.owner/.project`
  (anyOf), `BranchInfo.name` (nullable); HuggingFaceHub
  `CreateRepoBody.hardware/.sdk/.type`; ElevenLabs
  `CreateSpeechBody.voice_settings`; Anthropic `CreateMessageBody.system`,
  `.tool_choice`; GitHub `RepositoryInfo.owner` (allOf); lost component roots
  such as Anthropic `ContentBlock`, `MessageRole` (oneOf) and 121 OpenAI anyOf
  roots. The full per-API list prints from
  `schema_fidelity_baseline` with `--no-capture`.
- Compared with July: Bitbucket 109/216 → 110/219 untyped; HuggingFaceHub
  99/227 → 87/227; ElevenLabs 73/233 → 67/233. Overall 26.5% untyped
  (24.5% without OpenAI); July's ~30% roughly holds.
- Deliberate free-form outside OpenAI (29): Anthropic `Tool.input_schema`;
  ArtificialAnalysis `CritPtSubmission.generation_config`,
  `CritPtEvaluateBody.batch_metadata`; Gitea/GitHub
  `IssueSummary.pull_request`; HuggingFace `MetricResult.value`,
  `DatasetConfig.data_files`; LmStudio `InvalidToolCallMetadata.arguments`;
  Ollama `ShowModelResponse.model_info`; Emqx `LoginResponse.license`,
  `PublishBody.user_properties`, `CreateRuleBody.metadata`, `RuleAction.args`,
  `TestRuleBody.context`, `RuleInfo.metadata`, `TestRuleResponse.data`,
  `AlarmInfo.details`; Eversolo `InputOutputListResponse.outputInfo` (counted
  per API, hence 29). OpenAI's 1175 deliberate properties are declared
  `serde_json::Value` in `definitions/src/openai/types.rs` itself (the
  import lost them, not the bridge); Phase 2 cannot and need not change them.
- Spike observation: committed artifacts contain no `nullable` at all
  (e.g. `anthropic.json` `CreateMessageBody.system` is only a description).
- Artifacts: 15 `schematic/openapi/*.json` + 15
  `schematic/postman/*.postman_collection.json`, one per module group.

Retained helper: `schematic/definitions/tests/schema_fidelity_baseline.rs`
(4 tests, 0.59 s) with reusable `classify`/`walk`/`scan`/`typed_losses`/
`source_constraints` helpers, and only durable assertions: every registry
exports, the walk reaches properties, `serde_json::Value` is deliberate and
`String` kept, a nullable string/enum exported untyped is lost, and a typed
output dropping `enum`/`format` is a typed loss. Phase 2 grows it into the
R5 zero-lost scan.

**R14 / ledger row 14: not reproducible, guarded.** Every
`skip_serializing_if` field in `define/src/openapi/extensions.rs` already has
`serde(default)` or is an `Option`; `extensions.rs` is unchanged. The importer
does not read `x-schematic` yet (contracts.md D2), so each test checks that the
exported extension decodes back through the public `SchematicDocExtension` /
`SchematicOpExtension` types **and** that the file re-imports through both
`OpenApiImport::build` and `run_import` with method, path, body presence,
response kind and empty headers preserved. Tests in
`schematic/gen/tests/l1/extension_reimport.rs`:

- `absent_optional_collections_reimport` (absent env mapping, headers,
  errors, rate limit, retry guidance, request-id header, cross-origin;
  endpoints with no body, headers or errors; an empty multipart form)
- `explicitly_empty_env_mapping_reimports`
- `explicitly_empty_extension_arrays_decode_like_absent_ones` (edits the
  exported file to add `[]` for headers/errors/success, which the writer never
  emits)

Observed but out of row 14: a multipart body re-imports as `request: None`
(spec Problem item 5, Phase 4); the test limits body-presence comparison to
JSON and no-body endpoints with a comment saying why.

### Wave 2: Consumer inventory (R33)

Re-derived at `09de6ee35`. The spec's starting inventory has moved: there are
**4 runtime REST lists (not 6)**, **2 runtime WebSocket lists R33 never
names**, and the 16-name strict-completeness table no longer exists.

Runtime REST lists:

| # | Location | Symbol | Holds | Consumers |
| --- | --- | --- | --- | --- |
| R1 | `definitions/src/registry/lookup.rs:50-66` | `MODULES: &[Assemble]` | 15 module-group constructors → 18 APIs, catalog order | `all_apis()` (`lib.rs:245-248`) → `apis_by_module()` (`lib.rs:225-241`), `pipeline::resolve_all_apis()` (`gen/src/pipeline.rs:82-84`) |
| R2 | `definitions/src/registry/lookup.rs:23-45` | `registry_key_for` | RestApi name → kebab key (18 arms + lowercase fallback) | baseline keys, `registry_invariants.rs:27` |
| R3 | `gen/src/pipeline.rs:25` | `AVAILABLE_APIS` | 18 selection names + `all` (eversolo after elevenlabs) | help/error text, `main.rs:302`, `pipeline.rs:76` |
| R4 | `gen/src/pipeline.rs:33-80` | `resolve_api` | selection name → `define_*_api()` (eversolo after emqx-bearer); exact kebab keys, no aliases | `run_generate`, `run_bundle`, `run_validate`, tests |

Supporting lists that a single registration must also absorb or derive:
`lib.rs:164-204` (`pub mod` + 23 `pub use define_*`),
`gen/src/output/assemble/helpers.rs:32-37` `definitions_module_path` (one
entry, `unfolded_circle_core_rest`), and `gen/src/inference.rs:38`
`VARIANT_SUFFIXES` (6 suffixes, the Rust-side module-path resolver, versus
the lowercase `resolve_module_name` at `gen/src/export/naming.rs:22-27` used
by OpenAPI and Postman — the R36 pair).

Runtime WebSocket lists: W1 `gen/src/output/ws_modules.rs:21-52`
`WS_DEFINITION_MODULES` (5; feeds lib/prelude assembly, inventory, reserved
export names at `validation.rs:661`); W2 `ws_modules.rs:64-123`
`shipped_ws_module` (5; feeds `shipped_ws_apis()` → validation at
`pipeline.rs:357` and module generation at `output/mod.rs:263`).

Test tables: `definitions/src/registry/mod.rs:56-75` (18, guarded set-equal
with `all_apis()`); `definitions/tests/api_policy.rs:10-29` (18);
`gen/tests/l1/definition_baseline.rs:290-399` (18 tests + key table, guarded)
plus subset tables at `:28,32,36`; `gen/tests/fixtures/definition-baseline/`
(18 files); `definitions/tests/pagination_coverage/` (18 rows, guarded; 6
paged counts at `main.rs:394-401`); WS tables `gen/tests/l1/ws_codegen.rs:17-40`,
`gen/src/ws_codegen/plan.rs:1929-1942`, `definitions/tests/ws_conformance.rs:18-26`,
`definitions/tests/ws_definition_json.rs:318-329` (5 each, **unguarded**).
`gen/tests/l1/openapi_strict_completeness.rs:136-171` iterates `all_apis()`
and `apis_by_module()`: the 16-of-18 skip is already gone, but no test yet
compares the covered set with `apis_by_module()` as R35 asks.
Legacy-filename lists: `artifact_drift.rs:154-210`,
`e2e_generation.rs:656-720`, `level2/terminal_capture.rs:290-310`.

Canonical inventory (18 REST + 5 WebSocket = 23):

| Selection key | `RestApi.name` / WS name | Constructor | Protocol | Module group |
| --- | --- | --- | --- | --- |
| anthropic | Anthropic | `define_anthropic_api` | REST | anthropic |
| artificial-analysis-data | ArtificialAnalysisData | `define_artificial_analysis_data_api` | REST | artificial_analysis |
| artificial-analysis-critpt | ArtificialAnalysisCritPt | `define_artificial_analysis_critpt_api` | REST | artificial_analysis |
| bitbucket | Bitbucket | `define_bitbucket_api` | REST | bitbucket |
| openai | OpenAI | `define_openai_api` | REST | openai |
| elevenlabs | ElevenLabs | `define_elevenlabs_rest_api` | REST | elevenlabs |
| gitea | Gitea | `define_gitea_api` | REST | gitea |
| github | GitHub | `define_github_api` | REST | github |
| gitlab | GitLab | `define_gitlab_api` | REST | gitlab |
| huggingface | HuggingFaceHub | `define_huggingface_hub_api` | REST | huggingface (explicit `module_path`; suffix inference would differ) |
| lmstudio | LmStudio | `define_lmstudio_api` | REST | lmstudio |
| ollama-native | OllamaNative | `define_ollama_native_api` | REST | ollama |
| ollama-openai | OllamaOpenAI | `define_ollama_openai_api` | REST | ollama |
| emqx-basic | EmqxBasic | `define_emqx_basic_api` | REST | emqx |
| emqx-bearer | EmqxBearer | `define_emqx_bearer_api` | REST | emqx |
| eversolo | Eversolo | `define_eversolo_api` | REST | eversolo |
| samsung-smart-tv | SamsungSmartTv | `define_samsung_smart_tv_api` | REST | samsung_smart_tv |
| unfolded-circle-core-rest | UnfoldedCircleCoreRest | `define_unfolded_circle_core_rest_api` | REST | unfolded_circle_core_rest |
| — | ElevenLabsTTS | `define_elevenlabs_websocket_api` | WS | elevenlabs_ws |
| — | UnfoldedCircleCoreWs | `define_unfolded_circle_core_ws_api` | WS | unfolded_circle_core_ws |
| — | UnfoldedCircleDockWs | `define_unfolded_circle_dock_ws_api` | WS | unfolded_circle_dock_ws |
| — | UnfoldedCircleIntegrationWs | `define_unfolded_circle_integration_ws_api` | WS | unfolded_circle_integration_ws |
| — | SamsungSmartTvRemote | `define_samsung_smart_tv_remote_ws_api` | WS | samsung_smart_tv_remote_ws |

Every REST API has a registry constructor (its module's `module_group()`,
15 functions; member order `[data, critpt]`, `[native, openai]`,
`[basic, bearer]`). WebSocket APIs have no registry constructor and no
selection name. Selection ordering differs between R1, R3 and R4; P10-e's
"preserve ordering" adopts the **catalog order (R1)**, the order every
derived consumer already sees (adopted default; source: plan P10-e/R34
"preserve existing ordering", which names no single list).

Endpoint literals: **503** (`Endpoint::(new|get|post|put|patch|delete|head|options)\(`
in `definitions/src/**`, excluding test blocks and doc comments), equal to the
503 operations in the committed OpenAPI files; **539** counted per API (shared
EMQX endpoints twice), equal to the summed baselines and Postman requests.
The spec's ~519 matches neither. WebSocket endpoint literals: 8.

Dependents (exactly 8; all `path` dependencies with no features, no
`default-features`, not optional; `schematic-schema` declares no features):

| Package | Manifest | Crate | Area `test` recipe |
| --- | --- | --- | --- |
| homelab | `homelab/lib/Cargo.toml:10` | schematic-schema | `homelab/justfile` `_test_all` over 7 packages + `test-frontend` |
| eversolo-integration | `homelab/eversolo-integration/Cargo.toml:13` | schematic-schema | same |
| arcam-amp-integration | `homelab/arcam-amp-integration/Cargo.toml:12` | schematic-schema | same |
| unfolded-integration-helper | `homelab/unfolded-integration-helper/Cargo.toml:10` | schematic-schema | same |
| sony-receiver-integration | `homelab/sony-receiver-integration/Cargo.toml:12` | schematic-schema | same |
| sniff | `sniff/lib/Cargo.toml:44` | schematic-schema | `sniff/justfile` `_test_all "sniff --features remote; sniff-cli --features test-fixtures"` |
| biscuit-speaks | `biscuit-speaks/lib/Cargo.toml:27` | schematic-schema | `_test_all "biscuit-speaks --features playa; biscuit-speaks-cli"` |
| model-citizen | `model-citizen/lib/Cargo.toml:39` | **schematic-definitions** | `_test_all "model-citizen model-citizen-cli"` |

Modules used (lower bound): homelab — eversolo, samsung_smart_tv,
samsung_smart_tv_remote_ws, ws_shared; the four integrations —
unfolded_circle_integration_ws (+ eversolo, ws_shared); sniff — github,
gitlab, gitea, bitbucket; biscuit-speaks — elevenlabs, prelude;
model-citizen — lmstudio, ollama definitions. This is the input for P10-d's
default-features-on rule: every dependent relies on default features.

Artifact filename consumers (before resolver-driven renames): writers
`gen/src/export/openapi.rs:72-240`, `gen/src/export/postman/mod.rs:61-115`,
pipeline batch publish and owned-file filters (`gen/src/pipeline.rs` ~460-530),
`check_export_filename` (`validation.rs:293`), `output/inventory.rs`; tests
`artifact_drift.rs` (drift, legacy names, hard-coded
`artificial_analysis` files, filename set = `apis_by_module` keys),
`e2e_generation.rs:656-720`, `level2/terminal_capture.rs:290-310`,
`definition_bundles.rs:49-56` and `vocabulary_exports.rs:572,587`
(`include_str!` of the github artifacts), `postman_artifact_validation.rs`,
`postman_var_consistency.rs`; justfile `generate*`, `check-drift`,
`migration-check`; `.gitnexusignore:8`. CI workflows reference no artifact
path. No AsyncAPI output directory exists yet. Stale docs found:
`docs/generation/generated-clients.md:302` (13 selection names) and
`docs/io/export-postman.md:20-35` (`huggingfacehub` name, missing
`artificial_analysis`) — fixed by the docs phases, not here.

### Wave 2: Upstream hygiene (R43, R47, R50, R51, R52)

- **R50 holds.** `schematic/gen/schematic` does not exist;
  `git ls-files schematic/gen/schematic | wc -l` → `0`. New guard
  `upstream_hygiene::stale_nested_schematic_tree_is_absent` fails if
  `env!("CARGO_MANIFEST_DIR")/schematic` exists.
- **R43 holds.** `rg -n '(Strategy|Pagination|PaginationStrategy|MoreSignal|Collection|Paginated)::(github|gitlab|bitbucket|gitea)\b' -g '*.rs' ..`
  → no match. `rg -n -i 'fn [a-z_]*(github|gitlab|bitbucket|gitea)[a-z_]*\s*[<(]' define/src`
  → only `graphql.rs:65` `AddressRule::github()` (a GraphQL endpoint-address
  rule, outside R43's pagination scope; noted for review) and a private test
  fn at `metadata.rs:359`. `rg -n -i preset definitions/src define/src` →
  header-builder test names and the generic Relay preset
  (`define/src/pagination/relay.rs:1`) only; no shared vendor preset layer.
  Each API owns a private strategy function (`github/mod.rs:69`,
  `gitlab/mod.rs:65,74`, `gitea/mod.rs:65`, `bitbucket/mod.rs:69,77`,
  `elevenlabs/mod.rs:87,96,104`, `openai_pagination.rs`).
- **R52 holds.** Hard error at `define/src/builders.rs:283-284` (Endpoint)
  and `:470-471` (RestApi); generator severity `Severity::Error` at
  `gen/src/validation.rs:69` applied at `:226`. Existing passing tests:
  `construction_enforcement::query_bearing_paths_fail_endpoint_and_destination_api_build`,
  `definition_bundles::construction_runs_the_builder_validation`,
  `validation::tests::query_in_path_severity_is_an_error`,
  `validation::tests::query_in_path_is_one_error_per_output`, and the
  registered-endpoint scan
  `output_validation::live_definitions_have_no_embedded_queries_or_validation_gaps`
  (`EMBEDDED_QUERY_ENDPOINTS` is empty). No new test needed.
  `rg -n '"/[^"\s]*\?[^"]*"' definitions/src` → no path literal with `?`.
- **R51 measured; symptom differs from the spec.** EmqxBasic and EmqxBearer
  share `emqx` (`definitions/src/emqx/mod.rs:164,221`); `ollama` is a second
  combined module. The combined module **does** name both APIs
  (`schema/src/emqx/mod.rs:3-5` Basic intro, `:7` `---`, `:11-13` Bearer
  intro). What is lost: it has 11 `//!` lines and no `##` sections, while a
  single-API module (e.g. github, 61 lines) gets `## Authentication`,
  `## Features`, `## Examples` (and `## Rate limit`) from `ModuleDocBuilder`
  (`gen/src/codegen/module_docs.rs:58-77`). `build_combined_module_docs`
  (`gen/src/output/assemble/api.rs:176-208`) emits only name, docs URL,
  description, rate-limit and retry guidance, so each member's auth (Basic
  `EMQX_API_KEY`/`EMQX_API_SECRET` vs Bearer `EMQX_TOKEN`), endpoint list
  (Bearer-only `Login`/`Logout`) and example are dropped. Cosmetic: three
  empty `//!` lines after `---` (`emqx/mod.rs:8-10`, same in ollama).
  Retained fixture for Phase 8: `apis_by_module()["emqx"]` (and
  `["ollama"]`) through `assemble_combined_api_module` or
  `gen/tests/common/fresh_output.rs`; nearest existing test
  `grouped_module_docs_keep_each_apis_rate_limit_and_retry_guidance`
  (`api.rs:318`).
- **R47 open (D20 part 4).** `find sniff -type d -name '2026-10-09-consistent-use-schematic*'`
  → `sniff/fixes/2026-10-09-consistent-use-schematic` (active), frontmatter
  `status: draft-spec`, `reviewed: false`, `implemented: false`,
  `human_review: true`. `grep -rn FocusedProviderClient sniff/` → 58 matches
  in 11 files: the fix spec (5), `lib/CHANGELOG.md` (1), `lib/README.md` (3),
  `lib/tests/l1/{pr_for_branch 3, focused_provider 23, branch_head 2}`,
  `lib/src/remote/{types 1, focused 8 (struct at :63), mod 1, check_status 1, blocking 10}`.
  Reported separately; does not block this feature. sniff was not edited.

### Wave 3: Contract freeze

Frozen in [contracts.md](./contracts.md): prerequisite contracts, the
field-by-field `x-schematic` mapping (document and operation level), the
four input matrices expanded to real columns, reader smells, the AsyncAPI
model fields, GraphQL treatment, and discrepancies D1–D25 with adopted
resolutions. Highlights:

- `x-schematic.version` is integer 1 at document level only; absent = 1.
  Members decode independently from the raw value (no whole-struct derive).
  Unknown top-level member → INFO; nested members follow the upstream type
  (WARN where it denies unknown fields).
- Subset additions the plan missed: `retry_guidance` (both levels),
  `env_auth`, `secret_names`, per-parameter `secret`/`rust_name`,
  `path_template` (keeps `{+name}`; HuggingFace has 16), operation
  `env_mapping`, and `auth.<scheme>.{bearer, value_prefix}` for R54.
- Standard-only: `deprecated`, scopes, auth method/location/name; root
  `security` derived from the effective auth policy.
- Null exception: the v1 writer itself emits `null` at five positions
  (some `env_mapping` slots, form field `description`, `min`/`max`), so null
  is O there and W elsewhere (D19).
- `oauth_scopes` is already gone; `SchematicSchemaExtension` and
  `AuthExtension` are present and unused, deleted after the two auth details
  land (D23, R15).
- AsyncAPI: selectors and fallback on the component message (selector
  `value` is a string; empty value valid, empty `field` WARN); role and
  heartbeat (`{interval_secs, timeout_secs?}`, 0 → WARN) on the channel; role
  never reverses 3.0 directions.

### Wave 3: Validator spike (P10-k)

Run once on macOS in a temporary `gen/tests/l1/validator_spike.rs` (4 tests,
~0.8 s), since deleted with its `mod` line; scratch downloads lived in
`/tmp/schematic-spike-asyncapi/` and `/tmp/schematic-spike-oas/`.

1. **Registered schemas: usable with options.** Only recursive request
   family: Anthropic `CreateMessageBody`/`CountTokensBody`
   (`Message → ContentBlock → ToolResultContent → ContentBlock`). Schemas come
   from `TypeRegistration::schema()` (`define/src/registration.rs:202`,
   `SchemaSettings::openapi3()`, schemars 1.2.1) with `$schema` =
   the OAS 3.0 schema URI, refs under `#/components/schemas/`, and mixed
   style (root properties use `nullable`; nested components keep 2020-12
   `type: [T, "null"]`, `const`, `anyOf[{$ref},{type:null}]`). Default draft
   detection fails (`Unknown meta-schema`). Required: force
   `jsonschema::options().with_draft(Draft::Draft202012).offline()` (Draft7
   also works; **Draft4 falsely rejects** `const`-based `oneOf`), and rewrite
   `{type: T, nullable: true}` to `type: [T, "null"]` before compiling, since
   no draft understands `nullable`. Local recursive resolution works; any
   unresolvable `$ref` (remote, relative, missing pointer) fails at build
   time with a named error. jsonschema 0.55.0 has no features enabled
   anywhere in the workspace.
2. **AsyncAPI 3.0 full-schema validation: feasible offline.** Source
   `https://raw.githubusercontent.com/asyncapi/spec-json-schemas/469c2b2bf5ed88e0cdf1e48975b93a87c710d229/schemas/3.0.0.json`
   (last commit touching the file; 397,862 bytes; sha256
   `1786a007ac00344a8f529e5a6fc5db786b0be6757e0b94a8b95a8a6a52b7fe9a`;
   draft-07; self-contained bundle of 105 `$id`-keyed definitions;
   Apache-2.0 with a NOTICE file to carry when vendored). Compiles with
   `.offline()` in ~80 ms (debug), validates in <0.2 ms, accepts a minimal
   3.0 document and rejects a 2.x `publish` channel and `action: "publish"`.
   Adopted for the AsyncAPI phase: vendor it as a frozen fixture with its
   LICENSE/NOTICE attribution.
3. **Nullable representations for OpenAPI 3.0.3 (Phase 2).** Per OAS 3.0.3
   (`nullable` only applies with an explicit `type`; `$ref` siblings are
   ignored) and the OAI "Clarify nullable" proposal:
   - nullable `$ref`:
     `{"anyOf":[{"$ref":"#/components/schemas/X"},{"type":"object","nullable":true,"enum":[null]}]}`
     (`{nullable:true, allOf:[{$ref}]}` rejects null);
   - nullable string enum: `{"type":"string","nullable":true,"enum":["a","b",null]}`
     (an enum without `null` rejects null).
   Both checked with jsonschema against null, valid and invalid values.

### Adopted defaults and plan discrepancies

- adopted default: catalog order (`registry::MODULES`) is the canonical API
  ordering for the single registration (source: plan P10-e, spec R34).
- adopted default: D12 — no WebSocket payload type derives `JsonSchema` and
  `WebSocketApi` holds no schemas; adding derives is a definitions change
  beyond API-level constructor calls, so it is left unmade and the AsyncAPI
  exporter emits one unconstrained `components.schemas.<type_name>` per
  payload, described by its Rust type (source: plan Phase 1 "Execution
  authority"). R25's "typed message schemas" is an open gap for review 1.
- adopted default: P10-g is satisfied upstream — `2026-10-12-schematic-endpoint-methods`
  already generates one method plus `{Endpoint}Call` per REST endpoint
  (e.g. `schema/src/github/requests.rs:3220` `GitHub::get_repository`). No
  wrapper accepted gap is recorded and nothing is benchmarked; R39/R45 docs
  describe the endpoint methods as the primary call path and
  `client.send(Request::new(..))` as the typed alternative. The R45 "typed
  convenience methods (D53)" ACCEPTED GAP is superseded and recorded so at
  Phase 13 (source: plan "Resolve discrepancies in this plan and log them").
  That spec's author ruling (option A, no per-API feature flags) concerned
  its own scope; P10-d's module features still apply here.
- adopted default: pagination stays outside the v1 `x-schematic` subset and
  is listed as an R10/P10-l difference; adding it later is additive (D7).
- adopted default: AsyncAPI correlation hints, `auth_flow`, lifecycle
  messages, runtime hints and WS env metadata stay outside the subset per
  P10-a and are listed per API as R27 differences; `responds_with` maps to
  the standard 3.0 `operation.reply` (D11).
- adopted default: `env_auth` and operation `env_mapping` hold environment
  variable names, not auth fields, so they join the extension (D5).
- Plan corrections: the bridge lives at
  `define/src/openapi/export/conversion.rs`; the strict option is
  `OpenApiImportOptions::strict` and currently fails on INFO too (D3); the
  R33 inventory is 4 + 2 runtime lists, not 6; endpoint literals are 503
  (539 per API), not ~519; the strict-completeness skip is already gone.
- Recipe note for later phases: `just _test <pkg> -E 'test(x)'` breaks
  (unquoted args; a quoted `-E` is OR'd with the tier filter and runs every
  L1 test). Narrow scoped runs with `BISCUIT_TEST_NARROW='test(x)' just _test <pkg>`,
  or the positional substring filter (`just _test schematic-gen validator_spike`).

### Review agenda (Phase 1)

1. D12 WebSocket payload schemas: allow `JsonSchema` derives in
   `schematic-definitions`, or accept the placeholder-schema gap.
2. D1 / P10-g superseded by endpoint methods.
3. D5 `env_auth` / operation `env_mapping` classification under R54.
4. D7 pagination outside the v1 extension subset.
5. D11 WebSocket metadata outside the AsyncAPI subset (four of five APIs
   re-import with differences).
6. R47 sniff migration open (D20 part 4).
7. R51's real symptom (lost auth/features/examples sections, not lost API
   names).
8. `AddressRule::github()` is a public vendor-named constructor outside R43's
   pagination scope.

### Verification

Requirement-to-test mapping for this phase (all L1, F1 in-process at the
public crate APIs; no CLI, filesystem-persistence, or terminal boundary is
changed by Phase 1):

| Requirement | Test or evidence |
| --- | --- |
| R5/R6 baseline (measurement, no behavior change) | `schematic-definitions::schema_fidelity_baseline` (4 tests: registry walk plus paired deliberate/lost/typed-loss classifier fixtures) |
| R14 / ledger row 14 | `schematic-gen::l1 extension_reimport::{absent_optional_collections_reimport, explicitly_empty_env_mapping_reimports, explicitly_empty_extension_arrays_decode_like_absent_ones}` (public export → decode → `OpenApiImport::build` and `run_import`) |
| R50 | `schematic-gen::l1 upstream_hygiene::stale_nested_schematic_tree_is_absent` |
| R52 | existing `output_validation::live_definitions_have_no_embedded_queries_or_validation_gaps` plus the four `?`-path error tests cited above |
| R43, R47, R51, R33 | recorded greps and measurements above (the requirements name log evidence) |

The Input Robustness Matrix does not apply yet: Phase 1 changes no reader;
its frozen columns are in contracts.md for Phases 4 and 10.

Gates (run once, after all subagent work was merged and no subagent was
running), from `schematic/`:

- `just lint` → exit 0 (all five packages clean).
- `just test` → exit 0: L1 2778 run, 2778 passed, 20 skipped; fresh-output
  14 passed; compile-fail 1 passed. The four new binaries/modules ran
  (`schema_fidelity_baseline` 4, `extension_reimport` 3, `upstream_hygiene` 1).
- `just check-tier-coverage schematic` → 0 stranded.
- The affected-scope skip was not used: the working tree also carries the
  websocket review-repair changes, so it is not this phase's change set.

Scoped development runs used `BISCUIT_TEST_NARROW=...` with `just _test`. An
earlier mid-phase `just lint` failed on `clippy::unnecessary_sort_by` in the
temporary `validator_spike.rs`; that file was deleted before the gate.

Dependents: not run. Phase 1 changed only test targets (two new test
files, one new definitions test binary, and the `gen` L1 `main.rs` module
list). No library or binary code any of the eight dependents compile
changed. Cross-OS check: not run, for the same reason. The one new path
assertion joins a literal onto `CARGO_MANIFEST_DIR` and uses only
`Path::exists`.

Skills: none updated. Phase 1 changed no behavior, and the bridge location
correction is recorded in the plan and here for Phase 2.

Phase 1 status: checkpoint met. Prerequisites are verified, the public
reproduction is retained, the baseline and inventory are logged, the
mapping and matrices are frozen in contracts.md, the spike decisions are
recorded, and the baseline `just test`/`just lint` are green. R47 (sniff) is
reported open separately, as the plan requires.

## Phase 2

Started 2026-10-23 on macOS (Darwin 27.2.0), worktree `feat-better-schematics`,
HEAD `fdf26a265`, clean tree (so the uncommitted tree is exactly this phase).
No subagents were used: the bridge, the closure check, and their tests share
two files' worth of design, and splitting them would have raced on
`define/src/openapi/export/` and the regenerated artifacts.

### Wave 4: Bridge (R1–R4)

`define/src/openapi/export/conversion.rs` is rewritten as a JSON→JSON
transform (schemars JSON Schema → OAS 3.0.3 JSON) followed by
`serde_json::from_value::<openapiv3::Schema>`; `openapiv3`'s deserializer
keeps mixed keywords by falling back to `AnySchema`, so no keyword is dropped
by the typed model. Handles both dialects found by a keyword survey of all 18
registries (only OAS-expressible keywords plus `const`, type arrays, and
`{type: null}` / `{nullable: true, enum: [null]}` members; every `$ref` is
`#/components/schemas/…`):

- type arrays → `type` + `nullable`, several non-null types → `anyOf` of
  single-type members carrying their own keywords (never `oneOf`);
- nullable `$ref`/composition → `anyOf: [X, {type: object, nullable: true, enum: [null]}]`
  (the Phase 1 validator-checked form); nullable enums keep `null` in `enum`;
- `const` → one-value `enum`; untyped `enum` gets its shared JSON type;
- compositions, `properties`, `items`, `additionalProperties` (schema or
  boolean), `not` convert recursively; constraints beside a `$ref` move into
  `allOf: [{$ref}]` alongside it;
- bounds, lengths, pattern, format, required, default, multipleOf kept;
  numeric `exclusiveMinimum`/`exclusiveMaximum` → bound + boolean;
- any other keyword, tuple `items`, non-string `type` entries, non-array
  `enum`, or a non-object schema fails export with
  `OpenApiError::UnsupportedFeature { feature, context: Some(json-pointer) }`
  (no new error type; none of the 18 registries hits it).

`map_components` became fallible to propagate that error.

Decisions and deviations:

- **deviation (R3 "preserve defaults"), `"default": null` only:** dropped.
  `SchemaData::default` is an `Option<Value>`, so `openapiv3` reads
  `"default": null` as absent. Restoring it after deserialization was
  implemented and reverted: it broke
  `response_adapters::shipped_bitbucket_get_source_exports_json_and_a_fallback`,
  which requires every artifact to equal its own `openapiv3` re-parse (the
  importer's parser). The field's `nullable` plus its absence from `required`
  convey the same meaning; the R5 scan accepts a dropped null default only
  when the output is nullable. All non-null defaults are kept.
- **intended representation change:** a `$ref` with no siblings is now
  exported bare instead of `allOf: [{$ref}]` (465 such wrappers in HEAD's
  artifacts, 0 now; 1530 bare refs). OAS 3.0 ignores `$ref` siblings, so the
  wrapper is kept only when annotations or constraints need carrying.
  `define/tests/openapi_tests.rs` pinned the old wrapper and was updated to
  `/properties/status/$ref`; the importer accepts both forms (all
  `schematic-gen` re-import tests pass).
- adopted default: the plan's "bridge in `definitions/src/registry/conversion.rs`"
  is the Phase 1 correction — the bridge is in `schematic-define`, so the
  shape-level bridge unit tests live there and the R1–R4 value/export tests in
  `schematic-definitions` (source: Phase 1 log, "Plan corrections").

### Wave 5: Closure (R7) and loss audit (R5, R6)

**R7.** New `RestApi::validate_schema_closure` (`define/src/components.rs`):
walks every component schema once (so recursive models terminate without
following refs), schema-aware — values under `const`/`default`/`enum`/
`example`/`examples` are data, but a *property named* `default` or `enum` is
still a schema (a latent bug in the old bundle walker). Only
`#/components/schemas/{name}` resolves, with `%XX` and `~1`/`~0` decoded;
everything else (URLs, other files, deep pointers, `#/$defs/`, non-string or
empty `$ref`) is reported with `missing: None` and never fetched. New
`DefinitionError::UnresolvedSchemaReferences { api, references: Vec<SchemaReference> }`
lists *every* failure with `parent`, JSON-pointer `path`, `reference`, and
`missing` name.

Where it runs: API build (`builders::validate_api`), bundle construction and
load (the bundle's own `check_refs` walker was deleted and its result mapped
to the unchanged `BundleError::SchemaReference` contract, same precedence),
gen validation (`check_schema_references`, so every output reports
`invalid-schema-reference` and nothing is written), and `openapi::export`.
Spec 4a's typed registration already extracts nested definitions, so for
built APIs this is a **backstop**; all 18 shipped registries close.

- **adopted decision, not in `validate_references`:** putting the closure
  inside `validate_references` made `OpenApiImport::build` fail on two
  shipped import corpora — the Unfolded Circle import keeps deep pointer refs
  (`#/components/schemas/DeviceButtonLayout/properties/grid`, even
  `#/paths/~1cfg~1profile/…`) and an OpenAI import-matrix case leaves a
  dangling `BetaItemResource`. R7 requires blocking generation and export,
  not import, and importer diagnostics are Phase 4's input-robustness work,
  so `validate_references` keeps its endpoint scope and the closure is a
  separate check. Those imports still fail at export/generation, as they did
  before (the old export closure rejected the deep refs as missing names).
  **Phase 4 should turn these into importer ERROR diagnostics.**

**R53.** Containers already export inline (spec 4a): the new export test
asserts a `Vec<Child>` response is `{type: array, items: {$ref: Child}}` and no
container component exists; the 18-registry test asserts no `Array_of`/
`Map_of`/`Nullable_`/`Vec<`/`Option<`/`HashMap<`/`BTreeMap<`/`IndexMap<` key.
**Deviation:** nine Bitbucket keys contain `<` (`PaginatedResponse<Repository>`
etc.). They are generic *struct* instantiations, not containers, named by
schemars from the definition type; OAS 3.0.3's key pattern forbids `<`.
Renaming needs a `#[schemars(rename)]` on the Bitbucket type or a naming rule
in typed registration — a definitions change beyond API-level constructor
calls — so it is left unmade per the plan's authority rule, and the test pins
exactly those nine names so a new one fails. Review item.

**R5/R6 scan.** `definitions/tests/schema_fidelity_baseline.rs` was renamed to
`schema_fidelity.rs` (it is now the gate, not a baseline) and its classifier
taught the new forms: null members are the position's nullability (paired
member walk skips them on both sides), a folded `anyOf[X, null]` compares X
with the output, keywords moved into split-type `anyOf` members count as kept,
`$ref` targets and every source type must survive. The registry test now
asserts zero lost and zero typed-loss positions.

| | props | untyped | deliberate | lost | typed loss | components lost |
| --- | --- | --- | --- | --- | --- | --- |
| Before (Phase 1, 18 APIs) | 7433 | 1970 | 1204 | 766 | 624 (284 beyond default) | 150 of 1872 |
| After | 7558 | 1208 | 1208 | **0** | **0** | **0** of 1872 |

Per API after: every row has lost 0 and typed loss 0; untyped = deliberate:
Anthropic 2, ArtificialAnalysisData 2, ArtificialAnalysisCritPt 2, Bitbucket 0,
OpenAI 1175, ElevenLabs 0, Gitea 1, GitHub 1, GitLab 0, HuggingFaceHub 2,
LmStudio 4, OllamaNative 1, OllamaOpenAI 1, EmqxBasic 8, EmqxBearer 8,
Eversolo 1, SamsungSmartTv 0, UnfoldedCircleCoreRest 0. Other positions
(items, members, map values): 1826, of which 146 deliberate (all OpenAI
`serde_json::Value` items/members), 0 lost. Properties rose 7433 → 7558
because tagged-enum member objects (Anthropic 71 → 139, LmStudio 70 → 127)
are now exported and walked.

Deliberate free-form positions outside OpenAI (33; OpenAI's 1175 properties
and 146 other positions are `serde_json::Value` in `definitions/src/openai/types.rs`):
Anthropic `ContentBlock|oneOf[3].input`, `Tool.input_schema`;
ArtificialAnalysis{Data,CritPt} `CritPtSubmission.generation_config`,
`CritPtEvaluateBody.batch_metadata`; Gitea/GitHub `IssueSummary.pull_request`;
HuggingFaceHub `MetricResult.value`, `DatasetConfig.data_files`; LmStudio
`ChatOutputItem|oneOf[1].arguments`, `InvalidToolCallMetadata.arguments`,
`ChatStreamEvent|oneOf[11].arguments`, `ChatStreamEvent|oneOf[12].arguments`;
Ollama{Native,OpenAI} `ShowModelResponse.model_info`; Emqx{Basic,Bearer}
`LoginResponse.license`, `PublishBody.user_properties`,
`CreateRuleBody.metadata`, `RuleAction.args`, `TestRuleBody.context`,
`RuleInfo.metadata`, `TestRuleResponse.data`, `AlarmInfo.details`; Eversolo
`InputOutputListResponse.outputInfo`. The four new entries are composition
members the old export never reached.

### Wave 6: Artifacts

`just generate` (twice; the second run confirmed the final code reproduces
the committed files). Only the 15 `openapi/*.json` changed (+14.8k/−3.6k
lines); `schema/src` and `postman/` are byte-identical to HEAD, so the
generated Rust needed no comparison tree beyond `git status` (clean). A
structural diff over all 15 documents: everything outside
`components.schemas` is identical (paths, info, security, extensions),
component key sets are identical, and no property disappeared (property
positions 7029 → 7092 counted per document; Anthropic and LmStudio gained
member properties). `nullable` occurrences: 0 → 1978.

### Verification

Requirement-to-test mapping (all L1, F1 in-process at the crates' public
APIs; generation and export are exercised through `schematic-gen`'s public
validation and the real `just generate` recipe):

| Requirement | Tests |
| --- | --- |
| R1 type arrays, nullable (shape) | `schematic-define` `openapi::export::conversion::tests::{a_type_array_with_null_exports_typed_and_nullable, several_non_null_types_export_as_any_of_with_their_own_keywords, a_nullable_reference_joins_the_null_branch_in_either_dialect, a_folded_nullable_member_keeps_its_constraints}` |
| R1 accepted/rejected values: `Option<String>`, `Option<Vec<T>>`, `Option<Struct>`, nullable enum (root and nested dialects), integer/number union | `schematic-definitions` `schema_values::{optional_fields_accept_null_and_their_type_only, an_integer_and_number_union_accepts_both}` |
| R2 enums, const, `rename_all` export | `conversion::tests::enums_and_constants_keep_their_value_domain`; `schema_values::enums_keep_their_renamed_value_domain` |
| R3 compositions, tagged enum, flattened struct, sibling constraints, bounds, unsupported keywords | `conversion::tests::{compositions_convert_their_members_and_keep_sibling_constraints, bounds_formats_defaults_and_required_survive, an_unsupported_keyword_fails_with_its_schema_path}`; `schema_values::{tagged_enums_flattened_structs_and_maps_keep_their_shape, bounds_and_lengths_are_enforced}` |
| R4 `additionalProperties`, `HashMap<String,T>`, `HashMap<String,Value>` | `conversion::tests::{additional_properties_survive_as_a_schema_or_a_boolean, free_form_and_boolean_schemas_export_unconstrained_or_impossible}`; `schema_values::tagged_enums_flattened_structs_and_maps_keep_their_shape` |
| R5 paired fixtures + 18-registry zero-lost scan | `schema_fidelity::{a_serde_json_value_field_exports_as_deliberate_free_form, an_untyped_output_of_a_constrained_source_is_lost, a_typed_output_that_drops_a_constraint_is_a_typed_loss, every_registry_exports_without_losing_a_schema_constraint}` |
| R6 before/after counts | the table above (printed by the scan with `--no-capture`) |
| R7 unresolved nested ref (all facts), every failure listed, external/deep/wrong-type refs, escapes, data keywords, extracted nested definition, recursion, deserialized definition, export + bundle block | `schematic-define` `schema_closure::*` (6 tests) |
| R7 generation blocks | `schematic-gen` `validation::tests::a_dangling_reference_inside_a_component_blocks_rust_and_openapi_with_its_location` |
| R7 18 registries / R53 keys | `schema_fidelity::every_registry_closes_its_references_and_names_no_container_component` |
| R53 `Vec<T>` inline | `schema_values::a_vec_response_exports_inline_with_an_element_reference` |

Regression proof: with HEAD's `conversion.rs`/`components.rs`/`mod.rs`
swapped back in temporarily, all five R1–R4 `schema_values` value tests and
the registry zero-loss scan fail; with the new bridge they pass. The R53
inline-array test passes on both (spec 4a already did it; it is a guard).

Input Robustness Matrix: the only reader-like code changed is the component
`$ref` walk the bundle loader now shares. Its `$ref` field shapes: absent → no
reference; string resolving → ok; missing name → `Some(name)`; `null`, number
(wrong type), `""` and `#/components/schemas/` (empty) → reported with
`missing: None`, never read as absent; all asserted in
`schema_closure::every_unresolved_reference_is_reported_in_component_then_schema_order`.
Document shapes are unchanged (the bundle's JSON loader is untouched;
`bundle_json_matrix` passes). Smell grep over `conversion.rs` and
`components.rs`: no `#[serde(default)]`, no `unwrap_or_default`; `.ok()` only
in `percent_decode`, where a malformed escape yields `None` and so a reported
reference, not a resolved one; one `filter_map(Value::as_str)` over `type`
entries already checked to be strings.

Gates, run once after all edits with nothing else running, from `schematic/`:

- `just test` → exit 0: L1 2800 run, 2800 passed (4 slow), 20 skipped;
  fresh-output 14 passed (real APIs compile offline against the new
  `schematic-define`); compile-fail 1 passed.
- `just lint` → exit 0. `just lint` runs `schematic-define` clippy without
  features, so the `openapi` code is not linted by it; additionally
  `cargo clippy -p schematic-define --all-features --all-targets -- -D warnings`
  → exit 0.
- `just check-tier-coverage schematic` → 0 stranded. New tests are all L1 by
  name; `define/tests/schema_closure.rs` and the two `definitions/tests/*.rs`
  files are auto-discovered test binaries (neither package sets
  `autotests = false`); `schema_closure`'s export assertions are
  `#[cfg(feature = "openapi")]`, which CI enables (`all-features = true`).
- `just lint-schema` not run: generated Rust is byte-identical to HEAD
  (`git status schema/` clean after `just generate`), so it would lint
  unchanged code.
- The affected-scope skip was not considered: code changed, so both gates ran.

Development note: early `just _test schematic-define` runs lacked
`--all-features` and silently omitted the 233 `openapi`-gated tests (and the
new bridge tests failed to compile unnoticed); every later define run used
`just _test schematic-define --all-features`, as the area recipe does.

Dependents: `model-citizen` (direct, `schematic-definitions` data types only)
and `schematic-schema` (generated, byte-identical) with its consumers
(`homelab/*`, `sniff`, `biscuit-speaks`). None calls the builders, export, or
the closure at runtime (grep), and no definitions type changed, so only the
code touching those types was run:
`BISCUIT_TEST_NARROW='test(/scanner::(ollama|lmstudio)/)' just _test model-citizen`
→ 45 passed. `schematic-schema` compiles against the new crates in the
fresh-output gate.

Cross-OS check: not run. The change is pure JSON-to-JSON transformation and
string handling (JSON pointers, percent-decoding); it touches no paths,
processes, terminals, or `#[cfg]` OS code.

Skills: `schematic-define` skill updated (SKILL.md build rule, define.md
closure check, openapi.md conversion rules and gates).

### Review agenda (Phase 2)

1. `"default": null` is dropped from exported schemas (R3 deviation; the
   alternative breaks the artifact-equals-its-own-parse invariant).
2. Nine Bitbucket `PaginatedResponse<…>` component keys contain `<` (R53
   deviation; OAS 3.0.3 key pattern). Fix needs a Bitbucket
   `#[schemars(rename = "PaginatedResponse_{T}")]` or a typed-registration
   naming rule — both outside this feature's authority.
3. The closure check is not part of `validate_references`, so OpenAPI import
   still accepts unresolved and deep-pointer refs (Unfolded Circle corpus,
   OpenAI matrix); Phase 4 owns importer diagnostics.
4. Bare `$ref` properties are no longer wrapped in `allOf` (465 positions).
5. New public API: `RestApi::validate_schema_closure`,
   `DefinitionError::UnresolvedSchemaReferences`, `SchemaReference`.

Phase 2 status: checkpoint met. R1–R7/R53 have evidence (R53 with the
recorded `<` deviation); all 18 registries pass closure and the zero-loss
scan; lost count 766 → 0 and typed losses 624 → 0; deliberate free-form
positions are named above; artifacts regenerated and drift tests pass;
generated Rust unchanged; `just test` and `just lint` green.

## Phase 3

Scope: Wave 7 "Map headers" (R8, R9). Single-task wave, implemented directly
(no subagents).

### Design and adopted defaults

- New `schematic/define/src/openapi/export/headers.rs`. `map_paths` sets each
  operation's `security` first, then derives the operation's **credential
  headers** from its *effective* requirements (own `security`, else the
  document's) resolved against the emitted `components.securitySchemes`:
  `apiKey`/`in: header` → its `name`; `http`, `oauth2`, `openIdConnect` →
  `Authorization`; query/cookie keys → none. So "covered by a securitySchemes
  entry" is literal, and an endpoint auth override (replacement or
  `.public()`) changes which headers are credentials.
- Effective fixed headers = API headers overlaid by endpoint headers, names
  compared case-insensitively, endpoint wins (keeps the endpoint's spelling).
  This mirrors the runtime: API headers are client defaults and a request
  part with the same name replaces them (`runtime/src/client.rs`, request
  assembly).
- Each ordinary fixed header → operation-level `in: header`,
  `required: true`, `style: simple`,
  `schema: {type: string, enum: [value], default: value}`. Adopted default:
  emitted inline per operation, not as a `components.parameters` `$ref`
  (source: plan Wave 7 says "emit required ordinary headers"; inline avoids a
  new ref kind the importer cannot yet resolve — Phase 4 adds parameter-ref
  resolution).
- Reserved names: `Accept`/`Content-Type`/`Authorization` are never emitted,
  fixed or explicit. A fixed `Accept` equal (case-insensitive) to a 2xx
  response media type, or a fixed `Content-Type` equal to a request-body
  media type, is expressed by `content` and silent; otherwise it is a gap.
- Explicit header `ParamDef` collisions (adopted default, source: R9 "merge a
  matching ordinary header parameter or diagnose incompatible declarations"):
  compatible = an unconstrained string or a string enum listing the value →
  one parameter, the declared one, made `required: true` with the value as
  `default` (its enum, description, explode kept). Incompatible (non-string
  type, or enum without the value) → declared parameter exported unchanged +
  gap. Explicit header params whose name is reserved or a credential header
  are also dropped with a gap (checkpoint: "no duplicate/reserved/credential
  parameter is emitted").
- **Diagnostics:** export keeps returning `Result<OpenAPI, _>`; gaps flow
  through a new public `schematic_define::openapi::header_gaps(api)` →
  `Vec<HeaderGap { endpoint_id, name, value: Option<String>, reason }>`
  (`HeaderGapReason::{Credential{scheme}, Accept, ContentType, Authorization,
  IncompatibleParameter}`). `schematic-gen`'s `export/vocabulary.rs` turns
  each into a new `Feature::FixedHeader` (`fixed-header`) **warning** for the
  OpenAPI output — consistent with every other export-loss finding there
  (warning, output still written). One mapping implementation serves both
  the document and the diagnostics.
- "Preserve extra fixed-header detail in the extension": already true — the
  document- and operation-level `x-schematic.headers` keep every declaration;
  nothing new was needed.

### Artifact effect (regenerated with `just generate-openapi`)

Rust under `schematic/schema/` unchanged (no git diff after generation).
Fixed-header parameters now in artifacts: Anthropic 4 (`anthropic-version:
2023-06-01` on every operation; `X-Api-Key` is the `apiKeyAuth` credential and
stays a scheme), GitHub 32 (`X-GitHub-Api-Version`, `User-Agent` on 16
operations), GitLab 19 (`User-Agent`). Gitea and OpenAI gain none.
Changed artifacts this phase: `anthropic.json`, `github.json`, `gitlab.json`.

### Shipped gaps (review item)

17 `fixed-header` warnings now print on every `just generate`:
- GitHub, 16 operations: `Accept: application/vnd.github+json` (15) and the
  endpoint override `application/vnd.github.raw+json` (GetRepositoryContentRaw)
  — responses declare `application/json`/text, so OpenAPI cannot say it.
- Gitea `GetRepositoryContentRaw`: inherits `Accept: application/json` but
  returns text. This looks like a latent definition imperfection; fixing it
  (an endpoint `Accept` override) changes runtime behavior and generated Rust,
  which this phase must not do, so it is reported, not fixed.

Two existing gen tests asserted "no warnings" and were updated to pin these
exact gaps rather than weakened: `output_validation::
live_definitions_have_no_embedded_queries_or_validation_gaps` gains a pinned
`FIXED_HEADER_GAPS` (17 rows, OpenAPI only, warning severity), and
`cli_migrated_provider_generates_all_outputs_without_warnings` was renamed
`..._with_only_its_known_warning` and asserts exactly Gitea's one line.

### Requirement → test mapping

New file `schematic/gen/tests/l1/fixed_header_export.rs` (declared in
`tests/l1/main.rs`; L1, no tier marker). All go through public
`build_openapi_grouped`, `validate_openapi`, and `openapi::header_gaps`:

| Requirement | Test |
| --- | --- |
| R8 fixed header → required single-value enum + default; extension retained | `api_fixed_header_exports_as_a_required_single_value_enum_with_default` |
| R9 endpoint over API, case-insensitive, exactly one param | `endpoint_header_overrides_the_api_header_case_insensitively` |
| R9 credential exclusion via effective auth incl. override and `.public()` | `credential_headers_follow_the_effective_operation_auth` |
| R9 reserved Accept/Content-Type/Authorization (matching vs gap; public vs bearer Authorization) | `reserved_headers_are_never_parameters_and_unexpressed_values_warn` |
| R9 merge compatible / diagnose incompatible / explicit reserved param | `explicit_header_parameters_merge_when_compatible_and_warn_when_not` |
| R8 committed Anthropic artifact | `committed_anthropic_artifact_sends_anthropic_version_on_every_operation` |
| Checkpoint: no duplicate/reserved/credential header param in any artifact | `committed_artifacts_declare_no_duplicate_reserved_or_credential_header_parameter` (all 15 via `include_str!`, count guarded against the directory) |
| Shipped gaps pinned | `output_validation::live_definitions_have_no_embedded_queries_or_validation_gaps`, `output_validation::cli_migrated_provider_generates_all_outputs_with_only_its_known_warning` (spawns the CLI: wiring of the warning into rendered output) |
| Artifacts equal fresh output | existing `artifact_drift` |

Input robustness matrix: not applicable — this phase changes a writer
(OpenAPI export) and adds no reader or config field.

### Gates

- Targeted during development: `just _test schematic-gen -E '…'` (one early
  run used `cargo nextest run` directly before switching to the recipe).
  First run failed only on my own expected JSON (`explode: false`,
  omitted `required: false`), then the two "no warnings" tests above.
- `just test` (schematic): 2807 passed / 20 skipped; fresh-output 14 passed;
  compile-fail 1 passed. No failures, none load-sensitive.
- `just lint`: clean. `cargo clippy -p schematic-define --all-features
  --all-targets -- -D warnings`: clean.
- `just check-tier-coverage schematic`: 0 stranded.
- Scope calculator not used to skip gates; both gates ran.
- Dependents: no package outside `schematic/` imports `schematic_define::
  openapi` or `schematic_gen` (grep over `biscuit-speaks`, `homelab/*`,
  `sniff`, `model-citizen`); they consume `schematic-schema`, whose generated
  source is byte-identical, and define's change is additive (the `export`
  signature is unchanged). No dependent tests run.
- Cross-OS check: not run — no path, process, terminal, or `#[cfg]` code; the
  one filesystem read in the new test joins a literal onto
  `CARGO_MANIFEST_DIR`.

### Docs and skills

- `schematic/docs/io/export-openapi.md`: new "Fixed headers" section (rules,
  example, Mermaid flow).
- `schematic/docs/generation/validation.md`: `fixed-header` row; the header
  parameter paragraph now says OpenAPI reports what it leaves out.
- `.claude/skills/schematic-define/schematic/openapi.md`: fixed-header export
  rules and the pinned shipped gaps.

### Review agenda (Phase 3)

1. 17 `fixed-header` warnings on every generation (GitHub `Accept` vendor
   media types, Gitea raw-content `Accept`). Alternative was to treat any
   `Accept` as expressed and stay silent, which hides a real difference
   between the runtime and an OpenAPI-generated client.
2. Gitea `GetRepositoryContentRaw` sends `Accept: application/json` for a
   text response — probably a definition bug worth a separate fix.
3. New public API: `openapi::header_gaps`, `HeaderGap`, `HeaderGapReason`;
   new `schematic_gen::diagnostics::Feature::FixedHeader`.
4. Merged compatible parameters are `required: true` with a `default`: the
   runtime always sends the header, so a consumer must too.
5. Phase 4 importer: header parameters with a single-value `enum` equal to an
   `x-schematic.headers` entry must be recognized as the fixed header and not
   become a `ParamDef` (contracts.md `RestApi::headers` row). Until then a
   re-import of Anthropic/GitHub/GitLab artifacts gains header `ParamDef`s.

Phase 3 status: checkpoint met. Ordinary fixed headers are standard header
parameters; no duplicate/reserved/credential header parameter in any
committed artifact; Rust output unchanged; export, drift, and the common
phase checks pass.

## Phase 4

Scope: Waves 8–10 (R10–R18, R54). Started 2026-10-23.

Execution plan: Wave 8 runs as two parallel subagents with disjoint file
ownership (parameters + strict: `import/builder.rs` strict gate,
`import/mappings/{mod,parameters}.rs`; multipart: `import/mappings/responses.rs`,
`export/request_body.rs`). Wave 9 (extension reader/writer, auth reconciliation)
is done directly by the orchestrator. Wave 10 matrix tests follow.

### Wave 8: Standard import mappings (R16, R17, R18)

Two parallel subagents, disjoint files; both reported green scoped runs.

- **R16 strict** (`import/builder.rs`): strict fails only when a WARN or ERROR
  exists; INFO-only passes and is returned. The `OpenApiError::Validation` is
  located at the first WARN/ERROR and quotes the count. Docs corrected on
  `OpenApiImport::strict`, `OpenApiImportOptions::strict`, `build()`, and gen
  `ImportOptions::strict`. The tautological test was replaced by
  `build_strict_mode_passes_info_only_diagnostics`,
  `build_strict_mode_fails_on_warn_diagnostic`,
  `build_strict_mode_fails_on_error_diagnostic`.
- **R17 path-item parameters** (`import/mappings/{mod,parameters}.rs`): local
  `#/components/parameters/X` refs resolve before the merge; identity is
  `(name, in)`, header names case-insensitive; an operation parameter replaces
  the inherited one in place. Unresolved local ref and remote ref → ERROR at
  `#/paths/<escaped>/[<method>/]parameters/<i>`, parameter skipped, never
  fetched. `map_operation` gained `path_item_parameters`.
- **D17 typed path parameters**: all path parameters import into
  `EndpointParams.path` when any has a non-string schema, a description, or
  `deprecated`; otherwise none. Adopted defaults (subagent, source: D17 +
  `validate_typed_paths`): an array/JSON path type imports as `String` with
  WARN (validation forbids them); undeclared placeholders get a plain string
  parameter when typed import applies; declared parameters with no
  placeholder are dropped with WARN.
- Standard `deprecated` is now read for operations and parameters.
- **R18 multipart** (`import/mappings/responses.rs`, `export/request_body.rs`):
  export writes `encoding.<part>.contentType` (accept joined by `", "`, or
  `application/json` for a JSON part) and Files `minItems`/`maxItems`; import
  reads them back. Unresolved part/items refs → ERROR, part skipped; encoding
  for an unknown part, `headers`/`style`/`explode`/`allowReserved`, an array of
  non-file items, and a binary part on a URL-encoded body → WARN; empty
  `contentType` → ERROR.
- **Orchestrator correction to R18 (found by the OpenAI re-import guard):** the
  subagent's first rule made any component `$ref` part a JSON part, which
  turned OpenAI's `response_format` (a `$ref` to a string enum) and `model`
  (an `anyOf` of strings) into JSON parts. The rule now follows OpenAPI's
  default part encoding: a JSON `encoding.contentType` makes a JSON part; a
  primitive, a component naming one, or a composition of primitives is a text
  part silently; an object-valued part without a JSON encoding is WARN and
  stays text (adopted default, source: contracts.md matrix 2 "an
  object-valued part is WARN", R18 "never silently become text"). Tests:
  `primitive_parts_without_an_encoding_import_as_text_silently`,
  `object_parts_without_json_encoding_warn_and_stay_text`,
  `json_encoded_component_and_inline_parts_import_as_json`.
- Fixed the malformed missing-operationId INFO location (`#/paths~1…` →
  `#/paths/~1…`, `~` escaped) and the path-reference WARN location.

### Wave 9: Extension reader and writer (R10–R15, R54) — orchestrator

Writer (`openapi/extensions.rs`, `export/{mod,paths,security,components}.rs`):

- `SchematicDocExtension` / `SchematicOpExtension` are now serialize-only
  writer shapes (R-S1: the reader never derives `Deserialize` on them). New
  doc members `version: 1` (always), `env_auth`, `auth`, `secret_names`; new
  operation members `auth`, `env_mapping`, `path_template`, `params`. New
  public `AuthDetail`, `ParamDetail`, `EXTENSION_VERSION`.
- Operation `request` is written only for nonempty form bodies (JSON, text,
  binary bodies are standard-only; contracts.md).
- Root `security` is derived from `effective_auth_policy()` with one
  requirement per accepted method (D6) through the new
  `security::policy_security`, which also returns the R54 details of the
  schemes it registers. `map_security_requirements` is gone. A bearer header
  override registers a distinct scheme (description differs) from an API key
  on the same header, so details never collide.
- `SchematicSchemaExtension` and `AuthExtension` deleted with their
  re-exports, tests, the `Default` allowlist entry in `tests/builders.rs`,
  and the `docs/definition-builders.md` row (R15).

Reader (new `import/extension.rs` decode + `import/reconcile.rs` apply;
`import/auth.rs` deleted, its legacy first-scheme fallback moved into
`reconcile::map_root_auth`):

- Version gate: absent → 1; anything but integer 1, or a non-object
  `x-schematic`, → one WARN and every Schematic extension in the document
  (operations included) ignored (D).
- Each member decodes independently from the raw `Value`; unknown members
  INFO; malformed → WARN at `#/x-schematic/<member>…` or
  `#/paths/<escaped>/<method>/x-schematic/<member>…`, member dropped, siblings
  kept. Explicit `null` is WARN except at the writer-null positions
  (`env_mapping.{bearer_token,basic_user,basic_pass,api_key}`, form field
  `description`, `Files.min/max`), implemented as one null-position walk.
- Validation reuses `validate_api`'s pieces (`RateLimit::validate`,
  `RetryGuidance::validate`, `validate_error_entries`,
  `validate_success_shapes`, `validate_origin`, `valid_header_name`,
  `valid_rust_name`, new `builders::env_mapping_conflicts` factored out of
  `validate_api`).
- Standard wins (R12): `success`/`response` and the error decomposition are
  re-exported with `map_responses` and compared with the operation's
  standard responses (descriptions ignored, 2xx and error statuses compared
  apart); a match applies the extension, a mismatch keeps the standard-derived
  value with WARN. A legacy JSON/text/binary `request` is compared by
  re-exported `content`; a form `request` fills only `accept`, file counts, and
  a JSON part's reference, WARN on any disagreement.
- Auth: root `security` requirements are authoritative (none → anonymous, one
  → `AuthStrategy`, several → `AuthPolicy`); absent root `security` keeps the
  legacy first-scheme fallback. `auth.<scheme>.{bearer,value_prefix}` applies
  only to an `apiKey` header scheme the relevant `security` applies; else WARN
  and standard wins. Operation `security`: absent inherits, `[]` → `Public`,
  identical to root with no details → inherits, otherwise
  `EndpointAuth::Policy`; operation `auth`/`env_mapping` without an override →
  WARN. Adopted default (source: D6 "explicit/env_fallback split not
  round-tripped"): an imported policy falls back to its first
  environment-capable method, dropped after assembly when the effective
  mapping cannot supply it (`credential_source_problem`).
- Basic auth's lists are restored from `env_mapping.basic_user/basic_pass`
  (the builder writes both), so Bitbucket/EMQX keep their variables.
- Fixed-header recognition (Phase 3 handoff): a required header parameter
  whose only value is `Enum([v])` equal to an effective fixed header (endpoint
  over API, case-insensitive) is dropped as that header's standard form.
- `path_template` applies when stripping its markers gives the path key.
  `params.<loc>.<name>` sets `secret`/`rust_name`; it imports plain-string
  path parameters when a path entry needs one; it wins over the internal
  `x-schematic-query-rust-names` with WARN on disagreement (R-S12).
- Importer also: reads `info.version` into `RestApi::version`; absent
  `servers` → WARN at `#/servers` (contracts.md matrix 2; ten define fixtures
  gained a `servers` entry); the reserved request-name suffix uses the
  imported `request_suffix`; gen `run_import` overrides
  `request_id_header` only when `--request-id-header` is given (D24).
- `x-schematic-grouped-from` is reported as INFO.

Artifacts regenerated with `just generate-openapi`: all 15 OpenAPI files
change (`version: 1`; JSON `request` copies removed; Gitea gains `env_auth`
and `auth.apiKeyAuth.value_prefix`; Hugging Face gains 16 `path_template`;
ElevenLabs/Unfolded Circle multipart gain `encoding`). `schema/` Rust
unchanged (no diff).

Existing tests updated because they pinned the old lossy behavior:
`response_openapi.rs` (standard-only cases now export with
`skip_extensions()`/`without_extensions`, plus new
`extension_import_restores_declared_error_levels_and_stream_ends`),
`export::tests::export_strips_reserved_marker_from_path_and_params`,
gen `vocabulary_exports::{repeated_and_reserved_placeholders_declare_each_name_once,
environment_variable_names_never_become_exported_values}` (operation
`env_mapping` is now extension metadata, D5), gen `extension_reimport.rs`
(rewritten against the reader; `success: []` is now WARN at its pointer).

**OpenAI re-import (review item).** `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition`
now differs from `definitions/src/openai/mod.rs` in exactly two places:
`.version("2.3.0")` (info.version is read) and the realtime `session` part,
which the vendored spec declares `encoding.contentType: application/json`, now
a JSON part of `RealtimeSessionCreateRequestGA`. Re-importing is an
endpoint-level definitions change and changes generated Rust, which the
plan's execution-authority rule leaves unmade; the test pins exactly those two
differences (any other drift still fails, and running `just import-openai`
fails it until the pins are removed). Author decision at review: adopt with
`just import-openai`, or keep.

### Wave 9 follow-up: R10 round trip over every artifact (subagent + orchestrator)

New `gen/tests/l1/extension_round_trip.rs` (24 L1 tests with
`extension_reimport`): one `#[test]` per committed artifact via
`macro_rules!` with an `include_str!` literal each, plus a guard that the list
equals `schematic/openapi/*.json` and `apis_by_module()`. Each artifact must
import with **zero WARN/ERROR and pass `.strict()`**, re-export equal to the
committed document, and re-import equal to the current definition
(`canonical(serde_json::to_value(..))`, D9) except these named, value-checked
categories (each asserted as the exact set per artifact):

- Document: `grouped-title`, `grouped-server-description`,
  `grouped-from-sibling` (grouped writer, D8), `generic-component-names`
  (Bitbucket `X<Y>` → `X_Y_`).
- Definition: `version-default` (definitions leave `version` unset; import
  reads `info.version`), `grouped-name`, `grouped-operation-ids`,
  `grouped-auth-union` (EMQX) (D8), `schema-rust-path` (rebuilt exactly as
  `{module_path}::{Name}`), `schema-json-normalized` (JSON Schema vs OpenAPI
  3.0 form; components compared equal on the document side), `pagination`
  (D7).
- Not observed, so not listed: `pager_managed`, description vs summary, D17
  plain path parameters, the D6 policy split (no shipped API exercises them).

Purpose-built fixture (every v1 member load-bearing at both levels) round-trips
through JSON and YAML with exact definition and document equality.
R54: Gitea-style `value_prefix` and a `BearerToken{header}` override survive;
a detail on an `http bearer` scheme is WARN at `#/x-schematic/auth/apiKeyAuth`
and the standard scheme wins; every artifact's and the fixture's `auth`
entries hold only `bearer`/`value_prefix`, and no `x-schematic` holds
`strategy`, `scopes`, `oauth_scopes`, or a credential-like value.

Bugs found by this test and fixed:
- reader: extension shapes kept unsanitized component names
  (`PaginatedResponse<SourceEntry>`), so Bitbucket failed reference
  validation (subagent fix, `reconcile.rs::sanitize_shape`);
- gen grouped writer (`gen/src/export/openapi.rs::merge_path_parameters`)
  appended an identical path-item parameter again (14 duplicates in
  `emqx.json`, which OpenAPI forbids);
- `export/parameters.rs` omitted `explode: true` on non-`form` styles
  (OpenAI `deepObject` `metadata` came back `false`);
- imported component `rust_path` ignored the document's `module_path`
  (now the effective module: CLI option, else extension);
- classification WARNs ("no named schema …") survived after the extension
  restored the exact shapes, failing strict import of 9 artifacts; they are
  now dropped for the 2xx (resp. error) responses the extension restored.

### Wave 10: Reader acceptance (input matrices)

New `define/tests/openapi_extension_matrix.rs` (table 1 + version controls +
document shapes + strict), `define/tests/openapi_standard_matrix.rs`
(table 2), shared `define/tests/matrix_support/mod.rs` (control `RestApi`
exported once, one mutation per cell, 24 per-column probes so every cell also
proves its unrelated siblings still apply). Each cell runs from JSON text and
YAML text, permissive and strict; about 338 table-1 and 70 table-2 cells per
format; slowest test 2.2 s. Control rows import every column with no
WARN/ERROR. Duplicate keys (inside `x-schematic` and a standard member),
trailing content, and a second YAML document fail before `Value` conversion
(`check_text`).

Outcomes adopted where contracts.md's cell differs (each tagged `adopted(..)`
in the test; adopted defaults, source: R12 "standard wins" / plan matrix
codes):

- adopted default: an operation whose declared error table (its own
  `errors`/`no_declared_errors` plus the document defaults) does not reproduce
  its standard responses is WARN at that operation's `x-schematic/errors`
  even when the member is absent or `[]` (covers removing the document
  `x-schematic` while operation extensions remain).
- adopted default: document `errors` that disagree only at statuses the
  document entries export are withdrawn whole with one WARN at
  `#/x-schematic/errors`; every operation keeps its standard-derived table.
- adopted default: a rejected `success` does not fall back to the legacy
  `response`; a legacy `response` that disagrees is silently superseded
  (it only ever summarized the first shape).
- adopted default: an unresolved extension reference that disagrees with a
  standard reference (request JSON part, SSE item) is W (standard wins); it is
  ERROR only when the extension would fill an untyped standard part.
- adopted default: an empty `auth.<scheme>` entry is no detail (O).
- adopted default: a semantic structural error found after parsing
  (unresolved/remote `$ref`, empty `encoding.contentType`, empty
  `info.version`/`servers[0].url`/`externalDocs.url`, non-required path
  parameter, empty `requestBody.content`, empty `apiKey` `name`) is an ERROR
  diagnostic (strict fails, permissive skips the item) rather than a failed
  build; shape errors (null, wrong type, empty parameter `name`,
  `security: null`, `info.version: null`) fail the build.
- ruling precedence: parameter `schema: {}` stays `QueryParamType::Json`; the
  earlier import-normalization contract (`tests/import_normalization.rs`)
  pins it and supersedes contracts.md's `String`.
- known gap: a second YAML document fails without a location
  (`serde_yaml_ng`'s message carries none).

Smell grep (subagent, `extension.rs`, `reconcile.rs`, `mappings/parameters.rs`,
`mappings/responses.rs`): no `#[serde(default)]`. Load-bearing hits and their
coverage: `extension.rs` `unwrap_or_default()` after a reported rejection
(covered; rejection tracked by `errors_rejected`/`success_rejected`);
`reconcile.rs` `ext.errors.unwrap_or_default()` (absent = `[]`, covered);
`reconcile.rs` `.ok()` on re-export (failure ⇒ mismatch WARN, the safe
direction). Not covered, logged: `responses.rs` `u32::try_from(minItems).ok()`
drops a bound above `u32::MAX` silently; `parameters.rs`
`filter_map(|v| v.clone())` drops `null` members of a string `enum` silently
(pre-existing).

GraphQL: the extension carries no GraphQL member and the exporter still skips
GraphQL operations; the named-diagnostic verification belongs to Phase 9,
which installs those diagnostics (plan Wave 10 wording).

R15 grep (`SchematicSchemaExtension|AuthExtension|oauth_scopes` over
`schematic/` and `.claude/skills`, excluding features/fixes/research/reviews):
only `schematic/oauth/oauth-schematic-design.md` (explicitly a historical,
unmaintained proposal), the `docs/definition-builders.md` sentence saying
`oauth_scopes` was removed, and the round-trip test's forbidden-key list.

### Documentation and skills

Documenter subagent: `docs/io/openapi-extensions.md` rewritten (writer and
reader, members at both levels, version rule, decoding and standard-wins
rules, auth details, round-trip subset and exclusions, Mermaid read path;
schema-level section removed); `docs/io/import-openapi.md` (parameters, form
bodies, authentication, extensions, diagnostics and strict mode);
`docs/io/export-openapi.md` (request bodies, document security, `explode`);
`docs/generation/generated-clients.md` (strict wording, round-trip link);
`docs/definition-builders.md` (Default row); skill
`.claude/skills/schematic-define/{SKILL.md, schematic/openapi.md,
schematic/gen.md}`. Rustdoc drift fixed (D16): `RestApi::cross_origin`
(endpoints do override it), `RestApi::version` precedence; round-trip claims
in `extensions.rs`, `openapi/mod.rs`, `export/mod.rs` now name the supported
subset.

### Requirement → test mapping (Phase 4)

| Requirement | Tests (all L1, in-process at the public API) |
| --- | --- |
| R16 strict | `builder.rs` `build_strict_mode_{passes_info_only_diagnostics,fails_on_warn_diagnostic,fails_on_error_diagnostic}`; matrix strict checks per cell |
| R17 path-item params, refs | `openapi_import_parameters.rs` (11 tests); table 2 parameter columns |
| D17 typed path params | `typed_path_parameters_{import_in_placeholder_order,round_trip_through_export}`, `plain_string_path_parameters_stay_untyped` |
| R18 multipart | `openapi_multipart_import.rs` (15 tests, AddVoiceSample shape JSON+YAML round trip); real ElevenLabs/UC artifacts via `extension_round_trip::artifact::*` |
| R10 round trip | `extension_round_trip.rs` (15 artifacts + fixture JSON/YAML); `response_openapi::extension_import_restores_declared_error_levels_and_stream_ends` |
| R11 version | `openapi_extension_matrix.rs` version controls (absent, 1, 2, 1.0, "1", null, true, [1], non-object, `{}`) |
| R12 standard wins | matrix R12 cells; `an_auth_detail_on_an_http_bearer_scheme_warns_and_the_standard_scheme_wins` |
| R13 unknown/malformed | matrix unknown-key INFO and malformed-beside-valid-sibling cells, permissive and strict |
| R14 row 14 | `extension_reimport.rs` (rewritten against the reader): "not reproducible, guarded" stands |
| R15 dead structs | compile (symbols absent) + grep above; `extensions.rs` writer-shape tests |
| R54 auth details | `extension_round_trip::{an_api_key_value_prefix_and_a_bearer_header_override_survive_the_round_trip, no_fixture_extension_carries_an_auth_field_beyond_the_two_details}`, per-artifact R54 scan; `security.rs` `bearer_header_overrides_and_value_prefixes_get_details_on_distinct_schemes` |
| Phase 3 handoff (fixed headers) | every artifact round-trips with no header `ParamDef` gained (`extension_round_trip`) |

### Gates (Phase 4)

- `just test` (schematic): 2849 passed (4 slow), 20 skipped; fresh-output
  14 passed; compile-fail 1 passed. No failures, none load-sensitive.
- `just lint` (schematic): clean. `cargo clippy -p schematic-define
  --all-features --all-targets -- -D warnings`: clean. `cargo clippy -p
  schematic-gen --tests -- -D warnings`: clean.
- `just check-tier-coverage schematic`: 0 stranded.
- `just generate-openapi` regenerated all 15 OpenAPI artifacts; `schema/`
  (generated Rust) shows no diff. `lint-schema` not rerun (generated Rust
  unchanged).
- Scope calculator not used to skip gates; both gates ran.
- Dependents: no package outside `schematic/` depends on `schematic-define`
  or `schematic-gen`; dependents consume `schematic-schema`, whose generated
  source is byte-identical. No dependent runs needed.
- Cross-OS check: not run — no path, process, terminal, or `#[cfg]` code;
  test reads use `include_str!` or `manifest_dir!()` joins.

### Review agenda (Phase 4)

1. OpenAI re-import pins two pending differences (`.version("2.3.0")`, realtime
   `session` part as JSON per the vendored spec's `encoding`). Adopt with
   `just import-openai` (changes the generated Realtime client) or keep.
2. Adopted matrix outcomes above (error-table attribution, legacy `response`,
   ERROR-diagnostic vs build-error split for semantic structure errors).
3. Multipart object-valued parts without a JSON encoding stay text with WARN
   (rather than becoming JSON parts), to keep OpenAI's committed definition
   and generated client unchanged.
4. New public surface: `AuthDetail`, `ParamDetail`, `EXTENSION_VERSION`;
   `SchematicDocExtension`/`SchematicOpExtension` are serialize-only;
   `SchematicSchemaExtension`/`AuthExtension` deleted.
5. Absent `servers` is now a WARN, so strict import of a server-less document
   fails (contract matrix 2).
6. Known gaps logged: nullable string-enum members dropped in parameter enums;
   a `minItems` above `u32::MAX` dropped; `remap_reference_names` does not
   rename a form JSON part whose component yields its name to a request
   struct; a second YAML document's error has no location.

Phase 4 status: checkpoint met. R10–R18/R54 pass, mappings and the subset are
logged, both public matrices pass in JSON and YAML, generated Rust is
unchanged, and the artifact and common phase checks pass.

## Phase 5

Consolidate API registration (R33–R35). Implemented directly by the
orchestrator without subagents: the change threads one type through
`schematic-definitions` and `schematic-gen`, and its write set is not
divisible.

### R33 re-derivation

Phase 1's inventory (Wave 2 above) still held at the start of this phase:
4 runtime REST lists (`registry::MODULES`, `registry_key_for`,
`pipeline::AVAILABLE_APIS`, `pipeline::resolve_api`'s match), 2 runtime
WebSocket lists (`ws_modules::WS_DEFINITION_MODULES`,
`ws_modules::shipped_ws_module`), one supporting map
(`output/assemble/helpers.rs::definitions_module_path`), and test tables.
Endpoint literals were not re-measured (no endpoint literal changed).

### Wave 11: the inventory (R34)

- New `schematic/definitions/src/registry/inventory.rs` (replaces
  `registry/lookup.rs`): `ApiRegistration { name, module, definitions_path,
  protocol }`, `Protocol::{Rest(RestRegistration { key, group, member }),
  WebSocket(WebSocketRegistration { define, constructor, summary })}`, and the
  one `REGISTRATIONS` table (18 REST + 5 WebSocket, catalog order). Derived:
  `registrations()`, `find_rest`, `rest_apis` (builds each module group once,
  panics on an unregistered group member, a wrong member index, or a name
  mismatch), `group_by_module`, `websocket_apis`, `registry_key_for`.
- `all_apis()` and `apis_by_module()` (`definitions/src/lib.rs`) now derive
  from the inventory; `apis_by_module` groups by the registered module, and a
  test asserts that equals the API's own `module_path`/lowercased name, so the
  grouping is unchanged.
- `schematic-gen`: `available_apis()`/`available_apis_in`,
  `resolve_api()`/`resolve_api_in` (`pipeline.rs`) derive from REST entries;
  `ws_modules.rs` derives module files, re-exports, and `define_api` from
  WebSocket entries (`ws_registrations`, `ws_helper_name`);
  `assemble_lib_rs`, `assemble_prelude`, `read_rust_inventory`/
  `prelude_exports`, and the reserved-module-name check take the
  registrations; `RustOutputRequest` gains `registrations`;
  `validate_rust_client_in` added (`validate_rust_client` = shipped);
  `definitions_module_path` derives from REST entries.
- Removed hand-written lists (exact locations): `registry/lookup.rs`
  `MODULES` (15 entries) and `registry_key_for`'s 18-arm match;
  `gen/src/pipeline.rs` `AVAILABLE_APIS` and the 18-arm `resolve_api` match
  with its 15 `use` lines; `gen/src/output/ws_modules.rs`
  `WsDefinitionModule`/`WS_DEFINITION_MODULES` (5) and `shipped_ws_module`
  (5 arms), plus `shipped_ws_apis`; `gen/src/output/assemble/helpers.rs`
  `definitions_module_path` match arm; `assemble_prelude_with_options`
  (folded into `assemble_prelude(apis, registrations)`; `&[]` = no helpers).
- Test tables replaced by derivation: `definitions/src/registry/mod.rs`
  18-row key table (keys now live in the registrations; the inventory test
  checks `registry_key_for(name) == key`), and the four unguarded 5-row
  WebSocket tables — `gen/tests/l1/ws_codegen.rs::lower_all_apis`,
  `gen/src/ws_codegen/plan.rs::lower_all_known_ws_apis`,
  `definitions/tests/ws_conformance.rs::shipped`,
  `definitions/tests/ws_definition_json.rs::every_shipped_definition_round_trips_twice_without_loss`.
- Kept, with reasons: per-API expectation tables that hold data per API and
  are already guarded set-equal with the inventory-derived lists
  (`definitions/tests/api_policy.rs`, `gen/tests/l1/definition_baseline.rs`,
  `definitions/tests/pagination_coverage/`); `definitions/src/lib.rs`
  `pub mod`/`pub use define_*` (Rust module declarations the compiler needs;
  not a runtime list); the `gen/tests/l1` legacy-filename lists (Phase 6's
  resolver work).
- Grep (R34, recorded): `grep -rn --include='*.rs'` for `"ollama-native"`,
  `"OllamaNative"`, `"elevenlabs_ws"`, `"unfolded_circle_core_rest"`,
  `define_samsung_smart_tv_remote_ws_api`, `"samsung-smart-tv"` over
  `definitions/src`, `gen/src`, `gen/examples`: the only list holding them is
  `registry/inventory.rs`; other hits are each API's own definition, doc
  examples, and single-API unit tests.
- adopted default: no `aliases` field. Phase 1 found "exact kebab keys, no
  aliases" and none exist to preserve; an alias is a later field addition
  (source: spec R34 "preserve existing selection aliases").
- adopted default: catalog order (Phase 1 ruling) is the `--api` help order,
  so `eversolo` moves from after `elevenlabs` to after `emqx-bearer` in the
  help/error text. No artifact changes.
- adopted default: `generate_batch` keeps taking REST APIs (15 fixture call
  sites rely on it) and packages the shipped WebSocket entries; a fixture
  WebSocket entry is driven through `prepare_rust_output` (source: spec P10-e
  "tests supply a local fixture inventory to consumers").
- adopted default: the generated `define_api` doc is `Builds the {summary}.`
  from the registration's summary; the five shipped WebSocket modules are
  byte-identical (drift tests pass).

### Wave 12: propagation and the strict gate (R34, R35)

Requirement → test:

| Requirement | Test |
| --- | --- |
| R34 fixture reaches registry lists (REST shared module + WS) | `definitions/tests/api_inventory.rs::a_fixture_registration_reaches_every_derived_list` |
| R34 a group member cannot be skipped silently | `api_inventory::{an_unregistered_group_member_is_refused, a_registration_naming_the_wrong_member_is_refused, a_registration_past_the_group_is_refused}` |
| R34 registry lists derive from the inventory, in order | `api_inventory::the_registry_lists_derive_from_the_shipped_inventory_in_its_order` |
| R34 entries agree with the APIs they build (module, key, names, WS modules) | `api_inventory::every_shipped_registration_agrees_with_the_api_it_builds` |
| R34 selection names, help, inputs | `gen/tests/l1/api_inventory.rs::selection_names_and_inputs_come_from_the_inventory` |
| R34 REST exports (Rust, OpenAPI, Postman) | `gen l1 api_inventory::a_fixture_rest_registration_reaches_every_rest_output` |
| R34 WS packaging (module file, lib.rs, prelude.rs, partial-run inventory, reserved names, WS validation) | `gen l1 api_inventory::a_fixture_websocket_registration_reaches_every_websocket_consumer` |
| R35 covered set = `apis_by_module()`; one reasoned skip registry | `gen l1 openapi_strict_completeness::the_strict_gate_covers_every_registered_api` |
| R35 a fixture registration joins the gate | `openapi_strict_completeness::a_fixture_registration_joins_the_strict_gate` |
| unchanged registrations → unchanged artifacts and Rust | `artifact_drift::{generated_rust_artifacts_are_up_to_date, generated_openapi_artifacts_are_up_to_date, generated_postman_artifacts_are_up_to_date}` |

The strict gate's `STRICT_GATE_SKIPS: &[(&str, &str)]` is empty (all 18
pass); `real_apis_still_pass_strict_completeness` iterates
`strict_gate_apis(registrations())`. Fidelity: F2 in-process through each
crate's public API; the REST-output test publishes real files to a temp dir.
Input robustness matrix: not applicable (no file format or configuration
reader added or changed). Docs: new "Register an API" section with the
Mermaid flow in `docs/generation/workflow.md`; stale 13-name list in
`docs/generation/generated-clients.md` replaced; `definitions/README.md`
pointer; skill `definitions.md` checklist and `gen.md` example updated.

### Gates (Phase 5)

- `just test` (schematic): 2859 passed (4 slow: the pre-existing
  `schematic-runtime` TLS builder tests), 20 skipped; fresh-output 14 passed;
  compile-fail 1 passed. No failures.
- `just lint` (schematic): first run flagged one `redundant_closure` in the new
  `definitions/tests/api_inventory.rs`; fixed, rerun clean.
- `just check-tier-coverage schematic`: 0 stranded.
- Artifacts: `artifact_drift::generated_{rust,openapi,postman}_artifacts_are_up_to_date`
  pass, so the generated Rust, OpenAPI, and Postman output is byte-identical;
  no regeneration and no `lint-schema` needed.
- Dependents: `schematic-schema` consumers (homelab, the four integrations,
  sniff, biscuit-speaks) see byte-identical generated source; no runs needed.
  `model-citizen` depends on `schematic-definitions` but uses only
  `lmstudio`/`ollama` types: `cargo check -p model-citizen` clean and its
  `scanner::` tests 61 passed.
- Scope calculator not used to skip gates; both gates ran.
- Cross-OS check: not run. No path, process, terminal, or `#[cfg]` code
  changed; new tests write only under `tempfile` dirs and read no repository
  file.

Phase 5 status: checkpoint met. One fixture entry reaches every consumer
(registry lists, selection/help/input, REST outputs, WebSocket packaging, the
strict gate); the gate's covered set is checked against `apis_by_module()`
with one reasoned skip list (empty); unchanged registrations yield unchanged
artifacts and Rust output; the common phase checks pass.

## Phase 6

Unify module and artifact names (R36, R37). Implemented directly by the
orchestrator without subagents: the change moves one function and rewires
its call sites across two crates, so its write set is not divisible. The
`os` skill was loaded first (relevant fact: case-insensitive default file
systems on macOS and Windows).

### Wave 13: one resolver (R36)

- New `schematic/definitions/src/naming.rs` owns the policy:
  `resolve_module_name(&RestApi)` and `module_name(name, explicit)` (the form
  for callers holding no `RestApi`, such as later AsyncAPI naming), plus the
  suffix rule `infer_module_path` and its unit tests, moved unchanged from
  `gen/src/inference.rs`. Order: explicit `module_path`, else drop a
  Native/Client/Service/Hub/Api/Sdk suffix, else lowercase.
- Removed (exact locations): `gen/src/inference.rs` (whole file; and
  `pub mod inference` / `pub use inference::infer_module_path` in
  `gen/src/lib.rs`); `gen/src/export/naming.rs` (the lowercase-only
  `resolve_module_name`, and its `pub mod`/`pub use` in `export/mod.rs`);
  `get_module_path` in `gen/src/output/assemble/helpers.rs` (and its
  re-exports in `assemble/mod.rs` and `output/mod.rs`).
- Call sites now on the one resolver: Rust assembler (`assemble/api.rs`,
  `lib_rs.rs`, `prelude.rs`, `output/mod.rs` grouping, selection, and file
  names), validation (`validation.rs` group check, module-path check, export
  filename check), OpenAPI writer (`export/openapi.rs`), Postman writer
  (`export/postman/mod.rs`), export grouping (`pipeline.rs::export_batch`,
  `export_groups`), and the import pipeline's definitions module name
  (`import_pipeline.rs`, previously a third lowercase-only fallback: an
  imported `PagerDutyApi` wrote `define_pagerdutyapi_api` /
  `::schematic_definitions::pagerdutyapi` while generation filed it as
  `pagerduty`).
- Registration grouping: `apis_by_module` groups by the registration's
  `module`; the inventory invariant test now asserts that equals
  `resolve_module_name` of the built API (was `module_path` or lowercase).
- New validation (cross-platform trap in the spec's risks): module names that
  differ only by letter case are an `invalid-output-filename` error in Rust,
  OpenAPI, and Postman validation; a Rust module name equal to a reserved
  file (`lib`, `shared`, `prelude`, `ws_shared`) or a WebSocket module in
  another case is refused like the exact match already was.
- adopted default: the unified resolver adopts the Rust assembler's suffix
  heuristic; OpenAPI and Postman stems follow it (source: spec P10-f).
- adopted default: the resolver lives in a new `schematic_definitions::naming`
  module rather than `registry` (source: spec ownership table, "Proposed:
  location"); `registry` holds the inventory, naming is a separate policy.
- Docs-comment drift fixed: `RestApi::module_path` doc in
  `define/src/types.rs` said "defaults to `api.name.to_lowercase()`", which
  was only the export writers' rule; it now names the resolver.

### R37: renamed artifacts

None. Before the change, every shipped API resolved identically under both
policies: the three APIs without an explicit `module_path` (Anthropic,
ElevenLabs, LmStudio) carry no variant suffix, and every other API sets one.
So no committed OpenAPI, Postman, or Rust file was renamed or removed, and no
regeneration was needed (`artifact_drift::generated_{rust,openapi,postman}_artifacts_are_up_to_date`
pass). Old-name consumer search: `grep` for `get_module_path`,
`infer_module_path`, `export::resolve_module_name`, `export::naming`, and
`schematic_gen::inference` across the repository (outside `schematic/` and
`features/` history) found nothing; no package outside `schematic/` depends
on `schematic-gen`. The only name change is for non-shipped inputs whose name
ends in a suffix: two in-crate Postman unit tests used a `TestApi` fixture and
expected `testapi.postman_collection.json`, the very divergence R36 removes;
they (and the `write_postman` doc example) now expect
`test.postman_collection.json`, matching the Rust module `test.rs`.

### Requirement → test mapping

| Behavior | Test |
| --- | --- |
| resolution order: explicit wins, suffix inferred, fallback lowercased | `definitions` unit `naming::tests::{module_name_prefers_an_explicit_path_over_inference, module_name_infers_a_variant_suffix, module_name_falls_back_to_the_lowercased_name}` plus the moved `infer_module_path_*`/`split_camel_case_*` tests and 3 doctests |
| R36 `PagerDutyApi`-style names: Rust, OpenAPI, Postman use one stem; shared group (`PagerDutyApi` + `PagerDutyClient`, no explicit path) is one module in all three; fallback and explicit in the same batch; no `pagerdutyapi*` file | `gen l1 module_naming::every_writer_files_an_api_under_the_one_resolved_stem` (F2: real `generate_batch` publishing to a temp dir, exact file sets) |
| import names the definitions module with the resolver (inferred and explicit) | `module_naming::import_names_the_definitions_module_with_the_shared_resolver` (F2: `run_import` writing a real `mod.rs`) |
| case-insensitive stem uniqueness, every writer; nothing published; control | `module_naming::module_names_differing_only_by_case_are_refused_by_every_writer` |
| reserved/WS file names in another case | `module_naming::a_module_name_cannot_take_a_generated_file_in_another_case` |
| R36 real APIs: committed file names unchanged (OpenAPI, Postman, Rust `schema/src` entries) and each is a resolved stem | `module_naming::every_shipped_artifact_is_named_by_the_resolver` (reads `manifest_dir!().join("../openapi" / "../postman" / "../schema/src")` inline) |
| registered modules equal the resolver; unique ignoring case; portable `[a-z0-9_]` | `definitions/tests/api_inventory.rs::{every_shipped_registration_agrees_with_the_api_it_builds, shipped_module_names_are_unique_ignoring_case_and_portable}` |
| R37 / generated Rust unchanged | `artifact_drift::generated_{rust,openapi,postman}_artifacts_are_up_to_date` |

Regression proof: before this change, the import test would see
`define_pagerdutyapi_api`, the case test would see no diagnostic (no check
existed), and the two Postman unit tests failed in the first gate run on the
old expectation (`testapi.…`), confirming the writer now follows the Rust
stem. Input robustness matrix: not applicable (no file-format or
configuration reader added or changed; naming is a pure function of the
definition). New tests are L1, declared in `gen/tests/l1/main.rs`
(`mod module_naming`) and the existing `definitions` test targets; no
tier marker in any name.

### Gates (Phase 6)

- `just lint` (schematic): clean.
- `just test` (schematic): first run stopped at the 2 Postman unit tests
  above (expectation updated); rerun 2868 passed (4 slow: the pre-existing
  `schematic-runtime` TLS builder tests), 20 skipped; fresh-output 14 passed;
  compile-fail 1 passed.
- Doctests: `cargo test --doc -p schematic-definitions naming` 3 passed;
  `cargo test --doc -p schematic-gen export::postman` 4 passed.
- `just check-tier-coverage schematic`: 0 stranded.
- Dependents: generated Rust is byte-identical (drift tests), so the
  `schematic-schema` consumers see no change. No package outside
  `schematic/` depends on `schematic-gen`. `schematic-definitions` only
  gained a module; `cargo check -p model-citizen` clean.
- Scope calculator not used to skip gates; both gates ran.
- Cross-OS check: not run. The only path-adjacent change is a pure string
  comparison (case folding of module names); no filesystem, process, or
  `#[cfg]` code changed, and the new tests write only under `tempfile` dirs.

Phase 6 status: checkpoint met. Every writer resolves the same module/stem
through one definitions-owned function; no real filename changed (listed
above as none); no stale artifact survives; generated Rust is unchanged; the
drift and common phase checks pass.

## Phase 7

Generate module feature flags (R29–R32). Implemented directly by the
orchestrator without subagents: the write set (manifest generator, `lib.rs`
and prelude assemblers, the regenerated crate, and the tests) is one
integrated change, and dependent builds were serialized. The `os` skill was
not needed: no path, process, or `#[cfg(target_os)]` code changed.

### Wave 14: Feature generation (R29, R32)

- `gen/src/cargo_gen.rs`: new `ModuleFeatures::new(registry, registrations)`
  holds the resolved REST module groups of the registry and the WebSocket
  modules of the registrations, which are the same sets `lib.rs` declares.
  The manifest gains a `[features]` table: `default` lists every module,
  sorted; each module is a feature named for it; REST features enable
  nothing; each WebSocket feature enables
  `dep:futures-util`, `dep:thiserror`, `dep:tokio-tungstenite`
  (`WEBSOCKET_DEPENDENCIES`), which are now `optional = true`. With no
  WebSocket module those three dependencies are left out. A feature name TOML
  cannot leave bare is quoted. `generate_cargo_toml`,
  `generate_cargo_toml_with_paths`, and `write_cargo_toml` take
  `&ModuleFeatures`.
- `output/mod.rs`: `prepare_rust_output` builds the manifest from
  `request.registry` (the whole registry, never the selection), so a partial
  run writes the same manifest as a full run (R32).
- `assemble/lib_rs.rs`: each `pub mod` is `#[cfg(feature = "<module>")]`;
  `ws_shared` is `#[cfg(any(<every WS feature>))]` and carries
  `#[cfg_attr(not(all(<every WS feature>)), allow(dead_code))]`. Finding:
  without that allow, a one-WebSocket build (`elevenlabs_ws`) warned about 5
  correlation helpers (`request_id`, `PendingGuard`, …) that only some WS
  modules use; the default build still checks dead code fully. Each REST and
  WS doc-table row is `#![cfg_attr(feature = "<module>", doc = "…")]`, and a
  short "Features" section was added to the generated crate docs.
- `assemble/prelude.rs`: each re-export and its doc-list line are gated on
  the module's feature.
- `validation.rs`: a module path `default` is an `invalid-module-path` error
  (it would name the generated crate's `default` feature). Cargo was checked
  to accept a feature sharing a name with a non-optional dependency, so no
  other name needed refusing.
- The standalone import output (`assemble_lib_rs_with_options`,
  `generate_and_write_standalone`) is left ungated: it writes no manifest,
  so gating it would make every imported module disappear.
- Regenerated with `just generate`: `schema/Cargo.toml`, `schema/src/lib.rs`,
  `schema/src/prelude.rs` changed; OpenAPI and Postman are unchanged. rustdoc
  of a two-feature build (`cargo doc --no-deps --no-default-features
  --features anthropic,elevenlabs_ws`) is warning-free and its tables hold
  only the enabled rows.
- adopted default: one feature per resolved module group, named for the
  module, default = all, no `all-apis` umbrella (source: spec P10-d).
- adopted default: WebSocket-only dependencies are `futures-util`,
  `thiserror`, `tokio-tungstenite` (each is used only by `ws_shared.rs` or WS
  modules in the generated source). `schematic-define` is also used only by
  WS modules but stays non-optional: `schematic-runtime` and
  `schematic-definitions` depend on it anyway, so making it optional changes
  nothing (source: R30 "Dependencies still needed by the shared runtime may
  remain enabled").
- adopted default: feature modules come from the registry passed to the run
  (production: `rest_apis(registrations())`) plus the registrations' WS
  entries, not from `registrations()` REST `module` fields alone. They are
  equal for shipped data (Phase 6 invariant), and fixture registries
  (fresh-output and partial-generation tests) need their own modules
  declared (source: spec P10-d "generated from the full inventory").
- Observation for review, not changed: `tokio-stream` is declared in the
  generated manifest but no generated file uses it.
- Observation for Phase 8: the crate docs' Quick Start example and prelude
  example are `text` fences naming one API; when Phase 8 makes them compile,
  they need the named module's feature (or a gate).

### Wave 15: Feature and dependent evidence (R30, R31)

- R30, `module_features::slow_feature_selections_compile_only_their_module_groups`
  (F3, fresh-output mechanism: real 18 REST + 5 WS APIs generated outside the
  checkout, offline and locked). For each of no features, `anthropic` (one
  REST), `elevenlabs_ws` (one WS), `ollama` (shared group), and
  `anthropic,elevenlabs_ws` (pair): `cargo check --lib` succeeds with no
  `schematic-schema` warning; a `present` probe of the enabled items
  (module paths and prelude re-exports, `ws_shared::WsError` for WS)
  compiles; one probe per disabled item fails with
  `unresolved import schematic_schema::<path>`; `cargo tree -e normal` holds
  `tokio-tungstenite` exactly when a WS module is selected. 16 s warm.
  `FreshCrate::cargo_offline` was added for feature arguments and
  `cargo tree` (which takes no `--target-dir`).
- Manual cross-check on the committed crate before the test existed: the
  same selections built clean, and the REST-only tree had no tungstenite.
- `just lint-schema`: clean. `just test-schema`: 152 passed.
- R31 dependents: none of the seven `schematic-schema` dependents sets
  `default-features = false` (grep of their manifests); `model-citizen`
  depends only on `schematic-definitions`. No dependent source or manifest
  was changed.
  - `cargo build -p sniff -p biscuit-speaks -p model-citizen -p homelab -p eversolo-integration -p arcam-amp-integration -p sony-receiver-integration -p unfolded-integration-helper --all-targets`: success, no warnings.
  - `just test model-citizen homelab eversolo-integration arcam-amp-integration sony-receiver-integration unfolded-integration-helper`: 422 passed, 8 skipped (the selectors take whole areas, so the homelab CLI and server ran too).
  - sniff (2141 tests) and biscuit-speaks (441) exceed the 200-test threshold, so only the tests over the code that uses `schematic_schema` ran: `cargo nextest run -p sniff -E 'test(/remote/)'` 75 passed (the `remote::{github,gitlab,gitea,bitbucket,envelope}` users); `cargo nextest run -p biscuit-speaks -E 'test(/elevenlabs/)'` 48 passed.
  - With default features every `cfg` is true, so the code dependents compile is the same as before; the WS-only dependencies resolve to the same versions and features.

### Requirement → test mapping

| Behavior | Test |
| --- | --- |
| R29 one feature per module group, default = all, sorted; shared group is one feature | `cargo_gen::tests::every_shipped_module_group_is_one_default_feature`; `a_manifest_without_websocket_modules_declares_no_websocket_dependency` (PagerDutyApi + PagerDutyClient → one `pagerduty`) |
| R29 WS features enable exactly the WS-only deps; REST features enable nothing | `cargo_gen::tests::websocket_features_enable_exactly_the_websocket_only_dependencies` |
| R29 only WS-only deps are optional | `cargo_gen::tests::only_websocket_dependencies_are_optional` |
| no WS module → no WS deps; empty inventory → `default = []`; non-bare name quoted | `cargo_gen::tests::{a_manifest_without_websocket_modules_declares_no_websocket_dependency, an_empty_inventory_has_an_empty_default_feature, a_feature_name_toml_cannot_leave_bare_is_quoted}` |
| committed manifest is the generator's | `cargo_gen::tests::default_manifest_matches_the_committed_schema_manifest`; `artifact_drift::generated_rust_artifacts_are_up_to_date` (lib.rs, prelude.rs) |
| every cfg in lib.rs/prelude.rs names a declared feature and every module is gated; doc rows and re-exports gated | `module_features::every_feature_the_generated_code_names_is_a_declared_module_feature` (F2, real `prepare_rust_output` publish) |
| R32 partial run writes the full-inventory manifest; unselected siblings and WS keep features; lib.rs unchanged | `module_features::a_partial_run_writes_the_manifest_of_the_whole_registry` (stale owned manifest overwritten by a one-API partial run, compared byte-for-byte with the full run) |
| R30 selections compile, disabled items absent, transport gated | `module_features::slow_feature_selections_compile_only_their_module_groups` |
| module path `default` refused | `validation::tests` table case "module path naming the default feature" |
| R31 dependents unchanged | commands above |

Regression proof: before this change the manifest had no `[features]`
table (every new `cargo_gen` and `module_features` fast test fails on the
missing table), and every absent-item probe would have compiled. The first
slow run failed on my own probe check (an unused-import warning summary
matched the failure pattern); the check now matches only
`could not compile … (test "present")`. Input robustness matrix: not
applicable (no reader of a file format or configuration was added or
changed; the manifest is written, not read). New tests are L1: the fast
ones in `gen/tests/l1/module_features.rs` (declared `mod module_features` in
`main.rs`) and `cargo_gen` unit tests; the `slow_` test runs in
`just test-fresh-output` (part of `just test`) in the
`schematic-fresh-output` group.

### Gates (Phase 7)

- `just lint` (schematic): clean.
- `just test` (schematic): 2876 passed (4 slow, the existing
  `schematic-runtime` TLS tests), 21 skipped; fresh-output 15 passed
  (including the new slow test); compile-fail 1 passed. No failures, no
  reruns.
- `cargo test --doc -p schematic-gen cargo_gen`: 3 passed.
- `just check-tier-coverage schematic`: 0 stranded.
- `just lint-schema`, `just test-schema`: clean, 152 passed.
- Scope calculator not used to skip gates; both gates ran.
- Cross-OS check: not run. The change is manifest text, `#[cfg(feature)]`
  attributes, and string tests; no filesystem path, process, or OS-specific
  code changed. The new slow test reuses the existing fresh-output mechanism,
  whose OS handling is unchanged.
- Note: Phase 6's changes are still uncommitted in this worktree, so the
  working tree holds both phases; the frontmatter lists only Phase 7 files.

Phase 7 status: checkpoint met. Selected builds expose exactly their groups,
features are additive, the WebSocket transport is gated, full and partial
generation write the same manifest, all eight dependents build and pass
unchanged, and the common phase checks pass.

## Phase 8

Compile honest generated documentation (R38–R41, R51). Implemented directly
by the orchestrator without subagents: the write set (four generator
emitters, the regenerated crate, the recipe, and the tests) is one integrated
change. The `os` skill was not needed: no path, process, or
`#[cfg(target_os)]` code changed.

### Measured before the change (HEAD `b0721ec6c`, committed `schema/src`)

- 566 doc fences, all `text`: 539 request-type examples, 13 module
  examples plus 12 base-URL examples (the three combined modules had none), the
  crate Quick Start, and the prelude's import and example fences.
- 539 `///;` lines: `generate_doc_example` pushed the statement's `;` as its
  own doc line (`lines.push(";")`, missing the leading space).
- Request examples did not compile: optional setters were
  `.with_x(/* value */)`; form bodies used `Form { // ... ..Default::default() }`.
- Module and Quick Start examples used `#[tokio::main]` (the generated crate's
  `tokio` has no `rt-multi-thread`), passed an unbound `body` for a body
  endpoint, and did not import enum argument types. The prelude example
  `await`ed outside an async fn and named `OpenAI` unconditionally.
- R39 `client.list_models()`: the method exists now —
  `2026-10-12-schematic-endpoint-methods` generates one method per endpoint
  (Phase 1 D1). Every `client.<m>(` in the old docs named an existing method;
  the R39 symptom was already gone upstream.
- R40 `AuthStrategy::ApiKey` literal: no generated file contains one. The only
  copies of the old literal-bearing doc examples are in
  `gen/src/codegen/api_struct/{constructors,variant,fields,request_method}.rs`,
  which no `mod` declares (orphaned, never compiled). Left in place as
  out-of-scope cleanup; logged for review.
- R51: the three combined modules (`artificial_analysis`, `emqx`, `ollama`;
  the spec named emqx, Phase 1 found ollama, `artificial_analysis` also shares
  one) lost every member's `## Authentication`, `## Features`, `## Examples`.

### Wave 16: Documentation generation (R38–R41, R51)

- `codegen/request_structs/mod.rs`: `generate_doc_example` emits a
  `no_run` fence; the `;` ends the last statement line; every optional query,
  header, and cookie parameter gets a setter value of its type
  (`setter_example_arg`/`named_param_example`: `"x_value".to_string()`, `1`,
  `1.5`, `true`, the first enum variant, `vec![..]`), and its enum is
  imported; the body is `let body: T = Default::default();` with a bare form
  type imported from the module.
- `codegen/endpoint_methods/mod.rs`: `example_invocation` became
  `example_call(api, endpoint) -> ExampleCall`, which renders the method call
  and the typed send `client.send({Request}::new(..))` from the same
  arguments the endpoint method passes to `new`/`default`, and lists the
  request type and enum argument types to import.
- `codegen/module_docs.rs`: `ModuleDocBuilder::markdown()`; the example
  section uses the shared `client_example_lines` (prelude + request import,
  async wrapper in hidden `#` lines, method call, typed send, `.full()`;
  a body endpoint binds a default body and clones it until its last use);
  base-URL example is `no_run`.
- `output/assemble/api.rs`: combined module docs open with
  "Generated API clients for [`A`] and [`B`], which share this module." and
  give each API a `## {Api}` section holding its own `markdown()` with
  headings demoted outside code fences (hidden `# ` doctest lines are left
  alone).
- `output/assemble/lib_rs.rs`, `prelude.rs`: the Quick Start (heading
  included) and the prelude `## Examples` are the same client example of
  `example_api` (first API with a GET endpoint), each line gated
  `#![cfg_attr(feature = "<module>", doc = ..)]`. The prelude's hard-coded
  `openai::{ListModelsCall, …}` import fence was removed (its prose kept).
  The standalone import assembler emits the Quick Start ungated.
- Regenerated with `just generate`: only `schema/src/**` changed (577 `no_run`
  fences, 0 `text`, 0 `///;`); OpenAPI and Postman unchanged.
- No endpoint wrappers were added.
- adopted default: P10-g / D53 stays recorded as superseded upstream (Phase 1
  adopted default); the docs show the endpoint method first and
  `client.send(Request::new(..))` as the typed alternative, as R39 asks
  (source: contracts D1, Phase 1 log).
- adopted default: the gated crate and prelude examples drop out entirely
  (heading included) when the example API's module is disabled, rather than
  falling back to another API (source: R38 "docs do not import disabled
  modules"; simplest rule).

### Wave 17: Doctest integration (R38)

- `schematic/justfile`: new `doctest-schema` (`cargo test --doc
  --manifest-path schema/Cargo.toml`, then the same with
  `--no-default-features --features emqx`); `just doctest` calls it. Results:
  577 passed (default, 12 s warm), 78 passed (`emqx` alone, which leaves out
  the gated anthropic Quick Start and prelude example). Manual extra
  selections: `anthropic` 8 passed, `elevenlabs_ws` 0 (no REST docs) passed.
- `generated_docs::slow_generated_doc_examples_compile_with_default_and_selected_features`
  (F3, fresh-output crate of the real 18 REST + 5 WS APIs, offline and
  locked): `cargo test --doc` default and `emqx`-only both succeed with a
  non-zero passed count. 23 s warm; in the `schematic-fresh-output` group.
- `just lint-schema`: clean. `just test-schema`: 152 passed.
- Observation, not changed: `RUSTDOCFLAGS=-D warnings cargo doc` on the
  committed crate reports `rustdoc::bare_urls` for URLs inside definition
  descriptions (artificial_analysis, openai) in `lib.rs` table rows (present
  before this phase) and now also in the combined `artificial_analysis` member
  sections, which repeat each member's description. Fixing them is a
  definitions text change; plain `cargo doc` only warns.

### Requirement → test mapping

| Behavior | Test |
| --- | --- |
| R38 every generated fence is `no_run` (fresh real output) | `generated_docs::the_shipped_crate_docs_hold_only_compilable_examples_of_existing_methods` (F2, `generate_and_write_all` into a temp dir) |
| R38/R40 examples compile, default and a selected module | `generated_docs::slow_generated_doc_examples_compile_with_default_and_selected_features` (F3); `just doctest-schema` on the committed crate |
| R39 every `client.<m>(` exists on the module's client or `api_client!` | same fast test (per module; crate root/prelude against all modules) + `generated_docs::the_client_call_scan_reports_a_method_the_module_lacks` (scanner control) |
| R41 no `///;` / `;`-only doc line | same fast test; `request_structs::tests::generates_example_with_default_for_no_required_fields` (`;` on the statement line) |
| optional setters get typed values, enum imported, example parses | `request_structs::tests::optional_parameters_get_setter_calls_with_values_of_their_types` |
| body example: typed default body, bare type imported | `request_structs::tests::generates_example_with_body_construction`, `qualified_and_container_body_examples_are_valid_rust` |
| typed send mirrors the endpoint method's request construction | `endpoint_methods::tests::{example_typed_send_builds_the_request_the_endpoint_method_builds, typed_path_arguments_follow_the_template_with_their_declared_types}` |
| module example: no_run, method + typed send, no `tokio::main`; body bound and cloned | `module_docs::tests::{the_example_is_a_no_run_doctest_calling_the_method_and_the_typed_send, an_example_with_a_body_binds_a_default_body_and_clones_it_until_its_last_use}` |
| R51 each combined module (emqx, ollama, artificial_analysis) has a section per member with auth/features/examples, every endpoint, its `::new()`; emqx Basic `EMQX_API_KEY`/`EMQX_API_SECRET`, Bearer `EMQX_TOKEN` + `Login` | `generated_docs::a_shared_module_documents_every_member_api` |
| heading demotion skips fenced hidden lines | `output::assemble::api::tests::demoted_headings_leave_hidden_doctest_lines_alone` |
| committed crate is the generator's | `artifact_drift::generated_rust_artifacts_are_up_to_date` |

Regression proof: with this phase's `gen/src` changes reverted
(`git diff` saved, `git checkout gen/src`, run, `git apply`), both new fast
`generated_docs` tests fail — the fence check at `anthropic.rs:23` (a `text`
fence) and the R51 check at "artificial_analysis: no section for
ArtificialAnalysisData" — and pass with the change. The R41 assertion runs
after the fence assertion in the same test; the old output's 539 `///;` lines
are recorded above. Input robustness matrix: not applicable (no reader of a
file format or configuration changed; docs are written, not read). All new
tests are L1 in `gen/tests/l1/generated_docs.rs`, declared `mod
generated_docs` in `main.rs`, and unit tests in their modules; the `slow_`
test runs in `just test-fresh-output` (part of `just test`).

### Gates (Phase 8)

- `just lint` (schematic): clean (one clippy `match` → `if let`
  finding in the new `client_example` fixed before the clean run).
- `just test` (schematic): 2884 passed (4 slow, the existing
  `schematic-runtime` TLS tests), 22 skipped; fresh-output 16 passed
  (including the new slow doctest test, 94 s for the group); compile-fail 1
  passed. No failures, no reruns.
- `just check-tier-coverage schematic`: 0 stranded.
- `just lint-schema`: clean. `just test-schema`: 152 passed.
  `just doctest-schema`: 577 + 78 passed.
- Dependents (only doc comments of `schematic-schema` changed, so their
  compiled code is unchanged): `cargo build -p sniff -p biscuit-speaks -p
  model-citizen -p homelab -p eversolo-integration -p arcam-amp-integration
  -p sony-receiver-integration -p unfolded-integration-helper --all-targets`
  succeeded with no warnings; scoped `sniff` `test(/remote/)` 75 passed,
  `biscuit-speaks` `test(/elevenlabs/)` 48 passed. No dependent source or
  manifest changed.
- Scope calculator (`git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [schematic-gen, test-toolkit, darkmatter, schematic-define]`,
  `change_class: package`; not used to skip anything, both gates ran.
- Cross-OS check: not run. The change is doc text emitted by the generator,
  a bash `just` recipe that only calls `cargo test --doc`, and tests that read
  generated files through `Path` APIs; no filesystem path handling, process,
  or OS-specific code changed. The new slow test reuses the fresh-output
  mechanism, whose OS handling is unchanged.

Phase 8 status: checkpoint met. Real Rust examples compile under the default
features and a selected module, every shared module documents each member
API, and no missing method, incomplete literal, or `///;` line remains.
Generated source changes come only from generator changes (`just generate`).

## Phase 9

Implementer: claude/opus (Opus 5.5), 2026-10-23, macOS host. Phase 8's
changes were still uncommitted when this phase began; this phase's files are
listed separately in `source_files_during_phase_9`.

### Wave 18: Schema example engine

- New `schematic-gen` module `export::examples` (`ExampleSource`,
  `ExampleError`). It reads `RestApi::schemas`, the original registered
  components (not their OpenAPI conversion), rewrites OpenAPI 3.0
  `nullable: true` into a `null` type member (the Phase 1 spike's finding),
  and registers the whole component document once per API in a prepared
  `jsonschema::Registry` (Draft 2020-12, offline). Validators reference any
  subschema by `https://schematic.invalid/components#<pointer>` and are cached
  per pointer.
- P10-c order: first `examples` entry, `example`, `default`, `const`, each
  `enum` value, then a typed placeholder (`"string"`, `0`, `false`, format
  placeholders such as `2000-01-01T00:00:00Z`, moved into
  `minimum`/`exclusiveMinimum`/`maximum`/`multipleOf`/`minLength`/`maxLength`).
  Every authored candidate is validated before use. Objects hold required
  properties plus those declaring an example/default; `allOf` members merge
  (then the merged schema is built in full, including its own `oneOf`);
  `oneOf`/`anyOf` branches are tried in order and accepted only when the
  value validates against the whole schema. A component expanding inside
  itself more than three times fails; optional properties drop the failure
  and nullable members fall back to `null`. The final value is validated
  against the reference's own schema; any failure is an `ExampleError` with
  the schema path, never an emitted value.
- Probe over every JSON reference of every shipped API (644 references):
  first run 12 failures (inline `$ref`s containing `<`/`>` not
  percent-encoded; an `allOf` wrapper around a `oneOf` enum skipped the
  composition), both fixed; second run 0 failures in 0.67 s (debug).
- adopted default: `jsonschema` 0.55 (`default-features = false`) moves from
  `schematic-gen` dev-dependencies to dependencies, because validation
  happens in the generator before emission, not only in tests (source: plan
  P10-k "Reuse jsonschema … already present"; no new crate).
- Finding: the Draft 2020-12 validator rejects OpenAPI 3.0's boolean
  `exclusiveMinimum` at compile time, so such a schema yields a diagnostic
  (test `a_schema_the_validator_cannot_compile_is_a_diagnostic`). No
  registered schema uses it (schemars emits 2020-12 bounds).

### Wave 19: Postman mappings

All three tasks were implemented directly by the phase agent (no
subagents): the request, environment, and response builders share
`postman/item.rs`, `postman/types.rs`, and the new `Collected` sink, so a
disjoint split would have raced on them.

- **Request bodies (R19).** `item.rs` builds a JSON body from
  `ExampleSource::example(reference)`, pretty-printed. A failure leaves `raw`
  empty (never `{}`) and records an `example-value` warning (new
  `Feature::ExampleValue`). `validate_postman` adds
  `export::postman::export_diagnostics(api)`, which builds the single-API
  collection once more to collect them, so one code path decides both the
  emitted value and its diagnostic.
- **OAuth2 (R22).** New `ExportAuth::OAuth2 { config, client_id_var,
  client_secret_var }`. Both `map_auth` (API `AuthStrategy::OAuth2`, which
  previously exported as `bearer`) and `map_method` (endpoint policies) map
  to it. Postman `oauth2` attributes: `grant_type`
  (`authorization_code_with_pkce` / `client_credentials`), `authUrl`,
  `accessTokenUrl`, `challengeAlgorithm: S256`, space-joined `scope`,
  `clientId`/`clientSecret` variables (no secret for a public client),
  `client_authentication` (`header` for `ClientSecretBasic`, else `body`),
  `addTokenTo: header`. Device code: Postman has no such grant, so it carries
  `accessTokenUrl`, `scope`, and `accessToken: {{oauth2AccessToken}}`, and
  the Postman output now reports an `oauth2-grant` warning. Revocation URLs
  have no Postman attribute (logged, not exported).
- **Text and binary (R23).** `ExportBody::Binary` now carries its content
  type. Text is `raw`/`text` and binary is `file` mode, each with a
  `Content-Type` header unless a fixed header already sets one.
- **Headers, query, cookies (R24).** Values are collection variables named
  `{location}_{name}` (`path_owner`, `query_q`, `header_X-Trace`,
  `cookie_session`; non-`[A-Za-z0-9_-]` → `_`).
  - adopted default: the location prefix is the collision rule: a parameter
    variable can never equal a base-URL or credential variable (source:
    plan Wave 19 "Respect … collisions"; the spec names no scheme).
  - Optional query entries and headers are `disabled` (ParamDef has no
    example/default field, so no optional parameter is ever valued).
  - Precedence: fixed headers win over same-named header parameters;
    `Authorization` (bearer/Basic/OAuth2), the API-key header, and `Cookie`
    are excluded from header parameters; a query or cookie parameter that
    the API key uses is excluded.
  - Cookies merge into ONE `Cookie` header joined by `; ` (the runtime's
    `cookies.join("; ")` in `runtime/src/client.rs`). It holds the required
    cookies and its description lists the optional ones; when every cookie
    is optional it holds them all and is disabled. A fixed `Cookie` header
    wins.
    - adopted default: optional cookies are named in the description rather
      than enabled, so one header stays the only cookie entry (source: R24
      "Merge cookies into one Cookie header" + "Disable optional
      parameters").
  - Known gap, logged: values are Postman variables, so the runtime's
    percent-encoding of cookie values cannot be applied; the docs tell users
    to enter encoded values.
- **GraphQL (R55).** `postman/graphql.rs`: one `POST` per operation in a
  `GraphQL` folder, URL `{{graphqlUrl}}` (member-numbered), valued with the
  address derived by the surface's rules from the base URL. The derivation
  mirrors `schematic_runtime::graphql::derive_address`: whole segments,
  trailing slash ignored, first matching rule. The body is `{query,
  operationName?, variables}`, with `variables` from the example engine or
  `{}` for `Unit` (the spec's R55 wording; the runtime omits `variables` for
  `Unit`, D-27; Postman sending `{}` is harmless). A saved `200` holds
  `{"data": <example>}` plus declared HTTP errors. No address match produces
  a `graphql-operation` warning and leaves the operations out. The Postman
  `graphql_skips` warning was removed; OpenAPI keeps its per-operation skip
  warning (verified by test).
- **Environments (R20).** `postman/environment.rs`, from the built
  collection: one value per collection variable in collection order. Base
  URLs and GraphQL addresses are valued; credentials and `secret`
  parameters are typed `secret` and empty; nothing is read from the host.
  `GenerateOpts::postman_environment` / CLI `--postman-environment` (the
  explicit optional switch). The publisher counts `*.postman_environment.json`
  as owned only when requested, so a run without the switch never deletes
  them. The justfile `generate`, `generate-one`, and `generate-postman`
  recipes pass the switch, so environments are committed and drift-checked.
  - adopted default: shared-module members get their own base-URL variable
    (`baseUrl`, `baseUrl2`, … even for an equal URL) and API-name-suffixed
    credentials (`username_EmqxBasic`, `bearerToken_EmqxBearer`); a shared
    module therefore has collection-level auth only when no member
    authenticates (source: R20 "distinct server/auth variables for members
    of shared-module collections"). Endpoint-override variables get the
    member suffix after the endpoint suffix.
- **Saved responses (R21).** `postman/responses.rs`: one per success shape,
  then one per effective error entry.
  - Success status: the first declared status, else 204 for an empty shape
    and 200 otherwise.
  - Error ranges are saved under the lowest status that `ErrorTable::lookup`
    still resolves to that entry, so narrower overrides keep theirs; a fully
    shadowed range is not saved.
  - Bodies: JSON (with a `json_tag` field applied and re-validated), text
    `string`, a binary placeholder `<binary {media} body; not representable
    as an example>`, none for empty shapes.
  - Streams: SSE is one item frame plus its terminator (`data: [DONE]`;
    `event: name` with `data: {}` when consumed, or the item when `keep`; an
    item with the end field set). NDJSON is one item line plus a field-value
    terminator line. Bytes uses the binary placeholder.
  - Response `Content-Type` is the shape's media type.
- Removed: the Postman `schema-composition` warning and
  `Feature::SchemaComposition` (compositions are now exemplified or
  reported as `example-value`), `vocabulary::composition_keyword`, and the
  placeholder `postman/examples.rs`.

### Wave 20: Postman integration

- Regenerated with `just generate`. 15 collections changed and 15
  environments were added; `schema/src` and `openapi/` are unchanged by this
  phase (Rust and OpenAPI generation read no Postman code; regenerating
  after the OAuth2 fix left the Postman tree's md5 identical). No shipped API
  uses OAuth2, so that fix changes no committed artifact.
- Generation reported no `output=postman` warning: every shipped JSON
  request, saved response, stream frame, and error body has a valid
  generated example. There are no authored examples and no accepted gaps.
- Three representative generated examples, for author review:
  1. Anthropic `CreateMessage` request body:
     `{"max_tokens": 0, "messages": [], "model": "string"}`; its SSE saved
     response is `data: {"message":{…},"type":"message_start"}` followed by
     `event: message_stop` / `data: {}`.
  2. GitHub `GetRepository` 200 response: required fields only; nullable
     ones are `null`, strings `"string"`, and `owner` is
     `{"login": "string", …nulls}`.
  3. Ollama `Chat` NDJSON response: `{"error":"string"}` then
     `{"done":true,"error":"string"}`. The item type's first `anyOf` branch
     is the error shape, which is valid but not representative ("validate
     but mislead", spec risks).
- Observation (outside scope, logged): OpenAI `CreateChatCompletion`'s body
  example is `{}` because the imported `CreateChatCompletionRequest2`
  component registers no `required` list; that is a definition-import
  fidelity gap, not an exporter defect.

### Requirement-to-test mapping (Phase 9)

All new tests are L1 (no tier marker) in declared targets: the `schematic-gen`
lib unit tests and the `l1` binary (`postman_runnable` declared in
`tests/l1/main.rs`). Boundary: the crate's public export API and
`generate_batch` publishing to a temp dir (F2). Shipped-artifact checks read
in-process builds that `artifact_drift` proves byte-equal to the committed
files.

| Requirement | Tests |
| --- | --- |
| R19 / P10-c engine: priority order, invalid authored values skipped, bounds/formats, allOf merge + unsatisfiable allOf, oneOf exclusivity vs anyOf, unsatisfiable oneOf, depth-3 recursion (optional omitted, nullable → null, required → diagnostic), pattern → diagnostic, uncompilable schema → diagnostic, unresolved ref, wrapper refs and `<>` names, determinism | `export::examples::tests::*` (12) |
| R19 every shipped JSON request validates against the original registered schema (independent test-side validator) | `postman_runnable::every_shipped_example_validates::{15 modules}` + guard `the_validated_module_list_is_every_shipped_module` |
| R19 no shipped example gap; same bytes twice | `postman_runnable::shipped_collections_report_no_example_gaps_and_build_the_same_bytes_twice` |
| R19 failure → empty body + named diagnostic (endpoint, schema path), never invalid JSON | `postman_runnable::json_request_bodies_hold_a_validated_example_or_nothing`, `vocabulary_exports::postman_builds_composed_examples_or_reports_why_not`, `validation::tests::a_dangling_reference_names_the_api_endpoint_and_name_for_rust_and_openapi` |
| R20 optional environment switch (absent → no file; present → file), collection unchanged by the switch, every reference declared, credentials/secret params empty + `secret` | `postman_runnable::the_environment_is_optional_and_declares_every_collection_variable_with_empty_credentials` |
| R20 shipped environments mirror collections; shared-module members distinct (emqx) | `postman_runnable::shipped_environments_mirror_their_collections_with_separate_shared_module_members`, `postman::tests::grouped_members_get_their_own_server_and_credential_variables` |
| R20 environments drift-checked; committed variable completeness | `artifact_drift::generated_postman_artifacts_are_up_to_date` (now also environments), `postman_var_consistency::*`, `module_naming::every_shipped_artifact_is_named_by_the_resolver` |
| R21 success media types, statuses, variant names, empty/binary/text bodies, error range under narrower override, API-level error, each JSON body validated | `postman_runnable::saved_responses_cover_each_success_media_type_and_declared_error` |
| R21 finite streams (SSE marker, consumed named event, NDJSON field terminator, bytes) validated per frame; invalid terminator → diagnostic, no body | `postman_runnable::stream_examples_are_finite_and_end_with_their_terminator` |
| R22 OAuth2 golden: PKCE grant/URLs/scopes, client-credentials override with endpoint-suffixed variables, device code | `postman_golden::golden_oauth2_grants` (+ fixture `oauth2_grants.json`, Postman v2.1 schema-validated) |
| R23 text/binary modes and Content-Type, fixed header precedence | `postman_runnable::text_and_binary_bodies_keep_their_declared_content_type`, `postman::tests::{body_text, body_binary, body_json}` |
| R24 header/query/cookie variables, optional disabled, one merged Cookie header, constant precedence, credential exclusions (header/query/cookie API key, Authorization), all-optional cookies | `postman_runnable::parameters_become_declared_variables_with_constant_and_credential_precedence`, `output_validation::header_and_cookie_parameters_reach_every_postman_writer` |
| R24 parameter/credential variable collisions | `postman_runnable::parameter_variables_never_collide_with_server_or_credential_variables` |
| R55 GraphQL: one POST per operation, derived URL variable (ReplaceSuffix with prefix + trailing slash), body keys, typed variables example vs `{}`, data response; OpenAPI no path + one named diagnostic each; Postman no skip | `postman_runnable::graphql_operations_are_runnable_posts_and_openapi_skips_each_by_name`, `response_adapters::openapi_skips_graphql_operations_with_a_warning_and_postman_runs_them` |
| R55 address with no matching rule → diagnostic, operations left out | `postman_runnable::a_graphql_address_no_rule_derives_is_reported_and_its_operations_left_out` |

Updated for changed behavior (old assertions encoded the replaced output):
`postman::tests` (grouped variables, body/url signatures),
`postman_golden` (6 fixtures re-blessed; diffs reviewed: suffixed
member credentials, `baseUrl2`, saved responses, parameter variables),
`vocabulary_exports` (path variable values, variable keys, composition
test), `output_validation`, `module_naming`, `artifact_drift`.

Input robustness matrix: not applicable. This phase changes writers
(Postman collection/environment/example output), not a reader of a file
format or configuration.

### Gates (Phase 9)

- `just lint` (schematic): first run failed on clippy `match_overlapping_arm`
  in `responses.rs` (`200..=299` after `200`); fixed with guards; clean
  (exit 0) after the last edit.
- `just test` (schematic): first run stopped at
  `response_adapters::both_exporters_skip_graphql_operations_with_a_warning`
  (it encoded the removed Postman GraphQL skip); rewritten as
  `openapi_skips_graphql_operations_with_a_warning_and_postman_runs_them`.
  Second run: 2923 passed (4 slow, the existing `schematic-runtime` TLS
  tests), 22 skipped; fresh-output 16 passed; compile-fail 1 passed. No
  reruns of individual tests were needed and none were load-sensitive.
- `just check-tier-coverage schematic` (repo root): 0 stranded.
- `cargo test --doc -p schematic-gen export::`: 11 passed (new
  `ExampleSource` and `build_postman_environment` examples included).
- `just lint-schema` / `just test-schema`: not run; generated Rust in
  `schematic/schema/src` was not changed by this phase.
- Dependents: only `schematic-gen` changed. Its reverse dependencies are all
  inside `schematic/` (covered by `just test`). The eight external
  dependents use `schematic-schema`, whose sources this phase left unchanged,
  so no dependent build or test was needed.
- Scope calculator (`git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [schematic-gen, test-toolkit, darkmatter, schematic-define]`,
  `change_class: package` (the working tree still includes Phase 8's
  changes); not used to skip anything, both gates ran.
- Cross-OS check: not run. The change is in-memory JSON building plus file
  names joined with `Path::join`; no path comparison, process, or
  OS-specific code changed, and `jsonschema` already compiles on every CI
  environment as a dev-dependency.

### Review agenda (Phase 9)

1. Shared-module variable separation (R20 adopted default): every member
   gets its own `baseUrl{N}` and API-name-suffixed credentials, so `emqx`
   requests now carry request-level auth with `username_EmqxBasic` /
   `bearerToken_EmqxBearer`.
2. Parameter variable naming `{location}_{name}` (collision rule).
3. Optional cookies are named in the `Cookie` header description, not
   enabled, to keep one Cookie header (R24 wording).
4. GraphQL `variables: {}` for `Unit` operations follows R55 wording, while
   the runtime omits `variables` (D-27).
5. Example realism: "validate but mislead" cases such as Ollama's NDJSON
   item, whose first `anyOf` branch is the error shape. Authored `examples`
   on the definition types would fix them.
6. OpenAI `CreateChatCompletionRequest2` lacks `required` (an import
   fidelity gap), so its example is `{}`.

Phase 9 status: checkpoint met. Every emitted example validates against its
registered schema and output is deterministic. Every variable is declared,
with or without environments. OAuth2, content types, and parameters
survive. GraphQL is runnable in Postman and skipped by name in OpenAPI.
Drift passes for collections and environments.

## Phase 10

Export and persist AsyncAPI definitions (R25–R28, R56–R58). Implemented
directly by the orchestrator (no subagents): exporter and importer are one
contract and their round trip was easier to keep consistent in one hand.

### What was built

- **Exporter** `schematic-gen::export::asyncapi` (`gen/src/export/asyncapi/mod.rs`):
  `export_asyncapi`, `render_asyncapi`, `asyncapi_documents` (file naming),
  `prepare_asyncapi_output` (staged `MutationPlan` through the existing
  `output::prepare_export_output`), `dangling_refs`. AsyncAPI **3.0.0**: one
  server `default` (`host`/`protocol`/`pathname` from `base_url`), one channel
  per endpoint (`address`, `parameters`, `bindings.ws.{query,headers}`), one
  component message per `{EndpointId}.{MessageName}`, one operation per
  message and action (`{Endpoint}.send.{Message}` / `.receive.`), payload
  schema components, security scheme applied by the server.
  `responds_with` → the send operation's standard `reply`.
- New `Output::AsyncApi` (`"asyncapi"`) diagnostic output; export findings use
  `Feature::WebSocketDefinition`. Errors (not `ws(s)://`, duplicate message
  name, unknown `responds_with`, a name AsyncAPI keys cannot hold, a dangling
  `$ref`) fail with `GeneratorError::Validation`; header/cookie params and
  OAuth2 are warnings.
- **Importer** (`gen/src/asyncapi_import.rs` + `asyncapi_import/{reader,extensions,v3}.rs`):
  version dispatch (`2.x.y` → legacy reader, `3.0.y` → new reader, anything
  else ERROR); `import_asyncapi_text` (no writes) and `run_import_asyncapi`
  (persists); shared located-diagnostic reader with duplicate-key-rejecting
  loading (`UniqueValue`, copied from the OpenAPI importer because that one is
  private to `schematic-define`); any ERROR fails the import before writing.
- **Persistence (R26)**: `{module}.definition.json` (`WebSocketApi`),
  `{module}.models.json` (`ModelCatalog`), `{module}_asyncapi_import.txt`,
  planned with `prepare_export_output` and published with `MutationPlan`;
  `read_import_output` reads them back; `AsyncImportResult` gained
  `module_path`, `version`, `outputs`. A module path that is not
  `[A-Za-z0-9_]+` is a `ConfigError`.
- **CLI**: new `schematic-gen export-asyncapi --output DIR [--api NAME] [--dry-run]`;
  `import-asyncapi` prints the files it wrote.
- **Recipes/artifacts (R28)**: `just generate-asyncapi`; `just generate` also
  runs the export. `schematic/asyncapi/` holds 5 documents:
  `elevenlabs_ws.json`, `unfolded_circle_core_ws.json`,
  `unfolded_circle_dock_ws.json`, `unfolded_circle_integration_ws.json`,
  `samsung_smart_tv_remote_ws.json`.
- **Vendored schema**: `gen/tests/fixtures/asyncapi/spec-json-schemas/3.0.0.json`
  (sha256 `1786a007…7fe9a`, the Phase 1 spike's copy) with upstream `LICENSE`,
  `NOTICE` (fetched from the same commit), and `PROVENANCE.md`.

### Extension names and shapes (R56–R58, spec item 15)

| Extension | Placement | Shape |
| --- | --- | --- |
| `x-schematic-selectors` | component message | `[{"field", "value"}]`; `[]` = declared empty |
| `x-schematic-fallback` | component message | `true` |
| `x-schematic-role` | channel | `"client"` / `"host"` / `"both"` |
| `x-schematic-heartbeat` | channel | `{"interval_secs", "timeout_secs"?}` — **read back** |
| `x-schematic-param` | channel parameter / ws query property | non-default `type` (path), `style`, `explode`, `rust_name`, `secret`, `pager_managed`, `deprecated` (path) |
| `x-schematic-value-prefix` | httpApiKey scheme | string |
| `x-schematic-bearer-header` | http bearer scheme | string |

### Adopted defaults and decisions

- adopted default: AsyncAPI 3.0, API-specific documents, `{module}.json`, and
  `{module}.{snake_api}.json` when a module is shared (source: spec P10-a and
  Open Question "AsyncAPI document boundaries", option 1).
- adopted default: payload schemas are unconstrained `{}` components with a
  description naming the Rust type (source: contracts D12, Phase 1 log).
- adopted default: correlation, `auth_flow`, lifecycle roles, API runtime
  hints, and env metadata stay outside the subset (source: P10-a, D11).
- adopted default: `x-schematic-param` added (name logged here, spec R56
  "name and shape are logged"): without it Samsung's `secret` token query
  parameter and typed path parameters could not round-trip, which R27 lists
  under "parameters". Same rule for value prefix / bearer header (D57 item 22
  analogue for AsyncAPI).
- decision: one operation per message and action (not one per channel and
  action), because AsyncAPI 3.0 `reply` belongs to an operation and each
  message has its own `responds_with`.
- decision: an undeclared path placeholder exports as `{}`; a declared one
  always carries `x-schematic-param.type`; on import, a channel with any typed
  or described parameter declares all its placeholders (D17 analogue).
- deviation from the frozen matrix (contracts §3): an operation **without**
  `messages` is INFO and covers every message of its channel (the AsyncAPI 3.0
  meaning) rather than ERROR; an empty `messages` list is WARN as frozen.
- deviation: contracts §3 lists `info.description` absent as "O (today a
  placeholder)"; absent now reads as `""` for both versions, and a channel's
  absent description is `""` (was a placeholder for 2.x).
- behavior change (contracts R-S smells, matrix 3): 2.x auth now comes only
  from the server's **applied** `security`; a declared but unapplied
  component scheme is anonymous (INFO). The `maps_http_api_key_auth_scheme`
  unit test gained the `security` line it relied on implicitly. 2.x `apiKey`
  without `name` is ERROR (was `X-API-Key`); `apiKey` `in: user|password` is
  WARN and anonymous. A non-string payload `const` is INFO (was silent).
- Equality for R27 is order-insensitive (endpoints by id, messages and query
  parameters and headers by name): `serde_json` here has no `preserve_order`
  (enabling it would change every committed JSON artifact), AsyncAPI maps are
  unordered, and selection never uses declaration order.

### R27 round-trip results per API (spec item 15)

Each row is asserted by `asyncapi_round_trip::<module>_round_trips` (JSON and
YAML forms; everything outside the listed losses compares equal: channels,
messages, directions, payload type names, selectors, fallback, role, auth,
parameters, heartbeat):

| API | Listed differences | Re-import lowers to a client? |
| --- | --- | --- |
| `elevenlabs_ws` | env metadata; version → `1.0.0`; lifecycle `open`/`close` become ordinary messages (and `open_credential_field` is lost) | yes |
| `unfolded_circle_core_ws` | env metadata; version; correlation hints; auth-flow hints | **no** — `responds_with needs the endpoint's runtime.correlation hints` |
| `unfolded_circle_dock_ws` | same as Core | **no** (same reason) |
| `unfolded_circle_integration_ws` | same as Core | **no** (same reason) |
| `samsung_smart_tv_remote_ws` | version → `1.0.0` | yes |
| fixture (`common::asyncapi::fixture_api`) | env metadata only | n/a |

Schema validation of the output: **verified** offline — all five documents
and the fixture validate against the vendored AsyncAPI 3.0.0 JSON Schema
(Draft 7), and a 2.x-shaped channel under `asyncapi: 3.0.0` is rejected.
Heartbeat hints are read back (not only exported).

### Requirement-to-test mapping

All L1 in `schematic-gen` (`tests/l1`), F2 in-process at the public crate API
unless noted; CLI wiring is F3 (spawned binary).

| Requirement / behavior | Tests |
| --- | --- |
| R25 structure, ref closure, five APIs | `asyncapi_export::every_websocket_api_exports_a_closed_document`, `server_auth_and_parameters_use_standard_fields` |
| R25 determinism | `asyncapi_export::export_is_deterministic` |
| R25 schema validation | `asyncapi_export::exported_documents_validate_against_the_asyncapi_3_schema` |
| R25 refusals / lossy warnings | `asyncapi_export::definitions_that_cannot_become_a_valid_document_fail`, `unexportable_declarations_are_reported` |
| client perspective for every role | `asyncapi_export::operations_take_the_client_perspective_for_every_role`, `asyncapi_round_trip::each_role_survives_and_never_changes_directions` |
| filename rule incl. shared module | `asyncapi_export::documents_are_named_by_module_and_split_when_a_module_is_shared` |
| R56 selectors/fallback export + read-back, overlapping payloads | `asyncapi_export::schematic_metadata_is_written_as_extensions`, `asyncapi_round_trip::overlapping_payloads_are_told_apart_by_their_selectors` |
| R56 duplicate-fallback diagnostic | `asyncapi_import_matrix::fallback_extension` |
| R57 role per fixture | `asyncapi_round_trip::each_role_survives_and_never_changes_directions`, `asyncapi_import_matrix::role_extension` |
| R58 heartbeat export + read-back | `asyncapi_export::schematic_metadata_is_written_as_extensions`, `asyncapi_round_trip::heartbeat_hints_are_read_back`, `asyncapi_import_matrix::heartbeat_extension` |
| R27 per API (5) + guard | `asyncapi_round_trip::{elevenlabs_ws,unfolded_circle_core_ws,unfolded_circle_dock_ws,unfolded_circle_integration_ws,samsung_smart_tv_remote_ws}_round_trips`, `every_shipped_websocket_api_round_trips`, `the_fixture_keeps_every_subset_field` |
| base-path handshake URL (no duplicated segment) | `asyncapi_round_trip::a_base_path_assembles_the_same_handshake_url` |
| auth forms + applied security | `asyncapi_round_trip::every_auth_form_in_the_subset_round_trips`, `asyncapi_import_matrix::applied_security_decides_auth` |
| R26 persisted output read back | `asyncapi_persist::saved_output_reads_back_as_the_imported_definition_and_catalog` |
| R26 write/read/write/read round trip | `asyncapi_persist::persisted_definitions_survive_a_second_round_trip` |
| R26 dry run writes nothing | `asyncapi_persist::a_dry_run_writes_nothing` |
| R26 staged failure (conflict, failed import, unsafe module path) | `asyncapi_persist::a_conflicting_path_fails_before_anything_is_written`, `a_failed_import_writes_nothing` |
| CLI wiring (F3) | `asyncapi_persist::cli_commands_export_and_import` (control export + import, invalid protocol → located error, nothing written) |
| R28 drift + closure | `artifact_drift::generated_asyncapi_artifacts_are_up_to_date` (proven to fail on a byte edit and on a stray file) |
| 2.x compatibility retained | existing `asyncapi_import::*` (official UC 2.2.0 YAML + JSON) and unit tests; `asyncapi_import_matrix::legacy_reader_fields` |
| version dispatch | `asyncapi_import::tests::versions_dispatch_on_major_and_reject_everything_else`, `asyncapi_import_matrix::version_field` |

### Input robustness matrix (reader)

Default outcomes stated once: a malformed standard field is an ERROR at its
JSON pointer and the import fails with nothing written; a malformed
Schematic extension is a WARN and reads as absent. Exceptions are asserted
in `asyncapi_import_matrix` and listed in `schematic/docs/io/asyncapi.md`:
absent servers (WARN, `ws://localhost`), absent/empty applied security
(anonymous, INFO when absent), unsupported scheme types (WARN), empty channels
(WARN), absent payload (3.0 WARN, 2.x INFO), `{}` payload (any value), absent
`name` (channel key), absent operation `messages` (INFO, covers the channel),
empty operation `messages` (WARN), unused message (WARN, dropped), unplaced
parameter (WARN), object-typed query parameter (INFO, JSON), non-constant
header (WARN, dropped), `--api-name` replacing an absent title, absent
`x-schematic-role` (`--ws-role`), empty selector value and empty selector list
(valid), unknown extension members (INFO).
Document shapes (one loader, JSON and YAML branches):
`duplicate_keys_trailing_content_and_extra_documents_are_rejected`,
`yaml_bare_key_is_null_not_absent`. Smell grep done: the remaining
`unwrap_or_default`/`unwrap_or` uses follow an emitted ERROR or are
documented defaults; the 2.x parameter `schema.type` and non-string `const`
silent reads were fixed (ERROR / INFO).

### Gates and verification (macOS, this host)

- `just lint` (schematic): clean, after one fix (`clippy::field_reassign_with_default`
  in `channel_params`).
- `just test` (schematic): exit 0 — L1 `2972 tests run: 2972 passed (4 slow), 22 skipped`;
  fresh-output `16 passed`; compile-fail `1 passed`. The 4 SLOW tests are
  `schematic-runtime::l1 builder::*` TLS tests this phase did not touch; they
  passed while the suite was compiling (load-sensitive, not rerun).
- New/changed tests run in isolation first: `-E 'test(/asyncapi/)'` → 49
  passed; `artifact_drift::generated_asyncapi_artifacts_are_up_to_date`
  passed, and failed as expected on a one-byte edit and on a stray
  `asyncapi/stray.json` (both reverted).
- `just check-tier-coverage schematic`: stranded 0.
- Test inputs the scope calculator flagged outside `schematic-gen`:
  `cargo test --doc -p schematic-define --all-features WebSocketDefinitionsDoc`
  (the page `websocket-definitions.md` is a doctest) → 5 passed;
  `cargo nextest run -p darkmatter --test l1 -E 'test(/shipped_schema_corpus/)'`
  (reads `.claude/skills`) → 8 passed;
  `cargo nextest run -p test-toolkit -E 'binary(ci_workflow_contracts)'`
  (reads justfiles) → 178 passed.
- Dependents: only `schematic-gen` changed as a crate. Its reverse
  dependencies (`schematic-oauth`, `schematic-runtime`, the generated
  `schematic-schema`) are in this area and covered by `just test`; the
  generated Rust is unchanged, so the eight external dependents of
  `schematic-schema` are unaffected and were not rebuilt.
- Cross-OS check: not run. The change is in-memory JSON building, files
  joined with `Path::join`, temp-dir tests, and one `CARGO_BIN_EXE_` spawn; no
  path comparison, OS-specific API, or terminal behavior. Byte-exact drift of
  committed JSON follows the existing OpenAPI/Postman drift tests.
- `just lint-schema` / `just test-schema`: not run; no generated Rust changed.

### Review agenda (Phase 10)

1. Correlation hints outside the subset (D11): the Core, Dock, and
   Integration documents re-import into definitions that no longer lower to a
   client. An `x-schematic-correlation` channel extension (and `auth_flow`)
   would be additive.
2. `x-schematic-param`, `x-schematic-value-prefix`, `x-schematic-bearer-header`
   are extension names beyond the spec's proposed three; review the names.
3. 2.x auth now follows applied server security only (a declared, unapplied
   scheme no longer authenticates the import).
4. The operation-without-`messages` deviation from the frozen matrix (INFO,
   standard 3.0 meaning).
5. Unconstrained payload schemas (D12) remain an open R25 gap.

Phase 10 status: checkpoint met. All five WebSocket APIs export and
re-import (JSON and YAML); saved output is reusable and read back; no local
`$ref` dangles and no file collides; role, selector, fallback, and heartbeat
semantics survive; the reader matrix and drift pass; schema validation is
verified offline.
- Scope calculator after every Phase 10 edit (`git status --porcelain
  --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py`): `packages: [schematic-define, schematic-gen,
  test-toolkit, darkmatter]`, `change_class: package`; gates were required and
  ran (above).

## Phase 11

### Wave 24: Compile README (R42)

- `schematic/define/src/lib.rs`: new `#[cfg(doctest)] #[doc = include_str!("../README.md")] pub struct ReadmeDoc;`.
  Adopted default: the spec's "or an equivalent that makes the examples run"
  (R42), using the pattern `websocket/mod.rs` already uses for
  `websocket-definitions.md`. The crate-root `#![doc]` was not used: the
  README's H1 and `../docs/...` links are written for GitHub and would
  duplicate the crate title and break in rustdoc.
- Measured before the change: the README was not compiled; its second
  example used `Issue` and `context` without defining them, and its prose
  named `AddressRule::github()` as the GraphQL example.
- `schematic/define/README.md` rewritten around five compiled examples
  (quick start; endpoint facts with typed path params, a required query
  param, `deprecated`, and an asserted `?`-in-path build error; authentication
  policies with `.public()`, `.auth(AuthPolicy { .. })`, and a
  `ResolvedEndpointAuth::Public` assertion; success shapes with a repeated
  `.returns(..)`, SSE end event, endpoint `errors` and API `default_errors`;
  pagination with a provider-local strategy function, `default_pagination`,
  and `Collection::root().unpaginated()`). None sends a request, so none
  needs `no_run`; none is `ignore`d. The GraphQL bullet now says `rules`
  rather than naming a vendor constructor. It stays an overview and links to
  `docs/`.
- Drift proof (deliberately broken fixture): changing
  `api.endpoints[0].response.len()` to `.success.len()` in the README made
  `just _doctest schematic-define --all-features` fail with
  `error[E0609]: no field 'success' on type 'schematic_define::Endpoint'`
  and exit 101; restored, it passes (177 + 18).
- `schematic/README.md` and `schematic/definitions/README.md` contain no
  `schematic-define` examples; nothing to update there.

### Pre-existing doctest failure fixed (comment drift)

`just doctest` failed in `schematic-gen`: the `validation` module example
asserted `validate_postman(..).is_empty()` for an API with a dangling
`Item` reference. Since Phase 9 (runnable Postman examples), Postman builds
example bodies from schemas, so it now reports one `example-value`
**warning** (the topic page `docs/generation/validation.md` already says
so). The code is correct; the module doc was wrong. Fixed in
`gen/src/validation.rs`: the prose ("Postman never reads a schema") now
says Postman reads a schema only for an example body and an unresolved one
is a warning, and the example asserts every Postman diagnostic is a warning
of feature `ExampleValue`. Comment/doc-only change; no behavior change.

### Wave 24: Confirm invariants (R43, R50, R52)

This phase moves no presets, migrates no endpoint literals, and deletes no
upstream-owned tree.

- **R50 holds.** `ls schematic/gen/schematic` → no such file;
  `git ls-files schematic/gen/schematic | wc -l` → `0`;
  `upstream_hygiene::stale_nested_schematic_tree_is_absent` passed.
- **R43 holds** (same results as Phase 1).
  `rg -n '(Strategy|Pagination|PaginationStrategy|MoreSignal|Collection|Paginated)::(github|gitlab|bitbucket|gitea)\b' -g '*.rs' ..`
  → no match. `rg -n -i 'fn [a-z_]*(github|gitlab|bitbucket|gitea)[a-z_]*\s*[<(]' define/src`
  → `graphql.rs:65` `AddressRule::github()` (GraphQL address rule, outside
  R43's pagination scope, already on the Phase 1 review agenda) and the
  private test fn `metadata.rs:359`. `rg -n -i -l preset definitions/src define/src`
  → `define/src/headers/builder/tests.rs` and the generic
  `define/src/pagination/relay.rs` only; no shared vendor preset layer. The
  README now shows a provider-local strategy function and states that the
  crate ships no vendor presets.
- **R52 holds.** `rg -n '"/[^"\s]*\?[^"]*"' definitions/src` → no match.
  Passed: `output_validation::live_definitions_have_no_embedded_queries_or_validation_gaps`,
  `validation::tests::query_in_path_severity_is_an_error`,
  `validation::tests::query_in_path_is_one_error_per_output`,
  `construction_enforcement::query_bearing_paths_fail_endpoint_and_destination_api_build`,
  `definition_bundles::construction_runs_the_builder_validation`, and the new
  README assertion that `/items?limit=10` fails `build()`.

### Requirement-to-test mapping

| Requirement | Test |
| --- | --- |
| R42 README compiles (5 examples) | `schematic-define` doctests `lib.rs - ReadmeDoc (line …)` via `just doctest` (F2: real shipped README, normal doctest path) |
| R42 drift fails the recipe | broken-fixture run above (exit 101), reverted |
| R42 README claim: `?` path is an error | README endpoint-facts example assertion |
| R43 | recorded greps above |
| R50 | `upstream_hygiene::stale_nested_schematic_tree_is_absent` |
| R52 | the five tests above plus the grep |

The input robustness matrix does not apply: no reader or loader changed.

### Gates (macOS, this host)

- `just lint` (schematic): exit 0.
- `just doctest` (schematic): exit 0 — define 177 passed / 7 ignored + 18
  (pages), definitions 48 / 2 ignored, gen 19, oauth 3, runtime 3,
  schematic-schema 577 + 78. (First run failed on the pre-existing
  `validation` example above; rerun after the fix passed.)
- `just test` (schematic): exit 0 — L1 `2972 tests run: 2972 passed (4 slow), 22 skipped`;
  fresh-output `16 passed (1 slow)`; compile-fail `1 passed`. No failures,
  no reruns.
- `just check-tier-coverage schematic`: stranded 0.
- Dependents: no public API changed (`ReadmeDoc` exists only under
  `cfg(doctest)`; the gen change is a module doc), so no dependent package
  was rebuilt.
- Cross-OS check: not run; no path, process, or OS-specific code changed.
- Scope calculator after every Phase 11 edit (`git status --porcelain
  --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py`): `packages: [schematic-define, schematic-gen,
  test-toolkit, darkmatter]`, `change_class: package` (the tree still holds
  Phase 10's uncommitted work); gates were required and ran (above). Test
  inputs outside the area: `cargo nextest run -p darkmatter --test l1 -E
  'test(/shipped_schema_corpus/)'` (reads `.claude/skills`) → 8 passed;
  `cargo nextest run -p test-toolkit -E 'binary(ci_workflow_contracts)'`
  → 178 passed.

Phase 11 status: checkpoint met. The README examples compile through
`just doctest` and a drifted example fails it; R43, R50, and R52 hold; lint,
doctest, and test gates pass.

## Phase 12

Two audit subagents ran in parallel over disjoint file sets (docs/READMEs;
rustdoc + skills + command + dependency docs). Both were scoped to their own
files and ran only scoped verification; the area gates ran once, afterwards.

### Wave 25: Audit guidance (R48)

Verified against source, `--help` of every `schematic-gen` subcommand, the
justfile, and the committed `openapi/`, `postman/`, `asyncapi/` artifacts.
Drift corrected (doc said → code does):

- `schematic/README.md`: listed only OpenAPI and Postman outputs → AsyncAPI
  added to the intro and the overview diagram; links to the AsyncAPI page,
  the round-trip section, and Cargo feature selection.
- `schematic/gen/README.md`: no AsyncAPI → added, with an `export-asyncapi`
  example and links to the AsyncAPI page and "Register an API".
- `schematic/definitions/README.md`: endpoint counts were stale
  (HuggingFace 28+ → 26, ElevenLabs REST 35+ → 42, EMQX Basic 30+ → 36,
  EMQX Bearer 30+ → 38, Samsung REST 4 → 7, Samsung Remote WS 1 → 2).
- `docs/io/openapi-extensions.md`: added a Mermaid diagram of the round trip
  (export → import → export, with both comparisons) and the note that the
  comparisons are of parsed values, never bytes.
- `docs/io/export-openapi.md`: "Version Resolution" called the value the
  OpenAPI spec version → it is `info.version`; a group's shared document takes
  the first member's version.
- `docs/io/crates-openapi.md`: only the old crate survey → added "What
  Schematic uses" (`openapiv3` for export and import; own model extraction,
  no `progenitor`/`openapi-model-generator`; no AsyncAPI or Postman crate;
  `jsonschema` validates Postman examples; `TypeRef`/`SchemaComponent`).
- `docs/generation/workflow.md`: inventory example comments omitted the entry
  name, module group, and constructor arguments → fixed; added a Mermaid
  diagram of the generation pipeline (`registrations()` → Rust crate, OpenAPI,
  Postman, AsyncAPI), each output validated and published on its own.
- `docs/generation/generated-clients.md`: "three subcommands" → seven
  (`bundle`, `export-bundle`, `export-asyncapi`, `import-asyncapi` added);
  missing `--postman-environment`, `--definitions-out`, `--overlay`,
  `--exclude-path`, `--request-id-header` → added; module reference gained the
  Postman environment and AsyncAPI functions and marks
  `openapi_output`/`postman_output` as deprecated shims; the ASCII phase
  pipeline became a Mermaid diagram.
- `docs/generation/partial-generation.md`: library example omitted the
  required `registrations` field of `RustOutputRequest` → added; states that
  AsyncAPI is written by `export-asyncapi`, not `generate`.
- `docs/typed-registration.md`: "There is no separate API inventory" → wrong
  since `registry::registrations()`; now "no separate schema registry" with a
  link to "Register an API".
- Verified unchanged: `define/README.md` (compiled; no edit), `schema/README.md`,
  `docs/io/import-openapi.md`, `docs/io/export-postman.md`, `docs/io/asyncapi.md`
  (already had its round-trip diagram), remaining `docs/generation/*.md`.
- Every Mermaid block in edited files renders with `mmdc`. A grep of the
  added doc lines for `features/`, `fixes/`, dated slugs, and `R`/`D` IDs
  finds nothing.

### Wave 25: Audit contracts (R49)

Comment-only drift pass over the feature's changed hunks
(`git diff 8651df6e2 HEAD -- schematic/{define,definitions,gen}/src`);
`git diff -U0 -- '*.rs'` contains no non-comment line. Drift corrected:

- define `import/reconcile.rs`: `map_root_auth` history narration removed;
  `map_operation_auth` said inheritance needs only "no operation details" →
  also no `env_mapping`; `drop_fixed_header_parameters` cited a plan phase →
  points at `export/headers.rs`.
- define `export/conversion.rs::convert_component_schema` `## Errors` missed
  `OpenApiError::Validation`.
- define `export/parameters.rs`: inline comment said `explode: true` is
  written only for `deepObject` → every style except `form`.
- define `import/mappings/responses.rs::multipart_kind`: names the two
  reported cases (non-file array part; text part not `text/plain`).
- define `import.rs` module overview: security and parameter mappings
  restated to current behavior, including path-item parameter merge and
  `x-schematic` read-back.
- gen `postman/auth.rs::auth_variables`: claimed fixed literal keys and a
  broken `[map_auth]` link → keys come from `ExportAuth` with member and
  endpoint suffixes.
- gen `postman/responses.rs::saved_responses`: role format corrected to
  `"{name} response"`.
- gen `postman/mod.rs::build_postman_environment`: secret parameters are
  also typed `secret`, not only credentials.
- gen `postman/collection.rs::build_postman_collection_grouped`: the
  `GraphQL` folder is now mentioned.
- gen `postman/item.rs`, `export/vocabulary.rs`: stray requirement IDs
  removed (one was the wrong requirement).
- gen `export/asyncapi/mod.rs::export_asyncapi` `## Errors` gained host-less
  base URL, duplicate endpoint id, dangling `$ref`; `param_extension` defaults
  list gained pager-managed and `deprecated`.
- gen `asyncapi_import.rs::Import::payload_refs`: garbled comment rewritten.
- gen `postman/request.rs::build_url`: two HOW-narration comments deleted.
- definitions `naming.rs` `//!`: grouping reads each registration's static
  `module` (an L1 test checks it equals the resolver), not
  `resolve_module_name`; `infer_module_path` comment no longer contradicts
  itself about separators.
- definitions `registry/mod.rs` `//!`: it also holds and re-exports the
  registration inventory.
- gen `validation.rs`: `validate_openapi` docs gained the module-name case
  collision check and `validate_schema_closure`; `validate_postman` the case
  collision check; `validate_rust_client` the `default` module-name check and
  case collision check.

Guarantees agree across docs, code, and tests: strict mode (WARN or ERROR
fails, INFO passes); the auth extension carries exactly `{"bearer": true}` or
`{"value_prefix": "…"}`; one Cargo feature per module, all default, WebSocket
transport dependencies optional; example values validated against registered
schemas with a diagnostic instead of an invalid example.

Skills, command, dependency docs:

- `.claude/skills/schematic-define/SKILL.md`: verification block lists
  `generate-one`, `generate-{openapi,postman,asyncapi}`, `doctest`.
- `schematic/gen.md`: stale 9-key API list replaced by the rule (every REST
  `key` in `REGISTRATIONS`, then `all`); recipe table added.
- `schematic/definitions.md`: grouping reads the registration's `module`;
  Cargo features derive from the inventory; guarded test tables listed.
- `.claude/commands/create-schematic-api.md` (`.qwen`/`.roo`/`.opencode` are
  symlinks): registration steps are now `pub mod` plus one `rest(..)` entry in
  `REGISTRATIONS`, the guarded test tables, `generate-one <key>`.
- `schematic/docs/dependencies.md`: already current (Phase 10 recorded the
  `jsonschema` move and the definitions dev dependency); root
  `docs/dependencies.md` names no affected schematic consumer. No change.

Observations for review (not changed; new scope or pre-existing):

- **R34 "one registration step" is one source of truth, not one edit.** A new
  REST module must also be added to test tables that `include_str!` committed
  artifacts (`shipped_modules!` in `postman_runnable.rs`, `ARTIFACTS` in
  `extension_round_trip.rs`, the list in `fixed_header_export.rs`, `EXPECTED`
  in `definitions/tests/api_policy.rs`, `pagination_coverage/`). Each is
  guarded against the inventory, so an omission fails loudly. The skill and
  command now say so instead of claiming a single step.
- `generate-one` takes a registration key (`emqx-basic`), not a module name
  (`emqx` fails with "Unknown API" listing the keys). Docs use valid keys.
- `schematic/justfile` `install` runs `schematic-gen -- --api all` with no
  subcommand and `cargo check -p schematic-schema` (excluded from the
  workspace); pre-existing, outside this spec, left unchanged.
- `.claude/commands/create-schematic-api.md` still says SSE/NDJSON use binary
  success with caller framing and that future response methods are planned;
  the skill and code use `Success::sse`/`ndjson`. Predates this feature;
  "do not clean unrelated skill guidance" (plan Wave 25), left unchanged.
- Many `define` rustdocs and gen L1 test module docs cite requirement IDs, a
  few name this feature's slug; existing style, left unchanged.
- Already recorded earlier and left to Phase 13: nine Bitbucket
  `PaginatedResponse<…>` component keys contain `<` (Phase 2 deviation); the
  generated docs show per-endpoint methods (D53 recorded as superseded).
- Runtime ABI values in two doc examples (1 and 2; current 7) are
  illustrative and were left.

### Wave 26: Check regeneration (determinism)

All on the current tree, through recipes, from `schematic/`:

| Run | Result |
| --- | --- |
| `just generate` #1 (clean committed tree, before audits) | Rust 45 written / 0 deleted, OpenAPI 15 / 0, Postman 30 / 0, AsyncAPI 5 / 0; `git status` of `schema openapi postman asyncapi`: clean |
| `just generate` #2 (after the comment-only edits) | same counts; diff 0 lines |
| `just generate-one github` | 13 written, 0 deleted, 14 modules kept; OpenAPI 1, Postman 2; diff 0 |
| `just generate-one emqx-basic` (shared module group with `emqx-bearer`) | the whole `emqx` group regenerated, 14 kept; diff 0 |
| `just generate-one ollama-native` (shared group with `ollama-openai`) | the whole `ollama` group, 14 kept; diff 0 |

No file renamed, orphaned, or deleted; the full and partial runs leave the
generated manifest (`schema/Cargo.toml`, `lib.rs`, `prelude.rs`) byte-identical.
No generation edit was made, so the generated-crate checks (`lint-schema`,
`test-schema`) were not required by this phase; Phase 13 runs them.

### Requirement-to-test mapping

| Requirement | Evidence |
| --- | --- |
| R48 docs/README/skills | diff review above; `mmdc` render of every edited Mermaid block; `darkmatter` `shipped_schema_corpus` (reads `.claude/skills`) 8 passed; `just doctest` (compiles `define/README.md`) |
| R49 drift pass | comment-only diff check; `just lint` (incl. rustdoc lints); `just doctest` |
| Determinism | the regeneration table above |

No behavior changed, so no new test was added; the input robustness matrix
does not apply (no reader or loader changed).

### Gates (macOS, this host)

- `just lint` (schematic): exit 0.
- `just test` (schematic): exit 0 — L1 `2972 tests run: 2972 passed (4 slow), 22 skipped`;
  fresh-output `16 passed (1 slow)`; compile-fail `1 passed`. No failures.
- `just doctest` (schematic): exit 0 — define 177 / 7 ignored + 18, definitions
  48 / 2 ignored, gen 19, oauth 3, runtime 3, schema 577 + 78.
- `just check-tier-coverage schematic`: stranded 0.
- Dependents: no public API or generated output changed, so no dependent was
  rebuilt. Cross-OS: not run; no path, process, or OS-specific code changed.
- Scope calculator after every Phase 12 edit (`git status --porcelain
  --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py`): `packages: [schematic-define,
  schematic-definitions, schematic-gen, test-toolkit, darkmatter]`,
  `change_class: package`; gates were required and ran (above). Test inputs
  outside the area: `cargo nextest run -p darkmatter --test l1 -E
  'test(/shipped_schema_corpus/)'` → 8 passed; `cargo nextest run -p
  test-toolkit -E 'binary(ci_workflow_contracts)'` → 178 passed.

Phase 12 status: checkpoint met. Docs, READMEs, and skills describe built
behavior; every changed symbol received a comment drift pass; regeneration is
deterministic with no stale files; lint, test, doctest, and tier-coverage
gates pass.

## Phase 13

Closing evidence: the final acceptance run (Wave 27) and the ledger,
whole-work checklist, and review index (Wave 28). The orchestrator did the
work itself; two read-only subagents extracted closure evidence from the
seven predecessor logs, and every cited line was re-read before it went into
the table below. Working tree at start: Phase 12's edits are still
uncommitted (latest commit `29c74629d`), so the scope calculator sees
Phases 12 and 13 together.

### Wave 27: Final acceptance run (macOS, this host)

All from `schematic/`, on the current tree, serially, nothing else running:

| Command | Result |
| --- | --- |
| `just test` | exit 0 — L1 `2972 tests run: 2972 passed (4 slow), 22 skipped`; fresh-output `16 passed` (includes `artifact_drift`'s compile partner `slow_real_apis_compile_offline`, the R30 feature selections, and the generated-doc compile check); compile-fail `1 passed` |
| `just doctest` | exit 0 — define 177 (7 ignored) + README 18, definitions 48 (2 ignored), gen 19, oauth 3, runtime 3, `doctest-schema` 577 (default features) + 78 (`emqx` alone) |
| `just lint` | exit 0 (define, gen, definitions, oauth, runtime; `-D warnings`) |
| `just lint-schema` | exit 0 — clippy `-D warnings` on the generated crate (ledger row 21) |
| `just test-schema` | exit 0 — `152 tests run: 152 passed` |

- Drift: not run through the legacy `check-drift` recipe. Its two selections
  (`artifact_drift::*` and `fresh_output_compile::slow_real_apis_compile_offline`)
  are both inside `just test` above (L1 and fresh-output), and passed.
- Artifact and input-matrix coverage is inside the same L1 run:
  `artifact_drift::generated_{rust,openapi,postman,asyncapi}_artifacts_are_up_to_date`,
  `extension_round_trip::*` (15 artifacts), `postman_runnable::*`,
  `asyncapi_round_trip::*`, `openapi_extension_matrix::*`,
  `asyncapi_import_matrix::*`, and the import matrices of Phase 4.
- Determinism: Phase 12 regenerated twice through the recipes with zero
  diff; no generation input changed since, and the drift tests above pass.
- `just test-l2`: not run. No L2 test was added or changed by this feature
  (`git diff 09de6ee35 -- schematic/gen/tests/level2` is empty).

Dependents, on the current tree (none edited:
`git diff --stat 09de6ee35 -- sniff biscuit-speaks model-citizen homelab`
and `git status --porcelain` on the same paths are both empty):

| Command (repo root) | Result |
| --- | --- |
| `cargo build -p sniff -p biscuit-speaks -p model-citizen -p homelab -p eversolo-integration -p arcam-amp-integration -p sony-receiver-integration -p unfolded-integration-helper --all-targets` | exit 0, no warnings |
| `just test model-citizen homelab eversolo-integration arcam-amp-integration sony-receiver-integration unfolded-integration-helper` | exit 0 — `422 tests run: 422 passed, 8 skipped` |
| `cargo nextest run -p sniff -E 'test(/remote/)'` (sniff has 2158 tests; these are the `schematic_schema` users) | `75 passed` |
| `cargo nextest run -p biscuit-speaks -E 'test(/elevenlabs/)'` (441 tests; the `schematic_schema` users) | `48 passed` |
| `just check-tier-coverage schematic` | stranded 0 |

Phase 7's dependent results were not reused: Phases 8–10 changed generated
doc comments and the definitions crate after them, so every dependent was
rebuilt and retested here.

### Cross-OS evidence (R-acceptance "macOS, Linux, native Windows, WSL2")

The `os` skill was loaded first. Hosts declared: `BUILD_LINUX=build-linux`,
`BUILD_WIN=build-win-native`, `BUILD_WSL=build-win`. No earlier phase of this
feature collected other-OS evidence, so all of it was collected here with
`just cross-check` (archive mode, as CI runs it; WSL2 consumes the
`ubuntu-latest` archive like the `wsl2-ubuntu` cell).

**Defect found and fixed.** The first `schematic-gen` run failed one test on
all three legs (Linux `968/969`, native Windows `966/967`, WSL2 `968/969`):
`asyncapi_persist::cli_commands_export_and_import` panicked at
`gen/tests/l1/asyncapi_persist.rs:146` with `NotFound`. The Phase 10 test
spawned the CLI through `env!("CARGO_BIN_EXE_schematic-gen")`, a path baked in
on the build host that does not exist where a nextest archive is extracted,
which is how every CI cell runs. macOS never saw it because local runs do not
use an archive. Fix: `biscuit_test_harness::bin_exe!("schematic-gen")`, which
the sibling tests (`output_validation`, `partial_generation`,
`definition_bundles`, `ws_validation`) already use. No other
`CARGO_BIN_EXE` use remains under `schematic/`. The failing archive runs are
the before evidence; the rerun below is the after evidence; locally
`asyncapi_persist::*` 6 passed.

| Package (mode) | Linux (`build-linux`) | native Windows (`build-win-native`) | WSL2 (`build-win`) |
| --- | --- | --- | --- |
| `schematic-gen` L1 (archive), after the fix | 969 passed, 24 skipped | 967 passed, 24 skipped | 969 passed, 24 skipped |
| `schematic-definitions` L1 (archive) | 657 passed | 657 passed | 657 passed |
| `schematic-define` L1 (archive) | 850 passed | 850 passed | 850 passed |
| `schematic-schema` generated crate (native, `--all-features --manifest-path schematic/schema/Cargo.toml`) | 152 passed | 152 passed | 152 passed |
| macOS (this host) | all Wave 27 gates above | | |

- Each run is behavioral evidence for the tree it shipped (this working
  tree). `cross-check` published no Windows or WSL2 receipt ("the remote
  tested tree … is not this head's tree"), because the tree is uncommitted;
  that is expected and is not a failure.
- Not exercised on another OS: the `slow_` suites that run Cargo themselves
  (`just test-fresh-output`, `just test-compile-fail`) and the doctests.
  Archive mode cannot select them, and their nextest filterset
  (`test(/(^|::)slow_/)`) cannot pass through `cross-check` intact (the `os`
  skill's build-hosts page). They passed on macOS only. `schematic-runtime`
  and `schematic-oauth` were not touched by this feature and were not
  cross-checked.
- Dependents were built and tested on macOS only.

### Wave 28: Ledger disposition table (R44, R45)

Child specs by the parent's D50 numbering, written by directory name:
spec 1 `2026-10-11-schematic-generation-integrity`, spec 2
`2026-10-12-schematic-runtime-envelope`, spec 3 `2026-10-13-schematic-pagination`,
spec 4a `2026-10-14-schematic-definition-api`, spec 4b
`2026-10-15-schematic-definition-vocabulary`, spec 5
`2026-10-16-schematic-response-model-graphql`, spec 6
`2026-10-17-schematic-websocket`, spec 7 this feature
(`2026-10-18-schematic-fidelity-polish`). One more completed feature outside
the eight, `2026-10-12-schematic-endpoint-methods`, matters for D53. "GI log",
"RE log", "PG log", "DA log", "DV log", "RM log", "WS log" are those specs'
`implementation-log.md`; "parent" is the macro review spec. ✱ marks a row
whose starting assignment turned out wrong or incomplete.

**Defect ledger (parent section 10), 22 rows**

| Row | Defect | Disposition | Evidence |
| --- | --- | --- | --- |
| 1 | Non-JSON request bodies dropped | Closed, spec 1 | `text_body_reaches_the_server_verbatim_under_its_content_type`, `binary_body_reaches_the_server_byte_for_byte` (GI log:1814-1815, R21–R27 at :2275); live use is 0 text and 0 binary, so proof is by fixtures (:402-403) |
| 2 | `generate --api <one>` deletes others | Closed, spec 1 | `partial_generation::partial_run_regenerates_the_whole_group_and_keeps_every_other_module` (GI log:1346-1352, :2266-2267); here, Phase 12's `generate-one` runs kept all 14 other modules |
| 3 | Wrong `ws://remote.local/ws/ws` URL | Closed, spec 6 | `ws_validation::every_shipped_endpoint_composes_its_documented_url`, `ws_runtime::core_endpoints_dial_their_documented_channel_paths` (WS log:2124) |
| 4 | Generator panic on non-identifier type name | Regression only (FIXED in parent); spec 1 added `invalid-type-syntax` | GI log:798, :909 (`accepted_definitions_produce_no_diagnostics_for_any_output`, `rust_only_rejections_name_feature_and_endpoint`) |
| 5 ✱ | `ApiKeyParam` never sends key | Closed, spec 1 then spec 2 (the assignment named spec 1 only) | GI log:1816-1818 (R23–R24, marked temporary at :2297); RE log:2381-2382 (env scan removed), :3253-3255 (R14–R16 credential tests) |
| 6 | Duplicate `operationId` in `openapi/ollama.json` | Closed, spec 1 | `artifact_drift::committed_openapi_documents_have_unique_operation_ids` (GI log:1107, :2271); passes in this phase's `just test` |
| 7 | Nested reference completeness | Closed, spec 7 R7 | `RestApi::validate_schema_closure`; `schema_fidelity::every_registry_closes_its_references_and_names_no_container_component` (Phase 2, Wave 5) |
| 8 | Generated docs lie | Closed, spec 7 R38–R41, R51 (typed convenience methods: see accepted gaps) | `generated_docs::the_shipped_crate_docs_hold_only_compilable_examples_of_existing_methods`, `generated_docs::slow_generated_doc_examples_compile_with_default_and_selected_features`, `generated_docs::a_shared_module_documents_every_member_api`; `just doctest-schema` 577 + 78 (Phase 8; rerun above) |
| 9 | WS reconnect counter and lifecycle open | Closed, spec 6 | `ws_session::budget_resets_after_each_successful_redial`, `opening_message_is_written_first_on_every_connection` (WS log:2125, :2103-2104); untested case noted at :2162 (an opening write failing on a redial) |
| 10 | Dormant `E0308` in Bearer host template | Closed, spec 6 | `fresh_output_compile::slow_bearer_ws_host_compiles_and_checks_tokens` (WS log:2126) |
| 11 | WS env auth never applied | Closed, spec 6 | `ws_handshake::core_credential_precedence_per_call_then_client_then_env`, `redial_reuses_the_credential_selected_at_connect` (WS log:2127) |
| 12 | WS plan diagnostics discarded | Closed, spec 6 | `ws_validation::a_lowering_error_reaches_the_shared_format_and_blocks_the_rust_output`, `plan::tests::lowering_failure_keeps_every_finding_not_only_the_first_error` (WS log:2128) |
| 13 | `x-schematic` round trip unimplemented | Closed, spec 7 R10–R15 | `extension_round_trip::*` (every committed artifact re-imports strict and re-exports equal), `openapi_extension_matrix::*` (Phase 4, Waves 9–10) |
| 14 | `skip_serializing_if` without `serde(default)` | Not reproducible, guarded (spec 7 R14) | `extension_reimport::{absent_optional_collections_reimport, explicitly_empty_env_mapping_reimports, explicitly_empty_extension_arrays_decode_like_absent_ones}` (Phase 1 Wave 2; rewritten against the reader in Phase 4) |
| 15 | Test API list drift (2 of 18 skipped) | Closed, spec 7 R34, R35 | `openapi_strict_completeness::the_strict_gate_covers_every_registered_api` (skip list empty), `api_inventory::*` (Phase 5) |
| 16 ✱ | `generated_code_compiles` is `#[ignore]`d | Closed by the isolated compile check, spec 1; the old `#[ignore]`d tests still exist | `fresh_output_compile::{slow_fixture_apis_compile_and_send_offline, slow_real_apis_compile_offline, slow_negative_control_reports_compiler_error}` (GI log:2272); old tests and the `cargo test`-based `test-e2e` recipe kept (GI log:2247, "Default 9 … not applied"); fresh-output 16 passed above |
| 17 | Strict import fails on INFO; tautological test | Closed, spec 7 R16 | `build_strict_mode_{passes_info_only_diagnostics,fails_on_warn_diagnostic,fails_on_error_diagnostic}` (Phase 4, Wave 8) |
| 18 | Stale tracked `gen/schematic/schema/` tree | Closed, spec 1 (deletion, GI log:2035, :2277); verified absent, spec 7 R50 | `upstream_hygiene::stale_nested_schematic_tree_is_absent`; this phase: `ls schematic/gen/schematic` → no such file, `git ls-files schematic/gen/schematic \| wc -l` → `0` |
| 19 | README drifted, not doctested | Closed, spec 7 R42 | `#[cfg(doctest)] ReadmeDoc` in `define/src/lib.rs`; README doctests 18 passed (Phase 11; rerun above); broken-fixture proof in Phase 11 |
| 20 ✱ | Inline schemas import as `serde_json::Value` | Regression only (FIXED in parent); no child spec records it | No predecessor log names it. Indirect regression evidence here: every committed artifact re-imports and re-exports through the closure check (`extension_round_trip::*`, `validate_schema_closure` in `openapi::export`). No dedicated foreign-spec inline-schema test was found; recorded, not invented |
| 21 | Clippy warnings in generated crate | Regression only (FIXED in parent); verified, spec 7 R44 | `just lint-schema` exit 0 above (also GI log:1944-1945) |
| 22 | Two module-path resolvers | Closed, spec 7 R36 | `schematic_definitions::naming`; `module_naming::every_writer_files_an_api_under_the_one_resolved_stem`, `module_naming::every_shipped_artifact_is_named_by_the_resolver` (Phase 6) |

**Recommendations (parent section 9), 14 rows**

| Rec | Subject | Disposition | Evidence |
| --- | --- | --- | --- |
| 1 | Builders for `Endpoint` and `RestApi` | Closed, spec 4a | DA log:1996 (`builders::public_default_set_equals_documented_allow_list`), :1997 (compile-fail doctests), :2000 (`#[non_exhaustive]`), :2006 (all 18 providers, 539 endpoints) |
| 2 | Bind type references to types | Closed, spec 4a (Phase 9 redesign: `TypeRef` + `RestApi::schemas`) | DA log:1989, :2113, :2195-2208 |
| 3 ✱ | Silent drops become errors or features | Partly closed, spec 1 | Per-output validation R8–R10 (GI log:2268); type syntax, module path, duplicate ids are errors (:792-799, :2269); text/binary/`ApiKeyParam`/header/cookie implemented (D10). **Open:** no base-URL/path composition check was built (GI log:840-843, :970) |
| 4 | Verify fresh code compiles | Closed, spec 1 | Same as row 16; R20 "fresh = committed" (GI log:2274) |
| 5 | Streaming and response content types | Closed, spec 5 | RM log:3463-3466 (R4–R11c), Ollama R30, ElevenLabs R33 (:2725-2764); notes: LM Studio `DownloadModel` stays JSON (:3475) |
| 6 | `ApiResponse` as status-to-schema map | Closed, spec 5 | RM log:3468-3469 (`error_routes::lookup_prefers_an_exact_status_then_the_narrowest_range`, `response_openapi::every_shape_of_a_status_merges_into_one_response_object_with_its_notes`) |
| 7 ✱ | Params and required constant headers as OpenAPI parameters | Params: done before the children (parent section 10 status note). Constant headers: closed, spec 7 R8, R9. Typed path params and query fidelity are **not** rec 7; they closed under spec 4b (DV log:1011, :1019) | `fixed_header_export::*` (Phase 3); 17 `fixed-header` warnings remain where OpenAPI cannot express the header (Phase 3 "Shipped gaps") |
| 8 | Schemars bridge | Closed, spec 7 R1–R7 | Lost untyped 766 → 0, typed losses 624 → 0 (Phase 2); `conversion::tests::*`, `schema_values::*`, `schema_fidelity::*` |
| 9 | `x-schematic` read or strike | Closed, spec 7 R10–R15 (read for the v1 subset; `SchematicSchemaExtension`/`AuthExtension` struck) | Phase 4 Wave 9; row 13 |
| 10 | `--api <one>` cleanup, stale tree, resolver, feature flags | Closed: cleanup and stale tree spec 1 (rows 2, 18); resolver spec 7 R36 (row 22); feature flags spec 7 R29–R32; stale-tree check spec 7 R50 | `module_features::*`, `cargo_gen::tests::*` (Phase 7) |
| 11 | One generated list | Closed, spec 7 R34 | `api_inventory::*` in definitions and gen (Phase 5). Note: the guarded test tables still need an entry per new module (Phase 12 observation); each fails loudly if omitted |
| 12 | Docs honesty pass | Closed, spec 7 R38–R41 (typed send and `send_raw` from spec 2, RE log:3241, :3266); typed convenience methods: accepted gap, see below | Row 8 evidence |
| 13 | WebSocket follow-through | Closed, spec 6 | WS log:2110-2113 (R10 typed message enums, R11 `responds_with`, R8 receive timeout, R2–R5, R9) |
| 14 ✱ | `schematic-define` hygiene | README doctests: closed, spec 7 R42. Vendor pagination constructors: closed, spec 4a (DA log:2031, "no shared presets"), verified here R43. `ConnectionParam`/`ParamType` collapse: closed, spec 6 (WS log:2113). `env_auth[0]` retirement and `deprecated`: closed, spec 4b (DV log:1015, :1013). Old pagination types: removed, spec 4a (DA log:1546). Pagination payload: replaced, spec 3 (PG log:2606) | The assignment's "spec 4b" for the vendor-constructor move was wrong; 4a did it |

**Additional dispositions**

| Item | Disposition | Evidence |
| --- | --- | --- |
| `Schema::module_path` dead on the REST path ✱ | Closed, spec 4a, by moving REST to `TypeRef`; the field stays because only WebSocket `full_path` reads it (not deleted) | DA log:2011, :2028, :2200 |
| `?` in path templates | Spec 1 warning (GI log:913, :2280); spec 4a hard error and migration of the nine endpoints (Gitea 1, GitLab 1, OpenAI 7; DA log:2016-2017, :2026); verified here, R52 | This phase: `rg -n '"/[^"\s]*\?[^"]*"' schematic/definitions/src` → 0 matches; the R52 tests (Phase 11) pass in `just test` |
| Old pagination types removed, conversion to the spec 3 model | Closed, spec 4a | DA log:1546, :2007 (`pagination_coverage`), :2031 (all 35 declarations accounted for) |
| Vendor pagination constructors | Spec 4a (each API owns a strategy function, D55-4); verified here with no move, R43 | This phase: `rg '(Strategy\|Pagination\|PaginationStrategy\|MoreSignal\|Collection\|Paginated)::(github\|gitlab\|bitbucket\|gitea)\b' -g '*.rs' schematic` → 0 matches; `AddressRule::github()` (GraphQL address rule, outside R43) remains, on the Phase 1 agenda |
| `oauth_scopes` | Removed, spec 4a (D56-6; DA log:1546, :2004); `auth_policy` builder form kept (DA log:1999); the `x-schematic` copy is gone and the round-trip test forbids it | Phase 4: `no_fixture_extension_carries_an_auth_field_beyond_the_two_details`; R15 grep |
| emqx combined-module docs loss | Closed, spec 7 R51 (symptom was lost per-API sections, not lost names) | `generated_docs::a_shared_module_documents_every_member_api` (Phase 8) |
| Parent section 6 item 5: declared response type discarded at the call site | Closed, spec 2 | RE log:3241 (R2; `fresh_output_compile::slow_generated_types_reject_misuse_at_compile_time`), :2377-2378 |
| Parent section 6 item 6: generated docs wrong | Closed, spec 7 (row 8) | Row 8 |
| Parent section 6 item 7: no feature flags | Closed, spec 7 R29–R32 | Phase 7 |
| Parent section 6 item 8: `Schema::module_path` unused; type-name validation | `module_path`: above. Type-name validation: closed, spec 1 (`invalid-type-syntax`, GI log:798) and spec 4a (`invalid-schema-reference`, DA log:2198-2202, :2250) | — |

**Accepted gaps (R45's five, named separately)**

| Gap | Status | Reason, from predecessor evidence |
| --- | --- | --- |
| Optional-auth variant (Ollama) | ACCEPTED GAP | Parent D56 (1): "a per-API required/optional auth field (rejected; remains an accepted-gap candidate)"; the chosen option "matches D11's single-attempt contract and keeps the 401 evidence" (parent:2257-2261). DA log:2037: "optional auth (Ollama ignores but accepts keys) remains unimplemented" |
| Legacy-versus-modern auth dual-source cleanup | ACCEPTED GAP | DA log:2037: "wider legacy/modern auth dual-source cleanup remains beyond coherence checking"; DV log:1065 lists it as "unassigned auth remainder"; spec 4b review-1:94 "outside this feature". Only `env_auth[0]` retirement was assigned (spec 4b). No landing owns the rest |
| Invented HuggingFace stub shapes | ACCEPTED GAP | DA log:2037: "not re-modeled"; recorded "as an explicit deferred candidate until source-backed decisions exist; do not silently prune schemas or widen auth" |
| Orphaned types | ACCEPTED GAP | DA log:2037: "explicit orphan registrations are intentionally preserved pending a later audit" |
| Typed convenience methods (D53, R39) | ACCEPTED GAP as decided (parent:2183-2185: about 7 s extra compile time at 550 wrapper instantiations; the typed request already carries the response type; wrappers only help the no-filter case). **Superseded in practice:** `2026-10-12-schematic-endpoint-methods` generates one method per REST endpoint (e.g. `schema/src/github/requests.rs:3204` `get_repository`), and the generated docs show that method first and `client.send(Request::new(..))` as the typed alternative (Phase 1, Phase 8 adopted defaults) | The author should decide whether to retire this gap entry |

**Stale July figures corrected (D49, parent:2069 and the notes it added)**

| Figure | July | Corrected |
| --- | --- | --- |
| Section 7.1 endpoints with pagination | 27 | 35, all page-number (gitea 8, github 9, gitlab 9, bitbucket 9) (parent:273) |
| Section 6 item 1 "six endpoints POST empty bodies" | 6 | Partly fixed; 2026-10-11 counts text 0, binary 0, `ApiKeyParam` 0, header/cookie 0, multipart about 29 (parent:239). Spec 1 measured: 16 multipart, 7 header parameters, 18 POST/PUT/PATCH without body, 9 `?` paths (GI log:391-431) |
| Endpoint count | about 519 | 549 (spec 1, GI log:391-431); this feature counted 503 literals, 539 per API (Phase 1 Wave 2) |
| Untyped properties (section 7.1) | about 30% | 26.5% at the Phase 1 baseline, 0 lost after Phase 2 (Phase 1 Wave 2; Phase 2) |

Self-check (one-shot, not committed): a Python scan of this section's three
main tables counted 22 ledger rows and 14 recommendation rows, each with a
nonblank disposition and evidence cell — result: `ledger 22, numbers 1–22 complete, all nonblank`; `recs 14, numbers 1–14 complete, all nonblank`.

### Wave 28: Whole-work done checklist (R46, D20)

| D20 part | State | Evidence |
| --- | --- | --- |
| 1. Parent section 14 criteria per landing | **Holds, with recorded partials** | Specs 1, 2, 3 are in `_completed` with `implemented: true` and a final review `ready: true` (spec 1 review-3, spec 2 review-3, spec 3 review-2). Specs 4a, 5, 6 are `implemented: true` with a final review `ready: true` (4a review-3, 5 review-2, 6 review-2). **Spec 4b**: `implemented: true`, but its only review (review-1) is `review_scope: human-decision-triage-only`, `readiness_assessed: false` (review-1.md:59, :110: a later review must still do the implementation comparison). Spec 7 (this): implementation complete, ready for review 1. Known partials: rec 3's base-URL composition check (spec 1), the kept `#[ignore]`d e2e tests (spec 1) |
| 2. Every dependent builds and passes now | **Holds (macOS)** | Wave 27 dependent table above |
| 3. Ledger dispositions | **Holds, pending author judgment** | Tables above: 22 + 14 rows, every row closed, partly closed with the open part named, regression-only, or an accepted gap; the author judges the gaps at review 1 |
| 4. Sniff check (R47) | **OPEN** | Below |

**R47 sniff check** (read-only; sniff not edited):

```text
$ rg -c FocusedProviderClient sniff
→ 58 references in 10 files:
sniff/fixes/2026-10-09-consistent-use-schematic/spec.md
sniff/lib/CHANGELOG.md
sniff/lib/README.md
sniff/lib/src/remote/blocking.rs      (e.g. :25 use super::{FocusedProviderClient, PullRequestInfo};)
sniff/lib/src/remote/check_status.rs
sniff/lib/src/remote/focused.rs       (:63 pub struct FocusedProviderClient {)
sniff/lib/src/remote/mod.rs
sniff/lib/src/remote/types.rs
sniff/lib/tests/l1/branch_head.rs
sniff/lib/tests/l1/focused_provider.rs
sniff/lib/tests/l1/pr_for_branch.rs

$ find sniff -type d -name '*2026-10-09-consistent-use-schematic*'
sniff/fixes/2026-10-09-consistent-use-schematic   (active; no plan or log)
spec frontmatter: status: draft-spec, implemented: false, human_review: true
```

`FocusedProviderClient` is still defined and used, and sniff's spec is not
at "implementation complete". D20 part 4 is **open**. This feature is still
ready for review; the whole work cannot close until sniff's spec lands.

### Wave 28: Review index (spec "Human review agenda", 26 items)

Each item was logged as it occurred; this index points at it.

| # | Item | Where |
| --- | --- | --- |
| 1 | Adopted defaults (P10-a to P10-l, Open Questions) | Phase 1 "Adopted defaults and plan discrepancies"; Phase 3 "Design and adopted defaults"; Phase 4 Waves 8–10; Phase 6 Wave 13; Phase 8 Wave 16; Phase 9 Wave 19 and agenda; Phase 10 "Adopted defaults and decisions"; Phase 11 Wave 24 |
| 2 | Design choices not dictated | Resolver in `schematic_definitions::naming` (Phase 6); inventory `registry::registrations()` (Phase 5); validator `jsonschema` (Phase 1 validator spike, Phase 9); example rules (Phase 9 Wave 18); AsyncAPI layout and file names (Phase 10) |
| 3 | Deviations | Listed under "Deviations" below |
| 4 | Open design questions | Phase 1 agenda 1 (D12 payload schemas), Phase 4 agenda 1 (OpenAI re-import pins), Phase 10 agenda 1 (correlation hints) |
| 5 | Untyped counts, deliberate free-form list | Phase 1 Wave 2 (baseline table), Phase 2 Wave 5 (766 → 0 lost) |
| 6 | API lists enumerated (R33) | Phase 1 Wave 2 "Consumer inventory" (4 REST + 2 WS runtime lists, not 6); Phase 5 re-derivation |
| 7 | Row 14 outcome | Not reproducible, guarded (Phase 1 Wave 2, Phase 4) |
| 8 | `x-schematic` subset, version and conflict rules, R54 placement | Phase 4 Wave 9; contracts.md |
| 9 | Extension structs deleted or wired | `SchematicSchemaExtension`, `AuthExtension` deleted; doc/op extensions serialize-only writers with a separate reader (Phase 4 Wave 9) |
| 10 | Strict-mode change | Strict fails on WARN and ERROR, never INFO (Phase 4 Wave 8); absent `servers` is now WARN (Phase 4 agenda 5) |
| 11 | Renamed/removed artifacts (R37) | None (Phase 6 "R37: renamed artifacts") |
| 12 | Feature flags and dependent evidence | Phase 7 Waves 14–15; dependents rerun in Wave 27 above |
| 13 | P10-g outcome | Superseded by generated endpoint methods; docs show method + `client.send(..)` (Phase 1, Phase 8); D53 gap row above |
| 14 | Three representative Postman examples | Phase 9 Wave 19 ("Three representative generated examples") and agenda 5–6 |
| 15 | AsyncAPI round-trip results, validation, extension names | Phase 10 "R27 round-trip results per API" and "Extension names and shapes" |
| 16 | Ledger table, accepted gaps, wrong assignments | Wave 28 tables above (✱ rows) |
| 17 | Whole-work checklist, sniff grep | Wave 28 checklist above |
| 18 | Environments not exercised | "Cross-OS evidence" above |
| 19 | Unverified spec claims verified | Stale tree absent (Phase 1, Phase 11, this phase); bridge location, endpoint counts, list counts (Phase 1); no `pagerdutyapi`-style shipped files (Phase 6 R37) |
| 20 | Comment/doc drift | Phase 12 Wave 25 (R49 list); Phase 6 `RestApi::module_path` doc; Phase 11 pre-existing doctest drift |
| 21 | Pre-authorizations used | Listed below |
| 22 | R50 and R51 | Phase 11 Wave 24, Phase 8 Wave 16 |
| 23 | R43 and R52 greps | Phase 1 Wave 2, Phase 11 Wave 24, rerun this phase (additional dispositions) |
| 24 | `keep` and `Bytes` in the subset; `oauth_scopes` copy; JSON snapshots | `keep` and `Bytes` travel in `x-schematic.success` (contracts.md "Response model v2"; `response_openapi::extension_import_restores_declared_error_levels_and_stream_ends`); `oauth_scopes` forbidden by the round-trip test (Phase 4); snapshots: `gen/tests/fixtures/definition-baseline/` with `definition_baseline.rs` (contracts.md D9) |
| 25 | GraphQL in Postman, `variables` source, OpenAPI skips | Phase 9 Wave 19 "GraphQL (R55)" and agenda 4 |
| 26 | Container components over the 18 registries | None found; Bitbucket generic struct keys with `<` are the recorded R53 deviation (Phase 2 agenda 2) |

**Requirement index (R1–R58)**

| Requirements | Phase(s) with evidence | State |
| --- | --- | --- |
| R1–R6 | 1 (baseline), 2 | Met; R3 deviation (`"default": null` dropped) |
| R7, R53 | 2 | Met; R53 deviation (nine Bitbucket keys with `<`) |
| R8, R9 | 3 | Met; 17 `fixed-header` warnings where OpenAPI cannot express the header |
| R10–R18, R54 | 1 (R14), 4 | Met; pagination and AsyncAPI-only hints outside the v1 subset (adopted defaults) |
| R19–R24, R55 | 9 | Met |
| R25–R28, R56–R58 | 10 | Met, except R25's typed payload schemas: payloads are unconstrained `{}` components (D12 gap, open for review) |
| R29–R32 | 7 | Met |
| R33–R35 | 1, 5 | Met |
| R36, R37 | 6 | Met (no artifact renamed) |
| R38–R41, R51 | 8 | Met |
| R42, R43, R50, R52 | 11 (R43/R50/R52 also 1 and this phase) | Met |
| R44–R47 | 13 | Met as reporting requirements; D20 part 4 open |
| R48, R49 | 12 | Met |

**Deviations** (each logged in its phase): R3 `"default": null` dropped
(Phase 2); R53 Bitbucket `<` component keys (Phase 2); R25 unconstrained
payload schemas (Phases 1, 10); AsyncAPI operation without `messages` is INFO,
not the frozen ERROR (Phase 10); `info.description` absent reads as `""`
(Phase 10); 2.x auth only from applied `security` (Phase 10); multipart
object parts without JSON encoding stay text with WARN (Phase 4); R42 via a
`#[cfg(doctest)]` item rather than the crate-root `#![doc]` (Phase 11).

**Subset exclusions:** OpenAPI `x-schematic` v1 excludes pagination (D7);
AsyncAPI excludes correlation hints, `auth_flow`, lifecycle roles, runtime
hints, env metadata (D11), so the Core, Dock, and Integration documents
re-import into definitions that do not lower to a client (Phase 10).

**Validator and gap status:** Postman examples validated with `jsonschema`
against registered schemas (Phase 9); AsyncAPI output validated offline
against the vendored 3.0.0 schema (Phase 10); OpenAPI artifacts parse with
`openapiv3` and round-trip strict (Phase 4).

**Pre-authorizations used:** regenerated `schematic-schema`, OpenAPI, Postman,
and AsyncAPI artifacts (Phases 2, 3, 8, 9, 10, 12); changed the generated
crate's `Cargo.toml` features and optional dependencies (Phase 7); moved
`jsonschema` and added a definitions dev dependency (Phases 9, 10, recorded in
`schematic/docs/dependencies.md`); created `schematic/asyncapi/`, the AsyncAPI
doc page, and the `generate-asyncapi`/`doctest-schema` recipes (Phases 8, 10);
deleted `SchematicSchemaExtension`/`AuthExtension` (Phase 4); edited
`schematic-definitions` for the single list at the API-constructor level only
(Phase 5); built and tested every dependent read-only (Phases 7, 8, 13). No
artifact rename was needed (R37). Not used: commits, lifecycle moves, CI
changes, provider calls, dependent or sniff edits.

**Renamed artifacts:** none (R37).

**Remaining open items for the whole work:** D20 part 4 (sniff
`FocusedProviderClient`); spec 4b's verifying review; rec 3's base-URL
composition check; the D12 WebSocket payload schemas; the five accepted gaps
(author's judgment at review 1); the Phase 12 observations (stale `install`
recipe, stale stream text in `create-schematic-api.md`, test tables to extend
per new module).

### Requirement-to-test mapping (Phase 13)

| Behavior | Test or command |
| --- | --- |
| CLI wiring test runs where CI runs it (archive) | `asyncapi_persist::cli_commands_export_and_import`: failed on Linux, Windows, WSL2 archive runs before the fix, passes on all three and macOS after |
| R44 row 21 | `just lint-schema` exit 0 |
| R45 22 + 14 complete rows | one-shot Python self-check above (not committed) |
| R46 parts 1–4 | the checklist above |
| R47 | `rg -c FocusedProviderClient sniff` (58), sniff spec frontmatter |
| R50, R52, R43 | `ls`/`git ls-files`/`rg` greps rerun this phase; `upstream_hygiene::*` and the R52 tests in `just test` |

Input robustness matrix: not applicable (no reader changed). The only code
edit is the one test line; no tier marker; it stays in the declared `l1`
binary.

### Gates (Phase 13, final)

- `just test` (schematic), rerun after the fix: exit 0 — L1 `2972 passed (4 slow), 22 skipped`; fresh-output `16 passed`; compile-fail `1 passed`.
- `just lint` (schematic), rerun after the fix: exit 0.
- `just doctest`, `just lint-schema`, `just test-schema`: exit 0 (Wave 27; the fix touches no doc or generated code).
- `just check-tier-coverage schematic`: stranded 0.
- Scope calculator after every Phase 13 edit (`git status --porcelain
  --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py`): `packages: [schematic-define,
  schematic-definitions, schematic-gen, test-toolkit, darkmatter]`,
  `change_class: package` (it includes Phase 12's uncommitted edits); gates
  were required and ran. Out-of-area test inputs: `cargo nextest run -p
  darkmatter --test l1 -E 'test(/shipped_schema_corpus/)'` 8 passed;
  `cargo nextest run -p test-toolkit -E 'binary(ci_workflow_contracts)'` 178
  passed.
- Dependents: Wave 27 table (all pass).
- No failing or skipped required check on macOS. No pre-existing failure.

Skill review: no skill change. The fix applies an existing `rust-testing` /
`os` rule (spawn binaries with `bin_exe!`), and no schematic behavior
changed.

Phase 13 status: **implementation complete, ready for review.** All
acceptance commands pass on macOS; L1 and the generated crate pass on Linux,
native Windows, and WSL2. The ledger has 22 + 14 complete rows, and D20 parts
1–3 hold. D20 part 4 (sniff) is **open**, and the whole-work items listed
above stay open. No spec was moved, `just complete` was not run, and nothing
was committed.
