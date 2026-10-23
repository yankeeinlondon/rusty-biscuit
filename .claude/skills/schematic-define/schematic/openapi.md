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
| `--module-path` | Override module path for generated code (wins over `x-schematic.module_path` only when given) |
| `--output` | Output directory for generated code |
| `--dry-run` | Preview generated code without writing files |
| `--strict` | Fail on any WARN or ERROR diagnostic (INFO never fails) |
| `--definitions-out` | Write a definitions-crate module (`mod.rs` with `define_{api}_api()` and the fallible `module_group()`, plus `types.rs`) instead of a standalone client |
| `--overlay <RUST_PATH>` | With `--definitions-out`: a hand-written `fn(RestApi) -> RestApi` the factory applies, so additions survive a re-import (OpenAI: `crate::openai_pagination::apply`) |
| `--exclude-path <PREFIX>` | Drop endpoints by path prefix (repeatable) |
| `--request-id-header <HEADER>` | Set `RestApi::request_id_header`; overrides `x-schematic.request_id_header` only when given (`just import-openai` passes `x-request-id`) |

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
endpoint error entry (never hoisted to the API, unless `x-schematic.errors`
restores the API defaults). Stream end conditions cannot be read from
standard OpenAPI, so imported streams end at connection close unless the
operation's `x-schematic.success` restores them. Bodies and responses with no named schema
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
    pub location: String,  // RFC 6901 pointer, e.g. `#/x-schematic/auth`
}
```

Strict mode fails on any WARN or ERROR, never on INFO (pinned by
`build_strict_mode_*` in `import/builder.rs` and
`strict_import_passes_an_info_only_document`).

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

Component schemas convert faithfully to OAS 3.0.3 (`define/src/openapi/export/conversion.rs`;
rules and examples in `schematic/docs/io/export-openapi.md#how-component-schemas-convert`):

- Both schemars dialects convert: root `nullable`, nested 2020-12 `type: [T, "null"]`,
  `const` (→ one-value `enum`), `{type: "null"}` members.
- Nullable `$ref`/composition → `anyOf: [X, {type: object, nullable: true, enum: [null]}]`;
  nullable enum keeps `null` in `enum`; several types → `anyOf` (never `oneOf`).
- A bare `$ref` stays bare; one with siblings becomes `allOf: [{$ref}]` + siblings.
- `"default": null` is dropped (openapiv3 reparse would lose it anyway).
- An inexpressible keyword fails export with `OpenApiError::UnsupportedFeature`
  and its JSON pointer. Export also runs `validate_schema_closure` first.
- Gates: `schematic-definitions` tests `schema_fidelity` (zero lost/typed-loss
  positions over 18 registries) and `schema_values` (jsonschema accepts/rejects).

### Standard import mappings

- Path-item `parameters` merge into each operation (identity: name + `in`,
  header names case-insensitive; operation wins in place). Local
  `#/components/parameters` refs resolve first; unresolved or remote refs are
  ERROR at their pointer, never fetched.
- Typed path params import only when some path param is non-string,
  described, or deprecated (then every placeholder, template order). A plain
  undescribed string path param imports as an undeclared placeholder.
- Operation/parameter `deprecated`, `info.version` (into `RestApi::version`)
  are read. No `servers` → WARN, base URL `https://api.example.com`.
- ERROR: empty `info.version`/`servers[0].url`/`externalDocs.url`, a
  non-required path param, empty `requestBody.content`, `apiKey` with empty
  `name`. `security: null` fails the import.
- Multipart: `encoding.<part>.contentType` → file `accept` (comma-split) or a
  JSON part; `minItems`/`maxItems` → file counts. A primitive part (or a
  component/composition of primitives) is text silently; an object-valued part
  without JSON encoding is WARN and stays text; unresolved part refs ERROR;
  encoding `headers`/`style`/`explode`/`allowReserved` WARN.
- Auth: root `security` is authoritative (none/one/several → anonymous /
  `AuthStrategy` / `AuthPolicy`); absent root `security` falls back to the
  first component scheme. Operation `security`: absent inherits, `[]` public,
  equal to root (no op details) inherits, otherwise `EndpointAuth::Policy`.

### Schematic Extensions

