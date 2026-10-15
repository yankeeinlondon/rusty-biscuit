---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-14-schematic-definition-api/spec.md
plan: schematic/features/2026-10-14-schematic-definition-api/plan.md
implemented_by: codex/gpt-6.1-sol
started_phase: 1
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
---

# Implementation Log for 2026-10-14-schematic-definition-api (8 phases)

## Phase 1

### Scope and initial state

Implement contracts and safety nets only; do not rewrite shipped definitions. Preserve the preexisting pagination review changes and plan/spec edits. No staging, commits, formatting, CI scope changes, or interactive/focus-stealing tools. Parallel bounded inventory, interface refresh, registration spike, and reproduction work follows the plan's explicitly parallel waves; the coordinator owns serialization, capture, and final gates.

### Requirement-to-test map (before implementation)

| Changed behavior or contract | Concrete evidence planned | Boundary / fidelity / tier |
| --- | --- | --- |
| R17 plain serde on REST definitions and parameters, including runtime policy and legacy pagination | `definition_baseline::shipped_definitions_round_trip` walks every shipped API and endpoint, with repeated JSON encode/decode and equal resolved pagination and caller controls; `definition_serde` exercises parameter variants and malformed representations | Public serde data interchange and downstream resolver; F1, L1 |
| Historical definitions and ordered registry schemas remain unchanged | `definition_baseline` compares canonical JSON values to per-API shipped fixtures before typed deserialization; named differences only; description mutation must be detected | Definitions crate → generator and persisted fixtures; F2, L1 |
| All current lists, legacy declarations, inferred GET arrays and authored POST/alias lists survive | Capture every endpoint's resolution, caller knobs and parameter adoption beside unchanged definitions; retain pagination coverage/table tests | Public definition resolver → generated controls; F2, L1 |
| Rust clients, OpenAPI, Postman regenerate without drift | Existing artifact tests plus new complete Rust output file/byte comparison; `migration-check` combines historical and generated comparisons | Generator → shipped filesystem artifacts; F2, L1; normal CLI reproduction also exercised |
| Stable typed registration design | Isolated spike positive cases and rustc negative cases for paths/type identity/private descriptors, collisions, wrappers and destination validation | Stable compiler and schemars; F2, executable proof |
| Reproduction and prerequisites | Unchanged OpenAI import bytes; full three-output reproduction; concrete inventory and frozen-test map | Shipped OpenAPI → importer → definitions → generator; F2, L1/one-shot proof |

Serde here is an additive data-transfer contract, not a configuration loader. Required collection fields reject absence/null/wrong types; optional fields intentionally define absent and null as None. No permissive defaults, filtering or new configuration formats are introduced. JSON is the persisted baseline format; malformed JSON, duplicate typed fields and trailing content receive targeted negative coverage. Input outcomes will be recorded with tests before final verification.

### Adopted planning rules

Adopt Phase 1's Necessary Rules table: PD1 recursive containers and supported map keys; PD2 structured aggregate collisions; PD3–5 existing request-id/redaction/origin policy; PD6 retain EndpointParams Default and audit the remaining allow-list; PD7–8 required-fact typestate builders and reserved vocabulary; R12 non-exhaustive only after consumer migration; PD17 retain auth_policy and remove oauth_scopes only after a fresh scan; PD19 emitter-first OpenAI then ascending-size batches; PD20 canonical object keys, ordered arrays/registries, 5 MB threshold; PD21 remove REST Schema.module_path only after semantic reader audit. Fallible module-group assembly stays provider-neutral, paths never come from parsing type_name, and reserved structure remains None. These are design commitments for later phases, not shipped builder behavior.

### Reproduction and first targeted proof

`reproduction.md` records two independent normal CLI imports and full generations: both OpenAI files, 42 Rust source files plus Cargo.toml, 15 OpenAPI files and 15 Postman files are byte-identical to shipped artifacts and to each other. No transformation or exemption was needed. Preserved the handwritten overlay and known nine query-path warnings.

Before implementation, `definition_serde` failed compilation with E0277 because EndpointParams did not implement Serialize/Deserialize. Added plain derives to RestApi, Endpoint, EndpointParams, ParamDef, QueryParamType, ParamStyle, CrossOrigin, PaginationStyle, and the public pagination resolution/knob/adoption DTOs needed for captured downstream evidence. The two targeted parameter tests then passed (0.008–0.009 s). No definition factory changed. Existing literals/constructors remain valid.

`artifact_drift::generated_rust_artifacts_are_up_to_date` exercises public batch generation and compares the complete recursive source file set and manifest bytes; all seven artifact drift tests pass, new Rust guard 1.528 s. No permanent artifact allow-list exists.

Raw canonical pretty-JSON capture measured **8,449,975 bytes**, exceeding PD20's 5 MB limit. Adopted biscuit-hash BLAKE3 over compact recursively key-sorted JSON (array order retained), separately for API metadata, each endpoint, and each ordered registry entry's full schema content. Keep readable definition and pagination JSON alongside fingerprints for field-specific historical comparisons; registry entries retain their names and insertion order. This makes the baseline reviewable while avoiding repeated deeply nested schema text. `capture_definition_baseline` is an explicit one-shot example; normal tests and migration-check never write fixtures.

### Interface/inventory checkpoint and registration design

Wave 1 completed without a prerequisite hold. See `prerequisite-interfaces.md` for actual validator/diagnostic/isolated-compile interfaces, EndpointSpec and ABI 4, exact CrossOrigin constructors, overlay, adoption/resolution, all six R27 entries, and the retained no-GraphQL assertion. The three pagination author rulings remain invariants: separately opened partition sessions only, global-only JSON paging callback, and explicit optional retry retaining failed-position evidence without redelivery. No frozen test was retired in this phase.

`inventory.md` independently verifies 18 APIs, ordered endpoint IDs and registries, 547 assembled endpoints / 511 literals, 265 OpenAI, 36 container components, 35 legacy declarations, 94 pagers, 32 verification-task endpoints, nine query paths, five WebSocket definitions, Basic users Bitbucket/EmqxBasic, default allow-list, external literals and generated-client consumers. All shipped oauth_scopes/auth_policy values are None; their real readers are recorded before later removal. No shipped GET has a body. `Schema::full_path` has a semantic WebSocket reader in ws_codegen/plan.rs; preserve it when retiring REST module_path usage. No estimates needed correction.

The stable registration proof (`spike/README.md`) passed five nextest tests, a real separate consumer compiling six emitted nominal/alias/reexport/container/foreign/generic paths, three expected rustc failures, and clippy. Selected `Registration: JsonSchema + 'static` with local nominal implementations emitted beside type declarations and disjoint container/native implementations; no blanket overlap or second provider registration list. `register<T: Registration>()` and `register_as<T: Registration>(RustPath<T>)` return fallible private descriptors; the latter rejects containers. `rust_path!` carries actual T identity and supports concrete generic paths. Type names are diagnostic-only. First canonical registration fixes spelling/order; overrides must precede implicit discovery. Separate recursive wrapper metadata leaves reserved structure None. The prototype uses Option<Value> for the reserved field solely for isolation; production must use existing Option<TypeRef>.

Rust's orphan rules require foreign-type adapters in the trait-owning crate (proven Duration and Range<u64> adapters), rather than allowing provider crates to implement the trait for arbitrary foreign types. This is an explicit additional registration bound allowed by R1, not a claim of blanket foreign JsonSchema support. Carry that constraint and complete primitive/integer-map-key coverage into Phase 2. No human decision is needed before proceeding.

### Captured safety nets

Compact fixtures total **1,270,391 bytes**, one file per API. They preserve every definition, endpoint order, list resolution/origin, caller knob, adoption result, and ordered registry schema fingerprint, including binary Bitbucket GetSource as NotAList and all 35 inert legacy declarations. Historical comparisons occur on JSON Values before current-model deserialization. Exact exception paths include API, array index, endpoint ID/registry name, and field, with required nonempty causes; wildcard/stale exceptions fail. The source-maintained exception list is empty. Current Rust/OpenAPI/Postman artifacts never allow exceptions.

`just migration-check` passed all 32 selected tests (207 other tests correctly filtered), including 18 per-API baseline tests, catalog guard, changed-description control, exact-exception controls, field matrix, repeated filesystem write/read/write/read with equal downstream state, and seven artifact drift tests. Every new baseline test spells its repository fixture in a literal include_str! inside its own #[test], so CI can index the read. The test module is explicitly declared in gen/tests/l1/main.rs; define's ordinary integration target is auto-discovered. All new ordinary tests remain L1 and ran below 2 s locally. The shipped field matrix covers every new struct field's absent/null/wrong/empty/duplicate/trailing outcomes; native booleans versus quoted strings and mixed/all-invalid arrays are included. The documentation records a column per field. Optional missing/null equivalence is intentional DTO semantics, not a new config fallback. Preexisting inner AuthPolicy/EnvMapping serde defaults are unchanged.

### Exact targeted tests added

All tests below are compiled by a live target and selected in L1. No feature gate or tier marker suppresses them.

