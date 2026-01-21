# schematic-define

REST and WebSocket API definition types for the Schematic code generation system.

## Overview

`schematic-define` provides a declarative way to describe REST and WebSocket APIs. These definitions are consumed by `schematic-gen` to generate strongly-typed Rust client code with automatic authentication, request serialization, and response deserialization.

The definition process is intentionally **data-driven**: you describe *what* the API looks like (endpoints, methods, schemas) rather than *how* to call it. The generator handles the implementation details.

## Core Types

### REST API Types

| Type | Purpose |
|------|---------|
| `RestApi` | Complete API definition with base URL, auth, endpoints, and codegen options |
| `Endpoint` | Single endpoint with method, path, request/response schemas |
| `RestMethod` | HTTP methods (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS) |
| `AuthStrategy` | Authentication configuration (Bearer, API Key, Basic, None) |
| `ApiRequest` | Request body type (JSON, FormData, UrlEncoded, Text, Binary) |
| `ApiResponse` | Response type (JSON, Text, Binary, Empty) |
| `FormField` | Form field definition for multipart/URL-encoded requests |
| `FormFieldKind` | Form field type (Text, File, Files, Json) |
| `Schema` | Type name and optional module path for code generation |

### WebSocket API Types

| Type | Purpose |
|------|---------|
| `WebSocketApi` | Complete WebSocket API definition with base URL, auth, and endpoints |
| `WebSocketEndpoint` | Single WebSocket endpoint with path, parameters, and message schemas |
| `ConnectionParam` | Query/path parameter definition for WebSocket connections |
| `ParamType` | Parameter types (String, Integer, Boolean, Float) |
| `ConnectionLifecycle` | Open, close, and keepalive message schemas |
| `MessageSchema` | Single message type with direction (Client, Server, Bidirectional) |
| `MessageDirection` | Message flow direction enumeration |

## Definition Workflow

```
┌─────────────────────┐
│   Define RestApi     │
│  - name, base_url   │
│  - auth strategy    │
│  - endpoints[]      │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  schematic-gen      │
│  (code generator)   │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  Generated Client   │
│  - Type-safe API    │
│  - Auto auth        │
│  - Serialization    │
└─────────────────────┘
```

## Authentication Strategies

Authentication is configured in two parts:

1. `AuthStrategy` on `RestApi::auth` - defines *how* auth is applied
2. `env_auth`, `env_username`, `env_password` on `RestApi` - defines *where* credentials come from

### Bearer Token (Most Common)

```rust
use schematic_define::{RestApi, AuthStrategy};

let api = RestApi {
    auth: AuthStrategy::BearerToken { header: None }, // Uses "Authorization" header
    env_auth: vec!["OPENAI_API_KEY".to_string()],    // Env var(s) to check
    // ...
};
```

Generates: `Authorization: Bearer <token>`

Multiple env vars can be specified as a fallback chain - the first one found is used:

```rust
env_auth: vec!["OPENAI_API_KEY".to_string(), "OPENAI_KEY".to_string()],
```

### API Key in Custom Header

```rust
use schematic_define::{RestApi, AuthStrategy};

let api = RestApi {
    auth: AuthStrategy::ApiKey { header: "X-API-Key".to_string() },
    env_auth: vec!["MY_API_KEY".to_string()],
    // ...
};
```

Generates: `X-API-Key: <key>`

### Basic Authentication

```rust
use schematic_define::{RestApi, AuthStrategy};

let api = RestApi {
    auth: AuthStrategy::Basic,
    env_username: Some("SERVICE_USER".to_string()),
    env_password: Some("SERVICE_PASSWORD".to_string()),
    // ...
};
```

Generates: `Authorization: Basic <base64(user:pass)>`

### No Authentication

```rust
use schematic_define::{RestApi, AuthStrategy};

let api = RestApi {
    auth: AuthStrategy::None,
    env_auth: vec![],
    // ...
};
```

### Missing Credentials

If required credentials are not found in the environment, the generated code returns a `SchematicError::MissingCredential` error with the list of env vars that were checked:

