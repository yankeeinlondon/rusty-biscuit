# schematic-gen

Code generator that transforms REST API definitions into strongly-typed Rust client code.

## CLI Reference

```bash
# Generate client code
schematic-gen generate --api <NAME> [--output <DIR>] [--dry-run] [-v|-vv|-vvv]

# Generate with OpenAPI export
schematic-gen generate --api <NAME> --openapi-out <DIR> [--openapi-format json|yaml]

# Validate definition (no generation)
schematic-gen validate --api <NAME>

# Import from OpenAPI spec (feature-gated)
schematic-gen import --input <FILE> [--api-name <NAME>] [--module-path <PATH>] \
                     --output <DIR> [--dry-run] [--strict]

# Legacy syntax (backwards compatible)
schematic-gen --api <NAME> --output <DIR>
```

**Available APIs**: every REST registration's `key` in `REGISTRATIONS`
(`definitions/src/registry/inventory.rs`) in registration order, then `all`
(`pipeline::available_apis`); for example `anthropic`, `ollama-native`,
`emqx-bearer`, `unfolded-circle-core-rest`. Selecting one member generates its
whole module group. WebSocket APIs have no `--api` key; `generate` packages
them from the same inventory.

### `just` recipes (run in `schematic/`)

| Recipe | Does |
|---|---|
| `generate` | every API to `schema/src`, `openapi/`, `postman/` (with environments), then `export-asyncapi` to `asyncapi/` |
| `generate-one <key>` | one API's module group to `schema/src`, `openapi/`, `postman/`, keeping every other module |
| `generate-openapi` / `generate-postman` | `generate --api all` with only that export (`--no-postman` / `--no-openapi`) |
| `generate-asyncapi` | only `export-asyncapi --output asyncapi` |
| `check-drift` | committed artifacts equal fresh output; the real APIs compile offline |
| `doctest` | doctests of the schematic crates, then `doctest-schema` (the generated crate: default features, then `emqx` alone) |
| `test-schema` / `lint-schema` | nextest / clippy `-D warnings` on the generated crate (excluded from the workspace) |

## OpenAPI Import

Import any OpenAPI 3.x specification and generate Rust client code:

```bash
# Basic import
schematic-gen import --input petstore.yaml --output generated/src

# With custom API name
schematic-gen import --input api.json --api-name MyApi --output generated/src

# Strict mode (fail on warnings)
schematic-gen import --input api.yaml --output generated/src --strict
```

### Import Options

| Flag | Description |
|------|-------------|
| `--input` | Path to OpenAPI spec (JSON or YAML) |
| `--api-name` | Override API name (derived from spec title by default) |
| `--module-path` | Override module path for generated code |
| `--output` | Output directory for generated code |
| `--dry-run` | Print generated code without writing files |
| `--strict` | Fail on any WARN or ERROR diagnostic (INFO never fails) |

### What Gets Generated

- `{api_name}.rs` - API client struct, tag type, and request structs implementing `EndpointSpec`
- `types.rs` - Model structs/enums from OpenAPI schemas
- Proper Rust types from JSON Schema mappings
- Serde derives for serialization

## Generation Pipeline

```
API Definition (RestApi)
        │
        ▼
┌──────────────────┐
│ Phase 1: Codegen │  generate_api_struct(), generate_request_struct(),
│                  │  generate_request_helpers(), pagination impls,
│                  │  GraphQL operation requests, generate_endpoint_methods(),
│                  │  ModuleDocBuilder
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Phase 2: Assembly│  Combine TokenStreams, add imports, lint attrs,
│                  │  lib.rs ABI check, shared.rs, prelude.rs
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Phase 3: Validate│  Parse with syn to verify syntax
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Phase 4: Format  │  prettyplease for consistent style
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Phase 5: Output  │  Atomic write (temp + rename), Cargo.toml
└──────────────────┘
```

## Generated Output Structure

```
schema/src/
├── lib.rs           # Module declarations, SCHEMATIC_RUNTIME_ABI + compile-time check, `pub use schematic_runtime`
├── prelude.rs       # Each API's client struct, plus runtime Call, CallError, CallOptions, Credential, Response, ...
├── shared.rs        # encode_cookie_value, combine_cookie_headers, FormFile
├── ws_shared.rs     # WebSocket support shared by the *_ws modules
├── anthropic.rs     # A module under 2000 lines stays one file
├── lmstudio.rs
├── openai/          # A larger module is split into a directory:
│   ├── mod.rs       #   module docs, re-exports, each API's client struct and tag
│   ├── requests.rs  #   per-endpoint request structs, EndpointSpec impls, call types, endpoint methods
│   └── responses.rs #   (when the module has response types)
├── ollama/          # OllamaNative + OllamaOpenAI (combined, shared module)
├── emqx/            # EmqxBasic + EmqxBearer (combined, shared module)
├── elevenlabs_ws.rs # WebSocket clients
└── ...              # one entry per remaining module group
```

