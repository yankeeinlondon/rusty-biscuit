---
total_phases: 9
created: 2026-10-11
phase: 9
agent: codex/gpt-6.1-sol
yolo: "true"
source_files_during_phase_1:
  - Cargo.lock
  - schematic/define/src/cross_origin.rs
  - schematic/define/src/pagination.rs
  - schematic/define/src/pagination/resolve.rs
  - schematic/define/src/params/mod.rs
  - schematic/define/src/params/param_def.rs
  - schematic/define/src/types.rs
  - schematic/define/tests/definition_serde.rs
  - schematic/gen/Cargo.toml
  - schematic/gen/examples/capture_definition_baseline.rs
  - schematic/gen/examples/support/definition_snapshot.rs
  - schematic/gen/tests/l1/artifact_drift.rs
  - schematic/gen/tests/l1/definition_baseline.rs
  - schematic/gen/tests/l1/main.rs
  - schematic/justfile
  - schematic/features/2026-10-14-schematic-definition-api/spike/Cargo.lock
  - schematic/features/2026-10-14-schematic-definition-api/spike/Cargo.toml
  - schematic/features/2026-10-14-schematic-definition-api/spike/examples/mismatched_type.rs
  - schematic/features/2026-10-14-schematic-definition-api/spike/examples/misspelled_path.rs
  - schematic/features/2026-10-14-schematic-definition-api/spike/examples/private_descriptor.rs
  - schematic/features/2026-10-14-schematic-definition-api/spike/prove.py
  - schematic/features/2026-10-14-schematic-definition-api/spike/src/bin/emitted_path_source.rs
  - schematic/features/2026-10-14-schematic-definition-api/spike/src/lib.rs
  - schematic/gen/tests/fixtures/definition-baseline/anthropic.json
  - schematic/gen/tests/fixtures/definition-baseline/artificial-analysis-critpt.json
  - schematic/gen/tests/fixtures/definition-baseline/artificial-analysis-data.json
  - schematic/gen/tests/fixtures/definition-baseline/bitbucket.json
  - schematic/gen/tests/fixtures/definition-baseline/elevenlabs.json
  - schematic/gen/tests/fixtures/definition-baseline/emqx-basic.json
  - schematic/gen/tests/fixtures/definition-baseline/emqx-bearer.json
  - schematic/gen/tests/fixtures/definition-baseline/eversolo.json
  - schematic/gen/tests/fixtures/definition-baseline/gitea.json
  - schematic/gen/tests/fixtures/definition-baseline/github.json
  - schematic/gen/tests/fixtures/definition-baseline/gitlab.json
  - schematic/gen/tests/fixtures/definition-baseline/huggingface.json
  - schematic/gen/tests/fixtures/definition-baseline/lmstudio.json
  - schematic/gen/tests/fixtures/definition-baseline/ollama-native.json
  - schematic/gen/tests/fixtures/definition-baseline/ollama-openai.json
  - schematic/gen/tests/fixtures/definition-baseline/openai.json
  - schematic/gen/tests/fixtures/definition-baseline/samsung-smart-tv.json
  - schematic/gen/tests/fixtures/definition-baseline/unfolded-circle-core-rest.json
docs_updated_during_phase_1:
  - docs/dependencies.md
  - schematic/define/README.md
  - schematic/docs/dependencies.md
  - schematic/features/2026-10-14-schematic-definition-api/plan.md
  - schematic/features/2026-10-14-schematic-definition-api/spec.md
docs_created_during_phase_1:
  - schematic/docs/definition-snapshots.md
  - schematic/features/2026-10-14-schematic-definition-api/implementation-log.md
  - schematic/features/2026-10-14-schematic-definition-api/inventory.md
  - schematic/features/2026-10-14-schematic-definition-api/prerequisite-interfaces.md
  - schematic/features/2026-10-14-schematic-definition-api/reproduction.md
  - schematic/features/2026-10-14-schematic-definition-api/spike/README.md
skills_files_updated_during_phase_1:
  - .claude/skills/schematic-define/SKILL.md
  - .claude/skills/schematic-define/schematic/SKILL.md
packages:
  - schematic-runtime
  - schematic-define
  - schematic-gen
  - schematic-definitions
  - schematic-schema
  - sniff
source_files_during_phase_2:
  - Cargo.lock
  - schematic/define/Cargo.toml
  - schematic/define/src/lib.rs
  - schematic/define/src/models/type_ref.rs
  - schematic/define/src/registration.rs
  - schematic/define/tests/typed_registration.rs
  - schematic/definitions/src/registry/mod.rs
  - schematic/definitions/src/registry/typed.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
docs_updated_during_phase_2:
  - docs/dependencies.md
  - schematic/define/README.md
  - schematic/docs/dependencies.md
  - schematic/features/2026-10-14-schematic-definition-api/plan.md
  - schematic/features/2026-10-14-schematic-definition-api/implementation-log.md
  - schematic/features/2026-10-14-schematic-definition-api/spec.md
docs_created_during_phase_2:
  - schematic/docs/typed-registration.md
skills_files_updated_during_phase_2:
  - .claude/skills/schematic-define/SKILL.md
  - .claude/skills/schematic-define/schematic/SKILL.md
source_files_during_phase_3:
  - Cargo.lock
  - schematic/define/Cargo.toml
  - schematic/define/src/builders.rs
  - schematic/define/src/lib.rs
  - schematic/define/src/prelude.rs
  - schematic/define/src/schema.rs
  - schematic/define/src/registration.rs
  - schematic/define/src/params/mod.rs
  - schematic/define/src/pagination/resolve.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/paths.rs
  - schematic/define/src/openapi/export/request_body.rs
  - schematic/define/src/openapi/export/responses.rs
  - schematic/define/tests/builders.rs
  - schematic/define/tests/endpoint_literal_surface.rs
  - schematic/gen/src/output/mod.rs
  - schematic/gen/src/codegen/api_struct/tests.rs
  - schematic/gen/src/codegen/module_docs.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/src/definition_gen.rs
  - schematic/gen/tests/l1/http_client.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
docs_updated_during_phase_3:
  - docs/dependencies.md
  - schematic/define/README.md
  - schematic/docs/dependencies.md
  - schematic/docs/typed-registration.md
  - schematic/features/2026-10-14-schematic-definition-api/plan.md
  - schematic/features/2026-10-14-schematic-definition-api/implementation-log.md
  - schematic/features/2026-10-14-schematic-definition-api/spec.md
docs_created_during_phase_3:
  - schematic/docs/definition-builders.md
skills_files_updated_during_phase_3:
  - .claude/skills/schematic-define/SKILL.md
  - .claude/skills/schematic-define/schematic/SKILL.md
source_files_during_phase_4:
  - schematic/define/src/builders.rs
  - schematic/define/src/cross_origin.rs
  - schematic/define/src/lib.rs
  - schematic/define/src/oauth.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/parameters.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/openapi/import/mappings/parameters.rs
  - schematic/define/src/pagination.rs
  - schematic/define/src/pagination/resolve.rs
  - schematic/define/src/pagination/validate.rs
  - schematic/define/src/params/mod.rs
  - schematic/define/src/params/param_def.rs
  - schematic/define/src/request.rs
  - schematic/define/src/types.rs
  - schematic/define/tests/builders.rs
  - schematic/define/tests/cross_origin_policy.rs
  - schematic/define/tests/definition_forms.rs
  - schematic/define/tests/definition_serde.rs
  - schematic/define/tests/endpoint_literal_surface.rs
  - schematic/define/tests/pagination_support/mod.rs
  - schematic/definitions/src/anthropic/mod.rs
  - schematic/definitions/src/artificial_analysis/mod.rs
  - schematic/definitions/src/bitbucket/mod.rs
  - schematic/definitions/src/elevenlabs/endpoints/history.rs
  - schematic/definitions/src/elevenlabs/endpoints/sound_generation.rs
  - schematic/definitions/src/elevenlabs/endpoints/speech.rs
  - schematic/definitions/src/elevenlabs/endpoints/voices.rs
  - schematic/definitions/src/elevenlabs/endpoints/workspace.rs
  - schematic/definitions/src/emqx/endpoints/auth_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/banned_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/client_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/messaging_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/monitoring_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/rules_endpoints.rs
  - schematic/definitions/src/emqx/mod.rs
  - schematic/definitions/src/eversolo/mod.rs
  - schematic/definitions/src/gitea/mod.rs
  - schematic/definitions/src/github/mod.rs
  - schematic/definitions/src/gitlab/endpoints/branches.rs
  - schematic/definitions/src/gitlab/endpoints/issues.rs
  - schematic/definitions/src/gitlab/endpoints/merge_requests.rs
  - schematic/definitions/src/gitlab/endpoints/pipelines.rs
  - schematic/definitions/src/gitlab/endpoints/projects.rs
  - schematic/definitions/src/gitlab/endpoints/releases.rs
  - schematic/definitions/src/huggingface/mod.rs
  - schematic/definitions/src/lmstudio/mod.rs
  - schematic/definitions/src/ollama/mod.rs
  - schematic/definitions/src/openai/mod.rs
  - schematic/definitions/src/registry/mod.rs
  - schematic/definitions/src/samsung_smart_tv/mod.rs
  - schematic/definitions/src/unfolded_circle/core_rest/mod.rs
  - schematic/gen/src/codegen/api_struct/helpers.rs
  - schematic/gen/src/codegen/api_struct/tests.rs
  - schematic/gen/src/codegen/client/tests.rs
  - schematic/gen/src/codegen/endpoint_methods/names.rs
  - schematic/gen/src/codegen/endpoint_methods/tests.rs
  - schematic/gen/src/codegen/pagination/tests.rs
  - schematic/gen/src/codegen/request_enum.rs
  - schematic/gen/src/codegen/request_structs/body.rs
  - schematic/gen/src/codegen/request_structs/mod.rs
  - schematic/gen/src/codegen/request_structs/named_params.rs
  - schematic/gen/src/codegen/request_structs/path_params.rs
  - schematic/gen/src/codegen/request_structs/query_params.rs
  - schematic/gen/src/codegen/request_structs/shared.rs
  - schematic/gen/src/codegen/request_structs/single.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/src/definition_gen.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/collection.rs
  - schematic/gen/src/export/postman/tests.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/tests/l1/e2e_generation.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/openapi_strict_completeness.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/gen/tests/l1/pagination_validation.rs
  - schematic/gen/tests/l1/partial_generation.rs
  - schematic/gen/tests/l1/path_substitution.rs
  - schematic/gen/tests/l1/postman_golden.rs
  - schematic/gen/tests/l1/query_param_detection.rs
  - schematic/gen/tests/l1/query_params_codegen.rs
  - schematic/runtime/src/client.rs
  - schematic/runtime/src/origin.rs
  - schematic/runtime/tests/l1/pager_surface.rs
  - schematic/runtime/tests/l1/redirects.rs
docs_updated_during_phase_4:
  - schematic/define/README.md
  - schematic/docs/definition-builders.md
  - schematic/docs/io/import-openapi.md
  - schematic/docs/pagination.md
  - schematic/docs/runtime/building-clients.md
  - schematic/docs/runtime/origins-and-redirects.md
  - schematic/features/2026-10-14-schematic-definition-api/implementation-log.md
  - schematic/features/2026-10-14-schematic-definition-api/plan.md
  - schematic/features/2026-10-14-schematic-definition-api/spec.md
  - schematic/gen/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/schematic-define/schematic/SKILL.md
