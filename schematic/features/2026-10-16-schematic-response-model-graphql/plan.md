---
total_phases: 8
created: 2026-10-11
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - schematic-define
  - schematic-gen
  - schematic-runtime
  - schematic-schema
  - schematic-definitions
  - sniff
source_files_during_phase_2:
  - schematic/define/src/builders.rs
  - schematic/define/src/bundle.rs
  - schematic/define/src/components.rs
  - schematic/define/src/error_table.rs
  - schematic/define/src/graphql.rs
  - schematic/define/src/lib.rs
  - schematic/define/src/openapi/export/info.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/import/builder.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/pagination.rs
  - schematic/define/src/prelude.rs
  - schematic/define/src/registration.rs
  - schematic/define/src/request.rs
  - schematic/define/src/response.rs
  - schematic/define/src/types.rs
  - schematic/define/tests/builders.rs
  - schematic/define/tests/bundle_support/mod.rs
  - schematic/define/tests/cross_origin_policy.rs
  - schematic/define/tests/definition_bundles.rs
  - schematic/define/tests/fixtures/bundles/fixture.json
  - schematic/define/tests/graphql_definitions.rs
  - schematic/define/tests/response_vocabulary.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/src/validation.rs
docs_updated_during_phase_2:
  - schematic/docs/definition-builders.md
  - schematic/docs/io/definition-bundles.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_2:
  - schematic/docs/responses-and-errors.md
  - schematic/docs/graphql.md
skills_files_updated_during_phase_2:
  - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_3:
  - schematic/runtime/Cargo.toml
  - schematic/runtime/src/client.rs
  - schematic/runtime/src/codec.rs
  - schematic/runtime/src/endpoint.rs
  - schematic/runtime/src/error.rs
  - schematic/runtime/src/graphql.rs
  - schematic/runtime/src/lib.rs
  - schematic/runtime/src/pager/request.rs
  - schematic/runtime/src/response.rs
  - schematic/runtime/src/shapes.rs
  - schematic/runtime/src/stream.rs
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_a_non_json_endpoint.stderr
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_a_stream_endpoint.rs
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_a_stream_endpoint.stderr
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_mixed_alternatives.rs
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_mixed_alternatives.stderr
  - schematic/runtime/tests/l1/decode_boundary.rs
  - schematic/runtime/tests/l1/envelope.rs
  - schematic/runtime/tests/l1/error_routes.rs
  - schematic/runtime/tests/l1/graphql.rs
  - schematic/runtime/tests/l1/main.rs
  - schematic/runtime/tests/l1/shapes.rs
  - schematic/runtime/tests/l1/streams.rs
  - schematic/runtime/tests/l1/support.rs
  - schematic/runtime/tests/l1/timing.rs
docs_updated_during_phase_3:
  - schematic/docs/dependencies.md
  - schematic/docs/graphql.md
  - schematic/docs/responses-and-errors.md
  - schematic/docs/runtime/README.md
  - schematic/docs/runtime/codecs.md
  - schematic/docs/runtime/deadlines-and-timeouts.md
  - schematic/docs/runtime/hooks.md
  - schematic/docs/runtime/sending-and-responses.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_4:
  - schematic/define/src/builders.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/paths.rs
  - schematic/define/src/openapi/export/responses.rs
  - schematic/define/src/openapi/extensions.rs
  - schematic/define/src/openapi/import.rs
  - schematic/define/src/openapi/import/mappings/classify.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/openapi/import/mappings/responses.rs
  - schematic/define/src/openapi/import/query_paths.rs
  - schematic/define/src/openapi/mod.rs
  - schematic/define/src/request.rs
  - schematic/define/src/response.rs
  - schematic/define/src/types.rs
  - schematic/define/tests/response_import_matrix.rs
  - schematic/define/tests/response_openapi.rs
  - schematic/define/tests/response_vocabulary.rs
  - schematic/gen/src/codegen/api_struct/helpers.rs
  - schematic/gen/src/codegen/client/helpers.rs
  - schematic/gen/src/codegen/client/methods.rs
  - schematic/gen/src/codegen/client/mod.rs
  - schematic/gen/src/codegen/client/shared_auth.rs
  - schematic/gen/src/codegen/client/tests.rs
  - schematic/gen/src/codegen/endpoint_methods/names.rs
  - schematic/gen/src/codegen/error.rs
  - schematic/gen/src/codegen/graphql.rs
  - schematic/gen/src/codegen/mod.rs
  - schematic/gen/src/codegen/request_enum.rs
  - schematic/gen/src/codegen/request_structs/endpoint_spec.rs
  - schematic/gen/src/codegen/responses.rs
  - schematic/gen/src/codegen/variant.rs
  - schematic/gen/src/definition_gen.rs
  - schematic/gen/src/diagnostics.rs
  - schematic/gen/src/export/postman/item.rs
  - schematic/gen/src/export/vocabulary.rs
  - schematic/gen/src/lib.rs
  - schematic/gen/src/output/assemble/api.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/tests/common/components.rs
  - schematic/gen/tests/l1/e2e_generation.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/main.rs
  - schematic/gen/tests/l1/response_adapters.rs
  - schematic/runtime/src/api_client.rs
  - schematic/runtime/src/client.rs
  - schematic/runtime/src/endpoint.rs
  - schematic/runtime/src/error.rs
  - schematic/runtime/src/graphql.rs
  - schematic/runtime/src/lib.rs
  - schematic/runtime/src/pager/request.rs
  - schematic/runtime/src/response.rs
  - schematic/runtime/tests/l1/envelope.rs
  - schematic/runtime/tests/l1/error_routes.rs
  - schematic/runtime/tests/l1/graphql.rs
  - schematic/runtime/tests/l1/graphql_address.rs
  - schematic/runtime/tests/l1/main.rs
  - schematic/runtime/tests/l1/support.rs
  - schematic/runtime/tests/l1/whole_send_elapsed.rs
  - schematic/schema/src/lib.rs
docs_updated_during_phase_4:
  - schematic/docs/definition-builders.md
  - schematic/docs/generation/generated-clients.md
  - schematic/docs/generation/validation.md
  - schematic/docs/graphql.md
  - schematic/docs/io/export-openapi.md
  - schematic/docs/io/export-postman.md
  - schematic/docs/io/import-openapi.md
  - schematic/docs/io/openapi-extensions.md
  - schematic/docs/responses-and-errors.md
  - schematic/docs/runtime/README.md
  - schematic/docs/runtime/building-clients.md
  - schematic/docs/runtime/codecs.md
  - schematic/docs/runtime/sending-and-responses.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/schematic-define/SKILL.md
  - .claude/skills/schematic-define/schematic/gen.md
