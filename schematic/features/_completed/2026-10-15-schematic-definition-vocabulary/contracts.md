---
kind: research
---

# Vocabulary contract and evidence ledger

This is a preparation record for the next implementation phases. Existing behavior is distinguished from planned loader and vocabulary behavior. No new API is implemented here. Coordinates below are relative to the repository root.

## Actual prerequisite interfaces

| Boundary | Current interface and source | Consequence |
| --- | --- | --- |
| API construction | `RestApi::builder(&DefinitionContext, name, base_url)` → typestate `RestApiBuilder`; `.describes(...)` and authentication are required; `.build() -> Result<RestApi, DefinitionError>` in `schematic/define/src/builders.rs` | Preserve signatures, including `.basic_env(username, passwords)`, `.bearer_env(names)`, `.env_mapping(mapping)`, and `.auth_policy(policy)`. |
| Endpoint construction | `Endpoint::get/post/put/patch/delete/head/options(id, path)` → `EndpointBuilder`; `.describes`, `.returns(Success)` and `.build()`; `.request_json`, `.query`, `.header_param`, `.cookie`, `.pagination` | Add optional vocabulary methods. Do not replace the typestate construction API. |
| Context registration | `DefinitionContext::types_in`, `.register::<T>()`, `.register_as(rust_path!(...))`, `.finish()`, `.components()` in `define/src/registration.rs` | `Registration` includes serde, schemars, and nominal registration evidence. Context errors accumulate. Concrete generic Rust arguments and alias-first naming survive. |
| Descriptor | Private `TypeDesc { reference: TypeRef, rust_path: String }`; accessors `type_ref`, `into_type_ref`, `rust_path`, `wire_schema` | Contrary to the old draft, it has no serde, context identity, or reserved structural field. Accepted prerequisite lowering stores component names and container shape in REST `TypeRef`. Same-context structural registration needs a deliberate extension in Phase 2, not reimplementation of registration. |
| Schema identity | `SchemaComponent { name, rust_path: Option<String>, schema: Value }`; `RestApi::schemas`, `.component`, `.rust_type`, `.validate_references` in `define/src/components.rs` | Nested-only schemas lack a Rust binding; registration order is meaningful. OpenAPI-3.0 schemars settings and component namespace are authoritative; neither Rust paths nor `Vec<T>` spellings are component names. |
| Reference forms | `TypeRef::{Primitive,Array,Map,KeyedMap,Named,OneOf,AnyOf,AllOf,Optional,Unknown}` in `define/src/models/type_ref.rs` | Composition forms exist at wire-schema level but do not resolve to Rust types without a binding. `Schema { type_name, module_path }` is WebSocket-only. |
| Catalog | `ModelCatalog { module_path, types }`, `ModelDef::{Struct,Enum,Alias}` in `define/src/models/` | No catalog serde and no context-owned catalog exist yet. Preserve existing model structures while adding composition storage. |
| Endpoint identity | Query-free `(method, normalized path)`; `?` rejected in builder/shared validation/import | No selectors, fixed query declarations, or new endpoint identity rule. Existing placeholder checks precede typed path matching. |
| Authentication | API `auth_policy` retained; no endpoint `oauth_scopes`; unit `AuthStrategy::Basic`; flat `OAuth2Config` | `AuthMethod::Basic` and `EnvAuthStrategy::Basic` stay source-neutral. Explicit credential precedence and API mapping/policy overrides already exist. Endpoint auth override does not exist. |
| Runtime | `schematic/runtime/src/endpoint.rs` owns `EndpointSpec`; runtime call/client and pager modules own requests | Keep ABI/trait use and existing secret/origin protections. Metadata remains unused by runtime. |
| Pagination resolution | `define/src/pagination/resolve.rs`: `resolve`, `caller_knobs`, `param_adoptions` | `ParamAdoption` carries role, input, location, name, required, outcome, and `has_sent_default`. Use actual adopted outcomes, not a name heuristic. |
| Pagination defaults | `PageSize::sent_default` is sent, `server_default` is documentation; page/offset index base, constants, session values, and cursor availability in `pagination/strategy.rs` | Required inputs need a type-valid initial value. A server default alone cannot satisfy R14; cursor absence cannot be treated as a sent value. |
| One page | Generated `fetch_page()` in `gen/src/codegen/endpoint_methods/mod.rs`; runtime `pager/pages.rs` and `pager/session.rs` | Adopted fields remain removed from ordinary requests. No convenience client methods. |
| Query extraction | `QueryParamInfo { name, rust_name, description, param_type }` in `gen/src/codegen/request_structs/shared.rs` | Required/style/explode are currently lost. Phase 5 must retain them and share enum machinery with typed paths. |
| Historical snapshots | `gen/tests/fixtures/definition-baseline/*.json`; support `gen/examples/support/definition_snapshot.rs` | 18 bare-definition snapshots include ordered component fingerprints and resolved adoption data. Preserve original files; bundles are separate fixtures. |
| Isolated compilation | `gen/tests/l1/fresh_output_compile.rs`, declared by `gen/tests/l1/main.rs` | Compiles fresh generated clients outside the checkout, offline, with runtime requests to loopback fixtures. Reuse this harness, not committed schema as the only compile evidence. |

