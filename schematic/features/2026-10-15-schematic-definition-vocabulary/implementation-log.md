---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-15-schematic-definition-vocabulary/spec.md"
plan: "schematic/features/2026-10-15-schematic-definition-vocabulary/plan.md"
implemented_by: "codex/gpt-6.1-sol"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - schematic/features/2026-10-15-schematic-definition-vocabulary/plan.md
docs_created_during_phase_1:
  - schematic/features/2026-10-15-schematic-definition-vocabulary/implementation-log.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/contracts.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/inventory.md
skills_files_updated_during_phase_1: []
packages:
  - schematic-define
  - schematic-definitions
  - schematic-gen
  - schematic-oauth
  - schematic-runtime
human_review: false
message_to_agent: |-
  Phase 4 is complete; all schematic gates pass. Bundles live in
  define/src/bundle.rs (DefinitionBundle::new/with_models/to_json/from_json)
  and the CLI has `bundle` and `export-bundle`. The bundle shape is
  {format_version: 1, api (RestApi with its ordered schemas), models?}: the
  PD7 bindings/schemas maps were folded into api.schemas (see the log). For
  Phase 5: the bundle loader re-serializes and rejects any input member the
  typed value drops, accepting an explicit default only for the keys in
  OMITTED_DEFAULTS (deprecated, secret, pager_managed, path, explicit). If you
  add a field with `skip_serializing_if` (for example on ParamDef), add its
  key there. Then rerun define/tests/definition_bundles.rs and
  gen/tests/l1/definition_bundles.rs. The committed fixture
  define/tests/fixtures/bundles/fixture.json must be regenerated when
  RestApi/ParamDef serialization changes: the drift test writes the fresh
  copy to $TMPDIR/schematic-bundle-fixture.json. Some bundle_json_matrix
  cells match "bundle JSON line N column M", so fix their line numbers when
  the fixture moves. builders::validate_api is now the shared API validation
  for build() and bundles. The four >5 s ordinary L1 tests seen under heavy
  host load pass alone in under 3.5 s.
source_files_during_phase_2:
  - schematic/define/src/auth.rs
  - schematic/define/src/builders.rs
  - schematic/define/src/components.rs
  - schematic/define/src/lib.rs
  - schematic/define/src/metadata.rs
  - schematic/define/src/models/aliases.rs
  - schematic/define/src/models/enums.rs
  - schematic/define/src/models/mod.rs
  - schematic/define/src/models/structs.rs
  - schematic/define/src/oauth.rs
  - schematic/define/src/openapi/export/info.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/parameters.rs
  - schematic/define/src/openapi/export/security.rs
  - schematic/define/src/openapi/extensions.rs
  - schematic/define/src/openapi/import/builder.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/openapi/import/mappings/parameters.rs
  - schematic/define/src/pagination.rs
  - schematic/define/src/pagination/validate.rs
  - schematic/define/src/params/mod.rs
  - schematic/define/src/params/param_def.rs
  - schematic/define/src/prelude.rs
  - schematic/define/src/registration.rs
  - schematic/define/src/request.rs
  - schematic/define/src/serde_presence.rs
  - schematic/define/src/types.rs
  - schematic/define/src/websocket/mod.rs
  - schematic/define/tests/builders.rs
  - schematic/define/tests/composition.rs
  - schematic/define/tests/definition_serde.rs
  - schematic/define/tests/endpoint_literal_surface.rs
  - schematic/define/tests/pagination_support/mod.rs
  - schematic/define/tests/pagination_tables.rs
  - schematic/define/tests/vocabulary.rs
  - schematic/definitions/src/bitbucket/mod.rs
  - schematic/definitions/src/emqx/mod.rs
  - schematic/definitions/src/eversolo/mod.rs
  - schematic/definitions/src/openai/mod.rs
  - schematic/gen/src/asyncapi_import.rs
  - schematic/gen/src/codegen/api_struct/constructors.rs
  - schematic/gen/src/codegen/api_struct/fields.rs
  - schematic/gen/src/codegen/api_struct/tests.rs
  - schematic/gen/src/codegen/api_struct/variant.rs
  - schematic/gen/src/codegen/client/tests.rs
  - schematic/gen/src/codegen/endpoint_methods/tests.rs
  - schematic/gen/src/codegen/module_docs.rs
  - schematic/gen/src/codegen/pagination/tests.rs
  - schematic/gen/src/codegen/request_enum.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/src/definition_gen.rs
  - schematic/gen/src/export/auth.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/tests.rs
  - schematic/gen/src/output/assemble/lib_rs.rs
  - schematic/gen/src/output/mod.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/src/ws_codegen/client.rs
  - schematic/gen/src/ws_codegen/docs.rs
  - schematic/gen/tests/l1/definition_baseline.rs
  - schematic/gen/tests/l1/e2e_generation.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/http_client.rs
  - schematic/gen/tests/l1/openapi_strict_completeness.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/gen/tests/l1/pagination_validation.rs
  - schematic/gen/tests/l1/postman_golden.rs
  - schematic/gen/tests/l1/query_params_codegen.rs
  - schematic/oauth/src/manager/authorization_code.rs
  - schematic/oauth/src/manager/mod.rs
  - schematic/oauth/src/manager/revocation.rs
  - schematic/oauth/tests/wiremock_flows.rs
docs_updated_during_phase_2:
  - schematic/docs/definition-builders.md
  - schematic/docs/definition-snapshots.md
  - schematic/docs/generation/validation.md
  - schematic/docs/headers/README.md
  - schematic/docs/pagination.md
  - schematic/docs/typed-registration.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/implementation-log.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_3:
  - schematic/define/src/openapi/export/components.rs
  - schematic/define/src/openapi/export/conversion.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/parameters.rs
  - schematic/define/src/openapi/export/paths.rs
  - schematic/define/src/openapi/export/security.rs
  - schematic/define/src/openapi/extensions.rs
  - schematic/definitions/src/github/mod.rs
  - schematic/gen/src/codegen/api_struct/auth.rs
  - schematic/gen/src/codegen/api_struct/helpers.rs
  - schematic/gen/src/codegen/api_struct/mod.rs
  - schematic/gen/src/codegen/api_struct/tests.rs
  - schematic/gen/src/codegen/endpoint_methods/docs.rs
  - schematic/gen/src/codegen/endpoint_methods/mod.rs
  - schematic/gen/src/codegen/endpoint_methods/tests.rs
  - schematic/gen/src/codegen/module_docs.rs
  - schematic/gen/src/codegen/request_structs/body.rs
  - schematic/gen/src/codegen/request_structs/derives.rs
  - schematic/gen/src/codegen/request_structs/endpoint_docs.rs
  - schematic/gen/src/codegen/request_structs/into_parts.rs
  - schematic/gen/src/codegen/request_structs/mod.rs
  - schematic/gen/src/codegen/request_structs/named_params.rs
  - schematic/gen/src/codegen/request_structs/param_enums.rs
  - schematic/gen/src/codegen/request_structs/path_params.rs
  - schematic/gen/src/codegen/request_structs/query_params.rs
  - schematic/gen/src/codegen/request_structs/shared.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/src/diagnostics.rs
  - schematic/gen/src/export/auth.rs
  - schematic/gen/src/export/mod.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/collection.rs
  - schematic/gen/src/export/postman/item.rs
  - schematic/gen/src/export/postman/request.rs
  - schematic/gen/src/export/vocabulary.rs
  - schematic/gen/src/output/assemble/api.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/tests/l1/definition_baseline.rs
  - schematic/gen/tests/l1/endpoint_auth_output.rs
  - schematic/gen/tests/l1/main.rs
  - schematic/gen/tests/l1/typed_path_output.rs
  - schematic/gen/tests/l1/vocabulary_exports.rs
  - schematic/oauth/src/lib.rs
  - schematic/oauth/src/manager/authorization_code.rs
  - schematic/oauth/src/manager/client_credentials.rs
  - schematic/oauth/src/manager/mod.rs
  - schematic/oauth/tests/wiremock_flows.rs
  - schematic/openapi/github.json
  - schematic/postman/github.postman_collection.json
  - schematic/runtime/src/call.rs
  - schematic/runtime/src/client.rs
  - schematic/runtime/src/credentials.rs
  - schematic/runtime/src/options.rs
  - schematic/runtime/tests/l1/endpoint_auth.rs
  - schematic/runtime/tests/l1/main.rs
  - schematic/runtime/tests/l1/metadata_boundary.rs
  - schematic/schema/src/github/mod.rs
  - schematic/schema/src/github/requests.rs
docs_updated_during_phase_3:
  - schematic/docs/definition-builders.md
  - schematic/docs/generation/generated-clients.md
  - schematic/docs/generation/request-side.md
  - schematic/docs/generation/validation.md
  - schematic/docs/headers/README.md
  - schematic/docs/io/export-openapi.md
  - schematic/docs/io/export-postman.md
  - schematic/docs/io/openapi-extensions.md
  - schematic/docs/runtime/building-clients.md
  - schematic/docs/runtime/calling-an-api.md
  - schematic/docs/runtime/credentials.md
  - schematic/docs/runtime/README.md
  - schematic/docs/typed-registration.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/implementation-log.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_4:
  - schematic/define/src/builders.rs
  - schematic/define/src/bundle.rs
  - schematic/define/src/components.rs
  - schematic/define/src/lib.rs
  - schematic/define/tests/bundle_support/mod.rs
  - schematic/define/tests/definition_bundles.rs
  - schematic/define/tests/fixtures/bundles/fixture.json
  - schematic/gen/src/errors.rs
  - schematic/gen/src/main.rs
  - schematic/gen/src/pipeline.rs
  - schematic/gen/tests/l1/definition_bundles.rs
  - schematic/gen/tests/l1/main.rs
docs_updated_during_phase_4:
  - schematic/README.md
  - schematic/docs/definition-snapshots.md
  - schematic/docs/generation/workflow.md
  - schematic/gen/README.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/implementation-log.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/plan.md
docs_created_during_phase_4:
  - schematic/docs/io/definition-bundles.md
skills_files_updated_during_phase_4:
  - .claude/skills/schematic-define/SKILL.md
---

# Implementation Log for 2026-10-15-schematic-definition-vocabulary (6 phases)

## Phase 1

### Scope, starting state, and decisions

Phase 1 is evidence and inventory only. No implementation behavior, parser, schema, template, shipped definition, generated artifact, dependency, or test is changed. Existing tests provide the baseline; the next phases' behavior-to-test mapping is recorded in `contracts.md` before implementation begins. No regression is being repaired by this phase, so no new red/green test is appropriate.

The implementation log existed as an empty file at entry. The worktree was already dirty with definition-API review repairs (20 tracked files and three untracked review/log files); these are preserved and excluded from this phase's source-file metadata. HEAD is `a8842e0f9935121543adf650a8ff1eaa5503d97b`; committed tree is `21a263cce41e8c8c16e6a57e6e4a3674b639997f`. The reproducible baseline is this HEAD plus those existing repairs, not HEAD alone. No commit, staging, push, live provider call, CI change, or lifecycle move is authorized or performed.

Adopt PD1–PD3 and PD5–PD10 as planning defaults: validated metadata only; ParamDef typed paths; absent deprecation defaults false; same-context structural registration; complete ordered Basic EnvLists; canonical version-1 JSON bundles; query styles and enum wire spellings; grant-specific OAuth payloads retaining Required/Supported PKCE; unconditional query migration. PD4 is provisional. No author ruling on independent endpoint credential sources was found in the supplied spec/plan or prerequisite review records. The prompt authorizes all independent Phase 1 preparation. It does not settle that public API choice or explicitly confirm the future breaking changes listed under pre-authorizations. Later implementation must resolve those conditions, without blocking this read-only inventory.