source_files_during_phase_5:
  - Cargo.lock
  - schematic/define/src/builders.rs
  - schematic/define/src/components.rs
  - schematic/define/src/lib.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/paths.rs
  - schematic/define/src/openapi/export/responses.rs
  - schematic/define/src/openapi/extensions.rs
  - schematic/define/src/openapi/import/builder.rs
  - schematic/define/src/openapi/import/mappings/classify.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/openapi/import/mappings/responses.rs
  - schematic/define/src/pagination/resolve.rs
  - schematic/define/src/pagination/validate.rs
  - schematic/define/src/prelude.rs
  - schematic/define/src/request.rs
  - schematic/define/src/response.rs
  - schematic/define/src/types.rs
  - schematic/define/tests/builders.rs
  - schematic/define/tests/bundle_support/mod.rs
  - schematic/define/tests/cross_origin_policy.rs
  - schematic/define/tests/definition_bundles.rs
  - schematic/define/tests/fixtures/bundles/fixture.json
  - schematic/define/tests/openapi_tests.rs
  - schematic/define/tests/pagination_support/mod.rs
  - schematic/define/tests/pagination_tables.rs
  - schematic/define/tests/response_import_matrix.rs
  - schematic/define/tests/response_openapi.rs
  - schematic/define/tests/response_vocabulary.rs
  - schematic/definitions/Cargo.toml
  - schematic/definitions/src/anthropic/mod.rs
  - schematic/definitions/src/anthropic/types/mod.rs
  - schematic/definitions/src/anthropic/types/streaming.rs
  - schematic/definitions/src/bitbucket/mod.rs
  - schematic/definitions/src/elevenlabs/endpoints/speech.rs
  - schematic/definitions/src/elevenlabs/mod.rs
  - schematic/definitions/src/elevenlabs/types.rs
  - schematic/definitions/src/elevenlabs/types/common.rs
  - schematic/definitions/src/elevenlabs/types/speech.rs
  - schematic/definitions/src/emqx/mod.rs
  - schematic/definitions/src/eversolo/mod.rs
  - schematic/definitions/src/gitea/mod.rs
  - schematic/definitions/src/github/mod.rs
  - schematic/definitions/src/gitlab/mod.rs
  - schematic/definitions/src/huggingface/mod.rs
  - schematic/definitions/src/lmstudio/mod.rs
  - schematic/definitions/src/lmstudio/types.rs
  - schematic/definitions/src/ollama/mod.rs
  - schematic/definitions/src/ollama/types/completion.rs
  - schematic/definitions/src/ollama/types/models.rs
  - schematic/definitions/src/ollama/types/openai.rs
  - schematic/definitions/src/openai_pagination.rs
  - schematic/definitions/src/openai/mod.rs
  - schematic/definitions/src/registry/typed.rs
  - schematic/definitions/src/samsung_smart_tv/mod.rs
  - schematic/definitions/src/unfolded_circle/core_rest/mod.rs
  - schematic/definitions/tests/authoring_surface.rs
  - schematic/gen/src/codegen/endpoint_methods/tests.rs
  - schematic/gen/src/codegen/module_docs.rs
  - schematic/gen/src/codegen/pagination/tests.rs
  - schematic/gen/src/codegen/request_structs/endpoint_spec.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/src/codegen/responses.rs
  - schematic/gen/src/definition_gen.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/item.rs
  - schematic/gen/src/export/postman/tests.rs
  - schematic/gen/src/export/vocabulary.rs
  - schematic/gen/src/output/assemble/api.rs
  - schematic/gen/src/output/mod.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/tests/common/components.rs
  - schematic/gen/tests/l1/definition_baseline.rs
  - schematic/gen/tests/l1/e2e_generation.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/http_client.rs
  - schematic/gen/tests/l1/openapi_strict_completeness.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/gen/tests/l1/pagination_validation.rs
  - schematic/gen/tests/l1/path_substitution.rs
  - schematic/gen/tests/l1/postman_golden.rs
  - schematic/gen/tests/l1/query_param_detection.rs
  - schematic/gen/tests/l1/query_params_codegen.rs
  - schematic/gen/tests/l1/response_adapters.rs
  - schematic/gen/tests/l1/vocabulary_exports.rs
  - schematic/openapi/anthropic.json
  - schematic/openapi/bitbucket.json
  - schematic/openapi/elevenlabs.json
  - schematic/openapi/lmstudio.json
  - schematic/openapi/ollama.json
  - schematic/openapi/openai.json
  - schematic/postman/elevenlabs.postman_collection.json
  - schematic/postman/lmstudio.postman_collection.json
  - schematic/postman/ollama.postman_collection.json
  - schematic/postman/openai.postman_collection.json
  - schematic/schema/src/anthropic.rs
  - schematic/schema/src/bitbucket/requests.rs
  - schematic/schema/src/elevenlabs/requests.rs
  - schematic/schema/src/lmstudio.rs
  - schematic/schema/src/ollama.rs
  - schematic/schema/src/ollama/mod.rs
  - schematic/schema/src/ollama/requests.rs
  - schematic/schema/src/ollama/responses.rs
  - schematic/schema/src/openai/requests.rs
  - schematic/schema/tests/non_json_endpoints.rs
  - sniff/lib/src/remote/bitbucket.rs
  - sniff/lib/src/remote/envelope.rs
  - sniff/lib/tests/l1/remote_providers.rs
docs_updated_during_phase_5:
  - schematic/docs/definition-builders.md
  - schematic/docs/generation/generated-clients.md
  - schematic/docs/generation/validation.md
  - schematic/docs/io/definition-bundles.md
  - schematic/docs/io/export-openapi.md
  - schematic/docs/io/import-openapi.md
  - schematic/docs/io/openapi-extensions.md
  - schematic/docs/responses-and-errors.md
  - schematic/docs/runtime/codecs.md
  - schematic/docs/runtime/README.md
  - schematic/docs/runtime/sending-and-responses.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/schematic-define/schematic/definitions.md
  - .claude/skills/schematic-define/schematic/gen.md
  - .claude/skills/schematic-define/schematic/SKILL.md
  - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_6:
  - schematic/define/src/openapi/extensions.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/paths.rs
  - schematic/define/tests/response_openapi.rs
  - schematic/gen/tests/l1/response_adapters.rs
  - schematic/schema/tests/streaming_providers.rs
  - schematic/openapi/github.json
  - schematic/openapi/openai.json
docs_updated_during_phase_6:
  - schematic/docs/io/openapi-extensions.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_7:
  - schematic/define/src/graphql.rs
  - schematic/define/src/pagination.rs
  - schematic/define/src/pagination/relay.rs
  - schematic/define/tests/definition_bundles.rs
  - schematic/define/tests/graphql_definitions.rs
  - schematic/gen/Cargo.toml
  - schematic/gen/src/codegen/graphql.rs
  - schematic/gen/src/codegen/pagination/mod.rs
  - schematic/gen/src/lib.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/runtime/src/graphql.rs
  - schematic/runtime/src/lib.rs
  - schematic/runtime/src/pager/mod.rs
  - schematic/runtime/src/pager/pages.rs
  - schematic/runtime/tests/l1/graphql_pager.rs
  - schematic/runtime/tests/l1/main.rs
  - schematic/schema/src/lib.rs
docs_updated_during_phase_7:
  - schematic/docs/graphql.md
  - schematic/docs/pagination.md
  - schematic/docs/runtime/calling-an-api.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_8:
  - schematic/define/src/pagination/resolve.rs
  - schematic/define/src/types.rs
  - schematic/definitions/src/artificial_analysis/types.rs
  - schematic/gen/src/codegen/pagination/mod.rs
  - schematic/gen/src/lib.rs
  - schematic/runtime/src/pager/mod.rs
  - schematic/runtime/src/response.rs
docs_updated_during_phase_8:
  - schematic/README.md
  - schematic/define/README.md
  - schematic/definitions/README.md
  - schematic/runtime/README.md
  - schematic/schema/README.md
  - schematic/docs/definition-builders.md
  - schematic/docs/responses-and-errors.md
  - schematic/docs/runtime/README.md
  - schematic/docs/runtime/building-clients.md
  - schematic/docs/runtime/codecs.md
  - schematic/docs/runtime/sending-and-responses.md
  - schematic/docs/generation/generated-clients.md
  - schematic/docs/generation/validation.md
  - schematic/docs/generation/workflow.md
  - schematic/docs/io/crates-openapi.md
  - schematic/docs/io/export-openapi.md
  - schematic/docs/io/export-postman.md
  - schematic/docs/io/import-openapi.md
  - schematic/docs/io/openapi-extensions.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/schematic-define/SKILL.md
  - .claude/skills/schematic-define/schematic/SKILL.md
  - .claude/skills/schematic-define/schematic/define.md
  - .claude/skills/schematic-define/schematic/definitions.md
  - .claude/skills/schematic-define/schematic/gen.md
  - .claude/skills/schematic-define/schematic/openapi.md
  - .claude/skills/schematic-define/schematic/pagination.md
  - .claude/skills/schematic-define/schematic/responses.md
  - .claude/skills/sniff/remote-and-repository.md