## Output diagnostic inventory

`schematic/gen/src/diagnostics.rs` supplies `ValidationDiagnostic`, `Severity`, `Feature`, and `Output::{Rust,OpenApi,Postman}`. Display and JSON identify API/group, optional endpoint, feature, output, and message. `pipeline.rs`, `validate.rs`, and `export/` validate per output and retain successful sibling targets.

Current feature IDs: text-request-body, binary-request-body, api-key-param, query-parameter, header-parameter, cookie-parameter, duplicate-endpoint-id, duplicate-method-path, invalid-type-syntax, duplicate-operation-id, unresolved-link-operation, query-in-path, invalid-identifier, invalid-request-suffix, generated-name-collision, endpoint-method-rename, invalid-module-path, invalid-output-filename, invalid-schema-reference, pagination-strategy, pagination-exception, pagination-parameter. New vocabulary needs appropriate feature IDs; do not invent success for an unsupported target. In particular, explicit composition gets a Postman unsupported diagnostic while Rust binding and OpenAPI schema generation can succeed. Existing Postman examples must survive.

## Requirement-to-test mapping before implementation

All tests below are L1 unless stated otherwise. “Planned” names are explicit proposed cases, not claims that they currently exist. F1 pins synthetic contract decisions; F2 requires a real compiler, generated client, writer, or HTTP receiver to observe the output. Mock-server evidence is F2 for generated-client execution and F1 for provider wire semantics; no live provider evidence is claimed.

| Requirement | Boundary and concrete test/evidence | Status |
| --- | --- | --- |
| R1 typed paths | Planned `typed_paths_scalar_enum_and_reserved_wire`: compile fresh scalar/enum requests, send repeated/reserved placeholders to loopback HTTP; F2. `typed_paths_reject_invalid_declarations`: missing/extra/duplicate, required=false, non-Simple, Array/Json, nonfinite numbers, enum collisions; F1. Compare Rust docs and OpenAPI per-operation overrides. | Future Phase 2/3 |
| R2 endpoint auth | Planned `endpoint_auth_public_then_authenticated`: Authorization/custom/query/cookie credentials, explicit injection, absent environment on public call, ordinary headers/cookies preserved, subsequent authenticated request unchanged; real generated invocation, F2. `endpoint_auth_replacement_sources` waits for ruling; incompatible/missing mapping errors occur before send. OpenAPI security and Postman noauth output assertions, F2 writers. | Credential-source decision blocked |
| R3 deprecation | Planned `deprecation_round_trip_and_outputs`: absent/false/true, explicit null error, native boolean versus string, repeated serialize/read/write/read; Rust docs and OpenAPI flags; F1 + F2 writers. No Rust deprecated attributes. | Future |
| R4 metadata | Planned `metadata_bounds_and_replacement`: 99/100/599/600, reversed ranges, attempts 0/1/255, delay 0/positive, cap below/equal/above base, invalid header names, all fields preserved and endpoint replacement; F1. `metadata_reaches_docs_and_openapi` reads shipped GitHub artifact through normal generation, F2. | Future |
| R5 Basic | Existing `definition_baseline::{bitbucket,emqx_basic}`, artifact byte comparisons. Planned `basic_ordered_fallback_and_explicit_precedence`: empty/unset fallback values, multiple user/password names, explicit mapping/policy including explicit empty mapping, shared WebSocket path; F1 resolution plus F2 generated requests. | Current baseline passes; new semantics future |
| R6 composition | Planned `composition_same_context_and_round_trip`: oneOf/anyOf distinguishable values, allOf satisfying struct, empty/foreign/unregistered members, name conflicts, nested references; F1. Compiler and serde consume registered Rust binding, F2; OpenAPI references and Postman output-local diagnostic, F2 writers. | Requires descriptor/catalog extension |
| R7 bundles | Planned `bundle_load_and_export_without_registry`: real-tool bundles through public construction → JSON → filesystem read → isolated load/export → write/read again; canonical equality, full closure/cycles, no registry/compilation; F2. `bundle_json_field_shapes` and `bundle_json_document_shapes`: positive real fixture plus single-edit cells, located public errors; F1. CLI control + malformed shape/location, F2 wiring only. | Future Phase 4 |
| R8 OAuth2 | Planned `oauth_grant_payload_matrix`: all three tags, required URLs, wrong/null/empty shapes, contradictory non-null fields, both authorization-code PKCE modes; F1. Compile-fail illegal combinations, F2 compiler. Existing `schematic/oauth/tests/wiremock_flows.rs` normal token/refresh/revocation paths observe grant/client-auth behavior, F2 execution. | Future |
| R9 query fidelity | Planned `query_required_enum_and_serialization_wire`: normal generated clients send scalar/array/enum values, commas/pipes/spaces/percent/Unicode delimiters once encoded, None omission versus Some(empty) `name=`, both Form explode modes and supported delimited arrays; F2. Compile-fail required-field Default and enum mismatch, compiler F2. Unsupported style/type gives target-local diagnostic. | Future Phase 5 |
| R10 source preservation | Baseline fixture comparison, full authoring-source diff, generated-file difference allow-list; no broad normalization. | Baseline passes |
| R11 migration | Existing `definition_baseline::*`, `artifact_drift::*`, isolated compile harness; caller coordinates in inventory.md. All changed dependents tested serially later, narrowing packages over 200 tests to actual consumers. | Baseline passes |
| R12 adoption | Planned `github_rate_limit_adoption_outputs`; Bitbucket/EMQX use the unit Basic variant with `.authentication`, so a payload variant requires a narrowly permitted mechanical source migration; flag the R10/R12 conflict and preserve generated bytes. No OpenAI rewrite. | Future |
| R13 guidance | Later compiling authoring examples / `just doctest` and authoring-guidance corpus, F2 compiler; inspect docs/skills for drift. This evidence phase changes no public guidance. | Future |
| R14 pager exemption | Existing snapshot records adoption/default state. Planned `required_adopted_and_unmanaged_query_wire`: two required fields, only unmanaged in new(), pager first/subsequent pages and fetch_page observe defaults, F2. `required_adopted_without_valid_default_fails` names API/endpoint/strategy/parameter, F1. | Future |