- `schematic-define::definition_serde::parameter_variants_round_trip_twice`
- `schematic-define::definition_serde::parameter_collections_reject_malformed_json_matrix`
- `schematic-define::definition_serde::legacy_pagination_and_origin_policy_variants_round_trip` (all origin policies, all legacy styles, missing/present cursor limit, zero/u32::MAX, quoted/negative/overflow numeric failures, dependent query outputs)
- `schematic-gen::l1::artifact_drift::generated_rust_artifacts_are_up_to_date`
- `schematic-gen::l1::definition_baseline::{anthropic, artificial_analysis_critpt, artificial_analysis_data, bitbucket, elevenlabs, emqx_basic, emqx_bearer, eversolo, gitea, github, gitlab, huggingface, lmstudio, ollama_native, ollama_openai, openai, samsung_smart_tv, unfolded_circle_core_rest}` — one exact test for each named API and fixture
- `schematic-gen::l1::definition_baseline::baseline_catalog_matches_shipped_apis`
- `schematic-gen::l1::definition_baseline::shipped_definitions_round_trip`
- `schematic-gen::l1::definition_baseline::changed_description_is_detected_at_its_exact_field`
- `schematic-gen::l1::definition_baseline::shipped_json_field_matrix_preserves_required_and_optional_shapes`
- `schematic-gen::l1::definition_baseline::historical_exceptions_only_accept_the_exact_leaf_with_a_cause`
- `schematic-gen::l1::definition_baseline::historical_exception_requires_a_cause`
- `schematic-gen::l1::definition_baseline::historical_exception_rejects_wildcards`

The spike's five exact tests and three negative compiler examples are mapped in `spike/README.md`; the explicit proof runner also compiles actual emitted paths in a separate temporary consumer. Runtime context logic is F1/L1; the real compiler, Serde, shipped corpus and generator consumers supply F2 evidence across crate/persistence boundaries. No terminal/browser/device boundary is involved.

### Final verification and skipped coverage

All local evidence is on macOS. No Windows, Linux, or WSL2 host was exercised: this phase adds portable derives, canonical JSON/hash comparisons and temporary-file tests, with no OS-conditional code or path-normalization change. Cross-platform CI scope is unchanged.

- Final `just test`: **2,259/2,259** selected ordinary L1 tests passed; all six existing fresh-output/compile guards passed, then the runtime compile-fail guard passed. This includes unchanged OpenAI overlay re-import and original pagination coverage/resolutions. Full command exited 0.
- `just lint`: all five area packages passed; rerun after the final example/test edits, exited 0. No formatting command ran.
- `just doctest`: define 174, definitions 63, gen 18, oauth 3, runtime 1 passed (**259 passed** total); two preexisting ignored WebSocket documentation examples stayed ignored (core_ws and integration_ws module docs, both line 7).
- `just migration-check`: **32/32** selected tests passed; 207 unrelated tests filtered. Fixture-specific include_str! reads now sit directly inside each test, not inside a macro/helper. No accepted historical differences or artifact exemptions.
- `just test-schema`: **55/55** passed, including real shipped generated paging and compile-fail contracts; no skipped tests. Its preexisting trybuild compiler guard took 75.855 s and was reported SLOW. The area's existing six compiler guards also exceed the ordinary 5 s test budget in some runs; this phase adds no slow test or timeout override.
- `just check-tier-coverage schematic`: passed, **zero stranded tests**.
- Two preexisting tests named `openapi_strict_completeness::{real_registries_still_pass_strict_completeness, real_module_registries_still_pass_strict_completeness}` are excluded by the normal L1 name filter despite using local fixtures. Ran both explicitly: **2/2 passed**, 0.109–0.111 s. Kept their existing names/scope unchanged.
- Of the normal L1 run's 12 excluded tests, seven are the compiler guards subsequently run by `just test`, two are the strict-completeness tests explicitly run above, and three remain preexisting ignored tests: `e2e_generation::{generated_code_compiles, generated_code_passes_clippy, multiple_apis_compile}`. Their existing ignore reasons are slow Cargo compile/clippy work; the active fresh-output guards and generated-schema suite supply compilation evidence. No new ignored test was added. Counts of 233/339 tests filtered in the dedicated compiler runs are ordinary selection, not failures.
- The explicit capture example produced a second temporary capture matching all 18 committed fixtures byte for byte; invoking it again on the existing destination failed as intended and left all bytes unchanged. Capture now refuses to overwrite existing evidence.
- `git diff --check` passed. No preexisting or new test/lint failure remains. No test was deleted or replaced.

### Closing state

Phase 1's five GFM work items are checked as completed; all later-phase work items remain unchecked. Both plan and log carry complete source/test-input, documentation, skill and package inventories; spec has `human_review: false` and a fresh Phase 2 handoff. Updated the current serialization/migration topic, README link, dependency records and both Schematic skill entry points; corrected the skill's stale “no serde” statement. Adopted planning rules remain decisions for later phases, not a claim that builders have landed.

The preexisting pagination edits and the author's concurrent lifecycle move were preserved; this implementation did not move any feature or invoke `just complete`. No shipped API factory, generated Rust/OpenAPI/Postman output, public request Default, runtime behavior, or CI scope was rewritten. No staging, commits, cargo fmt, focus changes, unanswered prompts, or abandoned agent work. **Phase 1 implementation complete, ready for review.**

## Phase 2

### Behavior-to-test map (before implementation)

This phase is additive. Existing factories, importer output, request defaults, pagination, and generated artifacts remain the baseline. Success is typed registration plus a neutral ordered registry adapter and a module-group assembly seam, verified through public results.

| Behavior / boundary | Concrete test planned | Fidelity / tier |
| --- | --- | --- |
| Canonical registration, schema/Rust-name separation, aliases, renamed types, collision aggregation, explicit unused types | define `typed_registration` nominal/order/alias/collision tests; definitions adapter compares existing registries | F1 L1; schemars/registry consumer F2 L1 |
| Vec/Option/HashMap recurse; all primitives and Value remain inline; unsupported keys fail without registering values | define `typed_registration` wrapper/primitive/key tests with nested aliases and error context | F1 L1 |
| Private descriptors and typed paths cannot be constructed with strings, structural setters, misspellings, or mismatched types | compile-fail doctests compiled by `just doctest` using the real compiler | F2 L1 compiler gate |
| Descriptor Serde, destination name/path/wrapper checks, unsupported reserved TypeRef rejection | define `typed_registration` descriptor matrix and repeated JSON round trips; errors include API/endpoint/type | F2 L1 Serde boundary |
| Shared module groups retain ordered entries, nested extraction and unused components; isolated groups do not collide | definitions `typed_registry` fixtures including shipped Ollama, EMQX, HuggingFace aliases | F2 L1 cross-crate |
| Fully qualified typed fixture paths compile in generated clients | extend existing fresh-output fixture/isolated compiler guard | F2 existing compiler gate |
| All shipped definition/registry artifacts remain unchanged | existing definition_baseline corpus, artifact_drift, import_overlay; `just migration-check` and `just test` | F2 L1 shipped corpus and normal generation |

Descriptors are transfer data rather than configuration. The plan explicitly requires the descriptor JSON matrix, which covers missing/null/wrong/empty/duplicate/trailing data and semantic forgery through the public validation result. No persisted storage is introduced; repeated Serde round trips cover the transfer boundary. No terminal, browser, device or platform-specific behavior is added.

### Wave 4 complete

Added the disjoint registration dispatcher, typed rust_path token, private serde descriptor with reserved Option<TypeRef>, ordered neutral registrations, and contextual DefinitionError. Ordinary nominal declarations use definition_type!; the type's schema name remains independent from its proved Rust identifier. No type_name parsing occurs. Foreign adapters for Duration and Range<u64> live in the trait-owning crate. All 12 integer widths, String, bool, char, floats, unit and free-form Value stay inline; containers recurse, and maps check keys before registering values. Nullable wrappers start with an anyOf null alternative and run the existing OpenAPI 3.0 schemars transforms, preserving the same nullable/enum-null encoding as the registry.

The first canonical registration retains its name/path/order. Distinct schema-name conflicts immediately return an error and are collected for finish(); rejected duplicates never replace a schema. Destination validation matches complete previously registered shapes; matching imported data passes with no serialized context identity. Structure is reserved and any non-null structural value fails validation. Descriptor accessors expose transfer data, so later API builders must invoke validate_reference before lowering.

Initial targeted compilation exposed one missing test import; fixed it. All nine typed_registration tests now pass (0.011 s each). Build Context is checked off.

### Wave 5 adapter and compiler boundary

Added SchemaRegistry::from_context and assemble_module_group in schematic-definitions. Both reject collected collisions, preserve ordered neutral schemas, retain existing recursive nested-def extraction, and return explicit registrations even for an empty endpoint inventory. The callback owns the APIs, so no second catalog or reverse dependency is introduced. The four adapter tests pass, covering real HuggingFace Commit/CommitInfo identity, the Ollama progress-alias pattern, shared Ollama and EMQX factory groups, isolated same-name types, collisions, order, nested schemas and unused components. Adapt Registry is checked off.

Discovered that shipped PullModelProgress/PushModelProgress lack JsonSchema. The plan calls for small fixtures before converting those APIs, so the progress-alias case uses a small nominal fixture instead of changing shipped schemas; the real HuggingFace type is exercised directly with a test-only adaptation. The actual Ollama/EMQX factories run through the additive assembly seam with fixture registrations, not through migrated builders. Later migration must add the missing progress derive as appropriate.

Extended the existing isolated fresh-output compiler/runtime guard with a registered Option<Vec<HashMap<String, Duration>>> response. Its generated endpoint compiles outside the checkout and decodes both a nested duration and null through the real runtime against wiremock. The initial guard passed in 62.676 s (the existing slow compiler tier); no new compiler suite or timeout was added.

