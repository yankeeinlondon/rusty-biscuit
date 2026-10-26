---
kind: notes
feature: 2026-10-18-schematic-fidelity-polish
phase: 1
created: 2026-10-23
---

# Phase 1 contract freeze

This is a snapshot taken on 2026-10-23 at HEAD `09de6ee35` on
`feat/better-schematics`, with the uncommitted
`2026-10-17-schematic-websocket` review-repair changes present in the working
tree. It freezes the `x-schematic` field mapping and the input-robustness
matrix columns that the plan marks as provisional. Every finding was read from
code. Nothing was built or run. Paths are relative to the repository root, and
`file:N` means line N of the current working tree. Features are named only by
`{date}-{name}`.

`schemas/kind.yaml` does not exist in this worktree, and the vocabulary
feature's `contracts.md` uses `kind: research`. This file uses `kind: notes`
because the orchestrator's brief asked for it.

## Prerequisite contracts verified

| Contract the plan assumes | Exists? | Exact symbol and location | Notes |
| --- | --- | --- | --- |
| Typed send `client.send(Request::new(..))` | Yes | `Client::send` at `schematic/runtime/src/client.rs:748`. The generated per-API `send` is emitted by the macro at `schematic/runtime/src/api_client.rs:177`, and `send_raw` is at `:185`. Generated `{Endpoint}Request::new` exists, for example at `schematic/schema/src/artificial_analysis.rs:810`. | **Differs from P10-g.** Every REST endpoint also gets a generated inherent method that returns a `{Endpoint}Call`. See `schematic/gen/src/codegen/endpoint_methods/mod.rs:1-27` and its use at `schematic/schema/src/artificial_analysis.rs:920-935` (`fn evaluate_crit_pt(..) -> EvaluateCritPtCall`). The methods come from `2026-10-12-schematic-endpoint-methods`, which is completed. See D1. |
| Serializable definitions | Yes | Plain derives on `RestApi` (`schematic/define/src/types.rs:104`), `Endpoint` (`:467`), `EndpointParams` (`schematic/define/src/params/mod.rs:83`) and `ParamDef` (`schematic/define/src/params/param_def.rs:25`). Presence helpers `non_null`, `map_only` and `non_null_map` are in `schematic/define/src/serde_presence.rs:20,30,53`. | Under the plain derives, absent and `null` are both `None` for most `Option` fields (`schematic/docs/definition-snapshots.md`, "Optional fields intentionally define both omission and explicit `null` as `None`"). The vocabulary fields use `non_null`, which rejects `null`. |
| JSON definition snapshots | Yes, but historical | 18 files in `schematic/gen/tests/fixtures/definition-baseline/`, each with `definition`, `fingerprints`, `pagination` and `registry`. Compared by `schematic/gen/tests/l1/definition_baseline.rs`. Helpers `snapshot`, `canonical` and `differences` are in `schematic/gen/examples/support/definition_snapshot.rs:5,35,47`. | The fixtures predate spec 4a (legacy `auth: "Basic"` and `env_username`) and are normalized before comparison. See D9. |
| Response model v2 | Yes | `ResponseShape` (`schematic/define/src/response.rs:24-45`), `StreamFraming::{Sse,Ndjson,Bytes}` (`:49-58`), `StreamEnd` (`:63-80`), `keep` (`:42-43`), `JsonTag` (`:84-97`), `SuccessShape` (`:114-139`). `Endpoint.response: Vec<SuccessShape>` is at `types.rs:490`. | `Bytes` framing and end-marker `keep` already travel inside `x-schematic.success`, as asserted in `schematic/define/tests/response_openapi.rs:441` and `schematic/define/src/openapi/extensions.rs:652`. |
| Error table | Yes | `StatusMatch` (`schematic/define/src/error_table.rs:30-40`), `ErrorEntry` (`:99-108`), `RestApi.errors` (`types.rs:211-212`), `Endpoint.errors` and `no_declared_errors` (`types.rs:527-532`). | Errors are written to the doc and op extension and to standard `responses` as the *effective* table (`schematic/define/src/openapi/export/paths.rs:192`). |
| GraphQL operations on `RestApi` | Yes | `RestApi.graphql: Option<GraphQlSurface>` (`types.rs:215-216`), `GraphQlSurface` (`schematic/define/src/graphql.rs:18-29`), `GraphQlOperation` (`:109-135`), `GraphQlVariables::{Unit,Typed}` (`:84-88`). | No shipped definition declares a GraphQL surface. A grep for `graphql` in `schematic/definitions/src` finds nothing. |
| WebSocket selectors and fallback | Yes | `MessageSelector { field: String, value: String }` with an object-only `Deserialize` (`schematic/define/src/websocket/lifecycle.rs:55-80`). `MessageSchema.selectors: Option<Vec<MessageSelector>>` (`:131-136`, `non_null`). `MessageSchema.fallback: bool` (`:139-140`). | A selector value is a **string only** (doc at `lifecycle.rs:40-41`). Validation in `schematic/gen/src/ws_codegen/plan.rs:585-612,859-865` rejects an empty `field` but accepts an empty `value`. |
| WebSocket role | Yes | `EndpointRole::{Client,Host,Both}` with `#[serde(rename_all = "lowercase")]` (`schematic/define/src/websocket/mod.rs:140-150`). `WebSocketEndpoint.role` has `#[serde(default)]`, so absent means `Client` (`:207-208`). | It serializes as `"client"`, `"host"` or `"both"`. |
| `HeartbeatHints` | Yes | `HeartbeatHints { interval_secs: u64, timeout_secs: Option<u64> }` (`schematic/define/src/websocket/hints.rs:173-186`). `timeout_secs` uses `non_null` and is skipped when `None`. It sits at `WebSocketEndpoint.runtime: Option<WebSocketEndpointHints>` (`mod.rs:214-219`) → `.heartbeat` (`hints.rs:282-287`, `non_null_map`). | The hint is per endpoint, so it maps to **channel** placement. A value of 0 is rejected at generation (`ws_codegen/plan.rs:895-907`), not by serde. |
| Partial generation `--api` | Yes | `Cli.api` (`schematic/gen/src/main.rs:39-40`). Tested by `schematic/gen/tests/l1/partial_generation.rs`. | |
| Staged import outputs | Partial | `MutationPlan` (`schematic/gen/src/output/publish.rs:68-70`). The standalone OpenAPI import publishes through `Planner` (`schematic/gen/src/output/mod.rs:663`). | The `--definitions-out` import writes with `write_atomic` directly (`schematic/gen/src/definition_gen.rs:54-57`, `schematic/gen/src/model_gen.rs:50`). The AsyncAPI import writes only a report (`schematic/gen/src/asyncapi_import.rs:690`). See D14. |
| Isolated compile check | Yes | `schematic/gen/tests/l1/fresh_output_compile.rs`, with the helper in `schematic/gen/tests/common/fresh_output.rs`. Run by `just test-fresh-output` (`schematic/justfile:73,81`). | These are `slow_` tests that invoke Cargo. |
| `OpenApiDiagnostic` and severities | Yes | `DiagnosticSeverity::{Info,Warn,Error}` (`schematic/define/src/openapi/import/diagnostics.rs:7-15`). `OpenApiDiagnostic { severity, location, message }` (`:46-54`). `is_error`, `is_warn` and `is_info` are at `:118,123,128`. | Existing locations are inconsistent: `"#/paths{path}"` is not escaped (`schematic/define/src/openapi/import/builder.rs:368`), and there are bare strings such as `"request body"` and `"parameter"` (`mappings/responses.rs:24`, `mappings/parameters.rs:17`). See D22. |
| `ImportOptions::strict` | Differs | The real type is `OpenApiImportOptions::strict` (`schematic/define/src/openapi/import/builder.rs:479-480`). The builder method is `OpenApiImport::strict` (`:140-148`). The gate at `:200` fails on **any** diagnostic, INFO included. The gen wrapper is `ImportOptions::strict` (`schematic/gen/src/import_pipeline.rs:47-48`, documented as "warning-level") and delegates at `:102-103`. AsyncAPI strict already checks only WARN and ERROR (`asyncapi_import.rs:152`). | R16 changes the builder gate. See D3. |
| Pagination types that remain | Yes | `PaginationStyle` is gone (only an authoring-guidance test still names it, at `schematic/define/tests/authoring_guidance.rs:23`). What remains is `pagination::{Strategy, PageStrategy::{PageNumber,Offset,Cursor,NextLink}, Collection, Pagination, Alternate, ItemSelector, …}` (`schematic/define/src/pagination.rs:30-56`; `strategy.rs:469-472`; `collection.rs:12-63`), plus `RestApi.default_pagination` (`types.rs:200`), `Endpoint.pagination` (`:509-510`) and `ParamDef.pager_managed` (`param_def.rs:35-37`). | **No OpenAPI export of pagination exists.** See D7. |
| `oauth_scopes` | Removed | Absent from all code under `schematic/define/src/` (including `openapi/extensions.rs`), `schematic/gen/src` and `schematic/definitions/src`. Only the removal note remains, in `schematic/docs/definition-builders.md:247`. | Scopes travel only through standard OAuth2 flow `scopes` (`schematic/define/src/openapi/export/security.rs:78-110`). |
| `auth_policy` builder | Yes | `RestApiBuilder::auth_policy` (`schematic/define/src/builders.rs:369`). On endpoints: `public` (`:259`), `auth` (`:262`), `auth_with_env_mapping` (`:264-266`). `AuthPolicy { explicit, env_fallback }` (`schematic/define/src/auth.rs:270-278`). `EndpointAuth::{Public, Policy { policy, env_mapping }}` uses `deny_unknown_fields` (`auth.rs:350-364`). | The OpenAPI exporter ignores `RestApi.auth_policy`. See D6. |
| `deprecated` | Yes | `Endpoint.deprecated` (`types.rs:519-520`), `ParamDef.deprecated` (`param_def.rs:38-41`), `EndpointBuilder::deprecated` (`builders.rs:256`). Exported as standard `deprecated` (`export/paths.rs:221`, `export/parameters.rs:56`). | The importer hard-codes `false` (`import/mappings/mod.rs:87`, `mappings/parameters.rs:95`). |
| Rate limit and retry | Yes | `RateLimit` with `try_from = RawRateLimit` and `deny_unknown_fields` (`schematic/define/src/metadata.rs:96-111,166-191`). `RetryGuidance` with `deny_unknown_fields` (`:283-295,330-353`). `StatusSelector` and `Backoff` are snake_case and `deny_unknown_fields` (`:197-204,231-240`). | **`retry_guidance` is already in both extensions**, but the plan does not list it. See D4. |
| `cross_origin` | Yes | `CrossOrigin` (`schematic/define/src/cross_origin.rs:39-52`). `RestApi.cross_origin` (`types.rs:188`). `Endpoint.cross_origin` (`:506-507`). The origin check is `validate_origin` (`builders.rs:613-624`). | |
| Request id | Yes | `RestApi.request_id_header` (`types.rs:191`). The builder lower-cases it (`builders.rs:377`). | |
| Secret markers | Yes | `RestApi.secret_names` (`types.rs:195`, builder `secret_header` at `builders.rs:378`). `ParamDef.secret` (`param_def.rs:32-34`, builder `secret` at `:165`). | **Neither is exported** to the extension or to standard fields. |
| End-marker `keep` and `Bytes` framing | Yes | See response model v2 above. | |