GitHub adoption remains limited to rate-limit header names, epoch-second resets, and optional Retry-After; no provider-wide retry statuses or attempts. Although `basic_env(username, passwords)` can keep its signature, shipped Bitbucket and EMQX do not use it: they call `.authentication(AuthStrategy::Basic, ...)` at definitions/src/bitbucket/mod.rs:218 and definitions/src/emqx/mod.rs:157. Turning Basic into a payload variant cannot preserve those exact expressions. Flag this R10/R12 conflict: later implementation needs the narrowly permitted mechanical Basic authoring migration (and existing pattern-match test repairs), with byte-identical generated output, rather than an endpoint rewrite. OpenAI authoring remains unchanged. No spike is needed.

### Prerequisites and contract refresh

- `2026-10-11-schematic-generation-integrity`: completed; review 3 says production ready. Its output-local diagnostics, isolated compiler harness, and artifact comparisons exist.
- `2026-10-12-schematic-runtime-envelope`: completed; review 3 is ready with no human review. `schematic-runtime` owns EndpointSpec, call futures, redaction, redirect/origin rules, and pager request execution.
- `2026-10-13-schematic-pagination`: completed; review 2 is ready. Current provider definitions retain 93 authored pagers. Adoption and sent-default data are available; direct page requests use `fetch_page()`.
- `2026-10-14-schematic-definition-api`: implementation is present and review 3 reports ready, with both earlier findings fixed. Repairs are uncommitted in this working tree. Its spec still has historical `status: human-in-the-loop`, but `implemented: true`, `completed: true`, and `human_review: false`; its handoff mentions author review. This is reviewed working-tree evidence, not a claim of a separate merged commit or author sign-off.
- Reconciled actual interfaces, input outcomes, and future test obligations are in `contracts.md`. The historical claim of a serde structural TypeDesc does not match landed code; the review explicitly accepts the stored TypeRef lowering. Do not reconstruct the prerequisite. Composition/serialization extensions belong in later phases.

### Verification results

- `cd schematic && just migration-check`: PASS, 33 tests. All 18 historical snapshots match; no exception is active. Rust sources plus manifest, OpenAPI, and Postman regenerate in isolated TempDirs and match committed bytes/file sets. Description mutation fails comparison at its exact description and fingerprint leaves. Existing repeated write/read/write/read test passes.
- Exact retained tests: `definition_baseline::changed_description_is_detected_at_its_exact_field`, `definition_baseline::shipped_definitions_round_trip`, `definition_baseline::baseline_catalog_matches_shipped_apis`, `definition_baseline::shipped_json_field_matrix_preserves_required_and_optional_shapes`, all 18 API baseline tests, and `artifact_drift::generated_{rust,openapi,postman}_artifacts_are_up_to_date`.
- New targeted tests: none (no behavior changes). Existing tests are compiled by schematic-gen's declared `l1` target and selected by L1. Filesystem/serde round trips and snapshot mutation are L1/F1; generation through actual writers and Rust compiler consumption is L1/F2 at those boundaries. No live provider F3 evidence is claimed.
- Full `cd schematic && just test`: PASS, 2,287 ordinary L1 tests plus seven fresh-output compiler/request checks and one runtime compile-fail guard (2,295 total). `cd schematic && just lint`: PASS for all five packages. `just check-tier-coverage schematic`: PASS, zero stranded tests. Gates ran once; the migration baseline was a separate focused prerequisite check. No dependent package has been changed by Phase 1; downstream execution is unnecessary for a documentation-only evidence phase. The inherited dirty tree means the whole-tree scope calculator cannot justify skipping the requested area gates.

### Available environments

Loaded the OS skill. Current host is macOS. `BUILD_LINUX=build-linux`, `BUILD_WIN=build-win-native`, and `BUILD_WSL=build-win` are configured; those rigs support later `just cross-check` runs. No connectivity or execution evidence is claimed. Phase 1 adds no OS-sensitive code, so no cross-OS execution is required here.

### Inventory and checkpoint closure

`inventory.md` records all 18 APIs (539 endpoints, 1,818 ordered schema rows), 404 query declarations, 25 required query declarations, 81 enum-bearing declarations, and 93 resolved pagers. The required/enum union spans 106 fields and 63 generated request types. Basic users are Bitbucket and EmqxBasic; no shipped legacy auth or explicit policy contains OAuth2. All affected request references and direct external consumer construction/default/setter/assignment sites are listed by file and line, along with exact manifest package names. `sniff repo packages --package-area schematic --json` confirms schematic-define, schematic-definitions, schematic-gen, schematic-oauth, schematic-runtime, and excluded schematic-schema. Manifests confirm sniff, biscuit-speaks, model-citizen, homelab, eversolo-integration, arcam-amp-integration, sony-receiver-integration, unfolded-integration-helper, and schematic-oauth as the expected external/consumer scope; the inventory includes internal direct edges too.

`contracts.md` supplies R1–R14 requirement-to-evidence mapping, the actual builder/registration/runtime/diagnostic interfaces, field-specific absent/null/empty outcomes, recursive validator/document-shape obligations, an inherited struct-field audit, and a narrow future normalization recipe. Original snapshot files, fingerprints, and tests remain unchanged. No future normalization is enabled before its vocabulary exists. Later bundles need a strict presence-aware wire contract: raw Option serde currently loses explicit-null presence, and raw snapshot nulls must not be mistaken for a valid future bundle preflight contract.

Both Phase 1 plan checkboxes are complete. The baseline/checkpoint is verified on the reviewed working tree; no claim of a separate prerequisite merge or author sign-off is made. The preparation reconciliation is documenting actual TypeDesc/ModelCatalog interfaces and the necessary Basic source-form conflict instead of assuming the draft's proposed carrier/call forms already exist. No source files or skill files were changed by this phase; current runtime and authoring behavior remains unchanged, so no public README/topic/skill update is needed. Only plan.md, this log, contracts.md, and inventory.md were written. The spec is unchanged because no spec frontmatter keys were requested.

### Gate exclusions and timing observations

The full area gate intentionally filters other tiers: ordinary L1 reported 13 exclusions; the separate fresh-output pass reported 235 filtered tests and selected seven; the compile-fail pass reported 342 filtered tests and selected one. These are selection counts, not failed/skipped selected cases. No selected test failed, timed out, or was ignored. No L2/L3/browser/real service test was needed for an evidence-only change. No new or renamed test needs target registration or feature wiring. Dependent packages were inventoried but not executed: no package implementation, generated API, or source test changed in this phase.

Four existing ordinary L1 tests exceeded five seconds under the full gate: `shipped_openai_query_schema_gate_and_reimports` (7.497 s), `artifact_drift::generated_rust_artifacts_are_up_to_date` (6.844 s), `fresh_output_compile::generated_rest_code_references_the_runtime_and_defines_none_of_it` (6.829 s), and `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition` (6.955 s). They passed; they remain pre-existing test-budget violations, not newly authored tests or evidence of a fixed timing defect. The artifact drift test took 4.576 s in the earlier focused baseline run. The recipe's explicitly separated Cargo compiler checks also exceed five seconds; their existing tier/placement was preserved. No retries, broadened reruns, or timeout changes were made.

Raw command logs for this run: `/tmp/schematic-vocabulary-phase1-test.log`, `/tmp/schematic-vocabulary-phase1-lint.log`, and `/tmp/schematic-vocabulary-phase1-tiers.log`. No formatter was invoked. Last completed phase: 1. Replacement-policy implementation remains blocked on the recorded credential-source decision; future breaking scope confirmations remain unrecorded and must not be inferred from this preparation-only run.

### Final scope and metadata verification

After saving phase documentation and metadata, ran the required root command:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `change_class: package`; `packages: [schematic-define, schematic-definitions, schematic-gen, test-toolkit, darkmatter]`. The three source packages are inherited definition-API modifications; other selections are test/suite-input scope. Gates were therefore not skipped. This whole-tree scope must not be confused with Phase 1 source ownership: this session wrote only the four listed Markdown records. No additional package source or dependent API changed in this phase.

`md get` read back every requested plan/log frontmatter key successfully, including both review items and the replacement handoff message. `git diff --check` passed. The spec has no requested new metadata and was left unchanged. No background commands or agents remain pending. Phase 1 is implementation complete, ready for review; the human-review handoff records future credential and breaking-scope decisions.

## Phase 2

### Starting state and orchestration

Phase 1's documents and the definition-API repairs are committed (`c135102a4` and earlier); the working tree's only pre-existing modifications are outside this feature (`prompts/_implement/implement-plan.md` and the `2026-10-16-schematic-response-model-graphql` plan/log), and they are excluded from this phase's metadata. Wave 2 runs three subagents with disjoint file ownership (metadata, OAuth2, composition). Wave 3 runs as two sequential single-owner steps (3a: Basic reshape, then 3b: the remaining vocabulary integration) because both edit `types.rs` and `builders.rs`. The phase gates run once, after all of them.

Shared helper written before Wave 2: `define/src/serde_presence.rs::non_null`, used with `#[serde(default, deserialize_with = "non_null")]` so an omitted optional vocabulary field is `None` and an explicit `null` is rejected (input-matrix rule "absent → N/I, explicit null → E"). New optional vocabulary fields are skipped on serialization when absent/false/empty, so the historical bare-definition snapshots keep their bytes for every API that adopts nothing.

### Design choices this phase makes (adopted defaults and choices the spec leaves open)

- adopted default: PD1 metadata shapes (`RateLimit`, `ResetUnit`, `RetryGuidance`, `StatusSelector`, `Backoff`) in a dedicated `metadata` module; descriptive only (source: PD1). Types are `#[non_exhaustive]` with public readable fields and validating constructors, so other crates cannot build an unvalidated value by literal; deserialization runs the same validation.
- adopted default: PD9 per-grant `OAuth2Config` internally tagged by `grant_type` with the existing variant spellings. The authorization-code PKCE type is the existing `PkceRequirement` with `NotUsed` removed (Required/Supported only); it appears only on the authorization-code grant (source: PD9, R8).
- adopted default: PD5 same-context composition through `DefinitionContext` methods; `TypeDesc` is extended (not replaced) with an optional structure (source: PD5, R6).
- adopted default: PD6 `AuthStrategy::Basic { username_env, password_env }`; `env_username` removed from `RestApi` (source: PD6, R5).
- adopted default: PD4 with the spec's recommended credential-source answer, as Phase 1 recorded: `.public()`, `.auth(policy)`, plus a policy-with-endpoint-`EnvMapping` form (source: Open Questions recommendation).
- adopted default: PD2 typed paths reuse `ParamDef`; PD3 `deprecated: bool` on `Endpoint` and `ParamDef`, serde default false.

### Wave 2 results

**Define metadata (R4/PD1)** — `schematic/define/src/metadata.rs` (new); `fn valid_header_name` in `builders.rs` became `pub(crate)` for reuse. `RateLimit`/`RetryGuidance` are `#[non_exhaustive]` with readable public fields, validating constructors (`RateLimit::new(limit, remaining, reset, ResetUnit)` + `with_retry_after_header`, `RetryGuidance::new(retry_on, honor_retry_after, max_attempts, backoff)`), `validate()`, and `MetadataError` (thiserror, names field and value). Serde goes through private raw mirrors with `deny_unknown_fields` + `try_from` (validation on every deserialize); enums are externally tagged snake_case (`{"code":429}`, `{"range":{"start":500,"end":599}}`, `"none"`, `{"fixed":{"delay_ms":500}}`, `{"exponential":{"base_ms":250,"cap_ms":8000}}`, `"epoch_seconds"`). Only `retry_after_header` may be absent (→ None); null is an error. Bare `StatusSelector`/`Backoff` deserialization does not validate; validation happens through `RetryGuidance`. No runtime reader. Tests: 13 `metadata::tests` (status 99/100/599/600, reversed/equal ranges, attempts 0/1/255, fixed 0/1, exponential base 0 and cap below/equal/above base, per-field header-name rejection, full-value preservation with byte-stable re-serialization, pinned JSON, per-field serde matrices for both structs including one/all wrong `retry_on` elements, `[]` valid and preserved, unknown members), plus 2 doctests. Boundary: in-process serde/validation, L1/F1.