APIs that share a `module_path` (e.g., Ollama Native + OpenAI, EMQX Basic + Bearer)
are combined into a single module file. Each API gets its own client struct, tag
type, and request suffix to avoid naming collisions. Stale `.rs` files from previous
generations are automatically cleaned up. The combined module's docs open with a
sentence naming every API, then give each a `## {Api}` section with the docs its
own module would have (`ModuleDocBuilder::markdown`, headings one level deeper).

Every code example the generator writes is a `no_run` doctest (request types,
module docs, crate Quick Start, prelude); never emit a `text` fence for Rust. The
module, Quick Start, and prelude examples come from
`codegen::module_docs::client_example_lines` (endpoint method, then
`client.send({Name}Request::new(..))`, async wrapper in hidden `#` lines); the
crate-root and prelude copies are gated on the example API's module feature.
`just doctest-schema` compiles them with default features and with `emqx`
alone; `generated_docs::slow_*` does the same in a fresh crate.

## Generated Code Components

Generated REST code only describes requests; the hand-written
`schematic-runtime` sends them. The generated `Cargo.toml` depends on
`schematic-runtime` (path `../runtime`), not on `reqwest`.

The manifest declares one Cargo feature per module group (named for the
module, default = all), built by `cargo_gen::ModuleFeatures::new(registry,
registrations)` from the whole registry on every run, partial or full.
`lib.rs` gates each `pub mod`, `prelude.rs` each re-export, and both their doc
lines, with `#[cfg(feature = "<module>")]`; `ws_shared` compiles under any
WebSocket feature. `futures-util`, `thiserror`, and `tokio-tungstenite` are
optional, enabled only by WebSocket module features. Feature selections are
proven by the `slow_` test `module_features::slow_feature_selections_compile_only_their_module_groups`.

### `lib.rs` and the Shared Module

```rust
// lib.rs: the runtime ABI this crate was generated for, checked at compile time
pub const SCHEMATIC_RUNTIME_ABI: u32 = 7;
const _: () = assert!(schematic_runtime::ABI == SCHEMATIC_RUNTIME_ABI, "...; regenerate the crate ...");
pub use schematic_runtime;

// shared.rs: request helpers only
pub fn encode_cookie_value(value: &str) -> String;      // RFC 6265 %XX encoding
pub fn combine_cookie_headers(headers: &mut Vec<(String, String)>, cookies: &[(String, String)])
    -> Result<(), schematic_runtime::InvalidArgument>;   // folds cookies into one `Cookie` header
pub struct FormFile { .. }                               // multipart file part
```

A paginated endpoint implements the runtime's `schematic_runtime::pager::Paginated`
directly; the shared module defines no pagination trait.

Generated code defines no error type: an awaited call reports
`schematic_runtime::CallError<E>` (`.full()`: a `Response` envelope or
`schematic_runtime::TransportError`), client settings
`schematic_runtime::BuildError`.

### Per-Endpoint Request Struct

```rust
/// Request for `GetUser` endpoint.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetUserRequest {
    /// Path parameter: user_id
    pub user_id: String,
}

impl GetUserRequest {
    /// Creates a new request with required path parameters.
    pub fn new(user_id: impl Into<String>) -> Self {
        Self { user_id: user_id.into() }
    }
}

impl schematic_runtime::EndpointSpec for GetUserRequest {
    type Api = MyApi;
    type Success = User;
    type Failure = schematic_runtime::NoDeclaredError; // from the effective error table
    type Decoder = schematic_runtime::Json<User>;      // per success shape(s); see responses.md

    const ENDPOINT_ID: &'static str = "GetUser";
    const METHOD: schematic_runtime::Method = schematic_runtime::Method::GET;

    fn into_parts(self) -> Result<schematic_runtime::RequestParts, schematic_runtime::InvalidArgument> {
        let path = format!("/users/{}", urlencoding::encode(&self.user_id.to_string()));
        Ok(schematic_runtime::RequestParts {
            method: <Self as schematic_runtime::EndpointSpec>::METHOD,
            path,
            query: Vec::new(),   // unencoded declared pairs; the runtime encodes them once
            headers: vec![],
            body: schematic_runtime::RequestBody::Empty,
        })
    }
}
```