source_files_during_phase_5:
  - schematic/define/src/builders.rs
  - schematic/define/src/lib.rs
  - schematic/define/src/oauth.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/import.rs
  - schematic/define/src/openapi/import/builder.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/openapi/import/query_paths.rs
  - schematic/define/src/pagination/resolve.rs
  - schematic/define/src/pagination/validate.rs
  - schematic/define/src/request.rs
  - schematic/define/src/types.rs
  - schematic/define/tests/builders.rs
  - schematic/define/tests/cross_origin_policy.rs
  - schematic/define/tests/endpoint_pagination.rs
  - schematic/define/tests/import_normalization.rs
  - schematic/define/tests/pagination_support/mod.rs
  - schematic/definitions/src/anthropic/mod.rs
  - schematic/definitions/src/artificial_analysis/mod.rs
  - schematic/definitions/src/bitbucket/mod.rs
  - schematic/definitions/src/elevenlabs/endpoints/history.rs
  - schematic/definitions/src/elevenlabs/endpoints/sound_generation.rs
  - schematic/definitions/src/elevenlabs/endpoints/speech.rs
  - schematic/definitions/src/elevenlabs/endpoints/voices.rs
  - schematic/definitions/src/elevenlabs/endpoints/workspace.rs
  - schematic/definitions/src/emqx/endpoints/auth_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/banned_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/client_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/messaging_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/monitoring_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/rules_endpoints.rs
  - schematic/definitions/src/emqx/mod.rs
  - schematic/definitions/src/eversolo/mod.rs
  - schematic/definitions/src/gitea/mod.rs
  - schematic/definitions/src/github/mod.rs
  - schematic/definitions/src/gitlab/endpoints/branches.rs
  - schematic/definitions/src/gitlab/endpoints/issues.rs
  - schematic/definitions/src/gitlab/endpoints/merge_requests.rs
  - schematic/definitions/src/gitlab/endpoints/pipelines.rs
  - schematic/definitions/src/gitlab/endpoints/projects.rs
  - schematic/definitions/src/gitlab/endpoints/releases.rs
  - schematic/definitions/src/huggingface/mod.rs
  - schematic/definitions/src/lmstudio/mod.rs
  - schematic/definitions/src/ollama/mod.rs
  - schematic/definitions/src/openai/mod.rs
  - schematic/definitions/src/openai/types.rs
  - schematic/definitions/src/openai_pagination.rs
  - schematic/definitions/src/registry/mod.rs
  - schematic/definitions/src/samsung_smart_tv/mod.rs
  - schematic/definitions/src/unfolded_circle/core_rest/mod.rs
  - schematic/definitions/tests/pagination_coverage/main.rs
  - schematic/definitions/tests/pagination_coverage/openai.rs
  - schematic/gen/src/codegen/client/tests.rs
  - schematic/gen/src/codegen/endpoint_methods/tests.rs
  - schematic/gen/src/codegen/pagination/tests.rs
  - schematic/gen/src/codegen/request_enum.rs
  - schematic/gen/src/codegen/request_structs/mod.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/src/definition_gen.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/collection.rs
  - schematic/gen/src/export/postman/tests.rs
  - schematic/gen/src/import_pipeline.rs
  - schematic/gen/src/model_gen.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/tests/common/fresh_output.rs
  - schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt
  - schematic/gen/tests/l1/definition_baseline.rs
  - schematic/gen/tests/l1/e2e_generation.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/import_overlay.rs
  - schematic/gen/tests/l1/openapi_import_test.rs
  - schematic/gen/tests/l1/openapi_strict_completeness.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/gen/tests/l1/pagination_validation.rs
  - schematic/gen/tests/l1/partial_generation.rs
  - schematic/gen/tests/l1/path_substitution.rs
  - schematic/gen/tests/l1/postman_golden.rs
  - schematic/gen/tests/l1/query_param_detection.rs
  - schematic/gen/tests/l1/query_params_codegen.rs
  - schematic/openapi/openai.json
  - schematic/postman/openai.postman_collection.json
  - schematic/schema/src/openai/mod.rs
  - schematic/schema/src/openai/requests.rs
docs_updated_during_phase_5:
  - schematic/definitions/README.md
  - schematic/docs/definition-builders.md
  - schematic/docs/io/import-openapi.md
  - schematic/docs/pagination.md
  - schematic/docs/typed-registration.md
  - schematic/features/2026-10-14-schematic-definition-api/implementation-log.md
  - schematic/features/2026-10-14-schematic-definition-api/plan.md
  - schematic/features/2026-10-14-schematic-definition-api/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/schematic-define/SKILL.md
  - .claude/skills/schematic-define/schematic/SKILL.md
  - .claude/skills/schematic-define/schematic/definitions.md
source_files_during_phase_6:
  - schematic/define/src/builders.rs
  - schematic/define/src/lib.rs
  - schematic/define/src/oauth.rs
  - schematic/define/src/openapi/export/info.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/paths.rs
  - schematic/define/src/openapi/extensions.rs
  - schematic/define/src/openapi/import/builder.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/pagination.rs
  - schematic/define/src/pagination/collection.rs
  - schematic/define/src/pagination/resolve.rs
  - schematic/define/src/pagination/validate.rs
  - schematic/define/src/params/mod.rs
  - schematic/define/src/prelude.rs
  - schematic/define/src/request.rs
  - schematic/define/src/schema.rs
  - schematic/define/src/types.rs
  - schematic/define/tests/builders.rs
  - schematic/define/tests/construction_enforcement.rs
  - schematic/define/tests/cross_origin_policy.rs
  - schematic/define/tests/definition_serde.rs
  - schematic/define/tests/endpoint_literal_surface.rs
  - schematic/define/tests/endpoint_pagination.rs
  - schematic/define/tests/pagination_inventory.rs
  - schematic/define/tests/pagination_partitions.rs
  - schematic/define/tests/pagination_serde.rs
  - schematic/define/tests/pagination_support/mod.rs
  - schematic/define/tests/pagination_tables.rs
  - schematic/definitions/src/anthropic/mod.rs
  - schematic/definitions/src/anthropic/types/api.rs
  - schematic/definitions/src/anthropic/types/messages.rs
  - schematic/definitions/src/anthropic/types/tools.rs
  - schematic/definitions/src/artificial_analysis/mod.rs
  - schematic/definitions/src/artificial_analysis/types.rs
  - schematic/definitions/src/bitbucket/mod.rs
  - schematic/definitions/src/bitbucket/types/branches.rs
  - schematic/definitions/src/bitbucket/types/common.rs
  - schematic/definitions/src/bitbucket/types/downloads.rs
  - schematic/definitions/src/bitbucket/types/issues.rs
  - schematic/definitions/src/bitbucket/types/pull_requests.rs
  - schematic/definitions/src/bitbucket/types/repos.rs
  - schematic/definitions/src/bitbucket/types/tags.rs
  - schematic/definitions/src/elevenlabs/endpoints/history.rs
  - schematic/definitions/src/elevenlabs/endpoints/sound_generation.rs
  - schematic/definitions/src/elevenlabs/endpoints/speech.rs
  - schematic/definitions/src/elevenlabs/endpoints/voices.rs
  - schematic/definitions/src/elevenlabs/endpoints/workspace.rs
  - schematic/definitions/src/elevenlabs/mod.rs
  - schematic/definitions/src/elevenlabs/types/common.rs
  - schematic/definitions/src/elevenlabs/types/history.rs
  - schematic/definitions/src/elevenlabs/types/speech.rs
  - schematic/definitions/src/elevenlabs/types/voices.rs
  - schematic/definitions/src/elevenlabs/types/workspace.rs
  - schematic/definitions/src/emqx/endpoints/auth_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/banned_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/client_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/messaging_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/monitoring_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/rules_endpoints.rs
  - schematic/definitions/src/emqx/mod.rs
  - schematic/definitions/src/emqx/types/auth.rs
  - schematic/definitions/src/emqx/types/banned.rs
  - schematic/definitions/src/emqx/types/broker.rs
  - schematic/definitions/src/emqx/types/clients.rs
  - schematic/definitions/src/emqx/types/common.rs
  - schematic/definitions/src/emqx/types/messaging.rs
  - schematic/definitions/src/emqx/types/rules.rs
  - schematic/definitions/src/eversolo/mod.rs
  - schematic/definitions/src/eversolo/types.rs
  - schematic/definitions/src/gitea/mod.rs
  - schematic/definitions/src/gitea/types.rs
  - schematic/definitions/src/github/mod.rs
  - schematic/definitions/src/github/types/issues.rs
  - schematic/definitions/src/github/types/pull_requests.rs
  - schematic/definitions/src/github/types/releases.rs
  - schematic/definitions/src/github/types/repos.rs
  - schematic/definitions/src/github/types/workflow.rs
  - schematic/definitions/src/gitlab/endpoints/branches.rs
  - schematic/definitions/src/gitlab/endpoints/issues.rs
  - schematic/definitions/src/gitlab/endpoints/merge_requests.rs
  - schematic/definitions/src/gitlab/endpoints/pipelines.rs
  - schematic/definitions/src/gitlab/endpoints/projects.rs
  - schematic/definitions/src/gitlab/endpoints/releases.rs
  - schematic/definitions/src/gitlab/mod.rs
  - schematic/definitions/src/gitlab/types/branches.rs
  - schematic/definitions/src/gitlab/types/common.rs
  - schematic/definitions/src/gitlab/types/issues.rs
  - schematic/definitions/src/gitlab/types/merge_requests.rs
  - schematic/definitions/src/gitlab/types/pipelines.rs
  - schematic/definitions/src/gitlab/types/projects.rs
  - schematic/definitions/src/gitlab/types/releases.rs
  - schematic/definitions/src/huggingface/mod.rs
  - schematic/definitions/src/huggingface/types/collections.rs
  - schematic/definitions/src/huggingface/types/common.rs
  - schematic/definitions/src/huggingface/types/datasets.rs
  - schematic/definitions/src/huggingface/types/discussions.rs
  - schematic/definitions/src/huggingface/types/models.rs
  - schematic/definitions/src/huggingface/types/repos.rs
  - schematic/definitions/src/huggingface/types/spaces.rs
  - schematic/definitions/src/huggingface/types/stubs.rs
  - schematic/definitions/src/huggingface/types/users.rs
  - schematic/definitions/src/lib.rs
  - schematic/definitions/src/lmstudio/mod.rs
  - schematic/definitions/src/lmstudio/types.rs
  - schematic/definitions/src/ollama/mod.rs
  - schematic/definitions/src/ollama/types/completion.rs
  - schematic/definitions/src/ollama/types/models.rs
  - schematic/definitions/src/ollama/types/openai.rs
  - schematic/definitions/src/prelude.rs
  - schematic/definitions/src/registry/lookup.rs
  - schematic/definitions/src/registry/mod.rs
  - schematic/definitions/src/registry/typed.rs
  - schematic/definitions/src/samsung_smart_tv/mod.rs
  - schematic/definitions/src/samsung_smart_tv/types.rs
  - schematic/definitions/src/unfolded_circle/core_rest/mod.rs
  - schematic/definitions/src/unfolded_circle/core_rest/types.rs
  - schematic/definitions/tests/authoring_surface.rs
  - schematic/definitions/tests/pagination_coverage/forges.rs
  - schematic/definitions/tests/pagination_coverage/main.rs
  - schematic/gen/src/codegen/client/tests.rs
  - schematic/gen/src/codegen/endpoint_methods/tests.rs
  - schematic/gen/src/codegen/error.rs
  - schematic/gen/src/codegen/pagination/mod.rs
  - schematic/gen/src/codegen/pagination/tests.rs
  - schematic/gen/src/codegen/request_enum.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/src/definition_gen.rs
  - schematic/gen/src/diagnostics.rs
  - schematic/gen/src/export/naming.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/collection.rs
  - schematic/gen/src/export/postman/mod.rs
  - schematic/gen/src/export/postman/tests.rs
  - schematic/gen/src/output/assemble/api.rs
  - schematic/gen/src/pipeline.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt
  - schematic/gen/tests/l1/artifact_drift.rs
  - schematic/gen/tests/l1/definition_baseline.rs
  - schematic/gen/tests/l1/e2e_generation.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/openapi_strict_completeness.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/gen/tests/l1/pagination_validation.rs
  - schematic/gen/tests/l1/partial_generation.rs
  - schematic/gen/tests/l1/path_substitution.rs
  - schematic/gen/tests/l1/postman_golden.rs
  - schematic/gen/tests/l1/query_param_detection.rs
  - schematic/gen/tests/l1/query_params_codegen.rs
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
  - schematic/openapi/samsung_smart_tv.json
  - schematic/openapi/unfolded_circle_core_rest.json
  - schematic/postman/gitea.postman_collection.json
  - schematic/postman/gitlab.postman_collection.json
  - schematic/schema/Cargo.lock
  - schematic/schema/src/anthropic.rs
  - schematic/schema/src/artificial_analysis.rs
  - schematic/schema/src/bitbucket/requests.rs
  - schematic/schema/src/elevenlabs/requests.rs
  - schematic/schema/src/emqx/requests.rs
  - schematic/schema/src/eversolo/requests.rs
  - schematic/schema/src/gitea/mod.rs
  - schematic/schema/src/gitea/requests.rs
  - schematic/schema/src/github/requests.rs
  - schematic/schema/src/gitlab/requests.rs
  - schematic/schema/src/huggingface/requests.rs
  - schematic/schema/src/lmstudio.rs
  - schematic/schema/src/ollama.rs
  - schematic/schema/src/samsung_smart_tv.rs
  - schematic/schema/src/unfolded_circle_core_rest.rs
  - schematic/schema/tests/fixtures/compile_fail/after_on_a_page_number_endpoint.stderr
  - schematic/schema/tests/fixtures/compile_fail/pager_on_an_unpaginated_endpoint.stderr
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/gitlab.rs
docs_updated_during_phase_6:
  - schematic/README.md
  - schematic/define/README.md
  - schematic/definitions/README.md
  - schematic/docs/definition-builders.md
  - schematic/docs/definition-snapshots.md
  - schematic/docs/generation/request-side.md
  - schematic/docs/generation/validation.md
  - schematic/docs/io/import-openapi.md
  - schematic/docs/io/openapi-extensions.md
  - schematic/docs/pagination.md
  - schematic/docs/typed-registration.md
  - schematic/features/2026-10-14-schematic-definition-api/implementation-log.md
  - schematic/features/2026-10-14-schematic-definition-api/plan.md
  - schematic/features/2026-10-14-schematic-definition-api/spec.md
