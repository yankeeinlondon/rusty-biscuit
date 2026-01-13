# schematic-define

REST API definition types for the Schematic code generation system.

## Overview

`schematic-define` provides a declarative way to describe REST APIs. These definitions are consumed by `schematic-gen` to generate strongly-typed Rust client code with automatic authentication, request serialization, and response deserialization.

The definition process is intentionally **data-driven**: you describe *what* the API looks like (endpoints, methods, schemas) rather than *how* to call it. The generator handles the implementation details.

## Core Types

| Type | Purpose |
|------|---------|
| `RestApi` | Complete API definition with base URL, auth, and endpoints |
| `Endpoint` | Single endpoint with method, path, request/response schemas |
| `RestMethod` | HTTP methods (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS) |
| `AuthStrategy` | Authentication configuration (Bearer, API Key, Basic, None) |
| `ApiResponse` | Response type (JSON, Text, Binary, Empty) |
| `Schema` | Type name and optional module path for code generation |

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

## Response Types

| Variant | Generated Return Type | Use Case |
|---------|----------------------|----------|
| `ApiResponse::Json(schema)` | Deserialized struct | Most API responses |
| `ApiResponse::Text` | `String` | Plain text endpoints |
| `ApiResponse::Binary` | `Vec<u8>` | File downloads, images |
| `ApiResponse::Empty` | `()` | DELETE, 204 responses |

## Examples

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
    RestApi, Endpoint, RestMethod, AuthStrategy, ApiResponse, Schema
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
    endpoints: vec![
        // List all users
        Endpoint {
            id: "ListUsers".to_string(),
            method: RestMethod::Get,
            path: "/users".to_string(),
            description: "List all users".to_string(),
            request: None,
            response: ApiResponse::json_type("ListUsersResponse"),
        },
        // Get a specific user by ID (path parameter)
        Endpoint {
            id: "GetUser".to_string(),
            method: RestMethod::Get,
            path: "/users/{user_id}".to_string(),
            description: "Retrieve a user by ID".to_string(),
            request: None,
            response: ApiResponse::json_type("User"),
        },
        // Create a new user (with request body)
        Endpoint {
            id: "CreateUser".to_string(),
            method: RestMethod::Post,
            path: "/users".to_string(),
            description: "Create a new user".to_string(),
            request: Some(Schema::new("CreateUserRequest")),
            response: ApiResponse::json_type("User"),
        },
        // Update a user
        Endpoint {
            id: "UpdateUser".to_string(),
            method: RestMethod::Put,
            path: "/users/{user_id}".to_string(),
            description: "Update an existing user".to_string(),
            request: Some(Schema::new("UpdateUserRequest")),
            response: ApiResponse::json_type("User"),
        },
        // Delete a user
        Endpoint {
            id: "DeleteUser".to_string(),
            method: RestMethod::Delete,
            path: "/users/{user_id}".to_string(),
            description: "Delete a user".to_string(),
            request: None,
            response: ApiResponse::Empty,
        },
    ],
};
```

### Example 3: File Storage API with Multiple Auth & Response Types

A more complex API demonstrating various response types and API key authentication:

```rust
use schematic_define::{
    RestApi, Endpoint, RestMethod, AuthStrategy, ApiResponse, Schema
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
    endpoints: vec![
        // List files - returns JSON
        Endpoint {
            id: "ListFiles".to_string(),
            method: RestMethod::Get,
            path: "/files".to_string(),
            description: "List all files in storage".to_string(),
            request: None,
            response: ApiResponse::json_type("FileList"),
        },
        // Upload file - accepts binary, returns JSON metadata
        Endpoint {
            id: "UploadFile".to_string(),
            method: RestMethod::Post,
            path: "/files".to_string(),
            description: "Upload a new file".to_string(),
            request: Some(Schema::new("UploadRequest")),
            response: ApiResponse::json_type("FileMetadata"),
        },
        // Download file - returns binary data
        Endpoint {
            id: "DownloadFile".to_string(),
            method: RestMethod::Get,
            path: "/files/{file_id}/content".to_string(),
            description: "Download file contents".to_string(),
            request: None,
            response: ApiResponse::Binary,
        },
        // Get file metadata - returns JSON
        Endpoint {
            id: "GetFileMetadata".to_string(),
            method: RestMethod::Get,
            path: "/files/{file_id}".to_string(),
            description: "Get file metadata".to_string(),
            request: None,
            response: ApiResponse::json_type("FileMetadata"),
        },
        // Delete file - returns empty
        Endpoint {
            id: "DeleteFile".to_string(),
            method: RestMethod::Delete,
            path: "/files/{file_id}".to_string(),
            description: "Delete a file".to_string(),
            request: None,
            response: ApiResponse::Empty,
        },
        // Get raw text content (e.g., for text files)
        Endpoint {
            id: "GetTextContent".to_string(),
            method: RestMethod::Get,
            path: "/files/{file_id}/text".to_string(),
            description: "Get file as plain text".to_string(),
            request: None,
            response: ApiResponse::Text,
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

## Pre-built API Definitions

The `apis` module contains ready-to-use definitions:

```rust
use schematic_define::apis::define_openai_api;

let openai = define_openai_api();
assert_eq!(openai.name, "OpenAI");
assert_eq!(openai.endpoints.len(), 3);
```

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

## Dependencies

- `serde` - Serialization/deserialization
- `strum` - Enum utilities (Display, FromStr, Iterator)
- `thiserror` - Error types

## License

AGPL-3.0-only