Export writes `x-schematic` at the document root and on each operation, and
the importer **reads it back** (`import/extension.rs` decodes,
`import/reconcile.rs` applies). Writer shapes: `SchematicDocExtension`,
`SchematicOpExtension`, `AuthDetail`, `ParamDetail` (serialize-only;
`EXTENSION_VERSION = 1`).

```yaml
x-schematic:                     # document
  version: 1                     # only here; absent = 1; anything else disables all extensions (one WARN)
  module_path: gitea
  env_auth: ["GITEA_TOKEN"]      # env var names, never credentials
  auth: { apiKeyAuth: { value_prefix: "token " } }   # or { bearer: true }; nothing else allowed
  secret_names: ["x-session"]
  # also: request_suffix, env_mapping, headers, rate_limit, retry_guidance,
  #       errors, cross_origin, request_id_header
paths:
  /models/{repo_id}:
    get:
      x-schematic:               # operation
        response: { Json: { Named: Model } }   # legacy, always written
        path_template: /models/{+repo_id}      # only when the path has {+name}
        params: { query: { api-key: { secret: true }, type: { rust_name: kind } } }
        # also: request (form bodies only), success, errors, no_declared_errors,
        #       headers, rate_limit, retry_guidance, cross_origin,
        #       auth + env_mapping (only with an operation `security` override)
```

Reader rules:

- Each member decodes on its own. Unknown member → INFO. Malformed, failing
  `validate_api`'s checks, or explicit `null` → WARN at an RFC 6901 pointer
  (`#/x-schematic/<member>…`, `#/paths/<escaped>/<method>/x-schematic/<member>…`),
  member dropped, siblings kept. Allowed nulls: `env_mapping`'s
  `bearer_token`/`basic_user`/`basic_pass`/`api_key`, form field
  `description`, `Files.min`/`max` (the writer emits them).
- Standard wins: `success`/`response`/`errors` are re-exported with
  `map_responses` and compared with the standard `responses` (descriptions
  ignored); mismatch → WARN, standard-derived value kept. Conflicting document
  `errors` are withdrawn as a whole with one WARN. A form `request` only fills
  `accept`, file counts, and a JSON part's reference. Legacy JSON/text/binary
  `request` is compared only.
- Auth details apply only to an `apiKey` header scheme the relevant
  `security` applies; else WARN. Fixed headers exported as single-value-enum
  header parameters are recognized and not re-imported as parameters.
- Does not round-trip: pagination (`default_pagination`, endpoint
  `pagination`, `pager_managed`), GraphQL, component Rust paths (rebuilt as
  `{module_path}::{Name}`), the explicit/env-fallback split of a hand-written
  `AuthPolicy`, OAuth2 details beyond standard flows, grouped documents beyond
  the first API's metadata and `info.title`, a declared undescribed
  plain-string path param.

Export side: root `security` has one requirement per method of
`effective_auth_policy()`; multipart writes `encoding` and file
`minItems`/`maxItems`; `explode: true` is written only for non-`form` styles;
JSON shapes hold the stored `TypeRef`, never a Rust path. GraphQL operations
are not exported (`graphql-operation` warning).

Tests that pin behavior: `define/tests/openapi_extension_matrix.rs` (every
member × absent/null/wrong/empty, JSON and YAML),
`openapi_standard_matrix.rs`, `openapi_import_parameters.rs`,
`openapi_multipart_import.rs`, `response_openapi.rs`, and
`gen/tests/l1/extension_round_trip.rs` (every committed `schematic/openapi/`
artifact imports strictly and re-exports with only listed differences). Docs:
`schematic/docs/io/openapi-extensions.md`.

Fixed headers (`.header(name, value)`, API or endpoint) also export as
standard parameters: one required header parameter per operation with
`schema: {type: string, enum: [value], default: value}`, endpoint over API by
case-insensitive name. Left out, and reported as a `fixed-header` warning
(`openapi::header_gaps` lists the same): a header the operation's effective
security sends, `Accept`/`Content-Type` values its content does not declare,
`Authorization`, explicit reserved-name parameters, and a fixed value an
explicit header parameter's schema rejects (a compatible one merges into it:
required, value as default). GitHub's `Accept: application/vnd.github+json`
and Gitea's raw-content `Accept` are the pinned shipped gaps. See
`schematic/docs/io/export-openapi.md#fixed-headers`.

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
