# OpenAPI Vendor Extensions (x-schematic)

This document specifies the vendor extensions used by Schematic when exporting API definitions to OpenAPI 3.0.3 format. These extensions preserve Schematic-specific metadata that enables round-trip fidelity when importing and re-exporting API definitions.

## Overview

Schematic uses three extension types at different levels of the OpenAPI document:

| Extension | Level | Purpose |
|-----------|-------|---------|
| `x-schematic` | Document | API-wide configuration (module path, headers, env mapping) |
| `x-schematic` | Operation | Endpoint-specific metadata (request/response types, headers) |
| `x-schematic` | Schema | Type metadata (Rust type paths) |

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

### Env Mapping

The `env_mapping` object specifies how to resolve authentication credentials from environment variables:

```yaml
env_mapping:
  bearer_token:
    names: ["OPENAI_API_KEY", "OPENAI_TOKEN"]
  api_key:
    names: ["ANTHROPIC_API_KEY"]
    header: X-Api-Key
  basic_auth:
    username: ["API_USER"]
    password: ["API_PASSWORD"]
```

| Field | Type | Description |
|-------|------|-------------|
| `bearer_token` | `{names: string[]}` | Env vars for Bearer token authentication |
| `api_key` | `{names: string[], header: string}` | Env vars and header name for API key auth |
| `basic_auth` | `{username: string[], password: string[]}` | Env vars for HTTP Basic authentication |

## Operation-Level Extension

Added at the operation level (inside GET, POST, etc.) to preserve endpoint-specific metadata.

```yaml
paths:
  /models:
    get:
      operationId: ListModels
      x-schematic:
        request: null
        response:
          type: Json
          schema:
            type_name: ListModelsResponse
        headers: []
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `request` | `object?` | Request body specification. See [Request Types](#request-types). |
| `response` | `object` | Response specification. See [Response Types](#response-types). |
| `headers` | `array<[string, string]>` | Endpoint-specific headers. |

### Request Types

```yaml
# JSON body
request:
  type: Json
  schema:
    type_name: CreateMessageBody

# Multipart form data
request:
  type: FormData
  fields:
    - name: document
      kind:
        type: File
        accept: ["application/pdf", "image/*"]
      required: true
      description: The file to upload
    - name: metadata
      kind:
        type: Json
        schema:
          type_name: FileMetadata
      required: false

# URL-encoded form
request:
  type: UrlEncoded
  fields:
    - name: username
      kind: Text
      required: true
    - name: password
      kind: Text
      required: true

# Plain text
request:
  type: Text
  content_type: text/csv

# Binary
request:
  type: Binary
  content_type: application/octet-stream
```

### Response Types

```yaml
# JSON response
response:
  type: Json
  schema:
    type_name: MessageResponse

# Binary response (audio, images)
response:
  type: Binary

# Plain text response
response:
  type: Text

# No content (204)
response:
  type: Empty
```

## Schema-Level Extension

Added at the schema level to preserve Rust type information:

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
          type: Json
          schema:
            type_name: ListModelsResponse
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
          type: Json
          schema:
            type_name: Model
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
          type: Json
          schema:
            type_name: DeleteModelResponse
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
let doc = export(&api, &registry, &options)?;
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