## Input robustness contract

The future public bundle loader accepts JSON only. Default invalid outcome: a located error, no execution, no publication, and no partial export. Wrong whole-field types and invalid elements are errors without filtering; an all-invalid list is never an empty list. Duplicate JSON object keys are rejected recursively before construction of `Value`, including inside opaque schema/example JSON; this enforces syntax, not payload semantics. Trailing garbage, comments, trailing commas, and concatenated JSON documents fail; whitespace around one valid JSON document succeeds. YAML native/quoted values, multiple documents, and JSON5 syntax are outside the JSON-only contract.

The existing raw `RestApi` serde reader is **not** this loader. Its optional fields accept absent and null as None; optional nulls are emitted in current snapshots. Its required vectors reject absence/null but accept explicit empty; defaulted booleans reject null but accept absence as false. The existing matrix verifies API/endpoint/parameter rows only; it does not establish safety of auth, nested schemas, maps, or future catalog loading. Bundle preflight must see presence before typed deserialization. To enforce strict null rejection without rejecting its own output, bundle serialization must omit absent configuration options (or explicitly specify nullable semantics before implementation); do not silently pass raw snapshot nulls through preflight. This is a planned loader/writer obligation, not a Phase 1 change to bare serde or snapshots.

### Field validators and exceptions

The following policies are for **bundle loading**. Each listed field receives its own absent outcome; null is rejected for configuration fields unless explicitly allowed below. The generated appendix inventories every inherited struct field and its current raw-serde behavior, including pagination. It is not a claim that bare serde performs semantic validation.