Compile-fail coverage is in define module doctests and, with exact expected error codes and diagnostic names, the existing generated-crate misuse guard. This departs from the spec's suggested separate trybuild case: the predecessor already batches diagnostic-checked invalid cases into one isolated Cargo invocation with a compiling positive control, so reusing it avoids a new slow suite/dependency and still proves the same compiler boundary. No test or coverage obligation was dropped.

### Final review and gate correction

Expanded declaration coverage to named/tuple/unit structs and enums, including a nominal collection wrapper that remains a component. Expanded the descriptor matrix with genuine context-produced named, inline String/Value, map, Vec and Option controls; required nested identities, native versus quoted wrong scalar types, mixed/all-invalid arrays, duplicates (including reserved structure), missing/null, empty, trailing input and valid-but-unsupported structural combinators all go through public destination validation. Repeated read/write/read preserves Rust paths and wire schemas. The documentation table names every load-bearing descriptor field; registration has no defaulted identity fields or error-swallowing parser fallbacks.

Added DefinitionError::diagnostics(api) with the planned exact one-line-per-collision wording. The original full L1 gate passed 2,273 tests; its compiler misuse guard correctly rejected the misspelled path but exposed a test expectation mismatch (actual E0425, expected E0412). Corrected the expected code, retaining the exact failing input and diagnostic-name assertion. The guard's fail-fast left three subsequent generated compiler tests and the runtime guard unrun in that attempt; the final complete run will execute them. A temporary mutability compile error while applying the existing schema transforms was also corrected before targeted tests passed.

Two verification commands were accidentally invoked from the repository root (lint and doctest). Each was stopped, including its descendant processes, while waiting on build locks; neither is counted as evidence. Verification uses the explicit schematic justfile thereafter. No unrelated source was changed.

### Requirement-to-test results and exact targets

| Requirement | Public observable evidence | Boundary / fidelity / tier |
| --- | --- | --- |
| R1/R4: nominal registration and complete explicit component inventory | Separate schema/Rust names, ordered idempotent entries, nominal tuple/unit/enum models, nested-def extraction, explicit unused types | Real schemars and definitions adapter, F2 L1 |
| R2/R7: transfer data with destination validation | Descriptor JSON matrix; exact names/paths/wrappers/inline schemas; foreign and forged data rejected; matching data and repeated round trips accepted; Some(TypeRef) rejected | Real Serde plus public reference validation, F2 L1 |
| R3: inline wrappers and primitives | All primitive widths; Vec/Option/String-key map around each; integer-key nesting; nullable Value and nominal leaves; unsupported keys leave no registered values | Public wire schema/Rust path/registry state, F1 L1; runtime/compiler fixture F2 |
| R5: aliases and collisions | Renames, aliases/re-exports, foreign/concrete generic paths, distinct same-named types, collected collision order and contextual diagnostic wording | Public registration and actual registry consumers, F2 L1 |
| R6 additive module-group seam (migration remains later) | Shared Ollama/EMQX callbacks and isolated groups, original legacy factories/catalog untouched | Cross-crate adapter and shipped factory boundary, F2 L1 |
| Compiler guarantees | No string constructor/structural setter; unresolvable and mismatched typed paths fail with intended diagnostics; generated nullable nested endpoint compiles and sends | Real compiler/isolated generated crate/runtime, existing compiler gates |
| Historical equivalence | Existing definition_baseline corpus, import_overlay re-import and artifact_drift selected by full just test | Shipped artifacts and normal generator path, F2 L1 |

New ordinary tests, all compiled by live targets and selected in L1:

- `schematic-define::typed_registration::tests::nominal_renamed_alias_reexport_foreign_and_generic_paths`
- `schematic-define::typed_registration::tests::wrappers_inline_shape_nullability_and_order_reuse_leaf`
- `schematic-define::typed_registration::tests::primitives_value_and_integer_map_keys_create_no_components`
- `schematic-define::typed_registration::tests::collision_aggregation_is_ordered_and_never_merges_distinct_types`
- `schematic-define::typed_registration::tests::destination_checks_named_paths_wrappers_inline_and_reserved_structure`
- `schematic-define::typed_registration::every_primitive_and_map_integer_key_stays_inline`
- `schematic-define::typed_registration::destination_rejects_changed_wrapper_shape_even_with_matching_leaf`
- `schematic-define::typed_registration::descriptor_json_matrix_uses_public_destination_validation`
- `schematic-define::typed_registration::string_newtype_key_requires_explicit_contract`
- `schematic-define::typed_registration::same_names_in_different_modules_collide_and_nominal_containers_stay_components`
- `schematic-definitions::registry::typed::tests::aliases_preserve_shipped_huggingface_and_ollama_type_identity`
- `schematic-definitions::registry::typed::tests::shared_ollama_and_emqx_assemblies_reuse_one_ordered_context`
- `schematic-definitions::registry::typed::tests::adapter_preserves_order_nested_extraction_and_unused_types`
- `schematic-definitions::registry::typed::tests::isolated_groups_can_reuse_names_but_shared_collisions_fail_assembly`

`schematic-define` auto-discovers the new integration test file; the definitions tests are unit tests in the declared library module. None has a suppressing tier marker or feature gate. The new child test `typed_registration_paths_preserve_nullable_nested_wire_shape` is added to the existing `REQUEST_SIDE_TESTS` registration guard and executed by `fresh_output_compile::slow_fixture_apis_compile_and_send_offline`. Four new `MISUSES` rows are checked by `fresh_output_compile::slow_generated_types_reject_misuse_at_compile_time` with a positive compilation control. Four corresponding `registration` module compile-fail doctests execute under `just doctest`. The 14 new ordinary tests took 0.020–0.048 s in the final broad run.

Test Registration is checked off: all ten descriptor tests, all four adapter tests, the nullable runtime/compiler fixture, and all four compile-fail doctests have passed. The diagnostic-code correction is in the final full gate. The shared Verify Input Matrices task later in the plan remains unchecked because its JSON/YAML import portion belongs to Phase 5; this phase's descriptor JSON portion is complete.

### Final verification and skipped coverage

All final gates exited 0 on macOS:

- `just -f schematic/justfile test`: **2,273/2,273 ordinary L1 tests passed**, including all 14 new ordinary tests, the original definition/pagination/ordered-registry baseline corpus, artifact comparisons, OpenAI overlay re-import, and existing pagination coverage. All **six generated-crate compiler guards passed**, followed by the **runtime compiler guard**. The corrected misuse guard passed in 31.448 s; the nullable nested runtime fixture passed in 49.426 s. These are the existing compiler gates, not newly slow ordinary tests.
- `just -f schematic/justfile lint`: **all five schematic packages passed**. The final run includes the completed tuple/unit macro coverage, diagnostic rendering, and matrix edits. No formatting command was run.
- `just -f schematic/justfile doctest`: **263 passed**, including the four new compile-fail examples; two preexisting ignored WebSocket documentation examples remain ignored (`core_ws` and `integration_ws` module docs).
- `just check-tier-coverage schematic`: **zero stranded tests**. The definition integration target is auto-discovered; unit modules are reachable; the generated child-test list guard includes the new child test. No new cfg, ignored test, required-feature gate, or tier marker hides coverage.
- Serial follow-up with nextest (`-j 1`) passed all four selected tests: `artifact_drift::generated_rust_artifacts_are_up_to_date` **1.706 s**, `fresh_output_compile::generated_rest_code_references_the_runtime_and_defines_none_of_it` **1.975 s**, and the two preexisting `openapi_strict_completeness::real_*` local-registry tests **0.126/0.115 s**. The first two had taken 5.183/5.119 s during the broad run while doctests were compiling; the serial rerun resolved that contention concern without changing tests, dropping cases, raising timeouts, or adding a slow marker.
- The broad run's **12 excluded tests** comprise seven compiler guards subsequently executed by `just test`, two strict-completeness cases explicitly executed above, and three preexisting ignored generator cases: `e2e_generation::{generated_code_compiles, generated_code_passes_clippy, multiple_apis_compile}`. The active fresh-output guards exercise current generated compilation. No new ignored case was added, and no test was deleted.
- `git diff --check` passed. Frontmatter checks found each required key exactly once in plan/log/spec, and all three Phase 2 GFM tasks are complete. No historical exception or generated-artifact drift exemption was introduced.

No Linux, native Windows, or WSL2 run was added: this phase changes portable build-time type/schema dispatch and Serde data, with no OS-conditional code, filesystem path comparison, terminal, or browser behavior. Cross-platform CI remains unchanged. No live provider/API integration or L2 terminal behavior is claimed; neither is needed for these boundaries. The generated-schema crate was not separately rerun because its committed source and manifest did not change; the existing isolated real-API guard compiled all current generated Rust, including WebSocket modules.

### Closing state

Phase 2 implementation is complete, ready for review. The plan and log frontmatter list all nine changed source/manifest/lock files, six updated documentation files, one new topic page, two updated skill files, and the accumulated packages (`schematic-define`, `schematic-gen`, `schematic-definitions`). Spec frontmatter has `human_review: false` and a fresh handoff explaining destination-shape validation, collision propagation, the additive group seam, and the missing Ollama progress JsonSchema derive. Later phases and the import half of the shared input-matrix task remain pending as intended.

