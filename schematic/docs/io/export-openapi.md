---
prompt:

---

# Exporting OpenAPI from Schematic

This document proposes a first-class OpenAPI export path for the APIs defined in
`schematic-definitions`, alongside the existing Rust client generation in
`schematic-schema`. The design is additive, keeps `schematic-gen` in control of
the transformation, and preserves all Schematic metadata using a structured
vendor-extension surface.

## Goals

- Export OpenAPI 3.0.x documents for each API defined in `schematic-definitions`.
- Preserve **all** Schematic metadata without loss during conversion.
- Keep the export ergonomic (CLI + library API) and self documenting.
- Keep existing Rust code generation unchanged and stable.

## Non-Goals

- Import OpenAPI into Schematic (covered separately).
- OpenAPI 3.1 or AsyncAPI generation (may be future work).
- Serving Swagger UI / Redoc (left to downstream tooling).

## OpenAPI Modeling Choice

Use the `openapiv3` crate (OpenAPI 3.0.x) as the canonical AST. This aligns with
the repository review in `schematic/docs/io/crates-openapi.md` and keeps the
dependency surface minimal.

## High-Level Design

1. Add an OpenAPI export module inside `schematic-gen` that maps
   `schematic-define` types into `openapiv3::OpenAPI`.
2. Introduce a lightweight schema registry in `schematic-definitions` to provide
   JSON schema for each Rust type referenced by `Schema`.
3. Extend the CLI so a single run can emit both Rust clients and OpenAPI specs.

## CLI UX

Prefer a single generation command with an OpenAPI output flag to reduce
confusion and keep the workflow consistent:

```bash
# Emit Rust clients and OpenAPI together
schematic-gen generate --api openai --output schematic/schema/src --openapi-out schematic/openapi

# YAML output
schematic-gen generate --api openai --output schematic/schema/src --openapi-out schematic/openapi --openapi-format yaml
```

Recommended output layout:

```
schematic/
  openapi/
    openai.json
    elevenlabs.json
    huggingface.json
```

### Justfile Integration

Add explicit `just` targets so the workflow is discoverable and consistent:

```bash
# Generate Rust clients (current behavior)
just -f schematic/justfile generate

# Generate Rust clients + OpenAPI specs
just -f schematic/justfile generate-openapi

# Generate OpenAPI specs only (optional helper)
just -f schematic/justfile export-openapi
```

The `generate-openapi` target should call `schematic-gen generate` with
`--openapi-out` and optional `--openapi-format`. The `export-openapi` helper can
call the same command but skip the Rust output by using a temporary output dir
if needed.

## Schema Source of Truth

`schematic-define::Schema` only carries a Rust type name and module path. To
produce a correct OpenAPI schema, we need a registry that can map each schema
name to a concrete JSON schema.

### Proposed Registry API

Add a small registry in `schematic-definitions` (per module):

```rust
// definitions/src/openai/mod.rs
pub fn openapi_registry() -> SchemaRegistry {
    SchemaRegistry::new()
        .register::<Model>("Model")
        .register::<ListModelsResponse>("ListModelsResponse")
        .register::<DeleteModelResponse>("DeleteModelResponse")
}
```

`SchemaRegistry` is a tiny helper that:

- Stores the Rust type name and module path (from `Schema`)
- Generates JSON Schema using `schemars::schema_for!` (OpenAPI-compatible mode)
- Tracks doc comment descriptions via `schemars` settings

The exporter then receives both the `RestApi` and a registry of referenced
schemas. This ensures the OpenAPI spec is fully typed and self-documenting.

### Why `schemars`

`schemars` can derive JSON Schema directly from Rust types. Using
`SchemaSettings::openapi3()` keeps the emitted schema aligned with OpenAPI 3.0.
This avoids manual schema authoring and keeps the spec accurate as types evolve.

### Unregistered Types

If a `Schema` is referenced but not registered, the exporter should fail with a
clear error that lists the missing type names. This keeps the export strict and
prevents silently incorrect OpenAPI output.

## Mapping Rules

### Document-Level

- `openapi`: `3.0.3`
- `info.title`: `RestApi.name`
- `info.description`: `RestApi.description`
- `info.version`: CLI override if provided, otherwise `schematic-definitions`
  crate version (fallback: `0.0.0`)
- `servers`: `RestApi.base_url`
- `externalDocs`: `RestApi.docs_url`

### Paths & Operations

- `Endpoint.path` maps to OpenAPI `paths` entries
- `Endpoint.method` maps to the corresponding operation
- `operationId`: `Endpoint.id`
- `description`: `Endpoint.description`

### Path Parameters

Path parameters are inferred from `{param}` segments in the path. Since the
current model does not include param types, they are emitted as `string`.

```yaml
parameters:
  - name: model
    in: path
    required: true
    schema: { type: string }
```

### Headers

Static headers in `RestApi.headers` and `Endpoint.headers` become header
parameters with default values. These are tagged with a vendor extension to
mark them as fixed.

### Request Bodies (`ApiRequest`)