```rust
// Generated error when no credentials found
Err(SchematicError::MissingCredential {
    env_vars: vec!["OPENAI_API_KEY".to_string(), "OPENAI_KEY".to_string()],
})
```

## Code Generation Options

`RestApi` includes optional fields to customize generated code:

### Module Path

By default, the generated module uses the lowercased API name. Override with `module_path`:

```rust
let api = RestApi {
    name: "OllamaOpenAI".to_string(),
    module_path: Some("ollama".to_string()),  // Use "ollama" instead of "ollamaopenai"
    // ...
};
```

This is useful when:
- Multiple APIs share a definitions module (e.g., `OllamaNative` and `OllamaOpenAI` both in `ollama/`)
- The API name doesn't match the desired module name

### Request Suffix

By default, generated wrapper structs use the "Request" suffix (e.g., `ListModelsRequest`). Customize with `request_suffix`:

```rust
let api = RestApi {
    name: "MyApi".to_string(),
    request_suffix: Some("Req".to_string()),  // Use "Req" instead of "Request"
    // ...
};
// Generates: ListModelsReq, CreateUserReq, etc.
```

**Note**: The suffix must be alphanumeric. Invalid suffixes (containing spaces, hyphens, etc.) will cause a validation error.

## Request Types

Endpoints can accept different request body formats via `ApiRequest`:

| Variant | Content-Type | Use Case |
|---------|-------------|----------|
| `ApiRequest::Json(schema)` | `application/json` | Most API requests |
| `ApiRequest::FormData { fields }` | `multipart/form-data` | File uploads, mixed data |
| `ApiRequest::UrlEncoded { fields }` | `application/x-www-form-urlencoded` | Simple form data |
| `ApiRequest::Text { content_type }` | Custom text MIME | Raw text bodies |
| `ApiRequest::Binary { content_type }` | Custom binary MIME | Raw binary bodies |

### Form Fields

`FormField` describes individual fields in multipart or URL-encoded forms:

```rust
use schematic_define::{ApiRequest, FormField, FormFieldKind, Schema};

// File upload with optional metadata
let request = ApiRequest::form_data(vec![
    FormField::file_accept("audio", vec!["audio/*".into()])
        .with_description("Audio file (mp3, wav, ogg)"),
    FormField::text("name")
        .optional()
        .with_description("Optional name for the file"),
    FormField::json("metadata", Schema::new("FileMetadata"))
        .optional(),
]);
```

### FormFieldKind Variants

| Kind | Description |
|------|-------------|
| `Text` | Plain text field |
| `File { accept }` | Single file with optional MIME restrictions |
| `Files { accept, min, max }` | Multiple files with optional count constraints |
| `Json(Schema)` | Embedded JSON data |

### Builder Methods

`FormField` provides convenient builders:

```rust
use schematic_define::FormField;

// Required text field
let name = FormField::text("name");

// Optional text field with description
let bio = FormField::text("bio")
    .optional()
    .with_description("User biography");

// File upload accepting any type
let doc = FormField::file("document");

// File upload with MIME restrictions
let image = FormField::file_accept("avatar", vec!["image/*".into()]);

// Multiple files with constraints
let samples = FormField::files_with_constraints(
    "audio_samples",
    vec!["audio/*".into()],
    Some(1),   // min
    Some(10),  // max
);
```

## Response Types

| Variant | Generated Return Type | Use Case |
|---------|----------------------|----------|
| `ApiResponse::Json(schema)` | Deserialized struct | Most API responses |
| `ApiResponse::Text` | `String` | Plain text endpoints |
| `ApiResponse::Binary` | `Vec<u8>` | File downloads, images |
| `ApiResponse::Empty` | `()` | DELETE, 204 responses |

## WebSocket APIs

WebSocket APIs use a parallel type system that shares authentication strategies with REST APIs but provides WebSocket-specific concepts like connection parameters, message direction, and lifecycle management.

### WebSocket Example: ElevenLabs Text-to-Speech