Current authoring behavior is documented in `schematic/docs/typed-registration.md`, with README/dependency/skill updates. Existing shipped REST factories, registries, generated Rust/OpenAPI/Postman files, runtime ABI, request defaults, pagination and WebSocket behavior are unchanged. No staging, commits, cargo fmt, feature-lifecycle move, focus changes, prompts, or abandoned agent work occurred. All started verification processes finished or were explicitly stopped; no verification is pending.

## Phase 3

### Behavior-to-test map before implementation

| Requirement / changed behavior | Concrete public test | Boundary / fidelity / tier |
| --- | --- | --- |
| R8/R9 required facts and no Default | compile-fail builder doctests for missing description/success/auth and Default; `builders` integration tests | Rust compiler F2; ordinary F1 L1 |
| R10/R13 all verbs, success/request forms, repeated success rejection | `builders::verbs_success_and_request_forms_preserve_literals`, `repeated_success_and_invalid_facts_fail` | stored public Endpoint F1 L1; existing generator output/HTTP fixtures F2 L1 |
| R7 destination descriptor validation, wrappers and forgery | `builders::api_checks_transferred_descriptors_and_collisions` | public API assembly and Serde transfer F2 L1 |
| R11 auth coherence independent of call order and multi-method policy | `builders::authentication_mapping_conflicts_are_order_independent` | public RestApi/effective mapping F1 L1; HTTP generation F2 L1 |
| R8 Default allow-list and EndpointParams::new | `builders::default_allow_list_matches_public_source` and updated literal-surface coverage | CI-indexable shipped source corpus F2 L1 |
| R10 unchanged shipped artifacts and emitter | existing definition_baseline, artifact_drift, import_overlay corpus; normal `just test` | shipped definition/export/import/compiler F2 L1 |

These builders write definitions rather than read configuration, so the input robustness matrix does not apply. Transferred descriptors remain untrusted and need complete shape checks at API build. No persistence or OS-specific behavior is introduced. Existing legacy pagination tests stay until Phase 6. Wave 7 permits parallel ownership; one bounded fixture agent handles only the six named generator fixture files and is joined before completion. All work is non-interactive.

### Builder implementation and lowering

Added all seven verb constructors, independent description/success typestate, four Success forms and their shorthands, private ErrorSet::none, and all five stored request families. API build enforces description/auth typestate, records repeated auth choices, checks explicit environment conflicts independent of order, and preserves explicit multi-method policy authority. URL validation uses the existing workspace `url` crate as a new direct define dependency; dependency docs updated.

A required R7 coupling was absent from the old stored model: lowering only to a Rust string would lose schema-name/wrapper evidence before destination assembly. Added optional `Schema::descriptor` evidence, omitted on legacy serialized schemas, and Eq on descriptors. API build validates complete descriptor identity/shape and its agreement with the lowered Rust path, including JSON form fields. This is additive to the existing model rather than a process-local registry or hidden global state. Existing constructors leave evidence None. All legacy shipped baselines must remain exact; typed-built schemas intentionally carry evidence. Typed OpenAPI body export now consumes the wire schema through fallible conversion, retaining component names and wrapper/nullability instead of treating fully qualified Rust paths as free-form JSON. This is necessary to keep the new builder's ordinary output correct.

Initial compilation exposed struct-shaped stored request variants and a missing direct URL dependency; corrected both before tests. No formatter ran. Compiler tests reuse the existing diagnostic-checked isolated misuse suite, adding exact E0599/name assertions for all seven required-fact/error-placeholder failures, alongside seven compile-fail doctests.

### Pagination boundary discovered before migration

The frozen literal-surface fixture exposed that old pagination recognition intentionally accepts only the unqualified string `Vec<T>`, while typed descriptors emit `::std::vec::Vec<T>`. Simply migrating the fixture would silently lose its list result. Added a narrow descriptor-aware root-Vec branch to `root_array_item_type`; legacy string rules remain unchanged, and Option/nominal wrapper/non-GET cases still require explicit authorship. The new targeted matrix names these boundaries and checks public resolution/item path/caller controls. This brings forward the additive recognition portion needed by R21 without migrating any provider or pagination table. The comment was corrected to describe both authoring representations.

The Default audit found 20 public default types, including multiline derives for FrameFormat and RequestIdType missed by the preliminary single-line scan. Every existing default represents a valid empty value or complete protocol/tooling default; retained all 20 with explicit reasons in the current authoring topic. The corpus test uses literal include_str! reads of shipped source and compares the exact allow-list. Endpoint/RestApi and their builders remain outside the allow-list; generated request defaults are untouched.

### Waves 6 and 7 progress

The 15 new builder integration tests plus the revised frozen pagination test pass (16/16, maximum 0.016 s). They cover public literal/JSON equivalence, all auth/success/request families, custom content types, both setter orders for bearer/Basic/key mapping conflicts, both orders of username overwrite, multi-method policy, complete destination shapes, missing JSON/form references, schema/name/path/wrapper/structure forgeries, repeated whole-API read/write/read plus destination rebuild, collisions, primitive/free-form inline behavior, metadata order and typed list inference boundaries. Build Endpoints and Build APIs are checked off.

The six assigned generator fixture files now build valid required-fact bases. Tests that intentionally supply legacy string schemas or invalid model states mutate those stored fields afterward, preserving the exact diagnostic inputs rather than adding a public untyped builder escape hatch. Production emitter/provider definitions remain unchanged. All 412 generator library tests passed, followed by the new typed Rust/OpenAPI/Postman normal-output fixture (0.015 s), and cargo check of generator test targets. Migrate Fixtures is checked off. Refreshed the validation documentation example; pagination table/legacy-presence and no-GraphQL tests remain.

The existing fresh-output nested nullable duration fixture now authors its endpoint and API through builders before compiling/sending against wiremock. Its existing child registration and assertions remain unchanged. The compiler misuse matrix gains seven required-fact cases with explicit codes/names; it must pass before Prove Typestate is checked off.

The initial broad gate stopped on the two new exporter test assertions already found by targeted testing; 2,090 later ordinary tests and the compiler guards did not run in that attempt. Fixed the anyOf expectation, used a wrong whole-field type for the malformed wire-schema case, and corrected its empty-endpoint fixture setup. The first lint found Clippy's new_ret_no_self rule on the deliberate typestate constructor; a narrowly scoped allowance documents why Endpoint::new returns a required-fact builder. Final complete gates will rerun; no failure is being accepted.

### Scope and audit review

The WebSocketEndpoint Default is a retained scope exception for existing incremental WebSocket construction, not a claim that an empty id is a complete generated endpoint. The docs state that distinction; this REST-only phase does not break the five WebSocket APIs or their fixture construction. The other defaults represent valid empty metadata or complete protocol/tooling choices. Corrected the preliminary prose for RequestIdType to its actual u64 default (FrameFormat is JSON text).

Endpoint-stage build errors identify the endpoint while the API is not yet known; API assembly/reference and auth errors identify the destination API. Collected collision errors retain DefinitionError::diagnostics(api) from Phase 2 to render one contextual line per collision. No new global context identity or provider registry is introduced. Missing environment fallback credentials stay legal because the established runtime may send anonymously; explicit mappings are checked for contradictions with declared values rather than inventing a new required-credential rule.

### Final reference review

Added negative checks that delayed request/response registration errors cannot be hidden by a later valid setter, and that changing a lowered Rust path fails API assembly. Review also found that accepting a JSON schema with no evidence in the API builder could conceal a stripped descriptor. The API builder now rejects missing JSON descriptor evidence as well as forged evidence. Legacy direct construction/export stays available for the additive migration, while the new authoring boundary is strict. The isolated fixture API validates its typed duration endpoint before inserting its explicitly legacy test endpoints afterward. Added the original stripped-evidence case to the public destination test; no new untyped builder method or hidden registry was introduced.

Schema descriptors are definition transfer data, not configuration. Existing plain Serde remains unchanged for legacy schemas (absence/null means no evidence); the public API builder rejects either missing-evidence representation. This phase's additional tests assert API assembly, not merely deserializer results. Descriptor syntax/field robustness remains covered by the Phase 2 JSON matrix selected by the broad suite. There is no YAML configuration parser, persistence loader or file-writing workflow in this phase.

### Requirement-to-test evidence and exact new targets

Prove Typestate is checked off: all required-fact and placeholder failures passed the real compiler with exact E0599/name diagnostics in `fresh_output_compile::slow_generated_types_reject_misuse_at_compile_time`; the seven corresponding builders module doctests passed. The positive nested-duration fixture compiled and sent through the real generated runtime. No separate slow compiler suite, ignore, timeout override, deleted test, or tier rename was added.

The source/test inventories in plan/log list all 22 source/manifest/lock files touched by this phase. All tests below compile in live targets and run in L1; the isolated compiler children run through the existing live compiler recipe. No L2 resources are needed. The updated literal-surface test replaces only the old Endpoint-literal surface assertion, retaining empty EndpointParams/Default and public unpaginated-list behavior; all legacy pagination table and no-GraphQL coverage remains.