source_code:
  - Cargo.lock
  - schematic/define/src/builders.rs
  - schematic/define/src/bundle.rs
  - schematic/define/src/components.rs
  - schematic/define/src/error_table.rs
  - schematic/define/src/graphql.rs
  - schematic/define/src/lib.rs
  - schematic/define/src/openapi/export/info.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/paths.rs
  - schematic/define/src/openapi/export/responses.rs
  - schematic/define/src/openapi/extensions.rs
  - schematic/define/src/openapi/import.rs
  - schematic/define/src/openapi/import/builder.rs
  - schematic/define/src/openapi/import/mappings/classify.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/openapi/import/mappings/responses.rs
  - schematic/define/src/openapi/import/query_paths.rs
  - schematic/define/src/openapi/mod.rs
  - schematic/define/src/pagination.rs
  - schematic/define/src/pagination/relay.rs
  - schematic/define/src/pagination/resolve.rs
  - schematic/define/src/pagination/validate.rs
  - schematic/define/src/prelude.rs
  - schematic/define/src/registration.rs
  - schematic/define/src/request.rs
  - schematic/define/src/response.rs
  - schematic/define/src/types.rs
  - schematic/define/tests/builders.rs
  - schematic/define/tests/bundle_support/mod.rs
  - schematic/define/tests/cross_origin_policy.rs
  - schematic/define/tests/definition_bundles.rs
  - schematic/define/tests/fixtures/bundles/fixture.json
  - schematic/define/tests/graphql_definitions.rs
  - schematic/define/tests/openapi_tests.rs
  - schematic/define/tests/pagination_support/mod.rs
  - schematic/define/tests/pagination_tables.rs
  - schematic/define/tests/response_import_matrix.rs
  - schematic/define/tests/response_openapi.rs
  - schematic/define/tests/response_vocabulary.rs
  - schematic/definitions/Cargo.toml
  - schematic/definitions/src/anthropic/mod.rs
  - schematic/definitions/src/anthropic/types/mod.rs
  - schematic/definitions/src/anthropic/types/streaming.rs
  - schematic/definitions/src/artificial_analysis/types.rs
  - schematic/definitions/src/bitbucket/mod.rs
  - schematic/definitions/src/elevenlabs/endpoints/speech.rs
  - schematic/definitions/src/elevenlabs/mod.rs
  - schematic/definitions/src/elevenlabs/types.rs
  - schematic/definitions/src/elevenlabs/types/common.rs
  - schematic/definitions/src/elevenlabs/types/speech.rs
  - schematic/definitions/src/emqx/mod.rs
  - schematic/definitions/src/eversolo/mod.rs
  - schematic/definitions/src/gitea/mod.rs
  - schematic/definitions/src/github/mod.rs
  - schematic/definitions/src/gitlab/mod.rs
  - schematic/definitions/src/huggingface/mod.rs
  - schematic/definitions/src/lmstudio/mod.rs
  - schematic/definitions/src/lmstudio/types.rs
  - schematic/definitions/src/ollama/mod.rs
  - schematic/definitions/src/ollama/types/completion.rs
  - schematic/definitions/src/ollama/types/models.rs
  - schematic/definitions/src/ollama/types/openai.rs
  - schematic/definitions/src/openai/mod.rs
  - schematic/definitions/src/openai_pagination.rs
  - schematic/definitions/src/registry/typed.rs
  - schematic/definitions/src/samsung_smart_tv/mod.rs
  - schematic/definitions/src/unfolded_circle/core_rest/mod.rs
  - schematic/definitions/tests/authoring_surface.rs
  - schematic/gen/Cargo.toml
  - schematic/gen/src/codegen/api_struct/helpers.rs
  - schematic/gen/src/codegen/client/helpers.rs
  - schematic/gen/src/codegen/client/methods.rs
  - schematic/gen/src/codegen/client/mod.rs
  - schematic/gen/src/codegen/client/shared_auth.rs
  - schematic/gen/src/codegen/client/tests.rs
  - schematic/gen/src/codegen/endpoint_methods/names.rs
  - schematic/gen/src/codegen/endpoint_methods/tests.rs
  - schematic/gen/src/codegen/error.rs
  - schematic/gen/src/codegen/graphql.rs
  - schematic/gen/src/codegen/mod.rs
  - schematic/gen/src/codegen/module_docs.rs
  - schematic/gen/src/codegen/pagination/mod.rs
  - schematic/gen/src/codegen/pagination/tests.rs
  - schematic/gen/src/codegen/request_enum.rs
  - schematic/gen/src/codegen/request_structs/endpoint_spec.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/src/codegen/responses.rs
  - schematic/gen/src/codegen/variant.rs
  - schematic/gen/src/definition_gen.rs
  - schematic/gen/src/diagnostics.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/item.rs
  - schematic/gen/src/export/postman/tests.rs
  - schematic/gen/src/export/vocabulary.rs
  - schematic/gen/src/lib.rs
  - schematic/gen/src/output/assemble/api.rs
  - schematic/gen/src/output/mod.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/tests/common/components.rs
  - schematic/gen/tests/l1/definition_baseline.rs
  - schematic/gen/tests/l1/e2e_generation.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/http_client.rs
  - schematic/gen/tests/l1/main.rs
  - schematic/gen/tests/l1/openapi_strict_completeness.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/gen/tests/l1/pagination_validation.rs
  - schematic/gen/tests/l1/path_substitution.rs
  - schematic/gen/tests/l1/postman_golden.rs
  - schematic/gen/tests/l1/query_param_detection.rs
  - schematic/gen/tests/l1/query_params_codegen.rs
  - schematic/gen/tests/l1/response_adapters.rs
  - schematic/gen/tests/l1/vocabulary_exports.rs
  - schematic/openapi/anthropic.json
  - schematic/openapi/bitbucket.json
  - schematic/openapi/elevenlabs.json
  - schematic/openapi/github.json
  - schematic/openapi/lmstudio.json
  - schematic/openapi/ollama.json
  - schematic/openapi/openai.json
  - schematic/postman/elevenlabs.postman_collection.json
  - schematic/postman/lmstudio.postman_collection.json
  - schematic/postman/ollama.postman_collection.json
  - schematic/postman/openai.postman_collection.json
  - schematic/runtime/Cargo.toml
  - schematic/runtime/src/api_client.rs
  - schematic/runtime/src/client.rs
  - schematic/runtime/src/codec.rs
  - schematic/runtime/src/endpoint.rs
  - schematic/runtime/src/error.rs
  - schematic/runtime/src/graphql.rs
  - schematic/runtime/src/lib.rs
  - schematic/runtime/src/pager/mod.rs
  - schematic/runtime/src/pager/pages.rs
  - schematic/runtime/src/pager/request.rs
  - schematic/runtime/src/response.rs
  - schematic/runtime/src/shapes.rs
  - schematic/runtime/src/stream.rs
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_a_non_json_endpoint.stderr
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_a_stream_endpoint.rs
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_a_stream_endpoint.stderr
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_mixed_alternatives.rs
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_mixed_alternatives.stderr
  - schematic/runtime/tests/l1/decode_boundary.rs
  - schematic/runtime/tests/l1/envelope.rs
  - schematic/runtime/tests/l1/error_routes.rs
  - schematic/runtime/tests/l1/graphql.rs
  - schematic/runtime/tests/l1/graphql_address.rs
  - schematic/runtime/tests/l1/graphql_pager.rs
  - schematic/runtime/tests/l1/main.rs
  - schematic/runtime/tests/l1/shapes.rs
  - schematic/runtime/tests/l1/streams.rs
  - schematic/runtime/tests/l1/support.rs
  - schematic/runtime/tests/l1/timing.rs
  - schematic/runtime/tests/l1/whole_send_elapsed.rs
  - schematic/schema/src/anthropic.rs
  - schematic/schema/src/bitbucket/requests.rs
  - schematic/schema/src/elevenlabs/requests.rs
  - schematic/schema/src/lib.rs
  - schematic/schema/src/lmstudio.rs
  - schematic/schema/src/ollama.rs
  - schematic/schema/src/ollama/mod.rs
  - schematic/schema/src/ollama/requests.rs
  - schematic/schema/src/ollama/responses.rs
  - schematic/schema/src/openai/requests.rs
  - schematic/schema/tests/non_json_endpoints.rs
  - schematic/schema/tests/streaming_providers.rs
  - sniff/lib/src/remote/bitbucket.rs
  - sniff/lib/src/remote/envelope.rs
  - sniff/lib/tests/l1/remote_providers.rs