| Variant | Content-Type | Schema |
|---------|--------------|--------|
| `Json(Schema)` | `application/json` | `$ref` to components.schemas |
| `FormData` | `multipart/form-data` | `type: object` with form fields |
| `UrlEncoded` | `application/x-www-form-urlencoded` | `type: object` with form fields |
| `Text` | `content_type` | `type: string` |
| `Binary` | `content_type` | `type: string`, `format: binary` |

Form field mapping:

- `FormFieldKind::Text` => `type: string`
- `FormFieldKind::File` => `type: string`, `format: binary`
- `FormFieldKind::Files` => `type: array`, `items: { type: string, format: binary }`
- `FormFieldKind::Json(Schema)` => `$ref` + encoding `contentType: application/json`
- `required` controls the schema `required` list
- `description` maps to property `description`
- `accept`, `min`, `max` are stored in vendor extensions

### Responses (`ApiResponse`)

| Variant | Status | Content-Type | Schema |
|---------|--------|--------------|--------|
| `Json(Schema)` | 200 | `application/json` | `$ref` |
| `Text` | 200 | `text/plain` | `type: string` |
| `Binary` | 200 | `application/octet-stream` | `type: string`, `format: binary` |
| `Empty` | 204 | (none) | (none) |

If an API uses a non-standard response code, we preserve the exact Schematic
response metadata via vendor extensions (see below).

### Auth Strategy

Map `AuthStrategy` to `components.securitySchemes`:

- `BearerToken { header: None }` => `type: http`, `scheme: bearer`
- `BearerToken { header: Some("X-Token") }` => `type: apiKey`, `in: header`, `name: X-Token`
- `ApiKey { header }` => `type: apiKey`, `in: header`, `name: header`
- `Basic` => `type: http`, `scheme: basic`
- `None` => no security requirement

Environment variable configuration is preserved via vendor extensions.

## Vendor Extensions (No Fidelity Loss)

All Schematic-only metadata is stored under a consistent `x-schematic` namespace
so the OpenAPI document can round-trip back into Schematic without loss.

### Document Extension

```yaml
x-schematic:
  module_path: "openai"
  request_suffix: "Request"
  env_mapping:
    bearer_token: ["OPENAI_API_KEY", "OPENAI_KEY"]
    api_key: null
    basic_user: null
    basic_pass: null
  env_auth: ["OPENAI_API_KEY", "OPENAI_KEY"]
  env_username: null
  auth:
    strategy: "BearerToken"
    header: null
  headers:
    - name: "X-Api-Version"
      value: "2024-01"
```

### Operation Extension

```yaml
x-schematic:
  request:
    kind: "FormData"
    fields:
      - name: "audio"
        kind: "File"
        accept: ["audio/*"]
      - name: "metadata"
        kind: "Json"
        schema: "Metadata"
  response:
    kind: "Binary"
  headers:
    - name: "X-Endpoint-Feature"
      value: "beta"
```

### Schema Extension

Each component schema stores its Rust type path:

```yaml
components:
  schemas:
    Model:
      type: object
      x-schematic:
        rust_type: "crate::openai::types::Model"
```

## Validation Rules

The exporter should reuse the existing validation pass and add OpenAPI-specific
checks:

- Every `Schema` referenced by `ApiRequest`/`ApiResponse` must be registered.
- Multipart URL-encoded requests must not include file fields.
- `BearerToken` with custom header emits an `apiKey` security scheme and adds an
  `x-schematic` note indicating the intent was bearer semantics.

## Export Pipeline Integration

Add a new phase after validation and before output writing:

```
API Definition
  -> Validation
  -> OpenAPI Export (optional)
  -> Rust Code Generation (existing)
  -> Output
```

The OpenAPI phase is independent and does not change the Rust generation path.

## Example: OpenAI (abridged)

```yaml
openapi: 3.0.3
info:
  title: OpenAI
  description: OpenAI REST API for model management
  version: 0.0.0
servers:
  - url: https://api.openai.com/v1
paths:
  /models:
    get:
      operationId: ListModels
      responses:
        "200":
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ListModelsResponse"
components:
  schemas:
    ListModelsResponse:
      type: object
      x-schematic:
        rust_type: "crate::openai::types::ListModelsResponse"
```

## Version Resolution

OpenAPI spec version is resolved with this priority chain:

1. `--openapi-version <VERSION>` CLI flag (highest priority)
2. `RestApi.version` field on the API definition
3. Fallback: `"0.1.0"`

## Grouped Export

APIs sharing a module (`ollama`, `emqx`) produce grouped OpenAPI documents that merge:

- **Paths** — Union of all endpoints from all APIs
- **Security schemes** — Union keyed by scheme name
- **Per-operation security** — From the originating API's auth strategy
- **Servers** — Deduplicated union of base URLs
- **Info** — Module name as title, concatenated descriptions

## Schema Registries

All 16 REST APIs now have complete schema registries with `#[derive(JsonSchema)]` on response types. Missing a registry for a new API produces an error (use `--no-openapi` to skip).

## Default Artifact Generation

When the output path ends with `schema/src`, OpenAPI specs are generated automatically to `openapi/` without requiring explicit `--openapi-out`. Use `--no-openapi` to suppress.

## Open Questions / Future Enhancements

- Consider an AsyncAPI export path for WebSocket definitions.
- Provide a `--bundle` flag to emit a single combined spec for `--api all`.