## Frozen x-schematic mapping

### Rules for the whole extension

- **Version.** `x-schematic.version` exists only at document level. It is an
  integer, and its only supported value is `1`. If it is absent, the document
  is major 1. Operation-level extensions never carry a version.
- **Writer.** The writer omits every optional or default member. It never
  writes `null`, except at the legacy positions listed under "Explicit null
  written by the v1 writer" below.
- **Reader.** The reader decodes each member independently from the
  `serde_json::Value` held in `openapiv3` `extensions`. That map keeps an
  `x-schematic: null` as `Some(Value::Null)`, so the reader can tell a null
  extension from a missing one.
- **Unknown members.** An unknown top-level member of either extension object
  produces INFO. An unknown nested member produces INFO where the upstream
  type tolerates unknown fields. It produces WARN where the upstream type uses
  `deny_unknown_fields`: `RateLimit`, `RetryGuidance`, `StatusSelector`,
  `Backoff`, `EndpointAuth` and the pagination types. It also produces WARN
  inside an `auth` entry, because no extra auth field is allowed there (R54).
- **Upstream validation.** A member is valid only if it passes the same checks
  as `validate_api` and `validate_endpoint_vocabulary`
  (`schematic/define/src/builders.rs:431-500`). A member that fails them
  produces WARN and is not applied (field-level rejection).
- **Standard wins (R12).** Where a standard field carries the same meaning, the
  standard value is used. A disagreeing extension value produces WARN with the
  extension's location.
- **Diagnostic locations.** New extension diagnostics use RFC 6901 pointers
  with the existing `#` prefix: `#/x-schematic/<member>[/<index-or-key>…]` or
  `#/paths/<escaped path>/<method>/x-schematic/<member>…`.
- **Sibling extensions.** These are not part of the subset and are decoded as
  follows:
  - The document-level `x-schematic-grouped-from: [api names]`
    (`schematic/gen/src/export/openapi.rs:157-160`) is read as INFO. It tells
    the reader that the document merges several APIs, so only the first
    member's metadata applies.
  - The operation-level `x-schematic-query-rust-names`
    (`schematic/define/src/openapi/import/query_paths.rs:70,92,110`, read at
    `import/builder.rs:384-389`) stays an internal normalization carrier. See
    R-S12.

### Document level (`x-schematic` at the OpenAPI root)

| Semantic field (Rust path) | Standard OpenAPI location | Carried by the extension today | Frozen v1 spelling and shape | Notes |
| --- | --- | --- | --- | --- |
| (new) document version | — | No | `version: 1` (integer) | Absent means 1. Any other value disables the whole subset with WARN (D). |
| `RestApi::name` | `info.title` | No | — | Standard. In grouped documents the title is the module name (`export/openapi.rs:129-130`), so members' names do not round-trip. See D8. |
| `RestApi::description` | `info.description` | No | — | Standard. |
| `RestApi::version` | `info.version` | No | — | Standard. The exporter writes `options.version → api.version → "1.0.0"` (`schematic/define/src/openapi/export/info.rs:7-11`). The importer must read `info.version`, which it currently does not (it sets `None` at `import/builder.rs:415`). The round-trip comparison treats `None` as equal to the exporter's default. |
| `RestApi::base_url` | `servers[0].url` | No | — | Standard. |
| `RestApi::docs_url` | `externalDocs.url` | No | — | Standard. |
| `RestApi::module_path` | — | `SchematicDocExtension.module_path: Option<String>`, skipped when `None` (`extensions.rs:51-52`) | `module_path: string` | Unchanged. The CLI `--module-path` option overrides it. |
| `RestApi::request_suffix` | — | `request_suffix: Option<String>`, skipped when `None` (`:55-56`) | `request_suffix: string` | Unchanged. The importer currently hard-codes the reserved-name suffix `"Request"` (`import/builder.rs:180`). After this change it uses the imported suffix. |
| `RestApi::env_mapping` | — | `env_mapping: Option<EnvMapping>`, skipped when `None` (`:59-60`) | `env_mapping: {bearer_token?, basic_user?, basic_pass?, api_key?, oauth_client_id?, oauth_client_secret?, oauth_redirect_uri?}`. Each `EnvList` is `{"names": [string]}` and `api_key` is `{"names": EnvList, "header": string}`. | The upstream `EnvMapping` (`schematic/define/src/headers/env.rs:148-160`) **serializes `null`** for `bearer_token`, `basic_user`, `basic_pass` and `api_key` (no `skip_serializing_if`, seen in `schematic/openapi/github.json`). v1 keeps that writer output, and the reader treats those nulls as "no mapping". |
| `RestApi::env_auth` | — | **No** | `env_auth: [string]` (new) | Today Gitea's `GITEA_TOKEN` is lost on export: `env_auth` is set and `env_mapping` is `None`. These are environment variable names, not credentials. See D5. |
| `RestApi::headers` | Header parameters, which Phase 3 adds for required constant headers | `headers: Vec<(String,String)>` with `default` and skipped when empty (`:63-64`) | `headers: [[name, value], …]` (2-element arrays, as serde writes the tuple) | Kept for artifact compatibility. Header-entry fields are positional: index 0 is the name, index 1 the value. Once Phase 3 also exports them as header parameters, the importer recognizes a header parameter with the same name (case-insensitive) and a single-value `enum` equal to the value as the fixed header. It does not create a `ParamDef` for it. |
| `RestApi::auth` / `auth_policy` (method, location, name, scopes) | `components.securitySchemes` and root `security` | No | — | Standard is authoritative (R54). The exporter must derive root `security` from `effective_auth_policy()` with one requirement object per accepted method. Today it uses only `api.auth` (`export/components.rs:16`, `export/security.rs:129-133`). The importer must read the *applied* root `security`, not the first component scheme (`import/auth.rs:17-36`). See D6. |
| Bearer header override (`AuthMethod::BearerToken { header: Some(h) }`, `auth.rs:96,210-212`) | `apiKey`, `in: header`, `name: h` (`export/security.rs:23-32`) | No | `auth: { "<securitySchemes key>": { "bearer": true } }` (new) | The marker restores `BearerToken { header: Some(name) }` from an `apiKey` header scheme. The header name stays in the standard `name`. |
| API-key value prefix (`AuthMethod::ApiKey { value_prefix }`, `auth.rs:116,219`) | — (dropped by `export/security.rs:45-53`) | No | `auth: { "<securitySchemes key>": { "value_prefix": string } }` (new) | Gitea uses `"token "` (`schematic/definitions/src/gitea/mod.rs:292`). An entry may not hold both `bearer` and `value_prefix`. Any other key in an entry produces WARN. |
| `RestApi::rate_limit` | — | `rate_limit: Option<RateLimit>`, `default`, skipped when `None` (`:66-68`) | `rate_limit: {limit_header, remaining_header, reset_header, retry_after_header?, reset_unit: "seconds"/"epoch_seconds"/"http_date"}` | This is runtime-only metadata. |
| `RestApi::retry_guidance` | — | `retry_guidance: Option<RetryGuidance>` (`:70-72`) | `retry_guidance: {retry_on: [{"code": u16} / {"range": {"start","end"}}], honor_retry_after: bool, max_attempts: u8, backoff: "none" / {"fixed": {"delay_ms"}} / {"exponential": {"base_ms","cap_ms"}}}` | **Not in the plan**, but it is in the subset (D4). |
| `RestApi::errors` | Standard `responses` of each operation, as the effective table | `errors: Vec<ErrorEntry>` (`:74-76`) | `errors: [{status: {"Exact": u16} / {"Range": {"lo","hi"}}, shape: ResponseShape (no Stream), content_type?}]` | The extension carries the API/endpoint decomposition. The reader recomputes `effective_errors` and compares it with what `classify_responses` derives from the standard responses. On a mismatch, standard wins with WARN and the standard-derived table is attached to the endpoint. |
| `RestApi::cross_origin` | — | `cross_origin: CrossOrigin`, `default`, skipped when `SameOrigin` (`:78-80`) | `cross_origin: "SameOrigin"` / `"AllowWithoutCredentials"` / `"DangerAllowAny"` / `"Deny"` / `{"AllowOrigins": [origin]}` | Absent means `SameOrigin`. |
| `RestApi::request_id_header` | — | `request_id_header: Option<String>` (`:82-84`) | `request_id_header: string` | The reader lower-cases it, matching the builder. The CLI `--request-id-header` (`import_pipeline.rs:43`) overrides it. |
| `RestApi::secret_names` | — | **No** | `secret_names: [string]` (new) | Secret marker (D5). |
| `RestApi::default_pagination` | — | No | — (not in v1) | Outside the subset. Listed as a difference (D7). |
| `RestApi::graphql` | — | No | — (never) | Skipped with a diagnostic (R55). |
| `RestApi::schemas[*].name` / `.schema` | `components.schemas` | No | — | Standard. Phase 2 recovers fidelity. |
| `RestApi::schemas[*].rust_path` | — | No. `SchematicSchemaExtension.rust_type` was meant for it but is never emitted. | — | The importer rebuilds it as `{module_path}::{Name}`. Listed as a difference. |
| OAuth2 `revocation_url`, `pkce`, `client_auth`, device-code grant | Partly in standard flows | No | — (never, per R54) | Listed as differences. No shipped REST definition uses OAuth2. |
| `SchematicSchemaExtension` (`extensions.rs:190-201`) | — | Never emitted or read | **Delete** | References to remove: the re-export at `schematic/define/src/openapi/mod.rs:27,89-91`, `extensions.rs:11`, the tests at `extensions.rs:405-452`, the `Default` allowlist in `schematic/define/tests/builders.rs:371-372`, `schematic/docs/definition-builders.md:244`, `schematic/docs/io/openapi-extensions.md:14,301-…` and `.claude/skills/schematic-define/schematic/openapi.md:214`. |
| `AuthExtension` (`extensions.rs:207-222`) | — | Never emitted or read | **Delete** once `auth.bearer` and `auth.value_prefix` land | References: the re-export at `openapi/mod.rs:90` and the tests at `extensions.rs:454-527,555-565`. Its `strategy` and `env_auth` would duplicate standard auth and the new `env_auth`. |