docs_created_during_phase_6:
  - schematic/docs/generation/workflow.md
skills_files_updated_during_phase_6:
  - .claude/skills/schematic-define/SKILL.md
  - .claude/skills/schematic-define/schematic/SKILL.md
  - .claude/skills/schematic-define/schematic/define.md
  - .claude/skills/schematic-define/schematic/gen.md
source_files_during_phase_7:
  - schematic/gen/build.rs
  - schematic/gen/tests/l1/artifact_drift.rs
docs_updated_during_phase_7:
  - schematic/docs/generation/workflow.md
  - schematic/features/2026-10-14-schematic-definition-api/plan.md
  - schematic/features/2026-10-14-schematic-definition-api/implementation-log.md
  - schematic/features/2026-10-14-schematic-definition-api/spec.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/os/windows.md
  - .claude/skills/schematic-define/schematic/SKILL.md
source_files_during_phase_8:
  - schematic/define/src/lib.rs
  - schematic/define/src/request.rs
  - schematic/define/src/response.rs
  - schematic/define/src/schema.rs
  - schematic/define/tests/authoring_guidance.rs
  - schematic/definitions/src/registry/mod.rs
  - schematic/definitions/src/registry/typed.rs
docs_updated_during_phase_8:
  - .claude/commands/create-schematic-api.md
  - schematic/README.md
  - schematic/define/README.md
  - schematic/definitions/README.md
  - schematic/gen/README.md
  - schematic/docs/definition-builders.md
  - schematic/docs/typed-registration.md
  - schematic/docs/io/import-openapi.md
  - schematic/docs/io/export-openapi.md
  - schematic/docs/io/export-postman.md
  - schematic/features/2026-10-14-schematic-definition-api/plan.md
  - schematic/features/2026-10-14-schematic-definition-api/implementation-log.md
  - schematic/features/2026-10-14-schematic-definition-api/spec.md
  - schematic/docs/research/samsung-design.md
  - schematic/docs/research/multipart-form-data.md
  - schematic/docs/research/oauth/integration-with-schematic.md
docs_created_during_phase_8:
  - schematic/docs/authoring-examples.md
  - schematic/docs/generation/generated-clients.md
skills_files_updated_during_phase_8:
  - .claude/skills/schematic-define/SKILL.md
  - .claude/skills/schematic-define/schematic/SKILL.md