documentation:
  - .claude/skills/schematic-define/SKILL.md
  - .claude/skills/schematic-define/schematic/SKILL.md
  - .claude/skills/schematic-define/schematic/define.md
  - .claude/skills/schematic-define/schematic/definitions.md
  - .claude/skills/schematic-define/schematic/gen.md
  - .claude/skills/schematic-define/schematic/openapi.md
  - .claude/skills/schematic-define/schematic/pagination.md
  - .claude/skills/schematic-define/schematic/responses.md
  - .claude/skills/sniff/remote-and-repository.md
  - schematic/README.md
  - schematic/define/README.md
  - schematic/definitions/README.md
  - schematic/docs/definition-builders.md
  - schematic/docs/dependencies.md
  - schematic/docs/generation/generated-clients.md
  - schematic/docs/generation/validation.md
  - schematic/docs/generation/workflow.md
  - schematic/docs/graphql.md
  - schematic/docs/io/crates-openapi.md
  - schematic/docs/io/definition-bundles.md
  - schematic/docs/io/export-openapi.md
  - schematic/docs/io/export-postman.md
  - schematic/docs/io/import-openapi.md
  - schematic/docs/io/openapi-extensions.md
  - schematic/docs/pagination.md
  - schematic/docs/responses-and-errors.md
  - schematic/docs/runtime/README.md
  - schematic/docs/runtime/building-clients.md
  - schematic/docs/runtime/calling-an-api.md
  - schematic/docs/runtime/codecs.md
  - schematic/docs/runtime/deadlines-and-timeouts.md
  - schematic/docs/runtime/hooks.md
  - schematic/docs/runtime/sending-and-responses.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/runtime/README.md
  - schematic/schema/README.md
completed_phase: 8
implemented: true
---

# Schematic response model v2 and GraphQL implementation plan

## Work and successful completion

Replace the single response declaration with success shapes and effective HTTP error tables; add incremental SSE, NDJSON, and byte streams; let one endpoint (one `(method, path)`) declare several wire-distinguishable reply shapes as a tagged enum; and generate typed GraphQL operations through the existing generic send. Update generation, OpenAPI import/export, Postman, isolated definition bundles, the four provider definitions, and the Bitbucket source and OpenAI `GetResponse` endpoints (R32a, R32b) together. Preserve untouched definition call sites and single-shape Rust success types.

Success means R1–R44, including R3a and R11a–R11c, have observable passing checks; generated artifacts compile and regenerate deterministically; affected dependents pass; and current docs, skills, and the implementation log describe the resulting behavior. Report evidence separately for macOS, Linux, native Windows, and WSL2. Missing required evidence remains pending, even if implementation is otherwise ready for review. The terminal lifecycle state is **implementation complete, ready for review**; the author closes the feature.

This is a plan for the tree after the prerequisite reviews, not an instruction to implement their missing work here. As of the 2026-10-22 refresh, every prerequisite has landed and `schematic-runtime` exists at `schematic/runtime`; the definition still stores `Endpoint::response: ApiResponse`, which Phase 5 replaces. Phase 1 verifies the landed contracts before code changes.

Execution conventions:

- Waves are globally numbered. Tasks in a wave may run concurrently only where explicitly marked; the next wave starts after its prerequisites and checkpoint pass. A singleton wave is a sequential step.
- Parallel implementation agents receive bounded ownership of files, the non-interactive-session constraint, and explicit contracts. A coordinator owns shared module exports, manifests, recipes, generated artifacts, snapshots, and integration. Never have two agents regenerate or update the same files concurrently.
- Phases 2–4 are additive and keep the legacy response path green. Phase 5 performs the atomic switch; temporary compile failures inside that phase are expected, but it cannot finish with a broken tree. Do not introduce a persistent second public response system to avoid the switch.
- Keep behavior-changing comments, relevant topic docs, and the implementation log current within each phase. Phase 8 audits them rather than postponing all documentation until then.
- Use nextest through the repository recipes and the Test Toolkit. Local mock-server tests are L1. No provider calls, terminal focus changes, commits, lifecycle moves, or `cargo fmt` are part of this plan.

## Phase 1: Resolve prerequisites and establish the baseline

### Necessary Rules

1. **Prerequisites — landed (refreshed 2026-10-22).** Specs 1–3 are completed; `2026-10-14-schematic-definition-api` and `2026-10-15-schematic-definition-vocabulary` are implemented, and the latter's review 1 rulings are recorded (all six accept the implemented behavior). The spec's reader's note of 2026-10-22 lists the landed surface. In short: `Success(ApiResponse)` and `ErrorSet::none()` placeholders, plus the `.returns*`/`.errors` builders in `schematic/define/src/builders.rs`, which today reject a second success; `DefinitionContext` in `define/src/registration.rs`; 4b metadata (`rate_limit`, `retry_guidance`, `deprecated`, `EndpointAuth`) in `define/src/metadata.rs` and `types.rs`; `DefinitionBundle` version 1 in `define/src/bundle.rs`; x-schematic extensions in `define/src/openapi/extensions.rs`; baselines in `gen/tests/l1/definition_baseline.rs` (`adoptions()`), `artifact_drift`, and `definition_bundles`; `schematic-runtime` at `ABI = 5` with `RequestParts::query: Vec<(String, QueryValue)>`, `EndpointSpec`, `ErrorKind`, `NoDeclaredError`, `Decode`, `SupportsHooks`, and `send_raw`. Conform to the landed code: where a name, path, or count here differs, follow the code and log the difference. Do not rebuild prerequisite features.
2. **Authorization — granted.** The author confirmed the spec's pre-authorizations on 2026-10-21: regeneration/deletion of owned output, runtime/test dependencies and lockfile changes, the four provider migrations plus the R32a/R32b endpoint edits, and forced dependent edits. Log each use. The spec's "Not pre-authorized" list still applies. Stale `gen/schematic` removal remains optional and requires proof that it is unreferenced and generator-owned.
3. **Default adoption.** Adopt D-4–D-31 as written (D-1–D-3 are withdrawn by the author ruling of 2026-10-12, `2026-10-11-schematic-generation-integrity` review 1, item 1) unless prerequisite review outcomes supersede them; record each separately with choice, alternatives, reason, phase, and evidence. D-17 and the end-marker behavior in D-18 are already decided, not new author questions. Names remain proposed except `EndpointSpec`. Conform to landed `Success`, `ErrorSet`, registration, pagination, and runtime contracts rather than inventing replacements.
4. **Selection and precedence.** Apply R3/R3a: normalized concrete media types, 204-empty priority, status-specific candidate sets before status-less fallback, and `UnexpectedContentType` for an unselectable multi-shape 2xx. Then R3b: JSON shapes sharing a media type are separated by a declared top-level discriminator value or by array versus object; the R3c fallback takes any unmatched media type. Single-shape endpoints remain unchecked by content type and optional statuses. Apply exact-then-narrower-range precedence across the merged error table; endpoint entries replace only identical matches. Equal-width overlapping ranges and duplicate matches at one level fail validation.
5. **One endpoint per address and naming.** Endpoint identity stays spec 1's `(method, normalized path)`; no selector exists, nothing is injected into requests, and fields such as `stream` are ordinary optional fields sent as the caller set them (R13a; R13–R15 withdrawn). Shapes no tag separates fail validation. Shape notes are documentation only (variant rustdoc, OpenAPI response description). Use the specified success/failure enum naming defaults, with author-named variants for R3b groups and Bitbucket (`Directory`, `File`).
6. **Stream contract.** End markers are consumed unless `keep` is declared; Ollama keeps its final statistics. Item-level malformed frames continue; malformed retained terminators end. Early close, interruption, idle timeout, deadline expiry, and frame-limit breach each terminate with one distinct error. The 16 MiB limit counts assembled SSE/NDJSON frames, not HTTP chunks; byte streams bypass it. Retention and diagnostics remain bounded, and no background reader or full-body copy is added.
7. **Idle clock.** Interpret R11a's clock as starting at headers, then the previously yielded item, rather than restarting on every poll. Application delay cannot extend a shared deadline. Resolve implementation details against the landed runtime; record whether pending buffered items affect idle timing and test the chosen behavior. This detail is not permission to change the decided timeout semantics.
8. **GraphQL scope.** Opaque query/mutation documents, POST, JSON-only Accept, unit variables omitted, all non-2xx replies on the HTTP error path. Valid nonempty errors take precedence over typed-data decoding. GraphQL declared errors survive HTTP-error opt-out. Global hooks see the 2xx wrapper; typed mutators see only successful data. Pagination reads decoded data and injects typed variables through existing body-path machinery. No subscriptions, document parser, batching, provider migration in sniff, or shipped GitHub GraphQL definition.
9. **Export boundaries.** OpenAPI and Postman skip GraphQL with nonfatal contextual diagnostics here. A multi-shape endpoint exports as one operation: JSON shapes sharing a status and media type become a `oneOf` with a `discriminator` when R3b declares a field, and the fallback exports under `*/*`; re-import yields the same multi-shape endpoint. Export resolved error ranges faithfully, expanding affected classes where necessary. Reading `x-schematic` back is deferred; do not add fidelity work from the next spec.
10. **Provider evidence.** Verify current provider documentation during implementation, recording URL/date and endpoint-specific framing, termination, error events, and audio defaults. Research labels alone do not establish framing; ElevenLabs audio remains raw bytes. Missing evidence gets a documented conservative choice allowed by the spec, or an explicit unmet requirement where no allowed choice exists. Do not silently invent `done=true` or `[DONE]` for unrelated endpoints.
11. **Cost and validation rulings.** Existing typed-send and pager spikes already answer trait/wrapper compile cost and pager size; no repeat performance spike or multi-host benchmark. Mock tests prove incremental delivery and ownership, not live-provider behavior or HTTP/2 connection closure. Do not expand CI environments, execution bans, or scope; use the existing policy. Load the `os` skill before assessing remote availability or editing platform/path code.
12. **Input rules.** Apply the matrices below only to changed file readers: OpenAPI JSON/YAML and definition-bundle formats actually supported by the prerequisite loader. GraphQL HTTP replies and stream framing need protocol tests, not a configuration matrix. Unsupported bundle formats are not added to satisfy this plan. The matrices distinguish absent, null, invalid, and empty; JSON duplicate object members and YAML duplicate mapping keys are rejected for these structural fields. Preserve arbitrary JSON inside schemas/examples rather than treating it as malformed configuration.