- R8/R9/R13, F2 external authoring boundary L1: `builders::{every_verb_preserves_required_facts_and_empty_parameters, accepted_successes_and_requests_preserve_stored_wire_shapes, repeated_success_and_invalid_required_facts_report_errors, custom_body_content_types_survive_api_assembly}` plus the seven diagnostic-checked compile-fail cases/doctests.
- R10, F2 external stored definition/Serde L1: `builders::{endpoint_builder_equals_legacy_literal_without_normalization, api_builder_preserves_legacy_literal_metadata_and_authoring_order}`. No JSON field normalization is used in those equivalent fixtures.
- R11, F2 external authoring L1: `builders::{auth_forms_preserve_fields_and_explicit_mapping_conflicts_in_both_orders, explicit_multi_method_policy_preserves_authoritative_fallback, basic_and_key_mapping_conflicts_fail_independent_of_setter_order}`. Includes both orders of contradictory username declarations and all supplied key/header/password/user mapping conflicts.
- R7/R3, F2 destination/Serde boundary L1 (synthetic malformed cases F1): `builders::{destination_context_checks_transfers_roundtrips_and_forged_wrappers, missing_destination_references_fail_in_both_form_encodings, api_build_reports_every_registration_collision, primitive_and_free_form_json_successes_remain_component_free}`. Covers original missing/stripped/forged inputs exactly; positive matching controls, two full API roundtrips, request/form downstream checks, stable registry state, delayed errors and changed lowered paths.
- R8 passive shipped source corpus, F2 L1: `builders::public_default_set_equals_documented_allow_list` checks all 62 shipped source modules and all 20 documented Defaults through CI-indexable literals.
- R21/R27 public resolution, F2 L1: `builders::typed_list_recognition_respects_root_array_and_get_boundaries` and revised `endpoint_literal_surface::endpoint_builder_and_parameter_default_preserve_unpaginated_behavior`. Positive GET Vec and negative Option/nominal/non-GET cases preserve item Rust path and caller controls.
- Typed output/normal generation, F2 L1: `schematic-gen::output::tests::typed_builder_fixture_reaches_rust_openapi_and_postman` and `schematic-define::openapi::export::tests::{typed_builder_exports_named_wrapped_request_response_and_form_schemas, malformed_typed_wire_schema_returns_export_error}`. Asserts named references, nullable anyOf, maps, typed form arrays/optionality and missing-component error; malformed whole-field type returns export error without panic. The existing generated nested-duration compiler/runtime child now runs builder-authored source against wiremock and checks decoded nested values and null.
- R10/R24 passive shipped artifacts plus real normal invocation, F2 L1: existing `definition_baseline`, `artifact_drift`, `import_overlay`, strict completeness and fresh-output suites, selected by the full area gate. Historical JSON, ordered registries, generated Rust/OpenAPI/Postman file sets/bytes and OpenAI re-import remain exact.

### Final verification and skipped coverage

Final checks passed on macOS:

- `just -f schematic/justfile test`: **2,291/2,291 ordinary L1 tests passed** (6.389 s total run time), followed by **6/6 generated-crate guards** and **1/1 runtime compile-fail guard**; recipe exited 0. The final diagnostic misuse guard passed in 27.271 s, builder-authored nested-duration compile/send in 35.482 s, query-wire baseline in 11.430 s, and all shipped generated APIs in 7.223 s. These are existing compiler guards, with no new slow test/tier or timeout policy. No ordinary test was reported SLOW.
- `just -f schematic/justfile lint`: **all five packages passed**, exited 0 after the final implementation/test edits. No formatting command ran.
- `just -f schematic/justfile doctest`: **270 passed**, including seven new builder compile-fail examples (define: 174 ordinary + 11 compile-fail; definitions: 63; gen: 18; oauth: 3; runtime: 1). Two preexisting ignored WebSocket module examples remain ignored (`core_ws` and `integration_ws`).
- Final targeted builder/frozen-fixture run: **16/16 passed**, no skips, longest 0.016 s. Final ordinary gate also includes both added exporter tests and the typed Rust/OpenAPI/Postman generator fixture. Expanded delayed-error/changed-lowering/stripped-evidence cases passed through public API assembly.
- `just check-tier-coverage schematic`: **zero stranded tests**. Define integration tests are auto-discovered in declared Cargo targets; generator unit fixtures live in their declared library and the compiler cases in the existing declared l1 binary. No new feature gate or tier marker hides tests.
- Two preexisting strict-completeness tests excluded by their `real_` names ran explicitly with nextest: **2/2 passed**, 0.119/0.109 s (`openapi_strict_completeness::{real_module_registries_still_pass_strict_completeness, real_registries_still_pass_strict_completeness}`).
- The ordinary gate's **12 excluded tests** are the seven compiler guards subsequently executed by the recipe, those two strict-completeness cases explicitly run above, and three preexisting ignored generator cases: `e2e_generation::{generated_code_compiles, generated_code_passes_clippy, multiple_apis_compile}`. Their existing reasons are slow Cargo compilation/clippy; active fresh-output guards prove current generated code. No new ignored case, removed test, retry, timeout override or artifact exemption was added. The dedicated compiler runs' 233/339 filtered tests are ordinary selection.
- `git diff --check` passed. All four Phase 3 GFM tasks are checked; later-phase tasks remain pending. Required frontmatter keys occur exactly once and all 22 inventoried source/manifest/lock files exist. No external schematic_define::Schema literal/import use was found by the final concrete-symbol search outside schematic.

The two earlier failed exporter assertions/setup and the initial constructor Clippy finding were corrected, rerun, and pass. There is no remaining test or lint failure. Repeated final gates were justified by the last missing-evidence validation fix; no passing test repetition was used to conceal a failure.

No Linux/native Windows/WSL2 run was added: changes are portable Rust type/schema dispatch, definition data, URL validation and generator fixtures, with no OS conditionals, path comparisons, terminal/browser resources or host discovery. CI scope is unchanged. L2 is unnecessary for these boundaries. The excluded generated-schema crate was not separately rerun because committed generated source/manifest are unchanged and the area isolated guard compiles all shipped APIs. No live provider calls, focus changes or unanswered prompts occurred.

### Closing state

**Phase 3 implementation complete, ready for review.** Plan/log frontmatter lists the complete source, docs and skills inventories; accumulated packages remain schematic-define, schematic-gen and schematic-definitions. Spec frontmatter has human_review false and a fresh Phase 4 handoff. Current behavior is recorded in the new definition-builders topic, linked from README/typed registration, with dependency records and both Schematic skills updated.

Providers, production import emitter, WebSocket definitions, committed generated Rust/OpenAPI/Postman artifacts, request defaults, runtime ABI 4, legacy pagination contracts and prior-phase work are preserved. Literal prohibition, provider/emitter migration and definition-side runtime/query forms remain later-phase work. No feature lifecycle move, cargo fmt, staging or commits occurred. All delegated tasks returned in this turn, and all final verification processes finished; nothing is left waiting.

## Phase 4

### Behavior-to-test map before implementation

| Changed behavior | Concrete coverage | Boundary / fidelity / tier |
| --- | --- | --- |
| Endpoint origin replacement, invalid origins, caller tightening | definition_forms policy matrix; generated fixture with two loopback origins and sibling refusal | authoring F1 L1; generated compiler/runtime/HTTP F2 L1 |
| Request-id normalization, secret sugar, explicit auth policy | definition_forms metadata validation plus generator emission assertions and generated fixture | authoring F1 L1; generated consumer F2 L1 |
| Explicit parameter locations, wire ref / Rust git_ref, optional beta/recursive | definition_forms stored location tests; generator tests and generated wire fixture | F1 L1; Rust/compiler/HTTP F2 L1 |
| Serialized pager marker, matched control, required default, unmatched rejection | definition_forms adoption matrix, two JSON read/write/read cycles and public API build | definition transfer / resolved strategy F2 L1 |
| GET body rejection, retained path-query warning | definition_forms all request families and methods, existing shipped baseline and diagnostic corpus | F1 L1 plus passive shipped corpus F2 L1 |

The phase writes definition data and adds builder APIs, not a configuration reader; the configuration robustness matrix does not apply. Existing plain Serde compatibility remains, and marker transfer is tested through downstream API assembly. Existing artifact_drift, definition_baseline and normal generated fixture paths cover shipped artifacts. Provider migrations, OAuth scope removal and the hard path-query switch remain in their scheduled later phases. Wave 8 explicitly permits bounded parallel work; generator/runtime emission and generated fixture tests have separate ownership and are collected before closure. All work is non-interactive.

### Definition forms and compatibility

Added serializable endpoint origin replacement plus per-parameter Rust alias, secret and pager-adoption flags. All unused additions omit from JSON. The mechanical source-literal additions are necessary while providers and importer remain on the legacy model; no provider endpoint or parameter behavior was migrated. The generator's literal emitter must carry these fields until its scheduled builder conversion. The existing path-query severity constant already provides the hard-error switch; it stays Warning.

`ParamDef::new` defaults to optional; query/header_param/cookie state location, and path placeholders/body methods retain their existing locations. API request-id and secret-header sugar normalize lowercase and validate HTTP token syntax. `auth_policy` was already wired by Phase 3 and is retained with its generated-execution coverage. API build reuses resolved pagination validation; markers without matching location/name/control fail, including unpaginated endpoints. Required adopted inputs use the existing PagerManagedRequiredWithoutDefault. GET rejects all five body forms at endpoint build; destination API build catches transferred bodies with API/endpoint/GET context. Endpoint build reports the existing `<endpoint>` placeholder because no destination API exists yet. HEAD remains allowed, a deliberate preservation of existing behavior.

The initial targeted definition run passed 8/8; expanded tests additionally cover forged transferred GET bodies, all unmatched marker locations and invalid Rust aliases. JSON read/write/read is repeated twice and destination build rechecks downstream fields. Existing defaults are compatibility semantics, not silent conversion of required collections. No configuration reader was introduced.

