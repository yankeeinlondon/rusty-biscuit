# OpenAPI Support

Schematic supports bidirectional OpenAPI 3.x integration: import OpenAPI specs to generate Rust clients, or export existing API definitions to OpenAPI format.

**Feature Gate**: OpenAPI functionality requires the `openapi` feature in `schematic-define`.

## Import OpenAPI Specs

Transform any OpenAPI 3.x specification into a type-safe Rust client.

### CLI Usage

```bash
# Basic import
schematic-gen import --input petstore.yaml --output generated/src

# With custom API name
schematic-gen import --input api.json --api-name MyPetStore --output src

# Strict mode (fail on warnings)
schematic-gen import --input api.yaml --output src --strict

# Dry run (preview without writing)
schematic-gen import --input api.yaml --output src --dry-run
```

### Import Options

| Option | Description |
|--------|-------------|
| `--input` | Path to OpenAPI spec file (JSON or YAML) |
| `--api-name` | Override API name (default: derived from spec title) |
| `--module-path` | Override module path for generated code |
| `--output` | Output directory for generated code |
| `--dry-run` | Preview generated code without writing files |
| `--strict` | Fail on any warning-level diagnostic |
| `--definitions-out` | Write a definitions-crate module (`mod.rs` with `define_{api}_api()` and the fallible `module_group()`, plus `types.rs`) instead of a standalone client |
| `--overlay <RUST_PATH>` | With `--definitions-out`: a hand-written `fn(RestApi) -> RestApi` the factory applies, so additions survive a re-import (OpenAI: `crate::openai_pagination::apply`) |
| `--exclude-path <PREFIX>` | Drop endpoints by path prefix (repeatable) |
| `--request-id-header <HEADER>` | Set `RestApi::request_id_header` (OpenAPI declares none; `just import-openai` passes `x-request-id`) |

### Library API

```rust
use schematic_define::openapi::{OpenApiImport, OpenApiSource};
use schematic_gen::import_pipeline::{run_import, ImportOptions};

// Option 1: High-level pipeline
let options = ImportOptions {
    input: "api.yaml".to_string(),
    api_name: Some("MyApi".to_string()),
    module_path: None,
    output: "generated/src".to_string(),
    definitions_out: None,
    overlay: None,
    exclude_paths: vec![],
    request_id_header: None,
    dry_run: false,
    strict: false,
    verbose: true,
};
let result = run_import(&options)?;

// Option 2: Low-level builder
let source = OpenApiSource::from_file("api.yaml")?;
let result = OpenApiImport::new(source)
    .api_name("MyApi")
    .prefer_json()
    .strict()
    .build()?;

println!("Generated {} endpoints, {} models",
    result.api.endpoints.len(),
    result.models.models.len());
```

`result.api` carries one schema component per imported model and passes
`validate_references()`; an operation whose JSON body or response names an
undefined schema fails the import with `OpenApiError::Definition`. See
`schematic/docs/io/import-openapi.md`.

### Responses

Every 2xx response becomes a success shape (`text/event-stream` an SSE
stream, `application/x-ndjson`/`application/jsonl` an NDJSON stream of the
schema's component) and every exact `400`-`599`, `4XX`, or `5XX` response an
endpoint error entry (never hoisted to the API). Stream end conditions cannot
be read from OpenAPI, so imported streams end at connection close unless the
document carries `x-schematic`. Bodies and responses with no named schema
become `serde_json::Value` (`TypeRef::Primitive(Json)`); render that type path
with `syn` parsing, never `format_ident!`.

A JSON `oneOf` separable by `discriminator` or array/object becomes one tagged
shape per member whether inline or behind a schema `$ref` (chains resolved).
Tags use the same values as the union's dispatching `Deserialize` (mapping
key, single `enum` value, component name); each shape's `variant` is the
member's component name (`Array`/`Object` when unnamed). An inseparable
referenced union keeps `Json(Named)` with a warning; error statuses keep one
shape (a referenced union stays named). The OpenAI overlay
(`definitions/src/openai_pagination.rs`) restores `GetConversationItem` to
`ConversationItem` so the public type does not change.

### Imported builder assemblies