### Operation level (`x-schematic` on each operation)

| Semantic field (Rust path) | Standard OpenAPI location | Carried by the extension today | Frozen v1 spelling and shape | Notes |
| --- | --- | --- | --- | --- |
| `Endpoint::id` | `operationId` | No | — | Standard. Grouped documents rename clashing ids (`export/openapi.rs:136-141`). |
| `Endpoint::method` and `path` | Path-item key and method | No | — | Standard. |
| Reserved-expansion markers in `Endpoint::path` (`{+name}`) | — (stripped by `export/paths.rs:36-38,231-241`) | **No** | `path_template: string` (new) | Written only when the template contains a `{+…}` marker. Hugging Face has 16 such endpoints. Valid only if stripping its markers gives the OpenAPI path key; otherwise standard wins with WARN. |
| `Endpoint::description` | `summary` and `description` (both written) | No | — | Standard. The importer prefers `summary`. |
| `Endpoint::request`: JSON, Text, Binary | `requestBody.content` | `request: Option<ApiRequest>`, always written when present (`extensions.rs:125-127`) | `request` is **no longer written** for these variants | The standard content type carries them. A v1 document that still has them is compared with standard: a match is silent, a mismatch is WARN and standard wins. |
| `Endpoint::request`: FormData and UrlEncoded field detail | `multipart/form-data` or `application/x-www-form-urlencoded` object schema: `properties`, `required`, string `format: binary`, array items, `minItems`/`maxItems`, and `encoding.<field>.contentType` (Phase 4) | Same `request` member | `request: {"FormData": {"fields": [FormField]}}` or `{"UrlEncoded": {"fields": […]}}`. Each `FormField` is `{name, kind, required, description}`. `kind` is `"Text"`, `{"File": {"accept": [..]}}`, `{"Files": {"accept", "min", "max"}}` or `{"Json": TypeRef}`. | Multipart fidelity. Standard wins for the field set, `required` and descriptions. The extension fills only `accept` when `encoding` has no `contentType`, `min`/`max` when items have no bounds, and the `Json` part's `TypeRef`. The writer emits `description: null` and `min`/`max: null` (no `skip_serializing_if` at `schematic/define/src/request.rs:73-79,129`), and v1 keeps them. |
| `Endpoint::response`, first shape (legacy) | `responses` content | `response: ResponseShape`, a **required** field (`extensions.rs:129-131`) | `response: ResponseShape` (`{"Json": TypeRef}`, `"Text"`, `"Binary"`, `"Empty"` or `{"Stream": …}`) | Still written, so current artifacts are unchanged. On read it becomes optional: today an operation extension without `response` fails to decode as a whole (D18). `success` wins over `response` when both are present. |
| `Endpoint::response` (multi-shape, streams, selection facts) | Standard status and content; `oneOf` with a discriminator when shapes share a media type (`export/responses.rs:68-123`) | `success: Vec<SuccessShape>`, written unless there is one plain shape (`:147-151`, `export/paths.rs:203-206`) | `success: [{shape, content_type?, statuses?, json_tag?, fallback?, note?, variant?}]`. A stream shape is `{"Stream": {"framing": {"Sse": T} / {"Ndjson": T} / "Bytes", "end": {"DataLineEquals": s} / {"EventNamed": s} / {"FieldEquals": {"field","value"}} / "ConnectionClose", "keep"?: true}}`. | Response shapes, `Bytes` framing and end markers with `keep` are runtime-only semantics. Media types and statuses must agree with standard; otherwise standard wins with WARN. There is no multi-shape selector extension: one operation per `(method, path)`, per the author ruling of 2026-10-12. |
| `Endpoint::headers` | Header parameters (Phase 3) | `headers` (`:133-135`) | `headers: [[name, value], …]` | Same rules as the document-level `headers`. |
| `Endpoint::params` (query, header, cookie) | Operation and path-item `parameters` | No | — | Standard. The importer reads operation parameters only (`mappings/mod.rs:82`). R17 adds path-item merging. |
| `Endpoint::params.path` (typed path parameters) | Path parameters (`export/paths.rs:95-133`) | No | — | Standard, with ambiguity. An undeclared placeholder and a declared plain-string parameter export identically. The import rule is in D17. |
| `ParamDef::secret` | — | **No** | `params: {"<query/header/cookie/path>": {"<wire name>": {"secret": true}}}` (new) | Secret marker. The key must name a standard parameter at that location (header names compare case-insensitively); otherwise WARN. |
| `ParamDef::rust_name` | — | **No.** The importer's internal `x-schematic-query-rust-names` covers only `?` path normalization. | `params.<location>.<wire name>.rust_name: string` (new) | Validated with `valid_rust_name` (`builders.rs:605-611`). |
| `ParamDef::deprecated` | Parameter `deprecated` | No | — | Standard. The importer must read it. |
| `ParamDef::pager_managed` | — | No | — (not in v1) | Pagination, outside the subset (D7). |
| `Endpoint::rate_limit` | — | `rate_limit: Option<RateLimit>` (`:137-140`) | Same shape as the document level | Absent means inherit. `null` is WARN; today it is silently read as inherit. |
| `Endpoint::retry_guidance` | — | `retry_guidance` (`:142-145`) | Same shape as the document level | Same rules as `rate_limit`. |
| `Endpoint::errors` | Standard `responses` (effective) | `errors` (`:153-155`) | `errors: [ErrorEntry]` | See the document-level `errors` row. |
| `Endpoint::no_declared_errors` | — | `no_declared_errors: bool`, written only when true (`:157-159`) | `no_declared_errors: true` | |
| `Endpoint::cross_origin` | — | `cross_origin: Option<CrossOrigin>` (`:161-164`) | Same shape as the document level | Absent means inherit. `null` is WARN; today it is read as inherit. |
| `Endpoint::deprecated` | Operation `deprecated` (`export/paths.rs:221`) | No | — (never) | Standard only. The importer must read `op.deprecated`, which is hard-coded `false` at `mappings/mod.rs:87`. |
| `Endpoint::auth` (`Public` or `Policy.policy`) | Operation `security`: `[]` for public, one requirement per accepted method for a policy (`export/security.rs:172-190`) | No | — | Standard. The importer must map operation `security`; today it always sets `auth: None` (`mappings/mod.rs:87`). |
| `Endpoint::auth` `Policy.env_mapping` | — | **No** | `env_mapping: EnvMapping` (new, operation level) | Valid only when the operation's `security` replaces the root's; otherwise WARN. This is a judgment call (D5). |
| Bearer override and value prefix of an override method | Override schemes in components | No | `auth: { "<scheme key>": {"bearer": true} / {"value_prefix": s} }` (new) | Applies only to schemes named by **this operation's** `security`. Document-level details never apply to operation overrides, which keeps them separate when `register_scheme` (`export/security.rs:198-215`) reuses a component for an identical scheme. |
| `Endpoint::pagination` | — | No | — (not in v1) | Outside the subset (D7). |
| `SchematicOpExtension` as a whole | — | Required `response`. Plain `Option`s conflate `null` with absent. | Replaced by a presence-aware decoder over the frozen keys | |