**Reshape OAuth2 (R8/PD9)** — `OAuth2Config` is a `#[non_exhaustive]` enum internally tagged by `grant_type` (`AuthorizationCodePkce(AuthorizationCodeGrant)`, `ClientCredentials(ClientCredentialsGrant)`, `DeviceCode(DeviceCodeGrant)`), with grant-aware accessors (`grant_type()`, `token_url()`, `revocation_url()`, `default_scopes()`, `client_auth()`, `authorization_url()`, `device_authorization_url()`, `pkce()`) and `validate() -> Result<(), OAuth2ConfigError>` (absolute http/https URL with host). `PkceRequirement` dropped `NotUsed` and only appears on the authorization-code grant (adopted default: reuse the existing name as the authorization-code-specific PKCE type, rather than add a second type). Deserialization is a hand-written impl over a strict raw mirror: inapplicable fields accept absent or null and reject any value (including `""`); `revocation_url` absent → None, null/empty → error; deserialization runs `validate()`. Serialization emits only the selected grant's fields. Three `compile_fail` doctests pin the unrepresentable combinations. Consumers migrated: define `auth.rs` (docs/fixture), `openapi/import/mappings/mod.rs`, `openapi/export/security.rs` (the `unwrap_or_default()` on the authorization URL is gone because the field is required), lib/prelude re-exports; gen `definition_gen.rs` (importer emitter), `codegen/module_docs.rs`, `codegen/api_struct/tests.rs`, `tests/l1/{fresh_output_compile,http_client}.rs`; oauth `manager/{mod,authorization_code,revocation}.rs`, `tests/wiremock_flows.rs`. Findings: the OAuth manager has no device-code flow (nothing reads `device_authorization_url`), and it sends a PKCE challenge for both Required and Supported, so `pkce().is_some()` preserves the old behavior exactly. The manager never reads `client_auth` (pre-existing). Test replaced: `begin_authorization_without_pkce_when_not_used` (its variant no longer exists) → `begin_authorization_sends_pkce_when_only_supported`. The OpenAPI importer builds grant payloads directly without `validate()` because OpenAPI permits relative `tokenUrl`; Wave 3b adds definition-build validation of OAuth configs (see below). Committed generated trees are unchanged (`artifact_drift` passed). Stale non-docs design snapshot `schematic/oauth/oauth-schematic-design.md` still shows the flat struct; it is not a `docs/` page and was left as historical.

**Bridge composition (R6/PD5)** — `DefinitionContext::{one_of, any_of, all_of}::<T>(variants) -> Result<TypeDesc, DefinitionError>` and `models() -> &ModelCatalog`. **Descriptor extension (review item 6):** `TypeDesc` gained a private `structure: Option<TypeRef>` with `structure()`; the reference stays `Named(T)` and `rust_path` is T's, so the Rust generator references the bound type. `TypeRegistration` gained `structure()`. `components::flatten` takes the composition schema, which replaces T's schemars schema in `components()` (the explicit structure is authoritative), e.g. `{"oneOf":[{"$ref":"#/components/schemas/Cat"},{"$ref":"#/components/schemas/Dog"}]}`. New errors: `EmptyComposition`, `NonNominalMember`, `ForeignMember`, `DuplicateMember`; conflicts reuse `ComponentConflict` (T registered without the structure first, or with a different structure, including reordered members). Foreign-context detection compares the full descriptor (name, Rust path, structure) against this context's entry, because `TypeDesc` carries no context identity; a foreign descriptor identical in all three is indistinguishable (design choice, logged for review). Container/primitive members are rejected. A nested schemars copy of T that matches T's own schema is accepted in either registration order; a different nested schema under T's name conflicts. The catalog stores each composition as `ModelDef::Alias`. Serde derives were added to `ModelCatalog`, `ModelDef`, `StructDef`, `FieldDef`, `EnumDef`, `EnumVariant`, `TypeAlias` (`deny_unknown_fields`, optional fields absent → None / null → error). Tests: `schematic/define/tests/composition.rs` (15 L1 tests: oneOf/anyOf/allOf with distinguishable serde fixtures and round trips, a value matching neither variant fails, repeat/plain-after-compose reuse, every rejection, nested-copy agreement and conflict, catalog order plus byte-stable round trip, catalog serde presence rules, and a `RestApi::builder` build carrying the composed component) plus a `one_of` doctest. A comment in the fixture states that an untagged enum does not prove oneOf exclusivity; the author owns that.

Scoped checks reported by the three owners all passed (metadata 13 + 2 doctests; OAuth 208 define + 30 oauth + 78/33 gen selected, including `artifact_drift`, `import_overlay`; composition 71 + doctests; clippy clean on define/gen/oauth). Transient cross-agent compile errors cleared without edits outside each owner's files.

### Wave 3a: Basic carries its environment lists (R5/PD6)