`--definitions-out` emits fallible `Endpoint`/`RestApi` builder code and a
public `module_group() -> Result<Vec<RestApi>, DefinitionError>` over one
ordered `DefinitionContext`; catalog factories unwrap it. Nominal generated
models implement `Registration`; the emitter bounds stack use with small
endpoint helpers and batched registrations. The importer normalizes
query-bearing paths (OpenAI's seven beta operations share their ordinary ids
with an optional `beta` query input), rejects GET bodies, duplicate keys, and
malformed input, and clamps out-of-`i64` schema bounds with a warning.
OpenAI is regenerated through `just -f schematic/justfile import-openai`,
never edited by hand;
`import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition`
fails when the committed module and a fresh import differ.

### Type Mappings

| OpenAPI Type | Rust Type |
|--------------|-----------|
| `string` | `String` |
| `string` + `format: date-time` | `String` (or chrono types) |
| `integer` | `i64` |
| `integer` + `format: int32` | `i32` |
| `number` | `f64` |
| `boolean` | `bool` |
| `array` | `Vec<T>` |
| `object` | Named struct |
| `object` + `additionalProperties` | `HashMap<String, T>` |
| `oneOf`/`anyOf` | Enum; decoded by its `discriminator` when every member has a tag value, else untagged |
| `$ref` | Named type reference |

### Diagnostics

Import produces diagnostics for potential issues:

```rust
pub enum DiagnosticSeverity {
    Info,   // Informational messages
    Warn,   // Non-blocking issues
    Error,  // Blocking issues (fail in strict mode)
}

pub struct OpenApiDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub location: Option<String>,  // JSON path
}
```

Use `--strict` to fail on warnings.

## Export to OpenAPI

Generate OpenAPI 3.0.3 specs from existing Schematic API definitions.

### CLI Usage

```bash
# Export as JSON (default)
schematic-gen generate --api openai --openapi-out specs/

# Export as YAML
schematic-gen generate --api openai --openapi-out specs/ --openapi-format yaml

# Generate all with OpenAPI export
schematic-gen generate --api all --openapi-out specs/
```

### Library API

```rust
use schematic_define::openapi::{export, ExportOptions, ExportFormat};
use schematic_definitions::openai::define_openai_api;

// Components come from `api.schemas`; there is no separate registry argument.
let api = define_openai_api();

let options = ExportOptions::new()
    .with_version("1.0.0")
    .with_format(ExportFormat::Yaml);

let openapi_doc = export(&api, &options)?;
```

### Schematic Extensions

OpenAPI specs include `x-schematic` extensions for round-trip fidelity:

```yaml
# Document level
x-schematic:
  module_path: "my_api"
  request_suffix: "Request"
  env_mapping:
    bearer_token: ["API_KEY"]
  headers:
    - ["X-Custom", "value"]

# Operation level
x-schematic:
  request:
    Json: { Named: CreateUserBody }
  response:
    Json: { Named: User }        # first success shape; Text/Binary/Empty are scalars
  success: [...]                 # every shape, only when `response` cannot carry them
  errors: [...]                  # the endpoint's own error entries
  headers:
    - ["X-Request-ID", "12345"]
```

Emitted: document-level and operation-level `x-schematic`; the schema-level
`SchematicSchemaExtension` is defined but not emitted. The document level
carries `module_path`, `request_suffix`, `env_mapping`, `headers`,
`rate_limit`, `retry_guidance`, `errors` (API defaults), `request_id_header`,
and `cross_origin` (only when not `SameOrigin`). The operation level carries
`request`, `response`, `headers`, `success` (several shapes, a stream, or any
selection fact, variant, or note), `errors`, `no_declared_errors`, and the
endpoint's own `rate_limit`/`retry_guidance`/`cross_origin`. Deprecation and
endpoint auth overrides use standard `deprecated`/`security`. JSON shapes hold
the stored `TypeRef` (component names plus wrappers), never a Rust path.
GraphQL operations are not exported (`graphql-operation` warning). See
`schematic/docs/io/openapi-extensions.md`.

## Feature Configuration

In `Cargo.toml`:

```toml
[dependencies]
schematic-define = { path = "../define", features = ["openapi"] }
```

The `openapi` feature enables:
- `openapi` module with import/export functions
- OpenAPI 3.x parsing via `openapiv3` crate
- YAML support via `serde_yaml`