source_code:
  - Cargo.lock
  - schematic/define/src/cross_origin.rs
  - schematic/define/src/pagination.rs
  - schematic/define/src/pagination/resolve.rs
  - schematic/define/src/params/mod.rs
  - schematic/define/src/params/param_def.rs
  - schematic/define/src/types.rs
  - schematic/define/tests/definition_serde.rs
  - schematic/gen/Cargo.toml
  - schematic/gen/examples/capture_definition_baseline.rs
  - schematic/gen/examples/support/definition_snapshot.rs
  - schematic/gen/tests/l1/artifact_drift.rs
  - schematic/gen/tests/l1/definition_baseline.rs
  - schematic/gen/tests/l1/main.rs
  - schematic/justfile
  - schematic/features/2026-10-14-schematic-definition-api/spike/Cargo.lock
  - schematic/features/2026-10-14-schematic-definition-api/spike/Cargo.toml
  - schematic/features/2026-10-14-schematic-definition-api/spike/examples/mismatched_type.rs
  - schematic/features/2026-10-14-schematic-definition-api/spike/examples/misspelled_path.rs
  - schematic/features/2026-10-14-schematic-definition-api/spike/examples/private_descriptor.rs
  - schematic/features/2026-10-14-schematic-definition-api/spike/prove.py
  - schematic/features/2026-10-14-schematic-definition-api/spike/src/bin/emitted_path_source.rs
  - schematic/features/2026-10-14-schematic-definition-api/spike/src/lib.rs
  - schematic/gen/tests/fixtures/definition-baseline/anthropic.json
  - schematic/gen/tests/fixtures/definition-baseline/artificial-analysis-critpt.json
  - schematic/gen/tests/fixtures/definition-baseline/artificial-analysis-data.json
  - schematic/gen/tests/fixtures/definition-baseline/bitbucket.json
  - schematic/gen/tests/fixtures/definition-baseline/elevenlabs.json
  - schematic/gen/tests/fixtures/definition-baseline/emqx-basic.json
  - schematic/gen/tests/fixtures/definition-baseline/emqx-bearer.json
  - schematic/gen/tests/fixtures/definition-baseline/eversolo.json
  - schematic/gen/tests/fixtures/definition-baseline/gitea.json
  - schematic/gen/tests/fixtures/definition-baseline/github.json
  - schematic/gen/tests/fixtures/definition-baseline/gitlab.json
  - schematic/gen/tests/fixtures/definition-baseline/huggingface.json
  - schematic/gen/tests/fixtures/definition-baseline/lmstudio.json
  - schematic/gen/tests/fixtures/definition-baseline/ollama-native.json
  - schematic/gen/tests/fixtures/definition-baseline/ollama-openai.json
  - schematic/gen/tests/fixtures/definition-baseline/openai.json
  - schematic/gen/tests/fixtures/definition-baseline/samsung-smart-tv.json
  - schematic/gen/tests/fixtures/definition-baseline/unfolded-circle-core-rest.json
  - schematic/define/Cargo.toml
  - schematic/define/src/lib.rs
  - schematic/define/src/models/type_ref.rs
  - schematic/define/src/registration.rs
  - schematic/define/tests/typed_registration.rs
  - schematic/definitions/src/registry/mod.rs
  - schematic/definitions/src/registry/typed.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/define/src/builders.rs
  - schematic/define/src/prelude.rs
  - schematic/define/src/schema.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/paths.rs
  - schematic/define/src/openapi/export/request_body.rs
  - schematic/define/src/openapi/export/responses.rs
  - schematic/define/tests/builders.rs
  - schematic/define/tests/endpoint_literal_surface.rs
  - schematic/gen/src/output/mod.rs
  - schematic/gen/src/codegen/api_struct/tests.rs
  - schematic/gen/src/codegen/module_docs.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/src/definition_gen.rs
  - schematic/gen/tests/l1/http_client.rs
  - schematic/define/src/oauth.rs
  - schematic/define/src/openapi/export/parameters.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/openapi/import/mappings/parameters.rs
  - schematic/define/src/pagination/validate.rs
  - schematic/define/src/request.rs
  - schematic/define/tests/cross_origin_policy.rs
  - schematic/define/tests/definition_forms.rs
  - schematic/define/tests/pagination_support/mod.rs
  - schematic/definitions/src/anthropic/mod.rs
  - schematic/definitions/src/artificial_analysis/mod.rs
  - schematic/definitions/src/bitbucket/mod.rs
  - schematic/definitions/src/elevenlabs/endpoints/history.rs
  - schematic/definitions/src/elevenlabs/endpoints/sound_generation.rs
  - schematic/definitions/src/elevenlabs/endpoints/speech.rs
  - schematic/definitions/src/elevenlabs/endpoints/voices.rs
  - schematic/definitions/src/elevenlabs/endpoints/workspace.rs
  - schematic/definitions/src/emqx/endpoints/auth_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/banned_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/client_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/messaging_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/monitoring_endpoints.rs
  - schematic/definitions/src/emqx/endpoints/rules_endpoints.rs
  - schematic/definitions/src/emqx/mod.rs
  - schematic/definitions/src/eversolo/mod.rs
  - schematic/definitions/src/gitea/mod.rs
  - schematic/definitions/src/github/mod.rs
  - schematic/definitions/src/gitlab/endpoints/branches.rs
  - schematic/definitions/src/gitlab/endpoints/issues.rs
  - schematic/definitions/src/gitlab/endpoints/merge_requests.rs
  - schematic/definitions/src/gitlab/endpoints/pipelines.rs
  - schematic/definitions/src/gitlab/endpoints/projects.rs
  - schematic/definitions/src/gitlab/endpoints/releases.rs
  - schematic/definitions/src/huggingface/mod.rs
  - schematic/definitions/src/lmstudio/mod.rs
  - schematic/definitions/src/ollama/mod.rs
  - schematic/definitions/src/openai/mod.rs
  - schematic/definitions/src/samsung_smart_tv/mod.rs
  - schematic/definitions/src/unfolded_circle/core_rest/mod.rs
  - schematic/gen/src/codegen/api_struct/helpers.rs
  - schematic/gen/src/codegen/client/tests.rs
  - schematic/gen/src/codegen/endpoint_methods/names.rs
  - schematic/gen/src/codegen/endpoint_methods/tests.rs
  - schematic/gen/src/codegen/pagination/tests.rs
  - schematic/gen/src/codegen/request_enum.rs
  - schematic/gen/src/codegen/request_structs/body.rs
  - schematic/gen/src/codegen/request_structs/mod.rs
  - schematic/gen/src/codegen/request_structs/named_params.rs
  - schematic/gen/src/codegen/request_structs/path_params.rs
  - schematic/gen/src/codegen/request_structs/query_params.rs
  - schematic/gen/src/codegen/request_structs/shared.rs
  - schematic/gen/src/codegen/request_structs/single.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/collection.rs
  - schematic/gen/src/export/postman/tests.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/tests/l1/e2e_generation.rs
  - schematic/gen/tests/l1/openapi_strict_completeness.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/gen/tests/l1/pagination_validation.rs
  - schematic/gen/tests/l1/partial_generation.rs
  - schematic/gen/tests/l1/path_substitution.rs
  - schematic/gen/tests/l1/postman_golden.rs
  - schematic/gen/tests/l1/query_param_detection.rs
  - schematic/gen/tests/l1/query_params_codegen.rs
  - schematic/runtime/src/client.rs
  - schematic/runtime/src/origin.rs
  - schematic/runtime/tests/l1/pager_surface.rs
  - schematic/runtime/tests/l1/redirects.rs
  - schematic/define/src/openapi/import.rs
  - schematic/define/src/openapi/import/builder.rs
  - schematic/define/src/openapi/import/query_paths.rs
  - schematic/define/tests/endpoint_pagination.rs
  - schematic/define/tests/import_normalization.rs
  - schematic/definitions/src/openai/types.rs
  - schematic/definitions/src/openai_pagination.rs
  - schematic/definitions/tests/pagination_coverage/main.rs
  - schematic/definitions/tests/pagination_coverage/openai.rs
  - schematic/gen/src/import_pipeline.rs
  - schematic/gen/src/model_gen.rs
  - schematic/gen/tests/common/fresh_output.rs
  - schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt
  - schematic/gen/tests/l1/import_overlay.rs
  - schematic/gen/tests/l1/openapi_import_test.rs
  - schematic/openapi/openai.json
  - schematic/postman/openai.postman_collection.json
  - schematic/schema/src/openai/mod.rs
  - schematic/schema/src/openai/requests.rs
  - schematic/define/src/openapi/export/info.rs
  - schematic/define/src/openapi/extensions.rs
  - schematic/define/src/pagination/collection.rs
  - schematic/define/tests/construction_enforcement.rs
  - schematic/define/tests/pagination_inventory.rs
  - schematic/define/tests/pagination_partitions.rs
  - schematic/define/tests/pagination_serde.rs
  - schematic/define/tests/pagination_tables.rs
  - schematic/definitions/src/anthropic/types/api.rs
  - schematic/definitions/src/anthropic/types/messages.rs
  - schematic/definitions/src/anthropic/types/tools.rs
  - schematic/definitions/src/artificial_analysis/types.rs
  - schematic/definitions/src/bitbucket/types/branches.rs
  - schematic/definitions/src/bitbucket/types/common.rs
  - schematic/definitions/src/bitbucket/types/downloads.rs
  - schematic/definitions/src/bitbucket/types/issues.rs
  - schematic/definitions/src/bitbucket/types/pull_requests.rs
  - schematic/definitions/src/bitbucket/types/repos.rs
  - schematic/definitions/src/bitbucket/types/tags.rs
  - schematic/definitions/src/elevenlabs/mod.rs
  - schematic/definitions/src/elevenlabs/types/common.rs
  - schematic/definitions/src/elevenlabs/types/history.rs
  - schematic/definitions/src/elevenlabs/types/speech.rs
  - schematic/definitions/src/elevenlabs/types/voices.rs
  - schematic/definitions/src/elevenlabs/types/workspace.rs
  - schematic/definitions/src/emqx/types/auth.rs
  - schematic/definitions/src/emqx/types/banned.rs
  - schematic/definitions/src/emqx/types/broker.rs
  - schematic/definitions/src/emqx/types/clients.rs
  - schematic/definitions/src/emqx/types/common.rs
  - schematic/definitions/src/emqx/types/messaging.rs
  - schematic/definitions/src/emqx/types/rules.rs
  - schematic/definitions/src/eversolo/types.rs
  - schematic/definitions/src/gitea/types.rs
  - schematic/definitions/src/github/types/issues.rs
  - schematic/definitions/src/github/types/pull_requests.rs
  - schematic/definitions/src/github/types/releases.rs
  - schematic/definitions/src/github/types/repos.rs
  - schematic/definitions/src/github/types/workflow.rs
  - schematic/definitions/src/gitlab/mod.rs
  - schematic/definitions/src/gitlab/types/branches.rs
  - schematic/definitions/src/gitlab/types/common.rs
  - schematic/definitions/src/gitlab/types/issues.rs
  - schematic/definitions/src/gitlab/types/merge_requests.rs
  - schematic/definitions/src/gitlab/types/pipelines.rs
  - schematic/definitions/src/gitlab/types/projects.rs
  - schematic/definitions/src/gitlab/types/releases.rs
  - schematic/definitions/src/huggingface/types/collections.rs
  - schematic/definitions/src/huggingface/types/common.rs
  - schematic/definitions/src/huggingface/types/datasets.rs
  - schematic/definitions/src/huggingface/types/discussions.rs
  - schematic/definitions/src/huggingface/types/models.rs
  - schematic/definitions/src/huggingface/types/repos.rs
  - schematic/definitions/src/huggingface/types/spaces.rs
  - schematic/definitions/src/huggingface/types/stubs.rs
  - schematic/definitions/src/huggingface/types/users.rs
  - schematic/definitions/src/lib.rs
  - schematic/definitions/src/lmstudio/types.rs
  - schematic/definitions/src/ollama/types/completion.rs
  - schematic/definitions/src/ollama/types/models.rs
  - schematic/definitions/src/ollama/types/openai.rs
  - schematic/definitions/src/prelude.rs
  - schematic/definitions/src/registry/lookup.rs
  - schematic/definitions/src/samsung_smart_tv/types.rs
  - schematic/definitions/src/unfolded_circle/core_rest/types.rs
  - schematic/definitions/tests/authoring_surface.rs
  - schematic/definitions/tests/pagination_coverage/forges.rs
  - schematic/gen/src/codegen/error.rs
  - schematic/gen/src/codegen/pagination/mod.rs
  - schematic/gen/src/diagnostics.rs
  - schematic/gen/src/export/naming.rs
  - schematic/gen/src/export/postman/mod.rs
  - schematic/gen/src/output/assemble/api.rs
  - schematic/gen/src/pipeline.rs
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
  - schematic/openapi/samsung_smart_tv.json
  - schematic/openapi/unfolded_circle_core_rest.json
  - schematic/postman/gitea.postman_collection.json
  - schematic/postman/gitlab.postman_collection.json
  - schematic/schema/Cargo.lock
  - schematic/schema/src/anthropic.rs
  - schematic/schema/src/artificial_analysis.rs
  - schematic/schema/src/bitbucket/requests.rs
  - schematic/schema/src/elevenlabs/requests.rs
  - schematic/schema/src/emqx/requests.rs
  - schematic/schema/src/eversolo/requests.rs
  - schematic/schema/src/gitea/mod.rs
  - schematic/schema/src/gitea/requests.rs
  - schematic/schema/src/github/requests.rs
  - schematic/schema/src/gitlab/requests.rs
  - schematic/schema/src/huggingface/requests.rs
  - schematic/schema/src/lmstudio.rs
  - schematic/schema/src/ollama.rs
  - schematic/schema/src/samsung_smart_tv.rs
  - schematic/schema/src/unfolded_circle_core_rest.rs
  - schematic/schema/tests/fixtures/compile_fail/after_on_a_page_number_endpoint.stderr
  - schematic/schema/tests/fixtures/compile_fail/pager_on_an_unpaginated_endpoint.stderr
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/gitlab.rs
  - schematic/gen/build.rs
  - schematic/define/src/response.rs
  - schematic/define/tests/authoring_guidance.rs
documentation:
  - docs/dependencies.md
  - schematic/define/README.md
  - schematic/docs/dependencies.md
  - schematic/features/2026-10-14-schematic-definition-api/plan.md
  - schematic/features/2026-10-14-schematic-definition-api/spec.md
  - schematic/docs/definition-snapshots.md
  - schematic/features/2026-10-14-schematic-definition-api/implementation-log.md
  - schematic/features/2026-10-14-schematic-definition-api/inventory.md
  - schematic/features/2026-10-14-schematic-definition-api/prerequisite-interfaces.md
  - schematic/features/2026-10-14-schematic-definition-api/reproduction.md
  - schematic/features/2026-10-14-schematic-definition-api/spike/README.md
  - schematic/docs/typed-registration.md
  - schematic/docs/definition-builders.md
  - schematic/docs/io/import-openapi.md
  - schematic/docs/pagination.md
  - schematic/docs/runtime/building-clients.md
  - schematic/docs/runtime/origins-and-redirects.md
  - schematic/gen/README.md
  - schematic/definitions/README.md
  - schematic/README.md
  - schematic/docs/generation/request-side.md
  - schematic/docs/generation/validation.md
  - schematic/docs/io/openapi-extensions.md
  - schematic/docs/generation/workflow.md
  - .claude/commands/create-schematic-api.md
  - schematic/docs/io/export-openapi.md
  - schematic/docs/io/export-postman.md
  - schematic/docs/research/samsung-design.md
  - schematic/docs/research/multipart-form-data.md
  - schematic/docs/research/oauth/integration-with-schematic.md
  - schematic/docs/authoring-examples.md
  - schematic/docs/generation/generated-clients.md
completed_phase: 8
implemented: false
---

# Schematic definition API implementation plan

## Work and Successful Completion

Replace REST definition literals and hand-typed schema references with required-fact builders and a typed, module-scoped definition context. Capture the existing definitions and generated artifacts before changing them; introduce the context and builders additively; update the import emitter; convert OpenAI once and then the remaining APIs in verified batches; remove obsolete interfaces only after their consumers have migrated. Preserve WebSocket definitions and generated request defaults. Finish by regenerating artifacts, verifying dependents, and updating the authoring documentation.

Success means all R1–R31 have observable evidence: all shipped REST endpoints use builders, registries come from the same context assembly as definitions, generated Rust paths compile, invalid references and contradictory declarations fail with context, containers preserve their wire shape without becoming components, pagination resolutions and caller controls survive, all nine query-bearing paths are migrated (eight by merging into their same-route twins), and a request body on `GET` is rejected. Historical differences are individually explained; final committed Rust, OpenAPI, and Postman artifacts have no drift exemptions. The implementation log records decisions and evidence as they occur. The terminal state is **implementation complete, ready for review**, with platform evidence and any remaining limitations stated accurately.

Prerequisite facts were refreshed on 2026-10-14 after pagination passed review 2. The runtime and pagination interfaces now exist; the specification's **Delivered pagination interfaces and migration baseline** section records their actual names, coverage, and frozen tests. Phase 1 rechecks that baseline against the execution tree rather than inventing substitute predecessor features. This plan does not implement the predecessor features or reopen their settled rulings.

The author ruling of 2026-10-12 (`2026-10-11-schematic-generation-integrity` review 1, item 1) is incorporated: endpoint identity stays `(method, path)`, REST selectors are withdrawn, the `?`-suffixed Gitea and OpenAI endpoints merge into their twins with optional `recursive`/`beta` query parameters (no fixed-value query inputs are built), and every parameter declares its location with a `GET` body rejected (R29, R31).

Waves are numbered globally. Each wave is an execution step: finish its tasks and checkpoint before starting the next. “Parallel” means the listed tasks may be assigned to separate subagents with disjoint file ownership; the coordinator integrates shared changes and runs the checkpoint. All agents operate non-interactively, without commits, `cargo fmt`, or focus-stealing terminal/browser windows. Single-task waves and per-API migration batches are sequential.