### Wave 1 — Independent inventories (parallel, read-only)

- [x] **Prerequisite inventory**
  - Read prerequisite logs and review outcomes; map their final API to every refresh note in this spec. Verify authorization separately from feature defaults. Identify the runtime package and whether current area recipes include it.
  - Deliver a contract inventory and prerequisite/authorization status in `implementation-log.md`; coordinator merges parallel findings into that file.
- [x] **Consumer inventory**
  - Search all `ApiResponse` readers, endpoint counts by variant, streaming body fields, hooks, public generated types, extension/export/import code, and dependent call sites. Recheck the reported stranded Ollama types and the roughly 530 untouched endpoint definitions (539 endpoints in all at the 2026-10-22 refresh) rather than trusting counts.
  - Inventory sniff's eight fixtures and check-status document/variables. Search `sniff/lib`, `biscuit-speaks/lib`, `model-citizen/lib`, and all five named homelab crates, plus any additional actual consumers; record zero-reference results.
- [x] **Provider inventory**
  - Review definitions and research for Ollama, LM Studio, Anthropic, and ElevenLabs. Build an endpoint migration table: old/new success and failure types, shape set and wire tags, framing, terminator/keep, documented provider-error variants, content type/default codec, hook removal, and evidence source.
  - Add the Bitbucket source endpoint and OpenAI `GetResponse` (R32a, R32b) to the table; for Bitbucket, record what the provider sends for a `.json` file versus a directory listing (open research item).
  - Identify streaming or multi-shape definitions outside these and log them without editing them.

### Wave 2 — Baseline and targeted spike (sequential coordinator)

- [x] **Baseline capture**
  - After Wave 1 clears the gates, create the implementation log with stable feature identifier, `plan.md`, implementer, per-phase source/docs/skill files, packages touched, default decisions, deviations, and review agenda entries.
  - Regenerate Rust/OpenAPI/Postman into an isolated output directory and compare against the committed artifacts as left by `2026-10-15-schematic-definition-vocabulary` (`just migration-check` proves this). Resolve unexplained drift before proceeding; preserve original snapshots and define a field-specific comparison allow-list.
  - Capture public type signatures and untouched definition source files for R24. Run baseline L1 checks for affected Schematic packages, including the actual runtime and generated schema manifest. Preserve existing user changes.
- [x] **Timeout spike**
  - Once only, before runtime codec work, inspect the landed HTTP transport for a whole-body request timeout that would incorrectly kill a healthy stream. If prior runtime evidence already proves R11a, record that ruling and skip the experiment.
  - Otherwise use one local server and a minimal request with short controlled intervals to demonstrate header timeout, a stream living past that timeout while emitting items, and idle expiry. Record the smallest integration change needed to separate header/idle/shared-deadline bounds. This is a correctness spike, not a benchmark or an HTTP/2/OS matrix; later acceptance tests implement the contract without rerunning the spike.
- [x] **Contract freeze**
  - Freeze stored field names, runtime codec/error interfaces, generated enum names, diagnostics, and bundle format outcomes. Expand the matrix columns below to the exact landed wire fields, including nested variant payloads and any reused field whose reader changes. Select real-tool positive fixtures and test locations before parser work.
  - Make an R1–R44 ledger naming implementation task, public test boundary, feature flags, and verification command. Record remaining proposed rulings for review 1 without stopping for optional design feedback.

**Validation checkpoint:** the landed prerequisite contracts are verified and authorization is recorded; baseline is explained and green; all defaults and scope boundaries are recorded; exact file ownership and input contracts are available for Phase 2. Only a landed contract that is genuinely absent (not merely renamed or moved) ends the run here; a difference is conformed to and logged.

### Input Robustness Matrix

The following are planning outcomes for changed readers. Expand notation into exact field columns after prerequisite refresh; do not mark the freeze task complete with placeholder fields. Every table has a positive control. `Reject` means the public import/load operation fails with contextual diagnostics and yields no usable definition; it does not mean checking only a private parser return. `N/A` cells must still exercise an equivalent malformed member of the field's actual shape.

For optional bundle metadata newly introduced here, absence is explicitly permitted; explicit null is rejected unless the prerequisite's published format already defines it as a distinct supported value. Preserve that prior contract and spell out the exception in the frozen table. Empty statuses explicitly mean no status restriction; an empty API error table is not endpoint opt-out.

**OpenAPI input — run separately for JSON and YAML through the public importer.** Status/media names are object keys; malformed keys are load-bearing input mutations too.