| Validator / fields | Absent outcome | Present empty outcome / semantic constraints |
| --- | --- | --- |
| `RestApi.name`, `description`, `base_url`, `auth`, `env_auth`, `headers`, `endpoints`, `schemas`, `cross_origin`, `secret_names` | Required (builder defaults do not make inherited serialized fields optional) | Names/base URL must be usable; description empty preserved; explicit empty lists preserved. Empty endpoints/components allowed for a component-free API. Auth and origin enum shapes must be valid. |
| `RestApi.docs_url` | No documentation URL | Empty string is invalid URL. |
| `RestApi.auth_policy` | Derive from legacy auth | `{}` is explicit no-auth policy, preserved. |
| `RestApi.env_username` | No legacy username | Empty name rejected when selected; remove field in Basic migration. |
| `RestApi.module_path` | Derived module | Empty invalid. |
| `RestApi.request_suffix` | Request | Empty invalid. |
| `RestApi.version` | Export option or 1.0.0 | Empty string preserved as authored metadata pending existing exporter validation. |
| `RestApi.env_mapping` | Legacy mapping | `{}` explicitly replaces legacy mapping; do not refill it. |
| `RestApi.request_id_header` | No provider header | Empty invalid header name. |
| `RestApi.default_pagination` | No default pager | Empty object invalid strategy. |
| `Endpoint.id`, `method`, `path`, `description`, `response`, `headers` | Required | Empty description/list preserved; empty ID/path invalid; response/method require supported enum. |
| `Endpoint.request` | No body | Empty enum object invalid. GET body rejected. |
| `Endpoint.params` | No declared parameters | `{}` is invalid because inherited query/header/cookie vectors are required. |
| `Endpoint.cross_origin` | Inherit API | Empty enum invalid. |
| `Endpoint.pagination` | Infer collection or none | Empty object invalid collection. |
| `EndpointParams.query`, `header`, `cookie` | Required | [] explicitly no declarations. New `path` absent → []; [] preserves legacy untyped paths. |
| `ParamDef.name`, `required`, `param_type`, `explode`, `style` | Required | Empty name invalid; false is data; unknown enums invalid; [] Enum invalid, Array inner type must validate. |
| `ParamDef.description` | No description | Empty description preserved. |
| `ParamDef.rust_name` | Sanitize wire name | Empty invalid Rust identifier. |
| `ParamDef.secret`, `pager_managed`, future `deprecated`; endpoint future `deprecated` | false | Boolean only; quoted strings rejected. |
| `AuthPolicy.explicit` | [] | [] permits env-only/no-auth policy. Every method is validated; alternatives remain alternatives. |
| `AuthPolicy.env_fallback` | No environment fallback | Empty enum invalid. |
| `AuthStrategy` / `AuthMethod` / `EnvAuthStrategy`: `header`, `value_prefix` options | Default Authorization header / no prefix | Empty header invalid; empty prefix is meaningful. Required API-key header/name/location payloads reject absence and empty names. Unit tags carry no payload. OAuth2 payload is validated by grant. |
| `EnvMapping.bearer_token`, `basic_user`, `basic_pass`, `api_key`, `oauth_client_id`, `oauth_client_secret`, `oauth_redirect_uri` | No source for that credential | `{}` not an EnvList; explicit empty mapping does not inherit. Validate compatibility only for selected method. |
| `EnvList.names` | Required | [] stays explicit empty, never fallback to another mapping; empty name elements invalid. Order is meaningful. Mixed/all wrong elements fail. |
| `ApiKeyEnv.names`, `header` | Required | Empty names list explicit empty; empty header invalid. |
| Current `OAuth2Config.grant_type`, `token_url`, `default_scopes`, `pkce`, `client_auth` | Required | Scope [] explicit none; URL empty invalid; tags/PKCE valid for selected grant. |
| Current OAuth2 `authorization_url`, `device_authorization_url`, `revocation_url` | No URL | Empty string invalid. New authorization/device URLs required for corresponding grant; irrelevant non-null fields rejected. Until ruled otherwise irrelevant explicit null is rejected, not accepted as absence. |
| `SchemaComponent.name`, `schema` | Required | Empty name invalid; `{}` is legitimate free-form schema. `schema` and nested example/default payloads preserve allowed JSON null/empty values. References still must resolve. |
| `SchemaComponent.rust_path` | Nested component without binding | Empty string invalid Rust binding. |
| `TypeRef.Named`, `KeyedMap.key/value`, Array/Map/Optional inner values | Required payload | Empty names/key paths invalid. Known primitives preserved; composition [] invalid; Unknown is not a missing reference substitute. |
| `ApiRequest.Json`, `ApiResponse.Json`, `FormFieldKind.Json` | Required TypeRef payload | Recursively validate reference closure. Text/binary content_type required and nonempty. FormData/UrlEncoded fields required; [] explicit empty. File/Files accept vectors are required ([] unrestricted); Files.min/max absent → no bound, zero preserved where permitted, max must not contradict min. |
| `FormField.name`, `kind`, `required` | Required | Empty name invalid; false meaningful. |
| `FormField.description` | No description | Empty preserved. |
| `CrossOrigin.AllowOrigins` payload | Required | [] is explicit no allowed extra origins; validate entries, never expand scope. |
| Future bundle `format_version`, `api`, `bindings`, `schemas` | Required | Only version 1; empty maps allowed only if consistent and no missing references. No registry lookup or external fetch. |
| Future bundle `models` | No catalog | Empty catalog types [] explicit empty, not absent. |
| `ModelCatalog.module_path` | No override | Empty invalid module path. |
| `ModelCatalog.types` | Required | [] explicit empty. No catalog serde exists yet. |
| `ModelDef` Struct/Enum/Alias payload, `StructDef.name`, `EnumDef.name`, `TypeAlias.name`, `FieldDef.name`, `EnumVariant.name` | Required | Empty names invalid; collisions and Rust naming constraints checked. |
| `StructDef.fields`, `EnumDef.variants`, `EnumDef.untagged`, `FieldDef.required`, `FieldDef.field_type`, `TypeAlias.target` | Required | Empty fields allowed; empty variants invalid; booleans preserved; type refs recurse. |
| Model/field/variant `description` | No description | Empty preserved. |
| `StructDef.additional_properties`, `FieldDef.serde_rename`, `EnumVariant.value` | No extra properties / wire name defaults to field or variant name | Optional type recurses; empty wire spelling preserved only where valid, never treated as absent. |

Pagination remains configuration reachable through the API, not opaque schema data. Its inherited fields are all included in the appendix. Optional `size`, `backward`, `more`, `prev`, `total`, `follow_up`, `session`, `partition`, `method`, `path`, session handle locations, renewal and close fields default to None when absent; explicit null fails bundle preflight. PageSize server_default/sent_default/max and Partition.max absent → None; Collection.item_type absent → inferred root-array item type where allowed. Every remaining struct field is required. Empty arrays of constants, terminal_values, alternates, and carry-only names are explicit empty; empty cursor parts fail; selectors and destination names must be nonempty and valid; bounds/required sent values are checked by existing strategy validation plus R14. InputParam, ValueSource, MoreSignal, CursorSource, Boundary, LinkSource, PageStrategy, Carry, SessionOpen, Pagination, and selector payloads validate their tags and all required fields recursively. Empty objects are not valid tagged strategies. `JsonPath`/`ItemSelector` use their actual serialized segment representation, not an assumed dotted string loader.

### Matrix execution allocation

