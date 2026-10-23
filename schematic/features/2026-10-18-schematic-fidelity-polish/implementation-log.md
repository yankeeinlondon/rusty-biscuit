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
message_to_agent: |-
  Phase 7 (module feature flags) is complete; read "## Phase 7" in implementation-log.md.

  - schematic-schema now has one Cargo feature per module group (named for the module, default = all). lib.rs gates each `pub mod` and its doc-table row, prelude.rs each re-export and doc line, with `#[cfg(feature = "<module>")]`; ws_shared compiles under any WS feature. futures-util, thiserror, tokio-tungstenite are optional and enabled only by WS features.
  - The manifest comes from `cargo_gen::ModuleFeatures::new(registry, registrations)`; generate_cargo_toml / generate_cargo_toml_with_paths / write_cargo_toml take `&ModuleFeatures`.
  - Phase 8 (compiled docs): the crate-level Quick Start and the prelude example are still `text` fences naming one API (Anthropic / OpenAI). When they become no_run doctests they compile only with that module's feature; doctests run with default features, but a selected-feature doc check needs the example gated (e.g. `#![cfg_attr(feature = "...", doc = ...)]`, as the table rows are) or written against a module the check enables.
  - Phase 6 changes are still uncommitted in this worktree alongside Phase 7.
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