### API Client Struct

```rust
#[derive(Clone, Debug)]
pub struct MyApi(schematic_runtime::Client<MyApiApi>);    // the one name callers write
pub enum MyApiApi {}                                       // tag named by request types

impl schematic_runtime::Api for MyApiApi {
    const NAME: &'static str = "MyApi";
}

// Every client-wide setting, build, send, send_raw, and check_link
schematic_runtime::api_client!(MyApi, MyApiApi);

impl MyApi {
    pub const BASE_URL: &'static str = "https://api.example.com/v1";
    pub const DOCS_URL: Option<&'static str> = None;

    // Base URL, AuthConfig (credential placements + env fallback),
    // cross-origin policy, secret names, request-id header
    pub fn definition() -> schematic_runtime::ApiDefinition;
    // Seeded from definition() and the API's static headers (other than
    // Cookie); never fails
    pub fn new() -> Self;
    // Only when the API accepts HTTP Basic
    pub fn basic_auth(self, username: impl Into<String>, password: impl Into<String>) -> Self;
}
```

Sending is an endpoint method plus `.await` (next section); `client.send(request)`
takes a request object for generic code, and `send_raw(request, &CallOptions)`
skips decoding. Credentials (`api_key`, `basic_auth`, `credentials`), base URL, timeout, env
order, and response hooks are chained onto the client. They are validated by
the optional `build()?` (same type back) or on the first call
(`TransportErrorKind::InvalidClient`). See `schematic/gen/README.md` and the runtime topic pages
(`schematic/docs/runtime/`).

### Endpoint Methods and Call Types

Per REST endpoint the generator also emits an inherent method on the API
struct, named after the endpoint id in snake case (`ListReleases` →
`list_releases`), taking the required values in `{Endpoint}Request::new`
order, and a call type named like the request with `Request` replaced by
`Call` (`ListReleasesCall`). The call type wraps
`schematic_runtime::Call`, has one setter per optional value (the request field
name without `with_`), forwards `.anonymous()`, `.credential(..)`,
`.deadline(&d)`, `.full()`, and `.build()`, and implements `IntoFuture`:

```rust
// GitHub, from schematic/schema/src/github/requests.rs (abridged)
impl GitHub {
    pub fn list_releases(&self, owner: impl Into<String>, repo: impl Into<String>)
        -> ListReleasesCall<'_>;
}
pub struct ListReleasesCall<'a>(schematic_runtime::Call<'a, GitHubApi, ListReleasesRequest>);
impl<'a> ListReleasesCall<'a> {
    pub fn page(self, value: i64) -> Self;       // request.with_page(value)
    pub fn per_page(self, value: i64) -> Self;   // request.with_per_page(value)
    // anonymous, credential, deadline, full, build; `full` comes after the
    // setters, which its `FullCall` does not have
}

let releases = github.list_releases("owner", "repo").per_page(10).await?;
// Result<Success, CallError<Failure>>; CallError is #[non_exhaustive]
```

Name collisions are resolved by renaming the endpoint's name, never the shared
one:

- an endpoint method named like an API-struct method (`new`, `send`,
  `base_url`, `clone`, …) gains `_endpoint`
- a setter named like a call-type method (`full`, `build`, `anonymous`,
  `credential`, `deadline`, the reserved `pages`, `fetch_page`,
  `collect_all`, or a trait method such as `clone`) gains `_param`
- a Rust keyword goes through the request-field sanitizer (`type_`)

Each rename is an `endpoint-method-rename` warning; a name still claimed twice
is a `generated-name-collision` error (no numbered names are invented). The
rename list for shipped definitions is pinned by
`live_definitions_endpoint_method_renames_are_pinned` in
`schematic/gen/tests/l1/output_validation.rs` and is currently empty. Rules:
`schematic/gen/src/codegen/endpoint_methods/names.rs` and
`schematic/docs/generation/validation.md`; calling them:
`schematic/docs/runtime/calling-an-api.md`.

## Postman export