```rust
use schematic_define::{
    WebSocketApi, WebSocketEndpoint, ConnectionParam, ParamType,
    ConnectionLifecycle, MessageSchema, MessageDirection,
    AuthStrategy, Schema
};

let api = WebSocketApi {
    name: "ElevenLabsTTS".to_string(),
    description: "ElevenLabs Text-to-Speech WebSocket API".to_string(),
    base_url: "wss://api.elevenlabs.io/v1".to_string(),
    docs_url: Some("https://elevenlabs.io/docs/api-reference/websockets".to_string()),
    auth: AuthStrategy::ApiKey { header: "xi-api-key".to_string() },
    env_auth: vec!["ELEVEN_LABS_API_KEY".to_string()],
    endpoints: vec![
        WebSocketEndpoint {
            id: "TextToSpeech".to_string(),
            path: "/text-to-speech/{voice_id}/stream-input".to_string(),
            description: "Stream text and receive audio chunks".to_string(),
            connection_params: vec![
                ConnectionParam {
                    name: "model_id".to_string(),
                    param_type: ParamType::String,
                    required: false,
                    description: Some("Model to use for synthesis".to_string()),
                },
                ConnectionParam {
                    name: "output_format".to_string(),
                    param_type: ParamType::String,
                    required: false,
                    description: Some("Audio output format".to_string()),
                },
            ],
            lifecycle: ConnectionLifecycle {
                open: Some(MessageSchema {
                    name: "BOS".to_string(),
                    direction: MessageDirection::Client,
                    schema: Schema::new("BeginOfStreamMessage"),
                    description: Some("Begin-of-stream message".to_string()),
                }),
                close: Some(MessageSchema {
                    name: "EOS".to_string(),
                    direction: MessageDirection::Client,
                    schema: Schema::new("EndOfStreamMessage"),
                    description: Some("End-of-stream signal".to_string()),
                }),
                keepalive: None,
            },
            messages: vec![
                MessageSchema {
                    name: "TextChunk".to_string(),
                    direction: MessageDirection::Client,
                    schema: Schema::new("TextChunkMessage"),
                    description: Some("Text to synthesize".to_string()),
                },
                MessageSchema {
                    name: "AudioChunk".to_string(),
                    direction: MessageDirection::Server,
                    schema: Schema::new("AudioChunkResponse"),
                    description: Some("Audio data chunk".to_string()),
                },
            ],
        },
    ],
};
```

### Message Direction

WebSocket messages have a direction indicating their flow:

| Direction | Description |
|-----------|-------------|
| `Client` | Sent from client to server |
| `Server` | Sent from server to client |
| `Bidirectional` | Can flow in either direction |

### Connection Lifecycle

WebSocket connections can define special lifecycle messages:

- **open**: Message sent immediately after connection (e.g., initialization/config)
- **close**: Message sent before graceful disconnection
- **keepalive**: Heartbeat message to maintain connection

## REST API Examples

### Example 1: Simple Public API (No Auth)

A basic health-check API with no authentication:

```rust
use schematic_define::{RestApi, Endpoint, RestMethod, AuthStrategy, ApiResponse};

let api = RestApi {
    name: "HealthService".to_string(),
    description: "Simple health monitoring service".to_string(),
    base_url: "https://api.example.com".to_string(),
    docs_url: None,
    auth: AuthStrategy::None,
    env_auth: vec![],
    env_username: None,
    env_password: None,
    endpoints: vec![
        Endpoint {
            id: "GetHealth".to_string(),
            method: RestMethod::Get,
            path: "/health".to_string(),
            description: "Check service health".to_string(),
            request: None,
            response: ApiResponse::json_type("HealthStatus"),
        },
        Endpoint {
            id: "GetVersion".to_string(),
            method: RestMethod::Get,
            path: "/version".to_string(),
            description: "Get service version".to_string(),
            request: None,
            response: ApiResponse::Text,
        },
    ],
};
```

### Example 2: REST API with Bearer Token Auth

A user management API with CRUD operations:

```rust
use schematic_define::{
    RestApi, Endpoint, RestMethod, AuthStrategy, ApiRequest, ApiResponse
};

let api = RestApi {
    name: "UserService".to_string(),
    description: "User management REST API".to_string(),
    base_url: "https://api.myservice.com/v1".to_string(),
    docs_url: Some("https://docs.myservice.com/api".to_string()),
    auth: AuthStrategy::BearerToken { header: None },
    env_auth: vec!["MYSERVICE_API_KEY".to_string()],
    env_username: None,
    env_password: None,
    headers: vec![],
    endpoints: vec![
        // List all users
        Endpoint {
            id: "ListUsers".to_string(),
            method: RestMethod::Get,
            path: "/users".to_string(),
            description: "List all users".to_string(),
            request: None,
            response: ApiResponse::json_type("ListUsersResponse"),
            headers: vec![],
        },
        // Get a specific user by ID (path parameter)
        Endpoint {
            id: "GetUser".to_string(),
            method: RestMethod::Get,
            path: "/users/{user_id}".to_string(),
            description: "Retrieve a user by ID".to_string(),
            request: None,
            response: ApiResponse::json_type("User"),
            headers: vec![],
        },
        // Create a new user (with JSON request body)
        Endpoint {
            id: "CreateUser".to_string(),
            method: RestMethod::Post,
            path: "/users".to_string(),
            description: "Create a new user".to_string(),
            request: Some(ApiRequest::json_type("CreateUserRequest")),
            response: ApiResponse::json_type("User"),
            headers: vec![],
        },
        // Update a user
        Endpoint {
            id: "UpdateUser".to_string(),
            method: RestMethod::Put,
            path: "/users/{user_id}".to_string(),
            description: "Update an existing user".to_string(),
            request: Some(ApiRequest::json_type("UpdateUserRequest")),
            response: ApiResponse::json_type("User"),
            headers: vec![],
        },
        // Delete a user
        Endpoint {
            id: "DeleteUser".to_string(),
            method: RestMethod::Delete,
            path: "/users/{user_id}".to_string(),
            description: "Delete a user".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
        },
    ],
};
```

### Example 3: File Storage API with File Uploads

A file storage API demonstrating multipart form-data uploads:

```rust
use schematic_define::{
    RestApi, Endpoint, RestMethod, AuthStrategy, ApiRequest, ApiResponse, FormField
};

let api = RestApi {
    name: "FileStorage".to_string(),
    description: "Cloud file storage API".to_string(),
    base_url: "https://storage.example.com/api/v2".to_string(),
    docs_url: Some("https://storage.example.com/docs".to_string()),
    auth: AuthStrategy::ApiKey { header: "X-Storage-Key".to_string() },
    env_auth: vec!["STORAGE_API_KEY".to_string()],
    env_username: None,
    env_password: None,
    headers: vec![],
    endpoints: vec![
        // List files - returns JSON
        Endpoint {
            id: "ListFiles".to_string(),
            method: RestMethod::Get,
            path: "/files".to_string(),
            description: "List all files in storage".to_string(),
            request: None,
            response: ApiResponse::json_type("FileList"),
            headers: vec![],
        },
        // Upload file - multipart form-data
        Endpoint {
            id: "UploadFile".to_string(),
            method: RestMethod::Post,
            path: "/files".to_string(),
            description: "Upload a new file".to_string(),
            request: Some(ApiRequest::form_data(vec![
                FormField::file("file").with_description("The file to upload"),
                FormField::text("folder").optional().with_description("Target folder"),
                FormField::text("description").optional(),
            ])),
            response: ApiResponse::json_type("FileMetadata"),
            headers: vec![],
        },
        // Download file - returns binary data
        Endpoint {
            id: "DownloadFile".to_string(),
            method: RestMethod::Get,
            path: "/files/{file_id}/content".to_string(),
            description: "Download file contents".to_string(),
            request: None,
            response: ApiResponse::Binary,
            headers: vec![],
        },
        // Get file metadata - returns JSON
        Endpoint {
            id: "GetFileMetadata".to_string(),
            method: RestMethod::Get,
            path: "/files/{file_id}".to_string(),
            description: "Get file metadata".to_string(),
            request: None,
            response: ApiResponse::json_type("FileMetadata"),
            headers: vec![],
        },
        // Delete file - returns empty
        Endpoint {
            id: "DeleteFile".to_string(),
            method: RestMethod::Delete,
            path: "/files/{file_id}".to_string(),
            description: "Delete a file".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
        },
    ],
};
```