One public-loader matrix per shared validator: scalar string/URL/name, boolean, enum/tag, list, map, TypeRef, schema/reference closure, and each nested strategy/model shape. For every field using a shared validator include a positive control and an invalid shape to prove wiring. Walk absent, explicit null, wrong whole type, mixed/all wrong elements for collections, and empty once per validator; use real bundle fixtures and one edit per row. Validate dependent export results and no partial output. A JSON loader matrix separately covers duplicate keys at root, nested configuration, schema, and example payload depth, trailing/invalid content, and JSON-specific syntax. A small CLI control and one/two invalid cases prove normal invocation and located error rendering; do not spawn a process per cell. A repeated file write/read/write/read checks persistence and canonical bytes.

Code-smell inspection: existing load-bearing defaults are `AuthPolicy.explicit/env_fallback`, `ParamDef.rust_name/secret/pager_managed`, endpoint cross_origin/pagination and OAuth mapping options. Existing Option fields conflate absent/null; bundle preflight must retain presence before these deserializers. `param_adoptions(...).unwrap_or_default()` in snapshot support is a resolution-to-empty-list convenience, not loader validation. Arbitrary schema/example Values intentionally preserve null/empty data; do not apply configuration field validators inside them. Catalog serde is missing rather than safe. No Phase 1 parser is introduced, so this is an input contract and coverage plan, not a claim that future matrix tests already pass.

## Historical comparison normalization prepared for migration

Keep current bare snapshots and `definition_snapshot::{snapshot,canonical,differences}` unchanged. Current `definition_baseline::EXPLAINED` is empty. Prepare comparison at the **definition layer before recomputing fingerprints**, using these exact transformations only when the corresponding implementation lands:

1. Newly absent/default vocabulary: remove only `Endpoint.deprecated == false`, `ParamDef.deprecated == false`, `EndpointParams.path == []`, and absent optional vocabulary fields introduced by this feature. Never strip populated metadata, true flags, null booleans, arbitrary nested `deprecated` schema keys, descriptions, or schema content. New fields must be enumerated by exact type/path, not recursively erased by name.
2. Basic only: lower both representations to an ordered pair of username/password lists. Historical username is zero/one env_username; historical password behavior is the actual existing legacy mapping, which currently selects the first env_auth element. Current shipped definitions have single password entries. Normalize only these two APIs and require explicit mapping/policy equivalence. A second password name is a new behavior, never a byte-equivalence exemption.
3. Remove legacy env_username only for non-Basic APIs when absent/null and unused. Any populated value needs an exact-leaf explanation; never discard it generally.
4. Recompute only the API metadata/endpoint fingerprints affected by those exact default changes. Preserve registry component fingerprints/order and pagination resolution/adoption arrays. An explicit new feature/adoption changes semantics and needs named expected differences, not normalization.
5. Current generated artifacts always compare byte-for-byte with no historical normalization. Bundles receive new separate fixtures. Keep `changed_description_is_detected_at_its_exact_field` as a deliberate negative control; no normalization touches descriptions.

This is the narrow comparison recipe, recorded without prematurely changing test implementation in an evidence-only phase. The new defaults/Basic representation do not exist yet, so enabling normalization now would hide drift. When implemented later, add controls for every allowed transformation and mutations to populated values/order; retain the existing description control.

## Inherited struct-field audit

This appendix lists inherited fields from a source audit, separate from the strict public bundle contract. Raw-serde absence/null outcomes describe current deserializers; they do not prove semantic validity. Model catalog types have no serde contract. Empty outcomes use the shared validators above. Enum payloads are covered separately.

