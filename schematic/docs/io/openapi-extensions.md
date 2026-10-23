# OpenAPI Vendor Extensions (x-schematic)

This document specifies the vendor extensions used by Schematic when exporting API definitions to OpenAPI 3.0.3 format. These extensions preserve Schematic-specific metadata that enables round-trip fidelity when importing and re-exporting API definitions.

## Overview

Schematic currently emits two `x-schematic` extension types in exported OpenAPI documents:

| Extension | Level | Purpose |
|-----------|-------|---------|
| `x-schematic` | Document | API-wide configuration (module path, headers, env mapping, rate-limit and retry metadata, default error entries) |
| `x-schematic` | Operation | Endpoint-specific metadata (request/response types, success shapes, error entries, headers, an endpoint's own rate-limit and retry metadata) |

The library also defines a schema-level extension type (`SchematicSchemaExtension`) for Rust type metadata, but the current exporter does not emit schema-level `x-schematic` fields.

All extensions are optional. Documents without `x-schematic` extensions can still be imported, but some Schematic-specific features may require manual configuration.

## Document-Level Extension

Added at the OpenAPI document root level alongside `openapi`, `info`, and `paths`.

```yaml
openapi: "3.0.3"
info:
  title: OpenAI
  version: 1.0.0
x-schematic:
  module_path: openai
  request_suffix: Request
  env_mapping:
    bearer_token:
      names: ["OPENAI_API_KEY"]
  headers:
    - ["anthropic-version", "2023-06-01"]
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `module_path` | `string?` | Override generated Rust module name. Used when multiple APIs share a definitions module. |
| `request_suffix` | `string?` | Custom suffix for generated request wrapper structs (default: "Request"). |
| `env_mapping` | `object?` | Environment variable configuration for authentication. See [Env Mapping](#env-mapping). |
| `headers` | `array<[string, string]>` | Default headers applied to all endpoints. |
| `rate_limit` | `object?` | The API's rate-limit headers. Absent when undeclared. See [Rate-Limit and Retry Metadata](#rate-limit-and-retry-metadata). |
| `retry_guidance` | `object?` | The API's retry guidance. Absent when undeclared. See [Rate-Limit and Retry Metadata](#rate-limit-and-retry-metadata). |
| `errors` | `array?` | The API's default error entries, as declared. Absent when there are none. See [Success Shapes and Error Entries](#success-shapes-and-error-entries). |
| `cross_origin` | `string \| object?` | The API's policy for redirects and next-page links that leave the base origin: `"AllowWithoutCredentials"`, `"DangerAllowAny"`, `"Deny"`, or `{"AllowOrigins": ["https://…"]}`. Absent for the default, `SameOrigin`. |
| `request_id_header` | `string?` | The response header that carries the provider's request id, for example GitHub's `x-github-request-id`. Absent when the provider is not known to send one. |

Deprecation and an endpoint's own authentication are not repeated here: they
are exported as the standard OpenAPI `deprecated` flag and `security` /
`securitySchemes` members.

### Env Mapping

The `env_mapping` object specifies how to resolve authentication credentials from environment variables:

```yaml
env_mapping:
  bearer_token:
    names: ["OPENAI_API_KEY", "OPENAI_TOKEN"]
  api_key:
    names: ["ANTHROPIC_API_KEY"]
    header: X-Api-Key
  basic_user:
    names: ["API_USER"]
  basic_pass:
    names: ["API_PASSWORD"]
```

| Field | Type | Description |
|-------|------|-------------|
| `bearer_token` | `{names: string[]}` | Env vars for Bearer token authentication |
| `api_key` | `{names: string[], header: string}` | Env vars and header name for API key auth |
| `basic_user` | `{names: string[]}` | Env vars for HTTP Basic auth username |
| `basic_pass` | `{names: string[]}` | Env vars for HTTP Basic auth password |

## Operation-Level Extension

Added at the operation level (inside GET, POST, etc.) to preserve endpoint-specific metadata.

```yaml
paths:
  /models:
    get:
      operationId: ListModels
      x-schematic:
        response:
          Json:
            Named: ListModelsResponse
        headers: []
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `request` | `object?` | Request body specification. See [Request Types](#request-types). |
| `response` | `object \| string` | The first success shape's kind, in the form below. See [Response Types](#response-types). |
| `headers` | `array<[string, string]>` | Endpoint-specific headers. |
| `rate_limit` | `object?` | The endpoint's own rate-limit headers, which replace the API's whole. Absent when the endpoint inherits the document's value. |
| `retry_guidance` | `object?` | The endpoint's own retry guidance, which replaces the API's whole. Absent when the endpoint inherits the document's value. |
| `success` | `array?` | Every success shape, present only when `response` cannot say it all: several shapes, a stream, or any content type, status, tag, fallback, note, or variant name. See [Success Shapes and Error Entries](#success-shapes-and-error-entries). |
| `errors` | `array?` | The endpoint's own error entries, which overlay the document's `errors`. Absent when there are none. |
| `no_declared_errors` | `bool?` | `true` when the endpoint opts out of the document's error entries. Absent when false. |
| `cross_origin` | `string \| object?` | The endpoint's own cross-origin policy, in the document-level form, which replaces the API's. Absent when the endpoint inherits it. |

### Rate-Limit and Retry Metadata

Both values are descriptive: they tell a reader which response headers
report the rate limit and how the provider says to retry, and nothing in
Schematic acts on them. Each appears in the document extension when the API
declares it, and in an operation extension only when that endpoint declares
its own value; an operation without one uses the document's. Neither is
written when undeclared, and [`skip_extensions()`](#skipping-extensions)
leaves both out with the rest of `x-schematic`.

```yaml
x-schematic:
  rate_limit:
    limit_header: x-ratelimit-limit
    remaining_header: x-ratelimit-remaining
    reset_header: x-ratelimit-reset
    reset_unit: epoch_seconds       # seconds | epoch_seconds | http_date
    retry_after_header: retry-after # optional
  retry_guidance:
    retry_on:                       # status codes and inclusive ranges
      - code: 429
      - range: { start: 500, end: 503 }
    honor_retry_after: true
    max_attempts: 4                 # including the first attempt
    backoff:                        # none | fixed | exponential
      fixed: { delay_ms: 750 }
```

`backoff` is the string `none`, `{fixed: {delay_ms}}`, or
`{exponential: {base_ms, cap_ms}}`. Every `retry_guidance` member is
required; only `retry_after_header` is optional in `rate_limit`. GitHub's
document declares the `rate_limit` above (without `retry_guidance`).

### Schema references

A JSON body or response stores a `TypeRef`, never a Rust path or a copy of the
schema. A reference is one or more component names plus wrapper shape:
`Named`, `Array`, `Optional`, `Map`, `KeyedMap` (a map whose Rust key type is
not `String`), or `Primitive` for an inline type such as `String`, `U32`, or
free-form `Json`. Each name resolves through the API's `schemas`, which also
supply the Rust path and become `components.schemas`:

```yaml
x-schematic:
  response:
    Json:
      Array:
        Named: Item            # resolves to components.schemas.Item
```

```yaml
# The matching entry in the definition's `schemas` (RestApi::schemas)
schemas:
  - name: Item
    rust_path: ::my_definitions::models::Item
    schema: { type: object, properties: { id: { type: string } } }
```

Generated Rust spells this response `Vec<::my_definitions::models::Item>`; the
OpenAPI response schema is an array of `$ref: '#/components/schemas/Item'`.
`Schema { type_name, module_path }` remains only for WebSocket messages.

This is transfer data, not an authoring constructor. Build with the typed JSON
methods; a deserialized definition is checked with `RestApi::validate_references`
before use. The endpoint OAuth scope field is removed; API authentication policy
remains supported.

### Request Types

```yaml
# JSON body
request:
  Json:
    Named: CreateMessageBody

# Multipart form data
request:
  FormData:
    fields:
      - name: document
        kind:
          File:
            accept: ["application/pdf", "image/*"]
        required: true
        description: The file to upload
      - name: metadata
        kind:
          Json:
            Named: FileMetadata
        required: false

# URL-encoded form
request:
  UrlEncoded:
    fields:
      - name: username
        kind: Text
        required: true
      - name: password
        kind: Text
        required: true

# Plain text
request:
  Text:
    content_type: text/csv

# Binary
request:
  Binary:
    content_type: application/octet-stream
```

### Response Types

```yaml
# JSON response
response:
  Json:
    Named: MessageResponse

# JSON array of a component
response:
  Json:
    Array:
      Named: Model

# Free-form JSON
response:
  Json:
    Primitive: Json

# Binary response (audio, images)
response: Binary

# Plain text response
response: Text

# No content (204)
response: Empty
```

### Success Shapes and Error Entries

The standard `responses` map carries each shape's status, media type, and
schema (see [Exporting OpenAPI](export-openapi.md#responses)). What it cannot
carry, such as a stream's end condition or the JSON tag that tells two shapes
apart, is written here in the definition's own serialized form, so nothing is
lost. An endpoint with a single plain shape writes no `success` member, which
keeps existing documents unchanged.

```yaml
x-schematic:
  response: Empty
  success:
    - shape:
        Json:
          Named: Message
      note: returned when `stream` is false
    - shape:
        Stream:
          framing:
            Sse:
              Named: Event
          end:
            EventNamed: message_stop
      note: returned when `stream` is true
    - shape: Binary
      fallback: true
  errors:
    - status:
        Exact: 404
      shape: Empty
  no_declared_errors: false   # omitted when false
```

The error entries are the declared levels (document defaults, then the
endpoint's overlay or opt-out), not the resolved table: the `responses` map
already shows the resolved table. Reading these members back is not
supported yet.

## Schema-Level Extension (Reserved)

`SchematicSchemaExtension` exists for preserving Rust type information, but schema-level `x-schematic` fields are not currently emitted by `export()`. If emitted in a future release, the shape is:

```yaml
components:
  schemas:
    Model:
      type: object
      x-schematic:
        rust_type: "crate::openai::Model"
      properties:
        id:
          type: string
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `rust_type` | `string?` | Full Rust type path for code generation. |

## Complete Example

```yaml
openapi: "3.0.3"
info:
  title: OpenAI
  description: OpenAI REST API for model management
  version: 1.0.0
servers:
  - url: https://api.openai.com/v1
    description: OpenAI API Server
externalDocs:
  url: https://platform.openai.com/docs/api-reference
  description: API Documentation
x-schematic:
  env_mapping:
    bearer_token:
      names: ["OPENAI_API_KEY"]
security:
  - bearerAuth: []
paths:
  /models:
    get:
      operationId: ListModels
      summary: Lists the currently available models
      description: Lists the currently available models
      x-schematic:
        response:
          Json:
            Named: ListModelsResponse
      responses:
        "200":
          description: Successful JSON response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ListModelsResponse'
  /models/{model}:
    get:
      operationId: RetrieveModel
      summary: Retrieves a model instance
      description: Retrieves a model instance
      x-schematic:
        response:
          Json:
            Named: Model
      responses:
        "200":
          description: Successful JSON response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Model'
    parameters:
      - name: model
        in: path
        required: true
        schema:
          type: string
    delete:
      operationId: DeleteModel
      summary: Delete a fine-tuned model
      description: Delete a fine-tuned model
      x-schematic:
        response:
          Json:
            Named: DeleteModelResponse
      responses:
        "200":
          description: Successful JSON response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/DeleteModelResponse'
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
      description: Bearer token authentication
  schemas:
    Model:
      type: object
      description: An OpenAI model object.
      properties:
        id:
          type: string
        object:
          type: string
        created:
          type: integer
        owned_by:
          type: string
      required:
        - id
        - object
        - created
        - owned_by
    ListModelsResponse:
      type: object
      description: Response from the List Models endpoint.
      properties:
        object:
          type: string
        data:
          type: array
          items:
            $ref: '#/components/schemas/Model'
      required:
        - object
        - data
    DeleteModelResponse:
      type: object
      description: Response from the Delete Model endpoint.
      properties:
        id:
          type: string
        object:
          type: string
        deleted:
          type: boolean
      required:
        - id
        - object
        - deleted
```

## Skipping Extensions

To generate OpenAPI documents without vendor extensions (for maximum compatibility with third-party tools), use the `skip_extensions()` option:

```rust
use schematic_define::openapi::{export, ExportOptions};

let options = ExportOptions::new().skip_extensions();
let doc = export(&api, &options)?;
```

Or via CLI:

```bash
# Note: --skip-extensions flag not yet implemented in CLI
# Extensions are included by default
schematic-gen generate --api openai --openapi-out ./openapi
```

## Compatibility

These extensions are designed to be safely ignored by tools that don't understand them. Standard OpenAPI 3.0.3 parsers will process the core specification correctly while skipping `x-schematic` fields.

The extensions follow OpenAPI's vendor extension naming convention (prefix with `x-`) and use standard JSON types for all values.