| Shape | operation `responses` | response `content` | media entry `schema` | response `$ref` | media `oneOf` members | `discriminator.propertyName` |
| --- | --- | --- | --- | --- | --- | --- |
| Control | Import successes and HTTP errors | Preserve every declared media type | Register shape/item schema | Resolve local response | Separable members become JSON shapes | Members become discriminator-tagged shapes |
| Absent | Reject: no representable success | No-content success becomes empty; preserve status | JSON/stream fallback to Value with diagnostic | Use inline response | Single schema | Separate by array/object, else single-schema import with diagnostic |
| Explicit null | Reject | Reject | Reject | Reject | Reject | Reject |
| Wrong whole type | Reject | Reject | Reject | Reject | Reject | Reject |
| One invalid member | Reject invalid status/value | Reject invalid media/value | Reject malformed structural keyword | Reject malformed reference | Reject invalid schema enum member | Reject non-string member |
| All invalid members | Reject | Reject | Reject | Reject | Reject | Reject |
| Empty | Reject: no success | No-content success becomes empty | Valid unconstrained schema; register Value | Reject empty reference | Reject empty `oneOf` | Reject empty property name |
| Duplicate key | Reject | Reject | Reject structural duplicate | Reject | Reject duplicated mapping key | Reject duplicated mapping key |
| Trailing/invalid document | Reject | Reject | Reject | Reject | Reject | Reject |

For `oneOf` members, "invalid" means a member that is not a schema or reference; inseparable valid members are not a parse error but import as one schema with a diagnostic (R28). Preserve existing unrelated schema handling. A no-content error entry retains its empty error shape; an operation with only errors still fails for lack of representable success. Status keys `default`, 1xx/3xx, unsupported error-media alternatives, and invalid media keys (wildcards other than `*/*`, which maps to the R3c fallback) also get explicit R28 diagnostic tests, separate from syntactic rejection.

**Definition bundle — supported format(s), public isolated load followed by validation/export.** These columns identify semantic fields; freeze exact serialized paths in Wave 2.

| Shape | endpoint success set | API error entries | endpoint error entries | endpoint opt-out flag | shape content type | shape statuses | shape fallback flag | shape note |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Control | Load every shape | Resolve inherited errors | Resolve added/overridden errors | Disable HTTP declarations | Preserve normalized media | Select/export exact 2xx | Select unmatched media | Reach rustdoc/export |
| Absent | Reject | Published absence means no API defaults | Inherit API entries | Inherit/overlay policy | Use shape default | No restriction | Not a fallback | No note |
| Explicit null | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |
| Wrong whole type | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |
| One invalid member | Reject whole set | Reject whole table | Reject whole table | Reject non-bool | Reject non-string/invalid media | Reject invalid/non-2xx status | Reject non-bool; reject second fallback | Reject non-string |
| All invalid members | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |
| Empty | Reject | Explicit no defaults | Explicit no additions; still inherit | N/A: reject empty container | Reject empty media string | Explicit no restriction | N/A: reject empty container | Reject empty string |
| Duplicate key | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |
| Trailing/invalid document | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |

**Definition bundle — nested required variant payloads.** Each required payload is checked when its containing variant is present, not when an optional parent is omitted.

| Shape | response shape tag | JSON/item descriptor | stream framing tag | stream end tag | field-end path | field-end value | end keep flag | error status match | error shape descriptor | JSON tag field name | JSON tag scalar value |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Control | Select correct shape | Resolve canonical binding/schema | Select SSE/NDJSON/Bytes | Apply terminator/close | Match supported path | Match declared scalar | Deliver final item | Apply specificity | Decode declared shape | Select by top-level field | Select by distinct value |
| Absent | Reject | Reject for JSON/SSE/NDJSON; absent for Bytes | Reject | ConnectionClose only if published default | Reject for field end | Reject for field end | Consumed default | Reject | Reject | Absent unless the tag is `Field` | Reject for `Field` |
| Explicit null | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |
| Wrong whole type | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |
| One invalid member | Reject | Reject | Reject | Reject | Reject fan-out/invalid path | Reject non-scalar | Reject | Reject bad bound/status | Reject | Reject nested/invalid name | Reject non-scalar |
| All invalid members | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |
| Empty | Reject | Reject dangling/empty descriptor | Reject | Reject empty variant payload | Reject empty path | Empty string allowed; empty collection rejected | Reject empty container | Reject empty/unordered range | Reject | Reject | Empty string allowed; collection rejected |
| Duplicate key | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |
| Trailing/invalid document | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |

Expand error status match into exact status and range lower/upper columns, and end tags into data-marker/event-name columns; required strings reject absent/null/wrong types, accept an empty string only where it is a meaningful exact terminator, and otherwise reject empty. These columns must test end/framing compatibility, range-domain bounds, and dangling descriptors through the loaded definition's validation/export result.

**Definition bundle — GraphQL and address fields.** Split the operation columns across separate tables in the frozen contract if needed for readability.

| Shape | API operations | operation id | document | variables descriptor | data descriptor | operation name | operation error policy | operation pagination | address rules |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Control | Load operation collection | Resolve naming space | Preserve exact opaque text | Resolve typed variables | Resolve typed data | Preserve configured name | Resolve HTTP failures | Export/load Relay settings | Derive correct URL |
| Absent | No operations by defined default | Reject | Reject | Reject unless explicit unit marker | Reject | Omit wire field | Inherit API defaults | No pager | Reject for declared GraphQL surface unless published preset |
| Explicit null | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |
| Wrong whole type | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |
| One invalid member | Reject whole collection | Reject invalid name/collision | Reject non-string | Reject dangling binding | Reject dangling binding | Reject non-string | Reject whole table | Reject invalid strategy/path | Reject invalid rule |
| All invalid members | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |
| Empty | Explicit no operations | Reject | Send unchanged; server validates | Explicit unit marker only; reject empty descriptor | Reject | Preserve explicit empty string; no document parsing | Explicit empty overlay inherits; explicit none opts out | Reject empty strategy | Reject for GraphQL: no address rule |
| Duplicate key | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |
| Trailing/invalid document | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject | Reject |

Address-rule tag and required `suffix`, replacement, and append-path payloads, pagination input/output path and knob fields, schema-catalog entries, and Rust binding references each need their own frozen columns where their reader changes. Reuse the prerequisite matrix for unchanged fields; verify rather than widen the existing bundle format/version contract.

**Matrix test construction:** start from actual Schematic-exported OpenAPI JSON/YAML and actual registered/bundled definitions containing the load-bearing features. One data-driven public test per supported format walks all its tables, independently applying one edit per cell to the unedited fixture. Control results must prove the relevant success/error/wire-tag/GraphQL result before mutations. Assert resulting public definition/export or contextual rejection, never just deserializer `Ok`/`Err`. Grep changed readers for load-bearing serde defaults, absent/null-collapsing `Option`, `filter_map(..as_str())`, `unwrap_or_default()`, and `.ok()`; explain legitimate uses. A review finding audits the entire row and column, not only the reported cell.

## Phase 2: Add definition vocabulary and validation

### Wave 3 — Stored types (sequential)

- [x] **Vocabulary types**
  - Add/widen `Success`, stored success shapes, non-exhaustive stream framing, end/keep forms, `StatusMatch`, `ErrorSet`, R3b JSON tags, the R3c fallback marker, and shape notes in `schematic-define`; preserve old response storage until Phase 5. Keep existing `.returns(...)`, `.errors(ErrorSet::none())`, and API defaults call sites working.
  - Add registered GraphQL operation/address primitives and context entry points, including typed variables/data, explicit unit variables, operation name, HTTP error policy, and existing pagination declarations. Do not parse documents.
  - Derive serialization/equality and surrounding Debug/Clone traits; register descriptor closure for success, error, stream items, variables, and data in bundles without registering builtin bytes/unit.

### Wave 4 — Independent rules (parallel after Wave 3)

- [x] **Shape validation**
  - Implement normalized media/status distinguishability and empty/stream constraints; R3b separability (shared discriminator field with distinct canonical scalar values, or one array and one object shape) and R3c fallback limits; spec 1's `(method, path)` duplicate rule stays unchanged. Reuse rules from generator/importer rather than copying them. Cover R1, R3a–R3c, R13a, R16–R18, R23.