| Type.field | Rust shape | Current raw absent | Current raw null | Source |
| --- | --- | --- | --- | --- |
| `RestApi.name` | `String` | error | error | `schematic/define/src/types.rs:112` |
| `RestApi.description` | `String` | error | error | `schematic/define/src/types.rs:114` |
| `RestApi.base_url` | `String` | error | error | `schematic/define/src/types.rs:118` |
| `RestApi.docs_url` | `Option<String>` | None | None (presence lost) | `schematic/define/src/types.rs:120` |
| `RestApi.auth` | `AuthStrategy` | error | error | `schematic/define/src/types.rs:122` |
| `RestApi.auth_policy` | `Option<AuthPolicy>` | None | None (presence lost) | `schematic/define/src/types.rs:129` |
| `RestApi.env_auth` | `Vec<String>` | error | error | `schematic/define/src/types.rs:137` |
| `RestApi.env_username` | `Option<String>` | None | None (presence lost) | `schematic/define/src/types.rs:142` |
| `RestApi.headers` | `Vec<(String, String)>` | error | error | `schematic/define/src/types.rs:149` |
| `RestApi.endpoints` | `Vec<Endpoint>` | error | error | `schematic/define/src/types.rs:151` |
| `RestApi.module_path` | `Option<String>` | None | None (presence lost) | `schematic/define/src/types.rs:157` |
| `RestApi.request_suffix` | `Option<String>` | None | None (presence lost) | `schematic/define/src/types.rs:162` |
| `RestApi.version` | `Option<String>` | None | None (presence lost) | `schematic/define/src/types.rs:167` |
| `RestApi.env_mapping` | `Option<EnvMapping>` | None | None (presence lost) | `schematic/define/src/types.rs:189` |
| `RestApi.cross_origin` | `CrossOrigin` | error | error | `schematic/define/src/types.rs:192` |
| `RestApi.request_id_header` | `Option<String>` | None | None (presence lost) | `schematic/define/src/types.rs:195` |
| `RestApi.secret_names` | `Vec<String>` | error | error | `schematic/define/src/types.rs:199` |
| `RestApi.default_pagination` | `Option<Strategy>` | None | None (presence lost) | `schematic/define/src/types.rs:204` |
| `RestApi.schemas` | `Vec<crate::SchemaComponent>` | error | error | `schematic/define/src/types.rs:208` |
| `Endpoint.id` | `String` | error | error | `schematic/define/src/types.rs:380` |
| `Endpoint.method` | `RestMethod` | error | error | `schematic/define/src/types.rs:382` |
| `Endpoint.path` | `String` | error | error | `schematic/define/src/types.rs:386` |
| `Endpoint.description` | `String` | error | error | `schematic/define/src/types.rs:388` |
| `Endpoint.request` | `Option<ApiRequest>` | None | None (presence lost) | `schematic/define/src/types.rs:393` |
| `Endpoint.response` | `ApiResponse` | error | error | `schematic/define/src/types.rs:395` |
| `Endpoint.headers` | `Vec<(String, String)>` | error | error | `schematic/define/src/types.rs:405` |
| `Endpoint.params` | `Option<EndpointParams>` | None | None (presence lost) | `schematic/define/src/types.rs:413` |
| `Endpoint.cross_origin` | `Option<crate::CrossOrigin>` | None | None (presence lost) | `schematic/define/src/types.rs:416` |
| `Endpoint.pagination` | `Option<crate::pagination::Collection>` | None | None (presence lost) | `schematic/define/src/types.rs:419` |
| `EndpointParams.query` | `Vec<ParamDef>` | error | error | `schematic/define/src/params/mod.rs:84` |
| `EndpointParams.header` | `Vec<ParamDef>` | error | error | `schematic/define/src/params/mod.rs:86` |
| `EndpointParams.cookie` | `Vec<ParamDef>` | error | error | `schematic/define/src/params/mod.rs:88` |
| `ParamDef.name` | `String` | error | error | `schematic/define/src/params/param_def.rs:28` |
| `ParamDef.rust_name` | `Option<String>` | None | None (presence lost) | `schematic/define/src/params/param_def.rs:31` |
| `ParamDef.secret` | `bool` | default | error | `schematic/define/src/params/param_def.rs:34` |
| `ParamDef.pager_managed` | `bool` | default | error | `schematic/define/src/params/param_def.rs:37` |
| `ParamDef.required` | `bool` | error | error | `schematic/define/src/params/param_def.rs:39` |
| `ParamDef.description` | `Option<String>` | None | None (presence lost) | `schematic/define/src/params/param_def.rs:41` |
| `ParamDef.param_type` | `QueryParamType` | error | error | `schematic/define/src/params/param_def.rs:43` |
| `ParamDef.explode` | `bool` | error | error | `schematic/define/src/params/param_def.rs:48` |
| `ParamDef.style` | `ParamStyle` | error | error | `schematic/define/src/params/param_def.rs:50` |
| `AuthPolicy.explicit` | `Vec<AuthMethod>` | default | error | `schematic/define/src/auth.rs:260` |
| `AuthPolicy.env_fallback` | `Option<EnvAuthStrategy>` | None | None (presence lost) | `schematic/define/src/auth.rs:263` |
| `EnvList.names` | `Vec<String>` | error | error | `schematic/define/src/headers/env.rs:28` |
| `ApiKeyEnv.names` | `EnvList` | error | error | `schematic/define/src/headers/env.rs:110` |
| `ApiKeyEnv.header` | `String` | error | error | `schematic/define/src/headers/env.rs:111` |
| `EnvMapping.bearer_token` | `Option<EnvList>` | None | None (presence lost) | `schematic/define/src/headers/env.rs:150` |
| `EnvMapping.basic_user` | `Option<EnvList>` | None | None (presence lost) | `schematic/define/src/headers/env.rs:151` |
| `EnvMapping.basic_pass` | `Option<EnvList>` | None | None (presence lost) | `schematic/define/src/headers/env.rs:152` |
| `EnvMapping.api_key` | `Option<ApiKeyEnv>` | None | None (presence lost) | `schematic/define/src/headers/env.rs:153` |
| `EnvMapping.oauth_client_id` | `Option<EnvList>` | None | None (presence lost) | `schematic/define/src/headers/env.rs:155` |
| `EnvMapping.oauth_client_secret` | `Option<EnvList>` | None | None (presence lost) | `schematic/define/src/headers/env.rs:157` |
| `EnvMapping.oauth_redirect_uri` | `Option<EnvList>` | None | None (presence lost) | `schematic/define/src/headers/env.rs:159` |
| `OAuth2Config.grant_type` | `OAuth2GrantType` | error | error | `schematic/define/src/oauth.rs:34` |
| `OAuth2Config.authorization_url` | `Option<String>` | None | None (presence lost) | `schematic/define/src/oauth.rs:36` |
| `OAuth2Config.token_url` | `String` | error | error | `schematic/define/src/oauth.rs:38` |
| `OAuth2Config.revocation_url` | `Option<String>` | None | None (presence lost) | `schematic/define/src/oauth.rs:40` |
| `OAuth2Config.device_authorization_url` | `Option<String>` | None | None (presence lost) | `schematic/define/src/oauth.rs:42` |
| `OAuth2Config.default_scopes` | `Vec<String>` | error | error | `schematic/define/src/oauth.rs:44` |
| `OAuth2Config.pkce` | `PkceRequirement` | error | error | `schematic/define/src/oauth.rs:46` |
| `OAuth2Config.client_auth` | `OAuth2ClientAuthMethod` | error | error | `schematic/define/src/oauth.rs:48` |
| `SchemaComponent.name` | `String` | error | error | `schematic/define/src/components.rs:18` |
| `SchemaComponent.rust_path` | `Option<String>` | None | None (presence lost) | `schematic/define/src/components.rs:22` |
| `SchemaComponent.schema` | `Value` | error | payload null | `schematic/define/src/components.rs:25` |
| `ModelCatalog.module_path` | `Option<String>` | no serde yet | no serde yet | `schematic/define/src/models/mod.rs:87` |
| `ModelCatalog.types` | `Vec<ModelDef>` | no serde yet | no serde yet | `schematic/define/src/models/mod.rs:89` |
| `FieldDef.name` | `String` | no serde yet | no serde yet | `schematic/define/src/models/structs.rs:28` |
| `FieldDef.serde_rename` | `Option<String>` | no serde yet | no serde yet | `schematic/define/src/models/structs.rs:32` |
| `FieldDef.description` | `Option<String>` | no serde yet | no serde yet | `schematic/define/src/models/structs.rs:34` |
| `FieldDef.required` | `bool` | no serde yet | no serde yet | `schematic/define/src/models/structs.rs:36` |
| `FieldDef.field_type` | `TypeRef` | no serde yet | no serde yet | `schematic/define/src/models/structs.rs:38` |
| `StructDef.name` | `String` | no serde yet | no serde yet | `schematic/define/src/models/structs.rs:72` |
| `StructDef.description` | `Option<String>` | no serde yet | no serde yet | `schematic/define/src/models/structs.rs:74` |
| `StructDef.fields` | `Vec<FieldDef>` | no serde yet | no serde yet | `schematic/define/src/models/structs.rs:76` |
| `StructDef.additional_properties` | `Option<TypeRef>` | no serde yet | no serde yet | `schematic/define/src/models/structs.rs:81` |
| `EnumVariant.name` | `String` | no serde yet | no serde yet | `schematic/define/src/models/enums.rs:29` |
| `EnumVariant.value` | `Option<String>` | no serde yet | no serde yet | `schematic/define/src/models/enums.rs:34` |
| `EnumVariant.description` | `Option<String>` | no serde yet | no serde yet | `schematic/define/src/models/enums.rs:36` |
| `EnumDef.name` | `String` | no serde yet | no serde yet | `schematic/define/src/models/enums.rs:72` |
| `EnumDef.description` | `Option<String>` | no serde yet | no serde yet | `schematic/define/src/models/enums.rs:74` |
| `EnumDef.variants` | `Vec<EnumVariant>` | no serde yet | no serde yet | `schematic/define/src/models/enums.rs:76` |
| `EnumDef.untagged` | `bool` | no serde yet | no serde yet | `schematic/define/src/models/enums.rs:81` |
| `TypeAlias.name` | `String` | no serde yet | no serde yet | `schematic/define/src/models/aliases.rs:25` |
| `TypeAlias.description` | `Option<String>` | no serde yet | no serde yet | `schematic/define/src/models/aliases.rs:27` |
| `TypeAlias.target` | `TypeRef` | no serde yet | no serde yet | `schematic/define/src/models/aliases.rs:29` |
| `FormField.name` | `String` | error | error | `schematic/define/src/request.rs:118` |
| `FormField.kind` | `FormFieldKind` | error | error | `schematic/define/src/request.rs:121` |
| `FormField.required` | `bool` | error | error | `schematic/define/src/request.rs:126` |
| `FormField.description` | `Option<String>` | None | None (presence lost) | `schematic/define/src/request.rs:129` |
| `Alternate.label` | `String` | error | error | `schematic/define/src/pagination/collection.rs:32` |
| `Alternate.strategy` | `Strategy` | error | error | `schematic/define/src/pagination/collection.rs:33` |
| `Collection.items` | `ItemSelector` | error | error | `schematic/define/src/pagination/collection.rs:59` |
| `Collection.item_type` | `Option<String>` | None | None (presence lost) | `schematic/define/src/pagination/collection.rs:60` |
| `Collection.pagination` | `Pagination` | error | error | `schematic/define/src/pagination/collection.rs:61` |
| `Collection.alternates` | `Vec<Alternate>` | error | error | `schematic/define/src/pagination/collection.rs:62` |
| `FixedParam.param` | `InputParam` | error | error | `schematic/define/src/pagination/strategy.rs:116` |
| `FixedParam.value` | `String` | error | error | `schematic/define/src/pagination/strategy.rs:117` |
| `PageSize.param` | `InputParam` | error | error | `schematic/define/src/pagination/strategy.rs:159` |
| `PageSize.server_default` | `Option<u32>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:160` |
| `PageSize.sent_default` | `Option<u32>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:161` |
| `PageSize.min` | `u32` | error | error | `schematic/define/src/pagination/strategy.rs:162` |
| `PageSize.max` | `Option<u32>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:163` |
| `PageSize.adjustable` | `bool` | error | error | `schematic/define/src/pagination/strategy.rs:164` |
| `CursorPart.param` | `InputParam` | error | error | `schematic/define/src/pagination/strategy.rs:318` |
| `CursorPart.source` | `CursorSource` | error | error | `schematic/define/src/pagination/strategy.rs:319` |
| `CursorLeg.parts` | `Vec<CursorPart>` | error | error | `schematic/define/src/pagination/strategy.rs:338` |
| `CursorLeg.boundary` | `Boundary` | error | error | `schematic/define/src/pagination/strategy.rs:339` |
| `PageNumberStrategy.page` | `InputParam` | error | error | `schematic/define/src/pagination/strategy.rs:422` |
| `PageNumberStrategy.base` | `IndexBase` | error | error | `schematic/define/src/pagination/strategy.rs:423` |
| `PageNumberStrategy.size` | `Option<PageSize>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:424` |
| `PageNumberStrategy.more` | `MoreSignal` | error | error | `schematic/define/src/pagination/strategy.rs:425` |
| `OffsetStrategy.offset` | `InputParam` | error | error | `schematic/define/src/pagination/strategy.rs:432` |
| `OffsetStrategy.base` | `IndexBase` | error | error | `schematic/define/src/pagination/strategy.rs:433` |
| `OffsetStrategy.size` | `Option<PageSize>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:434` |
| `OffsetStrategy.more` | `MoreSignal` | error | error | `schematic/define/src/pagination/strategy.rs:435` |
| `CursorStrategy.forward` | `CursorLeg` | error | error | `schematic/define/src/pagination/strategy.rs:446` |
| `CursorStrategy.backward` | `Option<CursorLeg>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:447` |
| `CursorStrategy.size` | `Option<PageSize>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:448` |
| `CursorStrategy.more` | `Option<MoreSignal>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:449` |
| `CursorStrategy.resumable` | `bool` | error | error | `schematic/define/src/pagination/strategy.rs:450` |
| `NextLinkStrategy.next` | `LinkSource` | error | error | `schematic/define/src/pagination/strategy.rs:459` |
| `NextLinkStrategy.prev` | `Option<LinkSource>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:460` |
| `NextLinkStrategy.size` | `Option<PageSize>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:461` |
| `FollowUp.method` | `Option<RestMethod>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:548` |
| `FollowUp.path` | `Option<String>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:550` |
| `FollowUp.carry` | `Carry` | error | error | `schematic/define/src/pagination/strategy.rs:551` |
| `SessionStep.method` | `RestMethod` | error | error | `schematic/define/src/pagination/strategy.rs:588` |
| `SessionStep.path` | `String` | error | error | `schematic/define/src/pagination/strategy.rs:590` |
| `SessionStep.handle` | `Option<InputParam>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:592` |
| `SessionStep.constants` | `Vec<FixedParam>` | error | error | `schematic/define/src/pagination/strategy.rs:593` |
| `KeepAlive.default` | `String` | error | error | `schematic/define/src/pagination/strategy.rs:638` |
| `KeepAlive.on_open` | `Option<InputParam>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:639` |
| `KeepAlive.on_page` | `Option<InputParam>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:640` |
| `KeepAlive.adjustable` | `bool` | error | error | `schematic/define/src/pagination/strategy.rs:641` |
| `Session.open` | `SessionOpen` | error | error | `schematic/define/src/pagination/strategy.rs:678` |
| `Session.pages` | `Option<(RestMethod, String)>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:681` |
| `Session.handle_from_page` | `Option<ValueSource>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:683` |
| `Session.send_handle_as` | `Option<InputParam>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:686` |
| `Session.keep_alive` | `Option<KeepAlive>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:687` |
| `Session.renew` | `Option<SessionStep>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:689` |
| `Session.close` | `Option<SessionStep>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:690` |
| `Partition.index` | `InputParam` | error | error | `schematic/define/src/pagination/strategy.rs:749` |
| `Partition.count` | `InputParam` | error | error | `schematic/define/src/pagination/strategy.rs:750` |
| `Partition.max` | `Option<u32>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:751` |
| `Strategy.kind` | `PageStrategy` | error | error | `schematic/define/src/pagination/strategy.rs:796` |
| `Strategy.constants` | `Vec<FixedParam>` | error | error | `schematic/define/src/pagination/strategy.rs:797` |
| `Strategy.total` | `Option<ValueSource>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:798` |
| `Strategy.terminal_values` | `Vec<String>` | error | error | `schematic/define/src/pagination/strategy.rs:801` |
| `Strategy.follow_up` | `Option<FollowUp>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:803` |
| `Strategy.session` | `Option<Session>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:804` |
| `Strategy.partition` | `Option<Partition>` | None | None (presence lost) | `schematic/define/src/pagination/strategy.rs:805` |
| `ItemSelector.record` | `Vec<Step>` | error | error | `schematic/define/src/pagination/selector.rs:108` |
| `ItemSelector.projection` | `JsonPath` | error | error | `schematic/define/src/pagination/selector.rs:110` |