Every phase gate runs `just test` and `just lint` from `schematic/`, with the definition/artifact comparisons available at that point. Use nextest recipes for L1, `just doctest` explicitly for documentation tests, and `just test-l2` only for real-terminal coverage. The current `test` recipe does not run doctests; it does run the fresh-output and runtime compile-fail guards separately. Load the testing skills and Test Toolkit before authoring tests; use runtime fixture paths and CI-indexable repository reads. `just check-drift` already uses nextest and may be used. Do not use `test-e2e` unchanged: it still invokes `cargo test`; migrate it if needed.

### Delivered pagination contracts to preserve

- Use `schematic_define::pagination::{resolve, caller_knobs, param_adoptions, adopted_params, validate_pagination}` and the actual `Resolution`/`PaginationDiagnostic` types. The API fields being migrated are `default_pagination: Option<Strategy>` and `pagination_exceptions: Vec<PaginationException>`. Strategy helpers already live in their provider modules; preserve them rather than creating a preset layer.
- Preserve `schematic-runtime` ABI 4 and the primary generated endpoint-call surface. Adopted request fields are already removed; preserve authored `.page(..)`, `.page_size(..)`, cursor controls, and `fetch_page() -> Page<Item>` rather than restoring old request setters. Runtime capability names include `AdjustablePageSize`, `Sessioned`, `Sessionless`, and `Renewable`.
- Carry forward pagination review 1's three author rulings: reject partitioned first-page sessions while retaining separately opened shared sessions; paging runs the global JSON callback only; retained errors allow optional explicit retry after every error category without redelivering earlier items. Preserve the `Failure: Sync` bound without adding `Clone`, buffered delivery counts, and adjustable-size retry checkpoints. Automatic retry policies remain outside this migration.
- Capture the current 18-API baseline: 547 resolved endpoints, 511 production endpoint literals, 265 OpenAI endpoints, 35 inert legacy declarations, 94 authored pagers, and 32 verification-task endpoints. Bitbucket `GetSource` is binary and resolves `NotAList` despite a legacy declaration; metadata removal must not create a pager. ElevenLabs `ListVoices` and EMQX `ListAuthUsers` remain the existing definition gaps.
- Preserve `ImportOptions::overlay`, the generic emitted overlay call, and `definitions/src/openai_pagination.rs`. `gen/tests/l1/import_overlay.rs` currently proves repeated OpenAI imports reproduce both committed source files. Use that passing result as the reproduction baseline before changing the emitter.
- Use the specification's concrete frozen-test map for retirement. Retain coverage in `definitions/tests/pagination_coverage/`, the partition validation siblings, generated callback/retry/session tests in `gen/tests/l1/fresh_output_compile.rs`, and the generated child-test registration guard. Run `just test-schema` for the excluded crate's public paging and compile-fail tests.

## Phase 1 — Establish Contracts and Baselines

### Necessary Rules

These rulings adopt the specification's defaults where available. They are planning decisions to record in `implementation-log.md`, not claims that the design has already been implemented.

| Item | Adopted rule or prerequisite |
| --- | --- |
| Prerequisite refresh | Read predecessor review outcomes, including pagination review 2 and review 1's Human rulings. Start from the specification's delivered-interface and frozen-test maps; recheck actual validator, diagnostic, compile-check, runtime, pagination, adoption, and overlay interfaces before baseline capture. Do not fabricate substitute predecessor features. |
| PD1 | Containers recurse to element/value registrations. Maps accept String and Serde-supported integer keys; reject unsupported keys with API, endpoint, and type context. String-like newtypes require an explicit supported-key contract. Option affects wire nullability and the Rust wrapper, while remaining transparent only for component registration. |
| PD2 | Aggregate structured `DefinitionError::Collision { schema_name, first, second }` errors for each module group/API; fully qualified Rust type names are diagnostic text only. Render one line per collision using the specification's message unless the predecessor established a diagnostic format. |
| PD3–PD5 | Use the predecessor's `request_id_header`, `secret_names`, `CrossOrigin`, and exact-origin validation. Lowercase and validate one API request-id header. There is no delivered `ParamDef::secret` field; apply the specification's conditional default for secret forms. Endpoint cross-origin replaces the API policy wholesale; runtime callers may only tighten it. |
| PD6 | Keep `EndpointParams::Default`, preferring `EndpointParams::new()` in examples. Audit all other public definition defaults and record an exact allow-list with reasons. Endpoint, RestApi, and their builders have no public Default; generated request types retain theirs. |
| PD7–PD8 | Endpoint constructor requires id, method, and path; typestate requires description and one success shape. API constructor requires context, name, and base URL; typestate requires description and an explicit authentication choice. Adopt Design-table names, cover every authored HTTP verb and request form, and yield pagination names to the predecessor. Reserve `.path_param`, `.public`, `.auth`, `.deprecated`, `.rate_limit`, and the later response vocabulary names. |
| R12 proposal | Adopt `#[non_exhaustive]` on Endpoint and RestApi after every external consumer and emitter has migrated. Do not introduce it in the additive builder phase. Any use of the spec's source-scan-only fallback must be justified by concrete churn evidence and flagged for review; it weakens the construction guarantee. |
| PD17 | Keep auth_policy and its multi-method semantics; remove oauth_scopes only after repeating the shipped-use and reader scan. An unexpected live use requires a logged resolution, not silent data loss. |
| PD19 | Convert OpenAI through the emitter once in Phase 5. Convert the remaining APIs smallest first in Phase 6, one API per batch. Shared-module pairs integrate serially through their common context. Run R10/R24 after every batch; fix failures before continuing. |
| PD20 | Capture canonical JSON under `gen/tests/fixtures/definition-baseline/<api>.json`, sorting object keys and preserving endpoint, array, and registry entry order. Measure actual aggregate size. Above 5 MB, use biscuit-hash for API metadata, each endpoint, and each ordered registry entry including schema content; record canonicalization and algorithm. No performance spike is needed for this specified size decision. |
| PD21 | After migration, remove Schema.module_path from the REST path if no semantic reader remains; otherwise wire it from the context and test that reader. Emitter copying is not a reader. Preserve shared WebSocket behavior. |
| Fallible assembly | Adopt `define_x_api(&mut DefinitionContext) -> Result<RestApi, DefinitionError>`. One module-group assembly returns definitions and their registry together; lookups propagate errors distinctly from unknown names, rather than returning empty data or unwrapping. Registry adapters stay in schematic-definitions; schematic-define remains provider-neutral. |
| Naming and identity | Never derive a generated Rust path by parsing `std::any::type_name`. Ordinary names require a proven public identifier; exceptions use compile-checked `rust_path!` and optional schema_name. Prefer a typed macro token proving the authored path names T; support generic type paths only where the spike proves their stable implementation. register_as targets non-container leaves. |
| Wrapper storage | Reserve `structure: Option<TypeRef>` with no public setter; all authored registrations leave it None. Store the wrapper/reference information needed by validation and lowering separately, so R3/R7 work without reading the reserved carrier. Do not use the carrier as a second schema source. |
| Ownership limits | No deriving proc-macro, callback descriptors, global registry, shared vendor presets, convenience methods, GraphQL, response-v2 behavior, REST selectors, fixed-value query inputs, or vocabulary additions. Endpoint merges for the `?`-suffixed pairs and narrow wire/Rust-name separation are authorized here; general query fidelity and self-contained schema bundles remain later work. No CI cells or scope changes are added. |

### Wave 1 — Refresh and Inventory (Parallel)

- [x] **Refresh Interfaces**
  - Record predecessor review outcomes, including pagination review 2 and its three author rulings, and the actual API names in the implementation log, including EndpointSpec, CrossOrigin, pagination resolution/adoption, the generic overlay hook, and isolated compile checking.
  - Map the six R27 frozen tests to their actual names and locations; preserve the no-GraphQL half of the runtime surface test.
  - Own the contract/log document; no source changes. Exit requires a completed prerequisite interface map or an explicit dependency hold.
- [x] **Inventory Consumers**
  - Use sniff for repository/dependency discovery and text searches for concrete call sites. Record all REST API names, endpoint counts and ordered ids, registry entries/order, container components, defaulted fields, Basic/OAuth2 users, and five WebSocket definitions.
  - Recount the current baseline of 547 resolved endpoints / 511 production endpoint literals, 265 OpenAI endpoints, 36 container registrations, 35 legacy pagination declarations, 94 authored pagers, 32 verification-task endpoints, and nine query-bearing paths; treat observed inventory as authoritative.
  - Inventory GitLab repository-file request users before its field location changes, users of every request type and client method R29 removes (Gitea GetGitTreeRecursive, the seven OpenAI Beta* endpoints, including schematic-gen L1 tests naming them), GET endpoints declaring a request body, all Default implementations, oauth_scopes/auth_policy/module_path readers, external literals, registry lookups, and generated-schema dependents. Own an inventory artifact, not the shared log file.

Checkpoint: predecessor contracts and inventory agree; actual counts and consumers replace estimates throughout the execution checklist. Missing prerequisites stop baseline capture and dependent implementation, while preserving the inventory for later use.

### Wave 2 — Resolve Design Risks (Parallel, Once Only)

- [x] **Registration Spike**
  - Build one small throwaway stable-Rust prototype covering a nominal type, serde-renamed nominal type, alias/re-export, foreign type, primitives, Value, Vec, Option, HashMap, and nested containers. Prove private descriptors, ordered idempotent registration, collision aggregation, destination-context validation, and fully qualified emitted paths.
  - Resolve dispatch without overlapping blanket `T: JsonSchema` and container implementations. Candidate: a local registration trait with nominal implementations emitted by a declarative helper/import emitter, and disjoint implementations for containers/primitives/Value. JsonSchema supplies schema names; the nominal implementation supplies the Rust identifier. Do not introduce a second schema registration list merely to implement this trait. If a simpler schemars-supported dispatcher meets every contract, prefer it and document how it distinguishes nominal wrappers from containers.
  - Prove rust_path spelling/resolution and T identity, including a negative mismatched-T case; select the exact registration bounds and descriptor wrapper metadata. Reject designs relying on type_name parsing, specialization, or a new deriving proc-macro. Record chosen public signatures before Phase 2.
- [x] **Reproduction Proof**
  - On one host, run the unchanged OpenAI import with identical recipe options and pinned inputs into a temporary destination; compare mod.rs and types.rs byte for byte. Regenerate Rust/OpenAPI/Postman into temporary trees and compare their complete file sets with committed artifacts.
  - Classify differences as tool/version/options, nondeterminism, stale artifacts, or legitimate authored changes. Define a repeatable generic transform for legitimate import differences and prove reproduction before migration; never overwrite evidence to make the test green.
  - Preserve the hand-written overlay. This one-time reproduction investigation is not repeated later as a spike; later re-import and drift checks are regression validation.

Checkpoint: stable registration design compiles, required negative cases fail as expected, and import/artifact reproduction is proven or its concrete blocker is logged. No timing benchmark, OS measurement sweep, or broader importer rewrite is scheduled.

### Wave 3 — Capture Safety Nets (Sequential)

- [x] **Capture Baselines**
  - Add plain serde support first to RestApi, Endpoint, EndpointParams, ParamDef, QueryParamType, ParamStyle, and necessary inner runtime/pagination types (R17). Round-trip each type without changing authored definition behavior.
  - Capture unchanged definition JSON plus ordered registry schemas, and resolved pagination for every list endpoint and the 35 declarations. Include GET Vec recognition, POST arrays requiring explicit list authorship, aliases, and adopted caller controls.
  - Add a reusable non-mutating `just migration-check` recipe/helper: regenerate all three outputs into temporary directories, compare file sets/bytes, and compare historical definitions as JSON values before any changed-model deserialization. Accept only named API/endpoint/field or registry-entry differences with causes; never broad response/schema ignores.
  - Extend the normal L1 artifact drift suite to Rust clients, with final committed artifacts as its reference and no permanent allow-list. Add a deliberately changed-description control proving snapshot comparison detects behavioral drift. Measure fixture size and apply PD20 only if needed.
  - Initialize implementation-log frontmatter following the specified worktree example, listing spec/plan identifiers, implementer, touched packages, and per-phase source/doc/skill files. Log all rules, spikes, counts, and reproduction evidence as they occur.

