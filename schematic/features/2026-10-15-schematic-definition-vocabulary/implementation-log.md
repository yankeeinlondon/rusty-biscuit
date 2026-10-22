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
human_review: false
message_to_agent: |-
  Phase 2 is complete; all schematic gates pass and generated output is
  byte-identical. Define side now has: metadata (RateLimit/RetryGuidance,
  RestApi::effective_rate_limit/_retry_guidance), per-grant OAuth2Config
  with accessors (grant_type(), token_url(), pkce() ...), composition
  (ctx.one_of/any_of/all_of; TypeDesc::structure(); the composition
  replaces the bound type's schema in components()), Basic { username_env,
  password_env } with env_username removed, EndpointParams.path,
  deprecated on Endpoint/ParamDef, Endpoint.auth (EndpointAuth::Public /
  Policy { policy, env_mapping }) resolved by RestApi::endpoint_auth, and
  Endpoint::path_placeholders() (first-occurrence order, {+x} -> x).
  Nothing in gen/export/runtime reads these new fields yet: that is Phase
  3. The generator's generated-name-collision check must start reading
  declared path rust_names; define only checks a simple normalization.
  The importer always emits deprecated: false (OpenAI mod.rs is importer
  output; keep it byte-identical via import_overlay). Several gen files
  (codegen/api_struct/{constructors,fields,variant}.rs, codegen/client/*,
  codegen/request_enum.rs) are not declared modules and never compile;
  do not rely on them. EnvMapping and ParamDef.description serialize
  unset values as null, which matters for Phase 4's strict bundle loader.
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
