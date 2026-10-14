---
spec: /Volumes/coding/wt/rusty-biscuit/feat-modern-config/schematic/features/2026-10-13-schematic-pagination/spec.md
plan: schematic/features/2026-10-13-schematic-pagination/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - schematic/define/src/lib.rs
    - schematic/define/src/openapi/export/info.rs
    - schematic/define/src/openapi/export/mod.rs
    - schematic/define/src/openapi/import/builder.rs
    - schematic/define/src/pagination.rs
    - schematic/define/src/pagination/collection.rs
    - schematic/define/src/pagination/resolve.rs
    - schematic/define/src/pagination/selector.rs
    - schematic/define/src/pagination/strategy.rs
    - schematic/define/src/pagination/validate.rs
    - schematic/define/src/params/mod.rs
    - schematic/define/src/prelude.rs
    - schematic/define/src/request.rs
    - schematic/define/src/types.rs
    - schematic/define/tests/endpoint_literal_surface.rs
    - schematic/define/tests/pagination_inventory.rs
    - schematic/define/tests/pagination_model.rs
    - schematic/define/tests/pagination_serde.rs
    - schematic/define/tests/pagination_support/mod.rs
    - schematic/define/tests/pagination_tables.rs
    - schematic/definitions/src/anthropic/mod.rs
    - schematic/definitions/src/artificial_analysis/mod.rs
    - schematic/definitions/src/bitbucket/mod.rs
    - schematic/definitions/src/elevenlabs/mod.rs
    - schematic/definitions/src/emqx/mod.rs
    - schematic/definitions/src/eversolo/mod.rs
    - schematic/definitions/src/gitea/mod.rs
    - schematic/definitions/src/github/mod.rs
    - schematic/definitions/src/gitlab/mod.rs
    - schematic/definitions/src/huggingface/mod.rs
    - schematic/definitions/src/lmstudio/mod.rs
    - schematic/definitions/src/ollama/mod.rs
    - schematic/definitions/src/openai/mod.rs
    - schematic/definitions/src/registry/mod.rs
    - schematic/definitions/src/samsung_smart_tv/mod.rs
    - schematic/definitions/src/unfolded_circle/core_rest/mod.rs
    - schematic/gen/src/codegen/api_struct/tests.rs
    - schematic/gen/src/codegen/client/tests.rs
    - schematic/gen/src/codegen/endpoint_methods/tests.rs
    - schematic/gen/src/codegen/module_docs.rs
    - schematic/gen/src/codegen/request_enum.rs
    - schematic/gen/src/definition_gen.rs
    - schematic/gen/src/diagnostics.rs
    - schematic/gen/src/export/naming.rs
    - schematic/gen/src/export/openapi.rs
    - schematic/gen/src/export/postman/collection.rs
    - schematic/gen/src/export/postman/mod.rs
    - schematic/gen/src/export/postman/tests.rs
    - schematic/gen/src/output/mod.rs
    - schematic/gen/src/test_utils.rs
    - schematic/gen/src/validation.rs
    - schematic/gen/tests/l1/e2e_generation.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/l1/http_client.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/openapi_strict_completeness.rs
    - schematic/gen/tests/l1/output_validation.rs
    - schematic/gen/tests/l1/pagination_validation.rs
    - schematic/gen/tests/l1/partial_generation.rs
    - schematic/gen/tests/l1/postman_golden.rs
docs_updated_during_phase_2:
    - schematic/define/README.md
    - schematic/docs/generation/validation.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_3:
    - schematic/runtime/src/lib.rs
    - schematic/runtime/src/call.rs
    - schematic/runtime/src/client.rs
    - schematic/runtime/src/meta.rs
    - schematic/runtime/src/pager/mod.rs
    - schematic/runtime/src/pager/spec.rs
    - schematic/runtime/src/pager/engine.rs
    - schematic/runtime/src/pager/json.rs
    - schematic/runtime/src/pager/link.rs
    - schematic/runtime/src/pager/progress.rs
    - schematic/runtime/src/pager/request.rs
    - schematic/runtime/src/pager/pages.rs
    - schematic/runtime/src/pager/error.rs
    - schematic/runtime/tests/l1/main.rs
    - schematic/runtime/tests/l1/pager_support.rs
    - schematic/runtime/tests/l1/pager_mechanisms.rs
    - schematic/runtime/tests/l1/pager_outside.rs
    - schematic/runtime/tests/l1/pager_rules.rs
    - schematic/runtime/tests/l1/pager_surface.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_after_without_a_resumable_cursor.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_after_without_a_resumable_cursor.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_before_on_a_forward_only_cursor.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_before_on_a_forward_only_cursor.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_knob_after_pages.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_knob_after_pages.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_offset_on_a_page_number_endpoint.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_offset_on_a_page_number_endpoint.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_on_an_unpaginated_endpoint.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_on_an_unpaginated_endpoint.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_page_on_a_cursor_endpoint.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_page_on_a_cursor_endpoint.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_page_size_not_authored.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_page_size_not_authored.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_pages_on_a_session_endpoint.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_pages_on_a_session_endpoint.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_prev_without_a_backward_direction.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_prev_without_a_backward_direction.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_set_page_size_not_adjustable.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_set_page_size_not_adjustable.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_total_without_a_total.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_total_without_a_total.stderr
    - schematic/gen/src/lib.rs
    - schematic/gen/src/codegen/mod.rs
    - schematic/gen/src/codegen/pagination/mod.rs
    - schematic/gen/src/codegen/pagination/tests.rs
    - schematic/gen/src/codegen/traits.rs
    - schematic/gen/src/codegen/endpoint_methods/mod.rs
    - schematic/gen/src/codegen/endpoint_methods/names.rs
    - schematic/gen/src/codegen/endpoint_methods/docs.rs
    - schematic/gen/src/codegen/request_structs/mod.rs
    - schematic/gen/src/codegen/request_structs/endpoint_spec.rs
    - schematic/gen/src/codegen/request_structs/tests.rs
    - schematic/gen/src/output/mod.rs
    - schematic/gen/src/output/assemble/api.rs
    - schematic/gen/src/output/assemble/shared.rs
    - schematic/gen/src/validation.rs
    - schematic/gen/tests/common/fresh_output.rs
    - schematic/gen/tests/l1/e2e_generation.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/schema/src/lib.rs
    - schematic/schema/src/shared.rs
    - schematic/schema/src/bitbucket/requests.rs
    - schematic/schema/src/gitea/requests.rs
    - schematic/schema/src/github/requests.rs
    - schematic/schema/src/gitlab/requests.rs
docs_updated_during_phase_3:
    - schematic/docs/generation/validation.md
    - schematic/docs/generation/partial-generation.md
    - schematic/gen/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_4:
    - schematic/runtime/src/hooks.rs
    - schematic/runtime/src/pager/pages.rs
    - schematic/runtime/src/pager/request.rs
    - schematic/runtime/tests/l1/main.rs
    - schematic/runtime/tests/l1/pager_support.rs
    - schematic/runtime/tests/l1/pager_collect.rs
    - schematic/runtime/tests/l1/pager_transport.rs
    - schematic/define/src/pagination/resolve.rs
    - schematic/define/tests/pagination_tables.rs
    - schematic/gen/src/codegen/mod.rs
    - schematic/gen/src/codegen/pagination/mod.rs
    - schematic/gen/src/codegen/pagination/tests.rs
    - schematic/gen/src/codegen/request_structs/mod.rs
    - schematic/gen/src/codegen/request_structs/into_parts.rs
    - schematic/gen/src/output/assemble/api.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
docs_updated_during_phase_4:
    - schematic/define/README.md
    - schematic/gen/README.md
    - schematic/docs/runtime/README.md
    - schematic/docs/runtime/deadlines-and-timeouts.md
    - schematic/docs/runtime/hooks.md
    - schematic/docs/runtime/metadata-and-redaction.md
    - schematic/docs/runtime/origins-and-redirects.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_5:
    - schematic/runtime/Cargo.toml
    - schematic/runtime/src/pager/session.rs
    - schematic/runtime/src/pager/pages.rs
    - schematic/runtime/src/pager/error.rs
    - schematic/runtime/src/pager/request.rs
    - schematic/runtime/src/pager/engine.rs
    - schematic/runtime/src/pager/progress.rs
    - schematic/runtime/src/pager/mod.rs
    - schematic/runtime/tests/l1/main.rs
    - schematic/runtime/tests/l1/pager_support.rs
    - schematic/runtime/tests/l1/pager_session.rs
    - schematic/runtime/tests/l1/pager_partition.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_keep_alive_not_adjustable.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_keep_alive_not_adjustable.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_no_manual_open_or_close.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_no_manual_open_or_close.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_on_a_sessionless_endpoint.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_on_a_sessionless_endpoint.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_pages_escape_the_scope.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_pages_escape_the_scope.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_renew_without_a_renew_step.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_renew_without_a_renew_step.stderr
    - schematic/define/tests/pagination_partitions.rs
    - schematic/gen/src/codegen/pagination/mod.rs
    - schematic/gen/src/codegen/pagination/tests.rs
    - schematic/gen/src/codegen/endpoint_methods/mod.rs
    - schematic/gen/src/codegen/endpoint_methods/names.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
docs_updated_during_phase_5:
    - schematic/docs/runtime/README.md
    - schematic/docs/runtime/deadlines-and-timeouts.md
    - schematic/docs/generation/validation.md
    - schematic/docs/dependencies.md
    - schematic/gen/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_6:
    - schematic/definitions/src/anthropic/mod.rs
    - schematic/definitions/src/bitbucket/mod.rs
    - schematic/definitions/src/elevenlabs/mod.rs
    - schematic/definitions/src/emqx/mod.rs
    - schematic/definitions/src/gitea/mod.rs
    - schematic/definitions/src/github/mod.rs
    - schematic/definitions/src/gitlab/mod.rs
    - schematic/definitions/src/huggingface/mod.rs
    - schematic/definitions/src/lib.rs
    - schematic/definitions/src/openai/mod.rs
    - schematic/definitions/src/openai_pagination.rs
    - schematic/definitions/tests/pagination_coverage/main.rs
    - schematic/definitions/tests/pagination_coverage/forges.rs
    - schematic/definitions/tests/pagination_coverage/openai.rs
    - schematic/definitions/tests/pagination_coverage/others.rs
    - schematic/gen/src/definition_gen.rs
    - schematic/gen/src/import_pipeline.rs
    - schematic/gen/src/main.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/import_overlay.rs
    - schematic/gen/tests/l1/openapi_import_test.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt
    - schematic/justfile
    - schematic/schema/src/bitbucket/requests.rs
    - schematic/schema/src/elevenlabs/requests.rs
    - schematic/schema/src/gitea/requests.rs
    - schematic/schema/src/github/requests.rs
    - schematic/schema/src/gitlab/requests.rs
    - schematic/schema/src/openai/requests.rs
    - schematic/schema/tests/github_client.rs
    - sniff/lib/src/remote/envelope.rs
    - sniff/lib/src/remote/github.rs
    - sniff/lib/tests/l1/remote_providers.rs
docs_updated_during_phase_6:
    - schematic/docs/io/import-openapi.md
    - schematic/gen/README.md
    - schematic/definitions/README.md
    - schematic/specs/openai/README.md
    - sniff/docs/repos/repo-tech-design.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_7:
    - schematic/gen/src/cargo_gen.rs
    - schematic/schema/Cargo.toml
    - schematic/schema/Cargo.lock
    - schematic/schema/tests/pagination.rs
    - schematic/schema/tests/compile_fail.rs
    - schematic/schema/tests/fixtures/compile_fail/adopted_field_is_not_a_request_setter.rs
    - schematic/schema/tests/fixtures/compile_fail/adopted_field_is_not_a_request_setter.stderr
    - schematic/schema/tests/fixtures/compile_fail/after_on_a_page_number_endpoint.rs
    - schematic/schema/tests/fixtures/compile_fail/after_on_a_page_number_endpoint.stderr
    - schematic/schema/tests/fixtures/compile_fail/before_on_a_forward_only_cursor.rs
    - schematic/schema/tests/fixtures/compile_fail/before_on_a_forward_only_cursor.stderr
    - schematic/schema/tests/fixtures/compile_fail/knob_after_pages.rs
    - schematic/schema/tests/fixtures/compile_fail/knob_after_pages.stderr
    - schematic/schema/tests/fixtures/compile_fail/no_alternate_selection.rs
    - schematic/schema/tests/fixtures/compile_fail/no_alternate_selection.stderr
    - schematic/schema/tests/fixtures/compile_fail/offset_on_a_page_number_endpoint.rs
    - schematic/schema/tests/fixtures/compile_fail/offset_on_a_page_number_endpoint.stderr
    - schematic/schema/tests/fixtures/compile_fail/page_on_a_cursor_endpoint.rs
    - schematic/schema/tests/fixtures/compile_fail/page_on_a_cursor_endpoint.stderr
    - schematic/schema/tests/fixtures/compile_fail/pager_on_an_unpaginated_endpoint.rs
    - schematic/schema/tests/fixtures/compile_fail/pager_on_an_unpaginated_endpoint.stderr
    - schematic/schema/tests/fixtures/compile_fail/partitioned_without_partitions.rs
    - schematic/schema/tests/fixtures/compile_fail/partitioned_without_partitions.stderr
    - schematic/schema/tests/fixtures/compile_fail/prev_without_a_backward_direction.rs
    - schematic/schema/tests/fixtures/compile_fail/prev_without_a_backward_direction.stderr
    - schematic/schema/tests/fixtures/compile_fail/session_on_a_sessionless_endpoint.rs
    - schematic/schema/tests/fixtures/compile_fail/session_on_a_sessionless_endpoint.stderr
    - schematic/schema/tests/fixtures/compile_fail/set_page_size_not_adjustable.rs
    - schematic/schema/tests/fixtures/compile_fail/set_page_size_not_adjustable.stderr
    - schematic/schema/tests/fixtures/compile_fail/total_without_a_total.rs
    - schematic/schema/tests/fixtures/compile_fail/total_without_a_total.stderr
    - schematic/schema/tests/fixtures/pass/authored_controls.rs
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_8:
    - schematic/define/src/lib.rs
    - schematic/define/src/pagination/resolve.rs
    - schematic/define/src/pagination/strategy.rs
    - schematic/define/src/params/mod.rs
    - schematic/definitions/src/bitbucket/mod.rs
    - schematic/definitions/src/github/mod.rs
    - schematic/definitions/src/gitlab/mod.rs
    - schematic/gen/src/codegen/endpoint_methods/mod.rs
    - schematic/gen/src/codegen/endpoint_methods/names.rs
    - schematic/gen/src/codegen/pagination/mod.rs
    - schematic/gen/src/codegen/request_structs/into_parts.rs
    - schematic/runtime/src/pager/engine.rs
    - schematic/runtime/src/pager/pages.rs
    - schematic/runtime/src/pager/request.rs
    - schematic/schema/tests/compile_fail.rs
    - .config/nextest.toml
docs_updated_during_phase_8:
    - schematic/README.md
    - schematic/define/README.md
    - schematic/definitions/README.md
    - schematic/gen/README.md
    - schematic/runtime/README.md
    - schematic/docs/dependencies.md
    - schematic/docs/generation/request-side.md
    - schematic/docs/generation/validation.md
    - schematic/docs/runtime/README.md
    - schematic/docs/runtime/calling-an-api.md
    - schematic/docs/runtime/deadlines-and-timeouts.md
docs_created_during_phase_8:
    - schematic/docs/pagination.md
skills_files_updated_during_phase_8:
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/os/build-hosts.md
    - .claude/skills/os/windows-paths.md
source_code:
    - schematic/define/src/lib.rs
    - schematic/define/src/openapi/export/info.rs
    - schematic/define/src/openapi/export/mod.rs
    - schematic/define/src/openapi/import/builder.rs
    - schematic/define/src/pagination.rs
    - schematic/define/src/pagination/collection.rs
    - schematic/define/src/pagination/resolve.rs
    - schematic/define/src/pagination/selector.rs
    - schematic/define/src/pagination/strategy.rs
    - schematic/define/src/pagination/validate.rs
    - schematic/define/src/params/mod.rs
    - schematic/define/src/prelude.rs
    - schematic/define/src/request.rs
    - schematic/define/src/types.rs
    - schematic/define/tests/endpoint_literal_surface.rs
    - schematic/define/tests/pagination_inventory.rs
    - schematic/define/tests/pagination_model.rs
    - schematic/define/tests/pagination_serde.rs
    - schematic/define/tests/pagination_support/mod.rs
    - schematic/define/tests/pagination_tables.rs
    - schematic/definitions/src/anthropic/mod.rs
    - schematic/definitions/src/artificial_analysis/mod.rs
    - schematic/definitions/src/bitbucket/mod.rs
    - schematic/definitions/src/elevenlabs/mod.rs
    - schematic/definitions/src/emqx/mod.rs
    - schematic/definitions/src/eversolo/mod.rs
    - schematic/definitions/src/gitea/mod.rs
    - schematic/definitions/src/github/mod.rs
    - schematic/definitions/src/gitlab/mod.rs
    - schematic/definitions/src/huggingface/mod.rs
    - schematic/definitions/src/lmstudio/mod.rs
    - schematic/definitions/src/ollama/mod.rs
    - schematic/definitions/src/openai/mod.rs
    - schematic/definitions/src/registry/mod.rs
    - schematic/definitions/src/samsung_smart_tv/mod.rs
    - schematic/definitions/src/unfolded_circle/core_rest/mod.rs
    - schematic/gen/src/codegen/api_struct/tests.rs
    - schematic/gen/src/codegen/client/tests.rs
    - schematic/gen/src/codegen/endpoint_methods/tests.rs
    - schematic/gen/src/codegen/module_docs.rs
    - schematic/gen/src/codegen/request_enum.rs
    - schematic/gen/src/definition_gen.rs
    - schematic/gen/src/diagnostics.rs
    - schematic/gen/src/export/naming.rs
    - schematic/gen/src/export/openapi.rs
    - schematic/gen/src/export/postman/collection.rs
    - schematic/gen/src/export/postman/mod.rs
    - schematic/gen/src/export/postman/tests.rs
    - schematic/gen/src/output/mod.rs
    - schematic/gen/src/test_utils.rs
    - schematic/gen/src/validation.rs
    - schematic/gen/tests/l1/e2e_generation.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/l1/http_client.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/openapi_strict_completeness.rs
    - schematic/gen/tests/l1/output_validation.rs
    - schematic/gen/tests/l1/pagination_validation.rs
    - schematic/gen/tests/l1/partial_generation.rs
    - schematic/gen/tests/l1/postman_golden.rs
    - schematic/runtime/src/lib.rs
    - schematic/runtime/src/call.rs
    - schematic/runtime/src/client.rs
    - schematic/runtime/src/meta.rs
    - schematic/runtime/src/pager/mod.rs
    - schematic/runtime/src/pager/spec.rs
    - schematic/runtime/src/pager/engine.rs
    - schematic/runtime/src/pager/json.rs
    - schematic/runtime/src/pager/link.rs
    - schematic/runtime/src/pager/progress.rs
    - schematic/runtime/src/pager/request.rs
    - schematic/runtime/src/pager/pages.rs
    - schematic/runtime/src/pager/error.rs
    - schematic/runtime/tests/l1/main.rs
    - schematic/runtime/tests/l1/pager_support.rs
    - schematic/runtime/tests/l1/pager_mechanisms.rs
    - schematic/runtime/tests/l1/pager_outside.rs
    - schematic/runtime/tests/l1/pager_rules.rs
    - schematic/runtime/tests/l1/pager_surface.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_after_without_a_resumable_cursor.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_after_without_a_resumable_cursor.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_before_on_a_forward_only_cursor.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_before_on_a_forward_only_cursor.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_knob_after_pages.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_knob_after_pages.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_offset_on_a_page_number_endpoint.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_offset_on_a_page_number_endpoint.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_on_an_unpaginated_endpoint.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_on_an_unpaginated_endpoint.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_page_on_a_cursor_endpoint.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_page_on_a_cursor_endpoint.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_page_size_not_authored.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_page_size_not_authored.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_pages_on_a_session_endpoint.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_pages_on_a_session_endpoint.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_prev_without_a_backward_direction.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_prev_without_a_backward_direction.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_set_page_size_not_adjustable.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_set_page_size_not_adjustable.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_total_without_a_total.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_total_without_a_total.stderr
    - schematic/gen/src/lib.rs
    - schematic/gen/src/codegen/mod.rs
    - schematic/gen/src/codegen/pagination/mod.rs
    - schematic/gen/src/codegen/pagination/tests.rs
    - schematic/gen/src/codegen/traits.rs
    - schematic/gen/src/codegen/endpoint_methods/mod.rs
    - schematic/gen/src/codegen/endpoint_methods/names.rs
    - schematic/gen/src/codegen/endpoint_methods/docs.rs
    - schematic/gen/src/codegen/request_structs/mod.rs
    - schematic/gen/src/codegen/request_structs/endpoint_spec.rs
    - schematic/gen/src/codegen/request_structs/tests.rs
    - schematic/gen/src/output/assemble/api.rs
    - schematic/gen/src/output/assemble/shared.rs
    - schematic/gen/tests/common/fresh_output.rs
    - schematic/schema/src/lib.rs
    - schematic/schema/src/shared.rs
    - schematic/schema/src/bitbucket/requests.rs
    - schematic/schema/src/gitea/requests.rs
    - schematic/schema/src/github/requests.rs
    - schematic/schema/src/gitlab/requests.rs
    - schematic/runtime/src/hooks.rs
    - schematic/runtime/tests/l1/pager_collect.rs
    - schematic/runtime/tests/l1/pager_transport.rs
    - schematic/gen/src/codegen/request_structs/into_parts.rs
    - schematic/runtime/Cargo.toml
    - schematic/runtime/src/pager/session.rs
    - schematic/runtime/tests/l1/pager_session.rs
    - schematic/runtime/tests/l1/pager_partition.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_keep_alive_not_adjustable.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_keep_alive_not_adjustable.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_no_manual_open_or_close.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_no_manual_open_or_close.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_on_a_sessionless_endpoint.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_on_a_sessionless_endpoint.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_pages_escape_the_scope.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_pages_escape_the_scope.stderr
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_renew_without_a_renew_step.rs
    - schematic/runtime/tests/fixtures/compile_fail/pager_session_renew_without_a_renew_step.stderr
    - schematic/define/tests/pagination_partitions.rs
    - schematic/definitions/src/lib.rs
    - schematic/definitions/src/openai_pagination.rs
    - schematic/definitions/tests/pagination_coverage/main.rs
    - schematic/definitions/tests/pagination_coverage/forges.rs
    - schematic/definitions/tests/pagination_coverage/openai.rs
    - schematic/definitions/tests/pagination_coverage/others.rs
    - schematic/gen/src/import_pipeline.rs
    - schematic/gen/src/main.rs
    - schematic/gen/tests/l1/import_overlay.rs
    - schematic/gen/tests/l1/openapi_import_test.rs
    - schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt
    - schematic/justfile
    - schematic/schema/src/elevenlabs/requests.rs
    - schematic/schema/src/openai/requests.rs
    - schematic/schema/tests/github_client.rs
    - sniff/lib/src/remote/envelope.rs
    - sniff/lib/src/remote/github.rs
    - sniff/lib/tests/l1/remote_providers.rs
    - schematic/gen/src/cargo_gen.rs
    - schematic/schema/Cargo.toml
    - schematic/schema/Cargo.lock
    - schematic/schema/tests/pagination.rs
    - schematic/schema/tests/compile_fail.rs
    - schematic/schema/tests/fixtures/compile_fail/adopted_field_is_not_a_request_setter.rs
    - schematic/schema/tests/fixtures/compile_fail/adopted_field_is_not_a_request_setter.stderr
    - schematic/schema/tests/fixtures/compile_fail/after_on_a_page_number_endpoint.rs
    - schematic/schema/tests/fixtures/compile_fail/after_on_a_page_number_endpoint.stderr
    - schematic/schema/tests/fixtures/compile_fail/before_on_a_forward_only_cursor.rs
    - schematic/schema/tests/fixtures/compile_fail/before_on_a_forward_only_cursor.stderr
    - schematic/schema/tests/fixtures/compile_fail/knob_after_pages.rs
    - schematic/schema/tests/fixtures/compile_fail/knob_after_pages.stderr
    - schematic/schema/tests/fixtures/compile_fail/no_alternate_selection.rs
    - schematic/schema/tests/fixtures/compile_fail/no_alternate_selection.stderr
    - schematic/schema/tests/fixtures/compile_fail/offset_on_a_page_number_endpoint.rs
    - schematic/schema/tests/fixtures/compile_fail/offset_on_a_page_number_endpoint.stderr
    - schematic/schema/tests/fixtures/compile_fail/page_on_a_cursor_endpoint.rs
    - schematic/schema/tests/fixtures/compile_fail/page_on_a_cursor_endpoint.stderr
    - schematic/schema/tests/fixtures/compile_fail/pager_on_an_unpaginated_endpoint.rs
    - schematic/schema/tests/fixtures/compile_fail/pager_on_an_unpaginated_endpoint.stderr
    - schematic/schema/tests/fixtures/compile_fail/partitioned_without_partitions.rs
    - schematic/schema/tests/fixtures/compile_fail/partitioned_without_partitions.stderr
    - schematic/schema/tests/fixtures/compile_fail/prev_without_a_backward_direction.rs
    - schematic/schema/tests/fixtures/compile_fail/prev_without_a_backward_direction.stderr
    - schematic/schema/tests/fixtures/compile_fail/session_on_a_sessionless_endpoint.rs
    - schematic/schema/tests/fixtures/compile_fail/session_on_a_sessionless_endpoint.stderr
    - schematic/schema/tests/fixtures/compile_fail/set_page_size_not_adjustable.rs
    - schematic/schema/tests/fixtures/compile_fail/set_page_size_not_adjustable.stderr
    - schematic/schema/tests/fixtures/compile_fail/total_without_a_total.rs
    - schematic/schema/tests/fixtures/compile_fail/total_without_a_total.stderr
    - schematic/schema/tests/fixtures/pass/authored_controls.rs
    - .config/nextest.toml
documentation:
    - schematic/define/README.md
    - schematic/docs/generation/validation.md
    - schematic/docs/generation/partial-generation.md
    - schematic/gen/README.md
    - schematic/docs/runtime/README.md
    - schematic/docs/runtime/deadlines-and-timeouts.md
    - schematic/docs/runtime/hooks.md
    - schematic/docs/runtime/metadata-and-redaction.md
    - schematic/docs/runtime/origins-and-redirects.md
    - schematic/docs/dependencies.md
    - schematic/docs/io/import-openapi.md
    - schematic/definitions/README.md
    - schematic/specs/openai/README.md
    - sniff/docs/repos/repo-tech-design.md
    - schematic/README.md
    - schematic/runtime/README.md
    - schematic/docs/generation/request-side.md
    - schematic/docs/runtime/calling-an-api.md
    - schematic/docs/pagination.md
completed_phase: 8
implemented: true
packages:
    - schematic-define
    - schematic-gen
    - schematic-definitions
    - schematic-runtime
    - schematic-schema
    - sniff
---

# Implementation Log for 2026-10-13-schematic-pagination (8 phases)

## Phase 1

Phase 1 is a preflight phase. It changes no production source, test, doc or skill. It reconciles the spec with the delivered prerequisites, records the coverage inventory and the migration ledger, captures the artifact baseline, and decides whether the input robustness matrix applies. Checked on 2026-10-13 against HEAD `0cd6075e6` on `feat/modern-config`.

The four investigations ran in parallel as read-only subagents. Their full evidence reports are appended under "Evidence" below. Scratch artifacts, including the generated-output baseline and file hashes, are in `/tmp/pagination-phase1/`; they are temporary and can be regenerated.

### Prerequisite gate: PASS

All three prerequisites are in `_completed`, each spec has `status: implemented`, and each review 3 is `ready: true`:

- `2026-10-11-schematic-generation-integrity`
- `2026-10-12-schematic-runtime-envelope`
- `2026-10-12-schematic-endpoint-methods`

Every contract the spec needs exists in code:

- `schematic-runtime` 0.1.0, with `ABI = 3` matching the generator's `RUNTIME_ABI = 3`, and no `pager` module yet
- `EndpointSpec`, the `Json`/`Text`/`Binary`/`Empty` codecs, `Response`/`Outcome`/`into_parts`, and `RequestMeta.page`
- `TransportErrorKind::{CrossOriginRejected, InvalidArgument, DeadlineExceeded, …}` and `ErrorKind::RedirectFailed { target, category }`
- `CallError`, `Call::map_request`, `.full()` and `.build()`
- `Client::check_link::<R>(&Url)` and the checked absolute-URL request path
- `Deadline` and `send_raw`
- partial generation, the isolated fresh-output compile check, query separation, and typed `ValidationDiagnostic`

Phase 2 is not blocked. File:line citations are in the contract map below.

### Integration corrections found during reconciliation

The spec is a snapshot and is not edited. These are the corrections later phases apply; each cites the contract map.

1. **`validate_api` does not exist.** The validator entry point is `validate_rust_client(apis) -> Vec<ValidationDiagnostic>` in `schematic/gen/src/validation.rs:138`, together with the per-output validators. Pagination validation is wired there (Phase 2.2).
2. **The diagnostics format is already fixed.** It is `severity= api= endpoint= feature= output=: message`, from `ValidationDiagnostic` in `schematic/gen/src/diagnostics.rs`. The P8.3 default `<api>.<endpoint_id>: <rule>: <detail>` is aligned to it: the rule becomes new `Feature` ids, and the nearest-id hint goes in the message.
3. **There is no public relative-URL resolver, and the envelope exposes only a redacted final URL.** `ResponseMeta.request.url` is a `RedactedUrl`. The runtime resolves only redirect `Location` headers, internally (`client.rs:939`). Because the pager lives inside the runtime crate, it retains the unredacted final URL through an in-crate seam and resolves with `Url::join`. The same seam is the place to add a shared resolver; no public API is needed (Phase 4.2).
4. **The checked absolute-URL path works only through `RequestParts.path`.** A generated request cannot carry an arbitrary URL, so the pager follows a link through a runtime-internal wrapper `EndpointSpec`. The wrapper keeps the listing request's `Api`, `Success`, `Failure`, `Decoder` and `ENDPOINT_ID`, so `check_link::<R>` and origin refusal report the listing endpoint (Phase 4.2).
5. **`RequestMeta.page` has no input.** The pager sets it, in-crate, on each paged send's metadata (Phase 4.2).
6. **The `RestApi` literal count is larger than 18.** `RestApi` has no `Default`. Adding two fields touches:
   - the 18 production literals
   - 3 registry test literals
   - the importer builder (`schematic/define/src/openapi/import/builder.rs:362`)
   - the definition emitter (`schematic/gen/src/definition_gen.rs:141-159`)
   - many test literals in define and gen (about 122 raw `RestApi {` grep hits)

   Phase 2.1 owns all of them.
7. **`trybuild` and `wiremock` are already dev-dependencies of `schematic-runtime`** (`schematic/runtime/Cargo.toml:31-32`) and are already listed in both dependency docs. The spec's pre-authorization to add them is moot. The existing trybuild harness clears `RUSTC_WRAPPER`, which addresses the snapshot-portability risk the spec raises about kache-remapped paths.
8. **Only three pager names are reserved.** The generator's reserved list holds `pages`, `fetch_page` and `collect_all` (`PLANNED_CALL_METHODS`, `schematic/gen/src/codegen/endpoint_methods/names.rs:39`). Every new knob and entry name joins that list in Phase 3.2: `page_size`, `page`, `offset`, `after`, `before`, `prev`, `keep_alive`, `partitioned`, `set_page_size`, `total`, `with_session`, `renew`, `page_limit`.
9. **The ABI will change.** Under `schematic/runtime/src/lib.rs:96-101`, any change generated code can observe bumps the ABI. Generated pager impls will observe the new runtime traits, so Phase 3 bumps the runtime `ABI` and the generator's `RUNTIME_ABI` together, from 3 to 4. No ruling covers this; it is recorded as a design choice.
10. **The policy constructor's name differs.** The spec text says `allow_any`; code uses `danger_allow_any`. Tests use the code's name.
11. **`InvalidArgument` has no kinds.** It is a struct with a private reason. Page-size bounds, zero limits and arithmetic overflow use `InvalidArgument::new(reason)`, and the reason never quotes the value.
12. **The runtime `Call` fields are private to generated code,** and the generated wrapper is a private tuple field `.0`. Pager entry points are therefore runtime methods on `schematic_runtime::Call<'a, A, R>`, bounded by the pager traits, and the generated inherent forwards call them through `self.0`.
13. **The session-close grace has no spec-2 ruling.** The runtime-envelope log assigns it to this spec, so default 21 (5 s, capped by the per-request timeout) stands as Proposed.

### Authorization record

The full search is under "Authorization record" in the contract map. In summary:

| Step | Recorded confirmation | Treatment |
| --- | --- | --- |
| Regenerate `schematic-schema` | Not recorded | Required scope |
| Dependent-crate migrations (sniff, biscuit-speaks) | Not recorded | Required scope |
| Runtime pager module | Not recorded | Required scope; the dev-dependencies are already present |
| Definition authoring (`RestApi` fields, defaults, tables, overlay) | Not recorded | Required scope |
| Docs, skills and READMEs | Not recorded | Required scope (repo drift rule) |
| Network verification of U rows (optional) | Not recorded; explicitly PENDING | **Skipped**: U rows stay unpaginated and are listed as the R33 verification task |
| EMQX `ListAuthUsers` response fix (optional) | Not recorded; explicitly PENDING, and conditional on verification | **Skipped**: left unchanged and unpaginated |

Recorded as the plan requires: the only instruction covering the required-scope steps is the author's run instruction to implement this plan phase by phase. The runtime-envelope run used the same basis (`2026-10-12-schematic-runtime-envelope` implementation log). Nothing is committed, so every step stays reversible. `yolo: true` in the plan frontmatter is not treated as a grant. The two optional steps are not taken in any phase unless the author records approval.

### Inventory decisions for later phases

The recount is in "Coverage inventory" below. Decisions taken from it, smallest option consistent with the spec, are adopted as defaults here and applied in Phase 6:

- **Counts.** 18 REST APIs; 547 endpoints (= 547 request types = 547 call types; OpenAI 265); 511 production `Endpoint` literals (the spec's ~519 is stale); 35 legacy declarations (GitHub 9, GitLab 9, Bitbucket 9, Gitea 8); 156 collection endpoints (157 if Bitbucket `GetSource` is counted); 40 GET root `Vec<T>`; 61 OpenAI list candidates (S6 36, item-id 2, S7 9, S8 11, opt-outs 3).
- **Bitbucket has 8 wrapper lists, not 9.** `ListDirectoryContents` was merged into `GetSource` (a `Binary` response) by `9e65a90b5` on 2026-10-12. `GetSource` is not a list, and its `page`/`pagelen` fields stay.
- **Bitbucket `page` stays a request field on the 8 lists.** R7's knob rule decides this: `NextLink` has no page input, so nothing is adopted. This overrides R7's summary phrase "every `with_pagination` query parameter". `pagelen` is adopted through the authored page-size knob.
- **ElevenLabs `ListVoices` stays unpaginated.** The definition sends `page_token`, but the research note names the input `next_page_token`. The authoring table's contradiction rule ("record the discrepancy and leave that endpoint unpaginated") applies, so its fields are kept. It goes on the R33 verification list, and biscuit-speaks keeps its hand loop. Expected paginated count: 94 (GitHub 9, GitLab 9, Gitea 8, Bitbucket 8, OpenAI 58, ElevenLabs 2).
- **Two GET `Vec<T>` endpoints get opt-out entries:** GitLab `ListIssueParticipants` (unverified) and Gitea `GetTagReference` (not a list). Every other GET `Vec<T>` under a default is a real list.
- **The U-row wrapper collections get explicit unpaginated entries,** so the coverage test can assert `Unpaginated` as R28 requires:
  - Anthropic `ListModels`
  - the 7 EMQX wrapper lists in each of the two variants
  - Hugging Face `ListModelDiscussions`
  - the 2 Gitea tree endpoints

  Other wrapper collections with no pagination evidence resolve as "not a list" with no entry: Artificial Analysis, LM Studio, Ollama, and the 3 other ElevenLabs wrappers. Hugging Face's 10 and EMQX `ListAuthUsers`' root arrays have no API default, so they already resolve as unpaginated lists.
- **OpenAI page-size bounds vary by endpoint** (100, 10000, 1000, 180, and bucket-width-dependent maxima). The S6 default `PageSize` therefore carries no `max`, and a per-entry bound is authored only where the vendored document states one that holds for every request. That avoids R13 rejecting valid requests.
- **OpenAI cursor fields are typed `String`, `Option<String>` or `serde_json::Value`.** The cursor selector reads JSON string-or-null from the raw page, independent of the Rust field type.
- **GitLab keyset constants share names with declared params on `ListGroupProjects`** (`order_by`, `sort`). Alternates are not generated (P8 item 7), so Phase 2 validates an alternate's constants within the alternate only, and logs this as an open question for when alternates are emitted.
- **No adoptable parameter is provider-required,** so R2a needs no authored default for any real definition. R2a stays covered by model and runtime tests.

### Migration ledger (adopted-field removals)

The full ledger is in the evidence below. Actual dependents of `schematic-schema`: sniff, biscuit-speaks, and five homelab crates (`homelab`, `eversolo-integration`, `arcam-amp-integration`, `sony-receiver-integration`, `unfolded-integration-helper`). **model-citizen does not depend on `schematic-schema`,** contrary to the spec; it uses `schematic-definitions` types only.

| Site | Use | Migration (Phase 7.1) |
| --- | --- | --- |
| `sniff/lib/src/remote/github.rs:454` | `list_workflow_runs(..).per_page(limit)`, one page; anonymous retry on 401/403 | `.page_size(limit).fetch_page()`; per-call `.anonymous()` must survive into the pager |
| `sniff/lib/src/remote/bitbucket.rs:177,179` | `GetSource` `.pagelen`/`.page` hand loop | none (`GetSource` is not a list; fields kept) |
| `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:576,578` | `ListVoices` `.page_size`/`.page_token` loop | none while `ListVoices` stays unpaginated |
| `schematic/schema/tests/github_client.rs:549,556` | `per_page` | `fetch_page()` in the test |
| `schematic/schema/tests/non_json_endpoints.rs:65` | `GetSource` `pagelen` | none |
| `schematic/gen/tests/l1/fresh_output_compile.rs:1588-1698` | wire-baseline program sets Gitea `page`/`limit` and OpenAI `limit`/`after` | migrate the program; re-baseline the 4–5 affected lines of `query_request_lines.txt`, recorded as an intended change |
| homelab crates, model-citizen | none | none |

Documentation showing adopted setters (`schematic/docs/runtime/calling-an-api.md`, `request-side.md`, `deadlines-and-timeouts.md`, and the `schematic-define` skill) is drift that Phase 8.1 corrects.

### Input robustness scope: configuration-reader matrix does not apply

`RestApi`, `Endpoint`, `EndpointParams`, `ParamDef` and `PaginationStyle` derive neither `Deserialize` nor `Serialize`. Every define-side `from_str` is test-only, and the generator CLI takes an API name, not a definition file. The only file reader is the OpenAPI importer. It reads an OpenAPI document, an import format that produces no pagination, and this feature does not change its parsing.

**Boundary recorded:** serde on the new pagination model (R1) is a model round-trip facility with no configuration reader, so the per-field input robustness matrix is not built. Phase 2.3 keeps strict model round-trip and decode tests, and invents no loader:

- unknown fields rejected
- absent kept distinct from null where the model distinguishes them
- empty collections kept distinct from absent ones

The R12/R17 response-shape tests in Phases 3 and 4 remain required. If a later phase adds a reader, the matrix is built before that reader.

### Baseline (task 1.3)

All on macOS (M4 Max, 16 CPUs), rustc 1.98.1, nextest 0.9.136, kache 0.26.3 as `rustc-wrapper`. No pre-existing failures.

| Check | Result |
| --- | --- |
| Fresh generation into a temp dir vs committed `schematic/schema` | byte-identical (empty `diff -r`) |
| Fresh OpenAPI (15 files) and Postman (15 files) vs committed | byte-identical; sha256 of all 73 generated files recorded |
| `just test` (schematic area; includes runtime, fresh-output and compile-fail) | pass, 51.2 s: L1 1937/1937 (12 skipped), fresh-output 6/6, compile-fail 1/1 |
| `just lint` (includes runtime) | pass, 76.5 s |
| `just test-schema` | pass, 40/40 |
| `just lint-schema` | pass |
| `schematic-schema` cold build, all targets | 44.7 s with kache, 69.8 s without; library alone 35.5 s; cold clippy 60.6 s |

Recipe coverage: `just test` and `just lint` include `schematic-runtime`, so the plan's "area recipes must include the runtime package" holds with no recipe change. The generated surface has 547 request structs, 547 call types, 35 legacy `impl Paginated`, and 197 pagination-looking request fields across 79 structs.

### Requirement-to-test mapping for this phase

Phase 1 changes no behavior, so it adds no test. The evidence is the recorded baseline: the full L1 suite, lint, schema suite, and generation and export byte-equality. The input-matrix decision above sets what Phase 2.3 must test.

### Deviations in this phase

- Tasks 1.1, 1.2 and 1.3 ran concurrently. The plan orders 1.1 before 1.2 and 1.3 to gate them. The gate was confirmed first, with a direct check that the prerequisite symbols exist, and the three tasks are read-only, so running them together changed no outcome.
- The input-applicability survey (the last bullet of 1.3) ran with the 1.2 dependents survey because both are code-reading tasks.

### Human review agenda (status slots)

The agenda as written in the spec, copied verbatim:

> 1. Every default adopted from "Planning-time details" (P8 items 1 to 8, and defaults 9 to 22), with the value actually used.
> 2. The reconciliation of spec 2's actual names and kinds with this spec's names (runtime package name, `EndpointSpec` trait, `TransportError` kinds, the page field of `RequestMeta`, cross-origin and deadline entry points).
> 3. Every design choice not dictated by this spec: names of the define-side and runtime-side types, the shape of `PagerError`, the `with_session` signature, the entry-point method names, the generated constant layout, which rules from design.md section 8 were kept, renamed, or dropped.
> 4. Every request-struct field removed under the adoption rule, and every dependent-crate edit it forced.
> 5. The authored versus left-unpaginated list: per API, the endpoint ids authored, the ids left unpaginated, the reason (unverified, provider gap, definition gap, not a list), and the source for any row the implementer verified. This is the verification task of R33.
> 6. The re-counted numbers (list endpoints, declared endpoints, endpoint literals) and how they differ from the parent's.
> 7. The quick decode-path sample: the one host used, release-binary size before and after at the current endpoint count, the decision, and the numbers; no adoption percentage gate.
> 8. Compile-time and diagnostic observations: build time (`just lint-schema` / `just test-schema`) for `schematic-schema` before and after; the implementor-list truncation; any rust-analyzer observation.
> 9. Which operating systems and environments the tests were actually run on, and which are left to CI; any test made tolerant for Windows timers or paths; any trybuild snapshot made portable and how.
> 10. Every deviation from this spec, with the reason; every requirement not met or met differently.
> 11. Every open design question found, with the implementer's recommended answer. Candidates already known: whether to expose labeled alternates; whether `deny` should follow same-origin next links; whether to keep the table's nearest-id hint; whether partition support should ship now or wait for a first real user.
> 12. Drift found between comments and code, and how it was resolved.
> 13. Anything done under a pre-authorization that the author may want to reverse (regeneration diff size, edited dependents, the `ListAuthUsers` fix if taken).
> 14. The **Proposed** items that carry no ratification, each with the value used and the alternative: R14's widened stall rule (any repeat anywhere versus consecutive only); R9/R10's one generic pager, trait-bound encoding, and diagnostic attributes; the pager decode path (S11); named-struct strategy variants (S12).
> 15. The adoption rule (R7, default 12): which plain parameters were adopted, that each adopted one has its matching knob and is reachable for a single page through `fetch_page()` (R11a, D57 item 3), and every parameter whose field was kept because the strategy authors no matching knob.
> 16. The session-close grace (R22a, default 21): the grace value used and whether spec 2 ruled otherwise.
> 17. The `InvalidContinuation` incomplete reason (R21, P8 item 1) and the additional argument/transport mappings in `IncompleteReason`, and the wrapper-list item type default (default 22 / R26).
> 18. The stale-rule widening (R14): any test that depended on consecutive-only detection, and any provider whose legitimate pagination repeats a continuation.
> 19. Provenance only, no open question: D55-5 (pager-managed parameters exempt from required fidelity, R2a, R7), D56-1 (anonymous send when no credential resolves), D56-3 (per-request time under a deadline), D56-5 (`EndpointSpec` name), and D57 items 2, 3, 4, 5, 7, and 13 (latest handle across partitions, `fetch_page()`, the importer overlay, `SessionError`, the `RedirectFailed` mapping, and `Default` on request structs) are Decided; the log records only where the implementation applied them and any difference found when reconciling with spec 2 and spec 4b.
> 20. The rotating-handle partition risk (R23, D57 item 2): that it was logged, and any out-of-order handle or provider handle invalidation observed in tests or documentation.

Status slots, updated by each phase that produces evidence:

1. **Defaults adopted** (P8 items 1–8, defaults 9–22). *Phase 1:* confirmed that no prerequisite rules over them, except that default 17 (`deny` follows same-origin links) is satisfied by the landed runtime (`origin.rs:10-24`). The grace in default 21 has no spec-2 ruling. The other values are applied in later phases. *Status:* open.
2. **Reconciliation with spec 2.**
   - *Phase 1:* done; see "Integration corrections".
   - Names that match: the runtime package `schematic-runtime`, `EndpointSpec`, `TransportErrorKind::{CrossOriginRejected, InvalidArgument, DeadlineExceeded}`, `RequestMeta.page: Option<u32>`, `Client::check_link::<R>`, and `Deadline`/`.deadline(&Deadline)`.
   - Differences: `validate_api` (actually `validate_rust_client`), the resolver, the absolute-URL seam, and `danger_allow_any`.
   - *Status:* done.
3. **Design choices** (type names, `PagerError`, `with_session` signature, entry names, constant layout, section 8 rules). *Phase 1:* the ABI 3→4 bump, runtime-side entry points on `Call`, and in-crate seams were chosen. *Status:* open for later phases.
4. **Removed request fields and dependent edits.** *Phase 1:* the ledger is planned (above). *Status:* open (Phase 7).
5. **Authored versus left-unpaginated list, and verification task (R33).**
   - *Phase 1:* inventory recorded.
   - Unverified and left unpaginated:
     - Anthropic `ListModels`
     - EMQX (both variants)
     - Hugging Face (11)
     - GitLab `ListIssueParticipants`
     - Gitea trees (2)
     - ElevenLabs `ListVoices` (input name)
   - Provider gaps: OpenAI `RetrieveVectorStoreFileContent` and `SearchVectorStore`.
   - Definition gap: EMQX `ListAuthUsers`.
   - Not a list: Gitea `GetTagReference`, Bitbucket `GetSource`.
   - *Status:* open (Phase 6 and 8).
6. **Re-counted numbers.** *Phase 1:* done (above). 18 APIs; 547 endpoints; 511 production literals vs ~519; 35 declared vs the parent's 27; 156 collections vs ~157; OpenAI 265/61. *Status:* done.
7. **Decode-path sample.** *Phase 1:* fact recorded. Unpaginated JSON already uses `serde_json::from_slice` (`codec.rs:252-254`), and only hooked decoding uses `from_value`. *Status:* open (Phase 4.1).
8. **Compile-time observations.** *Phase 1:* baseline recorded. `schematic-schema` cold build takes 44.7 s with kache and 69.8 s without (library alone 35.5 s); cold clippy 60.6 s; host load average was 30–41 during measurement. *Status:* open (after-build in Phase 7).
9. **OS and environments.** *Phase 1:* macOS only; no OS-specific work. *Status:* open (Phase 8.2).
10. **Deviations.** *Phase 1:* concurrent preflight tasks (above). *Status:* open.
11. **Open design questions.**
    - Candidates from the spec: expose alternates, `deny` with same-origin (answered by the runtime: followed), the nearest-id hint, and partition timing.
    - New in Phase 1:
      - may an alternate's constant shadow a declared plain param (GitLab `ListGroupProjects`)?
      - the ElevenLabs `ListVoices` input name
    - *Status:* open.
12. **Comment and code drift.** *Phase 1:* drift detected, not yet corrected:
    - the `Paginated` doc in `schematic/gen/src/codegen/traits.rs` describes a nonexistent `fetch_all_pages`
    - runtime docs show adopted setters

    Both are corrected when the code changes (Phases 3 and 8). *Status:* open.
13. **Pre-authorized actions the author may reverse.** *Phase 1:* none taken. The authorization record is above. *Status:* open.
14. **Unratified Proposed items** (R14, R9/R10, S11, S12). *Phase 1:* none applied yet. *Status:* open.
15. **Adoption rule (R7).** *Phase 1:* planned adoptions are in the inventory. Kept fields: Bitbucket `page` ×8, `GetSource` `page`/`pagelen`, OpenAI `order`, and ElevenLabs `ListVoices` `page_size`/`page_token`. *Status:* open.
16. **Session-close grace.** *Phase 1:* spec 2 made no ruling, so 5 s is the default. *Status:* open (Phase 5).
17. **`InvalidContinuation` reason, argument/transport mappings, wrapper item type default.** *Phase 1:* confirmed that `InvalidArgument` (struct) and `RedirectFailed` exist to map onto. *Status:* open (Phase 4).
18. **Stall-rule widening.** *Status:* open (Phases 3 and 4).
19. **Decided provenance.** *Phase 1:* the landed runtime matches each of these:
    - D56-1 (anonymous when no credential resolves; `credentials.rs:295-314`)
    - D56-3 (`Expiration::start`, `deadline.rs:104-123`)
    - D56-5 (`EndpointSpec`)
    - D57 item 7 (`RedirectFailed`)
    - D57 item 13 (`Default` rule)

    *Status:* open for application in later phases.
20. **Rotating-handle partition risk.** *Status:* open (Phase 5).

### Validation run in this phase

The full baseline above is the validation: `just test`, `just lint`, `just test-schema`, `just lint-schema`, fresh-output and compile-fail all pass, and no file in the repository changed apart from this log and the plan and spec frontmatter.

### Evidence

#### Pagination Phase 1, task 1.1: contract map

Worktree `/Volumes/coding/wt/rusty-biscuit/feat-modern-config`, checked 2026-10-13 against HEAD `0cd6075e6`. The only working-tree change is the untracked `schematic/features/2026-10-13-schematic-pagination/implementation-log.md` (12 lines, preflight header only).

Abbreviations:

| Short form | Path |
| --- | --- |
| RT | `schematic/runtime/src/` |
| GEN | `schematic/gen/src/` |
| DEF | `schematic/define/src/` |
| PG | `schematic/features/2026-10-13-schematic-pagination/` |
| RE | `schematic/features/_completed/2026-10-12-schematic-runtime-envelope/` |
| EM | `schematic/features/_completed/2026-10-12-schematic-endpoint-methods/` |
| GI | `schematic/features/_completed/2026-10-11-schematic-generation-integrity/` |
| MR | `schematic/reviews/2026-07-31-macro-review/spec.md` |

##### Prerequisite gate verdict

**PASS. No prerequisite contract is absent.** Phase 2 is not blocked on runtime contracts. Some integration corrections must be recorded first; they are listed under "Discrepancies with spec refresh notes" below.

All three prerequisites are in `_completed`. Each spec has `status: implemented` and each review 3 has `ready: true`:

- GI spec.md:22, review-3.md:10
- RE spec.md:22, review-3.md:3
- EM spec.md:22, review-3.md:3

Every contract that spec "Dependencies and ordering" (PG spec.md:322) requires exists in code:

- the runtime package
- `EndpointSpec` and typed send
- `Response`, plus `RequestMeta` with its page field
- `TransportError` with `CrossOriginRejected` and the pre-send `InvalidArgument` kind
- `ErrorKind::RedirectFailed`
- the managed client and credential resolution
- the origin policy and `check_link::<R>`
- `Deadline`
- `send_raw`

The generation-integrity deliverables are also present: partial generation, the isolated compile check, query separation, and typed diagnostics.

##### Contract map

Status key: **M** = matches the spec/refresh notes; **D** = differs (explained); **A** = absent.

###### 1. Runtime package, ABI, modules

| Spec expectation | Code | Status |
| --- | --- | --- |
| `schematic-runtime` 0.1.0 at `schematic/runtime` | `schematic/runtime/Cargo.toml:2-3` | M |
| Runtime `ABI` = 3 | RT lib.rs:102 `pub const ABI: u32 = 3;` | M |
| Generator `RUNTIME_ABI` = 3, equality enforced | GEN lib.rs:101. The equality test at GEN lib.rs:117-141 parses the runtime's lib.rs. The generated crate asserts it at `schematic/schema/src/lib.rs:68-70` (emitted by GEN output/assemble/lib_rs.rs:139-150). Partial generation checks the marker at GEN output/inventory.rs:136, 440-480. | M |
| Modules include `call`, `client`, `endpoint`, `codec`, `response`, `error`, `meta`, `origin`, `deadline`; no `pager` | RT lib.rs:51-67 also has `api_client` (private), `credentials`, `encode` (private), `hooks`, `options`, `raw`, `redact`, `transport`. No `pager` module exists. `grep pager\|paginat RT` hits only meta.rs:64-65. | M |
| ABI impact of adding a pager | The rule at RT lib.rs:96-101 is "Increment it with every change that generated code can observe". Generated code that implements pager traits would observe them, so an ABI bump is likely. This is not ruled anywhere; record it as a decision. | note |

###### 2. `EndpointSpec` and codecs

| Spec expectation | Code | Status |
| --- | --- | --- |
| `EndpointSpec` with `Api`, `Success`, `Failure`, `Decoder`, `ENDPOINT_ID`, `METHOD`, `into_parts` | RT endpoint.rs:29-61. `Api: Api` :31, `Success: Send + 'static` :33, `Failure: ErrorBody` :36, `Decoder: Decode<Output = Self::Success>` :38, `ENDPOINT_ID` :41, `METHOD: Method` :45. Also `DECLARED_ERROR_STATUSES` (default 400..=599) :49. `fn into_parts(self) -> Result<RequestParts, InvalidArgument>` :60. The trait bound is `Send + 'static` :29. | M (one extra const) |
| `RequestParts` | RT endpoint.rs:80-94. Fields `method`, `path`, `query: Vec<(String,String)>` (unencoded, duplicates kept), `headers`, `body: RequestBody`. All fields are `pub`, so the parts can be rebuilt for a follow-up request. | M |
| Codecs `Json`, `Text`, `Binary`, `Empty` | RT codec.rs:242 / 279 / 291 / 303. Re-exported at RT lib.rs:71. | M |
| `Decode` uses a live body | RT codec.rs:33-54: `fn decode(body: LiveBody) -> impl Future<Output = Result<Self::Output, DecodeFailure>> + Send`, plus `decode_hooked`. `LiveBody` (codec.rs:61) has `collect()` (:162), `chunk()`, `status()`, `headers()` (unredacted, :125). | M |
| S11 decode path (from_slice vs from_value) | `Json::decode` collects the body and calls `serde_json::from_slice` (codec.rs:252-254). `decode_hooked` with a `pre_response_json` hook parses to `Value` and then runs `from_value` (codec.rs:264-268). Unpaginated sends therefore use `from_slice` when there are no hooks. A pager that decodes the page type through `R::Decoder` reuses that path. Item extraction from the decoded page is the open S11 question. | M (fact) |
| Decode failure evidence | `DecodeFailure { kind, retained: Bytes }` RT codec.rs:207-221. `DecodeFailureKind::{Interrupted, Malformed, Hook}` :225-233. | M |

###### 3. Envelope

| Spec expectation | Code | Status |
| --- | --- | --- |
| `Response<T,E>` | RT response.rs:27-32 `{ pub meta: ResponseMeta, pub outcome: Outcome<T,E> }` | M |
| `Outcome` | RT response.rs:36-41 `Success(T) \| Failure(ApiError<E>)` | M |
| `into_parts()` / `into_result()` | response.rs:70-72 `into_parts(self) -> (ResponseMeta, Outcome<T,E>)`. response.rs:62-67 `into_result(self) -> Result<T, ApiError<E>>` discards the metadata. Also `data()`, `error()`, `is_decode_failure()`, `is_interrupted()` (:45-84). | M |
| `ApiError<E>` | response.rs:98-104 `{ kind: ErrorKind<E>, body: Bytes }` (raw body kept) | M |
| `ResponseMeta` | RT meta.rs:117-134 `{ request: RequestMeta, status, headers (redacted, repeated kept), request_id: Option<String>, elapsed }`, `#[non_exhaustive]`. Elapsed is the whole send (meta.rs:128-133). | M |
| `RequestMeta.page: Option<u32>`, left unset by ordinary sends | meta.rs:64-66, documented "Reserved for pagers ... the runtime always leaves it `None`". Set to `None` in `capture` (:86) and `unassembled` (:103). No input sets it: `CallOptions` has only `auth` and `deadline` (RT options.rs:16-19). The fields are `pub` but the struct is `#[non_exhaustive]` (meta.rs:50), so code inside the runtime crate can assign `meta.request.page` after a send. | M. Pager must supply the input (RE implementation-log.md:3625-3631, 3939). |
| Final response URL for relative-link resolution | Exposed only as `ResponseMeta.request.url: RedactedUrl` (meta.rs:57). This is the hop that received the final response (meta.rs:44-48), with secret query values and userinfo redacted. `RedactedUrl::as_str()` is at RT redact.rs:217-224. The unredacted URL is not exposed. | D (see Discrepancies) |

###### 4. Errors

| Spec expectation | Code | Status |
| --- | --- | --- |
| `TransportError` | RT error.rs:16-26 `{ pub request: Box<RequestMeta>, pub kind: TransportErrorKind, pub elapsed: Duration }`, `#[non_exhaustive]`. Constructor `new` :30. | M |
| `TransportErrorKind` (full list) | RT error.rs:48-81: `CrossOriginRejected { target: RedactedUrl }`, `InvalidArgument(InvalidArgument)`, `InvalidUrl`, `Connect`, `Tls`, `Timeout`, `DeadlineExceeded { completed_requests: u64, deadline_elapsed: Duration }`, `InvalidClient(BuildError)`, `Other`. Marked `#[non_exhaustive]`. | M |
| Page-size error kind | There is no dedicated variant. `InvalidArgument` is a struct with a private `reason: Cow<'static, str>` (error.rs:109-124), built by `InvalidArgument::new(reason)`. It has no kinds or variants. Display is `"invalid argument: {reason}"` (:126-130). The docs say the reason must not quote the value (:107-108). A page-size bound error has to be `TransportErrorKind::InvalidArgument(InvalidArgument::new("..."))` or a new kind. | M (refresh note correct). Spec Design :240 leaves this open. |
| `ErrorKind<E>` (full list) | RT response.rs:140-168: `Declared(E)`, `Undeclared`, `MalformedError`, `MalformedSuccess`, `BodyInterrupted`, `RedirectRefused { reason: RedirectRefusal }`, `RedirectFailed { target: RedactedUrl, category: FailureCategory }`, `Hook`. Marked `#[non_exhaustive]`. | M (also has `Undeclared`, `MalformedError`, `Hook`, which the spec does not mention) |
| `RedirectRefusal` | response.rs:190-202: `CrossOrigin`, `HopLimit`, `MissingLocation`, `InvalidLocation`, `UnsafeLocation` | M |
| `FailureCategory` | response.rs:221-232: `Connect`, `Tls`, `Timeout`, `DeadlineExceeded`, `Other` | M |
| `CallError<E>` | RT call.rs:247-261 `Transport(TransportError) \| Api { error: ApiError<E>, meta: Box<ResponseMeta> }`, `#[non_exhaustive]`. `request_meta()` :266. | M |
| `BuildError` | RT error.rs:137-185, `#[non_exhaustive]`. Includes `OriginPolicyNotPermitted { definition, requested }`. | M |

###### 5. Client, calls, credentials

| Spec expectation | Code | Status |
| --- | --- | --- |
| Managed client | `Client<A: Api>` RT client.rs:129. `Client::new(ApiDefinition)` :204 (infallible). Settings are validated on first send or by `build()` :390. `ApiDefinition` :61-67 holds `base_url`, `auth`, `secret_names`, `request_id_header`, `cross_origin`. `DEFAULT_TIMEOUT` = 30 s at :50. | M |
| Credential order: per call, client, environment, anonymous | RT credentials.rs:295-314: `CallAuth::Anonymous` → anonymous, `PerCall` → that credential, `Inherit` → client credential, otherwise `resolve_env()` (:316), otherwise anonymous. | M |
| Typed `send` | `Client::send<R: EndpointSpec<Api=A>>(&self, R) -> Call<'_, A, R>` client.rs:573-578. No `CallOptions` argument. | M |
| `.full()` / `FullCall` | `Call::full(self) -> FullCall<'a,A,R>` RT call.rs:98-100. `FullCall` :52-54 forwards `anonymous`, `credential`, `deadline`, `build` (:134-168). Its await output is `Result<Response<R::Success,R::Failure>, TransportError>` :188-200. | M |
| `send_raw(request, &CallOptions)` | client.rs:626-631, returns `impl Future<Output = Result<RawResponse, TransportError>>`. It counts the deadline at :655-657. | M |
| `Call<'a, A, R>` and its private fields | RT call.rs:39-43 `{ client: &'a Client<A>, request: R, options: CallOptions }`, all private. The constructor `Call::new` is `pub(crate)` :57. There are no public accessors. Code inside the runtime crate (a `pager` module) can read the fields; generated code cannot. | M |
| `Call::map_request` | call.rs:90-93 `pub fn map_request(mut self, f: impl FnOnce(R) -> R) -> Self` | M |
| Per-call `.credential` / `.anonymous` / `.deadline(&Deadline)` | call.rs:68-85 | M |
| `.build()` checks | call.rs:125-131 (requires `R: Clone`) → `Client::check_arguments` client.rs:669-683 → `Inner::prepare` :849-893. Checked: encoding, header and body content type, credential kind and placement. URL errors, origin refusal, client settings and deadline are left to the send (call.rs:119-124). | M |
| Send core | `send_unboxed` client.rs:582-605 (`pub(crate)`) → `Inner::execute` :733-838. `completed()` counts the deadline (:1219-1224). | M |
| Generated API struct surface | The `api_client!` macro (RT api_client.rs:46+) forwards settings plus `send` (:165), `send_raw` (:173), `check_link` (:190). | M |

###### 6. Origin policy and links

| Spec expectation | Code | Status |
| --- | --- | --- |
| `Client::check_link::<R>(&Url)` | client.rs:699-713: `pub fn check_link<R>(&self, url: &Url) -> Result<(), TransportError> where R: EndpointSpec<Api = A>`. It does no network work. A refusal is `CrossOriginRejected { target }` with redacted target, Anonymous auth and `Duration::ZERO`. Invalid client settings give `InvalidClient`. `Url` is re-exported at RT lib.rs:87. | M |
| Checked absolute-URL request path | `Inner::assemble` client.rs:969-990: if `is_absolute_url(&path)` (:1199-1203, case-insensitive `http://`/`https://` prefix), then `Url::parse(&path)` is used instead of joining onto the base. The declared query is still appended and the fragment cleared (:988-990). `prepare` then checks the origin with `origins.link(&assembled.url)` (:856-860) and refuses with `CrossOriginRejected`. Credentials are attached only if the decision says so (:870-876). Test: `runtime/tests/l1/redirects.rs:328-350`. To use it, a request's `into_parts()` must return `path` = absolute URL. Generated request types build `path` from the template, so the pager needs a wrapper `EndpointSpec` (same `Api`, `Success`, `Failure`, `Decoder`, `ENDPOINT_ID`) or an in-crate seam. No such wrapper exists today. | M (mechanism present; the pager integration type is new work) |
| `deny` and same-origin links | Module table at RT origin.rs:10-24: "`deny` follows no redirect at all ... but a same-origin link is still allowed so that paging keeps working." `redirect()` refuses everything under Deny (:161-166). `link()` = `decide()` (:171-189), where the same origin is followed with credentials under every policy. Test at redirects.rs:345-349. | M (spec default 17 is satisfied) |
| `MAX_REDIRECTS` | client.rs:54 `pub const MAX_REDIRECTS: usize = 10;`. Enforced at :943-944 → `RedirectRefusal::HopLimit`. | M |
| `danger_allow_any` | Runtime: `OriginPolicy::danger_allow_any()` RT origin.rs:81, variant `DangerAllowAny` :53. Define side: `CrossOrigin::danger_allow_any()` DEF cross_origin.rs:76, name "danger_allow_any" :92. Compile-fail fixture: `runtime/tests/fixtures/compile_fail/allow_any_needs_the_danger_name.rs`. | M. The spec's D28/R18 text says `allow_any`. |
| Scheme, userinfo, origin rules | origin.rs:180-183: a non-http(s) scheme or userinfo is refused. An origin is (scheme, host, effective port) (:5-8). | M |
| Relative-link resolver | **No public resolver.** The runtime resolves only redirect `Location` headers, internally (`current.url.join(location)` client.rs:939). There is no public helper to resolve a relative reference against a response URL, and the envelope exposes only the redacted final URL (meta.rs:57). | **D** (the spec says "reuse the runtime's resolver") |
| `Link` header sanitizing | Not sanitized; RE implementation-log.md:1310-1312, 3664 assign the decision to spec 3. | open, assigned to pagination |

###### 7. Shared deadline

| Spec expectation | Code | Status |
| --- | --- | --- |
| `Deadline` type, monotonic clock | RT deadline.rs:20-79. Built with `Deadline::after(Duration)`. Methods: `expires_at`, `remaining`, `is_expired`, `elapsed`, `completed_requests`. Uses `tokio::time::Instant` (:16), which is monotonic and pausable in tests. Clones share an `Arc`. | M |
| Per-request time = min(remaining, per-request timeout) | `Expiration::start` deadline.rs:104-123 picks the earlier of `start+timeout` and the deadline. Used at client.rs:743. An expired deadline sends nothing (client.rs:735-741). | M |
| What it counts | deadline.rs:67-74: logical sends whose outcome was classified, HTTP error statuses included. Redirect hops, pre-head failures and dropped sends are not counted. Recorded by `completed()` client.rs:1219-1224 and by `send_raw` :655-657. `DeadlineExceeded` reports `completed_requests` and `deadline_elapsed`. | M (refresh note correct; pager page counts are separate) |
| Session close grace / cleanup-deadline precedence | **No spec-2 ruling.** RE spec.md:174-175, 1456-1457, 1684-1686 say the grace after expiry "is spec 3's to decide". RE implementation-log.md:3609-3624 says "The session-close grace after expiry belongs to spec 3." No author ruling appears in RE review-1..3 (the only mention is review-1.md:225, which defers it). The parent still has S10 open (MR:1730). | M (default 21, 5 s grace, stands; it is unratified **Proposed**) |
| Stream bounds | codec.rs:71-77, 104-116: idle = per-request timeout, and the deadline bounds the whole stream after `into_streaming()`. | info |

###### 8. Generator: endpoint methods and `Paginated`

| Spec expectation | Code | Status |
| --- | --- | --- |
| `codegen/endpoint_methods` emitter | `GEN codegen/endpoint_methods/{mod.rs,names.rs,docs.rs,tests.rs}`. Entry `generate_endpoint_methods(api: &RestApi) -> TokenStream` mod.rs:59, called from GEN output/assemble/api.rs:70. | M |
| Local `{Endpoint}Call` wrapping `schematic_runtime::Call<'a, Tag, Request>` | mod.rs:132, 149-151: `pub struct #call<'a>(schematic_runtime::Call<'a, #tag, #request>);` with a private tuple field. Contents: setters via `map_request` (:134-145), `anonymous`/`credential`/`deadline` (:157-169), `full()` (:173), `build()` (:184), and `IntoFuture` (:189-201). The endpoint method is `#call(self.0.send(...))` (:222-227). Count: 547 call types in `schematic/schema/src` (openai 265). | M |
| Reserved names incl. `pages`, `fetch_page`, `collect_all` | GEN codegen/endpoint_methods/names.rs:39 `PLANNED_CALL_METHODS = ["pages", "fetch_page", "collect_all"]`. Also `SHARED_CALL_METHODS` :35, `CALL_TYPE_TRAIT_METHODS` :43, `API_CLIENT_METHODS` :47-67, `API_STRUCT_METHODS` :72. A reserved setter gets the suffix `_param` (:332-335); a reserved endpoint method gets `_endpoint` (:315-318). `is_call_type_method` is at :385-389. New pager names (`page_size`, `after`, `with_session`, ...) are **not** reserved yet. | M; extend the list |
| `Paginated` marker and its generators | GEN codegen/traits.rs:35-56 `generate_paginated_trait()` emits `pub trait Paginated {}`. traits.rs:86-102 `generate_paginated_impl` is emitted only when `endpoint.params.pagination.is_some()` (:88-91). Re-exported at codegen/mod.rs:51. | M |
| Imports in `output/assemble/*.rs` | shared.rs:6, 16, 29 (trait into `shared.rs`). api.rs:16, 67-69 (impl), `has_paginated` :84-90, `shared_import` :92-98 (used at :106/124, 157/176, 260/284). GEN validation.rs:88 lists "Paginated" in `IMPORTED_TYPE_NAMES`. | M |
| Generated `shared.rs` | `schematic/schema/src/shared.rs:173` `pub trait Paginated {}`. There are 35 `impl Paginated` (github 9, gitlab 9, bitbucket 9, gitea 8). | M |

###### 9. Generation-integrity deliverables

| Spec expectation | Code | Status |
| --- | --- | --- |
| Partial generation | GEN output/mod.rs:41-43, `prepare_rust_output` :236 (doc :215-235). ABI and shared-file byte checks at output/inventory.rs:440-480. Tests in `schematic/gen/tests/l1/partial_generation.rs`. | M |
| Isolated fresh-output compile (D49 item 1, GI spec.md:103, :170) | Recipe `just test-fresh-output` (schematic/justfile:80-82), run by `just test` (:71-74). Harness `schematic/gen/tests/common/fresh_output.rs` (temp crate outside the checkout, `--offline --locked`). Tests in `schematic/gen/tests/l1/fresh_output_compile.rs`: `slow_fixture_apis_compile_and_send_offline` :1033, `slow_real_apis_compile_offline` :1063, `slow_generated_types_reject_misuse_at_compile_time` :1293, ABI mismatch :1347, negative control :1486, `slow_query_wire_matches_pre_split_baseline` :1706. | M |
| Query-parameter separation | `RequestParts.query` RT endpoint.rs:83-88. Generator side in GEN codegen/request_structs/shared.rs:121-131 and into_parts.rs:18-78. Runtime encodes once: client.rs:990, 1097, 1112. | M |
| Diagnostics format | GEN diagnostics.rs: `ValidationDiagnostic` :182-189 `{ severity, api: Vec<String>, endpoint: Option<String>, feature: Feature, output: Output, message }`. Constructors `new` :193, `for_group` :212. Display :239-252 is `severity={} api={} endpoint={} feature={} output={}: {}` (api joined with ","; endpoint is `<none>` when absent). Example :28-31. `Feature` (:99) uses kebab ids (:145-167). `Output` is rust/openapi/postman (:67). | M. The spec P8.3 default `<api>.<endpoint_id>: <rule>: <detail>` must be aligned to this format with new `Feature` ids. |

###### 10. `RestApi` shape and literals

| Spec expectation | Code | Status |
| --- | --- | --- |
| Final `RestApi` fields | DEF types.rs:124-235, `#[derive(Debug, Clone, PartialEq, Eq)]`, **no `Default`, no constructor**. Fields: `name` :131, `description` :133, `base_url` :137, `docs_url` :139, `auth` :141, `auth_policy` :148, `env_auth` :156, `env_username` :161, `headers` :168, `endpoints` :170, `module_path` :176, `request_suffix` :181, `version` :186, `env_mapping` :224, `cross_origin: CrossOrigin` :227, `request_id_header: Option<String>` :230, `secret_names: Vec<String>` :234. | M |
| "18 RestApi literals" | 18 production literals in `schematic/definitions/src` (lmstudio, gitlab, emqx ×2, artificial_analysis ×2, unfolded_circle/core_rest, elevenlabs, gitea, github, eversolo, anthropic, huggingface, bitbucket, ollama ×2, openai (mod.rs:5067), samsung_smart_tv), plus 3 test literals in definitions/src/registry/mod.rs:434, 491, 533. Because `RestApi` has no `Default`, every other literal also needs the new fields. These include DEF openapi/import/builder.rs:362 (importer), DEF request.rs:716, DEF types.rs tests (:567-773), DEF openapi/export tests, and many in GEN (validation.rs, output/mod.rs, test_utils.rs, codegen tests, tests/l1/*). A raw grep for `RestApi {` finds about 122 hits across schematic crates (some are not literals). | D (scope larger than 18) |
| `cross_origin`, `request_id_header`, `secret_names` values | All 21 definition literals use `CrossOrigin::same_origin()` and `secret_names: vec![]`. `request_id_header` is `None` except github/mod.rs:517 `Some("x-github-request-id")` and openai/mod.rs:5086 `Some("x-request-id")`. | M |
| Importer definition emitter | GEN definition_gen.rs `rest_api_tokens` :117-160. The literal is built at :141-159 and emits every field (`auth_policy: None` :147, `env_mapping: None` :155, `cross_origin` :156 via `cross_origin_tokens` :411-421, `request_id_header` :157, `secret_names` :158). `endpoint_params_tokens` :308-324 always emits `pagination: None, response_pagination: None`. GEN import_pipeline.rs `run_import` :68 applies only `request_id_header` (:100) and `exclude_paths` (:103-121). | M |
| Generic overlay hook (R35) | **Absent.** Nothing in GEN/DEF/definitions. The `import-openai` recipe is at schematic/justfile:201-213. | A (new work, as expected by R35) |

###### 11. Legacy pagination

| Item | Defined | Readers |
| --- | --- | --- |
| `PaginationStyle` | DEF pagination.rs:35-60 (`#[non_exhaustive]`: `PageNumber` :39, `OffsetLimit` :47, `Cursor` :55). Constructors `github` :72, `gitlab` :90, `bitbucket` :103, `gitea` :121, `cursor` :139, `offset_limit` :156. `to_query_params` :171. | Production reader: GEN codegen/traits.rs:88-91 and output/assemble/api.rs:88 only (both drive the `Paginated` marker). |
| `PaginationResponse` | DEF pagination.rs:276-291 (`BodyField`, `LinkHeader`, `TotalCount`) | No reader. No definition calls `with_response_pagination`. |
| `EndpointParams::with_pagination` | DEF params/mod.rs:130-134 (adds query params and sets `pagination`). Also `has_pagination` :204, `with_response_pagination` :227, `has_response_pagination` :235. | 35 endpoints: github 9, gitlab 9, bitbucket 9 (via `bitbucket_pagination()` bitbucket/mod.rs:105-106), gitea 8. |
| `EndpointParams.pagination` / `.response_pagination` | DEF params/mod.rs:99, 105 (struct :86-106, derives `Default`). Re-exports: DEF lib.rs:167, prelude.rs:54. | Emitter writes `None` (definition_gen.rs:320-321). OpenAI literals spell `pagination: None, response_pagination: None` explicitly (e.g. openai/mod.rs:115-116). |
| `Endpoint` fields (R3 frozen test) | DEF types.rs:446-490: `id`, `method`, `path`, `description`, `request`, `response`, `headers`, `params`, `oauth_scopes` | n/a |

###### 12. `validate_api`

| Spec expectation | Code | Status |
| --- | --- | --- |
| `validate_pagination` wired into "the existing `validate_api` path in `schematic-gen/src/validation.rs`" (spec R2 :117, Design :214) | **No `validate_api` exists** in schematic/gen, define or definitions; the only one is in the spike prototype (`spikes/pagination/prototype/src/lib.rs:1605`). The actual functions in GEN validation.rs: `validate_rust_client(apis: &[&RestApi]) -> Vec<ValidationDiagnostic>` :138-152 (runs `check_routes`, `check_rust_api` :333, `check_endpoint_method_names` :407-427, and `check_group_type_names`), `validate_openapi` :160, `validate_openapi_registry` :173, `validate_postman` :215, and `ensure_no_errors` :228-239, which turns errors into `GeneratorError::Validation { output, diagnostics }`. Callers: GEN pipeline.rs:428, 637; output/mod.rs:410; re-exported at lib.rs:110. | **D**. Wire into `validate_rust_client` (and the per-output validators as appropriate), emitting `ValidationDiagnostic`. |

##### Review rulings affecting pagination

Runtime envelope, review 1 (`human_resolved: true`; "Human rulings" section at RE review-1.md:231-311):

- **Item 1** (:241-244): endpoint call types are the primary surface. Pagination builds on them; the generic `pager(request)` entry default is superseded. Implemented.
- **Item 2** (:260-264): accepted as built: the type-level `Decode::decode(LiveBody)`, the owned `LiveBody`, `ErrorBody`, and `decode_hooked` gated by `SupportsHooks`. Any `SupportsHooks` codec must ship a hook test.
- **Item 4** (:276-280): `Client::check_link::<R>(&Url)` replaces the type-free form. An absolute `http(s)://` path is sent after the same origin check. "the pager built on endpoint methods is the primary caller". The pager follows absolute links "through the checked absolute-URL request path".
- **Item 5** (:284-288): ElevenLabs `ListVoices` keeps optional `page_size` (integer) and `page_token` (string); "A later pagination strategy may adopt `page_token`"; "authorizes no other endpoint change".
  - Code: `schematic/definitions/src/elevenlabs/endpoints/voices.rs:17-23` (query name `page_token`), and the response field `next_page_token` at `elevenlabs/types/voices.rs:131`.
  - The reviewer noted the input name may really be `next_page_token`; this is unverified (RE review-2.md:105-107, review-3.md:84-86).
- **Item 6** (:292-300): keep both partial-generation checks (the ABI marker and the shared-file byte comparison).

Runtime envelope, adopted defaults recorded in the implementation log. These are not author rulings, but they are the landed contract:

- `deny` passes same-origin links (PD2; :592, :3288).
- `MAX_REDIRECTS = 10` (PD3; :1859, :3289).
- Tighten-only overrides (PD1; D56/D57 item 6; :1832-1835).
- Deadline counting rules (:3609-3624; arithmetic from D56-3).
- `RequestMeta.page` is never set; "The pagination spec must add its input" (:3625-3631, :3939).
- `InvalidArgument::new` is the page-size path (:3637-3644).
- `RedirectFailed` categories, and "Spec 3 maps `RedirectFailed` as it maps `TransportError` (D57 item 7)" (:3655-3663).
- `Link` headers are not sanitized; spec 3 decides (:1310-1312, :3664).
- Session-close grace "belongs to spec 3" (:3609-3624). No author ruling exists (no hits for grace/S10/P6 in RE reviews), so default 21 (5 s) stands as Proposed.

Endpoint methods (EM review-1.md, `human_resolved: true`). The rulings are recorded as `**DECISION:**` lines. The "Human rulings" section (:103-115) still reads "(pending)". Reviews 2 and 3 treat the DECISION lines as authority (review-2.md:30, review-3.md:34-42). Authorship cannot be verified from git, because the file's only commit is the cycle close.

- **Item 1** (:18): `CallError::Api { error, meta: Box<ResponseMeta> }`. Matches RT call.rs:254-260.
- **Item 2** (:20-27): keep generic `FullCall`; `.full()` is the final endpoint-specific step ("`.full().per_page(10)` is not" supported). Pagination docs must use this ordering consistently. Code: the `FullCall` doc at RT call.rs:45-51, and the compile-fail fixture `request_change_after_full.rs`.
- Build cost: chained setters without per-API features (EM spec.md:51 "DECISION: A"; review-1.md:82). Measured cost: whole command +4.5%, unit +34% (EM implementation-log.md:1260-1267).
- Reserved names `pages`, `fetch_page`, `collect_all` (EM implementation-log.md:612-620, departure 6 :1255; EM spec.md:180 also reserves "any method later specs add").
- `Call::map_request` is public (departure 4, :519, :1253).
- The API struct field is private (`pub struct GitHub(Client<GitHubApi>)`, EM implementation-log.md:335, :651).
- **Review 3** (`ready: true`, no findings):
  - `Call::build` shares first-hop preparation with the sends (:32, :54-69).
  - Pagination handoff (:107-111): "pager methods extend existing local calls through thin forwards into the shared runtime ... accepted full-response ordering, ABI 3, effective call settings, shared argument preparation, envelope preservation, private call-state integration, collision reservations, and deadline request-count semantics". "No contested requirement or new scope was introduced."
  - Observation (:103-105): the 40-test schema suite does not run in CI; scheduling it is the author's choice.

Generation integrity:

- The D49 item 1 isolated compile check is delivered (GI spec.md:103, :170; GI review-3.md:100).
- Author rulings at GI review-1.md:89-147:
  - An endpoint is `method + path`. GitHub `GetGitTree`/`GetGitTreeRecursive` and the Bitbucket pair are merged (:99, :118-129). This affects the "Gitea tree" row's assumptions only indirectly; the GitHub tree pair is now one endpoint.
  - Deletion of the stale tree is authorized for spec 1 (:144-147).

##### Discrepancies with spec refresh notes

1. **`validate_api` does not exist** (spec R2 :117, Design :214). Use `validate_rust_client` and the per-output validators in GEN validation.rs, with `ValidationDiagnostic` and new `Feature` ids. The refresh notes do not mention this.
2. **No runtime relative-URL resolver, and no unredacted final URL in the envelope.** The spec (:106, R18) says "reuse the runtime's resolver"; the refresh notes are silent.
   - The only resolution is internal `Url::join` for redirects (client.rs:939).
   - `ResponseMeta.request.url` is a `RedactedUrl` (meta.rs:57), with secret query values replaced.
   - The pager must either resolve against an internally retained final URL (a new in-crate seam) or use `url::Url::join` itself. Record this as an integration correction; it is not a missing prerequisite.
3. **The checked absolute-URL path exists only through `RequestParts.path`** (client.rs:980-984). Generated request types cannot carry an arbitrary URL, so the pager needs a wrapper `EndpointSpec` or a crate-internal send seam. The refresh notes describe the path as available, which is true at the runtime level.
4. **RestApi literal count.** The spec and refresh notes say 18. Code has 18 production literals plus 3 registry test literals. `RestApi` has no `Default`, so the new fields also touch the importer builder (DEF openapi/import/builder.rs:362), the emitter (GEN definition_gen.rs:141-159), and many test literals in DEF and GEN (about 122 `RestApi {` grep hits in total).
5. **Dev-dependencies already present.** `trybuild = "1.0"` and `wiremock = "0.6"` are already dev-dependencies of schematic-runtime (`schematic/runtime/Cargo.toml:31-32`), and both are listed in `docs/dependencies.md` (:21, :1640, :1646) and `schematic/docs/dependencies.md` (:281, :311, :411). The spec's pre-authorization to "add" them (:313) is moot for the runtime. The existing trybuild harness clears `RUSTC_WRAPPER` for portability (`runtime/tests/l1/compile_fail.rs:12-15`) and runs through `just test-compile-fail` (justfile:90-91), which addresses the spec's kache-path risk.
6. **`RequestMeta.page` has no input.** The refresh note says "ordinary sends leave it unset", which is correct. But the runtime always leaves it `None` and offers no way to request it, so the pager must set it (in-crate) after each send (RE implementation-log.md:3625-3631).
7. **Origin policy naming.** D28/R18 say `allow_any`; code uses `danger_allow_any` (runtime and define). The refresh notes already state this. The R18 test-matrix wording in the spec is stale.
8. **New pager names are not reserved.** `PLANNED_CALL_METHODS` (names.rs:39) holds only `pages`, `fetch_page`, `collect_all`. Knob names (`page_size`, `page`, `offset`, `after`, `before`, `prev`, `keep_alive`, `partitioned`, `with_session`, `total`, ...) must be added under the same policy. The refresh notes already require this.
9. **ElevenLabs input name.** The definition sends `page_token` (voices.rs:19). The spec table says research names it `next_page_token`; "confirm the sent name before adopting" remains open and unverified.
10. **Likely ABI bump** once generated code implements runtime pager traits (RT lib.rs:96-101). The refresh notes say ABI is 3 and do not mention a bump.
11. **`InvalidArgument` has no kinds or variants** (a struct with a private reason). This matches the refresh note "no dedicated page-size variant". The spec's "invalid-argument kinds" phrasing in plan 1.1 has no code counterpart.
12. **Generated `Call` wrapper uses tuple field `.0` (private)** (mod.rs:149-151). Forwards generated in the same module can use `self.0`. The runtime `Call` fields are private to generated code, so pager entry points must be runtime methods on `Call<'a, A, R>` (bounded by pager traits) or take the `Call` by value. This confirms the refresh note's "narrow integration seam".

Facts that match the refresh notes and need no correction: ABI 3, the modules, `EndpointSpec`, the codecs, `into_parts`/`into_result`, `RedirectFailed`, `CallError`, `.full()` ordering, `.build()` checks, `check_link::<R>`, `deny`, `MAX_REDIRECTS = 10`, the deadline semantics, the credential order, `send_raw(request, &CallOptions)`, `map_request`, the reserved names, the 547 call types, and the `RestApi` cross-origin, request-id and secret-name fields.

##### Authorization record

**No author confirmation of any spec-3 pre-authorization is recorded anywhere.**

Where the requirement for confirmation is recorded:

- **MR:2114** (D50 item 8, under "Decision D50 (confirmed by the author, 2026-10-11)", :2094), verbatim: "**Before each plan runs**, its spec needs a short pass to settle its planning-time details ... and the author must pre-authorize the steps that would otherwise pause an unattended run: regenerating `schematic-schema`; creating the `schematic-runtime` package; deleting the stale `schematic/gen/schematic` tree; editing dependent crates (sniff, biscuit-speaks, model-citizen, the five homelab crates); and, for spec 4, rewriting the 253 endpoint definitions." This is a process rule, not a grant.
- **MR:2612-2623** ("Preparation tasks needing author permission"): "The implementer cannot take them without the author's explicit approval." Listed tasks: regenerate `schematic-schema` (:2616); verify the U rows (:2618); fix EMQX `ListAuthUsers` (:2619); "any other destructive or wide-scope action" (:2623). None is annotated as granted.
- **D55, D56, D57 (MR:2217, :2241, :2274):** design rulings only. The only permission in them is D57 item 1 (:2280), "pre-authorized for spec 1".
- **PG spec.md:308:** "None is granted until the author confirms the list". Network verification (:314) and the `ListAuthUsers` fix (:312) are explicitly **PENDING**. Lines :310-317 list regeneration, dependent edits, definition edits, runtime module and dev-dependencies, network docs, `Paginated` removal, docs and skills, and the size measurement. :318 lists what is not authorized. This is agent-drafted request text.
- **PG plan.md:** frontmatter is `yolo: true`, `agent: codex/gpt-6.1-sol`. :38 says "Do not assume `yolo: true` grants those permissions." :53 tells Phase 1 to review actual authorization. These are agent statements.
- **PG implementation-log.md:** no authorization recorded.
- **Precedent (not applicable to spec 3):** RE implementation-log.md:575-582 records the agent's inference that the author's phase-by-phase instruction granted spec 2's pre-authorized scope. GI implementation-log.md:353-360 records "Pre-authorization items 1–8 still have no recorded author confirmation." The explicit author grants found anywhere are GI review-1.md:118, :144; RE review-1.md:284; and MR:2280, and none covers spec 3.

| Step | Recorded confirmation |
| --- | --- |
| Regenerate `schematic-schema` | Not recorded |
| Dependent-crate migrations | Not recorded |
| Runtime module and dev-dependency changes | Not recorded (dev-dependencies already present, so only the new module and docs are needed) |
| Definition authoring (18 APIs, `RestApi` fields, tables) | Not recorded |
| Docs, skills, READMEs | Not recorded |
| Network verification of U rows (optional) | Not recorded; explicitly PENDING |
| EMQX `ListAuthUsers` fix (optional) | Not recorded; explicitly PENDING, and conditional on the EMQX rows being verified |

Per plan.md:38, the two optional steps are skipped and logged. The other steps are required implementation scope, and their missing authorization must be recorded before proceeding. The only candidate source is the author's own run instruction, which does not appear in any pagination file.

##### Dependency availability

- Root `Cargo.toml` has **no `[workspace.dependencies]`** section. `schematic/schema` is excluded from the workspace (root Cargo.toml:81).
- **trybuild:** `schematic/runtime/Cargo.toml:31` (`trybuild = "1.0"`), used by `runtime/tests/l1/compile_fail.rs:16` with 10 fixtures in `runtime/tests/fixtures/compile_fail/`. It is not used by any other schematic crate.
- **wiremock 0.6** as a dev-dependency:
  - `schematic/define/Cargo.toml:27`
  - `schematic/oauth/Cargo.toml:21`
  - `schematic/runtime/Cargo.toml:32` (used in `runtime/tests/l1/{send,credential_resolution,client_settings,...}.rs`)
  - `schematic/schema/Cargo.toml:27`

  It is not a dependency of `schematic-gen` (only emitted into generated fixture crates, GEN cargo_gen.rs:46) or `schematic-definitions`.
- Other runtime dev-dependencies: `biscuit-test-harness`, `test-toolkit`, `rcgen`, `tokio-rustls`, `tokio` (with `test-util` for paused time) (runtime/Cargo.toml:22-30).
- `schematic/schema/Cargo.toml:16`: `schematic-runtime = { version = "0.1.0", path = "../runtime" }`.
- The area `just test` already covers schematic-runtime (justfile:71-74), along with `test-fresh-output` and `test-compile-fail`.

#### Coverage inventory: `2026-10-13-schematic-pagination` Phase 1 task 1.2

Recounted against the current tree of `feat/modern-config` on 2026-10-13 (HEAD `0cd6075e6`). Method: a throwaway crate (`/tmp/pagination-phase1/dump`, built `--offline`) calls the 18 `define_*_api()` constructors and dumps every endpoint (id, method, path, response descriptor, query params, `pagination`, `response_pagination`) to `endpoints.jsonl`. A Python pass (`analyze.py`) then resolves response struct fields from `schematic/definitions/src`, per module and excluding `#[cfg(test)]`. I spot-read the results against the sources: Bitbucket `GetSource`, ElevenLabs `ListVoices`, UC aliases, the EMQX `PaginationMeta`, and the OpenAI vector-store schemas in `specs/openai/openapi.yaml`.

"Root `Vec<T>`" means the response descriptor is literally `ApiResponse::Json` with `type_name == "Vec<T>"` (`json_vec_type`). This is the only form that R4 step 2 and default 22 recognize. A type alias to `Vec<T>` (UC) is not recognized.

##### Recount summary

| Figure | Spec / inventory (2026-10-09/11) | Current tree | Note |
|---|---|---|---|
| REST APIs (`RestApi` from `apis_by_module`) | 18 | **18** | anthropic, AA data, AA critpt, bitbucket, openai, elevenlabs, gitea, github, gitlab, huggingface, lmstudio, ollama native, ollama openai, emqx basic, emqx bearer, eversolo, samsung, UC core_rest |
| Endpoints (resolved, all APIs) | 547 | **547** | = generated request types 547 = call types 547 (`schema/src` `EndpointSpec` impls) |
| OpenAI endpoints | 265 | **265** | |
| Production `Endpoint {` literals in `definitions/src` | ~519 | **511** | 547 − 36 EMQX endpoints that basic and bearer share through helper fns. Excludes `#[cfg(test)]`, `registry/` and WebSocket literals; with tests counted, the raw grep gives 569 |
| Declared legacy pagination (`pagination: Some`) | 35 (Gitea 8, GitHub 9, GitLab 9, Bitbucket 9) | **35** (same split) | all `PageNumber`; **one Bitbucket declaration (`GetSource`) is now on a `Binary` endpoint**, see D1 |
| `response_pagination: Some` | 0 | **0** | |
| GET root `Vec<T>` endpoints | n/a | **40** | GitHub 8, GitLab 10, Gitea 9, HF 10, ElevenLabs 1, EMQX 1+1 |
| Non-GET root `Vec<T>` descriptors | n/a | **0** | UC `RestoreBackup`/`UploadResource` return *aliases* of `Vec` |
| Collection endpoints (inventory method) | ~157 | **156** (+ `GetSource` dual-shape = 157) | Bitbucket `ListDirectoryContents` (wrapper) was merged into `GetSource` (`Binary`) by `9e65a90b5` on 2026-10-12 |
| OpenAI list candidates | 61 (60 GET + 1 POST) | **61** (60 GET + `SearchVectorStore` POST) | S6 36 + item-id 2 + S7 9 + S8 11 + 3 opt-outs |
| Paginated after authoring (verified rows only) | — | **95**, or 94 if ListVoices stays blocked (D2) | GitHub 9, GitLab 9, Gitea 8, Bitbucket **8**, OpenAI 58, ElevenLabs 3 |
| Resolve as list under R4 (paginated + unpaginated) | — | **113** | 95 paginated, plus 13 root arrays with no default (HF 10, EL `ListModels`, EMQX `ListAuthUsers` ×2), plus 5 opt-outs (Gitea `GetTagReference`, GitLab `ListIssueParticipants`, OpenAI ×3) |
| Collections that resolve "not a list" (wrapper, no entry) | — | **43** | AA 6, LM Studio 1, Ollama 3, EMQX 13×2, ElevenLabs 3, Anthropic 1, HF discussions 1, Gitea trees 2 (113 + 43 = 156). See G1 |
| Request fields a strategy would adopt (R7) | — | **186** (+1 pending) | GitHub 18, GitLab 18, Gitea 16, Bitbucket 8 (`pagelen`), OpenAI 125 (after 47, limit 58, before 9, page 11), ElevenLabs 1 (`page_size`); `page_token` pending D2 |
| Plain pagination-looking fields kept (not adopted) | — | **10** | Bitbucket `page` ×8 on wrapper lists (NextLink has no page input); `GetSource` `page`/`pagelen` |

##### Per-API inventory

Columns: the **Origin** is the intended R4 origin. **Declared params** lists every query param the endpoint declares today; a bold param is one the strategy adopts (same name and location). `!` marks a required param. No adoptable param is required, so R2a needs no strategy default anywhere. **Evidence** uses the spec's labels.

###### GitHub (16 endpoints; 9 declared legacy; 8 GET `Vec<T>`)

API default: `PageNumber{page, 1-based}`, size `per_page` (max 100; server default 30 as docs only), end signal `Link` rel `next`.

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| GetGitTree | GET | opaque/wrapper `GitTreeResponse` | — | none | — | `recursive` | n/a | U (inventory: `truncated`, not page-paginated) | not a list (wrapper `tree`, no entry) |
| ListPullRequests | GET | root `Vec<T>` | `PullRequestSummary` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`**, `state`, `sort`, `direction` | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListPullRequestFiles | GET | root `Vec<T>` | `PullRequestFile` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`** | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssues | GET | root `Vec<T>` | `IssueSummary` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`**, `state`, `sort`, `direction` | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssueComments | GET | root `Vec<T>` | `IssueComment` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`** | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssueTimeline | GET | root `Vec<T>` | `TimelineEvent` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`** | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListTags | GET | root `Vec<T>` | `RepoTag` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`** | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListReleases | GET | root `Vec<T>` | `Release` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`** | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListWorkflowRuns | GET | wrapper at `workflow_runs` | `WorkflowRun` | table entry (wrapper) | default PageNumber + total from body `total_count`; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`**, `status`, `branch`, `event` | yes | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListOrgRepos | GET | root `Vec<T>` | `RepositoryInfo` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`**, `repo_type`, `sort`, `direction` | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |

Other GitHub endpoints (GetRepository, GetRepositoryContentRaw, GetIssue, GetTagReference [`GitRef`, not an array], GetBranchReference, GetAnnotatedTag) are not lists.

###### GitLab (19 endpoints; 9 declared legacy; 10 GET `Vec<T>`)

API default: `PageNumber{page}`, size `per_page` (max 100; server default 20), end signal header `X-Next-Page` present, total header `X-Total`.

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListRepositoryTree | GET | root `Vec<T>` | `TreeItem` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` + labeled alternate `keyset` = `NextLink{Link rel next}` + constants `pagination=keyset`, `order_by=id`, `sort=asc` (modeled, not generated) | **`page`**, **`per_page`**, `recursive`, `git_ref`, `path` | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09); keyset constants V-notes | — |
| ListGroupProjects | GET | root `Vec<T>` | `Project` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` + labeled alternate `keyset` = `NextLink{Link rel next}` + constants `pagination=keyset`, `order_by=id`, `sort=asc` (modeled, not generated) | **`page`**, **`per_page`**, `archived`, `visibility`, `order_by`, `sort`, `include_subgroups` | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09); keyset constants V-notes | — |
| ListMergeRequests | GET | root `Vec<T>` | `MergeRequest` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`**, `state` | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListMergeRequestCommits | GET | root `Vec<T>` | `Commit` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`** | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListMergeRequestChanges | GET | opaque/wrapper `MergeRequestChanges` | — | none | — | — | n/a | U | not a list (MR object with embedded `changes`) |
| ListIssues | GET | root `Vec<T>` | `Issue` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`**, `state` | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssueNotes | GET | root `Vec<T>` | `Note` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`** | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssueParticipants | GET | root `Vec<T>` | `User` | **table entry: opt-out** (required, else it inherits the default) | unpaginated | — | n/a | U (believed paginated) | unverified (R28); GET `Vec<User>` in an API with a default, so an opt-out entry is needed to keep it unpaginated |
| ListTags | GET | root `Vec<T>` | `Tag` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`** | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListReleases | GET | root `Vec<T>` | `Release` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`** | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListProjectPipelines | GET | root `Vec<T>` | `Pipeline` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`**, `status`, `source`, `git_ref`, `sha` | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |

Note: ListGroupProjects declares plain `order_by` and `sort` query params; the keyset alternate's constants `order_by=id`/`sort=asc` share those names (alternates are not generated, but R2 `CollidesWithEndpointParam`/constant-vs-param handling must allow or reject this explicitly). ListRepositoryTree declares neither.

###### Gitea (16 endpoints; 8 declared legacy; 9 GET `Vec<T>`)

API default: `PageNumber{page}`, size `limit` (server cap 50; default 30 recorded), end signal `Link` rel `next`, total header `X-Total-Count`.

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| GetGitTree | GET | opaque/wrapper `GitTreeResponse` | — | none | (would be override `per_page`, `Flag(truncated)`, total `total_count`) | — | n/a | U | unverified; definition declares no `page`/`per_page` params; wrapper `tree` without entry = not a list |
| GetGitTreeRecursive | GET | opaque/wrapper `GitTreeResponse` | — | none | (would be override `per_page`, `Flag(truncated)`, total `total_count`) | — | n/a | U | unverified; definition declares no `page`/`per_page` params; wrapper `tree` without entry = not a list |
| ListPullRequests | GET | root `Vec<T>` | `PullRequestSummary` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`**, `state`, `sort` | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListPullRequestFiles | GET | root `Vec<T>` | `PullRequestFile` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`** | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssues | GET | root `Vec<T>` | `IssueSummary` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`**, `state`, `issue_type` | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssueComments | GET | root `Vec<T>` | `IssueComment` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`** | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssueTimeline | GET | root `Vec<T>` | `TimelineEvent` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`** | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListTags | GET | root `Vec<T>` | `RepoTag` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`** | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListReleases | GET | root `Vec<T>` | `Release` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`**, `draft`, `pre_release` | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| GetTagReference | GET | root `Vec<T>` | `GitRef` | **table entry: opt-out** | unpaginated | — | n/a | U (believed unpaginated prefix match) | opt-out (default 14): prefix-match ref lookup, not a list |
| ListOrgRepos | GET | root `Vec<T>` | `RepositoryInfo` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`** | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |


###### Bitbucket (15 endpoints; 9 declared legacy; 0 GET `Vec<T>`)

API default: `NextLink` from body `next` (absolute URL), backward `previous`, size `pagelen` (default 10, max 100), total `size`. The default reaches no endpoint by inheritance (no root arrays); each wrapper list needs a table entry at `values`.

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| GetSource | GET | Binary | — | none | — | `page`, `pagelen` | n/a | V-def (Binary) | **not a list**: `ApiResponse::Binary` (dual shape: directory JSON or file bytes); legacy `page`/`pagelen` stay plain fields |
| ListPullRequests | GET | wrapper at `values` | `PullRequest` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`**, `state` | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListPullRequestComments | GET | wrapper at `values` | `PullRequestComment` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListIssues | GET | wrapper at `values` | `Issue` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListIssueComments | GET | wrapper at `values` | `IssueComment` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListIssueChanges | GET | wrapper at `values` | `IssueChange` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListTags | GET | wrapper at `values` | `Tag` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListDownloads | GET | wrapper at `values` | `Download` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListWorkspaceRepos | GET | wrapper at `values` | `Repository` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |


###### OpenAI (265 endpoints; 0 declared legacy; 0 GET `Vec<T>`; 61 list candidates)

API default: `Cursor{after <- last_id, has_more}`, size `limit`. Every list is a wrapper at `data`, so every one needs a table entry (none inherits by R4 step 2).

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListVoiceConsents | GET | wrapper at `data` | `VoiceConsentResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListBatches | GET | wrapper at `data` | `Batch` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListChatCompletions | GET | wrapper at `data` | `CreateChatCompletionResponse` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | `model`, `metadata`, **`after`**, **`limit`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| GetChatCompletionMessages | GET | wrapper at `data` | `serde_json::Value` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListContainers | GET | wrapper at `data` | `ContainerResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`**, `name` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListContainerFiles | GET | wrapper at `data` | `ContainerFileResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListConversationItems | GET | wrapper at `data` | `ConversationItem` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`**, `include` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListEvals | GET | wrapper at `data` | `Eval` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`**, `order`, `order_by` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| GetEvalRuns | GET | wrapper at `data` | `EvalRun` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`**, `order`, `status` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| GetEvalRunOutputItems | GET | wrapper at `data` | `EvalRunOutputItem` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`**, `status`, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListFiles | GET | wrapper at `data` | `OpenAIFile` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | `purpose`, **`limit`**, `order`, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListFineTuningCheckpointPermissions | GET | wrapper at `data` | `FineTuningCheckpointPermission` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | `project_id`, **`after`**, **`limit`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListPaginatedFineTuningJobs | GET | wrapper at `data` | `FineTuningJob` | table entry (override) | Cursor{after <- LastItem(`id`), has_more}; knobs `.after()`, `.page_size()` | **`after`**, **`limit`**, `metadata` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListFineTuningJobCheckpoints | GET | wrapper at `data` | `FineTuningJobCheckpoint` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListFineTuningEvents | GET | wrapper at `data` | `FineTuningJobEvent` | table entry (override) | Cursor{after <- LastItem(`id`), has_more}; knobs `.after()`, `.page_size()` | **`after`**, **`limit`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListModels | GET | wrapper at `data` | `Model` | table entry: opt-out | unpaginated | — | n/a | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | no continuation in reply (`object`, `data` only) |
| AdminApiKeysList | GET | wrapper at `data` | `AdminApiKey` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, `order`, **`limit`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListAuditLogs | GET | wrapper at `data` | `AuditLog` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | `effective_at`, `project_ids[]`, `event_types[]`, `actor_ids[]`, `actor_emails[]`, `resource_ids[]`, `tenant_only`, **`limit`**, **`after`**, **`before`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListOrganizationCertificates | GET | wrapper at `data` | `OrganizationCertificate` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageCosts | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `api_key_ids`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListGroups | GET | wrapper at `data` | `GroupResponse` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListGroupRoleAssignments | GET | wrapper at `data` | `AssignedRoleDetails` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListGroupUsers | GET | wrapper at `data` | `GroupUser` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListInvites | GET | wrapper at `data` | `Invite` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjects | GET | wrapper at `data` | `Project` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `include_archived` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectApiKeys | GET | wrapper at `data` | `ProjectApiKey` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `owner_project_access` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectCertificates | GET | wrapper at `data` | `OrganizationProjectCertificate` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectGroups | GET | wrapper at `data` | `ProjectGroup` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectRateLimits | GET | wrapper at `data` | `ProjectRateLimit` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, **`after`**, **`before`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectServiceAccounts | GET | wrapper at `data` | `ProjectServiceAccount` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectSpendAlerts | GET | wrapper at `data` | `ProjectSpendAlert` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectUsers | GET | wrapper at `data` | `ProjectUser` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListRoles | GET | wrapper at `data` | `Role` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListOrganizationSpendAlerts | GET | wrapper at `data` | `OrganizationSpendAlert` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageAudioSpeeches | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageAudioTranscriptions | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageCodeInterpreterSessions | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageCompletions | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `batch`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageEmbeddings | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageFileSearchCalls | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `vector_store_ids`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageImages | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `sources`, `sizes`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageModerations | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageVectorStores | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageWebSearchCalls | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `context_levels`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListUsers | GET | wrapper at `data` | `User` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `emails` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListUserRoleAssignments | GET | wrapper at `data` | `AssignedRoleDetails` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectGroupRoleAssignments | GET | wrapper at `data` | `AssignedRoleDetails` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectRoles | GET | wrapper at `data` | `Role` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectUserRoleAssignments | GET | wrapper at `data` | `AssignedRoleDetails` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListInputItems | GET | wrapper at `data` | `ItemResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`**, `include` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListVectorStores | GET | wrapper at `data` | `VectorStoreObject` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListFilesInVectorStoreBatch | GET | wrapper at `data` | `VectorStoreFileObject` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`**, `filter` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListVectorStoreFiles | GET | wrapper at `data` | `VectorStoreFileObject` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`**, `filter` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| RetrieveVectorStoreFileContent | GET | wrapper at `data` | `serde_json::Value` | table entry: opt-out | unpaginated | — | n/a | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | provider gap: reply has `has_more`+`next_page`, no documented request input |
| SearchVectorStore | POST | wrapper at `data` | `VectorStoreSearchResultItem` | table entry: opt-out | unpaginated | — | n/a | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | provider gap: reply has `has_more`+`next_page`, no documented request input |
| ListVideos | GET | wrapper at `data` | `VideoResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListSkills | GET | wrapper at `data` | `SkillResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListSkillVersions | GET | wrapper at `data` | `SkillVersionResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListThreadItemsMethod | GET | wrapper at `data` | `ThreadItem` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListThreadsMethod | GET | wrapper at `data` | `ThreadResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`**, `user` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| BetaListInputItems | GET | wrapper at `data` | `BetaItemResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`**, `include` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |

OpenAI non-GET endpoints with list-shaped replies (D43: not lists, no entry): CreateConversationItems (POST `ConversationItemList`), CreateEmbedding (POST `CreateEmbeddingResponse`), CreateFineTuningCheckpointPermission (POST `ListFineTuningCheckpointPermissionResponse`), CreateImageEdit (POST `ImagesResponse`), CreateImage (POST `ImagesResponse`), CreateImageVariation (POST `ImagesResponse`), ActivateOrganizationCertificates (POST `OrganizationCertificateActivationResponse`), DeactivateOrganizationCertificates (POST `OrganizationCertificateDeactivationResponse`), ActivateProjectCertificates (POST `OrganizationProjectCertificateActivationResponse`), DeactivateProjectCertificates (POST `OrganizationProjectCertificateDeactivationResponse`).

###### ElevenLabs (42 endpoints; 0 declared legacy; 1 GET `Vec<T>`)

No API default (three different strategies). If a default were ever added, ListModels (GET `Vec<ModelInfo>`) would need an opt-out.

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListVoices | GET | wrapper at `voices` | `VoiceResponseModel` | table entry (wrapper) — **blocked** | Cursor{next_page_token <- next_page_token, has_more}, total `total_count`; knobs `.after()`, `.page_size()`, `total()` | **`page_size`**, **`page_token`** | `page_size` yes; `page_token` **name mismatch** with research input `next_page_token` | V-notes (`schematic/docs/research/elevenlabs-api.md`, 2026-05-07) | see discrepancy D2: confirm sent name before authoring/adopting |
| ListSharedVoices | GET | wrapper at `voices` | `LibraryVoiceResponseModel` | table entry (wrapper) | PageNumber{page, 0-based, has_more}, size `page_size` (30/100); knobs `.page()`, `.page_size()` | — | n/a (nothing declared) | V-notes (`schematic/docs/research/elevenlabs-api.md`, 2026-05-07) | — |
| ListModels | GET | root `Vec<T>` | `ModelInfo` | none | — | — | n/a | V-def | unpaginated list (root array, no default) |
| GetHistory | GET | wrapper at `history` | `SpeechHistoryItemResponseModel` | table entry (wrapper) | Cursor{start_after_history_item_id <- last_history_item_id, has_more}, size `page_size` (100/1000); knobs `.after()`, `.page_size()` | — | n/a (nothing declared; inputs are new) | V-notes (`schematic/docs/research/elevenlabs-api.md`, 2026-05-07) | — |
| ListServiceAccounts | GET | wrapper at `service-accounts` | `ServiceAccountModel` | none | — | — | n/a | U | unpaginated (U); wrapper without entry = not a list |
| ListServiceAccountApiKeys | GET | wrapper at `api-keys` | `ApiKeyModel` | none | — | — | n/a | U | unpaginated (U); wrapper without entry = not a list |
| ListWebhooks | GET | wrapper at `webhooks` | `WebhookModel` | none | — | — | n/a | U | unpaginated (U); wrapper without entry = not a list |


###### Anthropic (4 endpoints) — U

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListModels | GET | wrapper at `data` | `ModelInfo` | none | (would be Cursor{after_id <- last_id; before_id <- first_id; has_more}, limit 20/1..1000) | — | n/a | wrapper V-def; params U | U: unverified, stays unpaginated (no params declared) |

###### EMQX basic (36 endpoints) — U

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListNodes | GET | wrapper at `data` | `NodeInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListListeners | GET | wrapper at `data` | `ListenerInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListMetrics | GET | wrapper at `data` | `MetricsInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListStats | GET | wrapper at `data` | `StatsInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListAlarms | GET | wrapper at `data` | `AlarmInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared); wrapper omits `meta` |
| ListClients | GET | wrapper at `data` | `ClientInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListSubscriptions | GET | wrapper at `data` | `SubscriptionInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListTopics | GET | wrapper at `data` | `TopicInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListRetained | GET | wrapper at `data` | `RetainedMessage` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListRules | GET | wrapper at `data` | `RuleInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared); wrapper omits `meta` |
| ListAuthenticators | GET | wrapper at `data` | `AuthenticatorInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListAuthUsers | GET | root `Vec<T>` | `AuthUser` | none (root array → unpaginated list) | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared); **definition gap**: root array declared, server returns `{data, meta}` (U) |
| ListAuthzSources | GET | wrapper at `sources` | `AuthzSourceInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListBanned | GET | wrapper at `data` | `BanInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |

###### EMQX bearer (38 endpoints = basic + Login/Logout) — U

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListNodes | GET | wrapper at `data` | `NodeInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListListeners | GET | wrapper at `data` | `ListenerInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListMetrics | GET | wrapper at `data` | `MetricsInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListStats | GET | wrapper at `data` | `StatsInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListAlarms | GET | wrapper at `data` | `AlarmInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared); wrapper omits `meta` |
| ListClients | GET | wrapper at `data` | `ClientInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListSubscriptions | GET | wrapper at `data` | `SubscriptionInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListTopics | GET | wrapper at `data` | `TopicInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListRetained | GET | wrapper at `data` | `RetainedMessage` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListRules | GET | wrapper at `data` | `RuleInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared); wrapper omits `meta` |
| ListAuthenticators | GET | wrapper at `data` | `AuthenticatorInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListAuthUsers | GET | root `Vec<T>` | `AuthUser` | none (root array → unpaginated list) | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared); **definition gap**: root array declared, server returns `{data, meta}` (U) |
| ListAuthzSources | GET | wrapper at `sources` | `AuthzSourceInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListBanned | GET | wrapper at `data` | `BanInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |

###### Hugging Face Hub (26 endpoints) — U

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListModels | GET | root `Vec<T>` | `ModelInfo` | none (root array → unpaginated list) | (would be NextLink{Link rel next, limit}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListModelFiles | GET | root `Vec<T>` | `RepoFile` | none (root array → unpaginated list) | (would be NextLink{Link rel next, limit}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListModelCommits | GET | root `Vec<T>` | `Commit` | none (root array → unpaginated list) | (would be PageNumber{p, 0-based}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListModelDiscussions | GET | wrapper at `discussions` | `Discussion` | none | (would be PageNumber{p, 0-based}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListDatasets | GET | root `Vec<T>` | `DatasetInfo` | none (root array → unpaginated list) | (would be NextLink{Link rel next, limit}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListDatasetFiles | GET | root `Vec<T>` | `RepoFile` | none (root array → unpaginated list) | (would be NextLink{Link rel next, limit}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListDatasetCommits | GET | root `Vec<T>` | `Commit` | none (root array → unpaginated list) | (would be PageNumber{p, 0-based}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListSpaces | GET | root `Vec<T>` | `SpaceInfo` | none (root array → unpaginated list) | (would be NextLink{Link rel next, limit}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListSpaceFiles | GET | root `Vec<T>` | `RepoFile` | none (root array → unpaginated list) | (would be NextLink{Link rel next, limit}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListUserRepos | GET | root `Vec<T>` | `RepoInfo` | none (root array → unpaginated list) | (unverified endpoint) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| GetUserCollections | GET | root `Vec<T>` | `Collection` | none (root array → unpaginated list) | (unverified endpoint) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |

###### Remaining APIs (no pagination, no default)

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| UnfoldedCircleCoreRest::RestoreBackup | PUT | alias `BackupRestoreReportItems` = `Vec<BackupRestoreReportItem>` | `BackupRestoreReportItem` | none | — | — | n/a | V-def | non-GET write returning an array (alias, not `Vec<T>` descriptor): D43 not a list |
| UnfoldedCircleCoreRest::UploadResource | POST | alias `ResourceItems` = `Vec<ResourceItem>` | `ResourceItem` | none | — | — | n/a | V-def | non-GET write returning an array (alias, not `Vec<T>` descriptor): D43 not a list |

No GET `Vec<T>` endpoint exists in these 8 APIs. CritPt, Eversolo (option-list wrappers only), and Samsung have no list endpoints.

##### Legacy pagination declarations

All 35 come from `EndpointParams::with_pagination` (`gitea/mod.rs` 8, `github/mod.rs` 9, `gitlab/endpoints/*` 9, and `bitbucket/mod.rs` through `bitbucket_pagination()` 9). Each one materializes exactly two optional `Integer` query params: the page param ("Page number (1-indexed, default: 1)") and the size param ("Items per page (default: D, max: M)"). None sets `response_pagination`. The declared defaults are GitHub 100/100, GitLab 100/100 (`gitlab()` = `github()`), Gitea 50/100 and Bitbucket 50/100. They disagree with the providers (30, 20, 30 with cap 50, and 10).

| API | Endpoint | Response | Legacy style | Materialized query params | Resolves as |
|---|---|---|---|---|---|
| Bitbucket | GetSource | `Binary` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | **not a list** (Binary); params stay plain |
| Bitbucket | ListPullRequests | `PaginatedResponse<PullRequest>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListPullRequestComments | `PaginatedResponse<PullRequestComment>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListIssues | `PaginatedResponse<Issue>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListIssueComments | `PaginatedResponse<IssueComment>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListIssueChanges | `PaginatedResponse<IssueChange>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListTags | `PaginatedResponse<Tag>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListDownloads | `PaginatedResponse<Download>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListWorkspaceRepos | `PaginatedResponse<Repository>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Gitea | ListPullRequests | `Vec<PullRequestSummary>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListPullRequestFiles | `Vec<PullRequestFile>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListIssues | `Vec<IssueSummary>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListIssueComments | `Vec<IssueComment>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListIssueTimeline | `Vec<TimelineEvent>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListTags | `Vec<RepoTag>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListReleases | `Vec<Release>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListOrgRepos | `Vec<RepositoryInfo>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListPullRequests | `Vec<PullRequestSummary>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListPullRequestFiles | `Vec<PullRequestFile>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListIssues | `Vec<IssueSummary>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListIssueComments | `Vec<IssueComment>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListIssueTimeline | `Vec<TimelineEvent>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListTags | `Vec<RepoTag>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListReleases | `Vec<Release>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListWorkflowRuns | `WorkflowRunsResponse` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | wrapper list (table entry) |
| GitHub | ListOrgRepos | `Vec<RepositoryInfo>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListRepositoryTree | `Vec<TreeItem>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListGroupProjects | `Vec<Project>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListMergeRequests | `Vec<MergeRequest>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListMergeRequestCommits | `Vec<Commit>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListIssues | `Vec<Issue>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListIssueNotes | `Vec<Note>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListTags | `Vec<Tag>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListReleases | `Vec<Release>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListProjectPipelines | `Vec<Pipeline>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
##### OpenAI group reconciliation

Classification comes from the current reply struct fields plus the declared query params. It matches the inventory's group sizes exactly.

| Group | Count | Members (current definitions) | Request inputs declared | Reply signal |
|---|---|---|---|---|
| S6 (API default `after <- last_id`) | 36 | ListVoiceConsents, ListBatches, ListChatCompletions, GetChatCompletionMessages, ListContainers, ListContainerFiles, ListConversationItems, ListEvals, GetEvalRuns, GetEvalRunOutputItems, ListFiles, ListFineTuningCheckpointPermissions, ListFineTuningJobCheckpoints, AdminApiKeysList, ListAuditLogs, ListOrganizationCertificates, ListInvites, ListProjects, ListProjectApiKeys, ListProjectCertificates, ListProjectRateLimits, ListProjectServiceAccounts, ListProjectSpendAlerts, ListProjectUsers, ListOrganizationSpendAlerts, ListUsers, ListInputItems, ListVectorStores, ListFilesInVectorStoreBatch, ListVectorStoreFiles, ListVideos, ListSkills, ListSkillVersions, ListThreadItemsMethod, ListThreadsMethod, BetaListInputItems | `after`, `limit` on all 36 | `has_more`, `first_id`, `last_id` |
| S6 item-id variant (`after <- LastItem(id)`) | 2 | ListFineTuningEvents, ListPaginatedFineTuningJobs | `after`, `limit` | `has_more` only (no `first_id`/`last_id` in the struct): confirmed |
| S7 (`after <- next`) | 9 | ListGroups, ListGroupRoleAssignments, ListGroupUsers, ListProjectGroups, ListRoles, ListUserRoleAssignments, ListProjectGroupRoleAssignments, ListProjectRoles, ListProjectUserRoleAssignments | `limit`, `after`, `order` | `has_more`, `next` (vendored spec: "Cursor to fetch the next page") |
| S8 (`page <- next_page`) | 11 | UsageCosts, UsageAudioSpeeches, UsageAudioTranscriptions, UsageCodeInterpreterSessions, UsageCompletions, UsageEmbeddings, UsageFileSearchCalls, UsageImages, UsageModerations, UsageVectorStores, UsageWebSearchCalls | `page` and `limit` on **all 11**; `start_time` required (not pager-managed) | `has_more`, `next_page` |
| Opt-out: no continuation | 1 | ListModels | none | `object`, `data` only: confirmed |
| Opt-out: provider gap | 2 | RetrieveVectorStoreFileContent (GET), SearchVectorStore (POST) | none; the vendored spec has only path params, and `VectorStoreSearchRequest` has `query`, `rewrite_query`, `max_num_results`, `filters`, `ranking_options` | `has_more`, `next_page`: confirmed against `specs/openai/openapi.yaml` |

**`before` declared (9; each needs a backward leg `before <- first_id`):** ListAuditLogs, ListProjectRateLimits, ListProjectSpendAlerts, ListOrganizationSpendAlerts, ListVectorStores, ListFilesInVectorStoreBatch, ListVectorStoreFiles, ListThreadItemsMethod, ListThreadsMethod. All 9 are S6 and reply with `first_id`.

**Non-GET list-shaped replies (D43: not lists, no entry):** CreateConversationItems (POST `ConversationItemList`, which has `has_more`/`first_id`/`last_id`), CreateFineTuningCheckpointPermission (POST, list wrapper), Activate/DeactivateOrganizationCertificates and Activate/DeactivateProjectCertificates (POST), CreateEmbedding, CreateImage/CreateImageEdit/CreateImageVariation.

**Inventory rows that changed or were wrong (none blocks a group assignment):**
- UsageCosts and UsageImages **do** declare `limit`. The inventory said "UsageCosts and UsageImages declare no `limit`"; UsageCosts' `limit` has been present since the 2026-07-31 import (`61b6cc7aa`). All 11 S8 endpoints can therefore adopt `limit` as their size knob.
- AdminApiKeysList declares `after`, `order` **and `limit`**; the inventory listed only `after`, `order`.
- ListAuditLogs also declares array filters (`project_ids[]`, `event_types[]`, ...), UsageFileSearchCalls declares `vector_store_ids`, and UsageWebSearchCalls declares `context_levels`. None of them is pager-related.
- Wording only, not a contradiction: the `after` descriptions of ListGroups, ListGroupUsers and ListProjectGroups say "ID of the last group/user", but their reply schema documents `next` as "Cursor to fetch the next page". S7 (`after <- next`) agrees with the reply schema, so they stay S7.

##### Discrepancies and gaps

- **D1, Bitbucket `GetSource` (was `ListDirectoryContents`).** Commit `9e65a90b5` (2026-10-12) merged the directory listing into `GetSource` with `response: ApiResponse::Binary`, because one route serves both a JSON directory page and raw file bytes. The legacy declaration (page/pagelen) is still on it. Under R4 it is **not a list**: it is opaque, and a wrapper entry cannot name `values` on a `Binary` body. Bitbucket therefore has **8** wrapper lists, not the 9 in the authoring table. Its `page`/`pagelen` fields are kept, and sniff (`sniff/lib/src/remote/bitbucket.rs:177-179`) keeps hand-paging it. Coverage counts should say 8 + 1 declared non-list.
- **D2, ElevenLabs `ListVoices` input name.** The definition declares `page_token` (string) and `page_size` (`deadcabd9`, 2026-10-12). The research note `schematic/docs/research/elevenlabs-api.md` (2026-05-07, lines 489-490) names the request input **`next_page_token`**. `biscuit-speaks` sends `page_token`, but only a wiremock test checks it (`biscuit-speaks/lib/tests/elevenlabs_http.rs:150`), which is not provider evidence. Under the table's contradiction rule, the cursor leg stays unpaginated until the sent name is confirmed. If `next_page_token` is right, the `page_token` param is a definition bug, and fixing it is an endpoint-literal change that R3 does not list as an exception. `page_size` matches and can be adopted.
- **D3, GitLab `ListIssueParticipants`.** It is a GET `Vec<User>` in an API that gets a default, so R4 makes it **inherit the default unless it has an opt-out entry**. The authoring table calls it "believed paginated, unverified" and R28 requires it to stay unpaginated, so it needs an opt-out entry (a second GitLab entry alongside default 14's enumeration).
- **D4, Gitea `GetTagReference`.** This is the only Gitea GET `Vec<T>` needing an opt-out, as the spec expects. GitHub's `GetTagReference` returns a single `GitRef`, so GitHub needs no opt-out. Every GitHub GET `Vec<T>` (8) is a real list.
- **D5, EMQX `ListAuthUsers`.** The definition still declares root `Vec<AuthUser>` (both variants). Because EMQX has no default, it resolves as an unpaginated list. The type fix stays pending the author's approval.
- **D6, the "~519 Endpoint literals" figure.** The current count is 511 production literals (547 endpoints minus 36 shared EMQX), or 569 raw `Endpoint {` matches including test modules. Neither is 519, so the R3 zero-diff check should state its own counting rule.
- **G1, unpaginated wrapper collections resolve "not a list", not "Unpaginated list".** R4 promotes only GET `Vec<T>` without an entry. The U wrappers that R28 says should "resolve to `Unpaginated`" have no root array: Anthropic `ListModels`, EMQX's 8 paginated lists ×2 (7 wrappers per variant; `ListAuthUsers` is a root array), HF `ListModelDiscussions`, Gitea trees, and ElevenLabs' 3 other wrappers. To assert `Unpaginated` rather than "not a list", they need explicit wrapper entries marked unpaginated, with item types. That is 43 wrapper collections, or 18 if only the U rows that R28 names get entries (Anthropic 1, EMQX 7×2, HF 1, Gitea trees 2). Otherwise the coverage test asserts "not a list". The plan should decide which.
- **G2, OpenAI page-size bounds are not uniform.** `limit` documents 1..100 with default 20 on 19 endpoints, 1..10000 with default 10000 (ListFiles), 0..1000 with default 100 (ListGroups, ListGroupUsers), default 1000 (ListRoles, ListProjectRoles), 1..180 with default 7 (UsageCosts), and a bucket-width-dependent maximum (31/168/1440) on the other usage endpoints. Several give no bound. An API-default `PageSize` must carry no `max` (or a per-entry override), or `InvalidPageSize`/R13 checks will reject valid requests.
- **G3, OpenAI cursor field types vary.** `first_id`/`last_id` are typed `String`, `Option<String>`, `serde_json::Value`, or `Option<serde_json::Value>` across S6 wrappers, and S7 `next`/S8 `next_page` are `serde_json::Value` (nullable). Selectors must read a string or null from any of these. Item types include `serde_json::Value` (GetChatCompletionMessages, RetrieveVectorStoreFileContent), which default 22 allows.
- **G4, GitLab keyset constants vs plain params.** `ListGroupProjects` already declares plain `order_by` and `sort`, the same names as the keyset alternate's constants. Alternates are not generated, but alternate validation (R2/R6) must define whether a constant may shadow a declared param.
- **G5, ElevenLabs wire names with hyphens.** `ListServiceAccounts` is at `service-accounts` (serde rename) and `ListServiceAccountApiKeys` at `api-keys`. Any entry must use the wire path, not the Rust field name. This is moot while they stay unpaginated.
- **G6, the "ElevenLabs gets a default" assumption.** The authoring table gives ElevenLabs three different strategies and no API default. Its only GET `Vec<T>` (`ListModels`) therefore needs no opt-out unless a default is later added.

##### Adoption candidates

Every row below is query-located. A row is adopted only if the knob is authored (R7). Under each listed strategy the knob is authored, except where noted.

| API | Endpoints | Param | Location | Strategy input | Knob authored? |
|---|---|---|---|---|---|
| GitHub | 9 (8 root + ListWorkflowRuns) | `page` | query | PageNumber page | yes, `.page()` |
| GitHub | same 9 | `per_page` | query | PageSize | yes, `.page_size()` |
| GitLab | 9 declared (not ListIssueParticipants) | `page`, `per_page` | query | PageNumber page / PageSize | yes / yes |
| Gitea | 8 declared | `page`, `limit` | query | PageNumber page / PageSize | yes / yes |
| Bitbucket | 8 wrappers | `pagelen` | query | NextLink adjustable size (query) | yes, `.page_size()` |
| Bitbucket | 8 wrappers | `page` | query | none (NextLink has no page input) | **no, field kept** |
| Bitbucket | GetSource | `page`, `pagelen` | query | not a list | **no, fields kept** |
| OpenAI | 47 (S6 36 + item-id 2 + S7 9) | `after` | query | Cursor forward | yes, `.after()` |
| OpenAI | 9 (listed above) | `before` | query | Cursor backward leg | yes, `.prev()`/`.before()` |
| OpenAI | 58 (all paginated) | `limit` | query | PageSize | yes, `.page_size()` (bounds: G2) |
| OpenAI | 11 S8 | `page` | query | Cursor (named `page`) | yes, cursor knob |
| OpenAI | 47 with `order` | `order` | query | none (plain filter) | not adopted, field kept |
| ElevenLabs | ListVoices | `page_size` | query | PageSize | yes, if authored |
| ElevenLabs | ListVoices | `page_token` | query | Cursor input `next_page_token` per research | **pending D2**: name mismatch, so no adoption until confirmed |
| ElevenLabs | GetHistory, ListSharedVoices | (none declared) | — | new inputs `start_after_history_item_id`/`page_size`, `page`/`page_size` | n/a (no field removed) |

**Call sites of adopted fields** (a quick grep of the 8 `schematic-schema` dependents plus schematic's own tests and docs). This is a pointer for the 1.2 migration ledger, not the full ledger:
- `sniff/lib/src/remote/github.rs:454`: `list_workflow_runs(..).per_page(limit)` (adopted).
- `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:576-578`: `list_voices().page_size(..)` and `.page_token(..)` (adopted if ListVoices is authored). The loop itself is a hand-written pager.
- `schematic/schema/tests/github_client.rs:549`: `list_pull_requests(..).per_page(5)`.
- Docs: `schematic/docs/runtime/calling-an-api.md:22,83,221` (`.per_page`), `schematic/docs/runtime/deadlines-and-timeouts.md:43` (`.page(page)` hand loop), `.claude/skills/schematic-define/schematic/SKILL.md:122`, `.claude/skills/schematic-define/schematic/gen.md:247`.
- Not affected (fields kept): `sniff/lib/src/remote/bitbucket.rs:177-179` and `schematic/schema/tests/non_json_endpoints.rs:65` (`GetSource` `pagelen`/`page`).
- No hits in `homelab/*`. `sniff` gitlab/gitea call sites use no page setters.

Artifacts: `/tmp/pagination-phase1/{endpoints.jsonl,analyzed.json,analyze.py,render.py,dump/}`.

#### Phase 1: dependents of schematic-schema, adopted-field ledger, serialized authoring inputs

Worktree: `/Volumes/coding/wt/rusty-biscuit/feat-modern-config` (branch `feat/modern-config`), 2026-10-13. Read-only survey; no repo file edited.

##### Dependents of schematic-schema

Found by searching every `Cargo.toml` (normal and dev sections) for `schematic-schema` / `schematic-definitions`:

| Crate (manifest) | Dependency | Generated modules used | Paginated API used? |
| --- | --- | --- | --- |
| `sniff` (`sniff/lib/Cargo.toml:44`) | schematic-schema | `github`, `gitlab`, `gitea`, `bitbucket`, `schematic_runtime` (`sniff/lib/src/remote/{github,gitlab,gitea,bitbucket,envelope}.rs`) | **yes** |
| `biscuit-speaks` (`biscuit-speaks/lib/Cargo.toml:27`) | schematic-schema | `elevenlabs`, `prelude`, `schematic_runtime` (`biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs`) | **yes** (ElevenLabs `ListVoices`) |
| `homelab` lib (`homelab/lib/Cargo.toml:10`) | schematic-schema | `eversolo`, `samsung_smart_tv`, `samsung_smart_tv_remote_ws`, `ws_shared`, `schematic_runtime` | no |
| `eversolo-integration` (`homelab/eversolo-integration/Cargo.toml:13`) | schematic-schema | `eversolo` (types), `unfolded_circle_integration_ws` | no |
| `arcam-amp-integration` (`homelab/arcam-amp-integration/Cargo.toml:12`) | schematic-schema | `unfolded_circle_integration_ws` | no |
| `sony-receiver-integration` (`homelab/sony-receiver-integration/Cargo.toml:12`) | schematic-schema | `unfolded_circle_integration_ws` | no |
| `unfolded-integration-helper` (`homelab/unfolded-integration-helper/Cargo.toml:10`) | schematic-schema | `unfolded_circle_integration_ws` | no |
| `schematic-schema` own tests (`schematic/schema/tests/*.rs`) | (self) | `github`, `bitbucket`, `huggingface`, `artificial_analysis`, ws modules | **yes** |
| `schematic-gen` tests (`schematic/gen/tests/l1/fresh_output_compile.rs`) | none in Cargo; regenerates the crate into a temp dir (`FreshCrate::generate`) and compiles embedded Rust test programs against it | `gitea`, `gitlab`, `openai`, fixture APIs | **yes** (wire-baseline program) |

**Correction to the spec's list:** `model-citizen` does **not** depend on `schematic-schema`. It depends on `schematic-definitions` only (`model-citizen/lib/Cargo.toml:39`). It uses response/body types and no endpoint params: `schematic_definitions::ollama::{ShowModelResponse, ShowModelBody}` (`model-citizen/lib/src/scanner/ollama.rs:276,284,295,642`) and `schematic_definitions::lmstudio::{ListModelsResponse, ModelInfo}` (`scanner/lmstudio.rs:374,405`). R32 still holds for it trivially. The homelab dependents are the **five** crates listed above.

Other `schematic-definitions` dependents: `schematic-schema` (`schematic/schema/Cargo.toml:15`) and `schematic-gen` (`schematic/gen/Cargo.toml:26`). None outside `schematic/` reads `EndpointParams`. Inside, the legacy pagination metadata is read in two places, both by `schematic-gen` for the `Paginated` marker: `schematic/gen/src/output/assemble/api.rs:88` and `schematic/gen/src/codegen/traits.rs:91` (`p.pagination.is_some()`). `schematic/gen/src/definition_gen.rs:320-321` writes `pagination: None, response_pagination: None` into the definitions it emits.

##### Adopted-field migration ledger

The 35 declared endpoints are GitHub 9, Gitea 8, GitLab 9 and Bitbucket 9 (`with_pagination` in `schematic/definitions/src/{github,gitea}/mod.rs`, `gitlab/endpoints/*.rs`, `bitbucket/mod.rs:105` `bitbucket_pagination()`). GitHub `ListWorkflowRuns` is one of the GitHub 9. Bitbucket `GetSource` (`bitbucket/mod.rs:175-190`, `ApiResponse::Binary`) is one of the Bitbucket 9.

###### Production dependents

| # | Crate | file:line | Endpoint / type | Field | Kind | Single page or loop | Planned migration |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | sniff | `sniff/lib/src/remote/github.rs:451-454` | GitHub `ListWorkflowRuns` / `ListWorkflowRunsCall` | `per_page` | call setter `.per_page(limit as i64)` | single page, bounded by `limit`. The call is `.clone()`d and resent `.anonymous()` on 401/403 (lines 457-471) | **fetch_page**: `client.pager(ListWorkflowRunsRequest::new(owner, repo)).page_size(limit).fetch_page()`. The result type changes from the `workflow_runs` wrapper to `Page<WorkflowRun>`, and the code at ~475+ that reads the wrapper (`total_count` etc.) must adapt. **Gap to check:** the pager or `fetch_page()` needs per-request `.anonymous()` (and must be clonable or rebuildable) to keep the anonymous retry. |
| 2 | sniff | `sniff/lib/src/remote/bitbucket.rs:175-177` | Bitbucket `GetSource` / `GetSourceCall` | `pagelen` | call setter `.pagelen(100)` | inside a hand loop `for page in 1..=MAX_LISTING_PAGES` (`:174`, const `:149`) that stops when the body's `next` is `None` (`:188-192`) | **Depends on whether `GetSource` gets a strategy (open; see Notes).** If it is paginated (NextLink, size `pagelen`), `pagelen` is adopted: replace the loop with pager iteration or `collect_all` with `.page_size(100)`. Its page limit is 10 today (`MAX_LISTING_PAGES`), and it reports `truncated=true` at the limit (matches `Incomplete{PageLimitReached}`). If it is opted out (Binary response), **no change**. |
| 3 | sniff | `sniff/lib/src/remote/bitbucket.rs:179` | Bitbucket `GetSource` / `GetSourceCall` | `page` | call setter `call.page(page)` | same hand loop | Under R7's knob rule a NextLink strategy has no page-number knob, so `page` is **not adoptable** and the field stays (no change). R7 also says "every `with_pagination` query parameter … is adopted". These conflict for Bitbucket `page`; see Notes. If the loop is replaced per row 2, this call goes away with it. |
| 4 | biscuit-speaks | `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:576` | ElevenLabs `ListVoices` / `ListVoicesCall` | `page_size` | call setter `.page_size(VOICES_PAGE_SIZE)` | hand loop over all pages (`list_voices_raw`, `:561-615`), ending on `has_more` plus `next_page_token` | If the `ListVoices` Cursor strategy adopts `page_size` (it needs the PageSize knob): **remove** the hand loop and use `client.pager(ListVoicesRequest::new()).page_size(VOICES_PAGE_SIZE).collect_all()`. This keeps the "has_more without token" guard as an engine error. Otherwise, use `fetch_page()` per iteration. |
| 5 | biscuit-speaks | `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:577-579` | ElevenLabs `ListVoices` / `ListVoicesCall` | `page_token` | call setter `call.page_token(token)` | same hand loop | Adopted only if the strategy's cursor input is `page_token`. The definition sends `page_token` (`schematic/definitions/src/elevenlabs/endpoints/voices.rs:17-23`), but the spec table names the input `next_page_token`. If the input is authored as `page_token`, it is adopted: same migration as row 4. If it is `next_page_token`, it does **not** collide, the `page_token` field stays, and a second wire name appears, which is wrong. Confirm the sent name before adopting. |

Not affected, but similar in name:
- `sniff/lib/src/remote/bitbucket.rs:600-601`: `pagelen: None, page: None` belong to the **response** struct literal `PaginatedResponse { .. }` (fallback for `ListDownloads`). Response fields are not request fields: **no change**.
- `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:613`: `next_page_token: None` is in the **response** literal `ListVoicesResponse`: **no change**.
- These first-page-only plain calls set no adoptable field, so they need **no change** (R11a: the typed send still returns the wrapper): sniff `github.rs:349,377,381`, `gitlab.rs:330,359,363`, `gitea.rs:404,444,448`, `bitbucket.rs:372-389` (sets `.state()` only), `bitbucket.rs:496,560,589-591`.
- `sniff/lib/src/remote/gitea.rs:166` `get_git_tree_recursive` sets no `per_page`, and the spec leaves the tree unpaginated: **no change**.
- homelab (5 crates) and model-citizen: **zero uses**.
- Other `.limit(..)`/`limit:` hits in sniff (`remote/types.rs:364,388,643,664`, `provider.rs:151,765`, `focused.rs:330,720`, `filesystem/*`) belong to sniff's own query types and do not touch schematic: **no change**.
- **Hand-written pagination outside scope:** `sniff/lib/src/remote/focused.rs` builds page and size params by hand (`pagination_params` `:1922`, `direct_job_pagination_params` `:1932`) and reads `Link rel="next"` (`:104`, `:1905`) over its own HTTP client. It does not use schematic-schema. Retiring it is out of scope.

###### schematic-internal tests (same-phase migration required)

| # | file:line | Endpoint / type | Field | Kind | Planned migration |
| --- | --- | --- | --- | --- | --- |
| 6 | `schematic/schema/tests/github_client.rs:549` | GitHub `ListPullRequests` / `ListPullRequestsCall` | `per_page` | call setter `.per_page(5)` | **fetch_page** (`client.pager(req).page_size(5).fetch_page()`), or drop `per_page` from the test and assert only `state=closed`. The test compares the endpoint method with generic `send`, so dropping it keeps its intent. |
| 7 | `schematic/schema/tests/github_client.rs:556` | `ListPullRequestsRequest` | `per_page` | request builder `.with_per_page(5)` | same as row 6 |
| 8 | `schematic/schema/tests/non_json_endpoints.rs:65` | Bitbucket `GetSource` / `GetSourceCall` | `pagelen` | call setter `.pagelen(10)` | Only if `GetSource` is paginated: **remove** the setter (the test is about binary decoding; `pagelen` is incidental). Otherwise no change. |
| 9 | `schematic/gen/tests/l1/fresh_output_compile.rs:1588-1589` (inside the `WIRE_CAPTURE_TEST` raw string, compiled against the regenerated crate) | Gitea `ListPullRequestsRequest` | `page`, `limit` | field assignment `scalars.page = Some(2); scalars.limit = Some(0);` | Both are adopted (PageNumber `page` plus size `limit`). Wire line 1 of `schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt` (`?page=2&limit=0&state=…`) depends on them. Either capture through `gitea.pager(req).page(2)…fetch_page()`, if the pager exposes a start-page knob and sends `limit=0` (the R13 bounds may refuse 0, so the baseline changes), or drop the fields and update the baseline line (an approved wire change, which must be logged). |
| 10 | `fresh_output_compile.rs:1608` | Gitea `ListPullRequestsRequest` | `page` | assignment `paged.page = Some(1)` | Same choice as row 9. Baseline line 5 (`base-url-with-query … &page=1`). |
| 11 | `fresh_output_compile.rs:1625, 1627` | OpenAI `ListConversationItemsRequest` (`schematic/definitions/src/openai/mod.rs:535`, declares `limit`, `order`, `after`, `include`) | `limit`, `after` | assignment | Adopted (Cursor `after` plus size `limit`). Baseline line 7 (`?limit=5&order=a%2Bb&after=x%25y&include=…`) checks `after` percent-encoding and declared-order interleaving with `order`. A pager-sent query may reorder params. Either migrate to `fetch_page()` and re-baseline, or move the encoding check to a non-adopted param. |
| 12 | `fresh_output_compile.rs:1638` | OpenAI `BetaListInputItemsRequest` (`openai/mod.rs:4997`) | `limit` | assignment | Adopted. Baseline line 8 (`?beta=true&limit=3&include=…`). Same choice as row 11. |
| 13 | `fresh_output_compile.rs:1654-1655` (`into_parts_returns_unencoded_query_pairs_apart_from_the_path`, same raw string) | Gitea `ListPullRequestsRequest` | `page`, `limit` | assignment | Assert on non-adopted params only (`state`), or remove. `into_parts` will not carry adopted fields. |
| 14 | `fresh_output_compile.rs:1691` | OpenAI `ListConversationItemsRequest` | `limit` | assignment | Same as row 13. The expected `parts.query` at `:1695` changes. |
| 15 | `fresh_output_compile.rs:1698` | OpenAI `BetaListInputItemsRequest` | `limit` | assignment | Same as row 13 (`:1701`). |

Not affected (the APIs have no strategy, so nothing is adopted):
- Fixture APIs in `fresh_output_compile.rs:897,925,962,1235,1277,1279`: `FixtureBearer::list_widgets().limit(..)`, `FixtureNames::list_items(..).limit(..)`, `FixtureSettings` `EchoRequest::with_page`.
- Codegen unit tests on ad-hoc endpoints without strategies: `schematic/gen/tests/l1/query_params_codegen.rs:259,482-494,586,673-678`, `schematic/gen/src/codegen/request_structs/tests.rs:863,1065-1338`, `schematic/gen/src/codegen/endpoint_methods/tests.rs:446`, `schematic/gen/tests/l1/e2e_generation.rs:888-912`.
- Generated doc examples in `schematic/schema/src/**/requests.rs` (for example `gitlab/requests.rs:15-16`) are regenerated.

###### Documentation that shows adopted setters (drift, not compiled)

No test reads these files. Update them when the fields are removed:
- `schematic/docs/runtime/calling-an-api.md:22, 33, 83, 221`: `list_releases(..).per_page(10)`, `list_pull_requests(..).per_page(50)`
- `schematic/docs/runtime/deadlines-and-timeouts.md:43`: hand loop with `.page(page)`; a natural pager example
- `schematic/docs/generation/request-side.md:84`: Gitea `ListPullRequestsRequest::with_page(2)`, with the expected `parts.query` containing `page`
- `.claude/skills/schematic-define/schematic/SKILL.md:122`, `.claude/skills/schematic-define/schematic/gen.md:241-247`
- `sniff/docs/repos/repo-tech-design.md:857`: quotes the `list_workflow_runs(..).per_page(..)` line

##### Grep commands run

Run from the worktree root. `target/` was excluded.

```sh
grep -rn --include=Cargo.toml -E 'schematic-(schema|definitions)' .
grep -rn --include='*.rs' -E 'schematic_(schema|definitions)::[a-z_]+' homelab sniff/lib biscuit-speaks model-citizen schematic/schema/tests schematic/gen
P='\.(page|per_page|limit|after|before|pagelen|page_size|page_token|next_page_token|start_after_history_item_id)\(|\b(page|per_page|limit|after|before|pagelen|page_size|page_token|next_page_token|start_after_history_item_id)\s*[:=]'
grep -rnE "$P" sniff/lib/src/remote biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs homelab/lib/src model-citizen/lib/src
P2='\.(page|per_page|limit|after|before|pagelen|page_size|page_token|next_page_token|start_after_history_item_id)\(|with_(page|per_page|limit|after|before|pagelen|page_size|page_token)\(|\.(page|per_page|limit|after|before|pagelen|page_size|page_token)\s*=\s'
grep -rnE "$P2" --include='*.rs' sniff biscuit-speaks homelab model-citizen so-you-say
grep -rnE '<P with "= Some|None|[0-9]">' schematic/schema/tests schematic/gen/tests schematic/gen/src schematic/runtime schematic/definitions/tests schematic/define/tests
grep -rnE '\bwith_(page|per_page|limit|after|before|pagelen|page_size|page_token|start_after_history_item_id)\(' --include='*.rs' --include='*.md' .
grep -rnE '[A-Z][A-Za-z]+Request\s*\{|Request::(new|default)\(|\.send\(' --include='*.rs' sniff/lib/src/remote biscuit-speaks/lib homelab model-citizen/lib
grep -rnE '\.full\(\)|link_header|rel="next"|Link' sniff/lib/src/remote/*.rs
grep -rnE '\.(pagination|response_pagination)\b|PaginationStyle|PaginationResponse' --include='*.rs' .
grep -rnE '<setter regex>' --include='*.md' schematic/docs schematic/README.md schematic/skill .claude/skills sniff/docs biscuit-speaks
```

Results: homelab (all five crates) and model-citizen have **zero** matches on schematic request or call types. Every match is listed in the ledger above.

##### Serialized authoring inputs

**Conclusion: serde on the define model is only a round-trip and export facility. No configuration reader exists for REST API definitions, so the new pagination model would flow through no deserializer. "Input robustness" covers only the Rust-authored model and the OpenAPI importer, which is a separate import format.**

Evidence:

1. **The core REST authoring types do not derive `Deserialize`, or `Serialize`.**
   - `RestApi`: `schematic/define/src/types.rs:123`, `#[derive(Debug, Clone, PartialEq, Eq)]`
   - `Endpoint`: `types.rs:445`, same derives
   - `EndpointParams`: `schematic/define/src/params/mod.rs:85`, `#[derive(Debug, Clone, PartialEq, Eq, Default)]`
   - `ParamDef`: `params/param_def.rs:24`
   - `QueryParamType` (`:68`) and `ParamStyle` (`:113`): no serde
   - `PaginationStyle`: `schematic/define/src/pagination.rs:35`, `#[derive(Debug, Clone, PartialEq, Eq)]`

   A `RestApi`/`Endpoint`/`EndpointParams`/`PaginationStyle` value therefore cannot be read from JSON, YAML or TOML at all.
2. **Define-side types that do derive `Deserialize`:**
   - `PaginationResponse` (`pagination.rs:276`)
   - `RestMethod` (`types.rs:54`)
   - `ApiResponse` (`response.rs:43`)
   - `FormFieldKind`, `FormField`, `ApiRequest` (`request.rs:41,108,325`)
   - `Schema` (`schema.rs:113`)
   - `AuthStrategy`, `AuthMethod`, `EnvAuthStrategy`, `AuthPolicy`, `ApiKeyLocation` (`auth.rs:71,193,226,256,338`)
   - `OAuth2Config`, `OAuth2GrantType`, `PkceRequirement`, `OAuth2ClientAuthMethod` (`oauth.rs:31,52,64,77`)
   - `EnvList`, `ApiKeyEnv`, `EnvMapping` (`headers/env.rs:26,108,148`)
   - OpenAPI `x-schematic` extensions `SchematicDocExtension`, `SchematicOpExtension`, `SchematicSchemaExtension`, `AuthExtension` (`openapi/extensions.rs:40,86,127,144`)
   - WebSocket model: `FrameFormat`, `RequestIdType`, `CorrelationHints`, `AuthFlowHints`, `HeartbeatHints`, `WebSocketRuntimeHints`, `WebSocketEndpointHints` (`websocket/hints.rs:18,57,98,130,157,194,243`); `MessageDirection`, `MessageSchema`, `ConnectionLifecycle` (`websocket/lifecycle.rs:25,60,102`); `WebSocketEndpoint`, `WebSocketApi` (`websocket/mod.rs:151,201`); `ParamType`, `ConnectionParam` (`websocket/params.rs:21,57`)

   `PaginationResponse` derives serde, but it is only reachable inside `EndpointParams`, which does not.
3. **Production readers of these types: none.**
   - Every `serde_json::from_str::<DefineType>` in `schematic/define/src` is inside `#[cfg(test)]`. Examples: `pagination.rs:472-509`, `types.rs:549`, `auth.rs:536,591`, `oauth.rs:108-207`, `request.rs:650-866`, `openapi/extensions.rs:248-510`, `websocket/mod.rs:296-857`.
   - The `x-schematic` extension types are only **written** by the exporter (`openapi/export/mod.rs:78`, `openapi/export/paths.rs:84`). The importer never reads them back; nothing under `define/src/openapi/import` references them.
   - Production deserialization in `schematic/gen/src` is limited to `serde_json::Value` in `asyncapi_import.rs:218` (the AsyncAPI document), and to generated client and WS runtime code (`codegen/client/methods.rs:223,231`, `ws_codegen/client.rs:715,723`, `ws_codegen/host.rs:359,372`) that decodes API **responses**, not definitions. `schematic/definitions/src` deserializes only in tests (for example `gitlab/types.rs:65`).
4. **CLI:** the `schematic-gen` subcommands (`schematic/gen/src/main.rs`) take `--api <NAME>`, a name looked up in the compiled `schematic_definitions` registry, not a definition file. The `import --input` and `import-asyncapi --input` flags take OpenAPI or AsyncAPI documents.
5. **The OpenAPI importer is a separate import format.** `schematic/define/src/openapi/import/builder.rs:223-250` (`parse_source`) reads a path via `serde_yaml_ng::from_str` into `openapiv3::OpenAPI`, or JSON via `serde_json::from_str`. It then maps to a `RestApi` in Rust. It emits no pagination: there is no `pagination` reference under `define/src/openapi`, and `gen/src/definition_gen.rs:320-321` writes `pagination: None`. Per D57 item 4, OpenAI pagination will come from a hand-written overlay that the generated definitions call (`openai_pagination::apply(api)`), not from the imported document.

Existing serde-related tests in `schematic/define`, all round trips except where noted:
- `types.rs:544` `rest_method_serde_roundtrip`
- `pagination.rs:463,477,487,502`: the `pagination_response_serialization_*` tests (4)
- `request.rs:647,655,665,677,688,696,851`: form-field kind and form-field round trips, `api_request_serde_roundtrip_{json,form_data}`, `integration_all_request_variants_serializable`
- `auth.rs:533,585`
- `oauth.rs:131,152,183,215`. `endpoint_with_oauth_scopes_serde` at `:215` only asserts field access, because `Endpoint` has no serde.
- `websocket/mod.rs:293,355,366,382,406,441,470,551,567,578,599,627,648,660,677,698,713,727`, including deserialize-with-defaults tests at `:660,698,713`
- `openapi/extensions.rs:236,336,381,459,479,488,502`
- `openapi/options.rs:194,215,235,254`: export serialize to JSON or YAML, then re-parse as a `Value`
- `tests/openapi_tests.rs:62`: parses a YAML OpenAPI fixture into `openapiv3::OpenAPI` (importer input)

Implication for the plan: input-robustness work for the new `PageStrategy` or API fields needs **no** serde derive and no tolerance of unknown or missing fields. If a derive is added for symmetry with `PaginationResponse`, it is optional and covered only by round-trip tests. Validation (R2/R5/R7) operates on the Rust-constructed model.

##### Notes

- **The R7 adoption list conflicts with the knob rule for Bitbucket `page`.** R7 says every `with_pagination` query param on the 35 endpoints is adopted. `PaginationStyle::bitbucket()` materializes both `page` and `pagelen` (`schematic/define/src/pagination.rs:105-106`). The Bitbucket strategy is `NextLink`, with size `pagelen` and no page-number input, and R7 forbids adopting a param whose knob is not authored. Result: `pagelen` is adopted and `page` stays a request field (with no `CollidesWithEndpointParam`, since NextLink has no `page` input). This decides ledger row 3. Log it as the default chosen.
- **Bitbucket `GetSource` is one of the 35 but returns `ApiResponse::Binary`** (directory listing or raw file). The authoring table says each Bitbucket list is "a table entry at `values` with its item type". That cannot be checked structurally against a Binary response, and a file path returns no envelope. The planner must decide to paginate it (and handle the dual shape) or opt it out. This decides ledger rows 2, 3 and 8.
- **ElevenLabs `ListVoices` name mismatch:** the definition sends `page_token`. The spec table names the cursor input `next_page_token`, which is the response field. Author the input as `page_token <- next_page_token`, or `page_token` is not adopted and a second param name appears. `GetHistory` and `ListSharedVoices` declare **no** params (`params: None`, `elevenlabs/endpoints/history.rs:13`, `voices.rs:128`), so adding their strategies removes no field.
- **OpenAI:** no production dependent uses `schematic_schema::openai`. The only uses are the `schematic-gen` wire-baseline tests (rows 9-15). Re-baselining `schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt` lines 1, 5, 7 and 8 (and possibly line 2) would be a deliberate wire-test change and must be logged. Alternatively, keep the lines by sending through `fetch_page()` with matching knob values, if the pager serializes adopted params in the same position and order.
- **The sniff `ListWorkflowRuns` anonymous retry** relies on `Call: Clone` plus `.anonymous()`. Check that the pager or `fetch_page()` surface offers per-send credential override and is cheap to rebuild. If it does not, the migration in row 1 loses behavior.
- The legacy `Paginated` marker readers (`gen/src/output/assemble/api.rs:88`, `gen/src/codegen/traits.rs:91`) are removed in the generator phase (spec phase 7). They are not dependents but are listed for completeness.

#### Artifact baseline (task 1.3)

##### Host and toolchain context

- Apple M4 Max, 16 CPUs, 128 GiB, Darwin 27.2.0 arm64. rustc and cargo 1.98.1, nextest 0.9.136, just 1.58.0.
- Compiler cache: kache 0.26.3 through `~/.cargo/config.toml` (`build.rustc-wrapper = "kache"`). `RUSTC_WRAPPER` and `CARGO_TARGET_DIR` are unset. The repository has no `.cargo/config.toml`.
- HEAD `0cd6075e6f0bffd6d9c791a3ff2f93616545ba1f`. The only working-tree change is the untracked implementation log.
- Other Phase 1 agents ran at the same time (load average 30 to 41), so the cold-build times are upper bounds.

##### Generation baseline

- Command: `cargo run -p schematic-gen -- generate --api all --output <tmp>/schema-gen/src --openapi-out <tmp>/openapi --postman-out <tmp>/postman -v`. The generator writes the manifest one level above `--output`, so no tracked file was touched.
- It ran in 1.8 s and printed 27 expected `query-in-path` warnings (9 endpoints across 3 outputs).
- `diff -r` against `schematic/schema/src` and `Cargo.toml` is empty: the committed client equals a fresh generation.

##### Export baseline

- OpenAPI (15 files) and Postman (15 files) are byte-identical to the committed `schematic/openapi` and `schematic/postman`; both diffs are empty.
- `hashes.txt` holds the sha256 of all 73 generated files.

##### Request surface baseline

- 547 `*Request` structs, 547 `*Call<'a>` types and 547 `EndpointSpec` impls.
- 35 legacy `impl Paginated`: bitbucket 9, github 9, gitlab 9, gitea 8.
- 197 candidate pagination fields across 79 structs (in `adoptable-fields.txt`):
  - by name: limit 66, after 47, page 46, per_page 18, pagelen 9, before 9, page_token 1, page_size 1
  - by API: openai 125, bitbucket 18, github 18, gitlab 18, gitea 16, elevenlabs 2
- 19 near-miss names are listed separately for information, for example OpenAI `starting_after`.

##### Recipe results (warm cache, kache on)

| Recipe | Wall time | Result |
| --- | --- | --- |
| `just test` | 51.2 s | pass: L1 1937/1937 (12 skipped); fresh-output 6/6; compile-fail 1/1 |
| `just test-fresh-output` | 23.3 s | pass: 6/6 |
| `just test-compile-fail` | 1.9 s | pass: 1/1 |
| `just lint` | 76.5 s | pass |
| `just test-schema` | 0.9 s (up to date) | pass: 40/40 |
| `just lint-schema` | 0.4 s (up to date) | pass |

Cold build times for the generated crate, each on an empty target dir under `/tmp`:

| Build | Time |
| --- | --- |
| Full build, all targets, with kache | 44.7 s |
| Full build, all targets, without kache | 69.8 s |
| `schematic-schema` library alone | 35.5 s (32.8 s in the front end) |
| `schematic-definitions` | 60.8 s |
| Cold `clippy` (equivalent to `lint-schema`) | 60.6 s, pass |

##### Coverage of recipes

- `test` and `lint` both cover `schematic-runtime`, along with define, gen, definitions and oauth. Neither covers `schematic-schema`; only `test-schema` and `lint-schema` do.
- `test` also runs `test-fresh-output` and `test-compile-fail`.
- `lint` runs clippy on `schematic-define` without `--all-features`, so its `openapi`-gated code is linted only indirectly, through the gen and definitions runs.

##### Pre-existing failures

None.

## Phase 2

Phase 2 builds the define-side pagination model, its resolution and validation, the two `RestApi` fields, and the gen wiring. Started 2026-10-13 on `feat/modern-config`. Entries are appended as work lands.

### 2.1 Model contracts (done)

- **Layout.** `schematic/define/src/pagination.rs` keeps the legacy types in place and declares five submodules under `schematic/define/src/pagination/`: `selector` (`JsonPath`, `Step`, `ItemSelector`), `strategy` (inputs, value sources, the four kinds, shared dimensions, builders), `collection` (`Pagination`, `Alternate`, `Collection`, `PaginationException`), `resolve` (resolution, knobs, adoption), and `validate` (rules and diagnostics). Splitting the file into a directory module was avoided, so the legacy types show no move in the diff.
- **Named strategy structs (S12, Proposed default adopted).** `PageStrategy` is `#[non_exhaustive]` with `PageNumber(PageNumberStrategy)`, `Offset(OffsetStrategy)`, `Cursor(CursorStrategy)`, and `NextLink(NextLinkStrategy)`. The structs have public fields and no `#[non_exhaustive]`; only open enums carry it (R1). Alternative: struct variants, as in the prototype. Reversal: a mechanical rewrite of the enum and its matches.
- **Departures from the prototype**, all intentional:
  - `Method` is replaced by the crate's `RestMethod`.
  - `LinkTrust`, `NextLink.trust`, `NextLinkBuilder::trust`, `is_link_trusted` and `InvalidTrustedOrigin` are removed (default 16; link trust is the API's `CrossOrigin` policy).
  - `PageSize` gains `sent_default: Option<u32>`, the authored value R2a requires for a provider-required size. `server_default` stays documentation only.
  - `Collection` gains `item_type: Option<String>` (R26, default 22); `PaginationException { endpoint, collection }` is the table row (P8 item 3).
  - `Partition::new(index, count).max(n)` and `SessionStep::constant(..)` builders were added.
  - Runtime-side pieces stay out of define: `JsonPath::lookup`, `ItemSelector::{records, items, infer_root}`, `Strategy::is_terminal`, and `BoundaryTracker` need `serde_json::Value`, which define has only behind the `openapi` feature. They move to the runtime in Phase 3 (task 3.1), with the prototype's boundary and selector cases (task 3.3).
  - `Knob::Close` and `Knob::Alternate` are dropped: there is no manual close (D27), and alternates add no caller surface (R6). `Knob::Session` (`with_session`) is added.
- **Strict selector and path parsing.** `JsonPath::parse` and `ItemSelector::parse` keep empty segments instead of filtering them, so `""` and `"a..b"` are rejected by validation. A root array is spelled only `Collection::root()` (or `"[*]"`); `Collection::at("")` is invalid. This follows the plan's "root selector requires explicit root form".
- **No empty usable-looking defaults.** No model type derives or implements `Default`, including `Collection`, `ItemSelector`, `JsonPath`, `Pagination`, and `Boundary` (all had `Default` in the prototype).
- **Serde.** Every struct and struct-variant enum uses `#[serde(deny_unknown_fields)]`; no field has `#[serde(default)]`, so a missing list (`constants`, `terminal_values`, `alternates`, `parts`) or boolean (`adjustable`, `resumable`) is a decode error rather than an empty or false value. `Option` fields keep serde's meaning, absent = null = none; the model defines no difference between them (Phase 1 recorded that no configuration reader exists).
- **Exports.** The new types are public under `schematic_define::pagination` and the authoring subset is in `schematic_define::prelude`. They are not re-exported at the crate root: `definitions/src/openai/mod.rs` glob-imports the crate root next to its own glob of OpenAI types, and root re-exports of names such as `Session` or `Collection` could make those names ambiguous there.
- **Legacy (R8).** `PaginationStyle`, `PaginationResponse`, `with_pagination`, and the `pagination`/`response_pagination` fields are unchanged in code and documented as superseded. No `#[deprecated]`.
- **`RestApi` fields (R3).** `default_pagination: Option<Strategy>` and `pagination_exceptions: Vec<PaginationException>`. 92 literals were updated by a script that inserts both fields, empty, after each `secret_names:` line: 18 production definitions (in 15 files), the importer builder, the definition emitter, and doc and test literals in define, gen, and definitions. No `Endpoint` or `EndpointParams` literal changed. The emitter always emits `default_pagination: None, pagination_exceptions: vec![]` with a comment saying why: an imported document yields no pagination, and the overlay (Phase 6) adds it.

#### Contract frozen for Phase 3

- `resolve(&RestApi, &Endpoint) -> Resolution` with `NotAList`, `Unpaginated { list, origin }`, `Paginated { list, strategy, alternates, origin }`. `ResolvedList { items: ItemSelector, item_type: Option<String> }`. `ResolutionOrigin::{Exception, ApiDefault, Unauthored}`: the spec's "None" is spelled `Unauthored` to avoid confusion with `Option::None`.
- `caller_knobs(&Resolution) -> Vec<Knob>` / `strategy_knobs(&Strategy)`: the generator emits a knob forward and marker impl for exactly these. Order: start (`StartPage`/`StartOffset`/`ResumeCursor` forward then backward), `PageSize`, `AdjustPageSize`, `KeepAlive`, `Partitions`, `Next`, `Prev`, `Total`, `Renew`, `Session`.
- `param_adoptions(&Endpoint, &Strategy) -> Vec<ParamAdoption>` and `adopted_params(..)` (the `Adopted` subset). The request-struct emitter in Phase 3.2 removes exactly `adopted_params`, so validation and emission share one decision.
- `root_array_item_type(&ApiResponse)` names `T` for an explicit `Vec<T>`.
- The `&'static` runtime projection mirrors these define types with slices instead of `Vec` and `&'static str` instead of `String`; Phase 3.1 owns it.

### 2.2 Resolution validation (done)

- **Define side.** `validate_pagination(&RestApi) -> Vec<PaginationDiagnostic>` and `validate_strategy(&Strategy)`. A diagnostic carries `endpoint: Option<String>` (none for the API default), `alternate: Option<String>`, and a `PaginationError`. Its `rule()` is a stable kebab-case id, its `group()` one of `Strategy`, `Exception`, `Parameter`, and it renders as `<rule>: <detail>`, adding `(alternate `label`)` when the error is in an alternate.
- **Reporting the default once.** An API default's intrinsic errors are reported once with no endpoint. Each endpoint that inherits it is checked only for endpoint-dependent rules (paths, methods, carry names, declared parameters). The prototype re-reported the default's errors on every inheriting endpoint; with 38 OpenAI lists sharing one default that would flood the output.
- **Gen wiring.** The plan's spelling is `validation.rs`, and Phase 1 found there is no `validate_api`. `check_pagination` runs at the end of `check_rust_api`, inside `validate_rust_client`, so it applies to the Rust client only; OpenAPI and Postman are unaffected. Three new `Feature` ids, `pagination-strategy`, `pagination-exception`, and `pagination-parameter`, group the rules. The rule id leads the message, which is how P8 item 3's `<api>.<endpoint_id>: <rule>: <detail>` fits the landed `ValidationDiagnostic` format. Alternative: one `Feature` per rule (about 35 variants). Reversal: map `PaginationError::rule()` to features one-to-one.
- **Schema-aware checks (R5, R26).** Gen adds two rule ids of its own (all under `pagination-exception`):
  - `invalid-item-type`: an authored item type that `syn` cannot parse.
  - `unknown-item-type`: a single non-primitive identifier that is absent from the API's registry, where nested schemas count. `to_openapi_schemas()` flattens nested definitions; that matters because `WorkflowRun` exists only nested inside `WorkflowRunsResponse`.

  `not-an-array-location` walks the selector's record steps through the response type's registered JSON schema, following `$ref`, single-member `allOf`, and nullable `anyOf`/`oneOf`. It reports only a demonstrable non-array. A missing property, an unresolved reference, an unregistered or qualified response type, or no registry at all is skipped, and nothing claims the location was checked; the runtime enforces it in Phase 4. The registry is built lazily and only for APIs that have pagination, so the shipped definitions pay nothing today.
- **Define-side table rules** need no schema:
  - an unknown row id, with the nearest id by edit distance when it is within max(2, len/3)
  - a duplicate row id; the first row decides resolution
  - an invalid selector syntax
  - a wrapper key on an explicit `Vec<T>` response, or a row on a non-JSON response (`not-an-array-location`)
  - a paginated list with no item type (`missing-item-type`)
  - an empty item type

  An unpaginated row needs no item type.
- **Adoption and R2a.** `param_adoptions` classifies every declared parameter that a strategy input names; the outcomes are `Adopted`, `WithoutKnob`, `NotAdoptable`, and `NearMiss`. A near miss is the same name in another location (query, header, cookie, or path). Body inputs are not matched, because the body has no declared parameter list. Adoption requires the knob that replaces the field:
  - page, offset, page size, and partition index/count always have one
  - a cursor part has one only with `resumable`
  - keep-alive has one only when `adjustable`
  - constants and the session handle never do

  A required adopted parameter needs a sent value. Page and offset send their base, keep-alive its default, and a size its `sent_default`. Cursor parts and partition inputs have none, so a required one is rejected.
- **Alternates** are checked against every rule except the declared-parameter rules. An alternate is not generated, so it removes no field. This applies the Phase 1 decision for GitLab `ListGroupProjects`, where the keyset constants `order_by` and `sort` shadow declared parameters.
- **Destination conflicts.** `DuplicateInput` compares query names exactly, header names without case, and body paths by prefix (`pit` against `pit.id`). It covers the page and offset inputs, cursor parts on both legs, size, constants, partition inputs, the session handle, and keep-alive on pages. Keep-alive on open is excluded, since it goes on a different request.

#### Rules from design.md section 8: kept, renamed, dropped

| Rule | Status |
| --- | --- |
| `CursorWithoutEndSignal`, `FullPageOnCursor`, `FullPageWithoutPageSize`, `PageCountOnNonPageNumber`, `EmptyCursorLeg`, `InclusiveBoundaryNeedsItemKey`, `AdjustablePageSizeOnPageNumber`, `AdjustableLinkSizeNotQuery`, `NextLinkWithFollowUpPath`, `NextLinkMethodUnspecified`, `UnknownCarryParam`, `UnknownPathParam`, `SessionHandleUnreachable`, `SessionHandleNotSent`, `SessionStepNeedsHandle`, `KeepAliveWithoutInput`, `InvalidPartitionMax`, `AlternatesWithoutPrimary`, `AlternatesOnUnpaginated`, `DuplicateLabel`, `InvalidLabel` | kept |
| `InvalidPageSize` | kept; also checks `sent_default` is within bounds |
| `DuplicateInput` | kept; widened to header case and body-path prefixes |
| `BodyInputOnGet` | kept; HEAD is also body-less |
| `CollidesWithEndpointParam` | kept, narrowed by adoption: near misses, constants, and session handles only |
| `PagerManagedRequiredWithoutDefault` | added (R2a) |
| `AdoptionWithoutKnob` | added (R7); the prototype reported these as collisions |
| `EmptyValue`, `UnknownExceptionId`, `DuplicateExceptionId`, `InvalidItemSelector`, `NotAnArrayLocation`, `MissingItemType` | added (R5, R26, input robustness) |
| `invalid-item-type`, `unknown-item-type` | added, gen only (R26) |
| `InvalidTrustedOrigin` | dropped with `LinkTrust` (default 16) |

### 2.3 Model evidence (done)

New test files:
- `define/tests/pagination_model.rs` (29 tests)
- `define/tests/pagination_tables.rs` (20)
- `define/tests/pagination_inventory.rs` (15)
- `define/tests/pagination_serde.rs` (9)
- `define/tests/endpoint_literal_surface.rs` (1)
- `gen/tests/l1/pagination_validation.rs` (6), registered in `gen/tests/l1/main.rs`

Shared fixture builders are in `define/tests/pagination_support/mod.rs`. Every test is L1 with an untiered name.

- **Port of the 56 prototype tests.**
  - The `patterns.rs` and `generalizations.rs` assertions about the model are in `pagination_model.rs`, each R2 rule next to its valid sibling.
  - The 16 `inventory.rs` tests are in `pagination_inventory.rs` as fixture APIs written with a default and an exceptions table, and asserted through `resolve` and `caller_knobs`. Two prototype tests were merged into one (`unpaginated_endpoints_expose_no_knobs` and `every_paginated_endpoint_exposes_next`). `openai_plain_after_param_collides_with_strategy` was replaced by the adoption assertions, because R7 now adopts those parameters, plus `openai_cursor_without_resume_cannot_adopt_after`. Fixtures follow the Phase 1 inventory: Bitbucket has 8 lists, GitLab participants opt out, and ElevenLabs `ListVoices` is absent. The unverified Anthropic, EMQX, and Hugging Face strategies are labeled as mechanisms, not real definitions.
  - Not ported here: `p18_*` link trust (removed). The runtime-only assertions are deferred to Phase 3 task 3.3, where they target the runtime projection: JSON lookups through `JsonPath::lookup` and `ItemSelector::items`, `BoundaryTracker`, and `is_terminal`.
- **Serde (strict round trip; no configuration reader, per Phase 1).** The tests cover:
  - a repeated write/read/write round trip that is byte-identical, with equal knobs and validation
  - an unedited control
  - unknown fields at every level
  - an absent list or flag as an error, with `[]` accepted and distinct
  - an absent `Option` equal to `null`, an equality the model defines
  - wrong types: whole field, one element, every element, and composite elements
  - an unknown kind, location, or method
  - duplicate keys, top-level and nested inside the internally tagged kind
  - trailing content
- **R3.** `endpoint_literal_surface.rs` builds `Endpoint` and `EndpointParams` literals exactly as they are spelled today. The diff check: `git diff -U0 -- schematic/definitions/src` has 42 insertions and no deletions. They are 18 production literals and 3 registry-test literals, each gaining `default_pagination: None,` and `pagination_exceptions: vec![],`. No endpoint-literal line changed.

#### Requirement-to-test mapping (Phase 2 scope)

| Requirement | Tests |
| --- | --- |
| R1 model, builders, serde, no empty default | `pagination_model::builders_produce_named_kind_structs_with_documented_defaults`, `shared_dimension_setters_finish_the_builder`, `selector_*`, `empty_selector_is_not_a_root_array`, `json_path_*`; `pagination_serde::*`; the new doc examples (doctests) |
| R2 every rule plus its valid sibling | `pagination_model::{cursor_read_only_from_items_needs_an_end_signal, short_page_signal_rules, page_totals_end_only_page_numbers, page_size_bounds_must_be_consistent, one_destination_may_be_written_once, empty_names_and_locations_are_rejected, body_inputs_need_a_request_with_a_body, next_link_on_a_non_get_endpoint_states_its_follow_up_method, next_link_follows_the_link_so_it_has_no_follow_up_path, carried_names_and_step_paths_must_belong_to_the_endpoint, composite_and_inclusive_cursor_rules, adjustable_page_size_rules, partition_maximum_is_positive, session_handle_rules, session_steps_use_endpoint_path_params_and_bodies_only_where_allowed}`; `pagination_tables::{alternates_are_validated_and_resolved_but_add_no_knobs, a_near_miss_and_an_unadoptable_input_collide, a_declared_parameter_without_its_knob_is_refused_and_kept}` |
| R2a (define part) | `pagination_tables::a_required_managed_parameter_needs_a_value_to_send` (required size without/with `sent_default`, required page, required cursor) |
| R3 | `endpoint_literal_surface::endpoint_and_params_literals_compile_unchanged_and_behave_the_same`; the definitions diff above |
| R4 | `pagination_tables::{get_vec_is_a_list_that_inherits_the_default_or_is_unpaginated_without_one, only_get_with_an_explicit_vec_is_recognized, a_row_decides_before_inference, a_wrapper_row_without_a_default_is_an_unpaginated_list, root_array_item_type_reads_only_an_explicit_vec}` |
| R5 | `pagination_tables::{unknown_row_id_is_rejected_with_the_nearest_endpoint, duplicate_row_id_is_rejected_and_the_first_row_decides, row_list_shape_rules, an_unpaginated_row_needs_no_item_type}`; `pagination_validation::item_location_is_checked_against_the_registered_response_schema` |
| R6 (model part) | `pagination_tables::{alternates_are_validated_and_resolved_but_add_no_knobs, an_alternate_constant_may_shadow_a_declared_parameter}`; `pagination_inventory::gitlab_page_number_default_with_keyset_alternate_modeled_only` |
| R7 (model part) | `pagination_tables::{a_declared_parameter_with_its_knob_is_adopted, a_declared_parameter_without_its_knob_is_refused_and_kept, a_near_miss_and_an_unadoptable_input_collide, partition_inputs_are_adopted_with_their_knob}`; `pagination_inventory::{bitbucket_follows_body_next_and_hides_no_page_number, openai_cursor_without_resume_cannot_adopt_after}` |
| R8 (model part) | `pagination_tables::legacy_pagination_metadata_does_not_change_resolution` (resolution, knobs, validation, and adoption identical with and without legacy metadata; legacy alone makes no list) |
| R26 (define and gen part) | `pagination_tables::row_list_shape_rules` (missing item type, inferred `T`); `pagination_validation::{item_type_must_be_rust_type_syntax, item_types_resolve_through_the_registry_including_nested_schemas}` (registered nested, qualified, primitive accepted; unknown rejected) |
| Gen wiring | `pagination_validation::{each_rule_group_renders_as_its_feature_on_the_rust_client_only, a_pagination_error_blocks_only_the_rust_client, live_definitions_report_no_pagination_diagnostics}` |

Deferred by the plan: the generator half of R2a, R6, R7, and R8 (struct fields, `new()`, byte-identical constants) is Phase 3.2; the runtime half of R2a is Phase 4.

#### Input robustness

Phase 1 recorded that the configuration-reader matrix does not apply. The strict serde tests above stand in its place. Grep for load-bearing smells in `define/src/pagination/`: no `#[serde(default)]`. The two `unwrap_or_default()` calls are not parse fallbacks. One reads an endpoint with no declared parameters as having no query parameters, which is that type's meaning. The other is `caller_knobs` of a non-paginated resolution, which is empty by contract.

### Validation run in this phase (macOS, rustc 1.98.1)

| Check | Result |
| --- | --- |
| `just test` (schematic area) | pass: L1 2017/2017 (12 skipped), fresh-output 6/6, compile-fail 1/1 |
| `just lint` (schematic area) | pass, after two fixes: `#[allow(clippy::large_enum_variant)]` with a reason on `Pagination` and `Resolution` (precedent in `gen/src/model_gen.rs` and the OpenAI types), and a type alias in a test |
| `just _doctest schematic-define --all-features` / `schematic-gen` | pass: 174 / 18 |
| Fresh generation (`cargo run -p schematic-gen -- generate --api all ...` into `/tmp/pagination-phase2.*`) | `schematic/schema` and its `Cargo.toml`, OpenAPI, and Postman are all byte-identical to the committed output; the 27 expected `query-in-path` warnings are unchanged |
| `just test-schema` / `just lint-schema` | not re-run: the generated crate is byte-identical and its dependencies are unchanged (Phase 1 baseline pass reused) |
| Other OSes | not run. This phase adds pure data-model and string logic, with no paths, processes, timing, or terminal use. CI's pull-request Linux and macOS cells cover it. |

No pre-existing failures were found.

### Deviations in this phase

- `ResolutionOrigin::Unauthored` is the spec's "None" (R4); it was renamed to avoid `Option::None`.
- `AdoptionWithoutKnob` is a separate rule; the spec folds it into `CollidesWithEndpointParam`.
- Pagination diagnostics use three `Feature` ids, with the rule id in the message, rather than one feature per rule.
- R5 "Remove only strategy-local trust validation" is done. `InvalidTrustedOrigin` is the only rule dropped.
- Runtime-only model helpers (JSON lookup, boundary tracker, terminal check) were not added to define; they go to the runtime in Phase 3.
- R1 "every authoring example in design.md sections 5 and 9 compiles against the real API" is met through the ported fixtures. The design examples use the prototype's endpoint-level `.collection(..)`; they are rewritten as exceptions rows, and the prototype names `Method`, `NextLink { trust }`, and struct-variant kinds became `RestMethod`, no trust, and named structs.
- The `docs/` tree and README were touched now, ahead of Phase 8. `schematic/docs/generation/validation.md` gained the three features and a "Pagination rules" section, because the docs tree is the current record of validation. `schematic/define/README.md` replaced its legacy pagination how-to, which presented the superseded types as the way to paginate, with the model, and its 4 `RestApi` examples gained the two fields. Phase 8.1 still owns the full docs pass.

### Human review agenda (Phase 2 updates)

1. **Defaults.**
   - P8 item 3: a list of rows; reject duplicates and unknown ids with a nearest-id hint, in the landed diagnostic format.
   - P8 item 5 (S12): named structs.
   - P8 item 7: alternates are modeled and validated, and add no knobs.
   - Default 12: adopt-and-hide only with the knob.
   - Defaults 13 and 22: `item_type: Option<String>`, inferred only from an explicit `Vec<T>`.
   - Default 14: GET `Vec<T>` opt-outs use `Collection::root().unpaginated()` rows.
   - Default 15: legacy kept, documented as superseded.
   - Default 16: trust removed.
3. **Design choices.** See 2.1 and 2.2: type names, `Unauthored`, `sent_default`, three features, the default reported once, and the knob order.
11. **Open questions (new).**
    - Should an alternate's declared-parameter collisions be validated once alternates are generated? Recommended: yes, at that time.
    - Should a required partition input get an authored default? Recommended: no until a real API needs one.
    - The nearest-id threshold is max(2, len/3). Recommended: keep.
12. **Drift.** The `define/README.md` pagination section and table, and the `EndpointParams` field and `with_pagination` docs, presented the legacy types as live. They were corrected to say "superseded" (code assumed correct). The `Paginated` marker doc in gen is still Phase 3's.
14. **Unratified.** S12 named structs were adopted. The others are not applied yet.
15. **Adoption rule.** It is implemented and tested in the model. The real adoptions are listed in Phase 6 and 7.
17. **Wrapper item type default.** Implemented as authored `item_type`, checked by `syn` and the registry, with nested schemas counted.

## Phase 3

Phase 3 builds the runtime pager (task 3.1), its generator emission (3.2), and the core runtime fixtures (3.3). Started 2026-10-13 on `feat/modern-config`. Entries are appended as work lands. Task 3.1 was written first so its contract could be frozen; 3.2 and 3.3 then ran concurrently in two delegated workers with disjoint file ownership (`schematic/gen/**` versus `schematic/runtime/tests/**`, with runtime bug fixes reported back).

### 3.1 Pager mechanics (runtime)

New module `schematic_runtime::pager` (`schematic/runtime/src/pager/`):

| File | Holds |
| --- | --- |
| `mod.rs` | `Paginated`, the kind and session marker types, the capability traits, `CursorValue` |
| `spec.rs` | the `&'static` projection of the define model (`Strategy`, `Kind`, `Input`, `Source`, `MoreSignal`, `CursorLeg`, `LinkSource`, `Carry`, `FollowUp`, `Session`, `Partition`, `ItemSelector`, ...) |
| `engine.rs` | the sans-I/O traversal state machine: request planning, end detection, continuation extraction, loop history |
| `json.rs` | path lookup, record fan-out, continuation/flag/total interpretation |
| `link.rs` | the RFC 8288 `Link` parser (quoted parameters, multiple fields, multiple relation tokens) |
| `progress.rs` | `History` (exact keys) and `BoundaryTracker` (ported from the prototype) |
| `request.rs` | `PageRequest<R>`, the internal endpoint a page is sent as; `PageDecoder` (the decode seam); follow-up building |
| `pages.rs` | `Pager`, `Pages`, `Items`, `Call::pager`, `fetch_page`, `collect_all` |
| `error.rs` | `Page<T>`, `PagerError`, `PagerErrorKind`, `ContinuationProblem`, `Incomplete`, `IncompleteReason` |

Design choices (agenda items 3 and 14), each with its alternative and reversal:

- **One generic pager (R9, Proposed; adopted).** `Pager<'a, A, R>` configures and `Pages<'a, A, R>` iterates for every kind; the engine is not generic over the endpoint. Alternative: per-endpoint pager types. Reversal: wrap the engine in generated types.
- **Capability encoding (R10, Proposed; adopted with one change).** Each control is a method with its own `where R: <Capability>` bound and `#[diagnostic::on_unimplemented]`. Kind- and session-derived capabilities (`StartPage`, `StartOffset`, `Sessionless`, `Sessioned`) are blanket impls over the associated types `Paginated::Kind` and `Paginated::Session`, with `#[diagnostic::do_not_recommend]`. The other capabilities (`PageSizeKnob`, `ResumeAfter`, `ResumeBefore`, `HasPrev`, `HasTotal`, `AdjustablePageSize`, `KeepAliveKnob`, `Partitioned`, `Renewable`) are generator-emitted empty marker impls, one per knob in `caller_knobs`. `ResumeAfter`/`ResumeBefore` carry `type Cursor` (`String`, or a tuple of up to four `String`s for a composite cursor, R15). Reversal: blanket more of them from constants once Rust can bound on const values.
- **`.pages()` is absent on a session endpoint** through the `Sessionless` bound, so R22's "no `pages()` outside the scope" is a compile error now; `with_session` arrives in Phase 5.
- **Entry points (default 10).** `Call::pager()` is the runtime entry; generated call types forward `pager`, each authored knob, `pages`, `fetch_page`, and `collect_all` to it. Knobs return the `Pager`, so a knob ends the endpoint's own setter chain: endpoint setters come first, as they already do before `.full()`. `fetch_page` and `collect_all` return the runtime's boxed `CallFuture`, so a generated `#[inline]` forward compiles no per-endpoint state machine (the same reasoning as the generated `IntoFuture`). `Pager` also forwards `anonymous`, `credential`, and `deadline`, so a per-call setting survives into the pager in either order (the sniff ledger entry needs `.anonymous()`).
- **Pages are ordinary typed sends.** A page is sent as `PageRequest<R>`, a runtime-internal `EndpointSpec` that keeps `R`'s API, endpoint id, method, failure type, and declared error statuses. Credentials, origin policy, redirects, the deadline, hooks-free decoding, and redaction are therefore the client's own, and an origin refusal or invalid argument reports the listing endpoint. Pre-send refusals (size bounds, start below the base, overflow, a zero page limit) are returned by `PageRequest::into_parts`, so they surface as the runtime's own `TransportError` of kind `InvalidArgument` with request metadata and zero requests sent (D24, default 9). A followed link is checked with `Client::check_link::<R>` and then sent through the checked absolute-URL path (`RequestParts.path` holding the URL), per the runtime-envelope ruling.
- **Final-URL seam (Phase 1 correction 3).** `Client::send_located` (crate-private) returns the envelope plus the unredacted, pre-credential URL of the hop that received the final reply; `send_unboxed` now delegates to it. Relative links resolve against that URL with `Url::join` (RFC 3986), so `?page=2` and `../next` keep the endpoint path.
- **`RequestMeta.page`** is the page's 1-based position in its traversal, set on the envelope of every page send and on a page's transport error; ordinary sends leave it `None`. Alternative: the provider's page number, which cursor and link strategies do not have.
- **Deferred continuation failures.** A page that decodes but whose continuation is missing, unusable, or repeated is delivered; the failure is stored as that direction's next position and returned by the next request, before anything is sent. This is how R21's "items from a successfully decoded page are retained even when its continuation is invalid" holds for both `next()` and `collect_all`. A malformed collection or item decode failure is the envelope's `MalformedSuccess` with the raw body and no items (R17).
- **Failed stays failed.** `PagerError` holds non-`Clone` evidence (`ApiError<E>`), so a later call returns `PagerError::AlreadyFailed { first: PagerErrorKind }` without a request rather than a clone of the first error. Deviation from R14's "returns the same failure"; the kind is the same and no request is sent.
- **`PagerError` shape.** `Transport`, `Api { error, meta }`, `MissingContinuation`, `InvalidContinuation { problem }`, `RepeatedContinuation`, `BoundaryNoProgress`, `EmptyPageClaimsMore`, `DeadlineExceeded { completed_pages, error }`, `AlreadyFailed`. `EmptyPageClaimsMore` is the R12 "no-progress" contradiction of an empty page-number or offset page whose signal says more; `ContinuationProblem::{Cursor, Link, AmbiguousLink, MoreSignal, Total}` distinguishes unusable values. Page size bounds are not a pager variant: they are the runtime's `InvalidArgument` kind (Phase 1 correction 11).
- **`IncompleteReason` mapping (P8 item 1, R21).** The spec's five plus `InvalidContinuation`, `InvalidArgument`, `TransportError`. Missing or invalid continuation and a malformed end signal or total map to `InvalidContinuation`; repeated continuation, boundary no-progress, and an empty page claiming more map to `RepeatedContinuation`; a `RedirectFailed` envelope maps to `TransportError`; other envelope failures to `HttpError`; outer transport kinds to `CrossOriginRejected`, `InvalidArgument`, `DeadlineExceeded`, or `TransportError`.
- **Page limit (P8 item 2).** `DEFAULT_PAGE_LIMIT = 100`; `Pager::page_limit(n)` sets it and `configured_page_limit()` reads it. It counts page requests including failed ones. After the last permitted page, a proven end returns `Ok`; a deferred continuation failure is reported as that failure (no request is needed to know it); otherwise `PageLimitReached`.
- **End detection (R12).** An explicit signal decides when it is present: `Flag`, `Present`, `LinkRel`, and `Total`; a flag that is absent counts as no signal (cursor falls back to continuation presence, page number and offset to a non-empty page), and a flag of the wrong type is `InvalidContinuation { MoreSignal }`. `Total` decides only when the covered amount is known (page number needs an effective size for an item total); otherwise it falls back like a missing total. `FullPage` uses the effective size (caller's, else `sent_default`) and otherwise falls back to a non-empty page, so `server_default` alone never establishes it. A cursor value that is a number or boolean is a value; arrays and objects are values only for a JSON body destination (`search_after`).
- **Stall rule (R14, P8 item 6; Proposed, adopted).** Every continuation sent forward is kept exactly, per direction, including the caller's starting cursor; any repeat is `RepeatedContinuation`. Links are keyed by their resolved URL. An inclusive forward boundary uses `BoundaryTracker` instead of the history.
- **Size semantics (R13, R2a).** An unset optional size is omitted; `sent_default` is sent when the caller sets none; `server_default` is never sent. A followed link carries the provider's size unless the caller adjusted it during the traversal (R16 "a provider-issued size is not changed implicitly").
- **Follow-up shape (R16).** Non-link follow-ups keep the method and path unless `FollowUp` names them; a follow-up path template is filled from the endpoint's own path values by matching `Paginated::PATH` segment by segment. The carry rule filters query pairs and top-level JSON body fields; managed inputs are written last and replace any value. A followed link is sent verbatim with GET unless carry names values, which are set on its query replacing existing ones. Every `RequestBody` variant owns its bytes, so no body is non-replayable today. Phase 4.2 owns the acceptance tests of these rules.
- **Decode seam (R24, P8 item 4).** Provisional: the page body parses to `serde_json::Value` (`PageDecoder`) and items decode through `serde_json::from_value` (`request::decode_items`). Phase 4.1 compares this with a shared `from_slice` path; the seam is these two functions. Response hooks do not run on page bodies (the envelope's `Json` hooks are typed by the endpoint's success type, not a page); recorded for Phase 4.
- **ABI 3 → 4** (Phase 1 correction 9): `schematic_runtime::ABI` and the generator's `RUNTIME_ABI` change together.
- **Not built in this phase, by the plan:** session scope (`with_session`, keep-alive, renew, close), partitions (Phase 5); the shared deadline acceptance, origin-policy matrix, and decode measurement (Phase 4). The projection already carries sessions and partitions, so generated constants need no change later.

### 3.2 Pager emission (generator)

- **Emitter.** `schematic/gen/src/codegen/pagination/mod.rs` replaces `codegen/traits.rs` (deleted). For each endpoint that resolves `Paginated`, it emits `impl schematic_runtime::pager::Paginated` with `type Item` (the resolved item type through the response-type tokenizer), `type Kind`, `type Session`, the `&'static` `STRATEGY` and `ITEMS` constants, and `PATH`. It also emits one marker impl per knob in `caller_knobs`: `ResumeAfter`/`ResumeBefore` with `type Cursor` sized to the leg's parts, `PageSizeKnob`, `AdjustablePageSize`, `KeepAliveKnob`, `Partitioned`, `HasPrev`, `HasTotal`, and `Renewable`. Nothing is emitted for unpaginated lists, non-lists, or alternates. Paths are fully qualified, so the generated modules gain no `use`. A define variant the projection does not know (the enums are `#[non_exhaustive]`) emits `compile_error!`, following the existing `method_tokens` precedent. So does a paginated list without an item type, which validation already rejects.
- **Example (exact snapshot test `page_number_strategy_emits_its_projection_markers_and_forwards`):** a GitHub-like `ListReleases` emits `type Kind = PageNumberKind`, `Kind::PageNumber { page: Input::Query("page"), base: OneBased, more: MoreSignal::LinkRel { rel: "next" } }`, `size: Some(PageSize { param: Input::Query("per_page"), server_default: Some(30), sent_default: None, min: 1, max: Some(100), adjustable: false })`, `ITEMS = { record: &[Step::Each], projection: &[] }`, and `impl PageSizeKnob`. Its call type gains `pager`, `page`, `page_size`, `pages`, `fetch_page`, and `collect_all`, all `#[inline]`. The request struct has no `page`/`per_page` field, `with_*` method, or setter.
- **One adoption decision (R7, R2a).** `pager_view(api, endpoint)` removes exactly `adopted_params(endpoint, strategy)` (query and header) from the endpoint as the Rust client sees it. Request-struct emission, call-type setters and docs, and `resolve_names` read it. Name validation calls `resolve_names`, so validation and emission share it. The OpenAPI and Postman exporters do not use it, so plain parameters stay in the exports.
- **Forwards (default 10).** Paginated call types forward `pager()` and each authored setter-like knob. Without a session they also forward `pages()`, `fetch_page()`, and `collect_all()`. There are no `keep_alive`, `partitioned`, `with_session`, or `renew` forwards yet (Phase 5). Paginated call types get one extra doc paragraph; unpaginated output is unchanged.
- **Names.** `PLANNED_CALL_METHODS` now reserves `pager` beside `pages`, `fetch_page`, and `collect_all` on every call type; no shipped definition has a `pager` setter. The knob forwards (`page`, `offset`, `after`, `before`, `page_size`) are reserved per endpoint, only where that endpoint forwards them. A remaining setter of that name gains `_param` under the new `RenameReason::PagerMethod` and is reported as the existing rename warning, so an unpaginated endpoint's setters never change.
- **Legacy (R8, R25).** The `Paginated` marker trait in the generated `shared.rs`, its 35 impls, the `use crate::shared::Paginated` imports, `has_paginated`, and `"Paginated"` in `IMPORTED_TYPE_NAMES` are removed. `EndpointParams.pagination` no longer reaches the generator.
- **ABI.** `RUNTIME_ABI` is 4, matching the runtime (checked by `the_generator_emits_the_abi_of_the_workspace_runtime`).
- **Fresh-output fixture.** `fresh_output_compile.rs` gains a `FixturePages` API with page-number, cursor, and next-link endpoints; the cursor and next-link ones are wrapper lists. A `PAGER_CONTROL` program calls `.state(..).page_size(10).pages()`, `.after("x").fetch_page()`, `.collect_all()`, and `.page(2).page_limit(3).collect_all()` on the generated crate. Two compile-time misuses are asserted with their diagnostics: `.pager()` on an unpaginated endpoint and `.page_size` on the next-link endpoint. Both reuse the existing fixture crate builds. `tests/common/fresh_output.rs::doctored_runtime` now copies directories recursively, because the runtime gained `src/pager/`.
- **Regeneration (pre-authorized; agenda 13).** `just generate` was run in this phase, not Phase 7, because the ABI bump would otherwise leave the committed `schematic-schema` failing its compile-time ABI assertion between phases. `git diff --stat -- schematic/schema schematic/openapi schematic/postman`: 6 files, +2/−60. That is `lib.rs` (ABI 3 → 4), `shared.rs` (the marker trait), and `bitbucket`/`gitea`/`github`/`gitlab` `requests.rs` (35 `impl Paginated` lines and 4 imports). OpenAPI and Postman are byte-identical. No definition authors pagination yet, so no request field was removed. The 27 `query-in-path` warnings are unchanged.

### 3.3 Core fixtures (runtime tests)

New files under `schematic/runtime/tests/l1/`, all declared in `main.rs`; every test is L1 and untiered:

- `pager_support.rs`: 37 endpoint fixtures built like generator output by an `endpoint!` macro. Each has `&'static` constants and only its authored markers. A `Copy` builder `Def` assembles the strategy constants, because struct-update syntax cannot be used on `Strategy` in a constant: `follow_up` and `session` are not `Copy`.
- `pager_mechanisms.rs` (17), `pager_outside.rs` (10), `pager_rules.rs` (32), `pager_surface.rs` (11).
- `#[cfg(test)]` units in `src/pager/json.rs` (7), `progress.rs` (5), `link.rs` (3), and `request.rs` (3). They port prototype cases g2 (boundary tracker), g4 (selector fan-out, including two-level `responses[*].hits.hits[*]._source` in document order), p01/p05 (lookups), and g6 (terminal values).
- 11 compile-fail fixtures `tests/fixtures/compile_fail/pager_*.rs` with `.stderr`. The snapshots hold only crate-relative paths; the harness clears `RUSTC_WRAPPER`.

Every mechanism runs through the real runtime against a local wiremock server, not only through extraction helpers. The slowest pager test takes 27 ms.

**Bug found and fixed:** stepping `next, next, prev, next` on a bidirectional next-link pager returned `RepeatedContinuation`, because the forward history still held the page-2 link. R14 says a deliberate backward step is not a forward cycle. A page accepted in one direction now clears the other direction's history (`engine.rs`, `History::clear`). Regression test: `pager_rules::stepping_back_then_forward_again_is_not_a_repeated_continuation`. It fails without the fix (checked by reverting the two lines) and passes with it. The same test proves that a cycle inside one forward run after turning around is still caught.

#### Requirement-to-test mapping (Phase 3 scope)

| Requirement | Tests (boundary, fidelity) |
| --- | --- |
| R9 one generic pager; thin forwards | runtime fixtures shaped like generated impls (all `pager_*` tests); gen `page_number_strategy_emits_its_projection_markers_and_forwards`; fresh-output `slow_fixture_apis_compile_and_send_offline` (crate boundary, F3: generated crate compiled against the workspace runtime) |
| R10 unauthored knobs do not compile | 11 `pager_*` compile-fail cases (page_size, page, offset, after, before, prev, total, set_page_size, `.pager()` on unpaginated, knob after `.pages()`, `pages`/`fetch_page`/`collect_all` on a session endpoint); positive: `pager_surface::{every_control_is_available_on_a_fully_authored_endpoint, page_number_and_offset_endpoints_take_their_start_and_size_controls}`; generated surface: `slow_generated_types_reject_misuse_at_compile_time`. The real generated endpoint set runs in Phase 7 (R31). |
| R11 iteration surface | `pager_surface::{each_page_carries_its_number_in_the_page_and_the_request_metadata, items_flattens_every_page_in_order}`; `total`, `is_finished`, `pages_fetched`, `items_delivered` asserted in the mechanism tests |
| R11a `fetch_page` | `pager_surface::{fetch_page_sends_one_request_with_the_cursor_and_size_and_returns_that_page, an_ordinary_send_of_a_paginated_request_returns_the_wrapper_without_a_page_number}`; fresh-output `PAGER_CONTROL` program |
| R12 end detection | `pager_rules::{an_explicit_false_ends_despite_a_stale_cursor, an_explicit_true_without_a_cursor_is_a_missing_continuation_after_the_page, without_a_signal_a_missing_null_or_empty_cursor_ends, zero_and_false_cursors_are_values_not_ends, a_flag_of_the_wrong_type_is_an_invalid_more_signal_never_completion, a_true_means_last_flag_ends_when_true, an_empty_page_number_page_that_claims_more_is_an_error, an_empty_cursor_page_with_a_continuation_continues, a_missing_total_falls_back_to_a_non_empty_page_for_offsets, a_malformed_or_negative_total_is_an_invalid_total, a_server_default_is_never_sent_and_does_not_establish_a_full_page, page_number_arithmetic_is_checked_before_the_next_send, a_start_page_below_the_base_is_refused_without_a_request, an_offset_advances_by_source_records_from_its_zero_base}`; SCIM 1-based and both 0-based mechanism tests; `json` unit tests for terminal values |
| R13 / R2a (runtime half) size rules | `pager_rules::{a_size_outside_the_declared_bounds_is_refused_naming_the_bound_with_zero_requests, an_adjusted_size_applies_from_the_next_request_and_is_validated, an_optional_size_is_omitted_when_unset, a_full_page_rule_uses_the_sent_default_size_on_every_request_unless_overridden}`; generator half: `pagination::tests::{adopted_parameters_leave_the_request_and_the_call_type, only_adopted_required_parameters_keep_default}` |
| R14 stall and loop | `pager_rules::{a_cursor_cycle_is_a_repeated_continuation, a_return_to_the_starting_cursor_is_a_repeated_continuation, a_repeated_link_is_a_repeated_continuation, a_failed_pager_stays_failed_without_another_request, stepping_back_then_forward_again_is_not_a_repeated_continuation}`; `progress::tests::the_history_keeps_every_key_it_saw` |
| R15 composite and inclusive | `pager_rules::{a_composite_cursor_sends_every_part_and_ends_when_all_are_absent, a_composite_cursor_with_only_some_parts_is_a_missing_continuation, an_inclusive_boundary_drops_redelivered_ties_and_fails_on_a_page_of_only_ties}`; 4 tracker units |
| R16 (partial; Phase 4 owns acceptance) | AIP-158 `AllExcept(["skip"])`; verbatim link following in the keyset, OData, and Salesforce REST tests; constants on every non-link request; `request::tests::{follow_up_path_takes_the_endpoint_path_values, link_query_values_are_replaced_and_others_kept_in_order, body_inputs_create_nested_objects}` |
| R17 item location | `pager_rules::{a_malformed_collection_is_a_malformed_success_with_its_body_and_no_items, a_record_without_its_projection_is_malformed, a_malformed_second_page_contributes_no_items_to_collect_all}`; `json` selector units |
| R18 (Phase 3 part) | `pager_surface::{relative_links_resolve_against_the_response_url, distinct_next_links_are_ambiguous_and_identical_ones_are_followed, a_cross_origin_next_link_is_refused_before_any_request_reaches_it}`; `link::tests` (quoted commas, multiple relation tokens, malformed fields). The two-origin policy matrix is Phase 4.4. |
| R21 (partial) | `pager_rules::{the_default_page_limit_is_one_hundred_and_zero_is_refused_without_a_request, completion_exactly_at_the_page_limit_is_complete, reaching_the_page_limit_first_is_incomplete_with_the_items_so_far}`; reasons `InvalidContinuation`, `InvalidArgument`, `HttpError`, `RepeatedContinuation`, and `CrossOriginRejected` asserted in the rule and surface tests, including the items of a valid page kept when its continuation is invalid. `DeadlineExceeded`, `TransportError`, and `RedirectFailed` are Phase 4.3. |
| R25 generated surface | gen `pagination::tests::{unpaginated_lists_and_non_lists_emit_nothing_pagination_related, repeated_generation_is_identical, session_strategy_has_markers_but_no_outside_scope_iteration, composite_inclusive_cursor_with_backward_leg_and_total, next_link_with_body_prev_on_a_wrapper_list, one_based_offset_strategy}`; `e2e_generation::shared_module_defines_no_pagination_trait`; the regeneration diff |
| R26 (gen part) | `pagination::tests::wrapper_item_types_are_emitted_as_authored` (`serde_json::Value`, a qualified path, `Vec<String>`) |
| R6 / R8 independence | `pagination::tests::{alternates_do_not_change_the_output, legacy_pagination_metadata_does_not_change_the_output}`; `e2e_generation::legacy_pagination_metadata_emits_no_pager` |
| Names | `pagination::tests::{a_setter_named_like_a_pager_control_is_renamed, an_unpaginated_endpoint_reserves_only_the_pager_entry_points}` |
| R30 mechanism matrix | `pager_mechanisms.rs`: GitHub page + Link, GitLab `X-Next-Page` + `X-Total` and keyset Link, Gitea page/limit + `X-Total-Count`, Bitbucket body next + previous + size, OpenAI S6 / item-id / S7 / S8, Anthropic bidirectional with `.before`, ElevenLabs history / voices token / shared voices 0-based, EMQX nested `meta.hasnext` + `meta.count`, Hugging Face Link cursor and 0-based `p`. `pager_outside.rs`: AIP-158, Mastodon, SCIM, Salesforce Bulk 2.0, and `unverified_{x_api_v2, wordpress, odata_skiptoken, cosmos_db, salesforce_rest_query, reddit}` |
| R36 privacy | `pager_surface::{a_repeated_secret_cursor_is_not_in_the_error_text, an_invalid_secret_continuation_is_not_in_the_error_text}` (cursor, link, ambiguous links, `Incomplete` text) |

Input robustness: this phase adds no configuration reader. The pager reads provider response bodies, which the spec and Phase 1 record as outside the matrix. Their shapes are covered by the R12 and R17 response-shape tests above: absent, null, wrong type, empty, and malformed values each have a defined outcome. Smells grepped in `runtime/src/pager/`: no `#[serde(default)]`. `unwrap_or_default` and `filter_map(..as_str())` are not used on load-bearing reads. The `.ok()` in `decode_items` makes the whole page malformed when any item fails, so it never filters out an element.

### Validation run in this phase (macOS, rustc 1.98.1)

| Check | Result |
| --- | --- |
| `just test` (schematic area) | pass: L1 2113/2113 (12 skipped), fresh-output 6/6, compile-fail 1/1 (now 21 cases) |
| `just lint` (schematic area) | pass |
| `just _doctest schematic-runtime` / `schematic-gen` | pass: 1 / 18 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p schematic-runtime --no-deps` | clean |
| `just generate`, then `git diff --stat` of schema/openapi/postman | 6 schema files, +2/−60; OpenAPI and Postman unchanged |
| `just test-schema` / `just lint-schema` | pass: 40/40 / clean |
| `cargo check --all-targets` of the seven `schematic-schema` dependents (sniff, biscuit-speaks, homelab, eversolo-integration, arcam-amp-integration, sony-receiver-integration, unfolded-integration-helper) | pass. Their test suites were not run: the regeneration removed only an unused trait and changed the ABI value, and Phase 7.3 runs them after the real field removals. |
| `just cross-check schematic-runtime --os linux` | pass: 288 run, 1 skipped (the trybuild test, which runs through its own recipe) |
| Windows, WSL2 | not run here; Phase 8.2 owns platform evidence. Risk is low: the pager touches no paths, processes, or timers, and the compile-fail snapshots use crate-relative paths like the existing ones that already run on Windows. |

No pre-existing failures.

### Deviations in this phase

- **`AlreadyFailed` instead of the same error** (R14): explained under 3.1.
- **Explicit marker impls for most capabilities.** Only the kind- and session-derived ones are blanket impls. The spike blanket-implemented only kind-derived ones too, so this matches it; listed because R10 names the encoding as Proposed.
- **Early regeneration** of `schematic-schema` (ABI and legacy marker only), forced by the ABI bump; Phase 7 still owns the regeneration with authored pagination.
- **Salesforce REST `done` flag cannot be authored.** The define model's `NextLinkStrategy` has no "more" signal, so `done` (true means last) cannot end a next-link traversal. The `unverified_salesforce_rest_query` test proves only the relative `nextRecordsUrl` mechanism; `TrueMeansLast` is tested on an offset fixture. Recorded as an open question (agenda 11).
- **Response hooks do not run on page bodies.** The client's `pre_response_json` hook is not applied to a page document. Recorded for Phase 4's transport work.
- **Follow-up path filling** was implemented here (a small segment match against `Paginated::PATH`) rather than in Phase 4.2, because the constant had to exist for generated code either way.

### Human review agenda (Phase 3 updates)

1. **Defaults.** P8 item 1: eight reasons with the mapping under 3.1. P8 item 2: a page limit of 100, `page_limit`/`configured_page_limit`, which counts attempts. P8 item 6: widened stall rule, histories per direction and reset on a direction change. Default 9: knobs are infallible and bounds are checked before the send, as `InvalidArgument`. Default 10: thin forwards on call types. Default 19 is not exercised yet (Phase 4).
3. **Design choices.** See 3.1 and 3.2: runtime names (`Pager`, `Pages`, `Items`, `Page`, `PagerError`, `PagerErrorKind`, `ContinuationProblem`, `Incomplete`, `IncompleteReason`, `DEFAULT_PAGE_LIMIT`, `PageResult`, `CollectResult`); the `Paginated` associated types and constants; the generated constant layout (snapshot above); entry names `pager`, `pages`, `fetch_page`, `collect_all`, `page_limit`.
4. **Removed request fields.** None yet: no definition authors a strategy. The mechanism is tested.
8. **Compile-time observations.** `just lint-schema` finished in 7.0 s warm after regeneration. The schema output is unchanged apart from 62 lines, so a cold before/after comparison is left to Phase 7 with authored pagination. The `on_unimplemented` messages show in the compile-fail snapshots (for example "`PlainList` is not paginated", "has no authored page size").
9. **OS.** macOS full; Linux runtime cross-check; Windows and WSL2 are Phase 8.2. The trybuild snapshots hold crate-relative paths only.
10. **Deviations.** See above.
11. **Open questions (new).**
    - Should `NextLinkStrategy` gain an optional "more" signal (Salesforce REST `done`)? Recommended: yes, in the definition-vocabulary work, once a verified provider needs it.
    - Should re-traversal after `prev()` be allowed (implemented), or should a pager be one-directional per run? Recommended: allowed, as implemented.
12. **Drift.** Resolved by assuming the code is right:
    - The `Paginated` marker's doc described a nonexistent `fetch_all_pages`; the marker and its doc were deleted.
    - `RequestMeta.page`'s doc said the field was reserved for planned pagers; it now describes the page position.
    - The gen README's and `docs/generation/partial-generation.md`'s descriptions of the marker were replaced.
    - `docs/generation/validation.md`'s reserved-name table gained `pager` and the per-endpoint pager controls.
13. **Pre-authorized actions.** Regenerated `schematic-schema` (+2/−60, listed above), ahead of Phase 7.
14. **Unratified.** R9/R10 adopted as described. R14 widened, plus a reset of the opposite history on a direction change. S11 is provisionally `from_value` behind `request::{PageDecoder, decode_items}`.
17. **`InvalidContinuation` and the added reasons.** Implemented as listed in 3.1; the Phase 3 tests above cover five of the eight reasons.
18. **Stall-rule widening.** No test depended on consecutive-only detection. Walking back and forth through a bidirectional list repeats a continuation across directions; that is not treated as a loop.

## Phase 4

Phase 4 integrates typed transport and the complete-list semantics: the decode sample (4.1), the transport gaps left by Phase 3 (4.2), and the traversal and transport acceptance tests (4.3, 4.4). Started 2026-10-13 on `feat/modern-config`. Phase 3 had already routed every page through `Client::send_located`, so 4.2 closes the remaining rules rather than building the transport path.

### 4.1 Decode sample (S11, R24, default 4)

**Method.** The spike's `bloat/gen.py` drives the throwaway `pager-surface` crate, so it was adapted to the landed runtime (scratch copy in `/tmp/pager-bloat/gen.py`, not committed). It writes a standalone crate with one item struct, one `Json<_>` endpoint (the ordinary typed send), and, in `paged` mode, the `Paginated` impl and markers the generator emits per endpoint. It cycles six strategy templates: page number with a `Link` rel and size, cursor from the last item's id with `has_more`, cursor from a body field with size, next link from a body field with size, `Link` header next and prev, and offset with an item total. Half the templates use a `{data: [...]}` wrapper and half a root array. The binary sends every endpoint once and, in `paged` mode, runs `collect_all` on every pager, so each `Pages<_, _, R>` is monomorphized. The count is **94 endpoints**, Phase 1's expected paginated count, not the spike's 157 list candidates, because only paginated endpoints get a pager. Both modes use the workspace `Cargo.lock`, the default `release` profile, and `CARGO_TARGET_DIR=/tmp/pager-bloat/target`, with `RUSTC_WRAPPER` cleared.

**Candidate.** Items were decoded with `serde_json::from_slice` from their own text in the body. A non-generic walk over `&RawValue` spans (serde_json `raw_value`) mirrored `json::records` and `json::project`, and the engine returned the kept record indices instead of cloned values. Projection, continuation access (still from the parsed `Value`), envelope evidence, and one-pass item delivery were unchanged; the runtime's 289 L1 tests passed with it.

**Host.** macOS (Darwin 27.2.0, Apple Silicon), rustc 1.98.1, cold scratch target directory, no compiler cache.

| Binary | `from_value` (Phase 3) | `from_slice` candidate |
| --- | --- | --- |
| `base` (ordinary sends only) | 10,033,536 B | 10,057,872 B |
| `paged` (plus 94 pagers) | 12,670,624 B | 12,726,176 B |
| pager cost (`paged` − `base`) | 2,637,088 B | 2,668,304 B |

**Observation.** The candidate made the binary 55 KB larger, not smaller. `nm` shows why. The landed `Json<T>` codec already instantiates `<Value as Deserializer>::deserialize_struct::<ItemN::__Visitor>` for every success type, because its `pre_response_json` branch decodes through `from_value`, and that instantiation is present in both binaries. The spike's finding that 74% of the added IR is a second item deserializer measured a prototype whose unpaginated decode had no value path, so it does not hold for the landed runtime. The candidate removed nothing and added `from_trait::<SliceRead, ItemN>`, the raw walk, and the `raw_value` feature.

**Chosen path: keep `from_value`** (`request::decode_items`). The reuse that R24 prefers already happens through the codec's value path. The candidate was correct but added a second traversal of the body, a dependency feature, and 55 KB, for no size benefit. The candidate code was removed and the module doc of `request.rs` now records the reason. The 2.6 MB pager cost at 94 endpoints is the pager's own generic code (engine plumbing, `Pages`, futures, and `collect_all`, monomorphized per endpoint). It is logged for agenda item 8 with no adoption threshold, per the plan. A reduction would come from making less of `Pages` generic over `R`, which is outside this phase. Reversal: reapply the candidate if `Json<T>` ever stops decoding from a value; the seam is still `PageDecoder` and `decode_items`.

**Hooks (4.2 decision, recorded here because the decoder carries it).** `PageDecoder` now overrides `decode_hooked`: the client's global `pre_response_json` hook runs on every page body before the engine reads it, because a definition's item and continuation paths describe the body the hook produces, the same body the endpoint's own `Json` codec decodes. A hook refusal is the envelope's `ErrorKind::Hook` with the original bytes, mapped to `IncompleteReason::HttpError`. The endpoint's typed mutator does not run on a page, because it is typed by the endpoint's response, which a page is not. Alternative: run no hook on pages (Phase 3's behavior), which would make a hook-dependent definition page differently from its ordinary send. `hooks.rs`'s module doc now states the page behavior.

### 4.2 Transport integration

Phase 3 already sent every page as `PageRequest<R>` through `Client::send_located`, so pages already had credentials, redaction, the origin policy, redirects, the shared deadline, the page-only `RequestMeta.page`, and no retries. Phase 3's 3.1 entry records that design. This task closed the remaining rules:

- **Decoder:** `from_value` is kept (4.1), and the global JSON hook now runs on pages (see 4.1).
- **Ordinary sends of a paginated endpoint send required adopted defaults (R11a, the runtime half of R2a).** An ordinary typed send cannot see `Paginated`, so the generator writes the defaults into the request's `into_parts`. `schematic_define::pagination::ManagedRole::sent_default` (new, public) gives the value the pager sends when the caller sets none: the first page or offset of the index base, `PageSize::sent_default`, or the keep-alive default. `has_sent_default` now derives from it. `codegen::pagination::ordinary_send_defaults(api, endpoint)` lists the adopted parameters the endpoint declares required, with those values. `output/assemble/api.rs` passes them to the new crate-private `generate_request_struct_with_defaults`; the public `generate_request_struct_with_options` delegates to it with none. `into_parts` appends them after the ordinary query and headers. The pager then writes each managed input over the same name, so a page carries one value per input. Optional adopted inputs are still omitted.
- **Follow-up conflicts (R16).** Destination conflicts between managed inputs are a definition-validation rule (Phase 2). At run time, a managed input replaces any ordinary value of the same name: query by exact name, header without regard to case, and body by exact path segments. That is how a required default from `into_parts` gives way to the pager's value. The plan's "validate conflicts rather than silently overwriting them" is read as the definition-time rule. The define model never adopts a constant or session handle, so an ordinary field cannot shadow one, and a run-time refusal would also reject the required-default case above. Recorded as a deviation.
- **Carry on bodies (R16, "non-replayable bodies").** Every `RequestBody` variant owns its bytes, so no body is non-replayable in the sense of a consumed stream. The real gap was different: `carried_body` re-sent a URL-encoded, multipart, raw, or non-object JSON body whole under an `Only`/`AllExcept` rule, which silently ignored the rule. It now filters URL-encoded pairs and multipart parts by name, as it already did for JSON object fields, and drops a body left with nothing. A body whose fields cannot be named (raw bytes, or JSON that is not an object) is refused with `InvalidArgument` and zero requests. `All` and `Nothing` are unchanged.
- **Followed links are checked after carry (R18).** `check_link::<R>` now runs on the URL as it will be sent, after the carry rule and managed inputs changed its query. Before, it ran on the link as read. The origin cannot change in between, but the rule is now literal. The send path's own absolute-URL origin check still runs, and redirects of a followed link go through the client's redirect policy.
- **Link parsing, relative resolution, verbatim GET following, deadline mapping, and the `IncompleteReason` mapping** were delivered in Phase 3; 4.3 and 4.4 add their acceptance tests.
- **A shared deadline that expires during a page body read** arrives as the envelope's `BodyInterrupted`, because the head had already come, and maps to `HttpError` (R20, R21). Only an expiry before a page's head is `DeadlineExceeded`. Recorded as a known limit; changing it would mean guessing from an expired deadline what cut the body.
- **Integration seam.** `Call::into_inner` (crate-private) gives `Call::pager` the client, request, and per-call options. Generated call types forward to it, so their wrapper field stays private. An invalid request (`into_parts` error) surfaces at the first page as `InvalidArgument` with zero requests, the same refusal an ordinary send gives.

### 4.3 Traversal evidence and 4.4 Transport evidence

Two delegated workers wrote these tests concurrently, each owning one new file: `pager_collect.rs` (4.3) and `pager_transport.rs` (4.4). Before they started, `endpoint!` in `pager_support.rs` was changed to use absolute paths and is now re-exported (`pub(crate) use endpoint;`), so each suite declares its own fixtures without editing the shared file. Every test is L1 and untiered, runs against local wiremock servers or raw loopback sockets, and has no fixed synchronization sleeps; real-time margins are at least 100 ms. The slowest new test takes 0.32 s. Neither worker found a runtime defect.

`pager_collect.rs` (21 tests) is the R21 table plus the R16 request shapes:

- One test per `IncompleteReason`, each asserting the exact items, `pages`, the error variant, and the envelope evidence where a response arrived:
  - `page_limit_reached_keeps_every_page_counts_an_empty_one_and_has_no_error`
  - `repeated_continuation_keeps_the_page_that_repeated_and_sends_nothing_more`
  - `http_error_keeps_earlier_items_and_the_failed_page_body`
  - `invalid_continuation_keeps_the_items_of_the_page_that_carried_it`
  - `invalid_argument_is_refused_before_the_request_that_would_carry_it`
  - `transport_error_when_the_next_page_cannot_connect`
  - `transport_error_when_a_redirect_hop_fails_keeps_the_redirect_as_evidence` (an envelope `RedirectFailed` maps to `TransportError`)
  - `cross_origin_rejected_sends_nothing_to_the_other_origin`
  - `deadline_exceeded_keeps_the_pages_that_arrived_in_time`
- Start positions and limits:
  - `collect_all_starts_from_a_resumed_cursor_or_a_start_page`
  - `a_zero_page_limit_is_an_invalid_argument_with_no_request`
  - `the_last_permitted_page_completes_the_list_when_it_shows_the_end_after_an_empty_page`
- Follow-up shapes (R16):
  - `only_carry_sets_the_named_ordinary_inputs_on_a_next_link_and_drops_the_rest` (Shopify `Only`)
  - `a_follow_up_takes_its_method_filled_path_carried_inputs_and_each_cursor_destination` (method, filled path, query and JSON carry, cursor written to the query, to a header replacing a differently cased ordinary header, and to a JSON body path)
  - `all_except_carry_drops_the_named_url_encoded_pairs_and_keeps_repeated_ones`
  - `a_body_the_carry_rule_leaves_empty_is_dropped`
  - `only_carry_keeps_the_named_multipart_parts`
  - `a_raw_or_non_object_json_body_is_refused_for_a_follow_up_after_the_first_page`
  - `a_repeated_ordinary_query_value_stays_repeated_on_an_all_carry_follow_up`
- Managed inputs win:
  - `a_caller_page_size_replaces_an_ordinary_value_of_the_same_name_exactly_once`
  - `without_a_caller_size_the_sent_default_replaces_an_ordinary_value_exactly_once`

`pager_transport.rs` (20 tests) covers R18, R19, R20, R36, and hooks:

- Origins (R18):
  - `a_link_to_a_second_origin_follows_the_policy_and_its_credential_rule`: under the default policy B receives zero requests. Allow-listing B lets it receive the bearer credential, as `origin.rs` documents. Under the any-origin-without-credentials policy B gets no `Authorization`.
  - `under_the_default_policy_every_other_scheme_host_port_and_userinfo_link_is_refused_unsent`
  - `a_relative_link_resolves_against_the_url_page_one_was_redirected_to` (`?page=2` and `../next` after a 302)
  - `a_followed_link_that_redirects_to_another_origin_obeys_the_redirect_policy_and_credential_rule`
- `Link` header parsing (R18):
  - `the_next_link_is_found_across_fields_quoted_commas_and_multiple_relations`
  - `a_malformed_link_header_is_an_invalid_link_after_its_page_is_delivered`
  - `a_link_header_without_a_next_relation_ends_the_list`
- Envelope per page (R20), each with its `collect_all` reason:
  - `pages_are_ordinary_sends_and_an_error_status_keeps_its_status_headers_and_body_without_a_retry`
  - `a_page_that_is_not_json_is_a_malformed_success_with_its_bytes`
  - `a_page_body_cut_off_after_its_head_is_a_body_interruption`
  - `a_page_redirected_to_a_refused_origin_is_a_redirect_refusal_with_the_redirect_body`
  - `a_page_whose_redirect_hop_fails_keeps_the_redirect_as_evidence`
- Deadline and cancellation (R19):
  - `a_deadline_that_expires_while_page_two_waits_reports_the_completed_pages_and_stays_failed`
  - `a_deadline_that_expires_between_pages_sends_nothing_more` (the deadline is set on the call, not the pager)
  - `dropping_a_page_in_flight_closes_its_connection_and_sends_nothing_later` (raw socket: the server observes EOF, and a control request proves no later request arrived)
- Privacy (R36):
  - `a_refused_link_carrying_a_secret_cursor_keeps_it_out_of_the_error_text_and_summaries`
  - `the_client_credential_is_in_no_page_or_pager_error_text_or_debug_output`
- Hooks:
  - `the_global_json_hook_reshapes_every_page_before_items_and_continuations_are_read`
  - `a_global_hook_refusal_is_a_hook_failure_with_the_original_bytes`
  - `an_endpoint_mutator_does_not_run_on_pages` (the same mutator does fail an ordinary send, which shows it was registered)

Observations recorded from the workers:

- **`pages` counts a request refused before sending.** The engine counts a request when it plans it, so a size refusal reports `pages == 1` with nothing sent. This matches `Incomplete::pages`' doc ("page requests attempted, including the one that failed") and R21's "fetch attempts", and the plan's 1.x default counts attempts. Kept.
- **Carry rules do not filter headers.** `Carry` selects query and body names, as definition validation requires (`UnknownCarryParam` accepts only declared query parameters and the page size). Request headers are re-sent on every non-link follow-up, and kept on a followed link. Kept and recorded.
- **The cursor-secret test checks error text and summaries, not the `Debug` output of the inner `TransportError`.** That `Debug` output prints the request URL with the runtime's ordinary redaction, which does not hide a parameter named like a cursor. R36 allows this ("Request URLs and headers use spec 2's redaction").

#### Requirement-to-test mapping (Phase 4 scope)

| Requirement | Boundary and fidelity | Tests |
| --- | --- | --- |
| R24 / S11 decode path | runtime crate, F2 (real typed send) | decode sample above; the existing item-decode tests (`pager_rules::a_malformed_collection_*`, mechanism tests) cover the kept `from_value` path |
| R11a ordinary send with required defaults | define → gen → compiled crate → runtime → HTTP, F3 | gen `pagination::tests::an_ordinary_send_carries_required_adopted_defaults_and_omits_optional_ones`; define `pagination_tables::each_role_sends_its_authored_default_or_none`; fresh-output child test `required_adopted_parameters_are_sent_once_by_an_ordinary_send_and_by_the_pager` (run by `slow_fixture_apis_compile_and_send_offline`: a generated crate compiled against the workspace runtime sends `page=1&limit=25`, and its pager sends `page=3&limit=5`) |
| R16 follow-up shape | runtime → HTTP, F2 | the 9 follow-up and managed-input tests in `pager_collect.rs` above; Phase 3's AIP-158 test |
| R18 links and origins | runtime → two loopback origins, F2 | `pager_transport` origin and `Link` tests; Phase 3 `pager_surface` link tests |
| R19 shared deadline | runtime, real time and raw sockets, F2 | `pager_transport` deadline and drop tests; `pager_collect::deadline_exceeded_keeps_the_pages_that_arrived_in_time` |
| R20 envelope per page | runtime → HTTP and raw sockets, F2 | the five `pager_transport` envelope tests |
| R21 `collect_all` | runtime → HTTP, F2 | the `pager_collect` reason table, limit, and start tests |
| R36 privacy | runtime, F2 | `pager_transport` sentinel tests; Phase 3 cursor sentinels |
| Hooks on pages | runtime → HTTP, F2 | the three `pager_transport` hook tests |

Input robustness: this phase adds no configuration reader. Provider response bodies and `Link` headers are outside the matrix (Phase 1), and their shapes have defined outcomes in the tests above: malformed, wrong-type, absent, and ambiguous links. Smells grepped in the changed runtime and generator code: no `#[serde(default)]`; the `.ok()` in `decode_items` makes the whole page malformed rather than filtering an item.

### Validation run in this phase (macOS, rustc 1.98.1)

| Check | Result |
| --- | --- |
| `just test` (schematic area) | pass: L1 2156/2156 (12 skipped), fresh-output 6/6, compile-fail 1/1 |
| `just lint` (schematic area) | pass |
| `just generate`, then `git status` of schema/openapi/postman | no change (no shipped definition has a required adopted parameter) |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p schematic-runtime --no-deps` | clean |
| `just cross-check schematic-runtime --os linux` | pass: 329/329 |
| `pager_transport` suite, three consecutive full runtime runs (worker) | 330/330 each |
| Windows, WSL2 | not run; Phase 8.2 owns platform evidence. The new socket and deadline tests use loopback, raw sockets, and margins of at least 100 ms, like the existing `timing.rs` and `redirects.rs` suites that already pass on Windows. |

Pre-existing, not caused by this phase: `cargo doc -D warnings` fails for `schematic-define` (`headers/sensitive.rs`, last changed in `c5e4159f9`) and `schematic-gen` (`endpoint_methods/mod.rs`, `parser.rs`, `ws_codegen/mod.rs`). No file this phase changed is involved, and rustdoc is not one of the area's gates.

### Deviations in this phase

- **Decode path kept as `from_value`** (R24 prefers `from_slice`): the measured reason is under 4.1.
- **No run-time refusal of an ordinary value at a managed destination.** The managed value replaces it, and the definition-time destination rules stand in for the plan's "validate conflicts" (see 4.2).
- **A page body cut off by the shared deadline is `HttpError`, not `DeadlineExceeded`** (see 4.2).
- **The generator emits required adopted defaults in `into_parts`.** This is a generator change in a runtime-integration phase, because an ordinary send cannot see `Paginated`.
- **A refused follow-up carry is a new `InvalidArgument` case:** a raw or non-object JSON body under `Only`/`AllExcept`.

### Human review agenda (Phase 4 updates)

1. **Defaults.** Default 4 (decode path) was evaluated and `from_value` kept. Default 19 (real time with margins of at least 100 ms) is used by every deadline test.
3. **Design choices.** The global hook runs on pages and the typed mutator does not. `ManagedRole::sent_default` is public. Required adopted defaults are written into `into_parts`. Carry filters URL-encoded and multipart bodies and refuses unselectable ones.
7. **Single decode sample.** Done, as recorded in the table under 4.1: 94 endpoints, macOS, rustc 1.98.1, and a candidate 55 KB larger, so `from_value` was kept.
8. **Build observations.** The pager costs 2.6 MB of release binary at 94 endpoints in the synthetic crate. Almost all of it is per-endpoint monomorphization of the generic pager, not item deserializers. Reducing it would mean making less of `Pages` generic; that is not planned.
9. **OS.** macOS full; Linux runtime cross-check. Windows and WSL2 are Phase 8.2.
10. **Deviations.** See above.
11. **Open questions (new).** Should carry rules ever filter headers? Recommended: no, until a provider needs it, because definitions can only name query parameters.
12. **Drift, resolved by assuming the code is right:**
    - `docs/runtime/README.md` still listed pagers as planned and said `RequestMeta::page` was always `None` (stale since Phase 3). It now lists only sessions, partitions, and the paging topic page as planned.
    - `docs/runtime/metadata-and-redaction.md`'s `page` row was rewritten for the same reason.
    - `docs/runtime/origins-and-redirects.md` called pager link checks planned.
    - `docs/runtime/hooks.md` said every reply other than JSON endpoint replies skips hooks; it now describes pages.
    - `docs/runtime/deadlines-and-timeouts.md` gained the pager's deadline behavior.
    - `hooks.rs`'s module doc gained the page behavior.
    - `request.rs`'s module doc described the decode seam as provisional.
    - The gen and define READMEs gained the required-default rule.
    - The skill `schematic-define` gained the same rule.
14. **Unratified.** S11 is now final (`from_value`, with its reason).
15. **Adoption rule.** An adopted parameter that the endpoint declares required is still sent by an ordinary send, at its pager default. No shipped definition has one yet (Phase 1), so the regenerated output is unchanged.
17. **The added reasons.** All eight `IncompleteReason`s are now asserted through `collect_all` with their evidence (`pager_collect.rs`).
19. **Provenance.** D55-5 (R2a) is applied on the runtime side: the pager always sends a required managed input, and an ordinary send sends its default.

## Phase 5

Phase 5 adds scoped sessions and partitions: the lifecycle (5.1), session evidence (5.2), and partition evidence (5.3). Started 2026-10-13 on `feat/modern-config`. Entries are appended as work lands.

### 5.1 Scoped lifecycle

New runtime file `schematic/runtime/src/pager/session.rs`. Changes to `pages.rs`, `error.rs`, `request.rs`, `engine.rs`, `progress.rs`, `mod.rs`. `tokio` gains its `sync` feature (`docs/dependencies.md` updated).

**Final public signature (plan "Session signature"; spec Design `with_session`, parent P6/R7):**

```rust
impl<'a, A: Api, R: Paginated<Api = A>> Pager<'a, A, R> {
    pub fn with_session<F, O>(self, scope: F) -> CallFuture<'a, SessionResult<O, R>>
    where
        R: Sessioned,
        F: for<'s> FnOnce(&'s mut SessionScope<'a, A, R>) -> CallFuture<'s, O> + Send + 'a,
        O: Send + 'a;
}
pub type SessionResult<O, R> = Result<O, SessionError<O, <R as EndpointSpec>::Failure>>;
pub struct SessionError<O, E> { pub outcome: O, pub close: PagerError<E> }
```

A caller writes `.with_session(|scope| Box::pin(async move { scope.collect_all().await }))`. The closure receives a borrowed scope and returns a boxed future tied to that borrow, as the plan specifies. **Alternative tried and rejected:** `F: AsyncFnOnce(&mut SessionScope) -> O` with an unboxed `async fn`. It compiled and ran (the generated child test passed), but the fresh-output compile control showed that rustc cannot prove the returned future `Send` ("implementation of `AsyncFnOnce` is not general enough", the higher-ranked auto-trait limitation). A session could then not be spawned. The boxed form makes `Send` explicit; the control `sends(client.scan_entries().with_session(..))` now compiles. Reversal: revisit once return-type notation lets a bound name an async closure's future.

**Result shape (D57 item 5).** `with_session` returns the closure's output `O` unchanged when the close succeeds or no close is needed. When the close fails, it returns `Err(SessionError { outcome: O, close })`. `O` is whatever the closure returns, usually `Result<Vec<T>, Incomplete<..>>`, so a fully collected list stays `Ok` with every item inside `outcome`. `close` reuses `PagerError<E>` (`Transport` or `Api`) instead of a new error type; a close that runs out of grace is `PagerError::Transport` of kind `DeadlineExceeded`.

**Lifecycle rules as built:**

- **Open.** The session opens with the first page request of any traversal in the scope, not before the closure runs. `SessionOpen::FirstPage`: the first request of each traversal is the opening request (endpoint target, `keep_alive.on_open`, no handle). `SessionOpen::Request`: one open request (the step's method and filled path, the endpoint's headers, no endpoint query or body, step constants, `on_open` keep-alive) is sent under a `tokio::sync::Mutex`, so concurrent partitions send it once and wait for it. Its reply must hold the handle at the authored `Source`; otherwise the new `PagerError::MissingSessionHandle { meta }` (mapped to `IncompleteReason::InvalidContinuation`). A failed open is remembered and every later traversal gets `AlreadyFailed`. An open failure surfaces as the first page's error inside the closure; no handle means no close (R22a).
- **Pages.** A non-opening page request goes to `Session.pages` (method and filled path) unless it follows a link, carries the latest handle at `send_handle_as`, and carries `keep_alive.on_page`. The keep-alive value is the caller's `.keep_alive(..)` (bound `KeepAliveKnob`) or the authored default.
- **Latest handle.** Each 2xx page's `handle_from_page` is read right after the body parses, and after the global JSON hook, before items or continuations are checked. A page that later proves malformed still updates the handle the close uses. The latest observed value wins, in response-arrival order, across every traversal and partition. A handle is a non-empty string or a number. A missing, null, empty, or structured handle on a page keeps the previous one.
- **Renew.** `SessionScope::renew()` and `Pages::renew()` (bound `Renewable`) send the renew step with the latest handle and the step's constants. They return `Ok(false)` without sending when no handle is known yet. `Pages::renew` exists so a traversal holding the scope's borrow can still renew.
- **Close.** It runs after the closure's future completes, or after a panic while the closure is called or its future is polled; both are caught with `catch_unwind(AssertUnwindSafe(..))`. It runs only when a close step is authored and a handle was observed. The close request is sent with the call's credential mode and a fresh `Deadline::after(grace)` in place of the pages' deadline, so its time is min(grace, the client's per-request timeout) and it is still sent after the page deadline expired (R22a). The grace defaults to `DEFAULT_CLOSE_GRACE` (5 s) and is set with `Pager::close_grace` (bound `Sessioned`) and read with `configured_close_grace`. After a panic, the close is attempted and then the original panic resumes, so its close failure cannot be returned. Nothing is spawned: cancellation of the scope's future, panic abort, and process exit leave the session to the lease. The module doc of `session.rs` states this.
- **Scope surface.** `SessionScope` has `pages(&mut self)`, `fetch_page`, `collect_all` (with the pager's page limit), `partitioned(&mut self, n)` (bound `Partitioned`), and `renew(&self)`. Each traversal starts at the pager's configured position, from a clone of the configured engine (`Engine`, `History`, and `BoundaryTracker` now derive `Clone`). There is no `open()` or `close()`, and a session endpoint still has no `pages()`, `fetch_page()`, or `collect_all()` outside the scope (`Sessionless`).
- **Partitions (R23).** `Pager::partitioned(n)` (bounds `Partitioned + Sessionless`) and `SessionScope::partitioned(n)` return `n` independent `Pages`. Traversal `i` writes `index = i` (0-based) and `count = n` at the authored inputs on every request, including followed links. Count `0`, or a count above the authored `max`, is `InvalidArgument` with nothing sent. Each partition has its own engine, so its own continuation history and stall detection. Partitions share only the session state (`&SessionShared`: the latest handle behind a `std` mutex held only while reading or writing, and the open mutex). Page requests are never serialized and nothing is spawned; the caller polls the iterators.
- **Shared collection code.** `Pager::collect_all` and `fetch_page` now delegate to `pages::{collect, fetch_first}`, so the scope's `collect_all` and `fetch_page` use exactly the same completion and limit rules.
- **`Pages::items()`** lost its `Sessionless` bound: a session endpoint's `Pages` only exists inside a scope, so its item stream is allowed there.

**Generator (call-type forwards; Phase 3 deferred them here).** `KnobForward::KeepAlive` forwards `.keep_alive(value: impl Into<String>)` on an adjustable keep-alive. A session endpoint's call type forwards `with_session` with the same bounds, spelled with `schematic_runtime::CallFuture`. A partition without a session forwards `partitioned(count)`; a session's partitions are made inside its scope, so its call type forwards none. `PLANNED_CALL_METHODS` gains `with_session`, so a setter of that name on any call type is renamed now rather than later. `PagerForwards::has_control` covers the knob forwards and `partitioned` for the per-endpoint rename. The fresh-output fixture API gains `ScanEntries`, a point-in-time-style session with keep-alive, renew, close, and a partition. It now has:

- a compile control calling `keep_alive`, `with_session`, `renew`, `partitioned`, and `fetch_page`, and proving the future is `Send`;
- a misuse case, `.pages()` on the session call, which fails with E0599;
- a child runtime test, `a_generated_session_opens_pages_and_closes_inside_its_scope`, run through the generated client against wiremock: `POST /pit?keep_alive=5m`, two `GET /scan` pages carrying `pit` and `keep_alive`, and `DELETE /pit?pit=pit-1`.

Recorded for review:

- **Elasticsearch scroll ids often repeat** between pages. With a cursor read from `_scroll_id` and the widened stall rule (R14), a repeated id is `RepeatedContinuation`. No shipped definition authors a scroll. A future Elasticsearch definition needs either a scroll id that is not the cursor (the handle alone, with `send_handle_as`) or a stall-rule exception. Open question for agenda item 11.
- **First-page sessions with partitions** (sliced scroll) follow R23 literally: one shared latest handle, one close. Each slice's first request opens server-side state of its own, so with a real sliced scroll only the latest handle is closed and the others are left to their lease. That is the accepted out-of-order and rotating-handle risk (D57 item 2) applied to first-page opens. No shipped definition has one.
- **The close grace is forwarded on the call type.** A session endpoint's call type also forwards `close_grace(..)`, so `es.search(i).deadline(&d).close_grace(g).with_session(..)` reads in one chain. `close_grace` is reserved for that endpoint's setters only where a session is authored (`PagerForwards::has_control`).
- **Regeneration.** `just generate` after these changes leaves `schematic-schema`, OpenAPI, and Postman unchanged. No shipped definition authors a session or partition, and no shipped setter is named `with_session`.

### 5.2 Session evidence and 5.3 Partition evidence

Before the workers started, `pager_support.rs` gained a `session_endpoint!` macro and two shared fixtures. `PitSearch` is an Elasticsearch-style point in time: separate open `POST /logs/_pit` returning `id`, pages at `POST /_search` with `pit.id` and `pit.keep_alive` in the body, `search_after` from the last hit's `sort`, a rotating `pit_id` on every page, partitions in `slice.id` and `slice.max` (max 4), and close `DELETE /_pit`. `ScrollSearch` is an Elasticsearch-style scroll: opened by its first page, `_scroll_id` as both cursor and handle, pages at `POST /_search/scroll`, a renew step with a constant, and close `DELETE /_search/scroll`. Two smoke tests in `pager_session.rs` proved the whole lifecycle before delegation:

- `a_scroll_opened_by_its_first_page_pages_with_the_scroll_id_and_closes_the_latest_one`
- `a_point_in_time_opens_once_pages_at_its_target_and_closes_with_the_latest_handle`

Then two delegated workers ran concurrently with disjoint files: `pager_session.rs` plus the `pager_session_*` compile-fail fixtures (5.2), and `pager_partition.rs` plus `define/tests/pagination_partitions.rs` (5.3). Neither found a runtime defect. Every test is L1 and untiered, uses local wiremock or raw loopback sockets, and synchronizes on server events with no fixed sleeps. The slowest new test takes 0.32 s.

`pager_session.rs` (22 tests, with the 2 smoke tests):

- Requests sent:
  - `without_a_caller_keep_alive_the_authored_default_goes_on_the_open_and_every_page`
  - `the_close_grace_defaults_to_five_seconds_and_takes_the_callers_value`
  - `renewing_inside_the_scope_sends_the_renew_step_with_the_latest_handle_and_its_constant`
  - `renewing_before_any_handle_is_known_returns_false_and_sends_nothing`
- Every exit path:
  - `a_scope_that_returns_after_its_first_page_still_closes_the_latest_handle`
  - `a_scope_whose_closure_returns_an_error_closes_and_returns_that_error_unchanged`
  - `a_closure_that_panics_before_returning_its_future_sends_nothing_and_the_panic_resumes`
  - `a_closure_that_panics_after_a_page_closes_the_latest_handle_then_resumes_the_panic`
- Failed close (D57 item 5):
  - `a_failed_close_returns_the_complete_list_with_the_close_error`
  - `a_failed_close_after_a_closure_error_keeps_the_closure_error_as_the_outcome`
- Grace (R22a):
  - `a_close_slower_than_its_grace_is_abandoned_with_a_deadline_error_and_the_list_kept` (200 ms grace against a 10 s hang)
  - `an_early_return_under_a_long_page_deadline_bounds_the_close_by_the_grace`
  - `the_close_is_still_sent_and_succeeds_after_the_pages_deadline_expired`
- Latest handle:
  - `a_malformed_page_still_hands_its_new_handle_to_the_close`
  - `a_malformed_first_page_that_opened_a_scroll_still_closes_it`
- Open failure:
  - `a_failed_open_sends_no_page_and_no_close_and_a_second_traversal_does_not_reopen`
  - `an_open_reply_without_a_handle_is_a_missing_handle_with_no_page_and_no_close`
- Privacy (R36):
  - `no_handle_or_scope_summary_appears_in_session_error_text`
  - `a_page_error_whose_raw_body_carries_the_handle_keeps_it_out_of_its_display_text`
- Cancellation:
  - `cancelling_a_scope_while_a_page_is_in_flight_closes_its_connection_and_sends_no_close` (raw socket: the server observes EOF, and a control request proves no close ever arrived)

Compile-fail fixtures (trybuild now has 31 cases; no existing `.stderr` changed):

- `pager_session_no_manual_open_or_close` (E0599)
- `pager_session_pages_escape_the_scope` (a lifetime error, and E0521)
- `pager_session_renew_without_a_renew_step` ("has no session renew step")
- `pager_session_keep_alive_not_adjustable` ("has no adjustable keep-alive")
- `pager_session_on_a_sessionless_endpoint` ("has no server-side session")

The existing `pager_pages_on_a_session_endpoint` covers `pages`, `fetch_page`, and `collect_all` outside the scope. Permitted collection inside the scope is the runtime tests above.

`pager_partition.rs` (6 tests):

- `four_partitions_share_one_open_and_one_close_and_each_slice_pages_its_own_items`
- `each_partition_follows_its_own_cursor_and_a_stalled_or_failed_partition_leaves_the_others_complete`: slice 2 stalls with `RepeatedContinuation`, slice 3 gets a 500, and the others complete; one open and one close.
- `a_partition_count_of_zero_or_above_the_maximum_is_refused_before_anything_is_sent`: session and sessionless; no request at all, not even the open.
- `a_sessionless_partition_sends_its_index_and_count_on_every_request_and_keeps_its_own_history`
- `every_request_after_a_late_arriving_response_carries_that_latest_observed_handle_and_so_does_the_close`: a scripted one-request-per-connection server holds partition 0's first reply (`late`) until partition 1 has received `early` and requested its second page. The full sequence asserts that partition 1's second page carries `early`, and that partition 0's second page, partition 1's third page, and the close carry `late`. That is arrival order, not issue order (D57 item 2). The test shows the designed semantics and makes no claim of provider safety.
- `two_partitions_have_their_first_page_requests_in_flight_at_the_same_time`: each first reply is held until the other first request arrived, so it can only finish if partition requests are not serialized.

`define/tests/pagination_partitions.rs` (1 test, autotests): `a_rotating_handle_point_in_time_with_four_partitions_inherited_from_the_api_default_validates`. A rotating-handle point-in-time default with `Partition` max 4 has no `validate_pagination` errors, resolves with origin `ApiDefault`, and offers the knobs `Session`, `Partitions { max: Some(4) }`, and `KeepAlive`.

Recorded from the workers:

- **A raw page body can carry the handle.** `Display` of every error stays clean, but a page error keeps its raw response body unredacted, so its `Debug` output can show a handle the server put in that body. This is the envelope's documented limit (`error.rs` module doc, R36 "raw response evidence retains the runtime's documented privacy limitations"). The test asserts the current behavior.
- **Not covered:** cancellation while the open or close request itself is in flight (documented as left to the lease, like cancellation during a page, which is tested); a handle read from a response header (`handle_text` reads both, but no fixture uses a header).

#### Requirement-to-test mapping (Phase 5 scope)

| Requirement | Boundary and fidelity | Tests |
| --- | --- | --- |
| R22 only `with_session`; no `open`/`close`; no `pages()` outside | runtime compile surface (trybuild), F2; generated crate, F3 | `pager_session_no_manual_open_or_close`, `pager_session_pages_escape_the_scope`, `pager_pages_on_a_session_endpoint`; fresh-output misuse "pages of a session endpoint outside its scope" (E0599) and compile control (`with_session`, `keep_alive`, `close_grace`, `renew`, `partitioned`, `Send`) |
| R22 one close attempt on return, error, and unwinding panic | runtime → HTTP, F2 | the exit-path tests above |
| R22 keep-alive in band, latest handle, `Session.pages` target, renew, separate open | runtime → HTTP, F2; generated crate → HTTP, F3 | both smoke tests, the keep-alive, renew, and malformed-page tests; fresh-output child `a_generated_session_opens_pages_and_closes_inside_its_scope` |
| R22 cancellation spawns nothing | runtime, raw sockets, F2 | `cancelling_a_scope_while_a_page_is_in_flight_closes_its_connection_and_sends_no_close` |
| R22a grace, separate from the deadline, every close attempt | runtime, real time (margins ≥ 100 ms), F2 | the three grace tests |
| R22a no close without a handle | runtime → HTTP, F2 | the two open-failure tests and the construction-panic test |
| D57 item 5 `SessionError { outcome, close }` | runtime → HTTP, F2 | the two failed-close tests |
| R23 partitions | runtime → HTTP and raw sockets, F2; define, F1 | the 6 `pager_partition` tests; `pagination_partitions.rs` |
| R10 session knobs (`keep_alive`, `renew`, `with_session`) | trybuild | the three knob compile-fail fixtures |
| R36 handle and close privacy | runtime, F2 | the two privacy tests |
| Generator forwards | gen unit, F1 | `pagination::tests::{session_strategy_forwards_with_session_and_its_knobs_but_no_outside_scope_iteration, a_partition_without_a_session_forwards_partitioned_and_with_one_does_not}` |

Input robustness: this phase adds no configuration reader. Session handles come from provider responses, outside the matrix (Phase 1). Their shapes have defined outcomes: a non-empty string or a number replaces the latest handle; absent, null, empty, or structured keeps the previous one. An open reply without a usable handle is `MissingSessionHandle`. Smells grepped in `session.rs`: no `#[serde(default)]`. The only `unwrap_or` is `serde_json::from_slice(..).unwrap_or(Value::Null)` in `StepDecoder`, where a close or renew reply body is deliberately unread and an open reply without a JSON handle becomes `MissingSessionHandle` rather than an empty success.

### Validation run in this phase (macOS, rustc 1.98.1)

| Check | Result |
| --- | --- |
| `just test` (schematic area) | pass: L1 2186/2186 (12 skipped), fresh-output 6/6, compile-fail 1/1 (31 cases) |
| `just lint` (schematic area) | pass |
| `cargo clippy -p schematic-runtime -p schematic-define -p schematic-gen --all-targets -D warnings` | clean |
| `just generate`, then `git status` of schema/openapi/postman | no change |
| `pager_session` / `pager_partition` repeated runs (workers) | 3 consecutive passes each |
| `just cross-check schematic-runtime --os linux` | pass: 357/357, 1 skipped (the trybuild test, which runs through its own recipe) |
| Windows, WSL2 | not run; Phase 8.2 owns platform evidence. The new real-time tests use margins of at least 100 ms (a 200 ms grace against a 10 s hang, a 300 ms page deadline), and the raw-socket cancellation test reuses the Phase 4 pattern. `catch_unwind` assumes `panic = "unwind"`, the workspace default. |

Observation, not a failure: in the full `just test` run, nextest marked `fresh_output_compile::generated_rest_code_references_the_runtime_and_defines_none_of_it` as SLOW (> 5 s), though it passed. Run alone it takes 2.4–3.1 s. It generates every shipped API, and this phase changes none of their output, so the overrun is load from the parallel suite rather than this change. Recorded for Phase 8's acceptance audit.

### Deviations in this phase

- **`with_session` signature.** The plan's "boxed future tied to its borrow" was adopted only after an unboxed async-closure (`AsyncFnOnce`) form proved unable to give a `Send` future (5.1). The caller writes `Box::pin(async move { .. })`.
- **The session opens with the first page request** (lazily), not before the closure runs. An open failure is therefore the first page's `PagerError` inside the closure, not a separate `with_session` error. Alternative: open eagerly and return an open error from `with_session`, which would need a third result shape.
- **`SessionError::close` is a `PagerError<E>`,** not a dedicated close-error type. Only its `Transport` and `Api` variants occur.
- **`renew()` returns `Result<bool, _>`:** `false` means no handle was known, so nothing was sent.
- **`close_grace` is forwarded on session call types,** beyond the plan's list of forwards, so the R22a builder setting needs no `.pager()`.
- **`Pages::items()` lost its `Sessionless` bound,** because session `Pages` exist only inside a scope.

### Human review agenda (Phase 5 updates)

3. **Design choices.** The `with_session` signature and result shape (5.1); lazy open; `SessionScope` methods; `DEFAULT_CLOSE_GRACE`, `close_grace`, and `configured_close_grace`; `MissingSessionHandle`; `SessionError`; `SessionResult`; partitions as `Vec<Pages>` with `InvalidArgument` for a bad count; `keep_alive(impl Into<String>)` in the provider's own format.
9. **OS.** macOS full; Linux runtime cross-check. Windows and WSL2 are Phase 8.2.
10. **Deviations.** See above.
11. **Open questions (new).**
    - Elasticsearch scroll ids can repeat between pages, which the widened stall rule reports as `RepeatedContinuation` when the scroll id is the cursor. Recommended: when a scroll definition is authored, model the id as a session handle sent with `send_handle_as`, or add a per-strategy exception to the stall rule.
    - Partitions of a first-page-opened session (sliced scroll) close only the latest handle (5.1). Recommended: accept for now, since no shipped definition needs it, and revisit with an Elasticsearch definition.
16. **Session-close grace (default 21).** 5 s, configurable with `close_grace`, bounding every close as min(grace, the client's per-request timeout), independent of the page deadline. The runtime-envelope spec made no ruling that overrides it.
20. **Rotating handle with partitions (D57 item 2).** Implemented as decided: the latest observed handle, in response-arrival order, is used by every partition's next request and by the close. `every_request_after_a_late_arriving_response_carries_that_latest_observed_handle_and_so_does_the_close` demonstrates it. **Accepted risk:** when two partitions' responses complete out of order, the latest observed handle may not be the provider's newest. A provider that invalidates older handles could then reject the next request or leave a newer handle unclosed until its lease expires. The pager does not serialize partitions or validate against this, as R23 requires.

## Phase 6

Phase 6 authors pagination in the real definitions and preserves it across OpenAI re-imports. Started 2026-10-13 on `feat/modern-config`. Entries are appended as work lands.

The three Wave 11 tasks with disjoint files ran concurrently in delegated workers: 6.3 (the importer overlay hook, in `schematic/gen` and the `import-openai` recipe), 6.2 (the OpenAI overlay), and 6.1 with 6.4 (forges, ElevenLabs, and the unpaginated rows of the unverified APIs). Before they started, the shared R27 coverage harness was written so each worker filled only its own row file.

### Coverage harness (written first, for 6.1–6.5)

- `schematic/definitions/tests/pagination_coverage/main.rs` is one test binary (auto-discovered; `schematic-definitions` has no `autotests = false`). It loads every shipped API through `apis_by_module()` and holds one expectation table per API, split by family into `forges.rs`, `openai.rs`, and `others.rs`.
- **Every** endpoint is checked: an endpoint with a row must resolve exactly as the row says, and an endpoint without a row must resolve as "not a list". A paginated row asserts the origin, the item selector, the item type, a one-line description of the strategy (kind, inputs and sources, end signal, total, constants), the exact `caller_knobs`, and the set of adopted request fields. An unpaginated row asserts the origin, the item selector, and a reason: `Unverified { recalled, source_needed }`, `DefinitionGap { gap, recalled, source_needed }`, `ProviderGap`, `NotPaged`, or `NoPagination`. The `Unverified` and `DefinitionGap` rows are the R33 verification task.
- Before any authoring, the harness failed with 37 GET `Vec<T>` lists that had no row, which shows it detects missing coverage. The strategy description is compared as literal text written in the row; no expectation is computed from the definitions.

### Wire-baseline test migrated in this phase, not Phase 7

`fresh_output_compile::slow_query_wire_matches_pre_split_baseline` generates clients from the **live** definitions and set Gitea `page`/`limit` and OpenAI `limit`/`after` as request fields. Authoring those strategies adopts the fields, so the test would no longer compile, and `just test` runs it. The Phase 1 ledger scheduled this migration for Phase 7.1; it moves here because Phase 6 must leave `just test` green. The program now sends those values through the pager (`send(request)…pager().page(2).page_size(1).fetch_page()`, and `.page_size(5).after("x%y")` for OpenAI). Its `into_parts` checks drop the removed fields. The old `limit=0` value became `page_size(1)`, because a page size below the authored minimum of 1 is now refused before sending. The re-captured baseline lines are recorded below once the run is done.

The re-captured lines differ from the old baseline in exactly three entries, all for the expected reason. `ordered-scalars-reserved-unicode`, `repeated-array-keys`, and `legacy-embedded-query-plus-declared` now carry the pager-managed pairs after the request's own pairs (`…&sort=…&page=2&limit=1`, `…&include=…&after=x%25y&limit=5`, `…&include=%22x%20y%22&limit=3`), because the pager appends its inputs to the request's query. Every value's percent-encoding is unchanged. The other five lines are byte-identical. The `WIRE_BASELINE` doc comment records this as the second intended exception.

### 6.1 Forge strategies (done)

Each forge has one API-local helper; endpoint literals and the legacy `bitbucket_pagination()` are untouched.

| API | Default (helper) | Rows | Paginated | Adopted fields |
| --- | --- | --- | --- | --- |
| GitHub | `github_page_number()`: `page` from 1, `per_page` 1..100 (server default 30), more while `Link rel=next` | `ListWorkflowRuns`: `workflow_runs`, `WorkflowRun`, default + total body `total_count` (override) | 9 | `page`, `per_page` ×9 |
| GitLab | `gitlab_page_number()`: `page` from 1, `per_page` 1..100 (20), more while header `X-Next-Page` present, total header `X-Total` | `ListIssueParticipants` root opt-out (unverified); `ListRepositoryTree`, `ListGroupProjects` inherit with a `keyset` alternate (`gitlab_keyset()`: `Link rel=next`, constants `pagination=keyset`, `order_by=id`, `sort=asc`) | 9 | `page`, `per_page` ×9 |
| Gitea | `gitea_page_number()`: `page` from 1, `limit` 1..50 (30), `Link rel=next`, total header `X-Total-Count` | `GetTagReference` root opt-out (prefix lookup); `GetGitTree`, `GetGitTreeRecursive` at `tree`, `GitTreeEntry`, unpaginated (unverified) | 8 | `page`, `limit` ×8 |
| Bitbucket | `bitbucket_next_link()`: body `next`, prev body `previous`, `pagelen` 1..100 (10), total body `size` | `bitbucket_lists()`: 8 inheriting rows at `values` with item types | 8 | `pagelen` ×8 (`page` kept: a next link has no page input) |

Facts re-checked against the definitions: `WorkflowRunsResponse` has `total_count` and `workflow_runs` (`github/types/workflow.rs`); `PaginatedResponse` has `values`, `next`, `previous`, `size` (`bitbucket/types/common.rs`); `GitTreeResponse` has `tree`, `truncated`, `total_count` and the tree endpoints declare no parameters (`gitea/types.rs`). No new contradiction was found.

### 6.2 OpenAI overlay (done)

`schematic/definitions/src/openai_pagination.rs` is hand-written; `apply(api)` sets the default (`after` from body `last_id`, more while `has_more`, size `limit` with no bounds, resumable) and 61 rows. Every row was re-checked against the reply struct, the declared parameters, and `specs/openai/openapi.yaml`; the groups match the Phase 1 reconciliation exactly (no contradictions).

| Group | Endpoints | Origin | Adopted |
| --- | --- | --- | --- |
| Default, unbounded size | 8 (ListChatCompletions, GetChatCompletionMessages, ListEvals, GetEvalRuns, GetEvalRunOutputItems, ListFineTuningCheckpointPermissions, ListFineTuningJobCheckpoints, AdminApiKeysList) | ApiDefault | `after`, `limit` |
| `last_id` with documented bounds | 19 | Exception | `after`, `limit` |
| `last_id` + backward `before` from `first_id` | 9 | Exception | `after`, `before`, `limit` |
| `after` from the last item's `id` | 2 (ListFineTuningEvents, ListPaginatedFineTuningJobs) | Exception | `after`, `limit` |
| `after` from body `next` (S7) | 9 | Exception | `after`, `limit` |
| `page` from body `next_page` (S8, usage) | 11 | Exception | `page`, `limit` |
| Unpaginated | ListModels (no continuation), RetrieveVectorStoreFileContent and SearchVectorStore (provider gap) | Exception | none |

Page-size bounds are authored only where the vendored document states one for every request (G2): max 100 on 30 endpoints (server default 20 where stated), ListFiles max 10000 (default 10000), ListGroups/ListGroupUsers max 1000 (default 100), ListRoles/ListProjectRoles max 1000 (default 1000), four role-assignment lists max 1000, UsageCosts max 180 (default 7); ListProjectRateLimits and the two item-id lists carry only a server default; the other ten usage endpoints, whose bounds depend on `bucket_width`, carry none. The source line of each bound is in the worker's report and the overlay's comments.

### 6.3 Importer overlay (done)

- `schematic-gen import --overlay <RUST_PATH>` (`ImportOptions.overlay`). The value must parse as a plain `syn::Path`; empty text, non-paths, generic arguments, and calls are rejected with a `ConfigError` naming `--overlay`, before anything is written. It requires `--definitions-out` (clap `requires`, and `run_import` checks it for library callers).
- With an overlay the factory ends `let api = RestApi { .. }; <path>(api)` and the module's "Regenerate with" hint names the option. Without one the output is byte-identical to the previous importer (checked on petstore and OpenAI before and after the change). The emitter comment beside the empty pagination fields now states the overlay contract.
- `just import-openai` passes `--overlay crate::openai_pagination::apply`. `schematic/docs/io/import-openapi.md` gained "Importing into `schematic-definitions`" (example, rules, a Mermaid diagram); `schematic/gen/README.md` gained one example.
- Compile evidence: L1 tests parse the emitted file with `syn` and check the call's structure; a one-off scratch crate compiled a petstore import with an overlay (`cargo test --offline`, about 38 s) and was deleted. No repeatable `slow_` compile test was added: it would need the fresh-output helpers' private functions and a definitions stand-in with `schemars`. The lasting compile evidence is `schematic-definitions` compiling the re-imported `openai/mod.rs`, and `fresh_output_compile::slow_real_apis_compile_offline` compiling the clients generated from it.

### 6.4 Remaining coverage (done)

- **ElevenLabs** (no API default, helper `elevenlabs_lists()`): `GetHistory` at `history`, `SpeechHistoryItemResponseModel`, cursor `start_after_history_item_id` from body `last_history_item_id`, more while `has_more`, `page_size` 1..1000 (100), resumable; `ListSharedVoices` at `voices`, `LibraryVoiceResponseModel`, page number `page` from 0, more while `has_more`, `page_size` 1..100 (30). Verified against `schematic/docs/research/elevenlabs-api.md` (lines 708–728 and 983–999, research note dated 2026-05-07) and the response types. Neither endpoint declares those parameters, so nothing is adopted. `ListVoices` has an unpaginated row (definition gap D2: `page_token` declared, `next_page_token` researched); its fields stay, so biscuit-speaks is unaffected.
- **Unpaginated rows for unverified wrappers:** Anthropic `ListModels` (`data`), EMQX basic and bearer through one shared `common_lists()` (7 lists at `data` each), Hugging Face `ListModelDiscussions` (`discussions`). Root arrays with no default (Hugging Face 10, EMQX `ListAuthUsers` ×2, ElevenLabs `ListModels`) resolve unpaginated without a row. Hugging Face already has a `Collection` type, so the pagination import is aliased `ListCollection`.
- **Non-list array writes:** `others::unfolded_circle_writes_returning_arrays_are_not_lists` pins Unfolded Circle `RestoreBackup`/`UploadResource` as not lists.

### 6.5 Catalog integration (done)

- **Registration and re-import.** `pub mod openai_pagination;` is in `definitions/src/lib.rs`. `just import-openai` (now with `--overlay`) rewrote `definitions/src/openai/mod.rs`; the only differences from the previous import are the overlay tail and the hint line, and `types.rs` is byte-identical. The import takes about 1 s.
- **Re-import regression (R35).** `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition` imports the vendored document twice with the recipe's options and asserts both imports are identical, contain exactly one overlay call, and equal the committed `mod.rs` and `types.rs` byte for byte (1.1 s). Because the committed factory is the one the coverage test resolves, this also proves the resolution after a re-import equals the resolution before it. Unknown exception ids are rejected by `every_shipped_definition_validates` and by gen's `live_definitions_report_no_pagination_diagnostics`.
- **R27 coverage.** All 18 APIs pass: 94 paginated (GitHub 9, GitLab 9, Gitea 8, Bitbucket 8, OpenAI 58, ElevenLabs 2), 5 opted-out lists, and every other list unpaginated. `every_unverified_list_names_its_recalled_strategy_and_needed_source` pins the R33 verification task to its exact 32 endpoints: Anthropic `ListModels`; ElevenLabs `ListVoices` (definition gap); GitLab `ListIssueParticipants`; Gitea `GetGitTree`, `GetGitTreeRecursive`; EMQX basic and bearer `ListAlarms`, `ListAuthUsers` (definition gap), `ListBanned`, `ListClients`, `ListRetained`, `ListRules`, `ListSubscriptions`, `ListTopics`; Hugging Face `GetUserCollections`, `ListDatasetCommits`, `ListDatasetFiles`, `ListDatasets`, `ListModelCommits`, `ListModelDiscussions`, `ListModelFiles`, `ListModels`, `ListSpaceFiles`, `ListSpaces`, `ListUserRepos`. Each row carries the recalled strategy and the source needed.
- **Regeneration pulled forward from 7.1.** `fresh_output_compile::slow_real_apis_compile_offline` also asserts that freshly generated Rust equals the committed `schematic/schema/src`, so `just test` cannot pass after authoring without regenerating. `just generate` changed only the six authored APIs' `requests.rs` (+9,798/−3,587 lines on the first run; `git diff --numstat`: bitbucket 700/138, elevenlabs 200/0, gitea 704/305, github 783/330, gitlab 797/384, openai 6614/2430). OpenAPI and Postman are byte-identical (R29), and a second `just generate` produced identical bytes across `schema/src`, `openapi`, and `postman`.
- **Removed request fields (R7, agenda 4):** 185, matching the Phase 1 prediction of 186 less ElevenLabs `ListVoices` `page_size`, which stayed. GitHub `page` ×9, `per_page` ×9; GitLab `page` ×9, `per_page` ×9; Gitea `page` ×8, `limit` ×8; Bitbucket `pagelen` ×8; OpenAI `after` ×47, `before` ×9, `limit` ×58, `page` ×11. The generated crate gained 94 `impl schematic_runtime::pager::Paginated`.
- **Call sites migrated with it (ledger rows that the regeneration breaks):**
  - `schematic/schema/tests/github_client.rs::endpoint_method_and_generic_send_of_its_request_agree`: both paths now set the size through the pager (`.page_size(5).fetch_page()` and `send(request).pager().page_size(5).fetch_page()`) and compare `Page::items`.
  - `sniff/lib/src/remote/github.rs` `list_workflow_runs`: `call.clone().page_size(n).fetch_page()`, retried as `call.anonymous().page_size(n).fetch_page()` on a 401/403 to a credentialed request; the items come from `Page::items`. The limit is clamped to the definition's 1..=100 because a size outside the bounds is now refused before sending (previously `per_page` was sent unchecked and GitHub capped it). `sniff/lib/src/remote/envelope.rs` gained `Replies::page`, which classifies a `PagerError`'s transport and response failures exactly as a call's, and reports any other pager failure by its redacted `Display`. New test `remote_providers::github_tests::list_workflow_runs_clamps_the_page_size_to_githubs_bounds` (150 → `per_page=100`, 0 → `per_page=1` and no runs). `sniff/docs/repos/repo-tech-design.md` shows the new code.
  - biscuit-speaks (`ListVoices` unchanged) and the five homelab crates needed no edit; all compile.
- **Docs and skill.** `schematic/definitions/README.md` gained an overview "Pagination" section and a step in "Adding New APIs"; its Gitea row said "`limit` (default 50)" and is corrected to default 30, max 50 (code assumed correct); the Bitbucket section says the definition follows `next`. `schematic/specs/openai/README.md` explains the overlay. The `schematic-define` skill records the authored set, the coverage test, and `--overlay`.

#### Requirement-to-test mapping (Phase 6 scope)

| Requirement | Boundary, rung, tier | Tests |
| --- | --- | --- |
| R27 every list endpoint of the 18 APIs, origin + strategy + exact knobs | definitions crate through `resolve`/`caller_knobs` (F2, L1) | `pagination_coverage::{every_list_endpoint_of_every_api_resolves_as_authored, every_row_names_one_endpoint_of_a_shipped_api, paginated_counts_per_api}` |
| R28/R33 unverified rows stay unpaginated and are listed | same | `pagination_coverage::every_unverified_list_names_its_recalled_strategy_and_needed_source` (exact 32) |
| R5 unknown/duplicate ids, item types, array locations in real definitions | define validation and gen registry checks (L1) | `pagination_coverage::every_shipped_definition_validates`; `pagination_validation::live_definitions_report_no_pagination_diagnostics` |
| R7 adopted fields per endpoint, kept fields (Bitbucket `page`, OpenAI `order`) | same, plus generated output | coverage rows' `adopted`; the regenerated `requests.rs` (185 removals above) |
| D43 non-GET lists and non-list writes | same | coverage `OpenAI.SearchVectorStore` row; `others::unfolded_circle_writes_returning_arrays_are_not_lists` |
| R35 overlay hook: emitted call, no call without one, invalid paths, untouched overlay file, deterministic import | gen import pipeline, filesystem (F2, L1) | `definition_gen::tests::*` (7), `import_overlay::{an_overlay_import_emits_the_call_and_leaves_the_overlay_untouched, an_import_without_an_overlay_ends_with_the_literal, an_invalid_overlay_is_rejected_before_anything_is_written, an_overlay_without_definitions_out_is_rejected}` |
| R35 OpenAI re-import keeps pagination | real vendored document and committed definition (F3, L1) | `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition` |
| R25/R29 generated surface compiles; exports unchanged | generated crate, offline compile (F3, fresh-output) | `fresh_output_compile::slow_real_apis_compile_offline`; `artifact_drift::*`; `just test-schema`, `just lint-schema` |
| Wire bytes of migrated pager sends | raw TCP listener through generated clients (F3) | `fresh_output_compile::slow_query_wire_matches_pre_split_baseline` |
| sniff migration: one page, anonymous retry survives, size clamp | sniff provider over wiremock (F2, L1) | the 7 `list_workflow_runs` tests in `sniff::l1 remote_providers`, including the new clamp test |

### Validation run in this phase (macOS, rustc 1.98.1)

| Check | Result |
| --- | --- |
| `just test` (schematic area) | pass: L1 2204/2204 (12 skipped), fresh-output 6/6, compile-fail 1/1 |
| `just lint` (schematic area) | pass |
| `just test-schema` / `just lint-schema` | pass: 40/40 / clean |
| `just test sniff` (root) | pass: 3419/3419 (33 skipped); its 27 SLOW tests are host hardware and network probes this phase does not touch |
| `just lint` (sniff) | pass |
| `just test biscuit-speaks` (root) | pass: 505/505 (19 skipped) |
| `cargo check --all-targets --all-features` for biscuit-speaks and the five homelab crates | pass |
| `just generate` twice | identical hashes for `schema/src`, `openapi`, `postman`; exports unchanged from baseline |
| Other OSes | not run. The change is definition data, code generation, and one sniff call site, with no paths, processes, or timing. Phase 8.2 owns platform evidence. |

No pre-existing failure was found. The homelab crates' tests were not run (compile only, no call-site change); Phase 7.3 owns full dependent evidence.

### Deviations in this phase

- **Regeneration and the breaking migrations moved from 7.1 into 6.5.** The committed-output check in `slow_real_apis_compile_offline` makes regeneration necessary for `just test` to pass, and regeneration breaks exactly the ledger's call sites. Phase 7.1 still owns the reviews it lists; this phase already recorded the diff statistics, the field-level removal list, export equality, and a second deterministic generation.
- **The wire-baseline program was migrated here, not in 7.1** (see above).
- **OpenAI minimums of 0 are held at 1.** Several endpoints' schemas document `minimum: 0` (the 0..100 and 0..1000 rows above), but the model rejects a zero minimum, so a provider-valid `limit=0` is refused client-side. A zero-size page has no use for traversal.
- **Documented OpenAI defaults without a maximum are not authored on the 8 API-default rows,** because authoring a documentation-only value would turn them into overrides.
- **No repeatable compile test of overlay import output** (6.3 above); a one-off compile was run.

### Human review agenda (Phase 6 updates)

1. **Defaults.** Default 14 applied: every GET `Vec<T>` under a default has a row or is a real list (GitLab participants and Gitea tag reference opt out). P8 item 7: GitLab's `keyset` alternates are modeled on two endpoints and generate nothing. Default 13/22: wrapper item types are authored strings resolved through each registry.
3. **Design choices.** API-local strategy helpers per definition; one OpenAI overlay module at the crate root (`openai_pagination.rs`, not inside the importer-owned `openai/`); `--overlay` as a path option requiring `--definitions-out`; unpaginated rows (with item types) for unverified wrappers so they read as lists.
4. **Removed request fields and dependent edits.** 185 fields (above); edits to `schematic/schema/tests/github_client.rs` and sniff's `list_workflow_runs` (plus `Replies::page` and one test). Bitbucket `GetSource` and ElevenLabs `ListVoices` fields kept.
5. **Authored versus unpaginated (R33).** Authored: GitHub 9, GitLab 9, Gitea 8, Bitbucket 8, OpenAI 58, ElevenLabs 2 (sources: GitHub/GitLab/Gitea/Bitbucket per `sniff/docs/repos/*-api.md` 2026-02-15 and the 2026-10-09 inventory; OpenAI from the vendored document, retrieved 2026-07-31; ElevenLabs from `schematic/docs/research/elevenlabs-api.md`, 2026-05-07). Unpaginated: the 32-endpoint verification task above, plus provider gaps (OpenAI `RetrieveVectorStoreFileContent`, `SearchVectorStore`), no-continuation (OpenAI `ListModels`), not a list (Gitea `GetTagReference`), and no evidence (ElevenLabs `ListModels`). No U row was verified: network verification stays unauthorized.
10. **Deviations.** See above.
11. **Open questions (new).**
    - Should the model accept a page-size minimum of 0 for OpenAI's `0..N` limits? Recommended: no; a zero page is never useful for traversal, and accepting it would weaken `InvalidPageSize`.
    - Should documented defaults be authored on the 8 OpenAI API-default rows? Recommended: no; `server_default` is documentation only.
    - Gitea's max of 50 is the default `MAX_RESPONSE_ITEMS`, which a self-hosted server can raise; a caller asking for more on such a server is refused client-side. Recommended: keep the authoring table's 50 until a user needs more, then drop the max.
    - sniff clamps `list_workflow_runs`'s limit to 1..=100. Recommended: keep; sniff's callers ask for 2–5 runs.
12. **Drift.** `schematic/definitions/README.md` Gitea pagination default (50 → 30, max 50) and the GitHub module's `per_page` note (still accurate on the wire; left). The emitter comment beside the empty pagination fields was rewritten for the overlay. `sniff/docs/repos/repo-tech-design.md` showed the removed `.per_page(..)` call.
13. **Pre-authorized actions to reverse.** Regeneration of six `requests.rs` files (diff above), the re-import of `openai/mod.rs` (hint line and tail only), and the sniff and schema-test edits. The `ListAuthUsers` fix was not taken.
15. **Adoption rule.** Every adopted field has its knob: page → `.page()`, size → `.page_size()`, cursor parts → `.after()`/`.before()` (resumable). Kept for want of a knob or by design: Bitbucket `page` ×8, `GetSource` `page`/`pagelen`, OpenAI `order`, ElevenLabs `ListVoices` `page_size`/`page_token`. A caller that set one page through a removed field now uses `fetch_page()`, as sniff does.

## Phase 7

Phase 7 re-checks the regeneration, adds end-to-end evidence through the committed generated crate, and runs every dependent. Started 2026-10-13 on `feat/modern-config`, macOS host. Phase 6 already regenerated `schematic/schema` and migrated the call sites the regeneration broke (see "Regeneration pulled forward from 7.1"), so 7.1 here is a verification pass plus one generator change.

### 7.1 Generated migration (verified; one generator change)

- **Generator change: `trybuild` is a generated dev-dependency.** The compile-fail suite of 7.2 must live in `schematic-schema` (R31) and run under `just test-schema`. `schematic-schema/Cargo.toml` is generator-owned, so the dev-dependency is added to the template in `schematic/gen/src/cargo_gen.rs` (new L1 test `generate_cargo_toml_includes_trybuild_dev_dependency`) and `schema/Cargo.toml` was rewritten by `just generate`, never by hand. Test sources stay in `schema/tests/`, which the generator neither writes nor deletes; `schema/Cargo.toml` has no `autotests = false`, so a new test file is compiled without registration.
- **Dependents discovered from manifests,** not from the spec's list: `grep -rl schematic-schema --include=Cargo.toml` finds `sniff/lib`, `biscuit-speaks/lib`, and five homelab crates (`homelab/lib`, `eversolo-integration`, `arcam-amp-integration`, `unfolded-integration-helper`, `sony-receiver-integration`). `model-citizen` does **not** depend on `schematic-schema`; it depends on `schematic-definitions` (`model-citizen/lib/Cargo.toml`), whose Hugging Face module Phase 6 edited, so it is tested in 7.3 as a definitions dependent.
- **Field-level removal list, re-derived from `git diff a9184b381^ -- schema/src/<api>/requests.rs`** (removed `pub <field>:` lines; no field added): Bitbucket `pagelen` ×8; Gitea `page` ×8, `limit` ×8; GitHub `page` ×9, `per_page` ×9; GitLab `page` ×9, `per_page` ×9; OpenAI `after` ×47, `before` ×9, `limit` ×58, `page` ×11; ElevenLabs none. Total 185, identical to the Phase 6 list.
- **Schema diff statistics against the pre-feature baseline (`a9184b381^`):** 8 files under `schema/src`, +9,800/−3,647 (the six authored `requests.rs`, plus `lib.rs`/`shared.rs` losing the empty `Paginated` marker in Phase 3).
- **Export equality (R29):** `git diff a9184b381^ -- openapi postman` is empty after regeneration: OpenAPI and Postman are byte-identical to the baseline, with no response correction (the EMQX wrapper fix was not taken).
- **Determinism:** `just generate` twice; the SHA-256 over every file of `schema/src`, `schema/Cargo.toml`, `openapi`, and `postman` was identical across the runs. The only file the two runs changed relative to Phase 6's committed output is `schema/Cargo.toml` (the `trybuild` line).

### 7.2 Generated evidence (done)

Both new test files are in `schematic/schema/tests/` (auto-discovered; run by `just test-schema`, which runs every test of the generated crate with no tier filter). They use the **committed** generated clients, which `slow_real_apis_compile_offline` keeps byte-identical to fresh generation.

**`pagination.rs` — traversal through generated code against wiremock (F3: generated client → runtime → HTTP; L1, 14 tests, about 0.1 s in total).** Clients use an empty env-var order, so no host token reaches a mock and no test is `#[serial]`.

| Endpoint (strategy) | Tests and what each asserts |
| --- | --- |
| GitHub `ListReleases` (page number from 1, `per_page` 1..=100, `Link rel="next"`) | `github_page_number_collect_all_returns_every_release_in_order` (ids 1,2,3; wire queries `page=1&per_page=2`, `page=2&per_page=2`); `github_pages_report_each_page_and_stop_after_the_last` (page numbers, `total` `None`, status, no third request, `is_finished`, counters); `github_fetch_page_sends_one_request_with_the_configured_page_and_size` (R11a: `.page(3).page_size(5).fetch_page()` sends exactly `page=3&per_page=5` once although `rel=next` is present; returns `Page<Release>`); `github_ordinary_send_adds_no_pager_input` (plain `.await` sends neither input: no sent default is authored); `github_page_size_outside_the_authored_bounds_is_refused_before_sending` (0 and 101 refused with nothing sent; 1 and 100 accepted on the wire); `github_collect_all_stopped_by_the_page_limit_is_incomplete` (`PageLimitReached`, items of page 1, `error: None`); `github_collect_all_with_a_failing_second_page_is_incomplete` (`HttpError`, page-1 items kept, 502 in the error's metadata) |
| OpenAI `ListBatches` (cursor `after` ← body `last_id`, `has_more`, `limit` 1..=100) | `openai_cursor_collect_all_follows_last_id_until_has_more_is_false` (b1,b2,b3; queries `limit=2`, `after=b2&limit=2`); `openai_fetch_page_resumes_after_the_given_cursor_with_one_request` (R11a: `.after("b2").page_size(5).fetch_page()` sends `after=b2&limit=5` once even though `has_more: true`); `openai_collect_all_without_a_continuation_is_incomplete` (`has_more: true` without `last_id` → `InvalidContinuation`/`MissingContinuation`, one request) |
| Bitbucket `ListTags` (follows body `next`, `previous`, `pagelen` 1..=100, total body `size`) | `bitbucket_next_link_collect_all_follows_next_until_it_is_absent` (v1,v2,v3; the absolute `next` URL is sent as given: `pagelen=2&page=2`); `bitbucket_pages_report_the_body_total_on_every_page` (`Page::total` and `Pages::total` = 3); `bitbucket_fetch_page_sends_one_request_with_the_configured_size` (`pagelen=7` once, total 40); `bitbucket_next_link_to_another_origin_is_refused` (`CrossOriginRejected`, nothing reaches the other origin) |

Observed while writing them (expected behavior, recorded so the docs phase states it): a page-number traversal always sends the start page explicitly (`page=1` even when the caller set none), and a followed next link is sent as the provider wrote it, without re-adding `pagelen`.

**`compile_fail.rs` — the capability suite on the real generated set (trybuild).** `slow_generated_pager_surface_reports_the_intended_diagnostics` runs one pass control (`tests/fixtures/pass/authored_controls.rs`, every authored knob/accessor: GitHub `page`/`page_size`/`page_limit`/`collect_all`/`fetch_page`, `ListWorkflowRuns` `total()`, OpenAI `after`, `ListAuditLogs` `before`, Bitbucket `prev()`/`total()`/`items()`) and 13 compile-fail cases under `tests/fixtures/compile_fail/`, each checked against its `.stderr`:

| Case | Real endpoint | Diagnostic |
| --- | --- | --- |
| `page_on_a_cursor_endpoint` | OpenAI `ListBatches` | E0277 "is not paginated by page number" via `.pager()`; E0599 no `page` on the call type |
| `after_on_a_page_number_endpoint` | GitHub `ListReleases` | E0277 "has no resumable cursor"; E0599 on the call |
| `before_on_a_forward_only_cursor` | OpenAI `ListBatches` | E0277 "has no resumable backward cursor"; E0599 on the call |
| `offset_on_a_page_number_endpoint` | GitHub `ListReleases` | E0277 "is not paginated by offset" |
| `prev_without_a_backward_direction` | GitHub `ListReleases` | E0277 "has no backward direction" |
| `total_without_a_total` | GitHub `ListReleases` | E0277 "has no authored total" |
| `set_page_size_not_adjustable` | Bitbucket `ListTags` | E0277 "has no adjustable page size" |
| `knob_after_pages` | GitHub `ListReleases` | E0599 no `page_size` on `Pages` |
| `pager_on_an_unpaginated_endpoint` | GitHub `GetRepository` | E0277 "is not paginated" (generic send); E0599 no `pager` on the call |
| `session_on_a_sessionless_endpoint` | GitHub `ListReleases` | E0277 "has no server-side session" (`with_session`); E0277 "has no adjustable keep-alive" |
| `partitioned_without_partitions` | GitHub `ListReleases` | E0277 "has no partitions" |
| `no_alternate_selection` | GitLab `ListRepositoryTree` (models a `keyset` alternate) | E0599 no `alternate` on the call, no `keyset` on the pager |
| `adopted_field_is_not_a_request_setter` | GitHub `ListReleases` | E0599 no `per_page` setter; E0560 no `per_page` field (available: `owner`, `repo`) |

- **Session prohibitions on the real set** are proven against sessionless endpoints only: no shipped definition authors a session, so "`pages()` on a session endpoint" and "pages escaping the scope" stay covered by the runtime's fixture-type cases (`pager_pages_on_a_session_endpoint`, `pager_session_pages_escape_the_scope`), not by a real endpoint.
- **Portability.** trybuild rewrites this crate's path to `$DIR` and the runtime's to `$SCHEMATIC_RUNTIME`; no snapshot holds a host path, a line number, or a backslash. The test empties `RUSTC_WRAPPER` (kache rewrites paths to `/kache/workspace/…`, observed in the first run), as the runtime's compile-fail test does. No deviation to custom-text assertions was needed.
- **Known coupling.** Five snapshots include rustc's "the following other types implement trait …" list (first eight implementors, alphabetical, then "and $N others"). Adding a generated endpoint that sorts into the first eight, or a rustc upgrade, changes them; refresh with `TRYBUILD=overwrite cargo nextest run --manifest-path schematic/schema/Cargo.toml --test compile_fail` and review the diff. This is the same coupling the runtime's snapshots already have to rustc.
- **Cost.** Cold: 117.5 s (trybuild builds the crate once more under `schema/target/tests/trybuild`); warm: 7.7 s. It is `slow_` because it runs Cargo; `test-schema` runs it anyway. CI does not run `test-schema` (the generated crate is outside the workspace), so this suite, like the other `schematic-schema` tests, is local evidence.

**D49 and recipes.** `just test` (area): L1 2205/2205 (12 skipped), fresh-output 6/6 (includes `slow_real_apis_compile_offline`, the isolated compile of all 18 APIs, and `slow_generated_types_reject_misuse_at_compile_time` with the fixture pager control), compile-fail 1/1; 82 s. `just test-schema`: 55/55 (40 before + 14 + 1), 14.7 s warm. `just lint-schema`: clean, 34.3 s. `just lint` (area): clean.

**Build time after authoring (one sample, no new campaign).** On an empty target dir under `/tmp`, without kache: full `--all-targets` build 52.2 s (Phase 1 baseline 69.8 s); `schematic-schema` library alone after touching `lib.rs` 49.9 s (baseline 35.5 s). The library-alone figure grew by about 40 %, consistent with +6,153 net lines of generated Rust (mostly OpenAI's 58 `Paginated` impls and call-type forwards); the full-build figure fell, which shows host noise of the same order, so neither number is a measurement of the feature beyond "same order of magnitude". The author already accepted chained setters without per-API features (Default 10).

### 7.3 Dependent evidence (done)

Run from the repository root after the 7.1/7.2 changes, macOS, rustc 1.98.1.

| Dependent (from manifests) | Depends on | Tests | Lint |
| --- | --- | --- | --- |
| sniff (`sniff/lib` and its CLI) | `schematic-schema` | `just test sniff`: 3420/3420 (33 skipped), 95 s | `just lint` (sniff): pass |
| biscuit-speaks (lib, CLI) | `schematic-schema` | `just test homelab model-citizen biscuit-speaks`: 919/919 (27 skipped), 154 s — biscuit-speaks 320, biscuit-speaks-cli 107, homelab 49, homelab-cli 16, homelab-server 70, eversolo-integration 32, sony-receiver-integration 27, unfolded-integration-helper 29, arcam-amp-integration (in the same run), model-citizen 153, model-citizen-cli 17 | `just lint` in biscuit-speaks, homelab, model-citizen: pass |
| homelab (`lib`, `eversolo-integration`, `arcam-amp-integration`, `unfolded-integration-helper`, `sony-receiver-integration`) | `schematic-schema` | (same run) | (above) |
| model-citizen | `schematic-definitions` only | (same run) | (above) |

- **Removed-field uses:** none remain. Every dependent compiles against the regenerated crate, which is the proof; a text search confirms the only surviving `pagelen`/`per_page` mentions are Bitbucket `GetSource` (unpaginated, fields kept by design), the `PaginatedResponse` response struct, and sniff's own hand-written forge client in `sniff/lib/src/remote/focused.rs`, which is out of scope (its retirement is sniff's own spec). The single migrated call site (sniff `list_workflow_runs`, Phase 6) passes its 7 tests inside the sniff run.
- **No dependent edit was needed in this phase.** The homelab crates use only the Eversolo, Samsung, and Unfolded Circle clients and biscuit-speaks uses ElevenLabs `ListVoices`/TTS, none of which adopted a field; their envelope and error handling compile and pass unchanged.
- No pre-existing failure was found; no test was skipped beyond each suite's standing skips.

### Requirement-to-test mapping (Phase 7 scope)

| Requirement | Boundary, rung, tier | Tests or artifact |
| --- | --- | --- |
| R25 generated surface compiles, deterministic | generated crate; isolated offline compile (F3) | `fresh_output_compile::slow_real_apis_compile_offline`; two `just generate` runs with identical hashes; `just test-schema` / `just lint-schema` |
| R29 exports unchanged, request structs change only by removing adopted fields | generator output (artifact review) | `git diff a9184b381^ -- openapi postman` empty; the 185-field removal list (7.1), no field added |
| R10 unauthored knobs do not compile, on real endpoints | rustc over the generated crate (trybuild) | `schematic/schema/tests/compile_fail.rs` |
| R31 generated-pager smoke tests (page number, cursor, next link; `collect_all`; configured one-page sends; compile-fail on the generated surface) | generated client → runtime → HTTP over wiremock (F3, L1 of the generated crate) and rustc over the generated crate (trybuild) | `schematic/schema/tests/pagination.rs` (14), `schematic/schema/tests/compile_fail.rs` (1 pass control + 13 cases) |
| R11a configured single page sends exactly one request with both values | same | `github_fetch_page_sends_one_request_with_the_configured_page_and_size`, `openai_fetch_page_resumes_after_the_given_cursor_with_one_request`, `bitbucket_fetch_page_sends_one_request_with_the_configured_size` |
| R21 `collect_all` complete or `Incomplete`, R13 size bounds, R18 link origin check | same | `github_collect_all_stopped_by_the_page_limit_is_incomplete`, `github_collect_all_with_a_failing_second_page_is_incomplete`, `openai_collect_all_without_a_continuation_is_incomplete`, `github_page_size_outside_the_authored_bounds_is_refused_before_sending`, `github_ordinary_send_adds_no_pager_input`, `bitbucket_next_link_to_another_origin_is_refused` |
| R32 dependents build and pass | dependent crates (F3) | `just test sniff`, `just test homelab model-citizen biscuit-speaks`, area lints |
| Generated manifest carries `trybuild` | generator template (L1) | `cargo_gen::tests::generate_cargo_toml_includes_trybuild_dev_dependency` |

The Input Robustness Matrix does not apply: this phase adds no reader of a file format or configuration.

### Validation run in this phase (macOS)

| Check | Result |
| --- | --- |
| `just test` (schematic area) | pass: L1 2205/2205 (12 skipped), fresh-output 6/6, compile-fail 1/1 (82 s) |
| `just lint` (schematic area) | pass |
| `just test-schema` | pass: 55/55 |
| `just lint-schema` | pass |
| `just generate` ×2 | identical bytes; exports equal to the pre-feature baseline |
| Dependents | see 7.3 |
| Other OSes | not run here. The new tests use loopback wiremock and trybuild's path normalization; the one OS-sensitive surface is the trybuild snapshots, whose paths trybuild normalizes and which contain no separator. `schematic-schema` is local-only evidence (CI never runs `test-schema`), so Phase 8.2 should run `just test-schema` on Linux and Windows through `just cross-check` if it wants that crate's evidence per OS. |

### Deviations in this phase

- **Generator change in a "regenerate" phase:** `trybuild` added to the generated manifest (7.1). Without it, R31's compile-fail case could not live in `schematic-schema`.
- **Session-only prohibitions are not exercised on a real endpoint,** because no shipped definition authors a session (7.2).
- **Build-time comparison is a single noisy sample** (7.2), as the plan asked for no new performance campaign.

### Human review agenda (Phase 7 updates)

4. **Removed request fields and dependent edits.** Re-confirmed 185 removals; no further dependent edit was needed. model-citizen is a dependent of `schematic-definitions`, not of `schematic-schema`.
8. **Compile-time and diagnostic observations.** Library-alone cold build 49.9 s versus 35.5 s at baseline (one sample; host noise of the same size seen on the full build). The real-endpoint snapshots show the custom capability messages ("`ListBatchesRequest` is not paginated by page number", "has no authored total") and rustc's implementor list truncated to eight with "and $N others".
9. **Platform evidence.** macOS only in this phase; `schematic-schema` tests are not run by CI on any OS.
10. **Deviations.** Above.
13. **Pre-authorized actions to reverse.** The `trybuild` line in the generated manifest (template, `schema/Cargo.toml`, `schema/Cargo.lock` +38 lines: `trybuild 1.0.122`, `glob`, `target-triple`, `termcolor`) and the two new test files with their fixtures.

## Phase 8

Phase 8 finishes the documentation, collects platform evidence, and audits acceptance. Started 2026-10-13 on `feat/modern-config`, macOS host (rustc 1.98.1). Two workers ran concurrently on disjoint files: one wrote the docs (Markdown only) and one reviewed Rust doc comments (comment-only edits, `.rs` only). The platform runs went through `just cross-check` / `scripts/cross-check.sh` in the background while both worked.

### 8.1 Documentation drift (done)

**New topic page: `schematic/docs/pagination.md`.** It is written for a developer new to the repo. The caller's side comes first: the three entry points (`collect_all`, `pages`, `fetch_page`), configuring a pager and its controls table, moving one-page callers to `fetch_page` (with sniff's `list_workflow_runs` as the example), compile-time controls with a real diagnostic, a lifecycle diagram, the complete-or-`Incomplete` rule, how the end is detected, the rules every traversal follows (the start page is always sent, next links are followed as written, cross-origin policy, stall detection, page limit), bounds checked before sending, `PagerError` evidence, sessions (a sequence diagram covering open, handle, keep-alive, renew, close grace, cancellation, and panic), and partitions (including the rotating-handle risk). Then the authoring side: the default plus exceptions table, a resolution diagram (row first, then a GET with an explicit `Vec<T>`, then not a list), adoption with a diagram, builders, validation, and imported definitions through `--overlay`. It ends with per-API counts (94 paginated lists) and "Lists left unpaginated": the 32 unverified and definition-gap endpoints, each with provider, id, recalled strategy, and source needed, plus the provider gaps and no-continuation rows, each reason kept distinct. The page names no feature or fix directory, spec, phase, or requirement id (grep-checked). Its five Mermaid diagrams render with `mmdc`. The session and partition examples use hypothetical endpoints, marked as such, because no shipped definition authors either.

**Links:** `schematic/README.md` (topic-docs list and a Key Features line), `schematic/docs/runtime/README.md` (a "You can" bullet and a Pages row; the stale "Planned: paging topic page" item was removed because the page now exists), `schematic/runtime/README.md` (docs table plus the missing `pager` row in Module Layout), and one pointer each in the define, definitions, and gen READMEs. `.claude/skills/schematic-define/SKILL.md` gained a pointer to the page. `schematic/docs/dependencies.md` gained `trybuild` under `schematic-schema` dev-dependencies (added in Phase 7, not yet listed).

**Drift found and corrected (code assumed correct):**

| File | Stale claim | Correction |
| --- | --- | --- |
| `docs/runtime/calling-an-api.md` | opening example, Mermaid chain and `.build()?` example used `list_releases(..).per_page(10)` (no longer a setter); optional-values example chained `.per_page(50)` | `list_pull_requests(..).state(..)` and `.sort(..)`; a short "Paginated lists" section pointing to the topic page |
| `docs/generation/request-side.md` | Gitea example used `.with_page(2)` (adopted, removed) | `.with_state(..)`/`.with_sort(..)` with the query order and wire line corrected; note on adopted fields |
| `docs/runtime/deadlines-and-timeouts.md` | manual loop `.page(page).deadline(..).await?` (`.page()` now returns a pager, which cannot be awaited) | a loop over `get_repository`; the pager deadline paragraph is unchanged |
| `docs/generation/validation.md` | setter-naming example `per_page`/`with_per_page` | `state`/`with_state` |
| `gen/README.md` | `ListReleasesCall` example with `page`/`per_page` setters | `ListPullRequestsCall` with its real setters and the generated pager forwards |
| `define/src/lib.rs` crate docs | `PaginationStyle`/`PaginationResponse` presented as the pagination types | marked legacy, superseded by `pagination::Strategy` |
| `define/src/params/mod.rs` `with_response_pagination`, `has_pagination` | "metadata for potential code generation"; "has pagination parameters" | the signal is unread; `has_pagination` reports only a legacy style, and `resolve` decides pagination |
| `define/src/pagination/resolve.rs` `param_adoptions` | near miss is "the same name" | compared without regard to case |
| `gen/src/codegen/endpoint_methods/mod.rs`, `names.rs`, `pagination/mod.rs` | forwarded-control and reserved-setter lists missing `keep_alive`, `partitioned`, `close_grace`, `with_session` | lists completed to match `PagerForwards::has_control` |
| `runtime/src/pager/request.rs` `build_parts` | followed-link description omitted that the body is dropped | added |
| `definitions/src/bitbucket/mod.rs` module docs | page-based paging with `.with_page(2).with_pagelen(100)` (no longer exists) | follows `next`/`previous`; example `list_pull_requests(..).page_size(100).collect_all()` |
| `definitions/src/github/mod.rs` | "all `List*` endpoints support pagination via `page` and `per_page` query parameters" | the pager manages both through `.page(n)`/`.page_size(n)` |
| `definitions/src/gitlab/mod.rs` | `gitlab_page_number` called "offset pagination" | "offset-based pagination, modeled as a page-number strategy" |
| production `src/` comments in define, gen, runtime | requirement ids `(R2a)`, `(R6)`, `(R7)`, `(R11a)`, `(R14)`, `(R18)` | removed (they point at a snapshot spec) |

The old nonexistent `fetch_all_pages` claim was already deleted in Phase 3 (no occurrence remains outside specs and reviews). The Rust edits are comment-only: `git diff -U0 -- '*.rs'` shows no changed non-comment line. `cargo test --doc -p schematic-define --all-features` passes (174 doctests).

**Drift reported, not corrected:**

- `schema/src/bitbucket/requests.rs` (`GetSource`, two places) documents `pagelen` as "default: 50, max: 100". The text comes from `PaginationStyle::bitbucket()` in the definition data (Bitbucket's real default is 10), so fixing it is a definition change plus regeneration and is out of this phase's scope. Recorded as open question 11.
- Pre-feature requirement ids in sniff (`sniff/lib/src/remote/github.rs:95` "(R11.5)", `:240` "(R11.6)") are left, because the feature did not introduce them.
- `.claude/skills/schematic-define/SKILL.md` is 376 lines, over the ~200-line skill target, which was already true before this phase. Shrinking it is unrelated cleanup.

### 8.2 Platform evidence

Hosts declared on this machine: `BUILD_LINUX=build-linux`, `BUILD_WIN=build-win-native`, `BUILD_WSL=build-win` (no `BUILD_MACOS`; macOS is this host). Every rig run tested the working tree as one throwaway commit. Because that tree is not the head's tree, no run published a CI receipt ("publishing no … observation — the remote tested tree … is not this head's tree"). This is local rig evidence for the reviewer, not reusable CI evidence. CI runs Linux and macOS on the pull request, and the nightly runs Windows and WSL2 over what changed.

**L1 archive mode (`just cross-check <pkg> --os all`, the CI-shaped run):**

| Package | Linux | native Windows | WSL2 |
| --- | --- | --- | --- |
| schematic-runtime | 357/357 (1 skipped) | 353/353 (5 skipped) | 357/357 (1 skipped) |
| schematic-define | 632/632 | 632/632 | 632/632 |
| schematic-gen | 612/612 (14 skipped) | 610/610 (14 skipped) | 612/612 (14 skipped) |
| schematic-definitions | 574/574 | 574/574 | 574/574 |
| sniff | 2424/2424 (31 skipped) | 2375/2376: 1 unrelated failure (below) | 2424/2424 (31 skipped) |

The runtime's real-time deadline, grace, and cancellation tests (margins of at least 100 ms) pass on native Windows in this run, which is the Windows timer-tolerance evidence for R19/R22a.

**Suites that run Cargo themselves** (absent from archives) ran in native mode (`--all-features`, which changes nothing for the runtime and the schema crate, and only adds `terminal-tests` dependencies to gen, which the `slow_` filter leaves out). Results are in the table under "Platform results after the fixes" below.

**Two portability defects found and fixed in this phase:**

1. **trybuild cannot normalize a path dependency on native Windows.** `schematic-schema`'s real-endpoint compile-fail suite passed on Linux (55/55) and WSL2 (55/55), and failed on native Windows: 10 of its 13 cases mismatched, and 3 cases plus the pass control matched. Cause: trybuild 1.0.122 (still the latest; upstream `run.rs` is unchanged) records each path dependency with `Path::canonicalize`, which on Windows returns the verbatim `\\?\B:\…\schematic\runtime`. rustc prints `B:/…/schematic/runtime/src/pager/pages.rs:112:12`, so the `$SCHEMATIC_RUNTIME` rewrite never matches, the line numbers stay, and the gutter widens. Exactly the 10 snapshots containing `$SCHEMATIC_RUNTIME` failed. trybuild clears `RUSTFLAGS`, so a `--remap-path-prefix` workaround is not possible. Fix: `schematic/schema/tests/compile_fail.rs` runs, on Windows only, the compile-fail cases whose expected output names no `$SCHEMATIC_RUNTIME` path, asserts that at least one does, and states the limitation in its module doc. macOS, Linux, and WSL2 still compare all 13. The diagnostic text was otherwise identical on Windows. The runtime's own trybuild suite uses `$DIR` only and is not affected. Recorded in the `os` skill (`windows-paths.md`).
2. **The runtime's trybuild suite exceeded nextest's default 30 s kill on a cold build.** It took 17.0 s on Linux and 24.8 s on WSL2, and was killed at 30.06 s on native Windows. gen's fresh-output `slow_` tests already had a 60 s × 15 override, but the runtime's had none. Added `.config/nextest.toml` override `binary_id(schematic-runtime::l1) & test(/(^|::)slow_/)` with `slow-timeout = { period = "60s", terminate-after = 5 }`. CI never runs this suite (it runs only `test-fresh-output`), so the change affects local `just test` and rig runs only.

**sniff Windows failure (unrelated, not introduced by this feature):** `os::working_processes::tests::a_sibling_directory_sharing_a_name_prefix_is_not_inside` panicked with "the holder is visible at all" after 5.05 s (`sniff/lib/src/os/working_processes.rs:163`). The feature never touched `sniff/lib/src/os/` (`git diff a9184b381^ -- sniff/lib/src/os` is empty). The pagination change to sniff is `remote/envelope.rs`, `remote/github.rs`, and their test, whose 7 `list_workflow_runs` tests passed on all three rigs. The re-run result is recorded below.

**Tooling facts learned** (added to the `os` skill, `build-hosts.md`): `just cross-check` re-splits its arguments, so a filterset with parentheses fails, as the skill already said. Calling `scripts/cross-check.sh` directly still fails on a Windows archive leg, because the remote `_expected_manifest`/`_test` recipes re-split it the same way. A positional name filter works there. Trybuild and fresh-output suites, and the non-workspace `schematic-schema` crate, run on the rigs in native mode, for example `--all-features --manifest-path schematic/schema/Cargo.toml`.

### 8.3 Acceptance audit

**Cited tests exist.** A script extracted every test name cited in the requirement-mapping rows of Phases 2–7 (137 names) and searched `schematic/` and `sniff/` for each. All exist except four. Three are trybuild fixture files, not functions (`pager_pages_on_a_session_endpoint`, `pager_session_no_manual_open_or_close`, `pager_session_pages_escape_the_scope`, all in `runtime/tests/fixtures/compile_fail/`). One was renamed in Phase 5: `session_strategy_has_markers_but_no_outside_scope_iteration` is now `pagination::tests::session_strategy_forwards_with_session_and_its_knobs_but_no_outside_scope_iteration`.

**Requirement-to-evidence map (R1–R36).** Each row points to a concrete test or reviewed artifact. Phase sections above hold the full test lists.

| Req | Evidence | Status |
| --- | --- | --- |
| R1 model | `define/tests/pagination_model.rs` (builders, serde, selectors, no empty default), `pagination_inventory.rs` (prototype port) | met |
| R2 validation rules | `pagination_model` rule tests, each with its valid sibling; gen `pagination_validation` | met |
| R2a managed params exempt from required fidelity | `pagination_tables::a_required_managed_parameter_needs_a_value_to_send`; gen `an_ordinary_send_carries_required_adopted_defaults_and_omits_optional_ones`; runtime R13 tests | met |
| R3 defaults + table, literals untouched | `endpoint_literal_surface::endpoint_and_params_literals_compile_unchanged_and_behave_the_same` | met |
| R4 resolution order | `pagination_tables::{get_vec_is_a_list_…, only_get_with_an_explicit_vec_is_recognized, a_row_decides_before_inference, …}` | met |
| R5 table validation | `pagination_tables::{unknown_row_id_is_rejected_with_the_nearest_endpoint, duplicate_row_id_…, row_list_shape_rules}`; `pagination_coverage::every_shipped_definition_validates` | met |
| R6 alternates modeled, not generated | `pagination_tables::alternates_are_validated_…`; gen `alternates_do_not_change_the_output`; schema compile-fail `no_alternate_selection` | met |
| R7 adoption | `pagination_tables` adoption tests; coverage rows' `adopted`; 185-field removal list; schema compile-fail `adopted_field_is_not_a_request_setter` | met |
| R8 legacy inert | `pagination_tables::legacy_pagination_metadata_does_not_change_resolution`; gen `legacy_pagination_metadata_does_not_change_the_output`; `e2e_generation::legacy_pagination_metadata_emits_no_pager` | met |
| R9 one generic pager, thin forwards | runtime `pager_*` fixtures shaped like generated impls; gen forward tests; fresh-output fixture compile | met (Proposed, adopted) |
| R10 unauthored knobs do not compile | runtime trybuild suite (16 cases) and `schema/tests/compile_fail.rs` (13 real-endpoint cases; all on macOS/Linux/WSL2, the 3 path-free cases on Windows) | met |
| R11 iteration surface | `pager_surface::{each_page_carries_its_number_…, items_flattens_every_page_in_order}` | met |
| R11a `fetch_page` | `pager_surface::fetch_page_sends_one_request_with_the_cursor_and_size_…`; schema `github_fetch_page_…`, `openai_fetch_page_…`, `bitbucket_fetch_page_…`, `github_ordinary_send_adds_no_pager_input` | met |
| R12 end detection | `pager_rules` end-signal tests (stale cursor, missing continuation, zero cursors, totals, checked arithmetic) | met |
| R13 page-size rules | `pager_rules::a_size_outside_the_declared_bounds_is_refused_…_with_zero_requests`; schema `github_page_size_outside_the_authored_bounds_is_refused_before_sending` | met |
| R14 stall and loop | `pager_rules::{a_cursor_cycle_is_a_repeated_continuation, a_repeated_link_…, a_failed_pager_stays_failed_…}` | met (widened rule, Proposed, adopted) |
| R15 composite and inclusive cursors | `pager_rules::{a_composite_cursor_…, an_inclusive_boundary_drops_…}` | met |
| R16 follow-up shape | `pager_collect` follow-up tests; `pager_outside::aip_158_…` | met |
| R17 item location | selector unit tests; `pager_rules::a_malformed_collection_is_a_malformed_success_…` | met |
| R18 cross-origin links | `pager_transport` two-origin matrix; `pager_surface` link tests; schema `bitbucket_next_link_to_another_origin_is_refused` | met |
| R19 shared deadline | `pager_transport` deadline and drop tests; passed on native Windows in 8.2 | met |
| R20 envelope per page | five `pager_transport` envelope tests | met |
| R21 `collect_all` | `pager_collect` reason table (all eight reasons); schema `*_is_incomplete` tests | met |
| R22 sessions only through `with_session` | `pager_session` exit-path tests; trybuild `pager_session_*`, `pager_pages_on_a_session_endpoint` | met (runtime fixtures only: no shipped session definition) |
| R22a close grace | the three grace tests in `pager_session` | met |
| R23 partitions | `pager_partition` (6), `define/tests/pagination_partitions.rs` | met (fixtures only: no shipped partition definition) |
| R24 decode path | Phase 4.1 sample (`from_value` kept, +55 KB for the candidate) | met (Proposed, evaluated) |
| R25 generated surface | gen `pagination::tests`; `fresh_output_compile::slow_real_apis_compile_offline`; `just generate` twice, identical hashes (re-run in 8.3) | met |
| R26 wrapper item types | gen `pagination::tests::wrapper_item_types_are_emitted_as_authored`; `pagination_validation` item-type tests | met |
| R27 coverage of 18 APIs | `pagination_coverage::every_list_endpoint_of_every_api_resolves_as_authored` (+ counts) | met |
| R28 unverified stay unpaginated | `pagination_coverage::every_unverified_list_names_its_recalled_strategy_and_needed_source` (exact 32) | met |
| R29 exports unchanged | `git diff --quiet a9184b381^ -- openapi postman` (re-run in 8.3: equal) | met |
| R30 every mechanism + 10 outside patterns | `pager_mechanisms.rs` (17 tests: every inventoried mechanism); `pager_outside.rs`: AIP-158, Mastodon, SCIM, Salesforce Bulk 2.0, and the six named `unverified_{x_api_v2, wordpress, odata_skiptoken, cosmos_db, salesforce_rest_query, reddit}` | met |
| R31 generated-pager smoke | `schema/tests/pagination.rs` (14), `schema/tests/compile_fail.rs` | met |
| R32 dependents | Phase 7.3 (sniff 3420/3420; homelab, model-citizen, biscuit-speaks 919/919; lints); sniff L1 on three rigs in 8.2 | met |
| R33 verification task recorded | agenda item 5 below; `docs/pagination.md` "Lists left unpaginated"; coverage test names the same 32 | met |
| R34 docs and skills | `docs/pagination.md`; define, gen, runtime READMEs; `schematic-define` skill; grep of the new and changed docs finds no `2026-` directory name | met |
| R35 import keeps pagination | `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition` and the overlay tests | met |
| R36 privacy | `pager_surface` and `pager_transport` sentinel tests; `pager_session` handle and close privacy tests | met |

The same acceptance tests cover the decode path chosen (`from_value`); R24's alternative was not adopted, so no second run was needed.

**Artifact reviews re-run after the last change:** `just generate` produced identical SHA-256 over `schema/src`, `schema/Cargo.toml`, `openapi`, and `postman` before and after, with an empty `git diff` of those paths. OpenAPI and Postman still equal the pre-feature baseline (`a9184b381^`). The endpoint-literal surface test passes. The import-persistence test passes in `just test`. The schema diff size is unchanged from Phase 7 (8 files, +9,800/−3,647). The dependency docs and skill updates are listed under 8.1.

**Input Robustness Matrix:** not applicable. This phase adds no reader of a file format or configuration. The one source change in a test reads trybuild's own `.stderr` fixtures to choose cases.

### Human review agenda (reconciled, all 20 items)

1. **Defaults adopted.**
   - P8 items 1–8 and defaults 9–22 were applied as logged in Phases 2–6.
   - Highlights: a page limit of 100; the widened stall rule; knobs that are infallible and checked before sending; thin forwards on call types; `from_value` decoding; a 5 s close grace; named-struct strategies; adopt-and-hide only with the knob.
   - *Status:* closed for review.
2. **Reconciliation with the runtime-envelope spec.** Done in Phase 1. *Status:* closed.
3. **Design choices.** Phases 2–5 list them: type and entry names, the `PagerError` shape, the `with_session` boxed-future signature, lazy open, constant layout, and the API-local helpers and OpenAI overlay. *Status:* for review.
4. **Removed request fields and dependent edits.** 185 fields; edits in `schematic/schema/tests/github_client.rs` and sniff's `list_workflow_runs` (with `Replies::page`). *Status:* for review.
5. **Authored versus left unpaginated (R33).**
   - Authored: GitHub 9, GitLab 9, Gitea 8, Bitbucket 8, OpenAI 58, ElevenLabs 2.
   - Left unpaginated: the 32-endpoint verification task, listed in Phase 6.5 and in `docs/pagination.md`.
   - No unverified row was verified, because network verification was not authorized.
   - *Status:* awaiting the author's authorization for the verification task.
6. **Re-counted numbers.** Phase 1. *Status:* closed.
7. **Decode sample.** Phase 4.1 (`from_value` kept). *Status:* closed.
8. **Compile-time observations.** Phase 7: library-only cold build 49.9 s versus 35.5 s, from one noisy sample. Phase 8 adds that the runtime trybuild suite's cold build takes 17–30 s or more depending on the host (see 9). *Status:* for review.
9. **OS and environments.**
   - Local: macOS ran everything.
   - Rigs (local evidence, no receipts): Linux, native Windows, and WSL2 ran the L1 archive of every changed workspace package, the runtime's trybuild suite, gen's fresh-output compiles, and `schematic-schema`'s full suite.
   - Made portable: the `schematic-schema` trybuild cases are reduced on Windows (trybuild's path-dependency normalization defect), and the runtime trybuild timeout was raised.
   - Left to CI: the nightly Windows and WSL2 cells for the merged head. `schematic-schema` and the trybuild suites are never CI cells.
   - *Status:* see the final platform table.
10. **Deviations.** Every phase's "Deviations" section, plus this phase's: a test-only Windows reduction of the real-endpoint compile-fail cases, and one repo-wide nextest override. *Status:* for review.
11. **Open design questions.**
    - The questions from Phases 1–6 remain, each with its recommendation.
    - New in this phase: should `PaginationStyle::bitbucket()`'s documented defaults (50, which `GetSource`'s generated docs repeat) be corrected or removed with the legacy types? Recommended: remove them with the legacy types in the definition-API work, since nothing reads them.
    - New in this phase: should the trybuild Windows defect be reported upstream? Recommended: yes, as a small issue or PR that strips the verbatim prefix (dunce) in `run.rs`.
    - *Status:* for the author.
12. **Drift.** Phases 2–4 and 6, plus the 8.1 tables above. *Status:* closed.
13. **Pre-authorized actions to reverse.**
    - The regeneration of six `requests.rs` files.
    - The OpenAI re-import.
    - The sniff and schema-test edits.
    - The `trybuild` generated dev-dependency.
    - This phase: the `.config/nextest.toml` override and the Windows branch in `schema/tests/compile_fail.rs`.
    - *Status:* for review.
14. **Unratified Proposed items.** R14 widened: adopted. R9/R10 generic pager with capability traits and `on_unimplemented`: adopted. S11: `from_value` kept. S12 named structs: adopted. *Status:* awaiting ratification.
15. **Adoption rule.** Every adopted field has its knob. Kept fields: Bitbucket `page` ×8, `GetSource` `page`/`pagelen`, OpenAI `order`, ElevenLabs `ListVoices` `page_size`/`page_token`. *Status:* for review.
16. **Session-close grace.** 5 s, configurable through `close_grace`; the runtime-envelope spec made no ruling. *Status:* closed.
17. **`InvalidContinuation` and the added reasons; wrapper item type.** All eight reasons are tested through `collect_all`. The wrapper item type is an authored `item_type` checked through the registry. *Status:* closed.
18. **Stall-rule widening.** No test depended on consecutive-only detection. Known provider risk: an Elasticsearch scroll id can repeat (Phase 5). *Status:* for review.
19. **Decided provenance.**
    - D55-5: define (Phase 2) and runtime (Phase 4).
    - D56-1, D56-3, D56-5: the landed runtime (Phase 1).
    - D57 item 2, latest handle across partitions: Phase 5.
    - D57 item 3, `fetch_page`: Phases 3 and 7.
    - D57 item 4, the importer overlay: Phase 6.
    - D57 item 5, `SessionError`: Phase 5.
    - D57 item 7, `RedirectFailed`: Phase 4.
    - D57 item 13, `Default`: Phase 1.
    - *Status:* closed.
20. **Rotating-handle partition risk.** Logged in Phase 5 and stated in `docs/pagination.md` "Partitions". No out-of-order invalidation was observed. *Status:* accepted risk, for review.

### Platform results after the fixes (8.2, final)

| Suite | macOS (local) | Linux (`build-linux`) | native Windows (`build-win-native`) | WSL2 (`build-win`) |
| --- | --- | --- | --- | --- |
| L1, schematic-runtime / define / gen / definitions | `just test`: 2205/2205 | pass (archive) | pass (archive) | pass (archive) |
| runtime trybuild (`test-compile-fail`) | 1/1 | 1/1, 17.0 s cold | killed at 30 s before the override; 1/1 in 35.3 s after | 1/1, 24.8 s |
| gen fresh-output (`test-fresh-output`, D49 isolated compile) | 6/6 | 6/6, 146.6 s | 6/6, 186.9 s | 6/6, 188.3 s |
| `schematic-schema` (`test-schema`) | 55/55 | 55/55 | 54/55 before the fix (10 path-bearing snapshots); 55/55 after (3 path-free cases + pass control) | 55/55 |
| sniff L1 | Phase 7: 3420/3420 | 2424/2424 | 2375/2376 (unrelated `working_processes` failure; passes alone, 0.07 s) | 2424/2424 |

"Absence of evidence" items:

- No rig run published a CI receipt, because the tested tree is uncommitted. The nightly Windows and WSL2 cells of the merged head will run them, and `just ci-seed` can seed them from the rigs once the change is committed.
- The rigs' native-mode runs (trybuild, fresh-output, schema) are never CI cells, so this table is their only cross-OS evidence.
- No test takes window focus: pagination has no L2 or L3 test, and `just test-l2` is unaffected.

### Requirement-to-test mapping (Phase 8 changes)

| Changed behavior | Boundary, rung, tier | Test |
| --- | --- | --- |
| Real-endpoint compile-fail suite runs on Windows (path-free cases only) and on every other OS (all cases) | rustc through trybuild over the generated crate (F3, `test-schema`) | `schema/tests/compile_fail.rs::slow_generated_pager_surface_reports_the_intended_diagnostics`. Failed on native Windows before the change (10 mismatches, recorded above), passes after; unchanged on macOS, Linux, and WSL2 |
| Runtime trybuild suite is not killed on a cold build | nextest configuration (`test-compile-fail`) | `compile_fail::slow_compile_fail_cases_report_their_intended_diagnostic`: TIMEOUT at 30.06 s on native Windows before, PASS at 35.3 s after |
| Docs and comments | review | 8.1 tables; the comment-only diff check; the doctests of `schematic-define` |

### Validation run in this phase (macOS, final, after the last change)

| Check | Result |
| --- | --- |
| `just lint` (schematic area) | pass |
| `just test` (schematic area) | pass: L1 2205/2205 (12 skipped), fresh-output 6/6, compile-fail 1/1 |
| `just test-schema` / `just lint-schema` | pass: 55/55 / clean |
| `just generate` | identical hashes before and after; no diff in `schema`, `openapi`, `postman`; exports equal the pre-feature baseline |
| `cargo test --doc -p schematic-define --all-features` | pass (174) |
| `just check-tier-coverage schematic` | no stranded test |
| Runtime in area recipes | `test` runs `_test_all` over define, gen, definitions, oauth, and runtime, then `test-fresh-output` and `test-compile-fail` (runtime) |
| Rigs | the table above |

No pre-existing failure was found in the schematic area. The one red result outside it (the sniff Windows test) is unrelated and passed on re-run.

### Deviations in this phase

- **Test-only Windows reduction** in `schematic/schema/tests/compile_fail.rs`, caused by a trybuild defect (8.2 item 1). This is not a skip of the test: on Windows it still compiles the pass control and every case whose snapshot holds no dependency path.
- **Repo-wide `.config/nextest.toml` override** for the runtime's trybuild test, outside the plan's documentation-and-evidence scope, because a cold Windows run could not otherwise pass `just test`.
- **Both workers were launched as background agents** rather than in the foreground. Both finished and were collected before anything depended on them, so no work was lost.

### Status

**Implementation complete, ready for review.** Every Phase 8 task is done, and every requirement R1–R36 points to evidence above. The feature stays in its active directory. Nothing was committed or staged.