The first broad attempt found one compact Endpoint literal lacking its new field; corrected it. The initial linter caught new implementation ordering, collapsible conditions and a first-diagnostic loop; corrected them without formatting. Temporary mechanical edits to extension-only OAuth fields and function return signatures were identified and corrected before verification. These are development failures to rerun, not accepted gaps.

### Runtime wiring and targeted evidence

The generated definition emits per-endpoint policies through additive `ApiDefinition::endpoint_cross_origin(id, policy)`. Runtime selects the effective policy by EndpointSpec::ENDPOINT_ID for first-hop preparation, redirects and check_link, so pagination's absolute follow-up requests use the same replacement. Caller settings validate against the default and every endpoint policy and can only tighten. No runtime ABI bump or redaction implementation was introduced. Parameter secret wire names are unioned into existing SecretNames metadata, including adopted parameters (the original definition, not the pager-filtered request view, supplies metadata).

Rust aliases are separate from wire names through request fields, constructors, setters, default initialization, parameter encoding, generated call names and collision validation. Explicit aliases preserve spelling; inferred names retain existing sanitation. Exports keep the wire name. Empty allow-origin lists emit a typed empty vector so generated Rust has no unconstrained generic element type. Existing Some(AuthPolicy) emission/execution tests remain active.

The JSON matrix adds explicit definition-transfer reader coverage beyond the initial writer-focused assessment: all four new load-bearing fields (`cross_origin`, `rust_name`, `secret`, `pager_managed`) have documented absent/null/wrong-type/element/empty/duplicate/trailing outcomes. The positive control is real builder-produced serialized data with a required adopted limit and sent default. Every edit reaches public API assembly, not just a deserializer return. Optional absence/null is explicitly defined as no policy/alias; absent Boolean flags mean false and null fails. Read/write/read is repeated in the other transfer tests.

The first completed ordinary gate ran 2,199/2,304 tests before stopping on the OpenAI source reproduction check (2,198 passed, one failed; 105 not run, 12 excluded). The mechanical inline additions compiled but differed from the emitter's own exact source layout. Ran the existing `just import-openai` invocation to reproduce the model's new fields; it still reports 265 endpoints, 1,394 models, three existing warnings and no errors. No endpoint merge, parameter move, schema choice or provider behavior change occurred. OpenAI types.rs stayed byte-identical. The artifact tests already passed against historical generated Rust/OpenAPI/Postman. This run's two preexisting whole-corpus generator tests were flagged slow under concurrent compile/lint load; the final gate will run after targeted compiler work settles.

### Requirement-to-test mapping and exact targets

All new ordinary tests are compiled by auto-discovered define integration targets or the generator/runtime's existing declared L1 targets. The generated child names are checked against REQUEST_SIDE_TESTS, preventing silent orphaning. `just check-tier-coverage schematic` reports zero stranded tests. No new tier marker, feature gate, timeout, retry, ignore or test dependency was added.

- R14, F1 authoring and F2 runtime/HTTP L1: `definition_forms::{locations_markers_and_policy_survive_repeated_transfer, metadata_normalizes_header_names_and_rejects_invalid_origins_and_names}`; `redirects::{endpoint_origin_replacement_controls_redirects_links_and_caller_tightening, endpoint_origin_replacement_validates_exact_origins}`; `pager_surface::endpoint_origin_override_reaches_followed_page_and_caller_can_tighten_it`; child tests `endpoint_origin_override_allows_redirects_and_caller_can_tighten_it` and `endpoint_origin_override_governs_followed_pagination_links`. Parent second-origin recordings are exactly `/page-allowed` and `/redirect-allowed`; siblings and tightened callers add no requests. Negative link assertions name CrossOriginRejected.
- R15/R16, F1 emission and F2 generated runtime L1: `codegen::api_struct::tests::endpoint_origin_replacement_and_parameter_secrets_use_runtime_metadata`, `definition_gen::tests::emits_endpoint_policy_and_serializable_parameter_markers`, plus existing `an_explicit_policy_wins_over_the_legacy_strategy`, `the_translated_policy_is_emitted_in_runtime_terms` and generated `request_id_secret_names_and_cross_origin_policy_come_from_the_definition`. Parameter secret markers now drive that generated fixture's existing metadata checks; predecessor redaction logic is unchanged.
- R29/R31, F1 syntax and F2 compiler/HTTP L1: `codegen::request_structs::tests::explicit_rust_parameter_name_preserves_wire_query_and_ordinary_setters`, `validation::tests::rust_parameter_aliases_are_validated_and_collide_by_rust_name_only`, generated `optional_query_alias_and_booleans_encode_once_under_wire_names`. Exact values `feature/x y+z` and `feature/a b%&=ü` retain the wire name ref with Rust git_ref, absent optionals, beta/recursive true and false, one encoding pass, and decoded query checks. Existing required-adopted default/caller replacement and credential repeated-key conflict tests remain.
- R30, F2 definition transfer/resolver L1: `definition_forms::{pager_marker_requires_matching_strategy_and_authored_required_default, unmatched_markers_and_invalid_aliases_cannot_hide_inputs, json_definition_forms_matrix_reaches_public_assembly}`. Covers optional/required × matched/unmatched × absent/present sent default, all three declared locations, no resolved strategy, malformed aliases and two read/write/read cycles. The matrix's public positive control proves all four new fields load-bearing; empty wrong whole shapes and empty valid allow-lists stay distinct.
- R31, F1 authoring and transferred-definition boundary L1: `definition_forms::{every_get_body_family_is_rejected_and_post_is_accepted, transferred_get_body_reports_destination_api_endpoint_and_method}`. JSON/text/binary/multipart/urlencoded GET failures, corresponding POST successes, preserved HEAD behavior, API/endpoint/method diagnostics.
- R27 replaced only its obsolete half: `cross_origin_policy::endpoints_support_cross_origin_override_and_no_graphql_export` retains the no-GraphQL assertion.
- R24, passive shipped corpus and normal real-artifact path F2 L1: existing definition_baseline/artifact_drift/import_overlay and fresh-output guards compare all shipped historical definition data, ordered schemas, generated Rust/OpenAPI/Postman and exact re-imported OpenAI source. The existing `slow_fixture_apis_compile_and_send_offline` owns the new generated child tests (31 child tests, final targeted run passed). Existing shipped-API compiler guard remains selected by `just test`.

Targeted generator/runtime task reports seven passing tests; the separate generated fixture reports three passing parent/coverage guards, including all 31 child tests. Its intermediate wrong PagerError import was corrected to the public pager module path and the final exact-source rerun passed. The doctest gate exposed a missed ParamDef doc literal; updated it and the other live README/import/skill literal examples with the new required source fields. No cargo formatter ran.

### Workspace observation

During final metadata collection the index changed externally: Phase 4 files became staged even though this implementation issued no git add, git stage, git commit or index mutation command. HEAD remained unchanged. Left that index state untouched and collected the complete source inventory with `git diff HEAD --name-only` so external staging cannot erase the inventory. Final log/plan edits remain ordinary working-tree edits. This observation does not affect source bytes or verification.

### Final ordinary gate and performance resolution

All three Phase 4 plan tasks are checked as their evidence completed; later-phase tasks remain pending. The final ordinary area run passed **2,305/2,305** in **5.436 s**, with **no SLOW tests**. Its seven new define tests include final explicit downstream assertions for the adopted query name/location and exposed PageSize caller knob, in addition to API success. The seven earlier slow reports were concurrent verification build pressure, not a new production bottleneck; running without competing verification builds resolved them without changing tests, omitting cases, changing tiers or adding timeouts. The initial full area run had already passed all six generated-crate guards and the runtime compile-fail guard; the final complete recipe is finishing those guards again on final source.

`just doctest` now passes **270 tests**: define 174 ordinary plus 11 compile-fail, definitions 63, generator 18, OAuth 3 and runtime 1. The two preexisting ignored WebSocket module examples remain ignored. The stale validation example was corrected to build valid facts before explicitly corrupting stored API data; this documents the actual strict builder behavior rather than weakening code to satisfy the old example. No implementation behavior changed for that correction.

### Final verification ledger

- Final `just -f schematic/justfile test`: **2,305/2,305 ordinary L1 tests passed**, **no SLOW ordinary tests**, 5.436 s; **6/6 generated-crate guards passed**, 22.677 s; **1/1 runtime compile-fail guard passed**, 1.173 s. The generated fixture compiled and sent all 31 child tests in 5.684 s. ABI guard, exact compiler misuse diagnostics, real shipped APIs, negative compiler control and query-wire baseline all pass. Existing compiler guards necessarily invoke Cargo and retain their established separate recipe; no new slow suite was added.
- The artifact tests pass without baseline exemptions: OpenAI exact re-import 1.788 s; complete generated Rust drift 2.430 s; no-runtime-duplication source corpus 2.577 s. Historical JSON and ordered-registry tests pass. Committed schema/OpenAPI/Postman artifact files and OpenAI types.rs are unchanged.
- `just doctest`: **270 passed**, two preexisting ignored WebSocket module examples (`core_ws`, `integration_ws`). All documentation compile/runtime failures found during this phase were fixed and rerun successfully.
- Explicit strict-completeness nextest selection: **2/2 passed**, 0.117 s each. Their preexisting `real_` names exclude them from the ordinary L1 filter, so they were executed directly.
- The ordinary suite's 12 exclusions are six generator compiler guards and one runtime compiler guard subsequently run by `just test`, two strict-completeness cases explicitly run above, and three preexisting ignored generator tests (`e2e_generation::{generated_code_compiles, generated_code_passes_clippy, multiple_apis_compile}`). Their existing reason is expensive nested Cargo/clippy; active isolated compiler guards cover generated code. No new ignored test or unreported exclusion was introduced. The dedicated guard selections' 233/342 filtered tests and strict selection's 237 filters are ordinary scope selection, not failures.
- `just check-tier-coverage schematic`: **zero stranded tests**. Test placement and CI-visible repository reads remain intact.
- Shipped frozen definition inventory confirms **18 APIs, 547 endpoints, zero GET request bodies and the same nine query-bearing paths** (Gitea GetGitTreeRecursive, GitLab GetRepositoryFile and seven OpenAI Beta response endpoints). This is the passive real-artifact corpus covered by the baseline tests; no hard path-query error was enabled prematurely.
- No Linux/native Windows/WSL2 run was required for this change: portable Rust definition data, URL/header validation and runtime policy selection introduce no OS conditionals, path comparisons or terminal/browser behavior. Cross-platform CI scope is unchanged. No live provider request, focus change or terminal/browser window occurred. L2 adds no relevant boundary coverage here.