- `AuthStrategy::Basic { username_env: EnvList, password_env: EnvList }`; serialized `{"Basic":{"username_env":{"names":[..]},"password_env":{"names":[..]}}}`. Both fields required; `[]` is valid and disables that half's environment source; absent/null/wrong type/one or all wrong elements/the old unit tag `"Basic"` are errors. `EnvList` does not reject an empty name string today, so neither does Basic (pre-existing `EnvList` contract, noted for Phase 4's loader matrix). `AuthMethod::Basic`/`EnvAuthStrategy::Basic` remain unit, source-neutral variants.
- `RestApi::env_username` and `AuthExtension::env_username` are removed (the exporter never emitted the latter, so no bytes changed). `RestApi::legacy_env_mapping_for(auth, env_auth)` lost its username parameter; for Basic it returns the full ordered lists, `None` for an empty list, and never reads `env_auth`. No code reads `env_auth.first()`/`[0]` as a password (residual `rg` hit `definitions/src/huggingface/mod.rs:313` is a Bearer test assertion).
- Builders keep their signatures. `basic_env(u, ps)` stores the lists with empty `env_auth`. `.authentication(Basic{..}, env)` with nonempty `env` fails at build. `env_username(name)` is resolved at build: with Basic it fills an empty username list or must equal `[name]`; with other authentication it is accepted only when the explicit `env_mapping.basic_user` is exactly `[name]`, otherwise a build error. **Design choice for review:** UnfoldedCircleCoreRest (Bearer) calls `.env_username("UCR_CORE_USER")` with a matching explicit `basic_user`; this rule keeps that authoring line unchanged rather than editing a definition outside R12. The explicit-mapping conflict check now compares against the full Basic lists.
- **Definition-authoring changes (R10/R12 mechanical Basic migration, review item 3):** `definitions/src/bitbucket/mod.rs`: `.authentication(AuthStrategy::Basic, vec!["BITBUCKET_APP_PASSWORD".to_string()])` → `.basic_env("BITBUCKET_USERNAME", ["BITBUCKET_APP_PASSWORD"])`, deleted `.env_username("BITBUCKET_USERNAME")`, `AuthStrategy` import moved to the tests module; `definitions/src/emqx/mod.rs`: `.authentication(AuthStrategy::Basic, vec!["EMQX_API_SECRET".to_string()])` + `.env_username("EMQX_API_KEY".to_string())` → `.basic_env("EMQX_API_KEY", ["EMQX_API_SECRET"])`. Both explicit `env_mapping`s are unchanged. Mechanical test-assertion repairs: Bitbucket (two old-shape assertions → exact `Basic{..}` value + `env_auth.is_empty()`, one `matches!` → `Basic { .. }`), EMQX (same swap; deleted `assert!(api.env_username.is_none())` in the bearer test), Eversolo (deleted `assert!(api.env_username.is_none())`).
- Generated output is byte-identical: `artifact_drift::generated_{rust,openapi,postman}_artifacts_are_up_to_date`, `postman_golden::golden_mixed_auth_emqx`, and the Bitbucket/EMQX/Unfolded baseline tests pass.
- **Historical comparison (R11):** snapshot files and `definition_snapshot.rs` are unchanged; `definition_baseline.rs` normalizes only `definition/{auth, env_auth, env_username}` and recomputes `fingerprints/api_metadata` only when a rule applied: (1) a null `env_username` is removed for every API; (2) for `bitbucket` and `emqx-basic` only, `"auth":"Basic"` + string `env_username` + at most one `env_auth` name is lowered to the new shape (password = that first name, the old legacy mapping's selection); (3) for `unfolded-circle-core-rest` only, its populated `env_username` is removed when auth is not Basic and the snapshot's `env_mapping.basic_user` is exactly that name. `EXPLAINED` stays empty. Controls: `changed_description_is_detected_at_its_exact_field` (runs through normalization), `historical_normalization_lowers_basic_and_unused_usernames_only`, `populated_username_outside_the_named_equivalences_is_reported`, `basic_lowering_reports_new_password_names_and_unlowerable_history`, `normalization_leaves_other_auth_leaves_reported`.
- Importer/AsyncAPI: an imported Basic API becomes `Basic` with two empty lists (equivalent to the old `None`/`None` mapping); `definition_gen` emits the new shape. WebSocket Basic mapping now comes from the embedded lists (it previously put `env_auth` into `basic_user`; no shipped WebSocket API uses Basic).
- **Finding:** `schematic/gen/src/codegen/api_struct/{constructors,fields,variant}.rs`, `schematic/gen/src/codegen/client/*`, and `schematic/gen/src/codegen/request_enum.rs` are not declared as modules anywhere, so no build compiles them. They were edited mechanically so no stale `env_username` remains. Deleting dead files is outside this spec's authorizations; recorded for review (accepted-gap candidate "orphaned types").
- Docs/skill drift fixed in the same change: `schematic/docs/definition-builders.md`, `schematic/docs/headers/README.md`, `schematic/docs/definition-snapshots.md`, `.claude/skills/schematic-define/SKILL.md`.
- Scoped checks: five-package `cargo build --all-features --all-targets` clean; define filter 437 passed; gen filter 267 passed; definitions filter 209 passed; doctests `auth`/`types`/`builders` pass; clippy `-D warnings` clean; `just lint` clean. No package outside `schematic/` uses `env_username` or `AuthStrategy::Basic`.

### Wave 3b: vocabulary integrated into the builders (R1–R4 define side, R14 validation)

**Public surface.** `RestApi.{rate_limit, retry_guidance}`; `Endpoint.{rate_limit, retry_guidance, deprecated, auth}`; `EndpointParams.path: Vec<ParamDef>`; `ParamDef.deprecated`. Optional fields use the `non_null` pattern (absent → None, null → error) and every new field is skipped on serialization when absent/false/empty, so unadopted definitions serialize exactly as before. `EndpointAuth { Public, Policy { policy: AuthPolicy, env_mapping: Option<EnvMapping> } }` (`#[non_exhaustive]`, `deny_unknown_fields`; `"Public"` / `{"Policy":{...}}`) is the PD4 shape with the adopted optional endpoint mapping. Helpers for the next phase: `RestApi::effective_rate_limit`, `effective_retry_guidance` (endpoint value replaces the API value whole; absence inherits), `RestApi::endpoint_auth(&Endpoint) -> ResolvedEndpointAuth { Inherited, Public, Replaced }` (a replacement uses the endpoint mapping if present, otherwise `default_env_mapping()`), and `Endpoint::path_placeholders()` (`{+name}` → `name`, deduplicated, first-occurrence order; reuses `pagination::path_params`). Builders (all prior signatures preserved): API `.rate_limit`, `.retry_guidance`; endpoint `.rate_limit`, `.retry_guidance`, `.deprecated()`, `.public()`, `.auth(policy)`, `.auth_with_env_mapping(policy, mapping)`, `.path_param(ParamDef)` (forces required, `Simple`, `explode = false`); `ParamDef::path(name, type)` and `ParamDef::deprecated(self)`. Metadata types, `EndpointAuth`, and `ResolvedEndpointAuth` are re-exported from the crate root and prelude.

**Validation** (`DefinitionError::InvalidDefinition`, naming API/endpoint/parameter; endpoint `.build()` runs endpoint-only rules and `RestApiBuilder::build` reruns all of them, so values mutated after construction are caught):
- metadata `validate()` at API and endpoint level;
- typed paths, only when declared: exact set equality with placeholders (missing/extra), duplicates, `required: false` (rejected, never normalized), non-`Simple` style, `Array`/`Json`, empty enum, duplicate enum values, invalid `rust_name`, and Rust-name collision with the endpoint's other params;
- endpoint auth: declared at most once, empty policy rejected ("use public()"), OAuth2 `validate()` for endpoint policy and (new) API `auth`/`auth_policy`, credential-source compatibility of the policy's `env_fallback` against the effective mapping (Bearer → `bearer_token`; ApiKey → `api_key` with matching header, case-insensitive; Basic → `basic_user` and `basic_pass`; `ApiKeyParam` only with no endpoint mapping and an identical API fallback with nonempty `env_auth`);
- R14: `PaginationError::PagerManagedRequiredWithoutDefault { strategy, param }` now names the strategy kind, and the new `PagerManagedDefaultInvalid` (rule `pager-managed-default-invalid`) rejects a sent default that is not type-valid for a **required** adopted parameter (`Integer` i64, finite `Number`, `true`/`false`, enum member, otherwise nonempty). **Design choice for review:** optional adopted parameters' defaults are not type-checked (R14 scopes the rule to required parameters). All shipped definitions still build; none was affected.

**Sanitized-name collisions — what is checked where.** Define's identifier sanitizer lives behind the `openapi` feature (`openapi::import::naming`), so builder validation uses a simpler normalization: explicit `rust_name`, else the wire name with non-`[A-Za-z0-9_]` characters replaced by `_`, compared across path/query/header/cookie params. Snake-case-only collisions (`userId` vs `user_id`) are left to the generator's existing complete `generated-name-collision` check, which Phase 3 must extend to read declared path `rust_name`s.

**Importer and OpenAI.** `definition_gen.rs` emits `deprecated: false` for every `ParamDef` (the importer does not map OpenAPI `deprecated` in this phase; importing it would change shipped OpenAI definitions and is not in scope). **Definition-source change (review item 3):** `definitions/src/openai/mod.rs` is importer-generated; its only change is 287 inserted `deprecated: false,` lines (287 insertions, 0 deletions), proven byte-identical to the importer's output by `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition`. This is a regeneration of a generated file, not an authoring change, but it is listed here because R10's diff check counts definition sources.

**Mechanical repairs** (literals gained `deprecated: false`, `path: vec![]`, or the new fields): define `src/openapi/import/{builder.rs, mappings/mod.rs, mappings/parameters.rs}`, `src/openapi/export/{info.rs, mod.rs, parameters.rs}`, `src/request.rs`, `tests/{definition_serde.rs, endpoint_literal_surface.rs}`; gen `src/validation.rs`, `src/codegen/{api_struct,endpoint_methods,pagination,request_structs}/tests.rs`, `tests/l1/{fresh_output_compile,output_validation,query_params_codegen}.rs`; `define/tests/pagination_tables.rs` (two assertions moved to the struct-shaped error variant). No package outside `schematic/` constructs these literals.

**Docs/skill drift fixed in the same change:** `schematic/docs/definition-builders.md` (vocabulary section; generator/exporter/runtime consumers marked **planned**), `schematic/docs/pagination.md`, `schematic/docs/generation/validation.md` (new rule id), `.claude/skills/schematic-define/SKILL.md`.

**Tests** — `schematic/define/tests/vocabulary.rs` (new autotest binary, 18 L1 tests, in-process through the public builders and serde; F1): typed path acceptance/forcing, reserved and repeated placeholders with first-occurrence order, legacy untyped paths, 11 malformed-declaration cases through endpoint build and API build after mutation, deserialized `required: false` rejected, `params.path` serde shapes, deprecation flags and serde (absent false, null/string error, false not serialized), metadata replacement/inheritance, mutated-invalid metadata, metadata serde for each of the four fields, endpoint auth storage/resolution, incompatible/missing sources per method, malformed endpoint auth, endpoint auth serde, R14 missing and type-invalid defaults, page-number bases, and byte-identical re-serialization with every field populated. Plus a `path_placeholders` doctest.

**Scoped checks (owner):** five-package build clean; full `schematic-define` nextest 725 passed; gen filter (baseline, drift, import_overlay, definition_gen, query params, request structs, validation, pagination) 182 passed; `schematic-definitions` 567 passed; define doctests 165 + 13 passed; clippy `-D warnings` clean; `just lint` clean. `definition_baseline` and `artifact_drift` pass with no new normalization or `EXPLAINED` entries.

**Observations for later phases.** `EnvMapping` serializes unset credentials as explicit `null` (e.g. `"basic_user":null` inside an endpoint mapping), and `ParamDef.description` serializes `null`; Phase 4's strict bundle writer/loader must account for these pre-existing shapes (contracts.md already flags raw snapshot nulls).

### Phase gates, regression repair, and checkpoint

- `cd schematic && just test` (run once, after every subagent finished, with no subagent running): ordinary L1 **2,360 passed**, 13 skipped (tier exclusions). The fresh-output pass then failed one test: `fresh_output_compile::slow_generated_types_reject_misuse_at_compile_time`. Its 4a misuse case "a descriptor carries no structure" asserted that `TypeDesc` has no `structure` method (E0599). R6 deliberately adds the read-only `TypeDesc::structure()` accessor (the plan's permitted descriptor extension), so the case was pinning the old surface, not a regression. **Test replaced (review item 9):** the case now states the contract that still holds, "a descriptor's structure is set only by context composition", by assigning the private field and expecting E0616 naming `structure`. The `just test-fresh-output` rerun passed 7/7, and `just test-compile-fail` (skipped behind the failure) passed 1/1. No ordinary L1 test exceeded 5 s in this run; the four over-budget tests Phase 1 recorded did not reappear.
- `cd schematic && just lint`: PASS (all five packages). `just check-tier-coverage schematic`: PASS, 0 stranded. New test binaries `define/tests/{composition,vocabulary}.rs` are cargo autotests of `schematic-define` (L1); no new test carries a tier marker.
- Baseline checkpoint (re-run of the R11 comparison): `definition_baseline::*` (18 APIs plus controls) and `artifact_drift::generated_{rust,openapi,postman}_artifacts_are_up_to_date` pass within the full gate. **Generated-output differences: none.** Every generated Rust/OpenAPI/Postman byte is unchanged. The only historical-snapshot handling is Wave 3a's narrow normalization; `EXPLAINED` is empty.
- Dependents: the only direct reverse dependency outside `schematic/` is `model-citizen` (it uses `schematic_definitions::{ollama,lmstudio}` model types, which are unchanged). `cargo build -p model-citizen --all-targets` succeeded and `cargo nextest run -p model-citizen -E 'test(ollama) | test(lmstudio)'` passed 48/48. Other inventoried consumers (sniff, biscuit-speaks, homelab crates) depend on the generated `schematic-schema`, which is byte-identical, so they were not re-run.
- Scope calculator (all edits saved): `change_class: package`, `packages: [schematic-define, schematic-definitions, schematic-gen, schematic-oauth, test-toolkit, darkmatter]`; reverse dependency `model-citizen`. Gates were therefore required and were run. Darkmatter is selected because its tests read the `.claude/skills` tree; the tests that do so passed (`shipped_schema_corpus::shipped_markdown_{inline_schemas_parse_passively,named_type_references_resolve}`, both `current_root_migration_guard` tests: 4/4). The test-toolkit selection is a suite-input/lint-companion row (`archive-path-guard`), not a dependency of the changed code. It was not run locally; CI covers it.
- Cross-OS: this phase changes data types, serde contracts, and definition validation only. No path handling, process, terminal, or `#[cfg]` code changed, so no cross-OS run was made (no notable OS risk). Host exercised: macOS only.
- Docs drift: added the composition section to `schematic/docs/typed-registration.md` (Postman diagnostic marked **planned**) and updated `.claude/skills/schematic-define/SKILL.md` (composition pointer; removed the stale instruction to reserve the now-implemented `path_param/public/auth/deprecated/rate_limit` names). `define/tests/authoring_guidance.rs` passes.

**Requirement-to-test mapping for this phase.**

| Behavior | Tests | Boundary / rung |
| --- | --- | --- |
| R4 metadata bounds, preservation, serde matrix | `metadata::tests::*` (13), metadata doctests; `vocabulary::{endpoint_metadata_replaces_the_api_value_whole_and_absence_inherits, metadata_mutated_invalid_after_construction_is_rejected_by_build, metadata_serde_for_each_api_and_endpoint_field}` | in-process builder/serde, L1/F1 |
| R8 per-grant OAuth2, PKCE, serde matrix, unrepresentable combinations | `oauth::tests::*` (12), 3 `compile_fail` doctests (compiler, F2), `schematic-oauth` suite incl. `wiremock_flows.rs` (mock HTTP, F2), `begin_authorization_sends_pkce_when_only_supported` | L1 |
| R6 composition, rejection, catalog serde | `composition::*` (15), `one_of` doctest, fresh-output misuse case (compiler, F2) | L1 |
| R5 Basic lists, precedence, fallback order, serde | define `types`/`auth` tests, `builders` binary additions, `definition_baseline` normalization controls (4 new), `artifact_drift` byte equality, `postman_golden::golden_mixed_auth_emqx` | L1 F1 + F2 writers |
| R1 typed-path validation, R3 deprecation, R2 endpoint auth shape/sources, R14 strategy default | `vocabulary::*` (18), `path_placeholders` doctest, `pagination_tables` updates | L1/F1 |
| R10 builder signatures preserved | full `schematic-definitions` (567) and gate; definition-source diff limited to Bitbucket/EMQX Basic lines, three mechanical assertion edits, and importer-regenerated OpenAI `deprecated: false` lines | L1 |

Input-matrix coverage for the new deserializers is walked per validator in the tests above (absent, null, wrong type, one or all wrong elements, empty, unknown member). Document-shape rows (duplicate keys, trailing content) belong to Phase 4's single JSON loader; Basic's tests additionally cover duplicate keys and trailing content through `serde_json`.

**Code-smell audit (new load-bearing parse code):** no `#[serde(default)]` on a required field; every new optional configuration field uses `non_null`; new booleans default false only where the matrix says absent → F. The `unwrap_or_default()` that remained in `openapi/export/security.rs` for the authorization URL was removed with the OAuth2 change. No `filter_map(.. as_str())` or swallowed `.ok()` was added.

**Checkpoint met:** new builder forms, validation failures, metadata bounds, composition semantics, Basic precedence/fallbacks, and OAuth2 payload contracts have focused L1 evidence. Definition, generator, OAuth2, and shared WebSocket consumers compile. Affected L1, lint, and the baseline comparison pass with no output differences. No shipped authoring rewrite beyond the logged Basic lines. Last completed phase: 2.

**Unfinished or deferred (not Phase 2 scope):** generator/exporter/runtime consumers of every new field (Phase 3), including OpenAPI `deprecated`, endpoint security, typed path request fields, the generator's collision check reading declared path `rust_name`s, and the Postman composition diagnostic; the importer mapping OpenAPI `deprecated` (not authorized; it would change the shipped OpenAI definition); GitHub's R12 rate-limit adoption (Phase 3, Wave 5).

## Phase 3

### Starting state and orchestration

Phase 2 is committed (`13c168975`, plus planning records); the working tree was clean at entry. Wave 4 ran as three foreground-briefed subagents with disjoint file ownership, plus one task run directly by the orchestrator:

- **Resolve authentication**: `schematic/runtime/src/**`, runtime tests, `gen/src/codegen/api_struct/{auth,helpers,mod,tests}.rs`, `gen/tests/l1/endpoint_auth_output.rs`.
- **Connect exports**: `define/src/openapi/export/*`, `define/src/openapi/extensions.rs`, `gen/src/export/**`, `gen/tests/l1/vocabulary_exports.rs`.
- **Connect Rust output**: `gen/src/codegen/request_structs/**`, `endpoint_methods/**`, `module_docs.rs`, `parser.rs`, `gen/src/validation.rs` (Rust checks), `gen/tests/l1/typed_path_output.rs`.
- **Migrate OAuth consumers** (orchestrator): `schematic/oauth/**`.

Shared files were prepared before the fan-out, so no two owners edit one file: four new `Feature` ids in `gen/src/diagnostics.rs` (`endpoint-auth`, `schema-composition`, `oauth2-grant`, `path-parameter`); a stub `gen/src/export/vocabulary.rs` (`diagnostics(api, output)`) already called from `validate_openapi`/`validate_postman` in `gen/src/validation.rs`; and three new L1 modules declared in `gen/tests/l1/main.rs` (`endpoint_auth_output`, `typed_path_output`, `vocabulary_exports`). Docs are left to the orchestrator after integration.

Adopted ruling for replacement credentials (plan rule 2; source: spec Open Questions recommendation, recorded as adopted default in Phases 1–2), as briefed to the runtime owner: explicit credential precedence is preserved (per-call, then client-level, then environment); a credential is used for a replacement endpoint only when the replacement accepts its kind, and is never reinterpreted for another placement; a per-call credential of an unaccepted kind fails before send; a replacement endpoint that resolves no credential fails before send ("missing source"), while inherited endpoints keep the runtime's documented anonymous fallback.

### Migrate OAuth consumers (R8)

Phase 2 had already replaced every flat-field read in `schematic-oauth` with the per-grant accessors. This task made the remaining operations grant-aware and gave `client_auth` a reader:

- `OAuth2Manager::require_grant` (new, `manager/mod.rs`): `begin_authorization` and `exchange_code` require `AuthorizationCodePkce`; `acquire_client_credentials_token` requires `ClientCredentials`. Another grant returns `OAuthError::Configuration` naming both grants, before any request. Previously the client-credentials flow ran against any provider, and the authorization-code flow failed only incidentally (no authorization URL).
- **Design choice for review:** `client_auth` is now honored. `ClientSecretBasic` keeps the prior wire behavior exactly (`oauth2`'s `BasicAuth`, the old implicit default); `ClientSecretPost` sends id and secret in the body; `None` (public client) sends only the client id and never a configured secret. Before this change the field had no reader (D49 dead-surface criterion), so a `ClientSecretPost` provider silently got Basic. R8's "preserve client-authentication behavior" is met for the only behavior that existed (Basic).
- Default scopes: behavior preserved. The manager requests the runtime `scopes`, never the provider's `default_scopes` (it never did), and stores the requested scopes when a token response names none. The provider default scopes are read by the OpenAPI exporter.
- Device code: `schematic-oauth` has no device-code flow (pre-existing, Phase 2 finding); both flows now refuse a device-code provider explicitly. Recorded as an accepted-gap candidate; the grant itself is proven through definition validation and exporters.
- Revocation happy path remains untestable over plain-HTTP wiremock (`oauth2` requires HTTPS revocation URLs; existing note kept); revocation error paths stay covered by `manager::tests`.
- Serialized fixtures: `schematic-oauth` serializes only `StoredTokens`, which this spec does not change; no OAuth2 config fixture exists in the crate.
- Tests added to `schematic/oauth/tests/wiremock_flows.rs` (autotest, L1, F2 HTTP receiver): `client_secret_basic_sends_credentials_in_the_authorization_header` (exact header value, no secret in body, runtime scope requested), `client_secret_post_sends_credentials_in_the_body`, `a_public_client_sends_its_id_and_never_the_configured_secret`, `each_flow_refuses_a_provider_of_another_grant_before_sending` (all three grants; server receives nothing), `both_pkce_modes_send_the_verifier_with_the_code` (Required and Supported). Unit test renamed: `begin_authorization_fails_without_auth_url` → `begin_authorization_fails_for_another_grant` (its failure reason changed). Doc drift fixed: the manager's struct doc, the removed PKCE comment that described the incidental failure, and the test module doc.
- `cargo nextest run -p schematic-oauth`: 35/35 passed. `cargo clippy -p schematic-oauth --all-targets -- -D warnings`: clean.

### Resolve authentication (R2)

- **Runtime** (`schematic/runtime/src/{client,credentials,options,call}.rs`): new `ApiDefinition::endpoint_public(id)` and `ApiDefinition::endpoint_auth(id, AuthConfig)`, following the existing `endpoint_cross_origin` precedent. Per-endpoint state is built at validation and selected in `prepare` by `R::ENDPOINT_ID`. New `AuthConfig::resolve_required` and `accepted_kinds`.
    - Public endpoint: resolves anonymously (no environment read, no client, `api_key`, `basic_auth`, or per-call credential; a per-call credential is dropped, as R2 asks). It strips the union of every placement known to the API and to every replacement policy, including raw values the request itself carries there. Ordinary headers and cookies stay. No state carries over to the next call.
    - Replacement: adopted ruling as recorded above. **Design choice for review:** `.anonymous()` on a replacement endpoint is honored (sends anonymously, strips that policy's locations). `env_order` applies only to the API fallback. `.credentials(c)` builds when the API or any replacement accepts its kind; `.api_key(k)` takes the API key placement, else the first replacement's. Secret names include replacement placements, so a default header in such a location is refused.
- **ABI not bumped** (stays 4): both methods are additive, so older generated code runs unchanged on the new runtime. `SCHEMATIC_RUNTIME_ABI` is embedded in the committed `schematic/schema/src/lib.rs`, so a bump would change shipped bytes. Only definitions declaring endpoint auth emit the new calls; built against an older runtime, those would fail with "no method" instead of the readable ABI message. That cannot occur in this repository, which depends on the runtime by path. Recorded for review.
- **Generator** (`gen/src/codegen/api_struct/{auth,helpers,mod}.rs`): `policy_config(api, policy, mapping)` translates any policy; `auth_config` wraps it for the API. `definition()` emits `.endpoint_public`/`.endpoint_auth` only for endpoints declaring auth, so no shipped output changed. `basic_auth()` is emitted when the API or any replacement accepts Basic. Query/cookie key replacements read `api.env_auth`, as the API-level path does. No runtime-unplaceable method exists today (OAuth2 maps to bearer); a future untranslatable `#[non_exhaustive]` variant yields `AuthConfig::none()`, which fails before send rather than relocating a credential.
- **Tests:**
    - `schematic/runtime/tests/l1/endpoint_auth.rs` (11, L1, wiremock F2): `a_public_endpoint_sends_no_credential_from_any_source_and_strips_every_known_auth_location`, `an_authenticated_call_after_a_public_call_still_sends_its_credential`, `a_replacement_reads_its_own_environment_source_instead_of_the_apis`, `query_and_cookie_replacements_place_the_endpoint_credential_where_the_endpoint_expects_it`, `a_per_call_credential_the_replacement_does_not_accept_is_refused_before_any_request`, `a_replacement_with_nothing_resolving_is_refused_while_an_inherited_endpoint_goes_anonymous`, `a_client_bearer_token_is_never_reinterpreted_as_a_replacements_basic_credential`, `a_client_credential_only_a_replacement_accepts_builds_and_is_sent_only_there`, `an_api_key_takes_a_replacements_placement_when_the_api_has_none`, `replacement_locations_are_secret_in_stored_metadata_and_refused_as_default_headers`, `a_replacement_credential_follows_only_the_redirects_the_origin_policy_allows_credentials_on`. Environment via `test_toolkit::EnvGuard`, one unique variable per test.
    - `api_struct::tests` (4): an API without endpoint auth emits exactly its previous `definition()`; public; replacement with endpoint and API mappings plus `basic_auth`; parameter-key replacement.
    - `endpoint_auth_output::slow_generated_clients_honor_public_and_replacement_endpoint_auth` (L1 `slow_`, F2: a fresh crate generated from real builders, compiled offline, with 5 child wiremock tests in one Cargo build).
    - Owner checks: runtime 395/395; `api_struct` 17/17; `artifact_drift` and `endpoint_auth_output` 9/9; `slow_fixture_apis_compile_and_send_offline` passed.

### Connect exports (R1–R4, R6, R8 export side)

- **OpenAPI** (`define/src/openapi/export/{paths,security,parameters,conversion,components,mod}.rs`, `extensions.rs`):
    - Typed path parameters carry their type, enum values, description, `required: true`, and `deprecated`.
    - **Adopted design:** endpoints are grouped by normalized path. The shared path-item declaration comes from the first operation in get, put, post, delete, options, head, patch order, independent of visit order. Each operation that differs gets an operation-level override. Untyped paths are unchanged.
    - `deprecated: true` is emitted on operations and parameters (the hard-coded `None` is gone).
    - Endpoint auth: public gives `security: []`; a replacement lists each accepted method as a separate alternative.
    - **Design choice:** override-only schemes reuse the name of an identical registered scheme, else take the base name, else `base2`, `base3`, …; never an overwrite. `map_security_requirements` now names only schemes that exist, so a device-code API no longer references a missing scheme.
    - Composed components export as `oneOf`/`anyOf`/`allOf` of `$ref`s. **Finding (fixed):** the conversion previously turned them silently into `{}`.
    - `rate_limit`/`retry_guidance` appear in the document `x-schematic` extension, and in the operation extension only for endpoint overrides. Absent values are skipped; `skip_extensions` is honored.
- **Group merge** (`gen/src/export/openapi.rs`): when neither side declares operation security, nothing changes (EMQX bytes kept). Otherwise the effective requirements are unioned, and a public side contributes `{}` so the anonymous alternative survives. Rename suffixes use the same numbered rule, which fixes a possible overwrite of an earlier rename. A later API's differing path declaration becomes an override. Conflicting same-name composed components are reported.
- **Postman** (`gen/src/export/postman/{item,request,collection}.rs`, `gen/src/export/auth.rs`):
    - A public endpoint gets request-level `noauth`. A replacement gets the auth of its first accepted method.
    - **Design choice:** variables keep the standard names, suffixed with the endpoint id (`bearerToken_Admin`, empty value) when the endpoint has its own mapping. No variable name ever becomes a value.
    - A URL variable's description is `"{description} ({type})"`, or `one of: a, b` for an enum.
    - The request description gains `Deprecated.`, `Deprecated parameters: …`, `Rate limit headers: …`, `Retry guidance: …` lines from the effective metadata.
- **D10 warnings** (`gen/src/export/vocabulary.rs`; they never block output):
    - `endpoint-auth`: Postman, a policy with more alternatives than one request auth can express.
    - `schema-composition`: Postman, per endpoint referencing a component that is a pure-reference composition. Existing examples are kept and none is invented.
    - `oauth2-grant`: OpenAPI, device-code grant; OpenAPI 3.0 has no device flow.
    - Compositions that schemars itself emits inside ordinary types are not treated as composed components; this keeps shipped bytes unchanged.
- **Finding for review (pre-existing defect, not fixed):** committed `schematic/openapi/emqx.json` declares path parameters twice on shared paths (e.g. `/nodes/{node}` lists `node` twice), which OpenAPI forbids. The group merge appends identical declarations. Fixing it changes a shipped artifact for a reason outside this spec, so the bytes are kept and a rustdoc note on `merge_path_parameters` records it.
- **Limitation:** an endpoint whose only accepted method is an OAuth2 device-code grant gets no operation `security` (it inherits the document's) plus the `oauth2-grant` warning.
- **Tests:**
    - `gen/tests/l1/vocabulary_exports.rs` (19 fixture tests, L1/F2 through the real writers): shared GET/DELETE common plus override; agreeing declarations; repeated/reserved; enum path; deprecation true/absent; public `security: []` + Postman noauth; override-only scheme; forced scheme collision; alternatives + Postman warning with the output still written; env names never exported; merge keeps a public alternative; numbered renames; merge override; conflicting composed component; composition refs; Postman composition warning with output written; metadata every field at API/endpoint level incl. `skip_extensions`; device-code warning with output written; unadopted control.
    - Define unit tests: `paths` (7), `security` (3), `conversion` (2), `parameters` (1), `extensions` (2), `export::tests` (2, including real `ctx.one_of/any_of/all_of`). Gen: `vocabulary::tests` (1), `auth::tests` (1), and an `endpoint_auth_override` doctest.
    - Owner checks: define `openapi` 247 passed; gen `export` 106; gen l1 export filters 74; shipped-artifact filters 22.

### Connect Rust output (R1, R3, R4 docs)

- **Typed paths** (`gen/src/codegen/request_structs/{path_params,into_parts,mod,derives,shared,…}.rs`): declared placeholders become `String`/`i64`/`f64`/`bool`/enum fields, one per distinct placeholder in first-occurrence order.
    - `new(..)` takes paths in template order, then the body. Strings take `impl Into<String>`; other values are taken by value.
    - `into_parts` walks every occurrence, so a repeated placeholder uses one field and `{+x}` stays raw. Values are converted to their wire spelling before encoding. A non-finite `f64` returns `InvalidArgument("path parameter `x` must be a finite number")` before the path is built.
    - A typed-path request has no `Default` and no `From<&str>/From<String>` (D57 item 13). Legacy requests take the old code paths unchanged.
- **Enum machinery** (`request_structs/param_enums.rs`, reused by Phase 5): `param_enum_name(request_type, field_name)`, `enum_variant_name(wire)`, `variant_collisions(values)`, `generate_param_enum(name, doc, values)`. The enum is `#[non_exhaustive]` with `Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize`, one `#[serde(rename)]` per variant, `as_str()`, and `Display`.
    - **Adopted design:** variant names are PascalCase with characters outside `[A-Za-z0-9]` dropped; an empty name becomes `Empty`, a leading digit gets a `V` prefix, a keyword gets a `_` suffix.
    - **Adopted design:** the `{ParamPascal}` part of the type name comes from the Rust field name, so `rust_name` participates (PD8 leaves this open).
- **Docs only** (`request_structs/endpoint_docs.rs`, `endpoint_methods/{mod,docs}.rs`, `module_docs.rs`): "Deprecated." lines on request, method, call-type, field, and setter docs; no `#[deprecated]` anywhere. `## Rate limit` and `## Retry guidance` list every field of the effective values on request docs and the API values in module docs. Public and replacement endpoints get auth notes.
    - Orchestrator follow-up: grouped modules (`gen/src/output/assemble/api.rs::build_combined_module_docs`) now also show each API's metadata. Test `output::assemble::api::tests::grouped_module_docs_keep_each_apis_rate_limit_and_retry_guidance`.
    - **Gap noted for review:** module docs list only the API's environment variables, not a replacement endpoint's own variables; the request and endpoint docs list those.
- **Validation** (`gen/src/validation.rs`):
    - New `check_typed_path_params` (`path-parameter` errors: undeclared or extra placeholder, Array/Json, empty enum, invalid `rust_name`).
    - Variant collisions after sanitizing are `generated-name-collision`.
    - Path field names, including `rust_name` and snake-case collisions such as `userId`/`user_id`, now enter the field-collision check (the Phase 2 hand-off). Path enum names enter the module-group type-name check (request structs, call types, forms, grouped APIs, referenced types).
- **Tests:**
    - `param_enums::tests` (4).
    - `request_structs::tests` (16), e.g. `constructor_follows_template_order_not_declaration_order`, `a_repeated_placeholder_is_one_field_used_at_every_occurrence`, `a_non_finite_number_path_value_is_an_invalid_argument_before_the_path_is_built`, `a_typed_path_request_has_no_default_or_string_conversion_and_legacy_keeps_both`, `deprecation_is_documented_on_requests_fields_and_setters_without_attributes`, `every_rate_limit_and_retry_guidance_field_is_documented_on_the_request`, `a_definition_adopting_nothing_new_generates_unchanged_docs`.
    - `endpoint_methods::tests` (3), `module_docs::tests` (2), `validation::tests` (6).
    - `typed_path_output::the_typed_path_fixture_passes_rust_validation`.
    - `typed_path_output::slow_typed_path_clients_send_wire_values_and_refuse_default` (F2, one fresh crate): wiremock checks of the exact paths for integer, boolean, number, enum, reserved, encoded, and repeated values; NaN, +inf, and -inf fail before send; a second `cargo check --tests` proves `Default::default()` on a typed-path request fails with E0277 and an exhaustive match on the enum fails with E0004, while an all-optional request keeps `Default`.
    - Owner checks: gen lib filters 153 passed; gen l1 filters 74 passed; the slow test passed (30 s); clippy clean.

### Wave 5: prove outputs

- **R12 adoption** (`schematic/definitions/src/github/mod.rs`, the only definition-authoring change this phase): `fn github_rate_limit()` declares `x-ratelimit-limit`, `x-ratelimit-remaining`, `x-ratelimit-reset` (`ResetUnit::EpochSeconds`), and `retry-after`, attached with `.rate_limit(github_rate_limit()?)`. No retry guidance, statuses, attempts, or backoff (plan rule 4). A metadata error maps to `DefinitionError::InvalidDefinition` with the builders' `<api>` endpoint convention. Test `github::tests::api_declares_only_the_documented_rate_limit_headers` covers every field, absent retry guidance, no endpoint overrides, and that every endpoint's effective value is the API's.
- **Regeneration** (`cargo run -p schematic-gen -- generate --api all --output schema/src --openapi-out openapi --postman-out postman`; `just generate` minus its desktop notification). Only four files changed, all because of the GitHub adoption:
    - `schematic/openapi/github.json`: `x-schematic.rate_limit` added to the document extension (+8/-1).
    - `schematic/postman/github.postman_collection.json`: each of the 16 request descriptions gains the rate-limit line (16 lines changed).
    - `schematic/schema/src/github/mod.rs`: module docs gain a `## Rate limit` section (+9).
    - `schematic/schema/src/github/requests.rs`: each of the 16 request docs gains the effective `## Rate limit` section (+144).

  Every other API's Rust, OpenAPI, and Postman bytes are unchanged, including Bitbucket and EMQX Basic output and environment docs (R5). `artifact_drift::*` passes against the regenerated trees. The R11 allow-list (`EXPLAINED`) stays empty because the committed artifacts were regenerated rather than excepted.
- **Output assertions:** `vocabulary_exports::github_rate_limit_adoption_reaches_every_output` reads the committed GitHub OpenAPI, Postman, and module/request docs through `include_str!` (equal to fresh output by `artifact_drift`). It asserts every field, no retry guidance, and 16/16 Postman requests and request docs.
- **R4 runtime boundary:** `schematic/runtime/tests/l1/metadata_boundary.rs::the_runtime_neither_reads_nor_acts_on_rate_limit_or_retry_metadata` scans every runtime source file for rate-limit/retry/sleep spellings and checks that the runtime does not depend on `schematic-define`. Review of `git diff runtime/src`: no retry, sleep, backoff, or enforcement added; "retry" appears only in a doc line stating there is none.
- **Fixture compile coverage across the phase:** integer/boolean/number/enum/repeated/reserved paths and the `Default` compile-fail (`typed_path_output`); public/replacement auth (`endpoint_auth_output`); differing GET/DELETE metadata (`vocabulary_exports`); duplicate declarations (Phase 2 `vocabulary` tests) and field/module collisions (`validation::tests`).
- **Docs** (drift fixed in the same change, by a Documenter subagent plus the orchestrator):
    - `schematic/docs/definition-builders.md` (three **planned** markers replaced; reserved-name claim removed), `typed-registration.md` (Postman composition warning no longer planned).
    - `runtime/{credentials,building-clients,calling-an-api,README}.md` (endpoint authentication section and Mermaid decision diagram; the "no missing-credential error" statement narrowed to inherited endpoints; OAuth manager grant and client-auth behavior).
    - `generation/{generated-clients,request-side,validation}.md` (typed paths, doc output, new rule ids).
    - `io/{export-openapi,openapi-extensions,export-postman}.md`.
    - `headers/README.md`.
    - Drift found by the doc pass: `export-openapi.md` claimed `auth_policy` decides document security schemes; the code uses `auth` only, and the page now says so.
    - Skill `.claude/skills/schematic-define/SKILL.md`: the "consumers are still planned" note is replaced with current behavior and the new diagnostics.
    - `schematic/oauth/src/lib.rs` crate docs gain one sentence on the grant and client-auth rules.
- **R11 historical comparison after the adoption.** The first full gate failed `definition_baseline::{github, changed_description_is_detected_at_its_exact_field}` at `github/definition/rate_limit` plus the recomputed `fingerprints/api_metadata`, as expected for a deliberate R12 adoption. Fix in `gen/tests/l1/definition_baseline.rs`:
    - **Named, value-exact adoption normalization**, not an `EXPLAINED` exception: `adoptions()` lists `("github", "rate_limit", <exact adopted JSON>)`. `normalize_historical` inserts that value only into GitHub's historical definition, only when absent, and recomputes the metadata fingerprint as the Basic lowering does. The original snapshot files are untouched and `EXPLAINED` stays empty.
    - New negative control `adoptions_accept_only_the_named_api_field_and_exact_value`: a different GitHub reset header is reported at `github/definition/rate_limit/reset_header`, and the same metadata on Gitea is reported at `gitea/definition/rate_limit`. The description-mutation control is still exact. `historical_normalization_lowers_basic_and_unused_usernames_only` also permits the named adoption leaf.
    - `shipped_json_field_matrix_preserves_required_and_optional_shapes` walks GitHub's serialized `RestApi`, which now carries `rate_limit`. Its matrix helper gained a `strict` list: `rate_limit` may be absent (→ none) but an explicit `null` is an error, which is the Phase 2 `non_null` contract and the plan's input matrix. Older optional fields keep their legacy null-as-absent rows. Rerun: `definition_baseline::*` 30/30.

### Phase gates and checkpoint

- `cd schematic && just test` (run once all subagents had finished, with none running; rerun after the baseline fix): ordinary L1 **2455/2455 passed** (15 skipped tier exclusions); fresh-output 9/9 (new `slow_` tests: `endpoint_auth_output` 33.6 s, `typed_path_output` within the run); compile-fail 1/1. No ordinary L1 test exceeded 5 s (the whole ordinary pass took 7.9 s wall). Log: `/tmp/schematic-vocabulary-phase3-test.log`.
- `just lint`: PASS for all five packages (`/tmp/schematic-vocabulary-phase3-lint.log`). `just check-tier-coverage schematic`: PASS, 0 stranded. All new test modules are declared (`gen/tests/l1/{endpoint_auth_output,typed_path_output,vocabulary_exports}.rs`, `runtime/tests/l1/{endpoint_auth,metadata_boundary}.rs`); `oauth/tests/wiremock_flows.rs` is an autotest. No new test name carries a tier marker except the three intentional `slow_` fresh-output tests (Cargo-invoking, run by `just test-fresh-output` inside `just test`).
- Extra checks because generated code and rustdoc changed: `just doctest` PASS; `just lint-schema` PASS.
- **Dependents** (serial, targeted): `model-citizen` (the calculator's reverse dependency) built with `--all-targets`, and `-E 'test(ollama) | test(lmstudio)'` passed 48/48. The `schematic-schema` consumers `sniff`, `biscuit-speaks`, `homelab`, `eversolo-integration`, `arcam-amp-integration`, `sony-receiver-integration`, `unfolded-integration-helper` pass `cargo check --all-targets`. Their tests that use the generated clients or runtime credentials passed: sniff `test(remote)` 75/75, biscuit-speaks `test(elevenlabs) | test(cache)` 94/94, homelab `test(samsung) | test(eversolo) | test(rest)` 24/24. Darkmatter's skill-reading tests (`shipped_markdown_*`, `current_root_migration_guard`) passed 4/4. `test-toolkit` is selected only as a suite-input/lint-companion row (`archive-path-guard`); it was not run locally, and CI covers it.
- Scope calculator (run from the root after every edit was saved): `change_class: package`, `packages: [schematic-define, schematic-definitions, schematic-gen, schematic-oauth, schematic-runtime, test-toolkit, darkmatter]`, `reverse_dependencies: [model-citizen]`. Gates were therefore required and were run.
- **Cross-OS:** no notable OS risk. No path, process, terminal, `#[cfg]`, or file-system code changed. The runtime changes are in-memory credential selection; the new tests use wiremock and the existing fresh-crate harness, whose compile-fail matching already handles both separators. `just cross-check` was not run. Host exercised: macOS only.
- **Generated-output differences** (R11, per API and file): only the four GitHub files listed under Wave 5, all caused by the R12 rate-limit adoption. Every other API is byte-identical, including Bitbucket and EMQX Basic.

**Requirement-to-test mapping for this phase.**

| Behavior | Tests | Boundary / rung |
| --- | --- | --- |
| R2 public / replacement runtime resolution, stripping, precedence, missing and incompatible sources, redirects, redaction | `runtime l1 endpoint_auth::*` (11) | in-process runtime against a wiremock HTTP receiver, L1/F2 |
| R2 generated `definition()` wiring, `basic_auth` | `api_struct::tests` (4), `endpoint_auth_output::slow_generated_clients_honor_public_and_replacement_endpoint_auth` | generator tokens F1; fresh crate compiled offline and sent to a mock, F2 |
| R2 OpenAPI `security: []`, override-only schemes, alternatives, merge; Postman noauth and variables | `vocabulary_exports::{public_endpoint_has_empty_security_and_postman_noauth, replacement_*, group_merge_*, environment_variable_names_never_become_exported_values}`, `security::tests` | real writers, L1/F2 |
| R1 typed paths: Rust fields, order, wire spelling, non-finite, no `Default`, enums, collisions | `request_structs::tests` (16), `param_enums::tests` (4), `validation::tests` (6), `typed_path_output::*` (2) | tokens F1; compiler and HTTP F2 |
| R1 OpenAPI/Postman typed paths, common plus override | `vocabulary_exports::{shared_get_delete_*, agreeing_*, repeated_and_reserved_*, enum_path_*}`, `paths::tests` (7) | writers F2 |
| R3 deprecation (docs, OpenAPI, Postman; no attribute) | `request_structs::tests::deprecation_*`, `endpoint_methods::tests::deprecation_*`, `vocabulary_exports::deprecation_is_emitted_when_true_and_absent_when_false`, `parameters::tests` | F1/F2 |
| R4 metadata reaches docs and extensions; runtime never reads it | `request_structs::tests::{every_rate_limit_*, an_endpoint_retry_guidance_*}`, `module_docs::tests` (2), `output::assemble::api::tests::grouped_module_docs_*`, `vocabulary_exports::metadata_keeps_every_field_at_api_and_endpoint_level`, `extensions::tests` (2), `metadata_boundary::the_runtime_neither_reads_nor_acts_on_rate_limit_or_retry_metadata` | F1/F2; source scan F1 |
| R12 GitHub adoption | `github::tests::api_declares_only_the_documented_rate_limit_headers`, `vocabulary_exports::github_rate_limit_adoption_reaches_every_output`, `artifact_drift::*`, `definition_baseline::{github, adoptions_accept_only_the_named_api_field_and_exact_value}` | definition F1; committed artifacts equal to fresh generation, F2 |
| R6 composition export and Postman diagnostic | `vocabulary_exports::{composed_components_export_their_references, postman_warns_about_each_composed_component_*, group_merge_reports_a_conflicting_*}`, `export::tests::composed_components_export_as_compositions_of_references`, `conversion::tests` | F2 writers |
| R8 OAuth consumers: grants, PKCE modes, client auth, default scopes; OpenAPI device-code diagnostic | `wiremock_flows::*` (5 new), `manager::tests`, `vocabulary_exports::openapi_warns_about_device_code_*`, `security::tests` | HTTP receiver F2; writers F2 |
| R5 Basic unchanged | `artifact_drift::*` (Bitbucket/EMQX bytes), `postman_golden::golden_mixed_auth_emqx`, Phase 2 precedence tests | F2 |
| R11 historical comparison with the named adoption | `definition_baseline::*` (30) | F1 |

**Input robustness.** This phase adds no file or configuration reader. The only parse-contract change exercised is the adopted `rate_limit` on a shipped definition, whose serde contract (absent → none, `null` → error, `{}` → error, duplicate key and trailing content → error) is now walked by `shipped_json_field_matrix_preserves_required_and_optional_shapes`, in addition to Phase 2's per-validator matrices. Smell grep of new load-bearing parse code: none added (no new `#[serde(default)]`, `unwrap_or_default()`, `filter_map(.. as_str())`, or `.ok()` on configuration). The runtime's existing `std::env::var(..).ok()` in `first_non_empty` is unchanged.

**Checkpoint met.** Definition, generator, OAuth2, runtime, and generated fixture code compile. Local mock requests prove authentication isolation (public, replacement, sibling-after-public). Rust, OpenAPI, and Postman evidence covers the supported features, with D10 warnings for the gaps (`endpoint-auth`, `schema-composition`, `oauth2-grant`; a `path-parameter` Rust error for undeclarable paths). Schematic L1, lint, doctest, and `lint-schema` pass. Last completed phase: 3.

**For review (summary of this phase's choices):**
- the replacement "missing source" error, and `.anonymous()` honored on replacements;
- no ABI bump;
- honoring `client_auth`, and grant-guarded OAuth flows;
- scheme and Postman variable naming;
- the deterministic common path declaration;
- enum naming from the Rust field name;
- effective metadata repeated on every request doc and every Postman request (GitHub: 16 sections). Dropping the per-request copy and keeping module/collection-level only is a cheap alternative if the repetition is unwanted;
- the pre-existing duplicate path parameters in `openapi/emqx.json`;
- module docs not listing a replacement endpoint's own environment variables;
- `schematic-oauth` having no device-code flow.

**Unfinished or deferred (later phases, not Phase 3 scope):**
- self-contained bundles (Phase 4);
- query required/enum/style fidelity, using the new `param_enums` machinery, and caller migration (Phase 5);
- full documentation, README, and command pass, and multi-OS validation (Phase 6).

## Phase 4

### Starting state and orchestration

Phase 3's changes were still uncommitted in the working tree at entry (no Phase 3 commit exists after `4ef77b418`), so the scope calculator below sees Phases 3 and 4 together. No subagents were used: construction and loading share one representation, and the plan forbids delegating the loader against a guessed shape, so one owner wrote both, then the evidence.

### Design choices this phase makes

- **Bundle shape (departure from PD7's literal field list, adopted default for the rest).** PD7 proposed `{format_version, api, bindings: BTreeMap, schemas: BTreeMap, models}`. Since that draft, the prerequisite lowering made `RestApi.schemas: Vec<SchemaComponent { name, rust_path, schema }>` carry every context component: the canonical schema identity (`name`), its Rust binding (`rust_path`, absent for nested-only schemas), and the OpenAPI-3.0 schema, in registration order. That order is meaningful (it is the OpenAPI `components.schemas` order), and a `BTreeMap` would lose it, so an isolated export could not equal the registry export. Separate `bindings`/`schemas` maps would also be a second source of truth needing its own consistency rules. The bundle is therefore `{"format_version": 1, "api": RestApi (with its ordered schemas), "models"?: ModelCatalog}`. The PD7 matrix columns map as: `bindings` → `api.schemas[].rust_path`, binding `schema_identity` → `api.schemas[].name`, `schemas` → `api.schemas[].schema`. Version 1 only, other versions fail explicitly (PD7). Recorded here as a departure; the spec is unchanged.
- **API (adopted).** `schematic_define::{DefinitionBundle, BundleError, BUNDLE_FORMAT_VERSION}` in `define/src/bundle.rs`: `DefinitionBundle::new(api)`, `.with_models(catalog)`, `.api()`, `.models()`, `.into_api()`, `.to_json()`, `DefinitionBundle::from_json(text)`. Construction validates exactly what loading validates, so every constructed bundle loads.
- **Canonical JSON (PD7).** Keys sorted at every depth (explicitly, in case a build enables serde_json `preserve_order`), arrays kept in order, absent options omitted rather than `null`, two-space pretty print, trailing newline.
- **Null handling (contracts.md loader obligation).** The definition types serialize some absent options as `null` (`EnvMapping`, `ParamDef.description`, plain `Option` fields); the writer omits them, and the loader rejects explicit `null` everywhere outside `api.schemas[].schema`, which is opaque data. The one exception is a grant-inapplicable OAuth2 field (`authorization_url`, `device_authorization_url`, `pkce`), which PD9 and the plan's OAuth2 table (G) allow to be null. contracts.md had written "until ruled otherwise irrelevant explicit null is rejected"; the plan's later table and PD9's "rejects non-null contradictory fields" govern, so adopted default: null accepted for those three fields only (source: plan OAuth2 JSON table, spec PD9).
- **Dropped-member check (new, to meet "never silently dropped").** `RestApi`, `Endpoint`, `EndpointParams`, and `ParamDef` do not `deny_unknown_fields`. Rather than changing bare serde (which would change the historical-snapshot contract), the loader re-serializes the typed value and requires every input member to survive with its value; unknown or misspelled keys fail as `unknown member`. An explicit default that the writer omits (`false` for `deprecated`/`secret`/`pager_managed`, `[]` for `path`/`explicit`) is accepted, through a named list (`OMITTED_DEFAULTS`) coupled to those `skip_serializing_if` attributes; the skill notes that coupling.
- **Reference normalization (R7).** `DefinitionContext::components` (`components.rs::strip`) now resolves a stored component's `"$ref": "#"` to its registration root, as the wire-shape comparison already did. Only there is the root still known: a nested type recursing into the type that contained it carried `#` into its own component, where it would have meant "the document root" in an exported OpenAPI file. No shipped API has such a reference; every committed artifact is unchanged (artifact drift passes). Bundle construction then renames `#/$defs/X` and `#/definitions/X`, resolves a *registered* component's `#` to itself, and refuses every other reference (URLs, other files, other pointers, a nested component's `#`). Loading requires every `#/components/schemas/X` to name a bundled component. `$ref` keys under `const`/`default`/`enum`/`example`/`examples` are instance data and are left alone.
- **Shared validation (refactor).** The API-level checks of `RestApiBuilder::build` moved verbatim into `builders::validate_api(&RestApi)`, which build and the bundle both call. The Basic-with-`env_auth` rule moved into it because it is a fact of the fields; the builder-only rules (one authentication declaration, `env_username` merging) stay in `build`.
- **Catalog consistency.** Each catalog model must be a registered component, listed once, name only registered members, and a composition alias must equal its component's `oneOf`/`anyOf`/`allOf` schema and have one or more nominal members.
- **CLI wiring (contracts.md R7 row).** `schematic-gen bundle --api NAME [--out FILE]` writes a bundle; `schematic-gen export-bundle --input FILE [--openapi-out DIR] [--openapi-format json|yaml] [--postman-out DIR]` loads it without the registry and publishes through the normal OpenAPI/Postman writers as a partial (keep-other-files) batch. New `GeneratorError::Bundle { path, source }`.
- **Fixtures.** Shipped bundles are not committed: OpenAI's is 1.9 MB, and all 18 are produced and checked in-test from the registry. One real-tool fixture is committed, `define/tests/fixtures/bundles/fixture.json`, produced by `to_json()` from the typed fixture API in `define/tests/bundle_support/mod.rs`; a drift test fails when it is stale. **Literal review:** the fixture holds only example URLs under `example.com`, the header `X-Fixture-Version: 2026-10-21`, environment names (`FIXTURE_USER`, `FIXTURE_PASSWORD`, `FIXTURE_PASSWORD_OLD`), and a schema example `item-1`. The 18 shipped bundles (generated to a temp dir and inspected, not committed) hold only environment variable names and versioning/`User-Agent`/`Accept` headers; no credential values. Nothing is redacted, and no environment value is read.

### Wave 6: Bundle implementation

Construction, canonical writing, the strict loader, and the CLI landed together. Smoke run before tests: all 18 shipped APIs bundle, load, and export through the CLI; the isolated OpenAPI and Postman output for every single-API module was byte-identical to the committed `schematic/openapi/*.json` and `schematic/postman/*.json` (grouped modules compared per API in-test instead). One defect was found by the new tests and fixed: construction normalized references but did not check that namespaced targets exist; the closure check now runs in the shared `validate` used by both construction and loading.

### Wave 7: Bundle evidence

| Behavior | Test | Boundary / rung |
| --- | --- | --- |
| Committed fixture is the tool's output | `define/tests/definition_bundles.rs::committed_fixture_is_the_bundle_tool_output` | repository file vs. writer, F2 |
| Loads without the registry (this crate cannot reach `schematic-definitions`), equals the built definition and catalog, exports OpenAPI identically; recursive root, nested recursion into root, `Vec`/`HashMap`/`Option` containers, alias identity vs. Rust path, arbitrary JSON, `one_of` composition; data nulls kept | `committed_fixture_loads_and_exports_like_the_built_definition` | file → public loader → OpenAPI writer, F2 |
| Repeated write/read/write/read, byte-equal | `bundle_files_round_trip_byte_equal_through_repeated_writes_and_reads` | filesystem, F2 |
| Removed endpoint component → named error (API, endpoint, name) | `removed_endpoint_component_is_a_load_error_naming_api_endpoint_and_name` | public loader, F1 |
| Removed nested component → named error (API, referring component, reference) | `removed_nested_component_is_a_load_error_naming_its_referrer` | F1 |
| Recursive map determinism; array order; data null | `canonical_json_does_not_depend_on_map_insertion_order` | writer, F1 |
| Normalization of `$defs`/`definitions`/`#`, data keywords untouched | `construction_normalizes_local_references_into_the_component_namespace` | F1 |
| External, other-file, other-pointer, dangling, ambiguous nested `#` refused without fetching | `construction_refuses_external_unresolved_and_ambiguous_references` | F1 |
| Construction runs builder validation | `construction_runs_the_builder_validation` | F1 |
| Catalog consistency (empty kept, duplicate, unregistered, nested-only, member, mismatch, no members, empty module path) | `catalog_must_agree_with_the_bundled_components` | F1 |
| The input matrix: 205 single-edit cells plus a whitespace-wrapped positive control | `bundle_json_matrix` | public load (+ OpenAPI export and round trip for every loaded cell), F1 |
| Every shipped API (count from `all_apis()`, not hard-coded): file round trip, equality, byte stability, OpenAPI and Postman export equal to the registry's | `gen/tests/l1/definition_bundles.rs::every_shipped_bundle_round_trips_through_a_file_and_exports_like_the_registry` | registry vs. file-loaded bundle through the real writers, F2 (0.9 s for all 18) |
| CLI `bundle` then `export-bundle` reproduces the committed GitHub OpenAPI and Postman bytes | `bundle_then_export_bundle_reproduces_the_committed_artifacts` | spawned binary + repository artifacts, F2 |
| CLI malformed bundle (duplicate key; explicit null) → located error on stderr, non-zero exit, nothing written | `export_bundle_reports_a_located_error_and_writes_nothing` | spawned binary, F2 wiring |
| Bare-definition snapshots and their description-mutation negative control unchanged | existing `definition_baseline::*`, `artifact_drift::*` (38 run with the composition/registration tests, all pass) | F1/F2 |

**Matrix coverage (`bundle_json_matrix`).** Default outcome: a located error (line and column, JSON Pointer, or API/endpoint/component/name) and no bundle. Cells per the plan's tables, each one edit to the canonical fixture:
- *Bundle root:* `format_version` absent/null/string/empty/0/2/1.5; unknown root member; `api` absent/null/wrong/empty, unknown member, misspelled endpoint member, explicit default `false` (loads); `models` absent (loads, none)/null/wrong/`{}`/`{"types": []}` (loads, explicitly empty)/one and every wrong element/unregistered/disagreeing composition.
- *Components:* `schemas` absent/null/wrong/`[]` (fails: endpoints reference components)/one and every wrong element; identity absent/null/wrong/empty/duplicate; binding absent (fails: endpoint reference needs a binding)/null/wrong/empty; schema absent/null/wrong/`{}` (loads: arbitrary JSON)/external ref/dangling ref.
- *Additive definition fields,* API- and endpoint-level separately: `rate_limit`, `retry_guidance` (absent → none at API, inherits at endpoint), endpoint `auth` (absent inherits, `"Public"`, null, wrong, `{}`, nested wrong, policy accepting nothing), endpoint/parameter `deprecated` (absent false, true, null, string, empty), `params.path` (absent and `[]` legacy, null, wrong, one/every wrong element, undeclared placeholder).
- *Rate-limit fields:* each header absent/null/wrong/empty; `retry_after_header` absent loads as none; `reset_unit` absent/null/wrong/empty.
- *Retry guidance:* `retry_on` absent/null/wrong/every wrong/`[]` loads selecting nothing; `honor_retry_after`, `max_attempts` (incl. 0), `backoff` (absent/null/wrong/`{}`/`"none"` loads); selector code null/string/out of range; range start absent/null, end wrong, reversed; fixed delay absent/null/0; exponential base absent/null, cap wrong, cap below base.
- *Basic lists:* username/password wrapper absent/null/wrong/`{}`; `names` one/every wrong element; `[]` loads with that fallback disabled; order kept.
- *OAuth2, per grant:* authorization code (every field absent/null/wrong/empty; device URL null loads, non-null contradicts; revocation absent loads; scopes `[]` loads; PKCE Required loads, unknown fails; unknown member); client credentials (inapplicable URL/PKCE/device null load, non-null and empty contradict; token empty; scopes absent; client_auth wrong); device code (device URL absent/null/wrong/empty fail; authorization URL null loads; PKCE contradicts; token wrong).
- *Document shapes:* duplicate key at root, in API configuration, in an OAuth2 object, in a schema, at example depth; trailing garbage; second document; comment; trailing comma; truncated; not an object; empty document; surrounding whitespace (control, loads).

Not covered as separate cells (same validator as a covered column): every pagination strategy field, `EnvMapping` members, form-field shapes, and model struct/enum payloads. They reach the same three loader layers (null preflight, typed serde, dropped-member check), and Phase 2's per-validator matrices cover their typed serde shapes; the shipped round trip exercises pagination-bearing bundles (OpenAI, GitHub, GitLab) in their present forms.

**Isolation.** The loader reads only text: it has no access to `schematic-definitions`, never runs a Rust binding, never compiles a client, and never reads the environment or fetches a reference. The define-side tests prove it structurally (that crate cannot link the registry).

### Phase gates and checkpoint

- `cd schematic && just lint`: PASS for every package (`/tmp/phase4-lint.log`). The first run failed because `define/tests/definition_bundles.rs` used the `openapi` feature unconditionally while lint builds that crate without features; the export helper is now feature-gated (CI's L1 enables all features of `schematic-define`, so the export assertions run there).
- `cd schematic && just test` (no subagents, run once): ordinary L1 **2469/2469 passed** (15 skipped tier exclusions), fresh-output 9/9, compile-fail 1/1 (`/tmp/phase4-test.log`). Four ordinary tests reported over 5 s during the run (`import_normalization::shipped_openai_query_schema_gate_and_reimports`, `artifact_drift::generated_rust_artifacts_are_up_to_date`, `fresh_output_compile::generated_rest_code_references_the_runtime_and_defines_none_of_it`, `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition`). Rerun alone they took 2.4–3.3 s; host load average was about 90 from other sessions. Recorded as load-sensitive, none touched by this phase. The new bundle tests take 0.2 s (define, 11 tests) and 0.9 s (all 18 shipped bundles).
- `just doctest`: PASS, including the new `DefinitionBundle` example. `just check-tier-coverage schematic`: 0 stranded. No new test name carries a tier marker. `define/tests/definition_bundles.rs` is an autotest of `schematic-define`; `gen/tests/l1/definition_bundles.rs` is declared in `gen/tests/l1/main.rs`. Repository reads are `include_str!` inside each `#[test]`.
- Scope calculator (from the root, after every edit): `change_class: package`, `packages: [schematic-define, schematic-definitions, schematic-gen, schematic-oauth, schematic-runtime, test-toolkit, darkmatter]`, `reverse_dependencies: [model-citizen]`. This includes Phase 3's uncommitted changes. Gates were required and were run.
- **Dependents** (serial, targeted): `model-citizen` `cargo build --all-targets` clean and `-E 'test(ollama) | test(lmstudio)'` 48/48. Darkmatter's skill and Markdown corpus tests (`shipped_markdown_*`, `current_root_migration_guard`) 4/4, covering the skill edit and the new docs page. `test-toolkit` is a lint-companion row only, as in Phase 3; not run locally. No generated `schematic-schema` code changed, so its consumers were not rebuilt.
- **Cross-OS:** no notable risk. No path normalization, process, terminal, or `#[cfg]` code was added. The CLI writes with `std::fs`, tests use temp directories, and the artifact comparisons rely on the repository's `eol=lf` attributes, as `artifact_drift` already does. `just cross-check` was not run; host exercised: macOS only.
- **Generated-output differences (R11):** none in this phase. `just test`'s `artifact_drift` and the bundle CLI test confirm every committed Rust, OpenAPI, and Postman artifact still matches fresh generation. The `flatten` change alters stored component schemas only where a nested type's `"$ref": "#"` occurs, and no shipped API has one.

**Input robustness smell grep** (`define/src/bundle.rs`): one `#[serde(default, deserialize_with = "non_null")]`, on `models`, whose absent → "no catalog" is the contract, and null fails both the preflight and `non_null`. No `unwrap_or_default()`, `filter_map(.. as_str())`, or `.ok()` on a load-bearing parse.

**Checkpoint met.** Every shipped bundle (count taken from `all_apis()`, currently 18) loads in isolation and exports exactly as the registry does. Deterministic equality, all matrix cells, and the malformed-reference diagnostics pass. No credential was resolved or redacted. There are no regeneration differences. Last completed phase: 4.

**For review (this phase's choices):**
- the bundle shape departs from PD7's separate `bindings`/`schemas` maps (reasons above);
- the dropped-member check and its `OMITTED_DEFAULTS` list, coupled to `skip_serializing_if` attributes;
- null accepted for the three grant-inapplicable OAuth2 fields (plan table and PD9 over contracts.md's provisional rejection);
- `DefinitionContext::components` now resolves nested root references (`#`), with no shipped output change;
- two new CLI subcommands (`bundle`, `export-bundle`) and `GeneratorError::Bundle`;
- shipped bundles are checked in-test rather than committed (OpenAI's is 1.9 MB); one typed fixture bundle is committed;
- `bundle_json_matrix` checks some error messages by line/column only, where serde's own message for an internally tagged or `try_from` type is just `expected value`. Those cells pin the fixture's line numbers, so a fixture change moves them.

**Unfinished or deferred (later phases, not Phase 4 scope):**
- query required/enum/style fidelity and caller migration (Phase 5);
- the `schematic-define` README and `create-schematic-api` command pass for bundles, and multi-OS validation (Phase 6).