Phase gate: serde round-trips, snapshot mutation control, original pagination resolutions, and reproduced artifact comparisons pass; `just test`, `just lint`, and `just doctest` pass. No shipped definition has been rewritten.

## Phase 2 — Implement Typed Registration

### Wave 4 — Context and Descriptor Core (Sequential)

- [x] **Build Context**
  - Implement the spike-selected dispatcher, DefinitionContext, private TypeDesc, fallible registration, rust_path!, and DefinitionError (R1–R5, R7). One module group owns insertion order and collisions; repeated canonical registration does not reorder entries.
  - Use existing OpenAPI-compatible schema settings. Preserve explicit registrations of types not mentioned by endpoints and existing nested-def extraction; omit only container components and primitives/free-form JSON components.
  - Implement register_as and schema_name without merging distinct conflicting types. Test HuggingFace Commit/CommitInfo and Ollama progress alias behavior in small fixtures without converting those APIs yet.
  - Derive descriptor serde, reserve optional structural data, and validate every reference against the destination registry, Rust path, and wrapper shape. Matching imported/deserialized data may pass; foreign, forged, and unsupported structural values must not bypass validation.

### Wave 5 — Adapter and Boundary Coverage (Parallel)

- [x] **Adapt Registry**
  - In schematic-definitions, adapt ordered neutral registrations to SchemaRegistry without a reverse crate dependency. Add a module-group assembly seam alongside current literal-based lookup.
  - Test shared Ollama/EMQX groups versus isolated groups, identical registration order, nested schema extraction, and explicit unreachable-from-endpoint registrations. Avoid introducing a second manually maintained registry or API inventory.
- [x] **Test Registration**
  - In schematic-define, implement compile-fail coverage for string construction, the structural setter, misspelled rust_path, and mismatched typed paths according to the spike ruling.
  - Cover all container/primitive/Value combinations, nullable wire shape, map key rejection, aliases, renamed schema names, collision aggregation, descriptor serde, and matching/forged destination-context data. Execute the input matrix below through the public validation result.

Phase gate: R1–R7 have focused passing coverage; old definitions still generate the original baseline; new typed fixture paths pass the predecessor's isolated generated-crate compile check. Integration does not depend on public string descriptor constructors.

## Phase 3 — Add Required-Fact Builders

### Wave 6 — Builder Implementation (Sequential)

- [x] **Build Endpoints**
  - Implement constructors for every authored verb and typestate for description/success. Fallible build handles invalid facts and repeated success calls with API/endpoint diagnostics (R8–R10, R13).
  - Add non-exhaustive Success/ErrorSet placeholders and exactly the R13 forms: four success constructors, matching shorthands, ErrorSet::none, and API default_errors. Lower to existing ApiResponse in one place; no definition names that storage type.
  - Cover JSON, multipart, URL-encoded, text, and binary request authoring. Preserve present text/binary runtime gaps and output-specific diagnostics; do not claim new builder syntax fixes body transport.
- [x] **Build APIs**
  - Add API typestate and coherent auth calls, with no Default on values/builders. Auth choice cannot silently overwrite a prior choice; conflicting explicit env mappings fail in either setter order. Keep explicit AuthPolicy authoritative for supported multi-method cases.
  - Support existing headers, version, params, module_path, request_suffix, and inheritance precedence. Build endpoints before borrowing the context for the API builder; validate references and aggregate collisions at API build.
  - Audit the exact Default allow-list; add EndpointParams::new. Keep external literals temporarily legal until the emitter and providers migrate.

### Wave 7 — Fixture Migration and Compile Contracts (Parallel)

- [x] **Migrate Fixtures**
  - Convert external literal-based generator tests/helpers to builders in output, api_struct, module_docs, validation, definition_gen, and HTTP-client tests. Split file ownership before delegation; provider definitions remain unchanged.
  - Update the relevant R27 literal-surface test to required-fact coverage; keep pagination-table/legacy-presence tests until actual removal. Migrate crate-internal exporter examples only where needed for the public authoring contract.
- [x] **Prove Typestate**
  - Add compile-fail cases for Endpoint/RestApi Default, missing endpoint description/success, missing API description/auth, and unavailable non-none ErrorSet construction.
  - Test every accepted success/request form, second-success error, auth order/conflicts, intentional multi-method policy, parameter/header precedence, and literal-equivalent fixture output.

Phase gate: R8–R13 pass on new fixtures, all old providers remain baseline-equivalent, and future-vocabulary reserved names remain unused. Non-exhaustive enforcement is still deferred deliberately.

## Phase 4 — Connect Runtime and Query Forms

### Wave 8 — Definition Forms and Runtime Wiring (Parallel)

- [x] **Add Definition Forms**
  - Own schematic-define changes for endpoint CrossOrigin replacement, request-id/secret sugar, auth_policy, pager-managed markers, parameter locations with the GET-body rejection, and narrow wire-name/Rust-name mapping (R14–R16, R29–R31).
  - Store adoption markers as serializable definition data. Reject unmatched adoption and managed required inputs lacking the pagination-authored default. Reject a request body on an endpoint built with `Endpoint::get` with API, endpoint, and method context; location never enters endpoint identity. Reuse the resolved-strategy validator and PagerManagedRequiredWithoutDefault diagnostic.
  - Retain a path-query warning for existing providers; prepare the hard-error path but enable it only with completion of all nine migrations in Phase 6.
- [x] **Wire Runtime Emission**
  - Own generator/runtime changes consuming those agreed fields. Apply endpoint policy to redirects and followed links, and emit request-id/secret markers in the predecessor's runtime form without new redaction logic.
  - Preserve GitLab's git_ref Rust name for wire name ref. Optional `recursive`/`beta` parameters added by the merges are ordinary query inputs with setters, emitted through the normal encoder and exported like any other query parameter.

Checkpoint: integrate the two branches only after agreeing on field signatures. Replace R27's no-endpoint-cross-origin assertion with R14 coverage; retain its no-GraphQL assertion.

### Wave 9 — Verify Public Behavior (Sequential)

- [x] **Verify Behavior**
  - Use two local origins in the predecessor's L1 harness: an endpoint allow_origins override reaches the second origin; a sibling under same_origin refuses it and the second origin records zero requests for that sibling. Cover invalid exact origins and caller tightening.
  - Assert request-id normalization/header validity and secret-marker emission, without duplicating predecessor redaction tests. Cover Some(AuthPolicy) through generated execution/emission.
  - Assert default/caller query composition, encoding once, repeated-key conflicts, serialized markers, matched pager control, required-managed rejection, and GET-body rejection through public results. Old providers still match baselines.

Phase gate: R14–R15 and new R29–R31 fixtures pass, output-specific validation remains honest, and full area checks pass without enabling the hard path-query error prematurely.

## Phase 5 — Convert the Import Emitter and OpenAI

### Wave 10 — Repeatable Builder Imports (Sequential)

- [x] **Convert Emitter**
  - Change definition_gen.rs to emit fallible builders, nominal registration support selected by the spike, explicit typed registrations in preserved order, and the generic overlay call. The importer may use crate-internal construction without public Default.
  - Normalize imported query-bearing paths in the shared import route before builder construction. Constants and supported placeholders become caller query inputs; a constant-suffixed path whose base route has a twin merges into that twin (R29). Report a request body on an imported GET operation (R31). Reject malformed/ambiguous expressions with endpoint context. Keep the vendored OpenAI source unchanged.
  - Implement a tested recorded transform only where reproduction established its necessity. Keep handwritten overlay files outside the emitter's output ownership; no OpenAI-specific emitter branch.
- [x] **Convert OpenAI**
  - Before regenerating, run the R29 schema gate on each of the seven beta pairs: compare flattened request and response JSON schemas; use the beta type where it is a superset (log equal cases); for any pair where neither is a superset, do not merge, log the differences for the author, and keep the hard `?` error disabled.
  - Regenerate OpenAI's definitions once using the new emitter, merging each gated beta pair into its twin with an optional `beta` query parameter. Convert its hand-written pagination overlay to endpoint-level assignments; unknown overlay endpoint ids remain errors.
  - Route the OpenAI assembly/registry through its context; explicitly preserve old component names/order except the ruled container removals. Do not schedule another OpenAI definition rewrite in Phase 6.

### Wave 11 — Import Regression Coverage (Sequential)

- [x] **Prove Reimports**
  - For a small real importer-produced fixture, compile/evaluate emitted builder source and compare its public RestApi/registry result with the in-memory import. Re-import twice to prove determinism and execute JSON/YAML path-normalization matrices.
  - Test a generic configured overlay call and unchanged overlay source; re-import OpenAI into a temporary destination and prove resolved endpoint pagination survives.
  - Run R10/R24 for OpenAI and all nine-path fixture cases; log each removed Beta* endpoint, its surviving twin, and the URL equivalence with and without `beta` per endpoint/output. Check the isolated generated crate. Refresh committed affected artifacts through the generator so ordinary drift tests remain green.

Phase gate: R22 passes, OpenAI's original count/order (less exactly the merged Beta* endpoints) and pagination resolve identically, each merged endpoint with `beta=true` reproduces its removed endpoint's URL and without it its twin's URL, and artifacts are equal or individually explained against history while matching final committed output exactly.

## Phase 6 — Migrate Remaining APIs and Remove Legacy Surface

### Wave 12 — Verified Provider Batches (Sequential)

- [x] **Convert Providers**
  - Use Phase 1's smallest-first inventory for every remaining REST API. For each batch, migrate its definition function, ordered typed registrations, lookup assembly, headers/auth/params, and every endpoint in one pass. Neither WebSocket definitions nor type-module stored-form tests are wholesale rewritten.
  - Move pagination exceptions to endpoint builders and retain each API's existing strategy functions in its own module. Remove or convert all 35 Gitea/GitHub/GitLab/Bitbucket legacy declarations according to their current resolved behavior, preserving the 94 authored pagers and their controls across all APIs. Bitbucket GetSource's binary non-list remains a non-list; its inert declaration is removed without authoring a pager. No shared presets.
  - Merge Gitea GetGitTreeRecursive into GetGitTree with an optional `recursive` query parameter, and convert the GitLab ref query input, within their API batches; migrate the inventoried users of the removed Gitea request type. Migrate inventoried GitLab call sites in the same batch, preserving git_ref where possible; log unavoidable source changes.
  - Handle aliases/re-exports/foreign types with register_as/rust_path and schema_name only where needed. Preserve explicit registry order and nested components; enumerate dropped container components by API and name.
  - End each batch with original endpoint count/ordered-id assertions (less exactly the endpoints R29 merges, listed by id), R10 JSON comparison, R19 pagination-resolution comparison, R24 generated-byte/file-set comparison, isolated compile checks, and affected dependent tests. Regenerate affected committed outputs before ordinary drift tests. Fix failures before the next batch and log the last completed API for unattended recovery.

### Wave 13 — Legacy Removal and Enforcement (Sequential)