- [x] **GraphQL validation**
  - Validate registration, shared generated naming collisions after module/suffix resolution, address rules, and Relay/body-path compatibility. Test wrong variables at compile time and unregistered data at definition validation. Extend bundle reference validation and apply frozen matrix cases for these types.

**Validation checkpoint:** definition tests pass with the old stored response path; serialization is deterministic and isolated references resolve; specificity/overlap and inseparable-shape rejection cases produce contextual diagnostics. Record finalized public names and construction seams for runtime and generator work.

## Phase 3: Implement runtime decoding and streams

### Wave 5 — Independent codecs (parallel after Phase 2)

- [x] **Stream codecs**
  - Implement caller-polled owned SSE, NDJSON, and byte streams. Handle split UTF-8/delimiters, CR/LF, comments, multiline SSE data, blank NDJSON lines, and clean EOF final lines. Match consumed terminators before decoding and support retained terminators.
  - Add distinct malformed, early-close, interrupted, idle-timeout, deadline, and bounded frame-limit errors; preserve safe causes/pending bytes. Integrate the header/idle clock from the spike and expose the managed builder's default 16 MiB limit. Dropping the stream releases the body without spawned work; streams remain usable after dropping the client and are Send.
- [x] **Response dispatch**
  - Implement multi-shape status/media/204 selection and HTTP error lookup/decode using the frozen tables. Preserve status, headers, original raw buffered bodies, and spec 2 interruption/redaction behavior; produce Declared, MalformedError, Undeclared, or UnexpectedContentType as specified.
  - Add JSON-mutator marker support only for all-JSON multi-shape alternatives; exclude stream/text/binary codecs. Exercise every specificity and single-shape compatibility branch through public envelopes.
- [x] **GraphQL decoding**
  - Implement wrapper-first classification, typed errors with unknown fields, valid paths/locations, usable typed partial data only, and the composite GraphQL/HTTP failure type. Test errors-only, absent/null/bad data, empty/malformed errors, wrong error members, 2xx statuses, HTTP failures and HTTP opt-out.
  - Global JSON hooks see the wrapper once on 2xx; typed mutators run only on failure-free decoded data. Preserve original bytes on hook failures and skip the global hook on non-2xx.

### Wave 6 — Runtime integration (sequential)

- [x] **Runtime verification**
  - Wire shared exports, codecs, builder options, and manifests. Test R6–R12/R19/R37/R38/R41 through public send/envelope/stream results, including a delayed first item and immediate envelope availability.
  - Prove exactly one terminal error, malformed-item recovery, final retained statistics, final-line EOF, interrupted EOF without EndedEarly, oversize pending frames, and no full-body retention/background consumption. Use synchronized server signals, dynamic ports, and owned cleanup.
  - Prove the required one-item-per-second stream exceeds a 2 s idle timeout while healthy, and a silent stream expires. Keep elapsed-contract L1 cases under the repository 5 s limit; use short controlled durations for other cases and no readiness sleeps/retries. Add compile-time Send/ownership checks and hook compile-fail cases.

**Validation checkpoint:** runtime L1 passes, legacy sends still pass, codecs satisfy the generator contract, and the timeout spike is resolved. No generator fixture waits for types planned for a later phase.

## Phase 4: Prepare generator and consumer adapters

### Wave 7 — Code generation (parallel after Phase 3)

- [x] **REST generation**
  - Extend codegen request structs/traits/enums/module docs to emit plain or enum success/failure types, effective error lookup, stream codec metadata, and shape notes as variant rustdoc. Nothing is injected into request bodies. No convenience send wrapper methods.
  - Add fixtures for single/multiple successes, exact-status-distinct JSON, discriminator- and array/object-distinct JSON, a fallback shape, inherited/overlaid/opted-out errors, and hook marker eligibility. Keep the old production response path active.
- [x] **GraphQL generation**
  - Emit typed operation requests using `EndpointSpec`, POST body constants/typed variables, explicit operationName, composite failures, managed address override, and the existing generic send. Handle URL segments/proxy prefixes/trailing slashes, builder-invalid URLs, and cross-origin checks.
  - Prepare pagination integration using landed Relay/body-path contracts and freeze generated naming collision tests. Write generated fixture tests without adding shipped GitHub operations.
- [x] **Consumer adapters**
  - Prepare reusable v2 mapping functions for OpenAPI export/import, Postman first-shape content type, and `x-schematic` writes behind the legacy production adapters. Own disjoint exporter/importer/extension files; coordinator resolves shared imports.
  - Include status/media aggregation, `oneOf`/`discriminator` and `*/*` fallback export, resolved-range expansion, normalized import classification and contextual loss diagnostics. Implement the matrix's parser-boundary checks where supported upstream parsers would otherwise hide null, invalid members, duplicates, or trailing content. No extension reader.

### Wave 8 — Compile fixtures (sequential)

- [x] **Fixture integration**
  - Wire additive modules/manifests and generate isolated fixture crates. Compile through the actual runtime and run a representative generated stream/multi-shape/error/GraphQL request against local mocks.
  - Run compile-fail cases for wrong GraphQL variables and invalid hook registration on streams, text, binary, and mixed JSON alternatives; prove all-JSON alternatives remain eligible. Validate module/suffix/body/enum naming collisions.

**Validation checkpoint:** generator and define L1 pass; real fixture code compiles; adapter functions are ready to consume the new stored form; untouched production output remains at the explained baseline. Coordinate manifests and output directories rather than letting parallel agents mutate them.

## Phase 5: Switch storage and migrate the four providers atomically

### Wave 9 — Storage switch (sequential coordinator)

- [x] **Response switch**
  - Change `Endpoint::response` and API error/GraphQL fields to the finalized model; widen builder lowering in place and preserve explicit none versus omitted/inherit behavior. Activate Phase 4 generator, validator, importer, exporter, and extension adapters; remove the temporary legacy path after all consumers switch.
  - Update self-contained bundle loading, feature exports, and serde comparisons. Preserve original snapshots and narrowly enumerate response/error/GraphQL field differences. Reject dangling registrations and audit the matrix before accepting a baseline update.

### Wave 10 — Provider definitions (parallel by disjoint provider directory)

- [x] **Ollama migration**
  - Apply the Phase 1 endpoint inventory: native NDJSON and appropriate retained terminators, generate/chat as single endpoints with NDJSON and JSON shapes chosen by `Content-Type` (`stream` stays an optional field), and OpenAI-compatible SSE `[DONE]`. Register reachable stream types and account for every reported stranded type. Keep documented provider-error events typed and observable.
- [x] **LM Studio migration**
  - Replace both streaming binary declarations with registered SSE events and evidence-supported end conditions; use connection close where appropriate. Include or explicitly record unsupported documented error events.
- [x] **Anthropic migration**
  - Give the single CreateMessage endpoint a JSON shape and an SSE shape with registered Messages event variants, consuming message_stop by default; `stream` stays an optional field. Cover provider error events and both reply variants.
  - Bitbucket source endpoint (R32a): `Directory` JSON listing plus `File` fallback; record the `.json`-file finding. OpenAI `GetResponse` (R32b): add the SSE `ResponseStreamEvent` shape beside `Response`, with the documented end condition and its source.
- [x] **ElevenLabs migration**
  - Make StreamSpeech an incremental byte stream; select timestamp-stream framing from evidence. Correct documented binary/audio/archive media types without removing output-format query controls. Record research/provider discrepancies, defaults, and any removed hooks.

### Wave 11 — Regeneration and dependents (sequential)

- [x] **Artifact integration**
  - Regenerate `schematic-schema`, OpenAPI, and Postman from definitions. Never edit generated Rust/manifests by hand. Compare untouched definition source and public types against Phase 1 captures; update the explicit migration list for deliberate changes only.
  - Fix only forced dependent call sites found by the inventory; log every location and type change. Do not rewrite sniff's hand-written GraphQL client or adjacent dependent behavior.
  - Run definition/source-scan/round-trip and affected-package L1 checks, generated schema compile/L1/lint, and all actual dependent builds/tests. Ensure the runtime is included even if the area's recipe has not yet incorporated it.