The initial compiler/lint failures, OpenAI reproduction mismatch, missed doc literal, stale validation doctest and generated-child import error were corrected. No failing test is accepted. Full passing evidence is on macOS; no entire-monorepo run was attempted.

### Closing state

**Phase 4 implementation complete, ready for review.** Final `just lint` passed all five schematic packages after the final source/test edits. Plan/log frontmatter inventories are verified against all 87 changed source files, 10 updated documentation files, no created documentation files and one updated skill file; packages accumulate schematic-define, schematic-gen, schematic-definitions and schematic-runtime. Each required key occurs once. Spec keeps human_review false with a current next-phase handoff. `git diff HEAD --check` passes, including externally staged content.

All three Phase 4 tasks are checked; later-phase migration and shared import matrix tasks remain scheduled. No cargo fmt, commit, staging command, lifecycle move, new dependency, runtime ABI change or artifact baseline exemption occurred. The external index change is documented above and was left untouched. All delegated tasks returned and all final verification processes completed; nothing is waiting.

## Phase 5

### Behavior-to-test map before implementation

| Requirement / behavior | Concrete test coverage | Boundary, fidelity, tier |
| --- | --- | --- |
| R22 typed fallible emitted builders, ordered nominal registration and generic overlay | definition_gen tests; import_overlay repeated imports and compiled/evaluated small emitted fixture | Source/compiler/crate boundary F2 L1 through established compiler guard |
| R29 shared path normalization, constant and placeholder controls, twin reconciliation, invalid input | Public OpenApiImport JSON/YAML matrix and nine original path fixtures; shipped seven-pair schema gate | Deserialization/definition F2 L1 |
| R31 imported GET request bodies fail with endpoint context | Public import negative fixture | Import/validation F2 L1 |
| R19/R22 OpenAI context assembly and overlay retention | Exact reimport, pagination_coverage, unknown overlay endpoint rejection | Shipped artifact/import/registry F2 L1 |
| R10/R24 preserve order, component schemas, pagination and wire URLs except seven explained merges | Historical definition baseline with exact explained transformations, artifact_drift, fresh-output generated compiler/runtime guard | Shipped artifacts and generated consumers F2 L1 |

The plan permits bounded delegation; shared importer normalization and its input matrix have isolated define source/test ownership. Emitter, OpenAI assembly, artifact integration, docs and closure remain with the coordinator. All work is non-interactive; delegated results will be collected before closure. Existing staged and unstaged earlier-phase files are preserved. A start-of-phase byte copy is retained outside the repository to inventory only this phase.

### Builder emitter and schema gate checkpoint

The targeted emitter regression failed against the old literal form before implementation, then all 30 emitter/model-generation tests passed. Emission now uses typed request/response/form registrations and fallible Endpoint/RestApi builders; `module_group()` produces API and ordered schemas from one context, with existing infallible catalog wrappers. Generic overlays run after builder validation, and their files remain outside output ownership. Empty typed vectors avoid inference failures. Source-level tests formerly requiring literal tails were replaced with builder/overlay contracts rather than retaining obsolete expectations.

Nominal generated declarations implement Registration beside the declaration; true aliases reuse the underlying registration. This exposes historical alias components that were previously registered as independent names for Vec, String, bool and serde_json::Value; their exact inventory and downstream differences will be recorded at the artifact checkpoint. No provider-specific emitter branch or manual OpenAI transform was introduced. The shared import schema gate passed all seven shipped beta pairs before the normal `just import-openai` regeneration was started. Phase 1 and Phase 4 already proved byte reproduction by the unchanged emitter on the then-current source.

Endpoint Collection support is additive for this phase: explicit endpoint pagination takes precedence, while the legacy table remains available for the remaining providers until Phase 6. The historical Exception origin is retained as an interim compatibility representation. OpenAI's overlay assigns endpoint collections and rejects unknown IDs.

Intermediate compile runs overlapped structural edits and briefly reported a missing Endpoint field despite source presence. A direct all-features check and a fresh target selection passed; kache doctor reported healthy wiring. No cache purge, Cargo clean, wrapper change or formatting was used. The importer matrices exposed two test assumptions during its full define run; they are being fixed, not accepted as gaps.

### Seven beta pairs and historical accounting

The pre-regeneration gate resolves references and flattens compositions before comparing both sides. The beta/candidate type is selected on both equal and superset outcomes:

| Removed endpoint | Surviving endpoint | Path | Request gate | Response gate |
| --- | --- | --- | --- | --- |
| BetaCreateResponse | CreateResponse | /responses | superset | superset |
| BetaGetResponse | GetResponse | /responses/{response_id} | equal (no body) | superset |
| BetaDeleteResponse | DeleteResponse | /responses/{response_id} | equal (no body) | equal |
| BetaCancelResponse | CancelResponse | /responses/{response_id}/cancel | equal (no body) | superset |
| BetaCompactconversation | Compactconversation | /responses/compact | superset | superset |
| BetaListInputItems | ListInputItems | /responses/{response_id}/input_items | equal (no body) | superset |
| BetaGetinputtokencounts | Getinputtokencounts | /responses/input_tokens | superset | equal |

Each survivor's omitted beta control uses its ordinary path with no beta pair; beta=true uses the same method/path plus ?beta=true, reproducing the removed endpoint URL. False is an ordinary caller value, not a fixed-value endpoint selector. The generated wire guard now records all seven methods/paths with omitted, true and false controls and checks dependent RequestParts. Generated Rust changes remove seven request/call/client-method families, add beta setters to survivors and select compatible beta body/success types. OpenAPI and Postman remove seven query-bearing paths/operation IDs, add optional Boolean beta query parameters and select the same beta schemas. Existing beta header parameters and compatible declared query fields are retained.

Historical registry comparison preserves every remaining nominal name, schema fingerprint and registration order. The following 21 old alias entries now reuse underlying typed registration and stay inline under R3/R6 (the comparison pins this exact list): ChatCompletionMessageToolCalls, ChunkingStrategyRequestParam, ComputerActionList, CreateEvalItem, EvalGraderLabelModel, EvalGraderPython, EvalGraderScoreModel, EvalGraderStringCheck, EvalGraderTextSimilarity, EvalItemContentArray, EvalItemContentText, FineTuneChatCompletionRequestAssistantMessage, InputMessageContentList, Item, ParallelToolCalls, ToolsArray, TranscriptionChunkingStrategy, BetaItem, BetaComputerActionList, BetaInputMessageContentList, BetaToolsArray. This includes primitive/Value aliases as required by the same no-invented-component rule, not just Vec aliases. All 1,394 generated declarations remain; the registry contains 1,373 nominal components. Definitions retain 258 of the original 265 endpoint IDs in order, dropping exactly the seven rows above. Explicit endpoint collections replace OpenAI's exception rows; ListInputItems uses BetaItemResource, and every other pagination resolution must compare exactly.

The first integrated compiler attempt exposed a large debug stack frame. Per-endpoint helper functions alone were insufficient; bounded batches of 32 typed registrations also proved necessary. With both changes the historical test runs on nextest's default stack (0.772s), rather than aborting. No stack-size override was added. Its only remaining semantic discrepancy was the unrelated CreateRealtimeCall multipart body: round-tripping every path through a sorted Value changed fallback media ordering. That discrepancy is being corrected in the shared normalizer and will not be exempted. The initial isolated importer fixture also exposed missing serde_json in its stand-in manifest, now corrected. These are development failures to rerun.

### Matrix, execution coverage and integration corrections

The JSON and YAML import matrices independently start from the shipped `/responses/{response_id}/input_items?beta=true` operation. The plan now records every consumed field: paths, parameters, parameter name/location, required, schema/schema.type, and GET requestBody. Native versus quoted YAML type names agree; quoted Boolean required values fail. Absent parameter collections explicitly mean zero declarations; null rejects. Unconstrained schema objects/types produce Json only where explicitly defined. Duplicate authored keys are detected before map collection, and trailing content rejects. GET body presence, including null, fails with API/endpoint/GET context. Empty foundation metadata imports remain supported, but the public run_import invocation rejects both JSON and YAML APIs with no usable endpoints before writing.