## Path Parameters

Paths support template parameters using curly braces. These become fields in the generated request struct:

```rust
// Path: "/users/{user_id}/posts/{post_id}"
// Generated code will require both `user_id` and `post_id` parameters
```

## Prelude

For convenient imports, use the prelude:

```rust
use schematic_define::prelude::*;

// Now you have access to all core types:
// REST: RestApi, Endpoint, RestMethod, AuthStrategy, ApiRequest, ApiResponse,
//       FormField, FormFieldKind, Schema
// WebSocket: WebSocketApi, WebSocketEndpoint, ConnectionParam, ParamType,
//            ConnectionLifecycle, MessageSchema, MessageDirection
```

## Pre-built API Definitions

Pre-built API definitions (like OpenAI) are in the separate `schematic-definitions` crate:

```rust
use schematic_definitions::openai::define_openai_api;

let openai = define_openai_api();
assert_eq!(openai.name, "OpenAI");
assert_eq!(openai.endpoints.len(), 3);
```

See the [schematic-definitions README](../definitions/README.md) for available APIs.

## Naming Conventions

### Body Type Naming

When defining request body types, use a `*Body` suffix to avoid collisions with generated wrapper structs:

```rust
// ✗ BAD: Collides with generated wrapper struct
pub struct GenerateRequest { ... }  // Definition type
// Generated: pub struct GenerateRequest { body: GenerateRequest } ← Recursive!

// ✓ GOOD: Uses *Body suffix
pub struct GenerateBody { ... }  // Definition type
// Generated: pub struct GenerateRequest { body: GenerateBody } ← Works!
```

The generator creates `{EndpointId}Request` wrapper structs for each endpoint. If your body type uses the same name, you'll get a recursive struct that won't compile.

**Convention**: Name body types as `{EndpointId}Body` (e.g., `GenerateBody`, `CreateChatBody`, `EmbedBody`).

### Required Derives for Body Types

All request body types **must** derive `Default`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GenerateBody {
    pub model: String,   // Empty string by default
    pub prompt: String,  // Empty string by default
}
```

The generated wrapper structs implement `Default` and call `Default::default()` on the body type. Without this derive, generated code won't compile.

**Note**: A default with empty strings may be invalid for the API, but it's valid Rust. The API will return an error at runtime, not compile time.

## Schema with Module Paths

For types in specific modules, use `Schema::with_path`:

```rust
use schematic_define::Schema;

// Type in current scope
let simple = Schema::new("User");
assert_eq!(simple.full_path(), "User");

// Type in specific module
let qualified = Schema::with_path("User", "crate::models::user");
assert_eq!(qualified.full_path(), "crate::models::user::User");
```

## Migration from Schema to ApiRequest

If you have existing code using `Option<Schema>` for `Endpoint.request`, you need to migrate to `Option<ApiRequest>`:

```rust
// Before (deprecated pattern)
Endpoint {
    request: Some(Schema::new("CreateUserRequest")),
    // ...
}

// After (new pattern)
Endpoint {
    request: Some(ApiRequest::json_type("CreateUserRequest")),
    // ...
}
```

For backward compatibility, `ApiRequest` implements `From<Schema>`:

```rust
use schematic_define::{ApiRequest, Schema};

let schema = Schema::new("MyRequest");
let request: ApiRequest = schema.into(); // Converts to ApiRequest::Json(schema)
```

## Dependencies

- `serde` - Serialization/deserialization
- `strum` - Enum utilities (Display, FromStr, Iterator)
- `thiserror` - Error types

## License

AGPL-3.0-only