**Validation checkpoint:** the atomic switch is complete and green; all production readers use v2; only the four allowed provider source areas changed; public-type differences match the log; baseline comparison still detects an unrelated description mutation. Do not finish this phase on adapters or stubs that discard declared behavior.

## Phase 6: Prove importer, exporter, and migration behavior

### Wave 12 — Export/import checks (parallel by test ownership)

- [x] **OpenAPI coverage**
  - Assert merged response media/status maps, all success/error shape kinds, preserved class ranges and expanded winning statuses, stream metadata, `oneOf` with `discriminator` for tagged JSON groups, `*/*` for the fallback, and shape notes in response descriptions. Parse exported documents with `openapiv3`.
  - Assert real ElevenLabs content types and query-controlled defaults. Test `x-schematic` contains every new field plus rate-limit, deprecated, auth override, cross_origin, and request-id inherited from prerequisites.
- [x] **Import coverage**
  - Exercise normalized JSON/+json, SSE, NDJSON/jsonl, text, and binary mapping; deduplicate identical successes and preserve distinct same-media schemas with exact statuses. Test Value/connection-close fallback diagnostics, 204 empty, endpoint-level errors, default/1xx/3xx/multiple-error-media diagnostics, and no-success rejection.
  - Run JSON and YAML matrix walkers from real exported fixtures through public import/export. Verify multi-shape endpoints (including tagged `oneOf` groups and the fallback) round-trip, content types survive, and common HTTP errors are not hoisted.
- [x] **Postman coverage**
  - Update scoped golden expectations for first success media type and v2 storage. Assert contextual nonfatal GraphQL skip diagnostics in both exporters; do not implement future runnable GraphQL Postman requests.

### Wave 13 — Shipped runtime checks (sequential)

- [x] **Provider verification**
  - Through regenerated clients, prove one Ollama NDJSON and one SSE stream deliver incrementally, Anthropic returns each variant by `Content-Type` with the caller's `stream` sent unchanged, Bitbucket selects `Directory` and `File`, OpenAI `GetResponse` returns each variant, and ElevenLabs yields its first audio chunk before completion. Assert final Ollama statistics and typed provider-error events.
  - Run definition tests for every migrated endpoint from the inventory, confirming LM Studio framing/end evidence and ElevenLabs timestamp/media decisions. Complete provider evidence and hook-removal entries as findings occur.

**Validation checkpoint:** R5/R13a/R21/R26–R34 (including R32a/R32b)/R42 behavior passes, both input-format matrices pass, and exported multi-shape/range semantics agree with runtime lookup. Goldens and baseline updates explain individual differences rather than hiding them.

## Phase 7: Complete GraphQL integration and parity

### Wave 14 — Pagination and parity (parallel)

- [x] **Relay integration**
  - Complete existing cursor-strategy presets/body-path wiring: forward nodes/edges, optional backward leg, overridable variable names, author-declared size maximum, optional pager-managed variables, and decoded-data-relative paths.
  - Test two-page completion, null/omitted initial cursors, missing/invalid continuation, repeated cursors, limits/shared deadline, and GraphQL failure stopping collection without adding partial data. Compile-fail an unauthored pager knob.
- [x] **Check-status parity**
  - Generate a test-only operation from sniff's actual document/variables with nullable data models. Use all eight frozen fixtures with explicitly assigned HTTP status and JSON content type as R43 prescribes.
  - Assert typed success for success/passed/changed_head/null_rollup; usable partial failure for partial_errors; preserved errors and GitHub type for request_wide_errors/rate_limit; and raw HTTP failure for permission_failure at 403. Do not replicate sniff's domain decisions.
  - Declare cross-package fixture and source reads with `include_str!` or runtime manifest/repo-root joins recognized by the CI input index. Add only the required test `source-inputs` declaration for sniff's query source if needed; use the rust-testing rules and load rust-devops before changing CI metadata.

### Wave 15 — GraphQL contract checks (sequential)

- [x] **Operation verification**
  - Prove query and mutation POST bodies/headers, unit variables omission, unchanged invalid document strings, named/unnamed operations, anonymous credential reporting, same-origin overrides, Enterprise/proxy URL derivation, and REST path preservation through generated mocks.
  - Prove public `into_result()` fails on GraphQL and HTTP failures, while outer `?` handles transport errors only. Verify typed-data hooks versus wrapper hooks, malformed error structures, and server-errors precedence over undecodable data.
  - Load a generated definition bundle without Rust registry access, validate schemas/bindings/models, export it, and prove deterministic reserialization. Run the complete supported-format bundle matrix walkers and resolve dangling reference diagnostics with API/operation/type context.

**Validation checkpoint:** R35–R41/R43/R44 pass end to end; pagination uses the existing model; no GraphQL operation is added to shipped GitHub definitions; fixture reads are discoverable and work after nextest archive remapping.

## Phase 8: Audit documentation and final evidence

### Wave 16 — Docs and contract audit (parallel, disjoint ownership)

- [x] **Documentation audit**
  - Update READMEs and current topic docs for authoring success/error sets, several shapes at one address (wire tags, fallback, notes), incremental streams, timeouts/limits, provider-error item variants, GraphQL addresses/failures/hooks, and Relay pagination. Use compact examples and Mermaid stream/error-resolution flows for readers new to the repository.
  - Update `schematic/docs/io/import-openapi.md`, `export-openapi.md`, `export-postman.md`, and `openapi-extensions.md` with actual fidelity limits. Docs never name this feature or another spec by date/path. Update root/area dependency docs for any crate changes.
- [x] **Skill audit**
  - Update `.claude/skills/schematic-define/` and its Schematic generator/runtime guidance, plus other skills affected by actual architecture changes. Review comments/docs on every changed behavior-bearing symbol; fix drift and record it without unrelated cleanup.
  - Audit R1–R44 ledger, matrix rows/columns and parser-smell searches, public type changes, dependent call sites, and every human-review agenda entry. Disclosed deviations remain unmet criteria until accepted; log them without claiming passing evidence.

### Wave 17 — Final verification (sequential coordinator)

- [x] **Verification evidence**
  - Run `just test`, `just lint`, `just test-schema`, and `just lint-schema` from `schematic/`, ensuring runtime checks are included with its landed recipes. Use define's OpenAPI feature coverage; compile the generated fixture crate and generated schema via its own manifest because schema is currently excluded from the workspace.
  - Run root `just test {pkg}` and the applicable lint recipe for each affected dependent, including the five named homelab crates after confirming their package names. Run required L2 through `just test-l2` for affected areas using the shared harness without window focus. Do not use legacy recipes that invoke consolidated `cargo test`; use their nextest-backed equivalent.
  - Regenerate all owned outputs twice; the second run must produce no additional diff. Check definition source preservation, field-specific baselines, scoped artifact drift, and generated public-type differences. Explain any generated docs/type drift outside the migration set.
  - Load the `os` skill, reuse qualifying evidence per package/environment/tier, and obtain missing required evidence on macOS, Linux, native Windows, and WSL2 using authorized non-interactive hosts. Record platform-specific failures and any newly learned platform facts in the OS skill. Never turn inaccessible coverage into a pass or an execution ban.
- [x] **Review handoff**
  - Finalize `implementation-log.md` with all D-4–D-31 dispositions (D-1–D-3 withdrawn), names/locations, deviations and alternative choices, open proposed details, inventories, public type changes, removed hooks, provider sources/uncertainties, dependent edits, and environment evidence.
  - Report actual verified behavior and remaining blockers. When required checks pass, mark implementation complete and ready for review 1. Leave feature lifecycle placement and spec closure to the author; do not refresh downstream specs before review 1 supplies its outcomes.

**Validation checkpoint:** every ledger item points to passing evidence or an explicitly unresolved acceptance blocker; generated output is idempotent; docs and skills match behavior; all required platform and dependent checks are accounted for. No commit or lifecycle move is performed.
