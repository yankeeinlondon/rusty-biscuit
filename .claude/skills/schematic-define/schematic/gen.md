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

**Available APIs**: `anthropic`, `openai`, `elevenlabs`, `huggingface`, `lmstudio`, `ollama-native`, `ollama-openai`, `emqx-basic`, `emqx-bearer`, `all`

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
| `--strict` | Fail on any warning-level diagnostic |

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
│                  │  generate_request_helpers(), generate_paginated_trait(),
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
├── prelude.rs       # Each API's client struct, plus runtime CallOptions, Credential, Response, ...
├── shared.rs        # encode_cookie_value, combine_cookie_headers, FormFile, Paginated
├── ws_shared.rs     # WebSocket support shared by the *_ws modules
├── anthropic.rs     # A module under 2000 lines stays one file
├── lmstudio.rs
├── openai/          # A larger module is split into a directory:
│   ├── mod.rs       #   module docs, re-exports, each API's client struct and tag
│   ├── requests.rs  #   per-endpoint request structs and their EndpointSpec impls
│   └── responses.rs #   (when the module has response types)
├── ollama/          # OllamaNative + OllamaOpenAI (combined, shared module)
├── emqx/            # EmqxBasic + EmqxBearer (combined, shared module)
├── elevenlabs_ws.rs # WebSocket clients
└── ...              # one entry per remaining module group
```

APIs that share a `module_path` (e.g., Ollama Native + OpenAI, EMQX Basic + Bearer)
are combined into a single module file. Each API gets its own client struct, tag
type, and request suffix to avoid naming collisions. Stale `.rs` files from previous
generations are automatically cleaned up.

## Generated Code Components

Generated REST code only describes requests; the hand-written
`schematic-runtime` sends them. The generated `Cargo.toml` depends on
`schematic-runtime` (path `../runtime`), not on `reqwest`.

### `lib.rs` and the Shared Module

```rust
// lib.rs: the runtime ABI this crate was generated for, checked at compile time
pub const SCHEMATIC_RUNTIME_ABI: u32 = 1;
const _: () = assert!(schematic_runtime::ABI == SCHEMATIC_RUNTIME_ABI, "...; regenerate the crate ...");
pub use schematic_runtime;

// shared.rs: request helpers only
pub fn encode_cookie_value(value: &str) -> String;      // RFC 6265 %XX encoding
pub fn combine_cookie_headers(headers: &mut Vec<(String, String)>, cookies: &[(String, String)])
    -> Result<(), schematic_runtime::InvalidArgument>;   // folds cookies into one `Cookie` header
pub struct FormFile { .. }                               // multipart file part
pub trait Paginated {}                                   // marker for paginated endpoints
```

Generated code defines no error type: sends report
`schematic_runtime::TransportError` or a `Response` envelope, constructors
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
    type Failure = schematic_runtime::NoDeclaredError;
    type Decoder = schematic_runtime::Json<User>; // Text / Binary / Empty per ApiResponse

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

Sending is `client.send(request, &CallOptions::new())` (or `send_raw`);
credentials (`api_key`, `basic_auth`, `credentials`), base URL, timeout, env
order, and response hooks are chained onto the client. They are validated by
the optional `build()?` (same type back) or on the first call
(`TransportErrorKind::InvalidClient`). See `schematic/gen/README.md` and the runtime topic pages
(`schematic/docs/runtime/`).

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
Full rules: `schematic/docs/generation/validation.md`.

```bash
$ schematic-gen validate --api gitea
[WARN] severity=warning api=Gitea endpoint=GetGitTreeRecursive feature=query-in-path output=rust: ...
  [PASS] rust
  [PASS] openapi
  [PASS] postman
```

## Generator Modules

| Module | Function | Purpose |
|--------|----------|---------|
| `codegen/api_struct/` | `generate_api_struct()` | API client struct, `{Api}Api` tag and its `Api` impl, the `api_client!` invocation, constants, `definition()`, `new()`, `basic_auth()` for Basic APIs; `auth.rs` translates the effective auth policy into `AuthConfig`, `helpers.rs` builds the rest of `definition()` |
| `codegen/request_structs/` | `generate_request_struct()` | Per-endpoint structs; `endpoint_spec.rs` emits the `EndpointSpec` impl, `into_parts.rs` its `into_parts` body, `named_params.rs` header/cookie parameter fields, `form.rs` form structs |
| `codegen/shared.rs` | `generate_request_helpers()` | Request helpers of the generated `shared.rs`: `encode_cookie_value`, `combine_cookie_headers`, `FormFile` |
| `codegen/traits.rs` | `generate_paginated_trait()`, `generate_paginated_impl()` | `Paginated` marker trait and its impls |
| `codegen/module_docs.rs` | `ModuleDocBuilder` | Module documentation |
| `import_pipeline.rs` | `run_import()` | OpenAPI import orchestration |
| `model_gen.rs` | `generate_models()` | Generate types.rs from ModelCatalog |

`codegen/client/`, `codegen/error.rs`, `codegen/request_enum.rs`,
`codegen/variant.rs`, and `api_struct/{constructors,fields,request_method,variant}.rs`
are still on disk but no `mod` declares them, so they are not compiled; edits
there change nothing.

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
| Module path mismatch | Compile error: unresolved import | `just test-fresh-output` |
| Shared module missing `request_suffix` | Compile error: duplicate struct names | `just test-fresh-output` |
| Generated crate built against another runtime ABI | Compile error naming both ABIs | Regenerate with the matching `schematic-gen` |