Exact tests: import_normalization::{json_import_normalization_matrix, yaml_import_normalization_matrix, query_expressions_reject_ambiguity_and_get_bodies, placeholder_moves_to_query_and_preserves_rust_name, twins_merge_only_with_compatible_schemas_and_parameters, shipped_openai_query_schema_gate_and_reimports, flattened_schema_gate_selects_superset_and_blocks_incompatible_twins, declaration_duplicates_null_operations_and_query_conflicts_are_rejected, shipped_unfolded_circle_import_corpus, gitea_recursive_twin_becomes_optional_caller_control, schema_gate_rejects_new_restrictions, referenced_query_declarations_require_resolvable_parameter_objects}. Final delegated targeted evidence: 12/12 passed, 2.323s total, including the actual shipped multipart/media-order regression. Its full define run before that final correction passed 686/686; its final define all-features library Clippy passed.

Endpoint tests: endpoint_pagination::{endpoint_collection_precedes_legacy_row_and_preserves_inheritance, endpoint_collection_and_strategy_errors_reach_api_build, endpoint_collection_serde_optional_shape_and_repeated_round_trip, endpoint_explicit_collection_drives_public_pager_knobs}; overlay negative: openai_pagination::tests::overlay_rejects_unknown_endpoint_id. Optional endpoint Collection defines absent/null as no assignment, rejects scalar/mixed-array/all-malformed-array/empty-object representations, duplicate keys and trailing data. Two round trips re-enter public API build and verify item type and resolved unpaginated state. These are ordinary auto-discovered L1 tests; endpoint builder/transfer tests are F1, with shipped definition consumers and generated execution providing F2. Shipped provider documents supply F3 evidence of their recorded input format, without live network calls.

Integration tests in the declared generator l1 target cover typed emission, query/cookie/OAuth authentication retention, API runtime/parameter forms, generic overlays remaining byte-identical outside importer ownership, normal empty-import errors, and exact OpenAI reimports. The new slow_imported_builders_compile_and_match_public_import uses the established offline FreshCrate harness and live test-fresh-output recipe. It compiles/evaluates the normal petstore import output, compares public API data after the documented typed-evidence lowering and checks registry order/completeness and the generated client call. Its initial expected registry spelling was corrected to the importer's existing deconflicted CreatePetRequest2. Added stand-in dependencies support the generated model implementations; shipped manifests/dependencies are unchanged.

The ordinary full area run first stopped after 1,145/2,320 tests (1,143 passed, two failed) on the registry's legacy rule that skipped fully qualified schema names. The new typed wrapper regression exposed that rule, and completeness now follows descriptor wire-schema component references rather than skipping typed paths. The existing incomplete-registry regression remains active. The first area lint found redundant Ok(builder.build()?) in emitted endpoint helpers; emission now returns builder.build() directly. Regenerated definition source carries that fix. Neither failure is accepted; final gates must pass below. No formatting, signature bypass, timeout extension or ignored test was added.

### Final review of generated documentation and test expectations

The next ordinary run exposed the aggregate pagination-count assertion still counting the removed BetaListInputItems pager. OpenAI now has 57 pagers and the shipped total is 93; exhaustive per-endpoint resolution already passed. The importer request-id assertion still expected a literal field and now checks the equivalent builder setter. The output-warning corpus now derives counts from its exact two remaining query-bearing provider rows (Gitea/GitLab), rather than retaining independent nine/27 constants. These are changed migration expectations, not removed coverage. The following ordinary run passed all 2,320 tests, including historical OpenAI, registry order/fingerprints, exact reimport and artifact drift.

That run reported ten ordinary tests above five seconds under concurrent verification load, so it is not the final performance evidence. The final ordinary gate will run with four nextest workers, matching the repository's stated 2–4-core runner capacity, after source/artifact edits settle. This changes concurrency only: no test, case, tier, retry or timeout is removed or changed.

Review of generated public examples found a new typed-path interaction: a fully qualified body type had been placed inside a grouped schematic_schema import, producing invalid Rust examples (the historical text fence hid it from rustdoc). Added codegen::request_structs::tests::qualified_and_container_body_examples_are_valid_rust first; it failed on the exact emitted ::schematic_definitions::openai::BetaCreateResponse input. The fix imports the request alone and declares a typed default body for fully qualified nominal/Value/container paths. The targeted test passes and the affected OpenAI client artifacts are regenerated. Legacy provider examples are preserved. This is part of the migration's documentation correctness, not an unrelated format cleanup.

### Final test and artifact evidence

Final required test invocation: `NEXTEST_TEST_THREADS=4 just -f schematic/justfile test` passed **2,321/2,321 ordinary tests**, **zero SLOW ordinary tests**, in **17.578s**. All **7/7** generated-crate guards passed in **34.154s**, including the new compiled/evaluated importer output, all 18 shipped APIs, real-client wire capture, misuse diagnostics, ABI rejection, and compiler negative control. The runtime compile-fail guard passed **1/1** in **2.517s**. Compiler guards retain their established separate recipe and time budget because they invoke nested Cargo; no ordinary behavioral test was moved there to hide slowness.

The wire fixture's initial child compilation lacked imports for the six newly exercised request types; those imports are fixed, and the final guard passes all 21 added ordinary/true/false endpoint URL recordings plus the existing reserved/Unicode/empty/repeated query cases. The historical wire row for ListInputItems keeps the same decoded query and URL meaning, with beta moved after its declared include input; this is the sole changed historical row. Gitea and GitLab retain their pre-migration wire bytes. Both initial omitted imports and the earlier literal/count assertion failures were corrected and rerun; no failure is accepted.

Historical OpenAI comparison passes with the exact seven merges, endpoint collection relocation, BetaItemResource item type, typed-evidence lowering and exact 21 inline alias removals. It still compares every remaining nominal registry name/schema fingerprint/order and every other definition/pagination field. All other 17 historical definition baselines pass unchanged. Current generated Rust file-set/bytes/manifest, OpenAPI and Postman drift tests pass with no artifact exemptions. Exact vendored OpenAI reimport passes (both mod.rs and types.rs; two imports are byte-identical). Vendored source bytes are unchanged. No OpenAI-specific emitter branch or recorded provider transform was needed.

`just check-tier-coverage schematic` reports **zero stranded tests**. New define tests are auto-discovered, openapi-gated tests use CI's declared all-features setting, and the new compiler test belongs to the existing declared generator l1 binary with the live test-fresh-output recipe. Repository source literals remain in tests or the established visible fixture paths.

### Final verification ledger and closing state

- `just -f schematic/justfile lint`: **passed all five packages**, with warnings denied. This recipe runs Clippy and did not invoke cargo fmt.
- `just -f schematic/justfile doctest`: **270 passed**, zero failures; two preexisting ignored WebSocket module examples remain (`unfolded_circle::core_ws`, `unfolded_circle::integration_ws`).
- Explicit strict-completeness nextest selection: **2/2 passed** (0.428s and 4.040s). Their existing real_ names exclude them from the ordinary L1 filter, so they were executed directly.
- The ordinary gate's **13 exclusions** are seven generator compiler guards and one runtime compiler guard subsequently executed by just test, those two strict-completeness cases explicitly executed above, and three preexisting ignored expensive generator tests (`e2e_generation::{generated_code_compiles, generated_code_passes_clippy, multiple_apis_compile}`). Active isolated compiler guards cover current generated output. No new ignore or skipped case was added. The compiler selections' 234/342 filters and strict selection's 239 filters are ordinary scope selection.
- No final test or lint failure remains. The initial regression failures, multipart ordering defect, debug-stack overflow, incomplete typed-reference checking, obsolete count/literal expectations, redundant emitted Ok/question-mark lint and omitted wire-child imports were corrected and rerun successfully. Four-worker ordinary execution resolved the measured load concern without changing test cases or deadlines.
- Current emitted changes are confined to `schematic/definitions/src/openai/mod.rs` (typed builders, ordered assembly, seven merges), `schematic/definitions/src/openai/types.rs` (nominal Registration implementations alongside unchanged model declarations), `schematic/schema/src/openai/mod.rs` (258-endpoint overview), `schematic/schema/src/openai/requests.rs` (typed paths, surviving beta request/call/method forms, valid qualified-body examples), `schematic/openapi/openai.json` (merged operations, optional beta inputs, compatible schemas and inline alias removals), and `schematic/postman/openai.postman_collection.json` (merged operations and caller beta inputs). Every other generated module/export/manifest stays byte-identical. Historical fixtures remain unchanged.
- Evidence is local macOS. Linux, native Windows and WSL2 were not separately executed: this phase changes portable Rust data, Serde/import normalization, builder emission and hermetic compile/loopback fixtures, with no OS conditional or path-comparison changes. CI coverage remains unchanged. No terminal/browser/device resource is needed, and no focus-changing test or live provider request ran.

**Phase 5 implementation complete, ready for review.** All three Phase 5 plan tasks and the shared input-matrix task are checked; later-phase tasks remain pending. Required plan/log frontmatter inventories contain **84 source/artifact files, eight updated documentation files, no created documentation files and three updated skill files**. The complete file inventory is the frontmatter source_files_during_phase_5 list above; most non-OpenAI provider/test edits only initialize the additive Endpoint.pagination field while their migration remains scheduled. Accumulated packages retain schematic-define, schematic-definitions, schematic-gen and schematic-runtime. Spec frontmatter has human_review false and a fresh Phase 6 handoff. Metadata keys are unique.

No feature lifecycle move, cargo fmt, staging command or commit occurred. Earlier staged/unstaged work was preserved. All delegated tasks and verification processes finished in this turn; nothing is left waiting.