Topic page: `schematic/docs/io/export-postman.md`. Non-obvious facts:
- Examples come from `export::examples::ExampleSource` over
  `RestApi::schemas` (the registered schemas, never the OpenAPI conversion),
  with `nullable: true` read as a `null` type and every candidate validated
  by `jsonschema` 0.55 (a normal dependency of `schematic-gen`) through one
  prepared `Registry` per API. Cost: all 644 shipped references in ~0.7 s
  debug; do not compile a validator per reference from the full document
  (OpenAI took >5 s that way).
- A value that cannot be built is never emitted: the body stays empty and
  `validate_postman` reports `example-value` (built by
  `export::postman::export_diagnostics`, i.e. a second collection build).
  Shipped APIs report none; `postman_runnable::every_shipped_example_validates`
  validates every body, saved response, and stream frame independently.
- Parameter variables are location-prefixed (`path_owner`, `query_page`,
  `header_X-Id`, `cookie_session`) and declared on the collection;
  cookies merge into one `Cookie` header. Shared-module members get their
  own `baseUrl{N}` and API-name-suffixed credentials
  (`bearerToken_EmqxBearer`), so a shared module with any auth has
  request-level auth.
- Environments (`{module}.postman_environment.json`) are written only with
  `--postman-environment` (the `just` recipes pass it) and are drift-checked;
  without the flag a complete run neither writes nor deletes them.
- Golden fixtures: `BLESS_POSTMAN_GOLDEN=1` with the `postman_golden` filter.

## AsyncAPI export and import

Topic page: `schematic/docs/io/asyncapi.md`. Non-obvious facts:
- `export-asyncapi` (not `generate`; `just generate`/`generate-asyncapi` run
  it) writes one 3.0 document per WebSocket registration to
  `schematic/asyncapi/{module}.json` (`{module}.{snake_api}.json` when a
  module is shared); `export::asyncapi::asyncapi_documents` is the naming
  rule. Diagnostics use `Output::AsyncApi` + `Feature::WebSocketDefinition`.
- Documents are always client-perspective (`send` = client message); the
  role rides in `x-schematic-role` and never flips a direction. Other
  extensions: `x-schematic-{selectors,fallback,heartbeat,param,value-prefix,
  bearer-header}` (names are `export::asyncapi::X_*`).
- Not exported (asserted per API by `asyncapi_round_trip`): env metadata,
  lifecycle roles, correlation and auth-flow hints, API runtime hints. The
  three Unfolded Circle APIs re-import without correlation and then fail
  `lower_to_plan` (`responds_with needs ... correlation`). Payload schemas
  are unconstrained `{}` (no WS `JsonSchema` derives).
- The importer is `asyncapi_import` (+ `reader`, `extensions`, `v3`):
  version-dispatched; any malformed standard field is a located ERROR that
  fails the import with nothing written (`import_asyncapi_text` writes
  nothing; `run_import_asyncapi` persists `{module}.definition.json`,
  `.models.json`, and the report through `prepare_export_output`). Auth is
  the *applied* server security only. Imported endpoints/messages come back
  in key order: compare with `common::asyncapi::canonical`.
- Exports are validated offline against the vendored
  `gen/tests/fixtures/asyncapi/spec-json-schemas/3.0.0.json` (Draft 7).

## Validation