### Explicit null written by the v1 writer

The plan's default is that an explicit `null` is WARN. These positions are
exceptions, because the v1 writer itself emits `null` there. At these
positions `null` means O (none):

- `env_mapping.{bearer_token, basic_user, basic_pass, api_key}`, at both levels
- `request.*.fields[*].description`
- `request.*.fields[*].kind.Files.{min, max}`

At every other position `null` is W.

## Input matrices (frozen columns)

The codes are the plan's: E, W, I, O, V, D. A cell is listed only when it
differs from the plan's default row:

- Absent is O.
- Explicit null, wrong whole type, one wrong element, all wrong elements, and
  every wrong sub-field are W, and a W rejects the whole field.
- Empty is W unless listed.
- A duplicate key or trailing or invalid content is E. This is enforced today
  by `UniqueValue` in `check_text` (`import/query_paths.rs:281-286,420-444`)
  for every text source. `OpenApiSource::Document` skips it.
- Every column applies to JSON and to YAML.
- "Field" means the top-level member that the cell rejects. Nested
  sub-columns reject their parent member.

### 1. OpenAPI `x-schematic` fields

| Column | Absent | Explicit null | Wrong type / wrong element | Empty | Other exceptions |
| --- | --- | --- | --- | --- | --- |
| `x-schematic` (the object itself) | O (no metadata) | D | D (a non-object) | V: `{}` is major 1 with no metadata | — |
| `version` | I: major 1 | D | D for a string `"1"`, a fractional `1.0`, a bool, or an array | D (`""`) | An integer other than 1 is D. |
| `module_path` | — | — | — | W (`""`) | — |
| `request_suffix` | — | — | — | W | An invalid identifier suffix is W (generation's `invalid-request-suffix`). |
| `env_mapping` | — | — | — | V: `{}` means no mappings | An unknown sub-key is INFO. A mapping that conflicts with the auth declaration (`builders.rs:440-447`) is W. |
| ↳ `.bearer_token`, `.basic_user`, `.basic_pass` | O | **O** (writer emits null) | W | V: `{"names": []}` | — |
| ↳ `.oauth_client_id`, `.oauth_client_secret`, `.oauth_redirect_uri` | O | W | W | V | — |
| ↳ `EnvList.names` | W | W | W per element | V (`[]`) | An element `""` is W. |
| ↳ `.api_key` | O | **O** | W | W (`{}`: required members missing) | — |
| ↳ `.api_key.header` | W | W | W | W (`""`) | — |
| ↳ `.api_key.names` | W | W | W | V | — |
| `env_auth` (new) | O: `[]` | W | W per element | V | An element `""` is W. |
| `headers` (API) | O: no extra headers | W | W for a non-array, an entry that is not a 2-array, or a non-string element | V: no extra headers | — |
| ↳ entry `[0]` name | W | W | W | W (`""` or invalid token) | A case-insensitive duplicate name in the list is W. |
| ↳ entry `[1]` value | W | W | W | V (`""` is a legal header value) | — |
| `auth` (new) | O | W | W | V (`{}`) | A key that names no scheme, or a scheme that is not `apiKey` with `in: header` applied by root `security`, is W and standard wins. |
| ↳ `<scheme>.bearer` | O | W | W (only `true` is valid; `false` is W) | — | `bearer` together with `value_prefix` is W. Any other key in the entry is W. |
| ↳ `<scheme>.value_prefix` | O | W | W | **V** (`""`) | — |
| `rate_limit` | O | W | W | W (`{}` is missing required members) | An unknown sub-key is W (`deny_unknown_fields`, `metadata.rs:167`). |
| ↳ `.limit_header`, `.remaining_header`, `.reset_header` | W | W | W | W | An invalid header name is W (`RateLimit::validate`). |
| ↳ `.retry_after_header` | O | W (`non_null`, `metadata.rs:172`) | W | W | — |
| ↳ `.reset_unit` | W | W | W, including an unknown enum value | W | — |
| `retry_guidance` | O | W | W | W (`{}`) | An unknown sub-key is W. |
| ↳ `.retry_on` | W | W | W per element | V (`[]`) | — |
| ↳ `.retry_on[*]` `{"code"}` / `{"range": {"start","end"}}` | W | W | W | W | An out-of-range status or a reversed range is W (`metadata.rs:205-219`). |
| ↳ `.honor_retry_after` | W | W | W, including a quoted `"true"` | — | — |
| ↳ `.max_attempts` | W | W | W, including a value > 255 | — | `0` is W (`ZeroMaxAttempts`). |
| ↳ `.backoff` | W | W | W | W | `"none"`, `{"fixed": {"delay_ms"}}` or `{"exponential": {"base_ms","cap_ms"}}`. A zero delay is W, and `cap_ms < base_ms` is W. |
| `errors` (API) | O | W | W per entry | V: no API defaults | A mismatch with standard responses is WARN and standard wins (R12). |
| ↳ `[*].status` | W | W | W | W | An `Exact` or `Range` outside 400–599, or a reversed range, is W (`error_table.rs:72-79`). |
| ↳ `[*].shape` | W | W | W | W | A `Stream` shape is W (streams are not allowed on the error path). |
| ↳ `[*].content_type` | O: shape default | W (`non_null`) | W | W | An invalid media type is W. |
| `cross_origin` (API) | I: `SameOrigin` | W | W, including an unknown variant | W (`""`) | `{"AllowOrigins": []}` is V. An origin that fails `validate_origin` (`builders.rs:613-624`) is W. |
| `request_id_header` | — | — | — | W | An invalid header name is W. Upper case is normalized silently. |
| `secret_names` (new) | O | W | W per element | V | An element `""` or an invalid header name is W. |
| **Operation** `request` | O | W | W, including an unknown variant | W (`{}`) | A conflict with the standard body is WARN and standard wins. Legacy `Json`, `Text` and `Binary` values are compared with standard and are not an error. |
| ↳ `.fields` | W | W | W per element | W (`[]`: a form with no fields is omitted upstream, `responses.rs:131-136`) | — |
| ↳ `.fields[*].name` | W | W | W | W | A name that does not match a standard property is WARN and standard wins. |
| ↳ `.fields[*].kind` | W | W | W | W | `File` on a URL-encoded body is W. |
| ↳ `.fields[*].kind.File.accept` / `.Files.accept` | W (required upstream) | W | W per element | V: any type | Standard `encoding.<name>.contentType` wins. |
| ↳ `.fields[*].kind.Files.min` / `.max` | O | **O** (writer emits null) | W | — | `min > max` is W. Standard `minItems`/`maxItems` win. |
| ↳ `.fields[*].kind.Json` (`TypeRef`) | W | W | W | — | An unresolved `Named` is ERROR (reference closure). |
| ↳ `.fields[*].required` | W | W | W | — | A conflict with standard `required` is WARN and standard wins. |
| ↳ `.fields[*].description` | O | **O** | W | V | Standard `description` wins. |
| `response` (legacy) | **O** (today: decode failure) | W | W | W | — |
| `success` | O: standard-derived shapes | W | W per element | W (`[]` is never written; `Endpoint.response` must be nonempty) | A media type or status that disagrees with standard is WARN and standard wins. |
| ↳ `[*].shape` | W | W | W | W | — |
| ↳ `.shape.Stream.framing` | W | W | W, including an unknown variant | W | — |
| ↳ `.shape.Stream.framing.Sse` / `.Ndjson` (`TypeRef`) | W | W | W | — | An unresolved reference is ERROR. |
| ↳ `.shape.Stream.end` | W | W | W | W | `Bytes` framing with an end other than `ConnectionClose` is W. |
| ↳ `.end.DataLineEquals` | W | W | W | **V** (`""` is a legitimate terminator, `response.rs:65-67`) | — |
| ↳ `.end.EventNamed` | W | W | W | W | — |
| ↳ `.end.FieldEquals.field` / `.value` | W | W | W (`value` must be a string, number or bool) | W for `field` | — |
| ↳ `.shape.Stream.keep` | I: false | W | W | — | — |
| ↳ `[*].content_type` | O | W | W | W | — |
| ↳ `[*].statuses` | O | W | W per element (each must be 200–299) | V: none declared | — |
| ↳ `[*].json_tag` | O | W | W | W | `Field.value` must be a string, number or bool. `"Array"` and `"Object"` are valid. |
| ↳ `[*].fallback` | I: false | W | W | — | More than one fallback is W. |
| ↳ `[*].note` | O | W | W | V | — |
| ↳ `[*].variant` | O | W | W | W | An invalid identifier is W. |
| `headers` (operation) | O: no override | W | W, as for API headers | V: no override | Same entry sub-columns as the API headers. |
| `rate_limit`, `retry_guidance` (operation) | I: inherit | **W** (today: inherit) | W | W | Same sub-columns as the document level. |
| `errors` (operation) | O | W | W per entry | V | Same sub-columns as the document level. |
| `no_declared_errors` | I: false | W | W | — | — |
| `cross_origin` (operation) | I: inherit | **W** (today: inherit) | W | W | Same as the document level. |
| `auth` (operation) | O | W | W | V | W unless operation `security` replaces the root's. Same entry sub-columns as the document level. |
| `env_mapping` (operation) | O: the API mapping | W | W | V | W unless the operation has a `security` policy override. Same sub-columns as the document level. |
| `path_template` | O | W | W | W | If stripping its markers does not give the path key, W and standard wins. |
| `params` | O | W | W | V | An unknown location key is INFO. |
| ↳ `<loc>.<name>` | — | W | W | W (`{}`) | A name with no standard parameter at that location is W. An unknown key is INFO. |
| ↳ `.secret` | I: false | W | W | — | — |
| ↳ `.rust_name` | O | W | W | W | An invalid Rust identifier is W. |

The version controls also cover these cases:

- **Unsupported major.** An integer major such as `2` is D.
- **Fractional, string or null version.** Each is D.
- **Unknown top-level key.** INFO, while the other members still apply.
- **Malformed member beside a valid sibling.** W for the malformed member
  only.

### 2. OpenAPI standard fields changed by reconciliation

These columns add to the plan's table, whose cells stand. Defaults: Explicit
null, wrong type, wrong element, duplicate and trailing content are E.

| Column | Absent | Empty | Notes |
| --- | --- | --- | --- |
| `paths.<p>.parameters` (path item) | I: none inherited | V | R17 merges these into every operation. Identity is `(name, in)`, and header names compare case-insensitively. |
| `<op>.parameters` | I: inherit the path item's | V | An operation parameter overrides a path-item one with the same identity. |
| `parameter.name` | E | E | — |
| `parameter.in` | E | — | An unknown location is E. |
| `parameter.required` | I: false (`in: path` must be true, else E) | — | — |
| `parameter.$ref` | inline | E (`""`) | A local `#/components/parameters/X` is resolved before the merge. An unresolved one is ERROR. A remote one is ERROR without a fetch. |
| `parameter.schema.type` / `.format` / `.items` / `.enum` | I: `String` (`mappings/parameters.rs:89`) | — | An unsupported shape is WARN today. A typed path parameter is decided per D17. |
| `parameter.style` / `.explode` | I: the OpenAPI default per location (`mappings/parameters.rs:84-86`) | — | — |
| `parameter.deprecated` | I: false | — | Must be read (today `false`). |
| `parameter.description` | O | V | — |
| `requestBody` | I: no body | — | An absent `requestBody` is not the same as a present one with absent `content`, which is E. |
| `requestBody.content` | E | E | — |
| multipart `schema.properties` | I: unconstrained, unsupported parts diagnosed | V: no parts, WARN as today (`responses.rs:131-136`) | — |
| multipart `properties.<p>.type` / `.format` | I: text part | — | String with `format: binary` (or `base64`) is a file. An array of binary is `Files`. An **object-valued part is WARN**; today it is silently `Text` (`responses.rs:219`). |
| multipart `properties.<p>.items` | — | — | For an array, items that are not binary strings are WARN (today silently `Text`). |
| multipart `properties.<p>.minItems` / `.maxItems` | O | — | Maps to `Files.min` / `Files.max`. |
| multipart `required` | I: all optional | V | — |
| `encoding.<p>.contentType` | I: the media default | E (`""`) | Maps to `accept` (comma-split), or `application/json` to the `Json` kind. `encoding` for an unknown part is WARN. |
| `encoding.<p>.headers` / `.style` / `.explode` / `.allowReserved` | O | — | Unrepresentable, so WARN when present. |
| `<op>.security` | I: root security | V: public (`EndpointAuth::Public`) | A list of requirement objects becomes `EndpointAuth::Policy` with one method each. A requirement with more than one scheme (AND) is WARN because it cannot be represented. |
| root `security` | I: the first scheme (legacy) **only when no root `security` exists**. With `security` present, the applied requirements are authoritative. | V: anonymous | — |
| `securitySchemes.<n>.type` / `.in` / `.name` / `.scheme` | E | E | Unsupported schemes are WARN (`mappings/mod.rs:110-189`). |
| `securitySchemes.<n>.flows.<flow>.scopes` | E (OpenAPI requires the map) | V: no default scopes | Maps to `default_scopes`. |
| `<op>.deprecated` | I: false | — | Must be read. |
| `info.version` | E (required) | E | Maps to `RestApi::version`. |
| `servers[0].url` | I: `https://api.example.com` with WARN. This changes today's silent default (`mappings/mod.rs:38`). | E | — |
| `externalDocs.url` | O | E | — |

### 3. AsyncAPI standard fields (2.x vs 3.0)

These cells add to or override the plan's table. Defaults are as in the
plan's table.

| Column | 2.x | 3.0 | Absent | Empty | Notes |
| --- | --- | --- | --- | --- | --- |
| `asyncapi` | `2.x.y` | `3.0.y` | **E** (today ERROR that does not abort, `asyncapi_import.rs:239-246`) | E | Any other version is E. Today no version is checked. |
| `info.title` | ✓ | ✓ | E unless `--api-name` is given | E | Maps to `WebSocketApi::name`. |
| `info.version` | ✓ | ✓ | E | E | Maps to `WebSocketApi::version`. |
| `info.description` | ✓ | ✓ | O (today a placeholder string) | V | — |
| `externalDocs.url` | ✓ | ✓ | O | E | Maps to `docs_url`. |
| `servers.<n>.url` | ✓ | — | W: no server | E | — |
| `servers.<n>.host` / `.pathname` / `.protocol` | — | ✓ | W: no server. `host` absent is E. | E (`host`) | Assembles `base_url` with no duplicated base-path segment. `protocol` must be `ws` or `wss`; anything else is E. |
| `servers.<n>.security` | ✓ | ✓ | I: no auth | V: anonymous | The applied security is authoritative, not the first component scheme (`asyncapi_import.rs:281-354`). |
| `channels` | key = address | key = id, `.address` = path | E | V: zero endpoints with WARN | — |
| `channels.<c>.address` | — | ✓ | E | E | Maps to `WebSocketEndpoint::path`. |
| `channels.<c>.parameters.<p>` | ✓ | ✓ | I: untyped placeholders | V | Maps to `params.path`. |
| `channels.<c>.bindings.ws.query` | ✓ | ✓ | O | V | An object schema. Maps to `params.query` (required set from `required`). |
| `channels.<c>.bindings.ws.headers` | ✓ | ✓ | O | V | Constant (single-value `enum`/`const`) properties map to `WebSocketApi::headers`. |
| `channels.<c>.publish` / `.subscribe` | ✓ | — | W: no messages | — | Direction follows `--ws-role` (legacy). |
| `operations.<o>.action` | — | ✓ | E | E | `send` maps to `Client` and `receive` to `Server`. A message under both is `Bidirectional`. The role never reverses directions. |
| `operations.<o>.channel` / `.messages` | — | ✓ | E | V: no messages with WARN | `$ref` must resolve locally. |
| `operations.<o>.reply` | — | ✓ | O | — | A reply on a `send` operation maps to `responds_with` (D11). |
| `components.messages.<m>` | ✓ | ✓ | O | V | — |
| ↳ `.name` | O: the key | O: the key | — | E | Maps to `MessageSchema::name`. |
| ↳ `.payload` | ✓ | ✓ | W: no payload | V only for `{}` | `$ref` to `components.schemas` maps to `Schema::type_name` (D12). |
| `components.schemas` | ✓ | ✓ | O | V | — |
| `components.securitySchemes.<n>.type` / `.in` / `.name` / `.scheme` | ✓ | ✓ | E | E | `httpApiKey` (`in: query`/`header`), `http` bearer/basic and `apiKey`. Others are WARN. |

### 4. AsyncAPI Schematic metadata

| Column | Placement | Shape (upstream serde) | Absent | Explicit null | Wrong type / element | Empty | Other |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `x-schematic-selectors` | The message object, normally `components.messages.<m>` (the exporter always emits component messages) | `[{field, value}]`, from `MessageSelector` (`lifecycle.rs:55-80`) | **O: standard-derived.** Selectors inferred from payload string `const` properties (`asyncapi_import.rs:472-509`) are kept for both versions. | W | W rejects the whole list | V: `Some([])` as declared, and the `const` inference is suppressed | — |
| ↳ `field` | — | string | W | W | W | W (`""` is rejected at generation, `ws_codegen/plan.rs:859-865`) | — |
| ↳ `value` | — | string only | W | **W** (upstream does not support null) | W for a number, bool, object or array | **V** (`""` is accepted upstream) | An unknown key in a selector object is INFO. |
| `x-schematic-fallback` | Message object | `true` | I: false | W | W | W (`""`) | `false` is V. Fallback together with nonempty selectors is W. More than one fallback in a direction is W, and no message in that direction is given fallback. |
| `x-schematic-role` | Channel | `"client"` / `"host"` / `"both"` (`mod.rs:140`) | I: `Client`, or the `--ws-role` value on that channel | W | W, including an unknown string or a different case | W | Never changes operation directions. |
| `x-schematic-heartbeat` | Channel (`WebSocketEndpoint.runtime.heartbeat`) | `{interval_secs: u64, timeout_secs?: u64}` (`hints.rs:173-186`) | O | W | W | W (`{}`: `interval_secs` is required) | An unknown sub-key is INFO. |
| ↳ `interval_secs` | — | u64 | W | W | W, including a negative or fractional value | — | `0` is W. |
| ↳ `timeout_secs` | — | u64 | O | W (`non_null`) | W | — | `0` is W. |

## Reader smells

Each pattern below appears in code that later phases change. "LB" means the
use is load-bearing: absent, null and wrong type must be told apart there.

| Location | Pattern | LB? | Disposition |
| --- | --- | --- | --- |
| `schematic/define/src/openapi/extensions.rs:51-84` (doc ext) | `Option<T>`, `#[serde(default)]` on `headers`, `rate_limit`, `retry_guidance`, `errors`, `cross_origin`, `request_id_header` | Yes | Absent and null are conflated. `CrossOrigin`'s `default` makes `"cross_origin": null` a whole-struct error. Replace with a presence-aware member decoder (R-S1). |
| `extensions.rs:125-164` (op ext) | Required `response`, `Option` on `rate_limit`, `retry_guidance`, `cross_origin` | Yes | `null` reads as inherit, and a missing `response` fails the whole struct (D18). |
| `extensions.rs:214` | `#[serde(default)] env_auth` on `AuthExtension` | No | Deleted with the struct. |
| `schematic/define/src/headers/env.rs:150-159` | Plain `Option<EnvList>` / `Option<ApiKeyEnv>` with no `non_null` | Yes | The writer emits null, so null is O at these four positions. The OAuth fields use `default` and skip; null there is W through the presence-aware decoder. |
| `schematic/define/src/request.rs:76-79,129` | `Option<u32>` `min`/`max`, `Option<String>` `description`, written as null | Yes | Null is O (written by v1). |
| `schematic/define/src/openapi/import/builder.rs:200` | `!diagnostics.is_empty()` strict gate | Yes | R16: WARN or ERROR only. |
| `import/builder.rs:384-389` | `.and_then(Value::as_object)` and `.and_then(Value::as_str)` on `x-schematic-query-rust-names` | Yes | A malformed value is silently ignored, and the key is accepted from any source document. See R-S12. |
| `import/builder.rs:403-421` | `RestApi` built with every vocabulary field defaulted | Yes | This is where Phase 4 applies the decoded extension. |
| `import/mappings/mod.rs:87` | `deprecated: false, auth: None` hard-coded | Yes | Read the standard fields. |
| `import/mappings/mod.rs:35-39` | `servers.first() … unwrap_or_else("https://api.example.com")` | Yes | Silent default. Add WARN (matrix 2). |
| `import/mappings/parameters.rs:33,77` | Path parameters dropped | Yes | D17. |
| `import/mappings/parameters.rs:95` | `deprecated: false, secret: false` hard-coded | Yes | Read the standard `deprecated`, and `params.*.secret` from the extension. |
| `import/mappings/parameters.rs:145,181` | `filter_map(\|v\| v.clone())` over `enumeration` | Yes | A `null` enum member is dropped silently. It should be WARN, or kept where null is allowed. |
| `import/mappings/responses.rs:29` (`None => return None`) and `:58-73` | An unsupported media type falls through to `None` | Yes | Absent and unsupported content are conflated. Diagnose. |
| `import/mappings/responses.rs:187-189` | `let Ok(schema) = … else { return Text }` | Yes | An unresolved part `$ref` silently becomes text. It should be ERROR. |
| `import/mappings/responses.rs:203` | `.ok()` on resolving array items | Yes | An unresolved items ref silently becomes `Text`. |
| `import/mappings/responses.rs:239` | `.ok()` on the description | No | The description is cosmetic. Keep it, with justification. |
| `import/mappings/discriminator.rs:46` | `unwrap_or_default()` on discriminator mapping values | No | Missing mapping falls back to tag inference, which is documented. Keep. |
| `import/query_paths.rs:31,84` | `.and_then(Value::as_array).cloned().unwrap_or_default()` on `parameters` | Yes | A non-array `parameters` silently becomes empty. `check_text` already rejects a non-array at path-item and operation level (`:292,301`), so this is safe only behind that check. |
| `import/query_paths.rs:364` | `unwrap_or_default()` on a split | No | `split` always yields at least one item. |
| `import/auth.rs:17-36` | The first supported component scheme wins | Yes | It ignores the applied `security` (matrix 2). |
| `schematic/gen/src/asyncapi_import.rs:87-91,107-129` | `.and_then(Value::as_object).cloned().unwrap_or_default()` for `info`, `channels`, `components`, `messages`, `schemas` | Yes | A wrong type silently becomes empty. It must be E. |
| `asyncapi_import.rs:93-97,165-169,399-400` | `.and_then(Value::as_str).unwrap_or(..)` for the title, description and channel description | Yes | A wrong type silently becomes a placeholder. |
| `asyncapi_import.rs:239-253` | A missing `asyncapi` or `channels` is ERROR but does not abort | Yes | E per matrix 3, and no version check exists. |
| `asyncapi_import.rs:256-279` | `servers.into_iter().next()` reads 2.x `url` only, then falls back to `ws://localhost` | Yes | No 3.0 `host`/`pathname`. Becomes W per matrix 3. |
| `asyncapi_import.rs:288,291,296-297,308` | `unwrap_or_default()` / `unwrap_or("")` / `unwrap_or("X-API-Key")` on security schemes | Yes | A missing `name` silently becomes `X-API-Key`. It must be E. |
| `asyncapi_import.rs:500` | `.and_then(Value::as_str)` on `const` | Yes | A non-string `const` is silently not a selector. That is correct for the upstream string domain, but it should be INFO. |
| `asyncapi_import.rs:220-236` | `serde_json::from_str::<Value>` for JSON, and `Yaml` then `as_json` | Yes | **Duplicate keys are last-wins for JSON**, against the plan's rule. Detection is `trim_start().starts_with('{')`. Reuse the `UniqueValue` approach. |
| `asyncapi_import.rs:772-790` | `map_direction` reverses directions for `WsRole::Server` | Yes | It conflicts with "role does not reverse directions". Keep it for 2.x only (D13). |
| `schematic/gen/src/import_pipeline.rs:47-48` | The `strict` doc says "warning-level" while the builder fails on INFO | No (doc) | It becomes accurate after R16. |
| `import_pipeline.rs:120` | `request_id_header.clone_from(&options.request_id_header)` | Yes | It overwrites an imported `request_id_header` with `None` when the flag is absent. After Phase 4 it must override only when the flag is set. |

Rules adopted for Phase 4 and Phase 10 readers:

- **R-S1.** Decode `x-schematic` members from `Value`, key by key. Never
  derive `Deserialize` on the whole extension struct.
- **R-S12.** `x-schematic-query-rust-names` stays an internal carrier that the
  `?` normalization (`query_paths.rs:70`) writes into the parsed document. It
  is not part of the public subset. `params.<loc>.<name>.rust_name` is the
  public spelling. When both are present on one operation, the public spelling
  wins with WARN. Leaving the internal key readable from source documents is
  logged as a known gap, not fixed here.

## AsyncAPI model

The importer today accepts any `asyncapi` string; it requires only that the
field is present, and missing it is a non-aborting ERROR
(`asyncapi_import.rs:239-246`). It reads only 2.x structures:

- `servers.*.url` (`:256-279`)
- `channels.<path>.publish` / `.subscribe` (`:358-410`)
- `oneOf` message variants (`:511-516`)
- selectors inferred from payload string `const` (`:472-509`)

The role comes from the `--ws-role` CLI option (`:763-790`,
`schematic/gen/src/main.rs:167-168`). It does not read `x-schematic-*`,
`fallback`, heartbeat or any 3.0 field. Every fixture and vendored document
is 2.2.0: `schematic/gen/tests/fixtures/asyncapi/UCR-integration-asyncapi.json`
and `schematic/specs/unfolded-circle/{UCR-core,UCD2,UCR-integration}-asyncapi.yaml`.
No 3.0 handling exists. The import returns `WebSocketApi` and `ModelCatalog`
(`:56-72`) and persists only a report (`:690-729`).

The table below lists every field of the `WebSocketApi` model that an
exporter must write, or explicitly leave out.

| Rust path | 3.0 target | In the plan's subset? |
| --- | --- | --- |
| `WebSocketApi::name` (`schematic/define/src/websocket/mod.rs:261`) | `info.title` | Yes |
| `::description` (`:263`) | `info.description` | Yes |
| `::version` (`:299-300`) | `info.version` (default `"1.0.0"`) | Yes |
| `::base_url` (`:268`) | `servers.<n>.{protocol, host, pathname}` | Yes |
| `::docs_url` (`:270`) | `externalDocs.url` | Yes (info) |
| `::auth` / `::auth_policy` (`:272,275-280`) | `components.securitySchemes`, `servers.<n>.security` | Yes |
| `::env_auth` / `::env_mapping` (`:283,287-292`) | — | **No.** Not listed (D11). |
| `::headers` (`:294`) | `channels.<c>.bindings.ws.headers`, as single-value `enum` properties | Yes (WS headers binding) |
| `::endpoints[*].id` (`:195`) | Channel key | Yes |
| `::endpoints[*].path` (`:198`) | `channels.<c>.address` | Yes |
| `::endpoints[*].description` (`:200`) | `channels.<c>.description` | Yes |
| `::endpoints[*].params.path` / `.query` (`:204-205`) | `channels.<c>.parameters` / `bindings.ws.query` | Yes |
| `::endpoints[*].role` (`:207-208`) | `channels.<c>.x-schematic-role` | Yes |
| `::endpoints[*].messages[*].name` / `.direction` / `.description` (`lifecycle.rs:110-118`) | `components.messages.<key>` with `name`, and `operations` `send`/`receive` | Yes |
| `::messages[*].schema` (`Schema { type_name, module_path }`, `schematic/define/src/schema.rs:113-118`) | `payload: {$ref: '#/components/schemas/<type_name>'}` | Yes, but **there is no schema source** (D12). `module_path` is not carried. |
| `::messages[*].selectors` / `.fallback` (`lifecycle.rs:131-140`) | `x-schematic-selectors` / `x-schematic-fallback` on the component message | Yes |
| `::messages[*].responds_with` (`:122-127`) | `operations.<send>.reply.messages` (standard 3.0) | Not listed. Adopted as standard (D11). |
| `::endpoints[*].lifecycle.{open, close, keepalive}` (`lifecycle.rs:217-226`) | Ordinary messages in the operations | Partial. The lifecycle role is lost (D11). |
| `::lifecycle.open_credential_field` (`:233-238`) | — | **No** (D11) |
| `::endpoints[*].runtime.heartbeat` (`hints.rs:282-287`) | `channels.<c>.x-schematic-heartbeat` | Yes |
| `::endpoints[*].runtime.correlation` (`hints.rs:268-273`) | — | **Excluded** by the plan. Now used by three definitions (D11). |
| `::endpoints[*].runtime.auth_flow` (`hints.rs:275-280`) | — | **No.** Not listed. Used by three definitions (D11). |
| `::runtime` (`WebSocketRuntimeHints { frame_format, supports_reconnect, request_id_type }`, `hints.rs:216-227`) | — | **No.** Not listed (D11). |

Proposed component naming for Phase 10: message key `{EndpointId}.{MessageName}`,
which is unique per document when endpoints reuse a message name, with `name`
set to the message name. Schema key `<type_name>`.

## GraphQL

- **Type.** `RestApi.graphql: Option<GraphQlSurface>`
  (`schematic/define/src/types.rs:215-216`). `GraphQlSurface { address:
  Vec<AddressRule>, operations: Vec<GraphQlOperation> }`
  (`schematic/define/src/graphql.rs:22-29`). `GraphQlOperation { id,
  description, kind: Query|Mutation, document, variables: Unit|Typed(TypeRef),
  data: TypeRef, operation_name?, errors, no_declared_errors, pagination? }`
  (`:109-135`).
- **OpenAPI today.** `schematic_define::openapi::export` ignores `graphql`
  without saying so. `schematic/define/src/openapi/export/mod.rs:64-117` never
  reads it. The *generator* emits one WARN per operation
  (`Feature::GraphQlOperation`, `"graphql-operation"`;
  `schematic/gen/src/diagnostics.rs:160-162`) through
  `export::vocabulary::graphql_skips`
  (`schematic/gen/src/export/vocabulary.rs:31-46`), which `validate_openapi`
  calls (`schematic/gen/src/validation.rs:147-153`). This already meets R55 for
  OpenAPI: no path and one named diagnostic per operation. It is outside the
  round-trip subset.
- **Postman today.** The same `graphql_skips` call skips every operation with
  WARN (`validation.rs:162-167`). R55 replaces it with one `POST` per operation.
- **P10-g.** See D1. Wrapper methods exist, so there is no gap to accept.

## Discrepancies with the plan

Each item gives the adopted resolution, consistent with the plan's rules.

- **D1. P10-g is stale.** `2026-10-12-schematic-endpoint-methods`, which is
  completed, generates an inherent method and a `{Endpoint}Call` for every
  REST endpoint (`schematic/gen/src/codegen/endpoint_methods/mod.rs:1-27`).
  - **Resolution:** do not record an ACCEPTED GAP for wrappers, and do not
    benchmark. R39 documentation keeps the plan's
    `client.send(Request::new(..))` and may name endpoint methods, because
    they exist. Log P10-g as satisfied upstream, for review 1.
  - **Also for review 1:** that spec deferred per-API feature flags (its
    option A). P10-d's module features may supersede that deferral.
- **D2. No `x-schematic` reader exists.** The importer never reads either
  extension. The only extension it reads is the internal
  `x-schematic-query-rust-names` (`import/builder.rs:384`). R10 and R14 are
  therefore a new reader. The serde round trip at `extensions.rs:533-565` is
  not evidence of public import fidelity. This is as the plan expected.
- **D3. The strict symbol differs.** The plan names `ImportOptions::strict`.
  The real symbol is `OpenApiImportOptions::strict` and the
  `OpenApiImport::strict()` gate (`import/builder.rs:140-148,200,479-480`).
  The gen `ImportOptions::strict` (`import_pipeline.rs:47-48`) delegates to
  it. R16 targets the builder. The AsyncAPI gate (`asyncapi_import.rs:152`)
  already behaves correctly.
- **D4. `retry_guidance` was not anticipated.** It is in both extensions and
  in the definition model. Resolution: add it to the subset, at document and
  operation level.
- **D5. Upstream fields that the extension does not carry.** These are
  `RestApi.env_auth`, `RestApi.secret_names`, `ParamDef.secret`,
  `ParamDef.rust_name`, reserved-expansion markers, and the operation-level
  `EndpointAuth::Policy.env_mapping`.
  - **Resolution:** add `env_auth`, `secret_names`, `params.<loc>.<name>.{secret,
    rust_name}`, `path_template`, and operation `env_mapping`.
  - **Judgment call for review 1:** whether `env_auth` and operation
    `env_mapping` count as "auth fields" under R54. They hold environment
    variable *names* and mirror the already-carried `env_mapping`, so they
    were treated as not auth fields.
  - **Fallback:** if that is ruled otherwise, list them as R10 differences.
    Gitea then keeps losing `GITEA_TOKEN`.
- **D6. Exporter and importer auth.**
  - The exporter ignores `RestApi.auth_policy` (`export/components.rs:16`,
    `export/security.rs:129-133`), maps a bearer header override to a plain
    `apiKey`, and drops `value_prefix`.
  - The importer takes the first component scheme (`import/auth.rs:17-36`) and
    ignores root and operation `security`.
  - **Resolution:** root `security` is derived from `effective_auth_policy()`,
    with one requirement per accepted method (reusing `accepted_methods`,
    `export/security.rs:137-146`). The importer maps applied requirements:
    one requirement becomes `AuthStrategy`, several become `auth_policy`.
    `auth.<scheme>.{bearer, value_prefix}` carries the two R54 details.
  - **Not round-tripped:** the explicit/`env_fallback` split of a hand-written
    `AuthPolicy`, and OAuth2 `revocation_url`, `pkce`, `client_auth` and the
    device-code grant. These are listed differences; R54 forbids carrying
    them. No shipped REST definition sets `auth_policy`, an endpoint auth
    override or OAuth2, so artifacts do not drift.
- **D7. Pagination was not anticipated.** `default_pagination`,
  `Endpoint.pagination` and `ParamDef.pager_managed` come from spec 3, which
  P10-b's refresh list leaves out. No exporter writes them.
  - **Resolution:** keep them outside v1 and list them as R10/P10-l
    differences. This is large: in the historical snapshots OpenAI has 60
    paginated endpoints, Bitbucket 8 and each EMQX member 7.
  - **Low cost to undo:** a later `pagination` key is additive, because
    unknown v1 keys are INFO. This is review-critical, not blocking.
- **D8. Grouped documents.** `ollama.json` and `emqx.json` merge two APIs.
  They keep only the first member's `x-schematic`, add
  `x-schematic-grouped-from`, retitle `info` to the module name, and prefix
  clashing operation ids (`schematic/gen/src/export/openapi.rs:125-160`).
  - **Resolution:** importing a grouped artifact yields one `RestApi`.
    P10-l compares the first member's metadata and each endpoint by
    method and path, and lists the remaining differences. No extension is
    added to split members back out.
- **D9. Snapshot basis.** The committed `definition-baseline` fixtures are
  historical and normalized (`schematic/docs/definition-snapshots.md`).
  - **Resolution:** round-trip equality compares
    `canonical(serde_json::to_value(current definition))` from
    `schematic/gen/examples/support/definition_snapshot.rs:35` with the
    re-imported definition, not with the committed fixtures.
  - **Counts:** 15 OpenAPI artifacts versus 18 REST definitions. Two
    artifacts are grouped, and Artificial Analysis's two members share one
    module. Derive the counts; do not hard-code them.
- **D10. Multipart import already exists.** R18 is partial. Multipart already
  imports (`import/mappings/responses.rs:52-221`), but it ignores `encoding`,
  `minItems`/`maxItems` and JSON parts. Object-valued parts and unresolved
  parts silently become text (`:187-189,203,219`). The exporter writes no
  `encoding` or `accept` (`export/request_body.rs`, `File { accept: _ }`).
  Resolution: Phase 4 fills these gaps. The extension `request` is written
  only for form bodies.
- **D11. The AsyncAPI subset leaves out load-bearing WebSocket metadata.**
  - P10-a's premise that correlation hints "have zero users" is false:
    `CorrelationHints` is declared by Core, Dock and Integration
    (`schematic/definitions/src/unfolded_circle/*_ws/mod.rs`), and codegen
    consumes it (`ws_codegen/plan.rs:221,265`).
  - Also outside the plan's subset: `auth_flow` (three APIs; it drives the
    generated `connect_{endpoint}_with_setup`), lifecycle
    `open`/`close`/`open_credential_field` (ElevenLabs), `responds_with`
    (three APIs), the API `runtime` hints, `env_mapping` and `env_auth`.
  - **Resolution:** follow the plan. Correlation is excluded, and every
    unlisted item is outside the subset and logged as an R27 difference per
    API. One exception: `responds_with` maps to the standard 3.0
    `operation.reply`, which is not a new extension.
  - **Consequence:** four of the five APIs re-import with generated-client
    differences. This needs author attention at review 1. Not blocking,
    because the plan gives a default and later extensions would be additive.
- **D12. There is no JSON schema for WebSocket payloads (review-critical, not blocking).** WS
  payload types derive no `JsonSchema` (for example
  `schematic/definitions/src/unfolded_circle/dock_ws/types.rs`).
  `WebSocketApi` has no `schemas`, and no catalog or registry exists for WS
  types. So `components.schemas` "from the model catalog" (P10-a, R25) cannot
  be produced. Adding the derives is a definitions change beyond API-level
  constructor calls, which the plan's execution-authority rule forbids.
  - **Provisional resolution:** emit one `components.schemas.<type_name>`
    per payload. It is an unconstrained `{}` with a description naming the
    Rust type, so there is no dangling `$ref`. R27 then compares type names.
  - **What is unmet:** "typed messages and their schemas" (R25), recorded as
    an open gap.
  - **Classification (orchestrator, 2026-10-23):** not blocking. The plan's
    Execution-authority rule already decides this case: "A definitions
    change reaching beyond API-level constructor calls is left unmade and
    logged, and the run continues." The provisional resolution is the
    adopted default. The author may still allow the derives at review 1;
    doing so regenerates the committed `schematic/asyncapi/` artifacts,
    which is a diff to revise, not a public API later phases build on.
- **D13. `--ws-role` conflicts with "role does not reverse directions".**
  `--ws-role server` reverses directions (`asyncapi_import.rs:772-790`).
  - **Resolution:** keep that for 2.x input.
  - **For 3.0 input:** directions come only from `operations.*.action`.
    `--ws-role` sets the role only on channels without `x-schematic-role`.
- **D14. Staged import outputs are partial.** Only the standalone OpenAPI
  import uses `Planner`/`MutationPlan`. `--definitions-out` and `model_gen`
  write directly with `write_atomic`, and AsyncAPI writes only a report.
  - **Resolution:** R26 reuses `Planner`/`MutationPlan`
    (`schematic/gen/src/output/publish.rs:68-70`), the existing contract.
  - **Not done:** this feature does not migrate the `--definitions-out` path
    unless R26 requires it.
- **D15. GraphQL.** The OpenAPI skip already exists at generator level with a
  WARN per operation (see the GraphQL section). The Postman skip is replaced
  by R55. No shipped API exercises it, so the required tests use fixtures,
  for example the `graphql_fixture` in
  `schematic/gen/tests/l1/fresh_output_compile.rs:733`.
- **D16. Comment and documentation drift.** This was observed, not fixed,
  because this phase must not edit source. For Phases 4 and 12:
  - `RestApi.cross_origin` says "endpoints cannot override it"
    (`types.rs:186-187`), but `Endpoint.cross_origin` does override it
    (`:506`).
  - The `RestApi.version` doc (`types.rs:160-162`) has the precedence
    reversed compared with `export/info.rs:7-11`, where options win.
  - `extensions.rs:1-11` and `export/mod.rs:3-5,47-49` claim round-trip
    fidelity.
  - `schematic/docs/io/openapi-extensions.md:3` says the importer does not
    read the extensions, which is true today.
  - `import/builder.rs:140-143` documents the INFO-failing strict mode.
  - `import_pipeline.rs:47` documents strict as "warning-level".
- **D17. Typed path parameters.** The importer drops path parameters
  (`mappings/parameters.rs:33,77`). An undeclared placeholder and a declared
  plain-string parameter export identically (`export/paths.rs:95-133,268-286`).
  - **Resolution:** import a path parameter into `EndpointParams.path` when
    any path parameter of the operation has a non-string schema, a
    description or `deprecated`. Otherwise leave `path` empty.
  - **Listed difference:** a declared, undescribed, plain-string path
    parameter. Its generated request type regains `Default`.
- **D18. `response` is a required member.** It is not an `Option`
  (`extensions.rs:131`), so a hand-written operation extension without it
  fails as a whole. The presence-aware decoder makes it optional.
- **D19. Null handling.** Plain derives treat null as absent
  (`schematic/docs/definition-snapshots.md`), and the v1 writer emits nulls
  at five positions. The plan's "explicit null is W" therefore applies
  everywhere except those writer-emitted positions, which are O (see "Explicit
  null written by the v1 writer").
- **D20. Header entries are positional.** `[[name, value]]` pairs are kept for
  artifact compatibility, so the "nested header field" columns are index 0 and
  index 1.
- **D21. No AsyncAPI 3.0 input exists in the repository.** Phase 10 creates
  3.0 fixtures from its own exports. The validator spike covers the vendored
  3.0 schema question separately.
- **D22. Diagnostic locations.** Existing import locations are not
  consistently JSON pointers. New `x-schematic` diagnostics use escaped RFC
  6901 pointers with a `#` prefix. Existing locations are not rewritten
  (surgical).
- **D23. Obsolete items.** `oauth_scopes` is already gone, so nothing is left
  to delete. `SchematicSchemaExtension` and `AuthExtension` are still present
  and unused. Their only references are the re-exports, their own tests, the
  `Default` allowlist test, two documentation pages and one skill page (see
  the mapping tables). Delete them after `auth.bearer` and `auth.value_prefix`
  land (R15).
- **D24. Import CLI flags override imported metadata.** `--module-path`
  overrides the imported `module_path`, and `--request-id-header` overrides
  `request_id_header` only when given. Today `import_pipeline.rs:120`
  overwrites with `None`, which Phase 4 must fix.
- **D25. Stale counts.** The plan's "18 REST, 5 WebSocket" holds: 18 baseline
  REST definitions and 5 WS definitions under `schematic/definitions/src`.
  Endpoint-literal and dependent counts were not refreshed here; that is the
  inventory task.

**Blocking:** none. D12 was first drafted as blocking; it is covered by the
plan's Execution-authority default (see D12). D1, D5, D7, D11 and D12 are
review-1 agenda items.
