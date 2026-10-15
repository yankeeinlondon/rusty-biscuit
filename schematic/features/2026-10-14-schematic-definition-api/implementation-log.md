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
