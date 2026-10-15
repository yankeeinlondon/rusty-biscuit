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