`generate` validates the whole batch once per output (Rust, OpenAPI, Postman)
and writes only outputs with no error; one output's failure never blocks the
others, and any failure exits 1. Diagnostics print to stderr as
`severity=… api=… endpoint=… feature=… output=…: message`; warnings never fail.
A same-API duplicate method + normalized path (`/a/{x}` = `/a/{y}`) is an
error in every output, with no exception list (`endpoint_uniqueness_key`).
Grouped OpenAPI documents are built in memory for every module before any is
written (`export::openapi::build_openapi_grouped`); a later group member's
clashing `operationId` gets its API-name prefix, and the final
`operation_id_diagnostics` scan rejects any duplicate that remains.
Every Rust run also lowers the five shipped WebSocket definitions
(`output::ws_modules::validate_ws_definition_modules`) and reports their
findings on the Rust output as `feature=websocket-definition`, the message
starting with the definition path (`endpoints/{id}/path/...`): an error
blocks the Rust output (no WS module is dropped silently any more), a
warning does not (for example a `base_url` whose last segment repeats the
endpoint path's first segment), and info notes print only with `-v`. Test a
fixture `WebSocketApi` with `validate_ws_apis` / `render_ws_api_module`;
the fresh-output guard compiles a rendered fixture module through
`FreshCrate::add_module` (Bearer host; handshake path/query encoding).
WebSocket definition rules: `websocket.md`.
Full rules: `schematic/docs/generation/validation.md`.

```bash
$ schematic-gen validate --api example
[ERROR] severity=error api=Example endpoint=ReadTree feature=query-in-path output=rust: ...
  [FAIL] rust
  [FAIL] openapi
  [FAIL] postman
```

## Generator Modules

| Module | Function | Purpose |
|--------|----------|---------|
| `codegen/api_struct/` | `generate_api_struct()` | API client struct, `{Api}Api` tag and its `Api` impl, the `api_client!` invocation, constants, `definition()`, `new()`, `basic_auth()` for Basic APIs; `auth.rs` translates the effective auth policy into `AuthConfig`, `helpers.rs` builds the rest of `definition()` |
| `codegen/request_structs/` | `generate_request_struct()` | Per-endpoint structs; `endpoint_spec.rs` emits the `EndpointSpec` impl, `into_parts.rs` its `into_parts` body, `named_params.rs` header/cookie parameter fields, `form.rs` form structs |
| `codegen/shared.rs` | `generate_request_helpers()` | Request helpers of the generated `shared.rs`: `encode_cookie_value`, `combine_cookie_headers`, `FormFile` |
| `codegen/pagination/` | `paginated_impl()`, `operation_paginated_impl()` | `impl schematic_runtime::pager::Paginated` and capability markers for endpoints and GraphQL operations |
| `codegen/responses.rs` | `success_code()`, `failure_code()`, `api_error_body_impls()` | `Success`/`Decoder`/`Failure`, `{Id}Response`/`{Id}Error` enums, stream end markers, `ERRORS` |
| `codegen/graphql.rs` | `generate_operation_request()` | One request struct per GraphQL operation |
| `codegen/module_docs.rs` | `ModuleDocBuilder` | Module documentation |
| `import_pipeline.rs` | `run_import()` | OpenAPI import orchestration |
| `model_gen.rs` | `generate_models()` | Generate types.rs from ModelCatalog |

`codegen/responses.rs` turns `Endpoint::success_shapes()` and
`RestApi::effective_errors` into each `EndpointSpec`'s `Success`/`Decoder`/
`Failure`, the `{Id}Response`/`{Id}Error` enums, stream end markers, and
`ERRORS`; `api_error_body_impls` emits one `ErrorBody<{Api}Api>` per JSON
error type per API module (the trait takes the API marker because registered
types live in `schematic-definitions`, outside the orphan rule's reach).
`codegen/graphql.rs` emits one request struct per GraphQL operation
(`ADDRESS = Address::GraphQl`, a `Paginated` impl with
`page_root = graphql::page_data` when it declares pagination) and the
definition's `.graphql(&[..])` rules. The success, failure, and GraphQL
shapes are summarized in [`responses.md`](./responses.md).

`api_struct/{constructors,fields,request_method,variant}.rs` are still on disk
but no `mod` declares them, so they are not compiled; edits there change
nothing.

## Library API

```rust
use schematic_gen::output::{generate_and_write, generate_and_write_all};
use schematic_gen::validate_rust_client;
use schematic_definitions::openai::define_openai_api;

// Single API (generate_and_write also refuses an invalid batch on its own)
let api = define_openai_api();
for diagnostic in validate_rust_client(&[&api]) {
    eprintln!("{diagnostic}");
}
generate_and_write(&api, Path::new("schema/src"), false)?;

// Full generation: `apis` is the whole crate; obsolete generated modules go
let apis = vec![&api1, &api2, &api3];
generate_and_write_all(&apis, output_dir, dry_run)?;

// Partial generation: regenerate the selected APIs' complete module groups,
// keep every other module of the existing output byte-for-byte
use schematic_gen::output::{RustOutputRequest, Selection, prepare_rust_output};
let prepared = prepare_rust_output(&RustOutputRequest {
    registry: &all_apis,                 // every API, registry order
    registrations: registrations(),      // WebSocket modules to package
    selection: Selection::Apis(&[&api]), // or Selection::All
    output_dir,
    manifest_dir: Some(schema_dir),      // writes Cargo.toml too
})?;                                     // nothing written yet
prepared.plan.publish()?;                // reports completed/pending on I/O failure
```

Preparation checks ownership of every path it would write or delete (the
generated `.rs` header, the `Cargo.toml` marker line, export artifact names;
symlinks never followed) and, for a partial run, the existing inventory
(`lib.rs`/`prelude.rs` parse and match current definitions; while modules
are retained, `lib.rs`'s `SCHEMATIC_RUNTIME_ABI` equals the generator's
`RUNTIME_ABI` and `shared.rs`/`ws_shared.rs` are unchanged, otherwise the run
refuses with `RequiresFullGeneration` and changes nothing). See
`schematic/docs/generation/partial-generation.md`.

## Error Types

```rust
pub enum GeneratorError {
    ParseError(String),
    CodeGenError(String),
    WriteError { path: String, source: std::io::Error },
    OutputDirNotFound(String),
    ConfigError(String),
    // An output's validator reported errors; nothing of it was written.
    Validation { output: Output, diagnostics: Vec<ValidationDiagnostic> },
    // An existing output cannot be updated safely; nothing was changed.
    UnsafeOutput { root: String, problems: Vec<String>, remedy: String },
    // A partial run would keep modules while lib.rs's runtime ABI marker differs
    // (or is missing/malformed) or a shared file changes; nothing was changed.
    RequiresFullGeneration { root: String, changed: Vec<String>, retained: Vec<String> },
    // A write or deletion failed after publication started.
    PublicationFailed { root: String, failed: String, completed: Vec<String>, pending: Vec<String>, source: Box<GeneratorError> },
    // A generate run where at least one requested output failed.
    OutputsFailed { succeeded: Vec<Output>, failed: Vec<Output> },
}
```

## OpenAPI Export

Generate OpenAPI 3.0.3 specification from existing API definitions:

```bash
schematic-gen generate --api openai --openapi-out specs/ --openapi-format yaml
```

This exports the API definition for external tools (Swagger UI, code generators for other languages, etc.).

## Testing Gap Warning

**Tests run the fixture APIs through the runtime against wiremock, NOT against live providers!**

The fresh-output guard (`just test-fresh-output`, part of `just test`) generates
crates outside the checkout and compiles them offline:

| Test (`fresh_output_compile::`) | Proves |
|---------------------------------|--------|
| `slow_fixture_apis_compile_and_send_offline` | fixture APIs send through the runtime: wire bytes, credentials, codecs, hooks, definition settings |
| `slow_generated_types_reject_misuse_at_compile_time` | wrong success type, another API's request, mutator on a text endpoint, removed APIs fail to compile |
| `check_status_parity_preserves_every_sniff_fixture` | the GraphQL fixture operation reproduces every sniff check-status fixture (R43); `the_parity_document_is_sniffs_check_status_query` (fast) guards its document |
| `slow_a_runtime_of_another_abi_fails_the_build_with_a_readable_message` | the `lib.rs` ABI assert fires with its message |
| `slow_real_apis_compile_offline` | all 18 real APIs compile |
| `slow_query_wire_matches_pre_split_baseline` | query request lines match `tests/fixtures/wire_baseline/query_request_lines.txt` |
| `generated_rest_code_references_the_runtime_and_defines_none_of_it` (fast) | generated REST code uses `schematic_runtime` and defines no runtime of its own |

After modifying response handling:

```bash
# 1. Tests pass (necessary but not sufficient)
just -f schematic/justfile test
cargo nextest run -p schematic-gen --test l1 -E 'test(/^e2e_generation::(binary|text|empty|mixed)_response/)'

# 2. Generate code
just -f schematic/justfile generate

# 3. Verify compilation (schematic-schema is excluded from the workspace)
cargo check --manifest-path schematic/schema/Cargo.toml

# 4. Check the codec each non-JSON endpoint declares (Json<T>, Text, Binary, Empty):
grep -rn "type Decoder" schematic/schema/src/
```

| Scenario | Failure Mode | Detection |
|----------|--------------|-----------|
| Binary endpoint modeled as JSON | `MalformedSuccess` in the envelope (body kept); callers see the wrong success type | `grep "type Decoder"` |
| Streaming endpoint modeled as `Binary` | Caller gets one buffered body instead of `ItemStream<T>` | `grep "type Decoder"` (expect `Sse`/`Ndjson`) |
| Registered type named `{Id}Response` / `{Id}Error` | `generated-name-collision` | `schematic-gen validate` |
| Module path mismatch | Compile error: unresolved import | `just test-fresh-output` |
| Shared module missing `request_suffix` | Compile error: duplicate struct names | `just test-fresh-output` |
| Generated crate built against another runtime ABI | Compile error naming both ABIs | Regenerate with the matching `schematic-gen` |