- [x] **Remove Legacy Interfaces**
  - After the final provider migration, remove pagination exception tables and their validation, PaginationStyle, PaginationResponse, with_pagination, and the two legacy EndpointParams fields. Retain OpenAI overlay id validation.
  - Remove hand-typed registry filling and string-vs-string completeness checks; replace them with destination-context validation while preserving strict OpenAPI completeness coverage. Module-group assembly returns definitions/registry together for all lookup and generator entry points.
  - Repeat oauth_scopes/auth_policy and Schema.module_path reader scans. Remove oauth_scopes from live definition, exporter extension, emitter, and consumers; retain/test auth_policy. Apply/log PD21 without breaking WebSocket readers.
  - Enable the hard `?` error in shared validation/build/import boundaries only now, after confirming all nine paths are migrated and no merge left a same-route duplicate; if the schema gate blocked an OpenAI pair, leave it disabled and log the hold for the author. Assert diagnostics name API, endpoint, and offending path.
- [x] **Enforce Construction**
  - Apply non_exhaustive to Endpoint/RestApi after a workspace consumer scan confirms migration. Add external literal compile-fail coverage and source scans forbidding Endpoint literals, ApiResponse/ApiRequest/Schema constructors, json_type strings, and string registrations in provider authoring, with only narrow type/test exceptions.
  - Complete R27's six-entry retirement ledger: name each replacement and preserve pagination-resolution coverage, EndpointParams allow-list coverage, and no-GraphQL coverage. Remove tautological tests/tables only with a logged reason or behavioral replacement.
  - Verify removed identifiers have no live consumers across the workspace; distinguish historical documentation from code. Confirm every API exposes its expected count and id list.

Phase gate: R6, R12, R16, R18–R21, R23, R27–R31 pass across the complete provider set. Every generated URL for the nine cases preserves ordinary decoded path/query behavior; GitLab reserved characters and Unicode form one correctly encoded value. No legacy pagination types, query-bearing paths, or external construction literals remain live.

## Phase 7 — Regenerate and Verify Dependents

### Wave 14 — Final Artifact Integration (Sequential)

- [x] **Regenerate Artifacts**
  - Run the generator for the complete Rust/schema, OpenAPI, and Postman trees; never hand-edit generated source. Review complete file-set and byte differences against the Phase 1 baseline and log each intended difference by API, endpoint/registry entry, file, and cause.
  - Final drift L1 compares against final committed artifacts with no allow-list. Historical definition normalization remains narrow and reviewable; migration baselines remain available through review.
  - Run `just test`, `just lint`, `just doctest`, `just test-schema`, and `just lint-schema` in schematic. Include new schematic-runtime checks using the actual predecessor recipes; current area recipes omit that not-yet-created package.

### Wave 15 — Dependents and Platform Evidence (Parallel with Separate Build Targets)

- [x] **Verify Dependents**
  - Resolve actual package names from inventory and run root `just test {pkg}` for sniff/lib, biscuit-speaks/lib, model-citizen/lib, homelab/lib, eversolo-integration, arcam-amp-integration, sony-receiver-integration, and unfolded-integration-helper, plus any additional discovered direct consumers. Include schematic-oauth and all five WebSocket definition builds/tests.
  - Compile relevant lib/bin/test targets plus declared examples/benches and changed reverse dependencies. Run corresponding lint recipes. Fix only migration-induced call-site failures and log each touched consumer; an unrelated failure is recorded separately.
- [x] **Verify Platforms**
  - Load the os skill before choosing remote procedures or reporting a platform unavailable. Reuse qualifying evidence per required cell; otherwise run required checks on macOS, Linux, native Windows, and WSL2 using documented non-interactive host access and isolated target directories.
  - Exercise generated paths, compile-fail fixtures, temporary artifact paths, query encoding, and affected dependents on each required environment. Record exact commands, features, results, hosts, and missing evidence; never report one host's pass as four-platform coverage.
  - Use only documented unattended credential procedures. If a host is inaccessible, record the blocker and evidence still required; do not add CI cells or silently weaken acceptance. Run existing relevant L2 suites only via just test-l2 with background/no-focus harness visibility.

Phase gate: generated-schema and all affected consumer checks pass; the evidence ledger distinguishes passing environments from unverified ones. Full four-environment evidence is required for an unqualified completion claim.

## Phase 8 — Update Authoring Guidance and Close Evidence

### Wave 16 — Documentation and Closure (Parallel)

- [x] **Update Authoring Guidance**
  - Update schematic-define README/rustdoc, area and relevant package READMEs, definition/registration and IO topic pages, schematic-define/Schematic skills, and `.claude/commands/create-schematic-api.md` to teach builders, explicit context assembly, typed aliases, auth coherence, pagination, parameter locations with the GET-body rule, and endpoint identity as `(method, path)` (R26).
  - Provide compact compiling examples for ordinary types, alias registration, containers, and one API definition; explain required facts and errors. Use a Mermaid flow for context → endpoint builds → API validation → registry/output assembly. Current-behavior docs never name feature/fix directories.
  - Review docs/comments on every behavior-changed symbol and remove drift. Update dependency docs only for actual crate additions/removals. Search for stale literal/string/pagination authoring patterns, permitting only explicit historical mentions.
- [x] **Close Evidence Ledger**
  - Map every R1–R31 requirement to concrete tests, scan results, artifacts, and platform evidence. Record every PD default, non-exhaustive outcome, dispatcher choice/bounds, structural carrier, fallible signature, Default allow-list, field disposition, normalization, retired test, dependent edit, and unexplained failure as it occurs.
  - Complete all 16 review-agenda entries, including accepted-gap candidates (optional auth, wider auth dual-source cleanup, HuggingFace stub shapes, orphaned types) and the already accepted convenience-method omission. No gap is silently claimed fixed.
  - Document the implemented R13 method table, reserved names, final descriptor/serde/adoption contracts, and facts needed to refresh `2026-10-15-schematic-definition-vocabulary` and `2026-10-16-schematic-response-model-graphql` after author review 1. Do not implement their features or refresh them prematurely.

Final checkpoint: run focused documentation/doctest and changed-test checks, then the required area/dependent gates for any new executable changes. Review requirement coverage and historical differences; no broad test rerun without a new change or unresolved concern. Report **implementation complete, ready for review** only when evidence supports it, explicitly identifying justified exceptions or unverified environments. Do not commit, move the feature, or run `just complete`.

### Input Robustness Matrix

This applies to the descriptor deserialization boundary and the changed OpenAPI import normalization inputs. Plain output snapshots, serializers, and builder fields are not configuration inputs; do not impose a new format policy on every definition field or build the later self-contained bundle. Phase 1 binds the descriptor's final field spellings and importer-supported formats; add a column for every additional load-bearing field the selected implementation actually reads. Tables below define the new boundary contracts rather than changing unrelated importer behavior.

Descriptor JSON is read into data and then validated through the public destination-context/API build result. The fixture is a real context-produced descriptor referencing a registered nominal type, with separate context-produced primitive and wrapper fixtures for their positive cases. Reserved structural data round-trips internally but cannot become authored structural registration in this feature.

| Shape (JSON) | Component/schema name | Generated Rust path | Wrapper/reference shape metadata | Reserved structure |
| --- | --- | --- | --- | --- |
| Control | Matching registered nominal name builds | Matching registered public path builds | Matching wrapper and leaf registration builds | Explicit null builds as the authored None value |
| Absent | Reject missing required identity | Reject missing path | Reject missing required wire-shape metadata | Accept as reserved None, separately asserted from explicit null |
| Explicit null | Reject for nominal reference; native non-component case uses an explicit discriminator | Reject | Reject | Accept reserved None by this format's documented optional-field contract |
| Wrong whole type | Reject | Reject | Reject | Reject |
| One wrong element | Scalar: an array value is rejected | Scalar: an array value is rejected | Reject mixed malformed nested shape | Reject malformed nested TypeRef data |
| Every wrong element | Scalar: reject array value | Scalar: reject array value | Reject wholly malformed shape | Reject wholly malformed structural data |
| Empty | Reject empty nominal name | Reject empty path | Reject invalid empty wire shape | Reject empty object/array lacking a valid variant |
| Duplicate key | Reject | Reject | Reject | Reject |
| Trailing/invalid content | Reject whole input | Reject whole input | Reject whole input | Reject whole input |

Use separate non-component discriminator columns if the spike's descriptor representation requires them. Missing versus null may share a documented outcome only for the reserved optional structure; neither may silently clear a required identity. A syntactically valid forged name, path, shape, or Some(structure) that does not match the destination context yields an API/endpoint/type validation error. Matching deserialized references remain accepted. Private fields and successful serde parsing alone are not proof.

For imported query-bearing paths, the edited field is the path-map key under an existing operations object; existing method/operation facts stay fixed. Parameters below refer only to declarations the normalizer consumes to reconcile query inputs; a format permitting omitted parameters defines that absence explicitly. Run the table independently for JSON and YAML through the public import result, not a parsing helper.

| Shape (JSON and YAML, separate runs) | paths object/path key | parameters collection (if consumed) | Each consumed parameter name | Each consumed parameter location |
| --- | --- | --- | --- | --- |
| Control | Fixture imports with expected normalized path and caller query values | Valid declarations preserve query controls | Matching wire name appears in public definition/request | query location matches public request |
| Absent | Missing paths rejects/no usable API; deleting the selected key removes that endpoint, never invents it | Explicitly means no declared parameters; unambiguous path constants still normalize | Reject missing name | Reject missing location |
| Explicit null | Reject null paths/operation value | Reject; do not read as omitted declarations | Reject | Reject |
| Wrong whole type | Reject non-map paths or non-string key | Reject non-array value | Reject | Reject |
| One wrong element | Reject malformed entry among valid paths | Reject malformed entry among valid parameters | Reject malformed name among valid declarations | Reject malformed location among valid declarations |
| Every wrong element | Reject all-malformed entries | Reject all-malformed entries | Reject all malformed names | Reject all malformed locations |
| Empty | No usable API is rejected at definition validation; empty path key rejected | Empty array is explicit zero declarations | Reject empty name | Reject empty location |
| Duplicate key | Reject duplicate authored path keys where forbidden | Reject duplicate collection key | Reject duplicate field key; contradictory repeated wire declarations fail validation | Reject duplicate field key |
| Trailing/invalid content | Reject whole document | Reject whole document | Reject whole document | Reject whole document |

Keep legal repeated parameters in distinct locations distinct; this table does not forbid valid OpenAPI behavior. If reconciliation reads another field such as a parameter default, schema type, or Rust-name extension, Phase 1 must add its complete column and outcomes before implementation. Do not accept null via Option or silently filter malformed declarations. Import normalization also covers malformed query expressions, unsupported placeholders, duplicate query keys, and suffixed paths whose merge would conflict with the twin's declared parameters, with endpoint-specific errors. If enforcing these affected-input contracts exposes an existing parser defect, fix it at the shared read boundary or log a concrete unmet requirement; do not treat a one-off negative case as complete coverage.

The import matrices also cover every additional field read during reconciliation:

| Shape (JSON and YAML independently) | required | schema | schema.type | GET requestBody |
| --- | --- | --- | --- | --- |
| Control | Native true/false preserved in public ParamDef | Supported schema preserves query type | Nonempty supported string preserves query type | Absent permits GET |
| Absent | Explicit OpenAPI default false | Reject missing schema for consumed declaration | Explicit unconstrained type becomes Json | Accept no body |
| Explicit null | Reject | Reject | Reject | Reject with API/endpoint/GET context |
| Wrong whole type | Reject nonboolean, including quoted YAML false | Reject nonobject | Reject nonstring | Reject |
| One wrong element | Reject array representation | Reject mixed array representation | Reject array representation | Reject |
| Every wrong element | Reject array representation | Reject array representation | Reject array representation | Reject |
| Empty | Reject empty string/object/array | Accept explicit unconstrained object as Json | Reject empty string/object/array | Reject any body presence, including empty object |
| Duplicate key | Reject before map collection | Reject before map collection | Reject before map collection | Reject before map collection |
| Trailing/invalid content | Reject whole input | Reject whole input | Reject whole input | Reject whole input |

A metadata-only foundation import may contain no operations; normal `run_import`
rejects it as having no usable endpoints before writing either output form.
This preserves foundational metadata imports while enforcing the usable-API boundary.

The additive endpoint transfer field has the following explicit JSON contract;
its Collection representation is the existing pagination model.

| Shape | Endpoint pagination |
| --- | --- |
| Control | Explicit collection reaches API build, resolved item type and caller knobs |
| Absent | No endpoint assignment; legacy row/default/inference remains applicable |
| Explicit null | Same documented optional no-assignment state, separately asserted |
| Wrong whole type | Reject scalar instead of Collection |
| One wrong element | Reject mixed array instead of Collection |
| Every wrong element | Reject malformed array instead of Collection |
| Empty | Reject empty object or array lacking Collection fields |
| Duplicate key | Reject duplicate pagination field |
| Trailing/invalid content | Reject whole transfer document |

- [x] **Verify Input Matrices**
  - Phase 2 owns the descriptor JSON matrix; Phase 5 owns the JSON and YAML import matrices. One test per format walks its entire applicable matrix from a real-tool fixture, making one edit per cell and asserting the public definition/API validation outcome. The unedited control proves the fields affect output; assert endpoint presence, schema/path/shape, and emitted query values rather than just serde success.
  - Preserve duplicate keys and trailing content using textual mutations; a parsed JSON Value cannot represent duplicate keys. Use fixture variants for legal non-component/wrapper states. Unknown malformed data must not turn into empty successful output.
  - Audit affected read paths for `#[serde(default)]`, Option where absence/null differ, `filter_map(.. as_str())`, `unwrap_or_default()`, and `.ok()` swallowing load-bearing errors. Any review finding reports every other cell in its row and column. Matrix evidence is required at the owning phase's gate, not deferred to final review.


## Phase 9 — Normalize Schema References (Review 1 Remediation)

Author review 1, item 1 rejected embedding descriptor evidence in stored schema references. The follow-up rulings of 2026-10-18 fixed the design below. This phase replaces Phase 1's **Wrapper storage** rule and the stored-reference part of Wave 4's destination validation. Typed authoring (`DefinitionContext`, `Registration`, `rust_path!`, builders) is unchanged, so the roughly 1,700 provider uses of `TypeDesc` as `register()`'s return value need no edits.

### Necessary Rules

| Item | Rule |
| --- | --- |
| Reference form | REST `ApiResponse::Json`, `ApiRequest::Json` and `FormFieldKind::Json` store a `TypeRef`: canonical component names (`Named`) plus wrapper shape (`Array`, `Optional`, `Map`). A reference holds no Rust path and no copy of a registered schema. |
| `TypeRef` extension | Extend `TypeRef`, never replace it, only where lowering needs it: the Rust spelling of primitive widths (`i8`–`u128`, `isize`/`usize`, `f32`/`f64`, `char`, `()`), and Rust map key types. Free-form JSON uses the existing `PrimitiveType::Json`. Log each extension. Inline JSON schemas for primitives are derived from the `TypeRef`, not stored. |
| Registry | `RestApi` gains an ordered `schemas` field built by `RestApiBuilder::build` from the **whole** destination context: one entry per component name holding its Rust path and wire schema, with nested `$defs` flattened into entries (the `SchemaRegistry` extraction logic moves to `schematic-define`). Explicit registrations that no endpoint uses are retained. APIs that share a module group (ollama, emqx) each carry the full group registry; the duplication is intentional so that each `RestApi` is self-contained. |
| Validation | API build checks that every `Named` reference in every endpoint resolves in `api.schemas`. A dangling name fails with API, endpoint and name. Context collision aggregation is unchanged. A single reference-resolution function serves build and any deserialized `RestApi`. `DefinitionContext::validate_reference`, the context's descriptor list, `DefinitionError::InvalidDescriptor`, the lowered-path agreement check, and the forgery tests are removed; resolution and dangling-name tests replace them. |
| `TypeDesc` | Authoring handle only: it lowers to a `TypeRef` at the builder and is never stored on an endpoint. Its serde derives and the reserved `structure: Option<TypeRef>` carrier are removed, because no reader remains and a `TypeRef` reference can already carry `OneOf`/`AnyOf`/`AllOf`. **This departs from the spec's approved reserved-carrier constraint;** log it and surface it in the closing handoff for the next specification's refresh (R6). |
| WebSocket | `Schema` loses only its `descriptor` field. `type_name`, `module_path`, `Schema::new` and `Schema::with_path` stay for WebSocket messages. |
| Legacy untyped REST | REST has no string-schema form. Verified on 2026-10-18: no shipped artifact depends on the generator's untyped branch (`needs_response_import`). The `use super::responses::*` imports in the elevenlabs, github, bitbucket and openai artifacts come from unqualified pagination `item_type` strings, which this phase keeps. Fixtures that inject legacy REST strings become name references, or deliberately dangling ones where they test diagnostics. |
| `SchemaRegistry` | Deleted with no compatibility view: `SchemaRegistry`, `SchemaRegistryLike`, `validate_completeness`, `get_registry`, `get_registries_for_module`, every provider's `openapi_registry()`, and `ModuleGroup::registry`. OpenAPI export, generator validation and the pipeline read `api.schemas`; `registry_for` is removed. Strict OpenAPI completeness coverage is kept, re-pointed at `api.schemas`. No consumers exist outside `schematic/` (workspace scan, 2026-10-18). |
| Out of scope | Pagination `Collection::item_type` remains a Rust-path string, and WebSocket schemas remain strings. R7 bundle construction, `oneOf` vocabulary, and any reachable-only registry trimming belong to the next specification. |

### Wave 17 — Stored Model (Sequential)

- [x] **Normalize References**
  - In `schematic-define`, change the three REST JSON carriers to `TypeRef`, add `RestApi::schemas` and its entry type with serde derives, extend `TypeRef` as ruled, and lower `TypeDesc` to `TypeRef` in `Success::json`, request JSON and `FormField::json_desc`.
  - Populate `schemas` from the context in `RestApiBuilder::build`, flattening nested `$defs`; replace descriptor validation with the reference-resolution function. Remove `Schema::descriptor`, the `TypeDesc` serde derives and `structure`, `validate_reference`, the context's descriptor list and `InvalidDescriptor`.
  - Re-point pagination's root-`Vec` recognition (`root_array_item_type`) at `TypeRef::Array(Named)`, resolving the item's Rust path through `api.schemas`. Re-point OpenAPI body and response export at `TypeRef` plus `api.schemas` and drop the separate registry argument.
  - Update the touched rustdoc/module docs in `schema.rs`, `request.rs`, `response.rs`, `builders.rs` and `registration.rs` in the same change.
  - Replace the descriptor and forgery cases in `define/tests/builders.rs` and `define/tests/typed_registration.rs` with: resolved references for every wrapper and primitive form; a dangling name naming API, endpoint and name; a deserialized `RestApi` with a dangling reference rejected by the same function; whole-context registry retention, including an unused registration; and nested-`$defs` flattening.

Checkpoint: `schematic-define` builds and its L1 tests and doctests pass. Downstream crates may not compile yet.

### Wave 18 — Consumers (Parallel with Disjoint Ownership)

- [ ] **Migrate Definitions**
  - Delete `SchemaRegistry`, its lookups, `ModuleGroup::registry` and every provider's `openapi_registry()`; `assemble_module_group` returns the APIs only. After the callback returns, it sets every member API's `schemas` to the finished context's `components()` and re-runs `validate_references`. An API built before a later member registered its types would otherwise carry a partial registry, and grouped OpenAPI output (ollama, emqx) would change. Rewrite the bitbucket and registry tests that read `schema.descriptor` to assert `TypeRef` references and `api.schemas` entries. Keep `definitions/tests/registry_invariants.rs` coverage by pointing it at `api.schemas`.
- [ ] **Migrate Generator**
  - The Rust generator derives type paths from `TypeRef` plus `api.schemas`. Remove the untyped-schema branch of `needs_response_import`, keeping the pager-item branch. Remove `registry_for` and the `SchemaRegistry` parameters from the pipeline, validation and OpenAPI export. Re-point `openapi_strict_completeness` and the other `gen/tests/l1` users. Convert the legacy-string REST fixtures named in the Wave 7 log entry to name references or deliberate dangling references.
  - The import emitter (`definition_gen.rs`) keeps emitting builder source; confirm the OpenAI re-import still reproduces the committed definition.

Checkpoint: `just test`, `just lint` and `just doctest` pass in `schematic/`.

### Wave 19 — Artifacts, Baselines and Documentation (Sequential)

- [ ] **Regenerate and Compare**
  - Regenerate the definition JSON snapshots and the canonical baseline under `gen/tests/fixtures/definition-baseline/`, then the Rust, OpenAPI and Postman trees. Generated Rust, OpenAPI and Postman output should be byte-identical to the Phase 7 artifacts. Log any difference by API, file and cause; the only expected changes are in the definition snapshots and baseline (reference form and the new `schemas` field).
  - Run `just test-schema`, `just lint-schema`, the dependent checks from Wave 15 (including sniff's Gitea and GitLab users), and `just check-tier-coverage schematic`.
- [ ] **Correct Documentation**
  - Update `docs/typed-registration.md`, `docs/definition-builders.md`, `docs/io/export-openapi.md`, `docs/io/import-openapi.md`, `docs/generation/generated-clients.md`, `docs/generation/validation.md`, the `define` and `definitions` READMEs, and `.claude/skills/schematic-define/` to describe name-plus-wrapper references and `RestApi::schemas`, and to remove `SchemaRegistry`.
  - Amend the implementation log's **Builder implementation and lowering** entry with a dated departure note, and record this phase's evidence. Update the plan and log frontmatter inventories, and set `completed_phase: 9` and `implemented: true` only when every gate has passed.

Phase gate: every Phase 9 checkbox is complete; `just test`, `just lint`, `just doctest`, `just test-schema` and `just lint-schema` pass; generated artifacts show no unexplained drift; and no reader of `Schema::descriptor`, `SchemaRegistry` or `validate_reference` remains in the workspace. The terminal state is **implementation complete, ready for review**.

## Refresh log

### 2026-10-18 — Review 1 ruling: normalized schema references

- Added Phase 9 from author review 1, item 1 and its follow-up rulings: `TypeRef` references, a whole-context registry on `RestApi`, removal of legacy untyped REST schemas, and deletion of `SchemaRegistry`.
- Recorded the 2026-10-18 verification that no shipped artifact depends on the untyped branch, and the workspace scan showing no `SchemaRegistry` consumers outside `schematic/`.
- Phases 1–8 remain as executed; Phase 9 supersedes the Phase 1 **Wrapper storage** rule.

### 2026-10-14 — Pagination review 2 and implementation log

- Replaced the obsolete prerequisite hold with the delivered interface map and the three settled pagination rulings, while retaining Phase 1's execution-tree verification.
- Updated inventory expectations to distinguish 547 resolved endpoints from 511 production literals, and recorded 94 pagers and 32 verification-task endpoints.
- Clarified that removal of Bitbucket GetSource's inert legacy declaration preserves its binary non-list behavior; retained existing provider-local helpers and definition-gap exclusions.
- Added the actual resolution/adoption, ABI 4, capability, recovery, overlay, and frozen-test contracts that migration must preserve.
- Corrected runner advice: check-drift already uses nextest; test-e2e still needs conversion if used. No implementation, pipeline-order change, or new feature scope was introduced.
